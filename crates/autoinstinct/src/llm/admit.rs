//! Admission pipeline for LLM-emitted OCEL worlds.
//!
//! Treats `.response` as **adversarial untrusted output**. Every gate
//! rejects with a named [`LlmAdmissionError`] so the executive gauntlet
//! can attribute failures to a specific kill-zone reason.

use std::collections::HashSet;

use thiserror::Error;

use crate::doctrine::public_ontology_profiles;
use crate::llm::schema::OcelWorld;

/// Reasons a model output may be refused.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LlmAdmissionError {
    /// The response did not parse as JSON or violated the strict shape.
    #[error("shape gate: {0}")]
    Shape(String),
    /// World had zero objects or zero events.
    #[error("structural gate: {0}")]
    Structural(String),
    /// An event referenced an object id not declared in `objects`.
    #[error("dangling object reference: event {event} -> object {object}")]
    DanglingReference {
        /// Event id carrying the reference.
        event: String,
        /// Missing object id.
        object: String,
    },
    /// An IRI is outside the public-ontology allowlist.
    #[error("non-public ontology IRI: {0}")]
    NonPublicOntology(String),
    /// The world's `profile` did not match the caller's pack profile.
    #[error("profile mismatch: world says {world}, caller asked {caller}")]
    ProfileMismatch {
        /// Profile claimed by the model.
        world: String,
        /// Profile requested by the caller.
        caller: String,
    },
    /// An IRI looked PII-bearing (anything that isn't a public-ontology
    /// term or a `urn:blake3:` opaque token).
    #[error("PII-bearing IRI rejected: {0}")]
    PiiSuspected(String),
}

/// Admit a raw `.response` string.
///
/// Strips an optional UTF-8 BOM but rejects any markdown fence, prose, or
/// trailing junk so the prompt contract stays honest.
pub fn admit(response: &str, expected_profile: &str) -> Result<OcelWorld, LlmAdmissionError> {
    let trimmed = response.trim_start_matches('\u{feff}').trim();
    if trimmed.starts_with("```") {
        return Err(LlmAdmissionError::Shape(
            "markdown fence at start of response".into(),
        ));
    }
    if !(trimmed.starts_with('{') && trimmed.ends_with('}')) {
        return Err(LlmAdmissionError::Shape(
            "response is not a single top-level JSON object".into(),
        ));
    }
    let world: OcelWorld = serde_json::from_str(trimmed)
        .map_err(|e| LlmAdmissionError::Shape(e.to_string()))?;

    if world.profile != expected_profile {
        return Err(LlmAdmissionError::ProfileMismatch {
            world: world.profile.clone(),
            caller: expected_profile.to_string(),
        });
    }

    validate_structural(&world)?;
    validate_ontology(&world)?;
    validate_privacy(&world)?;
    Ok(world)
}

fn validate_structural(w: &OcelWorld) -> Result<(), LlmAdmissionError> {
    if w.objects.is_empty() {
        return Err(LlmAdmissionError::Structural(
            "world has zero objects".into(),
        ));
    }
    if w.events.is_empty() {
        return Err(LlmAdmissionError::Structural(
            "world has zero events".into(),
        ));
    }
    let known: HashSet<&str> = w.objects.iter().map(|o| o.id.as_str()).collect();
    for ev in &w.events {
        if ev.objects.is_empty() {
            return Err(LlmAdmissionError::Structural(format!(
                "event {} has zero object links",
                ev.id
            )));
        }
        for o in &ev.objects {
            if !known.contains(o.as_str()) {
                return Err(LlmAdmissionError::DanglingReference {
                    event: ev.id.clone(),
                    object: o.clone(),
                });
            }
        }
    }
    Ok(())
}

fn is_public_iri(iri: &str) -> bool {
    public_ontology_profiles().iter().any(|p| iri.starts_with(p))
}

fn validate_ontology(w: &OcelWorld) -> Result<(), LlmAdmissionError> {
    for o in &w.objects {
        check_ontology_term(&o.ontology_type)?;
    }
    for e in &w.events {
        check_ontology_term(&e.ontology_type)?;
    }
    Ok(())
}

fn check_ontology_term(iri: &str) -> Result<(), LlmAdmissionError> {
    if is_public_iri(iri) {
        Ok(())
    } else {
        Err(LlmAdmissionError::NonPublicOntology(iri.to_string()))
    }
}

/// Reject anything that walks like a PII-bearing IRI: a URL-style IRI
/// that isn't on the public-ontology allowlist, or a `urn:` other than
/// `urn:blake3:` / `urn:ccog:vocab:`.
fn validate_privacy(w: &OcelWorld) -> Result<(), LlmAdmissionError> {
    for o in &w.objects {
        check_id_privacy(&o.id, |i| LlmAdmissionError::PiiSuspected(i.to_string()))?;
    }
    for e in &w.events {
        check_id_privacy(&e.id, |i| LlmAdmissionError::PiiSuspected(i.to_string()))?;
    }
    Ok(())
}

fn check_id_privacy<F>(id: &str, mk: F) -> Result<(), LlmAdmissionError>
where
    F: Fn(&str) -> LlmAdmissionError,
{
    let looks_like_iri = id.contains("://") || id.starts_with("urn:");
    if !looks_like_iri {
        // Plain identifier (like "pallet-1") is fine; only events/objects
        // that *do* surface a URI must satisfy the allowlist.
        return Ok(());
    }
    if is_public_iri(id) {
        Ok(())
    } else {
        Err(mk(id))
    }
}

