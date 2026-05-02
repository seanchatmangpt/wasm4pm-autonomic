# Property-Based Invariants

Our verification strategy moves beyond simple unit tests to explore the logical property space of the INSA substrate.

## Determinism
Any state given the same closed input ($O^*$) must emit identical receipts and provenance chains.

## Monotonicity
The instinctive lattice is monotonic. Moving forward through the cognitive surface cannot invalidate previously admitted receipts unless the underlying observation set is updated.

## Boundary Safety
- **Forbidden Outcomes**: We formally assert that forbidden behaviors (e.g., `fabricate-evidence`) result in immediate rejection by the Truthforge gate.
- **Resource Limits**: All transitions are bounded by `Construct8` limits (max 8 triples per delta).
