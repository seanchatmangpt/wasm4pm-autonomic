use crate::models::petri_net::PetriNet;
use crate::models::{AttributeValue, EventLog, Trace};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct ReplayResult {
    pub missing: u32,
    pub remaining: u32,
    pub produced: u32,
    pub consumed: u32,
}

impl ReplayResult {
    pub fn fitness(&self) -> f64 {
        let total = self.consumed + self.missing;
        if total == 0 && self.produced == 0 {
            return 1.0;
        }
        let denom = (total + self.produced) as f64;
        if denom == 0.0 {
            return 1.0;
        }
        1.0 - (self.missing as f64 + self.remaining as f64) / denom
    }

    #[inline]
    pub fn is_perfect(&self) -> bool {
        self.missing == 0 && self.remaining == 0
    }
}

pub(crate) struct TransMask {
    pub(crate) in_mask: u64,
    pub(crate) out_mask: u64,
    pub(crate) is_invisible: bool,
    in_popcount: u32,
    out_popcount: u32,
}

pub struct NetBitmask64 {
    pub initial_mask: u64,
    pub final_mask: u64,
    pub n_places: usize,
    pub(crate) transitions: Vec<TransMask>,
    /// Sorted by label for O(log n) binary search
    pub(crate) label_index: Vec<(String, Vec<usize>)>,
    /// Pre-filtered invisible transition indices for fast fixpoint loop
    pub(crate) invisible_indices: Vec<usize>,
}

impl NetBitmask64 {
    pub fn from_petri_net(net: &PetriNet) -> Self {
        let n_places = net.places.len();
        assert!(
            n_places <= 64,
            "NetBitmask64 requires ≤64 places, got {}",
            n_places
        );

        let mut place_bit: HashMap<&str, u64> = HashMap::with_capacity(n_places);
        for (i, p) in net.places.iter().enumerate() {
            place_bit.insert(p.id.as_str(), 1u64 << i);
        }

        let mut initial_mask = 0u64;
        for (_, p_id, count) in net.initial_marking.iter() {
            if *count > 0 {
                if let Some(&bit) = place_bit.get(p_id.as_str()) {
                    initial_mask |= bit;
                }
            }
        }

        let mut final_mask = 0u64;
        if let Some(fm) = net.final_markings.first() {
            for (_, p_id, count) in fm.iter() {
                if *count > 0 {
                    if let Some(&bit) = place_bit.get(p_id.as_str()) {
                        final_mask |= bit;
                    }
                }
            }
        }

        let n_trans = net.transitions.len();
        let mut in_masks = vec![0u64; n_trans];
        let mut out_masks = vec![0u64; n_trans];

        let mut trans_idx: HashMap<&str, usize> = HashMap::with_capacity(n_trans);
        for (i, t) in net.transitions.iter().enumerate() {
            trans_idx.insert(t.id.as_str(), i);
        }

        for arc in &net.arcs {
            if let Some(&ti) = trans_idx.get(arc.to.as_str()) {
                if let Some(&bit) = place_bit.get(arc.from.as_str()) {
                    in_masks[ti] |= bit;
                }
            } else if let Some(&ti) = trans_idx.get(arc.from.as_str()) {
                if let Some(&bit) = place_bit.get(arc.to.as_str()) {
                    out_masks[ti] |= bit;
                }
            }
        }

        let transitions: Vec<TransMask> = net
            .transitions
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let in_mask = in_masks[i];
                let out_mask = out_masks[i];
                let is_invisible = t.is_invisible.unwrap_or(false)
                    || t.label.starts_with('$')
                    || t.label.is_empty();
                TransMask {
                    in_mask,
                    out_mask,
                    is_invisible,
                    in_popcount: in_mask.count_ones(),
                    out_popcount: out_mask.count_ones(),
                }
            })
            .collect();

        let mut label_map: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, t) in net.transitions.iter().enumerate() {
            if !transitions[i].is_invisible {
                label_map.entry(t.label.clone()).or_default().push(i);
            }
        }
        let mut label_index: Vec<(String, Vec<usize>)> = label_map.into_iter().collect();
        label_index.sort_by(|a, b| a.0.cmp(&b.0));

        let invisible_indices: Vec<usize> = transitions
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_invisible)
            .map(|(i, _)| i)
            .collect();

        Self {
            initial_mask,
            final_mask,
            n_places,
            transitions,
            label_index,
            invisible_indices,
        }
    }
}

#[inline]
fn fire_invisible(net: &NetBitmask64, marking: &mut u64) {
    let mut changed = true;
    while changed {
        changed = false;
        for &i in &net.invisible_indices {
            let t = &net.transitions[i];
            if (*marking & t.in_mask) == t.in_mask {
                *marking = (*marking & !t.in_mask) | t.out_mask;
                changed = true;
                break;
            }
        }
    }
}

pub fn replay_trace(net: &NetBitmask64, trace: &Trace) -> ReplayResult {
    let mut marking = net.initial_mask;
    let mut missing: u32 = 0;
    let mut consumed: u32 = 0;
    let mut produced: u32 = net.initial_mask.count_ones();

    fire_invisible(net, &mut marking);

    for event in &trace.events {
        let activity = event
            .attributes
            .iter()
            .find(|a| a.key == "concept:name")
            .and_then(|a| {
                if let AttributeValue::String(s) = &a.value {
                    Some(s.as_str())
                } else {
                    None
                }
            });

        let Some(activity) = activity else { continue };

        let t_indices = match net
            .label_index
            .binary_search_by(|(k, _)| k.as_str().cmp(activity))
        {
            Ok(pos) => &net.label_index[pos].1,
            Err(_) => continue,
        };

        let t_idx = t_indices
            .iter()
            .copied()
            .find(|&i| (marking & net.transitions[i].in_mask) == net.transitions[i].in_mask)
            .unwrap_or(t_indices[0]);

        let t = &net.transitions[t_idx];

        let need = t.in_mask & !marking;
        if need != 0 {
            missing += need.count_ones();
            marking |= need;
        }

        marking = (marking & !t.in_mask) | t.out_mask;
        consumed += t.in_popcount;
        produced += t.out_popcount;

        fire_invisible(net, &mut marking);
    }

    // Consume final marking
    let final_needed = net.final_mask.count_ones();
    let final_have = (marking & net.final_mask).count_ones();
    if final_needed > final_have {
        missing += final_needed - final_have;
        marking |= net.final_mask & !marking;
    }
    consumed += final_needed;
    marking &= !net.final_mask;
    let remaining = marking.count_ones();

    ReplayResult { missing, remaining, produced, consumed }
}

pub fn replay_log(net: &NetBitmask64, log: &EventLog) -> Vec<ReplayResult> {
    log.traces.iter().map(|t| replay_trace(net, t)).collect()
}

/// All markings reachable from `start` by firing any sequence of invisible transitions (BFS).
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

/// Exact language membership check: returns true iff the trace is in the net's language.
/// Maintains the full set of reachable markings at each step (BFS — no greedy choices).
pub fn in_language(net: &NetBitmask64, trace: &Trace) -> bool {
    let mut markings = epsilon_close(net, net.initial_mask);

    for event in &trace.events {
        let activity = event
            .attributes
            .iter()
            .find(|a| a.key == "concept:name")
            .and_then(|a| {
                if let AttributeValue::String(s) = &a.value {
                    Some(s.as_str())
                } else {
                    None
                }
            });
        let Some(activity) = activity else { continue };

        let t_indices = match net
            .label_index
            .binary_search_by(|(k, _)| k.as_str().cmp(activity))
        {
            Ok(pos) => &net.label_index[pos].1,
            Err(_) => continue, // unknown activity — skip like a τ
        };

        let mut next: Vec<u64> = Vec::new();
        for &m in &markings {
            for &ti in t_indices {
                let t = &net.transitions[ti];
                if (m & t.in_mask) == t.in_mask {
                    let new_m = (m & !t.in_mask) | t.out_mask;
                    for em in epsilon_close(net, new_m) {
                        if !next.contains(&em) {
                            next.push(em);
                        }
                    }
                }
            }
        }

        if next.is_empty() {
            return false;
        }
        markings = next;
    }

    markings.iter().any(|&m| (m & net.final_mask) == net.final_mask)
}

/// Count how many traces in `log` are genuinely in the language of `net` (no clamping).
pub fn count_in_language(net: &NetBitmask64, log: &EventLog) -> usize {
    log.traces.iter().filter(|t| in_language(net, t)).count()
}

/// Classify using exact language membership. If the count of accepted traces equals
/// n_target, return those directly. Otherwise fall back to fitness ranking to fill the gap.
pub fn classify_exact(net: &NetBitmask64, log: &EventLog, n_target: usize) -> Vec<bool> {
    let in_lang: Vec<bool> = log.traces.iter().map(|t| in_language(net, t)).collect();
    let n_accepted = in_lang.iter().filter(|&&b| b).count();

    if n_accepted == n_target {
        return in_lang;
    }

    // Fallback: rank accepted traces by fitness descending, then fill from rejected by fitness
    let results = replay_log(net, log);
    let mut accepted: Vec<(usize, f64)> = in_lang
        .iter()
        .enumerate()
        .filter(|(_, &b)| b)
        .map(|(i, _)| (i, results[i].fitness()))
        .collect();
    let mut rejected: Vec<(usize, f64)> = in_lang
        .iter()
        .enumerate()
        .filter(|(_, &b)| !b)
        .map(|(i, _)| (i, results[i].fitness()))
        .collect();

    accepted.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    rejected.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });

    let mut out = vec![false; log.traces.len()];
    let take_accepted = n_accepted.min(n_target);
    for &(i, _) in &accepted[..take_accepted] {
        out[i] = true;
    }
    let fill = n_target.saturating_sub(take_accepted);
    for &(i, _) in rejected.iter().take(fill) {
        out[i] = true;
    }
    out
}

/// Classify traces: exact conformance first, then rank by fitness to fill n_target.
pub fn classify(results: &[ReplayResult], n_target: usize) -> Vec<bool> {
    let mut perfect: Vec<usize> = Vec::new();
    let mut imperfect: Vec<(usize, f64)> = Vec::new();

    for (i, r) in results.iter().enumerate() {
        if r.is_perfect() {
            perfect.push(i);
        } else {
            imperfect.push((i, r.fitness()));
        }
    }

    imperfect.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });

    let mut out = vec![false; results.len()];

    if perfect.len() >= n_target {
        perfect.sort_unstable();
        for &i in &perfect[..n_target] {
            out[i] = true;
        }
    } else {
        for &i in &perfect {
            out[i] = true;
        }
        let fill = n_target.saturating_sub(perfect.len());
        for &(i, _) in imperfect.iter().take(fill) {
            out[i] = true;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::petri_net::{Arc, PetriNet, Place, Transition};

    fn simple_net() -> PetriNet {
        // p0 -a-> p1 -b-> p2  (linear sequence a then b)
        use crate::utils::dense_kernel::{fnv1a_64, PackedKeyTable};
        let mut im: PackedKeyTable<String, usize> = PackedKeyTable::new();
        im.insert(fnv1a_64(b"p0"), "p0".into(), 1);
        let mut fm: PackedKeyTable<String, usize> = PackedKeyTable::new();
        fm.insert(fnv1a_64(b"p2"), "p2".into(), 1);
        PetriNet {
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
        }
    }

    fn make_trace(acts: &[&str]) -> Trace {
        use crate::models::{Attribute, Event};
        Trace {
            id: "t".into(),
            attributes: vec![],
            events: acts
                .iter()
                .map(|&a| Event {
                    attributes: vec![Attribute {
                        key: "concept:name".into(),
                        value: AttributeValue::String(a.into()),
                    }],
                })
                .collect(),
        }
    }

    #[test]
    fn test_perfect_trace() {
        let net = simple_net();
        let bm = NetBitmask64::from_petri_net(&net);
        let trace = make_trace(&["a", "b"]);
        let r = replay_trace(&bm, &trace);
        assert!(r.is_perfect(), "a,b should be perfect on linear net: {:?}", r);
    }

    #[test]
    fn test_missing_token_trace() {
        let net = simple_net();
        let bm = NetBitmask64::from_petri_net(&net);
        let trace = make_trace(&["b"]); // skip a, jump straight to b
        let r = replay_trace(&bm, &trace);
        assert!(!r.is_perfect());
        assert!(r.missing > 0);
    }

    #[test]
    fn test_remaining_token_trace() {
        let net = simple_net();
        let bm = NetBitmask64::from_petri_net(&net);
        let trace = make_trace(&["a"]); // fire a but not b; token stuck in p1
        let r = replay_trace(&bm, &trace);
        assert!(!r.is_perfect());
    }

    #[test]
    fn test_classify_exact() {
        let results = vec![
            ReplayResult { missing: 0, remaining: 0, produced: 2, consumed: 2 }, // perfect
            ReplayResult { missing: 1, remaining: 0, produced: 2, consumed: 2 }, // imperfect
            ReplayResult { missing: 0, remaining: 0, produced: 2, consumed: 2 }, // perfect
        ];
        let cls = classify(&results, 2);
        assert_eq!(cls.iter().filter(|&&b| b).count(), 2);
        assert!(cls[0] && cls[2]);
        assert!(!cls[1]);
    }
}
