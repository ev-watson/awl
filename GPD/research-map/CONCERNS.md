# CONCERNS.md - Awl Project Status and Open Questions

**Project:** Awl, a Rust CLI and MCP server for bounded local model dispatch
**Analysis date:** 2026-05-01
**Focus:** status
**Scope note:** This map adapts GPD "status" categories to a software engineering research project. No physics content is inferred.

## Executive Status

Awl's current research question is still the product hypothesis in `HANDOFF_TO_GPD.md` line 11 and anchor `ANC-001`: bounded local execution should measurably save frontier tokens on net. That remains unproven. The partial Step 1 run in `experiments/results/awl_arm.jsonl` and `REPORT_DISPATCH_RELIABILITY.md` shows the dispatch pipeline can run end-to-end, but it also exposes a deterministic 7B failure and a wasted same-model retry.

The experiment is incomplete. `experiments/results/awl_arm.jsonl` has 3 local-arm records, but no `experiments/results/baseline.csv` is present. Without the frontier baseline arm, the success thresholds in `experiments/README.md` lines 89-95 and `UPDATED_PROGRESS_REPORT.md` lines 266-272 cannot be evaluated. This preserves `ANC-002` and `ANC-006`: Step 1 is halted, not completed.

`GPD/state.json` is missing from the workspace; only `GPD/state.json.lock` is present. Per the user-supplied context, the project contract is not authoritative, so this file relies on inspected source and the active anchors in `GPD/research-map/REFERENCES.md`.

## Carry-Forward Anchor Coverage

| Anchor | Status concern |
|---|---|
| `ANC-001` | Product hypothesis from `HANDOFF_TO_GPD.md` line 11 is open. No net frontier-token savings result exists. |
| `ANC-002` | Success thresholds from `experiments/README.md` and `UPDATED_PROGRESS_REPORT.md` remain the gate: >=25-40% token reduction and >=60-70% Awl pass rate. Current evidence is too small and lacks baseline. |
| `ANC-003` | Dispatch v2 JSON contract in `src/dispatch.rs` remains load-bearing. Adding fields such as `model` or `failure_category` must be additive and tested. |
| `ANC-004` | Rollback invariant in `src/dispatch.rs` is central to bounded failure cost. It is unit-tested, but not covered by a full mocked integration test. |
| `ANC-005` | Cost rates must remain split as $5/MTok input and $25/MTok output, not blended. Current tooling still uses blended flags in `experiments/tally.py` and `scripts/dispatch_cost_report.py`. |
| `ANC-006` | Partial Step 1 result is preserved: `01_string_helper` failed twice; `02_validate_input` and `03_fix_off_by_one` passed once. This is signal, not completion. |
| `ANC-007` | Retry policy decision is not implemented in source. `src/dispatch.rs` still defaults apply+verify to 2 attempts, and existing experiment task specs explicitly set `max_attempts`. |
| `ANC-008` | Model override decision is not implemented. `src/dispatch.rs`, `src/main.rs`, `src/mcp_server.rs`, and `experiments/run_awl_arm.sh` still lack per-dispatch model override. |
| `ANC-009` | The 5-item patch list remains the immediate work queue. Items 1-4 should ship before Step 1 resumes. |
| `ANC-010` | Branch protection constraints from `HANDOFF_TO_GPD.md` lines 33-41 remain active process constraints: PRs only, strict CI, do not weaken lint gates. |
| `ANC-011` | Tokenizer inflation remains an unquantified cost-model risk for real Awl dispatch payloads. |
| `ANC-012` | Pre-v2 failure modes from `UPDATED_PROGRESS_REPORT.md` remain historical controls for regression checks. |

## Deterministic Failures and Known Issues

1. **Deterministic local-model failure in Step 1.**
   `experiments/results/awl_arm.jsonl` records `01_string_helper` as `status: "error"`, `checks_passed: false`, `attempts: 2`, and `total_tokens: 4264`. The failed oracle is `test_preserves_trailing_newline` in `experiments/sandbox/01/test_textops.py`, surfaced in `experiments/results/01_string_helper.json`. `REPORT_DISPATCH_RELIABILITY.md` lines 13-17 and 106-132 classify this as a 7B-q4 capability gap and a wasted retry, not a transient flake.

2. **Retry policy decision has not reached the code.**
   `REPORT_DISPATCH_RELIABILITY.md` lines 73-79 and 146-150 say the user confirmed default apply+verify retries should drop from 2 to 1 with no auto-escalation. The source still has `let default = if apply && has_verify { 2 } else { 1 };` in `src/dispatch.rs` lines 1110-1112.

3. **Experiment task specs will override the retry-policy fix unless patched.**
   Even if `src/dispatch.rs` changes its default to 1, `run_capture()` uses `options.max_attempts.or(spec.max_attempts)` in `src/dispatch.rs` lines 514-518. Current task specs set explicit retries: `experiments/tasks/01_string_helper/task.json` line 13 sets `max_attempts: 2`, `experiments/tasks/02_validate_input/task.json` line 14 sets `max_attempts: 2`, and `experiments/tasks/03_fix_off_by_one/task.json` line 15 sets `max_attempts: 3`. The patch list must decide whether Step 1 task specs should remove or reduce these explicit values.

4. **Per-dispatch model override is absent.**
   `DispatchOptions` in `src/dispatch.rs` lines 30-41 has no `model` field, `TaskSpec` in `src/dispatch.rs` lines 184-211 has no `model` field, and model selection still calls `defaults::configured_model_for_level(options.level)` in `src/dispatch.rs` lines 571-575. The CLI dispatch parser in `src/main.rs` lines 108-212 has no `--model` flag, and the MCP dispatch tool schema in `src/mcp_server.rs` lines 60-72 has no `model` property. This blocks the 7B-only versus 14B-only sweep in `REPORT_DISPATCH_RELIABILITY.md` lines 138-142.

5. **Failure taxonomy is absent from output and telemetry summaries.**
   `apply_result()` in `src/dispatch.rs` lines 1212-1239 and `error_result()` in `src/dispatch.rs` lines 1241-1260 do not include `failure_category`. `scripts/dispatch_cost_report.py` lines 15-24 only groups raw event names into `ERROR_EVENTS` and `SUCCESS_EVENTS`, so it cannot distinguish format, schema, preflight, verify, timeout, network, and unknown failures as proposed in `REPORT_DISPATCH_RELIABILITY.md` lines 81-101.

6. **Cost reporting still uses blended rates.**
   `experiments/tally.py` lines 65-69 and 176-181 use a single `cost_per_mtok`. `scripts/dispatch_cost_report.py` lines 64-69 and 164-177 use a single `frontier_cost_per_mtok`. This conflicts with `ANC-005` and the resolved decision in `REPORT_DISPATCH_RELIABILITY.md` lines 146-151 to track Opus 4.7 input and output costs separately.

7. **Documentation and metadata disagree on license.**
   `HANDOFF_TO_GPD.md` line 15 says to see `LICENSE` and labels it `AGPL-3.0`, while `Cargo.toml` line 5, `LICENSE` line 1, and `README.md` lines 246-248 say MIT. This is not a token-savings blocker, but it is a release/compliance ambiguity and the existing `GPD/research-map/CONVENTIONS.md` also flags it.

## Stale or Incomplete Experiment State

- `experiments/results/awl_arm.jsonl` exists and contains exactly 3 local-arm records. `experiments/results/01_string_helper.json`, `experiments/results/02_validate_input.json`, and `experiments/results/03_fix_off_by_one.json` also exist.
- `experiments/results/baseline.csv` is missing. The manual baseline procedure is documented in `experiments/README.md` lines 48-74, but no baseline file was found under `experiments/results/`.
- The task pack has only 3 task directories under `experiments/tasks/`, while `HANDOFF_TO_GPD.md` lines 58 and 136-143 call for >=10 mixed tasks.
- The existing tasks are all Python. `experiments/tasks/03_fix_off_by_one/task.json` line 12 uses `context_paths`, but there are no Rust generation tasks and no compile-and-test Rust verification tasks.
- The Awl arm has only been run with `qwen2.5-coder:7b-instruct-q4_K_M`, as recorded in `experiments/results/awl_arm.jsonl`. No 14B run exists.
- The current `experiments/run_awl_arm.sh` lines 18-20 supports `AWL_BIN` and `AWL_LEVEL`, but not the planned `AWL_MODEL_OVERRIDE`.
- `experiments/tally.py` cannot currently produce the planned split-rate command from `REPORT_DISPATCH_RELIABILITY.md` line 141 because it lacks `--input-cost-per-mtok` and `--output-cost-per-mtok`.

## TODO/FIXME/HACK/XXX Scan

No `TODO`, `FIXME`, `HACK`, or `XXX` comments were found in the inspected repository with `rg -n "TODO|FIXME|HACK|XXX" -g '!target' -g '!GPD/research-map/CONCERNS.md' .`. This is a useful negative result, but it does not mean the project lacks open issues; the active concerns are encoded in reports, experiments, and source behavior rather than inline comments.

## Fragile Assumptions

1. **The local pass rate target may hide task-selection bias.**
   The thresholds in `experiments/README.md` lines 89-99 and `UPDATED_PROGRESS_REPORT.md` lines 266-272 are necessary but not sufficient. If the 10+ task pack overrepresents simple, bounded Python helpers, Awl could appear to pass while not predicting real Claude/Codex workflows.

2. **Frontier overhead is unmeasured.**
   The product hypothesis in `HANDOFF_TO_GPD.md` line 11 is about net frontier-token savings, but `experiments/README.md` lines 72-73 only instructs baseline operators to exclude unrelated context. It does not yet measure the paid tokens spent packaging Awl dispatches, reading compact results, deciding whether to accept them, or recovering from failures.

3. **No auto-escalation means model selection becomes a frontier-side prediction problem.**
   The resolved design in `REPORT_DISPATCH_RELIABILITY.md` lines 42-49 and 146-150 says the frontier chooses 7B versus 14B upfront. Step 1 can compare static 7B-only and 14B-only configurations, but it will not by itself validate whether the frontier can predict which individual tasks deserve 14B.

4. **Verify commands are the only correctness oracle.**
   `src/dispatch.rs` lines 1160-1209 treats `verify_command` exit code as the apply-mode oracle. This is pragmatic, but weak or incomplete tests can admit wrong local code. Tasks without a verify command enter the weaker apply-without-verify regime.

5. **Python safety and import checks lag Rust safety checks.**
   Rust hallucinated crate imports are checked by `unresolved_crate_imports()` tests in `src/dispatch.rs` lines 1562-1599. Python import hallucination has no analogous preflight, matching the deferral in `REPORT_DISPATCH_RELIABILITY.md` lines 88-92.

6. **The current shell validation surface is security-critical and undertested.**
   `src/safety.rs` lines 75-118 validates verify commands using allowlisted programs and forbidden fragments, but there is no `#[cfg(test)]` module in `src/safety.rs`. This matters because `run_verify_command()` executes through `bash -lc` in `src/dispatch.rs` lines 1160-1171.

7. **Hardcoded verify timeout can create false negatives.**
   `VERIFY_TIMEOUT_MS` is fixed at 120000 in `src/dispatch.rs` line 28, and timeout failure is represented as ordinary `success: false` output in `src/dispatch.rs` lines 1191-1196. Rust compile-and-test tasks could exceed this on slower machines, especially as the task pack expands.

8. **Repository-map and context budget limits may clip important evidence.**
   `src/dispatch.rs` lines 23-27 caps returned text, context per file, total context, and failure issues. These bounds are necessary for compactness, but they can hide details that the local model or frontier needs for debugging and selection heuristics.

## Missing Validation

1. **No full dispatch integration test.**
   There is no `tests/` directory. Unit tests cover many helpers, but no CI test exercises stdin JSON through model response parsing, apply, verify, rollback, and stdout schema with a mocked model.

2. **No HTTP/model mocking.**
   `send_request()` and the Ollama OpenAI-compatible `/chat/completions` path are not tested in CI without a running Ollama instance. `.github/workflows/ci.yml` lines 41-48 keeps dispatch eval optional behind `AWL_RUN_DISPATCH_EVAL`.

3. **No `src/safety.rs` tests.**
   Path containment, command allowlisting, forbidden fragment handling, cargo/git subcommand restrictions, and traversal cases are untested despite being security-critical.

4. **No `src/llm_io.rs` tests.**
   `strip_code_fences()` and `sanitize_json_strings()` in `src/llm_io.rs` lines 3-50 are on the structured-output recovery hot path, but they have no local tests for malformed JSON, escaped control characters, nested quotes, or non-fenced text.

5. **No functional tests for experiment tooling.**
   `.github/workflows/ci.yml` lines 38-39 smoke-tests `scripts/dispatch_cost_report.py` on an empty logs directory. There is no fixture test for non-empty dispatch logs, failure categories, `experiments/tally.py`, or baseline CSV parsing.

6. **No automated baseline validation.**
   The manual frontier-baseline procedure in `experiments/README.md` lines 48-74 is vulnerable to inconsistent token accounting, omitted retries, and apples-to-oranges context inclusion. There is no schema check for `experiments/results/baseline.csv`.

7. **Current map has minor drift.**
   `GPD/research-map/VALIDATION.md` still lists `src/hashline.rs` as lacking tests, but current `src/hashline.rs` lines 375-482 contains 10 tests. Because update mode is missing-only, this document records the drift but does not edit `GPD/research-map/VALIDATION.md`.

## Computational Bottlenecks

- 7B-q4 local dispatch wall time in `experiments/results/awl_arm.jsonl` is roughly 15-30 seconds per task. The failed `01_string_helper` run took 29686 ms and used 4264 local tokens across two attempts.
- 14B wall time is unmeasured. `REPORT_DISPATCH_RELIABILITY.md` lines 30-36 estimates 30-60 seconds by reputation, but Step 1 needs direct measurement through `experiments/run_awl_arm.sh`.
- Same-model verify retry is the largest observed avoidable local-token bottleneck. The first failed `01_string_helper` attempt used 1978 tokens and the second used 2286 tokens in `experiments/results/01_string_helper.json` lines 24-35.
- The frontier-side bottleneck is not measured at all: number of paid turns, packaging tokens, review tokens, and recovery tokens are required by `UPDATED_PROGRESS_REPORT.md` lines 253-264 but absent from the current result files.

## Risks to the Token-Savings Claim

1. **No baseline means no savings result.**
   Current local-arm data cannot establish any token reduction because `experiments/results/baseline.csv` is missing.

2. **Failures may erase savings.**
   The `01_string_helper` result shows an easy bounded task can fail deterministically and consume extra local attempts. If the frontier must then solve the task directly, packaging plus review plus direct recovery could exceed the baseline cost.

3. **Successful local tasks may be too cheap for the frontier anyway.**
   If tasks that 7B handles reliably are also tasks a frontier model can solve in very few paid tokens, the delegation overhead may dominate even at high local pass rates.

4. **Blended cost reporting can misstate the result.**
   Because Opus 4.7 output tokens cost 5x input tokens per `ANC-005`, any blended-rate tally can invert or exaggerate the cost conclusion for output-heavy baselines.

5. **Tokenizer inflation is unvalidated for Awl payloads.**
   `ANC-011` notes up to about 35% token inflation versus prior Claude tokenizers, but no measurement exists for Awl dispatch prompts, compact result reviews, or direct-frontier baseline tasks.

6. **Task-pack representativeness is unresolved.**
   The current pack has 3 tasks, all Python. A defensible result needs Python and Rust, write-from-scratch and edit-existing tasks, meaningful `context_paths`, and tasks near the 7B/14B boundary without tuning them to make 7B pass.

## Unresolved Design Questions

1. Should existing experiment `max_attempts` values in `experiments/tasks/*/task.json` be removed, reduced, or retained as explicit opt-ins after the default retry policy changes?
2. Should `model` be accepted only as a top-level dispatch option, only inside the JSON task spec, or both? Current architecture has both `DispatchOptions` and `TaskSpec` input surfaces.
3. How should `failure_category` be represented in successful results, if at all? Additive output must preserve the dispatch v2 contract in `src/dispatch.rs`.
4. Should timeout be a distinct `failure_category` even though `run_verify_command()` currently returns timeout as a failed verify result?
5. What exact token accounting should define "frontier overhead" in the Awl-assisted arm? The direct baseline instructions exist, but the assisted-arm paid-token measurement protocol is underspecified.
6. What is the minimum task size or complexity threshold for dispatch? `UPDATED_PROGRESS_REPORT.md` lines 201-206 already warns one-line edits may lose.
7. Should Step 1 compare only static 7B-only and 14B-only configurations, or also a post-Step-1 heuristic run where the frontier chooses per task? The resolved decision excludes auto-escalation, but not a later heuristic study.
8. How should license metadata be reconciled between `HANDOFF_TO_GPD.md`, `Cargo.toml`, `LICENSE`, and `README.md` before public release claims?

## Next Checks

1. Patch retry policy in `src/dispatch.rs`, then explicitly decide what to do with `max_attempts` in `experiments/tasks/01_string_helper/task.json`, `experiments/tasks/02_validate_input/task.json`, and `experiments/tasks/03_fix_off_by_one/task.json`.
2. Add per-dispatch `model` override through `src/dispatch.rs`, `src/main.rs`, `src/mcp_server.rs`, and `experiments/run_awl_arm.sh`; include tests for override beats level default and unset uses level default.
3. Add `failure_category` to dispatch outputs and aggregate it in `scripts/dispatch_cost_report.py`.
4. Replace blended cost flags with split input/output costs in `experiments/tally.py` and `scripts/dispatch_cost_report.py`.
5. Add fixture tests for `experiments/tally.py` and `scripts/dispatch_cost_report.py` before relying on their reports for Step 1.
6. Add focused unit tests for `src/safety.rs` and `src/llm_io.rs`.
7. Expand `experiments/tasks/` to >=10 tasks with Python, Rust, context-paths-required, write-from-scratch, and edit-existing cases.
8. Run Awl arm at 7B-only and 14B-only after model override lands, preserving separate outputs under `experiments/results/`.
9. Run the manual frontier-baseline arm and create `experiments/results/baseline.csv` with documented token accounting.
10. Run split-rate tally and compare against the `ANC-002` thresholds before declaring Step 1 positive or negative.
