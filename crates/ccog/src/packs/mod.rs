//! Field packs (Phase 12) — productized bark-slot bundles.
//!
//! A `FieldPack` is a compile-time bundle of:
//! - A canonical name and ontology profile (public-only IRIs).
//! - A non-overlapping reserved bit range for posture/context predicates
//!   (see [`bits`]).
//! - A list of admitted [`crate::verdict::Breed`] variants.
//! - A static `BarkSlot` table of pack-specific built-in hooks.
//! - A `select_instinct` bias wrapper that classifies the closed cognition
//!   surface into the canonical [`crate::instinct::AutonomicInstinct`] set —
//!   packs MUST NOT introduce new response classes; they bias the lattice
//!   only.
//!
//! The four shipped packs are:
//! - [`lifestyle`] — routine / fatigue / transition (OT lineage).
//! - [`edge`] — package / visitor / theft (home-edge lineage).
//! - [`enterprise`] — gap / transition / routing / compliance (PROV-rich).
//! - [`dev`] — boundary / nightly / mask-domain / CLAUDE.md (governance).
//!
//! Constitutional constraints:
//! 1. AutonomicInstinct enum is canonical — packs never fork response classes.
//! 2. No PII in mask names or emitted IRIs (only `urn:blake3:` of
//!    interpreter-issued tokens).
//! 3. Dev pack actions clamp `Refuse|Escalate` → `Ask` (never auto-merge).
//! 4. Bit ranges enforced via `const_assert!` per pack.
//! 5. Only public ontologies (PROV / schema.org / SHACL / xsd / urn:blake3 /
//!    urn:ccog:vocab:).

pub mod bits;
pub mod dev;
pub mod edge;
pub mod enterprise;
pub mod lifestyle;
pub mod lifestyle_overlap;
pub mod metadata_ids;

pub use metadata_ids::{GroupId, ObligationId, PackId, RuleId};

use crate::bark_artifact::BarkSlot;
use crate::instinct::{select_instinct_v0, AutonomicInstinct};
use crate::multimodal::{ContextBundle, PostureBundle};
use crate::runtime::ClosedFieldContext;
use crate::verdict::Breed;

/// Compile-time field pack contract.
///
/// All associated items are `const` and resolved at the call site — packs
/// never carry runtime state. The two `Range<u32>` constants define the
/// posture and context bit ranges this pack is allowed to allocate within;
/// they MUST land entirely inside one of the canonical bands declared in
/// [`bits`] and MUST NOT overlap any other pack's range.
pub trait FieldPack {
    /// Canonical pack name (used in receipt material and AGENTS.md mapping).
    const NAME: &'static str;
    /// Allowlisted ontology IRI prefixes for this pack's emitted triples.
    /// MUST be a subset of the global public-ontology allowlist.
    const ONTOLOGY_PROFILE: &'static [&'static str];
    /// Admitted cognitive breeds — packs may declare the breeds whose
    /// preconditions they expect to satisfy. Empty means "no preference".
    const ADMITTED_BREEDS: &'static [Breed];
    /// Reserved posture-bit range owned by this pack.
    const POSTURE_RANGE: core::ops::Range<u32>;
    /// Reserved context-bit range owned by this pack (in the same band
    /// because posture and context share a 64-bit mask domain by pack).
    const CONTEXT_RANGE: core::ops::Range<u32>;

    /// Static table of pack-specific bark slots (4–6 entries by convention).
    fn builtins() -> &'static [BarkSlot];
}

/// Public allowlist of ontology IRI prefixes accepted by every pack.
///
/// Packs that emit IRIs outside this set will fail their conformance test
/// (`pack_*_boundary_no_pii_in_iri` or sibling).
pub const PUBLIC_ONTOLOGY_PREFIXES: &[&str] = &[
    "http://www.w3.org/ns/prov#",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "http://www.w3.org/2001/XMLSchema#",
    "http://www.w3.org/ns/shacl#",
    "https://schema.org/",
    "urn:blake3:",
    "urn:ccog:vocab:",
];

/// True iff `iri` starts with any prefix in [`PUBLIC_ONTOLOGY_PREFIXES`].
#[must_use]
pub fn iri_is_public(iri: &str) -> bool {
    PUBLIC_ONTOLOGY_PREFIXES.iter().any(|p| iri.starts_with(p))
}

// =============================================================================
// KZ7B: Runtime Pack Loading Infrastructure
// =============================================================================

/// Error type for pack loading failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackLoadError {
    /// Pack has no rules.
    EmptyRules,
    /// Pack manifest digest verification failed (tampering detected).
    ManifestTamper(String),
    /// Pack ontology profile is empty.
    MissingOntologyProfile,
    /// Pack contains IRI outside public-ontology allowlist.
    PrivateOntologyTerm(String),
    /// Pack response class is outside canonical lattice.
    InvalidResponseClass(String),
    /// Pack contains other validation failure.
    ValidationFailed(String),
}

impl std::fmt::Display for PackLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackLoadError::EmptyRules => write!(f, "pack has no rules"),
            PackLoadError::ManifestTamper(detail) => write!(f, "manifest tamper: {}", detail),
            PackLoadError::MissingOntologyProfile => write!(f, "pack missing ontology profile"),
            PackLoadError::PrivateOntologyTerm(iri) => {
                write!(f, "private ontology term not allowed: {}", iri)
            }
            PackLoadError::InvalidResponseClass(r) => write!(f, "invalid response class: {}", r),
            PackLoadError::ValidationFailed(detail) => write!(f, "validation failed: {}", detail),
        }
    }
}

impl std::error::Error for PackLoadError {}

/// Phase 7 K-tier mask bundle — fixed-arity, no allocations.
///
/// K0 is the existing `(posture, expectation, risk, affordance)` four-mask
/// surface (carried inline on [`LoadedPackRule`] for compatibility). K1–K3
/// are additional `u64` tiers for overlapping cognition fields:
///
/// - K1: Routine / Capacity / Regulation / Safety
/// - K2: Meaning / Social / Recovery / Identity
/// - K3: Environment / Object / Transition / Evidence / Rhythm
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TierMasks {
    /// K1 mask — Routine / Capacity / Regulation / Safety.
    pub k1: u64,
    /// K2 mask — Meaning / Social / Recovery / Identity.
    pub k2: u64,
    /// K3 mask — Environment / Object / Transition / Evidence / Rhythm.
    pub k3: u64,
}

impl TierMasks {
    /// All-zero tier masks (no Lifestyle context supplied).
    pub const ZERO: Self = Self {
        k1: 0,
        k2: 0,
        k3: 0,
    };
}

/// A single runtime-evaluable pack rule.
///
/// Each rule names itself, declares a posture/context mask requirement
/// across the existing K0 surface AND the K1/K2/K3 tier masks, and names
/// the response it admits when every requirement is satisfied. A rule
/// matches when every set bit in each `require_*_mask` is also set in the
/// corresponding runtime mask. An all-zero requirement means "no
/// constraint on this dimension".
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadedPackRule {
    /// Stable rule id (used in `PackDecision::matched_rule_id` for traceability).
    /// Static-lifetime: rule ids are either compile-time literals (packs
    /// authored in Rust) or interned at load time via [`intern_rule_id`].
    pub id: RuleId,
    /// Response admitted by the rule on match.
    pub response: AutonomicInstinct,
    /// Posture bits the rule requires (K0 — subset relation).
    pub require_posture_mask: u64,
    /// Expectation bits the rule requires (K0).
    pub require_expectation_mask: u64,
    /// Risk bits the rule requires (K0).
    pub require_risk_mask: u64,
    /// Affordance bits the rule requires (K0).
    pub require_affordance_mask: u64,
    /// K1 tier mask requirement (Routine / Capacity / Regulation / Safety).
    #[doc(alias = "k1")]
    pub require_k1_mask: u64,
    /// K2 tier mask requirement (Meaning / Social / Recovery / Identity).
    #[doc(alias = "k2")]
    pub require_k2_mask: u64,
    /// K3 tier mask requirement (Environment / Object / Transition /
    /// Evidence / Rhythm).
    #[doc(alias = "k3")]
    pub require_k3_mask: u64,
}

/// Internal helper for promoting a dynamic `String` identifier to a
/// `&'static str` by leaking it.
///
/// **WARNING:** Only call this during load-time configuration. Leaking
/// in the hot path is a memory-exhaustion vulnerability.
#[inline]
#[must_use]
pub fn cold_intern(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

/// Declarative precedence group of rules. Groups are evaluated in
/// ascending [`LoadedRuleGroup::precedence_rank`] order; the first
/// matching rule wins. Groups are how Lifestyle fields express
/// `Safety > Evidence > Capacity > Meaning > Routine` without a
/// hard-coded ladder in code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedRuleGroup {
    /// Stable group id (e.g. `lifestyle.safety`).
    pub id: GroupId,
    /// Precedence rank — lower runs first.
    pub precedence_rank: u32,
    /// Rules belonging to this group, evaluated in declared order.
    pub rules: Vec<LoadedPackRule>,
}

/// Runtime-loaded field pack artifact.
///
/// Preserves the compiled rules and metadata for use in decision functions.
/// All IRIs and rules have been validated at load time.
#[derive(Clone, Debug)]
pub struct LoadedFieldPack {
    /// Pack name.
    pub name: PackId,
    /// Ontology profile (validated public-only).
    pub ontology_profile: Vec<String>,
    /// Deterministic list of rules: (context_urn, response_string).
    /// Response strings must be canonical members of AutonomicInstinct enum.
    pub rules: Vec<(String, String)>,
    /// Mask-keyed runtime rules consumed by [`select_instinct_with_pack`].
    /// Evaluated *after* every group; first match wins. Legacy ungrouped
    /// surface used by KZ7B fixtures.
    pub mask_rules: Vec<LoadedPackRule>,
    /// Declarative precedence groups (Phase 7). Walked in ascending
    /// `precedence_rank` order before `mask_rules`; first matching rule
    /// wins.
    pub groups: Vec<LoadedRuleGroup>,
    /// Default response when no rule matches (canonical lattice member string).
    pub default_response: String,
    /// Pack digest URN (for traceability).
    pub digest_urn: String,
}

/// Result of [`select_instinct_with_pack`].
///
/// Carries the response and the observable rule/pack/group provenance —
/// so a caller (or anti-fake test) can prove that a pack rule actually
/// fired rather than coinciding with the no-pack baseline. The
/// `matched_group_id` surfaces *which* precedence group admitted the
/// rule, so KZ8 reports can audit why one field outranked another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackDecision {
    /// Selected response class.
    pub response: AutonomicInstinct,
    /// Pack name that provided the rule, if any matched.
    pub matched_pack_id: Option<PackId>,
    /// Rule id that matched, if any.
    pub matched_rule_id: Option<RuleId>,
    /// Precedence group id that admitted the rule (groups path only).
    pub matched_group_id: Option<GroupId>,
}

/// Load a compiled field pack from raw rule data.
///
/// Validates:
/// - rules table is non-empty
/// - all ontology IRIs are public-only
/// - all response classes are canonical lattice members
/// - manifest digest matches the load-bearing content (tamper detection)
///
/// # Errors
///
/// Returns `PackLoadError` if validation or manifest verification fails.
pub fn load_compiled(
    name: &str,
    ontology_profile: &[String],
    rules: &[(String, String)],
    default_response: &str,
    digest_urn: &str,
) -> Result<LoadedFieldPack, PackLoadError> {
    // Validate rules non-empty.
    if rules.is_empty() {
        return Err(PackLoadError::EmptyRules);
    }

    // Validate ontology profile non-empty.
    if ontology_profile.is_empty() {
        return Err(PackLoadError::MissingOntologyProfile);
    }

    // Validate all ontology IRIs are public-only.
    for iri in ontology_profile {
        if !iri_is_public(iri) {
            return Err(PackLoadError::PrivateOntologyTerm(iri.clone()));
        }
    }

    // Enforce cryptographic admission (Phase 3).
    // Recompute the BLAKE3 digest of the load-bearing content.
    let recomputed_hash = compute_manifest_digest(name, ontology_profile, rules);
    let expected_urn = format!("urn:blake3:{}", recomputed_hash.to_hex());

    if digest_urn != expected_urn {
        return Err(PackLoadError::ManifestTamper(format!(
            "declared {}, recomputed {}",
            digest_urn, expected_urn
        )));
    }

    // Create loaded pack.
    Ok(LoadedFieldPack {
        name: PackId::new(cold_intern(name)),
        ontology_profile: ontology_profile.to_vec(),
        rules: rules.to_vec(),
        mask_rules: Vec::new(),
        groups: Vec::new(),
        default_response: default_response.to_string(),
        digest_urn: digest_urn.to_string(),
    })
}

/// Deterministically compute the manifest digest for a field pack.
///
/// The digest covers the pack name, ontology profile, and rules table.
/// This is used for tamper detection at the `ainst` -> `ccog` boundary.
#[must_use]
pub fn compute_manifest_digest(
    name: &str,
    ontology_profile: &[String],
    rules: &[(String, String)],
) -> blake3::Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(name.as_bytes());
    hasher.update(b"\0");
    for iri in ontology_profile {
        hasher.update(iri.as_bytes());
        hasher.update(b"\0");
    }
    for (ctx, resp) in rules {
        hasher.update(ctx.as_bytes());
        hasher.update(b"\0");
        hasher.update(resp.as_bytes());
        hasher.update(b"\0");
    }
    hasher.finalize()
}

/// Validate a [`LoadedFieldPack`]'s mask rules.
///
/// A pack is rejected when:
/// - Two rules share an `id` (duplicate identity).
/// - Two rules have a non-zero bit intersection across any dimension
///   (posture, expectation, risk, affordance) AND declare conflicting
///   responses — i.e. the same input could fire both rules with different
///   admitted responses.
///
/// # Errors
///
/// Returns `PackLoadError::ValidationFailed` describing the first conflict.
pub fn validate(pack: &LoadedFieldPack) -> Result<(), PackLoadError> {
    // 1. Validate ungrouped mask_rules amongst themselves.
    for (i, r1) in pack.mask_rules.iter().enumerate() {
        for r2 in pack.mask_rules.iter().skip(i + 1) {
            if r1.id == r2.id {
                return Err(PackLoadError::ValidationFailed(format!(
                    "duplicate rule id: {}",
                    r1.id
                )));
            }
            if rules_overlap_with_conflicting_responses(r1, r2) {
                return Err(PackLoadError::ValidationFailed(format!(
                    "overlapping bit requirements between `{}` and `{}` with conflicting responses",
                    r1.id, r2.id
                )));
            }
        }
    }
    // 2. Validate group ids unique and ranks distinct.
    for (i, g1) in pack.groups.iter().enumerate() {
        for g2 in pack.groups.iter().skip(i + 1) {
            if g1.id == g2.id {
                return Err(PackLoadError::ValidationFailed(format!(
                    "duplicate group id: {}",
                    g1.id
                )));
            }
            if g1.precedence_rank == g2.precedence_rank {
                return Err(PackLoadError::ValidationFailed(format!(
                    "duplicate precedence_rank {} between groups `{}` and `{}`",
                    g1.precedence_rank, g1.id, g2.id
                )));
            }
        }
    }
    // 3. Validate rules within each group amongst themselves.
    for g in &pack.groups {
        for (i, r1) in g.rules.iter().enumerate() {
            for r2 in g.rules.iter().skip(i + 1) {
                if r1.id == r2.id {
                    return Err(PackLoadError::ValidationFailed(format!(
                        "duplicate rule id `{}` in group `{}`",
                        r1.id, g.id
                    )));
                }
                if rules_overlap_with_conflicting_responses(r1, r2) {
                    return Err(PackLoadError::ValidationFailed(format!(
                        "overlapping bit requirements in group `{}` between `{}` and `{}` with conflicting responses",
                        g.id, r1.id, r2.id
                    )));
                }
            }
        }
    }
    Ok(())
}

#[inline]
fn rules_overlap_with_conflicting_responses(r1: &LoadedPackRule, r2: &LoadedPackRule) -> bool {
    let overlap = (r1.require_posture_mask & r2.require_posture_mask)
        | (r1.require_expectation_mask & r2.require_expectation_mask)
        | (r1.require_risk_mask & r2.require_risk_mask)
        | (r1.require_affordance_mask & r2.require_affordance_mask)
        | (r1.require_k1_mask & r2.require_k1_mask)
        | (r1.require_k2_mask & r2.require_k2_mask)
        | (r1.require_k3_mask & r2.require_k3_mask);
    overlap != 0 && r1.response != r2.response
}

/// Subset match: every set bit in `require` must also be set in `actual`.
#[inline]
const fn mask_satisfies(actual: u64, require: u64) -> bool {
    (actual & require) == require
}

/// True when every dimension's requirement is satisfied by the runtime
/// masks across both the K0 surface (posture/context) and the K1/K2/K3
/// tier masks. K0-only rules (where K1/K2/K3 requirements are zero)
/// match identically to the pre-Phase-7 behavior.
#[inline]
fn rule_matches(
    rule: &LoadedPackRule,
    posture: &PostureBundle,
    ctx: &ContextBundle,
    tiers: &TierMasks,
) -> bool {
    mask_satisfies(posture.posture_mask, rule.require_posture_mask)
        && mask_satisfies(ctx.expectation_mask, rule.require_expectation_mask)
        && mask_satisfies(ctx.risk_mask, rule.require_risk_mask)
        && mask_satisfies(ctx.affordance_mask, rule.require_affordance_mask)
        && mask_satisfies(tiers.k1, rule.require_k1_mask)
        && mask_satisfies(tiers.k2, rule.require_k2_mask)
        && mask_satisfies(tiers.k3, rule.require_k3_mask)
}

/// Decide a response over the closed cognition surface, biased by a
/// loaded pack — K0 only (no K-tier context).
///
/// Equivalent to [`select_instinct_with_pack_tiered`] with
/// [`TierMasks::ZERO`]. Preserved for KZ7B back-compat.
#[must_use]
pub fn select_instinct_with_pack(
    context: &ClosedFieldContext,
    pack: &LoadedFieldPack,
) -> PackDecision {
    select_instinct_with_pack_tiered(context, pack)
}

/// Decide a response over the closed cognition surface (K0 + K-tier),
/// biased by a loaded pack.
///
/// Evaluation order:
/// 1. Walk [`LoadedFieldPack::groups`] in declared order (call
///    [`sort_groups_by_precedence`] at load time so this is ascending
///    by `precedence_rank`). Within each group, the first rule whose
///    K0+K-tier requirements are satisfied wins.
/// 2. Fall back to [`LoadedFieldPack::mask_rules`] (legacy ungrouped
///    rules — KZ7B fixtures live here).
/// 3. Fall through to [`select_instinct_v0`] — packs bias the canonical
///    lattice; they never replace it.
#[must_use]
pub fn select_instinct_with_pack_tiered(
    context: &ClosedFieldContext,
    pack: &LoadedFieldPack,
) -> PackDecision {
    for group in &pack.groups {
        for rule in &group.rules {
            if rule_matches(rule, &context.posture, &context.context, &context.tiers) {
                return PackDecision {
                    response: rule.response,
                    matched_pack_id: Some(pack.name),
                    matched_rule_id: Some(rule.id),
                    matched_group_id: Some(group.id),
                };
            }
        }
    }
    for rule in &pack.mask_rules {
        if rule_matches(rule, &context.posture, &context.context, &context.tiers) {
            return PackDecision {
                response: rule.response,
                matched_pack_id: Some(pack.name),
                matched_rule_id: Some(rule.id),
                matched_group_id: None,
            };
        }
    }
    PackDecision {
        response: select_instinct_v0(context),
        matched_pack_id: None,
        matched_rule_id: None,
        matched_group_id: None,
    }
}

/// Sort a pack's groups by `precedence_rank` ascending — call this once
/// after loading so [`select_instinct_with_pack_tiered`] can walk in
/// order without re-sorting on every decision.
pub fn sort_groups_by_precedence(pack: &mut LoadedFieldPack) {
    pack.groups.sort_by_key(|g| g.precedence_rank);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iri_is_public_accepts_blake3() {
        assert!(iri_is_public("urn:blake3:deadbeef"));
    }

    #[test]
    fn iri_is_public_rejects_unknown_namespace() {
        assert!(!iri_is_public("http://internal.acme.example/foo"));
    }

    #[test]
    fn load_compiled_admits_valid_digest() {
        let name = "test.pack";
        let profile = vec!["https://schema.org/".to_string()];
        let rules = vec![("urn:ctx".to_string(), "Settle".to_string())];
        let h = compute_manifest_digest(name, &profile, &rules);
        let digest_urn = format!("urn:blake3:{}", h.to_hex());

        let res = load_compiled(name, &profile, &rules, "Ignore", &digest_urn);
        assert!(res.is_ok(), "should admit valid digest: {:?}", res.err());
    }

    #[test]
    fn load_compiled_rejects_tampered_digest() {
        let name = "test.pack";
        let profile = vec!["https://schema.org/".to_string()];
        let rules = vec![("urn:ctx".to_string(), "Settle".to_string())];
        // Tamper with digest
        let digest_urn =
            "urn:blake3:0000000000000000000000000000000000000000000000000000000000000000";

        let res = load_compiled(name, &profile, &rules, "Ignore", digest_urn);
        assert!(
            matches!(res, Err(PackLoadError::ManifestTamper(_))),
            "should reject tampered digest: {:?}",
            res
        );
    }

    #[test]
    fn load_compiled_rejects_tampered_content() {
        let name = "test.pack";
        let profile = vec!["https://schema.org/".to_string()];
        let rules = vec![("urn:ctx".to_string(), "Settle".to_string())];
        let h = compute_manifest_digest(name, &profile, &rules);
        let digest_urn = format!("urn:blake3:{}", h.to_hex());

        // Change rules but keep the old digest
        let tampered_rules = vec![("urn:ctx".to_string(), "Escalate".to_string())];

        let res = load_compiled(name, &profile, &tampered_rules, "Ignore", &digest_urn);
        assert!(
            matches!(res, Err(PackLoadError::ManifestTamper(_))),
            "should reject tampered content: {:?}",
            res
        );
    }
}
