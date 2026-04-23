//! Linear algebra utilities — Chapter 4 of *Data Science from Scratch* (Joel Grus).
//!
//! All operations are standalone functions; no OOP wrappers.
//! Shorter vectors are treated as zero-padded to match the longer one.

// ── Type aliases ─────────────────────────────────────────────────────────────

pub type Vector = Vec<f64>;
pub type Matrix = Vec<Vec<f64>>;

// ── Vector operations ─────────────────────────────────────────────────────────

/// Element-wise sum of two vectors; the shorter one is zero-padded.
pub fn vec_add(a: &[f64], b: &[f64]) -> Vec<f64> {
    let len = a.len().max(b.len());
    (0..len)
        .map(|i| a.get(i).copied().unwrap_or(0.0) + b.get(i).copied().unwrap_or(0.0))
        .collect()
}

/// Element-wise difference `a − b`; the shorter one is zero-padded.
pub fn vec_sub(a: &[f64], b: &[f64]) -> Vec<f64> {
    let len = a.len().max(b.len());
    (0..len)
        .map(|i| a.get(i).copied().unwrap_or(0.0) - b.get(i).copied().unwrap_or(0.0))
        .collect()
}

/// Element-wise sum across many vectors.
/// Returns an empty vector when `vecs` is empty.
pub fn vec_sum(vecs: &[Vec<f64>]) -> Vec<f64> {
    match vecs {
        [] => vec![],
        [first, rest @ ..] => rest.iter().fold(first.clone(), |acc, v| vec_add(&acc, v)),
    }
}

/// Scalar multiplication: `c * v`.
pub fn scalar_mul(v: &[f64], c: f64) -> Vec<f64> {
    v.iter().map(|&x| c * x).collect()
}

/// Element-wise mean of many vectors.
/// Returns an empty vector when `vecs` is empty.
pub fn vec_mean(vecs: &[Vec<f64>]) -> Vec<f64> {
    if vecs.is_empty() {
        return vec![];
    }
    let n = vecs.len() as f64;
    scalar_mul(&vec_sum(vecs), 1.0 / n)
}

/// Dot product; the shorter vector is zero-padded.
pub fn dot(a: &[f64], b: &[f64]) -> f64 {
    let len = a.len().min(b.len()); // beyond min, one side is 0 ⇒ no contribution
    (0..len)
        .map(|i| a[i] * b[i])
        .sum()
}

/// Sum of squares: `dot(v, v)`.
pub fn sum_of_squares(v: &[f64]) -> f64 {
    dot(v, v)
}

/// Euclidean magnitude: `√(sum_of_squares(v))`.
pub fn magnitude(v: &[f64]) -> f64 {
    sum_of_squares(v).sqrt()
}

/// Squared Euclidean distance between `a` and `b`.
pub fn squared_distance(a: &[f64], b: &[f64]) -> f64 {
    sum_of_squares(&vec_sub(a, b))
}

/// Euclidean distance between `a` and `b`.
pub fn distance(a: &[f64], b: &[f64]) -> f64 {
    squared_distance(a, b).sqrt()
}

// ── Matrix operations ─────────────────────────────────────────────────────────

/// Shape of a matrix: `(rows, cols)`.
/// `cols` is taken from the first row; empty matrix → `(0, 0)`.
pub fn shape(m: &[Vec<f64>]) -> (usize, usize) {
    match m {
        [] => (0, 0),
        [first, ..] => (m.len(), first.len()),
    }
}

/// Clone row `i` from matrix `m`.
///
/// # Panics
/// Panics if `i >= m.len()`.
pub fn get_row(m: &[Vec<f64>], i: usize) -> Vec<f64> {
    m[i].clone()
}

/// Extract column `j` from matrix `m`.
/// Rows shorter than `j+1` contribute `0.0`.
pub fn get_col(m: &[Vec<f64>], j: usize) -> Vec<f64> {
    m.iter()
        .map(|row| row.get(j).copied().unwrap_or(0.0))
        .collect()
}

/// Build a `rows × cols` matrix whose `(i, j)` entry is `entry_fn(i, j)`.
pub fn make_matrix<F: Fn(usize, usize) -> f64>(rows: usize, cols: usize, entry_fn: F) -> Matrix {
    (0..rows)
        .map(|i| (0..cols).map(|j| entry_fn(i, j)).collect())
        .collect()
}

/// `n × n` identity matrix.
pub fn identity_matrix(n: usize) -> Matrix {
    make_matrix(n, n, |i, j| if i == j { 1.0 } else { 0.0 })
}

/// Element-wise matrix addition.
/// Matrices are treated as grids of rows; shorter rows are zero-padded.
pub fn mat_add(a: &[Vec<f64>], b: &[Vec<f64>]) -> Matrix {
    let rows = a.len().max(b.len());
    (0..rows)
        .map(|i| {
            let ra = a.get(i).map(|r| r.as_slice()).unwrap_or(&[]);
            let rb = b.get(i).map(|r| r.as_slice()).unwrap_or(&[]);
            vec_add(ra, rb)
        })
        .collect()
}

/// Standard matrix multiplication `a × b`.
/// Inner dimensions must match; ragged rows are zero-padded.
/// Result shape: `(a.rows, b.cols)` where `b.cols` comes from the first row of `b`.
///
/// Returns an empty matrix if either argument is empty.
pub fn mat_mul(a: &[Vec<f64>], b: &[Vec<f64>]) -> Matrix {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let b_cols = b[0].len();
    a.iter()
        .map(|row_a| {
            (0..b_cols)
                .map(|j| {
                    row_a
                        .iter()
                        .enumerate()
                        .map(|(k, &v)| v * b.get(k).and_then(|r| r.get(j)).copied().unwrap_or(0.0))
                        .sum()
                })
                .collect()
        })
        .collect()
}

/// Transpose a matrix.
/// If the matrix is empty the result is empty; ragged rows are zero-padded.
pub fn transpose(m: &[Vec<f64>]) -> Matrix {
    if m.is_empty() {
        return vec![];
    }
    let cols = m.iter().map(|r| r.len()).max().unwrap_or(0);
    (0..cols).map(|j| get_col(m, j)).collect()
}

/// Matrix–vector product `m × v`.
/// Each row of `m` is dot-producted with `v`.
pub fn mat_vec_mul(m: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    m.iter().map(|row| dot(row, v)).collect()
}

// ── Utility / higher-level ────────────────────────────────────────────────────

/// Return `true` if every corresponding pair of elements differs by at most `tol`.
/// Vectors of different lengths are compared with zero-padding.
pub fn vec_approx_eq(a: &[f64], b: &[f64], tol: f64) -> bool {
    let len = a.len().max(b.len());
    (0..len).all(|i| {
        (a.get(i).copied().unwrap_or(0.0) - b.get(i).copied().unwrap_or(0.0)).abs() <= tol
    })
}

/// Pearson correlation between two equal-length slices.
/// Returns `0.0` when either slice has zero variance.
fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let mean_x = x.iter().sum::<f64>() / n;
    let mean_y = y.iter().sum::<f64>() / n;

    let cov: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(&xi, &yi)| (xi - mean_x) * (yi - mean_y))
        .sum();

    let std_x: f64 = x.iter().map(|&xi| (xi - mean_x).powi(2)).sum::<f64>().sqrt();
    let std_y: f64 = y.iter().map(|&yi| (yi - mean_y).powi(2)).sum::<f64>().sqrt();

    if std_x == 0.0 || std_y == 0.0 {
        0.0
    } else {
        cov / (std_x * std_y)
    }
}

/// Compute the `k × k` Pearson correlation matrix from a data matrix
/// where each row is one observation and each column is one variable.
///
/// `correlation_matrix[i][j]` is the Pearson r between column `i` and column `j`.
/// The diagonal is always `1.0`.  Returns an empty matrix if `data` is empty.
pub fn correlation_matrix(data: &[Vec<f64>]) -> Matrix {
    if data.is_empty() {
        return vec![];
    }
    let (_, k) = shape(data);
    if k == 0 {
        return vec![];
    }
    // pre-extract columns once
    let cols: Vec<Vec<f64>> = (0..k).map(|j| get_col(data, j)).collect();
    make_matrix(k, k, |i, j| {
        if i == j {
            1.0
        } else {
            pearson(&cols[i], &cols[j])
        }
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f64 = 1e-10;

    // helpers
    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    // ── vec_add ───────────────────────────────────────────────────────────────
    #[test]
    fn test_vec_add_same_length() {
        assert_eq!(vec_add(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]), vec![5.0, 7.0, 9.0]);
    }

    #[test]
    fn test_vec_add_different_lengths() {
        // shorter b is zero-padded
        assert_eq!(vec_add(&[1.0, 2.0, 3.0], &[4.0]), vec![5.0, 2.0, 3.0]);
    }

    #[test]
    fn test_vec_add_empty() {
        assert_eq!(vec_add(&[], &[]), Vec::<f64>::new());
    }

    // ── vec_sub ───────────────────────────────────────────────────────────────
    #[test]
    fn test_vec_sub() {
        assert_eq!(vec_sub(&[5.0, 7.0, 9.0], &[4.0, 5.0, 6.0]), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_vec_sub_padded() {
        // a is shorter: trailing elements of b subtracted from 0
        assert_eq!(vec_sub(&[1.0], &[1.0, 2.0]), vec![0.0, -2.0]);
    }

    // ── vec_sum ───────────────────────────────────────────────────────────────
    #[test]
    fn test_vec_sum_empty() {
        assert_eq!(vec_sum(&[]), Vec::<f64>::new());
    }

    #[test]
    fn test_vec_sum_multiple() {
        let vecs = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
        assert_eq!(vec_sum(&vecs), vec![9.0, 12.0]);
    }

    // ── scalar_mul ────────────────────────────────────────────────────────────
    #[test]
    fn test_scalar_mul() {
        assert_eq!(scalar_mul(&[1.0, 2.0, 3.0], 2.0), vec![2.0, 4.0, 6.0]);
    }

    // ── vec_mean ──────────────────────────────────────────────────────────────
    #[test]
    fn test_vec_mean() {
        let vecs = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        assert!(vec_approx_eq(&vec_mean(&vecs), &[2.0, 3.0], TOL));
    }

    #[test]
    fn test_vec_mean_empty() {
        assert_eq!(vec_mean(&[]), Vec::<f64>::new());
    }

    // ── dot ───────────────────────────────────────────────────────────────────
    #[test]
    fn test_dot() {
        assert!(approx(dot(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]), 32.0));
    }

    #[test]
    fn test_dot_empty() {
        assert!(approx(dot(&[], &[]), 0.0));
    }

    // ── sum_of_squares / magnitude ────────────────────────────────────────────
    #[test]
    fn test_sum_of_squares() {
        assert!(approx(sum_of_squares(&[3.0, 4.0]), 25.0));
    }

    #[test]
    fn test_magnitude() {
        assert!(approx(magnitude(&[3.0, 4.0]), 5.0));
    }

    // ── distance ──────────────────────────────────────────────────────────────
    #[test]
    fn test_squared_distance() {
        assert!(approx(squared_distance(&[1.0, 0.0], &[0.0, 1.0]), 2.0));
    }

    #[test]
    fn test_distance() {
        assert!(approx(distance(&[0.0, 0.0], &[3.0, 4.0]), 5.0));
    }

    // ── shape / get_row / get_col ─────────────────────────────────────────────
    #[test]
    fn test_shape() {
        let m = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        assert_eq!(shape(&m), (2, 3));
    }

    #[test]
    fn test_shape_empty() {
        assert_eq!(shape(&[]), (0, 0));
    }

    #[test]
    fn test_get_row() {
        let m = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        assert_eq!(get_row(&m, 1), vec![3.0, 4.0]);
    }

    #[test]
    fn test_get_col() {
        let m = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        assert_eq!(get_col(&m, 0), vec![1.0, 3.0]);
    }

    // ── make_matrix / identity ────────────────────────────────────────────────
    #[test]
    fn test_make_matrix() {
        let m = make_matrix(2, 3, |i, j| (i * 3 + j) as f64);
        assert_eq!(m, vec![vec![0.0, 1.0, 2.0], vec![3.0, 4.0, 5.0]]);
    }

    #[test]
    fn test_identity_matrix() {
        let id = identity_matrix(3);
        assert_eq!(
            id,
            vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0],
            ]
        );
    }

    // ── mat_add ───────────────────────────────────────────────────────────────
    #[test]
    fn test_mat_add() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let b = vec![vec![5.0, 6.0], vec![7.0, 8.0]];
        assert_eq!(mat_add(&a, &b), vec![vec![6.0, 8.0], vec![10.0, 12.0]]);
    }

    // ── mat_mul ───────────────────────────────────────────────────────────────
    #[test]
    fn test_mat_mul_square() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let b = vec![vec![5.0, 6.0], vec![7.0, 8.0]];
        // [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]] = [[19,22],[43,50]]
        assert_eq!(mat_mul(&a, &b), vec![vec![19.0, 22.0], vec![43.0, 50.0]]);
    }

    #[test]
    fn test_mat_mul_rectangular() {
        // 2×3 × 3×1 → 2×1
        let a = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let b = vec![vec![1.0], vec![1.0], vec![1.0]];
        assert_eq!(mat_mul(&a, &b), vec![vec![6.0], vec![15.0]]);
    }

    #[test]
    fn test_mat_mul_identity() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let id = identity_matrix(2);
        assert_eq!(mat_mul(&a, &id), a);
    }

    // ── transpose ────────────────────────────────────────────────────────────
    #[test]
    fn test_transpose() {
        let m = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        assert_eq!(
            transpose(&m),
            vec![vec![1.0, 4.0], vec![2.0, 5.0], vec![3.0, 6.0]]
        );
    }

    #[test]
    fn test_transpose_empty() {
        assert_eq!(transpose(&[]), Vec::<Vec<f64>>::new());
    }

    // ── mat_vec_mul ───────────────────────────────────────────────────────────
    #[test]
    fn test_mat_vec_mul() {
        let m = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        assert_eq!(mat_vec_mul(&m, &[1.0, 1.0]), vec![3.0, 7.0]);
    }

    // ── vec_approx_eq ─────────────────────────────────────────────────────────
    #[test]
    fn test_vec_approx_eq_true() {
        assert!(vec_approx_eq(&[1.0, 2.0], &[1.0 + 1e-11, 2.0 - 1e-11], TOL));
    }

    #[test]
    fn test_vec_approx_eq_false() {
        assert!(!vec_approx_eq(&[1.0, 2.0], &[1.1, 2.0], TOL));
    }

    // ── correlation_matrix ────────────────────────────────────────────────────
    #[test]
    fn test_correlation_matrix_diagonal() {
        let data = vec![
            vec![1.0, 2.0],
            vec![2.0, 4.0],
            vec![3.0, 6.0],
        ];
        let c = correlation_matrix(&data);
        // diagonal must be 1.0
        assert!(approx(c[0][0], 1.0));
        assert!(approx(c[1][1], 1.0));
    }

    #[test]
    fn test_correlation_matrix_perfect_positive() {
        // col 0 = [1,2,3], col 1 = [2,4,6] — perfect positive correlation
        let data = vec![
            vec![1.0, 2.0],
            vec![2.0, 4.0],
            vec![3.0, 6.0],
        ];
        let c = correlation_matrix(&data);
        assert!(approx(c[0][1], 1.0));
        assert!(approx(c[1][0], 1.0));
    }

    #[test]
    fn test_correlation_matrix_perfect_negative() {
        // col 0 = [1,2,3], col 1 = [3,2,1] — perfect negative correlation
        let data = vec![
            vec![1.0, 3.0],
            vec![2.0, 2.0],
            vec![3.0, 1.0],
        ];
        let c = correlation_matrix(&data);
        assert!(approx(c[0][1], -1.0));
    }

    #[test]
    fn test_correlation_matrix_empty() {
        assert_eq!(correlation_matrix(&[]), Vec::<Vec<f64>>::new());
    }

    #[test]
    fn test_correlation_matrix_single_var() {
        let data = vec![vec![1.0], vec![2.0], vec![3.0]];
        let c = correlation_matrix(&data);
        assert_eq!(c.len(), 1);
        assert!(approx(c[0][0], 1.0));
    }
}
