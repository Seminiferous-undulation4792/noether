# Composition Graphs (Lagrange JSON)

A composition graph is a JSON document that describes how stages connect. Noether calls this format **Lagrange**.

## Operators

There are 7 operators:

| `op` | Description |
|---|---|
| `Stage` | Run a single stage |
| `Sequential` | Run steps in order, each feeds the next |
| `Parallel` | Run branches concurrently, merge outputs |
| `Branch` | Conditional — choose one path at runtime |
| `Fanout` | Broadcast one input to multiple stages |
| `Merge` | Combine multiple inputs into one record |
| `Retry` | Wrap any node with retry-on-failure |
| `Const` | Inject a literal constant value |

## Stage

```json
{
  "op": "Stage",
  "stage_id": "8f3a1b…"
}
```

Input is the output of the previous step (or the graph's top-level input if first).

## Sequential

```json
{
  "op": "Sequential",
  "steps": [
    { "op": "Stage", "stage_id": "abc…" },
    { "op": "Stage", "stage_id": "def…" }
  ]
}
```

`steps[0]` output → `steps[1]` input → … → last step output is the Sequential's output.

## Const

Inject a literal value anywhere in the graph:

```json
{
  "op": "Sequential",
  "steps": [
    { "op": "Const", "value": { "url": "https://api.example.com/data" } },
    { "op": "Stage", "stage_id": "<http_get-hash>" }
  ]
}
```

## Parallel

Run two or more branches concurrently. Each branch receives the same input. Outputs are merged into a single record.

```json
{
  "op": "Parallel",
  "branches": [
    { "op": "Stage", "stage_id": "branch-a-hash" },
    { "op": "Stage", "stage_id": "branch-b-hash" }
  ]
}
```

## Branch

```json
{
  "op": "Branch",
  "condition": { "op": "Stage", "stage_id": "<bool-producing-stage>" },
  "then": { "op": "Stage", "stage_id": "<then-stage>" },
  "else": { "op": "Stage", "stage_id": "<else-stage>" }
}
```

## Retry

```json
{
  "op": "Retry",
  "node": { "op": "Stage", "stage_id": "<flaky-stage>" },
  "max_attempts": 3,
  "backoff_ms": 1000
}
```

## Composition ID

The `CompositionId` is the SHA-256 of the canonical JSON of the root node. The same graph always gets the same ID on any machine — traces are keyed by this ID.

## Type checking

`noether run --dry-run graph.json` type-checks every edge before execution. A type error looks like:

```json
{
  "ok": false,
  "error": {
    "code": "TYPE_ERROR",
    "message": "edge from Stage(abc) to Stage(def): output Record{url,status} is not subtype of input Record{url,body}"
  }
}
```

The type checker uses [structural subtyping](../architecture/type-system.md) — width subtyping means extra fields are always fine.
