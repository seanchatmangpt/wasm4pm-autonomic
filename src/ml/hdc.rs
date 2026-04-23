/// Hyperdimensional trace classification (HDC/HDIT)
///
/// Encodes process traces as 512-bit hypervectors via temporal binding,
/// builds a prototype from positive training traces via majority bundling,
/// and classifies test traces by Hamming distance to the prototype.
/// This is independent of the approximate Petri net.
use crate::utils::dense_kernel::fnv1a_64;
use rustc_hash::FxHashMap;

pub type Hv512 = [u64; 8];

#[derive(Clone, Debug)]
pub struct HdcClassifier {
    /// FNV-seeded hypervector per activity label (deterministic)
    pub vocab: FxHashMap<String, Hv512>,
    /// Bundled prototype from training positives (majority vote per bit)
    pub prototype: Hv512,
}

// ─ Activity to hypervector (seeded random projection) ──────────────────────
/// Maps activity name to deterministic 512-bit hypervector via FNV seeding.
/// Each of 8 words is seeded independently (by appending index to name), then XOR'd with rotations.
fn activity_hv(name: &str) -> Hv512 {
    let mut hv = [0u64; 8];
    for (i, slot) in hv.iter_mut().enumerate() {
        let mut seed_bytes = name.as_bytes().to_vec();
        seed_bytes.push(i as u8);
        let seed = fnv1a_64(&seed_bytes);
        *slot = seed ^ seed.rotate_left(17) ^ seed.rotate_right(31);
    }
    hv
}

// ─ Trace to hypervector (temporal binding via permutation + XOR) ────────────
/// Encodes word-level position via rotation, then XOR-binds across sequence.
fn rotate_hv(hv: &Hv512, position: usize) -> Hv512 {
    let word_rot = position % 8;
    let bit_rot = (position * 7 + 1) % 64;
    let mut out = [0u64; 8];
    for i in 0..8 {
        out[(i + word_rot) % 8] = hv[i].rotate_left(bit_rot as u32);
    }
    out
}

/// Encodes activity sequence as XOR-bundle of rotated hypervectors.
/// Temporal ordering captured via left-rotation at each position.
fn encode_trace(vocab: &FxHashMap<String, Hv512>, seq: &[String]) -> Hv512 {
    let mut acc = [0u64; 8];
    for (pos, activity) in seq.iter().enumerate() {
        if let Some(hv) = vocab.get(activity) {
            let rotated = rotate_hv(hv, pos);
            for (a, r) in acc.iter_mut().zip(rotated.iter()) {
                *a ^= r;
            }
        }
    }
    acc
}

// ─ Prototype from positive traces (majority bundling) ───────────────────────
/// Bundling: count votes per bit position, threshold at n/2 (majority).
fn build_prototype(encoded: &[Hv512]) -> Hv512 {
    if encoded.is_empty() {
        return [0u64; 8];
    }
    let n = encoded.len();
    let mut counts = [[0u32; 64]; 8];
    for hv in encoded {
        for (w, word) in hv.iter().enumerate() {
            for (b, count) in counts[w].iter_mut().enumerate() {
                *count += ((word >> b) & 1) as u32;
            }
        }
    }
    let mut proto = [0u64; 8];
    for (w, word_counts) in counts.iter().enumerate() {
        for (b, &c) in word_counts.iter().enumerate() {
            if c * 2 >= n as u32 {
                proto[w] |= 1u64 << b;
            }
        }
    }
    proto
}

// ─ Hamming distance (8× POPCNT) ──────────────────────────────────────────────
/// Computes Hamming distance between two 512-bit hypervectors.
fn hamming_512(a: &Hv512, b: &Hv512) -> u32 {
    let mut dist = 0u32;
    for (wa, wb) in a.iter().zip(b.iter()) {
        dist += (wa ^ wb).count_ones();
    }
    dist
}

// ─ Public API ────────────────────────────────────────────────────────────────

/// Fit an HDC classifier on positive training traces.
/// Builds vocabulary from activity set and prototype from majority bundling.
pub fn fit(train_seqs: &[Vec<String>]) -> HdcClassifier {
    let mut vocab: FxHashMap<String, Hv512> = FxHashMap::default();
    for seq in train_seqs {
        for act in seq {
            vocab.entry(act.clone()).or_insert_with(|| activity_hv(act));
        }
    }
    let encoded: Vec<Hv512> = train_seqs
        .iter()
        .map(|seq| encode_trace(&vocab, seq))
        .collect();
    let prototype = build_prototype(&encoded);
    HdcClassifier { vocab, prototype }
}

/// Classify test traces by Hamming distance to prototype.
/// Returns top `n_target` traces (smallest distances) as positive.
pub fn classify(
    classifier: &HdcClassifier,
    test_seqs: &[Vec<String>],
    n_target: usize,
) -> Vec<bool> {
    let mut distances: Vec<(usize, u32)> = test_seqs
        .iter()
        .enumerate()
        .map(|(i, seq)| {
            let hv = encode_trace(&classifier.vocab, seq);
            let dist = hamming_512(&hv, &classifier.prototype);
            (i, dist)
        })
        .collect();
    distances.sort_by_key(|&(i, d)| (d, i));
    let mut out = vec![false; test_seqs.len()];
    for &(i, _) in distances.iter().take(n_target) {
        out[i] = true;
    }
    out
}

/// A discriminative HDC classifier with separate positive and negative prototypes.
#[derive(Clone, Debug)]
pub struct LabeledHdcClassifier {
    pub vocab: FxHashMap<String, Hv512>,
    pub pos_prototype: Hv512,
    pub neg_prototype: Hv512,
}

/// Fit a discriminative HDC classifier from labeled training sequences and optional
/// extra vocabulary sequences (used only to populate the vocab map).
pub fn fit_labeled(
    train_seqs: &[Vec<String>],
    train_labels: &[bool],
    extra_vocab_seqs: &[Vec<String>],
) -> LabeledHdcClassifier {
    let mut vocab: FxHashMap<String, Hv512> = FxHashMap::default();
    // Populate vocab from all available sequences
    for seq in train_seqs.iter().chain(extra_vocab_seqs.iter()) {
        for act in seq {
            vocab.entry(act.clone()).or_insert_with(|| activity_hv(act));
        }
    }
    let pos_encoded: Vec<Hv512> = train_seqs
        .iter()
        .zip(train_labels.iter())
        .filter(|(_, &lbl)| lbl)
        .map(|(seq, _)| encode_trace(&vocab, seq))
        .collect();
    let neg_encoded: Vec<Hv512> = train_seqs
        .iter()
        .zip(train_labels.iter())
        .filter(|(_, &lbl)| !lbl)
        .map(|(seq, _)| encode_trace(&vocab, seq))
        .collect();
    let pos_prototype = build_prototype(&pos_encoded);
    let neg_prototype = build_prototype(&neg_encoded);
    LabeledHdcClassifier {
        vocab,
        pos_prototype,
        neg_prototype,
    }
}

/// Classify test traces using a discriminative HDC classifier.
/// Each trace is scored by (dist_to_neg - dist_to_pos); higher = more positive.
/// Returns top `n_target` traces as positive.
pub fn classify_labeled(
    classifier: &LabeledHdcClassifier,
    test_seqs: &[Vec<String>],
    n_target: usize,
) -> Vec<bool> {
    let mut scores: Vec<(usize, i32)> = test_seqs
        .iter()
        .enumerate()
        .map(|(i, seq)| {
            let hv = encode_trace(&classifier.vocab, seq);
            let d_pos = hamming_512(&hv, &classifier.pos_prototype) as i32;
            let d_neg = hamming_512(&hv, &classifier.neg_prototype) as i32;
            // Higher score = closer to positive prototype relative to negative
            (i, d_neg - d_pos)
        })
        .collect();
    // Sort descending by score, tie-break ascending by index
    scores.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut out = vec![false; test_seqs.len()];
    for &(i, _) in scores.iter().take(n_target) {
        out[i] = true;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_hv_deterministic() {
        let hv1 = activity_hv("activity_a");
        let hv2 = activity_hv("activity_a");
        assert_eq!(hv1, hv2);
    }

    #[test]
    fn test_activity_hv_distinct() {
        let hv_a = activity_hv("a");
        let hv_b = activity_hv("b");
        assert_ne!(hv_a, hv_b);
    }

    #[test]
    fn test_encode_trace() {
        let mut vocab = FxHashMap::default();
        vocab.insert("a".to_string(), activity_hv("a"));
        vocab.insert("b".to_string(), activity_hv("b"));

        let seq = vec!["a".to_string(), "b".to_string()];
        let hv = encode_trace(&vocab, &seq);
        assert_ne!(hv, [0u64; 8]);
    }

    #[test]
    fn test_encode_trace_deterministic() {
        let mut vocab = FxHashMap::default();
        vocab.insert("a".to_string(), activity_hv("a"));
        let seq = vec!["a".to_string(), "a".to_string()];
        let hv1 = encode_trace(&vocab, &seq);
        let hv2 = encode_trace(&vocab, &seq);
        assert_eq!(hv1, hv2);
    }

    #[test]
    fn test_hamming_symmetric() {
        let hv1 = activity_hv("test1");
        let hv2 = activity_hv("test2");
        assert_eq!(hamming_512(&hv1, &hv2), hamming_512(&hv2, &hv1));
    }

    #[test]
    fn test_hamming_same_is_zero() {
        let hv = activity_hv("same");
        assert_eq!(hamming_512(&hv, &hv), 0);
    }

    #[test]
    fn test_build_prototype() {
        let seqs = [
            vec!["a".to_string(), "b".to_string()],
            vec!["a".to_string(), "b".to_string()],
        ];
        let mut vocab = FxHashMap::default();
        vocab.insert("a".to_string(), activity_hv("a"));
        vocab.insert("b".to_string(), activity_hv("b"));
        let encoded: Vec<Hv512> = seqs.iter().map(|s| encode_trace(&vocab, s)).collect();
        let proto = build_prototype(&encoded);
        assert_ne!(proto, [0u64; 8]);
    }

    #[test]
    fn test_fit_and_classify() {
        let train = vec![
            vec!["x".to_string(), "y".to_string()],
            vec!["x".to_string(), "y".to_string()],
        ];
        let clf = fit(&train);
        assert!(!clf.vocab.is_empty());
        assert_ne!(clf.prototype, [0u64; 8]);

        let test = vec![
            vec!["x".to_string(), "y".to_string()],
            vec!["a".to_string(), "b".to_string()],
        ];
        let result = classify(&clf, &test, 1);
        assert_eq!(result.len(), 2);
        assert_eq!(result.iter().filter(|&&b| b).count(), 1);
    }

    #[test]
    fn test_classify_empty() {
        let clf = fit(&[vec!["a".to_string()]]);
        let result = classify(&clf, &[], 10);
        assert!(result.is_empty());
    }

    #[test]
    fn test_classify_target_exceeds_size() {
        let train = vec![vec!["a".to_string()]];
        let clf = fit(&train);
        let test = vec![vec!["a".to_string()], vec!["b".to_string()]];
        let result = classify(&clf, &test, 100);
        assert_eq!(result.iter().filter(|&&b| b).count(), 2);
    }
}
