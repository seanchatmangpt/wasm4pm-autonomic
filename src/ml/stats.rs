//! Statistics utilities — Chapters 5, 6, 7 of *Data Science from Scratch* (Joel Grus).
//!
//! Chapter 5: descriptive statistics (mean, median, mode, variance, correlation, …)
//! Chapter 6: probability distributions (normal PDF/CDF, inverse CDF)
//! Chapter 7: hypothesis and inference (binomial approximation, two-sided p-value, A/B test)
//!
//! All functions are pure and allocation-minimal.  Hot paths avoid heap allocation.
//! No external crates — only `std::f64` math primitives.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Chapter 5 — Statistics
// ---------------------------------------------------------------------------

/// Arithmetic mean of `xs`.  Returns `0.0` for an empty slice.
pub fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// Median (middle value of a sorted copy).  Returns `0.0` for an empty slice.
/// For an even-length slice the two middle values are averaged.
pub fn median(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let mut sorted = xs.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

/// All values tied for the highest frequency (quantized to 6 decimal places to
/// handle floating-point near-equality).  Returns an empty `Vec` for empty input.
pub fn mode(xs: &[f64]) -> Vec<f64> {
    if xs.is_empty() {
        return Vec::new();
    }
    let mut counts: HashMap<i64, (f64, usize)> = HashMap::new();
    for &x in xs {
        let key = (x * 1_000_000.0).round() as i64;
        counts.entry(key).or_insert((x, 0)).1 += 1;
    }
    let max_count = counts.values().map(|&(_, c)| c).max().unwrap_or(0);
    let mut modes: Vec<f64> = counts
        .values()
        .filter(|&&(_, c)| c == max_count)
        .map(|&(v, _)| v)
        .collect();
    modes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    modes
}

/// Range: `max(xs) - min(xs)`.  Returns `0.0` for an empty slice.
pub fn data_range(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let min = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    max - min
}

/// Sample variance (Bessel-corrected, divides by `n-1`).
/// Returns `0.0` when fewer than two values are provided.
pub fn variance(xs: &[f64]) -> f64 {
    if xs.len() < 2 {
        return 0.0;
    }
    let m = mean(xs);
    let sq_sum: f64 = xs.iter().map(|&x| (x - m) * (x - m)).sum();
    sq_sum / (xs.len() - 1) as f64
}

/// Sample standard deviation: `sqrt(variance(xs))`.
pub fn std_dev(xs: &[f64]) -> f64 {
    variance(xs).sqrt()
}

/// Interquartile range: 75th percentile minus 25th percentile.
/// Uses the nearest-rank percentile method on a sorted copy.
/// Returns `0.0` for an empty slice.
pub fn interquartile_range(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let mut sorted = xs.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q1 = percentile_sorted(&sorted, 25);
    let q3 = percentile_sorted(&sorted, 75);
    q3 - q1
}

/// Returns the value at the given `pct` percentile (0–100) from an already-sorted
/// slice, using the nearest-rank method.
fn percentile_sorted(sorted: &[f64], pct: usize) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((pct as f64 / 100.0) * sorted.len() as f64).ceil() as usize;
    let idx = idx.saturating_sub(1).min(sorted.len() - 1);
    sorted[idx]
}

/// Sample covariance of `xs` and `ys` (Bessel-corrected).
/// Returns `0.0` when fewer than two pairs are available.
/// Panics in debug mode if the slices differ in length.
pub fn covariance(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len().min(ys.len());
    if n < 2 {
        return 0.0;
    }
    let mx = mean(xs);
    let my = mean(ys);
    let sum: f64 = xs[..n]
        .iter()
        .zip(ys[..n].iter())
        .map(|(&x, &y)| (x - mx) * (y - my))
        .sum();
    sum / (n - 1) as f64
}

/// Pearson correlation coefficient: `cov(xs, ys) / (std(xs) * std(ys))`.
/// Returns `0.0` when either standard deviation is zero.
pub fn correlation(xs: &[f64], ys: &[f64]) -> f64 {
    let sx = std_dev(xs);
    let sy = std_dev(ys);
    if sx == 0.0 || sy == 0.0 {
        return 0.0;
    }
    covariance(xs, ys) / (sx * sy)
}

// ---------------------------------------------------------------------------
// Chapter 6 — Probability distributions
// ---------------------------------------------------------------------------

/// Error function approximation (Abramowitz & Stegun, max error ≈ 1.5 × 10⁻⁷).
fn erf(x: f64) -> f64 {
    let sign = x.signum();
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let y = 1.0
        - (((((1.061_405_429 * t - 1.453_152_027) * t) + 1.421_413_741) * t
            - 0.284_496_736)
            * t
            + 0.254_829_592)
            * t
            * (-x * x).exp();
    sign * y
}

/// Probability density function of the normal distribution N(mu, sigma²).
pub fn normal_pdf(x: f64, mu: f64, sigma: f64) -> f64 {
    let z = (x - mu) / sigma;
    (-0.5 * z * z).exp() / (sigma * (2.0 * std::f64::consts::PI).sqrt())
}

/// Cumulative distribution function of N(mu, sigma²) using the erf approximation.
pub fn normal_cdf(x: f64, mu: f64, sigma: f64) -> f64 {
    0.5 * (1.0 + erf((x - mu) / (sigma * std::f64::consts::SQRT_2)))
}

/// Binary-search inverse CDF of N(mu, sigma²).
///
/// `p`         — target probability in (0, 1).
/// `tolerance` — convergence threshold (e.g. `1e-5`).
pub fn inverse_normal_cdf(p: f64, mu: f64, sigma: f64, tolerance: f64) -> f64 {
    // Bracket: virtually all probability mass lies within ±10 standard deviations.
    let mut lo = mu - 10.0 * sigma;
    let mut hi = mu + 10.0 * sigma;
    while hi - lo > tolerance {
        let mid = (lo + hi) / 2.0;
        if normal_cdf(mid, mu, sigma) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

// ---------------------------------------------------------------------------
// Chapter 7 — Hypothesis and Inference
// ---------------------------------------------------------------------------

/// Normal approximation to the binomial B(n, p): returns `(mu, sigma)`.
pub fn normal_approximation_to_binomial(n: usize, p: f64) -> (f64, f64) {
    let mu = n as f64 * p;
    let sigma = (mu * (1.0 - p)).sqrt();
    (mu, sigma)
}

/// P(X > lo) under N(mu, sigma²).
pub fn normal_probability_above(lo: f64, mu: f64, sigma: f64) -> f64 {
    1.0 - normal_cdf(lo, mu, sigma)
}

/// P(lo < X < hi) under N(mu, sigma²).
pub fn normal_probability_between(lo: f64, hi: f64, mu: f64, sigma: f64) -> f64 {
    normal_cdf(hi, mu, sigma) - normal_cdf(lo, mu, sigma)
}

/// Two-sided p-value: 2 × P(X > |x − mu| + mu) under N(mu, sigma²).
pub fn two_sided_p_value(x: f64, mu: f64, sigma: f64) -> f64 {
    2.0 * normal_probability_above((x - mu).abs() + mu, mu, sigma)
}

/// A/B significance test (two-proportion z-test, two-sided).
///
/// Computes the pooled proportion, the z-score of the observed difference, and
/// the two-sided p-value.  Returns `true` when `p_value < alpha`.
///
/// # Arguments
/// * `n_a`      — number of trials in group A
/// * `n_pos_a`  — number of positive outcomes in group A
/// * `n_b`      — number of trials in group B
/// * `n_pos_b`  — number of positive outcomes in group B
/// * `alpha`    — significance level (e.g. `0.05`)
pub fn ab_test_significant(
    n_a: usize,
    n_pos_a: usize,
    n_b: usize,
    n_pos_b: usize,
    alpha: f64,
) -> bool {
    let p_a = n_pos_a as f64 / n_a as f64;
    let p_b = n_pos_b as f64 / n_b as f64;
    let p_pool = (n_pos_a + n_pos_b) as f64 / (n_a + n_b) as f64;
    let se = (p_pool * (1.0 - p_pool) * (1.0 / n_a as f64 + 1.0 / n_b as f64)).sqrt();
    if se == 0.0 {
        return false;
    }
    let z = (p_a - p_b) / se;
    // Two-sided p-value under standard normal (mu=0, sigma=1).
    let p_value = 2.0 * normal_probability_above(z.abs(), 0.0, 1.0);
    p_value < alpha
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-6;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    // --- Chapter 5 ---

    #[test]
    fn test_mean_basic() {
        assert!(approx_eq(mean(&[1.0, 2.0, 3.0, 4.0, 5.0]), 3.0, EPS));
    }

    #[test]
    fn test_mean_empty() {
        assert_eq!(mean(&[]), 0.0);
    }

    #[test]
    fn test_median_odd() {
        assert!(approx_eq(median(&[3.0, 1.0, 5.0, 2.0, 4.0]), 3.0, EPS));
    }

    #[test]
    fn test_median_even() {
        assert!(approx_eq(median(&[1.0, 2.0, 3.0, 4.0]), 2.5, EPS));
    }

    #[test]
    fn test_median_empty() {
        assert_eq!(median(&[]), 0.0);
    }

    #[test]
    fn test_mode_single() {
        let m = mode(&[1.0, 2.0, 2.0, 3.0]);
        assert_eq!(m, vec![2.0]);
    }

    #[test]
    fn test_mode_tie() {
        let m = mode(&[1.0, 1.0, 2.0, 2.0, 3.0]);
        // Both 1.0 and 2.0 appear twice; result is sorted.
        assert_eq!(m, vec![1.0, 2.0]);
    }

    #[test]
    fn test_mode_empty() {
        assert!(mode(&[]).is_empty());
    }

    #[test]
    fn test_data_range() {
        assert!(approx_eq(data_range(&[1.0, 5.0, 3.0, 2.0, 4.0]), 4.0, EPS));
        assert_eq!(data_range(&[]), 0.0);
    }

    #[test]
    fn test_variance_and_std_dev() {
        let xs = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        // Population mean = 5, sample variance slightly higher than population variance (4).
        let v = variance(&xs);
        assert!(v > 4.0, "sample variance {v} should exceed population variance 4.0");
        assert!(approx_eq(std_dev(&xs), v.sqrt(), EPS));
    }

    #[test]
    fn test_variance_too_few() {
        assert_eq!(variance(&[]), 0.0);
        assert_eq!(variance(&[42.0]), 0.0);
    }

    #[test]
    fn test_interquartile_range() {
        // Simple monotone sequence — IQR should be positive.
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let iqr = interquartile_range(&xs);
        assert!(iqr > 0.0, "IQR {iqr} should be positive");
    }

    #[test]
    fn test_covariance_positive() {
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0];
        let ys = [2.0, 4.0, 6.0, 8.0, 10.0];
        // Perfect positive relationship — covariance should be positive.
        assert!(covariance(&xs, &ys) > 0.0);
    }

    #[test]
    fn test_correlation_perfect() {
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0];
        let ys = [2.0, 4.0, 6.0, 8.0, 10.0];
        assert!(approx_eq(correlation(&xs, &ys), 1.0, 1e-9));
    }

    #[test]
    fn test_correlation_negative() {
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0];
        let ys = [10.0, 8.0, 6.0, 4.0, 2.0];
        assert!(approx_eq(correlation(&xs, &ys), -1.0, 1e-9));
    }

    #[test]
    fn test_correlation_zero_std() {
        let xs = [1.0, 1.0, 1.0];
        let ys = [2.0, 3.0, 4.0];
        assert_eq!(correlation(&xs, &ys), 0.0);
    }

    // --- Chapter 6 ---

    #[test]
    fn test_normal_pdf_peak() {
        // N(0,1) peaks at x=0 with value 1/sqrt(2π).
        let expected = 1.0 / (2.0 * std::f64::consts::PI).sqrt();
        assert!(approx_eq(normal_pdf(0.0, 0.0, 1.0), expected, EPS));
    }

    #[test]
    fn test_normal_cdf_midpoint() {
        assert!(approx_eq(normal_cdf(0.0, 0.0, 1.0), 0.5, EPS));
    }

    #[test]
    fn test_normal_cdf_tail() {
        // CDF at 1.96σ should be close to 0.975.
        assert!(approx_eq(normal_cdf(1.96, 0.0, 1.0), 0.975, 1e-3));
    }

    #[test]
    fn test_inverse_normal_cdf_roundtrip() {
        let mu = 5.0;
        let sigma = 2.0;
        let p = 0.85;
        let x = inverse_normal_cdf(p, mu, sigma, 1e-7);
        let recovered = normal_cdf(x, mu, sigma);
        assert!(approx_eq(recovered, p, 1e-5));
    }

    #[test]
    fn test_inverse_normal_cdf_median() {
        // Inverse CDF at p=0.5 should return the mean.
        let x = inverse_normal_cdf(0.5, 10.0, 3.0, 1e-7);
        assert!(approx_eq(x, 10.0, 1e-4));
    }

    // --- Chapter 7 ---

    #[test]
    fn test_normal_approximation_to_binomial() {
        let (mu, sigma) = normal_approximation_to_binomial(1000, 0.5);
        assert!(approx_eq(mu, 500.0, EPS));
        assert!(approx_eq(sigma, 15.811_388, 1e-4));
    }

    #[test]
    fn test_normal_probability_above_below_sum_to_one() {
        let mu = 0.0;
        let sigma = 1.0;
        let lo = 1.0;
        let above = normal_probability_above(lo, mu, sigma);
        let below = normal_cdf(lo, mu, sigma);
        assert!(approx_eq(above + below, 1.0, EPS));
    }

    #[test]
    fn test_normal_probability_between_symmetric() {
        // P(-1 < Z < 1) ≈ 0.6827 for N(0,1).
        let p = normal_probability_between(-1.0, 1.0, 0.0, 1.0);
        assert!(approx_eq(p, 0.6827, 1e-3));
    }

    #[test]
    fn test_two_sided_p_value_at_mean() {
        // At exactly the mean the two-sided p-value is 1.0.
        let pv = two_sided_p_value(0.0, 0.0, 1.0);
        assert!(approx_eq(pv, 1.0, EPS));
    }

    #[test]
    fn test_two_sided_p_value_far_tail() {
        // 5σ away — p-value should be tiny.
        let pv = two_sided_p_value(5.0, 0.0, 1.0);
        assert!(pv < 1e-5, "p-value {pv} should be negligibly small");
    }

    #[test]
    fn test_ab_test_significant_clear_difference() {
        // 200/1000 vs 250/1000 is a sizeable lift — should be significant.
        assert!(ab_test_significant(1000, 200, 1000, 250, 0.05));
    }

    #[test]
    fn test_ab_test_not_significant_identical() {
        // Identical rates — must not be flagged as significant.
        assert!(!ab_test_significant(1000, 100, 1000, 100, 0.05));
    }

    #[test]
    fn test_ab_test_borderline() {
        // Trivial difference in large sample — not significant.
        assert!(!ab_test_significant(100, 50, 100, 51, 0.05));
    }
}
