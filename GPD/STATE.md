# Research State

## Project Reference

See: GPD/PROJECT.md (updated 2026-05-01)

**Machine-readable scoping contract:** `GPD/state.json` field `project_contract`

**Core research question:** Can Awl's bounded local Ollama dispatch workflow produce defensible frontier-token savings in real Claude/Codex coding sessions without unacceptable pass-rate or reliability regressions?
**Current focus:** Phase 1: Reliability and Measurement Patches

## Current Position

**Current Phase:** 01
**Current Phase Name:** Reliability and Measurement Patches
**Total Phases:** 5
**Current Plan:** 1
**Total Plans in Phase:** 4 placeholder plans
**Status:** Ready to plan
**Last Activity:** 2026-05-01
**Last Activity Description:** Created shallow staged roadmap from authoritative project contract and requirements traceability.

**Progress:** [░░░░░░░░░░] 0%

## Active Calculations

No active execution yet. Next work is planning Phase 1 reliability and measurement patches.

## Intermediate Results

None yet. Current pilot evidence remains external to state and is anchored in `experiments/results/awl_arm.jsonl` and per-task result JSON files.

## Open Questions

- What direct-frontier baseline token data should populate experiments/results/baseline.csv?
- Which additional tasks should extend experiments/tasks/ to at least 10 mixed Python/Rust tasks without biasing the benchmark?
- Does 14B improve pass rate enough to justify slower local execution and frontier session overhead?
- Cargo.toml and README.md indicate MIT licensing while HANDOFF_TO_GPD.md says AGPL-3.0; resolve before release or publication metadata is cited.

## Performance Metrics

| Label | Duration | Tasks | Files |
| ----- | -------- | ----- | ----- |
| -     | -        | -     | -     |

## Accumulated Context

### Decisions

Full log: `GPD/DECISIONS.md`

**Recent high-impact:**
- Project roadmap decomposes the current milestone into five phases: reliability patches, task pack/baseline, fixed-model sweeps, Step 1 report/guidance, and metadata/publication readiness.
- Phase 1 is the only fully detailed shallow-mode phase; later phases are compact stubs until planned individually.

### Active Approximations

| Approximation | Validity Range | Controlling Parameter | Current Value | Status |
| ------------- | -------------- | --------------------- | ------------- | ------ |
| Pilot Awl arm used only as reliability evidence, not savings evidence | Before same-task direct frontier baseline exists | Baseline availability | Missing baseline | Valid constraint |
| Split token pricing required for claims | Step 1 measurement and reporting | Separate input/output token fields | Not yet patched | Required |

**Convention Lock:**

- Metric signature: not set
- Fourier convention: not set
- Natural units: not set
- Gauge choice: not set
- Regularization scheme: not set
- Renormalization scheme: not set
- Coordinate system: not set
- Spin basis: not set
- State normalization: not set
- Coupling convention: not set
- Index positioning: not set
- Time ordering: not set
- Commutation convention: not set
- Levi-Civita sign: not set
- Generator normalization: not set
- Covariant derivative sign: not set
- Gamma matrix convention: not set
- Creation/annihilation order: not set

### Propagated Uncertainties

| Quantity | Current Value | Uncertainty | Last Updated (Phase) | Method |
| -------- | ------------- | ----------- | -------------------- | ------ |
| Frontier-token reduction | Not measured | Baseline absent | Initialization | Same-task A/B required |
| 14B pass-rate effect | Not measured | 14B sweep absent | Initialization | Fixed-model sweep required |

### Pending Todos

None yet.

### Blockers/Concerns

- No direct frontier baseline data exists yet; `experiments/results/baseline.csv` or an approved split-token replacement must be collected before any savings claim is credible.
- No 14B-only sweep artifact exists.
- Expanded task pack is not frozen and must avoid tuning tasks to make 7B pass.
- MIT versus AGPL metadata conflict remains unresolved before release or publication metadata is cited.

## Session Continuity

**Last session:** 2026-05-01 roadmap initialization
**Stopped at:** Phase 1 ready to plan
**Resume file:** GPD/ROADMAP.md
**Last result ID:** none
**Hostname:** none
**Platform:** none
