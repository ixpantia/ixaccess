use std::collections::{HashSet, VecDeque};

use super::{IxAccessStructureV1, RoleId};

pub(super) struct IxAccessStructureV1BFS<'s> {
    pub structure: &'s IxAccessStructureV1,
    pub queue: VecDeque<RoleId>,
    pub visited: HashSet<RoleId>,
}

impl<'s> Iterator for IxAccessStructureV1BFS<'s> {
    type Item = RoleId;
    fn next(&mut self) -> Option<Self::Item> {
        let current_role_id = self.queue.pop_front()?;
        let current_index = current_role_id.into_index();
        for &next_role_index in &self.structure.role_graph[current_index] {
            let next_role_id = RoleId(next_role_index);
            if self.visited.insert(next_role_id) {
                self.queue.push_back(next_role_id);
            }
        }
        Some(current_role_id)
    }
}
