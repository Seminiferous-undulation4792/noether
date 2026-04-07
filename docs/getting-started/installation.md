# Installation

## From crates.io (recommended)

```bash
cargo install noether-cli
```

Requires Rust 1.75+. Install Rust via [rustup.rs](https://rustup.rs).

## Pre-built binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/alpibrusl/noether/releases):

| Platform | Archive |
|---|---|
| Linux x64 | `noether-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` |
| Linux arm64 | `noether-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz` |
| macOS x64 | `noether-vX.Y.Z-x86_64-apple-darwin.tar.gz` |
| macOS arm64 (M-series) | `noether-vX.Y.Z-aarch64-apple-darwin.tar.gz` |
| Windows x64 | `noether-vX.Y.Z-x86_64-pc-windows-msvc.zip` |

```bash
tar xzf noether-*.tar.gz
sudo mv noether /usr/local/bin/
noether version
```

## Build from source

```bash
git clone https://github.com/alpibrusl/noether.git
cd noether
cargo build --release -p noether-cli
./target/release/noether version
```

## Nix (for stage execution)

Noether uses [Nix](https://nixos.org) to run Python, JavaScript, and Bash stages in isolated, reproducible environments. Nix is only required at execution time, not to install the CLI.

```bash
# macOS / Linux
sh <(curl -L https://nixos.org/nix/install) --daemon
```

!!! note "Nix-free mode"
    Without Nix, you can still use `--dry-run` for type-checking and planning, and the `MockExecutor` in tests. Stage execution falls back to a warning if Nix is absent.

## Vertex AI (for LLM features)

`noether compose` and semantic indexing with real embeddings require Vertex AI credentials:

```bash
export VERTEX_AI_PROJECT="your-project-id"
export VERTEX_AI_LOCATION="us-central1"
export VERTEX_AI_TOKEN="$(gcloud auth print-access-token)"
export VERTEX_AI_MODEL="gemini-2.0-flash"
```

Without these, `noether compose` uses a deterministic mock LLM (suitable for testing).

## Verify installation

```bash
noether version
# {"ok":true,"command":"version","result":{"version":"0.1.0","..."},"meta":{"version":"0.1.0"}}

noether stage list | head -20
# Lists the first 20 stdlib stages
```
