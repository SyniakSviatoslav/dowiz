# Payments (card · cash · crypto) — Counsel Opinion

- **Seat:** Counsel (ethics · aesthetics · strategy) — Triadic Council
- **Authority:** ADVISORY. ETHICAL-STOPs below are **friction, not veto** — each pauses one
  launch act and asks for a *recorded human decision*; none blocks the design, none overrides a
  conscious human. Human is final.
- **Date:** 2026-06-30
- **Reads:** `docs/design/payments/proposal.md`, `docs/design/payments/research.md`, CLAUDE.md
  Ethics Charter, ADR-deliver-v2 (cash-as-proof), ADR-stage21 (NO-AUTO-DEDUCT).
- **Verdict in one line:** the *shape* is right and ethically literate (provider-agnostic port,
  webhook-as-truth, no-PAN, cash-as-floor, schema-rich/runtime-minimal). The friction is not in the
  architecture — it is in **what gets launched to a real human, in what order, with what disclosure**.

---

## 1. Reasoning by lens (only what is load-bearing)

**Justice / stakeholders.** Four parties, asymmetric exposure:
- *Customer (prepaid):* gains convenience, **loses the door as the moment of truth.** With cash a
  dispute resolves face-to-face; with prepaid the customer has paid before food arrives and must
  trust a webhook + a refund pipeline they cannot see.
- *Small owner:* prepaid moves **chargeback + PSP-fee risk onto the 1–5-person shop.** Cash had no
  chargeback. This is a real new cost the segment has never carried — it must be named, not assumed
  absorbed.
- *Courier:* see §6 (the missing perspective). Prepaid quietly removes the courier from the money
  loop and with it the cash handshake that today *is* their proof of delivery.
- *Platform:* gains a card rail. The Charter warns: do not let "platform gains PSP volume" silently
  reorder the product against cash (§ETHICAL-STOP-3).

**Dignity / autonomy.** Non-custodial crypto is the dignity-preserving choice precisely because it
collects **no customer KYC/ID** — proposal §4.5/NH-3 holds this. A custodial/KYC path would force
small-ticket food customers to surrender identity documents to buy a meal: disproportionate,
surveillance-creep, and a PII-egress red-line. Lock non-custodial-only; treat any drift to custodial
as a fresh council item, not an implementation detail.

**Honesty / consent.** Two honesty obligations the design must carry into UX, not just backend:
(a) the **all-in price including any surcharge** is shown *before* confirm (server-authoritative,
soft-confirm-must-not-be-a-trap); (b) **crypto's irreversibility** is disclosed *before* the customer
sends funds, with a written refund SLA. Without (b), crypto checkout is a trap dressed as a feature.

**Care / harm.** The collision that ruins a real person: *prepaid order, payment captured, food never
arrives / refused — and the refund is slow, manual, or (crypto) discretionary.* For card this is a
chargeback the **owner** eats; for crypto it is a customer left holding the loss with no
chargeback recourse. Both are care-harms the friction below targets — with friction (held launch,
written policy), not punishment.

**Long horizon / strategy.** Card-first / crypto-dark is **correct** and serves the launch trigger
(first real paid order). Crypto is, for v1 food-delivery, mostly polish *relative to the trigger* and
mostly *risk* (volatility, AML/laundering optics, 1–15 min confirmation latency vs a hot dinner
order, regulatory ambiguity). The one thing that makes crypto *strategically* legitimate rather than
hype is buried in research §1: **global card acquiring largely excludes Albania**, so crypto is a
genuine workaround to the acquiring gap — not a fashion. That earns it a *designed seat*, not a *v1
launch*. Reversibility is good: schema additive/inert, flags default-off — we can walk crypto back
to dark with zero data loss. Lock-in risk is well-mitigated by the port.

**Aesthetics / integrity.** The strongest part of the design, and worth naming: `payment_status ⊥
order_status` (money decoupled from fulfillment) is *conceptually honest* — it refuses to conflate
"money moved" with "food handed over," which is exactly the lie Option A would have told. Honest
schema → fewer phantom till-debts → less courier harm. The aesthetic and the ethic are the same
thing here. "Schema rich, runtime minimal" is respected. Keep it.

**Epistemic.** Steel-mans in §4. The carrying-and-unverified assumption: *that prepaid is a
customer/owner concern and the courier is neutral to it.* It is not — §6.

---

## 2. ETHICAL-STOPs (friction, not verdict — each pauses one act, awaits a recorded human decision)

**ETHICAL-STOP-1 — Do not flip `PAYMENTS_CRYPTO_ENABLED` to a real consumer without a recorded,
honest irreversibility disclosure + written refund SLA.**
- *Grounded line:* soft-confirm-must-not-be-a-trap + UI-tells-the-truth (server-authoritative) +
  care-harm.
- *Why:* confirmed crypto has no chargeback; refunds are manual/bespoke/discretionary (proposal
  §4.5). Letting a customer send irreversible funds without a plain-language "this cannot be
  reversed; refunds work like X within Y" is consent without information.
- *Cost of the friction:* near-zero — crypto is already dark. This only gates the future flip.
- *Resolution:* human writes the refund policy (NH-4) **and** the disclosure copy before launch.

**ETHICAL-STOP-2 — Do not launch card without a recorded decision on who bears the PSP fee, and a
hard rule that the courier never does.**
- *Grounded line:* NO-AUTO-DEDUCT (Stage-21 invariant) + server-authoritative honest price +
  soft-confirm-not-a-trap.
- *Why:* a PSP fee/surcharge must (a) **never** touch the courier till or earnings — it is not the
  courier's cost — and (b) if passed to the customer as a surcharge, be **visible before confirm**,
  not a silent line discovered after. A hidden surcharge is a dark pattern; a courier-borne fee is an
  auto-deduct the project has already ruled out.
- *Resolution:* human answers NH-4 (merchant-absorb vs transparent customer surcharge); the
  courier-never-bears rule is non-negotiable and should be an explicit invariant, not a default.

**ETHICAL-STOP-3 (latent / Charter) — The platform must not degrade, bury, or default-away-from cash
to grow its own PSP volume.**
- *Grounded line:* CLAUDE.md Ethics Charter ("never captured for the exclusive benefit of any narrow
  group; serves everyone") + dignity + serves-the-unbanked.
- *Why:* in Albania a large share of customers are unbanked / cash-preferring / privacy-preferring /
  elderly. Cash is the *inclusive* rail. The current design honors this (COD is the failure-first
  floor — good). The line is **future**: do not let a later UX iteration make card the default
  selection, demote cash to a secondary tap, or push owners toward prepay-only *for platform gain*.
  An *individual owner* legitimately choosing prepay-only is their autonomy; a *platform-level* nudge
  off cash is capture.
- *Status:* not currently crossed. Recorded as a line not to cross; revisit at any checkout-UX change
  that touches payment-method ordering or defaults.

*(PAN: already a standing red-line; proposal's SAQ-A / no-PAN / claim-check handling satisfies it.
The honest BYPASSRLS/B3 caveat is correctly carried. No new stop — affirm and keep the GUC-ready
policy.)*

---

## 3. Non-blocking aesthetic / strategic advice

- **Trust is a visible feature.** Show the provider/lock, the *all-in* price (with any surcharge
  itemized) before confirm, and a payment-status the customer can read at a glance. The
  awaiting-confirmation (crypto) state should read as *calm progress*, not anxiety ("Confirming
  payment — your order is reserved"), since 1–15 min of silence on a hot food order erodes trust fast.
- **Courier screen honesty.** For prepaid, the courier screen must say "PAID — hand over, collect
  nothing" (proposal OPEN-3 already names this) — and the courier needs a *prepaid proof-of-handover*
  that is as dignified and dispute-proof as cash-as-proof was (see §6).
- **Strategically: ship card, prove the rail to the launch trigger, leave crypto dark.** Resist
  all-three-at-once (NH-6). Each payment rail you launch is a refund pipeline, a dispute surface, and
  a reconciliation drift source you now own.
- **One refund *concept*, two ledgers** (proposal §4.3) is the right elegance — keep reporting reading
  both; do not let a future "unify the ledgers" refactor collapse the cash truth into the PSP truth.

## 4. Steel-man of rejected options

- **Option A (bolt "paid" onto cash-completion), steel-manned:** it would ship *fastest* and touch
  the least surface — for an owner who only ever wants "card = cash that's already in," the phantom
  till-debt is an edge they might tolerate. *Why still rejected:* it lies in the schema (money ≡
  fulfillment), and that lie lands on the courier as a real false debt and on reconciliation as
  permanent drift. Speed bought with a dishonest data model is paid back in courier harm. Reject
  stands.
- **Option C (hosted marketplace checkout), steel-manned:** maximal scope-shedding — no PCI, no
  webhook, no ledger to own; for a team that wanted *out* of payments entirely it is the cheapest
  path. *Why still rejected:* no such product onboards Albania food merchants with *our* courier +
  cash model, and it surrenders server-authoritative pricing (ADR-0005) and the working cash spine.
  You would throw away the system that already serves the unbanked to avoid building one webhook.
- **Crypto-in-v1, steel-manned:** because card acquiring largely *excludes Albania* (research §1),
  crypto is the one rail the operator can stand up *without* a bank contract — arguably the *fastest*
  path to a non-cash payment at all, and a real fit for diaspora/remittance-funded customers. *Why
  still dark:* irreversibility + AML optics + confirmation latency + no refund policy yet make a
  *consumer launch* premature, not the *design*. Designed-dark is the correct seat for it.

## 5. The question nobody asked

**What is the courier's proof-of-delivery — and standing in a dispute — once the cash handshake is
gone?**

Cash-as-proof was never only a money mechanism; it was the courier's *dignity artifact*. Handing over
food and receiving cash is a mutual, witnessed act that protects the courier from "it never arrived"
disputes. Prepaid silently removes the courier from the money loop: the customer has already paid the
platform, and if they later claim non-delivery, the chargeback/dispute lands on the **owner** while
the **courier** has no cash receipt to point to. Before card launches, answer: *what tangible,
dignified proof does a courier hold for a prepaid handover* (a tap-confirm is weaker than cash in
hand), and *does removing cash-as-proof leave the courier more exposed to disbelief, not less?* The
missing perspective is the courier's; the unverified assumption is that prepaid is a
customer-and-owner matter. It is not. (Ties to proposal OPEN-1/OPEN-3 — but as a *dignity* question,
not only a state-machine one.)

---

## 6. What REQUIRES a human (regulatory / business — NEEDS-HUMAN, gating)

These are not architecture; they are human decisions the design correctly defers. Counsel adds: do
not launch any card/crypto rail to a real customer until the relevant ones are *recorded*.

- **Albania legal / tax / AML / e-invoice** for online card **and** crypto acceptance (proposal NH-5).
  Albania has fiscalization/e-invoice obligations and crypto-AML exposure — a human (ideally local
  counsel/accountant) must confirm before money moves online. *Gating for both rails.*
- **Acquirer/PSP the operator can actually contract** (NH-1) — gates capture policy, fees, 3DS.
- **Fee bearer + refund policy** (NH-4) — see ETHICAL-STOP-2 (card) and ETHICAL-STOP-1 (crypto).
- **Crypto stance** (NH-3): confirm non-custodial-only, stablecoin-only — the dignity/privacy and
  no-MSB-scope choice. Any custodial/KYC drift returns to council.
- **Launch scope** (NH-6): Counsel recommendation = card-first behind flag, crypto designed-dark.

---

*Counsel applied to itself: this is friction proportional to a money + dignity surface, not
moralizing. The design is sound; the stops are cheap (all gate future flips, none block the build).
Nothing here overrides a conscious human — the operator may launch with eyes open and a recorded
decision. That is the point of writing them down.*
