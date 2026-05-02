# Phase 1: Reliability and Measurement Patches - Research

**Researched:** 2026-05-01
**Domain:** Software systems -- Rust dispatch reliability, telemetry instrumentation, and measurement tooling for bounded local coding delegation
**Confidence:** HIGH

## Summary

Phase 1 implements four measurement-critical patches to the Awl dispatch pipeline before Step 1 of the A/B savings experiment resumes. The patches are: (1) changing the default verified-apply attempt count from two to one, (2) adding per-dispatch model override through DispatchOptions/CLI/MCP/experiment harness, (3) adding first-class `failure_category` telemetry to dispatch outcomes, and (4) replacing blended token-cost accounting with split input/output token pricing. Each patch is independently shippable and has concrete source locations, test requirements, and validation gates.

The source code, project contract, and prior research all agree on what needs to change and why. The current `src/dispatch.rs` defaults to two verified-apply attempts for `apply && has_verify`, but the user-confirmed decision in `REPORT_DISPATCH_RELIABILITY.md` says this should be one. The current `DispatchOptions` struct has no `model` field, blocking reproducible 7B-only and 14B-only sweeps. Dispatch terminal outcomes lack a `failure_category` field, preventing failure-mode diagnosis. The reporting scripts `experiments/tally.py` and `scripts/dispatch_cost_report.py` use blended cost-per-MTok rates, but Claude Opus 4.7 pricing is asymmetric ($5 input, $25 output per MTok). All four gaps are documented in the reliability report, the project contract, and the project-level pitfalls research.

A critical secondary concern is that existing task JSON files (`experiments/tasks/*/task.json`) set explicit `max_attempts` values (2 or 3), which would bypass the source default change. The planner must ensure these overrides are audited and either removed or justified before the default change is considered effective.

**Primary recommendation:** Implement the four patches as separate logical units (potentially 2-3 PRs), each with targeted Rust unit tests and Python fixture tests, validated by `cargo fmt`, `cargo test`, `cargo clippy --workspace --all-targets -- -D warnings`, and existing GitHub CI checks on both ubuntu-latest and macos-latest.

## Active Anchor References

| Anchor / Artifact | Type | Why It Matters Here | Required Action | Where It Must Reappear |
| --- | --- | --- | --- | --- |
| `HANDOFF_TO_GPD.md` | prior artifact | Defines hard constraints (branch protection, CI gates, no weakened lints, all new public functions need tests), work queue, and stop conditions | read, use | plan, execution, verification |
| `REPORT_DISPATCH_RELIABILITY.md` | benchmark / prior artifact | Contains the approved patch list, user-confirmed retry and model-override decisions, failure taxonomy design, cost-rate specifics, and Step 1 continuation plan | read, use, compare | plan, execution, verification |
| `experiments/README.md` | method / spec | Defines benchmark protocol, baseline CSV schema, pass/fail thresholds, and task constraints that patches must preserve compatibility with | read, use | plan, execution, verification |
| `experiments/results/awl_arm.jsonl` | benchmark / dataset | Only existing local-arm data (3 tasks, 2 passes, 1 repeated failure with 4264 tokens); motivates retry default change and failure taxonomy | compare, cite | plan, verification |
| `GPD/state.json` | project contract | Authoritative claims, deliverables, acceptance tests, forbidden proxies, and observables that constrain Phase 1 scope | read, use | plan, execution, verification |

**Missing or weak anchors:** None. All required anchors for Phase 1 are present and consistent. The project contract in `GPD/state.json` is now populated and agrees with the reliability report and handoff document on all Phase 1 deliverables.

## Conventions

| Choice | Convention | Alternatives | Source |
| --- | --- | --- | --- |
| Project type | Software systems / empirical measurement (non-physical) | N/A | `GPD/state.json` custom:project_type |
| Token units | Provider-reported tokens, split input/output | Blended total tokens | `GPD/state.json` custom:token_units |
| Cost units | USD per million frontier tokens, split input/output | Blended $/MTok | `GPD/state.json` custom:cost_units |
| Time units | Wall-time seconds (duration_ms for ms-labeled fields) | N/A | `GPD/state.json` custom:time_units |
| Failure categories | format, schema, preflight, verify, timeout, network, model, unknown | Coarser success/error only | `GPD/state.json` custom:failure_category_enum |
| Result schema | snake_case JSON/CSV fields | N/A | `GPD/state.json` custom:result_schema_naming |
| Model naming | Exact Ollama model tag + fixed arm label | Informal tier labels | `GPD/state.json` custom:model_naming |
| Git workflow | Branch -> PR -> CI -> merge (never push to main) | Direct push | `HANDOFF_TO_GPD.md` hard constraints |
| CI evidence | Command exit status, date, branch, commit, CI check name | Informal "CI passed" claims | `GPD/state.json` custom:git_ci_evidence |

**CRITICAL:** All implementation below must preserve these conventions. In particular, split input/output token fields must replace blended totals in cost reporting, and failure_category values must use the exact enum defined in the convention lock.

## Mathematical Framework

### Key Equations and Starting Points

| Equation | Name/Description | Source | Role in This Phase |
| --- | --- | --- | --- |
| `effective_max_attempts = raw.unwrap_or(if apply && has_verify { 2 } else { 1 }).clamp(1, 5)` | Retry default policy | `src/dispatch.rs:1110-1112` | Change the `2` to `1` for `apply && has_verify` |
| `token_savings = (1 - awl_tokens / baseline_tokens) * 100` | Savings percentage | `experiments/tally.py:percent_savings` | Must work with split I/O tokens after patch |
| `cost = input_tokens * input_price_per_mtok / 1e6 + output_tokens * output_price_per_mtok / 1e6` | Split cost equation | `REPORT_DISPATCH_RELIABILITY.md` | Replace blended `total_tokens * cost_per_mtok / 1e6` |
| `model = override.unwrap_or(configured_model_for_level(level))` | Model selection with override | `REPORT_DISPATCH_RELIABILITY.md` | Add `model: Option<String>` to DispatchOptions |

### Required Techniques

| Technique | What It Does | Where Applied | Standard Reference |
| --- | --- | --- | --- |
| Rust struct field addition | Add `model: Option<String>` to `DispatchOptions` | `src/dispatch.rs`, `src/main.rs`, `src/mcp_server.rs` | Rust language reference |
| JSON schema extension | Add `model` property to MCP `awl_dispatch` input schema | `src/mcp_server.rs:server_tool_definitions` | MCP protocol, JSON Schema |
| Enum-like string field | Add `failure_category` string to dispatch result JSON | `src/dispatch.rs:apply_result`, `error_result` | Project convention lock |
| CLI flag parsing | Add `--model` flag to dispatch subcommand | `src/main.rs:parse_dispatch_options` | Existing pattern in same function |
| Environment variable passthrough | Add `AWL_MODEL_OVERRIDE` to experiment harness | `experiments/run_awl_arm.sh` | Existing `AWL_LEVEL` pattern |
| Python argparse migration | Replace `--cost-per-mtok` with `--input-cost-per-mtok` and `--output-cost-per-mtok` | `experiments/tally.py`, `scripts/dispatch_cost_report.py` | Python argparse stdlib |
| Fixture-based testing | Test tally and cost report scripts with known JSONL/CSV inputs | New test files | pytest or unittest patterns |

### Approximation Schemes

Not applicable. Phase 1 patches are exact code changes with deterministic behavior, not approximations. The split cost equation is exact arithmetic given provider-reported token counts and published rates.

## Standard Approaches

### Approach 1: Incremental Patch PRs (RECOMMENDED)

**What:** Implement each of the four patches as a focused, independently testable change. Ship as 2-3 PRs (grouping naturally related patches) rather than one monolithic PR.

**Why standard:** The handoff document explicitly states "each item below is independently shippable and should be its own PR." Small, focused PRs are easier to review, test, and merge without conflict. They also reduce blast radius if a single patch introduces a regression.

**Key steps:**

1. **Patch 1 (retry default):** Change the literal `2` to `1` in `effective_max_attempts` at `src/dispatch.rs:1111`. Update or add unit tests verifying (a) default is 1 for `apply && has_verify`, (b) default is 1 for non-apply or no-verify, (c) caller-supplied `max_attempts: 2` still works, (d) clamp to `[1, 5]` still works. Audit task JSON files for `max_attempts` overrides and document them.

2. **Patch 2 (model override):** Add `model: Option<String>` to `DispatchOptions`. In `src/dispatch.rs:run`, use `options.model.unwrap_or_else(|| configured_model_for_level(level))` instead of `configured_model_for_level(level)` alone. Add `--model` flag to `parse_dispatch_options` in `src/main.rs`. Add `"model"` property to the `awl_dispatch` MCP schema in `src/mcp_server.rs`. In `experiments/run_awl_arm.sh`, read `AWL_MODEL_OVERRIDE` env var and pass it through as a `--model` CLI flag or JSON field. Add tests: override beats level default; unset override uses level default.

3. **Patch 3 (failure category):** Add a `failure_category` string field to both `apply_result` and `error_result`. Wire each existing failure path in `run_apply_flow` and the non-apply path to set the appropriate category. The categories are: `format` (format retries exhausted), `schema` (missing code field, model status error), `preflight` (unresolved imports, missing target path), `verify` (verify checks failed), `timeout` (verify command timeout -- currently the verify command has a 120s hard timeout), `network` (Ollama unreachable or HTTP error), `model` (model returns an error status in the response), and `unknown` (catch-all). Update `scripts/dispatch_cost_report.py` to aggregate by category. Add tests for each category path.

4. **Patch 4 (split cost):** In `experiments/tally.py`, replace `--cost-per-mtok` with `--input-cost-per-mtok` (default $5) and `--output-cost-per-mtok` (default $25). Read `prompt_tokens`/`completion_tokens` from awl arm records. In `scripts/dispatch_cost_report.py`, replace `--frontier-cost-per-mtok` with split flags. Preserve total-token readability for auditing old artifacts. Add fixture tests with known input/output data.

**Known difficulties:**

- The `apply_result` function already takes 8 arguments (`#[allow(clippy::too_many_arguments)]`). Adding `failure_category` makes 9. Consider passing a struct instead, but the handoff says to keep changes focused, so a string parameter is acceptable for now.
- Task JSON `max_attempts` overrides can silently bypass the retry default change. The plan must include an audit step.
- The experiment harness `run_awl_arm.sh` constructs the dispatch input from task JSON's `dispatch` block. The `--model` flag must be injected via CLI, not JSON, to avoid modifying task specs.

### Approach 2: Monolithic PR (FALLBACK)

**What:** Combine all four patches into a single PR.

**When to switch:** Only if PR overhead is genuinely blocking and the user explicitly requests it.

**Tradeoffs:** Larger review surface, higher merge conflict risk, harder to bisect regressions, contradicts handoff guidance.

### Anti-Patterns to Avoid

- **Modifying task definitions as part of reliability patches:** Task JSON changes belong in Phase 2 (task-pack freeze). Phase 1 should audit and document existing `max_attempts` overrides but not change task specs.
- **Adding features beyond the four approved patches:** Python preflight, streaming dispatch, dispatch caching, and per-task token ceilings are explicitly out of scope per the handoff and contract.
- **Weakening CI gates to make patches pass:** Forbidden proxy `fp-weakened-gates`. If `clippy -D warnings` fails, fix the warning, do not suppress it.
- **Testing only happy paths:** Each patch must test both the default and the override/fallback behavior. The retry patch must test that `max_attempts: 2` still works when explicitly set.
- **Changing the response contract shape without user confirmation:** The handoff requires asking the user before changing the dispatch contract visible to callers. Adding `failure_category` is additive (new field, not changed field), so it does not break existing consumers, but the model override and its absence in MCP schema should be confirmed as expected.

## Existing Results to Leverage

### Established Results (DO NOT RE-DERIVE)

| Result | Exact Form | Source | How to Use |
| --- | --- | --- | --- |
| Retry default is wasteful for deterministic failures | `01_string_helper` failed identically on both attempts, burning 4264 tokens | `REPORT_DISPATCH_RELIABILITY.md`, `experiments/results/awl_arm.jsonl` | Motivates patch 1; cite directly, do not re-run the pilot |
| Failure categories design | format, schema, preflight, verify, timeout, network, unknown | `REPORT_DISPATCH_RELIABILITY.md` | Use as-is for patch 3 categories; add `model` per convention lock |
| Split cost rates | Claude Opus 4.7: $5/MTok input, $25/MTok output | `REPORT_DISPATCH_RELIABILITY.md` | Use as default values in patch 4 |
| Format/schema retries are separate from apply attempts | `dispatch_with_retry` handles format reconciliation independently | `src/dispatch.rs:dispatch_with_retry` | Do not conflate with patch 1; format retries stay at `FORMAT_RETRIES = 3` |
| Model selection precedence | CLI env -> user config -> built-in default | `src/defaults.rs:configured_string` | Override must beat this chain; use the same precedence pattern |

### Useful Intermediate Results

| Result | What It Gives You | Source | Conditions |
| --- | --- | --- | --- |
| `effective_max_attempts` implementation | Current retry policy logic, exact location | `src/dispatch.rs:1110-1112` | Line numbers may shift if prior changes are made |
| `DispatchOptions` struct | Current field set (no model field) | `src/dispatch.rs:31-41` | Add model field here |
| `parse_dispatch_options` | CLI flag parsing pattern | `src/main.rs:108-213` | Add `--model` flag following existing `--level` pattern |
| `server_tool_definitions` | MCP schema for `awl_dispatch` | `src/mcp_server.rs:41-76` | Add `model` property to `inputSchema.properties` |
| `apply_result` / `error_result` | Current output shape for dispatch results | `src/dispatch.rs:1212-1259` | Add `failure_category` field to both |
| `run_awl_arm.sh` harness | Current experiment driver, model/level handling | `experiments/run_awl_arm.sh` | Add `AWL_MODEL_OVERRIDE` passthrough |
| `tally.py` CLI | Current `--cost-per-mtok` flag | `experiments/tally.py:164-196` | Replace with split flags |
| `dispatch_cost_report.py` CLI | Current `--frontier-cost-per-mtok` flag | `scripts/dispatch_cost_report.py:37-75` | Replace with split flags |
| Existing test suite | 8 dispatch tests, defaults tests | `src/dispatch.rs:1373+`, `src/defaults.rs:141+` | Extend, do not break |

### Relevant Prior Work

| Paper/Result | Authors | Year | Relevance | What to Extract |
| --- | --- | --- | --- | --- |
| `REPORT_DISPATCH_RELIABILITY.md` | Prior Claude session | 2026 | Defines all four patches with rationale | Exact patch specifications, failure category enum, cost rates |
| `HANDOFF_TO_GPD.md` | User (prior session) | 2026 | Hard constraints, work queue, stop conditions | Branch workflow, CI requirements, forbidden changes |
| `GPD/literature/PITFALLS.md` | GPD project researcher | 2026 | Phase-mapped pitfalls including retry override contamination, blended pricing, CI blind spots | Pitfalls 3, 4, 6, 7, 9, 10 map directly to Phase 1 patches |

## Don't Re-Derive

- The retry default should be 1 for `apply && has_verify` -- user-confirmed decision in `REPORT_DISPATCH_RELIABILITY.md`. Implement, do not re-debate.
- The frontier picks 7B vs 14B per dispatch; Awl does not auto-escalate -- user-confirmed in same document. Implement override, do not add escalation logic.
- Format/schema/preflight retries live in `dispatch_with_retry` and remain separate from apply attempts -- established architecture. Do not merge them.
- Claude Opus 4.7 pricing is $5 input, $25 output per MTok -- confirmed in reliability report. Use as defaults, do not research pricing.
- Failure category enum is `format, schema, preflight, verify, timeout, network, model, unknown` -- convention lock in `GPD/state.json`. Use exactly these values.

## Computational Tools

### Core Tools

| Tool | Version/Module | Purpose | Why Standard | Fit for This Phase |
| --- | --- | --- | --- | --- |
| Rust / Cargo | stable (project uses `edition = "2021"`) | Build, test, lint, format the dispatch pipeline | Native toolchain for this crate | Perfect fit; all four Rust-side patches |
| `cargo fmt` | project default | Format check | Required by CI | Must pass before merge |
| `cargo clippy` | `--workspace --all-targets -- -D warnings` | Lint check | Required by CI | Must pass; new code must be warning-free |
| `cargo test` | project default | Unit/integration test runner | Required by CI | Existing dispatch tests + new patch tests |
| Python 3 | stdlib only (no external packages) | `tally.py` and `dispatch_cost_report.py` | Scripts use only stdlib | Patch 4 modifies these scripts |
| `gh` CLI | system-installed | PR creation and CI monitoring | Required for branch-protection workflow | Used for all PR submissions |
| Bash | system shell | `run_awl_arm.sh` experiment harness | Existing harness language | Patch 2 modifies this script |

### Supporting Tools

| Tool | Purpose | When to Use |
| --- | --- | --- |
| `python3 -m pytest` or `python3 -m unittest` | Fixture tests for tally.py and cost report | Patch 4 validation; optional if inline test functions suffice |
| `jq` | Inspect dispatch result JSON during manual verification | Ad-hoc verification of failure_category in dispatch output |
| `ollama` | Optional: smoke-test dispatches with real model after patches | Only for final integration check; not required for unit tests |

### Package / Framework Reuse Decision

All four patches modify existing code in the `awl` crate and its companion Python scripts. No new packages, frameworks, or dependencies are needed. The changes are:

- Rust: struct field additions, function parameter additions, match-arm wiring, test additions. All within existing `serde_json`, `reqwest`, `tokio` dependency set.
- Python: argparse flag changes, arithmetic changes, optional fixture test files. All stdlib.
- Bash: env var reading and CLI flag passthrough. No new dependencies.

Bespoke code is not being recommended -- these are targeted patches to an existing codebase.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
| --- | --- | --- |
| String `failure_category` field | Rust enum with serde serialization | More type-safe but requires more boilerplate; string field is consistent with existing JSON output style and keeps the patch focused |
| Adding `--model` CLI flag | Reading model from stdin JSON only | CLI flag is more ergonomic for the experiment harness and consistent with other flags like `--level` |
| Python fixture tests | No tests for tally/cost scripts | Unacceptable: these scripts produce the headline measurement result and must be tested |
| Separate `--input-cost-per-mtok` / `--output-cost-per-mtok` flags | Single `--cost-rates` JSON argument | Separate flags are simpler, more explicit, and consistent with existing single-flag pattern |

### Computational Feasibility

| Computation | Estimated Cost | Bottleneck | Mitigation |
| --- | --- | --- | --- |
| `cargo test` (full suite) | ~30-60 seconds | Compile time for test binary | Incremental compilation; only changed modules recompile |
| `cargo clippy` | ~30-60 seconds | Full crate analysis | Run once before PR, CI validates |
| Python fixture tests | < 5 seconds | None (stdlib only) | N/A |
| CI runs (ubuntu + macos) | ~5-10 minutes per PR | GitHub Actions queue | No mitigation needed; expected workflow |

**Installation / Setup:**
```bash
# No additional packages needed. Verify existing toolchain:
rustc --version   # Rust stable, edition 2021
cargo --version
python3 --version # Python 3, stdlib only
gh auth status    # GitHub CLI authenticated
```

## Validation Strategies

### Internal Consistency Checks

| Check | What It Validates | How to Perform | Expected Result |
| --- | --- | --- | --- |
| `effective_max_attempts(None, true, true)` returns 1 | Retry default change applied | Unit test in `dispatch::tests` | Returns 1, not 2 |
| `effective_max_attempts(Some(2), true, true)` returns 2 | Explicit override still works | Unit test in `dispatch::tests` | Returns 2 |
| `effective_max_attempts(None, false, false)` returns 1 | Non-apply default unchanged | Unit test in `dispatch::tests` | Returns 1 |
| `effective_max_attempts(Some(6), true, true)` returns 5 | Clamp to max 5 still works | Unit test in `dispatch::tests` | Returns 5 |
| DispatchOptions with model set overrides level default | Model override works | Unit/integration test | Override model string appears in dispatch output |
| DispatchOptions without model uses level default | Fallback preserved | Unit/integration test | Default model string appears |
| `apply_result` includes `failure_category` | Failure category in output | Unit test inspecting JSON output | Field present with expected value |
| `error_result` includes `failure_category` | Failure category in error output | Unit test inspecting JSON output | Field present with expected value |
| Each failure path in `run_apply_flow` maps to a category | No uncategorized failures | Code review + targeted tests | format, schema, preflight, verify, timeout, network, model, unknown all covered |
| `tally.py` with `--input-cost-per-mtok 5 --output-cost-per-mtok 25` | Split cost arithmetic | Python fixture test with known JSONL/CSV | Cost matches hand-calculated expected value |
| `tally.py` backward compatibility | Old `--cost-per-mtok` flag removed or deprecated | CLI validation | Clear error or deprecation message if old flag used |

### Known Limits and Benchmarks

| Limit | Parameter Regime | Known Result | Source |
| --- | --- | --- | --- |
| 7B deterministic failure on `01_string_helper` | L2 7B-q4, trailing-newline test | Failed both attempts identically | `experiments/results/awl_arm.jsonl` |
| 2/3 Awl pass rate on pilot | L2 7B-q4, 3 Python tasks | 02 and 03 passed first try | `experiments/results/awl_arm.jsonl` |
| Task JSON max_attempts overrides | tasks 01=2, 02=2, 03=3 | Bypass source default | `experiments/tasks/*/task.json` |

### Numerical Validation

| Test | Method | Tolerance | Reference Value |
| --- | --- | --- | --- |
| Split cost for 01_string_helper | input=4082, output=182, rates $5/$25 input/output | Exact (floating point) | `4082 * 5/1e6 + 182 * 25/1e6 = $0.024960` |
| Split cost for 02_validate_input | input=1931, output=142, rates $5/$25 | Exact | `1931 * 5/1e6 + 142 * 25/1e6 = $0.013205` |
| Split cost for 03_fix_off_by_one | input=2113, output=151, rates $5/$25 | Exact | `2113 * 5/1e6 + 151 * 25/1e6 = $0.014340` |
| Aggregate pilot cost (all 3 tasks) | Sum of above | Exact | `$0.052505` |
| Blended-rate equivalent for comparison | total=8601 tokens at $5/MTok | Exact | `$0.043005` (demonstrating blended underestimates by ~22%) |

### Red Flags During Computation

- `cargo clippy` produces new warnings in modified files -- fix before merging, never suppress with `#[allow]` unless the specific lint is already project-wide suppressed.
- CI fails on one platform but not the other -- investigate OS-specific behavior in path handling or process spawning.
- Existing tests break after retry default change -- likely means some test was implicitly relying on `max_attempts = 2`; fix the test to use explicit override if multi-attempt behavior is intended.
- `failure_category` field missing from some dispatch output paths -- indicates an uncovered failure branch; add the category.
- Python fixture test produces different results on different Python versions -- use integer arithmetic or explicit rounding.

## Common Pitfalls

### Pitfall 1: Task JSON max_attempts Bypasses Source Default Change

**What goes wrong:** Changing the source default from 2 to 1 has no effect on tasks that set `max_attempts` in their JSON spec. Current tasks 01, 02, and 03 all set explicit values.
**Why it happens:** `src/dispatch.rs` computes `options.max_attempts.or(spec.max_attempts)` before calling `effective_max_attempts`. The spec value overrides the default path.
**How to avoid:** Audit all `experiments/tasks/*/task.json` for `max_attempts` fields. Document the override in the PLAN. Do not remove overrides in Phase 1 (that belongs to Phase 2 task-pack freeze), but do ensure the code default path is tested independently.
**Warning signs:** After patching, `effective_max_attempts(None, true, true)` returns 1 in tests, but `01_string_helper.json` still says `"max_attempts": 2`.
**Recovery:** Phase 2 must audit and freeze task JSON before Step 1 reruns.

### Pitfall 2: Adding failure_category Without Covering All Paths

**What goes wrong:** A failure path returns a dispatch result without `failure_category`, creating inconsistent telemetry that makes failure-mode diagnosis unreliable.
**Why it happens:** `src/dispatch.rs` has many failure exit points: preflight errors, model status errors, missing code, unresolved imports, verify command errors, verify failures, timeout, network errors. It is easy to miss one.
**How to avoid:** Enumerate every `apply_result` and `error_result` call site. Map each to a failure category. Add a test or code-review checklist item that confirms every call site passes a category.
**Warning signs:** Dispatch results with `"status": "error"` but no `failure_category` field or with `"failure_category": null`.
**Recovery:** Add the missing category. The `unknown` category is the catch-all but should not be the most common.

### Pitfall 3: Model Override Leaks into Non-Override Paths

**What goes wrong:** Adding `model: Option<String>` to DispatchOptions could accidentally change model selection for callers who do not set the override, if the plumbing has a default value other than `None`.
**Why it happens:** Rust `Option<String>` defaults to `None`, which is correct, but the plumbing through CLI parsing, MCP schema, and JSON spec must all preserve `None` when no override is given.
**How to avoid:** Ensure `DispatchOptions::new()` sets `model: None`. Ensure `parse_dispatch_options` only sets it when `--model` flag is present. Ensure MCP parameter extraction uses `get("model")` with `None` fallback. Test: dispatch without `--model` uses the level default.
**Warning signs:** Existing test `cargo test` fails because model selection changed.
**Recovery:** Set the field only on explicit input.

### Pitfall 4: Blended Cost Flag Removed Without Backward Compatibility

**What goes wrong:** Users or scripts that currently pass `--cost-per-mtok` to `tally.py` get a hard error after the flag is removed.
**Why it happens:** The flag is replaced by `--input-cost-per-mtok` and `--output-cost-per-mtok`.
**How to avoid:** Either keep `--cost-per-mtok` as a deprecated alias that sets both input and output to the same value with a warning, or document the breaking change clearly. The contract says to "preserve enough total-token readability to audit old artifacts," which suggests backward-compatible deprecation.
**Warning signs:** CI or experiment scripts that reference the old flag break.
**Recovery:** Add the old flag back as deprecated.

### Pitfall 5: Weakening CI Gates

**What goes wrong:** A patch introduces a clippy warning or a failing test, and the executor suppresses the warning or removes the test to make CI pass.
**Why it happens:** Time pressure or unfamiliarity with the lint rule.
**How to avoid:** Forbidden proxy `fp-weakened-gates` in the project contract. The acceptance test `test-reliability-ci` explicitly requires all gates to pass without weakening.
**Warning signs:** New `#[allow(clippy::...)]` attributes on changed code. Tests removed or marked `#[ignore]`.
**Recovery:** Revert the suppression and fix the underlying issue.

## Level of Rigor

**Required for this phase:** Controlled software engineering with unit and fixture tests.

**Justification:** Phase 1 is code patches with deterministic expected behavior. Every change has a clear before/after, testable with unit tests and CI. The split cost arithmetic is exact floating-point calculation. No approximations, heuristics, or statistical analysis are needed. The standard of "done" is: code compiles, tests pass, CI passes, the new behavior matches the specification in `REPORT_DISPATCH_RELIABILITY.md`.

**What this means concretely:**

- Every changed function has at least one test exercising the changed behavior.
- Every new public Rust function has at least one unit test (per handoff hard constraint).
- Python scripts have fixture tests with known expected output.
- CI passes on both ubuntu-latest and macos-latest before merge.
- Code review confirms no existing tests were weakened.

## When Novel

Not applicable. All four patches are well-specified modifications to existing code with clear precedent in the codebase. No novel techniques are needed.

## Sources

### Primary (HIGH confidence)

- `REPORT_DISPATCH_RELIABILITY.md` -- user-confirmed patch specifications, failure category design, cost rates, and retry policy rationale.
- `HANDOFF_TO_GPD.md` -- hard constraints, work queue, CI requirements, and stop conditions.
- `GPD/state.json` -- project contract with claims, deliverables, acceptance tests, forbidden proxies, and convention lock.
- `src/dispatch.rs` -- current implementation of retry policy, apply flow, result construction, and telemetry.
- `src/defaults.rs` -- current model selection logic, environment variable precedence.
- `src/main.rs` -- current CLI flag parsing for dispatch subcommand.
- `src/mcp_server.rs` -- current MCP tool schema for awl_dispatch.
- `experiments/tally.py` -- current cost reporting with blended rate.
- `scripts/dispatch_cost_report.py` -- current dispatch log summarization with blended rate.
- `experiments/run_awl_arm.sh` -- current experiment harness with level-only model selection.
- `experiments/results/awl_arm.jsonl` -- pilot data motivating retry and failure taxonomy patches.

### Secondary (MEDIUM confidence)

- `GPD/literature/SUMMARY.md` -- project-level research summary, computational approach, phase ordering rationale.
- `GPD/literature/METHODS.md` -- recommended empirical methods, telemetry schema, reliability patch staging order.
- `GPD/literature/PITFALLS.md` -- phase-mapped pitfalls with specific file-backed evidence.
- `GPD/research-map/ARCHITECTURE.md` -- computational pipeline, key libraries, parallelization analysis.
- `GPD/research-map/FORMALISM.md` -- metrics, equations, approximation schemes, invariants.

### Tertiary (LOW confidence)

- None. All findings are grounded in inspected project artifacts.

## Caveats and Alternatives

**Self-critique:**

1. The research assumes the four patches are correctly specified in `REPORT_DISPATCH_RELIABILITY.md`. If the user's intent has evolved since that document was written, some specifications may need updating. However, the project contract in `GPD/state.json` was created more recently and agrees on all four deliverables, so drift risk is low.

2. The `failure_category` field mapping in `run_apply_flow` requires enumerating every failure exit point. I have identified the following paths from the source: preflight_failed (preflight), model_status_error (model), missing_code (schema), preflight_unresolved_imports (preflight), verify_command_error (verify or timeout depending on cause), verify_failed (verify), apply_without_verify (N/A -- success path), format_retries_exhausted (format), and network/HTTP errors (network). The `timeout` vs `verify` distinction for `verify_command_error` may need source-level inspection to determine if the error was a timeout specifically. Currently the verify command timeout is handled by `tokio::time::timeout` wrapping the process, and a timeout would surface as an error in `run_verify_command` -- the planner should check whether the error message distinguishes timeout from other verify command failures.

3. The split cost equation uses provider-reported token counts. The reliability report notes that Opus 4.7's tokenizer can produce ~35% more tokens for the same text than prior tokenizers. This affects absolute cost but not the savings ratio, since both arms use the same tokenizer. The planner should note this but it does not change Phase 1 implementation.

4. There is a latent tension between "preserve total-token readability for old artifacts" and "replace blended cost." The recommended approach is to keep total-token display in reports but compute cost from split tokens. Old artifacts with only total tokens would get an informational note rather than a cost estimate.

**What would change the recommendation:**

- If the user decides to change the failure category enum values, the convention lock must be updated first.
- If the user wants auto-escalation from 7B to 14B, that contradicts the confirmed decision and is out of scope.
- If CI infrastructure changes (different runners, different required checks), the validation strategy must adapt.

## Metadata

**Confidence breakdown:**

- Mathematical framework: HIGH -- all equations and code locations verified by direct source inspection.
- Standard approaches: HIGH -- patch specifications are user-confirmed and documented in multiple project artifacts.
- Computational tools: HIGH -- existing Rust/Python/Bash toolchain, no new dependencies.
- Validation strategies: HIGH -- clear unit test patterns, CI gates, and fixture test approach for every patch.

**Research date:** 2026-05-01
**Valid until:** Until `src/dispatch.rs`, `src/defaults.rs`, `src/main.rs`, `src/mcp_server.rs`, `experiments/tally.py`, `scripts/dispatch_cost_report.py`, or `experiments/run_awl_arm.sh` are modified. If any of these files change before Phase 1 planning, re-verify line numbers and function signatures.
