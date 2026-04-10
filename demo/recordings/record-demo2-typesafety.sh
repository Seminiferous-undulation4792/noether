#!/bin/bash
# Demo 2: Type safety — catches bugs before execution
set -euo pipefail
[ -f "../../target/release/noether" ] && export PATH="../../target/release:$PATH"

type_cmd() { echo ""; echo -n "$ "; for ((i=0; i<${#1}; i++)); do echo -n "${1:$i:1}"; sleep 0.04; done; echo ""; sleep 0.5; }
pause() { sleep "${1:-2}"; }
say() { echo -e "\033[36m$1\033[0m"; sleep 1; }

CSV_PARSE="72cdbe8850ff9f60c40dc3b4d40da7636c0673ff89c953508d1e782f03ebf023"
LIST_LEN="bb1b2e4dda7a8cb309b255c8c6d89a6befeb5df0aabbe0029b3ee888ac13c8d2"
TO_TEXT="85c780f2ac8543e9e8c25d194615e15b40b5afe1bb77bb02998f76588911f634"

clear
say "Every Noether pipeline is type-checked BEFORE it runs."
say "Let's see what happens with a valid vs broken pipeline."
pause 2

say ""
say "Valid pipeline: csv_parse → list_length → to_text"
say "  Record{text,...} → List<Map> → Number → Text   ✓ types match"
pause 1

GOOD=$(mktemp /tmp/good-XXXX.json)
echo "{\"description\":\"valid\",\"version\":\"0.1.0\",\"root\":{\"op\":\"Sequential\",\"stages\":[{\"op\":\"Stage\",\"id\":\"$CSV_PARSE\"},{\"op\":\"Stage\",\"id\":\"$LIST_LEN\"},{\"op\":\"Stage\",\"id\":\"$TO_TEXT\"}]}}" > "$GOOD"

type_cmd "noether run --dry-run valid-pipeline.json"
DR=$(noether run --dry-run "$GOOD" 2>&1 | grep -v '^Embedding\|^Warning\|^Nix' || true)
echo "$DR" | python3 -c "
import sys, json
d = json.load(sys.stdin)
if d['ok']:
    tc = d['data']['type_check']
    print(f'  ✓ Type check passed')
    print(f'    {tc[\"input\"][:45]}')
    print(f'    → {tc[\"output\"]}')
"
pause 3

say ""
say "Now a broken pipeline: list_length → csv_parse"
say "  List<Any> → Number → csv_parse expects Record{text,...}   ✗"
pause 1

BAD=$(mktemp /tmp/bad-XXXX.json)
echo "{\"description\":\"broken\",\"version\":\"0.1.0\",\"root\":{\"op\":\"Sequential\",\"stages\":[{\"op\":\"Stage\",\"id\":\"$LIST_LEN\"},{\"op\":\"Stage\",\"id\":\"$CSV_PARSE\"}]}}" > "$BAD"

type_cmd "noether run --dry-run broken-pipeline.json"
ERR=$(noether run --dry-run "$BAD" 2>&1 | grep -v '^Embedding\|^Warning\|^Nix' || true)
echo "$ERR" | python3 -c "
import sys, json
d = json.load(sys.stdin)
if not d['ok']:
    msg = d['error']['message']
    print(f'  ✗ Type error caught:')
    for line in msg.split('\n')[:3]:
        print(f'    {line.strip()[:70]}')
"
pause 3

say ""
say "The broken pipeline never executed. No wasted compute."
say "In code generation, this bug only shows up at runtime."
pause 3

rm -f "$GOOD" "$BAD"
