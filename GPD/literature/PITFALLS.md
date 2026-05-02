---
template_version: 1
research_type: "Literature Survey - Open Problems and Pitfalls"
domain: "Awl frontier-token savings and bounded local coding delegation"
researched: "2026-05-01"
confidence: "HIGH for local artifact facts; MEDIUM for experiment-design implications"
---

# Known Pitfalls Research

**Domain:** Awl bounded local Ollama dispatch for frontier-token savings in Claude/Codex coding sessions
**Scope:** Existing Awl project artifacts and approved Step 1 savings contract
**Mode:** balanced

This is a software-engineering measurement project, not a physics derivation. "Literature" here means project-local research artifacts, experiment definitions, telemetry, source code, and CI configuration. The core risk is false progress: a result can look positive while failing to answer whether Awl saves paid frontier tokens without reliability regressions.

## Evidence Sources

- `GPD/research-map/CONCERNS.md`
- `REPORT_DISPATCH_RELIABILITY.md`
- `HANDOFF_TO_GPD.md`
- `UPDATED_PROGRESS_REPORT.md`
- `experiments/README.md`
- `experiments/tasks/01_string_helper/task.json`
- `experiments/tasks/02_validate_input/task.json`
- `experiments/tasks/03_fix_off_by_one/task.json`
- `experiments/results/awl_arm.jsonl`
- `experiments/results/01_string_helper.json`
- `src/dispatch.rs`
- `src/defaults.rs`
- `experiments/run_awl_arm.sh`
- `experiments/tally.py`
- `scripts/dispatch_cost_report.py`
- `.github/workflows/ci.yml`
- `src/safety.rs`
- `src/llm_io.rs`
- `Cargo.toml`, `LICENSE`, `README.md`, `HANDOFF_TO_GPD.md`

## Critical Pitfalls

### Pitfall 1: Treating Awl Pass Rate As Token-Savings Evidence

**What goes wrong:**
Step 1 can report a plausible Awl pass rate while still providing no evidence of paid frontier-token savings. The current local arm has 3 tasks, 2 passing and 1 failing, and `python3 experiments/tally.py` reports a 2/3 Awl pass rate and 8601 local tokens. But there is no `experiments/results/baseline.csv`, so the token reduction column is blank and aggregate token reduction is absent.

**Why it happens:**
Pass rate is easy to compute from `experiments/results/awl_arm.jsonl`; the frontier-only baseline is manual and depends on real Claude/Codex token accounting. That makes pass rate a tempting proxy for savings.

**How to avoid:**
Do not let Step 1 complete, pass, or support a public claim unless every Awl task has a matching frontier-baseline row with pass/fail, wall time, and token accounting. Treat local-only tallies as pipeline smoke tests only.

**Warning signs:**
- `experiments/results/baseline.csv` is missing or has fewer task IDs than `awl_arm.jsonl`.
- `experiments/tally.py` prints total Awl tokens but no total baseline tokens.
- Reports cite "67% pass rate" without a matching aggregate token-reduction percentage.

**Phase to address:**
Phase 2, Step 1 experiment execution and writeup.

**File-backed evidence:**
- `GPD/research-map/CONCERNS.md` says baseline CSV is absent and token reduction is unknown.
- `experiments/README.md` makes baseline CSV a manual required input.
- `experiments/results/awl_arm.jsonl` contains only local-arm records.

---

### Pitfall 2: Counting Local Worker Tokens But Omitting Frontier Coordination Overhead

**What goes wrong:**
Awl may appear to save tokens if the comparison subtracts local worker tokens from direct-frontier tokens, while ignoring frontier tokens spent to formulate the dispatch, read compact results, decide whether to accept them, and recover from failed dispatches.

**Why it happens:**
The harness records local prompt/completion tokens per dispatch, but the assisted frontier session's paid overhead is outside Awl's local JSONL unless the experimenter records it explicitly.

**How to avoid:**
Define the assisted-arm ledger before rerunning Step 1. For each task, record direct-frontier tokens and frontier-with-Awl coordinator tokens separately from local worker tokens. Count failure recovery, including any frontier inspection of logs or generated code.

**Warning signs:**
- Baseline rows include direct frontier tokens, but no assisted-arm frontier overhead exists.
- A failed Awl attempt is counted only as local tokens, with no paid recovery cost.
- The writeup claims "tokens avoided" from `dispatch_cost_report.py` estimates rather than real direct-frontier counts.

**Phase to address:**
Phase 2, Step 1 protocol and tally schema.

**File-backed evidence:**
- `experiments/README.md` defines `frontier_tokens` only for the direct frontier arm.
- `REPORT_DISPATCH_RELIABILITY.md` notes frontier overhead is unresolved and Step 1 was halted.
- `UPDATED_PROGRESS_REPORT.md` states the product question is paid-token savings including retries and failures.

---

### Pitfall 3: Retry-Policy Patch Looks Applied But Task JSON Still Forces Retries

**What goes wrong:**
Changing the source default from two verified apply attempts to one may not change Step 1 behavior if task specs continue to set `max_attempts`. The current tasks override the default: task 01 and 02 set `max_attempts: 2`; task 03 sets `max_attempts: 3`.

**Why it happens:**
`src/dispatch.rs` computes `options.max_attempts.or(spec.max_attempts)` before calling `effective_max_attempts`. A source default patch is bypassed whenever task JSON provides an explicit value.

**How to avoid:**
Audit and either remove or explicitly justify every `max_attempts` field in `experiments/tasks/*/task.json` before rerunning Step 1. Make the experiment config record the intended retry policy.

**Warning signs:**
- Source default says one attempt, but result JSON still reports `attempts: 2` or `attempts: 3`.
- New Step 1 data is compared against old data without noting retry-policy differences.
- `max_attempts` remains in task JSON as hidden experiment policy.

**Phase to address:**
Phase 1 reliability patch 1, then Phase 2 task-pack freeze.

**File-backed evidence:**
- `src/dispatch.rs` currently sets `let default = if apply && has_verify { 2 } else { 1 };`.
- `experiments/tasks/01_string_helper/task.json` sets `max_attempts: 2`.
- `experiments/tasks/02_validate_input/task.json` sets `max_attempts: 2`.
- `experiments/tasks/03_fix_off_by_one/task.json` sets `max_attempts: 3`.

---

### Pitfall 4: Same-Model Verify Retries Repeat Deterministic Capability Failures

**What goes wrong:**
A failed 7B dispatch can burn more local tokens without producing new information. In the current Step 1 data, `01_string_helper` failed the same trailing-newline test on both attempts and consumed 4264 local tokens.

**Why it happens:**
The local retry loop treats verify failures as potentially self-correctable, but the same small model may repeat the same blind spot even after failure feedback. This is different from format/schema retries, which are often cheap and high-yield.

**How to avoid:**
Default verified apply attempts to one. Keep format/schema/preflight retries separate. On verify failure, return the compact failure to the frontier and let the frontier choose whether to take over or dispatch once to a stronger model.

**Warning signs:**
- Repeated `open_issues` differ only by attempt number.
- The same unit test fails on all attempts.
- Local token totals grow while `files_changed` stays empty after rollback.

**Phase to address:**
Phase 1 reliability patch 1.

**File-backed evidence:**
- `REPORT_DISPATCH_RELIABILITY.md` identifies `01_string_helper` as a deterministic 7B failure.
- `experiments/results/01_string_helper.json` records two failed attempts on `test_preserves_trailing_newline`.
- `experiments/results/awl_arm.jsonl` records 4264 local tokens for the failed task.

---

### Pitfall 5: Task Pack Tuned To Make 7B Look Better Than It Is

**What goes wrong:**
A small, easy, Python-only task pack can overstate Awl's usefulness. The current pack has only three tasks: two write-from-scratch Python functions and one Python edit-existing task. Only task 03 uses `context_paths`; no Rust task is present even though Awl itself is a Rust project.

**Why it happens:**
Bounded tasks are necessary for local delegation, but "bounded" can drift into "cherry-picked." If tasks are modified after inspecting 7B failures, the experiment measures task tuning rather than general savings.

**How to avoid:**
Pre-register a larger mixed task pack before rerunning. Include Python and Rust, write-from-scratch, edit-existing, context-path-required tasks, and edge-case-sensitive tasks. Freeze task specs and tests before comparing 7B vs 14B vs baseline.

**Warning signs:**
- Fewer than 10 tasks.
- All tasks use the same language or only pure functions.
- Failing edge cases are removed instead of retained as signal.
- New tasks are added only after seeing which model wins.

**Phase to address:**
Phase 2, Step 1 task-pack expansion and freeze.

**File-backed evidence:**
- `experiments/tasks/*/task.json` shows only three Python tasks.
- `HANDOFF_TO_GPD.md` calls for at least 10 mixed tasks, including Python and Rust and context-path-required cases.
- `REPORT_DISPATCH_RELIABILITY.md` says Step 1 should resume only after patch items land and task pack expands.

---

### Pitfall 6: 7B/14B Sweep Blocked Or Made Non-Reproducible By Model Selection Plumbing

**What goes wrong:**
The approved Step 1 matrix requires separate 7B-only and 14B-only runs. Without per-dispatch model override, runs may rely on ambient config or environment changes, making results hard to reproduce and hard to attribute.

**Why it happens:**
Current `DispatchOptions` has no `model` field. Level 2 falls back to `DEFAULT_IMPLEMENTATION_MODEL`, and `experiments/run_awl_arm.sh` only supports `AWL_LEVEL`, not `AWL_MODEL_OVERRIDE`.

**How to avoid:**
Add `model: Option<String>` to the dispatch contract and plumb it through CLI, MCP schema, task JSON, and experiment driver. Record the model in each result row and use separate output files for 7B-only and 14B-only runs.

**Warning signs:**
- Result files rely on global config state not captured by the experiment artifact.
- The same `awl_arm.jsonl` mixes model configurations.
- 14B results are produced by changing default level mappings rather than explicit run configuration.

**Phase to address:**
Phase 1 reliability patch 2, then Phase 2 model sweep.

**File-backed evidence:**
- `src/dispatch.rs` `DispatchOptions` lacks a model override.
- `src/defaults.rs` sets `DEFAULT_IMPLEMENTATION_MODEL` to the 7B model.
- `experiments/run_awl_arm.sh` reads `AWL_LEVEL` but not `AWL_MODEL_OVERRIDE`.
- `REPORT_DISPATCH_RELIABILITY.md` requires per-dispatch model override before meaningful Step 1 scaling.

---

### Pitfall 7: Blended Token Pricing Masks Real Paid-Cost Effects

**What goes wrong:**
Savings estimates can be materially wrong if input and output tokens are priced differently but the report uses one blended `--cost-per-mtok` rate. Output-heavy failures and input-heavy context tasks have different paid-cost profiles.

**Why it happens:**
`experiments/tally.py` accepts one `--cost-per-mtok`, and `scripts/dispatch_cost_report.py` accepts one `--frontier-cost-per-mtok`. Yet Awl result records already preserve `prompt_tokens` and `completion_tokens`.

**How to avoid:**
Use split input/output token columns and split price flags for both local-arm reports and baseline rows. Do not publish a paid-cost estimate from blended pricing except as explicitly labeled exploratory arithmetic.

**Warning signs:**
- Reports cite dollars saved from `--cost-per-mtok`.
- Baseline schema has only `frontier_tokens`, not input/output split.
- Per-task savings are insensitive to completion length.

**Phase to address:**
Phase 1 reliability patch 4 and Phase 2 tally schema.

**File-backed evidence:**
- `experiments/tally.py` documents `frontier_tokens` as a single combined field and exposes `--cost-per-mtok`.
- `scripts/dispatch_cost_report.py` exposes `--frontier-cost-per-mtok`.
- `REPORT_DISPATCH_RELIABILITY.md` explicitly calls for split input/output pricing.

---

### Pitfall 8: Weak Verify Commands Create False Passing Awl Tasks

**What goes wrong:**
Awl can report `status: ok` and `checks_passed: true` for code that satisfies the visible test suite but is semantically wrong outside those tests. This would inflate pass rate and understate frontier review cost.

**Why it happens:**
Apply mode trusts `verify_command` exit status as the correctness oracle. The current task tests are small `unittest` suites. That is appropriate for smoke tests, but not enough to prove general correctness for benchmark claims.

**How to avoid:**
For each task, define a verification-strength rubric. Add edge cases, negative cases, and where useful property-style tests. For Rust tasks, include `cargo test` and targeted unit tests. Keep tests fixed before model comparison.

**Warning signs:**
- A task passes with only happy-path tests.
- Verify command checks importability but not behavior.
- The frontier still has to inspect full generated code after a passing dispatch.

**Phase to address:**
Phase 2 task-pack design and verification gate.

**File-backed evidence:**
- Task specs use `python3 -m unittest discover` as the verify command.
- `experiments/tasks/01_string_helper/setup.sh` includes a trailing-newline test that exposed the 7B failure; this shows edge tests matter.
- `GPD/research-map/CONCERNS.md` identifies verify command as the only correctness oracle.

---

### Pitfall 9: Missing Failure Taxonomy Blocks Reliability Diagnosis

**What goes wrong:**
Failures get lumped together, so the team cannot tell whether reliability patches reduced format errors, schema errors, preflight failures, verify failures, timeouts, or network/model failures. This blocks model-selection guidance and can lead to wrong fixes.

**Why it happens:**
`scripts/dispatch_cost_report.py` groups raw event names into broad success and error sets. Dispatch result JSON does not expose a first-class `failure_category`.

**How to avoid:**
Add `failure_category` to `apply_result` and `error_result`, with categories such as `format`, `schema`, `preflight`, `verify`, `timeout`, `network`, and `unknown`. Require tally/report output to summarize by category.

**Warning signs:**
- Reports say "failed" without distinguishing verify failure from transport failure.
- 14B is recommended for failures that were actually schema or preflight issues.
- CI or experiments regress but the cause category is ambiguous.

**Phase to address:**
Phase 1 reliability patch 3.

**File-backed evidence:**
- `REPORT_DISPATCH_RELIABILITY.md` lists failure taxonomy as a required robustness gap.
- `GPD/research-map/CONCERNS.md` says explicit `failure_category` values are missing.
- `scripts/dispatch_cost_report.py` currently uses `ERROR_EVENTS` and `SUCCESS_EVENTS` sets.

---

### Pitfall 10: CI Passes Without Exercising The Real Dispatch Path

**What goes wrong:**
CI can pass while dispatch apply/verify behavior, Ollama response parsing, rollback, timeout behavior, cost reporting with real logs, or experiment tally logic remains broken.

**Why it happens:**
Normal CI runs `cargo fmt --check`, clippy, `cargo test`, and `dispatch_cost_report.py` against an empty logs directory. The local dispatch eval is optional and gated by `AWL_RUN_DISPATCH_EVAL == '1'`; if Ollama is absent, it exits successfully.

**How to avoid:**
Add deterministic mocked integration tests for stdin JSON to model response to apply to verify to rollback to stdout schema. Add fixture tests for `experiments/tally.py` and non-empty dispatch logs. Keep live Ollama eval optional, but do not rely on it as the only end-to-end check.

**Warning signs:**
- PRs touch dispatch result schema but only unit tests run.
- `scripts/dispatch_cost_report.py --logs-dir target/no-dispatches` is the only telemetry check.
- `experiments/tally.py` has no fixture test despite being part of the claim path.

**Phase to address:**
Phase 1 reliability patches and any PR touching dispatch or experiment tooling.

**File-backed evidence:**
- `.github/workflows/ci.yml` shows optional dispatch eval and empty-log cost report check.
- `GPD/research-map/CONCERNS.md` calls out missing full mocked dispatch integration and shallow experiment-tooling CI coverage.
- `src/dispatch.rs` has unit tests, but no full mocked HTTP/model integration path is evident in the inspected test module.

---

### Pitfall 11: Security And Shell-Validation Changes Regress Without Focused Tests

**What goes wrong:**
Reliability patches can accidentally broaden command execution or path write behavior. That can create safety regressions or force future maintainers to weaken verification gates when tests become inconvenient.

**Why it happens:**
`src/safety.rs` is security-critical: it resolves write paths, constrains workspace access, and validates shell commands. It allowlists programs including write-capable tools such as `rm`, `mv`, and `cp`, and parses command segments. The inspected file has no local `#[cfg(test)] mod tests`.

**How to avoid:**
Before expanding verify commands or CLI dispatch behavior, add focused tests for path traversal, symlinks, shell control operators, pipelines, allowed/disallowed cargo and git subcommands, and write-capable command cases.

**Warning signs:**
- Patches relax shell validation to make experiment tasks easier.
- New tasks require broader shell syntax without a security review.
- CI failures are handled by removing validation rather than adding precise allowlist cases.

**Phase to address:**
Phase 1 reliability hardening and every later phase that changes verification commands.

**File-backed evidence:**
- `src/safety.rs` implements path and command validation.
- `GPD/research-map/CONCERNS.md` says security-critical shell validation lacks focused tests.
- `HANDOFF_TO_GPD.md` forbids weakening lint, CI, hooks, or verification gates.

---

### Pitfall 12: Structured-Output Recovery Is Load-Bearing But Lightly Tested

**What goes wrong:**
Local models can return markdown fences, malformed JSON strings, or control characters. If sanitizer behavior regresses, dispatch may fail before verification or return malformed results to the frontier, recreating the original token-wasting failure mode.

**Why it happens:**
`src/llm_io.rs` contains `strip_code_fences` and `sanitize_json_strings`, and dispatch depends on them before parsing model responses. The inspected file has no local tests.

**How to avoid:**
Add tests for fenced JSON, fenced code, embedded newlines, tabs, carriage returns, escaped quotes, nested strings, and non-fenced text. Include at least one malformed-output fixture in mocked dispatch integration.

**Warning signs:**
- Local dispatch returns parse errors for common fenced responses.
- Frontier has to read raw local-model output to recover.
- A schema patch changes output parsing without fixture coverage.

**Phase to address:**
Phase 1 reliability hardening, especially patches that modify dispatch contract or response parsing.

**File-backed evidence:**
- `src/llm_io.rs` defines sanitizer utilities used by `src/dispatch.rs`.
- `UPDATED_PROGRESS_REPORT.md` identifies malformed JSON and structured-output discipline as a central original failure mode.
- `GPD/research-map/CONCERNS.md` calls out missing focused tests for structured-output recovery.

---

### Pitfall 13: License Metadata Conflict Undercuts Public Claims

**What goes wrong:**
Even if Step 1 succeeds technically, release or public writeup work can be blocked by inconsistent license metadata.

**Why it happens:**
Repository metadata says MIT in `Cargo.toml`, `LICENSE`, and `README.md`, while `HANDOFF_TO_GPD.md` states "License: see LICENSE (AGPL-3.0)." The handoff line contradicts the referenced license file.

**How to avoid:**
Resolve the metadata conflict before any public release, package claim, or external-facing report. Treat handoff notes as stale if repository metadata is authoritative, but record the reconciliation.

**Warning signs:**
- Docs mention AGPL and MIT in the same release preparation path.
- PR templates or package metadata are updated without checking license consistency.
- External claims cite the handoff instead of repo metadata.

**Phase to address:**
Phase 1 cleanup or pre-publication release phase; not a Step 1 measurement blocker.

**File-backed evidence:**
- `Cargo.toml` declares MIT.
- `LICENSE` is MIT.
- `README.md` says MIT.
- `HANDOFF_TO_GPD.md` says AGPL-3.0 while pointing to `LICENSE`.

## Open Problems That Can Block A Defensible Step 1

| Open problem | Why it matters | Required resolution | Phase |
|---|---|---|---|
| Frontier baseline missing | No token-savings claim is possible without direct comparison | Produce task-aligned `baseline.csv` or richer split-token baseline | Phase 2 |
| Assisted-arm frontier overhead undefined | Local success may still cost paid frontier tokens | Add coordinator-token accounting for dispatch, result review, and recovery | Phase 2 |
| 14B sweep unavailable through reproducible config | Cannot evaluate approved 7B vs 14B matrix | Add explicit model override and separate result artifacts | Phase 1 -> Phase 2 |
| Retry defaults and task overrides conflict | Experiment can measure stale retry behavior | Change source default and freeze task `max_attempts` policy | Phase 1 -> Phase 2 |
| Failure taxonomy missing | Reliability patches cannot be attributed to failure modes | Add first-class `failure_category` and report rollups | Phase 1 |
| Split token pricing missing | Paid-cost estimate can be wrong by token mix | Track input/output separately in baseline and reports | Phase 1 -> Phase 2 |
| Task pack too small and Python-only | Results are not generalizable to Awl's actual coding workflow | Pre-register at least 10 mixed tasks | Phase 2 |
| CI lacks mocked end-to-end dispatch | Patches can pass CI while breaking real dispatch semantics | Add mocked integration and fixture tests | Phase 1 |

## Approximation Shortcuts

| Shortcut | Immediate benefit | Long-term cost | When acceptable |
|---|---|---|---|
| Use Awl pass rate as the headline metric | Quick progress signal | Does not answer paid-token savings | Only as smoke-test status |
| Use one blended token price | Simple tally CLI | Misstates paid-cost savings for asymmetric input/output pricing | Exploratory notes only, clearly labeled |
| Keep three Python tasks | Fast reruns | Selection bias and weak generalization | Only before Step 1 resumes |
| Let task JSON override retry defaults | Per-task control | Hidden experiment policy and stale behavior | Only if each override is pre-registered and justified |
| Treat visible unit tests as full correctness oracle | Easy automation | False passes and hidden frontier review cost | Only for explicitly bounded tasks with documented coverage limits |
| Use ambient model config for 14B | Avoids contract plumbing | Non-reproducible model-sweep results | Never for formal Step 1 data |

## Measurement Traps

| Trap | Symptoms | Prevention | When it breaks |
|---|---|---|---|
| Baseline absent | Token savings column blank; only Awl pass rate shown | Require baseline rows before Step 1 conclusion | Immediately, for any savings claim |
| Task-ID mismatch | Baseline totals omit tasks silently or compare wrong tasks | Validate exact task-ID set equality | As soon as task pack changes |
| Assisted overhead omitted | Failed dispatches look cheap because recovery is external | Record frontier coordinator tokens for assisted arm | Any failed or ambiguous dispatch |
| Retried failures counted as useful effort | Local tokens grow but same test fails | Default one verify attempt; categorize failures | Model capability gaps |
| Model mix hidden | One result file includes multiple models or config drift | Separate result files and explicit model metadata | 7B/14B sweep |
| Split pricing omitted | Dollar estimate not tied to input/output mix | Store prompt and completion tokens separately | Output-heavy tasks and failures |

## Verification And CI Traps

| Trap | Risk | Prevention |
|---|---|---|
| Optional live dispatch eval treated as coverage | CI can pass without end-to-end dispatch behavior | Add mocked integration tests independent of Ollama |
| Empty-log cost-report check | Telemetry parser is not tested on real event fixtures | Add non-empty JSONL fixtures with failures and successes |
| `experiments/tally.py` untested | Report can silently miscompute the headline result | Add fixture tests with missing baseline, partial baseline, split pricing, failures |
| Safety validation untested locally | Verification-command patches may broaden execution unintentionally | Add focused `src/safety.rs` tests |
| Structured-output sanitizer untested | Frontier may pay to recover malformed local output | Add `src/llm_io.rs` fixtures and dispatch parser tests |
| Weak task tests | Awl pass rate overstates semantic reliability | Add task-specific verification-strength rubric |

## Interpretation Mistakes

| Mistake | Risk | Prevention |
|---|---|---|
| "2/3 passed, so Step 1 is promising" | Confuses local capability with paid-token savings | Always pair pass rate with direct-frontier and assisted-frontier token ledgers |
| "7B failed, so Awl cannot work" | Overgeneralizes from one tiny task pack | Run pre-registered mixed pack and 14B sweep |
| "14B should be default if it passes more" | Ignores latency and frontier overhead | Compare net paid-token savings and pass rate by model config |
| "Rollback means failure is free" | Ignores local time and paid frontier recovery | Count wall time, local tokens, and frontier recovery tokens |
| "CI green means reliability patches are safe" | Misses unmocked dispatch and experiment paths | Require targeted tests for changed behavior |
| "A passing verify command means no frontier review" | Hides semantic gaps in weak tests | Calibrate verification strength and record residual review needed |

## Publication Pitfalls

| Pitfall | Impact | Better approach |
|---|---|---|
| Claiming frontier-token savings without baseline | False-positive core result | State "local-arm smoke test only" until baseline exists |
| Reporting only aggregate savings | Hides task-class failures and retry burn | Include per-task table, pass/fail, failure category, model, attempts |
| Omitting failed tasks from savings average | Inflates Awl value | Report all attempted tasks and passing-only metrics separately |
| Using tuned task pack without disclosure | Non-reproducible benchmark | Pre-register task pack and include task JSON in artifact set |
| Ignoring license conflict | Release/compliance ambiguity | Reconcile license metadata before public claims |

## "Looks Correct But Is Not" Checklist

- [ ] **Awl pass rate:** Looks acceptable at 2/3, but savings remain unknown until `baseline.csv` exists and matches all task IDs.
- [ ] **Retry patch:** Looks like a one-line source fix, but task JSON `max_attempts` can keep old behavior.
- [ ] **14B sweep:** Looks runnable via config changes, but reproducibility requires explicit model override and separate artifacts.
- [ ] **Cost estimate:** Looks precise in dollars, but blended pricing and combined frontier tokens can be wrong.
- [ ] **CI green:** Looks reliable, but live dispatch, tally fixtures, and non-empty telemetry paths are not covered by normal CI.
- [ ] **Passing task:** Looks correct, but only as strong as the visible `verify_command`.
- [ ] **Rollback:** Looks safe, but reliability still depends on shell/path validation and compact failure reporting.

## Recovery Strategies

| Pitfall | Recovery cost | Recovery steps |
|---|---|---|
| Missing baseline | MEDIUM | Pause Step 1 conclusion; run direct frontier baseline for frozen task pack; rerun tally |
| Retry override contamination | LOW-MEDIUM | Audit task JSON; rerun local arm after retry-policy freeze; mark older data as non-comparable |
| Task pack bias | HIGH | Pre-register expanded mixed pack; keep old data only as pilot smoke-test evidence |
| Model config drift | MEDIUM | Implement explicit model override; regenerate 7B and 14B result files |
| Blended pricing report | LOW | Add split token fields and recompute; label old dollar estimates exploratory |
| Weak verify tests | MEDIUM-HIGH | Strengthen tests before rerun; do not patch tasks after seeing model outputs |
| CI blind spot | MEDIUM | Add mocked dispatch and fixture tests before merging reliability-contract changes |
| License conflict | LOW | Reconcile docs and metadata in a dedicated cleanup PR |

## Pitfall-To-Phase Mapping

| Pitfall | Prevention phase | Verification |
|---|---|---|
| Pass rate mistaken for savings | Phase 2 | Tally refuses or clearly marks incomplete baseline |
| Frontier overhead omitted | Phase 2 | Assisted-arm token ledger exists for every task |
| Retry source/task mismatch | Phase 1 and Phase 2 | Source tests plus task JSON audit before rerun |
| Deterministic same-model retries | Phase 1 | Failed verify defaults to one apply attempt unless explicitly overridden |
| Task-pack bias | Phase 2 | Frozen >=10 mixed task pack with documented categories |
| Missing model override | Phase 1 | CLI, MCP, task JSON/driver tests show override beats level default |
| Blended pricing | Phase 1 and Phase 2 | Reports compute input/output cost separately |
| Weak verify oracle | Phase 2 | Verification-strength rubric attached to each task |
| Missing failure taxonomy | Phase 1 | Result JSON and reports include `failure_category` |
| CI dispatch blind spots | Phase 1 | Mocked integration and fixture tests in normal CI |
| License conflict | Cleanup/pre-publication | Repo metadata and handoff/docs agree |

## Quality Gate Status

- [x] Pitfalls are specific to this project.
- [x] Misleading positive-result paths are explicit.
- [x] Verification and CI risks are included.

## Bottom Line

The current evidence supports a narrow conclusion only: Awl's local-arm pipeline can run bounded tasks and expose a real 7B failure mode. It does not yet support a Step 1 savings claim. A defensible result requires baseline token accounting, split pricing, frozen mixed tasks, explicit model selection, corrected retry policy, failure taxonomy, and stronger CI coverage for the dispatch and experiment-reporting paths.

---

_Known pitfalls research for: Awl frontier-token savings and bounded local coding delegation_
_Researched: 2026-05-01_
