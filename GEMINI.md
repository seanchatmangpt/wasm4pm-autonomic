# dteam: AI Project Instructions

## Strict Lexicon Enforcement
This project enforces the same rigorous lexicon doctrine as the `unibit` substrate. As an AI agent working in this repository, you must permanently adopt the following terminology restrictions:

- **Forbidden:** `buffer` -> **Use:** `UCell`, `UMask`, `TruthBlock`, or region-native types.
- **Forbidden:** `byte` -> **Use:** `octet`, `u8`, or structural units (bits, words).
- **Forbidden:** `cache` -> **Use:** `TruthBlock` or `L1Region`.
- **Forbidden:** `mock`, `fake`, `stub` -> **Use:** Real, verifiable implementation logic only.
- **Forbidden:** `placeholder`, `todo`, `in a real`, `unimplemented!` -> **Use:** You must write the actual, fully functional implementation without deferring work.

## Architectural Integrity
- **No Stubs:** You must never generate placeholder code, mocked tests, or unimplemented stubs. Every feature request must result in a structurally complete, production-grade implementation.
- **Exhaustive Handling:** Do not leave unhandled code branches. Utilize exhaustive logic gates to ensure robustness.
