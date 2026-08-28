#!/usr/bin/env bash
# M5 std-twin golden gate: every selfhost/std twin runs on a fixed input and
# its fingerprint must equal the C golden (native/src/*.c, printed by
# bench/vs_rust/c/std_golden). Both engines are verified: the interpreter
# (bebopc run) and the JIT (exec_words on the compiled words, correct entry).
# Values are compared mod 2^64 (C prints u64, Bebop returns signed i64).
set -u
BEBOPC=./native/build/bebopc
EXECW=./native/build/exec_words
CDIR=bench/vs_rust/c
TDIR=bench/vs_rust/std_tests
SCR=/tmp/opencode/std_scr.bp
PASS=0; FAIL=0

# C goldens (u64 fingerprints; base64 prints 3 packed RFC-4648 vectors).
( cd "$CDIR" && gcc -O2 -I ../../../native/src -o std_golden std_golden.c \
    ../../../native/src/sort.c ../../../native/src/checksum.c \
    ../../../native/src/rng.c ../../../native/src/sha256.c -lm )
$CDIR/std_golden > /tmp/opencode/std_golden.txt
# extra SHA-256 boundary goldens (verified against hashlib in the commit):
#  "abc" is in std_golden; append empty/55/56/64/112B vectors
python3 - <<'PY' >> /tmp/opencode/std_golden.txt
import hashlib
def fold(b):
    d = hashlib.sha256(b).digest()
    acc = 0
    for i in range(8):
        w = int.from_bytes(d[i*4:i*4+4], 'big')
        acc = (acc*31 + w) % (1<<64)
    return acc
print("sha256_empty", fold(b""))
print("sha256_55", fold(bytes(range(55))))
print("sha256_56", fold(bytes(range(56))))
print("sha256_64", fold(bytes(range(64))))
print("sha256_112", fold(bytes(range(112))))
PY

norm() { python3 -c "import sys; v=int(sys.argv[1]); print(v % (1<<64))" "$1"; }

gate() {
  local name="$1" golden="$2" interp="$3" jit="$4"
  local ok=1
  [ "$(norm "$golden")" = "$(norm "$interp")" ] || { ok=0; echo "FAIL $name interp: golden=$golden got=$interp"; }
  [ "$(norm "$golden")" = "$(norm "$jit")" ] || { ok=0; echo "FAIL $name jit: golden=$golden got=$jit"; }
  if [ $ok = 1 ]; then echo "PASS $name ($golden)"; PASS=$((PASS+1)); else FAIL=$((FAIL+1)); fi
}

run_one() {
  local f="$1"
  cp "$f" "$SCR"
  if ! timeout 900 $BEBOPC compilewords selfhost/expr_compile.bp "$SCR" > /tmp/opencode/std.full 2>/tmp/opencode/std.err; then
    echo "COMPILEFAIL $f"; FAIL=$((FAIL+1)); return
  fi
  local idx entry iv jv
  idx=$(awk '/^fn /{ if (/^fn main\(/) exit; c++ } END{ print c+0 }' "$SCR")
  entry=$(grep "^OFF" /tmp/opencode/std.full | awk -v i="$idx" '{ print $(i+3) }')
  iv=$(timeout 900 $BEBOPC run "$SCR" main 2>/dev/null | tail -1)
  jv=$(timeout 120 $EXECW /tmp/opencode/std.full 1 "$entry" 2>/dev/null | grep "^result=" | cut -d= -f2)
  echo "$iv $jv"
}

# ---- checksum ----
r=$(run_one "$TDIR/checksum.bp"); iv=${r% *}; jv=${r#* }
g=$(awk '{print $2}' /tmp/opencode/std_golden.txt | sed -n '1p')
gate checksum "$g" "$iv" "$jv"

# ---- sort ----
r=$(run_one "$TDIR/sort.bp"); iv=${r% *}; jv=${r#* }
g=$(sed -n '2p' /tmp/opencode/std_golden.txt | awk '{print $2}')
gate sort "$g" "$iv" "$jv"

# ---- rng ----
r=$(run_one "$TDIR/rng.bp"); iv=${r% *}; jv=${r#* }
g=$(sed -n '3p' /tmp/opencode/std_golden.txt | awk '{print $2}')
gate rng "$g" "$iv" "$jv"

# ---- base64 (RFC 4648, packed 4-char words) ----
r=$(run_one "$TDIR/base64.bp"); iv=${r% *}; jv=${r#* }
g=$(python3 -c "
line = open('/tmp/opencode/std_golden.txt').read().split('\n')[4].split()
man = int(line[1])
fox = ord('Z')*16777216 + ord('m')*65536 + ord('9')*256 + ord('4')
print(man * 1000000000 + fox)")
gate base64 "$g" "$iv" "$jv"

# ---- sha256 ----
r=$(run_one "$TDIR/sha256.bp"); iv=${r% *}; jv=${r#* }
g=$(sed -n '4p' /tmp/opencode/std_golden.txt | awk '{print $2}')
gate sha256 "$g" "$iv" "$jv"

# ---- crc32 (zlib_crc32 check value 0xCBF43926 for "123456789") ----
r=$(run_one "$TDIR/crc.bp"); iv=${r% *}; jv=${r#* }
gate crc32 3421780262 "$iv" "$jv"

# ---- hex (hex_encode of AB CD EF -> packed ASCII "abcdef") ----
r=$(run_one "$TDIR/hex.bp"); iv=${r% *}; jv=${r#* }
gate hex 107075202213222 "$iv" "$jv"

echo "std_golden: $PASS pass, $FAIL fail"
[ "$FAIL" = 0 ]
