#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# THE FIFTH READER (W-ES-P6, R-EMBED-STORE §4 T-ES-01). crates/bebop-wasm/gate.sh runs four readers of
# the store fixtures (bebop.bin, native, wasm32, oracle.py); this gate runs the fifth -- the C ABI of
# crates/bebop-ffi, driven from Python through ctypes (bindings/python/bebop_store.py) -- over the
# SAME fixtures and compares it with the SAME `.expected` files. Kept apart from gate.sh on purpose:
# gate.sh is owned by the wasm lane and ratchets bytes; this one only needs a native cdylib.
#
#   value fixtures  (kv, kv2, kv3, kv3max, kv3c, torn, kv0) : n= and root= must both match
#   refusal fixtures (ptaway, kv4stub, ptbad, empty)       : status= must match (the FFI keeps bebop-wasm's numbers)
#
# Every fixture is opened with Verify::Reachable (crc of every object the root reaches). A fixture
# the reader could not run is a FAILURE, never a skip. Green = 5/5 agree.
#
# --prove: flips one bit of a COPY of kv2.store inside a reachable object and expects a refusal or
# a moved root; a gate whose number cannot move measures nothing.
#
# Usage: tools/gates/ffi-readers.sh [--prove] [--no-build]
# Heavy: run it FOREGROUND under bebop-lang/tools/slot.sh on the box (never nest slot.sh inside).
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
[ -x "$CARGO" ] || CARGO=cargo
PROVE=0; BUILD=1
for a in "$@"; do
  case "$a" in
    --prove) PROVE=1 ;;
    --no-build) BUILD=0 ;;
    *) echo "ffi-readers: unknown argument $a" >&2; exit 2 ;;
  esac
done
if [ "$BUILD" = 1 ]; then
  (cd "$ROOT/crates/bebop-ffi" && "$CARGO" build --release --offline) || { echo "ffi-readers: RED build failed rc=$?" >&2; exit 1; }
fi
LIB="$ROOT/crates/bebop-ffi/target/release/libbebop_ffi.so"
[ -f "$LIB" ] || { echo "ffi-readers: RED no $LIB" >&2; exit 1; }
BEBOP_FFI_LIB="$LIB" PROVE="$PROVE" python3 -I - "$ROOT" <<'PY'
import os, sys
root = sys.argv[1]
sys.path.insert(0, os.path.join(root, "bindings", "python"))
import bebop_store as bs
fix = os.path.join(root, "crates", "bebop-wasm", "fixtures")

def expected(name):
    out = {}
    for line in open(os.path.join(fix, name + ".expected")):
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            k, v = line.split("=", 1)
            out[k] = int(v)
    return out

def read(data):
    """The fifth reader's answer: (status, n, root)."""
    try:
        with bs.Db.from_bytes(data, verify=bs.REACHABLE) as db, db.read() as r:
            return 0, len(r), r.fold_root()
    except bs.Refusal as e:
        return e.code, 0, 0

agree, bad = 0, []
FIXTURES = ["kv", "kv2", "kv3", "ptaway", "kv4stub", "kv3max", "kv3c", "torn", "ptbad", "empty", "kv0"]  # W-ES-P4 adds the last six
for name in FIXTURES:
    exp = expected(name)
    st, n, rt = read(open(os.path.join(fix, name + ".store"), "rb").read())
    if "status" in exp:
        ok = st == exp["status"]
        want = "status=%d" % exp["status"]
    else:
        ok = st == 0 and n == exp["n"] and rt == exp["root"]
        want = "status=0 n=%d root=%d" % (exp["n"], exp["root"])
    print("ffi %-8s status=%d n=%d root=%d  want %s  %s" % (name, st, n, rt, want, "AGREE" if ok else "DISAGREE"))
    if ok:
        agree += 1
    else:
        bad.append(name)
print("ffi-readers: %d/%d fixtures agree%s" % (agree, len(FIXTURES), "" if not bad else "; DISAGREE: " + " ".join(bad)))
rc = 0 if not bad else 1

if os.environ.get("PROVE") == "1":
    data = bytearray(open(os.path.join(fix, "kv2.store"), "rb").read())
    base = read(bytes(data))
    moved = 0
    # one bit in each of several late cells (entry payloads live near arena_used)
    for at in range(len(data) - 64, len(data) - 8 * 40, -8 * 7):
        d = bytearray(data); d[at] ^= 1
        if read(bytes(d)) != base:
            moved += 1
    print("ffi-readers --prove: %d flipped copies changed the answer (base %s)" % (moved, base))
    if moved == 0:
        print("ffi-readers: RED --prove: no flipped bit moved the answer; the gate measures nothing")
        rc = 1
sys.exit(rc)
PY
