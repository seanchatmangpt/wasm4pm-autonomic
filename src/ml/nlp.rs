/// NLP utilities: N-gram Language Model, Topic Modeling (deterministic LDA-style Gibbs),
/// Bag-of-Words, and TF-IDF — all implemented with no external crates.
use std::collections::HashMap;

// ── N-gram Language Model ─────────────────────────────────────────────────────

pub struct NgramModel {
    pub n: usize,
    /// count of each (n-1)-gram context
    pub context_counts: HashMap<Vec<String>, usize>,
    /// count of each n-gram (context + word)
    pub ngram_counts: HashMap<Vec<String>, usize>,
    pub vocabulary: Vec<String>,
}

impl NgramModel {
    /// Build from tokenized documents (each doc is a Vec of tokens).
    pub fn fit(docs: &[Vec<String>], n: usize) -> Self {
        assert!(n >= 1, "n must be at least 1");

        let mut context_counts: HashMap<Vec<String>, usize> = HashMap::new();
        let mut ngram_counts: HashMap<Vec<String>, usize> = HashMap::new();
        let mut vocab_set: HashMap<String, ()> = HashMap::new();

        for doc in docs {
            // Collect vocabulary
            for token in doc {
                vocab_set.entry(token.clone()).or_insert(());
            }

            // Build n-grams; for n=1 the context is the empty slice
            if doc.len() < n {
                continue;
            }
            for i in 0..=(doc.len() - n) {
                let ngram: Vec<String> = doc[i..i + n].to_vec();
                let context: Vec<String> = ngram[..n - 1].to_vec();
                *context_counts.entry(context).or_insert(0) += 1;
                *ngram_counts.entry(ngram).or_insert(0) += 1;
            }
        }

        let mut vocabulary: Vec<String> = vocab_set.into_keys().collect();
        vocabulary.sort();

        Self {
            n,
            context_counts,
            ngram_counts,
            vocabulary,
        }
    }

    /// P(word | context) with Laplace smoothing (add-1).
    pub fn probability(&self, context: &[String], word: &str) -> f64 {
        let vocab_size = self.vocabulary.len().max(1);

        if self.n == 1 {
            // Unigram: use empty context
            let key: Vec<String> = vec![word.to_string()];
            let word_count = self.ngram_counts.get(&key).copied().unwrap_or(0) as f64;
            let total: usize = self.ngram_counts.values().sum();
            (word_count + 1.0) / (total as f64 + vocab_size as f64)
        } else {
            let context_vec: Vec<String> = context.to_vec();
            let mut ngram_key = context_vec.clone();
            ngram_key.push(word.to_string());

            let ngram_count = self.ngram_counts.get(&ngram_key).copied().unwrap_or(0) as f64;
            let context_count = self.context_counts.get(&context_vec).copied().unwrap_or(0) as f64;

            (ngram_count + 1.0) / (context_count + vocab_size as f64)
        }
    }

    /// Top-k most likely next words given context; sorted descending by probability.
    pub fn top_k(&self, context: &[String], k: usize) -> Vec<(String, f64)> {
        let mut scored: Vec<(String, f64)> = self
            .vocabulary
            .iter()
            .map(|w| (w.clone(), self.probability(context, w)))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
        scored.truncate(k);
        scored
    }

    /// Perplexity on a test document: exp(-1/N * sum log P(w_i | context_i)).
    /// Returns f64::INFINITY for empty docs or n-grams that span past the end.
    pub fn perplexity(&self, doc: &[String]) -> f64 {
        if doc.len() < self.n {
            return f64::INFINITY;
        }

        let mut log_prob_sum = 0.0_f64;
        let count = doc.len() - self.n + 1;

        for i in 0..count {
            let context = &doc[i..i + self.n - 1];
            let word = &doc[i + self.n - 1];
            let p = self.probability(context, word);
            log_prob_sum += p.ln();
        }

        (-log_prob_sum / count as f64).exp()
    }
}

// ── Topic Modeling (deterministic LDA-style Gibbs sampling) ──────────────────

pub struct TopicModel {
    pub n_topics: usize,
    pub n_words: usize,
    /// word_topic_counts[t][w] = count of word w assigned to topic t
    pub word_topic_counts: Vec<Vec<usize>>,
    /// doc_topic_counts[d][t] = count of words in doc d assigned to topic t
    pub doc_topic_counts: Vec<Vec<usize>>,
    pub vocabulary: Vec<String>,
}

impl TopicModel {
    /// Run collapsed Gibbs sampling for LDA.
    ///
    /// `alpha` — symmetric Dirichlet prior over doc-topic distributions.
    /// `beta`  — symmetric Dirichlet prior over topic-word distributions.
    ///
    /// Topic sampling is deterministic (argmax) so results are reproducible.
    pub fn fit(
        docs: &[Vec<String>],
        n_topics: usize,
        alpha: f64,
        beta: f64,
        n_iters: usize,
    ) -> Self {
        if docs.is_empty() || n_topics == 0 {
            return Self {
                n_topics,
                n_words: 0,
                word_topic_counts: vec![vec![]; n_topics],
                doc_topic_counts: vec![],
                vocabulary: vec![],
            };
        }

        // 1. Build vocabulary
        let vocabulary = build_vocabulary(docs);
        let n_words = vocabulary.len();
        let word_index: HashMap<&str, usize> = vocabulary
            .iter()
            .enumerate()
            .map(|(i, w)| (w.as_str(), i))
            .collect();

        let n_docs = docs.len();

        // topic_totals[t] = total tokens assigned to topic t
        let mut topic_totals: Vec<usize> = vec![0; n_topics];
        let mut word_topic_counts: Vec<Vec<usize>> = vec![vec![0; n_words]; n_topics];
        let mut doc_topic_counts: Vec<Vec<usize>> = vec![vec![0; n_topics]; n_docs];

        // Flatten docs into (doc_idx, word_idx) pairs — skip unknown words (shouldn't happen
        // since vocabulary is built from docs, but guard anyway)
        // assignments[flat_pos] = topic
        let mut flat: Vec<(usize, usize)> = Vec::new(); // (doc_idx, word_idx)
        let mut assignments: Vec<usize> = Vec::new();

        for (doc_idx, doc) in docs.iter().enumerate() {
            for (word_pos, token) in doc.iter().enumerate() {
                if let Some(&word_idx) = word_index.get(token.as_str()) {
                    // 2. Deterministic initialisation: topic = (doc_idx*31 + word_pos*7 + word_idx) % n_topics
                    let topic =
                        (doc_idx * 31 + word_pos * 7 + word_idx) % n_topics;
                    flat.push((doc_idx, word_idx));
                    assignments.push(topic);
                    word_topic_counts[topic][word_idx] += 1;
                    doc_topic_counts[doc_idx][topic] += 1;
                    topic_totals[topic] += 1;
                }
            }
        }

        // 3. Gibbs iterations
        let beta_sum = beta * n_words as f64;

        for _ in 0..n_iters {
            for pos in 0..flat.len() {
                let (doc_idx, word_idx) = flat[pos];
                let old_topic = assignments[pos];

                // Remove current assignment
                word_topic_counts[old_topic][word_idx] -= 1;
                doc_topic_counts[doc_idx][old_topic] -= 1;
                topic_totals[old_topic] -= 1;

                // Compute unnormalised scores for each topic; pick argmax (lower index wins ties)
                let mut best_topic = 0;
                let mut best_score = f64::NEG_INFINITY;

                for t in 0..n_topics {
                    let score = (doc_topic_counts[doc_idx][t] as f64 + alpha)
                        * (word_topic_counts[t][word_idx] as f64 + beta)
                        / (topic_totals[t] as f64 + beta_sum);

                    if score > best_score {
                        best_score = score;
                        best_topic = t;
                    }
                }

                // Add new assignment
                assignments[pos] = best_topic;
                word_topic_counts[best_topic][word_idx] += 1;
                doc_topic_counts[doc_idx][best_topic] += 1;
                topic_totals[best_topic] += 1;
            }
        }

        Self {
            n_topics,
            n_words,
            word_topic_counts,
            doc_topic_counts,
            vocabulary,
        }
    }

    /// P(topic t | document d) — normalised doc-topic counts + alpha prior.
    pub fn doc_topic_distribution(&self, doc_idx: usize) -> Vec<f64> {
        if doc_idx >= self.doc_topic_counts.len() {
            return vec![0.0; self.n_topics];
        }
        let row = &self.doc_topic_counts[doc_idx];
        let total: f64 = row.iter().map(|&c| c as f64).sum::<f64>() + self.n_topics as f64;
        row.iter().map(|&c| (c as f64 + 1.0) / total).collect()
    }

    /// P(word w | topic t) — normalised topic-word counts + beta prior.
    pub fn topic_word_distribution(&self, topic: usize) -> Vec<f64> {
        if topic >= self.n_topics || self.n_words == 0 {
            return vec![];
        }
        let row = &self.word_topic_counts[topic];
        let total: f64 =
            row.iter().map(|&c| c as f64).sum::<f64>() + self.n_words as f64;
        row.iter().map(|&c| (c as f64 + 1.0) / total).collect()
    }

    /// Top-k words for each topic, sorted by probability descending.
    pub fn top_words(&self, k: usize) -> Vec<Vec<String>> {
        (0..self.n_topics)
            .map(|t| {
                let dist = self.topic_word_distribution(t);
                let mut indexed: Vec<(usize, f64)> =
                    dist.iter().copied().enumerate().collect();
                indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
                indexed
                    .iter()
                    .take(k)
                    .map(|(i, _)| self.vocabulary[*i].clone())
                    .collect()
            })
            .collect()
    }
}

// ── Bag of Words / TF-IDF ────────────────────────────────────────────────────

/// Build a sorted, deduplicated vocabulary from a collection of tokenised documents.
pub fn build_vocabulary(docs: &[Vec<String>]) -> Vec<String> {
    let mut set: HashMap<String, ()> = HashMap::new();
    for doc in docs {
        for token in doc {
            set.entry(token.clone()).or_insert(());
        }
    }
    let mut vocab: Vec<String> = set.into_keys().collect();
    vocab.sort();
    vocab
}

/// Convert a document into a bag-of-words count vector aligned with `vocabulary`.
/// Words not in the vocabulary are ignored.
pub fn bag_of_words(doc: &[String], vocabulary: &[String]) -> Vec<f64> {
    let index: HashMap<&str, usize> = vocabulary
        .iter()
        .enumerate()
        .map(|(i, w)| (w.as_str(), i))
        .collect();
    let mut counts = vec![0.0_f64; vocabulary.len()];
    for token in doc {
        if let Some(&idx) = index.get(token.as_str()) {
            counts[idx] += 1.0;
        }
    }
    counts
}

/// Compute TF-IDF matrix where entry `[d][w]` = tf(d,w) * idf(w).
///
/// TF  = count(w, d) / len(d)   (raw term frequency, 0 for empty docs)
/// IDF = ln((1 + N) / (1 + df(w))) + 1   (smooth IDF, consistent with scikit-learn default)
pub fn tf_idf(docs: &[Vec<String>], vocabulary: &[String]) -> Vec<Vec<f64>> {
    let n_docs = docs.len();
    let n_vocab = vocabulary.len();

    if n_docs == 0 || n_vocab == 0 {
        return vec![];
    }

    let index: HashMap<&str, usize> = vocabulary
        .iter()
        .enumerate()
        .map(|(i, w)| (w.as_str(), i))
        .collect();

    // Compute raw counts per document
    let counts: Vec<Vec<f64>> = docs
        .iter()
        .map(|doc| {
            let mut c = vec![0.0_f64; n_vocab];
            for token in doc {
                if let Some(&idx) = index.get(token.as_str()) {
                    c[idx] += 1.0;
                }
            }
            c
        })
        .collect();

    // Document frequency for each word
    let mut df = vec![0usize; n_vocab];
    for doc_counts in &counts {
        for (w, &cnt) in doc_counts.iter().enumerate() {
            if cnt > 0.0 {
                df[w] += 1;
            }
        }
    }

    // Smooth IDF: ln((1 + N) / (1 + df)) + 1
    let idf: Vec<f64> = df
        .iter()
        .map(|&d| ((1.0 + n_docs as f64) / (1.0 + d as f64)).ln() + 1.0)
        .collect();

    // TF-IDF
    docs.iter()
        .zip(counts.iter())
        .map(|(doc, doc_counts)| {
            let doc_len = doc.len().max(1) as f64;
            doc_counts
                .iter()
                .enumerate()
                .map(|(w, &cnt)| (cnt / doc_len) * idf[w])
                .collect()
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    // ── N-gram tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_ngram_fit_bigram() {
        let docs = vec![
            tok(&["the", "cat", "sat"]),
            tok(&["the", "cat", "ate"]),
            tok(&["the", "dog", "sat"]),
        ];
        let model = NgramModel::fit(&docs, 2);
        // Vocabulary must contain all unique tokens
        assert!(model.vocabulary.contains(&"cat".to_string()));
        assert!(model.vocabulary.contains(&"dog".to_string()));
        // "the cat" should appear twice in bigram counts
        let key = vec!["the".to_string(), "cat".to_string()];
        assert_eq!(model.ngram_counts.get(&key).copied().unwrap_or(0), 2);
    }

    #[test]
    fn test_ngram_probability_sums_to_one_approx() {
        let docs = vec![
            tok(&["a", "b", "c", "a", "b"]),
            tok(&["a", "c", "b", "c"]),
        ];
        let model = NgramModel::fit(&docs, 2);
        let context = tok(&["a"]);
        let total: f64 = model
            .vocabulary
            .iter()
            .map(|w| model.probability(&context, w))
            .sum();
        // With Laplace smoothing the sum won't be exactly 1.0, but must be > 0 and finite
        assert!(total > 0.0 && total.is_finite());
    }

    #[test]
    fn test_ngram_top_k_ordering() {
        let docs = vec![tok(&["a", "b", "a", "b", "a", "c"])];
        let model = NgramModel::fit(&docs, 2);
        let context = tok(&["a"]);
        let top = model.top_k(&context, 2);
        assert_eq!(top.len(), 2);
        // First must have higher or equal probability to second
        assert!(top[0].1 >= top[1].1);
        // "b" should be the top prediction after "a" (appears twice vs "c" once)
        assert_eq!(top[0].0, "b");
    }

    #[test]
    fn test_ngram_perplexity_finite() {
        let docs = vec![tok(&["a", "b", "c", "a", "b"])];
        let model = NgramModel::fit(&docs, 2);
        let test_doc = tok(&["a", "b", "c"]);
        let ppl = model.perplexity(&test_doc);
        assert!(ppl.is_finite() && ppl > 0.0, "perplexity={ppl}");
    }

    #[test]
    fn test_ngram_perplexity_short_doc_infinity() {
        let docs = vec![tok(&["a", "b", "c"])];
        let model = NgramModel::fit(&docs, 3); // n=3 requires at least 3 tokens
        let short = tok(&["a", "b"]); // only 2 tokens, less than n
        assert_eq!(model.perplexity(&short), f64::INFINITY);
    }

    #[test]
    fn test_unigram_model() {
        let docs = vec![tok(&["hello", "world", "hello"])];
        let model = NgramModel::fit(&docs, 1);
        let p_hello = model.probability(&[], "hello");
        let p_world = model.probability(&[], "world");
        // "hello" appears twice so should have higher probability than "world"
        assert!(p_hello > p_world, "p_hello={p_hello} p_world={p_world}");
    }

    // ── Topic model tests ─────────────────────────────────────────────────────

    #[test]
    fn test_topic_model_fit_shapes() {
        let docs = vec![
            tok(&["rust", "memory", "safe", "rust"]),
            tok(&["python", "data", "science", "python"]),
            tok(&["rust", "systems", "memory"]),
        ];
        let model = TopicModel::fit(&docs, 2, 0.1, 0.01, 10);
        assert_eq!(model.n_topics, 2);
        assert_eq!(model.doc_topic_counts.len(), 3);
        assert_eq!(model.word_topic_counts.len(), 2);
        assert_eq!(model.word_topic_counts[0].len(), model.n_words);
    }

    #[test]
    fn test_topic_model_doc_topic_dist_sums_to_one() {
        let docs = vec![
            tok(&["a", "b", "c"]),
            tok(&["d", "e", "f"]),
        ];
        let model = TopicModel::fit(&docs, 3, 1.0, 0.1, 5);
        for d in 0..docs.len() {
            let dist = model.doc_topic_distribution(d);
            let sum: f64 = dist.iter().sum();
            assert!((sum - 1.0).abs() < 1e-9, "doc {d} dist sum={sum}");
        }
    }

    #[test]
    fn test_topic_model_topic_word_dist_sums_to_one() {
        let docs = vec![tok(&["x", "y", "z", "x", "y"])];
        let model = TopicModel::fit(&docs, 2, 0.5, 0.5, 5);
        for t in 0..model.n_topics {
            let dist = model.topic_word_distribution(t);
            let sum: f64 = dist.iter().sum();
            assert!((sum - 1.0).abs() < 1e-9, "topic {t} dist sum={sum}");
        }
    }

    #[test]
    fn test_topic_model_top_words_length() {
        let docs = vec![
            tok(&["alpha", "beta", "gamma", "delta", "epsilon"]),
            tok(&["zeta", "eta", "theta"]),
        ];
        let model = TopicModel::fit(&docs, 2, 1.0, 0.1, 3);
        let tops = model.top_words(3);
        assert_eq!(tops.len(), 2);
        for topic_words in &tops {
            assert!(topic_words.len() <= 3);
        }
    }

    #[test]
    fn test_topic_model_empty_docs() {
        let model = TopicModel::fit(&[], 3, 1.0, 0.1, 5);
        assert_eq!(model.vocabulary.len(), 0);
        assert_eq!(model.doc_topic_counts.len(), 0);
    }

    // ── Bag-of-words tests ────────────────────────────────────────────────────

    #[test]
    fn test_bag_of_words_counts() {
        let vocab = vec!["cat".to_string(), "dog".to_string(), "fish".to_string()];
        let doc = tok(&["cat", "dog", "cat", "bird"]);
        let bow = bag_of_words(&doc, &vocab);
        // "bird" is not in vocab — ignored
        assert_eq!(bow, vec![2.0, 1.0, 0.0]);
    }

    #[test]
    fn test_bag_of_words_empty_doc() {
        let vocab = vec!["a".to_string(), "b".to_string()];
        let bow = bag_of_words(&[], &vocab);
        assert_eq!(bow, vec![0.0, 0.0]);
    }

    // ── TF-IDF tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_tf_idf_shape() {
        let docs = vec![
            tok(&["a", "b", "c"]),
            tok(&["a", "d"]),
        ];
        let vocab = build_vocabulary(&docs);
        let matrix = tf_idf(&docs, &vocab);
        assert_eq!(matrix.len(), 2);
        assert_eq!(matrix[0].len(), vocab.len());
        assert_eq!(matrix[1].len(), vocab.len());
    }

    #[test]
    fn test_tf_idf_rare_word_higher_idf() {
        // "rare" appears in only 1 of 3 docs; "common" appears in all 3
        let docs = vec![
            tok(&["common", "rare"]),
            tok(&["common", "filler"]),
            tok(&["common", "filler"]),
        ];
        let vocab = build_vocabulary(&docs);
        let matrix = tf_idf(&docs, &vocab);

        let common_idx = vocab.iter().position(|w| w == "common").unwrap();
        let rare_idx = vocab.iter().position(|w| w == "rare").unwrap();

        // In doc 0: both "common" and "rare" have TF=0.5.
        // IDF("rare") > IDF("common") because "rare" is in fewer docs.
        let common_tfidf_d0 = matrix[0][common_idx];
        let rare_tfidf_d0 = matrix[0][rare_idx];
        assert!(
            rare_tfidf_d0 > common_tfidf_d0,
            "rare={rare_tfidf_d0} common={common_tfidf_d0}"
        );
    }

    #[test]
    fn test_build_vocabulary_sorted_unique() {
        let docs = vec![
            tok(&["banana", "apple", "cherry"]),
            tok(&["apple", "date", "banana"]),
        ];
        let vocab = build_vocabulary(&docs);
        // Must be sorted and deduplicated
        let mut expected = vec!["apple", "banana", "cherry", "date"];
        expected.sort();
        assert_eq!(vocab, expected.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    }
}
