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
    "network_error",
    "format_retries_exhausted",
    "missing_code",
    "model_status_error",
    "preflight_failed",
    "preflight_unresolved_imports",
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
        "--frontier-direct-input-tokens",
        type=int,
        default=0,
        help="Estimated frontier input tokens for doing these tasks directly.",
    )
    parser.add_argument(
        "--frontier-direct-output-tokens",
        type=int,
        default=0,
        help="Estimated frontier output tokens for doing these tasks directly.",
    )
    parser.add_argument(
        "--frontier-input-cost-per-mtok",
        type=float,
        default=0.0,
        help="Paid frontier input cost per million tokens.",
    )
    parser.add_argument(
        "--frontier-output-cost-per-mtok",
        type=float,
        default=0.0,
        help="Paid frontier output cost per million tokens.",
    )
    parser.add_argument(
        "--frontier-cost-per-mtok",
        type=float,
        default=None,
        help="DEPRECATED: use split input/output cost flags instead.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit machine-readable JSON instead of text.",
    )
    args = parser.parse_args()
    if args.frontier_cost_per_mtok is not None:
        if args.frontier_input_cost_per_mtok or args.frontier_output_cost_per_mtok:
            parser.error(
                "--frontier-cost-per-mtok cannot be used together with "
                "--frontier-input-cost-per-mtok or --frontier-output-cost-per-mtok"
            )
        print(
            "warning: --frontier-cost-per-mtok is deprecated; use split "
            "input/output cost flags for accurate estimates.",
            file=sys.stderr,
        )
        args.frontier_input_cost_per_mtok = args.frontier_cost_per_mtok
        args.frontier_output_cost_per_mtok = args.frontier_cost_per_mtok
    return args


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


def event_failure_category(events: list[dict[str, Any]]) -> str | None:
    for event in reversed(events):
        category = event.get("failure_category")
        if isinstance(category, str) and category:
            return category
    return None


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
    failure_category = event_failure_category(events)
    if failed and not failure_category:
        failure_category = "unknown"
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
        "failure_category": failure_category,
        "events": event_names,
    }


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    dispatches = [summarize_log(path) for path in iter_logs(args.logs_dir, args.days)]
    local_tokens = sum(item["total_tokens"] for item in dispatches)
    frontier_direct_tokens = args.frontier_direct_tokens
    if frontier_direct_tokens == 0 and args.avg_frontier_direct_tokens:
        frontier_direct_tokens = args.avg_frontier_direct_tokens * len(dispatches)
    frontier_input_tokens = args.frontier_direct_input_tokens
    frontier_output_tokens = args.frontier_direct_output_tokens
    if frontier_input_tokens == 0 and frontier_output_tokens == 0 and frontier_direct_tokens:
        frontier_input_tokens = frontier_direct_tokens
    estimated_cost_avoided = (
        frontier_input_tokens / 1_000_000 * args.frontier_input_cost_per_mtok
        + frontier_output_tokens / 1_000_000 * args.frontier_output_cost_per_mtok
    )
    failure_categories: dict[str, int] = {}
    for item in dispatches:
        if not item["failed"]:
            continue
        category = item.get("failure_category") or "unknown"
        failure_categories[category] = failure_categories.get(category, 0) + 1

    return {
        "logs_dir": str(args.logs_dir),
        "dispatch_count": len(dispatches),
        "success_count": sum(1 for item in dispatches if item["succeeded"]),
        "failure_count": sum(1 for item in dispatches if item["failed"]),
        "apply_count": sum(1 for item in dispatches if item["apply"]),
        "local_worker_tokens": local_tokens,
        "frontier_direct_tokens_estimate": frontier_direct_tokens,
        "frontier_direct_input_tokens_estimate": frontier_input_tokens,
        "frontier_direct_output_tokens_estimate": frontier_output_tokens,
        "frontier_input_cost_per_mtok": args.frontier_input_cost_per_mtok,
        "frontier_output_cost_per_mtok": args.frontier_output_cost_per_mtok,
        "estimated_paid_cost_avoided": round(estimated_cost_avoided, 6),
        "failure_categories": failure_categories,
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
    if report["failure_categories"]:
        print("failure_breakdown:")
        for category, count in sorted(report["failure_categories"].items()):
            print(f"  {category}: {count}")
    if report["frontier_input_cost_per_mtok"] or report["frontier_output_cost_per_mtok"]:
        print(f"frontier_input_cost_per_mtok: {report['frontier_input_cost_per_mtok']}")
        print(f"frontier_output_cost_per_mtok: {report['frontier_output_cost_per_mtok']}")
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
