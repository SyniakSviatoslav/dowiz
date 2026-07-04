# S5-ORDERS/MONEY Port — Council Packet · COUNSEL OPINION

> Seat: **Counsel** (ethics · aesthetics · strategy) in the S5-orders/money Triadic Council.
> Advisory, non-blocking. Architect asks "will it work"; Breaker asks "how it breaks";
> this seat asks *should it exist, is it fair/honest/dignified, is it whole, what is the long
> horizon, who bears the cost, and what did nobody ask.* Docs only — no code, no design authority.
> Every load-bearing claim below was verified against live source, not trusted from the packet.

---

## OPINION: **PROCEED-WITH-REVISIONS** · **NO ETHICAL-STOP**

This is the crown-jewel red-line surface — money composition + irreversible state + the scariest
cutover — and the packet is disciplined, honest about its own deferrals, and correct on the three
load-bearing seams (money bytes, state folds, tenancy). I looked adversarially at every place this
surface could cross a grounded line and touch a real person in the money dimension. **None crosses a
grounded red-line in the direction the line protects**, so the friction here is Opinion, not a stop:

- The one place a customer could be **over**-charged — the tax composition (LC1 double-add / `i128`
  overflow / `f64` drift) — is *correctly and completely controlled* by the packet (Q1.2, S5-T4). The
  server stays the sole charge authority (ADR-0005), which is the anti-dark-pattern posture: the UI
  cannot lie about the charge because it is not the charge. I affirm this without additions; it is the
  Breaker's robustness domain and I do not re-litigate it.
- The `discountTotal=0` carry — the packet's own predicted counsel flag — is a real honesty gap, but
  its money **direction** is customer-neutral-to-owner-cost (a discount that never fires overcharges
  no one; it quietly cheats the *owner's* marketing, not the customer's wallet). That is below the
  threshold of a grounded red-line *charge* crossing. It earns the sharpest Opinion condition in this
  document, not a stop.
- The 085 watermark double-pay is a genuine near-term money landmine, but 085 is explicitly **out of
  S5** (S7/settlement) and already operator-assigned. My job is to give it a forcing function, not to
  stop an S5 money port over a migration S5 does not apply.

**Why no stop, and why S5 differs from S4 (where this seat did stop).** The S4 STOP was warranted by a
*verified intersection with a grounded line* — the doorway photo escaped the erasure graph entirely
(`анонімізувати-не-видаляти`) and the packet's own recommended disposition did not close it. Nothing in
S5 rises to that. The money-overcharge line is closed; the promotions gap does not touch a charge or a
confirm the way `сервер-авторитетний`/`soft-confirm-не-пастка` protect; the watermark is out of scope
and already owned. Issuing a STOP here would be verdict-not-friction — the exact overreach my mandate
forbids. This tracks the **S3** seat's posture (decline the stop on a carried, non-red-line-crossing
property; use conditions instead), not the S4 seat's.

**The one finding that changes the packet's own framing (verified, load-bearing).** The packet — and the
tasking that sent me — both describe the discount gap wrongly, in opposite directions, and the truth
sits between them and is worse than either. It is **not** "customers see promo codes that do nothing"
(customers cannot enter a promo code at all — there is no such field anywhere) and it is **not** merely
"an unbuilt feature" (the packet's words, Q1a). It is a **Potemkin feature**: a *fully built, live,
routed, guardrail-tested owner-facing Promotions program* sitting on top of a runtime that does not, and
cannot, redeem a single code. The misled party is the **small-business owner in Albania** — the launch-
trigger persona — not the customer. That reframing is the spine of §1 below and of condition §C-1.

---

## Verification note (I read the live source behind every load-bearing claim)

- **`discountTotal=0` is a literal, always** — `orders.ts:533` `const discountTotal = 0;`, and
  `:534` `total = subtotal + deliveryFee + chargedTax - discountTotal`. The subtraction term is a seam
  over a constant. Confirmed. The FE mirror agrees and says so (`packages/ui/src/lib/money.ts:84-85`:
  "`discountTotal` is 0 server-side today").
- **A whole owner Promotions surface is BUILT, LIVE, and ROUTED.**
  `apps/web/src/pages/admin/PromotionsPage.tsx` is a full CRUD console — create / edit / delete /
  toggle-active, discount value (%/fixed/free-delivery), min-order, max-uses, max-uses-per-customer,
  validity windows, applicable products, and a **`current_uses / max_uses` usage counter** rendered on
  every card (`:414-416`). It is routed at `/admin/promotions` and sits in the admin nav with a ticket
  icon (`AdminRoutes.tsx:43,275`). Its empty-state copy makes an explicit promise to the owner:
  *"Create your first promotion to start offering discounts to your customers."* (`:376`). It even
  carries two guardrail test files (`promotions-money-render.test.ts`, `promotions-audit-fixes.test.ts`)
  — the money-render one framed as an *"Albania go-to-market gap"* fix.
- **The redemption runtime does not exist.** `apps/api/src/routes/owner/promotions.ts` implements
  list/create/get/update/delete **plus** a `/validate` endpoint that *correctly* computes a
  `discount_amount` (`:206-219`) — but `/validate` is `requireRole(['owner'])` (a customer can never
  call it), it is **never called by the order-create path**, and `current_uses` is only ever **read**
  (`:189`), **never incremented** anywhere in the codebase (grep: zero writers). So `current_uses`
  is frozen at 0 for every promotion, forever.
- **There is no customer-facing promo entry, anywhere.** SPA client pages, components, and the server-
  rendered checkout were searched. The only client "promo" strings are a marketing feature-list item
  ("Full admin CRM — … promos & analytics") and a claim-button icon. The one stray `discountPercent`
  (`apps/api/src/client/checkout/app.ts:11`) is dead prototype state — initialised to `0`, never
  reassigned, in a file that also hardcodes `deliveryFee = 200 // for now` and is not the live order
  path. No customer ever types a code.
- **The 085 watermark is a literal, six days out.** `2026-07-10 00:00:00+00` is baked into the 085 fn
  bodies at three sites (draft `1790000000085_settlements-catchup.ts:66,133,148`), and the data-layer
  census R6 + REBUILD-MAP §7 both carry it. Today is 2026-07-04. Erring EARLY double-pays old courier
  cash rows; erring LATE is safe. Confirmed.
- **Q6's double-*charge* bound is real** — cash does not charge at create (S7 collects at delivery),
  crypto is dark (flags off), so there is no synchronous charge at create on either stack. The live
  cutover hazard is a duplicate *order*, not a duplicate customer *charge*. The packet's honesty here
  right-sizes the fear, and I affirm it (with one sharpening, §2).

---

## By charge

### 1. Q1a `discountTotal=0` — CARRY is right; the FLAG is mis-scoped

**The port decision is correct; the accepted-risk framing is not.** CARRY the `0`, keep the
`− discountTotal` seam, do **not** wire redemption during a money port. On that, the packet is right, and
for the right reasons — introducing a client-influenced discount into the exact transaction you are
trying to prove byte-identical is the worst possible moment to add a new money input (it directly
attacks the `S5-T11` "no request money field feeds `total`" invariant), and per-customer redemption
limits need an abuse model the port has no business inventing. Affirm Q1a as a *port* call.

**But the packet calls the gap "promo/discount redemption does not exist … an unbuilt feature, not a
defect." That is not what I found, and the difference is the whole ethical content.** Weighed across
lenses (plural, per mandate):

- **Honesty / `UI каже правду`.** A feature that is *unbuilt* is silent — it makes no promise. This one
  is *loud*: it ships an owner a console that says "Create your first promotion to start offering
  discounts to your customers," accepts a `SUMMER20 −20%` they author, lets them toggle it "active,"
  and renders a `0 / 100` uses counter. Every one of those is a statement to the owner that a capability
  exists. The runtime contradicts all of them. That is not an absent feature; it is a **standing untrue
  statement** by the product to its own paying user. The grounded `сервер-авторитетний` line keeps the
  UI from lying about the *charge*; nothing yet keeps the *owner console* from lying about what it *does*.
- **Care / who is harmed.** The customer is unharmed (no field, no wrong charge). The **owner** bears
  100% of it: the marketing effort spent authoring codes, the belief that a promotion is running, and —
  the sharpest cut — the `0 uses` counter, which an owner reads as *"my promotion flopped,"* when the
  truth is *"the system never let anyone redeem it."* A tool that quietly tells its user their idea
  failed, when in fact the tool failed, is a specific and avoidable injury to a small business.
- **Justice / asymmetry.** Asymmetric cost with no counterparty is the clearest injustice signature. No
  one *gains* from the Potemkin surface — not the platform, not the customer. It is pure entropy that
  falls entirely on the owner.
- **Long horizon.** This is a "what will we regret in a year" item. The first time an owner discovers
  their promotions never worked — likely when they ask support why redemptions are always zero — the
  trust cost is far larger than the honesty cost of saying so now. A rewrite that silently re-ships a
  live Potemkin surface as "parity" launders the neglect into a deliberate choice.

**Disposition.** No stop — this is owner-honesty, not a red-line *charge* crossing, and the fix is a
product decision on the Node stack that **S5 does not even own** (S5 ports the order path, not the owner
promotions admin). But the accepted-risk row the packet asks for must be **re-scoped and given teeth**
(§C-1): name the *real* risk ("a built, live, routed owner promotions program with no redemption runtime,
reporting 0 uses forever"), assign an owner, attach a **near-term trigger to either finish it or honestly
retract/label it**, and make a deliberate owner-legibility call on Node (hide the surface behind a flag,
or add an honest "not yet active" affordance) so the owner is not quietly misled while the rebuild runs.
The one thing that must not happen is the rewrite re-shipping the Potemkin surface *as parity* with a
footnote no one reads.

### 2. Q6 cutover double-order — the technical gate is proportionate; add the human act and the live-bytes probe

**Affirm the posture; do not over-engineer it; add two conditions from the S3 precedent.** The packet's
Q6(a) — atomic per-surface flip, land 086 first, time-box the overlap, gate on request-hash + money
byte-parity + a cross-stack idempotency probe, keep crypto dark — is the right shape for a 5–10
orders/min surface. A heavier control (dual-write reconciliation, per-request canary) would be
over-provisioning the fear; I explicitly do **not** ask for it. Two sharpenings, both from the honesty
and reversibility lenses:

- **"Rollback = flag-flip" is true at the fleet level and misleading at the order level.** A proxy
  flip-back leaves *committed* orders valid on either stack — true. But a **duplicate order already
  created** by a cross-stack retry is not undone by any flag: it is a data artifact a human (the owner)
  must notice and cancel, and for a **cash** order it can mean a *second physical delivery the customer
  is asked to pay cash for* — a real charge to a real person, arrived at without a duplicate *card*
  charge. The double-*charge* bound is real (affirmed above); the double-*order* is not free to reverse,
  and the packet should say so where it says "rollback is a proxy flag, not a data migration."
- **The gate's failure is silent, so prove it on real bytes and make the flip a distinct human act.** A
  one-byte `request_hash` drift does not error — it silently picks the wrong branch (false-422 or
  duplicate order). A golden-vector test proves *the cases you thought to write*; it cannot prove the
  case a real request introduces (header casing, unicode in `addressText`, an odd `menuVersion`) — the
  same "canonicalization divergence at the edges" the S4 seat flagged for hand-rolled SigV4. So the
  cross-stack idempotency probe must run **on the real overlap topology, both directions, before the
  flip** — not a unit vector alone. And, per the S3 precedent, **name the flip as a second, explicit
  operator go/no-go, distinct from DoD-green and from packet-approval**: the "human-in-the-loop /
  zero-autoban" line applied to the strangler flip itself, so no agent concludes "gates green → flip" as
  the tail of shipping (§C-2).

### 3. Q7 085 watermark — a forcing function, not a packet footnote

**Insist on it — this is the closest thing in the packet to a live, silent, real-people money harm.**
The watermark literal is six days out today; if the settlement apply slips past `2026-07-10` — entirely
plausible with a multi-surface rebuild in flight — and the migration lands with the stale literal,
**couriers are silently double-paid** (platform money out; the courier who is over-paid does not
complain, so nothing surfaces it). It is owned by "the operator," but ownership without a forcing
function is how a six-day fuse becomes a six-month audit finding. The S3/S4 lesson applies verbatim: *a
risk that lives only as a footnote in one packet is where it goes to be forgotten* — and here the
footnote is worse than usual, because it sits in the Q7 of a surface (S5 orders) that **does not even
depend on 085**. Condition (§C-3): lift the watermark **out of the S5 footnote** into a standalone,
tracked, operator-owned timing gate with a **forcing function** — at minimum a pre-apply assertion that
the literal `>= apply_date` (a one-line CI/migration guard on the three occurrences), or a dated
pre-apply checklist item with an owner. Proportionate friction: it blocks nothing; it only refuses to let
a silent double-pay land by inattention (`готівка → алерт-тертя`, applied to the payout side).

### 4. Scope, Charter, and the one real overcharge vector

- **Scope is disciplined, not bloated.** Payment webhook (S8), dispatch/settlement + 085 (S7), WS
  fan-out (S6), the `sales_channel` entity (deferred to a schema-evolution council) are all correctly
  excluded; the dark crypto create-fork ports inert; no schema change. The one non-tenancy FIX-IN-PORT,
  **Q5b MessengerKind**, is the *honest* direction and I affirm it: a customer who picks phone/signal/
  simplex is 422'd at the order boundary *today* (a live lost order), and re-shipping that break as
  "parity" would be the same neglect-laundering I flag in §1 — except here the fix is cheap, in-scope,
  and customer-facing, so **fix it**, gated on the Phase-0 DB-CHECK confirmation the packet already
  requires (else it trades a 422 for a 500). This is the mirror image of §1: fix the customer-facing
  break the port naturally closes; flag (don't silently carry) the owner-facing break it does not.
- **Charter: clean.** Orders and money. No military/warfare, no surveillance-for-harm, no commons-
  capture. The server-authoritative charge is the Charter's honesty spirit realised in code. No Charter
  violation the port introduces.
- **The overcharge vector is closed — affirm and leave it.** `i128` intermediate (never `f64`), byte-
  parity vs the zero-import hand-derived money vectors, the `inclusive ⇒ total = subtotal + fee`
  property (no oracle needed), and the large-cart × 100%-rate overflow vector are exactly the right
  red→green set for the one place a customer can be over-charged. I add nothing; adding to it would be
  re-litigating the Breaker's domain.

---

## Non-blocking aesthetic / strategic notes

- **The Potemkin surface is aesthetics-as-leading-indicator running *in reverse*.** My mandate holds
  that a whole, coherent surface predicts fewer bugs and less harm — beauty as an early indicator of
  both quality and ethics. The promotions console *inverts* it: it is a genuinely polished surface (real
  focus-trap modal, money-correct `formatALL` rendering, a11y audit fixes, two guardrail tests) built
  over a **null runtime**. The team spent care making a discount *display* correctly — a discount that
  never applies. A beautiful dead surface is more dangerous than an ugly one precisely because its polish
  earns the owner's trust that it works. Naming this is the point: the *legibility* fix (§C-1) is what
  makes the surface's honesty match its finish.
- **"Schema-rich, runtime-minimal" has a good twin and an evil twin here — name the line.** The
  `− discountTotal` seam over a `0` is the *good* version: a typed, exercised seam that constrains the
  future and costs nothing now (this is exactly the restraint the doctrine praises). The promotions CRM
  is the *evil* version: a **runtime-rich UI over a null runtime** — the opposite shape. The doctrine is
  about deferring *runtime you do not need*; it is not a licence to ship *UI you cannot back*. Keep the
  discount seam (it is restraint); fix or fence the promotions UI (it is not).
- **Server-authoritative composition is the aesthetic doing its ethical job.** One charge authority, the
  FE a display-only mirror with a cash-422 backstop — a single trust boundary the customer's wallet
  depends on, and it cannot lie because it is not the source. This is the design-language high point of
  the surface; the port's insistence on carrying it byte-identically is correct and worth affirming out
  loud.

---

## Steel-man of a rejected option (obligatory)

**Q1 option (b) — "wire real redemption now" — the option the packet rejects and I land against.**

Its strongest case, made fairly: the packet frames (b) as coupling "a money-math port to a net-new
feature + schema + a promo-abuse threat model, at the worst possible moment." But that framing overstates
how *net-new* it is, and my own verification is what strengthens the steel-man. **The feature is roughly
two-thirds built already**: the `promotions` table exists and is written; a full owner CRUD console
exists and is polished; and a *correct* discount calculator already lives in `/validate`
(`promotions.ts:206-219`, percentage and fixed both computed). What is missing is small and bounded — (a)
a customer-side way to submit a code, (b) calling the existing validator inside the order tx, (c)
incrementing `current_uses`. Against that, deferral has a real cost the packet does not price: the
Potemkin surface **persists indefinitely**, misleading owners every day it ships. And the timing argument
cuts the *other* way too — the money-composition tx is already open and under a byte-parity microscope
*right now*; the `− discountTotal` seam is in hand; this is, in a real sense, the **cheapest** moment the
system will ever have to close the gap at the root rather than paper over it with a register row. Closing
a live dishonesty is worth more than a clean diff. That is a genuinely strong position and I do not
dismiss it.

**Why I still land with defer-and-fence, not wire-now.** Two things the steel-man underweights, and both
are load-bearing on *this* surface specifically. First, the abuse model is not a formality: `max_uses`
and `max_uses_per_customer` presuppose a durable per-customer identity, but the order create path is
*anonymous/phone-keyed* — a per-customer cap is trivially spoofable by rotating phones (the same vector
the velocity throttles exist to blunt), so a redemption ledger honest enough to ship needs its own design,
not a hurried bolt-on. Second, and decisively: wiring redemption means threading a **client-influenced
money input** (the submitted code → a discount → `total`) into the *one* transaction whose entire DoD is
"prove `total` is byte-identical and no request field feeds it" (S5-T11). You do not open a new money
input in the tx you are trying to freeze. So (b) is right about the *destination* (the honesty gap should
close, and soon) and wrong about the *vehicle* (not through the money-port tx). I therefore **adopt the
steel-man's urgency into §C-1** — a near-term trigger to finish-or-honestly-retract, not an open-ended
"someday" — while keeping the port a port. (b) loses on timing and threat-surface, not on being right that
the gap deserves to close.

---

## Explicit conditions before code (§C)

Advisory — the human decides. Counsel-weighted, ordered by ethical load.

1. **[honesty, §1 — the sharpest flag] Re-scope the Q1a accepted-risk row, and give it teeth.** Replace
   "unbuilt feature" with the verified truth on a reviewed register: *"a built, live, routed, guardrail-
   tested owner Promotions program with no redemption runtime — codes are un-enterable by customers and
   `current_uses` is frozen at 0 forever."* Assign a **named owner** and a **near-term trigger**
   (`= "before the launch trigger — first real paid order — OR the promotions-redemption council, whichever
   first"`) to either finish redemption (its own council, not the money port) **or honestly label/fence
   the owner surface** on the Node stack (a flag, or a truthful "not yet active" affordance) so the owner
   is not quietly misled. CARRY `discountTotal=0` and the `− discountTotal` seam in the port as-is (Q1a is
   the right *port* call); the fix is a Node product decision, not an S5 code change. **Do not let the
   rewrite re-ship the Potemkin surface as silent "parity."**
2. **[cutover, §2] Name the flip as a second human act; probe on real bytes.** The proxy flip that makes
   Rust authoritative for real order/money writes is a **separate, explicit operator go/no-go**, distinct
   from DoD-green and packet-approval (zero-autoban applied to the strangler flip). Require the cross-stack
   idempotency + request-hash probe to run on the **real overlap topology, both directions, before the
   flip** — not a unit golden-vector alone. Record on the packet that a *duplicate order* (unlike a
   duplicate charge) is **not** undone by the rollback flag — it is a human-cleanup artifact, and for cash
   it can become a second delivery the customer is asked to pay for.
3. **[migration, §3] Give the 085 watermark a forcing function.** Lift it out of the S5 Q7 footnote into a
   standalone, tracked, operator-owned timing gate with a pre-apply assertion (`literal >= apply_date`
   across all three occurrences) or a dated checklist item + owner. It blocks nothing; it refuses a silent
   courier double-pay by inattention. Six days out today — treat it as live.
4. **[customer honesty, §4] Affirm Q5b(b) — fix the MessengerKind break, do not carry it.** Unify to the
   canonical 6-kind `MessengerKind`, gated on the Phase-0 DB-CHECK confirmation (admits all 6, else a 422
   becomes a 500), with the E2E delta (a `signal` order 422→201). Re-shipping a live checkout break as
   "parity" is the same neglect-laundering as §1, but here the fix is cheap and in-scope: close it.
5. **[money integrity, §4] Affirm the overcharge controls as the load-bearing red→green, and leave them.**
   `i128` intermediate (never `f64`), byte-parity vs the zero-import money vectors, the
   `inclusive ⇒ total = subtotal + fee` property, the large-cart × 100%-rate overflow vector. This is the
   one place a customer can be over-charged, and the packet closes it correctly. No additions — adding
   would re-litigate the Breaker's domain.

---

## The question nobody asked (§7)

The entire packet — and the tasking that framed it — measures money-honesty from the **customer's
wallet**: byte-parity so the customer is not over-charged, the cutover gate so the customer is not
double-*charged*, the MessengerKind fix so the customer is not turned away. Every seat in this council
speaks for the person being charged. That frame is correct and well-built.

**Nobody in this surface speaks for the owner as a person who is being quietly misled by their own
tool.** The small-business owner in Albania — the *launch-trigger* persona, the one whose first real paid
order is the whole point — opens a polished Promotions console, is told "start offering discounts to your
customers," authors `SUMMER20`, toggles it active, tells their regulars about it, and the system: gives
customers no way to enter it, never applies it if they somehow could, and renders `0 uses` back to the
owner as if to report that their idea failed. The provenance is faithful; the promise is false. Every
control in this packet protects the customer *from* the owner's charge; **none protects the owner from
their own dashboard.**

The unasked question is not technical and it does not block the port: *the port works hard to guarantee
the customer is charged honestly — what guarantees the owner is told honestly what their own dashboard can
and cannot do?* The honest answer is the same shape as the customer's: make the UI's honesty match the
runtime's reality — finish the capability, or say plainly it is not there yet. That question sits on the
§C-1 register with an owner and a trigger, so the person who cannot attend this council — the owner who
believed their promotion was running — is not discovered last, wondering why no one ever used their code.

---
*Advisory only. No ETHICAL-STOP. The human is final. Nothing here authorizes a code change, blocks S5, or
overrides a conscious operator decision.*
