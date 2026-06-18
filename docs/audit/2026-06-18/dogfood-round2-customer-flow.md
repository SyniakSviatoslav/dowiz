# Dogfood Round 2 — Customer Ordering Flow (dowiz.fly.dev)

**Date:** 2026-06-18 · **Target:** live prod · **Scope:** `/s/demo` menu → cart → checkout
**Note:** live runs the pre-fix build; findings below are about the customer flow (not the admin/SSR areas already patched locally).

## 🔴 CRITICAL — the public menu page does not hydrate; customers cannot order

**Source-confirmed (curl, no browser):** `GET /s/demo` serves **0 `<script src>` tags** (only `ld+json` + one inline state script), so no JS ever runs on the menu. By contrast `GET /admin`, `/s/demo/cart`, `/s/demo/checkout` each serve **2** script tags and hydrate.

Evidence chain:
1. `renderMenuPage` injects its hydration script via `loadAssetTags()` (`apps/api/src/lib/ssr-renderer.ts:12`), which **scans `apps/web/dist/index.html` / `dist/public/index.html`** for `<script src>` tags. In prod this returns `''` ("[SSR] No asset tags found, SPA will not hydrate") — the `<!--SSR_ASSETS-->` placeholder (line 366) is replaced with nothing (line 418).
2. The **correct, dedicated menu bundle exists and is deployed**: `apps/api/src/client/menu/app.ts` → built by `build-client.js` → served at **`/dist/menu/app.js` (HTTP 200 on prod)**. It contains `addToCart()` + the cart-FAB logic. It is simply **never referenced** by the menu HTML.
3. Even if the script were injected, the **SSR product cards carry only `data-product-id`** (35 of them) — **no add button, no `onclick`, no `cartFabWrapper`** element (verified in live HTML and in `ssr-renderer.ts`). So the menu→cart contract is **incompletely wired on both ends**: the SSR doesn't render the hooks the bundle expects, *and* the bundle isn't loaded.

**Impact:** A customer landing on the menu (the primary entry point) can view items but cannot add anything to the cart or start an order. `/s/demo/cart` loads but sits on "Loading…" with an empty cart (nothing can populate it). This is the strongest possible drift from the core promise — the order loop has no working entry.

**Why it's not a safe blind fix:** the remediation depends on the intended menu architecture — progressive enhancement (SSR renders add buttons + `cartFabWrapper`; reference `/dist/menu/app.js` like the sibling routes do) **vs** full-SPA hydration (mount the Vite app at `#root`). Both require real-browser + real-API verification that can't be done from here. Connects to deep-check **B2** (Dockerfile asset COPY collision can clobber the index.html `loadAssetTags` scans) and the 2026-06-09 SSR→SPA migration.

**Proposed fix (for review, NOT applied):**
- Replace the fragile `loadAssetTags()` scan with the stable pattern the cart/checkout/status routes already use: emit `<script type="module" src="/dist/menu/app.js"></script>` directly (the bundle is built + served).
- Add the missing SSR hooks the bundle needs: a per-card add control (`onclick="addToCart(event,'<id>',<price>)"` or a button the bundle binds) and the `cartFabWrapper` element.
- Verify end-to-end in a real browser against the API before shipping.

## 🟠 HIGH — production demo menu polluted with e2e test data

The public `demo` menu renders **~31 categories**, most of them e2e leftovers: `CExt-Cat-*` (×42 refs), `E2E-Cat-*`, `LC-Cat-*`, `WS2-Cat-*`, `Test-Cat-*`, `UI-FCat-*`, plus matching junk products. This is the visible symptom of deep-check **T2** (the e2e suite runs against live prod and creates persistent data). A real visitor sees a menu buried in test garbage. Remediation is process: run e2e against an isolated/ephemeral environment, or have specs clean up after themselves.

## 🟡 MEDIUM — CDN Tailwind in production

Both `renderMenuPage` and `renderClientShell` load `https://cdn.tailwindcss.com` (console: *"cdn.tailwindcss.com should not be used in production"*). Render-blocking third-party dependency, no purge/treeshake, and a runtime dependency on an external CDN for core styling. Build Tailwind into the served CSS instead.

## 🟢 LOW — missing PWA assets

`/icons/icon-192.png` 404s (repeatedly, from the manifest) and `/favicon.ico` 404s. Minor polish; the manifest references icons that aren't served.

## Already-fixed-locally, confirmed still live (pending deploy)
- **U2** double-encoded venue name (`Dubin &amp;amp; Sushi`) — visible in headings.
- **U5** HTML-escaped (invalid) JSON-LD.
