# Counsel Opinion — `courier-realtime-authz`

**Role:** Counsel (advisory). This is a lens-pass, not a gate. Human is final.
**Verdict in one line:** This plan *heals* red-line crossings rather than creating them. No grounded
ETHICAL-STOP on the plan as written. One conditional friction (R1) and one strategic caution (the
deferral horizon) below. Adopt Option A.

I verified the central claims against source before writing:
- `apps/api/src/websocket.ts:185-195` — courier branch authorizes by prefix + `courier:<sub>` only;
  `location:`/`order:` pass through unchecked, while owner (`:181`) calls `ownerCanAccessRoom`. The
  gap is real.
- `:220-253` — the customer GPS relay fans `client_location` to **every** courier member of the
  order room. So an unauthorized `order:` subscribe is a live customer-GPS leak, exactly as claimed.
- The customer branch (`:169-174`) is already correctly self-scoped — confirms N1 is REST-only.
- Courier FE subscribes **only** `courier:${courierId}` (TasksPage:68) and `order:${id}`
  (DeliveryPage:121). It never subscribes a `location:` room — so Option A's "DENY all `location:`
  for couriers" forecloses *nothing the product actually uses*.
- R1 is real and load-bearing: DeliveryPage passes the route param `id` to BOTH
  `/courier/assignments/${id}` (DeliveryPage:93) and `order:${id}` / `/orders/${id}/messages`
  (:121, :73). One of those two endpoints is keyed on a different identity unless `id` is the order
  id everywhere. Binding-scoped authz keys on `order_id`; if the FE ships the assignment id, a
  *legitimate* courier is denied their own run.

---

## 1. Reasoning by lens

**Justice / stakeholders.** The redistribution here is almost purely virtuous: the cost falls on the
attacker (a courier probing rooms they have no binding to) and the benefit accrues to the two
weakest, least-consenting parties — the **customer** whose live GPS + chat stops leaking to strangers,
and the **honest courier** whose runs stop being watchable by colleagues. One indexed point-read per
subscribe is the only cost, borne by the platform's own pool. This is least-privilege working as
distributive fairness, not as austerity.

**Dignity / autonomy (courier).** Two-sided, and both sides land right. The fix *protects* courier
dignity — your delivery, your customer's location, your message thread are no longer ambient data for
the shop's other riders (a quiet anti-surveillance win between peers, not just against outsiders).
And it does **not** strip a legitimate courier's agency: the `offered` status is inside the predicate,
so the offer-handshake still lets a courier *open and read* an order to decide accept/decline — the
"courier completes / courier decides" red line is preserved. The send-gate (`assigned`+ only, not
`offered`) is the correct asymmetry: read-to-decide is dignity, write-before-accepting would be
coercive presence on someone else's order.

**Honesty / consent.** Server-authoritative throughout (`sub`/`activeLocationId` from RS256, room
string treated as untrusted). Fail-closed on DB error. Denials carry no order/tenant detail (REST 404
non-enumerable). No dark pattern, no soft-confirm-as-trap. The UI tells the truth because the server
is the only authority. Clean.

**Care / harm.** Who does a failure hurt? A *false deny* hurts a real working courier mid-shift (can't
see their drop-off) — which is why R1 matters more than its "robustness bug" label suggests (see §2).
A *false allow* hurts a customer (GPS/PII to a stranger) — the status quo. The fix moves harm from the
non-consenting customer onto a bounded, observable failure mode (deny → retry/soft state). Friction,
not punishment.

**Long horizon / strategy.** Highly reversible (pure code, revert the commit). It sets the *correct
precedent*: "a courier's authority IS their binding," mirroring `ownerCanAccessRoom`. That conceptual
parity is the strategic asset — the next person reasoning about authz finds one shape, not two. The
thing I'd regret in a year is **not** this fix; it's a deferral list (B7/N1) that quietly never ships
(§3, caution).

**Aesthetics / integrity.** This is the elegant kind of fix: one predicate (`courierHasBinding`),
one source of truth, used by both WS and REST so the two surfaces *cannot* drift into a
"blocked-on-WS, open-on-REST" asymmetry. Rejecting the Option C "pre-offer browse" fallback as
"collapses to A with extra attack surface" is real restraint — the elegance is in what was *not*
built. "Schema rich, runtime minimal": no migration unless `EXPLAIN` demands it. Honest and coherent.

**Epistemic.** The proposal already steel-mans its rejections and names the eviction race (R2). The
one assumption I'd surface: see the open question (§5).

---

## 2. ETHICAL-STOP (grounded red lines)

**None on the plan as written.** Option A crosses no grounded red line; it *closes* crossings of
nul-PII (customer GPS), server-authoritative, and intra-tenant dignity. ETHICAL-STOP is friction on a
real crossing — there is none here.

**One conditional friction — fire only if it materializes (R1, courier-completes / dignity).**
If the implementing PR ships binding-scoped authz **without** reconciling the order-id vs
assignment-id conflation (proposal R1), the predicate will *deny legitimate couriers their own active
delivery*. That is a new injustice manufactured by the security fix — a courier locked out of the run
they were assigned. It touches the "courier always completes" line from the wrong direction
(over-restriction, not over-control). **Disposition:** not a STOP now because the architect already
flagged R1 as FIX-in-PR. It *becomes* a STOP only if the PR lands authz before the FE id-reconciliation
and the positive-control E2E (proposal §11 item 4) is hand-crafted with a synthetic room instead of
**driving the real `/courier/delivery/:id` FE route** — because a synthetic test would mask exactly
this denial. Recommendation: make E2E item 4 navigate the actual courier FE, so R1 cannot pass green
while breaking a real courier.

---

## 3. Non-blocking advice (aesthetic / strategic)

- **Give the deferral a horizon, not just an owner.** B7 and N1 are correctly deferred for
  blast-radius hygiene — I endorse the scoping. But the risk table assigns an owner and "track now"
  with **no date and no severity tag**. N1 is a *confirmed-shape* customer-PII gap (the customer WS
  room is already scoped, so N1 is the REST sibling — a known-live exposure being knowingly left
  open). A deferral without a deadline is how "leak-tolerance-creep" sets in: the urgent fix ships,
  the dignity-equivalent sibling sits forever. Ask: file B7/N1 as tracked tickets with a severity and
  a target window in this same PR, so the deferral is a *scheduling* decision, not a quiet permanent
  acceptance.
- **The DENY counter (proposal §9) is also a dignity instrument, not just security telemetry.** A
  spike of courier-authz DENYs could equally be a *broken FE over-denying honest couriers* (R1
  regressing) as an attacker probing. Label/alert on it from both readings, or you'll mistake your own
  users being locked out for an attack.
- **Name the in-process-bus assumption out loud in the ADR.** The R2 eviction reuses the
  "single-instance in-process bus." That is true today and is the right minimal choice — but it is a
  latent scaling coupling. One ADR line ("eviction correctness assumes single-instance bus; multi-
  instance fan-out would need a cross-process evict signal") saves a future reader from discovering it
  during an incident.

---

## 4. Steel-man of the rejected Option B (location-scoped)

The proposal rejects B as "coarser, re-opens the colleague leak." Here is its strongest honest case,
fairly stated: **In a genuine multi-courier shop, situational awareness is a real operational good.**
A rider who can see the shop's live board — which orders are out, where colleagues are, who is
stacked — can self-organize handoffs, cover a stalled drop, and reason about their own queue without
the dispatcher micromanaging. Tight per-binding blinkering optimizes for privacy at the possible cost
of *fleet coordination and courier autonomy-as-a-team* — and there is a dignity argument that adult
workers coordinating among themselves beats a model where every rider is an isolated, individually-
leashed node who only ever sees the one task handed to them. B is the "trust the team" design; A is
the "trust no peer" design.

**Why A still wins — but note the cost.** The decisive fact is empirical, not philosophical: the
courier FE **has no situational-awareness surface today** — it consumes only `courier:<sub>` and the
single `order:<id>` it is working. So B would not *enable* coordination; it would only *expose*
customer GPS and chat to riders with no UI that uses it — privacy cost with zero coordination benefit.
Building authz around a coordination feature that doesn't exist is YAGNI, and it would re-open the
exact leak we're closing. **But the steel-man leaves a residue worth keeping:** A encodes a *product
stance* — "couriers are isolated task-executors, not a coordinating team." If the product ever wants a
real situational-awareness board, the answer is **not** to loosen `order:` authz back to location-
coarse (which leaks customer PII), but to build a *purpose-built, PII-minimized* `location:<id>:couriers`
feed (positions/availability, no customer GPS, no chat) that A's "DENY all location: for couriers"
currently forecloses. That is the right future shape — and A doesn't prevent it, it just defers it
honestly. So A is correct *and* the steel-man names the door we're choosing to leave shut.

---

## 5. The open question nobody asked

The whole design treats the courier's binding as the unit of authority — elegant and correct for
authz. But **the customer never consented to any specific courier; they consented to "a delivery."**
When order O is reassigned A→B (R2), customer GPS that streamed to courier A's device for ten minutes
does not get un-seen by the eviction — eviction stops *future* frames, not the human memory of where
someone lives. We are scoping the *channel* tightly while the *data already delivered to a now-
unbound courier's screen* has no lifecycle at all.

So: **does a reassigned-away courier's app still hold the customer's last-known location, address, and
chat — and should a binding terminalization actively instruct that client to purge it, not just stop
the feed?** Nobody in the proposal asks what the courier's *device* retains after the binding ends.
That is the difference between "we stopped sending" and "we ensured they no longer hold it" — and for
customer location data, the second is the one the dignity/privacy red line actually cares about. Worth
one sentence of explicit disposition (likely: out-of-scope-here, tracked) rather than silence.

---

# Revision 2 — RE-EXAMINE (2026-06-29)

I read the UPDATED `proposal.md`, `resolution.md`, and ADR-0013, and re-verified two device-side facts
in source. **Verdict: clear to GO. Zero ETHICAL-STOP — prior or new.** The redesign is a strict,
monotonic privacy improvement; the residual windows it introduces are all *smaller* than the unbounded
status quo and fall on the least-vulnerable population.

## A. Prior points — all addressed, confirmed

- **R1 / §2 friction → HONORED as a hard gate (H2).** The positive-control E2E must drive the REAL
  `/courier/delivery/:id` route; synthetic room BANNED; FE `order:${task.orderId}` ships atomically.
  My conditional friction can no longer materialize green. Resolved.
- **§3a deferral horizon → HONORED.** N1 = HIGH / 2 weeks, B7 = MEDIUM / 1 month, filed with severity +
  window in proposal §10, resolution, and ADR "Deferred." Deferral is now scheduling, not silent drift.
- **§3b DENY-counter dual reading → HONORED (L1).** DENY labeled (probe vs. over-deny) **plus** a new
  eviction counter for the actual leak class. Good — they noticed the leak emits no DENY.
- **§3c single-instance bus coupling → RESOLVED by construction.** Per-instance fan-out revalidation
  removes the coupling I flagged; stated explicitly in the ADR. Better than my "name it in one line"
  ask — they eliminated it.
- **§5 device-retained PII → DISPOSITIONED, and now evidence-backed (see C below).** In-PR FE purge of
  rendered GPS/address/chat on `binding_revoked`; persistent-storage audit deferred LOW/next sprint.

## B. The three weighed questions

**(1) The ≤10s TTL leak-window — NOT a residual ETHICAL-STOP. Acceptable posture.**
Reframe who the "leaker recipient" is: in the dominant case (owner-reassign / decline / sweep) the
courier still receiving frames for ≤TTL is the one who was, seconds earlier, the **legitimately-bound
holder** of that exact customer GPS/address/chat. The window does not open access to a never-authorized
stranger; it bounds the *closing* of access to a just-deauthorized party. The marginal new information
in ~10 frames at 1/s — for a customer typically stationary, awaiting delivery, whose address the
ex-courier already legitimately knows — is near-zero. Against the **status quo (unbounded leak, to
anyone, including cross-tenant strangers)**, this is a categorical improvement. ETHICAL-STOP is friction
on *opening* a grounded crossing; this *shrinks and bounds* one. No STOP.
- *Non-blocking nudge:* the optional cache-bust accelerator is currently "additive / non-load-bearing."
  For the **involuntary** owner-reassign path specifically (where the courier did *not* choose to leave —
  the least-defensible 10s of continued customer GPS), make the cache-bust a default-on SHOULD, not a
  maybe. It is already designed, costs one `cache.delete(orderId)`, and shrinks that one path's window to
  broadcast-latency. Self-decline/sweep can stay at TTL.

**(2) Deferring the persistent-storage purge audit — HONEST scheduling, now evidence-backed; NOT
leak-tolerance-creep.** I checked the actual persistent surfaces rather than trust the LOW label:
- `apps/api/public/sw.js` explicitly **excludes `/api/` and `/ws/`** from caching (`pathname.startsWith("/api/")||...startsWith("/ws/")` are skipped) — so customer PII from API responses **never enters the SW cache by construction**. The SW caches only the app shell.
- `apps/web/src/pages/courier/DeliveryPage.tsx` (the page that renders customer GPS/address/chat) has **zero** localStorage / sessionStorage / IndexedDB / caches writes — customer data lives in React state only, which dies on unmount/navigate. The in-PR FE purge (clear state + navigate away) is therefore the complete and correct live-session remedy.
- Courier `localStorage` use (TasksPage) is the **access token only**, not customer address/GPS/chat.
  → The deferred audit is verifying a surface that present evidence says is already empty of customer PII.
  That makes LOW an *honest* label, and the in-PR rendered-purge the *sufficient* action. I withdraw my
  Q2 suspicion. **One refinement:** label the ticket "LOW — confirm-empty" and have its first step be the
  cheap grep-triage above, so the LOW rests on a re-check at audit time, not on today's snapshot (a future
  offline/PWA-caching feature could change this surface). Honest scheduling, not creep.

**(3) Cost-shift in the tri-state/cache — broadly fair, with ONE honesty correction owed to the spec.**
Cost distribution is just: the attacker / ex-bindee bears denial; the honest courier bears only
*recoverable* friction (a retryable soft error during a blip, ≤5s jittered reconnect, a few-hundred-ms
semaphore queue); the customer bears a residual leak strictly smaller than status quo. No billing/cost is
shifted onto the customer. **But:** the proposal/ADR claim "worst-case leak window = ≤TTL on **every**
path" is **slightly overstated**, and the gap lands on the customer.
- The revocation path *fail-SAFEs* on `UNAVAILABLE` (do NOT evict, retry ~2s) — correctly, so a DB blip
  can't mass-evict legitimate mid-delivery couriers. But `UNAVAILABLE` cannot distinguish a
  blip-affected *legitimate* courier from a just-*revoked* one. So during a sustained DB/pool-exhaustion
  event (the documented operational-pool starvation incident is exactly this), a courier whose binding
  terminalized *during* the incident **keeps receiving customer GPS for the incident's duration**, not
  ≤TTL. The window is `≤TTL` under DB-availability; `≤(incident duration)` for the narrow set revoked
  during a DB-unavailable window.
- This is a *defensible* fail-safe trade (you genuinely cannot tell the two populations apart without the
  DB, and mass-evicting honest couriers is worse) — so **not a STOP**. But the spec should state the bound
  *honestly*: "≤TTL under DB-availability; bounded by incident duration for bindings revoked during a
  DB-unavailable window." Truth-in-spec, since the inflated claim is the kind a future reader trusts.
- *Non-blocking option (the symmetric-dignity move):* cap the fail-safe — evict an already-admitted member
  after a hard residual ceiling (e.g. N consecutive `UNAVAILABLE` or ~60s), even under `UNAVAILABLE`. This
  re-bounds the customer-PII leak during a long incident, at the cost that a legitimate courier hitting a
  >ceiling blip is bounced and must re-subscribe (recoverable). For customer location data that trade
  favors the customer — the right asymmetry. Offer, not mandate.

## C. Residual concerns — blocking? No.

No remaining blocking concern. Two **non-blocking** items worth a sentence each in the PR:
(1) default-on cache-bust on the involuntary owner-reassign path; (2) correct the "≤TTL on every path"
claim to name the DB-unavailable exception, optionally with a hard fail-safe ceiling. Both are honesty/
polish refinements on an already-sound design. **Clear to GO.**

---

*Counsel is advisory. Aesthetic/strategic notes are non-blocking. The R1 friction is a watch-condition,
not a veto. The human operator decides.*
