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
