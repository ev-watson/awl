import random
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from sorting.radix_sort import radix_sort


def test_random_n1000():
    random.seed(42)
    arr = [random.randint(0, 10**9 - 1) for _ in range(1000)]
    assert radix_sort(arr.copy()) == sorted(arr)

def test_empty():
    assert radix_sort([]) == []

def test_single():
    assert radix_sort([7]) == [7]

def test_already_sorted():
    arr = list(range(1000))
    assert radix_sort(arr.copy()) == arr

def test_all_equal():
    arr = [42] * 1000
    assert radix_sort(arr.copy()) == arr

def test_no_mutation():
    arr = [3, 1, 4, 1, 5, 9, 2, 6]
    orig = arr.copy()
    radix_sort(arr)
    assert arr == orig

def test_range_boundary():
    arr = [10**9 - 1, 0, 10**9 - 2, 1, 500_000_000]
    assert radix_sort(arr.copy()) == sorted(arr)


if __name__ == "__main__":
    tests = [
        test_random_n1000,
        test_empty,
        test_single,
        test_already_sorted,
        test_all_equal,
        test_no_mutation,
        test_range_boundary,
    ]
    for t in tests:
        t()
        print(f"PASSED: {t.__name__}")
    print("ALL RADIX SORT TESTS PASSED")
