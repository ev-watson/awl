# CONCERNS.md — Awl Project Status Analysis

**Project:** Awl (Rust CLI + MCP server for local-model coding dispatch)
**Analysis Date:** 2026-04-30
**Focus:** status (development state, blockers, open questions, risks)
**Template:** Fallback structure (template file inaccessible at deterministic install path)

---

## 1. Current Development State

### What is shipped

PR #14 merged to `main` at commit `29ef94f`. The following capabilities are live in source but require `cargo build --release` to produce an updated binary:

- Dispatch v2 contract with `target_path`, `context_paths`, `verify_command`, `apply` mode, `max_attempts`, `auto_repomap` — `src/dispatch.rs` (1600 lines)
- Snapshot/write/verify/rollback cycle — `src/dispatch.rs` lines 1119-1200
- Hallucinated-import preflight (Rust only) — `src/repomap.rs:known_rust_modules`
- JSONL per-dispatch telemetry with `prompt_tokens`/`completion_tokens` — `src/dispatch.rs:59-90`
- L1 agent gated behind `AWL_ENABLE_MCP_AGENT=1` — `src/defaults.rs:97-101`
- Experiment harness: `experiments/run_awl_arm.sh`, `experiments/tally.py`, 3 tasks
- Cost report script: `scripts/dispatch_cost_report.py`
- 56 unit tests across 7,428 lines of Rust source; CI enforces `clippy -D warnings`, `cargo test`, `cargo fmt --check` on Ubuntu and macOS

### What is halted

Step 1 of the A/B frontier-token savings experiment. Three tasks ran at L2 (7B-q4):

| Task | Result | Attempts | Tokens | Wall (ms) |
|------|--------|----------|--------|-----------|
| `01_string_helper` | FAIL | 2 (max) | 4264 | 29686 |
| `02_validate_input` | PASS | 1 | 2073 | 15431 |
| `03_fix_off_by_one` | PASS | 1 | 2264 | 17577 |

Experiment halted because measuring more tasks on top of a defective retry policy and non-overridable model selection would "measure the wrong thing" — per `REPORT_DISPATCH_RELIABILITY.md`.

### What has never been run

- Frontier-baseline arm (no `experiments/results/baseline.csv` exists)
- End-to-end `tally.py` comparison
- Any 14B configuration
- Any escalation scenario

---

## 2. Blockers (Patch List)

Five items from `REPORT_DISPATCH_RELIABILITY.md` section "Proposed architectural changes." Items 1 and 2 are hard blockers for resuming Step 1. Items 3-4 are soft blockers (data quality). Item 5 depends on 1-2.

### BLOCKER-1: Drop default verify-retry from 2 to 1

- **File:** `src/dispatch.rs:1110-1112`
- **Current:** `effective_max_attempts` defaults to 2 when `apply && has_verify`
- **Change:** Default to 1. One-line diff: `let default = if apply && has_verify { 2 } else { 1 };` becomes `let default = 1;`
- **Impact:** Prevents wasting ~2200 tokens on same-model retry when the failure is a capability gap, not a transient flake. The 01_string_helper failure proved this: both attempts produced identical wrong code.
- **Risk:** Low. Callers can still pass `max_attempts: 2` explicitly. Format/schema/preflight retries are separate (in `dispatch_with_retry`) and unaffected.
- **Status:** User-confirmed. Ready to implement.
- **Effort:** Trivial (one-line code change + test update)

### BLOCKER-2: Per-dispatch model override

- **File:** `src/dispatch.rs:31-41` (`DispatchOptions` struct), `src/main.rs`, `src/mcp_server.rs:231-273`, `experiments/run_awl_arm.sh`
- **Current:** `DispatchOptions` has no `model` field. Model is determined solely by level via `defaults::configured_model_for_level` (`src/defaults.rs:74-88`). The `--model` flag exists only on the `agent` subcommand (`src/main.rs:273`), NOT on `dispatch`.
- **Change:** Add `model: Option<String>` to `DispatchOptions`. Plumb through CLI (`--model` flag on `dispatch`), MCP schema (new `model` property in `awl_dispatch`), and experiment harness (`AWL_MODEL_OVERRIDE` env var in `run_awl_arm.sh`). `configured_model_for_level` becomes the fallback when `model` is `None`.
- **Impact:** Enables the Step 1 sweep at two configurations (7B-only, 14B-only). Enables frontier to pick model per-dispatch based on task risk assessment.
- **Risk:** Medium. This is a dispatch contract change visible to MCP callers. The tool schema gains a new optional property. Existing callers are unaffected (new field defaults to `None`).
- **Status:** User-confirmed design. Not yet implemented.
- **Effort:** Moderate (touches 4 files, needs tests for override-beats-default and unset-uses-default)

### BLOCKER-3 (soft): Failure taxonomy in telemetry

- **File:** `src/dispatch.rs` — `apply_result` (line 1212) and `error_result` (line 1241), `scripts/dispatch_cost_report.py` (lines 15-24)
- **Current:** `apply_result` and `error_result` do not include a `failure_category` field. `dispatch_cost_report.py` maps events to `ERROR_EVENTS`/`SUCCESS_EVENTS` sets but does not aggregate by failure category.
- **Change:** Add `failure_category` enum: `format`, `schema`, `preflight`, `verify`, `timeout`, `network`, `unknown`. Wire each existing failure path in dispatch.rs. Update cost report to aggregate and display by category.
- **Impact:** Without this, the frontier cannot learn which task classes benefit from opting up to 14B. The category data informs the frontier's risk model.
- **Risk:** Low. Additive change to JSON output. Existing callers that do not read `failure_category` are unaffected.
- **Status:** Designed. Not yet implemented.
- **Effort:** Moderate (must audit all failure paths in dispatch.rs and assign categories)

### BLOCKER-4 (soft): Split cost reporting by input/output tokens

- **Files:** `experiments/tally.py` (lines 164-195), `scripts/dispatch_cost_report.py` (lines 37-75)
- **Current:** `tally.py` takes a single `--cost-per-mtok` blended rate. `dispatch_cost_report.py` takes `--frontier-cost-per-mtok` blended rate. Neither splits input vs output.
- **Change:** Replace with `--input-cost-per-mtok` (default $5) and `--output-cost-per-mtok` (default $25) matching Claude Opus 4.7 standard pricing. Read `prompt_tokens`/`completion_tokens` from the existing `usage` field (already captured by `run_awl_arm.sh` at lines 66-70).
- **Impact:** Without this, savings reporting systematically misestimates real avoided spend because Claude pricing is 5x asymmetric between input and output tokens. Additional note: Opus 4.7 tokenizer produces ~35% more tokens for the same text than prior Claude tokenizers, making accurate per-type accounting even more important.
- **Risk:** Low. Pure tooling change, no Rust code affected.
- **Status:** Designed. Not yet implemented.
- **Effort:** Small (Python-only changes)

### BLOCKER-5 (dependent): Resume Step 1 — scale task pack and run sweep

- **Files:** `experiments/tasks/` (currently 3 tasks, needs 10+), `experiments/run_awl_arm.sh`
- **Current:** 3 tasks exist: `01_string_helper` (write-from-scratch, Python), `02_validate_input` (write-from-scratch, Python), `03_fix_off_by_one` (edit-existing, Python with `context_paths`). All Python. No Rust generation tasks. No hard tasks exercising `context_paths` meaningfully.
- **Change:** Scale to 10+ tasks with mix of: write-from-scratch, edit-existing, context-paths-required, Python and Rust generation. Run sweep at 7B-only and 14B-only configurations via `AWL_MODEL_OVERRIDE`. Run frontier-baseline arm manually. Run tally per configuration.
- **Impact:** This IS the experiment. Without it, the product hypothesis remains untested.
- **Depends on:** BLOCKER-1 and BLOCKER-2 must land first; BLOCKER-3 and BLOCKER-4 strongly recommended.
- **Risk:** High (see Section 4 — the experiment might produce a negative result).
- **Effort:** Large (task design, two sweep runs, manual baseline arm, analysis)

---

## 3. Open Empirical Questions

These are questions that cannot be answered by reading code or documents. They require running experiments.

### Q1: Does 14B catch what 7B misses?

- **Evidence so far:** 7B-q4 fails `01_string_helper` deterministically (trailing-newline edge case). 14B is unmeasured on this or any experiment task.
- **What would answer it:** Run the same 10+ task pack at both 7B and 14B. Compare pass rates.
- **Why it matters:** If 14B has a substantially higher pass rate, the frontier can opt up to 14B for fragile tasks and still save tokens vs doing the task itself. If 14B is comparable to 7B, the extra 2x latency and memory cost are wasted.
- **Relevant code:** `src/defaults.rs:5-6` defines the model constants. The per-dispatch model override (BLOCKER-2) is the mechanism to test this.

### Q2: Does local dispatch actually save net frontier tokens?

- **Evidence so far:** None. No baseline arm has been run. The partial Step 1 data (2/3 passing, 8601 total tokens at 7B) cannot be compared to anything.
- **What would answer it:** Complete A/B experiment with frontier-baseline data. Compare `awl_arm.jsonl` total tokens (which the frontier does not pay) against `baseline.csv` frontier tokens (which the frontier does pay), factoring in the frontier overhead of packaging the dispatch and reviewing the result.
- **Why it matters:** This is the entire product hypothesis. If the answer is no, Awl does not save money and the project's value proposition is invalid.
- **Critical subtlety:** The "frontier overhead" of packaging a dispatch (composing the task JSON, reviewing the compact result) is NOT zero. If this overhead exceeds the tokens saved by not doing the task directly, Awl is net-negative even when every local dispatch succeeds.

### Q3: What is the frontier overhead per dispatch?

- **Evidence so far:** Not measured. Not even estimated.
- **What would answer it:** Instrument the frontier side during the A/B experiment. Count tokens spent on: reading the dispatch result, deciding whether to accept or redo, any follow-up turns.
- **Why it matters:** The success criterion is net savings. If packaging + reviewing costs 1500 tokens and the direct implementation costs 2500 tokens, the local dispatch must succeed to save 1000 tokens. But if the local dispatch fails (as 01 did), the frontier still spends 1500 tokens on packaging + review AND then does the task itself for 2500 tokens, totaling 4000 — 60% more than direct.
- **Risk:** This overhead may dominate for small tasks where direct implementation is cheap.

### Q4: What is the minimum task size where dispatch breaks even?

- **Evidence so far:** Not measured. `UPDATED_PROGRESS_REPORT.md` (line 205) acknowledges that "one-line edits where delegation overhead dominates" are likely net-negative.
- **What would answer it:** Vary task complexity in the experiment. Compare savings for small vs medium tasks.
- **Why it matters:** Without a minimum-size threshold, the frontier's dispatch-vs-direct decision is uninformed, leading to over-delegation on small tasks (wasting tokens on overhead) or under-delegation on large tasks (missing savings).

### Q5: Does the Opus 4.7 tokenizer shift change the economics?

- **Evidence so far:** `REPORT_DISPATCH_RELIABILITY.md` (line 151) notes "the Opus 4.7 tokenizer can produce up to ~35% more tokens for the same text vs prior Claude tokenizers."
- **What would answer it:** Compare actual token counts from Opus 4.7 frontier sessions against expectations. If Opus 4.7 produces 35% more tokens for the same task, the frontier cost of direct implementation rises proportionally — which would make local dispatch (unaffected by this tokenizer change) relatively more attractive.
- **Why it matters:** The success criterion uses token counts. If the baseline cost is measured with Opus 4.7's tokenizer and the savings threshold is 25%, the bar for Awl may be effectively lower or higher depending on whether the tokenizer inflation helps or hurts the comparison.

---

## 4. Risks

### RISK-1 (Critical): The product hypothesis might be wrong

- **Description:** Awl's entire value proposition is that bounded local dispatch saves frontier tokens on net. This has never been measured. There are plausible failure modes:
  - Frontier overhead of packaging + reviewing dispatches may dominate savings
  - The set of tasks where local 7B/14B succeeds reliably may be too narrow to matter in practice
  - Tasks where local dispatch succeeds may also be tasks where direct frontier implementation is cheap (small, mechanical), limiting the token differential
- **Evidence for concern:** The 01_string_helper failure — an "easy" task that 7B could not handle — suggests the reliability floor is low. If even easy tasks have a meaningful failure rate, the expected savings per dispatch are eroded by the failure cost.
- **Mitigation:** Run the experiment (BLOCKER-5). Accept a negative result as valid signal. Define the pivot criteria: if no configuration achieves >=25% token reduction at >=60% pass rate, the project needs a fundamental strategic change (different models, different task targeting, or wind-down).

### RISK-2 (High): Task pack may not be representative

- **Description:** The current 3 tasks are all Python, all "easy" difficulty, and only one uses `context_paths`. The expanded 10+ task pack must cover the actual distribution of tasks that a frontier model would consider delegating. If the pack is biased toward tasks Awl handles well, the experiment result is not predictive of real-world savings.
- **Specific gaps in current pack:**
  - No Rust generation tasks (Awl is a Rust project — Rust tasks are a natural use case)
  - No tasks requiring meaningful `context_paths` reading (03 has a test file as context, but the model must mostly write new code)
  - No tasks testing error handling, multi-function coordination, or import resolution
  - No tasks at the boundary of what 7B can handle (all are classified "easy")
- **Mitigation:** Include at least: 2 Rust tasks, 2 tasks with substantial context_paths, 2 "medium" difficulty tasks, 1 task that is expected to fail (to verify rollback works cleanly).

### RISK-3 (High): No auto-escalation — frontier must guess model tier correctly

- **Description:** The design decision (per `REPORT_DISPATCH_RELIABILITY.md`) is "no auto-escalation." The frontier picks 7B or 14B upfront. If the frontier picks 7B and it fails, tokens are wasted. If the frontier always picks 14B to be safe, latency doubles and the comparison vs direct frontier may narrow.
- **Why this matters:** The frontier's model selection heuristic does not exist yet. Step 1 is supposed to calibrate it. But Step 1 tests each configuration separately — it does not test the frontier's ability to predict which tasks need 14B.
- **Mitigation:** After Step 1, a Step 2 study where the frontier applies its learned heuristic and results are compared to both single-configuration baselines.

### RISK-4 (Medium): Verify timeout is hardcoded

- **File:** `src/dispatch.rs:28` — `const VERIFY_TIMEOUT_MS: u64 = 120_000;`
- **Description:** The verify command timeout is fixed at 120 seconds. Some tasks (especially Rust compilation + test) may exceed this on slower machines.
- **Impact:** A task that takes >120s to verify will be reported as a timeout failure even if the code is correct. This produces false negatives in the experiment.
- **Mitigation:** Deferred per `REPORT_DISPATCH_RELIABILITY.md` — "Defer until a real verify command actually exceeds 120s." This is acceptable for now but should be monitored during the expanded task pack run.

### RISK-5 (Medium): dispatch.rs is 1600 lines in a single file

- **File:** `src/dispatch.rs`
- **Description:** The entire dispatch pipeline — parsing, preflight, prompt construction, API calls, response processing, apply/verify/rollback, telemetry, retry — lives in one 1600-line file. Adding the failure taxonomy (BLOCKER-3) and model override (BLOCKER-2) will push it further.
- **Impact:** Increases risk of merge conflicts, makes code review harder, and makes it difficult for the frontier or local models to hold the full context when making changes.
- **Mitigation:** Not urgent. The file is well-structured with clear function boundaries. Splitting can happen after the experiment, not before.

### RISK-6 (Low): Python preflight does not exist

- **File:** `src/repomap.rs:known_rust_modules` — Rust-only
- **Description:** The hallucinated-import preflight only works for Rust. All three current experiment tasks are Python. A Python dispatch that hallucinated `import pandas` (when pandas is not needed or not installed) would not be caught by preflight.
- **Impact:** Low for the current experiment (tasks are constrained to "no external imports"). Could matter in production use with Python tasks.
- **Mitigation:** Deferred per `REPORT_DISPATCH_RELIABILITY.md` — "Defer until Step 1 shows it would have caught actual failures." Correct approach given current priorities.

---

## 5. Gap Analysis: Current State vs Success Criterion

**Success criterion** (from `HANDOFF_TO_GPD.md` line 143):
> Step 1 produces a defensible answer, with at least one configuration showing >=25% frontier-token reduction at >=60% Awl-pass rate, OR a defensible negative result with documented blockers.

### Distance to criterion

| Requirement | Current State | Gap |
|------------|---------------|-----|
| Task pack >= 10 tasks | 3 tasks exist | Need 7+ more tasks with required diversity |
| 7B-only sweep run | Partial (3 tasks, 2/3 pass = 67%) | Need full run on 10+ tasks |
| 14B-only sweep run | Not started | Need full run on 10+ tasks |
| Frontier-baseline arm | Not started | Need manual frontier runs for all tasks |
| Tally comparison | Not started | Depends on both arms completing |
| Retry policy fixed | Default still 2 | BLOCKER-1 (trivial to fix) |
| Model override plumbed | Not implemented | BLOCKER-2 (moderate effort) |
| Cost split by I/O tokens | Not implemented | BLOCKER-4 (small effort) |
| Failure taxonomy | Not implemented | BLOCKER-3 (moderate effort) |
| Defensible result | No result exists | Depends on all above |

### Estimated critical path

1. BLOCKER-1 (one-line change + test) — gate: PR must pass CI
2. BLOCKER-2 (model override, touches 4 files) — gate: PR must pass CI, user confirmation before merge since it changes dispatch contract
3. BLOCKER-4 (Python tally/cost updates) — gate: PR must pass CI
4. BLOCKER-3 (failure taxonomy) — can ship with or after BLOCKER-2
5. Design and create 7+ additional experiment tasks
6. Run 7B-only sweep on 10+ tasks
7. Run 14B-only sweep on 10+ tasks
8. Run frontier-baseline arm (manual, per `experiments/README.md` lines 49-70)
9. Run `tally.py` per configuration against baseline
10. Analyze and write up

Steps 1-4 are code changes (estimable). Steps 5-9 are experimental work (duration depends on task design time and frontier-baseline session time). Steps 6-7 are automated once the harness works.

---

## 6. Deferred Items (Not Blockers, Tracked for Completeness)

These are explicitly deferred per `REPORT_DISPATCH_RELIABILITY.md` and `HANDOFF_TO_GPD.md`. They are NOT scope for the current phase.

| Item | Rationale for Deferral | Revisit When |
|------|----------------------|-------------|
| Per-dispatch verify timeout | No real verify command has exceeded 120s yet | A task hits the timeout during Step 1 |
| Per-dispatch local-token ceiling | Local tokens are free; ceiling adds complexity without value | Local cost becomes a constraint |
| Dispatch cache by fingerprint | Useful for reproducibility, not for savings measurement | Step 1 complete, reproducibility matters |
| Python preflight | High false-positive risk; no Python hallucination failure observed yet | Step 1 reveals Python import hallucination failures |
| Streaming dispatch output | Major transport change; cosmetic benefit only | Base reliability proven |
| Verifier convenience wrappers (`awl verify`, `awl lint`) | Lower priority than A/B test | A/B test complete |

---

## 7. Priority Rankings

### Immediate (blocks experiment)

1. **BLOCKER-1:** Drop default verify-retry to 1 — `src/dispatch.rs:1111`
2. **BLOCKER-2:** Per-dispatch model override — `src/dispatch.rs`, `src/mcp_server.rs`, `src/main.rs`, `experiments/run_awl_arm.sh`

### High (improves experiment data quality)

3. **BLOCKER-4:** Split cost reporting by I/O tokens — `experiments/tally.py`, `scripts/dispatch_cost_report.py`
4. **BLOCKER-3:** Failure taxonomy in telemetry — `src/dispatch.rs`, `scripts/dispatch_cost_report.py`

### Required (experiment execution)

5. **BLOCKER-5:** Scale task pack to 10+, run sweep, run baseline, produce tally

### Monitor during experiment

6. **RISK-4:** Watch for verify timeout hits
7. **RISK-6:** Watch for Python import hallucinations

### Post-experiment

8. **RISK-5:** Consider splitting dispatch.rs if it exceeds ~2000 lines after patches
9. **RISK-3:** Design Step 2 (frontier model-selection heuristic validation)

---

_Analysis based on: `HANDOFF_TO_GPD.md`, `REPORT_DISPATCH_RELIABILITY.md`, `UPDATED_PROGRESS_REPORT.md`, `experiments/README.md`, `experiments/tasks/*/task.json`, `src/dispatch.rs`, `src/defaults.rs`, `src/mcp_server.rs`, `src/main.rs`, `experiments/run_awl_arm.sh`, `experiments/tally.py`, `scripts/dispatch_cost_report.py`, `.github/workflows/ci.yml`._
