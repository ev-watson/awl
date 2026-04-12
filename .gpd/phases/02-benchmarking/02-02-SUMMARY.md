---
plan: 02-02
phase: 02-benchmarking
status: complete
completed: 2026-04-11
commit: 299b95f
---

# Summary: Plan 02-02 — Full Benchmark Run

## What Was Done

Ran `python3 benchmark.py` — 90 trials (3 algorithms × 3 sizes × 10 trials each).
Zero correctness failures. All timing values positive and physically plausible.

## Results

| Algorithm | N=10,000 (mean±σ) | N=100,000 (mean±σ) | N=1,000,000 (mean±σ) |
|---|---|---|---|
| merge_sort | 0.0145±0.0018s | 0.1559±0.0024s | 1.9675±0.0219s |
| quicksort | 0.0064±0.0001s | 0.0792±0.0027s | 1.0102±0.0149s |
| radix_sort | 0.0049±0.0002s | 0.0518±0.0043s | **0.6284±0.0133s** |

**radix_sort is fastest at every size.**

## CSV Validation

- Raw rows: 90 (3 algorithms × 3 sizes × 10 trials)
- Summary rows: 9 (one per algorithm-size combination)
- All timing values > 0 ✓
- Correctness: all 90 trials passed `result == sorted(arr)` ✓

## Phase 2 Status

**Phase 2: COMPLETE.** results/benchmark_results.csv written with full data.
Phase 3 (Analysis & Validation) is unblocked.
