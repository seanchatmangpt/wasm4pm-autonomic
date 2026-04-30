# Pipeline Verification & Supply Chain Integrity

This document describes the validation guarantees provided by the `autoinstinct` master pipeline.

## 1. End-to-End Pipeline (master_pipeline_end_to_end_reality)
The system enforces a strict, fixture-less verification chain:
1. **Ingestion**: `world_to_corpus` validates OCEL logs against structural constraints (dangling references, empty sets).
2. **Motif Discovery**: `discover` produces deterministic motifs based on evidence support.
3. **Synthesis**: `synthesize` transforms discovered motifs into a formal policy.
4. **Gauntlet**: The policy is tested against generated JTBD scenarios (positive + perturbation + forbidden boundary).
5. **Compilation**: `compile` produces a signed `FieldPackArtifact`.
6. **Registration**: The `PackRegistry` verifies the digest matches the artifact, ensuring immutability through the deployment lifecycle.

## 2. Anti-Fake CLI Integration
The `ainst` CLI enforces these invariants in the main operational loop:
- `--mode anti-fake`: Runs the complete gauntlet, including the causal dependency harness and performance honesty tests.
- `--evidence`: Generates an audit log containing the git state, test output, and branch context.
- `--require-clean-git`: Forces strict consistency, ensuring the artifact was generated from a clean code tree (optional).

## 3. LLM Admission Gate
- **Shape Gate**: `serde(deny_unknown_fields)` ensures no injected adversarial payloads exist in the OCEL schema.
- **Fail-Hard Contract**: The admission logic rejects malformed LLM responses without falling back to cached fixtures, guaranteeing the system actually evaluates the proposed policy.
- **Ontology Enforcement**: Only profiles defined in the canonical ontology are permitted; private namespace terms trigger `NonPublicOntology` failures.
