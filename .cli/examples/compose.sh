#!/usr/bin/env bash
# .cli/examples/compose.sh
# Example noether compose invocations for agent reference.

set -euo pipefail

# ── Basic composition ──────────────────────────────────────────────────────

# Compose and execute in one call
noether compose "parse a CSV file and extract all email addresses"

# Dry-run: inspect the graph and estimated cost before committing
noether compose --dry-run "sort a list of numbers in descending order"

# Pass input data
noether compose \
  --input '{"csv": "name,email\nAlice,alice@example.com\nBob,bob@example.com"}' \
  "extract email addresses from this CSV"

# ── Model selection ────────────────────────────────────────────────────────

noether compose --model gemini-2.5-flash "summarise this text"
noether compose --model mistral-small-2503 "convert markdown to plain text"

# ── Capability control ─────────────────────────────────────────────────────

# Only allow network access (block filesystem, GPU, LLM stages)
noether compose \
  --allow-capabilities network \
  "fetch the current price of bitcoin from a public API"

# Allow network and LLM together
noether compose \
  --allow-capabilities network,llm \
  "fetch weather for London and summarise it"

# ── Cache control ──────────────────────────────────────────────────────────

# Re-compose without hitting the cache (e.g. after adding new stages)
noether compose --force "sort a list of numbers"

# ── JSON output (for agent use) ────────────────────────────────────────────

# All output is already JSON; pipe into jq for filtering
noether compose "count words in text" \
  --input '"the quick brown fox"' \
  | jq '.data.output'
