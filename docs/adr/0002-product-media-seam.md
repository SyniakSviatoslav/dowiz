# ADR 0002: `product_media` Seam — Schema Now, Rich Runtime Later

**Status:** ACCEPTED (2026-06-22, human GO at STOP-DESIGN-A + STOP-DESIGN-B). Phase 1
(inert schema + kill-switch) shipped as migration `1790000000054_product-media-seam.ts`
+ `MEDIA_RICH_ENABLED` (default off). The design docs say `…048`; the index rebased to
`…054` before implementation. Phases 2–5 remain behind GO/NO-GO gates.
**Supersedes:** nothing · **Extends:** the public-menu schema + RLS conventions
**Companion design:** `docs/design/cinematic-product-media/proposal.md`

## Context

The storefront ProductModal supports a single image (`products.image_key`, server-processed
webp, served by the `/images/*` proxy under a 1-year immutable cache). The product roadmap wants
**rich product media** — image today, then **video / 360-spin / 3D-model** — with a cinematic
open/close reveal, gated to Business tier, **without**:

- touching the hot public-menu path (`read_public_menu`, the SSR `/s/:slug` bundle — a cached
  DoS surface),
- changing the order/price/availability **server contract** (server stays authoritative),
- adding any **heavy dependency** (no three.js/WebGL/hls.js/Lottie/carousel/video lib),
- repeating the failure class that just caused a multi-hour outage (an unbounded/unguarded
  runtime on a boot/hot path).

The cheapest moment to add the schema is **now**, before the Phase-2 money/menu tables migrate.
The repo already provides every pattern we need: a dual RLS policy (`tenant_isolation` write +
`public_select` read, with `ENABLE`/`FORCE`), content-hash + immutable-cache storage on R2, and
code-split lazy client bundles hydrated from `__INITIAL_STATE__`.

## Decision

1. **Separate `product_media` table**, created **with RLS from creation** (`ENABLE` + `FORCE`),
   policies mirroring `products`: `tenant_isolation` (owner write, `app_member_location_ids()`,
   **plus `WITH CHECK`**) + `public_select` (`USING(true)`). `location_id` is **denormalised**
   onto the table so RLS needs **no join**. Columns: `kind` enum(image/video/spin/model),
   `storage_key`, `mime_type`, `bytes`, `width`, `height`, `duration_ms`, `poster_key`, `alt`,
   `sort_order`, `available`, `meta jsonb`. `product_id ... ON DELETE CASCADE`.

2. **`products.primary_media_id uuid REFERENCES product_media(id) ON DELETE SET NULL`** —
   nullable; `image_key` **remains the Tier-0 fallback forever**. Deleting media never breaks a
   product.

3. **Schema complete, runtime minimal.** Ship the **full** schema now; build **no** rich
   runtime. The client renders **`kind=image` only** via a `MediaRenderer` registry;
   video/spin/model resolve to `poster → primary image → on-brand gradient` stubs. The hot path
   reads **only** `primary_media_id` (a column on `products`, **zero join** to `product_media`);
   secondary media are fetched **lazily on modal open** by a separate endpoint.

4. **menu_version semantics — primary bumps, secondary does not.** `product_media` is
   **deliberately NOT wired to the `bump_menu_version` trigger**, so inserting / reordering /
   toggling secondary media causes **no** version bump (correct by construction). A
   `primary_media_id` change is a `products` UPDATE, which the existing trigger already covers →
   **bump**. SSR + JSON-LD include the primary image only and **exclude hidden/unavailable
   products**, invalidated via `menu_version` (consistent with the Cloudflare HTML cache).

5. **No server-side transcoding / sprite-packing / 3D pipeline.** The client validates
   dimensions + magic-bytes locally and uploads pre-sized assets **directly to R2 via short-TTL
   (≤5 min) presigned PUT** URLs (server issues URLs, never proxies bytes). The server
   re-validates magic-bytes on confirm, enforces a **per-location bytes budget** (default 150 MB)
   and a frame-count range (12–72) before issuing URLs, rate-limited. (Divergence from the
   server-processed image path is justified: 12–72 frames through one request would blow the
   request budget — the exact unbounded-runtime risk we are avoiding.)

6. **Zero heavy deps, one heavy decode at a time.** Spin = Canvas-2D frame sequence; video =
   native `<video>`; gallery = registry orchestrator; reveal = Canvas-2D particle pass (≤~80,
   single pass). All four are **code-split lazy chunks** loaded only when a product has that
   `kind` **and** the Business flag is on → **≈ 0 KB delta** on the base public bundle. The
   Gallery enforces a single active decode; neighbors are poster-prefetch only.

7. **Business-tier + feature-flag gate.** Introduce `locations.plan` + a runtime kill-switch
   (`MEDIA_RICH_ENABLED`, default **off**). The gate is enforced **server-side** (lazy endpoint
   returns `[]` when off / non-Business) **and** client-side (chunk not imported) — defence in
   depth. When off, the storefront is **byte-identical to today** and the schema is inert.

## Consequences

- **+** Irreversible cost (DDL, RLS, menu_version semantics) paid once in the cheapest window;
  reversible cost (runtime) deferred behind flags and proven phase-by-phase with GO/NO-GO gates.
- **+** Mirrors only **proven** repo patterns → no novel operational surface one outage after a
  novelty bit us.
- **+** Each phase independently rollback-able by flag flip; the schema never needs a
  down-migration (inert when off).
- **−** One new table + one nullable FK + a `/media/*` serve route + a **new** presigned-PUT path
  + a **new** tier/flag gate (the repo has none today — scoped into Phase 1 as the minimal
  `locations.plan` + env kill-switch, not a billing system).
- **−** `location_id` denormalised on `product_media` (deliberate — RLS without a join).

## Alternatives rejected

- **Columns-on-`products` (`media jsonb`).** Rejected primarily on the menu_version lever: any
  `products` jsonb write already trips the generic bump trigger, so **every** secondary-media
  edit would bump the version and bust the Cloudflare HTML cache. Also loses per-media RLS rows,
  FK integrity, and first-class `sort_order`/`available`/`bytes`.
- **Single-Phase-MVP (spin-only, no seam).** Rejected: re-migration when video/model arrive,
  couples the reveal to spin, and ships an unproven runtime to the storefront in one big launch
  right after an outage — violates failure-first + schema-window economics.

## Verification gates (must be TRUE before code, per phase)

- **P1 (schema):** ADR accepted · migration runs on staging first · `read_public_menu` returns
  byte-identical JSON for NULL `primary_media_id` · public-menu E2E green · cross-tenant insert
  rejected (RLS proof).
- **P2 (upload+spin):** presign refuses over-budget + bad-magic-byte · set-primary bumps version,
  reorder does **not** (X-Menu-Version assertion) · spin chunk absent for spin-less location
  (network proof) · reduced-motion → poster.
- **P3 (video):** exactly one decode active · save-data → poster-only (no video request) · pause
  control + reduced-motion · offscreen pause+release (no leaked decoder).
- **P4 (gallery):** never 2 heavy decodes · teardown on slide change/close · CLS = 0 · aria-live
  "N of M" + focus mgmt.
- **P5 (reveal):** no RAF/canvas leak over 50 open/close · reduced-motion → instant · V1–V6 each
  assert (esp. V5 deep-link graceful, V6 SSR/JSON-LD exclude hidden + version-invalidated) ·
  Add-to-Cart never blocked.
