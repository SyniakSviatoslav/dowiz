# BLUEPRINT — PHASE 14: DISPUTE / ESCROW + PER-HUB GRAPH-WIKI

> Master plan: `R2-MERGED-PHASE-ROADMAP.md` (this is phase 14 of 19). Canon: `ARCHITECTURE.md`.
> Anchors owned: **E8, E16, E51, F44, F48**.
> Hard dependencies: **Phase 2** (operator DECART rulings **O3** on F44 arbiter, **O4** on F48 merge
> semantics, **O12** on the "BD" expansion — this phase cannot be designed past its interface
> boundaries without them) and **Phase 13** (delivery-on-protocol: escrow reuses Phase 13's ledger;
> disputes reference Phase 13's order/PoD state). Parallel-safe with Phase 15 and Phase 16.
> This is a planning blueprint. It writes **no code** and resolves **no operator decision**. Where a
> ruling is required it stops at a typed interface and presents the candidates neutrally.

---

## 1. Current-state evidence

### 1.1 The F44 spec already exists — design is NOT the gap

A repo-wide `grep -rniI "escrow|arbitrat|dispute" --include=*.rs` over all of dowiz **and** bebop-repo
returns **zero matches** (R1-D §D-3): there is no dispute or escrow implementation anywhere. But the
*design* is not missing. `bebop-repo/docs/design/fable-protocol-2026-07-11/F2-dispute-arbitration.md`
is a full, falsifiable spec. Its **6-state fail-closed machine** (F2 §1):

| State | Entry | Message | Timeout | Success exit | Fail exit |
|---|---|---|---|---|---|
| `OPEN` | party raises dispute on `order_id` | `DisputeOpen{order_id, claimant, respondent, reason}` | — | → `EVIDENCE` | — |
| `EVIDENCE` | both parties submit | `SubmitEvidence{pod_proof?, photo_hash?, geo?, complaint}` | **T_ev = 48h** | → `AUTO_ARBITRATE` | timeout ⇒ bind PoD if present, else → `ESCALATE` |
| `AUTO_ARBITRATE` | advisor proposes | `AutoVerdict{winner, confidence, evidence_refs}` | **T_aa = 10m** | confidence ≥ θ ⇒ `SETTLE` | confidence < θ **or no verdict** ⇒ `ESCALATE` |
| `ESCALATE` | panel empanelled | `Empanel{jurors[]}` | **T_em = 24h** | → `JURY` | empanel fail ⇒ `SETTLE` as refund/hold |
| `JURY` | panel votes | `JuryVote{juror, side, stake}` | **T_j = 72h** | majority ≥ 2/3 ⇒ `SETTLE` | no quorum ⇒ `SETTLE` as refund/hold |
| `SETTLE` | terminal | `Settle{winner, payout, escrow_release}` | — | ledger release | — |

The **single invariant the whole machine exists to protect** (F2 §1 line 35, §4): *any timeout,
missing verdict, or ambiguous majority in `AUTO_ARBITRATE`/`ESCALATE`/`JURY` resolves to `SETTLE` with
**escrow HOLD + default refund to the claimant**, never to silent approval of the respondent.* The
advisor (an L5 neuro-symbolic proposer) has **no authority to move funds**; only `SETTLE` → a ledger
transfer can move money (F2 §4). F2 already carries its own falsifiable **RED test** (F2 §4 lines
78-87) — reproduced verbatim as acceptance criterion AC-1 in §5.

### 1.2 The two canon contradictions inside the F2 spec (stated precisely — NOT resolved here)

The F2 spec predates the mesh pivot and violates two locked laws. Neither is silently resolvable; both
are **operator decision O3** in Phase 2.

- **Contradiction A — jury weighted by `reputation.rs` vs M12 / NO-COURIER-SCORING.** F2 §1 line 37
  maps juror weights onto `reputation.rs::score` and the empanel set onto `guard.rs`'s ≥2/3
  `KillSwitch` supermajority. But reputation/behavioural scoring of participants is **architecturally
  forbidden**: M12 is capability-only; NO-COURIER-SCORING is a *structural* CI gate
  (`scripts/ci-no-courier-scoring.sh`, mirrored in `bebop2/proto-cap/src/matcher.rs`). A panel selected
  or weighted by a stored reputation score violates canon.
- **Contradiction B — UMA / Kleros vs M6 zero-protocol-dependency.** `PROTOCOL-CENTRALIZATION-MAP.md:141`
  says *"Dispute resolution — use UMA/Kleros, don't build."* An external arbitration protocol at the
  trust boundary violates M6 (no external crate/protocol at the wire/trust boundary). Building F2's
  state machine **in-repo** is the canon-consistent path; the UMA/Kleros note predates the pivot.

**Candidate M12-consistent resolutions already named by R1-D** (presented for the operator; **this
blueprint chooses neither**):
1. **Operator-gated arbiter capability** — arbitration is an ML-DSA-signed capability (M12) with
   red-line deny; a single operator-designated arbiter identity ratifies, never a reputation panel.
2. **Schelling-point voting among STAKED capability-holders** — jurors are sampled by *current
   economic stake* (a bonded deposit, slashable on dissent), **not** by reputation history. This is the
   critical M12 distinction: staking is a present economic commitment, not a behavioural record. F2 §3
   already sources the Schelling/bonded-stake math (Kleros whitepaper) and notes *zero of it is
   implemented* — it is applicable theory, absent code.

O3 selects one. **Phase 14's job is to make the interface accommodate either without a redesign** (§2.3).

### 1.3 The escrow primitive already exists as a double-entry ledger

Escrow is **not a new money mechanism** — it is a pair of entries in Phase 13's ledger. The primitive
is `crates/bebop/src/ledger.rs` (ported into the canonical path by Phase 13): `conserved()` asserts
`sum(balances) == 0` (:79-81); `transfer()` is idempotent, fails closed on insufficient funds (:105-107),
and preserves conservation by construction (:89-113). A HOLD is a transfer from the payer account to an
**escrow holding account** inside the same ledger; conservation is untouched. This is the entire basis
of the mid-dispute solvency proof (AC-2, §5).

### 1.4 F48 / E8 / E51 — the knowledge substrate is built; replication is the whole gap

The single-hub knowledge machinery exists and is **green**: `kernel/src/living_knowledge.rs` is the
PRIMARY recall path (recall@5 = 1.0, W18); `spine.rs`, `trigram.rs`, `csr.rs` (deterministic Jacobi
PPR), and `backup.rs` (sha3 content-addressed `BlockStore` + Buzhash-CDC dedup) back it. **Nothing is
replicated**: there is no sync protocol, no per-hub instancing, and no merge semantics anywhere
(R1-D §D-3, R1-E E8/E51). Per canon §8 / F48 this is the **PER-HUB REPLICATED** design (C4 corrected
from single-graph-wiki): each Hydra head keeps its OWN full graph and syncs opportunistically over the
protocol — **there is no central wiki** and this blueprint never designs one.

Two existing artifacts are load-bearing and were verified for this phase:

- **A pull-based sync algorithm already exists** — `bebop2/core/src/anti_entropy.rs`: given two views
  of an append-only hash-chained event log, it computes the divergence point and the exact missing
  suffix (`digest` → `diff` → `SyncPlan` → `apply_pull`). It is *pure, deterministic, std-only, no
  network, no async, and explicitly NOT CRDT-merged* — a fork is **detected and reported** but requires
  out-of-band reset. This is the reusable core of Phase 14's sync **transport** (§3.2), and it is
  ruling-independent.
- **The `crdt-fence` guard's intent is now concrete (critical for O4).** `scripts/ci-crdt-fence.sh`
  (MESH-08) fails the build if any crate that touches `order_machine|money|ledger|claim_machine|
  MeshEvent|assert_transition` depends on a CRDT-merge crate (`automerge`/`cr-sqlite`). Its intent is a
  **periphery fence**: CRDT is forbidden *for money/order state*, not globally. The graph-wiki is
  **knowledge state** and touches none of those symbols — so the fence **neither forbids nor mandates**
  CRDT for the wiki. O4 is therefore genuinely open; the guard removes the assumption that "CRDT is the
  answer" and equally the assumption that it is banned. This is stated so the operator rules on O4 with
  the fence's true scope in hand.

### 1.5 Dependency ledger for this phase

| Needs | From | For |
|---|---|---|
| O3 (arbiter model), O4 (merge policy), O12 ("BD" expansion) | Phase 2 rulings | §2.3, §3.3, §4 |
| Double-entry ledger in canonical path; order/PoD/`DeliveryStatus` fold; `.proto`/tonic wire; signed-envelope framing | Phase 13 | §2.1–2.2 |
| Signed-envelope transport, self-heal/anti-entropy carrier, capability tokens, red-line deny | Phase 9 + Phase 3 | §2.3, §3.2 |
| K/V diff-signer ceremony (for arbiter-capability minting audit) | Phase 6 | §2.3 |

---

## 2. F44 — protocol-message dispute machine + escrow-via-ledger + pluggable arbiter

### 2.1 F2 state machine as protocol messages (not an in-process enum)

Each F2 transition is a **signed envelope** carried over Phase 9's wire — never an internal method call —
so every hub interested in an order can independently verify and fold the dispute, matching the
delivery-domain pattern (`bebop2/delivery-domain` folds `DeliveryStatus` on every hub). Six message
kinds, one per F2 row: `DisputeOpen`, `SubmitEvidence`, `AutoVerdict`, `Empanel`, `JuryVote`, `Settle`.
Design constraints:

- **Envelope discipline.** Every message is an ML-DSA-signed frame admitted through the existing DoD
  gate (`bebop2/mesh-node/src/dod.rs` — payload/id/replay/expiry checks), reusing Phase 13's framing.
  A dispute is keyed by `(order_id, dispute_id)`; the machine folds deterministically from the ordered
  message log, exactly as `assert_transition_local` folds order state.
- **Evidence is content-addressed, not embedded.** `SubmitEvidence` carries *hashes* (`pod_proof`
  references a Phase 13 PoD `DeliveryClaim`; `photo_hash`, `geo`) — the bytes live in the `BlockStore`.
  A PoD reference is verified with the existing `verify_delivery` path (`crates/bebop/src/pod.rs:88-96`):
  it binds `order_id|courier_id|ts|(x,y)` cryptographically and is treated as **prima facie**, contestable
  evidence (F2 §2 is explicit that the signature binds the bytes, not the physical handoff).
- **Timers are protocol facts, not wall-clock trust.** T_ev=48h, T_aa=10m, T_em=24h, T_j=72h are
  evaluated against the order/dispute event timeline; a hub that has not observed the next transition by
  the deadline folds the fail-exit deterministically (per §2.2 this always lands on refund-hold).
- **Fail-closed transitions.** Any unknown message kind, out-of-scope capability, or malformed frame is
  dropped (M12 fail-closed), and any missing/late transition takes the F2 fail-exit — never silent
  respondent approval.

### 2.2 Escrow implemented as paired ledger entries (HOLD / RELEASE)

Escrow reuses Phase 13's double-entry ledger — **it is not a separate money path**:

- **HOLD (on `DisputeOpen`, or at order confirmation for disputable orders).** A single `transfer`
  moves the order value from the payer account to a per-order **escrow holding account**. Because
  `transfer` preserves `conserved()` (`ledger.rs:79-81`), the ledger still sums to exactly zero with
  the HOLD open — this is the mid-dispute solvency guarantee (AC-2). No funds are created or destroyed;
  they are parked.
- **RELEASE (only on `Settle`).** Exactly one `transfer` empties the escrow account: to the courier/
  respondent on a θ-confidence or ≥2/3-majority win, **or back to the claimant on the default-refund
  path.** The arbiter port (§2.3) can *decide a winner* but **cannot call `transfer`** — only the
  deterministic `Settle` handler does, mirroring F2 §4 ("the auto-arbitrator has no authority to release
  funds on its own").
- **Default-refund-on-timeout is a HARD invariant.** Encoded as: the only reachable `Settle` payloads
  are `{winner: respondent}` (requires an explicit θ-confidence verdict or ≥2/3 jury majority) or
  `{winner: claimant, reason: default_refund}` (every other exit — T_ev/T_aa/T_em/T_j timeout, empanel
  fail, no-quorum, low confidence). There is no `Settle` path that increments the respondent balance
  without a positive verdict. AC-1 (the F2 RED test) falsifies any violation.
- **Idempotency & replay.** `transfer` is idempotent by `transfer_id` (`ledger.rs:89-113`), so a replayed
  `Settle` envelope is a clean no-op, not a double-release — critical because the same envelope reaches
  multiple hubs.

### 2.3 The pluggable arbiter interface (accommodates BOTH O3 candidates)

The arbiter is a **PLACEHOLDER port** in this blueprint — a trait, not an implementation. Its concrete
body is selected by O3. The interface is designed so that **either candidate drops in without a
redesign**, and so that **neither can reintroduce reputation** into the trust path.

Interface shape (illustrative, not to be written as code this phase):

- `empanel(dispute_ctx) -> Result<Panel, EmpanelError>` — forms the deciding body. For the
  operator-gated candidate a `Panel` is the single arbiter identity presenting a valid signed **arbiter
  capability** (M12, red-line-deny scoped, minted via Phase 6's K/V ceremony for audit). For the
  Schelling candidate a `Panel` is a sample of **staked** capability-holders.
- `collect_verdict(panel, evidence_bundle) -> VerdictOutcome` where
  `VerdictOutcome ∈ { Decided{winner, confidence}, NoVerdict, EmpanelFailed }`. The deterministic F2
  driver — not the port — maps every non-`Decided`/low-confidence outcome to the default-refund `Settle`.

The two design rules that keep O3 open **and** M12-safe regardless of the choice:

1. **The port's inputs are `(dispute_ctx, evidence_bundle, stake_bonds?)` — never a reputation score.**
   The F2 spec's `reputation.rs::score` coupling is **excised at the interface**: the Schelling
   implementation samples and weights by *current economic stake* (a present, slashable bond), which is
   economic staking, not behavioural history. This is the concrete mechanism by which Contradiction A is
   avoided *without choosing O3* — both candidates consume the same reputation-free interface.
2. **The port cannot move money and cannot self-authorize.** It returns a verdict; the deterministic
   `Settle` handler holds the sole `ledger.transfer` capability, and any arbiter capability is minted
   under M12 red-line-deny (Money scope requires operator signature per Phase 3). A wrong or malicious
   verdict can at worst be *overturnable/appealable within the state machine* — it can never silently
   drain escrow.

Mapping to candidates:
- **O3 = operator-gated capability:** `empanel` returns the operator-designated arbiter; `collect_verdict`
  returns that arbiter's single signed decision; `ESCALATE`/`JURY` collapse to a one-signer panel.
- **O3 = staked Schelling voting:** `empanel` samples staked holders; `collect_verdict` runs the ≥2/3
  Schelling tally with slash-on-dissent; reward paid only on divergence-with-correctness (F2 §3, to keep
  the equilibrium strict and prevent free-riding on the advisor's prior).

Either way the state graph, escrow entries, timeouts, and default-refund invariant are **identical** —
only the `Panel`/verdict body differs. That is the redesign-avoidance property O3 requires.

---

## 3. F48 — per-hub graph instance + delta-sync + pluggable merge policy

### 3.1 Per-hub graph instance (each Hydra head keeps its own full copy)

Each hub owns a complete, independent instance of the knowledge substrate — `living_knowledge` +
`spine` + `trigram` + `csr` index over its own `BlockStore`. There is **no shared/central graph**; a
hub's graph is authoritative *for that hub* and survives the loss of every other hub (the no-SPOF
property, AC-4). The per-hub boundary is the instancing work: give the substrate a hub-scoped root so
two instances can coexist and diverge, and expose export/import at the delta granularity (§3.2). No
recall-path math changes — the recall@5=1.0 substrate is reused as-is.

### 3.2 Delta-sync transport (ruling-INDEPENDENT — build regardless of O4)

Sync is opportunistic, pull-based, peer-to-peer, and **content-addressed**, reusing the algorithm
already built in `anti_entropy.rs`:

1. **Digest exchange.** Two hubs exchange compact per-sequence digests of their graph logs over
   Phase 9 signed envelopes (the `digest` fn). No central node is ever consulted — any two peers can
   sync directly (AC-3's no-central-node requirement).
2. **Diff → pull plan.** Each side computes the divergence point and the exact suffix it is missing
   (`diff` → `SyncPlan`), then requests precisely those blocks.
3. **Content-address dedup on ingest.** Blocks arrive as envelope payloads addressed by sha3; the
   existing `BlockStore` dedup (`backup.rs`, Buzhash-CDC) means identical content is stored once —
   replication of identical knowledge is intrinsically convergent and free of duplication.
4. **Signed, capability-scoped.** Every delta envelope is ML-DSA-signed and DoD-admitted; a hub only
   ingests deltas it is capability-authorized to accept (M12), and a revoked peer's deltas are dropped.

This transport layer does **not** depend on O4: exchanging and deduping content-addressed blocks is
identical whichever merge policy is chosen. What O4 governs is strictly the reconciliation of the small
**mutable pointer layer** (§3.3), not the immutable block exchange.

### 3.3 The pluggable merge-policy interface (accommodates O4)

`anti_entropy` already resolves the common case: *local is a clean prefix of remote* → truncate-and-
reappend the authoritative suffix. The residual case is **true divergence of mutable pointers** — e.g.
two hubs that both advanced "current head of topic T" while offline. This is the *only* thing O4 rules
on, and it is a narrow surface because the immutable content-addressed blocks converge by union
automatically (§3.2). The blueprint stops at a **`MergePolicy` port** — `merge(local, remote) ->
Reconciled` — with two candidate bodies presented neutrally:

- **O4 candidate A — content-address union + total-order LWW on pointers.** Immutable blocks: set-union
  (already convergent). Mutable pointers: last-write-wins by a total order (a Lamport/hybrid-logical
  clock or sequence stamp). Simplest; no new dependency; consistent with the event-sourced,
  never-CRDT-merged stance `anti_entropy.rs` documents for the log.
- **O4 candidate B — CRDT-style convergent merge on the pointer layer.** An add-wins / OR-Set style
  convergent type for the mutable index so concurrent offline edits both survive without a total-order
  tiebreak. **Permitted for the wiki:** the `crdt-fence` (MESH-08) forbids CRDT only in crates touching
  `order_machine|money|ledger|claim_machine|MeshEvent|assert_transition` — the graph-wiki touches none
  of these, so it is outside the fence. (If O4 picks B, the fence's grep list must be verified to still
  exclude the wiki crate so the guard stays green — a mechanical check, not a redesign.)

Both candidates satisfy the AC-3 union-convergence test for immutable content; they differ only in how
concurrent *pointer* edits reconcile. The transport (§3.2) is shared; only the port body changes. That
is the redesign-avoidance property O4 requires.

---

## 4. E16 — the "spectral + BD memory" seam anchor

E16 is the **seam anchor** from the R2 merge (R2 §5): no R1 cluster originally owned it; it was assigned
to Phase 14 because the substrate it names is already built (the spectral/BD organs inside
`living_knowledge`/`spine`/`csr`, recall@5=1.0) and its **only** remaining work is exactly this phase's
per-hub-instancing problem — the same delta-sync + merge-policy machinery designed in §3 replicates the
spectral+BD memory across hubs. E16's consumers are F48 (this phase) and Phase 5's rank folding.

**"BD" is not yet an agreed term and this phase cannot fully close it.** "BD" in "spectral+BD memory"
(E8/E16) has **no authoritative expansion** anywhere in canon; ratifying it is **operator decision
O12** (Phase 2). This blueprint deliberately **does not guess** what "BD" stands for. The per-hub
instance and sync machinery in §3 are agnostic to the expansion — they replicate whatever the
spectral+BD organs already compute — so §3 can be built before O12 lands, but E16 is **not fully
closed** until O12 gives "BD" a definition against which a per-hub-BD acceptance test can be written.
This is stated plainly rather than resolved.

---

## 5. Acceptance criteria (numbered checklist)

Phase 14 is **done** when every item below is green. AC-1 is F2's own already-written RED test, run
verbatim; AC-3 and AC-4 are the two no-SPOF falsifiers.

1. **AC-1 — F2 default-refund RED test, VERBATIM** (F2 §4 lines 78-87). A dispute is opened; **no
   evidence** is submitted by either party; timeouts elapse. Given the dispute in `AUTO_ARBITRATE` with
   escrow held, WHEN the advisor returns `AutoVerdict{winner=respondent, confidence=0.41}` (θ=0.6) **or**
   returns no verdict within T_aa, THEN state MUST transition to `ESCALATE` (never `SETTLE`), AND
   `ledger.balance(respondent)` MUST be UNCHANGED, AND the escrow account MUST remain `== order value`.
   Equivalent tests for `ESCALATE` empanel-fail and `JURY` no-quorum both resolve to
   `SETTLE(refund_to_claimant, escrow_hold)`. **RED** if any respondent balance increases without a
   θ-confidence verdict or ≥2/3 majority — i.e. any silent approval fails the test.
2. **AC-2 — mid-dispute solvency.** With a HOLD open (funds parked in the escrow account and no `Settle`
   yet), `conserved()` MUST return true — the ledger sums to **exactly zero** while the dispute is live.
   After the default refund, it still sums to zero.
3. **AC-3 — offline divergence → union convergence, no central node.** Two hubs diverge while offline
   (each ingests knowledge the other lacks), then reconnect and delta-sync. Both hubs MUST end holding
   the **union** of each other's graph data, and the sync MUST consult **no central node at any point**
   (verified: the exchange is peer-to-peer digest→diff→pull only). Immutable content converges by union
   under either O4 candidate; pointer reconciliation follows the O4 ruling.
4. **AC-4 — kill-a-hub no-SPOF falsifier.** Killing **either** hub entirely leaves the **other** hub
   holding a complete, uninterrupted graph — demonstrating the replication actually removes the single
   point of failure, not just in theory.
5. **AC-5 — arbiter interface accommodates the O3 ruling with no redesign.** Whichever O3 candidate is
   chosen (operator-gated capability OR staked Schelling voting), it drops into the §2.3 port; the state
   graph, escrow entries, timeouts, and default-refund invariant are unchanged. The port's inputs carry
   **no reputation score** (Contradiction A avoided by construction), and the port **cannot** call
   `ledger.transfer` (only `Settle` can).
6. **AC-6 — no external arbitration dependency.** The dispute machine is implemented **in-repo** over
   signed envelopes; there is **no UMA/Kleros or any external protocol at the trust boundary**
   (Contradiction B avoided; M6 upheld). CI dep-audit shows no new external crate on the dispute path.
7. **AC-7 — merge-transport built, O4 port present.** The delta-sync transport (§3.2, reusing the
   `anti_entropy` digest/diff/pull algorithm + `BlockStore` content-address dedup) is built and passes
   AC-3/AC-4 independently of O4; the `MergePolicy` port exists with the chosen O4 body wired in, and if
   O4 = candidate B the `crdt-fence` grep list is verified to still exclude the wiki crate (guard green).
8. **AC-8 — E16/"BD" honestly bounded.** The per-hub spectral+BD memory replicates across hubs via §3;
   the phase records in canon that **E16 is not fully closed until O12 ratifies the "BD" expansion**, at
   which point a per-hub-BD acceptance test is added. No guessed expansion is committed.

**Blocked-until-Phase-2 (recorded, not resolved here):** O3 (arbiter model), O4 (merge policy), O12
("BD"). Phase 14 builds every ruling-independent layer (state machine, escrow-via-ledger, default-refund
invariant, sync transport, both pluggable ports) now, and slots the three rulings into their prepared
interfaces without a redesign when Phase 2 delivers them.

---

*Blueprint P14 complete. Sources read in full: `ARCHITECTURE.md`, `R2-MERGED-PHASE-ROADMAP.md`,
`R1-D §D-3`, `R1-E E8/E51`, and the primary spec `F2-dispute-arbitration.md`. Code grounding verified
this session: `crates/bebop/src/ledger.rs` (escrow primitive), `bebop2/core/src/anti_entropy.rs`
(existing pull-based sync), `scripts/ci-crdt-fence.sh` (MESH-08 fence scope), `kernel/src/{living_knowledge,
spine,trigram,csr,backup}.rs` (built substrate). No code written; O3/O4/O12 presented, not resolved.*
