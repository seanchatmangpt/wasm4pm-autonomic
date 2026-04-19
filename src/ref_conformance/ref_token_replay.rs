use std::collections::HashMap;
use crate::ref_models::ref_petri_net::{PetriNet, Marking, PlaceID, ArcType};
use crate::ref_models::ref_event_log::EventLogActivityProjection;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct TokenBasedReplayResult {
    pub produced: u64,
    pub consumed: u64,
    pub missing: u64,
    pub remaining: u64,
}

impl TokenBasedReplayResult {
    pub fn compute_fitness(&self) -> f64 {
        if self.consumed == 0 && self.produced == 0 { return 1.0; }
        0.5 * (1.0 - (self.missing as f64 / self.consumed.max(1) as f64))
            + 0.5 * (1.0 - (self.remaining as f64 / self.produced.max(1) as f64))
    }
}

/// Standard implementation following rust4pm logic.
pub fn apply_token_based_replay_standard(
    petri_net: &PetriNet,
    event_log: &EventLogActivityProjection,
) -> TokenBasedReplayResult {
    let mut result = TokenBasedReplayResult::default();
    let node_to_pos = petri_net.create_vector_dictionary();
    let num_places = petri_net.places.len();
    
    let trans_mapping: Vec<Option<usize>> = event_log.activities.iter()
        .map(|act| {
            petri_net.transitions.values()
                .find(|t| t.label.as_ref() == Some(act))
                .map(|t| *node_to_pos.get(&t.id).unwrap())
        })
        .collect();

    let initial_marking = petri_net.initial_marking.as_ref().expect("No initial marking");
    let final_marking = petri_net.final_markings.as_ref().and_then(|f| f.first()).expect("No final marking");

    for (trace, freq) in &event_log.traces {
        let mut marking: Vec<i64> = vec![0; num_places];
        for (p_id, count) in initial_marking {
            marking[*node_to_pos.get(&p_id.0).unwrap()] += *count as i64;
            result.produced += *count * freq;
        }

        for &act_idx in trace {
            if let Some(trans_idx) = trans_mapping[act_idx] {
                // Find arcs (O(N) search in standard)
                for arc in &petri_net.arcs {
                    match arc.from_to {
                        ArcType::PlaceTransition(from, to) => {
                            if *node_to_pos.get(&to).unwrap() == trans_idx {
                                let p_pos = *node_to_pos.get(&from).unwrap();
                                let weight = arc.weight as i64;
                                if marking[p_pos] < weight {
                                    result.missing += (weight - marking[p_pos]) as u64 * freq;
                                    marking[p_pos] = 0;
                                } else {
                                    marking[p_pos] -= weight;
                                }
                                result.consumed += arc.weight as u64 * freq;
                            }
                        }
                        ArcType::TransitionPlace(from, to) => {
                            if *node_to_pos.get(&from).unwrap() == trans_idx {
                                let p_pos = *node_to_pos.get(&to).unwrap();
                                marking[p_pos] += arc.weight as i64;
                                result.produced += arc.weight as u64 * freq;
                            }
                        }
                    }
                }
            }
        }

        for (p_id, count) in final_marking {
            let p_pos = *node_to_pos.get(&p_id.0).unwrap();
            let weight = *count as i64;
            if marking[p_pos] < weight {
                result.missing += (weight - marking[p_pos]) as u64 * freq;
                marking[p_pos] = 0;
            } else {
                marking[p_pos] -= weight;
            }
            result.consumed += *count * freq;
        }
        result.remaining += marking.iter().filter(|&&c| c > 0).map(|&c| c as u64).sum::<u64>() * freq;
    }
    result
}

/// Optimized implementation using bcinr-style pre-computed connectivity vectors.
pub fn apply_token_based_replay_optimized(
    petri_net: &PetriNet,
    event_log: &EventLogActivityProjection,
) -> TokenBasedReplayResult {
    let mut result = TokenBasedReplayResult::default();
    let num_places = petri_net.places.len();
    let num_transitions = petri_net.transitions.len();
    let node_to_pos = petri_net.create_vector_dictionary();

    // Mapping for places and transitions separately to avoid offset issues
    let mut place_to_idx = HashMap::new();
    let mut trans_to_idx = HashMap::new();
    
    for (i, id) in petri_net.places.keys().enumerate() {
        place_to_idx.insert(*id, i);
    }
    for (i, id) in petri_net.transitions.keys().enumerate() {
        trans_to_idx.insert(*id, i);
    }

    // 1. Pre-compute Connectivity Vectors
    let mut inputs: Vec<Vec<(usize, i64)>> = vec![Vec::new(); num_transitions];
    let mut outputs: Vec<Vec<(usize, i64)>> = vec![Vec::new(); num_transitions];

    for arc in &petri_net.arcs {
        match arc.from_to {
            ArcType::PlaceTransition(from, to) => {
                if let (Some(&p_idx), Some(&t_idx)) = (place_to_idx.get(&from), trans_to_idx.get(&to)) {
                    inputs[t_idx].push((p_idx, arc.weight as i64));
                }
            }
            ArcType::TransitionPlace(from, to) => {
                if let (Some(&t_idx), Some(&p_idx)) = (trans_to_idx.get(&from), place_to_idx.get(&to)) {
                    outputs[t_idx].push((p_idx, arc.weight as i64));
                }
            }
        }
    }

    // 2. Map Activities
    let trans_mapping: Vec<Option<usize>> = event_log.activities.iter()
        .map(|act| {
            petri_net.transitions.values()
                .find(|t| t.label.as_ref() == Some(act))
                .and_then(|t| trans_to_idx.get(&t.id).cloned())
        })
        .collect();

    let initial_marking: Vec<(usize, i64)> = petri_net.initial_marking.as_ref().unwrap().iter()
        .map(|(p, c)| (*place_to_idx.get(&p.0).unwrap(), *c as i64)).collect();
    let final_marking: Vec<(usize, i64)> = petri_net.final_markings.as_ref().unwrap().first().unwrap().iter()
        .map(|(p, c)| (*place_to_idx.get(&p.0).unwrap(), *c as i64)).collect();

    // 3. Fast Replay Loop
    for (trace, freq) in &event_log.traces {
        let mut marking = vec![0i64; num_places];
        for (p_idx, count) in &initial_marking {
            marking[*p_idx] += *count;
            result.produced += (*count as u64) * freq;
        }

        for &act_idx in trace {
            if let Some(t_idx) = trans_mapping[act_idx] {
                for (p_idx, weight) in &inputs[t_idx] {
                    if marking[*p_idx] < *weight {
                        result.missing += (*weight - marking[*p_idx]) as u64 * freq;
                        marking[*p_idx] = 0;
                    } else {
                        marking[*p_idx] -= *weight;
                    }
                    result.consumed += (*weight as u64) * freq;
                }
                for (p_idx, weight) in &outputs[t_idx] {
                    marking[*p_idx] += *weight;
                    result.produced += (*weight as u64) * freq;
                }
            }
        }

        for (p_idx, count) in &final_marking {
            if marking[*p_idx] < *count {
                result.missing += (*count - marking[*p_idx]) as u64 * freq;
                marking[*p_idx] = 0;
            } else {
                marking[*p_idx] -= *count;
            }
            result.consumed += (*count as u64) * freq;
        }
        result.remaining += marking.iter().filter(|&&c| c > 0).map(|&c| c as u64).sum::<u64>() * freq;
    }
    result
}
