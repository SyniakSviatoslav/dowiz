# Counsel Opinion — offer-sweep grace-cancel via `cancelUndispatchableOrder`

Role: Counsel (advisory). Scope: values · aesthetics · strategy · process. Not robustness (Breaker's lane), not design (Architect's lane).
Verdict up front: **ETHICAL-STOP on this change = NO.** One *pre-registered* ETHICAL-STOP is placed on the **grace-cancel enablement council**, not on this deploy. Several non-blocking advisories below.

---

## 1. Reasoning by lens (only what's load-bearing)

### Fairness / stakeholders — who bears the cost
The event being modelled is *courier scarcity*. When it fires, the cost falls on **customer** (waited 15+ min, then a robot cancel) and **owner** (order was in CONFIRMED/PREPARING/READY — food may already be *made* and is now written off, no sale). The platform bears nothing. That distribution is not hidden — the design is honest about it — but note that the whole proposal is engineer-framed (invariants, folds, guardrails); the two humans who actually eat the loss appear only as a status string (`event='CANCELLED'`) and a `console.log`. Honest ≠ humane; see §3.

Mitigating and genuinely decent: the sequence is **alert → owner grace window → then honest terminal**. The owner had 15 min of agency before the system acted. That respects owner autonomy and the "human-in-the-loop, no autobahn" line — the auto-cancel is the *last* resort after a human was given the wheel and declined it.

### Care / harm — the prepaid refund gap (the real one)
Grounded fact (verified): `refund_due` obligations are written **only** in `deliveryCompletion.ts` (the refuse/cancel-at-door completion path, lines ~127-140). `cancelUndispatchableOrder` does not route through `completeDelivery`, so a **paid prepaid (crypto) order that is grace-cancelled produces no `refund_due`** → it never appears in the owner refund queue (`routes/owner/refunds.ts`) → the customer's money sits with no obligation-of-record. A customer who *paid* and gets silently cancelled with no refund trail is the paradigm "real person hurt by a background job" — exactly the harm this office guards.

Why this is **not** a stop on *this* change: **both** flags are off — `DISPATCH_OWNER_GRACE_ENABLED=false` **and** `PAYMENTS_CRYPTO_ENABLED/PREPAID=false`. The harm requires *both* on simultaneously. Today it is latent, dark, and disclosed (§10 ACCEPTED). The proposal's claim that the gap "already exists for `PENDING/IN_DELIVERY→CANCELLED`" is **true** — no cancel path outside `completeDelivery` writes `refund_due`. So this change *widens the population of an existing dark gap by one path*; it does not open a new class of harm. Friction proportional to that = a pre-registered condition on enablement (§2), not a block now.

### Honesty / consent — is colocation gaming the gate?
No — with one condition. R3-3 enforces a **location** invariant ("raw cancels only in blessed, audited files"), not a **semantic** one. Moving the raw UPDATE into `orderStatusService.ts` (already in `RAW_CANCEL_ALLOW`, verified — 3 entries, unchanged) *serves* R3-3's stated intent: the cancel semantics (guard/fold/history/bus) now live in the blessed, audited seam instead of scattered in a 260-line worker. That is the honest read. The dishonest option — C, allowlisting the worker file — is correctly rejected as gate-laundering (it would leave every future cancel edit in that worker unguarded). Good instinct.

The **condition**: a green R3-3 must not be allowed to *mean* "the machine is still sole transition authority" — because Option B knowingly creates a **second** authority (a hardcoded `ALLOWED_FROM` array that bypasses `assertTransition`). What keeps colocation honest rather than a technicality is the **ADR addendum** the proposal commits to. That addendum must **merge with the code, not as a follow-up**. Code green + addendum slipped = you *have* gamed the gate (passing scan, undocumented shadow authority). Gate the merge on the addendum.

### Aesthetics / conceptual integrity
Option B is elegant *as containment* ("narrow blessed seam, wired only to the dark worker") and fits "schema-rich, runtime-minimal." But it buys that containment by introducing a **second source of truth for legal cancel-from-states**: `ALLOWED_FROM = ['CONFIRMED','PREPARING','READY','IN_DELIVERY']` hardcoded in the function, separate from the canonical `TRANSITIONS` table. That is a shadow transition table. If a future state is added, the canonical machine gets updated and this array silently won't. Minor smell: the array *mixes* an already-legal edge (`IN_DELIVERY→CANCELLED`) with the three illegal ones, blurring "sanctioned exception" from "already lawful." Elegant-looking, but the elegance is partly *seductive* — it hides a drift risk. Remedy is cheap (§3).

### Long horizon / strategy
Option B is the right call **for now**; Option A is the right *end state*, and the proposal says so and defers A to the ethics council — which is the correct sequencing, not debt-dodging. The accrued debt (second cancel mutator, DRY overlap, shadow array) is **small, disclosed, and explicitly reversible** ("collapse back into `updateOrderStatus` when A ratifies"). Reversibility is the key virtue here: this is a deploy-unblock that does not lock the domain into a shape you'll regret. What you *would* regret in a year is the shadow array drifting unnoticed, or the enablement of grace-cancel forgetting the refund wire. Both are pinnable now (§2, §3).

---

## 2. ETHICAL-STOP(s)

**On this change: NONE.** Dark on both sides, no human harmed, gate satisfied honestly, gap pre-existing and disclosed. Blocking this deploy would be severity-inflation.

**Pre-registered ETHICAL-STOP (attaches to the grace-cancel STOP-ETHICS enablement council, which the proposal already routes to):**

> **STOP-REFUND-BEFORE-GRACE.** `DISPATCH_OWNER_GRACE_ENABLED` and prepaid (`PAYMENTS_CRYPTO_ENABLED`) must **not be co-enabled** until a paid-prepaid grace-cancel writes a `refund_due` obligation (or is proven impossible by state). Grounded line: *care/harm — a customer with a false/uncompensated charge* + Ethics Charter (dignity, fairness). This is friction, not veto: it pauses **co-enablement** pending a recorded human decision; it does not block this deploy, the dark code, or either flag turned on **alone**. A conscious human may override with a recorded rationale — but the money-without-obligation trail should be a deliberate, signed choice, never a default.

Owner of the fix: payments council + grace-cancel council jointly (the gap lives at their intersection, which is why neither has owned it).

---

## 3. Non-blocking advisories (aesthetic / strategic / care)

- **Merge-gate the ADR addendum with the code.** It is what converts "green scan" into "honestly recorded exception." Not a follow-up ticket.
- **Pin the shadow transition table.** Add a test/assertion tying `ALLOWED_FROM` to the canonical `TRANSITIONS` (e.g. every state in `ALLOWED_FROM` must exist in the machine; a new order state forces a conscious review of this array). Cheap insurance against silent drift. Consider dropping `IN_DELIVERY` from the array or commenting *why* a legal edge shares the exception list — keep "exception" and "already-lawful" visually distinct.
- **Customer copy is a dignity surface, not a status enum.** `event='CANCELLED'` is honest-terminal but a bare "cancelled" reads as *rejection* / *your fault*. The message should attribute cause truthfully ("no courier was available" — not the customer's doing, not the kitchen's rejection) and, when prepaid, state that a refund is coming. Silence about money after a paid cancel is the actual dignity harm — the refund wire (§2) and this copy are the same act of respect from two sides.
- **The owner eats made-food cost with only a `console.log`.** Consider recording grace-cancel losses somewhere the owner can *see* (so they can act: add couriers, tighten radius). A silent write-off denies the owner the information needed to fix the underlying scarcity. This is strategic, not blocking.

---

## 4. Steel-man of the rejected option (Option A — widen the machine)

Option A is the **conceptually honest end state and it is genuinely better on the axis the codebase itself claims to value**: "the state machine is the sole transition authority." A kitchen that cannot fulfil an order it is preparing *should* be able to terminate it — that is a first-class domain truth, not a worker's private exception. Option A gives one authority, no shadow array, no drift risk, DRY, and the same cash-safety (updateOrderStatus writes no ledger/trace). The proposal's rejection rests entirely on a *coupling accident* — that owner PATCH routes pass request `newStatus` straight into `updateOrderStatus`, so widening the machine leaks an owner capability. That is a real and decisive blocker **today**, but notice it is an argument against the *current wiring*, not against Option A's shape. The clean long-term move is not "prefer B forever" — it is **fix the coupling** (validate owner-permitted target statuses at the route/Zod layer, independent of the machine's legal edges), *then* Option A becomes available with no leak. Option B is right for the deploy; do not let its convenience quietly retire the ambition of A. The proposal, to its credit, keeps A alive as the deferred follow-up — hold it to that.

---

## 5. The question no one asked

The entire document reasons about **invariants**; almost no line reasons about the **let-down customer as a person**. When a dispatch-exhausted order is auto-cancelled, *what is owed to the human who ordered dinner, waited, and maybe paid?* Right now the answer is "a `CANCELLED` push and, if prepaid, nothing." Is a robot-cancel after a 15-minute silence the whole of our obligation — or does dignity ask for an apology, a refund made visible, a re-order path, a "this was on us, not you"? That question belongs to the same council that decides whether to enable grace-cancel at all. Enabling a way to *disappoint* a customer without first deciding how to *treat* one is the gap beneath the refund gap.
