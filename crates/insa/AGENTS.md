# INSA AGENTS.md Operating Contract

This repository is governed by the Toyota Code Production System (TCPS) and the INSA Byte Law. 
As an AI agent operating in this repository, you must strictly adhere to the following rules. 
Failure to follow these rules is considered a critical defect and an architectural violation.

## 1. The Meaning of "Done"
- **No fake completion:** You must not claim a task is "done" just because the code compiles or looks plausible.
- **Evidence-first completion:** "Done" means evidentiary operational closure. It requires: closed field + lawful motion + bounded delta + canonical evidence + deterministic replay.
- **Stop the line:** Treat narrative-only proof as a defect. If a route cannot be proven, block it.
- **Completion Format:** Every completion report must include: Summary, Files changed, Commands run, Evidence (e.g., `just unrdf-check`, `doctor check --scope ontology,generated`, `truthforge ontology`), Blocked/unknown, and Next required gate.

## 2. Core Architectural Laws
- **Do not preserve exploration debt:** Delete code that exists only as exploration.
- **Do not treat MCP/A2A/HITL as cognition:** They are bounded projections after local instinct resolution fails to close.
- **Do not let projection results mutate state:** They must re-enter as `Observation -> CONSTRUCT8 -> O*`.
- **Do not widen Need9 first:** `Need9` means decompose, sequence, compose, or add another byte lane. It does not mean widen to `u16` or `Vec`.
- **Do not emit without proof:** Unproofed emission is structurally forbidden.
- **Do not report without replay:** Board/security reports are derived from POWL64 replayable evidence, not generated prose.
- **Do not use stable/nightly as authority boundary:** The boundary is `admitted` vs `unadmitted`. Nightly/SIMD/intrinsic paths are permitted when they provide a control edge and match the `ReferenceLawPath`.

## 3. unrdf Manufacturing Rules
`unrdf` is the ontology-to-artifact manufacturing station.
- **Do not edit generated artifacts by hand:** Generated outputs (in `crates/insa-generated/`, `docs/generated/`, etc.) are read-only manufactured parts.
- **Do not wrap unrdf with INSA CLI commands:** Use `just unrdf-sync` and `just unrdf-check`.
- **Do not generate hot-path machine law from ontology templates:** `unrdf` generates semantic projections (IRI constants, docs, Zod schemas, failure catalogs). Handcraft hot law.
- **Do not put ontology selection logic inside Nunjucks:** SPARQL selects; Nunjucks projects.
- **Do not claim done if generated artifacts are stale:** You must run `just unrdf-check`.
- **Atomic Commits:** Do not modify generated output without committing the source ontology/query/template changes together.

## 4. CLI Rules (`clap-noun-verb`)
`clap-noun-verb` provides the operational command grammar.
- **Thin CLI Wrappers:** Allowed: parse arguments, build request structs, call domain functions, return serializable output, map status to exit code. Forbidden: implementing domain law in CLI wrappers.
- **No filesystem mutation without plan/receipt:** Commands like `wizard apply` or `telco bind` must follow the plan/preview/apply/receipt/verify pattern.
- **No prose-only output for primary commands:** Output must be structured JSON.
- **Exit codes are mandatory:** Commands must map status to standard exit codes (0 = OK, 1 = BLOCKED, 2 = UNKNOWN, etc.).
- **No hidden interactive prompts in CI paths:** Use `--dry-run` and `--yes` for mutations.

## 5. Telco Rules (Communication Service Assurance)
`telco` is the provisioned communication service layer, not ping/curl.
- **Control ⊥ Data ⊥ Proof:** Separate route authority (control), payload (data), and receipt (proof).
- **Never allow in-band payload to become out-of-band control:** A tool/agent payload cannot dictate route or authority.
- **Treat reachability as necessary but insufficient:** An endpoint must handshake, validate schema, prove authority, and produce a receipt.
- **Classify all faults:** Isolate failures by layer (e.g., `EndpointUnreachable`, `SchemaMismatch`, `ReceiptMissing`).
- **All external paths are provisioned services:** They must define target, physical endpoint, capability, transport, schema, authority, timeout, fallback, and receipt policy.
