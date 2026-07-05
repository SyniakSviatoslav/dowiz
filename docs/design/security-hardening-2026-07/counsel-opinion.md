# Counsel Opinion — Security hardening batch (blue-team sweep 2026-07-02)

Role: Counsel (advisory). Scope: values · aesthetics · strategy · agent/process health.
Reviewing: `proposal.md` + `ADR-security-hardening-2026-07.md`. Date: 2026-07-02.
Authority: non-blocking. This document does not gate merge; the human decides. It contains
**0 ETHICAL-STOPs** and one grounded precondition (see §2).

---

## 1. Reasoning per lens (only what is load-bearing)

### Justice / stakeholders — who bears the cost
The batch shifts cost the right way: it removes exposure borne by the **customer** (address, phone,
order/payment leaking cross-tenant) and by the **tenant owner** (cross-tenant invite injection,
insider-removal window). No stakeholder is made to pay for another's convenience. The distribution is
fair.

One cost is **under-named**: the **courier cross-customer read** inside a tenant. OR-3 accepts scoping
`GET /orders/:id` (courier branch) to `activeLocationId` rather than to a live assignment binding. Under
the current BYPASSRLS pool that means a courier can enumerate **every order at their venue by UUID —
including addresses/phone of orders assigned to a different courier**. This is not cross-*tenant* (the
headline harm), but it is a genuine **customer data-minimization** gap: a courier sees PII for
deliveries that are not theirs. The proposal correctly reduces exposure versus today, but "accept as
follow-up" quietly demotes a live PII-minimization concern. It should be a **tracked** item, not a
footnote — see §3.

### Dignity / autonomy (courier)
Clean. Every fix *tightens* access; nothing here surveils, coerces, or removes courier agency. #4 and #6
close the insider-removal window (a revoked owner can no longer stream a live feed or write) — that is
pro-dignity for the people whose data the ex-insider would otherwise still see.

### Honesty / consent
Strong. #5 removes bearer tokens from URLs (credential hygiene); log-redaction of `sub`/`role` is good
PII hygiene; the server stays authoritative throughout. No dark pattern is introduced. The one
deontological wrinkle — **404-not-403 on cross-tenant miss** (#1) — is a small "lie" (the resource
exists) chosen to prevent existence-enumeration. Under a care/consequences lens this is the correct,
standard tradeoff; it does not rise to friction. Noted for multiplicity, not objection.

### Care / harm — who gets hurt by the failure mode
The live faucet is being closed, but there is a **puddle already on the floor** that the batch does not
mop: #5 stops *future* JWT-in-URL leaks, but bearer tokens (24h–14d TTL) that were **already** written
to Fly access logs / Referer / browser history during the `?token=` era **remain valid credentials
right now**. Closing the faucet is necessary; it is not remediation of the existing exposure. This is the
one real-user-harm item the proposal under-addresses (see §5, open question).

### Long horizon / strategy
- **Serves the launch trigger, is not polish.** A platform cannot ethically take a *first real paid
  order* while leaking customer PII cross-tenant. This batch is a launch **precondition**, correctly
  urgent.
- **Reversibility is good:** app-code reverts cleanly, #3 is forward-only metadata, #2 ships dark.
  Low lock-in.
- **The regret surface is the three deferrals:** #2 (re-orphan risk), the transitional dual-authority
  becoming permanent, and the courier cross-customer read never getting closed. All three are "later"
  items with no trigger. "Later" is exactly where #2 already died once.

### Aesthetics / integrity
- The unifying root — *identity-split × RLS-reliance* — collapsing 9 findings into one class + one
  guardrail is genuine conceptual coherence, not decoration.
- "Schema rich, runtime minimal" is honored on purpose (app predicate = runtime authority now; RLS
  policies land dark, switch on once, later). That restraint aligns with the grounded aesthetic line.
- The proposal names its own integrity smell (M1: two authorization strategies on one resource). The
  aesthetic prescription: **time-box the dual-authority state to a B3 trigger**, or it silently becomes
  the permanent architecture.

### Epistemic — assumption, missing perspective
- **The load-bearing, unverified assumption is the pool role.** Everything ("correct under both cases",
  "#2 is inert, defer it") rests on the pool being BYPASSRLS. The architect is admirably honest that
  this cannot be read and escalates it to the operator. Good epistemic hygiene — but the *consequence* of
  the assumption being wrong is under-stated (see §2).
- **Missing perspective:** the customer whose data is visible to a non-assigned courier (justice lens
  above), and the holder of a token already leaked to logs (care lens above). Both are absent from the
  "who is harmed" frame.

---

## 2. ETHICAL-STOP — grounded

**None triggered.** The proposal *honors* the grounded red lines it touches: PII-protection (it closes
the leaks), server-authoritative (preserved), human-in-the-loop (pool role and every migration routed to
the operator). Nothing proposed crosses a line; the human gate the charter would demand already exists.

**One grounded precondition (friction, not verdict).** The docs say the pool-role answer "gates only
#2's *effectiveness* and the B3 flip; Tier 1 ships regardless" (OR-1, ADR Open decision). That is
**imprecise in a way that matters for PII.** If the deployed pool is *unexpectedly already NOBYPASSRLS*
(the architect cannot rule this out — that is the whole reason for the escalation), then #2 is **not
latent — it is a live table-wide cross-tenant customer-PII siphon**, and shipping "Tier 1 regardless"
would leave it open while declaring the exposure resolved. Declaring this batch *done / PII-resolved
without the operator's recorded pool-role confirmation* would cross the PII-protection line. So:

- Treat the operator pool-role confirmation as a **hard precondition to closing the batch**, not a
  background risk item. If NOBYPASSRLS today → **#2 is promoted to Tier 1** (live), not deferred.
- This is exactly the recorded human decision the charter wants; the design already routes to it. No
  stop is needed — only sharpen the framing so the confirmation is understood as a *live-exposure gate*,
  not merely an *effectiveness gate*.

---

## 3. Non-blocking strategic / aesthetic advice

1. **Anti-orphan artifact for #2 must exist from the moment Tier 1 ships — not arrive with the fix.**
   The class already recurred once (migration 077 added anon INSERT siblings but never narrowed the
   fail-open SELECT/UPDATE — the proposal's own words: "a genuine open gap in the staged B3 work").
   Handing #2 back to the same B3 track that dropped it, with its guardrail *bundled into #2's own
   delivery*, is circular: the gate arrives with the fix, so it cannot guard the interim. Fix: land a
   **tracked, visible, failing-or-pending artifact now** — a `REGRESSION-LEDGER` red-line row and/or a
   skip-registered `verify:rls` probe that *cannot be silently closed* until #2 narrows C1. Make the gap
   loud, or it re-hides.
2. **Elevate the courier cross-customer read (OR-3) from footnote to tracked item.** Assignment-level
   scoping via `courier-room-authz` is the correct end state for PII minimization; "accept as follow-up"
   is fine only if it is *tracked* follow-up with an owner and a trigger.
3. **Time-box the dual-authority (M1) transitional state to a B3 trigger.** Don't let "until B3
   completes" be open-ended; the app-predicate + inert-RLS split is acceptable transitional, corrosive
   permanent.
4. **Prefer the mechanism you already own for #2 (aesthetic + risk).** The preferred #2 fix invents a
   new per-request scope-GUC discriminator (`app.anon_order_id` / `app.current_tenant`) — a fresh
   "forgot-to-set-the-GUC → fail-open regression" surface. The listed alternative (narrow
   `SECURITY DEFINER` scoped by order-id/token-hash, pinned search_path) reuses the definer mechanism
   #3 is already pinning and guarding — one isolation primitive, not two. Nudge B3 toward it.
5. **Per-finding recorded decision, not one batch stamp.** Nine red-line findings in one approval is a
   mild convergence-theater risk; the subtle ones (the #2 pool-conditional, the #1 courier
   `softVerifyAuth`/OR-3 accept) are where a spread-thin reviewer waves things through. Record the human
   decision **per tier + explicitly on OR-1 (pool) and OR-3 (courier-assignment accept)**, not as a
   single "approve batch."
6. **Escape-hatch discipline (Goodhart).** The static sweep's `-- no-location-id` escape comment is the
   exact seam the class re-enters through. OR-6 already says "reviewed additions only" — good; make that
   enforced (any new escape comment = required reviewer), not aspirational.

---

## 4. Steel-man of a rejected option — Option B (one atomic B3 batch)

The strongest case *against* the chosen Option C, stated at full strength:

The root is a **data-layer** root — identity-split × RLS-reliance. Option C's promise is "app layer is
authoritative *now*, RLS becomes primary *later*." But "*later*" is precisely where this class already
died once (partial 077). Option C therefore institutionalizes, for an **unbounded transitional period**,
the very M1 smell it names — two authorization strategies on one resource — and splits ownership of the
boundary across two tracks, so #2 again falls *between* the route-fix batch and the B3 track. Option B
forces the tenant boundary to be made correct as **one coherent, atomically-proven, single-owner unit**:
one boundary, one mechanism, one proof, no inert-RLS window, no split that something can slip through. It
is the more *elegant* answer and the one that closes the class at its root in a single reviewable change —
and it avoids the exact failure mode (orphaned partial RLS work) that has *already happened* in this
codebase.

**Why C still wins (honest rebuttal):** #1/#7 are bleeding *now*; Option B's role flip has high blast
radius and cannot ship atomically without the KNOWN TRAP (flip before C1-narrowing + courier GUC seating
→ leak or break). Forcing everything into B holds the live-PII fixes hostage to the slow, high-risk flip.
So C is right on urgency + risk. **But the steel-man extracts the price of C:** C is only defensible *if
it carries the explicit anti-orphan mechanism for #2* (advice §3.1). Without it, C inherits B3's
demonstrated failure. That is the actionable synthesis — adopt C, but pay its premium.

---

## 5. Open question no one asked

**What remediates the tokens already leaked during the `?token=` era?** #5 closes the faucet, but bearer
credentials with up to a **14-day TTL** that were written to Fly access logs, Referer headers, and
browser history while URL-auth was live are **valid right now**. The batch prevents *new* leaks; it does
not drain the existing puddle. Who owns the exposure-window remediation — key rotation, forced
session/refresh invalidation for the affected roles, or historical access-log scrubbing — and is it in
scope for this batch or a named follow-up? A fix that closes future leaks while leaving live leaked
credentials unaddressed has done the necessary but called it sufficient.

(Secondary, lower-confidence: does the current phone/IP-degraded idempotency `requestHash` (#8) ever risk
an idempotent replay returning one party's order response to another under a shared IP, and does the #8
one-time key shift open any transient window? Likely benign — phone is in the key — but worth one look
before calling #8 "not security-critical.")
