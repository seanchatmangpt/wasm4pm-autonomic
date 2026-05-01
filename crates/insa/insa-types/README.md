# `insa-types`

**Core Primitives of the INSA Architecture.**

This crate defines the foundational, zero-logic, cache-aligned primitives that enforce the `O*` (Closed-Field Observation) boundary. By offloading these definitions to a dedicated crate, INSA ensures that state representations are universally compatible, strictly typed, and completely free of dynamic allocation or branching logic.

## Semantic Field Masks
At the heart of the closed-field architecture are strictly typed bit-masks. Rather than utilizing unbounded vectors or hash maps for state tracking, INSA restricts local evaluation context to precisely 64 concurrent semantic states.

* **`FieldMask (u64)`**: A `repr(transparent)` structure tracking active semantic conditions within the execution environment.
* **`CompletedMask (u64)`**: A parallel `repr(transparent)` mask tracking the completion state of execution nodes within an active topology.
* **`FieldBit (u8)`**: A strictly validated primitive (0-63) enforcing safe mutation bounds when activating or deactivating flags on masks.

## Topology & Execution Identifiers
INSA routes logic across topological execution graphs. To guarantee cache-line efficiency and serialization stability, all identifiers are implemented as `repr(transparent)` primitives over bounded integers:

* **`u16` Identifiers**: `PackId`, `GroupId`, `RuleId`, `BreedId`, `NodeId`, `EdgeId`.
* **`u64` Identifiers**: `RouteId`.

## Domain Target Resolvers
* `ObjectRef`: The target identifier of the entity under semantic evaluation.
* `DictionaryDigest`: A Blake3-backed fast lookup hash for ontology terms.
* `PolicyEpoch`: Tracks the strict chronological validity of execution sets.