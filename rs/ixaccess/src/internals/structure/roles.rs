use std::collections::HashMap;

use crate::internals::error::Result;
use crate::internals::error::StructureError;
use crate::internals::role::Role;
use crate::internals::structure::IxAccessStructureV1;
use crate::interner::Interner;

use super::RoleId;

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

    pub(crate) fn delete_role(&mut self, role_to_delete: &Role) {
        let target_id = match self.get(role_to_delete) {
            Some(id) => id,
            None => return,
        };
        let target_index = target_id.inner();

        let mut new_state = IxAccessStructureV1::new();
        // Move the resource resolver to the new state
        new_state.resource_resolver =
            std::mem::replace(&mut self.resource_resolver, Interner::new());

        // Map old Index to new Index
        let mut id_mapping = HashMap::new();

        // Step 4: Re-intern Roles (except the deleted one)
        for (old_idx_zero, role_name) in self.role_resolver.strings().enumerate() {
            let old_index = crate::interner::Index::from_usize_index(old_idx_zero);
            if old_index == target_index {
                continue;
            }

            let role = Role::new(role_name);
            new_state.add_role(&role);
            let new_id = new_state.get(&role).expect("Just added role must exist");
            id_mapping.insert(old_index, new_id.inner());
        }

        // Step 5: Re-map Inheritance
        for (old_idx_zero, old_edges) in self.role_graph.iter().enumerate() {
            let old_index = crate::interner::Index::from_usize_index(old_idx_zero);
            let Some(&new_index) = id_mapping.get(&old_index) else {
                continue;
            };

            let new_idx_zero = RoleId(new_index).into_index();
            for &child_old_index in old_edges {
                if let Some(&child_new_index) = id_mapping.get(&child_old_index) {
                    new_state.role_graph[new_idx_zero].push(child_new_index);
                }
            }
            new_state.role_graph[new_idx_zero].sort();
        }

        // Step 6: Re-map Resource Assignments
        for (old_idx_zero, old_resources) in self.resource_assignment.iter_mut().enumerate() {
            let old_index = crate::interner::Index::from_usize_index(old_idx_zero);
            if let Some(&new_index) = id_mapping.get(&old_index) {
                let new_idx_zero = RoleId(new_index).into_index();
                new_state.resource_assignment[new_idx_zero] = std::mem::take(old_resources);
            }
        }

        // Step 7: Swap State
        *self = new_state;
    }
}
