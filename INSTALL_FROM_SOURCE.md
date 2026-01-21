# Installing AgentRoot from Source

## Quick Install

From the workspace root:

```bash
cargo install --path crates/agentroot-cli
```

This installs the `agentroot` binary to `~/.cargo/bin/`

## Detailed Steps

### 1. Clone Repository

```bash
git clone https://github.com/epappas/agentroot
cd agentroot
```

### 2. Build (Optional)

Build without installing:

```bash
cargo build --release
# Binary will be at: target/release/agentroot
```

### 3. Install

Install to `~/.cargo/bin/` (automatically in PATH):

```bash
cargo install --path crates/agentroot-cli
```

Or install from a specific crate directory:

```bash
cd crates/agentroot-cli
cargo install --path .
```

### 4. Verify Installation

```bash
which agentroot
# Output: /root/.cargo/bin/agentroot

agentroot --version
# Output: agentroot 0.1.0
```

## Important Notes

❌ **Don't run from workspace root:**
```bash
cargo install --path .
# Error: found a virtual manifest
```

✅ **Do specify the CLI crate path:**
```bash
cargo install --path crates/agentroot-cli
```

## Troubleshooting

### Error: "found a virtual manifest"

**Cause**: Trying to install from workspace root without specifying the crate

**Solution**: 
```bash
cargo install --path crates/agentroot-cli
```

### Installation Hangs

**Cause**: Building all dependencies (first time)

**Solution**: Wait 1-2 minutes for compilation to complete

### Permission Denied

**Cause**: No write access to `~/.cargo/bin/`

**Solution**:
```bash
# Create cargo bin directory
mkdir -p ~/.cargo/bin

# Ensure it's in PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

## Updating

To update an existing installation:

```bash
cd agentroot
git pull
cargo install --path crates/agentroot-cli --force
```

The `--force` flag replaces the existing installation.

## Uninstalling

```bash
cargo uninstall agentroot
```

## Development Installation

For development with hot reload:

```bash
# Install cargo-watch
cargo install cargo-watch

# Run with auto-rebuild
cargo watch -x 'run --bin agentroot'
```

## See Also

- [PUBLISHING.md](PUBLISHING.md) - Publishing to crates.io
- [README.md](README.md) - Project overview
- [QUICKSTART.md](QUICKSTART.md) - Quick reference
