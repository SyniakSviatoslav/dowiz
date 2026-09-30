# Oracle for gate `block_rt` (DG9, SPEC-DATALOG-AND-CODEC §B.6). The twelve block fixtures under
# crates/dowiz-hub/fixtures/blocks (165-dish, small, empty x menu_prices/bom/names/stock_levels),
# decoded and re-encoded by crates/bebop-wasm/oracle.py's reader (block_decode/block_encode: re-derived
# from the spec text, nothing from bebop), folded the way std_tests/block_rt.bp's rt_fix does:
#   acc = acc*31 + v (64-bit wrap) over, per fixture, n, nnz, vals (FNV-1a 64 over each decoded
#   column's count then its values, 8 LE bytes each), rt (1 when encode(decode(b)) == b), and the
#   first 8 bytes of sha256(re-encoding) read big-endian. -1 when any fixture is refused or rt fails.
import hashlib, importlib.util, pathlib, sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("bw_oracle", ROOT / "crates/bebop-wasm/oracle.py")
o = importlib.util.module_from_spec(spec); spec.loader.exec_module(o)
M64 = (1 << 64) - 1
def s64(u): u &= M64; return u - (1 << 64) if u >> 63 else u

acc = 0
for suffix in ["", "_small", "_empty"]:
    for name in ["menu_prices", "bom", "names", "stock_levels"]:
        b = (ROOT / "crates/dowiz-hub/fixtures/blocks" / (name + suffix + ".dwb")).read_bytes()
        try:
            nm, n, nnz, vals = o.block_decode(b)
        except o.Refused as r:
            sys.stderr.write("block_rt oracle: %s%s refused %s col=%s\n" % (name, suffix, r.code, r.col)); print(-1); sys.exit(0)
        h = o.FNV_OFFSET
        for v in vals:
            for x in [len(v)] + v:
                h = o.fnv(h, (x & M64).to_bytes(8, "little"))
        again = o.block_encode(nm, n, nnz, vals)
        if again != b:
            sys.stderr.write("block_rt oracle: %s%s rt=diff\n" % (name, suffix)); print(-1); sys.exit(0)
        k = int.from_bytes(hashlib.sha256(again).digest()[:8], "big")
        for x in [n, nnz, h, 1, k]:
            acc = (acc * 31 + x) & M64
print(s64(acc))
