# Cognitive Substrate: `ccog`

The `ccog` crate is the fundamental execution layer. It defines the formal state transitions and the dispatch substrate that powers autonomic instincts.

## Architectural Purpose
`ccog` translates declarative rules into high-performance, heap-free dispatch primitives. It handles the translation from RDF-based field context to bit-masked lattice responses.

## Key Primitives
- **`Powl64`**: A constant-time, bit-width-encoded partial order workflow representation.
- **`BarkDecision`**: A deterministic outcome resulting from the intersection of posture and risk masks.
- **`Construct8`**: A limited-arity, zero-allocation delta structure used to assert changes to the field context.

## Verification
`ccog` is audited against the [Anti-Fake Gauntlet](../integrity/anti_fake.md), guaranteeing that its dispatch logic is not only mathematically sound but also physically honest regarding performance and side-effect predictability.
