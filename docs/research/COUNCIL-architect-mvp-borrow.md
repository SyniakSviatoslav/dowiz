# COUNCIL — ARCHITECT seat: what DeliveryOS can borrow/adapt RIGHT NOW

> Deliberation only. apps/, contracts, Zod, migrations are READ-ONLY here. This is a design
> recommendation, grounded against the live tree (not the SYNTHESIS's assumptions).
> Every "exists/gap" claim below was verified by reading the actual file:line.

## TL;DR for the council

The single most important finding: **the SYNTHESIS systematically understates how much already
ships.** Most S5 "MVP ADAPT" items the prompt asked me to prioritise are **already built** in the
current tree — CartFAB-with-total, the modifier bottom-sheet UI, item-level 86 toggle + stop-list
filter, sold-out greying, courier accept/reject, owner accept/reject + prep-time display. The
genuine do-now surface is **much smaller** than S5 implies. Ponytail discipline says: do not
re-build what exists; close the two real schema gaps only when a consumer needs them, and fix the
**three real UI gaps** (stepper coverage, venue `busy` surfacing, owner audible alert).

---

## 1. Do-now shortlist (verified, sequenced)

Legend: **NEW** = genuinely additive · **SURFACE** = wire something that already exists ·
**DONE** = already shipped, no work (listed so the council does not re-authorise it).

### Already DONE — do NOT re-build (evidence)

| S5 item | Status | Evidence (file:line) |
|---|---|---|
| #21 persistent CartButton-with-total | **DONE** | `packages/ui/src/components/client/CartFAB.tsx` — fixed bottom-right, item-count badge + `<PriceDisplay amount={total}/>` (whole file, 41 lines) |
| #21 modifier bottom-sheet + live "Add €X" | **DONE** | inline in `apps/web/src/pages/client/MenuPage.tsx`: `ModifierGroup` iface L24-31, selection state L112-113, radio-vs-checkbox inferred from `max_select===1` L404, live delta `calcModifierDelta` L419-429. `BottomSheet` primitive exists at `packages/ui/src/components/molecules/BottomSheet.tsx` |
| #3 86 / stop-list in owner UI | **DONE** | `MenuManagerPage.tsx`: `handleToggleAvailable` L229-235 (PATCH `available`), "Stop-listed" filter L456/L468, per-product toggle |
| #25 item-level sold-out greyed-not-deleted | **DONE** | `ProductCard.tsx` L56 `opacity-55` when `!isAvailable`, hover/tap disabled L60-63 |
| #23 courier single-action accept/decline | **MOSTLY DONE** | `courier/TasksPage.tsx`: `handleAccept` L70, `handleReject` L83-92, WS new-assignment listener L51-60, `TaskCard` single-action; `SwipeToComplete` for delivery handoff in `DeliveryPage.tsx` |
| owner accept/reject + prep-time *display* | **MOSTLY DONE** | admin `OrderCard.tsx`: Accept→CONFIRMED L196, Reject→CANCELLED L197, prep-delta display L67-100 |
| #7 denormalised order-line snapshot | **DONE** | `order_items(name_snapshot,price_snapshot)` + `order_item_modifiers(name_snapshot,price_delta_snapshot)` (`1780338982010_menu_modifiers.ts` L32-38) |
| #9 `*_at` timestamps exist | **DONE (partial)** | `confirmed_at` (`1780310074262_orders.ts` L42), `ready_at`/`delivered_at` (`1780695000000_order_timelines.ts` L5-6) |
| #1 two-tier modifier schema | **DONE** | `modifier_groups(min_select,max_select,required)→modifiers(price_delta,available)→product_modifier_groups` (`1780338982010` L5-30) |

### Genuinely actionable NOW (the real shortlist)

| # | Item | Kind | Effort | Touches (verified) | Why now |
|---|---|---|---|---|---|
| **D1** | **OrderStatusStepper: extend `OrderProgress` to the real machine** | SURFACE/edit | **S** | `packages/ui/src/components/client/OrderProgress.tsx` (58 lines); consumer `OrderStatusPage.tsx` L436 `<OrderProgress status={order.status}/>` | The current stepper hard-codes 5 steps `PENDING·PREPARING·READY·IN_DELIVERY·DELIVERED` (L7-12) — it **drops CONFIRMED and has no PICKED_UP/pickup branch**. The real machine (verified `packages/domain/src/order-machine.ts`) has 10 states incl. `READY→PICKED_UP`. This is the highest-ROI correctness fix: the retention screen lies for pickup orders and for the confirmed state. Add CONFIRMED to happy path + a pickup branch variant. |
| **D2** | **Surface venue `busy` (and `closed`) state on the storefront** | SURFACE | **S** | `apps/web/src/pages/client/MenuPage.tsx` L257/L273/L634 (only consumes `isOpen` boolean today); contract already exposes it: `packages/shared-types/src/contracts/public/menu.ts:53` `status: z.enum(['open','closed','busy'])` | Contract already carries the 3-level state; client collapses it to `isOpen===false` (closed banner only, L634). `busy` (the operator-honesty signal, teardown §F3) never reaches the eater. Thread `status` through `LocationInfo` and render a `busy` chip + closed banner. NEW UI surface, zero schema/contract change. |
| **D3** | **Owner audible + persistent new-order alert** | NEW | **M** | new hook in `packages/ui/src/hooks/` + wire in `apps/web/src/pages/admin/DashboardPage.tsx`; OrderCard already has accept/reject | The one fully-missing owner primary-screen affordance (teardown §F4). `grep` for `AudioContext|\.play()|ring|beep` across `packages/ui/src/components/admin` + DashboardPage = **zero hits**. Web Audio loop on new `PENDING`, unlocked on first owner tap (iOS audio-context). Must be a hook (reusable, testable) not inline. |

**Dependency order:** D1 and D2 are independent (different surfaces) — parallelisable. D3 is
independent of both but heavier; sequence it after D1/D2 land since it needs an iOS-unlock E2E
proof and a real new-order signal on the dashboard.

### Schema GAPS — defer until a consumer exists (see §3 for why)

Both confirmed real against the migrations, but **neither has a consumer that needs it today**:

- **`modifier_groups.display_type`** (S5 #2) — confirmed absent (`1780338982010` L5-13, only
  `min_select/max_select/required`). **But the UI already infers radio-vs-checkbox correctly**
  from `max_select===1` (`MenuPage.tsx` L404) and the SYNTHESIS S3 itself tags it "gap (small)".
  YAGNI: adding the column changes no behaviour until a `quantity`/`select` display type that the
  min/max inference *cannot* express is actually needed. Defer.
- **`order_status_history.{comment,notify}`** (S5 #8) — confirmed absent (`1780338982015` L5-13:
  `from_status,to_status,actor,created_at` only). These exist **to feed the Telegram per-category
  notification design** (memory: telegram-notifications-council). That design's code gate is OPEN
  but unimplemented. Adding columns now = dead columns. Add them **as the first step of the
  Telegram-notif build**, not speculatively. Forward-only `ADD COLUMN ... NULL` when that lands.

---

## 2. The one license-clean code BORROW (R3 MERN MIT: RHF+zod Form + cart scaffold)

**Recommendation: do NOT lift it. Reimplement-not-borrow (downgrade S5 #18/#19 from BORROW to SKIP-copy).**

Grounded reasons:

1. **The cart scaffold (#19) is moot** — the cart UX already ships: `CartFAB.tsx` (sticky total),
   `CartDrawer.tsx`, and a real relational `CheckoutPage.tsx` with server-computed totals and the
   single COD path (`payment_method='cash'`). R3's value was "strip Stripe, wire COD" — we already
   have no Stripe and a working COD checkout. Lifting a MERN cart would be a net *regression*.
2. **The RHF+zod Form recipe (#18) buys little** — the owner editors already exist and work
   (`MenuManagerPage.tsx` uses local `useState` form state + `apiClient` PATCH, not RHF). Introducing
   `react-hook-form` is a **new dependency** for forms that already function. Ponytail: no new dep to
   replace working stdlib-equivalent code.
3. **Attribution cost** — any literal lift obliges carrying the MIT NOTICE / third-party-licenses
   entry. For two small recipes we'd reimplement anyway (port v3→v5, restyle to our tokens), the
   attribution overhead exceeds the saved keystrokes.
4. **Stack drift risk** — R3 is TanStack v3 + its own shadcn vendoring; we are v5 + our own
   `packages/ui`. A "port" is a rewrite in disguise.

**Net:** the only license-clean BORROW in the whole synthesis evaporates on contact with the real
tree — both its targets are already built. Keep the *idea* (colocated zod schema + typed form), apply
it the next time a **new** non-trivial owner editor is written, using our own `Input`/`Button` atoms
and a plain `zod.safeParse` — no `react-hook-form` dependency, no code lift, no NOTICE entry.

---

## 3. Explicit NOT-now list (challenge me)

| Item | S5 tag | Defer reason |
|---|---|---|
| `modifier_groups.display_type` | #2 MVP | No consumer: radio/checkbox already inferred from `max_select` (`MenuPage.tsx` L404). Column would sit unread. Add only when a `quantity` modifier (e.g. "3× extra shots") needs an explicit type the min/max can't express. |
| `order_status_history.{comment,notify}` | #8 MVP | Dead columns until the Telegram-notif worker consumes them. Bundle into that build (its gate is open, impl pending) as forward-only `ADD COLUMN NULL`. |
| RHF+zod Form BORROW + cart scaffold | #18/#19 MVP | Both targets already shipped (§2). New dep for no gain. |
| shadcn primitives lift | #17 | Already vendored in `packages/ui`; verify-only, never lift. |
| Mealtime/schedule engine | #4 later | `SCHEDULED` is scaffold (`order-machine.ts` L27,33 + `ScaffoldDisabledError`). Outside COD MVP. |
| Per-item modifier price override (4th table) | #5 SKIP | YAGNI; single `modifiers.price_delta` suffices. |
| Totals ledger rows | #10 SKIP | Fixed integer columns (`money_breakdown`) suffice for COD; revisit on coupons. |
| PaymentProvider seam | #12 later | Single COD path live; capture interface only when a real PSP appears. |
| Cancellation-ordering workflow | #15 later | Cancel legal only from `PENDING` today (`order-machine.ts` L19) — the complex fulfilment→settle→cancel ordering doesn't arise yet. |
| Live courier pin + ETA refinement | #24 MVP-tagged | **Partially done** (`CourierLiveMap`, `ETADisplay`, `useDeliveryEta` all exist + wired in `OrderStatusPage`/`DeliveryPage`). Not a *gap* — at most a throttling tune. Not a do-now build. |
| Courier offer **timer/countdown** | part of #23 | Accept/reject exist; a visible decline countdown is a polish add, not MVP-blocking. Defer behind D1-D3. |
| Owner prep-time **picker/limiter** | part of #22 | Prep-time *display* exists (`OrderCard.tsx` L67-100); a setter/limiter is enhancement, not the missing alert. The alert (D3) is the real gap. |

---

## 4. Recommended 1–2 stage sequence (additive, forward-only)

**Stage A — "the storefront/tracking tells the truth" (UI-only, zero schema/contract change).**
Cluster the two cheap, independent, high-ROI surfacings:
- **D1** extend `OrderProgress` to cover CONFIRMED + the `READY→PICKED_UP` pickup branch (read the
  real machine; `isTerminal` for REJECTED/CANCELLED already handled L20). Consumer unchanged
  (`OrderStatusPage.tsx` L436 still passes `status`).
- **D2** thread venue `status` (`open|closed|busy`) from the already-existing contract field into
  `MenuPage` `LocationInfo` and render a `busy` chip alongside the closed banner.
- Unblocks: nothing depends on these; they ship the §F1/§F3 honesty conventions with the least code.
- Proof (Mandatory Proof Rule): one Playwright spec each against staging — pickup order shows a
  PICKED_UP-terminated stepper (`toBeVisible` on the pickup step); a `busy`-status venue shows the
  busy chip. Both are real-DOM assertions on `/s/:slug` and the order-status surface.

**Stage B — "the owner can't miss an order" (one new hook + wiring).**
- **D3** audible+persistent alert hook in `packages/ui/src/hooks/`, unlocked on first owner
  interaction (iOS audio-context), fired on a new `PENDING` arriving via the existing dashboard WS.
  Reuse the existing accept/reject buttons in `OrderCard` — no new order-action surface.
- Sequenced second because it's the heaviest (Web Audio + iOS-unlock E2E is fiddly) and benefits
  from D1 landing first (the dashboard order list it hooks into is the same one feeding the stepper).
- Proof: Playwright owner spec — simulate a new PENDING order, assert the alert affordance becomes
  visible and the unlock control works; plus a unit test on the hook's unlock/loop logic.

**Schema work:** intentionally *not* in Stage A/B. `display_type` and
`order_status_history.{comment,notify}` ride in only when their consumer (a quantity-type modifier;
the Telegram-notif worker) is actually being built — forward-only `ADD COLUMN ... NULL`, no money /
state-machine / contract breakage, no migration churn ahead of need.

**No contract/money/state-machine touch in either stage.** D1 reads the existing machine, D2 reads
an existing contract field, D3 reads existing order data. All three are pure presentation/UX —
exactly the "convention, not framework" minimum.

---

## Architect's bottom line

The teardown's strategic conclusion (DeliveryOS schema already matches the strongest external
evidence; the niche lets us *cut* marketplace surfaces as a feature) is **correct and confirmed** —
but its MVP *build* list is stale relative to the tree. The honest do-now is three small UI fixes
(D1 stepper coverage, D2 busy surfacing, D3 owner alert), zero code-BORROW, and two schema columns
held back until their consumers exist. Anything more is re-building shipped code or pre-building for
absent consumers — both violate ponytail. I yield to BREAKER to attack D1's pickup-branch ambiguity
and D3's iOS-unlock failure mode, and to COUNSEL on whether deferring the two columns risks
fragmenting the Telegram-notif build.
