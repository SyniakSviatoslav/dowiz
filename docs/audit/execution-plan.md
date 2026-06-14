# Dowiz — Execution Plan: Audit Issues → Green Pilot

> **Source:** Full Service Audit v1 + Hardening Fix-Set v1 + Onboarding Fix-Spec v1 + Page Audit  
> **Total issues:** 42 found · **Plan:** 6 sprints, sequenced by dependency and impact

---

## Priority Legend

| Symbol | Meaning |
|--------|---------|
| 🔴 | Block pilot — must fix before first live restaurant |
| 🟠 | Fix this sprint — real risk on pilot |
| 🟡 | Backlog — fix when touching the area |
| ⚪ | Doc-only or verified OK — no code |

---

## Sprint 0: Unblock Courier Delivery (1 day)

> **Why first:** V16 = BROKEN. The entire courier delivery + cash flow is dead code until routes are registered. Nothing else matters if couriers can't deliver.

| # | Action | Files | Effort |
|---|--------|-------|--------|
| 🔴 S0-1 | Register `courier/assignments.ts` routes in `server.ts` | `server.ts:75` | 10 min |
| 🔴 S0-2 | Register `courier/shifts.ts` routes in `server.ts` | `server.ts:75` | 10 min |
| 🔴 S0-3 | Add auth (requireRole) to theme routes | `routes/owner/themes.ts:11,40,113` | 10 min |
| 🔴 S0-4 | Add auth (requireRole) to notification routes | `routes/owner/notifications.ts:10,26,53,90` | 10 min |

---

## Sprint 1: Hardening — Server-Side Safety (2 days)

> **Why next:** FX-4 + FX-6 + FX-9 are the biggest risk-to-effort ratio for a solo pilot. Prevent order flood, error leaks, and DB exhaustion.

| # | Action | Files | Effort | Ticket |
|---|--------|-------|--------|--------|
| 🔴 S1-1 | Implement per-phone order throttle: `(location_id, phone_hash)` — N orders / 15min window → 429 | `routes/orders.ts` | 2h | FX-4 |
| 🔴 S1-2 | Custom error handler: map all 500s → `{code, safe_message}`, never serialize `err.stack`/PG detail | `server.ts` (setErrorHandler) | 1h | FX-6 |
| 🔴 S1-3 | Set `statement_timeout` on operational pool (5-10s) + session pool (30s) | `packages/db/src/index.ts` | 30 min | FX-9 |
| 🔴 S1-4 | Set `connectionTimeoutMillis` on pg Pool (acquire timeout → fast fail → 503) | `packages/db/src/index.ts` | 15 min | FX-9 |
| 🟠 S1-5 | Zod `.max()` on all array/string inputs: order items count, address length, note length | `packages/shared-types/src/legacy.ts` | 1h | FX-7 |
| 🟠 S1-6 | Fastify `bodyLimit` to 64KB (order payload is tiny, prevent OOM) | `server.ts` | 5 min | FX-7 |
| 🟠 S1-7 | Add `location_id` to idempotency key dedup lookup | `routes/orders.ts` or `lib/idempotency.ts` | 1h | FX-5 |

---

## Sprint 2: Auth & Token Cleanup (1 day)

> **Why:** P0-1 + P0-2 are hardcoded backdoors. FX-3 doc update. Clean up auth surface.

| # | Action | Files | Effort | Ticket |
|---|--------|-------|--------|--------|
| 🔴 S2-1 | Remove hardcoded credentials from LoginPage (`+355691234567`/`password`) | `pages/courier/LoginPage.tsx:31` | 10 min | P0-2 |
| 🔴 S2-2 | Fix OTP enforcement: do NOT proceed to order creation if verify fails | `pages/client/CheckoutPage.tsx:53` | 30 min | P0-1 |
| 🟠 S2-3 | Remove PII (phone) from customer JWT claims — confirm FX-2 done | `routes/orders.ts:48` → verify | verify | FX-2 |
| 🟠 S2-4 | Switch operational pool to non-superuser DB role | `packages/db/src/index.ts` + Supabase | 2h | FX-NEW-1 |
| 🟠 S2-5 | Onboarding publish: send ALL data (menu items, courier choice, branding, logo) | `pages/admin/OnboardingPage.tsx:138` | 1h | P0-5 |
| 🟠 S2-6 | Add confirmation dialog before "Go Live" publish | `pages/admin/OnboardingPage.tsx:436` | 30 min | P0-6 |

---

## Sprint 3: Frontend Bugs & Component Fixes (2 days)

> **Why:** P0 issues in components, P1 regressions. Fix the actual bugs users will hit.

| # | Action | Files | Effort | Ticket |
|---|--------|-------|--------|--------|
| 🟠 S3-1 | Fix duplicate component system (atoms/ vs Base.tsx) — consolidate Button, Input, StatusBadge | `packages/ui/src/components/` | 3h | P0-7 |
| 🟠 S3-2 | Fix OrderCard action buttons (confirm/reject/assign) with proper handlers | `components/admin/AdminUI.tsx:125-129` | 1h | P1-32 |
| 🟠 S3-3 | Fix OTPModal not resetting state when reopened | `components/client/ClientUI.tsx:170-238` | 30 min | P1-33 |
| 🟠 S3-4 | Fix MapLibreBase.innerHTML → use textContent for marker labels | `components/molecules/MapLibreBase.tsx:101` | 15 min | FX-NEW-3 |
| 🟠 S3-5 | Fix CourierRoutes bottom tab bar: add `?embed=true` conditional on `position:fixed` | `routes/CourierRoutes.tsx:33` | 15 min | P1-14 |
| 🟠 S3-6 | Re-enable WebSocket on DashboardPage (`enabled: true`) | `pages/admin/DashboardPage.tsx:57` | 5 min | P1-7 |
| 🟠 S3-7 | Fix Audio object leak in useSound hook (reuse Audio instance) | `lib/hooks.ts:30` | 30 min | P1-34 |
| 🟠 S3-8 | Fix useGeoStream to actually send location data | `lib/hooks.ts:86` | 1h | P1-16 |
| 🟡 S3-9 | Fix hardcoded colors in StatusBadge → use `--status-*` variables | `components/Base.tsx:103-112` | 30 min | P1-27 |
| 🟡 S3-10 | Fix hardcoded colors in admin status helpers → CSS variables | `components/admin/AdminUI.tsx:133-141` | 15 min | P1-16 |

---

## Sprint 4: Missing States & Polish (2 days)

> **Why:** Empty states, error states, loading states — things that make the difference between "works" and "production-ready."

| # | Action | Files | Effort | Ticket |
|---|--------|-------|--------|--------|
| 🟡 S4-1 | Add empty state to CourierLiveMap when no couriers + no destination | `components/molecules/CourierLiveMap.tsx:21-28` | 15 min | P2-15 |
| 🟡 S4-2 | Add loading/disabled state to ConfirmDialog double-click guard | `components/molecules/ConfirmDialog.tsx:84-85` | 15 min | P2-17 |
| 🟡 S4-3 | Add pause-on-hover to Toast timer + toast limit (max 5) | `components/molecules/Toast.tsx:32-41,82` | 30 min | P2-16 |
| 🟡 S4-4 | Add 7d/30d period re-fetch to Analytics (currently toggle has no effect) | `pages/admin/AnalyticsPage.tsx:30` | 30 min | P0-10 |
| 🟡 S4-5 | Add pagination to CRM customer table | `pages/admin/CRMPage.tsx` | 1h | P2-19 |
| 🟡 S4-6 | Redact phone numbers in CRM CSV export | `pages/admin/CRMPage.tsx:110` | 15 min | P1-10 |
| 🟡 S4-7 | Add date range selector to Courier Earnings | `pages/courier/EarningsPage.tsx` | 30 min | P2-20 |
| 🟡 S4-8 | Fix hardcoded restaurant name/location data — fetch from server | `pages/client/MenuPage.tsx:101-102` | 1h | P1-2 |
| 🟡 S4-9 | Fix delivery fee hardcoded — fetch from location config | `pages/client/CheckoutPage.tsx:25` | 30 min | P1-3 |

---

## Sprint 5: Accessibility & Cross-Cutting (1 day)

> **Why:** Keyboard users, screen readers. Low effort, high inclusivity.

| # | Action | Files | Effort | Ticket |
|---|--------|-------|--------|--------|
| 🟡 S5-1 | Add `aria-label` to CartFAB (item count + total) | `components/client/ClientUI.tsx:142` | 10 min | P2-5 |
| 🟡 S5-2 | Add `role="tab"` + `aria-selected` to MenuPage category nav | `pages/client/MenuPage.tsx:117` | 15 min | P2-4 |
| 🟡 S5-3 | Add `aria-current="page"` to Courier bottom tabs | `routes/CourierRoutes.tsx:37-46` | 10 min | P2-6 |
| 🟡 S5-4 | Add `aria-pressed` to Dashboard filter buttons | `pages/admin/DashboardPage.tsx:130` | 15 min | P2-7 |
| 🟡 S5-5 | Add `aria-live="polite"` to Toast container | `components/molecules/Toast.tsx:77` | 5 min | P2-8 |
| 🟡 S5-6 | Add keyboard support to SwipeToComplete (Enter/Space to confirm) | `components/courier/CourierUI.tsx:170-188` | 1h | P1-30 |
| 🟡 S5-7 | Add keyboard support to Tooltip (onFocus/onBlur) | `components/molecules/Tooltip.tsx:47` | 15 min | P1-29 |
| 🟡 S5-8 | Add Escape key handler + focus trap to mobile admin drawer | `routes/AdminRoutes.tsx` | 30 min | P1-20 |
| 🟡 S5-9 | Add logout button for mobile admin | `components/admin/AdminUI.tsx:42` | 10 min | P2-20 |

---

## Sprint 6: Final Verification (1 day)

| # | Action | Effort |
|---|--------|--------|
| ⚪ S6-1 | Run full Playwright suite (92 tests × 3 breakpoints) — confirm ALL GREEN | 30 min |
| ⚪ S6-2 | Manual smoke test of all 18 screens via dev-hub.html | 1h |
| ⚪ S6-3 | Verify no regressions: `pnpm typecheck && pnpm lint:gates && pnpm build` | 15 min |
| ⚪ S6-4 | Write 10 new Playwright tests for fixed issues (courier delivery flow, OTP enforcement, throttle 429, confirmation dialog) | 2h |
| ⚪ S6-5 | Update `e2e/MATRIX.md` with new test results | 15 min |
| ⚪ S6-6 | Deploy to Fly staging, verify on real Supabase | 30 min |

---

## Dependency Graph

```
Sprint 0 (Unblock Courier)
  └→ Sprint 1 (Server Hardening)
       └→ Sprint 2 (Auth Cleanup)
            └→ Sprint 3 (Frontend Bugs)
                 └→ Sprint 4 (Polish)
                      └→ Sprint 5 (Accessibility)
                           └→ Sprint 6 (Verification)
```

**Parallelizable:** Sprints 3, 4, 5 can overlap (different files, no conflicts).

---

## Consolidated Issue Registry

| ID | Severity | Area | Issue | Sprint |
|----|----------|------|-------|--------|
| V16 | CRITICAL | Courier delivery | Routes unregistered — cash flow dead | S0 |
| V2 | HIGH | Auth | Theme/notification routes lack auth | S0 |
| V11 | CRITICAL | DoS | No per-phone order throttle | S1 |
| V12 | HIGH | Error handling | No custom error serializer | S1 |
| V9 | HIGH | DB | No statement_timeout | S1 |
| V4 | HIGH | Idempotency | No location_id in dedup | S1 |
| P0-2 | CRITICAL | Auth | Hardcoded credentials in LoginPage | S2 |
| P0-1 | CRITICAL | Checkout | OTP never enforced | S2 |
| P0-6 | CRITICAL | Onboarding | No publish confirmation | S2 |
| P0-5 | HIGH | Onboarding | Publish missing data | S2 |
| V1 | HIGH | DB | Superuser bypasses RLS | S2 |
| P0-7 | HIGH | Components | Duplicate Button/Input/StatusBadge | S3 |
| P0-12 | HIGH | CourierUI | SwipeToComplete event leak | S3 |
| P1-32 | HIGH | AdminUI | OrderCard buttons unhandled rejection | S3 |
| P1-7 | HIGH | Dashboard | WS disabled (enabled: false) | S3 |
| P1-16 | HIGH | Geo | useGeoStream never sends data | S3 |
| P1-34 | HIGH | Hooks | Audio object leak | S3 |
| P0-10 | MEDIUM | Analytics | 7d/30d toggle no-op | S4 |
| P1-2 | MEDIUM | Menu | Hardcoded restaurant data | S4 |
| P1-3 | MEDIUM | Checkout | Hardcoded delivery fee | S4 |
| P2-15/17/16 | MEDIUM | Components | Empty/loading/toast states | S4 |
| P2-4/5/6/7/8 | MEDIUM | A11y | Missing aria labels | S5 |
| P1-30 | HIGH | CourierUI | No keyboard on SwipeToComplete | S5 |
| P1-20 | HIGH | Admin | No Escape key on drawer | S5 |

---

## Effort Summary

| Sprint | Focus | Days | Issues |
|--------|-------|------|--------|
| S0 | Unblock courier delivery | 0.5 | 4 |
| S1 | Server hardening | 2 | 7 |
| S2 | Auth + token cleanup | 1 | 6 |
| S3 | Frontend bugs | 2 | 8 |
| S4 | Polish + states | 2 | 9 |
| S5 | Accessibility | 1 | 9 |
| S6 | Verification | 1 | 6 |
| **Total** | | **9.5 days** | **49** |

---

*dowiz / DeliveryOS · Audit Execution Plan v1 · Sprint-sequenced · Confidential*
