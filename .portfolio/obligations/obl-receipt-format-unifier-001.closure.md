# obl-receipt-format-unifier-001 — Receipt Format Unifier: ReceiptEnvelope v1

**Status:** CLOSED
**Date:** 2026-04-26

## What Was Built

ReceiptEnvelope v1 (`chatmangpt.receipt.envelope.v1`) is a schema-discriminated wrapper that lets any producer emit a receipt without modifying its native payload structure. Each envelope carries a producer identity `{system, kind}`, a `PayloadRef` containing a BLAKE3 content hash and optional path, an Ed25519 signature over the zeroed-chain form, and an `EnvelopeChainLink` whose `own_hash` is computed after signing so the signature is part of the envelope's identity. The chain links multiple receipts in sequence without merging their payloads.

## Envelope Rule Honored

Wrapper preserves producer-native payloads and hashes them; never blends schemas.

## Producer Kinds Now Chainable

| kind | system |
|------|--------|
| `doctor-verdict-automl` | dteam |
| `doctor-verdict-ralph-plan` | dteam |
| `ucausal-receipt` | (future) |
| `conformance-result` | (future) |

## CLI Surface

Binary: `ggen`

| Verb | Flags |
|------|-------|
| `ggen envelope sign` | `--payload_path`, `--schema`, `--system`, `--kind`, `--signing_key`, `--previous_envelope` |
| `ggen envelope verify` | `--envelope_path`, `--verifying_key` |
| `ggen envelope chain_verify` | `--envelope_dir`, `--verifying_key` |

All flags use underscores. Output is JSON to stdout.

## Files Delivered

- `~/ggen/crates/ggen-receipt/src/envelope.rs` — all types: `ReceiptEnvelope`, `EnvelopeChain`, `PayloadRef`, `ProducerIdentity`, `EnvelopeChainLink`
- `~/ggen/crates/ggen-cli/src/cmds/envelope.rs` — CLI verbs sign, verify, chain_verify
- `~/ggen/crates/ggen-receipt/src/lib.rs` — envelope module export
- `~/ggen/crates/ggen-cli/src/cmds/mod.rs` — envelope module registration

## What This Unlocks

- `obl-wasm4pm-cli-wrapper-001` — WASM4PM artifacts can be signed and chained as first-class receipts
- `obl-ostar-json-contract-001` — O* JSON contracts can reference envelope hashes as provenance anchors
- `obl-mcpp-full-six-stage-run-001` — MCPP six-stage run can emit per-stage receipts that chain into a single auditable sequence

## DO NOT FLATTEN

This envelope does NOT merge, coerce, or re-serialize producer payloads. The envelope references a payload by hash and path; it never includes payload fields at the envelope level. Any future producer that is added must provide its own schema and its own native serialization. The envelope layer remains a pointer-and-proof layer only.
