#!/usr/bin/env python3
"""G7 twin (T116): the sbench phases on sqlite 3 through the C API (ctypes, prepared
statements, file-backed db, WAL off, synchronous=OFF -> the same 'process-crash only'
durability class as the store). Same data (LCG 12345), same query ids (LCG 777 / 999 /
555), same folds. Prints one line per phase: name ns_or_ms fold (lookup/scan in ns per op)."""
import ctypes, ctypes.util, os, sys, time
M = (1 << 64) - 1; A = 6364136223846793005; C = 1442695040888963407
def lcg(x): return (x * A + C) & M
def s64(x): x &= M; return x - (1 << 64) if x >> 63 else x
def cell(u, v): return ((u + (1 << 30)) >> 21) * 1024 + ((v + (1 << 30)) >> 21)
L = ctypes.CDLL(ctypes.util.find_library('sqlite3'))
L.sqlite3_open.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.c_void_p)]
L.sqlite3_close.argtypes = [ctypes.c_void_p]
L.sqlite3_prepare_v2.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int, ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(ctypes.c_char_p)]
L.sqlite3_bind_int64.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int64]
L.sqlite3_step.argtypes = [ctypes.c_void_p]; L.sqlite3_reset.argtypes = [ctypes.c_void_p]
L.sqlite3_column_int64.argtypes = [ctypes.c_void_p, ctypes.c_int]; L.sqlite3_column_int64.restype = ctypes.c_int64
L.sqlite3_exec.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p]
DB = 'sbench.sqlite'
def opendb():
    db = ctypes.c_void_p(); assert L.sqlite3_open(DB.encode(), ctypes.byref(db)) == 0
    L.sqlite3_exec(db, b'PRAGMA journal_mode=OFF; PRAGMA synchronous=OFF; PRAGMA locking_mode=EXCLUSIVE; PRAGMA cache_size=-262144;', None, None, None); return db
def exe(db, sql): assert L.sqlite3_exec(db, sql.encode(), None, None, None) == 0, sql
def prep(db, sql):
    st = ctypes.c_void_p(); assert L.sqlite3_prepare_v2(db, sql.encode(), -1, ctypes.byref(st), None) == 0, sql; return st
ph = sys.argv[1]
if ph == 'insert':
    for f in (DB, DB + '-journal'):
        try: os.remove(f)
        except FileNotFoundError: pass
    db = opendb(); t0 = time.perf_counter()
    exe(db, 'CREATE TABLE p(id INTEGER PRIMARY KEY, u INTEGER, v INTEGER, cell INTEGER)'); exe(db, 'BEGIN')
    ins = prep(db, 'INSERT INTO p VALUES(?,?,?,?)'); x = 12345
    for i in range(1000000):
        x = lcg(x); u = (x >> 33) - (1 << 30); x = lcg(x); v = (x >> 33) - (1 << 30)
        L.sqlite3_bind_int64(ins, 1, i); L.sqlite3_bind_int64(ins, 2, u); L.sqlite3_bind_int64(ins, 3, v); L.sqlite3_bind_int64(ins, 4, cell(u, v))
        L.sqlite3_step(ins); L.sqlite3_reset(ins)
    exe(db, 'COMMIT'); exe(db, 'CREATE INDEX ic ON p(cell)')
    print('insert', int((time.perf_counter() - t0) * 1000), 0); L.sqlite3_close(db)
elif ph == 'lookup':
    db = opendb(); exe(db, 'BEGIN'); q = prep(db, 'SELECT u FROM p WHERE id=?'); x = 777; acc = 0; t0 = time.perf_counter()
    for _ in range(100000):
        x = lcg(x); L.sqlite3_bind_int64(q, 1, (x >> 20) % 1000000)
        assert L.sqlite3_step(q) == 100; acc = (acc + L.sqlite3_column_int64(q, 0)) & M; L.sqlite3_reset(q)
    print('lookup', round((time.perf_counter() - t0) * 1e9 / 100000, 1), s64(acc)); exe(db, 'COMMIT'); L.sqlite3_close(db)
elif ph == 'scan':
    db = opendb(); exe(db, 'BEGIN'); q = prep(db, 'SELECT id FROM p WHERE cell IN (?,?,?,?,?,?,?,?,?)'); x = 999; acc = 0; t0 = time.perf_counter()
    for _ in range(10000):
        x = lcg(x); cx = (x >> 30) % 1024; x = lcg(x); cy = (x >> 30) % 1024; k = 1
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                ok = 0 <= cx + dx < 1024 and 0 <= cy + dy < 1024
                L.sqlite3_bind_int64(q, k, (cx + dx) * 1024 + (cy + dy) if ok else -1); k += 1
        while L.sqlite3_step(q) == 100: acc = (acc + L.sqlite3_column_int64(q, 0)) & M
        L.sqlite3_reset(q)
    print('scan', round((time.perf_counter() - t0) * 1e9 / 10000, 1), s64(acc)); exe(db, 'COMMIT'); L.sqlite3_close(db)
elif ph == 'update':
    db = opendb(); q = prep(db, 'UPDATE p SET u=u+1 WHERE id=?'); x = 555; t0 = time.perf_counter(); exe(db, 'BEGIN')
    for _ in range(100000):
        x = lcg(x); L.sqlite3_bind_int64(q, 1, (x >> 20) % 1000000); L.sqlite3_step(q); L.sqlite3_reset(q)
    exe(db, 'COMMIT'); print('update', int((time.perf_counter() - t0) * 1000), 0); L.sqlite3_close(db)
elif ph == 'reopen':
    t0 = time.perf_counter()
    for _ in range(100):
        db = opendb(); q = prep(db, 'SELECT u FROM p WHERE id=1'); L.sqlite3_step(q); L.sqlite3_column_int64(q, 0); L.sqlite3_close(db)
    print('reopen', round((time.perf_counter() - t0) * 1e6 / 100, 1), 0)
elif ph == 'compact':
    db = opendb(); t0 = time.perf_counter(); exe(db, 'VACUUM'); print('compact', int((time.perf_counter() - t0) * 1000), 0); L.sqlite3_close(db)
elif ph == 'durable':
    db = opendb(); exe(db, 'PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL'); q = prep(db, 'UPDATE p SET u=u+1 WHERE id=?'); t0 = time.perf_counter()
    for k in range(1000):
        exe(db, 'BEGIN'); L.sqlite3_bind_int64(q, 1, k); L.sqlite3_step(q); L.sqlite3_reset(q); exe(db, 'COMMIT')
    print('durable', round((time.perf_counter() - t0) * 1e6 / 1000, 1), 0); L.sqlite3_close(db)
elif ph == 'size':
    print('size', os.path.getsize(DB), 0)
