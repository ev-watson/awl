# Validation and Cross-Checks

**Analysis Date:** 2026-05-01

## Reference Context

- Active Reference Registry: none confirmed in `GPD/state.json`; project contract load status is missing.
- Must-read references, prior outputs, user-asserted anchors, and known-good baselines: none supplied.
- Scope note: the repository is a Rust software project, not a physics manuscript. Physics validation categories are retained only where meaningful; otherwise they are marked absent.

## Physics Validation Status

| Validation Category | Status | Evidence / Notes |
|---|---|---|
| Analytic limits / exact solutions | Not present | No physics derivations, exact analytic solutions, or limiting-case calculations were found. |
| Conservation laws | Not present | No physical conservation-law checks found. |
| Sum rules / Ward identities | Not present | No gauge identities, sum rules, response functions, or correlators found. |
| Dimensional analysis | Not applicable | No dimensioned physics equations found. Software quantities use explicit operational units such as milliseconds, tokens, chars, and paths. |
| Comparison with published physics results | Not present | No bibliography, paper comparison, or physics reference reproduction found. |
| Numerical convergence | Not present as a physics/numerics study | There is no mesh, timestep, solver tolerance, or convergence sweep. Dispatch experiments are model/workflow benchmarks, not numerical convergence tests. |

## Software Quality Gates

The project documents and enforces the following local gates:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo package --no-verify --list
```

Evidence:

- `README.md` lists the main local quality gates.
- `CONTRIBUTING.md` instructs contributors to run formatting, clippy, tests, and package listing before opening a pull request.
- `.github/workflows/ci.yml` runs formatting, clippy, tests, `scripts/dispatch_cost_report.py --logs-dir target/no-dispatches`, and an Ubuntu package dry run.
- `.github/workflows/ci.yml` runs on both `ubuntu-latest` and `macos-latest`.
- `.github/workflows/ci.yml` includes optional `scripts/dispatch_eval.sh` only when `AWL_RUN_DISPATCH_EVAL=1`, because it depends on Ollama being installed.

Local verification executed during this mapping:

- `cargo fmt --check`: passed.
- `cargo test`: passed, 56 tests.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `python3 scripts/dispatch_cost_report.py --logs-dir target/no-dispatches --json`: passed with zero dispatches and zero failures.

## Unit Test Structure

Tests are inline Rust unit tests under `#[cfg(test)]` modules, not a separate `tests/` directory.

Observed test areas:

- `src/phases.rs`
  - phase naming and `NeedsHuman` terminal behavior.
  - phase-aware gate detection and verify-failure regression.
- `src/defaults.rs`
  - Ollama base URL normalization.
  - API-root/tag URL derivation.
  - MCP-agent enablement environment parsing.
- `src/config.rs`
  - config directory resolution precedence for custom config dir, XDG config home, appdata, and home fallback.
- `src/agent.rs`
  - phase completion signal naming.
  - truncation behavior.
  - tool-path extraction variants.
  - gate signal set.
  - system-message refresh behavior.
- `src/hashline.rs`
  - deterministic and content-sensitive line hashes.
  - hashline output format.
  - parsing of replace/delete/insert operations.
  - trailing-newline preservation.
- `src/dispatch.rs`
  - effective target-path selection.
  - snapshot/restore for new and existing files.
  - verify command failure reporting.
  - compaction of applied output.
  - separation of actual changed files from intended files.
  - truncation of open issues with dispatch-log pointer.
  - dispatch-log pruning.
  - ambiguous apply-target preflight rejection.
  - unresolved Rust crate import detection.
- `src/mcp_server.rs`
  - initialize, ping, health, tool listing, dispatch schema, unknown method handling.
  - MCP agent tool disabled by default.
- `src/repomap.rs`
  - Rust module discovery.
  - Python parsing.
  - import-edge deduplication and output inclusion checks.
- `src/tools.rs`
  - tool-result cache hit/eviction/hash determinism.
  - snapshot restore and removal behavior.

## Dispatch Validation Practices

The central validation pattern is apply, verify, rollback:

- `src/dispatch.rs` snapshots the target file before writing.
- generated code is written only to the resolved workspace-scoped target.
- `verify_command` is validated by `src/safety.rs` before execution.
- failed verification restores the previous file contents or removes a newly created failed target.
- dispatch results report `checks_run`, `checks_passed`, `attempts`, `open_issues`, and actual `files_changed`.
- full diagnostics are stored in local JSONL dispatch logs; compact summaries are returned to callers.

Important implementation anchors:

- `src/dispatch.rs`: `capture_snapshot`, `write_target`, `restore_snapshot`, `run_verify_command`, `apply_result`, and dispatch-log event emission.
- `src/safety.rs`: workspace path containment and shell command allowlist validation.
- `src/tools.rs`: tool-layer snapshot restore tests mirror dispatch rollback behavior.
- `README.md`: documents that failed checks roll back writes.
- `examples/awl-worker.md`: recommends `apply=true`, a single `target_path`, and `verify_command` when a local check can prove the change.

## Shell and Workspace Safety Checks

Validation is partly preventive:

- `src/safety.rs` allowlists shell programs and restricts cargo/git subcommands.
- shell commands are split around pipes, logical operators, and redirects, then each segment is validated.
- newline, semicolon, backtick, and command substitution fragments are explicitly forbidden.
- existing reads and write targets are canonicalized and checked against the workspace root.
- `Cargo.toml` forbids unsafe Rust code through lint configuration.

Coverage caveat:

- The scan found strong tests for dispatch preflight, snapshot/restore, and verify failure behavior, but no dedicated inline `src/safety.rs` test module was visible. Changes to the shell parser or workspace containment rules should add focused tests there.

## Experiment and Benchmark Harness

The repository contains a controlled A/B harness for frontier-token savings, not a completed benchmark result.

Relevant files:

- `experiments/README.md`: defines the A/B procedure, baseline CSV format, pass/fail thresholds, and task requirements.
- `experiments/run_awl_arm.sh`: runs each task setup, dispatches through Awl, captures status, attempts, token counts, wall time, dispatch id, and model.
- `experiments/tally.py`: compares Awl results with manually collected frontier baselines.
- `experiments/tasks/01_string_helper/task.json`: write-from-scratch Python text helper.
- `experiments/tasks/02_validate_input/task.json`: write-from-scratch email validation helper.
- `experiments/tasks/03_fix_off_by_one/task.json`: edit-existing moving-average off-by-one task.
- `scripts/dispatch_eval.sh`: smoke tests non-apply dispatch, apply success, and apply rollback.
- `scripts/dispatch_cost_report.py`: summarizes dispatch JSONL logs and estimates paid frontier cost avoided when a baseline is supplied.

Current benchmark status from inspected reports:

- `UPDATED_PROGRESS_REPORT.md` says the project is ready for controlled token-savings testing, but not for claiming real-world savings.
- `REPORT_DISPATCH_RELIABILITY.md` records a partial Step 1 experiment: one 7B failure and two 7B successes across three tasks, with the manual frontier-baseline arm not yet run.
- No authoritative baseline CSV was confirmed in the active intake. Therefore savings claims remain unvalidated.

## Known Limits Checked

Software edge cases with direct tests or harness coverage:

- phase gate signals are phase-aware and `VERIFY_FAILED` regresses only in verify phase.
- URL normalization handles blank, host-only, and `/v1`-suffixed Ollama endpoints.
- dispatch target inference rejects ambiguous multi-target apply requests.
- snapshot restore removes a failed newly created file and rewrites a failed existing file.
- verify command failure returns a failure result rather than panicking.
- non-apply mode does not claim model-intended files as actual changed files.
- hashline parsing covers replace, delete, delete range, insert, and heredoc replacement cases.
- repo-map parsing includes Python files and avoids duplicate import edges from unrelated symbols.

## Regression Tests

Regression coverage is embedded in unit tests and experiment tasks:

- `experiments/tasks/03_fix_off_by_one/setup.sh` intentionally creates a moving-average off-by-one bug and verifies the last valid window.
- `experiments/tasks/01_string_helper/setup.sh` includes a trailing-newline preservation test, a known failure mode in `REPORT_DISPATCH_RELIABILITY.md`.
- dispatch rollback tests in `src/dispatch.rs` and `src/tools.rs` protect against failed local work becoming dirty workspace state.
- `scripts/dispatch_cost_report.py --logs-dir target/no-dispatches` is a CI regression check for empty-log behavior.

## Comparison With External Results

No published physics or external scientific results are reproduced.

The intended external comparison is operational:

- Direct frontier implementation arm versus Awl-assisted arm.
- Baseline data must be manually recorded in `experiments/results/baseline.csv` using `task_id`, `frontier_tokens`, `frontier_pass`, and `wall_ms`.
- Without that baseline, `experiments/tally.py` cannot establish token savings.

## Error Analysis Methodology

Current error-analysis mechanisms:

- Dispatch JSONL logs record events such as model response validation, file writes, verify pass/fail, verify command errors, rollback, and formatting/schema failures.
- `scripts/dispatch_cost_report.py` groups known error and success event names and reports success/failure counts, apply counts, local worker tokens, and estimated paid cost avoided.
- `REPORT_DISPATCH_RELIABILITY.md` analyzes model-capability failure versus retry policy, identifying repeated verify failure with the same 7B model as low-yield local retry.
- `UPDATED_PROGRESS_REPORT.md` separates "ready for controlled testing" from "proven real-world savings," which is the correct evidentiary boundary.

Open methodology gaps:

- `experiments/tally.py` still uses blended `--cost-per-mtok`; `REPORT_DISPATCH_RELIABILITY.md` recommends separate input/output rates.
- `REPORT_DISPATCH_RELIABILITY.md` recommends a richer failure taxonomy, per-dispatch model override, and separate input/output cost telemetry; these are not fully reflected in the inspected scripts.
- The Step 1 experiment has only three tasks and no confirmed direct-frontier baseline.
- Optional `scripts/dispatch_eval.sh` depends on Ollama and is not run by default in CI.
- No property tests or fuzz tests were found for shell validation, path containment, JSON schema handling, or hashline edit parsing.

## Validation Verdict

Awl has credible software validation for its core local-worker safety claims: formatting, linting, unit tests, CI on Linux/macOS, snapshot/verify/rollback behavior, and compact dispatch logging. It does not contain physics validation artifacts, and it does not yet provide completed experimental evidence for the broader claim that Awl saves paid frontier tokens in real workflows. That broader claim remains provisional until the A/B baseline is collected and analyzed with the stated pass/fail thresholds.

---

_Validation map generated from inspected repository artifacts and local checks on 2026-05-01._
