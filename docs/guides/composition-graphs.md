# Composition Graphs (Lagrange JSON)

A composition graph is a JSON document that describes how stages connect.
Noether calls this format **Lagrange** — named after Joseph-Louis Lagrange,
whose formalism (the Lagrangian) is central to [Emmy Noether's theorem](../architecture/composition-engine.md#why-lagrange):
just as the Lagrangian describes a physical system and Noether's theorem derives
its conservation laws, a Lagrange graph describes a computation and Noether's
type checker derives its correctness guarantees.

## Operators

There are 9 operators:

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
| `Let` | Bind named sub-results and reference them in a body — solves the "carry an input field through a Sequential" problem |
| `RemoteStage` | Call a remote Noether HTTP endpoint |

> **Field-naming note.** Two operators that take child nodes use different
> shapes — this is intentional but trips up first-time graph authors:
>
> | Operator | Field | Shape | Why |
> |---|---|---|---|
> | `Sequential` | `stages` | array | order matters — pipeline order is preserved |
> | `Parallel` | `branches` | object | each child needs a name so its output can be addressed in the merged Record |
> | `Fanout` | `targets` | array | broadcast — order does not matter, but a list is the natural shape |
> | `Merge` | `sources` | array | union of inputs into a single target |
>
> Stage IDs may be **hand-authored as 8-character prefixes** (the same form
> `noether stage list` prints). The CLI resolves prefixes to the full
> 64-character SHA-256 at load time; ambiguous prefixes produce a clear
> error listing the matches.

## Stage

```json
{
  "op": "Stage",
  "id": "8f3a1b…"
}
```

Input is the output of the previous step (or the graph's top-level input if first).
The optional `config` field merges static parameters into the input record:

```json
{ "op": "Stage", "id": "8f3a1b…", "config": { "model": "gemini-2.5-flash" } }
```

## Sequential

```json
{
  "op": "Sequential",
  "stages": [
    { "op": "Stage", "id": "abc…" },
    { "op": "Stage", "id": "def…" }
  ]
}
```

`stages[0]` output → `stages[1]` input → … → last stage output is the Sequential's output.

> **Limitation — no field projection inside Sequential.** `Sequential`
> passes the *complete* output record of each stage as the input to the
> next. There is no inline `$input.*` reference syntax. To carry a field
> from the original input into a later stage, wrap the chain in a `Let`
> (see below) or include the field in every intermediate stage's output as
> a passthrough.

## Const

Inject a literal value anywhere in the graph:

```json
{
  "op": "Sequential",
  "stages": [
    { "op": "Const", "value": { "url": "https://api.example.com/data" } },
    { "op": "Stage", "id": "<http_get-hash>" }
  ]
}
```

## Parallel

Run named branches concurrently. Each branch receives `input[branch_name]` if
the input is a Record containing that key; otherwise it receives the full
input. Outputs are merged into a single Record keyed by branch name.

```json
{
  "op": "Parallel",
  "branches": {
    "left":  { "op": "Stage", "id": "branch-a-hash" },
    "right": { "op": "Stage", "id": "branch-b-hash" }
  }
}
```

The output is `{"left": <branch-a-output>, "right": <branch-b-output>}`.

## Branch

```json
{
  "op": "Branch",
  "predicate": { "op": "Stage", "id": "<bool-producing-stage>" },
  "if_true":   { "op": "Stage", "id": "<then-stage>" },
  "if_false":  { "op": "Stage", "id": "<else-stage>" }
}
```

## Retry

```json
{
  "op": "Retry",
  "stage": { "op": "Stage", "id": "<flaky-stage>" },
  "max_attempts": 3,
  "delay_ms": 1000
}
```

## Let

Bind named sub-results and reference them in a `body`. The classic use case
is the **scan → hash → diff** pattern, where `diff` needs a field
(`state_path`) from the *original* input that `hash` would otherwise erase.

```json
{
  "op": "Let",
  "bindings": {
    "scan":  { "op": "Stage", "id": "<scan-stage>" },
    "hash":  { "op": "Sequential", "stages": [
                 { "op": "Stage", "id": "<scan-stage>" },
                 { "op": "Stage", "id": "<hash-stage>" }
               ]}
  },
  "body": { "op": "Stage", "id": "<diff-stage>" }
}
```

Semantics:

- Every binding sub-graph receives the **outer Let input** (the same value
  given to the Let node). Bindings run **concurrently** — they cannot see
  each other's results. If you need ordering, wrap the chain in a
  `Sequential` inside the binding (as `hash` does above).
- After bindings complete, the `body` runs against an augmented record:
  `{ ...outer-input fields, "scan": <scan output>, "hash": <hash output> }`.
  A binding name shadows an outer-input field with the same name.
- The Let's output is the body's output.

The type checker treats `Let` as you'd expect: each binding is checked
against the outer input; the body is checked against the augmented record;
the Let's overall input requirement is the union of every binding's input
fields and every body field that isn't satisfied by a binding output.

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
