# Publishing AgentRoot to crates.io

This guide explains how to publish AgentRoot packages to crates.io so users can install with `cargo install agentroot`.

## Prerequisites

1. **crates.io Account**: Create account at [crates.io](https://crates.io)
2. **API Token**: Get from [crates.io/settings/tokens](https://crates.io/me/settings)
3. **Ownership**: You must be the package owner (or have permissions)

## Setup

### 1. Login to crates.io

```bash
cargo login <your-api-token>
```

This saves your token to `~/.cargo/credentials.toml`

### 2. Verify Package Metadata

All required fields are already configured in `Cargo.toml`:

```toml
[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
repository = "https://github.com/epappas/agentroot"
homepage = "https://github.com/epappas/agentroot"
documentation = "https://github.com/epappas/agentroot#readme"
readme = "README.md"
keywords = ["search", "semantic", "embeddings", "code-search", "knowledge-base"]
categories = ["command-line-utilities", "development-tools", "text-processing"]
authors = ["Evangelos Pappas <epappas@evalonlabs.com>"]
```

## Publishing Order

⚠️ **Important**: Publish in dependency order (core → mcp → cli)

### Step 1: Publish Core Library

```bash
cd crates/agentroot-core

# Dry run (test without publishing)
cargo publish --dry-run

# Check package contents
cargo package --list

# Publish
cargo publish
```

**Wait 1-2 minutes** for crates.io to index the package before proceeding.

### Step 2: Publish MCP Server

```bash
cd ../agentroot-mcp

# Dry run
cargo publish --dry-run

# Publish
cargo publish
```

**Wait 1-2 minutes** for indexing.

### Step 3: Publish CLI

```bash
cd ../agentroot-cli

# Dry run
cargo publish --dry-run

# Publish (this enables 'cargo install agentroot')
cargo publish
```

## Verification

After publishing, verify installation works:

```bash
# Install from crates.io
cargo install agentroot

# Verify
agentroot --version

# Test
agentroot status
```

## Updating Version

To release a new version:

### 1. Update Version Number

Edit `Cargo.toml` workspace version:

```toml
[workspace.package]
version = "0.1.1"  # Increment version
```

### 2. Update CHANGELOG.md

Document what changed:

```markdown
## [0.1.1] - 2026-01-21

### Added
- Feature X
- Feature Y

### Fixed
- Bug Z
```

### 3. Commit Version Bump

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to 0.1.1"
git tag v0.1.1
git push github master --tags
```

### 4. Republish (same order as before)

```bash
cd crates/agentroot-core && cargo publish
sleep 120  # Wait for indexing
cd ../agentroot-mcp && cargo publish
sleep 120
cd ../agentroot-cli && cargo publish
```

## Troubleshooting

### Error: "crate not found"

**Cause**: Dependency not yet indexed on crates.io

**Solution**: Wait 2-3 minutes after publishing dependencies before publishing dependents

### Error: "version already exists"

**Cause**: Cannot overwrite published versions

**Solution**: Increment version number in Cargo.toml

### Error: "failed to verify"

**Cause**: Missing files or broken dependencies

**Solution**: 
```bash
cargo publish --dry-run --verbose
# Check what files are included
cargo package --list
```

### Error: "authentication required"

**Cause**: Not logged in to crates.io

**Solution**:
```bash
cargo login <your-api-token>
```

## Pre-publish Checklist

Before publishing, ensure:

- ✅ All tests pass: `cargo test --all`
- ✅ No clippy warnings: `cargo clippy --all-targets --all-features`
- ✅ Code formatted: `cargo fmt --all`
- ✅ Documentation builds: `cargo doc --no-deps`
- ✅ Examples compile: `cargo build --examples`
- ✅ README is up to date
- ✅ CHANGELOG is updated
- ✅ Version number incremented
- ✅ Git committed and tagged
- ✅ CI passing (if applicable)

## Automation Script

Create `scripts/publish.sh`:

```bash
#!/bin/bash
set -e

VERSION=$1

if [ -z "$VERSION" ]; then
    echo "Usage: ./scripts/publish.sh <version>"
    exit 1
fi

echo "Publishing version $VERSION..."

# Update version
sed -i "s/^version = .*/version = \"$VERSION\"/" Cargo.toml

# Run tests
cargo test --all
cargo clippy --all-targets --all-features
cargo fmt --all --check

# Commit version bump
git add Cargo.toml
git commit -m "chore: bump version to $VERSION"
git tag "v$VERSION"

# Publish packages
echo "Publishing agentroot-core..."
cd crates/agentroot-core
cargo publish
sleep 120

echo "Publishing agentroot-mcp..."
cd ../agentroot-mcp
cargo publish
sleep 120

echo "Publishing agentroot-cli..."
cd ../agentroot-cli
cargo publish

echo "✅ Published version $VERSION"
echo "Don't forget to: git push github master --tags"
```

Usage:
```bash
chmod +x scripts/publish.sh
./scripts/publish.sh 0.1.1
```

## Package Visibility

After publishing:

- **crates.io**: https://crates.io/crates/agentroot
- **docs.rs**: https://docs.rs/agentroot (auto-generated)
- **lib.rs**: https://lib.rs/crates/agentroot (auto-listed)

## Yanking Versions

If you need to remove a version (but not delete):

```bash
cargo yank --vers 0.1.0
```

This prevents new installs but keeps existing ones working.

To un-yank:
```bash
cargo yank --vers 0.1.0 --undo
```

## Best Practices

1. **Semantic Versioning**: Follow [semver](https://semver.org)
   - `0.1.0` → `0.1.1` (patch: bug fixes)
   - `0.1.0` → `0.2.0` (minor: new features)
   - `0.1.0` → `1.0.0` (major: breaking changes)

2. **Test Before Publishing**: Always run `cargo publish --dry-run`

3. **Wait Between Publishes**: Give crates.io 2-3 minutes to index

4. **Tag Git Releases**: Use `git tag v0.1.0` for each release

5. **Update Changelog**: Document all changes

6. **Monitor Downloads**: Check [crates.io/crates/agentroot/stats](https://crates.io/crates/agentroot)

## Documentation

After publishing, docs.rs automatically builds documentation:
- https://docs.rs/agentroot
- https://docs.rs/agentroot-core

To preview docs locally:
```bash
cargo doc --open --no-deps
```

## License

Ensure `LICENSE` file is present in the repository root. AgentRoot uses MIT license.

## Support

- **crates.io Issues**: https://github.com/rust-lang/crates.io/issues
- **Cargo Book**: https://doc.rust-lang.org/cargo/reference/publishing.html
- **AgentRoot Issues**: https://github.com/epappas/agentroot/issues
