# Recon #2 — Performance / Scale / N+1 / Resource findings (2026-07-03)

> READ-ONLY deep recon, run #2. Excludes run #1's classes (unbounded per-tick worker scans,
> pool-exhaustion, prepared-stmt-on-pooler). Every finding: file:line · scaling curve ·
> symptom at scale · one-line fix. Ranked. No code was changed.

**Counts: 4 CRITICAL · 8 HIGH · 10 MEDIUM · 6 LOW = 28 findings.**

**Worst N+1:** the owner order board runs `(SELECT COUNT(*) FROM order_items WHERE order_id = o.id)`
per returned order row (`apps/api/src/routes/owner/dashboard.ts:69`, echoed at
`apps/api/src/lib/orderStatusService.ts:27`) while **`order_items` has no index on `order_id` at all**
(`packages/db/migrations/1780310074262_orders.ts` creates the table index-free; no later migration adds one).
One board load = up to 100 sequential seq-scans of the GLOBAL order_items table —
O(page_size × total order_items across ALL tenants) per refresh.

---

## CRITICAL

### C1 — Synchronous `execFileSync` OCR freezes the entire event loop (up to 120 s)
- `apps/api/src/lib/ai-ocr-parser.ts:308-331` — `paddleOcr()` does `writeFileSync` + `execFileSync(py, [script, tmp], { timeout: 120_000 })`.
- Reached from the request path: `apps/api/src/routes/owner/menu-import.ts:101` (`await parser.parse(...)`) and the **public** `POST /menu/import/anonymous` (`menu-import.ts:172-199`, 5/min/IP).
- Curve: O(1) per request, but the cost is **global** — the single Node process (HTTP, WS heartbeats, workers) is frozen for the full subprocess duration.
- Symptom: one owner uploading a menu photo (with `MENU_OCR_ENGINE=paddle`) stalls every customer checkout, WS ping, and worker tick for up to 2 minutes; health checks can fail → machine restart.
- Fix: swap to async `execFile` (or move OCR behind the existing queue as a background job returning a session id).

### C2 — No index on `order_items(order_id)` → the worst N+1 (dashboard board, order fetches)
- Query sites: `apps/api/src/routes/owner/dashboard.ts:69` (correlated COUNT per row), `apps/api/src/lib/orderStatusService.ts:27`, `apps/api/src/routes/orders.ts:762,796,821`, `apps/api/src/routes/customer/orders.ts:58-62`, `apps/api/src/routes/spa-proxy.ts:405,863,871`, `apps/api/src/notifications/workers/index.ts:558`.
- Absence evidence: `packages/db/migrations/1780310074262_orders.ts` (table created with zero indexes); `1780338982023_order_items_product_fk_set_null.ts` only touches the FK. Postgres does not auto-index FKs.
- Curve: each lookup O(total order_items across ALL tenants); board = O(100 × that) per load.
- Symptom: order board latency grows linearly with platform-wide order history; DB CPU burns on seq-scans of the fastest-growing table.
- Fix: `CREATE INDEX order_items_order_id_idx ON order_items(order_id);`

### C3 — No index on `products(location_id, is_available)` — hottest public path + RLS predicate
- Query sites: `packages/db/migrations/1790000000064_read-public-menu-perf.ts:132-136` (`read_public_menu`, executed on every cold storefront menu view / SSR), `apps/api/src/routes/owner/products.ts:72`. RLS `tenant_isolation` predicate is `location_id` (`1780310072731_menu.ts:44`) — also unindexed.
- Absence evidence: `1780310072731_menu.ts` creates `products` index-free; the only index is partial `products_loc_extkey_uniq ... WHERE external_key IS NOT NULL` (`1780338982026_menu_external_keys.ts:12`) — excludes UI-created products.
- Curve: O(total products across ALL tenants) seq-scan per cold menu render. The 30 s per-instance menu cache (`apps/api/src/routes/public/menu.ts:89-101`) amortizes but doesn't cure: every (slug × locale × instance × 30 s) still pays it, as does SSR.
- Symptom: storefront TTFB degrades as ANY tenant grows their menu; the tenant-count curve is the killer.
- Fix: `CREATE INDEX products_location_available_idx ON products(location_id, is_available);`

### C4 — Public `/api/telemetry`: `rateLimit: false` + per-event INSERT loop + no retention
- `apps/api/src/routes/public/telemetry.ts:38` — `config: { rateLimit: false }` on an unauthenticated endpoint; `:52-61` per-event INSERT loop (≤10 events) + `:63-72` per-CWV-metric INSERT loop, each a separate round trip on a checked-out client.
- No `DELETE FROM analytics_events` / retention job anywhere in `apps/api/src` (grep-verified) → the table grows forever, and it feeds owner-analytics GROUP BYs.
- Curve: O(traffic) unbounded writes; 1 HTTP request → up to ~20 sequential INSERTs.
- Symptom: cheap unauthenticated write amplification (DB bloat + pool pressure), analytics scans slow down monotonically over time.
- Fix: restore a rate limit + collapse to one multi-row `INSERT ... SELECT unnest(...)` + add a retention sweep (e.g. 90 d).

---

## HIGH

### H1 — menu-translate holds a pooled client across external HTTP calls + per-row UPDATE loops
- `apps/api/src/routes/owner/menu-translate.ts:35` — entire flow runs inside one `withTenant` client; `:90`, `:141`, `:187` await `translation.translate()` = external `fetch` to LibreTranslate (`apps/api/src/lib/libretranslate-provider.ts:26`); `:93-99`, `:144-150`, `:190-196` then upsert row-by-row.
- Curve: O(locales × (categories + products + modifiers)) sequential round trips, with the pool connection held for the full external-API latency.
- Symptom: one translate of a 200-item menu into 2 locales ≈ 600+ sequential queries + minutes of a held connection — pool starvation for the whole tenant during the call.
- Fix: read texts → release client → translate → re-acquire and bulk-upsert via one multi-row `INSERT ... ON CONFLICT`.

### H2 — Tesseract OCR per request, no concurrency cap, publicly reachable
- `apps/api/src/lib/ai-ocr-parser.ts:408-412` — `Tesseract.recognize(input.bytes, 'sqi+eng', ...)` spawns a fresh worker + loads WASM/traineddata per call; reachable via public `POST /menu/import/anonymous` (5/min/IP but unlimited across IPs).
- Curve: O(concurrent imports) in CPU cores and ~100 MB-class memory each.
- Symptom: a handful of parallel image imports saturate the machine's CPU/RAM; co-located API latency spikes.
- Fix: single shared Tesseract worker (createWorker once) behind a concurrency-1 queue.

### H3 — pdfjs PDF parse forced onto the main thread ("fake worker")
- `apps/api/src/lib/ai-ocr-parser.ts:349-351` — `(globalThis as any).pdfjsWorker = workerMod` disables the real worker → `getDocument(...)` + the per-page loop `:354-377` run CPU-bound on the event loop, on up to 10 MB uploads.
- Curve: O(pages × fragments) on-thread per import.
- Symptom: multi-second event-loop stalls during any PDF import (same class as C1, smaller magnitude).
- Fix: run `parse()` inside a `worker_threads` worker (fixes H2/H3 together).

### H4 — Customer-analytics: missing `orders(customer_id)` index + un-LIMITed GROUP BYs + waterfall
- `apps/api/src/routes/spa-proxy.ts:860-880` — three **sequential** queries; `:871` (prefs) and `:878` (heatmap) GROUP BY with no LIMIT over `WHERE o.customer_id = $1 AND o.location_id = $2`; only `(location_id,status)` / `(location_id,created_at)` exist (`1780310074262_orders.ts:44-45`) → scans all of a tenant's orders.
- Curve: O(orders per tenant) × 3, serialized.
- Symptom: CRM drawer gets slower with tenant history; 3× RTT floor.
- Fix: `CREATE INDEX orders_location_customer_created_idx ON orders(location_id, customer_id, created_at DESC);` + `Promise.all` the three reads.

### H5 — Checkout writes order items/modifiers row-by-row inside the held money transaction
- `apps/api/src/lib/order-persistence.ts:116-131` — nested `for` loops: one INSERT per item, one per modifier, sequential, inside the open checkout tx (the read side was already batched — see the comment at `apps/api/src/routes/orders.ts:443-449` — the write side was not).
- Curve: O(cart items + modifiers) round trips while holding locks + a pool client on the hottest money path.
- Symptom: large carts extend tx hold time linearly → lock contention and pool pressure at rush hour.
- Fix: two multi-row `INSERT ... VALUES ... RETURNING id` (or UNNEST) statements.

### H6 — `courier_shifts` has only a partial dispatch index; courier heartbeats seq-scan
- Query sites: `apps/api/src/routes/courier/shifts.ts:29,124,137,198,220,355,374` (`WHERE courier_id = $1 AND location_id = $2 AND status IN (...) ORDER BY started_at DESC`), `apps/api/src/lib/shiftService.ts:18`.
- Absence: `1780421036157_courier-shifts.ts:14` defines only `courier_shifts_dispatch_idx(location_id,status) WHERE status='available'` — no `courier_id` index, and `on_delivery` rows fall outside the partial.
- Curve: O(total shifts across all tenants) per heartbeat/ping — and heartbeats fire every few seconds per active courier.
- Symptom: the highest-frequency courier write path degrades platform-wide as shift history accumulates.
- Fix: `CREATE INDEX courier_shifts_courier_idx ON courier_shifts(courier_id, location_id, status);`

### H7 — Full 3-locale i18n catalog (~211 KB, 4296 lines) statically in the entry bundle
- `packages/ui/src/lib/i18n.ts:2` static-imports `i18n-catalog.js`; `:21` eagerly materializes ALL locales (sq/en/uk) at module eval; pulled into the entry graph via `apps/web/src/main.tsx:5,74` (`I18nProvider`).
- Curve: O(bundle) fixed tax on every route — including the customer storefront `/s/:slug` LCP.
- Symptom: every first paint downloads + parses ~211 KB of translations, two-thirds of it for languages the visitor never sees.
- Fix: split per-locale files + `await import('./i18n-catalog.${locale}.js')` for the active locale only.

### H8 — Admin bundle not code-split: 10 pages eager, 1 lazy
- `apps/web/src/routes/AdminRoutes.tsx:7-16` eagerly imports Dashboard, MenuManager (1292 lines), Branding, Couriers, CRM, Settings, Onboarding, Activation, SupplyLibrary, Promotions; only `AnalyticsPage` is lazy (`:18`). `vite.config.ts:40-55` splits only maplibre + vendor.
- Curve: O(sum of all admin pages) on first admin paint.
- Symptom: owner opening the order board downloads the entire admin surface before interactivity.
- Fix: wrap each page in `React.lazy` exactly like the `AnalyticsPage` pattern at `:18`.

---

## MEDIUM

### M1 — Storefront MenuPage re-render storm
- `packages/ui/src/components/client/ProductCard.tsx:53` — ProductCard NOT memoized (contrast `admin/OrderCard.tsx:31`); `apps/web/src/pages/client/MenuPage.tsx:1186-1219` passes fresh object + closures per card; `:237/:275-337` search state re-filters everything per keystroke; `:526` scroll-spy `setSelectedCategory` fires on every intersection tick.
- Curve: O(items) framer-motion re-renders per keystroke AND per scroll event.
- Symptom: janky typing/scroll on large menus (the exact surface sold to prospects).
- Fix: `React.memo(ProductCard)` + `useCallback` handlers + only set scroll-spy state on change.

### M2 — MenuManagerPage (1292-line monolith) re-renders all cards per keystroke
- `apps/web/src/pages/admin/MenuManagerPage.tsx:350` (searchQuery) → `:611-627` filter runs every render → `:786-790` nested full `.map` with no extracted/memoized item component; input at `:676`.
- Curve: O(products) per keystroke. Fix: extract memoized card + `useMemo` filtered list + debounce.

### M3 — No list virtualization anywhere (no react-window/virtuoso in either package.json)
- Full-DOM renders: `MenuManagerPage.tsx:786-790`, `MenuPage.tsx:1121→1151`, `DashboardPage.tsx:684` — each item a framer-motion node.
- Curve: O(items) DOM nodes. Symptom: slow mount/scroll at 200+ products or busy order queues.
- Fix: `content-visibility:auto` + `contain-intrinsic-size` as the cheap first step; virtualize if it persists.

### M4 — OrderCard memo defeated by inline props
- `apps/web/src/pages/admin/DashboardPage.tsx:703` fresh `onViewDetail` arrow + `:688-691` inline `variants` object → `memo(OrderCard)` never bails out.
- Curve: O(orders) re-render per WS status ping. Fix: `useCallback` + hoist variants to module const.

### M5 — Filtered order board lacks a `(location_id, status, created_at DESC)` composite
- `apps/api/src/routes/owner/dashboard.ts:64-75` — status filter + `ORDER BY created_at DESC` can't be served by either existing single-purpose index (`1780310074262_orders.ts:44-45`) → sort per filtered load.
- Curve: O(matching orders per tenant) sort per request. Fix: composite index superseding `orders_location_status_idx`.

### M6 — Owner analytics `topProducts` aggregates ALL order_items ever (no time bound)
- `apps/api/src/routes/spa-proxy.ts:320-327` — `FROM order_items oi ... JOIN orders o ... WHERE o.location_id = $1 GROUP BY ...` with no `created_at` filter (siblings at `:330` and `:345` are bounded to 7 d/30 d).
- Curve: O(tenant's entire item history) per analytics page view, forever growing. Fix: add `AND o.created_at >= NOW() - INTERVAL '90 days'`.

### M7 — Customer tracking endpoint: 5 sequential queries on a polled path
- `apps/api/src/routes/customer/orders.ts:31` (order) → `:58` (items) → `:67` (courier pos) → `:105` (rating) → `:116` (route) — serialized awaits; customers poll this while waiting for delivery.
- Curve: 5× RTT × poll rate × concurrent waiting customers. Fix: `Promise.all` the 4 independent reads after the order row.

### M8 — Couriers live board: correlated `MAX(recorded_at)` subquery by `shift_id` — column has NO index
- `apps/api/src/routes/owner/couriers.ts:165-167` — `cp.recorded_at = (SELECT MAX(recorded_at) FROM courier_positions WHERE shift_id = cs.id)`; courier_positions is indexed on `(courier_id, recorded_at)` and `(location_id, recorded_at)` only — **not** `shift_id`.
- Curve: O(shifts × total positions) per board poll. Fix: rewrite as `LEFT JOIN LATERAL ... WHERE courier_id = cs.courier_id ORDER BY recorded_at DESC LIMIT 1` (uses the existing index), like `dashboard.ts:96-100` already does.

### M9 — Delivered-route endpoint returns the raw breadcrumb unbounded
- `apps/api/src/routes/owner/couriers.ts:226-231` — `SELECT lat,lng,recorded_at ... BETWEEN start AND end ORDER BY recorded_at` with no LIMIT/downsampling; positions persist on every ping.
- Curve: O(pings during delivery) rows JSON-serialized per view (a 2 h delivery at 5 s pings ≈ 1440 rows). Fix: downsample in SQL (`row_number() % n = 0`) or cap with LIMIT + stride.

### M10 — `SELECT *` over-fetch on owner list endpoints
- `apps/api/src/routes/owner/products.ts:72,110`, `owner/categories.ts:69,102`, `owner/menu-availability.ts:87`, `owner/themes.ts:25,78` — full-row fetch including jsonb/attribute columns the list UI never shows.
- Curve: O(rows × row width) transfer per list load. Fix: name the columns the endpoint actually returns.

---

## LOW

- **L1** — `order_status_history` has zero indexes (`1780338982015_order_history.ts`); write-only today, but FK cascades from `orders` and RLS on `location_id` seq-scan. Fix: `CREATE INDEX order_status_history_order_idx ON order_status_history(order_id);`
- **L2** — `modifier_groups(location_id)` only partially indexed (`1780338982026_menu_external_keys.ts:13`); hit by `read_public_menu` (`1790000000064_read-public-menu-perf.ts:113`). Small tables — behind C3 in priority.
- **L3** — Onboarding `menu_items` inserted one-by-one on a held client: `apps/api/src/routes/spa-proxy.ts:797-808`. One-time path; batch when touched.
- **L4** — PUT product modifier-groups: delete-then-loop-insert `apps/api/src/routes/owner/products.ts:283-290`. Small payloads; batch when touched.
- **L5** — framer-motion pinned into the always-loaded vendor chunk (`apps/web/src/main.tsx:4`, `vite.config.ts:51`; 37 import sites). Fix: `LazyMotion` + `m` + `domAnimation` if storefront bundle budget tightens.
- **L6** — `three@^0.184.0` in `apps/web/package.json:24` with ZERO import sites (tree-shaken today, a landmine tomorrow); also `MenuManagerPage.tsx:461-462` post-save sequential refetch → `Promise.all`.

## Explicitly checked, NOT findings
- Order-create read side already batched set-based (`orders.ts:399-465` — modifier groups fixed per its own comment).
- Dashboard snapshot pagination is real keyset `(created_at,id)` (`dashboard.ts:44-56`); `countSql` (`:77-81`) is covered by `orders_location_status_idx`.
- Public menu has a real stale-while-revalidate cache with a size bound (`public/menu.ts:76-220`).
- Storefront images: lazy-loaded inside aspect-ratio boxes (`ProductCard.tsx:92-102`) — no CLS issue.
- Voice/whisper (`packages/voice`) has no web importer — not in the bundle. No `ILIKE '%…%'` on hot paths. `getLocationId` per-request membership check (`spa-proxy.ts:57-84`) is a deliberate ADR-0004 security re-check — N/A, not a perf bug.
- WS heartbeat/room-GC loops (`websocket.ts:288,301`) are O(connections)/30-60 s — fine.
