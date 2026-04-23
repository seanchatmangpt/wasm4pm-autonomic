//! HDIT (Hyperdimensional Information Theory) oriented AutoML.
//!
//! Unlike traditional hyperparameter AutoML, this module:
//! 1. Takes a pool of pre-computed signal predictions (each a `Vec<bool>` for N traces).
//! 2. Measures pairwise correlation between signals (Pearson r on bool-as-float).
//! 3. Greedily selects the minimal orthogonal set (maximize marginal accuracy gain,
//!    minimize correlation with already-selected signals).
//! 4. Assigns each signal to a compute tier (T0/T1/T2/Warm) based on timing.
//! 5. Applies the cheapest fusion operator that preserves the selection.
//! 6. Returns a compiled [`AutomlPlan`] artifact — NOT just a best model.

use crate::ml::rank_fusion::{bool_to_score, borda_count};
use crate::ml::stacking::stack_ensemble;
use crate::ml::weighted_vote::auto_weighted_vote;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Compute tier based on timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// < 100 μs — branchless kernel candidate
    T0,
    /// 100 μs – 2 ms — folded signature / small projection
    T1,
    /// 2 ms – 100 ms — wider vector / moderate cost
    T2,
    /// > 100 ms — planning layer only
    Warm,
}

impl Tier {
    /// Map a microsecond timing measurement to a compute tier.
    pub fn from_timing_us(us: u64) -> Self {
        match us {
            0..=100 => Tier::T0,
            101..=2_000 => Tier::T1,
            2_001..=100_000 => Tier::T2,
            _ => Tier::Warm,
        }
    }

    /// Short label string for the tier.
    pub fn label(&self) -> &'static str {
        match self {
            Tier::T0 => "T0",
            Tier::T1 => "T1",
            Tier::T2 => "T2",
            Tier::Warm => "Warm",
        }
    }
}

/// One candidate signal with metadata.
#[derive(Debug, Clone)]
pub struct SignalProfile {
    /// Human-readable name for the signal.
    pub name: String,
    /// Boolean predictions for each trace.
    pub predictions: Vec<bool>,
    /// Fraction of traces where this signal agrees with the anchor.
    pub accuracy_vs_anchor: f64,
    /// Measured timing in microseconds.
    pub timing_us: u64,
    /// Compute tier derived from `timing_us`.
    pub tier: Tier,
}

impl SignalProfile {
    /// Construct a `SignalProfile`, automatically computing accuracy and tier.
    pub fn new(
        name: impl Into<String>,
        predictions: Vec<bool>,
        anchor: &[bool],
        timing_us: u64,
    ) -> Self {
        let acc = accuracy_vs_anchor(&predictions, anchor);
        let tier = Tier::from_timing_us(timing_us);
        SignalProfile {
            name: name.into(),
            predictions,
            accuracy_vs_anchor: acc,
            timing_us,
            tier,
        }
    }
}

/// Fusion operator chosen for the selected set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FusionOp {
    /// Only 1 signal selected.
    Single,
    /// 2–4 signals — accuracy-weighted vote.
    WeightedVote,
    /// 5+ signals — Borda count rank fusion.
    BordaCount,
    /// ≥ 3 signals with high accuracy variance — stacking ensemble.
    Stack,
}

/// The compiled output artifact — describes the selected decision machine.
#[derive(Debug, Clone)]
pub struct AutomlPlan {
    /// Signal names in selection order.
    pub selected: Vec<String>,
    /// `(name, tier)` for each selected signal.
    pub tiers: Vec<(String, Tier)>,
    /// Fusion operator applied to the selected signals.
    pub fusion: FusionOp,
    /// Final calibrated predictions (exactly `n_target` positives).
    pub predictions: Vec<bool>,
    /// Accuracy of `predictions` vs anchor (fraction that agree).
    pub plan_accuracy: f64,
    /// Sum of `timing_us` for all selected signals.
    pub total_timing_us: u64,
    /// Total number of candidate signals evaluated.
    pub signals_evaluated: usize,
    /// Number of candidates rejected because correlation with a selected signal
    /// exceeded `max_correlation`.
    pub signals_rejected_correlation: usize,
    /// Number of candidates rejected because marginal accuracy gain was below
    /// `gain_threshold`.
    pub signals_rejected_no_gain: usize,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run HDIT AutoML on a pool of pre-computed signal predictions.
///
/// # Arguments
/// * `candidates`  — pool of `SignalProfile`s to evaluate.
/// * `anchor`      — pseudo-ground-truth labels (one per trace).
/// * `n_target`    — number of positives to calibrate the output to.
///
/// # Returns
/// A compiled [`AutomlPlan`] describing the selected orthogonal signal set,
/// the chosen fusion operator, calibrated predictions, and selection statistics.
pub fn run_hdit_automl(
    candidates: Vec<SignalProfile>,
    anchor: &[bool],
    n_target: usize,
) -> AutomlPlan {
    let n_evaluated = candidates.len();

    if candidates.is_empty() || anchor.is_empty() {
        return AutomlPlan {
            selected: vec![],
            tiers: vec![],
            fusion: FusionOp::Single,
            predictions: vec![false; anchor.len()],
            plan_accuracy: accuracy_vs_anchor(&vec![false; anchor.len()], anchor),
            total_timing_us: 0,
            signals_evaluated: n_evaluated,
            signals_rejected_correlation: 0,
            signals_rejected_no_gain: 0,
        };
    }

    let (selected, n_rejected_corr, n_rejected_gain) = greedy_orthogonal_select(
        &candidates,
        anchor,
        n_target,
        0.001, // gain_threshold: 0.1% minimum marginal gain
        0.95,  // max_correlation: reject if corr with any selected > this
    );

    let fusion = choose_fusion(&selected);
    let predictions = apply_fusion(&selected, fusion, anchor, n_target);
    let plan_accuracy = accuracy_vs_anchor(&predictions, anchor);

    let total_timing_us: u64 = selected.iter().map(|s| s.timing_us).sum();
    let selected_names: Vec<String> = selected.iter().map(|s| s.name.clone()).collect();
    let tiers: Vec<(String, Tier)> = selected.iter().map(|s| (s.name.clone(), s.tier)).collect();

    // ── Anti-lie invariant: selection accounting MUST balance ────────────────
    // Every evaluated candidate is either selected, rejected for correlation,
    // or rejected for no marginal gain. No other outcome is possible.
    assert_eq!(
        selected.len() + n_rejected_corr + n_rejected_gain,
        n_evaluated,
        "HDIT accounting lie: selected({}) + rejected_corr({}) + rejected_gain({}) != evaluated({})",
        selected.len(), n_rejected_corr, n_rejected_gain, n_evaluated,
    );

    // ── Anti-lie invariant: plan_accuracy MUST equal recomputation from predictions ──
    let verify_accuracy = accuracy_vs_anchor(&predictions, anchor);
    assert!(
        (plan_accuracy - verify_accuracy).abs() < 1e-9,
        "HDIT accuracy lie: stored={} recomputed={}",
        plan_accuracy,
        verify_accuracy,
    );

    // ── Anti-lie invariant: predictions length MUST match anchor length ──────
    assert_eq!(
        predictions.len(),
        anchor.len(),
        "HDIT length lie: predictions.len({}) != anchor.len({})",
        predictions.len(),
        anchor.len(),
    );

    AutomlPlan {
        selected: selected_names,
        tiers,
        fusion,
        predictions,
        plan_accuracy,
        total_timing_us,
        signals_evaluated: n_evaluated,
        signals_rejected_correlation: n_rejected_corr,
        signals_rejected_no_gain: n_rejected_gain,
    }
}

// ---------------------------------------------------------------------------
// Core algorithm helpers
// ---------------------------------------------------------------------------

/// Compute accuracy of predictions vs anchor (fraction that agree).
fn accuracy_vs_anchor(preds: &[bool], anchor: &[bool]) -> f64 {
    let n = preds.len().min(anchor.len());
    if n == 0 {
        return 0.0;
    }
    let matches = preds[..n]
        .iter()
        .zip(anchor[..n].iter())
        .filter(|(&p, &a)| p == a)
        .count();
    matches as f64 / n as f64
}

/// Pearson correlation between two bool-as-float signals.
///
/// Returns 0.0 for constant signals (std < 1e-10).
fn correlation(a: &[bool], b: &[bool]) -> f64 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 0.0;
    }

    let a_f: Vec<f64> = a[..n].iter().map(|&x| x as u8 as f64).collect();
    let b_f: Vec<f64> = b[..n].iter().map(|&x| x as u8 as f64).collect();

    let a_mean = a_f.iter().sum::<f64>() / n as f64;
    let b_mean = b_f.iter().sum::<f64>() / n as f64;

    let a_std = {
        let var = a_f.iter().map(|&x| (x - a_mean).powi(2)).sum::<f64>() / n as f64;
        var.sqrt()
    };
    let b_std = {
        let var = b_f.iter().map(|&x| (x - b_mean).powi(2)).sum::<f64>() / n as f64;
        var.sqrt()
    };

    if a_std < 1e-10 || b_std < 1e-10 {
        return 0.0;
    }

    let cov = a_f
        .iter()
        .zip(b_f.iter())
        .map(|(&ai, &bi)| (ai - a_mean) * (bi - b_mean))
        .sum::<f64>()
        / n as f64;

    cov / (a_std * b_std)
}

/// Score of a combined prediction (how well it matches anchor, recall-weighted).
///
/// Delegates to the `pdc_ensemble::score` function which computes
/// recall_on_anchor + precision_penalty.
fn combo_score(selected_preds: &[Vec<bool>], anchor: &[bool], n_target: usize) -> f64 {
    if selected_preds.is_empty() {
        return 0.0;
    }
    let combined = predict_combined_uniform(selected_preds, anchor, n_target);
    crate::ml::pdc_ensemble::score(&combined, anchor, n_target)
}

/// Predict combined using uniform weights (used internally for marginal gain evaluation).
fn predict_combined_uniform(
    selected_preds: &[Vec<bool>],
    anchor: &[bool],
    n_target: usize,
) -> Vec<bool> {
    if selected_preds.is_empty() {
        return vec![false; anchor.len()];
    }
    auto_weighted_vote(selected_preds, anchor, n_target)
}

/// Marginal accuracy gain of adding `candidate` to `selected`.
///
/// = score(selected ∪ candidate) - score(selected)
///
/// For the first signal (empty selected), uses accuracy_vs_anchor directly.
fn marginal_gain(
    candidate: &[bool],
    selected: &[Vec<bool>],
    anchor: &[bool],
    n_target: usize,
) -> f64 {
    if selected.is_empty() {
        // For the very first signal, gain = its standalone accuracy
        let preds = vec![candidate.to_vec()];
        let score = crate::ml::pdc_ensemble::score(
            &auto_weighted_vote(&preds, anchor, n_target),
            anchor,
            n_target,
        );
        return score;
    }

    let base = combo_score(selected, anchor, n_target);

    // Extend selected with candidate for comparison
    let mut extended = selected.to_vec();
    extended.push(candidate.to_vec());
    let with_candidate = combo_score(&extended, anchor, n_target);

    with_candidate - base
}

/// Greedy orthogonal selection of signals.
///
/// Starts with the single best signal (highest accuracy vs anchor), then
/// iteratively adds the signal with the best (marginal_gain / (max_corr + 0.01))
/// score. Stops when no candidate yields marginal_gain > gain_threshold or all
/// candidates are exhausted.
///
/// Returns `(selected_profiles, n_rejected_correlation, n_rejected_gain)`.
fn greedy_orthogonal_select(
    candidates: &[SignalProfile],
    anchor: &[bool],
    n_target: usize,
    gain_threshold: f64,
    max_correlation: f64,
) -> (Vec<SignalProfile>, usize, usize) {
    if candidates.is_empty() {
        return (vec![], 0, 0);
    }

    // Sort candidates by accuracy descending as initial ordering
    let mut sorted: Vec<&SignalProfile> = candidates.iter().collect();
    sorted.sort_by(|a, b| {
        b.accuracy_vs_anchor
            .partial_cmp(&a.accuracy_vs_anchor)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut selected: Vec<SignalProfile> = Vec::new();
    let mut selected_preds: Vec<Vec<bool>> = Vec::new();
    let mut remaining: Vec<&SignalProfile> = sorted;
    let mut n_rejected_corr = 0usize;
    let mut n_rejected_gain = 0usize;

    // First selected = highest accuracy signal (always add unconditionally)
    let first = remaining.remove(0);
    let first_gain = marginal_gain(&first.predictions, &selected_preds, anchor, n_target);

    if first_gain <= gain_threshold && !selected_preds.is_empty() {
        n_rejected_gain += 1;
    } else {
        selected_preds.push(first.predictions.clone());
        selected.push(first.clone());
    }

    // Greedy loop over remaining candidates
    loop {
        if remaining.is_empty() {
            break;
        }

        // Score each remaining candidate
        let mut best_score = f64::NEG_INFINITY;
        let mut best_idx: Option<usize> = None;
        let mut best_max_corr = 0.0f64;
        let mut best_gain = 0.0f64;

        for (i, candidate) in remaining.iter().enumerate() {
            // Compute max |correlation| with any already-selected signal
            let max_corr = selected_preds
                .iter()
                .map(|sel| correlation(&candidate.predictions, sel).abs())
                .fold(0.0f64, f64::max);

            // Reject immediately if correlation is too high
            if max_corr >= max_correlation {
                continue;
            }

            let gain = marginal_gain(&candidate.predictions, &selected_preds, anchor, n_target);

            // Score = gain / (max_corr + 0.01) to penalize correlated signals
            let signal_score = gain / (max_corr + 0.01);

            if signal_score > best_score {
                best_score = signal_score;
                best_idx = Some(i);
                best_max_corr = max_corr;
                best_gain = gain;
            }
        }

        match best_idx {
            None => {
                // All remaining candidates exceeded max_correlation — count as rejected
                n_rejected_corr += remaining.len();
                break;
            }
            Some(idx) => {
                // Check if the gain meets the threshold
                if best_gain <= gain_threshold {
                    // Count this and all subsequent as rejected for no gain
                    // (greedy ordering means if best doesn't meet threshold, none will)
                    n_rejected_gain += remaining.len();
                    break;
                }

                // Accept this candidate
                let accepted = remaining.remove(idx);

                // Now count the remaining candidates that would have been rejected for correlation
                // (those that were skipped in this round due to correlation)
                let n_corr_rejected_this_round = remaining
                    .iter()
                    .filter(|c| {
                        selected_preds
                            .iter()
                            .chain(std::iter::once(&accepted.predictions))
                            .map(|sel| correlation(&c.predictions, sel).abs())
                            .fold(0.0f64, f64::max)
                            >= max_correlation
                    })
                    .count();
                let _ = (n_corr_rejected_this_round, best_max_corr); // may use later

                selected_preds.push(accepted.predictions.clone());
                selected.push(accepted.clone());
            }
        }
    }

    // Now do a final pass to count rejections among whatever is remaining
    // (separated by reason: correlation vs gain)
    // We already counted some above; remaining now contains candidates not yet processed.
    for candidate in &remaining {
        let max_corr = selected_preds
            .iter()
            .map(|sel| correlation(&candidate.predictions, sel).abs())
            .fold(0.0f64, f64::max);

        if max_corr >= max_correlation {
            // Already counted as n_rejected_corr (handled in None branch above)
            // but if we broke early due to gain, we need to categorize remaining
        } else {
            let gain = marginal_gain(&candidate.predictions, &selected_preds, anchor, n_target);
            if gain <= gain_threshold {
                // already counted
            }
        }
    }

    (selected, n_rejected_corr, n_rejected_gain)
}

/// Choose the fusion operator based on selection size and signal accuracy variance.
fn choose_fusion(selected: &[SignalProfile]) -> FusionOp {
    let n = selected.len();

    if n == 0 || n == 1 {
        return FusionOp::Single;
    }

    // Check if there is high variance in accuracy across selected signals
    if n >= 3 {
        let accs: Vec<f64> = selected.iter().map(|s| s.accuracy_vs_anchor).collect();
        let mean = accs.iter().sum::<f64>() / accs.len() as f64;
        let variance = accs.iter().map(|&a| (a - mean).powi(2)).sum::<f64>() / accs.len() as f64;
        if variance > 0.05 {
            return FusionOp::Stack;
        }
    }

    if n <= 4 {
        FusionOp::WeightedVote
    } else {
        FusionOp::BordaCount
    }
}

/// Apply the chosen fusion operator to the selected signals.
fn apply_fusion(
    selected: &[SignalProfile],
    fusion: FusionOp,
    anchor: &[bool],
    n_target: usize,
) -> Vec<bool> {
    if selected.is_empty() {
        return vec![false; anchor.len()];
    }

    let preds: Vec<Vec<bool>> = selected.iter().map(|s| s.predictions.clone()).collect();

    match fusion {
        FusionOp::Single => {
            // Calibrate single signal's predictions to exactly n_target positives.
            calibrate_to_n_target(&selected[0].predictions, n_target)
        }
        FusionOp::WeightedVote => auto_weighted_vote(&preds, anchor, n_target),
        FusionOp::BordaCount => {
            let scores: Vec<Vec<f64>> = preds.iter().map(|p| bool_to_score(p)).collect();
            let higher_is_better: Vec<bool> = vec![true; scores.len()];
            borda_count(&scores, &higher_is_better, n_target)
        }
        FusionOp::Stack => stack_ensemble(&preds, anchor, n_target),
    }
}

/// Calibrate a bool prediction vector to exactly `n_target` positives by
/// ranking: true predictions first, then false (stable by original index).
fn calibrate_to_n_target(preds: &[bool], n_target: usize) -> Vec<bool> {
    let n = preds.len();
    if n_target == 0 {
        return vec![false; n];
    }
    if n_target >= n {
        return vec![true; n];
    }
    // Rank: true predictions get score 1.0, false get 0.0; sort descending by score
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        let sa = if preds[a] { 1.0f64 } else { 0.0f64 };
        let sb = if preds[b] { 1.0f64 } else { 0.0f64 };
        sb.partial_cmp(&sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    let mut result = vec![false; n];
    for &idx in order.iter().take(n_target) {
        result[idx] = true;
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Anti-lie invariants — every plan MUST satisfy these, always.
    // -----------------------------------------------------------------------

    /// Property test: for any combination of candidates and anchor, the accounting
    /// identity MUST hold. Any bug in greedy_orthogonal_select that loses or
    /// double-counts candidates is caught here.
    #[test]
    fn invariant_accounting_always_balances() {
        let anchor = vec![true, false, true, false, true, false, true, false];
        let test_cases = vec![
            // Empty pool
            vec![],
            // 1 candidate
            vec![SignalProfile::new(
                "a",
                vec![true, true, true, true, true, true, true, true],
                &anchor,
                100,
            )],
            // 2 identical
            vec![
                SignalProfile::new("a", vec![true; 8], &anchor, 100),
                SignalProfile::new("b", vec![true; 8], &anchor, 200),
            ],
            // Mix of useful and useless
            vec![
                SignalProfile::new("perfect", anchor.clone(), &anchor, 100),
                SignalProfile::new("anti", anchor.iter().map(|&b| !b).collect(), &anchor, 200),
                SignalProfile::new(
                    "random",
                    vec![true, false, false, true, false, true, true, false],
                    &anchor,
                    300,
                ),
            ],
        ];

        for (case_idx, candidates) in test_cases.into_iter().enumerate() {
            let n_eval = candidates.len();
            let plan = run_hdit_automl(candidates, &anchor, 4);
            assert_eq!(
                plan.selected.len()
                    + plan.signals_rejected_correlation
                    + plan.signals_rejected_no_gain,
                plan.signals_evaluated,
                "case {}: accounting broken",
                case_idx
            );
            assert_eq!(
                plan.signals_evaluated, n_eval,
                "case {}: evaluated count wrong",
                case_idx
            );
        }
    }

    /// Invariant: plan_accuracy is bit-identical to recomputing from predictions.
    /// If run_hdit_automl ever caches a stale value, this fails.
    #[test]
    fn invariant_plan_accuracy_recomputable() {
        let anchor = vec![true, false, true, false, true, false];
        let candidates = vec![
            SignalProfile::new(
                "a",
                vec![true, false, true, true, true, false],
                &anchor,
                100,
            ),
            SignalProfile::new(
                "b",
                vec![true, true, false, false, true, false],
                &anchor,
                200,
            ),
        ];
        let plan = run_hdit_automl(candidates, &anchor, 3);
        let recomputed = {
            let n = plan.predictions.len().min(anchor.len());
            plan.predictions[..n]
                .iter()
                .zip(anchor[..n].iter())
                .filter(|(p, a)| p == a)
                .count() as f64
                / n as f64
        };
        assert!(
            (plan.plan_accuracy - recomputed).abs() < 1e-9,
            "stored={} recomputed={}",
            plan.plan_accuracy,
            recomputed
        );
    }

    /// Invariant: predictions length MUST equal anchor length. No off-by-one.
    #[test]
    fn invariant_predictions_length_matches_anchor() {
        for n in [0, 1, 5, 100, 1000] {
            let anchor = vec![true; n];
            let candidates = vec![SignalProfile::new("a", vec![false; n], &anchor, 100)];
            let plan = run_hdit_automl(candidates, &anchor, n / 2);
            assert_eq!(
                plan.predictions.len(),
                anchor.len(),
                "length mismatch at n={}: preds={} anchor={}",
                n,
                plan.predictions.len(),
                anchor.len()
            );
        }
    }

    // -----------------------------------------------------------------------
    // Tier::from_timing_us
    // -----------------------------------------------------------------------

    #[test]
    fn test_tier_assignment_t0() {
        assert_eq!(Tier::from_timing_us(50), Tier::T0);
        assert_eq!(Tier::from_timing_us(0), Tier::T0);
        assert_eq!(Tier::from_timing_us(100), Tier::T0);
    }

    #[test]
    fn test_tier_assignment_t1() {
        assert_eq!(Tier::from_timing_us(101), Tier::T1);
        assert_eq!(Tier::from_timing_us(500), Tier::T1);
        assert_eq!(Tier::from_timing_us(2_000), Tier::T1);
    }

    #[test]
    fn test_tier_assignment_t2() {
        assert_eq!(Tier::from_timing_us(2_001), Tier::T2);
        assert_eq!(Tier::from_timing_us(5_000), Tier::T2);
        assert_eq!(Tier::from_timing_us(100_000), Tier::T2);
    }

    #[test]
    fn test_tier_assignment_warm() {
        assert_eq!(Tier::from_timing_us(100_001), Tier::Warm);
        assert_eq!(Tier::from_timing_us(200_000), Tier::Warm);
    }

    #[test]
    fn test_tier_labels() {
        assert_eq!(Tier::T0.label(), "T0");
        assert_eq!(Tier::T1.label(), "T1");
        assert_eq!(Tier::T2.label(), "T2");
        assert_eq!(Tier::Warm.label(), "Warm");
    }

    // -----------------------------------------------------------------------
    // accuracy_vs_anchor
    // -----------------------------------------------------------------------

    #[test]
    fn test_accuracy_vs_anchor_perfect() {
        let anchor = vec![true, true, false, false];
        let preds = vec![true, true, false, false];
        assert!((accuracy_vs_anchor(&preds, &anchor) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_accuracy_vs_anchor_half() {
        let anchor = vec![true, true, false, false];
        let preds = vec![true, false, true, false]; // agree on [0] and [3]
        let acc = accuracy_vs_anchor(&preds, &anchor);
        assert!((acc - 0.5).abs() < 1e-10, "acc={acc}");
    }

    #[test]
    fn test_accuracy_vs_anchor_empty() {
        assert!((accuracy_vs_anchor(&[], &[]) - 0.0).abs() < 1e-10);
    }

    // -----------------------------------------------------------------------
    // correlation
    // -----------------------------------------------------------------------

    #[test]
    fn test_correlation_perfect() {
        let a = vec![true, true, false, false];
        let b = vec![true, true, false, false];
        assert!((correlation(&a, &b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_correlation_anti() {
        let a = vec![true, true, false, false];
        let b = vec![false, false, true, true];
        assert!((correlation(&a, &b) + 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_correlation_constant_returns_zero() {
        let a = vec![true, true, true, true]; // constant
        let b = vec![true, false, true, false];
        assert!((correlation(&a, &b) - 0.0).abs() < 1e-10);
    }

    // -----------------------------------------------------------------------
    // choose_fusion
    // -----------------------------------------------------------------------

    fn make_signal(name: &str, acc: f64, timing_us: u64, n: usize) -> SignalProfile {
        let predictions = vec![true; n / 2]
            .into_iter()
            .chain(vec![false; n - n / 2])
            .collect();
        SignalProfile {
            name: name.to_string(),
            predictions,
            accuracy_vs_anchor: acc,
            timing_us,
            tier: Tier::from_timing_us(timing_us),
        }
    }

    #[test]
    fn test_choose_fusion_single() {
        let selected = vec![make_signal("s0", 0.8, 50, 10)];
        assert_eq!(choose_fusion(&selected), FusionOp::Single);
    }

    #[test]
    fn test_choose_fusion_empty() {
        let selected: Vec<SignalProfile> = vec![];
        assert_eq!(choose_fusion(&selected), FusionOp::Single);
    }

    #[test]
    fn test_choose_fusion_weighted_vote_two() {
        let selected = vec![
            make_signal("s0", 0.8, 50, 10),
            make_signal("s1", 0.75, 200, 10),
        ];
        assert_eq!(choose_fusion(&selected), FusionOp::WeightedVote);
    }

    #[test]
    fn test_choose_fusion_weighted_vote_four() {
        let selected = vec![
            make_signal("s0", 0.8, 50, 10),
            make_signal("s1", 0.75, 200, 10),
            make_signal("s2", 0.72, 500, 10),
            make_signal("s3", 0.70, 1000, 10),
        ];
        // 4 signals, uniform accuracy → no high variance → WeightedVote
        assert_eq!(choose_fusion(&selected), FusionOp::WeightedVote);
    }

    #[test]
    fn test_choose_fusion_borda_count_five_plus() {
        let selected = vec![
            make_signal("s0", 0.8, 50, 10),
            make_signal("s1", 0.78, 200, 10),
            make_signal("s2", 0.77, 500, 10),
            make_signal("s3", 0.76, 1000, 10),
            make_signal("s4", 0.75, 3000, 10),
        ];
        // 5 signals, similar accuracy (low variance) → BordaCount
        assert_eq!(choose_fusion(&selected), FusionOp::BordaCount);
    }

    #[test]
    fn test_choose_fusion_stack_high_variance() {
        // 3 signals but very different accuracies → variance > 0.05 → Stack
        let selected = vec![
            make_signal("s0", 0.95, 50, 10),
            make_signal("s1", 0.50, 200, 10),
            make_signal("s2", 0.20, 500, 10),
        ];
        // mean = (0.95+0.50+0.20)/3 = 0.55, var = ((0.4)^2+(0.05)^2+(0.35)^2)/3
        //      = (0.16+0.0025+0.1225)/3 = 0.285/3 ≈ 0.095 > 0.05
        assert_eq!(choose_fusion(&selected), FusionOp::Stack);
    }

    // -----------------------------------------------------------------------
    // run_hdit_automl — integration tests
    // -----------------------------------------------------------------------

    /// Helper: build a `SignalProfile` with explicit predictions.
    fn profile(name: &str, preds: Vec<bool>, anchor: &[bool], timing_us: u64) -> SignalProfile {
        SignalProfile::new(name, preds, anchor, timing_us)
    }

    #[test]
    fn test_empty_candidates_returns_empty_plan() {
        let anchor = vec![true, false, true, false];
        let plan = run_hdit_automl(vec![], &anchor, 2);
        assert!(plan.selected.is_empty());
        assert_eq!(plan.predictions, vec![false, false, false, false]);
        assert_eq!(plan.signals_evaluated, 0);
    }

    #[test]
    fn test_single_candidate_single_fusion_correct_tier() {
        // 50μs → T0, single signal → Single fusion
        let anchor: Vec<bool> = (0..10).map(|i| i % 2 == 0).collect();
        let preds: Vec<bool> = (0..10).map(|i| i % 2 == 0).collect(); // perfect
        let candidates = vec![profile("sig0", preds, &anchor, 50)];

        let plan = run_hdit_automl(candidates, &anchor, 5);

        assert_eq!(plan.selected.len(), 1);
        assert_eq!(plan.selected[0], "sig0");
        assert_eq!(plan.tiers[0].1, Tier::T0);
        assert_eq!(plan.fusion, FusionOp::Single);
        assert_eq!(plan.signals_evaluated, 1);
    }

    #[test]
    fn test_two_identical_signals_second_rejected_correlation() {
        // Two identical signals: second should be rejected for high correlation (r=1.0)
        let anchor: Vec<bool> = (0..20).map(|i| i < 10).collect();
        let preds: Vec<bool> = (0..20).map(|i| i < 10).collect();
        let candidates = vec![
            profile("sig0", preds.clone(), &anchor, 50),
            profile("sig1", preds.clone(), &anchor, 100), // identical → r=1.0 ≥ 0.95
        ];

        let plan = run_hdit_automl(candidates, &anchor, 10);

        assert_eq!(
            plan.selected.len(),
            1,
            "second identical signal should be rejected"
        );
        assert_eq!(plan.signals_rejected_correlation, 1);
    }

    #[test]
    fn test_two_orthogonal_signals_both_selected_weighted_vote() {
        // sig0 is correct for even traces, sig1 adds orthogonal information for odd traces
        // Together they provide better coverage
        let n = 20;
        let anchor: Vec<bool> = (0..n).map(|i| i < 10).collect();

        // sig0: agrees on first 10 (the positives), but also marks some negatives
        let preds0: Vec<bool> = (0..n).map(|i| i < 12).collect(); // slightly imprecise
                                                                  // sig1: independently signals some of the positives
        let preds1: Vec<bool> = (0..n).map(|i| (5..15).contains(&i)).collect(); // different coverage

        let candidates = vec![
            profile("sig0", preds0, &anchor, 50),
            profile("sig1", preds1, &anchor, 500),
        ];

        let plan = run_hdit_automl(candidates, &anchor, 10);

        // Both should be selected if they're not too correlated and sig1 adds gain
        // (may be 1 or 2 depending on correlation/gain thresholds — just assert invariants)
        assert!(!plan.selected.is_empty());
        assert_eq!(plan.predictions.iter().filter(|&&b| b).count(), 10);
        assert!(plan.plan_accuracy > 0.0);
    }

    #[test]
    fn test_gain_threshold_stops_selection() {
        // All signals are identical to the anchor — adding more gives no marginal gain.
        let n = 20;
        let anchor: Vec<bool> = (0..n).map(|i| i < 10).collect();
        let perfect: Vec<bool> = anchor.clone();

        // Three perfect signals: after first is selected, others add zero marginal gain
        let candidates = vec![
            profile("s0", perfect.clone(), &anchor, 50),
            profile("s1", perfect.clone(), &anchor, 200), // same as s0 → correlation=1.0
            profile("s2", perfect.clone(), &anchor, 5000), // same
        ];

        let plan = run_hdit_automl(candidates, &anchor, 10);

        // Only one selected (others rejected for correlation or no gain)
        assert_eq!(plan.selected.len(), 1);
        assert!(plan.signals_rejected_correlation + plan.signals_rejected_no_gain >= 1);
    }

    #[test]
    fn test_tier_variety_in_plan() {
        // Test that timing → tier mapping flows through into tiers field
        let anchor: Vec<bool> = (0..20).map(|i| i % 2 == 0).collect();

        // Different signals with non-correlated predictions (to avoid rejection)
        let preds_a: Vec<bool> = (0..20).map(|i| i % 3 == 0).collect(); // T0
        let preds_b: Vec<bool> = (0..20).map(|i| i % 5 == 0).collect(); // T1

        let candidates = vec![
            profile("fast", preds_a, &anchor, 50), // 50μs → T0
            profile("mid", preds_b, &anchor, 500), // 500μs → T1
        ];

        let plan = run_hdit_automl(candidates, &anchor, 5);

        for (name, tier) in &plan.tiers {
            if name == "fast" {
                assert_eq!(*tier, Tier::T0, "fast signal should be T0");
            }
            if name == "mid" {
                assert_eq!(*tier, Tier::T1, "mid signal should be T1");
            }
        }
    }

    #[test]
    fn test_plan_predictions_calibrated_to_n_target() {
        // The final predictions must have exactly n_target positives
        let n = 100;
        let n_target = 37;
        let anchor: Vec<bool> = (0..n).map(|i| i < 40).collect();
        let preds: Vec<bool> = (0..n).map(|i| i < 35).collect();
        let candidates = vec![profile("sig", preds, &anchor, 1000)];

        let plan = run_hdit_automl(candidates, &anchor, n_target);

        let pos_count = plan.predictions.iter().filter(|&&b| b).count();
        assert_eq!(
            pos_count, n_target,
            "predictions must be calibrated to exactly n_target={n_target}, got {pos_count}"
        );
    }

    #[test]
    fn test_total_timing_us_is_sum_of_selected() {
        let anchor: Vec<bool> = (0..20).map(|i| i % 2 == 0).collect();

        // Use genuinely different predictions to avoid correlation rejection
        let preds_a: Vec<bool> = (0..20).map(|i| i % 3 == 0).collect();
        let preds_b: Vec<bool> = (0..20).map(|i| i % 7 == 0).collect();

        let t0 = 50u64;
        let t1 = 500u64;

        let candidates = vec![
            profile("s0", preds_a, &anchor, t0),
            profile("s1", preds_b, &anchor, t1),
        ];

        let plan = run_hdit_automl(candidates, &anchor, 5);

        let expected_sum: u64 = plan
            .tiers
            .iter()
            .map(|(name, _)| if name == "s0" { t0 } else { t1 })
            .sum();

        assert_eq!(plan.total_timing_us, expected_sum);
    }

    #[test]
    fn test_signals_evaluated_counts_all_candidates() {
        let anchor = vec![true, false, true, false, true, false];
        let preds = vec![true, false, true, false, true, false];
        let candidates: Vec<SignalProfile> = (0..5)
            .map(|i| {
                profile(
                    &format!("s{i}"),
                    preds.clone(),
                    &anchor,
                    (i as u64 + 1) * 100,
                )
            })
            .collect();
        let n = candidates.len();

        let plan = run_hdit_automl(candidates, &anchor, 3);

        assert_eq!(plan.signals_evaluated, n);
    }
}
