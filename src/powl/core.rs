//! Hyper-optimized index-based Partially Ordered Workflow Language (POWL) implementation.
//! Ported and enhanced from PM4Py Python implementation for Digital Team Process Intelligence.
//! Incorporates Choice Graphs for Non-Block-Structured Decisions (van der Aalst, 2025).

use bcinr_core::dense_kernel::KBitSet;
use std::collections::HashSet;

pub const MAX_POWL_NODES: usize = 64;

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
    Transition {
        label: Option<String>,
        id: u64,
    },
    Operator {
        operator: PowlOperator,
        children: Vec<PowlNode>,
    },
    PartialOrder {
        nodes: Vec<PowlNode>,
        edges: Vec<(usize, usize)>,
    },
    ChoiceGraph {
        nodes: Vec<PowlNode>,
        edges: Vec<(usize, usize)>,
        start_nodes: Vec<usize>,
        end_nodes: Vec<usize>,
        empty_path: bool,
    },
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
            PowlNode::ChoiceGraph {
                nodes,
                edges,
                start_nodes,
                end_nodes,
                ..
            } => {
                for child in nodes {
                    child.validate_soundness()?;
                }
                if start_nodes.is_empty() {
                    return Err("ChoiceGraph must have start nodes".to_string());
                }
                if end_nodes.is_empty() {
                    return Err("ChoiceGraph must have end nodes".to_string());
                }

                // DFS acyclicity check
                let mut visited = HashSet::new();
                let mut rec_stack = HashSet::new();

                for node_idx in 0..nodes.len() {
                    if !visited.contains(&node_idx)
                        && Self::has_cycle(node_idx, edges, &mut visited, &mut rec_stack)
                    {
                        return Err("Graph contains cycles".to_string());
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn has_cycle(
        node: usize,
        edges: &[(usize, usize)],
        visited: &mut HashSet<usize>,
        rec_stack: &mut HashSet<usize>,
    ) -> bool {
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

pub struct PowlModel<const WORDS: usize> {
    pub root: PowlNode,
    // Flattened bitmask representation for high-speed replay
    pub partial_order_mask: Vec<KBitSet<WORDS>>,
    pub xor_exclusion_mask: Vec<KBitSet<WORDS>>,
    pub choice_routing_mask: Vec<KBitSet<WORDS>>,
    pub repetition_exclusion_mask: Vec<KBitSet<WORDS>>,
}

impl<const WORDS: usize> PowlModel<WORDS> {
    pub const MAX_NODES: usize = WORDS * 64;

    pub fn new(root: PowlNode) -> Self {
        root.validate_soundness()
            .expect("POWL Model must be structurally sound");
        let mut model = Self {
            root,
            partial_order_mask: vec![KBitSet::zero(); Self::MAX_NODES],
            xor_exclusion_mask: vec![KBitSet::zero(); Self::MAX_NODES],
            choice_routing_mask: vec![KBitSet::zero(); Self::MAX_NODES],
            repetition_exclusion_mask: vec![KBitSet::zero(); Self::MAX_NODES],
        };
        model.compile();
        model
    }

    pub fn dummy() -> Self {
        Self {
            root: PowlNode::dummy(),
            partial_order_mask: vec![KBitSet::zero(); Self::MAX_NODES],
            xor_exclusion_mask: vec![KBitSet::zero(); Self::MAX_NODES],
            choice_routing_mask: vec![KBitSet::zero(); Self::MAX_NODES],
            repetition_exclusion_mask: vec![KBitSet::zero(); Self::MAX_NODES],
        }
    }

    /// Compiles the recursive tree structure into flat bitmasks for O(1) validation.
    pub fn compile(&mut self) {
        for m in &mut self.partial_order_mask {
            m.clear();
        }
        for m in &mut self.xor_exclusion_mask {
            m.clear();
        }
        for m in &mut self.choice_routing_mask {
            m.clear();
        }
        for m in &mut self.repetition_exclusion_mask {
            m.clear();
        }

        let root_clone = self.root.clone();
        self.compile_node(&root_clone);

        // Transitive closure for partial order mask
        for k in 0..Self::MAX_NODES {
            for i in 0..Self::MAX_NODES {
                if self.partial_order_mask[i].contains(k) {
                    let k_mask = self.partial_order_mask[k];
                    self.partial_order_mask[i] = self.partial_order_mask[i].bitwise_or(k_mask);
                }
            }
        }
    }

    /// Recursively compiles a node, returning (entry_mask, exit_mask, footprint_mask)
    fn compile_node(
        &mut self,
        node: &PowlNode,
    ) -> (KBitSet<WORDS>, KBitSet<WORDS>, KBitSet<WORDS>) {
        match node {
            PowlNode::Transition { id, .. } => {
                let mut mask = KBitSet::zero();
                let _ = mask.set(*id as usize);
                self.repetition_exclusion_mask[*id as usize] = mask;
                (mask, mask, mask)
            }
            PowlNode::Operator { operator, children } => {
                if children.is_empty() {
                    return (KBitSet::zero(), KBitSet::zero(), KBitSet::zero());
                }

                let mut child_results = Vec::new();
                for child in children {
                    child_results.push(self.compile_node(child));
                }

                let mut total_footprint = KBitSet::zero();
                for res in &child_results {
                    total_footprint = total_footprint.bitwise_or(res.2);
                }

                match operator {
                    PowlOperator::SEQUENCE => {
                        let entry = child_results[0].0;
                        let mut exit = child_results[0].1;

                        for child_result in child_results.iter().skip(1) {
                            let next_exit = child_result.1;
                            let next_footprint = child_result.2;

                            // All nodes in the next subtree depend on the exit nodes of the previous subtree
                            for target_bit in 0..Self::MAX_NODES {
                                if next_footprint.contains(target_bit) {
                                    self.partial_order_mask[target_bit] =
                                        self.partial_order_mask[target_bit].bitwise_or(exit);
                                }
                            }

                            if !next_exit.is_empty() {
                                exit = next_exit;
                            }
                        }
                        (entry, exit, total_footprint)
                    }
                    PowlOperator::PARALLEL | PowlOperator::AND => {
                        let mut entry = KBitSet::zero();
                        let mut exit = KBitSet::zero();
                        for res in &child_results {
                            entry = entry.bitwise_or(res.0);
                            exit = exit.bitwise_or(res.1);
                        }
                        (entry, exit, total_footprint)
                    }
                    PowlOperator::XOR => {
                        let mut entry = KBitSet::zero();
                        for i in 0..child_results.len() {
                            entry = entry.bitwise_or(child_results[i].0);

                            for j in (i + 1)..child_results.len() {
                                let footprint_i = child_results[i].2;
                                let footprint_j = child_results[j].2;

                                for bit_i in 0..Self::MAX_NODES {
                                    if footprint_i.contains(bit_i) {
                                        self.xor_exclusion_mask[bit_i] =
                                            self.xor_exclusion_mask[bit_i].bitwise_or(footprint_j);
                                    }
                                }
                                for bit_j in 0..Self::MAX_NODES {
                                    if footprint_j.contains(bit_j) {
                                        self.xor_exclusion_mask[bit_j] =
                                            self.xor_exclusion_mask[bit_j].bitwise_or(footprint_i);
                                    }
                                }
                            }
                        }
                        (entry, KBitSet::zero(), total_footprint)
                    }
                    PowlOperator::LOOP => {
                        for bit in 0..Self::MAX_NODES {
                            if total_footprint.contains(bit) {
                                self.repetition_exclusion_mask[bit].clear();
                            }
                        }
                        if child_results.len() >= 2 {
                            let do_entry = child_results[0].0;
                            let do_exit = child_results[0].1;
                            let redo_entry = child_results[1].0;
                            let redo_exit = child_results[1].1;

                            for bit in 0..Self::MAX_NODES {
                                if child_results[1].2.contains(bit) {
                                    self.partial_order_mask[bit] =
                                        self.partial_order_mask[bit].bitwise_or(do_exit);
                                }
                            }

                            for src_bit in 0..Self::MAX_NODES {
                                if do_exit.contains(src_bit) {
                                    self.choice_routing_mask[src_bit] =
                                        self.choice_routing_mask[src_bit].bitwise_or(redo_entry);
                                }
                                if redo_exit.contains(src_bit) {
                                    self.choice_routing_mask[src_bit] =
                                        self.choice_routing_mask[src_bit].bitwise_or(do_entry);
                                }
                            }
                        } else if child_results.len() == 1 {
                            let do_entry = child_results[0].0;
                            let do_exit = child_results[0].1;
                            for src_bit in 0..Self::MAX_NODES {
                                if do_exit.contains(src_bit) {
                                    self.choice_routing_mask[src_bit] =
                                        self.choice_routing_mask[src_bit].bitwise_or(do_entry);
                                }
                            }
                        }
                        (child_results[0].0, child_results[0].1, total_footprint)
                    }
                    _ => (KBitSet::zero(), KBitSet::zero(), KBitSet::zero()),
                }
            }
            PowlNode::PartialOrder { nodes, edges } => {
                let mut child_results = Vec::new();
                let mut local_footprint = KBitSet::zero();
                for n in nodes {
                    let res = self.compile_node(n);
                    child_results.push(res);
                    local_footprint = local_footprint.bitwise_or(res.2);
                }

                for &(src_idx, tgt_idx) in edges {
                    if src_idx < child_results.len() && tgt_idx < child_results.len() {
                        let src_exit = child_results[src_idx].1;
                        let tgt_entry = child_results[tgt_idx].0;
                        let tgt_footprint = child_results[tgt_idx].2;

                        for bit in 0..Self::MAX_NODES {
                            if tgt_footprint.contains(bit) {
                                self.partial_order_mask[bit] =
                                    self.partial_order_mask[bit].bitwise_or(src_exit);
                            }
                        }
                        for src_bit in 0..Self::MAX_NODES {
                            if src_exit.contains(src_bit) {
                                self.choice_routing_mask[src_bit] =
                                    self.choice_routing_mask[src_bit].bitwise_or(tgt_entry);
                            }
                        }
                    }
                }

                let mut is_target = vec![false; nodes.len()];
                for &(_, tgt) in edges {
                    is_target[tgt] = true;
                }

                let mut entry = KBitSet::zero();
                let mut exit = KBitSet::zero();
                for i in 0..nodes.len() {
                    if !is_target[i] {
                        entry = entry.bitwise_or(child_results[i].0);
                    }
                    exit = exit.bitwise_or(child_results[i].1);
                }

                (entry, exit, local_footprint)
            }
            PowlNode::ChoiceGraph {
                nodes,
                edges,
                start_nodes,
                end_nodes,
                empty_path: _,
            } => {
                let mut child_results = Vec::new();
                let mut local_footprint = KBitSet::zero();
                for n in nodes {
                    let res = self.compile_node(n);
                    child_results.push(res);
                    local_footprint = local_footprint.bitwise_or(res.2);
                }

                for &(src_idx, tgt_idx) in edges {
                    if src_idx < child_results.len() && tgt_idx < child_results.len() {
                        let src_exit = child_results[src_idx].1;
                        let tgt_entry = child_results[tgt_idx].0;

                        for src_bit in 0..Self::MAX_NODES {
                            if src_exit.contains(src_bit) {
                                self.choice_routing_mask[src_bit] =
                                    self.choice_routing_mask[src_bit].bitwise_or(tgt_entry);
                            }
                        }
                    }
                }

                let mut entry = KBitSet::zero();
                for &idx in start_nodes {
                    if idx < child_results.len() {
                        entry = entry.bitwise_or(child_results[idx].0);
                    }
                }

                let mut exit = KBitSet::zero();
                for &idx in end_nodes {
                    if idx < child_results.len() {
                        exit = exit.bitwise_or(child_results[idx].1);
                    }
                }

                (entry, exit, local_footprint)
            }
        }
    }

    #[inline(always)]
    pub fn is_transition_valid(
        &self,
        idx: usize,
        executed_mask: KBitSet<WORDS>,
        prev_idx: usize,
    ) -> bool {
        if idx >= Self::MAX_NODES {
            return false;
        }

        let required_prereqs = self.partial_order_mask[idx];
        let exclusions = self.xor_exclusion_mask[idx];
        let repeat_excl = self.repetition_exclusion_mask[idx].contains(idx);

        // 1. Precedence Check
        if !executed_mask.contains_all(required_prereqs) {
            return false;
        }

        // 2. XOR Routing Check
        if !executed_mask.bitwise_and(exclusions).is_empty() {
            return false;
        }

        // 3. Repetition Check
        if repeat_excl && executed_mask.contains(idx) {
            return false;
        }

        // 4. Choice Graph Continuous Routing Check
        if prev_idx < Self::MAX_NODES {
            let allowed_next = self.choice_routing_mask[prev_idx];
            if !allowed_next.is_empty() && !allowed_next.contains(idx) {
                return false;
            }
        }

        true
    }

    #[inline(always)]
    pub fn is_trace_valid(&self, trace_indices: &[u8]) -> bool {
        if trace_indices.is_empty() {
            return true;
        }

        let mut executed_mask = KBitSet::zero();
        let mut prev_idx = Self::MAX_NODES;

        for &act_idx in trace_indices {
            let idx = act_idx as usize;
            if !self.is_transition_valid(idx, executed_mask, prev_idx) {
                return false;
            }

            let _ = executed_mask.set(idx);
            prev_idx = idx;
        }
        true
    }

    pub fn transitive_closure(adj: &mut [u64; MAX_POWL_NODES]) {
        for k in 0..MAX_POWL_NODES {
            for i in 0..MAX_POWL_NODES {
                let mask = 0u64.wrapping_sub((adj[i] >> k) & 1);
                adj[i] |= mask & adj[k];
            }
        }
    }

    pub fn transitive_reduction(adj: &mut [u64; MAX_POWL_NODES]) {
        let mut reduction = *adj;
        for i in 0..MAX_POWL_NODES {
            for j in 0..MAX_POWL_NODES {
                for k in 0..MAX_POWL_NODES {
                    if i != j && j != k && (adj[i] & (1 << j)) != 0 && (adj[j] & (1 << k)) != 0 {
                        reduction[i] &= !(1 << k);
                    }
                }
            }
        }
        *adj = reduction;
    }
}
