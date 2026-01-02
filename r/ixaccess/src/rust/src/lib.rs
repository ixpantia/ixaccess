use extendr_api::prelude::*;
use ixaccess::IxAccessClient as InnerClient;
use tokio::runtime::Runtime;

// Thread-local runtime for async operations
thread_local! {
    static RUNTIME: Runtime = Runtime::new().expect("Failed to create Tokio runtime");
}

/// Execute an async block in the thread-local runtime
fn block_on<F>(future: F) -> F::Output
where
    F: std::future::Future,
{
    RUNTIME.with(|rt| rt.block_on(future))
}

#[extendr]
pub struct IxAccessClientInternal {
    inner: InnerClient,
}

#[extendr]
impl IxAccessClientInternal {
    fn new(path: &str) -> Result<Self> {
        let inner = block_on(InnerClient::new(path));
        Ok(Self { inner })
    }

    fn list_roles(&self) -> Result<Vec<String>> {
        Ok(block_on(self.inner.list_roles())?)
    }
    fn list_all_roles_for_role(&self, role: &str) -> Result<Vec<String>> {
        Ok(block_on(self.inner.list_all_roles_for_role(role))?)
    }

    fn list_members_of(&self, role: &str) -> Result<Vec<String>> {
        Ok(block_on(self.inner.list_members_of(role))?)
    }

    fn exists_role(&self, role: &str) -> Result<bool> {
        Ok(block_on(self.inner.exists_role(role))?)
    }

    fn has_role(&self, assignee: &str, role: &str) -> Result<bool> {
        Ok(block_on(self.inner.has_role(assignee, role))?)
    }

    fn has_resource(&self, role: &str, tag: &str, value: &str) -> Result<bool> {
        Ok(block_on(self.inner.has_resource(role, tag, value))?)
    }

    fn add_role(&self, role: &str) -> Result<()> {
        Ok(block_on(self.inner.add_role(role))?)
    }

    fn add_roles(&self, roles: Vec<String>) -> Result<()> {
        Ok(block_on(self.inner.add_roles(roles))?)
    }

    fn assign_role(&self, assignee: &str, role: &str) -> Result<()> {
        Ok(block_on(self.inner.assign_role(assignee, role))?)
    }

    fn assign_resource_to_role(
        &self,
        role: &str,
        resource_tag: &str,
        resource_value: &str,
    ) -> Result<()> {
        Ok(block_on(self.inner.assign_resource_to_role(
            role,
            resource_tag,
            resource_value,
        ))?)
    }

    fn get_all_resources_for_role_by_tag(&self, role: &str, tag: &str) -> Result<Vec<String>> {
        Ok(block_on(
            self.inner.get_all_resources_for_role_by_tag(role, tag),
        )?)
    }

    fn get_all_resources_for_role(&self, role: &str) -> Result<List> {
        let res = block_on(self.inner.get_all_resources_for_role(role))?;
        let names: Vec<String> = res.keys().cloned().collect();
        let values: Vec<Robj> = res.values().cloned().map(|v| v.into_robj()).collect();
        Ok(List::from_names_and_values(names, values)?)
    }

    fn find_roles_with_resource(&self, tag: &str, value: &str) -> Result<Vec<String>> {
        Ok(block_on(self.inner.find_roles_with_resource(tag, value))?)
    }

    fn find_roles_with_resource_tag(&self, tag: &str) -> Result<Vec<String>> {
        Ok(block_on(self.inner.find_roles_with_resource_tag(tag))?)
    }

    fn unassign_role(&self, assignee: &str, role: &str) -> Result<()> {
        Ok(block_on(self.inner.unassign_role(assignee, role))?)
    }

    fn unassign_resource_from_role(
        &self,
        role: &str,
        resource_tag: &str,
        resource_value: &str,
    ) -> Result<()> {
        Ok(block_on(self.inner.unassign_resource_from_role(
            role,
            resource_tag,
            resource_value,
        ))?)
    }
}

// Macro to generate exports.
// This ensures exported functions are registered with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod ixaccess;
    impl IxAccessClientInternal;
}
