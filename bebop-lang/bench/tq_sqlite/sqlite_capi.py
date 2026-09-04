#!/usr/bin/env python3
"""T100 row (b'): sqlite through the C API (ctypes, prepared statement, no python
per-row wrapper): same data/queries as oracle.py, indexed 3x3 window query.
Prints indexed_capi_us_per_query and the window fold (must equal oracle's
window_fold_Q). Usage: sqlite_capi.py [N] [Q]."""
import ctypes, ctypes.util, sys, time
N = int(sys.argv[1]) if len(sys.argv) > 1 else 1000000
Q = int(sys.argv[2]) if len(sys.argv) > 2 else 1000
M = (1 << 64) - 1; A = 6364136223846793005; C = 1442695040888963407
def gen(n, x=12345):
    out = []
    for _ in range(n):
        x = (x * A + C) & M; u = (x >> 33) - (1 << 30)
        x = (x * A + C) & M; v = (x >> 33) - (1 << 30)
        out.append((u, v))
    return out, x
def cell(u, v): return ((u + (1 << 30)) >> 21) * 1024 + ((v + (1 << 30)) >> 21)
pts, x = gen(N); qs, _ = gen(Q, x)
L = ctypes.CDLL(ctypes.util.find_library('sqlite3'))
L.sqlite3_open.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.c_void_p)]
L.sqlite3_prepare_v2.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int, ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(ctypes.c_char_p)]
L.sqlite3_bind_int64.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int64]
L.sqlite3_step.argtypes = [ctypes.c_void_p]; L.sqlite3_reset.argtypes = [ctypes.c_void_p]
L.sqlite3_column_int64.argtypes = [ctypes.c_void_p, ctypes.c_int]; L.sqlite3_column_int64.restype = ctypes.c_int64
L.sqlite3_exec.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p]
db = ctypes.c_void_p(); assert L.sqlite3_open(b':memory:', ctypes.byref(db)) == 0
def exe(sql): assert L.sqlite3_exec(db, sql.encode(), None, None, None) == 0, sql
def prep(sql):
    st = ctypes.c_void_p(); assert L.sqlite3_prepare_v2(db, sql.encode(), -1, ctypes.byref(st), None) == 0, sql; return st
exe('CREATE TABLE p(id INTEGER PRIMARY KEY, u INTEGER, v INTEGER, cell INTEGER)')
exe('BEGIN'); ins = prep('INSERT INTO p VALUES(?,?,?,?)')
for i, (u, v) in enumerate(pts):
    L.sqlite3_bind_int64(ins, 1, i); L.sqlite3_bind_int64(ins, 2, u); L.sqlite3_bind_int64(ins, 3, v); L.sqlite3_bind_int64(ins, 4, cell(u, v))
    L.sqlite3_step(ins); L.sqlite3_reset(ins)
exe('COMMIT'); exe('CREATE INDEX ic ON p(cell)')
# 9 bound cells (out-of-grid neighbours are bound to -1, which no row has); the ORDER BY tie-break = lowest id
q = prep('SELECT id FROM p WHERE cell IN (?,?,?,?,?,?,?,?,?) ORDER BY (u-?)*(u-?)+(v-?)*(v-?), id LIMIT 1')
fold = 0; p = 1
t0 = time.perf_counter()
for qu, qv in qs:
    c = cell(qu, qv); cx, cy = c // 1024, c % 1024; k = 1
    for dx in (-1, 0, 1):
        for dy in (-1, 0, 1):
            ok = 0 <= cx + dx < 1024 and 0 <= cy + dy < 1024
            L.sqlite3_bind_int64(q, k, (cx + dx) * 1024 + (cy + dy) if ok else -1); k += 1
    L.sqlite3_bind_int64(q, 10, qu); L.sqlite3_bind_int64(q, 11, qu); L.sqlite3_bind_int64(q, 12, qv); L.sqlite3_bind_int64(q, 13, qv)
    r = L.sqlite3_column_int64(q, 0) if L.sqlite3_step(q) == 100 else -1
    L.sqlite3_reset(q)
    fold = (fold + (r + 1000000007) * p) % 1000000007; p = (p * 131) % 1000000007
us = (time.perf_counter() - t0) * 1e6 / Q
print(f'sqlite C-API indexed_capi_us_per_query={us:.1f} window_fold_Q{Q}={fold} N={N} Q={Q}')
