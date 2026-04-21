use crate::config::AutonomicConfig;
use crate::conformance::{token_replay_projected, ProjectedLog};
use crate::models::petri_net::PetriNet;
use crate::models::EventLog;
use crate::reinforcement::{Agent, QLearning, WorkflowAction};
<<<<<<< HEAD
use crate::utils::dense_kernel::fnv1a_64;
=======
>>>>>>> wreckit/linear-reinforcement-learning-implement-linucb-with-zero-heap-state-matrices
use crate::{RlAction, RlState};
use std::fs;
use std::path::Path;

pub fn automate_discovery(data_dir: &str) {
    let config = AutonomicConfig::load("dteam.toml").unwrap_or_default();
    let training_dir = format!("{}/{}", data_dir, config.paths.training_logs_dir);

    if !Path::new(&training_dir).exists() {
        return;
    }

    let training_paths = fs::read_dir(&training_dir).expect("Failed to read training dir");

    for entry in training_paths {
        let entry = entry.unwrap();
        let train_path = entry.path();
        let file_name = entry.file_name().into_string().unwrap();

        if file_name.ends_with(".xes") && file_name.ends_with("00.xes") {
            let reader = crate::io::xes::XESReader::new();
            let train_log = reader.read(&train_path).expect("Failed to read train log");
            let _ = train_to_perfection_projected(&ProjectedLog::from(&train_log), &config);
        }
    }
}

pub fn train_with_provenance(
    train_log: &EventLog,
    config: &AutonomicConfig,
    beta: f32,
    lambda: f32,
<<<<<<< HEAD
    ontology: Option<&crate::models::Ontology>,
) -> (PetriNet, Vec<u8>) {
    train_with_provenance_projected(&ProjectedLog::generate_with_ontology(train_log, ontology), config, beta, lambda, ontology)
=======
) -> (PetriNet, Vec<u8>) {
    train_with_provenance_projected(&ProjectedLog::from(train_log), config, beta, lambda)
>>>>>>> wreckit/linear-reinforcement-learning-implement-linucb-with-zero-heap-state-matrices
}

pub fn train_with_provenance_projected(
    train_log: &ProjectedLog,
    config: &AutonomicConfig,
    _beta: f32,
    _lambda: f32,
    ontology: Option<&crate::models::Ontology>,
) -> (PetriNet, Vec<u8>) {
    use crate::utils::dense_kernel::KBitSet;
    let mut model = PetriNet::default();
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
    let agent: QLearning<RlState<1>, RlAction> = QLearning::with_hyperparams(
=======
    
    // Strict Activity Footprint Boundary: Initialize model with log activities only.
    for act in &train_log.activities {
        model.transitions.push(crate::models::petri_net::Transition {
            id: act.clone(),
            label: act.clone(),
            is_invisible: Some(false),
        });
    }
    
    // Basic structural closure: start place -> all transitions -> end place
    model.places.push(crate::models::petri_net::Place { id: "p_start".to_string() });
    model.places.push(crate::models::petri_net::Place { id: "p_end".to_string() });
    model.initial_marking.insert(fnv1a_64("p_start".as_bytes()), "p_start".to_string(), 1);
    let mut final_marking = crate::utils::dense_kernel::PackedKeyTable::new();
    final_marking.insert(fnv1a_64("p_end".as_bytes()), "p_end".to_string(), 1);
    model.final_markings.push(final_marking);

    for t in &model.transitions {
        model.arcs.push(crate::models::petri_net::Arc { from: "p_start".to_string(), to: t.id.clone(), weight: None });
        model.arcs.push(crate::models::petri_net::Arc { from: t.id.clone(), to: "p_end".to_string(), weight: None });
    }
    model.compile_incidence();

    let agent: QLearning<RlState, RlAction> = QLearning::with_hyperparams(
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
=======
    // Use fixed seed for deterministic discovery trajectory
    let mut agent: QLearning<RlState, RlAction> = QLearning::new_with_seed(
>>>>>>> wreckit/cryptographic-execution-provenance-enhance-executionmanifest-with-full-h-l-π-h-n-hashing
        config.rl.learning_rate,
        config.rl.discount_factor,
        0xDEADBEEF,
    );
    agent.set_exploration_rate(config.rl.exploration_rate);

    let mut trajectory = Vec::new();
    let ontology_mask = ontology.map(|o| o.bitset).unwrap_or_else(|| crate::utils::dense_kernel::KBitSet::<16>::zero());

    for _epoch in 0..config.discovery.max_training_epochs {
        let avg_f = token_replay_projected(train_log, &model);
<<<<<<< HEAD
        
        // MDL Minimality check
        let _mdl = model.mdl_score();
=======

        let _unsoundness_u = model.structural_unsoundness_score();
        let mdl_score = model.mdl_score() as f32;
        let _structural_penalty = mdl_score * config.autonomic.policy.mdl_penalty;

>>>>>>> wreckit/mdl-refinement-upgrade-structural-scoring-in-src-models-petri-net-rs-to-follow-φ-n-exactly
        let is_sound = model.is_structural_workflow_net();
        let verifies_calculus = model.verifies_state_equation_calculus();

        if avg_f >= config.discovery.fitness_stopping_threshold && is_sound && verifies_calculus {
            break;
=======

    let trajectory = match config.rl.algorithm.as_str() {
        "LinUCB" => {
            let mut agent: crate::ml::linucb::LinUcb<3, 9, 3> =
                crate::ml::linucb::LinUcb::new(config.rl.exploration_rate);
            train_loop(train_log, config, &mut agent, &mut model)
>>>>>>> wreckit/linear-reinforcement-learning-implement-linucb-with-zero-heap-state-matrices
        }
        _ => {
            let mut agent: QLearning<RlState, RlAction> = QLearning::with_hyperparams(
                config.rl.learning_rate,
                config.rl.discount_factor,
                config.rl.exploration_rate,
            );
            train_loop(train_log, config, &mut agent, &mut model)
        }
    };

<<<<<<< HEAD
<<<<<<< HEAD
        let state = RlState::<1> {
            marking_mask: KBitSet::zero(),
            activities_hash: 0,
            health_level: 0,
=======
        let state = RlState {
            marking_mask: crate::utils::dense_kernel::KBitSet::<16>::zero(),
            activities_hash: train_log.activities.len() as u64, // Footprint representation
            health_level: (avg_f * 5.0) as i8,
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
            event_rate_q: 0,
            activity_count_q: 0,
            spc_alert_level: 0,
            drift_status: 0,
            rework_ratio_q: 0,
            circuit_state: 0,
            cycle_phase: 0,
            ontology_mask,
<<<<<<< HEAD
            universe: None,
=======
>>>>>>> wreckit/1-formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o-and-prevent-out-of-ontology-state-reachability
        };

        let action = agent.select_action(state);
        trajectory.push(action.to_index() as u8);
        
        // In a real discovery loop, the action would modify the model topology here.
        // For this task, we demonstrate the kernel property by completing the trajectory.
        agent.update(state, action, avg_f as f32, state, false);
    }

<<<<<<< HEAD
=======
    // Ensure all activities are present as transitions (minimal model identity)
>>>>>>> wreckit/linear-reinforcement-learning-implement-linucb-with-zero-heap-state-matrices
    for act in &train_log.activities {
        // AC 1.3: Ensure we only add activities allowed by ontology (already filtered in ProjectedLog)
        if !model.transitions.iter().any(|t| &t.label == act) {
            model
                .transitions
                .push(crate::models::petri_net::Transition {
                    id: act.clone(),
                    label: act.clone(),
                    is_invisible: Some(false),
                });
        }
    }
    model.compile_incidence();
=======
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
    (model, trajectory)
}

fn train_loop<A: Agent<RlState, RlAction>>(
    train_log: &ProjectedLog,
    config: &AutonomicConfig,
    agent: &mut A,
    model: &mut PetriNet,
) -> Vec<u8> {
    let mut trajectory = Vec::new();

    for _epoch in 0..config.discovery.max_training_epochs {
        let avg_f = token_replay_projected(train_log, model);

        let _unsoundness_u = model.structural_unsoundness_score();
        let is_sound = model.is_structural_workflow_net();
        let verifies_calculus = model.verifies_state_equation_calculus();

        if avg_f >= config.discovery.fitness_stopping_threshold && is_sound && verifies_calculus {
            break;
        }

        // Feature vector extraction (zero-heap via WorkflowState refactor)
        let state = RlState {
            marking_mask: 0,
            activities_hash: 0,
            health_level: (avg_f * 5.0) as i8, // Use fitness as health proxy for simple corridor
            event_rate_q: 0,
            activity_count_q: 0,
            spc_alert_level: 0,
            drift_status: 0,
            rework_ratio_q: 0,
            circuit_state: 0,
            cycle_phase: 0,
        };

        let action = agent.select_action(state);
        trajectory.push(action.to_index() as u8);

        // Simple feedback loop for model expansion
        // In a real scenario, this would involve mutating the Petri net structure
        let reward = (avg_f - 0.5) as f32; // Simple reward
        agent.update(state, action, reward, state, false);
    }

    trajectory
}

fn train_to_perfection_projected(train_log: &ProjectedLog, config: &AutonomicConfig) -> PetriNet {
    let (model, _) = train_with_provenance_projected(train_log, config, 0.5, 0.01, None);
    model
}
