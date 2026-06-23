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

## BLOCKED-server (flagged, NOT fixed here — server/infra read-only)
- 🔴 The public menu endpoint `/public/locations/demo/menu` intermittently returns EMPTY
  (0 products) for multi-second windows under load on staging — the storefront would blink
  empty for real customers. curl + the app both see it. This is a server/DB/infra issue, not a
  FE seam; needs a separate server-side diagnosis (caching/pool/read_public_menu under load).

> Iron rules: server read-only; never block the human (courier always completes); optimistic always
> reconciles to server; never accuse the client; flag-debt anything needing a server change.
