# COUNSEL — Ethics / Strategy Review of the Agent Exchange Plane

> Advisory opinion (2026-07-17), branch `feat/agentic-mesh-protocol-2026-07-17`. Written by
> Counsel at the operator's request for "an open-minded strategic ethical-aesthetic philosopher's"
> read. **This is non-blocking.** The operator decides; I inform. I evaluate the four blueprints
> (B1/B2/B3/B4) + the two failure-mode research docs (R1/R5) + the SYNTHESIS + CONSOLIDATED index.
> I do not gate, redesign, or fix. Where I flag friction I say so as friction, not verdict.

---

## §0 — What is genuinely good here (said first, because it is load-bearing)

Aesthetics are a leading indicator of both quality and ethics, and this arc scores high on the
axis that matters most: **intellectual honesty as a structural property.** Three of the four
blueprints re-read the live source their own research had cited and found the carried claim wrong
— the P07 dedup-ordering bug (B2 §1.1), the missing `TokenBucket::release` (B3 §2.2), the
under-priced envelope tax (B4 §2.4). A design process that catches its own errors before writing a
line of code is trustworthy in a way that no amount of assertion could substitute. The conceptual
economy — "reuse verbatim, one new primitive" (SYNTHESIS §3.4/§3.5) — is real restraint, the
"schema rich, runtime minimal" discipline honored. The rejections are argued, not asserted (the
§3.5 table cites a failure incident for every "no"). This is a beautiful design in the honest
sense. My criticisms below are *inside* that quality, not against it.

One more thing to credit plainly: the two most dangerous impulses — an autonomous **market/auction**
and **optimistic execution** — were both rejected with the sharpest citations in the whole arc
(R1 §5 Beanstalk, R1 §6 the three-year fraud-proof record). The design already declined the two
temptations that would have made it genuinely dangerous. What remains is a narrower, more defensible
thing. My review is about the residual.

---

## §1 — Real-world harm surface (Q1): the mundane framing is only *half* structurally enforced

The design's stated intent — "agent A pays agent B in compute-budget for completed work, not a
speculative trading platform" (R5 header; SYNTHESIS §2.4) — is honest intent. But intent is not
mechanism, and the question is what B1/B2/B3 *permit*, not what they promise.

**Two grounded harm patterns are real:**

1. **A compromised bridged agent can harm its own operator faster than a human can intervene —
   and this is by design, not by oversight.** B1 admits third-party agent *code* (LangGraph,
   CrewAI, MCP servers, bare binaries; §0/§2.3). The WASM sandbox is explicitly an **integrity
   boundary, not a confidentiality one** (B1 §2.2 step 4). Once admitted, the agent can
   autonomously initiate settlements up to its minted envelope, and B2 settlements are atomic with
   **no human at settlement time** (B2 §2.3). So yes: a malicious or subverted admitted agent can
   commit financial harm to *its own node operator* at machine speed. The blast radius is bounded
   (B3 caps × the `2Δ` grief window, B2 §2.3 caveat 2), and the operator's own keys had to sign the
   manifest — so the operator vouched for the agent. But "the operator vouched for it" is thin when
   the agent is an LLM-driven, non-deterministic process the operator does not fully understand.
   The honest statement: **the exposure cap makes the harm survivable, not preventable, and "faster
   than a human" is true and intended.**

2. **There are two economies, and only one wears the money red-line.** This is the sharpest
   finding. B2 §2.3 gives settlement a `leg_kind: Budget | LedgerMoney`, and B2 §2.4 arms the
   red-line gate + operator allow-listing + `(Ledger, Append)` co-scope **only for `LedgerMoney`.**
   The **Budget leg carries no red-line.** Budget units transfer between parties on settlement with
   full autonomy. The design treats budget units as inert "compute-budget," but nothing structurally
   ties a unit to real compute — it is an integer that moves on a signed event. **If earned budget
   units accumulate and re-circulate (B earns units from A, then spends them on C), they are a
   de-facto currency operating entirely outside money-law** — exactly the "resource token" R1 §7
   warns becomes a live reflexivity/run risk, and no document in the arc priced it. Whether this
   loophole is real turns on a single unresolved question the blueprints never answer: **are budget
   units consumable (spent, then gone) or transferable-and-accumulable (a balance)?** B2's
   "budget-transfer event" (§2.3 step 4) reads like the latter. If it is the latter, the "mundane,
   not a trading platform" framing is *not* structurally enforced — the mechanism permits an
   un-gated internal currency, and "outside any jurisdiction's oversight" becomes a literal
   property, not a hypothetical.

So my answer to Q1's core: the mundane framing is enforced for *money legs* (red-line + human
arming — genuinely good) and **not enforced for budget legs.** Close that, and the framing becomes
real. Leave it, and you have shipped a currency you are calling a budget.

---

## §2 — Is the exposure bound real (Q2): real for a snapshot, blind to patience and to information

B3's `ExposureLedger` is a faithful port of R5 §3's 15c3-5 position limit: per-counterparty stock,
healed only on settlement, checked pre-persist in the drift-gate slot (B3 §2.1). As a **stock**
bound it is a real structural guarantee — a single peer cannot have more than `cap` outstanding at
any instant, and the pre-persist check (B3 §4 acceptance 1) means it cannot be raced. Credit that.

But a stock bound was never designed to bound **cumulative** loss, and two grounded patterns walk
straight through it:

- **The patient griefer under the cap.** An attacker who commits, settles (freeing room), and
  re-commits stays under the cap forever while extracting a small loss each cycle. The exposure
  ledger bounds instantaneous blast radius, not lifetime extraction. R5's own 15c3-5 lineage is a
  per-instant control; it presumes a *separate* actor (a compliance regime, a counterparty who
  stops dealing with you) to handle repeat abuse. This design deliberately removes that actor
  (see below), so nothing bounds the cycle.

- **The information-goods residual — the harm that isn't in the ledger at all.** B2 §2.3 caveat 3
  states it plainly: A receives the output *bytes* before revealing the preimage `s`; an aborting
  A "keeps bytes it never claimed." The exposure cap bounds the *budget* griefed, but the value
  actually stolen is the **delivered work product**, which the ledger does not measure. For a
  compute mesh where the work *is* information (a computed result, an analysis, a dataset), the
  worker delivers, the buyer reads, aborts, keeps it free — and repeats across peers. **This is
  the single largest un-contained harm surface in the design**, and it is the one B3's own §2.5
  cannot touch, because the loss never enters the exposure stock.

Critically, the design's only stated containment for a bad peer is `ingest_peer_breach` zeroing the
cap (B3 §2.5) — but that fires on a **tamper `BreachAlert`** (hydra.rs `integrity_check` → Locked),
*not* on "this peer keeps aborting settlements." There is **no mechanism that detects a peer who is
merely griefing within the rules**, because detecting that would require remembering the peer's
history — which the arc forbids. So the honest answer to Q2: the bound is real for a snapshot and
does *not* close the patient-small-commitment pattern the research (R1 §5, R5 §6) already named. The
design closes the *auction* version of that pattern and leaves the *settlement-abort* version open.

---

## §3 — Power/access asymmetry (Q3): real, but ordinary — not the R1 failure-pattern

R1's meta-pattern 1 is "concentrated authority masquerading as decentralization" (R1 §Cross-cutting).
Does this design recreate it? My honest read: **no, not in the R1 sense — and the distinction is
worth being fair about.** R1's pattern is about *seizable trust* — a validator quorum, a fake-MPC
custody, a censoring relay — where bigger actors control the *protocol*. This mesh has no consensus
weight, no global head, no quorum (SYNTHESIS §2.4: "no sequencer to decentralize"). HRW assignment
(matcher.rs) is stake-free and deterministic — there is deliberately no "fastest hardware wins" lane
(SYNTHESIS §2.5; B3 §2.2 RC-2 guard). A well-capitalized node does *more business*; it does not gain
*authority over others*. That is ordinary market concentration, not the failure R1 warns against,
and calling it Lido-cartelization would be overstated.

The **one** place it does bite is downstream of §1's unresolved question: *if budget units become an
accumulable currency*, then capital concentration in units is real economic power, and the mesh has
— deliberately — no anti-concentration mechanism (caps are config, never history-derived; B3 §5).
That is fine for a compute-budget; it is a governance question for a currency. So Q3's answer is
conditional on Q1's: ordinary and acceptable if budget units are consumable; a real concentration
question if they circulate.

---

## §4 — Reversibility and kill-switches (Q4): M9 is a flow-stopper, never an undo

I verified the live mechanics (`hydra.rs:180-245`). `OrganismState::Locked` (M9 / owner re-seed) is
a **Law-pole reject on new commits at one node** (hydra.rs:227-231). It is genuinely strong for what
it does. But three properties mean it cannot "stop the bleeding" for whole categories of harm:

1. **It is per-node.** Locking my node refuses *my* new commits. It cannot stop other nodes'
   commits — by design (fork-freedom, SCOPE RULE). There is no global kill-switch, and there was
   never meant to be one. So "systemic exploit at scale" has no single stop.

2. **It does not halt in-flight settlements.** B3 §2.5 is explicit: in-flight commitments are
   **not force-failed** even on a peer breach — they resolve through B2's own tick-pure claim/refund
   legs, because force-failing would confiscate a conforming party's claim and break the Herlihy
   guarantee (B2 §2.3). This is correct, but it means: **once a settlement is claimed (`s` revealed,
   both halves in both logs, B2 §2.3 step 3), the value has moved and is irreversible the instant it
   commits.** Locking after does nothing.

3. **The information-goods residual is irreversible before settlement even begins** — the bytes were
   delivered (§2 above); no kill-switch un-reads them.

So the categories of **irreversible-by-construction** harm are: (a) any claimed atomic settlement,
(b) any autonomous Budget-leg transfer (no red-line to arm), and (c) any delivered work product.
This is not a flaw to fix — it is the *price of atomic DvP*, and you cannot add a clawback without
breaking Herlihy's "no conforming party ends up worse off." **But it should be named in
operator-facing terms:** the operator is trading reversibility for atomicity, per-transaction, and
that trade is itself irreversible. M9 stops the *next* transaction; it is not an undo for the last one.

---

## §5 — Scalability's ethical dimension (Q5): "operator-ruling per dispute" and "never reputation" both strain at scale

At 2 or 10 nodes, F44's dispute answer works: timeout/ambiguity → escrow HOLD + default refund to
claimant, **arbiter = operator ruling O3** (B2 §2.4; CONSOLIDATED CD-5). A human can adjudicate the
rare hard case. At thousands of nodes and autonomous agents, **"operator ruling per dispute" is a
2-node answer applied to a 1000-node problem** — you cannot put a human arbiter behind every disputed
settlement across a large mesh. The design punts this to F44 and never sizes it. It should be named
*now*: at scale, either the default-refund-to-claimant becomes the *only* automated resolution and
genuine disputes simply go un-adjudicated (accept that honestly), or a scalable arbitration story is
owed before scale arrives — discovering it after is the expensive order.

The deeper tension is **"never reputation."** The arc's rejection is well-grounded: R1 §4 cites
Cheng–Friedman (2005) — no *symmetric* reputation function is Sybil-proof, a theorem, not a tuning
gap. That rejection is *correct for global, gossiped, aggregated scores.* But the arc appears to
over-apply it. There is a category the impossibility result does **not** forbid: **first-party
bilateral memory** — "I, node A, refuse further *new* commitments to peer X because X defaulted on
*me* three times." That is not a symmetric aggregate over many raters (so Cheng–Friedman does not
bite), it is not gossiped (so no Sybil ballot-stuffing), and PQ identity makes it whitewash-resistant
*for the duration an identity persists*. At scale, this local bilateral memory is exactly the missing
containment for the §2 patient-griefer and information-goods thief — and forbidding it is what leaves
the memoryless blind spot. The tension is worth naming precisely: **"never reputation" should mean
"never a shared/aggregated/gossiped score," not "never a private first-party experience."** Conflating
the two is a real cost that grows with scale.

---

## §6 — Steel-man of the rejected option (required): "never *any* memory, not even bilateral"

I owe the strongest case *for* the arc's stricter line. It is genuinely defensible:

1. **There is no clean line between local memory and gossiped reputation.** The moment two honest
   nodes compare blocklists ("share who burned you"), you have a distributed reputation system with
   every Sybil/whitewash problem back in play. Forbidding memory entirely removes the gradient down
   which the system slides into the thing R1 §4 correctly rejects.
2. **Local memory is itself an attack surface.** An adversary can deliberately trigger timeouts to
   make an honest node distrust a *good* peer — griefing the memory. A stateless per-transaction bound
   has no such surface.
3. **Whitewash defeats it cheaply anyway.** Enrollment is deliberately cheap (anchor-delegation, not
   an entry fee — R1 §4's own Friedman–Resnick tradeoff). A burned peer re-enrolls under a fresh
   `NodeId` and the local memory is worthless — so the memory buys less than it appears to, at the
   cost of the slide in (1).
4. **The coherent philosophy:** *bound the blast radius of every single interaction so tightly that
   you never need to remember who is bad.* Structural containment (per-transaction caps, atomic DvP)
   is trustworthy in a way historical containment (your own possibly-poisoned memory) is not.

This is a real, honest stance, and for **small task-denominated budget units** it holds — the
per-transaction bound is tight enough that memoryless is fine. It weakens exactly where the value per
transaction rises: **money legs and information goods.** So the steel-man doesn't defeat my §5 point;
it *scopes* it. Bilateral memory is unnecessary for cheap compute exchange and increasingly necessary
as the exchanged value grows. The design should decide *per leg-kind*, not globally.

---

## §7 — ETHICAL-STOP assessment: zero hard stops, three friction lines to hold

No grounded red line is *crossed* here that would warrant a hard ETHICAL-STOP requiring a written
human decision before the arc proceeds. The two red lines that come closest are consciously handled:
money movement keeps a human arming the red-line gate (B2 §2.4 — the "human-in-loop / zero-autobahn"
principle is honored for money), and the autonomy-at-settlement is the operator's own explicit,
conscious design choice (stated in the task), which friction may inform but must not override. Applying
a delivery-app red line to domain-agnostic infrastructure would be exactly the out-of-domain misfire
my mandate warns against. So: **friction, not stops.** Three lines to hold, in decreasing groundedness:

- **F1 — PII confidentiality at the bridge.** B1 §2.2 step 4 states the WASM boundary is "an
  *integrity* boundary, not confidentiality." That means: **when this infrastructure later carries the
  delivery app's real user data (courier location, customer PII) into a bridged third-party agent,
  nothing in B1 structurally protects that data from the agent.** The existing "zero-PII-in-AI" and
  "anonymize-not-delete" red lines must be enforced *at manifest admission* — a manifest whose scopes
  touch PII should be red-lined exactly as money scopes are (B1 §2.2 step 2 has the hook; nothing uses
  it for PII today). Hold this line before this plane touches real user data.
- **F2 — the budget/money asymmetry (§1.2).** Decide whether budget units are consumable or a
  circulating currency *before* code, because that decision determines whether the money red-line
  actually contains value movement or whether a parallel un-gated economy exists beside it.
- **F3 — reversibility named to the operator (§4).** Atomic DvP is a conscious trade of undo for
  atomicity. That is a legitimate choice, but it should be signed off with eyes open, in writing, once
  — not discovered after the first irreversible settlement.

---

## §8 — Concrete safeguards I would add (non-blocking advice)

1. **Close the two-economy loophole.** Either pin budget units as *consumable-not-transferable* (spent,
   never accumulated into a re-spendable balance), or extend B2 §2.4's red-line arming to the Budget leg
   the moment units are transferable. This is the single most load-bearing change: it decides whether
   the operator's own "not a trading platform" intent is *true*.
2. **Bound the information-goods residual (§2), don't just caveat it.** Deliver valuable work in
   claimable *increments* so an abort forfeits only the last increment, or accept the residual explicitly
   and size Δ + per-peer caps to the *value of the work delivered*, not just the budget committed.
3. **Permit first-party bilateral memory (§5/§6), carefully bounded.** Allow a node to *locally,
   privately* refuse further *new* commitments to a peer that has defaulted on *it* N times — a private
   experience, never gossiped, never aggregated, never a shared score. Draw the line in a RED test:
   private-non-aggregated YES, shared/gossiped/scored NO. Decide it per leg-kind (unnecessary for cheap
   budget units; increasingly necessary as value rises).
4. **Elevate the Poly-Network invariant (CONSOLIDATED §5 Q2a / CD-8) from a canon-diff to a hard
   RED-test precondition on B1.** B1 is precisely the blueprint that adds new `(Resource, Action)` scopes;
   the invariant "no scope may authorize mutating the anchor roster / revocation path" should ship as a
   failing-test obligation the first time B1 touches `scope.rs`, not as a proposal the operator might merge.
5. **Name the scale-arbitration story now (§5).** Even if the answer is "at scale, default-refund is the
   only automated resolution and genuine disputes go un-adjudicated," write that down before scale, so it
   is a decision, not a discovery.

---

## §9 — The question nobody in the arc asked

Every document asks "can this be *forged*, *raced*, or *double-spent*?" — the robustness questions, well
answered. **Nobody asked: what is a budget unit a claim *on*, and who is harmed when that claim is
worthless?** The whole design treats the budget unit as a neutral accounting integer. But a unit is a
promise of future compute from *someone*, and the arc never says from whom, or what happens to a worker
holding earned units when the issuing node goes dark, forks away (fork-freedom!), or simply refuses to
honor them. Atomic DvP guarantees the *exchange* is fair; it guarantees nothing about whether the thing
exchanged retains value. R1 §7 (Terra/Luna, Iron Finance) is in the research folder precisely because
"the token was fine until confidence tipped" — and the perspective absent from the entire arc is the
**worker who did real work for units that later became unredeemable.** That worker is the "courier" of
this system — the party who performs and trusts they will be made whole. The design protects them at the
moment of exchange and is silent about the day after. Ask that question before scale, not after.

---

## §10 — Overall opinion: PROCEED WITH NAMED SAFEGUARDS

This is a disciplined, honest, aesthetically coherent design that already declined the two temptations
(autonomous auctions, optimistic execution) that would have made it dangerous, and that keeps a human on
the money red-line. It should **not** be reconsidered at the mechanism level — HTLC DvP, counterparty-
verified receipts, and the exposure ledger are the right primitives, well-grounded in R1/R5.

But **proceed with the §8 safeguards named before code**, in priority order: (1) resolve the
budget-unit-as-currency question so the "not a trading platform" intent is structurally true, not merely
stated; (2) bound the information-goods residual; (3) permit *private bilateral* memory (not shared
reputation) as the containment the per-transaction cap cannot provide, scoped per leg-kind. The Poly-Network
invariant (safeguard 4) is the cheapest and should be a hard precondition on B1 regardless.

The single biggest real-world harm surface I found: **the information-goods residual combined with the
memoryless "never reputation" stance** — a compute mesh where the work product is valuable information,
delivered before payment finality, where an aborting party keeps the bytes for free and the system is
structurally forbidden from remembering repeat offenders (B2 §2.3 caveat 3 + B3 §2.5 + SYNTHESIS §2.2).

The single most important safeguard I would add: **close the budget/money leg asymmetry (B2 §2.4)** — pin
budget units as consumable, or red-line-arm the Budget leg once they circulate — because that one decision
is the difference between the operator's stated "mundane pay-for-work" and an autonomous, un-gated,
cross-jurisdiction currency the mechanism currently permits.

*Counsel opinion. Advisory, non-blocking. No code read-write beyond this file; no other document edited.*
