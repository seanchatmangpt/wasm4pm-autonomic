#!/usr/bin/env bash
set -e

echo "=========================================================="
echo "Starting DDS Ralph Overnight Orchestration Loop"
echo "Target Models: gemini-3.1-flash-lite-preview"
echo "Concurrency: 5"
echo "=========================================================="

echo "Running pre-flight structural checks..."
cargo check
cargo test --lib

echo "Verifying T1 admissibility across all substrate patterns..."
cargo run --bin bench_scanner

echo "Pre-flight checks passed. Unleashing Ralph on the backlog..."

# Execute Ralph with the fallback model for all ideas
RUST_LOG=info cargo run --release --bin ralph -- \
    --model "gemini-3.1-flash-lite-preview" \
    --concurrency 5 \
    --offset 0

echo "=========================================================="
echo "Ralph execution complete. Please check the dev branch for merged artifacts."
echo "=========================================================="