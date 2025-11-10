#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) struct Signature(u64);

pub(super) const SIGNATURE: Signature = Signature(u64::from_le_bytes(*b"IXACCESS"));
pub(super) const SIGNATURE_SIZE: usize = size_of::<Signature>();

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) struct Version(pub u64);

pub(super) const VERSION_SIZE: usize = size_of::<Version>();

#[derive(Debug, Copy, Clone)]
pub(super) struct IxAccessFileHeader {
    pub signature: Signature,
    pub version: Version,
}

pub(super) const HEADER_V1: IxAccessFileHeader = IxAccessFileHeader {
    signature: SIGNATURE,
    version: Version(1),
};

pub(super) const HEADER_SIZE: usize = size_of::<IxAccessFileHeader>();

impl IxAccessFileHeader {
    pub fn from_bytes(bytes: &[u8]) -> IxAccessFileHeader {
        let signature = Signature(u64::from_le_bytes(
            bytes[0..SIGNATURE_SIZE].try_into().unwrap(),
        ));
        let version = Version(u64::from_le_bytes(
            bytes[SIGNATURE_SIZE..SIGNATURE_SIZE + VERSION_SIZE]
                .try_into()
                .unwrap(),
        ));
        assert_eq!(signature, SIGNATURE);
        IxAccessFileHeader { signature, version }
    }

    pub fn to_bytes(self) -> [u8; HEADER_SIZE] {
        let mut header_bytes = [0u8; HEADER_SIZE];
        header_bytes[0..SIGNATURE_SIZE].copy_from_slice(&self.signature.0.to_le_bytes());
        header_bytes[SIGNATURE_SIZE..SIGNATURE_SIZE + VERSION_SIZE]
            .copy_from_slice(&self.version.0.to_le_bytes());
        header_bytes
    }
}
