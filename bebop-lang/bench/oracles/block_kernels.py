# Oracle for gate `block_kernels` (LV-COL-E, R-BEBOP-LEVERS §5.4). INDEPENDENT of crates/bebop-wasm/oracle.py:
# it reads the three committed sales fixtures (sales_empty, sales_small, sales) from the bytes alone --
# crc32 trailer, the header's schema K64 against the `sales:v1(...)` string, each column by its §B.2
# descriptor (len, offset) -- and computes the kernel line std_tests/block_kernels.bp prints:
#   kernel sales rows=<n> groups=<g> gross=<fnv64> costed=<m> cost=<fnv64> top=<key:val,...|->
# gross = per-dish sums of `gross` (col 3) over every row; cost = per-dish sums of `cost` (col 6) over the
# rows with cqty (col 4) > 0; each sum must stay in i64 or this exits 1; fnv = FNV-1a 64 over the groups
# in ascending signed key order, key then sum as 8 LE bytes; top = 3 largest gross groups, sum desc then
# key asc, both 16-hex. The gate value: every character of the three lines, in order, folded with
# FNV-1a 64 as one 8-byte little-endian cell each (bebop's bk_fnv8), printed as a signed i64.
import pathlib, struct, sys, zlib

ROOT = pathlib.Path(__file__).resolve().parents[3]
FIX = ROOT / "crates/dowiz-hub/fixtures/blocks"
SCHEMA = b"sales:v1(day:i32:0,dish:i64:0,qty:i32:0,gross:i64:1,cqty:i32:0,cgross:i64:1,cost:i64:1)"
WIDTH = [4, 8, 4, 8, 4, 8, 8]          # i32/i64 per column, schema order
M64, OFF, PRIME = (1 << 64) - 1, 0xCBF29CE484222325, 0x100000001B3
LO, HI = -(1 << 63), (1 << 63) - 1


def die(msg):
    sys.stderr.write("block_kernels oracle: %s\n" % msg)
    sys.exit(1)


def fnv(h, x):
    for b in (x & M64).to_bytes(8, "little"):
        h = ((h ^ b) * PRIME) & M64
    return h


def columns(name):
    b = (FIX / (name + ".dwb")).read_bytes()
    if b[:4] != b"DWB1" or struct.unpack_from("<I", b, len(b) - 4)[0] != zlib.crc32(b[:-4]) & M64 & 0xFFFFFFFF:
        die("%s: magic or crc" % name)
    k64 = ((zlib.crc32(SCHEMA) & 0xFFFFFFFF) << 32) | len(SCHEMA)
    if struct.unpack_from("<Q", b, 16)[0] != k64 or struct.unpack_from("<H", b, 6)[0] != 7:
        die("%s: not a sales:v1 block" % name)
    n = struct.unpack_from("<I", b, 8)[0]
    cols = []
    for c, w in enumerate(WIDTH):
        ln, off = struct.unpack_from("<II", b, 24 + 16 * c + 4)
        if ln != n * w:
            die("%s: column %d length" % (name, c))
        cols.append(list(struct.unpack_from("<%d%s" % (n, "i" if w == 4 else "q"), b, off)))
    return n, cols


def sums(keys, vals, keep):
    g = {}
    for k, v, ok in zip(keys, vals, keep):
        if ok:
            g[k] = g.get(k, 0) + v
            if not LO <= g[k] <= HI:
                die("a sum leaves i64")
    return g


def fold(g):
    h = OFF
    for k in sorted(g):
        h = fnv(fnv(h, k), g[k])
    return h


def line(name):
    n, c = columns(name)
    g = sums(c[1], c[3], [True] * n)
    keep = [q > 0 for q in c[4]]
    cost = sums(c[1], c[6], keep)
    top = sorted(g.items(), key=lambda kv: (-kv[1], kv[0]))[:3]
    tops = ",".join("%016x:%016x" % (k & M64, v & M64) for k, v in top) or "-"
    return "kernel sales rows=%d groups=%d gross=%016x costed=%d cost=%016x top=%s" % (n, len(g), fold(g), sum(keep), fold(cost), tops)


h = OFF
for name in ("sales_empty", "sales_small", "sales"):
    for ch in line(name):
        h = fnv(h, ord(ch))
print(h - (1 << 64) if h >> 63 else h)
