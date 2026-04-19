//! Hyper-optimized index-based Partially Ordered Workflow Language (POWL) implementation.
//! Ported and enhanced from PM4Py Python implementation for Digital Team Process Intelligence.
//! Incorporates Choice Graphs for Non-Block-Structured Decisions (van der Aalst, 2025).

use crate::utils::dense_kernel::fnv1a_64;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowlOperator {
    XOR,
    AND,
    LOOP,
    SEQUENCE,
    PARALLEL,
    PARTIALORDER,
    CHOICEGRAPH,
}

#[derive(Debug, Clone)]
pub enum PowlNode {
    Transition { label: Option<String>, id: u64 },
    Operator { operator: PowlOperator, children: Vec<PowlNode> },
    PartialOrder { nodes: Vec<PowlNode>, edges: Vec<(usize, usize)> },
    ChoiceGraph { nodes: Vec<PowlNode>, edges: Vec<(usize, usize)>, start_nodes: Vec<usize>, end_nodes: Vec<usize>, empty_path: bool },
}

pub struct PowlModel {
    pub root: PowlNode,
    // Flattened bitmask representation for high-speed replay
    pub partial_order_mask: Vec<u64>, 
    pub xor_exclusion_mask: Vec<u64>, 
    pub choice_routing_mask: Vec<u64>,
    pub repetition_exclusion_mask: Vec<u64>,
}

impl PowlNode {
    pub fn dummy() -> Self {
        PowlNode::Transition { label: None, id: 0 }
    }

    pub fn validate_soundness(&self) -> Result<(), String> {
        match self {
            PowlNode::Operator { children, .. } => {
                for child in children {
                    child.validate_soundness()?;
                }
            }
            PowlNode::PartialOrder { nodes, .. } => {
                for child in nodes {
                    child.validate_soundness()?;
                }
            }
            PowlNode::ChoiceGraph { nodes, edges, start_nodes, end_nodes, .. } => {
                for child in nodes {
                    child.validate_soundness()?;
                }
                if start_nodes.is_empty() { return Err("ChoiceGraph must have start nodes".to_string()); }
                if end_nodes.is_empty() { return Err("ChoiceGraph must have end nodes".to_string()); }

                // DFS acyclicity check
                let mut visited = HashSet::new();
                let mut rec_stack = HashSet::new();
                
                for node_idx in 0..nodes.len() {
                    if !visited.contains(&node_idx) {
                        if Self::has_cycle(node_idx, edges, &mut visited, &mut rec_stack) {
                            return Err("Graph contains cycles".to_string());
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn has_cycle(node: usize, edges: &[(usize, usize)], visited: &mut HashSet<usize>, rec_stack: &mut HashSet<usize>) -> bool {
        visited.insert(node);
        rec_stack.insert(node);
        
        for &(src, tgt) in edges {
            if src == node {
                if !visited.contains(&tgt) {
                    if Self::has_cycle(tgt, edges, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(&tgt) {
                    return true;
                }
            }
        }
        
        rec_stack.remove(&node);
        false
    }
}

impl PowlModel {
    pub fn new(root: PowlNode) -> Self {
        root.validate_soundness().expect("POWL Model must be structurally sound");
        let mut model = Self {
            root,
            partial_order_mask: vec![0; 64],
            xor_exclusion_mask: vec![0; 64],
            choice_routing_mask: vec![0; 64],
            repetition_exclusion_mask: vec![0; 64],
        };
        model.compile();
        model
    }

    pub fn dummy() -> Self {
        Self {
            root: PowlNode::dummy(),
            partial_order_mask: vec![0; 64],
            xor_exclusion_mask: vec![0; 64],
            choice_routing_mask: vec![0; 64],
            repetition_exclusion_mask: vec![0; 64],
        }
    }

    /// Compiles the recursive tree structure into flat bitmasks for O(1) validation.
    pub fn compile(&mut self) {
        self.partial_order_mask.fill(0);
        self.xor_exclusion_mask.fill(0);
        self.choice_routing_mask.fill(0);
        self.repetition_exclusion_mask.fill(0);
        
        let root_clone = self.root.clone();
        self.compile_node(&root_clone);
        
        // Compute transitive closure of the partial order
        let mut adj = [0u64; 64];
        for i in 0..64 {
            adj[i] = self.partial_order_mask[i];
        }
        Self::transitive_closure(&mut adj);
        for i in 0..64 {
            self.partial_order_mask[i] = adj[i];
        }
    }

    /// Recursively compiles a node, returning (entry_mask, exit_mask, footprint_mask)
    fn compile_node(&mut self, node: &PowlNode) -> (u64, u64, u64) {
        match node {
            PowlNode::Transition { id, .. } => {
                let mask = 1u64 << *id;
                self.repetition_exclusion_mask[*id as usize] |= mask;
                (mask, mask, mask)
            },
            PowlNode::Operator { operator, children } => {
                if children.is_empty() { return (0, 0, 0); }
                
                let mut child_results = Vec::new();
                for child in children {
                    child_results.push(self.compile_node(child));
                }

                let mut total_footprint = 0u64;
                for res in &child_results { total_footprint |= res.2; }

                match operator {
                    PowlOperator::SEQUENCE => {
                        let entry = child_results[0].0;
                        let mut exit = child_results[0].1;
                        
                        for i in 1..child_results.len() {
                            let next_entry = child_results[i].0;
                            let next_exit = child_results[i].1;
                            let next_footprint = child_results[i].2;
                            
                            // All nodes in the next subtree depend on the exit nodes of the previous subtree
                            for target_bit in 0..64 {
                                if (next_footprint & (1 << target_bit)) != 0 {
                                    self.partial_order_mask[target_bit] |= exit;
                                }
                            }

                            // Keep previous exit if current exit is 0 (e.g., from XOR) to maintain partial order chain
                            if next_exit != 0 {
                                exit = next_exit;
                            }
                        }
                        (entry, exit, total_footprint)
                    },
                    PowlOperator::PARALLEL | PowlOperator::AND => {
                        let mut entry = 0u64;
                        let mut exit = 0u64;
                        for res in &child_results {
                            entry |= res.0;
                            exit |= res.1;
                        }
                        (entry, exit, total_footprint)
                    },
                    PowlOperator::XOR => {
                        let mut entry = 0u64;
                        let mut exit = 0u64;
                        for i in 0..child_results.len() {
                            entry |= child_results[i].0;
                            exit |= child_results[i].1;
                            
                            for j in (i + 1)..child_results.len() {
                                let footprint_i = child_results[i].2;
                                let footprint_j = child_results[j].2;
                                
                                for bit_i in 0..64 {
                                    if (footprint_i & (1 << bit_i)) != 0 {
                                        self.xor_exclusion_mask[bit_i] |= footprint_j;
                                    }
                                }
                                for bit_j in 0..64 {
                                    if (footprint_j & (1 << bit_j)) != 0 {
                                        self.xor_exclusion_mask[bit_j] |= footprint_i;
                                    }
                                }
                            }
                        }
                        // Returning 0 for exit prevents AND-logic over-constraining in partial order
                        (entry, 0, total_footprint)
                    },
                    PowlOperator::LOOP => {
                        // Unset repetition constraint for repeatable nodes
                        for bit in 0..64 {
                            if (total_footprint & (1 << bit)) != 0 {
                                self.repetition_exclusion_mask[bit] &= !(1 << bit);
                            }
                        }
                        if child_results.len() >= 2 {
                            let do_entry = child_results[0].0;
                            let do_exit = child_results[0].1;
                            let redo_entry = child_results[1].0;
                            let redo_exit = child_results[1].1;
                            let redo_footprint = child_results[1].2;
                            
                            for bit in 0..64 {
                                if (redo_footprint & (1 << bit)) != 0 {
                                    self.partial_order_mask[bit] |= do_exit;
                                }
                            }
                            
                            // Forward routing: Do -> Redo
                            for src_bit in 0..64 {
                                if (do_exit & (1 << src_bit)) != 0 {
                                    self.choice_routing_mask[src_bit] |= redo_entry;
                                }
                            }
                            // Back-edge routing: Redo -> Do
                            for src_bit in 0..64 {
                                if (redo_exit & (1 << src_bit)) != 0 {
                                    self.choice_routing_mask[src_bit] |= do_entry;
                                }
                            }
                        } else if child_results.len() == 1 {
                            // Self-loop routing
                            let do_entry = child_results[0].0;
                            let do_exit = child_results[0].1;
                            for src_bit in 0..64 {
                                if (do_exit & (1 << src_bit)) != 0 {
                                    self.choice_routing_mask[src_bit] |= do_entry;
                                }
                            }
                        }
                        (child_results[0].0, child_results[0].1, total_footprint)
                    },
                    _ => (0, 0, 0)
                }
            },
            PowlNode::PartialOrder { nodes, edges } => {
                let mut child_results = Vec::new();
                let mut local_footprint = 0u64;
                for n in nodes { 
                    let res = self.compile_node(n);
                    child_results.push(res);
                    local_footprint |= res.2;
                }
                
                for &(src_idx, tgt_idx) in edges {
                    if src_idx < child_results.len() && tgt_idx < child_results.len() {
                        let src_exit = child_results[src_idx].1;
                        let tgt_entry = child_results[tgt_idx].0;
                        let tgt_footprint = child_results[tgt_idx].2;
                        
                        for bit in 0..64 {
                            if (tgt_footprint & (1 << bit)) != 0 {
                                self.partial_order_mask[bit] |= src_exit;
                            }
                        }
                        // Add strict routing for immediate edges
                        for src_bit in 0..64 {
                            if (src_exit & (1 << src_bit)) != 0 {
                                self.choice_routing_mask[src_bit] |= tgt_entry;
                            }
                        }
                    }
                }
                
                let mut is_target = vec![false; nodes.len()];
                for &(_, tgt) in edges { is_target[tgt] = true; }
                
                let mut entry = 0u64;
                let mut exit = 0u64; 
                for i in 0..nodes.len() {
                    if !is_target[i] { entry |= child_results[i].0; }
                    exit |= child_results[i].1;
                }
                
                (entry, exit, local_footprint)
            },
            PowlNode::ChoiceGraph { nodes, edges, start_nodes, end_nodes, empty_path } => {
                let mut child_results = Vec::new();
                let mut local_footprint = 0u64;
                for n in nodes { 
                    let res = self.compile_node(n);
                    child_results.push(res);
                    local_footprint |= res.2;
                }
                
                // Populate internal routing edges
                for &(src_idx, tgt_idx) in edges {
                    if src_idx < child_results.len() && tgt_idx < child_results.len() {
                        let src_exit = child_results[src_idx].1;
                        let tgt_entry = child_results[tgt_idx].0;
                        
                        for src_bit in 0..64 {
                            if (src_exit & (1 << src_bit)) != 0 {
                                self.choice_routing_mask[src_bit] |= tgt_entry;
                            }
                        }
                    }
                }
                
                let mut entry = 0u64;
                for &idx in start_nodes {
                    if idx < child_results.len() { entry |= child_results[idx].0; }
                }
                
                let mut exit = 0u64;
                for &idx in end_nodes {
                    if idx < child_results.len() { exit |= child_results[idx].1; }
                }
                
                // If empty path is allowed, the entry block conceptually bypasses everything.
                // In bitset calculus without sentinel nodes, this requires relaxing the required entry footprint.
                // We leave empty_path out of the bitmask approximation for now.
                
                (entry, exit, local_footprint)
            }

        }
    }

    #[inline(always)]
    pub fn is_trace_valid(&self, trace_indices: &[u8]) -> bool {
        if trace_indices.is_empty() { return true; }
        
        let mut executed_mask = 0u64;
        let mut prev_idx = 64; 
        
        for &act_idx in trace_indices {
            let idx = act_idx as usize;
            if idx >= 64 { return false; } 
            
            let required_prereqs = self.partial_order_mask[idx];
            let exclusions = self.xor_exclusion_mask[idx];
            let repeat_excl = (self.repetition_exclusion_mask[idx] >> idx) & 1;
            
            // 1. Precedence Check: (Required & !Executed) == 0
            if (required_prereqs & !executed_mask) != 0 {
                return false;
            }
            
            // 2. XOR Routing Check: (Exclusions & Executed) == 0
            if (exclusions & executed_mask) != 0 {
                return false;
            }

            // 3. Repetition Check: (repeat_excl & already_executed) == 0
            if repeat_excl != 0 && (executed_mask & (1 << idx)) != 0 {
                return false;
            }
            
            // 4. Choice Graph Continuous Routing Check
            if prev_idx < 64 {
                let allowed_next = self.choice_routing_mask[prev_idx];
                if allowed_next != 0 && (allowed_next & (1 << idx)) == 0 {
                    return false;
                }
            }
            
            executed_mask |= 1 << idx;
            prev_idx = idx;
        }
        true
    }

    pub fn transitive_closure(adj: &mut [u64; 64]) {
        for k in 0..64 {
            for i in 0..64 {
                let mask = 0u64.wrapping_sub((adj[i] >> k) & 1);
                adj[i] |= mask & adj[k];
            }
        }
    }

    pub fn transitive_reduction(adj: &mut [u64; 64]) {
        let mut reduction = *adj;
        for i in 0..64 {
            for j in 0..64 {
                for k in 0..64 {
                    if i != j && j != k && (adj[i] & (1 << j)) != 0 && (adj[j] & (1 << k)) != 0 {
                        reduction[i] &= !(1 << k);
                    }
                }
            }
        }
        *adj = reduction;
    }
}

