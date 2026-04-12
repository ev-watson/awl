# Sorting Algorithm Benchmark: Merge Sort, Quicksort, Radix Sort

## What This Is

A computational benchmarking study comparing three pure Python sorting algorithm implementations — merge sort, quicksort, and radix sort — on random integer arrays of sizes 10K, 100K, and 1M. Each (algorithm, array size) combination is timed over 10 trials, and the fastest algorithm on average is identified. The deliverable is a timing table (mean ± stddev wall-clock seconds) with correctness and complexity-scaling verification.

## Core Research Question

Which of merge sort, quicksort, and radix sort is fastest on average across 10 trials on random integer arrays of size 10K, 100K, and 1M in pure Python?

## Scoping Contract Summary

### Contract Coverage

- **Claim**: One algorithm achieves the lowest mean wall-clock time averaged across all three array sizes, determined from 10 trials per combination with correctness verified
- **Acceptance signal**: Timing table with 9 rows (3 algorithms × 3 sizes), all correctness checks pass, complexity scaling ratios within expected bands
- **False progress to reject**: Claiming a winner from theoretical O() complexity alone without measured data; single-trial results reported as definitive

### User Guidance To Preserve

- **User-stated observables**: Mean ± stddev wall-clock time for each (algorithm, array size) combination; empirical complexity scaling ratio N=1M/N=10K
- **User-stated deliverables**: `results/benchmark_results.csv` — timing table with mean_time_s and stddev_time_s columns
- **Must-have references / prior outputs**: None confirmed yet — baseline established from scratch
- **Stop / rethink conditions**: Any algorithm produces unsorted output; timing ratio N=1M/N=10K for merge/quick falls outside [20, 500]; radix sort more than 5× slower than merge sort at N=1M

### Scope Boundaries

**In scope**

- Pure Python implementations of merge sort, quicksort, and radix sort
- Random integer arrays of size 10K, 100K, and 1M
- 10 timing trials per (algorithm, array size) combination
- Wall-clock timing using `time.perf_counter`
- Correctness verification against Python's `sorted()`

**Out of scope**

- numpy or stdlib sort implementations
- Non-integer or real-valued data
- In-place variants, stability analysis, or memory profiling
- Multi-threaded or parallel implementations

### Active Anchor Registry

- **complexity-scaling**: Theoretical O(n log n) for merge/quick, O(n) for radix
  - Why it matters: Gross violations of expected scaling ratios signal buggy implementations
  - Carry forward: execution | verification
  - Required action: compare

### Carry-Forward Inputs

- None confirmed yet — timing baseline established from scratch

### Skeptical Review

- **Weakest anchor**: Timing results may not reproduce across machines with different Python versions, CPU characteristics, or OS scheduling
- **Unvalidated assumptions**: Integer range for random arrays not yet chosen (affects radix sort digit count); 10 trials assumed sufficient for reliable mean estimation
- **Competing explanation**: Quicksort may outperform radix sort in pure Python despite O(n) vs O(n log n) due to lower per-element overhead and better cache behavior
- **Disconfirming observation**: If merge/quick scaling ratio falls outside [20, 500], or radix sort is >5× slower than merge sort at N=1M
- **False progress to reject**: Qualitative "radix sort should be faster" reasoning without measured wall-clock data

### Open Contract Questions

- Integer range for random arrays not yet specified — decide in Phase 1 (suggestion: 0 to 10^9, using LSD radix sort base 10)
- No external benchmark selected — timing is self-anchored

## Research Questions

### Answered

(None yet)

### Active

- [ ] Which algorithm is fastest on average across 10K, 100K, and 1M element arrays in pure Python?
- [ ] Do all three implementations scale as theoretically expected?
- [ ] What is the variance in timing across 10 trials — is 10 trials sufficient?

### Out of Scope

- Memory usage comparison — requires different tooling; out of current scope
- Sorting stability verification — not relevant to timing benchmark

## Research Context

### Physical System

Three sorting algorithms benchmarked on random integer arrays in pure Python:
- **Merge sort**: Divide-and-conquer, O(n log n), stable
- **Quicksort**: In-place divide-and-conquer (or recursive), O(n log n) average
- **Radix sort**: Non-comparison LSD sort, O(d·n) where d = number of digits

### Theoretical Framework

Algorithm analysis — worst/average-case complexity, cache behavior, Python interpreter overhead.

### Key Parameters and Scales

| Parameter | Symbol | Regime | Notes |
|-----------|--------|--------|-------|
| Array size | N | 10K, 100K, 1M | Three orders of magnitude |
| Trials | T | 10 | Per (algorithm, size) combination |
| Integer range | R | TBD | Affects radix sort digit count |
| Timing resolution | — | ~1 µs | time.perf_counter on modern hardware |

### Known Results

- Merge sort: O(n log n) worst-case; stable; O(n) extra memory
- Quicksort: O(n log n) average, O(n²) worst-case; in-place variants possible
- Radix sort: O(d·n) where d = digits; fast for bounded integers but high constant in Python

### What Is New

Empirical comparison of pure Python implementations at three scales with statistical rigor (mean ± stddev over 10 trials) and complexity-scaling verification as a built-in sanity check.

### Target Venue

Internal benchmark report / research exercise.

### Computational Environment

Pure Python, local workstation. No external dependencies required.

## Notation and Conventions

See `.gpd/CONVENTIONS.md` for all notation and conventions.

## Unit System

Wall-clock time in seconds (float), array sizes as integers.

## Requirements

See `.gpd/REQUIREMENTS.md` for the detailed requirements specification.

Key requirement categories: IMPL (implementation), BENCH (benchmark), VALD (validation), ANAL (analysis)

## Key References

No external benchmark references — timing baseline established from scratch.

## Constraints

- **Computational**: Pure Python only — no numpy, no Cython, no C extensions in sort implementations
- **Statistical**: Minimum 10 trials per (algorithm, size) combination required; mean + stddev must be reported
- **Correctness**: All implementations must be verified against `sorted()` before timing results are accepted

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Integer range for arrays | Affects radix sort digit count and performance | — Pending (Phase 1) |
| Quicksort variant (recursive vs iterative) | Impacts Python stack depth at N=1M | — Pending (Phase 1) |
| Radix sort base (base-10 vs base-256) | Affects number of passes and constant factors | — Pending (Phase 1) |

Full log: `.gpd/DECISIONS.md`

---

_Last updated: 2026-04-11 after initialization_
