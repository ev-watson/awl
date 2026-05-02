# Project Structure

**Analysis Date:** 2026-05-01
**Focus:** computation

## Active Reference Context

- Active Reference Registry: none confirmed in `GPD/state.json.project_contract.references`; `GPD/state.json` was missing at scan time.
- Must-read references: none confirmed.
- Prior outputs and baselines: none confirmed.
- User-asserted anchors and gaps: none beyond the mapping request.
- Stable knowledge documents: none found in the active context.

## Directory Layout

```text
/Users/blu3/awl/
+-- src/                         # Rust source for the awl CLI, MCP server/client, agent loop, tools, dispatch, safety, sessions
+-- scripts/                     # Install, dispatch evaluation, and dispatch-log cost-report helpers
+-- experiments/                 # A/B local-dispatch savings experiment harness, tasks, generated sandbox/results
+-- examples/                    # Example Claude/Codex MCP configuration and worker-agent prompt
+-- .github/                     # CI, release, issue templates, dependabot, CODEOWNERS
+-- GPD/                         # GPD state lock and research-map output directory
+-- target/                      # Cargo build output and generated eval/smoke artifacts; generated, not source
+-- Cargo.toml                   # Rust package metadata, dependencies, lint policy, binary target
+-- Cargo.lock                   # Locked Rust dependency graph
+-- README.md                    # User-facing architecture, configuration, usage, MCP, and experiment docs
+-- CHANGELOG.md                 # Release notes
+-- CONTRIBUTING.md              # Contribution process
+-- SECURITY.md                  # Security policy
+-- HANDOFF_TO_GPD.md            # Local project handoff note; not part of the Rust build
+-- REPORT_DISPATCH_RELIABILITY.md # Local reliability report; not part of the Rust build
+-- UPDATED_PROGRESS_REPORT.md   # Local progress report; not part of the Rust build
+-- .mcp.json                    # Local MCP registration for target/release/awl serve
+-- vault.sh                     # Optional macOS helper packaged by release workflow
```

## Directory Purposes

**`src/`:**
- Purpose: all compiled Rust code for `awl`.
- Key entry point: `src/main.rs`.
- Main modules:
  - `src/agent.rs`: long-running phased agent loop.
  - `src/dispatch.rs`: bounded local worker dispatch with optional apply/verify rollback.
  - `src/tools.rs`: tool registry and built-in tools.
  - `src/mcp_server.rs`: stdio MCP server.
  - `src/mcp_client.rs`: MCP client for proxying external tools into the agent.
  - `src/repomap.rs`: tree-sitter symbol scan and graph ranking.
  - `src/hashline.rs`: content-hashed line editing.
  - `src/safety.rs`: workspace path and shell-command validation.
  - `src/config.rs`, `src/defaults.rs`, `src/init.rs`, `src/doctor.rs`: configuration and health checks.
  - `src/session.rs`, `src/phases.rs`, `src/llm_io.rs`, `src/plan.rs`: state, phase control, LLM I/O helpers, and planning.

**`scripts/`:**
- `scripts/install.sh`: release-binary installer with platform detection and checksum verification.
- `scripts/dispatch_eval.sh`: local dispatch smoke/evaluation cases for non-apply, apply success, and rollback.
- `scripts/dispatch_cost_report.py`: JSONL dispatch-log summary and paid-token avoidance estimate.

**`experiments/`:**
- `experiments/run_awl_arm.sh`: runs the local-Awl arm across task directories.
- `experiments/tally.py`: compares Awl records against a manually populated frontier baseline CSV.
- `experiments/tasks/*/task.json`: dispatch task specs.
- `experiments/tasks/*/setup.sh`: idempotent sandbox/test fixture creation.
- `experiments/sandbox/`: generated task workspaces.
- `experiments/results/`: generated JSON/JSONL/CSV experiment outputs.

**`examples/`:**
- `examples/codex-config.toml`: Codex MCP server config example.
- `examples/claude-code.mcp.json`: Claude Code MCP server config example.
- `examples/awl-worker.md`: delegation-agent guidance for using Awl from a frontier host.

**`.github/`:**
- `.github/workflows/ci.yml`: Rust formatting, clippy, tests, dispatch cost-report check, optional dispatch eval, cargo package dry run.
- `.github/workflows/release.yml`: tagged-release binary builds for Linux x86_64, macOS x86_64, and macOS arm64.
- Issue templates, PR template, CODEOWNERS, and Dependabot config.

## File Organization and Naming

- Rust modules are flat under `src/` and are imported from `src/main.rs` using one file per module.
- CLI subcommands mostly correspond to module names: `dispatch`, `hashline`, `repomap`, `plan`, `agent`, `init`, `config`, `serve`, `doctor`, and `sessions`.
- Experiment tasks use numbered directory names such as `experiments/tasks/01_string_helper/`, each containing `task.json` and `setup.sh`.
- Generated experiment sandboxes/results mirror task IDs under `experiments/sandbox/<id>/` and `experiments/results/`.
- Dispatch/session logs are stored outside the repo by default under the Awl config directory, not in source control.

## Input and Output Formats

| Format | Used For | Files / Modules |
| --- | --- | --- |
| JSON | CLI stdin task specs, config, MCP JSON-RPC payloads, model response envelopes, experiment task specs/results | `src/dispatch.rs`, `src/plan.rs`, `src/mcp_server.rs`, `src/config.rs`, `experiments/tasks/*/task.json` |
| JSONL | Session transcripts and dispatch attempt/event logs | `src/session.rs`, `src/dispatch.rs`, `experiments/results/awl_arm.jsonl` |
| TOML | Rust package manifest and Codex MCP example | `Cargo.toml`, `examples/codex-config.toml` |
| YAML | GitHub Actions and issue templates | `.github/workflows/*.yml`, `.github/ISSUE_TEMPLATE/*.yml` |
| Shell | Install, experiment setup/run, local dispatch evaluation | `scripts/*.sh`, `experiments/**/*.sh`, `vault.sh` |
| Markdown | User docs, reports, examples, research maps | `README.md`, `examples/awl-worker.md`, `GPD/research-map/*.md` |
| CSV | Manual frontier-baseline input for experiment tally | `experiments/results/baseline.csv` when created |

No HDF5, NetCDF, NumPy arrays, Parquet, binary simulation checkpoints, or physics data files were found in the source scan.

## Dependency Graph Between Scripts and Modules

**Rust CLI path:**

- `src/main.rs` parses subcommands and calls module-level runners.
- `dispatch` -> `src/dispatch.rs` -> `src/defaults.rs`, `src/config.rs`, `src/llm_io.rs`, `src/repomap.rs`, `src/safety.rs`.
- `agent` -> `src/agent.rs` -> `src/tools.rs`, `src/phases.rs`, `src/session.rs`, `src/mcp_client.rs`, `src/defaults.rs`.
- `serve` -> `src/mcp_server.rs` -> `src/dispatch.rs`, `src/repomap.rs`, `src/hashline.rs`, optional `src/agent.rs`.
- `repomap` -> `src/repomap.rs` -> tree-sitter parsers, `petgraph`, `src/safety.rs`.
- `hashline` -> `src/hashline.rs` -> `src/safety.rs`.
- `doctor` -> `src/doctor.rs` -> `src/config.rs`, `src/defaults.rs`, `src/session.rs`, `src/mcp_client.rs`.

**Experiment path:**

- `experiments/run_awl_arm.sh` discovers `experiments/tasks/*/`, runs each `setup.sh`, extracts the `dispatch` object from `task.json`, then calls `awl dispatch --apply --auto-repomap`.
- `experiments/tasks/*/setup.sh` creates files under `experiments/sandbox/<id>/` and writes Python unittest fixtures.
- `experiments/tally.py` reads `experiments/results/awl_arm.jsonl` and optional `experiments/results/baseline.csv`.
- `scripts/dispatch_cost_report.py` reads Awl dispatch JSONL logs from the configured dispatch-log directory.

## Build and Execution

**Build system:** Cargo only. No Makefile, `pyproject.toml`, `requirements.txt`, CMake, Meson, or notebook build system was found.

Common commands documented or implied by CI:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
cargo package --locked --no-verify --list
```

User/runtime commands documented in `README.md`:

```bash
awl init --profile lite --no-check
awl doctor
awl agent --task "..."
awl dispatch --level 2
awl plan --level 2
awl repomap --path . --budget 4096
awl hashline read src/main.rs
awl serve
```

Experiment commands:

```bash
scripts/dispatch_eval.sh
scripts/dispatch_cost_report.py --days 7 --avg-frontier-direct-tokens 6000 --frontier-cost-per-mtok 15
./experiments/run_awl_arm.sh
./experiments/tally.py
```

## Job Submission and Deployment

- No Slurm, PBS, Kubernetes, Docker Compose, Terraform, or HPC job submission scripts were found.
- Release deployment is GitHub Actions based: `.github/workflows/release.yml` builds target-specific binaries, packages tarballs, writes SHA-256 checksum files, and publishes a GitHub release.
- `scripts/install.sh` downloads release tarballs and installs `awl` into `${BIN_DIR:-$HOME/.local/bin}`.

## Generated and Non-Source Areas

- `target/` is Cargo/build output and contains generated local eval artifacts; it should not be treated as source.
- `experiments/sandbox/` and `experiments/results/` are generated by experiment scripts.
- `GPD/research-map/` is generated research-map output. At scan time, only the output directory existed; previous tracked map files appeared deleted in `git status`.

## Missing or Absent Project Artifacts

- No project contract file at `GPD/state.json`.
- No Jupyter notebooks.
- No Julia, C, C++, or Fortran source.
- No physics derivation source, LaTeX manuscript, BibTeX database, or stable knowledge anchor in the active registry.
- No numerical solver configuration, data pipeline manifest, or scientific input deck.

---

*Structure analysis: 2026-05-01*
