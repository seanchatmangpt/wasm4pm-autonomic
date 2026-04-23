//! AutoML hyperparameter search for the PDC 2025 RL + ensemble pipeline.
//!
//! Provides two search strategies (exhaustive `GridSearch` and budget-capped
//! `RandomSearch`) and an `AutoMLRun` driver that evaluates each `TrialConfig`
//! via a caller-supplied function pointer.

use crate::utils::dense_kernel::fnv1a_64;

// ── Trial types ───────────────────────────────────────────────────────────────

/// All hyperparameters that the AutoML loop can vary in a single trial.
///
/// `Copy` is required — no heap allocation on the hot evaluation path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrialConfig {
    /// RL learning rate.
    pub learning_rate: f32,
    /// RL discount factor.
    pub discount_factor: f32,
    /// RL epsilon-greedy exploration rate.
    pub exploration_rate: f32,
    /// Maximum RL training epochs before early-stop.
    pub max_epochs: usize,
    /// Fitness early-stop threshold (maps to `DiscoveryConfig.fitness_stopping_threshold`).
    pub fitness_threshold: f32,
    /// Top-k classifiers used in the combinatorial search.
    pub classifier_k: usize,
    /// Decision-tree / stump max depth.
    pub tree_depth: usize,
    /// Neural-net hidden layer size.
    pub nn_hidden: usize,
    /// Neural-net learning rate.
    pub nn_lr: f32,
    /// Neural-net training epochs.
    pub nn_epochs: usize,
    /// Seed for deterministic RL training and random search.
    pub seed: u64,
}

impl Default for TrialConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.08,
            discount_factor: 0.95,
            exploration_rate: 0.2,
            max_epochs: 100,
            fitness_threshold: 0.9,
            classifier_k: 19,
            tree_depth: 3,
            nn_hidden: 4,
            nn_lr: 0.01,
            nn_epochs: 200,
            seed: 42,
        }
    }
}

impl TrialConfig {
    /// FNV-1a hash over the bit representation of all fields.
    pub fn hash(&self) -> u64 {
        let mut buf = Vec::with_capacity(64);
        buf.extend_from_slice(&self.learning_rate.to_bits().to_ne_bytes());
        buf.extend_from_slice(&self.discount_factor.to_bits().to_ne_bytes());
        buf.extend_from_slice(&self.exploration_rate.to_bits().to_ne_bytes());
        buf.extend_from_slice(&(self.max_epochs as u64).to_ne_bytes());
        buf.extend_from_slice(&self.fitness_threshold.to_bits().to_ne_bytes());
        buf.extend_from_slice(&(self.classifier_k as u64).to_ne_bytes());
        buf.extend_from_slice(&(self.tree_depth as u64).to_ne_bytes());
        buf.extend_from_slice(&(self.nn_hidden as u64).to_ne_bytes());
        buf.extend_from_slice(&self.nn_lr.to_bits().to_ne_bytes());
        buf.extend_from_slice(&(self.nn_epochs as u64).to_ne_bytes());
        buf.extend_from_slice(&self.seed.to_ne_bytes());
        fnv1a_64(&buf)
    }
}

/// Outcome of evaluating a single `TrialConfig`.
///
/// `Copy` — no heap; safe to store in pre-allocated `Vec`.
#[derive(Debug, Clone, Copy)]
pub struct TrialResult {
    pub trial: TrialConfig,
    /// Best combinator score after first RL train (no vote signal).
    pub pass1_score: f64,
    /// Best combinator score after second RL train (vote = pass1 score).
    pub pass2_score: f64,
    /// Vote signal fed into the second RL pass.
    pub ensemble_score: f32,
    /// FNV-1a hash of `trial` for deduplication.
    pub config_hash: u64,
}

impl TrialResult {
    /// Score used for ranking: the best of pass2 (prefer closed-loop result).
    #[inline]
    pub fn best_score(&self) -> f64 {
        self.pass2_score.max(self.pass1_score)
    }
}

// ── Search space ──────────────────────────────────────────────────────────────

/// Static hyperparameter grid.  All slices are `'static` — zero-copy, no heap.
#[derive(Debug)]
pub struct HyperparameterSpace {
    pub learning_rates: &'static [f32],
    pub discount_factors: &'static [f32],
    pub exploration_rates: &'static [f32],
    pub max_epochs_options: &'static [usize],
    pub fitness_thresholds: &'static [f32],
    pub classifier_k_options: &'static [usize],
    pub tree_depth_options: &'static [usize],
    pub nn_hidden_options: &'static [usize],
    pub nn_lr_options: &'static [f32],
    pub nn_epochs_options: &'static [usize],
}

impl HyperparameterSpace {
    /// Default production search space (~174 k combinations in full grid).
    pub const fn default_space() -> Self {
        Self {
            learning_rates: &[0.01, 0.05, 0.08, 0.15, 0.3],
            discount_factors: &[0.9, 0.95, 0.99],
            exploration_rates: &[0.05, 0.1, 0.2, 0.3],
            max_epochs_options: &[50, 100, 200],
            fitness_thresholds: &[0.8, 0.9, 0.95],
            classifier_k_options: &[5, 10, 19],
            tree_depth_options: &[3, 5, 7],
            nn_hidden_options: &[16, 32, 64],
            nn_lr_options: &[0.001, 0.01, 0.05],
            nn_epochs_options: &[10, 30, 50],
        }
    }

    /// Total combinations in the full Cartesian product.
    pub fn total_combinations(&self) -> usize {
        self.learning_rates.len()
            * self.discount_factors.len()
            * self.exploration_rates.len()
            * self.max_epochs_options.len()
            * self.fitness_thresholds.len()
            * self.classifier_k_options.len()
            * self.tree_depth_options.len()
            * self.nn_hidden_options.len()
            * self.nn_lr_options.len()
            * self.nn_epochs_options.len()
    }

    /// Convert a flat index into the corresponding `TrialConfig`.
    ///
    /// Index wraps modulo `total_combinations()` — callers may pass any value.
    pub fn trial_at(&self, mut idx: usize, seed_base: u64) -> TrialConfig {
        let total = self.total_combinations();
        if total == 0 {
            return TrialConfig::default();
        }
        idx %= total;

        let nn_ep_len = self.nn_epochs_options.len();
        let nn_lr_len = self.nn_lr_options.len();
        let nn_hid_len = self.nn_hidden_options.len();
        let td_len = self.tree_depth_options.len();
        let ck_len = self.classifier_k_options.len();
        let ft_len = self.fitness_thresholds.len();
        let me_len = self.max_epochs_options.len();
        let er_len = self.exploration_rates.len();
        let df_len = self.discount_factors.len();

        let nn_ep = idx % nn_ep_len;
        idx /= nn_ep_len;
        let nn_lr = idx % nn_lr_len;
        idx /= nn_lr_len;
        let nn_hid = idx % nn_hid_len;
        idx /= nn_hid_len;
        let td = idx % td_len;
        idx /= td_len;
        let ck = idx % ck_len;
        idx /= ck_len;
        let ft = idx % ft_len;
        idx /= ft_len;
        let me = idx % me_len;
        idx /= me_len;
        let er = idx % er_len;
        idx /= er_len;
        let df = idx % df_len;
        idx /= df_len;
        let lr = idx % self.learning_rates.len();

        TrialConfig {
            learning_rate: self.learning_rates[lr],
            discount_factor: self.discount_factors[df],
            exploration_rate: self.exploration_rates[er],
            max_epochs: self.max_epochs_options[me],
            fitness_threshold: self.fitness_thresholds[ft],
            classifier_k: self.classifier_k_options[ck],
            tree_depth: self.tree_depth_options[td],
            nn_hidden: self.nn_hidden_options[nn_hid],
            nn_lr: self.nn_lr_options[nn_lr],
            nn_epochs: self.nn_epochs_options[nn_ep],
            seed: seed_base,
        }
    }
}

// ── Search strategies ─────────────────────────────────────────────────────────

/// Drive the AutoML loop.
pub trait SearchStrategy: std::fmt::Debug {
    /// Return the next trial to evaluate, or `None` when exhausted / over budget.
    fn next_trial(&mut self) -> Option<TrialConfig>;
    /// Record the result of the last trial returned by `next_trial`.
    fn report(&mut self, result: TrialResult);
    /// Return the best `(config, score)` seen so far.
    fn best(&self) -> Option<(TrialConfig, f64)>;
    /// Number of trials reported so far.
    fn trials_completed(&self) -> usize;
}

// ── GridSearch ────────────────────────────────────────────────────────────────

/// Exhaustively enumerate every point in the Cartesian product of the search space.
#[derive(Debug)]
pub struct GridSearch {
    space: HyperparameterSpace,
    current: usize,
    total: usize,
    best: Option<(TrialConfig, f64)>,
    seed_base: u64,
    completed: usize,
}

impl GridSearch {
    pub fn new(space: HyperparameterSpace, seed_base: u64) -> Self {
        let total = space.total_combinations();
        Self {
            space,
            current: 0,
            total,
            best: None,
            seed_base,
            completed: 0,
        }
    }
}

impl SearchStrategy for GridSearch {
    fn next_trial(&mut self) -> Option<TrialConfig> {
        if self.current >= self.total {
            return None;
        }
        let trial = self
            .space
            .trial_at(self.current, self.seed_base + self.current as u64);
        self.current += 1;
        Some(trial)
    }

    fn report(&mut self, result: TrialResult) {
        self.completed += 1;
        let s = result.best_score();
        match &self.best {
            None => self.best = Some((result.trial, s)),
            Some((_, best_s)) if s > *best_s => self.best = Some((result.trial, s)),
            _ => {}
        }
    }

    fn best(&self) -> Option<(TrialConfig, f64)> {
        self.best
    }

    fn trials_completed(&self) -> usize {
        self.completed
    }
}

// ── RandomSearch ──────────────────────────────────────────────────────────────

/// Sample uniformly at random from the search space, up to a fixed budget.
#[derive(Debug)]
pub struct RandomSearch {
    space: HyperparameterSpace,
    rng: fastrand::Rng,
    budget: usize,
    trials_run: usize,
    best: Option<(TrialConfig, f64)>,
    total: usize,
}

impl RandomSearch {
    pub fn new(space: HyperparameterSpace, budget: usize, seed: u64) -> Self {
        let total = space.total_combinations();
        Self {
            space,
            rng: fastrand::Rng::with_seed(seed),
            budget,
            trials_run: 0,
            best: None,
            total,
        }
    }
}

impl SearchStrategy for RandomSearch {
    fn next_trial(&mut self) -> Option<TrialConfig> {
        if self.trials_run >= self.budget {
            return None;
        }
        let idx = if self.total > 0 {
            self.rng.usize(0..self.total)
        } else {
            0
        };
        let seed = self.rng.u64(..);
        let trial = self.space.trial_at(idx, seed);
        self.trials_run += 1;
        Some(trial)
    }

    fn report(&mut self, result: TrialResult) {
        let s = result.best_score();
        match &self.best {
            None => self.best = Some((result.trial, s)),
            Some((_, best_s)) if s > *best_s => self.best = Some((result.trial, s)),
            _ => {}
        }
    }

    fn best(&self) -> Option<(TrialConfig, f64)> {
        self.best
    }

    fn trials_completed(&self) -> usize {
        self.trials_run
    }
}

// ── AutoMLRun ─────────────────────────────────────────────────────────────────

/// Function-pointer evaluator: receives a `TrialConfig`, returns a `TrialResult`.
///
/// A fn-pointer (not a closure) ensures `AutoMLRun` is `Send + Sync` without
/// lifetime complications.
pub type Evaluator = fn(TrialConfig) -> TrialResult;

/// Drives the AutoML loop: pulls trials from a `SearchStrategy`, evaluates
/// them via `evaluator`, and accumulates results.
pub struct AutoMLRun {
    pub strategy: Box<dyn SearchStrategy>,
    pub evaluator: Evaluator,
    /// Accumulated results, pre-allocated with `max_results` capacity.
    pub results: Vec<TrialResult>,
    pub max_results: usize,
}

impl AutoMLRun {
    pub fn new(
        strategy: Box<dyn SearchStrategy>,
        evaluator: Evaluator,
        max_results: usize,
    ) -> Self {
        Self {
            strategy,
            evaluator,
            results: Vec::with_capacity(max_results),
            max_results,
        }
    }

    /// Run all trials (or until budget/exhaustion), return the best `TrialResult`.
    pub fn run(&mut self) -> Option<TrialResult> {
        while let Some(trial) = self.strategy.next_trial() {
            let result = (self.evaluator)(trial);
            self.strategy.report(result);
            if self.results.len() < self.max_results {
                self.results.push(result);
            }
        }
        self.best_result()
    }

    /// Best result seen so far (highest `pass2_score`).
    pub fn best_result(&self) -> Option<TrialResult> {
        self.results.iter().copied().max_by(|a, b| {
            a.best_score()
                .partial_cmp(&b.best_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_eval(trial: TrialConfig) -> TrialResult {
        TrialResult {
            trial,
            pass1_score: trial.learning_rate as f64,
            pass2_score: trial.learning_rate as f64 * 1.1,
            ensemble_score: trial.learning_rate,
            config_hash: trial.hash(),
        }
    }

    #[test]
    fn test_grid_search_exhaustive() {
        let space = HyperparameterSpace {
            learning_rates: &[0.01, 0.1],
            discount_factors: &[0.9],
            exploration_rates: &[0.1],
            max_epochs_options: &[50],
            fitness_thresholds: &[0.9],
            classifier_k_options: &[5],
            tree_depth_options: &[3],
            nn_hidden_options: &[16],
            nn_lr_options: &[0.01],
            nn_epochs_options: &[10],
        };
        let total = space.total_combinations();
        assert_eq!(total, 2);
        let mut gs = GridSearch::new(space, 0);
        let mut count = 0;
        while let Some(trial) = gs.next_trial() {
            gs.report(dummy_eval(trial));
            count += 1;
        }
        assert_eq!(count, total);
        assert_eq!(gs.trials_completed(), total);
        let (best_cfg, best_s) = gs.best().unwrap();
        assert!(best_s > 0.0);
        assert_eq!(best_cfg.learning_rate, 0.1); // 0.1 > 0.01
    }

    #[test]
    fn test_random_search_budget() {
        let space = HyperparameterSpace::default_space();
        let budget = 10;
        let mut rs = RandomSearch::new(space, budget, 42);
        let mut count = 0;
        while let Some(trial) = rs.next_trial() {
            rs.report(dummy_eval(trial));
            count += 1;
        }
        assert_eq!(count, budget);
        assert_eq!(rs.trials_completed(), budget);
        assert!(rs.best().is_some());
    }

    #[test]
    fn test_trial_hash_deterministic() {
        let t = TrialConfig::default();
        assert_eq!(t.hash(), t.hash());
        let mut t2 = t;
        t2.learning_rate = 0.5;
        assert_ne!(t.hash(), t2.hash());
    }

    #[test]
    fn test_automl_run_best() {
        let space = HyperparameterSpace {
            learning_rates: &[0.01, 0.05, 0.3],
            discount_factors: &[0.95],
            exploration_rates: &[0.2],
            max_epochs_options: &[100],
            fitness_thresholds: &[0.9],
            classifier_k_options: &[19],
            tree_depth_options: &[3],
            nn_hidden_options: &[4],
            nn_lr_options: &[0.01],
            nn_epochs_options: &[10],
        };
        let strategy = Box::new(GridSearch::new(space, 0));
        let mut run = AutoMLRun::new(strategy, dummy_eval, 100);
        let best = run.run().unwrap();
        assert_eq!(best.trial.learning_rate, 0.3);
    }
}
