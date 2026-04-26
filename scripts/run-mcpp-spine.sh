#!/usr/bin/env bash
# obl-mcpp-e2e-receipted-run-001 — minimal receipted spine
# Default kind:  automl     (Stage 1 = ingest existing AutomlPlan corpus)
# Alt kind:      ralph-plan (Stage 1 = ralph run → emit RalphPlan corpus)
#
# Stage 2: dteam doctor verification (--json [--kind=K])
# Stage 3: ggen receipt sign + chain
# Verify : ggen receipt verify + chain-verify
#
# Spine succeeds on any doctor verdict (pass/healthy/soft_fail/fatal).
# Spine fails only if a stage breaks plumbing or the receipt is invalid.

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

# ── Done ──────────────────────────────────────────────────────────────────
echo
log "SPINE COMPLETE"
log "  obligation:       ${OBLIGATION}"
log "  doctor verdict:   ${VERDICT}"
log "  signed receipt:   ${SIGNED_RECEIPT}"
log "  chain file:       ${CHAIN_FILE}"
log "  sign summary:     ${RUN_DIR}/sign-summary.json"
log "  verify summary:   ${RUN_DIR}/verify-summary.json"
log "  chain summary:    ${RUN_DIR}/chain-verify-summary.json"
echo

# Print final receipt-verify JSON to stdout for downstream consumers.
cat "${RUN_DIR}/verify-summary.json"
