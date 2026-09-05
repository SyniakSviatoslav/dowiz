#!/usr/bin/env bash
# watch.sh (2026-09-06, operator: instant feedback in the editor): recompile a .bp every
# time it is saved and print `line:col: message` (T90) or `OK <words> words <ms> ms`.
# Usage: tools/watch.sh file.bp [more.bp ...]   (Ctrl-C stops; polls mtime, no inotify
# under proot). Editors: run it in a split, or `:make` = `tools/watch.sh --once %` with
# `errorformat=%f:%l:%c:\ %m` in Vim / a problemMatcher `^(.*):(\d+):(\d+): (.*)$` in VS Code.
cd "$(dirname "$0")/.." || exit 1
ulimit -s 65536 2>/dev/null
BIN=${BEBOP_BIN:-./bebop.bin}; T=${BEBOP_TMP:-/tmp/opencode}/watch; mkdir -p "$T"
ONCE=0; [ "$1" = --once ] && { ONCE=1; shift; }
one() { local f=$1 t0 err rc
  t0=$(date +%s%N); err=$(./seed/build/seed "$BIN" compile "$f" "$T/$(basename "$f" .bp).bin" 2>&1 >/dev/null); rc=$?
  if [ $rc = 0 ]; then echo "OK $f $(( $(stat -c %s "$T/$(basename "$f" .bp).bin") / 4 )) words $(( ($(date +%s%N) - t0) / 1000000 )) ms"
  else echo "$f:$(echo "$err" | tail -n 1)  [exit $rc]"; fi; }
declare -A last
for f in "$@"; do one "$f"; last[$f]=$(stat -c %Y "$f"); done
[ $ONCE = 1 ] && exit 0
while sleep 0.5; do for f in "$@"; do m=$(stat -c %Y "$f"); [ "$m" != "${last[$f]}" ] && { last[$f]=$m; one "$f"; }; done; done
