#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# atk3 (W-MG-HOST, holes 2 and 10 of W-MG-ATK3): a hostile archive against the host, from a clean
# directory, no repo, no npm, the environment emptied. Four checks, each its own RED:
#   1. the PIN: `const PINNED` in examples/read.mjs = WASM_SHA256 of tools/verify/ein-verify.mjs
#      (tools/gates/atk3_pin.py writes it; a reader rebuilt without it is RED here);
#   2. the WORDS: read.mjs's table = crates/einfile-reader/src/status.rs TABLE, word for word;
#   3. the embedded wasm hashes to WASM_SHA256 (the reader the honest row runs IS the shipped one);
#   4. every row of tools/gates/atk3_bootstrap.py. An OPEN row is another owner's, printed, not red.
# Fewer rows than ROWS measures less than it says: REFUSED. Proof: tools/gates/atk3.prove.sh.
# No compiler. Exit 0 GREEN; 1 RED (the cause is the line above); 2 REFUSED.
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ROWS=29
say() { echo "atk3: $*"; }
S="$(mktemp -d "${TMPDIR:-/tmp}/atk3.XXXXXX")" || exit 2
trap 'rm -rf "$S"' EXIT
python3 -I - "$ROOT" "$S/reader.wasm" <<'PY' || { say "RED pin / words / digest"; exit 1; }
import base64, hashlib, re, sys
root, out = sys.argv[1:]
t = open(root + "/tools/verify/ein-verify.mjs").read()
r = open(root + "/examples/read.mjs").read()
sha = re.search(r'^const WASM_SHA256 = "([0-9a-f]{64})";$', t, re.M).group(1)
pin = re.search(r'^const PINNED = "([0-9a-f]{64})";$', r, re.M).group(1)
if sha != pin:
    sys.exit("atk3: RED read.mjs pins %s, the shipped reader is %s: run tools/gates/atk3_pin.py" % (pin, sha))
w = base64.b64decode(re.search(r'^const WASM_B64 = "([^"]*)";$', t, re.M).group(1))
if hashlib.sha256(w).hexdigest() != sha:
    sys.exit("atk3: RED the wasm in ein-verify.mjs does not hash to its WASM_SHA256")
open(out, "wb").write(w)
s = open(root + "/crates/einfile-reader/src/status.rs").read()
names = dict(re.findall(r"pub const (\w+): i32 = (\d+);", s))
table = {names[n]: x for n, x in re.findall(r'^\s*\((\w+), "(.*)"\),$', s, re.M)}
words = dict(re.findall(r'^  (\d+): "(.*)",$', r, re.M))
if not table or words != table:
    sys.exit("atk3: RED read.mjs WORDS is not status.rs TABLE: %r" % sorted(k for k in set(table) | set(words) if table.get(k) != words.get(k)))
print("atk3: pin %s = the shipped reader; %d words = status.rs; read.mjs %d lines" % (pin, len(words), r.count("\n")))
PY
mkdir "$S/clean" "$S/boot"
cp "$ROOT/examples/read.mjs" "$ROOT/tools/verify/ein-verify.mjs" "$ROOT/tools/verify/ein-verify.py" "$S/clean/" || exit 2
env -i PATH="$PATH" ATK3_READ="$S/clean/read.mjs" ATK3_MJS="$S/clean/ein-verify.mjs" ATK3_PY="$S/clean/ein-verify.py" \
  python3 "$ROOT/tools/gates/atk3_bootstrap.py" "$S/reader.wasm" "$S/boot" > "$S/out" 2>&1; rc=$?
cut -c1-150 "$S/out" | grep -E '^(HOLE|OPEN)|atk3-bootstrap:|Traceback|Error'
n=$(grep -cE '^(agree|HOLE|OPEN) ' "$S/out")
if [ "$n" -lt "$ROWS" ]; then say "REFUSED $n rows, fewer than $ROWS (rc=$rc): the harness measured less than it says"; [ "$rc" = 0 ] && exit 2; fi
if [ "$rc" != 0 ]; then say "RED rc=$rc"; exit 1; fi
say "GREEN $n rows from a clean dir, no HOLE"
