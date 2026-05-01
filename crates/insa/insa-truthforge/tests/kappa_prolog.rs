use insa_instinct::{InstinctByte, PrologByte};
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
