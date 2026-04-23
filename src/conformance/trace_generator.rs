use super::bitmask_replay::NetBitmask64;

/// All markings reachable from `start` via any sequence of invisible (tau) transitions.
/// BFS — same logic as the private epsilon_close in bitmask_replay.rs.
fn epsilon_close(net: &NetBitmask64, start: u64) -> Vec<u64> {
    let mut reachable: Vec<u64> = vec![start];
    let mut i = 0;
    while i < reachable.len() {
        let m = reachable[i];
        for &ti in &net.invisible_indices {
            let t = &net.transitions[ti];
            if (m & t.in_mask) == t.in_mask {
                let new_m = (m & !t.in_mask) | t.out_mask;
                if !reachable.contains(&new_m) {
                    reachable.push(new_m);
                }
            }
        }
        i += 1;
    }
    reachable
}

/// One step of forward simulation: given current marking set (after epsilon closure),
/// enumerate all (transition_index, resulting_markings_after_epsilon_close) for enabled
/// visible transitions.
fn enabled_visible(net: &NetBitmask64, markings: &[u64]) -> Vec<(usize, Vec<u64>)> {
    let mut result: Vec<(usize, Vec<u64>)> = Vec::new();

    for &m in markings {
        for ti in 0..net.transitions.len() {
            // Skip invisible transitions
            if net.invisible_indices.contains(&ti) {
                continue;
            }
            let t = &net.transitions[ti];
            if (m & t.in_mask) == t.in_mask {
                let new_m = (m & !t.in_mask) | t.out_mask;
                let new_markings = epsilon_close(net, new_m);
                // Avoid duplicate transition indices
                if !result.iter().any(|(idx, _)| *idx == ti) {
                    result.push((ti, new_markings));
                }
            }
        }
    }

    result
}

/// Generate a single positive trace by forward simulation.
/// Returns `Some(Vec<String>)` = sequence of activity names if the final marking is reached
/// within `max_steps`. Returns `None` if no valid trace is found (deadlock or steps exceeded).
///
/// `seed`: deterministic pseudo-random seed for choosing which transition to fire at each step.
pub fn generate_positive_trace(
    net: &NetBitmask64,
    max_steps: usize,
    seed: u64,
) -> Option<Vec<String>> {
    let mut markings = epsilon_close(net, net.initial_mask);
    let mut activities: Vec<String> = Vec::new();

    for step in 0..max_steps {
        // Check if any current marking satisfies the final marking condition
        if markings.iter().any(|&m| (m & net.final_mask) == net.final_mask) {
            return Some(activities);
        }

        let enabled = enabled_visible(net, &markings);

        if enabled.is_empty() {
            // Deadlock before reaching final marking
            return None;
        }

        // Deterministic pseudo-random choice
        let seed_step = seed
            .wrapping_add(step as u64)
            .wrapping_mul(0x9e3779b97f4a7c15);
        let chosen_idx = (seed_step % enabled.len() as u64) as usize;
        let (ti, new_markings) = &enabled[chosen_idx];

        // Look up the label for this transition index
        let label = net
            .label_index
            .iter()
            .find(|(_, indices)| indices.contains(ti))
            .map(|(lbl, _)| lbl.clone())
            .unwrap_or_default();

        markings = new_markings.clone();
        activities.push(label);
    }

    // One last check after the loop exhausts steps
    if markings.iter().any(|&m| (m & net.final_mask) == net.final_mask) {
        return Some(activities);
    }

    None
}

/// Generate `n_traces` distinct positive traces. Tries seeds 0, 1, 2, … until `n_traces` are
/// collected. Deduplicates: if the same sequence is generated again it is skipped.
/// If `max_tries` (= `n_traces * 10`) seeds are exhausted, returns however many were found.
pub fn generate_positive_traces(
    net: &NetBitmask64,
    n_traces: usize,
    max_steps: usize,
) -> Vec<Vec<String>> {
    let mut collected: Vec<Vec<String>> = Vec::with_capacity(n_traces);
    let max_tries = n_traces * 10;

    for seed in 0u64..max_tries as u64 {
        if collected.len() >= n_traces {
            break;
        }
        if let Some(trace) = generate_positive_trace(net, max_steps, seed) {
            if !collected.contains(&trace) {
                collected.push(trace);
            }
        }
    }

    collected
}

/// Generate negative traces by mutating positive traces.
/// For each positive trace, produce one negative by:
///   - 50% chance: delete a random activity (use `seed + index` as the randomizer)
///   - 50% chance: insert a random activity from `vocabulary` at a random position
///
/// `vocabulary`: all activity names (typically from `net_vocabulary`).
pub fn generate_negative_traces(
    positives: &[Vec<String>],
    vocabulary: &[String],
    seed: u64,
) -> Vec<Vec<String>> {
    let mut negatives: Vec<Vec<String>> = Vec::with_capacity(positives.len());

    for (i, trace) in positives.iter().enumerate() {
        let step_seed = seed
            .wrapping_add(i as u64)
            .wrapping_mul(0x9e3779b97f4a7c15);

        // 50% split: even → delete, odd → insert
        let mutated = if step_seed % 2 == 0 {
            // Delete a random activity
            if trace.is_empty() {
                trace.clone()
            } else {
                let del_idx = (step_seed.wrapping_mul(6364136223846793005) % trace.len() as u64)
                    as usize;
                let mut v = trace.clone();
                v.remove(del_idx);
                v
            }
        } else {
            // Insert a random activity from vocabulary at a random position
            if vocabulary.is_empty() {
                trace.clone()
            } else {
                let vocab_idx =
                    (step_seed.wrapping_mul(6364136223846793005) % vocabulary.len() as u64)
                        as usize;
                let insert_pos =
                    (step_seed.wrapping_mul(1442695040888963407) % (trace.len() as u64 + 1))
                        as usize;
                let mut v = trace.clone();
                v.insert(insert_pos, vocabulary[vocab_idx].clone());
                v
            }
        };

        negatives.push(mutated);
    }

    negatives
}

/// Extract the vocabulary (all visible transition labels) from the net.
pub fn net_vocabulary(net: &NetBitmask64) -> Vec<String> {
    net.label_index.iter().map(|(lbl, _)| lbl.clone()).collect()
}

/// Enumerate firing sequences of the net exhaustively via DFS, bounded by:
///   max_len: maximum number of visible events per trace
///   max_loop_iters: maximum times ANY single marking may appear in one path
///                   (0 means no loop allowed, 2 allows up to 2 repetitions)
///   max_traces: stop and return once this many complete traces have been found
///
/// A trace is "complete" when the current marking set contains any marking m
/// where (m & net.final_mask) == net.final_mask AND at least one visible event
/// was fired.
///
/// Returns a Vec of activity sequences (Vec<String>), deduplicated.
pub fn enumerate_language_bounded(
    net: &NetBitmask64,
    max_len: usize,
    max_loop_iters: usize,
    max_traces: usize,
) -> Vec<Vec<String>> {
    if max_traces == 0 {
        return vec![];
    }

    // Stack item: (markings, activities, visit_counts)
    // visit_counts: Vec<(marking, count)> — linear scan, bounded by path length
    type StackItem = (Vec<u64>, Vec<String>, Vec<(u64, usize)>);

    let initial_markings = epsilon_close(net, net.initial_mask);
    let mut stack: Vec<StackItem> = vec![(initial_markings, vec![], vec![])];

    let mut results: Vec<Vec<String>> = Vec::new();
    let mut seen: std::collections::HashSet<Vec<String>> = std::collections::HashSet::new();

    while let Some((markings, activities, visit_counts)) = stack.pop() {
        // Check completion: final marking reached and at least one visible event fired
        if !activities.is_empty()
            && markings.iter().any(|&m| (m & net.final_mask) == net.final_mask)
        {
            if seen.insert(activities.clone()) {
                results.push(activities);
                if results.len() >= max_traces {
                    break;
                }
            }
            // Don't expand further from a completed trace — continue exploring other branches
            continue;
        }

        // Length cap: don't expand beyond max_len visible events
        if activities.len() >= max_len {
            continue;
        }

        // Expand via enabled visible transitions
        for (ti, new_markings) in enabled_visible(net, &markings) {
            // Loop detection: check if any marking in new_markings has been visited
            // too many times in this path (>= max_loop_iters + 1 means we'd exceed the limit)
            let loop_exceeded = new_markings.iter().any(|&m| {
                let count = visit_counts
                    .iter()
                    .find(|(vm, _)| *vm == m)
                    .map(|(_, c)| *c)
                    .unwrap_or(0);
                count >= max_loop_iters + 1
            });
            if loop_exceeded {
                continue;
            }

            // Build new visit_counts: copy and increment for each marking in new_markings
            let mut new_visit_counts = visit_counts.clone();
            for &m in &new_markings {
                if let Some(entry) = new_visit_counts.iter_mut().find(|(vm, _)| *vm == m) {
                    entry.1 += 1;
                } else {
                    new_visit_counts.push((m, 1));
                }
            }

            // Look up the label for this transition index
            let label = net
                .label_index
                .iter()
                .find(|(_, indices)| indices.contains(&ti))
                .map(|(lbl, _)| lbl.clone())
                .unwrap_or_default();

            let mut new_activities = activities.clone();
            new_activities.push(label);

            stack.push((new_markings, new_activities, new_visit_counts));
        }
    }

    results
}

/// Returns the count of distinct traces found by exhaustive bounded enumeration.
/// Runs `enumerate_language_bounded` with `max_traces = 100_000`.
pub fn language_size_estimate(net: &NetBitmask64, max_len: usize, max_loop_iters: usize) -> usize {
    enumerate_language_bounded(net, max_len, max_loop_iters, 100_000).len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::petri_net::{Arc, PetriNet, Place, Transition};
    use crate::utils::dense_kernel::{fnv1a_64, PackedKeyTable};

    /// Build the simple linear net: p0 -[a]-> p1 -[b]-> p2
    fn simple_net() -> NetBitmask64 {
        let mut im: PackedKeyTable<String, usize> = PackedKeyTable::new();
        im.insert(fnv1a_64(b"p0"), "p0".into(), 1);
        let mut fm: PackedKeyTable<String, usize> = PackedKeyTable::new();
        fm.insert(fnv1a_64(b"p2"), "p2".into(), 1);
        let pn = PetriNet {
            places: vec![
                Place { id: "p0".into() },
                Place { id: "p1".into() },
                Place { id: "p2".into() },
            ],
            transitions: vec![
                Transition { id: "t_a".into(), label: "a".into(), is_invisible: Some(false) },
                Transition { id: "t_b".into(), label: "b".into(), is_invisible: Some(false) },
            ],
            arcs: vec![
                Arc { from: "p0".into(), to: "t_a".into(), weight: None },
                Arc { from: "t_a".into(), to: "p1".into(), weight: None },
                Arc { from: "p1".into(), to: "t_b".into(), weight: None },
                Arc { from: "t_b".into(), to: "p2".into(), weight: None },
            ],
            initial_marking: im,
            final_markings: vec![fm],
            ..Default::default()
        };
        NetBitmask64::from_petri_net(&pn)
    }

    /// Build a small choice net: p0 -[a]-> p1, p0 -[b]-> p1, p1 -[c]-> p2
    fn choice_net() -> NetBitmask64 {
        let mut im: PackedKeyTable<String, usize> = PackedKeyTable::new();
        im.insert(fnv1a_64(b"p0"), "p0".into(), 1);
        let mut fm: PackedKeyTable<String, usize> = PackedKeyTable::new();
        fm.insert(fnv1a_64(b"p2"), "p2".into(), 1);
        let pn = PetriNet {
            places: vec![
                Place { id: "p0".into() },
                Place { id: "p1".into() },
                Place { id: "p2".into() },
            ],
            transitions: vec![
                Transition { id: "t_a".into(), label: "a".into(), is_invisible: Some(false) },
                Transition { id: "t_b".into(), label: "b".into(), is_invisible: Some(false) },
                Transition { id: "t_c".into(), label: "c".into(), is_invisible: Some(false) },
            ],
            arcs: vec![
                Arc { from: "p0".into(), to: "t_a".into(), weight: None },
                Arc { from: "t_a".into(), to: "p1".into(), weight: None },
                Arc { from: "p0".into(), to: "t_b".into(), weight: None },
                Arc { from: "t_b".into(), to: "p1".into(), weight: None },
                Arc { from: "p1".into(), to: "t_c".into(), weight: None },
                Arc { from: "t_c".into(), to: "p2".into(), weight: None },
            ],
            initial_marking: im,
            final_markings: vec![fm],
            ..Default::default()
        };
        NetBitmask64::from_petri_net(&pn)
    }

    #[test]
    fn test_net_vocabulary_simple() {
        let net = simple_net();
        let vocab = net_vocabulary(&net);
        // label_index is sorted, so expect ["a", "b"]
        assert_eq!(vocab, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_net_vocabulary_choice() {
        let net = choice_net();
        let vocab = net_vocabulary(&net);
        // Sorted: ["a", "b", "c"]
        assert_eq!(vocab, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[test]
    fn test_generate_positive_trace_simple() {
        let net = simple_net();
        // The only valid trace is ["a", "b"]
        let trace = generate_positive_trace(&net, 10, 0);
        assert!(trace.is_some(), "Should find a trace on the simple linear net");
        let trace = trace.unwrap();
        assert_eq!(trace, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_generate_positive_trace_within_max_steps() {
        let net = choice_net();
        // Valid traces: ["a","c"] or ["b","c"]
        for seed in 0u64..20 {
            if let Some(trace) = generate_positive_trace(&net, 10, seed) {
                assert_eq!(trace.len(), 2, "Choice net traces must have length 2");
                assert_eq!(trace[1], "c", "Second activity must always be c");
                assert!(
                    trace[0] == "a" || trace[0] == "b",
                    "First activity must be a or b, got {}",
                    trace[0]
                );
            }
        }
        // At least one seed should succeed
        let found = (0u64..20).any(|s| generate_positive_trace(&net, 10, s).is_some());
        assert!(found, "Should generate at least one trace from the choice net");
    }

    #[test]
    fn test_generate_positive_traces_deduplication() {
        let net = simple_net();
        // Simple net has only one distinct trace; requesting 5 should still yield 1
        let traces = generate_positive_traces(&net, 5, 20);
        assert_eq!(traces.len(), 1, "Simple net has only one distinct trace");
        assert_eq!(traces[0], vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_generate_positive_traces_choice() {
        let net = choice_net();
        // Choice net has two valid traces: ["a","c"] and ["b","c"]
        let traces = generate_positive_traces(&net, 5, 20);
        assert!(traces.len() <= 2, "Choice net has at most 2 distinct traces");
        assert!(!traces.is_empty(), "Should find at least one trace");
        for t in &traces {
            assert_eq!(t.len(), 2);
            assert_eq!(t[1], "c");
        }
    }

    #[test]
    fn test_negative_traces_differ_from_positives() {
        let net = simple_net();
        let positives = generate_positive_traces(&net, 3, 20);
        assert!(!positives.is_empty());

        let vocab = net_vocabulary(&net);
        let negatives = generate_negative_traces(&positives, &vocab, 42);

        assert_eq!(negatives.len(), positives.len());

        // At least one negative must differ from its corresponding positive
        let any_different = positives
            .iter()
            .zip(negatives.iter())
            .any(|(p, n)| p != n);
        assert!(any_different, "At least one negative trace must differ from its positive");
    }

    #[test]
    fn test_negative_traces_length_change() {
        let net = choice_net();
        let positives = generate_positive_traces(&net, 4, 20);
        let vocab = net_vocabulary(&net);
        let negatives = generate_negative_traces(&positives, &vocab, 7);

        assert_eq!(negatives.len(), positives.len());
        for (p, n) in positives.iter().zip(negatives.iter()) {
            // Negatives should be either one shorter (delete) or one longer (insert)
            let len_diff = (p.len() as i64 - n.len() as i64).unsigned_abs() as usize;
            assert_eq!(
                len_diff, 1,
                "Negative trace length should differ by exactly 1 from positive; positive={p:?}, negative={n:?}"
            );
        }
    }

    #[test]
    fn test_epsilon_close_no_invisible() {
        // On the simple net (no invisible transitions) epsilon_close returns exactly {start}
        let net = simple_net();
        let closed = epsilon_close(&net, net.initial_mask);
        assert_eq!(closed, vec![net.initial_mask]);
    }

    // -----------------------------------------------------------------------
    // enumerate_language_bounded tests
    // -----------------------------------------------------------------------

    /// Build a looping net: p0 -[a]-> p1 -[b]-> p0 (loop), p1 -[c]-> p2 (exit).
    /// Language with max_loop_iters=0: ["a","c"]
    /// Language with max_loop_iters=1: ["a","c"], ["a","b","a","c"]
    fn loop_net() -> NetBitmask64 {
        let mut im: PackedKeyTable<String, usize> = PackedKeyTable::new();
        im.insert(fnv1a_64(b"p0"), "p0".into(), 1);
        let mut fm: PackedKeyTable<String, usize> = PackedKeyTable::new();
        fm.insert(fnv1a_64(b"p2"), "p2".into(), 1);
        let pn = PetriNet {
            places: vec![
                Place { id: "p0".into() },
                Place { id: "p1".into() },
                Place { id: "p2".into() },
            ],
            transitions: vec![
                Transition { id: "t_a".into(), label: "a".into(), is_invisible: Some(false) },
                Transition { id: "t_b".into(), label: "b".into(), is_invisible: Some(false) },
                Transition { id: "t_c".into(), label: "c".into(), is_invisible: Some(false) },
            ],
            arcs: vec![
                // a: p0 -> p1
                Arc { from: "p0".into(), to: "t_a".into(), weight: None },
                Arc { from: "t_a".into(), to: "p1".into(), weight: None },
                // b: p1 -> p0  (the loop back)
                Arc { from: "p1".into(), to: "t_b".into(), weight: None },
                Arc { from: "t_b".into(), to: "p0".into(), weight: None },
                // c: p1 -> p2  (exit to final)
                Arc { from: "p1".into(), to: "t_c".into(), weight: None },
                Arc { from: "t_c".into(), to: "p2".into(), weight: None },
            ],
            initial_marking: im,
            final_markings: vec![fm],
            ..Default::default()
        };
        NetBitmask64::from_petri_net(&pn)
    }

    #[test]
    fn test_enumerate_language_bounded_linear_net_exactly_one_trace() {
        let net = simple_net();
        let traces = enumerate_language_bounded(&net, 10, 0, 100);
        assert_eq!(traces.len(), 1, "Linear net has exactly one trace");
        assert_eq!(traces[0], vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_enumerate_language_bounded_choice_net_both_branches() {
        let net = choice_net();
        let traces = enumerate_language_bounded(&net, 10, 0, 100);
        assert_eq!(traces.len(), 2, "Choice net has exactly 2 distinct traces");

        let mut sorted = traces.clone();
        sorted.sort();
        assert_eq!(sorted[0], vec!["a".to_string(), "c".to_string()]);
        assert_eq!(sorted[1], vec!["b".to_string(), "c".to_string()]);
    }

    #[test]
    fn test_enumerate_language_bounded_loop_net_no_loop() {
        // max_loop_iters=0: only the direct exit trace should be found
        let net = loop_net();
        let traces = enumerate_language_bounded(&net, 10, 0, 100);
        assert_eq!(traces.len(), 1, "With max_loop_iters=0 only one trace is allowed");
        assert_eq!(traces[0], vec!["a".to_string(), "c".to_string()]);
    }

    #[test]
    fn test_enumerate_language_bounded_loop_net_one_iteration() {
        // max_loop_iters=1: direct trace plus one loop iteration
        let net = loop_net();
        let mut traces = enumerate_language_bounded(&net, 10, 1, 100);
        traces.sort();
        assert_eq!(traces.len(), 2, "With max_loop_iters=1 exactly 2 traces are expected");
        // After lex sort: ["a","b","a","c"] < ["a","c"]  (because "b" < "c" at index 1)
        assert_eq!(
            traces[0],
            vec!["a".to_string(), "b".to_string(), "a".to_string(), "c".to_string()]
        );
        assert_eq!(traces[1], vec!["a".to_string(), "c".to_string()]);
    }

    #[test]
    fn test_enumerate_language_bounded_deduplication() {
        // The simple linear net can only generate one trace; requesting many should still
        // yield exactly 1 deduplicated result.
        let net = simple_net();
        let traces = enumerate_language_bounded(&net, 20, 2, 1_000);
        assert_eq!(traces.len(), 1, "Deduplication must prevent identical traces");
    }

    #[test]
    fn test_enumerate_language_bounded_max_traces_zero() {
        let net = simple_net();
        let traces = enumerate_language_bounded(&net, 10, 0, 0);
        assert!(traces.is_empty(), "max_traces=0 must return empty vec");
    }

    #[test]
    fn test_enumerate_language_bounded_respects_max_traces_cap() {
        let net = choice_net();
        // There are only 2 distinct traces; capping at 1 should return at most 1
        let traces = enumerate_language_bounded(&net, 10, 0, 1);
        assert_eq!(traces.len(), 1, "max_traces=1 must return at most 1 trace");
    }

    #[test]
    fn test_language_size_estimate_simple() {
        let net = simple_net();
        assert_eq!(language_size_estimate(&net, 10, 0), 1);
    }

    #[test]
    fn test_language_size_estimate_choice() {
        let net = choice_net();
        assert_eq!(language_size_estimate(&net, 10, 0), 2);
    }
}
