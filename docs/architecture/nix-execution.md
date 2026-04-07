# Nix Execution Layer

Nix provides hermetic, reproducible sandboxed execution for Python, JavaScript,
and bash stages.  It is Noether's L1 — the layer below the stage store.

---

## Why Nix

A stage is identified by its content hash.  For that guarantee to mean anything,
the execution environment must also be reproducible.  The same Python stage on two
machines must produce the same output from the same input.

Nix achieves this through:

- **Content-addressed derivations** — every package is identified by the hash of its
  build recipe.  Two Nix derivations with the same hash produce bit-for-bit identical
  outputs.
- **No shared mutable state** — packages live in `/nix/store/<hash>-<name>`, isolated
  from system libraries.
- **Hermetic builds** — network access is blocked during evaluation; all inputs are
  declared explicitly.

---

## How `NixExecutor` works

```rust
// crates/noether-engine/src/executor/nix.rs
pub struct NixExecutor {
    nix_bin: PathBuf,
}

impl StageExecutor for NixExecutor {
    fn execute(&self, stage_id: &StageId, input: &Value) -> Result<Value, ExecutionError> {
        // 1. Look up stage implementation (Python code string)
        // 2. Write to a temp file
        // 3. Build a minimal Nix shell with the required packages
        // 4. Spawn: nix-shell --pure --run "python3 stage.py" < input.json
        // 5. Parse stdout as JSON
    }
}
```

The Python wrapper receives the stage input as JSON on stdin and must write its
output as JSON to stdout.  Stderr is captured for error reporting.

---

## Stage implementation languages

| Language | Executor | Isolation | Startup overhead |
|---|---|---|---|
| Rust (inline) | `InlineExecutor` | In-process | ~0 ms |
| Python | `NixExecutor` | Nix sandbox | ~200 ms (warm) |
| JavaScript | `NixExecutor` | Nix sandbox | ~150 ms (warm) |
| Bash | `NixExecutor` | Nix sandbox | ~50 ms |

The stdlib uses `InlineExecutor` for all Pure stages (zero overhead).
`NixExecutor` is used for stages that need Python libraries (numpy, pandas, etc.).

---

## Binary cache

Nix packages are fetched from `cache.nixos.org` on first use and cached in
`/nix/store`.  Subsequent runs of the same stage use the cache — startup overhead
drops from ~2 s (cold fetch) to ~200 ms (warm).

In CI and production, a team can run a private Nix binary cache to share built
derivations across machines.

---

## Current status

`NixExecutor` is implemented and used in the end-to-end nix tests:

```bash
cargo test -p noether-engine nix_e2e   # requires nix in PATH
```

The test suite passes with `nix` installed; it is skipped gracefully when Nix is not
available, so the rest of the test suite always passes.

The stdlib stages currently use inline Rust implementations.  Python stages
(for data science / ML use cases) are planned for Phase 5.

---

## Phase roadmap

| Phase | Nix feature |
|---|---|
| ✅ Phase 2 | `NixExecutor` — spawn subprocess, pass JSON over stdio |
| ✅ Phase 3 | `InlineExecutor` for Pure Rust stages (zero overhead) |
| 🔜 Phase 5 | Python stages via Nix (numpy, pandas, scikit-learn) |
| 🔜 Phase 6 | WASM target — compile Nix derivation to WASM for browser execution |
