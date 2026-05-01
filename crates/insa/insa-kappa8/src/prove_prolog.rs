use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{InstinctByte, KappaByte, KappaDetail16, PrologByte};
use insa_types::FieldMask;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelationId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Validity(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fact {
    pub rel: RelationId,
    pub a: TermId,
    pub b: TermId,
    pub validity: Validity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HornClause {
    pub head: RelationId,
    pub body1: RelationId,
    pub body2: Option<RelationId>,
    pub budget: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofStatus {
    Proved,
    Failed,
    Exhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProofResult {
    pub status: ProofStatus,
    pub depth: u8,
    pub support: FieldMask,
    pub prolog: PrologByte,
}

pub struct ProveProlog {
    pub facts: &'static [Fact],
    pub clauses: &'static [HornClause],
    pub goal_rel: RelationId,
}

impl ProveProlog {
    pub fn prove(&self, _ctx: &ClosureCtx) -> ProofResult {
        for fact in self.facts {
            if fact.rel == self.goal_rel && fact.validity.0 > 0 {
                return ProofResult {
                    status: ProofStatus::Proved,
                    depth: 1,
                    support: FieldMask::empty(),
                    prolog: PrologByte::empty().union(PrologByte::GOAL_PROVED),
                };
            }
        }

        for clause in self.clauses {
            if clause.head == self.goal_rel {
                let mut b1_proved = false;
                for fact in self.facts {
                    if fact.rel == clause.body1 && fact.validity.0 > 0 {
                        b1_proved = true;
                        break;
                    }
                }
                if b1_proved {
                    if let Some(b2) = clause.body2 {
                        let mut b2_proved = false;
                        for fact in self.facts {
                            if fact.rel == b2 && fact.validity.0 > 0 {
                                b2_proved = true;
                                break;
                            }
                        }
                        if b2_proved {
                            return ProofResult {
                                status: ProofStatus::Proved,
                                depth: 2,
                                support: FieldMask::empty(),
                                prolog: PrologByte::empty()
                                    .union(PrologByte::RULE_MATCHED)
                                    .union(PrologByte::GOAL_PROVED),
                            };
                        }
                    } else {
                        return ProofResult {
                            status: ProofStatus::Proved,
                            depth: 2,
                            support: FieldMask::empty(),
                            prolog: PrologByte::empty()
                                .union(PrologByte::RULE_MATCHED)
                                .union(PrologByte::GOAL_PROVED),
                        };
                    }
                }
            }
        }

        ProofResult {
            status: ProofStatus::Failed,
            depth: 2,
            support: FieldMask::empty(),
            prolog: PrologByte::empty().union(PrologByte::GOAL_FAILED),
        }
    }
}

impl CollapseEngine for ProveProlog {
    fn evaluate(&self, _ctx: &ClosureCtx) -> CollapseResult {
        let res = self.prove(_ctx);
        let (status, emits) = match res.status {
            ProofStatus::Proved => (CollapseStatus::Success, InstinctByte::SETTLE),
            ProofStatus::Failed => (CollapseStatus::Failed, InstinctByte::REFUSE),
            ProofStatus::Exhausted => (CollapseStatus::Partial, InstinctByte::ESCALATE),
        };

        let mut detail = KappaDetail16::empty();
        detail.kappa = KappaByte::PROVE;
        detail.prolog = res.prolog;

        CollapseResult {
            detail,
            instincts: emits,
            support: Cog8Support::new(res.support),
            status,
        }
    }
}
