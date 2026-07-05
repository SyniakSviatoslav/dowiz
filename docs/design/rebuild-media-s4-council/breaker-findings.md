# S4-MEDIA Port — Council Packet · BREAKER FINDINGS

> **Role:** System Breaker. Axis = *where does this design break*, not *is it nice*. Attacking the
> packet **as amended by `spike-evidence.md`** (which post-dates and answers Q3), and attacking the
> spike's own claims. Every finding is specific + demonstrable (a break scenario **or** a
> back-of-envelope number). **No fixes proposed** — that is the architect's job. Ranked
> CRIT / HIGH / MED / LOW.
>
> **Ground truth read:** old TS `spa-proxy.ts`, `themes.ts`, `product-media.ts`,
> `product-media-validation.ts`, `r2-storage.ts`, `image-url.ts`, `plugins/auth.ts`,
> `CheckoutPage.tsx`, `ContactInfoSection.tsx`; Rust `storage.rs`; crate `aws-sign-v4-0.3.0/src/lib.rs`
> (resolved in `rebuild/crates/api/Cargo.toml:90`); spike `…/attempt-b-pure-rust/src/main.rs`
> (`image` 0.25.10 + `webp` 0.3.1 per its `Cargo.lock`); oracle `e2e/tests/flow-ui-images.spec.ts`.
>
> **Verified-correct packet claims (no finding — recorded so the council isn't re-litigating):**
> product-image raw `db.query` (`spa-proxy.ts:252`) and theme-logo raw `db.connect()`
> (`themes.ts:139-143`) GUC-less — CONFIRMED. product-media confirm uses `withTenant`
> (`product-media.ts:242`) — CONFIRMED. theme-logo authz **is** a live active-owner membership re-read
> (`plugins/auth.ts:148-151`, applied via the `onRequest` hooks at `themes.ts:12-14`) — the TB-1 claim
> holds. Cross-tenant object **write** isolation holds: every key is `${locId}/…`-prefixed
> server-side; the only non-tenant-prefixed keys are `entry-photos/${uuid}` (random) — no
> cross-tenant overwrite path exists. `aws-sign-v4` is header-form only — CONFIRMED (see H1).

---

## CRIT

### C1 · B-FAIL / B-SCALE / B-ANTIPATTERN — In-process decode of *unauthenticated* bytes with the decompression-bomb cap unproven-by-the-spike; sharp's silent 268 MP default is NOT replicated, and topology-C makes one anonymous upload a whole-API OOM

**Claim attacked:** proposal §3/§4 "pure-Rust `image` … stays in-process in the `api` image" + §4
"Non-negotiable … a decompression-bomb bound … via `image`'s `Limits` API" (S4-T6, Q3 crit-6), *as
settled by the spike*.

**The break:**
- The current Node path is bomb-guarded **by default** without anyone noticing: `sharp`'s
  `limitInputPixels` defaults to `0x3FFF × 0x3FFF ≈ 268 MP`; oversized inputs are rejected before
  full raster alloc. The Rust `image` crate applies **no** such default on the entry point the spike
  actually used — `image::load_from_memory_with_format(input_png, ImageFormat::Png)`
  (`attempt-b-pure-rust/src/main.rs:68`) has **no `.limits(...)`**, no `ImageReader`, no dimension
  cap. So the naive port is **strictly more bomb-vulnerable than the code it replaces** — a parity
  *regression* on a safety property, on a route reachable by **anyone on the internet**
  (`POST /api/public/entry-photo`, `spa-proxy.ts:268`).
- **Number:** a max-dimension PNG `65535×65535` of a flat colour compresses to a few hundred KB — well
  under the 8 MB `fileSize` cap (`spa-proxy.ts:271`) and the `image/*` header check — and decodes to
  `65535² × 4 = 17.2 GB` RGBA. Even a modest `20000×20000` (400 MP, > sharp's 268 MP default →
  **sharp rejects it today**) is `1.6 GB` RGBA — instant OOM on a Fly machine sized 256 MB–1 GB.
- **Blast radius = the whole monolith.** The packet *recommends* topology **(C): transcode in-process
  in the single scratch `api`** (proposal §3, Q1). Decode is therefore in the same process/cgroup as
  `orders`, `menu`, WS — so one anonymous bomb OOM-kills the **entire API**, not a media sidecar. The
  packet's own §2 "single-digit uploads/sec" back-of-envelope is about *throughput* and does not
  price this: the DoS is **1 request**, not a rate.

**Why the "settled by evidence" framing fails here:** the spike is cited as having *removed the
imaging driver for the split* (proposal §4, §3), but the spike **never exercised a `Limits` cap, never
decoded an oversized input, and used the un-capped `load_from_memory_with_format` entry point**. The
one machine result that closes Q3 (76 ms / 36.7 MB, `main.rs:68-80`) was measured on a benign
`2000×1500` self-generated fixture. The safety-critical axis of the in-process decision — *can the
in-process decoder be forced to allocate gigabytes from a small untrusted file* — has **zero evidence
behind it**, and whether `image` 0.25.10's `Limits` are honoured by the `zune-jpeg`/webp decoders at
all is unverified (historically decoder-dependent). The DoD line (§10 "oversized-**dimension** fixture
rejected pre-alloc") tests *declared dimensions*, which is not the same as a compressed file that only
*decodes* large.

**Invariant violated:** "no new harm surface / no parity regression on a security property" (the port
must carry current safety *visibly*); B-FAIL "a timeout/oversized input must fail-safe, not cascade";
B-ANTIPATTERN "don't ignore the back-of-envelope on the actual attack (1 request, not QPS)".

---

## HIGH

### H1 · B-SEC / B-ANTIPATTERN — Q2(b) "hand-roll SigV4 query-presign … ~1 canonical-request function" materially understates the crypto surface: `aws-sign-v4` 0.3.0 cannot presign at all, and the reusable core it *does* have is the wrong shape

**Claim attacked:** proposal §5(b) / open-questions Q2(b): "extend the `aws-sign-v4` approach to the
query variant, ~1 canonical-request function." Task asks directly: *can reqwest+aws-sign-v4 produce a
query-string presigned PUT?*

**Evidence (crate source, `aws-sign-v4-0.3.0/src/lib.rs`):**
- The only output method is `AwsSign::sign()` → returns an **`AWS4-HMAC-SHA256 Credential=…,
  SignedHeaders=…, Signature=…` Authorization header string** (`lib.rs:133-150`). There is **no
  presign / query API**.
- `canonical_request()` is hardwired to header-form: it emits the payload hash as
  `digest(self.body)` (`lib.rs:130`) — a presign requires the literal `UNSIGNED-PAYLOAD` — and it
  builds the canonical query string **only from the URL's existing `query_pairs()`**
  (`lib.rs:127,174-181`) with **no hook to inject** the five `X-Amz-Algorithm/Credential/Date/
  Expires/SignedHeaders` params that a presign must fold into the *signed* canonical query string.
  So `AwsSign` is structurally unusable for presign; you cannot "extend" it.
- The crate's own tests cover **only** header-form canonical requests (`lib.rs:232-291`) — **zero**
  presign vectors. Option (b) therefore writes the security-critical canonicalization (exactly *what
  is in the signed set*) from scratch, shielded by no library test.

**The break the understatement enables:** if the operator green-lights (b) reading "≈1 function," the
real deliverable is a bespoke SigV4-query canonicalizer. The packet's own S4-T3 names the hazard (a
loose canonical request = a URL wider than {one key, PUT, ≤300 s}: e.g. omitting `Content-Type` from
`X-Amz-SignedHeaders` lets a holder PUT *any* content-type under the key; omitting/expanding
`X-Amz-Expires` widens the window) — but prices the *work* as trivial. Mispricing a crypto red-line is
how a "belt-and-suspenders" item ships thin. (`getSignedUrl(PutObjectCommand,{expiresIn:300})` in the
old code — `product-media.ts:167` — signs **only** `{bucket,key,method,ContentType}`; it does **not**
sign `Content-Length`, so even the *parity* baseline permits an arbitrary-size body under a
size-validated key — the hand-roll must at minimum not regress even that.)

**Invariant violated:** B-SEC "crypto surface honestly priced; signed-set enumerated"; B-ANTIPATTERN
"no false 'it's basically free' on a red-line." Note this only bites if (b) is chosen; recommendation
(d)→(a) avoids it — the finding is that the packet's framing makes (b) look cheaper than it is.

### H2 · B-CONSIST / B-ANTIPATTERN — The "DECIDED BY SPIKE" crate-parity rests on a **null oracle**: the render assertion is a URL regex, the spike never compared a pixel to sharp, and never decoded a JPEG

**Claim attacked:** proposal §4 header "Image processing stack — DECIDED BY SPIKE … `fit:inside` math
matches sharp **bit-for-bit**"; spike "Lossy quality parity confirmed"; DoD §10 "image-transcode
parity … within the Q3 perceptual tolerance."

**The break:**
- **The oracle checks nothing about the image.** `flow-ui-images.spec.ts:58` asserts only
  `body.imageUrl` matches `/^https:\/\/\S+\.webp$/` and `imageKey` ends `.webp` (`:59`). It never
  checks dimensions, never decodes the bytes, never compares to a baseline. "Within a perceptual
  tolerance the render assertion accepts" (Q3 crit-1, the *gating* criterion) is satisfied by **any
  string ending in `.webp`**. "Q3 perceptual tolerance" is **never quantified anywhere** in the
  packet or DoD.
- **The spike never measured parity against sharp.** `run_profile` (`main.rs:58-95`) decodes a
  **self-generated PNG**, resizes, encodes, and reports **size + dimensions + timing** — it never
  loads sharp's output and diffs it (no SSIM/PSNR). "bit-for-bit" in §4 refers to the *box-fit
  dimension math* (`out_w/out_h`), **not** pixel values. The resampler differs (`image` Lanczos3
  `main.rs:74` vs libvips' Lanczos3) so pixel output **will** differ; how much is unmeasured.
- **The dominant real input class was never decoded.** The spike fed a **PNG only**
  (`ImageFormat::Png`, `main.rs:68`). The product-image and entry-photo routes accept `image/*`
  (`spa-proxy.ts:275`) — overwhelmingly phone **JPEG**. `image`'s JPEG decoder (`zune-jpeg`) and its
  CMYK/progressive/ICC handling diverge from libvips; **not one JPEG was decoded** in the evidence
  that "settled" the crate.

**Invariant violated:** B-ANTIPATTERN "no DoD/verification — a decision presented as evidence-backed
whose evidence does not test the claimed property"; B-CONSIST "read-after-write parity of the served
artifact is asserted, not assumed."

### H3 · B-DATA / B-CONSIST — EXIF orientation: the spike's benchmarked decode path **cannot** read orientation, and the DoD's single rotated fixture won't catch the mirrored variants (2/4/5/7) — the exact silent-wrong-output class the packet flags

**Claim attacked:** proposal §4 / spike MUST-CARRY #1 / Q-EXIF-ORIENT: "explicitly
`decoder.orientation()` → `apply_orientation()` on **all three** handlers"; DoD §10 "a rotated-EXIF
fixture → an upright-pixel-oriented output."

**The break:**
- The spike (the only running evidence) uses `image::load_from_memory_with_format(...)`
  (`main.rs:68`), which returns a `DynamicImage` with **no decoder handle**. `ImageDecoder::
  orientation()` requires the lower-level `ImageReader`/decoder entry point. So the port must decode
  via a **different API than the one benchmarked** — meaning the 76 ms/36.7 MB/dimension-parity
  numbers do **not** describe the code that will actually ship (which also has to interleave the
  `Limits` from C1 on that same un-benchmarked path).
- **One fixture ≠ the class.** EXIF orientation has 8 values; 3,6,8 are pure rotations, **2,4,5,7 are
  mirrored (flip) variants** that `sharp.rotate()` handles today. A single "rotated-EXIF fixture"
  (§10) is almost certainly value 6 (phone portrait). An `apply_orientation` that handles rotations
  but drops the mirror bit passes that fixture and **silently ships left-right-flipped** logos/menu
  photos/doorway photos — the same "wrong output that still passes a some-webp-rendered assertion"
  the packet warned about, re-introduced *by the test being as narrow as the bug*.
- Compounding: the `webp` crate's `Encoder::from_image` (`main.rs:78`) accepts only RGB8/RGBA8; the
  spike's `.expect(...)` **panics** on `Luma16`/`Rgb16`/CMYK-decoded inputs (16-bit PNG, some JPEGs)
  that sharp coerces silently — another all-three-profiles divergence untested by the single PNG.

**Invariant violated:** B-DATA/B-CONSIST "silent-wrong-output must have a test that actually covers the
failure mode"; the DoD under-covers exactly the named silent class.

---

## MED

### M1 · B-DATA / B-CONSIST — Theme-logo is a **fixed key** served `immutable` for 1 year: a logo re-upload never busts the cache — the packet's own S4-T9 applies to the logo but it is carried verbatim, unflagged

**Evidence:** logo key is fixed: `locations/${locationId}/logo.webp` (`themes.ts:132`), **not**
content-addressed. It is served through `/images/*` with
`Cache-Control: public, max-age=31536000, immutable` (`spa-proxy.ts:171`, and `getImageUrl` routes it
there when `R2_PUBLIC_URL` is unset, `image-url.ts:20-22`). **Break:** an owner replaces their logo →
same key → browsers/Cloudflare keep serving the up-to-**one-year** stale copy (`immutable` tells the
CDN not even to revalidate). The packet frames S4-T9 ("sticky bad object … reusing a fixed
non-content-addressed key so a re-upload can't bust the cache") and its mitigation (content-hash
key) around **product-image only** (§7, Q-SHA-PROCESSED); it lists the logo under Q-STRIP-META/
Q-GUC-LOGO and **carries the fixed key verbatim without noting it is the exact S4-T9 trigger**. The
port is the moment this is decided and the packet doesn't surface it.

**Invariant violated:** B-DATA "cache/URL invalidation on mutable content"; the packet's own threat
row is not applied to an asset it clearly covers (M2 in its asset table).

### M2 · B-CONSIST / B-DATA — The 150 MB budget is bypassable beyond what S4-T10/Q7 admit: **confirm performs no budget check at all**, and presign has **no reservation** (TOCTOU) against a **client-declared** `bytes`

**Evidence:** budget is enforced **only** at presign, comparing `SUM(bytes)` to `LOCATION_BUDGET_BYTES`
using the **client-declared** `it.bytes` (`product-media.ts:124-131`, `sumIncomingBytes` over the
request body). **confirm** (`product-media.ts:178-265`) checks mime/prefix/frame-count but **never
calls `checkBudget`/`locationUsedBytes`** and writes the row with `Number(body.bytes||0)` (`:256`) —
also client-declared. **Break:** (a) declare `bytes:1` at presign → PUT the real 8 MB object → confirm
(no budget check) writes a row; the budget is defeated with one call. (b) No reservation between
presign and confirm → N concurrent presigns each read `used` below the cap and all pass (classic
TOCTOU) → post-confirm the location is over budget. The packet's S4-T10 concedes "`bytes` is
client-declared … budget is advisory," and Q7 only debates re-hashing for the *key name* — neither
states that **confirm does zero budget enforcement** or that presign has **no reservation**. Bounded
cost (storage accounting, not tenant-crossing) → MED, but the packet under-describes the hole.

**Invariant violated:** B-CONSIST "server does not trust client-declared amounts"; B-DATA "the one
bounded resource on the object plane is actually bounded."

### M3 · B-SCALE / B-FAIL — Q4(b)'s recommended "global rate cap" converts a per-IP abuse into **cross-tenant shared-fate**, and is specified with **no number**

**Evidence:** Q4(b)/§6 recommend "a **global** (not just per-IP) rate cap + a kill-switch." **Break:**
a single global token bucket sized low enough to blunt a botnet (C1's threat) is a **shared failure
domain** — one abuser saturating it now 429s *every* legitimate customer's optional entry-photo
upload across *all* tenants (`CheckoutPage.tsx:87` in the anonymous checkout funnel). The kill-switch
`ENTRY_PHOTO_ENABLED` is likewise all-tenants-or-nothing. The packet gives **no target rate**, so the
council can't tell whether the cap protects (blunts the botnet) or self-DoSes (blocks a lunch-rush of
concurrent checkouts). Back-of-envelope from §2 ("low-hundreds of orders/day system-wide," entry-photo
≤1 per opted-in order) says legit peak is a handful/minute — but that same smallness means a global
cap tight enough to matter against a botnet sits uncomfortably close to legit peak, with no stated
margin. An unquantified control is not a control.

**Invariant violated:** B-SCALE "a rate limit without a number is not a limit"; B-FAIL/OPS
"noisy-neighbor isolation — one tenant's abuse must not degrade another's availability."

### M4 · B-ANTIPATTERN / B-SCALE — Q2(c) server-proxy "buffered/**streamed**" is not achievable with the shipped signer: `aws-sign-v4` forces the **whole** ≤25 MB body into memory and re-hashes it per upload

**Evidence:** `AwsSign` takes `body: &'a [u8]` and computes `digest(self.body)` inside
`canonical_request` (`lib.rs:54,130`); the shipped `R2Storage` already hardcodes
`x-amz-content-sha256: EMPTY_PAYLOAD_SHA256` for its empty-body GETs (`storage.rs:227,245-247`).
**Break:** to PUT via this header-signer you must present the full body as a slice **and** SHA-256 it
whole (there is no `UNSIGNED-PAYLOAD` path — the crate always hashes the body, and no chunked
streaming signer). So option (c)'s stated cost "≤25 MB clip **buffered/streamed**" (§5(c), Q2(c)) is
wrong on "streamed": every proxied upload buffers the entire clip in the scratch `api`'s RSS and
runs a full-object SHA-256 in-process — meaningfully more memory + CPU than the packet advertises,
and it stacks on the same in-process budget as C1's decoder. Doesn't kill option (c) but the council
is pricing it with a false "streamed."

**Invariant violated:** B-ANTIPATTERN "honest cost accounting for the recommended dark-horse."

---

## LOW

### L1 · B-CONSIST — Entry-photo key is an **unbound, client-held opaque string** that crosses into S5 with no server record; the "only revealed to the assigned courier" control depends entirely on S5 not trusting a client-supplied key

`POST /api/public/entry-photo` writes **no DB row** — it returns `{key,url}` and the anonymous client
echoes `entryPhotoKey` back at order-create (`spa-proxy.ts:286,292`; `CheckoutPage.tsx:78,88`). Nothing
server-side binds key→order at S4. So S4's stated control ("unguessable, only revealed to the courier")
is only as good as **S5** rejecting an arbitrary client-supplied key. Since `/images/*` serves every
object public-by-key (TB-5), a client could attach *any* `/images/*` key as "their" entry photo. Out
of S4 scope to *fix*, but S4 hands S5 an unauthenticated opaque string with no provenance — flag the
seam so S5's council doesn't assume S4 validated it.

### L2 · B-CONSIST — product_media confirm `sort_order` is a read-modify-write race

confirm reads `COALESCE(MAX(sort_order),-1)+1` then INSERTs in the same `withTenant` txn but with no
unique constraint / lock (`product-media.ts:244-260`). Two concurrent confirms for one product →
duplicate `sort_order`. Owner-only, low-frequency (§2) → LOW; note as CARRY.

### L3 · B-SEC / B-SCALE — Q9 brand-extractor rider decodes SSRF-fetched **remote** logos and inherits the C1 gap, unmentioned

`extractLogoColor` (`brand-extractor.ts:306-313`) decodes bytes fetched from a **remote logo URL**
(`extractFromHtml(..., u.toString())`, `:295`) — i.e. untrusted, attacker-influenceable input feeding
`POST /api/owner/brand/generate`. If it rides the shared `ImageProcessor` (Q9(a)) it needs the same
bomb bound as C1, but the packet notes only the *crate-sharing* dependency, not that this rider is a
second untrusted-decode surface. LOW (owner-authenticated, not anonymous) but on the same missing cap.

### L4 · B-SEC — SigV4 canonical-URI path is not re-encoded by `aws-sign-v4` (latent, currently benign)

`canonical_request` uses `self.url.path()` raw (`lib.rs:121`) with no per-segment URI-encoding; keys
today are `[a-z0-9/.-]` (hash/uuid/ext) so signatures verify, but any future key containing a space or
non-ASCII byte would produce a canonical request R2 disagrees with → signature-mismatch upload
failures. Fail-safe (loud), so LOW; record so a key-scheme change doesn't silently break signing.

---

## Regression check vs the S3 breaker bar
The S3 breaker run found 2 CRIT / 4 HIGH, all confirmed. This surface's severity concentrates
differently: **1 CRIT** (in-process unauthenticated decode + unproven cap — genuinely severe because
of the topology-C monolith blast radius on an anon route) and **3 HIGH** (all *evidence-quality*
failures — the spike/oracle assert less than the packet claims). The GUC/`with_user` items the packet
foregrounds (Q5/REV-2/REV-10) are **correctly dispositioned** and are *not* re-raised — the real
breaks are on the axes the packet treats as settled: the bomb cap ("non-negotiable" but spike-untested),
the crate parity ("decided by spike" against a null oracle), and orientation (flagged but under-tested
and path-incompatible with the benchmark).
