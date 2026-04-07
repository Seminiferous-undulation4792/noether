# Composition Engine

The composition engine sits between the stage store (L2) and the agent interface (L4).
It takes a Lagrange graph, verifies it is type-safe, plans execution order, runs it,
and produces a structured trace.

## Why "Lagrange"?

The project is named after **Emmy Noether**, whose theorem connects symmetries in a
physical system to conservation laws.  In Noether's theorem, the symmetry is expressed
in the **Lagrangian** — a function named after **Joseph-Louis Lagrange**.

The composition graph format inherits that name: just as the Lagrangian is the object
you write down to describe a system and Noether's theorem guarantees its conservation
laws, the Lagrange graph is what you write down to describe a computation and Noether's
type system guarantees its correctness.

In practice: **a Lagrange graph is a JSON DAG of stages connected by typed edges.**

---

## Pipeline

```
CompositionGraph (JSON)
        │
  ┌─────▼──────┐
  │ Type checker│  check_graph() — recursive subtype check on every edge
  └─────┬──────┘
        │ ✓ type-safe
  ┌─────▼──────┐
  │   Planner  │  plan_graph() — flatten AST to linear ExecutionPlan
  └─────┬──────┘
        │
  ┌─────▼──────┐
  │  Executor  │  run_composition() — execute plan, collect trace
  └─────┬──────┘
        │
  CompositionResult { output: Value, trace: CompositionTrace }
```

---

## The Lagrange graph format

Compositions are expressed as a JSON AST with seven operators:

### `Stage` — leaf node

```json
{ "op": "Stage", "id": "39731ebb" }
```

References a stage by its content-addressed ID.  The engine resolves the ID to a
`StageSignature` for type-checking and to an implementation for execution.

### `Sequential` — pipeline

```json
{
  "op": "Sequential",
  "stages": [
    { "op": "Stage", "id": "b4dfc6b0" },
    { "op": "Stage", "id": "39731ebb" },
    { "op": "Stage", "id": "62bdb044" }
  ]
}
```

Output of each stage feeds the input of the next.
Type check: `output[n]` must be subtype of `input[n+1]`.

### `Parallel` — named fan-out

```json
{
  "op": "Parallel",
  "branches": {
    "weather":  { "op": "Stage", "id": "..." },
    "stations": { "op": "Stage", "id": "..." }
  }
}
```

Each branch receives `input[branch_name]` if the input is a Record containing that
key; otherwise all branches receive the full input.  Output is `Record{branch_name: branch_output, ...}`.

### `Branch` — conditional routing

```json
{
  "op": "Branch",
  "predicate": { "op": "Stage", "id": "..." },
  "if_true":   { "op": "Stage", "id": "..." },
  "if_false":  { "op": "Stage", "id": "..." }
}
```

The predicate stage must output `Bool`.

### `Fanout` — broadcast

```json
{
  "op": "Fanout",
  "source":  { "op": "Stage", "id": "..." },
  "targets": [
    { "op": "Stage", "id": "..." },
    { "op": "Stage", "id": "..." }
  ]
}
```

Source output is sent to all targets concurrently.

### `Merge` — collect

```json
{
  "op": "Merge",
  "sources": [
    { "op": "Stage", "id": "..." },
    { "op": "Stage", "id": "..." }
  ],
  "target": { "op": "Stage", "id": "..." }
}
```

All sources run, then their outputs are collected into a List fed to `target`.

### `Retry`

```json
{
  "op": "Retry",
  "stage": { "op": "Stage", "id": "..." },
  "max_attempts": 3,
  "delay_ms": 500
}
```

Retries the inner node up to `max_attempts` times with an optional delay.

---

## Type checker

`check_graph(node, store)` walks the AST recursively and calls `is_subtype_of(a, b)`
at every edge:

```rust
fn check_graph(node: &CompositionNode, store: &dyn StageStore)
    -> Result<NType, TypeCheckError>
```

Returns the output `NType` of the root node — you can feed this into a downstream
graph for composed composition.

`is_subtype_of` is structural:

- `Record{a,b,c}` ≤ `Record{a,b}` — extra fields are fine (width subtyping)
- `Text` ≤ `Text | Null` — union member  
- `Any` ≤ anything, anything ≤ `Any` — escape hatch

---

## Planner

`plan_graph(node)` flattens the AST into a linear `ExecutionPlan`:

```rust
pub struct ExecutionPlan {
    pub steps: Vec<PlanStep>,
    pub parallelization_groups: Vec<Vec<usize>>,
    pub cost: CostEstimate,
}
```

The planner tracks data dependencies and groups independent steps into
`parallelization_groups` for concurrent execution.  Cost estimation sums the
`time_ms_p50` hints from stage metadata.

---

## Executor

The `StageExecutor` trait is pluggable:

```rust
pub trait StageExecutor {
    fn execute(&self, stage_id: &StageId, input: &Value)
        -> Result<Value, ExecutionError>;
}
```

| Executor | When used |
|---|---|
| `InlineExecutor` | Rust-native stdlib stages (Pure, no subprocess) |
| `NixExecutor` | Python/JS/bash stages in Nix sandbox |
| `MockExecutor` | Tests (returns first example output) |
| `CompositeExecutor` | Combines multiple executors by stage ID |

The `run_composition` runner uses the planner output to schedule execution,
routes data between steps, handles retries, and collects a `CompositionTrace`.

---

## Trace

Every execution produces a `CompositionTrace`:

```json
{
  "composition_id": "sha256hex",
  "started_at": "2026-04-06T10:30:00Z",
  "duration_ms": 312,
  "status": "Ok",
  "stages": [
    {
      "stage_id": "39731ebb",
      "status": "Ok",
      "input": { "url": "https://..." },
      "output": { "status": 200, "body": "..." },
      "duration_ms": 287
    }
  ]
}
```

Retrieve a past trace with:

```bash
noether trace <composition_id>
```
