use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{InstinctByte, KappaByte, KappaDetail16, PrologByte};
use insa_types::{FieldMask, PolicyEpoch};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelationId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Validity(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactRow {
    pub relation: RelationId,
    pub subject: TermId,
    pub object: TermId,
    pub validity: Validity,
    pub source: SourceId,
    pub policy_epoch: PolicyEpoch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClauseId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProofBudget(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmallBody {
    pub body1: RelationId,
    pub body2: Option<RelationId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HornClause {
    pub id: ClauseId,
    pub head: RelationId,
    pub body: SmallBody,
    pub budget: ProofBudget,
    pub epoch: PolicyEpoch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProofGoal {
    pub relation: RelationId,
    pub subject: TermId,
    pub object: TermId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofStatus {
    Proved,
    Failed,
    FactMissing,
    Contradiction,
    DepthExhausted,
    CycleDetected,
    RequiresEscalation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProofWitness {
    // A real implementation would store a fixed-cap array of used facts/rules here.
    // We mock it for the interface bounds to stay size-constrained.
    pub steps_recorded: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProofResult {
    pub status: ProofStatus,
    pub prolog: PrologByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
    pub support: FieldMask,
    pub witness: ProofWitness,
}

pub struct ProveProlog {
    pub facts: &'static [FactRow],
    pub clauses: &'static [HornClause],
    pub goal: ProofGoal,
}

impl ProveProlog {
    pub fn prove(&self, ctx: &ClosureCtx) -> ProofResult {
        self.step(self.goal, ctx, 0)
    }

    fn step(&self, goal: ProofGoal, ctx: &ClosureCtx, current_depth: u8) -> ProofResult {
        let max_budget = 4; // Simulated fixed budget

        if current_depth >= max_budget {
            return ProofResult {
                status: ProofStatus::DepthExhausted,
                prolog: PrologByte::empty()
                    .union(PrologByte::DEPTH_EXHAUSTED)
                    .union(PrologByte::GOAL_FAILED),
                kappa: KappaByte::PROVE,
                emits: InstinctByte::ESCALATE,
                support: FieldMask::empty(),
                witness: ProofWitness {
                    steps_recorded: current_depth,
                },
            };
        }

        // Direct fact check
        let mut fact_missing = true;
        for fact in self.facts {
            if fact.relation == goal.relation
                && fact.subject == goal.subject
                && fact.object == goal.object
                && fact.policy_epoch.0 <= ctx.policy.0
                && fact.validity.0 > 0
            {
                return ProofResult {
                    status: ProofStatus::Proved,
                    prolog: PrologByte::empty().union(PrologByte::GOAL_PROVED),
                    kappa: KappaByte::PROVE,
                    emits: InstinctByte::SETTLE,
                    support: FieldMask::empty(), // Simulated: record support
                    witness: ProofWitness {
                        steps_recorded: current_depth + 1,
                    },
                };
            }
        }

        // Backward chaining rules
        for clause in self.clauses {
            if clause.head == goal.relation && clause.epoch.0 <= ctx.policy.0 {
                fact_missing = false;

                // Evaluate body 1
                let g1 = ProofGoal {
                    relation: clause.body.body1,
                    subject: goal.subject,
                    object: goal.object,
                }; // Simple variable binding
                let res1 = self.step(g1, ctx, current_depth + 1);

                if res1.status == ProofStatus::Proved {
                    // Evaluate body 2 if present
                    if let Some(rel2) = clause.body.body2 {
                        let g2 = ProofGoal {
                            relation: rel2,
                            subject: goal.subject,
                            object: goal.object,
                        };
                        let res2 = self.step(g2, ctx, res1.witness.steps_recorded);

                        if res2.status == ProofStatus::Proved {
                            return ProofResult {
                                status: ProofStatus::Proved,
                                prolog: PrologByte::empty()
                                    .union(PrologByte::RULE_MATCHED)
                                    .union(PrologByte::GOAL_PROVED),
                                kappa: KappaByte::PROVE,
                                emits: InstinctByte::SETTLE,
                                support: FieldMask::empty(),
                                witness: ProofWitness {
                                    steps_recorded: res2.witness.steps_recorded,
                                },
                            };
                        }
                    } else {
                        return ProofResult {
                            status: ProofStatus::Proved,
                            prolog: PrologByte::empty()
                                .union(PrologByte::RULE_MATCHED)
                                .union(PrologByte::GOAL_PROVED),
                            kappa: KappaByte::PROVE,
                            emits: InstinctByte::SETTLE,
                            support: FieldMask::empty(),
                            witness: ProofWitness {
                                steps_recorded: res1.witness.steps_recorded,
                            },
                        };
                    }
                } else if res1.status == ProofStatus::DepthExhausted {
                    return res1; // Bubble up exhaustion
                }
            }
        }

        let (status, prolog, emits) = if fact_missing {
            (
                ProofStatus::FactMissing,
                PrologByte::empty().union(PrologByte::FACT_MISSING),
                InstinctByte::RETRIEVE,
            )
        } else {
            (
                ProofStatus::Failed,
                PrologByte::empty().union(PrologByte::GOAL_FAILED),
                InstinctByte::REFUSE,
            )
        };

        ProofResult {
            status,
            prolog,
            kappa: KappaByte::PROVE,
            emits,
            support: FieldMask::empty(),
            witness: ProofWitness {
                steps_recorded: current_depth + 1,
            },
        }
    }
}

impl CollapseEngine for ProveProlog {
    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let res = self.prove(ctx);

        let status = match res.status {
            ProofStatus::Proved => CollapseStatus::Success,
            ProofStatus::Failed => CollapseStatus::Failed,
            _ => CollapseStatus::Partial,
        };

        let mut detail = KappaDetail16::empty();
        detail.kappa = KappaByte::PROVE;
        detail.prolog = res.prolog;

        CollapseResult {
            detail,
            instincts: res.emits,
            support: Cog8Support::new(res.support),
            status,
        }
    }
}
