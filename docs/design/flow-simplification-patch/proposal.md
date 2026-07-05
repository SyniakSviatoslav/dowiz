# Flow Simplification Patch — Architect Proposal

**Slug:** `flow-simplification-patch` · **Status:** Proposed → **RESOLVE-hardened** (design-only, no production code)
**Seat:** Architect (Triadic Council) · **Date:** 2026-06-28 · grounded against live source.

> **RESOLVE round (2026-06-28):** Breaker F1–F9 + Counsel §1–§10 dispositioned in `resolution.md`. Both
> HIGHs changed the design (F1 gated dispatch + IN_DELIVERY recovery §5; F2 claim-token transport §6); §2
> reversed to keep the running subtotal (Counsel cash-as-proof steel-man); §3 R3 decided
> (optional-but-inviting); F3/F4/F5 add a timestamp-decoupled customer signal; F6/F9 close the checkout
> seams. Sections below carry inline **RESOLVE /** blocks.
>
> **RESOLVE round 2 (2026-06-28):** the round-1 F1/F2 fixes were grounded on a **fictional dispatch
> primitive** and a **falsely-assumed-absent recipient check** — both re-grounded against live source
> (`resolution.md` → "RESOLVE round 2"). F1 → make the **real** endpoint (`orders.ts:785`, the only one the
> card reaches) honest, decoupled from the handshake flag (4-cell matrix); IN_DELIVERY recovery = **revert-to-
> READY** only (re-assign 409s). F2 → **in-page fetch-auth** (no navigation = no token loss) + the **real**
> `invited_contact_hash`/`CONTACT_MISMATCH` recipient binding. F4 auto-stamp **dropped** (ETA decays off
> `confirmed_at`; "Preparing" is a process label). §3 is now a **three-way** NEEDS-HUMAN
> (contextually-required recommended). PROTECTED FRICTION → code-level G-PF1/G-PF2. **Converged**, one
> NEEDS-HUMAN remaining.
>
> **RESOLVE round 3 (2026-06-28):** re-attacked the round-2 fixes against the **binding lifecycle** + the
> **recipient identity** (`resolution.md` → "RESOLVE round 3"). Three round-2 overstatements **corrected**:
> (R3-1) the `invited_contact_hash` check is a **string match on an unverified email**, not "proof of control"
> — the web path now **refuses token-only (NULL-hash) invites** (G-F2g), and the email-ownership gap is
> **DEFER-FLAG to P6/auth**; (R3-4) the IN_DELIVERY recovery is **not** a raw revert-to-READY (the central fold
> blanket-cancels a `picked_up` binding → food-out lie) but **reuses the shipped `releaseBindingAndReoffer`**
> asymmetry (pre-pickup→READY, picked_up→CANCELLED, G-F1b-i/ii); (R3-5) the ETA decay base is **unplumbed
> `confirmed_at`** → corrected to `COALESCE(preparing_at, confirmed_at, created_at)`. Plus R3-2 the §5 dispatch
> is made **`'offered'`-aware** (already-bound guard, G-F1a-2 — else a pending-offer order 500s on the
> mig-073 unique) and R3-3 the awaiting-courier **re-dispatch path = owner re-tap** (G-F1c; async auto-pickup
> DEFER-FLAGged with the `courier-dispatch.ts:76 this.boss` bug). **Converged**, one NEEDS-HUMAN remaining.

**Guiding principle:** remove structure built around choices that no longer exist
(single payment = cash, single order-type = delivery, deferred self-serve), and collapse
page-sequences into ONE surface where the transition adds nothing.

> **Honesty up front (the highest-value architect output here):** two of the six changes are
> *almost entirely already built*. §4 (contract) needs **one additive Zod default**, not a migration.
> §6 (claim) is a **surface/activation delta over the SHIPPED P6 vertical** — the ownership-transfer
> backend (`acceptClaim`/`claim_transfer` SECURITY-DEFINER, migration `…071_claim-invites`,
> `routes/public/claim.ts`) exists and has **zero web-UI callers**. Most of this patch is UI removal
> on a foundation whose seams are already cut. The patch also contains **one contradiction with the
> shipped+council-approved P6 model** (§6 "one action … goes live") that must be revised, not built.

---

## 0. Cross-cutting back-of-envelope

This patch changes **interaction structure**, not load. No new fan-out, no new external calls, no new
tables. Load implications are neutral-to-positive (fewer route transitions = fewer SSR/SPA mounts).

**Action-count deltas (patch §8 framing), grounded in the real surfaces:**

| Actor / path | Today (grounded) | After patch | Δ |
|---|---|---|---|
| Customer — new (first order) | menu → cart-bar → **cart drawer** → **checkout page** → fill name+phone+map+entrance+apt+notes → confirm (~7–9 taps + route hop) | menu → cart-bar → **checkout panel** (cart collapsed) → phone + map-pin + confirm (~5) | **≈ -3** |
| Customer — repeat (saved device) | same surfaces; fields pre-filled from `dos_checkout_draft_*` + `dos_last_delivery_*` | cart-bar → checkout panel → confirm (~1–2) | **≈ -2** |
| Owner — order to courier | Accept → Mark Preparing → Mark Ready → Assign (**4 taps**, `OrderCard.tsx:224–235`) | Accept → Send for delivery (**2 taps**; dispatch attempt, not a raw status flip — see §5/F1) | **-2** |
| Owner — onboarding | self-serve: upload menu → parse → create → activate → publish | **claim** a pre-built working service → (light-edit) → review → publish | qualitative: operator pre-does the build |
| **Operator** — per provisioned shadow (RESOLVE: Counsel §6 — on the ledger, not in a footnote) | n/a (no operator step today) | scrape source → AI build menu/theme/radius → assemble branded shadow → send hostile-recipient Art-14 claim notice | **+N (concentrated 3rd-party cost)** |

> **RESOLVE / F7 (action-count honesty):** "≈5 / ≈1–2" is a **target with a stated dependency**, not a
> settled count. It holds **only** under the §3 floor decision (phone + a droppable location required;
> entrance/apt/notes optional) and **excludes** conditional E27 preflight-acknowledge taps
> (`legacy.ts:70–71`), which fire only on distance-tier / far-address orders and add **+1 render+tap** when
> they do. The operator row above is the deliberate first-wave "do things that don't scale" bootstrap — real
> simplification for customer (−3) and owner (−2) **because** complexity concentrates on a third party.

**Contract/load implications:** none new. The only contract touch is §4 (additive default). The order
hot-path (`POST /orders`, the bounded 4.5s write-hold, idempotency, cash-422 backstop) is **untouched**.
Connection budget (API max:8 + worker + analytics + migrations) is unchanged.

**Information floor (patch §3) is a hard architectural constraint, not a nicety:** delivery physically
needs *where* and *who to call*. The floor is **phone + a droppable location once**. Anything that
deletes a field below this floor (e.g. "address on the menu", per-item buy-now) trades a real failure
(courier can't find / can't call) for a saved tap. **Rejected, recorded, not to be revisited:**
address-on-menu; per-item "buy now" bypassing the cart.

---

## §1 — Checkout 3 surfaces → 2 (checkout as a panel over the menu)

### Current state (grounded)
The patch says "remove the intermediate cart **page**." **There is no cart page.** The real topology:

1. **Menu** — `/s/:slug` index route (`ClientRoutes.tsx:12`, `MenuPage`).
2. **Cart drawer** — a `ResponsiveDialog` **bottom sheet** rendered inside `ClientLayout`
   (`ClientLayout.tsx:159–223`), opened by the sticky cart bar (`data-testid=cart-open`,
   `ClientLayout.tsx:131–158`). It slides up `translateY(100%)→0` (`ResponsiveDialog.tsx:78`).
   It already holds item rows + qty steppers + total + a "Checkout" button
   (`data-testid=cart-checkout`, `ClientLayout.tsx:207`) that **navigates** to…
3. **Checkout page** — `/s/:slug/checkout`, a separate **route** (`ClientRoutes.tsx:13`,
   `CheckoutPage` 1153 lines). This is the only surface that *leaves the menu*.

**Verifying the patch's embed claim:** the panel-over-menu primitive — the `ResponsiveDialog`
bottom sheet — **already exists and is used in BOTH normal and embed mode.** Embed mode only adds a
`embed-mode` body class (`ClientLayout.tsx:41–44`, `index.css:44`) and keeps the sticky bar
(`StickyActionBar embedSticky`). There is **no embed-specific checkout panel.** So the patch's
"panel over menu already exists in embed" is **imprecise but directionally right**: the *drawer
mechanism* exists everywhere; what's new is making **checkout** use it. This is a **merge of
mechanism**, not a new behavior — and it is identical in normal and embed (no embed/normal
divergence introduced → the red-line "embed/normal merge, no regression" is satisfied *because the
patch removes the divergence rather than adding one*).

### Proposed change
Collapse to **two surfaces**: menu → **checkout panel over the menu** (never leaves `/s/:slug`).
The cart drawer disappears as a distinct step; its contents become a **collapsed-by-default summary**
at the top of the checkout panel. Cart bar → opens checkout panel directly.

### Options
- **Option A — Checkout becomes a `ResponsiveDialog` panel (reuse the existing bottom-sheet).**
  *Concept: surface-merge via the existing overlay primitive.* The cart bar opens the checkout panel
  (same sheet the cart drawer used). `CheckoutPage` content moves into the panel; the `/checkout`
  route is retired (or kept as a thin redirect-to-menu-with-panel-open for deep links / refresh
  continuity). **Tradeoff:** keeps one proven primitive, zero new component; must handle deep-link /
  refresh (a customer who reloads on an open checkout) — solved by reading an `?checkout=1` query or
  in-memory state, falling back to "panel closed, cart intact".
- **Option B — Keep `/checkout` as a route but render it visually as an over-menu layer.**
  *Concept: route-as-overlay (modal route).* The menu stays mounted underneath; checkout renders in a
  portal. **Tradeoff:** keeps URL-addressable checkout (good for analytics/back-button) but requires
  keeping the menu mounted under a route — more plumbing, and the menu's own scroll/WS stay live under
  it (cost). Risk of double-mount.

### Decision → **Option A**, with the route retired to a **redirect seam** and the panel as a **history entry**.
The bottom-sheet primitive is proven in both modes; reusing it is the *boring, already-built* choice
and it is what makes normal==embed by construction. Keep `/s/:slug/checkout` as a **redirect that
opens the menu with the checkout panel open** (preserves existing deep links, refresh-continuity, and
the `dos_checkout_draft_*` restore that already lives in `CheckoutPage`’s mount effect).

**RESOLVE / F6 — browser-Back must close the panel, never exit the storefront (no-trap).** The open
panel is a **history entry** (standard modal-as-history pattern), not in-memory-only:
- Open the panel = `pushState('?checkout=1')` (one entry).
- Close the panel (close button **or** browser **Back**/`popstate`) = pop → menu, cart intact, still on
  `/s/:slug`.
- The retired `/s/:slug/checkout` deep-link does `history.replaceState` → `/s/:slug?checkout=1` so the dead
  route never lingers in history and the back-stack does **not** grow per open/close toggle.

So **Back/close on the panel = close + cart intact, still on the storefront** (red-line: no-trap — closing
never strands an order because no order exists until confirm; `clearCart()` only runs on success,
`CheckoutPage.tsx:510`). **Guardrail G-F6** (E2E): with the panel open, browser Back → menu visible, URL
still `/s/:slug`, cart intact.

### Data / migrations
**None.** Pure FE composition. `CartProvider` already holds cart state app-wide
(`ClientLayout` wraps `CartProvider`, line 233) so the panel reads the same `useSharedCart()` the page
uses today (`CheckoutPage.tsx:187`).

### Consistency / idempotency
Unchanged. The order POST (idempotency key, request-hash, cash-422) is identical; only *where the form
renders* changes. The cart↔menu_version reconcile path (`cartReconcile`) is untouched.

### Failures / degradation
- Panel open + `/info` fetch fails → existing `locationLoadFailed` guard disables submit + shows a
  retry message (`CheckoutPage.tsx:222–224, 296–299`). Preserved.
- Customer closes the panel mid-fill → cart preserved, draft preserved (`dos_checkout_draft_*`). No trap.
- Refresh on open panel → redirect seam reopens it from draft, or degrades to closed-panel + intact cart.

**RESOLVE / F9 — redirect-seam deep-link state matrix** (the reconcile trigger moves from route-mount to
**panel-open**; specify it so the seam never strands):

| Deep-link state | Behaviour |
|---|---|
| **Empty cart** (`?checkout=1`, no stored items) | Render menu, **panel closed**, cart intact-empty — nothing to check out. |
| **Stale cart** (items removed / price-changed since) | Run the existing `cartReconcile` (`CartProvider.tsx:62–68`) **on panel-open, before the total renders.** All items dropped → panel closes to menu; prices adjusted → panel shows reconciled total + the existing "prices updated" notice. |
| **Mid-order deep-link** (order already placed) | No order exists pre-confirm; `clearCart()` runs only on success (`CheckoutPage.tsx:510`), so a stale draft with no cart yields a **closed** panel. No partial-order state. |

**Guardrail G-F9** (E2E): deep-link `?checkout=1` with empty cart → panel **not** visible, menu visible.

### Security / tenant
No change. Slug-scoped public storefront; no new endpoints.

### Operability
UI-only; ships behind a FE flag if desired (`FLOW_SIMPLIFIED_CHECKOUT`) so the route→panel swap is
reversible without a deploy rollback. Observable via existing checkout E2E + the visual-regression net.

### Open/accepted risks
- **R1 (accepted):** retiring the `/checkout` route changes a URL that may appear in analytics/E2E
  selectors. *Mitigation:* keep the redirect seam; update `flow-ui-validation.spec` + storefront E2E
  selectors in the same change. Owner: implementer.

---

## §2 — Cart bar shows only item count; checkout = collapsible summary + fields

### Current state (grounded)
The cart bar shows **icon + count badge + "Cart · {total}"** with an `AnimatedNumber` price
(`ClientLayout.tsx:140–155`). The cart drawer lists every item with steppers + total.

### Proposed change (RESOLVE: revised — keep the running subtotal on the bar)
- **Cart bar → item count + running items-subtotal** ("N items · {subtotal}"). *(Reversed from the
  original "count only" — see the RESOLVE note below; Counsel §1/§2/§9.)* The bar keeps the
  always-known **items subtotal**, not the all-in total — so it carries no `feeKnown=false` ambiguity.
- **Checkout panel** = (a) collapsible item summary (collapsed by default, shows count; expands to the
  item rows + steppers that live in the drawer today), (b) delivery fields: phone, map-pin primary /
  text fallback, a **collapsed "pay with: exact" cash row**, the **all-in total** (items + delivery fee;
  "+ delivery fee at checkout" when `feeKnown=false`), one confirm.

### Options
- **Option A — count-only bar, total revealed only in the panel summary.** *Concept: progressive
  disclosure — price at the point of decision, not on the persistent chrome.* Tradeoff: a customer
  scanning the menu no longer sees a running total on the bar; they see it one tap away in the
  summary. Aligns with "remove what the transition adds nothing" — the bar's job is "you have N items,
  go to checkout", not running-total display.
- **Option B — count + total on the bar (status quo), only remove the cart drawer.** Tradeoff: keeps
  the running total but contradicts the patch's explicit "ONLY item count". Less simplification.

### Decision → **REVISED in RESOLVE: Option B-hybrid** — cut the cart *drawer* (scaffolding), keep the running **items-subtotal** on the bar (signal).
*(Original decision was Option A "count-only"; the Breaker/Counsel RESOLVE round overturned it.)*

Counsel's steel-man (§9) is grounded and decisive: **this is cash-as-proof.** The running subtotal is
not convenience chrome — it is the instrument by which a cash-constrained customer (the first-wave
demographic) shops *within the money they can physically produce at the door*. Removing it from the
persistent bar makes them assemble a cart and discover the number only at the commit point. That is the
shape of a dark pattern even unintended. And the original "feeKnown=false ambiguity" justification cuts
the **other** way — the honest response to an unknown *fee* is to surface cost **early** with honest
labeling, not to hide the number.

The resolution threads the needle by splitting the two numbers:
- **Cart bar shows the items subtotal** — always-known, integer, server-mirrorable
  (`estimateOrderTotal` items portion), carries **no** `feeKnown=false` ambiguity because it excludes
  the fee. This is the one number a cash customer most needs while building.
- **The all-in total (items + delivery fee) resolves in the panel** at the decision point — "+ delivery
  fee at checkout" when distance-tier-unknown, exact otherwise. Server total + cash-422 stay
  authoritative (ADR-0005).

This restores coherence with the patch's own principle — **cut scaffolding (the drawer), keep signal
(the subtotal)** — which the original Option A violated. Owner: product ratifies the bar copy.

**RESOLVE-2 / R2-7 — bar-copy + fee-sequencing rule (so the subtotal never masquerades as the total):**
- The **bar label reads as a subtotal, never a bare all-in price** — "N items · {subtotal}" carries an
  explicit **subtotal / nën-total** i18n token (al/en) so it cannot be mistaken for the total.
- The **delivery fee surfaces in the panel the INSTANT a tier is known** — on **pin-drop / address-resolve** —
  **never deferred to the confirm tap.** Flat/known fee → shown on **panel entry**; while `feeKnown=false` →
  "+ delivery fee at checkout"; the E27 preflight surcharge (`acknowledged_codes`, `legacy.ts:70–71`) surfaces
  at the **same** moment (address-resolve). Sequencing: subtotal (browse) → fee on address (panel) → all-in
  total before confirm; the customer never meets a number that grows at the final tap. **G-§2** (E2E): bar
  text contains the subtotal token (not a bare currency reading as total); on address-resolve the panel renders
  the fee line before the confirm action is enabled.

### Data / migrations
**None.**

### Consistency / idempotency
The bar now displays the **items subtotal** (a mirror of the server-side items portion); the all-in
total + cash-pledge / cash-422 path are unchanged. Integer money preserved (no new math;
`formatMoney`/`PriceDisplay` reused). The bar subtotal is display-only — the server total at confirm is
authoritative (a bar/server mismatch is impossible to exploit; cash-422 still backstops).

### Failures / degradation
The cash row stays collapsed with "pay with: exact" default; if `feeKnown=false` (distance tiers) the
panel shows "fee at checkout" exactly as today and the server total + cash-422 stay authoritative.

### Security / tenant — no change. ### Operability — UI-only, same FE flag.

### Open/accepted risks
- **R2 (RESOLVED — reversed):** the original concern was "some users like a running total." The RESOLVE
  round inverted the decision: in a cash-as-proof product the running subtotal is *signal, not chrome*,
  so the **bar keeps the items subtotal** and only the cart *drawer* is cut. The residual risk (a bar
  subtotal that excludes the not-yet-known fee) is handled by honest labeling — "+ delivery fee at
  checkout" in the panel. Owner: product (copy ratification).

---

## §3 — Information floor (one surface, INFO does not collapse)

### Current state (grounded)
Navigation is already 1 surface for *browsing* (menu) but checkout is a second route. The required
delivery fields today: name (required), phone (required, normalized to E.164,
`CheckoutPage.tsx:395–401`), map-pin, street address (required), **entrance (required)**, **apartment
(required)** (`CheckoutPage.tsx:404–413, 749–765`), and a required "how to find you" notes field
(`CheckoutPage.tsx:417–420`).

### Proposed change
Navigation → one surface (per §1). **INFO does not collapse**: a new customer still enters
**phone + a droppable address once**. Document the floor; do not let the simplification delete identity
or location.

### Observation / recommendation (not a hard option choice, but a real tension)
The patch's target "new ≈5 actions" collides with the **current required-field set** (name, phone,
pin, street, entrance, apartment, notes = 7 required inputs). To hit ≈5 without dropping below the
floor, the **map-pin should be primary and the text fields demoted to optional/fallback**:
- **Keep required:** phone; a location (map-pin **or** text address — at least one).
- **Demote to optional:** entrance, apartment, free-text notes (the courier-find detail). The pin +
  phone *is* the floor; entrance/apt improve last-50-meters but the courier can call.

**This is a CONTRACT-adjacent UX choice, flag for council:** today entrance/apartment/notes are
**client-enforced required** (not server-enforced — the server schema accepts the order without them).
So demoting them is a **client-only relaxation**; the server already tolerates their absence. Verify in
council whether the business wants entrance/apt to stay required (kitchen/courier ops call) — if yes,
the floor is 4 (phone+pin+entrance+apt) and "≈5" holds with name; if no, the floor is 2 (phone+pin).

### Rejected options (recorded, do not revisit)
- **Address-on-menu** — puts identity/location collection before intent; trades the floor for a tap and
  leaks the "who/where" question onto a browse surface.
- **Per-item "buy now" bypassing the cart** — fragments the order, breaks multi-item carts and the
  single idempotent `POST /orders`, and re-introduces a per-item surface the patch is trying to remove.

### Data / migrations — none. ### Consistency — unchanged. ### Security — no change.

### RESOLVE / R3 decided (Counsel §3 — "optional must mean skippable, never hidden")
**Required floor = phone + a droppable location (map-pin primary OR text address — at least one).**
`entrance` / `apartment` / `notes` are **optional but contextually present**: rendered inline as
clearly-optional fields (placeholder "Apartment / entrance (optional)"), **never buried behind a "more"
toggle.** This honours both dignity arguments — it spares the confident repeat customer two taps while
ensuring the **least-served** customer (limited phone comfort / language barrier / hearing difficulty /
buzzer-only building), for whom "the courier can call" is the *failure* not the safety net, is not
fighting a hidden field to specify their door. Optional means *skippable*, not *hidden*.

### RESOLVE-2 / §3 — the floor is a THREE-way choice (Counsel R2.3); architect recommends contextually-required
Record the entrance/apartment floor as **three** options, not a binary:
**hard-required | optional-but-inviting | contextually-required (pin-confidence-gated).**
**Architect recommendation: contextually-required** — required when the map-pin is **low-confidence**
(multi-unit / area-level geocode, pin far from a snapped address) and optional when **high-confidence**
(single-unit, pin snapped to a known address). It routes the one ask to exactly where omission causes a
failed last-50-metre delivery + the call the least-served customer cannot take, without taxing the confident
user. **Server-tolerant: a client-side conditional gate, no contract change** (the server already accepts the
order without these fields). Floor stays optional-but-inviting if a pin-confidence signal isn't available at
the seam. **NEEDS-HUMAN:** product/ops ratify the rule + the pin-confidence threshold.

### Open/accepted risks
- **R3 (RESOLVE-2 — NEEDS-HUMAN, three-way):** architect recommends **contextually-required
  (pin-confidence-gated)**; product/ops ratify (rule + threshold). hard-required → floor 4
  (phone+pin+entrance+apt); contextually/optional → floor 2 (phone+pin) with the fields visible. The patch
  does not advance to build on this field until product ratifies. Owner: product + ops.

---

## §4 — Remove order-type/time-slot/promo from the customer UI, keep in foundation (CONTRACT-IMPACT)

### Current state (grounded) — **this is the one real contract item; here are the exact facts**
- **Order-type switch** exists in the UI: `deliveryType` state with a delivery/pickup tab group
  (`CheckoutPage.tsx:193, 730–734`). Scheduled is already hidden (comment line 733).
- **The contract:** `CreateOrderInput.type = z.enum(['delivery', 'pickup'])` — **REQUIRED, no default**
  (`packages/shared-types/src/legacy.ts:42`). The route reads `input.type` and derives
  `isPickup = input.type === 'pickup'` (`orders.ts:93`); `type` also feeds the idempotency
  `requestHash` (`orders.ts:189`).
- **Time-slot / scheduled:** **not in the create contract at all** — the enum has no `'scheduled'`, and
  there is no `slot`/`scheduled_for` field anywhere in `CreateOrderInput`. The machine has a `SCHEDULED`
  state but it is **scaffold-disabled** (`order-machine.ts:38`).
- **Promo-code:** **not in the create contract at all** — there is no `promo`/`discount_code` field;
  `discountTotal` is hardcoded `0` in the route (`orders.ts:497`). Promotions exist as a separate
  owner-side subsystem (`routes/owner/promotions.ts`), unwired to checkout.

### The CRITICAL finding
If the client **stops sending `type`** (removing the order-type field from the payload), the **Zod
parse fails → 400 VALIDATION_FAILED** (`orders.ts:86–90`), **not** a silent default to delivery. This
is exactly the red-line "removing order-type must not 422 the order." So this **is** a contract change.

### Options
- **Option A — Add `.default('delivery')` to the schema; client omits `type`.**
  *Concept: tolerant-reader / additive default.* `type: z.enum(['delivery','pickup']).default('delivery')`.
  Forward-only, additive, **no migration** (Zod-only). `isPickup` stays false, `requestHash` stays
  stable (it hashes the resolved `input.type`, now `'delivery'`). Existing clients that still send
  `type` are unaffected. **Pickup stays fully supported server-side** (foundation seam preserved).
- **Option B — Client keeps sending `type:'delivery'` hardcoded; schema untouched.**
  *Concept: client-pins-the-default.* No contract change at all; the UI just removes the toggle and
  always sends `'delivery'`. Tradeoff: the *contract* still requires the field, so any future client
  (embed host, third party) that omits it 400s — the seam is in the client, not the contract.

### Decision → **Option A (additive default), and the client also keeps sending `'delivery'`** (belt
and suspenders). The default makes the **contract** tolerant (the architecturally correct place for the
seam — "schema rich, runtime minimal": delivery is the only runtime type, but the schema still *knows*
pickup), while the explicit client value keeps `requestHash` deterministic for in-flight clients.
Promo and time-slot need **no contract work** — they were never in the contract; removing them is pure
UI (there is nothing to remove server-side; the foundation already holds them inert).

### Data / migrations
**None.** Schema default is a code change in `packages/shared-types`, not a DB migration. No column is
added or dropped. `discountTotal=0` and the `SCHEDULED` scaffold state stay exactly as-is (foundation).

### Consistency / idempotency
`requestHash` includes `type` (`order-canonical.ts` via `orders.ts:189`). With Option A the resolved
value is `'delivery'` whether sent or defaulted, so two identical carts hash identically → idempotency
holds. **Verify in the guardrail:** a request omitting `type` and a request sending `type:'delivery'`
must produce the **same** `requestHash` (else a double-submit creates two orders). This is the one
test that must go red→green for §4.

### Failures / degradation
Removing the pickup tab removes the pickup *entry point*, not pickup support. If a pickup order ever
arrives (embed host, API client), the server still prices it correctly (`isPickup` branch,
`orders.ts:476–493`). No degradation; the capability is dormant, not deleted.

### Security / tenant — no change (no new field, no PII).

### Operability
Schema change ships with the FE; gated behind the same `FLOW_SIMPLIFIED_CHECKOUT` flag is **not**
needed for the default (it's backward-compatible), but the **UI toggle removal** ships under the flag.
Rollback = re-show the toggle; the default is harmless to keep.

### Open/accepted risks
- **R4 (must-fix before merge):** the requestHash-stability guardrail (above). Owner: implementer.
- **R5 (accepted):** pickup customers lose the self-serve pickup entry until/unless re-enabled. The
  patch explicitly defers pickup to foundation. Owner: product.
- **F8 (RESOLVE — ACCEPT-RISK):** with `.default('delivery')`, omitting both `type` and `delivery` now
  fails the superRefine (`legacy.ts:74–76`, path `['delivery']`) instead of the enum — a cosmetic envelope
  change (order still 400s; `requestHash` unaffected, it hashes the resolved value `order-canonical.ts:42`).
  Accepted; update any test asserting the old `type`-missing issue in the same change. Owner: implementer.

---

## §5 — Owner 2 taps → 1 (remove the manual READY tap from the main path)

### Current state (grounded) — **the machine already allows the shortcut**
- **State machine:** `CONFIRMED: ['PREPARING', 'IN_DELIVERY']` — **CONFIRMED→IN_DELIVERY is already a
  legal edge** (`order-machine.ts:20`). `PREPARING: ['READY']`, `READY: ['IN_DELIVERY','PICKED_UP']`.
  So PREPARING/READY are an **optional kitchen lane**, not a required gate. (The deliver-v2 work added
  `IN_DELIVERY: [...,'READY']` as a revert target, `order-machine.ts:28` — still valid.)
- **Owner UI today = 4 taps** (`OrderCard.tsx:224–235`): PENDING→**Accept**(CONFIRMED) →
  **Mark Preparing**(PREPARING) → **Mark Ready**(READY) → **Assign Courier**(IN_DELIVERY).
- **Assign mechanics:** the OrderCard "Assign Courier" button PATCHes status→IN_DELIVERY
  (`onUpdateStatus`→`/orders/:id/status`); the route then **auto-assigns the nearest available
  courier** inside the same tx (`orders.ts:785–821`). A separate manual-pick + offer-handshake path
  exists at `/owner/:loc/orders/:id/assign-courier` (deliver-v2 §A, flagged) and **accepts status
  CONFIRMED, PREPARING, or READY** (`dashboard.ts:235`). **Either path already permits assigning from
  CONFIRMED** — READY is not on the assign path.

So the patch's "after confirm+assign (already one tap → IN_DELIVERY)" is **correct**: the machine and
both assign routes already support CONFIRMED→IN_DELIVERY. **This is mostly UI removal**, exactly as the
patch's §5 brief anticipated.

### Options for the READY-removal mechanism
- **Option A — UI-only: collapse the OrderCard action set.** *Concept: surface the legal shortcut.*
  When `status==='CONFIRMED'`, render **"Send for delivery / Assign"** (→IN_DELIVERY) as the primary
  action; drop the standalone "Mark Preparing" and "Mark Ready" buttons from the default card.
  PREPARING/READY remain valid states (machine untouched). **No migration.** Tradeoff: kitchens that
  *want* a "preparing/ready" beat lose it until the toggle (below) is built.
- **Option B — Location-level toggle drives the action set.** *Concept: schema seam, runtime gated.*
  A `locations.kitchen_flow_enabled boolean default false` (additive). When true, the OrderCard shows
  the full Accept→Preparing→Ready→Assign lane; when false (default), the 2-tap lane. Tradeoff: needs an
  additive migration + an owner setting UI. The patch explicitly wants READY kept as a **deferred
  location-level toggle** — so the *column* is the foundation seam, but the **toggle UI is deferred**.

### Decision → **Option A now (UI-only), Option B's column as a deferred foundation seam.**
Ship the 2-tap path as a pure `OrderCard` change (no migration, no machine change). Add the
`kitchen_flow_enabled` column **later, additively, when the toggle is actually built** — do **not** add
an unused column now ("schema rich, runtime minimal" means cut the seam *when* you wire it, not
speculatively). Until then the card reads a constant `false`. PREPARING and READY stay in the machine
and in the dashboard queries (`dashboard.ts:100` IN_DELIVERY/READY; the owner-reassign IN_DELIVERY→READY
revert `dashboard.ts:288`) — all still land in valid states.

### RESOLVE / F1 — the no-trap fix (RESOLVE-2 re-grounded): make the REAL endpoint honest + revert recovery
Moving Assign to right-after-Accept turns the rare no-courier orphan into the **routine** case, and the
shipped `OrderCard` has **no IN_DELIVERY action branch** (`OrderCard.tsx:221–236`) to recover it.

**RESOLVE-2 correction (the round-1 fix was grounded on a fictional primitive):** there is **no** dispatch
primitive that auto-discovers a courier and synchronously returns `{dispatched:false}`. `/assign-courier`
(`dashboard.ts:214`) **requires an explicit `courierId`** (400 if missing, `:219–222`), 404s on no courier
(`:246`), and **409s on IN_DELIVERY** (`:235–237`) — it never auto-discovers. The **only** auto-discovering
path is the raw PATCH `/orders/:id/status`, which is also **the only endpoint the card's single
`onUpdateStatus` callback can reach** — and it `updateOrderStatus(IN_DELIVERY)` **unconditionally first**
(`orders.ts:779`), then looks up a courier, leaving the silent IN_DELIVERY orphan at 200 on `rowCount 0`
(`orders.ts:786–800,824`). So the fix must land on that endpoint, not on a primitive that does not exist.

- **(a) Make the auto-assign honest on the endpoint the card actually calls** (Breaker option b). Reorder the
  PATCH `/orders/:id/status` handler so that for `newStatus==='IN_DELIVERY' && type==='delivery'` the
  **courier-availability lookup runs BEFORE** `updateOrderStatus(IN_DELIVERY)`:
  - **courier found** → `updateOrderStatus(IN_DELIVERY)` + INSERT assignment + shift→`on_delivery` (today's
    body, just reordered); transition `CONFIRMED→IN_DELIVERY` (already-legal, `order-machine.ts:20`).
  - **`rowCount===0`** → **do NOT** advance; leave the order at its current status and return
    `{ id, status: <currentStatus>, dispatched: false, reason: 'no_courier' }` (HTTP 200,
    **server-authoritative — the response reports the REAL status, not the requested one**). The card renders
    an **"awaiting courier"** affordance on the still-CONFIRMED order. The order **never enters IN_DELIVERY
    with no courier.** No courier-picker needed (the lookup auto-selects the sole MVP courier); "Send for
    delivery" wires through the existing `onUpdateStatus('IN_DELIVERY')` unchanged.
  - *Flag-independence (R2-2):* this path **never** touches `/assign-courier`, so the no-orphan property holds
    regardless of `COURIER_OFFER_HANDSHAKE_ENABLED` (dark by default). All four `OWNER_TWO_TAP`×handshake
    cells are non-orphaning — see `resolution.md` R2-2 matrix. *(Option a — explicit courier-pick — is the
    heavier multi-courier path: a second card callback + a `courierId` picker the card lacks. Deferred.)*
  - *Binding-aware (RESOLVE-3 / R3-2):* the dispatch must be **`'offered'`-aware** — the mig-073 partial
    uniques `courier_assignments_order_active_uniq` and `courier_one_active_assignment` both include
    `'offered'` (`…073:22–24,32–33`), but the §5 lookup excludes only `('assigned','accepted','picked_up')`
    (`orders.ts:792–794`). Two corrections: **(i) already-bound guard** — before any flip/INSERT, if the
    order already has an active binding (`status IN ('offered','assigned','accepted','picked_up')`) →
    **no-op + clear signal**: `{status:'CONFIRMED',dispatched:false,reason:'offer_pending'}` for `'offered'`
    (never force IN_DELIVERY — keep the handshake's CONFIRMED-until-accept), `{...,dispatched:true,
    reason:'already_assigned'}` otherwise; **(ii) exclude `'offered'` from the availability lookup** (a
    courier mid-offer is not available, matching `courier_one_active_assignment`). Without these, "Send for
    delivery" on a pending-offer order INSERTs a conflicting active row → **23505 → uncaught 500**
    (`orders.ts:825–832`), not the honest pending signal.
- **(b) Add an IN_DELIVERY recovery branch to `OrderCard`** = **binding-aware recovery via the shipped
  `releaseBindingAndReoffer` rail** (RESOLVE-3 / R3-4 — corrects round-2's raw revert). `/assign-courier`
  **409s on IN_DELIVERY** (`dashboard.ts:235–237`), so a "re-assign" there is non-functional. Round-2 specified
  a raw `onUpdateStatus('READY')`, but the central fold (`orderStatusService.ts:129–140`) **blanket-cancels
  the binding regardless of `asg_status`, including `'picked_up'`** → it would set the order READY while the
  food is out with the courier, **contradicting the shipped `/abort`** (`bindingRelease.ts:37–40` sends
  `picked_up`→CANCELLED). The owner recovery therefore **reuses `lib/bindingRelease.ts`** (an owner-scoped
  endpoint loads the active binding `FOR UPDATE` and calls `releaseBindingAndReoffer`), whose asymmetric
  branch is the only correct one: pre-pickup (`assigned`/`accepted`) → **READY** + cancel + re-enqueue
  (re-dispatchable); `picked_up` (food out) → **CANCELLED**, **never READY**. After a pre-pickup revert, the
  owner re-taps **"Send for delivery"** from READY (the honest (a) dispatch) as a **fresh** action. This
  closes the **latent bug present today** (the card has *zero* IN_DELIVERY action) **without** inventing a
  raw path that lies on `picked_up`. *(Product may instead disable recovery on `picked_up` and show "courier
  has the food — contact courier"; the floor is simply **never READY-with-food-out**.)*
- **(c) Re-dispatch of an "awaiting courier" order (RESOLVE-3 / R3-3):** the §5 endpoint is **pull-based** and
  **never enqueues** `courier_dispatch_queue` (only `releaseBindingAndReoffer` does, `bindingRelease.ts:31`),
  so the async `CourierDispatchWorker` is **not** on this path. The specified re-dispatch path is the **owner
  re-tapping "Send for delivery"** from the CONFIRMED+awaiting card once a courier is online — a real,
  working loop, no new plumbing. *(DEFER-FLAG: the async auto-pickup is a future enhancement and currently
  **broken** — `courier-dispatch.ts:76` calls `this.boss.send` but the constructor stores `this.queue`
  (`:10–14`), throwing **after** COMMIT; its lookup is also `'offered'`-blind (`:55–58`). Owner: implementer,
  separate fix.)*

**Guardrails (red→green), owner: implementer:**
- **G-F1a** (API/integration, REVISED to the real shape): PATCH `/orders/:id/status` `{status:'IN_DELIVERY'}`
  with **zero available couriers** → response `{ status:'CONFIRMED', dispatched:false, reason:'no_courier' }`
  **and** the DB row stays `'CONFIRMED'`. *Red today* (`orders.ts:779` already flipped the row; `:824`
  returns `{status:'IN_DELIVERY'}`). Asserts the **real** endpoint's shape, not the fictional primitive.
- **G-F1a-2** (RESOLVE-3 / R3-2): PATCH `{status:'IN_DELIVERY'}` on an order that **already has an `'offered'`
  (or any active) binding** → **no 500, no second INSERT**; response is the `offer_pending`/`already_assigned`
  signal and the DB row is unchanged. *Red today* (the conflicting INSERT throws 23505 → 500).
- **G-F1b-i** (component/E2E, RESOLVE-3): IN_DELIVERY with an `accepted` (pre-pickup) binding → owner recovery
  → order `READY` + binding `cancelled` + re-offerable. *Red today* (no IN_DELIVERY recovery branch).
- **G-F1b-ii** (RESOLVE-3 / R3-4 guard): IN_DELIVERY with a `picked_up` binding → owner recovery → order
  `CANCELLED` (**NOT READY**); binding terminalized. *Red today* (a raw `updateOrderStatus('READY')` would set
  READY + cancel the picked_up binding — the food-out lie).
- **G-F1c** (RESOLVE-3 / R3-3): an order left CONFIRMED+awaiting, re-dispatched via the §5 action **once a
  courier is available**, reaches `IN_DELIVERY` with an assignment. *Proves the awaiting→delivering loop
  closes (no dead end).*

### RESOLVE / F3 + F4 + F5 (RESOLVE-2 re-decided) — honest customer signal WITHOUT fabricating a timestamp
The owner saves the tap; the customer keeps the signal — but we do **not** fabricate a physical
`preparing_at`. **RESOLVE-2 supersedes round-1's auto-stamp** (R2-5): stamping `preparing_at = confirmed_at`
asserts "the kitchen started cooking" when it has not — a **data-layer** lie (`preparing_at` is read by
`fetchOrderDelta`/`OrderProgress`), worse than a copy label, and it starts the ETA decay at accept-time.

- **F4 — do NOT auto-stamp `preparing_at`.** It stays NULL on 2-tap orders, which is **honest** (no kitchen
  ever marked preparing). *Re-grounding:* **no live consumer learns `ready_at − preparing_at`**
  (`synthesizeAndPersistEtaWindow` uses configured `prep_time_minutes` + decay, never `ready_at` —
  `etaGather.ts:184–217`), so nothing is starved. **DEFER-FLAG:** a future duration-learner **must filter
  `ready_at IS NOT NULL` and `preparing_at IS NOT NULL`** (never treat NULL as a 0-minute cook); it resumes
  with the deferred `kitchen_flow_enabled` toggle.
- **F3 — drive the customer ETA from timestamps, off a REAL, always-non-NULL base (RESOLVE-3 / R3-5 corrects
  the source).** Round-2 said "decay off `confirmed_at`" — but grounded, `synthesizeAndPersistEtaWindow`
  SELECTs `o.created_at`/`o.preparing_at` only (`etaGather.ts:192`), passes `createdAt`/`preparingAt`
  (`:216–217`), and `confirmed_at` is **neither selected nor passed**; worse, a directly-CONFIRMED order may
  have **NULL `confirmed_at`** (stamped only on the CONFIRMED *transition*, `orderStatusService.ts:11–18`). So
  the real spec is: decay base = **`COALESCE(preparing_at, confirmed_at, created_at)`** — add `o.confirmed_at`
  to the SELECT + a `confirmedAt` arg (additive; round-2 elided this plumbing), and `created_at` is the
  **always-non-NULL** floor (every order has it), closing the NULL-`confirmed_at` hazard by construction.
  `prepRemaining = max(0, prep − minutesSince(preparing_at ?? confirmed_at ?? created_at))` while `ready_at IS
  NULL`; hard-zero **only** on an authoritative "food out" signal (`ready_at IS NOT NULL`, or a real courier
  pickup). No invented timestamp is introduced. *kitchenAhead is grounded-clean:* it SUMs only
  `status='PREPARING'` orders (`etaGather.ts:101`), which 2-tap orders never are, so there is **no
  zero-interval skew**.
  **G-F3** (CORRECTED, unit): (a) a CONFIRMED order with `confirmed_at = now()−2min`, `preparing_at NULL`,
  `ready_at NULL`, `prep=15` → `prepRemaining ≈ 13` (NOT 0, NOT a flat 15); (b) a CONFIRMED order with
  **`confirmed_at NULL`** and `created_at = now()−2min` → `prepRemaining ≈ 13` (proves the `created_at`
  fallback). *Red today* (`etaGather.ts:81–86` returns the flat `orderPrep` for CONFIRMED; `confirmed_at` not
  in the SELECT).
- **F5 — drive `OrderProgress` dots from timestamp presence; "Preparing" is a PROCESS label.** A step shows ✓
  only if its `at` timestamp is non-NULL. The **"Preparing" step renders active/in-progress** (never ✓) when
  the order is past CONFIRMED and `ready_at IS NULL` — a status-driven process label, **not** a fabricated
  `preparing_at`. So no "Preparing ✓ / Ready ✓" over stages the kitchen never entered. **Copy rule:** the
  state copy reads **"Preparing your order"** (process), never "Your food is being cooked right now" (an
  asserted physical action). **G-F5** (component): `status='IN_DELIVERY'`, `readyAt=null` → Ready step not
  rendered completed/✓. *Red today* (`OrderProgress.tsx:84–102` fills every dot ≤ statusIndex).

### Data / migrations
**None now.** (Future: additive `locations.kitchen_flow_enabled boolean NOT NULL DEFAULT false`, RLS
already FORCE-on for `locations`; forward-only; no backfill needed — default covers existing rows.)

### Consistency / idempotency
The status PATCH is unchanged (`assertTransition` still validates each edge). CONFIRMED→IN_DELIVERY was
already legal, so no new edge is introduced — **zero state-machine change required.** The deliver-v2
no-trap folds (terminalize assignment on IN_DELIVERY→READY/CANCELLED) are untouched.

### Failures / degradation
- **No courier available (RESOLVE / F1 — corrected from the original "preserved behavior" framing):** the
  Breaker is right that §5 turns this from rare to routine, and that the raw PATCH leaves a silent
  IN_DELIVERY orphan with **no recovery button** on the shipped card. **Fixed** (see RESOLVE/F1 above): the
  dispatch attempt **keeps the order CONFIRMED** when no courier is found (no orphan, via the honest
  `orders.ts:785` endpoint — RESOLVE-2/R2-1) and the card exposes an "awaiting courier" affordance; any
  IN_DELIVERY without an assignment (legacy/edge) has a **revert-to-READY** recovery (RESOLVE-2/R2-4), after
  which the owner re-taps "Send for delivery" as a fresh action. No-trap holds by construction, under every
  flag cell. (The `/assign-courier` re-assign 409s on IN_DELIVERY, so it is **not** the IN_DELIVERY recovery.)
- Removing the kitchen beat means an order can go CONFIRMED→IN_DELIVERY before food is ready. That's a
  **process** choice (the patch's intent: delivery-only, owner controls timing by *when* they tap
  Assign). The READY toggle re-inserts the beat for kitchens that need it. **The customer-facing cost of
  this (ETA/progress honesty) is mitigated by RESOLVE/F3+F5** (timestamp-driven ETA + progress), so the
  IN_DELIVERY label never over-promises "out for delivery" before the food is made.

### Security / tenant
No change. `/orders/:id/status` stays `requireRole(['owner'])` + tenant-scoped via `withTenant`
(`orders.ts:750, 766`).

### Operability
UI-only; behind an owner FE flag (`OWNER_TWO_TAP`) if a staged rollout is wanted. Rollback = restore the
two intermediate buttons. Observable via the owner dashboard E2E (confirm→assign→IN_DELIVERY).

### Open/accepted risks
- **R6 (RESOLVED — F1/F3/F5):** the unassigned-IN_DELIVERY orphan is **prevented** (gated dispatch keeps
  the order CONFIRMED on no-courier) **and recoverable** (IN_DELIVERY card branch); the customer-facing
  honesty cost of pre-ready IN_DELIVERY is mitigated by timestamp-driven ETA + progress. Guardrails G-F1a/b,
  G-F3, G-F5. Owner: implementer (guardrails) + product (kitchen-flow toggle).
- **R7 (RESOLVED — re-grounded):** no downstream consumer **requires** READY. `synthesizeAndPersistEtaWindow`
  uses configured `prep_time_minutes` + `preparing_at`, **not** `ready_at` (`etaGather.ts:184–217`); READY
  is otherwise used only for display/filter + the reassign-revert. **DEFER-FLAG:** any *future*
  kitchen-duration learner must filter `ready_at IS NOT NULL` (don't treat NULL as 0-minute cook). Owner:
  implementer.

---

## §6 — Onboarding → CLAIM model (RECONCILIATION with shipped P6, not a rebuild)

### Current state (grounded) — **the claim backend is SHIPPED; the owner surface is the gap**
- **Self-serve onboarding exists:** `MenuFirstOnboarding` (`OnboardingPage` → menu-upload → parse →
  create+seed storefront → activation, `OnboardingPage.tsx:11–13`). This is the "deferred self-serve"
  the patch wants to de-emphasize.
- **P6 claim vertical is SHIPPED (dark):**
  - Routes: `POST /api/claim/accept` (verifyAuth-only, token is sole transfer authority),
    `POST /api/claim/decline` (no-auth, token-only, one-action erase), `POST /api/claim/request`
    (`routes/public/claim.ts`).
  - Modules: `acceptClaim` / `declineAndErase` (`modules/acquisition/claim.ts`).
  - Migration: `…071_claim-invites` (claim_invites table + claim_accept policies, ENABLE+FORCE RLS,
    256-bit token, single-use/TTL/revoked, `claim_transfer` SECURITY-DEFINER ownership transfer).
  - Council-hardened + approved (`docs/design/p6-claim-council-verdict.md`,
    memory `p6-provisioning-vertical`). Dark until operator sets `PROVISION_OPS_SECRET` + places
    migs 068–071.
  - **Web UI callers of `/claim/*`: ZERO** (grep `claim/accept|claim/decline|claim/request` over
    `apps/web/src/**/*.tsx` → no matches). **The owner-facing claim experience does not exist yet.**

### The DELTA (precisely scoped — what is missing to realize the patch's owner experience)
1. **Owner claim web surface (NEW, the real work):** a page (e.g. `/claim/:token` or `/claim?token=…`)
   that (a) resolves the token to its working preview, (b) shows the **already-built** branded service
   (menu ported, theme applied, radius set, demo orders) read-only-ish, (c) offers **light-edit**
   (items/prices/theme/radius — **NO full palette/layout/zone editors**, per the patch), (d) drives the
   **existing** `POST /api/claim/accept` (binds owner login → membership → `claim_transfer`), and (e)
   offers the **equally-prominent decline** (`POST /api/claim/decline`) per council H-decline/CC2.
2. **Approve subsystem (K4 — may still be unbuilt):** the per-product `allergens_confirmed=true` writer
   into **empty** allergen fields (council CC3 — distinct deliberate act, never confirm an AI guess).
   Verify whether K4 shipped; if not, it is part of this delta.
3. **Funnel wiring:** decide how `/start` + `OnboardingPage` route between **claim** (operator-provisioned
   shadow exists) and **self-serve** (no shadow). The patch wants claim primary, self-serve deferred to
   foundation — so de-emphasize (not delete) the self-serve entry.

### CONTRADICTION with the shipped + council-approved model — **must be revised, flagged for council**
The patch's §6 says claim is **"one action: takes ownership, binds owner login, goes live."** The
shipped P6 council verdict **explicitly forbids this**:
- **CC2:** "claim → review → publish stays **THREE acts**. No one-click 'claim & go live'" — a one-click
  go-live would launder unreviewed AI descriptions as the owner's word (violates decision #4).
- **CC3:** allergen confirmation is a **distinct deliberate act** into empty fields.
- **H-publish-coupling / H-publish:** `published_at` stays **NULL through claim**; publish only via the
  **gated activation path** (`activation.ts` requires `menu_confirmed_at IS NOT NULL`, which a shadow
  never has → a claimed-but-unreviewed shadow **physically cannot publish**).

**Architect ruling:** **REVISE the patch.** Claim **takes ownership and binds login** (one action — that
part is correct and already what `acceptClaim` does), but **"goes live" is a separate, gated act**
(review menu/allergens → publish). The owner opens a *working* service and can *demo/preview* it
instantly (the patch's real value — "a working service before you touch it"), but **public
orderability** waits on the review→publish gate. This preserves the never-orderable B3 invariant and the
AI-allergen safety architecture **without** weakening anything shipped. Do **not** redesign or duplicate
`claim_transfer`; do **not** collapse the 3-act sequence.

### Options for the claim-surface delta
- **Option A — New dedicated claim route/page consuming the shipped endpoints.** *Concept: thin surface
  over a shipped vertical.* Build `/claim` as a new SPA route; reuse the existing storefront preview
  (the shadow already renders via `read_preview_menu`) + the admin light-edit components (price/item
  editors already exist in `MenuManagerPage`, theme in `BrandingPage`, radius in `locations`). Wire to
  `/claim/accept` + `/claim/decline`. **Tradeoff:** new route + auth-bind UX, but zero new backend.
- **Option B — Fold claim into the existing `OnboardingPage`/`MenuFirstOnboarding` as a "claim" mode.**
  *Concept: reuse the onboarding shell.* Tradeoff: `MenuFirstOnboarding` is built around *upload→create*,
  the opposite mental model of *claim an existing thing*; bending it risks confusing two flows. The
  claim recipient is a **hostile/unsolicited** recipient (council CC1) needing an honest Art-14 notice —
  a different tone than the self-serve "build your store" funnel.

### Decision → **Option A (dedicated claim surface), preserving the 3-act sequence.**
Reuse the shipped endpoints, the shadow preview, and the existing light-edit components; build only the
**claim page + auth-bind + decline + the K4 approve writer (if unbuilt)**. Keep self-serve onboarding as
a deferred foundation path. **The single "claim" action = ownership+login bind; go-live stays gated.**

### Data / migrations
**Reuse P6 schema — no new migration for the transfer.** The only possible new migration is K4's
`allergens_confirmed` writer *if it was never built* (verify against the verdict's K4 condition); that
would be the owner-authored-allergens path, additive, owner-scoped (`requireRole(owner)` +
`requireLocationAccess`), RLS already FORCE-on. **Do not touch `claim_transfer`.**

### Consistency / idempotency
`acceptClaim` is the shipped authority — token single-use (`used_at`), one active per source. The
surface must treat accept as **idempotent on the token** (re-presenting a used token → 409
ALREADY_CLAIMED, already handled `claim.ts:33`). The surface must not invent its own transfer.

### Failures / degradation
- Token expired/used/revoked → `claim.ts` returns 401/409/422; the page shows the council-mandated
  honest message (not "try a different store" — never enumerate shadows, K2).
- Decline must be **equally prominent** to claim and **work without an account** (`/claim/decline`,
  no-auth, token-only) → calls `hardDeleteShadow` (H-decline). The surface must not hide it.
- Auth-bind fails mid-claim → token unconsumed, owner_id stays NULL, retry-safe (no partial transfer).

### Security / tenant — **the red-line to protect, not reopen.**
`claim_transfer` is the SECURITY-DEFINER ownership transfer (an UPDATE target invisible under the
claimer's RLS — the memory's key PG lesson). **The surface delta must NOT add any inline UPDATE to
`organizations.owner_id` or `memberships`** — it calls `acceptClaim` only. Token is the sole transfer
authority (K2, IDOR-closed); the page **must derive org/location from the matched invite, never from a
request param**. `/claim/accept` stays `verifyAuth`-only (K3); role re-derives from membership
post-claim (ADR-0004) — the surface should hint a re-auth (the route already returns `reauth: true`,
`claim.ts:28`).

#### RESOLVE / F2 — claim-token transport (the surface delta, NOT `claim_transfer`)
The API is already clean (token in **body**: `claim.ts:21,72` read `request.body`, never the query). The
break is the **web surface**: a `/claim/:token` or `/claim?token=…` page would write the 256-bit
sole-authority token into URL → history → **Referer** (to map-tile/font/analytics third parties) →
**CDN/access logs**, within the 72h TTL — re-opening ownership-theft + griefing-erase. Specify the
transport so the token never leaks:

1. **Fragment delivery + immediate scrub.** The operator link is `…/claim#t=<token>`. The **fragment is
   never sent in Referer (per spec), never in server/CDN access logs, never in the query string.** On
   mount the SPA (a) reads `location.hash`, (b) `history.replaceState(null,'','/claim')` to scrub it from
   the address bar **before any third-party resource loads**, (c) holds the token in an **in-memory
   variable only** — never `localStorage`/`sessionStorage` (persistence + XSS-exfil surface; cookies
   already forbidden by the zero-cookie red-line).
2. **Preview = read-only, token-in-body, no enumeration.** Resolving the token to its working preview is a
   POST with the token in the **body** (returns only already-public shadow menu/branding — no PII), generic
   on bad/expired/used (never reveal whether a slug is a claimable shadow, K2). This is the one net-new
   endpoint the surface needs; same body-only + no-enumeration + rate-limit rules as the shipped `/claim/*`.
3. **Account + accept happen IN-PAGE (no navigation) — this resolves the round-2 token-loss contradiction.**
   The Breaker (R2-3) was right that an auth *navigation* would destroy the in-memory token. But auth here is
   **zero-cookie JSON-token** (`/auth/*` returns the JWT in the body), so the SPA authenticates by **fetch,
   not navigation** — the `/claim` page never unloads and the in-memory token survives. A brand-new owner with
   **no account** registers / OTPs **in-place** (a step inside the same `/claim` route) **with the invited
   email**, holds the JWT in memory, then POSTs `{ token }` in the **body** to the shipped `/claim/accept`
   (`verifyAuth`-only, binds to the authed `userId`, `claim.ts:23–26`). The token is **never** persisted or
   re-URL'd; `used_at` is set at accept, so any post-accept navigation to `/admin` cannot replay it.
4. **Recipient binding (RESOLVE-3 / R3-1 — corrects the round-2 OVERSTATEMENT + adds the web NULL-hash
   refusal).** The invite carries `invited_contact_hash` (`…071:27`, **nullable**); `claim_transfer` enforces
   `sha256(lower(trim(users.email))) == invited_contact_hash` → `CONTACT_MISMATCH`/**403** (`…071:64–68`,
   `claim.ts:35`) **only when the hash is non-NULL** (the comment is explicit: *"Token-only when
   invited_contact_hash IS NULL"*). **Honesty correction:** round-2 called this *"proof of control of the
   invited email"* — that is **overstated.** It is a **string compare against the claimer's `users.email`**,
   and the auth path (`local.ts:88–108`) does **no email-ownership verification** (OTP off), so it proves only
   that the claimer **registered an account under that email string** — defeatable by registering the
   restaurant's scraped public contact. It is a **speed-bump**, not identity proof. Two parts:
   - **(a) FIXED on the web surface — refuse token-only invites.** The net-new **preview** endpoint and the
     **accept** path require the matched invite to have `invited_contact_hash IS NOT NULL`; a token-only
     invite returns the **generic "link no longer valid"** (no enumeration, K2). This makes *"binds any authed
     account"* **unreachable via the web path.** `claim_transfer` is **UNTOUCHED** (it still permits NULL-hash
     for non-web/ops paths; the web surface simply never reaches it with a token-only invite). The operator
     mint still MUST supply `invitedContact` (G-F2d) — G-F2d binds at mint, **G-F2g** enforces at the surface.
   - **(b) ACCEPT-RISK + DEFER-FLAG — the email-ownership gap is pre-existing P6/auth.** The defeat (register
     the scraped email → pass the string match) is a **shipped** P6 + auth weakness, **not** introduced here.
     The vertical is **dark**; the surface delta does not worsen it and adds the non-NULL-hash precondition.
     The real fix — **email-ownership verification / OTP before claim** — is **out of scope**; DEFER-FLAG to
     the **P6/auth owner.** Residual stated plainly: F2's web surface rests on **three** legs — (1) no token
     leak (transport, this patch), (2) operator mints a real contact (G-F2d, this patch), (3) the invited
     email not being attacker-registerable (**deferred**). Legs 1–2 closed here; leg 3 is the P6/auth seat's.
5. **Decline stays no-auth token-only** (H-decline), POST body from the in-memory value, rate-limited
   (`max 10/min`, `claim.ts:70`). Griefing **on-page** is **closed by transport** (no URL/Referer/log leak).
   **RESOLVE-3 / R3-6 residual (NOTED + DEFER-FLAG):** the transport fix protects the token only **while the
   user is on `/claim`** — it does **not** shrink the token's **72h life in the delivery channel** (email/SMS
   to the scraped contact), within which anyone with channel access can invoke the **destructive**
   `declineAndErase` → `hardDeleteShadow` (irreversible). ACCEPT for this patch (shipped P6 behavior,
   account-free by design, token reaches only the verified contact; the delta does not worsen it);
   **DEFER-FLAG to the P6/acquisition owner** a blast-radius reduction — a **shorter decline TTL** and/or a
   **soft-delete grace window** so an erroneous/hostile decline is recoverable, not an immediate hard-delete.
6. **Scrub-before-init (R2-6).** The `/claim` route is on the **telemetry exclusion list** (no app error-SDK
   / page-view beacon inits on it) **and** the fragment scrub runs as the **first synchronous pre-init
   statement** of the entry module (before any code reads `location.href`). Defense in depth.

**Guardrails (red→green), owner: implementer:**
- **G-F2a** (E2E): after mount, `page.url()` contains the token in **neither** `search` **nor** `hash`;
  address bar is `/claim`.
- **G-F2b** (E2E request-intercept): **no** outbound request URL and **no** Referer header carries the
  token — incl. the first pageload beacon; it appears **only** in POST bodies.
- **G-F2c** (API): `/claim/accept?token=…` with an empty body → `400 VALIDATION_FAILED` (proves query is
  ignored — server reads body only).
- **G-F2d** (integration): an acquisition-flow invite has `invited_contact_hash` **NOT NULL**; a claim by an
  authed user whose email ≠ invited contact → **403 CONTACT_MISMATCH** (proves the recipient binding is active,
  not token-only).
- **G-F2g** (RESOLVE-3 / R3-1, integration): a web claim (preview **or** accept) against a token whose invite
  has `invited_contact_hash IS NULL` → refused with the **generic** error, and `organizations.owner_id` stays
  NULL (proves the web path **refuses token-only invites** — the "binds any authed account" theft is
  unreachable via the surface; `claim_transfer` untouched).
- **G-F2e** (E2E): the full claim (preview → register-with-invited-email → accept) completes with **no
  full-page navigation** between reading the fragment and POSTing accept, and **no** `localStorage`/
  `sessionStorage` key is written (proves the first-time-owner path works without persisting/re-URLing the
  token).
- **G-F2f** (boot-order): the scrub shim executes before any telemetry/SDK init on `/claim`.

#### RESOLVE / Counsel §4 — PROTECTED FRICTION + the two §6-surface dignity notes
- **PROTECTED FRICTION (RESOLVE-2 — durable as CODE, not prose):** the claim → review → publish three-act
  sequence (CC2) and allergen confirmation as a distinct deliberate act into empty fields (CC3) exist for
  **consent and allergen safety** — **explicitly distinct from the incidental cart/page friction this patch
  removes.** To survive the *next* simplification pass this must land at the claim/activation code seam as a
  **named in-code marker** (`// PROTECTED-FRICTION (P6 council CC2/CC3): consent + allergen gate — do not
  collapse`) **plus two build-time guardrails:** **G-PF1** — `published_at` stays NULL through claim
  (activation requires `menu_confirmed_at`; assert red→green so a future fold can't quietly publish);
  **G-PF2** — allergen confirmation is a **distinct authenticated act writing only into empty fields** (assert
  an AI guess is never auto-confirmed and the act is not folded into `accept`/`publish`). So collapsing the
  gate trips a **deterministic red**, not just a prose warning. The council concurs with the **revised** ADR
  §6 only — the original "one action … goes live" prose is **not a build source.**
- **CC1 sequencing:** the honest Art-14 notice ("you didn't ask for this; here's exactly what we did and
  your options") **dominates the first screen**; the seductive working preview comes **second** —
  preview-before-notice would launder the consent.
- **H-decline parity:** decline stays **equally prominent** and **account-free**; claim-louder-than-decline
  is the dark-pattern tell the P6 verdict already named — the new surface must not reintroduce it.

### Operability
The whole claim vertical stays **dark until operator sets `PROVISION_OPS_SECRET` + places migs 068–071**
(memory). The new claim *surface* ships behind the same activation gate. Health: the claim page must
degrade to a plain "this link is no longer valid" on any `/claim/*` 4xx. Council CC4: track
**decline-without-complaint + zero C&Ds** as the health signal, not claim-rate.

### Open/accepted risks
- **R8 (must-revise):** patch §6 "one action … goes live" contradicts shipped CC2/CC3/H-publish.
  *Resolution:* revise to "one action = ownership+login bind; go-live is a separate gated act." Owner:
  architect (this proposal) → council to ratify.
- **R9 (verify):** is the K4 approve subsystem (`allergens_confirmed` writer) shipped? If not, it is in
  scope for this delta. Owner: implementer to grep/confirm before building.
- **R10 (RESOLVE-tightened — Counsel §6 "guard the exit"):** self-serve onboarding is de-emphasized but
  kept **test-warm and reversible** — its E2E stays green and its seam stays warm, so the operator-effort
  bootstrap does not quietly become the only path the week operator capacity runs out. De-emphasized
  foundations rot; this one is held reversible on purpose. Owner: product.

---

## Cross-cutting: red-lines preserved (summary)

| Red-line | How preserved |
|---|---|
| Order contract / state-machine | §4 additive default only (no migration); §5 uses an **already-legal** edge (no machine change). |
| Money (cash-only, integer) | No money math touched; cash-422 + integer totals untouched (§1/§2 only move *where* the form renders). |
| Claim ownership-transfer (RLS / SECURITY-DEFINER) | §6 surface **calls** `acceptClaim`; **zero** new inline ownership UPDATE; token = sole authority; org/loc derived from invite. **RESOLVE/F2:** token transport = fragment+scrub, in-memory, **body-only** — never URL/Referer/log. |
| No-trap states | §1 panel close = close + cart intact (no order exists pre-confirm) **+ RESOLVE/F6** browser-Back stays on storefront; **RESOLVE/F1** dispatch keeps order CONFIRMED on no-courier + IN_DELIVERY recovery branch (no orphan); §5 no new edge; deliver-v2 folds intact. |
| Embed/normal merge (no regression) | §1 **removes** the only divergence by making checkout use the same bottom-sheet in both modes. |
| i18n (al/en parity) | Any new/changed strings (count-only bar, "Send for delivery", claim page, Art-14 notice) need al/en keys via `scripts/i18n-add.ts` + parity gate. |

## Cross-cutting: what needs a flag vs UI-only

- **UI-only (no flag strictly needed, but recommended for staged rollout):** §1 panel, §2 count-only
  bar, §5 OrderCard collapse.
- **Backward-compatible contract change (no flag):** §4 `.default('delivery')`.
- **Behind activation/ops gate (already dark):** §6 claim surface (gated by `PROVISION_OPS_SECRET` +
  migs 068–071, per shipped P6).
- **Recommended flags:** `FLOW_SIMPLIFIED_CHECKOUT` (§1/§2/§4-UI), `OWNER_TWO_TAP` (§5).

## Cross-cutting: guardrails that MUST go red→green (RESOLVE round added the no-trap + transport set)

The RESOLVE round (`resolution.md`) escalated this from one guardrail to a set — the two HIGHs and the
customer-honesty MEDs each need a deterministic red→green proof:

| ID | Proves | Red today because |
|---|---|---|
| §4 requestHash | omit-`type` and send-`type:'delivery'` hash identically (no idempotency split) | (new correctness guard) |
| **G-F1a** (RESOLVE-2) | PATCH status→IN_DELIVERY with no courier returns `{status:'CONFIRMED',dispatched:false,reason:'no_courier'}` + DB row stays CONFIRMED | `orders.ts:779` flips first; `:824` returns `{status:'IN_DELIVERY'}` (silent orphan) |
| **G-F1a-2** (RESOLVE-3) | PATCH→IN_DELIVERY on an already-bound (`'offered'`/active) order → no 500, no 2nd INSERT; `offer_pending`/`already_assigned` signal, row unchanged | `orders.ts:792–794` lookup `'offered'`-blind → conflicting INSERT 23505 (`…073:22–24,32–33`) |
| **G-F1b-i** (RESOLVE-3) | IN_DELIVERY + `accepted` binding → owner recovery → READY + binding cancelled + re-offerable | `OrderCard.tsx:221–236` has no IN_DELIVERY branch |
| **G-F1b-ii** (RESOLVE-3 / R3-4) | IN_DELIVERY + `picked_up` binding → owner recovery → **CANCELLED, not READY** | central fold `orderStatusService.ts:129–140` blanket-cancels picked_up → food-out lie on a raw revert |
| **G-F1c** (RESOLVE-3) | CONFIRMED+awaiting → owner re-tap once a courier is online → IN_DELIVERY + assignment (loop closes) | §5 path never enqueues; no re-dispatch path was specified |
| **G-F2a/b/c** | claim token never in URL/hash/Referer/query; body-only | a `/claim?token=` surface would leak it |
| **G-F2d/e/f** (RESOLVE-2) | recipient email-match active (CONTACT_MISMATCH 403); claim completes with no navigation + no storage; scrub before telemetry init | invite mint must bind contact; SPA must fetch-auth in-page |
| **G-F2g** (RESOLVE-3 / R3-1) | web claim (preview/accept) against a NULL-hash invite → refused (generic), `owner_id` stays NULL — token-only theft unreachable via web | `…071:64–68` permits NULL-hash bind to any authed user; `claim_transfer` untouched |
| **G-F3** (RESOLVE-3 corrected) | ETA decays off `COALESCE(preparing_at,confirmed_at,created_at)` (≈13 not 0/flat-15; incl. NULL-`confirmed_at` → `created_at` fallback) | `etaGather.ts:81–86,192` flat for CONFIRMED; `confirmed_at` not selected/passed |
| **G-F5** | progress Ready dot not a false ✓ when `ready_at` NULL; "Preparing" is a process label | `OrderProgress.tsx:84–102` fills by statusIndex |
| **G-§2** (RESOLVE-2) | bar shows a labeled subtotal (not a bare total); fee surfaces on address-resolve, not at confirm | bar copy + fee-sequencing unspecified |
| **G-F6** | browser Back closes panel, stays on `/s/:slug` | panel not a history entry today |
| **G-F9** | empty-cart deep-link → closed panel, no strand | reconcile trigger unspecified for the new seam |
| **G-PF1/G-PF2** (RESOLVE-2) | `published_at` NULL through claim; allergen-confirm a distinct act into empty fields | consent gate is prose-only until annotated in code |

Everything else is covered by existing checkout/owner E2E + the visual-regression net + the shipped
P6/claim tests. **Full disposition + re-grounding: `resolution.md`.**
