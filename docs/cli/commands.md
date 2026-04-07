# CLI Commands

All commands produce ACLI-compliant JSON on stdout. Exit code is `0` on success, non-zero on error.

## `noether version`

```bash
noether version
```

Returns version and build metadata.

## `noether introspect`

```bash
noether introspect
```

Returns the full ACLI manifest: all commands, their arguments, and output schemas.

## `noether stage`

### `stage list`

```bash
noether stage list
```

Lists all active stages in the store.

### `stage get <hash>`

```bash
noether stage get 8f3a1b…
```

Returns the full stage spec for a given `StageId`.

### `stage search <query>`

```bash
noether stage search "parse json and extract field"
noether stage search "http fetch with timeout" --limit 5
```

Semantic search across all active stages. Returns stages ranked by weighted cosine similarity across three indexes (signature 30%, description 50%, examples 20%).

## `noether store`

### `store stats`

```bash
noether store stats
```

Returns stage counts by lifecycle state, total examples, signed/unsigned split.

### `store dedup`

```bash
noether store dedup
```

Detects functionally duplicate stages (same signature, different metadata). Reports candidates; does not auto-merge.

## `noether run`

```bash
noether run graph.json
noether run --dry-run graph.json       # type-check + plan, no execution
noether run --input '{"k":"v"}' graph.json
```

Executes a Lagrange composition graph. `--dry-run` validates types and prints the execution plan without running any stages.

## `noether compose`

```bash
noether compose "problem description"
noether compose --dry-run "problem"          # graph only, no execution
noether compose --model gemini-2.0-flash "problem"
```

LLM-powered composition. Searches the semantic index for candidate stages, builds a graph, type-checks it, and optionally executes it.

Requires `VERTEX_AI_PROJECT`, `VERTEX_AI_LOCATION`, `VERTEX_AI_TOKEN` env vars. Falls back to mock LLM if unset.

## `noether build`

```bash
noether build graph.json --output my-tool
noether build graph.json --output my-tool --target wasm  # planned
```

Compiles a composition graph into a standalone binary. The binary:

- Runs once and prints ACLI JSON when invoked without flags.
- Serves a browser dashboard on `--serve :PORT`.

## `noether trace`

```bash
noether trace <composition_id>
```

Retrieves the full execution trace for a past composition run, including per-stage inputs, outputs, timing, and retry history.

## Output format (ACLI)

All responses follow the Agent-friendly CLI protocol:

```json
{
  "ok": true,
  "command": "stage list",
  "result": { … },
  "meta": { "version": "0.1.0" }
}
```

On error:

```json
{
  "ok": false,
  "command": "run",
  "error": {
    "code": "TYPE_ERROR",
    "message": "stage abc… output Record{url} is not subtype of Record{url,body}"
  },
  "meta": { "version": "0.1.0" }
}
```
