# Prior Work: Awl Frontier-Token Savings and Bounded Local Coding Delegation

**Surveyed:** 2026-05-01  
**Domain:** software-systems evaluation of local LLM coding delegation  
**Research mode:** balanced  
**Confidence:** MEDIUM for internal artifact facts; LOW for any external rate/model-performance implication not yet independently sourced

## Scope and Evidence Boundary

This is a continuation survey for the known-results dimension of Awl's project question:

> Can Awl's bounded local Ollama dispatch workflow produce defensible frontier-token savings in real Claude/Codex coding sessions without unacceptable pass-rate or reliability regressions?

The survey intentionally prioritizes project-internal anchors over broad AI-agent literature. I inspected the approved anchors and immediately relevant support files:

- `GPD/state.json`
- `GPD/PROJECT.md`
- `HANDOFF_TO_GPD.md`
- `REPORT_DISPATCH_RELIABILITY.md`
- `UPDATED_PROGRESS_REPORT.md`
- `experiments/README.md`
- `experiments/results/awl_arm.jsonl`
- `experiments/results/01_string_helper.json`
- `experiments/results/02_validate_input.json`
- `experiments/results/03_fix_off_by_one.json`
- `GPD/research-map/FORMALISM.md`
- `GPD/research-map/REFERENCES.md`
- `GPD/research-map/ARCHITECTURE.md`
- `GPD/research-map/CONCERNS.md`
- source cross-checks in `src/dispatch.rs`, `src/defaults.rs`, `src/mcp_server.rs`, `src/tools.rs`, `experiments/tally.py`, `scripts/dispatch_cost_report.py`, and `experiments/tasks/*/task.json`

No external scholarly or vendor sources were pulled into this artifact. External/background anchors still need to be added before public-facing claims about pricing, tokenizer behavior, model capabilities, or protocol conformance.

## Key Known Results

| Result | Value / Observation | Conditions | Source | Confidence |
|---|---:|---|---|---|
| Awl's project contract is now present and defines the core research question, required observables, deliverables, forbidden proxies, and uncertainty markers. | Contract exists in `GPD/state.json`; earlier research-map files that say `GPD/state.json` is missing are stale on this point. | Current workspace on 2026-05-01. | `GPD/state.json`, `GPD/PROJECT.md` | HIGH |
| The only existing benchmark data is a partial 7B local Awl arm. | 3 tasks attempted; 2 passed; aggregate Awl pass rate 2/3 = 67%; total local worker tokens 8601. | L2 implementation model `qwen2.5-coder:7b-instruct-q4_K_M`; current three-task Python pack; current task retry overrides. | `experiments/results/awl_arm.jsonl`; `python3 experiments/tally.py` output | HIGH |
| `01_string_helper` failed deterministically under 7B with same-model retry. | Failed after 2 attempts; 4264 total local tokens; both failures hit `test_preserves_trailing_newline`; no file was left changed. | Python write-from-scratch task; `max_attempts: 2`; local verifier `python3 -m unittest ...`; apply/verify/rollback path. | `experiments/results/01_string_helper.json`, `REPORT_DISPATCH_RELIABILITY.md` | HIGH |
| `02_validate_input` passed first try under 7B. | 1 attempt; 2073 total tokens; 1931 prompt and 142 completion tokens; modified `experiments/sandbox/02/validators.py`; checks passed. | Python write-from-scratch task with bounded string-validation spec and unit tests. | `experiments/results/02_validate_input.json` | HIGH |
| `03_fix_off_by_one` passed first try under 7B. | 1 attempt; 2264 total tokens; 2113 prompt and 151 completion tokens; modified `experiments/sandbox/03/moving_average.py`; checks passed. | Python edit-existing task with one context path and unit tests. | `experiments/results/03_fix_off_by_one.json` | HIGH |
| No direct frontier baseline exists in the workspace. | `experiments/results/baseline.csv` was not found; tally therefore reports baseline tokens/pass/wall as absent. | Current workspace inspection. | `experiments/README.md`, `experiments/results/`, `python3 experiments/tally.py` | HIGH |
| No 14B-only sweep exists in the workspace. | 14B comparison is proposed, not measured. | Current workspace; model override not implemented in source. | `REPORT_DISPATCH_RELIABILITY.md`, `GPD/state.json`, source `rg` cross-check | HIGH |
| The central savings claim is unproven. | Current artifacts justify a controlled experiment, not a positive or negative answer. | Missing same-task direct frontier baseline and missing 14B comparison. | `UPDATED_PROGRESS_REPORT.md`, `REPORT_DISPATCH_RELIABILITY.md`, `GPD/state.json` | HIGH |
| Current source still defaults apply+verify to 2 attempts when no override is supplied. | `effective_max_attempts(raw, apply, has_verify)` uses default `2` for `apply && has_verify`. | Source at inspection time; task specs also explicitly set 2 or 3 attempts. | `src/dispatch.rs`, `experiments/tasks/*/task.json` | HIGH |
| Per-dispatch model override is not yet implemented in the current source. | `DispatchOptions` has no `model: Option<String>` field; CLI/MCP schema expose `max_attempts` but not a dispatch model override. | Source at inspection time. | `src/dispatch.rs`, `src/main.rs`, `src/mcp_server.rs`, `src/tools.rs` | HIGH |
| Failure category telemetry is not yet first-class. | No `failure_category` field found in source or result JSON; cost report currently groups event names rather than dispatch-level categories. | Source and results at inspection time. | `REPORT_DISPATCH_RELIABILITY.md`, `scripts/dispatch_cost_report.py`, `experiments/results/*.json` | HIGH |
| Cost accounting still uses blended cost flags in scripts. | `experiments/tally.py --cost-per-mtok`; `scripts/dispatch_cost_report.py --frontier-cost-per-mtok`. | Source at inspection time; split input/output rates are a planned patch. | `experiments/tally.py`, `scripts/dispatch_cost_report.py` | HIGH |
| License metadata is inconsistent. | `LICENSE` and `Cargo.toml` say MIT; `HANDOFF_TO_GPD.md` says "see LICENSE (AGPL-3.0)". | Current repo metadata and handoff disagree. | `LICENSE`, `Cargo.toml`, `README.md`, `HANDOFF_TO_GPD.md` | HIGH |

## Existing Internal Baselines

### Partial 7B Local Awl Arm

**Source:** `experiments/results/awl_arm.jsonl`

| Task | Type | Model | Attempts | Status | Prompt Tokens | Completion Tokens | Total Tokens | Wall ms | Condition |
|---|---|---|---:|---|---:|---:|---:|---:|---|
| `01_string_helper` | Python write-from-scratch | `qwen2.5-coder:7b-instruct-q4_K_M` | 2 | error | 4082 | 182 | 4264 | 29686 | Trailing-newline unit test failed twice; rollback restored prior file state. |
| `02_validate_input` | Python write-from-scratch | `qwen2.5-coder:7b-instruct-q4_K_M` | 1 | ok | 1931 | 142 | 2073 | 15431 | All 8 tests passed first try. |
| `03_fix_off_by_one` | Python edit-existing | `qwen2.5-coder:7b-instruct-q4_K_M` | 1 | ok | 2113 | 151 | 2264 | 17577 | All 5 tests passed first try. |

**Aggregate:** 3 attempted tasks, 2 passing tasks, 8601 total local worker tokens, 67% Awl pass rate.

**Conditions:** The benchmark uses only three Python tasks, all with local unit-test verifiers. The task specs override default attempts: tasks 01 and 02 set `max_attempts: 2`; task 03 sets `max_attempts: 3`. The result therefore measures the old retry regime for at least task 01, not the approved one-attempt policy.

**Relevance:** This is the only measured internal baseline. It is useful for identifying concrete failure modes and verifying harness plumbing, but it is not enough to establish frontier-token savings.

### Direct Frontier Baseline

**Source expected:** `experiments/results/baseline.csv`  
**Status:** missing.

`experiments/README.md` defines the manual frontier-baseline arm and expected CSV schema: `task_id,frontier_tokens,frontier_pass,wall_ms`. Without this file, no token-savings numerator or denominator can be computed for the central product hypothesis.

### 14B-Only Awl Arm

**Source expected:** a separate Awl run using `qwen2.5-coder:14b`, likely after `AWL_MODEL_OVERRIDE` or equivalent dispatch-level model override is implemented.  
**Status:** missing.

The project has an approved comparison target, but no internal artifact yet measures whether 14B improves pass rate, changes failure categories, or costs too much wall time.

### Threshold Baseline

The project uses success thresholds of at least 25% frontier-token reduction and at least 60% Awl pass rate for a defensible positive Step 1 result. The stronger historical range in `experiments/README.md` and `UPDATED_PROGRESS_REPORT.md` is 25-40% token reduction and 60-70% pass rate.

**Condition:** These are project decision thresholds, not externally validated universal standards. They are appropriate as acceptance criteria if the same-task baseline and Awl arms are collected with comparable accounting.

## Established Internal Techniques

### Bounded Local Worker Instead of Local Planner

Awl is framed as a narrow execution worker for Claude/Codex, not as a subordinate local planner. The frontier agent owns orchestration, task selection, final review, and fallback decisions.

**Sources:** `HANDOFF_TO_GPD.md`, `.agents/skills/awl-dispatch/SKILL.md`, `.claude/agents/awl-worker.md`, `GPD/PROJECT.md`  
**Conditions:** Applies to bounded tasks with explicit scope and acceptance checks. It does not cover architecture decisions, ambiguous requirements, security-sensitive judgment calls, or tiny edits where delegation overhead dominates.  
**Relevance:** This is the core design constraint for token savings: Awl can only help when the frontier handoff is compact and the return payload avoids expensive recovery work.

### Dispatch v2 Contract

Current dispatch supports target files, context paths, constraints, verify command, apply mode, max attempts, return truncation, auto repo map, and repo-map focus/budget.

**Sources:** `UPDATED_PROGRESS_REPORT.md`, `src/dispatch.rs`, `src/mcp_server.rs`, `src/tools.rs`  
**Conditions:** Apply mode is scoped to one effective target path; local verification must be expressible as an allowed shell command; correctness is only as strong as the verifier.  
**Relevance:** This is the mechanism that turns broad frontier work into a bounded local packet.

### OpenAI-Compatible Structured Output Discipline

The worker response is constrained by a JSON response schema with fields such as `status`, `code`, `explanation`, and `files_modified`, followed by local validation and recovery logic.

**Sources:** `UPDATED_PROGRESS_REPORT.md`, `src/dispatch.rs`, `src/llm_io.rs`  
**Conditions:** Depends on Ollama/model support quality and local JSON recovery. Format/schema retries are treated differently from apply/verify attempts.  
**Relevance:** Malformed local output was one of the original ways local work became paid frontier recovery. Structured output reduces that risk but still needs fixture coverage.

### Apply / Verify / Rollback

Apply mode snapshots the target file, writes generated code, runs the verifier, and restores prior contents on verification failure.

**Sources:** `UPDATED_PROGRESS_REPORT.md`, `REPORT_DISPATCH_RELIABILITY.md`, `src/dispatch.rs`, per-task result JSON files  
**Conditions:** Works when a single target path is supplied and a verifier exists. It protects workspace state but does not guarantee semantic correctness beyond the verifier.  
**Relevance:** This is the strongest internal technique supporting savings because failed local work can return as a compact failure instead of leaving the frontier to debug a dirty worktree.

### Compact Telemetry and Local Logs

Dispatch returns compact summary fields and stores fuller logs under the local Awl config directory. Experiment scripts also collect top-level usage, wall time, and status into `experiments/results/awl_arm.jsonl`.

**Sources:** `UPDATED_PROGRESS_REPORT.md`, `REPORT_DISPATCH_RELIABILITY.md`, `experiments/results/awl_arm.jsonl`, `src/dispatch.rs`, `experiments/run_awl_arm.sh`  
**Conditions:** Frontier savings depend on compact results being sufficient often enough that Claude/Codex does not need to inspect full logs or generated code for every task.  
**Relevance:** The return-size discipline is part of the savings mechanism, but compact-result sufficiency has not been directly measured.

### Repo-Map and Context Paths

Awl can inject scoped file context and an auto-generated repository map based on tree-sitter parsing and ranking.

**Sources:** `GPD/research-map/FORMALISM.md`, `GPD/research-map/ARCHITECTURE.md`, `src/repomap.rs`, `experiments/tasks/03_fix_off_by_one/task.json`  
**Conditions:** Current experiment evidence uses only one context-path task and no Rust context-path task. Python preflight is explicitly deferred.  
**Relevance:** Grounding is expected to reduce hallucinated imports and wrong edits, but current Step 1 data is too small to quantify the effect.

### A/B Savings Harness

The intended experiment compares a local Awl arm against a direct frontier arm on the same tasks. The tally script computes pass rate, total Awl tokens, and token savings when baseline data exists.

**Sources:** `experiments/README.md`, `experiments/tally.py`, `GPD/state.json`  
**Conditions:** Requires same-task task IDs in both `awl_arm.jsonl` and `baseline.csv`; direct frontier token accounting must be comparable and exclude unrelated session context as specified by `experiments/README.md`.  
**Relevance:** This is the main method needed to answer the project question. It exists as scaffolding but not as completed evidence.

## Planned or Approved Techniques Not Yet Established by Artifacts

| Technique / Output | Contract Status | Current Evidence | Missing Before It Becomes Established |
|---|---|---|---|
| Default apply+verify attempts from 2 to 1 | Approved in `REPORT_DISPATCH_RELIABILITY.md` and `GPD/state.json` | Source still defaults to 2; task specs override to 2/3 | Patch, tests, and rerun under new policy |
| Per-dispatch model override | Approved | Not found in `DispatchOptions`, CLI, MCP schema, or experiment harness | `model: Option<String>` plumbing and `AWL_MODEL_OVERRIDE` / equivalent |
| Failure category telemetry | Approved | Category list appears in reports only | `failure_category` in dispatch outputs and cost reports |
| Split input/output token cost accounting | Approved | Results already carry prompt/completion tokens; scripts still use blended flags | Script changes and report output using separate rates |
| 7B-only versus 14B-only Step 1 report | Approved deliverable | No 14B run and no direct frontier baseline | Expanded task pack, both Awl sweeps, baseline CSV, `experiments/results/report.md` |
| Frontier opt-up guidance from 7B to 14B | Approved deliverable | Only qualitative hypothesis | Data linking task type/failure category/pass rate/token savings to model choice |

## Missing Comparison Data

The following gaps block a defensible answer:

1. **Direct frontier baseline:** `experiments/results/baseline.csv` is absent. This is the critical missing comparison because local pass rate and local token count are forbidden proxies for the paid-token savings claim.

2. **14B-only sweep:** no artifact measures `qwen2.5-coder:14b` on the task pack. The 7B-vs-14B tradeoff remains a hypothesis.

3. **Expanded task pack:** only three tasks exist, all Python. The contract and reports call for at least 10 mixed tasks, including Python and Rust, write-from-scratch, edit-existing, and context-paths-required cases.

4. **Comparable assisted-frontier overhead:** current Awl arm records local worker tokens and wall time. It does not fully record paid frontier tokens spent packaging the request, reading Awl's compact result, deciding acceptance, or recovering from failure.

5. **Failure categories:** failures are not yet labeled as `format`, `schema`, `preflight`, `verify`, `timeout`, `network`, or `unknown`. Without these labels, opt-up guidance cannot distinguish model weakness from tool/protocol issues.

6. **Split price accounting:** result JSON contains prompt/completion token split, but current tally and cost-report scripts still use blended cost flags. Any cost result before the split patch should be treated as approximate.

7. **Statistical uncertainty:** no confidence intervals, sensitivity analysis, or repeated runs exist. With three tasks, the observed 67% pass rate is too fragile to generalize.

8. **Task-selection validity:** the task pack needs a documented rationale that it represents real bounded Claude/Codex delegation opportunities without being tuned to either 7B or 14B.

9. **Verifier strength:** local tests are the correctness oracle. Weak tests could inflate Awl pass rate while shifting semantic review back to the frontier.

10. **License metadata:** MIT versus AGPL conflict should be resolved before public reports or release metadata cite the project license.

## External / Background Context Needed

These are not prerequisites for the next local code patches, but they are needed before publication-quality conclusions:

| Needed Context | Why It Matters | Minimum Acceptable Source |
|---|---|---|
| Frontier model pricing and tokenizer behavior for the actual Claude/Codex model used in the baseline | Cost savings depend on input/output prices and tokenization; the reports cite Opus 4.7-style $5/$25 rates, but this survey did not verify an external rate card. | Official vendor pricing and tokenizer documentation captured with date. |
| Ollama and Qwen model tags/model-card details | The experiment should name exact local models, quantization, and runtime assumptions. | Official Ollama library page and/or Qwen model card for each exact tag. |
| MCP protocol and client behavior for Claude/Codex | Awl is an MCP server; client-side token overhead and tool result handling affect savings. | Official MCP/client documentation or measured client transcripts. |
| Structured-output API semantics for the Ollama OpenAI-compatible path | Awl relies on JSON schema response discipline; compatibility limits affect failure rates. | Official Ollama/OpenAI-compatible API docs plus local conformance tests. |
| Benchmark methodology for coding-agent evaluation | Prevents overclaiming from small, selected task packs. | Established coding-benchmark practice or a project-specific preregistered evaluation protocol. |
| Statistical treatment for small task packs | Needed to report uncertainty honestly around pass rate and savings. | Standard binomial/bootstrap confidence interval method documented in the Step 1 report. |

## Open Questions

1. **Can Awl show real paid-token savings on the same task pack?**  
   This remains open until direct frontier baseline tokens are collected and compared with Awl-assisted paid frontier overhead plus local result outcomes.

2. **Does 14B improve pass rate enough to justify its latency and larger local model cost?**  
   The current evidence only shows a 7B trailing-newline failure and two 7B successes. No 14B pass/fail or wall-time data exists.

3. **Can frontier-side model selection be predicted from task features?**  
   The approved design is no auto-escalation; the frontier picks 7B or 14B upfront. The project still needs evidence linking task features to expected 7B failure or 14B benefit.

4. **How much paid frontier overhead does Awl add even when local work succeeds?**  
   Packaging, compact result review, and final acceptance cost paid tokens. Existing local-arm artifacts do not measure that overhead.

5. **Which failures are tooling failures versus model capability gaps?**  
   `01_string_helper` looks like a verify/capability failure, but the telemetry does not yet carry explicit categories.

6. **Will the expanded task pack remain representative and unfitted?**  
   The task pack must grow, but new tasks should not be selected or tuned to manufacture 7B success.

## Planning Implications

- Treat `experiments/results/awl_arm.jsonl` as a partial internal baseline, not as evidence of savings.
- Prioritize reliability patches that prevent measuring the wrong regime: one default apply+verify attempt, model override, failure categories, and split cost accounting.
- Before declaring Step 1 complete, require same-task artifacts for direct frontier baseline, 7B-only Awl, and 14B-only Awl.
- Keep local pass rate separate from paid frontier savings. Passing local tasks are useful only if the frontier session actually spends fewer paid tokens end to end.
- Preserve the `01_string_helper` failure as a valuable disconfirming example; do not tune it away.
- Resolve the license conflict before public or release-facing writeups.

## Sources

- `GPD/state.json` -- authoritative project contract, observables, deliverables, references, forbidden proxies, uncertainty markers, and open questions.
- `GPD/PROJECT.md` -- human-readable scoping summary and key decisions.
- `HANDOFF_TO_GPD.md` -- project map, current mission, hard constraints, required reading order, and work queue.
- `REPORT_DISPATCH_RELIABILITY.md` -- current reliability research artifact, deterministic 7B failure interpretation, approved patch list, and Step 1 continuation plan.
- `UPDATED_PROGRESS_REPORT.md` -- historical progress against the original corrective plan and readiness claim for controlled testing.
- `experiments/README.md` -- A/B experiment protocol, baseline CSV schema, task constraints, and pass/fail thresholds.
- `experiments/results/awl_arm.jsonl` -- only current aggregate local-arm benchmark data.
- `experiments/results/01_string_helper.json` -- concrete repeated 7B verify failure.
- `experiments/results/02_validate_input.json` -- first-try 7B passing Python write-from-scratch result.
- `experiments/results/03_fix_off_by_one.json` -- first-try 7B passing Python edit-existing result.
- `GPD/research-map/FORMALISM.md` -- prior map of project metrics, regimes, approximations, and missing baseline caveat.
- `GPD/research-map/REFERENCES.md` -- prior anchor registry and benchmark-target map; note that its statement that `GPD/state.json` is missing is stale.
- `GPD/research-map/ARCHITECTURE.md` -- computational architecture summary.
- `GPD/research-map/CONCERNS.md` -- current gaps and issue prioritization; note that its statement that `GPD/state.json` is missing is stale.
- `src/dispatch.rs` -- source cross-check for current retry default and dispatch options.
- `src/defaults.rs` -- model default mapping: 14B agent, 7B implementation, 3B verification.
- `experiments/tasks/*/task.json` -- current retry overrides and task-pack scope.
- `experiments/tally.py` -- current tally behavior and blended cost flag.
- `scripts/dispatch_cost_report.py` -- current dispatch-cost behavior and blended frontier-cost flag.
