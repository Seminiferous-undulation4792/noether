# Using a Remote Registry

By default, `noether` stores stages in a local JSON file (`~/.noether/store.json`).
This is great for development but has two limitations: stages don't survive a machine
wipe and they can't be shared with other agents or team members.

A **noether-cloud** registry solves both problems.  It is a persistent, content-addressed
HTTP store that any `noether` CLI (or AI agent) can read from and write to.

---

## Quick start

### 1. Point the CLI at the registry

```bash
export NOETHER_REGISTRY=https://registry.example.com
```

That is the only required change.  Every `noether` command now uses the remote
registry as its stage store:

```bash
# List active stages from the registry
noether stage list

# Semantic search against registry stages
noether stage search "parse JSON and extract a field"

# LLM-compose a graph using registry stages, then execute it
noether compose "download a URL, parse the JSON body, and extract the 'name' field"
```

You can also pass it per-command without setting the env var:

```bash
noether --registry https://registry.example.com stage list
```

### 2. Publish a custom stage

Build and sign a stage, then push it to the registry:

```bash
# Submit a stage spec (ACLI JSON)
noether stage submit stage.json

# Or use the API directly
curl -X POST https://registry.example.com/stages \
  -H "Content-Type: application/json" \
  -d @stage.json
```

The registry validates the submission:

- SHA-256 content hash must match the declared `id`
- Ed25519 signature is verified if present (unsigned stages receive a warning)
- Description must be non-empty
- Near-duplicate detection against the semantic index

These checks run as a **Noether composition** — the registry validates stages using
stages (the `verify_stage_content_hash`, `verify_stage_ed25519`,
`check_stage_description`, `check_stage_examples`, and `merge_validation_checks`
stdlib stages wired into a `Parallel + Sequential` graph).

### 3. Running noether-cloud locally (Docker Compose)

For local development or self-hosting:

```bash
# From the noether-cloud repo
cd noether-cloud/infra
docker compose up -d

# Postgres + registry will be at:
#   postgres://postgres:noether@localhost:5432/noether
#   http://localhost:8080
```

Then:

```bash
export NOETHER_REGISTRY=http://localhost:8080
noether stage list   # reads from local registry
```

---

## How the remote store works

```
noether CLI
    │
    ├── startup: GET /stages?lifecycle=active
    │           → populates local MemoryStore cache
    │
    ├── reads (stage list/get/search/compose/run)
    │           → served from in-memory cache (no network latency)
    │
    └── writes (stage submit / lifecycle update)
                → POST /stages or PATCH /stages/:id/lifecycle
                → also updates local cache
```

The CLI caches all active stages at startup via a single bulk fetch.
Reads are instant (in-memory cosine similarity, no round-trips).
Writes go to the remote first, then update the cache — so a successful write
is immediately visible to subsequent reads in the same session.

---

## Self-hosting noether-cloud

The registry is a standard Rust/Axum binary.  It supports two backends:

| Backend | When to use |
|---|---|
| JSON file (`NOETHER_STORE_PATH`) | Single instance, development, <10k stages |
| PostgreSQL (`DATABASE_URL`) | Production, multiple replicas, >10k stages |

```bash
# JSON file backend (default)
NOETHER_STORE_PATH=./data/registry.json noether-registry

# PostgreSQL backend
DATABASE_URL=postgres://user:pass@host/dbname noether-registry
```

The registry seeds the full stdlib (75 stages including the validation pipeline)
on first startup.  No migrations need to be run manually — the registry applies
them automatically on connect.

---

## Authentication

The current registry trusts any well-formed, content-hash-valid stage submission.
Stage *identity* is enforced cryptographically (SHA-256 + optional Ed25519).

Production deployments should place an API gateway or reverse proxy in front of
the registry to enforce authentication.  A `NOETHER_API_KEY` env var is checked
by the registry for write operations when set.

---

## Environment variables reference

| Variable | Description | Default |
|---|---|---|
| `NOETHER_REGISTRY` | Remote registry base URL | *(local file store)* |
| `NOETHER_API_KEY` | API key for write operations | *(none — open)* |
| `DATABASE_URL` | PostgreSQL connection string (server-side) | *(JSON file store)* |
| `NOETHER_STORE_PATH` | Path to local JSON store | `~/.noether/store.json` |
