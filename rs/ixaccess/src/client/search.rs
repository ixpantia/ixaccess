use std::collections::HashMap;

use super::IxAccessClient;
use crate::{internals::Role, storage::StorageError};

impl IxAccessClient {
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

    /// Lists all roles that inherit from the given role.
    pub async fn list_members_of(
        &self,
        role: impl AsRef<str>,
    ) -> Result<Vec<String>, StorageError> {
        // Check and update if file changed
        let state = self.update_if_changed().await?;

        let role = Role::new(role);
        let members = state
            .structure
            .list_members_of(&role)
            .map_err(|e| StorageError::InvalidPath(e.to_string()))?;
        Ok(members.map(|s| s.to_string()).collect())
    }

    /// Checks if a role exists.
    pub async fn exists_role(&self, role: impl AsRef<str>) -> Result<bool, StorageError> {
        let state = self.update_if_changed().await?;
        let role = Role::new(role);
        Ok(state.structure.exists_role(&role))
    }

    /// Checks if an assignee has been granted a specific role, directly or indirectly.
    pub async fn has_role(
        &self,
        assignee: impl AsRef<str>,
        role: impl AsRef<str>,
    ) -> Result<bool, StorageError> {
        let state = self.update_if_changed().await?;
        let assignee = Role::new(assignee);
        let role = Role::new(role);
        Ok(state.structure.has_role(&assignee, &role))
    }

    /// Checks if a role (or any role it inherits from) has access to a specific resource.
    pub async fn has_resource(
        &self,
        role: impl AsRef<str>,
        resource_tag: impl AsRef<str>,
        resource_value: impl AsRef<str>,
    ) -> Result<bool, StorageError> {
        let state = self.update_if_changed().await?;
        let role = Role::new(role);
        Ok(state
            .structure
            .has_resource(&role, resource_tag, resource_value))
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

    /// Gets all resources assigned to a role, grouped by tag.
    pub async fn get_all_resources_for_role(
        &self,
        role: impl AsRef<str>,
    ) -> Result<HashMap<String, Vec<String>>, StorageError> {
        let role = Role::new(role);

        // Check and update if file changed
        let state = self.update_if_changed().await?;

        let resources = state.structure.get_all_resources_for_role(&role)?;
        Ok(resources
            .into_iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    v.into_iter().map(|s| s.to_string()).collect(),
                )
            })
            .collect())
    }

    /// Finds all roles that have been assigned a specific resource.
    pub async fn find_roles_with_resource(
        &self,
        tag: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> Result<Vec<String>, StorageError> {
        // Check and update if file changed
        let state = self.update_if_changed().await?;

        let roles = state.structure.find_roles_with_resource(tag, value)?;
        Ok(roles.into_iter().map(|s| s.to_string()).collect())
    }

    /// Finds all roles that have any resource with a specific tag.
    pub async fn find_roles_with_resource_tag(
        &self,
        tag: impl AsRef<str>,
    ) -> Result<Vec<String>, StorageError> {
        // Check and update if file changed
        let state = self.update_if_changed().await?;

        let roles = state.structure.find_roles_with_resource_tag(tag)?;
        Ok(roles.into_iter().map(|s| s.to_string()).collect())
    }
}
