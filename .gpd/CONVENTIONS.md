# Conventions: Sorting Algorithm Benchmark

Established: 2026-04-11

## Timing

- **Clock**: `time.perf_counter()` — wall-clock time with highest available resolution
- **Unit**: seconds (float), reported to 6 significant figures
- **Trial isolation**: Fresh random array generated for each trial; no re-use across trials or algorithms
- **Measurement scope**: Time the sort function call only; array generation is outside the timed block

## Array Generation

- **Type**: Python list of integers
- **Seed**: Fixed random seed (to be chosen in Phase 1) for reproducibility; different seed per trial to sample random variation
- **Range**: TBD in Phase 1 (expected: 0 to 10^9 inclusive)

## Algorithm Conventions

- **merge_sort(arr)**: Returns a new sorted list; does not modify input
- **quicksort(arr)**: Returns a sorted list; implementation variant chosen in Phase 1
- **radix_sort(arr)**: Returns a new sorted list; LSD (least-significant-digit) variant; assumes non-negative integers; base chosen in Phase 1

## Result Reporting

- **CSV columns**: `algorithm`, `array_size`, `trial`, `time_s` (raw); `algorithm`, `array_size`, `mean_time_s`, `stddev_time_s` (summary)
- **Algorithm identifiers**: `merge_sort`, `quicksort`, `radix_sort` (snake_case, exact)
- **Array sizes**: Integers `10000`, `100000`, `1000000`

## Scaling Analysis

- **Scaling ratio**: mean_time(N=1,000,000) / mean_time(N=10,000)
- **Expected bands**: merge_sort ∈ [20, 500], quicksort ∈ [20, 500], radix_sort ∈ [20, 300]

## Custom Conventions

See `.gpd/state.json` convention_lock for machine-readable entries.
