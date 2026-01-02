use std::collections::{HashMap, HashSet, VecDeque};

use bincode::config::{Fixint, LittleEndian, NoLimit};
use bytes::Bytes;

use crate::interner::{Index, Interner};

use super::bfs::IxAccessStructureV1BFS;
use super::error::{Result, StructureError};
use super::header::{IxAccessFileHeader, Version, HEADER_SIZE, HEADER_V1};
use super::resource::Resource;
use super::role::Role;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct RoleId(pub Index);

impl RoleId {
    pub fn into_index(self) -> usize {
        // This is non zero so this is safe (no overflow)
        self.0.into_usize() - 1
    }
    #[inline(always)]
    pub fn inner(self) -> Index {
        self.0
    }
}

#[derive(Debug)]
pub(crate) struct IxAccessStructureV1 {
    pub(super) role_resolver: crate::interner::Interner,
    pub(super) resource_resolver: crate::interner::Interner,
    pub(super) role_graph: Vec<Vec<Index>>,
    pub(super) resource_assignment: Vec<Vec<Resource>>,
}

impl bincode::Encode for IxAccessStructureV1 {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> std::result::Result<(), bincode::error::EncodeError> {
        bincode::Encode::encode(&self.role_resolver, encoder)?;
        bincode::Encode::encode(&self.resource_resolver, encoder)?;
        bincode::Encode::encode(&self.role_graph, encoder)?;
        bincode::Encode::encode(&self.resource_assignment, encoder)?;
        Ok(())
    }
}

impl<Context> bincode::Decode<Context> for IxAccessStructureV1 {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> std::result::Result<Self, bincode::error::DecodeError> {
        let role_resolver = bincode::Decode::decode(decoder)?;
        let resource_resolver = bincode::Decode::decode(decoder)?;
        let role_graph = bincode::Decode::decode(decoder)?;
        let resource_assignment = bincode::Decode::decode(decoder)?;
        Ok(Self {
            role_resolver,
            resource_resolver,
            role_graph,
            resource_assignment,
        })
    }
}

const BINCODE_CONFIG: bincode::config::Configuration<LittleEndian, Fixint, NoLimit> = {
    bincode::config::standard()
        .with_little_endian()
        .with_fixed_int_encoding()
        .with_no_limit()
};

impl IxAccessStructureV1 {
    pub(crate) fn read_from_buffer(buffer: &[u8]) -> Self {
        let header = IxAccessFileHeader::from_bytes(buffer);
        match header.version {
            Version(1) => {
                bincode::decode_from_slice(&buffer[HEADER_SIZE..], BINCODE_CONFIG)
                    .unwrap()
                    .0
            }
            Version(unsupported) => panic!("Unsupported version: {unsupported}."),
        }
    }

    pub(crate) fn to_bytes(&self) -> Bytes {
        use std::io::Write;
        let mut buffer = Vec::new();
        buffer.write(&HEADER_V1.to_bytes()).unwrap();
        bincode::encode_into_std_write(self, &mut buffer, BINCODE_CONFIG).unwrap();
        Bytes::from(buffer)
    }

    pub(crate) fn new() -> Self {
        Self {
            role_resolver: Interner::new(),
            resource_resolver: Interner::new(),
            role_graph: Default::default(),
            resource_assignment: Default::default(),
        }
    }

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

    #[inline]
    fn get(&self, role: &Role) -> Option<RoleId> {
        self.role_resolver.get(role.as_str()).map(RoleId)
    }

    #[inline]
    fn resolve(&self, role_id: RoleId) -> Option<&str> {
        self.role_resolver.resolve(role_id.0)
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

    fn get_or_create_resource(
        &mut self,
        resource_tag: impl AsRef<str>,
        resource_value: impl AsRef<str>,
    ) -> Resource {
        let tag = self.resource_resolver.get_or_intern(resource_tag);
        let value = self.resource_resolver.get_or_intern(resource_value);
        Resource { tag, value }
    }

    // Los recursos (resources) tienen la diferencia fundamental a los roles
    // de que no tienen que ser case insenstive, ni ascii. Un rescurso
    // podría ser un URL, un path a un archivo, el nombre de un reporte.
    pub(crate) fn assign_resource_to_role(
        &mut self,
        role: &Role,
        resource_tag: impl AsRef<str>,
        resource_value: impl AsRef<str>,
    ) -> Result<()> {
        let role_id = self
            .get(role)
            .ok_or_else(|| StructureError::RoleNotFound(role.as_str().to_string()))?;
        let resource = self.get_or_create_resource(resource_tag, resource_value);

        let resource_assignment = &mut self.resource_assignment[role_id.into_index()];
        resource_assignment.push(resource);
        resource_assignment.sort();
        resource_assignment.dedup();
        Ok(())
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

    pub(crate) fn unassign_resource_from_role(
        &mut self,
        role: &Role,
        resource_tag: impl AsRef<str>,
        resource_value: impl AsRef<str>,
    ) -> Result<()> {
        let role_id = self
            .get(role)
            .ok_or_else(|| StructureError::RoleNotFound(role.as_str().to_string()))?;

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

        let resource = Resource { tag, value };
        let resource_assignment = &mut self.resource_assignment[role_id.into_index()];
        resource_assignment.retain(|&r| r != resource); // This should stay sorted
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::TestResult;

    #[cfg(test)]
    use crate::internals::role::tests::AsciiRole;

    // Helper to create a test structure with some roles
    fn create_test_structure() -> IxAccessStructureV1 {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("admin"));
        structure.add_role(&Role::new("user"));
        structure.add_role(&Role::new("guest"));
        structure
    }

    #[test]
    fn test_add_role_single() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("admin"));

        let roles: Vec<_> = structure.list_roles().collect();
        assert_eq!(roles.len(), 1);
        assert!(roles.contains(&"admin"));
    }

    #[test]
    fn test_add_role_multiple() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("admin"));
        structure.add_role(&Role::new("user"));
        structure.add_role(&Role::new("guest"));

        let roles: Vec<_> = structure.list_roles().collect();
        assert_eq!(roles.len(), 3);
        assert!(roles.contains(&"admin"));
        assert!(roles.contains(&"user"));
        assert!(roles.contains(&"guest"));
    }

    #[test]
    fn test_add_role_duplicate() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("admin"));
        structure.add_role(&Role::new("admin"));

        let roles: Vec<_> = structure.list_roles().collect();
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], "admin");
    }

    #[test]
    fn test_add_role_case_insensitive_duplicate() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("admin"));
        structure.add_role(&Role::new("ADMIN"));
        structure.add_role(&Role::new("Admin"));

        let roles: Vec<_> = structure.list_roles().collect();
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], "admin");
    }

    #[test]
    fn test_add_role_empty_string() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new(""));

        let roles: Vec<_> = structure.list_roles().collect();
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], "");
    }

    #[test]
    fn test_add_role_special_characters() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("role-with-dash"));
        structure.add_role(&Role::new("role_with_underscore"));
        structure.add_role(&Role::new("role.with.dot"));
        structure.add_role(&Role::new("role123"));

        let roles: Vec<_> = structure.list_roles().collect();
        assert_eq!(roles.len(), 4);
    }

    #[test]
    fn test_add_role_whitespace() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("role with space"));
        structure.add_role(&Role::new("role\twith\ttab"));

        let roles: Vec<_> = structure.list_roles().collect();
        assert_eq!(roles.len(), 2);
    }

    #[test]
    fn test_assign_role_simple() {
        let mut structure = create_test_structure();
        structure
            .assign_role(&Role::new("user"), &Role::new("guest"))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("user"))
            .unwrap()
            .collect();
        assert!(roles.contains(&"user"));
        assert!(roles.contains(&"guest"));
        assert_eq!(roles.len(), 2);
    }

    #[test]
    fn test_assign_role_chain() {
        let mut structure = create_test_structure();
        structure
            .assign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();
        structure
            .assign_role(&Role::new("user"), &Role::new("guest"))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();
        assert!(roles.contains(&"admin"));
        assert!(roles.contains(&"user"));
        assert!(roles.contains(&"guest"));
        assert_eq!(roles.len(), 3);
    }

    #[test]
    fn test_assign_role_multiple_to_one() {
        let mut structure = create_test_structure();
        structure
            .assign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();
        structure
            .assign_role(&Role::new("admin"), &Role::new("guest"))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();
        assert!(roles.contains(&"admin"));
        assert!(roles.contains(&"user"));
        assert!(roles.contains(&"guest"));
        assert_eq!(roles.len(), 3);
    }

    #[test]
    fn test_assign_role_duplicate() {
        let mut structure = create_test_structure();
        structure
            .assign_role(&Role::new("user"), &Role::new("guest"))
            .unwrap();
        structure
            .assign_role(&Role::new("user"), &Role::new("guest"))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("user"))
            .unwrap()
            .collect();
        assert_eq!(roles.len(), 2);
    }

    #[test]
    fn test_assign_role_diamond_hierarchy() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("top"));
        structure.add_role(&Role::new("left"));
        structure.add_role(&Role::new("right"));
        structure.add_role(&Role::new("bottom"));

        structure
            .assign_role(&Role::new("top"), &Role::new("left"))
            .unwrap();
        structure
            .assign_role(&Role::new("top"), &Role::new("right"))
            .unwrap();
        structure
            .assign_role(&Role::new("left"), &Role::new("bottom"))
            .unwrap();
        structure
            .assign_role(&Role::new("right"), &Role::new("bottom"))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("top"))
            .unwrap()
            .collect();
        assert!(roles.contains(&"top"));
        assert!(roles.contains(&"left"));
        assert!(roles.contains(&"right"));
        assert!(roles.contains(&"bottom"));
        assert_eq!(roles.len(), 4);
    }

    #[test]
    fn test_assign_role_self() {
        let mut structure = create_test_structure();
        structure
            .assign_role(&Role::new("admin"), &Role::new("admin"))
            .unwrap();

        // Self-assignment should be deduplicated
        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], "admin");
    }

    #[test]
    fn test_assign_role_nonexistent_assignee() {
        let mut structure = create_test_structure();
        let result = structure.assign_role(&Role::new("nonexistent"), &Role::new("guest"));
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), StructureError::RoleNotFound(ref s) if s == "nonexistent")
        );
    }

    #[test]
    fn test_assign_role_nonexistent_role() {
        let mut structure = create_test_structure();
        let result = structure.assign_role(&Role::new("admin"), &Role::new("nonexistent"));
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), StructureError::RoleNotFound(ref s) if s == "nonexistent")
        );
    }

    #[test]
    fn test_assign_role_case_insensitive() {
        let mut structure = create_test_structure();
        structure
            .assign_role(&Role::new("ADMIN"), &Role::new("USER"))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();
        assert!(roles.contains(&"admin"));
        assert!(roles.contains(&"user"));
    }

    #[test]
    fn test_list_all_roles_for_role_single() {
        let structure = create_test_structure();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], "admin");
    }

    #[test]
    fn test_list_all_roles_for_role_with_assignments() {
        let mut structure = create_test_structure();
        structure
            .assign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();
        assert!(roles.contains(&"admin"));
        assert!(roles.contains(&"user"));
        assert_eq!(roles.len(), 2);
    }

    #[test]
    fn test_list_all_roles_for_role_transitive() {
        let mut structure = create_test_structure();
        structure
            .assign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();
        structure
            .assign_role(&Role::new("user"), &Role::new("guest"))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();
        assert_eq!(roles.len(), 3);
        assert!(roles.contains(&"admin"));
        assert!(roles.contains(&"user"));
        assert!(roles.contains(&"guest"));
    }

    #[test]
    fn test_list_all_roles_for_role_nonexistent() {
        let structure = create_test_structure();
        let role = Role::new("nonexistent");
        let result = structure.list_all_roles_for_role(&role);
        assert!(result.is_err());
        match result {
            Err(StructureError::RoleNotFound(s)) => assert_eq!(s, "nonexistent"),
            _ => panic!("Expected RoleNotFound error"),
        }
    }

    #[test]
    fn test_list_all_roles_for_role_case_insensitive() {
        let mut structure = create_test_structure();
        structure
            .assign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("ADMIN"))
            .unwrap()
            .collect();
        assert_eq!(roles.len(), 2);
    }

    #[test]
    fn test_list_roles_empty() {
        let structure = IxAccessStructureV1::new();
        let roles: Vec<_> = structure.list_roles().collect();
        assert_eq!(roles.len(), 0);
    }

    #[test]
    fn test_list_roles_order() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("zebra"));
        structure.add_role(&Role::new("admin"));
        structure.add_role(&Role::new("moderator"));

        let roles: Vec<_> = structure.list_roles().collect();
        assert_eq!(roles.len(), 3);
        // Order depends on interner implementation, just check all present
        assert!(roles.contains(&"zebra"));
        assert!(roles.contains(&"admin"));
        assert!(roles.contains(&"moderator"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut structure = create_test_structure();
        structure
            .assign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();
        structure
            .assign_role(&Role::new("user"), &Role::new("guest"))
            .unwrap();

        let bytes = structure.to_bytes();
        let deserialized = IxAccessStructureV1::read_from_buffer(&bytes);

        let roles1: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();
        let roles2: Vec<_> = deserialized
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();

        assert_eq!(roles1, roles2);
    }

    #[test]
    fn test_serialization_empty() {
        let structure = IxAccessStructureV1::new();
        let bytes = structure.to_bytes();
        let deserialized = IxAccessStructureV1::read_from_buffer(&bytes);

        let roles: Vec<_> = deserialized.list_roles().collect();
        assert_eq!(roles.len(), 0);
    }

    #[test]
    fn test_serialization_preserves_hierarchy() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("a"));
        structure.add_role(&Role::new("b"));
        structure.add_role(&Role::new("c"));
        structure
            .assign_role(&Role::new("a"), &Role::new("b"))
            .unwrap();
        structure
            .assign_role(&Role::new("b"), &Role::new("c"))
            .unwrap();

        let bytes = structure.to_bytes();
        let deserialized = IxAccessStructureV1::read_from_buffer(&bytes);

        let roles: Vec<_> = deserialized
            .list_all_roles_for_role(&Role::new("a"))
            .unwrap()
            .collect();
        assert_eq!(roles.len(), 3);
        assert!(roles.contains(&"a"));
        assert!(roles.contains(&"b"));
        assert!(roles.contains(&"c"));
    }

    #[test]
    fn test_complex_hierarchy() {
        let mut structure = IxAccessStructureV1::new();

        // Create a complex role hierarchy
        let roles = vec!["superadmin", "admin", "moderator", "user", "guest"];
        for role in &roles {
            structure.add_role(&Role::new(role));
        }

        structure
            .assign_role(&Role::new("superadmin"), &Role::new("admin"))
            .unwrap();
        structure
            .assign_role(&Role::new("superadmin"), &Role::new("moderator"))
            .unwrap();
        structure
            .assign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();
        structure
            .assign_role(&Role::new("moderator"), &Role::new("user"))
            .unwrap();
        structure
            .assign_role(&Role::new("user"), &Role::new("guest"))
            .unwrap();

        let superadmin_roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("superadmin"))
            .unwrap()
            .collect();
        assert_eq!(superadmin_roles.len(), 5);

        let user_roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("user"))
            .unwrap()
            .collect();
        assert_eq!(user_roles.len(), 2);
        assert!(user_roles.contains(&"user"));
        assert!(user_roles.contains(&"guest"));
    }

    #[test]
    fn test_bfs_visits_each_node_once() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("a"));
        structure.add_role(&Role::new("b"));
        structure.add_role(&Role::new("c"));
        structure.add_role(&Role::new("d"));

        // Create a diamond: a -> b, a -> c, b -> d, c -> d
        structure
            .assign_role(&Role::new("a"), &Role::new("b"))
            .unwrap();
        structure
            .assign_role(&Role::new("a"), &Role::new("c"))
            .unwrap();
        structure
            .assign_role(&Role::new("b"), &Role::new("d"))
            .unwrap();
        structure
            .assign_role(&Role::new("c"), &Role::new("d"))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("a"))
            .unwrap()
            .collect();
        // Should visit d only once despite two paths
        assert_eq!(roles.len(), 4);
        let d_count = roles.iter().filter(|&&r| r == "d").count();
        assert_eq!(d_count, 1);
    }

    #[test]
    fn test_role_graph_consistency() {
        let mut structure = IxAccessStructureV1::new();

        for i in 0..100 {
            structure.add_role(&Role::new(format!("role{}", i)));
        }

        // role_graph should have same length as role_resolver
        assert_eq!(structure.role_resolver.len(), structure.role_graph.len());
    }

    #[test]
    fn test_large_role_count() {
        let mut structure = IxAccessStructureV1::new();

        for i in 0..1000 {
            structure.add_role(&Role::new(format!("role{}", i)));
        }

        assert_eq!(structure.list_roles().count(), 1000);
        let roles: Vec<_> = structure.list_roles().collect();
        assert_eq!(roles.len(), 1000);
    }

    #[test]
    fn test_deep_hierarchy() {
        let mut structure = IxAccessStructureV1::new();

        // Create a chain of 100 roles
        for i in 0..100 {
            structure.add_role(&Role::new(format!("role{}", i)));
        }

        for i in 0..99 {
            structure
                .assign_role(
                    &Role::new(&format!("role{}", i)),
                    &Role::new(&format!("role{}", i + 1)),
                )
                .unwrap();
        }

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("role0"))
            .unwrap()
            .collect();
        assert_eq!(roles.len(), 100);
    }

    #[test]
    fn test_cycle_detection() {
        let mut structure = IxAccessStructureV1::new();

        structure.add_role(&Role::new("a"));
        structure.add_role(&Role::new("b"));
        structure.add_role(&Role::new("c"));

        // Create a cycle: a -> b -> c -> a
        structure
            .assign_role(&Role::new("a"), &Role::new("b"))
            .unwrap();
        structure
            .assign_role(&Role::new("b"), &Role::new("c"))
            .unwrap();
        structure
            .assign_role(&Role::new("c"), &Role::new("a"))
            .unwrap();

        // BFS should handle cycles and not infinite loop
        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("a"))
            .unwrap()
            .collect();

        // Should visit each role exactly once
        assert_eq!(roles.len(), 3);
        assert!(roles.contains(&"a"));
        assert!(roles.contains(&"b"));
        assert!(roles.contains(&"c"));
    }

    #[test]
    fn test_wide_hierarchy() {
        let mut structure = IxAccessStructureV1::new();

        structure.add_role(&Role::new("root"));
        for i in 0..50 {
            let role_name = format!("child{}", i);
            structure.add_role(&Role::new(&role_name));
            structure
                .assign_role(&Role::new("root"), &Role::new(&role_name))
                .unwrap();
        }

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("root"))
            .unwrap()
            .collect();
        assert_eq!(roles.len(), 51); // root + 50 children
    }

    #[quickcheck_macros::quickcheck]
    fn prop_add_role_idempotent(role_str: AsciiRole) -> bool {
        let mut structure1 = IxAccessStructureV1::new();
        structure1.add_role(&Role::new(&role_str.0));

        let mut structure2 = IxAccessStructureV1::new();
        structure2.add_role(&Role::new(&role_str.0));
        structure2.add_role(&Role::new(&role_str.0));

        let roles1: Vec<_> = structure1.list_roles().collect();
        let roles2: Vec<_> = structure2.list_roles().collect();

        roles1 == roles2
    }

    #[quickcheck_macros::quickcheck]
    fn prop_list_roles_contains_added(role_str: AsciiRole) -> bool {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new(&role_str.0));

        let roles: Vec<_> = structure.list_roles().collect();
        roles.contains(&role_str.0.to_ascii_lowercase().as_str())
    }

    #[quickcheck_macros::quickcheck]
    fn prop_assign_role_includes_both(role1_str: AsciiRole, role2_str: AsciiRole) -> TestResult {
        // Avoid same role names
        if role1_str.0.to_ascii_lowercase() == role2_str.0.to_ascii_lowercase() {
            return TestResult::discard();
        }

        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new(&role1_str.0));
        structure.add_role(&Role::new(&role2_str.0));
        structure
            .assign_role(&Role::new(&role1_str.0), &Role::new(&role2_str.0))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new(&role1_str.0))
            .unwrap()
            .collect();

        let lower1 = role1_str.0.to_ascii_lowercase();
        let lower2 = role2_str.0.to_ascii_lowercase();

        TestResult::from_bool(roles.contains(&lower1.as_str()) && roles.contains(&lower2.as_str()))
    }

    #[quickcheck_macros::quickcheck]
    fn prop_serialization_preserves_roles(roles: Vec<AsciiRole>) -> bool {
        let roles: Vec<_> = roles.into_iter().take(10).collect(); // Limit size

        let mut structure = IxAccessStructureV1::new();
        for role in &roles {
            structure.add_role(&Role::new(&role.0));
        }

        let bytes = structure.to_bytes();
        let deserialized = IxAccessStructureV1::read_from_buffer(&bytes);

        let original_roles: Vec<_> = structure.list_roles().collect();
        let deserialized_roles: Vec<_> = deserialized.list_roles().collect();

        original_roles == deserialized_roles
    }

    #[quickcheck_macros::quickcheck]
    fn prop_list_all_roles_includes_self(role_str: AsciiRole) -> bool {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new(&role_str.0));

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new(&role_str.0))
            .unwrap()
            .collect();
        let lower = role_str.0.to_ascii_lowercase();

        roles.contains(&lower.as_str())
    }

    #[quickcheck_macros::quickcheck]
    fn prop_assign_role_transitive(
        role1: AsciiRole,
        role2: AsciiRole,
        role3: AsciiRole,
    ) -> TestResult {
        // Make sure all roles are different
        let r1 = role1.0.to_ascii_lowercase();
        let r2 = role2.0.to_ascii_lowercase();
        let r3 = role3.0.to_ascii_lowercase();

        if r1 == r2 || r2 == r3 || r1 == r3 {
            return TestResult::discard();
        }

        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new(&role1.0));
        structure.add_role(&Role::new(&role2.0));
        structure.add_role(&Role::new(&role3.0));

        // Create chain: role1 -> role2 -> role3
        structure
            .assign_role(&Role::new(&role1.0), &Role::new(&role2.0))
            .unwrap();
        structure
            .assign_role(&Role::new(&role2.0), &Role::new(&role3.0))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new(&role1.0))
            .unwrap()
            .collect();

        TestResult::from_bool(
            roles.contains(&r1.as_str())
                && roles.contains(&r2.as_str())
                && roles.contains(&r3.as_str()),
        )
    }

    // ============================================================
    // Tests for get_all_resources_for_role_by_tag
    // ============================================================

    #[test]
    fn test_get_resources_simple() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("user"));

        structure
            .assign_resource_to_role(&Role::new("user"), "database", "users_db")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("user"), "database", "logs_db")
            .unwrap();

        let resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "database")
            .unwrap()
            .collect();

        assert_eq!(resources.len(), 2);
        assert!(resources.contains(&"users_db"));
        assert!(resources.contains(&"logs_db"));
    }

    #[test]
    fn test_get_resources_inherited() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("admin"));
        structure.add_role(&Role::new("user"));

        structure
            .assign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();

        structure
            .assign_resource_to_role(&Role::new("user"), "database", "users_db")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("admin"), "database", "admin_db")
            .unwrap();

        let resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("admin"), "database")
            .unwrap()
            .collect();

        assert_eq!(resources.len(), 2);
        assert!(resources.contains(&"users_db"));
        assert!(resources.contains(&"admin_db"));
    }

    #[test]
    fn test_get_resources_transitive_inheritance() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("superadmin"));
        structure.add_role(&Role::new("admin"));
        structure.add_role(&Role::new("user"));

        structure
            .assign_role(&Role::new("superadmin"), &Role::new("admin"))
            .unwrap();
        structure
            .assign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();

        structure
            .assign_resource_to_role(&Role::new("user"), "api", "/api/users")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("admin"), "api", "/api/admin")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("superadmin"), "api", "/api/super")
            .unwrap();

        let resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("superadmin"), "api")
            .unwrap()
            .collect();

        assert_eq!(resources.len(), 3);
        assert!(resources.contains(&"/api/users"));
        assert!(resources.contains(&"/api/admin"));
        assert!(resources.contains(&"/api/super"));
    }

    #[test]
    fn test_get_resources_multiple_tags() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("user"));

        structure
            .assign_resource_to_role(&Role::new("user"), "database", "users_db")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("user"), "api", "/api/users")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("user"), "file", "/data/users")
            .unwrap();

        let db_resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "database")
            .unwrap()
            .collect();
        assert_eq!(db_resources.len(), 1);
        assert_eq!(db_resources[0], "users_db");

        let api_resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "api")
            .unwrap()
            .collect();
        assert_eq!(api_resources.len(), 1);
        assert_eq!(api_resources[0], "/api/users");

        let file_resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "file")
            .unwrap()
            .collect();
        assert_eq!(file_resources.len(), 1);
        assert_eq!(file_resources[0], "/data/users");
    }

    #[test]
    fn test_get_resources_empty() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("user"));
        structure.add_role(&Role::new("admin"));

        // Assign a database resource to admin so the tag exists
        structure
            .assign_resource_to_role(&Role::new("admin"), "database", "admin_db")
            .unwrap();

        // Query for database resources on user role (which has none)
        let resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "database")
            .unwrap()
            .collect();

        assert_eq!(resources.len(), 0);
    }

    #[test]
    fn test_get_resources_no_duplicates_in_diamond() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("top"));
        structure.add_role(&Role::new("left"));
        structure.add_role(&Role::new("right"));
        structure.add_role(&Role::new("bottom"));

        // Diamond: top -> left, top -> right, left -> bottom, right -> bottom
        structure
            .assign_role(&Role::new("top"), &Role::new("left"))
            .unwrap();
        structure
            .assign_role(&Role::new("top"), &Role::new("right"))
            .unwrap();
        structure
            .assign_role(&Role::new("left"), &Role::new("bottom"))
            .unwrap();
        structure
            .assign_role(&Role::new("right"), &Role::new("bottom"))
            .unwrap();

        structure
            .assign_resource_to_role(&Role::new("bottom"), "db", "shared_db")
            .unwrap();

        let resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("top"), "db")
            .unwrap()
            .collect();

        // Should not have duplicates even though there are two paths to bottom
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0], "shared_db");
    }

    #[test]
    fn test_get_resources_role_not_found() {
        let structure = IxAccessStructureV1::new();
        let role = Role::new("nonexistent");

        let result = structure.get_all_resources_for_role_by_tag(&role, "database");
        assert!(result.is_err());
        match result {
            Err(StructureError::RoleNotFound(s)) => assert_eq!(s, "nonexistent"),
            _ => panic!("Expected RoleNotFound error"),
        }
    }

    #[test]
    fn test_get_resources_tag_not_found() {
        let mut structure = IxAccessStructureV1::new();
        let role = Role::new("user");
        structure.add_role(&role);

        let result = structure.get_all_resources_for_role_by_tag(&role, "nonexistent_tag");
        assert!(result.is_err());
        match result {
            Err(StructureError::ResourceTagNotFound(s)) => assert_eq!(s, "nonexistent_tag"),
            _ => panic!("Expected ResourceTagNotFound error"),
        }
    }

    #[test]
    fn test_get_resources_unicode_values() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("user"));

        structure
            .assign_resource_to_role(&Role::new("user"), "file", "/path/to/café.txt")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("user"), "url", "https://example.com/文档")
            .unwrap();

        let files: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "file")
            .unwrap()
            .collect();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], "/path/to/café.txt");

        let urls: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "url")
            .unwrap()
            .collect();
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], "https://example.com/文档");
    }

    #[test]
    fn test_get_resources_case_sensitive_values() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("user"));

        structure
            .assign_resource_to_role(&Role::new("user"), "api", "/API/users")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("user"), "api", "/api/USERS")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("user"), "api", "/api/users")
            .unwrap();

        let resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "api")
            .unwrap()
            .collect();

        // All three should be present as resources are case-sensitive
        assert_eq!(resources.len(), 3);
        assert!(resources.contains(&"/API/users"));
        assert!(resources.contains(&"/api/USERS"));
        assert!(resources.contains(&"/api/users"));
    }

    #[test]
    fn test_get_resources_deduplication() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("user"));

        // Assign the same resource multiple times
        structure
            .assign_resource_to_role(&Role::new("user"), "db", "main_db")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("user"), "db", "main_db")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("user"), "db", "main_db")
            .unwrap();

        let resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "db")
            .unwrap()
            .collect();

        // Should be deduplicated
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0], "main_db");
    }

    #[test]
    fn test_assign_resource_to_nonexistent_role() {
        let mut structure = IxAccessStructureV1::new();

        let result = structure.assign_resource_to_role(&Role::new("nonexistent"), "db", "test_db");
        assert!(result.is_err());
        match result {
            Err(StructureError::RoleNotFound(s)) => assert_eq!(s, "nonexistent"),
            _ => panic!("Expected RoleNotFound error"),
        }
    }

    #[test]
    fn test_get_resources_special_characters() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("service"));

        structure
            .assign_resource_to_role(
                &Role::new("service"),
                "conn",
                "postgresql://user:pass@localhost:5432/db?sslmode=require",
            )
            .unwrap();
        structure
            .assign_resource_to_role(
                &Role::new("service"),
                "path",
                "C:\\Program Files\\App\\config.json",
            )
            .unwrap();
        structure
            .assign_resource_to_role(
                &Role::new("service"),
                "regex",
                "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$",
            )
            .unwrap();

        let conn: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("service"), "conn")
            .unwrap()
            .collect();
        assert_eq!(conn.len(), 1);
        assert_eq!(
            conn[0],
            "postgresql://user:pass@localhost:5432/db?sslmode=require"
        );

        let path: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("service"), "path")
            .unwrap()
            .collect();
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], "C:\\Program Files\\App\\config.json");

        let regex: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("service"), "regex")
            .unwrap()
            .collect();
        assert_eq!(regex.len(), 1);
        assert_eq!(
            regex[0],
            "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
        );
    }

    #[quickcheck_macros::quickcheck]
    fn prop_bfs_no_duplicates(roles: Vec<AsciiRole>) -> TestResult {
        let roles: Vec<_> = roles.into_iter().take(5).collect();
        if roles.is_empty() {
            return TestResult::passed();
        }

        let mut structure = IxAccessStructureV1::new();
        for role in &roles {
            structure.add_role(&Role::new(&role.0));
        }

        // Assign all roles to first role
        if roles.len() > 1 {
            for i in 1..roles.len() {
                structure
                    .assign_role(&Role::new(&roles[0].0), &Role::new(&roles[i].0))
                    .unwrap();
            }
        }

        let result: Vec<_> = structure
            .list_all_roles_for_role(&Role::new(&roles[0].0))
            .unwrap()
            .collect();
        let unique_count = result.iter().collect::<HashSet<_>>().len();

        TestResult::from_bool(result.len() == unique_count)
    }

    #[test]
    fn test_unassign_role_simple() {
        let mut structure = create_test_structure();
        structure
            .assign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();

        let roles_before: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();
        assert_eq!(roles_before.len(), 2);

        structure
            .unassign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();

        let roles_after: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();
        assert_eq!(roles_after.len(), 1);
        assert_eq!(roles_after[0], "admin");
    }

    #[test]
    fn test_unassign_role_chain() {
        let mut structure = create_test_structure();
        structure
            .assign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();
        structure
            .assign_role(&Role::new("user"), &Role::new("guest"))
            .unwrap();

        structure
            .unassign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], "admin");
        assert!(!roles.contains(&"user"));
        assert!(!roles.contains(&"guest"));
    }

    #[test]
    fn test_unassign_role_nonexistent_assignee() {
        let mut structure = create_test_structure();
        let result = structure.unassign_role(&Role::new("nonexistent"), &Role::new("user"));
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), StructureError::RoleNotFound(ref s) if s == "nonexistent")
        );
    }

    #[test]
    fn test_unassign_role_nonexistent_role() {
        let mut structure = create_test_structure();
        let result = structure.unassign_role(&Role::new("admin"), &Role::new("nonexistent"));
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), StructureError::RoleNotFound(ref s) if s == "nonexistent")
        );
    }

    #[test]
    fn test_unassign_role_not_assigned() {
        let mut structure = create_test_structure();

        // No assignment between admin and guest, but both roles exist
        let result = structure.unassign_role(&Role::new("admin"), &Role::new("guest"));
        assert!(result.is_ok());

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], "admin");
    }

    #[test]
    fn test_unassign_role_multiple_assignments() {
        let mut structure = create_test_structure();
        structure
            .assign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();
        structure
            .assign_role(&Role::new("admin"), &Role::new("guest"))
            .unwrap();

        structure
            .unassign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();
        assert_eq!(roles.len(), 2);
        assert!(roles.contains(&"admin"));
        assert!(roles.contains(&"guest"));
        assert!(!roles.contains(&"user"));
    }

    #[test]
    fn test_unassign_role_case_insensitive() {
        let mut structure = create_test_structure();
        structure
            .assign_role(&Role::new("admin"), &Role::new("user"))
            .unwrap();

        structure
            .unassign_role(&Role::new("ADMIN"), &Role::new("USER"))
            .unwrap();

        let roles: Vec<_> = structure
            .list_all_roles_for_role(&Role::new("admin"))
            .unwrap()
            .collect();
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0], "admin");
    }

    #[test]
    fn test_unassign_resource_simple() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("user"));

        structure
            .assign_resource_to_role(&Role::new("user"), "database", "users_db")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("user"), "database", "logs_db")
            .unwrap();

        let resources_before: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "database")
            .unwrap()
            .collect();
        assert_eq!(resources_before.len(), 2);

        structure
            .unassign_resource_from_role(&Role::new("user"), "database", "users_db")
            .unwrap();

        let resources_after: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "database")
            .unwrap()
            .collect();
        assert_eq!(resources_after.len(), 1);
        assert_eq!(resources_after[0], "logs_db");
    }

    #[test]
    fn test_unassign_resource_all() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("user"));

        structure
            .assign_resource_to_role(&Role::new("user"), "database", "users_db")
            .unwrap();

        structure
            .unassign_resource_from_role(&Role::new("user"), "database", "users_db")
            .unwrap();

        let resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "database")
            .unwrap()
            .collect();
        assert_eq!(resources.len(), 0);
    }

    #[test]
    fn test_unassign_resource_nonexistent_role() {
        let mut structure = IxAccessStructureV1::new();

        let result =
            structure.unassign_resource_from_role(&Role::new("nonexistent"), "db", "test_db");
        assert!(result.is_err());
        match result {
            Err(StructureError::RoleNotFound(s)) => assert_eq!(s, "nonexistent"),
            _ => panic!("Expected RoleNotFound error"),
        }
    }

    #[test]
    fn test_unassign_resource_nonexistent_tag() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("user"));

        let result =
            structure.unassign_resource_from_role(&Role::new("user"), "nonexistent_tag", "value");
        assert!(result.is_err());
        match result {
            Err(StructureError::ResourceTagNotFound(s)) => assert_eq!(s, "nonexistent_tag"),
            _ => panic!("Expected ResourceTagNotFound error"),
        }
    }

    #[test]
    fn test_unassign_resource_nonexistent_value() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("user"));

        structure
            .assign_resource_to_role(&Role::new("user"), "database", "users_db")
            .unwrap();

        let result =
            structure.unassign_resource_from_role(&Role::new("user"), "database", "nonexistent");
        assert!(result.is_err());
        match result {
            Err(StructureError::ResourceTagNotFound(s)) => assert_eq!(s, "nonexistent"),
            _ => panic!("Expected ResourceTagNotFound error"),
        }
    }

    #[test]
    fn test_unassign_resource_multiple_tags() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("user"));

        structure
            .assign_resource_to_role(&Role::new("user"), "database", "users_db")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("user"), "api", "/api/users")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("user"), "file", "/data/users")
            .unwrap();

        structure
            .unassign_resource_from_role(&Role::new("user"), "database", "users_db")
            .unwrap();

        // Database resource should be gone
        let db_resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "database")
            .unwrap()
            .collect();
        assert_eq!(db_resources.len(), 0);

        // Other resources should remain
        let api_resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "api")
            .unwrap()
            .collect();
        assert_eq!(api_resources.len(), 1);

        let file_resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "file")
            .unwrap()
            .collect();
        assert_eq!(file_resources.len(), 1);
    }

    #[test]
    fn test_unassign_resource_case_sensitive() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("user"));

        structure
            .assign_resource_to_role(&Role::new("user"), "api", "/API/users")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("user"), "api", "/api/users")
            .unwrap();

        structure
            .unassign_resource_from_role(&Role::new("user"), "api", "/API/users")
            .unwrap();

        let resources: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "api")
            .unwrap()
            .collect();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0], "/api/users");
    }

    #[test]
    fn test_unassign_resource_unicode() {
        let mut structure = IxAccessStructureV1::new();
        structure.add_role(&Role::new("user"));

        structure
            .assign_resource_to_role(&Role::new("user"), "file", "/path/to/café.txt")
            .unwrap();
        structure
            .assign_resource_to_role(&Role::new("user"), "url", "https://example.com/文档")
            .unwrap();

        structure
            .unassign_resource_from_role(&Role::new("user"), "file", "/path/to/café.txt")
            .unwrap();

        let files: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "file")
            .unwrap()
            .collect();
        assert_eq!(files.len(), 0);

        let urls: Vec<_> = structure
            .get_all_resources_for_role_by_tag(&Role::new("user"), "url")
            .unwrap()
            .collect();
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], "https://example.com/文档");
    }
}
