//! AutoML evaluator: wires `TrialConfig` through the two-pass RL + ensemble
//! feedback loop and returns a `TrialResult`.

use crate::automation::train_with_provenance_and_vote;
use crate::config::AutonomicConfig;
use crate::conformance::bitmask_replay::NetBitmask64;
use crate::ml::automl::{SearchStrategy, TrialConfig, TrialResult};
use crate::ml::pdc_combinator::run_combinator;
use crate::ml::pdc_features::extract_log_features;
use crate::ml::pdc_supervised::{run_supervised, SupervisedPredictions};
use crate::ml::{decision_tree, neural_network};
use crate::models::{EventLog, Ontology};

// ── Evaluator ─────────────────────────────────────────────────────────────────

/// Holds the fixed context for a series of AutoML trials on one log/net pair.
pub struct AutoMLEvaluator<'a> {
    pub log: &'a EventLog,
    pub net: &'a NetBitmask64,
    pub n_target: usize,
    pub log_name: Option<String>,
    pub ontology: Option<&'a Ontology>,
}

impl<'a> AutoMLEvaluator<'a> {
    pub fn new(
        log: &'a EventLog,
        net: &'a NetBitmask64,
        n_target: usize,
        log_name: Option<String>,
        ontology: Option<&'a Ontology>,
    ) -> Self {
        Self {
            log,
            net,
            n_target,
            log_name,
            ontology,
        }
    }

    /// Two-pass evaluation:
    ///
    /// 1. Train RL with `vote=None`, run combinator → `pass1_score`.
    /// 2. Clamp `pass1_score` to `[0,1]` as `ensemble_score`.
    /// 3. Retrain RL with `vote=Some(ensemble_score)` → `pass2_score`.
    pub fn evaluate(&self, trial: &TrialConfig, base_config: &AutonomicConfig) -> TrialResult {
        let cfg = apply_trial(base_config, trial);

        // ── Pass 1 ───────────────────────────────────────────────────────────
        let (net1, _) = train_with_provenance_and_vote(
            self.log,
            &cfg,
            0.5,
            0.01,
            self.ontology,
            Some(trial.seed),
            None,
        );

        let pass1_score = if net1.places.len() <= 64 {
            let bm1 = NetBitmask64::from_petri_net(&net1);
            let r1 = run_combinator(
                self.log,
                &bm1,
                self.n_target,
                self.log_name.as_deref(),
                None,
            );
            r1.first().map(|r| r.score).unwrap_or(0.0)
        } else {
            0.0
        };

        let ensemble_score = pass1_score.clamp(0.0, 1.0) as f32;

        // ── Pass 2 ───────────────────────────────────────────────────────────
        let (net2, _) = train_with_provenance_and_vote(
            self.log,
            &cfg,
            0.5,
            0.01,
            self.ontology,
            Some(trial.seed.wrapping_add(1)),
            Some(ensemble_score),
        );

        let pass2_score = if net2.places.len() <= 64 {
            let bm2 = NetBitmask64::from_petri_net(&net2);
            let r2 = run_combinator(
                self.log,
                &bm2,
                self.n_target,
                self.log_name.as_deref(),
                None,
            );
            r2.first().map(|r| r.score).unwrap_or(0.0)
        } else {
            0.0
        };

        TrialResult {
            trial: *trial,
            pass1_score,
            pass2_score,
            ensemble_score,
            config_hash: trial.hash(),
        }
    }

    /// Drive a full AutoML search: pulls trials from `strategy`, evaluates each
    /// via `self.evaluate()`, and returns the best `TrialResult`.
    ///
    /// `max_results` caps the in-memory accumulator — does NOT cap trials (that
    /// is the strategy's budget). Returns `None` if no trials were run.
    pub fn run_automl(
        &self,
        mut strategy: Box<dyn SearchStrategy>,
        base_config: &AutonomicConfig,
        max_results: usize,
    ) -> Option<TrialResult> {
        let mut results: Vec<TrialResult> = Vec::with_capacity(max_results);
        while let Some(trial) = strategy.next_trial() {
            let result = self.evaluate(&trial, base_config);
            strategy.report(result);
            if results.len() < max_results {
                results.push(result);
            }
        }
        results.iter().copied().max_by(|a, b| {
            a.best_score()
                .partial_cmp(&b.best_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Single-pass evaluation: reuse the provided net without RL retraining.
    ///
    /// Useful for quickly sweeping ensemble/classifier hyperparameters when
    /// the RL model is already fixed.
    /// Single-pass evaluation: skip RL retraining, but ACTUALLY use the trial's
    /// supervised hyperparameters. Returns both a scored `TrialResult` and the
    /// ensemble predictions so the caller can use the actual trial-dependent output
    /// (not a stale constant from run_combinator).
    ///
    /// Anti-lie: the score returned here MUST change when the trial changes,
    /// otherwise the sweep is performing no work.
    pub fn evaluate_ensemble_only(
        &self,
        trial: &TrialConfig,
        _base_config: &AutonomicConfig,
    ) -> TrialResult {
        let (predictions, score) = self.evaluate_ensemble_only_with_preds(trial);

        // Anti-lie: if predictions are empty for a non-empty log, something lied.
        debug_assert_eq!(
            predictions.len(),
            self.log.traces.len(),
            "evaluate_ensemble_only lie: predictions.len({}) != log.traces.len({})",
            predictions.len(),
            self.log.traces.len(),
        );

        TrialResult {
            trial: *trial,
            pass1_score: score,
            pass2_score: score,
            ensemble_score: score.clamp(0.0, 1.0) as f32,
            config_hash: trial.hash(),
        }
    }

    /// Ensemble-only evaluation returning both predictions and score.
    ///
    /// Combines the trial-parameterized supervised classifiers (decision tree +
    /// neural net) with the net's `in_lang` signal via combinatorial ensemble —
    /// so score changes with trial hyperparameters.
    pub fn evaluate_ensemble_only_with_preds(&self, trial: &TrialConfig) -> (Vec<bool>, f64) {
        let (features, in_lang, _fitness) = extract_log_features(self.log, self.net);
        let pseudo_bool: Vec<bool> = in_lang.clone();

        // Parameterized supervised classifiers — score MUST change with trial
        let sup = run_supervised_with_trial(&features, &pseudo_bool, trial);

        // Ensemble the trial-dependent predictions with the net's baseline signal.
        // This is what makes the score trial-sensitive.
        let pool = vec![
            sup.decision_tree.clone(),
            sup.neural_net.clone(),
            in_lang.clone(),
        ];
        let predictions =
            crate::ml::pdc_ensemble::combinatorial_ensemble(&pool, &in_lang, self.n_target);
        let score = crate::ml::pdc_ensemble::score(&predictions, &in_lang, self.n_target);

        (predictions, score)
    }
}

// ── Supervised with trial params ──────────────────────────────────────────────

/// Run supervised classifiers with `tree_depth` and neural-net params from a trial.
///
/// All other classifiers use their default parameters.  Returns a
/// `SupervisedPredictions` with parameterized decision tree and neural net.
pub fn run_supervised_with_trial(
    features: &[Vec<f64>],
    labels: &[bool],
    trial: &TrialConfig,
) -> SupervisedPredictions {
    if features.is_empty() {
        return SupervisedPredictions::default();
    }
    let has_features = features.iter().any(|f| !f.is_empty());
    if !has_features {
        let n = features.len();
        return SupervisedPredictions {
            decision_tree: vec![false; n],
            neural_net: vec![false; n],
            ..SupervisedPredictions::default()
        };
    }

    // Parameterized classifiers from trial
    let decision_tree_preds = decision_tree::classify(features, labels, features, trial.tree_depth);
    let neural_net_preds = neural_network::classify(
        features,
        labels,
        features,
        trial.nn_hidden,
        trial.nn_lr as f64,
        trial.nn_epochs,
    );

    // Remaining classifiers use defaults via run_supervised
    let mut base = run_supervised(features, labels);
    base.decision_tree = decision_tree_preds;
    base.neural_net = neural_net_preds;
    base
}

// ── Config patching ───────────────────────────────────────────────────────────

/// Clone `base` and patch the RL/discovery fields with values from `trial`.
pub fn apply_trial(base: &AutonomicConfig, trial: &TrialConfig) -> AutonomicConfig {
    let mut cfg = base.clone();
    cfg.rl.learning_rate = trial.learning_rate;
    cfg.rl.discount_factor = trial.discount_factor;
    cfg.rl.exploration_rate = trial.exploration_rate;
    cfg.discovery.max_training_epochs = trial.max_epochs;
    cfg.discovery.fitness_stopping_threshold = trial.fitness_threshold as f64;
    cfg
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AutonomicConfig;

    fn base_config() -> AutonomicConfig {
        AutonomicConfig::load("dteam.toml").unwrap_or_default()
    }

    #[test]
    fn test_apply_trial() {
        let base = base_config();
        let trial = TrialConfig {
            learning_rate: 0.05,
            discount_factor: 0.9,
            exploration_rate: 0.1,
            max_epochs: 50,
            fitness_threshold: 0.8,
            ..TrialConfig::default()
        };
        let cfg = apply_trial(&base, &trial);
        assert!((cfg.rl.learning_rate - 0.05).abs() < 1e-6);
        assert!((cfg.rl.discount_factor - 0.9).abs() < 1e-6);
        assert!((cfg.rl.exploration_rate - 0.1).abs() < 1e-6);
        assert_eq!(cfg.discovery.max_training_epochs, 50);
        assert!((cfg.discovery.fitness_stopping_threshold - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_run_supervised_with_trial_empty() {
        let trial = TrialConfig::default();
        let preds = run_supervised_with_trial(&[], &[], &trial);
        assert!(preds.decision_tree.is_empty());
        assert!(preds.neural_net.is_empty());
    }

    #[test]
    fn test_run_supervised_with_trial_small() {
        let features = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]];
        let labels = vec![true, false, true];
        let trial = TrialConfig {
            tree_depth: 2,
            nn_hidden: 4,
            nn_epochs: 5,
            ..TrialConfig::default()
        };
        let preds = run_supervised_with_trial(&features, &labels, &trial);
        assert_eq!(preds.decision_tree.len(), 3);
        assert_eq!(preds.neural_net.len(), 3);
    }

    /// Anti-lie test: different trial hyperparameters MUST produce different
    /// supervised predictions (at least for one of tree/nn). If this ever
    /// fails, evaluate_ensemble_only has regressed to the "stale constant" lie.
    #[test]
    fn test_run_supervised_with_trial_varies_with_params() {
        // Small nonlinear-ish dataset where tree_depth=1 vs 4 should produce
        // different decision trees.
        let features = vec![
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![0.5, 0.5],
            vec![0.2, 0.8],
            vec![0.8, 0.2],
            vec![0.9, 0.9],
        ];
        let labels = vec![false, true, true, false, false, true, true, false];

        let trial_shallow = TrialConfig {
            tree_depth: 1,
            nn_hidden: 2,
            nn_epochs: 10,
            ..TrialConfig::default()
        };
        let trial_deep = TrialConfig {
            tree_depth: 5,
            nn_hidden: 16,
            nn_epochs: 100,
            ..TrialConfig::default()
        };

        let p1 = run_supervised_with_trial(&features, &labels, &trial_shallow);
        let p2 = run_supervised_with_trial(&features, &labels, &trial_deep);

        assert!(
            p1.decision_tree != p2.decision_tree || p1.neural_net != p2.neural_net,
            "LIE DETECTED: trial hyperparameters have zero effect on supervised predictions — sweep is a no-op"
        );
    }
}
