# AGENTS.md — `ccog` crate

Authoritative onboarding for agents (and humans) extending `ccog`. See
`CLAUDE.md` for the constitutional constraints; this file documents the
procedure for adding cognitive surfaces — bark slots, breeds, and field packs.

## Pack onboarding (Phase 12)

A "field pack" is a productized bundle of bark slots + reserved bit range +
admitted breeds + bias rule. Adding a pack is a six-step process.

### 1. Reserve a bit range

Edit `src/packs/bits.rs`. Append the new band as a `Range<u32>`. The band
MUST be:

- Disjoint from every other band (including 0–15 core).
- Contained in `0..64` (canonical mask domain).
- Follow the existing const-assert pattern:
  ```rust
  ccog_const_assert!(PREVIOUS_RANGE.end <= NEW_RANGE.start);
  ```

The current map is:

| Range | Owner |
|---|---|
| 0–15  | core (frozen) |
| 16–31 | Lifestyle / OT |
| 32–47 | Edge / Home |
| 48–55 | Enterprise |
| 56–63 | Dev / Agent Governance |

### 2. Create the pack file

Path: `src/packs/<name>.rs`.

Must declare:

- A `pub mod <Name>Bit { ... }` module of `pub const`s, each strictly inside
  the band; bracket the band membership with `ccog_const_assert!` lines.
- A zero-sized `pub struct <Name>Pack;` with `impl FieldPack for <Name>Pack`.
  All five associated constants (`NAME`, `ONTOLOGY_PROFILE`,
  `ADMITTED_BREEDS`, `POSTURE_RANGE`, `CONTEXT_RANGE`) plus
  `fn builtins() -> &'static [BarkSlot]`.
- A `pub static BUILTINS: &[BarkSlot] = &[ ... ];` const table with 4–6 slots.
  Each slot's `act` function MUST emit only IRIs starting with one of the
  prefixes in `PUBLIC_ONTOLOGY_PREFIXES` (PROV, schema.org, SHACL, xsd,
  `urn:blake3:`, `urn:ccog:vocab:`).
- A `pub fn select_instinct(snap, posture, ctx) -> AutonomicInstinct` bias
  wrapper that calls `crate::instinct::select_instinct_v0(...)` and applies
  pack-specific clamping. **Never returns a non-canonical variant** — packs
  bias the lattice; they do not extend it.

### 3. Wire the pack module

Edit `src/packs/mod.rs` and add `pub mod <name>;` near the existing pack
declarations. Re-export pack types via the public `packs::<name>` path; do
not re-export from the crate root unless the pack reaches GA.

### 4. Tests (positive + negative + boundary)

Path: `tests/pack_<name>_conformance.rs`.

Required test pattern:

- `pack_<name>_positive_*` — bias fires on the expected condition.
- `pack_<name>_negative_*` — bias does NOT fire on near-miss conditions.
- `pack_<name>_boundary_*` — the constitutional invariant (no PII, no
  auto-merge, no new variants, etc).

Cross-pack invariants in `tests/pack_namespace_isolation.rs` must already
pass after the new module is wired (the bit-overlap test enumerates every
pack — extend the array).

### 5. Bench

Path: `benches/pack_<name>_bench.rs`. Criterion harness with one or two
benchmarks: the `select_instinct` bias wrapper and a representative `act`
call. Annotate the bench as `FullProcess` tier per `CLAUDE.md` budget table.

Register the bench in `crates/ccog/Cargo.toml`:
```toml
[[bench]]
name = "pack_<name>_bench"
harness = false
```

### 6. CI / `cargo make` integration

The cross-cutting `pack-coverage` task in `Makefile.toml` runs every pack's
test binary in one shot. Adding a new pack does not require editing the
task; the test discovery picks up `tests/pack_*_conformance.rs` patterns.

## Adding a new bark slot

(Identical to the procedure already documented in `CLAUDE.md` "Adding a
hook / breed". No changes here.)

## Adding a new breed

(See `CLAUDE.md`. The breed must be wired exhaustively in
`breeds::strips::admit_breed` in one edit.)

## Hard rules (do not violate)

- Constitutional rules from `CLAUDE.md` "Do Not" section apply to all packs.
- Never delete a `*_does_not_*` / `*_emits_*` / `*_not_shacl_*` test, or the
  matching `*_boundary_*` test in `tests/pack_*_conformance.rs`.
- Never introduce `std::collections::HashMap` or `BTreeMap` in pack code —
  see `clippy.toml`.

## See also

- `CLAUDE.md` — crate constitution.
- `src/packs/bits.rs` — bit-range allocation source of truth.
- `tests/pack_namespace_isolation.rs` — cross-pack invariant suite.
