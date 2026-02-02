use bincode::config::{Fixint, LittleEndian, NoLimit};
use bytes::Bytes;

use crate::interner::{Index, Interner};

use super::header::{IxAccessFileHeader, Version, HEADER_SIZE, HEADER_V1};
use super::role::Role;

pub(super) mod bfs;
pub(crate) mod resources;
pub(crate) mod roles;
pub(crate) mod search;

#[cfg(test)]
mod tests;

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode, PartialOrd, Ord, Hash,
)]
pub(super) struct Resource {
    pub tag: Index,
    pub value: Index,
}

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
    pub(super) role_resolver: Interner,
    pub(super) resource_resolver: Interner,
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

pub(crate) const BINCODE_CONFIG: bincode::config::Configuration<LittleEndian, Fixint, NoLimit> = {
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

    #[inline]
    pub(super) fn get(&self, role: &Role) -> Option<RoleId> {
        self.role_resolver.get(role.as_str()).map(RoleId)
    }

    #[inline]
    pub(super) fn resolve(&self, role_id: RoleId) -> Option<&str> {
        self.role_resolver.resolve(role_id.0)
    }
}
