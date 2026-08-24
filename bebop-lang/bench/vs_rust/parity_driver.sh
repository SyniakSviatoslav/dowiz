#!/usr/bin/env bash
# Differential driver: interpreter (bebopc run) vs self-hosted compiled words
# (compilemany prewarm -> cached compilewords -> exec_words on .full).
# Deterministic, out-of-process; compiler loaded once per directory.
BEBOPC=./native/build/bebopc
EXECW=./native/build/exec_words
DIR=${1:-/tmp/opencode/pargen}
PASS=0; FAIL=0; SKIP=0; CRASH=0
$BEBOPC compilemany selfhost/expr_compile.bp "$DIR"/*.bp >/dev/null
for f in "$DIR"/*.bp; do
  I=$(timeout 30 $BEBOPC run "$f" main 2>/dev/null | tail -1)
  [ -z "$I" ] && { SKIP=$((SKIP+1)); continue; }
  if ! timeout 90 $BEBOPC compilewords selfhost/expr_compile.bp "$f" > /tmp/opencode/pd.full 2>/dev/null; then
    echo "COMPILEFAIL $f"; FAIL=$((FAIL+1)); continue
  fi
  N=$(timeout 30 $EXECW /tmp/opencode/pd.full 1 2>/dev/null | grep "^result=" | cut -d= -f2)
  if [ "$I" = "$N" ]; then PASS=$((PASS+1));
  elif [ -z "$N" ]; then echo "NATIVESTALL $f"; CRASH=$((CRASH+1));
  else echo "MISMATCH $f interp=$I native=$N"; FAIL=$((FAIL+1)); fi
done
echo "parity: pass=$PASS fail=$FAIL crash=$CRASH skip=$SKIP"
