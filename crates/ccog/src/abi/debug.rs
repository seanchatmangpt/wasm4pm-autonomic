//! Indented `Display`-style debug rendering for POWL8 / POWL64 (Phase 10,
//! offline only). Useful for human review during plan authoring and replay
//! diff inspection. Never on the hot path.

use std::fmt::Write;

use crate::powl::{Powl8, Powl8Node};
use crate::powl64::Powl64;

/// Render a [`Powl8`] as an indented multi-line string, one node per line.
///
/// Format: `<idx>: <variant>` where `<variant>` is the node kind plus its
/// payload (breed name, branch indices, etc.). Two-space indent.
pub fn powl8_to_debug_string(p: &Powl8) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Powl8 (root={}, nodes={})", p.root, p.nodes.len());
    for (idx, node) in p.nodes.iter().enumerate() {
        let _ = write!(out, "  {idx}: ");
        match *node {
            Powl8Node::Silent => {
                let _ = writeln!(out, "Silent");
            }
            Powl8Node::Activity(b) => {
                let _ = writeln!(out, "Activity({b:?})");
            }
            Powl8Node::PartialOrder { start, count, .. } => {
                let _ = writeln!(out, "PartialOrder(start={start}, count={count})");
            }
            Powl8Node::OperatorSequence { a, b } => {
                let _ = writeln!(out, "OperatorSequence(a={a}, b={b})");
            }
            Powl8Node::OperatorParallel { a, b } => {
                let _ = writeln!(out, "OperatorParallel(a={a}, b={b})");
            }
            Powl8Node::StartNode => {
                let _ = writeln!(out, "StartNode");
            }
            Powl8Node::EndNode => {
                let _ = writeln!(out, "EndNode");
            }
            Powl8Node::Choice { branches, len } => {
                let live: Vec<u16> = branches.iter().take(len as usize).copied().collect();
                let _ = writeln!(out, "Choice(branches={live:?}, len={len})");
            }
            Powl8Node::Loop { body, max_iters } => {
                let _ = writeln!(out, "Loop(body={body}, max_iters={max_iters})");
            }
        }
    }
    out
}

/// Render a [`Powl64`] chain-hash path as an indented multi-line string.
///
/// Each chain entry is one line, prefixed by its position in the path. The
/// hash is rendered as a `urn:blake3:` IRI for cross-tooling consistency.
pub fn powl64_to_debug_string(p: &Powl64) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Powl64 (chain_len={})", p.path().len());
    for (idx, h) in p.path().iter().enumerate() {
        let _ = writeln!(out, "  {idx}: urn:blake3:{}", h.to_hex());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::powl::Powl8Node;
    use crate::verdict::Breed;

    #[test]
    fn powl8_debug_string_includes_choice_loop() {
        let mut p = Powl8::new();
        p.push(Powl8Node::StartNode).unwrap();
        p.push(Powl8Node::Activity(Breed::Eliza)).unwrap();
        p.push(Powl8Node::Choice {
            branches: [1, 0, 0, 0],
            len: 1,
        })
        .unwrap();
        p.push(Powl8Node::Loop {
            body: 1,
            max_iters: 2,
        })
        .unwrap();
        let s = powl8_to_debug_string(&p);
        assert!(s.contains("Choice"));
        assert!(s.contains("Loop"));
        assert!(s.contains("max_iters=2"));
    }
}
