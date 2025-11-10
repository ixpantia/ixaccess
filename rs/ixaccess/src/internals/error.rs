use thiserror::Error;

use crate::storage::StorageError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum StructureError {
    #[error("Role not found: {0}")]
    RoleNotFound(String),
    #[error("Resource tag not found: {0}")]
    ResourceTagNotFound(String),
}

pub type Result<T> = std::result::Result<T, StructureError>;
