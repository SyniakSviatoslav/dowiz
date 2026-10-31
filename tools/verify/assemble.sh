#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# Assemble tools/verify/bebop-verify.py (ONE stdlib-only file, the deliverable) from the parts in
# tools/verify/py/*.py, in name order. The parts exist so each source file stays under 300 lines
# (DOWIZ-COMMON-RULES rule 8); the assembled file is what a stranger downloads and what gate.sh
# copies to a clean directory. No compiler, no cargo: a shell `cat`.
#
#   sh tools/verify/assemble.sh            write bebop-verify.py (says UNCHANGED or REWRITTEN)
#   sh tools/verify/assemble.sh --check    exit 1 when the committed file is not the assembly
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
V="$ROOT/tools/verify"
OUT="$V/bebop-verify.py"
TMP="${TMPDIR:-/tmp}/bebop-verify.assemble.$$"
cat "$V"/py/[0-9][0-9]_*.py > "$TMP" || { echo "verify-assemble: RED cannot read the parts" >&2; rm -f "$TMP"; exit 1; }
python3 -I -c "import ast,sys; ast.parse(open(sys.argv[1]).read())" "$TMP" || { echo "verify-assemble: RED the assembly does not parse"; rm -f "$TMP"; exit 1; }
if [ "${1:-}" = "--check" ]; then
  if cmp -s "$TMP" "$OUT"; then echo "verify-assemble: bebop-verify.py is the assembly of $(ls "$V"/py/*.py | wc -l | tr -d ' ') parts"; rm -f "$TMP"; exit 0; fi
  echo "verify-assemble: RED tools/verify/bebop-verify.py is STALE -- run sh tools/verify/assemble.sh and commit it"; rm -f "$TMP"; exit 1
fi
if cmp -s "$TMP" "$OUT"; then echo "verify-assemble: UNCHANGED ($(wc -l < "$OUT" | tr -d ' ') lines)"; rm -f "$TMP"; exit 0; fi
mv "$TMP" "$OUT" && chmod +x "$OUT" && echo "verify-assemble: REWRITTEN tools/verify/bebop-verify.py ($(wc -l < "$OUT" | tr -d ' ') lines) -- commit it"
