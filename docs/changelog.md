# Changelog

All notable changes to Noether are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Noether uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- `noether store dedup` — detect functionally duplicate stages
- Branch protection and CI workflows on `main`
- MkDocs documentation site

---

## [0.1.0] — 2026-04-07

### Added
- **Phase 0** — Type system (`NType`), structural subtyping, stage schema, Ed25519 signing, SHA-256 content addressing
- **Phase 1** — `StageStore` trait + `MemoryStore`, 50 stdlib stages, lifecycle validation
- **Phase 2** — Lagrange composition graph, type checker, `ExecutionPlan`, `run_composition`, traces
- **Phase 3** — Composition Agent, semantic three-index search, `VertexAiLlmProvider`, `noether compose`
- **Phase 4** — `noether build` with `--serve :PORT` browser dashboard, `--dry-run`, store dedup
- ACLI-compliant CLI with structured JSON output for all commands
- `noether-research/` design documents: NoetherReact, WASM target, Cloud Registry
