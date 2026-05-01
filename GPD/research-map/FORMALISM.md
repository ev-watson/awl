# Theoretical Frameworks

**Analysis Date:** 2026-05-01

## Active Reference Context

- Project contract: missing. `GPD/state.json` is not present; only `GPD/state.json.lock` was found. The missing contract is context only, not authoritative.
- Active reference registry: none confirmed in `state.json.project_contract.references`.
- Must-read references: none confirmed by machine-readable intake.
- Stable knowledge documents: none found in the workspace.

## Physical System

**Subject:** No physics research system was found in the inspected workspace. The repository is a Rust software project, `awl`, whose research content is an empirical software-systems question: whether bounded local Ollama workers reduce paid frontier-model token use for coding tasks.

**Evidence:** `README.md`, `HANDOFF_TO_GPD.md`, `UPDATED_PROGRESS_REPORT.md`, `REPORT_DISPATCH_RELIABILITY.md`, `experiments/README.md`, `src/dispatch.rs`, `src/defaults.rs`, `experiments/run_awl_arm.sh`, `experiments/tally.py`, and `experiments/results/awl_arm.jsonl`.

**Physics scales:** Not applicable.

- Energy: no physical energy scale found.
- Length: no physical length scale found.
- Time: software wall-clock time appears in dispatch telemetry and experiment reports, not as a physical dynamical variable.
- Dimensionless parameters: software metrics such as token counts, pass rates, retry counts, and percentage token reduction.

**Degrees of Freedom:**

- Frontier coordinator: Claude/Codex-side agent that chooses task decomposition and final review; described in `HANDOFF_TO_GPD.md` and `.claude/agents/awl-worker.md`.
- Local worker model: Ollama-hosted model selected by tier, with defaults in `src/defaults.rs`.
- Dispatch task specification: JSON fields `task`, `constraints`, `context_paths`, `target_path`, `verify_command`, `apply`, `max_attempts`, and repo-map controls in `src/dispatch.rs`.
- Target file state: snapshot, generated replacement, verified final state, or restored previous state in `src/dispatch.rs`.
- Verification process: local shell command with fixed timeout in `src/dispatch.rs`.
- Telemetry/log state: per-dispatch JSONL logs and compact top-level telemetry in `src/dispatch.rs`, `scripts/dispatch_cost_report.py`, and `experiments/run_awl_arm.sh`.

## Theoretical Framework

**Primary Framework: bounded local-worker cost-saving model**

- Formulation: empirical A/B comparison between direct frontier implementation and Awl-assisted local dispatch.
- Central hypothesis: bounded local execution can reduce paid frontier tokens when task packaging, local verification, rollback, and compact reporting prevent failed local work from becoming expensive frontier recovery.
- Locations: `HANDOFF_TO_GPD.md`, `UPDATED_PROGRESS_REPORT.md`, `REPORT_DISPATCH_RELIABILITY.md`, `experiments/README.md`, `experiments/tally.py`.

**Secondary Framework: finite-state agent workflow**

- `README.md` describes the agent loop as:

```text
Formulate -> Plan -> Execute -> Verify -> Complete
                         ^                   |
                         +---- on failure ---+
```

- Implementation evidence: `src/agent.rs` uses `PhaseState`, gate detection, regression to execute on verification failure, compaction thresholds, and wall/iteration limits.

**Secondary Framework: apply/verify/rollback invariant**

- Apply mode snapshots a file, writes generated code, runs the verifier when supplied, and restores the prior contents on failure.
- Location: `src/dispatch.rs`, especially `capture_snapshot`, `write_target`, `run_verify_command`, `restore_snapshot`, and `run_apply_flow`.

## Fundamental Equations and Metrics

| Equation / Rule | Type | Location | Status |
|---|---|---|---|
| `token_savings = (1 - awl_tokens / baseline_tokens) * 100` | experiment metric | `experiments/tally.py` (`percent_savings`) | Implemented |
| `total_tokens = prompt_tokens + completion_tokens` when provider total is absent | telemetry aggregation | `experiments/run_awl_arm.sh`, `scripts/dispatch_cost_report.py` | Implemented |
| `estimated_cost_avoided = frontier_direct_tokens / 1_000_000 * frontier_cost_per_mtok` | cost estimate | `scripts/dispatch_cost_report.py` | Implemented as blended-rate estimate |
| `pass_rate = passing_tasks / attempted_tasks` | experiment criterion | `experiments/tally.py`, `experiments/README.md` | Implemented/reporting |
| Success threshold: `>=25-40%` paid token reduction and `>=60-70%` Awl pass rate | benchmark criterion | `experiments/README.md`, `UPDATED_PROGRESS_REPORT.md` | Postulated by project docs |
| `effective_max_attempts = raw.unwrap_or(if apply && has_verify { 2 } else { 1 }).clamp(1, 5)` | retry policy | `src/dispatch.rs` | Implemented, but contradicted by later design decision to default verify retries to 1 |
| Structured worker response schema: `status`, `code`, `explanation`, `files_modified` | protocol constraint | `src/dispatch.rs` (`dispatch_response_format`, `validate_response`) | Implemented |

**Current benchmark observations:**

- `experiments/results/awl_arm.jsonl` records three L2 7B-q4 local-arm tasks.
- `01_string_helper`: failed after 2 attempts, 4264 total tokens, repeated trailing-newline failure.
- `02_validate_input`: passed after 1 attempt, 2073 total tokens.
- `03_fix_off_by_one`: passed after 1 attempt, 2264 total tokens.
- No frontier-baseline CSV was found; therefore token-savings claims remain unverified.

## Symmetries and Conservation Laws

No physical symmetries, gauge groups, Ward identities, anomalies, topological invariants, or Noether conservation laws were found.

Software invariants that play an analogous validation role:

- Workspace-state conservation under failed verified apply: failed verification restores the previous file contents. Location: `src/dispatch.rs`.
- Trusted changed-file reporting: `files_changed` / `files_modified` are actual Awl writes in apply mode; non-apply model claims are separated into `files_intended`. Locations: `src/dispatch.rs`, `README.md`, `UPDATED_PROGRESS_REPORT.md`.
- Workspace containment and shell validation: target paths and verify commands are validated before use. Locations: `src/dispatch.rs`, `src/safety.rs`.
- Local-first constraint: worker path uses local Ollama models rather than paid external APIs. Locations: `README.md`, `CONTRIBUTING.md`, `HANDOFF_TO_GPD.md`.

## Approximation Schemes and Effective Theories

- Local-worker approximation: Qwen/Ollama workers approximate bounded coding work that a frontier model could otherwise perform. This is the core empirical approximation, not a proven equivalence.
- Tiered model approximation: level 2 is implementation, level 3 is verification/lint. Defaults are `qwen2.5-coder:7b-instruct-q4_K_M` and `qwen2.5-coder:3b-instruct-q4_K_M`; the agent default is `qwen2.5-coder:14b`. Location: `src/defaults.rs`.
- Context truncation: context files are capped by per-file and total character limits before model dispatch. Location: `src/dispatch.rs`.
- Repo-map approximation: tree-sitter symbol extraction and PageRank produce a compact repository summary under a token budget. Location: `src/repomap.rs`.
- Deterministic generation assumption: dispatch uses temperature `0.0`; planning uses `0.1`; agent loop uses default `0.2`. Locations: `src/dispatch.rs`, `src/plan.rs`, `src/agent.rs`.

## Boundary Conditions and Constraints

- Apply mode requires a single target path or exactly one target file. Location: `src/dispatch.rs`.
- Verification is a local shell command run from the workspace root with a 120000 ms timeout. Location: `src/dispatch.rs`.
- Generated Rust code undergoes a fast pre-write unresolved crate-import check for `use crate::...` paths. Location: `src/dispatch.rs`.
- MCP exposes `awl_dispatch`, `awl_repomap`, `awl_hashline`, and `awl_health`; `awl_agent` is hidden unless `AWL_ENABLE_MCP_AGENT=1`. Location: `src/mcp_server.rs`.
- Experiment tasks must be bounded, have idempotent setup, write inside `experiments/sandbox/<id>/`, and include a verifier. Location: `experiments/README.md`.

## Phase Structure / Regimes

**Regimes studied or proposed:**

- 7B-only L2 implementation regime: partially observed in `experiments/results/awl_arm.jsonl`.
- 14B-only implementation regime: proposed but not observed in local artifacts.
- Direct frontier baseline: required by `experiments/README.md`, but no `experiments/results/baseline.csv` was found.
- Retry-policy regimes: current source defaults to two attempts for apply+verify; `REPORT_DISPATCH_RELIABILITY.md` records a user-confirmed decision to reduce that default to one.

**Known limiting cases:**

- If task is tiny, delegation overhead can exceed direct frontier cost. Locations: `.claude/agents/awl-worker.md`, `.agents/skills/awl-dispatch/SKILL.md`.
- If task correctness cannot be verified locally, the apply/verify/rollback guarantee weakens to unverified apply or non-apply intent. Locations: `README.md`, `src/dispatch.rs`.
- If a model repeats the same semantic error, same-model verify retry can add local tokens without new information. Evidence: `REPORT_DISPATCH_RELIABILITY.md`, `experiments/results/01_string_helper.json`.

## Units and Conventions

**Physics conventions:** No unit system, metric signature, Fourier convention, spin basis, gauge choice, renormalization scheme, or field normalization convention was found.

**Software/experiment conventions:**

- Token counts are separated into prompt/input and completion/output when available, with total as provider total or sum. Locations: `experiments/run_awl_arm.sh`, `scripts/dispatch_cost_report.py`.
- Existing scripts currently use blended cost rates; `REPORT_DISPATCH_RELIABILITY.md` records a planned change to separate input/output cost rates.
- All inspected source-level response contracts are JSON-based, not XML/tagged protocols.

## Critical Gaps

- No LaTeX, BibTeX, physics equations, manuscripts, notebooks, or cited physics literature were found.
- No authoritative project contract was available.
- No frontier-baseline arm was found, so the central token-savings hypothesis is not yet established.
- The source still implements two default apply+verify attempts, while the latest reliability report says the confirmed design decision is one default attempt.
- No local artifact demonstrates the proposed 14B sweep.

---

_Framework analysis: 2026-05-01_
