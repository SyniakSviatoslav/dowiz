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

# ---- scoord (SS-15 eigenvectors = the single coordinate system: concepts are
#      +-1 vectors; COORDINATES = spectral projections onto the topk eigenbasis
#      Q of the C8+I connection operator (dominant mode ~ constant vector ->
#      cyclic byte-shift leaves the DC coordinate invariant to fp error); search
#      = argmin(|dCoordinate|) over VALUES at any arena offset (no pointers);
#      layout-mirror proof = identical content at two offsets -> bit-exact equal
#      DC; orthonormality audit ob (offdiag Q^TQ residuals bounded). C8+I has a
#      SIMPLE max |lambda| so row 0 is the exact constant mode (C8 alone has a
#      +/-2 double eigenspace -> mixed basis). Fold 2010131 = python oracle
#      (mirror of fp topk at n=8,k=4,iters=64) bit-exact; identity 2/1, layout
#      1, rotation 3, orthonormality ok.) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/scoord.bp /tmp/opencode/scoord_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/scoord_test.bin | tail -1)
gate scoord 2010131 "$r"

# ---- sgamma (SS-16 eigenvalues = control-flow metrics: the spectral gap
#      gamma = lambda_1 - lambda_2 is the flow switch - connected P8+selfloop
#      gives gamma ~ 0.3473 (fp >>22 = 355, switch stays ON), two identical
#      P4+selfloop components give gamma ~ 0 (>>22 = 0, switch fires ->
#      graph disintegrates); the Fiedler vector evecs[1] sign-bipartitions
#      the line 4+/4- (work-stealing split). Self-loops REQUIRED: a pure
#      chain is bipartite (+/- symmetric spectrum) -> |lambda_1| = |-lambda_1|
#      freezes the power iteration at a mixed fixed point (observed lambda_1
#      1.87903 vs true 1.87939; lambda_2 unphysical, engine-dependent). The
#      +1 spectrum shift makes every |lambda| unique. Fold 3550431 = python
#      mirror bit-exact.) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/sgamma.bp /tmp/opencode/sgamma_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/sgamma_test.bin | tail -1)
gate sgamma 3550431 "$r"

# ---- tb (tokenbox: merged token-economy tool - rtk compressor + mempalace
#      search + content-address hashing in ONE str-free .bp binary; answers in
#      numbers only: tb h <path> = crc32 (== zlib bit-exact), tb ctx = corpus
#      digest, tb s <needle> <path> = line numbers, tb c = stdin dedup/trunc;
#      self-test fold 1111000 = crc32("123456789")==0xCBF43926 + empty-crc +
#      line_has + itoa checks; str literals/++ segfault in the .bin runtime
#      (R3 defect d, journal 1788288206) so tb is str-free by construction
#      (argv + cells + arithmetic only)) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/tb.bp /tmp/opencode/tb_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/tb_test.bin | tail -1)
gate tb 1111000 "$r"

# ---- seigtime (SS-17 eigentime: time = spectral iteration, not wall clock.
#      Synchronization = Hotelling iterations until the iterate enters an
#      EXACT cycle of the power map. Slow clock = C8+I ring (lambda2/lambda1
#      = 0.805): the normalize map is locally flat at the dominant constant
#      mode (c' ~ 2^32/sqrt(8) independent of c) so the rounding-locked
#      trajectory sawtooths into an exact period-30 cycle — first recurrence
#      td=123, absorb 16/16 (ab=1) -> e_slow=123301. Fast clock = J8 all-ones
#      (lambda2..8 = 0): exact period-1 fixpoint td=2, ab=1 -> e_fast=2011.
#      Time-scale separation e_slow >> e_fast, absorbing both. Detection =
#      ring history hist[240], shortest p in 1..30 with x_t == x_{t-p}, then
#      16-step cycle-membership absorb; e = td*1000 + per*10 + ab. Seed shift
#      (rng>>11)>>20 EMITS LSR (R3.b, loop-reassigned local) — oracle mirrors
#      unsigned per the shift law; first seed-sensitive gate (the eigentime
#      MEASURES the transient, unlike topk folds). Fold 1233012011 = es*10000
#      + ef = python oracle bit-exact.) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/seigtime.bp /tmp/opencode/seigtime_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/seigtime_test.bin | tail -1)
gate seigtime 1233012011 "$r"

# ---- srepl (SS-18 spectral self-replication: agent logic change = matrix
#      perturbation dA; spectral_drift(A0, A0+dA) -> DriftClass transition.
#      Base A0 = 0.25*(C8+I) (rho = 0.75, Damped class 0, gamma = 0.1465).
#      S1 within gamma: +0.01 on one self-loop -> drho = 0.00128 -> class
#      stable Damped->Damped (trans 0 = auto-fix / mmap snapshot regime).
#      S2 outside gamma: +0.4 on ALL self-loops -> rho = 1.15, 3 unstable
#      modes -> Damped->Unstable (trans 2 = .bt dump regime). Evolution =
#      pure spectral jumps, no textual recompilation. Fold 8449214 =
#      drho1q*100000 + trans2*10000 + unst2*1000 + trans1*100 + drho2q
#      (drhoq = drho >> 16) = python oracle bit-exact (topk seed shift
#      mirrored unsigned, R3.b).) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/srepl.bp /tmp/opencode/srepl_test.bin >/dev/null 2>&1 && timeout 60 ./seed/build/seed /tmp/opencode/srepl_test.bin | tail -1)
gate srepl 8449214 "$r"

# ---- sinc (SS-8 sinc(x)=sin(pi*x)/(pi*x) ideal interpolant, direct Taylor
#      series (no division): 1 - z2/3! + z4/5! - z6/7! + z8/9! - z10/11!,
#      z = pi*x in fp 2^32. Honest window |x|<=1 (fixed-point truncation):
#      sinc(0)=1.0 exact; sinc(1/2) = 2/pi to ~1e-8 (q05>>12 = 667544 vs
#      golden 667544.2); sinc(1) error 0.013% inside the 0.1% done-check
#      band (ok bit). Critical for Kalman (SS-1). Fold 6684880500081 =
#      q05*10^7 + q025*10^4 + e1q*10 + ok = python mirror bit-exact.) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/sinc.bp /tmp/opencode/sinc_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/sinc_test.bin | tail -1)
gate sinc 6684880500081 "$r"

# ---- kalman (SS-1 Kalman filter pure .bp: scalar-only fixed-point 1-D step
#      response, zero alloc. F=H=1.0, Q=0.001, R=0.01, z=5.0, 1000 iters
#      (done-check horizon). Riccati map reaches an EXACT fp fixpoint
#      (P1000==P999 -> fix=1 = 0 drift, fixed tick count by construction);
#      state tracks z inside the 0.1% band (err 3 fp units -> trk=1). Fold
#      28327900110011 = kq*10^8 + pq*10^4 + trk*10 + fix (kq=K>>12,
#      pq=P>>20) = python mirror bit-exact.) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/kalman.bp /tmp/opencode/kalman_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/kalman_test.bin | tail -1)
gate kalman 28327900110011 "$r"

# ---- calcbound (SS-5 calculus bounding: mean-value slope bounds on
#      f(x)=x^2-x give a bounding box for every golden mutation d in
#      {-1/8,-1/16,0,+1/16,+1/8} around x0=1.0: df in [min(fmin*d,
#      fmax*d)-eps, max(fmin*d,fmax*d)+eps], f'=2x-1 in [0.75,1.25],
#      eps=0.01 slack. Done-check: box CONTAINS all 5 actual results.
#      Fold 1024576000 = contained*10^9 + sum(|fi|>>16)*10^3 + (f0>>20)
#      = python mirror bit-exact (>> is logical - abs-first per shift law).) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/calcbound.bp /tmp/opencode/calcbound_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/calcbound_test.bin | tail -1)
gate calcbound 1024576000 "$r"

# ---- vecinv (SS-2 vector calculus as static invariants on the C8 ring:
#      div.grad==laplacian (row-sum of the stored flow == direct formula,
#      all 8 nodes), div.rot==0 for a skew circulation and survives node
#      relabel rotation (layout-invariant invariant); a broken asymmetric
#      edge leaks exactly 1 unit of divergence and the invariant fires.
#      Fold 1111018 = ident1*10^6 + ident2*10^5 + rot_ok*10^4 + caught*10^3
#      + div3*10 + lf0 = python mirror bit-exact.) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/vecinv.bp /tmp/opencode/vecinv_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/vecinv_test.bin | tail -1)
gate vecinv 1111018 "$r"

# ---- fir (SS-4 FIR as a structural ban on cyclic dependencies: forward-only
#      4-tap flow h={1,1/2,1/4,1/8}, fixed literal tap count = bounded masked
#      iteration (zero infinite-loop risk by construction). Impulse at each
#      lag reproduces the tap exactly; BIBO: all 16 worst-case sign patterns
#      |x|<=1 give |y| <= sum|h| = 15/8 exactly (equality at the aligned
#      pattern). Fold 11104857722880 = taps_ok*10^13 + bib_ok*10^12 +
#      sumq*10^5 + maxq (q=>>16, positives only - shift law) = python mirror
#      bit-exact.) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/fir.bp /tmp/opencode/fir_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/fir_test.bin | tail -1)
gate fir 11104857722880 "$r"

# ---- qlora (SS-7 QLoRA 4-bit agentic evolution: 8 strategy weights
#      quantized to 4-bit signed (round(|w|*8), error <= 1/16 all 8 - rt_ok);
#      live update = rank-1 adapter A[i]*B[i] at 2^-8 scale; updated state
#      re-quantizes and its packed FNV-64 content-address key CHANGES
#      (DecompCache invalidation on live hardware); adapter moves the
#      strategy output (moved). Fold 1116506000272 = rt_ok*10^12 +
#      moved*10^11 + invalid*10^10 + k0q*10^5 + ydeltaq (k0q=key0&65535,
#      ydeltaq=|ydelta|>>20) = python mirror bit-exact.) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/qlora.bp /tmp/opencode/qlora_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/qlora_test.bin | tail -1)
gate qlora 1116506000272 "$r"

# ---- bitmat (SS-12 bit matrices: switch/case -> parallel bit grids. The
#      dispatcher core = first-set-bit over an 8-bit condition flags word,
#      branch-free bit-grid reduction (idx = sum k*b_k*nf_k, nf = running
#      not-found mask) - the arithmetic the 23-builtin emit dispatcher
#      compiles to; fixed 8-step tick = the structural part of the <10-cycle
#      claim. Verified over ALL 256 flag patterns vs expected index (-1 when
#      empty; sum of outputs = 246). Fold 1000024600 = ok*10^9 + tot*100 =
#      python mirror bit-exact.) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/bitmat.bp /tmp/opencode/bitmat_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/bitmat_test.bin | tail -1)
gate bitmat 1000024600 "$r"

# ---- attn (SS-9 transformer attention via HDC: Hamming nearest-neighbour
#      over 64-bit hypervectors instead of softmax+float (s_j = 64 -
#      hv_pop1(Q^K_j), the vcnt path); winning key's VALUE bound by XOR
#      (bind tier). Q sits 3 bits from K2, >=19 from every other key ->
#      unique winner. Fold 2008568201 = win*10^9 + bestdist*10^6 +
#      (out&0xFFFF)*100 + uniq = python mirror bit-exact; hv_pop1 embedded
#      verbatim from hv.bp (gate hv).) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/attn.bp /tmp/opencode/attn_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/attn_test.bin | tail -1)
gate attn 2008568201 "$r"

# ---- lcres (SS-3 LC resonance as agent-loop timing, arithmetic core:
#      f0 = 1/(2*pi*sqrt(L*C)) in fp 2^32 for two tanks - (1/16,1) -> 2/pi
#      and (1/4,1/16) -> 4/pi with period pi/4, all three inside the 0.1%
#      band vs exact (ok bits). Restoring long division with integer-part
#      pre-loop (r<b invariant). Hardware jitter half (clock_ms + PID over
#      1000 real cycles) deferred: no clock syscall on the std gate surface.
#      Fold 1116675441335088 = ok1*10^15 + ok2*10^14 + okT*10^13 + f1q*10^7
#      + f2q = python mirror bit-exact.) ----
r=$(./seed/build/seed bebop.bin compile bench/vs_rust/std_tests/lcres.bp /tmp/opencode/lcres_test.bin >/dev/null 2>&1 && timeout 30 ./seed/build/seed /tmp/opencode/lcres_test.bin | tail -1)
gate lcres 1116675441335088 "$r"

echo "std_golden: $PASS pass, $FAIL fail"
[ "$FAIL" = 0 ]