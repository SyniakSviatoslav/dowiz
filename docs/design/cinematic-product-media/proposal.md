# Design Proposal — Cinematic Product Media (phased rich-media ProductModal program)

**Status:** DRAFT — awaiting STOP-DESIGN-A human gate
**Author:** System Architect (Triadic Council, design-time)
**Companion ADR:** `docs/adr/0002-product-media-seam.md`
**Red-line owner:** product/storefront; **DB owner:** packages/db; **API owner:** apps/api

---

## 1. Problem frame

### What
Add rich product media (image today; **video / 360-spin / 3D-model** later) to the public
storefront ProductModal, behind a Business-tier feature flag, with a cinematic open/close
reveal — **without touching the hot public menu path, the order/price contract, or adding a
single heavy dependency**.

### Why now
- The schema window is cheapest **now**, before the Phase-2 money/menu tables migrate. Adding
  a `product_media` table and one nullable FK on `products` while the menu schema is still
  small is a near-zero-risk forward-only migration. Doing it after Phase-2 means coordinating
  with live money tables.
- The storefront was just polished and is healthy; the app just survived a multi-hour outage
  caused by an unguarded boot path (pg-boss `createQueue` permission hang). We add **schema
  seams now, runtime later** so we never repeat "ship a heavy runtime to a hot path before
  it is proven."
- `products.image_key` is single-image and server-processed (sharp→webp, content-hash key,
  served by the `/images/*` proxy with `max-age=31536000, immutable`). That pattern is the
  proven base we extend; it stays as the **Tier-0 fallback** forever.

### Non-goals (explicit)
- **No server-side transcoding / sprite-packing / 3D pipeline.** Client uploads final assets;
  server only validates magic-bytes + stores. (Diverges from the existing image path, which
  *does* sharp-process — justified in §4: spin frames are pre-sized by the client, and
  transcoding 12–72 frames server-side blows the request budget.)
- **No server contract / price / availability-authority change.** The server stays
  authoritative for price and order validity. Client-side hiding of unavailable products is a
  *presentation* concern only; the order endpoint already re-validates availability and price.
- **No three.js / WebGL / hls.js / Lottie / carousel / video library.** Spin = Canvas-2D frame
  sequence; video = native `<video>`; reveal = Canvas-2D particle pass.
- **No change to `read_public_menu` hot path** beyond reading `primary_media_id` (already a
  column on `products`; zero join to `product_media`).
- **No new always-on runtime in Phase 1.** Phase 1 ships schema + a stubbed MediaRenderer
  registry that renders `kind=image` only; video/spin/model resolve to poster→primary→gradient.

---

## 2. Back-of-envelope numbers

**Topology (confirmed from ADR-0001 + repo):** single Supabase Postgres (Free-tier floor:
~60 pooler conns; budget = 3 session + 8 operational + 3 pg-boss = 14, leaving room for
migrations). Cloudflare in front of fly.io. Storage = Cloudflare R2 (`R2StorageProvider`)
served via the `/images/*` proxy (extended to `/media/*`), 1-year immutable cache.

### Per-asset sizes (target ceilings, enforced client-side + server magic-byte/size cap)
| Asset | Composition | Target bytes |
|---|---|---|
| Image (Tier-0, existing) | 1× webp, ≤800×800 q82 | ~60–120 KB |
| **Spin** | 12–72 equal-size webp frames, ≤512×512 q75, ~20–40 KB each | **0.24–2.9 MB** (cap budget at 36 frames ≈ 1.1 MB typical) |
| **Video** | 5s MP4 H.264, 720p, ~2–4 Mbps | **1.25–2.5 MB** + poster ~40 KB |
| Model (future) | glb, draco-compressed | deferred — **schema only**, no runtime in this program |

### Per-location budget math
Assume a mature location: **80 products**, of which **~15% (12 products)** adopt rich media on
Business tier (rich media is opt-in, premium, not every dish).

- Tier-0 images, all 80 products: 80 × 100 KB ≈ **8 MB**.
- Rich adopters, say 8 spins (~1.1 MB) + 4 videos (~2.5 MB + poster): 8×1.1 + 4×2.54 ≈
  **8.8 + 10.2 = ~19 MB**.
- **Per-location stored ≈ 27 MB.** Cap: **per-location bytes budget = 150 MB** (head-room
  ×5; enforced at upload — see §7 budget-exceeded path).
- Supabase Storage is **not** used for media (we use R2). Supabase Free-tier 1 GB Storage is
  irrelevant; the DB only stores rows (`product_media` ≈ 200 B/row × maybe 50 media rows/loc
  ≈ 10 KB/loc — negligible).

### Egress (the real cost)
- Media is fetched **lazily on modal open only** — never on grid render. Grid shows the
  primary image (already counted in the 8 MB).
- Modal opens per location/day: say 300 menu sessions × 3 modal-opens × 1 media set each.
  If 15% land on a rich product: ~135 rich opens/day. Spin (1.1 MB) or video (2.5 MB):
  ~135 × ~1.5 MB avg = **~200 MB/day/location egress**.
- **Cloudflare cache is the controller:** media URLs are content-hashed + `immutable`, so R2
  egress is paid **once per asset per cache-fill**, not per view. R2→CF egress is free
  (Cloudflare R2 has zero egress fees to the internet). The cost surface is R2 *storage* +
  CF requests, both bounded by the per-location bytes cap and the immutable cache.
- **save-data / slow-network → poster-only** (§7) collapses the heavy-fetch tail.

### Bundle-size delta (hard target ≈ 0 on the base public bundle)
- Phase 1 adds **only** a MediaRenderer registry (a `switch` + ~40 lines) + the
  `primary_media_id` plumbing → **≈ 0 KB** net on `/dist/menu/app.js` (the SSR-hydrated public
  bundle). Image rendering already exists.
- SpinViewer, VideoClip, Gallery, Reveal are **code-split lazy chunks**, loaded **only when**
  a product has that `kind` AND the flag/tier is on. A customer on a location with zero rich
  media downloads **zero** of these bytes. Target per-chunk: SpinViewer ≤ 4 KB gz, VideoClip
  ≤ 2 KB gz, Gallery ≤ 3 KB gz, Reveal ≤ 3 KB gz.

### Decode / CPU / battery (one heavy decode at a time — hard invariant)
- **One** `<video>` decoding OR **one** SpinViewer rasterising at any instant (Gallery
  enforces; neighbors are poster-prefetch only). A single 720p H.264 decode is well within a
  mid-range Android budget; a single Canvas-2D frame blit of a ≤512² webp is trivial.
- Offscreen/teardown: video paused + `src` released, spin RAF cancelled + canvas cleared on
  slide change / modal close → **no leak on repeat taps** (the outage lesson: bound every
  lifecycle).

**Headline:** ~27 MB stored / ~200 MB-day egress per mature location, **fully behind the
Cloudflare immutable cache (R2 egress free)**, **~0 KB base-bundle delta**, **one heavy decode
at a time**, **zero join on the hot path**.

---

## 3. Design options (≥2, named concepts + tradeoffs)

### Option A — **Seam-Now-Runtime-Later** (separate `product_media` table) — PROPOSED
Ship the **full** `product_media` schema + `products.primary_media_id` FK now; build **no**
rich runtime (image-only renderer + stubs). Spin/video/gallery/reveal land in later gated
phases as lazy chunks.

- **Concept:** *"Schema rich, runtime minimal"* + monolith-first. The expensive, coordination-
  heavy part (DDL, RLS, menu_version semantics) is paid once, in the cheap window. Runtime is
  deferred behind flags and proven phase-by-phase.
- **+** Cheapest migration window; zero hot-path runtime risk now; each phase independently
  rollback-able by flag; clean separation of media rows from the product row (multi-media,
  sort_order, per-media availability) without bloating `products`.
- **+** Mirrors the proven RLS dual-policy pattern (`tenant_isolation` write + `public_select`
  read) and the content-hash/immutable-cache storage pattern already in the repo.
- **−** One extra table + one nullable FK now for runtime that doesn't exist yet (acceptable:
  a table with no runtime costs ~nothing; the FK is `ON DELETE SET NULL`, non-breaking).
- **−** Requires denormalising `location_id` onto `product_media` (RLS without a join).

### Option B — **Columns-on-`products`** (no new table)
Add `media jsonb` (array of media descriptors) + `primary_media_id` semantics inside the JSON.

- **Concept:** Single-table, schemaless media list (mirrors how `attributes` jsonb already
  carries bom/taste).
- **+** No new table; one migration column.
- **−** No per-media RLS row, no FK integrity, no `sort_order`/`available` as first-class
  queryable columns; reorder/set-primary become jsonb surgery; per-media bytes accounting and
  budget enforcement become app-only (no DB constraint). **Breaks** the "secondary media
  change → no menu_version bump" requirement: any `products.attributes` write already trips the
  generic `bump_menu_version` trigger, so *every* secondary media edit would bump the version
  and bust the Cloudflare HTML cache. **Rejected on the menu_version lever alone.**
- **−** Mixing heavy media descriptors into the row read on the (warm) owner product list.

### Option C — **Single-Phase-MVP (spin-only, no seam)**
Skip the table; build SpinViewer directly against a `spin_frames jsonb` column, ship it all at
once.

- **Concept:** YAGNI-maximal — only build the one media kind we want first.
- **+** Fastest to a visible spin.
- **−** Re-migration when video/model arrive; no `kind`-agnostic gallery; couples the cinematic
  reveal to spin; ships an unproven runtime to the storefront in one big launch right after an
  outage. **Rejected: violates failure-first + schema-window economics.**

### DECISION → **Option A (Seam-Now-Runtime-Later)**, because
1. It pays the irreversible cost (DDL + RLS + menu_version semantics) in the cheapest window
   and the reversible cost (runtime) incrementally behind flags.
2. It is the **only** option that lets "secondary media change → no menu_version bump" be
   enforced cleanly (the bump trigger lives on a *column-scoped* `products` update of
   `primary_media_id`, and `product_media` is **not** wired to the bump trigger at all — see
   §5).
3. It mirrors patterns already proven in this repo (dual RLS policy, content-hash immutable
   storage, lazy-import code-split bundles), so it adds no novel operational surface — exactly
   the boring-and-proven bias we want one outage after a novelty bit us.

---

## 4. Decision record (ADR summary — full text in `docs/adr/0002-product-media-seam.md`)

- **Decision:** Separate `product_media` table (RLS-from-creation, FORCE) + nullable
  `products.primary_media_id` FK; full schema now, image-only runtime now, gated lazy phases
  later. `image_key` remains Tier-0 fallback. No server transcoding. One heavy decode at a time.
- **Drivers:** schema-window economics, hot-path inviolability, menu_version precision, zero
  heavy deps, failure-first after a fresh outage.
- **Consequences:** one new table, one FK, a `/media/*` serve route (extends `/images/*`), a
  presigned-PUT path (new pattern, justified below), a feature-flag + tier gate (new — §8/§9).

**Why presigned PUT for media when images are server-processed:** the existing image path
does `request.file()` → sharp → one webp. A spin is **12–72 frames**; pushing 12–72 multipart
files through one Fastify request and sharp-processing each blows the request time/memory
budget and risks a repeat of "one slow boot/hang took prod down." Instead the client validates
dimensions + magic-bytes locally and PUTs each pre-sized frame **directly to R2 via a
short-TTL presigned URL** (server issues the URLs, never proxies the bytes). Server then
records `product_media` rows referencing the uploaded keys. This keeps the API request small
and bounded.

---

## 5. Data / migrations (forward-only, RLS-from-creation, integer)

### DDL sketch (one forward-only node-pg-migrate file)
```sql
-- enum first (mirrors extensions-and-enums convention)
CREATE TYPE product_media_kind AS ENUM ('image', 'video', 'spin', 'model');

CREATE TABLE product_media (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  location_id   uuid NOT NULL REFERENCES locations(id),   -- DENORM for RLS w/o join
  product_id    uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
  kind          product_media_kind NOT NULL,
  storage_key   text NOT NULL,            -- R2 key (image) OR prefix/manifest (spin)
  mime_type     text NOT NULL,
  bytes         bigint NOT NULL DEFAULT 0 CHECK (bytes >= 0),
  width         int,
  height        int,
  duration_ms   int,                      -- video only
  poster_key    text,                     -- video/spin/model poster (sanitized, never SVG)
  alt           text,
  sort_order    int NOT NULL DEFAULT 0,
  available     boolean NOT NULL DEFAULT true,
  meta          jsonb NOT NULL DEFAULT '{}'::jsonb,  -- e.g. {frameCount, frameKeys[]} for spin
  created_at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX product_media_product_idx ON product_media (product_id, sort_order);

-- RLS FROM CREATION (mirror products exactly): owner-write + public-read
ALTER TABLE product_media ENABLE ROW LEVEL SECURITY;
ALTER TABLE product_media FORCE  ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON product_media
  USING ( location_id IN (SELECT app_member_location_ids()) )
  WITH CHECK ( location_id IN (SELECT app_member_location_ids()) );
CREATE POLICY public_select ON product_media FOR SELECT USING (true);

-- the FK on products; ON DELETE SET NULL so deleting media never orphans/breaks a product
ALTER TABLE products
  ADD COLUMN primary_media_id uuid REFERENCES product_media(id) ON DELETE SET NULL;
```

Notes:
- `WITH CHECK` is added on the write policy (the existing `products` policy omits it — we
  tighten here so an owner can't insert a row with another tenant's `location_id`).
- `product_id ON DELETE CASCADE` (media dies with the product); `primary_media_id ON DELETE
  SET NULL` (deleting the primary media falls back to `image_key`, never breaks the product).
- A `CHECK` could enforce `bytes` ceilings but budget is per-*location aggregate*, so it is
  enforced in the app at upload (§7), not a row CHECK.

### menu_version semantics — **primary → bump, secondary → no bump** (the critical lever)
The repo's generic `bump_menu_version_trigger_fn` fires on **any** `products` INSERT/UPDATE/
DELETE. We must NOT let it bump on every secondary-media edit. Enforcement:

1. **`product_media` is deliberately NOT wired to the bump trigger.** Inserting/updating/
   reordering/toggling a secondary media row touches only `product_media` → **no bump**. Correct
   by construction.
2. **`primary_media_id` change DOES bump**, because it is a column on `products`, and any
   `products` UPDATE already fires the trigger. To avoid *over*-bumping (e.g. a price edit also
   bumps, which is already today's behaviour and fine), we keep the existing per-row trigger but
   **add a column-scoped trigger is unnecessary** — the existing products trigger already covers
   `primary_media_id` writes. The set-primary endpoint issues a dedicated
   `UPDATE products SET primary_media_id = $1 WHERE ...` → trigger → bump. ✔
3. **Hot path reads `primary_media_id` only** (already on `products`; `read_public_menu` adds it
   to the product JSON object — a column read, **zero join** to `product_media`). Secondary
   media are fetched by a separate lazy endpoint on modal open.

This makes SSR↔JSON-LD↔client consistency follow the existing `menu_version` → Cloudflare HTML
cache invalidation with no new mechanism: a primary swap bumps the version, busts the cached
HTML/JSON-LD; a secondary gallery edit does not (gallery is fetched client-side, post-hydration,
not in SSR/JSON-LD — JSON-LD intentionally excludes secondary media).

### Backfill plan (lazy / deferred / non-breaking)
- **No backfill.** Existing products keep `image_key`; `primary_media_id` starts `NULL`. The
  renderer's fallback chain (§7) resolves `primary_media_id → image_key → gradient`, so a NULL
  FK is the steady state for every legacy product. Migration is instantaneous (one column add +
  one table create); zero rows rewritten.
- Optional later, opt-in: an owner who wants their existing `image_key` represented as a
  first-class `product_media(kind=image)` row can do so via the upload UI — never auto-migrated.

---

## 6. Consistency + idempotency

### Presigned upload (spin frames / video / image)
- Server issues N short-TTL (≤5 min) presigned **PUT** URLs for **content-addressed keys**
  (`{locationId}/{productId}/{kind}/{sha256-12}.{ext}`). Content-hash key ⇒ a re-PUT of the
  same bytes is **idempotent** (same key, immutable cache). A retried frame upload overwrites
  itself harmlessly.
- The DB row is written **after** all frames confirm 200 from R2, in **one** transaction
  (insert the `product_media` row with `meta.frameKeys[]`). If the client dies mid-upload,
  orphan R2 objects exist but **no** half-written media row — the product is unaffected (no FK
  points at a nonexistent row). Orphans are swept by a periodic R2-vs-DB reconcile (operability,
  best-effort, same spirit as the existing old-image cleanup).

### Set-primary (idempotent)
- `UPDATE products SET primary_media_id = $1 WHERE id = $2 AND location_id = $3 AND
  EXISTS(SELECT 1 FROM product_media WHERE id = $1 AND product_id = $2)`. Re-issuing with the
  same id is a no-op (same value) but still fires the bump trigger — acceptable (version is
  monotonic; a redundant bump only over-invalidates cache once). To avoid redundant bumps, the
  endpoint reads current value and skips the UPDATE if unchanged.

### Reorder (idempotent, atomic)
- Reorder is a single transaction: `UPDATE product_media SET sort_order = v.ord FROM
  (VALUES ...) v(id, ord) WHERE product_media.id = v.id AND product_media.product_id = $p`.
  Full-set replacement keyed by id ⇒ replaying the same payload is idempotent. Touches only
  `product_media` ⇒ **no menu_version bump** (secondary ordering is not in SSR/JSON-LD).

### SSR ↔ JSON-LD ↔ client consistency
- All three derive from the same `menu_version`. SSR HTML + JSON-LD include **primary image
  only** (and **exclude hidden/unavailable products** — §V6). Client hydrates from
  `__INITIAL_STATE__`, then lazily fetches the secondary media set on modal open. Because
  secondary media never enters SSR/JSON-LD and never bumps the version, the cached HTML and the
  live gallery can differ in *secondary* media without inconsistency that matters to crawlers or
  the cache. Primary swaps bump the version → CF HTML cache + the menu JSON `max-age=60` both
  refresh.

---

## 7. Failures + degradation (failure-first; every external call has timeout + fallback; zero cascade)

### Renderer fallback chains (each renderer, hard-coded)
- **Image:** `media → image_key → on-brand gradient` (the gradient block already exists in
  MenuPage modal).
- **SpinViewer:** `poster (instant) → first frame loads → interactive`. Any frame fetch fail /
  timeout (per-frame ≤4 s) → **stay on poster** (ordering never blocked). `prefers-reduced-
  motion` → poster only, no RAF. Missing poster → primary image → gradient.
- **VideoClip:** `poster → (autoplay muted) → loop`. `save-data` or `effectiveType ∈
  {slow-2g,2g,3g}` → **poster only, no video fetch**. Decode error / `onerror` → poster →
  primary → gradient. WCAG 2.2.2 pause control always present when playing.
- **Gallery:** media set fetch (lazy, on modal open) fail/timeout (≤4 s) → fall back to the
  **single primary hero** (exactly today's modal). 0 media → gradient. 1 media → single hero,
  zero chrome. Never blocks the Add-to-Cart button (it lives below the hero, always rendered).
- **Reveal:** Canvas-2D dissolve is **decorative only**. `prefers-reduced-motion` → instant
  show/hide (current behaviour). RAF teardown on close + on repeat taps (cancel prior RAF,
  clear canvas) → no leak. If canvas context is unavailable → skip straight to content.

### Cross-cutting degradation switches
- **Flag off / not Business tier:** server omits secondary media from the lazy endpoint and
  the client never loads the spin/video/gallery/reveal chunks → storefront is byte-identical to
  today. The gate is checked **server-side** (lazy media endpoint returns `[]`) AND client-side
  (chunk not imported) — defence in depth, no client-only trust.
- **Tier downgrade (Business→free) mid-life:** existing media rows stay in the DB (not deleted)
  but the lazy endpoint returns `[]` for non-Business → media simply stops showing; primary
  image (Tier-0) still shows. Re-upgrading restores it. No data loss, no migration.
- **Storage budget exceeded mid-upload:** the presign endpoint checks
  `SUM(bytes) + incoming ≤ per-location cap` **before** issuing URLs. Over cap → `413`-style
  refusal with a clear owner message; **partial uploads already in R2** (if a multi-frame set
  crossed the cap mid-way — prevented by summing the *whole* incoming set up front) are
  swept by reconcile. No DB row is written, so the product is untouched.
- **Network slow on the menu itself:** unchanged — media is never on the hot path; the grid +
  primary image render exactly as today.
- **Zero cascade:** no renderer failure can throw past its boundary (each wrapped in an error
  boundary / try-catch → fallback). A media outage degrades to today's storefront, never to a
  broken page or a blocked order.

---

## 8. Security + tenant isolation

- **RLS leak surface on public read:** `product_media.public_select USING(true)` mirrors
  `products`/`categories` — public read is intentional (it's a public menu). The leak risk is
  *cross-tenant write*, blocked by `tenant_isolation` + the added `WITH CHECK` (an owner cannot
  insert/alter a row with a `location_id` they don't own). `FORCE` ensures even the table owner
  role is subject to RLS (mirrors `force-rls` migration). The lazy media endpoint additionally
  filters by `product_id` belonging to the requested slug's location — defence in depth.
- **Signed-URL TTL for private model/video:** public images/videos use the public `/media/*`
  immutable cache (no signing — they're public menu content). **Private** kinds (future `model`,
  or any media flagged `meta.private`) are served via short-TTL (≤5 min) **GET** presigned URLs,
  never the public proxy. Upload PUT URLs are always ≤5 min TTL.
- **Magic-bytes policy (server-side, non-negotiable):** the presign endpoint validates the
  client-declared `mime_type` against an allow-list (`image/webp`, `image/jpeg`, `video/mp4`
  H.264, future `model/gltf-binary`) AND, on the post-upload confirm, the server fetches the
  first bytes of each uploaded object and verifies the magic number matches (webp `RIFF....WEBP`,
  mp4 `ftyp`, glb `glTF`). Mismatch → reject + delete the object. This catches a "rename a .exe
  to .mp4" upload that a client-only check would miss.
- **SVG-poster sanitization:** **posters may never be SVG.** `poster_key` is constrained to
  raster (webp/jpeg) by the same magic-byte check. SVG posters are an XSS vector (inline script)
  and are rejected outright — no sanitiser, just disallow.
- **Presigned policy:** PUT URLs are scoped to the exact content-addressed key (the owner cannot
  PUT to an arbitrary key), short TTL, single-use intent. Keys are namespaced by
  `{locationId}/{productId}/` so a presign for tenant A can never write tenant B's prefix.
- **Abuse / cost (malicious owner uploading huge media):** (1) per-location bytes cap enforced
  pre-presign; (2) per-frame and per-file size ceiling in the presigned policy
  (`content-length-range`); (3) frame-count cap (12–72) rejected outside range; (4) rate-limit
  on the presign endpoint (reuse the existing `@fastify/rate-limit` config seen on
  `/api/public/entry-photo`). A malicious owner can at worst fill their own capped budget on
  their own R2 prefix — bounded blast radius, no cross-tenant impact, no egress amplification
  (immutable cache).

---

## 9. Operability

- **Per-location egress/bytes budget monitoring:** `product_media.bytes` is summed per
  `location_id`; a scheduled check (reuse the `free-tier-watch` worker pattern already in the
  repo) emits an owner/ops alert at 80% of the 150 MB cap. Egress is observed via Cloudflare
  analytics (per-zone), not per-request app logging (keeps the hot path clean).
- **Cloudflare cache keys:** media served at `/media/{contentHash}.{ext}` ⇒ `immutable` ⇒ the
  content hash *is* the cache key; a changed asset is a new URL (no purge needed — same as the
  existing image path). The SSR HTML/menu-JSON cache keys are unchanged and continue to be
  invalidated by `menu_version` on a primary swap.
- **Feature-flag + Business-tier gate (new — current gap):** the repo has **no** tier/feature-
  flag table today (flags are env-only). This program introduces a minimal gate:
  `locations.plan text` (or a `location_features` row) read by (a) the lazy media endpoint
  (server-side authority) and (b) a `/public/.../info`-style hint the client uses to decide
  whether to import the rich chunks. **Phase 1 ships this column behind an env kill-switch**
  (`MEDIA_RICH_ENABLED=false`) so even a Business location shows nothing until we flip it — the
  schema-rich/runtime-minimal doctrine applied to the gate itself.
- **Rollback per phase:** every phase is independently revertible — flip `MEDIA_RICH_ENABLED`
  off (instant, no deploy if it's a runtime-read env/flag) → storefront returns to today. The
  schema (Phase 1) is forward-only and **inert** when the flag is off, so it never needs a
  down-migration; the rollback of a *runtime* phase is "stop importing the chunk + endpoint
  returns []." Observability for "is rich media live and healthy" surfaces in `/health` as a
  degraded-vs-down line (flag state + last reconcile time), <1 min to read.
- **Scaling gate:** the lazy media endpoint and presign endpoint run on the API tier already
  sized for the menu; they add no new always-on worker. Presign is bounded by rate-limit;
  reconcile is a low-frequency scheduled job on the existing pg-boss budget (no new connection).

---

## 10. Open + accepted risks

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R1 | **No tier table exists** — gate must be invented. | **Accept + scope into Phase 1** as `locations.plan` + env kill-switch; do not block on a full billing system. | apps/api |
| R2 | Presigned PUT is a **new pattern** (images are server-processed today). | **Accept**: justified by frame-count budget (§4); mitigated by magic-byte re-validation on confirm + content-length-range policy. | apps/api |
| R3 | Orphan R2 objects from aborted multi-frame uploads. | **Accept (deferred sweep)**: best-effort reconcile job; orphans cost only capped storage, never break a product. | apps/api |
| R4 | A redundant `primary_media_id` set re-bumps menu_version. | **Mitigate**: endpoint reads-before-writes, skips no-op; residual over-bump is harmless (monotonic). | packages/db |
| R5 | Client save-data / network detection (`navigator.connection`) is **non-uniform across browsers**. | **Accept**: where unavailable, default to **poster-only** (conservative); never default to heavy fetch. | apps/web |
| R6 | Cinematic reveal RAF/canvas **leak on rapid open/close** (the kind of unbounded lifecycle that caused the outage). | **Mitigate (gate)**: Phase-5 GO requires a proven teardown test (RAF cancelled, canvas released, no growing listener/handle count over 50 open/close cycles). | apps/web |
| R7 | Cross-tenant write via forged `location_id`. | **Closed**: `WITH CHECK` on `tenant_isolation` + `FORCE` RLS. | packages/db |

---

## 11. Sequenced phase plan with explicit GO / NO-GO gates

Each phase is dark-deployable; **launching** is a separate explicit act (flag flip).

### Phase 1 — Schema Seam `product_media` v1.0 (+ image-only renderer registry + gate column)
**Build:** the migration (DDL above, RLS-FROM-CREATION FORCE), `primary_media_id` on
`products`, `read_public_menu` reads `primary_media_id` (column, no join), `MediaRenderer`
registry rendering `kind=image` only (video/spin/model → poster→primary→gradient stub),
`locations.plan` + `MEDIA_RICH_ENABLED` env kill-switch (default off).
**GO requires (before any code):** ✅ ADR-0002 accepted at STOP-DESIGN-A · ✅ migration runs on
staging DB first (auto-migrate release_command) and `read_public_menu` returns byte-identical
JSON for a product with NULL `primary_media_id` · ✅ public menu Playwright E2E on staging green
(zero regression) · ✅ RLS proven: a cross-tenant insert is rejected.
**NO-GO if:** any hot-path latency regression, or RLS test leaks.

### Phase 2 — Tier-1 Upload UI + SpinViewer
**Build:** presign endpoint (magic-byte + budget + rate-limit), admin media manager (dnd-kit
reorder, set-primary, available toggle), client SpinViewer (Canvas-2D, code-split lazy).
**GO requires:** ✅ Phase 1 live + inert · ✅ presign refuses over-budget and bad-magic-byte
uploads (proven) · ✅ set-primary bumps menu_version, reorder does NOT (proven via X-Menu-Version
assertion) · ✅ SpinViewer chunk not loaded for a location without a spin (network-tab proof) ·
✅ `prefers-reduced-motion` → poster (Playwright).
**NO-GO if:** secondary edits bump the version, or the chunk ships on the base bundle.

### Phase 3 — VideoClip Renderer
**Build:** native `<video>` renderer, code-split, save-data/slow-net → poster, one-decode rule,
WCAG pause.
**GO requires:** ✅ exactly one decode active asserted · ✅ save-data → poster-only (no video
request) proven · ✅ pause control present + reduced-motion respected · ✅ offscreen pause+release
proven (no leaked decoder).
**NO-GO if:** two videos decode concurrently, or video fetched under save-data.

### Phase 4 — Media Gallery
**Build:** kind-agnostic orchestrator over the registry, active only when media>1, single heavy
decode + neighbor poster-prefetch, prev/next + dots + aria-live, zero auto-advance, zero CLS
(slides share the hero box sized from primary).
**GO requires:** ✅ never 2 heavy decodes (proven) · ✅ pause/teardown on slide change/close ·
✅ CLS = 0 (Lighthouse/Playwright layout-shift assertion) · ✅ aria-live "N of M" + focus mgmt ·
✅ 1-media → single hero zero-chrome, 0 → gradient.
**NO-GO if:** any CLS, or a second decode survives a slide change.

### Phase 5 — Cinematic ProductModal Reveal v1.1
**Build:** Canvas-2D dissolve-to-fog (≤~80 particles, single pass), minimized card
(photo/name/price), client-side hiding rules V1–V6 (unavailable not rendered V1; admin shows
all V2; modifier-level unavailability disables option not product V3; empty category drops from
CategoryNav, all-unavailable → empty-state + fallback phone V4; deep-link/reorder of hidden →
"temporarily unavailable" not 404, cart-drift preserved V5; SSR + JSON-LD exclude hidden,
invalidated via menu_version V6).
**GO requires:** ✅ R6 teardown test green (no leak over 50 open/close) · ✅ reduced-motion →
instant (no particle pass) · ✅ V1–V6 each have a passing assertion (esp. V5 deep-link → graceful
state, V6 SSR/JSON-LD exclude hidden + version-invalidated) · ✅ Add-to-Cart never blocked by the
animation.
**NO-GO if:** any RAF/canvas leak, animation blocks ordering, or a hidden product 404s on
deep-link.

---

## Appendix — repo grounding (what this design is built on)

- Hot path `read_public_menu` (SECURITY DEFINER) builds menu JSON, **no media join** — extended
  to read `products.primary_media_id` only.
  (`packages/db/migrations/1780338982022_read_public_menu.ts`)
- menu_version bump = generic per-table trigger on `products` etc.
  (`packages/db/migrations/1780338982021_menu_version_trigger.ts`) — the lever for primary-bump /
  secondary-no-bump.
- RLS dual-policy: `tenant_isolation` (write, `app_member_location_ids()`) + `public_select`
  (read, `USING(true)`) + `ENABLE`/`FORCE`. (`1780310072731_menu.ts`, `1780338741329_public-menu-rls.ts`,
  `1780421100051_force-rls.ts`)
- Storage = R2 via `/images/*` proxy, content-hash key, `max-age=31536000, immutable`, traversal
  guard. (`apps/api/src/routes/spa-proxy.ts`, `apps/api/src/lib/r2-storage.ts`) — extended to
  `/media/*`; presign is new (justified §4).
- SSR + JSON-LD (Menu/MenuItem) from `read_public_menu_all_locales`, LRU-cached, primary image
  only. (`apps/api/src/lib/ssr-renderer.ts`) — V6 hidden-exclusion + version invalidation.
- Public bundle hydration via `__INITIAL_STATE__` + `/dist/menu/app.js`; rich chunks are
  code-split lazy off this. (`apps/web/src/pages/client/MenuPage.tsx`, `ssr-renderer.ts`)
- Connection budget 14/60 from ADR-0001 — no new always-on worker; reconcile on existing pg-boss.
