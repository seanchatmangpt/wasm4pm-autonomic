use std::collections::{HashMap, HashSet, VecDeque};

// ── Graph representation ──────────────────────────────────────────────────────

pub struct Graph {
    pub nodes: Vec<String>,
    pub edges: Vec<(usize, usize)>, // directed edges (from, to) as node indices
    pub node_index: HashMap<String, usize>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            node_index: HashMap::new(),
        }
    }

    /// Add a node by name; returns its index. If it already exists, returns existing index.
    pub fn add_node(&mut self, name: &str) -> usize {
        if let Some(&idx) = self.node_index.get(name) {
            return idx;
        }
        let idx = self.nodes.len();
        self.nodes.push(name.to_string());
        self.node_index.insert(name.to_string(), idx);
        idx
    }

    /// Add a directed edge from `from` to `to`. Nodes are created if they do not exist.
    /// Self-loops are silently ignored.
    pub fn add_edge(&mut self, from: &str, to: &str) {
        let from_idx = self.add_node(from);
        let to_idx = self.add_node(to);
        if from_idx != to_idx {
            self.edges.push((from_idx, to_idx));
        }
    }

    /// Outgoing neighbors of `node`.
    pub fn neighbors(&self, node: usize) -> Vec<usize> {
        self.edges
            .iter()
            .filter(|&&(f, _)| f == node)
            .map(|&(_, t)| t)
            .collect()
    }

    /// Incoming neighbors of `node`.
    pub fn in_neighbors(&self, node: usize) -> Vec<usize> {
        self.edges
            .iter()
            .filter(|&&(_, t)| t == node)
            .map(|&(f, _)| f)
            .collect()
    }

    /// Out-degree of `node`.
    pub fn degree(&self, node: usize) -> usize {
        self.edges.iter().filter(|&&(f, _)| f == node).count()
    }

    /// True if there is a direct edge from `from` to `to`.
    pub fn is_connected(&self, from: usize, to: usize) -> bool {
        self.edges.iter().any(|&(f, t)| f == from && t == to)
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

// ── BFS shortest-path helper ──────────────────────────────────────────────────

/// BFS from `source` over directed edges.
/// Returns (distances, predecessors) where predecessors[v] = list of nodes that are
/// on a shortest path to v (i.e. the previous hop).
fn bfs_shortest_paths(
    graph: &Graph,
    source: usize,
) -> (Vec<Option<usize>>, Vec<Vec<usize>>) {
    let n = graph.nodes.len();
    let mut dist: Vec<Option<usize>> = vec![None; n];
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    dist[source] = Some(0);

    let mut queue = VecDeque::new();
    queue.push_back(source);

    while let Some(u) = queue.pop_front() {
        let d_u = dist[u].unwrap();
        for v in graph.neighbors(u) {
            match dist[v] {
                None => {
                    dist[v] = Some(d_u + 1);
                    preds[v].push(u);
                    queue.push_back(v);
                }
                Some(d_v) if d_v == d_u + 1 => {
                    // Another shortest path to v
                    preds[v].push(u);
                }
                _ => {}
            }
        }
    }

    (dist, preds)
}

// ── PageRank ──────────────────────────────────────────────────────────────────

/// Compute PageRank for all nodes via power iteration.
///
/// - `damping_factor`: typically 0.85
/// - `max_iters`: maximum number of iterations
/// - `tolerance`: stop when the max rank change across all nodes drops below this
///
/// Returns `HashMap<usize, f64>` mapping node index → rank.
pub fn page_rank(
    graph: &Graph,
    damping_factor: f64,
    max_iters: usize,
    tolerance: f64,
) -> HashMap<usize, f64> {
    let n = graph.nodes.len();
    if n == 0 {
        return HashMap::new();
    }

    let base = (1.0 - damping_factor) / n as f64;

    // Precompute out-degree for every node.
    let out_deg: Vec<usize> = (0..n).map(|i| graph.degree(i)).collect();

    let mut rank: Vec<f64> = vec![1.0 / n as f64; n];

    for _ in 0..max_iters {
        // Dangling node mass (nodes with out_degree == 0)
        let dangling_sum: f64 = (0..n)
            .filter(|&i| out_deg[i] == 0)
            .map(|i| rank[i])
            .sum();
        let dangling_contrib = damping_factor * dangling_sum / n as f64;

        let mut new_rank = vec![0.0_f64; n];
        for v in 0..n {
            // Contributions from in-neighbors
            let in_contrib: f64 = graph
                .in_neighbors(v)
                .iter()
                .map(|&u| rank[u] / out_deg[u] as f64)
                .sum();
            new_rank[v] = base + dangling_contrib + damping_factor * in_contrib;
        }

        // Check convergence
        let max_delta = (0..n)
            .map(|i| (new_rank[i] - rank[i]).abs())
            .fold(0.0_f64, f64::max);

        rank = new_rank;

        if max_delta < tolerance {
            break;
        }
    }

    (0..n).map(|i| (i, rank[i])).collect()
}

// ── Betweenness Centrality ────────────────────────────────────────────────────

/// Compute betweenness centrality for all nodes.
///
/// Uses Brandes' algorithm (BFS-based) to count the fraction of shortest paths
/// from every source that pass through each intermediate node.
///
/// The score is normalized by `(n-1)(n-2)/2` (undirected convention).
/// For directed graphs the normalization factor is `(n-1)(n-2)`.
/// Here we use the undirected convention to match the book's presentation.
///
/// Returns `HashMap<usize, f64>` mapping node index → centrality score.
pub fn betweenness_centrality(graph: &Graph) -> HashMap<usize, f64> {
    let n = graph.nodes.len();
    if n == 0 {
        return HashMap::new();
    }

    let mut centrality = vec![0.0_f64; n];

    for s in 0..n {
        // Stack of nodes in order of non-increasing distance from s
        let mut stack: Vec<usize> = Vec::new();
        // Predecessors on shortest paths from s
        let mut pred: Vec<Vec<usize>> = vec![Vec::new(); n];
        // Number of shortest paths from s to each node
        let mut sigma: Vec<f64> = vec![0.0; n];
        sigma[s] = 1.0;
        // Distance from s
        let mut dist: Vec<Option<usize>> = vec![None; n];
        dist[s] = Some(0);

        let mut queue = VecDeque::new();
        queue.push_back(s);

        while let Some(v) = queue.pop_front() {
            stack.push(v);
            let d_v = dist[v].unwrap();
            for w in graph.neighbors(v) {
                match dist[w] {
                    None => {
                        dist[w] = Some(d_v + 1);
                        queue.push_back(w);
                    }
                    _ => {}
                }
                if dist[w] == Some(d_v + 1) {
                    sigma[w] += sigma[v];
                    pred[w].push(v);
                }
            }
        }

        // Accumulation
        let mut delta = vec![0.0_f64; n];
        while let Some(w) = stack.pop() {
            for &v in &pred[w] {
                if sigma[w] > 0.0 {
                    delta[v] += (sigma[v] / sigma[w]) * (1.0 + delta[w]);
                }
            }
            if w != s {
                centrality[w] += delta[w];
            }
        }
    }

    // Normalize by (n-1)(n-2)/2  (undirected, pairs)
    let norm = if n > 2 {
        ((n - 1) * (n - 2)) as f64 / 2.0
    } else {
        1.0
    };

    (0..n)
        .map(|i| (i, centrality[i] / norm))
        .collect()
}

// ── Closeness Centrality ──────────────────────────────────────────────────────

/// Mean reciprocal distance to all other reachable nodes (harmonic closeness).
///
/// For isolated nodes the score is 0.
///
/// Returns `HashMap<usize, f64>` mapping node index → centrality score.
pub fn closeness_centrality(graph: &Graph) -> HashMap<usize, f64> {
    let n = graph.nodes.len();
    if n == 0 {
        return HashMap::new();
    }

    let mut result = HashMap::with_capacity(n);

    for s in 0..n {
        let (dist, _) = bfs_shortest_paths(graph, s);
        // Harmonic mean: sum of 1/d for all reachable t ≠ s
        let score: f64 = dist
            .iter()
            .enumerate()
            .filter(|&(t, d)| t != s && d.is_some())
            .map(|(_, d)| 1.0 / d.unwrap() as f64)
            .sum();
        result.insert(s, score);
    }

    result
}

// ── Connected Components ──────────────────────────────────────────────────────

/// Find weakly connected components (treating the graph as undirected).
///
/// Returns a `Vec<Vec<usize>>` where each inner `Vec` is the set of node indices
/// in one component, sorted in ascending order. Components are returned in order
/// of their smallest node index.
pub fn connected_components(graph: &Graph) -> Vec<Vec<usize>> {
    let n = graph.nodes.len();
    if n == 0 {
        return Vec::new();
    }

    let mut visited = vec![false; n];
    let mut components: Vec<Vec<usize>> = Vec::new();

    // Build an undirected adjacency list
    let mut adj: Vec<HashSet<usize>> = vec![HashSet::new(); n];
    for &(f, t) in &graph.edges {
        adj[f].insert(t);
        adj[t].insert(f);
    }

    for start in 0..n {
        if visited[start] {
            continue;
        }
        // BFS
        let mut component = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited[start] = true;

        while let Some(u) = queue.pop_front() {
            component.push(u);
            for &v in &adj[u] {
                if !visited[v] {
                    visited[v] = true;
                    queue.push_back(v);
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }

    components
}

// ── Friendship Paradox ────────────────────────────────────────────────────────

/// For each node, compare its (out-)degree to the mean (out-)degree of its
/// out-neighbors.
///
/// Returns `Vec<(node_idx, own_degree, avg_neighbor_degree)>`.
/// Nodes with no neighbors get `avg_neighbor_degree = 0.0`.
pub fn friendship_paradox(graph: &Graph) -> Vec<(usize, f64, f64)> {
    let n = graph.nodes.len();
    let degrees: Vec<f64> = (0..n).map(|i| graph.degree(i) as f64).collect();

    (0..n)
        .map(|i| {
            let nbrs = graph.neighbors(i);
            let own = degrees[i];
            let avg_nbr = if nbrs.is_empty() {
                0.0
            } else {
                nbrs.iter().map(|&j| degrees[j]).sum::<f64>() / nbrs.len() as f64
            };
            (i, own, avg_nbr)
        })
        .collect()
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ───────────────────────────────────────────────────────────────

    /// Build a simple directed graph:
    ///   A → B, A → C, B → C, C → D
    fn simple_graph() -> Graph {
        let mut g = Graph::new();
        g.add_edge("A", "B");
        g.add_edge("A", "C");
        g.add_edge("B", "C");
        g.add_edge("C", "D");
        g
    }

    // ── 1. Graph construction ─────────────────────────────────────────────────

    #[test]
    fn test_graph_construction() {
        let g = simple_graph();
        assert_eq!(g.nodes.len(), 4);
        assert_eq!(g.edges.len(), 4);

        let a = *g.node_index.get("A").unwrap();
        let b = *g.node_index.get("B").unwrap();
        let c = *g.node_index.get("C").unwrap();
        let d = *g.node_index.get("D").unwrap();

        assert!(g.is_connected(a, b));
        assert!(g.is_connected(a, c));
        assert!(!g.is_connected(b, a)); // directed
        assert!(!g.is_connected(d, c));
    }

    // ── 2. Neighbors and degree ───────────────────────────────────────────────

    #[test]
    fn test_neighbors_and_degree() {
        let g = simple_graph();
        let a = *g.node_index.get("A").unwrap();
        let c = *g.node_index.get("C").unwrap();

        // A has out-degree 2
        assert_eq!(g.degree(a), 2);
        let mut nbrs_a = g.neighbors(a);
        nbrs_a.sort_unstable();
        assert_eq!(nbrs_a.len(), 2);

        // C has one incoming neighbor: B (and also A)
        let in_c = g.in_neighbors(c);
        assert_eq!(in_c.len(), 2);
    }

    // ── 3. Self-loop ignored ──────────────────────────────────────────────────

    #[test]
    fn test_self_loop_ignored() {
        let mut g = Graph::new();
        g.add_edge("X", "X");
        // Edge should be dropped
        assert_eq!(g.edges.len(), 0);
        // Node still created once
        assert_eq!(g.nodes.len(), 1);
    }

    // ── 4. PageRank on a star graph ───────────────────────────────────────────

    #[test]
    fn test_page_rank_star() {
        // Hub → A, Hub → B, Hub → C
        let mut g = Graph::new();
        g.add_edge("Hub", "A");
        g.add_edge("Hub", "B");
        g.add_edge("Hub", "C");

        let pr = page_rank(&g, 0.85, 100, 1e-8);
        // Sanity: all ranks sum to ~1
        let total: f64 = pr.values().sum();
        assert!((total - 1.0).abs() < 1e-6, "PageRank should sum to 1, got {total}");

        // A, B, C receive the same rank (symmetric)
        let a = *g.node_index.get("A").unwrap();
        let b = *g.node_index.get("B").unwrap();
        let c = *g.node_index.get("C").unwrap();
        assert!((pr[&a] - pr[&b]).abs() < 1e-8);
        assert!((pr[&b] - pr[&c]).abs() < 1e-8);
    }

    // ── 5. PageRank empty graph ───────────────────────────────────────────────

    #[test]
    fn test_page_rank_empty() {
        let g = Graph::new();
        let pr = page_rank(&g, 0.85, 100, 1e-8);
        assert!(pr.is_empty());
    }

    // ── 6. Betweenness centrality on a path graph ─────────────────────────────

    #[test]
    fn test_betweenness_path() {
        // Linear directed path: A → B → C → D
        let mut g = Graph::new();
        g.add_edge("A", "B");
        g.add_edge("B", "C");
        g.add_edge("C", "D");

        let bc = betweenness_centrality(&g);

        let a = *g.node_index.get("A").unwrap();
        let b = *g.node_index.get("B").unwrap();
        let c = *g.node_index.get("C").unwrap();
        let d = *g.node_index.get("D").unwrap();

        // Endpoints A and D should have lower centrality than B and C
        // B lies on path A→C, A→D; C lies on path A→D, B→D
        assert!(bc[&b] > bc[&a], "B should have higher betweenness than A");
        assert!(bc[&c] > bc[&d], "C should have higher betweenness than D");
    }

    // ── 7. Connected components ───────────────────────────────────────────────

    #[test]
    fn test_connected_components() {
        let mut g = Graph::new();
        // Component 1: A — B — C
        g.add_edge("A", "B");
        g.add_edge("B", "C");
        // Component 2: D (isolated node — add it directly)
        g.add_node("D");
        // Component 3: E → F
        g.add_edge("E", "F");

        let mut comps = connected_components(&g);
        assert_eq!(comps.len(), 3, "Expected 3 components, got {}", comps.len());

        // Normalize for comparison: sort each component's sizes
        comps.sort_by_key(|c| c[0]);
        assert_eq!(comps[0].len(), 3); // A-B-C
        assert_eq!(comps[1].len(), 1); // D
        assert_eq!(comps[2].len(), 2); // E-F
    }

    // ── 8. Closeness centrality – fully connected chain ───────────────────────

    #[test]
    fn test_closeness_centrality_chain() {
        // A → B → C  (directed chain)
        let mut g = Graph::new();
        g.add_edge("A", "B");
        g.add_edge("B", "C");

        let cc = closeness_centrality(&g);
        let a = *g.node_index.get("A").unwrap();
        let b = *g.node_index.get("B").unwrap();
        let c = *g.node_index.get("C").unwrap();

        // A can reach B (dist 1) and C (dist 2): score = 1 + 0.5 = 1.5
        assert!((cc[&a] - 1.5).abs() < 1e-9, "A closeness = {}", cc[&a]);
        // B can reach C (dist 1): score = 1.0
        assert!((cc[&b] - 1.0).abs() < 1e-9, "B closeness = {}", cc[&b]);
        // C can reach nobody: score = 0.0
        assert!((cc[&c] - 0.0).abs() < 1e-9, "C closeness = {}", cc[&c]);
    }

    // ── 9. Friendship paradox ─────────────────────────────────────────────────

    #[test]
    fn test_friendship_paradox() {
        // Hub → A, Hub → B, Hub → C, A → Hub
        let mut g = Graph::new();
        g.add_edge("Hub", "A");
        g.add_edge("Hub", "B");
        g.add_edge("Hub", "C");
        g.add_edge("A", "Hub");

        let fp = friendship_paradox(&g);
        assert_eq!(fp.len(), 4);

        let hub_idx = *g.node_index.get("Hub").unwrap();
        let hub_entry = fp.iter().find(|&&(i, _, _)| i == hub_idx).unwrap();

        // Hub has out-degree 3; its neighbors (A, B, C) have degrees 1, 0, 0
        // avg neighbor degree = (1 + 0 + 0) / 3 ≈ 0.333
        assert_eq!(hub_entry.1, 3.0);
        assert!((hub_entry.2 - 1.0 / 3.0).abs() < 1e-9);
    }

    // ── 10. Single-node graph ─────────────────────────────────────────────────

    #[test]
    fn test_single_node() {
        let mut g = Graph::new();
        g.add_node("Solo");

        let pr = page_rank(&g, 0.85, 100, 1e-8);
        assert_eq!(pr.len(), 1);
        let (_, &v) = pr.iter().next().unwrap();
        assert!((v - 1.0).abs() < 1e-9);

        let comps = connected_components(&g);
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].len(), 1);

        let bc = betweenness_centrality(&g);
        assert_eq!(bc.len(), 1);
    }
}
