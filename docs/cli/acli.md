# ACLI — Agent-friendly CLI Protocol

ACLI is the structured output protocol that makes `noether` consumable by AI agents
without output parsing.  Every command writes a single JSON object to stdout.
The exit code is `0` on success, `1` on error.

---

## Response envelope

### Success

```json
{
  "ok": true,
  "command": "<subcommand name>",
  "data": { ... },
  "meta": {
    "version": "0.1.0"
  }
}
```

### Failure

```json
{
  "ok": false,
  "command": "<subcommand name>",
  "error": {
    "code": "TYPE_ERROR",
    "message": "stage abc… output Record{url} is not subtype of Record{url,body}"
  },
  "meta": {
    "version": "0.1.0"
  }
}
```

The `ok` field is always present and always a boolean — agents can branch on it
without inspecting `error.code`.

---

## Error codes

| Code | Meaning |
|---|---|
| `TYPE_ERROR` | Composition graph failed type-check |
| `STAGE_NOT_FOUND` | A stage ID referenced in the graph does not exist in the store |
| `EXECUTION_ERROR` | A stage returned a runtime error |
| `PARSE_ERROR` | Input JSON is malformed |
| `VALIDATION_FAILED` | Stage submission failed validation (content hash, signature, etc.) |
| `STORE_ERROR` | Underlying store operation failed |
| `NOT_FOUND` | Requested resource (stage, trace) does not exist |
| `INVALID_PARAM` | A query parameter has an invalid value |
| `MISSING_PARAM` | A required parameter is absent |

---

## `noether introspect`

Returns the full ACLI manifest: all commands, their arguments, and output schemas.
This is the **first call** an agent should make to discover capabilities:

```bash
noether introspect
```

```json
{
  "ok": true,
  "command": "introspect",
  "data": {
    "commands": [
      {
        "name": "stage list",
        "description": "List all active stages in the store",
        "args": [],
        "output_schema": { "stages": "array", "count": "number" }
      },
      {
        "name": "stage search",
        "description": "Semantic search across all active stages",
        "args": [
          { "name": "query", "type": "string", "required": true },
          { "name": "--limit", "type": "number", "default": 20 }
        ],
        "output_schema": { "results": "array" }
      }
    ]
  }
}
```

---

## Agent usage patterns

### Pattern 1: Capability discovery + composition

```python
import subprocess, json

def noether(cmd):
    r = subprocess.run(["noether"] + cmd.split(), capture_output=True)
    return json.loads(r.stdout)

# Discover what's available
manifest = noether("introspect")

# Find relevant stages
results = noether('stage search "parse CSV and compute statistics"')
for r in results["data"]["results"]:
    print(r["id"], r["description"], r["score"])

# Run a composition
output = noether('run --input {"query":"rust"} search.json')
if output["ok"]:
    process(output["data"])
else:
    handle_error(output["error"]["code"], output["error"]["message"])
```

### Pattern 2: LLM-powered solve

```python
result = noether('compose "download weather forecast for Berlin and format as a briefing"')
if result["ok"]:
    print(result["data"]["output"])   # the composed + executed result
else:
    # retry with more context
    result = noether('compose --dry-run "..."')
    print(result["data"]["graph"])    # inspect the planned graph
```

### Pattern 3: Pipeline verification before submission

```python
plan = noether('run --dry-run my-graph.json')
assert plan["ok"], f"Type error: {plan['error']['message']}"
# Only run if type-safe
output = noether('run my-graph.json')
```

---

## ACLI compliance guarantees

1. **Stdout is always valid JSON** — stderr is used for progress/debug messages only
2. **`ok` is always present** — agents never need to check exit codes
3. **`command` echoes the subcommand** — useful when batching multiple calls
4. **`meta.version` is semver** — agents can gate on API version
5. **Error codes are stable** — agents can write `if code == "TYPE_ERROR"` without parsing the message

---

## Built binary API

A binary built with `noether build` inherits the ACLI protocol:

```bash
# Single-shot: runs and prints ACLI JSON
./my-tool --input '{"query": "async rust"}'

# Server mode: wraps the composition as an HTTP API with browser dashboard
./my-tool --serve :8080
```

The `--serve` mode exposes:

- `POST /run` with `Content-Type: application/json` body → ACLI JSON response
- `GET /` → browser dashboard (HTML, JS, CSS embedded in binary)
- `GET /health` → `{"status": "ok", "version": "..."}`
