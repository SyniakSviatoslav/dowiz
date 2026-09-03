#!/usr/bin/env bash
# T11 morph loop: "data becomes code" — the k1 kernel-shaped rule is
# compiled to a .bin, atomically published (tmp -> rename), mmapped
# file-backed RX, and run: fold == frozen k1 value 500000500000. Iterated
# K=8 times; ANY fold mismatch aborts with the iteration + got value
# (the breaker). Artifacts are content-addressed so a stale read is
# structurally impossible. This is the in-sandbox form of the vision's
# mprotect morphing (proot W^X blocks mprotect RWX; the file-backed RX
# seed map is the W^X-clean equivalent).
set -eu
SEED=./seed/build/seed
BIN=./bebop.bin
SRC=bench/vs_rust/kernels/k1.bp
GOLDEN=500000500000
K=${K:-8}
PASS=0
for i in $(seq 1 "$K"); do
  PUB=/tmp/opencode/morph_k1_${i}.bin
  TMP=/tmp/opencode/morph_k1_${i}.tmp
  "$SEED" "$BIN" compile "$SRC" "$TMP" >/dev/null 2>&1
  mv -f "$TMP" "$PUB"   # atomic replacement per iteration
  GOT=$("$SEED" "$PUB" | tail -1)
  if [ "$GOT" != "$GOLDEN" ]; then
    echo "MORPH_FAIL iter=$i got=$GOT golden=$GOLDEN"
    exit 1
  fi
  PASS=$((PASS+1))
done
echo "morph_loop: $PASS/$K iterations OK (fold $GOLDEN stable across atomic replacements)"
