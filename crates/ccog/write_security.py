import os

def write_file(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(content)

cargo_toml_content = """[package]
name = "insa-security"
version = "0.1.0"
edition = "2021"
description = "Converged Security Closure rules and domain logic."
publish = false

[dependencies]
insa-types = { path = "../insa-types" }
insa-instinct = { path = "../insa-instinct" }
insa-kappa8 = { path = "../insa-kappa8" }
insa-hotpath = { path = "../insa-hotpath" }
"""

lib_rs_content = """//! Converged Security Closure domain pack.
//!
//! Provides the pre-admitted field masks and execution rules for evaluating
//! cross-domain enterprise security configurations (Identity, Site, Policy, Vendor, Cyber).

use insa_types::{FieldBit, FieldMask, PackId, GroupId, RuleId};
use insa_instinct::InstinctByte;
use insa_kappa8::KappaByte;
use insa_hotpath::cog8::{Cog8Row, CollapseFn};

/// The Pack ID for Converged Security Closure rules.
pub const SECURITY_PACK_ID: PackId = PackId(0x1000);

/// Security-specific Field Bits mapping overlapping risk domains.
#[allow(non_snake_case)]
pub mod SecurityFieldBit {
    use super::FieldBit;

    // Identity / HR Field
    pub const HR_TERMINATED: FieldBit = FieldBit::new_checked(0).unwrap();
    pub const HR_ACTIVE: FieldBit = FieldBit::new_checked(1).unwrap();
    
    // Badge / Site Field
    pub const BADGE_ACTIVE: FieldBit = FieldBit::new_checked(2).unwrap();
    pub const BADGE_USED_AFTER_HOURS: FieldBit = FieldBit::new_checked(3).unwrap();
    
    // Cyber / Network Field
    pub const VPN_ACTIVE: FieldBit = FieldBit::new_checked(4).unwrap();
    pub const REPO_ACCESS_ACTIVE: FieldBit = FieldBit::new_checked(5).unwrap();
    pub const CRITICAL_ASSET_EXPOSED: FieldBit = FieldBit::new_checked(6).unwrap();
    
    // Vendor Field
    pub const VENDOR_CONTRACT_EXPIRED: FieldBit = FieldBit::new_checked(7).unwrap();
    pub const VENDOR_API_KEY_VALID: FieldBit = FieldBit::new_checked(8).unwrap();
    
    // Vulnerability / CVE Field
    pub const CVE_CRITICAL_PRESENT: FieldBit = FieldBit::new_checked(9).unwrap();
    pub const CVE_REACHABLE: FieldBit = FieldBit::new_checked(10).unwrap();
    pub const MITIGATION_ABSENT: FieldBit = FieldBit::new_checked(11).unwrap();
    
    // Policy / Exception Field
    pub const EXCEPTION_EXPIRED: FieldBit = FieldBit::new_checked(12).unwrap();
    pub const POLICY_OWNER_MISSING: FieldBit = FieldBit::new_checked(13).unwrap();
}

/// The pre-admitted set of Converged Security rules.
pub static SECURITY_RULES: &[Cog8Row] = &[
    // Terminated Contractor Drift
    // HR terminated AND (badge active OR VPN active) AND Vendor Contract Expired
    Cog8Row {
        pack_id: SECURITY_PACK_ID,
        group_id: GroupId(1),
        rule_id: RuleId(1),
        breed_id: insa_types::BreedId(0),
        collapse_fn: CollapseFn::ExpertRule,
        required_mask: FieldMask(
            (1 << 0) | // HR_TERMINATED
            (1 << 7)   // VENDOR_CONTRACT_EXPIRED
            // Note: OR condition for badge/VPN needs separate rows or decomposed logic, 
            // demonstrating Need9 constraint. We will represent Badge active here.
            | (1 << 2) // BADGE_ACTIVE
        ),
        response: InstinctByte::REFUSE,
        kappa: KappaByte::RULE,
        priority: 200, // High priority security refuse
        ..Cog8Row::default()
    },
    // Insider-threat field mismatch
    // After hours badge AND large data export (simulated via VPN)
    Cog8Row {
        pack_id: SECURITY_PACK_ID,
        group_id: GroupId(2),
        rule_id: RuleId(1),
        breed_id: insa_types::BreedId(0),
        collapse_fn: CollapseFn::ExpertRule,
        required_mask: FieldMask(
            (1 << 3) | // BADGE_USED_AFTER_HOURS
            (1 << 4)   // VPN_ACTIVE
        ),
        response: InstinctByte::INSPECT,
        kappa: KappaByte::RULE,
        priority: 150,
        ..Cog8Row::default()
    },
    // Unowned Critical Vulnerability
    // CVE Critical + Reachable + Unmitigated + Owner Missing
    Cog8Row {
        pack_id: SECURITY_PACK_ID,
        group_id: GroupId(3),
        rule_id: RuleId(1),
        breed_id: insa_types::BreedId(0),
        collapse_fn: CollapseFn::ExpertRule,
        required_mask: FieldMask(
            (1 << 9) |  // CVE_CRITICAL_PRESENT
            (1 << 10) | // CVE_REACHABLE
            (1 << 11) | // MITIGATION_ABSENT
            (1 << 13)   // POLICY_OWNER_MISSING
        ),
        response: InstinctByte::ESCALATE,
        kappa: KappaByte::RULE,
        priority: 250, // Escalate immediately
        ..Cog8Row::default()
    }
];

// Add an empty impl block so `Cog8Row::default()` works correctly in the context of the array initialization above if it's missing traits.
// Oh wait, Cog8Row derives Default in insa_hotpath. We just need to make sure the fields are public.
"""

write_file('../insa/insa-security/Cargo.toml', cargo_toml_content)
write_file('../insa/insa-security/src/lib.rs', lib_rs_content)
print("Security files written successfully.")
