# Handoff to GPD: Awl Project

Date: 2026-04-30
From: Claude (prior session driver)
To: GPD orchestrator (Get-Physics-Done) and any Claude worker it spawns

This file is your project map. Read it first.

## What Awl is

Awl is a Rust CLI + stdio MCP server that lets a frontier coding assistant (Claude Code, Codex CLI) dispatch bounded coding tasks to local Ollama models (7B / 14B / 3B). The product hypothesis is that bounded local execution measurably saves frontier tokens on net. Whether it does is an empirical question currently being measured (Step 1 of the experiment plan, halted partway).

- Repo: `/Users/blu3/awl`
- Default branch: `main` (protected)
- License: see `LICENSE` (AGPL-3.0)
- Crate: `awl v0.3.0`

## Required reading order

1. This file.
2. `REPORT_DISPATCH_RELIABILITY.md` — most recent research artifact. Contains the architectural decisions awaiting implementation, the Step 1 experiment state, and the patch list. **This is your starting work queue.**
3. `UPDATED_PROGRESS_REPORT.md` — earlier progress narrative against the original plan. Background.
4. `experiments/README.md` — how to run Step 1 once unblocked.

Skim before doing anything else: `src/dispatch.rs` (the heart, 1600 lines), `src/defaults.rs` (model mapping), `experiments/run_awl_arm.sh`, `experiments/tally.py`.

## Mission

Continue Awl development with one north-star metric: **measurable frontier-token savings in real Claude/Codex sessions**. Everything else is secondary. Do not optimize for local model quality, dispatch reliability, or feature surface area in isolation — those are means, not ends.

The currently-blocking sub-mission is to reach a defensible Step 1 result (the A/B savings experiment). The reliability work in `REPORT_DISPATCH_RELIABILITY.md` is in service of that.

## Hard constraints

- **Never push directly to `main`.** Branch protection blocks it absolutely; `enforce_admins: true` means even the repo owner cannot bypass. The flow is: branch off main → commit on the branch → push the branch → `gh pr create` → CI runs → if main moved while the PR was open, update the branch (`git pull origin main` while on the branch, or "Update branch" in the GitHub UI) so CI re-runs against current main → merge the PR. The "must be up-to-date" rule is enforced by `required_status_checks.strict: true` and only governs PR merges, not direct pushes (which are unconditionally blocked). Required CI checks: `checks (ubuntu-latest)` and `checks (macos-latest)`.
- **Never run destructive git operations** (`reset --hard`, `push --force`, `checkout .` to discard work, `branch -D`) without explicit user confirmation in the same session.
- **Never disable or skip pre-commit hooks** (`--no-verify`).
- Do not weaken lint gates in `.github/workflows/ci.yml` (`-D warnings` stays).
- Do not introduce dependencies on external paid APIs in the worker code path. Awl's value proposition depends on local-only execution.
- All new public Rust functions need at least one unit test.
- Ollama must remain optional at compile time. The binary must build and surface a structured `error` dispatch result when Ollama is unreachable; do not crash.

## Current state (2026-04-30)

- PR #14 merged. `main` (commit `29ef94f`) contains dispatch v2, hallucinated-import preflight, the experiments harness, and the eval scripts.
- Step 1 ran partially: 3 tasks at L2 (7B-q4), 2 passed, 1 failed deterministically on a trailing-newline edge case.
- Experiment is **halted** pending architectural work surfaced by that failure.
- See `REPORT_DISPATCH_RELIABILITY.md` for the patch list and decision points.

## What to do first

Each item below is independently shippable and should be its own PR. Confirm with the user before opening a PR if the change touches dispatch contract or experiment definitions.

1. **Drop default verify-retry from 2 → 1** in `effective_max_attempts` (`src/dispatch.rs:1110`). User-confirmed. One-line diff plus test update. The frontier will handle a single failed attempt; one local retry is too expensive in tokens to be worth its yield.
2. **Per-dispatch model override** — add `model: Option<String>` to `DispatchOptions`. Plumb through `src/main.rs` (CLI flag `--model`), `src/mcp_server.rs` (dispatch tool schema), and `experiments/run_awl_arm.sh` (`AWL_MODEL_OVERRIDE` env var passthrough). `defaults::configured_model_for_level` becomes the fallback when unset. The frontier picks 7B vs 14B per dispatch based on its own task-risk assessment. **No auto-escalation.** Tests: override beats level-default; unset uses level-default.
3. **Failure taxonomy in telemetry** — add `failure_category` to `apply_result` and `error_result`. Categories: `format`, `schema`, `preflight`, `verify`, `timeout`, `network`, `unknown`. Wire each existing failure path. Update `scripts/dispatch_cost_report.py` to aggregate by category. Tests covering each category.
4. **Split cost reporting by input/output tokens** — `experiments/tally.py` and `scripts/dispatch_cost_report.py` currently take one `--cost-per-mtok` rate. Replace with `--input-cost-per-mtok` and `--output-cost-per-mtok`. Default to Claude Opus 4.7 standard rates: $5 input, $25 output. Read `prompt_tokens` / `completion_tokens` from the existing per-attempt `usage` field. The 1M context tier uses standard pricing — no long-context premium. Note: Opus 4.7's tokenizer can produce ~35% more tokens for the same text than prior Claude tokenizers, so real-world spend may rise even though the rate card is unchanged.
5. **Resume Step 1** — scale task pack to ≥10 mixed tasks (write-from-scratch, edit-existing, context-paths-required; Python and Rust). Run sweep at two configurations: 7B-only, 14B-only. Run baseline arm. Run tally per configuration. Write up. Use Step 1 results to inform frontier-side guidance on when opting up to 14B pays off.

## What NOT to do

- Do not add Python preflight, streaming dispatch output, dispatch caching, or per-task token ceilings. Deferred per the report — adding them now is scope creep.
- Do not tune any task in `experiments/tasks/` to be easier so 7B passes. The 01 failure is the kind of signal we want.
- Do not replace the OpenAI-compatible JSON-schema response format (`dispatch_response_format` in `src/dispatch.rs`) with a different protocol (XML, tool calls, etc.) without explicit user authorization. Structured-output discipline is load-bearing.
- Do not change `src/repomap.rs:known_rust_modules` without preserving its existing contract: returns module file stems, excludes `main`/`lib`, treats `mod.rs` as the parent dir name.
- Do not change `enforce_admins: true` on the branch protection settings.

## When to stop and ask the user

- Before opening any PR that changes the dispatch contract visible to callers (response shape, default behavior, tool schema).
- Before merging anything to `main`. Even after CI is green.
- Before changing experiment task definitions (`experiments/tasks/*/task.json`, `setup.sh`).
- Before declaring Step 1 done.
- When CI fails for a non-obvious reason. Do not "fix" by suppressing lints or removing tests.
- When you would otherwise take an action whose blast radius exceeds the local repo (e.g., publishing a release, modifying GitHub settings, enabling new MCP servers).

## File map

```
src/
  dispatch.rs       # dispatch v2 + preflight, retry, apply/verify/rollback (1600 lines)
  defaults.rs       # level → model mapping, env precedence
  repomap.rs        # tree-sitter repo summary, known_rust_modules
  agent.rs          # L1 agent loop (used by `awl agent` CLI; not on the dispatch hot path)
  mcp_server.rs     # stdio MCP server: dispatch / repomap / hashline / health / agent tools
  main.rs           # CLI entry
  llm_io.rs         # JSON sanitization, code-fence stripping
  safety.rs         # path resolution, shell command validation
  config.rs         # ~/.config/awl config loader
  tools.rs          # MCP tool definitions

experiments/
  README.md         # how to run Step 1
  run_awl_arm.sh    # local Awl arm driver
  tally.py          # A/B markdown report
  tasks/            # task pack (3 tasks; needs to grow to ≥10)
    01_string_helper/{task.json, setup.sh}
    02_validate_input/{task.json, setup.sh}
    03_fix_off_by_one/{task.json, setup.sh}
  results/          # gitignored: awl_arm.jsonl, baseline.csv, report.md
  sandbox/          # gitignored: per-task scratch dirs

scripts/
  dispatch_cost_report.py   # JSONL telemetry → markdown summary
  dispatch_eval.sh          # superseded by experiments/; keep for now

.claude/agents/awl-worker.md           # Claude Code subagent profile (committed)
.claude/skills/awl-dispatch/SKILL.md   # Claude Code skill for routing dispatches (committed)
examples/                              # MCP config templates for Claude Code / Codex / awl-worker

REPORT_DISPATCH_RELIABILITY.md   # current research artifact — read this second
UPDATED_PROGRESS_REPORT.md       # historical progress narrative
HANDOFF_TO_GPD.md                # this file
```

Local-only files (gitignored, present on the originating machine):
- `~/.config/awl/config.json` — model/base-url config
- `~/.config/awl/dispatches/*.jsonl` — per-dispatch telemetry logs
- `~/.config/awl/sessions/` — agent session logs
- `.mcp.json` (repo root) — frontier-side MCP wiring; hardcodes `/Users/blu3/awl/target/release/awl`

## Auth, env, and secrets

- No paid API keys are required. Ollama runs locally on `http://127.0.0.1:11434`.
- GitHub PR creation uses `gh` CLI. Verify with `gh auth status` before assuming credentials work in a fresh GPD environment.
- If GPD runs on a different machine: rebuild the binary (`cargo build --release`), regenerate `.mcp.json` from `examples/claude-code.mcp.json` with the new absolute path, and re-pull the three Ollama models (`qwen2.5-coder:14b`, `qwen2.5-coder:7b-instruct-q4_K_M`, `qwen2.5-coder:3b-instruct-q4_K_M`).

## Recently-stabilized facts (do not re-derive)

- L2 7B-q4 fails `01_string_helper`'s trailing-newline test deterministically; same-model retry is useless on this case.
- All three required Ollama models are pulled and verified by `awl doctor` on the originating machine.
- PR #14 is merged; `main` is at commit `29ef94f`.
- Branch protection on `main` requires PRs through GitHub with `enforce_admins: true` and two required CI checks. This is intentional, set up by the user, and should not be relaxed.
- Apply mode performs snapshot → write → verify → rollback. Do not assume a write is permanent until the dispatch result reports `checks_passed: true` and `status: ok`.

## Suggested GPD project initialization

If GPD's workflow expects a structured project state (phases, success criteria, etc.):

- **Roadmap source of truth:** `REPORT_DISPATCH_RELIABILITY.md` § "Proposed architectural changes" (5 patch items) + § "Where to resume" (Step 1 continuation).
- **Phase 1 goal:** ship items 1–4 of the patch list.
- **Phase 2 goal:** complete Step 1 sweep across the two model configurations (7B-only, 14B-only) and produce frontier-side guidance on per-task model selection.
- **Success criterion (project-level):** Step 1 produces a defensible answer, with at least one configuration showing ≥25% frontier-token reduction at ≥60% Awl-pass rate, OR a defensible negative result with documented blockers.
- **Verification gates:** every PR must pass `cargo clippy --all-targets -- -D warnings` and `cargo test` on Linux and macOS. The CI workflow at `.github/workflows/ci.yml` enforces this.

End of handoff.
