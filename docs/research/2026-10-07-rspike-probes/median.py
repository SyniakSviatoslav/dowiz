#!/usr/bin/env python3 -I
# medians over the RES lines of run.sh's raw output; checks every result value is the same across runs
import sys, statistics, collections
raw = open(sys.argv[1]).read().splitlines()
ms = collections.defaultdict(list); vals = collections.defaultdict(set); reps = {}
for l in raw:
    if l.startswith("RES "):
        _, name, t, n, v = l.split(); ms[name].append(float(t) / int(n)); vals[name].add(v); reps[name] = int(n)
    elif l.startswith("MISMATCH") or l.startswith("RC ") or l.startswith("COMPILEFAIL") or l.startswith("OBJDUMP"):
        print(l)
print(f"{'name':34} {'runs':>4} {'median ms/rep':>14} {'min':>10} {'max':>10}  result")
for name in ms:
    xs = sorted(ms[name])
    print(f"{name:34} {len(xs):4d} {statistics.median(xs):14.6f} {xs[0]:10.6f} {xs[-1]:10.6f}  {'/'.join(sorted(vals[name]))}")
