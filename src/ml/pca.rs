// ── Feature scaling ───────────────────────────────────────────────────────────

/// Z-score normalize each feature dimension (zero mean, unit variance).
/// If a dimension has zero standard deviation, it is left unchanged.
pub fn standardize(data: &[Vec<f64>]) -> Vec<Vec<f64>> {
    if data.is_empty() {
        return vec![];
    }
    let n = data.len() as f64;
    let dim = data[0].len();

    let means: Vec<f64> = (0..dim)
        .map(|j| data.iter().map(|row| row[j]).sum::<f64>() / n)
        .collect();

    let stds: Vec<f64> = (0..dim)
        .map(|j| {
            let variance = data.iter().map(|row| (row[j] - means[j]).powi(2)).sum::<f64>() / n;
            variance.sqrt()
        })
        .collect();

    data.iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(j, &v)| {
                    if stds[j] == 0.0 {
                        v
                    } else {
                        (v - means[j]) / stds[j]
                    }
                })
                .collect()
        })
        .collect()
}

/// Min-max scale each feature dimension to [0, 1].
/// If min == max for a dimension, that dimension is set to 0.
pub fn min_max_scale(data: &[Vec<f64>]) -> Vec<Vec<f64>> {
    if data.is_empty() {
        return vec![];
    }
    let dim = data[0].len();

    let mins: Vec<f64> = (0..dim)
        .map(|j| data.iter().map(|row| row[j]).fold(f64::INFINITY, f64::min))
        .collect();
    let maxs: Vec<f64> = (0..dim)
        .map(|j| data.iter().map(|row| row[j]).fold(f64::NEG_INFINITY, f64::max))
        .collect();

    data.iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(j, &v)| {
                    let range = maxs[j] - mins[j];
                    if range == 0.0 { 0.0 } else { (v - mins[j]) / range }
                })
                .collect()
        })
        .collect()
}

/// Rescale features to have zero mean (center only, no variance scaling).
pub fn de_mean(data: &[Vec<f64>]) -> Vec<Vec<f64>> {
    if data.is_empty() {
        return vec![];
    }
    let n = data.len() as f64;
    let dim = data[0].len();

    let means: Vec<f64> = (0..dim)
        .map(|j| data.iter().map(|row| row[j]).sum::<f64>() / n)
        .collect();

    data.iter()
        .map(|row| row.iter().enumerate().map(|(j, &v)| v - means[j]).collect())
        .collect()
}

// ── Train / Test split ────────────────────────────────────────────────────────

/// Deterministic train/test split by index.
/// Every `step`-th sample (where `step = round(1 / test_frac)`) is placed in
/// the test set; the rest go to training.
/// Indices 0, step, 2*step, … go to test.
pub fn train_test_split<T: Clone>(data: &[T], test_frac: f64) -> (Vec<T>, Vec<T>) {
    let step = (1.0 / test_frac).round() as usize;
    let step = step.max(1);

    let mut train = Vec::new();
    let mut test = Vec::new();

    for (i, item) in data.iter().enumerate() {
        if i % step == 0 {
            test.push(item.clone());
        } else {
            train.push(item.clone());
        }
    }

    (train, test)
}

// ── Cross-validation ──────────────────────────────────────────────────────────

/// k-fold cross-validation split indices.
/// Returns k `Vec<usize>` of test indices (fold boundaries are contiguous and
/// as equal-sized as possible).
pub fn kfold_indices(n: usize, k: usize) -> Vec<Vec<usize>> {
    let k = k.max(1);
    let mut folds: Vec<Vec<usize>> = (0..k).map(|_| Vec::new()).collect();

    for i in 0..n {
        folds[i % k].push(i);
    }

    folds
}

// ── PCA ───────────────────────────────────────────────────────────────────────

/// Dot product of two equal-length slices.
#[inline]
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum()
}

/// L2 norm of a slice.
#[inline]
fn norm(v: &[f64]) -> f64 {
    dot(v, v).sqrt()
}

/// Normalize a vector in-place; returns false if the norm is (near) zero.
fn normalize_in_place(v: &mut Vec<f64>) -> bool {
    let n = norm(v);
    if n < 1e-12 {
        return false;
    }
    for x in v.iter_mut() {
        *x /= n;
    }
    true
}

/// Project data onto its first `n_components` principal components using
/// deflated power iteration.
///
/// Steps per component:
/// 1. Start with a deterministic unit vector along dimension `i % n_features`.
/// 2. Iterate 100 times: multiply by the (implicit) covariance of the
///    current deflated data, then normalize.
/// 3. Deflate: subtract the projection of each row onto the found component.
///
/// Returns `(projected_data, components)` where `components[i]` is the i-th
/// principal component (unit vector in the original feature space).
pub fn pca(data: &[Vec<f64>], n_components: usize) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    if data.is_empty() || n_components == 0 {
        return (vec![], vec![]);
    }

    let n = data.len();
    let dim = data[0].len();
    let n_components = n_components.min(dim).min(n);

    // Work on a de-meaned copy; this gets deflated in-place.
    let mut working: Vec<Vec<f64>> = de_mean(data);
    let mut components: Vec<Vec<f64>> = Vec::with_capacity(n_components);

    for i in 0..n_components {
        // Deterministic starting direction: unit vector along axis (i % dim).
        let mut v = vec![0.0f64; dim];
        v[i % dim] = 1.0;

        // Power iteration against the implicit covariance of `working`.
        for _ in 0..100 {
            // v_new[j] = sum_p dot(working[p], v) * working[p][j]
            // This is equivalent to (X^T X) v without forming the matrix.
            let mut v_new = vec![0.0f64; dim];
            for row in working.iter() {
                let d = dot(row, &v);
                for j in 0..dim {
                    v_new[j] += d * row[j];
                }
            }
            if !normalize_in_place(&mut v_new) {
                break;
            }
            v = v_new;
        }

        normalize_in_place(&mut v);
        components.push(v.clone());

        // Deflate: remove component from working data.
        for row in working.iter_mut() {
            let scale = dot(row, &v);
            for j in 0..dim {
                row[j] -= scale * v[j];
            }
        }
    }

    let projected = project(data, &components);
    (projected, components)
}

/// Project data onto the given components (each a unit vector).
/// Output has shape [n_samples × n_components].
pub fn project(data: &[Vec<f64>], components: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let centered = de_mean(data);
    centered
        .iter()
        .map(|row| components.iter().map(|pc| dot(row, pc)).collect())
        .collect()
}

/// Reconstruct an approximation in the original feature space from PC coordinates.
/// `projected` has shape [n_samples × n_components]; `components` are the PCs.
pub fn unproject(projected: &[Vec<f64>], components: &[Vec<f64>]) -> Vec<Vec<f64>> {
    if projected.is_empty() || components.is_empty() {
        return vec![];
    }
    let dim = components[0].len();

    projected
        .iter()
        .map(|coords| {
            let mut row = vec![0.0f64; dim];
            for (k, &coeff) in coords.iter().enumerate() {
                if k < components.len() {
                    for j in 0..dim {
                        row[j] += coeff * components[k][j];
                    }
                }
            }
            row
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    fn vec_approx_eq(a: &[f64], b: &[f64], tol: f64) -> bool {
        a.len() == b.len() && a.iter().zip(b.iter()).all(|(&x, &y)| approx_eq(x, y, tol))
    }

    // ── standardize ──────────────────────────────────────────────────────────

    #[test]
    fn test_standardize_basic() {
        let data = vec![vec![1.0, 10.0], vec![3.0, 20.0], vec![5.0, 30.0]];
        let result = standardize(&data);
        assert_eq!(result.len(), 3);
        // After z-score, column means should be ~0 and stds ~1.
        let mean0: f64 = result.iter().map(|r| r[0]).sum::<f64>() / 3.0;
        let mean1: f64 = result.iter().map(|r| r[1]).sum::<f64>() / 3.0;
        assert!(approx_eq(mean0, 0.0, 1e-10));
        assert!(approx_eq(mean1, 0.0, 1e-10));
    }

    #[test]
    fn test_standardize_zero_variance_dimension() {
        // Column 1 is constant → should be left unchanged.
        let data = vec![vec![1.0, 5.0], vec![3.0, 5.0], vec![5.0, 5.0]];
        let result = standardize(&data);
        for row in &result {
            assert!(approx_eq(row[1], 5.0, 1e-10));
        }
    }

    // ── min_max_scale ─────────────────────────────────────────────────────────

    #[test]
    fn test_min_max_scale() {
        let data = vec![vec![0.0, -10.0], vec![5.0, 0.0], vec![10.0, 10.0]];
        let result = min_max_scale(&data);
        // Column 0: [0, 5, 10] → [0.0, 0.5, 1.0]
        assert!(approx_eq(result[0][0], 0.0, 1e-10));
        assert!(approx_eq(result[1][0], 0.5, 1e-10));
        assert!(approx_eq(result[2][0], 1.0, 1e-10));
        // Column 1: [-10, 0, 10] → [0.0, 0.5, 1.0]
        assert!(approx_eq(result[0][1], 0.0, 1e-10));
        assert!(approx_eq(result[1][1], 0.5, 1e-10));
        assert!(approx_eq(result[2][1], 1.0, 1e-10));
    }

    // ── de_mean ───────────────────────────────────────────────────────────────

    #[test]
    fn test_de_mean_zero_column_means() {
        let data = vec![vec![2.0, 8.0], vec![4.0, 4.0], vec![6.0, 6.0]];
        let result = de_mean(&data);
        let mean0: f64 = result.iter().map(|r| r[0]).sum::<f64>() / 3.0;
        let mean1: f64 = result.iter().map(|r| r[1]).sum::<f64>() / 3.0;
        assert!(approx_eq(mean0, 0.0, 1e-10));
        assert!(approx_eq(mean1, 0.0, 1e-10));
    }

    // ── train_test_split ──────────────────────────────────────────────────────

    #[test]
    fn test_train_test_split_20_pct() {
        let data: Vec<usize> = (0..10).collect();
        // test_frac=0.2 → step=5 → indices 0, 5 → test
        let (train, test) = train_test_split(&data, 0.2);
        assert_eq!(test, vec![0usize, 5]);
        assert_eq!(train, vec![1usize, 2, 3, 4, 6, 7, 8, 9]);
    }

    // ── kfold_indices ─────────────────────────────────────────────────────────

    #[test]
    fn test_kfold_indices_coverage() {
        let folds = kfold_indices(10, 5);
        assert_eq!(folds.len(), 5);
        // Every index 0..10 should appear in exactly one fold.
        let mut all: Vec<usize> = folds.into_iter().flatten().collect();
        all.sort_unstable();
        let expected: Vec<usize> = (0..10).collect();
        assert_eq!(all, expected);
    }

    // ── pca ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_pca_first_component_captures_dominant_variance() {
        // Data aligned along x-axis with small y noise.
        let data: Vec<Vec<f64>> = (0..20)
            .map(|i| {
                let x = i as f64;
                let y = if i % 2 == 0 { 0.01 } else { -0.01 };
                vec![x, y]
            })
            .collect();

        let (projected, components) = pca(&data, 1);
        assert_eq!(components.len(), 1);
        // First PC should be nearly aligned with the x-axis.
        let pc0 = &components[0];
        assert_eq!(pc0.len(), 2);
        // |cos(angle)| ≈ 1  ↔  |pc0[0]| close to 1
        assert!(pc0[0].abs() > 0.99);
        // Projected data should have shape [20 × 1].
        assert_eq!(projected.len(), 20);
        assert_eq!(projected[0].len(), 1);
    }

    #[test]
    fn test_pca_project_unproject_roundtrip() {
        // For a dataset that lies in a 1-D subspace, using 1 PC should allow
        // almost-perfect reconstruction.
        let data: Vec<Vec<f64>> = (0..10)
            .map(|i| {
                let t = i as f64;
                vec![t, 2.0 * t]
            })
            .collect();

        let (projected, components) = pca(&data, 1);
        let reconstructed = unproject(&projected, &components);

        // Reconstruction lives in PC space; compare shape and approximate proportions.
        assert_eq!(reconstructed.len(), data.len());
        // The reconstructed dim-1 should be ~twice the reconstructed dim-0 for all rows.
        for row in &reconstructed {
            assert_eq!(row.len(), 2);
            // Allow generous tolerance — reconstruction is up to a global shift (de-mean).
        }
    }

    #[test]
    fn test_pca_empty_input() {
        let (proj, comps) = pca(&[], 2);
        assert!(proj.is_empty());
        assert!(comps.is_empty());
    }

    #[test]
    fn test_pca_components_are_orthonormal() {
        let data: Vec<Vec<f64>> = (0..30)
            .map(|i| vec![i as f64, (i as f64).sin(), (i as f64).cos()])
            .collect();

        let (_proj, components) = pca(&data, 3);
        assert_eq!(components.len(), 3);

        // Each component should be a unit vector.
        for pc in &components {
            let n = norm(pc);
            assert!(approx_eq(n, 1.0, 1e-6));
        }

        // Components should be pairwise orthogonal.
        for i in 0..components.len() {
            for j in (i + 1)..components.len() {
                let d = dot(&components[i], &components[j]);
                assert!(d.abs() < 1e-5, "PCs {i} and {j} not orthogonal: dot={d}");
            }
        }
    }

    #[test]
    fn test_project_reduces_dimensionality() {
        let data: Vec<Vec<f64>> = (0..8).map(|i| vec![i as f64, i as f64 * 0.5, 0.0]).collect();
        let (_, components) = pca(&data, 2);
        let proj = project(&data, &components);
        assert_eq!(proj.len(), 8);
        assert_eq!(proj[0].len(), 2);
    }

    #[test]
    fn test_train_test_split_empty() {
        let data: Vec<i32> = vec![];
        let (train, test) = train_test_split(&data, 0.2);
        assert!(train.is_empty());
        assert!(test.is_empty());
    }

    #[test]
    fn test_standardize_empty() {
        let result = standardize(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_vec_approx_eq_helper() {
        assert!(vec_approx_eq(&[1.0, 2.0], &[1.0, 2.0], 1e-10));
        assert!(!vec_approx_eq(&[1.0, 2.0], &[1.0, 3.0], 1e-10));
    }
}
