//! Single-hidden-layer feedforward neural network with backpropagation.
//!
//! Implements binary classification following the approach in Chapter 18 of
//! "Data Science from Scratch" by Joel Grus.  All weights are stored with an
//! implicit bias column (last element) so no separate bias vector is needed.

// ── activation ────────────────────────────────────────────────────────────────

#[inline]
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Derivative of sigmoid expressed in terms of the *activation* value `s = sigmoid(x)`.
#[inline]
fn sigmoid_prime_of_activation(s: f64) -> f64 {
    s * (1.0 - s)
}

// ── dot product ───────────────────────────────────────────────────────────────

/// Dot product over the common prefix of `w` and `x`.
fn dot(w: &[f64], x: &[f64]) -> f64 {
    let len = w.len().min(x.len());
    let mut acc = 0.0;
    for i in 0..len {
        acc += w[i] * x[i];
    }
    acc
}

// ── network struct ────────────────────────────────────────────────────────────

/// A single-hidden-layer network for binary classification.
///
/// Weight layout (bias absorbed into last column / element):
/// - `hidden_weights[i]` has length `input_size + 1`; element `[input_size]` is the bias.
/// - `output_weights` has length `hidden_size + 1`; element `[hidden_size]` is the bias.
#[derive(Debug, Clone)]
pub struct Network {
    pub hidden_weights: Vec<Vec<f64>>, // [hidden_size][input_size + 1]
    pub output_weights: Vec<f64>,      // [hidden_size + 1]
}

/// Construct a `Network` with deterministic weight initialisation.
///
/// ```
/// # use dteam::ml::neural_network::new_network;
/// let net = new_network(2, 4);
/// assert_eq!(net.hidden_weights.len(), 4);
/// assert_eq!(net.hidden_weights[0].len(), 3); // input_size + 1
/// assert_eq!(net.output_weights.len(), 5);    // hidden_size + 1
/// ```
pub fn new_network(input_size: usize, hidden_size: usize) -> Network {
    // hidden_weights[i][j] = (i * 0.1 + j * 0.01 - 0.5) * 0.1
    let hidden_weights: Vec<Vec<f64>> = (0..hidden_size)
        .map(|i| {
            (0..=input_size)
                .map(|j| (i as f64 * 0.1 + j as f64 * 0.01 - 0.5) * 0.1)
                .collect()
        })
        .collect();

    // output_weights[i] = (i * 0.1 - 0.5) * 0.1
    let output_weights: Vec<f64> = (0..=hidden_size)
        .map(|i| (i as f64 * 0.1 - 0.5) * 0.1)
        .collect();

    Network {
        hidden_weights,
        output_weights,
    }
}

// ── forward pass ──────────────────────────────────────────────────────────────

/// Run a forward pass.
///
/// Returns `(hidden_with_bias, output)` where:
/// - `hidden_with_bias` is length `hidden_size + 1`; the last element is always `1.0`
///   (the bias node fed into the output layer).
/// - `output` is the scalar network output in `(0, 1)`.
///
/// `input` is silently zero-padded / truncated to `input_size`.
pub fn forward(net: &Network, input: &[f64]) -> (Vec<f64>, f64) {
    let input_size = if net.hidden_weights.is_empty() {
        0
    } else {
        net.hidden_weights[0].len().saturating_sub(1)
    };

    // Build [input..., 1.0] with exact length input_size + 1.
    let mut input_with_bias = vec![0.0f64; input_size + 1];
    let copy_len = input.len().min(input_size);
    input_with_bias[..copy_len].copy_from_slice(&input[..copy_len]);
    input_with_bias[input_size] = 1.0; // bias

    // Hidden layer activations.
    let hidden_size = net.hidden_weights.len();
    let mut hidden_with_bias = vec![0.0f64; hidden_size + 1];
    for (i, hw) in net.hidden_weights.iter().enumerate() {
        hidden_with_bias[i] = sigmoid(dot(hw, &input_with_bias));
    }
    hidden_with_bias[hidden_size] = 1.0; // bias node for output layer

    // Output neuron.
    let output = sigmoid(dot(&net.output_weights, &hidden_with_bias));

    (hidden_with_bias, output)
}

// ── training (backpropagation) ────────────────────────────────────────────────

/// Train `net` in-place using mini-batch-size-1 stochastic gradient descent.
///
/// For each `(x, label)` pair:
/// 1. Forward pass → `hidden_with_bias`, `output`.
/// 2. Output error: `δ_out = (output − target) · output · (1 − output)`.
/// 3. Hidden errors: `δ_h[i] = output_weights[i] · δ_out · hidden[i] · (1 − hidden[i])`.
/// 4. Update `output_weights[i] -= lr · δ_out · hidden_with_bias[i]`.
/// 5. Update `hidden_weights[i][j] -= lr · δ_h[i] · input_with_bias[j]`.
pub fn train(
    net: &mut Network,
    train_data: &[Vec<f64>],
    labels: &[bool],
    lr: f64,
    epochs: usize,
) {
    if train_data.is_empty() || labels.is_empty() || net.hidden_weights.is_empty() {
        return;
    }

    let input_size = net.hidden_weights[0].len().saturating_sub(1);
    if input_size == 0 {
        return;
    }

    let hidden_size = net.hidden_weights.len();
    let n = train_data.len().min(labels.len());

    for _ in 0..epochs {
        for idx in 0..n {
            let x = &train_data[idx];
            let target = if labels[idx] { 1.0f64 } else { 0.0f64 };

            // ── forward ──────────────────────────────────────────────────────
            // Build input_with_bias (zero-padded to input_size).
            let mut input_with_bias = vec![0.0f64; input_size + 1];
            let copy_len = x.len().min(input_size);
            input_with_bias[..copy_len].copy_from_slice(&x[..copy_len]);
            input_with_bias[input_size] = 1.0;

            let (hidden_with_bias, output) = forward(net, x);

            // ── output delta ──────────────────────────────────────────────────
            // δ_out = (output − target) · sigmoid'(output) = (output − target) · output · (1 − output)
            let delta_out = (output - target) * sigmoid_prime_of_activation(output);

            // ── hidden deltas ─────────────────────────────────────────────────
            // δ_h[i] = output_weights[i] · δ_out · sigmoid'(hidden[i])
            let mut delta_hidden = vec![0.0f64; hidden_size];
            for i in 0..hidden_size {
                let h = hidden_with_bias[i];
                delta_hidden[i] =
                    net.output_weights[i] * delta_out * sigmoid_prime_of_activation(h);
            }

            // ── weight updates ────────────────────────────────────────────────
            // Output weights (including bias at index hidden_size).
            for i in 0..=hidden_size {
                net.output_weights[i] -= lr * delta_out * hidden_with_bias[i];
            }

            // Hidden weights.
            for i in 0..hidden_size {
                for j in 0..=input_size {
                    net.hidden_weights[i][j] -= lr * delta_hidden[i] * input_with_bias[j];
                }
            }
        }
    }
}

// ── prediction ────────────────────────────────────────────────────────────────

/// Classify each vector in `test`; threshold at 0.5.
pub fn predict(net: &Network, test: &[Vec<f64>]) -> Vec<bool> {
    let input_size = net
        .hidden_weights
        .first()
        .map(|row| row.len().saturating_sub(1))
        .unwrap_or(0);

    if input_size == 0 {
        return vec![false; test.len()];
    }

    test.iter()
        .map(|x| {
            let (_, output) = forward(net, x);
            output >= 0.5
        })
        .collect()
}

// ── convenience API ───────────────────────────────────────────────────────────

/// Train a new network then classify `test`.
pub fn classify(
    train_data: &[Vec<f64>],
    labels: &[bool],
    test: &[Vec<f64>],
    hidden_size: usize,
    lr: f64,
    epochs: usize,
) -> Vec<bool> {
    let input_size = train_data.iter().map(|x| x.len()).max().unwrap_or(0);
    if input_size == 0 {
        return vec![false; test.len()];
    }

    let mut net = new_network(input_size, hidden_size);
    train(&mut net, train_data, labels, lr, epochs);
    predict(&net, test)
}

/// Convenience wrapper: `hidden_size = 4`, `lr = 0.01`, `epochs = 200`.
pub fn classify_default(
    train_data: &[Vec<f64>],
    labels: &[bool],
    test: &[Vec<f64>],
) -> Vec<bool> {
    classify(train_data, labels, test, 4, 0.01, 200)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── helper ────────────────────────────────────────────────────────────────

    fn xor_data() -> (Vec<Vec<f64>>, Vec<bool>) {
        let train = vec![
            vec![0.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
        ];
        let labels = vec![false, true, true, false];
        (train, labels)
    }

    // ── 1. XOR: network learns the non-linearly-separable XOR function ─────────
    #[test]
    fn test_xor_classification() {
        let (train, labels) = xor_data();
        // More hidden units and epochs give XOR reliable convergence with the
        // deterministic init used here.
        let result = classify(&train, &labels, &train, 8, 0.5, 5000);
        assert_eq!(
            result, labels,
            "network should learn XOR after sufficient training"
        );
    }

    // ── 2. Empty inputs ────────────────────────────────────────────────────────
    #[test]
    fn test_empty_train_returns_false() {
        // Empty training set → input_size = 0 → all-false predictions.
        let test = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let result = classify_default(&[], &[], &test);
        assert_eq!(result, vec![false, false]);
    }

    #[test]
    fn test_empty_test_returns_empty() {
        let (train, labels) = xor_data();
        let result = classify_default(&train, &labels, &[]);
        assert!(result.is_empty());
    }

    // ── 3. Single training example ─────────────────────────────────────────────
    #[test]
    fn test_single_example_converges() {
        // A single positive example — the network should output ≥ 0.5 for it
        // after enough epochs.
        let train = vec![vec![1.0, 1.0]];
        let labels = vec![true];
        let result = classify(&train, &labels, &train, 4, 0.5, 2000);
        assert_eq!(result, vec![true], "single positive example should converge");
    }

    // ── 4. Forward pass shape ──────────────────────────────────────────────────
    #[test]
    fn test_forward_pass_shape() {
        let net = new_network(3, 5); // input_size=3, hidden_size=5
        let input = vec![0.1, 0.2, 0.3];
        let (hidden_with_bias, output) = forward(&net, &input);

        // hidden_with_bias should be hidden_size + 1 = 6, last element = 1.0 (bias).
        assert_eq!(hidden_with_bias.len(), 6);
        assert_eq!(
            hidden_with_bias[5], 1.0,
            "last element of hidden_with_bias must be the bias node"
        );

        // Output must be a valid probability.
        assert!(
            (0.0..=1.0).contains(&output),
            "sigmoid output must be in [0, 1], got {output}"
        );

        // Each hidden activation must also be in (0, 1).
        for &h in &hidden_with_bias[..5] {
            assert!(
                (0.0..=1.0).contains(&h),
                "hidden activation out of range: {h}"
            );
        }
    }

    // ── 5. Weight dimensions from new_network ─────────────────────────────────
    #[test]
    fn test_network_weight_dimensions() {
        let net = new_network(2, 4);
        assert_eq!(net.hidden_weights.len(), 4, "wrong hidden layer count");
        for row in &net.hidden_weights {
            assert_eq!(row.len(), 3, "each hidden row should be input_size+1 = 3");
        }
        assert_eq!(
            net.output_weights.len(),
            5,
            "output weights should be hidden_size+1 = 5"
        );
    }

    // ── 6. Short input is zero-padded (no panic) ───────────────────────────────
    #[test]
    fn test_short_input_zero_padded() {
        let net = new_network(4, 3);
        // Feed only 2 features to a net expecting 4 — should not panic.
        let (_, output) = forward(&net, &[0.5, 0.5]);
        assert!((0.0..=1.0).contains(&output));
    }

    // ── 7. Predict on untrained network returns a Vec<bool> ───────────────────
    #[test]
    fn test_predict_untrained_returns_bools() {
        let net = new_network(2, 3);
        let test = vec![vec![0.0, 0.0], vec![1.0, 1.0]];
        let preds = predict(&net, &test);
        assert_eq!(preds.len(), 2);
    }
}
