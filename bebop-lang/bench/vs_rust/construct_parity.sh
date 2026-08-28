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
  if ! timeout 90 $BEBOPC compilewords selfhost/expr_compile.bp "$f" > /tmp/opencode/b28_ref.full 2>/dev/null; then
    echo "COMPILEFAIL $f"; FAIL=$((FAIL+1)); continue
  fi
  if ! ARENA_DUMP=/tmp/opencode/b28_arena.bin timeout 60 $EXECW /tmp/opencode/b28.bin 1 0 >/dev/null 2>&1; then
    echo "NATIVESTALL $f"; FAIL=$((FAIL+1)); continue
  fi
  verdict=$(python3 - "$f" <<'PY'
import struct, sys
raw = open('/tmp/opencode/b28_ref.full').read().split()
n = int(raw[0]); ref = [int(x) for x in raw[1:1+n]]
data = open('/tmp/opencode/b28_arena.bin','rb').read()
words = struct.unpack('<%dQ' % (len(data)//8), data)
for i in range(len(words)-n):
    if tuple(words[i:i+n]) == tuple(ref):
        print(f"MATCH {sys.argv[1]} ({n} words)")
        raise SystemExit(0)
print(f"MISMATCH {sys.argv[1]} ref={n}")
PY
)
  echo "$verdict"
  case "$verdict" in MATCH*) PASS=$((PASS+1));; *) FAIL=$((FAIL+1));; esac
done
echo "construct word-parity: pass=$PASS fail=$FAIL"
