---
phase: 01-reliability-and-measurement-patches
type: context
created: 2026-05-01
source: inline-discuss from execute-phase alignment check
---

# Phase 1 Context: Reliability and Measurement Patches

## Phase Goal

Implement the four measurement-critical reliability patches before Step 1 resumes, so subsequent baseline and model-sweep results are not contaminated by stale retry behavior, hidden model selection, missing failure categories, or blended-cost arithmetic.

## Decisions (LOCKED)

These decisions were confirmed by the user in prior sessions and recorded in HANDOFF_TO_GPD.md and REPORT_DISPATCH_RELIABILITY.md.

1. **Retry default: 2 → 1.** `effective_max_attempts` must default to 1 for apply+verify. The literal `2` at `src/dispatch.rs:1110` becomes `1`. User-confirmed: when verify-failure is a model capability gap, local retry is pure waste.

2. **Per-dispatch model override.** Add `model: Option<String>` to `DispatchOptions`. Plumb through CLI (`--model`), MCP tool schema, and experiment harness (`AWL_MODEL_OVERRIDE`). Override beats level-default; unset preserves level-default. No auto-escalation from 7B to 14B.

3. **Failure-category telemetry.** Add `failure_category` to `apply_result` and `error_result`. Categories: format, schema, preflight, verify, timeout, network, model, unknown. Wire each existing failure path in `run_apply_flow`.

4. **Split input/output cost accounting.** Replace single `--cost-per-mtok` in tally.py and dispatch_cost_report.py with `--input-cost-per-mtok` and `--output-cost-per-mtok`. Default: Claude Opus 4.7 standard rates ($5 input, $25 output per MTok). Read `prompt_tokens`/`completion_tokens` from per-attempt usage field.

## Agent's Discretion

- Internal implementation details (struct field ordering, helper function naming, test structure) are flexible.
- Whether to use `#[allow(clippy::too_many_arguments)]` on `apply_result` after adding `failure_category` (9 params) or refactor into a struct — executor's judgment.
- Fixture test structure for Python scripts.

## Deferred Ideas (OUT OF SCOPE)

- Python preflight, streaming dispatch output, dispatch caching, per-task local-token ceilings.
- Task definition changes (Phase 2).
- Auto-escalation from 7B to 14B.
- Replacing the OpenAI-compatible JSON-schema response format.

## Hard Constraints

- Never push directly to main. Branch → PR → CI → merge.
- Never skip pre-commit hooks or weaken lint gates.
- CI checks: `checks (ubuntu-latest)` and `checks (macos-latest)` must pass.
- All new public Rust functions need at least one unit test.
- Ollama must remain optional at compile time.
- Confirm with user before opening any PR that changes the dispatch contract visible to callers or experiment definitions.

## Key References

- `HANDOFF_TO_GPD.md` — project map, hard constraints, work queue
- `REPORT_DISPATCH_RELIABILITY.md` — patch list, user-confirmed decisions, Step 1 state
- `experiments/README.md` — benchmark harness protocol
- `experiments/results/awl_arm.jsonl` — pilot data (3 tasks, 2 pass, 1 deterministic 7B failure)
