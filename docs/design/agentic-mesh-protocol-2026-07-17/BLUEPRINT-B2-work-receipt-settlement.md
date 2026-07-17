# BLUEPRINT — B2: `WorkReceipt` + `Settlement` (pairwise delivery-versus-payment)

> Arc: `agentic-mesh-protocol-2026-07-17` · Blueprint 2 of 4 (SYNTHESIS §5 item 2). Anchors:
> **S9** (integer + event-sourcing + saga-compensation, `ARCHITECTURE.md:46`), **F44** (disputes:
> arbitration via protocol, LOCK + escrow, `ARCHITECTURE.md:113`), **M6**, **M12**, Hermetic
> **P5/P6/P7**. **Depends on:** B1 (agent identity — the grant a receipt binds to) and the
> **P07 §2 dedup fix** (hard precondition, §1.1). **NOT on B3** (B3 consumes B2's event shape).
> **Parallel-safe with B4** (which pins the latency numbers left symbolic here).
> **Planning artifact only — no code written or edited.** Two-party settlement only; multi-party
> swaps and disputes beyond the F44 escrow hook are out of scope.

---

## §0 Problem

Inline self-auditing — an agent minting its own proof-of-transition — was rejected as the
textbook RC-2 self-certification shape (Hermetic P7; R3 §3: MAST attributes 21.3% of multi-agent
failures to unverified claims; no surveyed framework verifies claims cryptographically). The
replacement (SYNTHESIS §2.2) is a **counterparty-verified signed receipt**: proof of the
*envelope* — authorized delivery of specific bytes under a specific revocable grant and budget —
checked by a different party on public data ("tool receipts, not ZK", R3 §3; AP2's
signed-mandate-chain, R3 §4). And SYNTHESIS §2.4 found the one place eventual consistency is
unacceptable — two-party exchange of work for value; nothing in the kernel provides atomic
two-party commitment across mutually distrusting nodes (R5 §5). The fix is a primitive, not a
consistency tier: **HTLC-style pairwise DvP** — receipt and payment locked under one hashlock,
expiry-tick refunds.

## §1 Current-state evidence (re-read live, 2026-07-17)

### 1.1 The commit path the new events must fit through — and its live caveat

- `EventLog::commit_after_decide` (`kernel/src/event_log.rs:339-361`): dedup check → `decide`
  BEFORE persist → `append`. Rejection = `CommitError::Rejected` with **nothing persisted**
  (`:353-356`); a durability fault = the distinct `CommitError::Store` pole (`:262-268`).
  `commit_after_decide_drift_gate` (`:389-419`) rejects `Unstable (ρ>1)` pre-persist (DEFAULT
  regime). `MeshEvent::event_id` = `SHA3-256(prev ‖ actor_pubkey ‖ actor_seq ‖ payload)`
  (`:145-155`).
- **Live caveat, verified this session:** `commit_after_decide` still computes the dedup id at
  `:348` *before* `append` binds `prev` to the tip (`:297-300`) — the replay-on-non-empty-log
  double-commit BLUEPRINT-P07 §1.1/§2 documents. A replayed `SettlementClaimed` re-running
  `decide` is unacceptable: **P07 §2's ordering fix is a hard precondition for B2.**

### 1.2 The verification template

`BreachAlert::witness_event_id` (`kernel/src/hydra.rs:135-148`): the receiver **re-derives the
claimed content-id deterministically from the claim's own fields** — fixed-layout bytes, sentinel
actor (`BREACH_WITNESS_ACTOR`, `:126`), `prev = 0`, `seq = 0` — and compares against the claimed
digest, trusting nothing the sender says ("cannot be faked, hidden, or masked"); `from_bytes`
fails closed on any length mismatch (`:107-119`). Re-derive, compare, fail closed — copied below.

### 1.3 The signing substrate

- `SignedFrame` (`/root/bebop-repo/bebop2/proto-cap/src/signed_frame.rs:78-104`): `capability`,
  opaque `payload`, optional `channel_binding`, Ed25519 `classical_sig`, ML-DSA-65 `pq_sig`,
  `delegation_chain`; signatures commit to hand-built domain-separated TLV (`:144-162`) —
  no serde on the signing path.
- House TLV (`proto-cap/src/tlv.rs:21-70`): `DOMAIN_TAG(16) ‖ struct_tag(u8) ‖ wire_version(u8)
  ‖ field_count(u8)`, then per field `FID ‖ u32_le(len) ‖ bytes`; distinct 16-byte domain per
  signed struct. `revocation_hash(cap) = sha3_256(cap.canonical_bytes_tlv())`
  (`revocation.rs:124-131`; reference layout `capability.rs:110-124`).
- `HybridGate::check` (`hybrid_gate.rs:124-209`): freshness → anchor-rooted chain → **armed
  `RedLinePolicy`** (only via `new_redlined`, `:83-99`; deny-by-default) → `RevocationSet` →
  real Ed25519 → real ML-DSA-65 → verify-then-record nonce. `is_red_line` maps
  `(Ledger, Append)`, `(Ledger, SettlementRecorded)`, `(Order, CreateOrder)`, `(Claim, _)` →
  `RedLineCategory::Money` (`redline.rs:73-78`).
- Scope discipline (MESH-03, `BLUEPRINTS-MESH-REAL.md:31-42`): additive, closed-enum, pinned
  discriminants, never renumbered. Live high-water marks: `Resource::Migration = 0x11`,
  `Action::RunMigration = 0x18` (`scope.rs`); `from_discriminant` fails closed.
- Ticks, not clocks: `Capability.expiry` is "a unix-ish monotonically-increasing counter … the
  caller supplies a comparable tick" (`capability.rs:57-59`); `HybridGate::check(now)` models it.

### 1.4 Dispute + scheduler context

F44's design exists: P14 §1.1 carries the 6-state fail-closed dispute machine whose single
invariant is *any timeout/ambiguity → SETTLE with escrow HOLD + default refund to claimant*;
escrow = paired entries in a conserved double-entry ledger (P14 §1.3); arbiter = operator ruling
O3, **never reputation**. Scheduler gap (SYNTHESIS §4.3 P5): no structurally-guaranteed
scheduler exists in-repo; the one precedent, `deploy/deep-clean.timer` (`OnCalendar=*-*-*
04:37:00`, `Persistent=true`), still requires an operator `systemctl enable` host action.

## §2 Target-state design

### 2.1 `WorkReceipt` — canonical-TLV schema + new scope variants

A new proto-cap struct, hand-built TLV in the house layout, new 16-byte domain tag
`DOMAIN_WORK_RECEIPT = *b"bebop2 wrcpt v1\0"`, `struct_tag 0x01`, `wire_version 0x01`,
`field_count 6`, fields in ascending FID order, each `FID ‖ u32_le(len) ‖ bytes`:

| FID | field | bytes | meaning |
|---|---|---|---|
| `0x01` | `revocation_hash` | 32 | `sha3_256(grant.canonical_bytes_tlv())` of the exact capability B worked under — binds the receipt to one specific, surgically revocable grant (`revocation.rs:124-131`) |
| `0x02` | `input_cid` | 32 | `sha3_256(input bytes)` — the request A sent |
| `0x03` | `output_cid` | 32 | `sha3_256(output bytes)` — the result B returned |
| `0x04` | `budget_consumed` | 8 (u64 LE) | declared spend in the grant's denomination |
| `0x05` | `nonce` | 8 | single-use; replay-checked by the verifier's nonce set |
| `0x06` | `expiry_tick` | 8 (u64 LE) | receipt invalid at `now ≥ expiry_tick` (local tick, §2.3) |

Fixed size: 169 bytes. The receipt rides as the `payload` of a `SignedFrame` whose `capability`
is B's grant, hybrid-signed under `RequireBoth`; the domain tag keeps receipt signatures in a
disjoint signing space. `from_bytes` mirrors `BreachAlert::from_bytes`: wrong length, unknown
domain/tag/version, or wrong field count → `None`, fail-closed.

**New scope variants** (additive, pinned, MESH-03 discipline):

- `Resource::WorkReceipt = 0x12`, `Resource::Settlement = 0x13`.
- `Action::ReceiptIssued = 0x19`, `ReceiptAccepted = 0x1A`, `SettlementOffered = 0x1B`,
  `SettlementLocked = 0x1C`, `SettlementClaimed = 0x1D`, `SettlementRefunded = 0x1E`.

Red-line mapping: `(WorkReceipt, _)` is NOT money (no value moves at receipt time). Settlement
money teeth come from a **mandatory co-scope rule** (§2.4).

### 2.2 Counterparty verification path — atomic, no partial accept

When A (who sent the input and received the output) receives B's receipt frame:

1. **Parse fail-closed** (§2.1 `from_bytes` rules).
2. **Re-derive `input_cid′ = sha3_256(input bytes A itself sent)`** — from A's own
   content-addressed copy (BlockStore), never from anything in the frame.
3. **Re-derive `output_cid′ = sha3_256(output bytes A actually received)`.**
4. **Compare both against the receipt's claimed cids.** Any mismatch = reject. This is
   `witness_event_id` applied twice — the anti-self-certification teeth: B cannot assert
   delivery of bytes A doesn't hold.
5. **Re-derive `revocation_hash′`** from A's retained copy of the grant it issued; compare.
6. **`HybridGate::check(frame, now, roster, revocations)`** — chain → red-line (if armed) →
   revocation → Ed25519 → ML-DSA → verify-then-record nonce (`hybrid_gate.rs:124-209`). Steps
   2–5 run first (three hashes are cheaper than two signature verifies — DoS ordering); the
   gate's H2 nonce discipline is untouched.
7. **Envelope checks:** `budget_consumed ≤` the granted envelope; `now < expiry_tick`.
8. **Commit:** steps 1–7 ARE the `decide` closure of `commit_after_decide` — one
   `ReceiptAccepted` event (`payload` = receipt TLV) into A's WORM log.

**Failure semantics: fully atomic reject — no partial-accept state exists.** Any failed step
returns `Err` from `decide` → `CommitError::Rejected` → nothing persisted (`event_log.rs:353-356`);
Law rejects are never retried, `Store` faults are the distinct retry/alarm pole. A rejected
receipt MAY be recorded as a separate `DisputeEvidence`-class event (F44 fodder), but that is a
distinct commit, never a half-accepted receipt.

### 2.3 `Settlement` — HTLC pairwise DvP

**Roles.** A = payer (issues grant, pays budget/money), B = worker (delivers output + receipt).
A is the secret-holder (Herlihy's leader): A picks preimage `s` (32 bytes from `EntropyRng`),
publishes `H = sha3_256(s)`. Assigning the secret to the payer preserves verify-then-pay and
deliberately places the free option on the buyer side (caveat 1).

**What is locked.** A's side: a payment escrow — a signed `SettlementOffered` event committing
budget units (money-scoped: a ledger HOLD per P14 §1.3, a transfer into a per-settlement escrow
account inside the conserved ledger) claimable only on presentation of `s`, refundable at
`offer_tick + 2Δ`. B's side: the `WorkReceipt` — B's `SettlementLocked` event encumbers the
receipt "claimable by A on reveal of `s`, void at `lock_tick + Δ`". An unclaimed receipt is not
a usable authorization artifact: downstream consumers require the matching claim event.

**Settlement TLV** (`DOMAIN_SETTLEMENT = *b"bebop2 setl v1\0\0"`): the offer carries
`hashlock H (32) ‖ payer_key (32) ‖ worker_key (32) ‖ leg_kind (u8: Budget|LedgerMoney) ‖
amount (8, u64 LE — integer only, S9) ‖ grant revocation_hash (32) ‖ delta_ticks Δ (8) ‖
nonce (8)`. `settlement_id = sha3_256(offer TLV)`; Lock/Claim/Refund events reference it.

**Protocol (all events via `commit_after_decide`; every frame hybrid-signed):**

1. A commits + sends `SettlementOffered` (escrow live, expiry `2Δ` on A's local tick).
2. B verifies the offer (`HybridGate`; red-line if money-legged), **only then** performs the
   work, delivers output bytes + receipt frame, commits `SettlementLocked{settlement_id,
   receipt_cid}` (expiry `Δ` on B's local tick).
3. A runs §2.2 verification. To accept, A commits `SettlementClaimed{settlement_id, s}` before
   its local `Δ` deadline and sends it to B. **The claim event's bytes contain `s`** — claiming
   one side structurally reveals the preimage: a claimed receipt is only exhibitable downstream
   via the claim event itself, so suppressing `s` while using the receipt is self-defeating
   (`witness_event_id`'s unforgeable/unhideable logic), and B can also pull the event via
   Sync·Pull anti-entropy (MESH-07).
4. B commits the payment claim — `(Ledger, SettlementRecorded)` for money, else the
   budget-transfer event — citing `settlement_id + s`, before A's `2Δ`. A's `decide` Law on its
   escrow accepts any presentation with `sha3_256(s) = H` before `2Δ`: a pure function of
   committed bytes and tick.
5. Stall ⇒ refund: after `Δ`, B's encumbrance is void and any late claim is rejected by `decide`
   (`now ≥ deadline`); after `2Δ`, A's escrow refunds unconditionally.

**Ticks, not wall-clock (P6).** Each escrow's timeout is enforced by the log holding it against
that node's OWN monotonic tick (the same `now` the facade already feeds `HybridGate::check`):
B enforces `Δ` from its lock tick, A enforces `2Δ` from its offer tick — Herlihy's per-ledger
local timeouts. The margin `2Δ − Δ = Δ` must absorb inter-node tick skew + RTT + verify +
commit; rule: **Δ = smallest named constant ≥ 10 × (RTT + verify + commit) in ticks**, pinned
with the `DT_STABLE` mirror-pin treatment (P3), B4's bench supplying the verify term. Reference
profile: at 1 tick/s and RTT ~100 ms, Δ = 60 ticks, 2Δ = 120.

**Herlihy (PODC 2018) conformance.** Claim: *no conforming party ends up worse off — over the
escrowed artifacts — under any deviating coalition* (two-party case). Argument:

- A offers, B never locks → without a reveal of `s` nobody can claim; A refunds at `2Δ`. Loss =
  time-value of a `2Δ` lock only.
- B locks, A never claims → A holds no claimed receipt (cannot lawfully exhibit the work as
  accepted); B's encumbrance voids at `Δ`; a late claim is tick-rejected. B's escrowed artifact
  returns intact.
- A claims (reveals `s`) → the reveal is in the claim bytes; B presents `s` within the remaining
  `≥ Δ` margin and the payment leg is claimable by construction (`sha3(s) = H`, pure check). A
  cannot hold a claimed receipt while denying B payment.
- No third state exists: each leg is exactly {claimable-with-`s`-before-deadline,
  refundable-after-deadline}, both pure `decide` functions of committed bytes and local tick —
  deviation can delay an outcome to a timeout but cannot take a conforming party's escrow
  without the compensating reveal.

**Honest caveats, with bounds:**

1. **Free option (timelock asymmetry).** A may wait until just before `Δ` and walk away if the
   deal turned unfavorable — a premium-free American option. R5 §5 cites Han, Lin & Yu (AFT
   2019, eprint 2019/896): implicit premium ≈ **2–3% of asset value for volatile assets** over
   hours-scale windows. Here the actual number is set by **the expiry-tick window length Δ ×
   the volatility of the exchanged value over that window**: for task-denominated budget units
   over Δ ≈ 60–120 ticks (~1–2 min) the option value is negligible (≪ 0.1%); it approaches
   2–3% only if Δ stretches to hours on volatile value. Rule: keep Δ minimal per the sizing
   rule; never widen it for convenience.
2. **Grief-lock (capital locked on abort).** A stall locks A's escrow for `2Δ` ticks and
   strands B's sunk compute (work is not escrow-able — the one residual the "escrowed
   artifacts" scoping names honestly). Bounds: per-settlement capital lock ≤ `2Δ` ticks (120 s
   at the reference profile); total griefable exposure ≤ B3's per-peer `ExposureLedger` cap ×
   `2Δ`, with burnt peers zeroed via `ingest_peer_breach`. Timeout = the same `Δ` constant; no
   second timeout system exists.
3. **Information-goods residual.** A receives output bytes before revealing `s`; an aborting A
   keeps bytes it never claimed. The protocol makes the *receipt* unclaimable, not the bytes
   unread. Mitigation is pairwise (exposure cap, stop-trading, F44 dispute with both half-logs
   as evidence) — never reputation.

### 2.4 Commit-path + red-line integration

All five event kinds (`ReceiptAccepted`, `SettlementOffered/Locked/Claimed/Refunded`) commit via
`commit_after_decide` (drift-gate variant where the node routes mutations through it), with
`decide` Laws shaped like `claim_machine` (MESH-04): pure fold, illegal transition rejected,
terminals `{Settled, Refunded}`. Money-scoped settlements (`leg_kind = LedgerMoney`) additionally
require: (a) a gate constructed via **`HybridGate::new_redlined(RequireBoth, RedLinePolicy)`**
(`hybrid_gate.rs:83-99`) — deny-by-default per S9/M12; and (b) the **co-scope rule**: the frame's
capability scope MUST include `(Ledger, Append)` — already `RedLineCategory::Money`
(`redline.rs:75`) — with the `decide` Law refusing any money-legged settlement whose scope lacks
it. A validly-signed money settlement without operator allow-listing is refused twice: by the
gate and by the Law. Amounts are integer minor units only (S9); escrow HOLD/RELEASE reuses
P14 §1.3's conserved double-entry pattern. Disputes escalate per F44 through P14's 6-state
machine — timeout/ambiguity → escrow HOLD + default refund to claimant, arbiter per operator
ruling O3, **never reputation**; B2 contributes the dispute's evidence rows: both parties'
settlement half-logs.

### 2.5 Timeout sweeps — sweep-on-commit (chosen), and why

**Chosen mechanism: sweep-on-commit.** A kernel-internal `SettlementBook` (read projection of
open settlements, min-heap keyed by local expiry tick) exposes `sweep(now)`; the node's single
commit surface (facade `submit_intent` / `Hydra::commit`) calls it with the same `now` it already
threads to `HybridGate::check`, immediately before processing each incoming commit. Each
locally-expired settlement materializes as a `SettlementRefunded` event through
`commit_after_decide` (idempotent — duplicate refund = structural no-op). Cost: O(expired) pops
per commit, amortized O(log n).

**Justification against P5's "structurally inevitable, not remembered":** a systemd timer (the
`deep-clean.timer` precedent) is host configuration requiring an operator `enable` — the exact
remembered-pendulum the Hermetic audit found dead, and a node restored from its event log to a
new host silently loses it. Sweep-on-commit rides the only path every event already takes: any
node alive enough to commit *anything* sweeps. The honest edge — a fully quiescent node
materializes its refund only at its next commit — is safe because **safety never depends on the
sweep**: a late claim is rejected by `decide`'s pure tick comparison whether or not a sweep ever
ran; the refund *right* is a pure function of the committed offer and `now`. The sweep is
bookkeeping liveness (projections, B3 exposure decrement), not correctness. No timer unit is
added for settlements.

## §3 Migration steps

1. Land **P07 §2** (bind-prev-before-dedup) with its replay-on-non-empty-log RED test — hard
   precondition.
2. Additive `Resource`/`Action` variants, pinned `0x12-0x13`/`0x19-0x1E`, round-trip +
   fail-closed-unknown-byte pin tests (MESH-03 discipline); red-line co-scope rule.
3. `proto-cap/src/work_receipt.rs` + `settlement.rs`: TLV structs, new domain tags, fail-closed
   `from_bytes`, TLV stability tests.
4. Kernel `settlement_machine.rs` (sibling of `claim_machine`, MESH-04 shape): decide Laws for
   all five events, tick-pure expiry checks, `SettlementBook` + `sweep(now)`.
5. Wire sweep-on-commit into the facade commit surface; pin Δ as a named mirror-pinned constant
   (value awaits B4's verify bench).
6. RED-first tests per §4; money-legged paths land only under the armed red-line gate.

## §4 Acceptance criteria (falsifiable, numbered)

1. **Mismatched content-id → rejected, never silently accepted.** A receipt whose `input_cid`
   or `output_cid` differs from the counterparty's own re-derivation is rejected atomically:
   `CommitError::Rejected`, log length unchanged, no partial state. RED: flip one output byte —
   the receipt must fail on the receiver, not the sender.
2. **Preimage never revealed → both sides refunded after expiry-tick, provably.** Drive ticks
   past `Δ` then `2Δ` with no claim: B's encumbrance voids, A's escrow refunds; a claim at
   `now ≥ deadline` is rejected by `decide` alone (no sweep run); balances net to pre-offer
   values exactly (S9 integer identity).
3. **Money-scoped settlement without red-line arming → refused.** A `LedgerMoney` settlement
   verified by an unarmed gate, or with a scope lacking `(Ledger, Append)`, is refused
   (`RedLineCategory::Money` / Law reject) even with both signatures valid.
4. **No self-certification path.** Verification requires the verifier's OWN copies of input
   bytes and grant; a node cannot accept a receipt for an input it never sent (the
   re-derivation inputs don't exist locally).
5. **Claim reveals preimage.** The committed `SettlementClaimed` bytes contain `s`;
   `sha3_256(s) = H` checked in `decide`; B's payment claim with `s` before `2Δ` accepted,
   after `2Δ` rejected.
6. **Replay safety.** Re-delivering any of the five events is `AppendOutcome::Duplicate`,
   `decide` run exactly once — RED on pre-P07 code (non-empty log), GREEN after.
7. **Sweep-on-commit fires structurally.** Committing any unrelated event at a node holding an
   expired settlement materializes `SettlementRefunded` in the same call; no settlement timer
   unit exists in the deploy tree.
8. **Forged receipt fails the gate.** No anchor-rooted chain (`UnknownIssuer`), a revoked grant
   (`revocation_hash` in the `RevocationSet`), or a replayed nonce → rejected by
   `HybridGate::check` before any commit.
9. **Pinned wire stability.** New discriminants and both TLV domains round-trip
   byte-identically; unknown discriminant/domain/version decodes to `None`.

## §5 What this unblocks

**B3** — the `ExposureLedger`'s settlement-driven decrement gets its concrete event shape (its
one inter-blueprint dependency). **B1** — bridged agents become payable: manifest-declared cost
denominations settle atomically. **F44/P14** — disputes gain cryptographic evidence rows (both
half-logs); escrow HOLD reuses this pattern. **SYNTHESIS §2.1** — the sealed-batch auction's
binding-offer deposits (forfeit-on-cancel, R5 §6) are exactly a `Settlement` leg. **MESH-03** —
`SettlementRecorded` gains atomic pairing instead of a bare ledger append. And inline
self-auditing is replaced, not merely deleted: every claim of work now has a verifier the
claimant cannot supply (P7).

---

## Extended Context

**Why B2 is the load-bearing trust mechanism of the whole plane.** The Agent Exchange Plane reuses
RED-tested machinery everywhere except one place: R3 §8's finding that *the decentralized
cryptographic trust plane is the only part that must be built*. B2 is that part. It exists to
replace **inline self-auditing**, which was rejected as the textbook RC-2 / Hermetic-P7
self-certification shape: an agent minting its own proof-of-transition is a claim verifying a
claim — the check reduces to the claim restating itself, with no independent second party in the
loop (R3 §3 measures the cost: MAST attributes 21.3% of multi-agent failures to unverified
claims). That rejection matters because it is not stylistic — it is the difference between a trust
plane and a trust-the-text plane. B2's `WorkReceipt` restores the missing second party
structurally: verification is performed by the **counterparty** A, on public data A independently
holds, re-deriving `input_cid′`/`output_cid′`/`revocation_hash′` from its own retained bytes
(`witness_event_id` logic applied twice, §2.2) — a verifier the claimant *cannot supply*. Every
trust claim in the sibling blueprints ultimately grounds out here: B1 authorizes an envelope, B3
meters it, but B2 is the only mechanism that proves a granted agent actually delivered the bytes
it was paid for.

**What breaks or stalls without it.** B3's `ExposureLedger` heals **only on settlement events,
never on a clock** (R5 §3's 15c3-5 stock-vs-flow rule) — settlement events are its *sole* heal
source. Without B2 there are no settlement events, so B3's caps decrement against nothing: they
would fill monotonically and refuse forever, or bound nothing at all. B3 is a meter with no
readings. B1's admitted agents, meanwhile, can *act* — they pass admission, get a `TokenBucket`
envelope, and run — but nothing verifies **what they did**: admission authorizes the door, the
receipt verifies the delivery. Without B2 the plane has gate-in and no proof-of-work-out, which is
exactly the trust-the-text gap MAST measures. B2 is the keystone that makes B1's door and B3's
meter mean anything.

**The real-world shape of the first use case — the mundane pay-peer-for-work case, not trading.**
R5 (mined for finance's risk-*containment* patterns) states its own scope up front (R5 §0): *this
document does not propose a speculative-trading feature.* The target is plain (R5 §1/§9, carried by
SYNTHESIS §2.4): **agent A pays agent B in compute-budget or capability-tokens for completed
work** — a bridged MCP-server agent finishes a task, returns bytes, and is paid a small integer
budget amount atomically against a signed receipt. A reader must not drift into reading
`Settlement`/HTLC/Herlihy as a trading primitive. The HTLC machinery is imported for exactly one
reason: to make pay-for-work atomic across two mutually-distrusting nodes — the single place
eventual consistency is unacceptable (SYNTHESIS §2.4) — **not** to enable price discovery, order
books, or arbitrage. Every one of those market shapes is gated behind separate, unbuilt,
explicitly-flagged preconditions (R1 §5 / §1(b): sealed-bid commit-reveal, non-atomically-
manipulable pricing) that no blueprint in this arc instantiates. B2 is settlement, not a market.

## Definition of Done

This DoD is distinct from and stronger than §4's per-mechanism acceptance criteria: §4 proves each
mechanism *works in isolation*; the DoD gates whether B2 **as a whole** may be declared done. All
nine §4 criteria GREEN is necessary but **not** sufficient.

**Hard gate 1 — the P07 §2 dedup-ordering fix lands FIRST (re-verified unfixed this session).**
Re-verified live 2026-07-17, not trusted from the consolidation: `commit_after_decide`
(`kernel/src/event_log.rs:348`) computes `let id = ev.event_id();` **before** `append`
(`:293-312`) rebinds `ev.prev` to the tip (`:297-301`) and recomputes the id at `:302`. On a
**non-empty** log the dedup id checked at `:350` therefore differs from the id `append` actually
stores, so a replayed event slips the dedup check, re-runs `decide`, and double-commits.
`git grep bind_prev` returns **zero hits across every local and origin branch**; the only P07
commit in history is `aedba0133` (P07 **§6** money tax-overflow, row #11) — **not §2**. The fix is
**still unfixed** as of 2026-07-17; B2's hard precondition stands exactly as §1.1 states it.
- **Stated plainly: B2 cannot be marked done — even if every line of its own code is written,
  reviewed, and its §4 criteria pass in isolation — while this precondition is unmet.** A
  `SettlementClaimed` replayed onto a non-empty log would re-run its `decide` (which verifies the
  hashlock and drives a money/preimage side effect) **twice**. That is a money-law violation, not a
  cosmetic bug; it is disqualifying regardless of B2's own code quality.
- **The CI/test-level gate that enforces it.** §4 acceptance criterion 6 ("Replay safety …
  `decide` run exactly once — RED on pre-P07 code (non-empty log), GREEN after") is the correct
  instrument and it exists. Confirmed sufficient **with one sharpening**: the test MUST (i) commit
  a **`SettlementClaimed`** onto a log already holding ≥ 1 prior event — the *non-empty*
  precondition is load-bearing, because an empty-log test passes even on buggy code (`tip()` is
  `None`, so `prev` stays zero and the two ids coincide); (ii) instrument `decide` with a
  call-counter; (iii) re-deliver the byte-identical event; (iv) assert `AppendOutcome::Duplicate`
  **and** counter == 1 **and** log `len()` unchanged. Against pre-P07 `event_log.rs` this MUST fail
  loudly (counter == 2, `len` grows). Sharpenings over the criterion as written: **pin the test to
  `SettlementClaimed` specifically** (it is the one event whose double-`decide` has an irreversible
  money/preimage effect — a receipt-only replay is comparatively benign, a claim replay is not),
  and additionally assert the **stored** content-id equals the id `append` computes with `prev`
  bound (not the pre-bind id) — this catches a partial fix that reorders the check but mis-keys the
  store. This makes the precondition **structurally enforced, not remembered** (consolidation §6,
  Ananke).

**Hard gate 2 — the Δ formula is resolved to ONE canonical authority NOW (not "later").** The
consolidation §5 Q1.5 flagged the RC-4 unpinned-mirror: B2 §2.3's "Δ = smallest named constant ≥
10 × (RTT + verify + commit) in ticks" vs B4 §2.2's "settlement window ≥ 100 × measured gate p99."
Resolved here, pinned:

> **Canonical Δ:** Δ = the smallest named tick constant satisfying **Δ ≥ 10 × (RTT + verify_p99 +
> commit)**, expressed in ticks, where `verify_p99` is read from B4's `docs/ledger/crypto-bench.jsonl`
> row for the `HybridGate` `RequireBoth` path, and `RTT`/`commit` are the node-profile constants.

B2 §2.3's formula is the **sole authority**; **B4's ledger row supplies the numeric `verify_p99`
input, it is NOT a second formula.** B4 §2.2's "100 × gate p99" is demoted to a
satisfied-by-construction sanity bound, never an independent definition (with `RTT, commit > 0`,
`10 × (RTT + verify + commit) > 10 × verify`, and at the reference profile 60 ticks already
dwarfs `100 × sub-millisecond p99` — both hold trivially). When Δ lands (§3 migration step 5) it
cites **this one line**, not two. This closes Q1.5 at design time, per the task's instruction to
pick one now.

**Remaining DoD gates (comprehensive, beyond §4):**
- **All nine §4 acceptance criteria GREEN** — the per-mechanism unit proof; the DoD subsumes it.
- **Discriminant-collision resolved (Wave-0, lead-agent act).** B2's `Resource::WorkReceipt = 0x12`
  collides with B1's `Resource::AgentBridge = 0x12` (consolidation §5 Q1.1 — both took the next
  byte after the live high-water mark `Resource::Migration = 0x11`). B2 may not land its `scope.rs`
  variants until the arc's discriminant-allocation ruling exists; after it, B2's variants are
  re-pinned to the assigned bytes with MESH-03 round-trip + fail-closed-unknown-byte pin tests
  green (§4 crit 9).
- **Money-red-line double-refusal proven.** Every `LedgerMoney` path lands only under
  `HybridGate::new_redlined(RequireBoth, …)` **and** the co-scope `(Ledger, Append)` Law (§2.4); a
  test proves a validly-signed money settlement without arming is refused twice (§4 crit 3).
- **`SettlementBook` is a pure fold** (see next section): a test reconstructs open/claimed/refunded
  state by folding the log from genesis and byte-compares against the live book — no out-of-band
  state, no persisted heap authority.
- **Conservation.** S9 integer identity: post-refund balances net to pre-offer values exactly (§4
  crit 2); no non-integer money on any path.
- **B1 landed** — the agent identity the receipt's `revocation_hash` binds to must exist; a receipt
  for a grant no admitted agent holds is unverifiable.
- **No new timer unit** in the deploy tree (§4 crit 7) — sweep-on-commit only.
- **Canon note (not a code-done gate):** CD-1/CD-5 (consolidation §7) are operator-merge items;
  B2's landing is what makes CD-5's "escrow substrate designed → built" true, but their merge is
  the operator's act, outside B2's code-done gate.

## Event-Driven Architecture Treatment

B2 is the **most event-native** of the four blueprints and this is made explicit here: it
introduces **no out-of-band state machine**. Every state transition IS a WORM-log event through
`commit_after_decide`; every projection (`SettlementBook` here, `ExposureLedger` in B3) is a
**pure fold** over those events; and the only non-event input anywhere is `now` (the local tick),
which is a query parameter, never persisted state.

**The new event kinds — enumerated against the §2.1 `Action` enum (`0x19–0x1E`).** Six new
discriminants are provisioned; §2.4 canonically names **five committed WORM event kinds**. For
each: trigger · payload · replay-idempotence · what `decide` checks *before* persist.

1. **`Action::ReceiptIssued = 0x19` — action-scope on B's issuing frame (committed-row status:
   under-specified, flagged).** This is the scope-action B's `SignedFrame` carries when B mints and
   signs the 169-byte `WorkReceipt` TLV. §2.4 lists exactly five *committed* kinds and does **not**
   list `ReceiptIssued` among them, so canonically it is the scope-action on the wire, not
   necessarily a row in B's own log. **Honest flag (the one internal under-specification):** the
   blueprint should pin, in `settlement_machine.rs`, whether B *also* commits `ReceiptIssued`
   locally as an idempotent "I issued receipt X" bookkeeping row, or whether `0x19` stays
   scope-only. **Recommended default: scope-only** (keeps the committed set at exactly five, matching
   §2.4). If committed: trigger = B finishes work and mints the receipt; payload = the 169-byte
   receipt TLV; idempotent = re-issue of the identical receipt is a `Duplicate` no-op (content-id =
   `SHA3(prev‖pubkey‖seq‖payload)`); `decide` checks = `from_bytes` fail-closed,
   `budget_consumed ≤` granted envelope, `expiry_tick` not already past B's tick.
2. **`Action::ReceiptAccepted = 0x1A` — committed on A's log (the anti-self-cert event).** Trigger
   = A completes the §2.2 seven-step verification of B's receipt frame. Payload = the receipt TLV
   verbatim (A's log carries the accepted bytes). Idempotent = yes; a re-delivered acceptance of the
   same receipt content-id is a `Duplicate` no-op (post-P07). **`decide` = §2.2 steps 1–7, which ARE
   the `decide` closure:** parse fail-closed → re-derive `input_cid′`/`output_cid′`/
   `revocation_hash′` from **A's own held bytes** and compare → `HybridGate::check` → `budget_consumed
   ≤` envelope → `now < expiry_tick`. Any mismatch → `Err` → nothing persisted. `decide` refuses to
   persist a receipt A cannot independently re-derive — the self-certification teeth live in the
   commit precondition, not a post-hoc audit.
3. **`Action::SettlementOffered = 0x1B` — committed on A's (payer's) log; binds B2↔B3.** Trigger =
   A opens an escrow: picks preimage `s`, publishes `H = sha3_256(s)`, commits the offer. Payload =
   the Settlement offer TLV (`H ‖ payer_key ‖ worker_key ‖ leg_kind ‖ amount ‖ grant
   revocation_hash ‖ Δ ‖ nonce`); `settlement_id = sha3_256(offer TLV)`. Idempotent = yes; identical
   offer bytes ⇒ identical `settlement_id` ⇒ `Duplicate`. **`decide` before persist:** for
   `leg_kind = LedgerMoney`, the armed red-line gate + co-scope `(Ledger, Append)` must be present
   or the offer is refused; `amount` integer `> 0`; **and B3's `try_commit` runs here** — the offer
   is refused pre-persist if it would breach the per-peer or aggregate exposure cap. The escrow is
   never opened if it breaches B3; this is the exact commit-path slot where decide-before-persist
   binds settlement to exposure.
4. **`Action::SettlementLocked = 0x1C` — committed on B's (worker's) log.** Trigger = B has verified
   A's offer, performed the work, delivered output + receipt, and encumbers its receipt. Payload =
   `{settlement_id, receipt_cid}` (lock tick = B's local tick). Idempotent = yes; same
   `{settlement_id, receipt_cid}` ⇒ `Duplicate`. **`decide`:** the offer referenced by
   `settlement_id` is one B verified (`HybridGate` on the offer frame, red-line if money-legged);
   `receipt_cid` matches a receipt B actually issued; B's local tick is inside the offer's window.
5. **`Action::SettlementClaimed = 0x1D` — committed; the hashlock-check-before-persist event.**
   Trigger = A accepts and claims before its local `Δ` deadline. Payload = `{settlement_id, s}` —
   **the preimage `s` is in the event bytes.** Idempotent = yes; same `{settlement_id, s}` ⇒
   `Duplicate`. **`decide` MUST verify the revealed preimage actually unlocks the stated hashlock
   BEFORE persisting:** `sha3_256(s) == H` (H read from the referenced offer) **and** `now < deadline`
   (tick-pure). If `sha3_256(s) ≠ H` → `Err` → nothing persisted — a bogus claim cannot be
   committed. This is the load-bearing decide-before-persist case: the hashlock check is a
   *precondition of the commit*, not an after-the-fact audit. Because the reveal is in the committed
   bytes, claiming structurally publishes `s` (`witness_event_id`'s unforgeable/unhideable logic); B
   pulls it via Sync·Pull anti-entropy (MESH-07) and presents `s` to claim payment before A's `2Δ`.
6. **`Action::SettlementRefunded = 0x1E` — committed on either side; idempotence is critical.**
   Trigger = expiry: B's encumbrance voids at `lock_tick + Δ`, A's escrow refunds at
   `offer_tick + 2Δ` — materialized by sweep-on-commit or an explicit refund commit. Payload =
   `{settlement_id}` (+ which leg). **Idempotent = load-bearing: a duplicate refund is a structural
   no-op (content-id collision).** This is precisely what makes sweep-on-commit safe to fire
   redundantly (§2.5). **`decide`:** `now ≥` the relevant deadline (tick-pure) **and** the settlement
   is not already `Claimed` (terminals `{Settled, Refunded}` — a claimed settlement cannot be
   refunded, a refunded one cannot be re-refunded); a pure-fold guard.

*Payment-leg note (not a new discriminant):* §2.3 step 4's payment claim commits `(Ledger,
SettlementRecorded)` for money (an **existing** scope already mapped to `RedLineCategory::Money`,
`redline.rs:75`) or the budget-transfer event otherwise, citing `settlement_id + s`; B2 gives
`SettlementRecorded` **atomic pairing** (the MESH-03 unblock, §5) rather than a bare ledger append.
Its `decide` re-checks `sha3_256(s) == H` before `2Δ`.

**Does replaying the full event log from genesis reconstruct open/claimed/refunded state?
Confirmed: `SettlementBook` is a pure fold.** Fold the events in log order:
`SettlementOffered` → insert an open entry (keyed by `settlement_id`, indexed in the min-heap by
local expiry tick); `SettlementLocked` → mark the leg locked; `SettlementClaimed` → transition to
`Settled` (terminal); `SettlementRefunded` → transition to `Refunded` (terminal). The **only**
non-event input is `now`, the query parameter to `sweep(now)` — the same `now` already threaded to
`HybridGate::check` — which is **not persisted state**. So `SettlementBook(log)` is a deterministic
function of the log alone, and `sweep(now)` is a deterministic function of `(fold(log), now)`. This
matches B3's `fold_exposure(events) -> ExposureLedger` ("the in-memory ledger is always exactly the
fold of the durable log," B3 §2.2) and the kernel's determinism discipline. The min-heap is a
**performance index, not authority** — derivable from the open set, not persisted; on restore it is
rebuilt by the fold. This is *why* sweep-on-commit (not a timer) is correct: a timer is lost when a
node is restored from its log to a fresh host; the fold is not (§2.5). A DoD test folds the log from
genesis and byte-compares against the live book to confirm no out-of-band state crept in.

## Long-Term Consequences, Safety, Scalability

**(a) Scalability — when does sweep-on-commit get expensive, and is the open set bounded?**
Cost is O(expired) pops per commit, amortized O(log n) with `n` = open settlements (§2.5). The open
set **is bounded, and self-bounding**: opening a settlement requires committing `SettlementOffered`,
and sweep-on-commit runs *immediately before* processing each incoming commit — so every new open
is preceded by a sweep that evicts expired entries; **and** `SettlementOffered`'s `decide` runs
B3's `try_commit`, which refuses the offer if it would breach `default_per_peer_cap` /
`aggregate_cap`. The `(cap+1)`-th open is refused by B3 *before* it can be inserted, so the heap
cannot grow past B3's `aggregate_cap`-worth of commitments (bounded by operator-set caps × active
peers). It does **not** grow unboundedly even on a quiet node: a node that stops calling
`commit_after_decide` runs no sweeps, but it also opens nothing new — the heap is a bounded, static
set during quiescence, not a growing one. **The stalled-sweep liveness consequence:** on a quiet
node, expired settlements sit unswept, so their `SettlementRefunded` events are not materialized and
B3's exposure-decrements are delayed — a peer can appear at/near its cap on settlements that already
expired, causing B3 to **falsely refuse new offers** to that peer. That is a liveness / false-refusal
harm, never a safety one: a late claim is rejected by `decide`'s pure `now ≥ deadline` comparison
whether or not a sweep ever ran (§2.5), and the refund *right* is a pure function of the committed
offer and `now`. **Mitigation (recommended as a DoD-level item):** evaluate B3's outstanding
exposure **lazily** in `try_commit` — count an entry against the cap only if `now < its expiry` —
so an expired-but-unswept settlement stops counting the moment it expires, without waiting for its
refund event. This is consistent with B3 being a pure fold parameterized by `now`, and it makes the
exposure check both safety- and liveness-correct without depending on the sweep. An optional second
lever (opportunistic `sweep(now)` on any idle/heartbeat path) is available but must **not** harden
into a remembered timer (P5); the lazy-expiry fix is the structural one.

**(b) Safety — grief-lock worst case, concrete number.** Caveat 2 (§2.3): a stall locks A's escrow
for `2Δ` ticks and strands B's sunk compute (work is not escrow-able — the honest residual). At the
reference profile (§2.3: 1 tick/s, RTT ~100 ms → **Δ = 60 ticks, 2Δ = 120 ticks**), a malicious
counterparty can lock a victim's committed budget for **at most 2Δ = 120 ticks = 120 seconds = 2
minutes per settlement.** (The victim is A, the payer, whose *escrow* is locked; B's residual is
sunk compute, not escrow.) Aggregate griefable exposure ≤ B3's per-peer `ExposureLedger` cap × `2Δ`,
and a peer that grief-locks repeatedly is zeroed for new stock via `ingest_peer_breach` (`cap → 0`).
**Is 2 minutes acceptable for the mundane pay-peer-for-work case?** Yes for small-value exchange: a
2-minute lock on a sub-cent-to-small integer budget is negligible cost, and the free-option premium
at Δ ≈ 60–120 ticks is ≪ 0.1% (caveat 1). **But the 120 s is an artifact of the coarse 1 tick/s
reference clock, not intrinsic.** The intrinsic floor is `2Δ = 2 × 10 × (RTT + verify + commit) ≈
20 × ~0.11 s ≈ 2.2 s` of real time; the reference profile rounds Δ up to 60 ticks (60 s) for margin.
**Does small-value need a lower default Δ?** The grief-minimizing move is a *shorter* Δ, but Δ is
floored by **safety** (`Δ ≥ 10 × (RTT + verify + commit)` — below it, honest parties miss their
claim window under normal skew + RTT). You therefore cannot make small-value settlements arbitrarily
short by lowering Δ; the correct lever is a **finer tick rate** (each tick shorter wall-clock at a
fixed tick-count safety margin), which shrinks the grief bound toward the ~2 s floor without a
second Δ constant. **Recommendation:** keep the single canonical Δ formula (DoD gate 2); for
small-value / high-frequency exchange run a higher tick rate rather than a special-case shorter Δ —
special-casing Δ by value is a second constant and re-opens the RC-4 mirror.

**(c) Ethics — autonomous cross-operator exchange and the compromised-bridge blast radius.** B2
enables autonomous, unattended value/work exchange between agents controlled by **different
people**. The real failure mode: a B1-bridged agent, compromised or misconfigured, starts emitting
`SettlementOffered` events — spending the operator's budget against another person's agent — that
the human operator never intended. **Does B3's exposure cap bound the blast radius? Yes, and it is
the designed containment.** Every `SettlementOffered`'s `decide` runs B3's `try_commit` *before* the
escrow lands (item 3 above), so a compromised agent can commit at most its per-peer cap against any
one counterparty and the aggregate cap in total outstanding before `try_commit` refuses with
`ExposureError::{PeerCapExceeded, AggregateCapExceeded}` → `CommitError::Rejected`, nothing persisted.
**The actual bound: a compromised agent can commit at most `aggregate_cap` in total outstanding
budget-legged exposure (and at most `default_per_peer_cap` / the peer's `cap` against a single
counterparty) before being refused.** These are **real operator-set config constants** on the
`ExposureLedger` struct (`default_per_peer_cap`, `aggregate_cap`, B3 §2.1), checked pre-persist in
the commit path — "caps are config + capability-derived, never history-scored" (B3 §2.3):
pre-committed and finite, **not unbounded**. For **money-legged** settlements the bound is tighter:
`leg_kind = LedgerMoney` requires the armed red-line gate (`HybridGate::new_redlined`) **and** the
co-scope `(Ledger, Append)` in the frame's capability (§2.4), deny-by-default. A bridged agent whose
operator-signed grant does **not** include the money co-scope can enter **zero** money-legged
settlements — refused twice (gate + Law) regardless of exposure headroom. **Net blast radius:** (i)
≤ `aggregate_cap` of budget-legged exposure (operator-set, finite), and (ii) **exactly 0** money
value unless the operator pre-armed *that specific grant* with the red-line money scope. A
misbehaving peer is zeroed (`cap → 0`) for new stock while its in-flight commitments still resolve
through their own claim/refund legs — never confiscated (Herlihy conformance, B3 §2.5). The one
residual the caps do **not** bound is the information-goods residual (caveat 3): a compromised A can
read output bytes it never pays for, up to `aggregate_cap`-many times before caps + stop-trading +
F44 dispute engage — pairwise-contained, never reputation.

---

*B2 blueprint. Evidence re-read live 2026-07-17: `/root/dowiz-agentic-mesh` (`event_log.rs`,
`hydra.rs`, `deploy/deep-clean.timer`, MESH-03, P07, P14) and `/root/bebop-repo/bebop2/proto-cap`
(`signed_frame.rs`, `capability.rs`, `scope.rs`, `tlv.rs`, `revocation.rs`, `redline.rs`,
`hybrid_gate.rs`). No code written; settlement touches money-red-line paths and earns a careful
separate implementation pass informed by this document.*

---

## Safety Hardening (post-adversarial-review)

> Appended 2026-07-17 in response to `SYSTEM-BREAKER-safety-stress-test.md` **F1** and **F4** and
> `COUNSEL-ethics-strategy-review.md` **§1 item 2 / §8 items 1–2**. These are **additive
> refinements**: nothing in §0–§5 or the four preceding sections is weakened, removed, or
> re-scoped. SH-1 and SH-2 are concrete design fixes that land with B2; SH-3 is a blocking
> **⚠ OPERATOR DECISION REQUIRED** item that gates B2's execution-readiness. SH-1/SH-2 add
> falsifiable acceptance criteria (§SH-4) that extend §4; SH-3 adds a third hard gate to the
> Definition of Done.

### SH-1 — F1 [HIGH] closure: the tick is a protocol constant bound to real time, not a free-running per-node counter

**Root cause restated.** F1 is not an intra-log race (the design wins that — decide-before-persist,
single-writer, dedup-before-decide; SYSTEM-BREAKER credits it). The break is that `Δ` (B's leg) and
`2Δ` (A's leg) are compared *as if they denominate the same wall-clock duration*, while
`capability.rs:57-59` defines the tick only as "a unix-ish monotonically-increasing counter … the
caller supplies a *comparable* tick" — *comparable* asserted, never enforced. Nothing bounds the
**rate**. The concrete break (SYSTEM-BREAKER F1 step 5): A's source at 2 ticks/s, B's at 1 tick/s —
both monotonic, both "valid" — collapses B's `Δ` real-time margin to zero and a conforming B loses
both the work and the payment.

**The fix — redefine the tick, structurally.** Promote the tick from a per-node counter to a
**protocol constant bound to real time**:

> **`TICK_HZ = 1` — one tick equals exactly one second of real time, derived on every conforming
> node from its OWN monotonic clock (`std::time::Instant`), never from a free-running counter and
> never from wall-clock (`SystemTime`).** Each node computes `now` as
> `floor(Instant::now().duration_since(node_local_monotonic_epoch).as_secs())`, where
> `node_local_monotonic_epoch` is an `Instant` captured once at process/log start. `TICK_HZ` is a
> pinned protocol invariant (mirror-pinned per P3, alongside `Δ`), **identical on every node by
> definition**, not a per-node choice.

Because the rate is now fixed protocol-wide, `Δ` ticks means `Δ` seconds and `2Δ` ticks means `2Δ`
seconds **on every node by definition**. The `2Δ − Δ = Δ` margin (§2.3) is now a real wall-clock
quantity, not a quantity denominated in an unbounded unit. The F1 rate-skew attack — two conforming
nodes running at different tick rates — is **eliminated by construction**: a 2-ticks/s node is no
longer "valid under the spec," it is a protocol violation (see residual, below). `Instant` (not
`SystemTime`) is deliberate: it is monotonic, immune to NTP steps / wall-clock jumps / leap seconds,
and never goes backward — the exact substrate a tick-pure `decide` requires.

**Determinism is preserved — ticks stay out of every content-id (Cause-and-Effect).** The fix does
not put any wall-clock reading into a hashed/signed payload:

- **Durations are constants, safe to hash.** The one tick-valued field inside a content-id-bearing
  payload is `delta_ticks Δ` in the Settlement offer TLV (§2.3; `settlement_id = sha3_256(offer
  TLV)`). `Δ` is a **duration constant**, identical on all nodes, not a wall-clock reading —
  hashing it is deterministic and origin-independent. Unchanged.
- **Absolute deadlines never cross the wire.** Every settlement deadline is *already* local-derived:
  A enforces `offer_commit_tick + 2Δ` against A's own clock, B enforces `lock_commit_tick + Δ`
  against B's own clock (§2.3). `offer_tick`/`lock_tick` are **not** fields of any offer/lock/claim/
  refund payload (verify: the offer TLV carries `H ‖ payer_key ‖ worker_key ‖ leg_kind ‖ amount ‖
  grant revocation_hash ‖ Δ ‖ nonce` — no absolute tick; Locked = `{settlement_id, receipt_cid}`;
  Claimed = `{settlement_id, s}`; Refunded = `{settlement_id}`). **No node ever enforces another
  node's leg, and no absolute cross-node tick is ever compared** — precisely why the fix needs no
  synchronization primitive: each leg is checked only against the clock of the node that holds it.
- **The one sender-supplied absolute tick is refined to a relative duration.** WorkReceipt field
  `0x06` was an absolute `expiry_tick` that B writes against B's clock and A checks against A's clock
  (§2.2 step 7; §2.1). Even at a fixed *rate*, two nodes' monotonic epochs have **different
  origins** (`Instant`'s zero is per-process), so an absolute value from B is meaningless to A.
  **Refinement: field `0x06` becomes a RELATIVE `validity_ticks` (a window length, u64 LE, 8 bytes —
  wire layout and the 169-byte receipt size unchanged).** The receiver re-derives the absolute
  deadline locally: `receipt_deadline = t_admit_local + validity_ticks`, where `t_admit_local` is the
  receiver's OWN tick when it first admits the receipt frame, and checks `now < receipt_deadline`.
  The receiver **never trusts B's number as an absolute deadline** — it re-derives against its own
  clock at check time. Being a duration, `validity_ticks` is origin-independent and safe inside the
  receipt content-id. Event `0x19` (ReceiptIssued) and `0x1A` (ReceiptAccepted) `decide` checks read
  `validity_ticks` the same relative way against each committer's own tick.

Net: **no absolute wall-clock or cross-node tick value enters any hashed/signed content-id**; only
durations (constants / relative windows) do. Cause-and-Effect determinism holds.

**Rejected alternative — network time sync (NTP/epoch anchor).** A shared clock (NTP, a signed
time-beacon, a consensus epoch) would let nodes compare absolute ticks directly. **Rejected**, on
this session's own established doctrine (the real-time-crypto research): network calls are unsuitable
for anything safety-critical or on the hot path. A time-sync dependency would (i) put a network
round-trip on the commit path that `decide` must stay pure of, (ii) introduce a *trusted third
party* — the time source — that a fork-free, quorum-free mesh (SYNTHESIS §2.4) has no basis to trust
and no way to authenticate without re-importing the consensus it deliberately lacks, and (iii) make
settlement liveness **fail under network partition**, the exact fragility every other leg of this
design avoids by being tick-pure and local. The local-monotonic-clock-as-protocol-constant fix
closes F1 with **zero** new network dependency and **zero** new trusted party — it is strictly
cheaper and strictly more robust than time sync.

**Residual risk — a deliberately miscalibrated local clock — named honestly.** The fix converts
rate-skew from *spec-permitted* to *non-conforming*, but it cannot physically force a node's crystal
to keep true seconds. Two sub-cases:

- **Honest drift is negligible.** Commodity oscillators drift ≤ ~50–100 ppm. Over a `2Δ = 120 s`
  reference window, 100 ppm ≈ **12 ms** of error against a `Δ = 60 s` margin — smaller by ~3.5
  orders of magnitude. The F1 break required a **100 % rate difference (a 2× clock = 1 000 000 ppm)**;
  honest hardware is nowhere near it. The gap is closed for all conforming hardware.
- **A deliberately fast clock remains a bounded, detectable attack, not a silent one.** The
  dangerous direction is a **payer (A) running its clock fast** to make its `2Δ` refund fire early in
  real time and strand a conforming worker B (the F1 loss). This is *not* eliminated — it is
  (1) **a protocol violation** (a conforming node runs `TICK_HZ = 1`), (2) **blast-radius bounded**
  by B3's exposure caps exactly as every other misbehavior is, and (3) **detectable locally with no
  network sync**: because each node times events on its **own** monotonic clock, a victim/observer V
  can measure, against V's own clock, how long peer P's settlements actually stay open before P
  refunds; a peer whose settlements consistently close in far less than `2Δ` real seconds *by V's own
  clock* is running a fast clock. To actually beat a conforming B's `RTT + verify + commit` (≈ `Δ/10`)
  the attacker must run its clock **> ~10× fast** — a gross anomaly obvious on the first few
  interactions. This detector is **advisory, non-blocking** (blocking on it would reintroduce the
  cross-node coupling NTP was rejected for): it feeds monitoring and can inform **pairwise
  stop-trading** (never a gossiped/aggregated reputation score — consistent with the arc's
  "never reputation" stance, COUNSEL §5/§6), and is corroborated by cross-checking a peer's implied
  cadence against the **ensemble median** across peers. Honest statement: SH-1 makes rate-skew a
  detectable, exposure-bounded protocol violation instead of a silently-conforming theft; it does not
  make a lying oscillator physically impossible.

**Reconciliation with §"Long-Term Consequences" (b).** That section recommends running a **finer
tick rate** (not a shorter `Δ`) for small-value / high-frequency exchange. Under SH-1 the tick rate
is a protocol constant, so that recommendation stands **only as a protocol-wide constant change** —
if the mesh ever adopts a finer `TICK_HZ`, **all nodes adopt it together**; it is never a per-node
knob, because a per-node rate is exactly the F1 hole. This hardens (b)'s precondition; it does not
remove the lever.

### SH-2 — F4 [HIGH] closure: a per-peer concurrent-settlement COUNT cap (B2's requirement on B3)

**Root cause restated.** B3 caps settlement **value** (`peer.outstanding ≤ cap`,
`aggregate_outstanding ≤ aggregate_cap`) but never **count** — `open: BTreeMap<[u8;32], Commitment>`
sums values, not entries (SYSTEM-BREAKER F4; B3 §2.1). So one counterparty can open **many** small
settlements to drive `aggregate_outstanding` toward the 85 % `HIGH_WATER = 17/20` trip, flipping the
node to `LimitState` where `try_commit` refuses **every** new commitment from **every** peer with
`Paused` — a single griefer converts the LULD friction-brake into a **node-wide transaction freeze**,
renewable for as long as the attacker keeps topping exposure above 70 %. The secondary amplifier is a
bloated `SettlementBook` min-heap and a burst of `SettlementRefunded` folds near a common expiry.

**The fix — cap the count, at the same pre-persist gate as value.** Add a **per-peer
`MAX_CONCURRENT_SETTLEMENTS`** cap (a *count* of open, non-expired commitments), checked in the
**same `try_commit` slot** where the value caps are checked — i.e. inside
`SettlementOffered.decide` (Event-Driven Architecture item 3), pre-persist, decide-before-persist. A
`SettlementOffered` that would make the peer's open-count exceed the cap is refused with a new
`ExposureError::PeerConcurrencyExceeded` → `CommitError::Rejected`, **nothing persisted** — identical
failure semantics to `PeerCapExceeded` / `AggregateCapExceeded`. The offer never lands, so the heap
never grows past `MAX_CONCURRENT_SETTLEMENTS × active-peers`, and the "many tiny settlements" burst
vector is blunted for a single identity.

**This is properly a B3 concern — stated as B2's REQUIREMENT on B3, not smuggled into B2's data
model.** The open-commitment set and its caps live in B3's `ExposureLedger` (`fold_exposure`, §2.1);
B2 does **not** own that state and must not shadow it. Explicitly:

> **B3 MUST add a per-peer concurrent-commitment COUNT cap** — a `max_concurrent_per_peer`
> field on `ExposureLedger` alongside `default_per_peer_cap` / `aggregate_cap`, incremented on each
> open (`SettlementOffered`) and decremented on each terminal (`SettlementClaimed` / `SettlementRefunded`),
> tracked as a *count* not a value, and enforced in `try_commit` at the same pre-persist point as the
> value caps, returning `PeerConcurrencyExceeded`. **B2 depends on this**: without it B2 cannot bound
> F4's count-grief, because B2's `SettlementOffered.decide` calls B3's `try_commit` and has no other
> lawful place to reject a count-flood. This is a hard inter-blueprint dependency, the mirror of §5's
> "B3 consumes B2's event shape" running the other way.

**Consistency with lazy-expiry (§"Long-Term Consequences" (a)).** The count is evaluated **lazily**,
exactly as that section prescribes for value: a commitment counts toward `max_concurrent_per_peer`
**only while `now < its expiry`**. An expired-but-unswept settlement stops counting the instant it
expires, so a stalled sweep can never *falsely* block an honest peer on stale concurrency. Count and
value use the same `now`-parameterized fold — no new state authority, no persisted counter (the count
is `open.values().filter(|c| c.peer == p && now < c.expiry).count()`, a pure fold).

**Default and rationale — `MAX_CONCURRENT_SETTLEMENTS = 16` per peer (operator-tunable).** Tie it to
a small multiple of the honest steady-state concurrency. Honest settlements do **not** live for the
full `2Δ` — that is the abort/stall worst case; in the honest flow A claims promptly after verifying
and B collects promptly (§2.3 steps 3–4), so an honest settlement closes in ≈ `RTT + verify + commit`
≈ `Δ/10` ≈ 6 s at the reference profile. A genuinely busy honest peer sustaining even ~1 new
settlement/second (already high for discrete agent tasks on a sparse mesh, R2 §2) carries
≈ `rate × lifetime` ≈ 6 concurrent in steady state. **16 gives ≈ 2.5× headroom over that** —
comfortably above honest need, yet small enough that a single griefing identity is capped at 16 heap
entries and 16 units of the burst pattern, far below the count needed to bloat `SettlementBook` or
drive the burst-refund secondary amplifier. It composes with F3: even under Sybil fragmentation, a
node's total concurrent open-count is `min(16 × identities, aggregate headroom)`, and the identity
count is F3's domain (per-`NodeId` enrollment), out of scope here. `16` is a starting default on the
same footing as B3's other caps — config, never history-derived (B3 §2.3, §5) — to be tuned per
deployment against observed honest concurrency.

### SH-3 — ⚠ OPERATOR DECISION REQUIRED: budget-leg value asymmetry (COUNSEL §1 item 2 / §8 item 1)

**The gap (not silently resolved here).** §2.4 arms the red-line gate + operator allow-listing +
`(Ledger, Append)` co-scope **only for `leg_kind = LedgerMoney`**. The **`Budget` leg carries no
red-line** (§2.4; `redline.rs:73-78` maps money scopes only). COUNSEL §1.2 names the sharp
consequence: *nothing structurally ties a budget unit to real compute — it is an integer that moves
on a signed event.* **If budget units are transferable-and-accumulable** (B earns units from A, then
spends them on C — the §2.3-step-4 "budget-transfer event" reads like this), **they are a de-facto,
ungated internal currency operating entirely outside money-law** — exactly the R1 §7 resource-token
reflexivity risk, and "not a trading platform" ceases to be *structurally* enforced (it stays
enforced for money legs, and only for money legs). The whole question turns on one thing the
blueprints never answer: **are budget units consumable (spent, then gone) or
transferable-and-accumulable (a re-spendable balance)?** This is a **SCOPE/VALUES decision, not an
engineering fix** — it is deliberately **not** resolved in this document.

The two options, stated precisely with honest tradeoffs:

- **Option A — pin budget units as CONSUMABLE-ONLY.** A budget unit is spent immediately on receipt
  of work — decremented from the payer's pre-authorized grant envelope, and **never credited into a
  re-spendable balance the worker can later transfer onward.** There is no accumulable balance, so
  there is no currency to gate: the loophole is closed **by construction**, and the Budget leg needs
  no red-line because no transferable value ever moves (it is *consumption of A's authorized budget*,
  not a *transfer of value to B*). **Tradeoff:** simplest, closes the gap completely, keeps
  budget-leg ergonomics (small, fast, autonomous, no human arming). **But** it is **too restrictive
  if legitimate accumulation is a real use case** — e.g. a worker agent earning credits across many
  payers to fund its own upstream sub-contracting (a legitimate compute-broker pattern) is forbidden
  entirely.

- **Option B — extend red-line arming to the Budget leg once units are transferable.** Treat
  accumulable budget-as-currency as a **first-class red-line category**: the accumulable
  budget-transfer path requires `HybridGate::new_redlined(RequireBoth, …)` + operator allow-listing +
  a co-scope, exactly as `LedgerMoney` does today (a new `RedLineCategory` or a mapping of the
  accumulable-budget action into `RedLineCategory::Money`). **Tradeoff:** preserves flexibility
  (accumulation allowed, the broker pattern lives) **but** imposes the **same human-arming friction
  on budget transfers that money already carries** — every accumulable budget transfer needs the
  operator to have pre-armed that specific grant, which is heavier operationally and defeats the
  "small, fast, autonomous pay-for-work" ergonomics that budget legs exist to provide.

**Marking — blocking.** Neither option is adopted here. Per this session's established convention for
P02's O-series decisions:

> **⚠ OPERATOR DECISION REQUIRED (blocking).** B2 **cannot be called execution-ready** until the
> operator rules budget-leg semantics: **Option A (consumable-only)** or **Option B (red-line-armed
> transferable budget)**. This is a **third hard gate on the Definition of Done** (peer to Hard gate 1
> — P07 §2 dedup fix — and Hard gate 2 — the canonical `Δ` authority): all nine §4 criteria GREEN and
> SH-1/SH-2's §SH-4 criteria GREEN are **necessary but not sufficient** while this ruling is
> outstanding. COUNSEL names it "the single most important safeguard … the difference between the
> operator's stated 'mundane pay-for-work' and an autonomous, un-gated, cross-jurisdiction currency
> the mechanism currently permits" (§8 item 1, §10). Until ruled, the design **permits** the ungated
> currency; the ruling is what makes the "not a trading platform" intent structurally true.

### SH-4 — Additive acceptance criteria (extend §4; falsifiable)

10. **Tick rate is a fixed protocol constant (SH-1).** A node's `now` advances at exactly one tick
    per real second off a monotonic `Instant`, provable by a test that samples `now` across a known
    `Instant` elapsed span and asserts a 1:1 tick-to-second mapping; `SystemTime` is not read on any
    `decide`/sweep path (grep-level assertion). RED: a build wiring a free-running counter or
    `SystemTime` fails.
11. **No absolute cross-node tick in any content-id (SH-1).** WorkReceipt field `0x06` is a relative
    `validity_ticks`; the receiver re-derives `receipt_deadline = t_admit_local + validity_ticks` and
    a receipt whose absolute *number* would differ across nodes still validates identically because
    only the duration is hashed. RED: two nodes with different monotonic epochs must both accept the
    same fresh receipt and both reject it after `validity_ticks` elapse **on their own clocks**.
12. **Rate-skew is detectable and bounded, not silently conforming (SH-1 residual).** A simulated
    payer whose settlements close in ≪ `2Δ` real seconds by the observer's own clock raises the
    advisory anomaly signal and does **not** block any commit; blast radius stays ≤ B3 exposure caps.
13. **Concurrent-count grief is refused pre-persist (SH-2).** With `MAX_CONCURRENT_SETTLEMENTS = 16`,
    the 17th concurrent non-expired `SettlementOffered` from one peer is refused
    (`PeerConcurrencyExceeded` → `CommitError::Rejected`, log length unchanged), while a 17th offer
    that only exceeds the count because an earlier settlement has **expired** (lazy-expiry) is
    **accepted**. RED against a B3 lacking the count cap: the 17th (and the 100th) offer commits and
    the aggregate drifts toward the 85 % freeze.
14. **Budget-leg ruling is present (SH-3, decision gate not code test).** B2's done-check fails while
    the SH-3 ⚠ operator decision is unresolved, regardless of code state — a documentation/gate
    assertion, mirroring Hard gate 1's structural-not-remembered treatment.

*Safety Hardening pass, 2026-07-17. Planning artifact only — no code written or edited. SH-1/SH-2 are
design fixes carried into B2's implementation pass; SH-3 is a blocking operator decision; SH-4
extends §4. Grounded in `capability.rs:57-59` (tick definition), the §2.1/§2.3 TLV layouts,
`redline.rs:73-78` (money-only red-line mapping), and B3 §2.1/§2.3 (value-only caps).*
