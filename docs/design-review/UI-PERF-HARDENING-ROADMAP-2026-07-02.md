# UI/UX + Perf + Hardening Roadmap (2026-07-02)

A sequenced plan for: design-system unification + new libs + more animation · responsive images ·
adaptive courier GPS · real service worker · ship-less-JS · perf Tiers 2–4 · fundamentals #1–4.

Companion docs: `design-animation-repos-2026-07-02.md` (lib verdicts), `PHONE-PERF-BATTERY-BACKLOG-2026-07-02.md`
(perf tiers), and the fundamentals gaps in the 2026-07-02 session assessment.

> **Not started.** This is the plan; each phase is built + shipped separately under Ship Discipline.

## Conventions every phase obeys (non-negotiable)
- **Ship Discipline:** feature-branch → pre-commit gate (lint→typecheck→build) → staging deploy → Playwright
  E2E vs staging + unit/integration + `pnpm typecheck` → **paste proof** → prod only on approval/merge.
- **Task-Exit Rule:** write the enriched exit checklist (states/error-matrix/edge/regression-radius/tokens/
  i18n al+en/security/contract-parity) **before** touching code; verify each item with proof after.
- **Mandatory Proof:** UI change → Playwright `expect(...).toBeVisible()`/`toContainText()` against
  `dowiz.fly.dev`/staging; API-only → a `request.*` assertion. Typecheck/build ≠ proof.
- **Flags default-off**; deploy dark, launch as a separate explicit act. **Bundle:** every phase must pass
  `.size-limit.json`. **Motion:** honor `prefers-reduced-motion` (already 128 refs) + `Save-Data`.
  **Money:** integer cents. **Migrations:** forward-only, RLS FORCE, `down()` for emergency rollback.

## Dependency graph (why the order is what it is)
```
Phase 0 (safety)  ─┐
                   ├─► Phase 1 (design system) ─► Phase 2 (motion + ship-less-JS)
dep-scan (0.2) ────┘        │
                            └─► Phase 3 (images)        [independent, parallelizable]
Phase 4 (service worker) ─► Phase 6 push · Phase 7 bg-sync · Phase 8 local-first
Phase 5 (courier GPS) ────────────────────────────────  [independent]
```
- **dep-scan (0.2) MUST precede Phase 1** — it scans the 6 new libs on entry.
- **Phase 4 (SW) MUST precede** Web Push, Background Sync, and local-first (they build on it).
- Phases 3 and 5 are independent and can run in parallel with 1–2 if there's capacity.

---

# Phase 0 — Safety net & guardrails (fundamentals #1–4)
**Goal:** close the latent traps and lay the gate that protects everything added later. Cheap, high-safety,
do first. (This is your "next 1–4" pulled to the front for dependency reasons — see 0.2.)

| # | Item | Approach | Gate/proof | Effort |
|---|------|----------|-----------|--------|
| 0.1 | **Fix emergency restore** (fund #1) | Wire `apps/api/src/scripts/restore.ts` to the real drill path (`runRestoreVerify` / `pg_restore` — the S3 client + decrypt stream are already imported), OR demote it and make manual `pg_restore` the runbook primary. | A real (dry-run + full) restore into a scratch DB with row-count/`verify:db` assertion; runbook step executes truthfully. | S–M |
| 0.2 | **Dependency/vuln scan CI gate** (fund #2) | Add `dependabot.yml` (or Renovate) + a `pnpm audit --audit-level=high` (or OSV) job to `.github/workflows/ci.yml`. **Do before Phase 1** so the new libs are scanned. | CI job red on a seeded high-sev advisory, green when clean. | S |
| 0.3 | **Reconcile DR runbooks** (fund #3) | Make one source of truth: `disaster-recovery.md` (RPO 4h) vs `backup/runbooks.md` (RPO 1h) — confirm the actual pg-boss backup cadence and align both. | Single documented RTO/RPO matching the scheduled job. | S |
| 0.4 | **External uptime → human page** (fund #4) | Add an external synthetic monitor on `/health` (also doubles as Supabase-Free auto-pause keep-alive) routing to a human channel on dead/degraded. | Kill staging → alert fires within threshold. | S |

Flag posture: none needed (infra/CI/ops). No product surface.

---

# Phase 1 — Design-system unification (close the prune with real libs)
**Goal:** replace the bespoke molecules this branch deleted with accessible, maintained primitives, on ONE
headless base + ONE motion engine. This is the foundation the animation work builds on.

| # | Item | Replaces | Approach | Gate/proof |
|---|------|----------|----------|-----------|
| 1.1 | **Radix primitives** | deleted `Modal`, `Tooltip` | Add `@radix-ui/react-dialog` + `-tooltip` (per-primitive, tree-shakes); style with existing Tailwind tokens; animate via Motion. Pick Radix as THE headless base (Base UI/Ark UI are the fallbacks if post-WorkOS velocity bites). | Playwright: focus-trap, Esc-close, `aria-*`, keyboard nav on the new Dialog/Tooltip. axe clean. |
| 1.2 | **vaul** | deleted `Drawer`, `BottomSheet` | Add `vaul` (built on Radix Dialog → composes with 1.1). Migrate storefront cart/checkout sheet + filter drawers. | Playwright: open/drag-dismiss/snap on mobile viewport; body-scroll-lock; reduced-motion path. |
| 1.3 | **sonner** | deleted `ToastManager` | Single `<Toaster/>` at app root; migrate all toast call-sites; pass i18n (al/en) content. | Playwright: order-placed/error toast visible + auto-dismiss; RTL/long-string OK; i18n parity. |
| 1.4 | **embla-carousel** | (net-new) | Add `embla-carousel-react` for menu-item photo galleries — only where a gallery is actually designed. | Playwright: swipe/drag advances; keyboard; a11y roles. |
| 1.5 | **Token/theme consolidation pass** | — | With bespoke components gone, unify spacing/color/type tokens to one source (`packages/ui/theme`); document the design system (tokens.md). Uses `color-system`/`spacing-system`/`typography-scale`/`theming-system` skills. | Visual-regression net green across 3 roles × al/en × breakpoints. |

Flag posture: component swaps are like-for-like (no flag if behavior-parity proven per surface); gate each
surface's swap behind the existing per-surface review. Effort: **M–L** (touch many call-sites; one edit/turn).

---

# Phase 2 — Motion system + ship-less-JS (coupled through Motion)
**Goal:** one coherent motion language + the biggest JS bundle win. (Perf #4 + "more animations".)

| # | Item | Approach | Gate/proof |
|---|------|----------|-----------|
| 2.1 | **LazyMotion + `m` migration** | Convert the 33 `framer-motion` `motion.*` imports in `packages/ui` to `LazyMotion` + `m` with a `domAnimation` feature set (~46KB→~4.6KB initial). Lint-ban stray `motion.*` imports. | `.size-limit.json` shows the drop; visual-regression net unchanged; animations still fire. |
| 2.2 | **Route-based code splitting** | Lazy-load admin + courier route bundles so customers don't ship them; `React.lazy` + Suspense per route group. | Network trace: customer storefront doesn't fetch admin/courier chunks; size-limit per entry. |
| 2.3 | **Motion tokens** | Define duration/easing tokens + reduced-motion variants (motion-system skill); replace ad-hoc spring configs with the token set. | Doc `motion.md`; visual-regression; reduced-motion honored. |
| 2.4 | **auto-animate** for lists | `useAutoAnimate` (~3KB) on menu list, cart lines, admin order queue, courier job list. | Playwright: add/remove/reorder animates; reduced-motion no-ops. |
| 2.5 | **number-flow** for numerics | Animated price/total/ETA/prep-time/KPI counters — **format from integer cents, animate display only**. | Playwright: total updates animate; value matches integer source; i18n locale format. |

Flag posture: 2.4/2.5 are additive polish (no flag). Effort: **M**.

---

# Phase 3 — Responsive image pipeline (perf #1) — *parallelizable*
**Goal:** stop serving one 800px raster through the Node origin for every thumbnail.

| # | Item | Approach | Gate/proof |
|---|------|----------|-----------|
| 3.1 | **Variant generation** | On upload (`sharp` already in `spa-proxy`), emit width variants (e.g. 160/400/800/1200) × **AVIF + WebP + JPEG fallback**; store variant keys in `product_media`/`products`. Backfill existing images via a one-off job. | API test: upload → variants exist in storage + DB; content-type correct. |
| 3.2 | **Edge serving** | Finish the R2 move: serve `/images|/media` from R2/edge CDN (not the Node API); long-cache immutable content-hashed keys (already content-addressed). | Response served from CDN, `cache-control: immutable`, not the origin process. |
| 3.3 | **`ResponsiveImage` component** | One shared component: `<picture>` with `srcset`/`sizes`, `loading="lazy"` (already used), `decoding="async"`, width/height to kill CLS, LQIP/blur placeholder. Replace raw `<img>` at menu/product/branding call-sites. | Playwright: correct variant chosen per viewport; no CLS; blur→full transition. |
| 3.4 | **Save-Data hook** | Serve the smaller variant + skip blur decode when `Save-Data`/slow `effectiveType` (ties to Phase 8 #14). | Trace under emulated 3G/Save-Data serves the small variant. |

Flag posture: variant pipeline dark until backfill done; component swap per-surface. Effort: **M–L**.

---

# Phase 4 — Real service worker (foundation for push/sync/offline)
**Goal:** replace naive cache-first-forever; unblock Phases 6–8.

| # | Item | Approach | Gate/proof |
|---|------|----------|-----------|
| 4.1 | **Strategy rewrite** | Precache versioned shell; **stale-while-revalidate** for the public menu (stop blanket-excluding `/api`, but never cache auth/mutations/`/ws`); navigation preload. Consider Workbox vs the current hand-rolled `sw.js` (keep it tiny — ponytail). | Playwright/offline: repeat `/s/:slug` opens from cache then revalidates; auth never cached. |
| 4.2 | **Update flow** | Keep `skipWaiting`/`clients.claim`; add a "new version" prompt instead of silent swap (avoids mid-order asset skew). | E2E: version bump → update prompt → reload picks new assets. |
| 4.3 | **Scope discipline** | Assert the cache never holds PII/authed responses (guardrail test). | Test: authed response is not put in cache. |

Flag posture: SW changes are risky (stale-serving bugs) — ship behind a version flag + easy kill (the
`UPDATE_CACHE_VERSION` message channel already exists). Effort: **M**.

---

# Phase 5 — Adaptive courier GPS (perf #2) — *parallelizable*
**Goal:** cut the courier's biggest battery drain without losing tracking fidelity.

| # | Item | Approach | Gate/proof |
|---|------|----------|-----------|
| 5.1 | **Adaptive sampling** | Replace `enableHighAccuracy:true`+`maximumAge:0` (`DeliveryPage.tsx`): allow cached fixes (`maximumAge` 5–15s), drop high-accuracy when stationary, sample by motion state. | Measured: fix frequency + accuracy vs baseline; tracking still smooth on the map. |
| 5.2 | **Throttled server posts** | Post on ~25m move OR ~10s, coalesce/batch; keep the active-delivery privacy guard intact. | API/WS test: post cadence bounded; positions still flow to customer track view. |
| 5.3 | **Wake Lock (Tier 3 #12)** | Acquire `navigator.wakeLock` only during an active delivery; release on complete/background. | Playwright (or manual): screen-lock held during delivery, released after. |
| 5.4 | **Telemetry** | Log fix count / post count / (where available) battery to validate the win. | Before/after numbers in the change's proof. |

Flag posture: flag the sampling change (default-on after staging validation) so it's revertible. Effort: **M**.

---

# Phase 6 — Tier 2 remainder (perceived speed + comms)
**Depends on:** Phase 4 (for 6.2).

| # | Item | Approach | Gate/proof |
|---|------|----------|-----------|
| 6.1 | **`content-visibility:auto` + virtualization** | `content-visibility:auto` + `contain-intrinsic-size` on off-screen menu sections; virtualize the admin order list (react-virtual). | Perf trace: reduced paint/layout on long lists; no scroll-jank; scroll-spy still works. |
| 6.2 | **Web Push order status** | Extend existing VAPID: push "accepted/assigned/arriving/delivered" so customers can close the tab (kills the held WS → battery/data win). | E2E: subscribe → server push → notification asserted; opt-in + revoke. |
| 6.3 | **Optimistic UI + skeletons** | Optimistic add-to-cart / place-order reconciled on server truth; skeletons on menu/order load (loading-states skill). | Playwright: instant UI feedback; rollback on server error; skeleton visible during load. |
| 6.4 | **preconnect + font subset** | `preconnect`/`dns-prefetch` to the image/CDN origin; subset per-tenant Google fonts to used glyphs (egress-safe loader exists). | First-paint KB drop; fonts still render al+en glyphs. |

Flag posture: 6.2 opt-in by user consent; rest additive. Effort: **M**.

---

# Phase 7 — Tier 3 (modern APIs, progressive enhancement)
**Depends on:** Phase 4 (for 7.1).

| # | Item | Approach | Gate/proof |
|---|------|----------|-----------|
| 7.1 | **Background Sync for order submit** | Queue the place-order POST in the SW; flush on reconnect so an order survives a dropped mobile connection. Must stay idempotent (idempotency_keys already exist). | E2E: submit offline → reconnect → exactly one order created. |
| 7.2 | **View Transitions API** | Progressive-enhance route transitions (menu→item→checkout); fallback = instant nav. | Playwright: transition present where supported; no regression where not. |
| 7.3 | **Speculation Rules prerender** | Prerender the likely next step (checkout from menu); **guard with `Save-Data`**. | Trace: next page prerendered on non-data-saver; skipped on Save-Data. |
| 7.4 | **Vibration haptics** | Subtle haptic on add-to-cart / order-confirmed / courier "arrived"; behind a user setting. | Manual + setting toggle test. |

Flag posture: all progressive-enhancement (feature-detected); 7.4 setting-gated. Effort: **M**.

---

# Phase 8 — Tier 4 (adaptive / niche)
**Depends on:** Phase 3 (8.1), Phase 4 (8.3).

| # | Item | Approach | Gate/proof |
|---|------|----------|-----------|
| 8.1 | **Network-aware loading** | Central `useNetworkQuality` (`effectiveType`/`Save-Data`) → smaller images (Phase 3.4), skip landing WebGL, defer non-critical fetches. | Trace under 2G/3G/Save-Data emulation. |
| 8.2 | **Web Share / Share Target** | Native share of storefront/menu link; optional share-target registration. | Playwright/manual: share sheet invoked with correct URL. |
| 8.3 | **Local-first menu mirror (IndexedDB)** | Only if Phase 4 SWR proves insufficient: mirror last-seen menu in IndexedDB for instant/offline open. | E2E: offline cold-open shows last menu; revalidates online. |

Flag posture: all flagged/feature-detected. **8.3 is do-only-if-needed** (overlaps Phase 4). Effort: S–M.
**Explicitly out (Tier 5):** WebGPU, Periodic Background Sync, Contact Picker, Battery Status API.

---

## Recommended execution order (by dependency + value)
1. **Phase 0** (all — cheap safety + the dep-scan gate that must precede new libs).
2. **Phase 1** → **Phase 2** (design system, then motion+bundle — they're coupled).
3. **Phase 3** and **Phase 5** in parallel with 1–2 if capacity (both independent, both high user-value).
4. **Phase 4** (SW) — required before push/sync/offline.
5. **Phase 6** → **Phase 7** → **Phase 8** (each leans on 4; descending ROI).

## Note on scope vs. your listed order
You listed the fundamentals (#1–4) last. I've pulled **0.2 dependency-scanning** to the very front because
it should scan the six new libs as they land, and **0.1 restore-fix** because it's a standalone safety trap
worth closing regardless. The rest of your ordering is preserved. If you'd rather do the UI/perf work first
and the fundamentals after, the only hard constraint is: **add the dep-scan gate before Phase 1.**
