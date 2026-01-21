# Installation Instructions for Users

Once published to crates.io, users can install AgentRoot in multiple ways:

## Option 1: cargo install (Easiest)

```bash
cargo install agentroot
```

This installs the latest stable release from crates.io.

## Option 2: From Source (Development)

```bash
git clone https://github.com/epappas/agentroot
cd agentroot
cargo install --path crates/agentroot-cli
```

## Option 3: Pre-built Binary (Future)

Download from releases:
```bash
# Linux
curl -L https://github.com/epappas/agentroot/releases/latest/download/agentroot-linux-x86_64.tar.gz | tar xz
sudo mv agentroot /usr/local/bin/

# macOS
curl -L https://github.com/epappas/agentroot/releases/latest/download/agentroot-macos-x86_64.tar.gz | tar xz
sudo mv agentroot /usr/local/bin/
```

## Verification

```bash
agentroot --version
```

## First Run

AgentRoot downloads the embedding model on first use:

```bash
# Test installation
agentroot status

# On first run:
# 📥 Downloading nomic-embed-text-v1.5.Q4_K_M.gguf (100.3 MB)
# ⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿ 100%
# ✅ Model loaded
```

Model is cached at: `~/.local/share/agentroot/models/`

## Quick Start

```bash
# Add your code
agentroot collection add ~/my-project --name myapp

# Index and search
agentroot update && agentroot embed
agentroot query "what you're looking for"
```

See [QUICKSTART.md](QUICKSTART.md) for more examples.

## Optional: Basilica Integration

For AI-powered features with GPU acceleration:

```bash
# Get endpoints at https://basilica.ai
export AGENTROOT_LLM_URL="https://your-id.deployments.basilica.ai"
export AGENTROOT_EMBEDDING_URL="https://your-id.deployments.basilica.ai"
export AGENTROOT_EMBEDDING_MODEL="intfloat/e5-mistral-7b-instruct"
export AGENTROOT_EMBEDDING_DIMS="4096"

# Use AI features
agentroot smart "natural language query"
agentroot metadata refresh myapp
```

See [VLLM_SETUP.md](VLLM_SETUP.md) for complete Basilica setup.

## Updating

```bash
cargo install agentroot --force
```

Or check for updates:

```bash
cargo install-update agentroot  # requires cargo-update
```

## System Requirements

- **OS**: Linux, macOS, or Windows
- **Rust**: 1.70+ (for building from source)
- **Memory**: 4GB minimum, 8GB recommended
- **Disk**: 200MB for binary + models

## Getting Help

- **Documentation**: [README.md](README.md)
- **Workflows**: [WORKFLOW.md](WORKFLOW.md)
- **Quick Reference**: [QUICKSTART.md](QUICKSTART.md)
- **Issues**: https://github.com/epappas/agentroot/issues
