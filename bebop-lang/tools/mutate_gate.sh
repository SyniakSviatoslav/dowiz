#!/usr/bin/env bash
# mutate_gate.sh (D11-E, 2026-09-04): one-token mutation per std gate — the first
# arithmetic operator (+ - *) on a NON-comment line inside the gate's OWN source
# (after its prelude expansion) is flipped (+ -> -, - -> +, * -> +); the mutated
# program is compiled with BEBOP_BIN and run; a gate whose frozen fold does NOT
# change is INSENSITIVE (its fold does not prove that operator). Prints one line
# per gate and a summary; exit 0 always (a report until the operator freezes a
# threshold). env: BEBOP_BIN, BEBOP_TMP.
cd "$(dirname "$0")/.." || exit 1
ulimit -s 65536 2>/dev/null
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}; T=${BEBOP_TMP:-/tmp/opencode}/mutate; mkdir -p "$T"
ins=0; sens=0; fail=0
awk '/std_tests\/[a-z0-9_]+\.bp/{match($0,/std_tests\/[a-z0-9_]+\.bp/);f=substr($0,RSTART,RLENGTH)} /^gate /{print $2, $3, f}' bench/vs_rust/std_golden.sh | while read -r g want f; do
  src=bench/vs_rust/$f; own=selfhost/std/$g.bp
  # prelude lines precede the gate's own text in the expansion
  total=$(wc -l < "$src"); ownl=$(wc -l < "$own"); skip=$(( total - ownl ))
  python3 - "$src" "$skip" "$T/$g.bp" <<'PY'
import sys, re
src, skip, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
L = open(src).read().split('\n'); done = False
for i in range(skip, len(L)):
    line = L[i]
    code = line.split('//')[0]
    m = re.search(r'(?<=[\w)\] ])\s([+\-*])\s(?=[\w(\[])', code)
    if m and not done:
        op = m.group(1); rep = {'+': '-', '-': '+', '*': '+'}[op]
        L[i] = code[:m.start(1)] + rep + code[m.end(1):] + (('//' + line.split('//', 1)[1]) if '//' in line else '')
        done = True
open(out, 'w').write('\n'.join(L))
print('mutated' if done else 'nomut')
PY
  ./seed/build/seed "$BEBOP_BIN" compile "$T/$g.bp" "$T/$g.bin" >/dev/null 2>&1 || { echo "$g COMPILEFAIL"; continue; }
  got=$(timeout 60 ./seed/build/seed "$T/$g.bin" 2>/dev/null | tail -1)
  if [ "$got" = "$want" ]; then echo "$g INSENSITIVE fold=$want"; else echo "$g sensitive"; fi
done | tee "$T/report.txt"
echo "mutation: $(grep -c ' sensitive' "$T/report.txt") sensitive, $(grep -c INSENSITIVE "$T/report.txt") insensitive, $(grep -c COMPILEFAIL "$T/report.txt") compilefail"
