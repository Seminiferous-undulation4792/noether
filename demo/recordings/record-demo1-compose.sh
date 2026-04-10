#!/bin/bash
set -euo pipefail
[ -f "../../target/release/noether" ] && export PATH="../../target/release:$PATH"

type_cmd() { echo ""; echo -n "$ "; for ((i=0; i<${#1}; i++)); do echo -n "${1:$i:1}"; sleep 0.04; done; echo ""; sleep 0.5; }
pause() { sleep "${1:-2}"; }
say() { echo -e "\033[36m$1\033[0m"; sleep 1; }

FILE_GROUP_ID=$(python3 -c "
import json
with open('$HOME/.noether/store.json') as f:
    store = json.load(f)
for s in store['stages']:
    if 'Read a CSV file with name,revenue,region' in s.get('description',''):
        if s.get('lifecycle') == 'Active': print(s['id']); break
")
JSON_SER="b96bc6ef0e959aea91a1ece9ef067baaa778cae1de2673ccc71504f5bf8b3705"

clear
say "An agent needs to analyze sales data from a CSV file."
say "Instead of writing pandas code, it uses a Noether pipeline."
pause 2

say ""
say "The CSV file:"
echo ""
cat /tmp/sales.csv | head -5
echo "  ... (8 rows)"
pause 2

say ""
say "The pipeline: csv_file_group_revenue → json_serialize"
echo ""
echo '  {
    "stages": [
      { "id": "'${FILE_GROUP_ID:0:12}'...", "_comment": "read file + parse + group + sum" },
      { "id": "b96bc6ef...",   "_comment": "json_serialize: Any → Text" }
    ]
  }'
pause 3

GRAPH=$(mktemp /tmp/d1-XXXX.json)
echo "{\"description\":\"revenue\",\"version\":\"0.1.0\",\"root\":{\"op\":\"Sequential\",\"stages\":[{\"op\":\"Stage\",\"id\":\"$FILE_GROUP_ID\"},{\"op\":\"Stage\",\"id\":\"$JSON_SER\"}]}}" > "$GRAPH"

say ""
say "Execute — just pass the file path:"
type_cmd 'noether run revenue.json --input '"'"'{"path": "/tmp/sales.csv"}'"'"''

RESULT=$(noether run "$GRAPH" --input '{"path":"/tmp/sales.csv"}' 2>/dev/null)
echo "$RESULT" | python3 -c "
import sys, json
d = json.load(sys.stdin)['data']
output = json.loads(d['output'])
print(f'  Result:')
for region in sorted(output, key=lambda r: output[r], reverse=True):
    print(f'    {region:6s}  \${output[region]:>10,}')
"
pause 4

say ""
say "US: \$1.38M. EU: \$645K. APAC: \$635K."
say "Read from disk, parsed, grouped, summed. No code written."
pause 3

rm -f "$GRAPH"
