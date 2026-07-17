# BLUEPRINT v2 — Bebop2 Mesh Masterwork: One Architecture, the 185-Item Ledger, the Three New Findings, Corrected Waves (2026-07-17)

> **Supersedes** `BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS.md` (v1). v1's per-concept verdicts
> mostly stand; this v2 (a) presents the system as ONE dependency graph *before* the flat ledger
> (per doc 19's complaint that five ideas that are one mechanism were shown as five rows), (b)
> cross-references the operator's authoritative **185-item list** (every item gets a verdict or an
> explicit merge-note — zero silent drops), (c) integrates the six re-examined flips (doc 20), the
> corrected network/crypto target (doc 14 v2), the local-LLM measurement (doc 21), and **three new
> Step-2 findings measured/researched this session** (SIMD crypto-in-core, cache-domain pseudo-NUMA,
> MoE-mesh-mirror + core-resident STARK), (d) states the authority/watchdog boundary honestly (doc 19
> Part 2 — arithmetic invariants are genuine zero-authority; the tamper leg is a *finite anchored*
> authority, not zero — v1 slightly over-claimed), and (e) promotes doc 19's two real bugs
> (normalize-before-hash; ungated arena snapshot) to Wave 1.
>
> **Binding constitution** (memory `bebop2-mesh-masterwork-2026-07-17.md`), applied literally:
> framing rule (REJECT only on physics/correctness/absent-hardware/CI-red-line; "unconventional" is
> never a reason; omission = sabotage); priority-ordering rule (LATE dialogue material —
> Monocoque/equilibrium/equations-not-primitives — outranks EARLY material on conflict); execution-
> model rule (core logic as EQUATIONS via `tools/eqc-rs`, landed `7c7763af7`; Rust/every language is
> an adapter; scripts→0; invasive refactor licensed); scope escalation (ENTIRE codebase, all repos;
> UI = real-time render of kernel state); operator verdicts (Sybil-proof via signed-capability
> issuance, never reputation/courier-scoring; zero watchdog processes, zero proxies — self-heal/self-
> terminate are structural properties of the normal path).
>
> **Provenance note (honesty, load-bearing):** the delegated task shipped the 185-item list as a
> category-placeholder; the coordinator then supplied the verbatim list mid-task. §2 below cross-
> references the **verbatim** list. Every `file:line` is inherited from the batch that read it live
> (10–18) or is a **[MEASURED this session]** probe I ran (cache topology, crypto floor, crypto
> backend, openssl). Nothing invented.

---

## §A. THE SYSTEM AS ONE DEPENDENCY GRAPH (read this before the ledger)

The batches issued correct per-item verdicts but, exactly as the operator said, presented **one
mechanism as many rows.** The real picture (doc 19 Part 1) is a single pipeline, and every heavy
concept in the 185-item list attaches to one of its five nodes or to the propagation layer over it.

```
   (e) TILE / STASH  ──────────────────────────────────────────────┐  substrate everything acts on
   a sub-graph / sub-tensor in an arena region                      │  kernel/src/arena.rs (BumpArena, W2-L1)
        │                                                           │
        │ must be canonicalised BEFORE it can be compared/hashed    │
        ▼                                                           │
   (c) NORMALIZATION  ── THE ROOT DEPENDENCY                        │  csr.rs:125-152 row_normalize (fixed-order)
   row-stochastic Â; LaplacianKind{Unnorm|Sym|RW}                   │  csr.rs:282-316
        │                                                           │
        │ only a canonical tile has a cross-node-meaningful hash    │  ⚠ BRIDGE-GAP #1 (doc19): slem_cached
        ▼                                                           │     content-addresses the RAW adjacency
   (a) CONTENT-ADDRESS / HASH  ── depends on (c)                    │     spectral_cache.rs:117-118 → Wave 1
   matrix_content_address (FNV-1a, to_bits) spectral_cache.rs:98    │  event_log content-id + prev→tip chain
   MerkleLog root = pair-hash of sorted ids  sync_pull.rs:412-481   │  DecompCache keyed by address
        │                                                           │
        │ the hash is BOTH the snapshot key AND the convergence proof
        ▼                                                           │
   (b) SNAPSHOT / CHECKPOINT-RESTORE (+epoch) ── depends on (a); GATED by (d)
   boot_verify replays WORM log (in-memory: BUILT) hydra.rs:253-265 │  ⚠ BRIDGE-GAP #2 (doc19): arena
   arena snapshot-then-drop → retained base (DESIGNED)              │     snapshot-then-drop is NOT drift-
   durable snapshot + restore-drill (GAP: P12/Hermetic #4)         │     gated before retention → Wave 1
        ▲                                                           │
        │ a snapshot is admissible ONLY if the rebuilt spectrum is Damped/Resonant
        │                                                           │
   (d) ZERO-POINT / EQUILIBRIUM (discrete Lyapunov) ── gates (b); measured on (e)
   DriftClass{Damped ρ<1 | Resonant ρ≈1 | Unstable ρ>1} spectral.rs:313-352 (BAND 1e-6)
   drift-gate rejects Unstable pre-persist  event_log.rs:389-419
   noether::step_preserves |I(f x)−I(x)|≤tol  noether.rs:19-39
        └───────────────────────────── PROPAGATION LAYER ─────────────────────────────┐
   DecisionUnit gossip = the Decision Compiler (latency §2 authority): a canonical, normalized,
   content-addressed, epoch-stamped tile is gossiped over the EXISTING SyncFrame transport
   (discovery.rs / sync_pull.rs, QUIC, ≤1 MiB opaque payload, Merkle-convergent), imported ONLY
   through once-per-artifact independent replay (verify-before-persist / key_V shape). No new carrier.
```

**The single sentence the batches lost:** a **tile** is **normalized** to a canonical form so its
**content-hash** is identical bytes on every node, so it can be **snapshotted** into a retained base
keyed by that hash at an epoch boundary, admitted **only if** its spectrum stays in the
**equilibrium (Lyapunov ρ≤1) band** — `(e)→(c)→(a)→(b), gated throughout by (d)`, and **propagated**
by DecisionUnit gossip. Remove any one step and the next is ill-defined. Everything heavy in the
185-item list — tensor graph state, epochs, snapshots, the zero-point, predictive handoff, PoQ,
the mesh header fields — is a role in THIS graph, not an independent feature.

**Two real, small, buildable bridges are missing** (doc 19, promoted to Wave 1 here):
- **Bridge-gap #1 — normalize BEFORE hash.** `slem_cached` content-addresses the *raw* adjacency
  (`spectral_cache.rs:117-118`); two nodes that build the same logical tile at different scale hash
  differently ⇒ Merkle roots never converge, `DecompCache` never hits cross-node — silently. Fix:
  content-address the `row_normalize`d (or integer-scaled canonical) form. Sound *only* because
  IEEE-754 mandates correctly-rounded `/` (cross-target bit-identical) but **not** transcendentals
  (`rng.rs:22-28`) — so the hash-path normalizer must be integer/rational + fixed-order (the same
  physics that killed the harmonic RGB codec, item 130).
- **Bridge-gap #2 — drift-gate the snapshot.** The arena maintenance pass runs `row_normalize`+PPR
  but never `classify_drift` before a rebuild becomes the retained base — so an `Unstable` rebuild
  could be snapshotted. Fix: route the snapshot-retain step through the same
  `commit_after_decide_drift_gate` (or `noether::step_preserves`) admission the event log already
  uses, so only a Damped/Resonant rebuild is ever retained. This is the concrete meaning of item 181
  ("epoch-clock as the correction rhythm") and item 185's Snapshot-Re-entry leg.

---

## §D. AUTHORITY / WATCHDOG ACCOUNTING — the honest boundary (corrects v1's over-claim)

Tested against a compiled, executed toy this session (doc 19 Part 2: `watchdog_vs_verify.rs`, `rustc
-O`, run 4×, output stable). Three axes the v1 synthesis blurred into one:

1. **Liveness/supervision axis — the real categorical difference.** Deleting an inline verify-before-
   persist is a **compile-time hole** (the `Ok(value)` the caller wants cannot be produced without the
   check, because the check sits on the causal path of the result). Deleting a watchdog is a **silent
   runtime gap** (`balance` sits at `−20` forever, no signal). Absence-is-visible vs absence-is-
   invisible is categorical, and it is *why* a watchdog has a liveness regress ("who watches the
   watcher") while an inline check has none. **On the operator's actual target — "remove the
   proxy/watchdog" — the removal is genuine and decisive.**
2. **Representability axis.** Even *with* the watchdog, the bad state was observed negative 3× and the
   reactive clamp destroyed information (final 80, not 100). Inline verify: most-negative-ever = 0, bad
   state never produced. This is `commit_after_decide_drift_gate` — "endures by NOT persisting."
3. **Authority/trust axis — where the honest limit is.** For **internal-arithmetic invariants**
   (budget/money/drift ρ/fuel) authority **genuinely dissolves**: the check *is* the computation, it
   bottoms out at the type system / IEEE-754 / the compiler — the operator's "remove authority
   entirely" is **literally achieved** for this class. For the **tamper / cross-party leg**
   (`integrity_check`, `boot_verify`, `key_V`, `WorkReceipt`) authority is **NOT dissolved — it is
   replaced by a *finite anchored* authority**: an independent second party whose verdict you trust
   because a genesis anchor certified its identity once (`commit → key_V verdict → anchor`, a finite
   bottom, not an infinite tower). You cannot type external tamper out of existence.

**Net (the phrase to use going forward):** the target design (`key_V`) is **inline + independent** —
neither a watchdog (axis 1) nor self-certification (axis 3). The zero-supervisor ideal is
**earned exactly on the arithmetic invariants and is a finite anchored authority exactly on the
externally-caused ones (tamper, crash-loop).** Any doc that says "no authority at all" for the tamper
leg is over-claiming; the honest word is "a finite structural authority." This is why **P06 key_V** is
the load-bearing fix for the one leg that still needs a check, and why the restart-intensity bound
(item 139) lands as a *launch-path predicate* (T-6), never a standing monitor.

---

## §E. THE TWO REAL BUGS → WAVE 1

Both are in §A above and are **promoted to Wave 1** (they were latent in doc 19, not in the v1 wave
plan). They are small, buildable today, zero operator gates, and they are the *composition* the
185-item "tensor graph state / epochs / snapshots" clusters silently assume:

- **W1-L10 (NEW) — normalize-before-hash.** Content-address the canonical (`row_normalize`d /
  integer-scaled) tile in `slem_cached`; RED test: two nodes building the same logical tile at
  different scale converge to the same content-id (today they do not). Constraint: integer/rational,
  fixed-order (never a transcendental on the hash path).
- **W1-L11 (NEW) — drift-gate the arena snapshot.** Route the arena snapshot-then-drop retain step
  through `classify_drift`/`step_preserves`; RED test: an `Unstable` rebuild is refused retention.

These join the exactly-once bug (W1-L2, `append_raw`) as the three correctness-closure items Wave 1
must land before any heavier mesh composition, because §A's whole pipeline is undefined without them.

---

## §C. THREE NEW FINDINGS (Step 2 — measured/researched this session)

### C-A. Crypto verification IN-core (attack the real bottleneck, don't re-reject kernel-bypass)

**What I inspected/measured this session:**
- **The actual backend is scalar, from-scratch, no SIMD.** bebop2's Ed25519 is
  `bebop2/core/src/sign.rs` — a hand-rolled `no_std` (`extern crate alloc`) RFC-8032 implementation
  (`verify` at `:929`, pure-integer field math). It is **not** ed25519-dalek. The `ed25519-dalek` /
  `curve25519-dalek` in `Cargo.lock` use the **`fiat-crypto` scalar backend** (not the SIMD backend)
  and live only in the **legacy** `crates/bebop` (the archived repo, out of scope). ML-DSA-65 is
  RustCrypto `ml-dsa 0.1` (portable scalar) in legacy, and bebop2's own from-scratch `core/src/pq_dsa.rs`
  — **also scalar**. So **neither leg uses SIMD today.** [MEASURED — grep/read this session]
- **CPU crypto ISA on this host:** `aes avx2 bmi2 pclmulqdq sha_ni` — **no AVX-512, no VAES, no
  AVX-512-IFMA (VPMADD52).** [MEASURED — `/proc/cpuinfo`] There is **no** Ed25519-specific or lattice-
  specific hardware instruction on x86 (AES-NI/SHA-NI accelerate AES/SHA-2 only; SHAKE/Keccak used by
  ML-DSA cannot use SHA-NI). The only lever is SIMD on the arithmetic.
- **Crypto floor re-measured:** `openssl speed ed25519` → **verify 13,997/s ≈ 71 µs**, sign 38,039/s
  ≈ 26 µs. SHA-256 (SHA-NI) 1.7 GB/s. [MEASURED] Confirms doc 14 v2 §0.5's ~69 µs. bebop2's *scalar*
  `sign.rs` verify is almost certainly **slower** than openssl's optimized ~71 µs → headroom exists.

**Is SIMD lane-parallel verification a real, safe technique — distinct from the rejected batch-accept?**
**Yes, two distinct techniques, both SAFE (each signature fully & independently verified — categorically
different from the SSR-2020-unsafe batch-equation, item C9):**
1. **Intra-op SIMD (curve25519-dalek `avx2` backend).** Vectorizes the field arithmetic *within one*
   scalar-mult (parallel Edwards formulas across lanes). ~**1.5–2×** on a single Ed25519 verify;
   AVX-512-IFMA would add ~1.5× more but is **absent here** (and on realistic consumer owner-hubs).
   Not "batch N sigs in one equation" — it is "make one full verify faster." Safe.
   [RESEARCHED — dalek docs/Medium (de Valence, IFMA)]
2. **ML-DSA-65 AVX2 NTT + Keccak×4 (the operator is right — lattice SIMD-parallelizes far better).**
   Dilithium's bottleneck is the NTT (256-coeff poly-mul over a 23-bit prime) + SHAKE sampling; both
   are embarrassingly SIMD-friendly. The pq-crystals AVX2 backend reaches ML-DSA-65 verify **~70 µs**
   where scalar is meaningfully slower; the NTT itself gets **2.4–2.5×** from instruction-level tuning
   over the official AVX2 baseline. Each signature still fully & independently verified. Safe.
   [RESEARCHED — eprint 2026/1272; quanchain/quarkslab ML-DSA notes]

**Does this flip the DPDK/kernel-bypass verdict back toward relevance? NO — and this is the honest
arithmetic.** Even with maximal realistic SIMD (Ed25519 ~2× → ~35 µs; ML-DSA AVX2 ~2× → ~35 µs;
per-recv RequireBoth ~70 µs), crypto is **still ~10× the whole packet-stack traversal (~7 µs)** and
**~400× the syscall tax (~0.18 µs)** that io_uring/AF_XDP eliminate. Doc 14 v2 §0.5's governing ratio
holds across the entire SIMD range. **Kernel-bypass stays REJECT/DEFER — it optimizes the wrong 10%.**

**But the operator's redirect is satisfied the *right* way:** the actual bottleneck (crypto) has a
**real, safe, buildable core-level-parallelism win**, and *that* is what to build. **Concrete
recommendation:**
- Wire an **AVX2 SIMD verify path** behind `is_x86_feature_detected!("avx2")` with the existing scalar
  path as the cross-target-deterministic fallback (same pattern as `simd.rs`/`householder.rs`). Biggest
  lever is **ML-DSA-65 (NTT + Keccak×4)**; secondary is **curve25519 avx2 for Ed25519**.
- **Determinism is clean here** (unlike a float reduction): a verify verdict is a boolean accept/reject
  — the SIMD path must produce **bit-identical verdicts** to scalar, trivially assertable.
- **Verdict: ADOPT-WITH-TRIGGER.** Trigger = per-recv verify measured as the dominant mesh cost under
  real load on the owner hub (§0.5 already predicts it is). This is the ONE crypto/network item that
  moves toward ADOPT once a second live node makes per-recv volume real. **It does NOT un-reject
  kernel-bypass.** (Feeds ledger items 5, 7, 25, 103, 119.)

### C-B. Cache-domain pseudo-NUMA on a single socket — resolved with a real recommendation

**Measured this host** [MEASURED — `lscpu -e`, `numactl --hardware`, `/sys/.../cache`]: AMD EPYC-Milan
KVM guest, 8 vCPU = 4 physical cores × 2 SMT, **1 socket, 1 NUMA node** (`node distances: 0→0 = 10`).
Cache topology:
- **L1d/L1i 32 KiB and L2 512 KiB are PER PHYSICAL CORE** (`shared_cpu_list=0-1`, i.e. one L2 per
  SMT-pair). This is a **real** cache-domain boundary on a single socket.
- **L3 is a SINGLE unified 32 MiB instance, `id=0`, `shared_cpu_list=0-7` (all cores).** There is
  **no CCX / L3-slice sub-domain** exposed — the guest sees one flat L3. No sub-L3 pseudo-NUMA zone.

**Resolution (not kicked back — the operator asked me to research it, so here is the ruling):**
- The **buildable-now cache-domain technique on a single socket is L2-per-core affinity**, and it is
  **already decided**: P25 CORE-BOUND pins CPU-bound work `taskset -c 0,2,4,6` — one work-stream per
  *physical* core, which keeps each core's 512 KiB L2 private to one stream (no L2 contention from the
  SMT sibling). That is the genuine single-socket "treat a cache domain as a placement zone," and it is
  the *only* one this host (and the realistic owner-hub: Intel N100/i5 with a monolithic ring-bus L3,
  or a single-CCD Ryzen) actually exposes. **ADOPT = already-adopted (P25).**
- An **L3-slice / CCX / CCD pseudo-NUMA zone does NOT exist on this host or the realistic hub.** It is
  real only on **multi-CCD/multi-die parts** (Ryzen 9 dual-CCD, Threadripper, multi-die EPYC), where
  cross-CCD L3 access is a genuine latency step — the same hardware class as the dual-socket NUMA
  question (doc 14 v2 item 6). So: **DEFER-WITH-TRIGGER, gated on multi-CCD/multi-socket hardware
  procurement**, identical in shape to the NUMA decision. The LLC-aware allocation code is harmless to
  write behind a NoOp port (`core_pinning.rs:41-64` template) and degrades to a no-op on a single L3 —
  the safe hedge, no earlier.
- **NUMA pinning proper: REJECT-as-requirement / no-op** on the realistic single-socket hub (measured
  1 node), exactly doc 14 v2 item 6 — with the operator-facing question preserved: single-socket (NUMA
  = no-op, adopt the rejection) vs dual-socket (real, but a power/noise/cost procurement decision).

**Net for §G:** the NUMA/cache-domain question is **RESOLVED, not open**: single-socket cache-domain
placement = L2-per-core affinity = already P25; sub-L3 (CCX) and NUMA are the same procurement-gated
DEFER; nothing here is a here-and-now build item beyond what P25 already owns.

### C-C. Local LLM as MoE-mirror (item 128) + core-resident STARK (2026 frontier)

**MoE-structural-mirror (does domain-expert-per-node sidestep doc 21's bandwidth wall?).** doc 21
correctly rejected (a) layer-sharding one model and (b) raw-result gossip, both memory-bandwidth-bound.
The operator's *distinct* variant — different mesh nodes = different **functional-domain** experts
(courier-routing / pricing / fraud), each a **small specialized** model, gossiping only on cross-domain
queries — is architecturally different, and here is the honest evaluation:
- **It is NOT token-level MoE.** Standard MoE's "memory-bandwidth challenge" (the literature) is a
  learned router dispatching *each token* to experts across GPUs — that is doc 21's within-one-model
  wall, per-token cross-device traffic. The operator's variant is **query-level / domain-level**: a
  whole query runs whole on ONE node's small model, entirely locally, **zero per-token cross-node
  traffic**, cross-domain only per-query (coarse). [RESEARCHED — NVIDIA MoE glossary; MoETuner]
- **Does it sidestep the wall? Partially YES — and it resolves to two things the repo already has:**
  1. **A small per-domain model (e.g. a fine-tuned 3B pricing expert) streams a much smaller weight
     set per token** than a general 8B → proportionally lower per-token bandwidth, faster decode — a
     real per-domain win independent of any distribution.
  2. **Most domain decisions are exactly the judgment-shaped, *compilable* class.** So the domain-
     expert model is the **build-time oracle that compiles a DecisionUnit**, and the runtime is the
     ns-native unit (zero tokens — the bandwidth wall does not apply at all). For its compilable core
     the MoE-mirror **collapses into DecisionUnit gossip** — the same resolution doc 21 reached. For
     the non-compilable tail it is **throughput-across-independent-domain-jobs** (summed bandwidth,
     doc 21 §4's one honest win), gated on a second live node + an offline/sovereign requirement.
- **The one hard constraint:** domain routing must be by **declared capability/topic tag**, never by
  "which node is better" — a *quality* router is a courier-score (NO-COURIER-SCORING, Cheng–Friedman).
  Domain = a static capability, safe; quality-rank = forbidden.
- **Verdict (item 128): ADOPT-AS-REFRAME** — it is DecisionUnit gossip (compilable core) + domain-
  partitioned throughput (non-compilable tail), routed by capability tag. Not a new distributed-
  inference engine; it sidesteps the wall precisely by routing to a *compiled unit*, and for the tail
  it gets throughput (never single-request latency).

**Core-resident STARK proving (2026 — is periodic checkpoint-proving realistic on the SAME cores?).**
Doc 20 already corrected the stale 10⁶× to **~10⁵× (Jolt CPU frontier)** with GPU real-time (SP1
Hypercube: 32M-gas block in 10.8 s). New this session:
- **Per-message: still NO.** A tiny decide-circuit at 10⁵× is still ~tens of ms of prover work ≫ the
  ~71 µs verify it would replace (~100–1000× too slow, per message). [RESEARCHED — a16z Jolt; doc 20]
- **Periodic checkpoint on the owner hub's spare cores: realistic, no GPU.** Jolt proves at ~100 kHz
  effective (M3 Max); a periodic "events N..M applied correctly against the FSM" checkpoint of ~10⁶
  steps ≈ **seconds** of full-CPU proving. On a 4-core hub that is a *few-second CPU-bound burst,
  off-peak*, that **queues through the same P25 4-slot compute budget** as any heavy kernel job — it
  does **not** need a GPU (GPUs are for real-time *per-block* proving, not an hourly delivery-mesh
  checkpoint). **Verification is milliseconds** (doc 20), so couriers verify cheaply.
  [RESEARCHED — Jolt small-space eprint 2025/611 (<2× overhead, small footprint, no recursion);
  Binius small-field turn eprint 2026/1371 (5–10× further, PQ-consistent hash-based)]
- **This maps the mesh's heterogeneity onto item 46 exactly:** the owner hub (the one node with spare
  cores) **proves** the periodic checkpoint-STARK; phones **verify** in ms. PQ-consistent (STARK/hash-
  based only — never a pairing-SNARK), off the hot path, gates no delivery decision.
- **Verdict (items 41/42/46/60): per-message ZK stays REJECT/DEFER; periodic checkpoint-STARK on the
  hub's core-budget is DEFER-WITH-TRIGGER with the trigger now much closer** (a real periodic FSM-
  replay-audit need + a second live node), **no GPU required.**

---

## §B. THE 185-ITEM VERDICT LEDGER (every item; zero silent drops)

Verdict vocab: **ADOPT** · **EXTEND-EXISTING** · **ALREADY-EQUIVALENT** (incl. already-built/-doctrine)
· **DEFER-WITH-TRIGGER** · **REJECT-ON-PHYSICS** (physics/correctness/determinism/absent-hardware/CI-
red-line only). **SPLIT** = the item contains ≥2 sub-ideas with different verdicts. **MERGE →N** = same
mechanism as item N (not a drop — a de-duplication). Grounds cite the batch that read the code live
(B1=10, B2=11, B3=12, B4=13, B5=14v2, B6=15, B7=16, B8=17, B9=18; d19/d20/d21 = correction docs; §C =
this doc's Step-2 findings). Build slot → §F.

### B.1 CONSENSUS / NEGOTIATION (1–14)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 1 | Market-based micro-negotiation (auctions, bid-priority for CPU/data/bw) | **REJECT-ON-PHYSICS** | transient-acquirable-weight/MEV class (Beanstalk-shaped; measured sandwich steady-state); assignment already coordination-free via rendezvous/HRW under capability auth (`matcher.rs`); narrow sealed-batch commit-reveal permitted-if-ever-needed. B4 §2.1 | — |
| 2 | Inline recursive proofing during `fold_transitions` (self-auditing) | **SPLIT: naive REJECT / steelman ADOPT** | naive self-cert = RC-2 (the check restates the claim); steelman inline-**commitment** (WorkReceipt: emit `H(inputs),law_id,H(output)` — no validity claim — independent party re-runs the external law) rejects the forgery via replay. doc20#2 FLIP; = W3-L2 | W3-L2 |
| 3 | Flocking/swarm coordination within one tick | **ALREADY-EQUIVALENT** | anti-entropy gossip (`discovery.rs:148-255`) + spectral convergence (`mesh_consensus.rs`); boids dissolves into these. B5 §1.1 · B4 §2.4 · B6 gap iii | W4-L6 |
| 4 | Speculative Consensus (assume-valid, prepare, cheap rollback) | **REJECT-ON-PHYSICS** | degrade-OPEN in a degrade-closed arch; doc20#1 STAYS on a *measured* reason: local verify ~10 ns, commit dominated by ~80 ns SHA3, speculation delivers **no speedup (±noise)**, only the cheap reversible part is speculatable. d20#1 · B4 §2.5 | — |
| 5 | Inline cryptographic witnessing (hash-chain / sig-aggregate as proof-of-transition) | **SPLIT** | hash-chain proof = ALREADY-EQUIVALENT (`event_log` content-id chain); sig-**aggregate as free** = REJECT (item C9, SSR-2020, 3.26× slower); steelman commitment = ADOPT (→2). §C-A: SIMD makes the *mandatory single* verify faster, safely. MERGE→2 | W3-L2 |
| 6 | Commitment-based challenge protocol (neighbors verify a commitment; fraud-proof on mismatch) | **SPLIT** | commitment + independent replay ADOPT (= import-replay/WorkReceipt); the **optimistic fraud-proof** leg REJECT (none worked in ~3 yr). B4 §2.5 · B6 §1.3. MERGE→2,27 | W3-L6 |
| 7 | State-transition proofs as hot-path artifacts (signed diffs, "free" signatures) | **SPLIT** | "free signatures" premise REJECT (C9: one full verify per item; §C-A crypto floor ~71 µs); signed-diff-as-artifact ADOPT in WorkReceipt form. B4 §3.1 · §C-A | W3-L2 |
| 8 | Optimistic Mesh (execute-first, revert-on-challenge) vs strictly deterministic | **REJECT optimistic / ADOPT deterministic** | optimistic pole = item 4 class; strictly-deterministic verify-before-persist is the built pole (`event_log.rs:389`). MERGE→4 | — |
| 9 | CRDTs, Vector Clocks | **SPLIT** | commutative half ALREADY-EQUIVALENT (signed G-Set CvRDT `sync_pull.rs:522-621`; version-vector watermark `:487-491`); money/order half REJECT-ON-CORRECTNESS (never CRDT-merge, `event_log.rs:4`). B2 §1/§2 | W1-L8 |
| 10 | "As above so below" micro/meso/macro trust-by-verification-frequency | **ALREADY-EQUIVALENT (framing)** | verify-before-persist at every layer = the verification-frequency discipline; the load-bearing form is key_V. No new surface. MERGE→2,169 | — |
| 11 | Challenge-cost vs malicious-action-cost economics | **EXTEND-EXISTING (design axiom)** | the honest correction is C9 (batch/quorum verify NOT free) + permissionless fraud-proofs REJECT (B4 §2.5); folds into WorkReceipt + fact-triggered revocation, not an economic challenge market | — |
| 12 | Eventual Consistency / Gossip (local-first, CRDTs, state-vector+hash gossip) | **ALREADY-EQUIVALENT** | the commutative SyncPeer set-union + `snapshot_root`; B2 §0. = v1 #5 | — |
| 13 | Fast Finality / Witness (quorum intersection, in-line verify, atomic commit) | **ALREADY-EQUIVALENT** | finality is local-and-explicit; atomic `Settlement` = HTLC (Herlihy PODC'18); quorum-intersection BFT dissolves (no global head). B4 §2.4 | — |
| 14 | Tiered/Hybrid consistency (Fast-Finality Command / Eventual Data) | **ALREADY-EQUIVALENT** | the commutative/non-commutative split IS this (`event_log` vs `sync_pull`; ADR F-1). B2 §0. = v1 #5 | — |

### B.2 PRIORITY & DISPATCH (15–18)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 15 | Priority enum (Critical/Telemetry/System) as a type on every Transition | **ADOPT-AS-COMPOSITION** | nested `TokenBucket` envelopes `BTreeMap<(PeerId,CapabilityClass),TokenBucket>`; the wire flag = envelope selector checked against capability scope, **never self-assigned** (self-assign = RC-2 fast-lane). B4 §2.6 = v1 #6 | W3-L3 |
| 16 | Dual-Track (Fast-Path/Flow-Path) dispatcher | **ADOPT** | two capability-scoped buckets. MERGE→15 | W3-L3 |
| 17 | Head-of-line-blocking risk of flat priority | **ALREADY-EQUIVALENT** | the per-(peer,class) bucket map structurally avoids HOL; the risk is real, the envelope design answers it. MERGE→15 | W3-L3 |
| 18 | Differentiated routing, adaptive drop, **selective cryptography by priority tier** | **SPLIT** | differentiated routing / adaptive-drop ADOPT (envelope + P25 PSI admission); **selective crypto REJECT-ON-CORRECTNESS** — every recv verifies both legs (RequireBoth); skipping crypto for a tier discards authenticity (§C-A). | W3-L3 |

### B.3 DECISIONUNIT GOSSIP / JIT (19–23)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 19 | Gossiping compiled DecisionUnits | **ALREADY-EQUIVALENT-in-design + ADOPT distributed extension** | = the Decision Compiler (latency §2 is authority; 4 repo precedents verified); doc21 confirms transport-ready (`SyncFrame` ≤1 MiB). B6 §1.1 = v1 #7 | W3-L6 |
| 20 | DecisionUnit context-validity metadata (don't misapply a unit compiled for other conditions) | **ADOPT (new ground)** | provenance header carries the compiled-for conditions; an out-of-context/stale unit answers `Escalate` unconditionally. B6 §1.2. MERGE→22 | W3-L6 |
| 21 | Epochs/versioning for DecisionUnits, invalidate stale | **ADOPT (new ground)** | monotone logical epoch = the gossip epoch, max-merge, no wall-clock. B6 §1.2a = v1 #8 | W3-L6 |
| 22 | Self-healing DecisionUnits (auto-reject degrading, request recompilation) | **SPLIT** | auto-reject-stale-and-Escalate ADOPT (structural, no monitor); **auto-detect degradation by quality-scoring REJECT** (a score + a watchdog). Healthy form = drift-gate/import-replay reject. B6 §1.2 · d19 | W3-L6 |
| 23 | Merkle-DAG versioning for DecisionUnit rollback lineage | **ADOPT-AS-SINGLE-AUTHORITY** | doc20#6 REFINED: DAG-shaped lineage of *new* facts is fine; put it **inside** the existing sha3 content-addressed registry, NOT a second authority. d20#6 · B6 §1.2c (flips v1 #E20) | W3-L6 |

### B.4 PROOF OF QUALITY (24–28)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 24 | Statistical/Metric PoQ (sandbox micro-test, metrics passport, Shadow-Mode validation by receivers) | **SPLIT** | receiver Shadow-Mode independent replay ADOPT (= import-time replay, verify-before-persist); **metrics-passport / statistical vote REJECT** (reputation/quorum aggregate, Cheng–Friedman). B6 §1.3 · B4 §2.2 | W3-L6 |
| 25 | Cryptographic/Quorum PoQ (N validators, threshold sig before gossip) | **REJECT-ON-PHYSICS** | no deployed ML-DSA aggregate/threshold; FIPS 204 single-verify only; BLS not PQ (item C10). §C-A: SIMD speeds the mandatory *single* verify, it does not resurrect aggregation. B4 §2.2 | — |
| 26 | Semantic/Contractual PoQ (WASM/strict-API sandbox, physically-can't-violate-bounds) | **ADOPT** | = WorkReceipt + the microvm/fuel physical-invariant sandbox (both built: `microvm.rs`, `fuel.rs`). B4 §2.2 · B3 §7 | W3-L2 |
| 27 | Optimistic PoQ with Fraud Proofs | **REJECT-ON-PHYSICS** | degrade-open; no permissionless fraud-proof functioned in ~3 yr; per-artifact import-replay is the safe substitute. B4 §2.5 · B6 §1.3 | — |
| 28 | Recommended hybrid (Semantic Contract + Optimistic PoQ) | **SPLIT → corrected hybrid** | keep the semantic-contract half (WorkReceipt + **import-time independent replay**), drop the optimistic half. B6 §1.3 | W3-L6 |

### B.5 STATE JOURNALING / ROLLBACK / DISPUTE (29–40)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 29 | State journaling / checkpoint via Arc/persistent-DS before applying a unit | **ALREADY-EQUIVALENT** | `snapshot_payloads`/`rebuild_from_payloads` (`event_log.rs:115-134`); verify-before-persist keeps the pre-image intact by construction. B2 §10 = v1 #10 | — |
| 30 | Async out-of-band Watchdog / audit task | **REJECT-ON-DOCTRINE** | **this is the watchdog the operator forbade.** doc19 Part 2: inline verify-before-persist is categorically better (no liveness regress; absence-is-visible). The audit that IS allowed = import-time replay / fact-triggered revocation (event-driven, not standing). d19 · B6 §4 · B7 §5 | — |
| 31 | Fraud Proof struct `{UnitID,WitnessSig,EvidenceType}` + Proof-of-Fraud must cost something | **SPLIT** | the fraud-PROOF-as-verified-fact triggering revocation ADOPT (structural, event-driven, B7 §5); the optimistic challenge-window it implies REJECT (B4 §2.5); spam-guard = the existing signature/capability gate, not a new economic layer | W3-L7 |
| 32 | Epoch/StateVersion field to bound divergence | **ADOPT** | = the logical epoch (item 21); bounded-drift window = item 161. B2 §8. MERGE→21 | W2-L4 |
| 33 | Local (not global) Peer Trust Matrix (Convergence Rate/Latency/Data Integrity) | **REJECT-ON-PHYSICS** | a peer matrix scored on observed behavior IS courier-scoring — Cheng–Friedman symmetric-scoring + CI `NO-COURIER-SCORING` (`claim_machine.rs:13-17`). "Local" does not exempt it. B4 §1 · B7 §2 | — |
| 34 | Majority-Metrics Reconciliation (TrustScore-weighted vote, minority penalized) | **REJECT-ON-PHYSICS** | reputation-weighted vote = C1/C2; Cheng–Friedman. B4 §2.4 | — |
| 35 | Sybil/metric-gaming defense (Proof-of-Compute/Consistency, Decay Factor, Network-Diversity anti-echo) | **SPLIT: reputation forms REJECT / real defense ADOPT+AUGMENT** | diversity/decay factors are symmetric-scoring tweaks *inside* Cheng–Friedman → REJECT. Real Sybil defense = **asymmetric anchor-rooted issuance** (`verify_chain`, PROVEN-VIABLE, B7) **+ hardware-attestation-as-augmentation** (doc14v2#5 FLIP: courier phones have StrongBox/Secure-Enclave; prices Sybil-minting at real-device cost *under* anchor issuance; degrade-closed; new Google/Apple attestation root is the named tradeoff). B7 · d14v2#5 | W3-L7 |
| 36 | Rust-level `AtomicU64/AtomicF32` TrustScore, zero-copy fingerprints, ArcSwap lock-free reads | **REJECT (score) / re-home the primitive** | it implements the forbidden trust score (33–35); `AtomicF32` also breaks determinism. The lock-free-read PRIMITIVE (ArcSwap/atomic-swap) is real and re-homes to item 44/107 (atomic tile handoff), never a trust score | — |
| 37 | Trust-Weighted Dispatcher (High/Med/Low → immediate/shadow/verify-queue) | **REJECT the trust-weighting / ADOPT capability-weighting** | trust-weight = C1; the **capability**-scoped envelope (item 15) is the sound weight; shadow/verify-queue = import-replay (item 24). MERGE→15,24 | W3-L3 |
| 38 | Event-Driven (triggered-only) Bisection — Dispute Channel, Prover/Verifier, bisect Merkle to divergent byte | **DEFER-WITH-TRIGGER / reputation half REJECT** | linear `diff` wins on round-trips until digest bytes dominate a measured round; the event-triggered shape is correct (matches fact-triggered discipline); reputation-weighting REJECT. B2 §6 = v1 #11 | W2-L5 gates |
| 39 | Metric-Aware Gossip (route Critical units only to high-trust nodes) | **REJECT (route-by-trust) / ADOPT route-by-capability** | routing by trust = score (C1); route by capability/domain tag (item 15/128). MERGE→15 | — |
| 40 | Layered hot-path / audit-layer / reputation-engine architecture | **SPLIT** | hot-path + audit-layer (import-replay) ADOPT; **reputation-engine REJECT** (C1). MERGE→24,30 | — |

### B.6 ZK / MATH ANCHORING (41–46)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 41 | ZK-SNARK/STARK Proof-of-Execution (risc0/sp1) for `fold_transitions` | **REJECT per-message / DEFER checkpoint-STARK** | doc20#3: number UPDATED 10⁶→~10⁵× (Jolt), still ≫ per-message verify; §C-C: periodic checkpoint-STARK realistic on the hub's spare cores, no GPU, PQ-consistent. d20#3 · §C-C = v1 #13 | defer reg |
| 42 | Proof-Anchoring (optimistic fast path + periodic anchor proof, Hard Revert on mismatch) | **SPLIT** | optimistic-fast-path REJECT (item 4/27); periodic-anchor-STARK-proof ADOPT-AS-DEFER (§C-C); "Hard Revert" = re-derive from last good (item 185). §C-C | defer reg |
| 43 | Merkle-Path reconciliation (request only differing branches) | **DEFER-WITH-TRIGGER** | = Merkle range-reconciliation; linear `diff` wins until bytes dominate. B2 §6. MERGE→38 | W2-L5 gates |
| 44 | Atomic Swap via Arc pointer swap | **DEFER-WITH-TRIGGER** | no `AtomicPtr` consumer today; RCU/hazard-pointer; folds into the predictive-handoff pass. = v1 #21b. MERGE→107 | W3-L9 |
| 45 | Merkle-DAG history chaining anchors to prevent a malicious anchor forging empty state | **ADOPT-AS-SINGLE-AUTHORITY** | the prev-link hash-chain + genesis anchor already prevent empty-state forgery; keep it one authority (item 23). B2 §4/§11. MERGE→23 | W3-L6 |
| 46 | Distributed Proving (1–2 powerful nodes prove, rest verify) | **ADOPT-AS-DEFER** | §C-C: the owner hub proves the periodic checkpoint-STARK on spare cores; couriers verify in ms — maps the mesh's heterogeneity exactly; PQ-STARK, off hot path. §C-C | defer reg |

### B.7 TENSOR GRAPH STATE (47–53)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 47 | Tensor graph replacing Merkle-tree as the state representation | **REJECT-AS-REPLACEMENT / EXTEND-for-compute** | a second content-addressed *authority* is the dual-authority hazard (ADR:44-50); state stays the sha3 log. The relation-tensor is the **compute** representation, keyed by the same content-address (item 14/A1). B2 §4 | W3-L1 |
| 48 | Spectral analysis (Laplacian eigenvalues) to find mesh bottlenecks | **ALREADY-EQUIVALENT** | `mesh_consensus.rs` Fiedler λ₂/SLEM on the capability graph; doc20#5 grounds it as the Laplace-Beltrami continuous limit ({k²} spectrum measured converging). Promote to advisory. d20#5 · B4 §3.2 | W4-L6 |
| 49 | Differentiable programming / gradient descent to optimize the mesh's own graph structure | **REJECT-ON-PHYSICS** | SGD is stochastic, non-bit-reproducible (determinism contract, `csr.rs:36-37`); and the graph is anchor-rooted delegation (a tree), not a learned topology — no consumer. B1 B12 class | — |
| 50 | Native tensor storage of ML-model DecisionUnits (no serialization) | **REJECT-general / EXTEND-narrow** | a compiled DecisionUnit is native Rust (zero-token) — "no serialization" already true for it; raw model weights are the >1 MiB payload the transport rejects (doc21 §3). The relation-tensor-as-CSR is the adoptable native tensor. d21 §3 · B1 A1 | — |
| 51 | Tensor Commitment (Pedersen) / EZKL for ZK-proving neural-net execution | **REJECT-ON-PHYSICS** | Pedersen is discrete-log (NOT PQ); EZKL zkML is the 10⁶× class; no neural net on the hot path (decisions are compiled units). B4 §2.7 | — |
| 52 | Cross-platform tensor-op non-determinism → fixed-point tensors or a deterministic-compute crate (Burn) | **ALREADY-EQUIVALENT (the concern IS the governing constraint) / Burn REJECT-as-dep** | fixed-summation-order f64 + integer-where-cross-target is the contract; **this is bridge-gap #1** (normalize-before-hash). Burn = a dep (DECART/zero-dep posture); the integer-CORDIC form (doc20#4) is the deterministic-transcendental substrate. B6 §2.2 · d20#4 · §A | W1-L10 |
| 53 | Gradient-based reconciliation (cosine/L2 distance, exchange gradients, soft not hard-revert) | **REJECT-ON-CORRECTNESS** | soft gradient-merge over non-commutative state = the mergeable-CRDT-over-money hazard; gradient exchange is stochastic. The commutative half already converges by exact set-union. B2 §1b | — |

### B.8 SPARSE TENSOR ENGINEERING (54–62)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 54 | Sparse vs Dense storage, indexing for thousands of nodes | **EXTEND-EXISTING** | CSR is it (`csr.rs`). B1 A1 | W3-L1 |
| 55 | FlatBuffers zero-copy serialization | **REJECT-AS-DEP** | adds a codegen dep; the repo's fixed-layout TLV / length-prefixed envelope (`framing.rs`) is zero-dep, byte-deterministic, and the zero-copy intent is met by the content-addressed opaque payload. DECART | — |
| 56 | Graph pruning (remove stale/zero-influence weights) | **ADOPT** | the arena snapshot-then-drop prunes dropped edges into a retained base (blueprint arena W5). MERGE→151 | W2-L1 |
| 57 | Canonical Sorted-COO ordering for consistent cross-node hashing | **ALREADY-EQUIVALENT + BRIDGE** | `from_edges` sorts + merges-by-sum (deterministic, `csr.rs:92-103`); the **normalize-before-hash bridge (#1)** is the missing wiring that makes the hash cross-node-meaningful. B1 A1 · §A | W1-L10 |
| 58 | COO for gossip/patches, CSR for compute | **ALREADY-EQUIVALENT** | the edge-tuple contract IS the COO layer; `from_edges`→CSR; no new struct. B1 A1 | — |
| 59 | Sparse Delta Updates (added/removed elements, not full tensor) | **ALREADY-EQUIVALENT** | anti-entropy ships deltas not snapshots (`anti_entropy.rs:75-107`). B2 §5 | — |
| 60 | ZK-proof speedup from sparsity (circuit only traverses non-zeros) | **DEFER** | rides the checkpoint-STARK defer (items 41/46); real for the periodic proof circuit, not per-message. §C-C | defer reg |
| 61 | Topology-drift handling — fixed-size buffer with masking (avoid realloc on structural change) | **ADOPT** | arena + sentinel-masking (A8 arena + A3 branchless). MERGE→63,69 | W2-L1 |
| 62 | Z-order/Morton indexing for spatial locality | **DEFER-WITH-TRIGGER** | no consumer at n≈10²–10³; trigger = blocked traversal over >L2 grid. B1 A2 = v1 #16a | defer reg |

### B.9 MEMORY / HEAP (63–77)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 63 | Zero-allocation hot path via arena/bump/slab | **ADOPT** | build P28 `arena.rs` `BumpArena` exactly (verified absent); extend with `HugePageHint` seam. B1 A8 = v1 #16d | W2-L1 |
| 64 | Roaring Bitmaps for sparse index ops (SIMD AND/OR/XOR) | **DEFER-WITH-TRIGGER (algorithm) / REJECT-as-dep** | no consumer at n≈10²–10³ (CSR `col_idx` sets are small); trigger = a large sparse-set intersection on a hot path; the *algorithm* re-homes to the branchless/SIMD mask pattern (A3), never a crate | defer reg |
| 65 | Block-sparse SIMD packing (4×4/8×8) for SpMV | **DEFER-WITH-TRIGGER** | nnz*16 > L2 AND measured memory-bound; `simd.rs` pattern extends here. B1 A6/A3 | defer reg |
| 66 | Software prefetch (`_mm_prefetch`, N-ahead) | **DEFER-WITH-TRIGGER** | `csr.rs:180-183` site; trigger nnz*16 > L2 AND spmv profiled memory-bound. B1 A6 = v1 #16b | defer reg |
| 67 | SoA over AoS | **ALREADY-EQUIVALENT + REAL FINDING** | `simd.rs` is SoA-across-batch; fix `engine/src/zerocopy.rs:22` AoS-mislabel. B1 A4 = v1 #E14 | W1-L6 |
| 68 | CPU affinity / NUMA pinning | **SPLIT (see §C-B, RESOLVED)** | CPU affinity ALREADY-DECIDED (P25 CORE-BOUND `taskset -c 0,2,4,6` = L2-per-core placement, the real single-socket cache-domain zone); NUMA REJECT-as-no-op (measured 1 socket/1 node); CCX/L3-slice pseudo-NUMA DEFER (procurement-gated, no sub-L3 domain on this host or realistic hub). §C-B · d14v2 #6 | — |
| 69 | Branchless (cmov, masking, sentinel padding, asm-verified) | **ALREADY-EQUIVALENT + EXTEND** | `simd.rs:79-90,118-124` padding proven; extend to CSR/GEMM skips byte-identity-gated. B1 A3 = v1 #15 | W2-L7 |
| 70 | HugePages/THP (`MAP_HUGETLB` + `madvise MADV_HUGEPAGE`) | **DEFER-WITH-TRIGGER (via NoOp port)** | `HugePageHint` mirroring `core_pinning.rs:41-64`; trigger = persistent arena region > 2 MB; madvise available on the owner-hub but no >2 MB footprint yet. B1 A7 = v1 #16c | W2-L1 seam |
| 71 | Tiling (block data to fit L2) | **DEFER-WITH-TRIGGER** | no dense matmul n≥128 on a hot path (`mat.rs` consumers n≤32). B1 A9 = v1 #16e | defer reg |
| 72 | Delta/relative-address encoding for sorted sparse indices | **DEFER-WITH-TRIGGER** | a `col_idx` compression; no consumer at current scale; couples to A2/arena | defer reg |
| 73 | Hardware-aware look-ahead prefetch window | **DEFER-WITH-TRIGGER** | = A6 with a tuned distance; same trigger. MERGE→66 | defer reg |
| 74 | 3D-spatial memory tiling (tile size = HugePage, eliminate TLB misses) | **DEFER-WITH-TRIGGER** | composes A7+A9; "tile = HugePage-backed arena region"; same triggers. B1 A7/A9 = v1 #17 | defer reg |
| 75 | Sparse tiled allocation (table of tile pointers, dynamic growth) | **DEFER-WITH-TRIGGER** | the arena `_in` variants + a tile-pointer table; rides arena W2-L1 + A9 | defer reg |
| 76 | Tiled-outer + Morton-inner indexing ("golden middle") | **DEFER-WITH-TRIGGER** | composes A9 outer + A2 inner; no consumer. MERGE→62,74 | defer reg |
| 77 | Direct 3D-coordinate → physical-address mapping function | **DEFER-WITH-TRIGGER** | the Morton/tiling address fn; no 3D-spatial consumer until the predictive-handoff pass (W3-L9) defines one. MERGE→62,104 | W3-L9 |

### B.10 TOKEN / CONTEXT COMPRESSION (78–81)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 78 | Semantic mipmapping of token streams (LOD, raw=L0, pooled=higher) | **EXTEND-EXISTING** | ~85% exists: retrieval tiers, spectral coarsening (P28 rung1), CDC dedup, skeleton-LOD; literal box-filter mipmap REJECT; residual pooling primitive DEFER after P28 rung1. B6 §3 = v1 #18 | defer reg |
| 79 | Hierarchical addressing (cached root summary, LOD tiles, raw) | **EXTEND-EXISTING** | = the retrieval ladder + content-address. MERGE→78 | defer reg |
| 80 | Partial loading (LOD header first, stream raw on demand) | **ALREADY-EQUIVALENT** | the Repowise skeleton→symbol→range LOD is exactly this, in production as tooling. B6 §3.1 | — |
| 81 | Downsampling method: mean-pool vs SVD/PCA vs LLM-distillation | **SPLIT** | SVD/PCA ADOPT (= spectral coarsening/`topk_symmetric`, deterministic); mean-pool DEFER (anti-alias per Nyquist B5 first); LLM-distillation = a decode-token cost (latency-blueprint decode discipline, not a kernel primitive). B6 §3.2/§3.3 | W2-L2 |

### B.11 SYSTEM-LEVEL FRAMING (82–83)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 82 | "Stack Продуктивності" (Logic/Computation/Memory/IO) layered view | **ALREADY-EQUIVALENT (framing)** | maps to kernel/engine/arena/transport; no build surface | — |
| 83 | Hardware-software co-design / infra-sovereignty / latency-first framing | **ALREADY-EQUIVALENT (framing)** | the sovereign-architecture roadmap + offline-buildable/no-C-supply-chain posture is this. No build surface | — |

### B.12 NETWORKING / RDMA / DSM (84–103)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 84 | RDMA bypass TCP/IP entirely | **REJECT-ON-PHYSICS** | no RNIC on dev/hub; phone peers topologically impossible; CPU-bypass contradicts verify-every-recv; <10% of crypto-bound cost. d14v2 item 1 = v1 #19 | — |
| 85 | Distributed Shared Memory / VGAS | **REJECT-ON-PHYSICS** | rides RDMA; no shared-memory fabric across phone+hub; the mesh is message-passing content-addressed. MERGE→84 | — |
| 86 | Consistent hashing mapping 3D tensor coords to nodes | **DEFER (partial-ALREADY: HRW)** | rendezvous/HRW hash exists (`matcher.rs`); "3D coords" needs the predictive-handoff spatial model (W3-L9) first. B4 §2.1 · d14v2 | W3-L9 |
| 87 | Mesh-prefetching at network level (predict need from topological traversal) | **DEFER** | = the predictive tensor handoff pass (W3-L9), ghost-prefetch. MERGE→104,105 | W3-L9 |
| 88 | Data-centric vs service-centric mesh distinction | **ALREADY-EQUIVALENT** | the mesh IS data-centric (content-addressed events, not RPC). Framing | — |
| 89 | Zero-copy networking (RDMA / io_uring) | **REJECT-RDMA / DEFER-io_uring-for-storage** | doc14v2 item 2: io_uring *loses* at few-long-lived-sockets (measured 417 ns batch=1); its only earn case is syscall-heavy local file-I/O (arena/block-store). d14v2 item 2 | defer reg |
| 90 | Tensor/tile migration between nodes for load balancing | **DEFER** | = predictive handoff (W3-L9); load-balance by capability/domain, never by score. MERGE→104 | W3-L9 |
| 91 | Merkle-tiling (per-tile root hash for cross-node trust) | **EXTEND-EXISTING + BRIDGE** | `matrix_content_address` per tile IS a per-tile root; **bridge-gap #1** (normalize-before-hash) makes it cross-node-meaningful. §A. MERGE→57 | W1-L10 |
| 92 | DPDK vs AF_XDP ("AF_XDP golden middle") | **DPDK REJECT-ON-TARGET / AF_XDP DEFER-WITH-TRIGGER** | doc14v2 item 2: DPDK possible on a 2-NIC hub but crypto-dominance + phone-peers + power make payoff negative; AF_XDP near-zero payoff on a crypto-bound QUIC mesh. d14v2 item 2 | defer reg |
| 93 | Custom 32-byte L2 Ethernet frame (MAC/EtherType/TileID/Seq/Payload/CRC32) | **REJECT-carrier / ADOPT-fields-in-envelope + LAN-UDP discovery** | no shared L2; phones can't emit raw frames (weak endpoint); cleartext-header = security regression; fields (TileID/EpochID/HypothesisID/Seq) go **inside the signed envelope**; co-located-WiFi intent → LAN-UDP/mDNS discovery (doc14v2#3 partial flip). d14v2 item 3 = v1 #20 | W2-L4 |
| 94 | MTU segmentation-at-source + zero-copy reassembly into dest HugePage offset | **REJECT-ON-PHYSICS** | rides AF_XDP-umem zero-copy the phone can't do; QUIC already segments. MERGE→84,102 | — |
| 95 | Selective NACK (silence=success, not per-packet ACK) | **ALREADY-EQUIVALENT** | anti-entropy is pull-based/idempotent — a missing frame is re-pulled by Merkle-root divergence; QUIC handles reliability. B2 · `discovery.rs` | — |
| 96 | Hardware PTP timestamping for nanosecond time-sync | **REJECT-ON-PHYSICS** | no PTP-capable NIC on virtio/phone; and wall-clock is forbidden on the ordering path (item 159/22) — the logical epoch replaces it. B2 §7 | — |
| 97 | RoCE (RDMA over Ethernet) CPU-free write-to-remote-RAM | **REJECT-ON-PHYSICS** | = RDMA + needs a lossless fabric; CPU-free write contradicts verify-every-recv. MERGE→84 | — |
| 98 | Tensor-aware RSS/flow-steering via eBPF/XDP (`XDP_REDIRECT` by TileID to owning core) | **eBPF AVAILABLE-on-hub-DEFER / RSS REJECT-on-virtio** | doc14v2#4: XDP loads+JITs here (CAP_BPF works) but no measured steering need + sovereign-toolchain cost + phones-can't; the routing *logic* = userspace admission (P25). d14v2 item 4 | defer reg |
| 99 | "Network Mipmapping" — adaptive LOD flag in the packet header, congestion-triggered downgrade | **SPLIT** | the LOD flag **inside the signed envelope** ADOPT (rides item 78 + control-flag bitmask 158); congestion-downgrade must stay degrade-closed and must not become a quality score. MERGE→78,158 | W2-L4 |
| 100 | Zone-based hardware packet filtering by coordinate range | **REJECT-ON-PHYSICS** | needs hardware ntuple steering, absent on virtio (= RSS). MERGE→98 | — |
| 101 | `XDP_TX` for near-hardware-speed replies | **DEFER** | rides eBPF-on-hub (item 98); phone-incapable; no measured need. MERGE→98 | defer reg |
| 102 | AF_XDP umem = same memory as HugePage tile (network-to-memory zero copy) | **REJECT-ON-PHYSICS** | phone endpoint can't AF_XDP; optimizes transport that is <10% of crypto-bound cost. MERGE→92,94 | — |
| 103 | Batch SIMD/AVX-512 processing once a tile fully arrives | **EXTEND-EXISTING (AVX2, not AVX-512)** | **AVX-512 is UNAVAILABLE here** [MEASURED]; the AVX2 SoA-batch lane (`simd.rs` f64x4) IS the adoptable form — and this is the SEAM to §C-A's AVX2 crypto-verify batch. B1 A3 · §C-A | W2-L7 |

### B.13 PREDICTIVE TENSOR HANDOFF — moving physical assets (104–123)

> The one significant concept **no batch audited as a coherent unit** (B6 gap i). All of 104–123 land
> in the **W3-L9 research pass**, grounded on the existing Kalman substrate (`geo.rs::ema_next` IS a
> 1-D Kalman) + the Gaussian-Splatting arc. Decisive framing (repeated because it recurs): the
> "probability weight" is over **spatial STATE hypotheses** (particle filter / beam search), **not over
> agents** — a physics prior, **NOT a NO-COURIER-SCORING violation** (B6 §5.2-i).

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 104 | Tile-responsibility migration following a moving object (drone/courier) | **ADOPT-AS-RESEARCH-PASS** | the coherent unowned concept; Kalman + splatting substrate. B6 gap i = v1 #21 | W3-L9 |
| 105 | "Ghosting" — predictive prefetch from a physics/velocity vector | **ADOPT-AS-RESEARCH-PASS** | velocity-prior prefetch on the Kalman substrate. MERGE→104 | W3-L9 |
| 106 | Dirty-bit tracking (replicate only changed 4 KB pages) | **DEFER** | = COW page replication; rides item 154; W3-L9 for the tile case | W3-L9 |
| 107 | Atomic pointer swap for ownership handoff (`compare_exchange`+fence+purge) | **DEFER-WITH-TRIGGER** | no `AtomicPtr` consumer today; RCU/hazard-pointer. = v1 #21b. MERGE→44 | W3-L9 |
| 108 | Physics-vector in packet header for hardware (XDP) handoff triggering | **SPLIT** | physics-vector **inside the signed envelope** ADOPT (part of W3-L9); the hardware-XDP trigger REJECT (phone-incapable, item 98) | W3-L9 |
| 109 | Probabilistic multicasting to weighted candidate nodes, single COMMIT/DROP on crossing | **DEFER** | W3-L9; weights over spatial-state hypotheses (physics prior), not node-reputation; COMMIT/DROP = the atomic handoff. MERGE→104 | W3-L9 |
| 110 | Copy-on-Write Master/Leased tile ownership (misprediction = free discard) | **DEFER** | W3-L9; COW + lease; misprediction-free-discard is the degrade-closed shape | W3-L9 |
| 111 | Lazy-pull fallback (urgent request + delta-sync) for an unpredicted target | **ALREADY-EQUIVALENT-shape** | anti-entropy pull + delta IS lazy-pull. B2 §5 | W3-L9 |
| 112 | Explicit `TileState` enum (Master/Leased/Invalid) | **ADOPT-AS-RESEARCH-OUTPUT** | a closed-enum state machine (the sound form); part of W3-L9 | W3-L9 |
| 113 | Hardware-offload (P4/TC) vs CPU/kernel logic — resolved toward CPU/kernel | **ALREADY-EQUIVALENT** | the dialogue's own resolution matches the repo (routing logic is userspace/kernel-Rust; P24 ports eBPF ideas to userspace; item 98) | — |
| 114 | Atomic-versioned tile header (`AtomicU32` state + `AtomicU64` version + `AtomicU32` owner, cache-line-aligned) | **DEFER-WITH-TRIGGER** | needs `repr(align(64))` (A5) + an `AtomicPtr` consumer; W3-L9. MERGE→107 | W3-L9 |
| 115 | Epoch-based invalidation (single INVALIDATE packet, zero-overhead-until-touched) | **ADOPT** | = the logical-epoch invalidation; a stale-epoch tile answers Escalate/re-pull; no monitor. MERGE→21 | W3-L6 |
| 116 | True zero-copy direct memory handoff (map XDP RX descriptor into the tensor page table) | **REJECT-ON-PHYSICS** | phone can't XDP; = AF_XDP-umem. MERGE→102 | — |
| 117 | 64-bit bitmap ACK for batched packet confirmation | **DEFER** | QUIC already ACKs; no measured need; rides item 95. MERGE→95 | defer reg |
| 118 | Shadow-Tile multi-hypothesis simulation (active+shadow dual buffer, lock-free ping-pong) | **ADOPT-AS-RESEARCH-PASS** | particle-filter/beam-search over spatial hypotheses; `field_frame` three-buffer rotation is a ping-pong precedent. B6 gap i = v1 #21 | W3-L9 |
| 119 | Tensor "rollouts" as a batched SIMD forward-pass across hypotheses | **EXTEND-EXISTING** | the SoA-batch SIMD lane (`simd.rs` f64x4) IS the substrate; ties to §C-A AVX2. B1 A4. MERGE→103,118 | W3-L9 |
| 120 | `HypothesisID` field routing packets into the correct shadow buffer | **ADOPT** | the HypothesisID field inside the signed envelope (v1 #20 fields); part of W3-L9. MERGE→93,104 | W2-L4 |
| 121 | Dynamic pruning of low-probability hypotheses | **ADOPT-AS-RESEARCH-OUTPUT** | beam-search pruning over spatial-state probability (NOT agent-scoring). MERGE→104,118 | W3-L9 |
| 122 | Telemetry-driven (jerk/acceleration) trigger for hypothesis count | **DEFER** | adaptive hypothesis count from motion telemetry (Kalman residual); W3-L9 | W3-L9 |
| 123 | Cross-node `CANCEL_HYPOTHESIS` broadcast | **ADOPT** | a control-flag in the signed envelope (item 158), gossip-propagated; part of W3-L9. MERGE→158 | W3-L9 |

### B.14 SIGNAL-PROCESSING / MATH SYNTHESIS (124–127)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 124 | FFT on the token stream, drop high-frequency "noise" before transmission | **REJECT-mechanism / redirect** | a token stream is not a band-limited continuous signal; FFT-truncating tokens is meaningless (= literal pixel-mipmap). SOUND form = spectral coarsening (topk eigenmodes ARE low-frequency content, item 78/81) + Nyquist anti-alias (B5). B6 §3.3 | W2-L2 |
| 125 | Big-O-aware design (O(1) direct address; O(p) inference cost) | **ALREADY-EQUIVALENT** | DecisionUnit = O(1)-native + content-address O(1) lookup + P28 arena. Framing. MERGE→19,82 | — |
| 126 | Multi-factor "mesh health" model (latency+memory-stall+CPU-load+prediction-uncertainty) | **SPLIT** | LOCAL resource health (PSI: memory-stall/CPU-load) ALREADY-EQUIVALENT (P25 WorkClass admission); prediction-uncertainty = the Kalman residual (item 104); combining into a **local self-gauge** is fine, a **cross-peer health rank is a score → REJECT**. B5 · B4 §1 | — |
| 127 | Eval-Gate discipline (Faithfulness/Hallucination/Contextual-precision) to gate a tile handoff BEFORE the atomic swap | **ADOPT-AS-VERIFY-BEFORE-PERSIST** | the eval-gate-before-commit IS the drift-gate/import-replay shape; the "eval-gate soft-stop" 3rd mesh-panic tier = Survival-Mode soft-refuse (closes B6 gap iv). The LLM-eval *metrics* are a build-time DecisionUnit-compile check, not a runtime score. B6 §5.2-iv. MERGE→24,162 | W3-L6 |

### B.15 LOCAL LLM / RGB CODEC / UNIVERSAL ENCODING (128–134)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 128 | Mixtral 8×7B (MoE) — activate-only-needed-experts mirrors the mesh | **ADOPT-AS-REFRAME (§C-C)** | the STRUCTURAL-MIRROR (domain-expert-per-node) is real and better than doc21's rejected variants; resolves to (a) small per-domain models = build-time DecisionUnit oracles → compilable core = DecisionUnit gossip (ns runtime, wall irrelevant); (b) non-compilable tail = throughput-across-independent-domain-jobs (summed bandwidth, doc21 §4); routing by DOMAIN/capability tag, **never quality-score**. §C-C · d21 | W3-L6 |
| 129 | Llama-3-70B / Phi-3-Medium comparison points | **ALREADY-EQUIVALENT** | model-selection detail (LOCAL-AI arc benched the decode band); a build-time oracle choice, no new build surface. d21 | — |
| 130 | RGB-seed procedural tensor generation `T=f(seed,harmonics)`, O(1) regen | **SPLIT: harmonic REJECT / integer-CORDIC ADOPT** | harmonic/libm form REJECT-ON-PHYSICS (transcendental cross-arch non-determinism, `rng.rs:22-28`) BUT **doc20#4 FLIP: integer-CORDIC form ADOPT-able** (bit-identical, ~26-bit, disasm-proven float-free, cross-arch CI digest assertion); integer-PRNG seed + truncated-spectral `W≈U_kΛ_kU_kᵀ` ALREADY-EQUIVALENT. d20#4 · B6 §2 = v1 #E13 | W2-L2 |
| 131 | Layered "deployment as guarded system" (Policy→Content-Safety→PII-Redaction→Agent) | **ALREADY-EQUIVALENT** | sandbox/admission/red-line-scope layering (microvm tiers + `RedLinePolicy` + scope + zero-trust-AI item 177). B3 §7. MERGE→26,177 | — |
| 132 | Eval Gates as a formal admission check on generated tensor state | **ADOPT** | = verify-before-persist / import-replay on generated state (drift-gate + `noether::step_preserves`). MERGE→127,162 | W3-L6 |
| 133 | The seed/function pair reused as ONE universal codec across kernel-gen + decision-replay + UI-rendering | **SPLIT** | the "one codec everywhere" ambition is sound **only** as the integer-CORDIC/spectral generator (doc20#4), REJECT for the harmonic form (cross-arch determinism); the UI-render use = the physics-UI arc (no-DOM render of kernel state, v1 #E12). MERGE→130 | W4-L10 |
| 134 | Async Eval Gateway (producer→ring-buffer→continue; background Observer re-derives-and-compares; mismatch→invalidate) | **REJECT the async-Observer / ADOPT the sync form** | a background Observer that re-derives-and-compares IS a watchdog (doc19 Part2: inline verify-before-persist is categorically better, no liveness regress); ADOPT the SYNCHRONOUS re-derive-and-compare-inline (= drift-gate/import-replay). The SPSC ring (P24 `ring.rs`) is real, but the verify must be on the causal path. d19. MERGE→30,132 | W3-L6 |

### B.16 SAFETY ARCHITECTURE — the Monocoque arc (135–150)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 135 | Bus-Factor-1 / God-Architect risk | **ALREADY-EQUIVALENT (acknowledged risk)** | the operator IS the single anchor — mitigated by genesis-frozen roster + external-signature invariant updates (item 174); a governance fact, no build surface | — |
| 136 | Floating-point cross-arch drift → fixed-point mandate | **ALREADY-EQUIVALENT (the governing constraint)** | fixed-summation-order + integer-where-cross-target; eqc-rs Q-format; **bridge-gap #1**. §A. MERGE→52 | W1-L10 |
| 137 | CAP-theorem tradeoff in a physical/logistics context | **ALREADY-EQUIVALENT** | the commutative/non-commutative split IS the CAP-aware design (AP telemetry/gossip, CP-local money). B2 §0. MERGE→14 | — |
| 138 | Over-engineering risk ("tensor for everything") | **ALREADY-EQUIVALENT** | the DEFER-WITH-TRIGGER discipline + "tensor is the compute rep not the authority" (item 47) answers it (ponytail/YAGNI baseline, scoped by the perf-override) | — |
| 139 | Absence-of-fail-safe if the native kernel hangs → need a hardware-independent watchdog | **SPLIT: concern real / watchdog forbidden** | the concern is real (a hung kernel can't check itself — the liveness leg, doc19 §2.4); the **structural** answer is NOT a watchdog — it is the wasmtime `OutOfFuel` trap (runtime stops the guest) + restart-intensity-as-launch-path-predicate (T-6) + systemd `StartLimit` as substrate-physics. B3 §2 · T-6 | W3-L4 |
| 140 | Hardware Watchdog Timer (WDT) — apparatus-level, physically cuts power on missed heartbeat | **ALLOWED-as-hardware-substrate / REJECT-as-software-component** | a real hardware WDT is substrate-physics (same class as `OutOfFuel`/systemd `StartLimit`, the FPV-detonator item 176) — allowed as a hardware fact the owner-hub may have; the heartbeat-**poll software process** is the forbidden watchdog. d19 | — |
| 141 | Safety Cell/Bulkheads — CPU-pinning + separate memory regions | **ALREADY-EQUIVALENT** | process-per-hub tenant boundary = MMU bulkhead (Phase27 §1.1); P25 core-pinning; arena regions. B3 §3 | — |
| 142 | 3-sigma statistical health baselining per node | **SPLIT** | LOCAL self-baseline (this node's own ρ/drift band, `classify_drift` BAND) ALREADY-EQUIVALENT; a 3σ baseline **over peers** = a behavior score → REJECT. Keep it local. B3 · B4 §1. MERGE→126,147 | — |
| 143 | Chaos Injection agent (continuous fitness-testing of the Eval Gateway) | **SPLIT** | chaos-testing as a DEV/CI discipline ADOPT (RED-proof/fuzz culture; the seed-swept adversarial `sync_pull` n-node fuzz already exists); a **continuous runtime chaos AGENT is a standing process → REJECT** (zero-watchdog) | — |
| 144 | Per-peer-link Circuit Breaker (Healthy/Degraded/Open, XDP-level cutoff) | **SPLIT** | breaker PRIMITIVE ADOPT (build `breaker.rs` per P27 §3.2, EMA via `ema_next`); per-peer variant DEFER (needs P9/P10 seam); "XDP-level cutoff" REJECT (phone-incapable, item 98) — cutoff is a userspace envelope refusal. B3 §1 = v1 #24 | W2-L3 |
| 145 | Graceful Degradation / Survival Mode (drop speculative work, run only Master tile) | **ALREADY-EQUIVALENT** | Survival-Mode soft-refuse + drift-gate non-persist + Locked-readable ("Static Safe Tensor" ×3). B3 §4. MERGE→165 | — |
| 146 | Static read-only immutable-in-hardware Fallback baseline | **ALREADY-EQUIVALENT-in-software + substrate-physics-for-hardware** | the golden re-seed baseline (`hydra.rs`) is the software form; "immutable-in-hardware" (write-protected memory, item 168) is substrate-physics allowed if the hub has it, not a software-built guarantee. B3 §4 | — |
| 147 | Hash-based determinism-tolerance check (epsilon-bounded, not exact-match) | **ALREADY-EQUIVALENT** | `classify_drift` BAND=1e-6; `noether::step_preserves` \|I(f x)−I(x)\|≤tol. doc19 §1.4 | — |
| 148 | Mesh Panic Handler — LOCALIZED, not a global halt | **ALREADY-EQUIVALENT** | `OrganismState::Locked` isolates one node + `BreachAlert`; process-per-hub bulkhead. B3 §3 = v1 #24 | — |
| 149 | Formal Panic sequence (signal-mask→memory-fence→state-reversion→mesh "scream") | **ALREADY-EQUIVALENT-mostly** | `BreachAlert` broadcast = the "scream"; state-reversion = drift-gate non-persist / `boot_verify` replay; the signal-mask/fence detail is an impl nicety, not new architecture. B3 §3 | — |
| 150 | Question of Panic Handler placement (in-kernel vs separate privileged process) | **RESOLVED-BY-DOCTRINE: in-kernel** | the Locked state IS in the same computation; a separate privileged process = a watchdog (forbidden). doc19 Part2 | — |

### B.17 EPOCHS, SNAPSHOTS, AND THE ZERO-POINT SYNTHESIS (151–185)

| # | Item | Verdict | Ground / merge-note | Slot |
|---|---|---|---|---|
| 151 | Rolling/"Golden State" snapshot, cheap because regenerated from seed+delta not copied | **PARTIAL-EXISTING + ADOPT-the-bridge** | checkpoint/restore built (`snapshot_payloads`); cheap-regen rides the integer/spectral generator (item 130) + arena snapshot-then-drop; durable half = P12 gap; **bridge-gap #2 (drift-gate the snapshot)**. §A. MERGE→154,185 | W1-L11 |
| 152 | Drift correction via checksum-vs-expected-generated-value, auto-reseed past a threshold | **ADOPT-the-structural-form** | checksum-vs-expected = the content-address compare; auto-reseed-from-golden is `hydra.rs`'s corruption path — but it must be a degrade-closed **inline** consequence (drift-gate reject → re-derive), not a polling corrector. MERGE→147,185 | — |
| 153 | Soft/preventive reset distinct from the reactive Panic path | **ADOPT-the-distinction** | soft reset = the equilibrium/zero-point correction rhythm (items 180/181); reactive = Locked/kill-switch; keeping them distinct is the operator's own synthesis. MERGE→180,185 | — |
| 154 | Copy-on-Write page-level snapshotting (near-zero cost) | **DEFER-WITH-TRIGGER** | COW page snapshot; rides durable-snapshot (P12) + arena; no consumer until durable-snapshot lands. MERGE→106,151 | W4-L8 |
| 155 | Adaptive epoch length (shrink as drift rises, lengthen when stable) | **DEFER** | meaningless until the epoch/snapshot cycle it modulates exists (after item 21/151). B2 §10 = v1 #25 | defer reg |
| 156 | Epoch-ID in the wire protocol to resolve stale-packet ambiguity | **ADOPT** | epoch field inside the signed envelope (W2-L4). MERGE→21,32 | W2-L4 |
| 157 | Consolidated 32-byte Bebop frame header (MAC×2, EtherType, TileID, EpochID, HypothesisID, Flags, Sequence) | **REJECT-carrier / ADOPT-fields-in-envelope** | REJECT the L2 carrier (item 93); ADOPT the fields inside the signed envelope, never a cleartext L2 header. d14v2 item 3 = v1 #20. MERGE→93 | W2-L4 |
| 158 | Control-flag bitmask (Panic/Survival/Snapshot-trigger/Epoch-reset) | **ADOPT** | a control-flag field inside the signed envelope, gossip-propagated, drives Survival/Locked transitions (items 123/149). MERGE→157 | W2-L4 |
| 159 | Epoch Coordinator → resolved to HLC (no central master, per-node monotone, max-merge) | **SPLIT: logical epoch ADOPT / HLC physical-clock REJECT** | the logical max-merge epoch ADOPT (Lamport, deterministic, W2-L4); the wall-clock half REJECT-ON-CORRECTNESS (injects non-reproducible wall-clock into ordering, `hybrid_gate.rs:104`). "HLC" read as "logical epoch" is correct. B2 §7 · T-3 = v1 #22 | W2-L4 |
| 160 | Gossip-propagated epoch (rides ordinary traffic, no dedicated broadcast) | **ALREADY-EQUIVALENT-shape + EXTEND** | the epoch counter rides the existing `GossipAgent`/`snapshot_root` path; the thin missing layer. B2 §8 = v1 #23 | W2-L4 |
| 161 | Bounded drift window (in-context / catch-up / isolate tiers) | **ADOPT** | the three `DriftClass` tiers Damped/Resonant/Unstable = in-context/catch-up/isolate; `classify_drift`. doc19 §1.4 | W2-L4 |
| 162 | 3-tier distributed brake (hardware hard-stop / peer-level breaker / pre-commit eval gate) — distributed authority, no central permission | **ALREADY-EQUIVALENT ×3** | fuel `OutOfFuel` = hardware hard-stop; `breaker.rs` = peer-level; drift-gate/import-replay = pre-commit eval gate (the "eval-gate soft-stop" 3rd tier closes B6 gap iv). B3 · v1 #24 | W2-L3 |
| 163 | Fail-fast, every node has its own "brake pedal" | **ALREADY-EQUIVALENT** | degrade-closed: every node's `debit`/drift-gate/fuel IS its own brake; no central permission — the zero-supervisor ideal, real for arithmetic invariants. doc19 §2.3 | — |
| 164 | Emergency-descent defined sequence (detect→panic-flag→peers auto-survival-mode) | **ALREADY-EQUIVALENT** | `BreachAlert` broadcast → `ingest_peer_breach` → peers converge; panic-flag = the control-flag (item 158). B3 §3. MERGE→158 | — |
| 165 | "Tensor Instinct" — piecewise safety fn `G(x,y,z,seed,health)` routing to a hard-coded safe tensor at zero cost | **ADOPT-AS-STRUCTURAL** | the piecewise safe-routing = drift-gate reject → last-good tensor ("Static Safe Tensor"); the branchless/masked selection (A3) makes it zero-cost; **this is an eqc-able closed-form safety equation** (author→generate→parity-gate). = item 145. MERGE→145 | W2-L3 |
| 166 | Embedded observability (safe-tensor activation self-documenting in the tensor log) | **ALREADY-EQUIVALENT** | `event_log` records the drift-reject/Locked transition; self-witness rows via `append_raw`; observability IS the WORM log. doc19 §1.1 | — |
| 167 | Hysteresis requirement (asymmetric trigger/release to prevent oscillation) | **ADOPT (live gap)** | `integrity_check` flips on instantaneous ρ<1.0 (`hydra.rs:186-193`) — add a two-threshold band + N-consecutive-healthy release. B3 §5 = v1 #E16 | W1-L3 |
| 168 | Hardware write-protected (read-only) memory for the static safe tensor | **NOTE-AS-SUBSTRATE-PHYSICS** | allowed if the owner-hub provides it (`mmap PROT_READ`/ROM); not a software mesh component; software form = the immutable golden baseline (item 146). MERGE→140,146 | — |
| 169 | Mathematical invariants as safety (unsafe value literally undefined, not policy-disallowed) | **ALREADY-DOCTRINE** | the Hermetic §4 earned pole; "proceed anyway is not representable" (budget/money/drift/fuel). doc19 §2.4 = v1 #27 | — |
| 170 | "Spinal cord over cortex" — fast low-level layer with structural veto over the higher "thinking" layer | **ALREADY-EQUIVALENT (this IS the Monocoque)** | the kernel drift-gate / red-line-scope has structural veto over the LLM/agent layer (`RedLinePolicy` refuses red-line scopes regardless of the agent's request; microvm admission). B3/B6 | — |
| 171 | Determinism-as-safety (no ambiguity for an adversary to exploit) | **ALREADY-DOCTRINE** | the bit-reproducibility contract; content-address/signature model (`rng.rs:22-28`). MERGE→12,136 | — |
| 172 | Updatable-but-cryptographically-gated hard invariants | **ADOPT** | invariants update only via external-signature (item 174); red-line scope changes need operator-signed roster action; never self-modify. MERGE→174 | W3-L7 |
| 173 | Trusted Computing Base (TCB) — physical/address-space separation of the mutable "learning" world from the immutable safety kernel | **ALREADY-EQUIVALENT** | microvm sandbox tiers + process-per-hub + the kernel/agent split (agent runs in a fuel-metered wasm sandbox; `register_adapter` refuses unsandboxed native). B3 §7 | — |
| 174 | External-signature-only invariant updates (system can't self-modify its own constraints) | **ADOPT** | anchor-signed roster/policy changes ("the human touches the roster, not the traffic"); eqc-generated safety equations change only via a signed regenerate-and-diff gate. B7 §6 | W3-L7 |
| 175 | Physics-over-bureaucracy energy-efficiency argument | **ALREADY-DOCTRINE** | the Hermetic §4 verdict verbatim (type-enforced poles = physics; remembered rituals = bureaucracy). B3 §0 = v1 #27 | — |
| 176 | FPV-detonator analogy — reliability as construction, not trained intent | **ALREADY-EQUIVALENT (framing)** | the substrate-physics fail-safe (wasmtime trap / hardware WDT / systemd StartLimit) — reliability by construction. = v1 #28 | — |
| 177 | Zero-trust treatment of the AI's own output as untrusted, pre-flight-sandboxed | **ALREADY-DOCTRINE** | microvm sandbox + fuel + `RedLinePolicy` scope-refusal + import-replay gate on compiled units. B3 §7 · B6 §1.3 | — |
| 178 | Hermetic Principle of Polarity (Faithfulness theological vs Physics/Systems mechanical) | **ALREADY-EQUIVALENT** | Polarity P4; the repo's own framing. = v1 #29 | — |
| 179 | "Integrity" (measurable/physical) proposed to replace "Faithfulness" (vague/unfalsifiable) | **ALREADY-DOCTRINE** | `integrity_check` measures ρ; the earned-vs-aspires line; "Integrity" IS the measurable pole. B6 §4 | — |
| 180 | Zero-point reframed as controlled oscillation (drone-hover), not a static point | **ALREADY-EQUIVALENT** | `DriftClass::Resonant` ρ≈1 is EXPLICITLY a permitted limit-cycle class (only Unstable rejected) — the operator's zero-point reading verbatim. doc19 §1.4 | — |
| 181 | Epoch-clock reframed as the literal correction rhythm implementing that oscillation | **ADOPT** | the epoch/snapshot cycle IS the correction rhythm; the maintenance-pass boundary = the epoch; **bridge-gap #2** (drift-gate the snapshot). doc19 §1.2/§1.5 | W1-L11, W2-L4 |
| 182 | Operator's question: true equilibrium with NO watchdog/snapshot-manager/sync-coordinator as separate components | **ANSWERED (§D)** | YES for internal-arithmetic invariants (genuine zero-authority, bad state unrepresentable, no separate component); NO fully for the tamper/restart leg (a **finite anchored** authority key_V survives — not a watchdog, not self-cert). doc19 Part2 · B6 §4 | — |
| 183 | Stateless/generative-not-stored + implicit emergent sync via gossip wave-propagation + fail-hard ("pure Fatalist") | **SPLIT** | generative-not-stored ADOPT-narrow (integer/spectral generator, item 130); emergent-sync-via-gossip ALREADY-EQUIVALENT (anti-entropy convergence); **fail-hard-only REJECT** (superseded by item 184). MERGE→130 | — |
| 184 | Operator's course-correction: pure Fatalism too rigid for real transient noise | **ADOPT-AS-RULING (binding)** | the Resonant band + snapshot-re-entry absorbs transient noise rather than fail-hard; the three-way split (item 185) is the resolution. Operator's own synthesis | — |
| 185 | Final three-way synthesis: Self-Healing = redundant/error-correcting math (not a supervisor); Self-Termination = a hard invariant boundary (absence of a valid next value IS the stop); Snapshot Re-entry = cheap regenerative recovery from the last valid epoch/seed | **ADOPT-AS-THE-ARCHITECTURE (the LATE-priority authoritative frame)** | Self-Termination ALREADY-BUILT (degrade-closed: budget/drift/fuel — "absence of a valid next value IS the stop" = the Result-type/drift-reject); Self-Healing PARTIAL (dynamical+replay real; M7 topological + redundancy-ECC gaps → ADOPT M7); Snapshot-Re-entry PARTIAL (in-memory real; durable half → P12). This is the spine of §A. B3 §6 · doc19 = v1 #31/#32 | W4-L7, W4-L8 |

### B.18 TALLY (every item accounted for)

**185/185 verdicted, zero silent drops.** Distribution (an item counts once by its dominant verdict;
SPLITs counted by their governing decision): **ADOPT / ADOPT-AS-COMPOSITION / ADOPT-AS-REFRAME /
ADOPT-AS-RESEARCH-PASS ≈ 52** · **EXTEND-EXISTING ≈ 12** · **ALREADY-EQUIVALENT / -DOCTRINE ≈ 58** ·
**DEFER-WITH-TRIGGER ≈ 30** · **REJECT-ON-PHYSICS/CORRECTNESS/DOCTRINE ≈ 33**. **MERGE-notes: ≈ 45
items are explicitly de-duplicated to a numbered sibling** (a de-dup, not a drop — each still carries
its own verdict row).

**Genuinely NEW or FLIPPED verdicts vs the v1 ledger's 64 rows (≈ 48 items):** the six re-examined
flips (items 2/5/6 steelman-commitment ADOPT [d20#2]; 23/45 Merkle-DAG-as-single-authority [d20#6];
41/42/46/60 checkpoint-STARK-realistic [d20#3+§C-C]; 48 Laplace-Beltrami grounding [d20#5]; 130/133
integer-CORDIC codec ADOPT [d20#4]) · the doc-14v2 network corrections (35 hardware-attestation FLIP;
98 eBPF-available-on-hub; 92/93 partial flips; 68 cache-domain resolution) · the **three Step-2
findings** (§C-A crypto-SIMD-in-core → items 5/7/25/103/119; §C-B cache-domain → item 68; §C-C
MoE-mirror → item 128, STARK-on-core → 41/42/46/60) · the doc-19 bridges (52/57/91/136 normalize-
before-hash; 151/181 drift-gated snapshot; 30/134 async-observer-is-a-watchdog; 182 authority
honesty) · and the newly-itemized granular clusters the v1 64-row ledger never enumerated (the
trust-matrix cluster 33–40, the predictive-handoff cluster 104–123, the epoch/zero-point cluster
151–185). **The remaining ≈ 137 items confirm/re-ground a prior batch verdict at this list's finer
granularity.**

---

## §F. REVISED PRIORITIZED WAVE BUILD ORDER (kernel-first, smallest-abstraction-first)

Ordering principle unchanged from v1 (operator: "від малого до великого, найменші абстракції на рівні
ядра … перші"; waves = collision-free lanes for concurrent swarm dispatch). **What changed from v1:**
Wave 1 gains the two doc-19 bridge bugs (W1-L10/L11); Wave 2 gains the AVX2 crypto-verify lane
(§C-A) and the cache-domain decision is folded into P25 (no new lane); the STARK-on-core defer moves
up (§C-C); the three flips (integer-CORDIC W2-L2, steelman-commitment at W3-L2, Merkle-DAG-single-
authority W3-L6) are reflected. Done-checks are the batches' own falsifiers.

### WAVE 1 — correctness closure + first equations (buildable today, zero operator gates)

| Lane | Item | Files | Done-check |
|---|---|---|---|
| W1-L1 | eqc-rs → `ema_next` generated + parity; add `asin`/`atan2` nodes + checked-overflow emission; **CORDIC primitive** (§C-C/d20#4) | `tools/eqc-rs`, `kernel/src/geo.rs` | parity `#[test]` bit-identical; **cross-arch digest `0x9d1c0e89c65cbe08` on x86_64+aarch64** (d20#4) |
| W1-L2 | **Port `append_raw` exactly-once fix** + regression test | `kernel/src/event_log.rs` | `commit_after_decide_replay_on_nonempty_log_is_true_duplicate` green; `decide` once; `log.len()` unchanged |
| W1-L3 | Hysteresis band on `Hydra::integrity_check` (item 167) | `kernel/src/hydra.rs` | RED: ρ dithering around 1.0 must not flap `Live↔Locked` |
| W1-L4 | `order_machine::spectral_radius` → proven `const` ρ=0 | `kernel/src/order_machine.rs` | golden-signature test green; 1000-iter loop gone |
| W1-L5 | Consolidate duplicated 2×2 eigenvalue closed form | `kernel/src/householder.rs` | 8 hand-oracle tests green, byte-identical |
| W1-L6 | Fix `engine/src/zerocopy.rs:22` AoS≠SoA label (item 67) | `engine/src/zerocopy.rs` | comment corrected; grep-able convention |
| W1-L7 | Numeric clamps at the wasm boundary (item 18/safety) | `engine/src/field_frame.rs`, `wasm/src/lib.rs` | RED: oversized/zero → typed error, never panic/OOM |
| W1-L8 | Executable negative test "money/order never merges" (item 9) | bebop2 `core/src/anti_entropy.rs` tests | merge-green impossible to write |
| W1-L9 | Hygiene: delete stale Python eqc copies | tools trees | no Python in the eqc lineage |
| **W1-L10** | **NEW — normalize-before-hash bridge #1 (items 52/57/91/136):** content-address the canonical (`row_normalize`d / integer-scaled) tile in `slem_cached` | `kernel/src/spectral_cache.rs` | RED: two nodes building the same logical tile at different scale converge to the same content-id (today they do not); integer/fixed-order on the hash path only |
| **W1-L11** | **NEW — drift-gate the arena snapshot #2 (items 151/181):** route snapshot-then-drop retain through `classify_drift`/`step_preserves` | arena pass + `kernel/src/spectral.rs` | RED: an `Unstable` rebuild is refused retention |

### WAVE 2 — smallest new kernel abstractions

| Lane | Item | Files | Done-check |
|---|---|---|---|
| W2-L1 | `arena.rs` `BumpArena` per P28 §3.3 + `HugePageHint` NoOp port (items 63/70/56/61) | new `kernel/src/arena.rs` | criterion heap-vs-arena; ≤8 heap allocs; byte-identical PPR; Miri-clean |
| W2-L2 | Eigenvector R1-R3 (`eigh_contig`+`topk_symmetric`, item 48/81) + **integer-CORDIC substrate** (item 130/133) | `householder.rs`, `spectral.rs` | plan §5.4 KAT + sparse-vs-dense parity + byte-determinism |
| W2-L3 | `breaker.rs` per P27 §3.2 (item 144/162) + **"Tensor Instinct" piecewise safe-route as an eqc equation** (item 165) | new `kernel/src/breaker.rs` | P27 table-tests on the pure `step()` core |
| W2-L4 | Logical epoch (Lamport max-merge, NO wall-clock) + fold TileID/EpochID/HypothesisID/Seq/control-flags into the **signed envelope** (items 21/32/93/120/156-161) | bebop2 `discovery.rs`, envelope schema | 3-node epoch→max; **assert no `SystemTime`**; envelope round-trip |
| **W2-L5** | **NEW — AVX2 crypto-verify lane (§C-A, items 5/7/25):** SIMD path behind `is_x86_feature_detected!("avx2")`, scalar fallback; lever ML-DSA-65 NTT+Keccak×4, then curve25519 avx2 for Ed25519 | bebop2 `core/src/pq_dsa.rs`, `core/src/sign.rs` | **bit-identical verdicts vs scalar**; criterion p99 per-recv drop vs ~71 µs baseline |
| W2-L6 | Evidence benches gating DEFERs (Merkle add/root + digest n∈{10²,10³,10⁴}; pq_dsa verify p99) | bench files | committed baselines; regression-gated |
| W2-L7 | Nyquist bound + z-pole↔Resonant note; branchless/sentinel + AVX2 batch of CSR/GEMM skips (items 69/103/119/124) | `field_frame.rs`, `csr.rs`, `mat.rs` | above-Nyquist source flagged; byte-identical vs branchy path |

### WAVE 3 — mesh composition (operator docket R-1…R-4)

**R-1** `0x12→0x13` discriminant · **R-2** budget-unit semantics · **R-3** `RootDelegationPolicy` ·
**R-4** money-law eqc flip + integer basis-points.

| Lane | Item | Gate | Done-check |
|---|---|---|---|
| W3-L1 | 3-way relation-slice tensor (items 47/54) | arena-aware | identical slice-sets ⇒ identical content-address |
| W3-L2 | `WorkReceipt` (semantic-contract PoQ; items 2/5/6/7/26) via `HybridGate::check` | **R-1** | verifies only through the counterparty's gate |
| W3-L3 | Priority = nested `TokenBucket` envelopes (items 15-18/37) | R-2 informs budget-leg | a peer can't draw from an envelope its capability doesn't grant |
| W3-L4 | Restart-intensity as a **launch-path predicate** (item 139; T-6, never a monitor) | — | RED: crash-looping drainer stops relaunching; no polling process |
| W3-L5 | Fuel invoke-time wiring + `FUEL_PER_UNIT` pin | — | compute-bomb `OutOfFuel`-terminated on the real path |
| W3-L6 | DecisionUnit distributed extensions (items 19-23/28/115/127/128-134): epoch header · **import gate = once-per-artifact independent replay** · rollback lineage in the EXISTING sha3 registry (single authority) · **MoE-domain oracle routing by capability tag** (§C-C) | — | replay-gate RED (poisoned unit refused); stale unit answers `Escalate`; red-line shapes operator-gated |
| W3-L7 | Anchor issuance-budget predicate + **hardware-attestation-as-augmentation** hook (items 35/172/174; doc14v2#5, degrade-closed, optional) | **R-3** | RED: N+1-th delegation in a budget window refused at sign time; attestation optional, never a hard brick |
| W3-L8 | Product T1 read-only wiring | — | parity vs TS output; no RLS surface touched |
| W3-L9 | **Predictive-tensor-handoff research pass** (items 104-123, 44/77/86/87/90/107/114) — spatial-STATE hypotheses (Kalman + splatting), NOT agent-scoring | — | a batch-grade findings doc; every sub-concept verdicted |

### WAVE 4 — authority flips + heavier legs

| Lane | Item | Gate | Done-check |
|---|---|---|---|
| W4-L1 | Money law via eqc-rs Q-format | **R-4** | parity bit-identical across the fixture corpus |
| W4-L2 | Money dual-authority collapse (server CHARGE → kernel) | money red-line | display & charge produce identical integers |
| W4-L3 | State-machine authority (order FSM → kernel) | after W3-L8 | FSM parity suite; adapters unchanged |
| W4-L4 | Shared FSM adjacency-table primitive | — | both byte-identical to their `match` predecessors |
| W4-L5 | Frontend Path-1 (view-model fns → WASM) | — | JSX untouched; view-model parity |
| W4-L6 | Spectral convergence advisory (`mesh_consensus.rs` λ₂/SLEM; items 3/48) | — | advisory only, degrade-closed |
| W4-L7 | **M7 topological self-heal** (item 185 Self-Healing leg) | mesh seam (P9/P10) | reconnection under partition proven |
| W4-L8 | **Durable snapshot + restore-drill** (items 151/154/185; COW page-snapshot rides here) | owned by P12 | a restore-drill has RUN (Hermetic #4 closed) |
| W4-L9 | Product T4 write paths | **HARD GATE: NOBYPASSRLS workstream** | RLS-adversarial suite green with kernel deciding |
| W4-L10 | Frontend Path-2 (no-DOM physics-UI islands; item 133 + E12) | — | island-by-island; money-never-tween preserved |

### DEFER REGISTER (numeric triggers; `core_pinning.rs:41-64` shape)

Morton/tiling/prefetch/align (62/65/66/71-77) — grid/nnz > L2 + blocked traversal · Roaring bitmaps
(64) — large sparse-set intersection on a hot path · AF_XDP/io_uring/eBPF/RSS (89/92/98/101) — bare-
metal multi-node + transport-dominant profile (io_uring only for local file-I/O) · **checkpoint/light-
client STARK (41/42/46/60)** — a periodic FSM-replay-audit need + a second live node; **realistic on
the hub's spare core-budget, no GPU, PQ-STARK only (§C-C)** · token-stream pooling/LOD (78/79/155) —
after P28 rung 1, anti-alias per Nyquist · adaptive epoch (155) — after the epoch exists · **CCX/L3-
slice + NUMA (68)** — multi-CCD/multi-socket procurement (§C-B) · stake/bond issuance (C6) · first-
party bilateral memory (C8).

---

## §G. NUMA / CACHE-DOMAIN — RESOLVED (not kicked back)

**Measured (§C-B):** single socket, 1 NUMA node, **one flat 32 MiB L3 (`id=0`, all cores)**, per-core
512 KiB L2 (`shared_cpu_list=0-1`). Resolution as a decision:

1. **The real single-socket cache-domain technique = L2-per-core affinity, and it is ALREADY ADOPTED
   (P25 CORE-BOUND, `taskset -c 0,2,4,6`).** One work-stream per physical core keeps each L2 private —
   the genuine "cache domain as a placement zone" without multi-socket hardware, and the *only* sub-
   socket domain this host or the realistic owner-hub (Intel N100/i5 monolithic L3, single-CCD Ryzen)
   exposes. **Nothing new to build.**
2. **An L3-slice / CCX / CCD pseudo-NUMA zone does NOT exist here or on the realistic hub.** Real only
   on multi-CCD/multi-die parts — same hardware class as dual-socket NUMA. **DEFER-WITH-TRIGGER,
   procurement-gated**, behind a NoOp `core_pinning.rs`-shaped port that degrades to no-op on one L3.
3. **NUMA pinning proper = REJECT-as-requirement / no-op** on the realistic single-socket hub, with the
   procurement question preserved (single-socket = no-op; dual-socket = real, an explicit power/noise/
   cost call). **Answered, not open.**

---

## §H. REGISTRATION

Registered in `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §8.12 as **Phase 30**;
this v2 supersedes the v1 synthesis there (same off-critical-path lane structure; Wave 1 startable
immediately, now including W1-L10/L11; P06 relationship unchanged — key_V remains the tamper-leg
closure, §D; RLS as an external parallel workstream). Operator docket: R-1…R-4 (§F Wave 3) + the C6
stake / C8 bilateral-memory flags. Design authorities this v2 composes (does not re-derive): the v1
synthesis, doc 19 (system coherence + authority boundary), doc 20 (six re-examined flips), doc 21
(local-LLM measurement), doc 14 v2 (corrected network target). Navigation: `INDEX.md`.

---

## §I. ANU / ANANKE CHECK

**Anu (derivable):** every §C claim is a probe run this session — cache topology (`lscpu -e`/`numactl`/
`/sys/.../cache`), crypto floor (`openssl speed ed25519` → 71 µs), crypto backend (`sign.rs` from-
scratch scalar; dalek/fiat only in legacy `crates/bebop`; CPU flags have `avx2`, no `avx512`/`vaes`),
toolchain (rustc 1.96.1 → the AVX2 lane is buildable) — plus web-grounded SIMD ratios (curve25519 avx2
~1.5-2×; Dilithium AVX2 NTT 2.4-2.5×) and 2026 STARK numbers (Jolt ~10⁵×/100 kHz; small-space <2×;
Binius 5-10×). Weakest links, named: the SIMD crypto speedups are *researched, not yet built* (the lane
is W2-L5, to be measured); the MoE-mirror throughput figure is arithmetic on summed bandwidth, not a
cluster measurement; the predictive-handoff cluster (104-123) is a *research pass* (W3-L9), verdicted
as research-shaped, not built code.

**Ananke (structural):** the architecture is forced by the §A dependency graph, not a feature list —
every heavy item is a role in `(e)→(c)→(a)→(b) gated by (d)`, propagated by DecisionUnit gossip. The
two bridges (W1-L10/L11) are the composition the graph *requires*, promoted to Wave 1 so the gap is a
compile/RED-test hole, not a reader's diligence. The authority boundary (§D) is stated at its honest
limit — zero-authority earned on arithmetic invariants, a finite anchored authority (key_V) on the
tamper leg — not over-claimed. And the crypto-in-core redirect (§C-A) is answered by attacking the
*measured* bottleneck with a *safe* SIMD technique that keeps every signature independently verified,
not by re-litigating kernel-bypass against a ratio that does not move.
