#!/bin/bash
# Demo 4: Stage reuse — 3 pipelines, shared stages, zero code
set -euo pipefail
[ -f "../../target/release/noether" ] && export PATH="../../target/release:$PATH"

type_cmd() { echo ""; echo -n "$ "; for ((i=0; i<${#1}; i++)); do echo -n "${1:$i:1}"; sleep 0.04; done; echo ""; sleep 0.5; }
pause() { sleep "${1:-2}"; }
say() { echo -e "\033[36m$1\033[0m"; sleep 1; }

CSV_PARSE="72cdbe8850ff9f60c40dc3b4d40da7636c0673ff89c953508d1e782f03ebf023"
LIST_LEN="bb1b2e4dda7a8cb309b255c8c6d89a6befeb5df0aabbe0029b3ee888ac13c8d2"
TO_TEXT="85c780f2ac8543e9e8c25d194615e15b40b5afe1bb77bb02998f76588911f634"
JSON_SER="b96bc6ef0e959aea91a1ece9ef067baaa778cae1de2673ccc71504f5bf8b3705"
LIST_DEDUP="ae554428b5974ca245a06adc2b17d25e52eaaa460adb011c98db785f111e3763"

INPUT='{"text":"status,priority,assignee\nopen,high,alice\nopen,medium,bob\nclosed,high,alice\nopen,high,carol\nclosed,low,bob\nopen,high,alice\nopen,medium,carol","has_header":true,"delimiter":null}'

clear
say "Same CSV data. Three different questions. Same building blocks."
pause 2

say ""
say "Pipeline A: How many tickets?"
GA=$(mktemp /tmp/a-XXXX.json)
echo "{\"description\":\"A\",\"version\":\"0.1.0\",\"root\":{\"op\":\"Sequential\",\"stages\":[{\"op\":\"Stage\",\"id\":\"$CSV_PARSE\"},{\"op\":\"Stage\",\"id\":\"$LIST_LEN\"},{\"op\":\"Stage\",\"id\":\"$TO_TEXT\"}]}}" > "$GA"
type_cmd "noether run count-tickets.json --input bugs.csv"
OA=$(noether run "$GA" --input "$INPUT" 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['output'])")
echo "  → $OA tickets"
rm -f "$GA"
pause 2

say ""
say "Pipeline B: Show me the data as JSON (reuses csv_parse)"
GB=$(mktemp /tmp/b-XXXX.json)
echo "{\"description\":\"B\",\"version\":\"0.1.0\",\"root\":{\"op\":\"Sequential\",\"stages\":[{\"op\":\"Stage\",\"id\":\"$CSV_PARSE\"},{\"op\":\"Stage\",\"id\":\"$JSON_SER\"}]}}" > "$GB"
type_cmd "noether run to-json.json --input bugs.csv"
OB=$(noether run "$GB" --input "$INPUT" 2>/dev/null | python3 -c "import sys,json; r=json.load(sys.stdin)['data']['output']; print(r[:60]+'...')")
echo "  → $OB"
rm -f "$GB"
pause 2

say ""
say "Pipeline C: How many unique tickets? (reuses csv_parse + list_length)"
GC=$(mktemp /tmp/c-XXXX.json)
echo "{\"description\":\"C\",\"version\":\"0.1.0\",\"root\":{\"op\":\"Sequential\",\"stages\":[{\"op\":\"Stage\",\"id\":\"$CSV_PARSE\"},{\"op\":\"Stage\",\"id\":\"$LIST_DEDUP\"},{\"op\":\"Stage\",\"id\":\"$LIST_LEN\"},{\"op\":\"Stage\",\"id\":\"$TO_TEXT\"}]}}" > "$GC"
type_cmd "noether run unique-tickets.json --input bugs.csv"
OC=$(noether run "$GC" --input "$INPUT" 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['output'])")
echo "  → $OC unique (found $(python3 -c "print(int(float('$OA'))-int(float('$OC')))") duplicate)"
rm -f "$GC"
pause 3

say ""
echo "  Stage reuse:"
echo "  ┌────────────────┬─────┬─────┬─────┐"
echo "  │ Stage          │  A  │  B  │  C  │"
echo "  ├────────────────┼─────┼─────┼─────┤"
echo "  │ csv_parse      │  ✓  │  ✓  │  ✓  │"
echo "  │ list_length    │  ✓  │     │  ✓  │"
echo "  │ to_text        │  ✓  │     │  ✓  │"
echo "  │ json_serialize │     │  ✓  │     │"
echo "  │ list_dedup     │     │     │  ✓  │"
echo "  └────────────────┴─────┴─────┴─────┘"
pause 3
say ""
say "5 stages. 9 usages. 3 pipelines. Zero lines of code."
pause 3
