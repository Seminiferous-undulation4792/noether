# Noether CLI Reference

Agent-native verified composition platform. All output is ACLI-compliant JSON.

## Discovery

```bash
noether introspect          # full command tree as JSON
noether --help              # human-readable help
noether <command> --help    # per-command help
```

## Commands

### `noether compose <problem>`

Translate a natural language problem into a typed composition graph and execute it.
The Composition Agent searches the stage store, builds a graph, type-checks it, and runs it.

```bash
noether compose "parse a CSV and extract all email addresses"
noether compose --dry-run "sort a list of numbers"
noether compose --model gemini-2.5-flash "fetch weather for London"
noether compose --input '{"data": "a,b\n1,2"}' "parse this CSV"
noether compose --force "re-do without cache" 
noether compose --allow-capabilities network "fetch the current BTC price"
```

**Flags**

| Flag | Type | Default | Description |
|---|---|---|---|
| `--model` | string | `VERTEX_AI_MODEL` env or `gemini-2.5-flash` | LLM model for composition |
| `--dry-run` | bool | false | Show graph and plan, do not execute |
| `--input` | JSON | null | Input value passed to the composition |
| `--force` | bool | false | Bypass the composition cache |
| `--allow-capabilities` | string | all | Comma-separated capabilities to grant |

---

### `noether run <graph.json>`

Execute a pre-defined Lagrange composition graph directly.

```bash
noether run rail-search.json
noether run rail-search.json --input '{"from":"Madrid","to":"BCN","date":"2026-04-10"}'
noether run rail-search.json --dry-run
noether run rail-search.json --allow-capabilities network,fs-read
```

**Flags**

| Flag | Type | Default | Description |
|---|---|---|---|
| `--dry-run` | bool | false | Type-check and plan only |
| `--input` | JSON | null | Input value |
| `--allow-capabilities` | string | all | Comma-separated capabilities to grant |

---

### `noether stage search <query>`

Search the stage store by semantic similarity.

```bash
noether stage search "convert text to number"
noether stage search "make an HTTP request"
noether stage search "parse structured data"
```

---

### `noether stage add <spec.json>`

Register a new stage from a specification file.
If the stage has no signature it is automatically signed with the local author key
(`~/.noether/author-key.hex`). If it is pre-signed the signature is verified first.

```bash
noether stage add my-stage.json
```

See `schemas/stage-spec.json` for the full specification format.

---

### `noether stage get <hash>`

Retrieve a stage by its content hash (or a unique prefix).

```bash
noether stage get a3f8c1d2
noether stage get a3f8c1d2e4b5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1
```

---

### `noether stage list`

List all active stages with their signatures.

```bash
noether stage list
```

---

### `noether store stats`

Show store statistics including near-duplicate rate.

```bash
noether store stats
```

---

### `noether store retro`

Scan for near-duplicate stages and optionally apply deprecations.

```bash
noether store retro --dry-run           # preview without changes
noether store retro --apply             # apply deprecations
noether store retro --threshold 0.95    # stricter similarity threshold
```

---

### `noether trace <composition_id>`

Retrieve the execution trace for a past composition.

```bash
noether trace a3f8c1d2e4b5f6a7b8c9d0e1f2a3b4c5
```

---

## Environment Variables

| Variable | Description |
|---|---|
| `NOETHER_HOME` | Override default `~/.noether` data directory |
| `VERTEX_AI_PROJECT` | GCP project ID for Vertex AI |
| `VERTEX_AI_LOCATION` | GCP region (e.g. `us-central1`) |
| `VERTEX_AI_TOKEN` | OAuth2 bearer token for Vertex AI |
| `VERTEX_AI_MODEL` | Default LLM model name |

---

## Output format

All output follows the [ACLI spec §2.2](https://alpibrusl.github.io/acli) envelope:

```json
{
  "ok": true,
  "command": "compose",
  "data": { ... },
  "meta": { "duration_ms": 342, "version": "0.1.0" }
}
```

Errors:

```json
{
  "ok": false,
  "command": "run",
  "error": {
    "code": "GENERAL_ERROR",
    "message": "2 capability violation(s)",
    "hints": [
      "stage 'http_get' requires Network; grant it with --allow-capabilities",
      "stage 'write_file' requires FsWrite; grant it with --allow-capabilities"
    ]
  },
  "meta": { "duration_ms": 2, "version": "0.1.0" }
}
```

See `schemas/` for JSON Schema definitions.
