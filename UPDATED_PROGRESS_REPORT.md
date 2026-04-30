# Awl Updated Progress Report

Date: 2026-04-29

Sources reviewed:
- `/Users/blu3/Desktop/reportreport.txt`
- `/Users/blu3/Desktop/AwlUsageReport.md`
- Current checkout at `/Users/blu3/awl`

## Executive Status

Awl is now in a place where controlled frontier-token savings tests are justified.
It is not yet proven that Awl saves money in real Claude/Codex sessions, but the main blockers identified in the original usage report have been addressed enough to run a fair measurement.

The original failure was not that local models were useless. The failure was that failed local work became paid frontier work: Claude had to read malformed JSON, inspect unverified code, identify hallucinated file changes, and manually recover. That erased any savings. The current implementation changes the economics by making Awl responsible for local write, verify, rollback, logging, and compact reporting.

## Reference Baseline

`AwlUsageReport.md` identified these practical failures:

- L2 dispatch broke JSON output discipline on multi-line code.
- `files_modified` was a model self-report, not a verified side effect.
- L2 produced plausible but semantically wrong code, such as inverted predicates and hallucinated imports.
- Dispatch returned `status: ok` without running tests or checks.
- The L1 `awl agent` loop stalled in repeated text without tool calls.
- Claude/Codex were routed through L1 even though L1 should be a local substitute for, not a subordinate to, a frontier orchestrator.
- There was no measurement harness for the actual product goal: paid-token savings.

`reportreport.txt` turned that into a plan:

1. Treat Awl as a local execution worker for frontier coordinators.
2. Build dispatch v2 with explicit targets, constraints, verify commands, apply mode, compact results, and retries.
3. Add structured output.
4. Add snapshot/write/verify/rollback behavior.
5. Add local grounding through `context_paths` and repo maps.
6. De-emphasize or gate L1 in MCP.
7. Harden agent-loop stall behavior.
8. Add eval and telemetry.

## Completed Against The Plan

### Dispatch v2 contract

Implemented.

Current dispatch supports:
- `target_path`
- `target_files`
- `context_paths`
- `constraints`
- `verify_command`
- `apply`
- `max_attempts`
- `max_return_chars`
- `auto_repomap`
- `repomap_focus`
- `repomap_budget`

This directly addresses the weak original dispatch contract. Claude/Codex can now hand Awl a compact bounded task without pasting large file contents.

### Structured output discipline

Implemented through OpenAI-compatible `response_format` JSON schema.

This does not adopt the XML/tagged format suggested as an alternative in `AwlUsageReport.md`, but it attacks the same failure mode: malformed JSON and markdown-fenced responses. The remaining risk is model/provider support quality, not absence of a schema.

### Real writes, verification, and rollback

Implemented.

Apply mode now:
- Requires a single effective target path.
- Snapshots the target before writing.
- Writes generated code locally.
- Runs `verify_command` when supplied.
- Retries locally with verification feedback.
- Restores the previous file state when verification fails.

This is the most important cost-saving change because failed local work no longer has to become a paid frontier debugging session.

### Trusted changed-file reporting

Implemented.

`files_changed` and compatibility `files_modified` now represent actual Awl writes. Model claims in non-apply mode are moved to `files_intended`.

This fixes the original `files_modified` hallucination problem.

### Compact failure output and local logs

Implemented.

Dispatch now writes JSONL logs under the Awl config directory and returns `dispatch_id` / `log_path` in telemetry. The CLI has:

```bash
awl dispatches --list
awl dispatches --show <dispatch-id>
awl dispatches --tail <dispatch-id>
awl dispatches --prune <days>
```

This allows Claude/Codex to receive a compact summary while full diagnostic detail stays local.

### Local grounding

Partially implemented and now testable.

Awl supports `context_paths` and `auto_repomap`. A tree-sitter compatibility issue that caused repo-map failures was fixed by aligning `tree-sitter` to `0.25`, and a Python repo-map regression test was added.

Remaining work is quality, not plumbing: repo-map ranking and symbol grounding can be improved, especially for catching hallucinated imports before generation.

### L1 gating for frontier sessions

Implemented.

`awl_agent` is hidden from MCP by default and requires:

```bash
AWL_ENABLE_MCP_AGENT=1
```

The docs and `examples/awl-worker.md` now direct Claude/Codex toward `awl_dispatch` level 2/3 and away from L1 orchestration.

### Agent-loop hardening

Implemented.

The agent loop now has configurable limits for:
- `max_iterations`
- `max_text_without_tool`
- `max_wall_seconds`

It also detects repeated same-content text responses and aborts instead of preserving a no-progress loop.

This addresses the report's L1 failure, although L1 remains secondary to the cost-saving goal.

### Eval and cost telemetry

Implemented enough for controlled testing.

Added:
- `scripts/dispatch_eval.sh`
- `scripts/dispatch_cost_report.py`
- optional CI hook for local dispatch eval when `AWL_RUN_DISPATCH_EVAL=1`

`dispatch_eval.sh` exercises:
- non-apply dispatch
- apply success
- apply rollback

`dispatch_cost_report.py` summarizes local dispatch logs and estimates paid frontier cost avoided when supplied with a direct-frontier token baseline.

## Current Verification State

Most recent verification from the current implementation:

- `cargo fmt --check` passed.
- `cargo test` passed with 51 tests.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo package --locked --no-verify --list --allow-dirty` passed.
- `cargo run --quiet -- repomap --path . --budget 120 --focus src/dispatch.rs` produced a real repository map.
- `python3 scripts/dispatch_cost_report.py --logs-dir target/no-dispatches --json` passed.

Earlier dispatch smoke testing showed:
- non-apply dispatch returns trusted `files_changed: []` and model intent in `files_intended`.
- apply success writes and verifies a target file.
- apply rollback leaves no failed target file behind.

## Are We Ready To Test Token Savings?

Yes, for controlled testing.

No, for claiming real-world savings yet.

The project has crossed the threshold from "not a fair test" to "ready for a fair test." The original tests were biased against Awl because failures leaked into Claude/Codex review. The current version can keep most failure detail local, rollback bad writes, and return compact summaries. That means a frontier-token savings experiment can now measure the intended workflow.

The correct next test is not "does Awl always write good code?" It is:

```text
For bounded tasks, does Claude/Codex spend fewer paid tokens when it delegates to Awl
than when it performs the task directly, including retries and failures?
```

The current codebase has the mechanisms needed to answer that question.

## Remaining Risks Before Production Confidence

### 1. L2 semantic quality is still unknown

Verification and rollback prevent bad local work from becoming dirty state, but they do not make Qwen 7B semantically reliable. The original inverted predicate and hallucinated import failures may still happen. The difference is that they should now be caught locally when a good `verify_command` exists.

### 2. Savings depend on task selection

Awl will likely save tokens for:
- bounded single-file edits
- boilerplate generation
- small pure functions with tests
- simple verifier tasks
- repeated mechanical changes

Awl will likely still lose tokens for:
- architecture decisions
- ambiguous tasks
- cross-file semantic reasoning without good tests
- one-line edits where delegation overhead dominates
- tasks whose correctness cannot be checked locally

### 3. Cost report needs real frontier baselines

`dispatch_cost_report.py` can estimate savings, but it needs either:
- actual direct Claude/Codex token counts, or
- agreed per-task direct-token estimates.

Until then, it is an accounting scaffold, not evidence.

### 4. Active MCP users need a rebuilt release binary

The source tree contains the new behavior. Any already-running `target/release/awl serve` process will not pick it up until rebuilt and restarted.

## Recommended Frontier Savings Experiment

Run an A/B comparison with 10 to 20 bounded tasks.

### Task selection

Use tasks that match Awl's intended role:
- One target file.
- One clear acceptance check.
- No architecture judgment.
- Can be verified by a local command.
- Expected direct Claude/Codex implementation cost above the delegation overhead.

Example task shape:

```json
{
  "task": "Add a pure helper function and its focused unit test.",
  "target_path": "src/example.rs",
  "context_paths": ["src/example.rs", "tests/example_test.rs"],
  "constraints": ["Keep the change minimal", "No unrelated refactors"],
  "verify_command": "cargo test example_test",
  "apply": true,
  "auto_repomap": true,
  "repomap_budget": 1000
}
```

### Measurement arms

1. Direct frontier: Claude/Codex performs the task without Awl.
2. Awl-assisted: Claude/Codex packages the task, Awl attempts apply/verify, Claude/Codex only reviews compact results.

### Metrics

Record:
- paid prompt tokens
- paid response tokens
- number of frontier turns
- local Awl tokens
- local wall time
- success without direct recovery
- rollback correctness on failure
- final tests passing
- whether Claude/Codex had to inspect generated code

### Initial success criteria

Awl is succeeding if:
- failed dispatches return compactly and do not dirty the worktree.
- verified successful dispatches require little or no frontier review.
- L2 usable-as-is rate is at least about 60-70% on bounded tasks.
- paid frontier tokens drop by at least 25-40% on the task set.

## Next-Step Plan

### Step 1: Rebuild and restart the MCP binary when current use is done

Run:

```bash
cargo build --release
```

Then restart any Claude/Codex MCP configuration that points at `target/release/awl serve`.

Do not evaluate savings against an old running binary.

### Step 2: Run a small local smoke calibration

Run:

```bash
scripts/dispatch_eval.sh
```

Confirm:
- non-apply results keep `files_changed` empty.
- apply success verifies.
- apply rollback restores/removes failed output.
- logs are visible with `awl dispatches --list`.

### Step 3: Run the first controlled frontier-token experiment

Use 5 tasks first, not 20. Keep them boring and verifiable. For each task, record direct-frontier estimate and Awl-assisted token usage.

After running Awl-assisted tasks, summarize:

```bash
scripts/dispatch_cost_report.py --days 1 --avg-frontier-direct-tokens <estimate> --frontier-cost-per-mtok <price>
```

If available, replace `--avg-frontier-direct-tokens` with a real `--frontier-direct-tokens` total.

### Step 4: Tighten task-selection guidance from results

Update `examples/awl-worker.md` after the experiment with concrete thresholds:
- maximum task size
- preferred verification commands
- when to skip Awl
- when to retry once
- when to fall back directly to Claude/Codex

### Step 5: Improve repo-map grounding quality

Now that parser compatibility is fixed, improve the usefulness of injected context:
- rank exports/imports more directly around `target_path`
- surface nearby tests
- include exported symbols for mentioned modules
- consider a preflight warning for requested imports that do not exist

This targets the original hallucinated import failure.

### Step 6: Add first-class verifier conveniences

Consider small wrappers such as:

```bash
awl verify --target <file> --command <cmd>
awl lint <file>
```

This is lower priority than the A/B savings test, but it would reduce JSON-envelope overhead for common L3 verification tasks.

## Bottom Line

The implementation has completed the main corrective plan from `reportreport.txt` and addressed most high-impact failures from `AwlUsageReport.md`.

Awl is now ready for a controlled test of whether it can save frontier-model token cost. It is not yet proven to save cost in general. The next milestone should be evidence: run a bounded A/B experiment, measure paid tokens and failure recovery cost, then tune task-selection and repo-grounding based on the data.
