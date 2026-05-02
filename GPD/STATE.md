# Research State

## Project Reference

See: GPD/PROJECT.md (updated 2026-05-01)

**Machine-readable scoping contract:** `GPD/state.json` field `project_contract`

**Core research question:** Can Awl's bounded local Ollama dispatch workflow produce defensible frontier-token savings in real Claude/Codex coding sessions without unacceptable pass-rate or reliability regressions?
**Current focus:** Phase 1 ready to plan: reliability and measurement patches.

## Current Position

**Current Phase:** 1
**Current Phase Name:** Reliability and Measurement Patches
**Total Phases:** 5
**Current Plan:** none
**Total Plans in Phase:** none
**Status:** ready_to_plan
**Last Activity:** Project initialized; roadmap and conventions established.
**Last Activity Description:** Ready for Phase 1 planning.

**Progress:** [░░░░░░░░░░] 0%

## Active Calculations

None yet.

## Intermediate Results

None yet.

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

None yet.

### Active Approximations

None yet.

**Convention Lock:**

*Custom conventions:*
- Custom:Project Type: software_systems_empirical_measurement_nonphysical
- Custom:Token Units: provider_reported_tokens_split_input_output
- Custom:Cost Units: usd_per_million_frontier_tokens_split_c_in_c_out
- Custom:Time Units: wall_time_seconds_duration_ms_only_for_ms_fields
- Custom:Model Naming: exact_ollama_model_tag_plus_fixed_arm_label
- Custom:Result Schema Naming: snake_case_json_csv_fields
- Custom:Failure Category Enum: format,schema,preflight,verify,timeout,network,model,unknown
- Custom:Experiment Comparability: same_frozen_task_ids_and_verifier_semantics_across_arms
- Custom:Git Ci Evidence: command_exit_status_date_branch_commit_and_ci_check_name

### Propagated Uncertainties

None yet.

### Pending Todos

None yet.

### Blockers/Concerns

None

## Session Continuity

**Last session:** 2026-05-01T21:57:47Z
**Stopped at:** Project initialized; ready for Phase 1 planning.
**Resume file:** none
**Last result ID:** none
**Hostname:** evans-mac.local
**Platform:** Darwin
