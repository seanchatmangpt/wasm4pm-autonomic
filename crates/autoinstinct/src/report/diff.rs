//! Pure structural diff between two scorecards.
//!
//! No Gemini involvement. The `report diff` CLI verb feeds the diff
//! struct as additional context to a `RegressionDiff` report, but the
//! diff itself is deterministic and audit-stable.

use serde::{Deserialize, Serialize};

use crate::scorecard::Scorecard;

/// Single dimension that changed between two scorecards.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DimensionChange {
    /// Dimension name (matches [`Scorecard::dimension_names`]).
    pub dimension: String,
    /// Previous value.
    pub previous: bool,
    /// Current value.
    pub current: bool,
}

/// Structural diff between two scorecards.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScorecardDiff {
    /// Dimensions that flipped value.
    pub changed: Vec<DimensionChange>,
    /// Previous overall_pass.
    pub previous_overall: bool,
    /// Current overall_pass.
    pub current_overall: bool,
    /// Previous commit.
    pub previous_commit: String,
    /// Current commit.
    pub current_commit: String,
    /// True if the toolchain string changed.
    pub toolchain_changed: bool,
}

/// Compute the structural diff between two scorecards.
#[must_use]
pub fn diff(prev: &Scorecard, curr: &Scorecard) -> ScorecardDiff {
    let mut changed = Vec::new();
    for &name in Scorecard::dimension_names() {
        let p = read_dim(prev, name);
        let c = read_dim(curr, name);
        if p != c {
            changed.push(DimensionChange {
                dimension: name.to_string(),
                previous: p,
                current: c,
            });
        }
    }
    ScorecardDiff {
        changed,
        previous_overall: prev.overall_pass,
        current_overall: curr.overall_pass,
        previous_commit: prev.commit_recorded.clone(),
        current_commit: curr.commit_recorded.clone(),
        toolchain_changed: prev.toolchain_recorded != curr.toolchain_recorded,
    }
}

fn read_dim(s: &Scorecard, name: &str) -> bool {
    match name {
        "git_clean" => s.git_clean,
        "kz1_doctrine_drift_pass" => s.kz1_doctrine_drift_pass,
        "kz2_causal_expected_outcomes_pass" => s.kz2_causal_expected_outcomes_pass,
        "kz2_prov_value_absence_load_bearing_pass" => s.kz2_prov_value_absence_load_bearing_pass,
        "kz4_ocel_authenticity_pass" => s.kz4_ocel_authenticity_pass,
        "kz6_allocation_positive_control_pass" => s.kz6_allocation_positive_control_pass,
        "kz6_zero_alloc_decide_pass" => s.kz6_zero_alloc_decide_pass,
        "kz7_manifest_tamper_pass" => s.kz7_manifest_tamper_pass,
        "kz7_bad_pack_rejection_pass" => s.kz7_bad_pack_rejection_pass,
        "kz7_runtime_loading_pass" => s.kz7_runtime_loading_pass,
        "kz7_rule_metadata_observable_pass" => s.kz7_rule_metadata_observable_pass,
        "kz7_no_placeholders_pass" => s.kz7_no_placeholders_pass,
        "ccog_regression_pass" => s.ccog_regression_pass,
        "autoinstinct_regression_pass" => s.autoinstinct_regression_pass,
        "master_ocel_to_pack_to_runtime_pass" => s.master_ocel_to_pack_to_runtime_pass,
        "kz8_report_authenticity_pass" => s.kz8_report_authenticity_pass,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_is_empty_when_unchanged() {
        let s = crate::scorecard::all_true_scorecard();
        let d = diff(&s, &s);
        assert!(d.changed.is_empty());
        assert!(!d.toolchain_changed);
    }

    #[test]
    fn diff_detects_dimension_change() {
        let prev = crate::scorecard::all_true_scorecard();
        let mut curr = crate::scorecard::all_true_scorecard();
        curr.kz7_runtime_loading_pass = false;
        curr.recompute_overall();
        let d = diff(&prev, &curr);
        assert_eq!(d.changed.len(), 1);
        assert_eq!(d.changed[0].dimension, "kz7_runtime_loading_pass");
        assert!(d.changed[0].previous);
        assert!(!d.changed[0].current);
        assert!(d.previous_overall);
        assert!(!d.current_overall);
    }

    #[test]
    fn diff_detects_toolchain_change() {
        let prev = crate::scorecard::all_true_scorecard();
        let mut curr = prev.clone();
        curr.toolchain_recorded = "rustc 99.0.0".into();
        let d = diff(&prev, &curr);
        assert!(d.toolchain_changed);
    }
}
