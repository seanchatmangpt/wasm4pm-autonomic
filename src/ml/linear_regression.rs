/// Simple Linear Regression (OLS, chapter 14) and Multiple Regression (GD, chapter 15)
/// from "Data Science from Scratch" by Joel Grus.

// ---------------------------------------------------------------------------
// Simple Linear Regression (OLS)
// ---------------------------------------------------------------------------

/// Holds the intercept (`alpha`) and slope (`beta`) for a simple linear model.
/// Prediction: y_hat = alpha + beta * x
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleLinear {
    pub alpha: f64,
    pub beta: f64,
}

#[inline]
fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.iter().copied().sum::<f64>() / v.len() as f64
}

/// Fit a simple OLS regression.
///
/// beta  = Cov(x, y) / Var(x)
/// alpha = mean(y) - beta * mean(x)
///
/// Returns `SimpleLinear { alpha: 0.0, beta: 0.0 }` when `x` or `y` is empty.
pub fn fit_simple(x: &[f64], y: &[f64]) -> SimpleLinear {
    let n = x.len().min(y.len());
    if n == 0 {
        return SimpleLinear { alpha: 0.0, beta: 0.0 };
    }

    let mx = mean(&x[..n]);
    let my = mean(&y[..n]);

    let mut cov = 0.0_f64;
    let mut var_x = 0.0_f64;
    for i in 0..n {
        let dx = x[i] - mx;
        cov += dx * (y[i] - my);
        var_x += dx * dx;
    }

    let beta = if var_x == 0.0 { 0.0 } else { cov / var_x };
    let alpha = my - beta * mx;

    SimpleLinear { alpha, beta }
}

/// Apply a fitted simple model to a slice of inputs.
pub fn predict_simple(model: &SimpleLinear, x: &[f64]) -> Vec<f64> {
    x.iter().map(|&xi| model.alpha + model.beta * xi).collect()
}

/// Coefficient of determination (R²).
///
/// R² = 1 - SS_res / SS_tot
///
/// Returns `1.0` when `ss_tot == 0` (constant target — perfect by convention).
pub fn r_squared(y_true: &[f64], y_pred: &[f64]) -> f64 {
    let n = y_true.len().min(y_pred.len());
    if n == 0 {
        return 1.0;
    }

    let my = mean(&y_true[..n]);
    let mut ss_res = 0.0_f64;
    let mut ss_tot = 0.0_f64;
    for i in 0..n {
        let r = y_true[i] - y_pred[i];
        ss_res += r * r;
        let t = y_true[i] - my;
        ss_tot += t * t;
    }

    if ss_tot == 0.0 {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    }
}

// ---------------------------------------------------------------------------
// Multiple Regression (batch gradient descent with optional L2)
// ---------------------------------------------------------------------------

/// Holds the weight vector for a multiple linear model.
///
/// `weights[0]` = intercept (bias); `weights[1..]` = feature coefficients.
#[derive(Debug, Clone, PartialEq)]
pub struct MultipleLinear {
    pub weights: Vec<f64>,
}

#[inline]
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(&ai, &bi)| ai * bi).sum()
}

/// Fit multiple linear regression via batch gradient descent with optional L2 regularization.
///
/// Each sample in `train` must have the same length.  A bias term (`1.0`) is
/// prepended internally — do **not** include it in the input features.
///
/// `lambda = 0.0` → no regularization (plain OLS GD).
pub fn fit_multiple(
    train: &[Vec<f64>],
    targets: &[f64],
    lr: f64,
    epochs: usize,
    lambda: f64,
) -> MultipleLinear {
    let n = train.len().min(targets.len());
    if n == 0 {
        return MultipleLinear { weights: vec![] };
    }

    let n_features = train[0].len();
    let n_weights = n_features + 1; // +1 for intercept
    let mut weights = vec![0.0_f64; n_weights];

    // Build design matrix (prepend 1.0) once to avoid repeated allocation per epoch.
    let x_ext: Vec<Vec<f64>> = train[..n]
        .iter()
        .map(|row| {
            let mut v = Vec::with_capacity(n_weights);
            v.push(1.0);
            v.extend_from_slice(row);
            v
        })
        .collect();

    let n_f64 = n as f64;

    for _ in 0..epochs {
        // Compute errors: error_i = dot(w, x_i) - target_i
        let errors: Vec<f64> = x_ext
            .iter()
            .zip(targets[..n].iter())
            .map(|(xi, &ti)| dot(&weights, xi) - ti)
            .collect();

        // Update each weight.
        for j in 0..n_weights {
            let grad_j = errors
                .iter()
                .zip(x_ext.iter())
                .map(|(&e, xi)| e * xi[j])
                .sum::<f64>()
                / n_f64;

            // L2 regularization applies to feature weights only (not intercept).
            let reg = if j == 0 { 0.0 } else { lambda * weights[j] };
            weights[j] -= lr * (grad_j + reg);
        }
    }

    MultipleLinear { weights }
}

/// Fit multiple linear regression with sensible defaults:
/// `lr = 0.001`, `epochs = 5 000`, `lambda = 0.0`.
pub fn fit_multiple_default(train: &[Vec<f64>], targets: &[f64]) -> MultipleLinear {
    fit_multiple(train, targets, 0.001, 5_000, 0.0)
}

/// Apply a fitted multiple model to a test set.
///
/// Each row in `test` must match the feature dimensionality used during fitting.
pub fn predict_multiple(model: &MultipleLinear, test: &[Vec<f64>]) -> Vec<f64> {
    test.iter()
        .map(|row| {
            // Prepend 1.0 for intercept.
            let mut x_ext = Vec::with_capacity(row.len() + 1);
            x_ext.push(1.0);
            x_ext.extend_from_slice(row);
            dot(&model.weights, &x_ext)
        })
        .collect()
}

/// Binary classifier built on multiple linear regression.
///
/// Fits a model treating `true → 1.0`, `false → 0.0`, then predicts on `test`
/// and thresholds at `0.5`.  Returns all `false` when `train` is empty.
pub fn classify_multiple(
    train: &[Vec<f64>],
    labels: &[bool],
    test: &[Vec<f64>],
) -> Vec<bool> {
    let n = train.len().min(labels.len());
    if n == 0 {
        return vec![false; test.len()];
    }

    let targets: Vec<f64> = labels[..n]
        .iter()
        .map(|&b| if b { 1.0 } else { 0.0 })
        .collect();

    let model = fit_multiple_default(train, &targets);
    predict_multiple(&model, test)
        .into_iter()
        .map(|p| p > 0.5)
        .collect()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Perfect linear relationship: y = 2x + 1
    #[test]
    fn test_fit_simple_perfect_line() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 2.0 * xi + 1.0).collect();

        let model = fit_simple(&x, &y);
        assert!((model.beta - 2.0).abs() < 1e-10, "beta should be 2.0, got {}", model.beta);
        assert!((model.alpha - 1.0).abs() < 1e-10, "alpha should be 1.0, got {}", model.alpha);
    }

    /// R² for a perfect fit should be 1.0.
    #[test]
    fn test_r_squared_perfect() {
        let y_true = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y_pred = y_true.clone();
        let r2 = r_squared(&y_true, &y_pred);
        assert!((r2 - 1.0).abs() < 1e-10, "R² for perfect fit should be 1.0, got {r2}");
    }

    /// R² for a constant prediction equal to the mean should be 0.0.
    #[test]
    fn test_r_squared_zero() {
        let y_true = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        // Predict the mean (3.0) for every point → SS_res == SS_tot → R² = 0
        let mean_val = mean(&y_true);
        let y_pred = vec![mean_val; y_true.len()];
        let r2 = r_squared(&y_true, &y_pred);
        assert!(r2.abs() < 1e-10, "R² for mean prediction should be 0.0, got {r2}");
    }

    /// Empty inputs should return zero-weight SimpleLinear without panic.
    #[test]
    fn test_fit_simple_empty() {
        let model = fit_simple(&[], &[]);
        assert_eq!(model.alpha, 0.0);
        assert_eq!(model.beta, 0.0);
    }

    /// Multiple regression on y = 3*x1 + 2*x2 + 1 should converge.
    #[test]
    fn test_fit_multiple_linear_plane() {
        // Generate samples from y = 3x₁ + 2x₂ + 1
        let n = 50;
        let train: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let x1 = i as f64;
                let x2 = (i % 5) as f64;
                vec![x1, x2]
            })
            .collect();
        let targets: Vec<f64> = train
            .iter()
            .map(|row| 3.0 * row[0] + 2.0 * row[1] + 1.0)
            .collect();

        // Use more epochs / smaller lr for tighter convergence on this scale.
        let model = fit_multiple(&train, &targets, 0.0005, 10_000, 0.0);

        // Predict the first training point.
        let preds = predict_multiple(&model, &train[..3]);
        for (i, &pred) in preds.iter().enumerate() {
            let expected = targets[i];
            assert!(
                (pred - expected).abs() < 1.0,
                "sample {i}: expected ~{expected:.2}, got {pred:.2}"
            );
        }
    }

    /// classify_multiple: well-separated classes → most predictions should be correct.
    ///
    /// Positive class lives around x=5.0, negative class around x=-5.0.
    /// Gradient descent with defaults converges well on this unit-scale problem.
    #[test]
    fn test_classify_multiple_separable() {
        // Positive examples near x = 5.0, negative near x = -5.0.
        let mut train: Vec<Vec<f64>> = (0..10).map(|_| vec![5.0]).collect();
        let mut labels: Vec<bool> = vec![true; 10];
        let neg: Vec<Vec<f64>> = (0..10).map(|_| vec![-5.0]).collect();
        train.extend(neg);
        labels.extend(vec![false; 10]);

        // Test on both centroids.
        let test = vec![vec![5.0], vec![-5.0]];
        let preds = classify_multiple(&train, &labels, &test);
        assert!(preds[0], "positive centroid should classify as true");
        assert!(!preds[1], "negative centroid should classify as false");
    }

    /// classify_multiple with empty training set returns all false.
    #[test]
    fn test_classify_multiple_empty_train() {
        let test = vec![vec![1.0], vec![2.0]];
        let result = classify_multiple(&[], &[], &test);
        assert_eq!(result, vec![false, false]);
    }

    /// predict_simple: intercept-only model (beta=0) predicts constant alpha.
    #[test]
    fn test_predict_simple_constant() {
        let model = SimpleLinear { alpha: 7.0, beta: 0.0 };
        let x = vec![1.0, 2.0, 3.0];
        let preds = predict_simple(&model, &x);
        assert!(preds.iter().all(|&p| (p - 7.0).abs() < 1e-12));
    }

    /// r_squared with constant y_true returns 1.0 (ss_tot == 0 guard).
    #[test]
    fn test_r_squared_constant_y_true() {
        let y_true = vec![5.0, 5.0, 5.0];
        let y_pred = vec![5.0, 5.0, 5.0];
        assert_eq!(r_squared(&y_true, &y_pred), 1.0);
    }

    /// fit_multiple_default is consistent with fit_multiple using the documented defaults.
    #[test]
    fn test_fit_multiple_default_consistent() {
        let train = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
        let targets = vec![10.0, 20.0, 30.0];

        let m1 = fit_multiple_default(&train, &targets);
        let m2 = fit_multiple(&train, &targets, 0.001, 5_000, 0.0);

        assert_eq!(m1.weights.len(), m2.weights.len());
        for (a, b) in m1.weights.iter().zip(m2.weights.iter()) {
            assert!((a - b).abs() < 1e-12, "weights diverge: {a} vs {b}");
        }
    }
}
