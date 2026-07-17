# BLUEPRINT — Bebop2 Mesh Masterwork: Synthesis + Prioritized Wave Build Order (2026-07-17)

> **The reasoning/planning pass** over the nine research batches in this directory
> (10-BATCH1 … 18-BATCH9), produced per the operator's routing rule ("Opus для досліджень та
> аудиту, fable для reasoning/planning"). Planning artifact only — no product code. Every claim
> carries a `file:line` cite inherited from the batch that grounded it, or is marked (proposal).
> Line numbers are used ONLY where a batch already cited them; nothing is invented here.
>
> **Binding constitution** (memory `bebop2-mesh-masterwork-2026-07-17.md`), applied literally:
> - **Framing rule** — every dialogue concept gets a genuine verdict; REJECT only on
>   independently-verifiable physics/correctness (absent hardware, determinism/impossibility
>   theorem, CI-enforced red-line). "Unconventional" is never a rejection reason. Omission =
>   sabotage; the ledger in §1 is complete against Batch 6 §5.1's coverage sweep.
> - **Priority-ordering rule** — later dialogue material (Monocoque hard-invariants,
>   equilibrium/zero-supervisor, equations-not-primitives) outranks earlier material
>   (market-consensus, raw tensor-memory tuning) wherever the two force a choice.
> - **Execution-model rule** — core logic as EQUATIONS via `tools/eqc-rs` (landed `7c7763af7`);
>   Rust and every other language is an adapter/bridge; scripts → zero; invasive refactor is
>   licensed, additive-only caution is not a default.
> - **Scope escalation** — the ENTIRE codebase across all repos (dowiz incl. product layer,
>   bebop-repo/bebop2, openbebop, dowiz-agentic-mesh, dowiz-spectral-evolution); UI is a
>   real-time render of kernel state (same target as the physics-UI / field-UI arcs).
> - **Operator verdicts, binding, not re-litigated:** Sybil-proof via signed-capability issuance
>   (PROVEN-VIABLE, Batch 7), never reputation/courier-scoring; zero watchdog processes, zero
>   proxy intermediaries — self-healing/self-termination are structural properties of the normal
>   path.
>
> **Design authorities this synthesis defers to (does not re-derive):**
> `BLUEPRINT-LATENCY-ELIMINATION-RESEARCH-AND-BRAINSTORM-2026-07-17.md` §2 (Decision Compiler ≡
> DecisionUnit gossip, confirmed Batch 6 §1.1); `BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md`
> (eigh_contig + spectral::topk_symmetric, lowrank.rs superseded);
> `BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md` (P28 arena + tensor ladder);
> `BLUEPRINT-FAULT-ISOLATION-DECENTRALIZED-ARCHITECTURE-2026-07-17.md` (P27 breaker design);
> `docs/adr/ADR-realtime-change-intelligence.md` (dual-authority prohibition);
> `BLUEPRINT-P06-v1-split-identity-verifier.md` (key_V independent verification).

---

## §0. Shape of the result, in four sentences

The nine batches prove the dialogue is ~60% already built or already doctrine in this codebase,
~20% genuinely new and adoptable, ~10% deferrable behind named numeric triggers, and ~10%
rejectable on hard physics/correctness — never on complexity. The single highest-leverage
correctness item is a LIVE exactly-once bug on this branch (`kernel/src/event_log.rs:359`,
Batch 2 §12) whose fix already exists in a sibling worktree. The smallest, most literal
application of the operator's equations-not-primitives rule is now unblocked (eqc-rs landed
`7c7763af7`) and is Wave 1. Everything heavier composes upward from two small new kernel files
(`arena.rs`, `breaker.rs`) plus the already-decided eigenvector refactor, and the whole
consensus/trust layer resolves to capability mechanics the code already implements
(`verify_chain`, Batch 7).

---

## §1. EXECUTIVE VERDICT LEDGER — every concept, every batch, nothing omitted

Verdict vocabulary: **ADOPT** · **EXTEND-EXISTING** · **ALREADY-EQUIVALENT** (incl.
already-doctrine/already-built) · **DEFER-WITH-FALSIFIABLE-TRIGGER** · **REJECT-ON-PHYSICS**
(reason inline; full restatement in §2). Build slot refers to §5.

### 1.1 Part-A dialogue concepts (spine = Batch 6 §5.1 coverage ledger)

| # | Concept | Verdict | Ground (batch · cite) | Build slot |
|---|---|---|---|---|
| 1 | Market-based micro-negotiation / bid-priority auctions | REJECT-ON-PHYSICS (transient-acquirable-weight class; measured MEV/auction failure record; prior-art-adjudicated) — narrow sealed-batch commit-reveal form stays permitted-if-ever-needed | B4 §2.1 | — |
| 2 | Self-auditing inline witnessing / JIT recursive proofing | REJECT-ON-PHYSICS (self-certification = RC-2: the check restates the claim); counterparty-verified `WorkReceipt` is the accepted form | B4 §2.2 | W3-L2 |
| 3 | Emergent swarm / flocking / "pulsing organism" | ALREADY-EQUIVALENT (anti-entropy gossip `discovery.rs:148-255` + spectral convergence `mesh_consensus.rs`); flocking-as-boids-primitive dissolves into these — verdict now explicit, closing Batch 6 gap iii | B5 §1.1 · B4 §2.4 | W2-L4, W4-L6 |
| 4 | Speculative / optimistic execution + rollback | REJECT-ON-PHYSICS (degrade-open in a degrade-closed architecture; no permissionless fraud proof worked in ~3 yr of mainnets; verify-cost ≪ network floor) | B4 §2.5 | — |
| 5 | Eventual-consistency vs fast-finality tiering | ALREADY-EQUIVALENT (finality is local-and-explicit; commutative/non-commutative split already encoded, `sync_pull.rs:522-621` vs `event_log.rs:66-172`) | B2 §0 · B4 §2.4 | — |
| 6 | Priority-tagged transitions / priority dispatcher | ADOPT as composition — nested `TokenBucket` envelopes keyed `(PeerId, CapabilityClass)`; priority flag = envelope selector checked against capability scope, never self-assigned | B4 §2.6 | W3-L3 |
| 7 | DecisionUnit gossip / JIT-compile swarm intelligence | ALREADY-EQUIVALENT in design (= the Decision Compiler, latency blueprint §2 is authority; four in-repo precedents verified) + ADOPT the distributed extension | B6 §1.1 | W3-L6 |
| 8 | Epoch-versioning for DecisionUnits | ADOPT (new ground) — monotone logical epoch in the unit's provenance header; SAME counter as the gossip epoch; max-merge; no wall-clock | B6 §1.2(a) | W3-L6 |
| 9 | Proof-of-Quality (4 forms + hybrid) for gossiped units | SPLIT: statistical-vote REJECT (reputation class, §2.9); optimistic-fraud-proof REJECT (§2.8); **import-time independent replay ADOPT** (once-per-artifact verify-before-persist — the per-artifact/per-transaction distinction is decisive) | B6 §1.3 · B4 §2.2 | W3-L6 |
| 10 | State journaling + rollback | ALREADY-EQUIVALENT (`snapshot_payloads`/`rebuild_from_payloads`, `core/src/event_log.rs:115-134`); rolling truncation DEFER + council-gated (§1.1 #25) | B2 §10 | — |
| 11 | Merkle-bisection dispute resolution | DEFER-WITH-TRIGGER (linear `diff` wins on round-trips until digest cost/bytes dominate a measured round); reputation-WEIGHTED half REJECT (red-line) | B2 §6 | W2-L5 gates |
| 12 | Determinism requirements (fixed-point, no SystemTime, no thread_rng) | ALREADY-EQUIVALENT as contract (`csr.rs:31-37`, `rng.rs:22-28`, `hybrid_gate.rs:104`); fixed-point exists only in eqc-rs Q-format emission — carried into W1/W4 money work | B6 §2.2 · B2 §7 | W1-L1, W4-L1 |
| 13 | ZK-proof anchoring (EZKL/risc0/sp1) | REJECT-ON-PHYSICS per-message (~10⁶× prover overhead); DEFER-WITH-TRIGGER for periodic checkpoint + light-client join, STARK-only (PQ-consistent) | B4 §2.7 | defer register |
| 14 | Sparse tensor graphs (COO/CSR canonical order + hashing) | EXTEND-EXISTING (`csr.rs:79-115` + `spectral_cache.rs:98` already are it; edge-tuple contract IS the COO layer — no new struct); 3-way (n×n×m) relation tensor = P28 rung 2 | B1 A1 | W3-L1 |
| 15 | Branchless (cmov / masking / sentinel padding) | ALREADY-EQUIVALENT (sentinel padding proven `simd.rs:79-90,118-124`) + EXTEND-EXISTING for the CSR/GEMM sparsity skips (`csr.rs:175`, `mat.rs:102`), byte-identity-gated | B1 A3 | W2-L7 |
| 16a | Cache-line alignment `#[repr(align(64))]` | DEFER-WITH-TRIGGER (no false-sharing surface — hot passes single-threaded; loads already unaligned-fast) | B1 A5 | defer register |
| 16b | Software prefetch | DEFER-WITH-TRIGGER (`nnz*16 B > L2` AND spmv measured memory-bound; `csr.rs:180-183` is the site) | B1 A6 | defer register |
| 16c | HugePages / THP via madvise | DEFER-WITH-TRIGGER via named `HugePageHint` NoOp port copying `core_pinning.rs:41-64`; trigger = persistent arena region > 2 MB | B1 A7 | W2-L1 (seam) |
| 16d | Arena allocator | **ADOPT** — build P28 §3.3's `kernel/src/arena.rs` `BumpArena` exactly as specified (verified absent this session); extend with the HugePageHint seam | B1 A8 | W2-L1 |
| 16e | Tiling (block layout to page size) | DEFER-WITH-TRIGGER (no dense matmul n≥128 on any hot path; `mat.rs:93-112` consumers are n≤32) | B1 A9 | defer register |
| 17 | 3D-spatial memory mapping (tile = HugePage) | DEFER-WITH-TRIGGER (composes A7+A9; same triggers) | B1 A7/A9 | defer register |
| 18 | Token "pixel"/mipmap LOD compression | EXTEND-EXISTING (~85% exists: retrieval tiers, spectral coarsening = P28 rung 1, `chunker.rs` CDC, skeleton-LOD tooling); literal pixel-mipmap REJECT (§2.6); residual token-stream pooling primitive DEFER until after P28 rung 1 lands | B6 §3 · B1 A10 | defer register |
| 19 | Distributed shared memory / RDMA / AF_XDP / DPDK | REJECT-ON-PHYSICS (no RNIC on either substrate; DPDK removes the single shared NIC; Firecracker prod unprivileged/virtio-only); AF_XDP-on-dev DEFER-WITH-TRIGGER (bare-metal multi-node + measured UDP-copy-dominant profile) | B5 §1.3-1.5 | — |
| 20 | Custom 32-byte L2 Ethernet framing | REJECT-ON-PHYSICS the carrier (no shared L2 segment on either substrate; cleartext header outside the signature envelope is a security regression); ADOPT the FIELDS (TileID/EpochID/HypothesisID/Seq) inside the signed envelope | B5 §1.2 | W2-L4 |
| 21 | Predictive tensor handoff for moving physical assets (drone-tiles, ghost-prefetch, atomic swap, shadow-tile multi-hypothesis + probability-weighted pruning) | ADOPT-AS-RESEARCH-PASS — the one significant concept no batch audited as a unit; hypothesis weights are over STATE (particle filter / Kalman substrate `geo.rs::ema_next`), not agents — NOT a scoring violation; adjacency: splatting + Kalman arcs | B6 §5.2-i | W3-L9 |
| 21b | Atomic pointer swap / lock-free tile handoff (RCU/hazard-pointer) | DEFER-WITH-TRIGGER — folded into the W3-L9 research pass output; no `AtomicPtr` consumer exists today | B6 §5.2-ii | W3-L9 |
| 22 | Hybrid Logical Clocks | REJECT-ON-PHYSICS the physical-clock half (injects non-reproducible node-local wall-clock into the path both dialogue and code require deterministic — self-contradiction); ALREADY-EQUIVALENT the logical half (per-actor `seq` + `max_seq`, `sync_pull.rs:487-591`); advisory signed non-ordering time field DEFER-WITH-TRIGGER | B2 §7 | — |
| 23 | Gossip-based epoch propagation | EXTEND-EXISTING — logical (Lamport max-merge) epoch counter riding the existing `GossipAgent`/`snapshot_root` path; the thin missing layer over a built, 3-node-proven transport | B2 §8 · B5 §1.1 | W2-L4 |
| 24 | Circuit-breaker / distributed watchdog / Mesh Panic Handler | SPLIT: panic-handler ALREADY-EQUIVALENT (`Locked` + `BreachAlert`, `hydra.rs:75-79,287-348`); compute hard-stop ALREADY-EQUIVALENT (wasmtime fuel, `fuel.rs:91-208`); breaker ADOPT (P27 §3.2 design → build `breaker.rs`); restart-intensity ADOPT in structural form only (§7 adjudication T-6 — launch-path predicate, never a standing monitor); per-peer breaker DEFER (needs P9/P10 seam); "eval-gate soft-stop" third tier ALREADY-EQUIVALENT — it maps to the drift-gate/Survival-Mode soft-refuse (`event_log.rs:389-419`), closing Batch 6 gap iv | B3 §1-§3 · B6 §5.2-iv | W2-L3, W3-L4 |
| 25 | Rolling snapshot / checkpoint-restore + adaptive epoch | PARTIAL-EXISTING (checkpoint/restore built); truncation DEFER + COUNCIL-GATED (append-only is a money red-line: `anti_entropy.rs:13`); adaptive epoch DEFER (sequenced after #23) | B2 §10 | defer register |
| 26 | "Monocoque" safety-as-structure | ALREADY-EQUIVALENT — it IS the repo's own Hermetic §4 verdict (`HERMETIC-ARCHITECTURE-PRINCIPLES.md:315-317`, earns-vs-aspires) | B3 §0 · B6 §4 | — |
| 27 | Hard invariants > RLHF "ethical bureaucracy" (AGI safety) | ALREADY-EQUIVALENT (same §4 verdict; type/arithmetic-enforced poles are the earned half) | B3 §0/§7 | — |
| 28 | FPV-hardware grounding (physical vs ethical failure) | ALREADY-EQUIVALENT (framing for #27; no build surface) | B3 §0 | — |
| 29 | Hermetic Polarity framing (Faithfulness vs Physics camps) | ALREADY-EQUIVALENT (Polarity P4) | B3 §0 | — |
| 30 | Descartes-square decision method | ALREADY-EQUIVALENT (meta-method; used in §3/§7 of this doc and in the operator docket rows) | B6 §5.1 #30 | — |
| 31 | Self-healing w/o watchdog as emergent flow topology | SPLIT (the deep Batch 6 result): REAL for internal-arithmetic invariants (budget/money/drift/fuel — bad state unrepresentable, no watcher); IRREDUCIBLE-CHECK for tamper + restart legs — external threats cannot be typed out of existence; the load-bearing fix for the tamper leg is P06 key_V (independent second party), not a cleverer type | B6 §4 | §6 |
| 32 | Self-Heal / Self-Terminate / Snapshot-Re-entry 3-way split | Self-Termination ALREADY-EQUIVALENT (= degrade-closed, fully built: `budget.rs:110-118`, `bounded_drainer.rs:70-82`, `fuel.rs`, drift-gate); Self-Healing PARTIAL (dynamical+replay real; M7 topological heal unbuilt) → ADOPT M7; Snapshot-Re-entry PARTIAL (in-memory real; durable half = P12) → ADOPT durable drill under P12 | B3 §6 | W4-L7, W4-L8 |

### 1.2 Part-B signal-processing addendum

| # | Concept | Verdict | Ground | Build slot |
|---|---|---|---|---|
| B1 | Laplace transform of Dirac δ (`L{δ}=1`) | REJECT-ON-PHYSICS (domain-mismatch: discrete deterministic kernel has no continuous s-plane surface; the impulse-seed idea is already live in PPR one-hot seeds `csr.rs:604-621`) | B1 B11 | — |
| B2 | Dim-reduction table | SPLIT: PCA ALREADY-EQUIVALENT (= `spectral::topk_symmetric`, the decided eigenvector primitive / P28 rung 1); Isomap DEFER-WITH-TRIGGER (needs k-NN + geodesic infra); t-SNE / UMAP / iterative-LLE REJECT-ON-PHYSICS (stochastic init/SGD violates the bit-reproducibility contract `csr.rs:36-37`) | B1 B12 | W2-L2 |
| B3 | Kuen surface (constant negative curvature) | REJECT-ON-PHYSICS (no hyperbolic-geometry consumer or implementation surface; flat grid + combinatorial graphs) | B1 B16 | — |
| B4 | Z-transform integration property | EXTEND-EXISTING as analytic lens — the `z=1` pole names the `DriftClass::Resonant` boundary (`spectral.rs:315-352`); document, no new primitive | B1 B13 | W2-L6 |
| B5 | Nyquist–Shannon sampling | EXTEND-EXISTING — a REAL missing bound: the 50 Hz clock (`lib.rs:219`, `field_frame.rs:40-53`) implies a 25 Hz source-bandwidth ceiling stated nowhere; add beside the CFL bound; carries to the mesh epoch clock and to any token-stream pooling tier | B1 B14 · B6 §3.3 | W2-L6 |
| B6 | Laplace integration property | REJECT-ON-PHYSICS (same domain-mismatch as B1); its discrete counterpart IS B4 — redirected, not lost | B1 B15 | — |
| B7 | Part-B primitives as an integrated token-stream FILTER pipeline (operator's stated purpose) | DEFER-WITH-TRIGGER — the honest home is the LOD/pooling tier atop P28 rung 1 (anti-alias before downsample per B5); named so Batch 6 gap v is owned, not dropped | B6 §5.2-v | defer register |

### 1.3 Consensus/trust cluster — resolved under the operator's binding rulings

| # | Concept | Verdict | Ground | Build slot |
|---|---|---|---|---|
| C1 | Reputation-weighted trust matrices (+ network-diversity Sybil factor) | REJECT-ON-PHYSICS — Cheng–Friedman (2005) theorem: NO symmetric reputation function (diversity-weighted included) is Sybil-proof; CI-enforced `NO-COURIER-SCORING` red-line (`claim_machine.rs:13-17`, `ci-no-courier-scoring.sh`); operator ruling confirms capability-only | B4 §1 · B7 §2 | — |
| C2 | BFT via reputation-weighted majority | REJECT-ON-PHYSICS (weighting half = C1; BFT-majority half dissolves — no global head exists; finality is local; the real convergence math is spectral λ₂/SLEM on the capability graph, `mesh_consensus.rs`) | B4 §2.4 | W4-L6 |
| C3 | Sybil-resistance WITHOUT reputation | **ADOPT — PROVEN-VIABLE**: asymmetric anchor-rooted issuance (`verify_chain`, `roster.rs:252-316`) is an instance of Cheng–Friedman's own theorem-permitted asymmetric flow class; identity free, authorization anchor-gated; N Sybils ⇒ `UnknownIssuer` ⇒ zero authority. Residual = bound the anchor's granting decision (`RootDelegationPolicy`, `node_id.rs:156-184`, fail-closed today) → operator ruling R-3 + a per-anchor issuance-budget predicate (pure, checked at delegation-sign time — no monitor) | B7 §0-§6 | W3-L7 |
| C4 | Hardware-attestation issuance cost | REJECT-ON-PHYSICS (Firecracker prod: unprivileged, virtio-only, no TPM/enclave passthrough) | B7 §4 | — |
| C5 | Proof-of-work issuance cost | REJECT-ON-PHYSICS-of-mechanism (symmetric cost taxes honest low-resource couriers as much as attackers — the Friedman–Resnick entry-fee failure; strictly dominated by C3) | B7 §4 | — |
| C6 | Refundable stake/bond at the anchor boundary | DEFER-WITH-TRIGGER (viable optional hardening; arms the money-leg red-line + "currency you call a budget" hazard — operator-gated if ever wanted; NOT required, C3 already delivers) | B7 §4 | defer register |
| C7 | Revocation vs zero-watchdog | ALREADY-EQUIVALENT with a bright line: fact-triggered revocation (verified fraud-proof / operator roster action, `revocation.rs:25-26,69-98`) is event-driven and structural — ALLOWED; behavior-monitored auto-revocation is a watchdog AND a score — FORBIDDEN twice over | B7 §5 | — |
| C8 | First-party bilateral memory ("X defaulted on ME") | DEFER — Cheng–Friedman does not forbid it, but it sits at the edge of the no-scoring line; separate operator call, not folded in | B7 §6 | operator docket |
| C9 | Batch/quorum signature verification as a free speedup | REJECT-ON-PHYSICS — measured refutation: SSR-2020 mixed-order forgery fix (`sign.rs:971`, commit `6541ae8`) forces one full verify per batch-accept; batching measured 3.26× SLOWER; no ML-DSA aggregate scheme exists (FIPS 204 single-verify only). Standing design axiom: budget one full verify per item, both legs | B4 §2.5/§3.1 | axiom, recorded here |
| C10 | Cryptographic-quorum PoQ (BLS/threshold/MPC per-message) | REJECT-ON-PHYSICS (BLS is pairing-based, not PQ; no deployed ML-DSA aggregate; per-message budget already dominated by mandatory single verifies) | B4 §2.2 | — |

### 1.4 Execution-model + product-layer concepts (Batches 8-9)

| # | Concept | Verdict | Ground | Build slot |
|---|---|---|---|---|
| E1 | eqc compiler in Python (the rule's own contradiction) | ALREADY-RESOLVED — pure-Rust `tools/eqc-rs/` landed `7c7763af7`, CI runs it, Python trio deleted; only an untracked `tools/eqc/__pycache__/` residue remains (verified this session) | B8 (A) | W1-L9 |
| E2 | `geo.rs:39-41 ema_next` as generated equation | **ADOPT — the template organ** (pure affine `±·`, zero blockers; also the 1-D Kalman the integration arc names) | B8 (B) #1 | W1-L1 |
| E3 | Money law (`domain.rs:95-111` + `money::apply_tax`) as generated fixed-point equation | ADOPT-GATED — blocked on (a) integer basis-points replacing `tax_rate: f64`, (b) eqc-rs checked-overflow emission (its own roadmap), (c) operator red-line gate. Wave 1 builds capability (b); the flip is W4 | B8 (B) #2 | W1-L1 → W4-L1 |
| E4 | `householder.rs:224-229` ≡ `246-250` duplicated 2×2 eigenvalue closed form | ADOPT — consolidate to ONE shared helper (real verbatim-duplication drift hazard; not eqc-able as-is — complex arithmetic) | B8 (B) #3 | W1-L5 |
| E5 | `haversine`/`bearing` via eqc-rs | ADOPT-AFTER-NODES — needs `asin`/`atan2` Expr nodes (f64 path only; fixed-point correctly refused) | B8 (B) #4 | W1-L1 (nodes) → follow-on |
| E6 | `order_machine.rs:311-361` runtime power iteration for ρ | ADOPT — replace with proven `const` ρ=0 (fixed lifecycle DAG ⇒ nilpotent adjacency; a 1000-iter loop rediscovering a theorem per call) | B8 (B) #8 | W1-L4 |
| E7 | FSM adjacency as `match` (order_machine + claim_machine, same shape twice) | ADOPT — shared pinned adjacency-table primitive (procedural→declarative-DATA; a DOD win, not an eqc win) | B8 (B) #9 / (D) | W4-L4 |
| E8 | eqc-ifying reductions/solvers/crypto (softmax, Laplacian scatter, SHA3/FNV, Durand-Kerner, complex ops, CRDT joins, Merkle trees) | REJECT-ON-PHYSICS-of-mechanism — variable-length reductions, GF(2) bit-permutations, and iterative solvers are outside scalar-arithmetic codegen; forcing eqc there is a category error, honestly declined | B8 (B)/(D) | — |
| E9 | Scripts → zero | ALREADY-MOSTLY-MET — of ~315 sh/py files, ~230+ are vendored skill bundles / CI red-line fences / build glue (justified-keep); genuine tail ≈ 15-20 telemetry/self-improvement Python (low-priority port), + stale worktree copies (delete-on-merge) | B8 (C) | hygiene register |
| E10 | Equation-IR + regenerate-and-diff CI gate | DEFER-WITH-TRIGGER (≥3 real organs flowing through eqc-rs) | B8 item 7 | defer register |
| E11 | Product TS layer → kernel/WASM | ADOPT-STAGED (T0-T4 risk tiers) — the bridge already exists (`kernel/src/wasm.rs` full surface + 66-line zero-dep `kernel_client.mjs` template); gap is wiring, not construction. **HARD EXTERNAL GATE:** T4 write paths blocked on the separate NOBYPASSRLS workstream (`docs/ops/P8-NOBYPASSRLS-FLAG.md`) — prod RLS is dormant (BYPASSRLS live), so the TS session-adapter is currently the only live tenant guard; the GUC/role/transaction envelope survives in ANY language, never deleted | B9 §A-§D | W3-L8, W4-L2/L3/L5/L9 |
| E12 | UI as real-time render of kernel state (no-DOM physics-UI) | ADOPT-AS-DESTINATION, staged: Path 1 (WASM view-models feed existing React) first; Path 2 grows island-by-island in the `web/` Astro/Svelte beachhead (already kernel-wired); money-never-tween + text/IME/a11y discrete layer preserved per the field-UI RED proofs — a correctness bound, not a rewrite-cost objection | B9 §C | W4-L5, W4-L10 |
| E13 | RGB-seed procedural state encoding (transmit a generator) | REJECT-ON-PHYSICS for the harmonic form (transcendental float paths are NOT cross-target bit-identical — `rng.rs:22-28`, the repo's own determinism audit; a seed-regenerated state would content-address differently per architecture — mesh-breaking P0); ALREADY-EQUIVALENT for the two sound generator forms: integer PRNG seed (`rng.rs` SplitMix64→PCG64) and truncated spectral reconstruction `W ≈ U_k Λ_k U_kᵀ` (P28 rung 1) | B6 §2 | W2-L2 |
| E14 | Batch 1's real code finding: `engine/src/zerocopy.rs:22` labels interleaved stride-5 (AoS) as "SoA" | ADOPT — fix the inverted label now; store true-SoA when the SIMD Kalman consumer (`simd.rs:20-24`) is built | B1 §4 | W1-L6 |
| E15 | Batch 2's LIVE bug: `commit_after_decide` double-commit on non-empty-log replay (`event_log.rs:359` uses `append`, rebinding `prev` ⇒ dedup misses) | **ADOPT — HIGHEST-PRIORITY CORRECTNESS ITEM.** Port the `append_raw` fix + regression test that already exist in `dowiz-agentic-mesh` (`kernel/src/event_log.rs:380`, test `:582-673`); money red-line (replayed `SettlementClaimed` must never re-run its side effect) | B2 §12 | W1-L2 |
| E16 | Hysteresis on the live `Live↔Locked` flip (`hydra.rs:186-193` flips on instantaneous ρ<1.0 — can flap at the boundary) | ADOPT — two-threshold band + N-consecutive-healthy release; the one live oscillation risk in the safety state machine | B3 §5 | W1-L3 |
| E17 | Numeric-range clamps at the wasm boundary (`compose(w,h,steps)` unclamped at the port; `ParticlePool::new(0)` panics) | ADOPT — bounded-range-as-type at the one boundary that lacks it (P27 A9/A16) | B3 §7 | W1-L7 |
| E18 | Money/order state must never CRDT-merge | ALREADY-EQUIVALENT + ADOPT the executable negative test (pin the red-line so a future merge-PR fails loudly; `apply_pull` fork-refusal `anti_entropy.rs:121-132` makes it impossible to write green today — keep it impossible) | B2 §1b | W1-L8 |
| E19 | MMR under `MerkleLog` (O(n log n) add/root today, `sync_pull.rs:452,457-481`) | DEFER-WITH-TRIGGER — bench first; adopt only if add/root dominates an anti-entropy round at measured n ≳ 10⁴ | B2 §4 | W2-L5 gates |
| E20 | A NEW Merkle-DAG authority for patch/unit history | REJECT-ON-PHYSICS (dual-authority hazard — the exact construction the RCI council overturned, `ADR-realtime-change-intelligence.md:44-50`; a second content-addressed authority can silently desync from the first). DecisionUnit rollback lineage lives in the SAME sha3 content-addressed log the units already register through | B2 §4/§11 · B6 §1.2(c) | W3-L6 |

**Tally: 14 ADOPT (+4 gated/staged) · 10 EXTEND-EXISTING · 17 ALREADY-EQUIVALENT · 16
DEFER-WITH-FALSIFIABLE-TRIGGER · 19 REJECT-ON-PHYSICS.** Zero concepts dropped; Batch 6 §5.1's
38-row sweep + its five gaps are all present above (gaps i/ii → #21/#21b; iii → #3; iv → #24;
v → B7).

---

## §2. REJECTION REGISTER — the physical/correctness reason, restated per the framing rule

Each rejection below is independently verifiable; none rests on convention, consensus, or
complexity.

1. **Reputation-weighted trust / diversity-factor Sybil patch / statistical-vote PoQ** — Cheng &
   Friedman (P2PECON 2005): no *symmetric* reputation function is Sybil-proof; a diversity factor
   is a symmetric-scoring tweak inside the theorem's class. Plus the CI-enforced
   `NO-COURIER-SCORING` red-line (executable gate `ci-no-courier-scoring.sh`; structural constraint
   `claim_machine.rs:13-17`). Plus the operator's binding ruling. (B4 §1, B7 §2.)
2. **Speculative/optimistic execution + rollback; optimistic-fraud-proof PoQ** — degrade-OPEN in a
   degrade-closed architecture; empirical record: no permissionless fraud proof functioned on any
   optimistic mainnet for ~3 years; the latency case is false (verify ≈ 0.1-1 ms ≪ 10-100 ms mesh
   RTT). The kernel already implements the superior pole: verify-before-persist
   (`event_log.rs:389`). (B4 §2.5.)
3. **Batch/aggregate signature verification as a speedup** — measured refutation: the SSR-2020
   mixed-order forgery (`R = R0 + T`, small-order filter blind) forced re-confirming every
   batch-accept with a full single verify (`sign.rs:971`, `6541ae8`); batch/64 measured 3.26×
   slower; no standardized ML-DSA aggregate exists. (B4 §2.5/§3.1.)
4. **Per-message ZK/validity proofs** — proving is ~10⁶× native execution (measured 59 s vs 15 µs
   in the cited literature); signature verify is the real-time primitive. Checkpoint/light-client
   STARK survives as an off-hot-path DEFER. (B4 §2.7.)
5. **HLC as ordering authority** — folds physical wall-clock into the ordering timestamp, breaking
   the replay-determinism both the code (`hybrid_gate.rs:104` monotonic, no wall-clock; per-actor
   `seq` ordering) and the dialogue's own determinism section require. Logical half already exists.
   (B2 §7.)
6. **Mergeable CRDT (LWW/counter) over money/order state** — re-introduces the
   double-spend/conflicting-append hazard; `apply_pull` structurally refuses forks
   (`anti_entropy.rs:121-132`); "money is event-sourced, NEVER CRDT-merged" is a stated code
   invariant (`event_log.rs:4`). (B2 §1b.)
7. **A second Merkle-DAG authority** — dual-authority desync hazard, the construction the RCI
   Triadic Council overturned (`ADR-realtime-change-intelligence.md:44-50`). (B2 §4.)
8. **Raw L2 / custom Ethernet frame carrier** — no shared L2 broadcast domain exists on either
   substrate (Fly microVMs are L3/WireGuard-routed; the dev host is one VM with no L2 peer), and a
   cleartext header would move routing-critical fields outside the signed envelope — discarding the
   mesh's authenticity model. Fields adopted INSIDE the envelope. (B5 §1.2.)
9. **RDMA/RoCE** — the device does not exist: no `/dev/infiniband` on dev (live probe); virtio-only
   on prod; RoCE additionally needs a lossless fabric nobody has. (B5 §1.3.)
10. **Full DPDK** — binding the single shared virtio NIC to VFIO removes the machine's only network
    path (self-defeating by construction); Firecracker has no NIC to pass through. (B5 §1.5.)
11. **RSS/hardware flow steering; eBPF loader in prod** — virtio/Firecracker expose no hardware
    ntuple steering; prod guest lacks `CAP_BPF`. The eBPF *ideas* are already ported to userspace
    by P24's stance. (B5 §1.6.)
12. **NUMA pinning** — 1 socket / 1 NUMA node (live `lscpu`): a no-op, adopted as a decision.
    (B5 §1.7.)
13. **Harmonic RGB-seed codec for general state** — transcendental float paths are reproducible
    per-target, not cross-target (`rng.rs:22-28`, the repo's own determinism boundary); a
    seed-regenerated state would hash/sign differently on x86 vs ARM — a mesh-breaking P0 against
    the content-address/signature model. Integer-PRNG and spectral-generator forms stand. (B6 §2.2.)
14. **t-SNE / UMAP / iterative-LLE** — stochastic initialization + SGD violate the bit-identical
    determinism contract (`csr.rs:36-37`); a mechanism rejection, not taste. (B1 B12.)
15. **Laplace-domain primitives (B1/B6) and the Kuen surface** — no continuous s-plane / no
    hyperbolic-geometry implementation surface exists in a discrete deterministic kernel; B6's
    discrete counterpart (B4, the z-transform lens) is adopted instead. (B1 §3.)
16. **Literal pixel-mipmap of a token stream** — box-filtering tokens is semantically meaningless;
    the intent lands as spectral coarsening (P28 rung 1) + CDC dedup. (B1 A10.)
17. **Free-form rapid-fire auctions / market consensus** — transient-acquirable-weight class
    (Beanstalk-shaped; measured MEV steady state); assignment already solved coordination-free by
    rendezvous/HRW under capability authorization (`matcher.rs`). Narrow sealed-batch form remains
    permitted if price discovery is ever genuinely needed. (B4 §2.1.)
18. **Self-auditing inline witnessing** — the author's own proof-of-transition is the RC-2
    self-certification gap; verification must cross the author/verifier divide (WorkReceipt /
    key_V shape). (B4 §2.2.)
19. **Hardware-attestation & PoW as issuance costs** — attestation: no TPM/enclave surface on
    Firecracker (physics); PoW: symmetric cost that taxes honest low-resource couriers while a
    resourced attacker out-computes them (the entry-fee exclusion failure) — strictly dominated by
    anchor-rooted issuance. (B7 §4.)
20. **eqc-ification of reductions, iterative solvers, GF(2) crypto, CRDT joins** — outside the
    reach of scalar-arithmetic codegen (variable-length Σ, bit-permutations, no closed form);
    correctly hand-written / type-enforced. (B8 (B)/(D).)

---

## §3. SAME-FILE CONFLICT RESOLUTIONS (no contradictory instructions per file)

| File | Touched by | Resolution |
|---|---|---|
| `kernel/src/householder.rs` | Batch 8 (eig2x2 dedup at `:224-229`/`:246-250`, inside the values-only deflation path) AND the eigenvector plan (additive `eigh_contig` + `tridiag_qr_symmetric`; existing functions byte-identical) | ONE ordered lane: **W1-L5 lands the eig2x2 consolidation first** (small, isolated, existing tests pin it), **then W2-L2 adds the eigh machinery** on the cleaned file. Never two concurrent writers. The eigenvector plan's additive-only guarantee (§5.4.7 there) is unaffected — the consolidation refactors a helper the additive code does not touch, and the 8 hand-oracle tests gate both steps. |
| `kernel/src/spectral.rs` | Eigenvector plan (adds `eigh` façade + `topk_symmetric` tier) AND Batch 1 B13 (z-pole ↔ `DriftClass::Resonant` doc note at `spectral.rs:315-352`) | ONE lane (W2-L2 owns the code additions); the B13 doc note rides W2-L6's doc pass but is a comment-only edit to a region W2-L2 does not modify — sequence W2-L6 after W2-L2 merges within the wave. |
| `engine/src/field_frame.rs` | Batch 3 clamps (`compose` port `:218-225`) AND Batch 1 B14 (Nyquist bound doc + test beside the CFL doc `:19-27,59-72`) | Sequenced: **W1-L7 clamps first**, W2-L6 Nyquist doc after. Different regions, but same file ⇒ never concurrent. |
| `kernel/src/event_log.rs` | Batch 2 only (`append_raw` port) | Single owner, W1-L2. The drift-gate region (`:389-419`) cited by Batches 3/4/6 is read-only precedent, not an edit target. |
| `kernel/src/order_machine.rs` | Batch 8 #8 (ρ=0 const, W1-L4) AND Batch 8 #9 / (D) (shared adjacency table, W4-L4) | Different waves by design; W4-L4 builds on the W1-L4 result (the const derivation and the adjacency table are the same underlying fact — the FSM graph is compile-time data). |
| `bebop2 proto-wire/discovery.rs` + envelope schema | Batch 2 §8 (epoch counter) AND Batch 5 §1.1-1.2 (same epoch + TileID/EpochID/HypothesisID/Seq fields) | These are the SAME design (Batch 5 cross-referenced Batch 2's HLC caveat): one W2-L4 lane designs the field set once — logical epoch only, no wall-clock, all fields inside the signed envelope. |
| P28 arena vs Batch 1 A8 | Batch 1 explicitly rules "build P28's arena, do not design a second one" | W2-L1 = build to P28 §3.3's spec verbatim + the HugePageHint seam extension. One arena. |
| Latency §2 Decision Compiler vs Batch 6 DecisionUnit gossip | Confirmed the same idea | Latency blueprint §2 stays design authority; W3-L6 adds ONLY the three distributed extensions (epoch, import gate, rollback-in-same-log). No re-design. |

---

## §4. EQUATIONS-NOT-PRIMITIVES, APPLIED CONCRETELY (why Wave 1 is what it is)

The eqc-rs port landed (`7c7763af7`: zero-dep `Expr` AST, dual f64/Q-format emission,
emit→compile→run→self-assert proof harness, CI-wired). The rule's smallest, most literal
application is now unblocked, and Batch 8 already named the exact organs with `file:line`:

1. **`geo.rs:39-41 ema_next`** — `prev + alpha*(sample-prev)`: pure `±·` affine map, zero
   blockers, and simultaneously the 1-D Kalman the integration arc identified. Wave 1 authors it
   as an `Expr`, generates the fn, and lands the parity `#[test]` beside it — proving the
   author-equation → generate → commit → parity-gate loop end-to-end on a real kernel organ for
   the first time.
2. **`domain.rs:95-111` money law** (`total = subtotal·(1+rate) + fee`, Q-format i64) — the
   flagship, deliberately staged: Wave 1 builds the eqc-rs capabilities it needs
   (checked-overflow emission; already on eqc-rs's own roadmap) and adds `asin`/`atan2` nodes
   (unblocking haversine/bearing as follow-ons); the money flip itself is W4-L1, gated on the
   integer-basis-points change and the operator's red-line sign-off. *(Adjudication T-1, §7:
   the task framing put the money law in Wave 1; Batch 8's evidence gates it — Wave 1 does the
   money law's unblocked half, the gated half keeps its gate.)*
3. **The honest boundary stands:** reductions, iterative solvers, GF(2) crypto, and the CRDT/FSM
   combinatorial layer are NOT eqc targets (ledger E8) — for those, "equations not primitives"
   means the other two moves the batches identified: replace a runtime loop with a proven theorem
   constant (`order_machine` ρ=0, W1-L4) and replace procedural `match` duplication with
   declarative pinned data (shared adjacency table, W4-L4).

---

## §5. PRIORITIZED BUILD ORDER — waves of collision-free lanes

Ordering principle: smallest kernel abstraction first (operator: "від малого до великого,
найменші абстракції на рівні ядра … перші"), waves sized for concurrent swarm dispatch
("розумні структури … на паралельну конкурентну роботу хвилями роїв"). Every lane in a wave is a
distinct file-set with no ordering dependency inside the wave; each wave's output unblocks the
next. Done-checks are the batches' own falsifiers. CPU-bound verification queues through the
P25 4-slot budget; the agent lanes fan out freely (roadmap §2 wave-admission note).

### WAVE 1 — correctness closure + first equations (all buildable today; zero operator gates)

| Lane | Item | Files | Done-check |
|---|---|---|---|
| W1-L1 | eqc-rs → `ema_next` generated + parity test; add `asin`/`atan2` nodes + checked-overflow emission mode | `tools/eqc-rs/src/lib.rs`, `kernel/src/geo.rs` | parity `#[test]` green vs hand-written `ema_next` (bit-identical f64); eqc-rs proof harness green on the new nodes |
| W1-L2 | **Port `append_raw` exactly-once fix** + regression test from `dowiz-agentic-mesh` (`event_log.rs:380`, test `:582-673`) onto this branch's `event_log.rs:359` | `kernel/src/event_log.rs` | `commit_after_decide_replay_on_nonempty_log_is_true_duplicate` present + green; `decide` called once across replay; `log.len()` unchanged |
| W1-L3 | Hysteresis band on `Hydra::integrity_check` (`hydra.rs:186-193`): trip at ρ≥1+ε_hi, release at ρ≤1−ε_lo or N consecutive healthy | `kernel/src/hydra.rs` | RED test: ρ dithering around 1.0 must not flap `Live↔Locked` |
| W1-L4 | `order_machine::spectral_radius` (`:311-361`) → proven `const` ρ=0 (nilpotent DAG), golden-signature gate kept | `kernel/src/order_machine.rs` | existing golden-signature test green; the 1000-iter loop gone; a doc-proof comment states the theorem |
| W1-L5 | Consolidate duplicated 2×2 eigenvalue closed form (`householder.rs:224-229` ≡ `246-250`) into one shared helper | `kernel/src/householder.rs` | all 8 existing hand-oracle/parity tests green, byte-identical eigenvalues |
| W1-L6 | Fix inverted layout label (`engine/src/zerocopy.rs:22`: interleaved stride-5 = AoS, not SoA) + note the true-SoA obligation for the future SIMD Kalman consumer (`simd.rs:20-24`) | `engine/src/zerocopy.rs` | comment corrected; grep-able convention recorded |
| W1-L7 | Numeric clamps at the wasm boundary: `compose(w,h,steps)` (`engine/src/field_frame.rs:218-225`, `wasm/src/lib.rs:57-59`) + reject `ParticlePool::new(0)` | `engine/src/field_frame.rs`, `wasm/src/lib.rs`, particle pool | RED tests: oversized/zero inputs return typed errors, never panic/OOM |
| W1-L8 | Executable negative test pinning "money/order never merges": two conflicting same-`seq` `SettlementRecorded` ⇒ `apply_pull` returns `EventLogError{fork/overlap}` | bebop2 `core/src/anti_entropy.rs` tests | the merge-green outcome is impossible to write |
| W1-L9 | Hygiene: delete untracked `tools/eqc/__pycache__/`; delete stale `dowiz-agentic-mesh/tools/eqc/*.py` + `markov_attractor.py` copies on that branch's merge | tools trees | `tools/eqc/` gone entirely; no Python in the eqc lineage |

### WAVE 2 — smallest new kernel abstractions (the substrate everything later sits on)

| Lane | Item | Files | Done-check |
|---|---|---|---|
| W2-L1 | `kernel/src/arena.rs` `BumpArena` exactly per P28 §3.3 (`UnsafeCell<Vec<u8>>` + bump offset + `high_water`, `T: Copy`, O(1) reset, degrade-closed, `_in` variants) + named `HugePageHint` NoOp port copying `core_pinning.rs:41-64` (trigger: region > 2 MB) | new `kernel/src/arena.rs` | criterion A/B heap-vs-arena in `BENCH_HISTORY.md`; ≤8 heap allocs on arena path; byte-identical PPR output; Miri-clean |
| W2-L2 | Eigenvector plan R1-R3: `reduce_hessenberg` accumulator + `eigh_contig` + `tridiag_qr_symmetric` (householder.rs, after W1-L5); `spectral::eigh` façade + `spectral::topk_symmetric` sparse tier (born arena-aware per P28 W5) | `kernel/src/householder.rs`, `kernel/src/spectral.rs` | the plan's §5.4 suite: KAT `A·v=λ·v`, orthonormality, values-parity, sparse-vs-dense parity, byte-determinism, reconstruction-error monotonicity, old suites untouched |
| W2-L3 | `kernel/src/breaker.rs` `CircuitBreaker` per P27 §3.2 (Closed/Open/HalfOpen, EMA trip via `geo.rs::ema_next` — now the eqc-generated organ, `min_calls` floor, `probe_successes` hysteresis) | new `kernel/src/breaker.rs` | P27's table-tests on the pure `step()` core |
| W2-L4 | Logical epoch counter (Lamport max-merge, NO wall-clock) in the gossip roster + fold `TileID/EpochID/HypothesisID/Sequence` into the signed envelope schema (never a cleartext header); resolves the `SystemTime::now()` determinism defect at `iroh_transport.rs:391` for ordering purposes | bebop2 `proto-wire/discovery.rs`, envelope schema | 3-node variant of `gossip_converges_3node`: differing start epochs converge to max; **assert no `SystemTime` on the path**; envelope round-trip test |
| W2-L5 | Evidence benches gating the DEFERs: criterion `MerkleLog::add`/`root` + `anti_entropy::digest` at n ∈ {10², 10³, 10⁴}; pure-Rust `pq_dsa` verify p99 on this host (B4-C2) | bench files | committed baselines in `BENCH_HISTORY.md`/`baseline.json`; regression-gated |
| W2-L6 | Nyquist source-bandwidth bound (fs=50 Hz ⇒ 25 Hz ceiling) documented + tested beside the CFL bound; z-pole ↔ `DriftClass::Resonant` doc note (`spectral.rs:315-352`); carry the Nyquist reasoning to the W2-L4 epoch-clock design note | `engine/src/field_frame.rs` (after W1-L7), `kernel/src/spectral.rs` (after W2-L2) | test asserting an above-Nyquist source is flagged as documented |
| W2-L7 | Branchless/sentinel hardening of the CSR/GEMM sparsity skips (`csr.rs:175`, `mat.rs:102`) using the proven `simd.rs` mask pattern | `kernel/src/csr.rs`, `kernel/src/mat.rs` | byte-identical output vs branchy path (the `simd.rs` bit-identity rule) + criterion delta |

### WAVE 3 — mesh composition on the Wave-2 substrate (operator rulings R-1…R-3 unblock marked lanes)

Operator docket for this wave (each a narrow, named ruling — Descartes-square treatments live in
the cited batch sections):
**R-1** `0x12→0x13` discriminant shift (B1's `AgentBridge=0x12` landed ⇒ B2's
`WorkReceipt→0x13`, `Settlement→0x14`; wire-stability hard-gate — B4-C3).
**R-2** budget-unit semantics: consumable-not-transferable vs accumulable balance (B4-C4;
decides whether the budget leg needs the money red-line arming).
**R-3** `RootDelegationPolicy` choice (`node_id.rs:156-184`, fail-closed today): OperatorSigned /
FirstContactQr / WebOfTrust-as-delegation-flow (never vote-count) — Batch 7 §6.
**R-4** money-law eqc flip + S2 integer basis-points (gates W4-L1).

| Lane | Item | Files / gate | Done-check |
|---|---|---|---|
| W3-L1 | 3-way relation-slice tensor: type-tagged CSR slices keyed by `matrix_content_address` (the P28 rung-2 / RESCAL substrate; also the mesh DecisionUnit state object) | kernel tensor layer (arena-aware) | identical slice-sets ⇒ identical content-address (DecompCache-style no-thrash falsifier) |
| W3-L2 | `WorkReceipt` (semantic-contract PoQ): canonical-TLV binding (capability revocation-hash, input/output content-ids, budget, nonce, expiry), `RequireBoth`, counterparty-verified via `HybridGate::check` (`hybrid_gate.rs:124`) | bebop2 proto-cap · **gated on R-1** | receipt verifies only through the counterparty's gate; appended to both WORM logs |
| W3-L3 | Priority = nested `TokenBucket` envelopes `BTreeMap<(PeerId, CapabilityClass), TokenBucket>`; wire priority flag = envelope selector checked against capability scope | dispatcher/bucket composition · **R-2 informs the budget-leg arming** | a peer cannot draw from an envelope its capability doesn't grant (RED test) |
| W3-L4 | Restart-intensity bound in structural form: a relaunch-count monotone fact checked by a pure predicate IN the launch path (degrade-closed refuse-to-relaunch past MaxR/MaxT) — never a standing monitor (§7 T-6); systemd `StartLimitBurst` permitted as substrate-physics (same class as the wasmtime OutOfFuel trap) | drainer launch path | RED test: a crash-looping drainer stops relaunching and surfaces one Blocker line; no polling process exists |
| W3-L5 | Fuel invoke-time wiring + `FUEL_PER_UNIT` pin via criterion bench (primitive is built: `fuel.rs:91-208`; admission mints budget but never invokes the loop) | agentic-mesh invoke path | a compute-bomb guest on the REAL invoke path is terminated by `OutOfFuel`, typed, never resumed |
| W3-L6 | DecisionUnit distributed extensions per Batch 6 §1.2-1.3: (a) unit epoch = gossip epoch in the provenance header; (b) **import gate = once-per-artifact independent replay** (receiving hub replays the harvested instance-set through the unit + its own oracle; disagreement ⇒ reject) — the verify-before-persist / key_V shape, never optimistic, never statistical; (c) rollback lineage keyed in the EXISTING sha3 content-addressed registry (no second DAG) | decision-units registry + gossip payload | replay-gate RED test (a poisoned unit is refused); a stale unit answers `Escalate` unconditionally; red-line shapes operator-gated |
| W3-L7 | Anchor issuance-budget predicate: per-anchor monotonic epoch/nonce budget checked at delegation-sign time (pure predicate, no monitor) — closes Batch 7's caveat | bebop2 proto-cap (`roster`/`node_id`) · **gated on R-3** | RED test: the N+1-th delegation inside a budget window is refused at sign time; `verify_chain` behavior unchanged |
| W3-L8 | Product T1 read-only wiring: `kernel/pkg` (Node target) into live API paths where a bug is cosmetic (`channel_ledger_js`, `reduce_anomalies_js`, `fsm_graph_report_js`, geo progress/ETA), reusing the `kernel_client.mjs` fail-closed decode pattern | product adapter layer | parity vs TS output on live fixtures; no RLS surface touched |
| W3-L9 | **Predictive-tensor-handoff research pass** (the one unowned dialogue concept): drone-following tiles / ghost prefetch / atomic pointer swap / shadow-tile multi-hypothesis with probability-weighted pruning — grounded on the existing Kalman substrate (`geo.rs::ema_next`) + splatting arc; output = a batch-grade findings doc with its own verdicts incl. the lock-free handoff mechanism (#21b) | research (no code) | a findings file in this directory; every sub-concept verdicted with cites |

### WAVE 4 — authority flips + heavier legs (each individually gated as marked)

| Lane | Item | Gate | Done-check |
|---|---|---|---|
| W4-L1 | Money law via eqc-rs Q-format: `domain::compute_order_total`/`money::apply_tax` generated, parity-gated bit-identically against the hand-written oracle | **R-4** (S2 integer basis-points + operator red-line) | parity `#[test]` bit-identical across the whole fixture corpus before the swap |
| W4-L2 | Money dual-authority collapse (T2b): server CHARGE path calls the kernel (`estimate_order_total_js`/`money.rs`), bit-identical parity-gated vs the TS `money.ts` oracle before the flip (`wasm.rs:332-335` names the split) | money red-line | display and charge produce identical integers on the full test corpus; flip only after proven in prod on the display path |
| W4-L3 | State-machine authority (T3): order FSM decision → kernel `apply_event_js`; port the shift-FSM (`shifts.ts:206-294` guards) into the kernel beside `order_machine`; DB-writing adapters stay | after W3-L8 proves the seam | FSM parity suite; adapters unchanged in behavior |
| W4-L4 | Shared FSM adjacency-table primitive: one pinned declarative table + `assert_transition` consumed by `order_machine` AND `proto-cap::claim_machine` (same lane as/after W4-L3 — shared surface) | — | both consumers byte-identical to their `match` predecessors; single source of adjacency truth |
| W4-L5 | Frontend Path-1: extract the pure view-model fns (`displayCategories`, `toggleModifier`, `canAdd`, `getAllProducts`, `bomToNutrition`) to kernel/WASM, feed the unchanged React tree; money preview via `estimate_order_total_js` | — | JSX untouched; view-model parity tests |
| W4-L6 | Spectral convergence advisory: promote `mesh_consensus.rs` λ₂/SLEM from test to a fail-closed advisory runtime signal (λ₂→0 ⇒ partition alarm) — the capability-grounded "swarm-converges" answer | — | advisory only, degrade-closed; parity with the test's analytic assertions |
| W4-L7 | M7 topological self-heal: mesh-node topology primitive + Dijkstra/Union-Find reconnection (completes the Self-Healing leg's topological axis) | mesh seam (P9/P10) | Hermetic #26 closed; reconnection under partition proven in the n-node suite |
| W4-L8 | Durable snapshot + restore-drill (completes Snapshot-Re-entry): compaction/retention for append-only stores + a `restore-verify` subcommand drilled on a timer | owned by P12 | Hermetic #4 closed: a restore-drill has RUN |
| W4-L9 | Product T4 write paths (`POST /orders`, `PATCH /:id/status`, courier assignment): kernel computes the decision; tenant-GUC + transaction envelope + `WHERE location_id` survive as the thin adapter, never deleted | **HARD EXTERNAL GATE: the separate NOBYPASSRLS workstream** (`docs/ops/P8-NOBYPASSRLS-FLAG.md`) must land its flip + resolve role-name drift first — prod RLS is dormant today and the TS adapter is the only live tenant guard | RLS-adversarial suite green with the kernel in the decision seat |
| W4-L10 | Frontend Path-2: no-DOM physics-UI islands in the `web/` Astro/Svelte beachhead (already kernel-wired via `kernel_client.mjs`); money-never-tween + text/IME/a11y discrete layer preserved per the field-UI RED proofs; joins the physics-ui/field-UI arcs (FE-01 VertexBridge gap is that arc's first wire) | — | island-by-island Gain−Loss adoption; never a flag-day cutover of `apps/web` |

### DEFER REGISTER — named seams with numeric triggers (the "birds fly later" list)

Every entry keeps the `core_pinning.rs:41-64` shape where code-adjacent: a named port, a NoOp
default, a numeric trigger in the doc comment, a `#[ignore]` failing-by-design activation test.

| Item | Trigger (measurable) | Source |
|---|---|---|
| Morton/Z-order (A2) | field grid working set > L2 (≈ >256×256 f32) AND a blocked stencil exists | B1 |
| `repr(align(64))` (A5) | a multi-thread adjacent-write structure appears, OR criterion shows an aligned-load win | B1 |
| Software prefetch (A6) | `nnz*16 B > L2` AND spmv profiled memory-bound | B1 |
| HugePage madvise behind `HugePageHint` (A7) | persistent arena region > 2 MB by `high_water()` | B1 (seam built W2-L1) |
| Tiling/blocked GEMM (A9) | dense matmul n≥128 on a hot path | B1 |
| MMR under `MerkleLog` | W2-L5 bench shows add/root dominating an anti-entropy round (n ≳ 10⁴) | B2 |
| Merkle range-reconciliation replacing linear `diff` | digest materialization/transfer dominates the measured round budget | B2 |
| Reorder buffer ("Sync-Debt") for the linear chain | a per-event push path that can deliver seq-gaps lands | B2 |
| Rolling checkpoint + truncation | measured replay/verify cost over budget; **council-gated** (money red-line) | B2 |
| Adaptive epoch length | after the epoch exists (W2-L4) + a measured need | B2 |
| Isomap embedding | a manifold-embedding consumer + a k-NN builder exist | B1 |
| Delta-gossip | roster scale where full-roster bytes/round are measurable | B5 |
| AF_XDP (dev only) | bare-metal multi-node + measured UDP-copy-dominant profile | B5 |
| isolcpus | a dedicated bare-metal mesh node | B5 |
| io_uring | a syscall-heavy local file-I/O path (arena/block-store) measured as bottleneck | B5 |
| Checkpoint/light-client STARK (B4-C8) | periodic FSM-replay audit need; STARK-only (PQ-consistent); off hot path | B4 |
| Token-stream pooling/LOD primitive | after P28 rung 1 lands (it IS the coarse tier); anti-alias per Nyquist | B6 |
| Regime-2 open-world shape matching | ≥5 live regime-1 DecisionUnits generating match telemetry | latency §2.4 |
| Stake/bond issuance hardening (C6) | operator wants economic skin-in-the-game AND accepts the money-leg gate | B7 |
| First-party bilateral memory (C8) | separate operator call (edge of the no-scoring line) | B7 |
| Equation-IR + regenerate-and-diff CI gate | ≥3 real organs flowing through eqc-rs | B8 |
| Telemetry/self-improvement Python tail (~15-20 files) port | opportunistic; lowest priority of the scripts→0 effort | B8 |
| Full per-object vector clocks | a multi-writer-single-object requirement lands | B2 |
| Advisory signed non-ordering wall-time field | a real-time-bounded product requirement (e.g. cross-node offer expiry) | B2 |

---

## §6. RELATIONSHIP TO P06 (key_V) — the cross-arc blocker gains a fourth consumer

P06's split-identity independent verification (`key_K` claims, `key_V` re-executes and signs;
`v1-verify` wired but `signed:false` until Phase 3 closes C4b) was already the 3-way convergent
blocker (H3 hermetic, E3-Phase-B spectral, roadmap Phases 7/9/10). This initiative adds:

1. **DecisionUnit import gate (W3-L6) is structurally the key_V pattern** — Batch 6 §1.3 proved
   the safe PoQ form is once-per-artifact independent replay, "identical in shape to the P06
   key_V independent re-execution" (its words). The UNSIGNED local form (receiving hub replays
   through its own oracle — verifier ≠ author by construction, since importer ≠ author-hub) can
   build before P06 closes. The SIGNED cross-hub form — a portable verdict any third hub can
   trust without re-replaying — plugs into P06's `Signer` slot and **waits on P06/C4b**. Until
   then, every importing hub pays its own replay (correct, just not amortized).
2. **The tamper-leg residue points at key_V** — Batch 6 §4's deepest finding: a system cannot
   make its own compromise unrepresentable to itself; `integrity_check`/`boot_verify` are the
   irreducible self-checks, and the only closure of that self-certification gap is an independent
   second party — key_V. The zero-supervisor ideal is fully earned on arithmetic invariants and
   closes on the tamper leg exactly when P06 lands.
3. **Sybil issuance is the same substrate** — Batch 7 §3 mapped P06's K≠V gate term-for-term onto
   anchor-gated issuance (both are "a claim needs a check by a different identity", both reuse the
   MESH-12 `load_genesis` shape). No new dependency — but rulings R-3 (delegation policy) and
   P06's verifier-isolation ruling (O9) are siblings and should be decided together.

**Net:** nothing in Waves 1-2 waits on P06. W3-L6's signed form and the tamper-leg closure do.
P06 remains the highest-leverage convergent unblock, now 4-way.

---

## §7. TENSION ADJUDICATIONS (explicit, per the "state which you chose and why" rule)

| # | Tension | Ruling here | Why (one line) |
|---|---|---|---|
| T-1 | Task framing: "Wave 1 = ema_next + money law" vs Batch 8 gating money on S2 + overflow-guard + operator red-line | Wave 1 does ema_next end-to-end + builds the money law's missing eqc-rs capability; the flip is W4-L1 behind R-4 | The gated split is the more falsifiable, buildable next step; ungating money by fiat would break the red-line the batches ground |
| T-2 | "No fear of complexity" vs "smallest kernel abstraction first" | Smallest-first governs wave ORDER; complexity-tolerance governs wave CONTENT (nothing was shrunk or dropped to look simple) | The operator's own від-малого-до-великого is itself the later, more specific instruction |
| T-3 | Batch 2 REJECTS HLC vs Batch 5 proposes an "epoch/HLC field" | Logical Lamport max-merge epoch only; no wall-clock ever on the ordering path | Batch 5 itself deferred to Batch 2's caveat; the two batches agree once "HLC" is read as "epoch" |
| T-4 | Batch 4 flagged reputation as operator-blocked; operator ruled | Ruling applied as final: reputation family → REJECT rows (C1/C2, statistical PoQ); capability issuance → ADOPT (C3), proven by Batch 7 | Binding, not re-litigated (memory) |
| T-5 | Batch 4 rejects optimistic fraud-proofs vs Batch 6 wants a PoQ gate on gossiped units | Import-time independent replay ADOPTED; optimistic + statistical forms stay REJECTED | The once-per-artifact/per-transaction distinction is real: an import gate is verify-before-persist, not degrade-open optimism |
| T-6 | Operator's zero-watchdog ruling vs Batch 3/6 finding that the restart (crash-loop) leg irreducibly needs a check | Restart-intensity lands ONLY in structural form: a monotone relaunch fact checked by a pure predicate IN the launch path (like `debit` IS the gate) — degrade-closed refuse-to-launch; a standing sampler process is never built; the systemd variant is substrate-physics, same class as the wasmtime OutOfFuel trap | Honors the ruling's letter (no supervising process) while closing the physics gap Batch 6 proved cannot be typed away |
| T-7 | Batch 9's "prod still runs the TS" vs the sovereign branches' apps-drop | The migration is staged T0→T4 with the RLS workstream as a hard external gate on T4; the attic is not treated as a completed migration | Deleting the only live tenant guard while the Postgres backstop is switched off is a correctness violation, not caution |
| T-8 | Invasive-refactor license vs the eigenvector plan's additive-only design | The additive shape is kept — it was chosen there for falsifiability (old oracles pin the new code), not for caution; W1-L5's consolidation IS an invasive edit where the evidence (verbatim duplication) demanded one | "Have courage" licenses refactors the evidence justifies; it does not mandate churn where additive is strictly safer per test-coverage |

---

## §8. REGISTRATION

Registered in `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §8.12 as
**Phase 30**, same protocol as §8.1: off-critical-path lane structure, Wave 1 startable
immediately, P06 relationship as stated in §6, RLS as an external parallel workstream (never
folded into this build order). Operator docket additions: R-1…R-4 (§5 Wave 3) + the C8
bilateral-memory flag. Navigation for this directory: `INDEX.md` (sibling file).
