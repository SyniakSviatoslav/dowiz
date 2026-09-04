# csr: base-131 i64-wrapping fold over row_ptr, then (col_idx, val) pairs of the five CSR GOLDENS graphs in golden.txt (P4, C3, K4W, B6, D2DUP)
import os, re
M = (1 << 64) - 1
def w(x): x &= M; return x - (1 << 64) if x >> 63 else x
txt = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "vs_rust", "spectral_golden", "golden.txt")).read()
sec = txt.split("CSR GOLDENS")[1].split("════")[1]
acc = 0
for m in re.finditer(r"row_ptr: ([\d ]+)\ncol_idx: ([\d ]+)\nval_fp32: ([\d ]+)", sec):
    rp, ci, vv = (list(map(int, g.split())) for g in m.groups())
    for v in rp: acc = w(acc * 131 + v)
    for c, v in zip(ci, vv): acc = w(w(acc * 131 + c) * 131 + v)
print(acc)
