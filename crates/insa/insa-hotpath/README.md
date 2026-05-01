# `insa-hotpath`

The Reference Law Path for INSA execution.

This is the primary execution kernel representing the "hot path" for compiled cognition. It contains the core semantic oracle for executing truth closures over bounded memory inputs. 

## Key Modules
* **`cog8`**: Contains the `Cog8Row`, a strictly aligned 32-byte C-repr struct. Each row evaluates a `required_mask` and `forbidden_mask` against the current semantic field. Matches map deterministically to an `InstinctByte` and `KappaByte`.
* **`construct8`**: Provides `Construct8Delta`, ensuring zero-allocation state mutations bounded strictly to 8 operations (e.g. `Set` or `Clear` on a `FieldBit`). By keeping this inline within a single struct array, the architecture prevents runaway state mutations.
* **`powl8`**: Sub-byte representation of logical operations.
* **`lut`**: Fast lookup tables.
* **`resolution`**: Converging outputs into a singular instinct outcome.
