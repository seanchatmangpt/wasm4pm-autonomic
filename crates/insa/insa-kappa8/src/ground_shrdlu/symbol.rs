use insa_types::{FieldMask, ObjectRef};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SymbolId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AliasId(pub u16);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GroundingRule {
    pub symbol: SymbolId,
    pub required_context: FieldMask,
    pub expected_object: ObjectRef,
}
