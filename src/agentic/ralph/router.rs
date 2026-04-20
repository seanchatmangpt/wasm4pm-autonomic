pub trait AgentRouter {
    fn route(&self, idea: &str) -> Option<&'static str>;
}

#[derive(Default)]
pub struct KeywordRouter;

impl AgentRouter for KeywordRouter {
    fn route(&self, idea: &str) -> Option<&'static str> {
        let idea_lower = idea.to_lowercase();
        if idea_lower.contains("q-table") || idea_lower.contains("rl ") || idea_lower.contains("sarsa") || idea_lower.contains("reinforcement") {
            Some("@richard_sutton")
        } else if idea_lower.contains("wf-net") || idea_lower.contains("soundness") || idea_lower.contains("deadlock") || idea_lower.contains("liveness") {
            Some("@dr_wil_van_der_aalst")
        } else if idea_lower.contains("replay") || idea_lower.contains("conformance") || idea_lower.contains("token") || idea_lower.contains("zero-heap") || idea_lower.contains("branchless") {
            Some("@carl_adam_petri")
        } else if idea_lower.contains("autonomic") || idea_lower.contains("discovery") || idea_lower.contains("loop") {
            Some("@arthur_ter_hofstede")
        } else {
            None
        }
    }
}
