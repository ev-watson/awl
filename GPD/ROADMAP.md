# Research Roadmap: Awl Frontier-Token Savings Study

**Created:** 2026-05-01
**Mode:** Shallow roadmap after post-scope continuation handoff
**Core research question:** Can Awl's bounded local Ollama dispatch workflow produce defensible frontier-token savings in real Claude/Codex coding sessions without unacceptable pass-rate or reliability regressions?

## Roadmap Principles

- The authoritative project contract is `GPD/state.json` field `project_contract`; this roadmap does not rewrite or supersede it.
- The measurement claim stays undecided until same-task direct frontier baseline data, 7B-only Awl data, and 14B-only Awl data exist.
- Every primary requirement in `GPD/REQUIREMENTS.md` maps to exactly one phase.
- False progress to reject across the roadmap: local model quality alone, pass rate alone, blended token-cost accounting after split costs are required, and qualitative convenience without same-task frontier-baseline token accounting.

## Requirement Coverage Summary

| Phase | Requirements |
| ----- | ------------ |
| Phase 1: Reliability and Measurement Patches | CODE-01, CODE-02, CODE-03, CODE-04, VAL-01 |
| Phase 2: Task Pack and Baseline | EXP-01, EXP-02 |
| Phase 3: Fixed-Model Awl Sweeps | EXP-03, VAL-03 |
| Phase 4: Step 1 Report and Guidance | VAL-02, DOC-01, DOC-03 |
| Phase 5: Metadata and Publication Readiness | DOC-02 |

Coverage: 13 primary requirements mapped to 5 phases; unmapped requirements: 0.

---

## Phase 1: Reliability and Measurement Patches

**Goal:** Implement the measurement-critical reliability patches before Step 1 resumes, so subsequent baseline and model-sweep results are not contaminated by stale retry behavior, hidden model selection, missing failure categories, or blended-cost arithmetic.

**Depends on:** Authoritative project contract in `GPD/state.json`; `HANDOFF_TO_GPD.md`; `REPORT_DISPATCH_RELIABILITY.md`; `UPDATED_PROGRESS_REPORT.md`; `experiments/README.md`; existing pilot artifacts in `experiments/results/`.

**Requirements:** CODE-01, CODE-02, CODE-03, CODE-04, VAL-01

**Objective IDs:** obj-retry-default, obj-model-override, obj-failure-category, obj-split-cost-accounting, obj-reliability-validation

### Contract Coverage

**Decisive contract items advanced**

- `claim-reliability-prep`: architectural patches needed before Step 1 are implemented without weakening dispatch safety, local-only execution, lint, CI, or branch-protection constraints.
- `test-reliability-ci`: local and GitHub CI evidence must support reliability patch completion.

**Deliverables advanced**

- `deliv-reliability-patches`
  - `src/dispatch.rs` retry default change and tests.
  - `DispatchOptions` model override through CLI and MCP schema.
  - `failure_category` in `apply_result` and `error_result` telemetry.
  - Input/output token cost flags in `experiments/tally.py` and `scripts/dispatch_cost_report.py`.

**Anchor coverage**

- `ref-handoff`: read and use for hard constraints, branch workflow, and stop conditions.
- `ref-reliability-report`: read, use, and compare against the approved patch list and deterministic 7B failure interpretation.
- `ref-progress-report`: preserve the distinction between controlled testing readiness and unproven savings.
- `ref-experiment-readme`: preserve benchmark protocol compatibility for later phases.
- `ref-partial-awl-results`: use only as pilot evidence and retry-policy motivation, not as savings evidence.

**Forbidden proxies advanced or blocked**

- `fp-weakened-gates`: explicitly blocked by VAL-01 and `test-reliability-ci`.
- `fp-local-quality-only`: not advanced; Phase 1 produces instrumentation and patch readiness, not a savings result.
- `fp-easy-task-tuning`: not touched in this phase; task definitions are deferred to Phase 2.

### Success Criteria

1. `effective_max_attempts` defaults verified apply to one attempt unless the caller explicitly opts into a higher `max_attempts`, with tests covering default and override behavior.
2. Per-dispatch model override is available through `DispatchOptions`, CLI dispatch input, MCP dispatch tool schema, and `experiments/run_awl_arm.sh` via `AWL_MODEL_OVERRIDE`, while configured level defaults still work when no override is set.
3. Terminal dispatch outcomes expose `failure_category` for `apply_result` and `error_result`, covering at least format, schema, preflight, verify, timeout, network, and unknown paths.
4. `experiments/tally.py` and `scripts/dispatch_cost_report.py` compute split input/output token costs from available prompt/input and completion/output token fields, while preserving enough total-token readability to audit old artifacts.
5. Local verification gates pass for the patch set: `cargo fmt`, `cargo test`, `cargo clippy --workspace --all-targets -- -D warnings`, plus reporting-script fixture checks where applicable; any required GitHub CI remains passing before merge.

**Plans:** Placeholder plans to be created by `plan-phase 1`.

- [ ] Plan 1: Retry default and model override plumbing.
- [ ] Plan 2: Failure-category telemetry and dispatch result schema updates.
- [ ] Plan 3: Split input/output cost accounting in experiment/reporting scripts.
- [ ] Plan 4: Validation pass, regression audit, and CI evidence collection.

---

## Phase 2: Task Pack and Baseline

**Goal:** Freeze a fair mixed benchmark task pack and collect same-task direct frontier baseline data with split input/output token counts.

**Objective IDs:** obj-task-pack-freeze, obj-direct-frontier-baseline

**Contract/anchor/proxy labels:** `claim-step1-answer`; `deliv-step1-report`; `test-step1-threshold`; `ref-experiment-readme`; `ref-handoff`; `ref-reliability-report`; block `fp-local-quality-only`, `fp-easy-task-tuning`.

**Plans:** 0 plans

- [ ] TBD (run plan-phase 2 to break down)

---

## Phase 3: Fixed-Model Awl Sweeps

**Goal:** Run fixed 7B-only and fixed 14B-only Awl sweeps over the same frozen task pack and verify task-level comparability.

**Objective IDs:** obj-7b-sweep, obj-14b-sweep, obj-comparability-audit

**Contract/anchor/proxy labels:** `claim-step1-answer`; `deliv-step1-report`; `test-step1-threshold`; `ref-partial-awl-results`; `ref-reliability-report`; block post hoc task exclusion and model-configuration drift.

**Plans:** 0 plans

- [ ] TBD (run plan-phase 3 to break down)

---

## Phase 4: Step 1 Report and Guidance

**Goal:** Produce the Step 1 report and frontier-side 7B versus 14B routing guidance from same-task baseline, 7B-only, and 14B-only evidence.

**Objective IDs:** obj-step1-threshold-verdict, obj-step1-report, obj-frontier-guidance, obj-doc-updates

**Contract/anchor/proxy labels:** `claim-step1-answer`; `deliv-step1-report`; `test-step1-threshold`; `obs-token-savings`; `obs-awl-pass-rate`; `obs-failure-category`; `obs-cost-split`; block pass-rate-only or local-token-only success claims.

**Plans:** 0 plans

- [ ] TBD (run plan-phase 4 to break down)

---

## Phase 5: Metadata and Publication Readiness

**Goal:** Resolve release/publication metadata conflicts and record dated assumptions before any public savings claim or release-facing summary.

**Objective IDs:** obj-license-reconciliation, obj-publication-metadata

**Contract/anchor/proxy labels:** open question on MIT versus AGPL metadata; `ref-handoff`; `ref-progress-report`; block public claims that rely on unresolved license, pricing, tokenizer, or model-tag assumptions.

**Plans:** 0 plans

- [ ] TBD (run plan-phase 5 to break down)

---

## Anchor Handoffs

- Phase 1 hands off patched retry behavior, model override, failure taxonomy, split-cost scripts, and validation evidence to Phase 2.
- Phase 2 hands off frozen task IDs, verifier semantics, and direct frontier baseline rows to Phase 3.
- Phase 3 hands off same-task 7B-only and 14B-only result artifacts plus comparability checks to Phase 4.
- Phase 4 hands off the Step 1 verdict and guidance constraints to Phase 5.
- Phase 5 must not upgrade the result into a public claim unless Phase 4 provides the required threshold evidence or explicitly negative/inconclusive blockers.

## Current Blockers Carried Forward

- No direct frontier baseline data exists yet; `experiments/results/baseline.csv` or an approved split-token replacement must be collected before any savings claim.
- No 14B-only sweep artifact exists.
- The expanded task pack is not frozen and must not be tuned to make 7B pass.
- MIT versus AGPL metadata conflict remains unresolved before release or publication metadata is cited.
