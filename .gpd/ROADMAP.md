# Roadmap: Sorting Algorithm Benchmark

## Overview

This project implements and benchmarks three pure Python sorting algorithms — merge sort, quicksort, and radix sort — on random integer arrays of size 10K, 100K, and 1M. Each combination is timed over 10 independent trials. The benchmark produces a timing table (mean ± stddev wall-clock seconds) with built-in correctness verification and complexity-scaling sanity checks, ultimately identifying which algorithm is fastest on average.

## Phases

- [ ] **Phase 1: Implementation** — Implement all three sorting algorithms in pure Python with design decisions resolved
- [ ] **Phase 2: Benchmarking** — Generate test arrays, run 10-trial timing harness, save results to CSV
- [ ] **Phase 3: Analysis & Validation** — Verify correctness and scaling, identify winner, report findings

## Phase Details

### Phase 1: Implementation

**Goal:** Implement correct, standalone pure Python versions of merge sort, quicksort, and radix sort. Resolve key design decisions (integer range, quicksort variant, radix base) so Phase 2 can run without revisiting implementation choices.

**Depends on:** Nothing (first phase)

**Requirements:** IMPL-01, IMPL-02, IMPL-03

**Success Criteria** (what must be TRUE):

1. All three algorithms pass a basic correctness check against `sorted()` on a small sample array (N=1000, random integers)
2. Quicksort handles N=1,000,000 without Python RecursionError (use iterative or sys.setrecursionlimit strategy)
3. Integer range and radix sort base are recorded in a `config.py` or equivalent so Phase 2 uses consistent inputs
4. Each implementation is a self-contained function (no external dependencies)

**Plans:** 3 plans (2 waves)

Plans:

- [ ] 01-01-PLAN.md -- config.py + merge sort implementation + tests (Wave 1)
- [ ] 01-02-PLAN.md -- quicksort implementation + tests including N=1M stress test (Wave 1, parallel)
- [ ] 01-03-PLAN.md -- LSD radix sort + comprehensive three-algorithm correctness gate (Wave 2)

---

### Phase 2: Benchmarking

**Goal:** Run the full 10-trial timing harness for all 9 (algorithm, array size) combinations. Verify correctness on each trial before accepting timing data. Save raw and summary results to CSV.

**Depends on:** Phase 1 (all three implementations complete and passing correctness checks)

**Requirements:** BENCH-01, BENCH-02, BENCH-03, BENCH-04, VALD-01

**Success Criteria** (what must be TRUE):

1. All 30 trial runs (3 algorithms × 3 sizes × 10 trials) complete without error
2. Zero correctness failures: each trial output matches `sorted()` on the same input
3. `results/benchmark_results.csv` exists with 9 summary rows (algorithm, array_size, mean_time_s, stddev_time_s) plus 30 raw-trial rows
4. Timing values are positive and physically plausible (no sub-microsecond or multi-hour results)

Plans:

- [ ] 02-01: Implement timing harness — array generation (fixed seed), trial loop, perf_counter timing, correctness check
- [ ] 02-02: Run benchmarks for N=10K (all 3 algorithms, 10 trials each); spot-check results
- [ ] 02-03: Run benchmarks for N=100K and N=1M; monitor for RecursionError or excessive runtime
- [ ] 02-04: Aggregate results, compute mean/stddev, write benchmark_results.csv

---

### Phase 3: Analysis & Validation

**Goal:** Verify complexity scaling, identify the winner, assess statistical robustness, and produce the final summary report.

**Depends on:** Phase 2 (benchmark_results.csv complete)

**Requirements:** VALD-02, VALD-03, ANAL-01, ANAL-02

**Success Criteria** (what must be TRUE):

1. Scaling ratios (N=1M / N=10K) computed and verified within expected bands for all three algorithms
2. Winner identified as the algorithm with lowest mean time averaged across all three sizes
3. Overlap analysis (mean ± stddev bands) shows whether the winner is clearly separated from competitors
4. Final summary printed to stdout and/or saved as `results/summary.md`

Plans:

- [ ] 03-01: Compute and check scaling ratios; flag any violations
- [ ] 03-02: Rank algorithms by average mean time across sizes; identify winner
- [ ] 03-03: Overlap analysis — compare ± stddev bands between algorithms at each size
- [ ] 03-04: Write summary report (`results/summary.md`) with winner, timing table, scaling check results

---

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|---------------|--------|-----------|
| 1. Implementation | 0/4 | Not started | — |
| 2. Benchmarking | 0/4 | Not started | — |
| 3. Analysis & Validation | 0/4 | Not started | — |
