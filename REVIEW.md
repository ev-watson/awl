# Awl — Code Review Guide

This document prepares a human reviewer to evaluate the Awl codebase efficiently. It is structured in three layers: architecture overview (understand the system before reading code), annotated execution traces (follow key workflows through the source), and cross-cutting concern audits (evaluate themes that span multiple files).

**Codebase:** 15 Rust source files, ~4,600 LOC. Single binary crate.

---

## 1. Module Dependency Graph

Read modules bottom-up — leaf modules first, then composites.

```
                          main.rs
                             |
        ┌────────┬───────┬───┼────┬─────────┬──────────┬──────────┐
        v        v       v   v    v         v          v          v
    agent.rs  dispatch  plan  hashline  repomap  mcp_server  doctor  session
        |        |       |      |         |         |           |
        v        v       v      v         v         v           v
    tools.rs  llm_io  llm_io  safety   safety   agent.rs    session
        |                                         phases
        v
    safety.rs
    mcp_client.rs
```

**Leaf modules** (no project dependencies — review these first):
- `defaults.rs` (new) — shared Ollama/model defaults and endpoint normalization
- `safety.rs` (192 LOC) — workspace containment, shell command allowlisting
- `llm_io.rs` (50 LOC) — JSON sanitization, code fence stripping
- `phases.rs` (168 LOC) — phase enum, state machine, gate signal detection

**Mid-level modules** (depend only on leaves):
- `session.rs` (192 LOC) — JSONL persistence, depends on `phases`
- `mcp_client.rs` (259 LOC) — async MCP connection, standalone
- `hashline.rs` (481 LOC) — content-hashed line editing, depends on `safety`
- `repomap.rs` (623 LOC) — PageRank code map, depends on `safety`
- `dispatch.rs` (254 LOC) — Level 2/3 model dispatch, depends on `llm_io`
- `plan.rs` (152 LOC) — task decomposition, depends on `llm_io`

**Composite modules** (depend on multiple mid-level modules):
- `tools.rs` (675 LOC) — tool trait, registry, 8 implementations. Depends on `safety`, `mcp_client`
- `agent.rs` (823 LOC) — core agent loop. Depends on `phases`, `tools`, `session`, `mcp_client`
- `mcp_server.rs` (399 LOC) — JSON-RPC server. Depends on `agent`, `phases`, `session`

**Entry point:**
- `main.rs` (173 LOC) — CLI dispatch, delegates to all modules

---

## 2. Execution Traces

### Trace A: Agent Task Lifecycle

This is the primary workflow — a user runs `awl agent --task "fix the auth bug"`.

**Step 1: CLI parsing** (`main.rs:43` → `agent.rs:390-515`)

```
main.rs:43    "agent" => agent::run_agent_cli(&args[1..])
```

`run_agent_cli` parses `--task`, `--persona`, `--goal`, `--idea`, `--model`, `--mcp-config`, `--resume` flags. Constructs `AgentConfig` (lines 12-31) with defaults (model: `qwen2.5-coder:14b`, base_url: `http://127.0.0.1:11434/v1`, max_tokens: 4096, max_iterations: 30, temperature: 0.2).

Creates `PhaseState::new(&task)` (→ `phases.rs:56-68`), which initializes phase to `Formulate`.

**Step 2: Agent loop entry** (`agent.rs:38-44` → `agent.rs:62-69`)

```rust
// agent.rs:62-69 — New session setup
let system_msg = json!({"role":"system","content":build_system_prompt(phase_state)});
let user_msg = json!({"role":"user","content":initial_task});
session.write_metadata(phase_state)?;
session.append(&system_msg)?;
session.append(&user_msg)?;
vec![system_msg, user_msg]
```

`build_system_prompt` (line 348) assembles: persona → goal → task → ideas → evidence → phase prompt → tool guidance. This is the system prompt the model sees.

**Step 3: Iteration loop** (`agent.rs:73-282`)

Each iteration:

1. **Token check** (line 74): If `estimate_tokens(&messages) > 3000`, compact older messages via LLM summarization, keeping 6 most recent.

2. **API call** (lines 82-100): POST to Ollama's `/v1/chat/completions` with model, messages, tool definitions, temperature. Non-streaming.

3. **Response parsing** (lines 103-126): Extract `choices[0].message`. Capture evidence lines (`EVIDENCE:` prefix). Append to session log. Emit phase output to stderr (filtered of gate signals and JSON).

4. **Branch: No tool calls** (lines 134-231):
   - Try inline tool call parsing (model sometimes embeds JSON in content)
   - Check for gate signal (`FORMULATE_COMPLETE`, `PLAN_COMPLETE`, etc.)
   - On `Advance`: call `phase_state.advance()`, refresh system message, inject phase prompt
   - On `Regress`: call `phase_state.regress_to_execute()`, inject failure context
   - No signal: increment `consecutive_text_count`. At 3, transition to `NeedsHuman`

5. **Branch: Tool calls present** (lines 234-281):
   - Parse each tool call's name, arguments JSON, and call ID
   - Execute via `registry.execute(name, args)` (→ `tools.rs:574`)
   - Append tool result to session and messages

**Step 4: Terminal states** (`agent.rs:162-164`, `agent.rs:284-290`)

- `Phase::Complete` reached via advance: return content to stdout
- Max iterations (30) exhausted: transition to `NeedsHuman`, print session ID for resume

### Trace B: Tool Execution (Security Path)

Follow a `bash` tool call from model response to shell execution.

```
agent.rs:269     registry.execute(name, parsed_args)
  → tools.rs:574   ToolRegistry::execute(&self, name, args)
    → tools.rs:578   find tool by name in self.tools
    → tools.rs:582   check MUTATING_TOOLS — bash is mutating, so cache is cleared after
    → tools.rs:143   BashTool::execute(args)
      → tools.rs:148   safety::validate_shell_command(command)
        → safety.rs:75    validate_shell_command()
          1. Check forbidden fragments: \n, ;, `, $(
          2. Split on |, &, >, < — validate EACH segment's program
          3. First program must be in ALLOWED_SHELL_COMMANDS
          4. cargo/git subcommands validated against secondary allowlists
      → tools.rs:149   safety::workspace_root()
        → safety.rs:26    canonicalize cwd as workspace boundary
      → tools.rs:151   Command::new("bash").arg("-lc").current_dir(workspace).arg(command)
      → tools.rs:161   truncate output to 8,000 characters
```

**Key security decisions to review:**
- Allowlisted commands only (line `safety.rs:4-8`)
- Piped commands validated per-segment (`safety.rs:94-110`)
- Forbidden patterns: newlines, semicolons, backticks, `$()`  (`safety.rs:10`)
- `&&`, `||`, `|`, `>`, `<` allowed for composition
- Execution scoped to workspace root via `current_dir()`

### Trace C: File Write (Containment Path)

Follow a `write_file` tool call through workspace containment.

```
tools.rs:205     WriteFileTool::execute(args)
  → tools.rs:213   safety::resolve_path_for_write(path)
    → safety.rs:51    resolve_path_for_write()
      → safety.rs:153   absolutize(path, root) — resolve relative to workspace
        → safety.rs:161   normalize_path() — handle ../ components
      → safety.rs:54    if file exists: canonicalize and check starts_with(root)
      → safety.rs:62    if new file: find deepest existing ancestor, canonicalize that,
                         verify it's within workspace, then append remainder
      → safety.rs:141   ensure_within_workspace(canonical, root) — path.starts_with(root)
  → tools.rs:217   fs::create_dir_all(parent) — create intermediate directories
  → tools.rs:218   fs::write(resolved, content)
```

**Key containment decisions to review:**
- All paths resolved to absolute before comparison
- `..` components normalized lexically before canonicalization
- New files: ancestor chain validated even when target doesn't exist yet
- Path traversal (`../../etc/passwd`) caught by `starts_with(root)` check

### Trace D: Phase State Machine

Follow a complete task through all phases.

```
phases.rs:16-24   Phase::next() defines the progression:
  Formulate → Plan → Execute → Verify → Complete
  Complete | NeedsHuman → None (terminal)

phases.rs:70-77   PhaseState::advance() — move to next phase, return new phase
phases.rs:79-86   PhaseState::regress_to_execute() — Verify failure path
                  Limited to MAX_REGRESSIONS (2) to prevent infinite loops

phases.rs:135-154  detect_gate() — scan model output for phase-specific signals:
  Formulate: look for FORMULATE_COMPLETE
  Plan: look for PLAN_COMPLETE
  Execute: look for EXECUTE_COMPLETE
  Verify: VERIFY_COMPLETE (advance) or VERIFY_FAILED (regress)
  Complete/NeedsHuman: no signals (terminal)

  Case-insensitive matching via to_ascii_uppercase()
  Phase-aware: FORMULATE_COMPLETE only matches in Formulate phase
```

**Key state machine decisions to review:**
- Gate signals are text strings in model output — model must emit them
- Regression limited to 2 attempts before hard failure
- Phase prompt refreshed on every transition (`refresh_system_message`)
- Evidence survives phase transitions and compaction

---

## 3. Cross-Cutting Concern Audits

### 3.1 Error Handling

**Pattern used:** `Result<T, Box<dyn std::error::Error>>` throughout. No custom error enum.

**Where to look:**
- `agent.rs:91-96` — Ollama connection error: includes URL and suggestion
- `agent.rs:248-260` — Malformed tool arguments: logs error, continues loop
- `tools.rs:574-606` — Tool execution: errors returned as `Err(String)`, converted to `"ERROR: {e}"` in agent
- `safety.rs:75-111` — Validation errors: descriptive, include the specific forbidden operator
- `session.rs:35` — Session ID generation: `timestamp_nanos_opt().unwrap_or_default()` — defaults to 0 on overflow

**Assessment questions:**
- Is `Box<dyn Error>` sufficient for a CLI tool, or should there be a structured error type?
- Are validation errors descriptive enough for the end user?
- Are any errors silently swallowed? (Check: `cache.lock().ok()` in `tools.rs:594`)

### 3.2 Security Model

**Workspace containment** (`safety.rs`):
- All file paths resolved to absolute and validated against canonicalized workspace root
- Path traversal blocked by `starts_with()` after normalization
- Write paths: even non-existent files validated via ancestor chain

**Shell execution** (`safety.rs` + `tools.rs`):
- Explicit allowlist of ~30 commands
- Piped/chained commands: each segment validated independently
- Forbidden patterns: `;`, backticks, `$()`, newlines
- Allowed operators: `&&`, `||`, `|`, `>`, `<`
- Execution scoped to workspace via `current_dir()`

**Assessment questions:**
- Can `rm -rf /` be composed via allowed operators? (`rm` is allowlisted — review what targets it can reach given workspace scoping)
- Are redirect targets (`> /etc/passwd`) validated? (Answer: no — `>` is handled by bash, not by the tool. Workspace scoping via `current_dir` helps but doesn't fully contain redirects with absolute paths)
- Is `cargo` subcommand allowlisting sufficient? (`cargo build` can execute build scripts)

### 3.3 State Management

**Session state** (`session.rs`):
- JSONL append-only log at `~/.config/awl/sessions/{id}.jsonl`
- Metadata header lines contain serialized `PhaseState`
- On resume: last metadata line wins, all messages replayed

**Phase state** (`phases.rs`):
- `PhaseState` struct with current phase, task, regression count, persona, goal, ideas, evidence
- Mutable reference passed through agent loop
- Persisted to session metadata on every phase transition

**Tool cache** (`tools.rs`):
- In-memory LFU cache, 64 entries, `Mutex`-protected
- Cleared on any mutating tool execution
- Not persisted across sessions

**Assessment questions:**
- Is the session log tamper-safe? (Answer: no — it's a plain text file; integrity depends on filesystem permissions)
- Can stale cache entries cause incorrect behavior? (Answer: no — cache is cleared on writes)
- Is evidence accumulation bounded? (Answer: no — grows indefinitely. Could bloat system prompt on long sessions)

### 3.4 External Dependencies

| Dependency | Version | Risk Surface |
|------------|---------|--------------|
| `reqwest` | 0.12 | HTTP client for Ollama API. TLS not needed (localhost). Network-facing. |
| `tokio` | 1 | Async runtime. Well-maintained. Large dependency tree. |
| `serde` / `serde_json` | 1 | JSON parsing. Mature. No known issues. |
| `tree-sitter` | 0.24 | C FFI for AST parsing. Memory safety depends on upstream. |
| `tree-sitter-{python,rust}` | 0.23 | Grammar definitions. Pin major version. |
| `petgraph` | 0.6 | Graph algorithms for PageRank. Pure Rust. |
| `glob` | 0.3 | File globbing. Mature. |
| `walkdir` | 2.5 | Directory traversal. Mature. |
| `chrono` | 0.4 | Timestamp generation. clock feature only. |
| `rand` | 0.8 | Session ID randomness. |
| `async-trait` | 0.1 | Proc macro for async trait methods. |

**Assessment questions:**
- `tree-sitter` uses C FFI with `unsafe` internally — is the `unsafe_code = "forbid"` lint in Cargo.toml misleading? (Answer: it forbids unsafe in *this crate* only; dependencies can still use unsafe)
- Is `rand` sufficient for session ID uniqueness, or should `uuid` be used?

---

## 4. Suggested Review Order

For a first-pass review of the entire codebase, this order minimizes context-switching:

1. **`phases.rs`** (168 LOC) — Start here. Small, self-contained state machine. Establishes the mental model for everything else.

2. **`safety.rs`** (192 LOC) — Security foundation. Understand the containment model before reviewing anything that uses it.

3. **`llm_io.rs`** (50 LOC) — Tiny utility. Quick read. Needed context for dispatch/plan.

4. **`session.rs`** (192 LOC) — Persistence layer. Understand how state is saved/restored.

5. **`tools.rs`** (675 LOC) — Tool trait and all 8 implementations. This is where the agent's capabilities are defined. Review the `BashTool` and `WriteFileTool` implementations carefully (security-critical).

6. **`agent.rs`** (823 LOC) — The core loop. Read `build_system_prompt` first (line 348), then `run_agent` (line 38). This is the longest file and the most complex.

7. **`mcp_client.rs`** (259 LOC) — Async MCP protocol. Read if you want to understand external tool integration.

8. **`mcp_server.rs`** (399 LOC) — JSON-RPC server. Read if you want to understand how Claude Code integrates.

9. **`hashline.rs`** (481 LOC) — The edit format. Interesting algorithm but self-contained.

10. **`repomap.rs`** (623 LOC) — PageRank code map. Algorithmically interesting. Independent of the agent loop.

11. **`dispatch.rs`** (254 LOC) + **`plan.rs`** (152 LOC) — Level 2/3 delegation. Similar patterns.

12. **`doctor.rs`** (122 LOC) — Health checks. Quick read.

13. **`main.rs`** (173 LOC) — CLI dispatch. Read last — it's just routing.

---

## 5. Known Limitations and Trade-offs

These are intentional design decisions, not bugs. A reviewer should understand *why* before suggesting changes.

| Decision | Rationale |
|----------|-----------|
| `Box<dyn Error>` everywhere | CLI tool — structured errors add complexity without clear user benefit. Trade-off: harder to programmatically handle errors from library consumers. |
| Non-streaming API calls | Ollama's tool-use support requires non-streaming responses. Trade-off: no incremental output during long generation. |
| 4K context limit for 14B model | Hardware constraint (M2 16GB, ~9-10GB for 14B model). Compaction at 3K tokens is necessary. Trade-off: model may lose context on complex tasks. |
| Text-based gate signals | Model emits `FORMULATE_COMPLETE` etc. as plain text. Simple, debuggable. Trade-off: model can emit false signals or fail to emit them. |
| `unsafe_code = "forbid"` | Zero unsafe in this crate. Dependencies (tree-sitter) use unsafe internally. |
| No concurrent tool execution | Tools run sequentially. Simplifies state management. Trade-off: slower for independent tool calls. |
| Regression limit of 2 | Prevents infinite Execute↔Verify loops. Trade-off: complex tasks that need 3+ attempts will fail. |
| Session logs outside vault | Sessions persist across vault open/close cycles. Trade-off: session content (including code) is not encrypted at rest by default. |
