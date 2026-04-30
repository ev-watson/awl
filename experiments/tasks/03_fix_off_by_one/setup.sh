#!/usr/bin/env bash
set -euo pipefail
SANDBOX="experiments/sandbox/03"
rm -rf "$SANDBOX"
mkdir -p "$SANDBOX"
cat >"$SANDBOX/moving_average.py" <<'PY'
def moving_average(values, window):
    if window <= 0 or window > len(values):
        raise ValueError("invalid window")
    out = []
    # BUG: range stops one short of the last full window.
    for i in range(len(values) - window):
        chunk = values[i:i + window]
        out.append(round(sum(chunk) / window, 4))
    return out
PY
cat >"$SANDBOX/test_moving_average.py" <<'PY'
import unittest
from moving_average import moving_average


class MovingAverage(unittest.TestCase):
    def test_simple(self):
        self.assertEqual(moving_average([1, 2, 3, 4], 2), [1.5, 2.5, 3.5])

    def test_window_equals_length(self):
        self.assertEqual(moving_average([1, 2, 3], 3), [2.0])

    def test_invalid_window_zero(self):
        with self.assertRaises(ValueError):
            moving_average([1, 2, 3], 0)

    def test_invalid_window_too_large(self):
        with self.assertRaises(ValueError):
            moving_average([1, 2], 3)

    def test_rounding(self):
        self.assertEqual(moving_average([1.0, 2.0, 3.3333333], 2), [1.5, 2.6667])


if __name__ == "__main__":
    unittest.main()
PY
