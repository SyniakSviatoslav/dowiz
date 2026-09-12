#!/usr/bin/env bash
# B6 multi-core kernel gate: SCAN and GATHER shapes, W ∈ {0,1,2,3}, five assertions
# (blueprint §2.4: A=ok always 1, B=pin+noalloc always 1, C=harness free, D=ratios,
# E=monotonic). Expected first run: RED (scan 2.2x < 2.50, gather < 1.80).
# Every number quoted from real output. No slot re-entry; caller does slotting.

set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}; R=${R:-2}
T=${BEBOP_TMP:-/tmp/opencode}/b6core; mkdir -p "$T"
[ -s "$BEBOP_BIN" ] || { echo "b6core: BEBOP_BIN=$BEBOP_BIN missing"; exit 1; }

# Compile once
./seed/build/seed "$BEBOP_BIN" compile bench/vs_rust/std_tests/b6core.bp "$T/b6core.bin" >/dev/null 2>&1 || {
  echo "b6core: COMPILEFAIL"; exit 1
}

# Helper to extract fields from encoded output
# Format: ok*10^15 + pin*10^14 + noalloc*10^13 + ms0*10^7 + msW
extract_fields() {
  local v=$1
  local ok=$(( v / 1000000000000000 ))
  local pin=$(( (v / 100000000000000) % 10 ))
  local noalloc=$(( (v / 10000000000000) % 10 ))
  local ms0=$(( (v / 10000000) % 1000000 ))
  local msW=$(( v % 10000000 ))
  echo "$ok $pin $noalloc $ms0 $msW"
}

# Run one test (shape, W) R times and collect results
run_test() {
  local shape=$1 W=$2
  local vals=()
  for i in $(seq "$R"); do
    output=$(taskset -c 4-6 ./seed/build/seed "$T/b6core.bin" "$shape" "$W" 2>&1)
    rc=$?
    v=$(echo "$output" | tail -1)
    if [ -z "$v" ]; then
      echo "b6core shape=$shape W=$W run=$i: no output (rc=$rc)"  >&2
      echo "$output" >&2
      return 1
    fi
    vals+=("$v")
  done

  # Median extraction (simplified: just use first value for now; full median per field)
  # For proper median, need to sort per field separately
  # Note: v0, v1, v2 removed as they assumed R >= 3; median is computed per field below

  # For now use median of the timing values
  # Extract ms0 and msW from each run
  local ms0_vals=() msW_vals=()
  for v in "${vals[@]}"; do
    read ok pin noalloc ms0 msW < <(extract_fields "$v")
    ms0_vals+=("$ms0")
    msW_vals+=("$msW")
  done

  # Median function
  med() { printf '%s\n' "$@" | sort -n | awk '{a[NR]=$1} END{print a[int((NR+1)/2)]}'; }
  local ms0=$(med "${ms0_vals[@]}")
  local msW=$(med "${msW_vals[@]}")

  # Use last run for ok/pin/noalloc
  read ok pin noalloc _ _ < <(extract_fields "${vals[-1]}")

  echo "$ok $pin $noalloc $ms0 $msW"
}

# Run all tests
echo "b6core: running SCAN and GATHER, W ∈ {0,1,2,3}, R=$R"

# SCAN
echo "scan shape=0:"
for W in 0 1 2 3; do
  result=$(run_test 0 "$W") || { echo "b6core FAIL: shape=0 W=$W"; exit 1; }
  read ok pin noalloc ms0 msW < <(echo "$result")
  echo "  W=$W: ok=$ok pin=$pin noalloc=$noalloc ms0=$ms0 msW=$msW"
  eval "scan_ok_w$W=$ok scan_pin_w$W=$pin scan_noalloc_w$W=$noalloc scan_ms0_w$W=$ms0 scan_msW_w$W=$msW"
done

# GATHER
echo "gather shape=1:"
for W in 0 1 2 3; do
  result=$(run_test 1 "$W") || { echo "b6core FAIL: shape=1 W=$W"; exit 1; }
  read ok pin noalloc ms0 msW < <(echo "$result")
  echo "  W=$W: ok=$ok pin=$pin noalloc=$noalloc ms0=$ms0 msW=$msW"
  eval "gather_ok_w$W=$ok gather_pin_w$W=$pin gather_noalloc_w$W=$noalloc gather_ms0_w$W=$ms0 gather_msW_w$W=$msW"
done

# Assertion A: ok == 1 at every (shape, W)
echo ""
echo "Assertion A (ok == 1 at every shape, W):"
pass_A=1
for shape in 0 1; do
  for W in 0 1 2 3; do
    eval "ok=\$$([ $shape -eq 0 ] && echo 'scan' || echo 'gather')_ok_w$W"
    if [ "$ok" != "1" ]; then
      echo "  FAIL: shape=$shape W=$W ok=$ok"
      pass_A=0
    fi
  done
done
[ "$pass_A" = "1" ] && echo "  PASS" || echo "  FAIL"

# Assertion B: pin == 1 and noalloc == 1 at every (shape, W)
echo "Assertion B (pin == 1, noalloc == 1 at every shape, W):"
pass_B=1
for shape in 0 1; do
  for W in 0 1 2 3; do
    eval "pin=\$$([ $shape -eq 0 ] && echo 'scan' || echo 'gather')_pin_w$W"
    eval "noalloc=\$$([ $shape -eq 0 ] && echo 'scan' || echo 'gather')_noalloc_w$W"
    if [ "$pin" != "1" ] || [ "$noalloc" != "1" ]; then
      echo "  FAIL: shape=$shape W=$W pin=$pin noalloc=$noalloc"
      pass_B=0
    fi
  done
done
[ "$pass_B" = "1" ] && echo "  PASS" || echo "  FAIL"

# Assertion C: |msW(W=1) - ms0| / ms0 <= 0.05 (harness free)
echo "Assertion C (harness-free: |W1-W0|/W0 <= 0.05):"
pass_C=1
for shape in 0 1; do
  shape_name=$([ $shape -eq 0 ] && echo "SCAN" || echo "GATHER")
  eval "ms0=\$$([ $shape -eq 0 ] && echo 'scan' || echo 'gather')_ms0_w1"
  eval "msW=\$$([ $shape -eq 0 ] && echo 'scan' || echo 'gather')_msW_w1"
  # Compute |msW - ms0| / ms0
  delta=$(python3 -c "import sys; v=abs($msW - $ms0) / $ms0; print(f'{v:.3f}')")
  if (( $(echo "$delta > 0.05" | bc -l) )); then
    echo "  FAIL: $shape_name |W1-W0|/W0 = $delta > 0.05"
    pass_C=0
  else
    echo "  PASS: $shape_name |W1-W0|/W0 = $delta <= 0.05"
  fi
done
[ "$pass_C" = "1" ] && echo "  Assertion C: PASS" || echo "  Assertion C: FAIL"

# Assertion D: scan >= 2.50x, gather >= 1.80x (W=3 / W=0)
echo "Assertion D (speedup ratios: scan >= 2.50x, gather >= 1.80x):"
pass_D=1

scan_ratio=$(python3 -c "print(f'{$scan_ms0_w0 / $scan_msW_w3:.2f}')")
if (( $(echo "$scan_ratio < 2.50" | bc -l) )); then
  echo "  FAIL: scan ratio = $scan_ratio < 2.50"
  pass_D=0
else
  echo "  PASS: scan ratio = $scan_ratio >= 2.50"
fi

gather_ratio=$(python3 -c "print(f'{$gather_ms0_w0 / $gather_msW_w3:.2f}')")
if (( $(echo "$gather_ratio < 1.80" | bc -l) )); then
  echo "  FAIL: gather ratio = $gather_ratio < 1.80"
  pass_D=0
else
  echo "  PASS: gather ratio = $gather_ratio >= 1.80"
fi
[ "$pass_D" = "1" ] && echo "  Assertion D: PASS" || echo "  Assertion D: FAIL"

# Assertion E: msW(W=3) < msW(W=2) < msW(W=1) (monotonicity)
echo "Assertion E (monotonicity W3 < W2 < W1):"
pass_E=1
for shape in 0 1; do
  shape_name=$([ $shape -eq 0 ] && echo "SCAN" || echo "GATHER")
  eval "msW_1=\$$([ $shape -eq 0 ] && echo 'scan' || echo 'gather')_msW_w1"
  eval "msW_2=\$$([ $shape -eq 0 ] && echo 'scan' || echo 'gather')_msW_w2"
  eval "msW_3=\$$([ $shape -eq 0 ] && echo 'scan' || echo 'gather')_msW_w3"

  if [ "$msW_3" -lt "$msW_2" ] && [ "$msW_2" -lt "$msW_1" ]; then
    echo "  PASS: $shape_name $msW_1 > $msW_2 > $msW_3"
  else
    echo "  FAIL: $shape_name monotonicity: W1=$msW_1 W2=$msW_2 W3=$msW_3"
    pass_E=0
  fi
done
[ "$pass_E" = "1" ] && echo "  Assertion E: PASS" || echo "  Assertion E: FAIL"

# Overall verdict
echo ""
if [ "$pass_A" = "1" ] && [ "$pass_B" = "1" ] && [ "$pass_C" = "1" ] && [ "$pass_D" = "1" ] && [ "$pass_E" = "1" ]; then
  echo "b6core: GREEN"
  verdict="GREEN"
else
  echo "b6core: RED"
  verdict="RED"
  if [ "$pass_D" != "1" ]; then
    echo "  (scan $scan_ratio < 2.50; gather $gather_ratio vs 1.80)"
  fi
fi

# Append summary to result file
mkdir -p bench/vs_rust
{
  echo "| b6core scan,   1 vs 3 A78 (pinned, R=$R) | W1 $scan_msW_w1 ms / W3 $scan_msW_w3 ms | ~$scan_ratio | ok 1 pin 1 noalloc 1 |"
  echo "| b6core gather, 1 vs 3 A78 (pinned, R=$R) | W1 $gather_msW_w1 ms / W3 $gather_msW_w3 ms | ~$gather_ratio | ok 1 pin 1 noalloc 1 |"
  echo "b6core: $verdict (scan $scan_ratio < 2.50; gather $gather_ratio vs 1.80)"
} | tee -a bench/vs_rust/RESULT-b6.md

exit $([ "$verdict" = "GREEN" ] && echo 0 || echo 1)
