#![doc = include_str!("../README.md")]
//! Security Domain Closure and Access Drift JTBD.

use insa_hotpath::cog8::Cog8Row;
use insa_instinct::{InstinctByte, KappaByte};
use insa_types::{FieldBit, FieldMask, GroupId, PackId, RuleId};

// FieldBits
pub const IDENTITY_TERMINATED: FieldBit = FieldBit::new_unchecked(0);
pub const VENDOR_CONTRACT_EXPIRED: FieldBit = FieldBit::new_unchecked(1);
pub const BADGE_ACTIVE: FieldBit = FieldBit::new_unchecked(2);
pub const VPN_ACTIVE: FieldBit = FieldBit::new_unchecked(3);
pub const REPO_ACCESS_ACTIVE: FieldBit = FieldBit::new_unchecked(4);
pub const RECENT_SITE_ENTRY: FieldBit = FieldBit::new_unchecked(5);
pub const DEVICE_SEEN_ON_SITE_NETWORK: FieldBit = FieldBit::new_unchecked(6);
pub const POLICY_REQUIRES_ACCESS_REMOVAL: FieldBit = FieldBit::new_unchecked(7);

pub fn build_access_drift_rows() -> Vec<Cog8Row> {
    vec![
        // Row 1: TerminatedButDigitallyActive
        Cog8Row {
            required_mask: FieldMask::empty()
                .with_bit(IDENTITY_TERMINATED)
                .with_bit(VPN_ACTIVE)
                .with_bit(REPO_ACCESS_ACTIVE),
            response: InstinctByte::REFUSE.union(InstinctByte::ESCALATE),
            kappa: KappaByte::RULE,
            pack_id: PackId(1),
            group_id: GroupId(1),
            rule_id: RuleId(1),
            ..Default::default()
        },
        // Row 2: TerminatedButPhysicallyActive
        Cog8Row {
            required_mask: FieldMask::empty()
                .with_bit(IDENTITY_TERMINATED)
                .with_bit(BADGE_ACTIVE)
                .with_bit(RECENT_SITE_ENTRY),
            response: InstinctByte::REFUSE
                .union(InstinctByte::INSPECT)
                .union(InstinctByte::ESCALATE),
            kappa: KappaByte::RULE,
            pack_id: PackId(1),
            group_id: GroupId(1),
            rule_id: RuleId(2),
            ..Default::default()
        },
        // Row 3: VendorExpiredButAccessActive
        Cog8Row {
            required_mask: FieldMask::empty()
                .with_bit(VENDOR_CONTRACT_EXPIRED)
                .with_bit(BADGE_ACTIVE),
            response: InstinctByte::REFUSE.union(InstinctByte::RETRIEVE),
            kappa: KappaByte::RULE,
            pack_id: PackId(1),
            group_id: GroupId(2),
            rule_id: RuleId(1),
            ..Default::default()
        },
        // Row 4: PolicyViolationClosure
        Cog8Row {
            required_mask: FieldMask::empty()
                .with_bit(POLICY_REQUIRES_ACCESS_REMOVAL)
                .with_bit(VPN_ACTIVE), // just an example active access
            response: InstinctByte::REFUSE,
            kappa: KappaByte::RULE,
            pack_id: PackId(1),
            group_id: GroupId(3),
            rule_id: RuleId(1),
            ..Default::default()
        },
    ]
}
