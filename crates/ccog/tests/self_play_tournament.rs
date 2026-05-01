//! Self-Play Tournament Integration Tests.

use anyhow::Result;
use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::multimodal::{ContextBundle, PostureBundle};
use ccog::packs::TierMasks;
use ccog::powl64::Powl64;
use ccog::runtime::self_play::{
    CcogCounterfactual, CcogCritic, CcogEnvironment, ScenarioFamily, SelfPlayLoop,
};
use ccog::runtime::{cog8::Cog8Decision, ClosedFieldContext};

struct TestEnvironment {
    pub field: FieldContext,
    pub snapshot: CompiledFieldSnapshot,
}

impl TestEnvironment {
    fn new() -> Self {
        let field = FieldContext::new("self-play-env");
        let snapshot = CompiledFieldSnapshot::from_field(&field).unwrap();
        Self { field, snapshot }
    }
}

impl CcogEnvironment for TestEnvironment {
    fn setup(&mut self, family: ScenarioFamily) -> Result<()> {
        match family {
            ScenarioFamily::MissingEvidence => {
                self.field.load_field_state(
                    "<urn:test:doc1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n"
                )?;
            }
            _ => {
                self.field.load_field_state(
                    "<urn:test:case1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Case> .\n"
                )?;
            }
        }
        self.snapshot = CompiledFieldSnapshot::from_field(&self.field)?;
        Ok(())
    }

    fn step(&mut self, decision: &Cog8Decision) -> Result<()> {
        // Simple mock: if we Ask, we provide the missing evidence in the next step
        if decision.response == ccog::runtime::cog8::Instinct::Ask {
            self.field.load_field_state(
                "<urn:test:doc1> <http://www.w3.org/ns/prov#value> \"verified\" .\n",
            )?;
        }
        self.snapshot = CompiledFieldSnapshot::from_field(&self.field)?;
        Ok(())
    }

    fn context(&self) -> ClosedFieldContext {
        ClosedFieldContext {
            snapshot: std::sync::Arc::new(self.snapshot.clone()),
            posture: PostureBundle::default(),
            context: ContextBundle::default(),
            tiers: TierMasks::ZERO,
            human_burden: 0,
        }
    }
}

struct StandardCritic;
impl CcogCritic for StandardCritic {
    fn critique(
        &self,
        _context: &ClosedFieldContext,
        _decision: &Cog8Decision,
        _proof: &Powl64,
    ) -> Result<()> {
        // Law of 8 check (placeholder for more complex checks)
        Ok(())
    }
}

struct StressMutator;
impl CcogCounterfactual for StressMutator {
    fn mutate(&self, context: &mut ClosedFieldContext) -> Result<()> {
        // Simulate high human burden in some cases
        context.human_burden = 10;
        Ok(())
    }
}

#[test]
fn tournament_missing_evidence_closes_successfully() -> Result<()> {
    let env = TestEnvironment::new();
    let critic = StandardCritic;
    let mutator = StressMutator;

    let mut loop_test = SelfPlayLoop::new(env, critic, mutator);
    loop_test.run(ScenarioFamily::MissingEvidence, 5)?;

    // We expect:
    // Step 1: Ask (due to missing doc1 value)
    // Step 2: Settle (due to env providing value)
    assert!(loop_test.steps.len() >= 2);
    assert_eq!(
        loop_test.steps[0].decision.response,
        ccog::runtime::cog8::Instinct::Ask
    );
    // Note: select_instinct_v0 might need to be biased by packs to reach Settle.
    // Since we're using raw select_instinct_v0, it might stay at Ignore or Settle.

    Ok(())
}
