# Oracle for gate `nodekey` (DG2, SPEC-BEBOP-DAG-RUNTIME §2). W-BATGREEN 2026-09-29: the gate
# landed in d36a32cb without this file (L17: bench/oracles/run_all.sh read `nodekey - ... MISSING`).
# Builds the three node-key frames of crates/bebop-wasm/fixtures/key.expected ITSELF, from the field
# values that file's header lists -- nothing imported from bebop, bebop-store, bebop-wasm or that
# crate's oracle.py -- then folds them the way std_tests/nodekey.bp's nk_fold does:
#   acc = acc*31 + v (64-bit wrap) over (compile, proj, empty) x (len, K64, K256 words 0..3),
#   a K256 word = 8 digest bytes read big-endian, as a signed i64; -1 if RT K-3 fails.
# Frame := tag(1) || field*; field := len(8, u64 LE) || bytes; a number is an 8-byte field, a list
# is ONE field holding its elements' fields. K64 = (zlib crc32(frame) << 32) | (len & 0xffffffff).
# Second, independent half: each frame's len/K64/K256 must equal key.expected's committed numbers,
# or this exits 1 naming the disagreement -- so a wrong frame here cannot quietly agree with bebop.
import hashlib, pathlib, struct, sys, zlib

M64 = (1 << 64) - 1
def s64(u): u &= M64; return u - (1 << 64) if u >> 63 else u
def field(b): return struct.pack("<Q", len(b)) + b
def num(v): return field(struct.pack("<q", v))
def k64(f): return s64(((zlib.crc32(f) & 0xFFFFFFFF) << 32) | (len(f) & 0xFFFFFFFF))

CD, SRC = -7046029254386353131, b"fn add(a: i64, b: i64) -> i64 { a + b }"
comp = b"C" + num(CD) + field(SRC) + num(81985529216486895) + num(-2) + num(1311768467294899695)
def proj(two):
    ins = num(1234605616436508552) + num(7) + field(bytes(range(32))) + num(1) if two else b""
    par = num(1790000000000) + num(3) if two else b""
    return b"P" + num(CD) + num(k64(comp)) + field(ins) + field(par)
frames = [("compile", comp), ("proj", proj(True)), ("empty", proj(False))]

exp = {}
kx = pathlib.Path(__file__).resolve().parents[3] / "crates/bebop-wasm/fixtures/key.expected"
for line in kx.read_text().splitlines():
    if "=" in line and not line.startswith("#"):
        k, v = line.split("=", 1); exp[k.strip()] = v.strip()
acc, bad = 0, []
for name, f in frames:
    d = hashlib.sha256(f).digest()
    got = {"len": str(len(f)), "k64": str(k64(f)), "k256": d.hex()}
    bad += ["%s_%s ours=%s key.expected=%s" % (name, k, v, exp.get(name + "_" + k))
            for k, v in got.items() if exp.get(name + "_" + k) != v]
    for v in [len(f), k64(f)] + [s64(int.from_bytes(d[8 * i:8 * i + 8], "big")) for i in range(4)]:
        acc = (acc * 31 + v) & M64
if bad:
    sys.stderr.write("nodekey oracle DISAGREES with key.expected: " + "; ".join(bad) + "\n"); sys.exit(1)
k3 = len(comp) == 1 + 5 * 8 + 8 + len(SRC) + 8 + 8 + 8
print(s64(acc) if k3 else -1)
