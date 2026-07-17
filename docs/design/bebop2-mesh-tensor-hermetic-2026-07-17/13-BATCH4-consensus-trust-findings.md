# BATCH 4 — Consensus / Trust / Negotiation cluster: research + audit findings

> Research + audit (NOT a blueprint) for the Bebop2-mesh brainstorm (`00-SOURCE-PROMPT.md`,
> `01-RAW-DIALOGUE-PART-A.md`). This cluster is the highest-risk one because the repo has already
> done real, hard-won, CI-enforced work here. **Method:** every dialogue concept was checked against
> live code and against the sibling agentic-mesh arc that already adjudicated most of it. Verdicts
> are grounded in file:line; epistemics tags below. Nothing here is decided — the one genuine
> architectural contradiction (reputation vs signed-capability) is FLAGGED for operator
> adjudication, not resolved.

## Epistemics tags

- `[VERIFIED-CODE]` — grounded in a file:line I read this session (live working tree).
- `[PRIOR-ART-ADJUDICATED]` — the exact concept was already evaluated + given a verdict in a sibling
  design doc in this repo (agentic-mesh arc / SOVEREIGN-EVENT-EXCHANGE blueprint). Do NOT re-litigate.
- `[THEOREM]` — backed by a published impossibility/optimality result cited in prior art.
- `[CONTRADICTION-FLAG]` — collides with an already-decided, CI-enforced principle. Operator-only.
- `[NEW-ANGLE]` — a genuine extension/angle the prior art did not fully cover; safe to build on.
- `[U]` — unmeasured / open risk carried forward honestly.

---

## §0 — Headline (read this first)

**The dialogue's consensus/trust/negotiation cluster is ~90% a re-paste of questions the
agentic-mesh arc already answered** (`SYNTHESIS-codebase-and-architecture-direction.md §2.1–2.5` and
`§3.5`, landed 2026-07-17). Memory recorded this as "market-negotiation / self-auditing /
optimistic-exec / finality-tiering / priority-dispatcher — all rejected-with-citation or dissolved."
I re-verified each verdict against the cited code and the citations hold. **We must not re-open these
as fresh design questions.** This batch's job is therefore:

1. **Confirm** the prior verdicts still bind (they do — file:line below).
2. **Flag ONE genuine live contradiction** the dialogue reintroduces: **reputation-weighted trust
   matrices + BFT-via-reputation-weighted-majority** directly contradict the repo's CI-enforced
   `NO-COURIER-SCORING` red-line and the Cheng–Friedman impossibility theorem the repo cites to
   justify it. This is the only item that needs operator adjudication, and I do **not** resolve it. §1.
3. **Extend** with the batch-signature honesty lesson (the B4 SSR-2020 fix) which directly refutes
   an unstated assumption in the dialogue's "quorum signature / optimistic PoQ" ideas. §2.2, §2.5.
4. **Give a build-order** for the non-contradicting residue (all small, all composition-over-new-
   machinery). §4.

---

## §1 — THE CONTRADICTION TO ADJUDICATE (flag, do not resolve)

**Dialogue asks for:** "reputation-weighted trust matrices with Sybil-resistance (network-diversity
factor against echo chambers)" and "Byzantine Fault Tolerance via reputation-weighted majority."

**This collides head-on with an already-decided, CI-ENFORCED red-line.** Both sides, verbatim:

### Side A — the repo's standing decision: trust = signed capability, NEVER reputation

- **The one-sentence principle** (`SOVEREIGN-EVENT-EXCHANGE-BLUEPRINT-2026-07-14.md:18-20`):
  > "Sovereignty is achieved by making every unit of state a signed, content-addressed,
  > canonically-encoded event that any peer can independently *verify from first principles* —
  > **never by trusting, ranking, or blacklisting a source.**"
- **The explicit rejection reasoning** (`…BLUEPRINT-2026-07-14.md:22-42`):
  > "The originating idea included a 'truth engine' that marks disagreeing sources as *rotten* …
  > Taken as an **epistemic** mechanism that is a confirmation-bias / echo-chamber machine, and it
  > contradicts this project's own governing rules … We therefore **reject the reputation form** and
  > keep only its sound kernel … (1) Cryptographic provenance … (2) Independent verification …
  > (3) Capability scoping — access is a narrow, expiring, signed grant, **not a trust score.**"
- **It is structurally enforced in code, not just prose:**
  - `claim_machine.rs:13-17` — "Structural constraint enforced here and NOWHERE ELSE:
    **NO-COURIER-SCORING**. The claim state carries no score / rating / trust / reputation / rank
    field." `[VERIFIED-CODE]`
  - `revocation.rs:26-27` — "CI GUARD: NO-COURIER-SCORING — revocation acts on public keys and
    capability hashes (identities / statements), **never on scores or reputation.**" `[VERIFIED-CODE]`
  - The CI gate is real and executable: `/root/bebop-repo/scripts/ci-no-courier-scoring.sh` (2199
    bytes, exec bit set). `[VERIFIED-CODE]` It is wired as a law-hook + CI
    (`…BLUEPRINT-2026-07-14.md:142`, "WIRED by this change … RED-proven (G7 closed 2026-07-14)").
- **It is backed by an impossibility theorem, not a preference** (`R1-web3-failure-modes.md:67-80`):
  - **Cheng & Friedman, 2005** `[THEOREM]`: "**No *symmetric* reputation function — the class that
    includes PageRank/EigenTrust-style scores — can be Sybil-proof.** Every such mechanism admits a
    beneficial Sybil attack. This is a theorem, not an engineering gap: you cannot tune your way out
    of it." (R1 §4). This directly kills the dialogue's "network-diversity factor" mitigation — a
    diversity factor is exactly a symmetric-scoring tweak the theorem covers.
  - Sybil (Douceur 2002), whitewashing (Friedman & Resnick 2001), collusive inflation — all cited as
    the failure classes reputation cannot escape; the only clean escapes named are entry-fees (exclude
    newcomers) or **unforgeable cryptographic identity — which IS capability-based trust**.
  - R2 (`R2-web3-good-patterns.md:251,323`): "reputation systems: scores are sybil-inflatable,
    gameable, and centralizing"; "no reputation-based trust (sybil-gameable)."
  - SYNTHESIS §3.5 final row (`SYNTHESIS…md:352`): "reputation anywhere — **Rejected / deferred** …
    R1 §4 (Cheng–Friedman impossibility — the standing NO-COURIER-SCORING confirmation)."

### Side B — the dialogue's case for reputation-weighted BFT (steel-manned)

- The dialogue's frame is *performance headroom*: at ~2 ms/call it can afford 10–15 negotiation
  rounds, so a weighted-majority BFT vote per state change is now "cheap enough." A reputation weight
  is proposed as the tie-breaker that makes a leaderless mesh converge without a central sequencer.
- The "network-diversity factor against echo chambers" is the dialogue's *own* attempt to
  Sybil-harden the reputation matrix — i.e. the dialogue is aware of the echo-chamber failure and
  proposes diversity-weighting as the fix (this is precisely the class Cheng–Friedman proves cannot
  be made Sybil-proof, so it is a good-faith but theoretically-doomed patch).
- The honest kernel of the dialogue's intent — "weight a claim by something other than raw count" —
  is **already satisfied** by capability weight: `SYNTHESIS §2.1 point 2` uses "participation weight =
  held capabilities under an anchor-rooted delegation chain (`verify_chain`)" as the
  non-transient, non-buyable weight. That is the sound form of "weighted majority" the dialogue is
  reaching for, minus the score.

### What operator adjudication actually decides

This is **not** a 50/50 open architectural question — it is a request to **reverse a CI-enforced
red-line backed by an impossibility theorem.** The decision the operator (and only the operator) can
make is narrow and explicit:

> **Do you want to lift the `NO-COURIER-SCORING` red-line to admit a reputation-weighted trust
> matrix / reputation-weighted BFT majority — knowing (a) it reverses a gate RED-proven closed on
> 2026-07-14, and (b) Cheng–Friedman (2005) proves any symmetric scoring function, including a
> diversity-weighted one, is not Sybil-proof?**

Descartes-square consequence sketch for the operator:
- **If YES (adopt reputation):** gains a leaderless soft-consensus tie-breaker; **costs** = reversal
  of a red-line, a provably-gameable trust surface (Sybil/whitewash/collusion), and it re-imports the
  "echo chamber" failure the blueprint explicitly named. Long-term: a scoring surface tends to
  centralize and to smuggle bias in through "convenience" features (R1 §4(b) warns of exactly this
  drift). **Every downstream consumer must now trust a mutable aggregate instead of a checkable fact.**
- **If NO (keep capability-only):** loses nothing the mesh actually needs (capability weight already
  gives non-transient weighted authorization); keeps the Sybil-immune boolean-grant model; stays
  degrade-closed. Long-term: the mesh's authority stays a held, unforgeable, revocable capability.

**My recommendation as auditor is to surface, not decide.** I flag that the weight of evidence in
this repo is heavily on Side A (a theorem + a CI gate + 3 sibling docs), and that adopting Side B is
a red-line reversal — but per instruction I leave the ruling to the operator. Everything in §2–§4
below is scoped to *exclude* this contradiction, so the rest of the batch is buildable regardless of
how the operator rules.

---

## §2 — Verdict per dialogue concept

### 2.1 Market-based micro-negotiation / bid-based priority scheduling
**Verdict: REJECTED AS DRAFTED; a narrow sealed-batch form is permitted but mostly unnecessary.**
`[PRIOR-ART-ADJUDICATED]` (`SYNTHESIS…md:140-169`, `§3.5:347`)

- Free-form rapid-fire auctions are the Beanstalk (~$182M) structural class — "any mesh mechanism
  where transient, acquirable weight decides an outcome is Beanstalk-shaped" — plus a measured
  steady state of 72,351 sandwich victims / ~$87.7M in one half-year for transparent low-latency
  auctions (`SYNTHESIS…md:142-145`, R1 §5).
- The **default is no auction**: contested work is assigned by the already-built deterministic,
  coordination-free rendezvous/HRW hash (`MESH-05`, `proto-cap/src/matcher.rs`) under capability
  authorization — price discovery is only needed where price genuinely varies, and assignment mostly
  doesn't need it (`SYNTHESIS…md:152-154`). `[VERIFIED-CODE]` `matcher.rs` exists (7170 bytes).
- IF ever built, "safe micro-negotiation" = all five of: no-auction-default, non-transient
  (capability) weight, sealed commit-reveal on the WORM log, batch window ≫ jitter with hash
  tie-break (never arrival time), binding offers with forfeited deposit (`SYNTHESIS…md:150-165`).
- **Extension for this batch:** the dialogue's "bid-based priority scheduling" is the same object as
  its "priority-tagged transitions" — see §2.6; both resolve to *envelope selection*, not an auction.

### 2.2 Proof-of-Quality for gossiped compiled DecisionUnits
Dialogue's four sub-forms + its recommended hybrid (semantic-contract + optimistic fraud-proof):

- **Statistical PoQ** — not adjudicated by name in prior art, but subsumed: a statistical acceptance
  vote over gossiped units IS a reputation/quorum aggregate; it inherits §1's contradiction (weighting
  a claim by how many peers "vote quality") and the Cheng–Friedman problem. **Verdict: falls under the
  §1 flag; do not build absent an operator ruling.** `[CONTRADICTION-FLAG]`
- **Cryptographic-quorum PoQ (aggregate/threshold signatures)** — **REJECTED / deferred.**
  `[PRIOR-ART-ADJUDICATED]` (`SYNTHESIS…md:352`: "per-message ZK, BLS aggregation, PQ threshold
  signing, GG18/GG20 MPC … Rejected/deferred", R2 §2/§7/§3, R4 §3). Hard reason from R4 §3
  (`R4…md:102-118`) `[VERIFIED-CODE]`: **there is no standardized, deployed ML-DSA aggregate or batch
  scheme** — FIPS 204 defines single-signature verify only; the PQ leg must budget one full verify per
  message. BLS aggregation is pairing-based (not PQ) and saves bandwidth, not per-message CPU
  (`R4…md:86-93`). A "quorum signature" over the mesh is therefore not a free primitive on the PQ leg.
- **Semantic-contract PoQ + optimistic fraud-proof (the dialogue's recommended hybrid)** —
  the semantic-contract half is **ACCEPTED in a specific, already-designed form**; the optimistic
  fraud-proof half is **REJECTED** (see §2.5). The accepted form is the **`WorkReceipt`**
  (`SYNTHESIS…md:171-196`) `[PRIOR-ART-ADJUDICATED]`: a canonical-TLV structure binding
  `(capability revocation_hash, input content-id, output content-id, declared budget, nonce,
  expiry-tick)`, carried as a hybrid `SignedFrame` (`RequireBoth`), and **checked by the counterparty**
  through `hybrid_gate.rs::HybridGate::check` (`hybrid_gate.rs:124`, chain → red-line → revocation →
  both sigs → nonce). `[VERIFIED-CODE]` The honest limit (stated, not hidden): a receipt proves
  *authorized delivery of specific bytes under a specific grant and budget* — it does **not** prove
  semantic quality; that judgment stays with the paying party (`SYNTHESIS…md:194-196`).
- **The dialogue's unstated assumption to correct:** it treats "self-auditing during the transition"
  as free because the transition is cheap. Prior art rejects *self-*auditing outright (§2.4) — the
  proof must be verified by a **different** party using only public data (`SYNTHESIS…md:171-182`, R3 §3:
  MAST attributes 21.3% of multi-agent failures to unverified claims, and *no surveyed framework
  verifies agent claims cryptographically*). Self-minted proof-of-transition = the textbook Hermetic
  RC-2 "check restates the claim." `[PRIOR-ART-ADJUDICATED]`

### 2.3 Reputation-weighted trust matrices + Sybil-resistance (network-diversity factor)
**Verdict: BLOCKED on the §1 operator contradiction.** `[CONTRADICTION-FLAG]` `[THEOREM]`
See §1 in full. Cheng–Friedman (2005) proves the whole symmetric-scoring family (which the
diversity-factor variant belongs to) cannot be made Sybil-proof; the repo's `NO-COURIER-SCORING` gate
is the code enforcement of that theorem. Do not build; escalate to operator.

### 2.4 Byzantine Fault Tolerance via reputation-weighted majority
**Verdict: the "reputation-weighted" half is BLOCKED (§1); the "BFT majority" half mostly DISSOLVES.**
`[CONTRADICTION-FLAG]` + `[PRIOR-ART-ADJUDICATED]`
- The reputation weighting is §1's contradiction.
- The BFT-majority machinery itself is largely unnecessary here because the architecture has **no
  global head to agree on**: finality is *local and explicit* — "an event is final for a participant
  when they hold the signatures they require" (`SYNTHESIS…md:221-230`, R1 §2). There is no sequencer
  to decentralize and no global state to reorg; divergence between logs is legitimate (SCOPE
  RULE / M11 fork-freedom). A leaderless BFT vote is solving a problem this mesh deliberately doesn't
  have. `[PRIOR-ART-ADJUDICATED]`
- **The one real "two nodes must agree" case** is value/work exchange, and the answer there is a
  *primitive*, not a consensus tier: HTLC-style delivery-versus-payment (Herlihy PODC 2018 —
  "no conforming party ends up worse off under any deviating coalition"), landing as B2's atomic
  `Settlement` (`SYNTHESIS…md:232-243`). `[PRIOR-ART-ADJUDICATED]`
- **The genuinely mesh-shaped consensus that already exists is NOT BFT voting — it is spectral.**
  `proto-cap/tests/mesh_consensus.rs` (`[VERIFIED-CODE]`, 416 lines) computes Fiedler λ₂ (gossip
  convergence speed) and SLEM/mixing-time τ on the real `AnchorRoster` + delegation topology, with
  hand-derived analytic assertions and a PARITY-GATE cross-check against `bebop2_core::linalg`
  (`mesh_consensus.rs:335-415`). Its own comment: "anchors are identities, **not scores**"
  (`mesh_consensus.rs:210`). This is the mathematically-honest "how fast does the mesh agree" tool —
  and it is capability-grounded, not reputation-grounded. Any dialogue interest in "how the swarm
  converges" should build on this, not on a BFT vote.

### 2.5 Speculative / optimistic consensus with rollback
**Verdict: REJECTED OUTRIGHT; verify-before-persist stands.** `[PRIOR-ART-ADJUDICATED]`
(`SYNTHESIS…md:198-219`, `§3.5:349`)
- R1 §6 record: no permissionless fraud proof on any optimistic-rollup mainnet for ~3 years; first
  ever mainnet fraud proof = Kroma, Apr 2024; documented challenger-censorship + "Hollow Victory"
  economic non-viability; **optimistic execution is *degrade-open* — in direct tension with this
  architecture's degrade-closed posture.**
- The latency case for speculation fails on R4 §5's numbers (`R4…md:143-161`) `[VERIFIED-CODE]`:
  hybrid verify is ~0.1–1 ms/message while mesh RTT is 10–100 ms — verification is 1–2 orders of
  magnitude *below* the latency floor the network already imposes, and dowiz's real event rates are
  orders of magnitude below 1,000 msg/s. Speculating to save a 0.3 ms check while waiting 30 ms for
  the wire imports the whole challenge-window / funded-challenger / censorship-resistance machinery
  R1 §6 shows nobody has made work, to avoid a cost that doesn't matter.
- The kernel already implements the superior, validity-first alternative:
  `kernel/src/event_log.rs:389 commit_after_decide_drift_gate` (verify BEFORE persist), with tests
  `drift_gate_rejects_unstable_in_default_regime` (`event_log.rs:650`) and
  `drift_gate_lifts_on_intervention` (`event_log.rs:677`). `[VERIFIED-CODE]` **Resolution: keep it;
  build nothing optimistic.**
- **Batch-signature honesty extension (directly refutes the dialogue's cheap-rollback assumption).**
  The dialogue assumes optimism/quorum batching is "free." The B4 work proves it is not: commit
  `6541ae8` (bebop `openbebop` remote) `[VERIFIED-CODE]` found that `bebop2/core/src/sign.rs`
  `verify_batch` (added at `sign.rs:971` in that commit) accepted a **mixed-order SSR-2020 forgery**
  `R = R0 + T` that the cofactorless single `verify()` correctly rejects; the small-order filter
  `8·R == O` is blind to it (`8·R = 8·R0 ≠ O`). The fix demotes the cofactored batch equation to a
  fast-reject/accept-*hint* and re-confirms every batch-accept via a full single `verify`, making the
  batch accept-set exactly equal to `verify`'s — which means **batching now costs ≥ N singles
  (measured 3.26× slower on batch/64); no throughput benefit remains.** The arc's earlier "~15-20%
  batch trim" claim was walked back everywhere. **Lesson for any PoQ/quorum/optimistic design:
  aggregating or batching signature checks is NOT a free speedup; correctness forces per-item
  verification, so "cheap rollback / cheap quorum verify" is a false premise.** R4 §3's independent
  literature note names the same pitfall: batch and single verify disagree on mixed-order points
  unless cofactored verification is pinned consistently (`R4…md:95-100`).

### 2.6 Priority-tagged transitions (Critical vs Telemetry) / priority dispatcher
**Verdict: NO new kernel machinery; it is composition over what exists.** `[PRIOR-ART-ADJUDICATED]`
(`SYNTHESIS…md:245-261`, `§3.5:351`)
- Finance's containment is *per-counterparty exposure*, not a smarter queue (R5 §3); sophistication is
  "in layering, not in a better bucket" (R3 §5). Priority = **which bucket a request draws from**: a
  `BTreeMap<(PeerId, CapabilityClass), TokenBucket>` of nested envelopes. A priority flag on the wire
  is an envelope selector, checked against the sender's capability scope — **a peer cannot self-assign
  a priority its capability doesn't grant, else the flag is a self-certified fast lane (RC-2)**.
- The **live analog** the operator pointed to is the B1/B2 discriminant + leg-kind work in
  `dowiz-agentic-mesh`. Two open items sit on it, both operator-facing:
  1. **`0x12 → 0x13` discriminant ruling** `[VERIFIED-CODE]`: the live scope high-water mark is
     `Resource::Migration = 0x11` (`bebop-repo/bebop2/proto-cap/src/scope.rs:192`). B1 pinned
     `Resource::AgentBridge = 0x12` (`BLUEPRINT-B1…md:77`) and **landed it** (memory: B1 AgentBridge
     `f30189262`); B2 *also* pinned `Resource::WorkReceipt = 0x12` and `Settlement = 0x13`
     (`BLUEPRINT-B2…md:105`) — a genuine collision (`CONSOLIDATED…md:241-242,276`). Because B1's `0x12`
     already landed, B2 must shift (`WorkReceipt → 0x13`, `Settlement → 0x14`); the ruling is
     "operator/lead's" and currently only prose (`BLUEPRINT-B1…md:794-796`). This is a *wire-stability*
     decision, not a design one — but it is a hard-fail gate before B2 code lands.
  2. **Budget/money-leg red-line asymmetry** `[VERIFIED-CODE]` — the closest thing to the dialogue's
     "Critical vs Telemetry" priority classes: B2 gives settlement a `leg_kind: Budget | LedgerMoney`,
     arms the red-line gate for the money leg but **not** the budget leg (`COUNSEL…md:54-70`,
     `:231,242-243,296-299`). Counsel's finding: "the mundane framing is enforced for *money legs*
     (red-line + human arming — genuinely good) and **not enforced for budget legs** … If earned
     budget accumulates into a re-spendable balance, you have shipped a currency you are calling a
     budget." The unresolved operator question: **are budget units consumable (spent, gone) or
     transferable-and-accumulable (a balance)?** This is an operator decision, not an auditor's.
- **Note:** money-guard discipline already exists in the engine
  (`dowiz-agentic-mesh/engine/src/money_guard.rs` — money is a discrete integer channel, never
  interpolated; RED-LINE). `[VERIFIED-CODE]` That constrains the presentation layer but does not
  answer the budget-vs-currency question above.

### 2.7 ZK-proof anchoring (EZKL / risc0 / sp1) for succinct proof-of-execution
**Verdict: REJECTED per-message; ACCEPTED only periodic/off-hot-path (checkpoint + light-client).**
`[PRIOR-ART-ADJUDICATED]` (`R4…md:163-183`) `[VERIFIED-CODE]`
- Signature verify proves "signer authorized this message" in ~0.1–1 ms; a validity proof proves "this
  computation ran correctly" but **proving is ~10⁶× native execution** in current zkVMs (a16z 2025;
  arXiv:2512.10020 measured one function at 59 s proving vs 15 µs native ≈ 3.9×10⁷). "**Signature
  verification is the real-time primitive, full stop. No per-message use of a validity proof survives
  contact with the 10⁶× prover overhead.**" (`R4…md:165-173`).
- The only defensible mesh uses are **periodic/batched, off the hot path**: (1) checkpoint/settlement
  proofs ("events N..M applied correctly against the FSM" hourly/daily — audit a checkpoint instead of
  replaying); (2) light-client join (verify one proof instead of replaying the whole sha3 chain)
  (`R4…md:174-180`). `[NEW-ANGLE]` — these two are genuinely valuable *later* and were flagged as such,
  not built; they are the honest home for the dialogue's ZK-anchoring instinct.
- If ever adopted: **STARKs (hash-based) are the PQ-consistent choice**; pairing-based SNARKs
  reintroduce a quantum-vulnerable assumption the hybrid gate exists to avoid (`R4…md:181-183`). Note
  the dialogue's specific tools: EZKL (zkML — the 1000×+/10⁶× class, rejected for per-message),
  risc0/sp1 (zkVMs — same overhead class; risc0/sp1 are STARK-ish so PQ-consistent *if* ever used for
  the checkpoint role, never the hot path).

---

## §3 — Genuine new angles (safe to carry forward)

1. **Batch-verification is not free — make it a stated design axiom.** `[NEW-ANGLE]` The B4 SSR-2020
   fix (§2.5) is a hard, measured refutation of the dialogue's implicit "cheap quorum/optimistic
   verify." Any future PoQ/quorum/finality design must budget **one full verify per item**, both legs,
   and must NOT claim a batching speedup. This is the single most load-bearing correction this batch
   adds to the dialogue.
2. **The mesh's real "consensus math" is spectral, not BFT.** `[NEW-ANGLE]` `mesh_consensus.rs` already
   gives Fiedler λ₂ / SLEM / mixing-time on the *capability* trust graph. The dialogue's "how does the
   swarm converge / detect partition" instinct has a home here (λ₂ = 0 ⇒ partition, fail-closed) that
   is capability-grounded and needs no reputation. Extend this test into an advisory runtime signal
   rather than inventing a vote.
3. **Checkpoint/light-client STARK is the honest ZK home** (§2.7) — off-hot-path, PQ-consistent,
   already flagged as "valuable later" in R4 §6. A candidate for a future wave, not this one.
4. **Semantic-contract PoQ = the `WorkReceipt`** (§2.2) is the constructive form of the dialogue's best
   PoQ idea and is already blueprinted (B2). The batch's contribution is to confirm the semantic-
   contract half is sound *and* to reject the optimistic-fraud-proof half it was paired with.

---

## §4 — Prioritized build-order (EXCLUDES the §1 contradiction)

Ordered small→large, kernel-primitives-first (per operator's "найменші абстракції на рівні ядра …
першими"). Nothing here needs the reputation ruling; all are composition or confirmation, not new
consensus machinery.

| # | Item | Why first / dependency | Type |
|---|---|---|---|
| **B4-C1** | **Write the "batch/quorum verify is not free" axiom into the arc's blueprint set** (cite `6541ae8` SSR-2020 fix + measured 3.26× + R4 §3 no-ML-DSA-aggregate). | Pure doc; blocks any downstream design from re-assuming cheap batching. Zero code. | Confirm |
| **B4-C2** | **P0 criterion bench: measure pure-Rust `pq_dsa` verify p99 on the deployment host.** | Named prerequisite in R4 §5 / SYNTHESIS §2.3 — every "real-time budget" claim is `[U]` until this exists. Kernel-adjacent, small. | Measure |
| **B4-C3** | **Operator ruling: `0x12 → 0x13` discriminant** (B1 `AgentBridge=0x12` landed ⇒ B2 shifts `WorkReceipt→0x13`, `Settlement→0x14`). | Wire-stability gate; hard-fail before B2 code lands (`BLUEPRINT-B1…md:794-796`). Operator/lead act, then a 1-line pin + round-trip test. | Ruling → tiny code |
| **B4-C4** | **Operator ruling: budget-unit semantics** (consumable-not-transferable vs accumulable balance) → then either pin consumable, or extend B2 §2.4 red-line arming to the Budget leg. | Counsel's single most-important safeguard (`COUNSEL…md:298-299`); decides whether the mesh ships "a currency you call a budget." Operator decision, then guard code. | Ruling → guard |
| **B4-C5** | **`WorkReceipt` (semantic-contract PoQ) via existing `HybridGate::check`** — canonical-TLV, `RequireBoth`, counterparty-verified, appended to both WORM logs. | The accepted PoQ form (§2.2); depends on C3 (discriminant) + C2 (budget claim honesty). Reuses `hybrid_gate.rs:124`; no new crypto. | Build |
| **B4-C6** | **Priority = nested `TokenBucket` envelopes** (`BTreeMap<(PeerId, CapabilityClass), TokenBucket>`), priority-flag = envelope selector checked against capability scope. | The non-machinery form of "priority-tagged transitions" (§2.6). Composition over existing `Dispatcher` + `TokenBucket`; no kernel change. | Build |
| **B4-C7** | **Spectral convergence as an advisory runtime signal** — promote `mesh_consensus.rs` λ₂/partition detection from a test into a fail-closed advisory (λ₂→0 ⇒ partition alarm). | Capability-grounded "swarm converges" answer (§3.2); reuses `bebop2_core::linalg`. Advisory only, degrade-closed. | Build (later) |
| **B4-C8** | **(Deferred, future wave) Checkpoint/light-client STARK** for periodic FSM-replay audit. | The only defensible ZK use (§2.7); off-hot-path, PQ-consistent (STARK). Not this wave — flagged so it isn't lost. | Defer |

**Explicitly NOT in build-order (blocked on §1 operator adjudication):** reputation-weighted trust
matrix, network-diversity Sybil factor, statistical-vote PoQ, reputation-weighted BFT majority. These
stay parked until the operator rules on lifting `NO-COURIER-SCORING`.

**Explicitly REJECTED (do not build, prior-art-adjudicated):** free-form rapid-fire auctions,
inline self-generated proof-of-transition, speculative/optimistic execution + local challenges,
per-message ZK/validity proofs, BLS/PQ-threshold/MPC quorum signatures as a per-message primitive.

---

## §5 — Citation index (all verified live this session)

- `bebop-repo/bebop2/proto-cap/src/claim_machine.rs:13-51` — NO-COURIER-SCORING structural constraint; pinned discriminants 0x20-0x23.
- `bebop-repo/bebop2/proto-cap/src/revocation.rs:26-27,105-120` — CI-guard no-scoring; UCAN monotonic revoke + gossip anti-entropy.
- `bebop-repo/bebop2/proto-cap/src/hybrid_gate.rs:124` (+ 43,61,83) — `HybridGate::check`: chain→red-line→revocation→both-sigs→nonce; `RequireBoth`.
- `bebop-repo/bebop2/proto-cap/src/scope.rs:176-216` — Resource discriminant map; high-water = `Migration=0x11`.
- `bebop-repo/bebop2/proto-cap/tests/mesh_consensus.rs:1-17,210,335-415` — spectral (Fiedler λ₂/SLEM) consensus on capability trust graph + core::linalg parity gate.
- `bebop-repo/scripts/ci-no-courier-scoring.sh` — the executable CI red-line gate.
- `bebop-repo/docs/design/SOVEREIGN-EVENT-EXCHANGE-BLUEPRINT-2026-07-14.md:18-42,67,142` — trust=signed-capability-NEVER-reputation; the rejection reasoning.
- `bebop-repo/bebop2/core/src/sign.rs:971` (commit `6541ae8`) — `verify_batch` SSR-2020 mixed-order fix; batching = no throughput benefit (3.26× slower).
- `dowiz/kernel/src/event_log.rs:339,389,650,677` — `commit_after_decide` / `_drift_gate` verify-before-persist.
- `dowiz-agentic-mesh/docs/design/agentic-mesh-protocol-2026-07-17/SYNTHESIS…md:140-261,343-352` — §2.1-2.5 + §3.5 verdict table (the master adjudication).
- `…/R1-web3-failure-modes.md:67-80` — Cheng–Friedman impossibility + Sybil/whitewash/collusion (reputation rejection theorem).
- `…/R2-web3-good-patterns.md:251,316,323` — reputation sybil-gameable; CapTP capability lineage.
- `…/R4-realtime-crypto-verification.md:84-118,143-183` — no ML-DSA aggregate/batch; validity-proof 10⁶× overhead; checkpoint/light-client STARK.
- `…/COUNSEL-ethics-strategy-review.md:54-70,231,242-243,296-299` — budget/money-leg red-line asymmetry.
- `…/BLUEPRINT-B1…md:77,370-378,794-796` & `BLUEPRINT-B2…md:105,412-413` & `CONSOLIDATED…md:241-242,276` — 0x12/0x13 collision + ruling status.
- `dowiz-agentic-mesh/engine/src/money_guard.rs` — money = discrete integer channel, never interpolated (RED-LINE).
</content>
</invoke>
