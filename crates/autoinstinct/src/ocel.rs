//! Object-Centric Event Log (OCEL 2.0) generation + validation.
//!
//! AutoInstinct treats OCEL logs as synthetic operational worlds, not flat
//! datasets. An LLM (or scripted scenario generator) emits OCEL JSON; this
//! module validates ontology profile alignment, public-namespace IRIs, and
//! object/event consistency before the log is admitted into the corpus.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One OCEL object instance.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OcelObject {
    /// Stable IRI for the object — must be public-ontology rooted.
    pub iri: String,
    /// Object type IRI (e.g. `https://schema.org/Vehicle`).
    pub object_type: String,
}

/// One OCEL event.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OcelEvent {
    /// Event IRI.
    pub iri: String,
    /// Event-type IRI.
    pub event_type: String,
    /// IRIs of objects this event refers to.
    pub objects: Vec<String>,
    /// ISO-8601 timestamp.
    pub timestamp: String,
}

/// OCEL 2.0 log.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OcelLog {
    /// Object instances by IRI.
    pub objects: Vec<OcelObject>,
    /// Events in temporal order.
    pub events: Vec<OcelEvent>,
}

/// Errors raised by OCEL validation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum OcelError {
    /// An IRI was outside the public-ontology allowlist.
    #[error("non-public IRI: {0}")]
    NonPublicIri(String),
    /// An event referenced an object not declared in the log.
    #[error("dangling object reference in event {event}: {object}")]
    DanglingReference {
        /// Event IRI carrying the bad reference.
        event: String,
        /// Object IRI that was missing.
        object: String,
    },
    /// Object IRI duplicated.
    #[error("duplicate object iri: {0}")]
    DuplicateObject(String),
}

/// Public-ontology allowlist mirroring `ccog::export::ontology`. AutoInstinct
/// re-checks because OCEL logs are external input that may not have passed
/// through ccog yet.
const PUBLIC_PREFIXES: &[&str] = &[
    "http://www.w3.org/",
    "https://schema.org/",
    "http://purl.org/",
    "urn:blake3:",
    "urn:ccog:",
];

fn is_public_iri(iri: &str) -> bool {
    PUBLIC_PREFIXES.iter().any(|p| iri.starts_with(p))
}

/// Validate `log` for ontology purity, object/event integrity, and unique
/// object IRIs. Errors stop at the first violation.
pub fn validate(log: &OcelLog) -> Result<(), OcelError> {
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for obj in &log.objects {
        if !is_public_iri(&obj.iri) {
            return Err(OcelError::NonPublicIri(obj.iri.clone()));
        }
        if !is_public_iri(&obj.object_type) {
            return Err(OcelError::NonPublicIri(obj.object_type.clone()));
        }
        if !seen.insert(&obj.iri) {
            return Err(OcelError::DuplicateObject(obj.iri.clone()));
        }
    }
    for ev in &log.events {
        if !is_public_iri(&ev.iri) {
            return Err(OcelError::NonPublicIri(ev.iri.clone()));
        }
        if !is_public_iri(&ev.event_type) {
            return Err(OcelError::NonPublicIri(ev.event_type.clone()));
        }
        for o in &ev.objects {
            if !seen.contains(o.as_str()) {
                return Err(OcelError::DanglingReference {
                    event: ev.iri.clone(),
                    object: o.clone(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(iri: &str, t: &str) -> OcelObject {
        OcelObject {
            iri: iri.into(),
            object_type: t.into(),
        }
    }

    fn ev(iri: &str, t: &str, objs: &[&str]) -> OcelEvent {
        OcelEvent {
            iri: iri.into(),
            event_type: t.into(),
            objects: objs.iter().map(|s| (*s).to_string()).collect(),
            timestamp: "2026-04-29T00:00:00Z".into(),
        }
    }

    #[test]
    fn ocel_validates_public_ontology() {
        let log = OcelLog {
            objects: vec![obj(
                "https://schema.org/Vehicle/v1",
                "https://schema.org/Vehicle",
            )],
            events: vec![ev(
                "urn:blake3:e1",
                "https://schema.org/Action",
                &["https://schema.org/Vehicle/v1"],
            )],
        };
        assert!(validate(&log).is_ok());
    }

    #[test]
    fn ocel_rejects_private_namespace() {
        let log = OcelLog {
            objects: vec![obj(
                "http://internal.example/v1",
                "https://schema.org/Vehicle",
            )],
            events: vec![],
        };
        assert!(matches!(validate(&log), Err(OcelError::NonPublicIri(_))));
    }

    #[test]
    fn ocel_rejects_dangling_reference() {
        let log = OcelLog {
            objects: vec![],
            events: vec![ev(
                "urn:blake3:e1",
                "https://schema.org/Action",
                &["urn:blake3:does-not-exist"],
            )],
        };
        assert!(matches!(
            validate(&log),
            Err(OcelError::DanglingReference { .. })
        ));
    }

    #[test]
    fn ocel_rejects_duplicate_object_iri() {
        let log = OcelLog {
            objects: vec![
                obj("urn:blake3:o1", "https://schema.org/Thing"),
                obj("urn:blake3:o1", "https://schema.org/Thing"),
            ],
            events: vec![],
        };
        assert!(matches!(validate(&log), Err(OcelError::DuplicateObject(_))));
    }
}
