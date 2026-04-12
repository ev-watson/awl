"""
Test suite for sorting.quicksort — iterative quicksort with median-of-three
pivot, Lomuto partition, and push-smaller-first stack discipline.

Run directly: python3 tests/test_quicksort.py
Pass condition: all 8 tests pass, no RecursionError, final line
"ALL QUICKSORT TESTS PASSED".
"""

import random
import sys
import time

# Allow running from project root with: python3 tests/test_quicksort.py
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from sorting.quicksort import quicksort


# ---------------------------------------------------------------------------
# Correctness oracle convention: all outputs compared against sorted().
# Integer range: [0, 10**9 - 1] per project conventions in config.py.
# ---------------------------------------------------------------------------


def test_random_n1000():
    """N=1000 random integers (seed 42) must equal sorted() output."""
    rng = random.Random(42)
    arr = [rng.randint(0, 10**9 - 1) for _ in range(1000)]
    result = quicksort(arr.copy())
    assert result == sorted(arr), (
        f"test_random_n1000 FAILED: first mismatch at index "
        f"{next(i for i,(a,b) in enumerate(zip(result,sorted(arr))) if a!=b)}"
    )


def test_empty():
    """Empty array returns empty list."""
    assert quicksort([]) == []


def test_single():
    """Single-element array returns that element."""
    assert quicksort([7]) == [7]


def test_already_sorted():
    """Already-sorted array of 1000 elements (potential worst-case for naive pivot)."""
    arr = list(range(1000))
    result = quicksort(arr.copy())
    assert result == arr, "test_already_sorted FAILED"


def test_reverse_sorted():
    """Reverse-sorted array of 1000 elements (another potential worst-case)."""
    arr = list(range(999, -1, -1))
    result = quicksort(arr.copy())
    assert result == sorted(arr), "test_reverse_sorted FAILED"


def test_all_equal():
    """Array of 1000 identical elements."""
    arr = [42] * 1000
    result = quicksort(arr.copy())
    assert result == arr, "test_all_equal FAILED"


def test_no_mutation():
    """Input array must not be modified by quicksort."""
    arr = [3, 1, 4, 1, 5, 9, 2, 6]
    orig = arr.copy()
    quicksort(arr)
    assert arr == orig, (
        f"test_no_mutation FAILED: arr={arr}, expected={orig}"
    )


def test_n1m_no_recursion():
    """N=1,000,000 random integers (seed 99) — decisive stress test.

    Must complete without RecursionError, MemoryError, or any exception.
    Result must equal sorted(arr).  Elapsed time is printed for informational
    purposes; any positive value is acceptable in Phase 1.
    """
    rng = random.Random(99)
    arr = [rng.randint(0, 10**9 - 1) for _ in range(1_000_000)]

    t0 = time.perf_counter()
    result = quicksort(arr.copy())
    elapsed = time.perf_counter() - t0

    assert result == sorted(arr), "test_n1m_no_recursion FAILED: result != sorted(arr)"
    print(f"N=1M quicksort: {elapsed:.2f}s — PASSED")


# ---------------------------------------------------------------------------
# Test runner
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    tests = [
        ("test_random_n1000",    test_random_n1000),
        ("test_empty",           test_empty),
        ("test_single",          test_single),
        ("test_already_sorted",  test_already_sorted),
        ("test_reverse_sorted",  test_reverse_sorted),
        ("test_all_equal",       test_all_equal),
        ("test_no_mutation",     test_no_mutation),
        ("test_n1m_no_recursion",test_n1m_no_recursion),
    ]

    for name, fn in tests:
        fn()
        print(f"PASSED: {name}")

    print("ALL QUICKSORT TESTS PASSED")
