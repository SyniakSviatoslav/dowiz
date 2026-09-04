#!/usr/bin/env bash
# prec_switch.sh — T42(a)/(b) decision D5: run bpref.py over every std_golden
# gate three times (current grammar, BPREF_CPREC=1, BPREF_ASR=1) and report
# which gates would change fold under C precedence / arithmetic `>>`.
# A gate whose CURRENT bpref fold != frozen fold is a bpref limitation (SKIP),
# not a grammar delta. env: TO (per-run timeout, default 600s).
set -u
cd "$(dirname "$0")/.."
TO=${TO:-600}
printf '%-12s %-22s %-8s %-8s %-8s\n' gate frozen cur cprec asr
# gate name -> source file: the compile line that precedes each gate line
awk '/std_tests\/[a-z0-9_]+\.bp/{match($0,/std_tests\/[a-z0-9_]+\.bp/);f=substr($0,RSTART,RLENGTH)} /^gate /{print $2, $3, f}' bench/vs_rust/std_golden.sh | while read -r g want f; do
  f=bench/vs_rust/$f
  run() { timeout "$TO" env "$@" python3 tools/bpref.py "$f" 2>/dev/null | tail -1; }
  cur=$(run X=0); cp=$(run BPREF_CPREC=1); as=$(run BPREF_ASR=1)
  st() { [ "$1" = "$want" ] && echo same || echo "DIFF:${1:-none}"; }
  if [ "$cur" != "$want" ]; then printf '%-12s %-22s SKIP(cur=%s)\n' "$g" "$want" "${cur:-none}"; continue; fi
  printf '%-12s %-22s %-8s %-8s %-8s\n' "$g" "$want" same "$(st "$cp")" "$(st "$as")"
done
