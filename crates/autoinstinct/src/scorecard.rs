//! Anti-fake scorecard (Phase 4 — Executive Gate).
//!
//! A [`Scorecard`] is the structured record produced by
//! `ainst run gauntlet --mode anti-fake`. It maps each release-blocking
//! anti-fake invariant to a single bool. The gate must exit nonzero unless
//! every required dimension is `true` — counting "tests passed" is not
//! sufficient because a passing test suite can hide a fake invariant.

use serde::{Deserialize, Serialize};

/// Required dimensions of the anti-fake gauntlet.
///
/// Every field is release-blocking. [`Scorecard::all_pass`] returns `true`
/// iff every field is `true` AND the metadata fields are non-empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scorecard {
    // ---------------- provenance ----------------
    /// Git working tree was clean when the gate ran.
    pub git_clean: bool,
    /// Recorded git commit hash.
    pub commit_recorded: String,
    /// Recorded toolchain string (rustc + cargo + platform).
    pub toolchain_recorded: String,

    // ---------------- KZ1: doctrine drift ----------------
    /// KZ1: doctrine drift detection passed.
    pub kz1_doctrine_drift_pass: bool,

    // ---------------- KZ2: causal ----------------
    /// KZ2: every perturbation produces its specific expected response.
    pub kz2_causal_expected_outcomes_pass: bool,
    /// KZ2: prov:value absence proven load-bearing for evidence-gap Ask.
    pub kz2_prov_value_absence_load_bearing_pass: bool,

    // ---------------- KZ4: OCEL authenticity ----------------
    /// KZ4: OCEL admission rejects flat/zero-counterfactual/private worlds.
    pub kz4_ocel_authenticity_pass: bool,

    // ---------------- KZ6: performance ----------------
    /// KZ6: positive control proves CountingAlloc detects deliberate alloc.
    pub kz6_allocation_positive_control_pass: bool,
    /// KZ6: hot-path `decide()` is alloc-free.
    pub kz6_zero_alloc_decide_pass: bool,

    // ---------------- KZ7: pack reality ----------------
    /// KZ7: manifest tamper rejected.
    pub kz7_manifest_tamper_pass: bool,
    /// KZ7: bad packs (overlap / missing profile / private ontology) rejected.
    pub kz7_bad_pack_rejection_pass: bool,
    /// KZ7: loaded pack changes runtime decision.
    pub kz7_runtime_loading_pass: bool,
    /// KZ7: matched_pack_id / matched_rule_id observable in decision.
    pub kz7_rule_metadata_observable_pass: bool,
    /// KZ7: no `assert!(true)` / `[Future]` / "blocked pending" placeholders.
    pub kz7_no_placeholders_pass: bool,

    // ---------------- regression ----------------
    /// `cargo test -p ccog --lib` clean.
    pub ccog_regression_pass: bool,
    /// `cargo test -p autoinstinct` clean.
    pub autoinstinct_regression_pass: bool,

    // ---------------- Phase 5 master integration ----------------
    /// End-to-end `OCEL → pack → ccog runtime → proof` story passes
    /// (`master_ocel_to_pack_to_ccog_runtime_to_proof`).
    pub master_ocel_to_pack_to_runtime_pass: bool,

    // ---------------- aggregate ----------------
    /// True iff every required dimension passes. Computed by [`Scorecard::recompute_overall`].
    pub overall_pass: bool,
}

impl Scorecard {
    /// True iff every required dimension passes AND provenance metadata is recorded.
    #[must_use]
    pub fn all_pass(&self) -> bool {
        !self.commit_recorded.is_empty()
            && !self.toolchain_recorded.is_empty()
            && self.git_clean
            && self.kz1_doctrine_drift_pass
            && self.kz2_causal_expected_outcomes_pass
            && self.kz2_prov_value_absence_load_bearing_pass
            && self.kz4_ocel_authenticity_pass
            && self.kz6_allocation_positive_control_pass
            && self.kz6_zero_alloc_decide_pass
            && self.kz7_manifest_tamper_pass
            && self.kz7_bad_pack_rejection_pass
            && self.kz7_runtime_loading_pass
            && self.kz7_rule_metadata_observable_pass
            && self.kz7_no_placeholders_pass
            && self.ccog_regression_pass
            && self.autoinstinct_regression_pass
            && self.master_ocel_to_pack_to_runtime_pass
    }

    /// Recompute and store [`Scorecard::overall_pass`] from the dimensions.
    pub fn recompute_overall(&mut self) {
        self.overall_pass = self.all_pass();
    }

    /// Serialize to canonical pretty JSON.
    ///
    /// # Errors
    /// Returns `serde_json::Error` if serialization fails (should not happen
    /// for a `Scorecard` value).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Names of the boolean dimensions that must all pass.
    #[must_use]
    pub fn dimension_names() -> &'static [&'static str] {
        &[
            "git_clean",
            "kz1_doctrine_drift_pass",
            "kz2_causal_expected_outcomes_pass",
            "kz2_prov_value_absence_load_bearing_pass",
            "kz4_ocel_authenticity_pass",
            "kz6_allocation_positive_control_pass",
            "kz6_zero_alloc_decide_pass",
            "kz7_manifest_tamper_pass",
            "kz7_bad_pack_rejection_pass",
            "kz7_runtime_loading_pass",
            "kz7_rule_metadata_observable_pass",
            "kz7_no_placeholders_pass",
            "ccog_regression_pass",
            "autoinstinct_regression_pass",
            "master_ocel_to_pack_to_runtime_pass",
        ]
    }
}

/// Names of the test binaries in `crates/autoinstinct/tests/` that the
/// executive gate runs. Each maps to one or more scorecard dimensions.
pub const KILLZONE_TEST_BINARIES: &[&str] = &[
    "anti_fake_doctrine",
    "anti_fake_causal",
    "anti_fake_ocel",
    "anti_fake_perf",
    "anti_fake_packs",
    "anti_fake_master",
];

/// Outcome of a single test binary run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BinaryOutcome {
    /// Test binary name (e.g. `anti_fake_packs`).
    pub binary: String,
    /// Overall test-binary exit success.
    pub success: bool,
    /// Names of tests reported as `... ok`.
    pub passing_tests: Vec<String>,
    /// Names of tests reported as `... FAILED`.
    pub failing_tests: Vec<String>,
}

impl BinaryOutcome {
    /// True iff every named test in `required` is in `passing_tests`.
    #[must_use]
    pub fn all_required_pass(&self, required: &[&str]) -> bool {
        required
            .iter()
            .all(|t| self.passing_tests.iter().any(|p| p == t))
    }
}

/// Parse `cargo test ...` stdout into a [`BinaryOutcome`].
///
/// Looks for `test <name> ... ok` and `test <name> ... FAILED` lines.
#[must_use]
pub fn parse_cargo_test_stdout(binary: &str, success: bool, stdout: &str) -> BinaryOutcome {
    let mut passing = Vec::new();
    let mut failing = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("test ") else {
            continue;
        };
        if let Some(name) = rest.strip_suffix(" ... ok") {
            passing.push(name.to_string());
        } else if let Some(name) = rest.strip_suffix(" ... FAILED") {
            failing.push(name.to_string());
        }
    }
    BinaryOutcome {
        binary: binary.to_string(),
        success,
        passing_tests: passing,
        failing_tests: failing,
    }
}

/// Build a scorecard with every required dimension set to `true` and
/// metadata stub strings — used as a base for tests and for the CLI gate.
#[must_use]
pub fn all_true_scorecard() -> Scorecard {
    Scorecard {
        git_clean: true,
        commit_recorded: "test-commit".to_string(),
        toolchain_recorded: "test-toolchain".to_string(),
        kz1_doctrine_drift_pass: true,
        kz2_causal_expected_outcomes_pass: true,
        kz2_prov_value_absence_load_bearing_pass: true,
        kz4_ocel_authenticity_pass: true,
        kz6_allocation_positive_control_pass: true,
        kz6_zero_alloc_decide_pass: true,
        kz7_manifest_tamper_pass: true,
        kz7_bad_pack_rejection_pass: true,
        kz7_runtime_loading_pass: true,
        kz7_rule_metadata_observable_pass: true,
        kz7_no_placeholders_pass: true,
        ccog_regression_pass: true,
        autoinstinct_regression_pass: true,
        master_ocel_to_pack_to_runtime_pass: true,
        overall_pass: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scorecard_all_pass_requires_every_dimension() {
        let mut s = all_true_scorecard();
        s.recompute_overall();
        assert!(s.all_pass());
        assert!(s.overall_pass);
    }

    #[test]
    fn scorecard_fails_if_any_required_dimension_false() {
        // Toggle each required dimension off in turn; all_pass() must reject.
        for setter in toggles() {
            let mut s = all_true_scorecard();
            setter(&mut s);
            s.recompute_overall();
            assert!(
                !s.all_pass(),
                "all_pass() must be false after toggling one dimension off"
            );
            assert!(!s.overall_pass);
        }
    }

    #[test]
    fn scorecard_fails_if_metadata_missing() {
        let mut s = all_true_scorecard();
        s.commit_recorded.clear();
        s.recompute_overall();
        assert!(!s.all_pass());

        let mut s = all_true_scorecard();
        s.toolchain_recorded.clear();
        s.recompute_overall();
        assert!(!s.all_pass());
    }

    #[test]
    fn scorecard_to_json_round_trip() {
        let s = all_true_scorecard();
        let j = s.to_json().expect("serialize");
        assert!(j.contains("\"kz7_runtime_loading_pass\": true"));
        let back: Scorecard = serde_json::from_str(&j).expect("deserialize");
        assert_eq!(s, back);
    }

    #[test]
    fn parse_cargo_test_stdout_recognizes_ok_and_failed() {
        let stdout = "\
running 3 tests
test foo_one ... ok
test foo_two ... FAILED
test foo_three ... ok

failures:
    foo_two

test result: FAILED. 2 passed; 1 failed; 0 ignored
";
        let outcome = parse_cargo_test_stdout("demo", false, stdout);
        assert_eq!(outcome.binary, "demo");
        assert!(!outcome.success);
        assert_eq!(outcome.passing_tests, vec!["foo_one", "foo_three"]);
        assert_eq!(outcome.failing_tests, vec!["foo_two"]);
        assert!(outcome.all_required_pass(&["foo_one"]));
        assert!(!outcome.all_required_pass(&["foo_two"]));
    }

    fn toggles() -> Vec<fn(&mut Scorecard)> {
        vec![
            |s| s.git_clean = false,
            |s| s.kz1_doctrine_drift_pass = false,
            |s| s.kz2_causal_expected_outcomes_pass = false,
            |s| s.kz2_prov_value_absence_load_bearing_pass = false,
            |s| s.kz4_ocel_authenticity_pass = false,
            |s| s.kz6_allocation_positive_control_pass = false,
            |s| s.kz6_zero_alloc_decide_pass = false,
            |s| s.kz7_manifest_tamper_pass = false,
            |s| s.kz7_bad_pack_rejection_pass = false,
            |s| s.kz7_runtime_loading_pass = false,
            |s| s.kz7_rule_metadata_observable_pass = false,
            |s| s.kz7_no_placeholders_pass = false,
            |s| s.ccog_regression_pass = false,
            |s| s.autoinstinct_regression_pass = false,
            |s| s.master_ocel_to_pack_to_runtime_pass = false,
        ]
    }
}
