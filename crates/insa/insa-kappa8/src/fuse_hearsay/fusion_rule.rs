use insa_instinct::InstinctByte;
use insa_types::FieldMask;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RuleId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RequiredMask(pub FieldMask);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConflictMask(pub FieldMask);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AuthorityMask(pub FieldMask);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FusionRule {
    pub id: RuleId,
    pub required_sources: RequiredMask,
    pub conflict_mask: ConflictMask,
    pub authority_required: AuthorityMask,
    pub emits_on_fail: InstinctByte,
}
