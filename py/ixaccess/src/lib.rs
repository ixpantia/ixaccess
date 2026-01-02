use pyo3::exceptions::{PyException, PyRuntimeError};
use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

// Import the Rust library
use ::ixaccess::IxAccessClient as RustIxAccessClient;

/// Python wrapper for IxAccessClient
#[pyclass]
struct IxAccessClient {
    inner: Arc<RustIxAccessClient>,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl IxAccessClient {
    /// Creates a new IxAccessClient instance.
    ///
    /// Args:
    ///     path: A URL-style path to the state file in a supported object store
    ///           (e.g., "gs://my-bucket/access-control.ix").
    ///
    /// Example:
    ///     >>> client = IxAccessClient("gs://your-bucket/access-control.ix")
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        let client = runtime.block_on(async { RustIxAccessClient::new(path).await });

        Ok(IxAccessClient {
            inner: Arc::new(client),
            runtime,
        })
    }

    /// Lists all top-level roles.
    ///
    /// Returns:
    ///     A list of role names.
    ///
    /// Example:
    ///     >>> roles = client.list_roles()
    ///     >>> print(roles)
    ///     ['admin', 'editor', 'viewer']
    fn list_roles(&self) -> PyResult<Vec<String>> {
        self.runtime.block_on(async {
            self.inner
                .list_roles()
                .await
                .map_err(|e| PyException::new_err(format!("Failed to list roles: {}", e)))
        })
    }

    /// Lists all roles assigned to a given role, including nested roles.
    ///
    /// Args:
    ///     role: The role name to query.
    ///
    /// Returns:
    ///     A list of role names that the specified role has been granted.
    ///
    /// Example:
    ///     >>> roles = client.list_all_roles_for_role("admin")
    ///     >>> print(roles)
    ///     ['editor', 'viewer']
    fn list_all_roles_for_role(&self, role: &str) -> PyResult<Vec<String>> {
        self.runtime.block_on(async {
            self.inner
                .list_all_roles_for_role(role)
                .await
                .map_err(|e| PyException::new_err(format!("Failed to list roles for role: {}", e)))
        })
    }

    /// Lists all roles that inherit from the given role.
    ///
    /// Args:
    ///     role: The role name to query.
    ///
    /// Returns:
    ///     A list of role names that inherit from the specified role.
    ///
    /// Example:
    ///     >>> members = client.list_members_of("viewer")
    fn list_members_of(&self, role: &str) -> PyResult<Vec<String>> {
        self.runtime.block_on(async {
            self.inner
                .list_members_of(role)
                .await
                .map_err(|e| PyException::new_err(format!("Failed to list members of role: {}", e)))
        })
    }

    /// Checks if a role exists.
    fn exists_role(&self, role: &str) -> PyResult<bool> {
        self.runtime.block_on(async {
            self.inner
                .exists_role(role)
                .await
                .map_err(|e| PyException::new_err(format!("Failed to check if role exists: {}", e)))
        })
    }

    /// Checks if an assignee has been granted a specific role, directly or indirectly.
    fn has_role(&self, assignee: &str, role: &str) -> PyResult<bool> {
        self.runtime.block_on(async {
            self.inner.has_role(assignee, role).await.map_err(|e| {
                PyException::new_err(format!("Failed to check role assignment: {}", e))
            })
        })
    }

    /// Checks if a role (or any role it inherits from) has access to a specific resource.
    fn has_resource(&self, role: &str, tag: &str, value: &str) -> PyResult<bool> {
        self.runtime.block_on(async {
            self.inner
                .has_resource(role, tag, value)
                .await
                .map_err(|e| {
                    PyException::new_err(format!("Failed to check resource access: {}", e))
                })
        })
    }

    /// Adds a new role to the access control structure.
    ///
    /// Args:
    ///     role: The name of the role to add.
    ///
    /// Example:
    ///     >>> client.add_role("admin")
    fn add_role(&self, role: &str) -> PyResult<()> {
        self.runtime.block_on(async {
            self.inner
                .add_role(role)
                .await
                .map_err(|e| PyException::new_err(format!("Failed to add role: {}", e)))
        })
    }

    /// Adds multiple roles at once.
    ///
    /// Args:
    ///     roles: A list of role names to add.
    ///
    /// Example:
    ///     >>> client.add_roles(["admin", "editor", "viewer"])
    fn add_roles(&self, roles: Vec<String>) -> PyResult<()> {
        self.runtime.block_on(async {
            self.inner
                .add_roles(roles)
                .await
                .map_err(|e| PyException::new_err(format!("Failed to add roles: {}", e)))
        })
    }

    /// Assigns a role to another role (the assignee).
    ///
    /// This creates a parent-child relationship in the role hierarchy.
    ///
    /// Args:
    ///     assignee: The role that will inherit permissions.
    ///     role: The role whose permissions will be inherited.
    ///
    /// Example:
    ///     >>> client.assign_role("admin", "editor")
    fn assign_role(&self, assignee: &str, role: &str) -> PyResult<()> {
        self.runtime.block_on(async {
            self.inner
                .assign_role(assignee, role)
                .await
                .map_err(|e| PyException::new_err(format!("Failed to assign role: {}", e)))
        })
    }

    /// Assigns a resource to a role.
    ///
    /// Args:
    ///     role: The role to assign the resource to.
    ///     resource_tag: The type/tag of the resource (e.g., "gcs_bucket").
    ///     resource_value: The specific resource identifier (e.g., "my-data-bucket").
    ///
    /// Example:
    ///     >>> client.assign_resource_to_role("viewer", "gcs_bucket", "data-bucket-1")
    fn assign_resource_to_role(
        &self,
        role: &str,
        resource_tag: &str,
        resource_value: &str,
    ) -> PyResult<()> {
        self.runtime.block_on(async {
            self.inner
                .assign_resource_to_role(role, resource_tag, resource_value)
                .await
                .map_err(|e| {
                    PyException::new_err(format!("Failed to assign resource to role: {}", e))
                })
        })
    }

    /// Gets all resources with a specific tag that are assigned to a role.
    ///
    /// This includes resources assigned to any roles that the given role inherits from.
    ///
    /// Args:
    ///     role: The role to query.
    ///     tag: The resource tag to filter by.
    ///
    /// Returns:
    ///     A list of resource values.
    ///
    /// Example:
    ///     >>> buckets = client.get_all_resources_for_role_by_tag("admin", "gcs_bucket")
    ///     >>> print(buckets)
    ///     ['data-bucket-1', 'logs-bucket-2']
    fn get_all_resources_for_role_by_tag(&self, role: &str, tag: &str) -> PyResult<Vec<String>> {
        self.runtime.block_on(async {
            self.inner
                .get_all_resources_for_role_by_tag(role, tag)
                .await
                .map_err(|e| {
                    PyException::new_err(format!("Failed to get resources for role: {}", e))
                })
        })
    }

    /// Gets all resources assigned to a role, grouped by tag.
    ///
    /// Args:
    ///     role: The role name to query.
    ///
    /// Returns:
    ///     A dictionary where keys are tags and values are lists of resource values.
    fn get_all_resources_for_role(&self, role: &str) -> PyResult<HashMap<String, Vec<String>>> {
        self.runtime.block_on(async {
            self.inner
                .get_all_resources_for_role(role)
                .await
                .map_err(|e| {
                    PyException::new_err(format!("Failed to get all resources for role: {}", e))
                })
        })
    }

    /// Finds all roles that have been assigned a specific resource.
    ///
    /// Args:
    ///     tag: The resource tag.
    ///     value: The resource value.
    fn find_roles_with_resource(&self, tag: &str, value: &str) -> PyResult<Vec<String>> {
        self.runtime.block_on(async {
            self.inner
                .find_roles_with_resource(tag, value)
                .await
                .map_err(|e| {
                    PyException::new_err(format!("Failed to find roles with resource: {}", e))
                })
        })
    }

    /// Finds all roles that have any resource with a specific tag.
    ///
    /// Args:
    ///     tag: The resource tag to search for.
    fn find_roles_with_resource_tag(&self, tag: &str) -> PyResult<Vec<String>> {
        self.runtime.block_on(async {
            self.inner
                .find_roles_with_resource_tag(tag)
                .await
                .map_err(|e| {
                    PyException::new_err(format!("Failed to find roles with resource tag: {}", e))
                })
        })
    }

    /// Removes a role assignment.
    ///
    /// Args:
    ///     assignee: The role to remove the assignment from.
    ///     role: The role being unassigned.
    ///
    /// Example:
    ///     >>> client.unassign_role("admin", "editor")
    fn unassign_role(&self, assignee: &str, role: &str) -> PyResult<()> {
        self.runtime.block_on(async {
            self.inner
                .unassign_role(assignee, role)
                .await
                .map_err(|e| PyException::new_err(format!("Failed to unassign role: {}", e)))
        })
    }

    /// Removes a resource assignment from a role.
    ///
    /// Args:
    ///     role: The role to remove the resource from.
    ///     resource_tag: The type/tag of the resource.
    ///     resource_value: The specific resource identifier.
    ///
    /// Example:
    ///     >>> client.unassign_resource_from_role("viewer", "gcs_bucket", "data-bucket-1")
    fn unassign_resource_from_role(
        &self,
        role: &str,
        resource_tag: &str,
        resource_value: &str,
    ) -> PyResult<()> {
        self.runtime.block_on(async {
            self.inner
                .unassign_resource_from_role(role, resource_tag, resource_value)
                .await
                .map_err(|e| {
                    PyException::new_err(format!("Failed to unassign resource from role: {}", e))
                })
        })
    }
}

/// IxAccess Python module - A library for managing access control roles and resources.
#[pymodule]
#[pyo3(name = "ixaccess")]
fn ixaccess_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<IxAccessClient>()?;
    Ok(())
}
