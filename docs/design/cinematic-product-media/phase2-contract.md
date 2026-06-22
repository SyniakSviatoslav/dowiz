# Phase 2–5 Build Contract (shared by all parallel agents)

Single source of truth so independently-built pieces slot together. Feature is DARK behind
`MEDIA_RICH_ENABLED` (default false) — never launch, only build + dark-deploy. Repo discipline:
**no heavy deps** (no three.js/WebGL/hls.js/Lottie/carousel/video lib/dnd-kit), match existing
conventions, code-split every renderer, one heavy decode at a time, every external call has a
timeout + fallback.

## Schema (already shipped — migration 054/055)
`product_media(id, location_id, product_id, kind ['image'|'video'|'spin'|'model'], storage_key,
mime_type, bytes, width, height, duration_ms, poster_key, alt, sort_order, available, meta jsonb,
created_at)`. `products.primary_media_id` (nullable FK). `locations.plan ('free'|'business')`.
`read_public_menu` now emits `primary_media_id` per product (migration 055).

## TS types (owner: Data agent → packages/shared-types + apps/web client Product type)
```ts
export type ProductMediaKind = 'image' | 'video' | 'spin' | 'model';
export interface ProductMedia {
  id: string;
  kind: ProductMediaKind;
  url: string;                 // resolved absolute URL to the asset (server resolves storage_key → /media/ or passthrough)
  posterUrl?: string | null;   // video/spin poster (raster only)
  mimeType: string;
  width?: number | null;
  height?: number | null;
  durationMs?: number | null;
  alt?: string | null;
  sortOrder: number;
  meta?: { frameCount?: number; frameUrls?: string[] } | null; // spin: ordered frame URLs
}
```
Client `Product` (apps/web MenuPage) gains: `primary_media_id?: string | null`.

## Lazy media endpoint (owner: Backend agent)
`GET /api/public/locations/:slug/products/:productId/media` →
`{ media: ProductMedia[] }`.
- Gated SERVER-SIDE: if `MEDIA_RICH_ENABLED !== 'true'` OR the location's `plan !== 'business'`
  → return `{ media: [] }` (200). Defence in depth (client also won't import chunks).
- Filters `available = true`, orders by `sort_order`.
- Resolves each row to `ProductMedia`: `url` = if `storage_key` looks like `http(s)://` use it
  verbatim (seed/external), else `/media/${storage_key}`. Same for `poster_key`→`posterUrl`.
  For `kind='spin'`, map `meta.frameKeys[]`→`meta.frameUrls[]` through the same resolver.
- `Cache-Control: public, max-age=60, stale-while-revalidate=300` (reflects plan/availability
  changes without a long stale window — Phase-2 X-blocker H1/H4).

## Media serve route (owner: Backend agent) — extends the existing `/images/*` proxy pattern
`GET /media/*` → fetch the R2 object by key (traversal-guarded like `/images/*`), serve with
`Cache-Control: public, max-age=31536000, immutable`. Reuse `apps/api/src/lib/r2-storage.ts`.

## Upload (owner: Backend agent) — presign + confirm (R2 not on staging → unit-prove logic only)
- `POST /api/owner/menu/products/:productId/media/presign` (auth, rate-limit 10/min): body
  `{ kind, items:[{ mimeType, bytes, sha256 }] }`. Validates: mime allow-list
  (image/webp,image/jpeg,video/mp4; poster raster only, NEVER svg), per-file size ceiling,
  spin frame-count 12–72, AND per-location `SUM(bytes)+incoming ≤ 150MB`. Over → 413. Returns
  short-TTL (≤5min) presigned PUT URLs for content-addressed keys
  `{locationId}/{productId}/{kind}/{sha256-12}.{ext}`. Tenant-scoped (can't write another prefix).
- `POST .../media/confirm`: re-validates magic-bytes server-side, writes ONE product_media row
  (with meta.frameKeys for spin) via `withTenant(server.db, userId,…)` on the operational pool
  (RC1: never a raw BYPASSRLS write). location_id scoped from membership server-side.
- `POST .../media/:mediaId/set-primary` (read-before-write, skip no-op → bump only on change),
  `POST .../media/reorder` (single tx, no bump), `PATCH .../media/:mediaId` (available toggle).
- Magic-byte + budget + tier-gate logic must be UNIT-TESTED (node --test) — R2 absent on staging.

## Client MediaRenderer registry (owner: Renderer agent — NEW files only, do NOT touch MenuPage)
Location: `apps/web/src/components/media/`. Export:
- `MediaRenderer` ({ media: ProductMedia; active: boolean; posterFallbackUrl?: string }) — a
  `switch(media.kind)`: `image` rendered inline (no chunk); `video|spin` are `React.lazy`
  code-split chunks loaded only when that kind is present AND `active`. Unknown/model → poster→
  posterFallback→nothing. Each renderer wrapped so a failure degrades to poster, never throws.
- `SpinViewer` (Canvas-2D or `<img>` frame scrub on pointer drag; poster→first-frame→interactive;
  per-frame fetch timeout 4s → stay on poster; `prefers-reduced-motion` → poster only, no RAF;
  teardown: cancel RAF + clear on unmount). ≤4KB gz.
- `VideoClip` (native `<video>` muted loop; poster→play; `navigator.connection.saveData` or
  effectiveType∈{slow-2g,2g,3g} → poster only, NO video fetch; WCAG 2.2.2 pause control;
  `prefers-reduced-motion` respected; offscreen/unmount → pause + release `src` = no leaked
  decoder). ≤2KB gz.
- `MediaGallery` ({ media: ProductMedia[]; ... }) orchestrator, active when media.length>1: ONE
  heavy decode at a time (only the active slide renders video/spin; neighbours poster-prefetch
  only), prev/next + dots, ZERO auto-advance, ZERO CLS (slides share the hero box sized from the
  primary), `aria-live` "N of M" + focus mgmt, teardown on slide change/close. 1→single hero
  zero-chrome, 0→render nothing (caller shows gradient). ≤3KB gz.
- `useReducedMotion()` + `useSaveData()` tiny hooks (matchMedia / navigator.connection, SSR-safe).
Provide a one-paragraph "How MenuPage integrates this" note in the file header (the parent does
the lazy fetch on modal open + passes media[]); do NOT edit MenuPage yourself.

## Cinematic reveal (owner: Renderer agent — NEW file `RevealOverlay` in components/media/)
Canvas-2D dissolve (≤80 particles, single pass), decorative only. `prefers-reduced-motion` →
instant (no particle pass). RAF teardown on close + repeat taps (cancel prior RAF, clear canvas)
→ no leak over 50 open/close. If 2D context unavailable → skip to content. MUST never block
Add-to-Cart. Standalone + a documented integration note; MenuPage wiring is done by the lead.

## Admin media manager (owner: Admin agent — owns MenuManagerPage.tsx + new component)
In the product edit UI: list product_media, upload (presign→PUT→confirm flow), reorder (up/down
buttons — NO dnd-kit), set-primary, available toggle. Gated on `MEDIA_RICH_ENABLED` + business
tier (read a hint; safe to no-op when off). New component `MediaManager.tsx`; wire it into the
existing product editor.

## Seed (owner: Data agent — NEW migration after 055)
Add real `product_media` rows on the demo location (sushi-durres) products using public CC image
URLs (like migration 046) for a gallery (3 images), and one `spin` (a few frame URLs) + set the
products' `primary_media_id`. Set that location's `plan='business'`. Use external `https://` URLs
in `storage_key` so the resolver passes them through (no R2 needed for E2E proof). Forward-only,
idempotent.

## Hiding rules V1–V6 (owner: lead, in MenuPage) — listed for awareness, not for agents to build.
```
