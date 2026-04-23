#!/usr/bin/env bash
# Compare two AutoML plan directories. Anti-lie: ONLY compares on-disk JSON,
# cannot be fooled by in-memory state. Reports:
#   - logs where selected signals changed
#   - logs where fusion op changed
#   - logs where plan_vs_gt changed by >1%
#   - logs where accounting_balanced flipped
#
# Usage: automl_plan_diff.sh <old_plan_dir> <new_plan_dir>

set -euo pipefail

if [ $# -ne 2 ]; then
    echo "Usage: $0 <old_plan_dir> <new_plan_dir>" >&2
    exit 1
fi

OLD_DIR="$1"
NEW_DIR="$2"

if [ ! -d "$OLD_DIR" ]; then echo "ERROR: $OLD_DIR not a directory" >&2; exit 2; fi
if [ ! -d "$NEW_DIR" ]; then echo "ERROR: $NEW_DIR not a directory" >&2; exit 2; fi

command -v jq >/dev/null 2>&1 || { echo "ERROR: jq required" >&2; exit 3; }

changed_selected=0
changed_fusion=0
changed_accuracy=0
accounting_flipped=0
total=0

for old_file in "$OLD_DIR"/*.json; do
    [ -e "$old_file" ] || continue
    stem=$(basename "$old_file" .json)
    new_file="$NEW_DIR/$stem.json"
    if [ ! -f "$new_file" ]; then
        echo "MISSING new plan for $stem"
        continue
    fi
    total=$((total + 1))

    old_sel=$(jq -c '.selected' "$old_file")
    new_sel=$(jq -c '.selected' "$new_file")
    old_fus=$(jq -r '.fusion' "$old_file")
    new_fus=$(jq -r '.fusion' "$new_file")
    old_acc=$(jq -r '.plan_accuracy_vs_gt // 0' "$old_file")
    new_acc=$(jq -r '.plan_accuracy_vs_gt // 0' "$new_file")
    old_bal=$(jq -r '.accounting_balanced // false' "$old_file")
    new_bal=$(jq -r '.accounting_balanced // false' "$new_file")

    if [ "$old_sel" != "$new_sel" ]; then
        echo "SELECTED $stem: $old_sel -> $new_sel"
        changed_selected=$((changed_selected + 1))
    fi
    if [ "$old_fus" != "$new_fus" ]; then
        echo "FUSION $stem: $old_fus -> $new_fus"
        changed_fusion=$((changed_fusion + 1))
    fi
    diff=$(awk "BEGIN {print ($new_acc - $old_acc)}")
    abs_diff=$(awk "BEGIN {d = ($new_acc - $old_acc); print (d < 0 ? -d : d)}")
    if awk "BEGIN {exit !($abs_diff > 0.01)}"; then
        echo "ACCURACY $stem: $old_acc -> $new_acc (delta=$diff)"
        changed_accuracy=$((changed_accuracy + 1))
    fi
    if [ "$old_bal" != "$new_bal" ]; then
        echo "ACCOUNTING $stem: $old_bal -> $new_bal  (ANTI-LIE VIOLATION if flipped to false)"
        accounting_flipped=$((accounting_flipped + 1))
    fi
done

echo ""
echo "── Diff Summary ────────────────────────────────────"
echo "  Total compared:            $total"
echo "  Selected signals changed:  $changed_selected"
echo "  Fusion op changed:         $changed_fusion"
echo "  Accuracy changed (>1%):    $changed_accuracy"
echo "  Accounting balanced flipped: $accounting_flipped  (any > 0 is an invariant violation)"

if [ "$accounting_flipped" -gt 0 ]; then
    echo "FAIL: accounting invariant violated in $accounting_flipped plans" >&2
    exit 4
fi
