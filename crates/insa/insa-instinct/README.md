# `insa-instinct`

**INST8, KAPPA8, and Conflict Resolution.**

The `insa-instinct` crate defines the absolute lowest-level outcome bounds of the system. It governs how the architecture resolves conflicts when multiple execution paths trigger simultaneously, synthesizing them into a singular, lawful `A = µ(O*)` directive.

## The Semantic Response Lane (`InstinctByte`)
Execution in INSA never results in arbitrary text or open-ended control flow. It always collapses into an `InstinctByte`—an 8-bit vector defining the exact action the system must take next:

* `SETTLE` (1<<0): The semantic field is closed. Execution may proceed.
* `RETRIEVE` (1<<1): The field is lacking facts required for closure; trigger memory resolution.
* `INSPECT` (1<<2): Trigger analytical observation.
* `ASK` (1<<3): Hand control to a human-in-the-loop or external Oracle.
* `AWAIT` (1<<4): Halt execution pending asynchronous state changes.
* `REFUSE` (1<<5): Absolute security denial. The trajectory is unlawful.
* `ESCALATE` (1<<6): Force a hard security audit.
* `IGNORE` (1<<7): Terminal no-op.

## The Attribution Vector (`KappaByte`)
To provide exact cryptographic traceability for *why* an action occurred, the architecture binds a `KappaByte` to every outcome, identifying the underlying `CollapseEngine` that formulated the decision:

* `REFLECT`, `PRECONDITION`, `GROUND`, `PROVE`, `RULE`, `RECONSTRUCT`, `FUSE`, `REDUCE_GAP`.

## Family8 Micro-Bytes and `KappaDetail16`
For ultra-granular telemetry, INSA uses "Family8" bytes (`PrologByte`, `MycinByte`, `StripsByte`, etc.). These track exact internal states (e.g., `PrologByte::DEPTH_EXHAUSTED` or `MycinByte::RULE_CONFLICT`). All telemetry bits are bundled into a `#[repr(C, align(16))] KappaDetail16` struct, ensuring telemetry remains statically sized at precisely 16 bytes.