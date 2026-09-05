#!/usr/bin/env python3
"""G8 twin (T117): the same 1M-node / 10M-edge graph in sqlite 3 (C API via ctypes):
edge table e(a,b) with an index on a (both directions inserted), BFS from S sources with
WITH RECURSIVE (levels), and the neighbour fold. Prints: build_ms, bfs fold + ns/edge,
nbr fold + us/query. Usage: sgraph_sqlite.py [S]"""
import ctypes, ctypes.util, os, sys, time
M = (1 << 64) - 1; A = 6364136223846793005; C = 1442695040888963407
def lcg(x): return (x * A + C) & M
def s64(x): x &= M; return x - (1 << 64) if x >> 63 else x
N = 1000000; E = 5000000; S = int(sys.argv[1]) if len(sys.argv) > 1 else 3
L = ctypes.CDLL(ctypes.util.find_library('sqlite3'))
L.sqlite3_open.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.c_void_p)]
L.sqlite3_prepare_v2.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int, ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(ctypes.c_char_p)]
L.sqlite3_bind_int64.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int64]
L.sqlite3_step.argtypes = [ctypes.c_void_p]; L.sqlite3_reset.argtypes = [ctypes.c_void_p]
L.sqlite3_column_int64.argtypes = [ctypes.c_void_p, ctypes.c_int]; L.sqlite3_column_int64.restype = ctypes.c_int64
L.sqlite3_exec.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p]
DB = 'sgraph.sqlite'
for f in (DB, DB + '-journal'):
    try: os.remove(f)
    except FileNotFoundError: pass
db = ctypes.c_void_p(); assert L.sqlite3_open(DB.encode(), ctypes.byref(db)) == 0
def exe(sql): assert L.sqlite3_exec(db, sql.encode(), None, None, None) == 0, sql
def prep(sql):
    st = ctypes.c_void_p(); assert L.sqlite3_prepare_v2(db, sql.encode(), -1, ctypes.byref(st), None) == 0, sql; return st
exe('PRAGMA journal_mode=OFF; PRAGMA synchronous=OFF; PRAGMA cache_size=-262144')
t0 = time.perf_counter()
exe('CREATE TABLE e(a INTEGER, b INTEGER)'); exe('BEGIN'); ins = prep('INSERT INTO e VALUES(?,?)'); x = 4242
for i in range(E):
    x = lcg(x); a = (x >> 20) % N; x = lcg(x); b = (x >> 20) % N
    L.sqlite3_bind_int64(ins, 1, a); L.sqlite3_bind_int64(ins, 2, b); L.sqlite3_step(ins); L.sqlite3_reset(ins)
    L.sqlite3_bind_int64(ins, 1, b); L.sqlite3_bind_int64(ins, 2, a); L.sqlite3_step(ins); L.sqlite3_reset(ins)
exe('COMMIT'); exe('CREATE INDEX ia ON e(a, b)')
build_ms = int((time.perf_counter() - t0) * 1000)
# BFS levels: WITH RECURSIVE with a visited-set emulation is not expressible directly; the
# standard idiom computes min level per node via a recursive CTE bounded by depth, which
# revisits nodes -- so the twin runs level-synchronous BFS with one SQL query per level
# (frontier table join), the fastest correct formulation we found for sqlite.
x = 31337; acc = 0; t0 = time.perf_counter()
exe('CREATE TABLE lvl(v INTEGER PRIMARY KEY, l INTEGER); CREATE TABLE fr(v INTEGER PRIMARY KEY); CREATE TABLE nx(v INTEGER PRIMARY KEY)')
step = prep('INSERT OR IGNORE INTO nx SELECT e.b FROM fr JOIN e ON e.a = fr.v WHERE e.b NOT IN (SELECT v FROM lvl)')
cnt = prep('SELECT count(*) FROM nx'); tot = prep('SELECT count(*), sum(l) FROM lvl'); addl = prep('INSERT INTO lvl SELECT v, ? FROM nx')
for _ in range(S):
    x = lcg(x); s = (x >> 20) % N
    exe('DELETE FROM lvl; DELETE FROM fr; DELETE FROM nx'); exe('INSERT INTO lvl VALUES(%d, 0); INSERT INTO fr VALUES(%d)' % (s, s)); l = 0
    while True:
        l += 1; exe('DELETE FROM nx'); L.sqlite3_step(step); L.sqlite3_reset(step)
        L.sqlite3_step(cnt); c = L.sqlite3_column_int64(cnt, 0); L.sqlite3_reset(cnt)
        if c == 0: break
        L.sqlite3_bind_int64(addl, 1, l); L.sqlite3_step(addl); L.sqlite3_reset(addl)
        exe('DELETE FROM fr; INSERT INTO fr SELECT v FROM nx')
    L.sqlite3_step(tot); acc = (acc + L.sqlite3_column_int64(tot, 0) + L.sqlite3_column_int64(tot, 1)) & M; L.sqlite3_reset(tot)
bfs_s = time.perf_counter() - t0
exe('BEGIN'); q = prep('SELECT b FROM e WHERE a = ?'); x = 4711; acc2 = 0; t0 = time.perf_counter()
for _ in range(100000):
    x = lcg(x); v = (x >> 20) % N; L.sqlite3_bind_int64(q, 1, v); d = 0
    while L.sqlite3_step(q) == 100: acc2 = (acc2 + L.sqlite3_column_int64(q, 0)) & M; d += 1
    acc2 = (acc2 + d) & M; L.sqlite3_reset(q)
nbr_us = (time.perf_counter() - t0) * 1e6 / 100000; exe('COMMIT')
print(f'build_ms={build_ms} bfs_fold={s64(acc)} bfs_ns_per_edge={bfs_s * 1e9 / (2 * E * S):.1f} bfs_sources={S} nbr_fold={s64(acc2)} nbr_us={nbr_us:.2f} size={os.path.getsize(DB)}')
