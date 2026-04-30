# Anti-Fake Architecture: CCog Runtime Integrity

This document outlines the architectural mechanisms implemented to enforce physical honesty in CCog's runtime dispatch.

## 1. Zero-Heap Hot Path (Kill Zone 6)
The hot path (`decide()` and `select_instinct_v0()`) is enforced as heap-free.
- **Allocator Control**: We utilize a thread-local `CountingAlloc` injected via `#[global_allocator]` in test binaries to monitor and trap any heap allocations.
- **Zero-Allocation Invariant**: Any allocation within the hot-path closure triggers a test panic. This prevents "soft-fake" implementations that utilize `Vec`, `String`, or hashers to mask logic.
- **Positive Control**: The `anti_fake_perf_control_allocation_is_detected` test verifies the integrity of the allocator itself by deliberately triggering a `Vec` allocation and confirming the capture count > 0.

## 2. Manifest Tamper Resistance (Phase 3)
The manifest system provides an audited supply chain for field packs.
- **Canonical Bytes**: Manifests are derived from the canonical serialization of the `FieldPackArtifact` structure.
- **Digest Matrix**: Every field—including pack name, version, ontology profile, and rules counts—is incorporated into the `manifest_digest_urn`. 
- **Tamper Detection**: The `manifest_tamper_matrix` independently modifies every field to verify that any deviation results in an immediate failure of the `verify()` check.

## 3. Causal Reason Encoding
- **Reason-Aware Dispatch**: `select_instinct_v0_with_reason` returns not just the `AutonomicInstinct` result, but the deterministic string reason describing *why* the slot matched.
- **Exact-Match Verification**: Causal gauntlets now demand not just that a perturbation results in a different state, but that it results in the *exact* expected reason string. This eliminates "coincidence" paths where a system changes due to error rather than intent.
