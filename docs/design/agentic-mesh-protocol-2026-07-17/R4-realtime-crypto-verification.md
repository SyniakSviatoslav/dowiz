# R4 — Real-Time Cryptographic Verification for the Agentic Mesh

> Research stream 4 of 6 · 2026-07-17 · planning artifact only (no code changes).
> Question: what does "crypto issued & validated in real time" actually mean for the substrate
> this project has ALREADY built — ML-DSA-65 from scratch (ACVP-KAT-verified), a real
> Ed25519⊕ML-DSA hybrid gate, canonical TLV, and sha3-256 content-addressed event chaining?
>
> **Epistemics legend:** [M] = measured/published number with citation · [E] = honest estimate
> derived from [M] numbers · [U] = unverified/uncertain, flagged as such. Nothing below is a
> fabricated precise number; anything without a citation is labeled.

## 0. The substrate we are budgeting for (verified in-repo)

- **PQ leg**: `bebop2/core/src/pq_dsa.rs` — ML-DSA-65 (FIPS 204, category 3), byte-exact port of
  the pq-crystals reference, verified against NIST ACVP keyGen/sigGen/sigVer vectors. Pure Rust,
  zero external crates, **no SIMD** (no AVX2 NTT, scalar Keccak). Signature = 3309 bytes
  (`proto-cap/src/signed_frame.rs`).
- **Classical leg**: Ed25519 via `bebop2-core::sign`, enforced together with the PQ leg by
  `proto-cap/src/hybrid_gate.rs` (`HybridPolicy::RequireBoth` — a frame is accepted only if BOTH
  legs verify; `ClassicalUntilPqAudit` exists as the ramp mode).
- **Hashing**: `kernel/src/event_log.rs` — pure-Rust FIPS-202 SHA3-256; content-id =
  `SHA3-256(prev ‖ actor_pubkey ‖ actor_seq ‖ payload)`.
- **Randomness**: `bebop2/core/src/rng.rs` — `EntropyRng`, a fail-closed ChaCha20 DRBG seeded from
  `getrandom(2)` (Linux) / `crypto.getRandomValues` (wasm) / `RDRAND` (bare metal), with
  `compile_error!` on unwired targets. Separately, `kernel/src/rng.rs` is a **deterministic
  SplitMix64→PCG64 simulation PRNG — explicitly NOT a CSPRNG** (its own doc says so; it exists for
  reproducible Monte-Carlo).
- **Crucial property already in place**: `pq_dsa.rs` declares "RNG-free on the crypto hot path" —
  and **verification (both legs) consumes zero randomness**. Randomness only matters for keygen,
  ML-DSA hedged signing (`rnd`), and protocol nonces. This decouples §7 (QRNG) from the verify
  hot path entirely, by construction.

## 1. ML-DSA-65 verification latency — what is actually measured

The canonical public numbers are the pq-crystals team's own benchmarks (one core of an Intel
Core i7-6600U, Skylake) [M]:

| Operation (Dilithium3 = ML-DSA-65 class) | Reference C | AVX2 |
|---|---|---|
| KeyGen | 544,232 cycles | 256,403 cycles |
| Sign | 2,348,703 cycles | 529,106 cycles |
| **Verify** | **522,267 cycles** | **179,424 cycles** |

Source: https://pq-crystals.org/dilithium/ (the repo README defers to SUPERCOP for current
KabyLake numbers; the table above is the project's published Skylake table).

At a ~3 GHz clock that is **≈ 60 µs (AVX2)** or **≈ 175 µs (reference C)** per verification [E].
So ML-DSA-65 verification is a **tens-to-low-hundreds-of-microseconds** operation on commodity
x86 — not milliseconds. Numbers do vary meaningfully by implementation (≈3× between reference and
AVX2 above) and platform (embedded ARM Cortex-M is 10–100× slower — see the RP2040 benchmarking
literature, arXiv:2603.19340), so the implementation class matters more than the CPU generation.

**Honest gap [U]:** our own from-scratch pure-Rust `pq_dsa.rs` has **no benchmark in the repo**
(no criterion bench, no timing test found). It is a faithful port of the *reference* (scalar)
implementation, so the right prior is "reference-C class or somewhat slower" — i.e. plan around
**0.2–1 ms per ML-DSA-65 verify** until measured. **Action item for the blueprint: a criterion
bench of `verify_pq` on the actual Hetzner-class host is a P0 prerequisite before any latency
number becomes an acceptance criterion.** Do not copy the AVX2 number into a requirement the
current code cannot meet.

## 2. Ed25519 verification — the classical baseline

From the original "High-speed high-security signatures" paper (Bernstein, Duif, Lange, Schwabe,
Yang, 2011; measured on a 2.4 GHz Westmere Xeon E5620) [M]:

- Single verification: ≈ **273,364 cycles** (≈ 114 µs at 2.4 GHz on 2011 hardware).
- **Batch verification of 64 signatures: < 134,000 cycles/signature**; batch-128 < 125,000;
  large batches < 114,000 — i.e. batching buys roughly **2×**.
- The quad-core machine verified ~71,000 sigs/s (≈ 18k/s/core).

Source: https://ed25519.cr.yp.to/ed25519-20110705.pdf. Modern Skylake+ cores with current
implementations (ed25519-dalek, donna) are faster; **"tens of µs per verify, ~100–130k cycles"**
is the honest modern order of magnitude [E]. Again [U]: the repo's own from-scratch Ed25519 is
unbenchmarked; same P0 bench applies.

**Hybrid gate combined cost:** `RequireBoth` runs two independent verifications, so latency is
essentially **additive** (they are also trivially parallelizable across two cores, but assume
serial for budgeting). The PQ leg dominates:

- Optimized-implementation world [E]: ~179k + ~130k ≈ 310k cycles ≈ **~100 µs/message** at 3 GHz.
- Reference-class world (our current substrate's likely class) [E]: ~522k + ~273k ≈ 800k cycles ≈
  **~270 µs/message**, possibly a few × worse for unvectorized pure Rust [U].

## 3. Batch and aggregate verification — where the honest bad news lives

**BLS aggregation (classical, pairing-based):** n signatures compress to one 96-byte signature.
But two caveats: (a) verification of an aggregate over n *distinct* messages still costs n+1
pairings — production libraries report ≈ 0.36 ms per signer/message pair even parallelized
(Project-Arda BGLS over BLS12-381; blst is the performance reference implementation) — so a
*single* BLS verify (2 pairings, ~1 ms class) is ~10× slower than Ed25519, and aggregation mainly
saves **bandwidth/storage**, not per-message CPU, unless messages are identical; (b) **BLS is
pairing-based and not post-quantum** — adopting it would contradict the hybrid-PQ stance of this
substrate. Mentioned only as the comparison point; not a recommendation.

**Ed25519 batch verification:** mature, deployed (Tendermint/consensus systems), ~2× amortized
speedup [M, §2]. This is real and available to our classical leg with a bounded implementation
effort (random linear combination + multi-scalar multiplication). One subtlety from the
literature: naive batch verify and single verify can disagree on edge-case (mixed-order point)
signatures unless the verification equation is pinned — the blueprint must specify cofactored
verification consistently across batch/single paths.

**PQ-native aggregation — the critical honest answer: NOT mature.** State of the art:

- *Aggregating Falcon Signatures with LaBRADOR* (CRYPTO 2024, ia.cr/2024/311) is the first
  rigorous treatment of aggregating a NIST PQ signature via a lattice proof system — and it is
  for **Falcon, not ML-DSA**, with prover cost that makes it a batching/settlement tool, not a
  per-message primitive.
- *Sequential Half-Aggregation of Lattice-Based Signatures* (ia.cr/2023/159) achieves only ~half
  the size savings, sequentially, for Fiat-Shamir lattice signatures.
- For **ML-DSA specifically there is no standardized, deployed aggregate scheme and no widely
  deployed batch-verification either** — nothing analogous to Ed25519's batch equation exists in
  production. FIPS 204 defines single-signature verify only.

**Design consequence:** "verify N agent messages in one real-time step" is achievable today only
on the **classical leg** (Ed25519 batch) plus embarrassing parallelism on the PQ leg (N
independent ML-DSA verifies across cores). The mesh protocol must therefore budget the PQ leg as
**one full verification per message** — no aggregation shortcut exists to design around. If PQ
aggregation matures, it is a drop-in throughput upgrade, never a correctness dependency.

## 4. Hardware acceleration on commodity CPUs

- **Keccak/SHA-3 (our event-log hash and ML-DSA's internal SHAKE):** x86 has **no SHA-3
  instructions** (SHA-NI covers SHA-1/SHA-256 only). Best x86 is vectorized software: the Keccak
  team reports ≈ 1.4 cycles/byte for KangarooTwelve on AVX2 (long messages); plain SHA3-256 runs
  ~8–12 cpb scalar [E]. ARMv8.2-A **does** have dedicated SHA-3 instructions (EOR3/RAX1/XAR/BCAX)
  — ≈ 0.75 cpb on Apple M1 (keccak.team/kangarootwelve.html). The TCHES literature ("Revisiting
  Keccak and Dilithium Implementations on ARMv7-M"; Kannwischer's AArch64 work) confirms Keccak
  is the **dominant cost inside lattice crypto on platforms without SHA-3 instructions** —
  ExpandA rejection sampling hammers SHAKE128.
- **ML-DSA itself:** no dedicated lattice instructions exist on any commodity CPU. The available
  acceleration is exactly the AVX2/AVX-512-vectorized NTT + vectorized Keccak in the pq-crystals
  code — worth ~3× on verify (522k→179k cycles, §1). AVX2 has been ubiquitous since ~2013,
  including every Hetzner EPYC/Xeon VPS class this repo's docs establish as the deployment target.
- **Event-log hashing cost is noise:** a ~1 KB event at ~10 cpb ≈ 10k cycles ≈ **~3 µs** [E] —
  two orders of magnitude below one signature verification. The sha3-256 chain is not the
  real-time bottleneck and needs no hardware story.

**Conclusion:** "real-time" verification at this mesh's scale needs **no special hardware**. A
single Hetzner-class x86 core covers it (§5); the biggest available speedup is *software* (porting
AVX2 NTT/Keccak paths or feature-gating an optimized backend), worth ~3× when/if throughput ever
demands it.

## 5. What "real-time" must mean operationally

Budget per message, hybrid dual-leg, serial, single core:

| Implementation class | Per-message verify | Throughput/core [E] |
|---|---|---|
| AVX2-optimized both legs | ~0.1 ms | ~8,000–10,000 msg/s |
| Reference-C class | ~0.3 ms | ~3,000–4,000 msg/s |
| Current pure-Rust scratch impl [U] | ~0.3–1 ms (unmeasured) | ~1,000–3,000 msg/s |

Honest headline: **order 10³ hybrid-verified messages/second per commodity core today, order 10⁴
with optimized implementations** — and near-linear scaling across cores since verifications are
independent. Two calibrations: (a) mesh network RTT (10–100 ms) exceeds per-message verify cost
(0.1–1 ms) by 1–2 orders of magnitude — verification latency is *not* the user-perceived latency
driver; (b) the dowiz delivery domain's actual event rates (orders, courier events, agent trades)
are orders of magnitude below 1,000 msg/s. **Verification throughput is not the scaling wall; the
unmeasured implementation is the open risk.** The blueprint's acceptance criterion should be
stated as a measured budget ("hybrid verify p99 ≤ X ms on the deployment host, X pinned by the P0
criterion bench"), not a copied literature number.

## 6. Signatures vs validity proofs (SNARK/STARK) — when each is the right tool

A signature verification proves *"this signer authorized this message"* in ~0.1–1 ms (§5). A
validity proof proves *"this computation ran correctly"* — but **proving** carries overhead
around **10⁶× native execution** in current zkVMs (a16z crypto, "The path to secure and efficient
zkVMs", 2025; a comparative study, arXiv:2512.10020, measured one function at 59 s proving vs
15 µs native ≈ 39,000,000×). Verification of the proof is fast (ms-class for SNARKs; STARKs have
larger proofs, tens–hundreds of KB, still fast to verify) — but somebody must pay the prover.

**Honest conclusion: signature verification is the real-time primitive, full stop.** No
per-message use of a validity proof survives contact with the 10⁶× prover overhead. The only
defensible mesh uses are **periodic/batched, off the hot path**:

1. **Checkpoint/settlement proofs** — prove "events N..M applied correctly against the FSM"
   hourly/daily, so peers audit a checkpoint instead of replaying. Genuinely valuable *later*;
   never latency-coupled.
2. **Light-client join** — a new node verifies one proof instead of replaying the whole sha3
   event chain. Same batched character.
3. If ever adopted: **STARKs (hash-based) are the PQ-consistent choice**; pairing-based SNARKs
   would reintroduce a quantum-vulnerable assumption the hybrid gate exists to avoid.

For the current blueprint: **out of scope for real-time; note as a future settlement-layer
option.**

## 7. ANU QRNG — correct integration per the existing doctrine

**What it is [M]:** ANU's generator measures quadrature fluctuations of the electromagnetic
vacuum via homodyne detection of laser light — true quantum randomness (Symul, Assad & Lam, Appl.
Phys. Lett. 98, 231103, 2011). Two service generations:

- **Legacy API** (`https://qrng.anu.edu.au/API/jsonI.php`): free, uint8/uint16/hex16, **max 1024
  values per request**, being retired in favor of the AWS service (per the official API docs).
  Community reports a ~1 request/minute throttle since ~2021 [U — not confirmed in the official
  docs fetched; treat as plausible but unverified].
- **ANU Quantum Numbers (AQN) on AWS** (`quantumnumbers.anu.edu.au`): API-key based; **free tier
  = 100 requests/month**; paid tier up to **100 requests/second at ~US$0.005/request**
  (ANU/innovationaus announcements).

**Latency reality [E]:** the service origin is in Australia. From Hetzner Falkenstein (this
repo's documented host class), great-circle ~15,000 km puts the physical floor near ~150 ms RTT;
realistic routed RTT is **~250–330 ms**, plus TLS setup on cold connections. And the free tier's
100 requests/*month* is one fetch per ~7 hours. **Both facts independently prove the QRNG can
never sit on a real-time path.** This is not a limitation to engineer around — it is the reason
the existing doctrine is correct.

**The doctrine, made concrete ("QRNG-seeded-never-replace"):**

1. **Hot path**: all runtime randomness comes from the local `EntropyRng` (ChaCha20 DRBG,
   fail-closed, already built in `bebop2/core/src/rng.rs`). Note again: **verification needs no
   randomness at all**; only keygen, ML-DSA hedged signing (`rnd` per FIPS 204), and nonces do.
2. **Periodic reseed (background, async, never blocking)**: fetch ≤1024 bytes from AQN on a slow
   cadence (hourly-to-daily fits even the free tier), then **mix, never replace**:
   `new_state = SHAKE256(old_state ‖ qrng_bytes ‖ getrandom_bytes)`. Hash-combining means a
   spoofed, biased, or observed QRNG response can never *reduce* entropy below what the local
   pool already had (the standard NIST SP 800-90C / Fortuna reseed posture). The QRNG is an
   untrusted *augmenter*, delivered over TLS, treated as adversarial input.
3. **Internal fallback (the "internally" requirement)**: if AQN is unreachable, nothing degrades —
   `getrandom(2)` (which itself folds in RDRAND/RDSEED and interrupt entropy) remains the seed
   source, exactly as `EntropyRng` already implements, fail-closed. The QRNG's absence is the
   default state, not an incident.
4. **Explicit red line**: `kernel/src/rng.rs` (SplitMix64→PCG64) is a deterministic simulation
   PRNG and **must never serve cryptographic entropy** — its own header says so. The internal
   fallback is `EntropyRng`, not the kernel PRNG. The blueprint should state this to prevent a
   future wiring mistake between the two RNGs that both live in this codebase.

## 8. Summary of load-bearing numbers

| Quantity | Value | Status |
|---|---|---|
| ML-DSA-65 verify (AVX2, Skylake) | 179,424 cycles ≈ 60 µs | [M] pq-crystals |
| ML-DSA-65 verify (ref C) | 522,267 cycles ≈ 175 µs | [M] pq-crystals |
| ML-DSA-65 verify (this repo's Rust) | unmeasured; assume 0.2–1 ms | [U] → P0 bench |
| Ed25519 verify | ~273k cycles (2011); tens of µs modern | [M]/[E] |
| Ed25519 batch-64 | <134k cycles/sig (~2× gain) | [M] |
| Hybrid gate per message | ~0.1–1 ms | [E] |
| Hybrid throughput/core | ~10³ now, ~10⁴ optimized | [E] |
| PQ aggregation for ML-DSA | does not exist in deployable form | [M] literature |
| sha3-256 event hash (~1 KB) | ~3 µs | [E] |
| zkVM proving overhead | ~10⁶× native | [M] a16z / arXiv:2512.10020 |
| AQN QRNG free tier | 100 req/month; ~250–330 ms RTT from EU | [M]/[E] |

**Sources:** pq-crystals.org/dilithium · Bernstein et al., *High-speed high-security signatures*
(ed25519.cr.yp.to) · ia.cr/2024/311 (Falcon+LaBRADOR, CRYPTO 2024) · ia.cr/2023/159 ·
supranational/blst + Project-Arda BGLS · keccak.team/kangarootwelve.html · TCHES
*Revisiting Keccak and Dilithium on ARMv7-M* · a16zcrypto.com zkVM-progress post ·
arXiv:2512.10020 · qrng.anu.edu.au/contact/api-documentation · quantumnumbers.anu.edu.au (AQN,
AWS Marketplace listing; ANU/innovationaus coverage) · Symul, Assad & Lam, APL 98, 231103 (2011).
