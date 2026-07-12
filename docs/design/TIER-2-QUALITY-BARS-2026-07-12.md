# Tier-2 — Quality bars (plan + checklists)

Companion to `ROADMAP-GROUND-TRUTH-2026-07-11.md`. Tier-2 is the "stable enough"
gate that must hold before external G11 (a real non-operator order) is meaningful.
It is NOT code-gated by a red-line; it is a set of bars + two automated gates
(Playwright zero-diff, GTM per-venue).

## 1. Design "stable enough" — 13-item checklist

Verified by human sign-off per venue page + the storefront. Each item must be
GREEN before that surface is promoted out of "demo".

1. Tap target ≥ 44px on every interactive control (courier/owner/client).
2. Color contrast ≥ WCAG AA (4.5:1 text) — no hardcoded hex bypassing tokens.
3. Order total is integer minor units end-to-end (no float money, no tween) — VERIFIED by kernel + Storefront (Tier-0 A).
4. Failure state exists for every network call (error boundary / fallback banner).
5. Empty state exists for cart / orders / menu (no blank screen).
6. Loading state exists for place-order + pay (no double-submit).
7. Push notification renders with a real icon (Tier-0 B: /icons/* served).
8. Offline state degrades gracefully (ratchet boot-grace, Tier-0 B).
9. RTL + i18n: all user copy via `t()` (no hardcoded-string in UI — Tier-2 lint gate).
10. Keyboard reachable + visible focus ring on primary flows.
11. No console.error on happy path (venue claim, order place).
12. Channel attribution present: every order carries `?ch=` → `channel` (Tier-0 D + Tier-3).
13. Realtime order status reflects kernel state machine (no client-only optimism that lies).

## 2. Storefront zero-diff Playwright gate — BUILT + VERIFIED (2026-07-12)

`e2e/tests/tier2-storefront-contract.spec.ts` — headless click-through against the
canonical Rust `dowiz-server` (serving `web/dist`), asserts:

- Load `/` → Storefront island renders, no console errors (§1 item 11).
- Menu items (Margherita / Pepperoni) render.
- `POST /api/orders` → 201, status `PENDING`, integer `subtotal`/`total` (no float/tween).
- Reload via `GET /api/orders/channel` → order persisted + attributed to its channel.
- Illegal transition `PENDING→DELIVERED` → **409** (kernel decide/fold Law is authoritative).
- Tier-3 plumbing: `POST /api/venues/:id/claim` → claimed=true; unknown venue → 404;
  `?ch=venue` order lands under that channel in `GET /api/orders/channel`.

Run: `VITE_BASE_URL=http://localhost:3000 npx playwright test e2e/tests/tier2-storefront-contract.spec.ts --project=desktop`
Result 2026-07-12: **3 passed** (self-boots the Rust server; chromium-1223 is installed).
Gate fails on console-error or assertion failure. The earlier "browser binaries cleaned for
disk" blocker is resolved — chromium is present.

## 3. GTM 8-point per-venue gate

A venue may be promoted to "public" (GTM) only when all 8 hold:

1. Venue claimed (`POST /api/venues/:id/claim` → claimed=true; Tier-3).
2. Menu imported + at least 1 priced item.
3. Owner bank/settlement detail present (deferred — Tier-4 money).
4. Storefront serves with OG image (Tier-0 D).
5. Channel attribution verified (`?ch=` → order).
6. Push notifications configured (Tier-0 B resubscribe path live).
7. Design "stable enough" 13-item signed off.
8. One successful test order (sandbox) in `PENDING`→`DELIVERED` without illegal transition.

## 4. Courier out-of-app signal (N1/N2)

Courier assignment + status changes must surface outside the app:

- N1 = push notification (Tier-0 B handler) on new assignment + status change.
- N2 = (optional) SMS/email fallback when push unavailable.

Scope: wire N1 through the existing push subscription table + `notify_courier`
path; N2 is a stub until a provider is chosen (deferred, non-blocking for G11).

## Sequencing

- Lint army (parallel) clears the warning debt first (this branch).
- Then: (2) Playwright gate + (4) N1 wiring are the only code items here;
  (1) and (3) are checklists/sign-off, not build-blocking.
- G11 itself stays external (real non-operator order) — Tier-2 makes it
  *meaningful*, not sufficient on its own.
