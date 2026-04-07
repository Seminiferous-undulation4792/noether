# LLM-Powered Composition

`noether compose` takes a natural-language problem description and returns a running
composition — no JSON graph authoring required.

---

## How it works

```
"problem description"
        │
   Semantic search  → top-20 candidate stages from the index
        │
   Prompt builder   → system prompt with type system + operators + candidates
        │
   LLM              → Lagrange JSON graph
        │
   Type checker     → validates the graph
        │  retry (up to 3 attempts on type error)
        │
   Executor         → runs the graph
        │
   ACLI output      → { ok, command, data: { output, trace, graph } }
```

The `CompositionAgent` does not hallucinate stage IDs — it only gets to choose from
the top-20 candidates returned by semantic search.  If the right stages aren't in the
index the LLM can say so; it cannot invent stages that don't exist.

---

## Configuration

```bash
# Vertex AI (Google Cloud) — Gemini, Claude, Mistral
export VERTEX_AI_PROJECT=my-project
export VERTEX_AI_LOCATION=us-central1
export VERTEX_AI_TOKEN=$(gcloud auth print-access-token)
export VERTEX_AI_MODEL=gemini-2.0-flash   # default

# Mistral native API (no Google Cloud required)
export MISTRAL_API_KEY=your-key
export VERTEX_AI_MODEL=mistral-large-latest

# Without any env vars → MockLlmProvider (returns deterministic test graph)
```

---

## Usage

```bash
# Compose and execute
noether compose "search GitHub and Hacker News for posts about Rust async and format as Markdown"

# Compose only — print the graph, do not execute
noether compose --dry-run "fetch weather for Berlin and format a one-sentence report"

# Use a specific model
noether compose --model mistral-large-latest "extract all URLs from a text file"

# Use a remote registry's stage index
NOETHER_REGISTRY=https://registry.example.com \
  noether compose "process CSV file and compute column averages"
```

---

## Output

On success:

```json
{
  "ok": true,
  "command": "compose",
  "data": {
    "output": "## GitHub & HN Results\n\n1. **tokio** ...",
    "graph": {
      "description": "search GitHub and HN for Rust async posts",
      "root": { "op": "Sequential", "stages": [...] }
    },
    "trace": {
      "composition_id": "a3f9...",
      "duration_ms": 1840,
      "stages": [...]
    },
    "attempts": 1
  }
}
```

On type error after retries:

```json
{
  "ok": false,
  "command": "compose",
  "error": {
    "code": "TYPE_ERROR",
    "message": "after 3 attempts: stage abc… output Text is not subtype of Record{url}"
  }
}
```

---

## Retry behaviour

If the LLM produces a graph that fails type-checking, the agent:

1. Adds the type error to the next prompt as a correction hint
2. Asks the LLM to regenerate (up to 3 attempts)
3. Returns `TYPE_ERROR` if all attempts fail

This loop catches the most common LLM mistake: chaining stages whose types don't match.

---

## Saving a composed graph

Inspect the `graph` field in the output to save it for `noether run`:

```bash
noether compose --dry-run "problem" | python3 -c "
import json, sys
d = json.load(sys.stdin)
print(json.dumps(d['data']['graph'], indent=2))
" > my-graph.json

noether run my-graph.json --input '{"query": "test"}'
```

---

## Writing effective prompts

| What you want | How to phrase it |
|---|---|
| Multi-source fetch | "fetch X and Y in parallel, then …" |
| Conditional logic | "if the response is empty, …, otherwise …" |
| Retry on error | "retry up to 3 times if the HTTP call fails" |
| Specific types | "the input is a CSV file path, output is a JSON array" |
| Pure computation | "no network access, compute entirely from the input" |

The agent's system prompt includes the full type system and all 7 operators —
the LLM understands `Parallel`, `Branch`, `Retry`, and `Fanout` natively.
