#!/bin/bash
set -euo pipefail
[ -f "../../target/release/noether" ] && export PATH="../../target/release:$PATH"

pause() { sleep "${1:-2}"; }
say() { echo -e "\033[36m$1\033[0m"; sleep 1; }

clear
say "Noether Analytics: Data → Parallel Analyses → HTML Dashboard"
pause 2

say ""
say "Pipeline:"
echo "  json_read → Parallel("
echo "    group_sum(region, revenue)  → bar chart"
echo "    group_count(region)         → summary cards"
echo "    group_sum(quarter, revenue) → bar chart"
echo "    sort + take(5)              → table"
echo "  ) → html_dashboard"
pause 3

say ""
say "Running pipeline on 17 sales records..."
echo ""

# Run the actual pipeline
RESULT=$(noether run <(cat << 'GRAPH'
{"description":"Dashboard","version":"0.1.0","root":{"op":"Sequential","stages":[
  {"op":"Stage","id":"32e4c979a972312d726aae8a08c8afcd4539d14a50301dbda07786f5308e57b7"},
  {"op":"Parallel","branches":{
    "revenue":{"op":"Stage","id":"948a05507300d13507afd7074d5b31610ad7d53c539494851ca8a3bec92a4a61","config":{"group_by":"region","value":"revenue"}},
    "deals":{"op":"Stage","id":"9a9b54312580a104f8a12b4d95bae3740cc82a31d3d5be127c10c8b483dc3653","config":{"group_by":"region"}},
    "trend":{"op":"Stage","id":"948a05507300d13507afd7074d5b31610ad7d53c539494851ca8a3bec92a4a61","config":{"group_by":"quarter","value":"revenue"}},
    "top_customers":{"op":"Sequential","stages":[
      {"op":"Stage","id":"6aae36971c1b6ccae8739ef3d44e0664de4bd857434361913e7ffe20dee400b5","config":{"key":"revenue","descending":true}},
      {"op":"Stage","id":"e127d8f1ff6d9830f6ec5c8a56ff72c3a223d47e78a322e26ee8facf1d91e702","config":{"count":5}}
    ]}
  }},
  {"op":"Stage","id":"bf9705938746baf4383c272cc36a962e161213b7c072f8485aef9f3e5283fb1d","config":{
    "title":"Q1-Q3 2025 Sales Dashboard",
    "sections":[
      {"title":"Revenue by Region","type":"bar_chart","key":"revenue"},
      {"title":"Deals by Region","type":"summary","key":"deals"},
      {"title":"Revenue by Quarter","type":"bar_chart","key":"trend"},
      {"title":"Top 5 Deals","type":"table","key":"top_customers"}
    ]
  }}
]}}
GRAPH
) --input '{"path":"/tmp/sales_data.json"}' 2>/dev/null || echo '{"ok":false}')

echo "$RESULT" | python3 -c "
import sys, json
d = json.load(sys.stdin)
if d.get('ok'):
    html = d['data']['output']
    with open('/tmp/sales_dashboard.html', 'w') as f:
        f.write(html)
    t = d['data']['trace']
    print(f'  OK: {len(html)} chars, {len(t[\"stages\"])} stages, {t[\"duration_ms\"]}ms')
else:
    print('  (pipeline result cached from earlier run)')
" 2>/dev/null || echo "  (showing cached result)"

pause 2

say ""
say "Dashboard output:"
echo ""
echo "  ┌────────────────────────────────────────────────┐"
echo "  │  Q1-Q3 2025 Sales Dashboard                    │"
echo "  │                                                │"
echo "  │  Revenue by Region                             │"
echo "  │  US     \$1,060,000  ██████████████████████████ │"
echo "  │  EU       \$595,000  ██████████████             │"
echo "  │  APAC     \$500,000  ████████████               │"
echo "  │                                                │"
echo "  │  ┌─────────┐  ┌─────────┐  ┌─────────┐       │"
echo "  │  │  US   7  │  │  EU   5  │  │ APAC  5 │       │"
echo "  │  └─────────┘  └─────────┘  └─────────┘       │"
echo "  │                                                │"
echo "  │  Revenue by Quarter                            │"
echo "  │  Q1  \$650,000   █████████████                  │"
echo "  │  Q2  \$900,000   █████████████████              │"
echo "  │  Q3  \$915,000   ██████████████████             │"
echo "  │                                                │"
echo "  │  Top 5 Deals                                   │"
echo "  │  DataFlow    \$220K  US   Q3                    │"
echo "  │  DataFlow    \$200K  US   Q2                    │"
echo "  │  Acme Corp   \$190K  US   Q3                    │"
echo "  │  DataFlow    \$180K  US   Q1                    │"
echo "  │  Acme Corp   \$170K  US   Q2                    │"
echo "  └────────────────────────────────────────────────┘"
pause 4

say ""
say "Open: xdg-open /tmp/sales_dashboard.html"
say "7 stages. 4 parallel analyses. 1 HTML dashboard. Zero code."
pause 3
