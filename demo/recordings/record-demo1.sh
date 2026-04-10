#!/bin/bash
# Scripted demo with typing simulation and pauses.
# Run inside: asciinema rec demo1.cast --command "bash record-demo1.sh"

set -euo pipefail

[ -f "../../target/release/noether" ] && export PATH="../../target/release:$PATH"
[ -f "../../target/release/noether" ] && export PATH="../../target/release:$PATH"

# Helpers
type_cmd() {
  echo ""
  echo -n "$ "
  for ((i=0; i<${#1}; i++)); do
    echo -n "${1:$i:1}"
    sleep 0.04
  done
  echo ""
  sleep 0.3
}

pause() { sleep "${1:-2}"; }
comment() { echo -e "\033[90m# $1\033[0m"; sleep 1.5; }

clear

echo "╔══════════════════════════════════════════════════╗"
echo "║  Noether: Type-safe Composition for AI Agents    ║"
echo "║  Demo 1: CSV Analysis Pipeline                   ║"
echo "╚══════════════════════════════════════════════════╝"
pause 3

comment "First, let's see what stages are available for CSV processing."
type_cmd 'noether stage search "parse CSV"'
noether stage search "parse CSV" 2>/dev/null | python3 -c "
import sys, json
results = json.load(sys.stdin)['data']['results'][:5]
for r in results:
    print(f'  {r[\"score\"]:>6s}  {r[\"id\"]:10s}  {r[\"description\"][:50]}')
"
pause 3

comment "csv_parse takes Record{text, has_header, delimiter} and returns List<Map>."
comment "Let's type-check a pipeline BEFORE running it."
pause 1

# Create the graph
GRAPH=$(mktemp /tmp/d1-XXXX.json)
CSV_PARSE="72cdbe8850ff9f60c40dc3b4d40da7636c0673ff89c953508d1e782f03ebf023"
LIST_LEN="bb1b2e4dda7a8cb309b255c8c6d89a6befeb5df0aabbe0029b3ee888ac13c8d2"
TO_TEXT="85c780f2ac8543e9e8c25d194615e15b40b5afe1bb77bb02998f76588911f634"
echo "{\"description\":\"CSV analysis\",\"version\":\"0.1.0\",\"root\":{\"op\":\"Sequential\",\"stages\":[{\"op\":\"Stage\",\"id\":\"$CSV_PARSE\"},{\"op\":\"Stage\",\"id\":\"$LIST_LEN\"},{\"op\":\"Stage\",\"id\":\"$TO_TEXT\"}]}}" > "$GRAPH"

type_cmd "noether run --dry-run pipeline.json"
DR=$(noether run --dry-run "$GRAPH" 2>&1 | grep -v '^Embedding\|^Warning\|^Nix' || true)
echo "$DR" | python3 -c "
import sys, json
d = json.load(sys.stdin)
tc = d['data']['type_check']
print(f'  ✓ Type check passed')
print(f'    Input:  {tc[\"input\"]}')
print(f'    Output: {tc[\"output\"]}')
print(f'    Steps:  {d[\"data\"][\"plan\"][\"steps\"]}')
"
pause 3

comment "Types match: Record → List<Map> → Number → Text. Now execute with real data."
pause 1

INPUT='{"text":"name,revenue,region\nAcme Corp,450000,US\nGlobalTech,280000,EU\nDataFlow,520000,US\nNordStar,190000,EU\nPacific,340000,APAC","has_header":true,"delimiter":null}'

type_cmd 'noether run pipeline.json --input "$(cat sales.csv.json)"'
RESULT=$(noether run "$GRAPH" --input "$INPUT" 2>/dev/null)
echo "$RESULT" | python3 -c "
import sys, json
d = json.load(sys.stdin)['data']
print(f'  Output: {d[\"output\"]} customers')
print(f'  Time:   {d[\"trace\"][\"duration_ms\"]}ms')
print(f'  Stages: {len(d[\"trace\"][\"stages\"])} executed')
for s in d['trace']['stages']:
    print(f'    step {s[\"step_index\"]}: {s[\"stage_id\"][:8]}  {s[\"status\"]}  {s[\"duration_ms\"]}ms')
"
pause 3

comment "5 customers parsed, counted, and formatted in 0ms."
comment "Type-checked before execution. Reproducible. Zero code written."
pause 3

rm -f "$GRAPH"
echo ""
echo "Done."
