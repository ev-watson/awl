# Awl Dispatch Reliability Research

Date: 2026-04-30

Sources reviewed:
- Step 1 awl-arm telemetry: `experiments/results/awl_arm.jsonl`
- Failed dispatch log: `~/.config/awl/dispatches/1777586666491028000-32287.jsonl`
- `src/dispatch.rs` (1600 lines, dispatch v2 + preflight)
- `src/defaults.rs` (level → model mapping)
- `scripts/dispatch_cost_report.py` (existing failure taxonomy)
- Local Ollama inventory: `qwen2.5-coder:14b` (9.0 GB), `qwen2.5-coder:7b-instruct-q4_K_M` (4.7 GB), `qwen2.5-coder:3b-instruct-q4_K_M` (1.9 GB)

## Why this report exists

The L2 implementation tier (7B-q4) failed `01_string_helper` deterministically. Both attempts produced the same `'\n'.join(line.capitalize() for line in text.splitlines())` solution, which drops trailing newlines, even after retry feedback included the failed test output. This is signal, not noise: 7B-q4 has a real edge-case capability gap, and the in-loop retry burned ~2200 extra tokens reproducing the same wrong answer. The user's instinct is correct: when the failure is a model capability gap (not a transient flake), local retry is pure waste — frontier rollback would have handled it once, our retry doubled the cost without changing the outcome.

The Step 1 experiment is **halted** pending the architectural work below. We do not want to scale measurement on top of a defective retry policy and a non-overridable model selection — that would measure the wrong thing.

## Question 1: Should L2 default to 14B instead of 7B?

Today (`src/defaults.rs:6`):
```
DEFAULT_IMPLEMENTATION_MODEL = "qwen2.5-coder:7b-instruct-q4_K_M"
```

7B-q4 is fast (~15–30s wall on these tasks on this machine) but has known capability gaps on subtle test cases. 14B is already on disk and used as the L1 agent model.

### Tradeoff

| factor                              | 7B-q4              | 14B (full)                  |
|-------------------------------------|--------------------|-----------------------------|
| disk                                | 4.7 GB             | 9.0 GB                      |
| typical dispatch wall (these tasks) | 15–30s observed    | ~30–60s likely, unmeasured  |
| memory pressure                     | low                | medium                      |
| edge-case handling                  | poor (01 fails)    | better, by reputation       |
| 01 trailing-newline test            | failed twice       | unmeasured                  |

The honest answer: **we don't know without measurement**. The asymmetry that matters:
- If 14B catches what 7B misses → fewer dispatch failures → fewer frontier rollbacks → net frontier-token savings, even if local wall is 2x.
- If 14B is comparable on success rate but 2x slower → frontier session stays open longer, frontier-side context overhead grows, and we lose.

### Decision

The frontier picks the model upfront, per-dispatch, based on its own risk assessment of the task. Awl does not auto-escalate. Concretely:

1. **Make the model overridable per-dispatch.** `configured_model_for_level` (`defaults.rs:74`) hardcodes level → model. Add `model: Option<String>` to `DispatchOptions` so callers can pin a model directly without env-var gymnastics. The frontier decides 7B (cheap, fast) vs 14B (slower, better edge-case handling) per dispatch.
2. **Keep L2 default at 7B.** Don't change it. Frontier opts up to 14B when it judges the task fragile.
3. **Run the Step 1 sweep across 7B and 14B as separate configurations.** This calibrates the frontier's risk model — when does opting up to 14B actually pay off? Without this we are guessing.

## Question 2: Should we remove the in-loop retry?

The user's argument is correct in the case it was made for. Today (`src/dispatch.rs:1110`):
```rust
fn effective_max_attempts(raw, apply, has_verify) -> usize {
    let default = if apply && has_verify { 2 } else { 1 };
    raw.unwrap_or(default).clamp(1, 5)
}
```
So `apply` mode with a verify command retries up to twice by default. The 01 failure used both attempts identically.

But not all failures are equivalent. There are five distinct categories:

| failure kind                       | retry yield with same model      | should retry by default? |
|------------------------------------|----------------------------------|--------------------------|
| JSON parse error                   | high — usually a transient flake | yes (already separate, in `dispatch_with_retry`) |
| Schema error (missing field)       | high — corrective prompt fixes   | yes                      |
| Preflight unresolved import        | high — concrete repo info helps  | yes                      |
| Verify failure (capability gap)    | low — same blind spot            | **no**                   |
| Verify failure (off-by-one in code)| medium — sometimes self-corrects | maybe (opt-in)           |

The current code conflates these. The verify-retry case is the one the user is criticizing.

### Decision

1. **Default `effective_max_attempts` for `apply && has_verify` from 2 → 1.** Caller can still opt in by passing `max_attempts: 2` if they have a specific reason. One-line change.
2. **No auto-escalation.** When the impl model fails on its single attempt, return the failure to the frontier and let frontier decide what to do (re-dispatch with 14B, take over the task itself, etc.). The user's reasoning: even one retry is extreme local-token cost relative to its expected yield, and the frontier will handle the failure anyway with better judgment than our local heuristic.
3. **Keep format/schema/preflight retries.** They live in `dispatch_with_retry` (already separate from the apply-attempt loop) and remain cheap and high-yield. These do not consume an apply attempt — they are pre-attempt format reconciliation.

This is a behavioral change visible to callers — frontier sees fewer retries by default. Document in the dispatch CLI help and the `awl-dispatch` skill.

## Other robustness gaps surfaced

In priority order. **Bold** items are recommended for the next implementation cycle; the rest are deferred with rationale.

1. **Failure taxonomy in telemetry.** `dispatch_cost_report.py` already maps events to `ERROR_EVENTS` / `SUCCESS_EVENTS` but does not roll them up by category. Add `failure_category` ∈ {`format`, `schema`, `preflight`, `verify`, `timeout`, `network`, `unknown`} to `apply_result` and `error_result`. This is the data that informs frontier whether to opt up to 14B for a given task class.
2. **Per-dispatch model override.** Same as Question 1 recommendation 1 — needed for the sweep and for frontier-side risk-based model selection.
3. **Cost telemetry split by input/output tokens.** Today `tally.py` and `dispatch_cost_report.py` take one `--cost-per-mtok` rate. Real Claude Opus 4.7 pricing is asymmetric: $5/MTok input, $25/MTok output. Update tally to accept `--input-cost-per-mtok` and `--output-cost-per-mtok` and read `prompt_tokens` / `completion_tokens` from the per-attempt `usage` field. Without this, savings reporting under- or over-estimates the real avoided spend depending on I/O ratio.
4. *Verify timeout.* Hardcoded at 120s (`dispatch.rs:28`). Some tasks (compile-then-test) need more. Make it `verify_timeout_ms: Option<u64>` per dispatch. Defer until a real verify command actually exceeds 120s.
5. *Per-dispatch local-token ceiling.* Caller passes `max_local_tokens`; dispatch aborts before the next model call if approaching cap. Defer — useful only when local cost matters; today it does not.
6. *Cache by (task fingerprint, context fingerprint, model).* Useful for replays and Step 1 reproducibility, not core to the savings thesis. Defer.
7. *Python preflight.* The current preflight (`src/repomap.rs:known_rust_modules`) is Rust-only. Python equivalent (`from <module> import` against installed packages or repo modules) is more heuristic and produces more false positives. Defer until Step 1 shows it would have caught actual failures.
8. *Streaming output to frontier.* Major change to `dispatch_with_retry` and the stdio MCP transport. Defer — cosmetic until base reliability is solid.

## Proposed architectural changes (concrete patch list)

In rough effort order. Each item below is independently shippable.

1. `src/dispatch.rs:1110` — change `effective_max_attempts` default from 2 to 1 for `apply && has_verify`. One-line diff plus tests.
2. `src/dispatch.rs` — add `model: Option<String>` to `DispatchOptions`. Plumb through `src/mcp_server.rs` (dispatch tool schema), `src/main.rs` (CLI flag `--model`), and `experiments/run_awl_arm.sh` (`AWL_MODEL_OVERRIDE` env var passthrough into the dispatch JSON). `defaults::configured_model_for_level` becomes the fallback when unset.
3. `src/dispatch.rs` — add `failure_category` to `apply_result` / `error_result`. Wire each existing failure path. Update `scripts/dispatch_cost_report.py` to aggregate by category.
4. `scripts/dispatch_cost_report.py` and `experiments/tally.py` — accept separate `--input-cost-per-mtok` and `--output-cost-per-mtok` flags. Read `prompt_tokens` and `completion_tokens` from the existing `usage` field. Default to Claude Opus 4.7 standard rates: $5 / $25.
5. `experiments/run_awl_arm.sh` — accept `AWL_MODEL_OVERRIDE` env var. Run the Step 1 sweep at two configurations: 7B-only, 14B-only.

Item 1 can ship alone. Items 2–3 form one coherent PR. Item 4 is independent and small. Item 5 follows. Step 1 cannot resume meaningfully until items 1–2 land — otherwise we measure the wrong thing.

## Step 1 experiment state (do not lose)

### What was run

`./experiments/run_awl_arm.sh` against three tasks at L2 (7B-q4) on 2026-04-30:

| task                 | status | attempts | tokens | wall    | notes                                    |
|----------------------|--------|----------|--------|---------|------------------------------------------|
| `01_string_helper`   | error  | 2 (max)  | 4264   | 29686ms | failed `test_preserves_trailing_newline` both attempts |
| `02_validate_input`  | ok     | 1        | 2073   | 15431ms | all 8 tests passed first try             |
| `03_fix_off_by_one`  | ok     | 1        | 2264   | 17577ms | all 5 tests passed first try             |

Local-arm result file: `experiments/results/awl_arm.jsonl` (gitignored; safe to keep or wipe).
Per-dispatch telemetry: `~/.config/awl/dispatches/177758*.jsonl`.

### What was not run

- The manual frontier-baseline arm. No baseline data exists.
- The tally script end-to-end.
- Any 14B or escalation configuration.

### What this partial result tells us

- Pipeline is sound: apply mode, verify, telemetry, repomap injection, retry, error reporting all work end-to-end.
- 7B-q4 has at least one repeatable capability gap on easy text-manipulation tasks.
- Retry on verify failure with the same model produced zero new information for ~2200 wasted tokens.
- The harness is fit to scale once the architectural changes above land.

### Where to resume

After items 1–4 from the patch list ship:

1. Build a task pack of ≥10 (currently 3). Mix: write-from-scratch (like 01, 02), edit-existing (like 03), and harder tasks that exercise `context_paths` (no examples yet). Cover both Python and Rust generation.
2. Run `./experiments/run_awl_arm.sh` at two model configurations (7B-only, 14B-only), via `AWL_MODEL_OVERRIDE`.
3. Run the manual frontier-baseline arm per `experiments/README.md`.
4. Run `./experiments/tally.py --input-cost-per-mtok 5 --output-cost-per-mtok 25` per Awl configuration against the same baseline.
5. Compare each Awl configuration. Inform frontier-side guidance on when to opt up to 14B.

The original success thresholds (≥25–40% token reduction, ≥60–70% Awl pass rate) still apply but should be re-justified per configuration.

## Resolved decisions (from user, 2026-04-30)

- Drop default `effective_max_attempts` for `apply && has_verify` from 2 → 1. Confirmed.
- No auto-escalation. Frontier picks 7B vs 14B upfront via per-dispatch `model` override. Confirmed.
- Step 1 matrix: 7B-only and 14B-only (no escalation configuration). Confirmed.
- Cost rates: Claude Opus 4.7 standard pricing, $5/MTok input and $25/MTok output. The 1M context tier uses the same standard pricing — no long-context premium. Note: the Opus 4.7 tokenizer can produce up to ~35% more tokens for the same text vs prior Claude tokenizers, so real-world spend may rise even though the rate card is unchanged. Track input and output separately in tally output rather than blending.
