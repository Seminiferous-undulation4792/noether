# Stage Store Build — Team Instructions

> **Audience**: Team tasked with filling the Noether stage store with high-quality, reusable stages from open-source libraries.
>
> **Goal**: Produce ~390 new stages across 50 target repositories, publish them to the Noether registry, and tag them so they are discoverable via semantic search.

---

## 1. Context

Noether is a typed, composable stage execution platform. A **stage** is the atomic unit: it has a declared input type, output type, effect set, examples, and an implementation. Stages live in a **store** (local JSON or remote registry) and are composed into graphs.

The store currently ships **80 stdlib stages** (built in Rust). This project extends the store with **Python implementations** of common open-source library operations — each operation becomes one stage.

Think of it like publishing packages to npm, but instead of full libraries you publish individual, well-typed functions.

---

## 2. Quick start

```bash
# Clone the Noether repo (you need the CLI)
git clone https://github.com/alpibrusl/noether
cd noether
cargo build --release -p noether-cli
export PATH="$PWD/target/release:$PATH"

# Verify the local store (created automatically on first use)
noether store stats          # should show 80 stdlib stages

# Verify the CLI works
noether stage list --tag io  # shows IO stages
noether stage search "pdf extract text"
```

The CLI binary is `noether`. All commands output JSON; pipe through `jq` for readability.

---

## 3. Anatomy of a stage spec file

Each stage you build is described in a JSON spec file, then registered with `noether stage add <file.json>`.

### Simple spec format (recommended)

```json
{
  "name": "html_extract_text",
  "description": "Extract all visible text from an HTML string, stripping tags",
  "input": { "Record": [["html", "Text"]] },
  "output": "Text",
  "effects": ["Fallible"],
  "language": "python",
  "implementation": "from bs4 import BeautifulSoup\nimport sys, json\ndata = json.load(sys.stdin)\nsoup = BeautifulSoup(data['html'], 'html.parser')\nprint(json.dumps(soup.get_text(separator=' ', strip=True)))",
  "examples": [
    { "input": {"html": "<h1>Hello</h1><p>World</p>"}, "output": "Hello World" },
    { "input": {"html": "<div><span>foo</span> <span>bar</span></div>"}, "output": "foo bar" },
    { "input": {"html": ""}, "output": "" },
    { "input": {"html": "<p>  spaces  </p>"}, "output": "spaces" },
    { "input": {"html": "no tags at all"}, "output": "no tags at all" }
  ],
  "tags": ["web", "html", "text", "pure"],
  "aliases": ["html_to_text", "strip_html", "html_get_text"]
}
```

### Type syntax reference

| JSON value | Meaning |
|---|---|
| `"Text"` | String |
| `"Number"` | Float64 |
| `"Bool"` | Boolean |
| `"Null"` | null |
| `"Any"` | unconstrained |
| `"Bytes"` | binary blob (base64 encoded) |
| `{ "Record": [["field", "Type"], ...] }` | Struct with named fields |
| `{ "List": "Type" }` | Homogeneous list |
| `{ "Map": ["Text", "Type"] }` | String-keyed map |
| `{ "Union": ["Type", "Null"] }` | Nullable value (optionality is expressed as a union with Null) |
| `{ "Union": ["TypeA", "TypeB"] }` | One of several types |
| `{ "Stream": "Type" }` | Streaming sequence of values |

> **Note**: there is no `Optional` type. Use `{"Union": ["YourType", "Null"]}` for nullable fields.
>
> **Both formats accepted**: The CLI accepts the simplified syntax shown above **and** the canonical `{"kind": "Text"}` / `{"kind": "Record", "value": {"field": {"kind": "Text"}}}` format used internally. Use whichever you prefer — the CLI normalizes automatically.

### Effect tags

| Tag | Meaning |
|---|---|
| `"Pure"` | No side effects (safe to cache/replay) |
| `"Fallible"` | Can return an error |
| `"Network"` | Makes external HTTP/socket calls |
| `"Process"` | Spawns or signals OS processes |
| `"NonDeterministic"` | Output may vary for same input |

All five effects are accepted by `noether stage add`. `"Process"` is unusual for library-wrapper stages — only use it if the stage explicitly spawns or manages an OS process.

**Rule of thumb**: most library wrappers should have `["Fallible"]`. Pure transformation stages should have `["Pure"]`. Anything that calls an external API needs `["Network", "Fallible"]`.

---

## 4. Implementation conventions

### Python stages

Noether executes Python stages in an isolated Nix environment. The stage receives its input as JSON on **stdin** and must write its output as a single JSON value to **stdout**.

```python
import sys, json

data = json.load(sys.stdin)

# ... your logic ...

result = {"key": "value"}
print(json.dumps(result))
```

**Dependencies**: List any pip packages in a comment at the top:
```python
# requires: beautifulsoup4==4.12.3, lxml==5.1.0
```

Noether's Nix executor reads this comment and installs packages before first run.

> **JavaScript**: A Node.js executor is not yet implemented. All stages in this project should use `"language": "python"`.

### Error handling

A stage signals failure by writing to **stderr** and exiting with a non-zero code:

```python
import sys, json
data = json.load(sys.stdin)
if not data.get("url"):
    print("url field is required", file=sys.stderr)
    sys.exit(1)
```

---

## 5. Tags and aliases

Every stage you create must have tags and aliases in the spec. This is how stages appear in search.

Add them to your spec JSON:

```json
{
  "name": "html_extract_text",
  "tags": ["web", "html", "text", "pure"],
  "aliases": ["html_to_text", "strip_html", "html_get_text"],
  ...
}
```

**Tagging guide**:

| Category | Tags to use |
|---|---|
| Text processing | `text`, `string`, `pure` |
| HTML/web scraping | `web`, `html`, `network` |
| Data / DataFrames | `data`, `analytics`, `pure` |
| Validation | `validation`, `pure` |
| Files | `io`, `filesystem`, `file` |
| PDF/Office docs | `media`, `document` |
| Images/video | `media`, `image`/`video` |
| Git/GitHub | `devtools`, `git`, `vcs` |
| LLM/AI | `llm`, `ai`, `non-deterministic` |
| Infrastructure | `infra`, `cloud` |
| Crypto/security | `crypto`, `security`, `pure` |

**Alias guide**: put alternative names people would type when searching:
- Common abbreviations: `strlen` for `text_length`
- Other library function names: `pd_groupby` for `df_group_by`
- Verb alternatives: `fetch` for `http_get`, `parse` for `csv_parse`

---

## 6. The 50 target repositories

Work through these in priority order. Each repo has an estimated stage count and a list of stage names to implement.

> **Type shorthand in tables**: `Type?` is used as a compact notation for `{"Union": ["Type", "Null"]}` (a nullable field). Use the full union form when writing actual spec JSON.

---

### Priority 1 — implement first

#### `jq (jq.py)` · 5 stages · [github.com/mwilliamson/jq.py](https://github.com/mwilliamson/jq.py)

jq is the gold standard for JSON transformation. These stages fill the largest single gap in the current store.

| Stage | Input | Output | Description |
|---|---|---|---|
| `jq_transform` | `{data: Any, expr: Text}` | `Any` | Apply a jq expression to any JSON value |
| `jq_keys` | `Any` | `List<Text>` | List keys of a JSON object (`.keys`) |
| `jq_select` | `{data: Any, filter: Text}` | `List<Any>` | Select elements matching a filter |
| `jq_map` | `{data: List<Any>, expr: Text}` | `List<Any>` | Map a jq expression over a list |
| `jq_reduce` | `{data: Any, expr: Text, init: Any}` | `Any` | Fold with a jq reduce expression |

```python
# requires: jq==1.6.0
import sys, json, jq
data = json.load(sys.stdin)
result = jq.first(data["expr"], data["data"])
print(json.dumps(result))
```

---

#### `instructor` · 3 stages · [github.com/jxnl/instructor](https://github.com/jxnl/instructor)

Structured LLM extraction with Pydantic validation and auto-retry. `llm_extract_structured` is the most requested missing stage.

| Stage | Input | Output | Description |
|---|---|---|---|
| `llm_extract_structured` | `{text: Text, schema: Any, model: {"Union": ["Text", "Null"]}?}` | `{extracted: Any, model: Text}` | Extract data from text matching a JSON schema, with LLM retry on failure |
| `llm_schema_retry` | `{prompt: Text, schema: Any, max_retries: {"Union": ["Number", "Null"]}}` | `Any` | Call LLM and retry until output matches schema |
| `llm_partial_extract` | `{text: Text, fields: List<Text>}` | `Any` | Extract specific named fields from text |

---

#### `tiktoken` · 4 stages · [github.com/openai/tiktoken](https://github.com/openai/tiktoken)

Token counting and context chunking — critical for any LLM cost management composition.

| Stage | Input | Output | Description |
|---|---|---|---|
| `token_count` | `{text: Text, model: Text?}` | `Number` | Count tokens for a given model (default: gpt-4o) |
| `chunk_for_context` | `{text: Text, max_tokens: Number, model: Text?}` | `List<Text>` | Split text into chunks that fit within token limit |
| `chunk_overlap` | `{text: Text, chunk_size: Number, overlap: Number}` | `List<Text>` | Split with overlapping windows for RAG chunking |
| `estimate_llm_cost` | `{tokens: Number, model: Text}` | `{cost_usd: Number, model: Text}` | Estimate API cost from token count |

---

#### `httpx` · 8 stages · [github.com/encode/httpx](https://github.com/encode/httpx)

Upgrades our existing http_get/post with auth, retries, streaming, and GraphQL.

| Stage | Input | Output | Description |
|---|---|---|---|
| `http_bearer_get` | `{url: Text, token: Text, headers: Map?}` | `{status: Number, body: Text, headers: Map}` | GET with Bearer token auth |
| `http_basic_auth` | `{url: Text, username: Text, password: Text, body: Text?}` | `{status: Number, body: Text}` | GET/POST with HTTP Basic auth |
| `http_retry_get` | `{url: Text, max_retries: Number, backoff_ms: Number?}` | `{status: Number, body: Text}` | GET with exponential backoff retry |
| `http_graphql` | `{url: Text, query: Text, variables: Any?, token: Text?}` | `{data: Any, errors: Any}` | Execute a GraphQL query |
| `http_delete` | `{url: Text, headers: Map?}` | `{status: Number, body: Text}` | HTTP DELETE |
| `http_patch` | `{url: Text, body: Text, headers: Map?}` | `{status: Number, body: Text}` | HTTP PATCH |
| `http_put` | `{url: Text, body: Text, headers: Map?}` | `{status: Number, body: Text}` | HTTP PUT |
| `http_head` | `{url: Text, headers: Map?}` | `{status: Number, headers: Map}` | HTTP HEAD — return headers only, no body |

---

#### `BeautifulSoup4` · 8 stages · [pypi.org/project/beautifulsoup4](https://pypi.org/project/beautifulsoup4/)

HTML parsing and extraction — essential for any web data pipeline.

| Stage | Input | Output | Description |
|---|---|---|---|
| `html_extract_text` | `{html: Text}` | `Text` | Extract all visible text, strip tags |
| `html_select_css` | `{html: Text, selector: Text}` | `List<Text>` | Select elements by CSS selector, return their text |
| `html_get_links` | `{html: Text, base_url: Text?}` | `List<{text: Text, href: Text}>` | Extract all `<a>` links |
| `html_get_tables` | `{html: Text}` | `List<List<List<Text>>>` | Extract HTML tables as 2D arrays |
| `html_get_meta` | `{html: Text}` | `Map<Text, Text>` | Extract `<meta>` tag name/content pairs |
| `html_strip_tags` | `{html: Text, keep_tags: List<Text>?}` | `Text` | Strip all tags except specified keep-list |
| `html_extract_og` | `{html: Text}` | `{title: Text?, description: Text?, image: Text?}` | Extract Open Graph metadata |
| `html_count_elements` | `{html: Text, selector: Text}` | `Number` | Count elements matching a CSS selector |

---

#### `PyMuPDF (fitz)` · 8 stages · [github.com/pymupdf/PyMuPDF](https://github.com/pymupdf/PyMuPDF)

PDF extraction. Probably the most needed document-processing stage group.

| Stage | Input | Output | Description |
|---|---|---|---|
| `pdf_extract_text` | `{path: Text}` | `Text` | Extract all text from a PDF |
| `pdf_page_count` | `{path: Text}` | `Number` | Return number of pages |
| `pdf_extract_tables` | `{path: Text, page: Number?}` | `List<List<List<Text>>>` | Extract tables from PDF pages |
| `pdf_to_images` | `{path: Text, dpi: Number?}` | `List<Bytes>` | Render each page to a PNG image |
| `pdf_get_metadata` | `{path: Text}` | `{title: Any, author: Any, pages: Number}` | Get PDF metadata |
| `pdf_split_pages` | `{path: Text, output_dir: Text}` | `List<Text>` | Split into individual page PDFs |
| `pdf_extract_page` | `{path: Text, page: Number}` | `Text` | Extract text from a specific page |
| `pdf_extract_links` | `{path: Text, page: Number?}` | `List<{uri: Text, page: Any}>` | Extract all hyperlinks and internal links |

---

#### `PyGithub` · 14 stages · [github.com/PyGithub/PyGithub](https://github.com/PyGithub/PyGithub)

Full GitHub automation surface. Needs a `GITHUB_TOKEN` env var.

| Stage | Description |
|---|---|
| `gh_create_issue` | Create an issue with title, body, labels, assignees |
| `gh_list_prs` | List open/closed PRs with filters |
| `gh_post_comment` | Post a comment on an issue or PR |
| `gh_merge_pr` | Merge a PR (squash/merge/rebase) |
| `gh_get_file` | Read a file's contents at a ref |
| `gh_create_branch` | Create a branch from a ref |
| `gh_list_checks` | List CI check runs for a commit |
| `gh_add_label` | Add labels to an issue/PR |
| `gh_close_issue` | Close an issue with optional comment |
| `gh_search_issues` | Search issues by query string |
| `gh_get_diff` | Get the diff of a PR |
| `gh_list_commits` | List commits on a branch |
| `gh_create_release` | Create a tagged release with notes |
| `gh_get_repo_stats` | Stars, forks, open issues, last push |

---

#### `polars` · 15 stages · [github.com/pola-rs/polars](https://github.com/pola-rs/polars)

DataFrame operations as pure stages. Use the Python API but the underlying execution is Rust.

| Stage | Input | Output | Description |
|---|---|---|---|
| `df_from_records` | `List<Any>` | `Bytes` | Build a DataFrame from a list of records (Arrow IPC output) |
| `df_to_records` | `Bytes` | `List<Any>` | Convert Arrow IPC DataFrame back to records |
| `df_select` | `{df: Bytes, columns: List<Text>}` | `Bytes` | Select columns |
| `df_filter` | `{df: Bytes, expr: Text}` | `Bytes` | Filter rows by a polars expression string |
| `df_group_by` | `{df: Bytes, by: List<Text>, aggs: List<Text>}` | `Bytes` | Group and aggregate |
| `df_join` | `{left: Bytes, right: Bytes, on: Text, how: Text?}` | `Bytes` | Join two DataFrames |
| `df_sort` | `{df: Bytes, by: List<Text>, descending: Bool?}` | `Bytes` | Sort rows |
| `df_pivot` | `{df: Bytes, index: Text, columns: Text, values: Text}` | `Bytes` | Pivot table |
| `df_melt` | `{df: Bytes, id_vars: List<Text>}` | `Bytes` | Melt wide to long format |
| `df_describe` | `{df: Bytes}` | `Any` | Summary statistics |
| `df_schema` | `{df: Bytes}` | `Map<Text, Text>` | Column names and types |
| `df_sample` | `{df: Bytes, n: Number?, fraction: Number?}` | `Bytes` | Random sample |
| `df_to_csv` | `{df: Bytes}` | `Text` | Serialize to CSV string |
| `df_from_csv` | `{csv: Text}` | `Bytes` | Parse CSV into DataFrame |
| `df_shape` | `{df: Bytes}` | `{rows: Number, cols: Number}` | Row and column counts |

---

### Priority 2 — implement after Priority 1

#### `boto3 (AWS SDK)` · 12 stages
S3, Lambda, SQS, SNS, SecretsManager. Needs `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` env vars.

Key stages: `s3_put`, `s3_get`, `s3_list`, `s3_delete`, `lambda_invoke`, `sqs_send`, `sqs_receive`, `sns_publish`, `secretsmanager_get`, `ecr_push`

#### `sentence-transformers` · 6 stages
Local semantic embeddings and similarity. No API key needed, but the first run downloads a ~400 MB model — ensure the Nix executor has sufficient disk space and network access for the initial fetch.

Key stages: `sentence_embed`, `semantic_similarity`, `cross_encode`, `cluster_sentences`, `paraphrase_detect`

#### `GitPython` · 12 stages
Programmatic git. Works on any local repo.

Key stages: `git_log`, `git_diff`, `git_blame`, `git_commit`, `git_branch`, `git_tag`, `git_status`, `git_stash`, `git_cherry_pick`, `git_show`

#### `Great Expectations` · 12 stages
Data quality expectations. Each expectation is a pure validation stage.

Key stages: `expect_col_not_null`, `expect_col_unique`, `expect_row_count`, `expect_col_type`, `expect_val_in_set`, `expect_col_regex`

#### `Faker` · 14 stages
Synthetic data generation. Stages use `["NonDeterministic"]` — output is random by design.

Key stages: `fake_name`, `fake_email`, `fake_address`, `fake_company`, `fake_phone`, `fake_uuid`, `fake_paragraph`, `fake_date`, `fake_url`, `fake_ip`, `fake_credit_card`

#### `spaCy` · 12 stages
Industrial-strength NLP. Requires spacy model download.

Key stages: `ner_extract`, `pos_tag`, `sentence_split`, `tokenize`, `dep_parse`, `lemmatize`, `coref_resolve`

---

### Priority 3 — remaining repos (work in parallel)

| Repo | Est. stages | Key stages |
|---|---|---|
| NLTK | 10 | `stem`, `stopword_remove`, `ngrams`, `word_freq` |
| pydantic | 6 | `schema_validate`, `schema_coerce`, `model_to_dict` |
| Jinja2 | 5 | `jinja_render`, `jinja_render_file`, `jinja_validate` |
| Python AST | 8 | `code_extract_functions`, `code_extract_imports`, `code_count_complexity` |
| docker-py | 10 | `docker_run`, `docker_stop`, `docker_logs`, `docker_exec` |
| kubernetes-client | 10 | `k8s_deploy`, `k8s_get_pods`, `k8s_tail_logs`, `k8s_exec` |
| Pillow | 9 | `image_resize`, `image_crop`, `image_convert`, `image_thumbnail` |
| paramiko | 5 | `ssh_exec`, `sftp_upload`, `sftp_download` |
| Playwright | 8 | `browser_navigate`, `browser_screenshot`, `browser_click`, `browser_fill` |
| mistune | 7 | `markdown_to_html`, `extract_headings`, `extract_links` |
| dateparser | 5 | `parse_natural_date`, `relative_date` |
| semver | 6 | `semver_parse`, `semver_bump_*`, `semver_satisfies` |
| textstat | 8 | `flesch_score`, `kincaid_grade`, `reading_time` |
| diff-match-patch | 5 | `text_diff`, `patch_apply`, `levenshtein` |
| sqlite-utils | 8 | `sql_insert_rows`, `sql_query`, `sql_upsert` |
| tabulate | 6 | `table_to_markdown`, `table_to_html` |
| feedparser | 5 | `rss_fetch`, `feed_to_article_list` |
| python-jose | 6 | `jwt_encode`, `jwt_decode`, `jwt_verify` |
| openpyxl | 6 | `xlsx_read_sheet`, `xlsx_write_rows`, `xlsx_to_records` |
| twilio-python | 5 | `twilio_send_sms`, `twilio_send_email` |
| stripe-python | 10 | `stripe_charge`, `stripe_refund`, `stripe_create_customer` |
| Guardrails AI | 6 | `guardrail_validate`, `pii_detect`, `pii_redact` |
| ChromaDB | 7 | `vector_upsert`, `vector_query`, `vector_delete` |
| Pygments | 5 | `code_highlight_html`, `code_tokenize`, `detect_language` |
| pandera | 7 | `df_schema_check`, `col_type_check`, `col_range_check` |
| dbt-core | 8 | `sql_render_model`, `sql_incremental` |
| DSPy | 6 | `dspy_predict`, `dspy_chain_of_thought` |
| pypandoc | 8 | `md_to_docx`, `md_to_pdf`, `html_to_md` |
| Prometheus client | 5 | `counter_inc`, `gauge_set`, `histogram_observe` |
| python-docx | 6 | `docx_extract_text`, `docx_extract_tables` |
| moviepy | 7 | `video_trim`, `video_extract_audio`, `video_get_metadata` |
| ftfy | 4 | `fix_encoding`, `normalize_unicode` |
| jsonpath-ng | 6 | `jsonpath_find`, `jsonpath_update` |
| croniter | 4 | `cron_next`, `cron_is_due` |
| hypothesis | 5 | `gen_int_list`, `gen_text`, `gen_record` |
| cerberus | 5 | `cerberus_validate`, `cerberus_coerce` |
| cookiecutter | 4 | `project_scaffold`, `template_render_dir` |
| loguru | 4 | `log_parse_entry`, `log_to_json` |

---

## 7. Registering stages

Once you have a spec file, register it:

```bash
# Add a single stage
noether stage add ./stages/html_extract_text.json

# Verify it was added
noether stage search "html extract text"

# Browse by tag
noether stage list --tag web

# Check it has good coverage
noether stage get <id-prefix>
```

### Batch registration script

```bash
#!/bin/bash
# register_all.sh — run from a folder of spec JSON files
PASS=0; FAIL=0
for f in stages/*.json; do
  if noether stage add "$f" 2>/dev/null; then
    PASS=$((PASS+1))
  else
    FAIL=$((FAIL+1))
    echo "FAILED: $f"
  fi
done
echo "Done: $PASS added, $FAIL failed"
```

### Publishing to the remote registry

```bash
# Point at the remote registry instead of local store
noether --registry https://registry.noether.dev stage add ./stages/html_extract_text.json
```

### Activating stages

New stages are registered as **Draft** and won't appear in compositions until promoted to **Active**:

```bash
# Activate a single stage (supports ID prefix)
noether stage activate f69c9aca

# Batch register + activate
./stages/register_all.sh --activate

# Activate via remote registry
NOETHER_REGISTRY=https://registry.noether.dev noether stage activate <id>
```

Valid lifecycle transitions: `Draft → Active → Deprecated → Tombstone`. The `activate` command only promotes Draft stages.

---

## 8. Quality checklist per stage

Before submitting, verify:

- [ ] **At least 5 examples** covering normal cases, edge cases, and error cases
- [ ] **Correct effects** — does it use the network? Can it fail? Is it non-deterministic?
- [ ] **Accurate type signatures** — use `{"Union": ["YourType", "Null"]}` for nullable fields, not `Any`
- [ ] **At least 2 tags** and **at least 2 aliases** (required)
- [ ] **Description is a single sentence** starting with a verb ("Extract…", "Convert…", "Return…")
- [ ] **Error path**: does the stage exit non-zero and write to stderr on failure?
- [ ] **No hardcoded credentials** in implementation code

### Near-duplicate check

Before adding, search for existing similar stages:

```bash
noether stage search "your description here"
```

If a stage with score > 0.92 and the same type signature exists, your stage is a duplicate — don't add it.

---

## 9. Folder layout for your work

Suggested structure for the team:

```
stages/
  web/
    html_extract_text.json
    html_get_links.json
    html_select_css.json
    http_bearer_get.json
    ...
  data/
    jq_transform.json
    df_from_records.json
    ...
  ai/
    llm_extract_structured.json
    token_count.json
    ...
  devtools/
    git_log.json
    gh_create_issue.json
    ...
  media/
    pdf_extract_text.json
    image_resize.json
    ...
register_all.sh
README.md        ← per-team notes, blockers, decisions
```

---

## 10. Reference: full stage spec fields

```json
{
  "name": "stage_name",
  "description": "One-sentence description starting with a verb",
  "input": "Type or {Record: ...}",
  "output": "Type or {Record: ...}",
  "effects": ["Pure|Fallible|Network|Process|NonDeterministic"],
  "language": "python",
  "implementation": "...python code...",
  "examples": [
    { "input": ..., "output": ... }
  ],
  "tags": ["tag1", "tag2"],
  "aliases": ["alias1", "alias2"]
}
```

All fields are required. `tags` and `aliases` must each contain at least 2 entries. `examples` needs at minimum 5 entries (3 happy-path, 1 edge case, 1 error case). The `input`/`output` fields are deserialized as `NType` — there is no `Optional` variant; express nullable fields as `{"Union": ["YourType", "Null"]}`.

---

## 11. Getting help

- **Noether docs**: `noether/docs/`
- **Type system reference**: `docs/architecture/type-system.md`
- **Existing stdlib stages** (80 examples to learn from): `crates/noether-core/src/stdlib/`
- **Stage spec parsing code**: `crates/noether-cli/src/commands/stage.rs`
- **Search API**: `noether stage search "<query>"` or `GET /search?q=<query>` on the registry
