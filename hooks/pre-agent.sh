#!/usr/bin/env bash
# Snapshot the working directory before the agent modifies files.
set -euo pipefail
WORKDIR="${1:-.}"
cd "$WORKDIR"
if [ -d .git ]; then
    git add -A && git commit -m "pre-agent snapshot $(date +%Y%m%d-%H%M%S)" --allow-empty -q 2>/dev/null || true
fi
