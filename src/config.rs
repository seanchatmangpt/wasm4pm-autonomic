use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomicConfig {
    pub meta: MetaConfig,
    pub kernel: KernelConfig,
    pub autonomic: AutonomicSystemConfig,
    pub rl: RlConfig,
    pub discovery: DiscoveryConfig,
    pub paths: PathConfig,
    pub wasm: WasmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaConfig {
    pub version: String,
    pub environment: String,
    pub identity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfig {
    pub tier: String,
    pub alignment: usize,
    pub determinism: String,
    pub allocation_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomicSystemConfig {
    pub mode: String,
    pub sampling_rate: u64,
    pub integrity_hash: String,
    pub guards: GuardConfig,
    pub policy: PolicyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConfig {
    pub risk_threshold: String,
    pub min_health_threshold: f32,
    pub max_cycle_latency_ms: u64,
    pub repair_authority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub profile: String,
    pub mdl_penalty: f32,
    pub human_weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlConfig {
    pub algorithm: String,
    pub learning_rate: f32,
    pub discount_factor: f32,
    pub exploration_rate: f32,
    pub exploration_decay: f32,
    pub reward_weights: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub max_training_epochs: usize,
    pub fitness_stopping_threshold: f64,
    pub strategy: String,
    pub drift_window: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    pub training_logs_dir: String,
    pub test_logs_dir: String,
    pub ground_truth_dir: String,
    pub artifacts_dir: String,
    pub manifest_bus_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmConfig {
    pub batch_size: usize,
    pub max_pages: usize,
}

impl Default for AutonomicConfig {
    fn default() -> Self {
        let mut reward_weights = HashMap::new();
        reward_weights.insert("fitness".to_string(), 0.6);
        reward_weights.insert("soundness".to_string(), 0.2);
        reward_weights.insert("simplicity".to_string(), 0.1);
        reward_weights.insert("latency".to_string(), 0.1);

        Self {
            meta: MetaConfig {
                version: "2026.04.18".to_string(),
                environment: "autonomous".to_string(),
                identity: "dteam-alpha-01".to_string(),
            },
            kernel: KernelConfig {
                tier: "K256".to_string(),
                alignment: 8,
                determinism: "strict".to_string(),
                allocation_policy: "zero_heap".to_string(),
            },
            autonomic: AutonomicSystemConfig {
                mode: "guarded".to_string(),
                sampling_rate: 100,
                integrity_hash: "fnv1a_64".to_string(),
                guards: GuardConfig {
                    risk_threshold: "Low".to_string(),
                    min_health_threshold: 0.7,
                    max_cycle_latency_ms: 50,
                    repair_authority: "senior_engineer".to_string(),
                },
                policy: PolicyConfig {
                    profile: "strict_conformance".to_string(),
                    mdl_penalty: 0.05,
                    human_weight: 0.8,
                },
            },
            rl: RlConfig {
                algorithm: "DoubleQLearning".to_string(),
                learning_rate: 0.08,
                discount_factor: 0.95,
                exploration_rate: 0.2,
                exploration_decay: 0.999,
                reward_weights,
            },
            discovery: DiscoveryConfig {
                max_training_epochs: 100,
                fitness_stopping_threshold: 0.995,
                strategy: "incremental".to_string(),
                drift_window: 1000,
            },
            paths: PathConfig {
                training_logs_dir: "data/pdc2025/training_logs".to_string(),
                test_logs_dir: "data/pdc2025/test_logs".to_string(),
                ground_truth_dir: "data/pdc2025/ground_truth".to_string(),
                artifacts_dir: "artifacts".to_string(),
                manifest_bus_path: "tmp/dmanifest_bus".to_string(),
            },
            wasm: WasmConfig {
                batch_size: 10,
                max_pages: 16,
            },
        }
    }
}

impl AutonomicConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        if !path.as_ref().exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
