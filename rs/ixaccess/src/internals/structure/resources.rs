use crate::internals::error::{Result, StructureError};
use crate::internals::role::Role;

use super::{IxAccessStructureV1, Resource};

impl IxAccessStructureV1 {
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
