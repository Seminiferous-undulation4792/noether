#!/bin/bash
set -euo pipefail
[ -f "../../target/release/noether" ] && export PATH="../../target/release:$PATH"

type_cmd() { echo ""; echo -n "$ "; for ((i=0; i<${#1}; i++)); do echo -n "${1:$i:1}"; sleep 0.04; done; echo ""; sleep 0.5; }
pause() { sleep "${1:-2}"; }
say() { echo -e "\033[36m$1\033[0m"; sleep 1; }

# Get stage IDs
JSON_READ=$(python3 -c "
import json
with open('$HOME/.noether/store.json') as f:
    s = json.load(f)
for x in s['stages']:
    if 'Read and parse a JSON file' in x.get('description','') and x.get('lifecycle')=='Active':
        print(x['id']); break
")
TRAIN_ID=$(python3 -c "
import json
with open('$HOME/.noether/store.json') as f:
    s = json.load(f)
for x in s['stages']:
    if 'Train a scikit-learn model' in x.get('description','') and x.get('lifecycle')=='Active':
        print(x['id']); break
")
PREDICT_ID=$(python3 -c "
import json
with open('$HOME/.noether/store.json') as f:
    s = json.load(f)
for x in s['stages']:
    if 'Load a trained sklearn model and add' in x.get('description','') and x.get('lifecycle')=='Active':
        print(x['id']); break
")
EVAL_ID=$(python3 -c "
import json
with open('$HOME/.noether/store.json') as f:
    s = json.load(f)
for x in s['stages']:
    if 'Evaluate model predictions' in x.get('description','') and x.get('lifecycle')=='Active':
        print(x['id']); break
")
IMPORTANCE_ID=$(python3 -c "
import json
with open('$HOME/.noether/store.json') as f:
    s = json.load(f)
for x in s['stages']:
    if 'Extract feature importance' in x.get('description','') and x.get('lifecycle')=='Active':
        print(x['id']); break
")

clear
say "Noether ML Pipeline: Train → Evaluate → Serve API"
say "End-to-end: from raw data to production REST endpoint"
pause 2

# ── Step 1: Train ──────────────────────────────────────────

say ""
say "Step 1: Train a RandomForest on iris data"
echo ""
echo "  Dataset: /tmp/iris_train.json (15 flowers, 3 species)"
echo "  Pipeline: json_read → sklearn_train(config: {model: RF})"
pause 1

TRAIN_GRAPH=$(mktemp /tmp/ml-train-XXXX.json)
cat > "$TRAIN_GRAPH" << EOF
{"description":"Train RF","version":"0.1.0","root":{"op":"Sequential","stages":[
  {"op":"Stage","id":"$JSON_READ"},
  {"op":"Stage","id":"$TRAIN_ID","config":{"target":"species","model":"RandomForestClassifier","params":{"n_estimators":10,"random_state":42},"save_path":"/tmp/iris_rf.pkl"}}
]}}
EOF

type_cmd 'noether run train.json --input '"'"'{"path": "/tmp/iris_train.json"}'"'"''
RESULT=$(noether run "$TRAIN_GRAPH" --input '{"path":"/tmp/iris_train.json"}' 2>/dev/null)
echo "$RESULT" | python3 -c "
import sys, json
d = json.load(sys.stdin)['data']['output']
print(f'  ✓ {d[\"model_type\"]} trained on {d[\"train_samples\"]} samples')
print(f'    Features: {d[\"feature_names\"]}')
print(f'    Model saved: {d[\"model_path\"]}')
"
rm -f "$TRAIN_GRAPH"
pause 3

# ── Step 2: Evaluate ───────────────────────────────────────

say ""
say "Step 2: Predict + evaluate on same data"
echo ""
echo "  Pipeline: json_read → sklearn_predict → sklearn_evaluate"
pause 1

EVAL_GRAPH=$(mktemp /tmp/ml-eval-XXXX.json)
cat > "$EVAL_GRAPH" << EOF
{"description":"Evaluate","version":"0.1.0","root":{"op":"Sequential","stages":[
  {"op":"Stage","id":"$JSON_READ"},
  {"op":"Stage","id":"$PREDICT_ID","config":{"model_path":"/tmp/iris_rf.pkl"}},
  {"op":"Stage","id":"$EVAL_ID","config":{"target":"species","predicted":"prediction"}}
]}}
EOF

type_cmd 'noether run evaluate.json --input '"'"'{"path": "/tmp/iris_train.json"}'"'"''
RESULT=$(noether run "$EVAL_GRAPH" --input '{"path":"/tmp/iris_train.json"}' 2>/dev/null)
echo "$RESULT" | python3 -c "
import sys, json
d = json.load(sys.stdin)['data']['output']
print(f'  ✓ Accuracy:  {d[\"accuracy\"]}')
print(f'    F1:        {d[\"f1\"]}')
print(f'    Samples:   {d[\"samples\"]}')
"
rm -f "$EVAL_GRAPH"
pause 3

# ── Step 3: Serve as API ───────────────────────────────────

say ""
say "Step 3: Serve as a multi-endpoint REST API"
echo ""
echo '  api.json:'
echo '  {
    "routes": {
      "/predict":    "predict.json",
      "/importance": "importance.json"
    }
  }'
pause 2

PREDICT_GRAPH=$(mktemp /tmp/ml-pred-XXXX.json)
echo "{\"description\":\"Predict species\",\"version\":\"0.1.0\",\"root\":{\"op\":\"Stage\",\"id\":\"$PREDICT_ID\",\"config\":{\"model_path\":\"/tmp/iris_rf.pkl\"}}}" > "$PREDICT_GRAPH"

IMP_GRAPH=$(mktemp /tmp/ml-imp-XXXX.json)
echo "{\"description\":\"Feature importance\",\"version\":\"0.1.0\",\"root\":{\"op\":\"Stage\",\"id\":\"$IMPORTANCE_ID\"}}" > "$IMP_GRAPH"

API_CONFIG=$(mktemp /tmp/ml-api-XXXX.json)
cat > "$API_CONFIG" << EOF
{"routes": {"/predict": "$PREDICT_GRAPH", "/importance": "$IMP_GRAPH"}}
EOF

type_cmd 'noether serve api.json --port :8092'
noether serve "$API_CONFIG" --port :8092 &
SERVER_PID=$!
sleep 2

say ""
say "Calling the API:"
echo ""

echo '  $ curl -X POST http://localhost:8092/predict \'
echo '      -d '"'"'[{"sepal_l":5.1,"sepal_w":3.5,"petal_l":1.4,"petal_w":0.2}]'"'"''
echo ""
PRED=$(curl -s -X POST http://localhost:8092/predict \
  -d '[{"sepal_l":5.1,"sepal_w":3.5,"petal_l":1.4,"petal_w":0.2},{"sepal_l":7.0,"sepal_w":3.2,"petal_l":4.7,"petal_w":1.4},{"sepal_l":6.3,"sepal_w":3.3,"petal_l":6.0,"petal_w":2.5}]')
echo "$PRED" | python3 -c "
import sys, json
d = json.load(sys.stdin)
species = {0: 'setosa', 1: 'versicolor', 2: 'virginica'}
for r in d['output']:
    sp = species.get(r['prediction'], str(r['prediction']))
    print(f'    → {sp:12s}  (sepal={r[\"sepal_l\"]}, petal={r[\"petal_l\"]})')
print(f'    {d[\"duration_ms\"]}ms')
"
pause 2

echo ""
echo '  $ curl -X POST http://localhost:8092/importance \'
echo '      -d '"'"'{"model_path":"/tmp/iris_rf.pkl"}'"'"''
echo ""
IMP=$(curl -s -X POST http://localhost:8092/importance \
  -d '{"model_path":"/tmp/iris_rf.pkl"}')
echo "$IMP" | python3 -c "
import sys, json
d = json.load(sys.stdin)
for f in d['output']:
    bar = '█' * int(f['importance'] * 30)
    print(f'    {f[\"feature\"]:10s} {f[\"importance\"]:.3f}  {bar}')
"

kill $SERVER_PID 2>/dev/null
rm -f "$PREDICT_GRAPH" "$IMP_GRAPH" "$API_CONFIG"
pause 3

say ""
say "End-to-end: train → evaluate → serve. No Flask. No Docker."
say "Just composition graphs + noether serve."
pause 3
