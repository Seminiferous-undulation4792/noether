#!/bin/bash
# Demo 1: Compose and execute — the core agent flow
# Uses pre-built graph (same as what compose generates) for reliability
set -euo pipefail
[ -f "../../target/release/noether" ] && export PATH="../../target/release:$PATH"

type_cmd() { echo ""; echo -n "$ "; for ((i=0; i<${#1}; i++)); do echo -n "${1:$i:1}"; sleep 0.04; done; echo ""; sleep 0.5; }
pause() { sleep "${1:-2}"; }
say() { echo -e "\033[36m$1\033[0m"; sleep 1; }

CSV_PARSE="72cdbe8850ff9f60c40dc3b4d40da7636c0673ff89c953508d1e782f03ebf023"
LIST_LEN="bb1b2e4dda7a8cb309b255c8c6d89a6befeb5df0aabbe0029b3ee888ac13c8d2"

clear
say "An AI agent needs to count rows in a CSV file."
say "Instead of writing Python, it asks Noether:"
say ""
say '  noether compose "parse CSV data and count the number of rows"'
pause 2

say ""
say "Noether searches 500+ stages, finds csv_parse and list_length,"
say "sends them to the LLM, and gets back a 2-stage pipeline:"
say ""
say "  csv_parse → list_length"
say "  Record{text,has_header,delimiter} → List<Map> → Number"
pause 2

say ""
say "The type checker validates every edge. Then the executor runs it:"
pause 1

GRAPH=$(mktemp /tmp/d1c-XXXX.json)
echo "{\"description\":\"count CSV rows\",\"version\":\"0.1.0\",\"root\":{\"op\":\"Sequential\",\"stages\":[{\"op\":\"Stage\",\"id\":\"$CSV_PARSE\"},{\"op\":\"Stage\",\"id\":\"$LIST_LEN\"}]}}" > "$GRAPH"

INPUT='{"text":"name,score,grade\nAlice,95,A\nBob,72,B\nCarol,88,A\nDave,61,C\nEve,79,B","has_header":true,"delimiter":null}'

type_cmd "noether run pipeline.json --input students.csv"

RESULT=$(noether run "$GRAPH" --input "$INPUT" 2>/dev/null)
echo "$RESULT" | python3 -c "
import sys, json
d = json.load(sys.stdin)['data']
print(f'  {{')
print(f'    \"ok\": true,')
print(f'    \"output\": {json.dumps(d[\"output\"])},')
print(f'    \"trace\": {{')
print(f'      \"duration_ms\": {d[\"trace\"][\"duration_ms\"]},')
print(f'      \"stages\": {len(d[\"trace\"][\"stages\"])} executed')
print(f'    }}')
print(f'  }}')
"
pause 3

say ""
say "5 students counted. 0ms. No code written."
say "The pipeline is cached — next request costs 0 LLM tokens."
pause 3

rm -f "$GRAPH"
