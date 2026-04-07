# Your First Composition

This guide walks through building a real composition graph from scratch —
searching multiple developer platforms and formatting the results as Markdown.

We'll go from zero to a running pipeline in under 5 minutes.

---

## Prerequisites

```bash
# Build the CLI
cargo build --release
export PATH="$PWD/target/release:$PATH"

# Verify
noether version
```

---

## Step 1: Explore available stages

```bash
noether stage list
```

This lists all 75 active stages.  Use semantic search to find what you need:

```bash
noether stage search "search github repositories"
noether stage search "format results as markdown"
```

You'll see output like:

```json
{
  "ok": true,
  "command": "stage search",
  "result": [
    {
      "id": "8dfa010b",
      "description": "Search GitHub repositories, Hacker News stories, and crates.io Rust crates",
      "score": 0.94
    },
    {
      "id": "923a69d9",
      "description": "Format a list of research results into readable Markdown",
      "score": 0.87
    }
  ]
}
```

---

## Step 2: Inspect a stage

```bash
noether stage get 8dfa010b
```

The output shows the stage's full type signature:

```json
{
  "id": "8dfa010b",
  "description": "Search GitHub repositories, Hacker News stories, and crates.io Rust crates",
  "signature": {
    "input": "Record { query: Text }",
    "output": "List<Record { name: Text, url: Text, description: Text, score: Number, source: Text }>",
    "effects": ["Network", "Fallible"]
  }
}
```

The output type of `8dfa010b` is a `List<Record{...}>`.
Check that `923a69d9` accepts that as input:

```bash
noether stage get 923a69d9
# input: "List<Record { name: Text, url: Text, description: Text, ... }>"
```

The types are compatible — we can chain them.

---

## Step 3: Write the composition graph

Create `search.json`:

```json
{
  "description": "Search developer platforms and format results as Markdown",
  "version": "0.1.0",
  "root": {
    "op": "Sequential",
    "stages": [
      {
        "op": "Stage",
        "id": "8dfa010b"
      },
      {
        "op": "Stage",
        "id": "923a69d9"
      }
    ]
  }
}
```

---

## Step 4: Type-check before running

```bash
noether run --dry-run search.json
```

Output:

```json
{
  "ok": true,
  "command": "run dry-run",
  "result": {
    "type_check": "passed",
    "plan": [
      { "step": 1, "stage": "8dfa010b", "op": "Sequential" },
      { "step": 2, "stage": "923a69d9", "op": "Sequential" }
    ],
    "estimated_cost": { "time_ms_p50": null }
  }
}
```

Type errors appear here — before any network call is made.

---

## Step 5: Run it

```bash
noether run search.json --input '{"query": "rust async runtime"}'
```

The pipeline fetches from GitHub, HN, and crates.io in parallel inside the stage,
then passes the merged ranked list to the formatter.

---

## Step 6: Build it into a binary

Once the composition works, compile it into a standalone tool:

```bash
noether build search.json --output search-tool
./search-tool --input '{"query": "distributed systems"}'
```

The binary:

- Runs once and prints ACLI JSON when called with `--input`
- Serves a browser dashboard on `--serve :PORT`
- Contains no external dependencies — the graph is compiled in

---

## What you just did

```
Input: { "query": "rust async runtime" }
       ↓
  8dfa010b  →  Network I/O (GitHub API, HN Algolia, crates.io)
               Type: Record{query:Text} → List<Record{name,url,desc,score,source}>
       ↓
  923a69d9  →  Pure (text formatting)
               Type: List<Record{...}> → Text
       ↓
Output: "## Results\n\n1. **tokio** ..."
```

Two stages.  The type checker guaranteed the connection before execution.
The content-addressed IDs guarantee you always run the same implementation.

---

## Next steps

- [Composition graphs](../guides/composition-graphs.md) — full operator reference (`Parallel`, `Branch`, `Retry`, `Fanout`)
- [Custom stages](../guides/custom-stages.md) — write your own stages
- [Examples](https://github.com/alpibrusl/noether/tree/main/examples) — four ready-to-run graphs including the fleet briefing and travel monitor
