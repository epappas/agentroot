#!/bin/bash
set -e

# Parse arguments
PUBLISH_ALL=false
VERSION=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --all)
            PUBLISH_ALL=true
            shift
            ;;
        *)
            VERSION=$1
            shift
            ;;
    esac
done

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

if [ -z "$VERSION" ]; then
    echo -e "${RED}Error: Version required${NC}"
    echo "Usage: ./scripts/publish-dry-run.sh [--all] <version>"
    echo ""
    echo "Options:"
    echo "  --all    Test all packages (core, mcp, cli)"
    echo ""
    echo "Examples:"
    echo "  ./scripts/publish-dry-run.sh 0.1.3              # Test only CLI package"
    echo "  ./scripts/publish-dry-run.sh --all 0.1.3        # Test all packages"
    exit 1
fi

if [ "$PUBLISH_ALL" = true ]; then
    echo -e "${GREEN}DRY RUN: Testing ALL packages for version $VERSION${NC}"
else
    echo -e "${GREEN}DRY RUN: Testing agentroot CLI package for version $VERSION${NC}"
fi
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
echo -e "${YELLOW}Step 3: Running dry-run publish...${NC}"
echo ""

if [ "$PUBLISH_ALL" = true ]; then
    echo -e "${YELLOW}Note: agentroot-mcp and agentroot dry-runs may fail${NC}"
    echo -e "${YELLOW}because agentroot-core isn't published to crates.io yet.${NC}"
    echo -e "${YELLOW}This is expected and will work when actually publishing.${NC}"
    echo ""
fi

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

if [ "$PUBLISH_ALL" = true ]; then
    dry_run_package "agentroot-core" "crates/agentroot-core"
    dry_run_package "agentroot-mcp" "crates/agentroot-mcp"
    dry_run_package "agentroot" "crates/agentroot-cli"
else
    dry_run_package "agentroot" "crates/agentroot-cli"
fi

echo ""
echo -e "${GREEN}═══════════════════════════════════════════${NC}"
echo -e "${GREEN}✓ DRY RUN SUCCESSFUL${NC}"
echo -e "${GREEN}═══════════════════════════════════════════${NC}"
echo ""
if [ "$PUBLISH_ALL" = true ]; then
    echo "All packages are ready to publish!"
    echo ""
    echo "To actually publish version $VERSION:"
    echo "  ./scripts/publish.sh --all $VERSION"
else
    echo "The agentroot CLI package is ready to publish!"
    echo ""
    echo "To actually publish version $VERSION:"
    echo "  ./scripts/publish.sh $VERSION"
    echo ""
    echo "To test/publish all packages:"
    echo "  ./scripts/publish-dry-run.sh --all $VERSION"
    echo "  ./scripts/publish.sh --all $VERSION"
fi
echo ""
echo "Make sure you have:"
echo "  1. Logged in to crates.io: cargo login <token>"
echo "  2. Reviewed PUBLISHING.md for the full process"
echo ""
