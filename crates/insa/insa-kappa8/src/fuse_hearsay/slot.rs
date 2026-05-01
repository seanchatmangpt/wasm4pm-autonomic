use crate::fuse_hearsay::freshness::FreshnessByte;
use crate::fuse_hearsay::source::{AuthorityByte, SourceId};
use insa_types::{FieldMask, ObjectRef};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SlotId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EvidenceKind(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DigestRef(pub u64);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EvidenceSlot {
    pub id: SlotId,
    pub source: SourceId,
    pub object: ObjectRef,
    pub kind: EvidenceKind,
    pub asserts: FieldMask,
    pub freshness: FreshnessByte,
    pub authority: AuthorityByte,
    pub digest: DigestRef,
}
