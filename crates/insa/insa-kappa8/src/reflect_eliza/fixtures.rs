use crate::reflect_eliza::pattern::{AskKind, PatternId, ReflectPattern, TemplateId};
use insa_instinct::{ElizaByte, InstinctByte};
use insa_types::FieldMask;

pub const CONTEXT_LOAN_DENIED: u64 = 1 << 0;
pub const CONTEXT_USER_FRUSTRATED: u64 = 1 << 1;

pub fn frustrate_loan_pattern() -> ReflectPattern {
    ReflectPattern {
        id: PatternId(1),
        required_context: FieldMask(CONTEXT_LOAN_DENIED | CONTEXT_USER_FRUSTRATED),
        template_id: TemplateId(1),
        emits: InstinctByte::INSPECT.union(InstinctByte::AWAIT),
        eliza_detail: ElizaByte::DETECT_AFFECT.union(ElizaByte::SLOW_PREMATURE_ACTION),
        ask_kind: AskKind::ConfirmIntent,
        _padding: [0; 3],
    }
}
