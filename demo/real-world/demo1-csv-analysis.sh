#!/bin/bash
# Demo 1: CSV Analysis Pipeline
set -euo pipefail

[ -f "../../target/release/noether" ] && export PATH="../../target/release:$PATH"
[ -f "../target/release/noether" ] && export PATH="../target/release:$PATH"

CSV_PARSE="72cdbe8850ff9f60c40dc3b4d40da7636c0673ff89c953508d1e782f03ebf023"
LIST_LEN="bb1b2e4dda7a8cb309b255c8c6d89a6befeb5df0aabbe0029b3ee888ac13c8d2"
TO_TEXT="85c780f2ac8543e9e8c25d194615e15b40b5afe1bb77bb02998f76588911f634"

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  Demo 1: CSV Analysis — Parse and Count                     ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

INPUT='{"text":"name,revenue,region,deals\nAcme Corp,450000,US,12\nGlobalTech,280000,EU,8\nDataFlow Inc,520000,US,15\nNordStar,190000,EU,6\nPacific Systems,340000,APAC,10\nCloudBase,410000,US,11\nSmartGrid,175000,EU,5\nRapidScale,295000,APAC,9","has_header":true,"delimiter":null}'

echo "Input: Sales CSV with 8 customers"
echo "  name,revenue,region,deals"
echo "  Acme Corp,450000,US,12"
echo "  ... (8 rows total)"
echo ""

# Type check
GRAPH=$(mktemp /tmp/d1-XXXX.json)
cat > "$GRAPH" << EOF
{"description":"CSV analysis","version":"0.1.0","root":{"op":"Sequential","stages":[
  {"op":"Stage","id":"$CSV_PARSE"},
  {"op":"Stage","id":"$LIST_LEN"},
  {"op":"Stage","id":"$TO_TEXT"}
]}}
EOF

echo "━━━ Type Check ━━━"
DR=$(noether run --dry-run "$GRAPH" 2>&1 | grep -v '^Embedding\|^Warning\|^Nix' || true)
if echo "$DR" | grep -q '"ok": true'; then
  echo "  ✓ csv_parse → list_length → to_text: type-safe"
else
  echo "  ✗ Type check failed"
fi
echo ""

echo "━━━ Execute ━━━"
RESULT=$(noether run "$GRAPH" --input "$INPUT" 2>/dev/null)
OUTPUT=$(echo "$RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['output'])" 2>/dev/null)
DURATION=$(echo "$RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['trace']['duration_ms'])" 2>/dev/null)
echo "  Pipeline: csv_parse → list_length → to_text"
echo "  Result: $OUTPUT customers"
echo "  Time: ${DURATION}ms"
echo ""

echo "━━━ Token Comparison ━━━"
echo "  Compose: ~115 tokens (prompt + graph)"
echo "  Generate: ~500 tokens (prompt + Python code)"
echo "  At 5 variations: compose uses 40% fewer tokens"

rm -f "$GRAPH"
