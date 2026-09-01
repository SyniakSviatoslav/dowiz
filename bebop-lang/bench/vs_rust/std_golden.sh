#!/usr/bin/env bash
# M7 std twin golden gate: compile with bebop.bin, run with seed,
# compare output against frozen expected values (no C compiler, no interp).
set -u
PASS=0; FAIL=0

run_test() {
  local f="$1" out_bin="$2"
  ./seed/build/seed bebop.bin compile "$f" "$out_bin" >/dev/null 2>&1 || return 1
  timeout 30 ./seed/build/seed "$out_bin" | tail -1
}

gate() {
  local name="$1" golden="$2" result="$3"
  if [ "$result" = "$golden" ]; then
    echo "PASS $name ($golden)"
    PASS=$((PASS+1))
  else
    echo "FAIL $name: golden=$golden got=$result"
    FAIL=$((FAIL+1))
  fi
}

# ---- checksum ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/checksum.bp /tmp/opencode/checksum_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/checksum_test.bin | tail -1)
gate checksum 96354 "$r"

# ---- sort ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/sort.bp /tmp/opencode/sort_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/sort_test.bin | tail -1)
gate sort 847859010857894 "$r"

# ---- rng ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/rng.bp /tmp/opencode/rng_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/rng_test.bin | tail -1)
gate rng -552671757612340580 "$r"

# ---- base64 (RFC 4648, packed 4-char words) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/base64.bp /tmp/opencode/base64_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/base64_test.bin | tail -1)
gate base64 1415005814517107508 "$r"

# ---- sha256 ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/sha256.bp /tmp/opencode/sha256_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/sha256_test.bin | tail -1)
gate sha256 65665208959391223 "$r"

# ---- crc32 (zlib_crc32 check value 0xCBF43926 for "123456789") ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/crc.bp /tmp/opencode/crc_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/crc_test.bin | tail -1)
gate crc32 3421780262 "$r"

# ---- hex (hex_encode of AB CD EF -> packed ASCII "abcdef") ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/hex.bp /tmp/opencode/hex_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/hex_test.bin | tail -1)
gate hex 107075202213222 "$r"

# ---- hv (Ф1 HDC core vs Rust golden: splitmix code/bind/bundle/permute/
#      hamming/popcount chain — bench/vs_rust/spectral_golden/golden.txt) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/hv.bp /tmp/opencode/hv_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/hv_test.bin | tail -1)
gate hv 4427592702613580868 "$r"

echo "std_golden: $PASS pass, $FAIL fail"
[ "$FAIL" = 0 ]