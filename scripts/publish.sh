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
    echo "Usage: ./scripts/publish.sh <version>"
    echo "Example: ./scripts/publish.sh 0.1.0"
    exit 1
fi

echo -e "${GREEN}Publishing AgentRoot version $VERSION${NC}"
echo ""

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "crates" ]; then
    echo -e "${RED}Error: Must run from workspace root${NC}"
    exit 1
fi

# Check if logged in to crates.io
if ! cargo login --help &> /dev/null; then
    echo -e "${RED}Error: cargo not found${NC}"
    exit 1
fi

echo -e "${YELLOW}Step 1: Checking if already logged in to crates.io...${NC}"
# Test if we can query crates.io (indirect way to check login)
if ! cargo search agentroot --limit 1 &> /dev/null; then
    echo -e "${RED}Warning: May not be logged in to crates.io${NC}"
    echo "Run: cargo login <your-token>"
    echo "Continue anyway? (y/N)"
    read -r response
    if [ "$response" != "y" ]; then
        exit 1
    fi
fi

echo -e "${YELLOW}Step 2: Running pre-publish checks...${NC}"

# Run tests
echo "  Running tests..."
if ! cargo test --lib -p agentroot-core --quiet; then
    echo -e "${RED}Tests failed!${NC}"
    exit 1
fi
echo -e "  ${GREEN}✓${NC} Tests passed"

# Run clippy
echo "  Running clippy..."
if ! cargo clippy --lib -p agentroot-core -- -D warnings 2>&1 | grep -q "Finished"; then
    echo -e "${YELLOW}Warning: Clippy warnings found (continuing anyway)${NC}"
fi
echo -e "  ${GREEN}✓${NC} Clippy check done"

# Check formatting
echo "  Checking format..."
if ! cargo fmt --all --check; then
    echo -e "${RED}Code not formatted! Run: cargo fmt --all${NC}"
    exit 1
fi
echo -e "  ${GREEN}✓${NC} Code formatted"

echo ""
echo -e "${YELLOW}Step 3: Updating version to $VERSION...${NC}"

# Update version in workspace Cargo.toml
if [[ "$OSTYPE" == "darwin"* ]]; then
    sed -i '' "s/^version = .*/version = \"$VERSION\"/" Cargo.toml
else
    sed -i "s/^version = .*/version = \"$VERSION\"/" Cargo.toml
fi

echo -e "  ${GREEN}✓${NC} Version updated in Cargo.toml"

# Update Cargo.lock
cargo update --workspace --quiet
echo -e "  ${GREEN}✓${NC} Cargo.lock updated"

echo ""
echo -e "${YELLOW}Step 4: Committing version bump...${NC}"

git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to $VERSION" --quiet || true
echo -e "  ${GREEN}✓${NC} Version committed"

echo ""
echo -e "${YELLOW}Step 5: Publishing packages to crates.io...${NC}"
echo ""

# Function to publish a package
publish_package() {
    local crate_name=$1
    local crate_path=$2
    
    echo -e "${YELLOW}Publishing ${crate_name}...${NC}"
    
    cd "$crate_path"
    
    # Dry run first
    echo "  Running dry-run..."
    if ! cargo publish --dry-run --quiet; then
        echo -e "${RED}Dry run failed for ${crate_name}!${NC}"
        cd - > /dev/null
        exit 1
    fi
    echo -e "  ${GREEN}✓${NC} Dry run passed"
    
    # Actual publish
    echo "  Publishing to crates.io..."
    if ! cargo publish; then
        echo -e "${RED}Publishing failed for ${crate_name}!${NC}"
        cd - > /dev/null
        exit 1
    fi
    echo -e "  ${GREEN}✓${NC} Published ${crate_name}"
    
    cd - > /dev/null
}

# Publish in order: core → mcp → cli
publish_package "agentroot-core" "crates/agentroot-core"
echo ""
echo -e "${YELLOW}Waiting 120 seconds for crates.io to index agentroot-core...${NC}"
sleep 120

publish_package "agentroot-mcp" "crates/agentroot-mcp"
echo ""
echo -e "${YELLOW}Waiting 120 seconds for crates.io to index agentroot-mcp...${NC}"
sleep 120

publish_package "agentroot-cli" "crates/agentroot-cli"

echo ""
echo -e "${GREEN}✓ Successfully published all packages!${NC}"
echo ""

echo -e "${YELLOW}Step 6: Creating git tag...${NC}"
git tag "v$VERSION"
echo -e "  ${GREEN}✓${NC} Created tag v$VERSION"

echo ""
echo -e "${GREEN}═══════════════════════════════════════════${NC}"
echo -e "${GREEN}✓ Published AgentRoot version $VERSION${NC}"
echo -e "${GREEN}═══════════════════════════════════════════${NC}"
echo ""
echo "Next steps:"
echo "  1. Push to GitHub:"
echo "     git push github master --tags"
echo ""
echo "  2. Verify installation:"
echo "     cargo install agentroot --force"
echo "     agentroot --version"
echo ""
echo "  3. Check on crates.io (may take a few minutes):"
echo "     https://crates.io/crates/agentroot"
echo ""
