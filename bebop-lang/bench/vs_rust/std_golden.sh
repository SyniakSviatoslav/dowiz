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

# ---- wht (N1 NEO-foundation: FWHT ADD/SUB butterfly — unit-vector dispatch
#      (e1/n8 Walsh row word=85) + self-inverse round trip (wht_pow2 then
#      wht_invert restores 8 cells exactly). 85001 = word*1000 + roundtrip_ok;
#      JIT==interp on both engines.) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/wht.bp /tmp/opencode/wht_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/wht_test.bin | tail -1)
gate wht 85001 "$r"

# ---- haar (N1b MULTITIER micro-tier: integer DWT — unit-vector dispatch
#      (e1/n8 Haarer row word=41) + exact inverse round trip (haar_pow2 then
#      haar_invert restores 8 cells losslessly). 41001 = word*1000 + ok;
#      branch-free ADD/SUB, no multiplies, no floats.) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/haar.bp /tmp/opencode/haar_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/haar_test.bin | tail -1)
gate haar 41001 "$r"

# ---- ntt (N1b MULTITIER meso-tier: number-theoretic transform over
#      Z_p, p = 998244353 = 2^23*119+1, primitive root 3 — finite-field
#      forward/inverse, exact END-equivalence of convolution. 141003 =
#      word*1000 + ok, word = sign binarization of the centered spectrum
#      of the ramp [1..8] (cells +36, +346334868, +201631260, +103943341;
#      oracle: independent Python NTT over the same modulus); ok =
#      roundtrip bit (ntt_inv(ntt(x)) == x, all 8 cells) + 2*conv bit
#      (NTT-multiply-then-invert circular convolution of ramp ⊛ reverse
#      == [176,156,144,140,144,156,176,204]).) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/ntt.bp /tmp/opencode/ntt_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/ntt_test.bin | tail -1)
gate ntt 141003 "$r"

# ---- store (Ф2/F4: .bt atomic-publish store — tmp -> sys_export ->
#      sys_rename(=renameat AT_FDCWD) -> read-back -> unpack vs golden;
#      fold 2245524994793680850, the SAME "BT4R" 220-byte stream bt.bp packs;
#      proof renameat publish round-trips byte-identically) ----
# ---- rev (N2 reversible/conservative logic: XOR-toggle, CNOT, Toffoli,
#      Fredkin are all self-inverse - bit-for-bit unwind without snapshots;
#      rev_round/rev_undo record deltas and restore the exact arena,
#      oracle-verified independently in Python over the same assertions) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/rev.bp /tmp/opencode/rev_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/rev_test.bin | tail -1)
gate rev 5092789399242 "$r"

r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/store.bp /tmp/opencode/store_test.bin >/dev/null 2>&1 && sleep 1 && timeout 60 ./seed/build/seed /tmp/opencode/store_test.bin | tail -1)
gate store 2245524994793680850 "$r"

# ---- petri (N4 bit-level Petri nets: marking bit-arrays, mark/get/clear
#      round-trip incl. bit 63/cell-1 places, branchless AND-mask pre-eval,
#      tzcnt deadlock (-1), lowest-enabled ordering on a 4-transition crossbar;
#      fold 61678606 = independent Python cell-faithful oracle) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/petri.bp /tmp/opencode/petri_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/petri_test.bin | tail -1)
gate petri 61678606 "$r"

# ---- lsm (N5 reservoir computing: xorshift64-built CSR reservoir (weights in
#      {-1,0,1}, leaky linear contraction -> Gershgorin spectral radius < 1,
#      echo-state verified by strict impulse decay m0>m1>m2); spike-driven
#      liquid; per-step FWHT sign-word decision prospects; fold -4383576415516299782
#      = independent Python oracle over the exact floor-div semantics) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/lsm.bp /tmp/opencode/lsm_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/lsm_test.bin | tail -1)
gate lsm -4383576415516299782 "$r"

# ---- holo (N6 holographic memory: message m[8]=[7,-3,5,-11,13,-17,19,-23]
#      WHT-dispersed into arena[32] as 4 copies; trims cut copy1 whole + copy3
#      cell 26; recovery = in-place FWHT(FWHT)/8; self-resolving consensus
#      (trust iff another copy agrees), winner = lowest-index max support;
#      invariants pf=15 (recovered==message, dead copy zero, damaged copy
#      mismatch), global-fingerprint dan=32 (m[0]+=1 moves ALL 32 picture
#      cells); recovered signs + xorshift64 stream spike the N5 liquid at
#      2^15 scale, per-spike FWHT sign-word and DC+Nyquist prospect folded
#      through *131; fold 2766693490590679850 = independent Python oracle
#      (xorshift LSR semantics, FWHT i64 wrap, floor-div step)) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/holo.bp /tmp/opencode/holo_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/holo_test.bin | tail -1)
gate holo 2766693490590679850 "$r"

echo "std_golden: $PASS pass, $FAIL fail"
[ "$FAIL" = 0 ]