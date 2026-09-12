#!/usr/bin/env bash
# cc.sh (operator 2026-09-12: "дозволена лише одна компіляція в процесі, ніколи не більше") --
# the slot-guarded compile. A one-off `./seed/build/seed <bin> compile <src> <out>` is the one
# heavy job that was NOT covered by the guards in chain.sh / battery.sh / std_par.sh, and with
# many lanes running it is the likeliest way two compilations end up on the box at once.
#
#   Usage:  tools/cc.sh <bin0> <src.bp> <out.bin>        (== seed <bin0> compile <src> <out>)
#           tools/cc.sh --raw <any command ...>          (any other heavy job, same one slot)
#
# It takes the SAME global flock as every other heavy job (/root/.cache/bebop/slots, absolute,
# so lane worktrees share it), waits under the Android phantom ceiling instead of refusing, and
# is a no-op wrapper when it is already inside a slot -- so nesting it is free.
set -u
cd "$(dirname "$0")/.." || exit 1
SLOT=tools/slot.sh; [ -f "$SLOT" ] || SLOT=/root/dowiz/bebop-lang/tools/slot.sh

if [ "${1:-}" = --raw ]; then
  shift; [ $# -gt 0 ] || { echo "cc: --raw needs a command" >&2; exit 2; }
  [ "${BEBOP_SLOT_HELD:-0}" = 1 ] && exec "$@"
  exec bash "$SLOT" "cc:raw" "$@"
fi

BIN0=${1:?usage: tools/cc.sh <bin0> <src.bp> <out.bin>}
SRC=${2:?src.bp}; OUT=${3:?out.bin}
[ -s "$BIN0" ] || { echo "cc: GUARD $BIN0 missing or empty (L12)" >&2; exit 1; }
[ -s "$SRC" ]  || { echo "cc: GUARD $SRC missing or empty" >&2; exit 1; }
mkdir -p "$(dirname "$OUT")"
SEED=${SEED:-./seed/build/seed}
[ -x "$SEED" ] || { echo "cc: GUARD $SEED not executable" >&2; exit 1; }

run() { "$SEED" "$BIN0" compile "$SRC" "$OUT"; local rc=$?
        [ $rc = 0 ] && [ -s "$OUT" ] || { echo "cc: FAILED rc=$rc out=$OUT" >&2; return 1; }
        echo "cc: $SRC -> $OUT  $(md5sum < "$OUT" | cut -c1-8)"; }

if [ "${BEBOP_SLOT_HELD:-0}" = 1 ]; then run; else
  export -f run 2>/dev/null || true
  bash "$SLOT" "cc:$(basename "$SRC")" bash -c '"$0" "$1" compile "$2" "$3"' "$SEED" "$BIN0" "$SRC" "$OUT" \
    && [ -s "$OUT" ] && echo "cc: $SRC -> $OUT  $(md5sum < "$OUT" | cut -c1-8)" \
    || { echo "cc: FAILED out=$OUT" >&2; exit 1; }
fi
