# S7-COURIER/DISPATCH Port — Council Packet · COUNSEL OPINION

> Seat: **Counsel** (ethics · aesthetics · strategy) in the S7-courier/dispatch Triadic Council.
> Advisory, non-blocking. Architect asks "will it work"; Breaker asks "how it breaks";
> this seat asks *should it exist, is it fair/honest/dignified, is it whole, what is the long
> horizon, who bears the cost, and what did nobody ask.* Docs only — no code, no design authority.
> Every load-bearing claim below was verified against live source, not trusted from the packet.

---

## OPINION: **PROCEED-WITH-REVISIONS** · **NO ETHICAL-STOP**

S7 is the surface where the **least-protected human in the whole system** — the courier: on the road,
paid in arrears, location-tracked, lower-trust by design — drives the physical fulfilment of a paid order,
collects cash, and reads their own pay. If any surface earned a STOP on dignity grounds it is this one, and
I looked adversarially at every place it could cross a grounded line and cost a real person — the courier
hijacked by a co-worker, the deactivated courier keeping a 14-day tail over a customer's GPS, the courier
underpaid in silence, the customer whose address a courier can harvest. **None crosses a grounded red-line
in the direction the line protects, because on every one the packet's *own* recommended disposition closes
the line** — so the friction here is Opinion, not a stop:

- The **session-liveness gap** the tasking names as the sharpest worry (a deactivated courier with a 14-day
  live tail over GPS + cash + pay) is the **rejected** option Q1(c), not the recommended one. The packet
  recommends Q1(a) — **carry the per-request `courierSessionValid` bind** — and S7 is REST, so the bind runs
  on *every* request: the moment a session is revoked (logout / owner-deactivate / password-change), the
  *next* courier call is 401. Verified: `me.ts:161-164` revokes all sessions on password-change. The line is
  closed by the packet's own disposition; a STOP would be verdict-not-friction. Affirm the CARRY, forbid the
  downgrade (§C-1).
- The **GPS consent boundary** — the one genuine surveillance asset over the least-protected human — is not
  merely carried, it is **consent-gated in code**: position is stored ONLY on active delivery, and the
  courier's *accept* is treated as the consent act (`courier-gps.ts:3-9`). This is `GPS-сміття-відкинуто` and
  `кур'єр-гідність` realised, not walked into. Affirm it loudly (§ aesthetics).
- The **cash-as-proof `cash===total`** rule the tasking frames as *unfair to the courier* is, verified, the
  **opposite** — it protects the courier from a till-debt for cash they never collected (§2). The unfairness
  the tasking intuits is real but lives one step away, in a **missing tip/change affordance**, not in the
  equality assert.

**Why no stop, and why S7 tracks S5/S6, not S4.** The S4 STOP was a *verified intersection*: the doorway
photo escaped the erasure graph (`анонімізувати-не-видаляти`) and the packet's *own* recommended disposition
left it open. Nothing in S7 rises to that. Each S7 harm-vector (cross-courier hijack, the logged-out tail,
cash-proof bypass, settlement double-pay, GPS off-delivery, the tenancy seat) is a **current** property the
port carries **visibly** with a red→green test, and the packet's recommended disposition *closes* each in the
direction the line protects. This is exactly the S5/S6 posture: conditions, not stop, when the packet's own
disposition holds the line.

**The two findings that change the packet's own framing (verified, load-bearing).** My verification
sharpened two of the packet's predicted counsel flags into something more specific than the packet states:

1. The **`/regenerate` cross-tenant blast is *not* independently idempotent-safe** — its safety is
   *conditional on 085 having landed*, and 085 has not (watermark `2026-07-10`, today is `2026-07-04`). The
   packet says "idempotent under 085 … *once 085 lands*"; the parenthetical is load-bearing and gets lost.
   Today, an owner at location A sweeping ALL tenants runs them through the **pre-085 buggy fn** (§3).
2. The **owner `/details` PII exposure is broader than the packet admits** — the packet calls it "plaintext
   customer name/phone"; the live source also returns **`o.delivery_address`** (plaintext, 20 orders,
   `couriers.ts:274-284`). The role-tier the packet asks Counsel to *ratify* must be ratified over its
   *actual* scope, not the under-stated one (§4).

---

## Verification note (I read the live source behind every load-bearing claim)

- **A courier cannot read another courier's payout.** `courier/settlements.ts:34` `WHERE p.courier_id = $1`;
  `:68` `WHERE p.id = $1 AND p.courier_id = $2`; the settlement-items query (`:79-85`) hangs off a payout row
  already verified as the caller's. The belt predicate holds independent of which pool role is live. The
  tasking's question — "does a courier see someone else's payout?" — answers **no**, at the read layer, today
  and post-B3. (The seat is still broken — pool `set_config` + `is_local=true` + no `BEGIN`, `:24-26` — which
  post-B3 makes the `settlement_items` bare-`current_setting` policy *error*; that is the packet's Q6
  FIX-IN-PORT and I affirm it, but it is a seat bug, not a cross-courier leak.)
- **`cash===total` is exact, server-authoritative, and integer.** `deliveryCompletion.ts:63`
  `if (isPaidFull && args.cashAmount !== args.total) throw CompletionError('CASH_AMOUNT_MISMATCH')` — 422
  before any mutation (`assignments.ts:375`). `total` is read server-side FOR UPDATE (`assignments.ts:320,
  325` `o.total`), `cash_amount` comes from the request body (`:308`); money is integer minor units
  end-to-end (`cash_amount: z.number().int().nonnegative()`, `:301`). There is **no rounding/"kopeck" class**.
  The no-cash tail (`deliveryCompletion.ts:75-78`) gives the courier an honest exit: `refused_payment` /
  `customer_cancelled_on_door` → order CANCELLED, **no HOLD**. Confirmed.
- **The GPS gate is consent-scoped in code.** `courier-gps.ts:3-9` — "stored ONLY while the courier is on an
  active delivery — tracking begins at the courier's CONSENT act (assignment 'accepted') … a future refactor
  must not silently re-add 'assigned' here or it re-opens pre-consent location tracking";
  `ACTIVE_DELIVERY_ASSIGNMENT_STATUSES = ['accepted', 'picked_up']`. The immutable `delivery_trace` stores the
  *delivery* coords (`gpsLat = dRow.delivery_lat`, the customer's location, `assignments.ts:367` →
  `deliveryCompletion.ts:106-113`), NOT the courier's movement trail; the courier's own positions are
  retention-purged (`courier-gps.ts:11`). The least-protected human's movement history is the *most*-protected
  data. Confirmed.
- **The owner `/details` returns plaintext customer name + phone + ADDRESS × 20.**
  `owner/couriers.ts:274-284` — `c.name AS customer_name, c.phone AS customer_phone, o.delivery_address`,
  `LIMIT 20`, bare pool (no tenant seat, relies on `WHERE a.courier_id=$1 AND o.location_id=$2`). The courier's
  own `/me/history` (`me.ts:15-27,249-264`) MASKS the name (`maskStr`) and returns **no** phone, **no**
  address. The packet's "plaintext name/phone" understates it by the address. Confirmed.
- **`/regenerate` runs ALL locations on the request path.** `owner/settlements.ts:301-317` —
  `new SettlementCronWorker(db,…).handleGenerate(new Date(referenceDate))` with the literal source comment
  "Technically processes all locations. For scale, we'd limit to locationId." Rate-limited 5/5min. An owner
  at location A triggers a settlement sweep across every tenant. S5-path-owned. Confirmed.
- **The 085 watermark is a literal, six days out (carried from the S5 verification).**
  `2026-07-10 00:00:00+00`, three sites in the 085 draft; today `2026-07-04`. Erring EARLY double-pays old
  courier cash rows; erring LATE is safe. This is the *same* landmine the S5 seat gave a forcing function
  (S5 `counsel-opinion.md:281-284`), and S7 is the surface it actually lands on.

---

## By charge

### 1. The least-protected human at the flip and after (Q1 🔴) — the bind is the fix; carry it, do not downgrade

**Affirm Q1(a) as the right and minimal answer to the tasking's "CARRY or FIX-IN-PORT?" — and name it
precisely: it is CARRY, because the fix already exists.** Unlike S6 (where the per-frame relay guard
re-authorizes the *binding*, not the *session*, leaving a mid-stream residual — S6 `counsel-opinion.md:54-59`),
S7 is stateless REST: `courierSessionValid` re-reads `courier_sessions` by `jti` on *every* request, no cache
(`plugins/auth.ts:24-92`). So the 14-day-tail the tasking fears is **already closed on Node** and the port's
job is not to *build* the fix but to *not lose* it. Weighed across lenses:

- **Care / `кур'єр-гідність` + the customer's privacy.** A revoked/deactivated courier loses the tail on the
  *next* call — over their own money/GPS *and* over the customer's live address/GPS they can see mid-task.
  Two real people are protected by one predicate. The only way this line opens is the rejected Q1(c)
  (JWT-only); the packet forbids it (🔴 S7-T3). The single dignity risk left is **scope-pressure downgrade** —
  a future "simplify auth" that drops the per-request read as "expensive." Name it as a red-line the port
  must not quietly cross.
- **Wholeness across stacks.** During cutover a courier request may hit Node or Rust; both must run the
  *identical* predicate against the *same* row, or a revoked courier is admitted on one stack (the packet has
  this, §9.3 + the S6 REV-S6-2 shared spec). Affirm — this is the whole version: REST (S7) and WS (S6) both
  bound by one liveness check.

**Disposition (§C-1).** Not a stop — the packet's own disposition closes the line. AFFIRM Q1(a); mark the
JWT-only path (Q1c) as a **forbidden downgrade** on the record, and require the session-liveness golden test
to run **against both stacks** so the parity is proven, not assumed.

### 2. Cash-as-proof (Q4 🔴) — the equality protects the courier; the unfairness is a *missing* affordance

**The tasking's worry — "розбіжність у копійках → 422, курʼєр застряг з їжею" — inverts once you read the
code, and the inversion is the finding.** Money is integer minor units, so there is no rounding class to
strand anyone. And `cash===total` does not strand the courier: it **refuses to record a `paid_full`** (which
writes a `courier_cash_ledger` HOLD = the cash the courier now *owes the till*, `deliveryCompletion.ts:118-123`)
**for cash the courier did not collect**. Relaxing to `>=` would over-state the HOLD and make the courier
liable for a till-debt on money that never reached their hand. So the exact-equality rule is **pro-courier**:
it protects the least-protected human from being booked as having collected what they didn't. The honest exit
for a short-paying customer already exists — `refused_payment` → CANCELLED, no HOLD, no phantom "delivered."
CARRY it verbatim (S7-T11); the packet is right.

**But the fairness intuition is not wrong — it is aimed one step off.** The rigidity a courier *actually*
hits is the **over-payment / "keep the change"** case: a customer hands 2000 ALL for a 1970 ALL order.
`cash===total` refuses it (2000 ≠ 1970); the `/delivered` body has **no tip-at-door field** (`assignments.ts:
298-302`) and `o.tip_amount` is set elsewhere, not captured here; so the courier's only in-system moves are
(a) make change they may not have, or (b) tap `refused_payment` — a *lie*, the customer *did* pay. The
exact-equality rule is correct; its **missing honest twin** — a "paid full + change/tip" affordance so the
courier never has to choose between making change and lying — is what makes it *feel* courier-hostile.

**Disposition (§C-3).** CARRY `cash===total` verbatim (do NOT relax to `>=` — that corrupts the HOLD; the
steel-man below lands here). But **name the tip/change affordance as the honest resolution** with an owner and
a trigger — a product decision, NOT a new money input threaded into the byte-parity port (the S5 lesson: do
not open a money input in the tx you are freezing). And require the 422 to reach the courier as a **legible
affordance** ("cash doesn't match — collect the full ALL X, or mark refused"), not a raw `CASH_AMOUNT_MISMATCH`
(the D6 error-envelope disposition, post-Astro FE-lockstep). The honest server rule deserves an honest
courier-facing surface.

### 3. Courier money — the shared ledger, the 085 landmine, and the coupling the packet loses (Q3 🔴)

**Affirm the DB-fn boundary and the one-canonical-DTO; insist on the 085 forcing function; and name the
coupling to `/regenerate`.** Three sub-charges:

- **Shared ledger, no leak.** Verified (§note): the courier reads only their own payout rows, with the
  stricter PII redaction the packet keeps (S7-T8/T9). One canonical DTO, role-scoped projection — a courier
  seeing a *different* amount than the owner approved would be a trust-breaking money bug, and the packet
  closes it. Affirm. The tasking's fear ("courier sees another's payout / owner sees what they shouldn't") is
  closed at the read on the courier side.
- **The 085 watermark is the one live, silent, real-people money harm on this surface — give it teeth here,
  where it lands.** The S5 seat already lifted it out of a footnote into an operator-owned timing gate with a
  pre-apply assertion (`literal >= apply_date`). S7 is the surface that *reads and generates* these payouts,
  so S7's DoD must **inherit that gate, not re-footnote it**: "the 085 watermark is verified before any
  settlement apply" (§11) must point at the S5 forcing function, not restate it as prose. Direction:
  erring EARLY double-pays; the over-paid courier does not complain, so nothing surfaces it —
  `готівка → алерт-тертя` applied to the payout side.
- **`/regenerate`'s safety is *conditional on 085*, and 085 has not landed.** The packet carries the
  cross-tenant blast as "idempotent under 085, a redundant sweep is a no-op … *once 085 lands*." That
  parenthetical is the whole safety argument, and today it is **false**: before the 085 apply, `/regenerate`
  sweeps ALL tenants through the **pre-085 buggy fn** — the exact SKIP-LOCKED-loss / non-idempotent-re-run
  paths 085 exists to close, now fanned across every tenant. So the two "carry" rows the packet treats as
  independent (Q-CROSS-TENANT-REGEN and Q-085) are **coupled**: the blast is only a no-op *after* the
  watermark gate is satisfied. Handled in §4 (make it an explicit, owned, *085-gated* accepted-risk).

### 4. Honest-dispatch, `/regenerate`, and the PII split — the three "explicit or nothing" items

- **Honest-dispatch — the ethical pillar holds.** `dispatch.ts:27-42` filters `c.status='active' AND
  cs.status='available' AND courier NOT IN (active-binding set)` and, on no courier, **stays put**
  (`{dispatched:false, reason:'no_courier'}`) — never advances a paid order to IN_DELIVERY without a real
  courier bound. The synthetic/dev courier is excluded from the roster (`couriers.ts:40`) and admissible only
  under the ADR-0003 gate. This is the packet's ethical spine — no customer is shown a fake courier, no order
  is orphaned — and the port preserves it (S7-T5). Affirm loudly. The one thing that must not slip: the
  synthetic exclusion + the find-then-advance ordering are **red-line CARRY**, not optimisation candidates.
- **`/regenerate` cross-tenant blast — explicit, owned, and 085-gated, never a silent carry.** The tasking
  asks "is 'explicit owned accepted-risk' enough, or is friction needed?" My answer: it is enough **only if
  the accepted-risk carries three things** the current row lacks — (i) a **named owner** (operator, product;
  the route is S5-path-owned), (ii) the **085 pre-condition stated in the row** ("no-op *after* 085; before
  085 it runs the buggy fn across all tenants"), and (iii) an acknowledgement that it **contradicts the
  tenant-isolation invariant** the rest of the system (S6's whole reason to exist) enforces — one tenant's
  action ranging over all tenants' money is architecturally the opposite of the isolation the platform sells.
  Not a stop (money direction is platform-out, operator-owned, S5-path, idempotent post-085), but the row must
  say all three, or it is a silent carry of a cross-tenant money write, which is exactly what the packet
  itself warns against.
- **The PII split — ratify the *actual* scope, and ask the minimality question.** The role-tier is
  defensible: the courier is masked (prevents a courier harvesting a customer contact list from their
  history — a genuinely pro-privacy design for the *customer*), the owner is plaintext (the merchant/data
  controller who must call the customer and resolve disputes). I **ratify the tier**. But two verified
  sharpenings: (i) the owner exposure includes **`delivery_address`**, not just name/phone — ratify over the
  real scope; (ii) the owner `/details` returns **20 orders of plaintext PII in perpetuity** — the minimality
  question nobody asked is whether the owner needs the *full standing history* in plaintext, or whether
  older-than-active rows could be masked like the courier's, with plaintext reserved for the active/recent
  window where disputes actually happen. Not a stop (owner is the legitimate data controller); a ratification
  that names the real scope and records the minimality question, so the tier is a *decision*, not an
  inheritance.

### 5. Charter, scope, and the real people

- **Charter: clean.** Courier operations, cash, and pay. No military/warfare, no surveillance-for-harm, no
  commons-capture. The one surveillance-adjacent asset — the courier's live location — is *consent-gated to
  active delivery* (`courier-gps.ts`) and *retention-purged*, and the only PII on the realtime bus is
  claim-check-free (S6). `нуль-PII-у-ШІ` is not in play (no AI path). The Charter's dignity spirit is
  *realised* in the GPS consent gate, not merely un-violated.
- **Scope: disciplined.** Settlement generation (085 + cron) is S8/DB; the order state machine +
  `updateOrderStatus` is S5; WS transport is S6; the offer-sweep workers are S8; no schema change. The FIX-IN-
  PORT set (the shifts.ts D1/D2 single-writer collapse, the broken tenancy seats, D6 typed errors) is the
  *honest* direction — re-shipping the worst-health file's arbitrary-row shift bug (`shifts.ts:196-203`) as
  "parity" through a deliberate rewrite would be the neglect-laundering the S5 seat named. The port is the
  clean moment to collapse three shift writers into one; take it. One scope watch-item: keep the seat-fix
  census **complete** — `me.ts` `/me/earnings` + `/me/history` are *also* bare-pool no-seat reads
  (`me.ts:187-223,249-264`), not just `settlements.ts`; the NOBYPASSRLS probe must cover them or a courier's
  own earnings/history 500s post-B3. (Breaker/architect turf; flagged, not owned.)
- **The three real people, by who-bears-the-cost.** The **customer** is protected *from* the courier
  (masked history, address only during the active task) — well-built. The **owner** gets the roster,
  live map, route replay, and settlement write-lifecycle — full agency. The **courier** bears the
  asymmetry: tracked, lower-trust, read-only over their own pay, no in-system voice to contest it (§7).
  Every seat speaks for the *frame/row/tap being correct*; §§1–4 are where I make the council also speak for
  the *person on the road*.

---

## Non-blocking aesthetic / strategic notes

- **The GPS consent gate is the design-language high point — say it out loud.** `ACTIVE_DELIVERY_ASSIGNMENT_
  STATUSES = ['accepted','picked_up']`, with a code comment that *names the courier's accept as the consent
  act* and forbids a future refactor from re-adding `'assigned'` (`courier-gps.ts:3-9`). This is aesthetics
  doing its ethical job: the harmful thing (pre-consent tracking) is made *hard to re-introduce* by a comment
  that reads like a standing instruction to the next engineer, and the retention purge means the courier's
  movement is the *most*-forgotten data on a surface built to remember everything else. A whole design treats
  the least-protected human's location as consent-in, retention-out. Affirm it, and port the comment with the
  constant — the comment is load-bearing.
- **"Schema-rich, runtime-minimal" is at its honest best here — the money engine is Postgres.** The port
  issues `SELECT app_generate_settlements($1,$2)` and never re-derives the aggregation in Rust (Q3a). This is
  the *good* twin of the doctrine (the S5 seat named its evil twin, the Potemkin promotions UI): the runtime
  the port does not build is the one place a double-pay could be re-introduced. Keep the port a thin caller;
  the restraint is the safety.
- **The single-completion primitive is the coherent-recovery high-point.** Both the courier `/delivered`
  and the owner-proxy `/deliver` funnel through one `completeDelivery` (`deliveryCompletion.ts`), so the
  cash-proof HOLD + `payment_outcome` + immutable trace are *structurally* guaranteed on every delivered
  order — the owner-proxy path can no longer skip them. One primitive, one truth about what happened at the
  door. This is the same "make the invariant unrepresentable" elegance the S6 seat praised in the relay
  chokepoint. Affirm.
- **Name the courier-app's honesty debt as a strategic item, not just a bug.** The whole surface is
  server-authoritative — the server decides the cash, the outcome, the pay, the shift, the session. That is
  *correct* (the courier cannot be made to lie about a charge). But a server-authoritative surface over a
  lower-trust human is only *dignified* if the human can *see why*: a raw 422, a silent 401-on-deactivation
  mid-shift, a payout number with no "this looks wrong" lever. The strategic risk over a year is not a money
  bug — it is a courier who experiences the app as opaque and coercive and churns. The legibility work
  (§C-3, §C-5) is what makes the server-authority *feel* like fairness rather than control.

---

## Steel-man of a rejected option (obligatory)

**Q4 option (b) — "relax `cash===total` to `cash>=total`" — the option the packet rejects and I land
against, steel-manned from the *courier's* side, not the money-leak side the packet argues.**

Its strongest case, made fairly: the packet rejects `>=` as "partial-handover money leak; over-collection is
a customer dispute." That frames `>=` as *courier-permissive-toward-fraud*. But read from the road, the
exact-equality rule is the one that puts the courier in a bad spot — in a **cash economy with informal tips
and no small change**, a customer handing "2000, keep it" for a 1970 order is *routine*, not adversarial, and
`cash===total` refuses the honest close: the courier must make change they may lack or tap a `refused`
outcome that is a lie about a customer who *did* pay. `cash>=total` would let the courier close the delivery
truthfully in the exact case the equality rule makes awkward. The steel-man's real force is not "let couriers
over-collect" — it is "**the system should not force an honest courier to lie or make change to close a
delivery a paying customer completed.**" That is a genuine dignity argument for the least-protected human, and
I do not dismiss it.

**Why I still land against `>=` — and adopt the steel-man's insight instead.** Two things the steel-man
underweights, both load-bearing on the money ledger. First, `>=` **corrupts the HOLD**: the cash-as-proof
HOLD writes `amount = cashAmount` as the courier's till-debt (`deliveryCompletion.ts:118-123`); under `>=`
the courier is booked as owing the *over-collected* 2000, not the 1970 they owe the till — so `>=` does not
*relieve* the courier, it *over-charges* their till-accountability. Second, the excess has nowhere honest to
go under `>=` — it silently inflates the HOLD instead of being recorded as a **tip** (a separate line the
courier should *keep*, not owe). So the steel-man is right about the *destination* (the courier must be able
to close a "keep the change" delivery honestly) and wrong about the *vehicle* (`>=` books the tip as a
till-debt against the courier — the opposite of the relief it seems to offer). The correct resolution is the
one the exact-equality rule *points at by its absence*: a **"paid full + change/tip" affordance** where
`cash === total` closes the delivery, the excess is recorded as a tip the courier keeps, and no lie is
required. I therefore **adopt the steel-man's urgency into §C-3** — name the tip/change affordance with an
owner and a near-term trigger — while keeping the port a port (carry `cash===total` verbatim; the affordance
is a product decision, not a money input threaded into the byte-parity tx). `>=` loses on the HOLD it
corrupts; it is *right* that the courier's honest over-payment case deserves a home.

---

## Explicit conditions before code (§C)

Advisory — the human decides. Counsel-weighted, ordered by ethical load.

1. **[dignity/privacy, §1 — the sharpest CARRY] Carry the session-liveness bind; mark JWT-only a forbidden
   downgrade; prove parity across stacks.** AFFIRM Q1(a) — the per-request `courierSessionValid` re-read (no
   cache). Record Q1(c) (verify-JWT-only) as a **forbidden downgrade** on the packet, so no future
   "auth simplification" quietly re-opens the 14-day tail over a real person's GPS + cash + pay. Require the
   session-liveness golden test to run **against both stacks** during overlap (revoke → 401 on Node *and*
   Rust), the S6 REV-S6-2 predicate as the shared spec. This is CARRY, not FIX-IN-PORT — the fix exists; the
   port must not lose it.
2. **[money/timing, §3 — the live landmine] Inherit the 085 forcing function; state the `/regenerate`
   coupling.** S7's DoD line "085 watermark verified before any settlement apply" must **point at the S5
   forcing function** (the pre-apply `literal >= apply_date` assertion, S5 `counsel-opinion.md:281-284`), not
   re-footnote it — S7 is the surface it lands on. And rewrite the Q-CROSS-TENANT-REGEN accepted-risk row to
   state the **085 coupling explicitly**: "a no-op *only after* 085 lands; before 085 an owner-triggered
   `/regenerate` runs the buggy fn across ALL tenants," with a **named owner** (operator + S5 lead) and an
   acknowledgement that it contradicts the platform's tenant-isolation invariant. Six days to the watermark
   today — treat both as live.
3. **[fairness/legibility, §2 — the courier's honest close] Carry `cash===total`; name the tip/change
   affordance; make the 422 legible.** CARRY the exact equality + the no-cash tail verbatim (do NOT relax to
   `>=` — it corrupts the HOLD). But record the **missing "paid full + change/tip" affordance** as an owned
   product item with a near-term trigger, so the courier is never forced to make change or lie to close a
   delivery a paying customer completed. And require the 422 to surface to the courier as a **legible
   affordance** ("collect the full ALL X, or mark refused"), not a raw `CASH_AMOUNT_MISMATCH` (the D6
   error-envelope work, post-Astro FE-lockstep). The honest server rule needs an honest courier-facing face.
4. **[privacy/ratify, §4] Ratify the PII split over its *actual* scope, and record the minimality question.**
   The role-tier is ratified (courier masked = pro-customer-privacy; owner plaintext = data controller). But
   ratify over the **verified scope**: owner `/details` returns plaintext **name + phone + `delivery_address`
   × 20 orders in perpetuity** (`couriers.ts:274-284`), broader than the packet's "name/phone." Record the
   **minimality question** the council should answer: does the owner need the *full standing* plaintext
   history, or can older-than-active rows be masked like the courier's, plaintext reserved for the
   active/recent dispute window? A decision, not an inheritance.
5. **[voice/agency, §7] Give the courier an in-system way to flag a wrong payout — or name its absence as an
   owned gap.** The courier settlement surface is **read-only**; the `disputed` status is **owner-written**
   (owner/settlements.ts). A courier who believes they are underpaid has **no in-system lever** — only
   out-of-band recourse. Byte-parity guarantees the courier sees the *same* number the owner approved; it does
   not guarantee that number is *right*, and the person who bears a wrong one has no voice in the system. Add a
   minimal courier-side "flag this payout" affordance (writes `disputed`, or a review request), OR record its
   absence as a **named, owned gap** with a trigger — not a silent asymmetry where the owner has write-agency
   and the courier has a window.

---

## The question nobody asked (§7)

Every seat in this council measures S7 from the **row, the frame, the tap being correct for the right
principal**: the actor-gate so no courier hijacks another's delivery, the session bind so a revoked courier
is dropped, the byte-parity so the courier sees the amount the owner approved, the cash assert so no partial
handover slips. Every control speaks for the courier's *data being handled correctly*.

**Nobody speaks for the courier's *agency over the decisions the system makes about them*.** The whole
surface is, correctly, server-authoritative — the server decides the cash, the outcome, the shift state, the
session, the pay. On every one of these the courier is a **subject, not a party**: they can *see* their
payout but the write-lifecycle (approve / pay / **dispute** / reopen) is entirely the owner's; they can be
*deactivated mid-shift* and discover it only as a 401 on their next tap; they can hit a 422 at a customer's
door with no path but "make change or mark refused." The tasking named the shape of it — *"недоплата → курʼєр
постраждає мовчки"* — and the code confirms the silence is **structural**: the courier's money surface is
read-only, so the person the packet calls lower-trust and the tasking calls least-protected has, by design, no
in-system way to say *"this is wrong."* Byte-parity makes the courier's view *accurate*; it does nothing to
make the courier *heard*.

The unasked question is not technical and it does not block the port: *the surface works hard to guarantee the
courier is paid the amount the owner approved — what guarantees the courier a voice when that amount is wrong,
or a legible reason when the app tells them no?* The honest answer is the same shape as the customer's
protection: give the least-protected human a lever, not just a window — a payout-flag, a legible 422, a
tip/change close, a reason for a mid-shift lockout — so the person on the road who cannot attend this council
is not the one who discovers, in silence, that the system decided against them and left them no way to answer.
That sits on §C-3/§C-5 with owners and triggers, so the courier is a party to the decisions the system makes
about their body's location and their day's pay — not only their subject.

---
*Advisory only. No ETHICAL-STOP. The human is final. Nothing here authorizes a code change, blocks S7, or
overrides a conscious operator decision.*
