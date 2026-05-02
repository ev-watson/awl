# Computational Architecture

**Analysis Date:** 2026-05-01
**Focus:** computation

## Active Reference Context

- Active Reference Registry: none confirmed in `GPD/state.json.project_contract.references`; `GPD/state.json` was not present, only `GPD/state.json.lock`.
- Must-read references: none confirmed.
- Prior outputs and baselines: none confirmed by project contract.
- User-asserted anchors and gaps: none beyond the mapping request.
- Stable knowledge documents: none found in the runtime-active context.

## Computational Setting

This repository is a Rust command-line and MCP tool server for local agentic coding with Ollama-hosted models. It is not a numerical physics simulation project in the usual ODE/PDE/linear-algebra sense. I found no NumPy, SciPy, PETSc, MPI, HDF5, Julia, C/C++, Fortran, or notebook-based solver pipeline. The main computational engine is an LLM-backed workflow orchestrator with safety checks, structured JSON I/O, file edit primitives, local verification commands, and repository mapping.

The primary executable is the `awl` binary declared in `Cargo.toml` and implemented by `src/main.rs`. Runtime model calls use OpenAI-compatible Ollama HTTP endpoints assembled in `src/defaults.rs` and invoked through `reqwest` in `src/agent.rs`, `src/dispatch.rs`, `src/doctor.rs`, and `src/plan.rs`.

## Solver and Algorithm Choices

| Component | Algorithm / Solver Role | Implementation |
| --- | --- | --- |
| Agent loop | Five-phase state machine: Formulate -> Plan -> Execute -> Verify -> Complete, with regression from Verify to Execute and NeedsHuman handoff on stalls/timeouts | `src/agent.rs`, `src/phases.rs`, `src/session.rs` |
| Local LLM dispatch | Bounded code-generation worker using Ollama chat completions, JSON schema validation, retry on malformed output, optional write/apply/verify rollback loop | `src/dispatch.rs` |
| Planning | LLM-generated structured implementation plans as JSON, separate from execution | `src/plan.rs` |
| Repository map | Tree-sitter symbol extraction for Rust/Python plus a PageRank-style relevance ranking over import/reference edges | `src/repomap.rs` |
| File editing | Content-hashed line anchors, edit parsing, validation against current file state, and snapshot-based undo | `src/hashline.rs`, `src/tools.rs` |
| Safety layer | Workspace-bounded path resolution and allowlisted shell command validation | `src/safety.rs` |
| MCP protocol | JSON-RPC over stdio with tool-list and tool-call handlers; optional client-side MCP tool proxying into the agent loop | `src/mcp_server.rs`, `src/mcp_client.rs` |

No deterministic numerical solver stack was detected. There are no finite-difference, finite-element, spectral, Monte Carlo, ODE/PDE, sparse linear algebra, or GPU kernels in the inspected source tree.

## Computational Pipeline

1. User entry arrives through CLI subcommands in `src/main.rs`.
   - Inputs: command-line flags, stdin JSON for `dispatch` and `plan`, environment variables, optional config files.
   - Outputs: human-readable CLI output or structured JSON.

2. Configuration is resolved in `src/config.rs` and `src/defaults.rs`.
   - Precedence documented in `README.md`: CLI flags, environment variables, user config, built-in defaults.
   - Main runtime config path defaults to `~/.config/awl/config.json`; sessions default to `~/.config/awl/sessions`.

3. Model-facing commands construct OpenAI-compatible chat-completion requests.
   - `src/agent.rs`: long-running tool-using agent requests with tool definitions.
   - `src/dispatch.rs`: constrained worker requests with strict JSON response schema.
   - `src/plan.rs`: compact JSON plan requests.

4. Tool calls route through `src/tools.rs`.
   - Built-in tools include shell, read/write/edit file, search/list files, repo map, dispatch, and undo.
   - Mutating tools clear cache and snapshot file state where applicable.
   - MCP tools can be added dynamically by `src/agent.rs` via `src/mcp_client.rs`.

5. Verification, persistence, and telemetry are written locally.
   - Agent sessions are JSONL logs managed by `src/session.rs`.
   - Dispatch attempt logs are JSONL files under the Awl config directory, managed by `src/dispatch.rs`.
   - Apply-mode dispatch writes one target file, runs `verify_command` when supplied, and restores the previous file on failure.

6. Optional experiment scripts benchmark local dispatch against frontier-model baselines.
   - `experiments/run_awl_arm.sh` runs each `experiments/tasks/*/task.json` through `awl dispatch --apply --auto-repomap`.
   - `experiments/tally.py` compares `experiments/results/awl_arm.jsonl` with manually collected `experiments/results/baseline.csv`.
   - `scripts/dispatch_cost_report.py` summarizes dispatch JSONL logs and estimates avoided paid-token cost.

## Key Libraries

Rust dependencies from `Cargo.toml`:

- `tokio`: async runtime, process I/O, synchronization, and timeouts.
- `reqwest`: HTTP client for Ollama-compatible APIs.
- `serde` / `serde_json`: config, JSON-RPC, JSONL logs, model request/response payloads.
- `tree-sitter`, `tree-sitter-rust`, `tree-sitter-python`: source parsing for repository maps.
- `petgraph`: directed graph and PageRank-like symbol ranking support.
- `walkdir`, `glob`: workspace traversal and file matching.
- `rand`, `chrono`: session and dispatch identifiers.
- `async-trait`: async tool trait implementation.

Python usage is stdlib-only in inspected scripts: `argparse`, `csv`, `json`, `os`, `pathlib`, `sys`, `time`, and `typing` in `scripts/dispatch_cost_report.py` and `experiments/tally.py`.

## Parallelization and Concurrency

- The Rust runtime uses Tokio multi-threaded runtimes in `src/agent.rs`, `src/dispatch.rs`, `src/doctor.rs`, `src/mcp_server.rs`, and `src/plan.rs`.
- The agent can receive multiple tool calls from a model response, but `src/agent.rs` executes them in a sequential loop.
- `src/mcp_client.rs` serializes communication with each connected MCP server through a mutex-protected stdin/stdout pair.
- CI and release workflows parallelize by matrix across operating systems/targets in `.github/workflows/ci.yml` and `.github/workflows/release.yml`.
- Experiment scripts iterate tasks sequentially; no GNU parallel, job scheduler, Slurm, MPI, Rayon, or distributed execution layer was found.

## MCP and External Tooling

`src/mcp_server.rs` exposes four default MCP tools: `awl_health`, `awl_dispatch`, `awl_repomap`, and `awl_hashline`. The full agent tool `awl_agent` is hidden unless `AWL_ENABLE_MCP_AGENT=1` is set. The local `.mcp.json` points the `awl` MCP server at `target/release/awl serve`. Example client configs are in `examples/codex-config.toml` and `examples/claude-code.mcp.json`.

No MCP simulation servers or physics-specific simulation backends were found. The MCP surface is for coding delegation and repository inspection.

## Data Flow and Formats

- CLI stdin for `dispatch` and `plan`: JSON task specifications.
- Model API traffic: JSON chat-completion payloads over HTTP.
- MCP traffic: JSON-RPC over stdio.
- Session logs: JSONL files with metadata and chat/tool messages (`src/session.rs`).
- Dispatch logs: JSONL event logs with attempt, telemetry, verification, and usage entries (`src/dispatch.rs`).
- Experiment inputs: JSON task files under `experiments/tasks/*/task.json`.
- Experiment outputs: JSON results per task plus aggregate JSONL/CSV under `experiments/results/` when generated.
- Build/package metadata: `Cargo.toml`, `Cargo.lock`, GitHub Actions YAML.

## Performance Bottlenecks and Failure Modes

- Ollama availability and model latency dominate `agent`, `dispatch`, and `plan` commands.
- `dispatch` can spend extra time on JSON-format retries, verification retries, and rollback loops; `FORMAT_RETRIES` is 3 and apply attempts are capped at 5.
- Verification commands are capped by a 120 second timeout in `src/dispatch.rs`.
- Repository mapping parses all supported Rust/Python source files under the workspace, then builds/ranks a symbol graph; this is lightweight for the current repo but scales with source count.
- Agent context compaction triggers around an estimated 3000 tokens in `src/agent.rs`; compaction itself is another model call.
- Tool cache invalidates on mutating tools, trading correctness for repeated read/search cost.
- Shell safety is syntactic allowlisting in `src/safety.rs`; it reduces command surface but is not a full sandbox.

## Evidence Gaps

- No project contract or authoritative reference registry was available in `GPD/state.json`.
- Generated `target/` and `experiments/results/` artifacts exist locally but were not treated as stable baselines because the intake provided no confirmed prior-output anchors.
- I did not find a numerical-science validation suite; tests are Rust unit tests plus CI commands and small Python experiment verification commands.

---

*Computation architecture analysis: 2026-05-01*
