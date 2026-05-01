use crate::precondition_strips::preconditions::{ForbiddenMask, RequiredMask};
use crate::precondition_strips::schema::{ActionId, ActionSchema, PolicyEpoch};
use insa_types::{CompletedMask, FieldMask};

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
        required: RequiredMask(FieldMask(
            IDENTITY_ACTIVE | VENDOR_VALID | POLICY_PERMITS_ACCESS,
        )),
        forbidden: ForbiddenMask(FieldMask(
            IDENTITY_TERMINATED | VENDOR_EXPIRED | ACCESS_REVOCATION_REQUIRED,
        )),
        add_effects: FieldMask(ACCESS_ALLOWED),
        clear_effects: FieldMask(0),
        completes: CompletedMask(0),
        policy_epoch: PolicyEpoch(1),
    }
}

pub fn revoke_access_schema() -> ActionSchema {
    ActionSchema {
        id: ActionId(2),
        required: RequiredMask(FieldMask(
            ACTIVE_ACCESS_PRESENT | REVOCATION_AUTHORITY_PRESENT,
        )),
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
