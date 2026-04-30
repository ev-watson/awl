#!/usr/bin/env bash
# run_awl_arm.sh — drive the local-Awl arm of the A/B savings experiment.
#
# For each task under experiments/tasks/<id>/:
#   1. Run setup.sh to materialize a clean sandbox.
#   2. Pipe task.json's `dispatch` block into `awl dispatch --apply --verify`.
#   3. Capture status, attempts, tokens, time, dispatch_id.
#   4. Append one JSONL record per task to experiments/results/awl_arm.jsonl.
#
# Pre-reqs: Ollama running locally with the configured L2 model pulled.
# Override the binary with AWL_BIN=/path/to/awl (default: cargo run --quiet --).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

AWL_BIN="${AWL_BIN:-cargo run --quiet --}"
LEVEL="${AWL_LEVEL:-2}"
RESULTS_DIR="experiments/results"
RESULTS_FILE="$RESULTS_DIR/awl_arm.jsonl"
mkdir -p "$RESULTS_DIR"
: >"$RESULTS_FILE"

now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }

for task_dir in experiments/tasks/*/; do
  id="$(basename "$task_dir")"
  task_json="$task_dir/task.json"
  setup="$task_dir/setup.sh"

  if [[ ! -f "$task_json" || ! -x "$setup" ]]; then
    echo "skipping $id: missing task.json or executable setup.sh" >&2
    continue
  fi

  echo "== $id =="
  bash "$setup"

  dispatch_input="$(python3 -c '
import json, sys
spec = json.load(open(sys.argv[1]))
print(json.dumps(spec["dispatch"]))
' "$task_json")"

  out_file="$RESULTS_DIR/$id.json"
  start="$(now_ms)"
  set +e
  printf '%s' "$dispatch_input" | $AWL_BIN dispatch --level "$LEVEL" --apply --auto-repomap >"$out_file"
  rc=$?
  set -e
  end="$(now_ms)"
  elapsed=$((end - start))

  python3 - "$id" "$rc" "$elapsed" "$out_file" "$RESULTS_FILE" <<'PY'
import json, sys

task_id, rc, elapsed_ms, out_path, results_path = sys.argv[1:]
rc = int(rc); elapsed_ms = int(elapsed_ms)

with open(out_path, encoding="utf-8") as fh:
    data = json.load(fh)

usage = data.get("usage") or []
if isinstance(usage, dict):
    usage = [usage]

prompt = sum(int(u.get("prompt_tokens") or u.get("input_tokens") or 0) for u in usage)
completion = sum(int(u.get("completion_tokens") or u.get("output_tokens") or 0) for u in usage)
total = sum(int(u.get("total_tokens") or 0) for u in usage) or (prompt + completion)

record = {
    "task_id": task_id,
    "exit_code": rc,
    "status": data.get("status"),
    "checks_passed": data.get("checks_passed"),
    "attempts": data.get("attempts"),
    "files_changed": data.get("files_changed", []),
    "open_issues": data.get("open_issues", []),
    "wall_ms": elapsed_ms,
    "model_ms": (data.get("telemetry") or {}).get("elapsed_ms"),
    "prompt_tokens": prompt,
    "completion_tokens": completion,
    "total_tokens": total,
    "dispatch_id": (data.get("telemetry") or {}).get("dispatch_id"),
    "model": (data.get("telemetry") or {}).get("model"),
}

with open(results_path, "a", encoding="utf-8") as fh:
    fh.write(json.dumps(record, sort_keys=True) + "\n")

print(f"  status={record['status']} checks_passed={record['checks_passed']} "
      f"attempts={record['attempts']} tokens={record['total_tokens']} wall={elapsed_ms}ms")
PY
done

echo
echo "wrote $RESULTS_FILE"
