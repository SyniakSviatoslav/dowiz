#!/usr/bin/env bash
# M6 pool gate: parallel work over the shared arena == serial reference,
# on BOTH engines (seed = real threads, interp = sequential emulation).
#   gate 1: par_sum(4,1000) == 10000
#   gate 1b: par_merge(4,1000) == 10000 (atomic sys_atomic_add merge)
#   gate 2: par_compile(4, k1.bp) == 4 * serial k1 word count (92)
#   gate 2b: par_compile(4, k7.bp) == 4 * serial k7 word count (1536)
#   gate 3: seed clone() returns 4 real child tids (multi-thread evidence)
set -u
BEBOPC=./native/build/bebopc
SEED=./seed/build/seed
SCR=/tmp/opencode/pool_scr.bp
PASS=0; FAIL=0

# ---- gate 1: par_sum ----
cat selfhost/std/pool.bp > "$SCR"
cat >> "$SCR" <<'EOF'

fn main() -> i64 {
  par_sum(4, 1000)
}
EOF
timeout 900 $BEBOPC compilewords selfhost/expr_compile.bp "$SCR" > /tmp/opencode/pool.full 2>/dev/null || { echo "COMPILEFAIL par_sum"; exit 1; }
IDX=$(awk '/^fn /{ if (/^fn main\(/) exit; c++ } END{ print c+0 }' "$SCR")
E=$(grep "^OFF" /tmp/opencode/pool.full | awk -v i="$IDX" '{ print $(i+3) }')
python3 seed/pack.py /tmp/opencode/pool.full $((E*4)) /tmp/opencode/pool_sum.bin >/dev/null
SV=$(timeout 60 $SEED /tmp/opencode/pool_sum.bin | tail -1)
IV=$(timeout 900 $BEBOPC run "$SCR" main 2>/dev/null | tail -1)
[ "$SV" = "10000" ] && [ "$IV" = "10000" ] && { echo "PASS par_sum (seed=$SV interp=$IV)"; PASS=$((PASS+1)); } || { echo "FAIL par_sum (seed=$SV interp=$IV want 10000)"; FAIL=$((FAIL+1)); }

# ---- gate 1b: par_merge (atomic sys_atomic_add merge) ----
cat selfhost/std/pool.bp > "$SCR"
cat >> "$SCR" <<'EOF'

fn main() -> i64 {
  par_merge(4, 1000)
}
EOF
timeout 900 $BEBOPC compilewords selfhost/expr_compile.bp "$SCR" > /tmp/opencode/poolm.full 2>/dev/null || { echo "COMPILEFAIL par_merge"; exit 1; }
IDX=$(awk '/^fn /{ if (/^fn main\(/) exit; c++ } END{ print c+0 }' "$SCR")
E=$(grep "^OFF" /tmp/opencode/poolm.full | awk -v i="$IDX" '{ print $(i+3) }')
python3 seed/pack.py /tmp/opencode/poolm.full $((E*4)) /tmp/opencode/pool_merge.bin >/dev/null
SV=$(timeout 60 $SEED /tmp/opencode/pool_merge.bin | tail -1)
IV=$(timeout 900 $BEBOPC run "$SCR" main 2>/dev/null | tail -1)
[ "$SV" = "10000" ] && [ "$IV" = "10000" ] && { echo "PASS par_merge (seed=$SV interp=$IV)"; PASS=$((PASS+1)); } || { echo "FAIL par_merge (seed=$SV interp=$IV want 10000)"; FAIL=$((FAIL+1)); }

# ---- gate 2: par_compile == 4 * serial count ----
SERIAL=$(timeout 900 $BEBOPC compilewords selfhost/expr_compile.bp bench/vs_rust/kernels/k1.bp 2>/dev/null | head -1)
cat selfhost/expr_compile.bp > "$SCR"
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
timeout 900 $BEBOPC compilewords selfhost/expr_compile.bp "$SCR" > /tmp/opencode/pc.full 2>/dev/null || { echo "COMPILEFAIL par_compile"; exit 1; }
IDX=$(awk '/^fn /{ if (/^fn main\(/) exit; c++ } END{ print c+0 }' "$SCR")
E=$(grep "^OFF" /tmp/opencode/pc.full | awk -v i="$IDX" '{ print $(i+3) }')
python3 seed/pack.py /tmp/opencode/pc.full $((E*4)) /tmp/opencode/pc.bin >/dev/null
WANT=$((SERIAL * 4))
SV=$(timeout 120 $SEED /tmp/opencode/pc.bin | tail -1)
IV=$(timeout 900 $BEBOPC run "$SCR" main 2>/dev/null | tail -1)
[ "$SV" = "$WANT" ] && [ "$IV" = "$WANT" ] && { echo "PASS par_compile (serial=$SERIAL x4: seed=$SV interp=$IV)"; PASS=$((PASS+1)); } || { echo "FAIL par_compile (serial=$SERIAL want=$WANT seed=$SV interp=$IV)"; FAIL=$((FAIL+1)); }

# ---- gate 2b: par_compile(4, k7.bp) — k7 queries multi-core, identical outputs ----
SERIAL7=$(timeout 900 $BEBOPC compilewords selfhost/expr_compile.bp bench/vs_rust/kernels/k7.bp 2>/dev/null | head -1)
cat selfhost/expr_compile.bp > "$SCR"
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
timeout 900 $BEBOPC compilewords selfhost/expr_compile.bp "$SCR" > /tmp/opencode/pc7.full 2>/dev/null || { echo "COMPILEFAIL par_compile_k7"; exit 1; }
IDX=$(awk '/^fn /{ if (/^fn main\(/) exit; c++ } END{ print c+0 }' "$SCR")
E=$(grep "^OFF" /tmp/opencode/pc7.full | awk -v i="$IDX" '{ print $(i+3) }')
python3 seed/pack.py /tmp/opencode/pc7.full $((E*4)) /tmp/opencode/pc7.bin >/dev/null
WANT7=$((SERIAL7 * 4))
SV=$(timeout 180 $SEED /tmp/opencode/pc7.bin | tail -1)
IV=$(timeout 900 $BEBOPC run "$SCR" main 2>/dev/null | tail -1)
[ "$SV" = "$WANT7" ] && [ "$IV" = "$WANT7" ] && { echo "PASS par_compile_k7 (serial=$SERIAL7 x4: seed=$SV interp=$IV)"; PASS=$((PASS+1)); } || { echo "FAIL par_compile_k7 (serial=$SERIAL7 want=$WANT7 seed=$SV interp=$IV)"; FAIL=$((FAIL+1)); }

# ---- gate 3: real-thread evidence (clone returns W child tids) ----
cat selfhost/std/pool.bp > "$SCR"
cat >> "$SCR" <<'EOF'

fn main() -> i64 {
  par_tids(4)
}
EOF
timeout 900 $BEBOPC compilewords selfhost/expr_compile.bp "$SCR" > /tmp/opencode/pt.full 2>/dev/null || { echo "COMPILEFAIL par_tids"; exit 1; }
IDX=$(awk '/^fn /{ if (/^fn main\(/) exit; c++ } END{ print c+0 }' "$SCR")
E=$(grep "^OFF" /tmp/opencode/pt.full | awk -v i="$IDX" '{ print $(i+3) }')
python3 seed/pack.py /tmp/opencode/pt.full $((E*4)) /tmp/opencode/pool_tids.bin >/dev/null
SV=$(timeout 60 $SEED /tmp/opencode/pool_tids.bin | tail -1)
IV=$(timeout 900 $BEBOPC run "$SCR" main 2>/dev/null | tail -1)
# seed: the kernel clone() created 4 real threads -> 4 nonzero child tids.
# interp: sequential emulation (sys_clone==0), no parent path -> 0.
[ "$SV" = "4" ] && [ "$IV" = "0" ] && { echo "PASS threads (seed=$SV child-tids interp=$IV)"; PASS=$((PASS+1)); } || { echo "FAIL threads (seed=$SV interp=$IV want 4/0)"; FAIL=$((FAIL+1)); }

echo "pool_parity: $PASS pass, $FAIL fail"
[ "$FAIL" = 0 ]
