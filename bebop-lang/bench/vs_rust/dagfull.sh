#!/usr/bin/env bash
# dagfull.sh -- the permanent memo-equals-recompute gate (docs/design/SPEC-BEBOP-DAG-RUNTIME-2026-09-28.md
# §8; BLUEPRINT DG4/DG6). The gate's VALUE is its printed counts, never only its rc (§8.1), and it never
# counts a skip as agreement (G-1, memory gates-that-count-skips-as-passes): a missing prerequisite is a
# NAMED refusal with exit 2.
#
#   dagfull.sh --sweep <frozen-dir>   every program of the frozen pre-jump manifest (<dir>/MANIFEST.tsv,
#                                     rev = the dir's `dag-pre-<rev>` suffix) compiled by the CANDIDATE from
#                                     the sources AT <rev> (git archive, read-only), sha256 and rc compared.
#                                     Prints `dagfull sweep <identical>/<programs> pre=<rev>`.
#   dagfull.sh compile [src.bp ...]   RT §8.2 compile arm, per program: cold compile (memo empty) -> c.bin;
#                                     `// dagfull` inserted inside the LAST fn's body -> warm compile -> w.bin,
#                                     which must equal a cold compile of the edited source; the line removed
#                                     -> warm again -> w2.bin, which must equal c.bin. Default programs: the
#                                     live sweep trees + bebop.bp. Prints `dagfull compile <identical>/<programs>
#                                     edit_ms_med <ms> hits <h>` (edit_ms over bebop.bp: 5 one-line edits
#                                     inside `fn em`, warm, median -- RT L-6's method).
#   dagfull.sh check                  RT L-4: `check` leaves the source directory unchanged.
#   dagfull.sh store|sched|datalog    DG5/DG6/DG8 arms: NOT BUILT by DG4 -- refused by name, exit 2.
#
# env: BEBOP_BIN (candidate, default ./bebop.bin), BEBOP_TMP (scratch), DAG_HITS=1 is set by this script
# for its warm compiles so the compiler prints `dag: hits <h>/<n>` on stderr (the hit count the mutation
# proof reads, RT §8.3 step 2). Run it through tools/slot.sh (one compile on the box at a time).
set -u
cd "$(dirname "$0")/../.." || exit 1
BB=$(realpath -m "${BEBOP_BIN:-./bebop.bin}"); SEED=$(realpath ./seed/build/seed)
T=${BEBOP_TMP:-/tmp/opencode}/dagfull; mkdir -p "$T"
[ -s "$BB" ] || { echo "dagfull: REFUSED -- candidate $BB missing or empty (L12)"; exit 2; }
[ -x "$SEED" ] || { echo "dagfull: REFUSED -- $SEED not executable"; exit 2; }
arm=${1:-}; shift || true

cc() { "$SEED" "$BB" compile "$1" "$2" > /dev/null 2> "$2.err"; }

case "$arm" in
--sweep)
  D=${1:?usage: dagfull.sh --sweep <frozen-dir>}; M=$D/MANIFEST.tsv
  [ -s "$M" ] || { echo "dagfull: REFUSED -- frozen manifest $M missing (G-1, L24)"; exit 2; }
  rev=$(basename "$D"); rev=${rev#dag-pre-}
  git -C .. cat-file -e "$rev^{commit}" 2>/dev/null || { echo "dagfull: REFUSED -- pre rev $rev is not a commit here"; exit 2; }
  W=$T/sweep-$rev; rm -rf "$W"; mkdir -p "$W/out"
  git -C .. archive "$rev" bebop-lang | tar -x -C "$W" || { echo "dagfull: REFUSED -- git archive $rev failed"; exit 2; }
  S=$W/bebop-lang
  rows=$(grep -vc '^#' "$M")
  live=$(cd "$S" && ls selfhost/*.bp selfhost/*/*.bp bench/vs_rust/std_tests/*.bp bench/parity_constructs/*.bp \
         bench/parity_constructs/neg/*.bp 2>/dev/null | wc -l)
  [ "$rows" = "$live" ] || { echo "dagfull: REFUSED -- manifest has $rows rows, the tree at $rev has $live .bp files (G-1)"; exit 2; }
  same=0; n=0; bad=""
  while IFS=$'\t' read -r p rc0 bytes sha; do
    case "$p" in '#'*|'') continue ;; esac
    n=$((n + 1)); o=$W/out/$n.bin
    (cd "$S" && "$SEED" "$BB" compile "$p" "$o" > /dev/null 2>&1); rc=$?
    if [ "$rc0" != 0 ]; then
      [ "$rc" = "$rc0" ] && same=$((same + 1)) || bad="$bad $p(rc $rc want $rc0)"
    else
      got=$( [ -s "$o" ] && sha256sum < "$o" | cut -c1-64 || echo none)
      [ "$rc" = 0 ] && [ "$got" = "$sha" ] && same=$((same + 1)) || bad="$bad $p(rc $rc)"
    fi
  done < "$M"
  [ -n "$bad" ] && echo "dagfull: sweep differs:$bad" | cut -c1-2000
  echo "dagfull sweep $same/$n pre=$rev"
  [ "$same" = "$n" ] && [ "$n" = "$rows" ]
  ;;
compile)
  progs=("$@")
  [ ${#progs[@]} -gt 0 ] || progs=(bebop.bp selfhost/*.bp selfhost/*/*.bp bench/vs_rust/std_tests/*.bp bench/parity_constructs/*.bp)
  W=$T/compile; rm -rf "$W"; mkdir -p "$W"
  same=0; n=0; hits=0; bad=""; refused=0
  for p in "${progs[@]}"; do
    [ -f "$p" ] || { echo "dagfull: REFUSED -- $p missing"; exit 2; }
    n=$((n + 1)); d=$W/$n; mkdir -p "$d"
    cc "$p" "$d/c.bin" || { refused=$((refused + 1)); n=$((n - 1)); continue; }   # refused programs: their rc is --sweep's; never counted as agreement
    # the edited copy lives beside the original so its `use` lines resolve the same way; removed below
    e=$(dirname "$p")/.dagfull_$n.bp
    python3 - "$p" "$e" <<'PY' || { echo "dagfull: REFUSED -- no fn body to edit in $p"; exit 2; }
import re, sys
s = open(sys.argv[1]).read()
# inside the LAST fn's body: after its opening `{` (a one-line body gets the comment on its own line)
m = [x.end() for x in re.finditer(r'^(?:kernel )?fn [^\n{]*\{', s, re.M)]
if not m: sys.exit(1)
open(sys.argv[2], 'w').write(s[:m[-1]] + '\n// dagfull\n' + s[m[-1]:])
PY
    cp "$d/c.bin.dag" "$d/w.bin.dag" 2>/dev/null   # warm: the cold compile's memo, under the edited name
    DAG_HITS=1 "$SEED" "$BB" compile "$e" "$d/w.bin" > /dev/null 2> "$d/w.err"
    cc "$e" "$d/wc.bin"                                  # cold compile of the edited source
    cp "$d/w.bin.dag" "$d/w2.bin.dag" 2>/dev/null
    DAG_HITS=1 "$SEED" "$BB" compile "$p" "$d/w2.bin" > /dev/null 2> "$d/w2.err"
    rm -f "$e" "$e".use
    h=$(sed -n 's/^dag: hits \([0-9]*\)\/.*/\1/p' "$d/w2.err" | tail -1); hits=$((hits + ${h:-0}))
    if cmp -s "$d/w.bin" "$d/wc.bin" && cmp -s "$d/w2.bin" "$d/c.bin"; then same=$((same + 1)); else bad="$bad $p"; fi
  done
  [ -n "$bad" ] && echo "dagfull: warm != cold:$bad" | cut -c1-2000
  # L-6: one-line edit inside `fn em` (compiler/fnval_match.bp since the split), warm, median of 5.
  # The compiler tree is copied so the edit never touches the checkout.
  E=$W/edit; mkdir -p "$E/tree/selfhost"; ms=()
  cp bebop.bp "$E/tree/"; cp -r compiler "$E/tree/"; cp -r selfhost/prelude "$E/tree/selfhost/"
  (cd "$E/tree" && "$SEED" "$BB" compile bebop.bp "$E/base.bin" > /dev/null 2>&1) || { echo "dagfull: REFUSED -- base compile of bebop.bp failed"; exit 2; }
  for i in 1 2 3 4 5; do
    python3 - "$E/tree" "$i" <<'PY' || { echo "dagfull: REFUSED -- fn em not found"; exit 2; }
import glob, os, sys
t, i = sys.argv[1], sys.argv[2]
for p in sorted(glob.glob('compiler/*.bp')) + ['bebop.bp']:
    s = open(p).read(); k = s.find('\nfn em(')
    if k < 0: continue
    b = s.index('{\n', k) + 2
    open(os.path.join(t, p), 'w').write(s[:b] + '  // x%s\n' % i + s[b:]); sys.exit(0)
sys.exit(1)
PY
    cp "$E/base.bin.dag" "$E/e$i.bin.dag"
    t0=$(date +%s%N); (cd "$E/tree" && "$SEED" "$BB" compile bebop.bp "$E/e$i.bin" > /dev/null 2>&1); t1=$(date +%s%N)
    ms+=($(( (t1 - t0) / 1000000 )))
  done
  med=$(printf '%s\n' "${ms[@]}" | sort -n | sed -n 3p)
  echo "dagfull compile $same/$n edit_ms_med $med hits $hits refused $refused (edit ms: ${ms[*]})"
  [ "$same" = "$n" ]
  ;;
check)
  W=$T/check; rm -rf "$W"; mkdir -p "$W/src"
  cp bench/vs_rust/std_tests/sproj.bp "$W/src/p.bp"; ls -la "$W/src" | md5sum > "$W/before"
  "$SEED" "$BB" check "$W/src/p.bp" > /dev/null 2>&1; rc=$?   # from the repo root: its `use` lines resolve
  ls -la "$W/src" | md5sum > "$W/after"
  if cmp -s "$W/before" "$W/after"; then echo "dagfull check: source dir unchanged (rc $rc)"
  else echo "dagfull check: source dir CHANGED: $(ls "$W/src" | tr '\n' ' ')"; exit 1; fi
  ;;
store|sched|datalog)
  echo "dagfull $arm: REFUSED -- arm not built (owned by DG5/DG6/DG8, BLUEPRINT §2-3); not counted as agreement"; exit 2 ;;
*) echo "usage: dagfull.sh --sweep <frozen-dir> | compile [src.bp ...] | check | store | sched | datalog"; exit 2 ;;
esac
