# S4-MEDIA Port — Council Packet · THREAT MODEL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Adversarial input to the S4 council. Assets, trust boundaries,
> and the failure modes the Rust port must not silently introduce. Read alongside `proposal.md`. Docs
> only; no code.

- **Method:** STRIDE-lite over the S4 media surface + fold-in of the S3 RLS-reliability posture (the
  `with_user`/`app.user_id` GUC-family discipline, REV-2/REV-4/REV-5/REV-10) + the object-plane
  boundary that has no RLS analog (R2 has no per-tenant ACL).
- **Scope note:** the B3 (NOBYPASSRLS) flip and the `app_member_location_ids()` search_path pin are
  **B3-council fixes**; their *disposition* is recorded here because they change what S4 must hold, but
  their *fix* lives in that council. The backup multipart upload and the menu-import OCR intake are
  **out of S4** (proposal §2) — their threats are owned by the backup/DR ops-binary and the OCR slice.

---

## 1. Assets

| ID | Asset | Where it lives | Why it matters |
|---|---|---|---|
| M1 | Product images / rich media objects | R2 (`${locId}/${pid}-${hash}.webp`, `${locId}/${productId}/…`) | The storefront's menu imagery; wrong-tenant write = cross-tenant menu defacement |
| M2 | Theme logo objects | R2 (`locations/${locationId}/logo.webp`) | Brand identity on the public storefront; overwrite = brand defacement |
| M3 | Entry-photo objects (customer doorway/building) | R2 (`entry-photos/${uuid}.webp`) | **Customer location-PII**; unauthenticated intake; no erasure-graph link |
| M4 | `product_media` metadata rows (key, bytes, sort, primary, available, meta) | tenant table, RLS-scoped | Points at the objects; wrong-GUC write = 0 rows post-flip = silent media outage |
| M5 | R2 credentials + presign signing material (`R2_SECRET_ACCESS_KEY`) | env / `secrecy::Secret` | A leaked/over-broad signature = writes into the bucket beyond one object |
| M6 | Per-location 150 MB storage budget | derived: `SUM(bytes)` over `product_media` | The only bounded resource on the object plane; orphans bypass it |
| M7 | Content-integrity of served bytes | R2 object + `/images/*` immutable cache | A mismatched-type or malicious object served with a 1-year immutable cache is sticky |

## 2. Trust boundaries

- **TB-1 owner → location (metadata plane)** — ADR-0004 P-d live `status='active'` membership re-read.
  product-media resolves via `getOwnerLocation` (`product-media.ts:46-64`); product-image via the
  spa-proxy `getLocationId` (`spa-proxy.ts:57+`); theme-logo via the `requireLocationAccess` hook
  (`auth.ts:148-151`). The JWT `activeLocationId` is **never trusted alone** — the live memberships row
  is authority. Port 1:1 through the S4 owner extractor.
- **TB-2 request → GUC seat (metadata plane)** — the `with_user` combinator turns the bearer identity
  into `app.user_id` inside one txn. **Currently seated on only 1 of 3 writes** (confirm yes;
  product-image + theme-logo no — proposal §8, Q5). A context-free write dissolves the RLS arm.
- **TB-3 client → R2 (object WRITE plane)** — **no RLS exists here.** The boundary is server-side **key
  derivation** (`${locId}/…` built from the membership `locId`, never client-supplied) + the confirm
  **prefix check** (`storageKey.startsWith(${locId}/${productId}/)`, `product-media.ts:209`) + the
  presign **scope** (signature bound to one key/method/content-type). A leaked presign = one object,
  one product, ≤300 s, same tenant.
- **TB-4 anyone → R2 (unauthenticated object WRITE)** — `POST /api/public/entry-photo`: the request
  body **is** the authority; only IP-rate-limit (8/min) + an 8 MB cap + an `image/*` header check gate
  it. No auth, no tenant, no CAPTCHA, no global cap. The weakest boundary in the surface.
- **TB-5 anyone → R2 (object READ)** — `/images/*` and `/media/*` are traversal-guarded **only** (S1
  `storage.rs::validate_object_key`); **no tenant/auth check**. Every object is public-by-key. Correct
  for M1/M2 (public menu content); the sole control for M3 (entry-photo) is key-unguessability.

## 3. Port-specific failure scenarios (Rust)

| # | Scenario | Trigger in the port | Mitigation to prove red→green |
|---|---|---|---|
| **S4-T1** | **Wrong/absent GUC on a media metadata write** — product-image / theme-logo write matches 0 rows post-flip (silent media-write outage) | Carrying the raw-pool `db.query`/`db.connect()` writes verbatim instead of `with_user` (Q5) | Route all three through `with_user`; NOBYPASSRLS probe asserts `app.user_id` seated on each media-metadata txn and the UPDATE/INSERT affects the intended row (inherits S3 REV-5 probe scope) |
| **S4-T2** | **Cross-tenant object write** — a client writes into another tenant's R2 prefix | Deriving the key from client input, or dropping the confirm prefix check | Keys built **only** from the membership `locId`; confirm rejects any `storageKey` outside `${locId}/${productId}/` → 400. Presign never signs a client-supplied key. Guardrail test on both |
| **S4-T3** | **Over-broad presign** — a signing bug yields a URL wider than {one key, PUT, ≤300 s} | Hand-rolled SigV4 query-presign (Q2b) with a loose canonical request (omitting content-type/expiry from the signed set) | If Q2b: an **offline byte-fidelity test vector** vs a known-good AWS/R2 presign; assert the signed canonical request pins method+key+expiry+content-type. If Q2a/c/d: no hand-rolled crypto to break |
| **S4-T4** | **PII/EXIF passthrough** — a ported transcode leaks camera GPS/EXIF into the stored/served object | The imaging crate does not strip metadata by default | **Spike-settled:** the decided crate (`image` 0.25, §4) strips metadata **by default**, matching sharp. DoD: assert **zero EXIF/GPS** in transcoded output for logo + entry-photo (fixture with GPS-EXIF → output has none) |
| **S4-T4b** | **Silent wrong-orientation output (REV-EXIF)** — sideways/mirrored images that still "render some webp" and pass a weak assertion | `image::open()` does NOT auto-apply EXIF orientation; sharp applies it implicitly on **every** profile | **FIX-IN-PORT:** explicit `decoder.orientation()`→`apply_orientation()` on **all three** handlers (product/logo/entry-photo). DoD test: rotated-EXIF fixture → **upright** pixel output. Not privacy — a parity-correctness silent failure the spike surfaced |
| **S4-T5** | **Active-content / XSS via SVG or type-confused upload** — an SVG or mislabeled file is stored and served as an image | Widening the mime allow-list or dropping the magic-byte sniff on port | Carry the allow-list (webp/jpeg/mp4, **never SVG**) + `sniffMime` verbatim; confirm re-sniffs stored bytes; guardrail asserts SVG + type-mismatch rejected. Entry-photo gains a pre-decode sniff (Q4b) it lacks today |
| **S4-T6** | **Decompression-bomb DoS** — a small file decodes to gigabytes, OOMing the **in-process** transcoder (the decided crate runs in `api`, §3/§4) | Decoding untrusted bytes with no pixel/dimension cap | **Non-optional:** a hard pixel/dimension cap **before decode** via `image`'s `Limits` API on every transcode incl. entry-photo; oversized-dimension fixture rejected pre-alloc. The 36.7 MB peak RSS the spike measured is for *benign* input — a bomb has no such bound without `Limits` |
| **S4-T7** | **Unauthenticated abuse of entry-photo** — DoS/cost flood, illegal-content hosting on the brand domain, PII intake with no retention (TB-4) | Porting the public front door with parity-only controls (IP-8/min + 8 MB) | Q4 disposition: sniff + **global** rate cap + `ENTRY_PHOTO_ENABLED` kill-switch + TTL reaper + bomb bound (Q4b), or a checkout-scoped token (Q4c). **CARRY-verbatim (Q4a) leaves this open — the packet's most likely ETHICAL-STOP** |
| **S4-T8** | **Orphan accumulation / budget bypass** — objects pile up (failed deletes, unconfirmed presigns, no delete route, unattached entry-photos) invisible to the 150 MB budget | Carrying the best-effort-swallowed cleanup with no reaper | CARRY the swallow (must not fail the upload); Q6 accepted-risk row + owner, or a reaper in the ops-binary slice. Bounded by upload volume (back-of-envelope); pennies of R2, not a live DoS |
| **S4-T9** | **Sticky bad object** — a wrong/malicious object served with `max-age=31536000, immutable` can't be flushed | Reusing a fixed (non-content-addressed) key so a re-upload can't bust the cache | Carry the content-hash key scheme (product-image sha256×12 of processed bytes) so any change = a new URL; assert the key changes when bytes change |
| **S4-T10** | **Confirm trusts client metadata** — a confirm writes attacker-chosen `bytes`/`width`/`height`/`mime` decoupled from the actual object | Skipping the confirm-side re-sniff, or trusting `body.bytes` for the budget | Carry the confirm magic-byte re-sniff (`product-media.ts:224-233`); note that `bytes` is client-declared and feeds the budget (Q7-adjacent) — the budget is advisory, not a hard integrity boundary; document the residual |

## 4. What the B3 RLS flip changes for S4

- **Today (BYPASSRLS):** RLS is bypassed; the explicit `WHERE location_id` (product-media/confirm) and
  the server-derived key prefix are the only live boundaries. The two raw-pool writes (product-image,
  theme-logo) "work" despite seating no GUC — **the danger is invisible**, exactly the S3 anonymizer-N1
  masking.
- **Post-flip (NOBYPASSRLS):** RLS is authoritative on the metadata plane. `product_media` INSERT needs
  `app.user_id` for `WITH CHECK`; `UPDATE products`/`UPDATE location_themes` need it to see the row. The
  two raw-pool writes **match 0 rows** (S4-T1). The B3-council fixes named (not fixed) here: the
  `app_member_location_ids()` search_path pin (S4 calls it transitively via member-keyed policies) and
  the GUC-always-seated invariant. **The R2 object plane is unaffected by B3** — R2 has no RLS; the flip
  changes nothing on TB-3/TB-4/TB-5. **S4's rule: metadata writes correct independent of which pool role
  is live; object isolation independent of RLS entirely.**

## 5. Residual risks (summary for the human)

- **Entry-photo is an unauthenticated harm + PII surface** (S4-T7 / Q4) — the single item most likely
  to warrant an **ETHICAL-STOP** or at least explicit operator acceptance. CARRY-verbatim re-ships an
  open image-hosting + doorway-PII intake with no retention binding and no erasure-graph link. The
  recommendation (Q4b/c) *reduces* it; it cannot be *eliminated* while the feature exists. **Owner:
  operator + counsel.**
- **R2 objects are public-on-read by design** (TB-5) — correct for menu media/logos; for entry-photos
  the only control is key-unguessability. A leaked key = a publicly-served customer doorway photo.
  Accepted as a *current* property; the reaper (Q6) shortens the window. **Owner: S4 lead.**
- **Presign hand-rolled crypto** (S4-T3 / Q2b) — only a risk if the operator chooses (b) over (a/c/d);
  bounded by the byte-fidelity test vector. A signing *failure* is fail-safe (uploads break, loud); a
  signing *looseness* is the real hazard. **Owner: architect + operator.**
- **Orphan accumulation** (S4-T8 / Q6) — monotonic R2 growth invisible to the DB budget; bounded by
  upload volume, cheap, deferred to an ops-binary reaper. **Owner: S4 lead → ops slice.**
- **`bytes` is client-declared** (S4-T10 / Q7) — the budget is advisory; a lying client can under-report
  size. Not a tenant-crossing or money bug; a storage-accounting fuzz. **Owner: S4 lead.**

None of M1–M7's failure modes except the entry-photo *compensating-control* additions and the two
raw-pool→`with_user` fixes is *introduced* by the rewrite — each is a **current** property the port must
carry **visibly** (matrix row + test). **ETHICAL-STOP candidate: the entry-photo unauthenticated
upload (S4-T7 / Q4)** — an unauthenticated intake that can host arbitrary/illegal content on the brand
domain and store customer location-PII with no retention binding. The architect's default (Q4b harden)
reduces but does not remove it; **counsel must rule whether CARRYing an unauthenticated harm surface
through a deliberate rewrite is acceptable, or whether Q4c (scoped token) / a stronger control is
required before this route ports.**
