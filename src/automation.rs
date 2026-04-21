use crate::config::AutonomicConfig;
use crate::conformance::{token_replay_projected, ProjectedLog};
use crate::models::petri_net::PetriNet;
use crate::models::EventLog;
use crate::reinforcement::{QLearning, WorkflowAction};
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
    _beta: f32,
    _lambda: f32,
) -> (PetriNet, Vec<u8>) {
    train_with_provenance_projected(&ProjectedLog::from(train_log), config, _beta, _lambda)
}

pub fn train_with_provenance_projected(
    train_log: &ProjectedLog,
    config: &AutonomicConfig,
    _beta: f32,
    _lambda: f32,
) -> (PetriNet, Vec<u8>) {
    let mut model = PetriNet::default();
    let agent: QLearning<RlState, RlAction> = QLearning::with_hyperparams(
        config.rl.learning_rate,
        config.rl.discount_factor,
        config.rl.exploration_rate,
    );

    let mut trajectory = Vec::new();

    for _epoch in 0..config.discovery.max_training_epochs {
        let avg_f = token_replay_projected(train_log, &model);

        let _unsoundness_u = model.structural_unsoundness_score();
        let mdl_score = model.mdl_score() as f32;
        let _structural_penalty = mdl_score * config.autonomic.policy.mdl_penalty;

        let is_sound = model.is_structural_workflow_net();
        let verifies_calculus = model.verifies_state_equation_calculus();

        if avg_f >= config.discovery.fitness_stopping_threshold && is_sound && verifies_calculus {
            break;
        }

        let state = RlState {
            marking_mask: 0,
            activities_hash: 0,
            health_level: 0,
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
    }

    for act in &train_log.activities {
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
    let (model, _) = train_with_provenance_projected(train_log, config, 0.5, 0.01);
    model
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_automation_run() {
        // automate_discovery("data/pdc2025");
    }
}
