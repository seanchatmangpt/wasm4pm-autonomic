//! POWL8 kinetic partial-order workflow primitive.
#![allow(
    clippy::large_enum_variant,
    clippy::needless_range_loop,
    clippy::collapsible_match
)]
//!
//! Minimal subset adapted from `unibit-powl`. Supports up to [`MAX_NODES`]
//! nodes with a [`BinaryRelation`] bit-matrix expressing partial orders.
//!
//! The kinetic dialect schedules cognitive breed invocations (Activity nodes
//! reference [`crate::verdict::Breed`]) over a partial order. Plans are
//! validated by [`Powl8::shape_match`]: bounds, child indices, and acyclicity
//! (Kahn's algorithm on PartialOrder submatrices, plus DFS over
//! `OperatorSequence` dependencies).

use crate::verdict::{Breed, PlanAdmission};

/// Maximum number of nodes in a POWL8 plan, bounding `BinaryRelation` to a
/// 64×64 bit-matrix.
pub const MAX_NODES: usize = 64;

/// Bit-packed adjacency matrix for partial orders over up to [`MAX_NODES`]
/// nodes.
///
/// `words[src]` holds a bit at position `tgt` iff edge `src → tgt` is present.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BinaryRelation {
    /// Per-source row of target bits: `words[src] & (1 << tgt) != 0` iff
    /// edge `src → tgt` exists.
    words: [u64; MAX_NODES],
}

impl BinaryRelation {
    /// Create an empty relation with no edges.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            words: [0u64; MAX_NODES],
        }
    }

    /// Add an edge from `src` to `tgt`. Out-of-bounds indices are ignored.
    pub fn add_edge(&mut self, src: usize, tgt: usize) {
        if src < MAX_NODES && tgt < MAX_NODES {
            self.words[src] |= 1u64 << tgt;
        }
    }

    /// Checked variant of [`Self::add_edge`]. Returns
    /// [`BinaryRelationError::OutOfBounds`] if either index exceeds
    /// [`MAX_NODES`].
    pub fn try_add_edge(&mut self, src: usize, tgt: usize) -> Result<(), BinaryRelationError> {
        if src >= MAX_NODES || tgt >= MAX_NODES {
            return Err(BinaryRelationError::OutOfBounds);
        }
        self.words[src] |= 1u64 << tgt;
        Ok(())
    }

    /// Return `true` iff an edge from `src` to `tgt` is present.
    #[must_use]
    pub const fn is_edge(&self, src: usize, tgt: usize) -> bool {
        if src < MAX_NODES && tgt < MAX_NODES {
            (self.words[src] >> tgt) & 1 == 1
        } else {
            false
        }
    }
}

/// Error type for [`BinaryRelation`] checked construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryRelationError {
    /// Source or target index exceeds [`MAX_NODES`].
    OutOfBounds,
}

impl Default for BinaryRelation {
    fn default() -> Self {
        Self::new()
    }
}

/// Plan node: either a marker, an Activity invoking a [`Breed`], a sub-plan
/// with its own partial order, or a binary operator over child indices.
#[derive(Clone, Copy, Debug)]
pub enum Powl8Node {
    /// Silent runtime marker. Carries no breed; advanced unconditionally when its
    /// predecessors are advanced. Used as a synchronization point in partial-order
    /// plans (e.g., to fan-in independent activities before a successor).
    Silent,
    /// Single breed invocation.
    Activity(Breed),
    /// Sub-plan with explicit partial order over `count` children located at
    /// indices `start..start + count` in the parent plan's node array.
    PartialOrder {
        /// Index of the first child node in the plan.
        start: u16,
        /// Number of consecutive child nodes participating in the partial order.
        count: u16,
        /// Edges of the partial order, indexed locally from 0..count.
        rel: BinaryRelation,
    },
    /// Children execute in sequence — `b` follows `a`.
    OperatorSequence {
        /// Index of the predecessor child node.
        a: u16,
        /// Index of the successor child node.
        b: u16,
    },
    /// Children execute in parallel — `a` and `b` are independent.
    OperatorParallel {
        /// Index of the first parallel child.
        a: u16,
        /// Index of the second parallel child.
        b: u16,
    },
    /// Plan entry marker; treated as advanced from the outset.
    StartNode,
    /// Plan exit marker; reachable only after all predecessors are advanced.
    EndNode,
    /// Exactly one of up to four branch nodes runs at runtime. Branch indices
    /// are global into `Powl8.nodes`; only `branches[..len as usize]` are
    /// considered. Each branch sees the Choice node itself as its sole
    /// declared predecessor (per [`Powl8::predecessor_masks`]).
    ///
    /// `Powl8Node` stays `Copy` because the branches array is a fixed
    /// `[u16; 4]` — no heap-allocated branch list.
    Choice {
        /// Up to four branch node indices (global into `Powl8.nodes`). Only
        /// the first `len` entries are read; trailing entries are ignored.
        branches: [u16; 4],
        /// Live branch count; must satisfy `1..=4`. `0` is malformed.
        len: u8,
    },
    /// Bounded loop: at compile time, `max_iters` ≤ 16 sequential copies of
    /// `body` are unrolled into the runtime plan. The combined executable
    /// node count must remain ≤ 64 or compile yields `Malformed`. `body`
    /// must be a different node index (no self-body).
    ///
    /// `Powl8Node` stays `Copy` because the only fields are `u16` + `u8`.
    Loop {
        /// Index of the body node (must differ from the Loop node itself).
        body: u16,
        /// Maximum unroll count; clamped at 16 by `shape_match`.
        max_iters: u8,
    },
}

/// Kinetic partial-order workflow plan, bounded to [`MAX_NODES`] nodes.
#[derive(Clone, Debug)]
pub struct Powl8 {
    /// Plan node array; `nodes.len() <= MAX_NODES` is enforced by [`Powl8::push`].
    pub nodes: Vec<Powl8Node>,
    /// Index of the root entry point in `nodes`.
    pub root: u16,
}

impl Powl8 {
    /// Create an empty plan with `root = 0`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: 0,
        }
    }

    /// Append a node and return its index. Returns
    /// [`PlanAdmission::Malformed`] if the plan is at capacity.
    pub fn push(&mut self, node: Powl8Node) -> Result<u16, PlanAdmission> {
        if self.nodes.len() >= MAX_NODES {
            return Err(PlanAdmission::Malformed);
        }
        let idx = self.nodes.len() as u16;
        self.nodes.push(node);
        Ok(idx)
    }

    /// Validate plan structure: bounds, valid breeds, acyclic relations.
    ///
    /// Returns `Ok(())` if Sound, `Err(PlanAdmission::Cyclic)` on detected
    /// cycle, and `Err(PlanAdmission::Malformed)` on out-of-bounds children
    /// or self-references in operators.
    pub fn shape_match(&self) -> Result<(), PlanAdmission> {
        // Bounds.
        if self.nodes.len() > MAX_NODES {
            return Err(PlanAdmission::Malformed);
        }
        if self.nodes.is_empty() {
            // An empty plan with root=0 has no valid root.
            return Err(PlanAdmission::Malformed);
        }
        if (self.root as usize) >= self.nodes.len() {
            return Err(PlanAdmission::Malformed);
        }

        let n = self.nodes.len();

        // Per-node structural checks.
        for (idx, node) in self.nodes.iter().enumerate() {
            match *node {
                Powl8Node::Silent | Powl8Node::StartNode | Powl8Node::EndNode => {}
                // The Breed enum itself bounds the discriminant to 0..=6;
                // the match here is purely defensive and total.
                Powl8Node::Activity(_) => {}
                Powl8Node::PartialOrder { start, count, rel } => {
                    let s = start as usize;
                    let c = count as usize;
                    if s.saturating_add(c) > n {
                        return Err(PlanAdmission::Malformed);
                    }
                    if c == 0 {
                        // Empty partial order is structurally fine but trivial.
                        continue;
                    }
                    // Kahn's algorithm on the local count-by-count submatrix.
                    // Local index `i` corresponds to global index `start + i`.
                    let mut indegree = [0u16; MAX_NODES];
                    for i in 0..c {
                        for j in 0..c {
                            if i != j && rel.is_edge(i, j) {
                                indegree[j] = indegree[j].saturating_add(1);
                            }
                        }
                    }
                    let mut queue: [usize; MAX_NODES] = [0usize; MAX_NODES];
                    let mut qhead = 0usize;
                    let mut qtail = 0usize;
                    for i in 0..c {
                        if indegree[i] == 0 {
                            queue[qtail] = i;
                            qtail += 1;
                        }
                    }
                    let mut peeled = 0usize;
                    while qhead < qtail {
                        let v = queue[qhead];
                        qhead += 1;
                        peeled += 1;
                        for w in 0..c {
                            if v != w && rel.is_edge(v, w) {
                                indegree[w] = indegree[w].saturating_sub(1);
                                if indegree[w] == 0 {
                                    queue[qtail] = w;
                                    qtail += 1;
                                }
                            }
                        }
                    }
                    if peeled != c {
                        return Err(PlanAdmission::Cyclic);
                    }
                }
                Powl8Node::OperatorSequence { a, b } | Powl8Node::OperatorParallel { a, b } => {
                    let ai = a as usize;
                    let bi = b as usize;
                    if ai >= n || bi >= n {
                        return Err(PlanAdmission::Malformed);
                    }
                    if ai == idx || bi == idx {
                        return Err(PlanAdmission::Malformed);
                    }
                }
                Powl8Node::Choice { branches, len } => {
                    let lc = len as usize;
                    if lc == 0 || lc > 4 {
                        return Err(PlanAdmission::Malformed);
                    }
                    for k in 0..lc {
                        let bi = branches[k] as usize;
                        if bi >= n {
                            return Err(PlanAdmission::Malformed);
                        }
                        if bi == idx {
                            // No self-loop: a Choice cannot select itself.
                            return Err(PlanAdmission::Malformed);
                        }
                    }
                }
                Powl8Node::Loop { body, max_iters } => {
                    let bi = body as usize;
                    if bi >= n {
                        return Err(PlanAdmission::Malformed);
                    }
                    if bi == idx {
                        return Err(PlanAdmission::Malformed);
                    }
                    if max_iters == 0 || max_iters > 16 {
                        return Err(PlanAdmission::Malformed);
                    }
                }
            }
        }

        // Whole-plan DFS-with-color over OperatorSequence dependencies.
        // Each OperatorSequence { a, b } contributes the edge a → b.
        // PartialOrder edges are handled per-node above; we still incorporate
        // them in the global cycle check by walking edges out of each
        // partial-order child to enforce composite acyclicity.
        const WHITE: u8 = 0;
        const GRAY: u8 = 1;
        const BLACK: u8 = 2;
        let mut color = [WHITE; MAX_NODES];

        // Build out-edges per node lazily inside dfs to avoid allocations.
        for start_node in 0..n {
            if color[start_node] != WHITE {
                continue;
            }
            // Iterative DFS using a small stack.
            let mut stack: [(usize, u32); MAX_NODES] = [(0usize, 0u32); MAX_NODES];
            let mut depth = 0usize;
            stack[depth] = (start_node, 0);
            depth += 1;
            color[start_node] = GRAY;
            while depth > 0 {
                let (u, edge_idx) = stack[depth - 1];
                if let Some(v) = self.nth_outgoing(u, edge_idx as usize) {
                    stack[depth - 1].1 = edge_idx + 1;
                    if v >= n {
                        return Err(PlanAdmission::Malformed);
                    }
                    match color[v] {
                        WHITE => {
                            color[v] = GRAY;
                            if depth >= MAX_NODES {
                                // Defensive: depth cannot exceed node count.
                                return Err(PlanAdmission::Malformed);
                            }
                            stack[depth] = (v, 0);
                            depth += 1;
                        }
                        GRAY => {
                            return Err(PlanAdmission::Cyclic);
                        }
                        _ => {}
                    }
                } else {
                    color[u] = BLACK;
                    depth -= 1;
                }
            }
        }

        Ok(())
    }

    /// Yield the `k`th outgoing edge target from node `u`, or `None` once
    /// exhausted. Outgoing edges are derived from `OperatorSequence` (a → b)
    /// and from `PartialOrder` local rel edges (mapped back to global indices).
    fn nth_outgoing(&self, u: usize, k: usize) -> Option<usize> {
        let mut counter = 0usize;
        // Outgoing edges from any node are produced by the structure of every
        // node in the plan, so we scan the plan once per query. This is O(n²)
        // overall for shape_match, which is fine at MAX_NODES = 64.
        for (idx, node) in self.nodes.iter().enumerate() {
            match *node {
                Powl8Node::OperatorSequence { a, b } if (a as usize) == u => {
                    if counter == k {
                        return Some(b as usize);
                    }
                    counter += 1;
                }
                Powl8Node::OperatorSequence { .. } => {}
                Powl8Node::PartialOrder { start, count, rel } => {
                    let s = start as usize;
                    let c = count as usize;
                    if u >= s && u < s + c {
                        let local_u = u - s;
                        for j in 0..c {
                            if local_u != j && rel.is_edge(local_u, j) {
                                if counter == k {
                                    return Some(s + j);
                                }
                                counter += 1;
                            }
                        }
                    }
                }
                Powl8Node::Choice { branches, len } => {
                    if idx == u {
                        let lc = (len as usize).min(4);
                        for k_b in 0..lc {
                            if counter == k {
                                return Some(branches[k_b] as usize);
                            }
                            counter += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Compute predecessors for every node into `preds[i]` using
    /// `OperatorSequence`, `OperatorParallel`, and `PartialOrder` edges.
    ///
    /// `OperatorSequence { a, b }` adds `a` as a predecessor of `b`.
    /// `OperatorParallel { a, b }` adds no predecessor edges between `a` and
    /// `b` (they are independent).
    /// `PartialOrder { start, count, rel }` adds local-edge predecessors
    /// mapped back to global indices.
    ///
    /// `preds[i]` is a 64-bit mask over node indices; bit `j` is set iff `j`
    /// is a direct predecessor of `i`.
    pub fn predecessor_masks(&self) -> [u64; MAX_NODES] {
        let mut preds = [0u64; MAX_NODES];
        let n = self.nodes.len().min(MAX_NODES);
        for (idx, node) in self.nodes.iter().enumerate() {
            match *node {
                Powl8Node::OperatorSequence { a, b } => {
                    let ai = a as usize;
                    let bi = b as usize;
                    if ai < n && bi < n {
                        preds[bi] |= 1u64 << ai;
                    }
                }
                Powl8Node::PartialOrder { start, count, rel } => {
                    let s = start as usize;
                    let c = count as usize;
                    if s.saturating_add(c) > n {
                        continue;
                    }
                    for i in 0..c {
                        for j in 0..c {
                            if i != j && rel.is_edge(i, j) {
                                preds[s + j] |= 1u64 << (s + i);
                            }
                        }
                    }
                }
                Powl8Node::Choice { branches, len } => {
                    if idx >= n {
                        continue;
                    }
                    let lc = (len as usize).min(4);
                    for k in 0..lc {
                        let bi = branches[k] as usize;
                        if bi < n {
                            preds[bi] |= 1u64 << idx;
                        }
                    }
                }
                _ => {}
            }
        }
        preds
    }

    /// Compile this authoring-time plan into a runtime-only [`CompiledPowl8`].
    ///
    /// The compile step:
    /// 1. Validates the plan via [`Self::shape_match`]; any error is returned.
    /// 2. Builds a directed dependency graph over the original node indices
    ///    using `OperatorSequence` (a → b) and `PartialOrder` rel edges
    ///    (mapped from local to global indices). `OperatorParallel` declares
    ///    no edges (children are independent).
    /// 3. Identifies the **executable** subset — every node except
    ///    `OperatorSequence`, `OperatorParallel`, and `PartialOrder`, which
    ///    are pure structural declarations.
    /// 4. Runs Kahn's algorithm restricted to executable nodes; only edges
    ///    whose endpoints are both executable contribute to indegree.
    /// 5. Returns a [`CompiledPowl8`] whose `order` is the topological order
    ///    of executable nodes (values are original `Powl8.nodes` indices),
    ///    `kinds` is the per-runtime-index kind, and `preds[i]` is a bitmask
    ///    over runtime indices (`0..order.len()`) of direct predecessors.
    ///
    /// Returns [`PlanAdmission::Malformed`] if the executable count exceeds
    /// 64 (the predecessor mask is `u64`), or [`PlanAdmission::Cyclic`] on a
    /// cycle defensively detected during compilation.
    pub fn compile(&self) -> Result<CompiledPowl8, PlanAdmission> {
        self.shape_match()?;

        let n = self.nodes.len();

        // Classify each original node: Some(kind) iff executable, None iff
        // it's a pure structural operator declaration.
        let mut kind_of: [Option<CompiledNodeKind>; MAX_NODES] = [None; MAX_NODES];
        for (idx, node) in self.nodes.iter().enumerate() {
            kind_of[idx] = match *node {
                Powl8Node::StartNode | Powl8Node::EndNode => Some(CompiledNodeKind::Boundary),
                Powl8Node::Silent => Some(CompiledNodeKind::Silent),
                Powl8Node::Activity(_) => Some(CompiledNodeKind::HookSlot(idx as u16)),
                Powl8Node::OperatorSequence { .. }
                | Powl8Node::OperatorParallel { .. }
                | Powl8Node::PartialOrder { .. }
                | Powl8Node::Loop { .. } => None,
                // Choice is executable: at runtime, the kernel evaluates a
                // selector and chooses exactly one branch. The selector slot
                // index is the original Choice node index.
                Powl8Node::Choice { .. } => Some(CompiledNodeKind::Choice {
                    selector_slot: idx as u16,
                }),
            };
        }

        // Loop unrolling: each `Powl8Node::Loop { body, max_iters }` contributes
        // (max_iters - 1) extra runtime copies of `body` chained in sequence
        // after the original `body` runtime entry. We track these as
        // `(after_orig_body, count)` pairs and insert them after the standard
        // Kahn pass, preserving topological correctness because each unrolled
        // copy depends only on the previous copy.
        let mut loop_unrolls: Vec<(u16, u8)> = Vec::new();
        for node in &self.nodes {
            if let Powl8Node::Loop { body, max_iters } = *node {
                if max_iters > 1 {
                    loop_unrolls.push((body, max_iters - 1));
                }
            }
        }

        // Count executable nodes (including loop unrolls); bail early if we
        // would exceed the u64 mask.
        let mut exec_count = 0usize;
        for k in kind_of.iter().take(n) {
            if k.is_some() {
                exec_count += 1;
            }
        }
        let unroll_extra: usize = loop_unrolls.iter().map(|(_, c)| *c as usize).sum();
        if exec_count + unroll_extra > 64 {
            return Err(PlanAdmission::Malformed);
        }

        // Outgoing adjacency over original indices, only retaining edges
        // where both endpoints are executable.
        let mut outgoing: [u64; MAX_NODES] = [0u64; MAX_NODES];
        let mut indegree: [u32; MAX_NODES] = [0u32; MAX_NODES];

        let try_add = |src: usize,
                       tgt: usize,
                       outgoing: &mut [u64; MAX_NODES],
                       indegree: &mut [u32; MAX_NODES]| {
            if src >= n || tgt >= n {
                return;
            }
            if kind_of[src].is_none() || kind_of[tgt].is_none() {
                return;
            }
            // Skip duplicate edges (idempotent set into the bitmask).
            let bit = 1u64 << tgt;
            if outgoing[src] & bit == 0 {
                outgoing[src] |= bit;
                indegree[tgt] = indegree[tgt].saturating_add(1);
            }
        };

        for (idx, node) in self.nodes.iter().enumerate() {
            match *node {
                Powl8Node::OperatorSequence { a, b } => {
                    try_add(a as usize, b as usize, &mut outgoing, &mut indegree);
                }
                // OperatorParallel declares independence — contributes no edges.
                Powl8Node::OperatorParallel { .. } => {}
                Powl8Node::PartialOrder { start, count, rel } => {
                    let s = start as usize;
                    let c = count as usize;
                    if s.saturating_add(c) > n {
                        continue;
                    }
                    for i in 0..c {
                        for j in 0..c {
                            if i != j && rel.is_edge(i, j) {
                                try_add(s + i, s + j, &mut outgoing, &mut indegree);
                            }
                        }
                    }
                }
                // Choice contributes itself as predecessor of each branch.
                Powl8Node::Choice { branches, len } => {
                    let lc = (len as usize).min(4);
                    for k in 0..lc {
                        try_add(idx, branches[k] as usize, &mut outgoing, &mut indegree);
                    }
                }
                // Loop is structural — body unrolling is handled below; the
                // Loop node itself contributes no edges to the dep graph.
                Powl8Node::Loop { .. } => {}
                _ => {}
            }
        }

        // Kahn's algorithm over executable nodes only.
        let mut order: Vec<u16> = Vec::with_capacity(exec_count);
        let mut queue: [usize; MAX_NODES] = [0usize; MAX_NODES];
        let mut qhead = 0usize;
        let mut qtail = 0usize;
        for idx in 0..n {
            if kind_of[idx].is_some() && indegree[idx] == 0 {
                queue[qtail] = idx;
                qtail += 1;
            }
        }
        while qhead < qtail {
            let u = queue[qhead];
            qhead += 1;
            order.push(u as u16);
            // Iterate outgoing edges from u.
            let mut row = outgoing[u];
            while row != 0 {
                let tgt = row.trailing_zeros() as usize;
                row &= row - 1;
                indegree[tgt] = indegree[tgt].saturating_sub(1);
                if indegree[tgt] == 0 && kind_of[tgt].is_some() {
                    queue[qtail] = tgt;
                    qtail += 1;
                }
            }
        }

        if order.len() != exec_count {
            // Defensive: shape_match should have caught any cycle.
            return Err(PlanAdmission::Cyclic);
        }

        // Build inverse lookup: original index → runtime index.
        let mut runtime_index: [Option<u8>; MAX_NODES] = [None; MAX_NODES];
        for (rt, &orig) in order.iter().enumerate() {
            runtime_index[orig as usize] = Some(rt as u8);
        }

        // Compute predecessor masks over runtime indices.
        let mut preds: Vec<u64> = vec![0u64; order.len()];
        let mut kinds: Vec<CompiledNodeKind> = Vec::with_capacity(order.len());
        for (rt, &orig) in order.iter().enumerate() {
            let kind = kind_of[orig as usize].expect("executable node must have a kind");
            kinds.push(kind);
            // For every executable predecessor src of orig, set bit at the
            // runtime index of src.
            for src in 0..n {
                if outgoing[src] & (1u64 << (orig as usize)) != 0 {
                    if let Some(src_rt) = runtime_index[src] {
                        preds[rt] |= 1u64 << src_rt;
                    }
                }
            }
        }

        // Append loop-unrolled body copies. Each `(body, extra)` extends the
        // body's runtime sequence by `extra` copies, each depending solely
        // on the previous one.
        for (body, extra) in &loop_unrolls {
            let body_rt = match runtime_index[*body as usize] {
                Some(rt) => rt as usize,
                None => continue, // Body wasn't executable; nothing to unroll.
            };
            let body_kind = match kind_of[*body as usize] {
                Some(k) => k,
                None => continue,
            };
            let mut prev_rt = body_rt;
            for _ in 0..*extra {
                let new_rt = order.len();
                if new_rt >= 64 {
                    return Err(PlanAdmission::Malformed);
                }
                order.push(*body);
                kinds.push(body_kind);
                preds.push(1u64 << prev_rt);
                prev_rt = new_rt;
            }
        }

        Ok(CompiledPowl8 {
            order,
            preds,
            kinds,
        })
    }
}

/// Runtime kind of a node in the compiled plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompiledNodeKind {
    /// Boundary marker (entry/exit of plan).
    Boundary,
    /// Silent / no-op runtime node — advanced on entry.
    Silent,
    /// Runtime activity node referencing an external slot index.
    HookSlot(u16),
    /// Phase-10 Choice node: at runtime exactly one of the branches advances,
    /// chosen by an external selector. `selector_slot` is the original
    /// `Powl8` node index of the Choice declaration — used by the runtime
    /// to look up the selector value (mask, oracle, RL policy, etc.).
    Choice {
        /// Original `Powl8.nodes` index of the Choice declaration.
        selector_slot: u16,
    },
}

/// Compiled POWL8: topologically ordered executable nodes with predecessor masks.
///
/// `order[i]` is the original [`Powl8`] node index of the `i`th executable
/// runtime node. Operator declarations (`OperatorSequence`, `OperatorParallel`,
/// `PartialOrder`) are stripped — they only contribute edges to the dependency
/// graph used to derive `preds`. `preds[i]` is a bitmask over runtime indices
/// (`0..order.len()`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledPowl8 {
    /// Topological order of executable nodes — values are original
    /// [`Powl8`] node indices.
    pub order: Vec<u16>,
    /// Per-runtime-index predecessor mask (over runtime indices, NOT original).
    pub preds: Vec<u64>,
    /// Per-runtime-index node kind.
    pub kinds: Vec<CompiledNodeKind>,
}

/// Advance policy for runtime walks of a [`CompiledPowl8`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvancePolicy {
    /// A node is advanced once it has been evaluated (regardless of fire/skip).
    OnEvaluation,
    /// A node is advanced only if its hook fired.
    OnFire,
}

impl Default for Powl8 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_relation_set_and_query() {
        let mut r = BinaryRelation::new();
        r.add_edge(0, 1);
        r.add_edge(2, 3);
        assert!(r.is_edge(0, 1));
        assert!(r.is_edge(2, 3));
        assert!(!r.is_edge(1, 0));
        assert!(!r.is_edge(63, 63));
    }

    #[test]
    fn binary_relation_oob_ignored() {
        let mut r = BinaryRelation::new();
        r.add_edge(MAX_NODES, 0);
        r.add_edge(0, MAX_NODES);
        assert!(!r.is_edge(MAX_NODES, 0));
        assert!(!r.is_edge(0, MAX_NODES));
    }

    #[test]
    fn powl8_push_caps_at_max_nodes() {
        let mut p = Powl8::new();
        for _ in 0..MAX_NODES {
            p.push(Powl8Node::Silent).unwrap();
        }
        assert!(matches!(
            p.push(Powl8Node::Silent),
            Err(PlanAdmission::Malformed)
        ));
    }

    #[test]
    fn shape_match_empty_is_malformed() {
        let p = Powl8::new();
        assert!(matches!(p.shape_match(), Err(PlanAdmission::Malformed)));
    }

    #[test]
    fn shape_match_root_oob_is_malformed() {
        let mut p = Powl8::new();
        p.push(Powl8Node::StartNode).unwrap();
        p.root = 5;
        assert!(matches!(p.shape_match(), Err(PlanAdmission::Malformed)));
    }

    #[test]
    fn shape_match_acyclic_partial_order_is_sound() {
        let mut p = Powl8::new();
        let s = p.push(Powl8Node::StartNode).unwrap();
        p.root = s;
        let _e = p.push(Powl8Node::Activity(Breed::Eliza)).unwrap();
        let _m = p.push(Powl8Node::Activity(Breed::Mycin)).unwrap();
        let _x = p.push(Powl8Node::Activity(Breed::Strips)).unwrap();
        let mut rel = BinaryRelation::new();
        rel.add_edge(0, 1);
        rel.add_edge(1, 2);
        let _po = p
            .push(Powl8Node::PartialOrder {
                start: 1,
                count: 3,
                rel,
            })
            .unwrap();
        assert!(p.shape_match().is_ok());
    }

    #[test]
    fn shape_match_cyclic_partial_order_is_cyclic() {
        let mut p = Powl8::new();
        p.push(Powl8Node::StartNode).unwrap();
        p.push(Powl8Node::Activity(Breed::Eliza)).unwrap();
        p.push(Powl8Node::Activity(Breed::Mycin)).unwrap();
        p.push(Powl8Node::Activity(Breed::Strips)).unwrap();
        let mut rel = BinaryRelation::new();
        rel.add_edge(0, 1);
        rel.add_edge(1, 2);
        rel.add_edge(2, 0);
        p.push(Powl8Node::PartialOrder {
            start: 1,
            count: 3,
            rel,
        })
        .unwrap();
        assert!(matches!(p.shape_match(), Err(PlanAdmission::Cyclic)));
    }

    #[test]
    fn shape_match_self_referencing_operator_is_malformed() {
        let mut p = Powl8::new();
        p.push(Powl8Node::StartNode).unwrap();
        p.push(Powl8Node::Activity(Breed::Eliza)).unwrap();
        p.push(Powl8Node::OperatorSequence { a: 1, b: 2 }).unwrap();
        // Index 2 references itself via b=2.
        assert!(matches!(p.shape_match(), Err(PlanAdmission::Malformed)));
    }

    #[test]
    fn shape_match_seq_oob_child_is_malformed() {
        let mut p = Powl8::new();
        p.push(Powl8Node::StartNode).unwrap();
        p.push(Powl8Node::OperatorSequence { a: 0, b: 99 }).unwrap();
        assert!(matches!(p.shape_match(), Err(PlanAdmission::Malformed)));
    }

    #[test]
    fn shape_match_seq_cycle_detected_globally() {
        let mut p = Powl8::new();
        // 4 nodes; build a cycle 0 -> 1 -> 2 -> 0 via OperatorSequence rows.
        p.push(Powl8Node::Activity(Breed::Eliza)).unwrap();
        p.push(Powl8Node::Activity(Breed::Mycin)).unwrap();
        p.push(Powl8Node::Activity(Breed::Strips)).unwrap();
        p.push(Powl8Node::OperatorSequence { a: 0, b: 1 }).unwrap();
        p.push(Powl8Node::OperatorSequence { a: 1, b: 2 }).unwrap();
        p.push(Powl8Node::OperatorSequence { a: 2, b: 0 }).unwrap();
        assert!(matches!(p.shape_match(), Err(PlanAdmission::Cyclic)));
    }

    #[test]
    fn compile_linear_plan_yields_topological_order() {
        // Plan: Start(0) → Eliza(1) → Mycin(2) → End(3)
        // Connected via OperatorSequence edges (4: 0→1, 5: 1→2, 6: 2→3).
        let mut p = Powl8::new();
        let s = p.push(Powl8Node::StartNode).unwrap();
        p.root = s;
        let _e = p.push(Powl8Node::Activity(Breed::Eliza)).unwrap();
        let _m = p.push(Powl8Node::Activity(Breed::Mycin)).unwrap();
        let _x = p.push(Powl8Node::EndNode).unwrap();
        p.push(Powl8Node::OperatorSequence { a: 0, b: 1 }).unwrap();
        p.push(Powl8Node::OperatorSequence { a: 1, b: 2 }).unwrap();
        p.push(Powl8Node::OperatorSequence { a: 2, b: 3 }).unwrap();

        let compiled = p.compile().expect("compile must succeed");
        assert_eq!(compiled.order, vec![0u16, 1, 2, 3]);
        assert_eq!(compiled.preds, vec![0u64, 0b1, 0b10, 0b100]);
        assert_eq!(
            compiled.kinds,
            vec![
                CompiledNodeKind::Boundary,
                CompiledNodeKind::HookSlot(1),
                CompiledNodeKind::HookSlot(2),
                CompiledNodeKind::Boundary,
            ]
        );
    }

    #[test]
    fn compile_partial_order_respects_rel_edges() {
        // 3 children with rel edges 0→1, 0→2.
        let mut p = Powl8::new();
        let s = p.push(Powl8Node::StartNode).unwrap();
        p.root = s;
        let _c0 = p.push(Powl8Node::Activity(Breed::Eliza)).unwrap();
        let _c1 = p.push(Powl8Node::Activity(Breed::Mycin)).unwrap();
        let _c2 = p.push(Powl8Node::Activity(Breed::Strips)).unwrap();
        let mut rel = BinaryRelation::new();
        rel.add_edge(0, 1);
        rel.add_edge(0, 2);
        p.push(Powl8Node::PartialOrder {
            start: 1,
            count: 3,
            rel,
        })
        .unwrap();

        let compiled = p.compile().expect("compile must succeed");

        // Find runtime indices of original nodes 1, 2, 3.
        let pos_of = |orig: u16| -> usize {
            compiled
                .order
                .iter()
                .position(|&v| v == orig)
                .expect("must contain orig")
        };
        let r1 = pos_of(1);
        let r2 = pos_of(2);
        let r3 = pos_of(3);

        // child0 (orig 1) precedes child1 (orig 2) and child2 (orig 3).
        assert!(r1 < r2, "child0 must precede child1");
        assert!(r1 < r3, "child0 must precede child2");

        // Predecessor masks for child1 and child2 must include child0's runtime bit.
        assert_ne!(
            compiled.preds[r2] & (1u64 << r1),
            0,
            "child1 must list child0 as predecessor"
        );
        assert_ne!(
            compiled.preds[r3] & (1u64 << r1),
            0,
            "child2 must list child0 as predecessor"
        );

        // child0 itself has no executable predecessors via the partial order.
        assert_eq!(compiled.preds[r1] & ((1u64 << r2) | (1u64 << r3)), 0);

        // PartialOrder declaration node is stripped: 4 executable nodes total
        // (Start + 3 activities).
        assert_eq!(compiled.order.len(), 4);
        assert_eq!(compiled.kinds.len(), 4);
        assert!(compiled.kinds.iter().all(|k| matches!(
            k,
            CompiledNodeKind::Boundary | CompiledNodeKind::HookSlot(_)
        )));
    }

    #[test]
    fn compile_strips_operator_nodes() {
        // Plan with 1 OperatorSequence node mixed in — the operator node
        // itself must not appear in compiled.order or compiled.kinds.
        let mut p = Powl8::new();
        let s = p.push(Powl8Node::StartNode).unwrap(); // 0
        p.root = s;
        p.push(Powl8Node::Activity(Breed::Eliza)).unwrap(); // 1
        p.push(Powl8Node::Activity(Breed::Mycin)).unwrap(); // 2
        let op = p.push(Powl8Node::OperatorSequence { a: 1, b: 2 }).unwrap(); // 3 — stripped
        p.push(Powl8Node::EndNode).unwrap(); // 4

        let compiled = p.compile().expect("compile must succeed");
        assert_eq!(compiled.order.len(), 4);
        assert!(
            !compiled.order.contains(&op),
            "operator declaration index must not appear in runtime order"
        );
        // All kinds must be Boundary or HookSlot — no leakage of operator semantics.
        for k in &compiled.kinds {
            assert!(matches!(
                k,
                CompiledNodeKind::Boundary | CompiledNodeKind::HookSlot(_)
            ));
        }
    }

    #[test]
    fn compile_returns_malformed_on_excess_runtime_nodes() {
        // The MAX_NODES cap is 64, so we cannot push a 65th executable node
        // through the public `push` API. We construct a Powl8 whose
        // `nodes` vec exceeds MAX_NODES via direct field access to model an
        // externally-constructed (e.g., deserialized) plan; compile() must
        // reject it because the predecessor mask is u64.
        let mut p = Powl8::new();
        for _ in 0..MAX_NODES {
            p.push(Powl8Node::Silent).unwrap();
        }
        // Forcibly add a 65th executable node to exceed the runtime cap.
        p.nodes.push(Powl8Node::Silent);
        assert_eq!(p.nodes.len(), 65);
        match p.compile() {
            Err(PlanAdmission::Malformed) => {}
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn compile_propagates_shape_match_errors() {
        // Cyclic plan via OperatorSequence: 0 → 1 → 0.
        let mut p = Powl8::new();
        p.push(Powl8Node::Activity(Breed::Eliza)).unwrap(); // 0
        p.push(Powl8Node::Activity(Breed::Mycin)).unwrap(); // 1
        p.push(Powl8Node::OperatorSequence { a: 0, b: 1 }).unwrap();
        p.push(Powl8Node::OperatorSequence { a: 1, b: 0 }).unwrap();
        match p.compile() {
            Err(PlanAdmission::Cyclic) => {}
            other => panic!("expected Cyclic, got {other:?}"),
        }
    }

    #[test]
    fn try_add_edge_rejects_oob() {
        let mut r = BinaryRelation::new();
        assert_eq!(
            r.try_add_edge(MAX_NODES, 0),
            Err(BinaryRelationError::OutOfBounds)
        );
        assert_eq!(
            r.try_add_edge(0, MAX_NODES),
            Err(BinaryRelationError::OutOfBounds)
        );
        assert_eq!(
            r.try_add_edge(MAX_NODES, MAX_NODES),
            Err(BinaryRelationError::OutOfBounds)
        );
        // Out-of-bounds calls must not have set any edge.
        assert!(!r.is_edge(0, 0));
    }

    #[test]
    fn try_add_edge_accepts_valid() {
        let mut r = BinaryRelation::new();
        assert_eq!(r.try_add_edge(0, 1), Ok(()));
        assert_eq!(r.try_add_edge(63, 63), Ok(()));
        assert!(r.is_edge(0, 1));
        assert!(r.is_edge(63, 63));
        assert!(!r.is_edge(1, 0));
    }
}
