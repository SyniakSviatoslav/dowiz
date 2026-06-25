# Breaker Findings — fee-courier-seed (Option A′ / FE-only courier / encrypted dev-seed)

Breaker: System Breaker DeliveryOS. Method: read-only verification of every load-bearing claim
against live source (`orders.ts`, `lib/money.ts`, `public/menu.ts`, `dev-guard.ts`, `server.ts`,
`mock-auth.ts`, migrations). Findings ranked by real severity. NO fixes — only how it breaks and which
invariant is violated.

Verdict headline: **the proposal's own safety claim — "server total is always final, a mismatch is
reconciled toward the server number, never away from it" (§1.4) — is FALSE on the cash-on-delivery
path.** The replicated `estTotal` becomes the literal amount of money collected at the door AND gates a
hard server-side 422. That is the dangerous one. Item 3's prod-safety proof holds; Item 2 has a real
inverse-lie. Details below.

---

## CRITICAL

### C1 · [B-CONSIST / MONEY] The "server total is always final" invariant is violated on cash-on-delivery — the replicated CTA number IS the money collected at the door, and it gates a server 422.

- **Claim under attack (proposal §1.4 / ADR-0005 §Decision pt.5):** "the server-returned `total` … is
  always the final authority … a mismatch is reconciled toward the server number, never away from it …
  even the replicated path can never overcharge."
- **Where it breaks (verified):** `CheckoutPage.tsx:428` sends `cash_pay_with: cashAmount`. The cash UI
  is keyed to the FE `total`, not the server total: `:839` red-borders the input when
  `cashAmount < total`; `:867` shows "exact"; `:870` computes `change = cashAmount - total`. The server
  then HARD-REJECTS: `orders.ts:568` `if (cashPayWith !== undefined && cashPayWith < total) → 422
  CASH_AMOUNT_TOO_LOW`. Under Option A′, FE `total` becomes `estTotal`. So:
  - **Break scenario A (charged wrong at the door):** flat-fee venue, but config is stale in the 30s
    `/info` cache (or the owner just changed `delivery_fee_flat`). Client `estTotal = 1500`, server
    `total = 1750`. Customer pays the courier the CTA number (1500) in cash. The order persists with
    server `total = 1750`. Courier collects 1500, app says 1750 owed → 250 ALL shortfall **at the door,
    real money, every order until the cache expires.** The confirmation screen "reconciling to the
    server number" does nothing — the cash already changed hands against the CTA.
  - **Break scenario B (customer cannot check out):** customer types `cashAmount = estTotal = 1500`
    (FE says "exact", green). Server `total = 1750` → `1500 < 1750` → **422 CASH_AMOUNT_TOO_LOW**. The
    FE has NO handler for that code (`:491` only handles `MIN_ORDER_NOT_MET`); it falls into the generic
    "failed to place order". A customer who entered the exact amount the UI told them is blocked with a
    cold error.
- **Violated invariant:** "server total is the single source of truth, the client number can never
  become the charge" (ADR-0005). On cash-on-delivery the client number IS the charge (it is what the
  human hands over) and it is also a server gate. Reconciliation-after-the-fact is structurally
  impossible for cash.
- **Why CRITICAL not HIGH:** money at the door, mismatch is silent (under-quote) or a dead-end
  (over-strict), and it occurs precisely on the path the proposal asserts is safe. Assume the
  implementer does the literal minimum (§1.4 code: set `total = estTotal`); they will wire the existing
  cash UI to it and ship the bug.

### C2 · [B-CONSIST / MONEY] `applyTaxClient` "exact integer mirror" is under-specified against the REAL server algorithm — the proposal's stated formula does not match `lib/money.applyTax` for tax-inclusive venues.

- **Claim under attack (§1.4):** "`applyTaxClient` must be an exact integer mirror of
  `apps/api/src/lib/money.applyTax` (same rounding — half-up, same minor-unit handling)." The ADR pseudo
  (§Decision pt.2) writes only `total = subtotal + fee + tax`.
- **What the server ACTUALLY does (verified, `lib/money.ts:23-44`):** tax is NOT a single "half-up"
  rule. There are two distinct BigInt algorithms:
  - tax-EXCLUDED: `tax = (sub*rateMicro + 500000) / 1_000_000` with `rateMicro = round(taxRate*1e6)`.
  - tax-INCLUDED: `net = (sub*1e6 + denom/2)/denom` where `denom = 1e6 + rateMicro`, then `tax = sub − net`.
    This is a *derived* tax (back-out), not a forward half-up — a naive `round(subtotal*rate)` mirror
    will diverge by 1 minor unit on a large fraction of inputs.
  - The rate itself is quantised to **6-dp micro-units via `BigInt(Math.round(taxRate*1_000_000))`** —
    any client mirror that keeps `taxRate` as a JS float and multiplies directly will drift.
- **Break scenario (number):** tax-inclusive venue, `tax_rate = 0.20`, `subtotal = 1003` (minor units,
  some non-ALL currency with minor_unit 0). Server: `denom = 1_200_000`; `net = (1003*1e6 + 600000)/1.2e6
  = floor(1003600000/1200000) = floor(836.33) = 836`; `tax = 1003 − 836 = 167`. A mirror doing
  `round(subtotal*rate/(1+rate)) = round(1003*0.2/1.2) = round(167.166) = 167` happens to match here, but
  on `subtotal = 1007`: server `net = floor((1007000000+600000)/1200000) = floor(839.66) = 839`, tax =
  168; naive mirror `round(1007*0.2/1.2)=round(167.83)=168` — matches — but the half-EVEN vs half-UP and
  the `denom/2` bias terms diverge on values where the fractional part lands within 1/SCALE of .5. The
  proposal does not pin WHICH of the two server branches the mirror replicates, nor the micro-unit
  quantisation, so "exact mirror" is an aspiration, not a spec.
- **Violated invariant:** money integer-exactness / `clientTotal == serverTotal`. The §1.6 parity
  guardrail is the only thing standing between this and shipped drift — and the guardrail is described,
  not written. **The proposal leans the entire money-correctness claim on a test that does not yet exist
  and whose matrix (§1.6) omits the tax-INCLUDED back-out branch and non-zero minor_unit explicitly.**
- **Note:** for ALL (tax_rate 0, minor_unit 0) tax collapses to 0 and this is moot — but the proposal's
  shared `applyTaxClient` is presented as general-purpose and will be reused for the non-ALL venues the
  product is heading toward. Severity CRITICAL because it is a money formula presented as "exact" while
  being unspecified; downgrade in practice to HIGH if the venue universe is provably ALL-only forever.

---

## HIGH

### H1 · [B-CONSIST / MONEY] FE min-order soft-gate and server 422 compare on DIFFERENT quantities → a customer who can't check out, or one let through then rejected.

- **Server (verified `orders.ts:519`):** `if (location.min_order_value !== null && subtotal < min_order_value)`
  — compares against **subtotal (pre-tax, pre-fee, pre-tip)**, and runs for **delivery AND pickup**
  (it is above the `isPickup` branch — no pickup exemption).
- **Proposal (§1.4 code):** `belowMin = minOrderValue != null && deliveryType === 'delivery' && subtotal < minOrderValue`
  — gates **delivery only**.
- **Break scenario:** a venue with `min_order_value` set. Customer chooses **pickup** with subtotal below
  min. FE `belowMin = false` (pickup excluded) → Order enabled → submit → server `:519` does NOT exempt
  pickup → **422 MIN_ORDER_NOT_MET**. The exact "422 surprise after submit" the proposal claims to
  eliminate (§1.3 Option A pro: "min-order can be enforced FE-side before submit") still fires, on
  pickup. The FE and server disagree on the predicate.
- **Second divergence:** FE `subtotal = items.reduce(price*qty)` (`CheckoutPage.tsx:341`) — does this
  include modifier prices? Server subtotal is `computeLineTotal(price, modifierPrices, qty)`
  (`orders.ts:506`) which ADDS modifier prices. If the cart `item.price` is the base product price
  without modifiers, FE subtotal < server subtotal → FE soft-gates a customer who is actually above min
  (locks them out), or vice-versa. **Verify cart `item.price` already folds in modifiers; if not, the
  two min-order checks compare different numbers.**
- **Violated invariant:** FE gate and server gate must be the same predicate over the same operands, or
  the soft-gate is a new source of false-block / false-allow.

### H2 · [B-CONSIST / MONEY] `free_delivery_threshold` boundary + tax interaction is unspecified — threshold compared on subtotal, but tip and tax are not in scope of the comparison, and the proposal never states subtotal-before-vs-after-tax.

- **Server (verified `orders.ts:530`):** threshold compared against **raw subtotal** (`subtotal >=
  free_delivery_threshold`), independent of tax. Good. The proposal's client formula (§1.4) also uses
  `subtotal >= freeDeliveryThreshold` — matches. **No bug at the boundary itself.**
- **Residual risk (MED, folded here):** for a **tax-INCLUDED** venue, the displayed `subtotal` the
  customer sees and the `subtotal` used for the threshold are the same integer — fine. But if any future
  surfacing shows a tax-exclusive subtotal in the cart while the threshold compares the inclusive number,
  the customer crosses the visible threshold but the fee does not waive. The proposal does not pin which
  subtotal is shown vs compared. Flag, not a today-break for ALL.
- **Tip:** `tip_amount` (`CheckoutPage.tsx:419`) is added to the order but the proposal's `estTotal =
  subtotal + estDeliveryFee + estTax` **omits tip**. If the CTA `Porosit • {estTotal}` is meant to be
  "what you pay" and tip is non-zero, the CTA under-quotes by the tip. Verify the server `total`
  (`orders.ts:565`) also excludes tip (it does — tip is not in the `total` expression) so they agree on
  *total*, but the cash `cash_pay_with` check (§C1) is against `total` which excludes tip — yet the
  customer hands over cash including tip. Tip is an unmodelled term in the cash reconciliation.

### H3 · [B-DATA / MONEY] `currency_minor_unit != 0` is claimed handled but the rounding direction is never proven for the `estDeliveryFee` and the free-threshold are raw integers, not minor-unit-scaled.

- The proposal repeatedly asserts "all integer ALL, minor_unit 0" and then claims the formula
  "generalises". But `delivery_fee_flat`, `free_delivery_threshold`, `min_order_value` are stored as raw
  minor-unit integers; `applyTax`'s `_minorUnit` parameter is **ignored** in the server
  (`lib/money.ts:23` — `_minorUnit` is unused; `roundHalfUp` exists but `applyTax` does NOT call it). So
  the server tax does not actually round to the currency's minor unit at all — it returns a raw
  per-minor-unit BigInt result. A client mirror that DOES honour `currencyMinorUnit` (as §1.4 passes it
  in) would round to a coarser unit than the server → divergence on any non-zero-minor-unit currency.
- **Break:** EUR (minor_unit 2), tax-excluded 20%, subtotal 1007 cents. Server tax = 201 cents (no
  minor-unit rounding — already in cents). A mirror that interprets `currencyMinorUnit=2` as "round to
  2dp" is a no-op here, but a mirror that rounds to the *major* unit diverges. The contract is
  ambiguous because the server's own `_minorUnit` is dead. The mirror has nothing coherent to mirror.
- **Violated invariant:** money formula must be deterministic and identical across client/server. The
  server's minor-unit handling is itself underdefined (dead param), so "exact mirror" inherits the
  ambiguity.

### H4 · [B-SEC / B-CONSIST] Item 2 inverse-lie: a genuinely-online courier is rendered "Pending setup" (never green) because the list endpoint cannot prove presence — the new model is honest about absence but actively WRONG about presence.

- **Verified:** Option A derivation (§2.4, ADR-0006) — `status==='active' AND (!maskedPhone || !lastLoginAt)
  → 'pending_setup'`, else `'active'` (never green/"online"). The list endpoint hard-codes
  `onlineStatus: null` (`couriers.ts:53`) and does NOT join `courier_shifts`.
- **Break scenario (capacity misread for dispatch):** a courier with phone + past login is RIGHT NOW on
  an `available` shift (the real online set per `couriers/live`, `couriers.ts:141`). On this screen they
  render flat "Active" — indistinguishable from a courier who logged in once last week and is offline.
  The owner reading "3 active" for a dispatch decision (do I have couriers to take this order?) gets a
  count of **enabled accounts, not available couriers** — could be 3 active accounts and 0 on shift. The
  proposal acknowledges this (R4, "loses live presence") but frames it as a clean trade; in practice
  "N active" on a screen titled like a fleet view will STILL be read as capacity. The lie moved from
  "all green" to "all neutral" — both misrepresent dispatch capacity, just in opposite directions.
- **Inverse direction:** an active, on-shift courier whose `lastLoginAt` is null because they
  authenticate via a path that doesn't stamp `last_login_at` (verify the courier-app login writes it) →
  shown "Pending setup" forever despite being the most active courier. The derivation treats
  `lastLoginAt == null` as onboarding-incomplete; if any real login path leaves it null, real couriers
  are libelled as un-onboarded.
- **Contract consumer check:** `onlineStatus` is in the shared contract
  (`packages/shared-types/.../couriers.ts:10`, enum `online|busy|offline|null`) and `mockData.ts`
  consumers populate it. The list endpoint returns `null`; Option A's FE ignores it. If the deferred
  Option B later populates `onlineStatus` from shifts, BOTH the "N active" account-count and the new
  presence field will coexist and can disagree on the same row — two truth sources on one card. The
  proposal defers the reconciliation rule.
- **Violated invariant:** display-truth (don't assert what you can't prove) — satisfied for green; but
  the new model asserts "pending_setup" (a negative claim) it equally cannot prove.

---

## MEDIUM

### M1 · [B-OPS / MONEY] `/info` 30s cache makes the replicated CTA structurally stale against an owner fee change — the divergence window is bounded but real and silent.

- **Verified:** `MENU_CACHE_TTL_MS` 30s (`menu.ts:202-211`), stale-on-error up to 1h
  (`:224`). An owner who changes `delivery_fee_flat` / `free_delivery_threshold` mid-service has up to
  **30s (or up to 1h on a DB hiccup, stale-on-error)** of customers seeing a replicated CTA computed
  from the OLD fee, while `orders.ts` reads the NEW fee live (no cache on the order path). Every order in
  that window: CTA ≠ charge → feeds C1 (cash-at-door) and C2 (cash 422). The proposal's "zero added
  query cost, fields ride the cached row" (§1.2) is exactly what makes the divergence silent.
- **Number:** at the stated peak 1–30 orders/min, a 30s window = up to **15 mis-quoted orders** per fee
  change; the 1h stale-on-error window during a DB blip = up to **1800**.
- **Violated invariant:** read-after-write on fee config — there is none; the CTA reads stale, the
  charge reads fresh.

### M2 · [B-SEC / B-DATA] §1.5 `delivery_tiers` EXISTS subquery on the PUBLIC `/info` role — the proposal itself flags it (R2) but ships the boolean anyway; if RLS hides the rows, `has_distance_tiers` returns FALSE and a tiered venue silently takes the WRONG (flat/exact) replication path.

- **Verified:** `/info` runs on the public operational role (`server.db.query`, no `set_config` tenant
  context in `refreshInfoRow`, `menu.ts:181`). `delivery_tiers` RLS is
  `USING (location_id IN app_member_location_ids())` (per proposal §1.5) — the anonymous public role has
  NO membership → the EXISTS returns **false** for every venue.
- **Break:** a genuinely distance-tiered venue → `has_distance_tiers = false` (RLS-hidden) →
  `feeKnown = true` → client replicates a flat/exact number for a venue whose real fee is
  distance-dependent → CTA is a fabricated precise number → C1/C2 fire with a potentially large delta
  (tier fees span ranges). The proposal's R2 is marked "open, escalate" but the §1.4 code path assumes
  `hasDistanceTiers` is trustworthy. Until R2 is resolved, **the degrade path never triggers** and the
  exact-replication path lies for tiered venues — the precise failure the whole Option A′ hybrid exists
  to avoid.
- **Violated invariant:** tenant-RLS read correctness AND the design's own "no false precision on tiered
  venues" guarantee. The boolean's failure mode is the unsafe direction (false → claim-exact), not the
  safe one.

### M3 · [B-CONSIST] Item-3 seed `courier_shifts` re-run idempotency is hand-waved — "DELETE this courier's open shifts" is described in prose (§3.5) but the §3.4 code does a bare INSERT with no DELETE, so re-running the seed accumulates shifts and the assignment's `shift_id` points at an arbitrary one.

- **Verified:** `courier_shifts` has **no natural key** (migration `1780421036157`); `/dev/create-assignment`
  uses `ON CONFLICT DO NOTHING` but `courier_shifts` has no unique constraint to conflict on, so
  `ON CONFLICT DO NOTHING` there is also effectively a plain insert. The §3.4 seed code is
  `INSERT INTO courier_shifts (...) VALUES (...,'on_delivery',...) RETURNING id` — **no dedup**. The
  prose at §3.5 says "DELETE this courier's existing shifts before inserting" but the shown code does
  not. Implementer-does-the-literal-minimum → ships the bare INSERT.
- **Break:** N visual-capture runs → N open shifts for the seed courier, all `on_delivery`. The
  `couriers/live` query (`cs.status IN ('available','on_delivery')`) now counts the seed courier N times
  / shows duplicate live rows; if any other test asserts shift counts it flakes. Not prod (dev-gated),
  but it breaks the "re-running converges to the same state" idempotency claim (§3.6).
- **Also:** §3.4 seeds `status='on_delivery'` for the shift but `/dev/create-assignment` (the proven
  pattern it claims to mirror) uses `'available'`. The assignment is `status='accepted'`. A shift in
  `on_delivery` with an `accepted` (not yet `picked_up`) assignment is an inconsistent state-machine
  snapshot — harmless for a static screenshot, but it is NOT the "mirror the proven path" the proposal
  asserts.

### M4 · [B-DATA] Item-3 `email_hash` collision/shadow risk is real if the synthetic constant ever overlaps a real courier, and the `ON CONFLICT (email_hash) DO UPDATE SET status='active'` can RESURRECT a deactivated/suspended real courier.

- **Verified:** `email_hash text NOT NULL UNIQUE` (`1780421029538:8`). The seed
  `ON CONFLICT (email_hash) DO UPDATE SET status='active', last_login_at=now()`.
- **Break (low prob, high blast if it lands):** `email_hash = sha256('vis-courier@dowiz.test')`. If a
  real courier ever registers that exact address (it is a `.test` TLD — registration may reject it, OR
  may not validate TLD), the seed's `DO UPDATE` flips their `status` to `'active'` and overwrites
  `last_login_at` — **re-activating a suspended courier and stamping a fake login**, on staging. On a
  shared staging DB used for both visual capture and manual courier testing, a tester who happens to use
  a `.test` email is silently un-suspended. The proposal asserts "no real PII / synthetic constants" but
  the conflict target is a hash of a string that is not provably unique against the real courier
  namespace. Verify courier registration rejects `.test` TLD; if not, this is a privilege-state write to
  a real row via a dev seed.
- **Violated invariant:** dev-seed must not mutate real account state. `DO UPDATE SET status='active'`
  on a UNIQUE business key is a write that reaches beyond synthetic rows iff the key collides.

---

## LOW

### L1 · [B-SEC] Item-3 `body.courierId` impersonation is correctly gated, but the proposal's framing ("if the gate ever leaked") understates that mock-auth ALREADY signs an arbitrary courier token.

- **Verified — gate holds:** `mock-auth` is under `/dev/` → `isDevRequestAuthorized` requires
  `ALLOW_DEV_LOGIN==='true' AND DEV_AUTH_SECRET set AND matching x-dev-auth-secret`
  (`dev-guard.ts:30-61`, `server.ts:521-524`), fails closed with 404. Prod sets neither (memory:
  prod ALLOW_DEV_LOGIN off, no secret). So adding `body.courierId` → `sub: body.courierId` is NOT a new
  prod surface. **The "impossible in prod" claim is PROVEN for the dev gate.** Confirmed: every
  `/dev/*` and `/api/dev/*` path 404s without the flag+secret; no fallthrough observed (single
  `onRequest` hook at `server.ts:514`, no route registered outside the prefix).
- **Residual (LOW):** once the gate is open (staging, with the shared secret), `body.courierId` signs a
  valid courier JWT for **any UUID the caller supplies** — including a real courier's id → a token for
  someone else's account, cross-tenant via `activeLocationId` also caller-supplied
  (`mock-auth.ts:13,19`). On staging this is an impersonation primitive for anyone holding the staging
  `x-dev-auth-secret`. Accepted-by-design for a dev tool, but it is broader than "impersonate the seeded
  courier" — it impersonates arbitrary courier ids. Note for the secret's blast radius, not a prod break.

### L2 · [B-OPS / MONEY] The `CHECKOUT_FEE_REPLICATION` kill-switch (§1.9) is "optional, default on" — so the dangerous path (C1/C2) is the DEFAULT and the safe fallback ("confirmed at checkout") is the off-state. A kill-switch that defaults to the risky behaviour is not a safety mechanism, it is a post-incident toggle.

- The §1.7 degrade path ("confirmed at checkout", order never blocked by a client guess) is provably
  safe re: C1/C2 (no cash gate keyed to a client number). The proposal makes replication (the C1/C2
  carrier) the default and the safe path the exception. Violated invariant: default-safe. The honest
  default per the breaker matrix is Option B (estimate, server-confirmed), with replication as the opt-in.

### L3 · [B-DATA] Item-3 `courier_assignments` ON CONFLICT (order_id) DO UPDATE re-points an existing real assignment if the seeded `order_id` is ever reused.

- **Verified:** `courier_assignments_order_uniq` UNIQUE on `order_id` (`1780421100041:23`). The seed's
  `ON CONFLICT (order_id) DO UPDATE SET courier_id=EXCLUDED.courier_id, ...` will hijack the assignment
  of whatever order the seed picked, re-pointing it at the seed courier. The seed creates its own order
  (mock-auth seed step 6) so the order_id is synthetic — LOW. But the UPDATE is unconditional on the
  conflict key; if the visual-order id collides with a real order on staging, a real order's courier
  assignment is silently reassigned. Bounded by the seed owning the order; flagged for completeness.

---

## Regression / cross-item

- **C1 is the regression risk:** the existing hardcoded `deliveryFee=200` is ALSO wrong on cash (same
  mechanism), so C1 is a pre-existing latent bug. But the proposal's value prop is "make the CTA exact"
  — shipping an *exact-looking* number that customers and couriers TRUST as the door amount makes C1
  worse, not better, because a confidently-wrong number is acted on where an obviously-rough "200" might
  be questioned. The fix's success metric (CTA matches charge) is the same property that weaponises the
  cash-door mismatch when the cache is stale or the venue is tiered.

## Severity summary

| ID | Sev | Vector | One-line |
|----|-----|--------|----------|
| C1 | CRITICAL | B-CONSIST/MONEY | Cash-on-delivery: replicated CTA = money at door + gates server 422; "server total final" is false for cash |
| C2 | CRITICAL | B-CONSIST/MONEY | `applyTaxClient` "exact mirror" unspecified vs real 2-branch BigInt `applyTax`; parity guardrail unwritten |
| H1 | HIGH | B-CONSIST | FE min-gate (delivery-only) ≠ server 422 (delivery+pickup); modifier-price subtotal mismatch |
| H2 | HIGH | B-CONSIST | tip omitted from estTotal/cash reconciliation; tax-inclusive subtotal display vs threshold ambiguity |
| H3 | HIGH | B-DATA | server `applyTax` `_minorUnit` param is DEAD; mirror has nothing coherent to mirror for non-ALL |
| H4 | HIGH | B-SEC/CONSIST | courier inverse-lie: on-shift courier shown "Pending setup"; "N active" misread as dispatch capacity |
| M1 | MED | B-OPS | 30s (up to 1h stale-on-error) `/info` cache → silent CTA-vs-charge divergence on fee change |
| M2 | MED | B-SEC/DATA | `delivery_tiers` RLS hidden from public role → `has_distance_tiers=false` → tiered venue takes exact path (lies) |
| M3 | MED | B-CONSIST | seed shift re-run not idempotent (code has no DELETE the prose promises); shift status mismatch |
| M4 | MED | B-DATA | seed `ON CONFLICT (email_hash) DO UPDATE status='active'` can resurrect a real suspended courier |
| L1 | LOW | B-SEC | dev gate PROVEN closed in prod; `body.courierId` impersonates ARBITRARY courier on open staging |
| L2 | LOW | B-OPS | kill-switch defaults to the risky path (replication on), safe degrade is opt-out |
| L3 | LOW | B-DATA | seed `ON CONFLICT (order_id) DO UPDATE` re-points assignment if order_id collides |

Prod-safety claim for Item 3 (§3.8): **upheld** (L1) — no path reaches the seed with the flag off or
secret absent; single `onRequest` gate, no fallthrough. The attack surface is the money path (C1/C2),
not the dev seed.

---

# RE-ATTACK round 2 — regression pass on the REVISED design (post-resolution, human chose Item 3-(b), cash-parity PROCEED)

Breaker: System Breaker DeliveryOS. Method: read-only re-verification of the REVISED `proposal.md` +
`resolution.md` + `ethical-decisions.md` against live source (`orders.ts`, `lib/money.ts`,
`CheckoutPage.tsx`, `courier/auth.ts`, `dev/mock-auth.ts`, `cartReconcile.ts`, `packages/domain`,
`shared-types`). Focus: did the FIXES open NEW holes, and do the four Item-3-(b) constraints actually
close the abuse surface on a leaked dev gate?

**Headline of this round: the cash-parity fix (C1/ETHICAL-STOP-1) is built on a server endpoint that
DOES NOT EXIST.** The whole "estimate-hint → review **server total** → confirm cash → submit" flow
requires the client to obtain the server-authoritative `total` *before* the irreversible commit. Verified
against `orders.ts`: there is **no price-quote / preview / preflight endpoint that returns `total`**. The
`total` is computed at exactly one place — `orders.ts:565`, **inside the order-INSERT transaction**, three
lines before the INSERT (`:596`) — and is first returned to the client only in the **201 create response**
(`:761`), i.e. *after the order already exists*. This is a chicken-and-egg that breaks the load-bearing
door-parity claim. **NEW-C1 below.** Everything else is secondary.

Prior CRITICAL/HIGH that ARE genuinely closed by the revision are listed explicitly at the end.

---

## CRITICAL (new — introduced by the fix)

### NEW-C1 · [B-CONSIST / MONEY] The cash-parity fix assumes a server-total "review step" that has no backing endpoint — `total` is only computable INSIDE the create transaction, so the customer cannot review the server total BEFORE the irreversible commit. The chicken-and-egg the round-1 C1 fix was supposed to dissolve is re-created one layer up.

- **Claim under attack (proposal §1.4 / §1.6 AC-CASH-PARITY / resolution C1 pt.1-2 / ethical-decisions
  ETHICAL-STOP-1):** "Before the customer can place a cash order, the UI displays the **server-computed
  `total`** (live fee math, not the estimate) in a review step the customer passes through… via an
  explicit **server-confirmed review step** (a price preflight)… the customer reviews the **server total**
  (returned by an order *preflight* that runs the exact `orders.ts` fee math without persisting, OR by
  surfacing the existing soft-confirm/hard-block preflight already on `POST /orders`)."
- **Where it breaks (verified, live source):**
  1. **No preflight returns a price.** Searched `apps/api/src/routes/` for `preflight|preview|quote|
     estimate|/price|computeTotal`. The only `preflight` is `evaluatePreflight` (`orders.ts:332`) — a
     **fraud/velocity** gate. Its two non-clean responses carry **no money**: `hard_block` →
     `422 {outcome, reasons}` (`orders.ts:341`); `soft_confirm` → `200 {outcome, reasons, requiresOtp,
     requiresConfirmation}` (`orders.ts:346-351`). **Neither returns `subtotal`, `deliveryFee`, `tax`, or
     `total`.** The resolution's parenthetical "OR by surfacing the existing soft-confirm/hard-block
     preflight already on POST /orders" is **factually wrong** — that preflight does not compute or expose
     the fee at all (it runs at step 4e, the fee math is step 8-9, at lines 518-565, which the soft_confirm
     ROLLBACK at `:345` returns *before reaching*).
  2. **`total` is computed only inside the INSERT transaction.** `total = subtotal + deliveryFee +
     taxTotal - discountTotal` at `orders.ts:565`; the very next persisted statement is the `INSERT INTO
     orders` (`:596`). The fee math (min-order 422, free-threshold, distance-tier DB read `:533`, flat,
     `applyTax` `:563`) is **not** factored into a callable pure function — it is inlined in the create
     path, interleaved with `client.query` ROLLBACK/INSERT side-effects. There is **no** "run the exact
     `orders.ts` fee math without persisting" function to call; the resolution describes one as if it
     exists.
  3. **`total` first reaches the client in the 201 create response** (`orders.ts:756-761`) — i.e. the
     order is already PENDING, the timeout job is enqueued (`:678`), the notify outbox is written
     (`:688`), the customer token is issued (`:746`), the bus event is published (`:704`). By the time the
     client knows the server `total`, **the irreversible commit has happened** (`COMMIT` at `:699`).
- **Break scenario (the door-parity claim collapses):** To satisfy AC-CASH-PARITY as written, the
  implementer has two options, both broken:
  - **(i) Build the missing preflight endpoint.** This is a NEW server surface the proposal claims
    requires "no new endpoint" (§1.2: "No new endpoint, no new DB round-trip") and "Not changing the
    server fee math" (§1.1 non-goals). It must duplicate the fee math (min-order/free-threshold/tier/flat/
    tax) — re-introducing exactly the **second copy of money logic** that C2's "share, don't mirror"
    resolution swore to eliminate. The distance-tier branch (`orders.ts:533-552`) reads `delivery_tiers`
    under tenant context and needs the delivery pin — a preflight would have to replicate the pin-distance
    walk server-side too. This is a whole sub-feature, undesigned, hidden inside a one-line acceptance
    criterion.
  - **(ii) "Review" the 201 response total — i.e. create the order, THEN show the total, THEN collect
    cash.** This is the **original round-1 C1 structure renamed**: the order is committed (PENDING, on the
    owner's board, timeout ticking) before the customer has agreed to the cash amount. If they balk at the
    higher server total, there is now an orphan PENDING order the auto-cancel sweep must reap, and the
    `cash_pay_with` was already sent in the create payload (`CheckoutPage.tsx:428`) keyed to the estimate
    → `orders.ts:568` `cashPayWith < total` → **422 CASH_AMOUNT_TOO_LOW on the order that was just
    created**. You cannot "review then confirm cash" against a number that only exists *because you
    already submitted the cash*.
- **Violated invariant:** door-handover parity / "shown == collected" (ethical-decisions ETHICAL-STOP-1,
  the PRIMARY acceptance criterion the human signed off on as "no ship without it"). The criterion is
  **not satisfiable by the design as written** — it presumes a server price-preflight that does not exist
  and that the proposal's own non-goals forbid building. The human approved a PROCEED on a guarantee whose
  mechanism is absent.
- **Why CRITICAL not HIGH:** it is the money red-line the entire C1/ETHICAL-STOP-1 resolution rests on;
  the gap is structural (not a tuning bug); and the implementer following the literal text will reach for
  option (ii) — committing the order before cash confirmation — which is *worse* than round-1 C1 (now
  there's an orphan order AND a 422), or will silently fall back to keying cash to `estTotal` (the exact
  round-1 C1 bug, un-fixed). The "fix" did not close C1; it relocated it behind a non-existent endpoint.
- **What would actually close it (NOT a fix — stating the missing invariant):** the server fee math must
  be a pure, side-effect-free function returning `total` (callable both by a real `/orders/quote` preflight
  AND by the create path), AND the cash amount must be sent only *after* that quote, AND the create path
  must validate `cash_pay_with` against the *same quoted total under the same MVCC/snapshot* — none of
  which the revised proposal specifies. Until then AC-CASH-PARITY is aspirational.

---

## HIGH (new — introduced by the fix / under-closed)

### NEW-H1 · [B-CONSIST / MONEY] Even granting a quote endpoint, the quoted total and the create-path total are read at DIFFERENT times against DIFFERENT MVCC snapshots and a 30s-cached `/info` — the "reviewed number == charged number" parity has no transactional anchor.

- **Where it breaks (verified):** the create-path `total` is computed inside the order transaction from a
  **fresh** read of `locations` fee columns + a **live** `delivery_tiers` query (`orders.ts:533`,
  same-snapshot MVCC, `:383-385` comment). Any "review step" total — whether from `/info` (30s cache,
  `menu.ts:202`) or a future quote endpoint called seconds earlier — is a **separate read at a separate
  time**. If the owner changes `delivery_fee_flat` / `free_delivery_threshold` between the review and the
  submit (or the `/info` cache is stale), the reviewed number ≠ the committed `total`, and the cash gate
  (`orders.ts:568`) fires on a number the customer never saw. M1 was marked accept-risk "corrected at the
  review step" — but the review step itself is a stale read with no lock against the create read.
- **Number:** at the stated 1–30 orders/min peak, a fee change opens a 30s window (up to 1h stale-on-error)
  where reviewed ≠ charged for up to ~15 (resp. ~1800) cash orders. Resolution M1 claims this is
  "harmless… corrected at the review step"; verified FALSE — the review and the charge are two unsynchronised
  reads.
- **Violated invariant:** read-after-write / single-snapshot consistency between the figure shown at
  review and the figure charged at commit. The design provides no `request_hash`-style binding of the
  reviewed total to the committed total.

### NEW-H2 · [B-CONSIST] The FE cart `subtotal` (H1 resolution) cannot be made modifier-parity-exact FE-side — `cartReconcile.ts` itself documents that a modified line "bundles deltas the base price can't verify FE-side", so the H1 "verify item.price folds modifiers" acceptance criterion is unsatisfiable for modified lines and silently degrades the min-gate.

- **Claim under attack (resolution H1 pt.2 / proposal §1.4 H1):** "the cart `subtotal` MUST equal the
  server's modifier-inclusive `computeLineTotal` sum… Implementation MUST verify `item.price` already
  folds modifier prices; if not, the FE computes a modifier-inclusive subtotal via the shared
  `computeLineTotal`."
- **Where it breaks (verified):** `CheckoutPage.tsx:341` `subtotal = items.reduce(price*qty)`.
  `cartReconcile.ts:29-30` states the FE *cannot* verify modifier deltas: "A line with modifiers bundles
  deltas the base price can't verify FE-side, so it's [skipped from reprice]." So the FE `item.price` for
  a modified line is a **stored bundled snapshot**, not recomputed from current menu modifier prices. The
  server recomputes `computeLineTotal(product.price, modifierPrices, qty)` from the **current** modifier
  `price_delta` (`orders.ts:485,506`). If a modifier's `price_delta` changed since the line was added,
  FE subtotal (stale bundled) ≠ server subtotal (fresh) → the min-gate compares different numbers and the
  free-threshold waiver flips at a different boundary. The H1 resolution's "use the shared
  `computeLineTotal`" cannot help: the FE does not hold per-modifier current prices for a stored line (by
  `cartReconcile`'s own admission). The acceptance criterion "verify item.price folds modifiers" passes
  trivially (it does fold them — but stale), masking the real divergence.
- **Violated invariant:** FE soft-gate and server 422 must compare the same operand. For modified lines
  the operands provably differ whenever a modifier price drifts. (Bounded today: ALL venues, modifiers
  rare; but the resolution presents this as closed, and it is not.)

### NEW-H3 · [B-SEC] Item-3-(b) constraint #3 (`.test`-TLD rejection) is NOT present in the code and the proposal mis-states the namespaced-hash math as making it unnecessary "defence-in-depth" — but the human made `.test` rejection a hard SHIP-BLOCKER. As written, the synthetic hash protects against collision; the `.test` reject (a separate, mandated constraint) is simply absent and must be BUILT, not assumed.

- **Claim under attack (ethical-decisions constraint #3, "ship-blocker"; proposal §3.4 pt.2; resolution
  M4):** "registration rejects `.test` TLDs (`auth.ts:34`) as defence-in-depth."
- **Where it stands (verified):** `courier/auth.ts:34` is `z.string().email().transform(e =>
  e.toLowerCase().trim())` — **no TLD restriction**. Zod's `.email()` accepts `vis-courier@dowiz.test`.
  The reject does **not exist today**; it is a to-build constraint. This is correctly a *design* item, but
  two regressions ride on it:
  1. **The namespaced sentinel hash makes `.test` rejection logically redundant for the SEED's safety**
     (the seed's `email_hash = sha256('synthetic:visual-net-courier:v1')` cannot be produced by ANY email
     input, so no registration — `.test` or otherwise — can collide with the seed row). Good: M4's core is
     closed by construction. **BUT** the human elevated `.test` rejection to an independent ship-blocker,
     and the proposal frames it as "defence-in-depth" — a softer status than "ship-blocker". If the
     implementer reads the proposal's framing (defence-in-depth, redundant given the hash) they may **skip
     it**, shipping partial-(b), which the human explicitly declared NO-GO. The doc tension (proposal:
     "defence-in-depth" vs human: "ship-blocker") will produce a missing constraint.
  2. **Independent of the seed, `.test` acceptance is a real registration-hygiene hole** that the
     constraint was meant to close: a courier can register `x@dowiz.test`, `x@anything.test`,
     `x@localhost.test` — non-routable addresses that pass email verification flows (if any) vacuously.
     That is the substance the human's constraint targets; the namespaced hash does not address it.
- **Violated invariant:** the human's acceptance was conditional on ALL FOUR constraints present;
  constraint #3 is absent in code and downgraded to "defence-in-depth" in prose → partial-(b) risk
  (= the `dev-login-backdoor` shape the human ruled NO-GO).

---

## MEDIUM (new / residual)

### NEW-M1 · [B-CONSIST] Item-3-(b) synthetic-only mint: the design replaces arbitrary `body.courierId` with "the seed returns the id… and the harness echoes it back for a server-side equality check, OR mock-auth re-derives it from the same namespaced sentinel hash" — the FIRST variant re-opens the hole the constraint closes.

- **Verified current state:** `mock-auth.ts:14` mints `crypto.randomUUID()` for `role:'courier'` (not yet
  the (b) design — confirms (b) is to-build). The (b) design (§3.4 / resolution L1) offers two mint
  shapes:
  - **(re-derive):** mock-auth recomputes `SYNTHETIC_COURIER_ID` by hashing the sentinel and selecting the
    courier row → un-abusable, the id is never caller-supplied. **Safe.**
  - **(echo-back):** "the seed returns the id to the harness and the harness echoes it back for a
    server-side equality check." This re-introduces a caller-supplied `courierId` that mock-auth must
    *validate* against the synthetic constant. If the equality check is `body.courierId === SYNTHETIC_ID`
    it is safe; but it is the **same shape** as the removed arbitrary-`body.courierId` (constraint #1's
    target) with a guard bolted on. A guard that compares against a value the seed *returns to the client*
    is one refactor/typo away from `if (body.courierId) sub = body.courierId` — exactly the
    `dev-login-backdoor` primitive. The constraint's intent ("mint ONLY the synthetic id, NEVER arbitrary
    caller input") is satisfied only by the **re-derive** variant; offering echo-back as an equal option
    weakens constraint #1.
- **Violated invariant (conditional):** constraint #1 ("never arbitrary caller input"). Echo-back is
  caller input + a guard; re-derive is no caller input. The design must pin re-derive, not offer both.
- **Cross-tenant note:** even the synthetic token, once minted, carries `activeLocationId` =
  `openId` (the visual venue). The seeded `courier_assignment` (`:543`) and `courier_locations` link
  (`:533`) scope it to that one venue, so it cannot act on *other* tenants' data — **this sub-claim holds**
  (the synthetic courier is membership-scoped to the seeded venue only).

### NEW-M2 · [B-DATA / B-OPS] The persistent synthetic `active` "Visual Net Courier" living in the shared staging DB re-introduces Item-2's dishonesty it was meant to fix: it is a `status='active'` account that the Item-2 "N active" badge will COUNT, and that the owner walkthrough will see.

- **Verified:** Item 2 (resolution H4) makes the badge **"N active" = count(status === 'active')**
  (`CouriersPage.tsx:222` today counts `status==='online'`; post-fix counts `'active'`). The Item-3-(b)
  seed inserts a courier with `status='active'` (`mock-auth.ts` design `:519`,
  `ON CONFLICT … DO UPDATE SET status='active'`) into the **shared staging DB** (R6 accepted: "persists
  between runs"). On the staging venue used for owner walkthroughs, the "Visual Net Courier" will:
  (a) increment the honest "N active" count by 1 (an enabled account that is NOT a real courier), and
  (b) appear in the courier list with a green-adjacent "Active" label.
- **Break scenario:** the demo/owner-walkthrough on staging (the same flow Item-2 honesty exists to
  protect) now shows "N active" inflated by the synthetic fixture, and a courier row for a non-person.
  Counsel A4's mitigation is *only* a display-name marker ("Visual Net Courier") + a doc note — it does
  **not** exclude the row from the count or the list. So the very "N active" honesty metric Item 2 ships
  is polluted by Item 3's persistent fixture. The two items, shipped together, partially cancel.
- **Violated invariant:** display-truth ("N active" should count real enabled accounts). The synthetic
  courier is an enabled account by construction but not a real one; nothing filters it. R6's "acceptable —
  documented marker" does not address the *count*, only the *name*.
- **Severity MED not HIGH:** staging-only (dev-gated, never prod per L1/§3.8 — re-verified the gate holds),
  and a walkthrough operator who reads the marker can mentally subtract it. But it is a real regression of
  the Item-2 honesty goal on the exact surface Item-2 fixes.

---

## LOW (new / residual)

### NEW-L1 · [B-CONSIST] Item-2 "N active" over-count vs the OLD `onlineStatus` semantics — a suspended-then-reactivated account with no shift now counts as "active", which is correct per the resolution, but the shared `onlineStatus` contract field still exists (`shared-types/.../couriers.ts:10`) returning `null`, and `mockData.ts` populates it with real presence values — two divergent truth sources remain on the same card type.

- **Verified:** post-fix the FE ignores `onlineStatus` (Option A reads `status` only). The list endpoint
  returns `onlineStatus: null` (`couriers.ts:53`). But `mockData.ts:67-69` still ships `onlineStatus:
  'busy'|'online'|'offline'` for the storybook/mock path. The "N active" count (`status==='active'`) and
  the dormant `onlineStatus` field can disagree on the same row; if a future Option-B populates
  `onlineStatus` without removing the "N active" account-count, two presence-ish numbers coexist (round-1
  H4's deferred-reconciliation note, now confirmed still unreconciled). No active break (field is ignored),
  hence LOW. No consumer reads the OLD `'online'` semantics post-fix — `CouriersPage.tsx:222`'s
  `status==='online'` filter is dead after the map change (couriers never have `status==='online'`; that
  was always the FE-mapped presence value, now removed). The over-count question ("suspended→reactivated,
  no shift, counts active") is **acceptable per the resolution** (R8: "N active = enabled accounts, not
  on-shift") — confirmed consistent, not a new hole.

### NEW-L2 · [B-DATA] Item-3-(b) idempotent seed DELETE scope is correctly synthetic-scoped — no leak — but the DELETE + INSERT shift sequence is not concurrency-safe under two simultaneous seed runs (two visual-capture jobs).

- **Verified:** `DELETE FROM courier_shifts WHERE courier_id = $synthetic AND location_id = $openId`
  (proposal §3.4 `:538`) is scoped to the synthetic courier + the visual venue — it **cannot** delete a
  real courier's shift (the synthetic `courier_id` is unique to the sentinel-hashed row; M3 closed). **The
  DELETE-scope-leak sub-question is closed: no real shift/assignment is reachable.** Residual: two
  concurrent seed runs (parallel CI jobs) interleave DELETE/INSERT on `courier_shifts` (no unique key) →
  could leave 0 or 2 shifts transiently; the assignment `ON CONFLICT (order_id) DO UPDATE` re-points to
  one. Dev-only, single-shot per capture run in practice → LOW. The proposal's idempotency claim holds for
  serial re-runs (the stated use), not concurrent ones.

---

## Prior findings that ARE genuinely closed by the revision (regression-checked)

- **C2 (applyTax mirror) — CLOSED.** Verified `apps/api/src/lib/money.ts` has **zero imports** (no `pg`,
  no node `crypto`) — it is pure and isomorphic. `packages/domain` already exists
  (`@deliveryos/domain`). Extracting `applyTax`/`computeLineTotal`/`assertNonNegative` verbatim into it
  and importing in both api + web pulls **no server-only deps into the web bundle** (the C2-(b) bundle
  question is answered: safe). The "share, don't mirror" resolution is sound and the second-copy drift is
  eliminated by construction. (NEW-C1 is a *different* gap — the create-path fee math around `applyTax` is
  not extracted, only `applyTax` itself.)
- **H3 (dead `_minorUnit`) — CLOSED as a contract decision.** Verified `money.ts:23` `_minorUnit` is
  unused; the shared function IS the contract; ALL collapses tax to 0. Pinning it as "operate in stored
  minor-unit integers, no coarser rounding" is internally consistent. No regression.
- **M2 (`delivery_tiers` RLS false→claim-exact) — CLOSED by construction.** The precomputed public
  `has_distance_tiers` boolean + fail-safe (unknown → degrade) removes the RLS-subject `EXISTS`. The unsafe
  direction is eliminated. (Caveat: the boolean must be *maintained* on the owner config write — a
  to-build write-path correctness item, but the read-side design is safe.)
- **M3 (seed shift idempotency) — CLOSED.** Code now matches prose (DELETE-before-insert, synthetic-scoped);
  shift `status='available'` matches the proven path. Re-verified the DELETE cannot reach a real row
  (NEW-L2).
- **M4 (email_hash resurrect real courier) — CORE CLOSED.** The namespaced non-email sentinel hash
  `sha256('synthetic:visual-net-courier:v1')` cannot be produced by any `z.string().email()` input → the
  `ON CONFLICT (email_hash) DO UPDATE` provably reaches only the synthetic row. A real suspended courier
  cannot be resurrected by the seed. (The separate `.test`-reject ship-blocker is still absent — NEW-H3 —
  but the *resurrection* vector M4 named is closed by the hash alone.)
- **L1 (prod gate) — STILL CLOSED.** Re-verified: `mock-auth` under `/dev/`, gated by
  `isDevRequestAuthorized` (flag + secret, fails closed 404). No new prod surface. The (b) synthetic-only
  mint further reduces the staging residual (subject to NEW-M1: pin re-derive, not echo-back).
- **L3 (assignment re-point) — CLOSED.** Seed owns its synthetic `order_id`; `ON CONFLICT (order_id)`
  target is synthetic.
- **L2 (kill-switch default) — moot post-C1-intent** (the flag toggles only the cosmetic CTA hint) — BUT
  this rests on NEW-C1 being solved; while NEW-C1 stands, the flag's "neither state can mis-collect" claim
  is unverifiable because the safe-collect mechanism (server-total review) does not exist.

---

## Severity summary — RE-ATTACK round 2

| ID | Sev | Vector | One-line |
|----|-----|--------|----------|
| NEW-C1 | CRITICAL | B-CONSIST/MONEY | Cash-parity review step has no backing endpoint — `total` only exists inside the create txn (orders.ts:565→596); customer cannot review server total before commit; AC-CASH-PARITY/ETHICAL-STOP-1 not satisfiable as written |
| NEW-H1 | HIGH | B-CONSIST/MONEY | Even with a quote endpoint, reviewed total and committed total are unsynchronised reads (30s `/info` cache + separate MVCC); no transactional binding of shown==charged |
| NEW-H2 | HIGH | B-CONSIST | FE subtotal can't be modifier-parity-exact for modified lines (cartReconcile.ts:30 admits it); H1 "verify item.price folds modifiers" passes trivially but is stale → min-gate/threshold divergence |
| NEW-H3 | HIGH | B-SEC | `.test`-TLD reject (human ship-blocker #3) absent in code AND downgraded to "defence-in-depth" in prose → partial-(b) risk (NO-GO shape) |
| NEW-M1 | MED | B-CONSIST | Synthetic-only mint offers an "echo-back" variant = caller input + guard (the backdoor shape); only the re-derive variant satisfies constraint #1 |
| NEW-M2 | MED | B-DATA/OPS | Persistent synthetic `active` courier pollutes Item-2's "N active" count + owner walkthrough — re-introduces the Item-2 dishonesty; marker fixes the name, not the count |
| NEW-L1 | LOW | B-CONSIST | Dormant `onlineStatus` contract field + mockData presence values coexist with "N active"; over-count is acceptable per R8 (not a new hole) |
| NEW-L2 | LOW | B-DATA | Seed DELETE is synthetic-scoped (no real-row leak — closed); residual is concurrent-run non-idempotency only |

**Closed (regression-verified):** C2 (bundle-safe, domain pure), H3 (contract pinned), M2 (precomputed
boolean + fail-safe), M3 (DELETE matches prose), M4-core (namespaced hash blocks resurrection), L1 (prod
gate holds), L3 (synthetic order ownership).

**The one load-bearing regression:** NEW-C1. The cash-parity guarantee the human approved as "no ship
without it" presumes a server price-preflight that does not exist and that the proposal's own non-goals
("no new endpoint", "not changing server fee math") forbid building. The fee math is inlined in the
create transaction (`orders.ts:518-565`), not extracted as a callable quote. Either the math is pulled out
into a pure quote function + a real preflight endpoint (a designed sub-feature, currently absent), or the
"review the server total before cash" flow degenerates to "create the order, then learn the total" — which
is round-1 C1 with an orphan-order tail. AC-CASH-PARITY is not satisfiable by the design as written.

---

# RE-ATTACK round 3 (exit check) — MONEY change, narrow scope

Breaker: System Breaker DeliveryOS. FINAL exit check on the round-2 dispositions
(`resolution.md` §"RESOLVE round 2", revised `proposal.md` §1.4, ADR-0005/0006). Method: read-only
re-verification of the three round-2 mechanisms named for this round against live source
(`orders.ts`, `packages/domain/src/`, `apps/api/src/lib/money.ts`, `courier/auth.ts`, `auth/local.ts`,
`public/access-requests.ts`, `CheckoutPage.tsx`). Scope: did the round-2 fixes introduce a NEW
CRITICAL/HIGH, or is this at hard-exit?

## Verdict: ONE NEW HIGH (the "shared `computeOrderTotal`" refactor the whole NEW-C1 resolution rests on does not exist and is a money-path change the plan under-owns). The other two mechanisms are at hard-exit.

---

### NEW-3-H1 · [B-CONSIST / MONEY · scope/risk the plan must own] The round-2 NEW-C1 resolution ("computability — the client runs the SAME shared `computeOrderTotal` the server runs, so the client number IS the server total by construction") presumes a shared total function that DOES NOT EXIST and a server create-path that calls it — neither is true today. Making it true is a real, non-trivial refactor of the money path, not a free consequence of the C2 share.

- **Claim under attack (resolution NEW-C1 / proposal §1.4 "Computable venues" / §1.6 AC-CASH-PARITY):**
  "the client computes `reviewTotal` via the **shared `@deliveryos/domain` `computeOrderTotal`** — the
  *same function* `orders.ts` runs, over the *same* public inputs. By construction `reviewTotal == server
  total` … This is **NOT** a second copy — it is the one imported module."
- **Where it breaks (verified, live source):**
  1. **`computeOrderTotal` does not exist anywhere.** `grep -rn "computeOrderTotal" apps packages` →
     **zero hits**. The function the resolution names as the load-bearing shared artifact is unwritten.
  2. **`@deliveryos/domain` does not export any money math.** `packages/domain/src/index.ts` exports only
     `./order-machine.js` + `./errors.js`; the dir contains `errors.ts`, `index.ts`, `order-machine.ts` —
     **no money module.** The C2 "share `applyTax`/`computeLineTotal`/`assertNonNegative`" extraction is
     *also* still un-done (those live in `apps/api/src/lib/money.ts:23/46/54`, an api-internal file).
  3. **The server computes the total INLINE, not via any callable fn.** `orders.ts:560-565`:
     `const taxTotal = applyTax(...); const discountTotal = 0; const total = subtotal + deliveryFee +
     taxTotal - discountTotal;` — the fee selection (min-order 422 `:518`, free-threshold `:530`, live
     `delivery_tiers` query `:533`, flat `:553`, `applyTax` `:562`) is **interleaved with `client.query`
     ROLLBACK/INSERT side-effects inside the create transaction**. There is no pure `computeOrderTotal`
     the create path calls; "the same function the server runs" has no referent.
- **Why this is the answer to #1(a) the round asked for:** the server still computes inline
  (`orders.ts:560-565`), and the money fns are NOT in `@deliveryos/domain`. So "shared" is NOT two-copies-
  becoming-one-by-import; it requires **(a)** extracting `applyTax`/`computeLineTotal`/`assertNonNegative`
  into the domain pkg (the C2 work, unstarted), **AND (b)** authoring a new pure `computeOrderTotal` that
  re-implements the fee-selection ladder (free-threshold + flat; tiers excluded by computability split)
  side-effect-free, **AND (c)** refactoring `orders.ts:518-565` to call that same fn so the two are
  provably one. Step (b)+(c) is a **refactor of the live money create-path** (🔴 money red-line), not a
  free corollary of importing a module. The parity guardrail (§1.6) only proves equality *if (c) is done*;
  if the implementer ships the client `computeOrderTotal` but leaves the server inline (the literal
  minimum — touching the money txn is scary, the client side is the visible task), they are **two copies
  again**, and NEW-C1's "by construction" guarantee silently becomes "by a hand-maintained mirror" — the
  exact drift C2 swore to eliminate, now hidden behind a function name that implies sharing.
- **Violated invariant:** `clientTotal === serverTotal` "by construction" (NEW-C1 / AC-CASH-PARITY). It
  holds ONLY if the server create-path is refactored to call the same `computeOrderTotal`. That refactor
  touches the money path, is currently unscoped in the proposal (§1.1 non-goal explicitly says "Not
  changing the server fee math"), and its risk/cost is not owned by the plan. The resolution presents
  "computability" as if the shared fn already binds both sides; verified, it binds neither yet.
- **Severity HIGH not CRITICAL:** it is not a guaranteed mis-charge — IF the implementer does the full
  extract + server refactor, the design is sound and NEW-C1 genuinely closes. It is HIGH because the plan
  treats a money-path refactor as a free consequence ("not a second copy" — but it IS two copies until the
  server is refactored), and the literal-minimum implementation path (client-only `computeOrderTotal`,
  server left inline) re-creates the drift undetected by any gate that tests matched inputs. The plan must
  explicitly OWN: "server `orders.ts:518-565` is refactored to call the shared `computeOrderTotal`; the
  parity guardrail asserts the *refactored server path* and the client produce byte-identical totals." As
  written, that ownership is absent — §1.1 non-goals even forbid it.

---

## Exit-check on the three named mechanisms

**#1 — Computability parity (the load-bearing money claim): NOT at hard-exit → NEW-3-H1 (HIGH).**
- (a) **Server computes INLINE** (`orders.ts:560-565`); `computeOrderTotal` does not exist; money fns are
  in `apps/api/src/lib/money.ts`, NOT `@deliveryos/domain`. "Shared" requires extracting the money fns AND
  authoring `computeOrderTotal` AND refactoring the server create-path to call it — a real money-path
  change, not free, and the proposal's §1.1 non-goals currently disclaim it. **This is the HIGH above.**
- (b) **No silent server-only input.** Verified `discountTotal = 0` hardcoded (`orders.ts:564`); no promo/
  coupon/discount is applied server-side today (`grep discount\|promo\|coupon orders.ts` → only the `= 0`
  literal). All other inputs (subtotal-with-modifiers, fee_flat, free-threshold, tax_rate, tax_mode) are
  on `/info` + the cart. So a parity test over matched inputs is NOT masking a hidden divergent input
  **today**. (Forward-flag only: the day `discountTotal` becomes nonzero server-side, it must land on
  `/info` AND in `computeOrderTotal` or NEW-C1's parity silently breaks — already noted as R3, not new.)

**#2 — Tiered option (ii) cash entry: at hard-exit (no NEW finding), with one verification the impl must
honour.** Verified the existing cash UI (`CheckoutPage.tsx:835/839/870/873`) keys `min`, the red-border
(`cashAmount > 0 && cashAmount < total`), the change line (`cashAmount - total`), and the
"Amount must be at least {total}" message ALL to a single `total` var. The round-2 design (proposal §1.4:
tiered → `cashFloor = undefined`, CTA `{subtotal}+`, "cash not pre-quoted") is COHERENT **on paper**: for a
tiered venue the customer is not asked to commit an exact figure, and the courier "collect: X" is the
server total. **No new CRITICAL/HIGH** — but it is a pure *design* statement; in code the cash block is
unconditionally keyed to `total` with no tiered-suppression branch. The real path where a tiered cash
customer commits a number ≠ door figure exists ONLY IF the implementer wires `total = reviewTotal`
unconditionally and forgets the `feeKnown === false ⇒ suppress cash entry` branch. That is an
implementation-discipline item the AC + E2E (§1.6) must assert (cash entry suppressed when `!feeKnown`),
not a design hole. Recorded as a verification obligation, not a finding.

**#3 — `.test` reject at 3 endpoints: at hard-exit (no NEW finding); sentinel hash ALONE closes M4.**
- Verified the reject is **absent at all three** today: `courier/auth.ts:34` (`z.string().email().transform`),
  `auth/local.ts:41` (`z.string().email().max(200)`), `public/access-requests.ts:56`
  (`z.string().email().max(320)`) — none restrict TLD; `rejectReservedTld` does not exist
  (`grep` → zero hits). This matches the round-2 resolution (NEW-H3): it is a **to-build** shared Zod
  refinement, correctly framed as a ship-blocker. No regression — it is net-new hardening.
- **No legitimate existing flow breaks:** no `@*.test` user is referenced in app source (the test owner is
  `test@dowiz.com`, a `.com`, unaffected; `*.test.ts` matches are spec files, irrelevant). Adding the
  refusal cannot lock out a real account that exists today.
- **Consistency flag (minor, NOT a new sev):** the resolution names `auth/local.ts:41` as "owner
  register/login", but that file holds only **`/auth/local/login`** (one email parse, `:41`); there is no
  register handler in `local.ts`. If owner *registration* parses email elsewhere, the 3-endpoint list is
  incomplete and that endpoint would be the one place `.test` slips through. This does NOT change M4
  safety (below) — recorded so the impl confirms it has covered every registration email parse, not just
  the three listed.
- **Sentinel hash ALONE fully closes M4:** confirmed (round-2 regression-verified) — the seed
  `email_hash = sha256('synthetic:visual-net-courier:v1')` cannot be produced by any `z.string().email()`
  input, so `ON CONFLICT (email_hash) DO UPDATE` provably reaches only the synthetic row regardless of
  whether `.test` is rejected. The `.test` reject closes a **different** threat (registration-namespace
  hygiene), so it is belt-and-suspenders **for M4 specifically** while remaining a legitimate independent
  ship-blocker for namespace hygiene — the round-2 framing is correct. **M4 is at hard-exit.**

---

## Round-3 result

**NEW CRITICAL/HIGH introduced/under-closed by the round-2 fixes:**

| ID | Sev | Vector | One-line |
|----|-----|--------|----------|
| NEW-3-H1 | HIGH | B-CONSIST/MONEY | NEW-C1's "shared `computeOrderTotal`, client==server by construction" presumes a fn that does not exist (`grep`→0) AND a server create-path that calls it — server still computes INLINE (`orders.ts:560-565`); making it true is an unowned refactor of the money path (§1.1 non-goals forbid it). Literal-minimum impl (client-only fn, server inline) = two copies again, drift undetected by a matched-input parity test. The plan must OWN the server refactor. |

**At hard-exit (no new sev):** #1(b) no hidden server-only input (`discountTotal=0`); #2 tiered cash
design is coherent (cash-suppression is an impl-discipline AC, not a hole); #3 `.test` reject breaks no
existing flow and the sentinel hash alone closes M4 (reject is correctly an independent hygiene
ship-blocker, belt-and-suspenders for M4).

**Bottom line:** not at hard-exit. ONE NEW HIGH — NEW-3-H1: the computability claim is sound *as a target*
but is not yet true in code, and the step that makes it true (extract money fns into `@deliveryos/domain`
+ author `computeOrderTotal` + refactor `orders.ts:518-565` to call it) is a money-path change the plan
currently disclaims in its non-goals. Either the proposal owns that server refactor explicitly (and the
§1.6 parity guardrail asserts the *refactored* server path against the client), or NEW-C1's "by
construction" guarantee degrades to an un-gated hand-maintained mirror. Everything else checked this round
is at hard-exit.
