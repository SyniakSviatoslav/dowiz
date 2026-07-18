# BLUEPRINT P-E — Network / Hardware / Crypto-in-Core (2026-07-17)

> **Phase P-E of the canonical CORE roadmap** (`docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md`
> §3), written against the §2 20-point contract. Wave-2 Fable planning pass.
> **Absorbs (does not re-derive):**
> `docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/14-BATCH5-network-hardware-findings.md`
> (**v2, target-corrected** — the top half; the Fly-scoped v1 in its appendix is SUPERSEDED) and
> `docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md`
> §C-A / wave item **W2-L5** (the AVX2 crypto-verify lane).
>
> **Center of gravity of this phase:** the AVX2 SIMD crypto-verify lane (§2–§3). Everything else in
> the network/hardware cluster is either an operator-gated overlay (§4), a restated rejection (§5),
> or an already-resolved item (§6).

---

## §0. Why this layer exists (context for a reader with zero session history)

Layer E is where the mesh meets the wire and the silicon. Its governing discovery — the single
fact that shapes every decision in this phase — is a **measured cost inversion**: on the receiving
node, verifying a message's two signature legs costs ~55–140 µs, while the *entire* kernel UDP
packet stack costs ~7 µs and a raw syscall ~0.18 µs. Crypto verification is **8–20× the whole
packet stack** (§1). Every instinct imported from datacenter networking — kernel-bypass (DPDK),
RDMA, NIC flow-steering — optimizes the ~7 µs and ignores the ~100 µs. So this layer's one real
build item is not a faster network path; it is a **faster verifier** (§2, the AVX2 SIMD
crypto-verify lane), and its longest rejection section (§5) exists to stop a future engineer from
re-litigating DPDK/RDMA on the old, wrong-target grounds.

The verifier work carries one non-negotiable constraint that a reader must hold before reading
§2: **the speedup may never come from batching signatures into a shared verdict.** That is not
caution — it is scar tissue. This month, bebop's own `verify_batch` accepted a *real* SSR-2020
mixed-order forgery (a small-order curve component invisible to the batch's combined equation),
and the fix was to re-verify every batch member singly, making batching 3.26× *slower* than
singles. So this layer draws a bright, tested line (§2.1): SIMD accelerates the *arithmetic inside
and across independent verifications*, and every verdict `out[i]` stays a pure function of input
`i` alone. "Parallel-independent-verify," never "batch-accept."

The rest of the cluster is smaller: an operator-gated hardware-attestation overlay that would
price Sybil-minting in real devices at the cost of admitting Google/Apple trust roots (§4, a
sovereignty tradeoff the operator rules on, not the code); the DPDK/RDMA rejection (§5); and a
table of already-resolved transport/hardware items (§6). The 2026-07-18 session fold-in (§14;
written as "§13" on its source branch, renumbered at merge) adds the wire-format and
forward-error-correction work that a later research pass routed to this layer.

---

## §1. Ground truth (verified THIS pass, live — contract item 1)

Every claim below was re-checked against the working tree this session, not inherited:

| Fact | Evidence (live this pass) |
|---|---|
| bebop2's Ed25519 backend is a hand-rolled, zero-dependency, scalar RFC-8032 implementation | `/root/bebop-repo/bebop2/core/src/sign.rs` (1226 lines): `pub fn verify(pubkey: &[u8; 32], msg: &[u8], sig: &[u8; 64]) -> bool` at `sign.rs:929`; canonical-S rejection (`S < L`) at `sign.rs:938-941`; final check `S·B == R + k·A` at `sign.rs:957-966`; RFC 8032 §7.1 KATs incl. a RED wrong-pk case at `sign.rs:986-1016` |
| bebop2's ML-DSA-65 backend is also from-scratch scalar | `/root/bebop-repo/bebop2/core/src/pq_dsa.rs`: `type Poly = [i32; 256]` (`pq_dsa.rs:93`), scalar `fn ntt` loop (`pq_dsa.rs:198`), signed Montgomery `montgomery_reduce` (`pq_dsa.rs:73-77`), `pub fn verify_internal_bytes(pk, msg, sig) -> bool` (`pq_dsa.rs:910`), SHAKE via `pq_kem::shake128/256` (`pq_dsa.rs:22-26`); ACVP FIPS-204 vector gate in `core/src/pq_dsa/acvp_tests.rs` |
| **No SIMD/AVX2 code path exists anywhere in bebop2 core today** | `grep -rn -i "avx2\|simd\|target_feature" /root/bebop-repo/bebop2/core/src/` → **zero matches** (run this session) |
| The crate is zero-dependency, `no_std`-capable, empty-import-wasm-gated | `core/Cargo.toml`: `[dependencies] # none.`; `std` is a default-on feature; `#![cfg_attr(not(feature = "std"), no_std)]` (`lib.rs:19`); empty-import wasm gate documented in `Cargo.toml` comments. `unsafe` is already present in the crate (bump allocator `lib.rs:46-67`, `rng.rs:268,321`) — no `forbid(unsafe_code)` barrier |
| Every mesh recv verifies BOTH signature legs, fully, per frame | `proto-wire/src/iroh_transport.rs:360-414` (`recv`): breach frames verify both legs directly (`:379`), all other frames go through the roster/hybrid gate (`:395-401`); the leg implementations are `proto-cap/src/signed_frame.rs:208` (`verify_classical` → `bebop2_core::sign::verify`) and `:229` (`verify_pq` → `pq_dsa::verify`) |
| Measured cost structure (doc 14 v2 §0.5, re-confirmed by synthesis §C-A this session) | Ed25519 verify ≈ 69–71 µs (openssl); per-recv hybrid crypto ≈ 55–140 µs; **entire** UDP TX+RX stack ≈ 7 µs; raw syscall ≈ 0.18 µs → **crypto is ~8–20× the whole packet stack** |
| Host ISA | `avx2 bmi2 aes sha_ni pclmulqdq` present; **no AVX-512 / VAES / IFMA** (synthesis §C-A, `[MEASURED]` `/proc/cpuinfo`) — AVX2 is the ceiling on this host and on realistic owner-hub consumer CPUs |
| Existing dispatch pattern to reuse | dowiz `kernel/src/simd.rs` (P11 §6 f64x4 SoA lane): `#[target_feature(enable = "avx2")]` at `simd.rs:58`, `std::is_x86_feature_detected!("avx2")` runtime dispatch at `simd.rs:164`, mirroring `householder.rs` — the repo's proven SIMD-with-scalar-fallback shape |
| Batch-accept is REJECTED with a real forgery in evidence | memory `crypto-safe-first-pass-2026-07-14.md` + bebop commit `6541ae8`: an Ed25519 `verify_batch` mixed-order **SSR-2020 forgery** (small-order component `R = R₀ + T` invisible to the small-order filter) was **actually accepted** by the pre-fix combined-equation batch; fixed by re-verifying every accept singly; batching measured **3.26× slower** than singles after the fix — the honest walk-back is in bebop `84a1e272d` |
| IssuanceBudget seam (P-D) | `docs/design/CORE-ROADMAP-2026-07-17/P-D-audit-root-delegation-policy.md:170-175` (reconstructed 2026-07-17; the lost version was `:131-135`): Option A (recommended) defines `IssuanceBudget { anchor_id, epoch, minted_count, max_per_epoch }` as a pure predicate at delegation-sign time; `:118,195` name attestation as an *additional pure precondition* slot |

---

## §2. The AVX2 SIMD crypto-verify lane (W2-L5) — the phase's one real build item

### §2.1 ZERO-AMBIGUITY DEFINITION — parallel-independent-verify is NOT batch-accept

This distinction is load-bearing and any agent executing this blueprint must hold it exactly.

**REJECTED (never build): batch-accept.** One *combined algebraic equation* over N signatures —
e.g. the random-linear-combination check `Σ zᵢ·(Sᵢ·B − Rᵢ − kᵢ·Aᵢ) = 0` — where a **single**
accept/reject verdict (or N verdicts derived from one shared equation) covers all N signatures.
The failure is not hypothetical: bebop's own `verify_batch` accepted a real SSR-2020 mixed-order
forgery this month (commit `6541ae8`, §1). Cross-signature algebra lets a forged signature hide
inside the linear combination. **No code path built under this blueprint may ever combine two
signatures' curve/lattice algebra into one equation, one accumulator, or one shared verdict.**

**ADOPTED (this blueprint): N independent full verifications, lane-parallel.** For every input
`i`, the complete, unabridged verification algorithm runs — for Ed25519 that is all of RFC 8032
§5.1.7 exactly as `sign.rs:929-967` does it today (canonical-S check, both point decompressions,
`Sᵢ·B == Rᵢ + kᵢ·Aᵢ` for *that* i alone); for ML-DSA-65 all of FIPS 204 Alg 8 exactly as
`pq_dsa.rs:910` does it today. SIMD is used **only to execute arithmetic faster**:

1. **Intra-verify SIMD** — vectorize the field/polynomial arithmetic *inside one* verification
   (parallel limb ops in one Edwards point operation; 8×i32 lanes across one poly's NTT
   butterflies; 4-way Keccak across the K×L=30 independent SHAKE streams of ONE ML-DSA matrix
   expansion). Nothing crosses a signature boundary because there is only one signature in flight.
2. **Lane-parallel independent verifies** — when N ≥ 2 inputs are queued, independent
   verifications may share SIMD registers (e.g. 4 independent SHAKE streams from 4 *different*
   signatures in one Keccak×4 state). The data of signature `i` and signature `j` may sit in
   adjacent SIMD lanes, but **no instruction ever combines them arithmetically**: each lane's
   output feeds only its own signature's own equation, and each verdict `out[i]` is a pure
   function of `(pk[i], msg[i], sig[i])` and nothing else.

**The falsifiable statement of the property** (this is what the tests in §3 pin):

> For all inputs, `verify_many(reqs)[i] == verify(reqs[i].pubkey, reqs[i].msg, reqs[i].sig)`
> — bit-identical to the scalar single-verify verdict, for every `i`, on every CPU,
> in every combination of valid/forged neighbors.

A corollary that batch-accept can never satisfy and this design satisfies by construction:
**changing any *other* element of the batch can never change verdict `i`** (tested in §3.3-T4).

### §2.2 Spec first — types & constants (contract items 3, 4; spec precedes test precedes code)

New types, defined before any implementation, in `bebop2/core/src/sign.rs` (Ed25519) and
`bebop2/core/src/pq_dsa.rs` (ML-DSA) — matching the crate's existing plain-fn/array style:

```rust
/// One independent Ed25519 verification request. Borrowed, zero-copy.
pub struct VerifyReq<'a> {
    pub pubkey: &'a [u8; 32],
    pub msg: &'a [u8],
    pub sig: &'a [u8; 64],
}

/// Number of independent Keccak-f[1600] streams interleaved in one AVX2 state.
/// 4 × 25 u64 lanes = 25 × __m256i. Fixed by the ISA, not tunable.
pub const KECCAK_X4_LANES: usize = 4;

/// i32 coefficients per __m256i in the vectorized NTT (256-coeff Poly = 32 vectors).
pub const NTT_AVX2_LANES: usize = 8;
```

Verdicts stay `bool` — the crate's existing verdict type (`sign.rs:929`, `pq_dsa.rs:910`).
Introducing an enum here would create a second verdict representation of the same concept
(Hermetic P2 violation); `bool` with fail-closed `false` is the existing single mechanism.

### §2.3 Exact function signatures (the contract other agents build against)

```rust
// ── bebop2/core/src/sign.rs ──────────────────────────────────────────────────
/// Verify N signatures, EACH fully and independently (RFC 8032 §5.1.7 per item).
/// out[i] is a pure function of reqs[i] alone; SIMD accelerates arithmetic only
/// and never mixes two signatures' algebra. Scalar-fallback on non-AVX2 targets
/// with bit-identical verdicts. NOT batch-accept (see blueprint §2.1).
pub fn verify_many(reqs: &[VerifyReq<'_>]) -> Vec<bool>;

// ── bebop2/core/src/pq_dsa.rs ────────────────────────────────────────────────
/// ML-DSA-65 counterpart: FIPS 204 Alg 8 per item, independently. Same contract.
pub fn verify_internal_bytes_many(reqs: &[(&[u8], &[u8], &[u8])]) -> Vec<bool>;
```

Internal split (the mechanism that makes the parity test possible — §2.6):

```rust
// Both files, pub(crate) so the parity/adversarial tests can target each path
// explicitly instead of relying on runtime dispatch:
pub(crate) fn verify_scalar(req: &VerifyReq<'_>) -> bool;      // = today's verify(), unchanged
#[cfg(all(feature = "std", target_arch = "x86_64"))]
pub(crate) fn verify_many_avx2(reqs: &[VerifyReq<'_>]) -> Vec<bool>; // the new lane
```

`verify()` (`sign.rs:929`) and `verify_internal_bytes()` (`pq_dsa.rs:910`) are **not modified**:
they remain the scalar reference implementations and the N=1 entry points. `verify_many`
dispatches: AVX2 detected → `verify_many_avx2`, else a plain loop over `verify_scalar`.

### §2.4 The two SIMD levers, in build order (from synthesis §C-A, measured/researched)

1. **ML-DSA-65 first — the bigger lever.** Dilithium's verify cost is NTT (256-coeff poly-mul
   over the 23-bit prime Q, `pq_dsa.rs:198`) + SHAKE sampling (matrix Â expansion = K×L = 30
   independent SHAKE128 streams). Both are embarrassingly SIMD-friendly: the pq-crystals AVX2
   baseline reaches ~70 µs verify, and the NTT itself gains **2.4–2.5×** from instruction-level
   tuning (§C-A, `[RESEARCHED — eprint 2026/1272]`). Implementation: vectorize `ntt`/`invntt`/
   `montgomery_reduce` 8×i32 per `__m256i` (`_mm256_mul_epi32`-based signed Montgomery, same
   algebra as `pq_dsa.rs:73-77`), and a 4-way interleaved Keccak-f[1600] (25 × `__m256i`) for
   ExpandA — intra-signature parallelism, verdict independence untouched by construction.
2. **Ed25519 second — the smaller lever.** Vectorize the GF(2^255−19) limb arithmetic inside one
   scalar-mult (parallel Edwards formulas), the curve25519-dalek-avx2 technique: ~1.5–2× on a
   single verify (§C-A). The current `sign.rs` limb layout (`[u64; 4]` schoolbook + 2^255≡19 fold,
   `sign.rs:66-140`) is the algebra to vectorize — same values, wider execution.

Honest arithmetic carried over from §C-A: even at maximal realistic SIMD (~2× both legs →
per-recv ~70 µs → ~35–70 µs), **crypto remains ~10× the whole packet stack**. This lane attacks
the cost that dominates; it does not (and is not claimed to) change the §5 rejections.

### §2.5 Scalar fallback — the bit-identical requirement (contract item 6 of the standard's DoD)

- **Dispatch:** `#[cfg(all(feature = "std", target_arch = "x86_64"))]` +
  `std::is_x86_feature_detected!("avx2")` at runtime — the exact `kernel/src/simd.rs:164` /
  `householder.rs` pattern, reused not re-invented (contract item 19).
- **Every other build takes the scalar path unconditionally:** `no_std` builds (the macro needs
  `std`), `wasm32-unknown-unknown` (the empty-import gate is untouched — the AVX2 module is
  cfg'd out entirely), ARM phones, pre-AVX2 x86. The scalar path is not a degraded mode; it is
  the *reference semantics* (Hermetic P1: the RFC/FIPS spec + KATs are the source of truth, and
  the scalar implementation is its pinned derivation).
- **Bit-identity is structurally cheap here** (unlike the float lanes of P11 §6): every operation
  is exact integer arithmetic — limb mul/add mod p, i32 Montgomery mod Q, Keccak XOR/rotate.
  There is no rounding, no reassociation hazard; vectorizing exact integer ops cannot change
  values, only their scheduling. The verdict is a boolean. So "bit-identical verdicts" is
  achievable exactly and is *asserted*, not hoped (§2.6).

### §2.6 The bit-identity GUARANTEE mechanism (Hermetic P2: parity-pinned divergence)

Hermetic P2 (verified statement, `HERMETIC-ARCHITECTURE-PRINCIPLES.md` §P2): where divergence
from one-implementation is forced, "the divergent implementations **must be pinned to each other
by a parity check** so they cannot drift." The AVX2 lane is precisely a forced divergence (the
forcing reason: measured 8–20× crypto dominance, §1). The pin has three layers:

1. **Verdict parity (the contract):** a permanent test asserts
   `verify_many_avx2(corpus)[i] == verify_scalar(corpus[i])` for every element of a corpus that
   includes: all RFC 8032 §7.1 KATs, the vendored ACVP FIPS-204 vectors (`pq_dsa/acvp_tests.rs`),
   and a deterministic PRNG-driven corpus (fixed seed, ≥10⁴ cases) of valid, bit-flipped,
   wrong-key, non-canonical-S (`S ≥ L`), low-order-point, and truncated inputs. Because both
   paths are exposed `pub(crate)` (§2.3), the test calls each **directly** — it never depends on
   dispatch behaving.
2. **Intermediate-value parity (drift caught at the first divergent limb, not the final bool):**
   `ntt_avx2(p) == ntt(p)` element-exact (`[i32; 256]` equality) and the AVX2 field-mul equal to
   scalar `fe_mul` limb-exact, over the same PRNG corpus. A scheduling bug surfaces as a named
   intermediate mismatch, not as a mysteriously flipped verdict.
3. **CI-time enforcement (the "smart index", contract item 14):** the parity tests run in the
   normal `cargo test` suite on the AVX2-capable CI/dev host — a semantic drift between the two
   implementations becomes a **CI-time failure**, never a runtime surprise. A build whose SIMD
   lane diverges from scalar is unshippable. This is the Self-Termination form of contract item
   13: an invariant boundary (test-gated unrepresentability of a shipped divergent lane), not a
   supervisor's runtime decision. The runtime fallback (detect → scalar) is a total function —
   there is no error path in dispatch, hence nothing to "handle."

---

## §3. TDD plan — RED → GREEN sequence (contract items 2, 3, 5, 10, 17)

Order is binding: spec (§2.2 types) → tests (this section, written first, RED) → implementation.
Verification is a pure predicate, so there is no event fold to model (`decide`-law N/A here,
stated honestly); the event-driven leg is the telemetry counter in §3.4.

### §3.1 RED state (before any implementation)

- T1 `verify_many` / `verify_internal_bytes_many` referenced by the parity test → **compile-RED**.
- Write T2–T6 below; all RED (missing symbols) or vacuous until the lane lands.

### §3.2 DoD — the benchmark test (GREEN gate, contract item 10)

New: `bebop-repo/bebop2/core/benches/verify_lane.rs` (new `benches/` dir; plain
`std::time::Instant` harness — **zero new dependencies**, keeping `[dependencies] # none.`).
Measures median + p99 ns/verify for: scalar Ed25519, AVX2 Ed25519, scalar ML-DSA-65, AVX2
ML-DSA-65, at N ∈ {1, 4, 32, 256}.

**DoD (falsifiable, machine-checkable):**
- Parity tests (§2.6 layers 1–2) GREEN on the AVX2 host **and** GREEN with the AVX2 path cfg'd
  off (scalar-only run) — proving the suite doesn't silently depend on the lane.
- Benchmark records a **measured** speedup: ML-DSA-65 AVX2 ≥ 1.5× scalar at N ≥ 4 (the §C-A
  research floor with margin; the 2.4–2.5× figure is the NTT alone, not whole-verify — do not
  gate on it). Ed25519 AVX2 ≥ 1.3× scalar. Numbers go into this blueprint's DoD table and the
  regression ledger entry — a real number, not an estimate. The speedup assertion lives in the
  bench binary (run explicitly), **not** in `cargo test` (perf assertions in unit CI are flaky;
  correctness assertions are not — split accordingly).
- `wasm32-unknown-unknown --no-default-features` build still produces an **empty import section**
  (the existing reloop-v2 gate re-run — proving the lane is fully cfg'd out of wasm).

### §3.3 Adversarial tests (contract item 5 — designed to break the invariant)

- **T2 — forged-signature-through-SIMD (the headline adversarial case):** take the RFC 8032 §7.1
  vector, flip one bit of `sig`, and assert `verify_many_avx2` **rejects** it — proving
  parallelism did not weaken verification. Same for ML-DSA with a bit-flipped ACVP signature.
- **T3 — SSR-2020 regression (named, permanent):** reconstruct the mixed-order forgery class that
  bebop's batch-accept actually accepted pre-`6541ae8` (`R = R₀ + T` with `T` small-order, S
  crafted to satisfy the *combined* equation) and assert the lane rejects it at its own index.
  Because each lane runs the full §5.1.7 check independently, this MUST reject; if it ever
  passes, cross-signature algebra has been smuggled in. Ledger name: `REG-P-E-ssr2020-lane`.
- **T4 — one-bad-in-batch / no-cross-contamination (the property batch-accept cannot have):**
  N−1 valid + 1 forged at index k → assert `out[k] == false` and `out[i] == true` for all i≠k;
  then permute k across all positions and N ∈ {2, 4, 5, 64} (including non-multiple-of-lane-width
  N to hit the remainder path). Also the dual: 1 valid among N−1 forged → the valid one accepts.
- **T5 — non-canonical S and invalid-point encodings through the AVX2 path** (the `sign.rs:938`
  early-rejects must fire identically in the lane; asserted against scalar per §2.6-1).
- **T6 — intentionally-failing sentinel:** a `#[should_panic]`/assert-fail test that feeds the
  parity harness a deliberately divergent stub (a scalar impl with the S-canonicity check
  removed) and asserts the harness **catches** it — testing the test, per the operator's
  "designed to literally break everything" rule.

### §3.4 Telemetry hook (contract item 10, second half)

Node-local counters (P24 native-telemetry ring pattern, `BLUEPRINT-NATIVE-TELEMETRY-LATENCY-
EXPLAINABLE-EVENTS-2026-07-17.md`): `verify_lane_dispatch{simd|scalar}`, `verify_lane_reject`
count, and a p99 verify-duration gauge — so a regression (or a dispatch silently falling back to
scalar on a host that should have AVX2) shows up automatically, not at review time.

---

## §4. Hardware attestation — ADOPT-AS-AUGMENTATION, **operator-gated overlay** (NOT auto-adopted)

Verdict inherited from doc 14 v2 item 5 (the flip: courier phones DO have StrongBox / Secure
Enclave — the v1 "no TPM in Firecracker" premise was wrong-target). This blueprint **designs the
seam and stops**: adoption requires an explicit operator decision, because it changes the trust
topology.

### §4.1 The decision the operator must make (stated, not made here)

Verifying an attestation means **accepting Google's and Apple's attestation roots into the
mesh's verification path** — for the attestation *evidence only*, never for authorization (which
stays anchor-rooted, `roster.rs:252-316`, Batch-7 C3). For a mesh whose posture is
no-external-dependency / offline-buildable / anchor-only trust, that is a real sovereignty
tradeoff: gain = Sybil-minting priced at real-device cost (a *tax*, not a *wall* — doc 14 v2
caveat 4); cost = two corporate trust roots + the keybox-leak history (caveat 3: prefer
StrongBox + RKP-provisioned attestation; treat pre-RKP TEE as lower-assurance). **⚠ OPERATOR
GATE: no code beyond the types below is built until the operator rules on this tradeoff.**

### §4.2 Where it plugs in — P-D's IssuanceBudget (cross-reference, contract item 7)

Per `P-D-audit-root-delegation-policy.md:131-135` (Option A) and `:107,217`: attestation slots
into the `RootDelegationPolicy` / `IssuanceBudget { anchor_id, epoch, minted_count,
max_per_epoch }` predicate as an **additional pure precondition checked at delegation-sign
time**. Proposed degrade-closed shape (resolving doc 14 v2's "must never brick a courier when
Google Play Integrity is down" against degrade-closed discipline):

- Split the budget: `max_per_epoch_unattested` (small baseline) vs `max_per_epoch_attested`
  (larger). Valid attestation evidence unlocks the larger budget for *minting new identities*.
- Attestation unavailability (root unreachable, service down) **never revokes or blocks existing
  capabilities** and never blocks renewal — it only holds *new* minting to the unattested
  baseline. Failure closes toward "fewer new identities," never toward "working courier bricked."
  This is the P4-conformant polarity: one budget mechanism, two named poles, collapse direction
  safe (Hermetic P4).

### §4.3 Types (design-only until the gate opens; target file `bebop2/proto-cap/src/attestation.rs`)

```rust
pub enum AttestationPlatform { AndroidKeyAttestation, IosAppAttest }
pub enum AttestedSecurityLevel { StrongBox, Tee, SecureEnclave }  // Software = not attested, unrepresentable
pub struct AttestationEvidence {
    pub platform: AttestationPlatform,
    pub security_level: AttestedSecurityLevel,
    pub challenge: [u8; 32],          // bound to a fresh mesh nonce at request time
    pub cert_chain_der: Vec<Vec<u8>>, // verified hub/anchor-side only, never in core
}
```

`Software` is deliberately not a variant: a software-level attestation *cannot be represented* as
attested evidence — the unsafe state is unreachable by type structure (contract item 6), not
filtered by a runtime check. X.509 chain verification runs **anchor/hub-side only** (verifier
role), never inside zero-dep `bebop2-core` — no new dependency enters the crypto core.

### §4.4 Settling probe (verbatim inherit, `[NEEDS-REAL-TARGET-HARDWARE]`)

Doc 14 v2 item 5 names it exactly: Android `KeyGenParameterSpec.Builder(...)
.setIsStrongBoxBacked(true).setAttestationChallenge(nonce)` → verify chain to Google's hardware
root, assert `securityLevel == StrongBox`, bind challenge to a fresh mesh nonce; iOS
`DCAppAttestService.generateKey`/`attestKey` → verify against Apple's App Attest root, bind the
key-id, consult the Risk Metric. Until that runs on real devices the flip stays
research-grounded, not probe-proven — stated honestly, as the source doc does.

Mesh/payload budget (contract item 12): attestation evidence is a **delegation-request-time
payload** (~1–3 KB DER chain), sent once per identity mint to an anchor — not per-frame, not
gossiped; the per-recv hot path (§2) carries zero new bytes.

---

## §5. DPDK / RDMA — REJECTED, and the reason is CRYPTO DOMINANCE (read this before ever re-litigating)

**The old rejection reason is DEAD. Do not cite it, do not re-argue against it.** The v1 batch
rejected DPDK/RDMA as "the hardware does not exist in Fly Firecracker microVMs." Fly is not the
target; the owner hub is real rooted hardware where a 2-NIC DPDK setup is *possible* and the
"single shared NIC" objection **dissolves** (doc 14 v2 item 2). The rejection survives anyway,
on the measured, target-correct grounds:

1. **Crypto-verification dominance — the governing measured ratio (doc 14 v2 §0.5, `[PROBED]`):**
   per-recv hybrid crypto ≈ 55–140 µs vs ≈ 7 µs for the **entire** kernel packet stack —
   **8–20×**. Kernel-bypass's hard ceiling is removing that 7 µs; the syscall tax it mostly
   targets is ~0.18 µs (<0.4% of per-message cost). This ratio lives between two costs on the
   *receiving node* — it holds on ideal bare metal and survives the §2 SIMD lane (post-SIMD
   crypto is still ~10× the stack, synthesis §C-A). **DPDK/RDMA optimize the wrong ~10%.**
2. **Weakest-endpoint rule (structural, target-independent):** both endpoints must have the
   capability; the mesh's other endpoint is always a sandboxed phone on LTE/5G. There is no DPDK
   and no RDMA path to a phone, ever.
3. **RDMA specifically contradicts the security model:** RDMA's value is bypassing the remote
   CPU — but every recv MUST run both signature legs on that CPU (`iroh_transport.rs:360-414`,
   `RequireBoth`). You cannot RDMA-write past the verifier without discarding the authenticity
   model. Plus: consumer hub NICs have no verbs, and RoCE needs a lossless DCB/PFC fabric —
   datacenter rack networking, not a depot.

**Un-reject trigger (falsifiable, from doc 14 v2):** a profile on real hub hardware showing
*transport, not crypto*, dominating hub↔hub latency — which §0.5 predicts will not appear, and
which the §2 lane makes even less likely (it lowers the crypto bar toward, but not to, the
transport cost). Anyone reopening this item must bring that measurement, not an argument.

---

## §6. The rest of the network/hardware cluster — resolved, deferred, or adopted elsewhere

| Item | Status (inherited, doc 14 v2 / synthesis §C-B — not re-derived) | This phase's action |
|---|---|---|
| LAN-local UDP/mDNS discovery + direct-subnet QUIC | **ADOPT** (the phone-capable realization of the co-located-WiFi intent; raw-L2 framing stays REJECT: phones can't emit raw frames + cleartext-header regression) | The one transport build item; extends `discovery.rs` anti-entropy gossip; envelope/signing untouched (`framing.rs` carrier-neutral by design). Schedule after W2-L5 — it needs a second live node to matter; the crypto lane doesn't |
| eBPF/XDP on the owner hub | **AVAILABLE (proved loadable, `[PROBED]`) — DEFER-WITH-TRIGGER** (no measured packet-steering need; sovereign-toolchain cost; phones can't) | No build. Trigger stays: a measured pps problem on the hub |
| RSS / flow steering | REJECT-on-virtio / DEFER-on-real-NIC | No build |
| NUMA pinning | **RESOLVED — no-op on the realistic single-socket hub**; the useful placement is L2-per-core affinity = **already covered by P25 CORE-BOUND core pinning** (`core_pinning.rs`), per the Step-2B/§C-B resolution. Dual-socket remains a procurement gate flagged to the operator | No build; nothing beyond what P25 owns |
| io_uring | DEFER — storage not network (measured: *loses* at batch=1, 417 ns vs 181 ns raw syscall) | No build; trigger = syscall-bound local file-I/O measurement on the arena/block-store |

---

## §7. Safety / hazard section (contract item 6 — argued from structure, not policy)

- **Unsafe state "batch-accept" is unreachable by construction:** the API (§2.3) has no combined
  verdict — `Vec<bool>` indexed 1:1 with inputs; there is no accumulator type spanning two
  signatures anywhere in the lane; and §2.6's parity pin equates every `out[i]` with the scalar
  single-verify, so any smuggled cross-signature algebra breaks CI (T3/T4 make the two known
  smuggling shapes explicit RED tests). The argument is type-structure + a pinned equation, not
  a review promise.
- **`unsafe` confinement:** all intrinsics live in the two new cfg-gated modules (§10), each
  block SAFETY-annotated with the exact `is_x86_feature_detected` precondition (the
  `kernel/src/simd.rs:166` comment pattern); the modules are compiled out of `no_std`/wasm
  builds entirely, so the empty-import wasm gate (§1) is structurally unaffected.
- **Isolation/bulkhead (contract item 11):** the lane is CPU-bound work on the P25 core-bound
  cores; its only failure classes are (a) wrong verdict — killed at CI by the parity pin, (b)
  absent ISA — a total-function dispatch to scalar. No shared mutable state, no new allocation
  on the hot path beyond the existing per-verify `Vec`s (`sign.rs:951`).
- **Rollback/fallback as math (contract item 13):** fallback = the retained scalar reference
  (redundant implementation + parity detection = error-*detecting* redundancy at build time);
  Self-Termination = the CI invariant boundary (§2.6-3). No Snapshot-Re-entry claim — nothing
  here is stateful.

## §8. Scaling, honest N/As, and discipline tags (contract items 8, 9, 15, 16)

- **Scaling axis (item 8):** `verify_many` scales in N signatures/call; memory is O(N) verdicts +
  O(1) SIMD state. Shape changes when a single drain tick queues N large enough that verify
  latency exceeds the drain budget — at that point chunk through the existing
  `bounded_drainer.rs`/`budget.rs` degrade-closed pattern (named seam, no new mechanism).
  Attestation budget scales per anchor×epoch — trivially small.
- **Linux-discipline verdict (item 9, framework reused):** **EXTENDS** — per-ISA runtime dispatch
  with a portable reference implementation is exactly the Linux `arch/` crypto-glue discipline;
  the scalar-as-reference rule is **ALREADY-EQUIVALENT** (the crate's KAT-anchored style).
- **Living-memory (item 15): honestly N/A** — verification is stateless; the only temporal data
  is the attestation epoch budget, which lives in P-D's ledger, cross-ref
  `internal-retrieval-living-memory-arc-2026-07-14` there, not here.
- **Tensor/spectral/eqc (item 16): honestly N/A for the crypto math** (exact integer algebra, not
  closed-form real analysis — `eqc-rs` compiles equations over ℝ, not GF(2^255−19)); the
  *pattern* reuse is the P11 §6 SoA SIMD lane shape (`kernel/src/simd.rs`), cited in §1.

## §9. Regression ledger entries (contract item 17 → `docs/regressions/REGRESSION-LEDGER.md`)

| Name | Pins |
|---|---|
| `REG-P-E-simd-scalar-parity` | §2.6 layers 1–2 (verdict + intermediate parity, both crypto legs) |
| `REG-P-E-ssr2020-lane` | §3.3-T3 — the mixed-order forgery class rejected at its own index |
| `REG-P-E-one-bad-in-batch` | §3.3-T4 — no cross-contamination in either direction |
| `REG-P-E-wasm-empty-import` | §3.2 — lane fully cfg'd out of the no_std wasm build |

## §10. Agent-executable build instructions (contract item 18 — zero session context needed)

Repo: `/root/bebop-repo` (push remote `openbebop`, NOT `origin` — it is archived). Branch off
bebop2's active line. Files (per synthesis W2-L5 targets + this design):

1. **`bebop2/core/src/pq_dsa_avx2.rs` (new)** — 8×i32 AVX2 NTT/invNTT + vectorized
   `montgomery_reduce` + 4-way interleaved Keccak-f[1600] for ExpandA. Declared from
   `core/src/lib.rs` behind `#[cfg(all(feature = "std", target_arch = "x86_64"))]`.
2. **`bebop2/core/src/sign_avx2.rs` (new)** — AVX2 GF(2^255−19) limb arithmetic (vectorize the
   `sign.rs:66-140` limb algebra), same cfg gate.
3. **`bebop2/core/src/sign.rs` / `pq_dsa.rs` (edit)** — add §2.2 types + §2.3 `verify_many` /
   `verify_internal_bytes_many` dispatch shims; expose `pub(crate) verify_scalar` (a rename-free
   wrapper around the existing body); **do not modify** the existing `verify` /
   `verify_internal_bytes` semantics.
4. **Tests** — parity + T2–T6 in the respective `#[cfg(test)]` modules (extend the existing
   suites at `sign.rs:975+` and `pq_dsa/acvp_tests.rs` style); write them FIRST (RED per §3.1).
5. **`bebop2/core/benches/verify_lane.rs` (new)** — the §3.2 Instant-based bench. No new
   dependencies of any kind, dev or otherwise, without a DECART report (standing rule).
6. **Acceptance = §3.2 DoD**, all four §9 regressions green, `cargo test` green in both
   scalar-only and AVX2 configurations, wasm empty-import gate green, measured numbers recorded.
7. Build order: ML-DSA lever first, Ed25519 second (§2.4). Attestation (§4): **types only,
   nothing more, until the operator rules on §4.1.**

## §11. Hermetic principles honored (contract item 20)

- **P1 MENTALISM** — the RFC 8032 / FIPS 204 specs + their KAT/ACVP vectors are the source of
  truth; both implementations are derived artifacts pinned to them; this blueprint's falsifiable
  done-checks precede the code.
- **P2 CORRESPONDENCE** — one concept (verify) keeps one entry point per leg; the forced
  scalar/AVX2 divergence is parity-pinned (§2.6), the exact remedy P2 prescribes for justified
  divergence.
- **P4 POLARITY** — one dispatch mechanism carrying two named poles (simd/scalar) with the
  collapse direction safe (unknown ISA → scalar; any verify failure → `false`, fail-closed);
  attestation's budget polarity collapses toward fewer-mints, never toward bricked-courier (§4.2).

## §12. Docs & memory cross-links (contract item 7)

Depends on / absorbs: `14-BATCH5-network-hardware-findings.md` (v2 top half),
`BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md` §C-A/§C-B + wave table W2-L5,
`P-D-audit-root-delegation-policy.md` (IssuanceBudget seam), `HERMETIC-ARCHITECTURE-PRINCIPLES.md`,
`BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-ADOPTION-2026-07-17.md` (verdict framework),
`BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17.md` (§3.4 hook).
Memory: `crypto-safe-first-pass-2026-07-14.md` (the SSR-2020 forgery + honest walk-back — the
reason §2.1 exists), `sovereign-architecture-19-phase-roadmap-2026-07-17.md` (P06 key_V remains
the cross-cutting blocker for P-D issuance; this lane does not depend on it),
`performance-priority-over-minimal-change-2026-07-17.md` (the scoped perf mandate this phase
executes). Supersedes: nothing — v1 Batch 5 is already superseded by its own v2.

---

## §13. Kalman SoA consumer swarm-dispatch readiness — added 2026-07-18

> Ground truth for this section was re-verified live this pass (2026-07-18), not inherited: the
> TODO wording quoted below is `kernel/src/simd.rs:21-23` as it reads today; the filter/ownership
> claims are `kernel/src/domain.rs:279-357` and `kernel/src/kalman.rs:149-224` as they read today.

### §13.1 Role & responsibility

The Kalman SoA consumer's job is to batch **N couriers' existing per-courier Kalman/EMA filter
update** into one SIMD-lane pass — a Structure-of-Arrays layout directly analogous to how
`simd.rs` already batches AVX2 softmax reduction across independent rows (`softmax_lane4`,
`simd.rs:59-149`; `softmax_batch_lane`, `simd.rs:157-180`). "The existing per-courier filter" is
the scalar 1-D `KalmanFilter` (`kalman.rs:149-224`, `predict`/`update` at `kalman.rs:200,212`)
wrapped by `TrustEstimate` (`domain.rs:279-334`); `geo.rs::ema_next` (`geo.rs:39`) is the
infinite-initial-covariance special case of that same filter, parity-proven bit-identical to it at
`kalman.rs:357-379` (`scalar_kf_equival_ema_next`) — confirming the memory pointer
(`integration-research-tf-attention-circuit-kalman-arc-2026-07-14`, "★KALMAN-FIRST") live. There is
no new filter to invent here: the consumer must execute the SAME `predict`/`update` arithmetic for
many couriers per SIMD step, not a redesigned filter, exactly as §2.1 of this blueprint forbids
combining two *signatures'* algebra — here the equivalent forbidden move is combining two
*couriers'* Kalman algebra into one shared computation instead of N independent ones executed in
parallel lanes.

**What "authority" concretely means here (Step 1 finding — narrower than courier-trust/scoring):**
it is **write-path / cadence authority**, not identity or reputation authority. Per
`domain.rs:336-357`, `apply_event_with_trust` is the sole composition point (the "W19 integration
point") that advances a courier's `TrustEstimate` — it takes `trust: &mut TrustEstimate` from the
caller and steps it (`predict` + `update`-or-hold-prior) exactly once per legal order-fold event,
fail-closed on a missing observation (`domain.rs:327-333`). There is no per-courier lock, actor, or
map in the kernel (`grep -rn "courier_id" kernel/src` and `grep -rn "TrustEstimate" --include=*.rs`
return zero call sites outside `domain.rs` itself) — ownership today is simply "whichever caller
holds the `&mut TrustEstimate` for that courier's fold." The risk a naive SoA consumer introduces
is a **second, out-of-band write path**: e.g. a periodic "batch tick" that steps N couriers'
filters together on a fixed cadence, decoupled from each courier's own event-sourced fold. That
would change *when* and *by what* a courier's trust estimate advances — a cadence/ownership change
— even though the arithmetic itself stayed bit-identical. That is the one real authority risk; it
is not about courier reputation/scoring as a concept (see §13.4 on NO-COURIER-SCORING below).

### §13.2 Definition of DONE (falsifiable, numbered)

a. A `kalman_batch_step` (name illustrative — final name decided at build time) SoA fn exists in
   `kernel/src/simd.rs` (or a new cfg-gated sibling module using the identical AVX2-detect +
   scalar-fallback shape as `softmax_batch_lane`, `simd.rs:157-180`), with a signature that takes N
   independent `(&mut TrustEstimate, Option<f64>)` pairs (or an SoA-native `x`/`P` array pair that
   round-trips losslessly into/out of `TrustEstimate`) and advances each exactly as
   `TrustEstimate::step` (`domain.rs:327-333`) would, one call site's worth of couriers at a time.
b. A bit-identity parity test — same pattern as `simd_softmax_bit_identical_to_scalar`
   (`simd.rs:218-232`) and `ema_next_generated_parity_bit_identical` (`geo.rs:552-558`) — proves,
   over a random battery of N independent couriers each with their own `x0`/`Q`/`R`/observation
   sequence (including `None` observations to hit the fail-closed hold-prior path), that calling
   the batched SoA fn once produces `f64::to_bits()`-exact-identical `x`/`P` state to calling
   `TrustEstimate::step` N times in scalar sequence, for every lane, on every N (including
   non-multiples of the lane width, mirroring `simd_softmax_handles_non_multiple_of_four`,
   `simd.rs:234-247`).
c. A benchmark (`std::time::Instant`, zero new dependencies, matching the P-E §3.2 bench-harness
   pattern already specified earlier in this file) proves a **measured** wall-clock speedup of the
   AVX2 SoA path over the scalar per-courier loop at realistic N (e.g. N ∈ {4, 32, 256}) — a real
   number recorded in this blueprint and the regression ledger, not an estimate.
d. No change to which code owns/writes courier state: `apply_event_with_trust`
   (`domain.rs:347-357`) remains the sole call site composing a courier's Kalman step with that
   courier's own FSM fold event. The batched fn is invoked **from inside** that ownership boundary
   — i.e., by a caller that already legitimately holds N couriers' `&mut TrustEstimate` for their
   respective, already-occurring fold events — never as a replacement ticker that advances a
   courier's trust independent of that courier's own fold event.

### §13.3 Definition of NOT-done / explicit anti-scope

1. Vectorizing the math while ALSO changing update cadence, locking, or which task authors courier
   state is NOT in scope — that is an authority change disguised as a perf change, exactly what
   `simd.rs:21-23`'s TODO comment warns against.
2. Adding a new Kalman filter design (different Q/R tuning, a different state vector, a
   "courier-optimized" variant) is NOT this item — it is a batching/execution change over the
   EXISTING `KalmanFilter`/`TrustEstimate` (`kalman.rs:149-224`, `domain.rs:279-334`); bit-identical
   output is required, not merely "close."
3. Shipping without the parity test because "it obviously matches" is NOT done — every other SIMD
   batching item in this repo (softmax, `simd.rs:218-232`) required a bit-identity test; this item
   is not exempt.
4. Introducing any new courier-reputation/IAM/access-control use of the batched trust estimate is
   NOT in scope. This would be the one path that actually could cross the real NO-COURIER-SCORING
   red line (`docs/design/ARCHITECTURE.md:31`) — see §13.4: that red line is about IAM/reputation
   as an access-control mechanism, not about the existing FSM-fold trust estimate, so simply
   batching the existing arithmetic does NOT cross it, but building a new scoring/ranking/gating
   use on top of the batched output WOULD.
5. Batching couriers whose fold events are NOT already co-occurring into one artificial SIMD tick
   (i.e., forcing couriers whose real events arrive at different times to "wait" so they can be
   stepped together) is NOT in scope — that changes *when* a courier's trust updates relative to
   its own order-fold event, which is precisely the cadence-authority risk named in §13.1. The SoA
   lane batches couriers whose fold events are ALREADY concurrent at the caller; it must not create
   artificial synchronization to fill a SIMD lane.
6. Reworking `TrustEstimate`'s or `KalmanFilter`'s public API (e.g. exposing raw `x`/`P` mutation
   that bypasses `predict`/`update`, `kalman.rs:200,212`) to make SoA layout more convenient is NOT
   in scope — the batching must go through the existing `predict`/`update` semantics, the same way
   `softmax_batch_lane` reuses `softmax_scalar`'s exact op order (`simd.rs:29-42`) rather than
   inventing a new reduction.

### §13.4 Context & docs

- `kernel/src/simd.rs` (whole file) — the existing AVX2 `f64x4` SoA softmax pattern
  (`softmax_lane4` at `simd.rs:59-149`, `softmax_batch_lane` at `simd.rs:157-180`, bit-identity
  tests at `simd.rs:218-274`) to mirror; the TODO itself is at `simd.rs:21-23`: *"The N-courier
  Kalman SoA consumer from §6 is a TODO — the `f64x4` lane primitive here is exactly the substrate
  it needs; integrating `kalman.rs` is deferred to avoid touching the per-courier filter authority
  (noted, not done, per task scope)."*
- `kernel/src/kalman.rs` (full n-D deterministic Kalman filter, `KalmanFilter` struct/`predict`/
  `update` at `kalman.rs:149-224`, `mat::Mat`-only, no external linear-algebra dep, no_std) and
  `kernel/src/domain.rs:279-357` (`TrustEstimate`, `apply_event_with_trust` — the fold-Law
  ownership boundary) — the existing per-courier filter this item batches, not redesigns.
- `kernel/src/geo.rs:39` (`ema_next`) — the scalar EMA precedent, proven the infinite-covariance
  special case of `KalmanFilter` at `kalman.rs:357-379`. Memory pointer (not an openable doc):
  `integration-research-tf-attention-circuit-kalman-arc-2026-07-14` — "★KALMAN-FIRST
  (geo.rs::ema_next IS 1D Kalman)".
- `docs/design/CORE-ROADMAP-INDEX.md` Layer A row (kernel primitives — eqc-rs wiring, `geo.rs`/
  `domain.rs`) and Layer E row (this blueprint) — cross-referenced because P-E's own SIMD-lane TODO
  points at this work, but the batching itself is kernel-primitive/execution-shape work over an
  existing filter, not a network/crypto lever; noted here so a future indexer files it under the
  right Layer rather than assuming it belongs to P-E's crypto-verify scope.
- **NO-COURIER-SCORING — confirmed real and live, checked this pass:** `docs/design/
  ARCHITECTURE.md:31` rejects "IAM/reputation(NO-COURIER-SCORING)" as an access-control/identity
  mechanism (DECART-gated rejection, alongside managed-cloud-default/k8s/GraphQL-mesh). **Not
  implicated by this item as scoped:** the `TrustEstimate` this item batches already exists today,
  is explicitly kept OFF `Order` per a documented "kernel money red line" (`domain.rs:285-288`:
  *"this struct carries courier trust as a separate estimate and is deliberately NOT stored on
  `Order` (which forbids courier-scoring fields)"*), and this item changes only the execution
  parallelism of that existing math — it introduces no new reputation/IAM/access-control
  consumption of the trust estimate (see anti-scope item 4 above, which is the trip-wire if a
  future change tried to add one).

---

## §14. Session fold-in (2026-07-18) — FEC + wire-format land in Layer E

> **Merge note (2026-07-18 reconciliation):** written on
> `research/dowiz-verify-redteam-2026-07-17` as its own "§13", concurrently with §13 above
> (Kalman SoA readiness) landing on `main`; renumbered to §14 at merge. Content unchanged.

Added after the 2026-07-17 writing pass; §1–§12 stand unretracted. Source: the round-2
fail-operational master synthesis
(`docs/design/fail-operational-layout-versioning-2026-07-17/round-2/BLUEPRINT-ROUND-2-MASTER-SYNTHESIS.md`),
whose own §6 mapping table routes two build artifacts to **Layer E** by operator ruling. Both are
carriers/wire-format work — the network half of this layer that the crypto-verify lane (§2) did not
cover. Designs live in the round-2 docs; this section records ownership, the operator rulings behind
them, and how they compose with §2's verify lane. Neither is re-derived here.

### §14.1 Reed-Solomon FEC on the loss-visible lanes (round-2 Fable-A, ADOPT-NOW by operator ruling)

**What flipped and why.** Round 1 deferred FEC; the operator ruled *"reed-solomon will be used, add
FEC too."* The reconciliation that makes this correct rather than cargo-culted (Fable-A §0): *the
decision changed; the physics did not.* Both live carriers (WSS/TCP, iroh/QUIC streams) already do
ARQ, so FEC stays **OUT** of reliable-stream lanes where it is physically inert. Adoption means
building FEC exactly where loss is app-visible:

- **L1 — the new QUIC unreliable-datagram lane** (RFC 9221; `quinn 0.11` is already a direct
  dependency) for latency-critical, supersedable telemetry (courier position/dispatch on the
  cellular profile). Quantified: an 8-datagram-class ML-DSA-65 signed frame at 5% loss goes from
  ~18.5% (k=4) / ~33.7% (k=8) unreconstructable to ~0.22% with the CellularDefault parity rule —
  **~84× at k=4, m=2.**
- **L2 — BPv7 bundle sharding** across couriers/paths (RAID-across-couriers — the one lane where FEC
  buys *delivery probability under partition*).
- **L3 — future non-ARQ carriers**, pre-hardened.

**The doctrine that ties FEC to §2 (load-bearing — this is why FEC is a Layer-E, not a Layer-B,
item):** FEC is a **reliability** control, never an **authenticity** control, and **FEC-decode sits
strictly BELOW crypto-verify** (Fable-A §2.2; pinned by test T3
`fec_valid_forgery_still_rejected_by_gate`). An attacker's tampered bytes are FEC-perfect by
construction — parity says nothing about provenance — so a tampered-then-validly-FEC-encoded frame
must reconstruct fine and then **die at `gate.check` with `CapabilityVerify`, not `Fec*`**. This is
the same authenticity-is-the-only-authority stance §2.1 takes for the verify lane, one layer down
the pipeline: FEC hands whole bytes up, the §2 verifier is still the sole gate. Nothing signed
changes on the FEC lane — signatures commit to the TLV signing domain, untouched.

- **Dependency:** `reed-solomon-simd = "3.1"` (v3.1.0, MIT AND BSD-3-Clause, ADR-020 clean; DECART
  done in Fable-A §1). Note it is *itself* SIMD — complementary to §2's hand-rolled AVX2 crypto
  lane, not a substitute; RaptorQ/fountain codes stay DEFER (licensing DECART precondition).
- **Bonus findings folded in as Wave-2 fixes:** `quinn::Connection` is currently dropped in
  `QuicTransport` (only `_endpoint` + streams retained) — the datagram lane needs it back (Fable-A
  §2.4); and the iroh stream-lane `recv` is missing the `ReplayLedger` + `max_frame_bytes` that the
  wss `recv` has — routed to whichever pass owns MESH-10 carrier parity (Fable-A §0.3). Both belong
  to this layer's transport surface.
- **Adaptive-ratio tension, recorded not resolved (round-2 §5.3 item 2):** a per-peer loss estimator
  that tunes `m` "walks straight at the NO-COURIER-SCORING / no-per-source-weight fence." Held OUT
  until explicitly re-adjudicated; `RecoveryRule::Fixed(m)` from netem measurements is the only
  tuning path today. This is the same NO-SCORING red-line Layer D and F enforce, arriving on the
  network tuning surface — flagged so a future perf pass does not cross it accidentally.

### §14.2 `LaneFrameHeader` — the 32-byte lane-boundary wire format (round-2 Fable-D, ADOPT)

The concrete reconciled wire artifact of the round-2 work, routed here as "Layer E wire format"
(round-2 §6). It is the header an adapter emits at the *lane boundary* (adapter→kernel), decoded
into the Layer-B ingest gate — **explicitly NEVER a network preamble** (the cleartext-first-bytes
carrier stays REJECT-ON-PHYSICS; magic is admissible *only* because of the lane-boundary placement,
Fable-D §5). Shape: `LaneFrameHeader { epoch_id: u64, payload_len: u32, tier: u8, content_address:
u64 }`, `LANE_HEADER_BYTES = 32`, magic `0xBEB0_0BEE`, schema `V1`, **no Confidence field, no CRC
field**, reserved-and-flags MUST be zero, FNV content-address with **recompute-as-sole-authority**
(a wire address value can never override the receiver's recompute — Fable-D T7).

Why it lands in Layer E rather than B: it is the *format on the boundary carrier*, the network/wire
half; its decoded *consumption* (the ingest gate, tier↔grant cross-check) is Layer B/D and cross-cited
there. Two decode laws worth surfacing because they are this layer's "what may exist on the wire"
doctrine (round-2 §6 routes the ConfidenceLevel rejection to "Layer E doctrine"):

- **`ConfidenceLevel` / `SampleQuality` / HDOP-as-R may not exist as a wire field, in any spelling**
  (Fable-C/D, REJECT-AS-CARRIER): a self-reported quality metric is priced at zero for a forger
  (#15 self-assigned-quality threat) — the legitimate idea lives receiver-side as `trace(P)` +
  `last_surprise`, never transmitted. This is the wire-side twin of §2.1's "no self-certification"
  rule. Pinned by `reserved_and_flags_must_be_zero` (Fable-D T4).
- **The self-incrimination rule for any future sender-settable flag bit:** admissible only if
  setting it can *worsen*, never *improve*, the sender's own lane treatment. No flag bits are
  assigned today. This is the general form of "the wire never carries an authority claim."

The header's decode-failure vocabulary is **reused** from the Layer-B `BridgeFault` enum — zero new
result types, per reuse-first. It composes cleanly under §2: `LaneFrameHeader` decode is a
lane-boundary event *after* the network path (§2's `gate.check` is still the sole authenticity
authority on the wire path); the two never overlap. `DeltaPatch` (the adapter's op-list payload
under this header) is Layer B, cross-cited only.

### §14.3 Net effect on this layer

§2 (AVX2 crypto-verify) remains the phase's center of gravity and one real *new-code* build. §14's
two artifacts extend Layer E's **network/carrier** surface — FEC where loss is app-visible, a typed
lane-boundary header where adapters meet the kernel — and both are governed by the same authenticity
bright line §2 draws: reliability and format are below the verifier, never beside it. No §5
rejection is weakened (DPDK/RDMA still optimize the wrong ~10%; FEC is application-layer erasure
coding, an entirely different lever). No operator gate is added beyond §4's attestation gate; the
FEC and header adoptions are operator-ruled ADOPT-NOW already.
