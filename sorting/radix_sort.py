"""LSD (Least Significant Digit) radix sort, base 256, 4 passes for integers
in [0, 10^9-1].  Uses counting sort with backward scan for stability.
O(4n) time, O(n) space.

ASSERT_CONVENTION: integer_range=[0, 10**9-1], radix_base=256, radix_passes=4,
lsd_scan_direction=backward, correctness_oracle=sorted()
"""

# Forbidden proxy: fp-forward-scan — the placement step MUST scan backward
# (right-to-left) to preserve the relative order of equal-byte elements from
# the previous pass.  Forward scan breaks LSD stability.


def radix_sort(arr: list) -> list:
    """Return a new sorted list containing the non-negative integers in *arr*.

    Uses LSD (Least Significant Digit) radix sort, base 256, 4 passes.
    The input array is not mutated.

    Correctness domain: non-negative integers in [0, 10**9 - 1].
    Caller is responsible for ensuring inputs satisfy this contract.

    Algorithm outline (per pass):
      A. Count occurrences of each byte value (0-255) in the current digit.
      B. Convert counts to exclusive-upper-bound prefix sums.
      C. Place elements into the output buffer scanning BACKWARD for stability.
      D. Swap work ↔ output so the next pass reads from the freshly sorted array.

    After 4 passes (bits 0-7, 8-15, 16-23, 24-31) the array is fully sorted.
    """
    if len(arr) <= 1:
        return list(arr)

    RADIX = 256

    work = list(arr)                    # working copy — caller's array is never touched
    output = [0] * len(work)            # single output buffer; reused across all 4 passes

    for pass_index in range(4):
        shift = 8 * pass_index          # 0, 8, 16, 24 — selects byte 0, 1, 2, 3

        # ------------------------------------------------------------------
        # Step A: count occurrences of each byte value in this digit position
        # ------------------------------------------------------------------
        count = [0] * RADIX
        for x in work:
            count[(x >> shift) & 0xFF] += 1

        # ------------------------------------------------------------------
        # Step B: prefix-sum → count[b] becomes the exclusive upper bound
        #         (i.e. the index AFTER the last slot for digit value b).
        # ------------------------------------------------------------------
        for i in range(1, RADIX):
            count[i] += count[i - 1]

        # ------------------------------------------------------------------
        # Step C: place elements into output, scanning BACKWARD for stability.
        #
        # We decrement count[b] before writing, so the first element we place
        # for digit b fills slot count[b]-1 (the last slot for that digit).
        # Because we scan right-to-left, elements with the same byte value
        # land in reverse encounter order — meaning the left-most equal
        # element in work[] ends up left-most in output[], preserving the
        # relative ordering established by earlier passes.
        # ------------------------------------------------------------------
        for i in range(len(work) - 1, -1, -1):
            b = (work[i] >> shift) & 0xFF
            count[b] -= 1
            output[count[b]] = work[i]

        # ------------------------------------------------------------------
        # Step D: copy sorted output back into work for the next pass.
        #         Using slice assignment avoids allocating a new list object.
        # ------------------------------------------------------------------
        work[:] = output

    return work
