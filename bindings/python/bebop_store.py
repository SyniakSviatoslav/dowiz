# SPDX-License-Identifier: MIT OR Apache-2.0
"""bebop_store: the bebop KV database from Python, through bebop-ffi's C ABI (ctypes, no build step).

    from bebop_store import Db
    with Db.create("shop.bebop") as db:
        with db.write() as w:
            w.put(b"order/1", b"new")        # committed when the block ends
    with Db.open("shop.bebop") as db, db.read() as r:
        r.get(b"order/1")                     # b"new"

Every refusal raises `Refusal` with `.code` (the C status), `.name` and the library's message.
The shared library is found at $BEBOP_FFI_LIB, else at crates/bebop-ffi/target/release (then debug)
relative to this repo.
"""

import ctypes as C
import os

_HERE = os.path.dirname(os.path.abspath(__file__))
_REPO = os.path.dirname(os.path.dirname(_HERE))

NONE, DATA, FULL = 0, 1, 2  # durability
ANCHOR, REACHABLE, WHOLE_ARENA = 0, 1, 2  # verify depth
OPEN_CREATE = 1


def _find_lib():
    env = os.environ.get("BEBOP_FFI_LIB")
    if env:
        return env
    for prof in ("release", "debug"):
        for name in ("libbebop_ffi.so", "libbebop_ffi.dylib", "bebop_ffi.dll"):
            p = os.path.join(_REPO, "crates", "bebop-ffi", "target", prof, name)
            if os.path.exists(p):
                return p
    raise OSError("libbebop_ffi not found: build crates/bebop-ffi (cargo build --release) or set BEBOP_FFI_LIB")


_lib = C.CDLL(_find_lib())
_P = C.c_void_p
_U8P = C.POINTER(C.c_uint8)


def _sig(name, args, ret=C.c_int32):
    f = getattr(_lib, name)
    f.argtypes = args
    f.restype = ret
    return f


_sig("bebop_abi_version", [])
_sig("bebop_status_name", [C.c_int32], C.c_char_p)
_sig("bebop_last_error", [C.c_char_p, C.c_size_t], C.c_size_t)
_sig("bebop_free", [_P, C.c_size_t], None)
_sig("bebop_db_open", [C.c_char_p, C.c_uint32, C.c_int32, C.POINTER(_P)])
_sig("bebop_db_open_bytes", [C.c_char_p, C.c_size_t, C.c_int32, C.POINTER(_P)])
_sig("bebop_db_create_mem", [C.POINTER(_P)])
_sig("bebop_db_close", [_P])
_sig("bebop_db_flush", [_P, C.c_int32, C.POINTER(C.c_int64)])
_sig("bebop_db_durable", [_P, C.POINTER(C.c_int64)])
_sig("bebop_db_verify", [_P, C.c_int32, C.POINTER(C.c_int64)])
_sig("bebop_db_compact", [_P, C.POINTER(C.c_int64)])
_sig("bebop_db_snapshot", [_P, C.POINTER(_P), C.POINTER(C.c_size_t)])
_sig("bebop_db_poisoned", [_P])
_sig("bebop_read_begin", [_P, C.POINTER(_P)])
_sig("bebop_read_end", [_P])
_sig("bebop_read_get", [_P, C.c_char_p, C.c_size_t, C.POINTER(C.c_int32), C.POINTER(_P), C.POINTER(C.c_size_t)])
_sig("bebop_read_len", [_P, C.POINTER(C.c_uint64)])
_sig("bebop_read_generation", [_P, C.POINTER(C.c_int64)])
_sig("bebop_read_fold_root", [_P, C.POINTER(C.c_int64)])
_sig("bebop_read_scan", [_P, C.c_char_p, C.c_size_t, C.c_char_p, C.c_size_t, C.POINTER(_P)])
_sig("bebop_read_prefix", [_P, C.c_char_p, C.c_size_t, C.POINTER(_P)])
_sig("bebop_scan_next", [_P, C.POINTER(C.c_int32), C.POINTER(_P), C.POINTER(C.c_size_t), C.POINTER(_P), C.POINTER(C.c_size_t)])
_sig("bebop_scan_free", [_P], None)
_sig("bebop_write_begin", [_P, C.POINTER(_P)])
_sig("bebop_write_put", [_P, C.c_char_p, C.c_size_t, C.c_char_p, C.c_size_t])
_sig("bebop_write_remove", [_P, C.c_char_p, C.c_size_t, C.POINTER(C.c_int32)])
_sig("bebop_write_get", [_P, C.c_char_p, C.c_size_t, C.POINTER(C.c_int32), C.POINTER(_P), C.POINTER(C.c_size_t)])
_sig("bebop_write_commit", [_P, C.c_int32, C.POINTER(C.c_int64)])
_sig("bebop_write_stage", [_P, C.POINTER(C.c_int64)])
_sig("bebop_write_abort", [_P], None)

ABI_VERSION = 1
if _lib.bebop_abi_version() != ABI_VERSION:
    raise OSError("bebop-ffi ABI %d, this module speaks %d" % (_lib.bebop_abi_version(), ABI_VERSION))


class Refusal(Exception):
    """The library said no. `.code` is the C status, `.name` its name, `.message` the reason."""

    def __init__(self, code, message):
        self.code = code
        self.name = _lib.bebop_status_name(code).decode()
        self.message = message
        super().__init__("%s (%d): %s" % (self.name, code, message))


def _check(rc):
    if rc != 0:
        n = _lib.bebop_last_error(None, 0)
        buf = C.create_string_buffer(n)
        _lib.bebop_last_error(buf, n)
        raise Refusal(rc, buf.raw[:n].decode("utf-8", "replace"))


def _take(ptr, n):
    """Copy a library buffer into bytes and free it."""
    try:
        return C.string_at(ptr, n) if n else b""
    finally:
        _lib.bebop_free(ptr, n)


def _b(x):
    return x.encode() if isinstance(x, str) else bytes(x)


def _get(fn, h, key):
    key = _b(key)
    found, p, n = C.c_int32(), _P(), C.c_size_t()
    _check(fn(h, key, len(key), C.byref(found), C.byref(p), C.byref(n)))
    v = _take(p, n.value)
    return v if found.value else None


class Db:
    """One database: a file (`open`, `create`), or an image in memory (`from_bytes`, `memory`)."""

    def __init__(self, handle, memory=False):
        self._h = handle
        # A memory/bytes database (Detached) promises no durability: it accepts only NONE.
        self._memory = memory

    @classmethod
    def open(cls, path, verify=ANCHOR):
        h = _P()
        _check(_lib.bebop_db_open(os.fsencode(path), 0, verify, C.byref(h)))
        return cls(h)

    @classmethod
    def create(cls, path, verify=ANCHOR):
        h = _P()
        _check(_lib.bebop_db_open(os.fsencode(path), OPEN_CREATE, verify, C.byref(h)))
        return cls(h)

    @classmethod
    def from_bytes(cls, data, verify=ANCHOR):
        """Adopt an image held in memory. A torn last commit is refused, never fallen back past."""
        data = bytes(data)
        h = _P()
        _check(_lib.bebop_db_open_bytes(data, len(data), verify, C.byref(h)))
        return cls(h, memory=True)

    @classmethod
    def memory(cls):
        h = _P()
        _check(_lib.bebop_db_create_mem(C.byref(h)))
        return cls(h, memory=True)

    def read(self):
        return ReadTxn(self)

    def write(self, durability=None):
        """The write transaction. Default durability: DATA for a file, NONE for memory (all it can give)."""
        if durability is None:
            durability = NONE if self._memory else DATA
        return WriteTxn(self, durability)

    def flush(self, durability=None):
        if durability is None:
            durability = NONE if self._memory else DATA
        g = C.c_int64()
        _check(_lib.bebop_db_flush(self._h, durability, C.byref(g)))
        return g.value

    def durable(self):
        g = C.c_int64()
        _check(_lib.bebop_db_durable(self._h, C.byref(g)))
        return g.value

    def verify(self, depth=WHOLE_ARENA):
        g = C.c_int64()
        _check(_lib.bebop_db_verify(self._h, depth, C.byref(g)))
        return g.value

    def compact(self):
        g = C.c_int64()
        _check(_lib.bebop_db_compact(self._h, C.byref(g)))
        return g.value

    def snapshot(self):
        p, n = _P(), C.c_size_t()
        _check(_lib.bebop_db_snapshot(self._h, C.byref(p), C.byref(n)))
        return _take(p, n.value)

    @property
    def poisoned(self):
        return _lib.bebop_db_poisoned(self._h) == 1

    def close(self):
        if self._h:
            _check(_lib.bebop_db_close(self._h))
            self._h = None

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()


class ReadTxn:
    """A snapshot of one generation; later commits do not change what it sees."""

    def __init__(self, db):
        self._h = _P()
        _check(_lib.bebop_read_begin(db._h, C.byref(self._h)))

    def get(self, key):
        return _get(_lib.bebop_read_get, self._h, key)

    def __len__(self):
        n = C.c_uint64()
        _check(_lib.bebop_read_len(self._h, C.byref(n)))
        return n.value

    @property
    def generation(self):
        g = C.c_int64()
        _check(_lib.bebop_read_generation(self._h, C.byref(g)))
        return g.value

    def fold_root(self):
        """The FNV-1a fold over the entries, signed, as every other reader of the format prints it."""
        r = C.c_int64()
        _check(_lib.bebop_read_fold_root(self._h, C.byref(r)))
        return r.value

    def scan(self, lo=None, hi=None):
        """(key, value) pairs with lo <= key < hi in key order; None = unbounded."""
        lo = None if lo is None else _b(lo)
        hi = None if hi is None else _b(hi)
        s = _P()
        _check(_lib.bebop_read_scan(self._h, lo, len(lo or b""), hi, len(hi or b""), C.byref(s)))
        return _drain(s)

    def prefix(self, p):
        p = _b(p)
        s = _P()
        _check(_lib.bebop_read_prefix(self._h, p, len(p), C.byref(s)))
        return _drain(s)

    def close(self):
        if self._h:
            _check(_lib.bebop_read_end(self._h))
            self._h = None

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()


def _drain(s):
    """Step a cursor to its end, copying each entry, and free it (generator)."""
    try:
        has, kp, kl, vp, vl = C.c_int32(), _P(), C.c_size_t(), _P(), C.c_size_t()
        while True:
            _check(_lib.bebop_scan_next(s, C.byref(has), C.byref(kp), C.byref(kl), C.byref(vp), C.byref(vl)))
            if not has.value:
                return
            yield (C.string_at(kp, kl.value) if kl.value else b"", C.string_at(vp, vl.value) if vl.value else b"")
    finally:
        _lib.bebop_scan_free(s)


class WriteTxn:
    """The one writer. As a context manager it commits on success and aborts on an exception."""

    def __init__(self, db, durability=DATA):
        self._h = _P()
        self.durability = durability
        self.generation = None
        _check(_lib.bebop_write_begin(db._h, C.byref(self._h)))

    def put(self, key, value):
        key, value = _b(key), _b(value)
        _check(_lib.bebop_write_put(self._h, key, len(key), value, len(value)))

    def remove(self, key):
        key = _b(key)
        was = C.c_int32()
        _check(_lib.bebop_write_remove(self._h, key, len(key), C.byref(was)))
        return bool(was.value)

    def get(self, key):
        return _get(_lib.bebop_write_get, self._h, key)

    def commit(self, durability=None):
        g = C.c_int64()
        h, self._h = self._h, None
        _check(_lib.bebop_write_commit(h, self.durability if durability is None else durability, C.byref(g)))
        self.generation = g.value
        return g.value

    def stage(self):
        g = C.c_int64()
        h, self._h = self._h, None
        _check(_lib.bebop_write_stage(h, C.byref(g)))
        self.generation = g.value
        return g.value

    def abort(self):
        if self._h:
            _lib.bebop_write_abort(self._h)
            self._h = None

    def __enter__(self):
        return self

    def __exit__(self, et, *rest):
        if et is not None:
            self.abort()
        elif self._h:
            self.commit()
