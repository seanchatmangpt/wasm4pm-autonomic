//! Ontology IRI audit (Phase 11).

use serde_json::Value;

/// Public-ontology IRI prefixes accepted by [`audit_iris`].
pub const ALLOWED_PREFIXES: &[&str] = &[
    "http://www.w3.org/ns/prov#",
    "http://www.w3.org/2002/07/owl#",
    "http://www.w3.org/2000/01/rdf-schema#",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "https://schema.org/",
    "http://schema.org/",
    "http://www.w3.org/ns/shacl#",
    "http://www.w3.org/2001/XMLSchema#",
    "http://www.w3.org/2004/02/skos/core#",
    "http://purl.org/dc/terms/",
    "urn:blake3:",
    "urn:ccog:vocab:",
];

/// Audit failure: an emitted IRI is outside the public-ontology allowlist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonPublicOntology(pub String);

impl std::fmt::Display for NonPublicOntology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "non-public ontology IRI: {}", self.0)
    }
}

impl std::error::Error for NonPublicOntology {}

/// Walk `value` and collect every IRI string under `@id`/`@type`/`@vocab`/`@context`.
///
/// # Errors
///
/// Returns `Err(NonPublicOntology(iri))` for the first IRI whose prefix matches no allowed prefix.
pub fn audit_iris(value: &Value, extra_allow: &[&str]) -> Result<Vec<String>, NonPublicOntology> {
    let mut found = Vec::new();
    walk(value, extra_allow, &mut found)?;
    Ok(found)
}

fn walk(v: &Value, extra: &[&str], found: &mut Vec<String>) -> Result<(), NonPublicOntology> {
    match v {
        Value::Object(map) => {
            for (k, child) in map {
                if k == "@id" || k == "@type" || k == "@vocab" {
                    collect_iris(child, extra, found)?;
                } else if k == "@context" {
                    audit_context(child, extra, found)?;
                } else {
                    walk(child, extra, found)?;
                }
            }
            Ok(())
        }
        Value::Array(arr) => {
            for c in arr {
                walk(c, extra, found)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn audit_context(v: &Value, extra: &[&str], found: &mut Vec<String>) -> Result<(), NonPublicOntology> {
    match v {
        Value::Object(map) => {
            for (_k, child) in map {
                match child {
                    Value::String(s) => check_iri(s, extra, found)?,
                    Value::Object(_) => audit_context(child, extra, found)?,
                    _ => {}
                }
            }
            Ok(())
        }
        Value::Array(arr) => {
            for c in arr {
                audit_context(c, extra, found)?;
            }
            Ok(())
        }
        Value::String(s) => check_iri(s, extra, found),
        _ => Ok(()),
    }
}

fn collect_iris(v: &Value, extra: &[&str], found: &mut Vec<String>) -> Result<(), NonPublicOntology> {
    match v {
        Value::String(s) => check_iri(s, extra, found),
        Value::Array(arr) => {
            for c in arr {
                collect_iris(c, extra, found)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn check_iri(s: &str, extra: &[&str], found: &mut Vec<String>) -> Result<(), NonPublicOntology> {
    if s.starts_with('@') || s.is_empty() || !looks_like_iri(s) {
        return Ok(());
    }
    if is_allowed(s, extra) {
        found.push(s.to_string());
        Ok(())
    } else {
        Err(NonPublicOntology(s.to_string()))
    }
}

fn looks_like_iri(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://") || s.starts_with("urn:")
}

fn is_allowed(iri: &str, extra: &[&str]) -> bool {
    ALLOWED_PREFIXES.iter().any(|p| iri.starts_with(p))
        || extra.iter().any(|p| iri.starts_with(p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn allows_prov_and_blake3_iris() {
        let v = json!({
            "@id": "urn:blake3:abc",
            "@type": "http://www.w3.org/ns/prov#Activity",
        });
        let found = audit_iris(&v, &[]).expect("public IRIs must be allowed");
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn rejects_example_org() {
        let v = json!({"@id": "http://example.org/foo"});
        let err = audit_iris(&v, &[]).expect_err("example.org must be rejected");
        assert_eq!(err.0, "http://example.org/foo");
    }

    #[test]
    fn rejects_private_ccog_internal() {
        let v = json!({"@type": "urn:ccog:internal:secret"});
        let err = audit_iris(&v, &[]).expect_err("internal namespace must be rejected");
        assert!(err.0.contains("internal"));
    }

    #[test]
    fn extra_allow_admits_custom_prefix() {
        let v = json!({"@id": "urn:test:fixture:1"});
        let found = audit_iris(&v, &["urn:test:fixture:"]).expect("extra_allow must work");
        assert_eq!(found.len(), 1);
    }
}
