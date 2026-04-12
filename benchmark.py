import csv
import random
import statistics
import time
import os

import config
from sorting.merge_sort import merge_sort
from sorting.quicksort import quicksort
from sorting.radix_sort import radix_sort

SORT_FUNCTIONS = {
    "merge_sort": merge_sort,
    "quicksort": quicksort,
    "radix_sort": radix_sort,
}


def generate_array(size: int, seed: int) -> list:
    """Generate a random integer array of given size using given seed."""
    rng = random.Random(seed)
    return [rng.randint(config.INT_MIN, config.INT_MAX) for _ in range(size)]


def time_sort(sort_fn, arr: list) -> tuple:
    """Time a sort function call. Returns (sorted_result, elapsed_seconds)."""
    t0 = time.perf_counter()
    result = sort_fn(arr)
    t1 = time.perf_counter()
    return result, t1 - t0


def run_benchmarks() -> list:
    """Run all 30 trials (3 algorithms x 3 sizes x 10 trials).

    Returns list of dicts with keys: algorithm, array_size, trial, time_s
    """
    raw_rows = []

    for algo_name in config.ALGORITHM_IDS:
        sort_fn = SORT_FUNCTIONS[algo_name]
        for size_idx, size in enumerate(config.ARRAY_SIZES):
            for trial in range(config.TIMING_TRIALS):
                seed = trial * 1000 + size_idx * 100
                arr = generate_array(size, seed)

                print(f"Running {algo_name} N={size:>9,} trial {trial+1:>2}/{config.TIMING_TRIALS}...",
                      end="", flush=True)

                result, elapsed = time_sort(sort_fn, arr.copy())

                # Correctness verification
                expected = sorted(arr)
                assert result == expected, (
                    f"CORRECTNESS FAILURE: {algo_name} N={size} trial {trial+1}: "
                    f"result does not match sorted(arr)"
                )

                print(f" {elapsed:.6f}s")

                raw_rows.append({
                    "algorithm": algo_name,
                    "array_size": size,
                    "trial": trial + 1,
                    "time_s": elapsed,
                })

    return raw_rows


def compute_summary(raw_rows: list) -> list:
    """Compute mean and stddev per (algorithm, array_size) combination."""
    groups = {}
    for row in raw_rows:
        key = (row["algorithm"], row["array_size"])
        groups.setdefault(key, []).append(row["time_s"])

    summary_rows = []
    for algo_name in config.ALGORITHM_IDS:
        for size in config.ARRAY_SIZES:
            key = (algo_name, size)
            times = groups[key]
            mean = statistics.mean(times)
            stddev = statistics.stdev(times) if len(times) > 1 else 0.0
            summary_rows.append({
                "algorithm": algo_name,
                "array_size": size,
                "mean_time_s": mean,
                "stddev_time_s": stddev,
            })

    return summary_rows


def save_results(raw_rows: list, summary_rows: list, path: str = "results/benchmark_results.csv") -> None:
    """Write raw trials and summary to CSV."""
    os.makedirs(os.path.dirname(path), exist_ok=True)

    with open(path, "w", newline="") as f:
        writer = csv.writer(f)

        # Raw section
        writer.writerow(["algorithm", "array_size", "trial", "time_s"])
        for row in raw_rows:
            writer.writerow([row["algorithm"], row["array_size"], row["trial"], f"{row['time_s']:.9f}"])

        # Blank separator
        writer.writerow([])

        # Summary section
        writer.writerow(["algorithm", "array_size", "mean_time_s", "stddev_time_s"])
        for row in summary_rows:
            writer.writerow([
                row["algorithm"],
                row["array_size"],
                f"{row['mean_time_s']:.9f}",
                f"{row['stddev_time_s']:.9f}",
            ])


def print_timing_table(summary_rows: list) -> None:
    """Print formatted timing table to stdout."""
    print("\n" + "=" * 70)
    print("BENCHMARK RESULTS (mean ± stddev, wall-clock seconds)")
    print("=" * 70)
    print(f"{'Algorithm':<15} {'N=10,000':>12} {'N=100,000':>12} {'N=1,000,000':>14}")
    print("-" * 70)

    for algo_name in config.ALGORITHM_IDS:
        row_parts = [f"{algo_name:<15}"]
        for size in config.ARRAY_SIZES:
            match = next(r for r in summary_rows
                         if r["algorithm"] == algo_name and r["array_size"] == size)
            cell = f"{match['mean_time_s']:.4f}±{match['stddev_time_s']:.4f}"
            row_parts.append(f"{cell:>14}")
        print("".join(row_parts))

    print("=" * 70)


def main():
    print("=" * 70)
    print("Sorting Algorithm Benchmark")
    print(f"Algorithms: {', '.join(config.ALGORITHM_IDS)}")
    print(f"Array sizes: {', '.join(str(s) for s in config.ARRAY_SIZES)}")
    print(f"Trials per combination: {config.TIMING_TRIALS}")
    print("=" * 70 + "\n")

    raw_rows = run_benchmarks()
    summary_rows = compute_summary(raw_rows)
    save_results(raw_rows, summary_rows)
    print_timing_table(summary_rows)

    print(f"\nResults saved to results/benchmark_results.csv")
    print(f"Total trials: {len(raw_rows)} | All correctness checks passed.\n")


if __name__ == "__main__":
    main()
