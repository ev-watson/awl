# Requirements: Sorting Algorithm Benchmark

**Defined:** 2026-04-11
**Core Research Question:** Which of merge sort, quicksort, and radix sort is fastest on average across 10 trials on random integer arrays of size 10K, 100K, and 1M in pure Python?

## Primary Requirements

### Implementations

- [ ] **IMPL-01**: Implement merge sort in pure Python — takes a list of integers, returns a new sorted list; no use of numpy or stdlib sort
- [ ] **IMPL-02**: Implement quicksort in pure Python — takes a list of integers, returns a sorted list; handle N=1M without hitting Python recursion limits
- [ ] **IMPL-03**: Implement LSD radix sort in pure Python — takes a list of non-negative integers, returns a new sorted list; choose integer range and base in Phase 1

### Benchmarks

- [ ] **BENCH-01**: Generate random integer arrays for sizes N = 10,000; 100,000; 1,000,000 using a fixed random seed for reproducibility
- [ ] **BENCH-02**: Time each of 9 (algorithm, array size) combinations over 10 independent trials using `time.perf_counter`; regenerate a fresh array for each trial
- [ ] **BENCH-03**: Compute mean and standard deviation of wall-clock time in seconds for each (algorithm, size) combination
- [ ] **BENCH-04**: Save results to `results/benchmark_results.csv` with columns: algorithm, array_size, trial, time_s, and a summary table with mean_time_s and stddev_time_s

### Validations

- [ ] **VALD-01**: Verify each algorithm produces output identical to Python's `sorted()` on the same input — run on at least one trial per (algorithm, size) combination before timing begins
- [ ] **VALD-02**: Verify empirical complexity scaling: compute time ratio N=1M / N=10K for each algorithm; merge sort and quicksort must fall in [20, 500]; radix sort must fall in [20, 300]
- [ ] **VALD-03**: Verify timing table completeness: 9 rows (3 algorithms × 3 sizes), all mean values positive, all stddev values non-negative

### Analysis

- [ ] **ANAL-01**: Identify the winner: the algorithm with the lowest mean wall-clock time averaged across all three array sizes
- [ ] **ANAL-02**: Report whether the winner is statistically robust — compare mean times with ± stddev bands to assess overlap between competitors at each size

## Follow-up Requirements

### Extended Analysis

- **EXTD-01**: Vary integer range to test radix sort sensitivity to digit count (e.g., 0–9999 vs 0–10^9)
- **EXTD-02**: Compare iterative vs recursive quicksort at N=1M for stack-depth impact
- **EXTD-03**: Benchmark against Python's built-in `sorted()` and `list.sort()` as a reference ceiling

## Out of Scope

| Topic | Reason |
|-------|--------|
| numpy sort | Excludes pure Python comparison; separate study |
| Memory profiling | Requires different tooling; not part of timing study |
| Sorting stability | Not relevant to wall-clock benchmark |
| Worst-case inputs (sorted/reverse-sorted arrays) | Current scope is random integers only |
| Multi-threading | Changes runtime model; out of scope |

## Accuracy and Validation Criteria

| Requirement | Accuracy Target | Validation Method |
|-------------|----------------|-------------------|
| BENCH-02 | 10 independent trials per combination | Fresh array per trial, perf_counter timing |
| BENCH-03 | Mean and stddev to 6 significant figures | Python statistics module or manual computation |
| VALD-01 | Zero mismatches across all runs | Direct comparison with sorted() |
| VALD-02 | Scaling ratios within expected bands | Computed from mean times in results table |
| ANAL-01 | Lowest mean across all 3 sizes | Direct comparison of per-size means |

## Contract Coverage

| Requirement | Decisive Output / Deliverable | Anchor / Benchmark | Prior Inputs | False Progress To Reject |
|-------------|------------------------------|--------------------|--------------|--------------------------|
| BENCH-03 | results/benchmark_results.csv | O(n log n) / O(n) scaling bands | None | Single-trial result without mean/stddev |
| VALD-01 | Correctness log | sorted() as oracle | None | Timing results without correctness check |
| VALD-02 | Scaling ratio table | Theoretical complexity | None | Qualitative "radix should be faster" |
| ANAL-01 | Winner identification | Mean timing comparison | deliv-table | Theory-only winner claim |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| IMPL-01 | Phase 1: Implementation | Pending |
| IMPL-02 | Phase 1: Implementation | Pending |
| IMPL-03 | Phase 1: Implementation | Pending |
| BENCH-01 | Phase 2: Benchmarking | Pending |
| BENCH-02 | Phase 2: Benchmarking | Pending |
| BENCH-03 | Phase 2: Benchmarking | Pending |
| BENCH-04 | Phase 2: Benchmarking | Pending |
| VALD-01 | Phase 2: Benchmarking | Pending |
| VALD-02 | Phase 3: Analysis & Validation | Pending |
| VALD-03 | Phase 3: Analysis & Validation | Pending |
| ANAL-01 | Phase 3: Analysis & Validation | Pending |
| ANAL-02 | Phase 3: Analysis & Validation | Pending |

**Coverage:**

- Primary requirements: 12 total
- Mapped to phases: 12
- Unmapped: 0

---

_Requirements defined: 2026-04-11_
_Last updated: 2026-04-11 after initial definition_
