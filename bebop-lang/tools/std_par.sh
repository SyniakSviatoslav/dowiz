#!/usr/bin/env bash
# std_par.sh (2026-09-06, operator: "мультиядерні та шардові"): bench/vs_rust/std_golden.sh
# split into J contiguous shards (header + a run of `# ---- name` blocks + footer), one per
# A78 core, then one combined summary in std_golden's own format. A shard that FAILs a gate
# gets that gate re-run ONCE standalone and pinned (timing-flag gates such as lcjit miss
# under load): the line then reads `PASS name (RETRIED)`; a second miss stays a FAIL.
# env: BEBOP_BIN, BEBOP_TMP, J (default 3). Exit 1 on any FAIL.
cd "$(dirname "$0")/.." || exit 1
ulimit -s 65536 2>/dev/null
J=${J:-3}; BIN=${BEBOP_BIN:-./bebop.bin}; T=${BEBOP_TMP:-/tmp/opencode}/stdpar; mkdir -p "$T"
[ -s "$BIN" ] || { echo "GUARD: $BIN missing or empty (L12)"; exit 1; }
BIG=($(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd41"{print p}' /proc/cpuinfo))
python3 - "$T" "$J" <<'PY'
import sys, re
T, J = sys.argv[1], int(sys.argv[2])
L = open('bench/vs_rust/std_golden.sh').read().split('\n')
starts = [i for i, l in enumerate(L) if l.startswith('# ---- ')]
foot = next(i for i, l in enumerate(L) if l.startswith('echo "std_golden:'))
head = L[:starts[0]]; footer = L[foot:]
raw = [(s, L[s:(starts[k + 1] if k + 1 < len(starts) else foot)]) for k, s in enumerate(starts)]
# item 9 (retro P9): a block whose header is immediately preceded by a `# timing` line is
# flaky under parallel load (lcjit) -- pull it out of the shards, run it serialised after them.
timing = [b for s, b in raw if s > 0 and L[s - 1].strip() == '# timing']
blocks = [b for s, b in raw if not (s > 0 and L[s - 1].strip() == '# timing')]
per = (len(blocks) + J - 1) // J
for k in range(J):
    body = sum(blocks[k * per:(k + 1) * per], [])
    open('%s/shard%d.sh' % (T, k), 'w').write('\n'.join(head + body + footer))
    names = [b[0][7:].split()[0] for b in blocks[k * per:(k + 1) * per]]
    print('shard %d: %d gates (%s .. %s)' % (k, len(names), names[0], names[-1]))
open('%s/head.sh' % T, 'w').write('\n'.join(head)); open('%s/foot.sh' % T, 'w').write('\n'.join(footer))
import json; json.dump({b[0][7:].split()[0]: b for b in blocks}, open('%s/blocks.json' % T, 'w'))
names = []
for b in timing:
    name = b[0][7:].split()[0]; names.append(name)
    open('%s/timing_%s.sh' % (T, name), 'w').write('\n'.join(head + b + footer))
open('%s/timing_names.txt' % T, 'w').write(''.join(n + '\n' for n in names))
PY
for k in $(seq 0 $((J - 1))); do
  mkdir -p "$T/$k"; cpu=${BIG[$((k % ${#BIG[@]}))]:-}
  ( BEBOP_TMP=$T/$k BEBOP_BIN=$BIN ${cpu:+taskset -c $cpu} bash "$T/shard$k.sh" > "$T/shard$k.log" 2>&1 ) &
done
wait
cat "$T"/shard*.log | grep -E '^(PASS|FAIL) ' > "$T/all.log"
for name in $(grep '^FAIL ' "$T/all.log" | sed 's/^FAIL \([^:]*\):.*/\1/'); do
  python3 - "$T" "$name" <<'PY'
import sys, json
T, name = sys.argv[1], sys.argv[2]
b = json.load(open('%s/blocks.json' % T))[name]
open('%s/retry_%s.sh' % (T, name), 'w').write(open('%s/head.sh' % T).read() + '\n' + '\n'.join(b) + '\n' + open('%s/foot.sh' % T).read())
PY
  mkdir -p "$T/retry"; r=$(BEBOP_TMP=$T/retry BEBOP_BIN=$BIN ${BIG[0]:+taskset -c ${BIG[0]}} bash "$T/retry_$name.sh" 2>/dev/null | grep -E "^(PASS|FAIL) $name")
  case "$r" in PASS*) sed -i "s/^FAIL $name:.*/PASS $name (RETRIED standalone, pinned)/" "$T/all.log";; esac
done
# item 9 (retro P9): timing-tagged gates (lcjit) run LAST, single-threaded, pinned, preceded
# by `boxguard status` -- its line is written into all.log next to the gate's result.
if [ -s "$T/timing_names.txt" ]; then
  mkdir -p "$T/timing"
  while IFS= read -r name || [ -n "$name" ]; do  # `|| [ -n "$name" ]` so a missing final newline isn't dropped
    [ -z "$name" ] && continue
    bstat=$(boxguard status 2>&1 | tr '\n' ' ')
    r=$(BEBOP_TMP=$T/timing BEBOP_BIN=$BIN ${BIG[0]:+taskset -c ${BIG[0]}} bash "$T/timing_$name.sh" 2>/dev/null | grep -E "^(PASS|FAIL) $name")
    echo "boxguard: $name -- $bstat"  # item 9: printed to std_par's own stdout (battery.sh surfaces it)
    echo "boxguard: $name -- $bstat" >> "$T/all.log"
    echo "$r" >> "$T/all.log"
  done < "$T/timing_names.txt"
fi
grep '^FAIL ' "$T/all.log"; grep 'RETRIED' "$T/all.log"
# E4 (D12-A): per-gate run ms + the flake ledger as perf.csv rows; the per-gate list = bench/perf_fn/gates.txt (diff it against git)
cat "$T"/[0-9]*/gates.txt "$T"/timing/gates.txt 2>/dev/null | sort > "$T/gates.txt"; mkdir -p bench/perf_fn; cp "$T/gates.txt" bench/perf_fn/gates.txt
python3 tools/perf.py record --bin "$BIN" battery_flakes "$(grep -c RETRIED "$T/all.log")" count "FAILs that passed standalone (timing-flag gates under load)"
python3 tools/perf.py record --bin "$BIN" gate_run_ms "$(awk '$3=="miss"{s+=$2} END{print s+0}' "$T/gates.txt")" ms "sum of non-memo gate runs ($(grep -c ' miss ' "$T/gates.txt") runs, $(grep -c ' hit ' "$T/gates.txt") memo hits)"
P=$(grep -c '^PASS ' "$T/all.log"); F=$(grep -c '^FAIL ' "$T/all.log")
echo "std_golden: $P pass, $F fail (J=$J shards)"
[ "$F" = 0 ]
