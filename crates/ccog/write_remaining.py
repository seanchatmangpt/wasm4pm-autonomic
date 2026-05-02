import os

def write_file(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(content)

old_files = [
    '../insa/insa-kappa8/src/ground_shrdlu.rs',
    '../insa/insa-kappa8/src/prove_prolog.rs',
    '../insa/insa-kappa8/src/rule_mycin.rs',
    '../insa/insa-kappa8/src/reconstruct_dendral.rs',
]

for old_file in old_files:
    if os.path.exists(old_file):
        os.remove(old_file)

# --- SHRDLU ---
shrdlu_mod = """pub mod symbol;
pub mod engine;
pub mod result;
pub mod fixtures;

pub use symbol::*;
pub use engine::*;
pub use result::*;
pub use fixtures::*;
"""

shrdlu_symbol = """use insa_types::{FieldMask, ObjectRef};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SymbolId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AliasId(pub u16);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GroundingRule {
    pub symbol: SymbolId,
    pub required_context: FieldMask,
    pub expected_object: ObjectRef,
}
"""

shrdlu_engine = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, KappaByte, ShrdluByte};
use crate::ground_shrdlu::symbol::GroundingRule;
use crate::ground_shrdlu::result::{GroundingResult, GroundingStatus};

pub struct GroundShrdlu;

impl GroundShrdlu {
    pub fn evaluate(rules: &[GroundingRule], symbol_detected: bool, present: FieldMask) -> GroundingResult {
        let mut detail = ShrdluByte::empty();
        let mut emits = InstinctByte::empty();
        
        if !symbol_detected {
            detail = detail.union(ShrdluByte::MISSING_OBJECT);
            emits = emits.union(InstinctByte::ASK).union(InstinctByte::RETRIEVE);
            return GroundingResult {
                status: GroundingStatus::Missing,
                detail,
                kappa: KappaByte::GROUND,
                emits,
                resolved_object: None,
            };
        }

        let mut matched_rules = 0;
        let mut last_object = None;

        for rule in rules {
            if (present.0 & rule.required_context.0) == rule.required_context.0 {
                matched_rules += 1;
                last_object = Some(rule.expected_object);
            }
        }

        if matched_rules == 1 {
            detail = detail.union(ShrdluByte::SYMBOL_RESOLVED).union(ShrdluByte::OBJECT_UNIQUE);
            emits = emits.union(InstinctByte::SETTLE);
            GroundingResult {
                status: GroundingStatus::Resolved,
                detail,
                kappa: KappaByte::GROUND,
                emits,
                resolved_object: last_object,
            }
        } else if matched_rules > 1 {
            detail = detail.union(ShrdluByte::AMBIGUOUS_REFERENCE).union(ShrdluByte::GROUNDING_FAILED);
            emits = emits.union(InstinctByte::INSPECT).union(InstinctByte::ASK);
            GroundingResult {
                status: GroundingStatus::Ambiguous,
                detail,
                kappa: KappaByte::GROUND,
                emits,
                resolved_object: None,
            }
        } else {
            detail = detail.union(ShrdluByte::GROUNDING_FAILED);
            emits = emits.union(InstinctByte::ASK).union(InstinctByte::RETRIEVE);
            GroundingResult {
                status: GroundingStatus::Failed,
                detail,
                kappa: KappaByte::GROUND,
                emits,
                resolved_object: None,
            }
        }
    }
}
"""

shrdlu_result = """use insa_types::ObjectRef;
use insa_instinct::{InstinctByte, KappaByte, ShrdluByte};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GroundingStatus {
    Resolved = 0,
    Ambiguous = 1,
    Missing = 2,
    Failed = 3,
}

impl Default for GroundingStatus {
    fn default() -> Self {
        Self::Missing
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GroundingResult {
    pub status: GroundingStatus,
    pub detail: ShrdluByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
    pub resolved_object: Option<ObjectRef>,
}
"""

shrdlu_fixtures = """use insa_types::{FieldMask, ObjectRef};
use crate::ground_shrdlu::symbol::{GroundingRule, SymbolId};

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
"""

shrdlu_test = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, ShrdluByte};
use insa_kappa8::ground_shrdlu::*;

#[test]
fn test_shrdlu_ambiguous() {
    let rules = [vendor_grounding_rule(), employee_grounding_rule()];
    // Both contexts present -> Ambiguous
    let present = FieldMask(CONTEXT_VENDOR | CONTEXT_EMPLOYEE);
    
    let res = GroundShrdlu::evaluate(&rules, true, present);
    assert_eq!(res.status, GroundingStatus::Ambiguous);
    assert!(res.detail.contains(ShrdluByte::AMBIGUOUS_REFERENCE));
    assert!(res.detail.contains(ShrdluByte::GROUNDING_FAILED));
    assert!(res.emits.contains(InstinctByte::INSPECT));
    assert!(res.emits.contains(InstinctByte::ASK));
}

#[test]
fn test_shrdlu_resolved() {
    let rules = [vendor_grounding_rule(), employee_grounding_rule()];
    let present = FieldMask(CONTEXT_VENDOR);
    
    let res = GroundShrdlu::evaluate(&rules, true, present);
    assert_eq!(res.status, GroundingStatus::Resolved);
    assert!(res.detail.contains(ShrdluByte::SYMBOL_RESOLVED));
    assert!(res.detail.contains(ShrdluByte::OBJECT_UNIQUE));
    assert!(res.emits.contains(InstinctByte::SETTLE));
    assert_eq!(res.resolved_object.unwrap().0, 100);
}
"""

write_file('../insa/insa-kappa8/src/ground_shrdlu/mod.rs', shrdlu_mod)
write_file('../insa/insa-kappa8/src/ground_shrdlu/symbol.rs', shrdlu_symbol)
write_file('../insa/insa-kappa8/src/ground_shrdlu/engine.rs', shrdlu_engine)
write_file('../insa/insa-kappa8/src/ground_shrdlu/result.rs', shrdlu_result)
write_file('../insa/insa-kappa8/src/ground_shrdlu/fixtures.rs', shrdlu_fixtures)
write_file('../insa/insa-truthforge/tests/kappa_shrdlu.rs', shrdlu_test)

# --- PROLOG ---
prolog_mod = """pub mod clause;
pub mod engine;
pub mod result;
pub mod fixtures;

pub use clause::*;
pub use engine::*;
pub use result::*;
pub use fixtures::*;
"""

prolog_clause = """#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RelationId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TermId(pub u16);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FactRow {
    pub relation: RelationId,
    pub subject: TermId,
    pub object: TermId,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HornClause {
    pub id: u16,
    pub head: RelationId,
    pub body1: RelationId,
    pub body2: RelationId, // 0 if none
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProofGoal {
    pub relation: RelationId,
    pub subject: TermId,
    pub object: TermId,
}
"""

prolog_engine = """use insa_instinct::{InstinctByte, KappaByte, PrologByte};
use crate::prove_prolog::clause::{FactRow, HornClause, ProofGoal};
use crate::prove_prolog::result::{ProofResult, ProofStatus};

pub struct ProveProlog;

impl ProveProlog {
    pub fn evaluate(facts: &[FactRow], clauses: &[HornClause], goal: &ProofGoal, max_depth: u8) -> ProofResult {
        let mut detail = PrologByte::empty();
        let mut emits = InstinctByte::empty();
        
        if max_depth == 0 {
            detail = detail.union(PrologByte::DEPTH_EXHAUSTED).union(PrologByte::PROOF_REQUIRES_ESCALATION);
            emits = emits.union(InstinctByte::ESCALATE).union(InstinctByte::INSPECT);
            return ProofResult { status: ProofStatus::DepthExhausted, detail, kappa: KappaByte::PROVE, emits };
        }

        // Direct fact check
        for fact in facts {
            if fact.relation == goal.relation && fact.subject == goal.subject && fact.object == goal.object {
                detail = detail.union(PrologByte::GOAL_PROVED);
                emits = emits.union(InstinctByte::SETTLE);
                return ProofResult { status: ProofStatus::Proved, detail, kappa: KappaByte::PROVE, emits };
            }
        }

        // Rule expansion (1-level for bounded execution)
        for clause in clauses {
            if clause.head == goal.relation {
                detail = detail.union(PrologByte::RULE_MATCHED);
                
                // Assume simple chaining subject -> middle -> object
                let mut body1_proved = false;
                let mut body2_proved = clause.body2.0 == 0; // true if no body2
                
                let mut middle_term = None;

                for fact in facts {
                    if fact.relation == clause.body1 && fact.subject == goal.subject {
                        body1_proved = true;
                        middle_term = Some(fact.object);
                        break;
                    }
                }

                if body1_proved && !body2_proved {
                    for fact in facts {
                        if fact.relation == clause.body2 && Some(fact.subject) == middle_term && fact.object == goal.object {
                            body2_proved = true;
                            break;
                        }
                    }
                }

                if body1_proved && body2_proved {
                    detail = detail.union(PrologByte::GOAL_PROVED);
                    emits = emits.union(InstinctByte::SETTLE);
                    return ProofResult { status: ProofStatus::Proved, detail, kappa: KappaByte::PROVE, emits };
                }
            }
        }

        detail = detail.union(PrologByte::FACT_MISSING).union(PrologByte::GOAL_FAILED);
        emits = emits.union(InstinctByte::RETRIEVE).union(InstinctByte::ASK);
        ProofResult { status: ProofStatus::Failed, detail, kappa: KappaByte::PROVE, emits }
    }
}
"""

prolog_result = """use insa_instinct::{InstinctByte, KappaByte, PrologByte};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ProofStatus {
    Proved = 0,
    Failed = 1,
    DepthExhausted = 2,
}

impl Default for ProofStatus {
    fn default() -> Self {
        Self::Failed
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProofResult {
    pub status: ProofStatus,
    pub detail: PrologByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
}
"""

prolog_fixtures = """use crate::prove_prolog::clause::{FactRow, HornClause, RelationId, TermId, ProofGoal};

pub fn sample_facts() -> [FactRow; 2] {
    [
        FactRow { relation: RelationId(1), subject: TermId(10), object: TermId(20) }, // 10 is parent of 20
        FactRow { relation: RelationId(2), subject: TermId(20), object: TermId(30) }, // 20 is parent of 30
    ]
}

pub fn sample_clause() -> HornClause {
    HornClause {
        id: 1,
        head: RelationId(3), // Grandparent
        body1: RelationId(1),
        body2: RelationId(2),
    }
}
"""

prolog_test = """use insa_instinct::{InstinctByte, PrologByte};
use insa_kappa8::prove_prolog::*;

#[test]
fn test_prolog_prove_rule() {
    let facts = sample_facts();
    let clauses = [sample_clause()];
    let goal = ProofGoal {
        relation: RelationId(3),
        subject: TermId(10),
        object: TermId(30),
    };
    
    let res = ProveProlog::evaluate(&facts, &clauses, &goal, 5);
    assert_eq!(res.status, ProofStatus::Proved);
    assert!(res.detail.contains(PrologByte::RULE_MATCHED));
    assert!(res.detail.contains(PrologByte::GOAL_PROVED));
    assert!(res.emits.contains(InstinctByte::SETTLE));
}

#[test]
fn test_prolog_prove_failed() {
    let facts = sample_facts();
    let clauses = [sample_clause()];
    let goal = ProofGoal {
        relation: RelationId(3),
        subject: TermId(10),
        object: TermId(40), // 40 does not exist
    };
    
    let res = ProveProlog::evaluate(&facts, &clauses, &goal, 5);
    assert_eq!(res.status, ProofStatus::Failed);
    assert!(res.detail.contains(PrologByte::FACT_MISSING));
    assert!(res.detail.contains(PrologByte::GOAL_FAILED));
    assert!(res.emits.contains(InstinctByte::RETRIEVE));
    assert!(res.emits.contains(InstinctByte::ASK));
}
"""

write_file('../insa/insa-kappa8/src/prove_prolog/mod.rs', prolog_mod)
write_file('../insa/insa-kappa8/src/prove_prolog/clause.rs', prolog_clause)
write_file('../insa/insa-kappa8/src/prove_prolog/engine.rs', prolog_engine)
write_file('../insa/insa-kappa8/src/prove_prolog/result.rs', prolog_result)
write_file('../insa/insa-kappa8/src/prove_prolog/fixtures.rs', prolog_fixtures)
write_file('../insa/insa-truthforge/tests/kappa_prolog.rs', prolog_test)


# --- MYCIN ---
mycin_mod = """pub mod rule;
pub mod engine;
pub mod result;
pub mod fixtures;

pub use rule::*;
pub use engine::*;
pub use result::*;
pub use fixtures::*;
"""

mycin_rule = """use insa_types::FieldMask;
use insa_instinct::InstinctByte;

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
"""

mycin_engine = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, KappaByte, MycinByte};
use crate::rule_mycin::rule::{ExpertRule, PolicyEpoch};
use crate::rule_mycin::result::{MycinResult, MycinStatus};

pub struct RuleMycin;

impl RuleMycin {
    pub fn evaluate(rules: &[ExpertRule], present: FieldMask, current_epoch: PolicyEpoch) -> MycinResult {
        let mut detail = MycinByte::empty();
        let mut emits = InstinctByte::empty();
        
        let mut highest_confidence = 0;
        let mut selected_emits = InstinctByte::empty();
        let mut matched_rules = 0;

        for rule in rules {
            if rule.epoch.0 != current_epoch.0 {
                detail = detail.union(MycinByte::POLICY_EPOCH_STALE);
                continue;
            }
            
            detail = detail.union(MycinByte::POLICY_EPOCH_VALID);

            let missing = (present.0 & rule.required.0) ^ rule.required.0;
            let forbidden = present.0 & rule.forbidden.0;
            
            if missing == 0 && forbidden == 0 {
                matched_rules += 1;
                detail = detail.union(MycinByte::RULE_MATCHED);
                
                if rule.confidence.0 > highest_confidence {
                    highest_confidence = rule.confidence.0;
                    selected_emits = rule.emits;
                }
            }
        }

        if matched_rules > 1 {
            detail = detail.union(MycinByte::RULE_CONFLICT).union(MycinByte::EXPERT_REVIEW_REQUIRED);
            emits = emits.union(InstinctByte::INSPECT).union(InstinctByte::ESCALATE);
            return MycinResult { status: MycinStatus::Conflict, detail, kappa: KappaByte::RULE, emits };
        }

        if matched_rules == 1 {
            detail = detail.union(MycinByte::RULE_FIRED);
            if highest_confidence >= 80 {
                detail = detail.union(MycinByte::CONFIDENCE_HIGH);
                emits = selected_emits;
            } else {
                detail = detail.union(MycinByte::CONFIDENCE_LOW).union(MycinByte::EXPERT_REVIEW_REQUIRED);
                emits = selected_emits.union(InstinctByte::INSPECT);
            }
            MycinResult { status: MycinStatus::Fired, detail, kappa: KappaByte::RULE, emits }
        } else {
            emits = emits.union(InstinctByte::ASK).union(InstinctByte::RETRIEVE);
            MycinResult { status: MycinStatus::NoMatch, detail, kappa: KappaByte::RULE, emits }
        }
    }
}
"""

mycin_result = """use insa_instinct::{InstinctByte, KappaByte, MycinByte};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MycinStatus {
    Fired = 0,
    Conflict = 1,
    NoMatch = 2,
}

impl Default for MycinStatus {
    fn default() -> Self {
        Self::NoMatch
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MycinResult {
    pub status: MycinStatus,
    pub detail: MycinByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
}
"""

mycin_fixtures = """use insa_types::FieldMask;
use insa_instinct::InstinctByte;
use crate::rule_mycin::rule::{ExpertRule, ExpertRuleId, Confidence, PolicyEpoch};

pub const FEVER: u64 = 1 << 0;
pub const COUGH: u64 = 1 << 1;

pub fn covid_rule() -> ExpertRule {
    ExpertRule {
        id: ExpertRuleId(1),
        required: FieldMask(FEVER | COUGH),
        forbidden: FieldMask(0),
        emits: InstinctByte::ESCALATE, // High risk
        confidence: Confidence(85),
        epoch: PolicyEpoch(1),
    }
}
"""

mycin_test = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, MycinByte};
use insa_kappa8::rule_mycin::*;

#[test]
fn test_mycin_fire_high_confidence() {
    let rules = [covid_rule()];
    let present = FieldMask(FEVER | COUGH);
    
    let res = RuleMycin::evaluate(&rules, present, PolicyEpoch(1));
    assert_eq!(res.status, MycinStatus::Fired);
    assert!(res.detail.contains(MycinByte::RULE_MATCHED));
    assert!(res.detail.contains(MycinByte::RULE_FIRED));
    assert!(res.detail.contains(MycinByte::CONFIDENCE_HIGH));
    assert!(res.emits.contains(InstinctByte::ESCALATE));
}

#[test]
fn test_mycin_stale_epoch() {
    let rules = [covid_rule()];
    let present = FieldMask(FEVER | COUGH);
    
    let res = RuleMycin::evaluate(&rules, present, PolicyEpoch(2)); // Stale rule
    assert_eq!(res.status, MycinStatus::NoMatch);
    assert!(res.detail.contains(MycinByte::POLICY_EPOCH_STALE));
    assert!(res.emits.contains(InstinctByte::ASK));
}
"""

write_file('../insa/insa-kappa8/src/rule_mycin/mod.rs', mycin_mod)
write_file('../insa/insa-kappa8/src/rule_mycin/rule.rs', mycin_rule)
write_file('../insa/insa-kappa8/src/rule_mycin/engine.rs', mycin_engine)
write_file('../insa/insa-kappa8/src/rule_mycin/result.rs', mycin_result)
write_file('../insa/insa-kappa8/src/rule_mycin/fixtures.rs', mycin_fixtures)
write_file('../insa/insa-truthforge/tests/kappa_mycin.rs', mycin_test)


# --- DENDRAL ---
dendral_mod = """pub mod fragment;
pub mod engine;
pub mod result;
pub mod fixtures;

pub use fragment::*;
pub use engine::*;
pub use result::*;
pub use fixtures::*;
"""

dendral_fragment = """use insa_types::{FieldMask};
use insa_instinct::InstinctByte;

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
"""

dendral_engine = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, KappaByte, DendralByte};
use crate::reconstruct_dendral::fragment::ReconstructionRule;
use crate::reconstruct_dendral::result::{DendralResult, DendralStatus};

pub struct ReconstructDendral;

impl ReconstructDendral {
    pub fn evaluate(rules: &[ReconstructionRule], fragments_present: FieldMask) -> DendralResult {
        let mut detail = DendralByte::empty();
        let mut emits = InstinctByte::empty();
        
        let mut valid_hypotheses = 0;

        for rule in rules {
            let missing = (fragments_present.0 & rule.fragments_required.0) ^ rule.fragments_required.0;
            let forbidden = fragments_present.0 & rule.constraints_forbidden.0;
            
            if missing != 0 {
                detail = detail.union(DendralByte::MISSING_FRAGMENT);
            }
            if forbidden != 0 {
                detail = detail.union(DendralByte::CONSTRAINT_VIOLATION).union(DendralByte::CANDIDATE_PRUNED);
            }
            
            if missing == 0 && forbidden == 0 {
                valid_hypotheses += 1;
                detail = detail.union(DendralByte::FRAGMENTS_SUFFICIENT).union(DendralByte::CANDIDATE_GENERATED);
                emits = emits.union(rule.emits);
            }
        }

        if valid_hypotheses == 1 {
            detail = detail.union(DendralByte::UNIQUE_RECONSTRUCTION);
            emits = emits.union(InstinctByte::SETTLE);
            DendralResult { status: DendralStatus::Unique, detail, kappa: KappaByte::RECONSTRUCT, emits }
        } else if valid_hypotheses > 1 {
            detail = detail.union(DendralByte::MULTIPLE_RECONSTRUCTIONS).union(DendralByte::RECONSTRUCTION_UNSTABLE);
            emits = emits.union(InstinctByte::INSPECT).union(InstinctByte::ASK);
            DendralResult { status: DendralStatus::Ambiguous, detail, kappa: KappaByte::RECONSTRUCT, emits }
        } else {
            emits = emits.union(InstinctByte::RETRIEVE).union(InstinctByte::ASK);
            DendralResult { status: DendralStatus::Failed, detail, kappa: KappaByte::RECONSTRUCT, emits }
        }
    }
}
"""

dendral_result = """use insa_instinct::{InstinctByte, KappaByte, DendralByte};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DendralStatus {
    Unique = 0,
    Ambiguous = 1,
    Failed = 2,
}

impl Default for DendralStatus {
    fn default() -> Self {
        Self::Failed
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DendralResult {
    pub status: DendralStatus,
    pub detail: DendralByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
}
"""

dendral_fixtures = """use insa_types::FieldMask;
use insa_instinct::InstinctByte;
use crate::reconstruct_dendral::fragment::{ReconstructionRule, FragmentId};

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
"""

dendral_test = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, DendralByte};
use insa_kappa8::reconstruct_dendral::*;

#[test]
fn test_dendral_reconstruct_unique() {
    let rules = [identity_reconstruction_rule()];
    let present = FieldMask(FRAG_IP | FRAG_USER_AGENT);
    
    let res = ReconstructDendral::evaluate(&rules, present);
    assert_eq!(res.status, DendralStatus::Unique);
    assert!(res.detail.contains(DendralByte::FRAGMENTS_SUFFICIENT));
    assert!(res.detail.contains(DendralByte::UNIQUE_RECONSTRUCTION));
    assert!(res.emits.contains(InstinctByte::SETTLE));
}

#[test]
fn test_dendral_missing_fragments() {
    let rules = [identity_reconstruction_rule()];
    let present = FieldMask(FRAG_IP); // Missing user agent
    
    let res = ReconstructDendral::evaluate(&rules, present);
    assert_eq!(res.status, DendralStatus::Failed);
    assert!(res.detail.contains(DendralByte::MISSING_FRAGMENT));
    assert!(res.emits.contains(InstinctByte::RETRIEVE));
}
"""

write_file('../insa/insa-kappa8/src/reconstruct_dendral/mod.rs', dendral_mod)
write_file('../insa/insa-kappa8/src/reconstruct_dendral/fragment.rs', dendral_fragment)
write_file('../insa/insa-kappa8/src/reconstruct_dendral/engine.rs', dendral_engine)
write_file('../insa/insa-kappa8/src/reconstruct_dendral/result.rs', dendral_result)
write_file('../insa/insa-kappa8/src/reconstruct_dendral/fixtures.rs', dendral_fixtures)
write_file('../insa/insa-truthforge/tests/kappa_dendral.rs', dendral_test)

print("Generated all remaining KAPPA8 packs.")
