# S4-MEDIA Port — Council Packet · OPEN QUESTIONS

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Everything the live Triadic Council (architect / breaker /
> counsel / human) must decide before S4 media is ported. Each question has options + a lane-R3
> recommendation — a *starting position for friction*, not a decision. Docs only.

Legend: **[INFRA]** topology/deploy · **[SEC]** security-correctness · **[CRYPTO]** signing-adjacent ·
**[SCOPE]** surface placement · **[CONTRACT]** shape/parity · **[PRIV]** privacy/PII. 🔴 = red-line,
operator sign-off required.

---

### Q1 🔴 [INFRA] The two-image split — commit now, or build the seam and split on demand?
REBUILD-MAP §2 verdicts a `scratch api` + `Debian-slim media-worker` split. S4 is the first surface
that forces it (first CPU-heavy, native-dependency workload). The build constraint decides: the
scratch/musl `api` cannot easily carry `cmake`-dependent native crates (`aws-lc-sys`, libvips); a
Debian-slim `media-worker` can. Back-of-envelope (proposal §2): single-digit uploads/sec — no
connection pressure, only CPU/egress.
- **(a)** Commit to the split now — `media-worker` owns transcode + presign (can use `aws-sdk-s3`
  presigner + libvips-FFI); `api` proxies. +1 always-on Fly app, cross-service call on a request-path op.
- **(b)** Keep processing in-process in `api`, split later — forces pure-Rust `image` (Q3) **and**
  hand-rolled presign (Q2) into S4.
- **(c)** Schema-rich/runtime-minimal: build the `trait ImageProcessor` seam + author the `media-worker`
  Dockerfile now, keep transcode **in-process**, extract the worker only when Q3 forces libvips **or**
  the post-S4 OCR slice needs Tesseract/pdfium (split pays across two surfaces, not one). *(recommend)*

**R3 recommendation:** (c). Do not stand up a second always-on runtime for a single-digit-QPS workload
(the Prime Video re-monolith lesson); build the *seam*, defer the *runtime*. **Q3 is now resolved to
pure-Rust `image`+`webp` (scratch/musl-safe, in-process) — so imaging NO LONGER forces the split.** The
only remaining in-packet driver is presign (Q2); extract the `media-worker` only when presign lands
there (Q2a/d) or the post-S4 OCR slice needs Tesseract/pdfium. 🔴 — binds deploy shape + the presign
path. Owner: architect + operator.

### Q2 🔴 [CRYPTO/SEC] Presigned PUT in Rust — the algorithm the shipped client does not implement
S1's R2 client signs with `aws-sign-v4` (SigV4 **Authorization-header**, for requests the server
sends). A presigned URL is the SigV4 **query-string** variant — a different canonical request.
`aws-sdk-s3`'s presigner does it but needs `cmake` (absent, `storage.rs:104-119`). Leaked-presign blast
radius is bounded (one object, one product, ≤300 s, same tenant — proposal §5) **iff signing is correct**.
- **(a)** Stand up `media-worker` (Debian-slim) and use `aws-sdk-s3`'s presigner — no hand-rolled crypto
  (forces Q1(a)).
- **(b)** Hand-roll SigV4 query-string presign on the existing `ring`/`hmac`/`sha2` stack — **🔴
  crypto-adjacent**: a loose canonical request = a more-permissive URL than intended. Requires an
  **offline byte-fidelity test vector** vs a known-good AWS/R2 presign.
- **(c)** Eliminate presign — **server-proxied upload**: keep the client contract identical (`url`
  points at a Rust API endpoint instead of R2; client still PUTs bytes to `url`), forward to R2 via the
  already-working header-signed `put`. Deletes the leaked-presign + hand-rolled-crypto surface; costs
  API bandwidth/memory (≤25 MB clip).
- **(d)** Hybrid-cutover — keep presign on **Node** until the `media-worker` image exists; Rust owns
  confirm + transcode meanwhile.

**R3 recommendation:** **(d) for cutover** — keep presign on Node while Rust owns confirm + transcode.
**Note Q3's resolution removed the "the split is coming anyway for imaging" argument** — presign is now
the *sole* in-packet reason to stand up a `media-worker`, so option (a) must justify a whole second
runtime on presign alone (weak, given single-digit QPS). That tilts the decision toward **(c)
server-proxied upload** (no second runtime, no hand-rolled crypto, deletes the leaked-presign threat
class at a bandwidth cost the back-of-envelope absorbs) — it deserves an explicit breaker look. **(b)
only** behind a signed-off byte-fidelity SigV4 test vector. 🔴 crypto. Owner: architect + operator (+
breaker on (c)).

### Q3 ✅ [INFRA] Image processing crate — RESOLVED BY SPIKE (row kept with evidence)
Three sharp transcode profiles (+ one read-only sampler), all WebP-only, q78–82, `fit:inside`
(proposal §4). A parallel spike lane resolved this in the *actual* build env — no longer open.
- **(a) CHOSEN — pure-Rust `image` 0.25 + `webp` 0.3.** `webp`→`libwebp-sys` is a **`cc`-only** build:
  **no `cmake` anywhere in the graph**, and it links a `cc`-compiled *static* libwebp (not a system
  shared lib) → **scratch/musl-safe, in-process in `api`**. Proven: builds/tests/clippy clean; product
  2000×1500→800×800 webp q82 = **76 ms / 36.7 MB peak RSS**, logo **64 ms**; `fit:inside` matches sharp
  **bit-for-bit**; metadata **stripped by default** (matches sharp). **Consequence: imaging does NOT
  force the split** — Q1/Q2 lose their imaging driver.
- **(b) REJECTED — `libvips` crate.** *Not* `cmake`-blocked (`build.rs` is plain `rustc-link-lib`) but
  needs system `libvips`/`glib`/`gobject` at **LINK** time → `apt-get install libvips-dev` (198 pkgs)
  → permanently breaks `cargo build`/`test`/`clippy` in dev/CI unless provisioning is kept in lockstep.
  The exact **"works in Docker, broken locally" trap already rejected for `aws-sdk-s3`**. Docker-only.

**Evidence:** worktree `agent-ad562cc71d9c3d9b1`, `rebuild/spikes/image-stack/` —
`attempt-a-libvips/` fails at link with verbatim `-lvips` errors; `attempt-b-pure-rust/` runs with the
timings above. Criteria evaluated: WebP lossy-q parity ✅ · `fit:inside` ✅ (bit-for-bit) · **EXIF
auto-orient ❌ — see below** · strip-metadata-by-default ✅ · build-env ✅ (scratch/musl) · bomb-bound
✅ (`image` `Limits`).

**🔴-adjacent CARRY (REV-EXIF):** `image::open()` does **NOT** auto-apply EXIF orientation; **sharp
applies it implicitly on every profile.** The port must **explicitly** call `decoder.orientation()`→
`apply_orientation()` on **all three** handlers (product/logo/entry-photo) — miss it and the failure is
**silent wrong output** (sideways/mirrored images that still "render some webp"). Named DoD item + test:
rotated-EXIF fixture → upright output. **Owner: S4 build lane.**

**R3 disposition:** DECIDED — build behind `trait ImageProcessor` with the pure-Rust impl; carry
REV-EXIF as a build-lane DoD item. No operator sign-off needed (evidence-settled); the residual work is
the explicit-orientation fix, not a choice.

### Q4 🔴 [SEC/PRIV] Unauthenticated entry-photo upload — CARRY, harden, or gate?
`POST /api/public/entry-photo` (`spa-proxy.ts:268-293`): no auth, IP-8/min, 8 MB, `image/*` header
check, sharp rotate+resize+webp, stored to a public URL, no retention binding. Abuse surface: DoS/cost,
illegal-content hosting on the brand domain, doorway/face PII with no erasure-graph link (proposal §6).
- **(a)** CARRY verbatim — parity-pure; accept the abuse surface as pre-existing.
- **(b)** CARRY the flow + **FIX compensating controls**: magic-byte sniff before decode; **global**
  rate cap + `ENTRY_PHOTO_ENABLED` kill-switch (default-on); TTL reaper on unattached photos; hard
  pixel/dimension bomb bound. *(recommend)*
- **(c)** Require a **short-lived checkout-scoped anonymous upload token** — removes "anyone on the
  internet" at the cost of a token seam in the pre-order flow.

**R3 recommendation:** (b). This is an **unauthenticated-harm surface being touched during a rewrite** —
the Ethics Charter's no-new-harm posture plus the "improve-on-parity where the port naturally can" rule
make hardening the right default; (c) is the stronger control if the operator wants "no open front door"
and accepts the checkout-flow seam. **CARRY-verbatim (a) is explicitly the weakest** — it re-ships an
open harm surface knowingly. 🔴 operator + counsel (ETHICAL-STOP-adjacent: harm-hosting + PII).

### Q5 [SEC] The two raw-pool media writes — inherit REV-2/REV-10 `with_user` (settled, confirm scope)
product-image (`spa-proxy.ts:252`, raw `db.query`) and theme-logo (`themes.ts:139-143`, raw
`db.connect()`) write tenant tables with **no GUC seat** — the never-copy leak class; 0 rows post-flip.
product-media confirm is already correct (`withTenant`).
- **(a)** Route all three through the S3 **`with_user`** seam (REV-10 non-confusable types); location
  bound from membership/path param, never the body (REV-4); NOBYPASSRLS probe asserts seating. *(recommend)*
- **(b)** Carry product-image/theme-logo raw-pool as-is — **rejected** (silent write outage post-flip).

**R3 recommendation:** (a). Directly inherited from S3 REV-2 (which named `themes`) and REV-10; not a
new decision so much as a scope confirmation that S4's two legacy writes are inside it. Not 🔴 on its
own (settled by the S3 ADR) but named so the build lane can't miss it. Owner: S4 lead.

### Q6 [SCOPE] Orphaned-object cleanup — carry the leak, or add a reaper?
Orphans accumulate from failed best-effort deletes, unconfirmed presigns, the absent media-delete
route, and unattached entry-photos; none count against the 150 MB DB budget (proposal §7).
- **(a)** CARRY the leak (accepted risk with a named owner); defer a reaper to an **ops-binary slice**
  (same home as the backup sidecar) so the API request path stays clean. *(recommend)*
- **(b)** Add a reaper now (sweep objects older than N days with no referencing row) — but it needs an
  R2 `list` + a DB reconcile, a new always-on job on a low-value target.

**R3 recommendation:** (a). Boring-and-proven: the leak is bounded by upload volume (back-of-envelope),
costs pennies of R2, and a reaper is a genuine ops job better owned with the backup/DR ops-binary than
bootstrapped into the media request surface. Records an accepted-risk row + owner. Owner: S4 lead →
ops-binary slice.

### Q7 [CONTRACT] product_media key uses a client-declared, never-verified sha256 — re-hash on confirm?
The key embeds `it.sha256.slice(0,12)` (client-declared, `product-media.ts:165`); confirm sniffs the
mime magic bytes but never re-hashes the stored object (proposal §7).
- **(a)** CARRY — the key is a name, not an integrity proof; the mime sniff is the real gate. *(recommend)*
- **(b)** Re-hash the stored object on confirm and reject on mismatch — closes the naming-lie at the
  cost of a full-object read (≤25 MB) on every confirm.

**R3 recommendation:** (a). The content-address is a CDN-cache-busting name; integrity is not a stated
property of this field, and (b) buys little for a real per-confirm cost. Inline-fix candidate only if a
threat emerges. Owner: S4 lead.

### Q8 [CONTRACT] Non-envelope response shapes (budget 413) — carry or normalize?
presign budget-exceeded returns a bare `{error, used, incoming, limit}` (`product-media.ts:128-131`),
not the `sendError` envelope.
- **(a)** CARRY verbatim; normalize in the post-Astro FE-lockstep pass (matches S3 Q3 posture). *(recommend)*
- **(b)** Normalize to the envelope now.

**R3 recommendation:** (a). Same posture as S2 Q4 / S3 Q3 — divergent shapes carry; migrating during
the port couples two risky changes. Inline-fix candidate. Owner: S4 lead.

### Q9 [SCOPE] Brand-extractor colour-sample — S4 rider or its own branding surface?
`brand-extractor.ts:307-311` (`resize(48×48).raw()` pixel extraction) shares the imaging crate Q3 picks
but its route (`POST /api/owner/brand/generate`) is a branding concern.
- **(a)** Port it as a **non-🔴 S4 rider** since it rides the same `ImageProcessor` seam. *(recommend)*
- **(b)** Defer to a branding surface — but then the imaging crate is stood up in S4 and re-imported
  later for one call site.

**R3 recommendation:** (a). It's one pure imaging call on the crate S4 already picks; a rider is cheaper
than a re-import. Note the dependency in the matrix. Owner: S4 lead.

---

## Decision-ordering note for the council
**Q3 (crate) is RESOLVED by the spike** — pure-Rust `image`+`webp`, in-process, scratch-safe. Its
cascade already fired: it **removed the imaging driver for the split**, so the once-coupled triad
collapses to **Q1↔Q2**. Now the **only** in-packet reason to stand up a `media-worker` is presign (Q2),
which means **Q1 (split) is downstream of Q2 (presign)**, not co-equal: decide presign first. If Q2
picks (c) server-proxy or (d) Node-during-cutover, **Q1 answers itself — no split** (converge to the
worker only when the post-S4 OCR slice needs it). If Q2 picks (a), Q1(a) follows. These two remain
**port-blocking** for the *upload/presign* code (not for transcode/metadata, which build now on the
decided crate + the S3 `with_user` seam). **Q4 (entry-photo)** is a **red-line scope decision** that
blocks *that one route* but not the whole surface — the packet's most likely **ETHICAL-STOP-adjacent**
item (unauthenticated harm + PII), wanting counsel eyes independent of Q1/Q2. **Q5** is settled by the
S3 REV-2/REV-10 ADR (confirmation, not litigation). The **REV-EXIF** orientation fix (from the Q3 spike)
is a build-lane DoD item, not a council decision. **Q6/Q7/Q8/Q9** are build-detail decisions that can
settle at build time without blocking approval.
