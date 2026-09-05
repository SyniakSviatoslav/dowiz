#!/usr/bin/env bash
# mutate_gate.sh (D11-E, 2026-09-04; T123 rewrite 2026-09-06): operator-mutation
# sensitivity of every std gate. The gate's EXPANDED program (the `<out>.use` file the
# compiler writes when the gate uses modules, else the source) is mutated at up to K=3
# arithmetic-operator sites (+ - *) on non-comment lines, preferring sites in the gate's
# own text (after the last `//e "` line of the expansion, i.e. after the modules); each
# mutant is compiled and run; a gate is INSENSITIVE only if NO mutant changes its fold
# (its fold then proves none of its first three operators). Prints one line per gate
# and a summary; exit 0 always. env: BEBOP_BIN, BEBOP_TMP, K (sites, default 3).
cd "$(dirname "$0")/.." || exit 1
ulimit -s 65536 2>/dev/null
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}; T=${BEBOP_TMP:-/tmp/opencode}/mutate; K=${K:-3}; mkdir -p "$T"
awk '/std_tests\/[a-z0-9_]+\.bp/{match($0,/std_tests\/[a-z0-9_]+\.bp/);f=substr($0,RSTART,RLENGTH)} /^gate /{print $2, $3, f}' bench/vs_rust/std_golden.sh | while read -r g want f; do
  src=bench/vs_rust/$f
  rm -f "$T/${g}_base.bin" "$T/${g}_base.bin.use" "$T/${g}_base.bin.becache"
  ./seed/build/seed "$BEBOP_BIN" compile "$src" "$T/${g}_base.bin" >/dev/null 2>&1 || { echo "$g COMPILEFAIL base"; continue; }
  exp="$T/${g}_base.bin.use"; [ -f "$exp" ] || exp="$src"
  n=$(python3 - "$exp" "$T" "$g" "$K" <<'PY'
import sys, re
exp, T, g, K = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
L = open(exp).read().split('\n')
own = 0
for i, line in enumerate(L):
    if line.startswith('//e "'): own = i + 1
sites = []
for i in list(range(own, len(L))) + list(range(0, own)):
    code = L[i].split('//')[0]
    for m in re.finditer(r'(?<=[\w)\] ])\s([+\-*])\s(?=[\w(\[])', code):
        sites.append((i, m.start(1), m.group(1)))
        if len(sites) >= K: break
    if len(sites) >= K: break
for k, (i, pos, op) in enumerate(sites):
    M = list(L); code = M[i].split('//')[0]; rest = ('//' + M[i].split('//', 1)[1]) if '//' in M[i] else ''
    M[i] = code[:pos] + {'+': '-', '-': '+', '*': '+'}[op] + code[pos + 1:] + rest
    open('%s/%s_m%d.bp' % (T, g, k), 'w').write('\n'.join(M))
print(len(sites))
PY
)
  changed=0; tried=0; cf=0
  for k in $(seq 0 $((n - 1))); do
    tried=$((tried + 1))
    ./seed/build/seed "$BEBOP_BIN" compile "$T/${g}_m$k.bp" "$T/${g}_m$k.bin" >/dev/null 2>&1 || { cf=$((cf + 1)); changed=$((changed + 1)); continue; }
    got=$(timeout 120 ./seed/build/seed "$T/${g}_m$k.bin" 2>/dev/null | tail -1)
    [ "$got" = "$want" ] || changed=$((changed + 1))
  done
  if [ "$n" = 0 ]; then echo "$g INSENSITIVE fold=$want (no operator site)"; elif [ "$changed" = 0 ]; then echo "$g INSENSITIVE fold=$want ($tried sites)"; else echo "$g sensitive ($changed/$tried mutants changed the fold, $cf failed to compile)"; fi
done | tee "$T/report.txt"
echo "mutation: $(grep -c ' sensitive' "$T/report.txt") sensitive, $(grep -c INSENSITIVE "$T/report.txt") insensitive"
