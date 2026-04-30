//! Phase 8 — Programmatic OCEL world generation.
//!
//! Generates ontology-grounded OCEL 2.0 worlds from a `ScenarioSpec`. The
//! generator is deterministic (BLAKE3-seeded RNG) so generated worlds
//! reproduce exactly given the same spec — load a generated world, run
//! discovery, run the gauntlet, admit or reject. No network calls; LLM
//! integration is layered on top via the spec.

use serde::{Deserialize, Serialize};

use crate::ocel::{validate, OcelEvent, OcelLog, OcelObject};

/// Generation spec.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScenarioSpec {
    /// Scenario name (folded into the seed).
    pub name: String,
    /// Object-type IRIs.
    pub object_types: Vec<String>,
    /// Event-type IRIs.
    pub event_types: Vec<String>,
    /// Number of objects per object_type.
    pub objects_per_type: u32,
    /// Number of events.
    pub events: u32,
}

/// Deterministic seedable RNG over BLAKE3.
struct Rng {
    state: [u8; 32],
}

impl Rng {
    fn new(seed: &str) -> Self {
        Self {
            state: *blake3::hash(seed.as_bytes()).as_bytes(),
        }
    }
    fn next_u32(&mut self) -> u32 {
        self.state = *blake3::hash(&self.state).as_bytes();
        u32::from_le_bytes(self.state[..4].try_into().unwrap())
    }
    fn pick<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        &slice[(self.next_u32() as usize) % slice.len().max(1)]
    }
}

/// Generate an OCEL log from a spec. Validates before returning.
pub fn generate(spec: &ScenarioSpec) -> Result<OcelLog, crate::ocel::OcelError> {
    let mut rng = Rng::new(&spec.name);
    let mut objects: Vec<OcelObject> = Vec::new();
    for ot in &spec.object_types {
        for i in 0..spec.objects_per_type {
            objects.push(OcelObject {
                iri: format!("{ot}/{i:08x}"),
                object_type: ot.clone(),
            });
        }
    }
    let mut events: Vec<OcelEvent> = Vec::new();
    for i in 0..spec.events {
        let event_type = if spec.event_types.is_empty() {
            "https://schema.org/Action".to_string()
        } else {
            spec.event_types[(rng.next_u32() as usize) % spec.event_types.len()].clone()
        };
        // Attach 1-3 objects (deterministic via rng).
        let attach_n = (rng.next_u32() % 3 + 1) as usize;
        let mut attached: Vec<String> = Vec::with_capacity(attach_n);
        for _ in 0..attach_n {
            if !objects.is_empty() {
                attached.push(rng.pick(&objects).iri.clone());
            }
        }
        // Deduplicate while preserving order.
        let mut seen: Vec<String> = Vec::new();
        for a in attached {
            if !seen.contains(&a) {
                seen.push(a);
            }
        }
        events.push(OcelEvent {
            iri: format!("urn:blake3:event-{:016x}", i),
            event_type,
            objects: seen,
            timestamp: format!("2026-04-29T{:02}:{:02}:00Z", (i / 60) % 24, i % 60),
        });
    }
    let log = OcelLog { objects, events };
    validate(&log)?;
    Ok(log)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> ScenarioSpec {
        ScenarioSpec {
            name: "supply-chain-dock".into(),
            object_types: vec![
                "https://schema.org/Vehicle".into(),
                "https://schema.org/Place".into(),
            ],
            event_types: vec![
                "https://schema.org/Action".into(),
                "https://schema.org/CheckAction".into(),
            ],
            objects_per_type: 3,
            events: 10,
        }
    }

    #[test]
    fn world_gen_is_deterministic() {
        let a = generate(&spec()).unwrap();
        let b = generate(&spec()).unwrap();
        assert_eq!(a.objects.len(), b.objects.len());
        assert_eq!(a.events.len(), b.events.len());
        for (e1, e2) in a.events.iter().zip(b.events.iter()) {
            assert_eq!(e1, e2);
        }
    }

    #[test]
    fn world_gen_validates_public_ontology() {
        let log = generate(&spec()).unwrap();
        validate(&log).expect("generated worlds must pass validation");
    }

    #[test]
    fn world_gen_changes_with_spec_name() {
        let a = generate(&spec()).unwrap();
        let mut s2 = spec();
        s2.name = "different".into();
        let b = generate(&s2).unwrap();
        // At least one event differs.
        assert!(a
            .events
            .iter()
            .zip(b.events.iter())
            .any(|(e1, e2)| e1.event_type != e2.event_type
                || e1.objects != e2.objects));
    }

    #[test]
    fn world_gen_rejects_private_ontology() {
        let mut s = spec();
        s.object_types.push("http://internal.example/Bad".into());
        assert!(generate(&s).is_err());
    }
}
