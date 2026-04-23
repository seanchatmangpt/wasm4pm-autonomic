use std::f64::consts::PI;

#[derive(Debug, Clone)]
pub struct GaussianNB {
    pub means: [Vec<f64>; 2],    // [0]=negative, [1]=positive
    pub vars: [Vec<f64>; 2],     // [0]=negative, [1]=positive; Bessel-corrected, floor 1e-9
    pub log_priors: [f64; 2],    // [0]=log P(neg), [1]=log P(pos)
}

/// Compute per-feature means and Bessel-corrected sample variances for a slice of samples.
/// Missing feature dimensions are treated as 0.0.
fn class_stats(samples: &[&Vec<f64>], n_features: usize) -> (Vec<f64>, Vec<f64>) {
    let n = samples.len();
    let mut means = vec![0.0_f64; n_features];

    for &x in samples.iter() {
        for j in 0..n_features {
            means[j] += x.get(j).copied().unwrap_or(0.0);
        }
    }
    for mean in means.iter_mut() {
        *mean /= n as f64;
    }

    let vars: Vec<f64> = if n > 1 {
        let mut sq_sum = vec![0.0_f64; n_features];
        for &x in samples.iter() {
            for j in 0..n_features {
                let diff = x.get(j).copied().unwrap_or(0.0) - means[j];
                sq_sum[j] += diff * diff;
            }
        }
        sq_sum
            .into_iter()
            .map(|s| {
                let v = s / (n - 1) as f64;
                if v < 1e-9 { 1e-9 } else { v }
            })
            .collect()
    } else {
        // n == 1: no Bessel correction possible; use floor variance
        vec![1e-9; n_features]
    };

    (means, vars)
}

/// Returns None if all labels are the same class (can't estimate one class's distribution).
pub fn fit(train: &[Vec<f64>], labels: &[bool]) -> Option<GaussianNB> {
    assert_eq!(train.len(), labels.len(), "train and labels must have equal length");

    let n_total = train.len();
    if n_total == 0 {
        return None;
    }

    // Separate into neg (false) and pos (true) groups
    let neg_samples: Vec<&Vec<f64>> = train
        .iter()
        .zip(labels.iter())
        .filter_map(|(x, &y)| if !y { Some(x) } else { None })
        .collect();

    let pos_samples: Vec<&Vec<f64>> = train
        .iter()
        .zip(labels.iter())
        .filter_map(|(x, &y)| if y { Some(x) } else { None })
        .collect();

    // Both classes must be present
    if neg_samples.is_empty() || pos_samples.is_empty() {
        return None;
    }

    let n_features = train.iter().map(|x| x.len()).max().unwrap_or(0);

    let (neg_means, neg_vars) = class_stats(&neg_samples, n_features);
    let (pos_means, pos_vars) = class_stats(&pos_samples, n_features);

    let n_neg = neg_samples.len();
    let n_pos = pos_samples.len();

    let log_prior_neg = ((n_neg as f64) / (n_total as f64)).ln();
    let log_prior_pos = ((n_pos as f64) / (n_total as f64)).ln();

    Some(GaussianNB {
        means: [neg_means, pos_means],
        vars: [neg_vars, pos_vars],
        log_priors: [log_prior_neg, log_prior_pos],
    })
}

/// Compute the log-likelihood of a single sample x under class c.
fn log_likelihood(model: &GaussianNB, x: &[f64], c: usize) -> f64 {
    let n_features = model.means[c].len();
    let mut ll = 0.0_f64;
    for j in 0..n_features {
        let xj = x.get(j).copied().unwrap_or(0.0);
        let mu = model.means[c][j];
        let var = model.vars[c][j];
        // -0.5 * (ln(2π·var) + (x − μ)² / var)
        ll += -0.5 * ((2.0 * PI * var).ln() + (xj - mu).powi(2) / var);
    }
    ll
}

pub fn predict(model: &GaussianNB, test: &[Vec<f64>]) -> Vec<bool> {
    test.iter()
        .map(|x| {
            let log_neg = model.log_priors[0] + log_likelihood(model, x, 0);
            let log_pos = model.log_priors[1] + log_likelihood(model, x, 1);
            // ties → false
            log_pos > log_neg
        })
        .collect()
}

/// Returns P(pos|x) for each test point (probability, not log).
pub fn predict_proba(model: &GaussianNB, test: &[Vec<f64>]) -> Vec<f64> {
    test.iter()
        .map(|x| {
            let log_neg = model.log_priors[0] + log_likelihood(model, x, 0);
            let log_pos = model.log_priors[1] + log_likelihood(model, x, 1);
            // Numerically stable softmax: subtract the max before exp
            let max_log = log_neg.max(log_pos);
            let exp_neg = (log_neg - max_log).exp();
            let exp_pos = (log_pos - max_log).exp();
            exp_pos / (exp_neg + exp_pos)
        })
        .collect()
}

/// Full pipeline. Falls back to all-false if fit returns None.
pub fn classify(train: &[Vec<f64>], labels: &[bool], test: &[Vec<f64>]) -> Vec<bool> {
    match fit(train, labels) {
        Some(model) => predict(&model, test),
        None => vec![false; test.len()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_train() -> (Vec<Vec<f64>>, Vec<bool>) {
        // Negative class: low values ~1.0; positive class: high values ~5.0
        let train = vec![
            vec![1.0, 1.1],
            vec![0.9, 1.0],
            vec![1.1, 0.8],
            vec![5.0, 5.1],
            vec![4.9, 5.0],
            vec![5.1, 4.8],
        ];
        let labels = vec![false, false, false, true, true, true];
        (train, labels)
    }

    #[test]
    fn test_fit_returns_some() {
        let (train, labels) = make_train();
        assert!(fit(&train, &labels).is_some());
    }

    #[test]
    fn test_fit_all_same_class_returns_none() {
        let train = vec![vec![1.0], vec![2.0]];
        let labels = vec![false, false];
        assert!(fit(&train, &labels).is_none());
    }

    #[test]
    fn test_predict_separable() {
        let (train, labels) = make_train();
        let model = fit(&train, &labels).unwrap();
        let test = vec![vec![1.0, 1.0], vec![5.0, 5.0]];
        let preds = predict(&model, &test);
        assert_eq!(preds, vec![false, true]);
    }

    #[test]
    fn test_predict_proba_ordering() {
        let (train, labels) = make_train();
        let model = fit(&train, &labels).unwrap();
        let test = vec![vec![1.0, 1.0], vec![5.0, 5.0]];
        let proba = predict_proba(&model, &test);
        assert!(proba[0] < 0.5, "neg sample should have low pos probability");
        assert!(proba[1] > 0.5, "pos sample should have high pos probability");
    }

    #[test]
    fn test_classify_fallback_on_none() {
        let train = vec![vec![1.0], vec![2.0]];
        let labels = vec![true, true]; // all same class → fit returns None
        let test = vec![vec![1.0], vec![2.0], vec![3.0]];
        let result = classify(&train, &labels, &test);
        assert_eq!(result, vec![false, false, false]);
    }

    #[test]
    fn test_single_sample_per_class_uses_floor_variance() {
        let train = vec![vec![0.0], vec![1.0]];
        let labels = vec![false, true];
        let model = fit(&train, &labels).unwrap();
        // With n == 1 per class, variance must be exactly 1e-9
        assert_eq!(model.vars[0][0], 1e-9);
        assert_eq!(model.vars[1][0], 1e-9);
    }

    #[test]
    fn test_missing_feature_dims_treated_as_zero() {
        // Positive samples have 2 features, negative has only 1
        let train = vec![vec![0.0], vec![5.0, 5.0], vec![5.0, 5.0]];
        let labels = vec![false, true, true];
        let model = fit(&train, &labels).unwrap();
        // neg mean for feature index 1 should be 0.0 (missing dim filled with 0)
        assert_eq!(model.means[0][1], 0.0);
    }

    #[test]
    fn test_log_priors_sum_to_one_in_probability() {
        let (train, labels) = make_train();
        let model = fit(&train, &labels).unwrap();
        let total = model.log_priors[0].exp() + model.log_priors[1].exp();
        assert!((total - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_empty_test_returns_empty() {
        let (train, labels) = make_train();
        let model = fit(&train, &labels).unwrap();
        assert!(predict(&model, &[]).is_empty());
        assert!(predict_proba(&model, &[]).is_empty());
    }
}
