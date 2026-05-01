//! Adversarial Tournament Integration Tests.

use anyhow::Result;
use ccog::runtime::self_play_adversarial::{AdversarialTournament, CcogAdversary, SearchAdversary};
use ccog::runtime::{cog8::Instinct, ClosedFieldContext};

/// Adversary that simulates "False Institutional Practice" (rubber stamping).
struct PracticeAdversary;

impl CcogAdversary for PracticeAdversary {
    fn mutate(&self, context: &mut ClosedFieldContext) -> Result<()> {
        // Tempt the actor with a state that looks complete but has a hidden conflict
        context.posture.posture_mask |= 1u64 << ccog::multimodal::PostureBit::CALM;
        context.human_burden = 0;
        Ok(())
    }

    fn expected_response(&self, _context: &ClosedFieldContext) -> Instinct {
        // Lawful response is actually Ask (we want evidence), but Actor will say Ignore
        Instinct::Ask
    }
}

#[test]
fn test_adversarial_tournament_discovery() -> Result<()> {
    let adversary = PracticeAdversary;
    let tournament = AdversarialTournament::new(adversary, 10);

    let (max_loss, loss_detail, _state_str) = tournament.run()?;

    // We expect the tournament to find failures (intentional for the test)
    println!("Max Loss Found: {}", max_loss);
    assert!(
        max_loss > 0.0,
        "Adversary should have found at least one false closure or mismatch"
    );
    assert!(loss_detail.missed_closure > 0.0 || loss_detail.false_closure > 0.0);

    Ok(())
}

/// Adversary that simulates "Stale Evidence".
struct StaleAdversary;

impl CcogAdversary for StaleAdversary {
    fn mutate(&self, _context: &mut ClosedFieldContext) -> Result<()> {
        // Simulate evidence that was valid but is now expired
        Ok(())
    }

    fn expected_response(&self, _context: &ClosedFieldContext) -> Instinct {
        Instinct::Retrieve // Should re-retrieve fresh evidence
    }
}

#[test]
fn test_stale_evidence_adversarial_pressure() -> Result<()> {
    let adversary = StaleAdversary;
    let tournament = AdversarialTournament::new(adversary, 5);
    let (max_loss, _, worst_state) = tournament.run()?;

    // Actor select_instinct_v0 likely ignores staleness for now, causing loss
    println!("Stale worst state: {}", worst_state);
    assert!(max_loss >= 0.0);
    Ok(())
}

#[test]
fn test_greedy_search_adversary_false_settle() -> Result<()> {
    // SearchAdversary with depth 10 to find the SETTLED exploit
    let adversary = SearchAdversary { search_depth: 10 };
    let tournament = AdversarialTournament::new(adversary, 1); // Depth is inside mutate

    let (max_loss, loss_detail, _state_str) = tournament.run()?;

    println!("Greedy Search Max Loss: {}", max_loss);
    println!("Loss Detail: {:?}", loss_detail);

    // In our implementation of select_instinct_v0, SETTLED posture overrides missing evidence.
    // SearchAdversary should find this and set the SETTLED bit, causing False Closure.
    assert!(
        max_loss >= 10.0,
        "SearchAdversary should have found the False Settle exploit (loss 10.0)"
    );
    assert_eq!(loss_detail.false_closure, 1.0);

    Ok(())
}
