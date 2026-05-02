# Layout and Admission Spec v0.4

## Canonical Rule
- **ReferenceLawPath** defines semantics.
- **AdmittedLayout** defines machine shape.
- **WireV1** defines canonical bytes.
- **Truthforge** defines admission.

## In-Memory Layout vs Wire/File Encoding
- In-memory layout may be `repr(C)` / aligned (e.g., 32-byte `Cog8Row`).
- Wire layout must be explicitly encoded. No raw struct transmute becomes `.powl64`.
- `.powl64` requires explicit endianness, magic headers (`POWL64\0\1`), and enum decode rejection.

## Equivalence Contract
`ReferenceLawPath(x) == CandidateFastPath(x)`
A fast path (SIMD, table, intrinsic) is admitted only if it yields the exact same closure, selection, and evidence as the reference path.
