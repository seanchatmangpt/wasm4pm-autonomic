import sys

files = {}

files["../insa/insa-kappa8/src/reconstruct_dendral/mod.rs"] = """pub mod candidate;
pub mod constraint;
pub mod engine;
pub mod fragment;
pub mod result;
pub mod witness;

pub use candidate::*;
pub use constraint::*;
pub use engine::*;
pub use fragment::*;
pub use result::*;
pub use witness::*;
"""

files["../insa/insa-kappa8/src/reconstruct_dendral/fragment.rs"] = """use insa_types::FieldMask;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct FragmentId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct ObjectRef(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct DigestRef(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct SourceId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TimeRange {
    pub start: u64,
    pub end: u64,
}

impl TimeRange {
    #[inline(always)]
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FragmentKind {
    LogEvent = 1,
    BadgeEvent = 2,
    RepoEvent = 3,
    IamEvent = 4,
    VendorRecord = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Fragment {
    pub id: FragmentId,
    pub kind: FragmentKind,
    pub object: Option<ObjectRef>,
    pub time: TimeRange,
    pub asserts: FieldMask,
    pub digest: DigestRef,
    pub source: SourceId,
}
"""

files["../insa/insa-kappa8/src/reconstruct_dendral/constraint.rs"] = """use crate::reconstruct_dendral::fragment::FragmentId;
use insa_types::{FieldMask, PolicyEpoch};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct ConstraintId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintKind {
    TimeOrder { before: FragmentId, after: FragmentId },
    SameObject { a: FragmentId, b: FragmentId },
    RequiredMask { mask: FieldMask },
    ForbiddenMask { mask: FieldMask },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconstructionConstraint {
    pub id: ConstraintId,
    pub kind: ConstraintKind,
    pub valid_time: crate::reconstruct_dendral::fragment::TimeRange,
    pub epoch: PolicyEpoch,
}
"""

files["../insa/insa-kappa8/src/reconstruct_dendral/candidate.rs"] = """use insa_types::FieldMask;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct CandidateId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct ReconstructionCandidate {
    pub id: CandidateId,
    pub support: FieldMask,
    pub inferred: FieldMask,
    pub satisfied_constraints: u64,
    pub violated_constraints: u64,
    pub fragments_used: u64,
    pub score: i32,
}

pub const MAX_CANDIDATES: usize = 16;
pub const MAX_FRAGMENTS: usize = 64;

pub struct CandidateArena {
    pub candidates: [ReconstructionCandidate; MAX_CANDIDATES],
    pub len: usize,
}

impl CandidateArena {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            candidates: [ReconstructionCandidate {
                id: CandidateId(0),
                support: FieldMask(0),
                inferred: FieldMask(0),
                satisfied_constraints: 0,
                violated_constraints: 0,
                fragments_used: 0,
                score: 0,
            }; MAX_CANDIDATES],
            len: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, candidate: ReconstructionCandidate) -> Result<(), &'static str> {
        if self.len < MAX_CANDIDATES {
            self.candidates[self.len] = candidate;
            self.len += 1;
            Ok(())
        } else {
            Err("Candidate explosion: budget exhausted")
        }
    }
}
"""

files["../insa/insa-kappa8/src/reconstruct_dendral/witness.rs"] = """#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct ReconstructionWitnessId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct ReconstructionWitness {
    pub id: ReconstructionWitnessId,
    pub fragments_evaluated: u64,
    pub candidates_pruned: u32,
    pub final_candidate_count: u16,
}
"""

files["../insa/insa-kappa8/src/reconstruct_dendral/result.rs"] = """use crate::reconstruct_dendral::candidate::CandidateId;
use crate::reconstruct_dendral::witness::ReconstructionWitnessId;
use insa_instinct::{DendralByte, InstinctByte, KappaByte};
use insa_types::FieldMask;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconstructionStatus {
    Success,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ReconstructionResult {
    pub status: ReconstructionStatus,
    pub detail: DendralByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
    pub selected: Option<CandidateId>,
    pub support: FieldMask,
    pub witness_index: ReconstructionWitnessId,
}
"""

files["../insa/insa-kappa8/src/reconstruct_dendral/engine.rs"] = """use crate::reconstruct_dendral::candidate::{CandidateArena, CandidateId, ReconstructionCandidate, MAX_CANDIDATES, MAX_FRAGMENTS};
use crate::reconstruct_dendral::constraint::{ConstraintKind, ReconstructionConstraint};
use crate::reconstruct_dendral::fragment::{Fragment, FragmentId};
use crate::reconstruct_dendral::result::{ReconstructionResult, ReconstructionStatus};
use crate::reconstruct_dendral::witness::{ReconstructionWitness, ReconstructionWitnessId};
use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{DendralByte, InstinctByte, KappaByte, KappaDetail16};
use insa_types::FieldMask;

pub struct ReconstructDendral {
    pub fragments: &'static [Fragment],
    pub constraints: &'static [ReconstructionConstraint],
}

impl ReconstructDendral {
    #[inline]
    fn fragment_index(&self, id: FragmentId) -> Option<usize> {
        for (i, f) in self.fragments.iter().enumerate() {
            if f.id == id {
                return Some(i);
            }
        }
        None
    }

    fn recalc_support(&self, fragments_used: u64) -> FieldMask {
        let mut support = 0;
        for (i, f) in self.fragments.iter().enumerate() {
            if (fragments_used & (1 << i)) != 0 {
                support |= f.asserts.0;
            }
        }
        FieldMask(support)
    }

    pub fn reconstruct(&self, ctx: &ClosureCtx) -> ReconstructionResult {
        let mut dendral = DendralByte::empty();

        if self.fragments.len() > MAX_FRAGMENTS {
            return ReconstructionResult {
                status: ReconstructionStatus::Failed,
                selected: None,
                emits: InstinctByte::ESCALATE.union(InstinctByte::INSPECT),
                dendral: dendral.union(DendralByte::RECONSTRUCTION_UNSTABLE),
                support: FieldMask::empty(),
                witness_index: ReconstructionWitnessId(0),
            };
        }

        let mut base_fragments = 0u64;
        let mut base_support = 0u64;

        for (i, frag) in self.fragments.iter().enumerate() {
            // Fragment is relevant if it asserts bounds present in the field context or asserts nothing
            if (ctx.present.0 & frag.asserts.0) != 0 || frag.asserts.0 == 0 {
                base_fragments |= 1 << i;
                base_support |= frag.asserts.0;
            }
        }

        if base_fragments == 0 {
            return ReconstructionResult {
                status: ReconstructionStatus::Failed,
                selected: None,
                emits: InstinctByte::RETRIEVE.union(InstinctByte::ASK),
                dendral: dendral.union(DendralByte::MISSING_FRAGMENT),
                support: FieldMask::empty(),
                witness_index: ReconstructionWitnessId(0),
            };
        }

        dendral = dendral.union(DendralByte::FRAGMENTS_SUFFICIENT);

        let mut arena = CandidateArena::new();
        let _ = arena.push(ReconstructionCandidate {
            id: CandidateId(1),
            support: FieldMask(base_support),
            inferred: FieldMask::empty(),
            satisfied_constraints: 0,
            violated_constraints: 0,
            fragments_used: base_fragments,
            score: base_fragments.count_ones() as i32,
        });

        dendral = dendral.union(DendralByte::CANDIDATE_GENERATED);

        let mut next_id = 2;
        let mut current_idx = 0;
        let mut pruned_count = 0;

        // Bounded Branching Candidate Pruning/Generation
        while current_idx < arena.len && arena.len < MAX_CANDIDATES {
            let cand = arena.candidates[current_idx];
            let mut violated = false;

            for cons in self.constraints {
                if cons.epoch.0 > ctx.policy.0 {
                    continue;
                }

                match cons.kind {
                    ConstraintKind::TimeOrder { before, after } => {
                        if let (Some(b), Some(a)) = (self.fragment_index(before), self.fragment_index(after)) {
                            if (cand.fragments_used & (1 << b) != 0) && (cand.fragments_used & (1 << a) != 0) {
                                if self.fragments[b].time.start > self.fragments[a].time.start {
                                    violated = true;
                                    
                                    // Branch to resolve contradiction
                                    let mut c1 = cand;
                                    c1.fragments_used &= !(1 << b);
                                    c1.id = CandidateId(next_id);
                                    c1.support = self.recalc_support(c1.fragments_used);
                                    c1.score = c1.fragments_used.count_ones() as i32;
                                    next_id += 1;

                                    let mut c2 = cand;
                                    c2.fragments_used &= !(1 << a);
                                    c2.id = CandidateId(next_id);
                                    c2.support = self.recalc_support(c2.fragments_used);
                                    c2.score = c2.fragments_used.count_ones() as i32;
                                    next_id += 1;

                                    arena.candidates[current_idx] = arena.candidates[arena.len - 1];
                                    arena.len -= 1;
                                    pruned_count += 1;
                                    
                                    let _ = arena.push(c1);
                                    let _ = arena.push(c2);
                                    break;
                                }
                            }
                        }
                    }
                    ConstraintKind::SameObject { a, b } => {
                        if let (Some(ai), Some(bi)) = (self.fragment_index(a), self.fragment_index(b)) {
                            if (cand.fragments_used & (1 << ai) != 0) && (cand.fragments_used & (1 << bi) != 0) {
                                if self.fragments[ai].object != self.fragments[bi].object {
                                    violated = true;
                                    
                                    let mut c1 = cand;
                                    c1.fragments_used &= !(1 << ai);
                                    c1.id = CandidateId(next_id);
                                    c1.support = self.recalc_support(c1.fragments_used);
                                    c1.score = c1.fragments_used.count_ones() as i32;
                                    next_id += 1;

                                    let mut c2 = cand;
                                    c2.fragments_used &= !(1 << bi);
                                    c2.id = CandidateId(next_id);
                                    c2.support = self.recalc_support(c2.fragments_used);
                                    c2.score = c2.fragments_used.count_ones() as i32;
                                    next_id += 1;

                                    arena.candidates[current_idx] = arena.candidates[arena.len - 1];
                                    arena.len -= 1;
                                    pruned_count += 1;
                                    
                                    let _ = arena.push(c1);
                                    let _ = arena.push(c2);
                                    break;
                                }
                            }
                        }
                    }
                    ConstraintKind::RequiredMask { mask } => {
                        if (cand.support.0 & mask.0) != mask.0 {
                            violated = true;
                            arena.candidates[current_idx] = arena.candidates[arena.len - 1];
                            arena.len -= 1;
                            pruned_count += 1;
                            break;
                        }
                    }
                    ConstraintKind::ForbiddenMask { mask } => {
                        if (cand.support.0 & mask.0) != 0 {
                            violated = true;
                            arena.candidates[current_idx] = arena.candidates[arena.len - 1];
                            arena.len -= 1;
                            pruned_count += 1;
                            break;
                        }
                    }
                }
            }

            if !violated {
                current_idx += 1;
            }
        }

        if pruned_count > 0 {
            dendral = dendral.union(DendralByte::CANDIDATE_PRUNED);
        }

        let witness_index = ReconstructionWitnessId(0); // Mapped properly in POWL64 layer

        if arena.len == 0 {
            return ReconstructionResult {
                status: ReconstructionStatus::Failed,
                selected: None,
                emits: InstinctByte::INSPECT.union(InstinctByte::REFUSE),
                dendral: dendral.union(DendralByte::CONSTRAINT_VIOLATION),
                support: FieldMask::empty(),
                witness_index,
            };
        }

        if arena.len == 1 {
            return ReconstructionResult {
                status: ReconstructionStatus::Success,
                selected: Some(arena.candidates[0].id),
                emits: InstinctByte::SETTLE,
                dendral: dendral.union(DendralByte::UNIQUE_RECONSTRUCTION),
                support: arena.candidates[0].support,
                witness_index,
            };
        }

        // Rank remaining
        let mut best_score = -1;
        let mut best_idx = 0;
        let mut ties = 0;

        for i in 0..arena.len {
            if arena.candidates[i].score > best_score {
                best_score = arena.candidates[i].score;
                best_idx = i;
                ties = 1;
            } else if arena.candidates[i].score == best_score {
                ties += 1;
            }
        }

        if ties > 1 {
            ReconstructionResult {
                status: ReconstructionStatus::Partial,
                selected: None,
                emits: InstinctByte::INSPECT.union(InstinctByte::ESCALATE),
                dendral: dendral
                    .union(DendralByte::MULTIPLE_RECONSTRUCTIONS)
                    .union(DendralByte::RECONSTRUCTION_UNSTABLE),
                support: FieldMask::empty(),
                witness_index,
            }
        } else {
            ReconstructionResult {
                status: ReconstructionStatus::Success,
                selected: Some(arena.candidates[best_idx].id),
                emits: InstinctByte::SETTLE,
                dendral: dendral.union(DendralByte::UNIQUE_RECONSTRUCTION),
                support: arena.candidates[best_idx].support,
                witness_index,
            }
        }
    }
}

impl CollapseEngine for ReconstructDendral {
    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let res = self.reconstruct(ctx);
        let mut detail = KappaDetail16::empty();
        detail.kappa = KappaByte::RECONSTRUCT;
        detail.dendral = res.dendral;

        let status = match res.status {
            ReconstructionStatus::Success => CollapseStatus::Success,
            ReconstructionStatus::Partial => CollapseStatus::Partial,
            ReconstructionStatus::Failed => CollapseStatus::Failed,
        };

        CollapseResult {
            detail,
            instincts: res.emits,
            support: Cog8Support::new(res.support),
            status,
        }
    }
}
"""

for path, data in files.items():
    with open(path, "w") as f:
        f.write(data)
