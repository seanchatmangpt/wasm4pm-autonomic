#!/usr/bin/env bash
# obl-mcpp-e2e-receipted-run-001 — minimal receipted spine
# Default kind:  automl     (Stage 1 = ingest existing AutomlPlan corpus)
# Alt kind:      ralph-plan (Stage 1 = ralph run → emit RalphPlan corpus)
#
# Stage 2: dteam doctor verification (--json [--kind=K])
# Stage 3: ggen receipt sign + chain  (legacy receipt chain)
# Stage 3b: ggen envelope sign        (ReceiptEnvelope unifier chain)
# Verify : ggen receipt verify + chain_verify
#          ggen envelope verify + chain_verify (envelope chain)
#
# Spine succeeds on any doctor verdict (pass/healthy/soft_fail/fatal).
# Spine fails only if a stage breaks plumbing or the receipt/envelope is invalid.

set -euo pipefail

# ── CLI ─────────────────────────────────────────────────────────────────────
KIND="automl"
for arg in "$@"; do
  case "$arg" in
    --kind=*)        KIND="${arg#--kind=}" ;;
    --kind)          shift || true; KIND="${1:-automl}" ;;
    -h|--help)
      cat <<USAGE
usage: run-mcpp-spine.sh [--kind=automl|ralph-plan]

  --kind=automl       (default) Stage 1 ingests artifacts/pdc2025/automl_plans
  --kind=ralph-plan             Stage 1 runs ralph in --test mode and emits
                                RalphPlan JSONs at artifacts/ralph/ralph_plans
USAGE
      exit 0
      ;;
  esac
done
case "$KIND" in
  automl|ralph-plan) ;;
  *) echo "unknown --kind: $KIND" >&2; exit 2 ;;
esac

OBLIGATION="obl-mcpp-e2e-receipted-run-001-${KIND}"
DTEAM_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GGEN_BIN="${GGEN_BIN:-${HOME}/ggen/target/debug/ggen}"
XDG="${XDG_CONFIG_HOME:-$HOME/.config}/ggen"
PRIV_KEY="${XDG}/portfolio.ed25519"
PUB_KEY="${XDG}/portfolio.ed25519.pub"

RUN_DIR="${DTEAM_ROOT}/.portfolio/runs/${OBLIGATION}"
RECEIPT_DIR="${DTEAM_ROOT}/.portfolio/receipts"
PLANS_DST="${RUN_DIR}/plans"
VERDICT_FILE="${RUN_DIR}/doctor-verdict.json"
UNSIGNED_RECEIPT="${RUN_DIR}/unsigned-receipt.json"
SIGNED_RECEIPT="${RECEIPT_DIR}/${OBLIGATION}.receipt.json"
CHAIN_FILE="${RECEIPT_DIR}/mcpp.chain.json"
ENVELOPE_CHAIN_FILE="${RECEIPT_DIR}/mcpp.envelope.chain.json"
OBL_ID="obl-receipt-format-unifier-001"

log() { echo "[spine] $*" >&2; }
fail() { echo "[spine] FAIL: $*" >&2; exit 1; }

# ── Prerequisites ───────────────────────────────────────────────────────────
[ -x "${GGEN_BIN}" ] || fail "ggen binary not found at ${GGEN_BIN} (set GGEN_BIN env)"
[ -f "${PRIV_KEY}" ] || fail "portfolio signing key missing at ${PRIV_KEY}"
[ -f "${PUB_KEY}"  ] || fail "portfolio public key missing at ${PUB_KEY}"

mkdir -p "${RUN_DIR}" "${RECEIPT_DIR}"

# ── Stage 1: produce plan corpus ───────────────────────────────────────────
if [ "${KIND}" = "ralph-plan" ]; then
  log "Stage 1: ralph --test → emit RalphPlans"
  RALPH_OUT="${RUN_DIR}/ralph_plans"
  rm -rf "${RALPH_OUT}" "${PLANS_DST}"
  mkdir -p "${RALPH_OUT}"
  # IDEAS.md is the input list ralph iterates over.
  if [ ! -f "${DTEAM_ROOT}/IDEAS.md" ]; then
    printf "1. Health check endpoint\n2. Add logging to autonomic cycle\n" \
      > "${DTEAM_ROOT}/IDEAS.md"
  fi
  cd "${DTEAM_ROOT}"
  cargo run --bin ralph --quiet -- \
    --test --concurrency 1 \
    --plans-out "${RALPH_OUT}" >/dev/null 2>&1 || true
  # Promote the emitted plans into the run plans dir for hashing/verification.
  mkdir -p "${PLANS_DST}"
  cp -R "${RALPH_OUT}/." "${PLANS_DST}/"
else
  PLANS_SRC="${DTEAM_ROOT}/artifacts/pdc2025/automl_plans"
  [ -d "${PLANS_SRC}" ] || fail "plan corpus missing at ${PLANS_SRC}"
  log "Stage 1: ingest plan corpus from ${PLANS_SRC}"
  rm -rf "${PLANS_DST}"
  mkdir -p "${PLANS_DST}"
  cp -R "${PLANS_SRC}/." "${PLANS_DST}/"
fi
PLAN_COUNT=$(find "${PLANS_DST}" -name '*.json' | wc -l | tr -d ' ')
log "  plans available: ${PLAN_COUNT}"
[ "${PLAN_COUNT}" -gt 0 ] || fail "no plans produced for kind=${KIND}"

# Hash each plan file (SHA-256 → hex) for receipt input_hashes
INPUT_HASHES_JSON=$(find "${PLANS_DST}" -name '*.json' | sort | while read -r f; do
  shasum -a 256 "$f" | awk '{print "\""$1"\""}'
done | paste -sd, -)
INPUT_HASHES_JSON="[${INPUT_HASHES_JSON}]"

# ── Stage 2: doctor verification ───────────────────────────────────────────
log "Stage 2: dteam doctor --json --kind=${KIND} --plans-dir ${PLANS_DST}"
set +e
cd "${DTEAM_ROOT}"
cargo run --bin doctor --quiet -- \
  --json --kind="${KIND}" --plans-dir="${PLANS_DST}" \
  > "${VERDICT_FILE}"
DOCTOR_EXIT=$?
set -e
log "  doctor exit code: ${DOCTOR_EXIT} (0=healthy, 1=soft_fail, 2=fatal)"
[ -s "${VERDICT_FILE}" ] || fail "doctor produced empty verdict"

VERDICT=$(grep -oE '"verdict": "[^"]+"' "${VERDICT_FILE}" | head -1 | cut -d'"' -f4)
log "  verdict: ${VERDICT}"

# Hash the verdict file for receipt output_hashes
VERDICT_HASH=$(shasum -a 256 "${VERDICT_FILE}" | awk '{print $1}')

# ── Stage 3: build unsigned receipt + sign + chain ─────────────────────────
log "Stage 3: build unsigned receipt → sign → chain"

NOW=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

cat > "${UNSIGNED_RECEIPT}" <<EOF
{
  "operation_id": "${OBLIGATION}",
  "timestamp": "${NOW}",
  "input_hashes": ${INPUT_HASHES_JSON},
  "output_hashes": ["${VERDICT_HASH}"],
  "signature": "",
  "previous_receipt_hash": null
}
EOF

# Pass --output to write the signed receipt to its final location.
# --chain-file links to the previous chain entry (if any) and appends the signed receipt.
"${GGEN_BIN}" receipt sign \
  --receipt_file "${UNSIGNED_RECEIPT}" \
  --private_key "${PRIV_KEY}" \
  --chain_file "${CHAIN_FILE}" \
  --output "${SIGNED_RECEIPT}" \
  > "${RUN_DIR}/sign-summary.json"

log "  signed receipt: ${SIGNED_RECEIPT}"
log "  chain file:    ${CHAIN_FILE}"

# ── Verify round-trip ─────────────────────────────────────────────────────
log "Verify: ggen receipt verify"
"${GGEN_BIN}" receipt verify \
  --receipt_file "${SIGNED_RECEIPT}" \
  --public_key "${PUB_KEY}" \
  > "${RUN_DIR}/verify-summary.json"

VERIFY_OK=$(grep -oE '"is_valid":[[:space:]]*(true|false)' "${RUN_DIR}/verify-summary.json" | head -1 | grep -oE 'true|false')
[ "${VERIFY_OK}" = "true" ] || fail "receipt verify failed (is_valid=${VERIFY_OK:-<missing>})"
log "  receipt verify: ok"

log "Verify: ggen receipt chain_verify"
"${GGEN_BIN}" receipt chain_verify \
  --chain_file "${CHAIN_FILE}" \
  --public_key "${PUB_KEY}" \
  > "${RUN_DIR}/chain-verify-summary.json"

CHAIN_OK=$(grep -oE '"is_valid":[[:space:]]*(true|false)' "${RUN_DIR}/chain-verify-summary.json" | head -1 | grep -oE 'true|false')
[ "${CHAIN_OK}" = "true" ] || fail "chain-verify failed (is_valid=${CHAIN_OK:-<missing>})"
log "  chain-verify: ok"

# ── Stage 3b: envelope sign (ReceiptEnvelope unifier chain) ───────────────
log "Stage 3b: ggen envelope sign → chain"

ENVELOPE_TS=$(date -u +"%s")
ENVELOPE_ID="recenv-${KIND}-${ENVELOPE_TS}"
ENVELOPE_FILE="${RECEIPT_DIR}/${OBL_ID}.${KIND}.envelope.json"

"${GGEN_BIN}" envelope sign \
  --payload_path "${VERDICT_FILE}" \
  --payload_schema chatmangpt.doctor.verdict.v1 \
  --producer_system dteam \
  --producer_kind doctor-verdict \
  --operation_id "${OBL_ID}" \
  --envelope_id "${ENVELOPE_ID}" \
  --private_key "${PRIV_KEY}" \
  --public_key_ref "${PUB_KEY}" \
  --chain_file "${ENVELOPE_CHAIN_FILE}" \
  --output "${ENVELOPE_FILE}" \
  > "${RUN_DIR}/envelope-sign-summary.json" \
  || fail "envelope sign failed"

log "  signed envelope: ${ENVELOPE_FILE}"
log "  envelope chain:  ${ENVELOPE_CHAIN_FILE}"

# ── Verify envelope round-trip ─────────────────────────────────────────────
log "Verify: ggen envelope verify"
"${GGEN_BIN}" envelope verify \
  --envelope_file "${ENVELOPE_FILE}" \
  --public_key "${PUB_KEY}" \
  > "${RUN_DIR}/envelope-verify-summary.json" \
  || fail "envelope verify failed"

ENV_VERIFY_OK=$(grep -oE '"is_valid":[[:space:]]*(true|false)' "${RUN_DIR}/envelope-verify-summary.json" | head -1 | grep -oE 'true|false')
[ "${ENV_VERIFY_OK}" = "true" ] || fail "envelope verify failed (is_valid=${ENV_VERIFY_OK:-<missing>})"
log "  envelope verify: ok"

log "Verify: ggen envelope chain_verify"
"${GGEN_BIN}" envelope chain_verify \
  --chain_file "${ENVELOPE_CHAIN_FILE}" \
  --public_key "${PUB_KEY}" \
  > "${RUN_DIR}/envelope-chain-verify-summary.json" \
  || fail "envelope chain_verify failed"

ENV_CHAIN_OK=$(grep -oE '"is_valid":[[:space:]]*(true|false)' "${RUN_DIR}/envelope-chain-verify-summary.json" | head -1 | grep -oE 'true|false')
[ "${ENV_CHAIN_OK}" = "true" ] || fail "envelope chain_verify failed (is_valid=${ENV_CHAIN_OK:-<missing>})"
log "  envelope chain-verify: ok"

# --- Stage 4: pictl conformance (wasm4pm) ---
if [ -n "${LOG_PATH:-}" ] && [ -f "${LOG_PATH:-}" ]; then
  log "Stage 4: pictl conformance → envelope"
  bash scripts/pictl-conformance-envelope.sh \
    "$LOG_PATH" \
    "$OBL_ID" \
    "recenv-pictl-${KIND}-$(date -u +%s)" \
    || fail "Stage 4: pictl conformance envelope failed"
else
  log "Stage 4: SKIP — set LOG_PATH=<path.xes> to enable"
fi

# ── Stage 5: ostar evidence → envelope ──────────────────────────────────────
if [ "${SKIP_OSTAR_STAGE:-0}" = "1" ]; then
  log "Stage 5: SKIP — SKIP_OSTAR_STAGE=1"
else
  log "Stage 5: emit ostar evidence → envelope sign → verify"
  if ! command -v python3 >/dev/null 2>&1; then
    fail "Stage 5: python3 not in PATH — required by emit-evidence.sh"
  fi
  log "Stage 5: NOTE — evidence is synthetic (no live ostar run); stage_id=mcpp-spine-synthetic"

  OSTAR_OBL_ID="obl-mcpp-full-six-stage-run-001"  # NOT the same as OBL_ID above
  OSTAR_TS=$(date -u +"%s")
  OSTAR_ENVELOPE_ID="recenv-ostar-${KIND}-${OSTAR_TS}"
  OSTAR_EVIDENCE_FILE="${RUN_DIR}/ostar-evidence.json"
  OSTAR_ENVELOPE_FILE="${RECEIPT_DIR}/${OSTAR_OBL_ID}.ostar-evidence.envelope.json"

  bash /Users/sac/chatmangpt/ostar/schemas/emit-evidence.sh \
    "mcpp-spine-synthetic" "${OSTAR_OBL_ID}" "pass" \
    "0.0" "0.0" "0.0" "0.0" \
    > "${OSTAR_EVIDENCE_FILE}" || fail "Stage 5: emit-evidence.sh failed"

  "${GGEN_BIN}" envelope sign \
    --payload_path    "${OSTAR_EVIDENCE_FILE}" \
    --payload_schema  chatmangpt.ostar.evidence.v1 \
    --producer_system ostar \
    --producer_kind   ostar-stage-evidence \
    --operation_id    "${OSTAR_OBL_ID}" \
    --envelope_id     "${OSTAR_ENVELOPE_ID}" \
    --private_key     "${PRIV_KEY}" \
    --public_key_ref  "${PUB_KEY}" \
    --chain_file      "${ENVELOPE_CHAIN_FILE}" \
    --output          "${OSTAR_ENVELOPE_FILE}" \
    > "${RUN_DIR}/ostar-envelope-sign-summary.json" \
    || fail "Stage 5: envelope sign failed"

  "${GGEN_BIN}" envelope verify \
    --envelope_file "${OSTAR_ENVELOPE_FILE}" \
    --public_key    "${PUB_KEY}" \
    > "${RUN_DIR}/ostar-envelope-verify-summary.json" \
    || fail "Stage 5: envelope verify failed"

  OSTAR_OK=$(grep -oE '"is_valid":[[:space:]]*(true|false)' \
    "${RUN_DIR}/ostar-envelope-verify-summary.json" | head -1 | grep -oE 'true|false')
  [ "${OSTAR_OK}" = "true" ] || fail "Stage 5: is_valid=${OSTAR_OK:-<missing>}"
  log "  Stage 5: ok — ${OSTAR_ENVELOPE_FILE}"
fi

# ── Proof gate ───────────────────────────────────────────────────────────────
if [ "${SKIP_OSTAR_STAGE:-0}" != "1" ]; then
  log "Proof gate: chain_verify (expect envelope_count >= 4)"
  "${GGEN_BIN}" envelope chain_verify \
    --chain_file "${ENVELOPE_CHAIN_FILE}" \
    --public_key "${PUB_KEY}" \
    > "${RUN_DIR}/proof-gate-chain-verify.json" \
    || fail "Proof gate: chain_verify failed"
  GATE_COUNT=$(grep -oE '"envelope_count":[[:space:]]*[0-9]+' \
    "${RUN_DIR}/proof-gate-chain-verify.json" | grep -oE '[0-9]+$')
  GATE_VALID=$(grep -oE '"is_valid":[[:space:]]*(true|false)' \
    "${RUN_DIR}/proof-gate-chain-verify.json" | head -1 | grep -oE 'true|false')
  [ "${GATE_VALID}" = "true" ] || fail "Proof gate: chain invalid"
  [ "${GATE_COUNT:-0}" -ge 4 ] || fail "Proof gate: envelope_count=${GATE_COUNT}, need >= 4"
  log "  Proof gate: PASSED (envelope_count=${GATE_COUNT})"
fi

# ── Done ──────────────────────────────────────────────────────────────────
echo
log "SPINE COMPLETE"
log "  obligation:             ${OBLIGATION}"
log "  doctor verdict:         ${VERDICT}"
log "  signed receipt:         ${SIGNED_RECEIPT}"
log "  chain file:             ${CHAIN_FILE}"
log "  sign summary:           ${RUN_DIR}/sign-summary.json"
log "  verify summary:         ${RUN_DIR}/verify-summary.json"
log "  chain summary:          ${RUN_DIR}/chain-verify-summary.json"
log "  signed envelope:        ${ENVELOPE_FILE}"
log "  envelope chain file:    ${ENVELOPE_CHAIN_FILE}"
log "  envelope sign summary:  ${RUN_DIR}/envelope-sign-summary.json"
log "  envelope verify:        ${RUN_DIR}/envelope-verify-summary.json"
log "  envelope chain summary: ${RUN_DIR}/envelope-chain-verify-summary.json"
echo

# Print final receipt-verify JSON to stdout for downstream consumers.
cat "${RUN_DIR}/verify-summary.json"
