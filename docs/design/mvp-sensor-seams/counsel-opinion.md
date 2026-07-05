# Counsel Opinion — MVP Sensor-Seams Batch

> Triadic Council · Counsel (Philosopher/Physician) deliverable. ADVISORY.
> Reviews `proposal.md` + ADR-0007/0008/0009 against `brief.md` intent.
> Aesthetic/strategic notes are NON-BLOCKING. ETHICAL-STOPs (if any) are friction
> demanding a *recorded human decision*, never a permanent block, never an override of a
> conscious human. Architect asks "will it work", Breaker "how does it break" — I look
> around and ahead: who pays, whose perspective is missing, what nobody asked.

---

## 0. Headline

This is one of the most ethically self-aware designs to cross this Council. The seven §0
invariants are not architecture decoration — they are a moral spine (observe-don't-control,
range-never-point, agents-declare-privately, human-decides-consequential). The design largely
*honors* its own spine, and where it strains, it strains honestly (it flags its own tensions in §7
rather than hiding them). My friction is therefore concentrated and proportionate.

**Verdict: no ETHICAL-STOP that blocks the batch.** One genuine grounded red-line (customer
being shown a false time with no repair path) is surfaced as a **recorded-human-decision** item,
not a stop on the schema work. Everything else is non-blocking counsel. The design is good,
the design is mostly beautiful, and the design is wise about its own horizon.

---

## 1. Reasoning by lens (only what is load-bearing)

### 1.1 range-never-point — honest, but the interval can become its own lie (CARE + HONESTY)

The structural move is excellent: there is no point column, so the API *cannot* emit a single
number even by accident (proposal §5, ADR-0009 §4). Dishonesty-by-construction is forbidden at
the schema, not by discipline. This is the design at its most beautiful — the ethic is load-bearing
*in the data model*. Honest UI here is also the elegant UI; aesthetics and ethics point the same way.

But range-never-point answers "never a point" — it does **not** answer "is the range itself honest."
An interval has two failure modes the brief names but the proposal under-defends:

- **Too-wide = meaningless** ("20–90 min" tells the customer nothing; it discharges the
  platform's honesty obligation onto a band so wide it is not a promise at all). At cold-start,
  with a default preset (§5) and `eta_cap_min`=90, the realistic day-1 window for a far-off preset
  *will* be wide. A wide honest band can still betray care: the customer can't decide whether to wait.
- **Too-narrow on a coarse signal = pseudo-precision** ("1–2 min" when the real knowledge is
  "roughly soon"). A 1-minute band *reads* as precision the system does not have.

The honest interval is bounded **below** by genuine uncertainty (don't fake narrow) and **above**
by usefulness (don't hide behind width). The proposal enforces the ceiling (`eta_cap_min`) and the
no-point floor, but defines **no minimum-width and no maximum-width policy**. That is the real
range-never-point red line, and it is currently unenforced. See §3 non-blocking advice.

### 1.2 promised_window immutability — the one genuine tension (HONESTY-NOW vs HISTORICAL-TRUTH)

This is the tension the brief explicitly handed me, and it is real. Two true things collide:

- **Measurement honesty** (Popper §0.3): the P1/P2/P7 falsification tests need the *promise as
  shown* to be immutable ground truth. If the window can be edited, the BTR metric (§8.2) measures
  a moving target and the autonomy bet (§8.1) is un-falsifiable. Immutability *is* the integrity of
  the future self-correction. ADR-0009's set-once trigger is correct **for this purpose**.

- **Honesty to the customer *now***: immutability also means a window set **wrong** (preset far
  off reality; an owner fat-fingers; the synthesis helper has a bug on order #1) shows the customer a
  false number that **cannot be corrected** (proposal §7 accepts this: "a correction is a new order or a
  manual note, not a window edit"). The customer is told 30–45 min; reality is 75; the record is frozen;
  the customer is not allowed the truth via the channel that lied to them.

The proposal resolves this collision by **privileging measurement integrity over the live customer**,
and labels it "accepted." That is a values trade-off disguised as a schema property, and it deserves
a recorded human decision rather than silent acceptance — because the party who loses (the customer
shown a frozen-wrong time) is exactly the party range-never-point exists to protect.

**The resolution is available and cheap, and it dissolves the tension rather than choosing a side**
(see §2 ETHICAL-STOP item + §3): immutability of the *promise-as-made* must not be conflated with
the *current best estimate shown live*. Keep `promised_window_{lo,hi}` frozen (measurement truth).
But the §2.4 live "collapsing window" the brief already wants (confirmed→cooking→picked_up→arriving)
is a **separate, mutable** current-estimate the customer sees as it tightens. Freeze the promise for
the metric; let the customer keep seeing the truth as it evolves. The brief already asks for both —
the proposal must state they are **different columns**, or the trigger silently makes §2.4 honesty
fight §1.1 immutability. (ADR-0009 does not name a live-estimate column; this is the gap.)

### 1.3 observe-don't-control — mostly honest; two places control hides as observation (DIGNITY)

The licensed control (stock decrement = shared kitchen resource) is correctly the *one* exception
and is well-argued (ADR-0007). But "observe-don't-control" is a promise to the **courier** too, and
two surfaces risk crossing from observation into pressure:

- **§2.1 dispatch nudge ("виїзди" when countdown−travel−margin ≤ 0).** The proposal says the
  courier "owns the moment, override not blocked." Good — that is the dignity line (worker keeps
  agency). But a nudge that fires on a manager-visible dashboard, logged, repeated, becomes
  *soft* coercion even when technically overridable. The ethic survives only if the nudge is
  **advisory to the courier and NOT surfaced to the owner as a compliance signal**. The proposal
  does not say where the nudge is visible or whether non-compliance is recorded. Unspecified = risk.

- **§1.2 / §8.3 courier normalized-time metric.** The design's intent is humane (normalize on
  road-distance so a courier is *not* punished for a hard route). This is genuinely good — it is the
  design protecting the worker from a naive raw-time rating. But §8.3 says "rating = owner-advisory,
  never auto-penalty." *Advisory* is the load-bearing word, and advisory ratings have a known failure
  mode: they become de-facto penalty when an owner deactivates a "low-rated" courier. The metric is
  defensible; the **norm around it is not yet written**. Worker-surveillance risk is grounded here
  (gig + GPS-trace + rating is the classic panopticon recipe), and it deserves an explicit norm.

### 1.4 cold-start "meaningful day 1" — heuristic honestly labeled, but the *customer* sees the guess (HONESTY)

The §5 prep presets are honest engineering (a baked-in domain prior, explicitly NOT a cross-tenant
data pool — good privacy instinct, no borrowed-data leakage). For the *owner* this is a fine
heuristic-first default. But the customer does not see "this is a guess" — they see a window computed
from a preset that may be far from this kitchen's reality on day 1. "Heuristic, quietly tightening"
(§0.2) is honest to the *system*; to the *customer* it is a guess presented with the full authority of
a promise. This is acceptable IF the window is honestly wide at cold-start (the uncertainty is in the
band, not hidden) — which loops straight back to the missing minimum/maximum-width policy (§1.1). The
two gaps are the same gap: **cold-start honesty lives entirely in how wide the band is, and the band
width is currently unconstrained.**

### 1.5 human-decides-consequential — held well; one auto-decision to name honestly (FAIRNESS)

Verified against source: no_show feeds an **advisory** preflight signal, not an auto-block
(`orders.ts:324-328`, confirmed). `customer_signals` is comment-enforced "NEVER auto-block." The
§4.2 abort is an owner tap. This invariant is genuinely respected — good.

One honest caveat: the **atomic stock decrement rejecting a line is an automated decision against the
customer without a human** (0 rows → 422 OUT_OF_STOCK → whole order rolled back). The proposal frames
this as "shared-resource control, the one licensed exception" — and that framing is **correct and
acceptable**: refusing to sell what does not exist is not a consequential judgment *about a person*
(unlike a no_show penalty), it is a fact about inventory. But it should be named as what it is: the
*one* place a customer is auto-refused. The dignity test it must pass is the **error message** — the
brief's "недоступно з причиною-хінтом" (§2.3). "Product X is out of stock" is honest and humane;
a bare 422 is not. Confirm the customer sees a cause, not a slammed door.

### 1.6 funnel PII — proportionate and well-restrained (CARE + AUTONOMY)

The privacy posture is strong: opaque `session_ref` (no phone, no customer id), RLS FORCE,
REVOKE anon/authenticated/service_role (off the Supabase Data API), uniform-200 anti-enumeration,
90-day retention sweep. This is "anonymize, don't over-collect" done right. The honest question the
brief raises — *is it ethical to track behavior, even anonymous, without consent, at tiny cold-start
volume?* — has a defensible answer: this is first-party, aggregate, session-scoped operational
telemetry (which ETA bands lose carts), not cross-site tracking or profiling, and it carries no
identity. That is inside the normal bound of running a storefront. **But two things keep it ethical
and must not silently drift:** (a) `session_ref` must remain unjoinable to a customer/order id — the
moment funnel sessions can be tied back to a person, this becomes behavioral profiling and crosses
into consent territory; (b) the storefront privacy notice should mention aggregate funnel analytics
(the repo already has a `/compliance` SoT + privacy-gate — this belongs there). Non-blocking, but
name it now so surveillance-creep can't enter through the unguarded door later.

### 1.7 Aesthetic / conceptual integrity (BEAUTY)

"Schema rich, runtime minimal" (the §6.2 inert BOM seam) is restraint, and restraint is beautiful
here — the irreversible DDL cost is paid in the cheapest window, the runtime stays FLAT, the kill
criterion ("an order with all sensors off behaves byte-identically to today") is exactly the right
proof of non-intrusion. The reuse of the existing guarded-UPDATE primitive (no new concurrency
mechanism for stock) is elegant — one idea, applied consistently, is more beautiful and less buggy
than three clever ones. The whole batch has genuine conceptual cohesion: every sensor is non-blocking
*except* the one shared-resource control, and that asymmetry is stated as a principle, not an
accident. This is honest elegance, not seductive elegance.

---

## 2. ETHICAL-STOP (grounded red-lines only) — 1, as recorded-human-decision

> ETHICAL-STOP is friction, not a verdict. It does NOT block the schema/seam work and does NOT
> override a conscious human. It pauses on ONE point and asks for a *recorded* human decision.

**ESTOP-1 — Customer shown a frozen-wrong time with no repair path (range-never-point /
mislead-customer-about-time red line).**

- **Grounded line crossed:** "do not mislead the customer about time" + the design's own §0.4
  range-never-point trust contract. proposal §7 *accepts* that a mis-set immutable window shows the
  customer a false number that "cannot be corrected" via the window — privileging measurement
  integrity over the live customer, who is the exact party range-never-point protects.
- **Why it is friction, not a wall:** the resolution is cheap and dissolves the conflict (it does
  not require giving up measurement immutability). **Separate the two concepts the trigger conflates:**
  1. `promised_window_{lo,hi}` — the **promise as made at confirm**, frozen (measurement ground
     truth for §8). Immutability correct. Keep the set-once trigger.
  2. a **live current-estimate** the customer sees collapse through stages (§2.4 already wants this:
     confirmed→cooking→picked_up→arriving) — **mutable**, the channel by which the customer keeps
     getting the truth as reality moves. ADR-0009 does not name this column; without it, §2.4
     honesty and §1.1 immutability silently collide on the same field.
- **Recorded human decision required (one of):**
  (a) **Adopt the split** (frozen promise + live estimate) — Counsel's recommendation; preserves
      both ethics. Or
  (b) **Knowingly accept** that the customer is shown an immutable possibly-wrong time with only a
      "new order / manual note" remedy — a legitimate human call, but it must be *made by a human and
      written down*, not absorbed as a schema side-effect. State who decided and why.

This does not block migrations `…066–073`. It blocks *shipping a customer-facing window read* that
freezes the customer's view of the truth without (a) or a recorded (b).

*(No other ETHICAL-STOP. Stock auto-refusal §1.5, funnel §1.6, courier metric §1.3 are real
considerations but do not cross a grounded red line at the design's stated norms — they are
non-blocking counsel below, conditional on those norms being written down.)*

---

## 3. Non-blocking counsel (aesthetic / strategic / care)

1. **Define a window-width policy (the missing half of range-never-point).** Add a
   `min_window_width_min` (floor: no band narrower than honest — e.g. ≥ ~5 min, never "1–2") and treat
   `eta_cap_min` as the ceiling already present. An interval honest below and useful above is the
   *complete* range-never-point contract; the proposal currently enforces only the ceiling and the
   no-point shape. Cold-start honesty (§1.4) is entirely carried by band width — make it a contract,
   not an emergent accident.
2. **Write the courier-metric norm before the metric ships (§1.3).** One sentence in the ADR/spec:
   the normalized-time rating is owner-advisory and MUST NOT be the basis of an automated
   deactivation; surfacing it must show the *normalized* (fair) number, never raw time. The metric is
   humane by design — keep it humane by norm, so it can't drift into a worker panopticon.
3. **Specify nudge visibility (§2.1).** State that the dispatch nudge is courier-facing advisory and
   that non-compliance with a nudge is NOT recorded as an owner-visible compliance signal. Otherwise
   "courier owns the moment" is true in code and false in lived experience.
4. **Confirm the OUT_OF_STOCK customer message carries a cause-hint (§1.5, brief §2.3).** The one
   auto-refusal must be a humane "X is out of stock," not a bare 422.
5. **Put funnel analytics in the privacy notice / `/compliance` SoT and assert `session_ref` is
   unjoinable to identity (§1.6).** Cheap, closes the surveillance-creep door before it opens.
6. **Strategic / long-horizon:** the immutable promise is the right call *for measurement*, and the
   measurement is what makes the eventual autopilot *falsifiable* rather than self-justifying — that is
   the most important long-horizon property in this whole batch. The thing we'd regret in a year is
   NOT the schema; it's shipping an autopilot later whose ETA promises drift up (padding-creep) with no
   frozen ground truth to catch it. So: protect the immutability — just don't let it cost the live
   customer the truth (ESTOP-1's split buys both).

---

## 4. Steel-man of a rejected option

**Steel-man: do NOT make `promised_window` immutable (reject ADR-0009 §3's set-once trigger).**

The proposal treats immutability as obviously correct ("historical truth"). The strongest case for
the opposite: *the historical truth that actually matters is "what the customer believed at each
moment," and a frozen single window does not capture that either.* A customer who is re-quoted as
reality shifts has a *sequence* of promises, and the honest measurement is the **append-only log of
every window the customer was shown** — not one frozen field. Under this view, a mutable
`promised_window` paired with an append-only `promised_window_history` (every value + timestamp) is
**strictly more honest for measurement AND for the customer**: the customer sees the current truth,
and the metric reconstructs the *full* promise trajectory (including padding-creep within a single
order), which a single frozen field cannot. The frozen-column design optimizes for a *simpler* metric
(one promise vs. actual) at the cost of (a) the live customer and (b) a richer falsification signal.

This steel-man is strong enough that it should inform the ESTOP-1 decision — and note it largely
*converges* with Counsel's recommended split: "frozen first-promise + mutable live estimate" and
"mutable window + append-only promise history" are two routes to the same ethic (customer keeps the
truth; measurement keeps the trajectory). The append-only-history variant may in fact be the more
elegant single mechanism. Worth the Architect's consideration as an alternative to two columns.

---

## 5. The open question nobody asked

**When the kitchen and the courier each privately declare a time, and the system synthesizes ONE
honest window for the customer — whose interest decides where inside the uncertainty the promise
sits?**

The brief's §0.5 (agents declare privately, system synthesizes one number) and §8.2 (variability is
an auto-loop, conservativeness is an owner knob) quietly hand the **owner** the lever that sets how
conservative the customer-facing promise is. That is a real allocation of power: a window can be honest
*and* skewed — biased wide to protect the venue's on-time-rate (OTP) at the cost of customers who'd
have ordered on a tighter true estimate, or biased tight to win the cart at the cost of customers who
get a late delivery. The funnel (§1.3) measures the *cost to the venue* of a wide window (lost carts)
but **nothing in the design measures the cost to the customer of a window biased for the venue's
metric** (the late arrival inside an "honest" band, the trust slowly spent). range-never-point
guarantees the customer gets a *band*; it does not guarantee the band is centered on *their* interest
rather than the venue's OTP target.

Nobody in the brief, proposal, or ADRs asks: *does the customer have any representation in where the
promise sits inside the honest range?* At cold-start, with the owner holding the conservativeness knob
and the funnel only counting the venue's losses, the answer is currently "no." Not a stop — a question
to carry into the autopilot design, before the loops make this asymmetry self-reinforcing.

---

## RE-EXAMINE round 2 — re-reading the revised proposal + resolution + ADR-0007/0008/0009 v2

> Scope: did the round-1 frictions actually get resolved, or only relabeled; did the fixes introduce
> new ethical questions; is the deferral of §5 acceptable. Verdict first, evidence below.

**Verdict: the council may proceed. ESTOP-1 is genuinely LIFTED — not cosmetically. No new red line.
No outstanding human decision blocks the seam batch. One real open question remains, correctly
recorded with an owner and correctly deferred (with a caveat below).**

### R2.1 — ESTOP-1: lifted (the split is real, and the customer reads the mutable channel)

I traced the load-bearing claim rather than trusting the disposition label, because "we adopted the
split" is only an actual lift if the **customer's read path** points at the mutable column. It does.
ADR-0009 v2 §3 (lines 133–138) states it explicitly and unambiguously:

- The trigger function (`orders_promised_window_set_once`) guards **only** `promised_window_*` — its
  body never references `live_eta_*`, so the live channel is mutable by construction, not by promise.
- `live_eta_*` is **seeded equal at confirm, then updated as the order collapses through stages**;
  **"the customer page reads `live_eta_*`… the §8 metric reads `promised_window_*`."**
- A mis-set first promise stays in the historical record (correct — it is a falsification data point),
  but the customer is no longer frozen into it.

This dissolves the tension exactly as recommended — it does not choose measurement over the customer,
it gives each its own column. **There is no residual hole where the customer is shown the frozen
number.** The one place the frozen value surfaces (the §8 metric / historical record) is owner/analytics-
facing, not customer-facing — which is the *correct* audience for "what was promised vs what happened."
The set-once-vs-live separation is also enforced both ways in the DoD (ADR-0009 §"Set-once test" +
"Live-mutable test", lines 206–208): a frozen-pair UPDATE must RAISE, a `live_eta_*` UPDATE must
SUCCEED. That is the regression that would catch a future refactor accidentally freezing the live
channel. **ESTOP-1 confirmed removed.** The accepted-risks table even re-labels it honestly
(proposal §7, line 333: "RESOLVED — split adopted… No human-needed disposition remains for the schema").

One small honesty note, not a stop: ADR-0009 says `live_eta_*` is "updated as the order collapses
through stages," but neither the ADR nor the proposal names the *width-floor enforcement on the live
channel as it tightens*. The DB `CHECK (live_eta_hi >= live_eta_lo + 1)` exists (ADR-0009 §3, line
113–114), so it can't become a literal point — good. But the `min_window_width_min` floor (Counsel #1)
is described only on the *synthesis* helper for the promised window; the live channel, as it collapses
near delivery, is exactly where pseudo-precision ("1–2 min") is most tempting. **Non-blocking note:**
apply the same `min_window_width_min` floor to each `live_eta_*` recompute, not just the initial
synthesis — otherwise the honest-below guarantee weakens precisely at the arriving stage.

### R2.2 — New ethical questions from the fixes

**(a) decrement-at-CONFIRM → the ACCEPTED-then-REJECTED-for-stock path. Is it honester or worse than
refusing up front?** This is the sharpest new question and it deserves a real answer, not a shrug.

The C1 re-architecture (decrement at confirm, not create) is the *right* lifecycle call and the
restock matrix (resolution C1, lines 53–59) is clean. But it relocates *when* a customer can be told
"out of stock": no longer at the moment of ordering, but at confirm — i.e. **after the customer has
committed and is waiting.** Two readings collide:

- **Worse (care/dignity):** being accepted and then refused is a sharper disappointment than being
  refused at the door. The customer has mentally "bought" the meal; the rejection lands later, possibly
  after they've stopped looking elsewhere. This is a real care cost the create-time refusal didn't have.
- **Honester (truth/fairness):** decrement-at-create would *reserve* stock for unconfirmed orders that
  may never be confirmed — meaning a *later, genuine* customer is falsely told "sold out" because a
  PENDING ghost is holding the unit. That is the worse lie: refusing a real buyer to protect a
  phantom. Decrement-at-confirm refuses only when the commitment is real on both sides. **The party
  who would have been wronged by the alternative (the second real customer, falsely refused) is more
  wronged than the party wronged here (the first customer, refused at confirm).**

On balance the design's choice is **more honest, not less** — it refuses based on real scarcity, not
reserved-by-a-ghost scarcity. But it carries a **dignity obligation at the rejection surface** that
the proposal only half-discharges: the OUT_OF_STOCK message has a cause-hint (good, Counsel #4 closed,
ADR-0007 v2 `{code:'OUT_OF_STOCK', error:'Product <name> is out of stock'}`). What is *not* specified
is whether a CONFIRMED→REJECTED-for-stock customer is told **at the moment of rejection through a
channel they'll see** (the live order page they're already watching), vs. silently rolled back. Since
`live_eta_*` already establishes a customer-facing live order channel, the stock-rejection should ride
the same channel with the same cause-hint. **Non-blocking note (R2-a):** specify that a post-confirm
stock rejection surfaces to the customer *on the live order view with the product name*, not just as
an API 422 the FE may swallow. The auto-refusal is ethically fine; the *manner* of telling is the
dignity test, and it's currently under-specified for the confirm-time path specifically.

**(b) dual-idiom RLS — privacy/tenant-hole risk.** This is the one I checked hardest, because a
disjunction-of-idioms policy is exactly the shape that *can* open a cross-tenant read. Reading the
actual policy (ADR-0009 §1, lines 58–66; resolution C2, lines 78–94): the USING/WITH CHECK is
`location_id IN (app_member_location_ids()) OR location_id = NULLIF(current_setting('app.current_tenant',
true),'')::uuid`. The safety hinges on two facts the resolution verified against source:

1. An owner context sets **only** `app.user_id` and **never** `app.current_tenant` (couriers set the
   reverse, `shifts.ts:337`). So in each context exactly one disjunct is live; the other evaluates on
   an **unset** GUC.
2. `NULLIF(current_setting(..., true), '')` makes an unset variable **NULL**, and `location_id = NULL`
   matches **zero rows** (not an error, not a wildcard).

This is the correct construction. The disjunction does **not** widen tenancy because the second
disjunct is dead (NULL) in the owner context and the first is dead (empty set — couriers aren't
members) in the courier context. The danger shape — where both GUCs are set in one connection and an
attacker controls one — is foreclosed by the verified "each world sets exactly one" property, and the
DoD includes the cross-context isolation test (ADR-0009 §"Cross-context RLS test", lines 204–205:
owner sees own rows, cross-tenant SELECT returns 0; courier can't read another tenant). **The residual
risk I'd flag for the Breaker, not as an ethical stop:** the safety depends on the invariant "no code
path ever sets *both* `app.user_id` and `app.current_tenant` on the same connection." That is true
today but is an *un-enforced* cross-cutting assumption — if a future handler sets both (e.g. an
owner-impersonates-courier debug path, or connection-pool GUC bleed without RESET), the policy widens
silently. This is a robustness/verification concern (Breaker's domain), not a values red line, and the
resolution already pins the presence + isolation tests. **Not an ESTOP.** Privacy posture intact:
`order_sensor_events` carries no PII (order_id + location_id + event_type + jsonb payload), so even a
hypothetical leak is geofence-crossing timestamps, not identity. I confirm no privacy red line crossed.

**(c) the 5 non-blocking advice items — genuinely closed or cosmetic?** Walked each against the v2
text:

- **#1 window-width floor** — **genuinely closed, and hardened beyond what I asked.** Not just a floor:
  `min_window_width_min DEFAULT 5` on `locations` + synthesis enforces `hi := max(hi, lo + floor)` AFTER
  the eta_cap clamp + DB `CHECK (hi >= lo + 1)` + Zod rejects `lo==hi` (L2, ADR-0009 §4 lines 152–158).
  Schema + value + render levels. This is the *complete* range-never-point contract I said was missing.
  (See R2.1 caveat: extend it to the `live_eta_*` recompute too.)
- **#2 courier-metric norm** — **closed at the norm level, correctly deferred at the metric level.**
  The metric is North-Star (§8.3), not this batch — so the honest move is to record the binding norm
  NOW so it can't ship without it. ADR-0009 §4c (lines 183–186) does exactly that: "surfaces the
  normalized number only… owner-advisory and MUST NOT be the basis of an automated deactivation,"
  owner = North-Star lead. Not cosmetic — it's a pre-committed guardrail on a future build.
- **#3 nudge-advisory** — **closed.** proposal §4.3 + §5 + ADR-0009 §4c (line 187–188): courier-facing
  advisory, non-compliance NOT an owner-visible compliance signal. The dignity line is now in writing.
- **#4 OUT_OF_STOCK cause** — **closed** (humane message pinned as a DoD assertion). See R2-a above
  for the one remaining sharpening (surface it on the live customer channel for the confirm-time path).
- **#5 funnel privacy-notice + unlinkability** — **closed, and strengthened.** M2 designs unlinkability
  *in* (session_ref never written on an order; FE rotates it at submit; grep-gate "no session_ref in
  orders.ts") rather than merely promising it, plus the `/compliance` + privacy-notice disclosure.
  Rotation-at-submit is a genuinely better answer than I asked for — it severs the timing-correlation
  vector structurally, not just by policy.

None of the five is cosmetic. Three (#1, #2, #5) came back *stronger* than the advice. This is an
Architect who treated the counsel as design input, not as a checkbox.

### R2.3 — Open question §5 (where in the band the promise sits; no customer-side cost signal): is
deferral acceptable?

**Yes, the deferral is acceptable — with one sharpening.** The reasoning:

The question is about **where inside the honest band the synthesized promise is centered**, and the
missing **customer-side cost signal** (late-within-band rate) to counter the owner's OTP conservativeness
knob. The Architect defers it to autopilot-design-time as a recorded human-needed item (proposal §7
line 334; resolution lines 347–355, 384–387). Is that legitimate, given "the collection seam is being
laid now"?

The honest test is: **does laying the seam now foreclose the answer, or pre-commit the asymmetry?** It
does not. The *centering policy* lives entirely in the synthesis helper (runtime), which is explicitly
out of this batch's scope (North-Star) — no column laid here decides where the band centers. So
deferring the *decision* is correct: it genuinely belongs at autopilot-design time, and deciding it now
would be deciding a runtime policy before its loop exists.

**But — the sharpening (R2-c), and this is the one thing I'd press on:** the *measurement of the
customer-side cost* is a **collection** concern, and collection seams ARE what this batch lays. The
funnel (`…070`) is being built now to measure the venue's lost-cart cost. The symmetric customer-side
signal — **late-within-band rate** (delivered after `live_eta_hi` / after `promised_window_hi`) — is
*derivable from columns this batch already lays* (`delivered_at`, `promised_window_*`, `live_eta_*`),
so it needs **no new seam**. The risk of pure deferral is asymmetry-by-default: we ship a seam that
*actively measures* the venue's cost (the funnel) while the customer's cost is merely "derivable later
if someone remembers." Loops optimize what they measure. If the funnel signal exists at autopilot time
and the late-within-band signal does not, the autopilot will be *built* tilted toward the venue's OTP
before anyone decides it should be — the asymmetry self-reinforces exactly as §5 warned, not through a
decision but through which signal happened to be wired first.

So: deferring the *decision* (where to center) = correct. But **the late-within-band metric should be
named NOW as a first-class reconstruction output of the M1 NULL-contract** (it's just another
both-endpoints-non-NULL duration: `delivered_at` vs `promised_window_hi`/`live_eta_hi`), so the two
costs arrive at autopilot-design time **as peers**, not one wired and one hypothetical. This is
**non-blocking** (it's a measurement the M1 contract can absorb with one more line, ADR-0009 §4b) — but
it's the difference between deferring a question fairly and deferring it in a way that quietly answers
it. I'd record it as a defer-flag on the *measurement* (built now, free) distinct from the defer of the
*decision* (autopilot-time, human).

### R2.4 — Health note on the process itself (Physician lens, applied lightly)

Brief observation, offered as care not critique: the resolution did not severity-inflate to look
thorough, nor did it accept-risk its way out of the one real red line — it took the more expensive
correct fix (the column split, the C1 re-architecture) where a weaker team would have written "accepted,
owner: Product." The accepted-risks that remain are genuinely tail-risk and owner-assigned (distributed
botnet at cold-start; privileged-write bypass as the *intended* escape hatch). No convergence-theater:
where I steel-manned the append-only log as possibly more elegant, the Architect *agreed* and recorded
it as the North-Star upgrade rather than defensively rejecting it. This is a healthy Architect↔Counsel
loop. The only process smell to name (mildly): the §5 deferral-of-decision was correctly separated from
what should be a *non*-deferred measurement (R2.3) — a small instance of "defer the hard thing whole"
where it could be split. Easily corrected; flagged, not a pathology.

### RE-EXAMINE summary

| Round-1 item | Round-2 status |
|---|---|
| **ESTOP-1** (frozen-wrong time, no repair) | **LIFTED.** Split is real; customer reads mutable `live_eta_*`; frozen value is owner/analytics-only; DoD enforces both directions. No customer-facing hole. |
| decrement-at-CONFIRM new question (R2-a) | **Net more honest** (refuses on real scarcity, not ghost-reserved). Non-blocking note: surface the confirm-time rejection on the live customer channel, not just a swallowable 422. |
| dual-idiom RLS (R2-b) | **No privacy/tenant red line.** Construction is sound (each world sets one GUC; NULLIF→NULL→0 rows). Residual "never set both GUCs on one conn" is a Breaker robustness concern, not an ESTOP; isolation test pinned. No PII on the table. |
| 5 non-blocking advice (R2-c) | **All closed; #1/#2/#5 strengthened beyond the advice.** None cosmetic. |
| open question §5 deferral | **Decision-deferral acceptable.** Caveat: name the customer-side **late-within-band** metric NOW as an M1 reconstruction output (free, no new seam) so the two costs reach autopilot-design as peers, not one-wired-one-hypothetical. |

**The council may exit.** No grounded red line remains unaddressed without a recorded human decision.
New non-blocking notes this round: (R2-a) surface confirm-time stock rejection on the live customer
channel with the cause-hint; (R2.1) apply the width floor to `live_eta_*` recomputes, not just initial
synthesis; (R2.3) record the late-within-band customer-cost metric as a built-now M1 output, deferring
only the centering *decision*. None blocks the schema/seam batch.
