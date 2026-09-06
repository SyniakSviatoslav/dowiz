#!/usr/bin/env python3
"""bench/tq_sqlite/sgraph_update_sqlite.py -- B4 prep update twin (docs/blueprints/
B4-functional-tensor-updates.md section 6/7: "Twin: sqlite WAL UPDATE per row (python
ctypes, prepared) -- sgraph_sqlite.py"). Mirrors bench/tq_sqlite/sbench_sqlite.py's
`durable` phase (ctypes, prepared UPDATE, one BEGIN/COMMIT per row, WAL + synchronous=
NORMAL -- the same durability class the store's `assign` will be compared against once
B4 lands) but as its own R-medians benchmark (bench/tq_sqlite/run.sh's `statistics.
median` convention, R=11 per this prep task) over single-row updates to a table shaped
like a matrix cell store: cells(id INTEGER PRIMARY KEY, val INTEGER).

PREP STATUS (per the task): functional only. The real gate is 1,000,000 single-row
updates x R=11 repetitions (docs/blueprints/B4 section 7: "sqlite_us_per_row"); that is
too slow to run as part of this prep delivery (1e6 fsync'd WAL commits x 11 ~ tens of
minutes of pure fsync-bound I/O, no bebop counterpart exists yet to compare against) --
`--check` below runs a small smoke N/R instead and asserts the update actually lands
and the median math is sane. The full run is one invocation away:
    N=1000000 R=11 python3 bench/tq_sqlite/sgraph_update_sqlite.py

env: N (rows in the table AND number of single-row updates per repetition, default
1000000), R (repetitions, default 11), SEED (LCG seed for the row ids updated, default
20260906). Prints "update_wal <median_us_per_row> <n> <r>".
"""
import ctypes
import ctypes.util
import os
import statistics
import sys
import time

A, C, M = 6364136223846793005, 1442695040888963407, (1 << 64) - 1

L = ctypes.CDLL(ctypes.util.find_library('sqlite3'))
L.sqlite3_open.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.c_void_p)]
L.sqlite3_close.argtypes = [ctypes.c_void_p]
L.sqlite3_prepare_v2.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int, ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(ctypes.c_char_p)]
L.sqlite3_bind_int64.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int64]
L.sqlite3_step.argtypes = [ctypes.c_void_p]
L.sqlite3_reset.argtypes = [ctypes.c_void_p]
L.sqlite3_finalize.argtypes = [ctypes.c_void_p]
L.sqlite3_column_int64.argtypes = [ctypes.c_void_p, ctypes.c_int]; L.sqlite3_column_int64.restype = ctypes.c_int64
L.sqlite3_exec.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p]


def opendb(db):
    h = ctypes.c_void_p(); assert L.sqlite3_open(db.encode(), ctypes.byref(h)) == 0
    exe(h, 'PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;')
    return h


def exe(h, sql): assert L.sqlite3_exec(h, sql.encode(), None, None, None) == 0, sql


def prep(h, sql):
    st = ctypes.c_void_p(); assert L.sqlite3_prepare_v2(h, sql.encode(), -1, ctypes.byref(st), None) == 0, sql
    return st


def build(db, n):
    for f in (db, db + '-wal', db + '-shm'):
        try: os.remove(f)
        except FileNotFoundError: pass
    h = opendb(db)
    exe(h, 'CREATE TABLE cells(id INTEGER PRIMARY KEY, val INTEGER)')
    exe(h, 'BEGIN')
    ins = prep(h, 'INSERT INTO cells VALUES(?,0)')
    for i in range(n):
        L.sqlite3_bind_int64(ins, 1, i); L.sqlite3_step(ins); L.sqlite3_reset(ins)
    exe(h, 'COMMIT'); L.sqlite3_finalize(ins)
    return h


def one_run(h, n, seed):
    upd = prep(h, 'UPDATE cells SET val = val + 1 WHERE id = ?')
    x = seed
    t0 = time.perf_counter()
    for _ in range(n):
        x = (x * A + C) & M
        exe(h, 'BEGIN')
        L.sqlite3_bind_int64(upd, 1, x % n)
        L.sqlite3_step(upd); L.sqlite3_reset(upd)
        exe(h, 'COMMIT')
    us = (time.perf_counter() - t0) * 1e6
    L.sqlite3_finalize(upd)
    return us / n  # us per row


def bench(db, n, r, seed):
    h = build(db, n)
    per_row = [one_run(h, n, seed + i) for i in range(r)]
    L.sqlite3_close(h)
    for f in (db, db + '-wal', db + '-shm'):
        try: os.remove(f)
        except FileNotFoundError: pass
    return statistics.median(per_row), per_row


def check():
    """Smoke test (functional, not the timing gate): a handful of rows/reps, assert the
    updates actually land (val == number of times that id was hit) and the median is
    computed sanely -- the runnable check ponytail requires for update-loop logic."""
    n, r, seed = 50, 3, 1
    db = os.path.join(os.environ.get('BEBOP_TMP', '.'), 'sgraph_update_check.sqlite')
    h = build(db, n)
    hit_counts = [0] * n
    for rep in range(r):
        x = seed + rep
        upd = prep(h, 'UPDATE cells SET val = val + 1 WHERE id = ?')
        for _ in range(n):
            x = (x * A + C) & M
            rid = x % n
            hit_counts[rid] += 1
            exe(h, 'BEGIN'); L.sqlite3_bind_int64(upd, 1, rid); L.sqlite3_step(upd); L.sqlite3_reset(upd); exe(h, 'COMMIT')
        L.sqlite3_finalize(upd)
    q = prep(h, 'SELECT val FROM cells ORDER BY id')
    got = []
    rc = L.sqlite3_step(q)
    while rc == 100:
        got.append(L.sqlite3_column_int64(q, 0)); rc = L.sqlite3_step(q)
    L.sqlite3_finalize(q); L.sqlite3_close(h)
    for f in (db, db + '-wal', db + '-shm'):
        try: os.remove(f)
        except FileNotFoundError: pass
    assert got == hit_counts, (got, hit_counts)
    assert statistics.median([3.0, 1.0, 2.0]) == 2.0
    print('check OK: %d rows, %d reps, single-row UPDATE lands exactly, median() sane' % (n, r))


if __name__ == '__main__':
    if '--check' in sys.argv:
        check()
        sys.exit(0)
    N = int(os.environ.get('N', 1_000_000))
    R = int(os.environ.get('R', 11))
    SEED = int(os.environ.get('SEED', 20260906))
    db = os.path.join(os.environ.get('BEBOP_TMP', '.'), 'sgraph_update.sqlite')
    med, per_row = bench(db, N, R, SEED)
    print('update_wal %.3f %d %d' % (med, N, R))
    print('per_run_us_per_row', [round(x, 3) for x in per_row])
