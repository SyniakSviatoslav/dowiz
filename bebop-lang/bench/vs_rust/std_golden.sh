#!/usr/bin/env bash
# M7 std twin golden gate: compile with bebop.bin, run with seed,
# compare output against frozen expected values (no C compiler, no interp).
ulimit -s 65536 2>/dev/null || true  # eval recursion: 113+ fn self-compile needs >8MB stack
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
# sha256("abc") = ba7816bf8f01cfea...ad; the frozen = fold(h[i]*31^..) of the
# TRUE digest (hashlib-verified 2026-09-01). The old frozen 65665208959391223
# was captured from the S0/S1-as-zero miscompile (the lost is_alpha A-Z fix).
gate sha256 -4000131497313522475 "$r"

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

# ---- spectral (SPECTRAL tier: topk_symmetric fp32 port vs Rust golden —
#      B6_bridge, k=3, 32 iters; frozen = total |λ_bp − λ_golden| fp dev,
#      re-baselined after the normalize_fp precision raise (>>14 -> >>8)) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/spectral.bp /tmp/opencode/spectral_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/spectral_test.bin | tail -1)
gate spectral 2038 "$r"

# ---- csr (Ф2: from_edges structural twin — fold over rp+ci+vv of the five
#      golden graphs; bench/vs_rust/spectral_golden/golden.txt CSR section) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/csr.bp /tmp/opencode/csr_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/csr_test.bin | tail -1)
gate csr -6945622865743784444 "$r"

# ---- bt (Ф2/F4: .bt rank-4 codec — pack/FNV/unpack/stride vs the Rust
#      golden byte stream; bench/vs_rust/spectral_golden/golden.txt .bt section) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/bt.bp /tmp/opencode/bt_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/bt_test.bin | tail -1)
gate bt -5708805812714944038 "$r"

# ---- cache (SS-6/Ф6 DecompCache falsifier: FNV key, 0 recomputes on
#      identical content, +1 on any change) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/cache.bp /tmp/opencode/cache_test.bin >/dev/null 2>&1 && sleep 1 && timeout 60 ./seed/build/seed /tmp/opencode/cache_test.bin | tail -1)
gate cache 38876254956 "$r"

echo "std_golden: $PASS pass, $FAIL fail"
[ "$FAIL" = 0 ]