---
phase: 02-benchmarking
plan: 01
status: completed
plan_contract_ref: 02-01-PLAN.md
---

# 02-01 SUMMARY: Benchmark Harness Implementation

## What Was Implemented

`benchmark.py` is the full timing harness for the sorting algorithm benchmark project. It generates reproducible random arrays, times each sort call with `time.perf_counter()`, verifies correctness against Python's built-in `sorted()` on every trial, and writes raw trial data plus per-combination summary statistics to a CSV file.

## Function Signatures

| Function | Signature | Description |
|---|---|---|
| `generate_array` | `(size: int, seed: int) -> list` | Generates a reproducible random integer array using `random.Random(seed)`. Values in `[config.INT_MIN, config.INT_MAX]`. |
| `time_sort` | `(sort_fn, arr: list) -> tuple` | Times one sort call via `time.perf_counter()`. Returns `(sorted_result, elapsed_seconds)`. |
| `run_benchmarks` | `() -> list` | Nested loop over `ALGORITHM_IDS x ARRAY_SIZES x TIMING_TRIALS` (3 x 3 x 10 = 30 trials). Seed per trial: `seed = trial * 1000 + size_idx * 100`. Asserts `result == sorted(arr)` on each trial. Returns list of raw row dicts. |
| `compute_summary` | `(raw_rows: list) -> list` | Groups raw rows by `(algorithm, array_size)`, computes `statistics.mean` and `statistics.stdev` per group. Returns list of summary row dicts. |
| `save_results` | `(raw_rows: list, summary_rows: list, path: str = "results/benchmark_results.csv") -> None` | Writes CSV with raw section then blank row then summary section. Creates the `results/` directory if absent. |
| `print_timing_table` | `(summary_rows: list) -> None` | Prints a 70-character-wide formatted table of `mean±stddev` per algorithm x size. |
| `main` | `() -> None` | Orchestrates: `run_benchmarks()` -> `compute_summary()` -> `save_results()` -> `print_timing_table()`. |

## CSV Format

File path: `results/benchmark_results.csv`

**Raw section** (first block):
```
algorithm,array_size,trial,time_s
merge_sort,10000,1,0.012345678
...
```

**Blank separator row** (one empty row between sections).

**Summary section** (second block):
```
algorithm,array_size,mean_time_s,stddev_time_s
merge_sort,10000,0.012345678,0.000123456
...
```

Timing values are formatted to 9 decimal places (nanosecond resolution from `time.perf_counter()`).

## Algorithm Mapping

| Config ID | Import |
|---|---|
| `merge_sort` | `sorting.merge_sort.merge_sort` |
| `quicksort` | `sorting.quicksort.quicksort` |
| `radix_sort` | `sorting.radix_sort.radix_sort` |

## Reproducibility

- Random arrays: `random.Random(seed)` with `seed = trial * 1000 + size_idx * 100`. Each of the 30 trials gets a distinct, deterministic seed.
- Timing clock: `time.perf_counter()` (highest-resolution wall-clock available on the platform).
- Correctness gate: every trial asserts `result == sorted(arr)` before recording; any failure raises `AssertionError` with algorithm, size, and trial number.

## Contract Results

```yaml
contract_results:
  claims:
    - id: claim-harness-correct
      outcome: satisfied
      evidence: >
        run_benchmarks() iterates over all 30 combinations (3 x 3 x 10),
        verifies correctness via assert result == sorted(arr) on each trial,
        and records time_s from time.perf_counter() which is always positive.
  deliverables:
    - id: deliv-benchmark
      status: produced
      path: benchmark.py
```

## Verification

- Syntax check: `python3 -c "import py_compile; py_compile.compile('benchmark.py', doraise=True); print('syntax OK')"` -- PASSED

## Commit

- `0c04a5a253aaea6499ae6d565c26251a0c11c3a1` -- `feat(02-01): benchmark harness — timing loop, correctness check, CSV output`

## Self-Check: PASSED

- benchmark.py created at `/Users/blu3/claw/benchmark.py`
- Syntax verification passed
- Commit exists: `0c04a5a`
- All 30 trials (3 algorithms x 3 sizes x 10 trials) covered by loop structure
- Correctness assertion present on every trial
- CSV written with raw + summary sections separated by blank row
- No full benchmark run performed (Wave 2 task)
