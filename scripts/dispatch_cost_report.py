#!/usr/bin/env python3
"""Summarize Awl dispatch logs and estimate paid-token savings."""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path
from typing import Any


ERROR_EVENTS = {
    "format_retries_exhausted",
    "missing_code",
    "model_status_error",
    "preflight_failed",
    "verify_command_error",
    "verify_failed",
}

SUCCESS_EVENTS = {"apply_without_verify", "model_response_valid", "verify_passed"}


def default_logs_dir() -> Path:
    if "AWL_CONFIG_DIR" in os.environ:
        return Path(os.environ["AWL_CONFIG_DIR"]) / "dispatches"
    if "XDG_CONFIG_HOME" in os.environ:
        return Path(os.environ["XDG_CONFIG_HOME"]) / "awl" / "dispatches"
    if "APPDATA" in os.environ:
        return Path(os.environ["APPDATA"]) / "awl" / "dispatches"
    return Path.home() / ".config" / "awl" / "dispatches"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize local Awl dispatch logs for cost-savings analysis."
    )
    parser.add_argument(
        "--logs-dir",
        type=Path,
        default=default_logs_dir(),
        help="Directory containing Awl dispatch JSONL logs.",
    )
    parser.add_argument(
        "--days",
        type=float,
        help="Only include logs modified in the last N days.",
    )
    parser.add_argument(
        "--frontier-direct-tokens",
        type=int,
        default=0,
        help="Estimated paid frontier tokens for doing these tasks directly.",
    )
    parser.add_argument(
        "--avg-frontier-direct-tokens",
        type=int,
        default=0,
        help="Estimated paid frontier tokens per dispatch if no total estimate is known.",
    )
    parser.add_argument(
        "--frontier-cost-per-mtok",
        type=float,
        default=0.0,
        help="Blended paid frontier cost per million tokens.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit machine-readable JSON instead of text.",
    )
    return parser.parse_args()


def iter_logs(logs_dir: Path, days: float | None) -> list[Path]:
    if not logs_dir.exists():
        return []
    cutoff = None
    if days is not None:
        cutoff = time.time() - (days * 24 * 60 * 60)
    paths = []
    for path in logs_dir.glob("*.jsonl"):
        if cutoff is not None and path.stat().st_mtime < cutoff:
            continue
        paths.append(path)
    return sorted(paths)


def load_events(path: Path) -> list[dict[str, Any]]:
    events = []
    with path.open(encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            line = line.strip()
            if not line:
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError as exc:
                raise ValueError(f"{path}:{line_number}: invalid JSON: {exc}") from exc
            if isinstance(event, dict):
                events.append(event)
    return events


def usage_tokens(usage: Any) -> tuple[int, int, int]:
    if not isinstance(usage, dict):
        return (0, 0, 0)
    prompt = int(usage.get("prompt_tokens") or usage.get("input_tokens") or 0)
    completion = int(usage.get("completion_tokens") or usage.get("output_tokens") or 0)
    total = int(usage.get("total_tokens") or prompt + completion)
    return (prompt, completion, total)


def summarize_log(path: Path) -> dict[str, Any]:
    events = load_events(path)
    prompt_tokens = 0
    completion_tokens = 0
    total_tokens = 0
    attempts = 0
    event_names = []
    start = {}

    for event in events:
        name = event.get("event")
        if isinstance(name, str):
            event_names.append(name)
        if name == "dispatch_start":
            start = event
        if "usage" in event:
            prompt, completion, total = usage_tokens(event.get("usage"))
            prompt_tokens += prompt
            completion_tokens += completion
            total_tokens += total
            attempts += 1

    failed = any(name in ERROR_EVENTS for name in event_names)
    succeeded = any(name in SUCCESS_EVENTS for name in event_names) and not failed
    return {
        "id": path.stem,
        "path": str(path),
        "level": start.get("level"),
        "apply": bool(start.get("apply")),
        "target_path": start.get("target_path"),
        "verify_command": start.get("verify_command"),
        "attempts": attempts,
        "prompt_tokens": prompt_tokens,
        "completion_tokens": completion_tokens,
        "total_tokens": total_tokens,
        "succeeded": succeeded,
        "failed": failed,
        "events": event_names,
    }


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    dispatches = [summarize_log(path) for path in iter_logs(args.logs_dir, args.days)]
    local_tokens = sum(item["total_tokens"] for item in dispatches)
    frontier_direct_tokens = args.frontier_direct_tokens
    if frontier_direct_tokens == 0 and args.avg_frontier_direct_tokens:
        frontier_direct_tokens = args.avg_frontier_direct_tokens * len(dispatches)
    estimated_cost_avoided = (
        frontier_direct_tokens / 1_000_000 * args.frontier_cost_per_mtok
    )

    return {
        "logs_dir": str(args.logs_dir),
        "dispatch_count": len(dispatches),
        "success_count": sum(1 for item in dispatches if item["succeeded"]),
        "failure_count": sum(1 for item in dispatches if item["failed"]),
        "apply_count": sum(1 for item in dispatches if item["apply"]),
        "local_worker_tokens": local_tokens,
        "frontier_direct_tokens_estimate": frontier_direct_tokens,
        "frontier_cost_per_mtok": args.frontier_cost_per_mtok,
        "estimated_paid_cost_avoided": round(estimated_cost_avoided, 6),
        "dispatches": dispatches,
    }


def print_text(report: dict[str, Any]) -> None:
    print(f"logs_dir: {report['logs_dir']}")
    print(f"dispatches: {report['dispatch_count']}")
    print(f"successes: {report['success_count']}")
    print(f"failures: {report['failure_count']}")
    print(f"apply_dispatches: {report['apply_count']}")
    print(f"local_worker_tokens: {report['local_worker_tokens']}")
    print(f"frontier_direct_tokens_estimate: {report['frontier_direct_tokens_estimate']}")
    if report["frontier_cost_per_mtok"]:
        print(f"frontier_cost_per_mtok: {report['frontier_cost_per_mtok']}")
        print(f"estimated_paid_cost_avoided: ${report['estimated_paid_cost_avoided']}")


def main() -> int:
    args = parse_args()
    try:
        report = build_report(args)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_text(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
