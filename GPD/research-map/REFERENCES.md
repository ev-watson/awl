# Reference and Anchor Map

**Analysis Date:** 2026-04-30

> **Adaptation note:** Awl is a software engineering project. "References" here
> means the project's internal research artifacts, architectural decisions,
> experiment definitions, and external dependencies that constrain design
> choices. There is no academic bibliography.

## Active Anchor Registry

| Anchor ID | Anchor | Type | Source / Locator | Why It Matters | Must Surface | Required Action | Carry Forward To |
|-----------|--------|------|------------------|----------------|--------------|-----------------|------------------|
| ANC-001 | Product hypothesis | definition | `HANDOFF_TO_GPD.md` line 11 | Defines the project's reason for existing: "bounded local execution measurably saves frontier tokens on net" | yes | test | verification |
| ANC-002 | Success thresholds | benchmark | `experiments/README.md` "Pass/fail thresholds"; `UPDATED_PROGRESS_REPORT.md` "Initial success criteria" | >= 25-40% token reduction at >= 60-70% Awl pass rate. These are the gates for declaring Step 1 done. | yes | compare | verification |
| ANC-003 | Dispatch v2 contract | definition | `src/dispatch.rs:258-277` (SYSTEM_PROMPT); `src/dispatch.rs:672-689` (`dispatch_response_format`) | The JSON schema `{status, code, explanation, files_modified}` is the API surface. Changing it is a breaking change requiring user authorization. | yes | use | execution |
| ANC-004 | Rollback invariant | definition | `src/dispatch.rs:1126-1147` (`capture_snapshot`, `restore_snapshot`); tests at lines 1467-1490 | Snapshot/write/verify/rollback is the mechanism that bounds failure cost. Breaking it destroys the savings thesis. | yes | avoid breaking | execution |
| ANC-005 | Cost rates (Opus 4.7) | benchmark | `HANDOFF_TO_GPD.md` item 4; `REPORT_DISPATCH_RELIABILITY.md` resolved decisions | $5/MTok input, $25/MTok output. The 5x asymmetry is load-bearing for the cost model. | yes | use | planning |
| ANC-006 | Step 1 partial results | prior artifact | `REPORT_DISPATCH_RELIABILITY.md` "Step 1 experiment state" | 3 tasks run, 2 passed, 1 failed deterministically. Wasted retry observed. This is the empirical basis for the retry policy change. | yes | compare | verification |
| ANC-007 | Retry policy decision | definition | `REPORT_DISPATCH_RELIABILITY.md` "Question 2" + "Resolved decisions" | User-confirmed: default max_attempts for apply+verify drops from 2 to 1. No auto-escalation. | yes | use | execution |
| ANC-008 | Model override decision | definition | `REPORT_DISPATCH_RELIABILITY.md` "Question 1" + "Resolved decisions" | User-confirmed: frontier picks model per-dispatch via new `model` field. Keep L2 default at 7B. | yes | use | execution |
| ANC-009 | Patch list (5 items) | method | `REPORT_DISPATCH_RELIABILITY.md` "Proposed architectural changes" + `HANDOFF_TO_GPD.md` "What to do first" | The work queue. Items 1-4 must ship before Step 1 resumes. Each is independently shippable. | yes | use | planning |
| ANC-010 | Branch protection rules | definition | `HANDOFF_TO_GPD.md` "Hard constraints" | Never push to main directly. PRs required. `enforce_admins: true`. Two CI checks required. | yes | avoid breaking | execution |
| ANC-011 | Tokenizer inflation | benchmark | `HANDOFF_TO_GPD.md` item 4 | Opus 4.7 tokenizer produces ~35% more tokens for same text than prior Claude tokenizers. Real-world spend may rise even at same rate card. | no | read | planning |
| ANC-012 | Pre-v2 failure modes | prior artifact | `UPDATED_PROGRESS_REPORT.md` "Reference Baseline" | Catalogs the failures that dispatch v2 was designed to fix: malformed JSON, hallucinated files_modified, no verification, no rollback. Needed context for understanding why the current contract exists. | no | read | planning |

## Benchmarks and Comparison Targets

**Primary benchmark: Step 1 A/B savings experiment**

- Definition: `experiments/README.md`
- Awl arm driver: `experiments/run_awl_arm.sh`
- Tally script: `experiments/tally.py`
- Baseline format: `experiments/results/baseline.csv` (manual frontier-only data)
- Status: 3/10+ tasks run on Awl arm only. No baseline data. Halted pending
  architectural work.

**Success thresholds (ANC-002):**

| Metric | Threshold | Source |
|--------|-----------|--------|
| Aggregate token reduction | >= 25-40% | `experiments/README.md`, `UPDATED_PROGRESS_REPORT.md` |
| Awl pass rate (usable-as-is) | >= 60-70% | Same |

These thresholds should be re-justified per model configuration (7B-only vs
14B-only) after the Step 1 sweep completes.

**Empirical reference point (ANC-006):**

| Task | Pass? | Tokens | Wall (ms) | Notes |
|------|-------|--------|-----------|-------|
| `01_string_helper` | No | 4264 | 29686 | Capability gap: trailing newline handling |
| `02_validate_input` | Yes | 2073 | 15431 | Clean first-attempt success |
| `03_fix_off_by_one` | Yes | 2264 | 17577 | Clean first-attempt success |

Pass rate: 2/3 = 67% (within target range, but n=3 is too small to be
meaningful). Mean tokens on passing tasks: 2168. No baseline data for
comparison.

## Prior Artifacts and Baselines

- `REPORT_DISPATCH_RELIABILITY.md`: The most recent and most authoritative
  research artifact. Contains the failure analysis from Step 1, the retry
  policy and model-override decisions, the failure taxonomy proposal, the
  cost-reporting gap analysis, and the concrete 5-item patch list. Later
  phases must treat this as the primary source for architectural decisions.

- `UPDATED_PROGRESS_REPORT.md`: Historical narrative documenting progress
  against the original corrective plan (`reportreport.txt`). Contains the
  fuller context for why dispatch v2 exists, what it replaced, and the
  reasoning behind the experiment design. Not authoritative for current
  decisions (superseded by the reliability report) but provides essential
  context for understanding the system.

- `HANDOFF_TO_GPD.md`: Project map and work queue, written specifically for
  GPD ingestion. Contains hard constraints, file map, and suggested GPD
  initialization. Authoritative for constraints and the ordered patch list.

- `experiments/results/awl_arm.jsonl` (gitignored, local only): Raw JSONL
  from the partial Step 1 run. 3 records. Present on the originating machine.

- `~/.config/awl/dispatches/1777586666491028000-32287.jsonl` (local only):
  Per-dispatch telemetry for the failed `01_string_helper` dispatch. Contains
  the full model request/response cycle including both attempts. Used in the
  reliability report analysis.

## Open Reference Questions

1. **No frontier-baseline data exists.** The manual baseline arm of the A/B
   experiment has never been run. Without it, token savings cannot be
   calculated. This is the single most important missing data point.

2. **14B model performance is unmeasured.** The 14B model has never been run
   through the experiment harness. The hypothesis that it catches capability
   gaps that 7B misses is plausible (by reputation) but unverified.

3. **Tokenizer inflation not empirically quantified for Awl dispatches.**
   The ~35% inflation figure (ANC-011) is a general claim. The actual
   inflation for Awl's typical dispatch payloads (JSON, short code snippets)
   may differ. This affects cost projections.

4. **Blended vs split cost rates.** The current `tally.py` uses a single
   `--cost-per-mtok` and does not distinguish input from output. The 5x I/O
   asymmetry (ANC-005) means blended rates systematically misestimate
   savings. The split-rate implementation (patch item 4) is needed before
   cost claims are meaningful.

5. **Task pack is undersized.** 3 tasks is insufficient for statistical
   confidence. The target is >= 10 mixed tasks covering write-from-scratch,
   edit-existing, and context-paths-required; in both Python and Rust.
   Currently all 3 tasks are Python write-from-scratch.

## Background Reading

- **Ollama OpenAI-compatible API:** Awl uses the `/v1/chat/completions`
  endpoint with `response_format: json_schema`. The `strict: true` flag
  relies on Ollama's structured output support. Model compatibility varies.
  - Relevant to: `src/dispatch.rs:596-615`, `src/defaults.rs:27-30`

- **Qwen2.5-Coder model family:** Three tiers are used (3B, 7B, 14B). The
  7B-q4 quantization (4-bit) trades quality for speed/memory. The 14B uses
  full precision. Empirical quality gap observed on edge-case handling.
  - Relevant to: `src/defaults.rs:3-5`, `REPORT_DISPATCH_RELIABILITY.md`
    Question 1 tradeoff table

- **MCP (Model Context Protocol):** Awl serves as a stdio MCP server. The
  frontier interacts via tool calls (`awl_dispatch`, `awl_repomap`,
  `awl_health`, etc.). The MCP transport is synchronous/blocking.
  - Relevant to: `src/mcp_server.rs`, `.claude/skills/awl-dispatch/SKILL.md`

- **Claude Code / Codex integration model:** The frontier assistant is the
  orchestrator. Awl is a worker. The skill file
  (`.claude/skills/awl-dispatch/SKILL.md`) defines when to dispatch vs when
  to handle directly. This is the "demand side" of the dispatch equation.

---

_Reference map: 2026-04-30_
