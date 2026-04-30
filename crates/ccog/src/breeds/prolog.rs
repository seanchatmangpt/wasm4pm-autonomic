//! Prolog-style relations breed: transitive proof via SPARQL property paths + bounded BFS.

use anyhow::Result;
use std::collections::{HashMap, HashSet, VecDeque};
use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::verdict::RelationProof;

const MAX_DEPTH: usize = 8;

/// Prove that `subject` reaches `target` via one-or-more `predicate` hops.
/// Returns `Some(proof)` with intermediate path or `None`. BFS depth-capped at 8.
pub fn prove_relation(
    subject: &GraphIri,
    predicate: &GraphIri,
    target: &GraphIri,
    field: &FieldContext,
) -> Result<Option<RelationProof>> {
    // Phase A: existence check
    let ask_sparql = format!(
        "ASK {{ <{}> <{}>+ <{}> }}",
        subject.as_str(), predicate.as_str(), target.as_str()
    );
    if !field.graph.ask(&ask_sparql)? {
        return Ok(None);
    }
    // Phase B: bounded BFS to extract path
    let mut visited: HashSet<String> = HashSet::new();
    let mut parent: HashMap<String, GraphIri> = HashMap::new();
    let mut queue: VecDeque<(GraphIri, usize)> = VecDeque::new();
    queue.push_back((subject.clone(), 0));
    visited.insert(subject.as_str().to_string());
    while let Some((current, depth)) = queue.pop_front() {
        if depth >= MAX_DEPTH { continue; }
        let q = format!("SELECT ?next WHERE {{ <{}> <{}> ?next }}", current.as_str(), predicate.as_str());
        let rows = field.graph.select(&q)?;
        for row in rows {
            for (k, n) in row {
                let name = k.strip_prefix('?').unwrap_or(&k);
                if name != "next" { continue; }
                let next = GraphIri(n);
                let key = next.as_str().to_string();
                if visited.contains(&key) { continue; }
                visited.insert(key.clone());
                parent.insert(key.clone(), current.clone());
                if next.as_str() == target.as_str() {
                    let mut path = vec![next.clone()];
                    let mut cur = next.as_str().to_string();
                    while let Some(p) = parent.get(&cur) {
                        path.push(p.clone());
                        cur = p.as_str().to_string();
                    }
                    path.reverse();
                    return Ok(Some(RelationProof {
                        subject: subject.clone(),
                        predicate: predicate.clone(),
                        target: target.clone(),
                        path,
                    }));
                }
                queue.push_back((next, depth + 1));
            }
        }
    }
    Ok(None)
}
