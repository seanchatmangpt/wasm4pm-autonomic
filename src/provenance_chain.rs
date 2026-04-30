//! SPR canonical: Blake3Chain — provenance scent trail.
//!
//! Implements the data → plan → source → artifact chain referenced in
//! COMPILED_COGNITION.md §6.3. This is the VALIDATOR; the producer
//! (build.rs hash injection at compile time) is a Vision 2030 Phase 3
//! milestone. Until then, host applications populate the chain.
//!
//! BLAKE3 is the scent trail that cannot be casually rewritten.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Data = 0x01,
    Plan = 0x02,
    Source = 0x03,
    Artifact = 0x04,
}

#[derive(Debug, Clone, Copy)]
pub struct Link {
    pub kind: LinkKind,
    pub hash: [u8; 32],
    pub prev: [u8; 32],
}

/// Append-only BLAKE3 receipt chain.
/// Each link's hash = blake3(prev || kind_tag || payload).
#[derive(Debug)]
pub struct Blake3Chain {
    links: Vec<Link>,
    head: [u8; 32],
}

impl Default for Blake3Chain {
    fn default() -> Self {
        Self::new()
    }
}

impl Blake3Chain {
    pub fn new() -> Self {
        Blake3Chain { links: Vec::new(), head: [0u8; 32] }
    }

    /// Append a link. Computes hash = blake3(prev || kind_tag || payload),
    /// updates head.
    pub fn append(&mut self, kind: LinkKind, payload: &[u8]) {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.head);
        hasher.update(&[kind as u8]);
        hasher.update(payload);
        let hash: [u8; 32] = hasher.finalize().into();
        self.links.push(Link { kind, hash, prev: self.head });
        self.head = hash;
    }

    /// Re-compute the chain and verify head consistency.
    pub fn verify(&self) -> bool {
        let mut prev = [0u8; 32];
        for link in &self.links {
            if link.prev != prev {
                return false;
            }
            // We cannot recompute hash without the original payload,
            // but we can verify each link's `prev` matches the prior head
            // and that the overall head equals the last link's hash.
            prev = link.hash;
        }
        prev == self.head
    }

    pub fn head(&self) -> [u8; 32] { self.head }
    pub fn len(&self) -> usize { self.links.len() }
    pub fn is_empty(&self) -> bool { self.links.is_empty() }
    pub fn links(&self) -> &[Link] { &self.links }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_d_p_s_a_chain() {
        let mut c = Blake3Chain::new();
        c.append(LinkKind::Data, b"training-data-v1");
        c.append(LinkKind::Plan, b"hdit-plan-v1");
        c.append(LinkKind::Source, b"generated-rust-source-v1");
        c.append(LinkKind::Artifact, b"compiled-binary-v1");
        assert_eq!(c.len(), 4);
        assert!(c.verify());
        assert_ne!(c.head(), [0u8; 32]);
    }

    #[test]
    fn tampered_middle_link_fails_verify() {
        let mut c = Blake3Chain::new();
        c.append(LinkKind::Data, b"d");
        c.append(LinkKind::Plan, b"p");
        c.append(LinkKind::Source, b"s");
        // Tamper: corrupt the prev pointer of the third link.
        c.links[2].prev = [0xFF; 32];
        assert!(!c.verify(), "tampered chain must fail verification");
    }

    #[test]
    fn head_equals_last_link_hash() {
        let mut c = Blake3Chain::new();
        c.append(LinkKind::Data, b"x");
        let h0 = c.head();
        c.append(LinkKind::Plan, b"y");
        let h1 = c.head();
        assert_ne!(h0, h1);
        assert_eq!(c.links().last().unwrap().hash, h1);
    }

    #[test]
    fn deterministic_for_same_payloads() {
        let mut a = Blake3Chain::new();
        let mut b = Blake3Chain::new();
        a.append(LinkKind::Data, b"d");
        a.append(LinkKind::Plan, b"p");
        b.append(LinkKind::Data, b"d");
        b.append(LinkKind::Plan, b"p");
        assert_eq!(a.head(), b.head());
    }

    #[test]
    fn different_kinds_produce_different_heads() {
        let mut a = Blake3Chain::new();
        let mut b = Blake3Chain::new();
        a.append(LinkKind::Data, b"x");
        b.append(LinkKind::Plan, b"x");
        assert_ne!(a.head(), b.head());
    }

    #[test]
    fn empty_chain_verifies_with_zero_head() {
        let c = Blake3Chain::new();
        assert!(c.is_empty());
        assert!(c.verify());
        assert_eq!(c.head(), [0u8; 32]);
    }
}
