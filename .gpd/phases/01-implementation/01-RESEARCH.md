# Phase 1 Research: Implementation

**Researched:** 2026-04-11
**Domain:** Algorithm implementation / pure Python sorting
**Confidence:** HIGH

---

## Summary

This phase implements three well-understood sorting algorithms in pure Python. The algorithms themselves are textbook material; the challenge is navigating Python-specific constraints — the 1000-frame recursion limit, list-slicing allocation overhead, and the high per-operation interpreter cost that can make O(n) radix sort slower than O(n log n) merge/quicksort in practice.

The three pending design decisions from PROJECT.md (integer range, quicksort variant, radix base) are all resolvable with high confidence. The recommended decisions are: random integers in range [0, 10^9 - 1] (fits in 4 bytes, enabling 4-pass base-256 radix sort), iterative quicksort with median-of-three pivot and push-smaller-first stack discipline (guaranteed O(log n) stack depth), and LSD radix sort in base 256 with 4 passes (fewest passes for the chosen integer range).

The goal of Phase 1 is correctness and resolved parameters — not micro-optimization. Every implementation should be simple, readable, and verifiable against `sorted()`. Phase 2 will time them; Phase 1 just needs them right.

**Primary recommendation:** Iterative bottom-up merge sort, iterative quicksort with median-of-three pivot and push-smaller-first stack, LSD radix sort in base 256 with 4 passes over range [0, 10^9 - 1].

---

## Active Anchor References

| Anchor / Artifact | Type | Why It Matters Here | Required Action | Where It Must Reappear |
|---|---|---|---|---|
| `sorted()` correctness oracle | Correctness oracle (project contract) | All three implementations must produce output identical to sorted() | Verify at N=1000 on random integers | Correctness check task; Phase 2 pre-benchmark gating |
| complexity-scaling O(n log n) / O(n) | Theoretical anchor (project contract) | Gross violations signal buggy implementations; timing ratio N=1M/N=10K must fall in [20, 500] for merge/quick | Verify ratio after Phase 2 timing | Phase 2 verification step |
| `time.perf_counter` timing convention | Project convention | Phase 2 timings must use this clock | Record in config.py so Phase 2 imports it | config.py; Phase 2 benchmark harness |
| `config.py` | Phase 1 decisive output | Records integer range, radix base, algorithm IDs for Phase 2 to consume consistently | Write in Phase 1 | Phase 2 benchmark harness imports it |

**Missing or weak anchors:** No external benchmark dataset or reference timing exists — timings are self-anchored. This is expected per PROJECT.md ("baseline established from scratch").

---

## Conventions

| Choice | Convention | Alternatives | Source |
|---|---|---|---|
| Integer range | [0, 10^9 - 1] (inclusive) | [0, 9999], [0, 10^6] | Research recommendation (see Design Decisions below) |
| Radix sort base | 256 (base-2^8, byte-at-a-time) | 10, 16, 65536 | Research recommendation |
| Radix sort passes | 4 (since 10^9 < 2^30 < 2^32) | 10 (base-10), 2 (base-65536) | Follows from base-256 + integer range |
| Quicksort variant | Iterative with explicit stack | Recursive with setrecursionlimit | Research recommendation |
| Pivot strategy | Median-of-three (first, mid, last) | Random pivot, first element | Research recommendation |
| Algorithm IDs | merge_sort, quicksort, radix_sort | — | Project contract |
| Timing clock | time.perf_counter() | time.time(), time.process_time() | Project contract |
| Time unit | seconds (float) | milliseconds | Project contract |
| Correctness oracle | sorted() | manual verification | Project contract |

---

## Design Decision Recommendations

These three decisions are flagged as pending in PROJECT.md. Recommendations below are final — record them in `config.py`.

### Decision 1: Integer Range — Recommend [0, 10^9 - 1]

**Rationale:**
- 10^9 - 1 < 2^30 < 2^32, so every integer fits in exactly 4 bytes
- This gives radix sort exactly 4 passes with base 256, which is the minimum for this range (clean, predictable)
- Large enough that random arrays at N=1M have negligible collision rate (mean ~1000 collisions out of 10^9 values)
- Matches the PROJECT.md open-question suggestion ("suggestion: 0 to 10^9")
- Base-10 alternative would require ceil(log10(10^9)) = 9 passes — 2.25× more work

**Config value:** `INT_RANGE = (0, 10**9 - 1)`

**Why not 0–9999:** Too few distinct values; at N=1M the array is mostly duplicates, which pathologically favors radix sort and defeats the purpose of a fair benchmark.
**Why not 0–10^6:** Only 3 bytes needed (base-256), reducing radix advantage at large N; also many duplicates at N=1M.

### Decision 2: Quicksort Variant — Recommend Iterative with Explicit Stack + Median-of-Three Pivot + Push-Smaller-First

**Rationale:**
- Python default recursion limit is 1000 frames. A naively recursive quicksort on N=1M with a bad pivot will hit this after ~1000 levels — RecursionError is certain on pathological inputs and possible on random inputs with unlucky pivots.
- `sys.setrecursionlimit` to 1,500,000 is technically possible but: (a) stack overflow risk depends on platform and Python version, (b) CPython 3.12+ decoupled the C-stack limit from the Python limit, making behavior version-dependent and unreliable.
- Iterative quicksort with an explicit Python list as stack has no recursion depth limit — safe at all N.
- **Push-smaller-first discipline:** When both sub-partitions are pushed onto the stack, always push the larger one first and the smaller one second (so smaller is processed first). This bounds the stack to O(log n) entries in the worst case (Sedgewick, CLRS 7-4).
- **Median-of-three pivot:** Pick pivot as median of arr[lo], arr[mid], arr[hi]. For random arrays this is nearly equivalent to random pivot but avoids sorted/reverse-sorted worst cases. For this benchmark with uniformly random integers, first-element pivot would work fine, but median-of-three is the professional standard with negligible overhead.
- **Lomuto vs Hoare:** Hoare's scheme does ~3× fewer swaps on average and handles equal elements well. Lomuto is simpler to implement correctly but degrades on all-equal arrays. Since the integer range is large (few duplicates at N=1M), either works. Recommend Lomuto for implementation simplicity, since correctness is the priority.

**Config value:** `QUICKSORT_VARIANT = "iterative_median3_lomuto"`

**Fallback if iterative implementation proves buggy:** Use recursive with `sys.setrecursionlimit(1500000)` + median-of-three pivot. This works reliably on random arrays (expected depth ~2 log2(N) ≈ 40 for N=1M) but is fragile on adversarial inputs and unreliable in CPython 3.12+.

### Decision 3: Radix Sort Base — Recommend Base 256 (4 Passes)

**Rationale:**
- For integers in [0, 10^9 - 1] ⊂ [0, 2^30):
  - Base 10: ceil(log10(10^9)) = 9 passes, count array size 10
  - Base 256: ceil(log_{256}(2^32)) = 4 passes, count array size 256
  - Base 65536: 2 passes, count array size 65536 (exceeds L1 cache, unlikely to help in Python)
- 4 passes × O(n + 256) is substantially fewer Python interpreter steps than 9 passes × O(n + 10)
- Count arrays of size 256 fit trivially in cache; iteration over 256 elements is cheap
- Byte extraction via `(x >> (8 * pass)) & 0xFF` is a single bitwise operation per element — fast in CPython
- Base-256 is the standard choice in performance-conscious radix sort implementations

**Config value:** `RADIX_BASE = 256`, `RADIX_PASSES = 4`

**Why not base 10:** More passes means more Python loop overhead — the bottleneck in pure Python is interpreter steps, not arithmetic.
**Why not base 65536:** Count array of size 65536 is large; in pure Python, initializing and iterating over it outweighs the benefit of 2 passes instead of 4.

---

## Merge Sort

### Recommended Approach: Iterative Bottom-Up (RECOMMENDED)

**What:** Start with sub-arrays of size 1, iteratively merge adjacent pairs doubling the sub-array size each round. No recursion. O(n log n) time, O(n) space.

**Why:** Avoids recursion depth entirely. At N=1M, top-down recursive merge sort requires depth ~20 (log2(1M) ≈ 20), well within Python's limit — so recursion is actually safe here. However, iterative bottom-up avoids function call overhead at each level and is the cleaner choice for a production-quality implementation.

**Key steps:**
1. Initialize `width = 1`
2. While `width < len(arr)`: merge all adjacent pairs of width-sized blocks
3. Double `width` each iteration
4. After ceil(log2(n)) iterations, the array is sorted

**Python-specific implementation notes:**

- **Do not use list slicing for merge.** `arr[lo:mid]` and `arr[mid:hi]` create new O(k) Python list objects. At N=1M with 20 passes, this allocates O(n log n) total bytes of temporary lists — significant GC pressure.
- **Use index-based merge instead.** Pass `lo`, `mid`, `hi` indices; allocate one temporary buffer per merge call of exactly the needed size, or reuse a pre-allocated scratch buffer of size n.
- **Pre-allocate scratch buffer:** Allocate `scratch = [0] * len(arr)` once before sorting begins. Each merge reads from `arr` and writes to `scratch` (or alternates), then copies back. This reduces allocation overhead significantly.
- **Merge function signature:** `merge(arr, scratch, lo, mid, hi)` — merge arr[lo:mid] and arr[mid:hi], store result in scratch[lo:hi], then copy back to arr[lo:hi].

**Pitfalls:**
- Forgetting to handle the case where the right sub-array boundary exceeds len(arr) (last block may be smaller than `width`)
- Off-by-one errors in lo/mid/hi index arithmetic
- Using slicing (creates copies) instead of index arithmetic (uses existing list)

### Alternative: Top-Down Recursive

**When to use:** If iterative bottom-up produces a bug and needs fallback. Recursion depth is ~20 for N=1M (well within Python's 1000 default limit), so RecursionError is not a concern for merge sort.

**Tradeoff:** Slightly more overhead from function calls; identical asymptotic complexity.

---

## Quicksort

### Recommended Approach: Iterative with Explicit Stack + Median-of-Three + Lomuto Partition

**What:** Use a Python list as an explicit stack of (lo, hi) pairs. Replace recursive calls with stack push/pop. Apply median-of-three pivot selection and push-smaller-first discipline.

**Python's recursion limit:** Default is 1000. For quicksort on N=1M with random pivot, expected recursion depth is ~2 log2(1M) ≈ 40, which is safe. However, worst-case depth is N=1M (sorted array with first-element pivot), which causes RecursionError. With median-of-three, sorted-array worst case is eliminated — but adversarial inputs remain possible. The iterative approach eliminates this fragility entirely.

**Iterative quicksort structure:**
```
stack = [(0, len(arr) - 1)]
while stack:
    lo, hi = stack.pop()
    if lo >= hi:
        continue
    p = partition(arr, lo, hi)  # pivot ends up at index p
    # Push larger partition first (will be processed later)
    # Push smaller partition second (processed next)
    if (p - 1 - lo) > (hi - p - 1):
        stack.append((lo, p - 1))
        stack.append((p + 1, hi))
    else:
        stack.append((p + 1, hi))
        stack.append((lo, p - 1))
```

**Median-of-three pivot selection:**
```
mid = (lo + hi) // 2
# Sort arr[lo], arr[mid], arr[hi] among themselves, return index of median
# Move median to arr[hi] (for Lomuto) or arr[lo]
```

**Lomuto partition:**
- Pivot = arr[hi] (after median-of-three places it there)
- Scan left to right, swap elements <= pivot to left partition
- Move pivot to its final position

**Push-smaller-first discipline:** Ensures the explicit stack never holds more than O(log n) pairs simultaneously. For N=1M, maximum stack depth ≈ 20 entries. Without this, a degenerate partition sequence could accumulate O(n) stack entries.

**Pitfalls:**
- Using `sys.setrecursionlimit` instead of iterative — fragile, version-dependent in CPython 3.12+
- Not implementing push-smaller-first — stack can grow to O(n) in degenerate cases
- Lomuto partition with first-element pivot on sorted arrays — O(n^2) time. Median-of-three eliminates this.
- Off-by-one in partition boundary: Lomuto final swap must move pivot to position i+1, not i

### Alternative: Recursive with sys.setrecursionlimit

**When to use:** If iterative implementation proves difficult to debug. Use `sys.setrecursionlimit(1500000)` with median-of-three pivot. Expected depth on random arrays is ~40 for N=1M — safe in practice.

**Risk:** CPython 3.12+ changed the relationship between Python and C stack limits. `setrecursionlimit` behavior is platform-dependent. Not recommended as the primary approach.

---

## Radix Sort (LSD)

### Recommended Approach: LSD Base-256, 4 Passes, Counting Sort Inner Loop

**What:** Least-Significant Digit radix sort over 4 byte positions of a 32-bit (base-256) representation. Each pass is a stable counting sort over the current byte position.

**Integer range:** [0, 10^9 - 1]. All values fit in 4 bytes (since 10^9 - 1 < 2^30 < 2^32). Pass 0 extracts the least significant byte, pass 3 the most significant.

**Byte extraction:** `(x >> (8 * pass_index)) & 0xFF`
- Pass 0: `x & 0xFF` (bits 0-7)
- Pass 1: `(x >> 8) & 0xFF` (bits 8-15)
- Pass 2: `(x >> 16) & 0xFF` (bits 16-23)
- Pass 3: `(x >> 24) & 0xFF` (bits 24-31, at most 0x3F since 10^9 < 2^30)

**Counting sort inner loop per pass:**
1. Allocate `count = [0] * 256`
2. For each element, increment `count[(x >> shift) & 0xFF]`
3. Convert to prefix sums: `count[i] += count[i-1]` for i in 1..255
4. Build output right-to-left (for stability): iterate `arr` backwards, place each element at `count[(x >> shift) & 0xFF] - 1`, decrement count
5. Copy output back to arr (or swap references)

**Python-specific notes:**
- Use a Python list of size 256 for `count`, initialized to 0. `[0] * 256` is fast in CPython.
- Allocate one `output = [0] * n` scratch array once before all 4 passes to avoid repeated allocation.
- Iterate elements backwards in step 4 to maintain LSD stability (required for correctness).
- The inner loop runs 4 × n iterations total — this is the dominant cost in pure Python.

**Pitfalls:**
- **Forward vs. backward scan in step 4:** Forward scan breaks stability. Must iterate `arr` backwards to maintain stable ordering from previous passes.
- **Prefix sum direction:** `count[i]` must represent the cumulative count of all elements with byte value ≤ i, not < i, depending on whether you use `count[digit] - 1` indexing or `count[digit]` indexing. Be consistent.
- **Handling the most-significant byte:** For range [0, 10^9-1], byte 3 values only range from 0x00 to 0x3B (since 10^9-1 ≈ 0x3B9AC9FF). The count array is still size 256 — values 0x3C through 0xFF simply have zero counts. This is correct behavior, not a bug.
- **Non-negative integers only:** LSD radix sort as implemented does not handle negative integers. The phase requirement specifies non-negative integers (IMPL-03), so this is in-spec.

**Performance expectation:** In pure Python, the 4 × n inner loop iterations of radix sort are expensive because Python has high per-iteration overhead. It is possible that radix sort will be slower than merge/quicksort at small N (10K, 100K) and competitive or faster only at N=1M. This is the interesting empirical question Phase 2 will answer. Do not pre-optimize — implement correctly and let the benchmark reveal the truth.

---

## Correctness Testing Strategy

### Primary Check: sorted() Comparison at N=1000

For each algorithm immediately after implementation:
```python
import random
arr = [random.randint(0, 10**9 - 1) for _ in range(1000)]
result = algorithm(arr.copy())
assert result == sorted(arr), f"FAILED: {algorithm.__name__}"
print(f"PASSED: {algorithm.__name__}")
```

**Why N=1000:** Fast to run, large enough to catch most bugs (index errors, off-by-ones), matches the success criterion in the phase requirements.

### Edge Cases to Test

| Case | Array | Why It Matters |
|---|---|---|
| Already sorted | list(range(1000)) | Triggers quicksort worst case with naive pivot |
| Reverse sorted | list(range(999, -1, -1)) | Triggers quicksort worst case with first/last pivot |
| All identical | [42] * 1000 | Tests equal-element handling in Lomuto/Hoare |
| Single element | [7] | Boundary condition |
| Empty array | [] | Boundary condition |
| Two elements | [5, 3] | Minimum non-trivial case |
| Large N=10000 | random.randint(0, 10^9-1) × 10000 | Pre-benchmark sanity check |

### Correctness Gating Rule

**No algorithm proceeds to Phase 2 benchmarking until it passes all edge cases above.** Any failure in `assert result == sorted(arr)` is a blocking bug.

### Non-Mutation Requirement

The requirements specify "returns a new sorted list" (IMPL-01, IMPL-02, IMPL-03). Verify input array is unchanged:
```python
original = arr.copy()
result = algorithm(arr)
assert arr == original, "Input array was mutated"
```

---

## Config Structure

Write a `config.py` at the project root (or a path imported by Phase 2) recording all Phase 1 decisions:

```python
# config.py — Written in Phase 1, imported by Phase 2 benchmark harness
# DO NOT modify after Phase 1 is complete without re-running Phase 1 correctness checks

# Integer range for random array generation
INT_MIN = 0
INT_MAX = 10**9 - 1  # 999_999_999 — fits in 4 bytes (base-256 radix, 4 passes)

# Array sizes for benchmarking
ARRAY_SIZES = [10_000, 100_000, 1_000_000]

# Number of timing trials per (algorithm, array size) combination
TRIALS_PER_COMBINATION = 10

# Timing clock
TIMING_CLOCK = "time.perf_counter"  # documentation string; use time.perf_counter() in code

# Radix sort parameters
RADIX_BASE = 256
RADIX_PASSES = 4  # ceil(log_256(2^32)) = 4; covers all integers < 2^32

# Algorithm identifiers (used as keys in timing table)
ALGORITHM_IDS = ["merge_sort", "quicksort", "radix_sort"]

# Correctness oracle
CORRECTNESS_ORACLE = "sorted"  # documentation; use sorted() in code

# Quicksort variant
QUICKSORT_VARIANT = "iterative_median3_lomuto"
```

**Why config.py matters for Phase 2:** The benchmark harness must generate random arrays with the same INT_MIN/INT_MAX used during correctness testing. If Phase 2 uses a different range, radix sort's pass count changes and results are not comparable.

---

## Computational Feasibility

| Operation | N | Estimated Time (pure Python) | Notes |
|---|---|---|---|
| Merge sort | 10,000 | ~0.1–0.3 s | Index-based merge, ~13 passes |
| Merge sort | 1,000,000 | ~20–60 s | ~20 passes; Python list overhead dominates |
| Quicksort | 10,000 | ~0.05–0.2 s | Iterative, random input |
| Quicksort | 1,000,000 | ~10–40 s | Expected depth ~40 frames |
| Radix sort | 10,000 | ~0.05–0.2 s | 4 passes, 10K × 4 = 40K iterations |
| Radix sort | 1,000,000 | ~5–20 s | 4 passes, 1M × 4 = 4M iterations |

These are rough estimates based on CPython's ~10–50M simple operations per second for list-heavy loops. Actual values will be measured in Phase 2. The key point: N=1M with all three algorithms is feasible in a single session (total ~1–3 minutes for 10 trials each).

**Stop condition from PROJECT.md:** If merge/quicksort timing ratio N=1M/N=10K falls outside [20, 500], suspect a buggy implementation. For O(n log n), expected ratio is (1M × 20) / (10K × 13) ≈ 154.

---

## Common Pitfalls

### Pitfall 1: List Slicing in Merge Sort

**What goes wrong:** `left = arr[lo:mid]` creates a new list object of size (mid-lo). At N=1M with 20 merge passes, this allocates ~20 MB of temporary lists per sort call, creating GC pressure and slowing the benchmark unfairly.
**Why it happens:** It's the most natural Python idiom.
**How to avoid:** Pass indices (lo, mid, hi) and write to a pre-allocated scratch buffer.
**Warning signs:** Merge sort is dramatically slower than expected even at N=10K.
**Recovery:** Rewrite merge to use index arithmetic and a scratch buffer.

### Pitfall 2: Recursive Quicksort on Large N

**What goes wrong:** RecursionError at ~1000 levels of recursion. For random arrays at N=1M, expected depth is ~40 — usually safe. But any adversarial input (pre-sorted, reverse-sorted, all-equal) causes O(n) recursion depth with naive pivot.
**Why it happens:** Python's call stack is limited.
**How to avoid:** Use iterative quicksort with explicit stack (recommended), or use median-of-three pivot with `sys.setrecursionlimit(1500000)` (fallback).
**Warning signs:** RecursionError on test arrays; sort hangs on nearly-sorted input.
**Recovery:** Switch to iterative implementation.

### Pitfall 3: LSD Radix Sort Stability (Forward vs. Backward Scan)

**What goes wrong:** Iterating the input array forward during the placement step breaks the stability required for LSD correctness. The sort produces incorrect output.
**Why it happens:** The placement loop feels like it should go forward (left to right), but backward iteration is required to maintain the stable ordering established in previous passes.
**How to avoid:** Iterate `arr` backwards in the placement step (step 4 of each counting sort pass).
**Warning signs:** `assert result == sorted(arr)` fails at N=1000 with duplicates.
**Recovery:** Reverse the iteration direction in the placement loop.

### Pitfall 4: Quicksort Stack Growth Without Push-Smaller-First

**What goes wrong:** Without push-smaller-first discipline, the explicit stack can accumulate O(n) entries on degenerate partition sequences, consuming O(n) memory and slowing the sort.
**Why it happens:** Natural implementation pushes both sub-arrays without size comparison.
**How to avoid:** Always push the larger sub-array first (processed later) and the smaller one second (processed next).
**Warning signs:** Stack list grows to millions of entries on a test sort; very high memory use.
**Recovery:** Add size comparison before each push pair.

### Pitfall 5: Input Array Mutation

**What goes wrong:** All three algorithms modify the input array in-place but the contract requires returning a new sorted list without mutating the input (IMPL-01, IMPL-02, IMPL-03).
**Why it happens:** In-place algorithms naturally modify the array passed to them.
**How to avoid:** For merge sort and radix sort: work on a copy internally. For quicksort: `arr = list(input_arr)` at the top of the function.
**Warning signs:** Correctness check passes but `assert arr == original` fails.
**Recovery:** Add `arr = list(input_arr)` at the start of each sort function.

---

## Validation Strategies

| Check | What It Validates | How to Perform | Expected Result |
|---|---|---|---|
| `result == sorted(arr)` at N=1000 | Correctness | Direct comparison after each implementation | Exact equality |
| `arr == original_copy` after sort | Non-mutation | Compare before/after | Exact equality |
| Edge cases (empty, single, reversed) | Boundary conditions | Run each edge case | No crash, correct output |
| Timing ratio N=1M/N=10K | Complexity scaling (O(n log n)) | Phase 2 result | 20–500 for merge/quick |
| Timing ratio N=1M/N=10K (radix) | Complexity scaling (O(n)) | Phase 2 result | ~100 (linear) |
| Radix sort on range boundary | Handles max value correctly | Test on [10^9 - 1] | No crash, correct output |

**Red flags during implementation:**
- RecursionError in quicksort (use iterative)
- `assert result == sorted(arr)` fails (implementation bug, debug before proceeding)
- Merge sort returns input with duplicates removed (common merge bug: wrong comparison operator)
- Radix sort returns partially sorted array (stability bug in placement step)

---

## Key References / Sources

### Primary (HIGH confidence)

- **CLRS:** Cormen, Leiserson, Rivest, Stein — "Introduction to Algorithms" (4th ed.)
  - Ch. 2: Merge sort derivation and correctness
  - Ch. 7: Quicksort, Hoare/Lomuto partition, randomized pivot analysis
  - Ch. 8: Counting sort and radix sort, LSD correctness proof
  - Problem 7-4: Stack depth analysis for quicksort tail call optimization
- **Sedgewick & Wayne** — "Algorithms" (4th ed.), Ch. 2: Sorting
  - Push-smaller-first discipline for iterative quicksort (O(log n) stack bound)
  - Bottom-up merge sort iterative implementation
- **Python docs:** `sys.setrecursionlimit` — https://docs.python.org/3/library/sys.html
  - Confirms default limit of 1000; notes CPython 3.12+ decoupled C-stack limit
- **Princeton COS 226 Lecture Notes** — "Radix Sorts" (Sedgewick)
  - Base-256 gives 4 passes for 32-bit keys; cache-fits at 256 entries
  - https://www.cs.princeton.edu/courses/archive/spr07/cos226/lectures/11RadixSort.pdf

### Secondary (MEDIUM confidence)

- GeeksforGeeks — "Iterative Quick Sort" (Python) — https://www.geeksforgeeks.org/python/python-program-for-iterative-quick-sort/
  - Confirmed push-smaller-first technique reduces stack to O(log n)
- GeeksforGeeks — "Hoare's vs Lomuto partition scheme" — https://www.geeksforgeeks.org/dsa/hoares-vs-lomuto-partition-scheme-quicksort/
  - Hoare: 3× fewer swaps; Lomuto: simpler, degrades on all-equal input (not a concern here given large integer range)
- GeeksforGeeks — "Iterative Merge Sort" — https://www.geeksforgeeks.org/dsa/iterative-merge-sort/
  - Confirmed bottom-up iterative eliminates recursion overhead
- Baeldung CS — "Top-Down vs Bottom-Up Merge Sort" — https://www.baeldung.com/cs/merge-sort-top-down-vs-bottom-up
  - Bottom-up is memory-efficient (no call stack overhead)
- StackAbuse — "Radix Sort in Python" — https://stackabuse.com/radix-sort-in-python/
  - Confirmed counting sort as inner loop; base-256 byte extraction pattern

### Tertiary (LOW confidence — for cross-reference only)

- GitHub Gist: LSD Radix sort in Python base 10 — https://gist.github.com/d47ffbe13e0170edc542a15004842232 (single source, used for pattern cross-check)

---

## Caveats and Alternatives

**What assumption am I making that might be wrong?**
- I assume base-256 is faster than base-10 in pure Python. This is well-supported theoretically (2.25× fewer passes), but CPython's attribute lookup overhead means the crossover point could shift. Phase 2 timing will reveal the empirical truth; the implementation choice is fixed by config.py and Phase 2 does not revisit it.

**Alternative I dismissed — why?**
- Recursive quicksort with `sys.setrecursionlimit(1500000)`: Dismissed because CPython 3.12+ changed the C-stack limit behavior, making it unreliable. Iterative is safer and has no correctness downside.
- Hoare partition instead of Lomuto: Hoare is ~3× fewer swaps, marginally faster, but harder to get right (the partition return index semantics differ from Lomuto). Since correctness is the Phase 1 goal, Lomuto is recommended. If Phase 2 benchmarks show quicksort significantly slower than expected, switching to Hoare is a valid micro-optimization.
- MSD radix sort instead of LSD: MSD can be more cache-friendly but requires recursion or explicit stack management per bucket, and is significantly more complex to implement correctly. LSD with counting sort is the standard pedagogical and production choice.

**Limitation I may be understating:**
- Pure Python radix sort overhead: in CPython, even O(n) with a large constant can lose to O(n log n) with a small constant. The Phase 2 benchmark may show radix sort slower than merge/quicksort at all three array sizes. This is a legitimate empirical result, not a bug. Do not re-implement if this occurs.

**Simpler alternative I considered:**
- Using `arr[:]` = `sorted(arr)` as a placeholder: rejected — the phase requires correct standalone implementations, not wrappers.

---

## Metadata

**Confidence breakdown:**

- Design decisions (integer range, quicksort variant, radix base): HIGH — well-supported by algorithm theory and Python-specific constraints
- Merge sort implementation approach: HIGH — standard textbook material
- Quicksort iterative approach: HIGH — well-documented in CLRS and multiple references
- Radix sort byte extraction: HIGH — standard bitwise technique
- Performance estimates: MEDIUM — CPython overhead is variable; estimates based on typical benchmarks, not measurements on this specific machine
- Correctness testing strategy: HIGH — sorted() oracle is exact

**Research date:** 2026-04-11
**Valid until:** These algorithm results are stable. Python version specifics (recursion limit behavior in 3.12+) may change in future releases.
