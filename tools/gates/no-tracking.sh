#!/bin/sh
# GATE-2 (W-MR0 2026-10-04): no guest behaviour leaves the device. The rules and their limits are
# in tools/gates/no-tracking.mjs; its proof is tools/gates/no-tracking.prove.sh.
set -u
cd "$(dirname "$0")/../.." || exit 2
exec node tools/gates/no-tracking.mjs "$(pwd)"
