# Oracle for gate `block_schema` (DG9, SPEC-DATALOG-AND-CODEC B-2). The four §B.4 schema strings as
# crates/bebop-wasm/oracle.py carries them (BLOCK_SCHEMAS, in DG7's schema::TABLE order), each keyed
# K64 = (zlib crc32(string) << 32) | len, folded acc*31 + K64 (64-bit wrap) as std_tests/block_schema.bp
# does. Second half: every key must be the `schema` header field of a committed fixture of that
# name, or this exits 1 -- so a drifted string here cannot quietly agree with bebop.
import importlib.util, pathlib, struct, sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("bw_oracle", ROOT / "crates/bebop-wasm/oracle.py")
o = importlib.util.module_from_spec(spec); spec.loader.exec_module(o)
M64 = (1 << 64) - 1
acc = 0
for s in o.BLOCK_SCHEMAS:
    name, k = o.block_parse(s)[0], o.k64_of(s.encode())
    fx = (ROOT / "crates/dowiz-hub/fixtures/blocks" / (name + ".dwb")).read_bytes()
    if struct.unpack_from("<Q", fx, 16)[0] != k:
        sys.stderr.write("block_schema oracle: %s key %016x is not the fixture's\n" % (name, k)); sys.exit(1)
    acc = (acc * 31 + k) & M64
print(acc - (1 << 64) if acc >> 63 else acc)
