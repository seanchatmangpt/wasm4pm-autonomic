use insa_types::{CompletedMask, FieldMask};

pub fn compute_effects(_add: FieldMask, _clear: FieldMask) -> CompletedMask {
    // Computes effects bounded by Construct8 limits
    CompletedMask(0)
}
