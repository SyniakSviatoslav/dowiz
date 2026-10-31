#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# verify-file: the one-file readers under run-all (W-MG-M5 hand-back: tools/verify/gate.sh was
# reached by nobody -- run-all walks tools/gates/*.sh only). No compiler here: the COMMITTED
# tools/verify/bebop-verify.mjs (the MIT reader wasm embedded) and bebop-verify.py (stdlib) are
# copied to a clean directory and read every conformance fixture, the log fixture and the DOC
# fixture, each answer held to its `.expected`. Two preconditions are refused by name first:
# bebop-verify.py must be the assembly of tools/verify/py/*.py, and bebop-verify.mjs must exist.
# The build, the byte ratchet and the archive kill-test (read.mjs) are gate.sh's with the build,
# under slot.sh: `bash bebop-lang/tools/slot.sh verify sh tools/verify/gate.sh --prove`.
# Its proof: tools/gates/verify-file.prove.sh. Exit 0 = GREEN; 1 = RED (the cause is the line above).
set -u
cd "$(dirname "$0")/../.." || exit 2
OUT="${TMPDIR:-/tmp}/verify-file.$$"
sh tools/verify/gate.sh --no-build > "$OUT" 2>&1; rc=$?
cat "$OUT"
last=$(grep '^verify-gate: ' "$OUT" | tail -1 | sed 's/^verify-gate: //')
rm -f "$OUT"
if [ "$rc" = 0 ]; then echo "verify-file: $last"; else echo "verify-file: RED rc=$rc -- $last"; fi
exit $rc
