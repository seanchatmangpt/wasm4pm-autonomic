#!/usr/bin/env bash
# pictl-conformance-envelope.sh
# Usage: bash pictl-conformance-envelope.sh <log.xes> <operation_id> <envelope_id>
# Runs pictl conformance, signs result as a ReceiptEnvelope, appends to mcpp.envelope.chain.json
set -euo pipefail

LOG_PATH="${1:?log.xes path required}"
OPERATION_ID="${2:-obl-mcpp-full-six-stage-run-001}"
ENVELOPE_ID="${3:-recenv-pictl-$(date -u +%s)}"

GGEN="${GGEN_BIN:-/Users/sac/ggen/target/debug/ggen}"
PICTL="${PICTL_BIN:-$(command -v pictl 2>/dev/null || echo '/Users/sac/wasm4pm/apps/pictl/dist/bin/pictl.js')}"

if [ -f "${PICTL}" ] && find /Users/sac/wasm4pm/apps/pictl/src -name '*.ts' \
    -newer "${PICTL}" -print -quit 2>/dev/null | grep -q .; then
  echo "WARNING: pictl dist may be stale — consider running pnpm build first" >&2
fi

PRIVATE_KEY="${GGEN_KEY:-${XDG_CONFIG_HOME:-$HOME/.config}/ggen/portfolio.ed25519}"
PUBLIC_KEY="${PRIVATE_KEY}.pub"
CHAIN_FILE="${CHAIN_FILE:-.portfolio/receipts/mcpp.envelope.chain.json}"
OUTPUT_DIR=".portfolio/receipts"

mkdir -p "$OUTPUT_DIR"

if [ ! -x "${PICTL}" ] && ! node "${PICTL}" --version >/dev/null 2>&1; then
  echo "ERROR: pictl not found at ${PICTL}" >&2; exit 1
fi

VERDICT_FILE="$OUTPUT_DIR/pictl-conformance-$(date -u +%Y%m%dT%H%M%SZ).json"
ENVELOPE_FILE="$OUTPUT_DIR/$ENVELOPE_ID.envelope.json"

echo "==> pictl conformance: $LOG_PATH"
$PICTL conformance "$LOG_PATH" --format json > "$VERDICT_FILE"

echo "==> ggen envelope sign"
"$GGEN" envelope sign \
  --payload_path    "$VERDICT_FILE" \
  --payload_schema  chatmangpt.pictl.conformance.v1 \
  --producer_system wasm4pm \
  --producer_kind   pictl-conformance \
  --operation_id    "$OPERATION_ID" \
  --envelope_id     "$ENVELOPE_ID" \
  --private_key     "$PRIVATE_KEY" \
  --public_key_ref  "$PUBLIC_KEY" \
  --chain_file      "$CHAIN_FILE" \
  --output          "$ENVELOPE_FILE"

echo "==> ggen envelope verify"
"$GGEN" envelope verify \
  --envelope_file "$ENVELOPE_FILE" \
  --public_key    "$PUBLIC_KEY"

echo "==> done: $ENVELOPE_FILE"
