# Awl Frontier-Token Savings Study

## What This Is

Awl is a Rust CLI and stdio MCP server for bounded local coding delegation to Ollama-hosted models. This project asks whether that local-worker workflow can measurably reduce paid frontier-model tokens in real Claude/Codex sessions while preserving enough pass rate, verification discipline, and caller-visible reliability to be worth using.

This is a software-systems research project, not a conventional physics derivation. The expected deliverables are reliability patches, an expanded A/B savings experiment, and frontier-side guidance for when a task should use 7B versus 14B local implementation models.

## Core Research Question

Can Awl's bounded local Ollama dispatch workflow produce defensible frontier-token savings in real Claude/Codex coding sessions without unacceptable pass-rate or reliability regressions?

## Scoping Contract Summary

### Contract Coverage

- `claim-reliability-prep`: reliability patches are implemented before Step 1 resumes, without weakening dispatch safety, local-only execution, lint, CI, or branch-protection constraints.
- `claim-step1-answer`: Step 1 produces either a defensible positive result, with at least one Awl configuration showing at least 25% frontier-token reduction at at least 60% Awl pass rate, or a defensible negative result with documented blockers.
- Acceptance signals: local and GitHub CI pass for reliability patches; the Step 1 report compares 7B-only and 14B-only Awl configurations against the same direct frontier baseline using split input/output token costs.
- False progress to reject: local model quality, local pass rate, or qualitative convenience without direct frontier-baseline token accounting must not count as evidence of savings.

### User Guidance To Preserve

- **User-stated observables:** frontier-token reduction, Awl pass rate, dispatch failure category distribution, and input/output token cost split.
- **User-stated deliverables:** reliability patches in `src/dispatch.rs`, CLI/MCP model override plumbing, telemetry/category reporting, split-cost scripts, and `experiments/results/report.md`.
- **Must-have references / prior outputs:** `HANDOFF_TO_GPD.md`, `REPORT_DISPATCH_RELIABILITY.md`, `UPDATED_PROGRESS_REPORT.md`, `experiments/README.md`, `experiments/results/awl_arm.jsonl`, and per-task result JSON files for tasks 01-03.
- **Stop / rethink conditions:** stop before any PR that changes caller-visible dispatch contract or experiment definitions; stop before merging to `main`; stop on non-obvious CI failures; re-scope if Step 1 cannot produce comparable baseline and same-task Awl configurations.

### Scope Boundaries

**In scope**

- Ship the confirmed pre-Step-1 reliability patches: default apply+verify attempts from two to one, per-dispatch model override, failure-category telemetry, and split input/output cost accounting.
- Resume Step 1 with an expanded task pack and separate 7B-only and 14B-only Awl configurations.
- Collect or preserve direct frontier baseline token data for the same tasks.
- Produce guidance for frontier-side selection of 7B versus 14B.

**Out of scope**

- Python preflight, streaming dispatch output, dispatch caching, and per-task local-token ceilings before Step 1 demonstrates a need.
- Tuning benchmark tasks to make 7B pass more often.
- Replacing the OpenAI-compatible JSON-schema response protocol without explicit user authorization.
- Weakening CI, lint, branch protection, pre-commit hooks, or the local-only worker constraint.

### Active Anchor Registry

- `ref-handoff`: `HANDOFF_TO_GPD.md`
  - Why it matters: user-supplied project map with mission, constraints, work queue, and stop conditions.
  - Carry forward: planning, execution, verification
  - Required action: read, use
- `ref-reliability-report`: `REPORT_DISPATCH_RELIABILITY.md`
  - Why it matters: current work queue and benchmark interpretation; records the deterministic 7B failure and user-confirmed decisions.
  - Carry forward: planning, execution, verification, writing
  - Required action: read, use, compare
- `ref-progress-report`: `UPDATED_PROGRESS_REPORT.md`
  - Why it matters: historical context for why controlled testing is justified but not yet proof of savings.
  - Carry forward: planning, writing
  - Required action: read, use
- `ref-experiment-readme`: `experiments/README.md`
  - Why it matters: defines the A/B experiment protocol, baseline collection, task constraints, and thresholds.
  - Carry forward: planning, execution, verification
  - Required action: read, use, compare
- `ref-partial-awl-results`: `experiments/results/awl_arm.jsonl`
  - Why it matters: only current benchmark data; 3 tasks, 2 passes, 1 deterministic trailing-newline failure after same-model retry.
  - Carry forward: planning, verification, writing
  - Required action: compare, cite

### Carry-Forward Inputs

- `GPD/research-map/FORMALISM.md`
- `GPD/research-map/REFERENCES.md`
- `GPD/research-map/ARCHITECTURE.md`
- `experiments/results/awl_arm.jsonl`
- `experiments/results/01_string_helper.json`
- `experiments/results/02_validate_input.json`
- `experiments/results/03_fix_off_by_one.json`

### Skeptical Review

- **Weakest anchor:** no direct frontier baseline exists yet; `experiments/results/baseline.csv` must be collected before any savings claim is credible.
- **Unvalidated assumptions:** the expanded task pack can represent real bounded coding work without being tuned to either model; frontier token accounting will be comparable across baseline and Awl-assisted sessions.
- **Competing explanation:** apparent savings could come from task selection or accounting differences rather than Awl's local-worker value; 14B may improve pass rate but lose on total workflow cost or latency.
- **Disconfirming observation:** no Awl configuration reaches at least 25% frontier-token reduction at at least 60% pass rate on the same task pack.
- **False progress to reject:** qualitative model improvement, blended-rate accounting after split-cost support exists, or local pass rate without same-task frontier-baseline comparison.

### Open Contract Questions

- What direct-frontier baseline token data should populate `experiments/results/baseline.csv`?
- Which additional tasks should extend `experiments/tasks/` to at least 10 mixed Python/Rust tasks without biasing the benchmark?
- Does 14B improve pass rate enough to justify slower local execution and frontier session overhead?
- `Cargo.toml` and `README.md` indicate MIT licensing while `HANDOFF_TO_GPD.md` says AGPL-3.0; resolve before release or publication metadata is cited.

## Research Questions

### Answered

- Awl is not a physics system; the workspace is a Rust software project whose research content is an empirical software-systems question about frontier-token savings.
- Dispatch v2 already supports bounded task specifications, structured JSON output, apply/verify/rollback, compact telemetry, context paths, and repository maps.
- The partial local 7B arm is not sufficient to prove savings: it has only three tasks, no frontier baseline, and no 14B comparison.

### Active

- [ ] Can reliability patches eliminate known measurement distortions before Step 1 resumes?
- [ ] Can an expanded task pack support a fair 7B-only versus 14B-only comparison?
- [ ] Does either Awl configuration clear the threshold of at least 25% frontier-token reduction at at least 60% Awl pass rate?
- [ ] Which failure categories should cause frontier takeover, 14B opt-up, or tooling fixes?

### Out of Scope

- General local model quality evaluation without frontier-token accounting -- this does not answer the product hypothesis.
- Auto-escalation from 7B to 14B inside Awl -- the confirmed decision is that the frontier chooses the model upfront.
- New protocol families, caching, streaming, Python preflight, or local-token ceilings before Step 1 shows they are needed.

## Research Context

### Physical System

No physical system is under study. The effective system is a frontier coordinator, a bounded local Ollama worker, a dispatch contract, a local verifier, and telemetry comparing local-assisted work against direct frontier work.

### Theoretical Framework

The framework is an empirical A/B measurement of software-agent delegation economics. The central model is that bounded local execution saves frontier tokens only when task packaging, local verification, rollback, and compact reporting keep failed local work from becoming expensive frontier recovery.

### Key Parameters and Scales

| Parameter | Symbol | Regime | Notes |
| --------- | ------ | ------ | ----- |
| Frontier-token reduction | `S` | target `S >= 25%` | Aggregate paid-token reduction versus direct frontier baseline. |
| Awl pass rate | `p` | target `p >= 60%` | Passing verifier tasks divided by attempted tasks. |
| Local implementation model | `M` | 7B-only, 14B-only | Selected upfront by frontier or experiment configuration. |
| Apply+verify attempts | `A` | default 1 after patch | Current source still defaults to 2 for apply+verify; this is a first patch target. |
| Task count | `N` | target `N >= 10` | Must include mixed Python/Rust and task types. |
| Cost rates | `c_in`, `c_out` | $5/MTok input, $25/MTok output | Split input/output accounting per reliability report. |

### Known Results

- `experiments/results/awl_arm.jsonl`: partial L2 7B local arm with 3 tasks, 2 passes, 1 deterministic trailing-newline failure.
- `REPORT_DISPATCH_RELIABILITY.md`: same-model verify retry burned extra local tokens on `01_string_helper` without correcting the failure.
- `UPDATED_PROGRESS_REPORT.md`: dispatch v2 machinery is sufficient for controlled testing, but not for claiming real-world savings yet.

### What Is New

This project should turn the current qualitative and partial evidence into a decision-grade Step 1 result. The new contribution is not another dispatch feature in isolation; it is a measured answer about whether and when local bounded workers reduce frontier-token cost.

### Target Venue

No publication venue is selected. Likely outputs are internal engineering reports, repository documentation, and frontier-side usage guidance.

### Computational Environment

Primary environment: `/Users/blu3/awl` on the local workstation. Awl is Rust-based, uses Cargo for builds/tests, and uses local Ollama at `http://127.0.0.1:11434` for model serving. Required local verification gates include `cargo test`, `cargo clippy --all-targets -- -D warnings`, and GitHub CI on Ubuntu and macOS before merge.

## Notation and Conventions

See `GPD/CONVENTIONS.md` for all notation and sign conventions.
Add `GPD/NOTATION_GLOSSARY.md` later only if the project needs a dedicated symbol glossary.

## Unit System

Not a physics unit system. Operational units are tokens, dollars per million tokens, milliseconds/seconds of wall time, task counts, pass rates, and percentages.

## Requirements

See `GPD/REQUIREMENTS.md` for the detailed requirements specification.

Key requirement categories: CODE (implementation), EXP (experiment), VAL (validation), DOC (documentation)

## Key References

- `HANDOFF_TO_GPD.md` -- project map, constraints, and work queue.
- `REPORT_DISPATCH_RELIABILITY.md` -- current research artifact and patch list.
- `UPDATED_PROGRESS_REPORT.md` -- prior progress and readiness context.
- `experiments/README.md` -- Step 1 protocol and pass/fail thresholds.
- `experiments/results/awl_arm.jsonl` -- partial 7B local-arm benchmark.

## Constraints

- **Git workflow:** never push directly to `main`; use branch, PR, CI, update branch if needed, then merge only after approval.
- **Safety:** never run destructive git operations or skip pre-commit hooks without explicit same-session user confirmation.
- **CI:** do not weaken `.github/workflows/ci.yml`; `-D warnings` stays.
- **Local-only worker path:** do not introduce external paid APIs into worker execution.
- **Rust API quality:** all new public Rust functions need at least one unit test.
- **Runtime robustness:** Ollama remains optional at compile time; unreachable Ollama should return structured dispatch errors rather than crashing.

## Key Decisions

| Decision | Rationale | Outcome |
| -------- | --------- | ------- |
| Drop default apply+verify attempts from 2 to 1 | Same-model retry repeated the `01_string_helper` capability failure and inflated local-token cost. | Approved in `REPORT_DISPATCH_RELIABILITY.md`; not yet implemented at initialization. |
| Add per-dispatch model override, no auto-escalation | Frontier should choose 7B or 14B upfront based on task risk; Awl should not hide that decision. | Approved in `REPORT_DISPATCH_RELIABILITY.md`; not yet implemented at initialization. |
| Compare 7B-only and 14B-only configurations | Needed to calibrate whether opt-up to 14B pays off. | Planned for Step 1 after reliability patches. |
| Use split input/output token costs | Blended costs can misstate savings under asymmetric pricing. | Planned for scripts and tally output. |
| Preserve hard git and CI constraints | Reliability patches must not trade away safety gates for speed. | Binding project constraint. |

Full log: `GPD/DECISIONS.md`

---

_Last updated: 2026-05-01 after approved `$gpd-new-project` scoping contract from `HANDOFF_TO_GPD.md`_
