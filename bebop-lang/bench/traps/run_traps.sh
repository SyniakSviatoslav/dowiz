#!/usr/bin/env bash
set -u

cd "$(dirname "$0")/../.." || exit 1
TMP=${TMPDIR:-/tmp}/traps-$$-${RANDOM}
mkdir -p "$TMP"

SEED=./seed/build/seed
[ -x "$SEED" ] || { echo "seed not executable: $SEED" >&2; exit 2; }
[ -s ./bebop.bin ] || { echo "bebop.bin missing or empty" >&2; exit 2; }

# Define trap expectations - code, documented_rc, documented_msg (for first line)
declare -A expected_rc
declare -A expected_msg

# Compile-time traps
expected_rc[97]="97"
expected_msg[97]="fn body without a tail expression"

expected_rc[100]="100"
expected_msg[100]="fn with more than 14 parameters"

expected_rc[101]="101"
expected_msg[101]="unbound symbol"

expected_rc[110]="110"
expected_msg[110]="invalid syntax in contract position"

expected_rc[111]="111"
expected_msg[111]="decimal point in numeric literal"

expected_rc[112]="112"
expected_msg[112]="module with contents"
# ROADMAP A18 sweep (2026-09-14): the lit_table (fntab[6000..6999]) overflow guard.
expected_rc[114]="114"
expected_msg[114]="too many string literals"

expected_rc[80]="80"
expected_msg[80]="arena exhausted"

expected_rc[87]="87"
expected_msg[87]="unresolved function"

expected_rc[102]="102"
expected_msg[102]="sys_ name inside a kernel fn"

expected_rc[82]="82"
expected_msg[82]="stack overflow"

expected_rc[109]="109"
expected_msg[109]="sys_clone"

expected_rc[88]="88"
expected_msg[88]="CAS module hash does not match"

expected_rc[89]="89"
expected_msg[89]="register collision"

expected_rc[90]="90"
expected_msg[90]="open failed"

expected_rc[96]="96"
expected_msg[96]="`++`"

expected_rc[99]="99"
expected_msg[99]="reserved word"

expected_rc[108]="108"
expected_msg[108]="sys_mapb"

# Counters
match_count=0
mismatch_count=0
not_triggered_count=0
total_count=0

report_trap() {
  local code="$1"
  local cc_rc="$2"
  local run_rc="$3"
  local expected_exit="$4"
  local observed_stderr="$5"
  local result="$6"

  total_count=$((total_count + 1))

  case "$result" in
    MATCH)
      match_count=$((match_count + 1))
      echo "TRAP $code expect_rc=$expected_exit got_rc=$run_rc MATCH \"$observed_stderr\""
      ;;
    MISMATCH)
      mismatch_count=$((mismatch_count + 1))
      echo "TRAP $code expect_rc=$expected_exit got_rc=$run_rc MISMATCH \"$observed_stderr\""
      ;;
    NOT_TRIGGERED)
      not_triggered_count=$((not_triggered_count + 1))
      echo "TRAP $code expect_rc=$expected_exit got_rc=$run_rc NOT_TRIGGERED \"$observed_stderr\""
      ;;
  esac
}

# Test compile-time traps
test_compile_trap() {
  local code="$1"
  local src="bench/traps/t${code}_*.bp"
  local src_file=$(ls $src 2>/dev/null | head -1)

  if [ -z "$src_file" ]; then
    echo "TRAP $code NOT_FOUND (no probe file)"
    not_triggered_count=$((not_triggered_count + 1))
    total_count=$((total_count + 1))
    return
  fi

  local out="$TMP/t${code}.bin"

  # Run compiler directly (not through cc.sh wrapper) to capture true exit code
  "$SEED" ./bebop.bin compile "$src_file" "$out" > "$TMP/cc_${code}.out" 2> "$TMP/cc_${code}.err"
  local cc_rc=$?

  local stderr=$(cat "$TMP/cc_${code}.err" 2>/dev/null | head -1)
  local expected="${expected_msg[$code]:-}"

  if [ $cc_rc -eq "${expected_rc[$code]:-999}" ]; then
    if [[ "$stderr" == *"${expected}"* ]] || [ -z "$expected" ]; then
      report_trap "$code" "$cc_rc" "$cc_rc" "${expected_rc[$code]}" "$stderr" "MATCH"
    else
      report_trap "$code" "$cc_rc" "$cc_rc" "${expected_rc[$code]}" "$stderr" "MISMATCH"
    fi
  else
    report_trap "$code" "$cc_rc" "$cc_rc" "${expected_rc[$code]}" "$stderr" "MISMATCH"
  fi
}

# Test runtime traps
test_runtime_trap() {
  local code="$1"
  local src="bench/traps/t${code}_*.bp"
  local src_file=$(ls $src 2>/dev/null | head -1)

  if [ -z "$src_file" ]; then
    echo "TRAP $code NOT_FOUND (no probe file)"
    not_triggered_count=$((not_triggered_count + 1))
    total_count=$((total_count + 1))
    return
  fi

  local out="$TMP/t${code}.bin"

  # Compile the probe
  "$SEED" ./bebop.bin compile "$src_file" "$out" > "$TMP/cc_${code}.out" 2> "$TMP/cc_${code}.err"
  local cc_rc=$?

  if [ $cc_rc -ne 0 ]; then
    report_trap "$code" "$cc_rc" "COMPILE_FAIL" "${expected_rc[$code]}" "$(cat "$TMP/cc_${code}.err" | head -1)" "NOT_TRIGGERED"
    return
  fi

  # Run the compiled binary
  timeout 30 "$SEED" "$out" 2> "$TMP/run_${code}.err" 1> "$TMP/run_${code}.out"
  local run_rc=$?

  local stderr=$(cat "$TMP/run_${code}.err" 2>/dev/null | head -1)
  local stdout=$(cat "$TMP/run_${code}.out" 2>/dev/null | head -1)

  if [ $run_rc -eq "${expected_rc[$code]:-999}" ]; then
    if [[ "$stderr" == *"${expected_msg[$code]}"* ]] || [ -z "${expected_msg[$code]}" ]; then
      report_trap "$code" "$cc_rc" "$run_rc" "${expected_rc[$code]}" "$stderr" "MATCH"
    else
      report_trap "$code" "$cc_rc" "$run_rc" "${expected_rc[$code]}" "$stderr" "MISMATCH"
    fi
  else
    report_trap "$code" "$cc_rc" "$run_rc" "${expected_rc[$code]}" "$stderr" "MISMATCH"
  fi
}

# Run tests
echo "=== Testing compile-time traps ==="
test_compile_trap 97
test_compile_trap 100
test_compile_trap 101
test_compile_trap 102
test_compile_trap 109
test_compile_trap 110
test_compile_trap 111
test_compile_trap 112
test_compile_trap 114

echo ""
echo "=== Testing additional compile-time traps ==="
test_compile_trap 88
test_compile_trap 96
test_compile_trap 99
test_compile_trap 108

echo ""
echo "=== Testing special-case traps ==="
# Test open failed: compile with non-existent source
TMP=${TMPDIR:-/tmp}/traps-$$-${RANDOM}
mkdir -p "$TMP"
"$SEED" ./bebop.bin compile /nonexistent-$$.bp "$TMP/nonexistent.bin" > "$TMP/cc_90.out" 2> "$TMP/cc_90.err"
cc_rc=$?
stderr=$(cat "$TMP/cc_90.err" 2>/dev/null | head -1)
if [ $cc_rc -eq 90 ]; then
  if [[ "$stderr" == *"open failed"* ]]; then
    echo "TRAP 90 expect_rc=90 got_rc=$cc_rc MATCH \"$stderr\""
    match_count=$((match_count + 1))
  else
    echo "TRAP 90 expect_rc=90 got_rc=$cc_rc MISMATCH \"$stderr\""
    mismatch_count=$((mismatch_count + 1))
  fi
else
  echo "TRAP 90 expect_rc=90 got_rc=$cc_rc MISMATCH \"$stderr\""
  mismatch_count=$((mismatch_count + 1))
fi
total_count=$((total_count + 1))

echo ""
echo "=== Testing runtime traps ==="
test_runtime_trap 80
test_runtime_trap 82
test_runtime_trap 87

echo ""
echo "trap_verify traps=$total_count match=$match_count mismatch=$mismatch_count not_triggered=$not_triggered_count"
rc=0
[ $mismatch_count -eq 0 ] && [ $not_triggered_count -eq 0 ] || rc=1
exit $rc
