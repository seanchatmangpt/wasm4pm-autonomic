//! Hierarchical (bottom-up agglomerative) clustering.
//!
//! Implements chapter 20 of "Data Science from Scratch" by Joel Grus.
//!
//! Algorithm overview:
//! 1. Start with n singleton clusters (one per input point).
//! 2. Repeatedly merge the pair with the smallest inter-cluster distance.
//! 3. Record each merge in a `Merge` event (forms the dendrogram).
//! 4. `cut` replays the merge history and stops when k clusters remain.

/// A single merge event recorded during agglomerative clustering.
///
/// `cluster_a` and `cluster_b` are cluster indices **at the time of the merge**.
/// Original points occupy indices `0..n`; merged clusters get indices `n`, `n+1`, …
#[derive(Debug, Clone)]
pub struct Merge {
    pub cluster_a: usize,
    pub cluster_b: usize,
    pub distance: f64,
}

/// Inter-cluster distance metric.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Linkage {
    /// Minimum distance between any two points (one from each cluster).
    Single,
    /// Maximum distance between any two points (one from each cluster).
    Complete,
    /// Mean distance over all pairs (one from each cluster).
    Average,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Squared Euclidean distance (avoids a sqrt when only ordering matters).
#[inline]
fn sq_euclidean(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y) * (x - y)).sum()
}

/// True Euclidean distance between two feature vectors.
#[inline]
fn euclidean(a: &[f64], b: &[f64]) -> f64 {
    sq_euclidean(a, b).sqrt()
}

/// Compute the inter-cluster distance between two sets of point indices
/// given the full feature matrix.
fn inter_cluster_distance(
    pts_a: &[usize],
    pts_b: &[usize],
    features: &[Vec<f64>],
    linkage: Linkage,
) -> f64 {
    match linkage {
        Linkage::Single => pts_a
            .iter()
            .flat_map(|&i| pts_b.iter().map(move |&j| euclidean(&features[i], &features[j])))
            .fold(f64::INFINITY, f64::min),
        Linkage::Complete => pts_a
            .iter()
            .flat_map(|&i| pts_b.iter().map(move |&j| euclidean(&features[i], &features[j])))
            .fold(f64::NEG_INFINITY, f64::max),
        Linkage::Average => {
            let sum: f64 = pts_a
                .iter()
                .flat_map(|&i| pts_b.iter().map(move |&j| euclidean(&features[i], &features[j])))
                .sum();
            let count = (pts_a.len() * pts_b.len()) as f64;
            if count == 0.0 { f64::INFINITY } else { sum / count }
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run agglomerative clustering and return the merge history (n−1 merges for n points).
///
/// Cluster indices start at `0..n` for the n singletons; each merge produces a
/// new cluster at index `n + merge_step`.
///
/// Returns an empty `Vec` when `features` is empty or contains a single point.
pub fn fit(features: &[Vec<f64>], linkage: Linkage) -> Vec<Merge> {
    let n = features.len();
    if n <= 1 {
        return Vec::new();
    }

    // `members[i]` holds the original point indices contained in cluster i.
    // We use Option so we can "remove" merged clusters without shifting indices.
    let mut members: Vec<Option<Vec<usize>>> = (0..n).map(|i| Some(vec![i])).collect();
    // Next cluster id to assign when we create a merged cluster.
    let mut next_id = n;

    let mut merges = Vec::with_capacity(n - 1);

    // We need to track which cluster ids are still "alive".
    // Use a Vec<usize> of live ids; size shrinks by 1 each iteration.
    let mut live: Vec<usize> = (0..n).collect();

    // Grow `members` lazily as we create merged clusters.
    // Reserve enough capacity to avoid repeated reallocation.
    members.reserve(n - 1);

    for _ in 0..n - 1 {
        // Find the pair (i, j) in `live` with minimum inter-cluster distance.
        let mut best_dist = f64::INFINITY;
        let mut best_i = 0usize;
        let mut best_j = 1usize;

        for (li, &ci) in live.iter().enumerate() {
            let pts_a = members[ci].as_ref().unwrap();
            for &cj in &live[li + 1..] {
                let pts_b = members[cj].as_ref().unwrap();
                let d = inter_cluster_distance(pts_a, pts_b, features, linkage);
                if d < best_dist {
                    best_dist = d;
                    best_i = ci;
                    best_j = cj;
                }
            }
        }

        // Record the merge.
        merges.push(Merge { cluster_a: best_i, cluster_b: best_j, distance: best_dist });

        // Build the new merged cluster.
        let mut new_pts = members[best_i].take().unwrap();
        new_pts.extend_from_slice(members[best_j].take().unwrap().as_slice());
        members.push(Some(new_pts));

        // Update `live`: remove best_i and best_j, add next_id.
        live.retain(|&c| c != best_i && c != best_j);
        live.push(next_id);
        next_id += 1;
    }

    merges
}

/// Cut the dendrogram at `k` clusters and return a cluster label (0..k) per input point.
///
/// Special cases:
/// - `k == 0` → all points get label 0.
/// - `k >= n` → each point is its own cluster (label == point index).
/// - `merges` is empty (n ≤ 1) → all points get label 0.
pub fn cut(merges: &[Merge], n_points: usize, k: usize) -> Vec<usize> {
    if n_points == 0 {
        return Vec::new();
    }
    if k == 0 {
        return vec![0usize; n_points];
    }
    if k >= n_points {
        // Each point is its own cluster.
        return (0..n_points).collect();
    }

    // Replay merges: each cluster id maps to a "root" representative.
    // We use a union-find-style parent array over cluster ids 0..n_points+n_merges.
    let total_clusters = n_points + merges.len();
    let mut parent: Vec<usize> = (0..total_clusters).collect();

    // We replay only (n_points - k) merges to get k clusters.
    let merges_to_apply = n_points.saturating_sub(k);

    for (step, merge) in merges.iter().enumerate().take(merges_to_apply) {
        let new_id = n_points + step;
        // Point cluster_a and cluster_b to new_id.
        parent[merge.cluster_a] = new_id;
        parent[merge.cluster_b] = new_id;
    }

    // Find root for each cluster id (path compression not needed; depth ≤ n).
    let find = |mut id: usize| -> usize {
        while parent[id] != id {
            id = parent[id];
        }
        id
    };

    // Map each original point (0..n_points) to its root.
    let roots: Vec<usize> = (0..n_points).map(|i| find(i)).collect();

    // Assign compact labels 0..k to distinct roots.
    let mut root_to_label: std::collections::HashMap<usize, usize> =
        std::collections::HashMap::new();
    let mut next_label = 0usize;
    roots
        .iter()
        .map(|&r| {
            let len = root_to_label.len();
            *root_to_label.entry(r).or_insert_with(|| {
                let label = next_label;
                next_label = len + 1;
                label
            })
        })
        .collect()
}

/// Full pipeline: `fit` then `cut` to exactly `k` clusters.
///
/// Returns a cluster label (0..k) per input point.
pub fn cluster(features: &[Vec<f64>], k: usize, linkage: Linkage) -> Vec<usize> {
    let merges = fit(features, linkage);
    cut(&merges, features.len(), k)
}

/// Binary unsupervised classification using 2-cluster agglomerative clustering.
///
/// Clusters all points into 2 groups, then assigns `true`/`false` to each cluster
/// based on which label appears more frequently among the `seed_labels` hints.
/// If a cluster has no seeds or seeds are tied, it defaults to `false`.
pub fn classify_unsupervised(
    features: &[Vec<f64>],
    seed_labels: &[Option<bool>],
    linkage: Linkage,
) -> Vec<bool> {
    let n = features.len();
    if n == 0 {
        return Vec::new();
    }

    let assignments = cluster(features, 2, linkage);

    // Tally true/false seed counts per cluster (0 or 1).
    let mut true_count = [0usize; 2];
    let mut false_count = [0usize; 2];

    for (i, &cluster_id) in assignments.iter().enumerate() {
        let cid = cluster_id.min(1); // guard: should always be 0 or 1
        if let Some(label) = seed_labels.get(i).copied().flatten() {
            if label {
                true_count[cid] += 1;
            } else {
                false_count[cid] += 1;
            }
        }
    }

    // A cluster is considered `true` if true seeds outnumber false seeds.
    let cluster_is_true: [bool; 2] = [
        true_count[0] > false_count[0],
        true_count[1] > false_count[1],
    ];

    assignments.iter().map(|&c| cluster_is_true[c.min(1)]).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(coords: &[(f64, f64)]) -> Vec<Vec<f64>> {
        coords.iter().map(|&(x, y)| vec![x, y]).collect()
    }

    #[test]
    fn test_fit_empty() {
        let merges = fit(&[], Linkage::Single);
        assert!(merges.is_empty());
    }

    #[test]
    fn test_fit_single_point() {
        let merges = fit(&[vec![1.0, 2.0]], Linkage::Single);
        assert!(merges.is_empty());
    }

    #[test]
    fn test_fit_two_points() {
        let features = pts(&[(0.0, 0.0), (3.0, 4.0)]);
        let merges = fit(&features, Linkage::Single);
        assert_eq!(merges.len(), 1);
        assert!((merges[0].distance - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_fit_three_points_single_linkage() {
        // Points: A=(0,0), B=(1,0), C=(10,0)
        // A-B dist=1, A-C dist=10, B-C dist=9
        // First merge: A+B (dist=1), then merge with C (dist=9, single=min(dist(A,C),dist(B,C))=9)
        let features = pts(&[(0.0, 0.0), (1.0, 0.0), (10.0, 0.0)]);
        let merges = fit(&features, Linkage::Single);
        assert_eq!(merges.len(), 2);
        assert!((merges[0].distance - 1.0).abs() < 1e-10);
        assert!((merges[1].distance - 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_fit_three_points_complete_linkage() {
        // Same points; complete linkage: first merge A+B (dist=1).
        // Then {A,B} vs C: max(dist(A,C), dist(B,C)) = max(10,9) = 10
        let features = pts(&[(0.0, 0.0), (1.0, 0.0), (10.0, 0.0)]);
        let merges = fit(&features, Linkage::Complete);
        assert_eq!(merges.len(), 2);
        assert!((merges[0].distance - 1.0).abs() < 1e-10);
        assert!((merges[1].distance - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_fit_three_points_average_linkage() {
        // {A,B} vs C: mean(dist(A,C), dist(B,C)) = (10+9)/2 = 9.5
        let features = pts(&[(0.0, 0.0), (1.0, 0.0), (10.0, 0.0)]);
        let merges = fit(&features, Linkage::Average);
        assert_eq!(merges.len(), 2);
        assert!((merges[0].distance - 1.0).abs() < 1e-10);
        assert!((merges[1].distance - 9.5).abs() < 1e-10);
    }

    #[test]
    fn test_cut_edge_cases() {
        let features = pts(&[(0.0, 0.0), (1.0, 0.0), (10.0, 0.0)]);
        let merges = fit(&features, Linkage::Single);

        // k=0 → all label 0
        let labels = cut(&merges, 3, 0);
        assert_eq!(labels, vec![0, 0, 0]);

        // k>=n → each its own cluster
        let labels = cut(&merges, 3, 3);
        assert_eq!(labels.len(), 3);
        assert_eq!(labels[0], 0);
        assert_eq!(labels[1], 1);
        assert_eq!(labels[2], 2);

        // k=1 → all in one cluster
        let labels = cut(&merges, 3, 1);
        assert!(labels.iter().all(|&l| l == labels[0]));
    }

    #[test]
    fn test_cluster_two_clear_groups() {
        // Two well-separated clusters: left=(0..2,0), right=(100..102,0)
        let features = pts(&[
            (0.0, 0.0),
            (1.0, 0.0),
            (2.0, 0.0),
            (100.0, 0.0),
            (101.0, 0.0),
            (102.0, 0.0),
        ]);
        let labels = cluster(&features, 2, Linkage::Single);
        assert_eq!(labels.len(), 6);
        // First 3 must share a label, last 3 must share a different label.
        let group_left = labels[0];
        let group_right = labels[3];
        assert_ne!(group_left, group_right);
        assert!(labels[0..3].iter().all(|&l| l == group_left));
        assert!(labels[3..6].iter().all(|&l| l == group_right));
    }

    #[test]
    fn test_cluster_empty_and_single() {
        let empty: Vec<Vec<f64>> = vec![];
        assert!(cluster(&empty, 2, Linkage::Single).is_empty());

        let single = vec![vec![1.0, 2.0]];
        let labels = cluster(&single, 2, Linkage::Single);
        assert_eq!(labels, vec![0]);
    }

    #[test]
    fn test_classify_unsupervised_basic() {
        let features = pts(&[
            (0.0, 0.0),
            (1.0, 0.0),
            (2.0, 0.0),
            (100.0, 0.0),
            (101.0, 0.0),
            (102.0, 0.0),
        ]);
        // Seed: first group is false, second group is true.
        let seeds: Vec<Option<bool>> = vec![
            Some(false),
            None,
            None,
            Some(true),
            None,
            None,
        ];
        let preds = classify_unsupervised(&features, &seeds, Linkage::Single);
        assert_eq!(preds.len(), 6);
        assert!(preds[0..3].iter().all(|&p| !p));
        assert!(preds[3..6].iter().all(|&p| p));
    }

    #[test]
    fn test_classify_unsupervised_empty() {
        let preds = classify_unsupervised(&[], &[], Linkage::Single);
        assert!(preds.is_empty());
    }

    #[test]
    fn test_n_merges_count() {
        for n in 2..=10 {
            let features: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
            let merges = fit(&features, Linkage::Single);
            assert_eq!(merges.len(), n - 1, "expected {n}-1 merges for {n} points");
        }
    }
}
