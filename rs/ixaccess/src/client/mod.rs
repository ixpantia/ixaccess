use arc_swap::ArcSwap;
use object_store::path::Path;
use std::sync::Arc;

use crate::{
    internals::IxAccessStructureV1,
    storage::{Storage, StorageError},
};

pub(crate) mod resources;
pub(crate) mod roles;
pub(crate) mod search;

/// An async client for managing access control roles and resources.
///
/// `IxAccessClient` provides a high-level API for interacting with an access control
/// structure stored in a remote object store (like Google Cloud Storage, AWS S3, or Azure Blob Storage).
///
/// The client maintains a local, in-memory cache of the access control state, which is
/// automatically updated when changes are detected in the remote state file. All write
/// operations are performed atomically to prevent race conditions.
///
/// # Cloud Storage URLs
///
/// The client uses URLs to specify the location of the state file in the object store.
/// The URL scheme determines the cloud provider:
///
/// - **Google Cloud Storage:** `gs://<bucket>/<path>`
/// - **Amazon S3:** `s3://<bucket>/<path>`
/// - **Azure Blob Storage:** `az://<container>/<path>`
/// - **Local file:** `/path/to/file` or `file:///path/to/file`
///
/// # Authentication
///
/// Authentication is handled automatically by the underlying `object_store` crate,
/// which uses the standard environment variables for each cloud provider.
///
/// ## Google Cloud Platform
///
/// The client will use the Application Default Credentials (ADC). You can provide credentials by:
/// - Setting the `GOOGLE_APPLICATION_CREDENTIALS` environment variable to the path of a service account key file.
/// - Running on a GCP service (e.g., GCE, GKE, Cloud Run) with a service account attached.
/// - Authenticating with the gcloud CLI using `gcloud auth application-default login`.
///
/// ## Amazon Web Services
///
/// The client will use the default credential provider chain. This typically involves:
/// - `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and `AWS_SESSION_TOKEN` environment variables.
/// - The `~/.aws/credentials` and `~/.aws/config` files.
/// - IAM roles for EC2 instances or ECS tasks.
///
/// ## Microsoft Azure
///
/// The client will use the default credential provider chain. This typically involves:
/// - `AZURE_STORAGE_ACCOUNT` and `AZURE_STORAGE_ACCESS_KEY` environment variables.
/// - Managed identity when running on Azure services.
///
/// # Example
///
/// ```no_run
/// use ixaccess::IxAccessClient;
///
/// #[tokio::main]
/// async fn main() {
///     let client = IxAccessClient::new("gs://my-bucket/access-control.ix").await;
///
///     // Add a new role
///     client.add_role("admin").await.unwrap();
///
///     // List all roles
///     let roles = client.list_roles().await.unwrap();
///     println!("Roles: {:?}", roles);
/// }
/// ```
#[derive(Debug)]
pub struct IxAccessClient {
    pub(super) storage: Storage,
    pub(super) path: Path,
    // Atomic reference to the current state
    pub(super) state: ArcSwap<ClientState>,
}

#[derive(Debug)]
pub(super) struct ClientState {
    pub(super) structure: IxAccessStructureV1,
    pub(super) e_tag: String,
}

impl IxAccessClient {
    /// Creates a new `IxAccessClient` instance.
    ///
    /// This function initializes a connection to the object store specified by the `path`.
    /// If the state file does not exist at the given path, it will be created with an
    /// empty access control structure. If it already exists, its content will be read
    /// to initialize the client's in-memory state.
    ///
    /// # Arguments
    ///
    /// * `path` - A URL-style path to the state file in a supported object store
    ///   (e.g., "gs://my-bucket/access-control.ix").
    pub async fn new(path: &str) -> IxAccessClient {
        let (storage, path) = Storage::from_path(path).unwrap();
        let mut structure = IxAccessStructureV1::new();

        match storage
            .create_file_if_not_exists(&path, structure.to_bytes())
            .await
        {
            Ok(e_tag) => {
                let state = ArcSwap::from_pointee(ClientState { structure, e_tag });
                IxAccessClient {
                    storage,
                    path,
                    state,
                }
            }
            Err(e) => match e {
                StorageError::ObjectStore(object_store::Error::AlreadyExists { .. }) => {
                    let (bytes, e_tag) = storage.read_file(&path).await.unwrap();
                    structure = IxAccessStructureV1::read_from_buffer(&bytes);
                    let state = ArcSwap::from_pointee(ClientState { structure, e_tag });
                    IxAccessClient {
                        storage,
                        path,
                        state,
                    }
                }
                e => panic!("{e}"),
            },
        }
    }

    pub(super) async fn update_if_changed(&self) -> Result<Arc<ClientState>, StorageError> {
        let current_state = self.state.load();

        // Fast path: check if file has changed without downloading
        if !self
            .storage
            .has_changed(&self.path, Some(&current_state.e_tag))
            .await?
        {
            return Ok(Arc::clone(&current_state)); // No change, nothing to do
        }

        // File changed, read new version
        let (bytes, new_e_tag) = self.storage.read_file(&self.path).await?;
        let new_structure = IxAccessStructureV1::read_from_buffer(&bytes);

        let new_state = Arc::new(ClientState {
            structure: new_structure,
            e_tag: new_e_tag,
        });
        // Atomically swap in the new state
        // This is lock-free and won't block readers
        self.state.store(Arc::clone(&new_state));

        Ok(new_state)
    }
}
