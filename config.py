# config.py — Written in Phase 1, imported by Phase 2 benchmark harness.
# DO NOT modify after Phase 1 is complete without re-running Phase 1 correctness checks.

# Integer range for random test arrays
INT_MIN = 0
INT_MAX = 10**9 - 1          # 999_999_999 — fits in 4 bytes (< 2^30 < 2^32)

# Radix sort parameters (LSD base-256, 4 passes)
RADIX_BASE = 256             # byte-at-a-time; count array of 256 entries fits in L1 cache
RADIX_PASSES = 4             # ceil(log_256(2^32)) = 4; covers all integers in INT range

# Quicksort variant identifier (for documentation/logging)
QUICKSORT_VARIANT = "iterative_median3_lomuto"

# Algorithm identifiers used in CSV output
ALGORITHM_IDS = ["merge_sort", "quicksort", "radix_sort"]

# Benchmark array sizes
ARRAY_SIZES = [10_000, 100_000, 1_000_000]

# Number of timing trials per (algorithm, array size) combination
TIMING_TRIALS = 10

# Timing clock function name (actual import: import time; time.perf_counter())
TIMING_CLOCK = "time.perf_counter"
