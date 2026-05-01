use insa_instinct::{InstinctByte, KappaByte, PrologByte, StripsByte};
use insa_kappa8::precondition_strips::{ActionSchema, PreconditionStrips};
use insa_kappa8::prove_prolog::{
    ClauseId, FactRow, HornClause, ProofBudget, ProofGoal, ProveProlog, RelationId, SmallBody,
    SourceId, TermId, Validity,
};
use insa_kappa8::reduce_gap_gps::{GapOperator, GoalState, ReduceGapGps};
use insa_kappa8::reflect_eliza::{ReflectEliza, ReflectPattern};
use insa_kappa8::ClosureCtx;
use insa_kappa8::CollapseEngine;
use insa_types::{CompletedMask, DictionaryDigest, FieldMask, ObjectRef, PolicyEpoch};

fn build_empty_ctx(present: u64) -> ClosureCtx {
    ClosureCtx {
        present: FieldMask(present),
        completed: CompletedMask::empty(),
        object: ObjectRef(0),
        policy: PolicyEpoch(0),
        dictionary: DictionaryDigest([0; 32]),
    }
}

static SCHEMA_1: ActionSchema = ActionSchema {
    id: 1,
    preconditions: FieldMask(0b11),
    forbidden: FieldMask(0b100),
    add_effects: FieldMask(0),
    clear_effects: FieldMask(0),
};

static STRIPS_SCHEMAS: &[ActionSchema] = &[SCHEMA_1];

#[test]
fn truthforge_precondition_strips() {
    let strips = PreconditionStrips {
        schemas: STRIPS_SCHEMAS,
    };

    // Test 1: Satisfied (has preconditions, no forbidden)
    let ctx_success = build_empty_ctx(0b11);
    let res_success = CollapseEngine::evaluate(&strips, &ctx_success);
    assert_eq!(res_success.status, insa_kappa8::CollapseStatus::Success);
    assert_eq!(res_success.detail.kappa, KappaByte::PRECONDITION);
    assert!(res_success
        .detail
        .strips
        .contains(StripsByte::PRECONDITIONS_SATISFIED));

    // Test 2: Missing preconditions
    let ctx_missing = build_empty_ctx(0b01);
    let res_missing = CollapseEngine::evaluate(&strips, &ctx_missing);
    assert_eq!(res_missing.status, insa_kappa8::CollapseStatus::Failed);
    assert!(res_missing.instincts.contains(InstinctByte::RETRIEVE)); // retrieves missing
    assert!(res_missing
        .detail
        .strips
        .contains(StripsByte::MISSING_REQUIRED));

    // Test 3: Forbidden present
    let ctx_forbidden = build_empty_ctx(0b111);
    let res_forbidden = CollapseEngine::evaluate(&strips, &ctx_forbidden);
    assert_eq!(res_forbidden.status, insa_kappa8::CollapseStatus::Failed);
    assert!(res_forbidden.instincts.contains(InstinctByte::REFUSE)); // blocks forbidden
    assert!(res_forbidden
        .detail
        .strips
        .contains(StripsByte::FORBIDDEN_PRESENT));
}

static OP_1: GapOperator = GapOperator {
    id: 1,
    required_preconditions: FieldMask(0),
    resolves: FieldMask(0b100),
    emits: InstinctByte::RETRIEVE,
};

static OP_2: GapOperator = GapOperator {
    id: 2,
    required_preconditions: FieldMask(0),
    resolves: FieldMask(0b1000), // unrelated bit
    emits: InstinctByte::ASK,
};

#[test]
fn truthforge_reduce_gap_gps() {
    let goal = GoalState {
        required: FieldMask(0n111),
        forbidden: FieldMask(0),
        completed: CompletedMask::empty(),
    };

    let gps = ReduceGapGps {
        goal: goal.clone(),
        operators: &[OP_1],
        max_depth: 5,
    };

    // Test 1: Goal already satisfied
    let ctx_done = build_empty_ctx(0b111);
    let res_done = CollapseEngine::evaluate(&gps, &ctx_done);
    assert_eq!(res_done.status, insa_kappa8::CollapseStatus::Success);
    assert!(res_done.instincts.contains(InstinctByte::SETTLE));

    // Test 2: Gap exists, operator can reduce it
    let ctx_gap = build_empty_ctx(0b011);
    let res_gap = CollapseEngine::evaluate(&gps, &ctx_gap);
    assert_eq!(res_gap.status, insa_kappa8::CollapseStatus::Partial);
    assert!(res_gap.instincts.contains(InstinctByte::RETRIEVE));

    // Test 3: Gap exists, no operator can reduce it
    let ctx_stuck = build_empty_ctx(0b001); // missing bit 1 and 2
    let gps_stuck = ReduceGapGps {
        goal: goal.clone(),
        operators: &[OP_2],
        max_depth: 5,
    };
    let res_stuck = CollapseEngine::evaluate(&gps_stuck, &ctx_stuck);
    assert_eq!(res_stuck.status, insa_kappa8::CollapseStatus::Failed);
    assert!(res_stuck.instincts.contains(InstinctByte::ESCALATE));
}

static PATTERN_1: ReflectPattern = ReflectPattern {
    id: 1,
    required_context: FieldMask(0b01),
    template_id: 1,
    emits: InstinctByte::INSPECT,
};

static ELIZA_PATTERNS: &[ReflectPattern] = &[PATTERN_1];

#[test]
fn truthforge_reflect_eliza() {
    let eliza = ReflectEliza {
        patterns: ELIZA_PATTERNS,
        expected_slots: FieldMask(0b11),
    };

    // Test 1: Pattern match but missing slot -> Ask
    let ctx_partial = build_empty_ctx(0b01);
    let res_partial = CollapseEngine::evaluate(&eliza, &ctx_partial);
    assert_eq!(res_partial.status, insa_kappa8::CollapseStatus::Partial);
    assert!(res_partial.instincts.contains(InstinctByte::INSPECT));
    assert!(res_partial.instincts.contains(InstinctByte::ASK));

    // Test 2: Pattern match and no missing slots
    let ctx_success = build_empty_ctx(0b11);
    let res_success = CollapseEngine::evaluate(&eliza, &ctx_success);
    assert_eq!(res_success.status, insa_kappa8::CollapseStatus::Success);
    assert!(res_success.instincts.contains(InstinctByte::INSPECT));
    assert!(!res_success.instincts.contains(InstinctByte::ASK));
}

static PROLOG_FACT_1: FactRow = FactRow {
    relation: RelationId(1),
    subject: TermId(1),
    object: TermId(2),
    validity: Validity(1),
    source: SourceId(1),
    policy_epoch: PolicyEpoch(0),
};

static PROLOG_FACTS: &[FactRow] = &[PROLOG_FACT_1];

static PROLOG_RULE_1: HornClause = HornClause {
    id: ClauseId(1),
    head: RelationId(2),
    body: SmallBody {
        body1: RelationId(1),
        body2: None,
    },
    budget: ProofBudget(5),
    epoch: PolicyEpoch(0),
};

static PROLOG_RULES: &[HornClause] = &[PROLOG_RULE_1];

#[test]
fn truthforge_prove_prolog() {
    let prolog_fact = ProveProlog {
        facts: PROLOG_FACTS,
        clauses: &[],
        goal: ProofGoal {
            relation: RelationId(1),
            subject: TermId(1),
            object: TermId(2),
        },
    };

    let ctx = build_empty_ctx(0);

    // Test 1: Direct fact prove
    let res_fact = CollapseEngine::evaluate(&prolog_fact, &ctx);
    assert_eq!(res_fact.status, insa_kappa8::CollapseStatus::Success);
    assert!(res_fact.detail.prolog.contains(PrologByte::GOAL_PROVED));

    // Test 2: Rule expansion prove
    let prolog_rule = ProveProlog {
        facts: PROLOG_FACTS,
        clauses: PROLOG_RULES,
        goal: ProofGoal {
            relation: RelationId(2),
            subject: TermId(1),
            object: TermId(2),
        },
    };

    let res_rule = CollapseEngine::evaluate(&prolog_rule, &ctx);
    assert_eq!(res_rule.status, insa_kappa8::CollapseStatus::Success);
    assert!(res_rule.detail.prolog.contains(PrologByte::RULE_MATCHED));
    assert!(res_rule.detail.prolog.contains(PrologByte::GOAL_PROVED));

    // Test 3: Fact missing
    let prolog_missing = ProveProlog {
        facts: PROLOG_FACTS,
        clauses: PROLOG_RULES,
        goal: ProofGoal {
            relation: RelationId(3),
            subject: TermId(1),
            object: TermId(2),
        },
    };
    let res_missing = CollapseEngine::evaluate(&prolog_missing, &ctx);
    assert_eq!(res_missing.status, insa_kappa8::CollapseStatus::Failed); // Engine fails on missing entirely currently
    assert!(res_missing.detail.prolog.contains(PrologByte::GOAL_FAILED));
}
