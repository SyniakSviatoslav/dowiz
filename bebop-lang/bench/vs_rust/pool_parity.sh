#!/usr/bin/env bash
# M7 pool gate: compile with bebop.bin, run with seed,
# compare against frozen expected values (no interp).
set -u
mkdir -p "${BEBOP_TMP:-/tmp/opencode}"
SEED=./seed/build/seed
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}
[ -s "$BEBOP_BIN" ] || { echo "GUARD: BEBOP_BIN=$BEBOP_BIN missing or empty (L12)"; exit 1; }
PASS=0; FAIL=0
SCR=${BEBOP_TMP:-/tmp/opencode}/pool_scr.bp

# D4 root cause (2026-09-04): the old "ptrace sandbox skip" blamed proot for
# clone returning 0 in both threads; the real defect was the LEGACY compiler
# (selfhost/expr_compile.bp) - the same programs compiled by bebop.bin
# (T45 port of the sys_clone/futex/atomic builtins) run 4 real threads under
# this proot (raw C clone probe: 4/4, par_tids = 4). No skip: a FAIL is a FAIL.
ulimit -s 65536 2>/dev/null || true

# Frozen expected values
PAR_SUM_EXPECT=10000
PAR_MERGE_EXPECT=10000
PAR_TIDS_EXPECT=4

# The par_compile gates ask ONE question: do W threads compiling the same kernel produce the
# same code as ONE thread? So the reference is the SAME program with `par_compile(1, ...)` --
# same binary, same embedded compiler, same code path, only the thread count differs. Nothing
# outside the run is consulted.
#
# It used to be `4 * <words from bench/vs_rust/census.txt>`, and that was an ordering bug found
# 2026-09-08 on ROADMAP A2b: tools/battery.sh runs this lane BEFORE the invariants lane that
# re-freezes census.txt, so on any codegen change that SHRINKS a kernel the gate compared
# against the PREVIOUS binary's number and went RED on a good compiler (want=1268 = 4*317 vs
# seed=1236 = 4*309), then passed when the identical binary was re-tested after the freeze.
# A parity gate must not depend on whether some other lane has run yet, and under SERIAL=0 the
# lanes run concurrently, so no lane ORDER could have fixed it either.
#
# A first attempt at the fix compiled the kernel with $BEBOP_BIN itself and used that as the
# reference. That was wrong and the gate caught it: the pooled threads do not run $BEBOP_BIN,
# they run the compiler built from the CURRENT bebop.bp source (the program embeds it), i.e.
# gen2 of $BEBOP_BIN. On the promoted fixpoint the two agree and it looked fine; on the
# pre-A2b binary they differ (317 vs 309) and it failed. par_compile(1, ...) has no such gap.
#
# The kernel's SIZE is still watched, by the lanes whose job that is: census.py --check /
# --freeze-check in invariants.sh (census_allow.txt records any increase) and construct_parity's
# WORD_DELTA rows.
par_expect() {  # par_expect <scr-with-par_compile(4,...)> <out-name>  -> 4 * the 1-thread words
  local scr=$1 name=$2 t=${BEBOP_TMP:-/tmp/opencode} w
  sed 's/par_compile(4, p,/par_compile(1, p,/' "$scr" > "$t/${name}_serial.bp"
  ./seed/build/seed "$BEBOP_BIN" compile "$t/${name}_serial.bp" "$t/${name}_serial.bin" >/dev/null 2>&1 \
    || { echo "COMPILEFAIL ${name}_serial (1-thread reference)"; exit 1; }
  w=$(timeout 180 ./seed/build/seed "$t/${name}_serial.bin" | tail -1)
  case "$w" in ''|*[!0-9-]*) echo "BADREF ${name}_serial returned '$w'"; exit 1;; esac
  echo $(( 4 * w ))
}

# ---- gate 1: par_sum ----
cat selfhost/std/pool.bp > "$SCR"
cat >> "$SCR" <<'EOF'

fn main() -> i64 {
  par_sum(4, 1000)
}
EOF
./seed/build/seed "$BEBOP_BIN" compile "$SCR" ${BEBOP_TMP:-/tmp/opencode}/pool_sum.bin >/dev/null 2>&1 || { echo "COMPILEFAIL par_sum"; exit 1; }
SV=$(timeout 60 ./seed/build/seed ${BEBOP_TMP:-/tmp/opencode}/pool_sum.bin | tail -1)
[ "$SV" = "$PAR_SUM_EXPECT" ] && { echo "PASS par_sum (seed=$SV)"; PASS=$((PASS+1)); } || { echo "FAIL par_sum (seed=$SV want $PAR_SUM_EXPECT)"; FAIL=$((FAIL+1)); }

# ---- gate 1b: par_merge (atomic sys_atomic_add merge) ----
cat selfhost/std/pool.bp > "$SCR"
cat >> "$SCR" <<'EOF'

fn main() -> i64 {
  par_merge(4, 1000)
}
EOF
./seed/build/seed "$BEBOP_BIN" compile "$SCR" ${BEBOP_TMP:-/tmp/opencode}/pool_merge.bin >/dev/null 2>&1 || { echo "COMPILEFAIL par_merge"; exit 1; }
SV=$(timeout 60 ./seed/build/seed ${BEBOP_TMP:-/tmp/opencode}/pool_merge.bin | tail -1)
[ "$SV" = "$PAR_MERGE_EXPECT" ] && { echo "PASS par_merge (seed=$SV)"; PASS=$((PASS+1)); } || { echo "FAIL par_merge (seed=$SV want $PAR_MERGE_EXPECT)"; FAIL=$((FAIL+1)); }

# ---- gate 2: par_compile == 4 * serial count ----
awk '/^fn main\(/{skip=1} !skip{print} skip&&/^}/{skip=0}' bebop.bp > "$SCR"
cat selfhost/std/pool_compile.bp >> "$SCR"
cat >> "$SCR" <<'EOF'

fn main() -> i64 {
  let p = zeros(64);
  let _ = p[0] = 98;
  let _ = p[1] = 101;
  let _ = p[2] = 110;
  let _ = p[3] = 99;
  let _ = p[4] = 104;
  let _ = p[5] = 47;
  let _ = p[6] = 118;
  let _ = p[7] = 115;
  let _ = p[8] = 95;
  let _ = p[9] = 114;
  let _ = p[10] = 117;
  let _ = p[11] = 115;
  let _ = p[12] = 116;
  let _ = p[13] = 47;
  let _ = p[14] = 107;
  let _ = p[15] = 101;
  let _ = p[16] = 114;
  let _ = p[17] = 110;
  let _ = p[18] = 101;
  let _ = p[19] = 108;
  let _ = p[20] = 115;
  let _ = p[21] = 47;
  let _ = p[22] = 107;
  let _ = p[23] = 49;
  let _ = p[24] = 46;
  let _ = p[25] = 98;
  let _ = p[26] = 112;
  par_compile(4, p, 27)
}
EOF
./seed/build/seed "$BEBOP_BIN" compile "$SCR" ${BEBOP_TMP:-/tmp/opencode}/par_compile.bin >/dev/null 2>&1 || { echo "COMPILEFAIL par_compile"; exit 1; }
SV=$(timeout 120 ./seed/build/seed ${BEBOP_TMP:-/tmp/opencode}/par_compile.bin | tail -1)
WANT=$(par_expect "$SCR" par_compile)
[ "$SV" = "$WANT" ] && { echo "PASS par_compile (4 threads == 4 x the 1-thread k1 words: seed=$SV)"; PASS=$((PASS+1)); } || { echo "FAIL par_compile (want=$WANT seed=$SV)"; FAIL=$((FAIL+1)); }

# ---- gate 2b: par_compile(4, k7.bp) — k7 queries multi-core, identical outputs ----
awk '/^fn main\(/{skip=1} !skip{print} skip&&/^}/{skip=0}' bebop.bp > "$SCR"
cat selfhost/std/pool_compile.bp >> "$SCR"
cat >> "$SCR" <<'EOF'

fn main() -> i64 {
  let p = zeros(64);
  let _ = p[0] = 98;
  let _ = p[1] = 101;
  let _ = p[2] = 110;
  let _ = p[3] = 99;
  let _ = p[4] = 104;
  let _ = p[5] = 47;
  let _ = p[6] = 118;
  let _ = p[7] = 115;
  let _ = p[8] = 95;
  let _ = p[9] = 114;
  let _ = p[10] = 117;
  let _ = p[11] = 115;
  let _ = p[12] = 116;
  let _ = p[13] = 47;
  let _ = p[14] = 107;
  let _ = p[15] = 101;
  let _ = p[16] = 114;
  let _ = p[17] = 110;
  let _ = p[18] = 101;
  let _ = p[19] = 108;
  let _ = p[20] = 115;
  let _ = p[21] = 47;
  let _ = p[22] = 107;
  let _ = p[23] = 55;
  let _ = p[24] = 46;
  let _ = p[25] = 98;
  let _ = p[26] = 112;
  par_compile(4, p, 27)
}
EOF
./seed/build/seed "$BEBOP_BIN" compile "$SCR" ${BEBOP_TMP:-/tmp/opencode}/par_compile_k7.bin >/dev/null 2>&1 || { echo "COMPILEFAIL par_compile_k7"; exit 1; }
SV=$(timeout 180 ./seed/build/seed ${BEBOP_TMP:-/tmp/opencode}/par_compile_k7.bin | tail -1)
WANT7=$(par_expect "$SCR" par_compile_k7)
[ "$SV" = "$WANT7" ] && { echo "PASS par_compile_k7 (4 threads == 4 x the 1-thread k7 words: seed=$SV)"; PASS=$((PASS+1)); } || { echo "FAIL par_compile_k7 (want=$WANT7 seed=$SV)"; FAIL=$((FAIL+1)); }

# ---- gate 3: real-thread evidence (clone returns W child tids) ----
cat selfhost/std/pool.bp > "$SCR"
cat >> "$SCR" <<'EOF'

fn main() -> i64 {
  par_tids(4)
}
EOF
./seed/build/seed "$BEBOP_BIN" compile "$SCR" ${BEBOP_TMP:-/tmp/opencode}/pool_tids.bin >/dev/null 2>&1 || { echo "COMPILEFAIL par_tids"; exit 1; }
SV=$(timeout 60 ./seed/build/seed ${BEBOP_TMP:-/tmp/opencode}/pool_tids.bin | tail -1)
[ "$SV" = "4" ] && { echo "PASS threads (seed=$SV child-tids)"; PASS=$((PASS+1)); } || { echo "FAIL threads (seed=$SV want 4)"; FAIL=$((FAIL+1)); }

echo "pool_parity: $PASS pass, $FAIL fail"
[ "$FAIL" = 0 ]