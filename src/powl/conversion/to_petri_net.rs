use crate::models::petri_net::{Arc, PetriNet, Place, Transition};
use crate::powl::core::{PowlNode, PowlOperator};
use bcinr_core::dense_kernel::{fnv1a_64, PackedKeyTable};

pub fn powl_to_wf_net(node: &PowlNode) -> PetriNet {
    let mut net = PetriNet::default();
    let mut id_counter = 0;

    // Helper to generate unique IDs
    let mut next_id = || -> String {
        id_counter += 1;
        format!("n{}", id_counter)
    };

    let source = Place {
        id: "source".to_string(),
    };
    let sink = Place {
        id: "sink".to_string(),
    };
    net.places.push(source.clone());
    net.places.push(sink.clone());

    fn convert_node(
        node: &PowlNode,
        net: &mut PetriNet,
        entry_place: &Place,
        exit_place: &Place,
        next_id: &mut impl FnMut() -> String,
    ) {
        match node {
            PowlNode::Transition { label, id } => {
                let t_id = format!("t{}", id);
                let t = Transition {
                    id: t_id.clone(),
                    label: label.clone().unwrap_or_else(|| format!("Activity_{}", id)),
                    is_invisible: Some(false),
                };
                net.transitions.push(t);
                net.arcs.push(Arc {
                    from: entry_place.id.clone(),
                    to: t_id.clone(),
                    weight: Some(1),
                });
                net.arcs.push(Arc {
                    from: t_id,
                    to: exit_place.id.clone(),
                    weight: Some(1),
                });
            }
            PowlNode::Operator { operator, children } => {
                if children.is_empty() {
                    // Invisible transition
                    let t_id = next_id();
                    net.transitions.push(Transition {
                        id: t_id.clone(),
                        label: "tau".to_string(),
                        is_invisible: Some(true),
                    });
                    net.arcs.push(Arc {
                        from: entry_place.id.clone(),
                        to: t_id.clone(),
                        weight: Some(1),
                    });
                    net.arcs.push(Arc {
                        from: t_id,
                        to: exit_place.id.clone(),
                        weight: Some(1),
                    });
                    return;
                }

                match operator {
                    PowlOperator::SEQUENCE => {
                        let mut current_entry = entry_place.clone();
                        for (i, child) in children.iter().enumerate() {
                            let current_exit = if i == children.len() - 1 {
                                exit_place.clone()
                            } else {
                                let p = Place { id: next_id() };
                                net.places.push(p.clone());
                                p
                            };
                            convert_node(child, net, &current_entry, &current_exit, next_id);
                            current_entry = current_exit;
                        }
                    }
                    PowlOperator::XOR => {
                        for child in children {
                            convert_node(child, net, entry_place, exit_place, next_id);
                        }
                    }
                    PowlOperator::PARALLEL | PowlOperator::AND => {
                        let split_t_id = next_id();
                        let join_t_id = next_id();

                        net.transitions.push(Transition {
                            id: split_t_id.clone(),
                            label: "tau_split".to_string(),
                            is_invisible: Some(true),
                        });
                        net.transitions.push(Transition {
                            id: join_t_id.clone(),
                            label: "tau_join".to_string(),
                            is_invisible: Some(true),
                        });

                        net.arcs.push(Arc {
                            from: entry_place.id.clone(),
                            to: split_t_id.clone(),
                            weight: Some(1),
                        });
                        net.arcs.push(Arc {
                            from: join_t_id.clone(),
                            to: exit_place.id.clone(),
                            weight: Some(1),
                        });

                        for child in children {
                            let p_in = Place { id: next_id() };
                            let p_out = Place { id: next_id() };
                            net.places.push(p_in.clone());
                            net.places.push(p_out.clone());

                            net.arcs.push(Arc {
                                from: split_t_id.clone(),
                                to: p_in.id.clone(),
                                weight: Some(1),
                            });
                            net.arcs.push(Arc {
                                from: p_out.id.clone(),
                                to: join_t_id.clone(),
                                weight: Some(1),
                            });

                            convert_node(child, net, &p_in, &p_out, next_id);
                        }
                    }
                    PowlOperator::LOOP => {
                        if children.len() >= 2 {
                            let do_node = &children[0];
                            let redo_node = &children[1];

                            let p_loop_start = Place { id: next_id() };
                            let p_loop_end = Place { id: next_id() };
                            net.places.push(p_loop_start.clone());
                            net.places.push(p_loop_end.clone());

                            // entry -> tau -> p_loop_start
                            let t_entry = next_id();
                            net.transitions.push(Transition {
                                id: t_entry.clone(),
                                label: "tau_loop_entry".to_string(),
                                is_invisible: Some(true),
                            });
                            net.arcs.push(Arc {
                                from: entry_place.id.clone(),
                                to: t_entry.clone(),
                                weight: Some(1),
                            });
                            net.arcs.push(Arc {
                                from: t_entry,
                                to: p_loop_start.id.clone(),
                                weight: Some(1),
                            });

                            convert_node(do_node, net, &p_loop_start, &p_loop_end, next_id);

                            // p_loop_end -> tau -> exit
                            let t_exit = next_id();
                            net.transitions.push(Transition {
                                id: t_exit.clone(),
                                label: "tau_loop_exit".to_string(),
                                is_invisible: Some(true),
                            });
                            net.arcs.push(Arc {
                                from: p_loop_end.id.clone(),
                                to: t_exit.clone(),
                                weight: Some(1),
                            });
                            net.arcs.push(Arc {
                                from: t_exit,
                                to: exit_place.id.clone(),
                                weight: Some(1),
                            });

                            // p_loop_end -> redo -> p_loop_start
                            let p_redo_start = Place { id: next_id() };
                            let p_redo_end = Place { id: next_id() };
                            net.places.push(p_redo_start.clone());
                            net.places.push(p_redo_end.clone());

                            let t_redo_entry = next_id();
                            net.transitions.push(Transition {
                                id: t_redo_entry.clone(),
                                label: "tau_redo_entry".to_string(),
                                is_invisible: Some(true),
                            });
                            net.arcs.push(Arc {
                                from: p_loop_end.id.clone(),
                                to: t_redo_entry.clone(),
                                weight: Some(1),
                            });
                            net.arcs.push(Arc {
                                from: t_redo_entry,
                                to: p_redo_start.id.clone(),
                                weight: Some(1),
                            });

                            convert_node(redo_node, net, &p_redo_start, &p_redo_end, next_id);

                            let t_redo_exit = next_id();
                            net.transitions.push(Transition {
                                id: t_redo_exit.clone(),
                                label: "tau_redo_exit".to_string(),
                                is_invisible: Some(true),
                            });
                            net.arcs.push(Arc {
                                from: p_redo_end.id.clone(),
                                to: t_redo_exit.clone(),
                                weight: Some(1),
                            });
                            net.arcs.push(Arc {
                                from: t_redo_exit,
                                to: p_loop_start.id.clone(),
                                weight: Some(1),
                            });
                        } else if children.len() == 1 {
                            // Self loop.
                            convert_node(&children[0], net, entry_place, exit_place, next_id);
                        }
                    }
                    _ => {
                        // Unhandled operators mapped to invisible for now
                        let t_id = next_id();
                        net.transitions.push(Transition {
                            id: t_id.clone(),
                            label: "tau".to_string(),
                            is_invisible: Some(true),
                        });
                        net.arcs.push(Arc {
                            from: entry_place.id.clone(),
                            to: t_id.clone(),
                            weight: Some(1),
                        });
                        net.arcs.push(Arc {
                            from: t_id,
                            to: exit_place.id.clone(),
                            weight: Some(1),
                        });
                    }
                }
            }
            PowlNode::PartialOrder { nodes, edges: _ } => {
                let split_t_id = next_id();
                let join_t_id = next_id();

                net.transitions.push(Transition {
                    id: split_t_id.clone(),
                    label: "tau_po_split".to_string(),
                    is_invisible: Some(true),
                });
                net.transitions.push(Transition {
                    id: join_t_id.clone(),
                    label: "tau_po_join".to_string(),
                    is_invisible: Some(true),
                });

                net.arcs.push(Arc {
                    from: entry_place.id.clone(),
                    to: split_t_id.clone(),
                    weight: Some(1),
                });
                net.arcs.push(Arc {
                    from: join_t_id.clone(),
                    to: exit_place.id.clone(),
                    weight: Some(1),
                });

                for child in nodes {
                    let p_in = Place { id: next_id() };
                    let p_out = Place { id: next_id() };
                    net.places.push(p_in.clone());
                    net.places.push(p_out.clone());

                    net.arcs.push(Arc {
                        from: split_t_id.clone(),
                        to: p_in.id.clone(),
                        weight: Some(1),
                    });
                    net.arcs.push(Arc {
                        from: p_out.id.clone(),
                        to: join_t_id.clone(),
                        weight: Some(1),
                    });

                    convert_node(child, net, &p_in, &p_out, next_id);
                }
            }
            PowlNode::ChoiceGraph {
                nodes,
                edges,
                start_nodes,
                end_nodes,
                empty_path,
            } => {
                let mut node_entry_places = Vec::with_capacity(nodes.len());
                let mut node_exit_places = Vec::with_capacity(nodes.len());

                // 1. Create entry/exit places for every node in the ChoiceGraph
                for _ in 0..nodes.len() {
                    let entry = Place { id: next_id() };
                    let exit = Place { id: next_id() };
                    net.places.push(entry.clone());
                    net.places.push(exit.clone());
                    node_entry_places.push(entry);
                    node_exit_places.push(exit);
                }

                // 2. Recursively convert each node
                for (i, child) in nodes.iter().enumerate() {
                    convert_node(
                        child,
                        net,
                        &node_entry_places[i],
                        &node_exit_places[i],
                        next_id,
                    );
                }

                // 3. Connect start nodes from entry_place
                for &start_idx in start_nodes {
                    if start_idx < nodes.len() {
                        let t_id = next_id();
                        net.transitions.push(Transition {
                            id: t_id.clone(),
                            label: "tau_cg_start".to_string(),
                            is_invisible: Some(true),
                        });
                        net.arcs.push(Arc {
                            from: entry_place.id.clone(),
                            to: t_id.clone(),
                            weight: Some(1),
                        });
                        net.arcs.push(Arc {
                            from: t_id,
                            to: node_entry_places[start_idx].id.clone(),
                            weight: Some(1),
                        });
                    }
                }

                // 4. Connect end nodes to exit_place
                for &end_idx in end_nodes {
                    if end_idx < nodes.len() {
                        let t_id = next_id();
                        net.transitions.push(Transition {
                            id: t_id.clone(),
                            label: "tau_cg_end".to_string(),
                            is_invisible: Some(true),
                        });
                        net.arcs.push(Arc {
                            from: node_exit_places[end_idx].id.clone(),
                            to: t_id.clone(),
                            weight: Some(1),
                        });
                        net.arcs.push(Arc {
                            from: t_id,
                            to: exit_place.id.clone(),
                            weight: Some(1),
                        });
                    }
                }

                // 5. Connect internal edges using places as buffers
                for &(src_idx, tgt_idx) in edges {
                    if src_idx < nodes.len() && tgt_idx < nodes.len() {
                        let t_id = next_id();
                        net.transitions.push(Transition {
                            id: t_id.clone(),
                            label: "tau_cg_edge".to_string(),
                            is_invisible: Some(true),
                        });
                        net.arcs.push(Arc {
                            from: node_exit_places[src_idx].id.clone(),
                            to: t_id.clone(),
                            weight: Some(1),
                        });
                        net.arcs.push(Arc {
                            from: t_id,
                            to: node_entry_places[tgt_idx].id.clone(),
                            weight: Some(1),
                        });
                    }
                }

                // 6. Handle empty path
                if *empty_path {
                    let t_id = next_id();
                    net.transitions.push(Transition {
                        id: t_id.clone(),
                        label: "tau_cg_empty".to_string(),
                        is_invisible: Some(true),
                    });
                    net.arcs.push(Arc {
                        from: entry_place.id.clone(),
                        to: t_id.clone(),
                        weight: Some(1),
                    });
                    net.arcs.push(Arc {
                        from: t_id,
                        to: exit_place.id.clone(),
                        weight: Some(1),
                    });
                }
            }
        }
    }

    convert_node(node, &mut net, &source, &sink, &mut next_id);

    // Set initial marking to the source place
    let mut initial_marking = PackedKeyTable::new();
    initial_marking.insert(fnv1a_64(source.id.as_bytes()), source.id.clone(), 1);
    net.initial_marking = initial_marking;

    let mut fm = PackedKeyTable::new();
    fm.insert(fnv1a_64(sink.id.as_bytes()), sink.id.clone(), 1);
    net.final_markings.push(fm);

    net
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_powl_to_wf_net() {
        let root = PowlNode::Operator {
            operator: PowlOperator::SEQUENCE,
            children: vec![
                PowlNode::Transition {
                    label: Some("A".to_string()),
                    id: 1,
                },
                PowlNode::Operator {
                    operator: PowlOperator::XOR,
                    children: vec![
                        PowlNode::Transition {
                            label: Some("B".to_string()),
                            id: 2,
                        },
                        PowlNode::Operator {
                            operator: PowlOperator::PARALLEL,
                            children: vec![
                                PowlNode::Transition {
                                    label: Some("C".to_string()),
                                    id: 3,
                                },
                                PowlNode::Transition {
                                    label: Some("D".to_string()),
                                    id: 4,
                                },
                            ],
                        },
                    ],
                },
                PowlNode::Transition {
                    label: Some("E".to_string()),
                    id: 5,
                },
            ],
        };

        let net = powl_to_wf_net(&root);

        // Assert it satisfies Workflow Net structural equation calculus
        assert!(net.is_structural_workflow_net());
        assert!(net.verifies_state_equation_calculus());
    }
}
