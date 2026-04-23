/// Online linear classifier using the Perceptron algorithm.
#[derive(Debug, Clone)]
pub struct Perceptron {
    pub weights: Vec<f64>,
    pub bias: f64,
}

/// Compute the dot product of `weights` and `x` over their shared prefix.
fn dot(weights: &[f64], x: &[f64]) -> f64 {
    let len = weights.len().min(x.len());
    let mut sum = 0.0;
    for j in 0..len {
        sum += weights[j] * x[j];
    }
    sum
}

/// Train a Perceptron on `train`/`labels` for `epochs` passes.
///
/// - `n_features` is the maximum feature-vector length found in `train`.
/// - Weights and bias are initialised to 0.0.
/// - On each misclassification the weights and bias are updated by ±1 scaled
///   by the raw input values.
pub fn fit(train: &[Vec<f64>], labels: &[bool], epochs: usize) -> Perceptron {
    let n_features = train.iter().map(|x| x.len()).max().unwrap_or(0);
    let mut weights = vec![0.0f64; n_features];
    let mut bias = 0.0f64;

    for _ in 0..epochs {
        for (x, &label) in train.iter().zip(labels.iter()) {
            let predicted = dot(&weights, x) + bias >= 0.0;
            if predicted != label {
                let y = if label { 1.0f64 } else { -1.0f64 };
                let shared = weights.len().min(x.len());
                for j in 0..shared {
                    weights[j] += y * x[j];
                }
                bias += y;
            }
        }
    }

    Perceptron { weights, bias }
}

/// Classify each vector in `test` using a trained `Perceptron`.
pub fn predict(p: &Perceptron, test: &[Vec<f64>]) -> Vec<bool> {
    test.iter()
        .map(|x| dot(&p.weights, x) + p.bias >= 0.0)
        .collect()
}

/// Train on `train`/`labels` for `epochs` passes then classify `test`.
pub fn classify(
    train: &[Vec<f64>],
    labels: &[bool],
    test: &[Vec<f64>],
    epochs: usize,
) -> Vec<bool> {
    let p = fit(train, labels, epochs);
    predict(&p, test)
}

/// Convenience wrapper: 100 training epochs.
pub fn classify_default(
    train: &[Vec<f64>],
    labels: &[bool],
    test: &[Vec<f64>],
) -> Vec<bool> {
    classify(train, labels, test, 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AND gate (linearly separable).
    #[test]
    fn test_and_gate() {
        let train = vec![
            vec![0.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
        ];
        let labels = vec![false, false, false, true];
        let result = classify_default(&train, &labels, &train);
        assert_eq!(result, labels);
    }

    /// OR gate (linearly separable).
    #[test]
    fn test_or_gate() {
        let train = vec![
            vec![0.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
        ];
        let labels = vec![false, true, true, true];
        let result = classify_default(&train, &labels, &train);
        assert_eq!(result, labels);
    }

    /// Empty training set → untrained model; dot product of empty weights is 0.0,
    /// so 0.0 + 0.0 >= 0.0 is true for every input (default-true behaviour).
    #[test]
    fn test_empty_train() {
        let p = fit(&[], &[], 10);
        assert!(p.weights.is_empty());
        assert_eq!(p.bias, 0.0);

        let test = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let result = predict(&p, &test);
        // With zero weights and zero bias, activation = 0.0 >= 0.0 → true.
        assert_eq!(result, vec![true, true]);
    }

    /// Empty test set → empty prediction vec.
    #[test]
    fn test_empty_test() {
        let train = vec![vec![1.0], vec![-1.0]];
        let labels = vec![true, false];
        let result = classify_default(&train, &labels, &[]);
        assert!(result.is_empty());
    }

    /// Zero epochs → untrained model predicts all true (bias 0 ≥ 0).
    #[test]
    fn test_zero_epochs() {
        let train = vec![vec![1.0], vec![-1.0]];
        let labels = vec![true, false];
        let result = classify(&train, &labels, &train, 0);
        assert_eq!(result, vec![true, true]);
    }
}
