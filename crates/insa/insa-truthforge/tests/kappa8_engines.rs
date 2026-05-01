use insa_instinct::{InstinctByte, KappaByte};
use insa_kappa8::precondition_strips::{ActionSchema, PreconditionStrips};
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
        dictionary: DictionaryDigest::default(),
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
    let res_success = strips.evaluate(&ctx_success);
    assert_eq!(res_success.status, insa_kappa8::CollapseStatus::Success);
    assert_eq!(res_success.detail.kappa, KappaByte::PRECONDITION);

    // Test 2: Missing preconditions
    let ctx_missing = build_empty_ctx(0b01);
    let res_missing = strips.evaluate(&ctx_missing);
    assert_eq!(res_missing.status, insa_kappa8::CollapseStatus::Failed);
    assert!(res_missing.instincts.contains(InstinctByte::RETRIEVE)); // retrieves missing

    // Test 3: Forbidden present
    let ctx_forbidden = build_empty_ctx(0b111);
    let res_forbidden = strips.evaluate(&ctx_forbidden);
    assert_eq!(res_forbidden.status, insa_kappa8::CollapseStatus::Failed);
    assert!(res_forbidden.instincts.contains(InstinctByte::REFUSE)); // blocks forbidden
}

static OP_1: GapOperator = GapOperator {
    id: 1,
    resolves: FieldMask(0b100),
    emits: InstinctByte::RETRIEVE,
};

static OP_2: GapOperator = GapOperator {
    id: 2,
    resolves: FieldMask(0b1000), // unrelated bit
    emits: InstinctByte::ASK,
};

static GPS_OPS_1: &[GapOperator] = &[OP_1];
static GPS_OPS_2: &[GapOperator] = &[OP_2];

#[test]
fn truthforge_reduce_gap_gps() {
    let goal = GoalState {
        required: FieldMask(0b111),
        forbidden: FieldMask(0),
    };

    let gps = ReduceGapGps {
        goal,
        operators: GPS_OPS_1,
    };

    // Test 1: Goal already satisfied
    let ctx_done = build_empty_ctx(0b111);
    let res_done = gps.evaluate(&ctx_done);
    assert_eq!(res_done.status, insa_kappa8::CollapseStatus::Success);
    assert!(res_done.instincts.contains(InstinctByte::SETTLE));

    // Test 2: Gap exists, operator can reduce it
    let ctx_gap = build_empty_ctx(0b011);
    let res_gap = gps.evaluate(&ctx_gap);
    assert_eq!(res_gap.status, insa_kappa8::CollapseStatus::Partial);
    assert!(res_gap.instincts.contains(InstinctByte::RETRIEVE));

    // Test 3: Gap exists, no operator can reduce it
    let ctx_stuck = build_empty_ctx(0b001); // missing bit 1 and 2
    let gps_stuck = ReduceGapGps {
        goal,
        operators: GPS_OPS_2,
    };
    let res_stuck = gps_stuck.evaluate(&ctx_stuck);
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
    let res_partial = eliza.evaluate(&ctx_partial);
    assert_eq!(res_partial.status, insa_kappa8::CollapseStatus::Partial);
    assert!(res_partial.instincts.contains(InstinctByte::INSPECT));
    assert!(res_partial.instincts.contains(InstinctByte::ASK));

    // Test 2: Pattern match and no missing slots
    let ctx_success = build_empty_ctx(0b11);
    let res_success = eliza.evaluate(&ctx_success);
    assert_eq!(res_success.status, insa_kappa8::CollapseStatus::Success);
    assert!(res_success.instincts.contains(InstinctByte::INSPECT));
    assert!(!res_success.instincts.contains(InstinctByte::ASK));
}
