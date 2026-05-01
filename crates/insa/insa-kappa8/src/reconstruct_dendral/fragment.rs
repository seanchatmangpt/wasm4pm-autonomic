use insa_instinct::InstinctByte;
use insa_types::FieldMask;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FragmentId(pub u16);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReconstructionRule {
    pub id: FragmentId,
    pub fragments_required: FieldMask,
    pub constraints_forbidden: FieldMask,
    pub generates_hypothesis: FieldMask,
    pub emits: InstinctByte,
}
