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

> **Correction (2026-07-17, post-F1 fix — measured; supersedes the 15–20 % claim above):** pin
> (iii), as landed (`bebop2/core/src/sign.rs::verify_batch`, 2026-07-17), is implemented as
> *confirm-every-accept*: a small-order filter alone does NOT close the SSR-2020 mixed-order gap
> (a mixed-order forgery slipped straight through it — see `batch_rejects_ssr2020_mixed_order_forgery`),
> so the cofactored batch equation is an accept-HINT / sound fast-REJECT only, and every
> batch-accept is confirmed by a full per-item cofactorless single verify. The accept path
> therefore costs the batch equation *plus* N singles — ≥ N singles by construction. Measured on
> the fixed code (bebop `docs/ledger/crypto-bench.jsonl`, 2026-07-17): batch/64 = 131.2 ms vs
> 64 × single = 40.3 ms → batch costs **3.26×** the singles with this repo's naive
> (non-Straus/Pippenger) `scalar_mul`. The "trims a hybrid burst by roughly 15–20 %" claim above is
> **structurally unreachable in this implementation** — batching currently has NO throughput
> benefit even on the classical leg; its only value is the sound fast-reject. The original figure
> (from R4 §2's ~273 k → ~134 k cycles/sig) remains true of Ed25519 batch verification *in
> general* — i.e., without this soundness pin, or under a ZIP-215-style cofactored-single
> acceptance authority — but not here. Recovering any real throughput would additionally require a
> Straus/Pippenger multi-scalar mult (out of scope per §5's DECART-gated deferral), and even then
> the accept path stays ≥ N singles while pin (iii) stands. The burst list (a)/(b) survives only
> as *where the fast-reject applies*; B1/B2 must budget no batching speedup (B1 §(a) carries the
> matching correction).

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

## Extended Context

**Why B4 is the numbers-provider the consolidation runs FIRST (Wave 0).** It is the smallest unit
in the arc — a bench-only crate plus `docs/ledger/` rows, **zero product-code edits** (§3 step 1) —
and it has **zero dependencies** (header: "Depends on: none"). Against that near-nothing cost sit
three *named constants in sibling blueprints that are symbolic against B4's output and cannot be
finalized without it*: B1's `FUEL_PER_UNIT` ("pinned after a B4 bench", CONSOLIDATED §4), B2's
settlement-window `Δ` ("value awaits B4's verify bench", CONSOLIDATED §4/§5 Q5), and B3's
`try_commit`-overhead acceptance criterion ("≤ 5 % of measured gate p99", §2.2). CONSOLIDATED §4
makes the ordering an explicit preference, not a description of tension: running B4 first converts
its own migration step 3 — a retroactive editing pass over three *landed* blueprints — into a plain
citation at landing time, "eliminating exactly the multi-document stale-number drift this arc's
discipline exists to prevent." Being parallel-safe with B1 anyway, B4-first costs nothing.

**What happens if B4 is SKIPPED.** Every other blueprint's latency-adjacent acceptance criterion —
B1's admission budget, B2's settlement-window sizing and sweep cost, B3's exposure-check overhead —
stays an *unverified literature estimate* (R4 §1's `0.2–1 ms/verify [U]`, extrapolated from
pq-crystals' Skylake cycle counts) wearing a requirement's clothes, never a measured fact. This is
**exactly the epistemic gap the whole Hermetic-audit arc existed to close.** The direct line:
Mentalism (`PRINCIPLE-1-MENTALISM.md §1`) holds that "an idea asserted as real without code that
manifests it" is a defect — and a latency *budget* asserted as a binding acceptance *requirement*
without a measurement that manifests it is precisely that defect. §0 already names the shape as
"the RC-2 'claim replaces check' shape"; the through-line is RC-1 itself (`HERMETIC-ARCHITECTURE-
PRINCIPLES.md:132` — "self-certification: the claim replaces the check"): skipping B4 leaves the
arc *self-certifying its own performance requirements*, the same class as ADR-020's phantom
authority and the dead cron the Hermetic pass catalogued. B4 is the check that stops the claim from
standing in for it.

**The concrete deliverable.** Not a paragraph of prose but a durably-recorded, host-fingerprinted
`docs/ledger/crypto-bench.jsonl` row that **other blueprints cite by SYMBOL, not by re-quoting a
number** — `ledger:hybrid_gate_check/d1.p99` becomes the single source of truth, and the literal
string "0.2–1 ms" becomes grep-forbidden in acceptance criteria across the arc (§4.1). This mirrors
the `claim-latency.jsonl` precedent (P01 §2.7, `docs/ledger/README.md`): one JSONL row per
observation, appender-only, consumers kept separate. The number lives in exactly one place and is
referenced, never copied — so it can never go stale in four documents at once.

## Definition of Done

Distinct from and additional to §4's acceptance criteria; all four must hold before B4's Wave-0 half
is "done."

1. **The bench ran on the real deployment host, and a future reader can prove which hardware
   produced the number.** The appender MUST populate the ledger row's `host` and `cpu` fields
   (§2.2) by *reading the live machine at run time* (hostname + `/proc/cpuinfo` model string), never
   from a hand-typed literal, and MUST record `commit_bebop`/`commit_dowiz` so the measurement is
   reproducible against exact source. A row whose `host`/`cpu` does not match the deployment host's
   live fingerprint is not a valid ledger entry (§2.1: "a laptop run is not a valid ledger entry") —
   the CI grep gate of §4.1 is extended to reject it. This is the operational proof of §1.5's
   "estimates-don't-transfer caveat is satisfied by running here."
2. **The DECART note for `criterion` is present and complete.** Confirmed: §2.1 carries it in full —
   criterion 0.5 vs a zero-dep `std::time::Instant` harness, DECIDED for criterion in an isolated
   never-shipped bench crate on three in-family precedents + statistical rigor, M6 trust boundary
   untouched, with the `Instant` harness named as the documented fallback if bebop2 policy ever
   forbids sibling dev-crates. It is not thin and needs no sharpening; the DoD requirement is only
   that it **stay inline** (Detailed Planning Protocol step 3) and that the named fallback remain the
   explicit degrade path — both hold as written.
3. **The cofactor pin has a falsifiable NEGATIVE test vector, not only positive ones — and a
   vacuity guard.** The blueprint's positive coverage exists (RFC 8032 §7.1 KATs accept under both
   single and batch paths, §2.3.iv / acceptance 3). The DoD ADDS the missing negative case: a
   known-**bad** "Taming the Many EdDSAs" (SSR-2020 §5) divergence vector — a signature with a
   small-order / mixed-order component in `A` or `R` that the **cofactorless single** verify
   *rejects* (`S·B ≠ R + k·A` after canonical-S, §1.3) but a **naive cofactored batch** equation
   (`8·S·B = 8·R + 8·k·A`) would *accept* (SSR-2020's canonical vectors #4/#5/#6). DoD assertion
   (BLOCKING): placed inside a batch, this frame's overall verdict is **REJECT** — the fallback to
   single verify fires and single, the sole acceptance authority (§2.3.iii), rejects it. Plus a
   **mutation/vacuity guard** in the same discipline as this session's `noether.rs`
   explicit-Euler-gains-energy test (which proves the energy monitor is real by feeding it a
   known-bad integrator that MUST trip it): temporarily stub out the fallback-to-single path and
   assert the batch NOW accepts the bad vector — proving the pin is load-bearing, not decorative. A
   positive-only suite is passed vacuously by a batch equation that accepts everything; the negative
   vector + mutation guard is what makes the pin non-vacuous.
4. **The one-shot-ness of the ledger row is enforced by an explicit uniqueness scheme** (designed in
   the next section): the appender writes a `run_key`, never overwrites an existing row, and the
   symbol-resolution rule is documented so `ledger:<id>.<stat>` deterministically resolves to one row.

## Event-Driven Architecture Treatment

**Honest framing: this blueprint is mostly NOT event-sourced, and that is correct.** A benchmark run
is a *local, one-off measurement on the deployment host* — it never rides `commit_after_decide`,
never enters the WORM `EventStore`, never gossips, and is not a `MeshEvent`. Manufacturing a
"`BenchEvent`" to force the mesh framing would be a fabricated connection: MESH-03's wire vocabulary
is for protocol frames exchanged between peers, not for dev-time host telemetry that no counterparty
verifies or replays. B4 produces *facts about the hardware and code*, not *state of the mesh*. The
blueprint states this plainly rather than dressing a bench loop as an event stream.

**But one real event-discipline question does apply: the `crypto-bench.jsonl` row itself deserves
the same idempotency discipline as `claim-latency.jsonl`** (the P01 §2.7 / Hermetic-remediation
precedent — one appender-only row per observation, consumers separate). §2.2 specifies the row's
*fields* but not its *key or uniqueness scheme*; designed concretely here:

- **Append-only, never overwrite or delete.** Exactly `claim-latency.jsonl`'s contract ("one JSONL
  entry per commit, appended by the appender"). Every bench run appends; no run silently clobbers a
  prior measurement. Old rows are *retained on purpose* as the drift record — the whole point of
  §Long-Term (c) is that today's number will change, and comparison across time requires that the
  old numbers stay readable.
- **Identity = a derived `run_key` field** (added to §2.2's schema): `run_key = sha3_256(bench_id ‖
  commit_bebop ‖ commit_dowiz ‖ host ‖ cpu ‖ msg_bytes ‖ chain_depth ‖ samples ‖ warmup_s ‖
  measure_s)` truncated to 16 hex — the content-id of *what configuration was measured*. Two runs of
  the identical configuration over time share a `run_key` but differ in `ts` (and may differ in
  `p99_ns`); grouping rows by `run_key` is exactly how a consumer sees the same measurement *drift*
  as hardware/implementation change. This is the analogue of claim-latency's `commit_sha` identity,
  generalized to "(config) measured (when)".
- **Symbol-resolution rule (the citation contract for B1/B2/B3).** `ledger:<bench_id>.<stat>`
  resolves to the row with the **newest `ts` whose `run_key` matches the CURRENT-HEAD configuration**
  (current `commit_bebop`/`commit_dowiz` + the live deploy-host fingerprint). Citations therefore
  always resolve to the freshest *valid* measurement; superseded rows remain in the file for
  comparison but are never cited. Idempotency falls out cleanly: identity is `run_key`, recency is
  `ts`, overwriting is forbidden, and "the same thing benched twice" is handled by keeping both rows
  and citing the newer — never by mutating history.

## Long-Term Consequences, Safety, Scalability

**(a) Scalability — the AVX2/NTT trigger, named (sharpening §5's vague "proves insufficient").**
§5 defers the ~3× AVX2/NTT-vectorized `pq_dsa` port (522 k → 179 k cycles) "only if §4.1's measured
throughput proves insufficient for *measured* mesh traffic." Sharpened into a falsifiable threshold
read off this blueprint's own bench, per this session's E53 waiver-form discipline:

> **E53 waiver — AVX2/NTT vectorization of `pq_dsa::verify`.**
> *what:* SIMD port, R4's published ~3× verify upgrade. *why-suspended:* measured verify cost sits
> 1–2 orders of magnitude below network RTT (R4 §5); speculative optimization forbidden.
> *owner:* whoever runs §3 step 6's re-bench. *date:* 2026-07-17.
> *trigger (named number):* start the work when **measured sustained mesh hybrid-verify demand
> crosses 50 % of one core's measured scalar throughput** — i.e. `T_core / 2` verifies/s, where
> `T_core = 1e9 / ledger:hybrid_gate_check/d1.mean_ns` (against R4's ~1,000 verify/s/core estimate
> that is ≈ **500 verifies/s/core sustained**, meaning the mesh would have to dedicate more than one
> of `dowiz-dev`'s 8 cores continuously to verification) — **OR** when a single MESH-07 Sync·Pull
> segment catch-up measures a p99 wall-clock re-verify time exceeding B2's settlement window
> **Δ = 60 ticks (60 s at the 1 tick/s reference profile)**, whichever fires first.

Both limbs are read directly from ledger rows (the crypto-bench `mean_ns`/`p99_ns` plus a companion
sustained-rate stat recorded at the B2 emitter), so the trigger is a checkable number, not "if
needed." Below it, per R4 §5, AVX2 optimizes a cost far under the latency floor and is out of scope.

**(b) Safety — the cofactor pin is a non-negotiable, blocking gate.** The batch-verification cofactor
pitfall is a named historical vulnerability class (SSR-2020 / ZIP-215, §2.3). State the consequence
of the pin being wrong or later silently regressed plainly: **a forged batch-verification result
accepts an invalid signature as valid** — an unauthorized or forged frame admitted into the mesh as
*authentic*. In an architecture whose entire premise (R3 §8) is that the cryptographic agent trust
plane is the one part that *must* be built and cannot be delegated to any external authority,
admitting a forgery is a **total soundness break of the trust plane — about as severe as a safety
bug gets in a crypto-verified mesh.** Therefore the negative-vector test (DoD item 3) and acceptance
criterion 3 are treated as **BLOCKING / non-negotiable, not one test among many**: a red there blocks
the *entire batch feature* from landing, and the system stays in its current safe state
(single-verify-only) until green. Batch verification is a pure optional throughput optimization and
must never sit on the critical acceptance path; "single verify remains the sole acceptance authority"
(§2.3.iii) is the structural backstop, but the fallback logic *is code that can regress*, which is
exactly why the mutation/vacuity guard (DoD item 3) is mandatory rather than nice-to-have.

**(c) Ethics / long-term — measurement staleness and the re-bench trigger, honestly.** The numbers
B4 records are true only of *today's* hardware and *today's* `pq_dsa.rs`; both will change, and a
citation `ledger:hybrid_gate_check/d1.p99` silently becomes a lie the moment either does — the row
now describes a machine or a code path that no longer runs. Three re-bench triggers, in decreasing
reliability:

1. **New hardware generation deployed** → the row's `host`/`cpu` fingerprint ≠ the live deploy-host
   fingerprint → the symbol resolves to no HEAD-matching row → stale, re-bench required.
2. **Implementation change on the verify path** (`pq_dsa.rs`, `sign.rs`, `hybrid_gate.rs`) → the
   row's `commit_bebop` ≠ HEAD → `run_key` mismatch → stale, re-bench required.
3. **A time-based cadence** (e.g. quarterly) — advisory only, see below.

The honest mitigation, and its limit: a **periodic re-bench cron would be a dead pendulum.** The "no
cron reliably fires" problem this session found elsewhere applies here directly and is *live right
now* — `PRINCIPLE-5-RHYTHM.md §2` re-verified the Hermes gateway DOWN with all four registered jobs
at `last_run=None`, so a "quarterly re-bench job" is structurally guaranteed **not** to fire while
MEMORY would record it as running. So trigger 3 is recorded as **secondary advisory only, explicitly
flagged unreliable.** The primary guarantee uses the pattern B2 chose over a timer (sweep-on-commit,
not a clock): make staleness detection **ride a path that always runs.** Extend acceptance-criterion
§4.1's CI grep gate so it ALSO fails when a *cited* ledger symbol's `run_key` does not match HEAD's
configuration, or when the resolved row's `host`/`cpu` ≠ the deploy-host fingerprint. Because every
change already runs CI, this is "structurally guaranteed to fire" in the `PRINCIPLE-5-RHYTHM.md` /
Ananke "structurally inevitable, not remembered" sense — triggers 1 and 2 become a failing build the
first time the hardware or the verify code moves, never a checklist item anyone has to recall.

---

*Planning artifact only — no code written or edited. All file:line citations re-resolved live
on 2026-07-17 against `/root/bebop-repo/bebop2` and `/root/dowiz-agentic-mesh`.*

---

## 2-Question Doubt Audit (blueprint-organization stage, 2026-07-17 decorrelated pass)

> Added by a decorrelated audit pass per `AGENTS.md`'s three-point doubt ritual. This blueprint
> already carries an inline DECART (§2.1, for `criterion`) and a working Anu/Ananke-shaped argument
> (Long-Term (c) invokes Ananke by name for the CI-grep staleness gate); the one part missing was the
> explicit per-blueprint 2-question audit, appended here. Nothing above this point is modified.
> Independently re-verified against `/root/bebop-repo` (read-only) and the live
> `docs/ledger/crypto-bench.jsonl` row set, 2026-07-17.

**Post-landing state re-checked first, since it gates everything else: honestly recorded, confirmed.**
The §2.3 "Correction (2026-07-17, post-F1 fix — measured)" block's every load-bearing number was
independently recomputed from the raw ledger row at commit `6541ae8` rather than trusted: `batch/64`
`mean_ns = 131,167,487.5` (≈131.2 ms, matches); `ed25519_verify_single` `mean_ns = 629,556.5`, so
`64 × single = 40,291,616 ns` ≈ 40.3 ms (matches); ratio `131,167,487.5 / 40,291,616 = 3.256` ≈ 3.26×
(matches exactly). The named regression test `batch_rejects_ssr2020_mixed_order_forgery` exists at
`bebop2/core/src/sign.rs:1382` in that commit, and `verify_batch`/`verify_batch_no_fallback`/
`verify_batch_inner` (`:971-984` onward) implement exactly the "confirm every accept via full single
verify" fix the correction note describes. This is a rare case of a post-landing claim standing up to
independent re-derivation from raw data, not just re-reading prose — recorded as a positive finding, not
merely "checked."

**Q1 — least confident about, each actually checked:**

1. **`ed25519_verify_single`'s recorded cost (≈630 µs) is high for Ed25519 verification and neither the
   blueprint nor this pass explains why.** Optimized implementations (e.g. `ed25519-dalek`) verify in
   tens of microseconds; this repo's is a "naive non-Straus/Pippenger `scalar_mul`" per the consolidated
   doc's own correction, which plausibly explains a 5-10× slowdown but not obviously a ~10× *from that*
   baseline too. Taken from the ledger at face value; not independently re-derived from a cycle count.
2. **`mldsa65_verify_single` (≈792 µs) is only ~26% above `ed25519_verify_single` (≈630 µs), which is
   surprisingly close for two structurally different schemes** (NTT-based lattice vs. elliptic-curve
   scalar mult) — this could mean ML-DSA-65 really is that cheap relative to this repo's slow Ed25519, or
   it could mean a shared fixed cost (fixture setup, `black_box` overhead, hashing) dominates both
   numbers. Neither this blueprint nor this pass isolates the two possibilities; flagged rather than
   resolved.
3. **The commit message's phrase "Instant-based, zero-dep" could misread as contradicting the §2.1
   DECART decision for `criterion` — checked directly, it does not.** `bebop2/bench/Cargo.toml` (at
   `6541ae8`) keeps `criterion = "0.5"` as the sole dev-dependency exactly as decided, used in
   `benches/crypto.rs`. The "Instant-based, zero-dep" description refers to a *second*, separate tool
   (`src/bin/record-ledger.rs`) that produces the durable `p99`-bearing ledger row specifically because
   "criterion does not expose p99" (the Cargo.toml's own comment) — a real gap in §2.2's original
   design (which specifies the ledger row's *fields* but not which tool computes them) that the
   implementation closed consistently with the DECART, not around it. Worth noting as an
   implementation-time elaboration the blueprint text doesn't anticipate, not a deviation.
4. **The landed code and ledger rows live only on `feat/b4-crypto-groundtruth-bench`** (local and
   `openbebop` remote in `/root/bebop-repo`) **— not on the branch actually checked out in this
   read-only clone right now** (`feat/verification-harness`, confirmed dirty with unrelated staged
   `delivery-domain` changes), and `git merge-base --is-ancestor 6541ae8 HEAD` returns **no** on that
   checkout. A re-verification pass that greps the currently-checked-out working tree instead of
   fetching/reading the specific commit would wrongly conclude neither `verify_batch` nor the ledger
   rows exist. (This is the same branch-vs-worktree trap this session's B3 pass hit independently for
   `TokenBucket::release` in the dowiz repo — see that appendix — now confirmed a second time on the
   bebop-repo side.)
5. **The 3.26× figure was recomputed from the recorded numbers, not from re-running the benchmark.**
   This pass confirms the *arithmetic* is honest (item above) but did not re-execute
   `cargo bench -p bebop2-bench` to confirm the measurement reproduces within noise on a second run —
   routine risk for a perf number, stated rather than silently assumed.
6. **DoD item 4's "never-overwrite" `run_key` uniqueness scheme was checked against the schema
   description, not against `record-ledger`'s actual insert logic.** The recorded rows' `run_key` fields
   are present and are 16 hex characters (matching "truncated to 16 hex"), and no duplicate `run_key`
   appears in the observed rows — consistent with, but not a substitute for, reading the binary's insert
   code to confirm it structurally refuses to overwrite rather than merely happening not to have been
   run twice yet.

**Q2 — the biggest thing this pass might be missing:** **this is the third time in one session that a
"landed" claim in this arc turned out to be real but living on a branch other than the one obviously
checked out** (dowiz's `TokenBucket::release` off `feat/harness-llm-backend`'s default checkout in the B3
pass; the P07 dedup fix similarly off-branch per that same pass; now B4's bench + ledger off
`feat/verification-harness`). Individually each is a non-issue once traced (the commit is real, the code
is real, the numbers check out) — but the *pattern* suggests this arc's "landed" bookkeeping is
branch-implicit rather than branch-explicit: nothing in any of these three documents states *which
branch* is authoritative for a "landed" claim, so each verification pass has had to independently
rediscover the right branch by trial. The honest blind spot: this pass has no way to confirm whether
`feat/b4-crypto-groundtruth-bench` is intended to merge into `feat/verification-harness` (or main) before
this arc is called done, or whether it is expected to stay a permanent side-branch that downstream
blueprints must remember to fetch — that decision sits with the operator/lead agent, not with a
read-only audit pass over `/root/bebop-repo`.

*2-Question audit pass, 2026-07-17. No code read in this pass was edited; nothing above this section was
modified. Grounded in live reads of `/root/bebop-repo` commit `6541ae8` (`sign.rs`, `bebop2/bench/
Cargo.toml`, `benches/crypto.rs`) and the `docs/ledger/crypto-bench.jsonl` rows at that commit.*
