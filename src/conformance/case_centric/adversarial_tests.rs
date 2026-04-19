#[cfg(test)]
mod tests {
    use crate::models::{EventLog, Trace, Event, Attribute, AttributeValue};
    use crate::models::petri_net::{PetriNet, Place, Transition, Arc};
    use crate::conformance::case_centric::token_based_replay::apply_token_based_replay;
    use crate::utils::dense_kernel::{PackedKeyTable, fnv1a_64};

    #[test]
    fn test_adversarial_replay_missing_tokens() {
        // Setup a simple net: P1 -> T1 -> P2
        let mut net = PetriNet::default();
        net.places.push(Place { id: "p1".to_string() });
        net.places.push(Place { id: "p2".to_string() });
        net.transitions.push(Transition { id: "t1".to_string(), label: "a".to_string(), is_invisible: Some(false) });
        net.arcs.push(Arc { from: "p1".to_string(), to: "t1".to_string(), weight: Some(2) }); // Requires 2 tokens
        net.arcs.push(Arc { from: "t1".to_string(), to: "p2".to_string(), weight: Some(1) });
        
        let mut initial = PackedKeyTable::new();
        initial.insert(fnv1a_64(b"p1"), "p1".to_string(), 1); // Only 1 token, should fail/missing
        net.initial_marking = initial;

        let mut log = EventLog::new();
        let mut trace = Trace::new("case_1".to_string());
        trace.events.push(Event { attributes: vec![Attribute { key: "concept:name".to_string(), value: AttributeValue::String("a".to_string()) }] });
        log.add_trace(trace);

        let result = apply_token_based_replay(&net, &log);
        
        // Assert: We had 1 token, needed 2, should have 1 missing
        assert_eq!(result.missing, 1);
        assert_eq!(result.produced, 0); // t1 should not fire
    }

    #[test]
    fn test_adversarial_replay_overflow() {
        // Setup a net that fires, but consumes too much
        let mut net = PetriNet::default();
        net.places.push(Place { id: "p1".to_string() });
        net.transitions.push(Transition { id: "t1".to_string(), label: "a".to_string(), is_invisible: Some(false) });
        net.arcs.push(Arc { from: "p1".to_string(), to: "t1".to_string(), weight: Some(1) });
        
        let mut initial = PackedKeyTable::new();
        initial.insert(fnv1a_64(b"p1"), "p1".to_string(), 10);
        net.initial_marking = initial;

        let mut log = EventLog::new();
        let mut trace = Trace::new("case_1".to_string());
        trace.events.push(Event { attributes: vec![Attribute { key: "concept:name".to_string(), value: AttributeValue::String("a".to_string()) }] });
        log.add_trace(trace);

        let result = apply_token_based_replay(&net, &log);
        
        // Assert: Fired, 1 consumed, 9 remaining
        assert_eq!(result.consumed, 1);
        assert_eq!(result.remaining, 9);
    }
}
