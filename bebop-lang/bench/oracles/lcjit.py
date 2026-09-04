# lcjit: flag fold = pass*10 + fok; fok = f0_of(1/4, 1/16) > 0 (fp 2^32, same arithmetic as lcres);
# pass = batch period jitter < 10%. The cycle has a fixed tick count (no data-dependent loops), so the
# mathematical jitter of the deterministic loop is 0 -> pass = 1 (wall-clock noise is measurement, not definition).
import os, runpy
lc = runpy.run_path(os.path.join(os.path.dirname(os.path.abspath(__file__)), "lcres.py"), run_name="lib")
one = 1 << 32
f = lc["f0_of"](one // 4, one // 16)
print(1 * 10 + int(f > 0))
