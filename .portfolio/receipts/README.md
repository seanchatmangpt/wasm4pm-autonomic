# Portfolio Receipts

This directory holds **audit-trail receipts** for proof-gated operations in dteam.
Files here are **intentionally tracked** (see the `!.portfolio/receipts/*.json`
allow-list rules in `.gitignore`); they are evidence, not build debris.

## File kinds

| Pattern | Meaning |
|---|---|
| `sr-mcpp-<UTC-timestamp>.json` | Skill-runner MCPP run capture: one JSON per `sr verify mcpp` invocation. Records `operation_id`, `timestamp`, `input_hashes`, `output_hashes`, `previous_receipt_hash`, and a `signature` field (currently `"unsigned"` pending key infrastructure). Forms an append-only chain. |
| `mcpp.chain.json` | Canonical MCPP chain head — the latest verified-state pointer for the MCPP capability. |
| `mcpp.envelope.chain.json` | Envelope-format chain head; pairs each chain entry with its serialized envelope. |
| `obl-*.receipt.json` | Per-obligation receipt: completion proof for a named portfolio obligation (`obl-mcpp-e2e-receipted-run-001`, `obl-mcpp-receipted-capability-composition-001`, etc.). |
| `obl-*.envelope.json` | Envelope wrapping the corresponding obligation receipt for chain inclusion. |
| `ralph-plan-*.json` | Plan-level proofs from the Ralph orchestrator (kernel-proof + spec receipts). |

## Retention

Receipts are **append-only**. Never edit a receipt after it is written; produce a
new receipt that references the prior one via `previous_receipt_hash`.

## Regeneration

Receipts are produced by the `sr` binary and the Ralph orchestrator:

```bash
cargo run --bin sr -- verify mcpp <args>      # emits sr-mcpp-<timestamp>.json
cargo run --bin ralph -- plan <args>          # emits ralph-plan-*.json
```

Hashes are BLAKE3 over canonicalized JSON (JCS).

## Schema pointers

The chain envelope schemas live alongside the data: `mcpp.envelope.chain.json` is
both an instance and its own structural reference for envelope shape.
