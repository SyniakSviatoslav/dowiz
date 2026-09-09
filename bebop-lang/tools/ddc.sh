#!/usr/bin/env bash
# tools/ddc.sh -- D5 (T89): diverse double-compiling, and the exact size of what it proves.
#
# WHY. `gen3 == gen4` is a FIXPOINT: it proves bebop.bin reproduces itself from
# bebop.bp. It cannot distinguish a faithful compiler from one that recognises
# its own source and re-inserts a backdoor (Thompson 1984; formalised in
# arXiv:1004.5534 "Fully Countering Trusting Trust through Diverse Double-
# Compiling"). DDC answers that by building the same source with a SECOND,
# independently written compiler and comparing.
#
# WHAT THIS SCRIPT REPORTS.
# The D5 row said the witness "already exists in-tree (selfhost/expr_compile.bp),
# so the fix ADDS NO DEPENDENCY". Measured 2026-09-09 (D5), that premise did not
# hold: the witness emitted code for the retired `exec_words` stack machine and
# its output did not run -- `fn main() -> i64 { 42 }` died with SIGBUS, and 1 of
# 75 frozen constructs "agreed", that one only because its frozen EXPECT is 0,
# which a broken binary also yields.
#
# D5b (2026-09-09) repaired the calling contract: `emit_epilogue` never popped
# the function result off the eval stack, so the frame teardown ran 16 bytes low
# and `ret` branched into the result. One `pop(insns, n, 0)` fixed it. The
# measurement is now real -- see `--measure` for today's number.
#
# It is still NOT diverse double-compiling, and this script will not say it is
# until every frozen construct agrees AND the two-stage form in `--full` runs.
# Two counts are printed and both matter:
#   agree      -- the witness's binary printed the same value as the reference's
#   non-vacuous-- the same, EXCLUDING constructs whose frozen EXPECT is 0, since
#                 a binary that traps or prints nothing can also yield 0 and such
#                 an agreement is worth nothing. Quote the non-vacuous number.
#
# `--gate` exits non-zero until the count is total. It is deliberately NOT wired
# into `battery.sh` or `std_golden.sh`: a red gate nobody can turn green is a
# broken build, and this finding belongs in docs/TRUST-CHAIN.md, which states the
# guarantee. Do not quote this script as "the trust chain is closed".
#
# Usage:  tools/ddc.sh              the chain, the evidence, and the measurement
#         tools/ddc.sh --chain      artifact hashes only, no compiles
#         tools/ddc.sh --why        the evidence for the limits stated above
#         tools/ddc.sh --measure    witness acceptance over the frozen corpus
#         tools/ddc.sh --recut      rewrite tools/ddc_subset.txt from a measurement
#                                   (for the day a revived witness accepts more
#                                   than a handful of constructs)
#         tools/ddc.sh --full       attempt Wheeler's two-stage DDC of bebop.bp
#                                   through the witness (slow, documented to
#                                   fail; kept runnable so the claim above stays
#                                   falsifiable rather than merely asserted)
set -u

cd "$(dirname "$0")/.." || exit 1
OUT=${BEBOP_TMP:-/tmp/opencode}/ddc
mkdir -p "$OUT" || exit 1
SEED=./seed/build/seed
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}
WIT_SRC=selfhost/attic/expr_compile.bp
WIT_DRV=selfhost/attic/ec_driver.bp
CP=bench/vs_rust/construct_parity.sh
SUBSET_FILE=tools/ddc_subset.txt

# $PIN from tools/slot.sh is a COMMAND PREFIX ("taskset -c 4,5,6"); older callers
# export a bare cpu list ("4,5,6"). Accept either (tools/perf.py:30-39).
PINCMD=""
if [ -n "${PIN:-}" ]; then
  case "$PIN" in
    taskset*) PINCMD="$PIN" ;;
    *) PINCMD="taskset -c $PIN" ;;
  esac
fi
run() { if [ -n "$PINCMD" ]; then $PINCMD "$@"; else "$@"; fi; }

h() { md5sum "$1" 2>/dev/null | awk '{print $1}'; }
sz() { wc -c < "$1" 2>/dev/null | tr -d ' '; }

for f in "$SEED" "$BEBOP_BIN" "$WIT_SRC" "$WIT_DRV" bebop.bp seed/seed.S "$CP"; do
  [ -s "$f" ] || { echo "GUARD: missing or empty $f"; exit 1; }
done

chain() {
  echo "== artifact chain (every hash from seed.S to the promoted binary)"
  printf '%-34s %9s  %s\n' file bytes md5
  for f in seed/seed.S seed/pack.py seed/build/seed bebop.bp \
           selfhost/prelude/sha256.bp "$BEBOP_BIN" "$WIT_SRC" "$WIT_DRV"; do
    printf '%-34s %9s  %s\n' "$f" "$(sz "$f")" "$(h "$f")"
  done
  if command -v objcopy >/dev/null 2>&1; then
    objcopy -O binary -j .text seed/build/seed "$OUT/seed.text" 2>/dev/null \
      && printf '%-34s %9s  %s\n' "seed/build/seed(.text)" "$(sz "$OUT/seed.text")" "$(h "$OUT/seed.text")"
  fi
  echo "seed.S -> seed/build/seed .text is gated by bench/vs_rust/invariants.sh (viii)."
  echo "bebop.bp + bebop.bin -> gen2..gen4 byte-exact is gated by tools/chain.sh --codegen."
  echo "NEITHER is a trust gate; both are reproducibility gates. See docs/TRUST-CHAIN.md."
  echo "bebop.bin is reproducible from nothing but itself. That is the open link."
}

why() {
  echo "== why DDC does not run in this tree (evidence, not assertion)"
  echo -n "witness \`use\` support (grep for a use handler in $WIT_SRC): "
  grep -c 'skip_use\|is_use\|"use"' "$WIT_SRC" 2>/dev/null || true
  echo "bebop.bp \`use\` lines the witness would silently drop: $(grep -c '^use ' bebop.bp)"
  echo "bebop.bp top-level fns: $(grep -c '^fn ' bebop.bp)   witness top-level fns: $(grep -c '^fn ' "$WIT_SRC")"
  echo "ec_driver.bp fn-table capacity: $(grep -o 'zeros([0-9]*)' "$WIT_DRV" | head -1) (overflow is UNCHECKED)"
  echo "the witness targets the RETIRED stack machine; invariants.sh (ix) gates it out:"
  grep -h 'push_words == 0' bench/vs_rust/invariants.sh | head -1 | sed 's/^/  /'
  echo "bpref cannot stand in as the witness -- every sys_* builtin is a stub:"
  grep -n "name.startswith('sys_')" tools/bpref.py | sed "s|^|  tools/bpref.py:|"
  echo "and the audit recorded the blocker before the D5 row was written:"
  grep -h 'T89 DDC' docs/ROADMAP-AUDIT*.md 2>/dev/null | sed 's/^/  /'
}

build_witness() {
  cat "$WIT_SRC" "$WIT_DRV" > "$OUT/witness.bp" || exit 1
  run $SEED "$BEBOP_BIN" compile "$OUT/witness.bp" "$OUT/witness.bin" >/dev/null 2>&1 \
    || { echo "FAIL: bebop.bin could not compile the witness source"; exit 1; }
  [ -s "$OUT/witness.bin" ] || { echo "FAIL: witness.bin is empty (silent-artifact class)"; exit 1; }
  echo "witness  $(sz "$OUT/witness.bp") B src -> $(sz "$OUT/witness.bin") B bin  md5 $(h "$OUT/witness.bin")"
}

smoke() {
  echo "== witness smoke test: the smallest program in the language"
  printf 'fn main() -> i64 { 42 }\n' > "$OUT/smoke.bp"
  run $SEED "$OUT/witness.bin" compile "$OUT/smoke.bp" "$OUT/smoke.wit.bin" >/dev/null 2>&1
  run $SEED "$BEBOP_BIN"       compile "$OUT/smoke.bp" "$OUT/smoke.ref.bin" >/dev/null 2>&1
  local vr vw rw
  vr=$($SEED "$OUT/smoke.ref.bin" 2>/dev/null | tail -1)
  vw=$($SEED "$OUT/smoke.wit.bin" 2>/dev/null | tail -1); rw=$?
  [ -n "$vr" ] || { echo "GUARD: the REFERENCE compiler printed nothing for 42 -- fix the tree, not this script"; exit 1; }
  echo "  ref     $(sz "$OUT/smoke.ref.bin") B -> '$vr'"
  echo "  witness $(sz "$OUT/smoke.wit.bin") B -> '$vw' (seed exit $rw; 0 = the D5b epilogue repair holds, 135 = SIGBUS regression)"
}

expect_of() {  # expect_of <name> -> the frozen EXPECT from construct_parity.sh
  sed -n "s/^[[:space:]]*$1)[[:space:]]*EXPECT=\([^;]*\);;.*/\1/p" "$CP" | head -1
}

measure() {  # measure <mode>  mode=report | recut
  local mode=$1 names f name exp a b va vb
  names=$(ls bench/parity_constructs/*.bp 2>/dev/null | xargs -n1 basename 2>/dev/null | sed 's/\.bp$//')
  local agree=0 diverge=0 skip=0 accepted="" agree_vac=0 total_vac=0
  for name in $names; do
    f=bench/parity_constructs/$name.bp
    [ -s "$f" ] || { skip=$((skip+1)); continue; }
    exp=$(expect_of "$name")
    case "$exp" in ''|*COMPILEFAIL*|*RUNFAIL*) skip=$((skip+1)); continue;; esac
    [ "$exp" = 0 ] && total_vac=$((total_vac+1))
    a=$OUT/$name.ref.bin; b=$OUT/$name.wit.bin
    run $SEED "$BEBOP_BIN" compile "$f" "$a" >/dev/null 2>&1 || { echo "REFFAIL $name"; diverge=$((diverge+1)); continue; }
    va=$(timeout 120 $SEED "$a" 2>/dev/null | tail -1)
    if [ "$va" != "$exp" ]; then echo "REFDRIFT $name ref=$va frozen=$exp"; diverge=$((diverge+1)); continue; fi
    if ! run $SEED "$OUT/witness.bin" compile "$f" "$b" >/dev/null 2>&1 || [ ! -s "$b" ]; then
      diverge=$((diverge+1)); [ "$mode" = report ] && echo "WITFAIL   $name (witness produced no binary)"; continue
    fi
    vb=$(timeout 120 $SEED "$b" 2>/dev/null | tail -1)
    if [ "$vb" = "$va" ]; then
      agree=$((agree+1)); accepted="$accepted $name"
      [ "$exp" = 0 ] && agree_vac=$((agree_vac+1))
      [ "$mode" = report ] && echo "AGREE     $name = $va (ref $(sz "$a") B, witness $(sz "$b") B -- different code, same value)$([ "$exp" = 0 ] && echo '  [VACUOUS: EXPECT=0]')"
    else
      diverge=$((diverge+1))
      [ "$mode" = report ] && echo "DIVERGE   $name frozen=$exp ref=$va witness='$vb'"
    fi
  done
  local total=$((agree+diverge))
  if [ "$mode" = recut ]; then
    { echo "# tools/ddc_subset.txt -- the surface subset BOTH code generators get right."
      echo "# Regenerated by \`tools/ddc.sh --recut\` on $(date -u +%Y-%m-%d)."
      echo "# Measured: $agree of $total frozen constructs agree. A name entering this list"
      echo "# widens the witnessed surface; a name leaving it narrows the trust claim and"
      echo "# must be explained in docs/TRUST-CHAIN.md."
      for name in $accepted; do echo "$name"; done
    } > "$SUBSET_FILE"
    echo "wrote $SUBSET_FILE: $agree of $total"
    return 0
  fi
  echo
  echo "ddc measurement: $agree of $total frozen constructs agree; $diverge diverge; $skip skipped (no numeric EXPECT)"
  echo "                 non-vacuous: $((agree-agree_vac)) of $((total-total_vac)) (EXCLUDING $total_vac constructs whose frozen EXPECT is 0,"
  echo "                 where $agree_vac agreed -- a trapping or silent binary also yields 0). QUOTE THIS NUMBER." 
  if [ "$agree" -lt "$total" ]; then
    echo "DDC: NOT ESTABLISHED. The in-tree witness does not compile the language it"
    echo "     is supposed to witness, so no diverse double-compiling has taken place."
    echo "     W2 == the golden fixpoint byte-exact is UNPROVEN and this script will not"
    echo "     print it. docs/TRUST-CHAIN.md states the guarantee that IS supported."
    return 1
  fi
  echo "DDC: every frozen construct agrees across two code generators."
  return 0
}

full_ddc() {
  echo "== --full: Wheeler two-stage DDC of bebop.bp through the witness"
  sed -e "1r selfhost/prelude/sha256.bp" -e '1d' bebop.bp > "$OUT/bebop_inlined.bp"
  echo "   stage 0: bebop.bp with its one \`use\` line inlined = $(sz "$OUT/bebop_inlined.bp") B"
  echo "   stage 1: W1 = witness(bebop.bp), timeout ${DDC_FULL_TIMEOUT:-900}s ..."
  if run timeout "${DDC_FULL_TIMEOUT:-900}" $SEED "$OUT/witness.bin" compile "$OUT/bebop_inlined.bp" "$OUT/W1.bin" >/dev/null 2>&1 && [ -s "$OUT/W1.bin" ]; then
    echo "   stage 1 produced W1 = $(sz "$OUT/W1.bin") B md5 $(h "$OUT/W1.bin")"
    echo "   stage 2: W2 = W1(bebop.bp) ..."
    if run timeout "${DDC_FULL_TIMEOUT:-900}" $SEED "$OUT/W1.bin" compile "$OUT/bebop_inlined.bp" "$OUT/W2.bin" >/dev/null 2>&1 && [ -s "$OUT/W2.bin" ]; then
      run $SEED "$BEBOP_BIN" compile bebop.bp "$OUT/gen2.bin" >/dev/null 2>&1
      echo "   W2   $(sz "$OUT/W2.bin") B md5 $(h "$OUT/W2.bin")"
      echo "   gen2 $(sz "$OUT/gen2.bin") B md5 $(h "$OUT/gen2.bin")"
      if cmp -s "$OUT/W2.bin" "$OUT/gen2.bin"; then
        echo "   DDC: W2 == the golden fixpoint, BYTE-EXACT."
      else
        echo "   DDC: W2 != the golden fixpoint. Either the witness is wrong or the"
        echo "        shipped binary contains something bebop.bp does not."
      fi
    else
      echo "   stage 2 FAILED: W1 is not a working compiler (expected -- see --why)."
      return 1
    fi
  else
    echo "   stage 1 FAILED (no W1). Measured 2026-09-09: rc=124, i.e. the witness did"
    echo "   not finish 359,331 B of source in 600 s; and it could not have succeeded"
    echo "   anyway -- it drops the \`use\` line and its fn table holds 256 of 287 fns."
    echo "   This is ROADMAP-AUDIT F-D. It is why the D5 gate as first written"
    echo "   ('tools/ddc.sh prints W2 == the golden fixpoint byte-exact') is not"
    echo "   reachable from this tree without first reviving the witness."
    return 1
  fi
}

case "${1:---gate}" in
  --chain)   chain ;;
  --why)     why ;;
  --recut)   build_witness; measure recut ;;
  --measure) build_witness; smoke; echo; measure report ;;
  --full)    chain; echo; build_witness; echo; full_ddc ;;
  --gate|"") chain; echo; why; echo; build_witness; smoke; echo; measure report ;;
  *) echo "usage: tools/ddc.sh [--gate|--chain|--why|--measure|--recut|--full]"; exit 2 ;;
esac
