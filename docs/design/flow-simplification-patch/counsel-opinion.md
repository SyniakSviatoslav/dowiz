# Counsel Opinion — Flow Simplification Patch

**Seat:** Counsel (Triadic Council) · advisory; non-blocking except a grounded ETHICAL-STOP.
**Date:** 2026-06-28 · grounded against the proposal, the ADR, and the shipped P6 claim council verdict.
**Posture:** the human decides. Friction here is proportional; most of what follows is advice, not a gate.

---

## 0. Frame

This is a strong, honest patch. Its design principle — *remove structure built around choices that no
longer exist* — is subtractive, grounded, and concept-coherent. The architect's radical candor (two of
six are already built; §6 "one action goes live" **contradicts the shipped model → REVISE**) is the seat
working as it should: self-correcting before the council touches it. I am not here to inflate friction on
a document that already caught its own worst move. I am here to name where the *simplification narrative*
shifts a cost onto a person the action-count table does not count.

One throughline runs under §2, §5, and §6: **fewer taps is not the same as less burden.** A patch that
measures itself in taps removed can silently transfer cognitive load (the lost running total), emotional
load (the lost "your food is being made" beat), and labor (the operator's hand-build) onto people whose
burden does not appear in the metric. That is the load-bearing tension, examined by lens below.

---

## 1. Honesty / clarity — §2 cart-bar total removal (the one cut against the patch's own principle)

The patch's principle is *cut structure built around dead choices*. The running total is not that. It is
**live, real, and used** — it is the one piece §2 removes that is signal, not scaffolding. So §2 is the
single place the simplification principle is mis-applied: it cuts information the user wants, dressed as
removing chrome.

Why this matters more here than in a generic product: **this is cash-only.** Cash-as-proof means the
customer must physically produce exact money at the door. The running total is therefore not a
convenience — it is the instrument that lets a cash-constrained customer (precisely the first-wave
demographic) shop *within their wallet* as they build the cart. Removing it from the persistent bar means
they assemble a cart and discover the number only at the commit point, after the mental commitment is
made. The most budget-conscious user bears the cost.

The patch's own justification cuts the other way: it says showing the total invites the "fee unknown"
(`feeKnown=false`) ambiguity on the bar. But the honest response to an unknown fee is to *show* "items: X
· fee at checkout" **before** commitment — not to hide the number until confirm. Hiding the cost until
the decision point is the shape of a dark pattern even when no one intends one.

**Non-blocking, strong:** keep a count if you must, but the running subtotal belongs where a cash customer
can reconcile it *before* the door — on the bar, or at minimum surfaced (not collapsed) and early in the
panel. This is advice, not a STOP: no grounded red-line says "show a running total."

---

## 2. Care / honesty — §5 READY removal: who loses signal is the customer, and the patch is silent on them

§5 is reasoned **entirely from the owner's hand** ("owner saves a tap"). The waiting customer is absent
from the section. Two care costs land on them:

1. **Lost progress signal during the longest wait.** The gap between "confirmed" and "in delivery" is the
   most anxious stretch — the food is being made, nothing is moving. The 2-tap default *encourages* owners
   to skip the PREPARING beat, so an anxious or first-time customer stares at "confirmed" until it
   suddenly jumps to "on its way," with no reassurance in between.
2. **The label may stop being true (R6, named but under-weighted).** On the 2-tap path an order can reach
   IN_DELIVERY *before the food is ready*. The customer-facing copy for that state ("out for delivery" /
   "on its way") then over-promises: it implies food-made-and-moving when it may mean
   owner-accepted-and-pre-assigned-while-cooking.

Lens test: this is **adjacent to** "UI tells the truth," but the server *is* authoritative — the status
genuinely is IN_DELIVERY. The issue is **labeling semantics**, a judgment, not a grounded red-line
crossing. So: non-blocking, but the sharpest honesty point in the patch. Two frames on the fix —
**care:** keep a lightweight customer-facing "preparing" signal decoupled from the owner's tap burden
(the owner's tap-count and the customer's information are different concerns the patch conflates);
**honesty:** choose IN_DELIVERY customer copy that is true across *both* readings (food-ready and
food-still-cooking) rather than copy that asserts movement that may not have begun.

The owner-side simplification is legitimate; the silent transfer of an information cost onto the waiting
customer should be named and mitigated, not assumed away with "the courier can call."

---

## 3. Dignity / care — §3 information floor and the cost of demoting entrance/apartment

The patch is **honest** about the "≈5 actions" framing: it openly states ≈5 holds *only if*
entrance/apartment/notes are demoted from required to optional, and flags R3 to council. No under-statement
there — credit it.

The dignity reasoning behind demotion is also correct, and worth defending against a knee-jerk "more
required fields = safer": forcing entrance/apartment on a single-door house makes people type garbage
("N/A", "1") that *pollutes* the courier signal anyway. Optional-but-present beats required-but-irrelevant
for both dignity and data quality.

The care caveat the patch does not state: the fallback for a missing entrance/apt is **"the courier can
call"** — and that fallback assumes a frictionless phone conversation. For the person who most needs the
door found *without* a call (limited phone comfort, a language barrier, hearing difficulty, a buzzer-only
building), the call is the failure, not the safety net. The asymmetry: demotion saves the *confident
repeat* customer two taps while loading the *least-served* customer with the one interaction they can
least afford. **Non-blocking recommendation:** demote, but keep the optional fields *contextually
inviting* (present and easy, not buried behind a "more" toggle), so the person who needs them is not
fighting to provide them. Optional must mean "skippable," never "hidden."

---

## 4. Consent / dignity — §6 claim: the contradiction is already reversed; protect the reversal

The dangerous move — §6's "one action: takes ownership, binds login, **goes live**" — directly crosses the
shipped, council-bound consent gate (CC2 three acts; CC3 allergen as a distinct deliberate act into empty
fields; H-publish: `published_at` NULL through claim; activation requires `menu_confirmed_at`). **The
architect already caught it and ruled REVISE, and the ADR Decision §6 encodes the reversal**: claim =
ownership + login bind (one act); go-live = a separate, gated act. As decided, the consent gate holds.
There is nothing here to STOP — STOPping a crossing the author already reversed would be friction-inflation,
and I hold myself to that.

What I add is about **narrative pressure**, not this document. The whole patch's telos is *collapse acts,
remove taps*. The consent architecture is the one place that deliberately *resists* collapse — three acts
**by design**, allergen confirmation **separate by design**. This document walls the gate off correctly.
The risk is **downstream**: a future "simplification round 2," or an implementer reading "review → confirm
allergens → publish" as three taps ripe for collapse, will feel the same pull that produced the original
§6 phrasing. The consent gate's friction is not incidental overhead like the cart drawer — **it is the
ethical substance.** A simplification value applied recursively without a stop-marker will eventually
mistake the consent gate for scaffolding.

**Binding ratification condition (not a STOP, because the ADR already resolves it):** the council should
record that it concurs with the **ADR Decision §6 (revised)** only, and that the proposal's original "one
action … goes live" prose is **not** a build source. And the three-act sequence should be annotated, in
ADR and code, as **PROTECTED FRICTION** — friction that exists for consent and allergen safety, explicitly
distinct from the incidental friction this patch removes — so the distinction survives the next
simplification pass.

Two dignity notes on the claim *surface* (the genuinely net-new, consent-sensitive work the "mostly
already built" framing risks under-caring for):
- **Sequencing:** the patch's warm "owner opens a working service" framing must not lead. Before claim, the
  service is **not theirs** and was built **without them**. The CC1 honest Art-14 notice ("you didn't ask
  for this; here's exactly what we did and your options") must dominate the *first* screen; the seductive
  working preview comes *second*. Preview-before-notice would let the seduction launder the consent.
- **Decline parity:** decline must stay equally prominent and account-free (H-decline). Claim-louder-than-
  decline is the dark-pattern tell the P6 verdict already named — the new surface must not reintroduce it.

---

## 5. Aesthetics / coherence (non-blocking)

Concept integrity is high. "Remove structure built around choices that no longer exist" is a clean,
honest, *subtractive* principle. Two cuts are genuinely elegant: §1 achieving normal==embed by **removing**
the divergence rather than branching on a mode (simplification that reduces surface, not adds one), and the
"schema rich, runtime minimal" restraint of refusing to add `kitchen_flow_enabled` speculatively. The lone
incoherence is §2: it is the one cut that removes *signal* rather than *scaffolding*, breaking the patch's
own stated principle (see §1 above). Tightening §2 would make the patch coherent with itself.

---

## 6. Strategy (non-blocking)

The patch is **honest** that §6 scales by operator labor, not owner self-serve ("operator pre-does the
build"). Good. Two strategic additions:

- **The cost is honest in the prose but absent from the metric.** The action-count table reads customer
  −3, owner −2 — and is silent on **operator +N** (scrape → AI → branded build → hostile-recipient notice,
  per shadow). The "simplification" is real for two parties precisely because complexity is *concentrated
  on a third*. This is a legitimate first-wave bootstrap (do things that don't scale) — but it should
  appear **on the ledger**, not only in a footnote, or the simplification claim is partial.
- **Guard the exit.** A model that scales by operator effort has a hidden ceiling (the operator) and a
  reversal seam (self-serve onboarding, here "de-emphasized but kept as foundation"). De-emphasized
  foundations rot. Keep the self-serve path's tests green and its seam warm, so the bootstrap stays
  *reversible* and does not quietly become the only path the day operator capacity runs out.

---

## 7. Agent-health (brief, with kindness)

The architect seat is healthy: grounded every claim to source lines, steel-manned options, refused
speculative schema, and reversed its own most dangerous proposal. One watch, not a pathology: the
repeated "most of this is already built / UI removal" framing could anchor the implementer into treating
the **claim surface + K4 allergen writer** as low-care. The backend is shipped; the *consent UX* is
net-new and is the entire ethical payload of §6. "Mostly already built" is true of the bytes and false of
the stakes — keep the care budget on the new surface.

---

## 8. ETHICAL-STOP(s)

Tested against the project's grounded red-lines — no-trap-states, consent-before-publish for provisioned
locations, data-minimization, server-authoritative, human-in-loop:

- **No-trap:** §1 panel-close = close + cart intact; no order exists pre-confirm. Preserved. No crossing.
- **Consent-before-publish:** the original §6 "one action goes live" WOULD cross it; the ADR Decision §6
  **reverses it** (published_at NULL through claim; gated activation). As decided, no live crossing.
- **Data-minimization:** §4 adds no field/PII; §6 reuses the shipped token model (org/loc derived from the
  invite, no enumeration); H-erase-on-claim already mandated. No crossing.
- **Server-authoritative:** §2/§5 relocate/relabel display; server total + cash-422 and the state machine
  stay authoritative. The §5 labeling concern is a semantics judgment, not a crossing.
- **Human-in-loop:** claim accept is `verifyAuth`-only with a deliberate human bind; courier/owner human
  acts unchanged. No crossing.

**NO ETHICAL-STOP.** One **binding ratification condition** (§4 above, not a STOP): council records
concurrence with the **revised** ADR §6 only, and the three-act consent gate is annotated as PROTECTED
FRICTION. All §1–§3 and §5–§6 items above are **non-blocking advice.**

---

## 9. Steel-man of a rejected option

**§2 Option B — keep count + total on the cart bar (rejected for "ONLY item count").** This is stronger
than the patch credits, *because the product is cash-only*. In a card product the running total is
convenience; in a cash-as-proof product it is the instrument by which a customer reconciles a cart against
the physical money they can produce at the door. Option B is not "less simplification" — it removes the
cart *drawer* (the real redundant surface) while keeping the *one number a cash customer most needs*. The
patch's stated reason to drop the total (the `feeKnown=false` ambiguity) actually argues *for* surfacing
cost early, not for hiding it. Option B better honors the patch's own principle — cut scaffolding, keep
signal — than the chosen Option A does.

(Secondary steel-man — §5 Option B, the `kitchen_flow_enabled` toggle *now* rather than deferred: for a
kitchen that needs the preparing/ready beat to set customer expectation, the toggle is not speculative
schema — it is the seam that keeps the *customer's* progress signal alive while the owner gets fewer taps
by default. The patch's "cut the seam when you wire it" is right in general; here the customer-facing cost
of deferring is the case for wiring it sooner.)

---

## 10. The question nobody asked

Every number in this patch counts **taps removed**. Not one counts **understanding added or lost.** A
cash-constrained customer who can no longer see the running total, and an anxious customer who can no
longer see the food being made, each reach their next state in *fewer taps* and *understand it less*. If
"simplification" is measured only in actions, it can increase the cognitive and emotional burden it claims
to remove — and it can do so while looking like progress on the dashboard. **Did we simplify the
experience, or just shorten it? And the one perspective entirely absent from the action-count table — the
operator, whose hand-built labor is the actual product right now — what is *their* tap count, and what
becomes of this whole model the week they run out?**

---

### Verdict

**NO ETHICAL-STOP.** Approve as advisory-concur, conditioned only on council ratifying the **revised** ADR
§6 (claim ≠ go-live; consent gate as PROTECTED FRICTION). §1, §3, §4, §6-surface: sound. §2 (running total)
and §5 (customer progress signal + IN_DELIVERY honesty): non-blocking but please revisit — they are where
fewer taps quietly buys less understanding.

---

# RE-EXAMINE — round 2 (post-fix)

**Date:** 2026-06-28 · re-grounded against `resolution.md` + the revised `proposal.md`.
**Question answered:** did round-1 resolutions close the value/justice/care concerns, and did any fix open a
new one? Verdict first: **all four round-1 care concerns are closed or materially improved; two fixes open a
small, named, non-blocking honesty seam each; one fix is improvable (a third option the human decision should
weigh). No red line eroded. No residual STOP.**

## R2.1 — §2 running total: the revision landed; a smaller honesty seam took its place

The reversal is the right move and faithfully recorded: cut the cart *drawer* (real scaffolding), keep the
**items-subtotal on the bar** (signal), resolve the fee in the panel with "+ delivery fee at checkout". My
round-1 steel-man (Option B-hybrid) is now the design. The dark-pattern shape I named — assemble a cart,
discover the number only at commit — is gone for the *food* cost.

**But the fix shifts, not erases, the surprise.** The bar now shows items-subtotal; the **fee** still first
appears at the panel. A cash-constrained customer who mentally budgets to the bar number can still be short
at the door by the fee. Two things make this **not** a re-opened dark pattern, plus one residual:

- **The fee genuinely cannot be honest on the bar before an address.** It is distance-tier
  (`feeKnown=false`) until a pin/address exists, which doesn't exist while browsing. You cannot surface on
  the bar a number you do not yet have. So **subtotal-only is the honest *floor*** — it is the one number
  that is always true at browse time. Showing a guessed fee early would be *less* honest, not more.
- The fee is small and bounded; the surprise is a tier-fee, not a hidden total.

**Residual (non-blocking, copy-level):** the bar number must *read as a subtotal*, not as the total. A bare
"N items · {price}" invites the customer to treat it as all-in; then the panel fee reads as an *addition to
the total they thought they'd agreed to* — a small honesty seam. Mitigation, folded into the copy the
resolution already routes to product: label the bar number as **items / subtotal** (not a bare price that
reads as total), and surface the fee in the panel **the instant a tier is known** (on pin/address resolve),
never deferred to the confirm tap. Where the fee is flat/known up front (not distance-tiered), surface it on
entry to the panel, not at confirm. **Counsel: subtotal-only is the honest floor given the fee is unknowable
until address; the fix is honest as long as the bar copy does not let a subtotal masquerade as a total.**
Not a STOP — server total + cash-422 stay authoritative; the floor is genuinely the most-honest number
available at browse time.

## R2.2 — §5 customer signal: restored, and the reintroduced overclaim is smaller than the one it cured

Auto-stamp `preparing_at` on Accept (F4) + timestamp-driven ETA (F3) + timestamp-driven progress dots (F5)
**restore exactly what round-1 said the customer lost**: the "your food is being made" beat across the
anxious confirmed→moving gap, and an ETA/progress bar that decays by real cook time instead of lying to zero
at IN_DELIVERY. The owner keeps the saved tap; the customer keeps the signal; the model keeps its timestamps.
This is the clean resolution of the round-1 tension — credited.

**Does it reintroduce a labeling-honesty issue in the other direction?** Yes, a *bounded* one, and it is the
right trade. Stamping `preparing_at` at Accept means "preparing" can show before the kitchen physically picks
up the ticket (accept-then-cook-10-min-later in a busy kitchen). This is the mirror of the round-1 IN_DELIVERY
overclaim — but **materially smaller**, for three reasons:

1. In a 2-tap, single-venue, cash world, **owner-accept *is* the commit-to-cook** — accept≈start is close to
   true, far closer than "out for delivery"≈"food left the building" was.
2. "Preparing" asserts a *process*, not a physical irreversible event; the customer **takes no action** on
   it (unlike "on its way", which can send someone to the door). A micro-overclaim on a no-action signal is
   low-harm.
3. There is **no more-honest timestamp available** without re-imposing the owner PREPARING/READY tap — the
   exact burden we removed. The alternative to a slightly-early "preparing" is *no signal at all*, which
   round-1 judged the worse harm across an anxious wait.

The ETA decaying from accept can *under-promise* remaining time when a kitchen is backed up (says ~13 left
when cooking hasn't started) — but that is inherent to any prep estimate, bounded by `prep_time`, and
**strictly more honest** than the status-label zeroing it replaces. **Residual (non-blocking, copy):** the
customer copy for this state should read as process ("Preparing your order") not as asserted physical action
("Your food is being cooked right now"). **Counsel: net more honest than the status quo; the reintroduced
overclaim is bounded, no-action, and the least-harm option given the tap we removed.** Not a STOP.

## R2.3 — §3 field floor: optional-but-inviting is good, but a *third* option is more caring for the least-served

The resolution adopted optional-but-inviting and routed the hard-required business question to product/ops —
which honours my round-1 dignity argument (skippable, never hidden) and my round-1 care caveat (the
least-served, for whom "the courier can call" is the *failure*). Good as far as it goes.

**But re-examined on the care dimension, the binary the human decision is being handed (hard-required vs
optional-but-inviting) is the wrong frame, and optional-but-inviting under-protects the very person it was
meant to protect.** The argument: the least-served customer (language barrier, low digital comfort,
buzzer-only) is **also the least likely to proactively fill an *optional* field**. Optional-but-inviting
reliably protects two groups — the confident *skipper* and the conscientious *filler* — but the vulnerable
*non-filler* skips the optional door-detail precisely because it is optional, then faces the call they cannot
take. The fallback hasn't moved; it just became invisible.

**The more caring default is contextually-required, not flat-optional:** make entrance/apartment optional when
the map-pin is **high-confidence** (clear single-unit geocode, pin snapped to a known address) and
**required when the pin is low-confidence** (pin dropped far from any snapped address, a multi-unit building
geocode, an ambiguous/area-level result). This routes the one ask to exactly where omission actually causes a
failed delivery and an unwanted call — friction proportional to real risk — and stays out of everyone else's
way. It protects the vulnerable non-filler (the system asks *for* them, when it matters) without taxing the
confident user (silent when the pin is unambiguous). It is also server-tolerant (a client-side conditional
gate; no contract change), so it crosses no red line.

**Counsel to the human decision (additive, non-blocking):** treat §3 as a **three-way** choice, not a binary.
Recommend **contextually-required (pin-confidence-gated)** as the most caring default — it dominates
flat-optional on care for the least-served and dominates hard-required on dignity for the confident user.
Keep optional-but-inviting as the floor if a pin-confidence signal isn't readily available at the seam. This
is advice to the ratifier, not a gate.

## R2.4 — PROTECTED FRICTION annotation: recorded faithfully

Verified against `resolution.md` §C (Counsel §4) and `proposal.md` §6 RESOLVE/Counsel §4. Both halves are
present and distinct: (1) **claim → review → publish** three-act consent sequence (CC2) and (2) **allergen
confirmation as a distinct deliberate act into empty fields** (CC3), both annotated as PROTECTED FRICTION,
**explicitly distinct from the incidental cart/page friction this patch removes**. The two surrounding
dignity rules survive too — CC1 (Art-14 notice dominates the first screen, preview second) and H-decline
parity (equally prominent, account-free). The "original 'one action … goes live' prose is **not** a build
source" guard is recorded. **Faithful.**

One carry-forward, not a defect: the annotation currently lives in the ADR/resolution *prose*. Its durable
job — surviving the *next* simplification pass — depends on the **code annotation at the claim/activation
seam** (the proposal says "annotate in code"). That annotation is the real guardrail against a future
implementer reading three acts as three collapsible taps; it must actually land when the surface is built,
not stay prose-only. Flagged as a standing condition, below.

## R2.5 — red-line re-walk: none eroded (several strengthened)

- **No-trap:** strengthened — F1 gated dispatch keeps the order CONFIRMED on no-courier + an IN_DELIVERY
  recovery branch (also closes a latent today-bug); F6 Back stays on `/s/:slug`; F9 no empty/stale strand. OK
- **Consent-before-publish:** unchanged and hardened — three-act gate held, "go-live ≠ claim", PROTECTED
  FRICTION annotated. OK
- **Server-authoritative:** intact. The §2 bar subtotal is a server-mirrorable **display**; server total +
  cash-422 authoritative. The §5 `preparing_at` auto-stamp is a **server-side, COALESCE-guarded write**, and
  F3 ETA / F5 progress are **server-timestamp-driven** — this fix moves the customer signal *toward* server
  truth, not away. OK
- **Data-minimization:** no new field, no new PII; §2 subtotal and `preparing_at` are existing data; §6 token
  model untouched (org/loc derived from invite, no enumeration). OK
- **Honesty / UI-tells-truth:** **net more honest** (timestamp-driven ETA/progress replaces status-label
  lies; subtotal-floor replaces hidden-total). The two new seams (R2.1 subtotal-as-total copy, R2.2
  preparing-on-accept) are **labeling judgments with honest floors**, mitigated by copy, neither a grounded
  crossing. OK

No fix introduced a soft-confirm-as-trap, a dark-pattern, a PII egress, an enumeration, or an inline
ownership UPDATE. The two HIGH fixes (F1 no-trap, F2 token transport) move *toward* the red lines they touch.

## R2.6 — residual ETHICAL-STOP + binding-condition status

**Residual ETHICAL-STOP: NONE.** Re-walked every grounded red line above; no fix crosses one. The two new
honesty seams (R2.1, R2.2) are non-blocking copy-level judgments with honest floors, not crossings. The §3
care improvement (R2.3) is additive advice to the human ratifier, not a gate.

**Binding ratification condition — now HOLDS as recorded rule**, with one standing carry-forward:
- The council records concurrence with the **revised** ADR §6 only (claim = ownership+login bind; go-live a
  separate gated act); the original "one action … goes live" prose is **not** a build source. — **Recorded**
  (`resolution.md` §C Counsel §4). OK
- The three-act consent sequence + allergen confirmation are annotated **PROTECTED FRICTION**, distinct from
  incidental friction. — **Recorded in prose; standing condition: the code-level annotation at the
  claim/activation seam must materialize when the surface is built** (its whole purpose is to survive the
  next simplification pass — prose alone will not). This is the one item I ask the council to keep on the
  implementer's exit checklist, not because it is unresolved here, but because its durability is deferred to
  build time.

**Net:** round-1 closed honestly. §2 and §5 — the two places I said fewer taps was quietly buying less
understanding — now *add* understanding back (subtotal kept; preparing/ETA/progress restored from real
timestamps) while removing the tap. §3 is improvable (R2.3) but not wrong. The patch is coherent with its own
principle for the first time across all six sections. Advisory-concur stands; no STOP.

---

# RE-EXAMINE — round 3 (convergence confirmation)

**Date:** 2026-06-28 · re-grounded against `resolution.md` "RESOLVE round 2" (R2-1…PF).
**Scope:** confirm only — did the round-2 fixes hold the care/honesty line, and did any open a new one?
Verdict first: **all three points I asked about are satisfied; the R2-5 re-decision is *more* honest than
the round-2 version I previously blessed; no new value/honesty/care concern opened. No residual STOP.**

## Q1 — preparing_at auto-stamp dropped (R2-5): signal preserved AND more honest

The round-2 design auto-stamped `preparing_at = confirmed_at`. I had passed it (R2.2) as a *bounded*
overclaim — but I named it as an overclaim, and it was a **data-layer** one (`preparing_at` is read by
`fetchOrderDelta`/`OrderProgress`, so the lie propagated into consumers, not just copy). The re-decision
**removes that lie at the source**: `preparing_at` stays NULL on 2-tap orders (the kitchen never marked
preparing — true), ETA decays off **`confirmed_at`** (a *real* event — the owner did accept), and
"Preparing" renders as a **status-driven process label**, never a ✓ over a fabricated timestamp.

Did the customer lose the beat again? **No.** The customer still gets (a) a "Preparing your order" beat
across the anxious confirmed→moving gap, and (b) a live ETA that decays by real elapsed time rather than
flat-lining or zeroing. The signal I fought for in round-1 is intact. What changed is the *substrate*:
the beat is now carried by a real event + a process label instead of a fabricated physical timestamp.
This is the strictly-better resolution — it satisfies care (customer keeps the beat) **and** honesty (no
invented "kitchen started" event, no data-layer lie) at once. It did **not** swing back to
under-informing: the waiting customer is no worse off than the round-2 version, and the data layer is
cleaner. The copy rule is correctly pinned — "Preparing your order" (process), never "being cooked right
now" (asserted physical action). My only round-2 residual on this point (copy must read as process) is now
*recorded as a copy rule*, so the seam is closed, not merely noted. **Satisfied — net honesty improved.**

## Q2 — §3 three-way NEEDS-HUMAN with contextually-required recommended: care concern satisfied

This is exactly the frame I argued for in R2.3. The decision is no longer the binary that under-protected
the vulnerable non-filler; it is a **three-way** human choice (hard-required | optional-but-inviting |
contextually-required pin-gated), and the architect's *recommendation* is **contextually-required** — the
option I named as most caring for the least-served. The care logic is preserved verbatim: friction
proportional to real last-50-metre failure risk, asked exactly when omission would cause the failed
delivery + the call the vulnerable customer cannot take, silent for the confident user. Server-tolerant
(client-side gate, no contract change → no red line). The fallback floor (optional-but-inviting when no
pin-confidence signal) is the right safety net. **Satisfied.** It correctly remains NEEDS-HUMAN — the
business rule + threshold are a product/ops call, not a Counsel one; recording it as a routed human
decision with a care-grounded recommendation is the right disposition.

## Q3 — PROTECTED FRICTION as code-level marker + G-PF1/G-PF2: durability condition met

This closes my one standing carry-forward from R2.4/R2.6. The annotation no longer lives in prose alone —
it is specified as (a) a **named in-code PROTECTED-FRICTION marker** on CC2 (three-act sequence) and CC3
(allergen confirmation into empty fields), plus (b) two **red→green guardrails**: G-PF1 (`published_at`
stays NULL through claim; activation requires `menu_confirmed_at`) and G-PF2 (allergen confirmation is a
distinct authenticated act writing only into empty fields, never auto-confirmed, never folded into
accept/publish). A future simplification pass that tries to collapse the consent gate now trips a
**deterministic red**, not a prose warning a tired implementer can skim past. This is precisely the
durability I asked for — the consent gate's friction is the *ethical substance*, and it is now defended as
code, not narrative. **Satisfied.**

## Q4 — new value/honesty/care concern opened by the round-2 fixes? None grounded

I re-walked each round-2 change for a fresh seam:
- **R2-1/R2-2 (honest no-courier endpoint, returns real status, no orphan, flag-decoupled):** moves
  *toward* server-authoritative + no-trap. The endpoint now reports the REAL status, not the requested
  one — an honesty *gain*. No new concern.
- **R2-3 (in-page fetch-auth + real recipient-binding via `invited_contact_hash`):** strengthens consent
  (token + proof-of-control-of-invited-email, not token-alone) without touching `claim_transfer`. The one
  thing I flag as *guidance, not a concern*: the recipient defense is only active when the operator mint
  supplies `invitedContact` — the resolution makes that a MUST (G-F2d asserts non-NULL hash). Keep that on
  the operator-mint exit checklist; if an invite ever ships with a NULL hash it silently degrades to
  token-only authority. Recorded in the resolution, so not a new open item — just the one place the
  strengthening depends on operator discipline rather than code-forced invariant.
- **R2-4 (revert-to-READY single step; re-dispatch is a fresh act):** honest two-step, no trap, no
  overclaim. No concern.
- **R2-5/R2-7 (copy rules):** both pin the honest floor in copy; seams closed.

The R2-5 re-decision did not open a counter-seam: dropping the fabricated stamp removes a lie and keeps
the signal — there is no new direction of dishonesty. The "Preparing" label showing immediately at accept
(before a busy kitchen physically starts) is the same bounded, no-action, process-label micro-overclaim I
already weighed in R2.2 — and it is now **less** of a concern than the round-2 version, because it is no
longer backed by a fabricated timestamp in the data layer. **No new grounded concern.**

## Round-3 close

- **Residual ETHICAL-STOP: NONE.** Re-walked every grounded red line (no-trap, consent-before-publish,
  server-authoritative, data-minimization, money-integer, claim_transfer-untouched, human-in-loop). No
  round-2 fix crosses one; several strengthen them (R2-1 toward no-trap + server-authoritative; R2-3
  toward recipient-bound consent; R2-5 toward data-layer honesty).
- **Binding conditions hold as recorded rule: YES.** Concurrence with the **revised** ADR §6 only is
  recorded; PROTECTED FRICTION is now a code-level marker + G-PF1/G-PF2 guardrails (my durability
  carry-forward is discharged); §3 is a properly-routed NEEDS-HUMAN with a care-grounded recommendation.
- **Convergence:** confirmed from the Counsel seat. The three things I asked about are satisfied, the
  R2-5 re-decision is a genuine honesty improvement over what I previously blessed, and no fix opened a
  new value/honesty/care seam. Advisory-concur stands; no STOP.
