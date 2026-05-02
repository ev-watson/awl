---
phase: 01-reliability-and-measurement-patches
plan: 02
depth: standard
provides:
  - split-input-output-cost-accounting
  - reporting-cost-fixtures
completed: "2026-05-02T05:49:49Z"
status: completed
completed_at: "2026-05-02T05:49:49Z"
plan_contract_ref: GPD/phases/01-reliability-and-measurement-patches/01-02-PLAN.md#/contract
key_files:
  created:
    - experiments/test_tally.py
    - scripts/test_dispatch_cost_report.py
  modified:
    - experiments/tally.py
    - scripts/dispatch_cost_report.py
contract_results:
  claims:
    claim-split-cost-tally:
      status: passed
      summary: "tally.py uses split input/output cost flags and fixture tests reproduce the pilot arithmetic."
      linked_ids: [deliv-tally-patch, deliv-tally-tests, test-tally-split-cost, test-tally-backward-compat, ref-reliability-report]
    claim-split-cost-report:
      status: passed
      summary: "dispatch_cost_report.py uses split frontier input/output cost flags and keeps the old blended flag as a deprecated conflict-checked alias."
      linked_ids: [deliv-cost-report-patch, deliv-cost-report-tests, test-cost-report-split, ref-reliability-report]
  deliverables:
    deliv-tally-patch:
      status: passed
      path: experiments/tally.py
      summary: "Report output shows input/output/total Awl tokens and computes Awl cost from split rates."
      linked_ids: [claim-split-cost-tally, test-tally-split-cost]
    deliv-cost-report-patch:
      status: passed
      path: scripts/dispatch_cost_report.py
      summary: "Cost report computes avoided cost from split direct frontier input/output token estimates."
      linked_ids: [claim-split-cost-report, test-cost-report-split]
    deliv-tally-tests:
      status: passed
      path: experiments/test_tally.py
      summary: "Unittest fixtures cover pilot costs, blended undercount, no-cost mode, total-token readability, and deprecated flag behavior."
      linked_ids: [claim-split-cost-tally, test-tally-split-cost, test-tally-backward-compat]
    deliv-cost-report-tests:
      status: passed
      path: scripts/test_dispatch_cost_report.py
      summary: "Unittest fixtures cover split cost math, deprecated flag conflict, and failure category aggregation used by Plan 03."
      linked_ids: [claim-split-cost-report, test-cost-report-split]
  acceptance_tests:
    test-tally-split-cost:
      status: passed
      summary: "python3 -m unittest experiments.test_tally reports pilot task costs $0.024960, $0.013205, $0.014340 and total $0.052505."
      linked_ids: [claim-split-cost-tally, deliv-tally-patch, deliv-tally-tests, ref-reliability-report]
    test-tally-backward-compat:
      status: passed
      summary: "--cost-per-mtok warns when used alone and errors when combined with split flags."
      linked_ids: [claim-split-cost-tally, deliv-tally-patch]
    test-cost-report-split:
      status: passed
      summary: "python3 -m unittest scripts.test_dispatch_cost_report verifies split avoided cost from known input/output token counts."
      linked_ids: [claim-split-cost-report, deliv-cost-report-patch, deliv-cost-report-tests]
  references:
    ref-reliability-report:
      status: completed
      completed_actions: [read, use]
      missing_actions: []
      summary: "Used for the $5/$25 split-rate requirement and blended-cost rejection."
    ref-experiment-readme:
      status: completed
      completed_actions: [read, use]
      missing_actions: []
      summary: "Used to preserve total-token readability while noting baseline split-token data is still needed."
  forbidden_proxies:
    fp-blended-cost:
      status: rejected
      notes: "Primary cost computation now uses prompt/completion token splits."
    fp-no-tests:
      status: rejected
      notes: "Both reporting scripts have fixture tests with known arithmetic."
  uncertainty_markers:
    weakest_anchors:
      - "baseline.csv still lacks split frontier input/output token columns; Phase 2 must collect them for precise savings cost comparison."
    unvalidated_assumptions:
      - "Phase 2 frontier baseline collection can expose split input/output tokens in a comparable schema."
    competing_explanations:
      - "Split Awl-side cost does not by itself establish paid-token savings without direct frontier baseline data."
    disconfirming_observations:
      - "Fixture test cost diverges from the hand-calculated pilot total."
comparison_verdicts:
  - subject_id: test-tally-split-cost
    subject_kind: acceptance_test
    subject_role: decisive
    reference_id: ref-reliability-report
    comparison_kind: benchmark
    metric: pilot_split_cost_total
    threshold: "== 0.052505"
    verdict: pass
    recommended_action: "Carry split-rate reporting into Phase 2 baseline schema updates."
  - subject_id: test-cost-report-split
    subject_kind: acceptance_test
    subject_role: decisive
    reference_id: ref-reliability-report
    comparison_kind: benchmark
    metric: dispatch_report_split_cost
    threshold: "known fixture cost == 0.010000"
    verdict: pass
    recommended_action: "Use split direct-frontier input/output estimates in dispatch cost reports."
---
# Plan 01-02 Summary

Split input/output token cost accounting is implemented for both reporting scripts. The focused Python fixture suite passed 12 tests across `experiments.test_tally` and `scripts.test_dispatch_cost_report`.

The remaining limitation is intentional and carried forward: existing baseline CSV examples only contain total frontier tokens, so precise direct-frontier cost comparison still requires Phase 2 split-token baseline collection.

```yaml
gpd_return:
  status: completed
  files_written:
    - "experiments/tally.py"
    - "experiments/test_tally.py"
    - "scripts/dispatch_cost_report.py"
    - "scripts/test_dispatch_cost_report.py"
    - "GPD/phases/01-reliability-and-measurement-patches/01-02-SUMMARY.md"
  issues:
    - "Baseline split input/output token schema remains Phase 2 work."
  next_actions:
    - "$gpd-execute-phase 01"
  phase: "01"
  plan: "01-02"
  tasks_completed: 3
  tasks_total: 3
  conventions_used:
    units: "provider_reported_tokens_split_input_output"
    cost_units: "usd_per_million_frontier_tokens_split_c_in_c_out"
    result_schema: "snake_case_json_csv_fields"
```
