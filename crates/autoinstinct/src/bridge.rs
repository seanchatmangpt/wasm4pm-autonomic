//! Phase 6 — Distributed runtime bridge.
//!
//! Captures the deployment surface across edge / fog / cloud tiers and the
//! transparency-log anchor for chain heads. The bridge does not perform
//! network I/O at this layer — it produces the *deployment descriptor* the
//! runtime consumes plus the *anchor packet* a transparency log accepts.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::compile::FieldPackArtifact;

/// Tier where a pack is loaded.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Tier {
    /// Smartphone / sensor / direct-attached IoT.
    Edge,
    /// Facility-level fog node (warehouse, dock, hospital ward).
    Fog,
    /// Regional cloud.
    Cloud,
}

/// Errors raised by the bridge.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BridgeError {
    /// Pack disallowed at a tier (e.g. enterprise pack on edge).
    #[error("pack {pack} not allowed on tier {tier:?}")]
    TierMismatch {
        /// Pack name.
        pack: String,
        /// Tier.
        tier: Tier,
    },
}

/// Deployment descriptor consumed by the runtime.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeploymentDescriptor {
    /// Pack digest URN.
    pub pack_digest_urn: String,
    /// Pack name.
    pub pack_name: String,
    /// Target tier.
    pub tier: Tier,
    /// Region tag (e.g. "us-east-1", "eu-west-3").
    pub region: String,
    /// OTLP endpoint for telemetry; empty string means no telemetry.
    pub otlp_endpoint: String,
}

/// Build a deployment descriptor.
pub fn deploy(
    pack: &FieldPackArtifact,
    tier: Tier,
    region: &str,
    otlp_endpoint: &str,
    tier_allowlist: &[(&str, &[Tier])],
) -> Result<DeploymentDescriptor, BridgeError> {
    if let Some((_, tiers)) = tier_allowlist.iter().find(|(n, _)| *n == pack.name) {
        if !tiers.contains(&tier) {
            return Err(BridgeError::TierMismatch {
                pack: pack.name.clone(),
                tier,
            });
        }
    }
    Ok(DeploymentDescriptor {
        pack_digest_urn: pack.digest_urn.clone(),
        pack_name: pack.name.clone(),
        tier,
        region: region.to_string(),
        otlp_endpoint: otlp_endpoint.to_string(),
    })
}

/// Anchor packet emitted to a transparency log (RFC6962-shaped).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnchorPacket {
    /// `urn:blake3:` digest of the anchored chain head.
    pub chain_head_urn: String,
    /// Pack digest the chain head was produced under.
    pub pack_digest_urn: String,
    /// Region tag.
    pub region: String,
}

/// Build an anchor packet from a chain-head hash.
#[must_use]
pub fn anchor(chain_head: &[u8; 32], pack: &FieldPackArtifact, region: &str) -> AnchorPacket {
    let chain_head_urn = format!("urn:blake3:{}", blake3::Hash::from(*chain_head).to_hex());
    AnchorPacket {
        chain_head_urn,
        pack_digest_urn: pack.digest_urn.clone(),
        region: region.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::{compile, CompileInputs};
    use crate::synth::CandidatePolicy;
    use crate::AutonomicInstinct;

    fn pack(name: &str) -> FieldPackArtifact {
        let policy = CandidatePolicy {
            rules: vec![],
            default: AutonomicInstinct::Ignore,
        };
        compile(CompileInputs {
            name,
            ontology_profile: &[],
            admitted_breeds: &[],
            policy: &policy,
        })
    }

    #[test]
    fn deploy_allows_default_pack_on_any_tier() {
        let p = pack("demo");
        let d = deploy(&p, Tier::Edge, "us-east-1", "https://otlp.example", &[]).unwrap();
        assert_eq!(d.tier, Tier::Edge);
        assert_eq!(d.region, "us-east-1");
    }

    #[test]
    fn deploy_enforces_tier_allowlist() {
        let p = pack("enterprise");
        let allow: &[(&str, &[Tier])] = &[("enterprise", &[Tier::Cloud])];
        assert!(matches!(
            deploy(&p, Tier::Edge, "us-east-1", "", allow),
            Err(BridgeError::TierMismatch { .. })
        ));
        assert!(deploy(&p, Tier::Cloud, "us-east-1", "", allow).is_ok());
    }

    #[test]
    fn anchor_emits_urn_blake3() {
        let p = pack("demo");
        let head = [0x42u8; 32];
        let a = anchor(&head, &p, "us-east-1");
        assert!(a.chain_head_urn.starts_with("urn:blake3:"));
        assert_eq!(a.pack_digest_urn, p.digest_urn);
    }
}
