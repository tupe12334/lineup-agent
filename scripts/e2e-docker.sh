#!/usr/bin/env bash
set -euo pipefail

# E2E Docker Test Runner for lineup-agent
# Usage: ./scripts/e2e-docker.sh [--build] [--verbose] [--no-bail]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Default options
BUILD=false
BAIL=true

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --build|-b)
            BUILD=true
            shift
            ;;
        --no-bail)
            BAIL=false
            shift
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

cd "$PROJECT_ROOT"

echo -e "${YELLOW}========================================${NC}"
echo -e "${YELLOW}  lineup-agent E2E Tests (Docker/Linux)${NC}"
echo -e "${YELLOW}========================================${NC}"

# Build if requested or if image doesn't exist
if [ "$BUILD" = true ] || ! docker images | grep -q "lineup-agent"; then
    echo -e "${YELLOW}Building E2E test container...${NC}"
    docker compose -f docker/docker-compose.e2e.yml build e2e-test
fi

# Create test results directory
mkdir -p test-results

# Run tests
echo -e "${YELLOW}Running E2E tests in Docker container...${NC}"

COMPOSE_CMD="pnpm test:e2e"
if [ "$BAIL" = true ]; then
    COMPOSE_CMD="$COMPOSE_CMD --bail=1"
fi

if docker compose -f docker/docker-compose.e2e.yml run --rm e2e-test $COMPOSE_CMD; then
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  E2E Tests PASSED${NC}"
    echo -e "${GREEN}========================================${NC}"
    exit 0
else
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}  E2E Tests FAILED${NC}"
    echo -e "${RED}========================================${NC}"
    exit 1
fi
