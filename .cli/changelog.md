# Noether Changelog

Parameter and command lifecycle per ACLI spec §1.2.

## 0.1.0 (initial release)

### New commands
- `compose` — LLM-powered composition from natural language
- `run` — execute a Lagrange graph directly
- `stage search/add/list/get` — stage store management
- `store stats/retro` — store health and deduplication
- `trace` — retrieve execution traces
- `introspect` / `version` — ACLI standard discovery

### New options (since 0.1.0)
- `compose --allow-capabilities` — capability policy for composed graphs
- `compose --force` — bypass composition cache
- `run --allow-capabilities` — capability policy for direct graph execution

## Planned (next release)

- `build` — compile a composition graph into a standalone binary
- `stage promote` — promote a frequently-used composition to an atomic stage
- `serve` — expose a composition graph as an HTTP endpoint (under consideration)
