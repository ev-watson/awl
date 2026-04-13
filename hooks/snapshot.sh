#!/usr/bin/env bash
# Create an APFS snapshot before offline agent runs.
set -euo pipefail
SNAP_NAME="claw-$(date +%Y%m%d-%H%M%S)"
tmutil localsnapshot 2>/dev/null && echo "APFS snapshot: $SNAP_NAME" || echo "snapshot skipped (not supported or SIP)"
