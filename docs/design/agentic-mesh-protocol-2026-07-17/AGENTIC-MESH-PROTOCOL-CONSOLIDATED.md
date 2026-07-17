# AGENTIC-MESH-PROTOCOL — Consolidated Index (2026-07-17)

> Arc: `docs/design/agentic-mesh-protocol-2026-07-17/`, branch `feat/agentic-mesh-protocol-2026-07-17`
> (worktree). **Planning artifact only — no code is written or edited by this arc.** This document is
> the entry point; the ten source documents below remain the source of truth and are deliberately
> KEPT, not merged away (see §6 for why the Detailed Planning Protocol's delete-intermediates step is
> not applied here, same deviation the spectral-evolution arc recorded). Where a summary sentence here
> and a source document disagree, the source document wins — except where §5 flags a cross-document
> contradiction, which no single source can win.

---

## §0 — Index and how to read

Five research passes, one codebase-grounded synthesis, four execution blueprints. Read in this order:

1. **`R1-web3-failure-modes.md`** — root-caused failure dossier (bridges, consensus, oracles,
   reputation, MEV/auctions, optimistic execution, tokenomics); two meta-patterns (concentrated
   authority; manipulable instantaneous state on a critical path).
2. **`R2-web3-good-patterns.md`** — what actually worked and why, mechanism-first (ERC-4337 policy
   layer, validity-vs-fraud proofs, FROST/DKG, sequencer escape hatches, content addressing, seL4/
   Cap'n Proto capability lineage, signature-not-SNARK real-time verification).
3. **`R3-agentic-infrastructure.md`** — MCP/A2A/x402/AP2/framework survey; the load-bearing gap: no
   existing protocol supplies a decentralized, PQ-capable, cryptographic agent trust plane (R3 §8).
4. **`R4-realtime-crypto-verification.md`** — measured latency budgets for the built substrate;
   the P0 flag (in-repo `pq_dsa` verify is unbenchmarked); QRNG doctrine made concrete.
5. **`R5-quant-trading-risk-lessons.md`** — finance risk-containment patterns (circuit breakers,
   15c3-5 exposure limits, frequent batch auctions, HTLC DvP settlement, named-exploit RED list).
6. **`SYNTHESIS-codebase-and-architecture-direction.md`** — live-code grounding (§1), the five
   open-question resolutions (§2), the Agent Exchange Plane (§3), requirement checks (§4), the
   blueprint wave (§5). The arc's center of gravity.
7. **`BLUEPRINT-B1-agent-bridge-port-manifest-admission.md`** — execution-ready.
8. **`BLUEPRINT-B2-work-receipt-settlement.md`** — execution-ready **after** the P07 §2 dedup fix
   lands (hard precondition, §3/§4 below).
9. **`BLUEPRINT-B3-exposure-ledger-envelopes.md`** — split-landable (rate-envelope half early,
   ledger half after B2's TLV freezes).
10. **`B4-crypto-groundtruth-bench-batching.md`** — the numbers-provider; its measured ledger
    retroactively grounds B1/B2/B3's latency-adjacent acceptance criteria.

**Closure note:** this arc supersedes and resolves the task queued earlier as "Bebop2
consensus/dispatcher evolution" — the market-negotiation, self-auditing, optimistic-execution,
finality-tiering, and priority-dispatcher questions from the pasted external conversation are all
five resolved, with citations, in `SYNTHESIS` §2.1–§2.5 (restated in §2 below). Nothing from that
queue remains open outside this arc.

This document indexes; it does not re-blueprint.

---

## §1 — The architecture direction in one page

**The Agent Exchange Plane: three thin layers over the existing mesh substrate** (SYNTHESIS §3).
The organizing finding is R3 §8's: every surveyed protocol — MCP, A2A, x402, AP2, TEE attestation —
punts the decentralized cryptographic trust plane to OAuth/DNS/CA authorities, a ~70%-share
facilitator, a platform issuer, or a hardware vendor; and every orchestration framework is
trust-the-text (MAST measures 21.3% of multi-agent failures as unverified claims, R3 §3). "That part
must be built, and the survey says it is the only part that must be." Everything else is deliberate
reuse of RED-tested machinery.

- **Layer 1 — `AgentBridge`** (SYNTHESIS §3.1 → B1): the `LlmBackend` port pattern generalized.
  Discovery artifact = `AgentManifest`, borrowing MCP's three-primitive grammar and A2A's Agent Card
  (R3 §1–2) but mandatorily hybrid-signed (canonical TLV, never optional JWS), fail-closed capability
  bits, enumerable-only config axes (E3 lattice discipline), and validation-policy-as-data with an
  unrelaxable `RequireBoth` floor (R2 §1's ERC-4337 lesson). Admission = the existing
  `HybridGate::check`, then `SandboxTier` caging (WASM default, microVM behind the fail-closed KVM
  probe) and a minted `TokenBucket` envelope.
- **Layer 2 — `WorkReceipt` + `Settlement`** (SYNTHESIS §3.2 → B2): counterparty-verified signed
  receipts ("prove the envelope, not the model computation," R3 §3) and HTLC-style pairwise
  delivery-versus-payment (Herlihy PODC 2018, R5 §5), both riding `SignedFrame` into the WORM log via
  `commit_after_decide`. Money-scoped settlements stay behind the armed red-line gate and S9
  integer-money law.
- **Layer 3 — `ExposureLedger`** (SYNTHESIS §3.3 → B3): per-counterparty outstanding-commitment
  caps that heal **only on settlement events, never by a clock** — the stock-vs-flow distinction R5
  §3 shows `TokenBucket` cannot cover — checked pre-persist in the same commit-path slot as the
  drift gate, with a graduated limit-state and defined auto-reopen (R5 §1's LULD lesson).

**Reused verbatim** (full list: SYNTHESIS §3.4): `HybridGate`/`verify_chain`/`RevocationSet`/
`NodeId`/`pq_dsa`/`signed_frame` · `sha3_256`/`MeshEvent`/`commit_after_decide(_drift_gate)` ·
`FileEventStore`+`BlockStore` · `TokenBucket`+`Dispatcher`/`Quirks`/`CachingBackend` · `SandboxTier`
· `matcher.rs` HRW assignment · Hydra breach machinery · `EntropyRng` + QRNG-seeded-never-replace.
**Rejected** (full table with citations: SYNTHESIS §3.5): transparent auctions, self-generated
proofs-of-transition, optimistic execution, finality-tiering machinery, priority-queue kernel
machinery, per-message ZK, BLS aggregation, PQ threshold signing, GG18/GG20 MPC, reputation anywhere.

---

## §2 — The five open questions and their resolutions

**2.1 Market-based micro-negotiation / rapid-fire auctions → rejected as drafted; a five-precondition
sealed-batch form is recorded as dormant law, and no blueprint builds it.** R1 §5 supplied the
structural precedent — Beanstalk (~$182M: transient acquirable weight deciding an outcome) and the
measured steady state of transparent low-latency auctions (72,351 sandwich victims, ~$87.7M in one
half-year) — verdict "rejected-unless-sealed"; R5 §4 supplied the constructive fix (Budish–Cramton–
Shim frequent batch auctions; never raw arrival-time tie-breaks) and R5 §6 the spoofing exclusion
(binding offers, forfeited deposit). SYNTHESIS §2.1 fuses these into five preconditions (default =
no auction, HRW assignment; non-transient capability weight; commit-reveal on the WORM log; batch
window ≫ jitter with hash tie-break; binding deposits) — deliberately **not** blueprinted until a
concrete allocation problem defeats rendezvous/HRW (SYNTHESIS §5). B2 §5 notes its `Settlement` leg
is exactly the deposit mechanism precondition 5 will need, without instantiating any auction.

**2.2 Inline self-auditing → rejected as RC-2 self-certification; replaced by the counterparty-
verified `WorkReceipt`.** Hermetic RC-2 names the shape ("the check reduces to the claim restating
itself"); R3 §3 measures it (MAST: 21.3% of failures are unverified claims; zkML rejected at 1000×+
overhead; TEE = imported hardware trust; "tool receipts, not zero-knowledge proofs" as the pragmatic
middle); R2 §6 (CapTP lineage) and R3 §4 (AP2's Intent→Cart→Payment signed mandate chain, "the single
best pattern in this whole survey") point to the same fix. SYNTHESIS §2.2 defines the receipt —
canonical TLV binding `(revocation_hash, input_cid, output_cid, budget_consumed, nonce, expiry_tick)`,
verified by the **counterparty** on public data via `HybridGate::check` plus `witness_event_id`-style
re-derivation — and B2 §2.1–2.2 carries it to a 169-byte wire schema with an atomic no-partial-accept
verification path. The stated honest limit survives intact: a receipt proves authorized delivery of
specific bytes under a specific grant, never semantic quality.

**2.3 Speculative/optimistic execution with local challenges → rejected outright; verify-before-
persist stands.** R1 §6's record (no permissionless mainnet fraud proof for ~3 years; first ever =
Kroma, April 2024; challenger censorship; "Hollow Victory" economics; optimistic = degrade-open vs
this architecture's degrade-closed posture) and R2 §2's independent rejection (a sparse mesh cannot
meet the honest-watcher liveness assumption) close the mechanism; R4 §5 closes the motivation —
hybrid verification is ~0.1–1 ms/message (~10³ msg/s/core now, ~10⁴ optimized) against 10–100 ms
network RTT, so speculation would dodge a cost 1–2 orders of magnitude below the latency floor.
SYNTHESIS §2.3's resolution: keep `commit_after_decide_drift_gate` (validity-first), build nothing
optimistic; no blueprint contains a challenge window. The carried caveat became B4's whole reason to
exist (the in-repo verify cost is unmeasured — R4 §1 [U]).

**2.4 Eventual consistency vs fast finality → dissolves structurally; the one genuinely new piece is
pairwise atomic settlement.** R1 §2's prevention rule (finality local and explicit: "an event is
final for a participant when they hold the signatures they require") is already this architecture —
local-first logs, legitimate divergence per the SCOPE RULE, no sequencer to decentralize (R2 §4:
"starts from the escape-hatch side"). The single place eventual consistency is unacceptable is R5
§5's: two-party exchange of work for value. SYNTHESIS §2.4 resolves it as a primitive, not a tier —
HTLC delivery-versus-payment with Herlihy's PODC 2018 guarantee and both honest caveats carried
(timelock free option, ~2–3% implicit premium only at hours-scale windows on volatile value;
grief-lock bounded by `2Δ` × B3's per-peer cap) — and B2 §2.3 implements it with per-node local
ticks (Δ = 60 ticks, 2Δ = 120 at the 1 tick/s reference profile) and a written two-party conformance
argument. Settlement finality = both halves in both WORM logs; everything else stays anti-entropy.

**2.5 Priority-tagged dispatcher → no new kernel machinery; priority = envelope selection; exposure
is the one new primitive.** R5 §3's finding (finance contains risk with per-counterparty exposure
limits that heal only on settlement, not a smarter queue), R5 §1's graduated-ladder refinement, and
R3 §5's hierarchical-envelope recommendation ("sophistication is in layering, not a better bucket")
resolve in SYNTHESIS §2.5: a `BTreeMap<(PeerId, CapabilityClass), TokenBucket>` of nested envelopes,
where a wire priority hint is only ever checked against the capability-derived class (a self-assigned
fast lane is RC-2 again). B3 §2.2 implements the two-level check with an aggregate-refusal refund,
and B3 §2.1/§2.3 the exposure ledger with limit-state (enter at 85%, reopen at ≤70% + dwell) — making
the rejected dispatcher "permanently unnecessary" (B3 §5).

---

## §3 — The four blueprints, summarized with pointers

**B1 — `AgentBridge` port + signed `AgentManifest` + admission** (F2/F10/M5/M6/M12). Core decision:
generalize the proven `LlmBackend`/`llm-adapters` seam rather than invent a protocol — the manifest
is a strict canonical-TLV artifact where free-form values are *unrepresentable* (unknown axis or
out-of-domain index fails decode before any gate runs) and the validation-policy floor is unrelaxable
at three layers (no weak code point on the wire, decode error on unknown bytes, admission return
type). Most interesting finding: **MCP's open-world string grammar never enters the manifest** — the
bridge produces a draft, the operator's keys sign a closed-enum scope set plus a `sha3_256` digest of
the tool-allowlist map, so post-admission server drift (registry poisoning, `listChanged`) is
detected as a digest mismatch forcing re-admission (B1 §2.3). Dependencies: none unbuilt ("every
verification primitive it wires exists and is RED-tested," B1 header); B2/B3 consume its admitted
identity. Two consolidation-level flags: B1 introduces Wasmtime without an inline DECART (§5 Q1.4 —
`wasmtime-46.0.1` **is** in the offline registry cache, verified this session, so the W21-class
network block does not apply), and its `Resource::AgentBridge = 0x12` collides with B2 (§5 Q1.1).

**B2 — `WorkReceipt` + `Settlement`, pairwise DvP** (S9/F44/M6/M12, Hermetic P5/P6/P7). Core
decision: the payer holds the hash preimage (verify-then-pay; the free option deliberately sits on
the buyer side), the claim event's own bytes contain the preimage (claiming structurally reveals it),
and timeout sweeps are **sweep-on-commit**, not a timer — riding the only path every event already
takes, with safety never depending on the sweep (a late claim is rejected by `decide`'s pure tick
comparison regardless; B2 §2.5's answer to the Hermetic dead-pendulum finding). The re-verification
catch: **B2 rediscovered the P07 dedup-ordering bug live** — `commit_after_decide` computes the
dedup `event_id()` at `event_log.rs:348` *before* `append` rebinds `prev` to the tip (`:297-300`),
so replaying an event onto a non-empty log re-runs `decide` and double-commits — and named the P07
§2 fix a **hard precondition** (B2 §1.1, migration step 1, acceptance 6). This consolidation
independently re-verified the bug is live and unfixed (§5 Q1.2).

**B3 — `ExposureLedger` + hierarchical envelopes** (R5 §3/§1, R3 §5, Hermetic P4/RC-2). Core
decision: refine the synthesis sketch from `per_peer: BTreeMap<PeerId, Commitment>` to a per-peer
rollup holding an **open set** of commitments (settlement events must match a specific commitment;
expiry is per-commitment), with `try_commit` pure/read-only and `apply` running only after the
durability barrier, so the in-memory ledger is always exactly the fold of the durable log. Burnt
peers are zeroed for *new* stock while in-flight commitments resolve through B2's own claim/refund
legs — force-failing them would confiscate a conforming party's claim, breaking the Herlihy
guarantee (B3 §2.5). The re-verification catch: **`token_bucket.rs` has no `release()` API** — the
live public surface is `new`/`try_acquire`/`available` only (confirmed by this consolidation) — so
the two-level envelope check's aggregate-refusal refund was impossible as designed; B3 §2.2 designs
the addition (`release(n)`, capped at capacity, F33 bound preserved, RED-first as migration step 1).

**B4 — Crypto ground-truth bench + Ed25519 batching + envelope budget** (R4 §1's P0; M6, Hermetic
P1/P6/RC-1). Core decision: replace every literature latency number with a host-fingerprinted,
durably-recorded measurement (`docs/ledger/crypto-bench.jsonl`, mirroring the claim-latency ledger
precedent), after which B1/B2/B3 acceptance criteria cite ledger symbols and the string "0.2–1 ms"
becomes grep-forbidden in criteria. The batching half is classical-leg-only (no PQ analogue exists,
R4 §3) with the SSR-2020 cofactor pitfall pinned: cofactored batch equation, deterministic
SHAKE256-derived coefficients (verification stays RNG-free), single verify remains the sole
acceptance authority. The re-verification catch: **B4 found R4's envelope-tax number incomplete** —
recomputing from source confirms ~3,383 B (≈3.3 KiB) for the hybrid *signatures* (R4's "~3.4 KB"
holds at decimal rounding), but R4 never priced the 1,952-byte raw ML-DSA public key: shipped per
frame, the true wire delta is ≈5,330 B, roughly 2 KB worse. B4 §2.4 pins the repair — after
admission the PQ key is referenced by 32-byte `pq_key_id`, never re-shipped — turning the omission
into a wire-schema test (B4 acceptance 4).

**Why these three catches matter more than trivia:** each is a case where a blueprint author re-read
the live source its own R-doc or synthesis had already cited and found the carried claim wrong or
incomplete — the exact drift class the Detailed Planning Protocol's step 1 and the 2-question ritual
at research stage exist to catch (`AGENTS.md`), and the same class the spectral-evolution arc's
verification pass caught (`append_jsonl` → `append_to`). The discipline is demonstrably firing at the
blueprint stage, not just being cited.

---

## §4 — Sequencing (derived, not assumed)

Cross-checked against each blueprint's own header and migration steps, not the arc's headers. The
stated dependencies:

| BP | Own stated hard deps | Own stated soft/partial deps |
|---|---|---|
| B1 | "nothing already-unbuilt" | step 7's `FUEL_PER_UNIT` "pinned after a B4 bench" |
| B2 | **B1** (agent identity) + **P07 §2 dedup fix** ("hard precondition," §1.1, migration step 1) | step 5: Δ constant "value awaits B4's verify bench" |
| B3 | **B1** (`CapabilityClass`) + **B2** (settlement TLV / expiry_tick) for the ledger half | step 1 (`TokenBucket::release`) is "kernel-local, no dependencies"; steps 2–3 (envelope half) need only B1 |
| B4 | none — "numbers-provider for B1/B2/B3" | steps 4–6 (batching + `ENVELOPE_BATCH_*`) land "where B2's emitter lands" |

The re-derivation **changed the picture in four places** relative to the headers alone: (a) B2
carries a soft B4 dependency its header does not list — its Δ constant explicitly awaits B4's bench;
(b) B3's "landable before B2" half still requires B1 (only its step 1 is truly dependency-free);
(c) B4 is not fully parallel-safe end-to-end — its steps 4–6 interleave with B2's emitter; (d) a
shared integration point **no blueprint names**: B1 pins `Resource::AgentBridge = 0x12` and B2 pins
`Resource::WorkReceipt = 0x12` — both took the next discriminant after the live high-water mark
(`Resource::Migration = 0x11`, `scope.rs`, verified this session), a genuine collision (§5 Q1.1).
One shared discriminant-allocation ruling must precede whichever lands second; B1's Actions are
additionally unnumbered while B2 pins `0x19–0x1E`, so the allocation pass covers both enums. The
MESH-03 wire-stability pin tests will catch a violation mechanically, but the allocation itself is a
lead-agent integration task, not a per-blueprint one.

**Recommendation — B4's measurement half runs FIRST (Wave 0), and this is a real preference, not a
description of tension.** Reasons: it is the smallest unit in the arc (a bench crate + ledger rows,
zero product-code edits); it has zero dependencies; and three separate blueprints hold named
constants symbolic against it (B1 `FUEL_PER_UNIT`, B2 Δ, B3's `try_commit`-overhead criterion).
Running it first converts B4's own migration step 3 — a retroactive editing pass over three *landed*
blueprints — into a plain citation at landing time, eliminating exactly the multi-document
stale-number drift this arc's discipline exists to prevent. The honest counter: B4's numbers change
no *design*, only acceptance-criterion constants, so full parallelism plus the retrofit pass is
legal — but the retrofit pass is pure drift surface and B4-first costs nothing (it is parallel-safe
with B1 anyway). The binding rule either way: **no blueprint's latency-adjacent acceptance criteria
are final until the first ledger rows exist.**

- **Wave 0 (parallel):** P07 §2 dedup fix (tiny kernel change, gates B2; RED = replay-on-non-empty-
  log) · B4 steps 1–3 (bench + ledger + criteria grounding) · B1 steps 1–6 · B3 step 1
  (`TokenBucket::release`) · the one lead-agent act: discriminant allocation for both enums.
- **Wave 1:** B2 (P07 + B1 landed; Δ cites the ledger) · B3 steps 2–3 (envelope half, over B1's
  `CapabilityClass`) · B1 step 7 (Wasmtime fuel loop, `FUEL_PER_UNIT` from the ledger).
- **Wave 2:** B3 steps 4–7 (ledger half, after B2's TLV freezes) · B4 steps 4–6 (batch verify +
  `ENVELOPE_BATCH_*` at B2's emitter).

---

## §5 — The 2-question doubt audit (blueprint-organization stage, per `AGENTS.md`)

**Q1 — what this consolidation is least confident about, cross-checked:**

1. **Cross-blueprint drift — found one real collision.** B1 §2.1 pins `Resource::AgentBridge =
   0x12`; B2 §2.1 pins `Resource::WorkReceipt = 0x12` (and `Settlement = 0x13`). Both correctly cite
   the live high-water mark (`Resource::Migration = 0x11` — re-verified this session against
   `bebop2/proto-cap/src/scope.rs:192`) and independently claimed the next free byte. Neither
   document knows about the other's claim; the synthesis assigns no discriminants, so it cannot
   arbitrate. Investigated to root and carried into §4 as a named Wave-0 allocation act. This is the
   exact "one blueprint's design silently assumes something a sibling contradicts" case the ritual
   names.
2. **The P07-still-open claim — independently verified, not trusted.** Live read:
   `kernel/src/event_log.rs:348` computes `let id = ev.event_id();` before the `append` prev-rebind
   at `:297-300` — the divergence P07 §1.1 describes, byte-for-byte. Branch sweep: exactly two blob
   variants of `event_log.rs` exist across all local and origin branches, both pre-fix, and
   `git grep bind_prev` (the fix's designed primitive) over every branch returns zero hits. **The
   P07 §2 fix is not landed anywhere in this repository as of 2026-07-17; B2's hard precondition
   stands as stated.**
3. **F10 depth default: two documents recommend different values for the same canon anchor.** B1
   §2.2/§2.4 sets `DEFAULT_MAX_AGENT_DEPTH = 3`; `BLUEPRINT-P02-canon-repair-operator-decisions.md`
   O8 recommends a default cap of 8 for F10's sub-hub recursion, and that docket is unruled. Not a
   hard contradiction (O8 was a proposal; B1's is agent-delegation depth), but both cite F10 and a
   reader will find two defaults. Flagged into the CD-3 canon-diff as an explicit operator constant
   rather than silently picking either.
4. **B1 lacks an inline DECART for Wasmtime.** The Detailed Planning Protocol step 3 requires the
   DECART in the planning artifact; B4 wrote one for criterion, B1 did not for Wasmtime (a new
   runtime dependency, even if adapter-side). Partially de-risked by a live probe this session:
   `wasmtime-46.0.1` and its component-model crates are present in the offline
   `~/.cargo/registry` cache, so the wgpu/W21 network-block failure mode does not apply. The DECART
   is still owed before B1 step 7 is called execution-ready; recorded here rather than papered over.
5. **Δ has two floor formulations.** B2 §2.3: "Δ = smallest named constant ≥ 10 × (RTT + verify +
   commit) in ticks"; B4 §2.2's symbolic criterion for B2: "settlement window ≥ 100 × measured gate
   p99." Both are satisfied trivially by the 60-tick reference profile and are not numerically
   contradictory — but two formulas for one constant is the RC-4 unpinned-mirror shape. When Δ lands
   (B2 step 5), it must cite exactly one authority: B2's formula with B4's ledger row supplying the
   verify term.
6. **bebop2 file:line citations were spot-verified, not exhaustively re-read.** This consolidation
   re-verified the load-bearing subset live (`scope.rs` discriminants; `revocation.rs` RevocationSet/
   merge/gossip_payload/drop_anchor; `node_id.rs` from_keys/load_genesis/RootDelegationPolicy;
   `hybrid_gate.rs` verify-then-record H2 fix; `facade.rs` KernelFacade; `token_bucket.rs` API;
   `event_log.rs` commit paths) on the current checkouts (`bebop-repo` is on
   `feat/verification-harness`). The remaining citations are trusted to the blueprints' own same-day
   live re-reads. Routine risk, stated.
7. **External-source numbers are carried under the R-docs' own epistemics, not re-fetched.** R4's
   [M]/[E]/[U] legend and R3's flagged x402 volume inconsistency ($50M cumulative vs ~$600M
   annualized — the R-doc itself refuses to reconcile them) are preserved as-is; no web claim was
   re-verified here.

Items 1 and 2 were the "real risk" bucket and were both investigated to root; the rest are stated
assumptions with named owners.

**Q2 — the biggest things this arc might be missing.** The cross-check found **three R-doc findings
that made it into neither SYNTHESIS §3.5's rejection table nor any blueprint — silently dropped**
(verified by grep over the synthesis + all four blueprints, zero hits each):

- **(a) R1 §1(b)'s Poly-Network invariant — the sharpest drop.** R1's explicit instruction: "*a
  capability must never be able to authorize rewriting the capability-issuance root* … the
  delegation graph must be acyclic with respect to trust-anchor mutation. **Write that as an
  invariant now.**" No document in the arc wrote it. It is structurally mitigated today
  (`verify_chain` is narrow-only; roster mutation is not a capability-reachable path;
  `RootDelegationPolicy` fails closed) — but B1 is precisely the blueprint that adds new
  `(Resource, Action)` scopes, and nothing forbids a future scope pair whose action mutates the
  roster. **Repair applied here:** the invariant is hereby recorded as arc design law — *no
  `(Resource, Action)` scope may authorize mutation of the `AnchorRoster`/genesis or the
  `RevocationSet`-drop path; anchor mutation is an out-of-band operator act, never a
  capability-authorized frame* — and proposed for canon as CD-8 (§7), with a RED-test obligation on
  whichever blueprint first touches `scope.rs`.
- **(b) R2 §5's replication-policy sharpening** ("write the replication policy … into the protocol
  layer as explicit per-object pin/replica counts, not as ops convention" — R2 summary table row 6).
  Not rejected, not deferred-with-name, just absent. Lower stakes (the 3-2-1-1-0 ops doctrine
  exists), but the protocol-layer idea has no carrier. E53-form backlog entry: *what:* per-object
  pin/replica counts in the protocol layer; *why-suspended:* no cross-node availability consumer
  exists until receipts/settlements are exchanged between real peers; *owner:* whoever implements
  MESH-07 Sync·Pull consumption of B2 events; *trigger:* the first exchange where a capability
  references bytes absent from every reachable peer's pin set (R2 §5's own named failure mode);
  *date:* 2026-07-17.
- **(c) R3 §5(2)'s admission-time static loop analysis** (IAL-SCAN-style loop-dependence checking on
  bridged agent graphs). B1's chain-witnessed depth cap bounds *depth*, and `TokenBucket` bounds
  *damage*, but neither detects a loop within the granted depth — R3 §5(3) itself notes the kernel's
  Markov/attractor loop-signal machinery as the runtime complement. E53-form entry: *owner:* B1's
  implementer at the `delegate = true` path; *trigger:* first admitted manifest requesting
  `granted_depth > 1`; *date:* 2026-07-17.

The two directed checks both **pass**: (i) R2 §3's "do not plan on PQ threshold signing today" is
respected everywhere — SYNTHESIS §3.5 rejects/defers it, and B1's policy enum only permits *future
narrowing* variants ("threshold-classical ⊕ single-PQ per R2 §3 — both legs plus more"), i.e.
threshold on the FROST-Ed25519 leg only, exactly R2's recommendation; no blueprint assumes PQ
threshold works. (ii) All five frequent-batch-auction preconditions from SYNTHESIS §2.1 are intact:
no blueprint builds any auction; no allocation mechanism in B1–B4 is first-come-wins (B1 admission
is per-operator, not contested; B3 priority derives only from the verified capability class; B4
batching confers no ordering advantage); and B2 §5 supplies the deposit leg for precondition 5
without instantiating the mechanism. No rejected auction shape re-entered through a side door.

---

## §6 — Anu (logic) & Ananke (organization) check

**Anu — is §4 derived or asserted?** Derived, and the derivation changed the conclusion four ways
(§4: B2's unlisted soft B4-dependency; B3's "pre-B2" half being B1-gated; B4's steps 4–6 coupling to
B2's emitter; the 0x12 collision as an unnamed shared integration point). Two sibling contradictions
were found and named rather than left standing (0x12; F10's 3-vs-8), and the B4-first ordering is
argued from a named cost (the retrofit editing pass as drift surface) with its honest counter stated,
not promoted to a fake hard dependency. The P07 precondition was re-derived from the live file and a
branch sweep, not accepted from B2's assertion.

**Ananke — does this get found, and do the good outcomes fire without being remembered?** Three
structural anchors: (1) **MEMORY.md's active-arcs index needs one line pointing at THIS file** — the
same discoverability fix the spectral-evolution arc named; without it, the arc is findable only by
someone who already knows the directory exists. That line is the parent session's act (memory lives
outside this worktree), named here as the required step, not assumed done. (2) The P07 precondition
is structurally enforced, not remembered: B2's migration step 1 and acceptance criterion 6 ("RED on
pre-P07 code (non-empty log), GREEN after") make landing B2 without the fix a failing test, not a
forgotten checklist item. (3) The two silent-drop repairs in §5 Q2 carry E53-form owners and
falsifiable triggers, so they resurface at a build event (first cross-peer exchange; first
depth > 1 grant) rather than depending on anyone recalling this audit. One protocol deviation,
stated as in the spectral arc: step 7's delete-intermediates is not applied — the four blueprints
are execution artifacts and R1–R5 + SYNTHESIS are the claim-provenance chain; this document is the
single navigable entry point over them, serving step 7's purpose without destroying provenance. The
canon-diff list below is likewise Ananke-shaped: the operator merges §7 or the canon simply never
mentions this arc — there is no third path where canon "eventually" learns of it.

---

## §7 — Proposed canon-diffs + applied mesh-real corrections

### 7.1 Canon-diff proposals (operator merges; per the P02 §0 standing note, this document does NOT
edit `ARCHITECTURE.md` or `STRATEGIC-VECTORS-LOCKED` — "merge, never append" is the operator's act)

**CD-1 — `ARCHITECTURE.md` §8 (honest gaps): add the arc, its precondition, and its entry point.**
NEW line proposed for §8:
```
- Agent Exchange Plane (agentic-mesh 2026-07-17): AgentBridge/WorkReceipt/Settlement/ExposureLedger DESIGNED (4 blueprints), not built. Hard precondition: event_log.rs dedup-ordering fix (P07 §2) — live bug, unfixed on every branch as of 2026-07-17. Entry: docs/design/agentic-mesh-protocol-2026-07-17/AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md.
```
Rationale: canon's §8 is the gap register; a designed-not-built plane with a live-bug precondition is
exactly what belongs there.

**CD-2 — `ARCHITECTURE.md:63` (F2): the bridge-admission mechanism now exists as design.**
BEFORE:
```
- **F2** Hub opens a NEW inbound port for a bridge. SIT: possible. NOW: hub self-authorizes. FUT: port-scan surface grows. PRO: flexibility. CON: attack surface. LOCK + deny-by-default+rate-limit.
```
AFTER (append to the same line):
```
… LOCK + deny-by-default+rate-limit. Mechanism designed: mandatory hybrid-signed AgentManifest admitted via HybridGate + SandboxTier + minted TokenBucket (agentic-mesh B1).
```

**CD-3 — `ARCHITECTURE.md:71` (F10): name the depth mechanism; surface the unresolved default.**
BEFORE:
```
- **F10** Hub delegates to a sub-agent that opens its own sub-hub. SIT: possible (Hydra). NOW: recursion. FUT: depth blowup. PRO: emergent. CON: unbounded. LOCK + max-depth-cap.
```
AFTER:
```
- **F10** … LOCK + max-depth-cap (depth = delegation-chain-witnessed InvokeAgent links, cryptographic not self-reported — agentic-mesh B1. DEFAULT ⚠ one operator constant needed: B1 proposes 3, P02-O8 proposed 8; rule once, pin everywhere).
```
Rationale: two live documents recommend different defaults for the same anchor (§5 Q1.3); canon is
the only place a single ruling stops the fork.

**CD-4 — `ARCHITECTURE.md:100` (F33): "throttles … queue" misstates the shipped semantics.**
BEFORE:
```
- **F33** Hub hits GPU budget. SIT: possible. NOW: TokenBucket throttles. FUT: queue. PRO: cost-bound. CON: slow. LOCK.
```
AFTER:
```
- **F33** Hub hits GPU budget. SIT: possible. NOW: TokenBucket REFUSES (degrade-closed try_acquire→false, typed BudgetExceeded — never a silent queue; token_bucket.rs:46-63). FUT: per-peer envelopes + settlement-healed ExposureLedger (flow ≠ stock — agentic-mesh B3). PRO: cost-bound. CON: refused work must re-request. LOCK.
```
Rationale: live-verified — the primitive refuses; no queue exists. Canon may not carry a semantics
the code contradicts.

**CD-5 — `ARCHITECTURE.md:113` (F44): the escrow substrate is now designed.**
BEFORE:
```
- **F44** Hub disputes an order. SIT: possible. NOW: arbitration via protocol. FUT: resolved. PRO: fair. CON: slow. LOCK + escrow.
```
AFTER (append):
```
… LOCK + escrow (escrow substrate designed: HTLC pairwise DvP settlement, agentic-mesh B2; dispute evidence = both parties' settlement half-logs; arbiter per O3, never reputation).
```

**CD-6 — `ARCHITECTURE.md:81` (F18): "tuned" gets its named constants.**
BEFORE:
```
- **F18** Hub batches 10k frames then flushes. SIT: possible. NOW: latency. FUT: throughput. PRO: eff. CON: lag. LOCK + tuned.
```
AFTER (append):
```
… LOCK + tuned (tuning = named ENVELOPE_BATCH_{MIN_EVENTS,MAX_WAIT_TICKS,MAX_EVENTS} constants, agentic-mesh B4 §2.4; money-scoped frames never batched).
```

**CD-7 — `STRATEGIC-VECTORS-LOCKED-2026-07-16.md:94` (E13-20 cluster): gloss E17/E18/E19 with their
realizations.**
BEFORE:
```
- Agent infra/models (E13-20): self-host llama.cpp/vLLM GOAL; managed-advisory until GPU; harmonic+kelly tiering; spectral+BD memory; MCP; per-agent capability-tokens; TokenBucket; paired-debate.
```
AFTER:
```
- Agent infra/models (E13-20): self-host llama.cpp/vLLM GOAL; managed-advisory until GPU; harmonic+kelly tiering; spectral+BD memory; MCP (E17 — discovery grammar ONLY, behind mandatory mesh-signed AgentManifest, never OAuth/DNS trust: agentic-mesh R3/B1); per-agent capability-tokens (E18 — realized as AgentManifest admission, B1); TokenBucket (E19 — flow bound; stock bound = ExposureLedger, B3); paired-debate.
```

**CD-8 — `ARCHITECTURE.md:21` (M12): write R1's Poly-Network invariant into the capability model.**
BEFORE:
```
- **M12** Capability model (proto-cap, in-repo, 43★ UCAN rejected as heavier): ML-DSA-signed, fail-closed, nonce-replay, expiry, RevocationSet, red-line deny (auth/money/secrets/migrations). Per-agent scope. LOCK.
```
AFTER (append):
```
… Per-agent scope. INVARIANT (R1 §1, Poly-Network lesson): no capability scope may authorize mutation of the capability-issuance root (AnchorRoster/genesis) or the revocation path — anchor mutation is an out-of-band operator act, never a capability-authorized frame; the delegation graph is acyclic w.r.t. trust-anchor mutation. LOCK.
```
Rationale: R1 demanded this be written as an invariant now; the arc dropped it (§5 Q2a). This diff
is the repair.

### 7.2 Mesh-real corrections — applied in-place this session (working docs, not canon)

Both corrections are genuine, live-verified staleness of the "revocation does not exist" family —
the same class as the MESH-09 SQLite correction precedent — not forced connections:

- **`mesh-real/MESH-REAL-PLAN.md` §1** (Capability/authz + Identity rows): the 2026-07-13 STUB
  column claims — "revocation НЕ існує", "genesis-loader-prod-unbuilt", "H2 insert-before-verify",
  "KernelFacade-unbuilt", "node_id … PROPOSED-unbuilt" — are all five now false on the live tree.
  Verified this session: `RevocationSet`/`merge`/`drop_anchor`/`gossip_payload`
  (`bebop2/proto-cap/src/revocation.rs:49,94,105,114`); verify-then-record H2 fix with RED property
  test (`hybrid_gate.rs:188-206,571`); `load_genesis` fail-closed (`node_id.rs:116`);
  `KernelFacade::submit_intent` (`facade.rs:64-96`); `NodeId::from_keys` (`node_id.rs:46`).
  A dated `⚠ CORRECTED` block was added under the table.
- **`mesh-real/BLUEPRINTS-MESH-REAL.md` MESH-11/MESH-12**: the "Мета" premises ("revocation НЕ існує
  сьогодні"; genesis-loader "зараз лише-тести-enroll") are stale for the same reason; a dated
  `⚠ CORRECTED` block was added noting what is now BUILT and what genuinely remains open
  (mesh-wide propagation *guarantees* beyond anti-entropy union; the HUMAN root-delegation-policy
  choice — `RootDelegationPolicy` defaults to `Unspecified` and fails closed until the operator
  rules, so MESH-12's human gate stands).

No other mesh-real claim contradicted this arc; the new event kinds (B2) extend MESH-03's vocabulary
additively, and B2 §5 records that `SettlementRecorded` gains atomic pairing — an extension, not a
correction, so no marker was placed for it.

---

*Consolidation written 2026-07-17 on `feat/agentic-mesh-protocol-2026-07-17`. Sources: the ten arc
documents (all read in full), `AGENTS.md` (2-question ritual, Detailed Planning Protocol, Anu/Ananke
doctrine), `ARCHITECTURE.md` + `STRATEGIC-VECTORS-LOCKED-2026-07-16.md` (canon, read not edited),
`BLUEPRINT-P02-canon-repair-operator-decisions.md` (§1 diff style, O8), `BLUEPRINT-P07-money-law-closure.md`
(§1.1/§2), `mesh-real/` (both docs), `hermetic-architecture-2026-07-16/HERMETIC-ARCHITECTURE-PRINCIPLES.md`
and `spectral-energy-flow-evolution-2026-07-16/SPECTRAL-EVOLUTION-CONSOLIDATED.md` (structural
models). Live probes this session: `kernel/src/event_log.rs:290-361` (P07 bug), all-branch blob sweep
+ `git grep bind_prev` (fix absent), `kernel/src/token_bucket.rs` API surface, `bebop2/proto-cap/`
(scope.rs discriminants, revocation.rs, node_id.rs, hybrid_gate.rs, facade.rs), offline cargo
registry (`wasmtime-46.0.1` present). No code written or edited.*
