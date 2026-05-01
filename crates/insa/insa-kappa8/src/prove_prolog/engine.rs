use crate::prove_prolog::clause::{FactRow, HornClause, ProofGoal};
use crate::prove_prolog::result::{ProofResult, ProofStatus};
use insa_instinct::{InstinctByte, KappaByte, PrologByte};

pub struct ProveProlog;

impl ProveProlog {
    pub fn evaluate(
        facts: &[FactRow],
        clauses: &[HornClause],
        goal: &ProofGoal,
        max_depth: u8,
    ) -> ProofResult {
        let mut detail = PrologByte::empty();
        let mut emits = InstinctByte::empty();

        if max_depth == 0 {
            detail = detail
                .union(PrologByte::DEPTH_EXHAUSTED)
                .union(PrologByte::PROOF_REQUIRES_ESCALATION);
            emits = emits
                .union(InstinctByte::ESCALATE)
                .union(InstinctByte::INSPECT);
            return ProofResult {
                status: ProofStatus::DepthExhausted,
                detail,
                kappa: KappaByte::PROVE,
                emits,
            };
        }

        // Direct fact check
        for fact in facts {
            if fact.relation == goal.relation
                && fact.subject == goal.subject
                && fact.object == goal.object
            {
                detail = detail.union(PrologByte::GOAL_PROVED);
                emits = emits.union(InstinctByte::SETTLE);
                return ProofResult {
                    status: ProofStatus::Proved,
                    detail,
                    kappa: KappaByte::PROVE,
                    emits,
                };
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
                        if fact.relation == clause.body2
                            && Some(fact.subject) == middle_term
                            && fact.object == goal.object
                        {
                            body2_proved = true;
                            break;
                        }
                    }
                }

                if body1_proved && body2_proved {
                    detail = detail.union(PrologByte::GOAL_PROVED);
                    emits = emits.union(InstinctByte::SETTLE);
                    return ProofResult {
                        status: ProofStatus::Proved,
                        detail,
                        kappa: KappaByte::PROVE,
                        emits,
                    };
                }
            }
        }

        detail = detail
            .union(PrologByte::FACT_MISSING)
            .union(PrologByte::GOAL_FAILED);
        emits = emits.union(InstinctByte::RETRIEVE).union(InstinctByte::ASK);
        ProofResult {
            status: ProofStatus::Failed,
            detail,
            kappa: KappaByte::PROVE,
            emits,
        }
    }
}
