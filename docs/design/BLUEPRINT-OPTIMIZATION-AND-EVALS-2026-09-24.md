# Optimization audit and an evals system for every indicator

**Date:** 2026-09-24. **HEAD read:** `5abeb0ac` (2026-09-24 17:50; the tree moved from `017532fc` to `5abeb0ac`
while this was being written — other lanes commit here — so every `file:line` is as read from the working tree
during the afternoon). **Method:** read-only. No build was run; the wasm numbers are from the artifact
`workers/api/build/index_bg.wasm` that `worker-build --release` left on 2026-09-22 22:20 and from
`target/wasm32-unknown-unknown/release/dowiz_api_worker.wasm` (2026-09-23 18:18). Live numbers are GET-only
probes of `https://sushi-durres.dowiz.org` and `https://dowiz.org` with `curl -w` from this box (Android/proot,
so the DNS+TLS part of every timing is 100–420 ms of *this* network and is subtracted where a server number is
claimed). **measured** = a command was run here and its output is quoted; **derived** = arithmetic over measured
inputs; **hypothesis** = neither; **UNVERIFIED** = could not be checked from this box.

**Read first, not repeated here:** `BLUEPRINT-LAUNCH-GAPS-2026-09-24.md` §7 (G7: what binds at 10/50/200 venues —
the two serial crons, the 100-domain rule, the F18 alarm design), `BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md`
and the memory notes `dowiz-hub-unit-costs` ($5.93/venue/month; requests, not duration, bind) and
`dowiz-hub-seven-phases-2026-09-21` (28 KB → ≈1.2 KB per delivered order on the wire; the object folds its own
log). `BLUEPRINT-EVALS-TRAFFIC-MEMORY-LATENCY-2026-09-21.md` designed four budgets and `e2e/evals/traffic.mjs`;
that runner is **still unrun** (`/tmp/dowiz-evals` does not exist, measured) three days later. This document does
not re-argue those; it measures what they left unmeasured and then builds the catalogue and harness around all of it.

**The five largest things this measured, in one paragraph:** the storefront's photos are the whole byte story
(every thumbnail is the full photo, mean **124 KB**, `imageUrlSmall` is null on **165 of 165** products, so a full
scroll is ≈ **9.5 MB** against ≈ **125 KB** for everything else); every static asset ships with
`cache-control: public, max-age=0, must-revalidate`, so a repeat visit revalidates all **27** boot files; the
Worker bundle is **2.43 MB** (905 KB gzip / 693 KB brotli) where its own `Cargo.toml` comment still says
"~447 KB" — a 5.4× drift in nine days, 14 % of it a `name` section the runtime never executes; every request on
every venue resolves through **one** platform Durable Object and pulls whole images (registry, and for owners the
sessions and identity tables) across the object boundary into the Worker; and `/manifest.webmanifest` costs a whole
**catalogue** image read (**0.84 s** TTFB) to answer 296 bytes.

---

## Part A — the optimization audit

### A.1 The Worker: bundle, sections, crates (measured)

| Artifact | Bytes | Note |
|---|---|---|
| `build/index_bg.wasm` (what wrangler uploads) | **2,426,605** | 2026-09-22 22:20 build |
| — gzip -9 | 904,804 | what a `wrangler deploy` upload is judged by (compressed) |
| — brotli -q11 | 693,308 | |
| `build/index.js` shim | 31,958 | |
| `target/.../release/dowiz_api_worker.wasm` (pre-`wasm-opt`) | 4,894,962 | 2026-09-23 18:18; newer than the bundle |
| `Cargo.toml:70` comment | "~447 KB wasm + 22 KB JS" | written at `cb95cdb2`, 2026-09-15 (measured via `git log -S`) |

**Sections of the bundle** (measured with a 40-line LEB128 walker, because the box's `wasm-objdump` is too old
to parse the reference-types the build uses):

| Section | Bytes | Share |
|---|---|---|
| `code` | 1,824,804 | 75.2 % |
| `custom:name` | **348,560** | **14.4 %** — symbol names; kept because `strip = false` is forced (`Cargo.toml:60–69`: `strip=true` breaks wasm-bindgen's catch wrappers) |
| `data` | 234,831 | 9.7 % |
| everything else | 18,410 | 0.7 % |

The pre-`wasm-opt` file carries `__wasm_bindgen_unstable` 315 KB, `.debug_*` ≈ 340 KB and an `export` section of
137 KB, none of which survive into the bundle — so `worker-build` did run `wasm-opt` (it downloads binaryen into
`build/.tmp/worker-build`; there is no `wasm-opt` on PATH here). The `name` section survives `wasm-opt` because
`-Oz` keeps it unless told otherwise.

**Code bytes by crate** (measured: name-section prefix of each function body, 3,703 functions, 1,819,648 code
bytes):

| Crate | Bytes | Share | Functions |
|---|---|---|---|
| `dowiz_api_worker` | 958,184 | **52.7 %** | 341 |
| `core` | 252,318 | 13.9 % | 1,371 |
| unnamed (`?`) | 185,772 | 10.2 % | 786 |
| `js_sys` | 104,201 | 5.7 % | 80 |
| `worker` | 48,017 | 2.6 % | 172 |
| `dowiz_hub` | 45,712 | 2.5 % | 140 |
| `serde_json` | 33,227 | 1.8 % | 83 |
| `url` (+ `icu_normalizer` 4.6 KB it drags in) | 30,355 | 1.7 % | 41 |
| `dowiz_core` | 25,090 | 1.4 % | 44 |
| `alloc` | 24,253 | 1.3 % | 201 |
| `bebop_store` | 15,103 | 0.8 % | 34 |
| `argon2` + `password_hash` + `blake2` | 18,806 | 1.0 % | 23 |

The kernel (`dowiz_kernel`) does not appear by name: LTO inlined it into its callers (its 1.5 MB standalone
`.wasm` is the `deps/` artifact, not a runtime cost). **The bundle is the handlers, not the libraries.** The
twelve largest functions (measured): two `js_sys::futures::future_to_promise` closures at 49,634 and 25,585 B
(the wasm-bindgen glue that owns every exported async future), `storefront::place` 35,736, `bootstrap::seed`
23,273, `owner::update_product` 20,983, `platform::create_hub` 19,552, `storefront::menu` 16,000,
`cloud::push_place` 15,937, `route` 15,639, `auth::verify` 15,546, `catalogue::import::import_menu` 14,362,
`integrations::check` 14,183. These are `async fn` state machines whose bodies hold inlined `serde_json::json!`
builders and every `.await` point's live set — the well-known shape where the generated future is several times
the size of the same logic written as a sync function called from a thin async wrapper.

**Startup.** Cloudflare compiles the module at first load and enforces a startup CPU limit; a 2.4 MB module is
well inside the 10 MB (paid) / 3 MB (free, compressed 1 MB) *size* limits but the **startup time is UNMEASURED
from this box** — no token here reads the dashboard's "startup time". The only proxy measured: the cold `/`
(served by `serve_root`, `lib.rs:184`, which only touches `ASSETS`) took **0.530 s** total with **0.422 s**
in DNS+TLS from here, i.e. ≈ **110 ms** at the edge; a subsequent asset hit was ≈ 50 ms server-side. A
DO-backed read (`menu?fresh=1`, bypassing the edge cache) measured **260 / 135 / 106 ms** server-side over three
consecutive runs — the first is the object waking, the next two are its memory. Edge HITs on the same route:
≈ 50 ms. So the user-visible latency structure is: edge ≈ 50 ms, Worker + object memory ≈ 105–135 ms, object
cold ≈ 260 ms, plus whatever the phone's own radio adds.

### A.2 The Durable Object and the images: per-request reads and writes (measured from code)

- **Writes are already chunk-diffed.** `hubdo.rs:44` `CHUNK = 96 KiB`; `put_image` (`:1043–1067`) writes only
  the chunks `changed_chunks` reports plus one `meta` value, in one transaction. An append to a trimmed log is
  chunk 0 (superblocks + PartTab, `bebop-parttab-21-cells`) + the tail chunk + meta = **3 values**. Nothing here
  to win; this was phase 1.
- **Reads are one `get_multiple` then memory** (`hubdo.rs:359–410`): all chunks in one storage call, then the
  `mem` map for the object's life. A hibernated object pays the storage read once per wake.
- **Folds happen in the object** for the two hot console reads: `owner::orders` and `owner::dashboard` call
  `hubstore::orders`/`venue_record` (`owner.rs:262, 612–632`), which are `/fold/*` commands memoised per generation
  (`hubdo.rs:412–433`). The whole log crosses into the Worker on **five** remaining paths (measured: callers of
  `hubstore::load`): `services/operations/waste.rs`, `services/engagement/assist.rs`,
  `services/customers/handlers.rs`, `exceptions.rs`, `cloud.rs` (the nightly). None is per-poll. Good.
- **`/manifest.webmanifest` reads the whole catalogue** (`storefront.rs:665–676`: `Place::of_slug` then
  `hubstore::load_catalog`) to answer name, colours and icon. Live: **0.843 s** TTFB, 296 bytes, `max-age=300`.
  The catalogue image was 538 KB on this venue on 2026-09-21 (`dowiz-hub-seven-phases`, the un-done Kv packing);
  a whole-image DO→Worker transfer for a JSON that fits in a settings read.
- **The platform object is on every request's path.** `Place::of_slug` (`hubstore.rs:167–172`) calls
  `identity_store::registry` → `platform_store::load` → `stub.fetch("https://hub/img/registry")`
  (`platform_store.rs:159–170`) and `Table::load`s the **whole registry image in the Worker**, per request. An
  authenticated owner request additionally loads `sessions` (`auth.rs:553, 598, 637`) and `identity`
  (`auth.rs:517`) the same way — ceilings 4 MiB and 2 MiB (`platform_store.rs:60–64`). So one owner request =
  **up to three platform-object requests carrying whole images, plus the venue object**, and every venue's every
  request serialises through the single `__platform` object. The launch-gaps table lists the registry's
  *ceiling* (F21) but not this *request concentration*; at 200 venues × 3,900 req/day (the cost blueprint's
  modelled rate) that is ≈ 9 platform-object requests per second on average, each moving an image whose real
  size is **UNVERIFIED** (no platform health route exists; `GET /api/platform/health` answered 404, measured).
- **Crons** (`lib.rs:544–563`): the minute cron runs three sweeps. `outbox::sweep` = one registry read + one
  `waiting()` object read per venue (`rails.rs:151–183`); `ebills::poll::sweep` = one registry read + one
  `ebills/tick` command per venue *before* the `enabled` check (`poll.rs:33–48`); `fiscal::rail::sweep` returns
  at `SEND_ENABLED = false` (`fiscal/mod.rs:40`, `rail.rs:59`) and costs nothing today. Net: **2 object requests
  per venue per minute = 2,880/venue/day** (the G7 number stands) **plus 2 platform reads per minute
  regardless of venue count** = 2,880/day of whole-registry transfers into the Worker. The nightly
  (`cloud.rs`, per venue): `load_settings`, idempotency sweep, errlog prune, one combined hub+settings load, witness,
  rotate, backup — ≈ 7 object requests and one whole log read per venue per night. The fix for the fan-out is F18
  (alarms) in the launch-gaps doc and is not repeated here.

### A.3 The surfaces: bytes, requests, polling (measured on disk and live)

**What is served** (`workers/api/public`, 18.0 MB in 328 files, measured): platform video 13.3 MB (3 promo MP4s,
one plays), `lib/` 1.85 MB (of which `map/maplibre-gl.js` 954 KB and 10 font files 454 KB), `kit/` **746 KB in
180 files — an older storefront** (`<title>dowiz · food delivery`, last touched `b7019114` 2026-09-22) that no
surface links (only `e2e/kit-regression` uses it), `admin/` 394 KB, `store/` 330 KB, `room/` 144 KB,
`courier/` 123 KB.

**Boot payload per surface** (measured by walking each `index.html`'s stylesheets and the static `import` graph
from its entry; gzip -9 per file summed; live brotli where probed):

| Surface | Files at boot | Raw | gzip | Largest pieces | Live (br) |
|---|---|---|---|---|---|
| Storefront `/` | 27 (22 JS, 4 CSS, 1 HTML) | 346,359 | **113,377** | `store.css` 90,699 · `store/i18n.js` 29,815 · `booking.js` 21,079 · `state.js` 19,225 | `/` 2,413 · `store.css` 21,797 · `icons.css` 7,495 · `i18n.js` 11,012 · `app.js` 5,170 |
| Owner console `/admin/` | 11 (6 JS, 4 CSS) + fonts | 217,225 | 64,746 | `admin/i18n.js` 82,691 (665 keys × 3) · `app.js` 17,278 · `core.js` 12,645 | + Unbounded/Manrope: latin 50,904 + 24,836; **cyrillic 31,424 + 14,500 `<link rel=preload>`ed for every language** (`admin/index.html`) |
| Waiter `/room/` | 22 (18 JS, 3 CSS) | 233,065 | 85,667 | **`admin/i18n.js` 82,691** (imported for `st` status words only, `room/i18n.js:6,142`) · `room/i18n.js` 19,487 · `outbox.js` 16,316 | not probed |
| Courier `/courier/` | 13 (8 JS, 4 CSS) | 223,064 | 74,024 | `courier/app.js` 62,800 (one file) · `guide.js` 18,131 | not probed |
| Landing `dowiz.org/` | 5 CSS/JS + 4 vendor + fonts + poster | ≈ 300 KB | ≈ 80 KB | `gsap` 72,435 + `ScrollTrigger` 44,157 + `SplitText` 7,247 + `lenis` 16,768 = **140 KB raw of animation vendor JS**; `store-dubin-a/b.jpg` 376 + 418 KB | `/` 7,237 · `landing.js` 14,833 · `landing.css` 6,822 · `poster-en.jpg` 47,947 |

The storefront uses **system fonts** (no `@font-face` in `tokens.css`/`store.css`, measured) — the six font
files are a console/landing cost only, and there they are correctly split by `unicode-range` (`admin.css:14–31`);
only the *preload* hints ignore the language.

**Photos are the storefront** (measured live): the menu JSON is 88,331 B raw / **11,978 B brotli**, edge-cached
`max-age=14400, stale-while-revalidate=300` (`cf-cache-status: HIT`, `age: 21`). It lists **190 products in 21
categories, 165 with `imageUrl`, 77 unique photos, and `imageUrlSmall` non-null on 0**. `menu.js:106` already
emits `srcset="… 480w, … 1200w" sizes="(max-width: 640px) 50vw, 400px"` *when* a small variant exists, and
`lib/shrink.js:68` `shrinkPair` already produces one at upload — so every photo uploaded before `shrinkPair`
landed (all of them on this venue) is drawn at ≈ 200 CSS px from a ≈ 1600 px JPEG. Ten sampled photos:
44–166 KB, **mean 124,068 B**; all `cache-control: public, max-age=31536000, immutable`, edge HIT. Derived:
first viewport ≈ 4–6 lazy-loaded photos ≈ **500–750 KB**; a full scroll ≈ 77 × 124 KB ≈ **9.5 MB** — about
**80× the code and 800× the menu JSON**. A 480 px q0.72 variant of the same photos is ≈ 12–20 KB
(hypothesis from the encoder settings, not measured on these files).

**Caching of the shell** (measured live): every static asset answers
`cache-control: public, max-age=0, must-revalidate` with a weak ETag (`/app.js`: `W/"a0f1…"`). There is no
`Cache-Control` line in `public/_headers` (measured; it sets only HSTS, CSP, referrer, frame, permissions), so this
is Workers Assets' default. Consequence: a **repeat** storefront open makes **27 conditional requests** (all 304,
but each a round trip the SW's network-first `fetch` also waits for, `sw.js:fetch`). The SW shell cache only
serves when the network *fails*, not when it is slow. File names are not fingerprinted, so `immutable` cannot be
switched on without a versioning step.

**Requests per screen** (measured from the JS):

| Screen | At open | Then |
|---|---|---|
| Storefront menu | 1 `GET …/menu?locale=` (`app.js:70`) + 1 `consent/wordings` (`consent.js:22`, 394 B) + photos | none until an order; `track.js:292` polls `/api/order/:id` every **12 s** in `READY`/`IN_DELIVERY`, else **30 s**, and skips the poll while the `order:<id>` socket has spoken within 90 s (`live.js:24, 108`) |
| Owner console | `loadVenue` = **3** `menu?locale=…&fresh=1` reads (one per language, `app.js:199–206`, edge cache bypassed by design), + `dashboard` + `couriers` + `orders?since=` + the socket = **7** | `POLL_MS` 15 s with live orders, 60 s idle (`core.js:22–25`) — but `poll()` runs `loadOrders` only when `socket.due()` (`app.js:294`), and `lastHeard` is refreshed by **every** frame including the 25 s pongs (`live.js:69`), so with a healthy socket the poll never fires. Side effect: `loadStats()` sits inside that same branch (`app.js:302`), so the dashboard totals stop refreshing while the socket is healthy and no order moves; `refreshNow()` reloads orders only. A correctness observation for Part B, not a cost. |
| Waiter | 1–2 (`loadRoom`/`loadFloor`) | unconditional **20 s** poll while visible (`room/app.js:27, 244`); no socket = **180 requests/hour/device** |
| Courier | `load()` = tasks (+ shift/earnings/history per panel) + socket | 12 s on shift / 60 s off (`courier/app.js:1249`), socket-gated like the console; GPS goes over the socket as a frame, throttled to 10 s / 20 m (`:386`) |
| Landing | HTML + 4 vendor + 2 own + 3 fonts + poster + 1 video | none |

**i18n completeness** (measured by importing each surface's `T` in node and flattening keys): storefront 302
keys in the union, of which 42 are `qtyWords.*` (number words that are *meant* to differ per language); outside
`qtyWords` **sq = en = uk = 260, 0 missing**. Console 665/665/665, waiter 157/157/157, courier 131/131/131 — all
0 missing. The i18n payload is complete; its only cost is size (`admin/i18n.js` 82.7 KB raw is the single
biggest JS file on two surfaces).

**CSP:** one strict policy in `_headers` (no `unsafe-inline`), applied to every asset; violations are not reported
anywhere (no `report-to`), so today a CSP break is found by a person (`dowiz-csp-blocked-every-stylesheet`).

### A.4 The improvements, ranked by measured gain

Each row: what, expected gain (with the number it comes from), risk, and **the zero-behaviour-change proof** —
the command or test that shows nothing a user sees is different.

| # | Improvement | Expected gain | Risk | Proof of zero behaviour change |
|---|---|---|---|---|
| **O1** | **Serve the small photo variant.** A one-off owner-console action ("re-shrink photos") that runs `shrinkPair` over every product with `imageUrlSmall == null` and writes the 480 px key; `menu.js:106` then uses its existing `srcset`. | Full scroll 9.5 MB → ≈ 1.5 MB; first viewport 500–750 KB → ≈ 100 KB (derived from mean 124 KB → ≈ 15 KB). The largest single win on the platform by a factor of ten. | Low: `imageUrl` is untouched, the dish sheet (`dish.js:96`) still draws the full photo. | `imageUrlSmall` non-null count = 165 on the live menu; visual diff of the dish sheet = 0; the storefront's byte budget in `traffic.mjs` falls and is re-baselined. |
| **O2** | **Fingerprint and cache the shell.** A deploy step that rewrites asset references to `/v/<content-hash>/…` (or `?v=`) from one manifest, which `sw.js`, `room/sw.js` and `courier/sw.js` derive their `SHELL` lists from; `_headers` then sets `Cache-Control: public, max-age=31536000, immutable` for `/v/*` and `/lib/font/*`, `/lib/vendor/*`. | Repeat open: 27 → ≈ 3 requests (HTML + API); on a 150 ms mobile RTT that is ≈ 0.4–0.8 s of revalidation removed from every return visit (derived: HTTP/2 parallelism assumed). | Medium: a wrong hash pins a stale file for a year. The three SW shell lists already broke once for exactly this reason (`sw.js` comment). | `curl -I` on each boot file shows the header and `cf-cache-status`; `e2e/visual` run; the SW shell lists are *generated* from the same manifest, so `node e2e/kit-regression/courier-cold.mjs` proves offline still boots. |
| **O3** | **Manifest from settings, not the catalogue.** `storefront::manifest` reads brand name/colours/icon from `load_settings` (or from the venue record the menu response already carries) instead of `load_catalog`. | 0.84 s → ≈ 0.15 s TTFB; one 538 KB DO→Worker transfer per manifest fetch removed (per install prompt / PWA check). | Nil. | `diff <(curl …/manifest.webmanifest) <(curl …)` byte-identical before/after. |
| **O4** | **Console boot: one catalogue read.** Replace the three `menu?locale=…&fresh=1` reads with one owner endpoint returning all three languages, or keep `fresh=1` only for the editing language. | 2 DO-backed reads (≈ 135–260 ms each, ≈ 88 KB raw / 12 KB br each) per console open; −2 Worker + −2 object requests per open. | Low. | `S.products[i].translations` equal under both paths (a `room/logic.test.mjs`-style unit test on the merge). |
| **O5** | **Preload fonts by language.** Emit the `<link rel=preload>` for the Cyrillic files only when `lang === 'uk'` (console and landing). | −46 KB per console open for `sq`/`en` owners (`unbounded-cyrillic` 31,424 + `manrope-cyrillic` 14,500). | Nil. | Font files requested per language, counted headlessly. |
| **O6** | **Waiter app stops importing the console's dictionary.** Move `st` (7 status words × 3 languages) into `lib/status-words.js`; `room/i18n.js:142` imports that. | −82.7 KB raw / ≈ −25 KB gz per waiter boot (`admin/i18n.js` is 35 % of the room's boot bytes). | Nil. | `statusWord(s)` equality test across all statuses and languages. |
| **O7** | **Strip the `name` section after wasm-bindgen.** Keep `strip = false` for the link (the catch-wrapper reason in `Cargo.toml:60–69` is about the *link* step); add a post-build `wasm-opt --strip-debug` (or `wasm-strip`) on `build/index_bg.wasm`, which removes only custom sections. Also correct the "~447 KB" comment. | −348,560 B raw (−14 %), ≈ −100 KB brotli; faster upload; smaller compile input. | Low; the failure mode the comment describes (missing externref table) is a link-time failure and would appear in `worker-build`, not after it. | Export list identical (the section walker in §A.1 lists them); `cargo test` unchanged; `wrangler deploy --dry-run` succeeds; panic-recovery proven by the existing catch-wrapper test if any, else by a deliberate `unreachable!()` route in preview. |
| **O8** | **Slim the handler state machines.** For the ten largest closures (§A.1): move the `json!` response shaping and validation into sync functions in `services/*` (the testability-seam rule already asks for this, `dowiz-worker-services-layout`), leaving the `async fn` as sequencing. | 15–25 % of the 958 KB (hypothesis: typical async-state-machine shrink when live-across-await sets fall; measure per function with the §A.1 walker before/after). | Medium: touches the hottest handlers (`place`, `menu`). | Route table byte-identical (`grep -o '"/api/[^"]*"' lib.rs | sort` before/after); 820 Worker tests; `e2e/walk`; the one-image/one-venue gates. |
| **O9** | **Platform lookups become object commands.** `Place::of_slug` asks the platform object `GET /lookup/slug/<x>` answered from its memory; session verification becomes `POST /session/verify`. The images stop crossing into the Worker per request. | Per request: registry (≤ 512 KiB) and, for owners, sessions (≤ 4 MiB) + identity (≤ 2 MiB) no longer transferred and re-`Table::load`ed; −20–60 ms per authenticated request (hypothesis; measure with the live probe before/after). Real image sizes UNVERIFIED — **measure first** (the `platform/health` route below). | Medium: auth path. | `e2e/tests` auth matrix (401/403/200 per role × route) identical; `tools/gates/one-venue.sh` still 0. |
| **O10** | **Landing images to AVIF/WebP at display size.** `store-dubin-a/b.jpg` 376 + 418 KB → ≈ 60–90 KB each at 1200 px; consider dropping `SplitText`/`lenis` if the scroll story survives without them (−24 KB raw). | −650 KB per landing visit (derived). | Nil. | `e2e/visual` diff on the landing at 3 widths. |
| **O11** | **Stop uploading `kit/`.** Move it under `e2e/kit-regression/fixture/` and serve it there for the two kit tests. | −746 KB / −180 files per deploy; no runtime change. | Nil. | The two kit-regression tests still pass from the new path. |
| **O12** | **Split `store.css` by screen.** The booking, tracking and sea rules (≈ 40 % of 90.7 KB, hypothesis) load with their dynamic module. | ≈ −8 KB br before first paint. | Low. | Visual diff; the SW shell list updated by O2's generator. |
| — | **F18 (per-venue alarms), F19 (wildcard route), F21 (registry capacity)** | see `BLUEPRINT-LAUNCH-GAPS-2026-09-24.md` §7.3 | | |

What is **not** proposed: a bundler for the surfaces (the module graph is 22 files and 113 KB gz; HTTP/2 makes
the count cheap once O2 removes the revalidation), a CDN for photos other than the KV+edge already in place, and
any change to the poll intervals — the socket already suppresses the polls where it is healthy, and the waiter's
unconditional 20 s poll is the one place a socket would pay (a candidate row, sized M, not urgent at one venue).

---

## Part B — an evals system for every indicator

### B.1 Principles

1. **Aggregate only; no scoring of participants.** `DECISIONS.md` OD-8 deleted `reputation.rs` and
   `tools/gates/no-scoring.sh` refuses any `(courier|customer|staff|…)_(score|rating|rank|tier)` identifier.
   Every product and reliability indicator below is a per-venue or per-platform aggregate over a window; nothing
   is keyed by a person, a phone, a courier id or a customer id, and the collectors never write such a key into a
   report. Where the truth lives in a per-order record (ETA error, PENDING→CONFIRMED time), the collector folds it
   into a distribution and discards the rows.
2. **A measurement that cannot fail is a dashboard.** Every indicator has a threshold or a ratchet; the run exits
   non-zero on a breach.
3. **Baselines like `tools/gates/*.baseline`:** one `key=value` file per indicator group, written on the first
   run, compared on every run, **lowered in any commit, raised only with a dated note** (the file-size gate's rule,
   `tools/gates/file-size.sh:38–60`).
4. **Three runs, the median, for anything with a clock in it** (the 09-21 blueprint §3). Counts and bytes are
   taken once — they are exact.
5. **The number carries its commit and the deployed version** (`head-may-not-reproduce-its-own-artifacts`).

### B.2 The catalogue

Columns: **Indicator · Source (file/route/command) · Collection · Threshold / regression rule · Shown where.**
`CI` = every push, no network; `nightly` = the scheduled run against production, GET-only plus the one QA order
`traffic.mjs` already places and cleans up; `live` = continuous, from the hub's own gauges.

#### Performance

| Indicator | Source | Collection | Rule | Shown |
|---|---|---|---|---|
| Wasm bundle bytes (raw, gzip, brotli) and per-section | `workers/api/build/index_bg.wasm`; the §A.1 walker as `tools/evals/wasm-sections.py` | CI after `worker-build --release` (via `bash bebop-lang/tools/slot.sh opt …` on this box) | ratchet: raw ≤ baseline; `name` section = 0 after O7 | report §wasm; `wasm.baseline` |
| Code bytes per crate, top-12 functions | same walker, name-section prefixes | CI | `dowiz_api_worker` share ≤ baseline; a function > 40 KB names itself | report |
| Worker startup time | Cloudflare dashboard "startup time" per version | **UNVERIFIED from this box**; manual, recorded into `docs/measurements/` with the version id | ≤ 200 ms | report (manual row) |
| Boot bytes per surface (raw, gz) and file count | `tools/evals/graph.mjs` (the import-graph walk in §A.3) over `public/*/index.html` | CI | ratchet per surface | report §surfaces; `surfaces.baseline` |
| Served tree bytes, dead assets | `find public -type f`, minus the union of every import graph, `SHELL` list and `<link>` | CI | dead bytes = 0 after O11 | report |
| i18n completeness per surface per language | `tools/evals/i18n.mjs` (the scratch script in §A.3, keys flattened, `qtyWords.*` excluded) | CI | missing = 0 for sq/en/uk | report §i18n |
| TTFB / total per boot file and per public API route (median of 3) | `curl -w` GET-only against `HOST` for the 14 URLs in §A.1/§A.3 (`tools/evals/live.mjs`) | nightly | server-side (ttfb − tls) ≤ baseline + 25 %; `cf-cache-status` = HIT on assets and the menu | report §live; `live.baseline` |
| Photo weight: mean bytes of a 10-photo sample; `imageUrlSmall` coverage | live menu JSON + 10 `/media` GETs | nightly | coverage = 100 % after O1; mean small ≤ 25 KB | report |
| API requests, wire bytes, p95, cells per delivered order | `e2e/evals/traffic.mjs` (exists, unrun) | nightly | its four budgets, re-baselined from the first run | report §order; `order.baseline` |
| DO requests / min / venue and Worker requests / day | Cloudflare GraphQL analytics (`workersInvocationsAdaptive`, `durableObjectsInvocationsAdaptive`) | **UNVERIFIED** — needs an `Account Analytics:Read` token, which no file on the box holds; until then **modelled** from the per-screen table in §A.3 and the cron shape (2/venue/min) | modelled vs measured gap reported; measured ≤ 1.2 × modelled once the token exists | report §requests |
| Image bytes written per order, per command | `/api/owner/health` `images[].usedPerMille` + `generation` before/after the QA order (what `traffic.mjs` already reads) | nightly | cells per order ≤ baseline (any increase fails) | report §order |
| Polling cadence per surface | constants grep (`POLL_MS`, `POLL_IDLE_MS`, `wait = S.onShift ? …`) | CI | each equals its documented value; a change is a dated note | report |

#### Correctness

| Indicator | Source | Collection | Rule | Shown |
|---|---|---|---|---|
| Conservation laws (13) | `e2e/gates/conservation.mjs` + `conservation.prove.mjs` | CI (prove) · nightly (live) | all 13 hold; the prove run must go RED when told to | report §laws |
| Recipe/stock conservation | `e2e/gates/recipes.mjs` + `.prove.mjs` | same | same | report |
| Gates green count / total | `tools/gates/*.sh`, `unreached.py` (20 scripts, 13 baselines) | CI | 20/20; a lowered baseline is recorded | report §gates |
| Test counts per module | `cargo test -- --list` per crate (dowiz-core 3,659 `#[test]` attrs, workers/api 820, dowiz-hub 452, kernel 241, bebop-store 35, measured by grep); `*.test.mjs` 154 files; `e2e/tests` 135 files | CI | count per module ≥ baseline (a deleted test is a dated note) | report §tests; `tests.baseline` |
| Mutation score | CI already runs `cargo mutants` on 5 kernel files (`ci.yml:125`) and **records no score anywhere** (grep: no baseline, no doc) | CI, parse `mutants.out/outcomes.json` | caught/(caught+missed) ≥ baseline; target 80 % on money, stock, tenancy files (`PLAN-HARDENING` F3) | report §mutation; `mutants.baseline` |
| Route table | `grep -o '"/api/[^"]*"' workers/api/src/lib.rs \| sort` (158 today) | CI | diff against the committed list is empty or dated | report |
| Fold identity | `Hub::chain_check()`, `/fold/generation` vs. `traffic.mjs`'s status per screen (law 1) | nightly | 0 disagreements | report |
| Console stats freshness | headless: open console, hold a healthy socket 2 min, compare `S.stats` timestamp | nightly | refreshed at least once per `STATS_EVERY × POLL_MS` — **RED today** by §A.3 | report |

#### Reliability

| Indicator | Source | Collection | Rule | Shown |
|---|---|---|---|---|
| Outbox: waiting, oldest age, abandoned/day | `/api/owner/health` `outbox.{waiting, oldestMs}`; `errors` records with `what = outbox.abandoned` (`rails.rs:171`) | live (owner token from `/root/.dowiz_owner`) | abandoned = 0/day; oldest ≤ 10 min | report §reliability; the console's health card |
| Fiscal / ebills backlog | `health.fiscal`, `health.ebills`, `rails` (`fiscal/rail.rs:14`) | live | backlog age ≤ 24 h; `failing` = 0 | same |
| Error log rate | `/api/owner/health` `errors` (last twenty) + `GET /api/platform/errors` | live, count per hour per kind | ≤ baseline per kind; a new `what` string is a report line | report; console |
| Witness verdict | `health.verdict`, `intact`, `hub.witness` records | nightly | `verdict = intact`; 0 `CONTRADICTED` | report |
| Backup freshness and seal state | `health.archive`, `archives`, `backupSeal` | nightly | last copy < 26 h; `backupSeal ≠ Off` once G6 is closed | report |
| Image headroom | `worstUsedPerMille`, `worstGrowingPerMille` per venue | live | < 800 ‰; growth/day extrapolates to > 30 days of headroom | report; console |
| Stranded / unheld stock | `health.stranded`, `unheld` | live | 0 | report |
| Quarantine | `health.quarantined` | live | 0 new per day | report |
| Socket health | `lib/live.js` state per surface, from the nightly headless run: reconnect count, `due()` fallbacks | nightly | reconnects ≤ 1 per 10 min | report |

#### Product (aggregate per venue per day; never per person)

| Indicator | Source | Collection | Rule | Shown |
|---|---|---|---|---|
| Orders per venue per day, by channel | `/api/owner/analytics` (`lib.rs` route), `/api/owner/history` folds | nightly | trend only; no gate | report §product; console analytics |
| PENDING → CONFIRMED (p50/p90 minutes) | fold of `Placed`/`Advanced(CONFIRMED)` event timestamps per order, distribution only | nightly | p90 ≤ baseline + 25 %; owners see it as "how fast the kitchen answers" | report; console |
| PENDING → DELIVERED, per stage | same fold, per transition | nightly | trend | report |
| Booking conversion | `reservations` created / `reservations/:id/pass` issued / arrived (`booking.rs` statuses) as counts | nightly | trend | report |
| ETA error (bias, MAE, p90, coverage) | the backtest of the 09-21 blueprint §4 over archived logs, **per venue** — never per courier (OD-8) | nightly | bias ≥ 0 and coverage ≥ 80 % | report; console as "your dishes take N min longer than stated" |
| Abandoned orders | orders that left the FSM without an end state (`dowiz-order-fsm-has-no-exit`) | nightly | count/day; trend | report |
| Stock refusals at checkout | `StockLedger::stranded()` + placement refusals | nightly | 0 orders accepted for stock not there | report |

#### UX

| Indicator | Source | Collection | Rule | Shown |
|---|---|---|---|---|
| Tap-target violations | `tools/gates/tap-size.sh` (44 px, baseline 8) | CI | ratchet toward 0 | report §ux |
| Contrast failures | axe-core in the nightly headless run over the four surfaces at 360 px and 1280 px, `color-contrast` rule only, counted | nightly | 0 serious | report |
| CSP violations | `Content-Security-Policy-Report-Only` twin with `report-to` at `POST /api/csp` (a new route; counts per directive, no page URL beyond the path) | live | 0 per day; a new directive name is a report line | report |
| i18n completeness | above | CI | 0 missing | report |
| First contentful paint per surface | `traffic.mjs` already reads the navigation entry; extend to the four surfaces | nightly | ≤ baseline + 25 % | report |
| Offline shell completeness | the three `SHELL` lists vs. the import graph (`graph.mjs`) | CI | shell ⊇ static graph (the exact failure `sw.js` documents) | report |
| Console tab weight | dynamic chunk bytes per tab (`admin/*.js`, `more.js` 60.7 KB) | CI | ratchet | report |

#### Cost

| Indicator | Source | Collection | Rule | Shown |
|---|---|---|---|---|
| $/venue/month, modelled | the cost model of `dowiz-hub-unit-costs` fed by the measured request counts above (or the modelled ones until the analytics token exists) | nightly | ≤ $1.00 marginal at overage rates (today ≈ $0.65 derived) | report §cost |
| Requests/day: cron share vs user share | 2 × 1,440 × venues (derived from `rails.rs`, `poll.rs`) vs the per-screen table | nightly | cron share ≤ 50 % of the day's requests (F18 closes it) | report |
| Bytes DO → Worker per request class | O9's before/after; `platform/health` sizes of registry/sessions/identity (new route, aggregate only) | nightly | ≤ baseline | report |
| Workers Logs volume | `head_sampling_rate = 0.1` × requests/day | derived | < 20 M/month | report |
| Plan | Free vs Paid — **UNVERIFIED** (the cost blueprint says free, the memory note says $5) | manual, once | recorded | report header |

### B.3 The harness: `tools/evals/`

```
tools/evals/
  run.mjs              orchestrator: --suite ci|nightly|live, --host, --out; exit 1 on any breach
  collect/
    static.mjs         boot graphs, bytes, dead assets, shell ⊇ graph, poll constants, route table
    wasm.py            sections + per-crate + top functions (from §A.1; no build, reads build/index_bg.wasm)
    i18n.mjs           the key walk with stubbed storage (from §A.3)
    live.mjs           GET-only curl-equivalents (fetch + performance timing), median of 3, cf-cache-status
    order.mjs          wraps e2e/evals/traffic.mjs; reads its JSON
    health.mjs         /api/owner/health + /api/platform/errors with the role credentials; aggregates only
    product.mjs        /api/owner/analytics + /api/owner/history folds → distributions; drops rows
    gates.mjs          runs tools/gates/*.sh, parses baselines, counts green
    tests.mjs          cargo test -- --list per crate; *.test.mjs and e2e counts; mutants.out/outcomes.json
    ux.mjs             axe-core contrast at two widths; CSP report counts
    cost.mjs           the unit-cost model over the collected counts
  baselines/
    wasm.baseline surfaces.baseline live.baseline order.baseline tests.baseline mutants.baseline ux.baseline
  report.mjs           JSON → markdown (one table per group, breaches first, the commit and version in the header)
```

**Outputs per run:** `docs/measurements/evals/<YYYY-MM-DD>-<short-commit>-<suite>.json` and `.md`; the markdown's
first section is the breach list (empty = green), the second is every indicator with value, baseline, delta. The
JSON is one flat object `{ indicator_id: { value, unit, baseline, rule, source, collected_at } }` so a later
dashboard needs no schema migration. The nightly `.md` for the venue-facing rows (PENDING→CONFIRMED, ETA bias,
outbox) is also what the owner console's health card reads, through the same `/api/owner/health` — nothing is
shown to an owner that is not already computed for them.

**Where each suite runs:**

- **CI** (`ci.yml`, a new job after the gates): `static`, `wasm` (needs the build artifact — on GitHub that is a
  `worker-build` step; on this box `bash bebop-lang/tools/slot.sh opt 'cd workers/api && worker-build --release'`),
  `i18n`, `gates`, `tests`. No network, no credentials. Fails the push on any ratchet breach.
- **Nightly** (`.github/workflows/evals-nightly.yml`, or the box's own cron via `slot.sh`): `live`, `order`,
  `health`, `product`, `ux`, `cost`, against `HOST=https://sushi-durres.dowiz.org` with the QA credentials in
  `/root/.dowiz_owner` (the courier one is `QA_COURIER_*`). Places one `QA-SOAK`-marked order and puts the venue
  back, as `traffic.mjs` already does. Median of three for timings.
- **Live** (every 5 minutes, `tools/health-gate.mjs` already exists as the fail-closed shape): `health` only —
  outbox age, error rate, image headroom, witness — writing one line per tick to `docs/measurements/live/` and
  raising through the existing `tools/ops-alert` path on a threshold.

**Regression rules, in one place** (`tools/evals/rules.mjs`): `ratchet` (≤ baseline; lowering rewrites the
baseline in the same run and says so), `plus25` (≤ baseline × 1.25, timings), `exact` (counts that must not
move), `zero` (must be 0), `trend` (recorded, never fails). A rule is named in the catalogue row and in the JSON.

### B.4 Wave H — proposed roadmap rows (not written to the roadmap)

| Row | What | Size |
|---|---|---|
| H1 | `tools/evals/run.mjs` + `report.mjs` + `rules.mjs`; baselines dir; the JSON/markdown contract | S |
| H2 | `collect/static.mjs` + `collect/i18n.mjs` + `collect/wasm.py` (from this document's scripts) and the CI job | S |
| H3 | `collect/live.mjs` (GET-only probes, median of 3, cache status, photo sample) | S |
| H4 | Run `e2e/evals/traffic.mjs` once, commit its baseline, wrap it as `collect/order.mjs` | S |
| H5 | `collect/health.mjs` + `GET /api/platform/health` (registry/sessions/identity sizes, cron `{fired_at, venues, ms}` record — the F18 instrument) | M |
| H6 | `collect/product.mjs`: PENDING→CONFIRMED, stage times, booking conversion, ETA backtest per venue — distributions only, OD-8 checked by `no-scoring.sh` extended to `tools/evals/` | M |
| H7 | `collect/ux.mjs`: axe contrast at two widths; `POST /api/csp` report-only endpoint and its counts | M |
| H8 | `collect/tests.mjs` + `collect/gates.mjs` + the mutation-score baseline from the existing `cargo mutants` job | S |
| H9 | `collect/cost.mjs`: the unit-cost model over collected counts; the Cloudflare analytics token as a documented manual step | S |
| H10 | Nightly workflow + live tick through `tools/health-gate.mjs`; `docs/measurements/evals/` convention | S |
| H11 | **O1** small photo variants: re-shrink action + live coverage check | S |
| H12 | **O2** fingerprinted assets, generated SW shell lists, `_headers` cache lines | M |
| H13 | **O3** manifest from settings; **O4** one catalogue read at console boot; **O5** language-aware font preload; **O6** shared status words | S |
| H14 | **O7** strip the `name` section post-bindgen; correct the `Cargo.toml` size comment; wasm size in the CI eval | S |
| H15 | **O8** sync-shape the ten largest handlers, measured per function with `wasm.py` before/after | L |
| H16 | **O9** platform lookups as object commands (after H5 has measured the image sizes) | M |
| H17 | **O10** landing images; **O11** move `kit/` out of `public/`; **O12** split `store.css` | S |
| H18 | Console stats freshness fix (`loadStats` outside the `due()` branch) with the H6 check that would have caught it | S |

### B.5 What could not be determined from this box

- The **deployed** bundle's size and startup time (the Cloudflare MCP returns only the script's name and id;
  no token reads the dashboard). The 2.43 MB is the local 09-22 build; the source has moved since.
- Real DO / Worker **request counts** and the real sizes of the platform images — the same analytics-token gap
  the cost blueprint recorded on 09-20 and the launch-gaps doc on 09-24. Every request number above is modelled
  from code and marked so.
- Which **plan** the account is on.
- The small-variant photo size on *these* photos (the 12–20 KB figure is from the encoder settings, not a run).
