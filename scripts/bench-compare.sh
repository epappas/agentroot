#!/usr/bin/env bash
# Benchmark comparison script
# Usage: ./scripts/bench-compare.sh [baseline-name]

set -e

BASELINE="${1:-main}"
CURRENT="current"

echo "=== Agentroot Benchmark Comparison ==="
echo "Comparing against baseline: $BASELINE"
echo ""

# Save current baseline if it doesn't exist
if [ ! -d "target/criterion/$BASELINE" ]; then
    echo "Creating baseline '$BASELINE'..."
    cargo bench --bench indexing --bench search -- --save-baseline "$BASELINE"
    echo "Baseline '$BASELINE' created. Run this script again to compare."
    exit 0
fi

# Run benchmarks and compare
echo "Running benchmarks..."
cargo bench --bench indexing --bench search -- --baseline "$BASELINE" > bench-comparison.txt 2>&1

# Display results
cat bench-comparison.txt

# Check for regressions
if grep -q "Performance has regressed" bench-comparison.txt; then
    echo ""
    echo "⚠️  WARNING: Performance regression detected!"
    echo "Review the results above for details."
    exit 1
elif grep -q "Performance has improved" bench-comparison.txt; then
    echo ""
    echo "✅ Performance has improved!"
else
    echo ""
    echo "✅ No significant performance changes."
fi

# Save as new baseline option
echo ""
read -p "Save current results as new baseline? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cargo bench --bench indexing --bench search -- --save-baseline "$BASELINE"
    echo "Baseline '$BASELINE' updated."
fi
