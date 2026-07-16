# BLUEPRINT — Phase 9a: ORDER-CRITICAL PRODUCT SURFACE (the G11 fast-path)

> **The most consequential blueprint in the living-interface arc.** Phase 9a is the *actual* G11
> deliverable: a real customer places and completes a real order on the new interface. The operator's
> 2026-07-16 ruling (`LIVING-INTERFACE-ROADMAP.md` §8) selected the commercial-delivery-first charter
> and split the old catch-all "Phase 9" into **9a (order-critical, on the G11 fast-path)** and **9b
> (full owner/admin + multimodal + cross-platform)**. This document sequences and integrates the
> already-blueprinted DZ-07/08, RW-06/07/08, RW-02/03/12+FE-17, and FE-15/DZ-11 work units into the
> shortest chain to a working order. It **re-blueprints nothing** — those items keep their decided
> content — and it writes/edits **no product code**.
>
> **Depends on:** Phase 6 only (transitively 3, 4, 5) — verified in roadmap §8.2: no order-critical
> item reads Phase 7 (sonification) or Phase 8 (memory-viz). **Fast path to G11:**
> `0 → 1 → 2 → 3 → 4 → 5 → 6 → 9a`.
> **Acceptance authority:** the `/reliability-gate` skill's L0–L11 order-lifecycle trace, re-pointed at
> the new interface (§7). This blueprint grounds every acceptance item in exactly what that gate
> checks; it invents no parallel verification scheme.
> **Sources read in full:** roadmap §1/§4/§8; DZ-07/08/11; RW-02/03/06/07/08/12; FE-15/17;
> `.claude/skills/reliability-gate/SKILL.md`; `DeliveryOS-As-Built-Summary-v1.md`;
> `docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P16-product-ui-rebuild.md`. Load-bearing
> current-state facts re-verified against the live tree 2026-07-16.

---

## 1. Current-state evidence (what exists, what remains — cited)

### 1.1 The deletion finding, re-confirmed

Phase 16's framing holds at HEAD. `git ls-files apps/web` and `git ls-files packages/ui` both return
**0 files**; `apps/web/` on disk holds only stale `dist/` + `node_modules/`. The commits
`79ef316f6` + `db766de47` (2026-07-13) deleted `apps/web`, `packages/ui` (incl. all i18n),
`packages/domain`, `packages/shared-types`; `fce5738b0` quarantined `apps/api`, `apps/worker`,
`packages/db` into `attic/`. **Consequence for Phase 9a: this is a REBUILD with feature-inventory
preservation, not a port** — exactly the framing P16 §0 established. The two DZ-07/08 screens are
built fresh on the canonical `web/` shell against kernel wasm exports; the legacy React sources named
by RW-02/03/12/FE-17 exist only in the untracked/attic tree and are therefore build-forward
*constraints*, not files to edit (see §5).

### 1.2 The RW-06/07/08 kernel ports — what already exists vs. remains

The task flagged an earlier note that `geo.rs` was "~70% ported." That note is now **stale**; the
pure math is complete. Precise status, verified in `kernel/src/`:

| Port | Kernel authority | WASM bridge (`wasm.rs`) | Tests | Remaining for 9a |
|---|---|---|---|---|
| **RW-06 geo** | `geo.rs` (351 LOC): `haversine_meters`, `lerp_lat_lng`, `bearing_deg`, `ema_next`, `polyline_length_meters`, `progress_along_route`, `eta_seconds`, `is_out_of_order`, `should_snap`, `is_arriving`, `point_in_polygon` (delivery-zone ray-cast) | **DONE** — 10 `geo_*_js` exports (`wasm.rs:474–599`): haversine/lerp/bearing/progress/progress_flat/eta/should_snap/is_arriving/point_in_polygon/is_out_of_order | present (`haversine_london_paris`, `eta_basic`, `point_in_polygon_square`, …) | **None in kernel.** Only the thin rAF marker/ETA *consumer* remains — and it lived in deleted `apps/web`, so it is new UI build in DZ-07 Track / DZ-08 Delivery, not a port. |
| **RW-07 cart** | `cart.rs` (`Cart` state machine: `add`/`remove`/`clear`/`item_count`/`price(inject unit_price)`/`reconcile`; `CartLine`/`PricedLine`; `format_money`) | **MISSING** — grep for `cart` in `wasm.rs` = 0 | present (`add_dedupes_and_removes`, `total_via_integer_money`, `reconcile_drops_drifted`) | **Add `cart_*_js` exports.** Both legacy impls (apps/web `CartProvider` + packages/ui `use-cart`) already deleted, so "DELETE one of two" is a no-op; the thin localStorage/cross-tab shell is new UI. |
| **RW-08 messenger** | `messenger.rs`: `normalize_phone`, `telegram_link`, `whatsapp_link`, `viber_link` | **MISSING** — grep = 0 | present (`normalize_phone_strips_formatting`, `telegram_link_clean`, `whatsapp_link_parity`, `viber_link_parity`) | **Add `messenger_*_js` + `format_money_js` exports.** `formatMoney` landed as `cart.rs::format_money` + `money.rs::convert_all_to_eur_cents`/`format_all` (`money.rs:142`). |

**The server-pricing gap (load-bearing).** `place_order_js` (`wasm.rs:176–281`) today accepts
client-supplied `unit_price: i64` on each item (`wasm.rs:56`, passed through at `:122`/`:131`). The
reliability-gate's L2 PASS criterion is *"server recomputes total from `products` table — no client
total trusted."* The authority to close this exists: `catalog.rs::PriceCatalog::unit_price(product_id,
&modifier_ids)` and `cart.rs::Cart::price(|p| …)` take an injected price function. **Wiring
`place_order_js` (and cart totals) to price through `catalog.rs` instead of trusting the client
`unit_price` is a hard prerequisite of the checkout screen** — it is the single kernel change that
makes L2 honest. This is called out as a sequencing gate in §4.

Other order-path exports already present in `wasm.rs` (24 `_js` total): `place_order_js`,
`apply_event_js` (FSM transitions), `estimate_order_total_js` (subtotal→fee/tax/total),
`fsm_graph_report_js`, the `geo_*_js` family, the `spectral_*_js` family.
`order_machine.rs` supplies `OrderStatus`, `assert_transition`, `fold_transitions`, `allowed_next` —
the FSM is the kernel's, never a JS chart.

### 1.3 Reconciliation with the sovereign-roadmap Phase 16 blueprint (verdict up front)

**Verdict: COMPLEMENTARY, not redundant, neither supersedes.** P16 (`BLUEPRINT-P16-product-ui-rebuild.md`)
is the *horizontal* rebuild — **all 26 pages** (3 client + 7 courier + 16 owner) + i18n recovery
(sq/en/uk, ~1291 keys) + WCAG-AA + responsive matrix, on a different roadmap (the R2 19-phase
sovereign chain), depending on Phase-4 `field_frame::compose` + Phase-13 delivery spine. **Phase 9a is
the *vertical* G11 slice through it**: CLIENT (DZ-07) + COURIER (DZ-08) only, sequenced for the fastest
path to one completed order, owner/admin + i18n-as-gate + full WCAG buildout deferred to 9b.

The two are two projections of the same rebuild. Phase 9a **inherits, does not rebuild**, P16's shared
machinery:
- the **`RECONCILIATION-LEDGER.md` methodology** (P16 §2) — 9a populates only the client + courier rows;
- the **wasm-math grep gate** (P16 §3, `scripts/ci-no-client-math.sh`) — the vehicle that operationalizes
  RW-02/03's "no JS re-implements money/geo/FSM math";
- the **Sea & Sheet architecture** (P16 §4) + **hybrid-DOM a11y** (P16 §6 = DZ-11/FE-15, see §6);
- the two oracles: `DeliveryOS-As-Built-Summary-v1.md` (92 tests ×3 breakpoints; client 3 screens,
  courier 12 tests/screens, cash cycle, OTP layer) + the DZ per-screen master checklists.

Where they **diverge is only the acceptance mechanism**: P16 item 5 is a broad Playwright E2E
(*client places → courier PoD → owner settlement*, over a real Phase-13 mesh hub, incl. PoD signing +
payout); **Phase 9a uses the narrower, sharper `/reliability-gate` L0–L11 order-lifecycle trace** — the
exactly-once/recoverable/cross-surface proof that is precisely what "the first real order actually
completed" needs. The reliability-gate is a strict subset of P16 item 5 (it omits PoD-signing +
payout-over-mesh). **Non-duplication rule:** Phase 9a does not author a second ledger, a second
wasm-math gate, or a second Sea&Sheet spec — it scopes P16's to client+courier and swaps P16's broad
E2E for the reliability-gate as the done-test. If both blueprints are executed, 9a lands first (G11);
P16's owner/i18n/WCAG remainder is 9b-and-later work.

---

## 2. DZ-07 CLIENT — screen-by-screen build sequence

Four screens on the DZ-01 three-act shell (arrive → choose → receive), each a URL state with working
back-navigation, the Sea beneath and the Sheet above. Build order follows the L-stage order so each
screen's kernel prerequisite is green before it is built.

**S1 — Menu (`/s/:slug`, Act 2) → covers L0.** SSR real DOM (stays DOM per DZ-11 — never migrates;
public SEO + native screen-reader). Cache headers ≤60s + `menu_version` (L0 PASS). Sheet renders hero
(Google rating/reviews, geo ETA via `geo_eta_js`, `StateChip` open/closed/busy), category tabs +
scroll-spy, Chef's Picks, search/sort(price/protein/kcal)/allergen-filter persisted; the item grid
**SPREADs by diffusion from the tap** (not pop). Closed store → Sea calm dark. The Sea under is the
static-gradient/reduced-motion fallback until Phase-4 `compose` blits the live field (P16 §4 dependency
honesty). *No money computed here.*

**S2 — Detail (bottom-sheet / modal over Menu) → feeds L1.** Rich media (ADR-0002 lazy+gated), reveal =
Green's bloom, kcal/macro/taste-axes/allergen-list/ingredients, **modifier groups** (radio/checkbox/
select/quantity with req/min/max price-deltas), qty stepper. Add-to-cart writes to `cart.rs` via the
new `cart_*_js` exports; the live line + cart total render `<Money>` from a **kernel** integer (never a
tween), plus toast + cart ripple + haptic. *Prerequisite: `cart_*_js` exports (§4).*

**S3 — Checkout (`/s/:slug/checkout`, progressive disclosure) → covers L1→L2.** Delivery/pickup tabs;
contact (name/phone Albanian→E.164 via `normalize_phone`); messenger deep-link via `messenger_*_js`;
entrance photo → R2 (local-first outbox, uploaded when online — DZ-06); `MapWithPin` + My-Location
(pin/zone via `geo_point_in_polygon_js`); cash amount+change+tip; **summary** (subtotal/fee/tax/tip/
`<Money>` total via `estimate_order_total_js`); draft persist. **Place order = `place_order_js`
(idempotency key + OTP send/verify/intent-hash + full error matrix + fallback phone).** This is the L2
node: the order POST is one transaction, idempotent, **server-priced** (see §4 — `place_order_js` must
price through `catalog.rs`, not the client `unit_price`). The contact/OTP/address fields are **real DOM
`<input>` overlays** (FE-15, §6) so IME/autofill/mobile-keyboard/`type=tel` all work. This is the
riskiest screen: it carries the exactly-once and server-pricing threads.

**S4 — Track (`/s/:slug/track?t=…`, Act 3, Sea matures) → covers L7–L11 client surface.** DZ-04
OrderStatus→Море (amplitude jump + terracotta→gold as status advances; illegal transition → red recoil
validated locally via the kernel FSM, **no server round-trip**). Status fetch + live WS (route /
courier-position / status **terminal-lock** / message) + 30s watchdog; `CourierLiveMap` tweened+rotated
marker = the geo field flow (`geo_lerp_js`/`geo_bearing_js`/`geo_should_snap_js`); honest ETA range;
`OrderProgress` stepper (delivery/pickup branch, `fsm_graph_report_js`); terminal CTAs; **rating +
feedback + Google-invite** (StarRatingBlock, `canSubmit`-gated — L9); `MessageThread`; offline banner.
`aria-live` announces each status change (§6). Track is where the client sees DELIVERED (L8) and where
the exactly-once client surface of the L11 matrix is proven.

**Feature-preservation gate (DZ-07):** every row of the client master checklist has a REBUILT
disposition in the reconciliation ledger (modifier groups, OTP + full error matrix, WS tracking + 30s
watchdog, local-first cart, `<Money>` snap). Silent absence = RED (P16 §2 methodology).

---

## 3. DZ-08 COURIER — screen-by-screen build sequence

`BottomTabBar` (Tasks / Earnings / History / Shift) + full-bleed login/delivery. Build order follows
the courier leg of the lifecycle (L4→L7).

**C1 — Login / Invite → gates entry.** email/pw error-shake; invite redeem (role-aware
Courier/Dispatcher, 16-char code, validity states). Real DOM `<input>` overlay (FE-15).

**C2 — Shift (Act 1) → precondition for L4/L5.** live HH:MM:SS timer, start/end (Sea energy-raise on
start), on/off pulsing dot, today's stats grid, messenger save. A courier must be on-shift + online for
task fan-out to reach them.

**C3 — Tasks (Act 1→2) → covers L4/L5.** assignment fetch + real online-status + WS `task_assigned` →
**one ripple + ping + dedupe** (the exactly-once thread at the courier edge); accept→delivery /
reject→optimistic-restore; `TaskCard` **60s countdown + auto-decline** + pickup→dropoff timeline;
online/offline empty state. This is the L4 (CONFIRMED→assigned) and L5 (PREPARING/READY→courier push)
courier surface.

**C4 — Delivery (Act 3, Море = map) → covers L6/L7.** live GPS **12s heartbeat** + `CourierLiveMap`
(courier/dest/client pins + route) = geo field flow (`geo_progress_js`/`geo_eta_js`/`geo_is_arriving_js`);
WS `client_location`; mid-delivery cancel; drop-off card; entry-photo → fullscreen; tip; cash breakdown;
call/message; mark-picked-up; cash-collected; **`SwipeToComplete`** (keyboard-operable,
**resets-on-failure — never fakes success**) → delivered = **gold bloom** + celebration. The DELIVERED
swipe is the L7 termination node: it must drive `apply_event_js` to `Delivered` and the (mesh/attic)
backend's single-transaction `delivery_trace` INSERT `ON CONFLICT (order_id) DO NOTHING` (idempotency) +
feedback job after commit.

**C5 — Earnings → money surface.** `<Money>` today/week/month (**snap, never count-up**) + payouts +
`StatusBadge`.

**C6 — History → post-delivery.** completed list (locale date sq/en/uk, 5-star, feedback).

**Feature-preservation gate (DZ-08):** full courier checklist; 60s auto-decline; `SwipeToComplete`
never-fake-success; GPS field flow; money snap. The `NO-COURIER-SCORING` reconcile (whether displaying
a courier rating violates the structural gate) is an **operator ruling flagged, not resolved here** —
carried as a ledger row (P16 §2 step 4e).

---

## 4. RW-06/07/08 kernel-port prerequisites — exact sequencing vs. the UI screens

The rule is **export-before-screen**: a UI screen may not be built against a kernel port until that
port's WASM bridge is green. Three of the four ports have kernel authority already (§1.2); the gating
work is the WASM exports plus the one server-pricing rewire.

| # | Prerequisite (kernel/wasm) | State | Blocks | Must land before |
|---|---|---|---|---|
| P0 | **`geo_*_js`** family | **DONE** (`wasm.rs:474–599`) | S4 Track map, C4 Delivery | — (already green) |
| P1 | **`cart_*_js`** exports over `cart.rs` (`add`/`remove`/`item_count`/`price`/`reconcile`) | kernel done; bridge missing | S2 Detail add-to-cart, S3 Checkout summary | **S2** |
| P2 | **`place_order_js` priced through `catalog.rs`** (stop trusting client `unit_price`) + `estimate_order_total_js` wired to the same catalog | gap (§1.2) | S3 Checkout place-order, L2 server-priced | **S3** |
| P3 | **`messenger_*_js` + `format_money_js`** exports over `messenger.rs` / `cart.rs::format_money` | kernel done; bridge missing | S3 Checkout messenger link, C5 Earnings display | **S3 / C5** |

**Therefore the landing order is:** `P0 (done) → P1 → P2 → P3`, interleaved with UI as
`S1 (needs nothing new) → P1 → S2 → P2 + P3 → S3 → [C1..C4 need only P0, already green] → S4 → C5/C6`.
Concretely: **RW-07's cart consolidation (P1) must precede the checkout/detail screens, and the
`catalog.rs` server-pricing rewire (P2) must precede the checkout place-order** — both are named in the
task as the canonical ordering constraint, and both are confirmed by the reliability-gate L2 criterion.

**RW-06 is off the critical path** — it is already done, so DZ-07 Track and DZ-08 Delivery can be built
against it immediately; the courier leg (C1–C4) needs no new kernel export beyond `geo_*_js`.

**RW-02/03 (delete `channel.js` / `money.ts` + utils transition-table)** are, at HEAD, **git no-ops** —
those files live only in the deleted/attic tree (roadmap §7, C8 confirmed this class of delete is a
no-op on quarantined dead code). Their *substance* survives as an invariant enforced by the inherited
wasm-math grep gate (§5), not as a delete to perform. `parse/ETA/normalizePhone/generateIdempotencyKey`
helpers RW-03 marks "PORT not delete" are covered by `geo.rs` + `messenger.rs::normalize_phone` +
`place_order_js`'s idempotency-key handling.

---

## 5. Money-tween-kill mechanism (RW-02/03/12 + FE-17) — concrete design

"Kill the money-tween" means **both an ESLint/grep gate AND a structural/runtime guarantee** — not one
or the other. The new UI must be the *only* UI (no legacy fallback confusion), and money must be
provably un-interpolable. Three enforcement layers, mirroring P16 §4's "enforced three ways":

1. **Structural (build-forward, primary).** The `<Money>` component (DZ-02) renders the integer-cent
   value `estimate_order_total_js` / `cart_*_js` / `format_money_js` return, and has **no `tween`,
   `duration`, or `from` prop** — count-up is *unreachable by construction*. There is no
   `AnimatedNumber` or `CountUpPrice` in the new tree; the four legacy sites FE-17 names
   (`ClientLayout:154`, `EarningsPage:47–176`, `DashboardPage:421`, `AnalyticsPage:262`) exist only in
   the deleted tree, so FE-17 in 9a is a **never-introduce constraint**, not a legacy edit. `<Money>` is
   mono + tabular + integer-from-kernel + never-tween + never-round (DZ Appendix B rule 2).

2. **Static gate (falsifiable CI, secondary).** An ESLint rule / grep in the inherited
   `ci-no-client-math.sh` (P16 §3) that **denies number-animation on money-bound identifiers** over
   `web/src/**` (price/total/fee/tax/cents). **RED→GREEN falsifier:** plant one money-bound
   `AnimatedNumber` on a probe branch → gate RED; remove → GREEN. This is the roadmap 9a done-test
   line *"grep money-bound `AnimatedNumber` = 0."* Non-money counts (order counts) may still odometer
   (FE-17 explicitly permits this).

3. **Boundary (runtime invariant, backstop).** FE-09 / `engine/src/money_guard.rs`: `Money(pub i64)` is
   deliberately **not** a `FieldValue`. Money never enters the field integrator, so the Sea physically
   *cannot* interpolate a money value even if a view tried — the field↔state boundary is the runtime
   backstop under the two static layers.

**RW-12's other clauses** (dedup `hooks.ts` vs packages/ui, `safeStorage` ×2, delete `devBootstrap`/
`mockData` from prod) are, like RW-02/03, no-ops on the deleted tree; in 9a they reduce to the
build-forward rule "the new UI ships no duplicate hook/storage and no mock/dev bootstrap." The **money
red-line 🔴** stands: none of this touches money *computation* (money.rs/catalog.rs), only presentation.

---

## 6. FE-15 / DZ-11 hybrid a11y — design (non-optional, kept in 9a)

A canvas-first UI is invisible to assistive tech; WCAG makes the hybrid DOM mandatory, so it stays in
9a and is not deferred. Three mechanisms (DZ-11 / FE-15 contract, = P16 §6):

- **(a) SSR menu stays real DOM.** The public `/s/:slug` menu (S1 / L0) is never migrated to canvas —
  it is server-rendered DOM, natively screen-reader-navigable and SEO-indexable. This is also the L0
  cache-header surface. *Documented permanent losses on the canvas surfaces (browser Ctrl+F / page
  translate over field-drawn text) are recorded, not hidden.*
- **(b) Transparent `<input>` overlay for forms.** Every checkout/OTP/contact/address field (S3) and
  the courier login (C1) is a real transparent DOM `<input>` positioned over the field, `type=email/tel`
  preserved for autofill + mobile keyboard + IME composition. Forms are never canvas-faked.
- **(c) Hidden semantic DOM mirror.** A parallel hidden transparent DOM tree mirrors the field's widget
  list per-frame: dishes as real `<button role aria-label tabindex>`, cards/steppers with roles,
  reconciled each frame from the immediate widget list. Order-status changes on Track (S4) announce via
  `aria-live`. Keyboard nav (incl. `SwipeToComplete` keyboard path, C4) operates the mirror.

**RED→GREEN gate:** RED = screen reader finds the canvas invisible; GREEN = it announces role + label,
the form accepts typed input + autofill, keyboard nav works. This is the roadmap 9a done-test line
*"screen-reader reads the semantic mirror (RED: canvas invisible to AT)."*

---

## 7. Acceptance criteria — the `/reliability-gate` L0–L11 trace (numbered)

Phase 9a is **done** iff the `/reliability-gate` skill returns **GO** when run against the new
interface. The gate traces **ONE order** `/s/:slug` → L0–L11 → delivered+feedback and checks five
threads on every run: 🔴 **exactly-once**, 🔴 **recoverable**, 🔴 **cross-surface consistent**, 🔴
**proof-by-artifact** (PASS only with a code citation), 🔴 **timely signal** (dead-man's-switch at every
stuck point). GO = all L0–L11 PASS *with artifact* + exactly-once throughout + N=2 cross-instance +
zero cross-tenant leak + zero partial state on rollback; a run is still GO if the only failures are in
the skill's documented Known-debt list (that list stays flag-only).

**One required re-point (state explicitly).** The skill's file targets (`apps/api/src/routes/orders.ts`,
`apps/web/src/pages/client/MenuPage.tsx`, …) name the **legacy/attic** tree. The **L0–L11 stage
semantics and the five threads are architecture-agnostic and preserved unchanged**; only the audited
*surfaces* re-point to the new interface — the `web/` shell pages (S1–S4, C1–C6) + the kernel order
path (`place_order_js` server-priced via `catalog.rs`, `apply_event_js` FSM, `order_machine.rs`,
`geo_*_js`, the local-first event-log/outbox — DZ-06). This re-pointing of the gate's target list is a
Phase-9a deliverable; it is *not* a new verification scheme.

**Stage → responsible screen → PASS criterion (each item is one numbered acceptance):**

1. **L0 Entry** → **DZ-07 S1 Menu**. SSR `/s/:slug` DOM route exists with cache headers ≤60s +
   `menu_version`; SSR menu **stays DOM** (item ties to §6a). *Artifact:* SSR route + cache header.
2. **L1 Order-creation prep** → **DZ-07 S2 Detail + cart**. Modifier groups build a cart via
   `cart_*_js`; cart total is a kernel integer (`cart.rs::price`), dedupe by (product, options).
   *Artifact:* `cart.rs` + `cart_*_js` export.
3. **L2 Order POST** → **DZ-07 S3 Checkout**. `place_order_js` is one BEGIN/COMMIT-equivalent with
   idempotency key + items + timeout-job; **server recomputes total from `catalog.rs`, no client
   `unit_price` trusted** (§4 P2); double-POST same key → same response; **OTP** send/verify enforced
   (not skipped on error). *Artifact:* `place_order_js` priced-through-catalog + idempotency guard.
4. **L3 Notifications → CONFIRMED** → FSM via `apply_event_js` (PENDING→CONFIRMED); order-timeout
   neutralized on CONFIRMED. **Scope note:** the *owner accept* UI is **9b**; in 9a the CONFIRMED
   transition is driven by the kernel FSM + the mesh/attic backend, and the client (S4) + courier (C3)
   surfaces observe it. Flagged as the one genuine cross-role coupling.
5. **L4 CONFIRMED → assigned** → **DZ-08 C3 Tasks**. WS `task_assigned` → **one ripple + ping +
   dedupe** (exactly-once at courier edge); accept→delivery / reject→restore. *Artifact:* dedupe path.
6. **L5 PREPARING/READY** → **DZ-08 C2 Shift + C3 Tasks**. READY → courier push; anti-race
   `WHERE status=$current RETURNING id` (rowCount=0 → 409). *Artifact:* status-service guard.
7. **L6 IN_DELIVERY** → **DZ-08 C4 Delivery**. GPS 12s heartbeat + geo field flow (`geo_*_js`
   route-snap/ETA/arrival); GPS bounds (accuracy>100m→400, speed>150km/h→400 — or flag-only per skill
   Known-debt). *Artifact:* `geo.rs` consumers + GPS filter.
8. **L7 DELIVERED (termination)** → **DZ-08 C4 SwipeToComplete**. `apply_event_js`→`Delivered`; single
   transaction with `delivery_trace` INSERT `ON CONFLICT (order_id) DO NOTHING` + (if cash)
   `courier_cash_ledger` INSERT; feedback job enqueued AFTER commit; `SwipeToComplete`
   **never-fake-success, resets-on-failure**. *Artifact:* DELIVERED handler + idempotent trace.
9. **L8 Post-DELIVERED WS** → **DZ-07 S4 Track**. DELIVERED publishes to `order:{id}` +
   `location:{id}:dashboard`; Track shows DELIVERED under terminal-lock. *Artifact:* publish + terminal
   lock.
10. **L9 Feedback** → **DZ-07 S4 Track**. `StarRatingBlock` shown at DELIVERED, `canSubmit`-gated;
    rating UPSERT `ON CONFLICT (order_id)` (exactly-once). *Artifact:* ratings UPSERT.
11. **L10 Ratings propagation** → owner `avg_rating` is **9b**; the client *submit* (item 10) is 9a.
    Flagged: owner-side propagation not in 9a scope (driven by mesh/attic backend for the gate).
12. **L11 Cross-surface matrix** → **DZ-07 S4 Track + DZ-08**. DELIVERED **shows on the client
    surface and is absent from the active board**; exactly-once across surfaces; N=2 via Postgres
    `NOTIFY` broadcast; no cross-tenant leak. Owner surfaces (DispatchView/dashboard) are **9b** — for a
    9a GO they are served by the mesh/attic backend, not the rebuilt owner UI. *Artifact:* surface matrix.

**Plus the roadmap §4 9a-row extras (each a numbered acceptance):**

13. **Money snap:** `grep` money-bound `AnimatedNumber` over the new UI tree = **0** (§5 gate; RED on
    planted violation).
14. **A11y:** screen reader reads the semantic mirror (§6; RED: canvas invisible to AT).
15. **Degrade path works:** WebGPU/wasm absent → WebGL2/static-gradient Sea, state still legible
    (reduced-motion path doubles as degrade — FE-16).
16. **SSR menu stays DOM** (§6a) — the public entry never becomes canvas.

**Falsifier for the whole phase:** any L0–L11 leg FAIL (outside the Known-debt list), any surface
duplication, a trusted client total at L2, a faked `SwipeToComplete` success, a money tween, or a
canvas-invisible flow → **NO-GO**.

---

## 8. Reconciliation note — overlap with sovereign-roadmap BLUEPRINT-P16 (verdict)

**Verdict: COMPLEMENTARY. Neither supersedes; do not duplicate.** Restating §1.3 as the explicit
reconciliation the task asks for:

- **Same underlying work, two altitudes.** Both rebuild the deleted product UI on the kernel/`web/`
  shell with feature-inventory preservation, both use the As-Built + DZ oracles, both mandate Sea&Sheet
  + `<Money>` snap + hybrid-DOM a11y + the wasm-math gate. P16 is the **whole surface** (26 pages +
  i18n + WCAG + responsive, on the R2 sovereign roadmap, gated on Phase-4 `compose` + Phase-13 spine).
  Phase 9a is the **G11 vertical slice** (client + courier only, on the living-interface roadmap,
  sequenced for the shortest order path).
- **Containment.** Phase 9a's page set ⊂ P16's page set (9a = P16's 3 client + relevant courier pages;
  9a excludes P16's 16 owner pages, full i18n-as-blocking-gate, and the full per-page WCAG/responsive
  matrix — those are 9b-and-later).
- **Shared machinery is inherited once.** The `RECONCILIATION-LEDGER.md`, `ci-no-client-math.sh`
  wasm-math gate, Sea&Sheet spec, and DZ-11/FE-15 a11y architecture are authored by/with P16 and
  **reused** by 9a scoped to client+courier rows — 9a authors none of them a second time.
- **Divergence is only the done-test.** P16 item 5 = broad Playwright E2E incl. PoD signing + payout
  over a real Phase-13 mesh hub; **9a = the `/reliability-gate` L0–L11 trace** (a strict subset:
  order-lifecycle exactly-once/recoverable/cross-surface, minus PoD/payout). 9a's gate is the *sharper*
  instrument for "the first real order completed"; P16's E2E is the broader instrument for "the whole
  product is rebuilt."
- **Sequencing between them.** If both run, **9a lands first and yields G11**; P16's owner/i18n/WCAG
  remainder is 9b-and-later. No work is done twice: 9a is the fast, order-critical projection; P16 is
  the complete horizontal rebuild that 9a's slice slots into.

**One friction to carry:** the reliability-gate skill's file-target list is legacy (attic). Re-pointing
it at the new interface (§7) is a 9a deliverable and must not fork the gate's L0–L11 stage semantics or
its five threads.

---

*Blueprint for Phase 9a of the living-interface roadmap — the G11 fast-path. Integrates DZ-07/08,
RW-06/07/08, RW-02/03/12+FE-17, FE-15/DZ-11 into a dependency-ordered build to one completed order.
Current-state facts (apps/web deletion; geo.rs done + `geo_*_js` bridge; cart.rs/messenger.rs kernel
authority present, WASM bridge missing; `place_order_js` client-price gap; 24 wasm `_js` exports)
re-verified against the live tree 2026-07-16. Acceptance is grounded in the `/reliability-gate` L0–L11
trace, not a parallel scheme. Complementary to sovereign-roadmap BLUEPRINT-P16 (the horizontal rebuild);
9a is its G11 vertical slice. Planning only — no product code, CI config, or canon edited.*
