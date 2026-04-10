#!/bin/bash
# Demo 3: Stage Reuse — 3 Pipelines, Shared Stages
set -euo pipefail

[ -f "../../target/release/noether" ] && export PATH="../../target/release:$PATH"
[ -f "../target/release/noether" ] && export PATH="../target/release:$PATH"

CSV_PARSE="72cdbe8850ff9f60c40dc3b4d40da7636c0673ff89c953508d1e782f03ebf023"
LIST_LEN="bb1b2e4dda7a8cb309b255c8c6d89a6befeb5df0aabbe0029b3ee888ac13c8d2"
TO_TEXT="85c780f2ac8543e9e8c25d194615e15b40b5afe1bb77bb02998f76588911f634"
JSON_SER="b96bc6ef0e959aea91a1ece9ef067baaa778cae1de2673ccc71504f5bf8b3705"
LIST_DEDUP="ae554428b5974ca245a06adc2b17d25e52eaaa460adb011c98db785f111e3763"

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  Demo 3: Stage Reuse — 3 Pipelines, Shared Stages           ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

INPUT='{"text":"status,priority,assignee\nopen,high,alice\nopen,medium,bob\nclosed,high,alice\nopen,high,carol\nclosed,low,bob\nopen,high,alice\nopen,medium,carol","has_header":true,"delimiter":null}'

echo "Input: Bug tracker CSV (7 tickets, some duplicates)"
echo "  status,priority,assignee"
echo "  open,high,alice"
echo "  ... (7 rows)"
echo ""

# Pipeline A
echo "━━━ Pipeline A: CSV → count rows ━━━"
GA=$(mktemp /tmp/d3a-XXXX.json)
echo "{\"description\":\"A\",\"version\":\"0.1.0\",\"root\":{\"op\":\"Sequential\",\"stages\":[{\"op\":\"Stage\",\"id\":\"$CSV_PARSE\"},{\"op\":\"Stage\",\"id\":\"$LIST_LEN\"},{\"op\":\"Stage\",\"id\":\"$TO_TEXT\"}]}}" > "$GA"
RA=$(noether run "$GA" --input "$INPUT" 2>/dev/null)
OA=$(echo "$RA" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['output'])" 2>/dev/null)
echo "  Stages: csv_parse → list_length → to_text"
echo "  Result: $OA rows"
rm -f "$GA"
echo ""

# Pipeline B
echo "━━━ Pipeline B: CSV → JSON ━━━"
GB=$(mktemp /tmp/d3b-XXXX.json)
echo "{\"description\":\"B\",\"version\":\"0.1.0\",\"root\":{\"op\":\"Sequential\",\"stages\":[{\"op\":\"Stage\",\"id\":\"$CSV_PARSE\"},{\"op\":\"Stage\",\"id\":\"$JSON_SER\"}]}}" > "$GB"
RB=$(noether run "$GB" --input "$INPUT" 2>/dev/null)
OB=$(echo "$RB" | python3 -c "import sys,json; r=json.load(sys.stdin)['data']['output']; print(r[:70]+'...' if len(r)>70 else r)" 2>/dev/null)
echo "  Stages: csv_parse → json_serialize"
echo "  Result: $OB"
rm -f "$GB"
echo ""

# Pipeline C
echo "━━━ Pipeline C: CSV → deduplicate → count unique ━━━"
GC=$(mktemp /tmp/d3c-XXXX.json)
echo "{\"description\":\"C\",\"version\":\"0.1.0\",\"root\":{\"op\":\"Sequential\",\"stages\":[{\"op\":\"Stage\",\"id\":\"$CSV_PARSE\"},{\"op\":\"Stage\",\"id\":\"$LIST_DEDUP\"},{\"op\":\"Stage\",\"id\":\"$LIST_LEN\"},{\"op\":\"Stage\",\"id\":\"$TO_TEXT\"}]}}" > "$GC"
RC=$(noether run "$GC" --input "$INPUT" 2>/dev/null)
OC=$(echo "$RC" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['output'])" 2>/dev/null)
echo "  Stages: csv_parse → list_dedup → list_length → to_text"
echo "  Result: $OC unique rows (was $OA total)"
rm -f "$GC"
echo ""

echo "━━━ Reuse Analysis ━━━"
echo ""
echo "  ┌──────────────────┬─────────┬─────────┬─────────┐"
echo "  │ Stage            │ A       │ B       │ C       │"
echo "  ├──────────────────┼─────────┼─────────┼─────────┤"
echo "  │ csv_parse        │   ✓     │   ✓     │   ✓     │"
echo "  │ list_length      │   ✓     │         │   ✓     │"
echo "  │ to_text          │   ✓     │         │   ✓     │"
echo "  │ json_serialize   │         │   ✓     │         │"
echo "  │ list_dedup       │         │         │   ✓     │"
echo "  └──────────────────┴─────────┴─────────┴─────────┘"
echo ""
echo "  5 unique stages, 9 total usages, 0 lines of code written."
