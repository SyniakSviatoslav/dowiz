#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# verify-file's proof: the gate is triggered before it is trusted. Every case runs the REAL
# tools/gates/verify-file.sh from a SCRATCH ROOT (a copy of tools/verify, the fixtures and
# examples/read.mjs -- gate.sh resolves its root from its own path, so nothing real is touched):
#   1. clean copy                                         -> rc 0, GREEN
#   2. kv3.expected's root moved by one                   -> rc 1, "kv3: ... DISAGREE"
#   3. doc.expected's doc_verify says broken=2            -> rc 1, "doc: ... DISAGREE"
#   4. bit 0 of a kv3max value cell flipped IN THE FIXTURE -> rc 1 (both readers say 10, .expected says 0)
#   5. tools/verify/py/50_doc.py gains a comment line     -> rc 1, "STALE" (the committed assembly no longer matches)
# No compiler; seconds. Exit 0 when every case answered as wanted, 1 otherwise.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
S=$(mktemp -d "${TMPDIR:-/tmp}/verify-file-prove.XXXXXX")
trap 'rm -rf "$S"' EXIT
fail=0
fresh() {
  rm -rf "$S/r"; mkdir -p "$S/r/tools/gates" "$S/r/crates/bebop-wasm/fixtures" "$S/r/examples"
  cp -r "$REPO/tools/verify" "$S/r/tools/verify"
  cp "$REPO/tools/gates/verify-file.sh" "$S/r/tools/gates/"
  cp "$REPO/crates/bebop-wasm/fixtures/"*.store "$REPO/crates/bebop-wasm/fixtures/"*.expected "$S/r/crates/bebop-wasm/fixtures/"
  cp "$REPO/examples/read.mjs" "$S/r/examples/"
}
want() { # want <rc> <label> <grep>
  sh "$S/r/tools/gates/verify-file.sh" > "$S/out" 2>&1; rc=$?
  hit=$(grep -m1 -- "$3" "$S/out" | cut -c1-120)
  echo "verify-file.prove: $2 -> rc=$rc (want $1): ${hit:-$(tail -1 "$S/out" | cut -c1-120)}"
  [ "$rc" -eq "$1" ] && [ -n "$hit" ] || { tail -5 "$S/out"; fail=1; }
}
F="$S/r/crates/bebop-wasm/fixtures"
fresh; want 0 "clean scratch" "verify-file: GREEN"
fresh; sed -i 's/^root=-4092201869538170098$/root=-4092201869538170097/' "$F/kv3.expected"; want 1 "kv3.expected root moved by one" "kv3: .*DISAGREE"
fresh; sed -i 's/^doc_verify=\(.*\)broken=0/doc_verify=\1broken=2/; s/^doc_verify=\(.*\)broken=3/doc_verify=\1broken=2/' "$F/doc.expected"; want 1 "doc_verify says broken=2" "doc: .*DISAGREE"
fresh; python3 -I - "$F/kv3max.store" <<'PY'
import struct, sys
b = bytearray(open(sys.argv[1], "rb").read())
c = lambda i: struct.unpack_from("<q", b, 8 * i)[0]
sb = max((at for at in (0, 512) if c(at) == 3554557610294396226), key=lambda at: c(at + 2))
pt = c(sb + 3); root = c(pt + 18); rec = root + c(root + 2 + 6)
b[8 * (rec + 2 + 4 + (c(rec + 4) + 7) // 8)] ^= 1
open(sys.argv[1], "wb").write(b)
PY
want 1 "kv3max fixture bit flipped" "kv3max: .*DISAGREE"
fresh; echo "# a comment the assembly does not have" >> "$S/r/tools/verify/py/50_doc.py"; want 1 "a python part edited, assembly not rerun" "STALE"
[ "$fail" = 0 ] && echo "verify-file.prove: GREEN (5 cases: one pass, four named reds)" || echo "verify-file.prove: RED"
exit $fail
