# S4-MEDIA Port — Council Packet · PROPOSAL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Description input to the live Triadic Council
> (system-architect + system-breaker + counsel + human). No S4 code is ported to Rust until this
> packet is council-APPROVED, every quirk-register row (§9) is dispositioned one by one, and the
> operator signs the 🔴 open questions (`open-questions.md`). Docs only; no product code.

- **Lane:** R3 (complete-rebuild) · **Surface:** S4 media pipeline (REBUILD-MAP §3 Phase B, 4th
  strangler — `S3 catalog → S4 media → S5 orders`)
- **Date:** 2026-07-04 · **Source commit:** `fix/audit-remediation@b28b1764` (working tree)
- **Census SSOT:** `inventory/10-api-realtime-jobs.md` §4 (R2/S3 storage) + §5 (sharp pipeline) — the
  four upload/processing call sites + the R2 client + the read-proxy (S1, already shipped).
- **Governing ADRs / prior councils:** ADR-0002 (product-media seam), ADR-0004 (owner-token
  revocation / P-d per-route membership re-read), the **S3-catalog RESOLVE** (`rebuild-catalog-s3-council/
  resolution.md` — inherits **REV-10** tenancy-GUC contract ADR and the **`with_user`** owner-write
  seam; **REV-2** named `themes` as a sanctioned FIX-IN-PORT raw-pool divergence, which lands here).
- **Load-bearing build constraint (from the shipped S1 R2 client, `rebuild/crates/api/src/storage.rs:104-119`):**
  `aws-sdk-s3` / `object_store` both pull `aws-lc-rs`→`aws-lc-sys`, which needs `cmake`; the current
  build env has none. The S1 read path was ported with `aws-sign-v4` (SigV4 **Authorization-header**
  signing) + `reqwest`(rustls) instead. **Presign (SigV4 query-string) is a different algorithm the
  shipped client does not implement** — this constraint is load-bearing for §5/§the two-image split.
- **Parity oracle:** the 174-spec Playwright net; for this surface the load-bearing specs are
  `e2e/tests/flow-ui-images.spec.ts` (owner upload → 800×800 webp render), `e2e/tests/
  flow-client-product-images.spec.ts` (storefront image render), `e2e/tests/media-render.spec.ts`
  (traversal-guarded read proxy — already green on the S1 Rust surface). Cutover DoD in §10.

---

## 1. Port objective and the load-bearing seam

S1 shipped the **read** side of storage (`/images/*`, `/media/*` proxy + `LocalFsStorage`/`R2Storage`
GET, `storage.rs`). S3 shipped the **first tenant-scoped writes** through the `with_user`
(`app.user_id`→memberships) combinator. **S4 is the first Rust surface that ingests bytes from a
client and mutates object storage** — presign, upload, image-transcode, R2 `put`/`delete`, and the
tenant-scoped **metadata** writes that reference the stored keys.

The single load-bearing seam is the **two-layer boundary between an untrusted byte stream and two
authorities that never trust each other**: (a) **R2 has no per-tenant access control** — the *only*
thing that keeps tenant A from writing into tenant B's object space is that the **object key is
server-derived from the membership-resolved `locId`, never client-supplied**; and (b) the **metadata
row** that points at that key is a tenant-scoped write that must seat `app.user_id` via `with_user`
(inherited S3 seam) or it silently matches 0 rows post-B3-flip. A leaked/forged key on the R2 side
and a missing GUC on the DB side are two independent failure modes; the port must hold both.

**The sharpest seam fact the current code gets *inconsistently* (see §8 and Q5):** of the three
tenant-scoped media writes, exactly **one** is correct. `product-media` confirm writes through
`withTenant(db, userId, …)` (`product-media.ts:242`) — the right `app.user_id` family. But the legacy
**product-image** route writes `UPDATE products SET image_key…` on the **raw pool** (`spa-proxy.ts:252`,
`db.query`, no GUC) and the **theme-logo** route writes `UPDATE location_themes…` on a **raw
`db.connect()`** (`themes.ts:139-143`, no GUC). Both are exactly the never-copy leak class REV-2 named
for `themes`; both match 0 rows post-NOBYPASSRLS-flip. S4 must route **all three** through the S3
`with_user` seam — a **FIX-IN-PORT divergence** with a documented E2E delta, not a verbatim carry.

## 2. Scope — what is S4, what is explicitly NOT (and the boundary)

**In this packet (S4):**
1. **Product rich-media presign→confirm flow** (`product-media.ts` — presign, confirm, set-primary,
   reorder, available-toggle). ADR-0002 seam; the only presigned-direct-to-R2 upload in the system.
2. **Product image upload + in-process transcode** (`spa-proxy.ts:213-261` — sharp resize 800×800
   webp q82, content-hashed key, old-key cleanup). The single most-exercised media path (`flow-ui-images`).
3. **Theme logo upload + transcode** (`themes.ts:119-149` — sharp resize 512×512 webp q80). **Deferred
   from S3 per REV-2**; lands here with the media stack. Carries the S3 raw-pool→`with_user` fix.
4. **Public unauthenticated entry-photo upload** (`spa-proxy.ts:268-293` — sharp rotate+resize
   1024×1024 webp q78, IP-rate-limited). 🔴 — §6, Q4.
5. **The R2 `put`/`delete` verbs** (`r2-storage.ts:48-79`) the above depend on (S1 shipped `get` only).
6. **`product_media` metadata table** writes/reads (the confirm INSERT + set-primary/reorder/toggle),
   through the S3 `with_user` seam (§8).
7. **`getImageUrl`** (`image-url.ts` — public-URL-vs-proxy string logic; trivial, ports with the read side).

**NOT S4 (explicit boundary — each a separate slice/sidecar):**
- **Backup multipart streaming upload** (`workers/backup/upload.ts`, `@aws-sdk/lib-storage` 5MB parts).
  This is 🔴 in the census but belongs to the **backup/DR sidecar ops binary** (REBUILD-MAP §8:
  `tokio::process` pg_dump + aes-gcm + multipart), **never the API request path**. S4 does not touch it.
- **Menu-import OCR/LLM pipeline** (`menu-import.ts`, tesseract/paddle/pdfium + PII-redactor). **Deferred
  to its own slice post-S4 per S3 Q5→(a) (RATIFIED, REV-9).** S4 **builds the `media-worker` image**
  that the OCR sidecar will later attach to (Tesseract/pdfium are Debian-slim-only) — an *enabling
  dependency*, not an S4 deliverable. No OCR code ships in S4.
- **Brand-extractor colour sampling** (`brand-extractor.ts:307-311`, `resize(48×48).raw()` pixel
  extraction) — a read-only imaging op feeding `POST /api/owner/brand/generate`. It shares the imaging
  crate S4 picks (Q3) but its route is a catalog/branding concern; **note the shared-crate dependency,
  port the route with its own surface** (candidate: fold into S4 as a non-🔴 rider — Q9).

**Back-of-envelope (why this is CPU/egress-bound, not connection-bound).** Media writes are
human-driven: an owner uploads a menu's worth of images once during onboarding (tens of products ×
1–3 images = low hundreds of objects/location, a burst), near-zero steady state; entry-photos are ≤1
per delivery order that opts in. At the target scale (tens of active locations, low-hundreds of
orders/day system-wide) that is **single-digit uploads/sec at peak onboarding, sub-1/sec steady**.
The **metadata** writes are single-statement `with_user` txns — negligible on the 20-conn operational
pool (same envelope as S3). The real resources are **(a) CPU + peak memory for image decode/encode**
(an 8 MB image or 25 MB clip decoded in-process — a **decompression-bomb DoS surface**, Q3/threat
S4-T6) and **(b) R2 egress + object count**. **No new DB connection pressure.** Conclusion: the
scaling axis is the image processor (bound it), not the pool — which reframes the two-image-split
question as *"where does the CPU-heavy, native-dependency-laden decoder live"*, not *"can the pool take it."*

---

## 3. The two-image split — commit now, or keep processing in-process and split later?

REBUILD-MAP §2 standing verdict: `libvips FFI · Tesseract sidecar · pdfium → **two images**: scratch
`api` (~15-25 MB, musl-static) + Debian-slim `media-worker``. S4 is the first surface that forces the
question, because it is the first surface with a **native-dependency-laden, CPU-heavy** workload. The
build constraint decides more than aesthetics: **the scratch/musl `api` image cannot easily carry
`cmake`-dependent native crates** (`aws-lc-sys` for `aws-sdk-s3`; libvips for FFI), while a **Debian-slim
`media-worker` can** (apt-get libvips + cmake). So "where does processing live" and "how do we presign"
(§5) and "which imaging crate" (§4) are **one coupled decision**.

| Option | Concept | Where processing/presign lives | Cost | Rollback |
|---|---|---|---|---|
| **A — commit to the split now** | Bulkhead / two-image | `media-worker` (Debian-slim) owns transcode + presign (can use `aws-sdk-s3` presigner + libvips-FFI); `api` (scratch) proxies or the FE calls the worker | +1 deploy unit, +1 Fly app, cross-service call for a synchronous owner request (presign/upload is request-path, not a job) | Flag-flip media routes back to Node; worker is independently redeployable |
| **B — keep processing in-process in `api`, split later** | Monolith-first (ADR-001) | Everything in the scratch `api` image | Transcode in-process is **proven viable** (§4 spike: pure-Rust `image`+`webp`, no longer a cost). The *only* forced item is presign: no `aws-sdk-s3` in scratch → server-proxy (§5c) or hand-rolled SigV4 (§5b) | Simplest topology; single rollback unit; no cross-service latency |
| **C — hybrid strangler (recommend as the *cutover* posture)** | Strangler-by-surface | `api` (Rust) owns transcode (pure-Rust `image`) + the metadata writes + confirm; **presign stays on Node** until the `media-worker` image is stood up with `cmake`+`aws-sdk-s3` | Two writers during cutover (Node presigns, Rust confirms) — but the presign→confirm flow already spans two calls, so the seam is natural | Cleanest incremental rollback: each of {transcode, presign, metadata} flips independently |

**R3 recommendation: schema-rich, runtime-minimal — build the *seam* for the split, do not stand up
the second runtime prematurely.** Concretely: **(C) as the cutover posture.** The Q3 spike (§4) has now
**removed the imaging driver for the split**: pure-Rust `image` + `webp` transcodes in-process in the
scratch `api` image (76 ms / 36.7 MB peak, `cc`-only, no `cmake`, no system shared lib). So the split is
**no longer forced by processing** — it hinges *only* on presign (§5/Q2) and the future OCR slice.
Rationale unchanged and strengthened: back-of-envelope (§2) says the workload is single-digit
uploads/sec — a second always-on Fly app for that is over-provisioning (the Prime Video "we recombined
the monolith" lesson). Keep transcode **in-process in `api`** behind a `trait ImageProcessor` (the
seam), and only extract a `media-worker` **when** (i) presign lands there to avoid hand-rolled crypto
(§5/Q2 option a/d), **or** (ii) the OCR slice (post-S4) needs Tesseract/pdfium — at which point the
split pays for itself across two surfaces, not one. **This is a 🔴 topology decision (Q1)** because it
binds the deploy shape and the presign path. The `media-worker` Dockerfile can be *authored* (schema)
now and left *unbuilt* (runtime) until presign or OCR justifies it.

---

## 4. Image processing stack — DECIDED BY SPIKE: pure-Rust `image` 0.25 + `webp` 0.3

**Sharp parity requirements — the THREE transcode profiles the port must reproduce, exact (+ one
read-only sampler):**

| Profile | Transform | Source |
|---|---|---|
| Product image (800/82) | `resize(800×800, fit:inside).webp(q82)` | `spa-proxy.ts:222-226` |
| Theme logo (512/80) | `resize(512×512, fit:inside).webp(q80)` — comment says "strip EXIF" | `themes.ts:127-130` |
| **Entry-photo (1024/78 + rotate)** | `.rotate()` (EXIF auto-orient) `.resize(1024×1024, fit:inside).webp(q78)` — the **unauthenticated** route | `spa-proxy.ts:279-280` |
| Brand sample (read-only) | `resize(48×48, fit:inside).ensureAlpha().raw()` (pixel extraction, no encode) | `brand-extractor.ts:307-311` |

All three encode profiles target **WebP only** (never AVIF), quality **78–82**, `fit:inside` (fit
within the box, aspect-preserving). Stripping metadata removes **GPS/camera PII** from a
customer-supplied doorway photo (threat S4-T4).

**Crate decision — RESOLVED by the parallel spike lane (Q3 now decided-by-evidence, not open):**
- **CHOSEN: pure-Rust `image` 0.25 + `webp` 0.3** (`webp`→`libwebp-sys`, a **`cc`-only** build — **no
  `cmake` anywhere in the graph**). Proven in the *actual sandbox*: builds/tests/clippy clean; product
  profile 2000×1500→800×800 webp q82 = **76 ms / 36.7 MB peak RSS**, logo profile **64 ms**. `fit:inside`
  math matches sharp **bit-for-bit** (neither clamps upscale — sharp defaults allow it, `image::resize`
  uses the same formula). Metadata **stripped by default** (matches sharp's default). Because `webp` links
  a `cc`-compiled static libwebp (not a system shared lib), it stays **scratch/musl-safe and in-process
  in the `api` image** — so **imaging does NOT force the two-image split** (§3); the split now hinges on
  presign (§5/Q2) and the future OCR slice alone.
- **REJECTED: the `libvips` crate** — *not* `cmake`-blocked (its `build.rs` is plain `rustc-link-lib`),
  but it needs system `libvips`/`glib`/`gobject` shared libs at **LINK** time → `apt-get install
  libvips-dev` (198 pkgs) → permanently breaks `cargo build`/`test`/`clippy` in the dev/CI sandbox unless
  provisioning is kept in lockstep. That is the exact **"works in Docker, broken locally" trap already
  rejected once for `aws-sdk-s3`** (`storage.rs:104-119`). Docker-only viable, not chosen.
- **Spike evidence tree (cite in the build lane):** worktree `agent-ad562cc71d9c3d9b1` under
  `rebuild/spikes/image-stack/` — `attempt-a-libvips/` fails at link with verbatim `-lvips` errors;
  `attempt-b-pure-rust/` runs with the timings above.

**🔴-adjacent parity gap the port MUST carry (REV-EXIF — silent-wrong-output class):** `image::open()`
does **NOT** auto-apply EXIF orientation; **sharp applies it implicitly on every profile.** The Rust
port must **explicitly** read `decoder.orientation()` → `apply_orientation()` on **all three**
handlers (product, logo, entry-photo) — not just the one that spelled `.rotate()` in Node. Miss it and
the failure is *silent wrong output* (sideways/mirrored images that still pass a "some-webp-rendered"
assertion). This is a **named DoD item with a dedicated test**: a rotated-EXIF fixture → an
upright-pixel-oriented output (see §10, quirk `Q-EXIF-ORIENT`).

**Non-negotiable regardless of crate:** a **decompression-bomb bound** — a hard pixel/dimension cap via
`image`'s `Limits` API *before* decode, so an 8 MB file cannot allocate gigabytes (threat S4-T6). The
seam stays a `trait ImageProcessor` so the impl is swappable, but the pick is settled.

---

## 5. Presigned PUT — 300s TTL, key derivation, leaked-presign blast radius, and the crypto 🔴

**Current behavior (`product-media.ts:82-172`):** `POST …/media/presign` — after owner P-d membership
resolve + `productInLocation` + per-item mime/size/sha256-shape validation + per-location 150 MB budget
check, it builds a **content-addressed, tenant-scoped key** `${locId}/${productId}/${subKind}/${sha256
.slice(0,12)}.${ext}` and mints an `@aws-sdk/s3-request-presigner` `getSignedUrl(PutObjectCommand,
{expiresIn: 300})`. The client PUTs bytes **directly to R2**; a later `confirm` re-sniffs the magic
bytes and writes the `product_media` row. **300 s TTL is the parity contract** (`PRESIGN_TTL_SECONDS
= 300`, `:29`).

**What a leaked presigned URL CAN do (blast radius — priced, not hand-waved):**
- PUT arbitrary bytes to **exactly one key** — `${locId}/${productId}/${subKind}/${hash}.${ext}` — for
  **≤300 s**. The signature is scoped to `{bucket, key, method:PUT, ContentType}`; it is **not** a
  bucket-wide credential.
- Overwrite that one content-addressed object (the key encodes the *declared* hash, so an attacker with
  the URL can write mismatched bytes under a hash-named key — but `confirm` re-sniffs the **mime magic
  bytes** before persisting metadata, so a mismatched *type* is caught; a same-type substitution is not).

**What it CANNOT do:** write into **another tenant's prefix** (the key is server-built from the
membership-resolved `locId` — the client never supplies it, and `confirm` rejects any `storageKey` not
starting with `${locId}/${productId}/`, `:209`); write after 300 s; enumerate or read other objects;
touch the DB. The blast radius of a leaked presign is **one object, one product, ≤5 min, same tenant** —
acceptable, *provided the signing is correct*.

**The Rust presign problem (🔴 crypto-adjacent, Q2):** the shipped S1 client signs with `aws-sign-v4`,
which produces a SigV4 **Authorization header** — the algorithm for a request *the server itself
sends*. A **presigned URL** is the SigV4 **query-string** variant (`X-Amz-Algorithm`,
`X-Amz-Credential`, `X-Amz-Date`, `X-Amz-Expires`, `X-Amz-SignedHeaders`, `X-Amz-Signature` in the
query, `UNSIGNED-PAYLOAD`) — a *different canonical request*. `aws-sdk-s3`'s presigner does this, but
it needs `cmake` (absent). Options:

- **(a) Stand up the `media-worker` (Debian-slim) and use `aws-sdk-s3`'s presigner there** — no
  hand-rolled crypto; forces §3-A. *(cleanest crypto posture; heaviest topology)*
- **(b) Hand-roll SigV4 query-string presign** on the existing `ring`/`hmac`/`sha2` stack (extend the
  `aws-sign-v4` approach to the query variant, ~1 canonical-request function). **🔴 crypto-adjacent:** a
  signing bug is either *fail-safe* (R2 rejects → uploads break, loud) **or**, if the canonical request
  is built too loosely (e.g. omitting `ContentType`/size from the signed set), a **more-permissive URL
  than intended**. Requires a **byte-fidelity test vector** against a known-good AWS/R2 presign (the
  algorithm is fully specified + deterministic, so this is testable offline). *(no new topology; owns a
  crypto surface in the scratch image)*
- **(c) Eliminate presign — server-proxied upload.** Keep the client contract identical (FE still gets
  `{uploads:[{key,url}], expiresIn}` and PUTs bytes to `url`), but point `url` at a **Rust API endpoint**
  that accepts the PUT, validates, and forwards to R2 via the **already-working** SigV4 header-signed
  `put` (`storage.rs` extended). **Removes the entire leaked-presign + hand-rolled-crypto surface.**
  Cost: bytes flow through the API (≤25 MB clip buffered/streamed — bandwidth + memory), losing the
  direct-to-R2 offload; and it is a **topology change** (behavior-identical to the client, different on
  the wire). *(smallest attack surface; largest deviation from the R2-offload design)*
- **(d) Hybrid-cutover — keep presign on Node** until the `media-worker` image exists (§3-C); Rust owns
  confirm + processing meanwhile. *(defers the crypto decision without blocking S4)*

**R3 recommendation:** **(d) for cutover.** The Q3 spike (§4) **removed the "the split is coming anyway
for imaging" argument** — presign is now the *sole* in-packet reason to stand up a `media-worker`, so
(a) must justify a whole second always-on runtime on presign alone (weak at single-digit QPS). That
tilts the packet toward **(c) server-proxied upload** as the failure-first dark-horse: it deletes the
leaked-presign threat class entirely and needs no second runtime, at a bandwidth cost the
back-of-envelope absorbs — worth an explicit breaker look. **(b) only if** the operator wants presign in
the scratch `api` and accepts owning a crypto surface — and then **only behind a byte-fidelity SigV4
presign test vector** signed off as a 🔴 crypto item. 🔴 — Q2.

---

## 6. Unauthenticated entry-photo upload — the sharpest red-line (Q4 🔴)

**Exact current behavior (`spa-proxy.ts:268-293`):** `POST /api/public/entry-photo` — **no auth**, IP
rate-limited **8/min**, accepts a multipart `file` field ≤ **8 MB**, requires `mimetype` starts with
`image/`, then sharp `.rotate().resize(1024×1024, inside).webp(q78)`, stores to
`entry-photos/${crypto.randomUUID()}.webp` in R2, returns `{key, url}`. **No DB write** — the key is
handed back to the (anonymous, pre-order) checkout client, which passes it to order-create later. The
object is served publicly via the `/images/*` proxy (traversal-guarded only, **no tenant/auth check** —
S1). The stated control model: *"the key is unguessable and only revealed to the assigned courier
during the active order."*

**Abuse surface (why this is 🔴):**
- **DoS / cost** — anyone on the internet can burn 8/min/IP × 8 MB = **~64 MB/min/IP of R2 writes**,
  each producing a permanent orphan (no order ever attaches). Distributed across IPs this is an
  unbounded, unauthenticated R2 fill + egress bill. The only limiter is IP rate-limit (trivially
  botnet-bypassable) and the 8 MB size cap.
- **Illegal / abusive content hosting** — an unauthenticated actor can upload arbitrary imagery and
  receive a **stable public URL** on the product's domain (the app becomes an open image host for CSAM/
  hate/etc. laundered through the brand's domain). Content-addressing by UUID makes it unguessable but
  the uploader *holds* the URL and can distribute it.
- **PII intake** — the photo is by design a **customer's doorway/building** (and may incidentally
  contain faces, plates, house numbers). EXIF-GPS is stripped by `.rotate()`+re-encode (good), but the
  **image content itself is location-PII** stored indefinitely with no retention/erasure binding
  (GDPR-adjacent; the S9 GDPR slice erases customer rows but entry-photo objects are keyed by UUID, not
  by customer — **orphaned from the erasure graph**).

**Disposition: CARRY the flow (it is a live UX-3 feature) but FIX-IN-PORT the compensating controls —
this is where the port *improves* on parity, with the council's blessing.** Proposed compensating
controls (each a Q4 sub-decision):
- **Magic-byte sniff before store** (reuse `sniffMime`, `product-media-validation.ts:68`) — reject
  anything not actually webp/jpeg *before* the sharp decode, closing the "claim `image/` header, send a
  bomb" gap (also feeds S4-T6 decompression-bomb bound).
- **Global (not just per-IP) rate cap + a kill-switch flag** (default-on) — a system-wide token bucket
  so a botnet can't fan out; `ENTRY_PHOTO_ENABLED` flag lets ops kill the open front door instantly.
- **TTL/reaper on unattached entry-photos** — objects not referenced by an order within N hours are
  swept (ties into §7 orphan cleanup + the GDPR erasure graph gap).
- **Hard pixel/dimension cap** pre-decode (bomb bound, Q3 crit-6).

**Alternative dispositions the council must weigh:** *(a)* CARRY verbatim (parity-pure, accept the
abuse surface as pre-existing — matches the default fix-vs-carry rule but leaves an unauthenticated
harm surface unaddressed at exactly the moment we're rewriting it); *(b)* require a **short-lived
checkout-scoped token** (mint a cheap anonymous upload token at checkout-open, gate the endpoint on it —
removes "anyone on the internet" but adds a token seam to the pre-order flow); *(c)* CARRY + compensating
controls *(recommend)*. **🔴 operator sign-off** — this is an unauthenticated-harm surface being
touched during a rewrite; the Ethics Charter and the "no new harm surface" posture make the disposition
a human decision, not an architect default.

## 7. Content-addressed keys + old-key cleanup — parity, and the orphan-accumulation quirk

**sha256×12 parity (two *different* content-addressing schemes — carry both, exactly):**
- **Product image** (`spa-proxy.ts:235`): key = `${locId}/${pid}-${sha256(processed).slice(0,12)}.webp`
  — hashed over the **server-processed** bytes. Rationale carried verbatim: `/images/*` serves
  `max-age=31536000, immutable`, so a fixed key would pin the first upload forever; a content-hash means
  a changed image is a new URL the CDN fetches fresh. **CARRY.**
- **Product media** (`product-media.ts:165`): key uses `it.sha256.slice(0,12)` — the **client-declared**
  hash of the **raw** upload, **never re-verified** against the stored bytes (confirm sniffs the *mime
  magic bytes* but does not re-hash). So the "content address" is a **best-effort name, not an integrity
  proof**; a client can name an object with a hash that doesn't match its bytes. **CARRY** (it's a
  naming scheme, not a security control — the mime sniff is the real gate) — but Q7 asks whether confirm
  should re-hash (closes the naming-lie, costs a full-object read on confirm).

**Old-key cleanup — the best-effort-swallowed quirk (CARRY) + orphan accumulation (Q6):**
- Product-image swap captures `oldKey` then, post-DB-update, `storage.delete(oldKey)` inside a
  try/catch that **swallows the error** (`spa-proxy.ts:257-259`, "cleanup is best-effort"). **CARRY
  verbatim** — a failed cleanup must never fail the user-visible upload. Consequence: **every failed
  delete leaks one orphaned object.**
- **Orphan sources (none of which count against the 150 MB budget — the budget is `SUM(bytes)` over
  `product_media` *rows*, so orphaned R2 objects are invisible to it):** (1) failed best-effort deletes;
  (2) **presigned-but-never-confirmed** uploads (client PUT to R2, never called confirm → object exists,
  no row); (3) `product_media` has **no delete route** (`product-media.ts` exposes presign/confirm/
  set-primary/reorder/available-toggle — **no DELETE**), so replaced/hidden media objects accumulate;
  (4) unattached **entry-photos** (§6). **Net: R2 object count grows monotonically; only the DB budget
  is bounded.** This is a *pre-existing* property (CARRY), but the port is the natural place to decide
  whether a **reaper** (sweep orphans older than N days with no referencing row) is added — Q6 (default:
  **carry the leak, defer the reaper to an ops-binary slice**, note it as an accepted risk with an owner).

## 8. Tenancy — metadata through `with_user`; R2 keys are NOT RLS-protected

**Two distinct authority planes; spell out the boundary because they are easy to conflate:**

- **DB metadata plane (RLS-governed).** Every tenant-scoped media metadata write — the `product_media`
  confirm INSERT, set-primary `UPDATE products`, reorder/available-toggle `UPDATE product_media`, and
  the product-image `UPDATE products.image_key` and theme-logo `UPDATE location_themes.logo_url` — is a
  tenant table write and **MUST** go through the S3 **`with_user(pool, UserId, …)`** combinator
  (`app.user_id`→memberships), inheriting **REV-10** (the tenancy-GUC contract ADR: `UserId`/`TenantId`
  are non-confusable, a wrong-context call is a compile error). `product-media` confirm already does
  this (`withTenant(db, userId)`); **product-image and theme-logo do NOT** (raw pool — Q5, §1). Post-flip
  those two match 0 rows. **Fix all three onto `with_user`; the location is bound from the
  membership-resolved `locId` / validated path param, never the body** (REV-4).
- **R2 object plane (NOT RLS-governed — no per-tenant ACL exists).** Cloudflare R2 has no notion of
  `app.user_id`. The tenant boundary on the object plane is **purely the key derivation + the
  presign/confirm prefix check**: keys are server-built from the membership-resolved `locId`
  (`${locId}/…`), and confirm rejects any `storageKey` outside `${locId}/${productId}/`. **WRITE
  isolation = server-controlled key + presign scope** (a client can never sign/confirm into another
  tenant's prefix). **READ isolation = none by design** — every object is public via the traversal-only
  `/images/*` proxy; menu images and logos *are* public storefront content, so this is correct for
  them. **Entry-photos are the exception** (§6): their only read control is key-unguessability, not a
  tenant/auth check — spell this out so the council prices it.

**The authz boundary in one sentence:** *the DB row that references a key is tenant-isolated by
`with_user`+RLS; the object the key points at is tenant-isolated on **write** by server-side key
derivation and **not at all on read** — which is intended for public menu media and a **named residual
risk** for entry-photos.*

## 9. Quirk register — carry-vs-fix (default = CARRY-VERBATIM)

**FIX-IN-PORT only for a 🔴 security-correctness issue or a build-correctness bug, each with an explicit
test/E2E delta.** Everything else CARRIES; shape-migration rows defer to post-Astro FE-lockstep.

| ID | Quirk (source) | Disposition |
|---|---|---|
| Q-GUC-PRODIMG | product-image write `UPDATE products` on the **raw pool**, no GUC (`spa-proxy.ts:252`) | **FIX-IN-PORT** → route through `with_user`; 0-rows post-flip otherwise. NOBYPASSRLS probe (inherits REV-5) |
| Q-GUC-LOGO | theme-logo write `UPDATE location_themes` on raw `db.connect()`, no GUC (`themes.ts:139-143`) | **FIX-IN-PORT** → `with_user` (REV-2 named `themes` exactly this); authz via the existing `requireLocationAccess` P-d |
| Q-GUC-CONFIRM | product-media confirm already correct: `withTenant(db, userId)` (`product-media.ts:242`) | **CARRY** → port to `with_user` verbatim (the reference impl for the other two) |
| Q-PRESIGN-TTL | presign `expiresIn: 300` (`product-media.ts:29`) | **CARRY** — 300 s is the contract; whichever presign path (Q2) preserves it |
| Q-PRESIGN-CRYPTO | presign via `@aws-sdk/s3-request-presigner`; no Rust equivalent without `cmake` (`storage.rs:104-119`) | **🔴 decision** (Q2) — media-worker+aws-sdk / hand-rolled query-SigV4 / server-proxy / Node-during-cutover |
| Q-KEY-DERIVE | keys server-built from membership `locId`; confirm rejects out-of-prefix (`product-media.ts:165,209`) | **CARRY verbatim** — the sole cross-tenant WRITE boundary on the object plane |
| Q-SHA-DECLARED | product_media key uses **client-declared** sha256, never re-verified (`product-media.ts:165`) | **CARRY** — naming scheme, not integrity; mime-sniff is the gate. Q7: re-hash on confirm? |
| Q-SHA-PROCESSED | product-image key = sha256 of **processed** bytes, ×12 (`spa-proxy.ts:235`) | **CARRY verbatim** — immutable-cache-busting rationale |
| Q-CLEANUP-SWALLOW | old-key `storage.delete` best-effort, error swallowed (`spa-proxy.ts:257-259`) | **CARRY verbatim** — cleanup must never fail the upload |
| Q-ORPHANS | orphaned objects accumulate (failed deletes, unconfirmed presigns, no media-delete route, unattached entry-photos); invisible to the 150 MB budget | **CARRY leak + DEFER reaper** (Q6) — accepted risk w/ owner; reaper = ops-binary slice |
| Q-ENTRY-UNAUTH | `POST /api/public/entry-photo` unauth, IP-8/min, 8 MB, public URL, no retention binding (`spa-proxy.ts:268`) | **🔴 CARRY flow + FIX compensating controls** (Q4) — sniff, global cap + kill-switch, TTL reaper, bomb bound |
| Q-BUDGET-SHAPE | budget 413 returns a **bare non-envelope** `{error, used, incoming, limit}` (`product-media.ts:128-131`) | **CARRY** — post-Astro FE-lockstep (matches S3 Q3 posture); inline-fix candidate |
| Q-NO-SVG | mime allow-list omits SVG (active-content/XSS); sniff never recognizes it (`product-media-validation.ts:13,68`) | **CARRY verbatim** — a security invariant, not a gap; guardrail asserts SVG rejected |
| Q-STRIP-META | logo "strip EXIF"; entry-photo `.rotate()` bakes+strips (privacy) (`themes.ts:126`, `spa-proxy.ts:279`) | **CARRY as a REQUIREMENT** — spike confirms `image` strips metadata by default (parity); assert no EXIF/GPS passthrough post-port |
| Q-IMG-CRATE | sharp (libvips native binary) → Rust crate pick | **DECIDED (spike):** pure-Rust `image` 0.25 + `webp` 0.3 (`cc`-only, no `cmake`, scratch/musl-safe, in-process); `libvips` crate REJECTED (LINK-time system-lib trap). Evidence `rebuild/spikes/image-stack/`. Behind `trait ImageProcessor` |
| Q-EXIF-ORIENT | `image::open()` does NOT auto-apply EXIF orientation; sharp applies it implicitly on **every** profile | **FIX-IN-PORT (silent-wrong-output):** explicit `decoder.orientation()`→`apply_orientation()` on **all three** handlers (product/logo/entry-photo). DoD test: rotated-EXIF fixture → upright output (REV-EXIF) |
| Q-MEDIA-SERVE-GATE | rich-media served only when flag ON **and** plan=`business` (`product-media-validation.ts:145`) | **CARRY verbatim** — defense-in-depth `{media:[]}` when gated |
| Q-BRAND-SAMPLE | `brand-extractor` `resize(48×48).raw()` shares the imaging crate but is a branding route | **NOTE dependency** (Q9) — rides Q3's crate pick; port with its own surface or as an S4 non-🔴 rider |

## 10. Cutover DoD (REBUILD-MAP §3, this surface)

Media E2E slice green (as-is specs — `flow-ui-images.spec.ts`, `flow-client-product-images.spec.ts`,
`media-render.spec.ts`) · `openapi-diff` empty for the S4 namespace · invariant-cluster red→green:
**all three metadata writes seat `app.user_id` under a live NOBYPASSRLS probe** (product-image +
theme-logo fixed from raw-pool; confirm carried) · **image-transcode parity** (pure-Rust `image`+`webp`,
decided §4) — a fixture upload yields webp at 800×800/512×512/1024×1024, `fit:inside` matching sharp ·
**REV-EXIF orientation** — a rotated-EXIF fixture yields **upright** output on all three handlers
(explicit `apply_orientation`, the silent-wrong-output guard) · **EXIF/GPS stripped** on
entry-photo/logo (assert no EXIF in output) · **SVG rejected** at presign + sniff · **decompression-bomb
bound** proven (oversized-dimension fixture rejected pre-alloc via `image` Limits) · **presign scope** — a confirm with a
`storageKey` outside `${locId}/${productId}/` → 400; a presign never signs another tenant's prefix ·
**300 s TTL** preserved · entry-photo compensating controls per Q4 disposition · orphan posture per Q6
(accepted-risk row or reaper) · map-coverage zero-diff for the S4 namespaces · **council sign-off +
rollback plan** (flag-flip media routes back to Node behind the proxy; if §3-A, the `media-worker` is
independently rollback-able). No 🔴 S4 row builds before this packet is APPROVED and the 🔴 questions
are operator-signed.

---

**council seats to run: breaker, counsel** (architect authored; human operator signs 🔴 Q1/Q2/Q4 —
**Q3 is spike-resolved**, no sign-off, carrying only the REV-EXIF build-lane fix).
**packet-status: 🟡 DRAFT.**
