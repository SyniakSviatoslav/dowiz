#!/bin/sh
# L9: single source of truth — regenerate runtime self-source from selfhost/.
# Usage: gen_selfsrc.sh <out.bp> <bootstrap-path-string>   (LEGACY mode, T45: reads selfhost/attic/expr_compile.bp)
#        gen_selfsrc.sh std [outdir]   (T38) expand every gate source: for each
#        bench/vs_rust/std_tests/<g>.bp write outdir/<g>.bp = the prelude files named
#        = selfhost/std/<g>.bp verbatim (T47c: preludes come in through `use` lines).
set -e
if [ "$1" = std ]; then
  OUTDIR="${2:-bench/vs_rust/std_tests}"; mkdir -p "$OUTDIR"
  for t in bench/vs_rust/std_tests/*.bp; do
    g=$(basename "$t"); src="selfhost/std/$g"
    [ -f "$src" ] || { echo "$src: missing (gate source without a selfhost/std twin)" >&2; exit 1; }
    # T47c (2026-09-05): the prelude is included by `use "selfhost/prelude/<m>.bp"` lines
    # inside the gate source now (bebop.bin resolves them at compile time); the
    # std_tests copy is verbatim. The old `// prelude:` header concatenation is gone.
    cat "$src" > "$OUTDIR/$g"
  done
  echo "$OUTDIR: $(ls "$OUTDIR"/*.bp | wc -l) gate sources expanded"
  exit 0
fi
OUT="$1"; PATHSTR="${2:-/tmp/bebop_self_src.bp}"
python3 - "$OUT" "$PATHSTR" <<'PY'
import re,sys
out,path=sys.argv[1],sys.argv[2]
comp=open('selfhost/attic/expr_compile.bp').read()  # T45: legacy self-source mode, kept for archaeology only
m=re.search(r'fn self_bootstrap\(\)[\s\S]*?\n\}\n',comp)
body=comp.replace(m.group(0),'',1) if m else comp
pb="\n".join(f"  let _ = p[{k}] = {v};" for k,v in enumerate(path.encode()))
boot=f"""fn self_bootstrap() -> i64 {{
  let p = zeros({len(path)+1});
{pb}
  let _ = p[{len(path)}] = 0;
  let fd = sys_open(p, {len(path)}, 0);
  let srcv = sys_slurp(fd, 400000);
  let _ = sys_close(fd);
  let words = emit_words(srcv);
  let cnt = sum_words(words, 1);
  cnt
}}
"""
open(out,'w').write(boot+body)
print(f"{out}: generated ({len(boot+body)} bytes)")
PY
