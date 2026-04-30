//! Constitutional invariants from `SPR.md`.
//!
//! Every claim in the doctrine that can be machine-checked is exposed as
//! a function here so tests, CI, and downstream code can ask: "is this
//! still constitutional?" The doctrine and the code stay coupled by the
//! tests in this module — diverging breaks the build.

use serde::{Deserialize, Serialize};

use crate::AutonomicInstinct;

/// Earned-zero classification per SPR §"Earned Action".
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EarnedZeroClass {
    /// Cost floor measured at the kernel boundary.
    KernelFloor,
    /// Closure was achieved and the response was deliberately zero.
    ClosureEarnedAdmission,
    /// Predecessor plan node did not advance.
    PredecessorSkip,
    /// `(require_mask & present_mask) != require_mask`.
    RequirementMaskFailure,
    /// Posture/context denied the response.
    ContextDenial,
    /// Manual-only hook; not fired by default.
    ManualOnlySkip,
    /// Trace replay diverged from canonical decision.
    ConformanceFailure,
}

impl EarnedZeroClass {
    /// Stable string token for serialization and audit logs.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::KernelFloor => "KernelFloor",
            Self::ClosureEarnedAdmission => "ClosureEarnedAdmission",
            Self::PredecessorSkip => "PredecessorSkip",
            Self::RequirementMaskFailure => "RequirementMaskFailure",
            Self::ContextDenial => "ContextDenial",
            Self::ManualOnlySkip => "ManualOnlySkip",
            Self::ConformanceFailure => "ConformanceFailure",
        }
    }

    /// Every supported earned-zero class. Used by the constitutional
    /// invariant test.
    #[must_use]
    pub const fn all() -> &'static [EarnedZeroClass] {
        &[
            Self::KernelFloor,
            Self::ClosureEarnedAdmission,
            Self::PredecessorSkip,
            Self::RequirementMaskFailure,
            Self::ContextDenial,
            Self::ManualOnlySkip,
            Self::ConformanceFailure,
        ]
    }
}

/// The three identity surfaces that must never be conflated.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IdentitySurface {
    /// What was sealed.
    Receipt,
    /// How it was earned.
    Trace,
    /// What it cost.
    Benchmark,
}

impl IdentitySurface {
    /// Every supported identity surface.
    #[must_use]
    pub const fn all() -> &'static [IdentitySurface] {
        &[Self::Receipt, Self::Trace, Self::Benchmark]
    }
}

/// Forbidden regressions per SPR §"Forbidden Regressions".
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ForbiddenRegression {
    /// `<gap-doc> prov:value "placeholder"` etc.
    FakeProvValue,
    /// Phrase binding emitting `skos:definition "derived from prefLabel"`.
    DerivedFromPrefLabelPlaceholder,
    /// `<instance> sh:targetClass <Class>` on ordinary instances.
    ShaclInstanceMisuse,
    /// `Utc::now` mixed into receipt URN material.
    TimestampReceiptIdentity,
    /// Decision and materialization in one fused call.
    FusedDecideMaterialize,
    /// `Manual` trigger auto-firing without explicit invocation.
    ManualTriggerAutoFire,
    /// Mixing plan-node, runtime-slot, and predicate-bit mask domains.
    MaskDomainConfusion,
    /// POWL64 path written but never replayed.
    DecorativePowl64Path,
    /// Pack posture/context bits leaking outside their reserved range.
    PackBitLeakage,
    /// Healthcare pack claiming diagnostic authority.
    HealthcareOverclaiming,
}

impl ForbiddenRegression {
    /// Every forbidden regression. Each must have ≥1 boundary detector
    /// somewhere in the workspace; the doctrine test asserts coverage.
    #[must_use]
    pub const fn all() -> &'static [ForbiddenRegression] {
        &[
            Self::FakeProvValue,
            Self::DerivedFromPrefLabelPlaceholder,
            Self::ShaclInstanceMisuse,
            Self::TimestampReceiptIdentity,
            Self::FusedDecideMaterialize,
            Self::ManualTriggerAutoFire,
            Self::MaskDomainConfusion,
            Self::DecorativePowl64Path,
            Self::PackBitLeakage,
            Self::HealthcareOverclaiming,
        ]
    }
}

/// Public ontology profiles AutoInstinct admits per SPR.
#[must_use]
pub const fn public_ontology_profiles() -> &'static [&'static str] {
    &[
        "https://schema.org/",
        "http://www.w3.org/ns/prov#",
        "http://www.w3.org/ns/sosa/",
        "http://www.w3.org/2004/02/skos/core#",
        "http://www.w3.org/2006/time#",
        "http://www.opengis.net/ont/geosparql#",
        "http://qudt.org/schema/qudt/",
        "http://www.w3.org/ns/shacl#",
        "http://www.w3.org/ns/odrl/2/",
        "http://www.w3.org/2001/XMLSchema#",
        "urn:blake3:",
        "urn:ccog:vocab:",
    ]
}

/// Canonical CLI verb-noun pairs per SPR §"CLI Grammar".
#[must_use]
pub const fn canonical_cli_commands() -> &'static [(&'static str, &'static str)] {
    &[
        ("generate", "ocel"),
        ("validate", "ocel"),
        ("ingest", "corpus"),
        ("discover", "motifs"),
        ("propose", "policy"),
        ("generate", "jtbd"),
        ("run", "gauntlet"),
        ("compile", "pack"),
        ("publish", "pack"),
        ("deploy", "edge"),
        ("verify", "replay"),
        ("export", "bundle"),
    ]
}

/// Canonical response lattice per SPR. `Ignore` is the safe default per
/// `ccog::instinct::AutonomicInstinct::Default`.
#[must_use]
pub const fn canonical_response_lattice() -> &'static [AutonomicInstinct] {
    &[
        AutonomicInstinct::Settle,
        AutonomicInstinct::Retrieve,
        AutonomicInstinct::Inspect,
        AutonomicInstinct::Ask,
        AutonomicInstinct::Refuse,
        AutonomicInstinct::Escalate,
        AutonomicInstinct::Ignore,
    ]
}

/// Benchmark tier names per SPR §"Benchmark Tiers".
#[must_use]
pub const fn benchmark_tiers() -> &'static [&'static str] {
    &[
        "KernelFloor",
        "CompiledBark",
        "Materialization",
        "ReceiptPath",
        "FullProcess",
        "ConformanceReplay",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Constitutional check 1: every earned-zero class has a stable token
    /// and the set is exactly seven (matches SPR §"Earned-zero classes").
    #[test]
    fn doctrine_earned_zero_classes_are_seven() {
        let all = EarnedZeroClass::all();
        assert_eq!(all.len(), 7);
        let mut tokens: Vec<&'static str> = all.iter().map(|c| c.token()).collect();
        tokens.sort();
        tokens.dedup();
        assert_eq!(tokens.len(), 7, "tokens must be unique");
    }

    /// Constitutional check 2: identity surfaces are exactly three and
    /// are not aliased (Receipt ≠ Trace ≠ Benchmark).
    #[test]
    fn doctrine_three_identity_surfaces() {
        let all = IdentitySurface::all();
        assert_eq!(all.len(), 3);
        assert_ne!(all[0], all[1]);
        assert_ne!(all[1], all[2]);
        assert_ne!(all[0], all[2]);
    }

    /// Constitutional check 3: every SPR-listed forbidden regression has
    /// a discriminant in [`ForbiddenRegression`].
    #[test]
    fn doctrine_forbidden_regressions_are_ten() {
        let all = ForbiddenRegression::all();
        assert_eq!(all.len(), 10);
    }

    /// Constitutional check 4: response lattice has exactly seven canonical
    /// classes and never grows here (forks are caught at the type level
    /// in ccog::instinct).
    #[test]
    fn doctrine_response_lattice_is_seven() {
        let lattice = canonical_response_lattice();
        assert_eq!(lattice.len(), 7);
    }

    /// Constitutional check 5: CLI grammar has exactly twelve canonical
    /// verb-noun pairs (PRD §14.1).
    #[test]
    fn doctrine_cli_grammar_is_twelve() {
        let pairs = canonical_cli_commands();
        assert_eq!(pairs.len(), 12);
        // Each pair must have a non-empty verb and noun.
        for (v, n) in pairs {
            assert!(!v.is_empty());
            assert!(!n.is_empty());
        }
    }

    /// Constitutional check 6: benchmark tiers are exactly six and follow
    /// the canonical names.
    #[test]
    fn doctrine_benchmark_tiers_are_six_canonical() {
        let tiers = benchmark_tiers();
        assert_eq!(tiers.len(), 6);
        assert_eq!(tiers[0], "KernelFloor");
        assert_eq!(tiers[5], "ConformanceReplay");
    }

    /// Constitutional check 7: every public-ontology prefix is anchored
    /// to a recognized commons (no private namespaces leak in).
    #[test]
    fn doctrine_public_ontology_prefixes_are_public() {
        for p in public_ontology_profiles() {
            let ok = p.starts_with("http://www.w3.org/")
                || p.starts_with("https://schema.org/")
                || p.starts_with("http://purl.org/")
                || p.starts_with("http://qudt.org/")
                || p.starts_with("http://www.opengis.net/")
                || p.starts_with("urn:blake3:")
                || p.starts_with("urn:ccog:");
            assert!(ok, "private namespace leaked into public profile: {}", p);
        }
    }

    /// Constitutional check 8: `Ignore` is the canonical safe default.
    #[test]
    fn doctrine_safe_default_is_ignore() {
        let default = AutonomicInstinct::default();
        assert_eq!(default, AutonomicInstinct::Ignore);
    }
}
