use std::collections::{HashMap, HashSet, VecDeque};

use crate::internals::error::{Result, StructureError};
use crate::internals::role::Role;
use crate::interner::Index;

use super::bfs::IxAccessStructureV1BFS;
use super::{IxAccessStructureV1, Resource, RoleId};

impl IxAccessStructureV1 {
    pub(crate) fn exists_role(&self, role: &Role) -> bool {
        self.get(role).is_some()
    }

    pub(crate) fn has_role(&self, assignee: &Role, role: &Role) -> bool {
        let Some(assignee_id) = self.get(assignee) else {
            return false;
        };
        let Some(role_id) = self.get(role) else {
            return false;
        };

        for inherited_id in self.bfs(assignee_id) {
            if inherited_id == role_id {
                return true;
            }
        }
        false
    }

    pub(crate) fn has_resource(
        &self,
        role: &Role,
        resource_tag: impl AsRef<str>,
        resource_value: impl AsRef<str>,
    ) -> bool {
        let Some(role_id) = self.get(role) else {
            return false;
        };
        let Some(tag) = self.resource_resolver.get(resource_tag.as_ref()) else {
            return false;
        };
        let Some(value) = self.resource_resolver.get(resource_value.as_ref()) else {
            return false;
        };

        let target = Resource { tag, value };

        for inherited_role_id in self.bfs(role_id) {
            let idx = inherited_role_id.into_index();
            if self.resource_assignment[idx].binary_search(&target).is_ok() {
                return true;
            }
        }
        false
    }

    pub(super) fn bfs(&self, role_id: RoleId) -> IxAccessStructureV1BFS<'_> {
        IxAccessStructureV1BFS {
            structure: self,
            queue: {
                let mut queue = VecDeque::new();
                queue.push_back(role_id);
                queue
            },
            visited: {
                let mut visited = HashSet::new();
                visited.insert(role_id);
                visited
            },
        }
    }

    pub(crate) fn list_roles(&self) -> impl Iterator<Item = &str> {
        self.role_resolver.strings()
    }

    pub(crate) fn list_all_roles_for_role<'a>(
        &'a self,
        role: &Role,
    ) -> Result<impl Iterator<Item = &'a str>> {
        let initial_role_index = self
            .get(role)
            .ok_or_else(|| StructureError::RoleNotFound(role.as_str().to_string()))?;
        let bfs = self.bfs(initial_role_index);
        // SAFETY: This unwrap is safe because role_id comes from our internal BFS traversal
        // over roles that exist in the structure. We maintain the invariant that all role IDs
        // in the graph are valid indices into the role_resolver.
        Ok(bfs.map(|role_id| self.resolve(role_id).unwrap()))
    }

    pub(crate) fn get_all_resources_for_role_by_tag<'a>(
        &'a self,
        role: &Role,
        tag: impl AsRef<str>,
    ) -> Result<impl Iterator<Item = &'a str>> {
        let role_id = self
            .get(role)
            .ok_or_else(|| StructureError::RoleNotFound(role.as_str().to_string()))?;
        let tag_str = tag.as_ref();
        let tag_index = self
            .resource_resolver
            .get(tag_str)
            .ok_or_else(|| StructureError::ResourceTagNotFound(tag_str.to_string()))?;

        Ok(self.bfs(role_id).flat_map(move |r| {
            self.resource_assignment[r.into_index()]
                .iter()
                .filter(move |r| r.tag == tag_index)
                // SAFETY: This unwrap is safe because r.value comes from our internal
                // resource_assignment structure. We maintain the invariant that all resource
                // value indices stored in the structure are valid indices into the resource_resolver.
                .map(|r| self.resource_resolver.resolve(r.value).unwrap())
        }))
    }

    pub(crate) fn list_members_of(&self, role: &Role) -> Result<impl Iterator<Item = &str>> {
        let role_id = self
            .get(role)
            .ok_or_else(|| StructureError::RoleNotFound(role.as_str().to_string()))?;

        let transpose = self.get_transpose_graph();
        let mut members = HashSet::new();
        let mut queue = VecDeque::new();

        // Start from the target role
        queue.push_back(role_id.inner());

        while let Some(current_id) = queue.pop_front() {
            let current_idx = RoleId(current_id).into_index();
            for &parent_id in &transpose[current_idx] {
                if members.insert(parent_id) {
                    queue.push_back(parent_id);
                }
            }
        }

        Ok(members
            .into_iter()
            .filter_map(|id| self.resolve(RoleId(id))))
    }

    pub(crate) fn get_all_resources_for_role(
        &self,
        role: &Role,
    ) -> Result<HashMap<&str, Vec<&str>>> {
        let role_id = self
            .get(role)
            .ok_or_else(|| StructureError::RoleNotFound(role.as_str().to_string()))?;

        let mut all_resources: HashMap<&str, Vec<&str>> = HashMap::new();

        for inherited_role_id in self.bfs(role_id) {
            let idx = inherited_role_id.into_index();
            for resource in &self.resource_assignment[idx] {
                let tag = self.resource_resolver.resolve(resource.tag);
                let value = self.resource_resolver.resolve(resource.value);

                if let (Some(tag), Some(value)) = (tag, value) {
                    all_resources.entry(tag).or_default().push(value);
                }
            }
        }

        // Deduplicate and sort values for each tag
        for values in all_resources.values_mut() {
            values.sort();
            values.dedup();
        }

        Ok(all_resources)
    }

    pub(crate) fn find_roles_with_resource(
        &self,
        resource_tag: impl AsRef<str>,
        resource_value: impl AsRef<str>,
    ) -> Result<Vec<&str>> {
        let tag = self
            .resource_resolver
            .get(resource_tag.as_ref())
            .ok_or_else(|| {
                StructureError::ResourceTagNotFound(resource_tag.as_ref().to_string())
            })?;
        let value = self
            .resource_resolver
            .get(resource_value.as_ref())
            .ok_or_else(|| {
                StructureError::ResourceTagNotFound(resource_value.as_ref().to_string())
            })?;

        let target_resource = Resource { tag, value };
        let mut direct_roles = Vec::new();

        for (idx, assignments) in self.resource_assignment.iter().enumerate() {
            if assignments.binary_search(&target_resource).is_ok() {
                direct_roles.push(Index::from_usize_index(idx));
            }
        }

        self.expand_roles_to_members(direct_roles)
    }

    pub(crate) fn find_roles_with_resource_tag(
        &self,
        resource_tag: impl AsRef<str>,
    ) -> Result<Vec<&str>> {
        let tag = self
            .resource_resolver
            .get(resource_tag.as_ref())
            .ok_or_else(|| {
                StructureError::ResourceTagNotFound(resource_tag.as_ref().to_string())
            })?;

        let mut direct_roles = Vec::new();

        for (idx, assignments) in self.resource_assignment.iter().enumerate() {
            if assignments.iter().any(|r| r.tag == tag) {
                direct_roles.push(Index::from_usize_index(idx));
            }
        }

        self.expand_roles_to_members(direct_roles)
    }

    fn expand_roles_to_members(&self, starting_roles: Vec<Index>) -> Result<Vec<&str>> {
        let transpose = self.get_transpose_graph();
        let mut all_members = HashSet::new();
        let mut queue = VecDeque::new();

        for role_id in starting_roles {
            if all_members.insert(role_id) {
                queue.push_back(role_id);
            }
        }

        while let Some(current_id) = queue.pop_front() {
            let current_idx = RoleId(current_id).into_index();
            for &parent_id in &transpose[current_idx] {
                if all_members.insert(parent_id) {
                    queue.push_back(parent_id);
                }
            }
        }

        let mut result: Vec<&str> = all_members
            .into_iter()
            .filter_map(|id| self.resolve(RoleId(id)))
            .collect();
        result.sort();
        Ok(result)
    }

    fn get_transpose_graph(&self) -> Vec<Vec<Index>> {
        let mut transpose = vec![Vec::new(); self.role_graph.len()];
        for (u_idx, edges) in self.role_graph.iter().enumerate() {
            let u_id = Index::from_usize_index(u_idx);
            for &v_id in edges {
                let v_idx = RoleId(v_id).into_index();
                transpose[v_idx].push(u_id);
            }
        }
        transpose
    }
}
