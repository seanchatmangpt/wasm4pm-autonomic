use crate::ground_shrdlu::symbol::{GroundingRule, SymbolId};
use insa_types::{FieldMask, ObjectRef};

pub const CONTEXT_VENDOR: u64 = 1 << 0;
pub const CONTEXT_EMPLOYEE: u64 = 1 << 1;

pub fn vendor_grounding_rule() -> GroundingRule {
    GroundingRule {
        symbol: SymbolId(1),
        required_context: FieldMask(CONTEXT_VENDOR),
        expected_object: ObjectRef(100),
    }
}

pub fn employee_grounding_rule() -> GroundingRule {
    GroundingRule {
        symbol: SymbolId(1),
        required_context: FieldMask(CONTEXT_EMPLOYEE),
        expected_object: ObjectRef(200),
    }
}
