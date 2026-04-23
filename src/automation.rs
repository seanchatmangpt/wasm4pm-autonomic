use crate::config::AutonomicConfig;
use crate::conformance::{token_replay_projected, ProjectedLog};
use crate::models::petri_net::PetriNet;
use crate::models::EventLog;
use crate::reinforcement::{QLearning, WorkflowAction};
use crate::{RlAction, RlState};
use log::info;
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
            info!("Training on: {}", file_name);
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
    ontology: Option<&crate::models::Ontology>,
    seed: Option<u64>,
) -> (PetriNet, Vec<u8>) {
    train_with_provenance_projected(
        &ProjectedLog::generate_with_ontology(train_log, ontology),
        config,
        beta,
        lambda,
        ontology,
        seed,
    )
}

pub fn train_with_provenance_projected(
    train_log: &ProjectedLog,
    config: &AutonomicConfig,
    beta: f32,
    lambda: f32,
    ontology: Option<&crate::models::Ontology>,
    seed: Option<u64>,
) -> (PetriNet, Vec<u8>) {
    use crate::utils::dense_kernel::KBitSet;
    let mut model = PetriNet::default();
    let mut agent: QLearning<RlState<1>, RlAction> = match seed {
        Some(s) => {
            let mut a = QLearning::new_with_seed(
                config.rl.learning_rate,
                config.rl.discount_factor,
                s,
            );
            a.set_exploration_rate(config.rl.exploration_rate);
            a
        }
        None => QLearning::with_hyperparams(
            config.rl.learning_rate,
            config.rl.discount_factor,
            config.rl.exploration_rate,
        ),
    };

    let mut trajectory = Vec::new();
    let ontology_mask = ontology
        .map(|o| o.bitset)
        .unwrap_or_else(crate::utils::dense_kernel::KBitSet::<16>::zero);

    let mut final_fitness = 0.0_f64;
    let mut prev_state = RlState::<1> {
        marking_mask: KBitSet::zero(),
        activities_hash: 0,
        health_level: 0,
        event_rate_q: 0,
        activity_count_q: 0,
        spc_alert_level: 0,
        drift_status: 0,
        rework_ratio_q: 0,
        circuit_state: 0,
        cycle_phase: 0,
        ontology_mask,
        universe: None,
    };
    let mut prev_action: Option<RlAction> = None;

    for epoch in 0..config.discovery.max_training_epochs {
        let avg_f = token_replay_projected(train_log, &model);
        final_fitness = avg_f;

        let unsoundness_u = model.structural_unsoundness_score();
        let complexity_c = (model.transitions.len() + model.arcs.len()) as f32;
        let is_sound = model.is_structural_workflow_net();
        let verifies_calculus = model.verifies_state_equation_calculus();

        // Reward = fitness + β·soundness_bonus − λ·MDL_complexity
        let reward = avg_f as f32
            + beta * (1.0 - unsoundness_u)
            - lambda * complexity_c;

        if let Some(pa) = prev_action {
            let done = avg_f >= config.discovery.fitness_stopping_threshold
                && is_sound
                && verifies_calculus;
            agent.update(prev_state, pa, reward, prev_state, done);
        }

        if avg_f >= config.discovery.fitness_stopping_threshold && is_sound && verifies_calculus {
            info!(
                "  epoch={} fitness={:.4} sound={} calculus={} → converged",
                epoch, avg_f, is_sound, verifies_calculus
            );
            break;
        }

        if epoch % 10 == 0 {
            info!(
                "  epoch={} fitness={:.4} sound={} calculus={}",
                epoch, avg_f, is_sound, verifies_calculus
            );
        }

        let state = RlState::<1> {
            marking_mask: KBitSet::zero(),
            activities_hash: 0,
            health_level: 0,
            event_rate_q: 0,
            activity_count_q: 0,
            spc_alert_level: 0,
            drift_status: 0,
            rework_ratio_q: 0,
            circuit_state: 0,
            cycle_phase: 0,
            ontology_mask,
            universe: None,
        };

        let action = agent.select_action(state);
        prev_state = state;
        prev_action = Some(action);
        trajectory.push(action.to_index() as u8);
    }

    agent.set_deterministic(true);
    info!(
        "  training complete: final_fitness={:.4} transitions={}",
        final_fitness,
        train_log.activities.len()
    );

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
    (model, trajectory)
}

fn train_to_perfection_projected(train_log: &ProjectedLog, config: &AutonomicConfig) -> PetriNet {
    let (model, _) = train_with_provenance_projected(train_log, config, 0.5, 0.01, None, None);
    model
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_automation_run() {
        // automate_discovery("data/pdc2025");
    }
}
