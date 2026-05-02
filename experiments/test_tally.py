#!/usr/bin/env python3
"""Fixture tests for tally.py split input/output cost accounting.

These tests verify that the split-rate cost arithmetic matches
hand-calculated values from the Step 1 pilot data.

Pilot data (from experiments/results/awl_arm.jsonl):
  Task 01: prompt_tokens=4082, completion_tokens=182
  Task 02: prompt_tokens=1931, completion_tokens=142
  Task 03: prompt_tokens=2113, completion_tokens=151

Claude Opus 4.7 rates: $5/MTok input, $25/MTok output
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

# Allow importing tally from experiments/
EXPERIMENTS_DIR = Path(__file__).resolve().parent
TALLY_SCRIPT = EXPERIMENTS_DIR / "tally.py"


PILOT_RECORDS = [
    {
        "task_id": "01_string_helper",
        "attempts": 2,
        "checks_passed": False,
        "prompt_tokens": 4082,
        "completion_tokens": 182,
        "total_tokens": 4264,
        "wall_ms": 29686,
        "status": "error",
    },
    {
        "task_id": "02_validate_input",
        "attempts": 1,
        "checks_passed": True,
        "prompt_tokens": 1931,
        "completion_tokens": 142,
        "total_tokens": 2073,
        "wall_ms": 15431,
        "status": "ok",
    },
    {
        "task_id": "03_fix_off_by_one",
        "attempts": 1,
        "checks_passed": True,
        "prompt_tokens": 2113,
        "completion_tokens": 151,
        "total_tokens": 2264,
        "wall_ms": 17577,
        "status": "ok",
    },
]

# Hand-calculated expected costs at $5 input / $25 output per MTok
EXPECTED_COSTS = {
    "01_string_helper": 4082 * 5 / 1e6 + 182 * 25 / 1e6,   # 0.024960
    "02_validate_input": 1931 * 5 / 1e6 + 142 * 25 / 1e6,   # 0.013205
    "03_fix_off_by_one": 2113 * 5 / 1e6 + 151 * 25 / 1e6,   # 0.014340
}
EXPECTED_TOTAL_COST = sum(EXPECTED_COSTS.values())  # 0.052505


def _write_jsonl(path: Path, records: list[dict]) -> None:
    with path.open("w", encoding="utf-8") as fh:
        for record in records:
            fh.write(json.dumps(record) + "\n")


def _run_tally(args: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(TALLY_SCRIPT)] + args,
        capture_output=True,
        text=True,
    )


class TestSplitCostPilotData(unittest.TestCase):
    """Verify split-rate cost arithmetic against pilot data."""

    def setUp(self) -> None:
        self._tmpdir = tempfile.TemporaryDirectory()
        self.tmpdir = Path(self._tmpdir.name)
        self.awl_path = self.tmpdir / "awl_arm.jsonl"
        _write_jsonl(self.awl_path, PILOT_RECORDS)

    def tearDown(self) -> None:
        self._tmpdir.cleanup()

    def test_split_cost_pilot_data(self) -> None:
        """Cost at $5 input / $25 output per MTok matches hand-calculated pilot values."""
        result = _run_tally([
            "--awl", str(self.awl_path),
            "--input-cost-per-mtok", "5",
            "--output-cost-per-mtok", "25",
        ])
        self.assertEqual(result.returncode, 0, f"tally failed: {result.stderr}")
        output = result.stdout

        # Check per-task costs appear in output
        for task_id, expected_cost in EXPECTED_COSTS.items():
            cost_str = f"${expected_cost:.6f}"
            self.assertIn(
                cost_str,
                output,
                f"Expected cost {cost_str} for {task_id} not found in output",
            )

        # Check total cost
        total_str = f"${EXPECTED_TOTAL_COST:.6f}"
        self.assertIn(
            total_str,
            output,
            f"Expected total cost {total_str} not found in output",
        )

    def test_per_task_cost_values(self) -> None:
        """Verify each task's cost individually against hand calculation."""
        self.assertAlmostEqual(
            EXPECTED_COSTS["01_string_helper"], 0.024960, places=6,
            msg="Task 01 cost mismatch",
        )
        self.assertAlmostEqual(
            EXPECTED_COSTS["02_validate_input"], 0.013205, places=6,
            msg="Task 02 cost mismatch",
        )
        self.assertAlmostEqual(
            EXPECTED_COSTS["03_fix_off_by_one"], 0.014340, places=6,
            msg="Task 03 cost mismatch",
        )
        self.assertAlmostEqual(
            EXPECTED_TOTAL_COST, 0.052505, places=6,
            msg="Total cost mismatch",
        )


class TestBlendedVsSplitDifference(unittest.TestCase):
    """Demonstrate that blended cost underestimates real spend."""

    def test_blended_underestimates(self) -> None:
        """Blended $5/MTok on total_tokens gives $0.043005, 18.1% below split $0.052505."""
        total_tokens = sum(r["total_tokens"] for r in PILOT_RECORDS)
        self.assertEqual(total_tokens, 8601)

        blended_cost = total_tokens * 5.0 / 1e6
        self.assertAlmostEqual(blended_cost, 0.043005, places=6)

        split_cost = EXPECTED_TOTAL_COST
        self.assertAlmostEqual(split_cost, 0.052505, places=6)

        # Blended underestimates by (split - blended) / split = ~18.1%
        underestimate_pct = (split_cost - blended_cost) / split_cost * 100
        self.assertGreater(underestimate_pct, 15.0, "Blended should underestimate by >15%")
        self.assertLess(underestimate_pct, 25.0, "Sanity: underestimate should be <25%")


class TestNoCostFlags(unittest.TestCase):
    """Verify tally works without cost flags (no crash, no cost lines)."""

    def setUp(self) -> None:
        self._tmpdir = tempfile.TemporaryDirectory()
        self.tmpdir = Path(self._tmpdir.name)
        self.awl_path = self.tmpdir / "awl_arm.jsonl"
        _write_jsonl(self.awl_path, PILOT_RECORDS)

    def tearDown(self) -> None:
        self._tmpdir.cleanup()

    def test_no_cost_flags(self) -> None:
        """Omitting cost flags produces a report without cost lines."""
        result = _run_tally(["--awl", str(self.awl_path)])
        self.assertEqual(result.returncode, 0, f"tally failed: {result.stderr}")
        output = result.stdout

        # Report should still have per-task results
        self.assertIn("01_string_helper", output)
        self.assertIn("02_validate_input", output)
        self.assertIn("03_fix_off_by_one", output)

        # No cost column should appear (no $ sign in aggregate for cost)
        self.assertNotIn("split-rate", output)
        self.assertNotIn("awl total cost", output)

    def test_total_tokens_visible(self) -> None:
        """Total tokens per task remain visible for old-artifact readability."""
        result = _run_tally(["--awl", str(self.awl_path)])
        self.assertEqual(result.returncode, 0, f"tally failed: {result.stderr}")
        output = result.stdout

        # Total tokens should be visible in the per-task table
        self.assertIn("4264", output)   # task 01 total_tokens
        self.assertIn("2073", output)   # task 02 total_tokens
        self.assertIn("2264", output)   # task 03 total_tokens


class TestBackwardCompat(unittest.TestCase):
    """Verify backward compatibility with deprecated --cost-per-mtok flag."""

    def setUp(self) -> None:
        self._tmpdir = tempfile.TemporaryDirectory()
        self.tmpdir = Path(self._tmpdir.name)
        self.awl_path = self.tmpdir / "awl_arm.jsonl"
        _write_jsonl(self.awl_path, PILOT_RECORDS)

    def tearDown(self) -> None:
        self._tmpdir.cleanup()

    def test_deprecated_flag_warns(self) -> None:
        """Old --cost-per-mtok flag works but prints deprecation warning."""
        result = _run_tally([
            "--awl", str(self.awl_path),
            "--cost-per-mtok", "5.0",
        ])
        self.assertEqual(result.returncode, 0, f"tally failed: {result.stderr}")
        self.assertIn("deprecated", result.stderr.lower())

    def test_deprecated_flag_produces_cost(self) -> None:
        """Old flag at $5/MTok blended still produces a cost report."""
        result = _run_tally([
            "--awl", str(self.awl_path),
            "--cost-per-mtok", "5.0",
        ])
        self.assertEqual(result.returncode, 0, f"tally failed: {result.stderr}")
        # With blended $5 for both input and output, cost should appear
        self.assertIn("awl total cost", result.stdout)

    def test_deprecated_and_split_conflict(self) -> None:
        """Cannot use --cost-per-mtok with --input-cost-per-mtok simultaneously."""
        result = _run_tally([
            "--awl", str(self.awl_path),
            "--cost-per-mtok", "5.0",
            "--input-cost-per-mtok", "5.0",
        ])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cannot be used together", result.stderr)


if __name__ == "__main__":
    unittest.main()
