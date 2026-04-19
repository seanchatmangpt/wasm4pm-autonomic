use serde::{Deserialize, Serialize};
use crate::models::{EventLog, Trace};
use crate::models::petri_net::{PetriNet, Arc};
use rustc_hash::FxHashMap;

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
}

impl From<&EventLog> for ProjectedLog {
    fn from(log: &EventLog) -> Self {
        let mut act_to_idx = FxHashMap::default();
        let mut activities = Vec::new();
        let mut traces_map = FxHashMap::default();

        for trace in &log.traces {
            let mut trace_acts = Vec::with_capacity(trace.events.len());
            for event in &trace.events {
                let activity = event.attributes.iter()
                    .find(|a| a.key == "concept:name")
                    .and_then(|a| if let crate::models::AttributeValue::String(s) = &a.value { Some(s.as_str()) } else { None })
                    .unwrap_or("No Activity");

                let index = *act_to_idx.entry(activity.to_string()).or_insert_with(|| {
                    let idx = activities.len();
                    activities.push(activity.to_string());
                    idx
                });
                trace_acts.push(index);
            }
            *traces_map.entry(trace_acts).or_insert(0) += 1;
        }

        Self {
            activities,
            traces: traces_map.into_iter().collect(),
        }
    }
}

pub fn token_replay_projected(log: &ProjectedLog, petri_net: &PetriNet) -> f64 {
    let num_places = petri_net.places.len();
    if num_places > 64 { return 0.0; }

    let mut place_to_idx = FxHashMap::default();
    for (i, p) in petri_net.places.iter().enumerate() { place_to_idx.insert(p.id.clone(), i); }

    let num_transitions = petri_net.transitions.len();
    let dummy_t_idx = num_transitions;

    let mut input_masks = vec![0u64; num_transitions + 1];
    let mut output_masks = vec![0u64; num_transitions + 1];

    for arc in &petri_net.arcs {
        let mut is_input = false;
        let t_idx_opt = if let Some(pos) = petri_net.transitions.iter().position(|t| t.id == arc.to) {
            is_input = true; Some(pos)
        } else {
            petri_net.transitions.iter().position(|t| t.id == arc.from)
        };

        if let Some(t_idx) = t_idx_opt {
            let p_id = if is_input { &arc.from } else { &arc.to };
            if let Some(&p_idx) = place_to_idx.get(p_id) {
                if is_input { input_masks[t_idx] |= 1u64 << p_idx; }
                else { output_masks[t_idx] |= 1u64 << p_idx; }
            }
        }
    }

    let mut input_counts = vec![0u32; num_transitions + 1];
    let mut output_counts = vec![0u32; num_transitions + 1];
    for i in 0..num_transitions {
        input_counts[i] = input_masks[i].count_ones();
        output_counts[i] = output_masks[i].count_ones();
    }

    let mut initial_mask = 0u64;
    for (p_id, c) in &petri_net.initial_marking {
        if *c > 0 { if let Some(&p_idx) = place_to_idx.get(p_id) { initial_mask |= 1u64 << p_idx; } }
    }

    let mut final_mask = 0u64;
    if let Some(fm) = petri_net.final_markings.first() {
        for (p_id, c) in fm { if *c > 0 { if let Some(&p_idx) = place_to_idx.get(p_id) { final_mask |= 1u64 << p_idx; } } }
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
            let t_idx = act_to_t_idx[act_idx];
            let in_mask = input_masks[t_idx];
            missing_tokens += (in_mask & !marking).count_ones();
            marking = (marking & !in_mask) | output_masks[t_idx];
            consumed_tokens += input_counts[t_idx];
            produced_tokens += output_counts[t_idx];
        }

        missing_tokens += (final_mask & !marking).count_ones();
        consumed_tokens += final_count;
        marking &= !final_mask;
        let remaining_tokens = marking.count_ones();

        let total_tokens_needed = consumed_tokens + missing_tokens;
        let fitness = if total_tokens_needed == 0 { 1.0 } 
            else { 1.0 - (missing_tokens as f64 + remaining_tokens as f64) / (total_tokens_needed as f64 + produced_tokens as f64) };

        total_fitness += fitness * (*freq as f64);
        total_freq += freq;
    }

    if total_freq == 0 { 1.0 } else { total_fitness / total_freq as f64 }
}

pub fn token_replay(log: &EventLog, petri_net: &PetriNet) -> Vec<ConformanceResult> {
    let num_places = petri_net.places.len();
    if num_places > 64 {
        // Fallback for > 64 places, but for our bounded Engine this shouldn't be hit.
        return log.traces.iter().map(|trace| replay_trace_standard(trace, petri_net)).collect();
    }

    let mut place_to_idx = FxHashMap::default();
    let mut act_to_t_idx = FxHashMap::default();

    for (i, p) in petri_net.places.iter().enumerate() { place_to_idx.insert(p.id.clone(), i); }
    for (i, t) in petri_net.transitions.iter().enumerate() { 
        act_to_t_idx.insert(t.label.clone(), i);
    }

    let num_transitions = petri_net.transitions.len();
    let dummy_t_idx = num_transitions;

    let mut input_masks = vec![0u64; num_transitions + 1];
    let mut output_masks = vec![0u64; num_transitions + 1];

    for arc in &petri_net.arcs {
        let mut is_input = false;
        let mut is_output = false;
        
        let t_idx_opt = if petri_net.transitions.iter().any(|t| t.id == arc.to) {
            is_input = true;
            petri_net.transitions.iter().position(|t| t.id == arc.to)
        } else if petri_net.transitions.iter().any(|t| t.id == arc.from) {
            is_output = true;
            petri_net.transitions.iter().position(|t| t.id == arc.from)
        } else {
            None
        };

        if let Some(t_idx) = t_idx_opt {
            let p_id = if is_input { &arc.from } else { &arc.to };
            if let Some(&p_idx) = place_to_idx.get(p_id) {
                if is_input {
                    input_masks[t_idx] |= 1u64 << p_idx;
                } else {
                    output_masks[t_idx] |= 1u64 << p_idx;
                }
            }
        }
    }

    let mut input_counts = vec![0u32; num_transitions + 1];
    let mut output_counts = vec![0u32; num_transitions + 1];
    for i in 0..num_transitions {
        input_counts[i] = input_masks[i].count_ones();
        output_counts[i] = output_masks[i].count_ones();
    }

    let mut initial_mask = 0u64;
    for (p_id, c) in &petri_net.initial_marking {
        if *c > 0 {
            if let Some(&p_idx) = place_to_idx.get(p_id) {
                initial_mask |= 1u64 << p_idx;
            }
        }
    }

    let mut final_mask = 0u64;
    if let Some(fm) = petri_net.final_markings.first() {
        for (p_id, c) in fm {
            if *c > 0 {
                if let Some(&p_idx) = place_to_idx.get(p_id) {
                    final_mask |= 1u64 << p_idx;
                }
            }
        }
    }
    let final_count = final_mask.count_ones();

    log.traces.iter().map(|trace| {
        let mut marking: u64 = initial_mask;
        let mut missing_tokens = 0;
        let mut consumed_tokens = 0;
        let mut produced_tokens = initial_mask.count_ones();

        for event in &trace.events {
            let mut t_idx = dummy_t_idx;
            if let Some(attr) = event.attributes.iter().find(|a| a.key == "concept:name") {
                if let crate::models::AttributeValue::String(s) = &attr.value {
                    if let Some(&idx) = act_to_t_idx.get(s) {
                        t_idx = idx;
                    }
                }
            }
            
            let in_mask = input_masks[t_idx];
            let missing = in_mask & !marking;
            missing_tokens += missing.count_ones();
            marking = (marking & !in_mask) | output_masks[t_idx];
            consumed_tokens += input_counts[t_idx];
            produced_tokens += output_counts[t_idx];
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
            1.0 - (missing_tokens as f64 + remaining_tokens as f64) / (total_tokens_needed as f64 + produced_tokens as f64)
        };

        ConformanceResult {
            case_id: trace.id.clone(),
            fitness,
            deviations: Vec::new(),
        }
    }).collect()
}

fn replay_trace_standard(trace: &Trace, petri_net: &PetriNet) -> ConformanceResult {
    let mut markings = petri_net.initial_marking.clone();
    let mut consumed_tokens = 0;
    let mut produced_tokens = 0;
    let mut missing_tokens = 0;

    for event in &trace.events {
        let activity = event.attributes.iter()
            .find(|a| a.key == "concept:name")
            .and_then(|a| if let crate::models::AttributeValue::String(s) = &a.value { Some(s) } else { None });

        if let Some(activity) = activity {
            if let Some(transition) = petri_net.transitions.iter().find(|t| &t.label == activity) {
                let input_arcs: Vec<&Arc> = petri_net.arcs.iter().filter(|a| a.to == transition.id).collect();
                let mut can_fire = true;
                for arc in &input_arcs {
                    let token_count = markings.get(&arc.from).unwrap_or(&0);
                    if *token_count < arc.weight.unwrap_or(1) {
                        can_fire = false;
                        missing_tokens += arc.weight.unwrap_or(1) - *token_count;
                    }
                }

                if can_fire {
                    for arc in &input_arcs {
                        let token_count = markings.get_mut(&arc.from).unwrap();
                        *token_count -= arc.weight.unwrap_or(1);
                        consumed_tokens += arc.weight.unwrap_or(1);
                    }
                    let output_arcs: Vec<&Arc> = petri_net.arcs.iter().filter(|a| a.from == transition.id).collect();
                    for arc in &output_arcs {
                        *markings.entry(arc.to.clone()).or_insert(0) += arc.weight.unwrap_or(1);
                        produced_tokens += arc.weight.unwrap_or(1);
                    }
                } else {
                    missing_tokens += 1;
                }
            }
        }
    }

    let remaining_tokens: usize = markings.values().sum();
    let total_tokens_needed = consumed_tokens + missing_tokens;
    let fitness = if total_tokens_needed == 0 {
        1.0
    } else {
        1.0 - (missing_tokens as f64 + remaining_tokens as f64) / (total_tokens_needed as f64 + produced_tokens as f64)
    };

    ConformanceResult {
        case_id: trace.id.clone(),
        fitness,
        deviations: Vec::new(),
    }
}
