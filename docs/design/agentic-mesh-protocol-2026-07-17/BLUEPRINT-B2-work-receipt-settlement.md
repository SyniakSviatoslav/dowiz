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

*B2 blueprint. Evidence re-read live 2026-07-17: `/root/dowiz-agentic-mesh` (`event_log.rs`,
`hydra.rs`, `deploy/deep-clean.timer`, MESH-03, P07, P14) and `/root/bebop-repo/bebop2/proto-cap`
(`signed_frame.rs`, `capability.rs`, `scope.rs`, `tlv.rs`, `revocation.rs`, `redline.rs`,
`hybrid_gate.rs`). No code written; settlement touches money-red-line paths and earns a careful
separate implementation pass informed by this document.*
