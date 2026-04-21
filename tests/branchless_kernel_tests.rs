#[cfg(test)]
mod tests {
    use dteam::dteam::kernel::branchless::apply_branchless_update;
    use dteam::models::petri_net::{PetriNet, Place, Transition, Arc};

    #[test]
    fn test_branchless_transition_update() {
        let mut net = PetriNet::default();
        net.places.push(Place { id: "p1".to_string() });
        net.places.push(Place { id: "p2".to_string() });
        net.transitions.push(Transition { id: "t1".to_string(), label: "A".to_string(), is_invisible: None });
        
        // p1 -> t1 (weight 1), t1 -> p2 (weight 1)
        net.arcs.push(Arc { from: "p1".to_string(), to: "t1".to_string(), weight: Some(1) });
        net.arcs.push(Arc { from: "t1".to_string(), to: "p2".to_string(), weight: Some(1) });
        
        let incidence = net.incidence_matrix();

        // Initially p1 has token (mask 0b01)
        let initial_marking = 0b01u64;
        let new_marking = apply_branchless_update(initial_marking, 0, &incidence);
        
        // p2 should have token (mask 0b10)
        assert_eq!(new_marking, 0b10u64);
    }
}
