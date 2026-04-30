# Awl A/B savings experiment

Measure whether dispatching bounded coding tasks to a local 7B model via Awl
saves frontier tokens versus solving the same tasks on a frontier model.

## Layout

```
experiments/
├── tasks/                    # one dir per task
│   └── <id>/
│       ├── task.json         # dispatch spec + metadata
│       └── setup.sh          # creates a clean sandbox + tests
├── sandbox/                  # generated; recreated each run by setup.sh
├── results/                  # JSONL + CSV inputs/outputs (created on first run)
├── run_awl_arm.sh            # drives the local-Awl arm
├── tally.py                  # compares Awl vs frontier-baseline
└── README.md
```

## Pre-reqs

- `ollama serve` running locally
- L2 model pulled (default: `qwen2.5-coder:7b`); confirm with `awl doctor`
- `python3` on `PATH` (no extra packages required)

## Running the local arm

From the repo root:

```bash
./experiments/run_awl_arm.sh
```

By default this uses `cargo run --quiet -- dispatch …`. To run the installed
binary instead:

```bash
AWL_BIN=/usr/local/bin/awl ./experiments/run_awl_arm.sh
```

Optional: `AWL_LEVEL=3` to run the verifier-tier model (3B) instead of L2.

Output: one JSON line per task at `experiments/results/awl_arm.jsonl` plus
the raw dispatch result at `experiments/results/<id>.json`. Each record
captures status, attempts, token counts, dispatch_id, and wall time.

## Running the frontier-baseline arm

This step is manual — it depends on which frontier you're benchmarking
against and how billing exposes per-call token counts.

For each task in `experiments/tasks/<id>/`:

1. Read `task.json` → `dispatch.task` (the natural-language description) and
   `dispatch.target_path` / `verify_command`.
2. Run `bash experiments/tasks/<id>/setup.sh` to materialize the sandbox.
3. Hand the task to your frontier model (e.g. Claude Code without Awl tools
   enabled, or Codex). Let it edit the target file directly.
4. Run the task's `verify_command` and record pass/fail.
5. Note frontier input + output tokens for that task and the wall time.

Record one row per task in `experiments/results/baseline.csv`:

```csv
task_id,frontier_tokens,frontier_pass,wall_ms
01_string_helper,2400,true,18000
02_validate_input,3100,true,22000
03_fix_off_by_one,4800,true,31000
```

`frontier_tokens` should be **input + output** for the call(s) that actually
solved the task (exclude unrelated context the frontier session carried).

## Aggregating

```bash
./experiments/tally.py
```

For a paid-cost-avoided estimate:

```bash
./experiments/tally.py --cost-per-mtok 5.0
```

The report prints to stdout as markdown — pipe to a file or a paste buffer.

## Pass/fail thresholds

Per `UPDATED_PROGRESS_REPORT.md`:

- **Token reduction:** ≥25–40% (aggregate across tasks)
- **Awl usable-as-is rate:** ≥60–70% (passing tasks ÷ tasks attempted)

If both clear: ship. If only the rate clears, dig into which tasks bled
tokens (verify retries are the usual culprit). If only the reduction clears,
look at which tasks failed and whether Step 2 (preflight) or Step 3
(selection lint) would have caught them.

## Adding a task

```
experiments/tasks/04_my_task/
├── task.json
└── setup.sh
```

Constraints:

- `setup.sh` must be idempotent — it deletes and recreates `experiments/sandbox/<id>/`.
- `task.json.dispatch.target_path` must point inside `experiments/sandbox/<id>/`.
- `verify_command` must exit 0 on success; the harness uses the exit code.
- Keep tasks bounded — anything Awl's L2 (7B) reasonably can't do isn't
  useful as a savings benchmark; it'll just fail and the baseline will too.
