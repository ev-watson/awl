"""Correctness tests for merge_sort. Run with: python3 tests/test_merge_sort.py"""

import random
import sys
import os

# Allow running from project root or tests/ directory
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from sorting.merge_sort import merge_sort


def test_random_n1000():
    """Random array of 1000 integers matches sorted() oracle (seed=42)."""
    random.seed(42)
    arr = [random.randint(0, 10**9 - 1) for _ in range(1000)]
    result = merge_sort(arr.copy())
    assert result == sorted(arr), "Random N=1000 correctness failed"


def test_empty():
    """Empty array returns empty list."""
    assert merge_sort([]) == [], "Empty array failed"


def test_single():
    """Single-element array returns that element."""
    assert merge_sort([7]) == [7], "Single element failed"


def test_already_sorted():
    """Already-sorted array of 1000 elements is returned correctly sorted."""
    arr = list(range(1000))
    assert merge_sort(arr.copy()) == arr, "Already-sorted array failed"


def test_reverse_sorted():
    """Reverse-sorted array of 1000 elements is correctly sorted."""
    arr = list(range(999, -1, -1))
    assert merge_sort(arr.copy()) == sorted(arr), "Reverse-sorted array failed"


def test_all_equal():
    """Array of 1000 identical elements is returned unchanged."""
    arr = [42] * 1000
    assert merge_sort(arr.copy()) == arr, "All-equal array failed"


def test_no_mutation():
    """merge_sort does not mutate its input array."""
    arr = [3, 1, 4, 1, 5, 9, 2, 6]
    original = arr.copy()
    merge_sort(arr)
    assert arr == original, f"Input array was mutated: {arr} != {original}"


if __name__ == "__main__":
    tests = [
        test_random_n1000,
        test_empty,
        test_single,
        test_already_sorted,
        test_reverse_sorted,
        test_all_equal,
        test_no_mutation,
    ]

    for test in tests:
        test()
        print(f"PASSED: {test.__name__}")

    print("ALL MERGE SORT TESTS PASSED")
