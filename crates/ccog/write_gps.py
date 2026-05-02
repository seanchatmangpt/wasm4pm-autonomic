import os

def write_file(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(content)

# Patch GpsByte to ensure bits() method
byte_path = '../insa/insa-instinct/src/byte.rs'
with open(byte_path, 'r') as f:
    byte_content = f.read()

bits_method = """
    #[inline(always)]
    pub const fn bits(self) -> u8 {
        self.0
    }
"""

if "pub const fn bits(self) -> u8" not in byte_content.split("impl GpsByte {")[1].split("impl KappaDetail16 {")[0]:
    byte_content = byte_content.replace(
"""impl GpsByte {
    pub const GOAL_KNOWN: Self = Self(1 << 0);""",
"""impl GpsByte {
    pub const GOAL_KNOWN: Self = Self(1 << 0);""" + bits_method)
    with open(byte_path, 'w') as f:
        f.write(byte_content)


mod_rs = """pub mod goal;
pub mod gap;
pub mod operator;
pub mod select;
pub mod engine;
pub mod result;
pub mod witness;
pub mod fixtures;

pub use goal::*;
pub use gap::*;
pub use operator::*;
pub use select::*;
pub use engine::*;
pub use result::*;
pub use witness::*;
pub use fixtures::*;
"""

goal_rs = """use insa_types::{FieldMask, CompletedMask};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct GoalId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PolicyEpoch(pub u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RequiredMask(pub FieldMask);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ForbiddenMask(pub FieldMask);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GoalState {
    pub id: GoalId,
    pub required: RequiredMask,
    pub forbidden: ForbiddenMask,
    pub completed: CompletedMask,
    pub policy_epoch: PolicyEpoch,
}
"""

gap_rs = """use insa_types::{FieldMask, CompletedMask};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Gap {
    pub missing_required: FieldMask,
    pub present_forbidden: FieldMask,
    pub incomplete: CompletedMask,
    pub width: u8,
    pub reserved: [u8; 7],
}

impl Gap {
    pub fn compute_width(&self) -> u8 {
        (self.missing_required.0.count_ones() + 
         self.present_forbidden.0.count_ones() + 
         self.incomplete.0.count_ones()) as u8
    }
}
"""

operator_rs = """use insa_types::{FieldMask, CompletedMask};
use insa_instinct::InstinctByte;
use insa_hotpath::powl8::Powl8Op;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct OperatorId(pub u16);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GapOperator {
    pub id: OperatorId,
    pub resolves_required: FieldMask,
    pub clears_forbidden: FieldMask,
    pub completes: CompletedMask,
    pub emits: InstinctByte,
    pub motion: Powl8Op,
}
"""

select_rs = """use crate::reduce_gap_gps::gap::Gap;
use crate::reduce_gap_gps::operator::GapOperator;

pub struct OperatorSelector;

impl OperatorSelector {
    /// Selects the operator that reduces the gap the most.
    pub fn select_smallest_lawful_operator<'a>(gap: &Gap, operators: &'a [GapOperator]) -> Option<&'a GapOperator> {
        let mut best_operator = None;
        let mut best_resolved_count = 0;

        for op in operators {
            let resolved_req = op.resolves_required.0 & gap.missing_required.0;
            let cleared_forb = op.clears_forbidden.0 & gap.present_forbidden.0;
            let completed_inc = op.completes.0 & gap.incomplete.0;
            
            let resolved_count = resolved_req.count_ones() + cleared_forb.count_ones() + completed_inc.count_ones();

            if resolved_count > best_resolved_count {
                best_resolved_count = resolved_count;
                best_operator = Some(op);
            }
        }

        best_operator
    }
}
"""

engine_rs = """use insa_types::{FieldMask, CompletedMask};
use insa_instinct::{InstinctByte, KappaByte, GpsByte};
use crate::reduce_gap_gps::goal::{GoalState, PolicyEpoch};
use crate::reduce_gap_gps::gap::Gap;
use crate::reduce_gap_gps::operator::{GapOperator, OperatorId};
use crate::reduce_gap_gps::select::OperatorSelector;
use crate::reduce_gap_gps::result::{GapReductionResult, GapStatus};
use crate::reduce_gap_gps::witness::GapWitnessId;

pub struct ReduceGapGps;

impl ReduceGapGps {
    pub fn compute_gap(current: FieldMask, current_completed: CompletedMask, goal: &GoalState) -> Gap {
        let missing = (current.0 & goal.required.0.0) ^ goal.required.0.0;
        let forbidden = current.0 & goal.forbidden.0.0;
        let incomplete = (current_completed.0 & goal.completed.0) ^ goal.completed.0;

        let mut gap = Gap {
            missing_required: FieldMask(missing),
            present_forbidden: FieldMask(forbidden),
            incomplete: CompletedMask(incomplete),
            width: 0,
            reserved: [0; 7],
        };
        gap.width = gap.compute_width();
        gap
    }

    pub fn reduce(current: FieldMask, current_completed: CompletedMask, goal: &GoalState, operators: &[GapOperator], current_epoch: PolicyEpoch) -> GapReductionResult {
        let mut detail = GpsByte::empty().union(GpsByte::GOAL_KNOWN);
        let mut emits = InstinctByte::empty();

        let is_stale = goal.policy_epoch.0 != current_epoch.0;
        if is_stale {
            // Cannot trust goal, emit Await/Retrieve
            emits = emits.union(InstinctByte::AWAIT).union(InstinctByte::RETRIEVE);
            detail = detail.union(GpsByte::NO_PROGRESS);
            return GapReductionResult {
                status: GapStatus::StaleGoal,
                detail,
                kappa: KappaByte::REDUCE_GAP,
                emits,
                gap: Gap::default(),
                selected_operator: OperatorId(0),
                witness_index: GapWitnessId(0),
            };
        }

        let gap = Self::compute_gap(current, current_completed, goal);

        if gap.width == 0 {
            detail = detail.union(GpsByte::PROGRESS_MADE);
            emits = emits.union(InstinctByte::SETTLE);
            return GapReductionResult {
                status: GapStatus::GapClosed,
                detail,
                kappa: KappaByte::REDUCE_GAP,
                emits,
                gap,
                selected_operator: OperatorId(0),
                witness_index: GapWitnessId(0),
            };
        }

        detail = detail.union(GpsByte::GAP_DETECTED);
        
        if gap.width <= 8 {
            detail = detail.union(GpsByte::GAP_SMALL);
        } else {
            detail = detail.union(GpsByte::GAP_LARGE);
            emits = emits.union(InstinctByte::INSPECT);
            return GapReductionResult {
                status: GapStatus::GapTooLarge,
                detail,
                kappa: KappaByte::REDUCE_GAP,
                emits,
                gap,
                selected_operator: OperatorId(0),
                witness_index: GapWitnessId(0),
            };
        }

        let selected = OperatorSelector::select_smallest_lawful_operator(&gap, operators);
        
        if let Some(op) = selected {
            detail = detail.union(GpsByte::OPERATOR_AVAILABLE).union(GpsByte::PROGRESS_MADE);
            emits = emits.union(op.emits);
            GapReductionResult {
                status: GapStatus::OperatorSelected,
                detail,
                kappa: KappaByte::REDUCE_GAP,
                emits,
                gap,
                selected_operator: op.id,
                witness_index: GapWitnessId(0),
            }
        } else {
            detail = detail.union(GpsByte::OPERATOR_BLOCKED).union(GpsByte::NO_PROGRESS);
            emits = emits.union(InstinctByte::REFUSE).union(InstinctByte::ESCALATE);
            GapReductionResult {
                status: GapStatus::NoOperatorAvailable,
                detail,
                kappa: KappaByte::REDUCE_GAP,
                emits,
                gap,
                selected_operator: OperatorId(0),
                witness_index: GapWitnessId(0),
            }
        }
    }
}
"""

result_rs = """use crate::reduce_gap_gps::gap::Gap;
use crate::reduce_gap_gps::operator::OperatorId;
use crate::reduce_gap_gps::witness::GapWitnessId;
use insa_instinct::{GpsByte, InstinctByte, KappaByte};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GapStatus {
    GapClosed = 0,
    OperatorSelected = 1,
    GapTooLarge = 2,
    NoOperatorAvailable = 3,
    StaleGoal = 4,
}

impl Default for GapStatus {
    fn default() -> Self {
        Self::StaleGoal
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GapReductionResult {
    pub status: GapStatus,
    pub detail: GpsByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
    pub gap: Gap,
    pub selected_operator: OperatorId,
    pub witness_index: GapWitnessId,
}
"""

witness_rs = """#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct GapWitnessId(pub u64);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GapWitness {
    pub id: GapWitnessId,
}
"""

fixtures_rs = """use insa_types::{FieldMask, CompletedMask};
use insa_instinct::InstinctByte;
use insa_hotpath::powl8::Powl8Op;
use crate::reduce_gap_gps::goal::{GoalState, GoalId, PolicyEpoch, RequiredMask, ForbiddenMask};
use crate::reduce_gap_gps::operator::{GapOperator, OperatorId};

pub const BADGE_ACTIVE: u64 = 1 << 10;
pub const VPN_ACTIVE: u64 = 1 << 11;
pub const REPO_ACCESS_ACTIVE: u64 = 1 << 12;

pub fn access_drift_goal() -> GoalState {
    GoalState {
        id: GoalId(1),
        required: RequiredMask(FieldMask(0)), // Not requiring any new bits
        forbidden: ForbiddenMask(FieldMask(BADGE_ACTIVE | VPN_ACTIVE | REPO_ACCESS_ACTIVE)), // Forbidding these access bits
        completed: CompletedMask(0),
        policy_epoch: PolicyEpoch(1),
    }
}

pub fn revoke_badge_operator() -> GapOperator {
    GapOperator {
        id: OperatorId(1),
        resolves_required: FieldMask(0),
        clears_forbidden: FieldMask(BADGE_ACTIVE),
        completes: CompletedMask(0),
        emits: InstinctByte::REFUSE.union(InstinctByte::ESCALATE),
        motion: Powl8Op::Block,
    }
}

pub fn disable_vpn_operator() -> GapOperator {
    GapOperator {
        id: OperatorId(2),
        resolves_required: FieldMask(0),
        clears_forbidden: FieldMask(VPN_ACTIVE),
        completes: CompletedMask(0),
        emits: InstinctByte::REFUSE.union(InstinctByte::RETRIEVE),
        motion: Powl8Op::Act,
    }
}
"""

test_content = """use insa_types::{FieldMask, CompletedMask};
use insa_instinct::{InstinctByte, GpsByte};
use insa_kappa8::reduce_gap_gps::*;

#[test]
fn test_reduce_gap_access_drift() {
    let goal = access_drift_goal();
    
    // Current state has badge and vpn active (both forbidden by goal)
    let present = FieldMask(BADGE_ACTIVE | VPN_ACTIVE);
    let completed = CompletedMask(0);
    let epoch = PolicyEpoch(1);

    let operators = [revoke_badge_operator(), disable_vpn_operator()];

    let res = ReduceGapGps::reduce(present, completed, &goal, &operators, epoch);
    
    // Gap should be detected
    assert_eq!(res.status, GapStatus::OperatorSelected);
    assert!(res.detail.contains(GpsByte::GAP_DETECTED));
    assert!(res.detail.contains(GpsByte::GAP_SMALL));
    assert!(res.detail.contains(GpsByte::OPERATOR_AVAILABLE));
    assert!(res.detail.contains(GpsByte::PROGRESS_MADE));
    
    // It should have selected one of the operators (RevokeBadge or DisableVPN)
    // Since both clear 1 forbidden bit, the selector grabs the first it evaluates which is RevokeBadge (Op 1)
    assert_eq!(res.selected_operator.0, 1);
    assert!(res.emits.contains(InstinctByte::REFUSE));
    assert!(res.emits.contains(InstinctByte::ESCALATE));
}

#[test]
fn test_reduce_gap_no_operator() {
    let goal = access_drift_goal();
    let present = FieldMask(REPO_ACCESS_ACTIVE); // Gap is repo access
    let completed = CompletedMask(0);
    let epoch = PolicyEpoch(1);

    // Provide only badge and VPN operators, NO repo operator
    let operators = [revoke_badge_operator(), disable_vpn_operator()];

    let res = ReduceGapGps::reduce(present, completed, &goal, &operators, epoch);
    
    // Gap detected, but no operator resolves it
    assert_eq!(res.status, GapStatus::NoOperatorAvailable);
    assert!(res.detail.contains(GpsByte::OPERATOR_BLOCKED));
    assert!(res.detail.contains(GpsByte::NO_PROGRESS));
    
    // Should escalate since it is stuck
    assert!(res.emits.contains(InstinctByte::REFUSE));
    assert!(res.emits.contains(InstinctByte::ESCALATE));
}
"""

if os.path.exists('../insa/insa-kappa8/src/reduce_gap_gps.rs'):
    os.remove('../insa/insa-kappa8/src/reduce_gap_gps.rs')

write_file('../insa/insa-kappa8/src/reduce_gap_gps/mod.rs', mod_rs)
write_file('../insa/insa-kappa8/src/reduce_gap_gps/goal.rs', goal_rs)
write_file('../insa/insa-kappa8/src/reduce_gap_gps/gap.rs', gap_rs)
write_file('../insa/insa-kappa8/src/reduce_gap_gps/operator.rs', operator_rs)
write_file('../insa/insa-kappa8/src/reduce_gap_gps/select.rs', select_rs)
write_file('../insa/insa-kappa8/src/reduce_gap_gps/engine.rs', engine_rs)
write_file('../insa/insa-kappa8/src/reduce_gap_gps/result.rs', result_rs)
write_file('../insa/insa-kappa8/src/reduce_gap_gps/witness.rs', witness_rs)
write_file('../insa/insa-kappa8/src/reduce_gap_gps/fixtures.rs', fixtures_rs)

write_file('../insa/insa-truthforge/tests/kappa_gps.rs', test_content)

print("ReduceGap / GPS pack successfully generated.")
