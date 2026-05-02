#!/usr/bin/env python3
"""Fixture tests for dispatch_cost_report.py split costs and failure categories."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPTS_DIR = Path(__file__).resolve().parent
REPORT_SCRIPT = SCRIPTS_DIR / "dispatch_cost_report.py"


def _write_jsonl(path: Path, events: list[dict]) -> None:
    with path.open("w", encoding="utf-8") as handle:
        for event in events:
            handle.write(json.dumps(event) + "\n")


def _run_report(args: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(REPORT_SCRIPT)] + args,
        capture_output=True,
        text=True,
    )


class TestDispatchCostReport(unittest.TestCase):
    def setUp(self) -> None:
        self._tmpdir = tempfile.TemporaryDirectory()
        self.tmpdir = Path(self._tmpdir.name)

    def tearDown(self) -> None:
        self._tmpdir.cleanup()

    def test_split_cost_from_jsonl(self) -> None:
        _write_jsonl(
            self.tmpdir / "one.jsonl",
            [
                {"event": "dispatch_start", "level": 2, "apply": True},
                {
                    "event": "model_response_valid",
                    "usage": {
                        "prompt_tokens": 1000,
                        "completion_tokens": 200,
                        "total_tokens": 1200,
                    },
                },
                {"event": "verify_passed"},
            ],
        )

        result = _run_report(
            [
                "--logs-dir",
                str(self.tmpdir),
                "--frontier-direct-input-tokens",
                "1000",
                "--frontier-direct-output-tokens",
                "200",
                "--frontier-input-cost-per-mtok",
                "5",
                "--frontier-output-cost-per-mtok",
                "25",
                "--json",
            ]
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["estimated_paid_cost_avoided"], 0.01)
        self.assertEqual(report["frontier_input_cost_per_mtok"], 5.0)
        self.assertEqual(report["frontier_output_cost_per_mtok"], 25.0)

    def test_failure_category_aggregation(self) -> None:
        _write_jsonl(
            self.tmpdir / "verify.jsonl",
            [
                {"event": "dispatch_start", "apply": True},
                {"event": "verify_failed", "failure_category": "verify"},
            ],
        )
        _write_jsonl(
            self.tmpdir / "schema.jsonl",
            [
                {"event": "dispatch_start", "apply": True},
                {"event": "missing_code", "failure_category": "schema"},
            ],
        )

        result = _run_report(["--logs-dir", str(self.tmpdir), "--json"])
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["failure_categories"]["verify"], 1)
        self.assertEqual(report["failure_categories"]["schema"], 1)

    def test_missing_failure_category_falls_back_to_unknown(self) -> None:
        _write_jsonl(
            self.tmpdir / "old.jsonl",
            [
                {"event": "dispatch_start", "apply": True},
                {"event": "verify_failed"},
            ],
        )

        result = _run_report(["--logs-dir", str(self.tmpdir), "--json"])
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["failure_categories"]["unknown"], 1)

    def test_deprecated_frontier_cost_flag_conflicts_with_split_flags(self) -> None:
        result = _run_report(
            [
                "--logs-dir",
                str(self.tmpdir),
                "--frontier-cost-per-mtok",
                "5",
                "--frontier-input-cost-per-mtok",
                "5",
            ]
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cannot be used together", result.stderr)


if __name__ == "__main__":
    unittest.main()
