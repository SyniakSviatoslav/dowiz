#!/usr/bin/env bash
# eval.sh (2026-09-06, operator: a micro-REPL): evaluate one expression or one file without
# hand-writing a program: `tools/eval.sh '1 << 40 | 5'`, `tools/eval.sh -f probe.bp` (a whole
# program), `tools/eval.sh -p helpers.bp 'fact(10)'` (helpers prepended). Compiles with
# $BEBOP_BIN AND runs tools/bpref.py in parallel, prints both values and DIFF when they
# disagree (the oracle is the semantics, the binary is the truth). ~0.3 s per expression.
cd "$(dirname "$0")/.." || exit 1
ulimit -s 65536 2>/dev/null
BIN=${BEBOP_BIN:-./bebop.bin}; T=${BEBOP_TMP:-/tmp/opencode}/eval.$$; mkdir -p "$T"; trap 'rm -rf "$T"' EXIT
PRE=""; while [ $# -gt 1 ]; do case "$1" in -p) PRE=$2; shift 2;; -f) cp "$2" "$T/p.bp"; shift 2;; *) break;; esac; done
[ -f "$T/p.bp" ] || { [ -n "$PRE" ] && cat "$PRE" > "$T/p.bp"; printf 'fn main() -> i64 {\n  %s\n}\n' "$1" >> "$T/p.bp"; }
( timeout 20 python3 tools/bpref.py "$T/p.bp" 2>&1 | tail -n 1 > "$T/ref" ) &
err=$(./seed/build/seed "$BIN" compile "$T/p.bp" "$T/p.bin" 2>&1 >/dev/null); rc=$?
[ $rc = 0 ] || { echo "compile exit $rc: $err"; wait; echo "bpref: $(cat "$T/ref")"; exit $rc; }
got=$(cd "$T" && timeout 10 /root/dowiz/bebop-lang/seed/build/seed "$T/p.bin" 2>&1 | tail -n 1); rc=$?
wait; ref=$(cat "$T/ref")
[ "$got" = "$ref" ] && echo "$got" || echo "bebop $got (exit $rc) | bpref $ref  DIFF"
