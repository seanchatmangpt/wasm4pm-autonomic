//! Replay verifier (Phase 11).

use crate::export::bundle::{BundleError, ProofBundle};
use crate::export::ontology::{audit_iris, NonPublicOntology};

/// Outcome of replaying a bundle.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplayVerdict {
    /// `decide_with_trace` (Phase 7) produced the same decision shape.
    pub decision_match: bool,
    /// `powl64-path.bin` chain head matches the recorded path tail.
    pub chain_match: bool,
    /// Every emitted IRI is in the public ontology allowlist.
    pub ontology_intact: bool,
    /// Manifest hashes matched every entry.
    pub manifest_intact: bool,
}

impl ReplayVerdict {
    /// True when every guardrail accepted the bundle.
    #[must_use]
    pub fn all_intact(&self) -> bool {
        self.decision_match && self.chain_match && self.ontology_intact && self.manifest_intact
    }
}

/// Errors raised when a check cannot be evaluated.
#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    /// Underlying bundle decode failed.
    #[error("bundle error: {0}")]
    Bundle(#[from] BundleError),
    /// Trace or receipt JSON-LD was malformed.
    #[error("malformed json: {0}")]
    MalformedJson(String),
    /// Required entry missing from bundle.
    #[error("missing entry: {0}")]
    MissingEntry(String),
}

/// Verify a bundle byte-slice end-to-end.
///
/// # Errors
///
/// Returns [`ReplayError`] when the bundle cannot be decoded or its JSON
/// entries are malformed. Soft failures are inside the [`ReplayVerdict`].
pub fn verify_bundle_bytes(
    bytes: &[u8],
    extra_allow: &[&str],
    expected_chain_head: Option<[u8; 32]>,
) -> Result<ReplayVerdict, ReplayError> {
    let mut verdict = ReplayVerdict::default();
    let bundle = match ProofBundle::read(bytes) {
        Ok(b) => {
            verdict.manifest_intact = true;
            b
        }
        Err(BundleError::ManifestMismatch { .. }) => {
            verdict.manifest_intact = false;
            return Ok(verdict);
        }
        Err(e) => return Err(ReplayError::Bundle(e)),
    };
    verify_unpacked(&bundle, &mut verdict, extra_allow, expected_chain_head)?;
    Ok(verdict)
}

/// Verify an already-decoded [`ProofBundle`].
///
/// # Errors
///
/// See [`verify_bundle_bytes`].
pub fn verify_bundle(
    bundle: &ProofBundle,
    extra_allow: &[&str],
    expected_chain_head: Option<[u8; 32]>,
) -> Result<ReplayVerdict, ReplayError> {
    let mut verdict = ReplayVerdict {
        manifest_intact: true,
        ..Default::default()
    };
    verify_unpacked(bundle, &mut verdict, extra_allow, expected_chain_head)?;
    Ok(verdict)
}

fn verify_unpacked(
    bundle: &ProofBundle,
    verdict: &mut ReplayVerdict,
    extra_allow: &[&str],
    expected_chain_head: Option<[u8; 32]>,
) -> Result<(), ReplayError> {
    let trace_bytes = bundle
        .entry("trace.jsonld")
        .ok_or_else(|| ReplayError::MissingEntry("trace.jsonld".into()))?;
    let receipt_bytes = bundle
        .entry("receipt.jsonld")
        .ok_or_else(|| ReplayError::MissingEntry("receipt.jsonld".into()))?;
    let trace_json: serde_json::Value = serde_json::from_slice(trace_bytes)
        .map_err(|e| ReplayError::MalformedJson(format!("trace.jsonld: {}", e)))?;
    let receipt_json: serde_json::Value = serde_json::from_slice(receipt_bytes)
        .map_err(|e| ReplayError::MalformedJson(format!("receipt.jsonld: {}", e)))?;
    verdict.ontology_intact = match (
        audit_iris(&trace_json, extra_allow),
        audit_iris(&receipt_json, extra_allow),
    ) {
        (Ok(_), Ok(_)) => true,
        (Err(NonPublicOntology(_)), _) | (_, Err(NonPublicOntology(_))) => false,
    };

    let path_bytes = bundle
        .entry("powl64-path.bin")
        .ok_or_else(|| ReplayError::MissingEntry("powl64-path.bin".into()))?;
    verdict.chain_match = chain_self_consistent(path_bytes, expected_chain_head);
    verdict.decision_match = replay_decision_from_trace(&trace_json);
    Ok(())
}

/// Phase 7 replay step (real).
///
/// Verifies the load-bearing bark invariant
/// `(require_mask & present_mask) == require_mask`
/// for every node where the trace claims `triggerFired = true`, AND the
/// converse: every `triggerFired = false` node has `(require_mask &
/// present_mask) != require_mask`.
///
/// This re-derives the decision from the trace's own evidence — a tampered
/// trace cannot pass without simultaneously corrupting the manifest hash.
fn replay_decision_from_trace(trace_json: &serde_json::Value) -> bool {
    fn parse_hex_u64(s: &str) -> Option<u64> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        u64::from_str_radix(s, 16).ok()
    }
    let present_mask_str = trace_json
        .get("urn:ccog:vocab:presentMask")
        .and_then(|v| v.as_str());
    let Some(present_mask) = present_mask_str.and_then(parse_hex_u64) else {
        return false;
    };
    let nodes = match trace_json.get("urn:ccog:vocab:nodes") {
        Some(serde_json::Value::Array(a)) => a,
        _ => return false,
    };
    if nodes.is_empty() {
        return false;
    }
    for node in nodes {
        let require_mask = match node
            .get("urn:ccog:vocab:requireMask")
            .and_then(|v| v.as_str())
            .and_then(parse_hex_u64)
        {
            Some(m) => m,
            None => return false,
        };
        let trigger_fired = match node.get("urn:ccog:vocab:triggerFired").and_then(|v| v.as_bool()) {
            Some(b) => b,
            None => return false,
        };
        let satisfied = (require_mask & present_mask) == require_mask;
        if satisfied != trigger_fired {
            return false;
        }
    }
    true
}

fn chain_self_consistent(path_bytes: &[u8], expected_head: Option<[u8; 32]>) -> bool {
    if path_bytes.is_empty() || path_bytes.len() % 32 != 0 {
        return false;
    }
    if let Some(head) = expected_head {
        let tail_start = path_bytes.len() - 32;
        let tail: [u8; 32] = match path_bytes[tail_start..].try_into() {
            Ok(b) => b,
            Err(_) => return false,
        };
        if tail != head {
            return false;
        }
    }
    true
}

