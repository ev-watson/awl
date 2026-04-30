---
name: awl-dispatch
description: Use awl to offload bounded implementation, verification, or repo-local execution work to local Ollama-backed models. Invoke manually when you want Claude to save frontier-model tokens on medium-sized, well-scoped tasks.
argument-hint: "[task or dispatch brief]"
disable-model-invocation: true
allowed-tools:
  - mcp__awl__awl_health
  - mcp__awl__awl_dispatch
  - mcp__awl__awl_repomap
  - mcp__awl__awl_hashline
  - Read
  - Grep
  - Glob
  - Bash(printf:*)
  - Bash(./target/release/awl dispatch:*)
---

Use `awl` as a narrow local worker, not a second planner. Claude owns orchestration, approach selection, user-facing reasoning, and final review. Awl handles bounded execution packets once Claude has decided what should be done.

## When to use it

- Medium-sized implementation tasks with explicit file or symbol scope
- Verification, lint, or review passes with crisp acceptance checks
- Repo-local context gathering before a bounded edit

## When not to use it

- Architecture decisions or ambiguous requirements
- Security-sensitive changes requiring frontier judgment
- Tiny one-line edits where delegation overhead dominates
- Tasks that require long narrative handoff

## Workflow

1. Prefer the `awl` MCP server when available.
2. Call `awl_health` first if availability is unclear.
3. If target scope is unclear, call `awl_repomap` or perform only enough local search to package the task.
4. Use `awl_dispatch` with level `2` for implementation and level `3` for verification or lint-style work.
5. Keep the handoff compact: objective, target files or symbols, relevant snippets, constraints, and acceptance checks.
6. Review the result yourself before making user-facing claims or applying conclusions.
7. Return a concise summary covering status, files changed or reviewed, checks run, and unresolved issues.

## Fallback

If the MCP server is unavailable and shell execution is available, use the repo-local CLI fallback:

```bash
printf '%s\n' '{"task":"...","context":"...","constraints":[]}' | ./target/release/awl dispatch --level 2
```

Use `--level 3` for verification-heavy work.

Task to handle:

$ARGUMENTS
