// ── Recommender Systems — Collaborative Filtering ────────────────────────────
// Implements chapter 23 of "Data Science from Scratch" by Joel Grus.
// No external crates; deterministic; zero heap allocation on hot similarity paths.

// ── Cosine Similarity (sparse) ───────────────────────────────────────────────

/// Cosine similarity between two sparse rating vectors.
/// Each slice is a sorted-or-unsorted list of `(index, value)` pairs.
/// Missing entries are treated as 0.  Returns 0.0 if either vector is zero.
pub fn cosine_similarity(a: &[(usize, f64)], b: &[(usize, f64)]) -> f64 {
    let mut dot = 0.0_f64;
    let mut norm_a = 0.0_f64;
    let mut norm_b = 0.0_f64;

    // Build a temporary lookup for b by index.
    // We avoid HashMap by using a linear scan — vectors are small in practice.
    for &(ia, va) in a {
        norm_a += va * va;
        for &(ib, vb) in b {
            if ia == ib {
                dot += va * vb;
                break;
            }
        }
    }
    for &(_, vb) in b {
        norm_b += vb * vb;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 { 0.0 } else { dot / denom }
}

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Convert a dense `Option<f64>` row to a sparse `(index, value)` vec.
fn to_sparse(row: &[Option<f64>]) -> Vec<(usize, f64)> {
    row.iter()
        .enumerate()
        .filter_map(|(i, v)| v.map(|r| (i, r)))
        .collect()
}

/// Return the top-k `(id, score)` pairs from an iterator, highest score first.
/// Ties are broken by id to keep results deterministic.
fn top_k(scores: impl Iterator<Item = (usize, f64)>, k: usize) -> Vec<(usize, f64)> {
    let mut v: Vec<(usize, f64)> = scores.collect();
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
    v.truncate(k);
    v
}

// ── User-based Collaborative Filtering ───────────────────────────────────────

pub struct UserCF {
    /// `ratings[user_id][item_id] = Some(rating)` or `None` if not rated.
    pub ratings: Vec<Vec<Option<f64>>>,
    pub n_users: usize,
    pub n_items: usize,
}

impl UserCF {
    /// Build from sparse triples `(user_id, item_id, rating)`.
    pub fn from_ratings(
        triples: &[(usize, usize, f64)],
        n_users: usize,
        n_items: usize,
    ) -> Self {
        let mut ratings = vec![vec![None::<f64>; n_items]; n_users];
        for &(u, i, r) in triples {
            ratings[u][i] = Some(r);
        }
        Self { ratings, n_users, n_items }
    }

    /// `k` most similar users to `user_id` (cosine similarity on shared items),
    /// excluding `user_id` itself.  Returns `Vec<(other_user_id, similarity)>`.
    pub fn similar_users(&self, user_id: usize, k: usize) -> Vec<(usize, f64)> {
        let target = to_sparse(&self.ratings[user_id]);
        let scores = (0..self.n_users)
            .filter(|&u| u != user_id)
            .map(|u| {
                let other = to_sparse(&self.ratings[u]);
                (u, cosine_similarity(&target, &other))
            });
        top_k(scores, k)
    }

    /// Predict rating for `(user_id, item_id)` as a similarity-weighted average
    /// of the `k` most similar users who have rated the item.
    /// Returns `None` if no neighbour has rated the item.
    pub fn predict(&self, user_id: usize, item_id: usize, k: usize) -> Option<f64> {
        let neighbours = self.similar_users(user_id, k);
        let mut weighted_sum = 0.0_f64;
        let mut weight_sum = 0.0_f64;
        for (u, sim) in &neighbours {
            if let Some(r) = self.ratings[*u][item_id] {
                weighted_sum += sim * r;
                weight_sum += sim.abs();
            }
        }
        if weight_sum == 0.0 { None } else { Some(weighted_sum / weight_sum) }
    }

    /// Top-`n` items for `user_id` that they have **not** already rated,
    /// ranked by predicted rating (highest first).
    pub fn recommend(&self, user_id: usize, n: usize, k_neighbors: usize) -> Vec<(usize, f64)> {
        let scores = (0..self.n_items)
            .filter(|&i| self.ratings[user_id][i].is_none())
            .filter_map(|i| self.predict(user_id, i, k_neighbors).map(|s| (i, s)));
        top_k(scores, n)
    }
}

// ── Item-based Collaborative Filtering ───────────────────────────────────────

pub struct ItemCF {
    /// `ratings[user_id][item_id] = Some(rating)` or `None`.
    pub ratings: Vec<Vec<Option<f64>>>,
    pub n_users: usize,
    pub n_items: usize,
}

impl ItemCF {
    /// Build from sparse triples `(user_id, item_id, rating)`.
    pub fn from_ratings(
        triples: &[(usize, usize, f64)],
        n_users: usize,
        n_items: usize,
    ) -> Self {
        let mut ratings = vec![vec![None::<f64>; n_items]; n_users];
        for &(u, i, r) in triples {
            ratings[u][i] = Some(r);
        }
        Self { ratings, n_users, n_items }
    }

    /// Column view: ratings for `item_id` across all users as a sparse vec.
    fn item_vec(&self, item_id: usize) -> Vec<(usize, f64)> {
        (0..self.n_users)
            .filter_map(|u| self.ratings[u][item_id].map(|r| (u, r)))
            .collect()
    }

    /// `k` most similar items to `item_id` (cosine similarity over users),
    /// excluding `item_id` itself.
    pub fn similar_items(&self, item_id: usize, k: usize) -> Vec<(usize, f64)> {
        let target = self.item_vec(item_id);
        let scores = (0..self.n_items)
            .filter(|&i| i != item_id)
            .map(|i| (i, cosine_similarity(&target, &self.item_vec(i))));
        top_k(scores, k)
    }

    /// Predict rating for `(user_id, item_id)` as a similarity-weighted average
    /// of the `k` most similar items that `user_id` has rated.
    pub fn predict(&self, user_id: usize, item_id: usize, k: usize) -> Option<f64> {
        let neighbours = self.similar_items(item_id, k);
        let mut weighted_sum = 0.0_f64;
        let mut weight_sum = 0.0_f64;
        for (i, sim) in &neighbours {
            if let Some(r) = self.ratings[user_id][*i] {
                weighted_sum += sim * r;
                weight_sum += sim.abs();
            }
        }
        if weight_sum == 0.0 { None } else { Some(weighted_sum / weight_sum) }
    }

    /// Top-`n` unrated items for `user_id`, ranked by predicted rating.
    pub fn recommend(&self, user_id: usize, n: usize, k_neighbors: usize) -> Vec<(usize, f64)> {
        let scores = (0..self.n_items)
            .filter(|&i| self.ratings[user_id][i].is_none())
            .filter_map(|i| self.predict(user_id, i, k_neighbors).map(|s| (i, s)));
        top_k(scores, n)
    }
}

// ── Matrix Factorization (SGD) ────────────────────────────────────────────────

/// Latent-factor model trained by stochastic gradient descent.
/// Corresponds to the SVD-style approach in ch 23 of "Data Science from Scratch".
pub struct MatrixFactorization {
    /// `user_factors[u]` — latent vector for user `u`.
    pub user_factors: Vec<Vec<f64>>,
    /// `item_factors[i]` — latent vector for item `i`.
    pub item_factors: Vec<Vec<f64>>,
}

impl MatrixFactorization {
    /// Train via SGD.
    ///
    /// # Arguments
    /// * `triples`  — `(user_id, item_id, rating)` observed ratings
    /// * `n_users`  — total number of users
    /// * `n_items`  — total number of items
    /// * `n_factors`— latent dimension
    /// * `lr`       — learning rate
    /// * `n_epochs` — training epochs
    /// * `lambda`   — L2 regularisation coefficient
    pub fn fit(
        triples: &[(usize, usize, f64)],
        n_users: usize,
        n_items: usize,
        n_factors: usize,
        lr: f64,
        n_epochs: usize,
        lambda: f64,
    ) -> Self {
        // Deterministic initialisation (no RNG dependency).
        let mut user_factors: Vec<Vec<f64>> = (0..n_users)
            .map(|u| {
                (0..n_factors)
                    .map(|f| (u as f64 * 0.1 + f as f64 * 0.01) * 0.1)
                    .collect()
            })
            .collect();
        let mut item_factors: Vec<Vec<f64>> = (0..n_items)
            .map(|i| {
                (0..n_factors)
                    .map(|f| (i as f64 * 0.1 + f as f64 * 0.01) * 0.1)
                    .collect()
            })
            .collect();

        for _epoch in 0..n_epochs {
            for &(u, i, r) in triples {
                // Predicted rating = dot(user_factors[u], item_factors[i]).
                let pred: f64 = user_factors[u]
                    .iter()
                    .zip(item_factors[i].iter())
                    .map(|(uf, vf)| uf * vf)
                    .sum();
                let error = r - pred;

                // SGD update — note: we must snapshot item_factors[i] before
                // updating user_factors[u] so that the item update uses the
                // pre-step user vector, matching the book's formulation.
                let item_snap: Vec<f64> = item_factors[i].clone();
                let user_snap: Vec<f64> = user_factors[u].clone();

                for f in 0..n_factors {
                    user_factors[u][f] +=
                        lr * (error * item_snap[f] - lambda * user_snap[f]);
                    item_factors[i][f] +=
                        lr * (error * user_snap[f] - lambda * item_snap[f]);
                }
            }
        }

        Self { user_factors, item_factors }
    }

    /// Predicted rating for `(user_id, item_id)` = dot product of latent vectors.
    pub fn predict(&self, user_id: usize, item_id: usize) -> f64 {
        self.user_factors[user_id]
            .iter()
            .zip(self.item_factors[item_id].iter())
            .map(|(u, v)| u * v)
            .sum()
    }

    /// Top-`n` item recommendations for `user_id`, excluding `rated_items`.
    pub fn recommend(
        &self,
        user_id: usize,
        rated_items: &[usize],
        n: usize,
    ) -> Vec<(usize, f64)> {
        let scores = (0..self.item_factors.len())
            .filter(|i| !rated_items.contains(i))
            .map(|i| (i, self.predict(user_id, i)));
        top_k(scores, n)
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: small 3-user × 4-item dataset.
    //   User 0: rated items 0,1,2
    //   User 1: rated items 0,1,3
    //   User 2: rated items 2,3
    fn small_triples() -> Vec<(usize, usize, f64)> {
        vec![
            (0, 0, 5.0), (0, 1, 4.0), (0, 2, 1.0),
            (1, 0, 5.0), (1, 1, 3.0), (1, 3, 2.0),
            (2, 2, 4.0), (2, 3, 5.0),
        ]
    }

    // ── cosine_similarity ─────────────────────────────────────────────────────

    #[test]
    fn test_cosine_identical_vectors() {
        let v = vec![(0, 1.0), (1, 2.0), (2, 3.0)];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-9, "identical vecs should have sim=1, got {sim}");
    }

    #[test]
    fn test_cosine_orthogonal_vectors() {
        let a = vec![(0, 1.0)];
        let b = vec![(1, 1.0)];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0, "orthogonal sparse vecs should have sim=0");
    }

    #[test]
    fn test_cosine_zero_vector() {
        let a: Vec<(usize, f64)> = vec![];
        let b = vec![(0, 1.0)];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
        assert_eq!(cosine_similarity(&b, &a), 0.0);
    }

    #[test]
    fn test_cosine_partial_overlap() {
        // a = (1,0,1), b = (1,1,0) → dot=1, |a|=√2, |b|=√2 → sim=0.5
        let a = vec![(0, 1.0), (2, 1.0)];
        let b = vec![(0, 1.0), (1, 1.0)];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.5).abs() < 1e-9, "expected 0.5, got {sim}");
    }

    // ── UserCF ────────────────────────────────────────────────────────────────

    #[test]
    fn test_usercf_similar_users_excludes_self() {
        let cf = UserCF::from_ratings(&small_triples(), 3, 4);
        let neighbours = cf.similar_users(0, 5);
        assert!(
            neighbours.iter().all(|&(u, _)| u != 0),
            "similar_users must not include the query user"
        );
    }

    #[test]
    fn test_usercf_similar_users_top_k_count() {
        let cf = UserCF::from_ratings(&small_triples(), 3, 4);
        // Only 2 other users exist; k=5 should return at most 2.
        let neighbours = cf.similar_users(0, 5);
        assert!(neighbours.len() <= 2);
    }

    #[test]
    fn test_usercf_predict_rated_item_returns_some() {
        let cf = UserCF::from_ratings(&small_triples(), 3, 4);
        // User 0 hasn't rated item 3; users 1 (sim>0) has.
        let pred = cf.predict(0, 3, 2);
        assert!(pred.is_some(), "should predict from neighbour who rated item 3");
    }

    #[test]
    fn test_usercf_predict_no_neighbour_rated() {
        // User 2 has only rated items 2 and 3.
        // User 2's cosine with user 0 may be 0 (no overlap on items 0,1),
        // so if we restrict k=1 and that neighbour has not rated item 0, None is acceptable.
        // We test a clear isolation: create a singleton user with no shared items.
        let triples = vec![(0, 0, 5.0), (1, 1, 3.0)]; // no shared items
        let cf = UserCF::from_ratings(&triples, 2, 2);
        let pred = cf.predict(0, 1, 1);
        // sim=0 between users, so weight_sum=0 → None.
        assert!(pred.is_none());
    }

    #[test]
    fn test_usercf_recommend_excludes_rated() {
        let cf = UserCF::from_ratings(&small_triples(), 3, 4);
        let recs = cf.recommend(0, 4, 2);
        // User 0 has rated items 0,1,2 — recommendations must not contain those.
        for &(item, _) in &recs {
            assert!(item == 3, "user 0 should only be recommended item 3 (the unrated one), got {item}");
        }
    }

    // ── ItemCF ────────────────────────────────────────────────────────────────

    #[test]
    fn test_itemcf_similar_items_excludes_self() {
        let cf = ItemCF::from_ratings(&small_triples(), 3, 4);
        let neighbours = cf.similar_items(0, 5);
        assert!(neighbours.iter().all(|&(i, _)| i != 0));
    }

    #[test]
    fn test_itemcf_predict_for_unrated_item() {
        let cf = ItemCF::from_ratings(&small_triples(), 3, 4);
        // User 0 has not rated item 3; item 3 is similar to items user 0 has rated.
        let pred = cf.predict(0, 3, 3);
        assert!(pred.is_some(), "ItemCF should predict item 3 for user 0");
        let p = pred.unwrap();
        assert!(p > 0.0 && p <= 5.0, "predicted rating should be in (0,5], got {p}");
    }

    #[test]
    fn test_itemcf_recommend_returns_unrated_items() {
        let cf = ItemCF::from_ratings(&small_triples(), 3, 4);
        let recs = cf.recommend(0, 4, 3);
        // User 0 has rated items 0,1,2 — only item 3 is unrated.
        for &(item, _) in &recs {
            assert_eq!(item, 3);
        }
    }

    // ── MatrixFactorization ───────────────────────────────────────────────────

    #[test]
    fn test_mf_predict_range() {
        let triples = small_triples();
        let mf = MatrixFactorization::fit(&triples, 3, 4, 5, 0.01, 100, 0.01);
        for u in 0..3 {
            for i in 0..4 {
                let p = mf.predict(u, i);
                assert!(p.is_finite(), "prediction must be finite, got {p}");
            }
        }
    }

    #[test]
    fn test_mf_fit_reduces_error() {
        // After training, predictions for observed ratings should be closer than
        // the initial (untrained) predictions.
        let triples = small_triples();
        let mf_cold = MatrixFactorization::fit(&triples, 3, 4, 5, 0.0, 1, 0.0); // no learning
        let mf_warm = MatrixFactorization::fit(&triples, 3, 4, 5, 0.01, 500, 0.01);

        let rmse = |mf: &MatrixFactorization| -> f64 {
            let sum: f64 = triples.iter()
                .map(|&(u, i, r)| (r - mf.predict(u, i)).powi(2))
                .sum();
            (sum / triples.len() as f64).sqrt()
        };

        let cold = rmse(&mf_cold);
        let warm = rmse(&mf_warm);
        assert!(warm < cold, "trained model RMSE ({warm:.4}) should beat untrained ({cold:.4})");
    }

    #[test]
    fn test_mf_recommend_excludes_rated() {
        let triples = small_triples();
        let rated_by_u0 = vec![0usize, 1, 2];
        let mf = MatrixFactorization::fit(&triples, 3, 4, 5, 0.01, 200, 0.01);
        let recs = mf.recommend(0, &rated_by_u0, 4);
        for &(item, _) in &recs {
            assert!(!rated_by_u0.contains(&item), "recommend must exclude rated items, got {item}");
        }
    }

    #[test]
    fn test_mf_deterministic() {
        let triples = small_triples();
        let mf1 = MatrixFactorization::fit(&triples, 3, 4, 4, 0.01, 50, 0.01);
        let mf2 = MatrixFactorization::fit(&triples, 3, 4, 4, 0.01, 50, 0.01);
        for u in 0..3 {
            for i in 0..4 {
                assert_eq!(
                    mf1.predict(u, i), mf2.predict(u, i),
                    "MF must be deterministic"
                );
            }
        }
    }
}
