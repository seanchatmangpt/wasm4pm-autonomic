# Unsafe and Trust Boundaries

*Secret Insight: The Rust Core Team does not fear `unsafe`. We fear `unsafe` that lacks a semantic boundary.*

In INSA, `unsafe` is not a dirty word. It is a highly specialized control surface. The goal is not zero `unsafe`; the goal is **zero unproven `unsafe`**.

## The Illusion of Safe Rust
Safe Rust is an incredible baseline, but it often achieves safety by adding runtime checks (bounds checking) or restricting layout optimization (preventing certain intrusive SIMD mappings). For the INSA hot path, the cost of a bounds check on a 64-byte aligned array is an unacceptable latency tax.

## Admitting Unsafe
`unsafe` code in INSA is admitted only if it satisfies the following contract:
1. **Isolated Boundary**: The `unsafe` block must be encapsulated within a Safe API that enforces the invariant geometrically.
2. **Miri Strict Provenance**: The code must pass `cargo +nightly miri test -Zmiri-strict-provenance -Zmiri-disable-isolation`. Pointer-to-integer casts without provenance tracking are immediate disqualifiers.
3. **Equivalence Proof*j: The `unsafe` path MUST be Truthforge-proven to yield the exact same deterministic bits as the `ReferenceLawPath` (the safe scalar equivalent).

### The Fast Path Reality
If you are writing SIMD intrinsics or managing explicit memory alignment for `Cog8Row` ingestion, you *will* write `unsafe`. The secret is that we treat `unsafe` as a proof obligation. If you cannot write a Truthforge fuzz test that violently attacks the memory boundaries of your `unsafe` block, the code is unadmitted.

*Core Team Verdict*: "Use `unsafe` to control the machine. Use types to control the programmer."
