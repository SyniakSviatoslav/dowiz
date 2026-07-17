# BLUEPRINT B4 — Crypto Ground-Truth Bench + Ed25519 Batching + Envelope Budget

> Arc: `agentic-mesh-protocol-2026-07-17` · scoped by SYNTHESIS §5 item 4 · primary source
> `R4-realtime-crypto-verification.md` (§1 P0 flag, §2–3 batching, §5 budgets).
> **Depends on: none. Numbers-provider for B1/B2/B3** — their latency-adjacent acceptance
> criteria cite THIS blueprint's measured ledger rows, not literature. Parallel-safe with
> B1/B2/B3 (touches only a new bench crate + `docs/ledger/`; the one shared integration point —
> rewriting B1–B3's criteria to cite the ledger — happens after the first measured run).
> Anchors: M6 (zero-dep trust boundary), M12, Hermetic P1/P6/RC-1. **Planning artifact only.**
> House precedent: `hermetic-architecture-2026-07-16/HERMETIC-REMEDIATION-PLAN.md` P01 §2.7
> (the claim-latency ledger — a measurement artifact other blueprints depend on).

---

## §0 Problem

The single most load-bearing unverified number in this arc is the ML-DSA-65 verify cost.
R4 §1 budgets **0.2–1 ms/verify [U]** by extrapolating pq-crystals' published Skylake cycle
counts (522,267 cycles reference C; 179,424 AVX2), but this repo's `pq_dsa.rs` is a from-scratch
pure-Rust port that has **no benchmark anywhere in either repo** — it could be
reference-class-or-slower. B1's admission-path latency, B2's settlement-window sizing, and B3's
exposure-check overhead all silently assume real-time hybrid verification is cheap. Until
measured on the actual deployment host, every such criterion is an estimate wearing a
requirement's clothes — the RC-2 "claim replaces check" shape. B4 replaces the estimate with a
recorded measurement, designs the one real batching lever (classical leg only), and pins the
envelope-size decision rule.

## §1 Current-state evidence (live re-read, 2026-07-17)

1. **`pq_dsa::verify` implementation class — reference-C, not schoolbook, not SIMD.**
   `/root/bebop-repo/bebop2/core/src/pq_dsa.rs`: byte-exact pq-crystals port, ACVP-KAT-verified
   (header `:1-15` — its "schoolbook" mention describes the *prior broken* impl, since replaced).
   `verify_internal_bytes` (`:910-973`) is FIPS 204 Alg 8 with a real scalar Cooley–Tukey NTT +
   Montgomery reduction (`ntt` `:198`, `poly_pointwise_montgomery` `:136`); no AVX2, no SIMD.
   **Correct prior: reference-C class or somewhat slower.** Constants confirmed:
   `PUBLICKEYBYTES = 1952` (`:58`), `SIGNATUREBYTES = 3309` (`:64`).
2. **The full path being benched.** `proto-cap/src/hybrid_gate.rs` `HybridGate::check`
   (`:124-209`): freshness → `verify_chain` (each `Delegation` link = one more Ed25519 verify —
   cost scales with chain depth) → optional armed red-line → `RevocationSet` lookups (classical
   key, PQ key id, capability hash) → real Ed25519 (`:171`) → real ML-DSA-65 under `RequireBoth`
   (`:180-186`) → **verify-then-record nonce insert under a `Mutex`** (`:193-206`). Two bench
   traps live here: (a) re-checking the same frame trips `NonceRejected` — a naive `iter()` loop
   would bench the error path; (b) the `seen` mutex is shared state — single-threaded benching
   understates contention, which is acceptable and must be stated.
3. **Ed25519 leg is cofactorless.** `core/src/sign.rs::verify` (`:832-870`) checks
   `S·B == R + k·A` (`:860-869`) with canonical-S rejection (`S < L`, `:842`) and RFC 8032 §7.1
   KATs, including RED KATs. No small-order/torsion rejection on `A` or `R`. This is exactly the
   configuration where naive batch verification historically diverges from single verification
   (§2.3).
4. **criterion status + DECART call.** `criterion = "0.5"` is already a `[dev-dependencies]`
   precedent three times in-family: `dowiz/kernel/Cargo.toml:66` (+ `kernel/benches/criterion.rs`
   and the `BENCH_RESULTS.md` capture convention), `dowiz/llm-adapters/Cargo.toml:17`,
   `bebop-repo/crates/bebop/Cargo.toml:64`. It is **absent from every `bebop2/*` crate** (grep:
   zero hits) — deliberate; `core`/`proto-cap` are the M6 zero-dep trust boundary.
   `criterion-0.5.1` sits in the offline `~/.cargo/registry` cache (no network fetch). Per
   `docs/operating-model/integration-decart-rule.md`, dev-only tooling that never ships is
   explicitly **not covered** by the DECART requirement; a decart note is recorded anyway in
   §2.1 because the placement decision is load-bearing.
5. **Deployment host = this box.** `dowiz-dev` is Hetzner-class AMD EPYC-Milan, 8 cores, AVX2
   present (unused by the scalar code). R4's estimates-don't-transfer caveat is satisfied by
   running here, CPU fingerprint recorded per run.
6. **Ledger precedent live:** `docs/ledger/claim-latency.jsonl` + `README.md` (P01 §2.7 pattern:
   one JSONL row per observation, appender-only, consumers separate). B4 mirrors it.
7. **B1/B2/B3 have not landed** (directory holds R1–R5 + SYNTHESIS only); their batching
   touch-points below are reasoned from SYNTHESIS §5's scoping.

## §2 Target-state design

### §2.1 The benchmark suite

**Placement:** a new bench-only crate (proposal: `bebop-repo/bebop2/bench/`) depending on
`bebop2-core` + `proto-cap`, with `criterion` as its only dev-dependency. The M6 crates'
`Cargo.toml`s stay byte-identical — even their dev-dependency sections.

**Decart note (recorded, though dev-only is exempt):** criterion 0.5 vs a zero-dep
`std::time::Instant` harness. Criterion wins on: three in-family precedents + an existing
capture convention; warm-up/outlier/CI statistics a hand-rolled loop would re-implement badly
(this blueprint's whole point is defensible numbers); offline-available in the registry.
**Probe (strongest case against):** M6 zero-dep purity — answered structurally by the separate
never-shipped bench crate; the `Instant` harness is the documented fallback if bebop2 policy
ever forbids sibling dev-crates. `DECISION: criterion 0.5 in an isolated bench crate —
in-family precedent + statistical rigor, trust boundary untouched.`

**Benches (all with realistic inputs, not empty messages):**

| id | function | input |
|---|---|---|
| `ed25519_verify_single` | `sign::verify` | ~400 B signing domain (frame TLV: capability ~90 B + representative `WorkReceipt`-class payload ~250 B + 32 B binding) |
| `mldsa65_verify_single` | `pq_dsa::verify` | same message; second variant at ~3.4 KB (batched-envelope class — SHAKE `mu` cost scales with message size) |
| `hybrid_gate_check/d1` | full `HybridGate::check` | chain depth 1, empty revocation set, `RequireBoth` |
| `hybrid_gate_check/d3_rev10k` | full `HybridGate::check` | chain depth 3, 10,000-entry `RevocationSet` |
| `sha3_256_1kib` | event-log hash | sanity anchor (expected ~µs, per R4 §4) |

**Statistical validity:** criterion defaults strengthened to warm-up 3 s, measurement 5 s,
sample-size 100 (not the kernel baseline's quick `sample-size 10`). `hybrid_gate_check` MUST use
`iter_batched` over pre-generated frames with **distinct nonces** (evidence item 2a), verified by
a RED companion test that a replayed frame returns `NonceRejected`. Single-threaded; the mutex
caveat stated in the results doc. Run on `dowiz-dev` (or successor deployment host) only —
a laptop run is not a valid ledger entry.

### §2.2 Output → durable ledger → retroactive grounding

New `docs/ledger/crypto-bench.jsonl` (dowiz), mirroring `claim-latency.jsonl` (P01 §2.7). One
row per bench per run:

```
{"ts":…, "host":"dowiz-dev", "cpu":"AMD EPYC-Milan", "commit_bebop":…, "commit_dowiz":…,
 "bench_id":"hybrid_gate_check/d1", "mean_ns":…, "median_ns":…, "ci95_low_ns":…,
 "ci95_high_ns":…, "p99_ns":…, "samples":100, "msg_bytes":400, "chain_depth":1}
```

Human-readable capture goes next to the benches (the `BENCH_RESULTS.md` convention). From the
first recorded run onward, **B1/B2/B3 latency criteria cite the ledger symbolically** — B1:
"manifest admission ≤ 5 × `ledger:hybrid_gate_check/d1.p99`"; B2: "settlement window ≥ 100 ×
measured gate p99" (order-1 s windows per SYNTHESIS §2.1 should pass trivially — but now
*checked*, not assumed); B3: "`try_commit` overhead ≤ 5 % of measured gate p99". The string
"0.2–1 ms" becomes grep-forbidden in acceptance criteria across the arc.

### §2.3 Ed25519 batch verification (classical leg only — no PQ analogue exists, R4 §3)

**Where it actually applies in this arc:** under `RequireBoth` the unbatchable ML-DSA leg
dominates, so batching the classical leg (~273 k → ~134 k cycles/sig, R4 §2) trims a hybrid
burst by roughly 15–20 % — real but modest, stated honestly. The bursts that exist by design:
**(a) B2's settlement/receipt frames landing in the WORM log in bursts + MESH-07 Sync·Pull
log-segment catch-up** (bulk re-verification of a peer's frames — the strongest case, and the
only place the full 2× appears, on `ClassicalUntilPqAudit`-era classical-only frames);
**(b) B1 boot-time re-admission of the stored manifest set.** Entry point: a new
`HybridGate::check_batch(frames, …)` — per-frame cheap checks in the existing order, then
batched classical legs, then per-frame PQ legs (embarrassingly parallel across cores), then
nonce inserts. Per-message admission keeps calling `check`.

**The cofactor pin (the known historical pitfall, named):** "Taming the Many EdDSAs" (Chalkias–
Garillot–Nikolaenko, SSR 2020) showed cofactor**less** batch verification is non-deterministic
on mixed-order edge signatures, and cofactored-batch can accept what cofactorless-single
rejects; ZIP-215 / `ed25519consensus` is the deployed resolution. This repo's single verify is
cofactorless (§1.3). Pins: (i) batch equation **cofactored**; (ii) coefficients `z_i`
**deterministic**, SHAKE256-derived from the batch transcript (Fiat–Shamir style) — preserving
the "verification consumes zero randomness" invariant and making batch verdicts reproducible;
(iii) **single verify remains the sole acceptance authority** — any batch failure,
non-canonical encoding, or small-order `A`/`R` falls back to per-frame single verify, so
batch-accept can never admit a frame the single path wouldn't; (iv) verdict equality
demonstrated on RFC 8032 §7.1 KATs + the SSR-2020 edge-vector suite (§4.3).

### §2.4 Envelope size budget (recomputed, not quoted)

Per-frame hybrid signature tax, from source: `SIGNATUREBYTES = 3309` (pq_dsa.rs:64) + 64 B
Ed25519 = **3,373 B raw**; + TLV field framing (`FID(1) + u32_le len(4)` = 5 B/field × 2) ≈
**3,383 B ≈ 3.3 KiB** — R4's "~3.4 KB" confirmed at decimal rounding. **Honest addition R4 did
not price:** if `subject_key_pq` (1,952 B, `PUBLICKEYBYTES`) ships inside each frame's
capability, the full hybrid wire delta is ≈ **5,330 B**. Pin: after B1 admission the PQ public
key is resolved from the manifest/delegation chain and referenced by `pq_key_id` (32 B) —
**never re-shipped per frame**.

**Decision rule — one message vs batch N events under one hybrid envelope:** bytes saved =
(N−1) × 3,383 B, verifies saved = (N−1) × measured gate cost, so batching pays at **N ≥ 2**;
the binding constraints are latency (events wait for the flush window) and coupling (one
envelope = one all-or-nothing verify). Named constants, checked in the **emitter's** flush path
(B2 settlement sweep; Sync·Pull gossip flush), mirror-pinned per P3-Vibration:
`ENVELOPE_BATCH_MIN_EVENTS = 2`, `ENVELOPE_BATCH_MAX_WAIT_TICKS` (≤ B2's settlement-window tick
budget), `ENVELOPE_BATCH_MAX_EVENTS` (bounds verify blast radius). Scope pins: batch only
same-counterparty, same-capability-scope events; **money-scoped settlements are never batched**
(red-line granularity — each stands alone under the armed gate).

## §3 Migration steps

1. Create the bench crate + the five §2.1 benches (bench code only; zero product-code edits).
2. Run on the deployment host; capture `BENCH_RESULTS.md`-style doc + append the first
   `docs/ledger/crypto-bench.jsonl` rows.
3. Rewrite B1/B2/B3 latency-adjacent acceptance criteria to cite ledger symbols (the one
   post-fan-out integration step; each blueprint gains a "grounded by B4 run <ts>" line).
4. RED-first: implement `sign::verify_batch` (cofactored, deterministic coefficients) against
   RFC 8032 §7.1 + SSR-2020 vectors; then `HybridGate::check_batch` with fallback-to-single.
5. Add the `ENVELOPE_BATCH_*` constants where B2's emitter lands; mirror-pin.
6. Re-run the suite (second ledger rows, incl. `ed25519_verify_batch/{8,64}` vs N singles);
   evaluate the AVX2 trigger (§ out-of-scope) against measured mesh traffic.

## §4 Acceptance criteria (falsifiable)

1. `docs/ledger/crypto-bench.jsonl` contains ≥ 1 row per §2.1 bench id with `host`/`cpu`
   fingerprint matching the deployment host and real measured `p99_ns` — **a recorded number,
   not an estimate**; no B1–B3 acceptance criterion still carries "0.2–1 ms" or any [E]/[U]
   literature figure (grep check).
2. The `hybrid_gate_check` bench provably benches the success path: companion RED test shows a
   replayed frame returns `NonceRejected`, and the bench uses pre-generated distinct-nonce frames.
3. Batch/single consistency demonstrated on known-good vectors: RFC 8032 §7.1 KATs accept under
   both paths; every SSR-2020 edge vector's verdict is documented per path, with any divergence
   resolved by the fallback rule (single verify = sole acceptance authority); two runs of batch
   verify on identical input yield identical verdicts (deterministic coefficients — RNG-free
   verification preserved).
4. The envelope thresholds are **named, documented constants** (`ENVELOPE_BATCH_MIN_EVENTS`,
   `ENVELOPE_BATCH_MAX_WAIT_TICKS`, `ENVELOPE_BATCH_MAX_EVENTS`), mirror-pinned, checked in the
   emitter flush path; a wire-schema test proves frames reference the PQ key by `pq_key_id` and
   never embed the 1,952 B key per message.
5. Money-scoped settlement frames are demonstrably excluded from batching (RED test: a
   money-scope event in a batch flush is emitted as its own envelope).
6. No AVX2/NTT-vectorized code exists in the diff (out-of-scope guard).

## §5 What this unblocks

Retroactively grounds every other blueprint's latency assumption: B1's admission budget, B2's
settlement-window sizing and sweep cost, B3's exposure-check overhead all become citations of a
measured, host-fingerprinted, durably-recorded number — closing R4's P0 and the arc's last
RC-2-shaped estimate. It also produces the trigger data for the one named upgrade path.

**Out of scope (named):** AVX2/NTT-vectorized porting of `pq_dsa` — the known ~3× verify
upgrade (522 k → 179 k cycles, pq-crystals' published numbers). Trigger: only if §4.1's measured
throughput proves insufficient for *measured* mesh traffic. Building it speculatively would
optimize a cost R4 §5 shows sits 1–2 orders of magnitude below network RTT.

---

*Planning artifact only — no code written or edited. All file:line citations re-resolved live
on 2026-07-17 against `/root/bebop-repo/bebop2` and `/root/dowiz-agentic-mesh`.*
