# Resolution — fee-courier-seed (Architect, RESOLVE step)

Council: Architect (this doc) resolving Breaker findings (`breaker-findings.md`) + Counsel
ETHICAL-STOPs/advice (`counsel-opinion.md`). Date: 2026-06-25. Branch: `fix/design-system-consistency`.

Every Breaker finding and every Counsel ETHICAL-STOP below carries a disposition:
**fix** (proposal/ADR updated) · **accept-risk** (justification + owner) · **defer-flag** (MISSING,
tracked) · **human-needed** (recorded for human decision). Verifications were re-checked against live
source before resolving — the load-bearing source claims (cash gate `orders.ts:568`, FE 422 handling
`CheckoutPage.tsx:491`, `applyTax` two-branch BigInt `lib/money.ts:23-44`, `last_login_at` stamping,
invite-redeem, `/info` role) all reproduce as the Breaker stated.

---

## CRITICAL

### C1 — cash-on-delivery: replicated CTA = money at the door + gates a server 422 → **FIX**

Reproduced verbatim: `CheckoutPage.tsx:835` `min={total}`, `:839` red-border on `cashAmount < total`,
`:870` change `= cashAmount - total`, `:873` "Amount must be at least {total}"; server hard-reject
`orders.ts:568` `cashPayWith < total → 422 CASH_AMOUNT_TOO_LOW`; FE handles only `MIN_ORDER_NOT_MET`
(`:491`), no `CASH_AMOUNT_TOO_LOW` handler. Under Option A′ as written, `total → estTotal`, so the FE
cash gate and the change-due figure are keyed to a *client-guessed* number — the literal cash the human
hands over and a server 422 trigger. The Breaker's verdict that "server total is always final" is FALSE
for cash is correct.

**Resolution (design change, now in proposal §1.4 / §1.6 / ADR-0005 Decision pt.5-7):**

1. **The cash amount is NEVER keyed to the client estimate.** The cash input, the `min`, the red-border
   threshold, the change-due, and the door-collection figure are all keyed to the **server-authoritative
   `total`** — which the client obtains *before* the customer commits to a cash amount, via an explicit
   **server-confirmed review step** (a price preflight; see ETHICAL-STOP-1). The estimate `estTotal` is
   only ever a *pre-review CTA hint* (`Porosit • {estTotal}` / `{subtotal}+`); it never becomes the
   collected sum.
2. **Order of operations is inverted to make this safe:** the customer reviews the **server total**
   (returned by an order *preflight* that runs the exact `orders.ts` fee math without persisting, OR by
   surfacing the existing soft-confirm/hard-block preflight already on `POST /orders`) and only then
   confirms cash. The cash amount the FE sends is validated against that server total, not the estimate,
   so `cashPayWith < total` can no longer fire for a customer who entered the displayed exact amount.
3. **FE handles `CASH_AMOUNT_TOO_LOW` explicitly** (defence-in-depth for the stale-window race): a
   designed message "The total updated to {server_total}. Please confirm the cash amount." that re-shows
   the authoritative total and re-prompts — never the cold generic "failed to place order".
4. **Tip is excluded from `total` server-side** (`orders.ts:565` has no tip term — verified) but is
   physically collected on top in cash. The review step shows **two lines**: "Order total (collected):
   {server_total}" and, when `tip > 0`, "+ Tip (cash to courier): {tip}", so the door figure is
   unambiguous (resolves H2's tip seam at the same point).

This converts "server total is final" from an after-the-fact reconciliation (structurally impossible for
cash) into a **before-commit review** — the only shape that makes "shown == collected" true for cash.

### C2 — `applyTaxClient` "exact mirror" under-specified vs the real two-branch BigInt `applyTax` → **FIX (adopt Counsel A1: SHARE, don't mirror)**

Reproduced: `lib/money.ts:23-44` is two distinct BigInt algorithms — tax-EXCLUDED forward half-up
`(sub*rateMicro + SCALE/2)/SCALE` and tax-INCLUDED back-out `net = (sub*SCALE + denom/2)/denom; tax =
sub − net`, with `rateMicro = BigInt(Math.round(taxRate*1e6))` (6-dp micro-units). A float reimplementation
drifts. The proposal pinned neither the branch nor the quantisation.

**Resolution — take Counsel A1 over the proposal's "reimplement + parity test":** extract the exact
`applyTax` (and `computeLineTotal`, `assertNonNegative`) into **`@deliveryos/domain`**, a single
isomorphic (no Node-only deps) implementation **imported by both** `apps/api` and `apps/web`. One source
of truth beats a parity test guarding two copies — the test catches drift *after* it is written; shared
code prevents the second copy from existing. The function moves **verbatim** (same BigInt branches, same
`rateMicro` micro-unit quantisation, same half-up). The client passes the same operands the server reads
from `/info` (`tax_rate`, `price_includes_tax`, `currency_minor_unit`, integer `subtotal`).

If a true isomorphic share is infeasible (bundle/SSR constraint surfaces in implementation), the fallback
is the parity guardrail — but treated as a **permanent, never-deletable gate**, and its matrix is
expanded to explicitly include the **tax-INCLUDED back-out branch** and **non-zero `minor_unit`**
(Breaker's omission). Shared-code is the primary; parity-test is the documented fallback. See H3 for the
minor-unit contract the shared function pins.

---

## HIGH

### H1 — FE min-gate (delivery-only) ≠ server 422 (delivery+pickup); modifier-price subtotal mismatch → **FIX**

Reproduced: server `orders.ts:519` checks `subtotal < min_order_value` **above** the `isPickup` branch
(`:529`) → applies to **pickup too**; FE proposal gated delivery-only. Second divergence: server subtotal
is `computeLineTotal(price, modifierPrices, qty)` (`orders.ts:506`, modifier-inclusive); FE
`items.reduce(price*qty)` (`CheckoutPage.tsx:341`) — must verify `item.price` already folds modifiers.

**Resolution (proposal §1.4):**
1. FE min-gate drops the `deliveryType === 'delivery'` condition:
   `belowMin = minOrderValue != null && subtotal < minOrderValue` (matches server — pickup included).
2. **Subtotal parity is an explicit acceptance criterion:** the cart `subtotal` MUST equal the server's
   modifier-inclusive `computeLineTotal` sum. Implementation MUST verify `item.price` includes modifier
   prices; if it does not, the FE subtotal is computed modifier-inclusive before any gate/CTA. The C2
   shared-domain `computeLineTotal` is reused here so the two subtotals are computed by the same function.

### H2 — tip omitted from estTotal/cash reconciliation; tax-inclusive subtotal display vs threshold → **FIX**

The threshold-on-raw-subtotal comparison matches the server (`orders.ts:530`) — no boundary bug. The two
live seams: (a) **tip** is added in cash on top of `total` but absent from the estimate — resolved by C1
pt.4 (the review step shows order-total and tip as separate explicit lines; the collected figure is
total + tip when tip > 0). (b) **subtotal shown vs compared** for tax-inclusive venues: the proposal now
pins that the **same integer `subtotal`** is used for both the threshold comparison and any displayed
subtotal — there is no tax-exclusive surfacing. For today's ALL universe (`tax_rate = 0`) this is moot;
the rule is documented so a future non-ALL venue cannot reintroduce the divergence.

### H3 — server `applyTax` `_minorUnit` param is DEAD; the mirror has nothing coherent to mirror → **FIX (pin the contract)**

Reproduced: `lib/money.ts:23` `_minorUnit` is **unused**; `applyTax` does NOT call `roundHalfUp`/round to
the currency's minor unit — it returns the raw per-minor-unit BigInt. So the server contract IS "tax is
computed in the stored minor-unit integer space, no further minor-unit rounding."

**Resolution:** the shared `@deliveryos/domain` `applyTax` (C2) IS the contract — there is no second
interpretation to drift against because there is no second copy. The pinned rule, recorded in ADR-0005
and the function's doc-comment: **`applyTax` operates entirely in stored minor-unit integers; the
`minorUnit` argument is informational/forward-compat and MUST NOT trigger coarser rounding.** The client
therefore does NOT round to the major unit. Today's universe is ALL (`tax_rate = 0`, `minor_unit = 0`) →
tax collapses to 0; the contract is pinned now so the first non-ALL venue inherits a single, unambiguous
formula. (If a future product decision wants minor-unit rounding, that is a *server* change to the shared
function — both sides move together by construction.)

### H4 — courier inverse-lie: a real on-shift courier rendered "Pending setup" forever → **FIX (re-pin the criterion; the proposal's was unprovable)**

This is the most important courier finding and the Breaker + my re-verification expose that the
**proposal's "Pending setup" criterion is not just risky — it is unprovable and actively wrong:**

- `last_login_at` is stamped **only** by the password-login path (`courier/auth.ts:308`). The
  **invite-redeem** path (`auth.ts:88-148`) creates the courier, issues a JWT + 30-day session, and
  **never stamps `last_login_at`** — and the **refresh-rotation** path (`:353-468`) does not stamp it
  either. So a fully-onboarded courier who accepted their invite and authenticates via session/refresh
  has `last_login_at == null` indefinitely.
- **Phone is optional at invite-redeem** (`auth.ts:38` `phone: z.string().optional()`), so a fully
  onboarded courier can legitimately have `maskedPhone == null`.
- A `couriers` row in the list **only exists because an invite was redeemed** (the sole prod creation
  path is `auth.ts:89`; `server.ts:732` is dev-gated). **Row existence already proves onboarding.**

So `(!maskedPhone || !lastLoginAt) → pending_setup` would brand a real, phone-less or refresh-authed,
possibly on-shift courier as "Pending setup" forever. The Breaker's inverse-lie is a reachable production
state, not a hypothetical.

**Resolution (proposal §2.4 / ADR-0006 — re-pinned):**

1. **Drop the FE-derived "Pending setup" entirely.** The list endpoint cannot prove onboarding state and
   must not assert it. The honest display is **account-status only**:
   - `suspended` → "Suspended"
   - `deactivated` → "Inactive"
   - `active` → **"Active"** (account enabled — explicitly NOT a presence claim; no green dot).
2. **"N active" counts `status === 'active'` accounts** — honest as "enabled accounts", and the label/
   tooltip names exactly that ("accounts enabled — see live map for who's on shift", Counsel A5) so it is
   not misread as dispatch capacity.
3. **Make `last_login_at` a real signal (server-side, additive, one line) so a future presence/onboarding
   model has truth to stand on:** stamp `last_login_at = now()` at **invite-redeem** (`auth.ts` after the
   courier INSERT) and at **refresh rotation** (`:468` path), matching the password-login stamp. This is
   the documented prerequisite for the deferred Option B; without it `lastLoginAt` is meaningless. Filed
   as a tracked follow-up, NOT gating Item 2's FE-only ship.
4. **"N active" never claims presence.** Option B (real presence from `courier_shifts` + heartbeat
   staleness) remains the deferred end-state; until then the screen says only what it can prove.

This removes the inverse-lie by refusing to assert onboarding the endpoint cannot see — the same
"say only what you can prove" discipline Counsel identified as the unifying concept.

---

## MEDIUM

### M1 — 30s (up to 1h stale-on-error) `/info` cache → silent CTA-vs-charge divergence → **ACCEPT-RISK (mitigated by C1 review step)**

Reproduced: `MENU_CACHE_TTL_MS` 30s, stale-on-error ≤1h (`menu.ts:202-224`); order path reads fee live.
The window is real. **But after C1, the divergence is harmless at the only place it could hurt:** the
customer reviews the **server total** (live fee math) before committing cash, so a stale estimate is
corrected at the review step, never collected at the door. The residual is purely cosmetic (the pre-review
CTA hint may be 30s stale) and is explicitly labelled an estimate until review.
**Accepted.** Owner: Product. Justification: the authoritative figure is read live at review/commit; the
cached value is never the collected sum. A `checkout_fee_divergence` counter (estimate vs server review
total) surfaces drift in <1 min if the window ever widens.

### M2 — `delivery_tiers` RLS hidden from public role → `has_distance_tiers=false` → tiered venue takes the exact (lying) path → **FIX (fail-safe + precomputed boolean)**

Reproduced: `/info` runs `server.db.query` with **no tenant context** (`refreshInfoRow`, `menu.ts:181`);
`delivery_tiers` RLS is membership-scoped → the public role's `EXISTS` returns FALSE for every venue,
which is the **unsafe** direction (claim-exact for a tiered venue).

**Resolution (proposal §1.5 / ADR-0005 Open→Decided):**
1. Do **not** derive `has_distance_tiers` via an RLS-subject `EXISTS` on the public role. Instead expose a
   **precomputed `has_distance_tiers` boolean maintained on `locations`** (set true/false when tiers are
   added/removed via the owner config write), read directly from the already-public `locations` row in
   `/info`. No tenant rows are exposed (it is a single boolean about *whether* tiers exist, which is
   public-by-nature at checkout). **No migration required if** an existing column/derived flag is reused;
   otherwise a small additive boolean column is the cheapest correct option (still additive, forward-only).
2. **Fail-safe default:** if `has_distance_tiers` is `undefined`/unknown for any reason, the client treats
   it as **true** → degrades to "fee confirmed at checkout". The unsafe direction (false → claim-exact) is
   eliminated by construction: ambiguity always degrades.

### M3 — seed shift re-run not idempotent (code has no DELETE the prose promises); shift status mismatch → **FIX (only if Item 3(b) is chosen)**

Reproduced: `courier_shifts` has no natural key; §3.4 code is a bare INSERT; §3.5 prose promises a DELETE
the code omits; §3.4 seeds `status='on_delivery'` while the proven `/dev/create-assignment` uses
`'available'`.
**Resolution (conditional on Item 3 disposition):** if Item 3 ships (option b), the seed MUST
`DELETE FROM courier_shifts WHERE courier_id = $synthetic AND location_id = $openId` before the INSERT
(scoped to the synthetic courier only), and the shown code MUST match the prose. Shift status is pinned to
**`available`** to match the proven path and to keep the state-machine snapshot consistent (an `available`
shift with an `accepted` assignment is the coherent pre-pickup state). If Item 3 is dropped (option a),
this is **N/A**.

### M4 — seed `ON CONFLICT (email_hash) DO UPDATE status='active'` can resurrect a real suspended courier → **FIX (only if Item 3(b); make collision impossible)**

Reproduced: `email_hash NOT NULL UNIQUE`; `z.string().email()` (`auth.ts:34`) accepts a `.test` TLD (zod's
email check does not validate TLD existence), so a tester *could* register `vis-courier@dowiz.test` and
the seed's `DO UPDATE SET status='active', last_login_at=now()` would silently un-suspend them and stamp a
fake login.
**Resolution (conditional on Item 3(b)):** the synthetic `email_hash` MUST be an **id-namespaced constant
that cannot collide with any real-courier email hash** — e.g. hash a namespaced sentinel
`sha256('synthetic:visual-net-courier:v1')` (NOT a hash of a parseable email address), so no registration
input can ever produce the same hash. **Additionally**, courier registration MUST reject reserved/`.test`
TLDs at the schema (`auth.ts:34`) as defence-in-depth. With a non-email-derived namespaced hash, the
`DO UPDATE` provably reaches only the synthetic row. If Item 3 is dropped (option a), this is **N/A**.

---

## LOW

### L1 — dev gate PROVEN closed in prod; `body.courierId` impersonates ARBITRARY courier on open staging → **FIX (fold into Item 3(b) constraint)**

Breaker confirms the prod gate holds (no new prod surface). The residual — `body.courierId` signs a token
for *any* UUID on an open (staging) gate — is the same impersonation class as the prior
`dev-login-backdoor` CRITICAL. **Resolution (conditional on Item 3(b)):** `mock-auth` MUST mint a token
**only for the single synthetic seeded courier id** (a server-side constant the seed itself produced),
**never for arbitrary `body.courierId` caller input**. The capability is reduced from "impersonate any
courier" to "impersonate the one synthetic fixture" — un-abusable even if the staging gate leaks. See
ITEM 3 disposition.

### L2 — kill-switch defaults to the risky path → **ACCEPT-RISK (neutralised by C1)**

The Breaker's "default should be the safe degrade" is correct *under the original design where the
estimate could be collected*. After C1, the collected sum is **always** the server-reviewed total
regardless of the replication flag, so the flag now only toggles the *pre-review CTA hint* (exact number
vs `{subtotal}+`) — neither state can mis-collect. The risky property the flag guarded no longer exists.
**Accepted**, default-on, owner: Product. The flag remains a cosmetic fast-kill, which is its honest role
post-C1.

### L3 — seed `ON CONFLICT (order_id) DO UPDATE` re-points an assignment if order_id collides → **FIX (only if Item 3(b))**

Bounded by the seed owning its synthetic order. Conditional on Item 3(b): the seed creates and owns its
order id, so the conflict target is synthetic; documented as accepted-bounded. N/A if Item 3 dropped.

---

## ETHICAL-STOPs (Counsel)

### ETHICAL-STOP-1 — "shown ≠ collected" at the cash door → **FIX (now an explicit acceptance criterion + E2E)**

Counsel asks (friction, not veto): record that the customer sees the **server-authoritative `total`
before any irreversible commitment**, in a real review step, and that this figure equals the courier's
door-collection amount.

**Resolution — recorded as a hard ACCEPTANCE CRITERION (proposal §1.6 + ADR-0005 §Acceptance) and an
E2E, not an implicit assumption:**

> **AC-CASH-PARITY:** Before the customer can place a cash order, the UI displays the
> **server-computed `total`** (live fee math, not the estimate) in a review step. The cash amount,
> `min`, change-due, and door-collection figure are keyed to that server total. When `tip > 0` the review
> shows order-total and tip as separate explicit lines; the collected sum = `total + tip`. The figure the
> customer last reviews == the figure surfaced to the courier on the delivery screen (door-handover
> parity, Counsel §5 / A3).

> **E2E (Playwright, against staging):** drive a cash checkout where the venue fee config differs from the
> hardcoded/estimate path; assert the review step shows the **server** total (not the estimate), assert
> the order persists with `total` == the reviewed figure, and assert the courier delivery screen renders
> the same "collect: {total}" number. A second assertion: a `CASH_AMOUNT_TOO_LOW` 422 produces the
> designed re-prompt with the updated server total, never the generic failure string.

This satisfies Counsel's door-handover-parity (§5) and A3 (show "collect: X" on the courier screen).
The STOP **clears** on the human recording acceptance of this criterion. The criterion is fully satisfiable
by the hardened design.

---

## ITEM 3 — strategic a-vs-b → **HUMAN-NEEDED (Architect recommends (b)-hardened; records (a) as clean)**

Counsel steel-manned NOT building Item 3 and weighted proportionality **above** the proposal, on the
grounds that the `mock-auth` `body.courierId` change is an impersonation expansion of the exact class as
the project's prior `dev-login-backdoor` CRITICAL. This is a strategy/proportionality call, not an ethics
veto, and it is genuinely close — so it is recorded for a **human decision**.

**Option (a) — DROP Item 3.** Leave the live-courier-delivery screen as a documented visual gap; the
not-found state is already captured and verified centered. Zero new dev-seed surface, zero `mock-auth`
broadening, no synthetic courier persisted on staging. Cost: the 390px courier-delivery snapshot renders
a static/mock state, not a live end-to-end render. **This is a clean, defensible choice** and aligns with
"the best code is the code never written."

**Option (b) — BUILD Item 3, hard-constrained** (only if the end-to-end render signal is judged worth it):
- `mock-auth` mints a token **only** for the single synthetic seeded courier id (server-side constant);
  **arbitrary `body.courierId` is removed** (L1). Un-abusable even if the staging gate leaks.
- synthetic `email_hash` is a **namespaced non-email sentinel** `sha256('synthetic:visual-net-courier:v1')`
  that cannot collide with any real courier hash; registration rejects `.test` TLD (M4).
- seed is **idempotent**: `DELETE` the synthetic courier's open shifts before insert; shift `status =
  'available'` to match the proven path (M3); `ON CONFLICT` targets are synthetic-owned (L3).

**Architect recommendation:** prefer **(a)** unless the team explicitly values exercising the real
courier auth + crypto + RLS path in CI over the proportionality cost. The marginal screenshot fidelity is
small; the surface (a permanent synthetic courier + an impersonation primitive, however constrained) is
permanent. If (b) is chosen, it MUST ship with *all four* constraints above — partial (b) is not
acceptable (it re-creates the backdoor shape). **This needs a human to pick a vs b.** Until then, Item 3
is **flag-held**: the FE/courier-screen change does not block Items 1 & 2, which ship independently.

---

## Decision-log notes

- **Scope/naming drift (Counsel §5 secondary):** the branch `fix/design-system-consistency` carries
  two money/state-machine changes (Items 1 & 2) and a test-infra extension (Item 3) — none are
  design-system work. **Recorded** so the health-pass ledger reflects what shipped. Recommendation:
  rename/retarget the money + courier work onto an honestly-named branch (e.g.
  `fix/checkout-fee-and-courier-truth`) on the next push, or split Item 3 onto its own branch given its
  independent (and human-pending) disposition.
- **Counsel A2 (UX honesty)** adopted: the degrade label names the reason — "delivery fee depends on your
  address — confirmed at checkout" — equal visual weight to the price (proposal §1.7).
- **Counsel A4 (dev-seed hygiene)** adopted under Item 3(b): synthetic courier keeps the unmistakable
  "Visual Net Courier" display name; add a documented note so a staging owner-walkthrough cannot mistake
  it for a real courier (prevents Item 3 from re-introducing Item 2's dishonesty).

---

## Disposition summary

| Finding | Sev | Disposition | One-line |
|---|---|---|---|
| C1 | CRIT | **fix** | Cash keyed to server total via a before-commit review step; FE handles `CASH_AMOUNT_TOO_LOW`; tip shown separately |
| C2 | CRIT | **fix** | Share `applyTax`/`computeLineTotal` via `@deliveryos/domain` (Counsel A1) — one impl, not a mirror; parity-test is the permanent fallback |
| H1 | HIGH | **fix** | FE min-gate matches server (pickup included); subtotal computed modifier-inclusive via shared `computeLineTotal` |
| H2 | HIGH | **fix** | Tip excluded from `total`, shown as a separate collected line; same integer subtotal shown == compared |
| H3 | HIGH | **fix** | Pin: `applyTax` works in stored minor-unit integers, no coarser rounding; the shared fn IS the contract |
| H4 | HIGH | **fix** | Drop unprovable "Pending setup"; account-status-only display; stamp `last_login_at` at invite-redeem + refresh (follow-up) |
| M1 | MED | **accept-risk** (Product) | Stale estimate is corrected at the server-total review step; never the collected sum |
| M2 | MED | **fix** | Precomputed `has_distance_tiers` from public `locations`; unknown → degrade (fail-safe) |
| M3 | MED | **fix** (if Item 3(b)) | DELETE synthetic shifts before insert; shift `status='available'`; else N/A |
| M4 | MED | **fix** (if Item 3(b)) | Namespaced non-email `email_hash`; reject `.test` TLD on registration; else N/A |
| L1 | LOW | **fix** (if Item 3(b)) | `mock-auth` mints only the synthetic courier id; arbitrary `body.courierId` removed |
| L2 | LOW | **accept-risk** (Product) | Flag now only toggles a cosmetic CTA hint; neither state can mis-collect post-C1 |
| L3 | LOW | **fix** (if Item 3(b)) | Seed owns its synthetic order id; ON CONFLICT target is synthetic |
| ETHICAL-STOP-1 | — | **fix** | Explicit AC-CASH-PARITY acceptance criterion + door-handover-parity E2E |
| ITEM 3 (a vs b) | — | **human-needed** | Architect recommends (a) drop, or (b) only with all 4 hard constraints; human picks |

## Still needs a human decision

1. **ITEM 3 — (a) drop vs (b) build-hardened.** Architect recommends (a) on proportionality; (b) is
   acceptable ONLY with all four constraints (synthetic-only mint, namespaced hash + `.test` reject,
   idempotent seed, synthetic-owned conflicts). Items 1 & 2 ship regardless.
2. **ETHICAL-STOP-1 sign-off.** Record human acceptance of AC-CASH-PARITY (server total reviewed before
   commit == door-collected sum). Design satisfies it; the STOP needs the recorded human "proceed".
3. **Branch naming/scope.** Approve renaming/splitting `fix/design-system-consistency` so the ledger
   reflects money + courier-truth + test-infra, not design-system work.

---

# RESOLVE round 2 — RE-ATTACK + RE-EXAMINE (post-revision regression pass)

Architect resolving the NEW findings the Breaker raised against the REVISED design
(`breaker-findings.md` §"RE-ATTACK round 2") and the carried-forward non-blocking items in
`counsel-opinion.md` §"RE-EXAMINE round 2". Each NEW finding → **fix** / **accept-risk+owner** /
**defer-flag**. Source re-verified before resolving — the load-bearing claims reproduce:
`total` computed only inside the create txn (`orders.ts:565` → INSERT `:596`); the public menu carries
`modifier_groups[].price_delta` (`MenuPage.tsx:19,458-466`); cart line price folds modifiers at add-time
(`MenuPage.tsx:510`); `reconcileCart` skips modifier-bearing lines (`cartReconcile.ts:30`);
`courier/auth.ts:34` + `auth/local.ts:41` + `public/access-requests.ts:56` all use `z.string().email()`
with no TLD restriction; `owner/couriers.ts:34` is the list WHERE clause.

## NEW-C1 (CRITICAL) — "review server total before commit" needs an endpoint that doesn't exist → **FIX (by computability — no endpoint built)**

The Breaker is correct that `total` is computed only inside the create transaction (`orders.ts:565`) and
that `evaluatePreflight` rolls back before the fee math, so there is no price-quote endpoint. But the
Breaker under-weighted the C2 fix's structural consequence, which dissolves the chicken-and-egg **without**
a new endpoint:

**Decision — split by computability, pinned explicitly (proposal §1.4 NEW-C1, ADR-0005 Decision pt.5,
§1.6 AC-CASH-PARITY):**

- **Computable venues** (`has_distance_tiers === false` AND `deliveryFeeFlat != null` — flat fee +
  `free_delivery_threshold` + `tax_rate`/mode, ALL on `/info`; covers the demo + the common case): the
  client computes `reviewTotal` via the **shared `@deliveryos/domain` `computeOrderTotal`** — the *same
  function* `orders.ts` runs, over the *same* public inputs. By construction `reviewTotal == server total`
  (proven by the §1.6 parity guardrail across the boundary matrix). This **IS** the server-authoritative
  total; cash `min`/red-border/change-due/door figure are keyed to it. **This is NOT a second copy** — it
  is the one imported module; **NO endpoint, NO duplicate fee math.** The C2 "share don't mirror" fix is
  exactly what makes the client number authoritative rather than an estimate.
- **Distance-tiered venues** (`has_distance_tiers === true` OR unknown — minority; RLS hides
  `delivery_tiers` from the public role so the client genuinely cannot compute the fee): we pick **option
  (ii)** — the client shows **no exact cash pre-commit figure** (`Porosit • {subtotal}+`); the order
  submits; the server computes the real `total`; the **courier delivery screen surfaces it as "collect: X"**
  and that is what the courier collects at the door. The customer is never asked to pre-commit an exact cash
  number the design cannot authoritatively back. Door-handover parity holds (the collected figure is the
  server total; no smaller customer-shown figure exists to undercut it). Option (i) — a real read-only
  `/orders/quote` endpoint calling the SAME shared module (not a duplicate) — is recorded as a **deferred,
  named scope addition** for when tiered venues become common; **not built now (YAGNI)**.

This closes AC-CASH-PARITY/ETHICAL-STOP-1 as actually-satisfiable: the original non-goal "no new endpoint"
holds because computability — not a quote round-trip — supplies the authoritative number for every venue
the launch trigger needs, and tiered venues route door-truth through the courier screen instead of a fake
customer pre-quote. Owner: Product/Architect.

## NEW-H1 (HIGH) — reviewed vs committed total are unsynchronised reads (30s `/info` cache + MVCC) → **ACCEPT-RISK (422 cash-handler backstop is the named binding)**

Even for a computable venue, the `/info` config (≤30s cached, ≤1h stale-on-error) and the live
create-txn `locations` read are two snapshots; an owner fee change between them makes
`reviewTotal != server total`. **Disposition: accept-risk, bound by the AC-CASH-422 re-prompt — stated
exactly:** the server `cashPayWith < total` 422 (`orders.ts:568`) catches the under-quote direction; the
FE re-prompt (AC-CASH-422) re-shows the *new* server total and re-blocks submit until cash ≥ it, so a
customer can never commit cash below the live charge. The over-quote direction (config dropped) is
self-correcting — the customer's cash already covers the lower total. We deliberately do **not** add a
`request_hash`-bound quote-lock: it requires the unbuilt `/orders/quote` endpoint + a hold/expire
mechanism to close a ≤30s window the 422 already makes safe-by-direction. A `checkout_fee_divergence`
counter surfaces the window in <1 min. Owner: Product. (proposal §1.4 NEW-H1, ADR-0005 accepted-risk.)

## NEW-H2 (HIGH) — modifier deltas stale FE-side → **ACCEPT-RISK (public menu DOES carry modifier prices; recompute from current menu)**

The Breaker's premise ("FE cannot hold per-modifier prices") is **false** — verified: the public menu
serves `modifier_groups[].modifiers[].price_delta` (`MenuPage.tsx:19,458-466`) and the cart stores
`item.price = base + Σ deltas` at add-time (`MenuPage.tsx:510`). The genuine residual is narrow: for a
**modifier-bearing** line whose `price_delta` drifted after add-time, `reconcileCart` skips repricing
(`cartReconcile.ts:30`), so the stored line price is stale. **Fix:** checkout recomputes `subtotal`
modifier-inclusive via the **shared `computeLineTotal` over the freshly-loaded menu** `price_delta`s — the
same function + same current inputs the server reads (`orders.ts:485,506`) — so the min-gate and
free-threshold operands match the server. **Accepted residual:** a `price_delta` changing within the ≤30s
menu-read window can still differ by that delta, caught by the same server 422 + AC-CASH-422 backstop as
NEW-H1, never silently mis-collected. Owner: Product. (proposal §1.4 H1/NEW-H2, ADR-0005 accepted-risk.)

## NEW-H3 (HIGH, human ship-blocker) — `.test`-TLD rejection must be ACTUALLY IMPLEMENTED → **FIX (concrete, listed code change — definitive)**

**Definitive ruling: the namespaced sentinel hash and the `.test` reject close DIFFERENT threats; the
`.test` reject is a real, mandatory, listed change, NOT optional and NOT "defence-in-depth".**

- The sentinel hash `sha256('synthetic:visual-net-courier:v1')` alone fully closes the M4 *resurrection*
  vector (it can never be produced by any `z.string().email()` input, so the seed's `ON CONFLICT DO UPDATE`
  provably reaches only the synthetic row — no `.test` email is relevant to seed safety).
- BUT the human elevated `.test`/reserved-TLD rejection to an **independent ship-blocker** because it is
  registration-namespace hygiene (non-routable `*.test`/`*.localhost`/`*.invalid`/`*.example` addresses
  pass `z.string().email()` vacuously) — a threat the sentinel hash does **not** touch. So it must be BUILT.

**Concrete listed change (proposal §3.4 constraint #2):** a shared Zod refinement `rejectReservedTld`
(rejecting RFC-2606 `{.test, .example, .invalid, .localhost}` with 400) applied at **every** registration/
auth email parse:
- `apps/api/src/routes/courier/auth.ts:34` (courier invite-redeem),
- `apps/api/src/routes/auth/local.ts:41` (owner register/login),
- `apps/api/src/routes/public/access-requests.ts:56` (access-request email).
Proof obligation: a red→green test asserting each endpoint 400s on a `@x.test` email. The proposal's prior
"defence-in-depth" framing is corrected to "ship-blocker, separate threat." Owner: API agent.

## NEW-M1 (MED) — synthetic-only mint must RE-DERIVE the id, never echo caller input → **FIX (pin re-derive; echo-back REMOVED)**

The echo-back variant (seed returns id → harness sends it back → server equality-checks) is **removed** —
it re-introduces a caller-supplied `courierId` + a guard, the exact `dev-login-backdoor` shape, one typo
from `if (body.courierId) sub = body.courierId`. **Pinned: `/dev/mock-auth` `role:'courier'` re-derives the
synthetic id server-side** by hashing the same sentinel and `SELECT id FROM couriers WHERE email_hash = $1`
— it reads NO caller-supplied id. Constraint #1 ("never arbitrary caller input") holds by construction, not
by a guard (proposal §3.4 NEW-M1, code shown). Owner: API agent.

## NEW-M2 / Counsel A4-Q2 (MED) — persistent synthetic `active` courier pollutes Item-2's "N active" count → **FIX (structural filter, not just a display name)**

Counsel correctly noted the round-1 resolution closed A4 "for humans only" (display marker), not
programmatically — the synthetic `status='active'` row is still counted in Item-2's honest "N active" on
the shared staging DB. **Fix:** filter the synthetic row out of the owner couriers query by its sentinel
`email_hash` — `apps/api/src/routes/owner/couriers.ts:34` WHERE clause gains
`AND c.email_hash <> SYNTHETIC_COURIER_EMAIL_HASH` (shared constant). This removes it from BOTH the list
AND the `count(status==='active')`; on prod it is a no-op (the row never exists behind the dark gate). The
"Visual Net Courier" display name is kept as belt-and-suspenders. No `is_synthetic` column / migration
needed — the existing UNIQUE `email_hash` already identifies the row. **Not claimed "closed" by name only —
it is closed by the count/list filter.** (proposal §3.4 pt.5, §3.10 R6, ADR-0006 cross-item guard.) Owner:
API agent.

## Counsel RE-EXAMINE carried-forward (non-blocking) — dispositions

- **Q1 (review step must be read-only/neutral):** satisfied by construction — there is no new mandatory
  server "preflight" screen (NEW-C1 resolved by computability), so no new pre-commit side-effect and no new
  dark-pattern surface. The computable-venue review is a local computation; no pre-filled/pre-committed
  cash value, no nudged default. **Acknowledged, no action needed.**
- **Q2 (synthetic-active honesty seam):** closed structurally by NEW-M2 above (the count/list filter), not
  just accepted. Counsel asked for "explicit accepted-risk OR a one-line filter" — we took the filter.
- **Q3 (`last_login_at` stamping touches the auth red-line for a display feature):** **defer-flag,
  unchanged.** It remains a deferred Option-B prerequisite (ADR-0006), explicitly NOT gating this PR and
  NOT stamped until Option B ships (don't write a column nothing reads). When it lands it gets its own
  care-pass (no write-amplification / timing-signal on refresh-rotation). Owner: API agent. Recorded.
- **A1 (shared money math):** already adopted in round 1 (C2 → `@deliveryos/domain`); NEW-C1's resolution
  *depends* on it (computability = the shared fn). Confirmed.

## Round-2 disposition summary

| Finding | Sev | Disposition | One-line |
|---|---|---|---|
| NEW-C1 | CRIT | **fix** (computability) | Computable venues: client `computeOrderTotal` (shared fn, public inputs) IS the server total — no endpoint, no copy. Tiered: no exact cash pre-quote; courier "collect: X" is the door figure. `/orders/quote` deferred (YAGNI). |
| NEW-H1 | HIGH | **accept-risk** (Product) | reviewTotal vs committed are 2 reads; bound by the server 422 + AC-CASH-422 re-prompt (safe-by-direction); no quote-lock built |
| NEW-H2 | HIGH | **accept-risk** (Product) | Public menu carries modifier prices; recompute subtotal via shared `computeLineTotal` over current menu; ≤30s drift caught by the same 422 backstop |
| NEW-H3 | HIGH | **fix** (ship-blocker) | `.test`/reserved-TLD reject is a real shared Zod refinement at 3 listed endpoints; sentinel hash closes a DIFFERENT (resurrection) threat — not a substitute |
| NEW-M1 | MED | **fix** | mock-auth RE-DERIVES synthetic id (SELECT by sentinel hash); echo-back variant removed |
| NEW-M2 | MED | **fix** | owner/couriers.ts:34 WHERE `email_hash <> sentinel` excludes synthetic from list AND "N active" count; prod no-op |
| NEW-L1 | LOW | **accept-risk** (Product) | dormant `onlineStatus`/mockData coexistence — over-count acceptable per R8; no new hole (Breaker concurs) |
| NEW-L2 | LOW | **accept-risk** (conductor) | seed DELETE is synthetic-scoped (no real-row leak); residual is concurrent-run non-idempotency only — single-shot per capture in practice |
| Q1 | — | acknowledged | review step is read-only by construction (no preflight screen) |
| Q2 | — | **fix** | closed by NEW-M2 filter (not name-only) |
| Q3 | — | **defer-flag** | `last_login_at` stamping stays deferred to Option B + own care-pass |

## Still needs a human decision (round 2 — unchanged from round 1)

1. **ITEM 3 — (a) drop vs (b) build-hardened.** Now FIVE mandatory constraints if (b): synthetic-only
   **re-derive** mint (NEW-M1), namespaced sentinel hash + **real `.test` reject at 3 endpoints** (NEW-H3),
   idempotent seed (M3), synthetic-owned conflicts (L3), and **owner-list/count exclusion filter**
   (NEW-M2). Partial (b) is NO-GO (recreates the backdoor shape and/or re-pollutes the honest count). Items
   1 & 2 ship regardless of this decision.
2. **ETHICAL-STOP-1 sign-off.** Counsel RE-EXAMINE confirms the STOP is cleared and the hardened design
   *holds* it; the human "proceed" is recorded. NEW-C1's computability resolution makes AC-CASH-PARITY
   actually-satisfiable without the phantom endpoint — no re-open.
3. **Branch naming/scope.** Unchanged — rename/split `fix/design-system-consistency`.

**Nothing in round 2 re-opens an ETHICAL-STOP.** The one CRITICAL (NEW-C1) is resolved by the structural
consequence of the already-accepted C2 share (computability), not by building forbidden scope.

---

# RESOLVE round 3 (exit check) — the one open HIGH

Architect resolving the single NEW HIGH the Breaker raised in `breaker-findings.md`
§"RE-ATTACK round 3 (exit check)". Source re-verified before resolving — **every** load-bearing Breaker
claim reproduces against live source:

- `@deliveryos/domain` exports only `./order-machine.js` + `./errors.js` (`packages/domain/src/index.ts`);
  **no money module exists.**
- `applyTax` / `computeLineTotal` / `assertNonNegative` live in `apps/api/src/lib/money.ts:23/46/54`
  (an api-internal file). The file is pure — no `pg`, no node `crypto`, no Node-only deps — so it is
  isomorphic-extractable (the C2 bundle question stays answered: safe).
- The server computes the total **INLINE**, not via a callable function: `orders.ts:560-565`
  (`taxTotal = applyTax(...); discountTotal = 0; total = subtotal + deliveryFee + taxTotal - discountTotal`),
  with the fee-selection ladder (min-order 422 `:519`, free-threshold `:530`, **live `delivery_tiers`
  query `:533`**, flat `:553`) **interleaved with `client.query('ROLLBACK')` side-effects inside the
  create transaction.** There is no pure `computeOrderTotal` the create path calls.
- `grep computeOrderTotal apps packages` → **zero hits.** The named shared artifact is unwritten.
- `discountTotal = 0` is a hardcoded literal (`orders.ts:564`); no promo/coupon/discount path exists today.

## NEW-3-H1 (HIGH) — "shared `computeOrderTotal`, client==server by construction" presumes a fn + a refactored server path that do not exist → **FIX, by committing to Approach M (mirror-with-hard-gate). The proposal/ADR now OWN this explicitly; the money create-path is NOT touched.**

**The finding is correct and I own it without hedging.** The round-2 NEW-C1 resolution leaned the entire
"client number IS the server total **by construction**" guarantee on a `computeOrderTotal` that does not
exist and a server create-path that calls it — which is false in code. Making "by construction" literally
true requires extracting the money fns to `@deliveryos/domain` AND authoring a pure `computeOrderTotal`
AND **refactoring `orders.ts:518-565` — the authoritative money create-path — to call it.** That last step
is a 🔴 money red-line the proposal's §1.1 non-goals explicitly disclaim ("Not changing the server fee
math"). The plan tacitly required a money-path refactor and called it free. It is not free.

I considered both dispositions the round demanded I choose between:

- **Approach R (refactor-to-share):** extract the fns + author `computeOrderTotal` + refactor the live
  create-path to call it; parity holds "by construction." Cleaner long-term single source. **Cost:** it
  touches the authoritative money create-path — the highest-consequence red-line in the repo
  (`docs/regressions/` red-line globs include money + `orders.ts`-class paths), on a path the project's own
  history (`dev-login-backdoor`, the cash-422 finding) shows is unforgiving. To be safe it must be
  *behaviour-preserving* (a characterization test pinning server `total` before==after across the boundary
  matrix), and the fee-selection ladder is **interleaved with `ROLLBACK`/`INSERT` side-effects** (the tier
  branch does a `client.query` mid-ladder and the min-order/out-of-range branches `ROLLBACK`+return) — so
  the extraction is not a clean lift; it requires separating the *pure fee arithmetic* from the *impure
  control-flow* (the 422 returns, the live tier read), which is exactly where a behaviour-preserving
  refactor is most likely to drift a `ROLLBACK` or a 422 boundary by accident.

- **Approach M (mirror-with-hard-gate):** leave `orders.ts` money math **EXACTLY as-is** — zero change to
  the charged amount. The client uses a separate small pure `estimateOrderTotal` (in `@deliveryos/domain`)
  that **mirrors** the server formula. The single source of truth for what is CHARGED stays the untouched
  server create-path. Safety is **not** "by construction" — it is enforced by two deterministic gates:
  (1) a **permanent parity guardrail** that, over a matrix of inputs, computes the **REAL server total**
  (via a characterization fixture that exercises the actual `orders.ts` fee arithmetic) and asserts
  `client estimateOrderTotal == server total`, red→green, ship-blocker; and (2) the runtime **cash-422
  backstop** + AC-CASH-422 re-prompt. The owned residual: a hand-maintained mirror can drift — mitigated by
  the parity gate failing CI on any drift, so drift cannot reach prod silently.

### DECISION — Approach M (mirror-with-hard-gate). Recommended and committed.

**Why M over R, given this is a money red-line and the project's standing caution:**

1. **M changes ZERO charged-amount code.** The authoritative server create-path (`orders.ts:518-565`,
   including the cash-422 gate `:568`) is byte-for-byte untouched. For a 🔴 money red-line, "do not touch
   the path that decides what a human is charged" is the strongest possible safety posture. R, however
   carefully characterized, *does* touch it — and the failure mode of a botched behaviour-preserving
   refactor of a tax/fee ladder is a silent mis-charge, the single worst outcome this whole design exists
   to prevent. The asymmetry is decisive: M's worst case is a *displayed* number drifting (caught by the
   gate before merge, and by the 422 at runtime); R's worst case is the *charged* number drifting.

2. **M still delivers the user-visible guarantee in full.** "Displayed == charged" for the customer does
   NOT require the server to call the same function — it requires the displayed number to **equal** the
   charged number. The parity guardrail proves that equality over the boundary matrix (the same property R
   gets "by construction"), and the cash-422 + re-prompt makes any residual stale-window drift
   safe-by-direction at runtime. The customer cannot tell whether equality came from a shared import or a
   gated mirror; only the codebase can, and the gate makes the difference auditable.

3. **R's "single source" benefit is real but deferred-able and currently low-value.** Today there is
   exactly ONE divergent-risk input: `discountTotal`, hardcoded `0`. The fee ladder is otherwise a pure
   function of `/info` public inputs + the cart subtotal. So the drift surface a shared import would
   eliminate is small *today*. R is the right end-state **when** the server money math grows (nonzero
   discounts, promo, multi-line fees) — at which point the extraction earns its red-line risk. Per
   `/ponytail` / YAGNI: do not take a live money-path refactor's risk now to eliminate a near-empty drift
   surface; take it when the surface is real. **Approach R is recorded as the deferred end-state**
   (named, owned), to be done as its own behaviour-preserving, characterization-gated change — never
   bundled into this FE-display PR.

**What ships under M (concrete, replacing the round-2 "computeOrderTotal by construction" language):**

- Extract `applyTax`, `computeLineTotal`, `assertNonNegative` **verbatim** from `apps/api/src/lib/money.ts`
  into a new `@deliveryos/domain` money module (`packages/domain/src/money.ts`, re-exported from
  `index.ts`). `apps/api` re-imports them from the domain pkg (a pure move, no behaviour change — the
  characterization step below pins it). This is the C2 "share, don't mirror" extraction for the *building
  blocks*, and it is safe because those three fns have **no impure interleaving** (they are already pure).
- Author a new pure `estimateOrderTotal(input)` in `@deliveryos/domain` that mirrors the server's
  **fee-selection arithmetic for the computable case only** — flat-fee + free-threshold + tax + integer
  modifier-inclusive subtotal. It deliberately does NOT model the distance-tier branch (the client cannot
  read `delivery_tiers` — M2) nor the 422 control-flow (those stay server-only). For tiered/unknown venues
  the client suppresses the exact cash figure (the round-2 tiered path, unchanged).
- The server create-path is **NOT** refactored to call `estimateOrderTotal`. `orders.ts:518-565` stays
  inline and authoritative. The function name is `estimateOrderTotal` (not `computeOrderTotal`) precisely
  so no reader infers a non-existent "shared by both sides" property — the name states it is the client
  mirror, and the gate is what guarantees it equals the server.

**Mandatory ship-blockers (Mandatory Proof Rule — unchanged in force, sharpened in target):**

1. **Parity guardrail (red→green, permanent, never-deletable).** A test that, for the matrix
   {subtotal ∈ [0, min−1, min, threshold−1, threshold, threshold+1]} × {fee flat values} ×
   {tax 0 / **included (back-out)** / excluded} × {`minor_unit` 0 and non-zero}, computes the **REAL
   server total** — by exercising the actual `orders.ts` fee arithmetic via a characterization fixture
   (extract the pure arithmetic the create-path runs, or drive the create endpoint and read back `total`)
   — and asserts `estimateOrderTotal(sameInputs) === serverTotal`. It must be proven **red** (mutate the
   client mirror by 1 minor unit → fails) then **green**. A `docs/regressions/REGRESSION-LEDGER.md` row
   records it. This is the gate that converts "hand-maintained mirror" from a drift hazard into a
   CI-blocked invariant — it is the load-bearing artifact of Approach M.
2. **Door-handover-parity E2E (Playwright, staging):** cash checkout at a venue whose fee config ≠ the old
   hardcoded path → assert the review shows the server total, the order persists with `total` == the
   reviewed figure, and the courier delivery screen renders the same "collect: {total}".
3. **Cash-422 E2E:** a forced `CASH_AMOUNT_TOO_LOW` 422 → assert the designed re-prompt with the updated
   server total, never the generic failure string.

**Owned residual risk (accept-risk):** the mirror is hand-maintained; if a future server money change lands
without updating `estimateOrderTotal`, the two drift. **Mitigation (deterministic, not advisory):** the
parity guardrail fails CI on any drift over the matrix — drift cannot reach prod silently; it is a build
break. The narrow window the gate does NOT cover (an input value outside the matrix boundaries) is bounded
by the runtime cash-422 backstop (safe-by-direction: under-quote → 422 → re-prompt). **When the server
money math grows beyond the current pure-flat-fee shape (e.g. nonzero `discountTotal`), the disposition
escalates to Approach R** (extract the create-path arithmetic into the shared fn so both sides move
together) — recorded as the named deferred end-state. **Owner: Product/eng** (mirror maintenance + the
escalation trigger); **API agent** (the domain extraction + characterization fixture).

§1.1 non-goals are corrected to OWN this honestly: this PR DOES add a `@deliveryos/domain` money module and
a client `estimateOrderTotal`, and DOES re-point `apps/api`'s money-fn imports to the domain pkg (a pure,
characterization-pinned move) — but it does **NOT** change the server fee-selection / charge arithmetic
(`orders.ts:518-565` stays inline and authoritative). The red-line is held: zero change to what is charged.

### Minor note (Breaker round-3, registration email-parse coverage) — **resolved: sentinel hash makes `.test` non-load-bearing for SEED safety; the 3-endpoint reject is independent hygiene and must cover EVERY registration email parse.**

The Breaker flagged that the resolution names `auth/local.ts:41` as "owner register/login" but that file
holds only `/auth/local/login` (one email parse) — if owner *registration* parses email elsewhere, the
3-endpoint list is incomplete. Disposition, two parts:

1. **For M4 / seed safety, `.test` rejection is NOT load-bearing.** Re-confirmed: the seed
   `email_hash = sha256('synthetic:visual-net-courier:v1')` is a namespaced **non-email** sentinel that
   **no `z.string().email()` input can ever produce**, so `ON CONFLICT (email_hash) DO UPDATE` provably
   reaches only the synthetic row regardless of whether any endpoint rejects `.test`. The resurrection
   vector is closed by the hash **alone**. Therefore partial `.test` coverage does **not** weaken seed
   safety — `.test` rejection is belt-and-suspenders for M4 specifically.
2. **As an independent registration-namespace-hygiene ship-blocker (NEW-H3), the reject MUST cover EVERY
   email-parse entry point, not a hardcoded list of three.** The resolution is sharpened: the
   `rejectReservedTld` Zod refinement is applied to **every** registration/auth/access-request email parse
   across the codebase — the implementer MUST `grep` for all `z.string().email(` call sites and apply the
   refinement to each, then prove each 400s on a `@x.test` email (red→green). The three currently named
   (`courier/auth.ts:34`, `auth/local.ts:41`, `public/access-requests.ts:56`) are the verified-present
   set; if owner registration parses email at a fourth site, it is in scope by this rule. The proof
   obligation is "no `z.string().email(` without `rejectReservedTld`", enforceable as a lint/grep guard,
   not a fixed endpoint count. Owner: API agent.

This makes the `.test` constraint's coverage complete-by-construction (every parse site) while explicitly
recording that its *absence* on any single site cannot compromise the seed (the sentinel hash already
does that job).

## Round-3 disposition summary

| Finding | Sev | Disposition | One-line |
|---|---|---|---|
| NEW-3-H1 | HIGH | **fix** (Approach M) | Commit to mirror-with-hard-gate: client `estimateOrderTotal` (domain pkg) mirrors the server; `orders.ts:518-565` charge path UNTOUCHED; parity guardrail (real server total == client mirror, red→green, permanent) + cash-422 backstop are the ship-blockers; Approach R (server refactor) recorded as the deferred end-state for when server money math grows. §1.1 non-goals corrected to own the (pure) domain extraction. Owner: Product/eng + API agent. |
| `.test` coverage note | — | **fix** | Sentinel hash makes `.test` reject non-load-bearing for SEED safety (partial coverage OK there); as independent hygiene, the reject MUST cover EVERY `z.string().email(` site (grep-enforced), not the 3 named. Owner: API agent. |

## Exit status after round 3

**ALL CRITICAL/HIGH are resolved (fixed or accept-risk + owner). HARD-EXIT.**

- CRITICAL: C1 (**fix**), C2 (**fix**), NEW-C1 (**fix** — now via Approach M, not "by construction").
- HIGH: H1, H2, H3, H4 (**fix**); NEW-H1, NEW-H2 (**accept-risk**, Product, cash-422 backstop);
  NEW-H3 (**fix**, every-site coverage); **NEW-3-H1 (**fix**, Approach M — this round).**
- MED/LOW: dispositioned in rounds 1–2 (fix / accept-risk+owner), Item-3 conditionals gated on the
  human a-vs-b call.
- Open HUMAN-NEEDED (carried, NOT a Breaker CRITICAL/HIGH): Item 3 (a) drop vs (b) build-hardened;
  ETHICAL-STOP-1 sign-off (design satisfies it); branch rename/split. Items 1 & 2 ship regardless.

No finding remains unowned. The mandatory ship-blockers (parity guardrail red→green + door-handover-parity
E2E + cash-422 E2E) gate the launch per the Mandatory Proof Rule.
</content>
</invoke>
