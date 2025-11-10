//! Storage abstraction layer for Gauge
//!
//! This module provides a unified storage interface that works with various backends:
//! - Local filesystem
//! - Google Cloud Storage (GCS) - requires `gcs` feature
//! - AWS S3 - requires `aws` feature
//! - Azure Blob Storage - requires `azure` feature
//!
//! The storage layer handles atomic operations, concurrency control, and provides
//! a consistent API across all backend types.
//!
//! # Cloud Storage Support
//!
//! Cloud storage backends are enabled via Cargo features and use the `object_store` crate
//! for implementation. Authentication is handled through each provider's standard credential chain.
//!
//! ## Google Cloud Storage
//!
//! Enable with the `gcs` feature and set up authentication:
//! ```bash
//! export GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account-key.json"
//! # or use: gcloud auth application-default login
//! ```
//!
//! ## AWS S3
//!
//! Enable with the `aws` feature and configure credentials:
//! ```bash
//! export AWS_ACCESS_KEY_ID="your-access-key"
//! export AWS_SECRET_ACCESS_KEY="your-secret-key"
//! ```
//!
//! ## Azure Blob Storage
//!
//! Enable with the `azure` feature and set up credentials:
//! ```bash
//! export AZURE_STORAGE_ACCOUNT="your-account"
//! export AZURE_STORAGE_KEY="your-key"
//! ```

use std::sync::Arc;

use bytes::Bytes;
use object_store::{DynObjectStore, ObjectStore, PutMode, PutOptions, path::Path};

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Object store error: {0}")]
    ObjectStore(#[from] object_store::Error),
    #[error("Object store path error: {0}")]
    ObjectStorePath(#[from] object_store::path::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Structure Error: {0}")]
    Structure(#[from] crate::internals::StructureError),
    #[error("Error: {0}")]
    Generic(String),
}

/// Create storage backend from a path string.
///
/// Supports local filesystem, GCS, S3, and Azure blob storage URIs.
fn create_storage_from_path(path_str: &str) -> Result<(Storage, Path)> {
    if let Some(scheme_end) = path_str.find("://") {
        let scheme = &path_str[..scheme_end];
        let path_part = &path_str[scheme_end + 3..];

        match scheme {
            "file" => {
                let store = object_store::local::LocalFileSystem::new();
                let storage = Storage::new(Arc::new(store));
                let obj_path = Path::from(std::path::absolute(path_part)?.to_str().unwrap());
                Ok((storage, obj_path))
            }

            "gs" => {
                use object_store::gcp::GoogleCloudStorageBuilder;

                let parts: Vec<&str> = path_part.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(StorageError::InvalidPath(
                        "GCS path must be in the format gs://<bucket>/<key>".to_string(),
                    ));
                }
                let bucket = parts[0];
                let object_key = if parts.len() > 1 { parts[1] } else { "" };

                let mut builder = GoogleCloudStorageBuilder::new().with_bucket_name(bucket);

                if let Ok(service_account_path) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
                    builder = builder.with_service_account_path(&service_account_path);
                } else if let Ok(service_account_key) = std::env::var("GOOGLE_SERVICE_ACCOUNT") {
                    builder = builder.with_service_account_key(&service_account_key);
                }

                let store = builder.build().map_err(|e| {
                    let error_msg = format!(
                        "Failed to create GCS store: {}. \n\
                         Please ensure you have configured authentication:\n\
                         1. Set GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account-key.json, or\n\
                         2. Set GOOGLE_SERVICE_ACCOUNT with the JSON key content, or\n\
                         3. Run 'gcloud auth application-default login', or\n\
                         4. Run on GCE/GKE with a service account attached.",
                        e
                    );
                    StorageError::ObjectStore(
                        object_store::Error::Generic {
                            store: "GoogleCloudStorage",
                            source: Box::new(std::io::Error::new(
                                std::io::ErrorKind::PermissionDenied,
                                error_msg,
                            )),
                        },
                    )
                })?;

                let storage = Storage::new(Arc::new(store));
                let obj_path = Path::from(object_key);
                Ok((storage, obj_path))
            }

            "az" | "azure" => {
                use object_store::azure::MicrosoftAzureBuilder;

                let parts: Vec<&str> = path_part.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(StorageError::InvalidPath(
                        "Azure path must be in the format az://<container>/<key>".to_string(),
                    ));
                }
                let container = parts[0];
                let object_key = if parts.len() > 1 { parts[1] } else { "" };

                let mut builder = MicrosoftAzureBuilder::new().with_container_name(container);

                if let Ok(account) = std::env::var("AZURE_STORAGE_ACCOUNT") {
                    builder = builder.with_account(account);
                }

                if let Ok(key) = std::env::var("AZURE_STORAGE_KEY") {
                    builder = builder.with_access_key(key);
                }

                let store = builder
                    .build()
                    .map_err(|e| {
                        StorageError::ObjectStore(
                            object_store::Error::Generic {
                                store: "MicrosoftAzure",
                                source: Box::new(std::io::Error::new(
                                    std::io::ErrorKind::PermissionDenied,
                                    format!("Failed to create Azure store: {}. Ensure Azure credentials (e.g., AZURE_STORAGE_ACCOUNT, AZURE_STORAGE_KEY) are set.", e),
                                )),
                            },
                        )
                    })?;

                let storage = Storage::new(Arc::new(store));
                let obj_path = Path::from(object_key);
                Ok((storage, obj_path))
            }

            _ => Err(StorageError::InvalidPath(format!(
                "Unsupported URI scheme: '{}'",
                scheme
            ))),
        }
    } else {
        // No scheme, treat as local filesystem path
        let store = object_store::local::LocalFileSystem::new();
        let storage = Storage::new(Arc::new(store));
        let obj_path = Path::from(std::path::absolute(path_str)?.to_str().unwrap());
        Ok((storage, obj_path))
    }
}

/// Storage abstraction that works with local filesystem and cloud storage
///
/// This type wraps an `object_store` backend and provides higher-level operations
/// for atomic file updates, conditional writes, and listing operations.
///
/// # Thread Safety
///
/// Storage instances are cheaply cloneable (Arc internally) and can be safely
/// shared across threads.
#[derive(Debug)]
pub struct Storage {
    inner: Arc<DynObjectStore>,
}

impl Storage {
    pub fn from_path(path_str: &str) -> Result<(Self, Path)> {
        create_storage_from_path(path_str)
    }

    /// Create a new Storage instance with the given object store
    pub fn new(store: Arc<DynObjectStore>) -> Self {
        Self { inner: store }
    }

    /// Read the contents of a file in a lock-less manner
    pub async fn read_file(&self, path: &Path) -> Result<(Bytes, String)> {
        let result = self.inner.get(path).await?;
        let e_tag = result.meta.e_tag.clone().unwrap();
        let bytes = result.bytes().await?;
        Ok((bytes, e_tag))
    }

    /// Update a file atomically using a closure.
    /// Reads the current content, applies the update function, and writes back atomically.
    /// Uses conditional put with e_tag and/or version to ensure atomicity (optimistic concurrency control).
    /// Falls back to simple overwrite if conditional updates are not supported.
    pub async fn update_file<F>(&self, path: &Path, update: F) -> Result<()>
    where
        F: FnOnce(Bytes) -> Result<Bytes>,
    {
        // Read current content and get e_tag/version for optimistic concurrency control
        let get_result = self.inner.get(path).await?;
        let e_tag = get_result.meta.e_tag.clone();
        let version = get_result.meta.version.clone();
        let current = get_result.bytes().await?;

        // Apply update
        let updated = update(current)?;

        // Try to write back atomically using conditional put with e_tag and/or version
        // GCS requires version, S3/Azure use e_tag. Provide both for maximum compatibility.
        if e_tag.is_some() || version.is_some() {
            let opts = PutOptions {
                mode: PutMode::Update(object_store::UpdateVersion { e_tag, version }),
                ..Default::default()
            };

            match self
                .inner
                .put_opts(path, updated.clone().into(), opts)
                .await
            {
                Ok(_) => {
                    return Ok(());
                }
                Err(object_store::Error::NotImplemented) => {
                    // Fall through to simple overwrite
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        self.inner.put(path, updated.into()).await?;

        Ok(())
    }

    /// Create a file if it doesn't exist, with optional initial content
    pub async fn create_file_if_not_exists(
        &self,
        path: &Path,
        initial_data: Bytes,
    ) -> Result<String> {
        let opts = PutOptions {
            mode: PutMode::Create,
            ..Default::default()
        };

        self.inner.put_opts(path, initial_data.into(), opts).await?;
        let result = self.inner.head(path).await?;
        Ok(result.e_tag.unwrap())
    }

    /// List all files with a given prefix
    pub async fn list_files(&self, prefix: Option<&Path>) -> Result<Vec<Path>> {
        let list_result = self.inner.list(prefix);

        let mut paths = Vec::new();
        use futures::stream::TryStreamExt;

        let collected: Vec<_> = list_result.try_collect().await?;
        for meta in collected {
            paths.push(meta.location);
        }

        Ok(paths)
    }

    /// Check if a file exists
    pub async fn exists(&self, path: &Path) -> Result<bool> {
        match self.inner.head(path).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// Check if a file has changed by comparing its ETag against a known value.
    ///
    /// This avoids downloading the file's content and is ideal for cache validation.
    ///
    /// It returns `true` if the file does not exist, if no `last_known_e_tag` is provided,
    /// or if the current ETag doesn't match. It returns `false` only when the file
    /// exists and the ETags match.
    pub async fn has_changed(&self, path: &Path, last_known_e_tag: Option<&str>) -> Result<bool> {
        match self.inner.head(path).await {
            Ok(meta) => {
                match (last_known_e_tag, meta.e_tag.as_deref()) {
                    (Some(known), Some(current)) => Ok(known != current),
                    // Conservatively assume change if we can't compare ETags.
                    _ => Ok(true),
                }
            }
            Err(object_store::Error::NotFound { .. }) => {
                // The file is gone, so it has definitely "changed".
                Ok(true)
            }
            Err(e) => Err(e.into()),
        }
    }
}
