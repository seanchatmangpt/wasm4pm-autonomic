//! Layer-based deep learning abstractions.
//!
//! Implements the layer abstraction introduced in Chapter 19 of
//! "Data Science from Scratch" by Joel Grus.  All computation is done
//! in pure Rust with no external crates.  Hot paths are allocation-free
//! wherever possible; `Vec` is used only for layer-local caches whose
//! size is fixed after construction.

// ── Activation functions ──────────────────────────────────────────────────────

/// Logistic sigmoid: σ(x) = 1 / (1 + e^{-x}).
#[inline]
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Derivative of sigmoid expressed in terms of the *output* value `s = σ(x)`:
/// dσ/dx = s · (1 − s).
#[inline]
pub fn sigmoid_grad(s: f64) -> f64 {
    s * (1.0 - s)
}

/// Hyperbolic tangent activation (re-exported for symmetry with other activations).
#[inline]
pub fn tanh_act(x: f64) -> f64 {
    x.tanh()
}

/// Derivative of tanh expressed in terms of the *output* value `t = tanh(x)`:
/// d(tanh)/dx = 1 − t².
#[inline]
pub fn tanh_grad(t: f64) -> f64 {
    1.0 - t * t
}

/// Rectified linear unit: max(0, x).
#[inline]
pub fn relu(x: f64) -> f64 {
    if x > 0.0 { x } else { 0.0 }
}

/// Sub-gradient of ReLU: 1 if x > 0, 0 otherwise.
#[inline]
pub fn relu_grad(x: f64) -> f64 {
    if x > 0.0 { 1.0 } else { 0.0 }
}

// ── Layer trait ───────────────────────────────────────────────────────────────

/// A differentiable building block in a neural network.
pub trait Layer {
    /// Forward pass: compute outputs from `inputs`.
    fn forward(&mut self, inputs: &[f64]) -> Vec<f64>;

    /// Backward pass: given `grad` (gradient of loss w.r.t. this layer's
    /// outputs), return the gradient w.r.t. this layer's inputs.
    /// Also accumulates parameter gradients internally.
    fn backward(&mut self, grad: &[f64]) -> Vec<f64>;

    /// Flat view of all trainable parameters (weights then biases).
    fn params(&self) -> Vec<f64>;

    /// Flat view of gradients corresponding to `params()`.
    fn grads(&self) -> Vec<f64>;

    /// In-place SGD step: param -= lr * grad.
    fn update(&mut self, lr: f64);
}

// ── Linear Layer ──────────────────────────────────────────────────────────────

/// Fully-connected affine layer: output[i] = Σ_j weights[i][j] · input[j] + bias[i].
pub struct Linear {
    /// Shape: `[output_size][input_size]`.
    pub weights: Vec<Vec<f64>>,
    /// Shape: `[output_size]`.
    pub bias: Vec<f64>,
    /// Input cached during the forward pass for use in backward.
    last_input: Vec<f64>,
    weight_grads: Vec<Vec<f64>>,
    bias_grads: Vec<f64>,
}

impl Linear {
    /// Construct a `Linear` layer with deterministic weight initialisation.
    ///
    /// `weights[i][j] = (i as f64 * 0.1 + j as f64 * 0.01 − 0.5) * 0.2`
    /// `bias[i] = 0.0`
    pub fn new(input_size: usize, output_size: usize) -> Self {
        let weights: Vec<Vec<f64>> = (0..output_size)
            .map(|i| {
                (0..input_size)
                    .map(|j| (i as f64 * 0.1 + j as f64 * 0.01 - 0.5) * 0.2)
                    .collect()
            })
            .collect();

        let weight_grads: Vec<Vec<f64>> = (0..output_size)
            .map(|_| vec![0.0; input_size])
            .collect();

        Linear {
            bias: vec![0.0; output_size],
            bias_grads: vec![0.0; output_size],
            last_input: vec![0.0; input_size],
            weights,
            weight_grads,
        }
    }
}

impl Layer for Linear {
    fn forward(&mut self, inputs: &[f64]) -> Vec<f64> {
        let input_size = self.weights.first().map(|r| r.len()).unwrap_or(0);
        // Cache the input (zero-pad / truncate to input_size).
        self.last_input = vec![0.0; input_size];
        let copy = inputs.len().min(input_size);
        self.last_input[..copy].copy_from_slice(&inputs[..copy]);

        self.weights
            .iter()
            .zip(self.bias.iter())
            .map(|(row, &b)| {
                let dot: f64 = row.iter().zip(self.last_input.iter()).map(|(&w, &x)| w * x).sum();
                dot + b
            })
            .collect()
    }

    fn backward(&mut self, grad: &[f64]) -> Vec<f64> {
        let output_size = self.weights.len();
        let input_size = self.weights.first().map(|r| r.len()).unwrap_or(0);

        // Reset accumulated gradients.
        for wg in self.weight_grads.iter_mut() {
            for g in wg.iter_mut() {
                *g = 0.0;
            }
        }
        for bg in self.bias_grads.iter_mut() {
            *bg = 0.0;
        }

        // grad_input[j] = Σ_i weights[i][j] * grad[i]
        let mut grad_input = vec![0.0f64; input_size];
        let grad_len = grad.len().min(output_size);

        for i in 0..grad_len {
            // weight_grads[i][j] = grad[i] * last_input[j]
            for j in 0..input_size {
                self.weight_grads[i][j] = grad[i] * self.last_input[j];
                grad_input[j] += self.weights[i][j] * grad[i];
            }
            self.bias_grads[i] = grad[i];
        }

        grad_input
    }

    fn params(&self) -> Vec<f64> {
        let mut out = Vec::new();
        for row in &self.weights {
            out.extend_from_slice(row);
        }
        out.extend_from_slice(&self.bias);
        out
    }

    fn grads(&self) -> Vec<f64> {
        let mut out = Vec::new();
        for row in &self.weight_grads {
            out.extend_from_slice(row);
        }
        out.extend_from_slice(&self.bias_grads);
        out
    }

    fn update(&mut self, lr: f64) {
        for (row, grad_row) in self.weights.iter_mut().zip(self.weight_grads.iter()) {
            for (w, g) in row.iter_mut().zip(grad_row.iter()) {
                *w -= lr * g;
            }
        }
        for (b, g) in self.bias.iter_mut().zip(self.bias_grads.iter()) {
            *b -= lr * g;
        }
    }
}

// ── Sigmoid Activation Layer ──────────────────────────────────────────────────

/// Element-wise sigmoid activation.
pub struct Sigmoid {
    last_output: Vec<f64>,
}

impl Sigmoid {
    pub fn new() -> Self {
        Sigmoid { last_output: Vec::new() }
    }
}

impl Default for Sigmoid {
    fn default() -> Self {
        Self::new()
    }
}

impl Layer for Sigmoid {
    fn forward(&mut self, inputs: &[f64]) -> Vec<f64> {
        self.last_output = inputs.iter().map(|&x| sigmoid(x)).collect();
        self.last_output.clone()
    }

    fn backward(&mut self, grad: &[f64]) -> Vec<f64> {
        self.last_output
            .iter()
            .zip(grad.iter())
            .map(|(&s, &g)| g * sigmoid_grad(s))
            .collect()
    }

    fn params(&self) -> Vec<f64> { Vec::new() }
    fn grads(&self) -> Vec<f64> { Vec::new() }
    fn update(&mut self, _lr: f64) {}
}

// ── Tanh Activation Layer ─────────────────────────────────────────────────────

/// Element-wise hyperbolic tangent activation.
pub struct Tanh {
    last_output: Vec<f64>,
}

impl Tanh {
    pub fn new() -> Self {
        Tanh { last_output: Vec::new() }
    }
}

impl Default for Tanh {
    fn default() -> Self {
        Self::new()
    }
}

impl Layer for Tanh {
    fn forward(&mut self, inputs: &[f64]) -> Vec<f64> {
        self.last_output = inputs.iter().map(|&x| tanh_act(x)).collect();
        self.last_output.clone()
    }

    fn backward(&mut self, grad: &[f64]) -> Vec<f64> {
        self.last_output
            .iter()
            .zip(grad.iter())
            .map(|(&t, &g)| g * tanh_grad(t))
            .collect()
    }

    fn params(&self) -> Vec<f64> { Vec::new() }
    fn grads(&self) -> Vec<f64> { Vec::new() }
    fn update(&mut self, _lr: f64) {}
}

// ── ReLU Activation Layer ─────────────────────────────────────────────────────

/// Element-wise rectified linear unit activation.
pub struct Relu {
    last_input: Vec<f64>,
}

impl Relu {
    pub fn new() -> Self {
        Relu { last_input: Vec::new() }
    }
}

impl Default for Relu {
    fn default() -> Self {
        Self::new()
    }
}

impl Layer for Relu {
    fn forward(&mut self, inputs: &[f64]) -> Vec<f64> {
        self.last_input = inputs.to_vec();
        inputs.iter().map(|&x| relu(x)).collect()
    }

    fn backward(&mut self, grad: &[f64]) -> Vec<f64> {
        self.last_input
            .iter()
            .zip(grad.iter())
            .map(|(&x, &g)| g * relu_grad(x))
            .collect()
    }

    fn params(&self) -> Vec<f64> { Vec::new() }
    fn grads(&self) -> Vec<f64> { Vec::new() }
    fn update(&mut self, _lr: f64) {}
}

// ── Softmax Layer ─────────────────────────────────────────────────────────────

/// Numerically-stable softmax over all inputs.
///
/// Backward pass returns the upstream gradient unchanged — this is the
/// correct pairing when the loss is cross-entropy and the gradient is
/// computed as `(softmax_output − one_hot_target)` before calling
/// `backward`.  For other loss / output pairings compute the full
/// Jacobian externally.
pub struct Softmax {
    last_output: Vec<f64>,
}

impl Softmax {
    pub fn new() -> Self {
        Softmax { last_output: Vec::new() }
    }
}

impl Default for Softmax {
    fn default() -> Self {
        Self::new()
    }
}

impl Layer for Softmax {
    fn forward(&mut self, inputs: &[f64]) -> Vec<f64> {
        let max = inputs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let exps: Vec<f64> = inputs.iter().map(|&x| (x - max).exp()).collect();
        let sum: f64 = exps.iter().sum();
        self.last_output = exps.iter().map(|&e| e / sum).collect();
        self.last_output.clone()
    }

    /// Pass-through gradient (correct when paired with cross-entropy loss
    /// whose gradient is already `softmax_output − target`).
    fn backward(&mut self, grad: &[f64]) -> Vec<f64> {
        grad.to_vec()
    }

    fn params(&self) -> Vec<f64> { Vec::new() }
    fn grads(&self) -> Vec<f64> { Vec::new() }
    fn update(&mut self, _lr: f64) {}
}

// ── Dropout Layer ─────────────────────────────────────────────────────────────

/// Inverted dropout regularisation layer.
///
/// During training a deterministic, index-based mask is applied:
/// `mask[i] = 0.0` if `i % rate_inv == 0`, otherwise `1.0 / (1.0 − rate)`,
/// where `rate_inv = round(1.0 / rate)`.  This gives reproducible behaviour
/// without a PRNG, matching the deterministic spirit of the rest of the codebase.
///
/// During inference (`training = false`) the layer is an identity.
pub struct Dropout {
    pub rate: f64,
    mask: Vec<f64>,
    training: bool,
}

impl Dropout {
    pub fn new(rate: f64) -> Self {
        Dropout { rate, mask: Vec::new(), training: true }
    }

    /// Switch between training and inference mode.
    pub fn set_training(&mut self, training: bool) {
        self.training = training;
    }
}

impl Layer for Dropout {
    fn forward(&mut self, inputs: &[f64]) -> Vec<f64> {
        if !self.training {
            return inputs.to_vec();
        }

        let rate_inv = (1.0 / self.rate).round() as usize;
        let rate_inv = rate_inv.max(1); // guard against divide-by-zero
        let scale = 1.0 / (1.0 - self.rate);

        self.mask = (0..inputs.len())
            .map(|i| if i % rate_inv == 0 { 0.0 } else { scale })
            .collect();

        inputs.iter().zip(self.mask.iter()).map(|(&x, &m)| x * m).collect()
    }

    fn backward(&mut self, grad: &[f64]) -> Vec<f64> {
        if !self.training || self.mask.is_empty() {
            return grad.to_vec();
        }
        grad.iter().zip(self.mask.iter()).map(|(&g, &m)| g * m).collect()
    }

    fn params(&self) -> Vec<f64> { Vec::new() }
    fn grads(&self) -> Vec<f64> { Vec::new() }
    fn update(&mut self, _lr: f64) {}
}

// ── Loss functions ────────────────────────────────────────────────────────────

/// Mean-squared error: (1/n) Σ (predicted[i] − target[i])².
pub fn mse_loss(predicted: &[f64], targets: &[f64]) -> f64 {
    let n = predicted.len().min(targets.len());
    if n == 0 { return 0.0; }
    let sum: f64 = predicted.iter().zip(targets.iter()).map(|(&p, &t)| (p - t).powi(2)).sum();
    sum / n as f64
}

/// Gradient of MSE loss w.r.t. `predicted`: 2(predicted[i] − target[i]) / n.
pub fn mse_grad(predicted: &[f64], targets: &[f64]) -> Vec<f64> {
    let n = predicted.len().min(targets.len());
    if n == 0 { return Vec::new(); }
    let scale = 2.0 / n as f64;
    predicted.iter().zip(targets.iter()).map(|(&p, &t)| scale * (p - t)).collect()
}

/// Binary cross-entropy: −(1/n) Σ [t·log(p) + (1−t)·log(1−p)], clamped to avoid log(0).
pub fn binary_cross_entropy(predicted: &[f64], targets: &[f64]) -> f64 {
    let n = predicted.len().min(targets.len());
    if n == 0 { return 0.0; }
    let eps = 1e-15_f64;
    let sum: f64 = predicted.iter().zip(targets.iter()).map(|(&p, &t)| {
        let p = p.clamp(eps, 1.0 - eps);
        -(t * p.ln() + (1.0 - t) * (1.0 - p).ln())
    }).sum();
    sum / n as f64
}

/// Gradient of binary cross-entropy w.r.t. `predicted`:
/// (1/n) · (−t/p + (1−t)/(1−p)), clamped.
pub fn binary_cross_entropy_grad(predicted: &[f64], targets: &[f64]) -> Vec<f64> {
    let n = predicted.len().min(targets.len());
    if n == 0 { return Vec::new(); }
    let eps = 1e-15_f64;
    let scale = 1.0 / n as f64;
    predicted.iter().zip(targets.iter()).map(|(&p, &t)| {
        let p = p.clamp(eps, 1.0 - eps);
        scale * (-(t / p) + (1.0 - t) / (1.0 - p))
    }).collect()
}

// ── Sequential Network ────────────────────────────────────────────────────────

/// A stack of `Layer`s executed in order during the forward pass and in
/// reverse during the backward pass.
pub struct Sequential {
    pub layers: Vec<Box<dyn Layer>>,
}

impl Sequential {
    pub fn new() -> Self {
        Sequential { layers: Vec::new() }
    }

    /// Append a layer to the end of the stack.
    pub fn add(&mut self, layer: Box<dyn Layer>) {
        self.layers.push(layer);
    }

    /// Run all layers in order, returning the final output.
    pub fn forward(&mut self, input: &[f64]) -> Vec<f64> {
        let mut current = input.to_vec();
        for layer in self.layers.iter_mut() {
            current = layer.forward(&current);
        }
        current
    }

    /// Backpropagate `grad` through all layers in reverse order.
    pub fn backward(&mut self, grad: &[f64]) {
        let mut current = grad.to_vec();
        for layer in self.layers.iter_mut().rev() {
            current = layer.backward(&current);
        }
    }

    /// Apply one SGD step to every layer.
    pub fn update(&mut self, lr: f64) {
        for layer in self.layers.iter_mut() {
            layer.update(lr);
        }
    }

    /// Train on binary classification data using sigmoid output and MSE loss.
    ///
    /// Each sample is forward-passed, loss gradient is computed w.r.t. the
    /// scalar sigmoid output, and backpropagation updates all layer parameters.
    pub fn train_binary(
        &mut self,
        train: &[Vec<f64>],
        labels: &[bool],
        lr: f64,
        epochs: usize,
    ) {
        let n = train.len().min(labels.len());
        for _ in 0..epochs {
            for idx in 0..n {
                let output = self.forward(&train[idx]);
                let target = if labels[idx] { 1.0 } else { 0.0 };
                // Scalar MSE gradient for a single output neuron.
                let grad = mse_grad(&output, &[target]);
                self.backward(&grad);
                self.update(lr);
            }
        }
    }

    /// Predict binary labels by thresholding the first output neuron at 0.5.
    pub fn predict_binary(&mut self, test: &[Vec<f64>]) -> Vec<bool> {
        test.iter()
            .map(|x| {
                let out = self.forward(x);
                out.first().copied().unwrap_or(0.0) >= 0.5
            })
            .collect()
    }
}

impl Default for Sequential {
    fn default() -> Self {
        Self::new()
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── 1. Activation functions ───────────────────────────────────────────────

    #[test]
    fn test_activation_functions() {
        // sigmoid
        let s = sigmoid(0.0);
        assert!((s - 0.5).abs() < 1e-10, "sigmoid(0) should be 0.5, got {s}");
        let s1 = sigmoid(1.0);
        assert!(s1 > 0.5 && s1 < 1.0);

        // sigmoid_grad
        let g = sigmoid_grad(0.5);
        assert!((g - 0.25).abs() < 1e-10, "sigmoid_grad(0.5) = 0.25");

        // tanh
        let t = tanh_act(0.0);
        assert!(t.abs() < 1e-10, "tanh(0) = 0");
        let tg = tanh_grad(0.0);
        assert!((tg - 1.0).abs() < 1e-10, "tanh_grad(0) = 1");

        // relu
        assert!((relu(3.0) - 3.0).abs() < 1e-10);
        assert!(relu(-1.0).abs() < 1e-10);
        assert!((relu_grad(1.0) - 1.0).abs() < 1e-10);
        assert!(relu_grad(-1.0).abs() < 1e-10);
    }

    // ── 2. Forward pass shape for Linear ─────────────────────────────────────

    #[test]
    fn test_linear_forward_shape() {
        let mut layer = Linear::new(3, 5);
        let out = layer.forward(&[0.1, 0.2, 0.3]);
        assert_eq!(out.len(), 5, "output size should match output_size");
    }

    // ── 3. Backward pass for Linear ──────────────────────────────────────────

    #[test]
    fn test_linear_backward() {
        let mut layer = Linear::new(2, 3);
        let input = [1.0, 2.0];
        let _ = layer.forward(&input);

        // Upstream gradient of shape [output_size=3]
        let upstream = [1.0, 0.0, -1.0];
        let grad_input = layer.backward(&upstream);

        // grad_input[j] = Σ_i weights[i][j] * upstream[i]
        let expected: Vec<f64> = (0..2)
            .map(|j| {
                layer.weights.iter().zip(upstream.iter()).map(|(row, &g)| row[j] * g).sum::<f64>()
            })
            .collect();

        assert_eq!(grad_input.len(), 2);
        for (got, exp) in grad_input.iter().zip(expected.iter()) {
            assert!((got - exp).abs() < 1e-10, "grad_input mismatch: {got} vs {exp}");
        }

        // weight_grads[i][j] should equal upstream[i] * input[j]
        for (i, row) in layer.weight_grads.iter().enumerate() {
            for (j, &wg) in row.iter().enumerate() {
                let expected_wg = upstream[i] * input[j];
                assert!(
                    (wg - expected_wg).abs() < 1e-10,
                    "weight_grads[{i}][{j}] = {wg}, expected {expected_wg}"
                );
            }
        }
    }

    // ── 4. Sigmoid layer round-trip ───────────────────────────────────────────

    #[test]
    fn test_sigmoid_layer() {
        let mut layer = Sigmoid::new();
        let inputs = [0.0, 1.0, -1.0, 2.0];
        let out = layer.forward(&inputs);

        assert_eq!(out.len(), 4);
        for (&x, &s) in inputs.iter().zip(out.iter()) {
            let expected = sigmoid(x);
            assert!((s - expected).abs() < 1e-10, "sigmoid mismatch at x={x}");
        }

        // Backward: grad_in[i] = grad[i] * sigmoid_grad(out[i])
        let grad = [1.0, 1.0, 1.0, 1.0];
        let grad_in = layer.backward(&grad);
        for (&s, &gi) in out.iter().zip(grad_in.iter()) {
            let expected = sigmoid_grad(s);
            assert!((gi - expected).abs() < 1e-10, "sigmoid backward mismatch");
        }
    }

    // ── 5. Sequential train on linearly-separable data ───────────────────────

    #[test]
    fn test_sequential_train_binary() {
        // OR gate — linearly separable.
        let train = vec![
            vec![0.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
        ];
        let labels = vec![false, true, true, true];

        let mut net = Sequential::new();
        net.add(Box::new(Linear::new(2, 4)));
        net.add(Box::new(Sigmoid::new()));
        net.add(Box::new(Linear::new(4, 1)));
        net.add(Box::new(Sigmoid::new()));

        net.train_binary(&train, &labels, 0.5, 3000);
        let preds = net.predict_binary(&train);
        assert_eq!(preds, labels, "network should learn OR gate after training");
    }

    // ── 6. Loss functions ─────────────────────────────────────────────────────

    #[test]
    fn test_loss_functions() {
        let predicted = [0.8, 0.2, 0.6];
        let targets = [1.0, 0.0, 1.0];

        // MSE
        let mse = mse_loss(&predicted, &targets);
        let expected_mse = ((0.8 - 1.0f64).powi(2) + (0.2 - 0.0f64).powi(2) + (0.6 - 1.0f64).powi(2)) / 3.0;
        assert!((mse - expected_mse).abs() < 1e-10, "mse_loss mismatch: {mse}");

        // MSE gradient
        let grad = mse_grad(&predicted, &targets);
        assert_eq!(grad.len(), 3);
        let expected_g0 = 2.0 / 3.0 * (0.8 - 1.0);
        assert!((grad[0] - expected_g0).abs() < 1e-10, "mse_grad[0] mismatch");

        // BCE is non-negative and finite
        let bce = binary_cross_entropy(&predicted, &targets);
        assert!(bce >= 0.0 && bce.is_finite(), "BCE must be non-negative and finite, got {bce}");

        // BCE gradient has correct length
        let bce_grad = binary_cross_entropy_grad(&predicted, &targets);
        assert_eq!(bce_grad.len(), 3);
    }

    // ── 7. Dropout mask shape and inference identity ──────────────────────────

    #[test]
    fn test_dropout() {
        let mut drop = Dropout::new(0.5); // rate_inv = 2 → mask[0]=0, mask[1]=scale, ...
        let inputs = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

        // Training mode: some inputs are zeroed.
        let out_train = drop.forward(&inputs);
        assert_eq!(out_train.len(), inputs.len());
        // Index 0 should be zeroed (0 % 2 == 0).
        assert!(out_train[0].abs() < 1e-10, "dropout should zero index 0");
        // Index 1 should be scaled (non-zero since input is non-zero).
        assert!(out_train[1].abs() > 1e-10, "dropout should scale index 1");

        // Inference mode: identity.
        drop.set_training(false);
        let out_infer = drop.forward(&inputs);
        for (&x, &o) in inputs.iter().zip(out_infer.iter()) {
            assert!((x - o).abs() < 1e-10, "dropout inference should be identity");
        }
    }

    // ── 8. Softmax output sums to 1 ───────────────────────────────────────────

    #[test]
    fn test_softmax_sums_to_one() {
        let mut sm = Softmax::new();
        let inputs = [1.0, 2.0, 3.0, 4.0];
        let out = sm.forward(&inputs);
        let sum: f64 = out.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10, "softmax should sum to 1, got {sum}");
        for &p in &out {
            assert!(p >= 0.0 && p <= 1.0, "softmax output must be in [0,1]");
        }
    }

    // ── 9. ReLU and Tanh layers ───────────────────────────────────────────────

    #[test]
    fn test_relu_layer() {
        let mut layer = Relu::new();
        let inputs = [-2.0, -1.0, 0.0, 1.0, 2.0];
        let out = layer.forward(&inputs);
        assert_eq!(out, vec![0.0, 0.0, 0.0, 1.0, 2.0]);

        let grad = [1.0; 5];
        let grad_in = layer.backward(&grad);
        // ReLU sub-gradient: 0 for x <= 0, 1 for x > 0.
        assert_eq!(grad_in, vec![0.0, 0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn test_tanh_layer() {
        let mut layer = Tanh::new();
        let inputs = [0.0, 1.0, -1.0];
        let out = layer.forward(&inputs);
        assert!((out[0]).abs() < 1e-10, "tanh(0) = 0");

        let grad = [1.0; 3];
        let grad_in = layer.backward(&grad);
        // At tanh(0) = 0, gradient is 1 - 0^2 = 1.
        assert!((grad_in[0] - 1.0).abs() < 1e-10, "tanh backward at 0 should be 1");
    }
}
