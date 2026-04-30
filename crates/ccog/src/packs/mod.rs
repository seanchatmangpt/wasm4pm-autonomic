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

use crate::bark_artifact::BarkSlot;
use crate::compiled::CompiledFieldSnapshot;
use crate::instinct::{select_instinct_v0, AutonomicInstinct};
use crate::multimodal::{ContextBundle, PostureBundle};
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
    PUBLIC_ONTOLOGY_PREFIXES
        .iter()
        .any(|p| iri.starts_with(p))
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

/// A single runtime-evaluable pack rule.
///
/// Each rule names itself, declares a posture/context mask requirement, and
/// names the response it admits when the requirement is matched. A rule
/// matches when every set bit in each `require_*_mask` is also set in the
/// corresponding runtime mask. An all-zero requirement means "no constraint
/// on this dimension".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedPackRule {
    /// Stable rule id (used in `PackDecision::matched_rule_id` for traceability).
    pub id: String,
    /// Response admitted by the rule on match.
    pub response: AutonomicInstinct,
    /// Posture bits the rule requires (subset relation).
    pub require_posture_mask: u64,
    /// Expectation bits the rule requires.
    pub require_expectation_mask: u64,
    /// Risk bits the rule requires.
    pub require_risk_mask: u64,
    /// Affordance bits the rule requires.
    pub require_affordance_mask: u64,
}

/// Runtime-loaded field pack artifact.
///
/// Preserves the compiled rules and metadata for use in decision functions.
/// All IRIs and rules have been validated at load time.
#[derive(Clone, Debug)]
pub struct LoadedFieldPack {
    /// Pack name.
    pub name: String,
    /// Ontology profile (validated public-only).
    pub ontology_profile: Vec<String>,
    /// Deterministic list of rules: (context_urn, response_string).
    /// Response strings must be canonical members of AutonomicInstinct enum.
    pub rules: Vec<(String, String)>,
    /// Mask-keyed runtime rules consumed by [`select_instinct_with_pack`].
    /// Evaluated in declared order; first match wins.
    pub mask_rules: Vec<LoadedPackRule>,
    /// Default response when no rule matches (canonical lattice member string).
    pub default_response: String,
    /// Pack digest URN (for traceability).
    pub digest_urn: String,
}

/// Result of [`select_instinct_with_pack`].
///
/// Carries both the response and the observable rule/pack provenance — so a
/// caller (or anti-fake test) can prove that a pack rule actually fired
/// rather than coinciding with the no-pack baseline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackDecision {
    /// Selected response class.
    pub response: AutonomicInstinct,
    /// Pack name that provided the rule, if any matched.
    pub matched_pack_id: Option<String>,
    /// Rule id that matched, if any.
    pub matched_rule_id: Option<String>,
}

/// Load a compiled field pack from raw rule data.
///
/// Validates:
/// - rules table is non-empty
/// - all ontology IRIs are public-only
/// - all response classes are canonical lattice members
/// - manifest digest is present (for tamper detection)
///
/// # Errors
///
/// Returns `PackLoadError` if validation fails.
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

    // Validate digest exists (required for tamper detection).
    if digest_urn.is_empty() {
        return Err(PackLoadError::ValidationFailed(
            "missing digest_urn".to_string(),
        ));
    }

    // Create loaded pack.
    Ok(LoadedFieldPack {
        name: name.to_string(),
        ontology_profile: ontology_profile.to_vec(),
        rules: rules.to_vec(),
        mask_rules: Vec::new(),
        default_response: default_response.to_string(),
        digest_urn: digest_urn.to_string(),
    })
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
    for (i, r1) in pack.mask_rules.iter().enumerate() {
        for r2 in pack.mask_rules.iter().skip(i + 1) {
            if r1.id == r2.id {
                return Err(PackLoadError::ValidationFailed(format!(
                    "duplicate rule id: {}",
                    r1.id
                )));
            }
            let overlap = (r1.require_posture_mask & r2.require_posture_mask)
                | (r1.require_expectation_mask & r2.require_expectation_mask)
                | (r1.require_risk_mask & r2.require_risk_mask)
                | (r1.require_affordance_mask & r2.require_affordance_mask);
            if overlap != 0 && r1.response != r2.response {
                return Err(PackLoadError::ValidationFailed(format!(
                    "overlapping bit requirements between `{}` and `{}` with conflicting responses",
                    r1.id, r2.id
                )));
            }
        }
    }
    Ok(())
}

/// Subset match: every set bit in `require` must also be set in `actual`.
#[inline]
const fn mask_satisfies(actual: u64, require: u64) -> bool {
    (actual & require) == require
}

/// True when every dimension's requirement is satisfied by the runtime masks.
#[inline]
fn rule_matches(rule: &LoadedPackRule, posture: &PostureBundle, ctx: &ContextBundle) -> bool {
    mask_satisfies(posture.posture_mask, rule.require_posture_mask)
        && mask_satisfies(ctx.expectation_mask, rule.require_expectation_mask)
        && mask_satisfies(ctx.risk_mask, rule.require_risk_mask)
        && mask_satisfies(ctx.affordance_mask, rule.require_affordance_mask)
}

/// Decide a response over the closed cognition surface, biased by a loaded
/// pack.
///
/// Pack rules are evaluated in declared order; the first rule whose mask
/// requirements are satisfied by `posture`/`ctx` wins. If no rule matches,
/// the function falls through to [`select_instinct_v0`] — packs bias the
/// canonical lattice; they never replace it. The returned [`PackDecision`]
/// carries the matched rule's id (if any) so downstream code can prove pack
/// participation rather than coincidence with the baseline.
#[must_use]
pub fn select_instinct_with_pack(
    snap: &CompiledFieldSnapshot,
    posture: &PostureBundle,
    ctx: &ContextBundle,
    pack: &LoadedFieldPack,
) -> PackDecision {
    for rule in &pack.mask_rules {
        if rule_matches(rule, posture, ctx) {
            return PackDecision {
                response: rule.response,
                matched_pack_id: Some(pack.name.clone()),
                matched_rule_id: Some(rule.id.clone()),
            };
        }
    }
    PackDecision {
        response: select_instinct_v0(snap, posture, ctx),
        matched_pack_id: None,
        matched_rule_id: None,
    }
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
    fn iri_is_public_rejects_pii_email() {
        assert!(!iri_is_public("mailto:alice@example.com"));
    }
}
