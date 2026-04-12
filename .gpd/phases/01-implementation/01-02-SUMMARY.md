---
phase: 01-implementation
plan: 02
depth: full
one-liner: "Implemented iterative quicksort (median-of-three pivot, Lomuto partition, push-smaller-first stack) — N=1M sorts correctly in 0.94s without RecursionError"
subsystem:
  - implementation
  - validation
tags:
  - quicksort
  - iterative
  - median-of-three
  - Lomuto-partition
  - push-smaller-first
  - stress-test

requires:
  - phase: 01-implementation
    provides: "config.py with INT_MIN=0, INT_MAX=10^9-1, QUICKSORT_VARIANT=iterative_median3_lomuto"

provides:
  - "sorting/quicksort.py: iterative quicksort, no recursion, O(log n) stack depth"
  - "tests/test_quicksort.py: 8-test suite including N=1M stress test"
  - "N=1M correctness verified: result == sorted(arr) with seed 99 in 0.94s"

affects:
  - 01-03 (radix sort — same test structure)
  - 02-benchmark (Phase 2 timing harness consumes sorting/quicksort.py)

methods:
  added:
    - "Iterative quicksort with explicit Python list stack"
    - "Median-of-three pivot: _median3 sorts arr[lo], arr[mid], arr[hi] in-place, places median at arr[hi]"
    - "Lomuto partition: _partition with pivot at arr[hi], returns pivot final index"
    - "Push-smaller-first discipline: larger partition pushed first (stack bottom), smaller second (stack top)"
  patterns:
    - "Two-element sub-array fast path: single conditional swap, skip median3+partition overhead"
    - "Work on list(arr) copy at entry to preserve non-mutation contract"

key-files:
  created:
    - sorting/quicksort.py
    - tests/test_quicksort.py

key-decisions:
  - "Placed median at arr[hi] (not arr[hi-1]) — Lomuto pivot is arr[hi]; after 3-sort swap arr[mid]<->arr[hi] achieves this"
  - "Two-element fast path added: hi - lo == 1 handled by conditional swap, avoiding median3/partition call on trivial subarrays"
  - "Push-smaller-first: left_size > right_size pushes (lo,p-1) first then (p+1,hi); else reverses — bounds stack to O(log n)"

patterns-established:
  - "Test file pattern: 8 named test functions + __main__ runner printing PASSED: {name} + ALL {ALGO} TESTS PASSED"
  - "N=1M stress test uses time.perf_counter and prints elapsed for informational logging"
  - "sys.path.insert(0, project_root) in test files for direct python3 tests/test_X.py execution"

conventions:
  - "No external dependencies (pure Python)"
  - "Correctness oracle: sorted()"
  - "Integer range: [0, 10^9 - 1]"
  - "Timing clock: time.perf_counter()"
  - "No recursion, no sys.setrecursionlimit"

plan_contract_ref: ".gpd/phases/01-implementation/01-02-PLAN.md#/contract"

contract_results:
  claims:
    claim-quicksort-correct:
      status: passed
      summary: "quicksort(arr) returns a new sorted list identical to sorted(arr) for all 7 correctness tests (random N=1000, empty, single, already-sorted, reverse-sorted, all-equal, no-mutation). Input array confirmed unchanged."
      linked_ids: [deliv-quicksort, deliv-quicksort-tests, test-qs-random, test-qs-edge-cases, test-qs-no-mutation]
      evidence:
        - verifier: executor-self-check
          method: sorted() oracle comparison
          confidence: high
          claim_id: claim-quicksort-correct
          deliverable_id: deliv-quicksort
          acceptance_test_id: test-qs-random
          evidence_path: "tests/test_quicksort.py"
    claim-quicksort-no-recursion-error:
      status: passed
      summary: "quicksort on N=1,000,000 random integers (seed 99) completed in 0.94s without RecursionError, MemoryError, or any exception. Result == sorted(arr) confirmed. Explicit stack; no Python call-stack recursion."
      linked_ids: [deliv-quicksort, test-qs-n1m-stress]
      evidence:
        - verifier: executor-self-check
          method: direct execution + sorted() oracle comparison
          confidence: high
          claim_id: claim-quicksort-no-recursion-error
          deliverable_id: deliv-quicksort
          acceptance_test_id: test-qs-n1m-stress
          evidence_path: "tests/test_quicksort.py"
  deliverables:
    deliv-quicksort:
      status: passed
      path: sorting/quicksort.py
      summary: "Iterative quicksort with explicit stack, median-of-three pivot, Lomuto partition, push-smaller-first. Contains: def quicksort, def _median3, def _partition; uses stack.append/stack.pop; no recursion."
      linked_ids: [claim-quicksort-correct, claim-quicksort-no-recursion-error]
    deliv-quicksort-tests:
      status: passed
      path: tests/test_quicksort.py
      summary: "8-test suite covering all required cases. All tests pass. N=1M stress test prints elapsed time."
      linked_ids: [claim-quicksort-correct, claim-quicksort-no-recursion-error]
  acceptance_tests:
    test-qs-random:
      status: passed
      summary: "seed=42, N=1000, quicksort(arr.copy()) == sorted(arr): PASSED"
      linked_ids: [claim-quicksort-correct, deliv-quicksort]
    test-qs-edge-cases:
      status: passed
      summary: "All 5 edge cases pass: [], [7], range(1000), range(999,-1,-1), [42]*1000 — each result == sorted(input)"
      linked_ids: [claim-quicksort-correct, deliv-quicksort]
    test-qs-no-mutation:
      status: passed
      summary: "arr==[3,1,4,1,5,9,2,6] unchanged after quicksort(arr) call: PASSED"
      linked_ids: [claim-quicksort-correct, deliv-quicksort]
    test-qs-n1m-stress:
      status: passed
      summary: "seed=99, N=1M: completed in 0.94s, result == sorted(arr), no exception of any kind: PASSED"
      linked_ids: [claim-quicksort-no-recursion-error, deliv-quicksort]
  references: {}
  forbidden_proxies:
    fp-recursive-quicksort:
      status: rejected
      notes: "No recursion in sorting/quicksort.py — confirmed by AST import check (zero imports) and grep showing def quicksort appears exactly once with no recursive body call"
    fp-no-push-smaller-first:
      status: rejected
      notes: "Push-smaller-first discipline implemented: left_size vs right_size comparison controls push order, bounding stack to O(log n)"
    fp-theory-only:
      status: rejected
      notes: "All claims backed by direct sorted() oracle comparison runs, not theory alone"
  uncertainty_markers:
    weakest_anchors:
      - "N=1M stress test uses random input only (seed 99); adversarial sorted-array input at N=1M not tested separately (median-of-three handles it per test_already_sorted at N=1000)"
    unvalidated_assumptions:
      - "0.94s timing on test machine; actual Phase 2 timing will differ per hardware"
    competing_explanations: []
    disconfirming_observations: []

duration: 8min
completed: 2026-04-11
---

# Phase 01, Plan 02: Quicksort Implementation Summary

**Implemented iterative quicksort (median-of-three pivot, Lomuto partition, push-smaller-first stack) — N=1M sorts correctly in 0.94s without RecursionError**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-04-11T22:55Z (approx)
- **Completed:** 2026-04-11
- **Tasks:** 2/2
- **Files modified:** 2 created

## Key Results

- N=1M quicksort (seed 99): **0.94s**, result == sorted(arr) — PASSED (decisive gating test for Phase 2)
- All 8 correctness tests pass including 5 edge cases, non-mutation, and N=1M stress test
- Explicit stack maximum depth at N=1M is O(log2(1M)) ≈ 20 entries — push-smaller-first discipline verified by design

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement iterative quicksort** - `0a38d95` (implement)
2. **Task 2: Write quicksort test suite including N=1M stress test** - `95e2ac0` (validate)

## Files Created/Modified

- `sorting/quicksort.py` - Iterative quicksort: `_median3`, `_partition`, `quicksort`
- `tests/test_quicksort.py` - 8-test suite, direct-run with `python3 tests/test_quicksort.py`

## Next Phase Readiness

- `sorting/quicksort.py` ready for Phase 2 timing harness import
- N=1M correctness gate passed — quicksort can proceed to Phase 2 benchmarking
- Same test file pattern established for radix sort (plan 01-03)

## Contract Coverage

- Claim IDs advanced: claim-quicksort-correct → passed, claim-quicksort-no-recursion-error → passed
- Deliverable IDs produced: deliv-quicksort → sorting/quicksort.py (passed), deliv-quicksort-tests → tests/test_quicksort.py (passed)
- Acceptance test IDs run: test-qs-random → passed, test-qs-edge-cases → passed, test-qs-no-mutation → passed, test-qs-n1m-stress → passed
- Reference IDs surfaced: none declared in contract
- Forbidden proxies rejected: fp-recursive-quicksort (rejected), fp-no-push-smaller-first (rejected), fp-theory-only (rejected)
- Decisive comparison verdicts: N/A (no cross-method or benchmark comparison required)

## Validations Completed

- `quicksort([3,1,4,1,5,9,2,6]) == [1,1,2,3,4,5,6,9]`: PASSED (basic correctness)
- `_median3([5,1,3], 0, 2)` returns pivot=3 at arr[hi]: PASSED (median placement)
- All 5 edge cases ([], [7], ascending, descending, all-equal) == sorted(): PASSED
- Non-mutation: arr unchanged after quicksort(arr): PASSED
- N=1M stress test (seed 99): no RecursionError, result == sorted(arr), elapsed 0.94s: PASSED
- AST import check: zero imports in sorting/quicksort.py (no sys, no setrecursionlimit): PASSED
- Plan-level grep: `def quicksort` appears exactly once, no recursive call inside body: PASSED

## Decisions Made

- Median placed at `arr[hi]` via swap `arr[mid], arr[hi] = arr[hi], arr[mid]` after 3-element sort — this is the standard Lomuto pivot placement
- Two-element fast path (`hi - lo == 1`): single conditional swap avoids median3+partition overhead for trivial subarrays
- Push-smaller-first logic: `if left_size > right_size: push(lo, p-1) then push(p+1, hi)` else reversed — explicit size comparison on each partition

## Deviations from Plan

None — plan executed exactly as specified.

## Issues Encountered

None.

## Open Questions

- Adversarial N=1M sorted-array input: `test_already_sorted` at N=1000 passes (median-of-three handles sorted input), but explicit N=1M sorted-array test was not run. Phase 2 stress tests may add this.

---

_Phase: 01-implementation_
_Completed: 2026-04-11_
