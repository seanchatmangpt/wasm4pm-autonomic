use crate::models::petri_net::{Arc, PetriNet};
use crate::models::{EventLog, Trace};
use crate::utils::dense_kernel::{fnv1a_64, PackedKeyTable, DenseIndex, NodeKind};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

pub mod case_centric;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenReplayDeviation {
    pub event_index: usize,
    pub activity: String,
    pub deviation_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceResult {
    pub case_id: String,
    pub fitness: f64,
    pub deviations: Vec<TokenReplayDeviation>,
}

/// A pre-projected log for high-performance discovery.
pub struct ProjectedLog {
    pub activities: Vec<String>,
    pub traces: Vec<(Vec<usize>, u64)>, // (activity indices, frequency)
<<<<<<< HEAD
    pub violation_count: usize,
=======
    pub ontology_hash: u64,
>>>>>>> wreckit/ontology-mapping-automated-activity-to-index-mapping-with-fnv-1a-collision-guards
}

impl ProjectedLog {
    pub fn generate(log: &EventLog) -> Self {
        Self::generate_with_ontology(log, None)
    }

    pub fn generate_with_ontology(log: &EventLog, ontology: Option<&crate::models::Ontology>) -> Self {
<<<<<<< HEAD
        let mut unique_activities = std::collections::HashSet::new();
        let mut violation_count = 0;

        for trace in &log.traces {
            for event in &trace.events {
                let activity = event
                    .attributes
                    .iter()
                    .find(|a| a.key == "concept:name")
                    .and_then(|a| {
                        if let crate::models::AttributeValue::String(s) = &a.value {
                            Some(s.as_str())
                        } else {
                            None
                        }
                    })
                    .unwrap_or("No Activity");

                if let Some(ont) = ontology {
                    if !ont.contains(activity) {
                        violation_count += 1;
                        continue; // Prune out-of-ontology events (AC 1.2 option b)
                    }
                }
                unique_activities.insert(activity.to_string());
            }
        }

        let activity_index = DenseIndex::compile(
            unique_activities.into_iter().map(|s| (s, NodeKind::Generic))
        ).expect("Collision in activity names");

        let mut traces_map = PackedKeyTable::new();
        let activities = activity_index.symbols().to_vec();
<<<<<<< HEAD
=======
        let mut act_to_idx = PackedKeyTable::new();
        let mut activities = Vec::new();
        let mut traces_map = PackedKeyTable::new();
        let mut violation_count = 0;
>>>>>>> wreckit/1-formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o-and-prevent-out-of-ontology-state-reachability
=======
        let ontology_hash = activity_index.ontology_hash();
>>>>>>> wreckit/ontology-mapping-automated-activity-to-index-mapping-with-fnv-1a-collision-guards

        for trace in &log.traces {
            let mut trace_acts = Vec::with_capacity(trace.events.len());
            for event in &trace.events {
                let activity = event
                    .attributes
                    .iter()
                    .find(|a| a.key == "concept:name")
                    .and_then(|a| {
                        if let crate::models::AttributeValue::String(s) = &a.value {
                            Some(s.as_str())
                        } else {
                            None
                        }
                    })
                    .unwrap_or("No Activity");

                if let Some(ont) = ontology {
                    if !ont.contains(activity) {
<<<<<<< HEAD
                        continue;
                    }
                }

                if let Some(idx) = activity_index.dense_id_by_symbol(activity) {
                    trace_acts.push(idx as usize);
                }
=======
                        violation_count += 1;
                        continue; // Prune out-of-ontology events (AC 1.2 option b)
                    }
                }

                let h = fnv1a_64(activity.as_bytes());
                let index = if let Some(&idx) = act_to_idx.get(h) {
                    idx
                } else {
                    let idx = activities.len();
                    activities.push(activity.to_string());
                    act_to_idx.insert(h, activity.to_string(), idx);
                    idx
                };
                trace_acts.push(index);
>>>>>>> wreckit/1-formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o-and-prevent-out-of-ontology-state-reachability
            }

            if trace_acts.is_empty() { continue; }

            let mut hasher = rustc_hash::FxHasher::default();
            trace_acts.hash(&mut hasher);
            let h = hasher.finish();
            if let Some(freq) = traces_map.get_mut(h) {
                *freq += 1;
            } else {
                traces_map.insert(h, trace_acts, 1);
            }
        }

        Self {
            activities,
            traces: traces_map.iter().map(|(_, k, v)| (k.clone(), *v)).collect(),
<<<<<<< HEAD
            violation_count,
=======
            ontology_hash,
>>>>>>> wreckit/ontology-mapping-automated-activity-to-index-mapping-with-fnv-1a-collision-guards
        }
    }
}

impl From<&EventLog> for ProjectedLog {
    fn from(log: &EventLog) -> Self {
        Self::generate(log)
    }
}

pub fn token_replay_projected(log: &ProjectedLog, petri_net: &PetriNet) -> f64 {
    let num_places = petri_net.places.len();
    if num_places > 1024 {
        return 0.0;
    }

    let replay_data = if let Some(ref rd) = petri_net.cached_replay_data {
        rd
    } else {
        // Fallback or panic? For zero-allocation, we expect it to be cached.
        return 0.0;
    };

    let input_masks = &replay_data.input_masks;
    let output_masks = &replay_data.output_masks;
    let initial_mask = replay_data.initial_mask;
    let final_mask = replay_data.final_mask;

    let num_transitions = petri_net.transitions.len();
    let dummy_t_idx = num_transitions;

<<<<<<< HEAD
<<<<<<< HEAD
    #[derive(Clone, Copy)]
    struct TransMasks {
        in_mask: u64,
        out_mask: u64,
        in_count: u32,
        out_count: u32,
    }

    let mut trans_masks = vec![
        TransMasks {
            in_mask: 0,
            out_mask: 0,
            in_count: 0,
            out_count: 0,
        };
        num_transitions + 1
    ];
=======
    let mut input_masks = vec![crate::utils::dense_kernel::KBitSet::<16>::zero(); num_transitions + 1];
    let mut output_masks = vec![crate::utils::dense_kernel::KBitSet::<16>::zero(); num_transitions + 1];
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o

    for arc in &petri_net.arcs {
        let mut is_input = false;
        let t_idx_opt = if let Some(pos) = petri_net.transitions.iter().position(|t| t.id == arc.to)
        {
            is_input = true;
            Some(pos)
        } else {
            petri_net.transitions.iter().position(|t| t.id == arc.from)
        };

        if let Some(t_idx) = t_idx_opt {
            let p_id = if is_input { &arc.from } else { &arc.to };
            if let Some(&p_idx) = place_to_idx.get(fnv1a_64(p_id.as_bytes())) {
                if is_input {
<<<<<<< HEAD
                    trans_masks[t_idx].in_mask |= 1u64 << p_idx;
                } else {
                    trans_masks[t_idx].out_mask |= 1u64 << p_idx;
=======
                    let _ = input_masks[t_idx].set(p_idx);
                } else {
                    let _ = output_masks[t_idx].set(p_idx);
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
                }
            }
        }
    }

    for i in 0..num_transitions {
<<<<<<< HEAD
        trans_masks[i].in_count = trans_masks[i].in_mask.count_ones();
        trans_masks[i].out_count = trans_masks[i].out_mask.count_ones();
=======
        input_counts[i] = input_masks[i].pop_count();
        output_counts[i] = output_masks[i].pop_count();
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
    }

    let mut initial_mask = crate::utils::dense_kernel::KBitSet::<16>::zero();
    for (_, p_id, c) in petri_net.initial_marking.iter() {
        if *c > 0 {
            if let Some(&p_idx) = place_to_idx.get(fnv1a_64(p_id.as_bytes())) {
                let _ = initial_mask.set(p_idx);
            }
        }
    }

    let mut final_mask = crate::utils::dense_kernel::KBitSet::<16>::zero();
    if let Some(fm) = petri_net.final_markings.first() {
        for (_, p_id, c) in fm.iter() {
            if *c > 0 {
                if let Some(&p_idx) = place_to_idx.get(fnv1a_64(p_id.as_bytes())) {
                    let _ = final_mask.set(p_idx);
                }
            }
        }
    }
    let final_count = final_mask.pop_count();

    let mut act_to_t_idx = vec![dummy_t_idx; log.activities.len()];
    for (i, act) in log.activities.iter().enumerate() {
=======
    // Use a stack-allocated buffer for activity to transition mapping
    // KTier 1024 is the max, so 1024 * 4 bytes = 4KB.
    let mut act_to_t_idx = [dummy_t_idx; 1024];
    for (i, act) in log.activities.iter().enumerate().take(1024) {
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
        if let Some(pos) = petri_net.transitions.iter().position(|t| &t.label == act) {
            act_to_t_idx[i] = pos;
        }
    }

    let mut total_fitness = 0.0;
    let mut total_freq = 0;

    let final_count = final_mask.count_ones();

    for (trace, freq) in &log.traces {
        let mut marking = initial_mask;
        let mut missing_tokens = 0;
        let mut consumed_tokens = 0;
        let mut produced_tokens = initial_mask.pop_count();

        for &act_idx in trace {
<<<<<<< HEAD
<<<<<<< HEAD
            unsafe {
                let t_idx = *act_to_t_idx.get_unchecked(act_idx);
                let tm = trans_masks.get_unchecked(t_idx);
                missing_tokens += (tm.in_mask & !marking).count_ones();
                marking = (marking & !tm.in_mask) | tm.out_mask;
                consumed_tokens += tm.in_count;
                produced_tokens += tm.out_count;
            }
=======
            let t_idx = act_to_t_idx[act_idx];
            let in_mask = input_masks[t_idx];
            missing_tokens += marking.missing_count(in_mask);
            marking = marking.bitwise_and(in_mask.bitwise_not()).bitwise_or(output_masks[t_idx]);
            consumed_tokens += input_counts[t_idx];
            produced_tokens += output_counts[t_idx];
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
=======
            if act_idx >= 1024 {
                continue;
            }
            let t_idx = act_to_t_idx[act_idx];
            let in_mask = input_masks[t_idx];
            missing_tokens += (in_mask & !marking).count_ones();
            marking = (marking & !in_mask) | output_masks[t_idx];
            consumed_tokens += in_mask.count_ones();
            produced_tokens += output_masks[t_idx].count_ones();
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
        }

        missing_tokens += marking.missing_count(final_mask);
        consumed_tokens += final_count;
        marking = marking.bitwise_and(final_mask.bitwise_not());
        let remaining_tokens = marking.pop_count();

        let total_tokens_needed = consumed_tokens + missing_tokens;
        let fitness = if total_tokens_needed == 0 {
            1.0
        } else {
            1.0 - (missing_tokens as f64 + remaining_tokens as f64)
                / (total_tokens_needed as f64 + produced_tokens as f64)
        };

        total_fitness += fitness * (*freq as f64);
        total_freq += freq;
    }

    if total_freq == 0 {
        1.0
    } else {
        total_fitness / total_freq as f64
    }
}

pub fn token_replay(log: &EventLog, petri_net: &PetriNet) -> Vec<ConformanceResult> {
    let num_places = petri_net.places.len();
    if num_places > 1024 {
        return log
            .traces
            .iter()
            .map(|trace| replay_trace_standard(trace, petri_net))
            .collect();
    }

    let mut place_to_idx = PackedKeyTable::with_capacity(num_places);
    let mut act_to_t_idx = PackedKeyTable::with_capacity(petri_net.transitions.len());

    for (i, p) in petri_net.places.iter().enumerate() {
        place_to_idx.insert(fnv1a_64(p.id.as_bytes()), p.id.clone(), i);
    }
    for (i, t) in petri_net.transitions.iter().enumerate() {
        act_to_t_idx.insert(fnv1a_64(t.label.as_bytes()), t.label.clone(), i);
    }

    let num_transitions = petri_net.transitions.len();
    let dummy_t_idx = num_transitions;

<<<<<<< HEAD
    #[derive(Clone, Copy)]
    struct TransMasks {
        in_mask: u64,
        out_mask: u64,
        in_count: u32,
        out_count: u32,
    }

    let mut trans_masks = vec![
        TransMasks {
            in_mask: 0,
            out_mask: 0,
            in_count: 0,
            out_count: 0,
        };
        num_transitions + 1
    ];
=======
    let mut input_masks = vec![crate::utils::dense_kernel::KBitSet::<16>::zero(); num_transitions + 1];
    let mut output_masks = vec![crate::utils::dense_kernel::KBitSet::<16>::zero(); num_transitions + 1];
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o

    for arc in &petri_net.arcs {
        let mut is_input = false;
        let t_idx_opt = if petri_net.transitions.iter().any(|t| t.id == arc.to) {
            is_input = true;
            petri_net.transitions.iter().position(|t| t.id == arc.to)
        } else if petri_net.transitions.iter().any(|t| t.id == arc.from) {
            petri_net.transitions.iter().position(|t| t.id == arc.from)
        } else {
            None
        };

        if let Some(t_idx) = t_idx_opt {
            let p_id = if is_input { &arc.from } else { &arc.to };
            if let Some(&p_idx) = place_to_idx.get(fnv1a_64(p_id.as_bytes())) {
                if is_input {
<<<<<<< HEAD
                    trans_masks[t_idx].in_mask |= 1u64 << p_idx;
                } else {
                    trans_masks[t_idx].out_mask |= 1u64 << p_idx;
=======
                    let _ = input_masks[t_idx].set(p_idx);
                } else {
                    let _ = output_masks[t_idx].set(p_idx);
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
                }
            }
        }
    }

    for i in 0..num_transitions {
<<<<<<< HEAD
        trans_masks[i].in_count = trans_masks[i].in_mask.count_ones();
        trans_masks[i].out_count = trans_masks[i].out_mask.count_ones();
=======
        input_counts[i] = input_masks[i].pop_count();
        output_counts[i] = output_masks[i].pop_count();
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
    }

    let mut initial_mask = crate::utils::dense_kernel::KBitSet::<16>::zero();
    for (_, p_id, c) in petri_net.initial_marking.iter() {
        if *c > 0 {
            if let Some(&p_idx) = place_to_idx.get(fnv1a_64(p_id.as_bytes())) {
                let _ = initial_mask.set(p_idx);
            }
        }
    }

    let mut final_mask = crate::utils::dense_kernel::KBitSet::<16>::zero();
    if let Some(fm) = petri_net.final_markings.first() {
        for (_, p_id, c) in fm.iter() {
            if *c > 0 {
                if let Some(&p_idx) = place_to_idx.get(fnv1a_64(p_id.as_bytes())) {
                    let _ = final_mask.set(p_idx);
                }
            }
        }
    }
    let final_count = final_mask.pop_count();

    log.traces
        .iter()
        .map(|trace| {
            let mut marking = initial_mask;
            let mut missing_tokens = 0;
            let mut consumed_tokens = 0;
            let mut produced_tokens = initial_mask.pop_count();

            for event in &trace.events {
                let mut t_idx = dummy_t_idx;
                if let Some(attr) = event.attributes.iter().find(|a| a.key == "concept:name") {
                    if let crate::models::AttributeValue::String(s) = &attr.value {
                        if let Some(&idx) = act_to_t_idx.get(fnv1a_64(s.as_bytes())) {
                            t_idx = idx;
                        }
                    }
                }

<<<<<<< HEAD
                unsafe {
                    let tm = trans_masks.get_unchecked(t_idx);
                    let missing = tm.in_mask & !marking;
                    missing_tokens += missing.count_ones();
                    marking = (marking & !tm.in_mask) | tm.out_mask;
                    consumed_tokens += tm.in_count;
                    produced_tokens += tm.out_count;
                }
=======
                let in_mask = input_masks[t_idx];
                missing_tokens += marking.missing_count(in_mask);
                marking = marking.bitwise_and(in_mask.bitwise_not()).bitwise_or(output_masks[t_idx]);
                consumed_tokens += input_counts[t_idx];
                produced_tokens += output_counts[t_idx];
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
            }

            missing_tokens += marking.missing_count(final_mask);
            consumed_tokens += final_count;
            marking = marking.bitwise_and(final_mask.bitwise_not());
            let remaining_tokens = marking.pop_count();

            let total_tokens_needed = consumed_tokens + missing_tokens;
            let fitness = if total_tokens_needed == 0 {
                1.0
            } else {
                1.0 - (missing_tokens as f64 + remaining_tokens as f64)
                    / (total_tokens_needed as f64 + produced_tokens as f64)
            };

            ConformanceResult {
                case_id: trace.id.clone(),
                fitness,
                deviations: Vec::new(),
            }
        })
        .collect()
}

fn replay_trace_standard(trace: &Trace, petri_net: &PetriNet) -> ConformanceResult {
    let mut markings = petri_net.initial_marking.clone();
    let mut consumed_tokens = 0;
    let mut produced_tokens = 0;
    let mut missing_tokens = 0;

    for event in &trace.events {
        let activity = event
            .attributes
            .iter()
            .find(|a| a.key == "concept:name")
            .and_then(|a| {
                if let crate::models::AttributeValue::String(s) = &a.value {
                    Some(s)
                } else {
                    None
                }
            });

        if let Some(activity) = activity {
            if let Some(transition) = petri_net.transitions.iter().find(|t| &t.label == activity) {
                let input_arcs: Vec<&Arc> = petri_net
                    .arcs
                    .iter()
                    .filter(|a| a.to == transition.id)
                    .collect();
                let mut can_fire = true;
                for arc in &input_arcs {
                    let h = fnv1a_64(arc.from.as_bytes());
                    let token_count = markings.get(h).unwrap_or(&0);
                    if *token_count < arc.weight.unwrap_or(1) {
                        can_fire = false;
                        missing_tokens += arc.weight.unwrap_or(1) - *token_count;
                    }
                }

                if can_fire {
                    for arc in &input_arcs {
                        let h = fnv1a_64(arc.from.as_bytes());
                        let token_count = markings.get_mut(h).unwrap();
                        *token_count -= arc.weight.unwrap_or(1);
                        consumed_tokens += arc.weight.unwrap_or(1);
                    }
                    let output_arcs: Vec<&Arc> = petri_net
                        .arcs
                        .iter()
                        .filter(|a| a.from == transition.id)
                        .collect();
                    for arc in &output_arcs {
                        let h = fnv1a_64(arc.to.as_bytes());
                        if let Some(token_count) = markings.get_mut(h) {
                            *token_count += arc.weight.unwrap_or(1);
                        } else {
                            markings.insert(h, arc.to.clone(), arc.weight.unwrap_or(1));
                        }
                        produced_tokens += arc.weight.unwrap_or(1);
                    }
                } else {
                    missing_tokens += 1;
                }
            }
        }
    }

    let remaining_tokens: usize = markings.iter().map(|(_, _, v)| *v).sum();
    let total_tokens_needed = consumed_tokens + missing_tokens;
    let fitness = if total_tokens_needed == 0 {
        1.0
    } else {
        1.0 - (missing_tokens as f64 + remaining_tokens as f64)
            / (total_tokens_needed as f64 + produced_tokens as f64)
    };

    ConformanceResult {
        case_id: trace.id.clone(),
        fitness,
        deviations: Vec::new(),
    }
}
