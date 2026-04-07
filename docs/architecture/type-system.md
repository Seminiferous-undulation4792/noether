# Type System

Noether uses **structural typing** — two types are compatible if their structure matches, regardless of name.

## NType

```rust
pub enum NType {
    Text,
    Number,
    Bool,
    Bytes,
    Null,
    List(Box<NType>),
    Map(Box<NType>),
    Record(BTreeMap<String, NType>),
    Union(BTreeSet<NType>),
    Stream(Box<NType>),
    Any,
}
```

## Subtyping rules

| Rule | Example |
|---|---|
| `T <: T` | `Text <: Text` |
| `T <: Any`, `Any <: T` | Any is a bidirectional escape hatch |
| Width subtyping | `Record { a, b, c } <: Record { a, b }` |
| Depth subtyping | `Record { x: Number } <: Record { x: Any }` |
| List covariance | `List<Number> <: List<Any>` |
| Union | `Text <: Text \| Number`, `Text \| Number <: Any` |

!!! warning "Any is explicit"
    `Any` is an intentional escape hatch, not a default. Prefer concrete types in stage signatures for better composition safety.

## Record subtyping in practice

The most important rule for composition is **width subtyping**. A stage that outputs `Record { url, status, body }` can feed into a stage that only requires `Record { url, body }` — the extra `status` field is ignored.

This enables **adapter-free composition**: you rarely need explicit projection stages.

## Union normalisation

`NType::union()` is the only constructor for unions. It:
1. Flattens nested unions: `Union(Union(A, B), C) → Union(A, B, C)`
2. Deduplicates members.
3. Sorts members for deterministic canonical form.

Never construct `NType::Union(...)` directly.

## Type inference

`infer_type(value: &serde_json::Value) → NType` maps JSON values to NTypes:

| JSON | NType |
|---|---|
| `"hello"` | `Text` |
| `42`, `3.14` | `Number` |
| `true` | `Bool` |
| `null` | `Null` |
| `[1, 2, 3]` | `List(Number)` |
| `{"a": 1}` | `Record { a: Number }` |
| `[1, "x"]` | `List(Number \| Text)` |
