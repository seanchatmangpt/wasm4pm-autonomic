# CLAUDE.md — `ccog` crate

Compiled Cognition core: field-cognition facade over RDF graph closure with
nanoscale bark dispatch, deterministic receipts, POWL8/POWL64 motion ABI.

> ccog knows what the graph permits the field to do.
> `A = μ(O*)` — action is lawful only when projected from semantic closure.
> Pipeline: `O_raw + posture + history + context → O* → Decide → Materialize → Seal → Trace`.

## Build

Always scope to `-p ccog` — parent `dteam` crate has unrelated compile errors
(missing `wasm_bindgen`, `StaticPackedKeyTable`, stale bench arity). Do NOT
use `cargo make` from workspace root for ccog work.

| Goal | Command |
|------|---------|
| Build / test / bench | `cargo {build,test,bench} -p ccog` |
| Single test / bench tier | `cargo test -p ccog -- <name>` / `cargo bench -p ccog --bench ccog_hot_path_bench -- kernel_floor` |
| Lint | `cargo clippy -p ccog -- -D warnings` |

`#![forbid(unsafe_code)]` + `#![deny(missing_docs)]` everywhere. 170+ tests
must stay green.

## Module map

**Hot path (decide → materialize → seal):** `compiled.rs` (snapshot) ·
`compiled_hook.rs` (`Predicate` bits + `compute_present_mask`) ·
`bark_kernel.rs` (POWL8-ordered) · `bark_artifact.rs` (const `BUILTINS` table) ·
`admit.rs` (denial-polarity).

**Plan + ABI:** `powl.rs` (`Powl8`/`CompiledPowl8`) · `powl64.rs` (chain `path`,
polarity-folded BLAKE3, collision-preserving cells) · `construct8.rs` (≤8-triple
writeback) · `receipt.rs` (`derive_urn` + `canonical_material`).

**Cognition:** `breeds/{eliza,mycin,strips,shrdlu,prolog,hearsay,dendral}.rs` ·
`multimodal.rs` (`PostureBundle` + `ContextBundle`) · `instinct.rs`
(`select_instinct_v0` → `AutonomicInstinct`) · `trace.rs` (`CcogTrace`,
`BarkSkipReason`).

**Warm path:** `field.rs` · `graph.rs` (SPARQL + `quads_for_pattern`) · `hooks.rs`
(reference, NOT hot) · `facade.rs` · `runtime/{scheduler,delta,posture,step}.rs` ·
`verdict.rs` · `utils/dense.rs` (`PackedKeyTable` + FNV-1a).

`impl Default for PackPosture` lives in `trace.rs`, not `verdict.rs` — search both.

## Performance contract — never conflate tiers

| Tier | Measures | Target |
|------|----------|--------|
| KernelFloor | `decide()` only — no alloc | sub-µs |
| CompiledBark | `decide` + `materialize` | ≤5µs |
| Materialization | act fns over snapshot | ≤2µs |
| ReceiptPath | `seal()` BLAKE3 + URN | ≤5µs |
| FullProcess | `process_with_hooks` warm | ≤30µs |

`decide()` MUST stay alloc-free — no `Vec`/`format!`/`Utc::now`/`Construct8`/fn-ptr acts.

## Three identity surfaces — keep separate

1. **Receipt material** (`Receipt::canonical_material`): `hook_id || plan_node_le_u16
   || delta_bytes || field_id || prior_chain_or_zero32 || polarity`. NUL-separated.
   Never `Utc::now`, never bench tier.
2. **Trace material** (`CcogTrace`): present_mask, predecessor masks, typed
   `BarkSkipReason`.
3. **Bench tier** (`BenchmarkTier`): annotation only. Never hashed.

`Utc::now` allowed only as receipt metadata timestamp, never in URN material.

## Conformance rules (boundary detectors — load-bearing)

Tests like `*_does_not_fill_evidence`, `*_emits_ask_action_not_placeholder`,
`*_emits_was_informed_by`, `*_not_shacl_shape` encode discovered semantic
boundaries. **Do not delete when refactoring** — fix the refactor instead.

1. Missing-evidence hook MUST NOT fabricate evidence. Emit `schema:AskAction`
   gap-finding triples; never `<doc> prov:value "placeholder"`.
2. Phrase-binding emits `prov:wasInformedBy <urn:blake3:{hash(label)}>`; never
   `skos:definition "derived from prefLabel"`.
3. Transition-admissibility uses `prov:Activity` + `prov:used`; never
   `sh:targetClass` on instances.
4. Receipt activity IRIs MUST be `urn:blake3:` URNs (no `example.org` in
   production paths).
5. `HookRegistry::fire_matching` is warm/reference, NOT hot. Hot loops use
   `BarkKernel` / `bark_artifact::bark`.

Public ontologies only: `rdf`, `xsd`, `skos`, `schema`, `prov`, `dcterms`, `sh`,
`odrl`, plus `urn:blake3:{hex}` and `urn:ccog:*`. No `ccog:` namespace prefix.

Policy grep (expects 0 hits in production paths):

```bash
grep -rn 'stub\|todo!\|unimplemented!\|FIXME\|XXX' crates/ccog/src/ \
  | grep -v -E 'tests/|#\[cfg\(test\)\]|MUST NOT|"placeholder"|category error|sentinel|no placeholder'
```

## Use Rust nightly features when they improve the implementation

The user wants real production code and accepts a nightly toolchain when it
materially helps. Pin via `rust-toolchain.toml`. Keep `#![forbid(unsafe_code)]`.

Adopt where the win is concrete (each `#![feature(...)]` needs a one-line
comment naming the win — "consistency" / "future-proofing" is not enough):

- `const_trait_impl` — const `BitAnd`/`From` for mask helpers.
- `generic_const_exprs` — type-level mask widths replace `MAX_NODES`.
- `portable_simd` — SIMD lanes for `compute_present_mask` on wider snapshots.
- `let_chains` — flatten the `select_instinct_v0` lattice.
- `stmt_expr_attributes` — `#[inline(always)]` on expressions in bark hot path.
- `test` + `bench_black_box` — replace criterion for sub-µs `KernelFloor` timing.

Apply to hot path first (`bark_kernel.rs`, `bark_artifact.rs`, `compiled.rs`,
`compiled_hook.rs`, `admit.rs`, `powl.rs`). Stable always preferred when it
matches the nightly version.

## Code conventions

- Hot path is alloc-free; decide returns fixed-size structs.
- Const fns where possible (`admit*`, mask helpers, `GlobeCell::new`).
- Prefer `HookCheck::SnapshotFn`/`SnapshotAdmit` over `Fn`/`Admit`.
- Receipt URNs from canonical material, not timestamps; chain folds polarity
  at every step (genesis included).
- u64 masks for plan-node domains; full URLs (never prefix strings).

## Adding a hook / breed

**Hook:** add `Predicate` bit → update `compute_present_mask` → add
`BarkSlot` to `BUILTINS` → write `fn(&CompiledFieldSnapshot) -> Result<Construct8>`
emitting only public-ontology / `urn:blake3` IRIs → add a negative test
asserting the hook does NOT fabricate facts.

**Breed:** add `verdict::Breed` variant with `repr(u8)` → wire precondition in
`breeds::strips::admit_breed` (exhaustive match) → implement `breeds/<name>.rs` →
positive + negative admissibility tests.

## Parallel agent dispatch

- Strict file-disjoint ownership — one agent per file.
- Agents that stash WIP outside their ownership lose other agents' work in flight.
- Verify with `wc -l` / `Read` before trusting "completed" reports.
- Commit early in parent so reverts are recoverable.
- Use `isolation: "worktree"` for deeply overlapping work.

## Gotchas

- `dev-worktree` submodule always shows modified (internal dirty state, pointer
  unchanged). Leave alone.
- Branch `feat/ccog-phase-4-compiled-field` contains Phases 4 + 5 + 6 — name is stale.
- system-reminder file-modification notes can be from running agents (content
  reverts as agents stash). Re-check, don't panic.

## Do Not (Phase 12 field packs)

1. Do not fork `AutonomicInstinct` per pack — packs bias the canonical
   lattice; they never introduce new response classes. The lattice is
   constitutional.
2. Do not put PII in mask names or emitted IRIs. Pack act fns emit only
   `urn:blake3:` URNs of interpreter-issued tokens (and other public
   ontologies) — never visitor IDs, addresses, emails, or raw user text.
3. Do not auto-merge dev pack actions. The dev pack clamps `Refuse` and
   `Escalate` to `Ask`; it must always surface for human review.
4. Do not allocate posture/context bits outside the pack's reserved range.
   Bands are: 0–15 core, 16–31 lifestyle, 32–47 edge, 48–55 enterprise,
   56–63 dev. Cross-band collisions are caught by `const_assert!` —
   silencing them is a violation.
5. Do not introduce new ontology namespaces beyond the public allowlist
   (PROV / schema.org / SHACL / xsd / `urn:blake3:` / `urn:ccog:vocab:`).
   New namespaces require an explicit constitutional amendment.

## See also

- `/Users/sac/dteam/CLAUDE.md` — parent `dteam` crate
- `/Users/sac/dteam/docs/COMPILED_COGNITION.md` — architecture paper
- `/Users/sac/.claude/plans/yes-i-think-synchronous-squirrel.md` — current phase plan
- `crates/ccog/AGENTS.md` — pack onboarding procedure
