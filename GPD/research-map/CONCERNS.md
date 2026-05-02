# Research Gaps and Open Issues

**Analysis Date:** 2026-05-01
**Focus:** status
**Scope note:** This is a software-engineering research project, not a physics derivation. Physics-specific categories below are interpreted as engineering equivalents: untested regimes, uncontrolled approximations, fragile assumptions, stale branches, and missing validation.

## Active Reference Context

- Active Reference Registry: none confirmed in `GPD/state.json.project_contract.references`.
- Must-read references: none confirmed.
- Prior outputs and known-good baselines: none confirmed by the machine-readable intake.
- User-asserted anchors and gaps: none supplied.
- Stable knowledge documents: none supplied as runtime-active.
- Project contract: missing. `GPD/state.json` is absent and `GPD/state.json.lock` is an empty file, so no convention lock or authoritative project contract could be validated.

## Evidence Sources Inspected

- Local project reports: `HANDOFF_TO_GPD.md`, `REPORT_DISPATCH_RELIABILITY.md`, `UPDATED_PROGRESS_REPORT.md`, `experiments/README.md`.
- Source and tooling: `src/dispatch.rs`, `src/defaults.rs`, `src/main.rs`, `src/mcp_server.rs`, `src/safety.rs`, `src/llm_io.rs`, `src/repomap.rs`, `scripts/dispatch_cost_report.py`, `experiments/tally.py`, `experiments/run_awl_arm.sh`, `experiments/tasks/*/task.json`.
- Experiment artifacts: `experiments/results/awl_arm.jsonl`, `experiments/results/01_string_helper.json`, `experiments/results/02_validate_input.json`, `experiments/results/03_fix_off_by_one.json`, `experiments/sandbox/*`.
- Repository metadata: `.github/workflows/ci.yml`, `.github/ISSUE_TEMPLATE/*`, `.github/PULL_REQUEST_TEMPLATE.md`, branch and PR state via `git branch --all --verbose --no-abbrev` and `gh pr list`.
- Negative searches: no notebooks were found (`*.ipynb`, `*.nb`, `*.qmd`, `*.Rmd`); no substantive commented-out code blocks were found by a broad commented-code scan; GitHub issues returned an empty list.
- Verification run during this map: `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `python3 scripts/dispatch_cost_report.py --logs-dir target/no-dispatches --json`, and `python3 experiments/tally.py`.

## Executive Status

Awl's central research question remains unresolved: whether bounded local Ollama dispatch measurably saves frontier-model tokens in real Claude/Codex workflows. The local dispatch machinery is functional enough to test, and current Rust quality gates are passing, but the controlled savings experiment is incomplete and cannot support a positive or negative cost claim yet.

The current Step 1 local arm has only three tasks in `experiments/results/awl_arm.jsonl`: `01_string_helper` failed after two attempts, while `02_validate_input` and `03_fix_off_by_one` passed on one attempt. `python3 experiments/tally.py` reports a 2/3 Awl pass rate and 8601 local worker tokens, but there is no `experiments/results/baseline.csv`, so token reduction against a frontier-only baseline is unknown.

## Known Issues

**Deterministic 7B failure in the local arm**
- What exists: `experiments/results/01_string_helper.json` records `status: "error"`, `checks_passed: false`, `attempts: 2`, and two failures of `test_preserves_trailing_newline`.
- What's missing: a policy and measurement path for deciding when 7B is too weak and when 14B is worth the extra local latency.
- Files: `experiments/results/01_string_helper.json`, `experiments/sandbox/01/test_textops.py`, `REPORT_DISPATCH_RELIABILITY.md`.
- Impact: an easy bounded Python task can consume local tokens and still require frontier recovery.
- Priority: Critical.

**Verified apply retries still default to two attempts**
- What exists: `src/dispatch.rs` still sets `let default = if apply && has_verify { 2 } else { 1 };` in `effective_max_attempts`.
- What's missing: the resolved retry-policy change described in `REPORT_DISPATCH_RELIABILITY.md`: apply+verify should default to one attempt, with caller opt-in for more.
- Files: `src/dispatch.rs`.
- Impact: same-model verify retries can repeat deterministic capability failures and inflate local-token cost.
- Priority: Critical.

**Experiment task specs override the intended retry-policy change**
- What exists: `experiments/tasks/01_string_helper/task.json` and `experiments/tasks/02_validate_input/task.json` set `max_attempts: 2`; `experiments/tasks/03_fix_off_by_one/task.json` sets `max_attempts: 3`.
- What's missing: a decision to remove, reduce, or explicitly justify per-task retry overrides before rerunning Step 1.
- Files: `experiments/tasks/*/task.json`, `experiments/run_awl_arm.sh`, `src/dispatch.rs`.
- Impact: even after changing the source default, the current experiment pack would continue to use extra retries.
- Priority: Critical.

**Per-dispatch model override is absent**
- What exists: model selection still flows through level defaults such as `DEFAULT_IMPLEMENTATION_MODEL = "qwen2.5-coder:7b-instruct-q4_K_M"` in `src/defaults.rs`.
- What's missing: `model: Option<String>` through `DispatchOptions`, task JSON, CLI, MCP schema, and `AWL_MODEL_OVERRIDE` passthrough for `experiments/run_awl_arm.sh`.
- Files: `src/dispatch.rs`, `src/defaults.rs`, `src/main.rs`, `src/mcp_server.rs`, `experiments/run_awl_arm.sh`.
- Impact: the planned 7B-only versus 14B-only Step 1 sweep is blocked without environment-level workarounds.
- Priority: Critical.

**Failure taxonomy is not first-class**
- What exists: `scripts/dispatch_cost_report.py` groups raw event names into success and failure sets.
- What's missing: explicit `failure_category` values such as `format`, `schema`, `preflight`, `verify`, `timeout`, `network`, and `unknown` in dispatch outputs and cost reports.
- Files: `src/dispatch.rs`, `scripts/dispatch_cost_report.py`.
- Impact: experiment results cannot yet explain which failures should trigger model opt-up, frontier takeover, or tooling fixes.
- Priority: High.

**Cost reporting still uses blended token prices**
- What exists: `experiments/tally.py` accepts `--cost-per-mtok`; `scripts/dispatch_cost_report.py` accepts `--frontier-cost-per-mtok`.
- What's missing: split input/output pricing using `prompt_tokens` and `completion_tokens`.
- Files: `experiments/tally.py`, `scripts/dispatch_cost_report.py`, `experiments/README.md`.
- Impact: savings estimates can be materially wrong when output-heavy and input-heavy workflows have different rates.
- Priority: High.

**License metadata conflict**
- What exists: `Cargo.toml`, `LICENSE`, and `README.md` say MIT; `HANDOFF_TO_GPD.md` labels the license as AGPL-3.0 while also pointing to `LICENSE`.
- What's missing: reconciliation of the handoff document with repository metadata.
- Files: `Cargo.toml`, `LICENSE`, `README.md`, `HANDOFF_TO_GPD.md`.
- Impact: release and compliance ambiguity; not a token-savings blocker but should be fixed before public claims.
- Priority: Medium.

## Theoretical and Methodology Gaps

**No frontier baseline**
- Current status: `experiments/results/baseline.csv` was not found.
- Why it matters: the experiment cannot answer whether Awl saves paid frontier tokens without a direct frontier-only comparison.
- Files: `experiments/README.md`, `experiments/results/`.
- Priority: Critical.

**Task pack is too small and too narrow**
- Current status: only three tasks exist, all Python. The reports call for at least ten mixed tasks, including Python and Rust, write-from-scratch, edit-existing, and context-paths-required cases.
- What could go wrong: a small Python-only pack can overfit task selection and produce a non-generalizable pass rate.
- Files: `experiments/tasks/`, `HANDOFF_TO_GPD.md`, `REPORT_DISPATCH_RELIABILITY.md`.
- Priority: High.

**Frontier overhead is underspecified**
- Current status: baseline instructions mention direct-frontier tokens, but the assisted-arm protocol does not yet fully account for packaging the Awl request, reading compact results, deciding acceptance, and recovering from failures.
- What could go wrong: local worker success may not translate to net paid-token savings once frontier orchestration overhead is counted.
- Files: `experiments/README.md`, `UPDATED_PROGRESS_REPORT.md`.
- Priority: High.

**Model-selection hypothesis is unvalidated**
- Current status: reports resolve "no auto-escalation"; the frontier should pick 7B or 14B upfront.
- Missing validation: static 7B-only and 14B-only sweeps are necessary, but they do not prove that a frontier coordinator can predict per-task opt-up decisions.
- Files: `REPORT_DISPATCH_RELIABILITY.md`, `experiments/run_awl_arm.sh`.
- Priority: Medium.

**Verify command is the only correctness oracle**
- Current status: apply mode trusts `verify_command` exit status; tasks without strong local tests can pass wrong code.
- What could go wrong: weak tests could make Awl look successful while shifting semantic review back to the frontier.
- Files: `src/dispatch.rs`, `experiments/tasks/*/setup.sh`.
- Priority: High.

## Missing Validation

**No full mocked dispatch integration test**
- What to verify: stdin JSON -> model response parsing -> apply -> verify -> rollback -> stdout schema.
- Current status: unit tests cover many helpers, and `cargo test --workspace` passed with 56 tests, but CI does not exercise the full dispatch path with a mocked Ollama response.
- Files to modify: likely `src/dispatch.rs` and a new test harness.
- Priority: High.

**No HTTP/model mock coverage**
- What to verify: `/chat/completions` handling, response-format behavior, API errors, network failures, malformed usage payloads.
- Current status: dispatch eval is optional in CI behind `AWL_RUN_DISPATCH_EVAL`; normal CI does not require Ollama.
- Files: `src/dispatch.rs`, `.github/workflows/ci.yml`.
- Priority: Medium.

**Security-critical shell validation lacks focused tests**
- What to verify: path containment, command allowlisting, forbidden fragments, cargo/git subcommand restrictions, traversal cases, and pipeline parsing.
- Current status: `src/safety.rs` implements these checks, but no local test module was found in that file.
- Files: `src/safety.rs`, `src/dispatch.rs`.
- Priority: High.

**Structured-output recovery lacks focused tests**
- What to verify: code-fence stripping, malformed JSON with embedded control characters, escaped strings, nested quotes, and non-fenced text.
- Current status: `src/llm_io.rs` has no tests despite being on the local-model response recovery path.
- Files: `src/llm_io.rs`, `src/dispatch.rs`.
- Priority: Medium.

**Experiment tooling has only shallow CI coverage**
- What to verify: non-empty dispatch log fixtures, failure category rollups, split input/output cost calculations, baseline CSV validation, and mismatch handling between Awl and baseline task IDs.
- Current status: CI runs `python3 scripts/dispatch_cost_report.py --logs-dir target/no-dispatches`; `experiments/tally.py` is not fixture-tested.
- Files: `.github/workflows/ci.yml`, `scripts/dispatch_cost_report.py`, `experiments/tally.py`.
- Priority: Medium.

## Numerical and Computational Concerns

**Avoidable local-token burn from same-model retry**
- Problem: `01_string_helper` used 4264 total local tokens across two failed attempts; the second attempt reproduced the same trailing-newline failure class.
- Files: `experiments/results/01_string_helper.json`, `src/dispatch.rs`.
- Resolution: reduce verified apply default to one attempt and let callers explicitly request more.

**14B latency and pass rate are unmeasured**
- Problem: reports estimate 14B may take 30-60 seconds, but no 14B Step 1 run exists.
- Files: `REPORT_DISPATCH_RELIABILITY.md`, `experiments/run_awl_arm.sh`.
- Resolution: implement model override, then run separate 7B-only and 14B-only result files.

**Verify timeout is hardcoded**
- Problem: `VERIFY_TIMEOUT_MS` is fixed at 120000 in `src/dispatch.rs`.
- Symptoms: Rust compile-and-test tasks on slower machines could fail as timeouts rather than correctness failures.
- Resolution: defer unless observed, but classify timeout separately if failure taxonomy is added.

**Context and failure truncation can hide useful evidence**
- Problem: dispatch uses fixed caps for context files, total context, return chars, and failure issue chars.
- Files: `src/dispatch.rs`.
- Symptoms: frontier recovery may need to open dispatch logs, reducing the intended compact-result savings.
- Resolution: measure how often compact outputs are insufficient during Step 1.

## Stale or Dead Content

**Open dependency PRs and local review branches**
- What exists: GitHub PRs #15, #16, and #17 are open dependency bumps for `reqwest`, `tree-sitter`, and `tokio`. Multiple local `review-*` and `pr-*` branches remain; several are behind `main` or correspond to merged/closed dependency work.
- Files/refs: local git branches, `gh pr list`.
- Risk: branch clutter can confuse which dependency state is authoritative.
- Action: audit after the current research-map task; delete only with explicit user approval.

**Deleted tracked research-map file state was present earlier in the working tree**
- What exists: initial `git status --short --branch` during this task showed tracked `GPD/research-map/*.md` files as deleted and `GPD/state.json.lock` untracked. The directory later contained the other map documents again, but `CONCERNS.md` was still the requested scoped output.
- Files: `GPD/research-map/`, `GPD/state.json.lock`.
- Risk: map artifacts may be mid-regeneration or manually edited; do not infer canonical project state from them.
- Action: verify artifact state at the orchestrator level.

**Intentional BUG marker in experiment fixture**
- What exists: `experiments/tasks/03_fix_off_by_one/setup.sh` contains a `# BUG:` comment to seed the benchmark task.
- Risk: low; this is intentional fixture content, not production code debt.
- Action: keep unless task definitions are revised.

## TODO/FIXME/HACK/XXX and Notebook Scan

- `TODO`, `FIXME`, `HACK`, and `XXX` were not found as substantive project debt markers in source or notes.
- The only `BUG` marker found is the intentional benchmark seed in `experiments/tasks/03_fix_off_by_one/setup.sh`.
- GitHub issues are currently empty.
- No Jupyter, Mathematica, Quarto, or R Markdown notebooks were found, so no notebook error outputs could be inspected.
- A broad commented-out-code scan did not surface substantive disabled code blocks.

## Priority Ranking

**Critical (blocks correctness or the main research claim):**
1. Verified apply retry default still permits repeated same-model failures.
2. Current experiment task specs explicitly request extra retries.
3. Per-dispatch model override is absent, blocking 7B/14B comparison.
4. No frontier baseline exists, so token savings are unmeasured.

**High (blocks completeness or interpretability):**
1. Failure taxonomy is missing from dispatch results and cost reports.
2. Cost reporting uses blended token pricing instead of split input/output rates.
3. Task pack is only three Python tasks and is not representative.
4. Frontier overhead accounting is underspecified.
5. Security-critical command validation and full dispatch integration lack focused tests.

**Medium (improves quality and release confidence):**
1. License metadata conflict between `HANDOFF_TO_GPD.md` and repository metadata.
2. 14B latency/pass-rate estimates are unmeasured.
3. Experiment scripts lack fixture tests.
4. Hardcoded verify timeout may become a false-negative source as Rust tasks are added.
5. Branch and dependency-review clutter needs cleanup after explicit approval.

**Low (nice to have):**
1. Improve compact-result sufficiency measurements.
2. Add more explicit validation for structured-output sanitization edge cases.
3. Document when to skip Awl for one-line or architecture-heavy tasks.

---

*Gap analysis: 2026-05-01*
