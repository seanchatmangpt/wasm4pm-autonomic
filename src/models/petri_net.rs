use serde::{Deserialize, Serialize};
use rustc_hash::FxHashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Place {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transition {
    pub id: String,
    pub label: String,
    pub is_invisible: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Arc {
    pub from: String,
    pub to: String,
    pub weight: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PetriNet {
    pub places: Vec<Place>,
    pub transitions: Vec<Transition>,
    pub arcs: Vec<Arc>,
    pub initial_marking: FxHashMap<String, usize>,
    pub final_markings: Vec<FxHashMap<String, usize>>,
}

impl PetriNet {
    /// Builds a temporary node-to-index mapping using the faster FxHasher.
    /// This is now only used for cold paths.
    fn build_node_index(&self) -> FxHashMap<&str, usize> {
        let mut map = FxHashMap::with_capacity_and_hasher(
            self.places.len() + self.transitions.len(), 
            Default::default()
        );
        for (i, p) in self.places.iter().enumerate() { map.insert(p.id.as_str(), i); }
        let offset = self.places.len();
        for (i, t) in self.transitions.iter().enumerate() { map.insert(t.id.as_str(), offset + i); }
        map
    }

    /// Evaluates if the net is a structurally valid workflow net.
    /// Highly optimized with pre-calculated indices and bitset algebra.
    pub fn is_structural_workflow_net(&self) -> bool {
        if self.places.is_empty() || self.transitions.is_empty() { return false; }
        
        let id_to_index = self.build_node_index();
        let place_count = self.places.len();
        let total_nodes = place_count + self.transitions.len();
        let num_words = (total_nodes + 63) / 64;
        
        let mut in_degrees = vec![0u64; num_words];
        let mut out_degrees = vec![0u64; num_words];
        
        for arc in &self.arcs {
            if let (Some(&from_idx), Some(&to_idx)) = (id_to_index.get(arc.from.as_str()), id_to_index.get(arc.to.as_str())) {
                out_degrees[from_idx / 64] |= 1u64 << (from_idx % 64);
                in_degrees[to_idx / 64] |= 1u64 << (to_idx % 64);
            }
        }
        
        let mut source_places_count = 0;
        let mut sink_places_count = 0;
        
        for i in 0..place_count {
            let has_in = (in_degrees[i / 64] & (1u64 << (i % 64))) != 0;
            let has_out = (out_degrees[i / 64] & (1u64 << (i % 64))) != 0;
            if !has_in { source_places_count += 1; }
            if !has_out { sink_places_count += 1; }
        }
        
        if source_places_count != 1 || sink_places_count != 1 { return false; }
        
        for i in place_count..total_nodes {
            let has_in = (in_degrees[i / 64] & (1u64 << (i % 64))) != 0;
            let has_out = (out_degrees[i / 64] & (1u64 << (i % 64))) != 0;
            if !has_in || !has_out { return false; }
        }
        
        true
    }

    /// Generates the Incidence Matrix (W).
    pub fn incidence_matrix(&self) -> Vec<Vec<i32>> {
        let mut matrix = vec![vec![0; self.transitions.len()]; self.places.len()];
        let id_to_index = self.build_node_index();
        let place_count = self.places.len();
        
        for arc in &self.arcs {
            let weight = arc.weight.unwrap_or(1) as i32;
            if let (Some(&from_idx), Some(&to_idx)) = (id_to_index.get(arc.from.as_str()), id_to_index.get(arc.to.as_str())) {
                if from_idx < place_count && to_idx >= place_count {
                    matrix[from_idx][to_idx - place_count] -= weight;
                } else if from_idx >= place_count && to_idx < place_count {
                    matrix[to_idx][from_idx - place_count] += weight;
                }
            }
        }
        matrix
    }

    /// Verifies the structural bounds of the workflow net state equation.
    pub fn verifies_state_equation_calculus(&self) -> bool {
        if !self.is_structural_workflow_net() { return false; }
        let w = self.incidence_matrix();
        for t_idx in 0..self.transitions.len() {
            let mut consumes = false;
            let mut produces = false;
            for p_idx in 0..self.places.len() {
                if w[p_idx][t_idx] < 0 { consumes = true; }
                if w[p_idx][t_idx] > 0 { produces = true; }
            }
            if !consumes || !produces { return false; }
        }
        true
    }

    /// Computes a smooth unsoundness score using bitset algebra and FxHash.
    pub fn structural_unsoundness_score(&self) -> f32 {
        if self.places.is_empty() || self.transitions.is_empty() { return 10.0; }
        
        let id_to_index = self.build_node_index();
        let place_count = self.places.len();
        let total_nodes = place_count + self.transitions.len();
        let num_words = (total_nodes + 63) / 64;
        
        let mut in_degrees = vec![0u64; num_words];
        let mut out_degrees = vec![0u64; num_words];
        
        for arc in &self.arcs {
            if let (Some(&from_idx), Some(&to_idx)) = (id_to_index.get(arc.from.as_str()), id_to_index.get(arc.to.as_str())) {
                out_degrees[from_idx / 64] |= 1u64 << (from_idx % 64);
                in_degrees[to_idx / 64] |= 1u64 << (to_idx % 64);
            }
        }
        
        let mut score = 0.0;
        let mut source_places_count = 0;
        let mut sink_places_count = 0;
        for i in 0..place_count {
            let has_in = (in_degrees[i / 64] & (1u64 << (i % 64))) != 0;
            let has_out = (out_degrees[i / 64] & (1u64 << (i % 64))) != 0;
            if !has_in { source_places_count += 1; }
            if !has_out { sink_places_count += 1; }
        }
        
        score += (source_places_count as f32 - 1.0).abs();
        score += (sink_places_count as f32 - 1.0).abs();
        
        for i in place_count..total_nodes {
            let has_in = (in_degrees[i / 64] & (1u64 << (i % 64))) != 0;
            let has_out = (out_degrees[i / 64] & (1u64 << (i % 64))) != 0;
            if !has_in { score += 1.0; }
            if !has_out { score += 1.0; }
        }
        
        for i in 0..place_count {
            let has_in = (in_degrees[i / 64] & (1u64 << (i % 64))) != 0;
            let has_out = (out_degrees[i / 64] & (1u64 << (i % 64))) != 0;
            if !has_in && !has_out { score += 2.0; } 
        }
        
        score
    }

    /// Optimized to use direct ID hashing instead of expensive string formatting.
    pub fn canonical_hash(&self) -> u64 {
        let mut hasher = rustc_hash::FxHasher::default();
        let mut p_ids: Vec<_> = self.places.iter().map(|p| &p.id).collect();
        p_ids.sort();
        for id in p_ids { id.hash(&mut hasher); }
        
        let mut t_ids: Vec<_> = self.transitions.iter().map(|t| &t.id).collect();
        t_ids.sort();
        for id in t_ids { id.hash(&mut hasher); }
        
        let mut arcs = self.arcs.clone();
        arcs.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
        for arc in arcs {
            arc.from.hash(&mut hasher);
            arc.to.hash(&mut hasher);
            arc.weight.unwrap_or(1).hash(&mut hasher);
        }
        
        hasher.finish()
    }
}
