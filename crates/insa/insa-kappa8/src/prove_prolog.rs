use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{InstinctByte, KappaByte};
use insa_types::FieldMask;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RelationId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermId(pub u32);

#[derive(Debug, Clone)]
pub struct Fact {
    pub rel: RelationId,
    pub a: TermId,
    pub b: TermId,
    pub validity: u8,
}

#[derive(Debug, Clone)]
pub struct HornClause {
    pub head: RelationId,
    pub body_a: RelationId,
    pub body_b: Option<RelationId>, // Max 2 clauses for bounded stack
    pub budget: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofStatus {
    Proved,
    Failed,
    Exhausted,
}

#[derive(Debug, Clone)]
pub struct ProofResult {
    pub status: ProofStatus,
    pub depth: u8,
    pub support: FieldMask,
}

pub struct ProveProlog {
    pub facts: &'static [Fact],
    pub rules: &'static [HornClause],
}

impl ProveProlog {
    pub fn prove_goal(&self, goal: RelationId, depth: u8) -> ProofResult {
        if depth > 10 {
            return ProofResult {
                status: ProofStatus::Exhausted,
                depth,
                support: FieldMask(0),
            };
        }

        // Direct fact check
        for fact in self.facts {
            if fact.rel == goal {
                return ProofResult {
                    status: ProofStatus::Proved,
                    depth,
                    support: FieldMask(fact.validity as u64),
                };
            }
        }

        // Rule expansion (simplified backward chaining)
        for rule in self.rules {
            if rule.head == goal {
                let res_a = self.prove_goal(rule.body_a, depth + 1);
                if res_a.status != ProofStatus::Proved {
                    continue;
                }

                if let Some(body_b) = rule.body_b {
                    let res_b = self.prove_goal(body_b, depth + 1);
                    if res_b.status == ProofStatus::Proved {
                        return ProofResult {
                            status: ProofStatus::Proved,
                            depth: core::cmp::max(res_a.depth, res_b.depth),
                            support: FieldMask(res_a.support.0 | res_b.support.0),
                        };
                    }
                } else {
                    return res_a;
                }
            }
        }

        ProofResult {
            status: ProofStatus::Failed,
            depth,
            support: FieldMask(0),
        }
    }
}

impl CollapseEngine for ProveProlog {
    const KAPPA_BIT: KappaByte = KappaByte::PROVE;

    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        // Evaluate an implied goal based on context
        let implied_goal = RelationId(1);
        let res = self.prove_goal(implied_goal, 0);

        let emits = match res.status {
            ProofStatus::Proved => InstinctByte::SETTLE,
            ProofStatus::Failed => InstinctByte::REFUSE.union(InstinctByte::ASK),
            ProofStatus::Exhausted => InstinctByte::ESCALATE,
        };

        CollapseResult {
            kappa: Self::KAPPA_BIT,
            instincts: emits,
            support: Cog8Support::new(FieldMask(ctx.present.0 | res.support.0)),
            status: match res.status {
                ProofStatus::Proved => CollapseStatus::Success,
                ProofStatus::Failed => CollapseStatus::Failed,
                ProofStatus::Exhausted => CollapseStatus::Partial,
            },
        }
    }
}
