# ARCHITECTURE.md - Awl Computational Architecture

**Analysis Date:** 2026-05-01
**Focus:** computation
**Project:** Awl v0.3.0 - Rust CLI + stdio MCP server for bounded local-model coding dispatch

> **Adaptation note:** Awl is a software engineering research project, not a
> physics project. "Computational pipeline" here means the local-model dispatch
> algorithm, CLI/MCP data flow, verification/rollback mechanism, telemetry, and
> experiment harness used to test the frontier-token-savings hypothesis.

## Anchor-Carrying Context

The active anchor registry in `GPD/research-map/REFERENCES.md` must remain
visible when interpreting this architecture. The project contract is not
authoritative because `GPD/state.json` is missing; `GPD/state.json.lock` exists
but carries no contract payload.

| Anchor | Architectural implication |
|--------|---------------------------|
| ANC-001 product hypothesis | `HANDOFF_TO_GPD.md` line 11 defines the north-star claim: bounded local execution should measurably save frontier tokens on net. The dispatch architecture is only valuable insofar as it supports this empirical test. |
| ANC-002 success thresholds | `experiments/README.md` lines 89-95 and `UPDATED_PROGRESS_REPORT.md` define the Step 1 gates: >=25-40% token reduction and >=60-70% Awl pass rate. |
| ANC-003 dispatch v2 contract | `src/dispatch.rs` lines 269-293 and 672-690 define the strict response schema `{status, code, explanation, files_modified}`. This is the model-facing API surface. |
| ANC-004 rollback invariant | `src/dispatch.rs` lines 1119-1158 and tests at lines 1434-1451 enforce snapshot/write/verify/rollback. |
| ANC-005 cost rates and ANC-011 tokenizer inflation | `HANDOFF_TO_GPD.md` and `REPORT_DISPATCH_RELIABILITY.md` require split accounting for Claude Opus 4.7 $5/MTok input and $25/MTok output, with possible tokenizer inflation. Current scripts still use blended costs. |
| ANC-006 partial Step 1 results | `REPORT_DISPATCH_RELIABILITY.md` and `experiments/results/awl_arm.jsonl` show 3 Awl-arm tasks, 2 passes, 1 deterministic 7B failure. |
| ANC-007 retry policy decision | `REPORT_DISPATCH_RELIABILITY.md` confirms the default apply+verify retry should drop from 2 to 1; current `src/dispatch.rs` line 1111 still defaults to 2. |
| ANC-008 model override decision | Frontier-side model choice per dispatch is confirmed, but `src/dispatch.rs` `DispatchOptions` has no `model` field and `src/main.rs` has no `dispatch --model` flag. |
| ANC-010 branch protection rules | `HANDOFF_TO_GPD.md` forbids direct pushes to `main` and requires PRs with `checks (ubuntu-latest)` and `checks (macos-latest)`. |

## System Boundary

Awl is a local worker invoked either as a CLI (`src/main.rs`) or as a stdio MCP
server (`src/mcp_server.rs`). The frontier assistant remains the orchestrator:
it decides whether a task is bounded enough to dispatch, selects level 2 or
level 3, supplies `target_path`, `context_paths`, constraints, and
`verify_command`, then reviews the compact result. Awl does not call paid
external APIs; the model backend is local Ollama through an OpenAI-compatible
`/v1/chat/completions` endpoint.

Primary runtime regimes:

- CLI dispatch: stdin JSON -> `awl dispatch --level 2|3` in `src/main.rs`.
- MCP dispatch: `awl_dispatch` tool -> JSON-RPC stdio in `src/mcp_server.rs`.
- Local agent loop: `awl agent` / `awl_agent`, implemented in `src/agent.rs`,
  but hidden from MCP unless `AWL_ENABLE_MCP_AGENT=1`; it is not the main
  savings path.
- Repository mapping: `awl repomap` and `auto_repomap` use `src/repomap.rs` to
  inject bounded local code context without making the frontier paste files.
- Experiment harness: `experiments/run_awl_arm.sh`, `experiments/tally.py`, and
  `experiments/tasks/*` implement the A/B savings study.

## Dispatch Data Flow

The dispatch hot path is `src/dispatch.rs`.

1. `src/main.rs` parses CLI flags in `parse_dispatch_options()` and reads stdin
   JSON. MCP requests arrive through `execute_dispatch()` in `src/mcp_server.rs`.
2. `dispatch::run_capture()` parses the dispatch JSON into `TaskSpec`
   (`src/dispatch.rs` lines 184-211). If raw JSON contains bare control
   characters inside strings, it retries after `sanitize_json_strings()` from
   `src/llm_io.rs`.
3. Effective values are resolved: `apply`, `target_path`, `verify_command`,
   `max_return_chars`, `max_attempts`, and repo-map options (`src/dispatch.rs`
   lines 501-528).
4. A JSONL dispatch log is created under `config_dir()/dispatches` by
   `DispatchLog::new()` (`src/dispatch.rs` lines 59-65). This usually resolves
   to `~/.config/awl/dispatches/*.jsonl`, unless `AWL_CONFIG_DIR` or
   `AWL_CONFIG_PATH` changes configuration paths.
5. `preflight()` validates `context_paths`, target writability, and apply-mode
   target ambiguity before model inference (`src/dispatch.rs` lines 370-395).
6. If `auto_repomap` is true, `build_repo_map_context()` calls
   `crate::repomap::generate(Path::new("."), budget, focus)` and appends the
   resulting map to the worker prompt (`src/dispatch.rs` lines 439-461).
7. `build_user_message()` assembles task description, target path, context,
   local file contents, constraints, verify command, and optional repo map
   (`src/dispatch.rs` lines 295-343).
8. Model and endpoint are selected through `defaults::configured_ollama_base_url()`,
   `defaults::ollama_chat_completions_url()`, `defaults::configured_model_for_level()`,
   and `defaults::max_tokens_for_level()` (`src/dispatch.rs` lines 571-576).
9. Awl sends a deterministic structured-output chat request with
   `temperature: 0.0`, `stream: false`, and `response_format:
   dispatch_response_format()` (`src/dispatch.rs` lines 583-599).
10. Non-apply mode returns generated code after `normalize_non_apply_output()`.
    Apply mode enters `run_apply_flow()`.

## Dispatch Contract

The model-facing contract has two reinforcing layers:

- System prompt at `src/dispatch.rs` lines 269-293 tells the local worker to
  return only JSON with `status`, `code`, `explanation`, and `files_modified`.
- `dispatch_response_format()` at `src/dispatch.rs` lines 672-690 sends an
  OpenAI-compatible `json_schema` response format with `strict: true` and
  `additionalProperties: false`.

Post-hoc validation is still required because local providers can produce
malformed outputs. `validate_response()` checks object shape and field types
(`src/dispatch.rs` lines 463-486). `dispatch_with_retry()` handles JSON parse
errors and schema errors with up to `FORMAT_RETRIES = 3` correction attempts
(`src/dispatch.rs` lines 23 and 959-1053). These format retries are separate
from apply/verify attempts.

Important limitation: the contract is strict only for the model response.
Awl's returned result includes additional operational fields such as
`files_changed`, `checks_run`, `checks_passed`, `attempts`, `usage`,
`open_issues`, and `telemetry`.

## Apply/Verify/Rollback Flow

Apply mode is the reliability core and the main defense against failed local
work turning into paid frontier debugging.

`run_apply_flow()` in `src/dispatch.rs` lines 733-957 implements:

1. Generate a candidate through `dispatch_with_retry()`.
2. Reject model `status: error` or missing `code`.
3. For Rust targets, call `unresolved_crate_imports()` with
   `repomap::known_rust_modules()` before writing. This catches invented
   `use crate::missing_module` paths in `.rs` files only (`src/dispatch.rs`
   lines 398-437 and 792-835).
4. Snapshot target state with `capture_snapshot()` (`src/dispatch.rs`
   lines 1119-1136).
5. Write complete replacement content with `write_target()` (`src/dispatch.rs`
   lines 1138-1147).
6. If a `verify_command` exists, run it under the workspace root with
   `run_verify_command()` and a hardcoded 120 second timeout
   (`src/dispatch.rs` lines 1160-1208).
7. On verify pass, return `status: ok`, `checks_passed: true`, and trusted
   `files_changed`.
8. On verify failure or command error, restore the snapshot
   (`src/dispatch.rs` lines 1149-1158), log rollback, feed compact verifier
   output back to the model if attempts remain, and eventually return error.

The rollback invariant is covered by unit tests
`snapshot_restore_removes_new_dispatch_file` and
`snapshot_restore_rewrites_existing_dispatch_file` in `src/dispatch.rs` lines
1434-1451. This is a load-bearing invariant for ANC-004.

## Retry Design

There are two retry systems:

- Format/schema retry in `dispatch_with_retry()` (`src/dispatch.rs` lines
  959-1053): up to 4 total model calls for malformed JSON or schema mismatch.
  This is intended to recover transient structured-output failures.
- Apply/verify retry in `run_apply_flow()` (`src/dispatch.rs` lines 744-945):
  repeats generation/write/verify after preflight or verifier failure, bounded
  by `effective_max_attempts()`.

Current code sets `effective_max_attempts(raw, apply, has_verify)` to 2 by
default for `apply && has_verify`, otherwise 1 (`src/dispatch.rs` lines
1110-1112). This conflicts with the user-confirmed decision in
`REPORT_DISPATCH_RELIABILITY.md` to default verify retries to 1 after the
`01_string_helper` deterministic failure. Callers can still override with
`max_attempts`, and current experiment tasks explicitly set `max_attempts` in
`experiments/tasks/*/task.json`.

## Model and Provider Choices

Model selection currently happens by level, environment, and config, not per
dispatch:

| Level | Role | Default | Source |
|-------|------|---------|--------|
| L1 | Local agent loop | `qwen2.5-coder:14b` | `src/defaults.rs` line 5 |
| L2 | Implementation dispatch | `qwen2.5-coder:7b-instruct-q4_K_M` | `src/defaults.rs` line 6 |
| L3 | Verification/lint dispatch | `qwen2.5-coder:3b-instruct-q4_K_M` | `src/defaults.rs` line 7 |

`configured_model_for_level()` resolves L2/L3 via environment variable, user
config, then compiled default (`src/defaults.rs` lines 74-88 and 121-132).
Environment variables are `AWL_IMPLEMENTATION_MODEL` and
`AWL_VERIFICATION_MODEL`; L1 uses `AWL_AGENT_MODEL`.

Provider endpoint behavior:

- Base URL precedence is `OLLAMA_BASE_URL`, then `OLLAMA_HOST`, then
  `~/.config/awl/config.json`, then `http://127.0.0.1:11434/v1`
  (`src/defaults.rs` lines 21-40).
- Chat URL is normalized to `{base}/chat/completions` (`src/defaults.rs`
  lines 50-52).
- Model max output is 8192 tokens for L2 and 4096 for L3
  (`src/defaults.rs` lines 58-64).

Open gap tied to ANC-008: per-dispatch model override is not implemented.
`DispatchOptions` lacks a `model: Option<String>` field (`src/dispatch.rs`
lines 30-41), `TaskSpec` lacks `model` (`src/dispatch.rs` lines 184-211),
`awl dispatch` help has no `--model` (`src/main.rs` lines 215-305), and
`awl_dispatch` MCP schema has no `model` property (`src/mcp_server.rs`
lines 53-75).

## Local Grounding and Repo Map Algorithm

Awl has two grounding mechanisms:

- Explicit `context_paths`: local files are read by Awl and inserted into the
  prompt, capped at 8000 chars per file and 24000 chars total
  (`src/dispatch.rs` lines 23-26 and 345-368).
- `auto_repomap`: generates a tree-sitter symbol map using `src/repomap.rs`,
  with focus files from target, context, and `repomap_focus`.

`src/repomap.rs` supports Python and Rust only (`detect_language()` at lines
53-59). It scans non-hidden, non-`target`, non-`node_modules` directories
(`scan_recursive()` lines 78-110), extracts functions/classes/methods/imports,
builds a directed import graph, scores nodes with personalized PageRank, then
renders a token-budgeted map (`render_map()` lines 413-465). The token budget
is approximate (`1 token ~= 4 chars`) and defaults to 4096 globally
(`src/defaults.rs` line 12), while dispatch auto-repomap defaults to 1200 and
clamps to 200-4096 (`src/dispatch.rs` line 454).

Rust preflight reuses `known_rust_modules()` (`src/repomap.rs` lines 467-499)
to build the set of crate-internal modules. Python import preflight does not
exist.

## CLI and MCP Surfaces

CLI entry is `src/main.rs`:

- `awl dispatch`: local model dispatch from stdin JSON.
- `awl dispatches`: list/show/tail/prune dispatch JSONL logs.
- `awl repomap`: generate repository map.
- `awl hashline`: read/apply content-hashed line edits.
- `awl plan`: ask a local model for a plan.
- `awl agent`: full local agent loop.
- `awl serve`: stdio MCP server.
- `awl doctor`: health checks.

MCP server entry is `src/mcp_server.rs`:

- Protocol versions supported at lines 11-17.
- Tool definitions are emitted by `server_tool_definitions()` lines 41-136.
- `awl_dispatch` maps MCP arguments into the same dispatch JSON and
  `DispatchOptions` at lines 231-273.
- `awl_agent` is hidden unless `defaults::mcp_agent_enabled()` is true
  (`src/mcp_server.rs` lines 112-133, 194-203).

The MCP `awl_dispatch` tool is the intended frontier-facing computation
surface. The L1 agent loop in `src/agent.rs` is a local-only alternative and
has guards for max iterations, max text-only turns, repeated-text stalls, and
wall time (`src/agent.rs` lines 15-39 and 82-101).

## Telemetry and Outputs

Every dispatch returns compact JSON and writes detailed local JSONL telemetry.

Returned top-level telemetry is added by `add_top_level_telemetry()` in
`src/dispatch.rs` lines 1288-1302:

- `model`
- `level`
- `elapsed_ms`
- `dispatch_id`
- `log_path`

Dispatch JSONL events include `dispatch_start`, `repomap_injected`,
`model_selected`, `model_response_valid`, `model_response_invalid_json`,
`model_response_invalid_schema`, `format_retries_exhausted`, `file_written`,
`verify_passed`, `verify_failed`, `verify_command_error`,
`preflight_unresolved_imports`, and `model_status_error`
(`src/dispatch.rs` lines 530-562, 579-582, 839-902, 995-1048).

`scripts/dispatch_cost_report.py` reads local logs, sums
`prompt_tokens`, `completion_tokens`, and `total_tokens`, counts success/failure
events, and estimates avoided frontier cost from a blended
`--frontier-cost-per-mtok` (`scripts/dispatch_cost_report.py` lines 15-24 and
158-179). This is not yet aligned with ANC-005 because it does not split input
and output cost rates.

## Experiment Pipeline

The Step 1 experiment tests Awl-assisted local dispatch against a manual
frontier-only baseline.

Awl arm:

1. `experiments/run_awl_arm.sh` loops over `experiments/tasks/*/`.
2. Each task's `setup.sh` recreates `experiments/sandbox/<id>/`.
3. The script extracts the `dispatch` block from `task.json`.
4. It runs `cargo run --quiet -- dispatch --level "$LEVEL" --apply --auto-repomap`
   by default (`experiments/run_awl_arm.sh` line 49).
5. It writes raw output to `experiments/results/<id>.json` and a compact JSONL
   record to `experiments/results/awl_arm.jsonl`.

Baseline arm:

- Manual frontier runs are described in `experiments/README.md` lines 48-73.
- Expected file is `experiments/results/baseline.csv` with
  `task_id,frontier_tokens,frontier_pass,wall_ms`.
- No baseline file was found in the inspected workspace.

Aggregation:

- `experiments/tally.py` loads `awl_arm.jsonl` and `baseline.csv`, computes
  pass rates and token savings, and optionally estimates avoided cost with a
  single blended `--cost-per-mtok` (`experiments/tally.py` lines 59-69 and
  150-154).
- Split input/output pricing is pending and required for ANC-005.

Current local results in `experiments/results/awl_arm.jsonl` match ANC-006:
`01_string_helper` failed after 2 attempts with 4264 local tokens; 
`02_validate_input` passed with 2073 tokens; `03_fix_off_by_one` passed with
2264 tokens. This is insufficient to establish the product hypothesis because
the task pack has only 3 tasks and no frontier baseline.

## Safety and Verification Controls

Path safety is centralized in `src/safety.rs`:

- `workspace_root()` canonicalizes cwd (`src/safety.rs` lines 26-31).
- `resolve_existing_path()` and `resolve_path_for_write()` enforce workspace
  containment for reads and writes (`src/safety.rs` lines 33-73).
- `validate_shell_command()` rejects empty commands, forbids newline,
  semicolon, backtick, and `$(`, and allowlists command names plus cargo/git
  subcommands (`src/safety.rs` lines 75-146).

Verification commands run through `bash -lc` after validation and under the
workspace root (`src/dispatch.rs` lines 1160-1168). The allowed-command policy
is broad enough to include `rm`, `cp`, and `mv`; this is acceptable for task
sandboxes but should be treated as security-sensitive. `GPD/research-map/VALIDATION.md`
already flags `src/safety.rs` as lacking unit tests.

CI in `.github/workflows/ci.yml` enforces:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `python3 scripts/dispatch_cost_report.py --logs-dir target/no-dispatches`
- `cargo package --locked --no-verify --list` on Ubuntu

Branch protection constraints are external to code but captured in
`HANDOFF_TO_GPD.md`: no direct push to `main`, `enforce_admins: true`, and
required checks `checks (ubuntu-latest)` and `checks (macos-latest)`.

## Performance Bottlenecks

Observed and structural bottlenecks:

- Local model latency dominates dispatch wall time. Partial Step 1 results show
  ~15-30s per 7B-q4 task in `experiments/results/awl_arm.jsonl`.
- Same-model verify retry can double local token use without new information.
  The `01_string_helper` failure used 4264 tokens across two attempts.
- `src/dispatch.rs` is a 1600-line hot file containing parsing, preflight,
  prompt construction, HTTP, apply/verify/rollback, telemetry, compaction, and
  tests. This increases review and local-model context costs.
- `auto_repomap` requires tree-sitter parsing and PageRank over the repository.
  This is bounded by budget but still extra per-dispatch work.
- `context_paths` inserts raw file text into the local model prompt. The caps
  prevent runaway prompts but can truncate relevant details without semantic
  awareness.
- `run_verify_command()` polls every 50ms and has a fixed 120s timeout; Rust
  compile/test tasks may approach or exceed that on slower machines.
- Cost reporting is presently a bottleneck for research validity, not runtime:
  blended cost rates in `experiments/tally.py` and `scripts/dispatch_cost_report.py`
  cannot represent the 5x input/output pricing asymmetry.

## Dependencies

Core Rust dependencies from `Cargo.toml`:

- `reqwest`, `tokio`: async HTTP calls to Ollama and async process/runtime use.
- `serde`, `serde_json`: dispatch specs, JSON-RPC, config, telemetry.
- `tree-sitter`, `tree-sitter-python`, `tree-sitter-rust`: repo-map parsing.
- `petgraph`: PageRank graph representation.
- `glob`, `walkdir`: file discovery and tool operations.
- `chrono`, `rand`, `async-trait`: sessions/tools/supporting utilities.

External runtime dependencies:

- Ollama serving the configured local models.
- `python3` for experiment scripts, task tests, and cost scripts.
- Cargo/Rust toolchain for build, tests, CI, and Rust verify commands.

No external paid API dependency is present in the worker code path, consistent
with `HANDOFF_TO_GPD.md`.

## Critical Open Architecture Gaps

1. **Per-dispatch model override missing:** Required for the confirmed 7B/14B
   Step 1 sweep and frontier-side risk-based routing (ANC-008).
2. **Verify retry default still 2:** Current code conflicts with the confirmed
   retry policy change (ANC-007), although task JSON can override attempts.
3. **Failure taxonomy absent from returned results:** `apply_result()` and
   `error_result()` do not include `failure_category`, making routing learning
   weaker.
4. **Cost accounting still blended:** `experiments/tally.py` and
   `scripts/dispatch_cost_report.py` do not support $5 input / $25 output
   split rates (ANC-005).
5. **No frontier baseline data:** Without `experiments/results/baseline.csv`,
   token savings cannot be computed.
6. **Task pack too small and narrow:** Current tasks are all Python, and only
   three exist. The target is at least 10 mixed Python/Rust tasks.
7. **Security-critical safety logic under-tested:** `src/safety.rs` has no unit
   tests despite controlling path containment and verify command validation.

## Architecture Summary

Awl's computation pipeline is coherent for bounded local execution: structured
JSON dispatch, local grounding, deterministic local model calls, strict schema
validation, trusted writes, verifier-driven acceptance, rollback on failure,
and local telemetry. The architecture directly addresses the pre-v2 failure
modes in `UPDATED_PROGRESS_REPORT.md`.

The remaining weakness is not basic plumbing; it is measurement validity and
policy calibration. The code cannot yet run the confirmed 7B/14B per-dispatch
model sweep, still retries verify failures by default, and reports costs with
a blended rate. Until those are fixed and the frontier baseline is run, the
product hypothesis in `HANDOFF_TO_GPD.md` line 11 remains unproven.
