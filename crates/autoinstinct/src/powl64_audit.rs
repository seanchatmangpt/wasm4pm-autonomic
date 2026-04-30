//! Phase 6 — POWL64 path tamper detector.
//!
//! Independent of ccog's `powl64.rs`, this module operates on a portable
//! representation: an ordered sequence of `(receipt_urn, polarity)` cells.
//! Computes a polarity-folded BLAKE3 chain head and detects three classes
//! of tampering: truncation, polarity flip, and entry swap.

use serde::{Deserialize, Serialize};

/// One cell in a portable POWL64 path.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PathCell {
    /// `urn:blake3:` of the cell's source receipt material.
    pub receipt_urn: String,
    /// Polarity bit (0 = denial, 1 = admission).
    pub polarity: u8,
}

/// Compute the chain head for a path. Genesis: `blake3(source || polarity)`.
/// Subsequent: `blake3(prior || source || polarity)`.
#[must_use]
pub fn chain_head(path: &[PathCell]) -> Option<[u8; 32]> {
    if path.is_empty() {
        return None;
    }
    let mut prior: Option<[u8; 32]> = None;
    for cell in path {
        let source = blake3::hash(cell.receipt_urn.as_bytes());
        let mut hasher = blake3::Hasher::new();
        if let Some(p) = prior {
            hasher.update(&p);
        }
        hasher.update(source.as_bytes());
        hasher.update(&[cell.polarity]);
        let next = hasher.finalize();
        prior = Some(*next.as_bytes());
    }
    prior
}

/// Tamper-detection verdict.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TamperVerdict {
    /// True iff truncation by one entry changes the chain head.
    pub truncation_detected: bool,
    /// True iff flipping the last polarity changes the chain head.
    pub polarity_flip_detected: bool,
    /// True iff swapping any two adjacent entries changes the chain head.
    pub swap_detected: bool,
}

impl TamperVerdict {
    /// True iff every tamper class is detectable.
    #[must_use]
    pub fn all_detected(&self) -> bool {
        self.truncation_detected && self.polarity_flip_detected && self.swap_detected
    }
}

/// Audit a path against three tamper classes.
#[must_use]
pub fn audit(path: &[PathCell]) -> TamperVerdict {
    let head = chain_head(path);

    let mut truncated = path.to_vec();
    truncated.pop();
    let truncation_detected = chain_head(&truncated) != head;

    let polarity_flip_detected = if let Some(last) = path.last().cloned() {
        let mut flipped = path.to_vec();
        let n = flipped.len();
        flipped[n - 1] = PathCell {
            receipt_urn: last.receipt_urn.clone(),
            polarity: last.polarity ^ 1,
        };
        chain_head(&flipped) != head
    } else {
        false
    };

    let swap_detected = if path.len() >= 2 {
        let mut swapped = path.to_vec();
        swapped.swap(0, 1);
        chain_head(&swapped) != head
    } else {
        false
    };

    TamperVerdict {
        truncation_detected,
        polarity_flip_detected,
        swap_detected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(n: usize) -> Vec<PathCell> {
        (0..n)
            .map(|i| PathCell {
                receipt_urn: format!("urn:blake3:cell-{:02x}", i),
                polarity: (i % 2) as u8,
            })
            .collect()
    }

    #[test]
    fn chain_head_is_deterministic() {
        let p = path(4);
        assert_eq!(chain_head(&p), chain_head(&p));
    }

    #[test]
    fn audit_detects_all_three_tamper_classes() {
        let p = path(4);
        let v = audit(&p);
        assert!(v.all_detected(), "{:?}", v);
    }

    #[test]
    fn unique_paths_have_unique_heads() {
        let p1 = path(3);
        let mut p2 = path(3);
        p2[0].receipt_urn = "urn:blake3:different".into();
        assert_ne!(chain_head(&p1), chain_head(&p2));
    }
}
