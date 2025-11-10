use crate::interner::Index;

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode, PartialOrd, Ord, Hash,
)]
pub(super) struct Resource {
    pub tag: Index,
    pub value: Index,
}
