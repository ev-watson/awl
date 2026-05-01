# Conceptual Frameworks

**Analysis Date:** 2026-04-30

> **Adaptation note:** Awl is a software engineering project, not physics.
> "Formalism" here means the product hypothesis, dispatch contract, failure
> taxonomy, cost model, and retry/escalation decision theory. Template
> sections are mapped to their SE analogues.

## System Under Study

**Subject:** Frontier-token savings via bounded local code-execution dispatch

Awl is a Rust CLI + stdio MCP server that dispatches bounded coding tasks to
local Ollama models (3B / 7B / 14B) on behalf of a frontier coding assistant
(Claude Code, Codex CLI). The product hypothesis is that this measurably saves
frontier tokens on net.

**Scales:**

- Token budgets: 4 096 (L3 verification) to 8 192 (L2 implementation) max local
  output tokens per dispatch (`src/defaults.rs:68-73`).
- Context windows: individual context files capped at 8 000 chars; total context
  across all `context_paths` capped at 24 000 chars; repo-map budget
  200-4 096 tokens (`src/dispatch.rs:23-26`, `src/defaults.rs:10`).
- Wall time: 15-30 s observed for 7B-q4; ~30-60 s estimated for 14B
  (`REPORT_DISPATCH_RELIABILITY.md`, Question 1 table).
- Cost rates: Claude Opus 4.7 standard pricing -- $5/MTok input, $25/MTok output.
  Opus 4.7 tokenizer produces up to ~35% more tokens for the same text vs prior
  tokenizers (`HANDOFF_TO_GPD.md`, item 4; `REPORT_DISPATCH_RELIABILITY.md`,
  resolved decisions).

**Dimensionless parameters (key ratios):**

- **Awl pass rate:** passing-tasks / tasks-attempted. Target: >= 60-70%.
- **Aggregate token reduction:** 1 - (awl_tokens / baseline_tokens). Target: >= 25-40%.
- **Retry yield:** probability that a same-model retry on verify failure
  produces a different (correct) result. Empirically ~0 for capability gaps,
  higher for format/schema/preflight failures.
- **I/O cost asymmetry ratio:** output_cost / input_cost = $25 / $5 = 5x. This
  makes output-heavy frontier calls disproportionately expensive and output-light
  Awl dispatches (which only return compact JSON) disproportionately cheap.

**Degrees of Freedom (dispatch contract fields):**

- `task` (string): natural-language task description -- `src/dispatch.rs:188`
- `target_path` / `target_files`: write target(s) -- `src/dispatch.rs:194-196`
- `context_paths`: grounding files -- `src/dispatch.rs:198`
- `constraints`: list of constraints -- `src/dispatch.rs:192`
- `verify_command`: acceptance check -- `src/dispatch.rs:199`
- `apply`: boolean, enables write/verify/rollback -- `src/dispatch.rs:200`
- `max_attempts`: retry budget -- `src/dispatch.rs:201`
- `max_return_chars`: output truncation -- `src/dispatch.rs:202`
- `auto_repomap`, `repomap_focus`, `repomap_budget`: local grounding injection -- `src/dispatch.rs:203-206`

## Theoretical Framework

**Primary framework: Bounded local execution as frontier-token arbitrage**

The core thesis: for bounded tasks (single-file, verifiable, no architectural
judgment), the cost of packaging a dispatch + receiving a compact result is less
than the cost of the frontier performing the task directly. The "savings" are
frontier tokens that were never consumed because the local model handled the work.

- Formulation: A/B comparison. Arm A (Awl-assisted) measures local tokens consumed
  + frontier tokens for packaging/reviewing. Arm B (baseline) measures frontier
  tokens for direct task execution.
- File: `experiments/README.md`, `UPDATED_PROGRESS_REPORT.md` "Recommended
  Frontier Savings Experiment"

**Secondary framework: Failure-cost accounting**

Savings are only real if failed dispatches do not leak into frontier debugging
sessions. The prior failure mode (pre-dispatch-v2) was that malformed JSON,
hallucinated file changes, and unverified writes forced the frontier to read,
diagnose, and recover -- erasing any savings. Dispatch v2 addresses this with:
snapshot/write/verify/rollback, trusted `files_changed`, and compact failure
reporting.

- File: `UPDATED_PROGRESS_REPORT.md` "Executive Status" and "Remaining
  Risks Before Production Confidence"

**Supporting framework: Frontier-side dispatch routing**

The frontier (Claude Code / Codex) decides WHEN to dispatch and at WHAT model
tier. Awl does not auto-escalate. The frontier's risk assessment determines
7B vs 14B per dispatch. This is a decision-theoretic problem: the frontier
trades local wall time and dispatch overhead against expected savings.

- File: `.claude/skills/awl-dispatch/SKILL.md`, `REPORT_DISPATCH_RELIABILITY.md`
  Questions 1 and 2

## Fundamental Equations (Dispatch Contract)

### Equation Catalog

| ID | Equation / Invariant | Type | Location | Status |
|----|---------------------|------|----------|--------|
| EQ-001 | Dispatch response schema: `{status, code, explanation, files_modified}` with `status in {"ok","error"}` | Defining contract | `src/dispatch.rs:258-277` (SYSTEM_PROMPT) | Enforced via JSON schema |
| EQ-002 | `dispatch_response_format()`: OpenAI-compatible `json_schema` with `strict: true` | Defining contract | `src/dispatch.rs:672-689` | Enforced at API level |
| EQ-003 | `effective_max_attempts(raw, apply, has_verify) = raw.unwrap_or(default).clamp(1,5)` where `default = if apply && has_verify { 2 } else { 1 }` | Policy rule | `src/dispatch.rs:1110-1113` | Active; pending change to default=1 |
| EQ-004 | Apply-mode invariant: snapshot -> write -> verify -> rollback-on-failure | Defining contract | `src/dispatch.rs:842-946` (apply flow) | Verified by tests |
| EQ-005 | `files_changed` = actual Awl-written files; `files_intended` = model claims (non-apply) | Defining contract | `src/dispatch.rs:1283-1303` (`normalize_non_apply_output`) | Verified by test |
| EQ-006 | Token savings = 1 - (awl_arm_tokens / baseline_arm_tokens), computed per-task and aggregate | Derived metric | `experiments/tally.py:63` (`percent_savings`) | Implemented; not yet measured E2E |
| EQ-007 | Cost avoided = (baseline_tokens / 1e6) * cost_per_mtok | Derived metric | `experiments/tally.py:106-108` | Active; pending split by I/O |
| EQ-008 | Format retry loop: up to FORMAT_RETRIES=3 attempts for JSON parse/schema errors, separate from apply attempts | Policy rule | `src/dispatch.rs:22,958-1030` (`dispatch_with_retry`) | Active |
| EQ-009 | Preflight: `unresolved_crate_imports()` checks `use crate::X` against `known_rust_modules` | Pre-write guard | `src/dispatch.rs:420-450` | Active; Rust-only |
| EQ-010 | Verify timeout: hardcoded 120 000 ms | Policy constant | `src/dispatch.rs:27` (VERIFY_TIMEOUT_MS) | Active; deferred configurability |

### Dependency Graph

```
EQ-001 (response schema)
  +-- EQ-002 (JSON schema enforcement) -- enforces EQ-001 at API level
  +-- EQ-008 (format retry) -- recovers violations of EQ-001

EQ-003 (max attempts policy)
  +-- EQ-004 (apply flow) -- uses max_attempts to bound the retry loop

EQ-004 (apply invariant)
  +-- EQ-005 (trusted files_changed) -- consequence of snapshot/rollback
  +-- EQ-009 (preflight) -- pre-write guard within apply loop
  +-- EQ-010 (verify timeout) -- bounds verify step in apply loop

EQ-006 (savings metric)
  +-- EQ-007 (cost avoided) -- monetary projection of EQ-006
```

## Symmetries and Invariants

**Exact invariants:**

- **Rollback invariant (EQ-004):** If verify fails or errors, the file system
  returns to its pre-dispatch state. Tested by `snapshot_restore_removes_new_dispatch_file`
  and `snapshot_restore_rewrites_existing_dispatch_file` (`src/dispatch.rs:1467-1490`).
- **Trusted side-effect reporting (EQ-005):** `files_changed` / `files_modified`
  reflect only actual Awl-written files, never model self-reports. Tested by
  `non_apply_output_separates_intended_from_changed_files` (`src/dispatch.rs:1498-1530`).
- **No-external-API invariant:** Awl's worker code path never calls paid external
  APIs. All model inference is via local Ollama. (`HANDOFF_TO_GPD.md`, Hard
  constraints).

**Approximate invariants (hold under conditions):**

- **Structured output discipline:** The model is instructed to return valid JSON
  matching the schema. This works when the model + provider supports
  `response_format: json_schema`. When it doesn't, the format retry loop
  (EQ-008) provides up to 3 correction attempts. Failure mode: if the model
  fundamentally cannot produce valid JSON (degenerate output), all 4 attempts
  are wasted.
- **Savings positivity:** Net frontier-token savings > 0. This holds only when
  the dispatch packaging cost + failure-handling cost < baseline direct cost.
  Violated when: tasks are too small (delegation overhead dominates), the local
  model fails deterministically (retry wastes tokens), or failure output leaks
  into frontier debugging.

## Parameters and Couplings

**Fundamental parameters:**

| Parameter | Symbol / Key | Value | Location |
|-----------|-------------|-------|----------|
| L2 default model | `DEFAULT_IMPLEMENTATION_MODEL` | `qwen2.5-coder:7b-instruct-q4_K_M` | `src/defaults.rs:4` |
| L3 default model | `DEFAULT_VERIFICATION_MODEL` | `qwen2.5-coder:3b-instruct-q4_K_M` | `src/defaults.rs:5` |
| L1 agent model | `DEFAULT_AGENT_MODEL` | `qwen2.5-coder:14b` | `src/defaults.rs:3` |
| Max tokens (L2) | -- | 8192 | `src/defaults.rs:69` |
| Max tokens (L3) | -- | 4096 | `src/defaults.rs:70` |
| Format retries | `FORMAT_RETRIES` | 3 | `src/dispatch.rs:22` |
| Max return chars | `DEFAULT_MAX_RETURN_CHARS` | 4000 | `src/dispatch.rs:23` |
| Context per file | `DEFAULT_CONTEXT_FILE_CHARS` | 8000 | `src/dispatch.rs:24` |
| Total context chars | `DEFAULT_TOTAL_CONTEXT_CHARS` | 24000 | `src/dispatch.rs:25` |
| Failure issue chars | `DEFAULT_FAILURE_ISSUE_CHARS` | 700 | `src/dispatch.rs:26` |
| Verify timeout | `VERIFY_TIMEOUT_MS` | 120000 ms | `src/dispatch.rs:27` |
| Repo-map default budget | `DEFAULT_REPOMAP_BUDGET` | 4096 | `src/defaults.rs:10` |
| Temperature | -- | 0.0 | `src/dispatch.rs:608` |
| Opus 4.7 input rate | -- | $5/MTok | `HANDOFF_TO_GPD.md` item 4 |
| Opus 4.7 output rate | -- | $25/MTok | `HANDOFF_TO_GPD.md` item 4 |

**Derived quantities:**

- **Effective max attempts:** Computed by EQ-003. Currently 2 for apply+verify,
  1 otherwise. Pending change to 1 for apply+verify.
- **Model selection:** `configured_model_for_level(level)` resolves env var >
  config file > default constant. Priority chain at `src/defaults.rs:75-90`.
  Pending addition of per-dispatch `model` override.

**Configuration precedence (model selection):**

```
env var (AWL_IMPLEMENTATION_MODEL) > config file (~/.config/awl/config.json) > compiled default
```

Implemented in `configured_string()` at `src/defaults.rs:114-124`.

## Phase Structure / Regimes

**Regime 1: Apply mode with verify command (primary savings path)**

Conditions: `apply = true`, `verify_command` is set, `target_path` resolved.
Flow: generate -> preflight -> snapshot -> write -> verify -> [pass: return ok |
fail: rollback, optionally retry]. This is the regime where Awl provides the
strongest value proposition: verified local work requires minimal frontier review.

- Key files: `src/dispatch.rs:698-949`

**Regime 2: Apply mode without verify command**

Conditions: `apply = true`, no `verify_command`.
Flow: generate -> preflight -> write -> return ok. No rollback on failure because
there is no failure signal. Risk: undetected bad writes require frontier cleanup.
Value proposition is weaker.

- Key file: `src/dispatch.rs:926-940`

**Regime 3: Non-apply mode (generation only)**

Conditions: `apply = false`.
Flow: generate -> return code in JSON. Frontier handles all writes. Value is
purely in generation offload. `files_changed` is empty; `files_intended` carries
model claims.

- Key file: `src/dispatch.rs:654-668`

**Regime 4: Verification / lint mode (L3)**

Conditions: `level = 3`, typically non-apply.
Uses the smallest model (3B). Intended for review/lint tasks where the frontier
wants a second opinion cheaply. Not currently exercised in the experiment harness.

**Phase transition (pending):**

The boundary between "same-model retry is useful" and "same-model retry is waste"
is the critical transition in this system. Currently all verify failures
are treated identically (retry). The proposed change (EQ-003 default to 1)
eliminates same-model retry on verify failure by default, shifting the boundary:
the frontier decides whether to re-dispatch at a higher tier or handle the task
directly.

## Failure Taxonomy

The dispatch reliability report identifies five distinct failure categories, each
with different retry economics:

| Category | Description | Retry Yield | Default Retry? | Location |
|----------|-------------|-------------|----------------|----------|
| `format` | JSON parse error in model output | High -- transient | Yes (EQ-008) | `src/dispatch.rs:993-1010` |
| `schema` | Valid JSON but missing required fields | High -- corrective prompt | Yes (EQ-008) | `src/dispatch.rs:979-992` |
| `preflight` | Hallucinated crate-internal import | High -- concrete feedback | Yes (within apply loop) | `src/dispatch.rs:800-840` |
| `verify` (capability gap) | Model lacks skill for the task | Low -- same blind spot | **No** (pending) | `src/dispatch.rs:851-920` |
| `verify` (fixable bug) | Off-by-one, edge case | Medium -- sometimes self-corrects | Maybe (opt-in) | Same |
| `timeout` | Verify command exceeds 120 s | N/A | No | `src/dispatch.rs:1196-1203` |
| `network` | Ollama unreachable | N/A | No | `src/dispatch.rs:1048-1055` |

The current code conflates the two `verify` subcategories. The proposed
`failure_category` telemetry field (`REPORT_DISPATCH_RELIABILITY.md`, "Other
robustness gaps", item 1) would allow the frontier to distinguish them
empirically over time.

## Cost Model

**The savings equation (conceptual):**

```
net_savings = baseline_cost - awl_assisted_cost

baseline_cost = (baseline_input_tokens / 1e6) * input_rate
              + (baseline_output_tokens / 1e6) * output_rate

awl_assisted_cost = (packaging_input_tokens / 1e6) * input_rate
                  + (review_output_tokens / 1e6) * output_rate
                  + 0  (local tokens are free)
```

**Key asymmetry:** The I/O cost ratio is 5x ($25/$5). Frontier output tokens
are 5x more expensive than input tokens. Awl's compact JSON result is
output-light on the frontier side (the frontier reads a short status message
rather than generating code). This means Awl saves disproportionately on the
expensive token type.

**Current implementation gap:** `tally.py` and `dispatch_cost_report.py` use a
single blended `--cost-per-mtok` rate. This conflates I/O and underestimates
the savings from reduced frontier output. Pending fix: item 4 in the patch
list (`HANDOFF_TO_GPD.md`).

**Break-even condition:** Awl saves tokens when:

```
frontier_packaging_cost + P(failure) * frontier_recovery_cost < baseline_direct_cost
```

Where `P(failure)` is the dispatch failure probability and
`frontier_recovery_cost` is the cost of the frontier handling a failed dispatch
(reading the compact error, deciding next action). The rollback invariant
(EQ-004) bounds recovery cost by ensuring the worktree is clean after failure.

**Empirical data point (from partial Step 1):**

| Task | Status | Attempts | Tokens | Wall (ms) |
|------|--------|----------|--------|-----------|
| `01_string_helper` | error | 2 (max) | 4264 | 29686 |
| `02_validate_input` | ok | 1 | 2073 | 15431 |
| `03_fix_off_by_one` | ok | 1 | 2264 | 17577 |

Task 01 demonstrates the anti-pattern: 2 attempts = ~4264 local tokens consumed
with zero value. ~2200 of those tokens were the wasted retry. With max_attempts=1,
the cost would have been ~2064 tokens (still wasted, but less so).

---

_Framework analysis: 2026-04-30_
