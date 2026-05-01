# STRUCTURE.md - Awl Project Structure Map

**Analysis Date:** 2026-05-01
**Focus:** computation
**Project:** Awl v0.3.0 - Rust CLI + stdio MCP server for bounded local-model coding dispatch

> **Adaptation note:** This file maps software structure rather than physics
> artifacts. It covers source layout, experiment assets, configuration, data
> formats, dependency relationships, and build/test commands. Existing map
> artifacts in `GPD/research-map/REFERENCES.md`, `GPD/research-map/VALIDATION.md`,
> `GPD/research-map/CONVENTIONS.md`, `GPD/research-map/FORMALISM.md`, and
> `GPD/research-map/status.md` were read and preserved by reference.

## Top-Level Layout

```text
/Users/blu3/awl/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── HANDOFF_TO_GPD.md
├── REPORT_DISPATCH_RELIABILITY.md
├── UPDATED_PROGRESS_REPORT.md
├── src/
├── scripts/
├── experiments/
├── examples/
├── .github/
├── .claude/
├── GPD/research-map/
└── vault.sh
```

Important policy and research anchors:

- `HANDOFF_TO_GPD.md`: current project handoff, product hypothesis, hard
  constraints, branch protection rules, and patch list.
- `REPORT_DISPATCH_RELIABILITY.md`: most recent research artifact; authoritative
  for retry/model/cost decisions.
- `UPDATED_PROGRESS_REPORT.md`: historical baseline and progress narrative.
- `experiments/README.md`: Step 1 A/B experiment protocol and success thresholds.
- `GPD/research-map/REFERENCES.md`: active anchor registry ANC-001 through
  ANC-012.

`GPD/state.json` is absent; `GPD/state.json.lock` exists. Therefore no project
contract was treated as authoritative.

## Rust Crate Metadata

`Cargo.toml` defines a single binary crate:

- Package: `awl`
- Version: `0.3.0`
- Edition: `2021`
- Binary: `[[bin]] name = "awl", path = "src/main.rs"`
- License field: `MIT`
- Note: `HANDOFF_TO_GPD.md` says to see `LICENSE` and reports AGPL-3.0; this
  license discrepancy is already flagged in `GPD/research-map/CONVENTIONS.md`.

Release and lint settings in `Cargo.toml`:

- `[profile.release] lto = true`, `strip = true`
- `[lints.rust] unsafe_code = "forbid"`
- `[lints.clippy] all = "warn"`, `pedantic = "warn"`
- Relaxed clippy lints: `module_name_repetitions`, `missing_panics_doc`,
  `missing_errors_doc`

Dependencies:

- Runtime/model I/O: `reqwest`, `tokio`
- Serialization: `serde`, `serde_json`
- Code mapping: `tree-sitter`, `tree-sitter-python`, `tree-sitter-rust`,
  `petgraph`
- File/tool support: `glob`, `walkdir`
- Utility: `rand`, `chrono`, `async-trait`

## Source Directory

`src/` contains the executable and all core modules. Current line counts from
inspection:

| File | Lines | Role |
|------|------:|------|
| `src/dispatch.rs` | 1600 | Dispatch v2 hot path: task parsing, prompt assembly, model call, strict JSON schema, apply/verify/rollback, retry, telemetry, dispatch log management. |
| `src/tools.rs` | 947 | Tool registry for the local `awl agent` loop: bash, file read/write/edit, search/list, repomap, dispatch, and MCP proxies. |
| `src/agent.rs` | 939 | L1 local agent loop with phase discipline, tool calls, session logging, compaction, and stall guards. Secondary to the frontier-token-savings path. |
| `src/repomap.rs` | 701 | Tree-sitter Python/Rust symbol extraction, import graph, PageRank ranking, repo-map rendering, and Rust module discovery. |
| `src/mcp_server.rs` | 614 | stdio MCP server exposing `awl_health`, `awl_dispatch`, `awl_repomap`, `awl_hashline`, and gated `awl_agent`. |
| `src/hashline.rs` | 482 | Content-hashed line display and edit application utility. |
| `src/main.rs` | 306 | CLI entry point and argument parsing for all subcommands. |
| `src/mcp_client.rs` | 260 | Client for connecting the local agent to external MCP servers. |
| `src/init.rs` | 250 | `awl init` profile/config writer. |
| `src/defaults.rs` | 203 | Default models, environment-variable precedence, Ollama URL normalization, token budgets, MCP agent gate. |
| `src/safety.rs` | 199 | Workspace path containment and shell command validation. |
| `src/doctor.rs` | 196 | `awl doctor` health checks for config, Ollama, models, sessions, workspace, MCP config. |
| `src/session.rs` | 188 | JSONL session persistence for `awl agent`. |
| `src/config.rs` | 177 | User config path resolution, load/save, CLI display. |
| `src/phases.rs` | 168 | Agent phase state machine and gate detection. |
| `src/plan.rs` | 148 | Local model planning subcommand. |
| `src/llm_io.rs` | 50 | Markdown fence stripping and JSON string sanitization. |

Total inspected Rust source lines: 7428.

## Core Module Relationships

```text
src/main.rs
  ├── dispatch::run / dispatch::run_logs
  ├── mcp_server::run_server
  ├── repomap::run
  ├── hashline::run
  ├── plan::run
  ├── agent::run_agent_cli
  ├── init::run
  ├── config::run_cli
  ├── doctor::run
  └── session list/prune

src/mcp_server.rs
  ├── dispatch::run_capture
  ├── repomap::generate
  ├── hashline::run_capture / apply_from_string
  └── agent::run_agent when AWL_ENABLE_MCP_AGENT=1

src/dispatch.rs
  ├── defaults::{configured_ollama_base_url, configured_model_for_level, max_tokens_for_level}
  ├── llm_io::{strip_code_fences, sanitize_json_strings}
  ├── safety::{resolve_existing_path, resolve_path_for_write, validate_shell_command}
  ├── repomap::{generate, known_rust_modules}
  ├── config::config_dir for dispatch logs
  └── reqwest/tokio for Ollama chat completions

src/agent.rs
  ├── tools::ToolRegistry
  ├── mcp_client::{load_mcp_config, McpClient}
  ├── phases::{PhaseState, GateSignal}
  ├── session::Session
  └── defaults for model/base-url/MCP config
```

The computational center is `src/dispatch.rs`; `src/mcp_server.rs` and
`src/main.rs` are input adapters over the same dispatch engine.

## Important Source Files

### `src/dispatch.rs`

Key definitions:

- `DispatchOptions`: CLI/MCP-side option struct with `level`, `apply`,
  `verify_command`, `target_path`, retry/output limits, and repo-map controls.
  It currently lacks `model`, which is required by the confirmed model override
  decision.
- `TaskSpec`: stdin/MCP dispatch payload with `task`, `context`, `constraints`,
  `target_path`, `target_files`, `context_paths`, `verify_command`, `apply`,
  `max_attempts`, `max_return_chars`, `auto_repomap`, `repomap_focus`,
  `repomap_budget`.
- `SYSTEM_PROMPT` and `dispatch_response_format()`: strict dispatch v2 model
  contract.
- `run_capture()`: end-to-end dispatch entry that returns JSON text.
- `run_apply_flow()`: generate/write/verify/rollback loop.
- `dispatch_with_retry()`: JSON/schema retry loop.
- `capture_snapshot()`, `write_target()`, `restore_snapshot()`: rollback
  primitives.
- `run_verify_command()`: local verifier execution with hardcoded 120s timeout.
- `apply_result()` and `error_result()`: returned result shapes. These do not
  yet include `failure_category`.

Tests live inline in `src/dispatch.rs` lines 1373-1600 and cover target
resolution, rollback, verify failure reporting, compact output, trusted
changed-file reporting, dispatch log pruning, ambiguous targets, and Rust
unresolved-import preflight.

### `src/defaults.rs`

Defines model defaults:

- `DEFAULT_AGENT_MODEL = "qwen2.5-coder:14b"`
- `DEFAULT_IMPLEMENTATION_MODEL = "qwen2.5-coder:7b-instruct-q4_K_M"`
- `DEFAULT_VERIFICATION_MODEL = "qwen2.5-coder:3b-instruct-q4_K_M"`

Defines environment variables:

- `AWL_AGENT_MODEL`
- `AWL_IMPLEMENTATION_MODEL`
- `AWL_VERIFICATION_MODEL`
- `OLLAMA_BASE_URL`
- `OLLAMA_HOST`
- `AWL_ENABLE_MCP_AGENT`

Provides URL normalization and max token budgets. L2 has 8192 max tokens; L3
has 4096.

### `src/mcp_server.rs`

Exposes MCP tools:

- `awl_health`
- `awl_dispatch`
- `awl_repomap`
- `awl_hashline`
- `awl_agent` only when `AWL_ENABLE_MCP_AGENT=1`

The `awl_dispatch` schema includes `level`, `task`, `context`, `constraints`,
`target_path`, `target_files`, `context_paths`, `verify_command`, `apply`,
`max_attempts`, `max_return_chars`, `auto_repomap`, `repomap_focus`, and
`repomap_budget`. It currently does not include `model`.

### `src/repomap.rs`

Provides:

- Directory scan over `.rs` and `.py`, skipping hidden directories, `target`,
  `__pycache__`, `node_modules`, and `.git`.
- Tree-sitter symbol extraction for Rust and Python.
- Import graph construction and PageRank.
- Token-budgeted text rendering for prompts.
- `known_rust_modules()` for Rust preflight.

### `src/safety.rs`

Provides workspace containment and shell validation. It is security-critical
because `verify_command` and the local agent `bash` tool run through this
module. Current validation allows common shell tools plus `cargo` and `git`
subcommand allowlists, and rejects newline, semicolon, backtick, and `$(`.

`GPD/research-map/VALIDATION.md` notes that `src/safety.rs` has no unit tests.

## Documentation and Research Artifacts

Primary docs:

- `README.md`: installation, configuration, CLI usage, MCP integration,
  dispatch behavior, development gates.
- `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `CHANGELOG.md`:
  repository governance and release/security process.
- `LICENSE`: repository license text.

Research artifacts:

- `HANDOFF_TO_GPD.md`: project map and active work queue. It must surface
  branch protection rules, no-paid-API constraint, success criterion, and the
  5-item patch list.
- `REPORT_DISPATCH_RELIABILITY.md`: current decision record for retry policy,
  model override, failure taxonomy, split cost accounting, and Step 1 state.
- `UPDATED_PROGRESS_REPORT.md`: background on pre-v2 failures and progress
  against the corrective plan.
- `GPD/research-map/REFERENCES.md`: active anchors, especially ANC-001 through
  ANC-012.
- `GPD/research-map/FORMALISM.md`: product hypothesis, dispatch equations,
  invariants, regimes, and failure taxonomy.
- `GPD/research-map/CONVENTIONS.md`: coding, contract, retry, telemetry, and
  workflow conventions.
- `GPD/research-map/VALIDATION.md`: tests, CI, experiment method, validation
  gaps.
- `GPD/research-map/status.md`: blockers and open empirical questions.

New files from this computation mapping pass:

- `GPD/research-map/ARCHITECTURE.md`
- `GPD/research-map/STRUCTURE.md`

## Experiment Directory

`experiments/` contains the A/B savings experiment:

```text
experiments/
├── README.md
├── run_awl_arm.sh
├── tally.py
├── tasks/
│   ├── 01_string_helper/
│   │   ├── task.json
│   │   └── setup.sh
│   ├── 02_validate_input/
│   │   ├── task.json
│   │   └── setup.sh
│   └── 03_fix_off_by_one/
│       ├── task.json
│       └── setup.sh
├── results/      # gitignored, present locally
└── sandbox/      # gitignored, generated by setup.sh
```

`experiments/README.md` specifies:

- Local arm prerequisite: Ollama running locally, L2 model pulled, `python3`.
- Awl arm output: `experiments/results/awl_arm.jsonl` and
  `experiments/results/<id>.json`.
- Manual baseline file: `experiments/results/baseline.csv`.
- Success thresholds: >=25-40% token reduction and >=60-70% usable Awl pass
  rate.

`experiments/run_awl_arm.sh`:

- Defaults `AWL_BIN` to `cargo run --quiet --`.
- Defaults `AWL_LEVEL` to `2`.
- Recreates `experiments/results/awl_arm.jsonl`.
- Runs each task's `setup.sh`.
- Pipes the `dispatch` block from `task.json` into `awl dispatch --level "$LEVEL" --apply --auto-repomap`.
- Extracts `prompt_tokens`, `completion_tokens`, `total_tokens`, status,
  attempts, model, dispatch id, and wall time.

Current tasks:

| Task | File | Type | Current dispatch traits |
|------|------|------|-------------------------|
| `01_string_helper` | `experiments/tasks/01_string_helper/task.json` | Python write-from-scratch | target `experiments/sandbox/01/textops.py`, unittest verify, `max_attempts: 2`; known 7B-q4 deterministic trailing-newline failure. |
| `02_validate_input` | `experiments/tasks/02_validate_input/task.json` | Python write-from-scratch | target `experiments/sandbox/02/validators.py`, unittest verify, `max_attempts: 2`; passed in partial Step 1. |
| `03_fix_off_by_one` | `experiments/tasks/03_fix_off_by_one/task.json` | Python edit-existing | context path `experiments/sandbox/03/test_moving_average.py`, target `experiments/sandbox/03/moving_average.py`, unittest verify, `max_attempts: 3`; passed in partial Step 1. |

Local generated files observed:

- `experiments/results/awl_arm.jsonl`
- `experiments/results/01_string_helper.json`
- `experiments/results/02_validate_input.json`
- `experiments/results/03_fix_off_by_one.json`
- `experiments/sandbox/01/*`
- `experiments/sandbox/02/*`
- `experiments/sandbox/03/*`

These are gitignored by `.gitignore`.

## Scripts

`scripts/dispatch_cost_report.py`:

- Reads dispatch JSONL logs from `~/.config/awl/dispatches` by default, with
  overrides for `AWL_CONFIG_DIR`, `XDG_CONFIG_HOME`, and `APPDATA`.
- Counts success/failure based on event names.
- Sums prompt/completion/total local worker tokens.
- Estimates avoided paid frontier cost from `--frontier-direct-tokens`,
  `--avg-frontier-direct-tokens`, and blended `--frontier-cost-per-mtok`.
- CI smoke-tests it on an empty log directory.
- Gap: no split input/output cost rates and no failure-category aggregation.

`scripts/dispatch_eval.sh`:

- Runs three local dispatch smoke cases: non-apply, apply success, and rollback.
- Writes outputs under `target/awl-eval`.
- Uses L3 by default in the scripted cases.
- Superseded by `experiments/` for the main savings experiment but still useful
  as a local smoke check.

`scripts/install.sh`:

- Release installer included in crate packaging.

## Examples and Integration Config

`examples/` contains:

- `examples/claude-code.mcp.json`: MCP config template for Claude Code.
- `examples/codex-config.toml`: Codex config template.
- `examples/awl-worker.md`: local worker profile example.

Committed Claude integration files:

- `.claude/agents/awl-worker.md`
- `.claude/skills/awl-dispatch/SKILL.md`

`.claude/settings.local.json` exists but is ignored by `.gitignore`; it was not
needed for this structure map.

Local-only integration noted by `HANDOFF_TO_GPD.md`:

- `.mcp.json` may exist locally and is gitignored.
- Active MCP users need rebuilt `target/release/awl` after source changes.

## Configuration and Runtime State

User config:

- Default path: `~/.config/awl/config.json`.
- Overrides: `AWL_CONFIG_PATH`, `AWL_CONFIG_DIR`.
- Config fields in `src/config.rs`: `base_url`, `agent_model`,
  `implementation_model`, `verification_model`, `sessions_dir`, `mcp_config`.

Runtime directories:

- Dispatch logs: `~/.config/awl/dispatches/*.jsonl` by default.
- Agent sessions: `~/.config/awl/sessions/` by default.
- Experiment results: `experiments/results/` (gitignored).
- Experiment sandboxes: `experiments/sandbox/` (gitignored).
- Build artifacts: `target/` (gitignored).

Environment variables:

- `OLLAMA_BASE_URL`: explicit OpenAI-compatible Ollama endpoint.
- `OLLAMA_HOST`: Ollama host shorthand.
- `AWL_AGENT_MODEL`: L1 model override.
- `AWL_IMPLEMENTATION_MODEL`: L2 model override.
- `AWL_VERIFICATION_MODEL`: L3 model override.
- `AWL_SESSIONS_DIR`: session log directory override.
- `AWL_MCP_CONFIG`: MCP config path override.
- `AWL_CONFIG_PATH`: full config file path override.
- `AWL_CONFIG_DIR`: config directory root override.
- `AWL_ENABLE_MCP_AGENT`: exposes `awl_agent` over MCP when set to `1`,
  `true`, `yes`, or `on`.
- `AWL_BIN`: experiment harness binary override.
- `AWL_LEVEL`: experiment harness model level override.
- `AWL_MODEL_OVERRIDE`: documented as pending in research artifacts but not
  implemented in `experiments/run_awl_arm.sh`.

## Input and Output Formats

### Dispatch stdin / MCP input

The dispatch JSON shape is defined by `TaskSpec` in `src/dispatch.rs`:

```json
{
  "task": "description of what to do",
  "context": "optional inline context",
  "constraints": ["optional hard constraints"],
  "target_path": "optional/file/to/write",
  "target_files": ["optional files in scope"],
  "context_paths": ["files Awl should read locally"],
  "verify_command": "optional command for apply mode",
  "apply": true,
  "max_attempts": 1,
  "max_return_chars": 4000,
  "auto_repomap": true,
  "repomap_focus": ["optional focus files"],
  "repomap_budget": 1200
}
```

CLI flags in `src/main.rs` can override or supplement selected fields:
`--level`, `--apply`, `--verify`, `--target-path`, `--max-attempts`,
`--max-return-chars`, `--auto-repomap`, `--repomap-focus`, and
`--repomap-budget`.

### Model response contract

The local model must produce:

```json
{
  "status": "ok",
  "code": "complete generated source",
  "explanation": "brief explanation",
  "files_modified": ["intended/path"]
}
```

`status` may be only `ok` or `error`. The schema is enforced by
`dispatch_response_format()` and `validate_response()` in `src/dispatch.rs`.

### Awl dispatch result

Apply mode returns operational fields including:

- `status`
- `summary`
- `files_changed`
- `checks_run`
- `checks_passed`
- `attempts`
- `usage`
- `open_issues`
- `code`
- `explanation`
- `files_modified`
- `telemetry`

Non-apply mode preserves generated `code` subject to truncation, moves model
claims to `files_intended`, and sets trusted `files_changed` /
`files_modified` to empty arrays.

### Dispatch telemetry JSONL

Each line in `~/.config/awl/dispatches/<dispatch_id>.jsonl` is a JSON object
with an `event` field. Important events include:

- `dispatch_start`
- `model_selected`
- `repomap_injected`
- `model_response_valid`
- `model_response_invalid_json`
- `model_response_invalid_schema`
- `format_retries_exhausted`
- `file_written`
- `verify_passed`
- `verify_failed`
- `verify_command_error`
- `preflight_failed`
- `preflight_unresolved_imports`
- `model_status_error`
- `missing_code`

### Experiment outputs

`experiments/results/awl_arm.jsonl` records one JSON object per task:

- `task_id`
- `exit_code`
- `status`
- `checks_passed`
- `attempts`
- `files_changed`
- `open_issues`
- `wall_ms`
- `model_ms`
- `prompt_tokens`
- `completion_tokens`
- `total_tokens`
- `dispatch_id`
- `model`

`experiments/results/baseline.csv` is expected but missing. Its documented
columns are `task_id,frontier_tokens,frontier_pass,wall_ms`.

## Build, Test, and Verification Commands

Local development commands from `README.md` and CI:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo package --locked --no-verify --list
python3 scripts/dispatch_cost_report.py --logs-dir target/no-dispatches
```

Build and run:

```bash
cargo build --release
./target/release/awl --version
cargo run --quiet -- dispatch --level 2
cargo run --quiet -- repomap --path . --budget 4096
cargo run --quiet -- doctor
```

Experiment commands:

```bash
./experiments/run_awl_arm.sh
AWL_BIN=target/release/awl ./experiments/run_awl_arm.sh
AWL_LEVEL=3 ./experiments/run_awl_arm.sh
./experiments/tally.py
python3 scripts/dispatch_cost_report.py --logs-dir target/no-dispatches --json
```

Optional local dispatch smoke test:

```bash
scripts/dispatch_eval.sh
```

Do not infer experiment success from these commands alone. Step 1 requires
expanded tasks, a 7B and 14B sweep after model override exists, manual
frontier-baseline data, and tally output.

## CI and Repository Policy

`.github/workflows/ci.yml` runs on pushes to `main`, pull requests, and manual
dispatch. Matrix OS targets are `ubuntu-latest` and `macos-latest`.

CI steps:

1. Checkout.
2. Install stable Rust with `rustfmt` and `clippy`.
3. Cache Rust dependencies.
4. `cargo fmt --check`.
5. `cargo clippy --all-targets -- -D warnings`.
6. `cargo test`.
7. `python3 scripts/dispatch_cost_report.py --logs-dir target/no-dispatches`.
8. Optional `scripts/dispatch_eval.sh` when `AWL_RUN_DISPATCH_EVAL=1` and
   Ollama is installed.
9. `cargo package --locked --no-verify --list` on Ubuntu only.

`HANDOFF_TO_GPD.md` records branch protection constraints:

- Never push directly to `main`.
- PR flow required.
- `enforce_admins: true`.
- Required checks: `checks (ubuntu-latest)` and `checks (macos-latest)`.
- Do not weaken `.github/workflows/ci.yml` lint gates.

## Gitignore and Generated Surfaces

`.gitignore` excludes:

- `/target/`
- editor/OS files
- `.vscode/`, `.idea/`
- `.claude/*` except `.claude/agents/**` and `.claude/skills/**`
- `.codex/`
- `.agents/`
- `.mcp.json`
- backup/orig files
- `experiments/sandbox/`
- `experiments/results/`

Because `experiments/results/` is gitignored, the local partial Step 1 data is
present in this workspace but may not exist after clone or on another machine.
Research claims based on it should cite `REPORT_DISPATCH_RELIABILITY.md` and
explicitly state that the raw result files are local generated artifacts.

## Structural Dependency Graph

```text
Cargo.toml
  -> src/main.rs
    -> src/dispatch.rs
      -> src/defaults.rs
        -> src/config.rs
      -> src/safety.rs
      -> src/llm_io.rs
      -> src/repomap.rs
      -> ~/.config/awl/dispatches/*.jsonl
    -> src/mcp_server.rs
      -> src/dispatch.rs
      -> src/repomap.rs
      -> src/hashline.rs
      -> src/agent.rs (gated)
    -> src/agent.rs
      -> src/tools.rs
      -> src/mcp_client.rs
      -> src/session.rs
      -> src/phases.rs
    -> src/doctor.rs
      -> src/defaults.rs
      -> src/config.rs
      -> src/session.rs

experiments/tasks/*/task.json
  -> experiments/run_awl_arm.sh
    -> awl dispatch
      -> experiments/results/<id>.json
      -> experiments/results/awl_arm.jsonl
  -> experiments/tally.py
    -> experiments/results/baseline.csv

scripts/dispatch_cost_report.py
  -> ~/.config/awl/dispatches/*.jsonl
```

## Structure Risks and Gaps

1. `src/dispatch.rs` is large and central. Changes for model override,
   failure taxonomy, retry policy, and telemetry all touch the same file.
2. `model` override is missing from every dispatch input layer:
   `src/dispatch.rs`, `src/main.rs`, `src/mcp_server.rs`, and
   `experiments/run_awl_arm.sh`.
3. `experiments/tally.py` and `scripts/dispatch_cost_report.py` use blended
   cost rates, which conflicts with ANC-005 split-rate accounting.
4. `experiments/tasks/` has only 3 tasks, all Python. Step 1 needs at least 10
   mixed tasks including Rust and meaningful `context_paths`.
5. `experiments/results/baseline.csv` is absent, so the A/B comparison cannot
   compute savings.
6. `src/safety.rs`, `src/llm_io.rs`, `src/init.rs`, `src/plan.rs`,
   `src/session.rs`, `src/mcp_client.rs`, and `src/doctor.rs` have no local
   unit tests according to `GPD/research-map/VALIDATION.md`; `src/safety.rs`
   is the most critical gap.
7. Gitignore excludes local experiment outputs. This is correct for generated
   data, but it means future mappers must not assume raw Step 1 files are
   available unless present locally.

## Practical Navigation Guide

For dispatch behavior, start with `src/dispatch.rs`, then inspect
`src/defaults.rs`, `src/safety.rs`, and `src/repomap.rs`.

For frontier/MCP integration, start with `src/mcp_server.rs`, then compare
`examples/claude-code.mcp.json`, `examples/codex-config.toml`, and
`.claude/skills/awl-dispatch/SKILL.md`.

For the experiment, start with `experiments/README.md`, then
`experiments/run_awl_arm.sh`, `experiments/tally.py`, and
`experiments/tasks/*/task.json`.

For current research decisions, start with `REPORT_DISPATCH_RELIABILITY.md`;
then use `HANDOFF_TO_GPD.md` for workflow constraints and
`GPD/research-map/REFERENCES.md` for anchor IDs.
