use crate::internals::error::Result;
use crate::internals::error::StructureError;
use crate::internals::role::Role;
use crate::internals::structure::IxAccessStructureV1;

impl IxAccessStructureV1 {
    pub(crate) fn add_role(&mut self, role: &Role) {
        let count_before = self.role_resolver.len();
        self.role_resolver.get_or_intern(role.as_str());
        if count_before == self.role_resolver.len() {
            return;
        }
        self.role_graph.push(Vec::new());
        self.resource_assignment.push(Vec::new());
        assert_eq!(self.role_resolver.len(), self.role_graph.len());
    }

    pub(crate) fn assign_role(&mut self, assignee: &Role, role: &Role) -> Result<()> {
        let assignee_role_id = self
            .get(assignee)
            .ok_or_else(|| StructureError::RoleNotFound(assignee.as_str().to_string()))?;
        let role_id = self
            .get(role)
            .ok_or_else(|| StructureError::RoleNotFound(role.as_str().to_string()))?;
        let roles = &mut self.role_graph[assignee_role_id.into_index()];
        roles.push(role_id.inner());
        roles.sort();
        roles.dedup();
        Ok(())
    }

    pub(crate) fn unassign_role(&mut self, assignee: &Role, role: &Role) -> Result<()> {
        let assignee_role_id = self
            .get(assignee)
            .ok_or_else(|| StructureError::RoleNotFound(assignee.as_str().to_string()))?;
        let role_id = self
            .get(role)
            .ok_or_else(|| StructureError::RoleNotFound(role.as_str().to_string()))?;
        let roles = &mut self.role_graph[assignee_role_id.into_index()];
        roles.retain(|&r| r != role_id.inner()); // This should stay sorted
        Ok(())
    }
}
