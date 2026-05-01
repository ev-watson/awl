# VALIDATION.md — Awl Project Testing and Validation Practices

**Analysis Date:** 2026-04-30
**Focus:** methodology (adapted from physics template for a Rust software engineering project)
**Project:** Awl v0.3.0 — Rust CLI + MCP server for dispatching bounded coding tasks to local Ollama models

---

## CI/CD Pipeline

### Workflow Definition

File: `.github/workflows/ci.yml`

**Triggers:**
- Push to `main`
- All pull requests (any branch)
- Manual dispatch (`workflow_dispatch`)

**Matrix:**
- `ubuntu-latest` and `macos-latest` (cross-platform)
- `fail-fast: false` — both OS jobs run to completion even if one fails

**Steps (in order):**

| Step | Command | Gate? |
|------|---------|-------|
| Checkout | `actions/checkout@v6` | — |
| Install Rust | `dtolnay/rust-toolchain@stable` with `rustfmt, clippy` | — |
| Cache | `Swatinem/rust-cache@v2` | — |
| Format check | `cargo fmt --check` | Yes — PR blocked on format violations |
| Lint | `cargo clippy --all-targets -- -D warnings` | Yes — PR blocked on any clippy warning |
| Test | `cargo test` | Yes — PR blocked on test failures |
| Cost report smoke test | `python3 scripts/dispatch_cost_report.py --logs-dir target/no-dispatches` | Yes — ensures the Python analysis script parses without errors on empty input |
| Optional dispatch eval | `scripts/dispatch_eval.sh` (only if `AWL_RUN_DISPATCH_EVAL=1` and `ollama` is installed) | No — opt-in only |
| Package dry run | `cargo package --locked --no-verify --list` (Ubuntu only) | Yes — ensures crate packaging works |

**Required checks for PR merge:** `checks (ubuntu-latest)` and `checks (macos-latest)` must both pass. Enforced by branch protection with `required_status_checks.strict: true`.

### Quality Gate Summary

All PRs must clear:
1. `cargo fmt --check` — consistent formatting
2. `cargo clippy --all-targets -- -D warnings` — zero warnings (pedantic + all enabled)
3. `cargo test` — all unit tests pass
4. `python3 scripts/dispatch_cost_report.py --logs-dir target/no-dispatches` — Python script is valid
5. Cross-platform: both Ubuntu and macOS

`HANDOFF_TO_GPD.md` hard constraint: "Do not weaken lint gates in `.github/workflows/ci.yml` (`-D warnings` stays)."

---

## Unit Test Inventory

**Total test functions:** 56 `#[test]` annotations across 10 modules.

### Per-Module Test Breakdown

| Module | File | Test Count | Test Names |
|--------|------|-----------|------------|
| `dispatch` | `src/dispatch.rs:1373-1600` | 14 | `effective_target_prefers_cli_value`, `effective_target_uses_single_target_file`, `snapshot_restore_removes_new_dispatch_file`, `snapshot_restore_rewrites_existing_dispatch_file`, `verify_command_reports_failure_without_panicking`, `compact_applied_output_strips_code`, `non_apply_output_separates_intended_from_changed_files`, `compact_issues_points_to_dispatch_log_when_truncated`, `prune_dispatch_logs_only_removes_old_jsonl_files`, `preflight_rejects_ambiguous_apply_targets`, `unresolved_imports_flags_unknown_crate_module`, `unresolved_imports_skips_non_rust_targets`, `unresolved_imports_dedupes_repeated_idents`, `unresolved_imports_returns_empty_when_all_known` |
| `mcp_server` | `src/mcp_server.rs:443-575` | 8 | `test_initialize_response`, `test_initialize_negotiates_older_supported_protocol`, `test_ping_response`, `test_tools_list`, `test_health_tool`, `test_agent_tool_is_disabled_by_default`, `test_unknown_method`, `test_dispatch_tool_schema` |
| `agent` | `src/agent.rs:862-939` | 6 | `test_phase_completion_signal`, `test_truncate`, `test_refresh_system_message_updates_existing_prompt`, `test_extract_tool_path_variants`, `test_gate_signals_filtered`, `test_refresh_system_message_inserts_missing_system_prompt` |
| `tools` | `src/tools.rs:867-947` | 5 | `test_cache_hit`, `test_cache_eviction`, `test_hash_args_deterministic`, `test_restore_snapshot_rewrites_previous_contents`, `test_restore_snapshot_removes_new_file` |
| `defaults` | `src/defaults.rs:141-203` | 5 | `base_url_defaults_when_env_is_missing_or_blank`, `base_url_normalization_appends_openai_compat_suffix`, `base_url_normalization_accepts_ollama_host_style_values`, `tags_url_is_derived_from_api_root`, `mcp_agent_env_parsing_accepts_common_enabled_values` |
| `repomap` | `src/repomap.rs:612-701` | 3 | `import_edges_are_not_duplicated_by_unrelated_symbols`, `known_rust_modules_collects_file_stems_and_mod_dirs`, `generate_parses_python_files` |
| `config` | `src/config.rs:128-177` | 3 | `config_dir_prefers_xdg_home`, `config_dir_falls_back_to_home_config`, `config_dir_uses_custom_dir_override` |
| `phases` | `src/phases.rs:145-168` | 2 | `test_needs_human_phase`, `test_detect_gate_is_phase_aware` |

### Modules Without Tests

| Module | File | Lines | Notes |
|--------|------|-------|-------|
| `hashline` | `src/hashline.rs` | 482 | No `#[cfg(test)]` module |
| `llm_io` | `src/llm_io.rs` | 50 | No tests for `strip_code_fences` or `sanitize_json_strings` |
| `safety` | `src/safety.rs` | 199 | No tests for path resolution, workspace containment, or shell command validation |
| `main` | `src/main.rs` | 306 | CLI argument parsing untested (typical for main) |
| `init` | `src/init.rs` | 250 | No tests |
| `plan` | `src/plan.rs` | 148 | No tests |
| `session` | `src/session.rs` | 188 | No tests |
| `mcp_client` | `src/mcp_client.rs` | 260 | No tests |
| `doctor` | `src/doctor.rs` | 196 | No tests |

**Key gap:** `safety.rs` (path containment, shell command validation) has zero tests despite being security-critical. The shell command allowlist, forbidden fragment detection, cargo/git subcommand validation, and workspace boundary enforcement are all untested.

**Key gap:** `llm_io.rs` (JSON sanitization) has zero tests. The `sanitize_json_strings` function handles subtle edge cases (escaped characters, control character encoding) and is on the dispatch hot path.

### Test Patterns

**Filesystem tests:** Tests that create temporary files use nanosecond timestamps for unique paths:
```rust
fn test_path(name: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock drift")
        .as_nanos();
    format!("target/awl-dispatch-tests/{name}-{nanos}.txt")
}
```
Tests clean up after themselves with `fs::remove_file` or `fs::remove_dir_all`.

**JSON construction:** Tests use `serde_json::json!()` macro for inline JSON fixtures.

**Assertion style:** Standard `assert_eq!`, `assert!`, `assert!(expr.contains(...))`. No custom assertion macros.

**No mocks:** No HTTP mocking framework. The `send_request` function (Ollama HTTP calls) is not tested in CI — it requires a running Ollama instance.

**No integration tests:** The `tests/` directory does not exist. All tests are unit tests within `src/*.rs` modules.

---

## Experiment Methodology

### A/B Design

Source: `experiments/README.md`, `REPORT_DISPATCH_RELIABILITY.md`

**Hypothesis:** Dispatching bounded coding tasks to a local 7B model via Awl saves frontier tokens versus solving the same tasks on a frontier model.

**Arms:**

| Arm | Description | Driver |
|-----|-------------|--------|
| Awl arm (treatment) | Tasks dispatched to local Ollama model via `awl dispatch --level 2 --apply --auto-repomap` | `experiments/run_awl_arm.sh` (automated) |
| Frontier baseline (control) | Same tasks solved by frontier model directly (Claude Code without Awl tools) | Manual (per `experiments/README.md`) |

**Primary metric:** Aggregate frontier-token reduction (%) = `(baseline_tokens - awl_tokens) / baseline_tokens * 100`

**Secondary metrics:**
- Awl pass rate: passing tasks / tasks attempted
- Per-task token savings (passing tasks only)
- Wall time comparison
- Estimated paid cost avoided at specified $/MTok rates

**Success thresholds** (from `UPDATED_PROGRESS_REPORT.md`):
- Token reduction: >= 25-40% aggregate
- Awl pass rate: >= 60-70%

### Task Pack Structure

Source: `experiments/tasks/`

Each task is a directory containing:
- `task.json` — dispatch spec with `id`, `description`, `difficulty`, and `dispatch` block (task, constraints, target_path, verify_command, max_attempts)
- `setup.sh` — idempotent script that creates a clean sandbox with stub code and test files

**Current tasks (3 of target 10):**

| Task ID | Type | Difficulty | Language | Description |
|---------|------|-----------|----------|-------------|
| `01_string_helper` | Write-from-scratch | Easy | Python | Implement `capitalize_each_line` |
| `02_validate_input` | Write-from-scratch | Easy | Python | Implement `validate_email` |
| `03_fix_off_by_one` | Edit-existing | Medium | Python | Fix off-by-one bug in `moving_average` |

**Task conventions** (from `experiments/README.md`):
- `setup.sh` must be idempotent — deletes and recreates `experiments/sandbox/{id}/`
- `task.json.dispatch.target_path` must point inside `experiments/sandbox/{id}/`
- `verify_command` must exit 0 on success (exit code is the only oracle)
- Tasks should be bounded — "anything Awl's L2 (7B) reasonably can't do isn't useful as a savings benchmark"

**Missing task types (needed for >= 10 task pack):**
- Tasks exercising `context_paths` (no examples yet)
- Rust code generation tasks
- Multi-constraint tasks
- Tasks with more complex verify commands (e.g., compile + test)

### Experiment Harness

**Awl arm driver:** `experiments/run_awl_arm.sh`

For each task:
1. Run `setup.sh` to create clean sandbox
2. Extract `dispatch` block from `task.json` via Python one-liner
3. Pipe dispatch JSON into `awl dispatch --level $LEVEL --apply --auto-repomap`
4. Capture exit code, wall time (Python `time.time()` milliseconds), dispatch result
5. Extract usage tokens (`prompt_tokens`, `completion_tokens`, `total_tokens`) from result
6. Append JSONL record to `experiments/results/awl_arm.jsonl`
7. Save raw dispatch result to `experiments/results/{id}.json`

**Environment overrides:**
- `AWL_BIN` — path to awl binary (default: `cargo run --quiet --`)
- `AWL_LEVEL` — dispatch level (default: 2)
- `AWL_MODEL_OVERRIDE` — planned but not yet implemented in the script

**Frontier baseline arm:** Manual process. For each task, the operator:
1. Reads `task.json` dispatch description
2. Runs `setup.sh` to materialize sandbox
3. Hands task to frontier model without Awl tools
4. Runs verify command and records pass/fail
5. Records input + output tokens and wall time

Results go in `experiments/results/baseline.csv` with columns: `task_id, frontier_tokens, frontier_pass, wall_ms`.

**Tally script:** `experiments/tally.py`
- Reads `awl_arm.jsonl` and `baseline.csv`
- Produces per-task and aggregate markdown report
- Accepts `--cost-per-mtok` for avoided-cost estimation
- **Pending change:** split into `--input-cost-per-mtok` and `--output-cost-per-mtok` (`HANDOFF_TO_GPD.md` item 4)

### Partial Step 1 Results

Source: `REPORT_DISPATCH_RELIABILITY.md` "Step 1 experiment state"

Run on 2026-04-30, L2 (7B-q4), 3 tasks:

| Task | Status | Attempts | Tokens | Wall (ms) | Notes |
|------|--------|----------|--------|-----------|-------|
| `01_string_helper` | error | 2 (max) | 4264 | 29686 | Failed `test_preserves_trailing_newline` both attempts |
| `02_validate_input` | ok | 1 | 2073 | 15431 | All 8 tests passed first try |
| `03_fix_off_by_one` | ok | 1 | 2264 | 17577 | All 5 tests passed first try |

**Key finding:** 7B-q4 has a deterministic capability gap on the trailing-newline edge case. Retry with same model reproduced the same wrong answer, burning ~2200 extra tokens with zero new information.

**Not yet run:**
- Frontier baseline arm (no baseline data exists)
- Tally script end-to-end
- Any 14B configuration
- Any escalation configuration

---

## Cross-Check Patterns

### Dispatch Contract Validation

The dispatch contract is validated at three levels:

1. **Schema enforcement:** `dispatch_response_format()` sends strict JSON schema to Ollama; `validate_response()` verifies post-hoc (`src/dispatch.rs:489-511`)
2. **Preflight checks:** `preflight()` validates context paths exist and target path is writable; `unresolved_crate_imports()` catches hallucinated Rust module references (`src/dispatch.rs:400-443`)
3. **Verify-then-rollback:** apply mode writes to disk, runs verify command, rolls back on failure — the accept/reject oracle is the verify command exit code

### Trusted vs. Claimed Side Effects

Apply mode distinguishes between:
- `files_changed` / `files_modified` — actual files Awl wrote to disk (trusted)
- `files_intended` — files the model said it modified in non-apply mode (untrusted model self-report)

This distinction is enforced by `normalize_non_apply_output()` at `src/dispatch.rs:1287-1310`.

### Health Check System

`awl doctor` (`src/doctor.rs`) validates:
- Config file parses correctly
- Ollama API is reachable
- All three configured models (agent, implementation, verification) are pulled in Ollama
- Sessions directory is writable
- Workspace root resolves
- MCP config is valid (if present)

No automated scheduling — manual invocation only.

### Cost Report Validation

`scripts/dispatch_cost_report.py` is smoke-tested in CI: the pipeline step `python3 scripts/dispatch_cost_report.py --logs-dir target/no-dispatches` verifies the script runs without errors on an empty logs directory. This is a minimal sanity check, not a functional test.

---

## Validation Gaps

### Critical Gaps

1. **No `safety.rs` tests.** The shell command validation (allowlist, forbidden fragments, cargo/git subcommand restrictions) and path containment (workspace boundary enforcement, traversal prevention) have zero unit tests. These are security-critical functions that protect against command injection and path traversal. Every code change to `safety.rs` currently relies on CI passing the rest of the test suite, which never exercises these functions.

2. **No `llm_io.rs` tests.** `strip_code_fences()` and `sanitize_json_strings()` are on the dispatch hot path and handle subtle edge cases (nested backticks, escaped characters, control characters in JSON strings). No tests verify correct behavior on malformed LLM output.

3. **No integration tests.** There is no `tests/` directory and no test that exercises the full dispatch pipeline end-to-end (stdin JSON -> model call -> apply -> verify -> rollback -> stdout JSON). All tests are unit tests of individual functions.

4. **No HTTP mocking.** `send_request()` (the Ollama HTTP call) cannot be tested without a running Ollama instance. This means the full apply flow is untestable in CI.

### Moderate Gaps

5. **Experiment baseline arm is manual.** The frontier-baseline arm requires a human to run tasks, record tokens, and fill in `baseline.csv`. This is error-prone and not reproducible. There is no tooling to automate or validate baseline data collection.

6. **Only 3 of 10 target tasks exist.** The task pack needs >= 10 tasks for a defensible Step 1 result. Missing: tasks with `context_paths`, Rust targets, multi-file tasks, tasks with compile-then-test verify commands.

7. **No 14B configuration data.** All experiment runs used 7B-q4. The 14B model is pulled locally but has never been tested through the experiment harness.

8. **Cost reporting uses blended rate.** `tally.py` takes a single `--cost-per-mtok` but frontier pricing is asymmetric ($5 input / $25 output for Claude Opus 4.7). This materially distorts savings estimates. Pending fix in `HANDOFF_TO_GPD.md` item 4.

### Minor Gaps

9. **`hashline.rs` (482 lines) has no tests.** Lower priority since it is a utility module, not on the security or dispatch critical path.

10. **`session.rs`, `plan.rs`, `init.rs`, `mcp_client.rs`, `doctor.rs` all lack tests.** These modules are lower risk but contribute to the gap between tested and untested code.

11. **No `cargo test` coverage reporting.** There is no `tarpaulin`, `llvm-cov`, or other coverage tool in CI. The ratio of tested to untested code is not tracked.

---

## Verification Commands Used in Tasks

Each experiment task defines its own verify command:

| Task | Verify Command | Oracle |
|------|---------------|--------|
| `01_string_helper` | `python3 -m unittest discover -v -s experiments/sandbox/01 -p 'test_*.py'` | Exit code 0 = all tests pass |
| `02_validate_input` | `python3 -m unittest discover -v -s experiments/sandbox/02 -p 'test_*.py'` | Exit code 0 = all tests pass |
| `03_fix_off_by_one` | `python3 -m unittest discover -v -s experiments/sandbox/03 -p 'test_*.py'` | Exit code 0 = all tests pass |

All three use Python `unittest discover`. The test files are created by `setup.sh` alongside the stub code, ensuring the test oracle is always fresh and deterministic.

---

## Testing Policy

From `HANDOFF_TO_GPD.md` hard constraints:
- "All new public Rust functions need at least one unit test."
- Every PR must pass `cargo clippy --all-targets -- -D warnings` and `cargo test` on Linux and macOS.
- "When CI fails for a non-obvious reason. Do not 'fix' by suppressing lints or removing tests."

**Implied convention (from existing code):** Tests are placed in a `#[cfg(test)] mod tests` block at the bottom of each source file. No separate test files.

---

_Analysis performed on commit `29ef94f` (main)._
