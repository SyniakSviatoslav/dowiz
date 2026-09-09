#!/bin/sh
# L9: single source of truth — regenerate runtime self-source from selfhost/.
# Usage: gen_selfsrc.sh <out.bp> <bootstrap-path-string>   (LEGACY mode, T45: reads selfhost/attic/expr_compile.bp)
#        gen_selfsrc.sh std [outdir]   (T38) expand every gate source: for each
#        bench/vs_rust/std_tests/<g>.bp write outdir/<g>.bp = the prelude files named
#        = selfhost/std/<g>.bp verbatim (T47c: preludes come in through `use` lines).
set -e
MANIFEST=bench/vs_rust/std_tests/GENERATED.txt

# ---------------------------------------------------------------- std mode
# Why a manifest (2026-09-09, lane C5). The expansion drift that has now cost
# FOUR lanes a gate is not really a drift: it is an AMBIGUOUS SOURCE OF TRUTH.
# Until today authority was decided by a file test -- `selfhost/std/<g>.bp` wins
# IF IT EXISTS, otherwise the std_tests copy is its own source. Three
# consequences, all of them live:
#   1. a lane edits selfhost/std/<g>.bp, forgets to regenerate, and the drift is
#      COMMITTED -- so every other lane inherits a RED rung (v) it did not cause;
#   2. deleting a twin silently PROMOTES the std_tests copy to authoritative,
#      with no diagnostic anywhere;
#   3. the comment above claimed "a few gate sources ... csr_build_profile.bp,
#      join_twin.bp, scrash_small.bp" are local copies. Measured 2026-09-09:
#      there are TWENTY-THREE local-only gate sources, so twenty of them were
#      undocumented and their authority rested on an absence.
# GENERATED.txt makes authority EXPLICIT and committed: one line per gate source,
# `generated <twin-path>` or `local`. `--check` then verifies four things a byte
# comparison alone cannot -- that every file is registered, that no `generated`
# entry has lost its twin, that no `local` entry has quietly grown one, and only
# then that the bytes match.
if [ "$1" = std ]; then
  MODE=fix; OUTDIR=bench/vs_rust/std_tests
  shift
  while [ $# -gt 0 ]; do
    case "$1" in
      --check) MODE=check ;;
      --fix)   MODE=fix ;;
      *)       OUTDIR="$1" ;;
    esac
    shift
  done
  [ "$MODE" = check ] && OUTDIR="${BEBOP_TMP:-/tmp/opencode}/std_expand_check"
  mkdir -p "$OUTDIR"
  drift=0; unreg=0; lost=0; grew=0; n=0

  for t in bench/vs_rust/std_tests/*.bp; do
    g=$(basename "$t"); src="selfhost/std/$g"; n=$((n+1))
    ent=""
    [ -f "$MANIFEST" ] && ent=$(sed -n "s|^$g[[:space:]]\{1,\}\([a-z]*\).*|\1|p" "$MANIFEST" | head -1)
    if [ -f "$MANIFEST" ] && [ -z "$ent" ]; then
      echo "UNREGISTERED $g: not in $MANIFEST (add it as \`$g generated selfhost/std/$g\` or \`$g local\`)"
      unreg=$((unreg+1))
    fi
    if [ "$ent" = generated ] && [ ! -f "$src" ]; then
      echo "LOST-TWIN $g: $MANIFEST says generated from $src, which does not exist."
      echo "  Authority used to fall back to the copy SILENTLY. It no longer does."
      lost=$((lost+1)); continue
    fi
    if [ "$ent" = local ] && [ -f "$src" ]; then
      echo "GREW-TWIN $g: $MANIFEST says local, but $src now exists -- decide which is authoritative"
      echo "  and record it in $MANIFEST before either can be trusted."
      grew=$((grew+1)); continue
    fi
    if [ -f "$src" ]; then
      if [ "$MODE" = check ]; then
        cmp -s "$src" "$t" || {
          echo "EXPANSION-DRIFT $g: bench/vs_rust/std_tests/$g != selfhost/std/$g"
          echo "  EDIT selfhost/std/$g (it is the source); the std_tests copy is generated."
          echo "  FIX:  sh tools/gen_selfsrc.sh std --fix    then commit BOTH files."
          drift=$((drift+1)); }
      else
        cat "$src" > "$OUTDIR/$g"
      fi
    elif [ "$MODE" != check ] && [ "$t" != "$OUTDIR/$g" ]; then
      # a genuinely local gate source: self-expansion is a no-op, and with the
      # DEFAULT outdir source and destination are ONE FILE -- the shell truncates
      # the redirect target before `cat` opens it, which emptied 21 gate sources
      # in a lane tree on 2026-09-08 (B4). Skipping is the fix and it stays.
      cat "$t" > "$OUTDIR/$g"
    fi
  done

  if [ "$MODE" = check ]; then
    bad=$((drift + unreg + lost + grew))
    echo "expansion: $n gate sources checked; drift=$drift unregistered=$unreg lost-twin=$lost grew-twin=$grew"
    [ "$bad" = 0 ] || exit 1
    exit 0
  fi
  echo "$OUTDIR: $n gate sources expanded"
  exit 0
fi

# `gen_selfsrc.sh manifest` (re)writes GENERATED.txt from what is on disk today.
# Run it when a gate source is ADDED or when a twin is deliberately created or
# removed -- never to silence a LOST-TWIN or GREW-TWIN report, which are the two
# findings the manifest exists to surface.
if [ "$1" = manifest ]; then
  : > "$MANIFEST".tmp
  echo "# GENERATED.txt -- which bench/vs_rust/std_tests sources are generated and from where." >> "$MANIFEST".tmp
  echo "# Written by \`sh tools/gen_selfsrc.sh manifest\`; checked by \`gen_selfsrc.sh std --check\`" >> "$MANIFEST".tmp
  echo "# and by bench/vs_rust/invariants.sh rung (v). A \`generated\` copy is NOT to be edited:" >> "$MANIFEST".tmp
  echo "# edit the twin named on its line. A \`local\` source has no twin and IS the truth." >> "$MANIFEST".tmp
  for t in bench/vs_rust/std_tests/*.bp; do
    g=$(basename "$t")
    if [ -f "selfhost/std/$g" ]; then echo "$g generated selfhost/std/$g" >> "$MANIFEST".tmp
    else echo "$g local" >> "$MANIFEST".tmp; fi
  done
  mv "$MANIFEST".tmp "$MANIFEST"
  echo "$MANIFEST: $(grep -vc "^#" "$MANIFEST") entries, $(grep -v "^#" "$MANIFEST" | grep -c " generated ") with a twin"
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
