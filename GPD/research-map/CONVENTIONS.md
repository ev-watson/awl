# CONVENTIONS.md — Awl Project Methodology Conventions

**Analysis Date:** 2026-04-30
**Focus:** methodology (adapted from physics template for a Rust software engineering project)
**Project:** Awl v0.3.0 — Rust CLI + MCP server for dispatching bounded coding tasks to local Ollama models

---

## Project-Level Conventions

### Language and Toolchain

- **Language:** Rust (edition 2021)
- **Build system:** Cargo
- **Binary name:** `awl` (single binary, `src/main.rs`)
- **License:** AGPL-3.0 (note: `Cargo.toml` says MIT, `HANDOFF_TO_GPD.md` says AGPL-3.0 — discrepancy flagged below)

### Lint Configuration

Defined in `Cargo.toml` (lines 47-52):

```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
missing_panics_doc = "allow"
missing_errors_doc = "allow"
```

- `unsafe_code = "forbid"` — absolute prohibition on unsafe Rust. No exceptions.
- Clippy pedantic is enabled project-wide. CI enforces with `-D warnings` (all warnings are errors).
- Three pedantic lints are explicitly relaxed: `module_name_repetitions`, `missing_panics_doc`, `missing_errors_doc`.
- `src/dispatch.rs` has file-level `#![allow(...)]` for `clippy::doc_markdown`, `clippy::format_push_string`, `clippy::too_many_lines` (line 1).

### Release Profile

Defined in `Cargo.toml` (lines 44-46):

```toml
[profile.release]
lto = true
strip = true
```

Link-time optimization enabled. Debug symbols stripped from release binary.

### Git Workflow and Branch Protection

Source: `HANDOFF_TO_GPD.md` "Hard constraints" section.

- **Default branch:** `main` (protected)
- **Direct push to main:** unconditionally blocked. `enforce_admins: true` — even the repo owner cannot bypass.
- **Required flow:** branch off main -> commit on branch -> push branch -> `gh pr create` -> CI runs -> merge PR
- **Strict status checks:** `required_status_checks.strict: true` — PR branch must be up-to-date with main before merge
- **Required CI checks:** `checks (ubuntu-latest)` and `checks (macos-latest)` must both pass
- **Destructive operations:** Never `reset --hard`, `push --force`, `checkout .`, `branch -D` without explicit user confirmation
- **Pre-commit hooks:** Never skip (`--no-verify` is forbidden)

### Error Handling Pattern

- All public functions return `Result<T, Box<dyn std::error::Error>>` or `Result<T, String>`.
- No `unwrap()` in production code paths (enforced by clippy pedantic and code review).
- `unwrap()` / `expect()` permitted in `#[cfg(test)]` modules.
- Ollama availability is optional at runtime: the binary builds without Ollama, and returns a structured `error` dispatch result when Ollama is unreachable. No panics.

### Module Organization

Defined in `src/main.rs` (lines 6-19):

| Module | Responsibility | Lines |
|--------|---------------|-------|
| `dispatch` | Dispatch v2 contract: apply/verify/rollback, retry, telemetry | 1600 |
| `tools` | MCP tool definitions and cache | 947 |
| `agent` | L1 agent loop (CLI `awl agent`; not on dispatch hot path) | 939 |
| `repomap` | Tree-sitter repo summary, `known_rust_modules` | 701 |
| `mcp_server` | Stdio MCP server: dispatch/repomap/hashline/health/agent tools | 614 |
| `hashline` | File-and-line hashing utility | 482 |
| `main` | CLI entry point, argument parsing | 306 |
| `mcp_client` | MCP client for connecting to external servers | 260 |
| `defaults` | Level-to-model mapping, env precedence, URL normalization | 203 |
| `safety` | Path resolution, workspace containment, shell command validation | 199 |
| `session` | Agent session log persistence | 188 |
| `doctor` | Health check (`awl doctor`) | 196 |
| `phases` | Agent phase state machine | 168 |
| `config` | `~/.config/awl` config loader | 177 |
| `plan` | Plan subcommand | 148 |
| `init` | Init subcommand | 250 |
| `llm_io` | JSON sanitization, code-fence stripping | 50 |

Total: 7,428 lines of Rust.

---

## Structured Output Discipline

The dispatch contract uses OpenAI-compatible JSON-schema response format. This is explicitly load-bearing — `HANDOFF_TO_GPD.md` forbids replacing it with XML, tool calls, or other protocols without user authorization.

### Response Schema

Defined in `src/dispatch.rs:672-689` (`dispatch_response_format()`):

```json
{
  "type": "json_schema",
  "json_schema": {
    "name": "dispatch_response",
    "strict": true,
    "schema": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "status": {"type": "string", "enum": ["ok", "error"]},
        "code": {"type": "string"},
        "explanation": {"type": "string"},
        "files_modified": {"type": "array", "items": {"type": "string"}}
      },
      "required": ["status", "code", "explanation", "files_modified"]
    }
  }
}
```

- `strict: true` and `additionalProperties: false` — model must return exactly these four fields.
- `status` is an enum, not a free-form string.
- The system prompt (`SYSTEM_PROMPT` at `src/dispatch.rs:271-289`) embeds the schema textually as reinforcement.

### Response Validation

`validate_response()` at `src/dispatch.rs:489-511` performs post-hoc validation:
- Checks value is a JSON object
- Checks `status` is `"ok"` or `"error"`
- Checks `code`, `explanation`, `files_modified` are present and correctly typed
- Schema violations trigger format retry (up to `FORMAT_RETRIES = 3` at `src/dispatch.rs:22`)

### JSON Sanitization Pipeline

When raw LLM output is malformed:
1. `strip_code_fences()` (`src/llm_io.rs:3-12`) — removes markdown triple-backtick wrappers
2. `sanitize_json_strings()` (`src/llm_io.rs:14-50`) — escapes bare control characters (newlines, tabs) inside JSON string values
3. If parsing still fails, the format retry loop sends corrective feedback to the model

---

## Dispatch Contract Conventions

### Apply Mode Flow

Defined in `run_apply_flow()` at `src/dispatch.rs:745-952`:

1. **Generate** — send task to local model via `dispatch_with_retry()`
2. **Preflight** — check for hallucinated `use crate::` imports against `known_rust_modules` (Rust targets only)
3. **Snapshot** — `capture_snapshot()` saves previous file contents (or records file-did-not-exist)
4. **Write** — `write_target()` writes generated code to target path
5. **Verify** — `run_verify_command()` executes the caller-specified acceptance check
6. **Pass:** return `apply_result` with `status: "ok"`, `checks_passed: true`
7. **Fail:** `restore_snapshot()` rolls back to previous state, feed verify output back to model for next attempt
8. **Exhausted:** return `apply_result` with `status: "error"` after all attempts consumed

A write is NOT permanent until the dispatch result reports `checks_passed: true` and `status: "ok"`.

### Retry Semantics

Two distinct retry loops exist:

**Format retries** (`dispatch_with_retry()` at `src/dispatch.rs:955-1025`):
- Up to `FORMAT_RETRIES = 3` additional attempts
- Triggered by JSON parse errors or schema validation failures
- Corrective feedback includes the specific error
- These do NOT consume an apply attempt

**Apply-level retries** (`run_apply_flow()` loop):
- Controlled by `effective_max_attempts()` at `src/dispatch.rs:1110`
- Current default: 2 attempts for `apply && has_verify`, 1 otherwise
- Clamped to range [1, 5]
- Triggered by verify failures or preflight import rejections
- Each attempt is a full generate-snapshot-write-verify cycle
- **Pending change:** default dropping from 2 to 1 (user-confirmed, `HANDOFF_TO_GPD.md` item 1)

### Safety Constraints

**Path containment** (`src/safety.rs`):
- All file operations resolve through `resolve_existing_path()` or `resolve_path_for_write()`
- Both canonicalize paths and verify they are within the workspace root
- Traversal attacks (`..`) are neutralized by `normalize_path()`

**Shell command validation** (`src/safety.rs:7-11, 84-140`):
- Allowlisted commands only: `cargo`, `git`, `rg`, `grep`, `find`, `ls`, `cat`, `sed`, `awk`, `head`, `tail`, `wc`, `pwd`, `echo`, `printf`, `cut`, `sort`, `uniq`, `tr`, `basename`, `dirname`, `stat`, `file`, `python3`, `python`, `node`, `make`, `mkdir`, `touch`, `cp`, `mv`, `rm`, `diff`
- Forbidden shell operators: `\n`, `;`, backtick, `$(` — prevents command injection
- Allowed operators: `&&`, `||`, `|`, `>`, `<`
- Cargo subcommand allowlist: `build`, `check`, `clippy`, `fmt`, `metadata`, `test`, `tree`
- Git subcommand allowlist: `blame`, `branch`, `diff`, `grep`, `log`, `ls-files`, `rev-parse`, `show`, `status`

**Verify timeout** (`src/dispatch.rs:28`): `VERIFY_TIMEOUT_MS = 120_000` (120 seconds). Hardcoded; processes exceeding this are killed.

### Telemetry Format

Every dispatch writes JSONL to `~/.config/awl/dispatches/{id}.jsonl`. Events:
- `dispatch_start` — level, apply, target_path, verify_command, auto_repomap
- `model_selected` — model name
- `repomap_injected` — chars, budget
- `preflight_failed` — error string
- `preflight_unresolved_imports` — attempt, target_path, unresolved module list
- `model_response_valid` — format_attempt, raw_content, parsed, usage
- `model_response_invalid_json` — format_attempt, error, raw_content, usage
- `model_response_invalid_schema` — format_attempt, error, raw_content, usage
- `file_written` — attempt, target_path
- `verify_passed` — attempt, target_path, command
- `verify_failed` — attempt, target_path, command, output, rollback
- `verify_command_error` — attempt, target_path, error, rollback
- `apply_without_verify` — attempt, target_path
- `model_status_error` — attempt, summary
- `missing_code` — attempt
- `format_retries_exhausted` — error, usage

Top-level telemetry added to every dispatch result: `model`, `level`, `elapsed_ms`, `dispatch_id`, `log_path`.

### Model Configuration Hierarchy

Defined in `src/defaults.rs`:

| Level | Role | Default Model | Env Override | Config Override |
|-------|------|---------------|--------------|-----------------|
| 1 (agent) | L1 agent loop | `qwen2.5-coder:14b` | `AWL_AGENT_MODEL` | `config.agent_model` |
| 2 (impl) | Implementation dispatch | `qwen2.5-coder:7b-instruct-q4_K_M` | `AWL_IMPLEMENTATION_MODEL` | `config.implementation_model` |
| 3 (verify) | Verification dispatch | `qwen2.5-coder:3b-instruct-q4_K_M` | `AWL_VERIFICATION_MODEL` | `config.verification_model` |

Precedence (highest first): env var -> user config file (`~/.config/awl/config.json`) -> compiled default.

Ollama base URL precedence: `OLLAMA_BASE_URL` -> `OLLAMA_HOST` -> config file -> `http://127.0.0.1:11434/v1`.

**Pending change:** per-dispatch `model: Option<String>` override (`HANDOFF_TO_GPD.md` item 2) — not yet implemented.

---

## Naming Conventions

### Rust Code

- Snake case for functions, variables, modules: `run_apply_flow`, `dispatch_with_retry`, `effective_max_attempts`
- Pascal case for types: `DispatchOptions`, `TaskSpec`, `ChatRequest`, `FileSnapshot`
- Screaming snake case for constants: `FORMAT_RETRIES`, `DEFAULT_MAX_RETURN_CHARS`, `VERIFY_TIMEOUT_MS`
- Constants defined at module top: `src/dispatch.rs:22-28`
- Env var constants colocated with their usage: `src/defaults.rs:14-18`

### CLI Interface

- Subcommands: `dispatch`, `dispatches`, `hashline`, `repomap`, `plan`, `agent`, `init`, `config`, `serve`, `doctor`, `sessions`
- Flags use `--kebab-case`: `--level`, `--apply`, `--verify`, `--target`, `--auto-repomap`, `--repomap-focus`, `--repomap-budget`, `--max-attempts`, `--max-return-chars`

### MCP Tool Naming

All MCP tools use `awl_` prefix with snake_case:
- `awl_health` — health check
- `awl_dispatch` — dispatch tasks to local models
- `awl_repomap` — generate repo summary
- `awl_hashline` — file-and-line hashing
- `awl_agent` — agent loop (disabled by default; requires `AWL_ENABLE_MCP_AGENT=1`)

### File Organization Conventions

- Source: `src/*.rs` (flat module layout, no subdirectories)
- Experiments: `experiments/tasks/{id}/{task.json, setup.sh}` — one directory per task
- Results: `experiments/results/` (gitignored)
- Sandbox: `experiments/sandbox/` (gitignored, recreated by each task's `setup.sh`)
- Telemetry: `~/.config/awl/dispatches/*.jsonl` (local-only, not committed)
- Scripts: `scripts/` (Python analysis tools)
- Examples: `examples/` (MCP config templates)
- Claude integration: `.claude/agents/awl-worker.md`, `.claude/skills/awl-dispatch/SKILL.md`

---

## Constants and Magic Numbers

All dispatch constants defined at `src/dispatch.rs:22-28`:

| Constant | Value | Purpose |
|----------|-------|---------|
| `FORMAT_RETRIES` | 3 | Max format retry attempts for JSON/schema errors |
| `DEFAULT_MAX_RETURN_CHARS` | 4,000 | Truncation limit for code and explanation in results |
| `DEFAULT_CONTEXT_FILE_CHARS` | 8,000 | Per-file context truncation limit |
| `DEFAULT_TOTAL_CONTEXT_CHARS` | 24,000 | Total context truncation across all `context_paths` |
| `DEFAULT_FAILURE_ISSUE_CHARS` | 700 | Truncation limit for individual open_issues entries |
| `VERIFY_TIMEOUT_MS` | 120,000 | Verify command timeout in milliseconds |

Model token limits in `src/defaults.rs:68-73`:

| Level | Max Tokens |
|-------|-----------|
| 2 (impl) | 8,192 |
| 3 (verify) | 4,096 |

Repomap budget default: `DEFAULT_REPOMAP_BUDGET = 4096` characters (`src/defaults.rs:12`).

---

## Known Convention Issues

1. **License discrepancy:** `Cargo.toml` declares `license = "MIT"` (line 5), but `HANDOFF_TO_GPD.md` states "License: see `LICENSE` (AGPL-3.0)". These are fundamentally incompatible licenses. Must resolve before any release.

2. **No `CLAUDE.md`:** The repo has `.claude/agents/awl-worker.md` and `.claude/skills/awl-dispatch/SKILL.md` committed, and `.claude/settings.local.json` present, but no top-level `CLAUDE.md` for general repo conventions visible to Claude Code.

---

_Analysis performed on commit `29ef94f` (main)._
