//! Phase 7 — Domain pack library.
//!
//! Catalog of seven Fortune-5 production domains with their ontology
//! profile, admitted breeds, posture/context bit ranges, and pack-tier
//! constraints. AutoInstinct uses this as the canonical domain registry —
//! every compiled pack must declare its domain so downstream governance
//! (Phase 9) can apply domain-specific rules (HIPAA for healthcare,
//! SOX/PCI for financial, etc.).

use serde::{Deserialize, Serialize};

use crate::bridge::Tier;

/// One supported product domain.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Domain {
    /// Lifestyle Redesign / Occupational Therapy.
    Lifestyle,
    /// Edge / Home cognition (cameras, doors, packages).
    Edge,
    /// Enterprise process intelligence.
    Enterprise,
    /// Developer / agent governance.
    Dev,
    /// Supply chain (camera/drone/badge/GPS/cold-chain/customs).
    SupplyChain,
    /// Healthcare-adjacent care support (never claims diagnosis).
    Healthcare,
    /// Financial compliance, fraud, audit.
    Financial,
}

/// Domain profile — the constitutional contract a pack must obey to claim
/// the domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainProfile {
    /// Human-readable domain name.
    pub name: &'static str,
    /// Public ontology prefixes admitted for this domain.
    pub ontology_profile: &'static [&'static str],
    /// Admitted breed names.
    pub admitted_breeds: &'static [&'static str],
    /// Allowed deployment tiers.
    pub allowed_tiers: &'static [Tier],
    /// Domain-specific compliance flags (e.g. "HIPAA", "GDPR", "SOX", "PCI").
    pub compliance_flags: &'static [&'static str],
}

/// Resolve the canonical profile for a domain.
#[must_use]
pub fn profile(d: Domain) -> DomainProfile {
    match d {
        Domain::Lifestyle => DomainProfile {
            name: "lifestyle",
            ontology_profile: &[
                "https://schema.org/",
                "http://www.w3.org/ns/prov#",
                "http://www.w3.org/2004/02/skos/core#",
                "urn:blake3:",
                "urn:ccog:vocab:",
            ],
            admitted_breeds: &["eliza", "mycin", "prolog"],
            allowed_tiers: &[Tier::Edge, Tier::Fog],
            compliance_flags: &["GDPR"],
        },
        Domain::Edge => DomainProfile {
            name: "edge",
            ontology_profile: &[
                "https://schema.org/",
                "http://www.w3.org/ns/prov#",
                "urn:blake3:",
            ],
            admitted_breeds: &["strips", "shrdlu", "eliza"],
            allowed_tiers: &[Tier::Edge, Tier::Fog],
            compliance_flags: &["GDPR"],
        },
        Domain::Enterprise => DomainProfile {
            name: "enterprise",
            ontology_profile: &[
                "https://schema.org/",
                "http://www.w3.org/ns/prov#",
                "http://www.w3.org/ns/shacl#",
                "http://www.w3.org/2001/XMLSchema#",
                "urn:blake3:",
                "urn:ccog:vocab:",
            ],
            admitted_breeds: &["eliza", "mycin", "strips", "shrdlu", "prolog", "hearsay", "dendral"],
            allowed_tiers: &[Tier::Fog, Tier::Cloud],
            compliance_flags: &["SOC2", "ISO27001"],
        },
        Domain::Dev => DomainProfile {
            name: "dev",
            ontology_profile: &[
                "http://www.w3.org/ns/prov#",
                "urn:blake3:",
                "urn:ccog:vocab:",
            ],
            admitted_breeds: &["mycin", "dendral", "prolog"],
            allowed_tiers: &[Tier::Cloud],
            compliance_flags: &[],
        },
        Domain::SupplyChain => DomainProfile {
            name: "supply-chain",
            ontology_profile: &[
                "https://schema.org/",
                "http://www.w3.org/ns/prov#",
                "http://www.w3.org/ns/sosa/",
                "http://qudt.org/schema/qudt/",
                "urn:blake3:",
            ],
            admitted_breeds: &["mycin", "strips", "shrdlu", "dendral"],
            allowed_tiers: &[Tier::Edge, Tier::Fog, Tier::Cloud],
            compliance_flags: &["SOC2", "ISO27001"],
        },
        Domain::Healthcare => DomainProfile {
            name: "healthcare",
            ontology_profile: &[
                "https://schema.org/",
                "http://www.w3.org/ns/prov#",
                "http://purl.org/dc/terms/",
                "http://www.w3.org/2004/02/skos/core#",
                "urn:blake3:",
            ],
            admitted_breeds: &["eliza", "mycin", "dendral"],
            allowed_tiers: &[Tier::Edge, Tier::Fog],
            compliance_flags: &["HIPAA", "GDPR"],
        },
        Domain::Financial => DomainProfile {
            name: "financial",
            ontology_profile: &[
                "http://www.w3.org/ns/prov#",
                "https://schema.org/",
                "http://www.w3.org/2001/XMLSchema#",
                "urn:blake3:",
            ],
            admitted_breeds: &["mycin", "strips", "dendral", "prolog"],
            allowed_tiers: &[Tier::Cloud],
            compliance_flags: &["SOX", "PCI", "SOC2"],
        },
    }
}

/// Iterate every supported domain. Useful for pack registry sweeps.
#[must_use]
pub fn all() -> &'static [Domain] {
    &[
        Domain::Lifestyle,
        Domain::Edge,
        Domain::Enterprise,
        Domain::Dev,
        Domain::SupplyChain,
        Domain::Healthcare,
        Domain::Financial,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_domain_has_at_least_one_breed() {
        for d in all() {
            let p = profile(*d);
            assert!(!p.admitted_breeds.is_empty(), "{} must admit ≥1 breed", p.name);
        }
    }

    #[test]
    fn every_domain_has_ontology_profile() {
        for d in all() {
            let p = profile(*d);
            assert!(p.ontology_profile.iter().any(|i| i.starts_with("urn:blake3:")));
        }
    }

    #[test]
    fn dev_pack_is_cloud_only() {
        let p = profile(Domain::Dev);
        assert_eq!(p.allowed_tiers, &[Tier::Cloud]);
    }

    #[test]
    fn healthcare_carries_hipaa() {
        let p = profile(Domain::Healthcare);
        assert!(p.compliance_flags.contains(&"HIPAA"));
    }

    #[test]
    fn financial_carries_sox_and_pci() {
        let p = profile(Domain::Financial);
        assert!(p.compliance_flags.contains(&"SOX"));
        assert!(p.compliance_flags.contains(&"PCI"));
    }
}
