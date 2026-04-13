#!/usr/bin/env python3
"""Probe GPD MCP servers to discover exposed tools and assess domain value."""

import subprocess
import json
import time
import sys

servers = [
    ("gpd-conventions",  "gpd.mcp.servers.conventions_server"),
    ("gpd-patterns",     "gpd.mcp.servers.patterns_server"),
    ("gpd-protocols",    "gpd.mcp.servers.protocols_server"),
    ("gpd-verification", "gpd.mcp.servers.verification_server"),
    ("gpd-errors",       "gpd.mcp.servers.errors_mcp"),
    ("gpd-state",        "gpd.mcp.servers.state_server"),
    ("gpd-skills",       "gpd.mcp.servers.skills_server"),
]

INIT = json.dumps({
    "jsonrpc": "2.0", "id": 1, "method": "initialize",
    "params": {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "probe", "version": "0.1"},
    },
})
NOTIF = json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
LIST = json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})

python = "/Users/blu3/.gpd/venv/bin/python"
results = {}

for name, module in servers:
    proc = None
    try:
        proc = subprocess.Popen(
            [python, "-m", module],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        proc.stdin.write((INIT + "\n").encode())
        proc.stdin.flush()
        time.sleep(0.5)
        init_resp = proc.stdout.readline()

        proc.stdin.write((NOTIF + "\n").encode())
        proc.stdin.flush()
        proc.stdin.write((LIST + "\n").encode())
        proc.stdin.flush()
        time.sleep(0.5)
        list_resp = proc.stdout.readline()

        data = json.loads(list_resp)
        tools = data.get("result", {}).get("tools", [])
        results[name] = tools
        print(f"\n=== {name} ({len(tools)} tools) ===")
        for t in tools:
            desc = t.get("description", "")[:120]
            schema = json.dumps(
                t.get("inputSchema", {}).get("properties", {}), indent=None
            )
            print(f"  {t['name']}")
            print(f"    desc: {desc}")
            print(f"    params: {schema}")
    except Exception as e:
        print(f"\n=== {name} ERROR: {e} ===")
        results[name] = f"ERROR: {e}"
    finally:
        if proc:
            proc.terminate()

with open("/tmp/gpd-audit.json", "w") as f:
    json.dump(results, f, indent=2)
print(f"\nFull results saved to /tmp/gpd-audit.json")
