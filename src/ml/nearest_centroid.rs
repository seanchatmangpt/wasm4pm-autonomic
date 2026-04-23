/// Nearest Centroid classifier.
///
/// For each class (positive / negative) the centroid is the element-wise mean
/// of all training vectors belonging to that class.  Prediction assigns a test
/// vector to the class whose centroid is closest under squared Euclidean
/// distance.  Ties are broken in favour of the negative class.

#[derive(Debug, Clone)]
pub struct Centroids {
    pub pos: Vec<f64>,
    pub neg: Vec<f64>,
}

/// Compute the element-wise mean of a non-empty slice of vectors.
///
/// Vectors may differ in length; short vectors are zero-padded implicitly.
fn mean_vector(vecs: &[&Vec<f64>]) -> Vec<f64> {
    let n = vecs.len() as f64;
    let dim = vecs.iter().map(|v| v.len()).max().unwrap_or(0);
    let mut centroid = vec![0.0_f64; dim];
    for v in vecs {
        for (i, &x) in v.iter().enumerate() {
            centroid[i] += x;
        }
    }
    for c in centroid.iter_mut() {
        *c /= n;
    }
    centroid
}

/// Squared Euclidean distance between two vectors.
///
/// Missing dimensions (when lengths differ) are treated as 0.0.
fn sq_dist(a: &[f64], b: &[f64]) -> f64 {
    let len = a.len().max(b.len());
    (0..len)
        .map(|i| {
            let ai = *a.get(i).unwrap_or(&0.0);
            let bi = *b.get(i).unwrap_or(&0.0);
            (ai - bi) * (ai - bi)
        })
        .sum()
}

/// Fit a [`Centroids`] model from labelled training data.
///
/// Returns `None` if either class has zero representatives in `train`.
pub fn fit(train: &[Vec<f64>], labels: &[bool]) -> Option<Centroids> {
    let pos_vecs: Vec<&Vec<f64>> = train
        .iter()
        .zip(labels.iter())
        .filter_map(|(v, &l)| if l { Some(v) } else { None })
        .collect();

    let neg_vecs: Vec<&Vec<f64>> = train
        .iter()
        .zip(labels.iter())
        .filter_map(|(v, &l)| if !l { Some(v) } else { None })
        .collect();

    if pos_vecs.is_empty() || neg_vecs.is_empty() {
        return None;
    }

    Some(Centroids {
        pos: mean_vector(&pos_vecs),
        neg: mean_vector(&neg_vecs),
    })
}

/// Predict class labels for `test` vectors given pre-fitted [`Centroids`].
///
/// Returns `true` (positive) iff the squared distance to the positive centroid
/// is strictly less than the squared distance to the negative centroid.
/// Ties resolve to `false` (negative).
pub fn predict(centroids: &Centroids, test: &[Vec<f64>]) -> Vec<bool> {
    test.iter()
        .map(|v| sq_dist(v, &centroids.pos) < sq_dist(v, &centroids.neg))
        .collect()
}

/// Convenience pipeline: fit on `train`/`labels`, then predict on `test`.
///
/// Falls back to all-`false` predictions if [`fit`] returns `None` (i.e. one
/// class is absent from the training set).
pub fn classify(train: &[Vec<f64>], labels: &[bool], test: &[Vec<f64>]) -> Vec<bool> {
    match fit(train, labels) {
        Some(centroids) => predict(&centroids, test),
        None => vec![false; test.len()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(xs: &[f64]) -> Vec<f64> {
        xs.to_vec()
    }

    #[test]
    fn fit_returns_none_when_all_positive() {
        let train = vec![v(&[1.0, 2.0]), v(&[3.0, 4.0])];
        let labels = vec![true, true];
        assert!(fit(&train, &labels).is_none());
    }

    #[test]
    fn fit_returns_none_when_all_negative() {
        let train = vec![v(&[1.0]), v(&[2.0])];
        let labels = vec![false, false];
        assert!(fit(&train, &labels).is_none());
    }

    #[test]
    fn fit_computes_correct_centroids() {
        // pos: [1,0] and [3,0] → mean [2,0]
        // neg: [0,1] and [0,3] → mean [0,2]
        let train = vec![v(&[1.0, 0.0]), v(&[3.0, 0.0]), v(&[0.0, 1.0]), v(&[0.0, 3.0])];
        let labels = vec![true, true, false, false];
        let c = fit(&train, &labels).unwrap();
        assert!((c.pos[0] - 2.0).abs() < 1e-12);
        assert!((c.pos[1] - 0.0).abs() < 1e-12);
        assert!((c.neg[0] - 0.0).abs() < 1e-12);
        assert!((c.neg[1] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn predict_basic() {
        let train = vec![v(&[0.0]), v(&[10.0])];
        let labels = vec![false, true];
        let centroids = fit(&train, &labels).unwrap();
        // 3.0 is closer to 0.0 (neg) than 10.0 (pos) → false
        // 8.0 is closer to 10.0 (pos) → true
        let preds = predict(&centroids, &[v(&[3.0]), v(&[8.0])]);
        assert_eq!(preds, vec![false, true]);
    }

    #[test]
    fn predict_tie_breaks_negative() {
        // pos centroid at 10, neg centroid at 0; test at 5 is equidistant
        let train = vec![v(&[0.0]), v(&[10.0])];
        let labels = vec![false, true];
        let centroids = fit(&train, &labels).unwrap();
        let preds = predict(&centroids, &[v(&[5.0])]);
        assert_eq!(preds, vec![false]);
    }

    #[test]
    fn predict_handles_mismatched_lengths() {
        // pos centroid [1.0], neg centroid [0.0]
        // test vec [1.0, 99.0]: dimension 1 is missing in centroids → treated as 0
        let train = vec![v(&[0.0]), v(&[1.0])];
        let labels = vec![false, true];
        let centroids = fit(&train, &labels).unwrap();
        // dist_pos([1,99]) = (1-1)^2 + (99-0)^2 = 9801
        // dist_neg([1,99]) = (1-0)^2 + (99-0)^2 = 1 + 9801 = 9802
        // → positive
        let preds = predict(&centroids, &[v(&[1.0, 99.0])]);
        assert_eq!(preds, vec![true]);
    }

    #[test]
    fn classify_falls_back_on_none() {
        let train = vec![v(&[1.0])];
        let labels = vec![true]; // no negative class
        let test = vec![v(&[1.0]), v(&[2.0]), v(&[3.0])];
        let preds = classify(&train, &labels, &test);
        assert_eq!(preds, vec![false, false, false]);
    }

    #[test]
    fn classify_empty_feature_vectors() {
        // Empty feature vectors → centroids are empty vecs; all distances are 0 → tie → false
        let train = vec![v(&[]), v(&[])];
        let labels = vec![true, false];
        let test = vec![v(&[])];
        let preds = classify(&train, &labels, &test);
        assert_eq!(preds, vec![false]);
    }

    #[test]
    fn classify_end_to_end() {
        let train = vec![
            v(&[1.0, 1.0]),
            v(&[1.5, 1.5]),
            v(&[5.0, 5.0]),
            v(&[5.5, 5.5]),
        ];
        let labels = vec![false, false, true, true];
        let test = vec![v(&[1.2, 1.2]), v(&[5.2, 5.2])];
        let preds = classify(&train, &labels, &test);
        assert_eq!(preds, vec![false, true]);
    }
}
