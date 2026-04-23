/// Multinomial Naive Bayes classifier for activity-frequency feature vectors.
///
/// Features are non-negative floats treated as activity frequency counts.
/// Laplace smoothing α = 1.0 is applied to avoid zero-probability issues.
pub fn classify(train: &[Vec<f64>], labels: &[bool], test: &[Vec<f64>]) -> Vec<bool> {
    // Edge case: empty training set
    if train.is_empty() || labels.is_empty() {
        return vec![false; test.len()];
    }

    let n_total = labels.len() as f64;
    let n_pos = labels.iter().filter(|&&l| l).count();
    let n_neg = labels.len() - n_pos;

    // Edge case: all positives or all negatives
    if n_pos == 0 {
        return vec![false; test.len()];
    }
    if n_neg == 0 {
        return vec![true; test.len()];
    }

    let n_features = train[0].len();

    // Edge case: zero features
    if n_features == 0 {
        return vec![false; test.len()];
    }

    // Accumulate per-feature sums and totals for each class
    let mut sum_pos = vec![0.0f64; n_features];
    let mut sum_neg = vec![0.0f64; n_features];
    let mut total_pos: f64 = 0.0;
    let mut total_neg: f64 = 0.0;

    for (x, &label) in train.iter().zip(labels.iter()) {
        // Guard against ragged rows — use up to n_features columns
        let len = x.len().min(n_features);
        if label {
            for j in 0..len {
                sum_pos[j] += x[j];
                total_pos += x[j];
            }
        } else {
            for j in 0..len {
                sum_neg[j] += x[j];
                total_neg += x[j];
            }
        }
    }

    // Laplace-smoothed conditional log-probabilities
    // P(x_j | pos) = (sum_pos_j + 1.0) / (total_pos + n_features)
    let denom_pos = total_pos + n_features as f64;
    let denom_neg = total_neg + n_features as f64;

    let log_p_pos: Vec<f64> = (0..n_features)
        .map(|j| ((sum_pos[j] + 1.0) / denom_pos).max(1e-300).ln())
        .collect();
    let log_p_neg: Vec<f64> = (0..n_features)
        .map(|j| ((sum_neg[j] + 1.0) / denom_neg).max(1e-300).ln())
        .collect();

    let log_prior_pos = (n_pos as f64 / n_total).max(1e-300).ln();
    let log_prior_neg = (n_neg as f64 / n_total).max(1e-300).ln();

    // Classify each test vector
    test.iter()
        .map(|x| {
            let len = x.len().min(n_features);
            let mut log_pos = log_prior_pos;
            let mut log_neg = log_prior_neg;
            for j in 0..len {
                let freq = x[j];
                if freq != 0.0 {
                    log_pos += freq * log_p_pos[j];
                    log_neg += freq * log_p_neg[j];
                }
            }
            log_pos > log_neg
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_train() {
        let result = classify(&[], &[], &[vec![1.0, 2.0]]);
        assert_eq!(result, vec![false]);
    }

    #[test]
    fn test_all_positive() {
        let train = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let labels = vec![true, true];
        let test = vec![vec![1.0, 1.0]];
        assert_eq!(classify(&train, &labels, &test), vec![true]);
    }

    #[test]
    fn test_all_negative() {
        let train = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let labels = vec![false, false];
        let test = vec![vec![1.0, 1.0]];
        assert_eq!(classify(&train, &labels, &test), vec![false]);
    }

    #[test]
    fn test_zero_features() {
        let train = vec![vec![], vec![]];
        let labels = vec![true, false];
        let test = vec![vec![]];
        assert_eq!(classify(&train, &labels, &test), vec![false]);
    }

    #[test]
    fn test_basic_separation() {
        // Feature 0 dominant in positives, feature 1 dominant in negatives
        let train = vec![
            vec![10.0, 0.0],
            vec![8.0, 1.0],
            vec![0.0, 10.0],
            vec![1.0, 8.0],
        ];
        let labels = vec![true, true, false, false];
        let test = vec![
            vec![5.0, 0.0], // clearly positive
            vec![0.0, 5.0], // clearly negative
        ];
        let result = classify(&train, &labels, &test);
        assert_eq!(result[0], true, "high feature-0 trace should be positive");
        assert_eq!(result[1], false, "high feature-1 trace should be negative");
    }

    #[test]
    fn test_empty_test_set() {
        let train = vec![vec![1.0, 0.0]];
        let labels = vec![true];
        let result = classify(&train, &labels, &[]);
        assert!(result.is_empty());
    }
}
