# Awl

Awl is a local-first coding agent CLI written in Rust. It runs entirely against [Ollama](https://ollama.com), keeps session state on disk, and exposes a structured tool-using agent loop instead of a single prompt-response wrapper.

## What It Does

Awl runs a five-phase agent loop:

```text
Formulate -> Plan -> Execute -> Verify -> Complete
                         ^                   |
                         +---- on failure ---+
```

The agent can read and write files, edit with hashline anchors, search the workspace, build a repository map, delegate subtasks to smaller local models, and undo the last file edit. Sessions are stored as JSONL logs so interrupted work can be resumed.

## Installation

### Recommended: release binary

```bash
curl -fsSL https://raw.githubusercontent.com/ev-watson/awl/main/scripts/install.sh | bash
```

That installs the latest release binary into `~/.local/bin` by default.

### Alternative: install with Cargo

```bash
cargo install --git https://github.com/ev-watson/awl awl --locked
```

### Alternative: build from source

```bash
git clone https://github.com/ev-watson/awl.git
cd awl
cargo build --release
./target/release/awl --version
```

## First Run

Install and start Ollama first. Then choose a model profile.

### Lite profile

This is the easiest profile to get running on smaller machines.

```bash
ollama serve
awl init --profile lite --no-check
ollama pull qwen2.5-coder:7b-instruct-q4_K_M
ollama pull qwen2.5-coder:3b-instruct-q4_K_M
awl doctor
```

### Default profile

This keeps the full 14B/7B/3B hierarchy.

```bash
ollama serve
awl init --profile default --no-check
ollama pull qwen2.5-coder:14b
ollama pull qwen2.5-coder:7b-instruct-q4_K_M
ollama pull qwen2.5-coder:3b-instruct-q4_K_M
awl doctor
```

`awl init` writes the user config file and prints the exact `ollama pull` commands for the selected configuration.

## Configuration

Awl resolves runtime configuration in this order:

1. CLI flags
2. Environment variables
3. User config file
4. Built-in defaults

The config file lives at `~/.config/awl/config.json` by default. You can inspect the active path with:

```bash
awl config path
awl config show
```

Example config:

```json
{
  "base_url": "http://127.0.0.1:11434/v1",
  "agent_model": "qwen2.5-coder:14b",
  "implementation_model": "qwen2.5-coder:7b-instruct-q4_K_M",
  "verification_model": "qwen2.5-coder:3b-instruct-q4_K_M",
  "sessions_dir": "/path/to/.config/awl/sessions"
}
```

Supported environment variables:

| Variable | Purpose |
|---|---|
| `OLLAMA_BASE_URL` | OpenAI-compatible Ollama endpoint, for example `http://127.0.0.1:11434/v1` |
| `OLLAMA_HOST` | Ollama host shorthand, for example `127.0.0.1:11434` |
| `AWL_AGENT_MODEL` | Override the top-level agent model |
| `AWL_IMPLEMENTATION_MODEL` | Override the level 2 implementation model |
| `AWL_VERIFICATION_MODEL` | Override the level 3 verification model |
| `AWL_SESSIONS_DIR` | Override the session log directory |
| `AWL_MCP_CONFIG` | Override the MCP config path |
| `AWL_CONFIG_PATH` | Override the full config file path |
| `AWL_CONFIG_DIR` | Override the config directory root |

## Profiles

| Profile | Models | Best for |
|---|---|---|
| `default` | 14B agent, 7B implementation, 3B verification | Higher quality local runs on machines with enough RAM |
| `lite` | 7B agent, 3B implementation, 3B verification | Faster setup, lower memory usage, easier first install |

You can still override any model explicitly with `awl init --agent-model ...` or environment variables.

## Usage

```bash
cd ~/myproject

awl agent --task "Implement a quicksort function in src/sort.rs with tests"
awl agent --resume <session-id>

echo '{"task":"write a binary search","context":"Rust, no unsafe"}' | awl dispatch --level 2
echo '{
  "task": "replace src/search.rs with a tested binary search helper",
  "target_path": "src/search.rs",
  "context_paths": ["src/search.rs", "tests/search_test.rs"],
  "auto_repomap": true,
  "verify_command": "cargo test search_test",
  "apply": true
}' | awl dispatch --level 2 --apply
echo '{"task":"plan a parser refactor","context":"src/parser.rs"}' | awl plan --level 2

awl repomap --path . --budget 4096
awl dispatches --list
awl dispatches --prune 30
scripts/dispatch_cost_report.py --avg-frontier-direct-tokens 6000 --frontier-cost-per-mtok 15
awl hashline read src/main.rs
awl sessions --list
awl doctor
```

Core subcommands:

| Command | Purpose |
|---|---|
| `awl init` | Write or update the user config file |
| `awl config` | Show config path or saved config |
| `awl agent` | Run the full agent loop |
| `awl dispatch` | Delegate a JSON task to level 2 or 3 |
| `awl dispatches` | Inspect local dispatch attempt logs |
| `awl plan` | Ask level 2 or 3 for an implementation plan |
| `awl repomap` | Generate a ranked repository map |
| `awl hashline` | Read or apply hashline edits |
| `awl serve` | Run Awl as an MCP server on stdio |
| `awl doctor` | Check Ollama, models, sessions, and workspace state |
| `awl sessions` | List or prune saved sessions |

`awl dispatch` can either return generated code or apply it locally. In apply mode,
Awl requires a single `target_path` or one `target_files` entry, snapshots that file,
writes the generated replacement, runs `verify_command` when provided, and restores
the previous contents if verification fails. `context_paths` lets the frontier
coordinator pass file names instead of paying tokens to paste file contents.
Dispatch results distinguish trusted local effects from model intent:
`files_changed` and `files_modified` are actual Awl writes, while non-apply
model claims are moved to `files_intended`. Each dispatch also writes a local
JSONL attempt log under the Awl config directory and returns its `dispatch_id`
and `log_path` in `telemetry`. Use `awl dispatches --prune <days>` to remove
old local attempt logs.
Use `auto_repomap: true` with optional `repomap_focus` / `repomap_budget` to
inject a small local repository map without making the frontier coordinator read
it first.

### MCP integration

`awl serve` is the stdio MCP server for Claude Code and Codex. Register it with the host client rather than running it manually in a standalone terminal:

```bash
claude mcp add --transport stdio --scope project awl -- /path/to/awl serve
codex mcp add awl -- /path/to/awl serve
```

For a source checkout, build first with `cargo build --release`, then point the MCP client at `target/release/awl`.

For Claude/Codex cost-saving workflows, prefer `awl_dispatch` level 2 or 3 with
compact `target_path`, `context_paths`, constraints, and `verify_command` fields.
Keep `awl_agent` for local-only runs or explicit experiments where a second local
orchestrator is worth the latency. The MCP server hides `awl_agent` by default;
set `AWL_ENABLE_MCP_AGENT=1` before `awl serve` only when you explicitly want to
expose the full local agent loop.

For quick local calibration, run:

```bash
scripts/dispatch_eval.sh
scripts/dispatch_cost_report.py --days 7 --avg-frontier-direct-tokens 6000 --frontier-cost-per-mtok 15
```

It exercises non-apply dispatch, apply success, and rollback behavior and prints
one JSON summary per case. The cost report reads local dispatch logs, sums local
worker tokens, and can estimate paid frontier cost avoided when you provide a
direct-frontier token baseline.

## Platform Support

- Prebuilt release binaries are produced for `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, and `aarch64-apple-darwin`.
- The core CLI is intended for macOS and Linux.
- Windows is not supported.
- `vault.sh` is optional and macOS-only.

## Optional Vaults

The repository includes `vault.sh`, a macOS helper for per-project encrypted APFS sparse bundles. It is not required for Awl itself and is not installed automatically by the release installer.

```bash
vault.sh create ~/myproject
vault.sh open ~/myproject
cd ~/myproject && awl agent --task "..."
vault.sh close ~/myproject
```

## Development

The main local quality gates are:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo package --no-verify --list
```

The repository includes issue templates, a pull request template, and code ownership for the public contribution path.

See [CONTRIBUTING.md](https://github.com/ev-watson/awl/blob/main/CONTRIBUTING.md), [CODE_OF_CONDUCT.md](https://github.com/ev-watson/awl/blob/main/CODE_OF_CONDUCT.md), [CHANGELOG.md](https://github.com/ev-watson/awl/blob/main/CHANGELOG.md), and [SECURITY.md](https://github.com/ev-watson/awl/blob/main/SECURITY.md) for repo policy and release details.

## License

MIT
