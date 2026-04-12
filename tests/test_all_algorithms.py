import random
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from sorting.merge_sort import merge_sort
from sorting.quicksort import quicksort
from sorting.radix_sort import radix_sort
import config


def test_agreement_random_n1000():
    random.seed(42)
    arr = [random.randint(config.INT_MIN, config.INT_MAX) for _ in range(1000)]
    expected = sorted(arr)
    assert merge_sort(arr.copy()) == expected, "merge_sort disagrees on random N=1000"
    assert quicksort(arr.copy()) == expected, "quicksort disagrees on random N=1000"
    assert radix_sort(arr.copy()) == expected, "radix_sort disagrees on random N=1000"

def test_agreement_empty():
    expected = sorted([])
    assert merge_sort([]) == expected
    assert quicksort([]) == expected
    assert radix_sort([]) == expected

def test_agreement_single():
    arr = [999_999_999]
    expected = sorted(arr)
    assert merge_sort(arr.copy()) == expected
    assert quicksort(arr.copy()) == expected
    assert radix_sort(arr.copy()) == expected

def test_agreement_sorted():
    arr = list(range(500))
    expected = sorted(arr)
    assert merge_sort(arr.copy()) == expected
    assert quicksort(arr.copy()) == expected
    assert radix_sort(arr.copy()) == expected

def test_agreement_reverse():
    arr = list(range(499, -1, -1))
    expected = sorted(arr)
    assert merge_sort(arr.copy()) == expected
    assert quicksort(arr.copy()) == expected
    assert radix_sort(arr.copy()) == expected

def test_agreement_all_equal():
    arr = [42] * 500
    expected = sorted(arr)
    assert merge_sort(arr.copy()) == expected
    assert quicksort(arr.copy()) == expected
    assert radix_sort(arr.copy()) == expected

def test_no_algorithm_mutates():
    random.seed(7)
    arr = [random.randint(0, 10**9 - 1) for _ in range(200)]
    original = arr.copy()
    merge_sort(arr)
    assert arr == original, "merge_sort mutated input"
    quicksort(arr)
    assert arr == original, "quicksort mutated input"
    radix_sort(arr)
    assert arr == original, "radix_sort mutated input"


if __name__ == "__main__":
    tests = [
        test_agreement_random_n1000,
        test_agreement_empty,
        test_agreement_single,
        test_agreement_sorted,
        test_agreement_reverse,
        test_agreement_all_equal,
        test_no_algorithm_mutates,
    ]
    for t in tests:
        t()
        print(f"PASSED: {t.__name__}")
    print("=" * 50)
    print("ALL PHASE 1 CORRECTNESS TESTS PASSED")
    print("All three algorithms agree with sorted() on 6 test arrays.")
    print("Phase 2 benchmarking is unblocked.")
    print("=" * 50)
