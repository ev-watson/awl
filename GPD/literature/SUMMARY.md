---
template_version: 1
research_type: "Literature Survey Summary"
domain: "Awl frontier-token savings and bounded local coding delegation"
researched: "2026-05-01"
confidence: "MEDIUM"
---

# Research Summary

**Project:** Awl Frontier-Token Savings Study  
**Domain:** software-systems evaluation of bounded local coding delegation  
**Researched:** 2026-05-01  
**Confidence:** MEDIUM overall. Local artifact facts are HIGH confidence; savings, 14B performance, and model-selection guidance remain unproven.

## Executive Summary

Awl is not being evaluated as a general local-model benchmark. The project question is narrower and stricter: can a bounded local Ollama worker reduce paid frontier tokens in real Claude/Codex coding sessions while preserving acceptable pass rate, reliability, and caller-visible safety? The current literature survey supports a reliability-first experiment design, not a savings conclusion.

The only measured internal result is a partial 7B local Awl arm: three Python tasks, two passing, one deterministic trailing-newline verification failure, and 8601 local worker tokens. This is useful pilot evidence that the dispatch pipeline can run bounded tasks and expose real failure modes, but it cannot establish frontier-token savings because the direct frontier baseline is missing and no 14B-only sweep exists.

The next roadmap should therefore prioritize measurement correctness before expanding claims: patch retry defaults, per-dispatch model override, failure taxonomy, and split input/output cost accounting; freeze a larger mixed task pack; collect the same-task frontier baseline; then run separate 7B-only and 14B-only Awl sweeps. False progress to reject: local model quality, local pass rate, blended token-cost arithmetic, or qualitative convenience without same-task frontier-baseline token accounting.

## Key Findings

### Known Results

The project contract in `GPD/state.json` and `GPD/PROJECT.md` defines two decisive outputs:

- `claim-reliability-prep`: implement the pre-Step-1 reliability patches without weakening dispatch safety, local-only execution, lint, CI, or branch-protection constraints.
- `claim-step1-answer`: produce either a defensible positive result, with at least one Awl configuration showing at least 25% frontier-token reduction at at least 60% Awl pass rate, or a defensible negative result with documented blockers.

Current file-backed results:

- `experiments/results/awl_arm.jsonl` records a partial 7B local arm with 3 attempted tasks, 2 passes, and 1 failure.
- `01_string_helper` failed twice under 7B on trailing-newline preservation and consumed 4264 local worker tokens. This supports the decision to avoid same-model verify retries by default.
- `02_validate_input` and `03_fix_off_by_one` passed first try under 7B.
- `experiments/results/baseline.csv` is missing, so no direct frontier-token reduction can be computed.
- No 14B-only sweep artifact exists, so claims about 14B improving pass rate or cost-effectiveness are speculative.
- Current source and task specs still have retry-policy issues: source defaults verified apply to 2 attempts, and task JSON files explicitly set `max_attempts` values of 2 or 3.
- Current reporting scripts use blended token-cost flags even though the contract requires split input/output token costs.

### Computational Approaches

The recommended computational approach is to keep the existing Rust CLI/MCP dispatch stack and patch its measurement-critical surfaces rather than introduce a new harness. `src/dispatch.rs` should remain the owner of model selection, apply/verify/rollback, JSON validation, telemetry, and dispatch logs. `src/main.rs`, `src/mcp_server.rs`, and `src/tools.rs` should surface the same dispatch contract through CLI and MCP. `experiments/run_awl_arm.sh`, `experiments/tally.py`, and `scripts/dispatch_cost_report.py` should remain the lightweight experiment and reporting layer.

Core approach:

- Paired same-task A/B comparison: compare direct frontier work, 7B-only Awl, and 14B-only Awl on the same frozen task pack.
- Intent-to-treat accounting: include every attempted task, including failed local dispatches and recovery costs.
- Apply/verify/rollback: keep failed local work from dirtying the workspace, but do not treat rollback as making failures free.
- Failure taxonomy: classify failures as format, schema, preflight, verify, timeout, network, model/status, or unknown.
- Split input/output token accounting: compute paid frontier cost from separate input and output token counts, not a blended rate.
- Model-selection calibration: produce frontier-side guidance only after 7B-only and 14B-only sweeps use identical tasks and accounting.

### Prior Work Landscape

The strongest prior work for this project is internal, not external: `HANDOFF_TO_GPD.md`, `REPORT_DISPATCH_RELIABILITY.md`, `UPDATED_PROGRESS_REPORT.md`, `experiments/README.md`, and the existing pilot result files. These establish the current state of Awl dispatch v2, the approved reliability patch list, the A/B experiment threshold, and the specific ways a result can become false progress.

External methodology anchors in `METHODS.md` are useful as design discipline rather than direct evidence for Awl: HumanEval, EvalPlus, SWE-bench, RealHumanEval, and controlled-experiment methodology support using same-task comparisons, stronger tests, real workflow overhead accounting, and transparent uncertainty. They do not prove Awl savings.

Must reproduce or collect before a Step 1 claim:

- Same-task direct frontier baseline with input/output token counts.
- Same frozen task pack run through 7B-only Awl and 14B-only Awl.
- Pass rate, failure category, attempts, wall time, local worker tokens, and frontier coordination overhead for every task.
- Step 1 report in `experiments/results/report.md` that can state either a positive threshold result or a negative result without proxy metrics.

Potential contributions after required evidence exists:

- Defensible 7B versus 14B model-selection guidance for bounded local coding tasks.
- Failure-category evidence distinguishing model capability gaps from tooling/schema/preflight failures.
- A reproducible small-scale methodology for measuring frontier-token savings from local coding delegation.

Defer:

- Python preflight, streaming dispatch output, dispatch caching, and per-task local-token ceilings until Step 1 shows they are needed.
- Dynamic 7B-to-14B auto-escalation; the approved design says the frontier chooses the model upfront.
- Public dollar-savings claims until provider prices, tokenizer behavior, and raw split token counts are captured with dates.

### Methods and Tools

Recommended methods are empirical and operational. The study should use paired task-level A/B measurement, pre-registered task inclusion, fixed retry policy, identical verifier commands across arms, split token accounting, failure taxonomy, and targeted software tests for the dispatch path and reporting scripts. The key measurement equation is direct frontier cost minus Awl-assisted frontier cost, where both sides use split input/output token prices. Local Ollama tokens should be reported as load and latency, but they are not paid frontier tokens unless an explicit local compute-cost model is introduced.

Major components:

1. Rust dispatch pipeline - bounded local worker execution, structured output, apply/verify/rollback, telemetry.
2. Experiment task pack - frozen task definitions, setup scripts, verifiers, language/type/difficulty strata.
3. Baseline ledger - direct frontier input/output tokens, pass/fail, and wall time per task.
4. Awl arm ledgers - 7B-only and 14B-only results, including frontier coordination overhead and local worker telemetry.
5. Reporting scripts - tally pass rate, token reduction, split cost, failure categories, and confidence/uncertainty.
6. CI and fixture tests - protect dispatch contract, safety rules, structured-output recovery, tally math, and cost-report parsing.

### Critical Pitfalls

1. **Treating Awl pass rate as token-savings evidence** - avoid by requiring same-task direct frontier baseline rows before any savings conclusion.
2. **Omitting frontier coordination overhead** - record paid tokens for dispatch packaging, compact-result review, acceptance, log inspection, and recovery.
3. **Measuring stale retry behavior** - patch the default to one verified apply attempt and audit task JSON `max_attempts` before reruns.
4. **Tuning the task pack toward 7B** - pre-register at least 10 mixed Python/Rust tasks and preserve meaningful edge cases such as the trailing-newline failure.
5. **Hiding model configuration drift** - add explicit per-dispatch model override and separate 7B-only and 14B-only result artifacts.
6. **Using blended token pricing** - store and report input/output tokens separately for both direct and assisted arms.
7. **Trusting weak verifiers too much** - strengthen task tests and record any residual frontier review needed after a passing dispatch.
8. **Letting CI miss the real dispatch/report paths** - add mocked dispatch integration tests plus non-empty JSONL/CSV fixtures for reporting scripts.
9. **Confusing rollback with zero cost** - rollback protects the worktree, but failed local attempts still cost local time and paid frontier recovery.
10. **Leaving license metadata inconsistent** - resolve MIT versus AGPL notes before public release or publication-facing claims.

## Implications for Research Plan

### Phase 1: Reliability And Measurement Patches

**Rationale:** The current harness can otherwise measure the wrong regime: same-model retries, non-reproducible model selection, missing failure categories, and blended cost accounting.  
**Delivers:** default verified apply attempts set to one; explicit model override through CLI/MCP/experiment driver; first-class `failure_category`; split input/output cost flags and outputs; focused tests.  
**Validates:** local `cargo fmt`, `cargo test`, `cargo clippy --workspace --all-targets -- -D warnings`, plus fixture tests for tally and cost reports.  
**Avoids:** false improvements caused by stale retry policy, hidden model config, or blended-price savings estimates.

### Phase 2: Task-Pack Freeze And Baseline Collection

**Rationale:** No Step 1 conclusion is possible without a same-task direct frontier baseline and a task pack large enough to avoid the current three-task Python-only bias.  
**Delivers:** at least 10 mixed Python/Rust tasks, frozen task IDs/specs/tests, verification-strength rubric, direct frontier baseline with input/output tokens, pass/fail, and wall time.  
**Validates:** exact task-ID set equality across baseline and Awl arms; same verifier commands; documented task strata and inclusion rationale.  
**Avoids:** cherry-picking, task-ID mismatch, weak oracle inflation, and baseline absence.

### Phase 3: Fixed-Model Awl Sweeps

**Rationale:** The approved comparison is not dynamic escalation. It is two separate fixed-model evaluations against the same frozen task pack.  
**Delivers:** 7B-only result artifact, 14B-only result artifact, per-task attempts/pass/failure category/wall time/local tokens, and assisted-arm frontier overhead ledger.  
**Uses:** model override, apply/verify/rollback, failure taxonomy, split token accounting, distinct result output paths.  
**Builds on:** Phase 1 dispatch/reporting patches and Phase 2 frozen baseline.

### Phase 4: Step 1 Report And Routing Guidance

**Rationale:** Only after comparable data exists can the project answer the core question without proxy metrics.  
**Delivers:** `experiments/results/report.md` with same-task direct versus Awl comparisons, 7B-only and 14B-only aggregates, input/output cost accounting, pass rates, failure categories, and frontier-side model-selection guidance.  
**Validates:** either at least one configuration achieves >=25% frontier-token reduction at >=60% Awl pass rate, or the report documents a negative/inconclusive result with blockers.  
**Avoids:** claiming savings from local token counts, pass rate alone, or qualitative convenience.

### Phase 5: Cleanup And Publication Readiness

**Rationale:** Public-facing conclusions need stable metadata and dated external assumptions.  
**Delivers:** license reconciliation, dated provider pricing/tokenizer assumptions, exact Ollama model tags and quantization notes, and any release/documentation cleanup needed.  
**Builds on:** the Step 1 result, not the pilot data alone.

### Phase Ordering Rationale

- Reliability and telemetry patches come before new runs because otherwise new measurements can inherit known distortions.
- Baseline collection and task-pack freeze come before model sweeps because pass rate without direct baseline is a forbidden proxy for savings.
- 7B and 14B sweeps must be fixed-model and same-task to separate model capability from task difficulty.
- Reporting comes last so positive, negative, or inconclusive status is tied to artifacts rather than expectation.

### Phases Requiring Deep Investigation

- **Phase 2:** task-pack design and baseline collection need care because task selection and token accounting can dominate the result.
- **Phase 3:** 14B sweep interpretation is currently speculative; model identity, latency, pass rate, and failure mix must be measured.
- **Phase 4:** assisted-frontier overhead accounting needs explicit ledgers so the local worker's compact return does not hide paid coordination costs.

Phases with established methodology:

- **Phase 1:** patch targets are concrete and source-backed, though they still require careful tests.
- **Phase 5:** license and metadata reconciliation are straightforward cleanup once the authority for repository metadata is decided.

## Confidence Assessment

| Area | Confidence | Notes |
|---|---|---|
| Local artifact facts | HIGH | Survey files agree on pilot results, missing baseline, missing 14B sweep, and patch targets. |
| Computational approach | HIGH | Existing Rust/CLI/MCP and experiment-script paths are identified and appropriate. |
| Methods | MEDIUM-HIGH | Paired A/B, intent-to-treat, split cost, and benchmark controls are well supported, but final schema details still need implementation. |
| Prior work | MEDIUM | Internal anchors are strong; external methodology anchors inform evaluation design but are not Awl-specific evidence. |
| Pitfalls | HIGH | False-progress paths are explicit and repeatedly supported by project artifacts. |
| Savings conclusion | LOW | No direct frontier baseline and no 14B sweep exist. |
| 14B guidance | LOW | The 14B comparison is approved but unmeasured. |

**Overall confidence:** MEDIUM. The plan direction is well grounded; the central empirical answer is not yet available.

### Gaps to Address

- **Missing direct frontier baseline:** create `experiments/results/baseline.csv` or a richer split-token replacement before any savings claim.
- **Missing 14B-only sweep:** implement explicit model override and run the same frozen task pack under `qwen2.5-coder:14b`.
- **Assisted-frontier overhead not yet measured:** record paid frontier tokens for packaging, result review, acceptance, log inspection, and recovery.
- **Task pack too small and narrow:** expand beyond three Python tasks to a pre-registered mixed Python/Rust pack.
- **Retry-policy contamination:** remove or justify task-level `max_attempts` overrides before comparing new data.
- **Cost scripts still blended:** update tally and dispatch-cost reports to split input/output costs.
- **Failure taxonomy absent:** add `failure_category` to dispatch outputs and reports.
- **Verifier strength uneven:** document per-task verifier coverage and strengthen tests before model comparison.
- **License conflict unresolved:** reconcile MIT repository metadata with the AGPL note in the handoff before public claims.

## Sources

### Primary Project Anchors

- `GPD/state.json` - active project contract, observables, claims, deliverables, forbidden proxies, and uncertainty markers.
- `GPD/PROJECT.md` - human-readable project scope, known results, thresholds, decisions, and constraints.
- `GPD/literature/PRIOR-WORK.md` - internal known results, missing comparison data, and planned techniques.
- `GPD/literature/METHODS.md` - recommended empirical methods, telemetry schema, validation strategy, and benchmark controls.
- `GPD/literature/COMPUTATIONAL.md` - implementation paths, experiment data flow, model sweep plan, and resource estimates.
- `GPD/literature/PITFALLS.md` - false-progress risks, measurement traps, CI traps, and phase mapping.

### Required Carry-Forward Anchors

- `HANDOFF_TO_GPD.md` - user-supplied project map, constraints, work queue, and stop conditions.
- `REPORT_DISPATCH_RELIABILITY.md` - approved reliability patch rationale, deterministic 7B failure interpretation, and Step 1 continuation plan.
- `UPDATED_PROGRESS_REPORT.md` - prior progress and warning that controlled testing is justified but savings are not proven.
- `experiments/README.md` - A/B experiment protocol, baseline expectations, and task constraints.
- `experiments/results/awl_arm.jsonl` - only current local-arm aggregate data.
- `experiments/results/01_string_helper.json`, `02_validate_input.json`, `03_fix_off_by_one.json` - per-task pilot evidence.

### External Methodology Anchors From `METHODS.md`

- Chen et al., "Evaluating Large Language Models Trained on Code", arXiv:2107.03374.
- Liu et al., "Is Your Code Generated by ChatGPT Really Correct?", arXiv:2305.01210.
- Jimenez et al., "SWE-bench: Can Language Models Resolve Real-World GitHub Issues?", arXiv:2310.06770.
- Mozannar et al., "The RealHumanEval", arXiv:2404.02806.
- Kohavi, Tang, and Xu, *Trustworthy Online Controlled Experiments*, Cambridge University Press, 2020.

## Readiness For Roadmap

Ready for roadmap creation: yes, with blockers carried forward. The roadmap should not start from "prove savings"; it should start from "make the measurement claim testable, then accept positive, negative, or inconclusive results according to the approved contract."

---

_Research analysis completed: 2026-05-01_  
_Ready for research plan: yes, with missing baseline and missing 14B sweep explicit_
