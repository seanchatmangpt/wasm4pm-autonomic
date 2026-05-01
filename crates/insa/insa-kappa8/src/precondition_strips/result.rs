use insa_instinct::{InstinctByte, KappaByte, StripsByte};
use insa_types::FieldMask;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreconditionResult {
    pub detail: StripsByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
    pub missing_required: FieldMask,
    pub present_forbidden: FieldMask,
    pub add_effects: FieldMask,
    pub clear_effects: FieldMask,
}

impl Default for PreconditionResult {
    fn default() -> Self {
        Self {
            detail: StripsByte::empty(),
            kappa: KappaByte::empty(),
            emits: InstinctByte::empty(),
            missing_required: FieldMask(0),
            present_forbidden: FieldMask(0),
            add_effects: FieldMask(0),
            clear_effects: FieldMask(0),
        }
    }
}
