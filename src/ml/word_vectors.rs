/// Word embedding model trained via skip-gram with negative sampling.
/// Implements the word2vec skip-gram approach from Chapter 21 of
/// "Data Science from Scratch" by Joel Grus.
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// A trained word embedding model.
pub struct WordVectors {
    pub vocabulary: Vec<String>,
    pub word_index: HashMap<String, usize>,
    /// Input (center-word) embeddings — shape [vocab_size][embedding_dim].
    pub embeddings: Vec<Vec<f64>>,
}

impl WordVectors {
    /// Train skip-gram word2vec using negative sampling.
    ///
    /// - `docs`          — tokenised documents
    /// - `embedding_dim` — dimensionality of word vectors
    /// - `window_size`   — context window radius (one side)
    /// - `n_negative`    — negative samples per positive pair
    /// - `lr`            — learning rate
    /// - `epochs`        — training passes over the corpus
    pub fn train(
        docs: &[Vec<String>],
        embedding_dim: usize,
        window_size: usize,
        n_negative: usize,
        lr: f64,
        epochs: usize,
    ) -> Self {
        // ---- 1. Build vocabulary -----------------------------------------------
        let mut word_index: HashMap<String, usize> = HashMap::new();
        let mut vocabulary: Vec<String> = Vec::new();

        for doc in docs {
            for token in doc {
                if !word_index.contains_key(token) {
                    word_index.insert(token.clone(), vocabulary.len());
                    vocabulary.push(token.clone());
                }
            }
        }

        let vocab_size = vocabulary.len();

        if vocab_size == 0 || embedding_dim == 0 {
            return Self {
                vocabulary,
                word_index,
                embeddings: Vec::new(),
            };
        }

        // ---- 2. Deterministic initialisation -----------------------------------
        // embedding[i][j] = (i * 0.1 + j * 0.01 - 0.5) * 0.1
        let init = |i: usize, j: usize| -> f64 {
            (i as f64 * 0.1 + j as f64 * 0.01 - 0.5) * 0.1
        };

        let mut input_emb: Vec<Vec<f64>> = (0..vocab_size)
            .map(|i| (0..embedding_dim).map(|j| init(i, j)).collect())
            .collect();

        let mut ctx_emb: Vec<Vec<f64>> = (0..vocab_size)
            .map(|i| (0..embedding_dim).map(|j| init(i, j)).collect())
            .collect();

        // ---- 3. Training loop --------------------------------------------------
        for _ in 0..epochs {
            for doc in docs {
                let doc_len = doc.len();
                for p in 0..doc_len {
                    let c = match word_index.get(&doc[p]) {
                        Some(&idx) => idx,
                        None => continue,
                    };

                    // Determine window bounds (saturating to doc edges).
                    let lo = p.saturating_sub(window_size);
                    let hi = (p + window_size).min(doc_len - 1);

                    for q in lo..=hi {
                        if q == p {
                            continue;
                        }

                        let w = match word_index.get(&doc[q]) {
                            Some(&idx) => idx,
                            None => continue,
                        };

                        // -- Positive sample (c, w) --------------------------------
                        let pos_score = dot(&input_emb[c], &ctx_emb[w]);
                        let pos_grad = lr * (1.0 - sigmoid(pos_score));

                        // Accumulate gradient before applying so both see
                        // the same pre-update vectors.
                        let delta_c: Vec<f64> =
                            ctx_emb[w].iter().map(|v| pos_grad * v).collect();
                        let delta_w: Vec<f64> =
                            input_emb[c].iter().map(|v| pos_grad * v).collect();

                        axpy_inplace(&mut input_emb[c], &delta_c);
                        axpy_inplace(&mut ctx_emb[w], &delta_w);

                        // -- Negative samples -------------------------------------
                        for n in 0..n_negative {
                            let neg = (p * 31 + n) % vocab_size;

                            let neg_score = dot(&input_emb[c], &ctx_emb[neg]);
                            let neg_grad = lr * sigmoid(neg_score);

                            // Snapshot both before mutating.
                            let emb_c_snap: Vec<f64> = input_emb[c].clone();
                            let ctx_neg_snap: Vec<f64> = ctx_emb[neg].clone();

                            for k in 0..embedding_dim {
                                input_emb[c][k] -= neg_grad * ctx_neg_snap[k];
                                ctx_emb[neg][k] -= neg_grad * emb_c_snap[k];
                            }
                        }
                    }
                }
            }
        }

        Self {
            vocabulary,
            word_index,
            embeddings: input_emb,
        }
    }

    /// Cosine similarity between two words.
    /// Returns `0.0` if either word is absent from the vocabulary.
    pub fn similarity(&self, word_a: &str, word_b: &str) -> f64 {
        match (self.get(word_a), self.get(word_b)) {
            (Some(a), Some(b)) => cosine_sim(a, b),
            _ => 0.0,
        }
    }

    /// `k` most similar words to `word`, ranked by cosine similarity.
    /// The query word itself is excluded from results.
    pub fn most_similar(&self, word: &str, k: usize) -> Vec<(String, f64)> {
        let query = match self.get(word) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let mut scored: Vec<(usize, f64)> = self
            .embeddings
            .iter()
            .enumerate()
            .filter(|(i, _)| self.vocabulary[*i] != word)
            .map(|(i, emb)| (i, cosine_sim(query, emb)))
            .collect();

        // Sort descending by similarity (NaN-safe).
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);

        scored
            .into_iter()
            .map(|(i, sim)| (self.vocabulary[i].clone(), sim))
            .collect()
    }

    /// Embedding vector for `word`, or `None` if out of vocabulary.
    pub fn get(&self, word: &str) -> Option<&Vec<f64>> {
        self.word_index.get(word).map(|&i| &self.embeddings[i])
    }

    /// Vector-offset analogy: `word_a` is to `word_b` as `word_c` is to ?
    ///
    /// Computes `target = emb(b) - emb(a) + emb(c)` then returns the `k`
    /// vocabulary entries nearest to `target` by cosine similarity,
    /// excluding the three input words.
    pub fn analogy(
        &self,
        word_a: &str,
        word_b: &str,
        word_c: &str,
        k: usize,
    ) -> Vec<(String, f64)> {
        let a = match self.get(word_a) {
            Some(v) => v,
            None => return Vec::new(),
        };
        let b = match self.get(word_b) {
            Some(v) => v,
            None => return Vec::new(),
        };
        let c = match self.get(word_c) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let dim = a.len();
        // target = b - a + c
        let target: Vec<f64> = (0..dim).map(|i| b[i] - a[i] + c[i]).collect();

        let excluded = [word_a, word_b, word_c];

        let mut scored: Vec<(usize, f64)> = self
            .embeddings
            .iter()
            .enumerate()
            .filter(|(i, _)| !excluded.contains(&&*self.vocabulary[*i]))
            .map(|(i, emb)| (i, cosine_sim(&target, emb)))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);

        scored
            .into_iter()
            .map(|(i, sim)| (self.vocabulary[i].clone(), sim))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn cosine_sim(a: &[f64], b: &[f64]) -> f64 {
    let ab = dot(a, b);
    let na = dot(a, a).sqrt();
    let nb = dot(b, b).sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        (ab / (na * nb)).clamp(-1.0, 1.0)
    }
}

#[inline]
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x.clamp(-500.0, 500.0)).exp())
}

/// `dst += src` element-wise (same length assumed).
fn axpy_inplace(dst: &mut [f64], src: &[f64]) {
    for (d, s) in dst.iter_mut().zip(src.iter()) {
        *d += s;
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_corpus() -> Vec<Vec<String>> {
        vec![
            "the cat sat on the mat".split_whitespace().map(String::from).collect(),
            "the dog lay on the rug".split_whitespace().map(String::from).collect(),
            "a cat and a dog are friends".split_whitespace().map(String::from).collect(),
        ]
    }

    // Test 1 — Vocabulary is built correctly.
    #[test]
    fn test_vocabulary_building() {
        let docs = tiny_corpus();
        let wv = WordVectors::train(&docs, 4, 2, 1, 0.05, 1);

        // Every token in the corpus must appear in the vocabulary.
        let all_tokens: std::collections::HashSet<&str> = docs
            .iter()
            .flat_map(|d| d.iter())
            .map(|s| s.as_str())
            .collect();

        assert_eq!(wv.vocabulary.len(), all_tokens.len());
        for token in &all_tokens {
            assert!(
                wv.word_index.contains_key(*token),
                "token '{}' missing from word_index",
                token
            );
        }
        // word_index must agree with embeddings length.
        assert_eq!(wv.embeddings.len(), wv.vocabulary.len());
    }

    // Test 2 — Similarity is a value in [-1, 1] after training.
    #[test]
    fn test_similarity_after_training() {
        let docs = tiny_corpus();
        let wv = WordVectors::train(&docs, 8, 2, 2, 0.05, 5);

        let sim_cat_dog = wv.similarity("cat", "dog");
        // Result must be a valid number in range.
        assert!(sim_cat_dog.is_finite(), "similarity is not finite");
        assert!(
            (-1.0..=1.0).contains(&sim_cat_dog),
            "similarity out of range: {}",
            sim_cat_dog
        );

        // A word compared to itself should be 1.0 (or very close).
        let self_sim = wv.similarity("cat", "cat");
        assert!(
            (self_sim - 1.0).abs() < 1e-9,
            "self-similarity should be 1.0, got {}",
            self_sim
        );

        // Unknown word → 0.0.
        let unknown_sim = wv.similarity("cat", "ZZZUNKNOWN");
        assert_eq!(unknown_sim, 0.0);
    }

    // Test 3 — most_similar returns the requested number of results.
    #[test]
    fn test_most_similar_length() {
        let docs = tiny_corpus();
        let wv = WordVectors::train(&docs, 8, 2, 2, 0.05, 5);

        let results = wv.most_similar("cat", 3);
        assert_eq!(results.len(), 3, "expected 3 results, got {}", results.len());

        // The query word itself must not appear in results.
        for (word, _) in &results {
            assert_ne!(word, "cat", "'cat' must not appear in its own most_similar");
        }

        // Sorted descending.
        for w in results.windows(2) {
            assert!(
                w[0].1 >= w[1].1,
                "most_similar not sorted: {} > {}",
                w[0].1,
                w[1].1
            );
        }

        // k larger than vocab-1 → clamps to available words.
        let all_results = wv.most_similar("cat", 1000);
        assert_eq!(all_results.len(), wv.vocabulary.len() - 1);

        // Unknown word → empty.
        let empty = wv.most_similar("ZZZUNKNOWN", 5);
        assert!(empty.is_empty());
    }

    // Test 4 — analogy returns plausible results and excludes input words.
    #[test]
    fn test_analogy_excludes_inputs_and_returns_k() {
        let docs = tiny_corpus();
        let wv = WordVectors::train(&docs, 8, 2, 2, 0.05, 10);

        let results = wv.analogy("cat", "mat", "dog", 2);
        // We expect up to 2 results (vocab may be small, but should be non-empty).
        assert!(!results.is_empty(), "analogy returned no results");
        assert!(results.len() <= 2);

        // None of the three query words should appear in results.
        for (word, _) in &results {
            assert_ne!(word, "cat");
            assert_ne!(word, "mat");
            assert_ne!(word, "dog");
        }

        // Similarities must be finite.
        for (_, sim) in &results {
            assert!(sim.is_finite(), "analogy similarity is not finite");
        }

        // Unknown word → empty.
        let empty = wv.analogy("ZZZUNKNOWN", "cat", "dog", 3);
        assert!(empty.is_empty());
    }

    // Test 5 — empty corpus produces an empty model without panic.
    #[test]
    fn test_empty_corpus() {
        let wv = WordVectors::train(&[], 4, 2, 1, 0.05, 3);
        assert!(wv.vocabulary.is_empty());
        assert!(wv.embeddings.is_empty());
        assert_eq!(wv.similarity("a", "b"), 0.0);
        assert!(wv.most_similar("a", 3).is_empty());
    }

    // Test 6 — get() returns correct dimensionality.
    #[test]
    fn test_get_embedding_dim() {
        let docs = tiny_corpus();
        let dim = 12;
        let wv = WordVectors::train(&docs, dim, 2, 1, 0.05, 2);

        let emb = wv.get("cat").expect("'cat' must be in vocabulary");
        assert_eq!(emb.len(), dim, "embedding dim mismatch");

        assert!(wv.get("ZZZUNKNOWN").is_none());
    }
}
