#!/usr/bin/env bash
# Differential driver: interpreter (bebopc run) vs self-hosted compiled words
# (bebopc compilewords -> exec_words). Deterministic, out-of-process.
BEBOPC=./native/build/bebopc
EXECW=./native/build/exec_words
DIR=${1:-/tmp/opencode/pargen}
PASS=0; FAIL=0; SKIP=0; CRASH=0
for f in "$DIR"/*.bp; do
  I=$(timeout 30 $BEBOPC run "$f" main 2>/dev/null | tail -1)
  [ -z "$I" ] && { SKIP=$((SKIP+1)); continue; }
  if ! timeout 90 $BEBOPC compilewords selfhost/expr_compile.bp "$f" > /tmp/opencode/pd.full 2>/dev/null; then
    echo "COMPILEFAIL $f"; FAIL=$((FAIL+1)); continue
  fi
  grep -v "^OFF" /tmp/opencode/pd.full > /tmp/opencode/pd.w
  grep "^OFF" /tmp/opencode/pd.full > /tmp/opencode/pd.off
  N=$(timeout 30 $EXECW /tmp/opencode/pd.w 1 /tmp/opencode/pd.off 2>/dev/null | grep "^result=" | cut -d= -f2)
  if [ "$I" = "$N" ]; then PASS=$((PASS+1));
  elif [ -z "$N" ]; then echo "NATIVESTALL $f"; CRASH=$((CRASH+1));
  else echo "MISMATCH $f interp=$I native=$N"; FAIL=$((FAIL+1)); fi
done
echo "parity: pass=$PASS fail=$FAIL crash=$CRASH skip=$SKIP"
