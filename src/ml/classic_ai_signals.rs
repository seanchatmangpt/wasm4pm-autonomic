//! Classical AI signal generators for HDIT AutoML.
//!
//! This module is a thin re-export and integration layer. The actual algorithms
//! live in their own modules (`crate::ml::eliza`, `mycin`, `strips`, `shrdlu`,
//! `hearsay`). All five run at nanosecond scale on the hot path, making them
//! suitable as T0 tier signals.
//!
//! # Why not unibit?
//!
//! dteam is the stable production path. unibit-ai-classic encodes these systems
//! as POWL8/POWL64 Motion packets — useful for formal verification, but unibit
//! is nightly. dteam provides faithful, idiomatic Rust implementations that:
//!
//! - run inline as execution physics, not advisory cognition
//! - integrate directly with HDIT AutoML's signal pipeline
//! - have no unibit-* dependency

pub use crate::ml::eliza::eliza_automl_signal;
pub use crate::ml::hearsay::hearsay_automl_signal;
pub use crate::ml::mycin::mycin_automl_signal;
pub use crate::ml::shrdlu::shrdlu_automl_signal;
pub use crate::ml::strips::strips_automl_signal;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::eliza::{kw, keyword_bit};
    use crate::ml::hdit_automl::{run_hdit_automl, Tier};
    use crate::ml::mycin::{fact, org};
    use crate::ml::shrdlu::{self, Cmd};
    use crate::ml::strips;

    #[test]
    fn cross_pollinate_all_five_classical_systems_into_automl() {
        let anchor = vec![true, false, true, false];

        // ELIZA: dream keyword present in slots 0, 2 (matches anchor)
        let eliza_inputs = vec![
            keyword_bit(kw::DREAM),
            keyword_bit(kw::YOU),       // YOU has no rule → no match → false
            keyword_bit(kw::DREAM),
            0,                          // no keywords → no match → false
        ];
        let eliza_sig = eliza_automl_signal("eliza_dream", &eliza_inputs, &anchor);

        // MYCIN: STREP diagnosis present in slots 0, 2
        let mycin_patients = vec![
            fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS,
            fact::GRAM_NEG | fact::ANAEROBIC,
            fact::GRAM_POS | fact::AEROBIC | fact::FEVER | fact::RIGORS,
            fact::FEVER,
        ];
        let mycin_sig = mycin_automl_signal("mycin_strep", &mycin_patients, org::STREP, &anchor);

        // STRIPS: HOLDING_A reachable from slot 0 (initial state) and slot 2
        let strips_states = vec![
            strips::CLEAR_A | strips::ON_TABLE_A | strips::ARM_EMPTY | strips::CLEAR_B | strips::ON_TABLE_B,
            strips::HOLDING_B, // arm not empty, can't pickup A immediately
            strips::CLEAR_A | strips::ON_TABLE_A | strips::ARM_EMPTY | strips::CLEAR_C | strips::ON_TABLE_C,
            0, // empty state, no preconditions met
        ];
        let strips_sig = strips_automl_signal("strips_pickup_A", &strips_states, strips::HOLDING_A, &anchor);

        // SHRDLU: PickUp(A) succeeds in slots 0, 2
        let shrdlu_states = vec![
            shrdlu::initial_state(),
            shrdlu::holding(1), // arm not empty
            shrdlu::initial_state(),
            shrdlu::holding(2),
        ];
        let shrdlu_sig = shrdlu_automl_signal("shrdlu_pickup_A", &shrdlu_states, Cmd::PickUp(0), &anchor);

        // Hearsay-II: all reach sentence (signal won't differentiate, used for completeness)
        let hearsay_inputs = vec![0xCAFE_u64, 0x0_u64, 0xBABE_u64, 0x0_u64];
        let _hearsay_sig = hearsay_automl_signal("hearsay", &hearsay_inputs, &anchor);

        // Verify each signal correctly predicts against anchor
        assert_eq!(eliza_sig.accuracy_vs_anchor, 1.0, "ELIZA should match anchor");
        assert_eq!(mycin_sig.accuracy_vs_anchor, 1.0, "MYCIN should match anchor");
        assert_eq!(strips_sig.accuracy_vs_anchor, 1.0, "STRIPS should match anchor");
        assert_eq!(shrdlu_sig.accuracy_vs_anchor, 1.0, "SHRDLU should match anchor");

        // All four perfect signals must be T0 tier (nanosecond execution)
        assert_eq!(eliza_sig.tier, Tier::T0, "ELIZA must be T0");
        assert_eq!(mycin_sig.tier, Tier::T0, "MYCIN must be T0");
        assert_eq!(strips_sig.tier, Tier::T0, "STRIPS must be T0");
        assert_eq!(shrdlu_sig.tier, Tier::T0, "SHRDLU must be T0");

        // Run AutoML with all four perfect signals
        let candidates = vec![eliza_sig, mycin_sig, strips_sig, shrdlu_sig];
        let plan = run_hdit_automl(candidates, &anchor, 2);
        assert!(!plan.selected.is_empty(), "AutoML must select at least one signal");
        assert_eq!(plan.plan_accuracy, 1.0, "perfect signals → perfect plan");
    }

    #[test]
    fn all_signals_are_nanosecond_tier() {
        use crate::ml::hdit_automl::Tier;
        let anchor = vec![true; 50];

        let eliza_sig = eliza_automl_signal("e", &[keyword_bit(kw::DREAM); 50], &anchor);
        let mycin_sig = mycin_automl_signal("m", &[fact::GRAM_NEG | fact::ANAEROBIC; 50], org::BACTEROIDES, &anchor);
        let strips_sig = strips_automl_signal("s", &[strips::HOLDING_A; 50], strips::HOLDING_A, &anchor);
        let shrdlu_sig = shrdlu_automl_signal("sh", &[shrdlu::initial_state(); 50], Cmd::PickUp(0), &anchor);

        assert_eq!(eliza_sig.tier, Tier::T0);
        assert_eq!(mycin_sig.tier, Tier::T0);
        assert_eq!(strips_sig.tier, Tier::T0);
        assert_eq!(shrdlu_sig.tier, Tier::T0);
    }
}
