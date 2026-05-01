//! ELIZA breed: phrase binding to public graph terms via SKOS concepts.
//!
//! Uses a per-call label index built from `quads_for_pattern` over `skos:prefLabel`,
//! `skos:altLabel`, and `schema:name`. No SPARQL parsing on the hot path.

use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::utils::dense::{fnv1a_64, PackedKeyTable};
use crate::verdict::BoundTerms;
use anyhow::Result;
use oxigraph::model::{NamedNode, Term};

/// ELIZA: Phrase binding to public ontology terms.
/// Admits only job-language phrases that bind to SKOS concepts.
/// Performs bounded JTBD n-gram extraction with longest-match-first.
pub fn bind_phrase(phrase: &str, field: &FieldContext) -> Result<BoundTerms> {
    let index = build_label_index(field)?;
    if index.is_empty() {
        return Ok(BoundTerms { terms: Vec::new() });
    }

    let tokens: Vec<&str> = phrase.split_whitespace().collect();
    let mut found_iris: Vec<GraphIri> = Vec::new();

    // 2-grams (longest-match-first)
    for window in tokens.windows(2) {
        let key = window.join(" ").to_ascii_lowercase();
        if let Some(iris) = index.get(fnv1a_64(key.as_bytes())) {
            found_iris.extend(iris.iter().cloned().map(GraphIri));
        }
    }
    // 1-grams
    for token in &tokens {
        let key = token.to_ascii_lowercase();
        if let Some(iris) = index.get(fnv1a_64(key.as_bytes())) {
            found_iris.extend(iris.iter().cloned().map(GraphIri));
        }
    }

    if found_iris.is_empty() {
        return Ok(BoundTerms { terms: Vec::new() });
    }
    found_iris.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    found_iris.dedup_by(|a, b| a.as_str() == b.as_str());
    Ok(BoundTerms { terms: found_iris })
}

/// Build a lowercase-label → concept-IRIs index from skos:prefLabel, skos:altLabel, schema:name.
fn build_label_index(field: &FieldContext) -> Result<PackedKeyTable<String, Vec<NamedNode>>> {
    let predicates = [
        NamedNode::new("http://www.w3.org/2004/02/skos/core#prefLabel")?,
        NamedNode::new("http://www.w3.org/2004/02/skos/core#altLabel")?,
        NamedNode::new("https://schema.org/name")?,
    ];
    let mut index: PackedKeyTable<String, Vec<NamedNode>> = PackedKeyTable::new();
    for pred in &predicates {
        for (subject, object) in field.graph.pairs_with_predicate(pred)? {
            let label = match &object {
                Term::Literal(lit) => lit.value().to_string(),
                _ => continue,
            };
            let key = label.to_ascii_lowercase();
            match index.get_mut(fnv1a_64(key.as_bytes())) {
                Some(iris) => {
                    if !iris.iter().any(|n| n.as_str() == subject.as_str()) {
                        iris.push(subject);
                    }
                    continue;
                }
                None => {
                    index.insert(fnv1a_64(key.as_bytes()), key.clone(), vec![subject]);
                }
            };
        }
    }
    Ok(index)
}
