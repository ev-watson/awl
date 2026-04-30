#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUT_DIR="${OUT_DIR:-target/awl-eval}"
mkdir -p "$OUT_DIR"

run_case() {
  local name="$1"
  shift
  local out="$OUT_DIR/${name}.json"
  local start end elapsed
  start="$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)"
  "$@" >"$out"
  end="$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)"
  elapsed=$((end - start))
  python3 - "$name" "$elapsed" "$out" <<'PY'
import json
import sys
name, elapsed, path = sys.argv[1], int(sys.argv[2]), sys.argv[3]
with open(path, encoding="utf-8") as fh:
    data = json.load(fh)
telemetry = data.get("telemetry", {})
usage = data.get("usage", {})
if isinstance(usage, list):
    total_tokens = sum(item.get("total_tokens", 0) for item in usage if isinstance(item, dict))
else:
    total_tokens = usage.get("total_tokens", 0)
print(json.dumps({
    "case": name,
    "status": data.get("status"),
    "checks_passed": data.get("checks_passed"),
    "files_changed": data.get("files_changed"),
    "files_intended": data.get("files_intended"),
    "attempts": data.get("attempts"),
    "elapsed_ms": elapsed,
    "model_elapsed_ms": telemetry.get("elapsed_ms"),
    "total_tokens": total_tokens,
    "log_path": telemetry.get("log_path"),
}, sort_keys=True))
PY
}

run_case non_apply \
  cargo run --quiet -- dispatch --level 3 --max-return-chars 1000 <<'JSON'
{
  "task": "Generate a Python function named eval_double(x) that returns x * 2. Return only source code in code.",
  "constraints": ["No markdown", "Keep explanation to one sentence"]
}
JSON

run_case apply_success \
  cargo run --quiet -- dispatch --level 3 --apply \
    --target-path target/awl-eval/eval_success.py \
    --verify "python3 -m py_compile target/awl-eval/eval_success.py" \
    --max-attempts 1 --max-return-chars 1000 <<'JSON'
{
  "task": "Generate a valid Python module defining eval_success() returning 1.",
  "constraints": ["No markdown", "Only valid Python source code in code"]
}
JSON

rm -f target/awl-eval/eval_rollback.py
run_case apply_rollback \
  cargo run --quiet -- dispatch --level 3 --apply \
    --target-path target/awl-eval/eval_rollback.py \
    --verify "python3 -m py_compile target/awl-eval/missing.py" \
    --max-attempts 1 --max-return-chars 1000 <<'JSON'
{
  "task": "Generate a valid Python module defining eval_rollback() returning 1.",
  "constraints": ["No markdown", "Only valid Python source code in code"]
}
JSON

if [[ -e target/awl-eval/eval_rollback.py ]]; then
  echo '{"case":"rollback_file_absent","status":"failed"}'
else
  echo '{"case":"rollback_file_absent","status":"ok"}'
fi
