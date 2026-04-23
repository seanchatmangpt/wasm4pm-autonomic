//! Gradient Descent utilities — Chapter 8 of *Data Science from Scratch* (Grus).
//!
//! All functions are pure and allocation-free on hot paths where possible.
//! No external crates are used.

// ─── Finite differences ──────────────────────────────────────────────────────

/// Compute the derivative of `f` at `x` via the symmetric difference quotient.
///
/// `h` should be a small positive value (e.g. `1e-5`).
/// Returns `(f(x+h) - f(x-h)) / (2h)`.
pub fn difference_quotient<F: Fn(f64) -> f64>(f: F, x: f64, h: f64) -> f64 {
    if h == 0.0 {
        return 0.0;
    }
    (f(x + h) - f(x - h)) / (2.0 * h)
}

/// Partial derivative of `f : ℝⁿ → ℝ` with respect to dimension `i`,
/// evaluated at point `v`, using step size `h`.
///
/// Panics if `i >= v.len()`.
pub fn partial_difference_quotient<F: Fn(&[f64]) -> f64>(
    f: F,
    v: &[f64],
    i: usize,
    h: f64,
) -> f64 {
    if v.is_empty() || h == 0.0 {
        return 0.0;
    }
    assert!(i < v.len(), "index {i} out of bounds for v.len()={}", v.len());

    let mut w = v.to_vec();
    w[i] = v[i] + h;
    let fwd = f(&w);
    w[i] = v[i] - h;
    let bwd = f(&w);
    (fwd - bwd) / (2.0 * h)
}

/// Estimate the full gradient of `f : ℝⁿ → ℝ` at `v` via finite differences.
///
/// Returns a `Vec<f64>` of length `v.len()`.
/// Returns an empty `Vec` when `v` is empty.
pub fn estimate_gradient<F: Fn(&[f64]) -> f64>(f: F, v: &[f64], h: f64) -> Vec<f64> {
    (0..v.len())
        .map(|i| partial_difference_quotient(&f, v, i, h))
        .collect()
}

// ─── Core descent primitives ──────────────────────────────────────────────────

/// Move one gradient-descent step: returns `v - step_size * gradient`.
///
/// `v` and `gradient` must have the same length; the result has the same length.
/// Returns an empty `Vec` when inputs are empty.
pub fn gradient_step(v: &[f64], gradient: &[f64], step_size: f64) -> Vec<f64> {
    v.iter()
        .zip(gradient.iter())
        .map(|(vi, gi)| vi - step_size * gi)
        .collect()
}

// ─── Batch descent ───────────────────────────────────────────────────────────

/// Batch gradient descent: minimise `f(v)` using the supplied analytical
/// gradient function.
///
/// Stops early when the Euclidean norm of the step is less than `tolerance`,
/// or after `max_iters` iterations — whichever comes first.
///
/// Returns the minimiser found.
pub fn minimize<G: Fn(&[f64]) -> Vec<f64>>(
    initial: &[f64],
    gradient_fn: G,
    step_size: f64,
    max_iters: usize,
    tolerance: f64,
) -> Vec<f64> {
    if initial.is_empty() || step_size == 0.0 {
        return initial.to_vec();
    }

    let mut v = initial.to_vec();

    for _ in 0..max_iters {
        let grad = gradient_fn(&v);
        let next = gradient_step(&v, &grad, step_size);

        // Euclidean norm of the step taken
        let step_norm: f64 = next
            .iter()
            .zip(v.iter())
            .map(|(n, o)| (n - o).powi(2))
            .sum::<f64>()
            .sqrt();

        v = next;

        if step_norm < tolerance {
            break;
        }
    }

    v
}

// ─── Stochastic descent ───────────────────────────────────────────────────────

/// Stochastic gradient descent: one gradient update per training example,
/// cycling through `data` for `epochs` full passes.
///
/// `gradient_fn(params, x_i, y_i)` returns the gradient of the loss for a
/// single example.
///
/// Returns the parameter vector after all updates.
pub fn stochastic_minimize<X, Y, G>(
    initial: &[f64],
    data: &[(X, Y)],
    gradient_fn: G,
    step_size: f64,
    epochs: usize,
) -> Vec<f64>
where
    G: Fn(&[f64], &X, &Y) -> Vec<f64>,
{
    if initial.is_empty() || data.is_empty() || step_size == 0.0 {
        return initial.to_vec();
    }

    let mut params = initial.to_vec();

    for _ in 0..epochs {
        for (x, y) in data.iter() {
            let grad = gradient_fn(&params, x, y);
            params = gradient_step(&params, &grad, step_size);
        }
    }

    params
}

// ─── Mini-batch descent ───────────────────────────────────────────────────────

/// Mini-batch gradient descent: split `data` into chunks of `batch_size`
/// (deterministic order, no shuffle), apply `gradient_fn` to each chunk,
/// and step the parameters for `epochs` full passes over `data`.
///
/// `gradient_fn(params, batch)` returns the gradient of the loss for that
/// mini-batch.
///
/// Returns the parameter vector after all updates.
pub fn minibatch_minimize<X, Y, G>(
    initial: &[f64],
    data: &[(X, Y)],
    gradient_fn: G,
    step_size: f64,
    batch_size: usize,
    epochs: usize,
) -> Vec<f64>
where
    G: Fn(&[f64], &[(X, Y)]) -> Vec<f64>,
{
    if initial.is_empty() || data.is_empty() || step_size == 0.0 || batch_size == 0 {
        return initial.to_vec();
    }

    let mut params = initial.to_vec();

    for _ in 0..epochs {
        for batch in data.chunks(batch_size) {
            let grad = gradient_fn(&params, batch);
            params = gradient_step(&params, &grad, step_size);
        }
    }

    params
}

// ─── Linear regression example ────────────────────────────────────────────────

/// Fit a simple linear model `ŷ = slope·x + intercept` via gradient descent,
/// minimising mean squared error over `data`.
///
/// Returns `[slope, intercept]`.
///
/// If `data` is empty, returns `[0.0, 0.0]`.
pub fn linear_regression_gd(data: &[(f64, f64)], lr: f64, epochs: usize) -> [f64; 2] {
    if data.is_empty() {
        return [0.0, 0.0];
    }

    let n = data.len() as f64;
    let mut slope = 0.0_f64;
    let mut intercept = 0.0_f64;

    for _ in 0..epochs {
        let mut d_slope = 0.0_f64;
        let mut d_intercept = 0.0_f64;

        for &(x, y) in data.iter() {
            let pred = slope * x + intercept;
            let err = pred - y;
            // ∂MSE/∂slope = (2/n) * err * x  — factor of 2 absorbed into lr
            d_slope += err * x;
            d_intercept += err;
        }

        slope -= lr * (2.0 / n) * d_slope;
        intercept -= lr * (2.0 / n) * d_intercept;
    }

    [slope, intercept]
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-6;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    // 1. difference_quotient: derivative of x² at x=3 should be ≈6.
    #[test]
    fn test_difference_quotient_square() {
        let deriv = difference_quotient(|x| x * x, 3.0, 1e-5);
        assert!(
            approx_eq(deriv, 6.0, 1e-8),
            "expected ≈6.0, got {deriv}"
        );
    }

    // 2. difference_quotient: derivative of sin(x) at 0 should be ≈1.
    #[test]
    fn test_difference_quotient_sin() {
        let deriv = difference_quotient(f64::sin, 0.0, 1e-5);
        assert!(approx_eq(deriv, 1.0, 1e-8), "expected ≈1.0, got {deriv}");
    }

    // 3. partial_difference_quotient: ∂/∂x₁ of (x₀² + x₁²) at [1,2] ≈ 4.
    #[test]
    fn test_partial_difference_quotient() {
        let f = |v: &[f64]| v[0] * v[0] + v[1] * v[1];
        let v = [1.0_f64, 2.0];
        let pd = partial_difference_quotient(f, &v, 1, 1e-5);
        assert!(approx_eq(pd, 4.0, 1e-8), "expected ≈4.0, got {pd}");
    }

    // 4. estimate_gradient: full gradient of ‖v‖² at [1,2,3] should be ≈[2,4,6].
    #[test]
    fn test_estimate_gradient() {
        let f = |v: &[f64]| v.iter().map(|x| x * x).sum::<f64>();
        let v = [1.0_f64, 2.0, 3.0];
        let grad = estimate_gradient(f, &v, 1e-5);
        let expected = [2.0, 4.0, 6.0];
        for (g, e) in grad.iter().zip(expected.iter()) {
            assert!(approx_eq(*g, *e, 1e-7), "got {g}, expected {e}");
        }
    }

    // 5. gradient_step: one step of descent on v=[3,4] with grad=[1,1], lr=0.5
    //    should give [2.5, 3.5].
    #[test]
    fn test_gradient_step() {
        let v = [3.0_f64, 4.0];
        let grad = [1.0_f64, 1.0];
        let next = gradient_step(&v, &grad, 0.5);
        assert!(approx_eq(next[0], 2.5, EPS), "v[0]={}", next[0]);
        assert!(approx_eq(next[1], 3.5, EPS), "v[1]={}", next[1]);
    }

    // 6. minimize: minimise f(x) = (x-5)² starting from x=0.
    //    Analytical gradient: 2(x-5).  Minimiser should be near 5.
    #[test]
    fn test_minimize_quadratic() {
        let result = minimize(
            &[0.0],
            |v| vec![2.0 * (v[0] - 5.0)],
            0.1,
            10_000,
            1e-9,
        );
        assert!(
            approx_eq(result[0], 5.0, 1e-4),
            "expected ≈5.0, got {}",
            result[0]
        );
    }

    // 7. minimize: minimise f(x,y) = x² + y² (bowl), should converge to (0,0).
    #[test]
    fn test_minimize_bowl() {
        let result = minimize(
            &[3.0, -4.0],
            |v| vec![2.0 * v[0], 2.0 * v[1]],
            0.1,
            10_000,
            1e-9,
        );
        assert!(approx_eq(result[0], 0.0, 1e-4), "x={}", result[0]);
        assert!(approx_eq(result[1], 0.0, 1e-4), "y={}", result[1]);
    }

    // 8. linear_regression_gd: perfect linear data y = 2x + 1.
    //    Use small x range (0..5) so gradients stay bounded.
    #[test]
    fn test_linear_regression_perfect_data() {
        let data: Vec<(f64, f64)> = (0..5).map(|i| {
            let x = i as f64;
            (x, 2.0 * x + 1.0)
        }).collect();

        // lr=0.01 is safe for x in [0,4]; sum of x² ≈ 30 so step sizes stay <1.
        let [slope, intercept] = linear_regression_gd(&data, 0.01, 10_000);
        assert!(approx_eq(slope, 2.0, 0.05), "slope={slope}");
        assert!(approx_eq(intercept, 1.0, 0.05), "intercept={intercept}");
    }

    // 9. stochastic_minimize: minimise f(x) = (x-7)² via SGD.
    //    Data is just one point; gradient for MSE at a single (x,y) is 2(param·x - y)·x.
    //    Use data=[(1.0, 7.0)] so the gradient reduces to 2*(p - 7).
    #[test]
    fn test_stochastic_minimize() {
        let data = vec![(1.0_f64, 7.0_f64)];
        let result = stochastic_minimize(
            &[0.0],
            &data,
            |params, x, y| vec![2.0 * (params[0] * x - y) * x],
            0.05,
            2_000,
        );
        assert!(
            approx_eq(result[0], 7.0, 0.1),
            "expected ≈7.0, got {}",
            result[0]
        );
    }

    // 10. minibatch_minimize: two batches, same quadratic target as test 6.
    #[test]
    fn test_minibatch_minimize() {
        // Represent as per-example gradients averaged over the batch
        let data: Vec<(f64, f64)> = (0..10).map(|i| (i as f64, (i as f64 - 5.0).powi(2))).collect();

        // gradient_fn averages 2*(p - 5) over the batch (target always 5)
        let result = minibatch_minimize(
            &[0.0],
            &data,
            |params, batch| {
                let n = batch.len() as f64;
                let sum: f64 = batch.iter().map(|_| 2.0 * (params[0] - 5.0)).sum();
                vec![sum / n]
            },
            0.1,
            4,
            5_000,
        );
        assert!(
            approx_eq(result[0], 5.0, 1e-3),
            "expected ≈5.0, got {}",
            result[0]
        );
    }

    // 11. Edge cases: empty input, zero step_size.
    #[test]
    fn test_edge_cases() {
        // empty initial → pass-through
        let r = minimize(&[], |v| v.to_vec(), 0.1, 100, 1e-6);
        assert!(r.is_empty());

        // zero step_size → pass-through
        let r2 = minimize(&[3.0], |v| vec![2.0 * v[0]], 0.0, 100, 1e-6);
        assert!(approx_eq(r2[0], 3.0, EPS));

        // zero h → difference quotient returns 0
        assert_eq!(difference_quotient(|x| x * x, 3.0, 0.0), 0.0);
    }
}
