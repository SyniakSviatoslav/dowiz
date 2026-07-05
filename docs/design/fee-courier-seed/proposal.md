# Design Proposal — Delivery-Fee Source-of-Truth · Courier-Status Honesty · Encrypted Dev-Seed

Branch: `fix/design-system-consistency` · Author: Architect (Triadic Council, design-time) · Date: 2026-06-25

Three independent serious changes, bundled because they share a verification surface (the 390px
visual capture + checkout/courier flows). Items #1 and #2 are FE-only display-truth fixes that must
NOT silently change a server contract; item #3 is a dev-only test-infra extension. No production
migration is introduced by any of the three.

---

## ITEM 1 — [MONEY/CONTRACT] Delivery fee at checkout

### 1.1 Problem + non-goals

`apps/web/src/pages/client/CheckoutPage.tsx:342` hardcodes the delivery fee:

```ts
const deliveryFee = deliveryType === 'delivery' ? 200 : 0;
const total = subtotal + deliveryFee;     // line 343
```

This `total` drives the `Porosit • {total}` CTA the customer reads. The order-create payload does
**not** send a fee — the server is authoritative (`CreateOrderInput` carries no fee field, confirmed
`apps/api/src/routes/orders.ts:90`). The displayed number is therefore a pure FE guess and is wrong
in every case where the venue is not configured to a flat 200 with zero tax and no free-delivery
threshold.

The **authoritative** server math (`apps/api/src/routes/orders.ts:519-566`) is:

1. `min_order_value` enforced: `subtotal < min_order_value` → **422 MIN_ORDER_NOT_MET** (orders.ts:519-526).
2. Free-over-threshold waiver: `free_delivery_threshold != null && subtotal >= free_delivery_threshold` → `deliveryFee = 0` (orders.ts:530-531).
3. Otherwise, distance tiers (`delivery_tiers`, ordered by `max_distance_km`) take precedence when present and the location has a pin; out-of-range → **422 NOT_DELIVERABLE** (orders.ts:533-552).
4. Else flat fee `delivery_fee_flat` (orders.ts:553-554); if neither tiers nor flat → **422 DELIVERY_NOT_CONFIGURED** (orders.ts:555-557).
5. Tax: `taxTotal = applyTax(subtotal, tax_rate, price_includes_tax, currency_minor_unit)` (orders.ts:563).
6. `total = subtotal + deliveryFee + taxTotal - discountTotal` (`discountTotal = 0` today, orders.ts:565).
7. `assertNonNegative(total)` (orders.ts:566). All values **integer minor units** (ALL has `currency_minor_unit = 0`, migration `1780338982014`).

Field names confirmed (migration `1780338982014_location_commerce.ts`): `currency_code`,
`currency_minor_unit`, `tax_rate`, `price_includes_tax`, `min_order_value`,
`free_delivery_threshold`, `delivery_fee_flat`, plus the `delivery_tiers` table.

**The gap:** the public `/public/locations/:slug/info` endpoint (`apps/api/src/routes/public/menu.ts:213-282`)
returns `currency_code` + `currency_minor_unit` but **NOT** `min_order_value`, `free_delivery_threshold`,
`delivery_fee_flat`, `tax_rate`, or `price_includes_tax`. The storefront has no way to read the fee
config today, which is why a constant was hardcoded.

**Non-goals.** Not adding promo/discount surfacing (server `discountTotal = 0` today; out of scope).
Not replicating distance-tier geometry in the browser (the pin-distance tier walk needs the venue
lat/lng + `delivery_tiers` rows; we deliberately do NOT ship that to the client — see Decision).
Not changing the order-create contract. Not introducing a migration.

**Non-goal correction (NEW-3-H1 — owned honestly).** This PR DOES add a `@deliveryos/domain` money module
(`packages/domain/src/money.ts`: `applyTax`/`computeLineTotal`/`assertNonNegative` extracted **verbatim**
from `apps/api/src/lib/money.ts`, plus a new pure client-side `estimateOrderTotal`), and DOES re-point
`apps/api`'s money-fn imports to that domain pkg — a **pure, characterization-pinned move** (no output
changes). It does **NOT** change the server **fee-selection / charge arithmetic**: the authoritative
create-path `orders.ts:518-565` (the fee ladder, `applyTax` call, `total = …`, and the cash-422 gate
`:568`) stays **inline and byte-for-byte authoritative**. We chose **Approach M (mirror-with-hard-gate)**
over Approach R (refactor the server create-path to call the shared fn) precisely to hold the 🔴 money
red-line: zero change to what is charged. The client's `estimateOrderTotal` is a **mirror** of the server
fee arithmetic (computable case only); "displayed == charged" is guaranteed not "by construction" (the
server does not call the client fn) but by the **permanent parity guardrail** (§1.6 — real server total ==
client mirror, red→green, ship-blocker) plus the runtime cash-422 backstop. Approach R (extract the
create-path arithmetic so both sides literally share one fn) is the recorded **deferred end-state**, taken
as its own behaviour-preserving change **when the server money math grows** (e.g. nonzero `discountTotal` /
promo) — never bundled into this FE-display PR (YAGNI: today the only divergent-risk input, `discountTotal`,
is hardcoded `0`). See resolution.md §"RESOLVE round 3" + ADR-0005.

### 1.2 Back-of-envelope

Scale: a single venue serves ~1–30 orders/min at peak; checkout is a per-session human action, not a
fan-out. `/info` is already cached (30s TTL, `getInfoRow`, menu.ts:202-211) and would carry the extra
fields at zero added query cost (they are columns on `locations`, already in the row scope — only the
SELECT list grows). **No new connection budget**: the fields ride the existing `/info` read. No new
endpoint, no new DB round-trip.

**Worked money example — where client vs server diverge today.** Venue: `min_order_value = 800`,
`free_delivery_threshold = 2000`, `delivery_fee_flat = 250`, `tax_rate = 0` (ALL, tax-inclusive,
minor_unit 0). Cart subtotal = 1500 (delivery).

| Case | Client shows today (hardcoded) | Server charges (authoritative) | Divergence |
|---|---|---|---|
| subtotal 1500 | total = 1500 + 200 = **1700** | 1500 + 250 = **1750** | **−50** under-quoted |
| subtotal 2000 (== threshold) | total = 2000 + 200 = **2200** | fee waived → **2000** | **+200** over-quoted |
| subtotal 1999 (just under) | 1999 + 200 = **2199** | 1999 + 250 = **2249** | **−50** under-quoted |
| subtotal 500 (< min 800) | 500 + 200 = **700**, lets you press Order | **422 MIN_ORDER_NOT_MET** | order silently rejected after submit |

The **free-over-2000 boundary** is the sharpest: the customer who adds one more item to cross 2000
should see the fee drop to 0. The hardcoded client never does, so it over-quotes by the whole fee
exactly when the customer did the thing the venue incentivised. Replicating the boundary correctly:

```
deliveryFee = (subtotal >= free_delivery_threshold) ? 0 : delivery_fee_flat
total       = subtotal + deliveryFee + applyTax(subtotal, ...)   // all integer
```

The one place the client provably **cannot** match the server is distance-tiered venues (the fee
depends on the delivery pin vs venue lat/lng). For those, any client number is an estimate.

### 1.3 Options

**Option A — "Replicated formula" (client mirrors the server to the cent).**
Concept: *server-authoritative config + client-side deterministic replication*. Extend `/info` to
return `min_order_value`, `free_delivery_threshold`, `delivery_fee_flat`, `tax_rate`,
`price_includes_tax`. Client recomputes the exact integer formula incl. free-over-threshold and tax.
- Pros: the CTA number matches the charge for flat-fee venues (the common case); the free-delivery
  boundary becomes a live incentive; min-order can be enforced FE-side *before* submit (no 422 surprise).
- Cons: distance-tier venues still cannot be matched client-side → the replicated number is a lie for
  them unless we special-case. Two formula copies (client + server) must be kept in lockstep forever —
  a money-drift risk every time the server math changes (e.g. when `discountTotal` becomes nonzero).

**Option B — "Estimate + server-confirmed total drives the CTA".**
Concept: *itemized estimate, server is the price authority at confirm*. Client shows an itemized
breakdown using `/info` config where available, BUT labels the delivery line `confirmed at checkout`
and the final `Porosit` CTA shows the **subtotal-grounded** figure with the fee clearly marked as an
estimate; the SERVER-returned `total` (already on the order-create response, orders.ts:760-761) is the
authority and is shown on the confirmation/status screen. The order is never blocked by a client guess.
- Pros: no money-drift risk (server stays the single source of truth); distance-tier venues are honest
  (no false precision); resilient to `/info` failure (fall back to "fee confirmed at checkout").
- Cons: the customer presses Order without a guaranteed final number — slightly worse for trust on the
  flat-fee common case, where Option A could have shown the exact figure.

**Option A′ (chosen hybrid) — replicate where deterministic, estimate where not.**
Concept: *replicate the flat-fee + free-threshold + tax formula client-side (deterministic), and
degrade to an "estimate, confirmed at checkout" label exactly when the venue is distance-tiered or
`/info` is unavailable*. Min-order is enforced FE-side as a soft pre-check (disable Order + inline
message) **and** remains hard-enforced by the server 422.

### 1.4 Decision (hardened post-Breaker C1/C2/H1/H2/H3, Counsel A1, ETHICAL-STOP-1)

**Adopt Option A′ — but the estimate is ONLY a pre-review CTA hint; the SERVER total is collected.**
Rationale (truth-of-engineering): for the dominant flat-fee venue the client CAN match the server to the
cent using shared money math, so we show an exact CTA hint — the honest, higher-trust outcome that makes
the free-over-threshold incentive real. We refuse to fabricate precision we cannot earn: distance-tiered
venues and any `/info` failure degrade to "delivery fee depends on your address — confirmed at checkout".

**The load-bearing correction (C1, ETHICAL-STOP-1): for cash-on-delivery the displayed estimate is NEVER
the money collected.** The replicated `estTotal` is a *hint only*. The cash input, its `min`, the
red-border threshold, the change-due, and the door-collection figure are all keyed to the
**server-authoritative `total`**, never to a loose `estTotal`. The flow is
**review the authoritative total → confirm cash against it → submit** (never estimate → cash → reconcile,
which is structurally impossible for cash).

**NEW-C1 / NEW-3-H1 — there is no `/orders/quote` price endpoint, and we do NOT need one for the common
case. The "authoritative total before commit" is obtained by a GATED CLIENT MIRROR (Approach M), NOT by a
shared fn the server calls.** The honest statement (verified — NEW-3-H1): `computeOrderTotal` does not
exist today; `@deliveryos/domain` has no money module; the server computes the total **inline** at
`orders.ts:560-565` with the fee ladder interleaved with `ROLLBACK`/`INSERT` side-effects. Making the
client number equal the server "by construction" would require **refactoring the authoritative money
create-path** to call a shared fn — a 🔴 money red-line we deliberately do NOT take in this PR. Instead:
- We extract the **pure building blocks** (`applyTax`/`computeLineTotal`/`assertNonNegative`) verbatim into
  `@deliveryos/domain` (they have no impure interleaving — safe), and `apps/api` re-imports them (a pure,
  characterization-pinned move).
- The client uses a separate pure **`estimateOrderTotal`** (in `@deliveryos/domain`) that **mirrors** the
  server fee-selection arithmetic for the **computable case only** (flat-fee + free-threshold + tax +
  integer modifier-inclusive subtotal). The server create-path is **unchanged**; it does NOT call
  `estimateOrderTotal`.
- "Displayed == charged" is guaranteed by the **§1.6 parity guardrail** (computes the REAL server total for
  a matrix of inputs and asserts `estimateOrderTotal == serverTotal`, red→green, permanent ship-blocker) +
  the runtime **cash-422 backstop** — NOT by a shared-by-both-sides import. The function is named
  `estimateOrderTotal` (not `computeOrderTotal`) so no reader infers a non-existent "server calls it too"
  property; the gate is what makes it equal the server.

For any venue whose fee is a pure function of **public inputs** (`delivery_fee_flat` +
`free_delivery_threshold` + `tax_rate` + `price_includes_tax` + integer `subtotal`, all on `/info`), the
client mirror is provably equal to the server total over the gated matrix. We split explicitly:

- **Computable venues** (`has_distance_tiers === false` AND `deliveryFeeFlat != null` — flat-fee +
  threshold + tax; covers the demo and the dominant case): the client computes `reviewTotal` locally via
  the mirror `estimateOrderTotal` (`delivery-fee + computeLineTotal-subtotal + applyTax`). **This
  `reviewTotal` equals the server total over the gated matrix** — the cash door figure is confirmed against
  it. No endpoint. The parity guardrail (§1.6) proves `estimateOrderTotal == server total` for identical
  inputs against the **real** server arithmetic, so "authoritative" is a CI-gated property, not a hope, and
  a hand-maintained mirror cannot drift to prod silently (drift = a build break). **Approach R** (refactor
  `orders.ts:518-565` to call the shared fn so the equality holds "by construction") is the deferred
  end-state for when the server money math grows — see §1.1 + ADR-0005.
- **Distance-tiered venues** (`has_distance_tiers === true` OR unknown — the minority; RLS hides
  `delivery_tiers` from the public role so the client genuinely **cannot** compute the fee): we do NOT show
  a precise cash number we cannot authoritatively back. The CTA degrades to `Porosit • {subtotal}+`; cash
  is **not pre-quoted to an exact figure**. The customer is **not asked to pre-commit an exact cash amount**
  for a tiered venue. Instead the order submits, the server computes the real `total`, and the
  **courier delivery screen shows that server `total` as "collect: X"** — the courier collects the
  server-confirmed amount at the door (door-handover parity holds: the figure the courier collects is the
  server total, and no smaller customer-shown figure exists to undercut it). This is the chosen path
  (NEW-C1 option (ii)); option (i) — a real read-only `/orders/quote` endpoint calling the same shared
  module — is recorded as a deferred, named scope addition for when tiered venues become common, NOT built
  now (YAGNI; the shared-function computability covers every venue the launch trigger needs).

Cash review parity (resolves C1, H2-tip, ETHICAL-STOP-1, Counsel §5/A3): the review step shows two
explicit lines — "Order total (collected): {server_total}" and, when `tip > 0`,
"+ Tip (cash to courier): {tip}" — so the door figure is unambiguous; `total` excludes tip server-side
(`orders.ts:565`, verified) and tip is collected on top. The same `{server_total}` is surfaced to the
courier delivery screen as "collect: X" (door-handover parity).

Min-order is a FE soft-gate backed by the server 422 (told *before* submit). The FE also **explicitly
handles `CASH_AMOUNT_TOO_LOW`** (defence-in-depth for the stale-window race) with a designed re-prompt
that re-shows the updated server total — never the generic "failed to place order".

Concrete client shape (`CheckoutPage.tsx`, replacing line 342-343):

```ts
// from /info (integer minor units); undefined when /info failed or field absent
const { deliveryFeeFlat, freeDeliveryThreshold, minOrderValue,
        taxRate, priceIncludesTax, currencyMinorUnit, hasDistanceTiers } = venueCommerce;

// applyTax + computeLineTotal + assertNonNegative are SHARED building blocks from @deliveryos/domain —
// ONE impl, imported by api + web (C2 — pure fns, safe to extract). estimateOrderTotal is a CLIENT MIRROR
// of the server fee arithmetic (Approach M, NEW-3-H1): the server does NOT call it; the §1.6 parity gate
// proves estimateOrderTotal == the REAL server total (red→green, permanent). The server create-path
// (orders.ts:518-565) is UNTOUCHED — zero change to the charged amount.
//
// H1/H2: `subtotal` MUST be the shared computeLineTotal sum over the CURRENT menu modifier price_deltas
// (the public menu carries modifier_groups[].price_delta — MenuPage.tsx:458-466), NOT the stale stored
// cart-line price. Cart `item.price` already folds modifiers at add-time (MenuPage.tsx:510), but for a
// modifier-bearing line whose price_delta drifted, reconcileCart deliberately skips repricing
// (cartReconcile.ts:30). So checkout recomputes subtotal modifier-inclusive from the freshly-loaded menu:
const subtotal = items.reduce((s, it) =>
  s + computeLineTotal(currentBasePrice(it), currentModifierDeltas(it), it.quantity), 0);

// Fail-SAFE: unknown tiers => treat as tiered => degrade (M2). Never claim-exact on ambiguity.
const feeKnown = deliveryType !== 'delivery'
  ? true                                  // pickup: fee is provably 0
  : (hasDistanceTiers === false && deliveryFeeFlat != null);

const estDeliveryFee = deliveryType !== 'delivery' ? 0
  : (freeDeliveryThreshold != null && subtotal >= freeDeliveryThreshold) ? 0
  : (deliveryFeeFlat ?? 0);

// reviewTotal := estimateOrderTotal(subtotal, estDeliveryFee, taxRate, priceIncludesTax, currencyMinorUnit)
const reviewTotal = subtotal + estDeliveryFee + applyTax(subtotal, taxRate, priceIncludesTax, currencyMinorUnit);
// COMPUTABLE venue: reviewTotal == server total over the gated parity matrix (client MIRROR, §1.6 gate —
// NOT "by construction"; the server does not call this fn). It is the authoritative door figure — cash
// min/red-border/change-due are keyed to reviewTotal, never a loose hint; drift is a CI build break.
// TIERED/degrade: feeKnown === false → no exact cash figure; CTA shows `${subtotal}+`, cash not pre-quoted;
// the server total surfaces as the courier "collect: X" door figure (NEW-C1 option ii).
const cashFloor = feeKnown ? reviewTotal : undefined;   // undefined ⇒ no exact pre-commit cash on tiered

// H1: min-gate matches server (pickup INCLUDED) and uses the SAME modifier-inclusive subtotal.
const belowMin = minOrderValue != null && subtotal < minOrderValue;
```

**NEW-H1 — reviewed total vs committed total are two reads (30s `/info` cache + separate MVCC).** Even
for a computable venue, the `/info` config the client reads (≤30s cached, ≤1h stale-on-error) and the
`locations` row the create txn reads live are two snapshots. If the owner changes `delivery_fee_flat` /
`free_delivery_threshold` between review and submit, `reviewTotal != server total`. **Disposition:
accept-risk, with the AC-CASH-422 re-prompt as the binding backstop** (not bound transactionally). The
server `cashPayWith < total` 422 (`orders.ts:568`) catches the under-quote direction; the FE re-prompt
(AC-CASH-422) re-shows the *new* server total and re-blocks until cash ≥ it — so the customer can never
submit a cash figure below the live charge. The over-quote direction (config dropped) is self-correcting:
the customer's cash already covers the lower total. A `checkout_fee_divergence` counter surfaces the window
in <1 min. We deliberately do NOT add a `request_hash`-bound quote-lock: it would require the
unbuilt `/orders/quote` endpoint + a hold/expire mechanism (a whole sub-feature) to close a ≤30s window
the 422 backstop already makes safe-by-direction. Owner: Product.

**C2 / H3 — share, don't mirror.** Per Counsel A1, extract `applyTax`, `computeLineTotal`,
`assertNonNegative` **verbatim** into **`@deliveryos/domain`** (isomorphic, no Node-only deps) and import
the *same* implementation in `apps/api` and `apps/web`. One source of truth eliminates the second copy the
parity test would have to guard. The pinned contract (H3): `applyTax` operates entirely in **stored
minor-unit integers** (the two BigInt branches — tax-excluded forward half-up and tax-included back-out,
`rateMicro` quantised to 6-dp micro-units); the `minorUnit` argument is informational and MUST NOT trigger
coarser rounding (the server's `_minorUnit` is dead today — `lib/money.ts:23`). For ALL (`tax_rate = 0`,
minor_unit 0) tax collapses to 0. If an isomorphic share proves infeasible at implementation, the
documented fallback is the §1.6 parity guardrail treated as a **permanent, never-deletable** gate whose
matrix explicitly includes the tax-INCLUDED back-out branch and non-zero `minor_unit`.

**H1 / NEW-H2 — subtotal parity (the public menu DOES carry modifier prices).** Verified: the public menu
serves `modifier_groups[].modifiers[].price_delta` (`MenuPage.tsx:19, 458-466`), and the cart stores
`item.price = base + Σ modifier deltas` at add-time (`MenuPage.tsx:510`). So the Breaker's premise that the
FE "cannot hold per-modifier prices" is false — it holds them. The one real residual (NEW-H2): for a
**modifier-bearing** line whose `price_delta` drifted after add-time, `reconcileCart` deliberately skips
repricing (`cartReconcile.ts:30`), so the *stored* line price is stale. **Resolution:** at checkout the FE
recomputes `subtotal` modifier-inclusive via the **shared `computeLineTotal` over the CURRENT freshly-loaded
menu** `price_delta`s (not the stored snapshot) — the same function and the same current inputs the server
reads (`orders.ts:485,506`), so the operands match. The FE min-gate compares the same operand as the server
(`subtotal < min_order_value`, pickup included, H1). **Bounded residual, accepted:** if a modifier
`price_delta` changes in the ≤30s window between the menu read and the order create, the FE and server
subtotals can still differ by that delta — caught by the server 422 + the AC-CASH-422 re-prompt (same
backstop as NEW-H1), never silently mis-collected. Owner: Product.

### 1.5 Data / migrations

**No DB migration.** All fields already exist on `locations` (`1780338982014`). The only change is the
`/info` SELECT list + response object in `apps/api/src/routes/public/menu.ts`:
- add `l.min_order_value, l.free_delivery_threshold, l.delivery_fee_flat, l.tax_rate, l.price_includes_tax`
  to the `refreshInfoRow` SELECT (menu.ts:181-190) and to the response (menu.ts:265-280).
- `has_distance_tiers` (M2 RESOLVED — fail-safe, no RLS-subject read): do **NOT** derive it via an
  `EXISTS` subquery on `delivery_tiers`. Verified: `/info` runs `server.db.query` with **no tenant
  context** (`refreshInfoRow`, `menu.ts:181`), and `delivery_tiers` RLS is membership-scoped, so a public
  `EXISTS` returns FALSE for *every* venue — the **unsafe** direction (a tiered venue would take the
  exact-replication path and lie). Instead expose a **precomputed `has_distance_tiers` boolean maintained
  on the already-public `locations` row** (set when the owner adds/removes tiers in the config write) and
  read it directly in `/info`. It reveals only *whether* tiers exist (public-by-nature at checkout), not
  any tier rows. Reuse an existing column if one exists; otherwise add a small additive boolean
  (forward-only). **Client fail-safe:** when `has_distance_tiers` is `undefined`/unknown the client treats
  it as **true** → degrades to "confirmed at checkout". Ambiguity always degrades; the unsafe
  false→claim-exact path is eliminated by construction.

Contract addition is **additive** to the `/info` response (new optional fields) — `packages/shared-types`
`info` contract gains optional fields; no consumer breaks.

### 1.6 Consistency + idempotency + ACCEPTANCE CRITERIA

Determinism: the fee/tax math is pure integer arithmetic over `/info` inputs — same inputs → same
output, no float anywhere (money invariant: integer ALL). The shared building blocks
(`applyTax`/`computeLineTotal`/`assertNonNegative`) come from `@deliveryos/domain` (one impl, §1.4 C2 —
pure, safe to extract); the client `estimateOrderTotal` **mirrors** the server fee-selection arithmetic
(Approach M, NEW-3-H1 — the server create-path is NOT refactored to call it, so equality is NOT "by
construction"). The **load-bearing artifact** is therefore the parity gate:

**Parity guardrail (red→green, PERMANENT, never-deletable — the ship-blocker that makes the mirror safe):**
for the matrix {subtotal ∈ [0, min−1, min, threshold−1, threshold, threshold+1]} × {fee flat values} ×
{tax 0 / **included (back-out)** / excluded} × {`minor_unit` 0 and non-zero} (tax-included branch + non-zero
minor-unit explicit, Breaker C2 omission), the test computes the **REAL server total** — by exercising the
actual `orders.ts` fee arithmetic via a characterization fixture (extract the pure arithmetic the
create-path runs, or drive `POST /orders` and read back `total`) — and asserts
`estimateOrderTotal(sameInputs) === serverTotal`. It MUST be proven **red** (mutate the client mirror by 1
minor unit → fails) then **green**, with a `docs/regressions/REGRESSION-LEDGER.md` row. This converts the
hand-maintained mirror from a drift hazard into a CI-blocked invariant: drift cannot reach prod silently.
(Approach R — refactor the server to call the shared fn so equality holds by construction — is the recorded
deferred end-state for when server money math grows; until then this gate IS the guarantee, §1.1/ADR-0005.)

**AC-CASH-PARITY (ETHICAL-STOP-1 — recorded acceptance criterion, not an assumption; NEW-C1-resolved
without a phantom endpoint):** the authoritative cash figure is obtained by COMPUTABILITY:
- **Computable venue** (flat-fee + threshold + tax): the FE computes `reviewTotal` via the client mirror
  `@deliveryos/domain` `estimateOrderTotal` over public `/info` inputs. This `reviewTotal` **equals** the
  server total over the §1.6 parity-gated matrix (Approach M — the server does NOT call this fn; equality is
  CI-gated, not "by construction"), so the cash amount, `min`, change-due, and door figure are keyed to it.
  **No `/orders/quote` endpoint; the server charge path is untouched.**
- **Tiered venue** (`has_distance_tiers` true/unknown): the client cannot compute the fee (RLS hides
  tiers), so it shows NO exact cash pre-commit figure — CTA `{subtotal}+`; the order submits; the server
  `total` is surfaced to the courier delivery screen as "collect: {total}" and that is what the courier
  collects. The customer is never asked to pre-commit an exact cash number the design cannot back.
When `tip > 0`, the review shows order-total and tip as separate explicit lines; the collected sum =
`total + tip`. The figure the customer last reviews (computable venue) == the courier "collect: {total}"
door figure; for tiered venues the courier figure is the authoritative server total. Door-handover parity
holds in both branches.

**AC-CASH-422:** the FE handles `CASH_AMOUNT_TOO_LOW` (orders.ts:570) with a designed re-prompt that
re-shows the updated server total — never the generic "failed to place order".

**E2E (Playwright, staging):** (1) cash checkout at a venue whose fee config ≠ the estimate path → assert
the review shows the **server** total, the order persists with `total` == the reviewed figure, and the
courier delivery screen renders the same "collect: {total}"; (2) a forced `CASH_AMOUNT_TOO_LOW` 422 →
assert the designed re-prompt with the updated total, not the generic failure. The order-create response
already returns the authoritative `total` (orders.ts:760-761); a `checkout_fee_divergence` counter logs
estimate-vs-server-review drift in <1 min.

### 1.7 Failure + degradation

`/info` already degrades gracefully: stale-on-error within 1h, else **503** (menu.ts:218-231) — it
never 500s the storefront. Checkout MUST NOT block on the fee:
- `/info` failed / fields absent / `has_distance_tiers` true-or-unknown → CTA shows `Porosit • {subtotal}+`
  with a muted, equal-weight reason line **"delivery fee depends on your address — confirmed at checkout"**
  (Counsel A2: name *why* it can't be exact). The order still submits; the server computes the real
  `total`, surfaced at the review step before any cash commit (§1.4).
- Below-min: FE disables Order with an inline "Minimum order {minOrderValue}" message (soft, pickup
  included per H1); the server 422 remains the hard backstop if the FE config is stale.
- `CASH_AMOUNT_TOO_LOW`: handled explicitly (AC-CASH-422) — the designed re-prompt re-shows the updated
  server total; never the generic failure. (Defence-in-depth: the review step already keys cash to the
  server total, so this only fires on the narrow stale-window race.)
- Zero cascade: a fee-config read failure degrades the label only, never the order path.

### 1.8 Security + tenant

Fee config is public-by-nature (it is shown to every customer at checkout) — no PII, no secret. The
`/info` cache is per-slug and already bounded (menu.ts:193-198). Tenant isolation is intact: `/info` is
slug-scoped. The only watch-item is the `delivery_tiers` RLS read above (§1.5 flag).

### 1.9 Operability

Observable in <1 min: a `checkout_fee_divergence` counter (estimate-hint vs server-review total) surfaces
client/server drift immediately. Rollback: the change is additive — reverting the FE to the old constant
is a one-line revert; the `/info` extra fields are harmless if unused. The `CHECKOUT_FEE_REPLICATION`
flag (default on) now toggles only the **pre-review CTA hint** (exact number vs `{subtotal}+`); after C1
the collected sum is **always** the server-reviewed total regardless of the flag, so neither flag state
can mis-collect (L2 resolved — the flag is a cosmetic fast-kill, not a safety mechanism).

### 1.10 Open / accepted risks

- **R1 (accepted):** distance-tier venues show an estimate, not the exact fee. Owner: Product. Justified
  — false precision is worse than an honest "confirmed at checkout"; the server total is reviewed before
  any cash commit, so the estimate is never collected.
- **R2 (RESOLVED, §1.5):** `delivery_tiers` RLS visibility — replaced the RLS-subject `EXISTS` with a
  precomputed public `has_distance_tiers` boolean + fail-safe (unknown → degrade). Owner: Data agent.
- **R3 (RESOLVED via Approach M, NEW-3-H1):** the pure building blocks (`applyTax`/`computeLineTotal`/
  `assertNonNegative`) are **shared** via `@deliveryos/domain` (one impl); the client total
  (`estimateOrderTotal`) is a **mirror** of the server fee-selection arithmetic — the server create-path is
  NOT refactored to call it (the 🔴 money red-line is held; zero change to the charged amount). The parity
  guardrail (computes the REAL server total == client mirror, red→green, permanent) is the load-bearing
  guarantee, NOT a fallback. **Owned residual:** a hand-maintained mirror can drift — the gate fails CI on
  any drift over the matrix (drift cannot reach prod silently). **Escalation trigger:** when server money
  math grows (e.g. `discountTotal` nonzero / promo), the disposition becomes **Approach R** (extract the
  create-path arithmetic so both sides literally share one fn) as its own behaviour-preserving,
  characterization-gated change. Owner: Product/eng (mirror + trigger), API agent (extraction + fixture).
- **R7 (accepted, M1):** up-to-30s `/info` cache staleness on the pre-review CTA hint. Owner: Product.
  The authoritative total is read live at the review step; the cached value is never the collected sum.

---

## ITEM 2 — [STATE-MACHINE/DATA] Courier status honesty

### 2.1 Problem + non-goals

`apps/web/src/pages/admin/CouriersPage.tsx:179` maps the **account** status onto a **presence** label:

```ts
status: c.status === 'active' || c.status === 'available' ? 'online'
        : c.status === 'on_delivery' ? 'busy' : 'offline',
```

`couriers.status` is `active | deactivated | suspended` (migration `1780421029538`, default `'active'`).
So **every active account renders green "Online"** regardless of whether the courier is on shift — a
freshly-invited, phone-less courier who has never logged in shows "Online" and inflates the
`onlineCount` (CouriersPage.tsx:222). `'available'` in the FE map is a presence value
(`courier_shifts.status`) that the API does not even return on this endpoint (see contract below), so
that branch is dead. The count reads as a false "fleet is live" metric.

**The contract** (`GET /api/owner/locations/:locationId/couriers`, `apps/api/src/routes/owner/couriers.ts:17-70`)
returns per courier: `id, name, maskedPhone, maskedEmail, status` (= account status), `role`,
`onlineStatus: null` (presence is **not** surfaced here today — couriers.ts:53), `ordersToday`,
`deliveriesCompleted`, `rating`, `lastLoginAt`, `createdAt`.

Three orthogonal axes exist in the schema and must not be collapsed:
- **ACCOUNT** — `couriers.status` ∈ {active, deactivated, suspended} (migration `1780421029538`).
- **PRESENCE** — `courier_shifts.status` ∈ {offline, available, on_delivery} (migration `1780421036157`).
- **ONBOARDING** — derived: no-phone = `maskedPhone == null` (`phone_encrypted IS NULL`); never-logged-in
  = `lastLoginAt == null`.

**Non-goals.** Do not change the `couriers.status` state machine, the PATCH semantics, or the API
contract's existing fields. The minimal honest fix is FE-derived display; surfacing real presence is a
**documented optional contract addition** (`onlineStatus`), not a requirement of this change.

### 2.2 Back-of-envelope

A venue has O(1–20) couriers. The list is a single cached-free admin read; no scale concern. The "N
online" badge is a trust signal — its only requirement is to be *true*. Presence truth lives in
`courier_shifts` (the `couriers/live` endpoint at couriers.ts:141-193 already reads `cs.status IN
('available','on_delivery')` — that is the real online set). The honest count is "couriers with an
active shift", which the list endpoint does not currently join.

### 2.3 Options

**Option A — FE-only display fix, no contract change (account-status only; presence + onboarding both
unprovable here ⇒ neutral).**
Concept: *display truth from available data; never assert presence OR onboarding we can't prove*. Since
the list endpoint returns only account status — and (H4) cannot prove onboarding from `maskedPhone`/
`lastLoginAt` either (phone is optional; `last_login_at` is null after invite-redeem/refresh) — the FE
asserts only account state:
- `status === 'active'` → neutral **"Active"** (account-enabled), NOT a green online dot, NOT an
  onboarding claim.
- `deactivated` / `suspended` → **"Inactive"** / **"Suspended"**.
- The green "N online" badge is replaced by **"N active"** (total enabled accounts), labelled as
  "accounts enabled — see live map for who's on shift".
- Pros: zero contract/server change; impossible to over-claim presence OR libel a real courier as
  un-onboarded; ships immediately.
- Cons: the admin loses a real live-presence count from THIS screen (must open the live map, which has it).

**Option B — additive contract: surface real presence on the list (`onlineStatus`).**
Concept: *documented additive contract field carrying real presence*. Populate the already-present-but-
null `onlineStatus` (couriers.ts:53) from `courier_shifts` (LEFT JOIN latest shift, derive
online/busy/offline from `cs.status` + heartbeat freshness). FE then shows a truthful green "Online"
only when a current shift is `available`, "Busy" when `on_delivery`, else "Offline".
- Pros: the badge becomes a genuine live metric; uses the same presence source as the live map.
- Cons: a contract change (additive, low risk) + a heartbeat-staleness rule (how old before "offline") +
  the `last_login_at` stamping prerequisite (H4) if any onboarding signal is ever wanted.

### 2.4 Decision

**Ship Option A now (FE-only, this PR); document Option B as the follow-up.** Rationale: the task scope
is honesty *without changing server contracts*, and Option A delivers exactly that with zero risk — the
display stops lying immediately. The "N online" badge is replaced by **"N active"** (enabled accounts) so
no false presence is asserted; genuine live presence already exists on the live-map screen
(`couriers/live`). Option B (`onlineStatus` populated) is the documented additive follow-up, once the
heartbeat-staleness threshold is agreed.

**H4 CORRECTION (the proposal's original "Pending setup" criterion was unprovable and would libel real
couriers — re-pinned):** verified against source —
- `last_login_at` is stamped **only** by the password-login path (`courier/auth.ts:308`). The
  **invite-redeem** path (`auth.ts:88-148`) creates the courier, issues a JWT + 30-day session, and
  **never stamps it**; the **refresh-rotation** path (`:353-468`) does not stamp it either → a fully
  onboarded, session/refresh-authed courier has `last_login_at == null` indefinitely.
- **Phone is optional at invite-redeem** (`auth.ts:38`) → a fully onboarded courier may have
  `maskedPhone == null`.
- A `couriers` row in this list **exists only because an invite was redeemed** (sole prod creation path
  `auth.ts:89`; `server.ts:732` is dev-gated) → **row existence already proves onboarding.**

So `(!maskedPhone || !lastLoginAt) → pending_setup` would brand a real, possibly on-shift courier as
"Pending setup" forever (the Breaker's inverse-lie — a reachable production state). The list endpoint
**cannot prove onboarding-incompleteness**, so it must not assert it.

**Honest display = account-status only** (replacing the CouriersPage.tsx:179 map and the :222 count):

```ts
type CourierDisplay = 'active' | 'suspended' | 'inactive';
function deriveDisplay(c): CourierDisplay {
  if (c.status === 'suspended')   return 'suspended';
  if (c.status === 'deactivated') return 'inactive';
  return 'active';   // status === 'active' — account ENABLED, explicitly NOT a presence claim, no green dot
}
// badge: "N active" = count(status === 'active'). Honest as "enabled accounts" — NOT dispatch capacity.
// Label/tooltip (Counsel A5): "accounts enabled — see live map for who's on shift".
```

There is no FE-derived "Pending setup" — the screen says only what it can prove (account status). The map
presence (CouriersPage.tsx:205-216) keeps using the real shift-derived status where available; absent
that, it renders neutral, not "online".

**Prerequisite for the deferred Option B (server-side, additive, follow-up — NOT gating this PR):** make
`last_login_at` a real signal by stamping it at **invite-redeem** (after `auth.ts:89`) and at **refresh
rotation** (`:468`), matching the password-login stamp (`:308`). Until then `lastLoginAt` is meaningless
and MUST NOT drive any display claim. This is the documented prerequisite a real onboarding/presence model
needs before it can stand on `lastLoginAt`.

### 2.5 Data / migrations

**Option A: none.** Option B (deferred) is additive read-only (`onlineStatus` populated from a LEFT
JOIN on `courier_shifts`, RLS already FORCE-isolated by `app.current_tenant`, migration `1780421036157`)
— no schema change, only a query + response-field change.

### 2.6 Consistency + idempotency

Pure display derivation — idempotent and stateless. No write path touched.

### 2.7 Failure + degradation

`deriveDisplay` reads only `status` (always present), so there is no missing-field degrade path and no
dependence on `maskedPhone`/`lastLoginAt` (which the endpoint cannot use to prove onboarding — H4). The
screen never over-claims "online": no path renders green from the list endpoint.

### 2.8 Security + tenant

No change. The list endpoint already runs under `set_config('app.current_tenant')` with RLS FORCE on
`courier_locations` (couriers.ts:24-25). No additional PII surfaced (phone stays masked).

### 2.9 Operability

Trivially observable via the screen itself. Rollback = one-line FE revert. No flag needed.

### 2.10 Open / accepted risks

- **R4 (accepted):** the list screen no longer shows live presence after Option A; the live map carries
  it. Owner: Product. Resolved fully by Option B when implemented.
- **R8 (accepted, H4):** "N active" counts enabled accounts, not on-shift couriers, and is labelled as
  such. Owner: Product. The honest signal is "accounts enabled"; dispatch capacity lives on the live map.
- **R9 (follow-up, H4):** `last_login_at` is not stamped at invite-redeem/refresh; stamping it is the
  documented prerequisite for Option B. Owner: API agent. Not gating this PR (Item 2 uses status only).

---

## ITEM 3 — [TEST-INFRA] Encrypted-courier dev-seed for visual capture

### 3.1 Problem + non-goals

Goal: make `/courier/delivery/:assignmentId` render LIVE in the 390px visual capture. The chain is
`courier_assignments.courier_id → couriers(id)`, and `couriers` requires encrypted PII
(`email_encrypted`, `full_name_encrypted` bytea NOT NULL; `email_hash` NOT NULL UNIQUE; `password_hash`
NOT NULL — migration `1780421029538`). The current visual seed (`/dev/seed-visual-state`,
`apps/api/src/routes/dev/mock-auth.ts:228-433`) creates an owner + venues + menu + **one order** and
returns a *stable but non-existent* `VIS_COURIER_ID` (`00000000-0000-4000-8000-0000000000c1`,
mock-auth.ts:227) — there is **no `couriers` row, no shift, no assignment**, so the courier delivery
screen has nothing live to render. `/dev/mock-auth` already accepts `role: 'courier'` but signs a token
for a **random** courierId each call (mock-auth.ts:14) — it cannot impersonate the seeded courier.

**Non-goals.** No real PII (synthetic constants only). No production surface. No new courier endpoint.
No change to the courier auth/crypto path beyond reusing `encryptPII` + `argon2`.

### 3.2 Back-of-envelope

The seed is a single-shot dev fixture, called once per visual-capture run. Three small UPSERTs
(courier, shift, assignment) + one `argon2.hash` (~50–100ms). Negligible. Re-runnable: keyed on stable
natural keys so re-running never duplicates (mirrors the existing seed's discipline, mock-auth.ts:266).

### 3.3 Options

**Option A — extend `seed-visual-state` to UPSERT courier + shift + assignment; `mock-auth` accepts `body.courierId`.**
Concept: *deterministic encrypted fixture behind the existing dev gate*. In the visual seed, after the
order is created (mock-auth.ts:382-417), UPSERT one courier with synthetic encrypted PII keyed on a
fixed `email_hash`, link it to the open venue (`courier_locations`), open a `courier_shift`
(`status='available'`), and create a `courier_assignment` for the seeded order (`status='assigned'`).
Return the `assignmentId` + a fixed `courierId` in the response. `/dev/mock-auth` gains an optional
`body.courierId` so the harness impersonates exactly that courier.
- Pros: one seed produces the whole live chain; deterministic ids; reuses the proven dev gate and the
  established `encryptPII`/`argon2`/`email_hash` pattern (courier auth.ts:77-91).
- Cons: the seed grows; must respect RLS on `courier_shifts`/`courier_assignments`/`courier_locations`.

**Option B — separate `/dev/seed-courier` endpoint.**
Concept: *isolated dev seeder*. A standalone handler creates the courier chain on demand.
- Pros: smaller blast radius on the visual seed; reusable independently.
- Cons: the harness must orchestrate two calls (seed-visual → seed-courier) and pass ids between them;
  more moving parts; duplicates the venue/order lookup.

### 3.4 Decision — HELD for a human a-vs-b call (Counsel steel-man + Breaker L1/M3/M4 + prior `dev-login-backdoor`)

**This item is flag-held pending a human decision; it does NOT block Items 1 & 2.** Counsel steel-manned
NOT building Item 3 and weighted proportionality *above* the proposal: the `mock-auth` `body.courierId`
change is an impersonation expansion of the same class as the project's prior `dev-login-backdoor`
CRITICAL. The Architect agrees this is a close strategy/proportionality call and records both choices:

- **(a) DROP Item 3 (Architect's recommendation).** Leave the live-courier-delivery screen as a documented
  visual gap; the not-found state is already captured and verified centered. Zero new dev-seed surface,
  zero `mock-auth` broadening, no synthetic courier persisted on staging. The 390px courier snapshot
  renders a static/mock state. Clean, defensible, "best code is the code never written".

- **(b) BUILD Item 3, hard-constrained** (only if the end-to-end real-auth/crypto/RLS render signal is
  judged worth the permanent surface). **All FIVE constraints are mandatory — partial (b) recreates the
  backdoor shape and/or re-pollutes Item-2's honest count, and is not acceptable:**
  1. **(L1) `mock-auth` mints a token ONLY for the single synthetic seeded courier id** (a server-side
     constant the seed produced). **Arbitrary `body.courierId` caller input is REMOVED** — un-abusable
     even if the staging dev gate leaks.
  2. **(M4 / NEW-H3) Synthetic `email_hash` is a namespaced non-email sentinel**
     `sha256('synthetic:visual-net-courier:v1')` (NOT a hash of a parseable email). The M4
     **resurrection** vector is closed by the hash *alone*: the namespaced sentinel can never be produced
     by any `z.string().email()` input, so `ON CONFLICT (email_hash) DO UPDATE` provably reaches ONLY the
     synthetic row — no `.test` email is relevant to the seed's safety.
     **NEW-H3 (the human ship-blocker, a SEPARATE constraint): `.test`/reserved-TLD rejection is a real,
     listed code change, NOT prose and NOT "defence-in-depth".** It is independent registration hygiene
     (non-routable `*.test`, `*.localhost`, `*.invalid`, `*.example` addresses pass `z.string().email()`
     vacuously). It MUST be implemented as a shared Zod refinement (a single `rejectReservedTld` helper)
     applied at **every** registration/auth email parse:
       - `apps/api/src/routes/courier/auth.ts:34` (courier invite-redeem),
       - `apps/api/src/routes/auth/local.ts:41` (owner register/login),
       - `apps/api/src/routes/public/access-requests.ts:56` (access-request email).
     Reject RFC-2606 reserved TLDs `{.test, .example, .invalid, .localhost}` (plus the second-level
     reserved domains) with a 400. This is a hard ship-blocker per the human; the sentinel hash does NOT
     substitute for it (they close different threats: hash → seed-row isolation; `.test` reject →
     registration-namespace hygiene).
  3. **(M3) Idempotent seed:** `DELETE FROM courier_shifts WHERE courier_id = $synthetic AND location_id =
     $openId` before insert; shift `status = 'available'` (matches the proven `/dev/create-assignment`
     path and keeps the state-machine snapshot coherent with the `accepted` pre-pickup assignment).
  4. **(L3) Synthetic-owned conflicts:** the seed creates and owns its order id; every `ON CONFLICT`
     target is synthetic.
  5. **(NEW-M2 / Counsel A4-Q2) Exclude the synthetic courier from owner-facing lists AND counts —
     structurally, not just by display name.** The synthetic courier is `status='active'`, so Item 2's
     honest "N active" badge would COUNT it on the shared staging DB — re-introducing the exact Item-2
     dishonesty (an inflated active count on owner walkthroughs). A display-name marker fixes the *name*,
     not the *count*. **Resolution:** filter the synthetic row out of the owner couriers query by its
     namespaced sentinel `email_hash`. Concrete change — `apps/api/src/routes/owner/couriers.ts:34` WHERE
     clause gains:
       `AND c.email_hash <> $2`  with `$2 = sha256('synthetic:visual-net-courier:v1')`
     (the same sentinel constant; a shared `SYNTHETIC_COURIER_EMAIL_HASH` export). On prod the synthetic
     row never exists (dev-gate dark), so the predicate is a harmless no-op there; on staging it removes the
     fixture from both the list AND the `count(status==='active')`. Keep the "Visual Net Courier" display
     name as belt-and-suspenders, but the count/list filter is the load-bearing fix. (No `is_synthetic`
     column needed — the existing UNIQUE `email_hash` sentinel already identifies the row; no migration.)

The design below reflects **(b)-hardened**; if the human picks (a) it is deleted wholesale.
Rationale (if (b)): the visual seed already owns the order the assignment must reference
(mock-auth.ts:385-417), so producing the courier chain in the same transaction-scoped handler keeps the
fixture coherent and the harness contract a single call. Reuse the **proven** courier-creation pattern
(`encryptPII(...)`, `argon2.hash(...)`, RLS-scoped writes, auth.ts:77-91) so we inherit its correctness —
but with the namespaced sentinel hash and synthetic-only mint, not the original arbitrary-impersonation
shape.

Design (added to `seedVisualHandler` after step 6, mock-auth.ts:417):

```ts
// 6b. Encrypted SYNTHETIC courier — idempotent on a NAMESPACED NON-EMAIL sentinel hash (M4):
//     the email_hash cannot collide with any real courier (it is not a hash of a parseable email),
//     so ON CONFLICT DO UPDATE provably reaches ONLY the synthetic row — it can never resurrect a
//     real suspended courier. Registration must ALSO reject `.test`/reserved TLDs (defence-in-depth).
const VIS_COURIER_NAME  = 'Visual Net Courier';
const VIS_COURIER_PHONE = '+355690000009';
const emailHash = crypto.createHash('sha256').update('synthetic:visual-net-courier:v1').digest('hex'); // NOT an email
const phoneHash = crypto.createHash('sha256').update('synthetic:visual-net-courier-phone:v1').digest('hex');
const pwHash    = await argon2.hash('visual-net-not-a-secret');   // synthetic; courier never logs in via this
const courierRes = await db.query(
  `INSERT INTO couriers (email_encrypted, email_hash, phone_encrypted, phone_hash,
                         full_name_encrypted, password_hash, status, last_login_at)
     VALUES ($1,$2,$3,$4,$5,$6,'active', now())
   ON CONFLICT (email_hash) DO UPDATE SET status='active', last_login_at=now()
   RETURNING id`,
  [encryptPII('synthetic-vis-courier'), emailHash, encryptPII(VIS_COURIER_PHONE), phoneHash,
   encryptPII(VIS_COURIER_NAME), pwHash]);
const SYNTHETIC_COURIER_ID = courierRes.rows[0].id;  // the ONLY id mock-auth may mint a token for

// 6c. Link to the OPEN venue + shift + assignment, RLS-scoped (mirror /dev/create-assignment).
const client = await db.connect();
try {
  await client.query('BEGIN');
  await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [openId]);
  await client.query(`SELECT set_config('app.user_id', $1, true)`, [ownerId]);
  await client.query(
    `INSERT INTO courier_locations (courier_id, location_id, role)
       VALUES ($1,$2,'courier') ON CONFLICT (courier_id, location_id) DO NOTHING`,
    [SYNTHETIC_COURIER_ID, openId]);
  // (M3) shifts have no natural key → DELETE this synthetic courier's shifts first → re-run idempotent.
  await client.query(
    `DELETE FROM courier_shifts WHERE courier_id = $1 AND location_id = $2`,
    [SYNTHETIC_COURIER_ID, openId]);
  const shift = await client.query(
    `INSERT INTO courier_shifts (courier_id, location_id, status, started_at, last_heartbeat_at)
       VALUES ($1,$2,'available', now(), now()) RETURNING id`, [SYNTHETIC_COURIER_ID, openId]); // 'available' matches the proven path
  const asgn = await client.query(
    `INSERT INTO courier_assignments (order_id, courier_id, location_id, shift_id, status, assigned_at, accepted_at)
       VALUES ($1,$2,$3,$4,'accepted', now(), now())
     ON CONFLICT (order_id) DO UPDATE SET courier_id=EXCLUDED.courier_id, shift_id=EXCLUDED.shift_id, status='accepted'
     RETURNING id`,
    [orderId, SYNTHETIC_COURIER_ID, openId, shift.rows[0].id]);  // orderId is synthetic, seed-owned (L3)
  await client.query('COMMIT');
  assignmentId = asgn.rows[0].id;
} catch (e) { await client.query('ROLLBACK').catch(()=>{}); throw e; } finally { client.release(); }
```

Return shape gains `courierId: SYNTHETIC_COURIER_ID` + `assignmentId`. **`/dev/mock-auth` does NOT accept
an arbitrary `body.courierId` (L1).**

**NEW-M1 — PIN the re-derive variant; the echo-back variant is REMOVED.** `mock-auth` mints a courier
token by **re-deriving** the synthetic id server-side from the same namespaced sentinel hash and selecting
the courier row — it NEVER reads, compares, or echoes any caller-supplied id:

```ts
// /dev/mock-auth, role:'courier' — re-derive ONLY. No body.courierId is read.
const emailHash = crypto.createHash('sha256').update('synthetic:visual-net-courier:v1').digest('hex');
const row = await db.query(`SELECT id FROM couriers WHERE email_hash = $1`, [emailHash]);
if (!row.rows[0]) return reply.status(404).send({ error: 'synthetic courier not seeded' });
const sub = row.rows[0].id;   // the ONLY id this path can mint a token for
```

The echo-back shape (seed returns id → harness sends it back → server validates) is rejected: it
re-introduces a caller-supplied `courierId` + a guard — the exact `dev-login-backdoor` shape (one typo
from `if (body.courierId) sub = body.courierId`). Re-derive takes NO caller input, so constraint #1 ("never
arbitrary caller input") holds by construction, not by a guard. A leaked staging gate can mint a token for
the one synthetic fixture and nothing else.

### 3.5 Data / migrations

**None — dev-seed only.** No schema change. Re-run idempotency: `couriers` keyed on the **namespaced
sentinel `email_hash`** (UNIQUE, non-email → no real-courier collision, M4); `courier_locations`
`ON CONFLICT DO NOTHING`; `courier_assignments` `ON CONFLICT (order_id) DO UPDATE` (synthetic order, L3).
`courier_shifts` has no natural key — the shown code **DELETEs this synthetic courier's shifts for
`openId` before inserting** (M3 — the code now matches the prose; no shift accumulation across runs).

### 3.6 Consistency + idempotency

Whole courier chain is UPSERT/scoped-reseed → re-running the seed converges to the same state
(stable `email_hash` → stable courier id; `order_id` unique → stable assignment). Encryption uses a
fresh random IV each run (ciphertext bytes differ) but that does not affect the row identity or
idempotency — the natural keys are the hashes, not the ciphertext.

### 3.7 Failure + degradation

If `COURIER_PII_ENCRYPTION_KEY` is missing, `encryptPII` throws (pii-cipher.ts:10-11) and the seed
fails loudly — correct (a dev fixture should not silently produce a broken courier). The visual harness
should treat a courier-seed failure as a hard error for the courier-screen snapshot only, not for the
venue/menu snapshots already produced earlier in the handler. **Verify `COURIER_PII_ENCRYPTION_KEY` is
set on staging** (it must be, since real courier auth already depends on it) — flag for the conductor.

### 3.8 Security + tenant — PROD-SAFETY PROOF

**The hard constraint is prod-safety.** The proof is layered and already established (ADR-0003):

1. **Dev-gate inheritance.** Both `/dev/seed-visual-state` and `/api/dev/seed-visual-state` are
   registered under the `/dev` (+ `/api/dev`) prefix, which `server.ts`'s `onRequest`
   `isDevRequestAuthorized` hook gates: **every** dev path 404s unless `ALLOW_DEV_LOGIN === 'true'`
   **AND** a matching `x-dev-auth-secret` header is present (ADR-0003, fails closed). The new courier
   code lives *inside* the existing `seedVisualHandler`, so it adds **zero new prod surface** — it
   inherits the same gate (confirmed by the in-file comment, mock-auth.ts:205-209). `/dev/mock-auth`
   minting the **synthetic-only** courier token is likewise under the same gate — and (L1) it mints ONLY
   the single synthetic id, never arbitrary caller input, so a leaked staging gate cannot impersonate a
   real courier (defence-in-depth beyond the prod gate, removing the `dev-login-backdoor`-class shape).
2. **No real PII.** The `email_hash` is a namespaced non-email sentinel (not a parseable address);
   name/phone are fixed synthetic constants;
   the password is a non-secret literal used only to satisfy the NOT NULL `password_hash` (the courier
   never authenticates via password in the capture — the harness impersonates via the dev token). No
   real person's data is encrypted or stored.
3. **RLS FORCE correctness.** The shift/assignment/location-link writes run inside a transaction with
   `set_config('app.current_tenant', openId, true)` + `set_config('app.user_id', ownerId, true)` —
   mirroring the proven `/dev/create-assignment` path (mock-auth.ts:90-92) so the RLS-FORCE policies on
   `courier_shifts`, `courier_assignments`, `courier_locations` (migrations `1780421036157`,
   `1780421029538`) are satisfied with the correct tenant context.

**Prod-safety verification (the proof artifact):** an E2E assertion that `POST /dev/seed-visual-state`
**without** the `x-dev-auth-secret` header (and/or with `ALLOW_DEV_LOGIN` unset) returns **404** on a
prod-config instance — i.e. the courier rows can never be created in prod. This is the same gate the
existing seed already relies on; the change adds nothing that escapes it. Memory note confirms prod has
`ALLOW_DEV_LOGIN` off and no dev-secret set → the path is dark in prod.

### 3.9 Operability

The seed returns `courierId` + `assignmentId` for the harness contract; a failed run errors loudly
(no silent half-seed — the courier writes are transaction-wrapped). Rollback: the courier rows are
dev-only and tenant-scoped to the visual venue; no prod impact, nothing to roll back in prod.

### 3.10 Open / accepted risks

- **R5 (open, verify — only if Item 3(b)):** `COURIER_PII_ENCRYPTION_KEY` present on the staging instance
  running the visual capture. Owner: conductor/ops. (Almost certainly yes — real courier auth needs it.)
- **R6 (RESOLVED, NEW-M2 — only if Item 3(b)):** the synthetic courier persists in the staging DB between
  runs, but is **filtered out of the owner couriers list AND the "N active" count** by its sentinel
  `email_hash` (`owner/couriers.ts:34` WHERE `c.email_hash <> SYNTHETIC_COURIER_EMAIL_HASH`). It can no
  longer pollute Item-2's honest count or appear in an owner walkthrough. Display-name marker kept as
  belt-and-suspenders. Prod: no-op (row never exists). Owner: API agent.
- **R10 (HUMAN-NEEDED):** Item 3 a-vs-b is held for a human decision (§3.4). Items 1 & 2 ship regardless.
  If (b): all **five** hard constraints (synthetic-only **re-derive** mint [NEW-M1], namespaced sentinel
  hash + **real `.test` reject at 3 endpoints** [NEW-H3], idempotent seed [M3], synthetic-owned conflicts
  [L3], **owner-list/count exclusion filter** [NEW-M2]) are mandatory; partial (b) recreates the
  `dev-login-backdoor` shape and/or re-pollutes Item-2's honest "N active" count.

---

## Summary of decisions

| Item | Decision | Concept | Contract change | Migration |
|---|---|---|---|---|
| 1 Fee | Option A′ — estimate is a HINT ONLY; **server `total` reviewed before any cash commit** (== door-collected sum); **Approach M** (client `estimateOrderTotal` MIRRORS the server, charge path UNTOUCHED; parity gate red→green is the guarantee); shared `@deliveryos/domain` building blocks; degrade fail-safe for tiered/`/info`-down. Approach R (server refactor) deferred until money math grows | server-authoritative config + gated client mirror + before-commit review + graceful degradation | additive `/info` fields (+ public `has_distance_tiers`) | none (or one additive boolean if no reusable column) |
| 2 Courier | Option A — FE-only **account-status-only** display ("N active" = enabled accounts, no "Pending setup"); Option B (`onlineStatus`) follow-up + `last_login_at` stamping prerequisite | account vs presence vs onboarding separation; say only what you can prove | none (B additive, deferred) | none |
| 3 Seed | **HELD for human (a) drop vs (b) build-hardened**; (b) = synthetic-only **re-derive** mint + namespaced sentinel hash + **real `.test` reject (3 endpoints)** + idempotent seed + **owner list/count exclusion filter** (all FIVE mandatory) | deterministic encrypted dev fixture, un-abusable even on a leaked dev gate, invisible to Item-2's honest count | none (dev-only) | none |

ADRs: `docs/adr/0005-delivery-fee-source-of-truth.md`, `docs/adr/0006-courier-status-display-model.md`.
Resolution: `docs/design/fee-courier-seed/resolution.md` (per-finding dispositions).

**Branch/scope note (Counsel §5):** `fix/design-system-consistency` carries money + state-machine +
test-infra work, none of which is design-system. Recommend renaming/splitting on the next push so the
health-pass ledger reflects what shipped.
