use dteam::models::petri_net::{PetriNet, Place, Transition, Arc};

#[test]
fn test_adversarial_disconnected_islands() {
    let mut net = PetriNet::default();
    // Island 1 (Sound WF-net)
    net.places.push(Place { id: "p1".to_string() });
    net.places.push(Place { id: "p2".to_string() });
    net.transitions.push(Transition { id: "t1".to_string(), label: "A".to_string(), is_invisible: None });
    net.arcs.push(Arc { from: "p1".to_string(), to: "t1".to_string(), weight: None });
    net.arcs.push(Arc { from: "t1".to_string(), to: "p2".to_string(), weight: None });
    
    // Island 2 (Disconnected)
    net.places.push(Place { id: "p3".to_string() });
    net.places.push(Place { id: "p4".to_string() });
    net.transitions.push(Transition { id: "t2".to_string(), label: "B".to_string(), is_invisible: None });
    net.arcs.push(Arc { from: "p3".to_string(), to: "t2".to_string(), weight: None });
    net.arcs.push(Arc { from: "t2".to_string(), to: "p4".to_string(), weight: None });

    net.compile_incidence();
    // Should fail because multiple sources (p1, p3) and sinks (p2, p4)
    assert!(!net.is_sound());
}

#[test]
fn test_adversarial_multiple_sources() {
    let mut net = PetriNet::default();
    net.places.push(Place { id: "p1".to_string() });
    net.places.push(Place { id: "p2".to_string() });
    net.places.push(Place { id: "p3".to_string() });
    net.transitions.push(Transition { id: "t1".to_string(), label: "A".to_string(), is_invisible: None });
    
    net.arcs.push(Arc { from: "p1".to_string(), to: "t1".to_string(), weight: None });
    net.arcs.push(Arc { from: "p2".to_string(), to: "t1".to_string(), weight: None });
    net.arcs.push(Arc { from: "t1".to_string(), to: "p3".to_string(), weight: None });

    net.compile_incidence();
    assert!(!net.is_sound());
}

#[test]
fn test_adversarial_sink_hole_cycle() {
    let mut net = PetriNet::default();
    net.places.push(Place { id: "p1".to_string() });
    net.places.push(Place { id: "p2".to_string() });
    net.places.push(Place { id: "p3".to_string() }); // Sink hole
    net.transitions.push(Transition { id: "t1".to_string(), label: "A".to_string(), is_invisible: None });
    net.transitions.push(Transition { id: "t2".to_string(), label: "B".to_string(), is_invisible: None });
    
    net.arcs.push(Arc { from: "p1".to_string(), to: "t1".to_string(), weight: None });
    net.arcs.push(Arc { from: "t1".to_string(), to: "p2".to_string(), weight: None });
    
    // Cycle that doesn't reach p2 (the sink)
    net.arcs.push(Arc { from: "p2".to_string(), to: "t2".to_string(), weight: None });
    net.arcs.push(Arc { from: "t2".to_string(), to: "p3".to_string(), weight: None });
    net.arcs.push(Arc { from: "p3".to_string(), to: "t2".to_string(), weight: None });

    net.compile_incidence();
    // p3 is a source of a cycle but has no path to sink p2? 
    // Wait, in this case p2 has an output arc to t2, so it's not a sink anymore.
    // p3 has no output arc, so p3 IS the sink.
    // But p1 is the source. 
    // Is it sound? Let's check.
    // Every node must be on a path from p1 to p3.
    // p2 -> t2 -> p3 (Yes)
    // t2 -> p3 (Yes)
    // p3 is sink.
    // Wait, t2 is on a cycle p3 -> t2 -> p3.
    assert!(!net.is_sound()); // Should fail connectivity or structural checks
}

#[test]
fn test_adversarial_dead_transition() {
    let mut net = PetriNet::default();
    net.places.push(Place { id: "p1".to_string() });
    net.places.push(Place { id: "p2".to_string() });
    net.transitions.push(Transition { id: "t1".to_string(), label: "A".to_string(), is_invisible: None });
    net.transitions.push(Transition { id: "t2".to_string(), label: "Dead".to_string(), is_invisible: None });
    
    net.arcs.push(Arc { from: "p1".to_string(), to: "t1".to_string(), weight: None });
    net.arcs.push(Arc { from: "t1".to_string(), to: "p2".to_string(), weight: None });
    
    // t2 is not connected to anything
    net.compile_incidence();
    assert!(!net.is_sound());
}
