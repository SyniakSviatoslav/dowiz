#!/usr/bin/env python3
"""bench/tq_sqlite/tpch_sqlite.py -- B7 prep twin (docs/blueprints/B7-dsl-planner.md
section 6/7; ctypes + prepared statements per the section-8 floor rule, docs/
LANG-DB-DESIGN.md:400, mirroring bench/tq_sqlite/sbench_sqlite.py's opendb/exe/prep
helpers). Loads bench/tq_sqlite/gen_lineitem.py's CSV (same rows as bench/oracles/tpch.py)
into sqlite and runs the Q6/Q1 twins, printing "<phase> <us> <fold>" plus a VM_STEP count
via sqlite3_stmt_status (the section-8 VDBE-step floor). Folds must equal bench/oracles/
tpch.py's q6_fold/q1_fold exactly -- `--check` does that comparison in-process.

usage:
  tpch_sqlite.py load <csv> <db>
  tpch_sqlite.py q6 <db>
  tpch_sqlite.py q1 <db>
  tpch_sqlite.py --check <csv> <db>   # load + q6 + q1 + compare against bench/oracles/tpch.py
"""
import csv
import ctypes
import ctypes.util
import os
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'oracles'))
from tpch import gen_rows, q6_fold, q1_fold  # noqa: E402

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
L.sqlite3_stmt_status.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int]; L.sqlite3_stmt_status.restype = ctypes.c_int
VM_STEP = 4  # SQLITE_STMTSTATUS_VM_STEP (sqlite3.h: FULLSCAN_STEP=1, SORT=2, AUTOINDEX=3, VM_STEP=4)


def opendb(db):
    h = ctypes.c_void_p()
    assert L.sqlite3_open(db.encode(), ctypes.byref(h)) == 0
    exe(h, 'PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;')
    return h


def exe(h, sql):
    assert L.sqlite3_exec(h, sql.encode(), None, None, None) == 0, sql


def prep(h, sql):
    st = ctypes.c_void_p()
    assert L.sqlite3_prepare_v2(h, sql.encode(), -1, ctypes.byref(st), None) == 0, sql
    return st


def load(csv_path, db_path):
    for f in (db_path, db_path + '-wal', db_path + '-shm'):
        try: os.remove(f)
        except FileNotFoundError: pass
    h = opendb(db_path)
    exe(h, 'CREATE TABLE lineitem(shipdate INTEGER, discount INTEGER, quantity INTEGER, '
           'extendedprice INTEGER, returnflag INTEGER, linestatus INTEGER, tax INTEGER)')
    exe(h, 'BEGIN')
    ins = prep(h, 'INSERT INTO lineitem VALUES(?,?,?,?,?,?,?)')
    t0 = time.perf_counter()
    with open(csv_path) as f:
        r = csv.reader(f)
        next(r)  # header
        n = 0
        for row in r:
            for i, v in enumerate(row):
                L.sqlite3_bind_int64(ins, i + 1, int(v))
            L.sqlite3_step(ins); L.sqlite3_reset(ins)
            n += 1
    exe(h, 'COMMIT')
    exe(h, 'CREATE INDEX ix_ship ON lineitem(shipdate)')
    L.sqlite3_finalize(ins)
    L.sqlite3_close(h)
    print('load', int((time.perf_counter() - t0) * 1000), n)


def q6(db_path):
    h = opendb(db_path)
    q = prep(h, 'SELECT SUM(extendedprice * discount) FROM lineitem '
                'WHERE shipdate >= ? AND shipdate < ? AND discount BETWEEN ? AND ? AND quantity < ?')
    from tpch import Q6_LO, Q6_HI
    L.sqlite3_bind_int64(q, 1, Q6_LO); L.sqlite3_bind_int64(q, 2, Q6_HI)
    L.sqlite3_bind_int64(q, 3, 5); L.sqlite3_bind_int64(q, 4, 7); L.sqlite3_bind_int64(q, 5, 24)
    t0 = time.perf_counter()
    assert L.sqlite3_step(q) == 100  # SQLITE_ROW
    fold = L.sqlite3_column_int64(q, 0)
    us = (time.perf_counter() - t0) * 1e6
    steps = L.sqlite3_stmt_status(q, VM_STEP, 1)
    L.sqlite3_finalize(q); L.sqlite3_close(h)
    print('q6', round(us, 1), fold, 'vm_steps', steps)
    return fold


def q1(db_path):
    h = opendb(db_path)
    q = prep(h, 'SELECT returnflag, linestatus, COUNT(*), SUM(quantity), SUM(extendedprice), '
                'SUM(extendedprice * (100 - discount)) FROM lineitem WHERE shipdate <= ? '
                'GROUP BY returnflag, linestatus ORDER BY returnflag, linestatus')
    from tpch import Q1_CUTOFF
    from lag_common import combine
    L.sqlite3_bind_int64(q, 1, Q1_CUTOFF)
    t0 = time.perf_counter()
    vals = []
    rc = L.sqlite3_step(q)
    while rc == 100:
        for col in (2, 3, 4, 5):
            vals.append(L.sqlite3_column_int64(q, col))
        rc = L.sqlite3_step(q)
    us = (time.perf_counter() - t0) * 1e6
    steps = L.sqlite3_stmt_status(q, VM_STEP, 1)
    L.sqlite3_finalize(q); L.sqlite3_close(h)
    fold = combine(vals)
    print('q1', round(us, 1), fold, 'vm_steps', steps)
    return fold


def main():
    args = sys.argv[1:]
    if args[0] == '--check':
        csv_path, db_path = args[1], args[2]
        load(csv_path, db_path)
        got_q6 = q6(db_path)
        got_q1 = q1(db_path)
        rows = list(gen_rows())
        exp_q6, exp_q1 = q6_fold(rows), q1_fold(rows)
        ok6 = (got_q6 == exp_q6); ok1 = (got_q1 == exp_q1)
        print('CHECK q6', 'OK' if ok6 else 'MISMATCH', got_q6, exp_q6)
        print('CHECK q1', 'OK' if ok1 else 'MISMATCH', got_q1, exp_q1)
        sys.exit(0 if (ok6 and ok1) else 1)
    elif args[0] == 'load':
        load(args[1], args[2])
    elif args[0] == 'q6':
        q6(args[1])
    elif args[0] == 'q1':
        q1(args[1])
    else:
        print('usage: tpch_sqlite.py load|q6|q1|--check ...'); sys.exit(2)


if __name__ == '__main__':
    main()
