use super::IxAccessClient;
use crate::{
    internals::{IxAccessStructureV1, Role},
    storage::StorageError,
};

impl IxAccessClient {
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

    /// Deletes a role from the access control structure.
    ///
    /// This removes the role, all its assignments to other roles, and all its
    /// resource assignments.
    pub async fn delete_role(&self, role: impl AsRef<str>) -> Result<(), StorageError> {
        let role = Role::new(role);
        self.storage
            .update_file(&self.path, |b| {
                let mut structure = IxAccessStructureV1::read_from_buffer(&b);
                structure.delete_role(&role);
                Ok(structure.to_bytes())
            })
            .await?;
        Ok(())
    }
}
