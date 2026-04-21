pub mod controller;
pub mod execution;
pub mod patterns;
pub mod phase;
pub mod router;
pub mod verifier;
pub mod workspace;

pub use controller::AutonomicController;
pub use execution::ExecutionEngine;
pub use phase::{GeminiPhaseRunner, PhaseRunner};
pub use router::{AgentRouter, KeywordRouter};
pub use verifier::{CargoVerifier, DoDVerifier};
pub use workspace::{GitWorktreeManager, WorkspaceManager};
