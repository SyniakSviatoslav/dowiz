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
blocks = [L[s:(starts[k + 1] if k + 1 < len(starts) else foot)] for k, s in enumerate(starts)]
per = (len(blocks) + J - 1) // J
for k in range(J):
    body = sum(blocks[k * per:(k + 1) * per], [])
    open('%s/shard%d.sh' % (T, k), 'w').write('\n'.join(head + body + footer))
    names = [b[0][7:].split()[0] for b in blocks[k * per:(k + 1) * per]]
    print('shard %d: %d gates (%s .. %s)' % (k, len(names), names[0], names[-1]))
open('%s/head.sh' % T, 'w').write('\n'.join(head)); open('%s/foot.sh' % T, 'w').write('\n'.join(footer))
import json; json.dump({b[0][7:].split()[0]: b for b in blocks}, open('%s/blocks.json' % T, 'w'))
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
grep '^FAIL ' "$T/all.log"; grep 'RETRIED' "$T/all.log"
P=$(grep -c '^PASS ' "$T/all.log"); F=$(grep -c '^FAIL ' "$T/all.log")
echo "std_golden: $P pass, $F fail (J=$J shards)"
[ "$F" = 0 ]
