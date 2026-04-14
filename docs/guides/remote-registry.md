# Remote Registry

A remote registry is a persistent, content-addressed HTTP store that any
`noether` CLI — or AI agent — can read from and write to. It solves two
limitations of the default local JSON store: stages don't survive a machine
wipe, and they can't be shared between developers or CI runners.

Noether ships with an implementation (`noether-registry`, in the
`noether-cloud` repo) and a public instance at `registry.alpibru.com`
you can use out-of-the-box.

---

## Using the public registry

Read access is open — no credentials needed, modeled on Docker Hub / npm /
crates.io. Authenticated writes are available on request.

```bash
export NOETHER_REGISTRY=https://registry.alpibru.com

noether stage list                       # browse stdlib + curated stages
noether stage search "parse CSV"         # semantic search
noether stage get <prefix>               # lookup by 8-char prefix
```

The CLI merges the remote set with your local store. Custom stages you've
run `noether stage add` against live in `~/.noether/store.json` and shadow
remote stages on ID collision.

Check the registry itself:

```bash
curl https://registry.alpibru.com/health
# { "ok": true, "result": { "status": "ok", "store": { "total_stages": 486, ... } } }

curl https://registry.alpibru.com/docs
```

## Publishing stages

Writes require an API key. Set `NOETHER_API_KEY` and every mutating command
(`stage add`, `stage sync`, `stage activate`) targets the remote:

```bash
export NOETHER_REGISTRY=https://registry.alpibru.com
export NOETHER_API_KEY=<your-key>

noether stage add my-stage.json          # one spec
noether stage sync ./stages/             # bulk-import a directory, idempotent
```

The registry validates the content hash, verifies any Ed25519 signature on
the spec, and auto-deprecates prior versions sharing the same canonical
identity (name + types + effects).

You can also post directly with `curl`:

```bash
curl -X POST https://registry.alpibru.com/stages \
  -H "X-API-Key: $NOETHER_API_KEY" \
  -H "Content-Type: application/json" \
  -d @my-stage.json
```

## API surface

All routes return ACLI envelopes (`{"ok": true/false, ...}`).

| Method | Path | Auth | Purpose |
|---|---|---|---|
| `GET` | `/stages` | public | paginated list (default 50, max 200) |
| `GET` | `/stages/{id}` | public | single stage by full ID |
| `GET` | `/stages/search?q=...` | public | semantic search (3-index fusion) |
| `GET` | `/health` | public | store stats + index size |
| `GET` | `/docs` | public | HTML API reference |
| `POST` | `/stages` | write | submit + validate |
| `DELETE` | `/stages/{id}` | write | remove |
| `PATCH` | `/stages/{id}/lifecycle` | write | promote, deprecate, tombstone |
| `POST` | `/compositions/run` | write | execute a graph, returns output + trace |

Pagination: `GET /stages?limit=200&offset=400`.

## Self-hosting

The registry binary is open source. One-shot local run:

```bash
# from the noether-cloud repo
cargo run --release --bin noether-registry
# → listening on 0.0.0.0:3000
```

Environment knobs:

| Env | Default | Purpose |
|---|---|---|
| `NOETHER_BIND` | `0.0.0.0:3000` | bind address |
| `DATABASE_URL` | — | if set, uses PostgreSQL; otherwise JSON file |
| `NOETHER_STORE_PATH` | `.noether/registry.json` | JSON file path |
| `NOETHER_STAGES_DIR` | — | load every `*.json` spec under this dir at boot |
| `NOETHER_EMBEDDING_PROVIDER` | auto-detect | `mistral` \| `openai` \| `vertex` \| `mock` |
| `NOETHER_EMBEDDING_CACHE` | `.noether/embeddings.json` | file-backed embedding cache |
| `NOETHER_EMBEDDING_BATCH` | `32` | batch size for embedding calls |
| `NOETHER_EMBEDDING_DELAY_MS` | `1100` | inter-batch sleep (set `100` on paid tiers) |
| `NOETHER_API_KEY` | — | required for writes; empty string disables auth |

Production deployment: see [`noether-cloud/infra/`](https://github.com/alpibrusl/noether-cloud/tree/main/infra)
for a reference `docker-compose.prod.yml` + Kubernetes manifests.

## Scheduled compositions

`noether-scheduler` runs Lagrange graphs on cron and fires a webhook with
the result. It's a separate binary that ships from the same repo as
`noether`:

```bash
cargo install noether-scheduler
# or download from github.com/alpibrusl/noether/releases/latest
```

Point it at a JSON config file. All three invocation forms work:

```bash
noether-scheduler --config scheduler.json    # flag
noether-scheduler scheduler.json             # positional
noether-scheduler                             # defaults to ./scheduler.json
```

Config format:

```json title="scheduler.json"
{
  "store_path": ".noether/registry.json",
  "registry_url": "https://registry.alpibru.com",
  "jobs": [
    {
      "name": "sprint-tick",
      "cron": "* * * * *",
      "graph": "compositions/sprint_tick.json",
      "input": { "sprint_id": "sprint-1" }
    },
    {
      "name": "hourly-health",
      "cron": "0 * * * *",
      "graph": "graphs/health-check.json",
      "webhook": "https://hooks.example.com/noether-health"
    }
  ]
}
```

Per-job fields:

| Field | Required | Purpose |
|---|---|---|
| `name` | yes | job identifier for logs + trace correlation |
| `cron` | yes | standard 5-field crontab expression |
| `graph` | yes | path to a Lagrange JSON graph |
| `input` | no | static JSON value passed as the graph's root input |
| `webhook` | no | POST the result (output + trace ID) to this URL |

Top-level fields: `store_path` for a local file store, or `registry_url`
(+ optional `registry_api_key`) to resolve stages against a remote
registry. Use one, not both.

## Troubleshooting

**`401 invalid or missing X-API-Key`** — either auth is enabled (you need
`NOETHER_API_KEY`), or your key is wrong. Read routes never return 401.

**`429 Too Many Requests`** from embedding provider on boot — bump
`NOETHER_EMBEDDING_DELAY_MS` (1100 ms for free tiers, 100 ms for paid
Mistral). Progressive caching ensures partial work survives crashes.

**New curated stage added to disk but not showing up** — the registry loads
`NOETHER_STAGES_DIR` only at boot. Restart the container, or `POST /stages`
directly. Spec changes without a corresponding `stage sync` will be
ignored.

**CLI talks to the wrong endpoint** — `NOETHER_REGISTRY` wins over the
per-command `--registry` flag. Unset the env var when testing a different
registry.
