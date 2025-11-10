use arc_swap::ArcSwap;

use object_store::path::Path;
use std::sync::Arc;

use crate::{
    internals::{IxAccessStructureV1, Role},
    storage::{Storage, StorageError},
};

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

    pub async fn list_roles(&self) -> Result<Vec<String>, StorageError> {
        // Check and update if file changed
        let state = self.update_if_changed().await?;

        Ok(state
            .structure
            .list_roles()
            .map(|s| s.to_string())
            .collect())
    }

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

    pub async fn get_all_resources_for_role_by_tag(
        &self,
        role: impl AsRef<str>,
        tag: impl AsRef<str>,
    ) -> Result<Vec<String>, StorageError> {
        let role = Role::new(role);

        // Check and update if file changed
        let state = self.update_if_changed().await?;

        Ok(state
            .structure
            .get_all_resources_for_role_by_tag(&role, tag)?
            .map(|s| s.to_string())
            .collect())
    }

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
