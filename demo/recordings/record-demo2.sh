#!/bin/bash
# Scripted demo: Parallel text analysis
set -euo pipefail

[ -f "../../target/release/noether" ] && export PATH="../../target/release:$PATH"
[ -f "../../target/release/noether" ] && export PATH="../../target/release:$PATH"

type_cmd() {
  echo ""
  echo -n "$ "
  for ((i=0; i<${#1}; i++)); do echo -n "${1:$i:1}"; sleep 0.04; done
  echo ""; sleep 0.3
}
pause() { sleep "${1:-2}"; }
comment() { echo -e "\033[90m# $1\033[0m"; sleep 1.5; }

clear
echo "╔══════════════════════════════════════════════════╗"
echo "║  Noether Demo 2: Parallel Text Analysis          ║"
echo "╚══════════════════════════════════════════════════╝"
pause 3

comment "Problem: analyze text in 4 ways simultaneously."
comment "Parallel branches each receive the SAME input — no data loss."
pause 2

TEXT_LEN="3dd4e4c610b68873203a2a973019596753f1cb4435f670561c9d151b4029827b"
TEXT_UPPER="1b68a050bbbfddc6347fdfb5f3be249e5d333f13c5fd5ded4d3afb5ad10ce879"
TEXT_REVERSE="fbd972ad87959f191b0427acdcd1ef29e916a7b3e4c977f1925a8cabb5bac730"
TEXT_TRIM="bd8e439044b0a352491347b44556b22d9cf0aa08a6b4ebb427bda62d2bc5b9fb"
JSON_SER="b96bc6ef0e959aea91a1ece9ef067baaa778cae1de2673ccc71504f5bf8b3705"

GRAPH=$(mktemp /tmp/d2-XXXX.json)
cat > "$GRAPH" << EOF
{"description":"parallel","version":"0.1.0","root":{"op":"Sequential","stages":[
  {"op":"Parallel","branches":{
    "char_count":{"op":"Stage","id":"$TEXT_LEN"},
    "uppercased":{"op":"Stage","id":"$TEXT_UPPER"},
    "reversed":{"op":"Stage","id":"$TEXT_REVERSE"},
    "trimmed":{"op":"Stage","id":"$TEXT_TRIM"}
  }},
  {"op":"Stage","id":"$JSON_SER"}
]}}
EOF

INPUT='"Noether is a type-safe composition platform for AI agents."'

comment "4 parallel branches: char_count, uppercase, reverse, trim"
type_cmd "noether run parallel.json --input '\"Noether is a type-safe composition platform...\"'"

START=$(date +%s%N)
RESULT=$(noether run "$GRAPH" --input "$INPUT" 2>/dev/null)
END=$(date +%s%N)
MS=$(( (END - START) / 1000000 ))

echo "$RESULT" | python3 -c "
import sys, json
d = json.load(sys.stdin)['data']
output = json.loads(d['output'])
print(f'  Results (all from the SAME input text):')
print(f'')
for key in sorted(output):
    val = str(output[key])
    if len(val) > 45: val = val[:42] + '...'
    print(f'    {key:15s} = {val}')
print(f'')
print(f'  Execution: {d[\"trace\"][\"duration_ms\"]}ms, {len(d[\"trace\"][\"stages\"])} stages')
"
pause 4

comment "Key insight: in a sequential pipeline, char_count returns a Number,"
comment "destroying the text. Parallel preserves the original for all branches."
pause 3

rm -f "$GRAPH"
echo ""
echo "Done."
