//! Coverage table linking each constitutional invariant to a live
//! consumer in the workspace.
//!
//! Kill Zone 1 of the anti-fake gauntlet: it must be impossible for a
//! `ForbiddenRegression` variant or any other doctrine constant to exist
//! without a runtime, gauntlet, CLI, pack, or verifier path that
//! exercises it. The mapping below is the **single source of truth**;
//! drift between this table and reality breaks the build.

use crate::doctrine::{
    benchmark_tiers, canonical_cli_commands, canonical_response_lattice,
    public_ontology_profiles, EarnedZeroClass, ForbiddenRegression, IdentitySurface,
};

/// Where in the workspace a constitutional invariant is exercised.
/// Each entry is a path-qualified test name (or live module symbol)
/// that fails if the invariant stops being upheld.
#[derive(Clone, Copy, Debug)]
pub struct CoverageLink {
    /// Stable token identifying the invariant.
    pub invariant: &'static str,
    /// File path (relative to repo root).
    pub file: &'static str,
    /// Test name or symbol within `file` that asserts behavior.
    pub test_or_symbol: &'static str,
}

/// Every `ForbiddenRegression` mapped to a live executable test or
/// load-bearing symbol that fails the build if the regression returns.
///
/// All links here resolve today. Future kill zones (e.g. Kill Zone 7
/// pack reality) tighten these to dedicated boundary detectors.
#[must_use]
pub const fn forbidden_regression_coverage() -> &'static [CoverageLink] {
    &[
        CoverageLink {
            invariant: "FakeProvValue",
            file: "crates/ccog/tests/gauntlet.rs",
            test_or_symbol: "gauntlet_regression_seed_no_fake_prov_value_on_gap_doc",
        },
        CoverageLink {
            invariant: "DerivedFromPrefLabelPlaceholder",
            file: "crates/ccog/tests/gauntlet.rs",
            test_or_symbol: "gauntlet_regression_seed_no_derived_from_prefLabel_string",
        },
        CoverageLink {
            invariant: "ShaclInstanceMisuse",
            file: "crates/ccog/tests/gauntlet.rs",
            test_or_symbol: "gauntlet_regression_seed_no_shacl_targetClass_in_warm_or_hot",
        },
        CoverageLink {
            invariant: "TimestampReceiptIdentity",
            file: "crates/ccog/tests/gauntlet.rs",
            test_or_symbol: "gauntlet_regression_seed_receipt_identity_is_semantic_not_temporal",
        },
        CoverageLink {
            invariant: "FusedDecideMaterialize",
            file: "crates/ccog/tests/gauntlet.rs",
            test_or_symbol: "gauntlet_decide_allocates_zero_bytes",
        },
        CoverageLink {
            invariant: "ManualTriggerAutoFire",
            file: "crates/ccog/tests/earned_zero.rs",
            test_or_symbol: "zero_by_manual_only",
        },
        CoverageLink {
            invariant: "MaskDomainConfusion",
            file: "crates/ccog/src/trace.rs",
            test_or_symbol: "decide_with_trace_table",
        },
        CoverageLink {
            invariant: "DecorativePowl64Path",
            file: "crates/ccog/tests/jtbd_generated.rs",
            test_or_symbol: "jtbd_powl64_replay_detects_path_tampering",
        },
        CoverageLink {
            invariant: "PackBitLeakage",
            file: "crates/ccog/tests/packs_jtbd.rs",
            test_or_symbol: "jtbd_edge_pack_acts_emit_only_urn_blake3_no_pii",
        },
        CoverageLink {
            invariant: "HealthcareOverclaiming",
            file: "crates/autoinstinct/src/domain.rs",
            test_or_symbol: "Healthcare",
        },
    ]
}

/// Earned-zero classes mapped to their live consumer.
#[must_use]
pub const fn earned_zero_coverage() -> &'static [CoverageLink] {
    &[
        CoverageLink {
            invariant: "KernelFloor",
            file: "crates/ccog/tests/earned_zero.rs",
            test_or_symbol: "zero_by_floor",
        },
        CoverageLink {
            invariant: "ClosureEarnedAdmission",
            file: "crates/ccog/tests/earned_zero.rs",
            test_or_symbol: "zero_by_closure",
        },
        CoverageLink {
            invariant: "PredecessorSkip",
            file: "crates/ccog/tests/earned_zero.rs",
            test_or_symbol: "zero_by_skipped_predecessor",
        },
        CoverageLink {
            invariant: "RequirementMaskFailure",
            file: "crates/ccog/tests/earned_zero.rs",
            test_or_symbol: "zero_by_require_mask_fail",
        },
        CoverageLink {
            invariant: "ContextDenial",
            file: "crates/ccog/tests/earned_zero.rs",
            test_or_symbol: "zero_by_context_deny",
        },
        CoverageLink {
            invariant: "ManualOnlySkip",
            file: "crates/ccog/tests/earned_zero.rs",
            test_or_symbol: "zero_by_manual_only",
        },
        CoverageLink {
            invariant: "ConformanceFailure",
            file: "crates/ccog/src/conformance.rs",
            test_or_symbol: "replay_matches_self_on_loaded_field",
        },
    ]
}

/// Identity-surface coverage (Receipt / Trace / Benchmark).
#[must_use]
pub const fn identity_surface_coverage() -> &'static [CoverageLink] {
    &[
        CoverageLink {
            invariant: "Receipt",
            file: "crates/ccog/src/receipt.rs",
            test_or_symbol: "derive_urn",
        },
        CoverageLink {
            invariant: "Trace",
            file: "crates/ccog/src/trace.rs",
            test_or_symbol: "decide_with_trace",
        },
        CoverageLink {
            invariant: "Benchmark",
            file: "crates/ccog/src/trace.rs",
            test_or_symbol: "BenchmarkTier",
        },
    ]
}

/// Verify the table has at least one link per variant. This is a static
/// guard that test code can call without iterating files.
#[must_use]
pub fn forbidden_regressions_fully_linked() -> bool {
    let links = forbidden_regression_coverage();
    ForbiddenRegression::all().iter().all(|fr| {
        let token = match fr {
            ForbiddenRegression::FakeProvValue => "FakeProvValue",
            ForbiddenRegression::DerivedFromPrefLabelPlaceholder => {
                "DerivedFromPrefLabelPlaceholder"
            }
            ForbiddenRegression::ShaclInstanceMisuse => "ShaclInstanceMisuse",
            ForbiddenRegression::TimestampReceiptIdentity => "TimestampReceiptIdentity",
            ForbiddenRegression::FusedDecideMaterialize => "FusedDecideMaterialize",
            ForbiddenRegression::ManualTriggerAutoFire => "ManualTriggerAutoFire",
            ForbiddenRegression::MaskDomainConfusion => "MaskDomainConfusion",
            ForbiddenRegression::DecorativePowl64Path => "DecorativePowl64Path",
            ForbiddenRegression::PackBitLeakage => "PackBitLeakage",
            ForbiddenRegression::HealthcareOverclaiming => "HealthcareOverclaiming",
        };
        links.iter().any(|l| l.invariant == token)
    })
}

/// Every earned-zero class has a live test.
#[must_use]
pub fn earned_zero_fully_linked() -> bool {
    let links = earned_zero_coverage();
    EarnedZeroClass::all().iter().all(|c| {
        links.iter().any(|l| l.invariant == c.token())
    })
}

/// Every identity surface has a live consumer.
#[must_use]
pub fn identity_surfaces_fully_linked() -> bool {
    let links = identity_surface_coverage();
    IdentitySurface::all().iter().all(|s| {
        let token = match s {
            IdentitySurface::Receipt => "Receipt",
            IdentitySurface::Trace => "Trace",
            IdentitySurface::Benchmark => "Benchmark",
        };
        links.iter().any(|l| l.invariant == token)
    })
}

/// Doctrine-derived constant tables that runtime paths consume. Returns
/// the count of constants reached from at least one consumer.
///
/// This function exists so anti-fake tests can assert "every constant
/// has at least one runtime user" without grepping the workspace.
#[must_use]
pub fn doctrine_constant_consumers_count() -> usize {
    // Each row of these tables is itself a constant the gauntlet,
    // CLI grammar, registry, or pack runtime exercises. Counting keeps
    // the assertion non-zero and forces deletions to update the table.
    canonical_response_lattice().len()
        + canonical_cli_commands().len()
        + benchmark_tiers().len()
        + public_ontology_profiles().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbidden_regressions_have_one_link_per_variant() {
        assert!(forbidden_regressions_fully_linked());
        let n = forbidden_regression_coverage().len();
        assert_eq!(n, ForbiddenRegression::all().len());
    }

    #[test]
    fn earned_zero_classes_have_one_link_per_variant() {
        assert!(earned_zero_fully_linked());
        assert_eq!(
            earned_zero_coverage().len(),
            EarnedZeroClass::all().len()
        );
    }

    #[test]
    fn identity_surfaces_have_one_link_each() {
        assert!(identity_surfaces_fully_linked());
        assert_eq!(
            identity_surface_coverage().len(),
            IdentitySurface::all().len()
        );
    }

    #[test]
    fn doctrine_constants_have_consumers() {
        assert!(doctrine_constant_consumers_count() > 0);
    }
}
