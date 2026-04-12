"""LSD (Least Significant Digit) radix sort, base 256, 4 passes for integers in [0, 10^9-1].
Uses counting sort with backward scan for stability. O(4n) time, O(n) space.
"""


def radix_sort(arr: list) -> list:
    """Sort a list of non-negative integers using LSD radix sort (base 256).

    Args:
        arr: List of non-negative integers in [0, 10**9 - 1]. Does not mutate input.

    Returns:
        A new sorted list.
    """
    if len(arr) <= 1:
        return list(arr)

    RADIX = 256
    work = list(arr)
    output = [0] * len(work)

    for pass_index in range(4):
        shift = 8 * pass_index  # 0, 8, 16, 24

        # Step A: count occurrences of each byte value
        count = [0] * RADIX
        for x in work:
            count[(x >> shift) & 0xFF] += 1

        # Step B: convert counts to prefix sums (exclusive upper bound indices)
        for i in range(1, RADIX):
            count[i] += count[i - 1]

        # Step C: place elements into output, iterating BACKWARD for stability
        for i in range(len(work) - 1, -1, -1):
            b = (work[i] >> shift) & 0xFF
            count[b] -= 1
            output[count[b]] = work[i]

        # Step D: copy output back to work for next pass
        work[:] = output

    return work
