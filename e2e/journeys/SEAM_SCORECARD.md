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
| F12 | Q4 aria-live: status changes weakly announced | OrderStatusPage.tsx:388,431 | TODO |
| F13 | Q5 continuity-on-refresh mid-journey untested | — | TODO (add assertion) |
| F14 | Two divergent WS clients (forever vs 10-cap reconnect) — one freezes permanently | useWebSocket.ts vs ui/websocket.ts | TODO/refactor |

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

## Remaining (polish-debt, next rounds)
- F7 owner hollow-card flash · F9-full cart↔menu_version reconcile · F12 aria-live announce ·
  F13 continuity-on-refresh assertion · F14 consolidate the two WS clients (forever vs 10-cap).

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
