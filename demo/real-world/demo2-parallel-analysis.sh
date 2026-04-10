#!/bin/bash
# Demo 2: Parallel Text Analysis
set -euo pipefail

[ -f "../../target/release/noether" ] && export PATH="../../target/release:$PATH"
[ -f "../target/release/noether" ] && export PATH="../target/release:$PATH"

TEXT_LEN="3dd4e4c610b68873203a2a973019596753f1cb4435f670561c9d151b4029827b"
TEXT_UPPER="1b68a050bbbfddc6347fdfb5f3be249e5d333f13c5fd5ded4d3afb5ad10ce879"
TEXT_REVERSE="fbd972ad87959f191b0427acdcd1ef29e916a7b3e4c977f1925a8cabb5bac730"
TEXT_TRIM="bd8e439044b0a352491347b44556b22d9cf0aa08a6b4ebb427bda62d2bc5b9fb"
JSON_SER="b96bc6ef0e959aea91a1ece9ef067baaa778cae1de2673ccc71504f5bf8b3705"

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  Demo 2: Parallel Text Analysis — 4 Branches                ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

INPUT='"Noether is an agent-native verified composition platform. Named after Emmy Noether."'
echo "Input: \"Noether is an agent-native verified composition platform...\""
echo ""

GRAPH=$(mktemp /tmp/d2-XXXX.json)
cat > "$GRAPH" << EOF
{"description":"parallel analysis","version":"0.1.0","root":{"op":"Sequential","stages":[
  {"op":"Parallel","branches":{
    "char_count":{"op":"Stage","id":"$TEXT_LEN"},
    "uppercased":{"op":"Stage","id":"$TEXT_UPPER"},
    "reversed":{"op":"Stage","id":"$TEXT_REVERSE"},
    "trimmed":{"op":"Stage","id":"$TEXT_TRIM"}
  }},
  {"op":"Stage","id":"$JSON_SER"}
]}}
EOF

echo "━━━ Type Check ━━━"
DR=$(noether run --dry-run "$GRAPH" 2>&1 | grep -v '^Embedding\|^Warning\|^Nix' || true)
if echo "$DR" | grep -q '"ok": true'; then
  echo "  ✓ 4 parallel branches, all receive Text, results merge into Record"
fi
echo ""

echo "━━━ Execute ━━━"
START=$(date +%s%N)
RESULT=$(noether run "$GRAPH" --input "$INPUT" 2>/dev/null)
END=$(date +%s%N)
WALL_MS=$(( (END - START) / 1000000 ))

echo "$RESULT" | python3 -c "
import sys, json
d = json.load(sys.stdin)['data']
output = json.loads(d['output'])
print('  Results:')
for key in sorted(output):
    val = str(output[key])
    if len(val) > 50: val = val[:47] + '...'
    print(f'    {key:15s} = {val}')
print(f'')
print(f'  Stages: {len(d[\"trace\"][\"stages\"])} executed')
print(f'  Wall clock: $WALL_MS ms')
" 2>/dev/null

echo ""
echo "  Key insight: all 4 analyses ran on the SAME input."
echo "  Sequential chaining would lose the original text after char_count."

rm -f "$GRAPH"
