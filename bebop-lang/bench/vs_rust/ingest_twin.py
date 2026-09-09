#!/usr/bin/env python3
"""Scaled ingest twin (A7 step 3): bebop cells vs python vs sqlite vs rust.
Prints ms, MB/s, maxrss KB, fold per row. Folds must agree."""
import os, resource, sqlite3, subprocess, sys, time

DATA = sys.argv[1] if len(sys.argv) > 1 else '/tmp/opencode/w2-a7/ingest.txt'
BBIN = sys.argv[2] if len(sys.argv) > 2 else '/tmp/opencode/w2-a7/ingest.bin'
N = os.path.getsize(DATA)

def run(cmd):
    t = time.perf_counter()
    r = subprocess.run(cmd, capture_output=True, text=True)
    dt = (time.perf_counter() - t) * 1000
    rss = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    return r.stdout.strip().split('\n')[-1], dt, rss, r.returncode

def py_row():
    t = time.perf_counter()
    lines = sum_ = 0
    for ln in open(DATA):
        p = ln.split(',')
        lines += 1; sum_ += int(p[0])
    dt = (time.perf_counter() - t) * 1000
    return lines * 1000003 + sum_, dt

s = resource.getpagesize()
print('file bytes: %d' % N)
out, ms, rss, rc = run(['./seed/build/seed', BBIN, DATA])
print('bebop-cells fold=%s ms=%.0f ms/MB=%.1f maxrss=%dKB rc=%d' % (out, ms, ms / (N / 1e6), rss, rc))
fold, ms = py_row()
print('python fold=%d ms=%.0f ms/MB=%.1f' % (fold, ms, ms / (N / 1e6)))

t = time.perf_counter()
con = sqlite3.connect(':memory:')
con.execute('create table t(id integer, u integer, v integer, cell integer, label text)')
rows = [tuple([int(x) for x in ln.split(',')[:4]] + [ln.split(',')[4].strip()]) for ln in open(DATA)]
con.executemany('insert into t values (?,?,?,?,?)', rows)
con.commit()
f2 = con.execute('select count(*), sum(id) from t').fetchone()
ms = (time.perf_counter() - t) * 1000
print('sqlite fold=%d ms=%.0f ms/MB=%.1f' % (f2[0] * 1000003 + f2[1], ms, ms / (N / 1e6)))

for rb in ('/tmp/opencode/w2-a7/ingest_best', '/tmp/opencode/w2-a7/ingest_common'):
    if os.path.exists(rb):
        out, ms, rss, rc = run([rb, DATA])
        print('%s fold=%s ms=%.0f ms/MB=%.1f maxrss=%dKB rc=%d' % (os.path.basename(rb), out, ms, ms / (N / 1e6), rss, rc))
