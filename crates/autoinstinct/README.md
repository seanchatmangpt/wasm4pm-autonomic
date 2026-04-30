# autoinstinct: Policy Synthesis & Verification

The `autoinstinct` crate manages the lifecycle of autonomic policy synthesis: ingestion of observational data, discovery of causal motifs, policy proposal, and strict formal verification of admitted behaviors.

## Core Responsibilities

- **Policy Ingestion**: Bridges OCEL world-state into trace corpora.
- **Motif Discovery**: Analyzes causal patterns and synthesizes candidate policies.
- **Formal Verification (Gauntlet)**: Admits policies only when they satisfy positive assertions, causal perturbation tests, and boundary forbidden-response checks.
- **Artifact Lifecycle**: Compiles field packs, generates tamper-resistant manifests, and supports runtime deployment/registry verification.

## Architecture

- **`ocel/`**: Implements OCEL-to-Corpus bridges and validation logic.
- **`motifs/`**: Statistical and causal motif discovery engine.
- **`synth/`**: Policy synthesis from motif support.
- **`gauntlet/`**: The admit/deny gate for policy artifacts.
- **`manifest/`**: Tamper-detection and auditing for field-pack deployment.

## Anti-Fake Gauntlet Compliance

`autoinstinct` implements the master anti-fake verification gate (`ainst run gauntlet --mode anti-fake`). This toolchain ensures that every synthesized policy is physically earned through evidence-backed motifs and passes end-to-end supply chain verification (OCEL -> Policy -> Pack -> Deployment).

## Documentation

- [Pipeline Verification](./docs/pipeline_verification.md): Technical breakdown of the end-to-end verification and supply chain guarantees.
