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
