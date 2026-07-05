# Lane 3 — Frontend / Build / Bundle Strategy (rebuild-plan research)

Date: 2026-07-04 · Researcher: Lane 3 (Fable) · Scope: apps/web (20,705 LOC measured), packages/ui (14,195 LOC measured), apps/api SSR + client bundles, build tooling, i18n, assets, PWA.

Verdict up front: **no framework swap earns its keep.** The storefront's mobile problem is an
**asset problem** (icon webfont, full 3-locale i18n catalog on the critical path, single-size
images, motion library in the entry), not a runtime problem. An incremental asset diet cuts the
storefront critical payload by roughly **-850 kB font, -60–80 kB gzip JS/CSS** with S/M efforts,
which beats every rewrite option on ROI by an order of magnitude.

---

## 0. Measured baseline (verified 2026-07-04, local prod build)

Storefront `/s/:slug` (human UA → SPA) critical path, gzip unless noted:

| Asset | raw | gzip | Notes |
|---|---|---|---|
| `index-*.js` (entry) | 11.4 kB | 4.1 kB | shell: router, providers |
| `vendor-*.js` | 274.0 kB | 89.0 kB | react + react-dom + **framer-motion** (`apps/web/vite.config.ts:51`) |
| `sound-prefs-*.js` (shared @deliveryos/ui chunk) | 301.2 kB | 85.0 kB | **contains the FULL i18n catalog, all 3 locales** (verified: Albanian strings grep-hit only this chunk) |
| `chunk-6CSD*.js` | 37.3 kB | 13.4 kB | shared |
| `ClientRoutes-*.js` | 164.7 kB | 42.8 kB | storefront routes (MenuPage.tsx = 1,811 LOC) |
| **JS total (critical)** | ~789 kB | **~234 kB** | |
| `index-*.css` | 390.0 kB | 66.2 kB | Tailwind + **full tabler-icons.min.css (247.7 kB raw of the 390)** |
| `tabler-icons-*.woff2` | **820.3 kB** | (already compressed) | loads on first icon glyph — every page |

Other verified facts:

- **Three runtimes exist, not two**: (1) React 18.3 SPA (`apps/web/package.json:20-21`),
  (2) Preact 10.29 + htm + preact-render-to-string **server-side** for bot SSR
  (`apps/api/package.json` deps; `apps/api/src/lib/ssr-renderer.ts:1-3`), (3) a **vanilla-TS
  esbuild client layer**, 1,583 LOC, in `apps/api/src/client/` (menu/cart/checkout/status/pwa/widget)
  built by `apps/api/build-client.js`. Only `menu/app.js` is referenced by the SSR page
  (`apps/api/src/lib/ssr-renderer.ts:431`); grep finds **no reference to `cart/app.js`,
  `checkout/app.js`, `status/app.js`** outside `apps/api/src/client/` → likely dead code
  (hand to the dead-code sweep for confirmation via `get_dead_code`).
- SSR is **bot-UA-only**: `apps/api/src/routes/public/ssr.ts:37,46` (`isBot` →
  `renderMenuPage`, else `serveSpaShell`). Humans never see SSR HTML.
- Route-level code splitting is already done well: every surface lazy
  (`apps/web/src/main.tsx:16-29`), maplibre isolated in a lazy `map` chunk with a documented
  preload-helper hack (`apps/web/vite.config.ts:40-55`).
- i18n: hand-rolled key-major catalog SSOT, 4,343 lines / 219,160 bytes / ~2,170 keys × 3 locales
  (`packages/ui/src/lib/i18n-catalog.ts`), eagerly materialized for **all locales** at module load
  (`packages/ui/src/lib/i18n.ts:14-22` `fromCatalog`), CI parity gate exists
  (`scripts/i18n-parity.mjs` referenced at `i18n.ts:37`).
- Data layer: hand-rolled but solid `apiClient.ts` (232 lines; single-flight + cross-tab Web-Locks
  token refresh, Zod-parsed responses — `apps/web/src/lib/apiClient.ts:12-45`); `publicApi.ts`
  (45 lines); one `useWebSocket.ts` (173 lines). No react-query/SWR/zustand/redux anywhere
  (grep verified). 10 files still call `fetch(` directly outside the clients; ~10 pages hand-roll
  `isLoading` `useState` (e.g. `apps/web/src/hooks/useMenuData.ts:31-40`).
- Build: `vite build` measured **8.6 s** wall (Vite ^6, `apps/web/package.json:37`).
- Budgets: `.size-limit.json` gates `index-*` + `vendor-*` at 250 kB — **the 85 kB-gz
  `sound-prefs` shared chunk and `ClientRoutes` escape the gate entirely.** `lighthouserc.cjs:17`
  runs **`preset: 'desktop'`** — a mobile-first product is CI-gated on desktop throttling.
- Images: upload pipeline already sharp→WebP q78 but **single 1024 px size**
  (`apps/api/src/routes/spa-proxy.ts:280`); no srcset, no AVIF; product cards render ~160 px
  yet download the 1024 px asset.
- `three` (^0.184) is a dependency of apps/web (`package.json:24`) with **zero imports in src**
  and no trace in dist — dead dependency (PaperScene P1 never landed).
- PWA: SW is cache-first for shell HTML + assets (`apps/api/src/client/pwa/sw.ts:47-52`), cache
  name bumps only via an `UPDATE_CACHE_VERSION` message that **nothing ever sends** (the only
  grep hit for the string is the SW itself) → shell HTML is cached forever-stale. The comment in
  `apps/web/index.html:24` still claims "sw.js is non-caching" — drift. This is the
  `apps/api/public/sw.js` "prior defect" biomarker class.

---

## 1. The React+Preact(+vanilla) dual → **KEEP, formalized (option c)** — effort S, risk low

- **(a) Preact everywhere: REJECT.** preact/compat is documented-broken with this exact dep set:
  react-router v7 under Preact yields empty/non-interactive documents
  ([react-router#13261](https://github.com/remix-run/react-router/discussions/13261)) and
  framer-motion fails under compat
  ([fresh#591](https://github.com/denoland/fresh/discussions/591),
  [motion#1369](https://github.com/framer/motion/issues/1369)). Unifying would force replacing the
  router (wouter-preact) **and** the motion layer across 35k LOC for a ~40 kB-gz win
  (react+react-dom ≈ 44 kB gz vs preact/compat ≈ 4–5 kB). Effort L, risk high, gain 40 kB —
  the asset diet below yields more for less.
- **(b) React SSR everywhere: REJECT.** Server-rendering the 20.7k-LOC SPA means making it
  SSR-safe (window/localStorage guards throughout, e.g. `i18n.ts:7`), pulling react-dom/server
  into the Fastify hot path, and re-solving hydration for admin surfaces nobody crawls. Bot SEO is
  already served by the Preact renderer (442 lines, LRU-cached — `apps/api/src/lib/ssr-renderer.ts`).
- **(c) Keep + formalize: ADOPT.** The server Preact layer is ~1.6k LOC of *templates*, not a
  second app. Formalize the seam: (i) one shared menu-DTO type consumed by both renderers
  (already partially via `@deliveryos/shared-types` — `ssr-renderer.ts:6`); (ii) a contract test
  asserting SSR HTML and SPA render the same product names/prices for `/s/demo`; (iii) **delete**
  the unreferenced `cart/checkout/status` client bundles after dead-code confirmation.

## 2. Build tooling → **Vite stays; bump 6→8 (Rolldown) opportunistically** — effort S, risk low-med

Vite 8 (stable March 2026) ships Rolldown as the default bundler; Rolldown 1.0 locked its API
May 2026; Cloudflare acquired VoidZero June 2026 (continuity is fine)
([Vite 8 announcement](https://vite.dev/blog/announcing-vite8),
[Rolldown 1.0](https://voidzero.dev/posts/announcing-rolldown-1-0),
[Rolldown integration guide](https://v7.vite.dev/guide/rolldown)). At **8.6 s** measured prod
build and a small dev graph, Rspack/Turbopack migrations are pure cost (config rewrite, plugin
risk) for seconds saved → **REJECT both**. Vite 6→8 is the free-ish win (expect ~2–4× on the
bundle step, i.e. seconds), **but** two local hazards must be re-verified under Rolldown:
the `manualChunks` function with the **preload-helper routing hack**
(`apps/web/vite.config.ts:46` — Rolldown prefers `advancedChunks`; the vendor/map isolation and
the "don't split React" constraint at `vite.config.ts:37-39` must be re-proven with a
storefront-LCP check), and `@vitejs/plugin-react` version pairing. Do it as its own PR with
size-limit + lhci proof, not bundled into feature work.

## 3. Framework alternatives (Solid/Svelte/Qwik/Astro) → **REJECT rewrite; Astro not now** — gain doesn't clear cost

Honest numbers: post-asset-diet the storefront critical JS is ~150–170 kB gz on a lazy-loaded,
route-split SPA. Solid/Svelte would cut maybe 40–60 kB gz of runtime at the price of rewriting
35k LOC of components + packages/ui + the voice layer — years of churn for one Lighthouse tier.
Qwik's resumability solves a hydration cost this app doesn't measurably have (hydration work is
small next to the 820 kB font + images). Astro islands/server islands
([docs.astro.build/concepts/islands](https://docs.astro.build/en/concepts/islands/)) are the right
architecture *for content sites*; here the only public page is `/s/:slug`, which is
cart-interactive end-to-end, and bot-SEO is already served. Adopting Astro would add a fourth
runtime + second server. **Only revisit** if marketing/SEO page count grows (landing, city pages,
venue directories) — then Astro for those pages specifically, storefront untouched.

## 4. Data fetching / state → **ADOPT TanStack Query for admin/courier only** — effort M, risk low

`apiClient` (auth/refresh/Zod) is good and stays as the fetch layer. What's missing is the
*caching/lifecycle* layer: ~10 pages hand-roll `isLoading/error` state, refetch-on-focus and
WS-driven invalidation are ad hoc, and 10 direct `fetch(` calls bypass the clients (converge
those first — that's the existing `refactor-converge` loop's job). TanStack Query v5 is
13.4 kB gz, zero deps ([bundlephobia](https://bundlephobia.com/package/@tanstack/react-query)).
Scope the `QueryClientProvider` **inside** `AdminRoutes`/`CourierRoutes` so the storefront pays
0 bytes; queryFn = existing `apiClient`. Storefront keeps its single one-shot menu fetch (adding
a cache lib there is YAGNI). WS messages → `queryClient.invalidateQueries` replaces bespoke
refetch plumbing. **REJECT** any state library (zustand/redux) — CartProvider + context is
sufficient and working.

## 5. i18n → **KEEP the catalog SSOT; add a build-time split** — effort M, risk low; PILOT paraglide only if the split stalls

The hand-rolled catalog + parity gate is architecturally the same idea as compile-time i18n —
what's missing is only *emission granularity*: today all ~2,170 keys × 3 locales
(~219 kB source) land in one always-loaded chunk. Fix in-house: a codegen step (which the repo
already does for parity checking) that emits `catalog.sq.ts` / `catalog.en.ts` / `catalog.uk.ts`
(+ optionally storefront/admin key subsets keyed by prefix), loaded per active locale via dynamic
import in `I18nProvider`. Expected: sound-prefs chunk −40–55 kB gz on the storefront critical
path. A library swap (i18next: **adds** 200+ kB class runtime patterns; paraglide: compiled,
47 kB vs 205 kB in its benchmark, [paraglidejs.com/benchmark](https://paraglidejs.com/benchmark))
would buy tree-shaking per *message*, but costs a migration of ~2,170 keys, the custom parity
gate, and the `t(key, fallback, options)` call convention across both packages — not worth it
while the in-house split is a ~1-day change. Keep `sq`-first defaults (note: locale codes are
`sq/en/uk`, `i18n.ts:3`).

## 6. Bundle/asset pipeline & CWV → the actual wins, ranked (all storefront-LCP relevant)

| # | Change | Effort | Gain (quantified) |
|---|---|---|---|
| 6.1 | **Icon webfont → tree-shaken SVGs.** 165 distinct `ti ti-*` classes used (grep, apps/web+ui) vs ~5,900 shipped. Replace `@tabler/icons-webfont` import (`apps/web/src/main.tsx:8`) with a generated SVG-sprite/icon component from `@tabler/icons` sources; codemod the class names. | M | **−820 kB woff2** off every page + ~−248 kB raw CSS (−~35–40 kB gz of the 66 kB CSS); removes a render-blocking font fetch on 3G/4G AL networks |
| 6.2 | **i18n per-locale split** (§5) | M | −40–55 kB gz critical JS |
| 6.3 | **Images: srcset + AVIF.** Emit 320/640/1024 + AVIF alongside WebP at upload (sharp already there, `spa-proxy.ts:280`); `<img srcset sizes loading=lazy>` in MenuPage cards + SSR renderer. AVIF ≈ −20–30% vs WebP at same quality. | S/M | product-card payload −70–85%; direct LCP win (LCP element on `/s/:slug` is the hero/card image) |
| 6.4 | **framer-motion diet.** Entry-level `MotionConfig/AnimatePresence` page transitions (`main.tsx:4,42-48`) keep ~45 kB gz of motion in `vendor` for every visitor; MenuPage uses 46 `motion.` calls. Move to `LazyMotion + m` with `domAnimation` (~15–20 kB gz saved) or CSS-transition page fades (~40 kB saved). Also: `framer-motion` v12 is the legacy alias — the maintained package is `motion` ([motion.dev upgrade guide](https://motion.dev/docs/react-upgrade-guide)); swap on the React 19 bump. | M | −15–40 kB gz vendor |
| 6.5 | **Budget-gate the whole storefront.** `.size-limit.json` misses `sound-prefs`/`ClientRoutes`; change to "all `assets/*.js` minus `map-*`/Admin/Courier ≤ N". Switch `lighthouserc.cjs` to `preset: 'mobile'` (or add a mobile run) — desktop preset on a mobile-first product hides regressions. | S | closes the gate the last 3 findings walked through |
| 6.6 | Remove dead `three` dep (`apps/web/package.json:24`); confirm-and-delete `apps/api/src/client/{cart,checkout,status}` bundles | S | hygiene; install/CI time |

React 18→19: bundle-neutral, do it as a standalone chore after 6.4's motion swap (framer-motion
v12/motion v12 line is React-19-compatible; react-router v7 already is —
[react.dev/blog React 19](https://react.dev/blog/2024/12/05/react-19)).

## 7. PWA/offline → **fix the SW defect first; no Workbox** — effort S, risk low

The current SW is a live staleness bug (see §0: cache-first HTML, version bump never triggered).
For cash-first ordering on spotty networks the honest offline scope is: (i) **navigations
network-first with cache fallback** (menu stays browsable in a tunnel, updates when online);
(ii) hashed `/assets/*` cache-first forever (safe — content-addressed); (iii) API stays
network-only (an order MUST NOT pretend to submit offline — queuing a POST that charges cash on
delivery without server ack is a money-path landmine, keep it out of the SW). That's a ~30-line
rewrite of `apps/api/src/client/pwa/sw.ts` + deleting the dead `UPDATE_CACHE_VERSION` path — no
Workbox/vite-plugin-pwa needed for 3 routes of logic (Workbox shines at precache-manifest scale;
[developer.chrome.com/docs/workbox](https://developer.chrome.com/docs/workbox/using-workbox-without-precaching)).
Add a regression guardrail: an E2E that deploys asset-hash N+1 and asserts the client picks up the
new shell on next navigation (this is the red→green for the prior-defect biomarker).

---

## Recommendation table

| # | From → To | Effort | Risk | Expected gain | Verdict |
|---|---|---|---|---|---|
| 1 | React SPA + Preact bot-SSR dual → same, formalized (shared DTO + SSR↔SPA contract test; delete dead client bundles) | S | Low | dual stops rotting; −~1k dead LOC | **KEEP** (formalize) |
| 1a | → Preact everywhere | L | High (router+motion compat broken) | ~−40 kB gz | **REJECT** |
| 1b | → React SSR everywhere | L | Med | SEO already covered | **REJECT** |
| 2 | Vite 6 → Vite 8/Rolldown | S | Low-Med (manualChunks/preload-helper hack) | build 8.6 s → ~2–4 s; toolchain current | **ADOPT** (own PR, re-verify chunking) |
| 2a | → Rspack / Turbopack | M | Med | none at this size | **REJECT** |
| 3 | React → Solid/Svelte/Qwik rewrite; or Astro storefront | XL | High | ~40–60 kB gz vs 35k-LOC rewrite | **REJECT** (Astro: revisit only for future marketing/SEO pages) |
| 4 | hand-rolled per-page fetch state → TanStack Query v5 scoped to admin/courier (apiClient stays as queryFn) | M | Low | kills ~10 hand-rolled loading/error copies; WS→invalidateQueries; 0 storefront bytes | **ADOPT** |
| 5 | monolithic i18n catalog chunk → build-time per-locale (+surface) split; catalog stays SSOT | M | Low | −40–55 kB gz critical JS | **ADOPT** (paraglide = PILOT only if split stalls) |
| 6.1 | tabler icon webfont → tree-shaken SVG (165 used) | M | Low | **−820 kB** font + −~35 kB gz CSS | **ADOPT** (top CWV win) |
| 6.3 | single 1024px WebP → srcset 320/640/1024 + AVIF | S/M | Low | card payload −70–85%; LCP | **ADOPT** |
| 6.4 | full framer-motion in vendor → LazyMotion/CSS transitions (then `motion` pkg on React 19 bump) | M | Med (visual) | −15–40 kB gz | **ADOPT** |
| 6.5 | size-limit/lhci gaps → gate all storefront chunks + mobile preset | S | Low | closes budget blind spot | **ADOPT** |
| 7 | cache-first-forever SW → network-first navigations, cache-first hashed assets; API never cached | S | Low (E2E-gated) | fixes live staleness defect; real offline browse | **ADOPT** |

**Sequencing (incremental, each own PR + proof):** 6.5 (gates first) → 7 (SW defect) → 6.1 →
6.3 → 5 → 6.4 → 4 → 2 → 1 formalization → React 19 chore.

**Mandatory-Proof flags:** §6.4 (MenuPage/checkout animations), §7 (SW affects checkout shell),
§6.3 (SSR renderer markup) all touch or wrap the order path → each requires the Playwright E2E
against `/s/demo` → checkout flow per the Mandatory Proof Rule before "done". §7's network-only
API rule is a money red-line: never queue order POSTs offline.

**Sources:** [vite.dev/blog/announcing-vite8](https://vite.dev/blog/announcing-vite8) ·
[voidzero.dev/posts/announcing-rolldown-1-0](https://voidzero.dev/posts/announcing-rolldown-1-0) ·
[v7.vite.dev/guide/rolldown](https://v7.vite.dev/guide/rolldown) ·
[github.com/remix-run/react-router/discussions/13261](https://github.com/remix-run/react-router/discussions/13261) ·
[github.com/denoland/fresh/discussions/591](https://github.com/denoland/fresh/discussions/591) ·
[github.com/motiondivision/motion/issues/1369](https://github.com/framer/motion/issues/1369) ·
[paraglidejs.com/benchmark](https://paraglidejs.com/benchmark) ·
[bundlephobia.com/package/@tanstack/react-query](https://bundlephobia.com/package/@tanstack/react-query) ·
[docs.astro.build/en/concepts/islands](https://docs.astro.build/en/concepts/islands/) ·
[motion.dev/docs/react-upgrade-guide](https://motion.dev/docs/react-upgrade-guide) ·
[react.dev/blog/2024/12/05/react-19](https://react.dev/blog/2024/12/05/react-19) ·
[developer.chrome.com/docs/workbox](https://developer.chrome.com/docs/workbox/using-workbox-without-precaching)
