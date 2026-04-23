#!/bin/bash
# Extract and analyze HDC + AutoML accuracy results from pdc2025 output

OUTPUT_FILE="$1"

if [ ! -f "$OUTPUT_FILE" ]; then
    echo "Usage: $0 <output_file>"
    exit 1
fi

echo "=== HDC AND AUTOML ACCURACY ANALYSIS ==="
echo ""

# Extract key accuracy numbers
echo "Strategy Accuracies:"
grep -E "Strategy (F|G|H|HDC|E|AutoML):|classify_exact|fitness_rank|in_language|orthogonal" "$OUTPUT_FILE" | tail -20

echo ""
echo "=== COMPARISON TO 67.78% BASELINE ==="

# Try to get percentages
HDC_ACC=$(grep "Strategy HDC" "$OUTPUT_FILE" | grep -oE "[0-9]+\.[0-9]{2}%" | head -1)
AUTOML_ACC=$(grep "AutoML orthogonal" "$OUTPUT_FILE" | grep -oE "[0-9]+\.[0-9]{2}%" | head -1)

if [ -n "$HDC_ACC" ]; then
    echo "HDC Accuracy: $HDC_ACC"
else
    echo "HDC Accuracy: Not found in output"
fi

if [ -n "$AUTOML_ACC" ]; then
    echo "AutoML Accuracy: $AUTOML_ACC"
else
    echo "AutoML Accuracy: Not found in output"
fi

echo ""
echo "Baseline (F/G/H/Fusions): 67.78%"
