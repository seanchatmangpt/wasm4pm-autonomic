use crate::models::petri_net::{Arc, PetriNet};
use crate::models::{EventLog, Trace};
use crate::utils::dense_kernel::{fnv1a_64, DenseIndex, NodeKind, PackedKeyTable};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

pub mod bitmask_replay;
pub mod case_centric;
pub mod token_replay;
pub mod trace_generator;

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
    pub violation_count: usize,
}

impl ProjectedLog {
    pub fn generate(log: &EventLog) -> Self {
        Self::generate_with_ontology(log, None)
    }

    pub fn generate_with_ontology(
        log: &EventLog,
        ontology: Option<&crate::models::Ontology>,
    ) -> Self {
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
            unique_activities
                .into_iter()
                .map(|s| (s, NodeKind::Generic)),
        )
        .expect("Collision in activity names");

        let mut traces_map = PackedKeyTable::new();
        let activities = activity_index.symbols().to_vec();

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
                        continue;
                    }
                }

                if let Some(idx) = activity_index.dense_id_by_symbol(activity) {
                    trace_acts.push(idx as usize);
                }
            }

            if trace_acts.is_empty() {
                continue;
            }

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
            violation_count,
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
    if num_places > 64 {
        return 0.0;
    }

    let mut place_to_idx = PackedKeyTable::with_capacity(num_places);
    for (i, p) in petri_net.places.iter().enumerate() {
        place_to_idx.insert(fnv1a_64(p.id.as_bytes()), p.id.clone(), i);
    }

    let num_transitions = petri_net.transitions.len();
    let dummy_t_idx = num_transitions;

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
                    trans_masks[t_idx].in_mask |= 1u64 << p_idx;
                } else {
                    trans_masks[t_idx].out_mask |= 1u64 << p_idx;
                }
            }
        }
    }

    for tm in trans_masks.iter_mut().take(num_transitions) {
        tm.in_count = tm.in_mask.count_ones();
        tm.out_count = tm.out_mask.count_ones();
    }

    let mut initial_mask = 0u64;
    for (_, p_id, c) in petri_net.initial_marking.iter() {
        if *c > 0 {
            if let Some(&p_idx) = place_to_idx.get(fnv1a_64(p_id.as_bytes())) {
                initial_mask |= 1u64 << p_idx;
            }
        }
    }

    let mut final_mask = 0u64;
    if let Some(fm) = petri_net.final_markings.first() {
        for (_, p_id, c) in fm.iter() {
            if *c > 0 {
                if let Some(&p_idx) = place_to_idx.get(fnv1a_64(p_id.as_bytes())) {
                    final_mask |= 1u64 << p_idx;
                }
            }
        }
    }
    let final_count = final_mask.count_ones();

    let mut act_to_t_idx = vec![dummy_t_idx; log.activities.len()];
    for (i, act) in log.activities.iter().enumerate() {
        if let Some(pos) = petri_net.transitions.iter().position(|t| &t.label == act) {
            act_to_t_idx[i] = pos;
        }
    }

    let mut total_fitness = 0.0;
    let mut total_freq = 0;

    for (trace, freq) in &log.traces {
        let mut marking: u64 = initial_mask;
        let mut missing_tokens = 0;
        let mut consumed_tokens = 0;
        let mut produced_tokens = initial_mask.count_ones();

        for &act_idx in trace {
            if act_idx >= act_to_t_idx.len() {
                continue; // guard: trace has unknown activity
            }
            unsafe {
                let t_idx = *act_to_t_idx.get_unchecked(act_idx);
                let tm = trans_masks.get_unchecked(t_idx);
                missing_tokens += (tm.in_mask & !marking).count_ones();
                marking = (marking & !tm.in_mask) | tm.out_mask;
                consumed_tokens += tm.in_count;
                produced_tokens += tm.out_count;
            }
        }

        missing_tokens += (final_mask & !marking).count_ones();
        consumed_tokens += final_count;
        marking &= !final_mask;
        let remaining_tokens = marking.count_ones();

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
    if num_places > 64 {
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
                    trans_masks[t_idx].in_mask |= 1u64 << p_idx;
                } else {
                    trans_masks[t_idx].out_mask |= 1u64 << p_idx;
                }
            }
        }
    }

    for tm in trans_masks.iter_mut().take(num_transitions) {
        tm.in_count = tm.in_mask.count_ones();
        tm.out_count = tm.out_mask.count_ones();
    }

    let mut initial_mask = 0u64;
    for (_, p_id, c) in petri_net.initial_marking.iter() {
        if *c > 0 {
            if let Some(&p_idx) = place_to_idx.get(fnv1a_64(p_id.as_bytes())) {
                initial_mask |= 1u64 << p_idx;
            }
        }
    }

    let mut final_mask = 0u64;
    if let Some(fm) = petri_net.final_markings.first() {
        for (_, p_id, c) in fm.iter() {
            if *c > 0 {
                if let Some(&p_idx) = place_to_idx.get(fnv1a_64(p_id.as_bytes())) {
                    final_mask |= 1u64 << p_idx;
                }
            }
        }
    }
    let final_count = final_mask.count_ones();

    log.traces
        .iter()
        .map(|trace| {
            let mut marking: u64 = initial_mask;
            let mut missing_tokens = 0;
            let mut consumed_tokens = 0;
            let mut produced_tokens = initial_mask.count_ones();

            for event in &trace.events {
                let mut t_idx = dummy_t_idx;
                if let Some(attr) = event.attributes.iter().find(|a| a.key == "concept:name") {
                    if let crate::models::AttributeValue::String(s) = &attr.value {
                        if let Some(&idx) = act_to_t_idx.get(fnv1a_64(s.as_bytes())) {
                            t_idx = idx;
                        }
                    }
                }

                unsafe {
                    let tm = trans_masks.get_unchecked(t_idx);
                    let missing = tm.in_mask & !marking;
                    missing_tokens += missing.count_ones();
                    marking = (marking & !tm.in_mask) | tm.out_mask;
                    consumed_tokens += tm.in_count;
                    produced_tokens += tm.out_count;
                }
            }

            let missing_final = final_mask & !marking;
            missing_tokens += missing_final.count_ones();
            consumed_tokens += final_count;
            marking &= !final_mask;
            let remaining_tokens = marking.count_ones();

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
