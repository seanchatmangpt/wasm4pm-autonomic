## Open Gaps — What Closes Each Follow-On Obligation

_Written 2026-04-26 from current codebase state._

---

### obl-ralph-plan-emission-001
**Gap:** Ralph emits structurally valid `chatmangpt.ralph.plan.v1` JSON, but phase completion is determined by whether artifact files exist on disk at emit time — not by whether the spec-kit phases actually ran to completion during the current run.

In test mode (`--test`), absent artifacts are classified as `skipped` (not `blocked`), so a run that never touched `plan.md` or `tasks.md` emits `verdict: soft_fail` with `SKIPPED_PHASES`. In real mode, absent files are `blocked` — which is honest, but the hashes (`constitution_hash`, `spec_hash`) are SHA-256 of whatever file exists at emit time, not of the artifacts produced _during_ this specific run.

**What closes it:**
1. Ralph must record a per-idea phase journal: when a phase runner completes, stamp `{phase, completed_at, artifact_path, artifact_hash}` into an in-memory struct before calling `emit_ralph_plan`.
2. Pass that journal into `emit_ralph_plan` instead of re-scanning the disk at emit time.
3. Remove the `is_test` branch from `emit_ralph_plan` — test mode should use the same journal path; tests should just pre-populate the journal with real or synthetic entries.
4. The `constitution_hash` must be the hash of the constitution file that was _loaded_ at run start, not at emit time (move the hash call to `main` initialization).

**Proof gate:** `cargo run --bin ralph` with no `--test` flag, one idea, produces a plan JSON where `verdict: "pass"` and all four phase artifacts exist and their hashes match SHA-256 of the actual files. Doctor reports zero pathologies.

---

### obl-wasm4pm-cli-wrapper-001
**Gap:** `pictl conformance` exists and emits JSON (`--format json`), but its output schema is not declared, not versioned, and not wrappable by `ggen envelope sign` today because:
1. The JSON output has no `schema` discriminator field (just a raw `{status, fitness, precision, ...}` object).
2. `pictl` is a Node.js/TypeScript CLI; the spine script would need `node` or `npx pictl` in the path and a way to reliably invoke it from a Bash script.
3. No receipt/envelope emission is wired inside `pictl conformance` itself.

**What closes it:**
1. Add `schema: "chatmangpt.pictl.conformance.v1"` to the JSON result object in `~/wasm4pm/apps/pictl/src/commands/conformance.ts` (one field addition).
2. Declare the schema as a JSON Schema 2020-12 document alongside the command (matches the pattern used for `chatmangpt.ralph.plan.v1`).
3. Add a thin Bash wrapper `scripts/pictl-conformance.sh` that: (a) invokes `pictl conformance <log> --format json > output.json`, (b) calls `ggen envelope sign` wrapping that output.
4. Wire the wrapper into `run-mcpp-spine.sh` as Stage 4.

**Proof gate:** `bash scripts/pictl-conformance.sh <log.xes>` produces a signed envelope whose `producer.kind` is `pictl-conformance`, `payload.schema` is `chatmangpt.pictl.conformance.v1`, and `ggen envelope verify` returns `is_valid: true`.

---

### obl-ostar-json-contract-001
**Gap:** ostar-proto has no declared evidence schema. There is no `chatmangpt.ostar.evidence.v1` JSON Schema defining what ostar emits as proof of closure for a manufacturing stage. Without this, `ggen envelope sign` cannot assign a meaningful `payload_schema`, and the envelope chain cannot distinguish ostar evidence from any other artifact.

Current state: ostar appears to operate as a TypeScript monorepo with process-mining algorithms compiled to WASM. There is no `ostar-proto` package or evidence emission surface visible in `~/chatmangpt/ostar/`.

**What closes it:**
1. Define `chatmangpt.ostar.evidence.v1` — a JSON Schema 2020-12 document covering at minimum: `schema`, `stage_id`, `operation_id`, `verdict` (`pass|soft_fail|fatal`), `metrics` (fitness, precision, simplicity, generalization), `artifacts[]` (paths + hashes).
2. Add an emit function (in whatever language ostar's stage runners use) that writes this JSON at stage completion.
3. Add `ggen envelope sign --payload_schema chatmangpt.ostar.evidence.v1 ...` to the spine script as Stage 5.

**Proof gate:** ostar runs a manufacturing stage, emits `ostar-evidence.json` conforming to the schema, and `ggen envelope chain_verify` on the extended chain returns `envelope_count: N+1` where N is the current chain length.

---

### obl-mcpp-full-six-stage-run-001
**Gap:** Blocked by all three above. The declared six-stage spine is:

| Stage | Producer | Schema | Status |
|---|---|---|---|
| 1 | dteam ralph | `chatmangpt.ralph.plan.v1` | partial (plan emitted but not from live run) |
| 2 | dteam doctor | `chatmangpt.doctor.verdict.v1` | **done** |
| 3 | ggen receipt | legacy Receipt chain | **done** |
| 3b | ggen envelope | `chatmangpt.receipt.envelope.v1` | **done** |
| 4 | pictl conformance | `chatmangpt.pictl.conformance.v1` | no schema discriminator yet |
| 5 | ostar evidence | `chatmangpt.ostar.evidence.v1` | schema not declared |
| 6 | ggen-a2a-mcp | — | not wired to mcpp-cli |

Stage 6 (ggen-a2a-mcp ↔ mcpp-cli) is an independent gap: `mcpp-cli` exists but is not wired to receive envelope-chain events and is not registered as an MCP server that `ggen` knows about.

**What closes it:** obl-ralph-plan-emission-001 + obl-wasm4pm-cli-wrapper-001 + obl-ostar-json-contract-001 must all land first. Then a single spine script update adds Stages 4 and 5, and an mcpp-cli MCP registration closes Stage 6.

**Proof gate:** `run-mcpp-spine.sh` exits 0, `mcpp.envelope.chain.json` has 5 entries, all `is_valid: true`, genesis `producer.kind = doctor-verdict-automl`, latest `producer.kind = ostar-evidence`.
