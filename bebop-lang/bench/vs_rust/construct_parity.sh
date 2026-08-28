#!/usr/bin/env bash
# B2-8 construct word-parity gate: for every construct file, compile with the
# interpreter (compilewords) and with the JIT-compiled compiler (self-hosted
# words executing under exec_words), then compare the emitted word streams
# byte-for-byte. The compiled compiler is built ONCE; its boot re-reads one
# fixed scratch path per construct.
set -u
BEBOPC=./native/build/bebopc
EXECW=./native/build/exec_words
DIR=${1:-bench/parity_constructs}
SCR=/tmp/opencode/b28_src.bp
PASS=0; FAIL=0

./tools/gen_selfsrc.sh /tmp/opencode/b28_selfsrc.bp "$SCR" >/dev/null
if ! timeout 300 $BEBOPC compilewords selfhost/expr_compile.bp /tmp/opencode/b28_selfsrc.bp > /tmp/opencode/b28.full 2>/dev/null; then
  echo "compiler build failed"; exit 1
fi
python3 - <<'PY'
import struct
raw = open('/tmp/opencode/b28.full').read().split()
n = int(raw[0]); words = [int(x) for x in raw[1:1+n]]
open('/tmp/opencode/b28.bin','wb').write(struct.pack('<%dI'%n,*words))
PY

for f in "$DIR"/*.bp; do
  cp "$f" "$SCR"
  # interp reference: compilewords (words) + run main (value)
  if ! timeout 90 $BEBOPC compilewords selfhost/expr_compile.bp "$f" > /tmp/opencode/b28_ref.full 2>/dev/null; then
    echo "COMPILEFAIL $f"; FAIL=$((FAIL+1)); continue
  fi
  IVAL=$(timeout 30 $BEBOPC run "$f" main 2>/dev/null | tail -1)
  # JIT words via the compiled compiler boot + arena dump
  if ! ARENA_DUMP=/tmp/opencode/b28_arena.bin timeout 60 $EXECW /tmp/opencode/b28.bin 1 0 >/dev/null 2>&1; then
    echo "NATIVESTALL $f"; FAIL=$((FAIL+1)); continue
  fi
  # native value: run the compiled words of the construct itself,
  # entering at fn main (idx of ^fn main( among ^fn lines)
  IDX=$(awk '/^fn /{ if (/^fn main\(/) exit; c++ } END{ print c+0 }' "$f")
  ENTRY=$(grep "^OFF" /tmp/opencode/b28_ref.full | awk -v i="$IDX" '{ print $(i+3) }')
  NVAL=$(timeout 30 $EXECW /tmp/opencode/b28_ref.full 1 "$ENTRY" 2>/dev/null | grep "^result=" | cut -d= -f2)
  verdict=$(python3 - "$f" "$IVAL" "$NVAL" <<'PY'
import struct, sys
name, ival, nval = sys.argv[1], sys.argv[2], sys.argv[3]
raw = open('/tmp/opencode/b28_ref.full').read().split()
n = int(raw[0]); ref = [int(x) for x in raw[1:1+n]]
data = open('/tmp/opencode/b28_arena.bin','rb').read()
words = struct.unpack('<%dQ' % (len(data)//8), data)
wok = any(tuple(words[i:i+n]) == tuple(ref) for i in range(len(words)-n))
vok = (ival == nval) and (nval != '')
if wok and vok:
    print(f"MATCH {name} ({n} words, value {ival})")
else:
    print(f"MISMATCH {name} words={wok} value={ival}!={nval}")
PY
)
  echo "$verdict"
  case "$verdict" in MATCH*) PASS=$((PASS+1));; *) FAIL=$((FAIL+1));; esac
done
echo "construct word+value parity: pass=$PASS fail=$FAIL"
