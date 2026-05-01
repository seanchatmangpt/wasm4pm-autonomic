use crate::precondition_strips::preconditions::{ForbiddenMask, RequiredMask};
use insa_types::{CompletedMask, FieldMask};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ActionId(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PolicyEpoch(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EffectMask {
    pub add: FieldMask,
    pub clear: FieldMask,
    pub complete: CompletedMask,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActionSchema {
    pub id: ActionId,
    pub required: RequiredMask,
    pub forbidden: ForbiddenMask,
    pub add_effects: FieldMask,
    pub clear_effects: FieldMask,
    pub completes: CompletedMask,
    pub policy_epoch: PolicyEpoch,
}
