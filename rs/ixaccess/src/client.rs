use arc_swap::ArcSwap;

use object_store::path::Path;
use std::sync::Arc;

use crate::{
    internals::{IxAccessStructureV1, Role},
    storage::{Storage, StorageError},
};

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
    storage: Storage,
    path: Path,
    // Atomic reference to the current state
    state: ArcSwap<ClientState>,
}

#[derive(Debug)]
struct ClientState {
    structure: IxAccessStructureV1,
    e_tag: String,
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

    async fn update_if_changed(&self) -> Result<Arc<ClientState>, StorageError> {
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

    /// Lists all top-level roles.
    ///
    /// This function returns a vector of strings, where each string is the name of a role.
    /// It checks for remote changes before returning the list.
    pub async fn list_roles(&self) -> Result<Vec<String>, StorageError> {
        // Check and update if file changed
        let state = self.update_if_changed().await?;

        Ok(state
            .structure
            .list_roles()
            .map(|s| s.to_string())
            .collect())
    }

    /// Lists all roles assigned to a given role, including nested roles.
    ///
    /// This function returns a flattened list of all roles that the specified `role`
    /// has been granted, directly or indirectly.
    pub async fn list_all_roles_for_role(
        &self,
        role: impl AsRef<str>,
    ) -> Result<Vec<String>, StorageError> {
        // Check and update if file changed
        let state = self.update_if_changed().await?;

        let role = Role::new(role);
        let roles = state
            .structure
            .list_all_roles_for_role(&role)
            .map_err(|e| StorageError::InvalidPath(e.to_string()))?;
        Ok(roles.map(|s| s.to_string()).collect())
    }

    /// Adds a new role to the access control structure.
    ///
    /// If the role already exists, this operation has no effect.
    pub async fn add_role(&self, role: impl AsRef<str>) -> Result<(), StorageError> {
        let role = Role::new(role);
        self.storage
            .update_file(&self.path, |b| {
                let mut structure = IxAccessStructureV1::read_from_buffer(&b);
                structure.add_role(&role);
                Ok(structure.to_bytes())
            })
            .await?;
        Ok(())
    }

    /// Adds multiple roles at once.
    ///
    /// This is more efficient than calling `add_role` in a loop as it performs a single
    /// write operation.
    pub async fn add_roles<II, I>(&self, roles: II) -> Result<(), StorageError>
    where
        II: IntoIterator<Item = I>,
        I: AsRef<str>,
    {
        let roles: Vec<Role> = roles.into_iter().map(Role::new).collect();
        self.storage
            .update_file(&self.path, |b| {
                let mut structure = IxAccessStructureV1::read_from_buffer(&b);
                for role in &roles {
                    structure.add_role(role);
                }
                Ok(structure.to_bytes())
            })
            .await?;
        Ok(())
    }

    /// Assigns a role to another role (the assignee).
    ///
    /// This creates a parent-child relationship in the role hierarchy. For example,
    /// assigning the "editor" role to the "admin" role means that "admin" inherits
    /// all permissions of "editor".
    pub async fn assign_role(
        &self,
        assignee: impl AsRef<str>,
        role: impl AsRef<str>,
    ) -> Result<(), StorageError> {
        let assignee = Role::new(assignee);
        let role = Role::new(role);
        self.storage
            .update_file(&self.path, |b| {
                let mut structure = IxAccessStructureV1::read_from_buffer(&b);
                structure.assign_role(&assignee, &role)?;
                Ok(structure.to_bytes())
            })
            .await?;
        Ok(())
    }

    /// Assigns a resource to a role.
    ///
    /// Resources are identified by a `resource_tag` (e.g., "gcs_bucket") and a
    /// `resource_value` (e.g., "my-data-bucket").
    pub async fn assign_resource_to_role(
        &self,
        role: impl AsRef<str>,
        resource_tag: impl AsRef<str>,
        resource_value: impl AsRef<str>,
    ) -> Result<(), StorageError> {
        let role = Role::new(role);
        self.storage
            .update_file(&self.path, move |b| {
                let mut structure = IxAccessStructureV1::read_from_buffer(&b);
                structure.assign_resource_to_role(&role, resource_tag, resource_value)?;
                Ok(structure.to_bytes())
            })
            .await?;
        Ok(())
    }

    /// Gets all resources with a specific tag that are assigned to a role.
    ///
    /// This includes resources assigned to any roles that the given `role` inherits from.
    pub async fn get_all_resources_for_role_by_tag(
        &self,
        role: impl AsRef<str>,
        tag: impl AsRef<str>,
    ) -> Result<Vec<String>, StorageError> {
        let role = Role::new(role);

        // Check and update if file changed
        let state = self.update_if_changed().await?;

        let resources = state
            .structure
            .get_all_resources_for_role_by_tag(&role, tag)?;

        Ok(resources.map(|s| s.to_string()).collect())
    }

    /// Removes a role assignment.
    ///
    /// This revokes the permissions that `assignee` inherited from `role`.
    pub async fn unassign_role(
        &self,
        assignee: impl AsRef<str>,
        role: impl AsRef<str>,
    ) -> Result<(), StorageError> {
        let assignee = Role::new(assignee);
        let role = Role::new(role);
        self.storage
            .update_file(&self.path, |b| {
                let mut structure = IxAccessStructureV1::read_from_buffer(&b);
                structure.unassign_role(&assignee, &role)?;
                Ok(structure.to_bytes())
            })
            .await?;
        Ok(())
    }

    /// Removes a resource assignment from a role.
    pub async fn unassign_resource_from_role(
        &self,
        role: impl AsRef<str>,
        resource_tag: impl AsRef<str>,
        resource_value: impl AsRef<str>,
    ) -> Result<(), StorageError> {
        let role = Role::new(role);
        self.storage
            .update_file(&self.path, move |b| {
                let mut structure = IxAccessStructureV1::read_from_buffer(&b);
                structure.unassign_resource_from_role(&role, resource_tag, resource_value)?;
                Ok(structure.to_bytes())
            })
            .await?;
        Ok(())
    }
}
