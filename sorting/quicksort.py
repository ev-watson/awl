"""
Iterative quicksort with explicit stack, median-of-three pivot selection,
Lomuto partition scheme, and push-smaller-first stack discipline.
O(n log n) average time, O(log n) stack space.

Forbidden proxies (per contract):
  - No recursion (no sys.setrecursionlimit workaround)
  - Push-smaller-first discipline enforced (O(log n) stack depth)
  - Correctness verified against sorted() oracle
"""


def _median3(arr: list, lo: int, hi: int) -> int:
    """Sort arr[lo], arr[mid], arr[hi] in-place and place the median at arr[hi].

    After this call arr[hi] holds the median of the three candidates, which
    Lomuto partition will use as the pivot.  Returns the pivot value.
    """
    mid = (lo + hi) // 2
    # Bring the three candidates into sorted order among themselves.
    if arr[lo] > arr[mid]:
        arr[lo], arr[mid] = arr[mid], arr[lo]
    if arr[lo] > arr[hi]:
        arr[lo], arr[hi] = arr[hi], arr[lo]
    if arr[mid] > arr[hi]:
        arr[mid], arr[hi] = arr[hi], arr[mid]
    # Now arr[lo] <= arr[mid] <= arr[hi].
    # arr[mid] is the median.  Swap it to arr[hi] so Lomuto can treat arr[hi]
    # as the pivot.  After the swap: arr[hi] = median (pivot), arr[mid] = old max.
    arr[mid], arr[hi] = arr[hi], arr[mid]
    return arr[hi]


def _partition(arr: list, lo: int, hi: int) -> int:
    """Lomuto partition on arr[lo..hi] with pivot at arr[hi].

    Rearranges arr[lo..hi] so that every element left of the returned index p
    is <= arr[p], and every element right of p is > arr[p].  Returns p (the
    pivot's final position).
    """
    pivot = arr[hi]
    i = lo - 1
    for j in range(lo, hi):
        if arr[j] <= pivot:
            i += 1
            arr[i], arr[j] = arr[j], arr[i]
    # Place pivot at its final sorted position.
    arr[i + 1], arr[hi] = arr[hi], arr[i + 1]
    return i + 1


def quicksort(arr: list) -> list:
    """Return a new sorted list containing the elements of *arr*.

    The input array is not mutated.  Uses an explicit Python list as the
    recursion stack (no Python call-stack recursion) so it is safe for
    arbitrarily large N without RecursionError.

    Stack discipline: the larger sub-partition is pushed first (processed
    later) and the smaller sub-partition is pushed second (processed next).
    This bounds the stack to O(log n) entries in the worst case.
    """
    if len(arr) <= 1:
        return list(arr)

    work = list(arr)  # do not mutate the caller's array
    stack = [(0, len(work) - 1)]

    while stack:
        lo, hi = stack.pop()

        if hi - lo <= 0:
            continue

        # Two-element sub-array: a single conditional swap is faster than a
        # full partition + median-of-three.
        if hi - lo == 1:
            if work[lo] > work[hi]:
                work[lo], work[hi] = work[hi], work[lo]
            continue

        # Select pivot using median-of-three; pivot ends up at work[hi].
        _median3(work, lo, hi)

        # Partition and locate the pivot's final sorted index.
        p = _partition(work, lo, hi)

        left_size = p - 1 - lo
        right_size = hi - (p + 1)

        # Push-smaller-first: push the LARGER partition first (it goes deeper
        # in the stack and is processed later), push the SMALLER partition
        # second (it sits on top and is processed next).  This keeps the
        # maximum live stack depth at O(log n).
        if left_size > right_size:
            stack.append((lo, p - 1))      # larger — pushed first (processed later)
            stack.append((p + 1, hi))      # smaller — pushed second (processed next)
        else:
            stack.append((p + 1, hi))      # larger — pushed first (processed later)
            stack.append((lo, p - 1))      # smaller — pushed second (processed next)

    return work
