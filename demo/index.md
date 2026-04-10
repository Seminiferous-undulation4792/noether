# Noether — Type-safe Composition for AI Agents

When your AI coding assistant needs to build a data pipeline, it writes Python from scratch every time. 300 tokens for a CSV parser. 500 tokens for an API call + JSON extraction. Each time, from zero — no reuse, no type safety, no guarantee the code is correct until it runs.

Noether is different. Instead of generating code, it **composes pre-built, typed stages** into pipelines. The type checker validates every connection before anything executes. Stages are reusable — the same `csv_parse` stage works in every pipeline that needs CSV parsing.

## What an agent sees

An AI assistant uses Noether through a single command:

```bash
noether compose "parse CSV data and count the number of rows"
```

Noether searches its stage store for relevant building blocks, sends them to an LLM, and receives back a typed composition graph. The type checker validates it. If it passes, the executor runs it.

The output is structured JSON — the agent reads `ok`, gets the `output`, and moves on:

```json
{
  "ok": true,
  "data": {
    "output": "3.0",
    "trace": { "duration_ms": 0, "stages": [...] }
  }
}
```

No Python written. No code review needed. The pipeline is type-checked and reproducible.

---

## Demo 1: Compose and execute in one command

The simplest flow: describe what you need, get the result.

[![Demo 1: Compose and Execute](https://asciinema.org/a/zGgMmxgKpG78iUtH.svg)](https://asciinema.org/a/zGgMmxgKpG78iUtH)

```bash
# The agent asks Noether to compose a pipeline:
$ noether compose "parse CSV data and count the number of rows" \
    --input '{"text":"name,score\nAlice,95\nBob,72\nCarol,88","has_header":true,"delimiter":null}'

# Noether:
#   1. Searches 500+ stages for "parse CSV" and "count rows"
#   2. Sends top candidates to the LLM (Gemini, Claude, or GPT)
#   3. LLM returns: csv_parse → list_length (2-stage pipeline)
#   4. Type checker validates: Record{text,has_header,delimiter} → List<Map> → Number ✓
#   5. Executor runs it: output = 3.0

{
  "ok": true,
  "data": {
    "output": 3.0,
    "from_cache": false,
    "attempts": 1
  }
}
```

The second time the same problem is requested, it's **instant** — the composition is cached:

```bash
$ noether compose "parse CSV data and count the number of rows" --input '...'
# Cache hit — 0 LLM calls, 0 tokens, 0ms
```

---

## Demo 2: Type safety catches broken pipelines

This is Noether's key differentiator. Before any code runs, the type checker validates every connection.

[![Demo 2: Type Safety](https://asciinema.org/a/9TB5bLcqHigMbmA7.svg)](https://asciinema.org/a/9TB5bLcqHigMbmA7)

**A valid pipeline** — each stage's output type matches the next stage's input:

```bash
$ noether run --dry-run pipeline.json
# csv_parse:    Record{text, has_header, delimiter} → List<Map<Text,Text>>
# list_length:  List<Any> → Number
# to_text:      Any → Text
# ✓ Type check passed. All edges are compatible.
```

**A broken pipeline** — feeding a Number into csv_parse (which expects a Record):

```bash
$ noether run --dry-run broken.json
# list_length: List<Any> → Number
# csv_parse:   expects Record{text, has_header, delimiter}
# ✗ Type error: Number is not subtype of Record{text, has_header, delimiter}
```

The broken pipeline **never executes**. No wasted compute, no runtime crash, no debugging. The error is caught in under 1ms.

In traditional code generation, this bug surfaces only at runtime — after the agent generated the code, ran it, read the traceback, and tried to fix it. That cycle costs tokens and time.

---

## Demo 3: Parallel processing preserves data

When you chain stages sequentially, each one transforms the data — and the original is lost:

```
text → text_length → 42 → text_upper → ???
                     ↑ the text is gone, only a number remains
```

Noether's `Parallel` operator runs multiple branches on the **same input**:

[![Demo 3: Parallel Processing](https://asciinema.org/a/E0MdzCOx24zYIXu7.svg)](https://asciinema.org/a/E0MdzCOx24zYIXu7)

```bash
$ noether run parallel-graph.json --input '"Noether is a composition platform."'

# 4 branches run concurrently, each receives the FULL text:
#   char_count  → 35.0
#   uppercased  → "NOETHER IS A COMPOSITION PLATFORM."
#   reversed    → ".mroftalp noitisopmoc a si rehtoN"
#   trimmed     → "Noether is a composition platform."

# Results merge into a single record:
{
  "char_count": 35.0,
  "uppercased": "NOETHER IS A COMPOSITION PLATFORM.",
  "reversed": ".mroftalp noitisopmoc a si rehtoN",
  "trimmed": "Noether is a composition platform."
}
```

Every branch gets the original text. No data loss. No intermediate variables. The executor runs them concurrently.

---

## Demo 4: Reuse without duplication

Three different analyses on the same CSV data. The `csv_parse` stage appears in all three — but it's defined once, tested once, and reused everywhere.

[![Demo 4: Stage Reuse](https://asciinema.org/a/7f1Ri88zn1TxslDP.svg)](https://asciinema.org/a/7f1Ri88zn1TxslDP)

```bash
# Pipeline A: CSV → count rows
$ noether run pipeline-a.json --input '...'
→ 7.0 rows

# Pipeline B: CSV → JSON (reuses csv_parse)
$ noether run pipeline-b.json --input '...'
→ [{"status":"open","priority":"high","assignee":"alice"}, ...]

# Pipeline C: CSV → deduplicate → count unique (reuses csv_parse + list_length)
$ noether run pipeline-c.json --input '...'
→ 6.0 unique rows
```

**Stage usage across pipelines:**

| Stage | Pipeline A | Pipeline B | Pipeline C |
|-------|:---:|:---:|:---:|
| csv_parse | ✓ | ✓ | ✓ |
| list_length | ✓ | | ✓ |
| to_text | ✓ | | ✓ |
| json_serialize | | ✓ | |
| list_dedup | | | ✓ |

5 unique stages. 9 total usages. **Zero lines of code written.**

With code generation, each pipeline would need its own CSV parsing code (~300 tokens each). With composition, pipelines B and C reuse A's stages at zero additional cost.

---

## How your coding assistant uses it

Add this to your project's `CLAUDE.md` (or equivalent):

```markdown
When building data pipelines or multi-step transformations,
use `noether compose "description"` instead of writing code.

If compose fails, fall back to writing Python.
```

That's it. The agent discovers Noether through the ACLI protocol:

```bash
# Agent's first call — discovers all commands:
noether introspect

# Agent searches for relevant stages:
noether stage search "parse CSV"

# Agent composes a pipeline:
noether compose "parse CSV and count rows" --input '...'

# Agent reads the result:
# { "ok": true, "data": { "output": 3.0 } }
```

Every response is JSON with an `ok` field. The agent branches on success/failure without parsing exit codes or stderr.

---

## Token cost comparison

| Pipeline variations | Compose (Noether) | Generate (code) |
|---|---|---|
| 1 | ~2,150 tokens | ~600 tokens |
| 2 | ~2,300 tokens | ~1,200 tokens |
| 3 | ~2,450 tokens | ~1,800 tokens |
| **4** | **~2,600 tokens** | **~2,400 tokens** |
| 5 | ~2,750 tokens | ~3,000 tokens |
| 10 | ~3,500 tokens | ~6,000 tokens |

Noether costs more for a single pipeline but **saves tokens at 4+ variations** — and agents iterate constantly.

Plus: compose results are cached. The second time the same problem appears, it costs **0 tokens**.

---

## Try it

```bash
# Build from source (requires Rust toolchain)
git clone https://github.com/alpibrusl/noether solv-noether
cd solv-noether && cargo build --release -p noether-cli
export PATH="$PWD/target/release:$PATH"

# Verify it works
noether stage search "parse CSV"

# Set up an LLM provider (pick one)
export VERTEX_AI_PROJECT=your-project VERTEX_AI_MODEL=gemini-2.5-flash
# or: export OPENAI_API_KEY=sk-...
# or: export ANTHROPIC_API_KEY=sk-ant-...

# Compose your first pipeline
noether compose "parse CSV data and count rows"
```
