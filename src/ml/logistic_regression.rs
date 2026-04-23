/// Logistic Regression via batch gradient descent.
///
/// All hot-path arithmetic is pure stack/`Vec` — no external crates.
#[derive(Debug, Clone)]
pub struct LogisticRegressor {
    pub weights: Vec<f64>,
    pub bias: f64,
}

#[inline]
fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z.clamp(-500.0, 500.0)).exp())
}

#[inline]
fn dot(weights: &[f64], x: &[f64]) -> f64 {
    weights
        .iter()
        .zip(x.iter())
        .map(|(w, xi)| w * xi)
        .sum::<f64>()
}

/// Train a [`LogisticRegressor`] with batch gradient descent.
///
/// # Panics
/// Never panics — degenerate inputs (empty train, 0 epochs, feature-length
/// mismatch) are handled gracefully.
pub fn fit(train: &[Vec<f64>], labels: &[bool], lr: f64, epochs: usize) -> LogisticRegressor {
    // Determine feature dimensionality.
    let n_features = train.iter().map(|x| x.len()).max().unwrap_or(0);
    let mut weights = vec![0.0_f64; n_features];
    let mut bias = 0.0_f64;

    let n = train.len();
    if n == 0 || epochs == 0 || n_features == 0 {
        return LogisticRegressor { weights, bias };
    }

    let n_f64 = n as f64;

    for _ in 0..epochs {
        let mut grad_w = vec![0.0_f64; n_features];
        let mut grad_b = 0.0_f64;

        for (x, &label) in train.iter().zip(labels.iter()) {
            let z = dot(&weights, x) + bias;
            let pred = sigmoid(z);
            let error = pred - if label { 1.0 } else { 0.0 };

            // Accumulate per-feature gradient; treat missing dims as 0.
            for j in 0..n_features {
                let xi_j = x.get(j).copied().unwrap_or(0.0);
                grad_w[j] += error * xi_j;
            }
            grad_b += error;
        }

        // Apply mean gradient step.
        for j in 0..n_features {
            weights[j] -= lr * (grad_w[j] / n_f64);
        }
        bias -= lr * (grad_b / n_f64);
    }

    LogisticRegressor { weights, bias }
}

/// Predict binary labels for `test` using a fitted model.
///
/// The threshold is `> 0.5` (strict) so that an all-zero model (e.g. from
/// empty training data or 0 epochs) always predicts `false`.
pub fn predict(model: &LogisticRegressor, test: &[Vec<f64>]) -> Vec<bool> {
    test.iter()
        .map(|x| {
            let z = dot(&model.weights, x) + model.bias;
            sigmoid(z) > 0.5
        })
        .collect()
}

/// Return posterior probability P(y=true | x) for each test point.
pub fn predict_proba(model: &LogisticRegressor, test: &[Vec<f64>]) -> Vec<f64> {
    test.iter()
        .map(|x| {
            let z = dot(&model.weights, x) + model.bias;
            sigmoid(z)
        })
        .collect()
}

/// Fit on `train`/`labels`, then predict `test`.
pub fn classify(
    train: &[Vec<f64>],
    labels: &[bool],
    test: &[Vec<f64>],
    lr: f64,
    epochs: usize,
) -> Vec<bool> {
    let model = fit(train, labels, lr, epochs);
    predict(&model, test)
}

/// Convenience wrapper: lr = 0.01, epochs = 1000.
pub fn classify_default(
    train: &[Vec<f64>],
    labels: &[bool],
    test: &[Vec<f64>],
) -> Vec<bool> {
    classify(train, labels, test, 0.01, 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn test_sigmoid_boundaries() {
        assert!(approx_eq(sigmoid(0.0), 0.5, 1e-10));
        assert!(sigmoid(1000.0) > 0.999);
        assert!(sigmoid(-1000.0) < 0.001);
    }

    #[test]
    fn test_linearly_separable() {
        // Two clearly separated clusters along x[0].
        let train: Vec<Vec<f64>> = (0..20)
            .map(|i| vec![if i < 10 { i as f64 } else { (i + 10) as f64 }])
            .collect();
        let labels: Vec<bool> = (0..20).map(|i| i >= 10).collect();

        let test: Vec<Vec<f64>> = vec![vec![2.0], vec![25.0]];
        let preds = classify_default(&train, &labels, &test);

        assert!(!preds[0], "x=2 should be false");
        assert!(preds[1], "x=25 should be true");
    }

    #[test]
    fn test_empty_train() {
        let preds = classify_default(&[], &[], &[vec![1.0, 2.0]]);
        assert_eq!(preds, vec![false]);
    }

    #[test]
    fn test_zero_epochs() {
        let train = vec![vec![1.0], vec![2.0]];
        let labels = vec![false, true];
        let model = fit(&train, &labels, 0.01, 0);
        assert_eq!(model.weights, vec![0.0]);
        assert_eq!(model.bias, 0.0);
    }

    #[test]
    fn test_predict_proba_range() {
        let train: Vec<Vec<f64>> = vec![vec![0.0], vec![1.0]];
        let labels = vec![false, true];
        let model = fit(&train, &labels, 0.1, 500);
        let probas = predict_proba(&model, &[vec![0.0], vec![1.0]]);
        for p in &probas {
            assert!(*p >= 0.0 && *p <= 1.0);
        }
        assert!(probas[1] > probas[0], "P(true|x=1) should exceed P(true|x=0)");
    }

    #[test]
    fn test_feature_length_mismatch() {
        // Training with 2 features, test with 1 — must not panic.
        let train = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let labels = vec![false, true];
        let test = vec![vec![2.0]]; // shorter
        let preds = classify(&train, &labels, &test, 0.01, 100);
        assert_eq!(preds.len(), 1);
    }

    #[test]
    fn test_all_same_label() {
        let train: Vec<Vec<f64>> = (0..5).map(|i| vec![i as f64]).collect();
        let labels = vec![true; 5];
        let test = vec![vec![10.0]];
        let preds = classify_default(&train, &labels, &test);
        // With all positive labels gradient still converges — must not panic.
        assert_eq!(preds.len(), 1);
    }
}
