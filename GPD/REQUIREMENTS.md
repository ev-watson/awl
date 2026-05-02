# Requirements: Awl Frontier-Token Savings Study

**Defined:** 2026-05-01
**Core Research Question:** Can Awl's bounded local Ollama dispatch workflow produce defensible frontier-token savings in real Claude/Codex coding sessions without unacceptable pass-rate or reliability regressions?

## Primary Requirements

Requirements for the main research deliverable. Each requirement maps to exactly one roadmap phase.

### Reliability Patches

- [ ] **CODE-01**: Change the verified apply default attempt count from two to one in `effective_max_attempts`, while preserving caller opt-in for higher `max_attempts` and updating tests for the new default.
- [ ] **CODE-02**: Add a per-dispatch model override through `DispatchOptions`, CLI dispatch input, MCP dispatch tool schema, and `experiments/run_awl_arm.sh` via `AWL_MODEL_OVERRIDE`, with configured level defaults as fallback.
- [ ] **CODE-03**: Add first-class `failure_category` telemetry to `apply_result` and `error_result`, using categories that distinguish at least format, schema, preflight, verify, timeout, network, and unknown failures.
- [ ] **CODE-04**: Replace blended token-cost accounting in `experiments/tally.py` and `scripts/dispatch_cost_report.py` with split input/output token pricing based on available prompt/input and completion/output token fields.

### Experiment Design

- [ ] **EXP-01**: Freeze an expanded benchmark task pack of at least ten bounded tasks, including Python and Rust, write-from-scratch, edit-existing, and context-paths-required cases, without tuning tasks to make 7B pass.
- [ ] **EXP-02**: Collect same-task direct frontier baseline data with task ID, pass/fail, wall time, and input/output token counts suitable for split-cost comparison.
- [ ] **EXP-03**: Run fixed 7B-only and fixed 14B-only Awl sweeps over the same frozen task pack, producing distinct result artifacts and preserving per-task pass/fail, attempts, wall time, local model usage, and failure category.

### Validation

- [ ] **VAL-01**: Verify every reliability patch with `cargo fmt`, `cargo test`, `cargo clippy --workspace --all-targets -- -D warnings`, and required GitHub CI checks without weakening existing gates.
- [ ] **VAL-02**: Verify the Step 1 result against the approved threshold: at least one configuration shows `>=25%` frontier-token reduction at `>=60%` Awl pass rate, or the report clearly documents a negative or inconclusive result with blockers.
- [ ] **VAL-03**: Audit task-level result comparability so the direct frontier baseline, 7B-only Awl arm, and 14B-only Awl arm use the same frozen task IDs and verifier semantics.

### Documentation and Guidance

- [ ] **DOC-01**: Produce `experiments/results/report.md` with same-task comparisons, 7B-only and 14B-only aggregate results, split input/output cost accounting, pass rates, failure categories, and frontier-side model-selection guidance.
- [ ] **DOC-02**: Resolve the MIT versus AGPL metadata conflict between repository files and `HANDOFF_TO_GPD.md` before any release or public savings claim.
- [ ] **DOC-03**: Update user-facing dispatch or experiment documentation to reflect the retry default, per-dispatch model override, split token pricing, and any benchmark protocol changes.

## Follow-up Requirements

Deferred to future work or follow-up investigation. These are acknowledged but not part of the current roadmap unless the approved scope changes.

### Deferred Tooling

- **FUT-01**: Add Python preflight only if Step 1 failures show that Python import or repo-context preflight would have prevented real failures.
- **FUT-02**: Add streaming dispatch output only if compact result latency or observability becomes a measured bottleneck.
- **FUT-03**: Add dispatch caching only if repeated task fingerprints appear in realistic workflows and the cache can be validated safely.
- **FUT-04**: Add per-dispatch local-token ceilings only if local token consumption becomes a decision-relevant cost or latency constraint.

### Extended Measurement

- **FUT-05**: Measure dynamic frontier-side 7B/14B routing after fixed 7B-only and 14B-only sweeps establish calibration data.
- **FUT-06**: Add broader repository/language task packs after Step 1 produces a trustworthy result or identifies blockers.

## Out of Scope

Explicitly excluded to prevent scope creep.

| Topic | Reason |
| ----- | ------ |
| Auto-escalation from 7B to 14B inside Awl | The approved design says the frontier selects the model upfront per dispatch. |
| Making existing benchmark tasks easier | This would bias the pass-rate measurement and erase capability-gap evidence. |
| Replacing the OpenAI-compatible JSON-schema response protocol | Structured-output discipline is load-bearing and requires explicit user authorization to change. |
| Weakening CI, lint, hooks, or branch protection | Passing gates are part of the reliability deliverable. |
| External paid APIs in the worker path | Awl's value proposition depends on local-only execution. |

## Accuracy and Validation Criteria

Standards that results must meet before being considered complete.

| Requirement | Accuracy Target | Validation Method |
| ----------- | --------------- | ----------------- |
| CODE-01 | Default verified apply attempts are one unless caller explicitly opts into more. | Unit tests around `effective_max_attempts` and targeted review of task JSON overrides before sweeps. |
| CODE-02 | Override beats level default; unset override preserves configured level default. | Rust unit/integration tests plus CLI/MCP schema inspection and experiment harness dry run. |
| CODE-03 | Every dispatch terminal failure path assigns an expected category or explicitly `unknown`. | Unit tests or fixture logs for representative category paths and cost-report aggregation. |
| CODE-04 | Cost reports use split input/output rates and preserve legacy total-token readability where useful. | Python fixture tests for prompt/completion usage fields, missing fields, and aggregate cost totals. |
| EXP-01 | At least ten tasks, mixed Python/Rust and mixed task types, with frozen IDs and verifiers. | Manifest review against task strata and no post hoc exclusion without recorded rationale. |
| EXP-02 | Every baseline row has same task ID set, pass/fail, wall time, input tokens, and output tokens. | Baseline CSV/schema validation before tallying. |
| EXP-03 | 7B-only and 14B-only arms use the identical frozen task pack and distinct result artifacts. | Task-ID equality checks and model identity recorded in result files. |
| VAL-02 | Positive claim requires `>=25%` frontier-token reduction and `>=60%` pass rate; otherwise report negative/inconclusive status. | `experiments/results/report.md` reviewed against same-task ledgers and approved contract. |

## Contract Coverage

Make the approved scoping contract visible in requirement form so planning does not drift.

| Requirement | Decisive Output / Deliverable | Anchor / Benchmark / Reference | Prior Inputs / Baselines | False Progress To Reject |
| ----------- | ----------------------------- | ------------------------------ | ------------------------ | ------------------------ |
| CODE-01 | `deliv-reliability-patches` | `REPORT_DISPATCH_RELIABILITY.md` | `experiments/results/01_string_helper.json` | Same-model retry appears useful without measuring repeated failure cost. |
| CODE-02 | `deliv-reliability-patches` | `REPORT_DISPATCH_RELIABILITY.md` | `src/defaults.rs`, `experiments/run_awl_arm.sh` | Hidden environment-only model drift. |
| CODE-03 | `deliv-reliability-patches` | `REPORT_DISPATCH_RELIABILITY.md` | `scripts/dispatch_cost_report.py` | Unclassified failures forcing qualitative interpretation. |
| CODE-04 | `deliv-reliability-patches` | `REPORT_DISPATCH_RELIABILITY.md` | `experiments/tally.py`, `scripts/dispatch_cost_report.py` | Blended-cost arithmetic after split costs are required. |
| EXP-01 | `deliv-step1-report` | `experiments/README.md` | Existing three-task pilot pack | Task pack tuned toward 7B or too small to generalize. |
| EXP-02 | `deliv-step1-report` | `experiments/README.md` | Missing `experiments/results/baseline.csv` | Claiming savings without direct frontier baseline. |
| EXP-03 | `deliv-step1-report` | `REPORT_DISPATCH_RELIABILITY.md` | Missing 14B-only sweep | Speculative 14B guidance without same-task evidence. |
| VAL-02 | `claim-step1-answer` | `GPD/state.json` project contract | `experiments/results/awl_arm.jsonl` | Pass-rate-only or local-token-only success claims. |
| DOC-01 | `deliv-step1-report` | `GPD/literature/SUMMARY.md` | Same-task ledgers from baseline, 7B, and 14B arms | Qualitative convenience narrative without threshold verdict. |

## Traceability

Which phases cover which requirements. The roadmapper may revise phase names while preserving one-phase-per-requirement coverage.

| Requirement | Phase | Status |
| ----------- | ----- | ------ |
| CODE-01 | Phase 1: Reliability and Measurement Patches | Pending |
| CODE-02 | Phase 1: Reliability and Measurement Patches | Pending |
| CODE-03 | Phase 1: Reliability and Measurement Patches | Pending |
| CODE-04 | Phase 1: Reliability and Measurement Patches | Pending |
| EXP-01 | Phase 2: Task Pack and Baseline | Pending |
| EXP-02 | Phase 2: Task Pack and Baseline | Pending |
| EXP-03 | Phase 3: Fixed-Model Awl Sweeps | Pending |
| VAL-01 | Phase 1: Reliability and Measurement Patches | Pending |
| VAL-02 | Phase 4: Step 1 Report and Guidance | Pending |
| VAL-03 | Phase 3: Fixed-Model Awl Sweeps | Pending |
| DOC-01 | Phase 4: Step 1 Report and Guidance | Pending |
| DOC-02 | Phase 5: Metadata and Publication Readiness | Pending |
| DOC-03 | Phase 4: Step 1 Report and Guidance | Pending |

**Coverage:**

- Primary requirements: 13 total
- Mapped to phases: 13
- Unmapped: 0

---

_Requirements defined: 2026-05-01_
_Last updated: 2026-05-01 after roadmap traceability validation_
