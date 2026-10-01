# dl_sushi_gen.py -- writes selfhost/std/dl_sushi.bp (set 5's EDB, row DG8) from design/dubin-sushi-menu.json
# and the authored supply table in dl_common.py. `python3 bench/oracles/dl_sushi_gen.py [--check]`:
# --check exits 1 (and says so) when the committed file differs from what this would write.
import sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import dl_common as C
out = C.BL / "selfhost/std/dl_sushi.bp"
text = C.sushi_bp()
if "--check" in sys.argv:
    ok = out.exists() and out.read_text() == text
    print("dl_sushi.bp %s" % ("current" if ok else "STALE -- re-run bench/oracles/dl_sushi_gen.py"))
    sys.exit(0 if ok else 1)
out.write_text(text)
print("wrote %s (%d lines)" % (out, text.count("\n")))
