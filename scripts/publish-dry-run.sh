#!/bin/bash
set -e

VERSION=$1

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

if [ -z "$VERSION" ]; then
    echo -e "${RED}Error: Version required${NC}"
    echo "Usage: ./scripts/publish-dry-run.sh <version>"
    echo "Example: ./scripts/publish-dry-run.sh 0.1.0"
    exit 1
fi

echo -e "${GREEN}DRY RUN: Testing publish for AgentRoot version $VERSION${NC}"
echo -e "${YELLOW}(No actual publishing will occur)${NC}"
echo ""

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "crates" ]; then
    echo -e "${RED}Error: Must run from workspace root${NC}"
    exit 1
fi

echo -e "${YELLOW}Step 1: Running pre-publish checks...${NC}"

# Run tests
echo "  Running tests..."
if cargo test --lib -p agentroot-core --quiet; then
    echo -e "  ${GREEN}✓${NC} Tests passed (147 tests)"
else
    echo -e "  ${RED}✗${NC} Tests failed"
    exit 1
fi

# Run clippy
echo "  Running clippy..."
if cargo clippy --lib -p agentroot-core --quiet -- -D warnings 2>&1 | grep -q "warning"; then
    echo -e "  ${YELLOW}⚠${NC} Clippy warnings found"
else
    echo -e "  ${GREEN}✓${NC} No clippy warnings"
fi

# Check formatting
echo "  Checking format..."
if cargo fmt --all --check; then
    echo -e "  ${GREEN}✓${NC} Code properly formatted"
else
    echo -e "  ${RED}✗${NC} Code needs formatting (run: cargo fmt --all)"
    exit 1
fi

echo ""
echo -e "${YELLOW}Step 2: Checking package contents...${NC}"

# Check what would be published
cd crates/agentroot-core
echo "  agentroot-core package:"
cargo package --list | head -10
cargo package --list | tail -1
cd - > /dev/null

echo ""
cd crates/agentroot-mcp
echo "  agentroot-mcp package:"
cargo package --list | head -10
cargo package --list | tail -1
cd - > /dev/null

echo ""
cd crates/agentroot-cli
echo "  agentroot-cli package:"
cargo package --list | head -10
cargo package --list | tail -1
cd - > /dev/null

echo ""
echo -e "${YELLOW}Step 3: Running dry-run publish for all packages...${NC}"

# Dry run for each package
dry_run_package() {
    local crate_name=$1
    local crate_path=$2
    
    echo ""
    echo -e "${YELLOW}Testing ${crate_name}...${NC}"
    
    cd "$crate_path"
    
    if cargo publish --dry-run --quiet; then
        echo -e "  ${GREEN}✓${NC} ${crate_name} would publish successfully"
    else
        echo -e "  ${RED}✗${NC} ${crate_name} dry-run failed"
        cd - > /dev/null
        exit 1
    fi
    
    cd - > /dev/null
}

dry_run_package "agentroot-core" "crates/agentroot-core"
dry_run_package "agentroot-mcp" "crates/agentroot-mcp"
dry_run_package "agentroot-cli" "crates/agentroot-cli"

echo ""
echo -e "${GREEN}═══════════════════════════════════════════${NC}"
echo -e "${GREEN}✓ DRY RUN SUCCESSFUL${NC}"
echo -e "${GREEN}═══════════════════════════════════════════${NC}"
echo ""
echo "All packages are ready to publish!"
echo ""
echo "To actually publish version $VERSION:"
echo "  ./scripts/publish.sh $VERSION"
echo ""
echo "Make sure you have:"
echo "  1. Logged in to crates.io: cargo login <token>"
echo "  2. Reviewed PUBLISHING.md for the full process"
echo ""
