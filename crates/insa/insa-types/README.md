# `insa-types`

Fundamental, zero-logic, cache-shaped primitives for the INSA architecture.

This crate serves as the foundation of the INSA ecosystem. It defines the core identifiers, masks, and objects that contain no execution logic, ensuring they remain lightweight, cache-friendly, and universally compatible across higher-level execution crates.

## Key Primitives
* **`FieldMask`, `CompletedMask`**: 64-bit representations (`u64`) of the closed-field presence state (`O*`). Modified strictly via bounded `FieldBit` (0-63).
* **Domain Resolvers**: `ObjectRef`, `DictionaryDigest`, `PolicyEpoch`.
* **Transparent Identifiers**: Cache-aligned primitives mapped to standard types.
  * `u16`: `PackId`, `GroupId`, `RuleId`, `BreedId`, `NodeId`, `EdgeId`.
  * `u64`: `RouteId`.
