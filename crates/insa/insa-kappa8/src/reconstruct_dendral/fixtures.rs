use crate::reconstruct_dendral::fragment::{FragmentId, ReconstructionRule};
use insa_instinct::InstinctByte;
use insa_types::FieldMask;

pub const FRAG_IP: u64 = 1 << 0;
pub const FRAG_USER_AGENT: u64 = 1 << 1;
pub const FRAG_LOCATION: u64 = 1 << 2;

pub fn identity_reconstruction_rule() -> ReconstructionRule {
    ReconstructionRule {
        id: FragmentId(1),
        fragments_required: FieldMask(FRAG_IP | FRAG_USER_AGENT),
        constraints_forbidden: FieldMask(0),
        generates_hypothesis: FieldMask(1 << 10),
        emits: InstinctByte::INSPECT,
    }
}
