use super::IxAccessClient;
use crate::{
    internals::{IxAccessStructureV1, Role},
    storage::StorageError,
};

impl IxAccessClient {
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
