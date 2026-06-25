# DeliveryOS — UI Improvements Spec (MVP wave from the teardown research)

> Full spec of the UI/UX improvements shipped from the OSS-teardown research + Bolt teardown,
> filtered through the Triadic Council (`COUNCIL-VERDICT.md`). Built by 4 parallel verticals,
> all **additive / forward-only**: the 10-state machine, integer-money, RLS, and existing
> contracts are untouched. Companion: `UI-IMPROVEMENTS-TESTPLAN.md` (Playwright plan).
> Branch `feat/product-media-seam`; commits `c9f47b6a` (verticals) + this wave (testid/toggle fixes).
>
> **Principled exclusions** (you asked "not breaking the main systems"): OrderTotal ledger
> (restructures money), `order_status_history.notify` (couples to the unbuilt Telegram subsystem).

Source lens: white-label **per-restaurant** (`/s/:slug`), Albania, 77% cash, COD MVP,
brand = logo+colour (every surface reads `var(--brand-*)`, survives `derivePalette`).

---

## 1. Owner new-order alert (Bolt §C/§F4) — module: orders/ui · MVP

**Problem (verified):** `useSound` did `new Audio().play()` on a WS event (not a user gesture) and
swallowed the autoplay rejection → **silent on iOS Safari**; one-shot, not persistent. An owner who
thinks they'll be alerted and isn't = a lost order in a cash market.

**Shipped** (`apps/web/src/lib/hooks.ts`, `apps/web/src/pages/admin/DashboardPage.tsx`):
- **Audio-context unlock on the first user gesture** (pointerdown/keydown/touchstart, `{once}`);
  `armed` flips true only on a real `play()` success.
- **Persistent ping** — loops every ~4s while an unacknowledged PENDING exists; stops on
  accept/reject/acknowledge (natural pruning when the order leaves PENDING).
- **Honest state** (council gate: *a silent false promise of an alert is worse than no alert*):
  `owner-alert-status` (armed) vs `owner-alert-enable` (muted/blocked, one-tap unlock) + a visible
  persistent `owner-new-order-banner` whenever audio can't carry the alert → a missed order is
  impossible even with sound blocked.

**Acceptance / testids:** `owner-alert-status[data-state=armed]`, `owner-alert-enable[data-state=
muted|blocked]`, `owner-new-order-banner`. On load (no gesture) → enable shows `blocked`; after a
click → `armed`; a new order while blocked → banner visible.

## 2. Order-status stepper (Bolt §A7/§F1) — module: orders/ui · MVP

**Problem (verified):** `OrderProgress` hard-coded 5 steps, **dropped CONFIRMED**, had **no pickup
branch** → it misrepresented pickup orders. `ready_at` was never written; only `confirmed_at`/
`delivered_at` were stamped; the customer endpoint returned only `status`+`created_at`.

**Shipped:**
- **Stepper rewrite** (`packages/ui/.../client/OrderProgress.tsx`): the real 10-state machine —
  CONFIRMED included; **delivery branch** (`…IN_DELIVERY→DELIVERED`) vs **pickup branch**
  (`…READY→PICKED_UP`) chosen off `type`; terminal styling for `REJECTED`/`CANCELLED`. Honest with
  status-only; lights filled steps + shows per-step times when timestamps are present.
- **Additive backend instrumentation** (migration `059`): nullable `orders.{preparing_at,
  in_delivery_at,picked_up_at}` (+ `ready_at`); `updateOrderStatus` ALSO stamps each transition
  (guard/transition logic untouched — verified zero-diff on `order-machine.ts`); exposed on
  `customer/orders` + the WS delta + the customer contract (additive optional). `order_status_history
  .comment` added; **no `notify`**.

**Acceptance / testids:** `order-progress[data-order-type]`; `order-step-{pending,confirmed,
preparing,ready,in_delivery,delivered,picked_up,rejected,cancelled}[data-active]`;
`order-step-<key>-time`. Pickup order shows the PICKED_UP branch, not IN_DELIVERY/DELIVERED.

## 3. Couriers day-one (Bolt §B/§F7) — module: couriers/ui · MVP

**State:** the loop already works — `DeliveryPage` GPS heartbeat (delivery-scoped, privacy-guarded)
→ `courier_positions` → `courier-events` → `order.courier_updated` → the customer map pin; offer
accept/reject + active-delivery advance + tap-to-call all exist.

**Shipped (the §F7 gap):** an **offer countdown timer** on `TaskCard` — a shrinking bar + seconds
pill; on expiry **auto-declines** (`onReject` → back to dispatch). Testids added:
`courier-offer-timer[data-remaining]`, `task-accept`, `courier-offer-decline`, and
`courier-advance-action` (wrapping the swipe-to-deliver).

**Acceptance:** an offered task shows the timer counting down; at 0 it auto-declines; accept →
`/courier/delivery/:id`; the active screen advances via real endpoints + emits GPS so the customer
pin moves.

## 4. Menu availability + top-steals — module: menu/locations/ui · MVP

**4a. Venue `busy` (Bolt §A3/§F3) — gap: the client collapsed status to `isOpen===false`, so `busy`
never reached the eater.** Shipped: `/info` emits `status: open|closed|busy` (busy = open AND
`now() < kitchen_busy_until`); MenuPage shows a distinct `venue-busy-banner` vs `venue-closed-banner`
+ a header `venue-state-chip`. **Owner toggle** added (`KitchenBusyToggle`, `kitchen-busy-toggle`)
— reads honest initial state from `/info`, PATCHes a 30-min window / clears. (Completes the loop:
storefront *shows* busy; owner *sets* it.)

**4b. `StateChip`** (`packages/ui/.../client/StateChip.tsx`) — open/closed/busy (venue) +
available/sold-out (item); brand-token-native. Used in the venue header (`venue-state-chip`) and on
sold-out cards (`item-state-chip`) alongside the existing greying.

**4c. Modifier `display_type` (top-steal — TastyIgniter `MenuOption.display_type` / Enatega) —
additive.** Migration `060`: nullable `modifier_groups.display_type` CHECK(`radio|checkbox|select|
quantity`); surfaced through `read_public_menu` (migration `063` — a **verbatim copy of the live 055
def + only additive hunks**, signature `p_locale text DEFAULT ''::text` preserved), the public
contract (optional), the client renderer (`modifier-group[data-display-type]`, explicit instead of
inferred from `max_select`), and the admin modifier editor.

**4d. Mealtime / SCHEDULE engine (top-steal — TastyIgniter `Mealtime`) — additive.** Migration `062`:
new `menu_schedules` table (mode `daily|recurring|period` + window fields, nullable product/category
target, one-target CHECK) with **FORCE RLS + `tenant_isolation` keyed on `location_id`** (mirrors
`product_media`); SQL fns `menu_schedule_matches` + `product_available_now(product,category,tz)`,
**timezone-aware** (uses `locations.timezone`), AND-combined with `is_available` in `read_public_menu`
— **a product with no schedule is always available** (purely additive). Owner editor
`MenuScheduleEditor` (`schedule-editor`) + CRUD API (`owner/menu-availability.ts`). DB-proven:
breakfast 09:00 true / 20:00 false; recurring Mon true / Tue false; period in/out window; RLS
cross-tenant insert blocked by WITH CHECK.

---

## Deployment + test status (against `https://dowiz-staging.fly.dev`)

Migrations `059–063` applied via `release_command` (read_public_menu `063` serves cleanly).
Per `UI-IMPROVEMENTS-TESTPLAN.md` the planner **verified live**: `venue-state-chip[data-state=open]`
on `/s/demo`; `owner-alert-enable[data-state=blocked]` → arms on click → `owner-alert-status`;
`schedule-editor` on `/admin/menu`. **6 cases GO now**, the rest **blocked on seed data** (a busy
window, a sold-out item, a placed order for the stepper, a modifier group for `display_type`, a
courier session + dispatched offer). Auth: `/login` (not `/admin/login`), `test@dowiz.com /
test123456`; `/api/dev/mock-auth` is 404 on staging → use `/api/auth/local/login`.

**To unblock the rest of the suite (seed fixtures on the demo location):** PATCH a product to
`available:false` (item-state-chip); PATCH kitchen-busy (busy banner); attach a modifier group with
a `display_type` (modifier-group); place a storefront order + advance it (stepper); create a courier
session + dispatch (offer timer). The stepper is best proven by an **end-to-end order placement**
spec (no DB seeding needed — drive `/s/demo` → cart → checkout → status).

## Acceptance summary (Definition of Done for this wave)
- ✅ All additive: 10-state machine zero-diff, integer money untouched, RLS FORCE preserved + new
  tables FORCE-RLS, contracts additive-only (`.strict()` valid).
- ✅ Every surface brand-token-native (no hardcoded colours — `local/no-hardcoded-color`).
- ✅ Full typecheck green (12/12); migrations applied + RLS-enforced on throwaway PG; deployed staging.
- ✅ Council ethical gates honoured (honest alert state; no money client-side; no `notify`; RLS additive).
- ⏳ Playwright: 6 GO cases automatable now; the rest gated on seed fixtures (plan in the companion).
