//! Phase 5 — Pack lifecycle (channels, rollout, rollback).
//!
//! A `Channel` is a deployment lane (`canary`, `staging`, `prod`). A
//! `LifecycleLog` records every promotion and rollback so audit consumers
//! can reconstruct which pack version was live at any instant.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::compile::FieldPackArtifact;

/// Deployment channels — strictly ordered.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Channel {
    /// Pre-flight; small percentage of traffic.
    Canary,
    /// Pre-production; full traffic on a staging environment.
    Staging,
    /// Production; user-facing.
    Prod,
}

/// Errors raised by lifecycle operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LifecycleError {
    /// Promote attempted skipping a stage (e.g., Canary → Prod).
    #[error("illegal promotion: {from:?} → {to:?}")]
    IllegalPromotion {
        /// Source channel.
        from: Channel,
        /// Target channel.
        to: Channel,
    },
    /// Rollback called when no prior version exists on the target channel.
    #[error("no prior version on {0:?}")]
    NoPriorVersion(Channel),
    /// Pack not present in the catalog.
    #[error("pack {0} not found")]
    PackNotFound(String),
}

/// One lifecycle event.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// Pack first published to a channel (typically Canary).
    Published {
        /// Channel.
        channel: Channel,
        /// Pack digest URN.
        digest_urn: String,
    },
    /// Pack promoted from one channel to the next.
    Promoted {
        /// Source channel.
        from: Channel,
        /// Target channel.
        to: Channel,
        /// Pack digest URN.
        digest_urn: String,
    },
    /// Pack rolled back to a prior digest on a channel.
    RolledBack {
        /// Channel.
        channel: Channel,
        /// Digest now active.
        active_digest_urn: String,
        /// Digest demoted.
        demoted_digest_urn: String,
    },
}

/// Append-only lifecycle log.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LifecycleLog {
    /// Events in append order.
    pub events: Vec<LifecycleEvent>,
    /// Currently active digest per channel.
    pub active: IndexMap<Channel, String>,
    /// History per channel (most-recent last).
    pub history: IndexMap<Channel, Vec<String>>,
}

impl LifecycleLog {
    /// Empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Publish a pack on a channel (typically Canary). Replaces the active
    /// digest on that channel.
    pub fn publish(&mut self, channel: Channel, pack: &FieldPackArtifact) {
        self.history
            .entry(channel)
            .or_default()
            .push(pack.digest_urn.clone());
        self.active.insert(channel, pack.digest_urn.clone());
        self.events.push(LifecycleEvent::Published {
            channel,
            digest_urn: pack.digest_urn.clone(),
        });
    }

    /// Promote the active digest from `from` to `to`. Promotion may only
    /// step Canary → Staging → Prod (no skips, no inversions).
    pub fn promote(&mut self, from: Channel, to: Channel) -> Result<(), LifecycleError> {
        let valid = matches!(
            (from, to),
            (Channel::Canary, Channel::Staging) | (Channel::Staging, Channel::Prod)
        );
        if !valid {
            return Err(LifecycleError::IllegalPromotion { from, to });
        }
        let digest = self
            .active
            .get(&from)
            .ok_or(LifecycleError::NoPriorVersion(from))?
            .clone();
        self.history.entry(to).or_default().push(digest.clone());
        self.active.insert(to, digest.clone());
        self.events.push(LifecycleEvent::Promoted {
            from,
            to,
            digest_urn: digest,
        });
        Ok(())
    }

    /// Roll back a channel to its second-most-recent digest.
    pub fn rollback(&mut self, channel: Channel) -> Result<(), LifecycleError> {
        let history = self
            .history
            .get_mut(&channel)
            .ok_or(LifecycleError::NoPriorVersion(channel))?;
        if history.len() < 2 {
            return Err(LifecycleError::NoPriorVersion(channel));
        }
        let demoted = history.pop().expect("≥2");
        let active = history.last().expect("≥1").clone();
        self.active.insert(channel, active.clone());
        self.events.push(LifecycleEvent::RolledBack {
            channel,
            active_digest_urn: active,
            demoted_digest_urn: demoted,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::{compile, CompileInputs};
    use crate::synth::CandidatePolicy;
    use crate::AutonomicInstinct;

    fn pack(default: AutonomicInstinct) -> FieldPackArtifact {
        let policy = CandidatePolicy {
            rules: vec![],
            default,
        };
        compile(CompileInputs {
            name: "lifecycle-test",
            ontology_profile: &[],
            admitted_breeds: &[],
            policy: &policy,
        })
    }

    #[test]
    fn lifecycle_promotes_through_all_channels() {
        let mut log = LifecycleLog::new();
        let p = pack(AutonomicInstinct::Ignore);
        log.publish(Channel::Canary, &p);
        log.promote(Channel::Canary, Channel::Staging).unwrap();
        log.promote(Channel::Staging, Channel::Prod).unwrap();
        assert_eq!(log.active.get(&Channel::Prod), Some(&p.digest_urn));
        assert_eq!(log.events.len(), 3);
    }

    #[test]
    fn lifecycle_rejects_skipped_promotion() {
        let mut log = LifecycleLog::new();
        let p = pack(AutonomicInstinct::Ignore);
        log.publish(Channel::Canary, &p);
        assert!(matches!(
            log.promote(Channel::Canary, Channel::Prod),
            Err(LifecycleError::IllegalPromotion { .. })
        ));
    }

    #[test]
    fn lifecycle_rolls_back_to_prior_version() {
        let mut log = LifecycleLog::new();
        let p1 = pack(AutonomicInstinct::Ignore);
        let p2 = pack(AutonomicInstinct::Ask);
        log.publish(Channel::Canary, &p1);
        log.publish(Channel::Canary, &p2);
        assert_eq!(log.active.get(&Channel::Canary), Some(&p2.digest_urn));
        log.rollback(Channel::Canary).unwrap();
        assert_eq!(log.active.get(&Channel::Canary), Some(&p1.digest_urn));
    }

    #[test]
    fn lifecycle_rollback_requires_prior_version() {
        let mut log = LifecycleLog::new();
        let p = pack(AutonomicInstinct::Ignore);
        log.publish(Channel::Canary, &p);
        assert!(matches!(
            log.rollback(Channel::Canary),
            Err(LifecycleError::NoPriorVersion(_))
        ));
    }
}
