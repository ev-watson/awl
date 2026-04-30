#!/usr/bin/env python3
"""Compare A/B arms of the Awl token-savings experiment.

Inputs:
  - experiments/results/awl_arm.jsonl    (produced by run_awl_arm.sh)
  - experiments/results/baseline.csv     (you fill this in by hand from your
    frontier billing dashboard, after running each task with frontier-only)

baseline.csv columns:
  task_id, frontier_tokens, frontier_pass, wall_ms

Output: a markdown report on stdout summarizing per-task and aggregate results.
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path
from typing import Any


def load_awl_arm(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    out: list[dict[str, Any]] = []
    with path.open(encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            out.append(json.loads(line))
    return out


def load_baseline(path: Path) -> dict[str, dict[str, Any]]:
    if not path.exists():
        return {}
    out: dict[str, dict[str, Any]] = {}
    with path.open(encoding="utf-8", newline="") as fh:
        reader = csv.DictReader(fh)
        for row in reader:
            task_id = (row.get("task_id") or "").strip()
            if not task_id:
                continue
            tokens = int(row.get("frontier_tokens") or 0)
            wall = int(row.get("wall_ms") or 0)
            passed = (row.get("frontier_pass") or "").strip().lower() in {"1", "true", "yes", "y"}
            out[task_id] = {
                "frontier_tokens": tokens,
                "frontier_pass": passed,
                "wall_ms": wall,
            }
    return out


def percent_savings(awl: int, baseline: int) -> float | None:
    if baseline <= 0:
        return None
    return (1.0 - awl / baseline) * 100.0


def render_report(
    awl_records: list[dict[str, Any]],
    baseline: dict[str, dict[str, Any]],
    cost_per_mtok: float,
) -> str:
    lines: list[str] = []
    lines.append("# A/B savings — Awl vs frontier-only baseline\n")

    # Per-task table.
    lines.append("## Per-task results\n")
    lines.append(
        "| task | awl pass | awl tokens | awl wall (ms) | "
        "baseline pass | baseline tokens | baseline wall (ms) | "
        "token savings |"
    )
    lines.append(
        "|---|---|---|---|---|---|---|---|"
    )

    awl_pass_count = 0
    base_pass_count = 0
    awl_total_tokens = 0
    base_total_tokens = 0
    awl_wall_total = 0
    base_wall_total = 0
    counted_savings: list[float] = []

    for record in awl_records:
        task_id = record["task_id"]
        awl_pass = bool(record.get("checks_passed"))
        awl_tokens = int(record.get("total_tokens") or 0)
        awl_wall = int(record.get("wall_ms") or 0)

        base = baseline.get(task_id, {})
        base_pass = bool(base.get("frontier_pass", False))
        base_tokens = int(base.get("frontier_tokens") or 0)
        base_wall = int(base.get("wall_ms") or 0)

        savings = percent_savings(awl_tokens, base_tokens)
        savings_str = f"{savings:.1f}%" if savings is not None else "—"

        lines.append(
            f"| {task_id} | {'✓' if awl_pass else '✗'} | {awl_tokens} | {awl_wall} | "
            f"{'✓' if base_pass else ('—' if not base else '✗')} | "
            f"{base_tokens or '—'} | {base_wall or '—'} | {savings_str} |"
        )

        awl_pass_count += int(awl_pass)
        base_pass_count += int(base_pass)
        awl_total_tokens += awl_tokens
        base_total_tokens += base_tokens
        awl_wall_total += awl_wall
        base_wall_total += base_wall
        if savings is not None and awl_pass:
            counted_savings.append(savings)

    n = len(awl_records)
    lines.append("")
    lines.append("## Aggregate\n")
    lines.append(f"- tasks attempted: **{n}**")
    lines.append(
        f"- awl pass rate: **{awl_pass_count}/{n}** "
        f"({100.0 * awl_pass_count / n if n else 0:.0f}%)"
    )
    if baseline:
        baseline_n = sum(1 for r in awl_records if r["task_id"] in baseline)
        lines.append(
            f"- baseline pass rate: **{base_pass_count}/{baseline_n}** "
            f"({100.0 * base_pass_count / baseline_n if baseline_n else 0:.0f}%)"
        )
    lines.append(f"- total awl tokens: **{awl_total_tokens}**")
    if base_total_tokens:
        lines.append(f"- total baseline tokens: **{base_total_tokens}**")
        agg_savings = percent_savings(awl_total_tokens, base_total_tokens)
        lines.append(
            f"- aggregate token reduction: **{agg_savings:.1f}%**"
            if agg_savings is not None
            else "- aggregate token reduction: —"
        )
    if counted_savings:
        avg = sum(counted_savings) / len(counted_savings)
        lines.append(
            f"- per-task token reduction (passing tasks only, mean): **{avg:.1f}%**"
        )

    if cost_per_mtok > 0 and base_total_tokens:
        avoided = (base_total_tokens / 1_000_000) * cost_per_mtok
        lines.append(
            f"- estimated paid cost avoided at ${cost_per_mtok}/Mtok: **${avoided:.4f}**"
        )

    lines.append("")
    lines.append(
        "**Success threshold (per UPDATED_PROGRESS_REPORT.md):** "
        "≥25–40% paid token reduction, ≥60–70% awl-passing tasks."
    )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--awl",
        type=Path,
        default=Path("experiments/results/awl_arm.jsonl"),
    )
    parser.add_argument(
        "--baseline",
        type=Path,
        default=Path("experiments/results/baseline.csv"),
    )
    parser.add_argument(
        "--cost-per-mtok",
        type=float,
        default=0.0,
        help="Frontier blended $/Mtok for the avoided-cost line (e.g. 5.0).",
    )
    args = parser.parse_args()

    awl = load_awl_arm(args.awl)
    if not awl:
        print(f"error: no awl results at {args.awl}", file=sys.stderr)
        return 1

    baseline = load_baseline(args.baseline)
    print(render_report(awl, baseline, args.cost_per_mtok))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
