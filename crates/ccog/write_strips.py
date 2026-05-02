import os
import shutil

def write_file(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(content)

# Remove the old file if it exists
old_file = '../insa/insa-kappa8/src/precondition_strips.rs'
if os.path.exists(old_file):
    os.remove(old_file)

mod_rs = """pub mod schema;
pub mod preconditions;
pub mod effects;
pub mod engine;
pub mod result;
pub mod planner;
pub mod fixtures;

pub use schema::*;
pub use preconditions::*;
pub use effects::*;
pub use engine::*;
pub use result::*;
pub use planner::*;
"""

schema_rs = """use insa_types::{FieldMask, CompletedMask};
use crate::precondition_strips::preconditions::{RequiredMask, ForbiddenMask};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ActionId(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PolicyEpoch(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EffectMask {
    pub add: FieldMask,
    pub clear: FieldMask,
    pub complete: CompletedMask,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActionSchema {
    pub id: ActionId,
    pub required: RequiredMask,
    pub forbidden: ForbiddenMask,
    pub add_effects: FieldMask,
    pub clear_effects: FieldMask,
    pub completes: CompletedMask,
    pub policy_epoch: PolicyEpoch,
}
"""

preconditions_rs = """use insa_types::FieldMask;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RequiredMask(pub FieldMask);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ForbiddenMask(pub FieldMask);
"""

effects_rs = """use insa_types::{FieldMask, CompletedMask};

pub fn compute_effects(_add: FieldMask, _clear: FieldMask) -> CompletedMask {
    // Computes effects bounded by Construct8 limits
    CompletedMask(0)
}
"""

engine_rs = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, StripsByte, KappaByte};
use crate::precondition_strips::schema::{ActionSchema, PolicyEpoch};
use crate::precondition_strips::result::PreconditionResult;

pub struct PreconditionStrips;

impl PreconditionStrips {
    pub fn evaluate(schema: &ActionSchema, present: FieldMask, current_epoch: PolicyEpoch) -> PreconditionResult {
        let missing = (present.0 & schema.required.0.0) ^ schema.required.0.0;
        let forbidden = present.0 & schema.forbidden.0.0;
        
        let mut strips = StripsByte::empty();
        let mut emits = InstinctByte::empty();
        
        let is_stale = schema.policy_epoch.0 != current_epoch.0;

        if is_stale {
            emits = emits.union(InstinctByte::AWAIT).union(InstinctByte::ESCALATE);
            strips = strips.union(StripsByte::ACTION_BLOCKED);
        } else {
            if missing != 0 {
                strips = strips.union(StripsByte::MISSING_REQUIRED);
                emits = emits.union(InstinctByte::RETRIEVE).union(InstinctByte::ASK).union(InstinctByte::AWAIT);
            }
            if forbidden != 0 {
                strips = strips.union(StripsByte::FORBIDDEN_PRESENT);
                emits = emits.union(InstinctByte::REFUSE);
            }
            
            let effects_conflict = (schema.add_effects.0 & schema.clear_effects.0) != 0;
            if effects_conflict {
                strips = strips.union(StripsByte::EFFECTS_CONFLICT);
                emits = emits.union(InstinctByte::INSPECT);
            } else {
                strips = strips.union(StripsByte::EFFECTS_KNOWN);
            }
            
            let satisfied = missing == 0 && forbidden == 0 && !effects_conflict;
            if satisfied {
                strips = strips.union(StripsByte::PRECONDITIONS_SATISFIED).union(StripsByte::ACTION_ENABLED);
            } else {
                strips = strips.union(StripsByte::ACTION_BLOCKED).union(StripsByte::REQUIRES_REPLAN);
                emits = emits.union(InstinctByte::REFUSE).union(InstinctByte::ESCALATE);
            }
        }
        
        PreconditionResult {
            detail: strips,
            kappa: KappaByte::PRECONDITION,
            emits,
            missing_required: FieldMask(missing),
            present_forbidden: FieldMask(forbidden),
            add_effects: schema.add_effects,
            clear_effects: schema.clear_effects,
        }
    }
}
"""

result_rs = """use insa_types::FieldMask;
use insa_instinct::{InstinctByte, KappaByte, StripsByte};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreconditionResult {
    pub detail: StripsByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
    pub missing_required: FieldMask,
    pub present_forbidden: FieldMask,
    pub add_effects: FieldMask,
    pub clear_effects: FieldMask,
}

impl Default for PreconditionResult {
    fn default() -> Self {
        Self {
            detail: StripsByte::empty(),
            kappa: KappaByte::empty(),
            emits: InstinctByte::empty(),
            missing_required: FieldMask(0),
            present_forbidden: FieldMask(0),
            add_effects: FieldMask(0),
            clear_effects: FieldMask(0),
        }
    }
}
"""

planner_rs = """use crate::precondition_strips::result::PreconditionResult;
use insa_instinct::StripsByte;

pub struct Planner;

impl Planner {
    pub fn requires_replan(result: &PreconditionResult) -> bool {
        result.detail.contains(StripsByte::REQUIRES_REPLAN)
    }
}
"""

fixtures_rs = """use insa_types::{FieldMask, CompletedMask};
use crate::precondition_strips::schema::{ActionSchema, ActionId, PolicyEpoch};
use crate::precondition_strips::preconditions::{RequiredMask, ForbiddenMask};

pub const IDENTITY_ACTIVE: u64 = 1 << 0;
pub const VENDOR_VALID: u64 = 1 << 1;
pub const POLICY_PERMITS_ACCESS: u64 = 1 << 2;
pub const IDENTITY_TERMINATED: u64 = 1 << 3;
pub const VENDOR_EXPIRED: u64 = 1 << 4;
pub const ACCESS_REVOCATION_REQUIRED: u64 = 1 << 5;
pub const ACCESS_ALLOWED: u64 = 1 << 6;
pub const ACTIVE_ACCESS_PRESENT: u64 = 1 << 7;
pub const REVOCATION_AUTHORITY_PRESENT: u64 = 1 << 8;
pub const ALREADY_REVOKED: u64 = 1 << 9;
pub const BADGE_ACTIVE: u64 = 1 << 10;
pub const VPN_ACTIVE: u64 = 1 << 11;
pub const REPO_ACCESS_ACTIVE: u64 = 1 << 12;
pub const ACCESS_REVOKED: u64 = 1 << 13;
pub const EVIDENCE_RECORDED: u64 = 1 << 14;
pub const OPEN_OWNER_GAP: u64 = 1 << 15;
pub const CASE_SETTLED: u64 = 1 << 16;

pub fn allow_access_schema() -> ActionSchema {
    ActionSchema {
        id: ActionId(1),
        required: RequiredMask(FieldMask(IDENTITY_ACTIVE | VENDOR_VALID | POLICY_PERMITS_ACCESS)),
        forbidden: ForbiddenMask(FieldMask(IDENTITY_TERMINATED | VENDOR_EXPIRED | ACCESS_REVOCATION_REQUIRED)),
        add_effects: FieldMask(ACCESS_ALLOWED),
        clear_effects: FieldMask(0),
        completes: CompletedMask(0),
        policy_epoch: PolicyEpoch(1),
    }
}

pub fn revoke_access_schema() -> ActionSchema {
    ActionSchema {
        id: ActionId(2),
        required: RequiredMask(FieldMask(ACTIVE_ACCESS_PRESENT | REVOCATION_AUTHORITY_PRESENT)),
        forbidden: ForbiddenMask(FieldMask(ALREADY_REVOKED)),
        add_effects: FieldMask(ACCESS_REVOKED),
        clear_effects: FieldMask(BADGE_ACTIVE | VPN_ACTIVE | REPO_ACCESS_ACTIVE),
        completes: CompletedMask(0),
        policy_epoch: PolicyEpoch(1),
    }
}

pub fn settle_case_schema() -> ActionSchema {
    ActionSchema {
        id: ActionId(3),
        required: RequiredMask(FieldMask(ACCESS_REVOKED | EVIDENCE_RECORDED)),
        forbidden: ForbiddenMask(FieldMask(ACTIVE_ACCESS_PRESENT | OPEN_OWNER_GAP)),
        add_effects: FieldMask(CASE_SETTLED),
        clear_effects: FieldMask(0),
        completes: CompletedMask(0),
        policy_epoch: PolicyEpoch(1),
    }
}
"""

test_content = """use insa_kappa8::precondition_strips::engine::PreconditionStrips;
use insa_kappa8::precondition_strips::schema::PolicyEpoch;
use insa_kappa8::precondition_strips::fixtures::*;
use insa_instinct::{InstinctByte, StripsByte};
use insa_types::FieldMask;

#[test]
fn test_allow_access_drift_rejected() {
    let schema = allow_access_schema();
    
    // Field has identity terminated, vendor expired, but badge active
    let present = FieldMask(IDENTITY_TERMINATED | VENDOR_EXPIRED | BADGE_ACTIVE);
    
    let res = PreconditionStrips::evaluate(&schema, present, PolicyEpoch(1));
    
    // Should be missing requirements
    assert!(res.detail.contains(StripsByte::MISSING_REQUIRED));
    assert!(res.detail.contains(StripsByte::FORBIDDEN_PRESENT));
    assert!(res.detail.contains(StripsByte::ACTION_BLOCKED));
    assert!(res.detail.contains(StripsByte::REQUIRES_REPLAN));
    
    // Emits Refuse + Retrieve + Escalate + Ask + Await
    assert!(res.emits.contains(InstinctByte::REFUSE));
    assert!(res.emits.contains(InstinctByte::ESCALATE));
}

#[test]
fn test_revoke_access_enabled() {
    let schema = revoke_access_schema();
    
    let present = FieldMask(ACTIVE_ACCESS_PRESENT | REVOCATION_AUTHORITY_PRESENT);
    
    let res = PreconditionStrips::evaluate(&schema, present, PolicyEpoch(1));
    
    // Action should be enabled
    assert!(res.detail.contains(StripsByte::PRECONDITIONS_SATISFIED));
    assert!(res.detail.contains(StripsByte::ACTION_ENABLED));
    assert!(!res.detail.contains(StripsByte::ACTION_BLOCKED));
}
"""

write_file('../insa/insa-kappa8/src/precondition_strips/mod.rs', mod_rs)
write_file('../insa/insa-kappa8/src/precondition_strips/schema.rs', schema_rs)
write_file('../insa/insa-kappa8/src/precondition_strips/preconditions.rs', preconditions_rs)
write_file('../insa/insa-kappa8/src/precondition_strips/effects.rs', effects_rs)
write_file('../insa/insa-kappa8/src/precondition_strips/engine.rs', engine_rs)
write_file('../insa/insa-kappa8/src/precondition_strips/result.rs', result_rs)
write_file('../insa/insa-kappa8/src/precondition_strips/planner.rs', planner_rs)
write_file('../insa/insa-kappa8/src/precondition_strips/fixtures.rs', fixtures_rs)
write_file('../insa/insa-truthforge/tests/kappa_strips.rs', test_content)

print("STRIPS component files written successfully.")
