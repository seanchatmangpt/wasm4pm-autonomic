import os
import shutil

def write_file(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(content)

# Remove old single files if they exist
old_files = [
    '../insa/insa-kappa8/src/reflect_eliza.rs',
    '../insa/insa-truthforge/tests/kappa8_engines.rs'
]
for old_file in old_files:
    if os.path.exists(old_file):
        os.remove(old_file)

mod_rs = """pub mod pattern;
pub mod engine;
pub mod result;
pub mod fixtures;

pub use pattern::*;
pub use engine::*;
pub use result::*;
pub use fixtures::*;
"""

pattern_rs = """use insa_types::{FieldMask, CompletedMask};
use insa_instinct::{InstinctByte, ElizaByte};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PatternId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TemplateId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum AskKind {
    #[default]
    None = 0,
    Clarify = 1,
    MissingSlot = 2,
    ConfirmIntent = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReflectPattern {
    pub id: PatternId,
    pub required_context: FieldMask,
    pub template_id: TemplateId,
    pub emits: InstinctByte,
    pub eliza_detail: ElizaByte,
    pub ask_kind: AskKind,
    pub _padding: [u8; 3],
}
"""

engine_rs = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, KappaByte, ElizaByte};
use crate::reflect_eliza::pattern::{ReflectPattern, AskKind};
use crate::reflect_eliza::result::{ReflectResult, ReflectStatus};

pub struct ReflectEliza;

impl ReflectEliza {
    pub fn evaluate(patterns: &[ReflectPattern], present: FieldMask, expected_slots: FieldMask) -> ReflectResult {
        let mut detail = ElizaByte::empty();
        let mut emits = InstinctByte::empty();
        
        let missing_slots = (present.0 & expected_slots.0) ^ expected_slots.0;
        
        if missing_slots != 0 {
            detail = detail.union(ElizaByte::DETECT_MISSING_SLOT).union(ElizaByte::ASK_CLARIFYING);
            emits = emits.union(InstinctByte::ASK).union(InstinctByte::INSPECT);
            return ReflectResult {
                status: ReflectStatus::Incomplete,
                detail,
                kappa: KappaByte::REFLECT,
                emits,
                missing_slots: FieldMask(missing_slots),
                selected_pattern: None,
            };
        }

        let mut best_pattern = None;
        
        for pat in patterns {
            if (present.0 & pat.required_context.0) == pat.required_context.0 {
                best_pattern = Some(pat);
                break;
            }
        }
        
        if let Some(pat) = best_pattern {
            detail = detail.union(pat.eliza_detail);
            emits = emits.union(pat.emits);
            
            if pat.ask_kind != AskKind::None {
                emits = emits.union(InstinctByte::ASK);
                detail = detail.union(ElizaByte::ASK_CLARIFYING);
            }
            
            ReflectResult {
                status: ReflectStatus::Matched,
                detail,
                kappa: KappaByte::REFLECT,
                emits,
                missing_slots: FieldMask(0),
                selected_pattern: Some(pat.id),
            }
        } else {
            detail = detail.union(ElizaByte::DEFER_TO_CLOSURE);
            emits = emits.union(InstinctByte::SETTLE);
            ReflectResult {
                status: ReflectStatus::NoMatch,
                detail,
                kappa: KappaByte::REFLECT,
                emits,
                missing_slots: FieldMask(0),
                selected_pattern: None,
            }
        }
    }
}
"""

result_rs = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, KappaByte, ElizaByte};
use crate::reflect_eliza::pattern::PatternId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReflectStatus {
    Matched = 0,
    Incomplete = 1,
    NoMatch = 2,
}

impl Default for ReflectStatus {
    fn default() -> Self {
        Self::NoMatch
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReflectResult {
    pub status: ReflectStatus,
    pub detail: ElizaByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
    pub missing_slots: FieldMask,
    pub selected_pattern: Option<PatternId>,
}
"""

fixtures_rs = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, ElizaByte};
use crate::reflect_eliza::pattern::{ReflectPattern, PatternId, TemplateId, AskKind};

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
"""

test_content = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, ElizaByte};
use insa_kappa8::reflect_eliza::*;

#[test]
fn test_reflect_eliza_missing_slots() {
    let patterns = [frustrate_loan_pattern()];
    let present = FieldMask(CONTEXT_LOAN_DENIED);
    let expected = FieldMask(CONTEXT_LOAN_DENIED | CONTEXT_USER_FRUSTRATED);
    
    let res = ReflectEliza::evaluate(&patterns, present, expected);
    assert_eq!(res.status, ReflectStatus::Incomplete);
    assert!(res.detail.contains(ElizaByte::DETECT_MISSING_SLOT));
    assert!(res.detail.contains(ElizaByte::ASK_CLARIFYING));
    assert!(res.emits.contains(InstinctByte::ASK));
    assert!(res.emits.contains(InstinctByte::INSPECT));
}

#[test]
fn test_reflect_eliza_match() {
    let patterns = [frustrate_loan_pattern()];
    let present = FieldMask(CONTEXT_LOAN_DENIED | CONTEXT_USER_FRUSTRATED);
    let expected = FieldMask(CONTEXT_LOAN_DENIED | CONTEXT_USER_FRUSTRATED);
    
    let res = ReflectEliza::evaluate(&patterns, present, expected);
    assert_eq!(res.status, ReflectStatus::Matched);
    assert!(res.detail.contains(ElizaByte::DETECT_AFFECT));
    assert!(res.detail.contains(ElizaByte::SLOW_PREMATURE_ACTION));
    assert!(res.detail.contains(ElizaByte::ASK_CLARIFYING));
    assert!(res.emits.contains(InstinctByte::INSPECT));
    assert!(res.emits.contains(InstinctByte::ASK));
    assert!(res.emits.contains(InstinctByte::AWAIT));
    assert_eq!(res.selected_pattern.unwrap().0, 1);
}
"""

write_file('../insa/insa-kappa8/src/reflect_eliza/mod.rs', mod_rs)
write_file('../insa/insa-kappa8/src/reflect_eliza/pattern.rs', pattern_rs)
write_file('../insa/insa-kappa8/src/reflect_eliza/engine.rs', engine_rs)
write_file('../insa/insa-kappa8/src/reflect_eliza/result.rs', result_rs)
write_file('../insa/insa-kappa8/src/reflect_eliza/fixtures.rs', fixtures_rs)

write_file('../insa/insa-truthforge/tests/kappa_eliza.rs', test_content)

print("Reflect / ELIZA pack generated successfully.")
