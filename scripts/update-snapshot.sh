#!/usr/bin/env bash
set -euo pipefail

# Update E2E lint --fix output snapshot
# Run this after adding or modifying rules

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "🔄 Updating E2E snapshot..."

# Run the snapshot test with -u flag to update snapshots
pnpm test:e2e -- snapshots/snapshot.test.ts -u

SNAPSHOT_FILE="e2e/snapshots/__snapshots__/snapshot.test.ts.snap"

if [ -f "$SNAPSHOT_FILE" ]; then
    echo ""
    echo "✅ Snapshot updated successfully!"
    echo ""
    echo "Review the changes and commit when ready:"
    echo "  git diff $SNAPSHOT_FILE"
    echo "  git add $SNAPSHOT_FILE"
else
    echo "❌ Failed to generate snapshot"
    exit 1
fi
