//! Pack registry.
//!
//! The registry is an in-memory index of compiled `FieldPackArtifact`s
//! keyed by `(name, digest_urn)` so two packs with the same name but
//! different content cannot collide. Persistence is JSON-LD via the
//! existing serde derives — no separate format.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::compile::FieldPackArtifact;

/// Errors raised by registry operations.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RegistryError {
    /// Re-registration with same key but different bytes.
    #[error("conflict: pack {name} digest {existing} vs {incoming}")]
    Conflict {
        /// Pack name.
        name: String,
        /// Already-registered digest.
        existing: String,
        /// Incoming digest.
        incoming: String,
    },
}

/// Compiled-pack registry.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PackRegistry {
    /// Packs keyed by name → digest → artifact. Insertion order preserved.
    pub packs: IndexMap<String, IndexMap<String, FieldPackArtifact>>,
}

impl PackRegistry {
    /// New empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a pack. Idempotent if `(name, digest)` already exists with
    /// identical content; errors if `name` is registered with a different
    /// digest at the same insertion key.
    pub fn register(&mut self, pack: FieldPackArtifact) -> Result<(), RegistryError> {
        let name = pack.name.clone();
        let digest = pack.digest_urn.clone();
        let inner = self.packs.entry(name.clone()).or_default();
        if let Some(existing) = inner.get(&digest) {
            if existing != &pack {
                return Err(RegistryError::Conflict {
                    name,
                    existing: digest.clone(),
                    incoming: pack.digest_urn,
                });
            }
            return Ok(());
        }
        inner.insert(digest, pack);
        Ok(())
    }

    /// Look up a specific version by digest URN.
    #[must_use]
    pub fn get(&self, name: &str, digest_urn: &str) -> Option<&FieldPackArtifact> {
        self.packs.get(name).and_then(|m| m.get(digest_urn))
    }

    /// Number of registered packs (across all names + digests).
    #[must_use]
    pub fn len(&self) -> usize {
        self.packs.values().map(|m| m.len()).sum()
    }

    /// True iff registry has no packs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::{compile, CompileInputs};
    use crate::synth::CandidatePolicy;
    use crate::AutonomicInstinct;

    fn pack(name: &str, default: AutonomicInstinct) -> FieldPackArtifact {
        let policy = CandidatePolicy {
            rules: vec![],
            default,
        };
        compile(CompileInputs {
            name,
            ontology_profile: &[],
            admitted_breeds: &[],
            policy: &policy,
        })
    }

    #[test]
    fn registry_round_trips_pack() {
        let mut r = PackRegistry::new();
        let p = pack("test", AutonomicInstinct::Ignore);
        r.register(p.clone()).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r.get(&p.name, &p.digest_urn).unwrap(), &p);
    }

    #[test]
    fn registry_idempotent_on_same_digest() {
        let mut r = PackRegistry::new();
        let p = pack("test", AutonomicInstinct::Ignore);
        r.register(p.clone()).unwrap();
        r.register(p.clone()).unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn registry_separates_versions_by_digest() {
        let mut r = PackRegistry::new();
        let p1 = pack("test", AutonomicInstinct::Ignore);
        let p2 = pack("test", AutonomicInstinct::Ask);
        r.register(p1).unwrap();
        r.register(p2).unwrap();
        assert_eq!(r.len(), 2, "different defaults → different digests → separate slots");
    }
}
