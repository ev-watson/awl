# Reference and Anchor Map

**Analysis Date:** 2026-05-01

## Active Reference Context

- Project contract: missing at `GPD/state.json`; not authoritative.
- Active anchor registry: none confirmed in `state.json.project_contract.references`.
- Must-read references from intake: none confirmed.
- Prior outputs and baselines from intake: none confirmed.
- Stable knowledge documents: none found.
- Physics literature artifacts: no `.bib`, `.tex`, paper PDFs, notebooks, or literature-review documents were found in the inspected workspace.

## Active Anchor Registry

| Anchor ID | Anchor | Type | Source / Locator | Why It Matters | Contract Subject IDs | Must Surface | Required Action | Carry Forward To |
|---|---|---|---|---|---|---|---|---|
| `contract-missing-state-json` | Missing project contract | context_gap | `GPD/state.json` absent; `GPD/state.json.lock` present | Prevents treating any project-contract reference registry as authoritative |  | yes | avoid | planning,verification |
| `awl-readme-core-loop` | Awl README | background/method | `README.md` | Defines the local-first CLI, five-phase agent loop, model profiles, MCP integration, dispatch behavior, and quality gates |  | yes | read,use,cite | planning,writing |
| `handoff-gpd-awl-map` | GPD handoff | prior artifact | `HANDOFF_TO_GPD.md` | Provides the current mission, file map, constraints, work queue, Step 1 status, and stabilized facts |  | yes | read,use | planning,execution,verification |
| `report-dispatch-reliability` | Dispatch reliability report | prior artifact/benchmark | `REPORT_DISPATCH_RELIABILITY.md` | Latest research artifact: records L2 7B failure, retry-policy decision, model-override decision, patch list, and Step 1 continuation plan |  | yes | read,use,compare,cite | planning,execution,verification,writing |
| `updated-progress-report` | Prior progress report | prior artifact/background | `UPDATED_PROGRESS_REPORT.md` | Summarizes completed implementation against the original token-savings plan and defines readiness and residual risks |  | yes | read,use | planning,writing |
| `experiment-readme` | A/B savings experiment protocol | method/benchmark | `experiments/README.md` | Defines local Awl arm, manual frontier-baseline arm, output files, task constraints, and pass/fail thresholds |  | yes | read,use | execution,verification,writing |
| `dispatch-core-source` | Dispatch implementation | method | `src/dispatch.rs` | Implements dispatch schema, apply/verify/rollback, retries, response validation, telemetry, and current default retry policy |  | yes | read,use,compare | execution,verification |
| `defaults-model-map` | Model defaults and tier mapping | method | `src/defaults.rs` | Defines 14B agent, 7B implementation, 3B verification defaults and environment/config precedence |  | yes | read,use | planning,execution |
| `mcp-server-tool-contract` | MCP tool schema | method | `src/mcp_server.rs` | Defines externally visible tool surface and gating of full `awl_agent` behind `AWL_ENABLE_MCP_AGENT=1` |  | yes | read,use | execution,verification |
| `repomap-source` | Repository map approximation | method | `src/repomap.rs` | Implements tree-sitter symbol extraction and compact repo context used for grounded dispatch |  | no | read,use | execution |
| `run-awl-arm-harness` | Local Awl experiment arm | benchmark/method | `experiments/run_awl_arm.sh` | Drives per-task setup, dispatch, telemetry extraction, and `awl_arm.jsonl` generation |  | yes | read,use | execution,verification |
| `tally-script` | A/B comparison script | benchmark/method | `experiments/tally.py` | Computes per-task and aggregate token savings and pass rates from local and frontier baseline arms |  | yes | read,use,compare | verification,writing |
| `cost-report-script` | Dispatch cost summary | benchmark/method | `scripts/dispatch_cost_report.py` | Summarizes dispatch logs and estimates paid frontier cost avoided; currently uses blended cost rate |  | no | read,use,compare | verification,writing |
| `awl-arm-jsonl-partial` | Partial L2 7B local-arm results | benchmark | `experiments/results/awl_arm.jsonl` | Records the only found local experiment data: 3 tasks, 2 passes, 1 repeated failure, token and wall-time counts |  | yes | compare,cite | verification,writing |
| `result-01-string-helper` | Repeated trailing-newline failure | benchmark | `experiments/results/01_string_helper.json`; task spec `experiments/tasks/01_string_helper/task.json` | Concrete evidence that same-model verify retry can repeat a semantic error and burn extra local tokens |  | yes | compare,cite | planning,verification,writing |
| `result-02-validate-input` | Passing write-from-scratch task | benchmark | `experiments/results/02_validate_input.json`; task spec `experiments/tasks/02_validate_input/task.json` | Evidence that L2 7B can pass a bounded easy Python generation task with verifier |  | yes | compare,cite | verification,writing |
| `result-03-fix-off-by-one` | Passing edit-existing task | benchmark | `experiments/results/03_fix_off_by_one.json`; task spec `experiments/tasks/03_fix_off_by_one/task.json` | Evidence that L2 7B can pass a bounded edit-existing task with context-path grounding |  | yes | compare,cite | verification,writing |
| `awl-worker-agent-guidance` | Frontier delegation guidance | method/prior artifact | `.claude/agents/awl-worker.md`; examples variant `examples/awl-worker.md` | States when to delegate, when not to delegate, and that frontier agent owns final judgment |  | no | read,use | planning,execution |
| `awl-dispatch-skill-guidance` | Codex/Claude dispatch skill | method/prior artifact | `.agents/skills/awl-dispatch/SKILL.md`; `.claude/skills/awl-dispatch/SKILL.md` | Documents intended usage pattern for `awl_dispatch` as a bounded worker, not a planner |  | no | read,use | planning,execution |
| `ollama-runtime-context` | Ollama runtime dependency | background | `https://ollama.com`; local config references in `README.md` and `src/defaults.rs` | External runtime assumed for local model serving; not a physics or literature reference |  | no | read | planning |
| `github-repo-locator` | Public repository locator | background | `https://github.com/ev-watson/awl`; `Cargo.toml` repository/homepage | Durable locator for repository identity and release context |  | no | cite | writing |

## Benchmarks and Comparison Targets

- Partial Awl local arm, L2 7B-q4:
  - Source: `experiments/results/awl_arm.jsonl`
  - Status: present but incomplete.
  - Observed: 2/3 tasks passed; total local worker tokens across three records were 8601; one task failed after two attempts.

- `01_string_helper` trailing-newline failure:
  - Source: `experiments/results/01_string_helper.json`, `REPORT_DISPATCH_RELIABILITY.md`
  - Status: failed and used as architectural signal.
  - Required comparison: rerun under the planned one-attempt default and under 14B-only configuration before claiming model-selection guidance.

- Frontier-only baseline:
  - Source: expected `experiments/results/baseline.csv` per `experiments/README.md`
  - Status: missing.
  - Consequence: no inspected artifact proves paid-token savings yet.

- 14B-only sweep:
  - Source: proposed in `REPORT_DISPATCH_RELIABILITY.md`
  - Status: missing.
  - Consequence: the 7B-vs-14B tradeoff remains unmeasured locally.

## Prior Artifacts and Baselines

- `HANDOFF_TO_GPD.md`: Treat as the operational project map for next work, but cross-check against source because some claims can drift.
- `REPORT_DISPATCH_RELIABILITY.md`: Treat as the current research artifact and work queue. It supersedes the earlier retry-policy implication in source intent, but the source has not yet caught up.
- `UPDATED_PROGRESS_REPORT.md`: Treat as historical context and motivation; it states readiness for controlled testing, not proof of real-world savings.
- `experiments/results/awl_arm.jsonl`: Treat as a partial benchmark only. It is not sufficient for final claims because no baseline arm exists.

## Literature Foundations

No academic papers, BibTeX entries, DOI locators, arXiv IDs, or formal literature review artifacts were found. The inspected reference foundation is local project documentation and source code, not an external scholarly literature base.

Background technologies and non-scholarly locators found:

- Ollama runtime: `https://ollama.com`, referenced by `README.md` and `CONTRIBUTING.md`.
- Repository identity: `https://github.com/ev-watson/awl`, referenced by `Cargo.toml` and `README.md`.
- OpenAI-compatible response format is mentioned in project docs/source, but no external specification anchor was found in the workspace.

## Open Reference Questions

- Where are the original `AwlUsageReport.md` and `reportreport.txt` cited by `UPDATED_PROGRESS_REPORT.md`? They were not present in this workspace.
- Is there an authoritative project contract intended to live at `GPD/state.json`? It is missing here.
- What frontier baseline token data should populate `experiments/results/baseline.csv`?
- Which exact 14B model configuration and task matrix will be used for the 14B-only sweep?
- Should external references be added for OpenAI-compatible structured outputs, MCP protocol semantics, Ollama model-serving behavior, and Qwen model-card details? None are locally anchored yet.
- The handoff says license is AGPL-3.0, while `Cargo.toml` and `README.md` indicate MIT. This should be resolved before publishing or citing release metadata.

## Required Carry-Forward Actions

- Carry `contract-missing-state-json` into planning and verification; do not infer contract claims or reference requirements from absent state.
- Carry `report-dispatch-reliability` into planning, execution, verification, and writing as the current work queue.
- Carry `experiment-readme`, `run-awl-arm-harness`, `tally-script`, and `awl-arm-jsonl-partial` into experiment planning and verification.
- Carry `result-01-string-helper` into retry-policy and model-selection discussions; it is the concrete disconfirming case for same-model verify retry.
- Carry the missing baseline and missing 14B sweep as unresolved blockers for any final token-savings conclusion.

---

_Reference map: 2026-05-01_
