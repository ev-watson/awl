# Awl

A self-contained, locally-hosted agentic coding CLI written in Rust. Awl orchestrates a three-tier hierarchy of language models to perform autonomous coding tasks without any cloud dependency. All inference runs on-device via [Ollama](https://ollama.com). All data stays on your machine.

## How It Works

Awl runs a structured agent loop with **phase discipline** — a five-phase state machine that guides the model through task completion:

```
Formulate → Plan → Execute → Verify → Complete
                      ↑                   |
                      └── (on failure) ←──┘
```

During each phase, the agent has access to 9 built-in tools (file I/O, shell execution, search, code map, subtask dispatch, and edit undo) and can connect to external tools via [MCP](https://modelcontextprotocol.io). Session state is persisted to JSONL so interrupted work can be resumed.

## Architecture

| Tier | Model | Role |
|------|-------|------|
| Level 1 | Qwen2.5-Coder 14B (Q4_K_M) | Autonomous agent — orchestrates via tool-use loop |
| Level 2 | Qwen2.5-Coder 7B (Q4_K_M) | Implementation subtasks via `awl dispatch` |
| Level 3 | Qwen2.5-Coder 3B (Q4_K_M) | Verification, linting via `awl dispatch` |

When online, Claude or any upstream orchestrator can invoke Awl as an MCP server (`awl serve`), adding local model capabilities to cloud workflows.

## Features

- **Phase-disciplined agent loop** with gate signals, evidence extraction, and automatic regression on verification failure
- **9 built-in tools**: `bash`, `read_file`, `write_file`, `edit_file`, `search_files`, `list_files`, `repomap`, `dispatch`, `undo_edit`
- **PageRank-ranked code map** via tree-sitter AST parsing — token-budgeted codebase context
- **Hashline edit format** — content-hashed line references for stable, conflict-resistant file edits
- **Session persistence** — JSONL logs with full conversation replay and `--resume` support
- **Context compaction** — automatic summarization near token limits to preserve working memory
- **MCP server and client** — expose tools to upstream orchestrators, consume tools from external providers
- **Workspace containment** — all file operations scoped to the working directory; shell commands allowlisted
- **Per-project encrypted vaults** — AES-256 APFS sparse bundles via `vault.sh`
- **Fully offline** — zero network calls beyond the configured Ollama endpoint

## Requirements

- macOS with Apple Silicon (M1/M2/M3) or x86_64
- 16GB RAM minimum (14B model requires ~9-10GB)
- ~15GB free disk space (models + projects)
- Rust stable toolchain ([rustup.rs](https://rustup.rs))
- [Ollama](https://ollama.com) for local model serving

## Installation

```bash
# Install Ollama and pull models
brew install ollama
ollama pull qwen2.5-coder:14b
ollama pull qwen2.5-coder:7b-instruct-q4_K_M
ollama pull qwen2.5-coder:3b-instruct-q4_K_M

# Build Awl
git clone https://github.com/etwatson/awl.git
cd awl
cargo build --release

# Add to PATH
alias awl="$(pwd)/target/release/awl"

# Verify
awl --version
awl doctor
```

## Usage

```bash
# Start Ollama
ollama serve &

# Run the agent on a task
cd ~/myproject
awl agent --task "Implement a quicksort function in src/sort.rs with tests"

# Use persona and context files for richer agent behavior
awl agent \
  --persona "a systems engineer who writes minimal, correct Rust" \
  --task "Read instructions.md for project context, then implement the parser"

# Resume an interrupted session
awl agent --resume <session-id>

# Delegate a subtask to Level 2 (7B)
echo '{"task": "write a binary search", "context": "Rust, no unsafe"}' | awl dispatch --level 2

# Generate a ranked code map
awl repomap --path . --budget 4096

# View file with hashline anchors
awl hashline read src/main.rs

# List and manage sessions
awl sessions --list
awl sessions --prune 30

# Run health checks
awl doctor
```

Set `OLLAMA_BASE_URL` to point at a non-default Ollama host when needed. Awl accepts either the server root (for example `http://127.0.0.1:11434`) or the OpenAI-compatible base URL (`http://127.0.0.1:11434/v1`).

## Subcommands

| Command | Purpose |
|---------|---------|
| `awl agent --task "..."` | Run autonomous agent (14B, tool-use loop) |
| `awl agent --resume <id>` | Resume interrupted session |
| `awl dispatch --level {2,3}` | Send subtask to 7B or 3B (stdin JSON) |
| `awl plan --level {2,3}` | Decompose task into steps (stdin JSON) |
| `awl repomap --path . --budget N` | PageRank-ranked code map |
| `awl hashline read/apply <file>` | Content-hashed line references |
| `awl serve` | MCP server on stdio (JSON-RPC 2.0) |
| `awl doctor` | Health checks |
| `awl sessions` | Manage session logs |

## Project Structure

```
src/
  main.rs          Entry point, subcommand routing
  agent.rs         Core agent loop, tool execution, compaction, phase output
  tools.rs         Tool trait, registry, 8 built-ins, LFU cache
  repomap.rs       PageRank code map via tree-sitter + petgraph
  hashline.rs      Content-hashed line references for stable edits
  mcp_server.rs    MCP server (stdio JSON-RPC 2.0, 4 tools)
  mcp_client.rs    MCP client for external tool servers
  safety.rs        Workspace containment, command allowlisting
  session.rs       JSONL session persistence, resume, prune
  phases.rs        Phase state machine, gate detection
  dispatch.rs      Level 2/3 model invocation with retry
  plan.rs          Task decomposition via Level 2/3
  doctor.rs        Health check subcommand
  defaults.rs      Shared model, endpoint, and CLI defaults
  llm_io.rs        JSON sanitization, code fence stripping
vault.sh           Per-project AES-256 encrypted vault manager
```

## Encrypted Vaults

Awl includes `vault.sh` for per-project AES-256 encrypted storage using macOS APFS sparse bundles:

```bash
vault.sh create ~/myproject    # Create encrypted vault (sets password)
vault.sh open ~/myproject      # Decrypt and mount
cd ~/myproject && awl agent --task "..."
vault.sh close ~/myproject     # Unmount and lock
vault.sh list                  # Show all vaults
```

Each project has its own vault and password. When closed, the project directory is empty and the data is fully inaccessible. Vaults are stored at `~/.awl-vaults/<name>.sparsebundle`.

## How It Differs

Most agentic coding tools are either thin API wrappers (prompt → response, no autonomy) or massive cloud-dependent systems. Awl is:

- **Fully local**: all inference on-device, zero cloud dependency
- **Structured autonomy**: phase state machine with gate signals, not just prompt chaining
- **Three-tier hierarchy**: 14B orchestrates, 7B implements, 3B verifies — each model does what it's sized for
- **Tool-native**: the agent reads, writes, searches, and executes within a contained workspace
- **Compact**: ~4,500 lines of Rust, readable end-to-end

## License

MIT
