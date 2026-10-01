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
#   dagfull.sh datalog                DG8 arm (SPEC-DATALOG-AND-CODEC A.7): the five rule sets of dl_fix.bp,
#                                     DL_EVENTS (10^4) seeded events each, incremental == from scratch.
#                                     Prints `dagfull datalog <equal>/<rulesets> events <n>`.
#   dagfull.sh sched                  DG6 arm (RT §7, §8.2): the scheduler probe shapes, W=0 vs W=3 twice in one
#                                     process. Prints `dagfull sched <equal>/<shapes> wakes .. threshold .. report ..
#                                     speedup_x100 ..` (details at the arm).
#   dagfull.sh store                  DG5 arm: NOT BUILT -- refused by name, exit 2.
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
m = [x.end() for x in re.finditer(r'^(?:(?:pure|sched|kernel) )*fn [^\n{]*\{', s, re.M)]
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
datalog)
  # DG8 (SPEC-DATALOG-AND-CODEC A.7): each of selfhost/std/dl_fix.bp's five rule sets (A.6) -- its fixture,
  # DL_EVENTS (default 10^4) seeded EDB events each propagated through the dirty set, and every 100 events
  # and after the last the incremental IDB compared with a from-scratch re-derivation (empty memo) cell for
  # cell and by fold (dl_gate: -1 and a `stale` line naming predicate, row count and first row otherwise).
  # DL_ROOT=<tree> evaluates a scratch copy of selfhost/std (dagfull.prove.sh step 4's mutation).
  EV=${DL_EVENTS:-10000}; R0=$(realpath "${DL_ROOT:-.}"); W=$T/datalog; rm -rf "$W"; mkdir -p "$W"
  [ -f "$R0/selfhost/std/dl_fix.bp" ] || { echo "dagfull datalog: REFUSED -- $R0/selfhost/std/dl_fix.bp missing"; exit 2; }
  names=(- unavailable fsm_ok courier_may personal allergen); eq=0; n=0; bad=""
  for s in 1 2 3 4 5; do
    n=$((n + 1))
    printf 'use "selfhost/std/dl_fix.bp"\nfn main() -> i64 { dl_gate(%d, 0, %d, 100) }\n' "$s" "$EV" > "$R0/.dagfull_dl$s.bp"
    (cd "$R0" && "$SEED" "$BB" compile ".dagfull_dl$s.bp" "$W/dl$s.bin" > /dev/null 2> "$W/dl$s.err"); rc=$?
    rm -f "$R0/.dagfull_dl$s.bp" "$R0/.dagfull_dl$s.bp.use"
    [ "$rc" = 0 ] || { echo "dagfull datalog: REFUSED -- set $s does not compile (rc $rc): $(tail -1 "$W/dl$s.err")"; exit 2; }
    timeout 600 "$SEED" "$W/dl$s.bin" > "$W/dl$s.out" 2>&1; rc=$?
    v=$(tail -1 "$W/dl$s.out")
    echo "dagfull datalog: set $s ${names[$s]} SEED $((12345 + 1000 * s)) events $EV rc $rc value $v $(grep -m1 '^dl: ' "$W/dl$s.out")"
    case "$v" in ''|-1|*[!0-9-]*) bad="$bad ${names[$s]}" ;; *) [ "$rc" = 0 ] && eq=$((eq + 1)) || bad="$bad ${names[$s]}(rc $rc)" ;; esac
  done
  [ -n "$bad" ] && echo "dagfull datalog: incremental != scratch:$bad"
  echo "dagfull datalog $eq/$n events $EV"
  [ "$eq" = "$n" ]
  ;;
sched)
  # DG6 (RT §7, §8.2): the R §3 probe shapes on selfhost/prelude/sched.bp via selfhost/std/sched_probe.bp.
  # Each program runs W=0 twice and W=SCHED_W twice in ONE process (same-session control) and prints
  # `sched N L K W serial_ms a b par_ms c d folds s1 s2 p1 p2 wakes x1 x2 par_levels y1 y2`.
  #   sched_det        (T-1) all four folds equal, every shape
  #   sched_wakes      (T-2) wakes == par_levels on every run, and the 8x4 shape ran all 4 levels parallel
  #   sched_threshold  (T-4) 64x8x100000 routed serial (par_levels 0 0) and par wall <= 1.2x serial wall
  #   report           (T-6) BEBOP_SCHED_REPORT=1 prints one `level ..` line per level per run (4 runs x L)
  #   speedup_x100     serial/par wall on 8x4x5000000 (RT acceptance >= 230; printed, gated only with
  #                    SCHED_SPEEDUP_GATE=1 -- a wall ratio on a shared phone is a measurement, not a law)
  # SCHED_ROOT=<tree> evaluates a scratch copy of selfhost/ (dagfull.prove.sh step 5's mutation).
  # SCHED_SHAPES="N:L:K ..." overrides the shape list (the prove step runs 24:1:2000000 only).
  R0=$(realpath "${SCHED_ROOT:-.}"); W=$T/sched; rm -rf "$W"; mkdir -p "$W"; SW=${SCHED_W:-3}
  for f in selfhost/prelude/sched.bp selfhost/std/sched_probe.bp; do
    [ -f "$R0/$f" ] || { echo "dagfull sched: REFUSED -- $R0/$f missing (G-1)"; exit 2; }
  done
  shapes=${SCHED_SHAPES:-"8:4:5000000 3:4:20000000 64:8:100000 24:1:2000000 1:24:2000000"}
  eq=0; n=0; wk=0; wn=0; bad=""; thr=none; spd=none; rep=none
  for sh in $shapes; do
    IFS=: read -r N L K <<< "$sh"; n=$((n + 1))
    printf 'use "selfhost/std/sched_probe.bp"\nfn main() -> i64 { sp_probe(%d, %d, %d, %d) }\n' "$N" "$L" "$K" "$SW" > "$R0/.dagfull_sched$n.bp"
    (cd "$R0" && "$SEED" "$BB" compile ".dagfull_sched$n.bp" "$W/s$n.bin" > /dev/null 2> "$W/s$n.cerr"); rc=$?
    rm -f "$R0/.dagfull_sched$n.bp" "$R0/.dagfull_sched$n.bp.use"
    [ "$rc" = 0 ] || { echo "dagfull sched: REFUSED -- shape $sh does not compile (rc $rc): $(tail -1 "$W/s$n.cerr")"; exit 2; }
    rp=0; [ "$N:$L:$K" = "8:4:5000000" ] && rp=1
    BEBOP_SCHED_REPORT=$rp timeout 600 "$SEED" "$W/s$n.bin" > "$W/s$n.out" 2> "$W/s$n.err"; rc=$?
    v=$(tail -1 "$W/s$n.out"); ln=$(grep -m1 '^sched N ' "$W/s$n.out")
    echo "dagfull sched: shape $sh rc $rc value $v | $ln"
    set -- $ln   # sched N n L l K k W w serial_ms a b par_ms c d folds s1 s2 p1 p2 wakes x1 x2 par_levels y1 y2
    if [ "$rc" = 0 ] && [ "$v" = 1 ] && [ $# = 26 ]; then eq=$((eq + 1)); else bad="$bad $sh(rc $rc value $v)"; fi
    if [ $# = 26 ]; then
      wn=$((wn + 1)); [ "${22}" = "${25}" ] && [ "${23}" = "${26}" ] && wk=$((wk + 1)) || bad="$bad $sh(wakes ${22},${23} != par_levels ${25},${26})"
      if [ "$N:$L:$K" = "8:4:5000000" ]; then
        [ "${25}" = "$L" ] && [ "${26}" = "$L" ] || bad="$bad $sh(par_levels ${25},${26} want $L)"
        spd=$(( (${11} + ${12}) * 100 / ( (${14} + ${15}) > 0 ? (${14} + ${15}) : 1 ) ))
        lv=$(grep -c '^level [0-9]* width [0-9]* est_us [0-9]* mode \(serial\|W=[0-9]*\) wall_us [0-9]*$' "$W/s$n.err")
        [ "$lv" = $((4 * L)) ] && rep="$lv/$((4 * L))" || { rep="$lv/$((4 * L))"; bad="$bad report($lv lines want $((4 * L)))"; }
      fi
      if [ "$N:$L:$K" = "64:8:100000" ]; then
        if [ "${25}" = 0 ] && [ "${26}" = 0 ] && [ $(( (${14} + ${15}) * 10 )) -le $(( (${11} + ${12}) * 12 + 20 )) ]; then thr=serial
        else thr=RED; bad="$bad threshold(par_levels ${25},${26} par_ms ${14}+${15} serial_ms ${11}+${12})"; fi
      fi
    fi
  done
  [ -n "$bad" ] && echo "dagfull sched: RED:$bad" | cut -c1-2000
  sg=ok; [ "$spd" != none ] && [ "$spd" -lt "${SCHED_MIN_SPEEDUP:-230}" ] && sg=low
  echo "dagfull sched $eq/$n wakes $wk/$wn threshold $thr report $rep speedup_x100 $spd ($sg) W=$SW"
  [ "$eq" = "$n" ] && [ "$wk" = "$wn" ] && [ "$wn" = "$n" ] && [ -z "$bad" ] || exit 1
  [ "${SCHED_SPEEDUP_GATE:-0}" = 1 ] && [ "$sg" = low ] && exit 1
  exit 0
  ;;
store)
  echo "dagfull $arm: REFUSED -- arm not built (owned by DG5, BLUEPRINT §2-3); not counted as agreement"; exit 2 ;;
*) echo "usage: dagfull.sh --sweep <frozen-dir> | compile [src.bp ...] | check | datalog | sched | store"; exit 2 ;;
esac
