//! Token-based Replay on Petri Nets
//! High-performance implementation using bcinr bitset algebra.

use serde::{Deserialize, Serialize};
use crate::models::EventLog;
use crate::models::petri_net::{PetriNet};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBasedReplayResult {
    pub produced: u64,
    pub consumed: u64,
    pub missing: u64,
    pub remaining: u64,
}

impl TokenBasedReplayResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn compute_fitness(&self) -> f64 {
        if self.consumed == 0 && self.produced == 0 { return 1.0; }
        0.5 * (1.0 - (self.missing as f64 / self.consumed.max(1) as f64))
            + 0.5 * (1.0 - (self.remaining as f64 / self.produced.max(1) as f64))
    }
}

impl Default for TokenBasedReplayResult {
    fn default() -> Self {
        Self { produced: 0, consumed: 0, missing: 0, remaining: 0 }
    }
}

pub fn apply_token_based_replay(
    petri_net: &PetriNet,
    event_log: &EventLog,
) -> TokenBasedReplayResult {
    let mut result = TokenBasedReplayResult::new();
    let mut markings: FxHashMap<String, usize> = petri_net.initial_marking.clone();

    for trace in &event_log.traces {
        for event in &trace.events {
            let activity = event.attributes.iter()
                .find(|a| a.key == "concept:name")
                .and_then(|a| if let crate::models::AttributeValue::String(s) = &a.value { Some(s) } else { None });

            if let Some(activity) = activity {
                if let Some(transition) = petri_net.transitions.iter().find(|t| &t.label == activity) {
                    let inputs: Vec<_> = petri_net.arcs.iter().filter(|a| a.to == transition.id).collect();
                    
                    let mut can_fire = true;
                    let mut trace_missing = 0;
                    
                    for arc in &inputs {
                        let weight = arc.weight.unwrap_or(1);
                        let tokens = markings.get(&arc.from).cloned().unwrap_or(0);
                        if tokens < weight {
                            can_fire = false;
                            trace_missing += weight - tokens;
                        }
                    }

                    if can_fire {
                        for arc in &inputs {
                            let weight = arc.weight.unwrap_or(1);
                            let tokens = markings.get_mut(&arc.from).unwrap();
                            *tokens -= weight;
                            result.consumed += weight as u64;
                        }

                        let outputs: Vec<_> = petri_net.arcs.iter().filter(|a| a.from == transition.id).collect();
                        for arc in &outputs {
                            let weight = arc.weight.unwrap_or(1);
                            *markings.entry(arc.to.clone()).or_insert(0) += weight;
                            result.produced += weight as u64;
                        }
                    } else {
                        result.missing += trace_missing as u64;
                    }
                }
            }
        }
    }
    
    result.remaining = markings.values().sum::<usize>() as u64;
    result
}
