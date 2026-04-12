"""Iterative bottom-up merge sort with pre-allocated scratch buffer. O(n log n) time, O(n) space."""


def _merge(arr, scratch, lo, mid, hi):
    """Merge arr[lo:mid] and arr[mid:hi] using scratch buffer, writing result back to arr[lo:hi].

    Uses index arithmetic exclusively — no list slicing. The scratch buffer is
    pre-allocated by the caller and reused across all merge calls.
    """
    i = lo    # left partition pointer
    j = mid   # right partition pointer
    k = lo    # write pointer into scratch

    while i < mid and j < hi:
        if arr[i] <= arr[j]:
            scratch[k] = arr[i]
            i += 1
        else:
            scratch[k] = arr[j]
            j += 1
        k += 1

    # Drain remaining elements from the left partition
    while i < mid:
        scratch[k] = arr[i]
        i += 1
        k += 1

    # Drain remaining elements from the right partition
    while j < hi:
        scratch[k] = arr[j]
        j += 1
        k += 1

    # Copy merged result from scratch back into arr
    for idx in range(lo, hi):
        arr[idx] = scratch[idx]


def merge_sort(arr: list) -> list:
    """Return a new sorted list containing the same elements as arr.

    The input array is not mutated. Implements iterative bottom-up merge sort
    with a single pre-allocated scratch buffer reused across all merge calls.
    No list slicing is used; all index arithmetic is explicit.
    """
    if len(arr) <= 1:
        return list(arr)  # return a copy, not the original

    work = list(arr)             # working copy — original arr is never touched
    scratch = [0] * len(work)   # scratch buffer allocated once, reused for all merges

    width = 1
    n = len(work)
    while width < n:
        lo = 0
        while lo < n:
            mid = min(lo + width, n)
            hi = min(lo + 2 * width, n)
            if mid < hi:  # only merge if the right partition is non-empty
                _merge(work, scratch, lo, mid, hi)
            lo += 2 * width
        width *= 2

    return work
