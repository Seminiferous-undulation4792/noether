#!/usr/bin/env bash
# .cli/examples/run.sh
# Example noether run invocations for agent reference.

set -euo pipefail

# ── Execute a pre-composed graph ───────────────────────────────────────────

# Run with default null input
noether run graph.json

# Pass input as JSON string
noether run rail-search.json \
  --input '{"from": "Madrid", "to": "Barcelona", "date": "2026-04-10"}'

# ── Dry-run: type-check and plan only ─────────────────────────────────────

noether run graph.json --dry-run

# ── Capability control ─────────────────────────────────────────────────────

# Restrict to network only — any stage requiring fs-write or gpu will be blocked
noether run graph.json --allow-capabilities network

# Allow multiple capabilities
noether run etl-pipeline.json --allow-capabilities network,fs-read,fs-write

# ── Retrieve the trace afterwards ─────────────────────────────────────────

RESULT=$(noether run graph.json --input '{"x": 42}')
COMPOSITION_ID=$(echo "$RESULT" | jq -r '.data.composition_id')
noether trace "$COMPOSITION_ID"

# ── Minimal graph format (Lagrange JSON) ──────────────────────────────────
#
# Single stage:
# { "description": "count words", "root": { "op": "stage", "id": "<hash>" } }
#
# Sequential pipeline:
# { "description": "csv → emails",
#   "root": { "op": "sequential", "stages": [
#     { "op": "stage", "id": "<csv_parse_hash>" },
#     { "op": "stage", "id": "<extract_emails_hash>" }
#   ]}
# }
#
# Use `noether compose --dry-run "<problem>"` to generate a graph
# without executing it, then save the graph field for reuse.
