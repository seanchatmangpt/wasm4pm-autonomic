use insa_instinct::InstinctByte;
use insa_types::FieldMask;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ExpertRuleId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Confidence(pub u8); // 0-100

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PolicyEpoch(pub u64);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExpertRule {
    pub id: ExpertRuleId,
    pub required: FieldMask,
    pub forbidden: FieldMask,
    pub emits: InstinctByte,
    pub confidence: Confidence,
    pub epoch: PolicyEpoch,
}
