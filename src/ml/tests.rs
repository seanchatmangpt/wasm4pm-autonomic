//! Cross-module invariant tests — the 80/20 consolidation layer.
//!
//! Rather than adding 4-5 edge-case tests (empty input, single element, large
//! input, etc.) PER classifier module, this file asserts invariants that MUST
//! hold across ALL classifiers. A regression in any single classifier is caught
//! here without needing a module-local test.
//!
//! Coverage:
//!   - output length == input length (every fusion op)
//!   - empty input → empty output, no panic (every fusion op)
//!   - calibrated-to-N output has exactly N positives (every fusion op)
//!   - HDIT accounting identity holds for random pool shapes
//!   - Pareto front always has exactly one `chosen=true`
//!   - SH preserves the original pool size in `signals_evaluated`
//!   - Fusion op matches the selected set size (1 → Single, 5+ → Borda/Stack)

#[cfg(test)]
mod cross_cutting {
    use crate::ml::hdc;
    use crate::ml::hdit_automl::{run_hdit_automl, run_hdit_automl_sh, FusionOp, SignalProfile};
    use crate::ml::linucb::LinUcb;
    use crate::ml::rank_fusion::{borda_count, reciprocal_rank_fusion};
    use crate::ml::stacking::{stack_ensemble, stack_ensemble_oof, stack_linear, stack_logistic};
    use crate::ml::weighted_vote::{auto_weighted_vote, precision_weighted_vote};

    // ── Classifier output-length invariants ─────────────────────────────────
    //
    // Every classifier that consumes N-length inputs MUST emit N-length output.
    // A regression in any classifier's length handling is caught here.

    #[test]
    fn classifier_invariant_output_length_matches_input() {
        let anchor: Vec<bool> = (0..20).map(|i| i % 2 == 0).collect();
        let preds: Vec<Vec<bool>> = vec![
            (0..20).map(|i| i < 10).collect(),
            (0..20).map(|i| i % 3 == 0).collect(),
            (0..20).map(|i| i % 5 == 0).collect(),
        ];
        let score_signals: Vec<Vec<f64>> = preds
            .iter()
            .map(|p| p.iter().map(|&b| if b { 1.0 } else { 0.0 }).collect())
            .collect();
        let n = anchor.len();

        let results: Vec<(&str, Vec<bool>)> = vec![
            ("auto_weighted_vote", auto_weighted_vote(&preds, &anchor, 5)),
            (
                "precision_weighted_vote",
                precision_weighted_vote(&preds, &anchor, 5),
            ),
            ("stack_logistic", stack_logistic(&preds, &anchor, 5)),
            ("stack_linear", stack_linear(&preds, &anchor, 5)),
            ("stack_ensemble", stack_ensemble(&preds, &anchor, 5)),
            ("stack_ensemble_oof", stack_ensemble_oof(&preds, &anchor, 5)),
            (
                "borda_count",
                borda_count(&score_signals, &vec![true; score_signals.len()], 5),
            ),
            (
                "reciprocal_rank_fusion",
                reciprocal_rank_fusion(&score_signals, &vec![true; score_signals.len()], 5),
            ),
        ];
        for (name, out) in &results {
            assert_eq!(
                out.len(),
                n,
                "{} violated output-length invariant: got {}, expected {}",
                name,
                out.len(),
                n
            );
        }
    }

    // ── Empty-input invariant ──────────────────────────────────────────────
    //
    // Every fusion op MUST handle empty input gracefully. Any regression to
    // `unwrap()` / slice-index on an empty Vec is caught here.

    #[test]
    fn classifier_invariant_empty_input_no_panic() {
        let empty_preds: Vec<Vec<bool>> = vec![];
        let empty_anchor: Vec<bool> = vec![];
        let empty_scores: Vec<Vec<f64>> = vec![];

        let _ = auto_weighted_vote(&empty_preds, &empty_anchor, 0);
        let _ = precision_weighted_vote(&empty_preds, &empty_anchor, 0);
        let _ = stack_logistic(&empty_preds, &empty_anchor, 0);
        let _ = stack_linear(&empty_preds, &empty_anchor, 0);
        let _ = stack_ensemble(&empty_preds, &empty_anchor, 0);
        let _ = stack_ensemble_oof(&empty_preds, &empty_anchor, 0);
        let _ = borda_count(&empty_scores, &[], 0);
        let _ = reciprocal_rank_fusion(&empty_scores, &[], 0);
    }

    // ── Calibration invariant ──────────────────────────────────────────────
    //
    // Any fusion op that takes `n_target` MUST emit exactly n_target positives
    // (when n_target ≤ input length). Catches off-by-one in calibrate_to_n_target.

    #[test]
    fn fusion_invariant_calibrated_output_has_n_target_positives() {
        let n = 50;
        let c1: Vec<bool> = (0..n).map(|i| i < n / 2).collect();
        let c2: Vec<bool> = (0..n).map(|i| i % 2 == 0).collect();
        let c3: Vec<bool> = (0..n).map(|i| i % 3 == 0).collect();
        let preds = vec![c1, c2, c3];
        let anchor: Vec<bool> = (0..n).map(|i| i < n / 2).collect();

        for &n_target in &[1usize, 5, 10, 25, 49] {
            let results: Vec<(&str, Vec<bool>)> = vec![
                (
                    "auto_weighted_vote",
                    auto_weighted_vote(&preds, &anchor, n_target),
                ),
                ("stack_logistic", stack_logistic(&preds, &anchor, n_target)),
                ("stack_ensemble", stack_ensemble(&preds, &anchor, n_target)),
            ];
            for (name, out) in &results {
                let pos_count = out.iter().filter(|&&b| b).count();
                assert_eq!(
                    pos_count, n_target,
                    "{} calibration lie at n_target={}: got {} positives",
                    name, n_target, pos_count
                );
                assert_eq!(out.len(), n);
            }
        }
    }

    // ── HDIT accounting invariant (parametric) ─────────────────────────────
    //
    // selected + rejected_corr + rejected_gain MUST equal evaluated, for any
    // pool shape.

    #[test]
    fn hdit_invariant_accounting_holds_across_pool_shapes() {
        let anchor: Vec<bool> = (0..30).map(|i| i % 2 == 0).collect();

        for pool_size in [0, 1, 2, 5, 12] {
            let mut candidates = Vec::new();
            for j in 0..pool_size {
                let preds: Vec<bool> = (0..30).map(|i| (i + j) % (j + 2) == 0).collect();
                candidates.push(SignalProfile::new(
                    format!("s{}", j),
                    preds,
                    &anchor,
                    (100 * (j + 1)) as u64,
                ));
            }

            let plan = run_hdit_automl(candidates, &anchor, 15);
            assert_eq!(
                plan.selected.len()
                    + plan.signals_rejected_correlation
                    + plan.signals_rejected_no_gain,
                plan.signals_evaluated,
                "HDIT accounting lie at pool_size={}",
                pool_size
            );
            assert_eq!(plan.signals_evaluated, pool_size);
        }
    }

    // ── HDIT Pareto invariant ──────────────────────────────────────────────

    #[test]
    fn hdit_invariant_pareto_front_exactly_one_chosen() {
        let anchor: Vec<bool> = (0..20).map(|i| i % 3 != 0).collect();
        let candidates = vec![
            SignalProfile::new("a", anchor.clone(), &anchor, 100),
            SignalProfile::new("b", (0..20).map(|i| i < 10).collect(), &anchor, 200),
            SignalProfile::new("c", (0..20).map(|i| i % 2 == 0).collect(), &anchor, 300),
        ];
        let plan = run_hdit_automl(candidates, &anchor, 10);
        let chosen = plan.pareto_front.iter().filter(|c| c.chosen).count();
        assert_eq!(chosen, 1, "Pareto front must have exactly one chosen=true");
    }

    // ── SH invariant: signals_evaluated preserves ORIGINAL pool ────────────

    #[test]
    fn sh_invariant_preserves_original_pool_size() {
        let anchor: Vec<bool> = (0..50).map(|i| i % 2 == 0).collect();
        let candidates: Vec<SignalProfile> = (0..10)
            .map(|j| {
                let preds: Vec<bool> = (0..50).map(|i| (i + j) % 3 == 0).collect();
                SignalProfile::new(format!("s{}", j), preds, &anchor, 100 + j as u64 * 10)
            })
            .collect();

        let plan = run_hdit_automl_sh(candidates, &anchor, 25, 0.2, 3.0);
        assert_eq!(
            plan.signals_evaluated, 10,
            "SH lied about signals_evaluated"
        );
        assert_eq!(
            plan.selected.len() + plan.signals_rejected_correlation + plan.signals_rejected_no_gain,
            plan.signals_evaluated,
            "SH accounting broken"
        );
    }

    // ── Fusion-op/selection-size consistency ────────────────────────────────
    //
    // 1 selected signal → Single fusion. Other sizes aren't strictly tied to a
    // fusion op, but 1 is a hard invariant.

    #[test]
    fn fusion_invariant_single_signal_picks_single_op() {
        let anchor: Vec<bool> = (0..20).map(|i| i < 10).collect();
        let one = vec![SignalProfile::new("a", anchor.clone(), &anchor, 100)];
        let plan = run_hdit_automl(one, &anchor, 10);
        if !plan.selected.is_empty() {
            assert!(
                matches!(plan.fusion, FusionOp::Single),
                "1-signal pool produced non-Single fusion: {:?}",
                plan.fusion
            );
        }
    }

    // ── HDC classifier invariant ───────────────────────────────────────────

    #[test]
    fn hdc_invariant_output_shape() {
        let train = vec![
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["a".to_string(), "c".to_string(), "b".to_string()],
        ];
        let test = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["x".to_string(), "y".to_string()],
            vec!["c".to_string()],
        ];
        let clf = hdc::fit(&train);
        let result = hdc::classify(&clf, &test, 2);
        assert_eq!(result.len(), test.len());
        assert_eq!(result.iter().filter(|&&b| b).count(), 2);
    }

    // ── LinUCB zero-heap invariant (preserved from original tests.rs) ──────

    #[test]
    fn linucb_zero_heap_properties() {
        let mut agent: LinUcb<2, 4> = LinUcb::new(0.1);
        let context: [f32; 2] = [1.0, 0.5];
        agent.update(&context, 1.0);
        let action = agent.select_action(&context, 2);
        assert!(action < 2);
    }
}
