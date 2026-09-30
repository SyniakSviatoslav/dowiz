# Oracle for gate `block_neg` (DG9, SPEC-DATALOG-AND-CODEC B-1). The same fifteen cases as
# std_tests/block_neg.bp on the 165-dish menu_prices fixture: the untouched twin must decode with
# encode(decode(b)) == b, and each single flipped byte (crc re-sealed, except for the crc's own case)
# must be refused by crates/bebop-wasm/oracle.py's block_check with the region's code AND column
# (DG7's check() order). Prints pass * 1000 + fail.
import importlib.util, pathlib, struct, zlib

ROOT = pathlib.Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("bw_oracle", ROOT / "crates/bebop-wasm/oracle.py")
o = importlib.util.module_from_spec(spec); spec.loader.exec_module(o)
b0 = (ROOT / "crates/dowiz-hub/fixtures/blocks/menu_prices.dwb").read_bytes()
nm, n, nnz, vals = o.block_decode(b0)
ok = 1 if o.block_encode(nm, n, nnz, vals) == b0 else 0
lay = o.block_check(b0)[4]
pad = lay[4][0] + lay[4][1]
def case(at, x, seal, code, col):
    b = bytearray(b0); b[at] ^= x
    if seal:
        struct.pack_into("<I", b, len(b) - 4, zlib.crc32(bytes(b[:-4])) & 0xFFFFFFFF)
    try:
        o.block_check(bytes(b)); return 0
    except o.Refused as r:
        return 1 if (r.code, r.col) == (code, col) else 0
cases = [(0, 1, 1, "bad_magic", "-"), (4, 1, 1, "bad_version", "-"), (len(b0) - 1, 1, 0, "bad_crc", "-"),
         (16, 1, 1, "unknown_schema", "-"), (6, 1, 1, "bad_ncols", "-"), (8, 1, 1, "bad_length", "dish"),
         (12, 1, 1, "bad_length", "mods_col"), (40, 1, 1, "bad_type", "price"), (42, 1, 1, "bad_unit", "price"),
         (43, 1, 1, "bad_reserved", "price"), (48, 1, 1, "bad_alignment", "price"), (48, 8, 1, "bad_offsets", "price"),
         (44, 8, 1, "bad_length", "price"), (pad, 1, 1, "bad_padding", "mods_val")]
ok += sum(case(*c) for c in cases)
print(ok * 1000 + (15 - ok))
