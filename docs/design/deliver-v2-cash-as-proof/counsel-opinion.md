# Counsel Opinion — `deliver` v2 (Cash-as-Proof)

> COUNSEL seat, Triadic Council. Advisory (non-blocking) except a grounded ETHICAL-STOP.
> Pairs with `proposal.md` (ARCHITECT) + `docs/adr/ADR-deliver-v2-cash-as-proof.md`.
> Verdict line at the bottom. The human decides.

## Verdict (one line)

**NO ETHICAL-STOP — the grounded red lines hold.** The design is, on the values axis, the *more*
dignified of the two forks: it removes the auto-verdict that would have punished honest staff. I record
**3 binding conditions** (human to accept/reject) where a real-but-ungrounded fairness gap could harden
into a trap if left unspecified. They are conditions, not vetoes.

---

## 1. Reasoning by lens (only what is load-bearing)

### Dignity / courier power — the "bond" is NOT what the prose makes it sound
The proposal repeatedly sells the **"costly-to-fake cash bond"** as the security primitive. Read against
source, the bond is **cash the courier already physically holds** — the handler enforces
`cash_amount === total` (`assignments.ts:324-327`) and writes a `'hold'` row for that exact sum. The
courier **never posts their own capital**; "lying costs money" only because at reconciliation they would
be short of cash they were entrusted with — i.e. ordinary till accountability, the same as any cashier.
This is **dignity-preserving, not dignity-extracting.** The framing concern in the task ("staff personally
finances owner's fraud risk") does **not** hold for the mechanism as built.

The real residual lives one stage downstream, at **reconciliation** (out of this change's scope but
inseparable from its ethics): if a genuine shortfall *not the courier's fault* — robbery, a customer who
short-pays and walks, a counting error — is charged to the courier's pay **regardless of fault**, *that*
is where "staff personally finances the risk" becomes true. Nothing in v2 creates this, but v2's whole
justification ("the bond makes lying costly") leans on reconciliation existing. → **Binding condition C1.**

Naming honestly: the deconstruction of the verdict engine is **the pro-courier move**. False-positive
friction on an honest staff courier is the cruelty the Stage-20 design would have manufactured; deleting
it (by never building it) is the single most humane decision here. I affirm it without reservation.

### Justice / customer & burden-of-proof (§C, R-3) — the sharpest edge
"An unhappy customer MUST prove delivered ≠ ordered." The proposal frames the residual as **symmetric**
(one-time customer who didn't inspect ⟷ rare bad-actor owner). It is **not symmetric in power**:

| | Owner | One-time customer |
|---|---|---|
| Holds the immutable record | yes | no access |
| Controls adjudication | yes ("owner policy") | none |
| Repeat / reputation stake | yes | none |
| Lever (refund/chargeback) | n/a | **none (cash)** |
| Already paid | no | **yes** |

So "burden on the accuser" is being placed on the party who is **simultaneously already out the money,
holds no evidence, and faces the adjudicator's own conflict of interest.** Burden-of-proof on the accuser
is a *fair* doctrine **only when the accuser can access the evidence.** Today the evidence
(`delivery_trace` snapshot) is RLS-scoped owner-only. This systematically falls hardest on exactly the
most vulnerable buyer (one-time, elderly, low-literacy). It is **not** a grounded red-line crossing —
DeliveryOS has no "platform must give the customer a refund lever" line — so **not a STOP.** But it is a
genuine justice + care concern with a cheap honest fix. → **Binding condition C2.**

### "No state is a trap" — true structurally, with ONE hole: `paid_partial`
The structural win is real and elegant: putting offer/accept on `courier_assignments.status` (A2) means
offer-timeout/decline **cannot** touch customer state because customer state lives on a different row.
That *structurally* guarantees the red line rather than relying on code discipline. Affirmed.

The hole is `paid_partial`. The enum carries the value, but the delivered handler **422s whenever
`cash_collected && cash_amount !== total`** — so a partial collection has **no expressible delivered
path**, and §7 lumps `paid_partial` into the "no-cash tail" with *"no `'hold'` row → food returns."* That
is semantically incoherent: "partial" means *some* cash *was* taken. If a courier ever holds partial cash
with **no ledger row**, the cash is unreconciled → at shift close it either silently vanishes (owner
loss) or the courier is asked for money the system never recorded (**a silent debt trap landing on the
courier** — the precise pattern the task flags in #3). This is an underspecification, not a designed
trap, so **not a STOP** — but it is the one place where "no state is a trap" is not yet true.
→ **Binding condition C3.**

### Privacy / data-minimization on `delivery_trace` (gps_lat/lng, name_snapshot)
Persisting a GPS pin + name/price snapshot that **nothing reads** is the data-minimization tension. No
grounded line is crossed: RLS FORCE + owner-only + no AI surface (so `нуль-PII-у-ШІ` and
`анонімізувати-не-видаляти` both hold; `claim-check` is preserved — bus stays id-only). The honest
defense is that the crumb **does** have a consumer: **human dispute-adjudication evidence** (§C). That is
a legitimate purpose — *if it is stated and bounded.* "Record GPS forever, just in case" with no declared
purpose and no retention is textbook surveillance-creep. The §C customer-history window is 7 days; the
GPS/snapshot crumbs deserve the same explicit purpose + retention. Non-blocking recommendation (below),
adjacent to C2.

### Strategy / card seam (§D, R-4) — deferral is honest, framing should be honester
Deferring card is correct and the ADR is admirably loud ("must not bake in cash=proof"; burden does not
generalize). One strategic truth to say out loud: the model is not *more just* than a card model — its
"fairness" rests on cash-only's **absence of any recourse mechanism**. That is a market *constraint*, not
a design *virtue*. The day card arrives, statutory chargeback re-imports the consumer protection this
design quietly does without. Worth stating so a future reader doesn't mistake "no chargeback" for "we
designed away the need for one."

### Aesthetics / coherence
"Crumbs stay, the gate goes" is genuinely elegant — the design's beauty is that it *subtracts* the most
failure-prone surface rather than adding cleverness. "Schema rich, runtime minimal" (flag-off inert
columns) is honored. The §9 guardrail (lint: no transition may branch on a signal row) is the single best
artifact here — it makes "never build the verdict engine" *deterministic* instead of a good intention.
Affirmed strongly.

### Agent-health (process, with kindness)
The grounding pass ("the dangerous parts were never built; verified by grep; delete-surface ≈ zero") is
honest, falsifiable, and well-evidenced — no over-claiming. One motivated-reasoning *smell* to surface
gently: the recurring "costly-to-fake bond" language subtly markets **courier-collected cash as the fraud
control**, which is a short conceptual step from *"the courier is the fraud-control mechanism."* That is
the vector by which an anti-fake intent drifts into **punishment aimed at staff** at reconciliation. The
§9 guardrail blocks resurrection of the *verdict engine*; it does **not** block a future *courier-scoring/
penalty* layer landing on reconciliation. Watch that seam. Diagnosis is of the *framing*, not the authors
— the design itself is clean.

---

## 2. ETHICAL-STOP(s)

**None.** Walked every grounded line: human-in-loop / zero-autoverdict (held — one tap), friction-not-
verdict (held — only friction is reconciliation), courier-completes (held), GPS-garbage-rejected (held —
GPS never thresholded), cash→alert-friction (held), anonymize-not-delete (held — immutable trace),
zero-PII-to-AI (held — no AI), claim-check (held — id-only bus), soft-confirm-not-a-trap (held),
server-authoritative (held — server owns amount/outcome), schema-rich/runtime-minimal (held). a11y =
N/A (no UI surface in this change). **No line is at its grounded crossing — no STOP.**

## 3. Binding conditions (human to record a decision on; not vetoes)

- **C1 — Reconciliation must not charge no-fault shortfalls to the courier.** Before the bond is sold as
  the security primitive, the ADR (or the Stage-21 reconciliation spec) must state that a genuine
  no-fault shortfall (robbery / customer short-pay / counting error) is **not** silently deducted from a
  minimum-wage courier without a human, friction-not-verdict review. Otherwise "the bond" *does* become
  "staff personally finance the owner's risk."
- **C2 — The accuser must be able to see the evidence.** Burden-of-proof on the customer (§C) is only
  fair if the customer has at least **read-access to their own immutable order snapshot** (what they
  ordered, price, delivered-at). Cheap, honest, and it converts "prove something only the owner can see"
  into a real dispute. Pair with a **declared purpose + retention bound** for the `gps/name/price`
  crumbs (match the §C 7-day window unless a longer audit purpose is explicitly justified).
- **C3 — Specify `paid_partial`, or forbid it as a delivered outcome.** Either define its exact ledger /
  reconciliation treatment (where does partial cash land, who is accountable) **before** the handshake
  flag flips on, or explicitly reject `paid_partial` as a `delivered` path (enum value present but
  handler-rejected) so no courier ever holds unrecorded cash. Do not ship an enum value with undefined
  cash semantics — that is the one residual trap.

## 4. Non-blocking advice (aesthetic / strategic)

- Soften the "cash bond" prose to "collected-cash accountability" so no reader mistakes it for
  courier-posted surety, and so the framing doesn't seed a future courier-penalty layer (agent-health).
- State plainly that cash-only's lack of chargeback is a *market constraint*, not a fairness *virtue*
  (strategy / honesty for the card-seam reader).
- Extend the §9 guardrail's spirit to forbid not just verdict-gates but any *automated courier penalty*
  derived from a crumb — close the reconciliation seam against scoring-creep.

## 5. Steel-man of a rejected option (≥1)

**Option 1 — the Verdict Engine — rejected, rightly.** Its strongest surviving kernel is **not** the
gate; it is **independent corroboration**. Strip away the auto-`block`/`friction` (the cruelty) and what
Option 1 alone offered was a *neutral third signal* in a dispute. Option 2 discards corroboration
**entirely** and replaces it with "the owner adjudicates." That is exactly what makes R-3's burden fall
hardest on the powerless customer (C2): there is now **no neutral party in the room.** So the honest
steel-man is: *the rejected option contained the one thing the chosen design lacks — a signal nobody
controls.* The correct response is **not** to build the gate, but to recover the kernel cheaply (C2:
give the customer their own immutable snapshot as their independent evidence). Rejecting Option 1's
machinery is right; we should not pretend we gave up nothing.

## 6. The question nobody asked

**The entire ethical case rests on "courier = embedded staff, repeat, reputation-bound" — and nothing in
the system verifies or enforces that the assumption is true.** What happens to every dignity and fairness
guarantee when the assumption breaks: a restaurant uses a gig/temp/last-shift courier with no future
reputation stake, no embedding, no debt-relationship beyond today? The burden-of-proof model, the
"reputation" lever, the "fraud = direct owner risk" claim — all silently degrade to nothing, and the
one-time customer (C2) faces an accuser with no skin in the game either. The carrying assumption is
load-bearing for the *ethics*, not just the mechanics, yet it is **asserted, never checked.** Worth
asking before launch: what *is* the courier's actual employment status in the target tenants, and does
the fairness story survive when embedding is absent?

---

## RE-EXAMINE round 2 (COUNSEL · post-fix)

> Read the updated `proposal.md` (§5–§10, R-3/R-8/R-9) + `resolution.md` (C1/C2/C3 FOLDED,
> embedded-staff ACCEPT-RISK). Question: did the fixes close the holes — and did they open new ones?
> **Verdict: HARDENED. No new ETHICAL-STOP. The 3 binding conditions hold and improved. Three
> non-blocking tightenings carried + one human decision correctly left open (Stage-21 / embedded-staff).**

### C3 / Q1 — `paid_partial` forbidden: honest, but the honesty now lives at the doorstep, not in the schema
Forbidding `paid_partial` is the **leaner and more honest** of my two offered branches at the *ledger*
level: there is now provably **zero** unrecorded cash inside the system — exactly one `'hold'` (=total)
on `paid_full`, zero on every tail. The silent-debt trap I flagged is closed **in the machinery.**

But the task's scenario is sharp and correct: forbidding does not abolish the partial-payment *event*,
it abolishes its *representation*. If a customer physically hands over part of the cash and the courier
has already released (or leaves) the food, the courier's only two truthful-looking taps are `paid_full`
(false — full cash not collected) or `refused_payment` → CANCELLED (false — money **did** move, food
**did not** return). **Both are lies, and the silent debt simply relocates from the system to the
doorstep** — now with *less* trace than the underspecified-enum version, because CANCELLED asserts "zero
cash expected." So forbidding is honest **only under one unstated human-protocol invariant**: *the
courier never releases food without full cash in hand, and never accepts partial cash.* That operational
rule is what makes "short on cash → refused_payment, food returns" a *true* record rather than a swept-up
one. The resolution **assumes** that protocol (H-2: "a customer short on cash → food returns") but never
**states** it as a courier-facing rule or a UX affordance.
- **Holds (not a STOP):** ledger-level trap closed; the design intent is clean.
- **Tightening (extends C3):** state the doorstep invariant explicitly — *"no partial handover: full cash
  before food, or nothing changes hands"* — in the courier UX/training and the ADR. Without it, the
  partial-event injustice did not vanish; it moved to where the software can't see it. **Forbidding is
  honest *iff* the no-partial-handover rule is real and taught.** That is the difference between a clean
  subtraction and sweeping-under-the-rug.

### C2 / Q2 — customer evidence: substantive for the dispute that matters, with one honest limit; retention is over-long
**Not cosmetic.** The named dispute class in §C is a **content** dispute (delivered ≠ ordered). For that
dispute the *load-bearing* evidence is precisely the **immutable order snapshot** (items + integer price)
— which the customer now holds via their own authenticated read. The customer can now say "the snapshot
proves I ordered X; I received Y," independent of the owner. That is the right evidence for the right
dispute, and it genuinely recovers the steel-man kernel (a signal the owner does not solely control).
GPS / delivery-time is **irrelevant** to a content dispute, so withholding it is correct minimization,
not a power-imbalance.
- **Honest limit (state it, don't paper over it):** for the *other* dispute — "it was never delivered at
  all" — neither party holds dispositive proof (GPS proximity ≠ proof of handover), and adjudication
  remains the owner's. The read-access does **not** equalize *that* case; nothing cheap does, because it
  is inherent to cash-with-no-recourse. C2 closed the case it could close; say plainly it did not close
  the existence-dispute case.
- **Q2b — 90-day GPS retention is misaligned with its own declared purpose.** Purpose = dispute
  adjudication; the dispute *window* is **7 days** (§C). GPS's only declared consumer therefore expires
  with the dispute, yet the pin is held **90 days** then nulled. That is an **83-day overhang of location
  data past its stated purpose** — the surveillance-creep smell re-opening, smaller but real. `анонімізувати-
  не-видаляти` still **holds** (GPS→NULL, facts retained, owner-only, no AI), so **not a STOP** — but a
  purpose-bound retention must have its bound *derived from* the purpose, not picked. **Tightening:**
  either align GPS-null to the dispute-close window (7d + a stated settlement buffer), **or** write the
  explicit reason 90d is needed (e.g. slowest off-platform settlement). Right now it is "90d just in
  case" wearing a 7-day-purpose coat.

### C1 / Q3 — deferring no-auto-deduct to Stage-21 is acceptable, *because v2 is inert on this axis* — but the guard must be a durable artifact, not prose
The justification leans on Stage-21, but the **harm does not exist until Stage-21.** Verified in the
resolution (and consistent with source): deliver-v2 writes **only** the `'hold'` record and creates **no
deduction logic**. So shipping v2 alone harms **no** courier on this axis — the ethical exposure of v2
*by itself* is zero. The dangerous part (no-fault auto-deduct) can only materialize when reconciliation
deduction is built. Therefore deferring is **safe** *provided the deduction layer cannot land without
honoring C1.*
- **Holds (not a STOP):** sequencing is the safety — inert-now, gated-later.
- **The weakness:** a "carried constraint" living in a **design doc** has low recall — design docs get
  forgotten; a future Stage-21 author reading `'hold'` rows could naively wire "shortfall → deduct"
  without ever seeing this note. The resolution's real teeth are the **"no courier-scoring/penalty layer
  at reconciliation without its own Triadic Council"** rule. That is the right barrier — *if it is
  encoded as a durable artifact a Stage-21 PR actually trips* (ADR red-line + the §9 guardrail spirit),
  not just a sentence here. **Strengthening (not a STOP):** materialize C1 **now** as the durable
  artifact — a Stage-21 spec stub / pending-guardrail that fails until the no-auto-deduct invariant is
  written — so the protection cannot be silently skipped by forgetting. Acceptable to defer the
  *mechanism*; not acceptable to leave the *guard* as narrative.

### Q4 (new surface) — tail→CANCELLED collapse: honest in the *record*, but it hides the accusation from the *accused*
At the **record** level the collapse is honest: `payment_outcome` + `cancellation_reason` distinguish
`refused_goods` / `refused_payment` / `customer_cancelled_on_door` / courier-decline server-side, so
reconciliation and the owner see *who/why*. Good — the distinction is preserved where fairness needs it.

The gap is at the **customer view.** The customer sees a flat **"Cancelled"** and — per the resolution —
is **not** shown the recorded `payment_outcome`. Now connect this to the H-3 residual: a courier who
pockets the food and taps `refused_goods` records the *customer* as the refuser. That customer — home,
waiting, never served — sees only "Cancelled," with **no signal that the system recorded them as having
refused.** This is the **inverse of C2**: C2 says *the accuser must see the evidence*; Q4 reveals *the
accused cannot see the accusation.* A false "you refused" is invisible to, and therefore undisputable by,
the person it is recorded against. The collapse erases the accusation from the only party with motive to
contest it.
- **Not a STOP:** no grounded line ("customer must see cancellation reason") exists; the UI showing
  "Cancelled" is *under-informative*, not *deceptive* — server stays authoritative and truthful, so
  `сервер-авторитетний` / UI-tells-truth holds (omission, not lie).
- **Tightening (extends C2, cheap):** surface the customer's **own** recorded reason on their **own**
  order — a humane rendering of `payment_outcome` ("Cancelled — payment not completed" / "Cancelled —
  refused at door"), their own data. Steel-man for *not* doing it: showing "you refused" to someone who
  did not (a lying courier) is inflammatory and unprovable — but humane wording handles that, and the
  symmetry (the accused can at least *see and contest* the record) outweighs it. This is the same justice
  move as C2, pointed the other direction.

### Q5 — embedded-staff: ACCEPT-RISK is sufficient *for v2*, because v2 needs no structural guard — there is nothing yet to gate; fuse it with C1 at Stage-21
A structural guard ("disable bond/reconciliation for non-staff couriers") is **not needed in v2** for a
simple reason: **v2 contains no bond, no deduction, no score** — there is nothing to switch off. The
mechanics (offer/accept, tap-completes, `'hold'` record, crumbs) are identical and harmless for a gig or
a staff courier alike; neither is harmed *by v2*. The embedded-staff assumption is load-bearing for the
**ethics narrative**, and that narrative only cashes out into **harm** at the same downstream place as C1
— reconciliation. So **Q5 collapses into C1.** The dominating mitigation is **not** "gate by employment
status" but **"no auto-deduct from *any* courier without human friction-not-verdict review"** — which
makes the employment-status question *moot for harm* (no one is auto-deducted regardless of contract),
while leaving the **fairness-of-burden** narrative (is §C fair when the courier is a stranger?) as a
genuine product/launch judgment, correctly **NEEDS-HUMAN.** Note C2's fix already partly insulates the
gig case: the customer's own-snapshot evidence works regardless of courier reputation.
- **Sufficient (not a STOP).** **Recommendation:** fold R-9 (embedded-staff) and R-8 (C1) into **one**
  Stage-21 invariant — *reconciliation never auto-deducts; shortfalls are owner-reviewed friction* — and
  drop the framing that a structural "non-staff gate" is the answer. The no-auto-deduct rule dominates
  employment-status and is one durable artifact instead of two soft notes.

### Do the binding conditions still hold? New STOP?
- **C1 — HOLDS,** hardened by the "v2 is inert / harm is downstream" verification; carry the strengthening
  (durable artifact, fuse with R-9).
- **C2 — HOLDS,** substantively (content-dispute evidence is real, not cosmetic); two tightenings —
  surface the cancellation reason to the customer (Q4), align GPS retention to its purpose window (Q2b).
- **C3 — HOLDS** at the ledger level (zero unrecorded cash in the system); one tightening — state the
  doorstep no-partial-handover invariant that makes forbidding honest rather than relocating (Q1).
- **New ETHICAL-STOP: NONE.** Re-walked every grounded line after the fixes — none is at its crossing.
  The two new sharp edges (Q2b retention overhang, Q4 hidden-accusation) are *quality/justice* concerns
  within held lines (`анонімізувати-не-видаляти` holds; UI under-informs but does not deceive), not
  crossings.

### Final counsel verdict (RE-EXAMINE)
**HARDENED — загартовано.** The fixes did not create new traps; they closed the holes at the layer they
operate on (schema/ledger/handler) and pushed the residuals to where they honestly belong — the
**doorstep protocol** (C3/Q1), the **customer view** (C2/Q4), and **Stage-21 reconciliation** (C1 + Q5,
now fusable). None of those residuals crosses a grounded red line, so there is **no STOP**. What remains
is **one human decision, correctly open:** the Stage-21 no-auto-deduct invariant + the embedded-staff
fairness call (fused), plus three cheap, non-blocking tightenings that convert "honest by intent" into
"honest by stated rule." Ship-clearable on the values axis; the human records the Stage-21 invariant and
the three tightenings before the bond is *sold* as the primitive or the handshake flag flips.

---

## RE-EXAMINE round 3 (COUNSEL · final confirmation)

> Read RESOLVE round 2 (`resolution.md` §"RESOLVE round 2") + updated `proposal.md` (§5/§6/§8/§9/§10).
> Question: did each round-2 tightening **land as a recorded rule/artifact** — not as intent/prose?
> **Verdict: all four landed as written rule. No residual ETHICAL-STOP — загартовано. One human
> decision remains open (the already-named Stage-21 no-auto-deduct + embedded-staff), correctly.**

**1. no-partial-handover — LANDED as an explicit courier rule (not just enum-ban).** proposal §6 (the
"🔴 Carried no-partial-handover rule (R2-4 / counsel-Q1)" bullet) + resolution R2-4: the carried rule is
stated verbatim — *"No partial handover — full cash in hand before the food changes hands; short on cash →
no goods, tap `refused_payment`"* — and, decisively, the completion UI **offers no partial-amount
affordance**, so partial collection is *operationally prevented*, not merely enum-rejected. Owner Product +
courier-UX. The doorstep invariant my round-2 note demanded is now a written rule + a UX absence, not an
assumption. This is the difference between a clean subtraction and sweeping-under-the-rug.

**2. GPS retention — LANDED: 90d→14d derived-from-purpose + a REAL anonymize-not-delete worker.** proposal
§8 + resolution R2-7: the bound is `DELIVERY_TRACE_GPS_RETENTION = 14 days = 7d dispute + 7d settlement
buffer`, explicitly *derived from* the §C purpose, not picked — closing the 83-day overhang. The mechanism
is no longer prose: `workers/delivery-trace-retention.ts` (mirroring `access-request-retention.ts`) does
`SET gps_lat=NULL, gps_lng=NULL, name_snapshot=NULL, price_snapshot=NULL` (anonymize — the non-PII facts
total/delivered_at/distance/payment_outcome retained, 🔴 anonymize-not-delete holds), with a boot-assert
(`process.exit(1)` if the schedule is missing). The red-line is now an *enforced control with a falsifiable
failure mode*, not a sentence.

**3. C1 no-auto-deduct — LANDED as a durable on-disk guardrail, not a reminder; R-8+R-9 merged + anti-
scoring-creep guard.** proposal §9 + R-8/R-9(merged) §10 + resolution C1/Q5: materialized as (a) a
**failing pending-guardrail** `stage21-no-auto-deduct.invariant.test.ts` that stays **RED until**
`docs/adr/ADR-stage21-reconciliation.md` exists carrying the markers `NO-AUTO-DEDUCT` **and**
`NO-COURIER-SCORING` — so the protection cannot be forgotten; (b) an `eslint-plugin-local` ban on any
non-`'hold'` `courier_cash_ledger` write **and** any penalty/score derived from a `delivery_trace`/signal
column (= the anti-scoring-creep guard I asked for, fused in); (c) a regression-ledger row. R-8
(no-auto-deduct) and R-9 (embedded-staff) are collapsed into **one** invariant — *no one is auto-deducted
regardless of employment status*, which dominates the employment-status question for **harm**.
*Honest scope note:* this is a design-time change ("NO production code in this change"), so these are
**specified** durable artifacts the implementation is now **bound** to author red→green — not yet bytes on
disk. The design has converted "carried constraint in a doc" into "a guardrail a Stage-21 PR must trip,"
which is exactly the by-rule (not by-intent) form requested. The only by-intent residue is the implementer
actually creating the specified files; the spec now names their paths, markers, and red-state precisely, so
the obligation is checkable. Acceptable.

**4. Q4 — LANDED: the accused sees their OWN recorded outcome, humanely.** proposal §8 ("The accused must
see the accusation") + resolution Q4: the customer's authenticated order read surfaces the customer's
**own** `orders.payment_outcome` + `orders.cancellation_reason` (RLS-scoped to `customer_id=$me`),
humane-mapped client-side (raw enum never exposed) — `refused_payment`→"payment was not completed",
`refused_goods`/`customer_cancelled_on_door`→"recorded as refused at the door", `courier_aborted`→"the
delivery could not be completed". The inverse-of-C2 gap (a falsely-recorded refuser couldn't see the
accusation) is closed; the accused can now see and contest, server still authoritative. A written
field + render rule, not intent.

### Are the binding conditions now "honest by RECORDED rule" (not by intent)?
**Yes — all of them.** C1 (durable failing guardrail + eslint ban + ledger row), C2 (customer own-snapshot
read + worker-enforced retention), C3 (no-partial-handover rule + UX affordance absence), and Q4
(own-outcome humane render) are each now backed by a written rule, a guardrail, or a worker. The three
"honest by intent → honest by stated rule" conversions my round-2 verdict required are done.

### Residual ETHICAL-STOP?
**None — загартовано.** Re-walked every grounded line after round-2 resolution: human-tap authority,
friction-not-verdict, courier-completes, no-trap-states (now structurally enforced on both completion paths
+ all exits), GPS-garbage-rejected (never thresholded), anonymize-not-delete (now worker-enforced),
zero-PII-to-AI, claim-check (id-only bus), soft-confirm-not-a-trap, server-authoritative, schema-rich/
runtime-minimal (flag-off inert), money-integer. None is at its crossing. No STOP.

### Anything left beyond the one already-named human decision?
**No new ethical decision.** The single open human decision is unchanged and correctly open: the **Stage-21
no-auto-deduct invariant fused with the embedded-staff fairness call** (now guarded red-on-disk so it
cannot be silently skipped). Two items sit *beside* it, neither a new values-axis decision:
- **Flag-flip sequencing** (`COURIER_OFFER_HANDSHAKE_ENABLED` on only when the accept/decline + `/abort`
  UI ships) — an **engineering-readiness gate**, not an ethical choice.
- **R2-9** (pre-existing masked courier name/phone/live-GPS to the customer during an active order) — a
  **pre-existing** privacy thread, *not introduced by v2*, correctly DEFER-FLAGged to Privacy; v2 adds no
  new courier PII. Honest to track, but not a decision v2 forces.

**Final round-3 counsel verdict: загартовано — by recorded rule, not by intent.** All four round-2
hardenings landed as artifact/rule/worker; no residual ETHICAL-STOP; the one human decision (Stage-21
no-auto-deduct + embedded-staff) remains open and is now durably guarded against silent omission. Clear on
the values axis.
