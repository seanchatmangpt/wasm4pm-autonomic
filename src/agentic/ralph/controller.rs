use crate::dteam::orchestration::{Engine, EngineResult};
use crate::models::EventLog;
use anyhow::Result;
use std::fs;
use std::io::Write;
use tracing::info;

pub struct AutonomicController {
    ideas_path: String,
}

impl AutonomicController {
    pub fn new(ideas_path: &str) -> Self {
        Self {
            ideas_path: ideas_path.to_string(),
        }
    }

    pub fn evaluate_dogfood(&self, log: &EventLog) -> Result<()> {
        if cfg!(debug_assertions) {
            info!("\n--- Process Complete. Running Meta-Engine (Dogfooding) ---");
        }
        
        let engine = Engine::builder().build();
        let result = engine.run(log);

        if let EngineResult::Success(_net, manifest) = result {
            if cfg!(debug_assertions) {
                info!(
                    "  >> Meta-Process Analysis Success. Model Canonical Hash: {}",
                    manifest.model_canonical_hash
                );
            }
            if manifest.mdl_score > 0.0 {
                if cfg!(debug_assertions) {
                    info!("  >> dteam identifies structural optimization potential. Injecting self-optimization task...");
                }
                let mut file = fs::OpenOptions::new().append(true).open(&self.ideas_path)?;
                writeln!(
                    file,
                    "DDS-AUTO: Optimize Ralph Loop topology based on manifest hash {}",
                    manifest.model_canonical_hash
                )?;
            }
        }
        
        Ok(())
    }
}
