# Seam Scorecard — final UX/seam-polish loop (FE-only, server read-only)

Synthesized from 3 adversarial audits (hater · UX-critique · QA-seams). Status: ROUGH → SMOOTH
(proven by a fix + verification) or BLOCKED-server (needs a server change — out of scope, flagged).

## Tier-1 order-lifecycle seams

| ID | Seam / defect | Class (Q) | File:line | Sev | Status |
|----|---------------|-----------|-----------|-----|--------|
| F1 | Courier "Slide to Deliver" shows "Delivered!" BEFORE server responds; error swallowed; navigates regardless → optimistic overrides server truth (red-line) | Q-reconcile/Q7 | DeliveryPage.tsx handleComplete | 🔴crit | FIXING |
| F2 | Courier never learns order cancelled/rejected mid-delivery (no order.status WS branch) → drives to dead order | frozen/dead-end | DeliveryPage.tsx WS onMessage | 🔴crit | FIXING |
| F3 | Terminal states (REJECTED/CANCELLED/DELIVERED) are dead-ends — no "order again", phone gated behind WS banner only | Q1 | OrderStatusPage.tsx terminal | high | FIXING |
| F4 | Token-expiry mid-tracking renders "Not Found" (mislabel) with no reload/menu/phone CTA | Q6/Q1 | OrderStatusPage.tsx:381 | high | FIXING |
| F5 | Live map first frame centered on hardcoded Tirana, jerks to real courier pos (FOWS) | Q2 | OrderStatusPage.tsx:52,421 | major | FIXING |
| F6 | Owner kanban status flip reverts silently on failure (no error toast) | Q7 | DashboardPage.tsx handleUpdateStatus | major | TODO |
| F7 | Owner new-order card flashes nameless/item-less ~800ms during rush | FOWS | DashboardPage.tsx mergeDelta | major | TODO |
| F8 | Checkout has no venue-closed awareness → full form then raw "Failed to place order" | Q6/Q7 | CheckoutPage.tsx | high | TODO |
| F9 | Cart never reconciles to menu_version → stale price ambush at checkout | Q7 | CartProvider.tsx:5 | high | TODO (partial-BS: detection may need server) |
| F10 | Phantom status toast on cold page open | Q4/surprise | OrderStatusPage.tsx:271 | minor | TODO |
| F11 | DeliveryPage/OrderStatusPage 404 → fabricated mock task / fake PENDING order leaks into real failure path | Q2/Q7 | DeliveryPage.tsx:92, OrderStatusPage.tsx:173 | high | VERIFY (dev-gated?) |

## Resilience / a11y

| ID | Seam | File:line | Status |
|----|------|-----------|--------|
| F12 | Q4 aria-live: status changes weakly announced | OrderStatusPage.tsx:388,431 | SMOOTH (sr-only role=status announcer; proof pending deploy) |
| F13 | Q5 continuity-on-refresh mid-journey untested | — | SMOOTH (assertion added, GREEN on staging) |
| F14 | Two divergent WS clients (forever vs 10-cap reconnect) — one freezes permanently | useWebSocket.ts vs ui/websocket.ts | SMOOTH (dead capped client deleted; one client) |

## Status after batch 1+2 (commits b033bf8e, 2074f7d4 + seam-polish.spec.ts)
- F1/F2/F11 SMOOTH (courier delivery honesty; cancel-aware; dev-mock gated) — committed, typecheck green.
- F3 SMOOTH — verified on real staging UI (CANCELLED green: order-terminal-exit + order-again→/s/;
  REJECTED is the identical render block, blocked from a clean run only by the menu flake below).
- F4 (token-expiry soft state), F5 (map FOWS), F6 (owner error toast), F8/F9-partial (humane checkout
  errors), F10 (no phantom toast) — committed, typecheck green.

## Batch 3 + hater re-audit → SATISFIED
- F2 residual closed: cancellation banner lifted out of the picked-up branch (shows pre- AND
  post-pickup). F8/F9 residual closed: 200-body hard_block now shows the designed "review your
  cart" message (not the generic). Hater re-audit verdict: **SATISFIED — no remaining blockers**
  (F1,F3,F4,F5,F6,F10,F11 verified SMOOTH; F7/F12/F14 non-blocking). Commits b033bf8e, 2074f7d4,
  c1fb3ea1; real-UI proof e2e/tests/seam-polish.spec.ts (CANCELLED green on staging).

## Polish-debt round → SMOOTH (commit f79e2910, FE-only) ✅
- **F7** owner hollow-card flash: OrderCard shows a shimmer placeholder while `isOrderDetailsPending`
  (itemCount>0 but items not yet backfilled) instead of a nameless / "0 items" card; count falls
  back to `itemCount`.
- **F9** cart↔menu_version reconcile: cart stores the `menu_version` it was priced against; on every
  menu load `reconcileCart()` re-prices drifted modifier-free lines, drops sold-out/removed items,
  and shows a non-blocking notice — checkout no longer ambushes with a server hard-block. (Modifier
  lines deferred to the server guard.)
- **F12** a11y: dedicated sr-only `role="status"` aria-live region speaks each status transition as
  explicit localized text (visual toast wasn't reliably announced; stepper is graphical).
- **F14** WS consolidation: deleted the dead reconnect-capped `WsClient` (0 consumers, froze after
  10 tries); `useWebSocket` (reconnect-forever) is the single client; `no-direct-websocket` rule
  tightened to one exception.
- Proof: `e2e/tests/polish-debt-logic.spec.ts` 11/11 GREEN (F9 reconcile branches · F7 predicate ·
  F14 re-add guard) + `e2e/tests/polish-debt.spec.ts` **F13 GREEN on staging** (refresh continuity);
  **F12 proof pending deploy** (the new sr-announcer element 404s on the not-yet-deployed staging —
  the expected red half; goes green once f79e2910 ships). Ledger rows 5 (updated) + 7.
- ⚠️ Deploy gap: no fly CLI in this sandbox → operator must deploy f79e2910 to staging, then re-run
  `VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test polish-debt --project=desktop`
  to flip F12 green.

## BLOCKED-server → FIXED + DEPLOYED + PROVEN on staging (commits d120a914, 57f32e11) ✅
- 🔴→🟢 `/public/locations/demo/menu` "returns empty under load" — diagnosed read-only then
  reproduced on staging: NOT a 0-product body but **HTTP 500 @ ~5.0s** (== operational-pool
  `connectionTimeoutMillis`); the FE catch renders that as the empty storefront. 20/20
  concurrent curls → 500; single hit → 200 / 45 products.
- Root cause = operational-pool connection-acquisition starvation: no cache on the hottest read
  + 2 conns/request (Promise.all of read_public_menu + a redundant locations lookup) + heavy
  per-row `product_available_now`; burst > pool (max:8) → checkout waits 5s → 500. Ruled out
  RLS/GUC-leak (products/categories/locations have `public_select USING(true)`; SET LOCAL).
- Fix (server-side, d120a914): F1 in-process cache (30s TTL + SWR + stale-on-error) · F2 fold
  location_id/name into read_public_menu, drop the redundant query · F3 `OPERATIONAL_POOL_SIZE`
  env (default 20, was 8; Supavisor txn-mode multiplexes) · F4 set-based availability predicate
  (≡ product_available_now). Migration 1790000000064 is CREATE OR REPLACE only with a REAL
  `down()` (restores 063) for rollback.
- Proof (deployed to dowiz-staging, migration 064 applied via release_command):
  - BEFORE: 20/20 concurrent → HTTP 500 @ ~5.1s. AFTER: 0×5xx across 120+ reqs, max ~0.3s, 200s
    with 45 products; only per-IP 429s (global 100/min limiter) when one IP over-hammers.
  - `e2e/tests/menu-load.spec.ts` GREEN on staging — 3/3 projects (mobile/tablet/desktop).
  - F4 equivalence SQL green (`packages/db/tests/read-public-menu-availability-equivalence.sql`,
    18 cases); both fn bodies parse; ledger #15.

> Iron rules: server read-only; never block the human (courier always completes); optimistic always
> reconciles to server; never accuse the client; flag-debt anything needing a server change.
