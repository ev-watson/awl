# Computational Approaches: Awl Dispatch Reliability and Step 1 Sweep

**Surveyed:** 2026-05-01
**Domain:** Local coding-agent delegation, Ollama model dispatch, benchmark harnesses
**Confidence:** MEDIUM. The implementation paths and current 7B results are file-backed. Runtime performance for 14B is not yet measured in this repository, so 14B wall-time estimates remain provisional.

## Anchor Cross-Check

`GPD/state.json` is present and contains the active scoping contract for the Awl frontier-token savings study. The older `GPD/research-map/ARCHITECTURE.md` and `GPD/research-map/STRUCTURE.md` statements that `GPD/state.json` was missing are stale and should not override the current project anchors in `GPD/PROJECT.md` and `GPD/state.json`.

## Recommended Stack

The computational work is a reliability patch set over Awl's bounded local dispatch pipeline plus a controlled two-configuration Step 1 sweep. The core implementation should stay in the existing Rust CLI/MCP code paths: `src/dispatch.rs` owns model selection, apply/verify, rollback, JSON validation, telemetry, and dispatch logs; `src/main.rs` owns CLI flags; `src/mcp_server.rs` owns the MCP `awl_dispatch` schema and argument plumbing; `src/defaults.rs` remains the fallback source for level-to-model defaults.

The experiment stack should remain lightweight: `experiments/run_awl_arm.sh` runs the local arm, `experiments/tasks/*/task.json` defines dispatch inputs, `experiments/tasks/*/setup.sh` creates deterministic Python `unittest` sandboxes, `experiments/tally.py` compares Awl results to manual frontier baselines, and `scripts/dispatch_cost_report.py` summarizes dispatch JSONL logs. No package installation is required for the surveyed scripts; Python usage is stdlib-only.

## Algorithms and Code Paths

| Algorithm / Path | Problem | Current Implementation | Required Patch |
| --- | --- | --- | --- |
| Level-to-model selection | Choose the Ollama worker for a dispatch | `src/dispatch.rs` calls `defaults::configured_model_for_level(options.level)`; defaults are in `src/defaults.rs` | Add `model: Option<String>` to `DispatchOptions`; use `options.model.unwrap_or_else(configured_model_for_level)`; surface through CLI, MCP, and `AWL_MODEL_OVERRIDE` |
| Apply/verify loop | Generate a complete target file, write it, run verifier, rollback on failure | `run_apply_flow` in `src/dispatch.rs`; snapshots before write; `run_verify_command` uses 120s timeout | Change default verified apply attempts from 2 to 1 in `effective_max_attempts`; keep explicit `max_attempts` overrides |
| Format/schema retry | Recover malformed model JSON without writing files | `dispatch_with_retry` in `src/dispatch.rs`; `FORMAT_RETRIES = 3`; strict JSON schema via `response_format` and `validate_response` | Keep separate from apply retries; classify exhausted format/schema failure as `failure_category` |
| Rust import preflight | Catch hallucinated `use crate::...` modules before write | `unresolved_crate_imports` in `src/dispatch.rs` using `repomap::known_rust_modules` | Keep retryable; classify as `preflight`; no Python equivalent yet |
| Failure taxonomy | Make failed dispatches analyzable by type | Events exist in dispatch JSONL and `scripts/dispatch_cost_report.py` has `ERROR_EVENTS` | Add top-level `failure_category` to `apply_result` and `error_result`; aggregate by category in cost report |
| Token accounting | Estimate paid frontier-token savings | `run_awl_arm.sh` sums `usage.prompt_tokens` and `usage.completion_tokens`; `tally.py` currently accepts blended `--cost-per-mtok` | Preserve split prompt/completion tokens; add `--input-cost-per-mtok` and `--output-cost-per-mtok` to tally/report scripts |
| Step 1 sweep | Compare 7B-only and 14B-only local arms | `experiments/run_awl_arm.sh` runs all task directories sequentially at `AWL_LEVEL`, outputting `experiments/results/awl_arm.jsonl` | Add model override passthrough and run separate 7B/14B result files or isolated output directories |

## Data Flow and Artifacts

```text
experiments/tasks/<id>/setup.sh
-> recreates experiments/sandbox/<id>/ with target file and unittest tests
-> experiments/tasks/<id>/task.json dispatch object
-> experiments/run_awl_arm.sh extracts dispatch JSON
-> awl dispatch --level 2 --apply --auto-repomap [--model override after patch]
-> src/main.rs parses DispatchOptions
-> src/dispatch.rs builds prompt, optional repo map, and Ollama chat request
-> local Ollama OpenAI-compatible /v1/chat/completions
-> strict JSON response parsed and validated
-> target file snapshot, write, verify_command, rollback-or-keep
-> top-level dispatch JSON in experiments/results/<id>.json
-> per-task aggregate record in experiments/results/awl_arm*.jsonl
-> dispatch event log under Awl config dir dispatches/*.jsonl
-> experiments/tally.py + baseline.csv produce markdown comparison
-> scripts/dispatch_cost_report.py summarizes dispatch-log telemetry
```

### Data Formats

| Artifact | Format | Producer | Consumer |
| --- | --- | --- | --- |
| `experiments/tasks/*/task.json` | JSON | Hand-authored task pack | `run_awl_arm.sh`, frontier-baseline operator |
| Dispatch stdin | JSON object, usually `task_json["dispatch"]` | `run_awl_arm.sh` | `src/main.rs` / `src/dispatch.rs` |
| Model response | Strict JSON object: `status`, `code`, `explanation`, `files_modified` | Ollama worker | `dispatch_with_retry` |
| Dispatch result | JSON | `src/dispatch.rs` | `run_awl_arm.sh`, user/MCP host |
| Dispatch event log | JSONL | `DispatchLog` in `src/dispatch.rs` | `dispatch_cost_report.py`, manual debugging |
| Awl arm aggregate | JSONL | `run_awl_arm.sh` | `experiments/tally.py` |
| Baseline | CSV: `task_id,frontier_tokens,frontier_pass,wall_ms` | Manual frontier run | `experiments/tally.py` |

## Step 1 Sweep Plan

The approved sweep is not an escalation policy. It is two separate fixed-model runs:

1. 7B-only: `qwen2.5-coder:7b-instruct-q4_K_M`
2. 14B-only: `qwen2.5-coder:14b`

After the `DispatchOptions.model` and `AWL_MODEL_OVERRIDE` patches land, the intended commands are:

```bash
AWL_MODEL_OVERRIDE=qwen2.5-coder:7b-instruct-q4_K_M ./experiments/run_awl_arm.sh
AWL_MODEL_OVERRIDE=qwen2.5-coder:14b ./experiments/run_awl_arm.sh
```

To avoid overwriting `experiments/results/awl_arm.jsonl`, either extend `run_awl_arm.sh` with `AWL_RESULTS_FILE`/`AWL_RESULTS_DIR` or run each configuration after moving the previous result to a model-specific filename. A cleaner patch is to make the script derive a sanitized suffix from `AWL_MODEL_OVERRIDE`, for example `awl_arm_qwen2.5-coder_7b-instruct-q4_K_M.jsonl`.

## Resource Estimates

| Computation | Time | Memory / Disk | Storage | Hardware / Services |
| --- | --- | --- | --- | --- |
| Existing 7B Step 1, 3 tasks | Observed 15.4s, 17.6s, and 29.7s wall per task | Model on disk: 4.7 GB | Per-task JSON plus one JSONL; small | Local CPU/GPU through Ollama, `python3`, Cargo-built `awl` |
| 14B Step 1, 3 tasks | Unmeasured here; prior report expects roughly 30-60s/task | Model on disk: 9.0 GB | Same as 7B | Same; more memory pressure than 7B |
| Verification commands | Usually seconds for current Python unittest tasks; hard timeout 120s | Minimal | Verifier output truncated in return, full detail in logs | `bash -lc`, `python3 -m unittest` |
| Repomap injection | Lightweight for current repo; scales with Rust/Python source count | Tree-sitter parse + graph memory | None unless logs retained | Rust process using tree-sitter and petgraph |
| Dispatch logs | One JSONL file per dispatch under Awl config dir | Minimal per run, can grow with raw model content | `~/.config/awl/dispatches/*.jsonl` unless `AWL_CONFIG_DIR` changes | Local filesystem |

Grounded local model inventory from `ollama list` on 2026-05-01:

| Model | Size | Role |
| --- | ---: | --- |
| `qwen2.5-coder:7b-instruct-q4_K_M` | 4.7 GB | Current L2 implementation default |
| `qwen2.5-coder:14b` | 9.0 GB | Candidate fixed 14B sweep model and current default agent model |
| `qwen2.5-coder:3b-instruct-q4_K_M` | 1.9 GB | Current verifier-tier model |

## Current Empirical Anchor

The existing local 7B Step 1 result file records three tasks:

| Task | Result | Attempts | Tokens | Wall Time | Key Signal |
| --- | --- | ---: | ---: | ---: | --- |
| `01_string_helper` | failed | 2 | 4264 | 29686 ms | Failed trailing-newline preservation twice |
| `02_validate_input` | passed | 1 | 2073 | 15431 ms | Basic string-validation task passed |
| `03_fix_off_by_one` | passed | 1 | 2264 | 17577 ms | Existing-code off-by-one fix passed |

This supports the reliability patch rationale: same-model verify retry can burn local tokens without changing the outcome when the failure is a capability or edge-case gap. It does not yet establish 14B superiority, because no 14B run is recorded.

## Integration Points

### Rust

- `src/dispatch.rs`
  - Extend `DispatchOptions` with `model: Option<String>`.
  - Add an effective model helper that falls back to `defaults::configured_model_for_level`.
  - Add `failure_category` to `apply_result` and `error_result`.
  - Wire categories at all current return sites: `format`, `schema`, `preflight`, `verify`, `timeout`, `network`, `model_status`, `unknown`.
  - Change `effective_max_attempts(None, apply=true, has_verify=true)` from 2 to 1 and add/adjust unit tests.

- `src/main.rs`
  - Add `--model <name>` to `parse_dispatch_options`.
  - Document the flag under the `dispatch` help text.

- `src/mcp_server.rs`
  - Add optional `model` to the `awl_dispatch` input schema.
  - Read `model` in `execute_dispatch`, include it in the synthesized dispatch input only if needed, and set `options.model`.
  - Update schema tests to check the optional property.

- `src/defaults.rs`
  - Keep current model defaults and environment/config fallback behavior.
  - No new default should replace 7B; 14B is selected only by explicit per-dispatch override for this experiment.

### Shell and Python

- `experiments/run_awl_arm.sh`
  - Add `MODEL_ARGS=()` and append `--model "$AWL_MODEL_OVERRIDE"` when the environment variable is non-empty.
  - Include the selected model in each JSONL record through existing `telemetry.model`.
  - Avoid overwriting cross-model results by adding configurable result filenames.

- `experiments/tally.py`
  - Replace or supplement `--cost-per-mtok` with `--input-cost-per-mtok` and `--output-cost-per-mtok`.
  - Use per-record `prompt_tokens` and `completion_tokens`; keep total-token savings for readability.
  - Report pass rate and cost estimates by result file/model configuration.

- `scripts/dispatch_cost_report.py`
  - Keep `usage_tokens()` split.
  - Add input/output cost flags and compute local-token totals by prompt/completion.
  - Aggregate dispatches by `failure_category`, falling back to event-derived categories for old logs.

## Validation Strategy

| Result | Validation Method | Acceptance |
| --- | --- | --- |
| Model override works | Unit tests for CLI/MCP parsing plus one dry dispatch with a harmless task if Ollama is available | `telemetry.model` equals override; fallback still equals configured level model |
| Default verify attempts reduced | Unit test for `effective_max_attempts(None, true, true)` | Returns 1; explicit task `max_attempts` still respected and clamped 1-5 |
| Failure taxonomy present | Unit tests on `apply_result`/`error_result`; fixture parse in cost-report script | All error returns include `failure_category`; reports group categories |
| Split pricing correct | Small JSONL fixture with known prompt/completion counts | Cost equals `prompt * input_rate + completion * output_rate` divided by 1M |
| Sweep reproducible | Run both model configurations against the same task pack after `setup.sh` reset | Separate result artifacts, same task IDs, same verifier commands |

Recommended implementation checks:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/dispatch_cost_report.py --json
./experiments/tally.py --awl <model-specific-awl-arm.jsonl> --baseline experiments/results/baseline.csv
```

The final two commands depend on generated result/baseline files. Absence of `baseline.csv` should remain an explicit missing-input state, not an inferred pass.

## Pitfalls and Stability Issues

- The current task JSON files contain explicit `max_attempts` values of 2 or 3. If the experiment is meant to test the new default one-attempt policy, those task specs must be edited or the harness must override them; otherwise the default change will not affect Step 1.
- `experiments/run_awl_arm.sh` currently truncates `experiments/results/awl_arm.jsonl` on each run. Fixed-model sweeps need distinct output paths to prevent silent overwrite.
- `run_verify_command` reports timeouts as verifier failures, but without a category patch they are indistinguishable from semantic test failures in top-level results.
- Network/Ollama errors can currently escape as command errors rather than structured dispatch JSON, depending on where `send_request` fails. The sweep harness should record nonzero exit codes and missing JSON as infrastructure failures.
- `dispatch_with_retry` keeps raw model content in dispatch logs. This is useful for debugging but can grow logs and may contain generated code; logs should not be treated as clean benchmark summaries.
- Repomap preflight only checks Rust crate-internal imports. Python benchmark failures such as the trailing-newline bug are caught only by verifier tests, not static preflight.
- The current baseline process is manual. Frontier token counts must be collected consistently as input/output where possible; a single blended `frontier_tokens` field is insufficient for asymmetric pricing.
- 14B may improve pass rate but increase wall time and local resource pressure. The sweep should report pass rate, attempts, wall time, prompt tokens, completion tokens, and failure category together rather than optimizing only for token reduction.

## Sources Inspected

- `src/dispatch.rs` -- dispatch options, model call, JSON retry, apply/verify rollback, telemetry, result shape.
- `src/defaults.rs` -- default model names, environment/config model fallback, Ollama URL construction.
- `src/main.rs` -- dispatch CLI parsing and user help.
- `src/mcp_server.rs` -- MCP `awl_dispatch` schema and argument plumbing.
- `experiments/run_awl_arm.sh` -- local-arm sweep driver and JSONL record construction.
- `experiments/tally.py` -- Awl/baseline comparison and current blended-cost handling.
- `scripts/dispatch_cost_report.py` -- dispatch log parser, token extraction, and current success/failure event taxonomy.
- `experiments/tasks/*/task.json` and `setup.sh` -- current Step 1 task pack and verifier commands.
- `GPD/research-map/ARCHITECTURE.md` and `GPD/research-map/STRUCTURE.md` -- repository architecture and data-flow map.
- `REPORT_DISPATCH_RELIABILITY.md` -- approved reliability-patch rationale, current 7B failure evidence, and 7B/14B sweep decision.
