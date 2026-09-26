# Blueprint: everything on the Cloudflare FREE plan — where it breaks, and what makes it fit

Date: 2026-09-26. Lane FREE (research, read-only: nothing built, deployed or committed).
Question from the operator: make EVERYTHING work for free, now, on the Workers Free plan,
instead of moving to Workers Paid.

Every number carries one of three labels:

- **DOC** — quoted from a Cloudflare documentation page fetched live on 2026-09-26; the URL and
  the page's own "Last updated" date are in §7.
- **MEASURED** — read today from the GraphQL Analytics API (`/root/.cf_analytics_token`,
  account-scoped, read-only), from the repo, or from the code by line number.
- **ESTIMATED** — arithmetic on measured inputs; the inputs are named beside it.

Where this document and `docs/measurements/COSTS-NOW-AND-BEBOP-2026-09-26.md` (commit 37eebe59)
disagree, this one re-derived the number today and says why (§1.3, §3).

---

## 0. The answer in ten lines

1. **Free is workable, and the CPU wall is not the first wall.** Two things trip earlier as
   hubs are added: the **minute cron** (already 9.7 ms CPU p50 at 2 venues, 656 of 1,440 runs
   over 10 ms on 09-25, MEASURED) and the **Durable Object request floor it creates**
   (≈ 1440 × (3 + 3 per venue) per day, MEASURED from per-object counts). Together they cap
   Free at **≈ 4 to 6 hubs** today, before any customer traffic.
2. **The 622 CPU kills have two measured sources**: (a) requests that pull a **whole image
   into the Worker** — the 538 KB catalogue is parsed in the Worker by ~40 handlers, and every
   hour with kills on 09-24 had ≥ 100 KB of object bytes per Worker request while hours
   without such pulls sat at 1–4 ms p50; (b) the minute cron. A third, **login argon2id**, is
   ESTIMATED at 20–60 ms in wasm and cannot fit 10 ms in the Worker at any honest parameter.
3. The kills cluster in **17:00–22:00 UTC (19:00–00:00 in Durrës)**: the isolate's over-limit
   allowance is spent exactly when the kitchens are busiest. An order placement killed at
   1102 is a customer who saw an error.
4. **Thirteen rows (FT1–FT13)** move the work to where Free does not count it: into the
   objects (catalogue answers, alarms, night jobs, login), onto R2 behind a custom domain
   (photos, no Worker on the read path), and off the poll (sockets, ping auto-response).
5. Capacity on Free (§5): **today ≈ 4–6 hubs** (cron), **≈ 2 busy hubs** (object requests);
   **after FT1–FT8 ≈ 64 hubs at today's per-hub traffic, ≈ 45 busy hubs**, bound by the
   100k/day request caps and 100k/day rows written.
6. **Where Free ends (§6):** ≈ 45 busy or ≈ 64 quiet hubs; and a property no row removes —
   every daily cap **fails closed for every venue at once at 00:00 UTC**. One crawler, one QA
   sweep, blacks out every storefront until midnight. That is the honest reason to buy the
   $5 plan the day a venue pays; the rows below are worth doing on either plan because they
   are the same lines the Paid bill is made of.
7. Operator idea 1 (batch object writes): **NO for the order log** (an append that is not
   durable is an order that did not land, and the first eviction loses it); rows written are
   the third wall, not the first. Idea 2 (photos to R2): **YES**, with the read path off the
   Worker (FT3). Idea 3 (hibernation): **already in place and the object does hibernate**
   (MEASURED: 218 s active in a 15,983-request day); what wakes it is the client's 25 s ping,
   fixed by auto-response (FT6). Idea 4 (IndexedDB): saves CPU and bytes, **not requests**,
   because a 304 still invokes the Worker; keep it for feel, not for the cap (FT11).
8. The Workers Logs that would name the killed routes are readable **only in the dashboard**
   (3-day retention on Free, 10 % sample): filter `$workers.outcome = exceededCpu` and read
   `$workers.requestUrl`. No token on this box can do it (OPEN 1).
9. Durable Object CPU: the limits page's table says **30 s default per request**; its FAQ says
   objects follow the Workers plan. MEASURED: 0 `exceededCpu` over 50,155 object requests,
   including whole-log folds at 246 ms wall. The blueprint treats the object as the place CPU
   can go, and FT13 includes the one deliberate over-10-ms probe that settles it (OPEN 2).
10. Nothing here needs bebop, a new language or a new platform. Effort for the whole set is
    ≈ 14–20 lane-days (§4).

---

## 1. Free-plan limits against today's usage

### 1.1 Traffic and the cron share (MEASURED)

`workersInvocationsAdaptive` counts **cron invocations as requests**: hours with no human
traffic on 09-24 show 54–76 requests (one per minute) at p50 ≈ 4 ms, and the scheduled dataset
shows the minute cron running 1,440 times a day. The COSTS doc's "28,970 Worker requests in 7
days" therefore contains **10,080 cron invocations**; human and client traffic was **18,890**,
or **1,349 per hub-day** across 2 hubs (QA and lane traffic included — OPEN 3).

Per object on 09-25 (`durableObjectsInvocationsAdaptiveGroups` by `objectId`, MEASURED):

| object | requests | bytes out | reading |
|---|---:|---:|---|
| `__platform` | 4,335 | 13.8 MB | ≈ 3 per minute: the registry read by the outbox sweep and by the ebills sweep every minute, plus `record_cron` hourly, plus every request's `of_slug`/`identity` read |
| venue A | 4,348 | 14.7 MB | ≈ 3 per minute: outbox drain + ebills tick (+ report) |
| venue B | 2,887 | 36.8 MB | ≈ 2 per minute + the day's traffic |

So the cron floor is **≈ 1440 × (3 + 3 V) object requests per day** for V venues, and on a
quiet day with 2 venues that is 8 of the 11,570 object requests per minute
(`workers/api/src/lib.rs:566` scheduled → `outbox/rails.rs:158`, `ebills/poll.rs:33`;
`fiscal/rail.rs:58` returns at once while `SEND_ENABLED` is false).

### 1.2 The table

Headroom is limit ÷ today's daily figure. "Breaks at" is the hub count at which the line trips
at **profile A** (today's per-hub traffic, 1,349 Worker requests per hub-day) and **profile B**
(a busy hub: 30 orders, console 12 h, courier 8 h, 100 visits, ≈ 15,000 Worker requests per
hub-day — the COSTS doc's model, ESTIMATED). Formulas are in §5.

| Product · line | Free limit (DOC) | Today (MEASURED, worst day 09-19..09-25) | Headroom | Breaks at (A / B) |
|---|---|---|---:|---|
| Workers · requests | 100,000/day, resets 00:00 UTC; over it: Error 1027, or **429 on `run_worker_first` paths** | 7,495 (09-19), of which 1,440 cron | 13× | 73 / 6 hubs |
| Workers · CPU per invocation | **10 ms** (Paid: 30 s default, 5 min max); "built-in flexibility" for infrequent overruns | success p50 2.7 ms, **p90 20 ms, p99 91 ms**; 622 kills at exactly 10,000 µs | **0×: already tripping** | now |
| Workers · CPU per Cron Trigger | **10 ms** on Free | minute cron **p50 9.7 ms, p90 12.6 ms, max 69.5 ms** (09-25, 1,440 runs, all "success"); nightly **210 ms** (09-25, success) | ≈ 1× | ≈ 4–6 hubs (EST, §2.3) |
| Workers · subrequests | 50 external per invocation; 1,000 to internal services (KV, DO, R2) | success mean 2.85/request (09-24); nightly loops every venue with ≥ 1 S3 `PutObject` each | fine | nightly: 50 venues-with-bucket |
| Workers · memory | 128 MB per isolate | not reported by any dataset | — | — |
| Workers · size / startup | 64 MiB / 1 s | ≈ 447 KB wasm + 22 KB JS (Cargo.toml note) | 140× | — |
| Workers · Cron Triggers | 5 per account | 2 | 2.5× | — |
| Workers · Static Assets | **requests free and unlimited**; 20,000 files, 25 MiB each | 412 files, 19 MB; 14,741 asset requests on 09-18 | free | never |
| Workers Logs | 200,000 events/day, **3-day retention** | `head_sampling_rate = 0.1` → ≤ 750 events/day | 260× | ≈ 2 M requests/day |
| Durable Objects · requests | **100,000/day** (HTTP + WebSocket messages + alarms; SQLite-backed classes only on Free) | **16,069** (09-24); 11,570 on a quiet day of which ≈ 8,640 cron | 6× | **15 / 2 hubs** |
| Durable Objects · duration | 13,000 GB-s/day | 27.9 GB-s (09-24) | 466× | ≈ 900 hubs |
| Durable Objects · rows written | **100,000/day** (`put`/`delete`/`setAlarm` each = 1 row) | **11,315** (09-24, an import day); 7-day mean 3,126 | 9× | 64 / ≈ 66–100 hubs |
| Durable Objects · rows read | 5,000,000/day | 17,118 (09-24) | 292× | never |
| Durable Objects · storage | 5 GB per account; 10 GB per object | 9.65 MB across 3 objects (`durableObjectsSqlStorageGroups`, 09-25) | 518× | ≈ 1,000 hubs |
| Durable Objects · classes | 100 | 1 (`HubImages`) | — | — |
| Durable Objects · CPU | table: 30 s default; FAQ: "same per-invocation CPU limits as any Workers" | `exceededCpuErrors` 0 in 50,155 requests; wall p99 up to 246 ms | see OPEN 2 | — |
| KV · reads | 100,000/day | 5,804 (09-19) | 17× | ≈ 190 / ≈ 90 hubs |
| KV · writes | **1,000/day** to different keys; 1/s per key | **822 (09-18, the photo import)**; a photo = 2 writes (blob + `#type`, `media.rs:50-56`), 4 with `?variant=small` | 1.2× on an import day | **1–3 venue onboardings per DAY**; steady 26 hubs |
| KV · storage | 1 GB per account | 10.4 MB, 156 keys | 96× | ≈ 200 hubs at 5 MB of photos each |
| R2 · storage / ops | 10 GB-month; Class A 1 M/month; Class B 10 M/month; egress free | 22 MB in 4 buckets; 6 PutObject in the window | ≈ 450× | never (as used today) |
| D1 | 5 M rows read, 100k rows written/day, 5 GB | unbound since `cf834d24`; 2.1 MB left | — | — |
| Queues | 10,000 operations/day; 24 h retention | not used | — | ≈ 3,300 messages/day if used |
| Email | sends to verified destination addresses free and unlimited; 50 recipients per mail | waitlist → one verified address | free | never |
| WebSockets | Worker: the upgrade is 1 request, messages are not; DO: each incoming message is 1 request (billed 20:1 on Paid, "billing only"); `setWebSocketAutoResponse` pongs are not charged | max 3 sockets open; 107–321 `hibernation`-type object invocations/day | see FT6 | — |
| Analytics | GraphQL, account-scoped; 1-week window per query on this account | works | — | — |

### 1.3 Three corrections to the COSTS doc

- **DO requests, not Worker requests, is the first daily cap**, and the count is dominated
  by the minute cron (§1.1). The COSTS doc's "≈ 31 hubs until DO requests trip" used 7-day
  averages that predate the ebills sweep (added mid-week: DO requests 1,656 on 09-22 → 7,380 on
  09-23 while Worker requests fell).
- **Worker requests per hub-day are 1,349, not 2,069**, once the 1,440 daily cron invocations
  are removed.
- **The 10 ms limit applies to Cron Triggers too**, and both crons are already over it.

---

## 2. The 10 ms CPU wall

### 2.1 What the analytics can and cannot split (MEASURED)

`workersInvocationsAdaptive` dimensions: `cacheStatus, coloCode, date, datetime*, scriptName,
scriptTag, scriptVersion, status, usageModel, environmentName, isPreview, dispatchNamespaceName`.
**There is no path, route or host dimension**, so a route can only be inferred from time, bytes
and version. `workersInvocationsScheduled` gives `cron, status, cpuTimeUs, datetime` per run.
`durableObjectsInvocationsAdaptiveGroups` splits by `objectId` and `type` (`http` /
`hibernation`). The Workers Logs that hold `requestUrl` are dashboard-only on this box (OPEN 1).

### 2.2 The kill timeline, 09-24 (MEASURED, UTC hours)

| hour | Worker OK req | OK CPU p50 / p90 / p99 ms | kills | DO req | DO bytes out to Worker | bytes / OK req |
|---|---:|---|---:|---:|---:|---:|
| 02 | 318 | 1.0 / 4.2 / 10.8 | 0 | 300 | 1.8 MB | 6 KB |
| 05 | 56 (cron only) | 4.1 / 5.0 / 8.5 | 0 | 298 | 1.8 MB | — |
| 08 | 153 | **12.9 / 67.2 / 181** | 0 | 793 | 75.0 MB | 490 KB |
| 13 | 269 | **13.4 / 87.4 / 178** | 0 | 1,098 | 179.7 MB | 668 KB |
| 16 | 675 | 1.7 / 16.7 / 114 | 0 | 1,141 | 89.6 MB | 133 KB |
| **17** | 561 | 6.4 / **84.3** / 177 | **32** | 1,508 | **392.4 MB** | 700 KB |
| **18** | 1,142 | 1.3 / 17.4 / 159 | **177** | 1,459 | **397.6 MB** | 348 KB |
| **19** | 213 | 9.4 / 39.6 / 160 | **16** | 783 | 100.2 MB | 470 KB |
| 20 | 314 | 15.7 / 28.8 / 120 | 0 | 1,316 | 167.1 MB | 532 KB |
| **21** | 220 | 7.5 / 52.0 / 126 | **29** | 728 | 125.2 MB | 569 KB |
| **22** | 333 | 7.5 / **90.4** / 147 | **23** | 1,165 | 322.6 MB | 969 KB |

Every hour with kills shipped ≥ 100 MB from the objects into the Worker; every hour under
10 MB sat at p50 ≤ 4 ms and p90 ≤ 5 ms (cron included). The kills are the busiest hours of
whole-image traffic, when the isolate's over-limit allowance is exhausted. The killed requests
themselves had made ≈ 1 subrequest each (328 subrequests for 277 kills): they fetched something,
then burned CPU on it. That is the shape of "pull an image, parse it", not of "hash a password"
(a login also makes one platform read first, so it is not excluded by this alone).

The same allowance explains why the crons survive: on 09-25 the minute cron exceeded 10 ms in
656 of 1,440 runs and the nightly used 210 ms, all reported `success`. The docs say the
allowance exists and that a Worker "hitting the limit consistently" is terminated.

### 2.3 The suspects, ranked (evidence beside each)

| # | Suspect | Where | Evidence | CPU per hit |
|---|---|---|---|---|
| **S1** | **Whole catalogue image parsed in the Worker.** `hubstore::load_catalog` pulls the 538 KB image (67,242 cells, 8× amplified) from the object and `Catalog::load` → `Kv::load` walks every entry eagerly (`bebop-store/src/kv.rs:87-127`). **~40 call sites**, including the hot ones: `/api/menu` (`storefront.rs:228`, edge-cached 30 s), the tracking poll `/api/order/:id` → `live_eta::attach_one` (`live_eta.rs:345`, every 12 s per customer), the console's `owner::orders` → `live_eta::attach`, the manifest (`storefront.rs:677, 813`), `eta.rs:94`, `preview.rs:42`, `legs.rs:28`, `till.rs:226`, `handlers.rs:116`. Also the log: `hubstore::load` in `exceptions.rs:69`, `cloud.rs:643`, `customers/handlers.rs:222`, `assist.rs:250`, `waste.rs:157` | MEASURED correlation in §2.2; hours where bytes/request ≈ 500 KB show p50 10–13 ms | **≈ 10 ms** (ESTIMATED from §2.2: p50 of catalogue-heavy hours minus p50 of light hours) |
| **S2** | **The minute cron.** Two registry `Table::load`s of the platform image, then per venue an outbox `with_table` load + JSON, an ebills `tick` command and its report | MEASURED: p50 9.7 ms, p90 12.6 ms, 656/1,440 over 10 ms at 2 venues; 09-23 (outbox sweep alone) hours show p50 3.6 ms, so the ebills sweep added ≈ 6 ms | **9.7 ms at V = 2; slope ≈ 3 ms per venue (EST)** → "consistently over" at ≈ 4–6 venues |
| **S3** | **Login argon2id** m = 8 MiB, t = 3, p = 1 (`auth.rs:248-252`), constant-work miss (`auth.rs:344`), plus `upgrade_hash` re-hashing legacy hashes on the same request. Routes: `/api/auth/login`, `/api/courier/auth/login`, `/api/staff/login` | Code comment: the default m = 19 MiB "answered 503 error 1102 under any load at all". Native argon2id ≈ 1 ms per MiB-pass; 24 MiB-passes ≈ 24 ms native, wasm 1.5–3× | **20–60 ms (ESTIMATED, unmeasured)** — over the limit on every login; survives on the allowance until a busy hour |
| **S4** | **The nightly cron**: per venue `load_settings`, `idempotency::sweep`, `errlog::prune_at`, `witness::nightly`, `hubstore::rotate` (whole log load + rewrite), export + gzip + S3 put (`cloud.rs:589-700`) | MEASURED: 210,082 µs at 2 venues, success | **≈ 105 ms per venue** |
| S5 | Platform Table parse on every request: `Place::of_slug` reads the registry (`hubstore.rs:167`), `owner_and_venue` reads identity (`owner.rs:159`), sessions on refresh | MEASURED: the platform object served 4,335 of 11,570 requests on 09-25; the platform images are small (13.8 MB / 4,335 = 3 KB each) | ≈ 1 ms (EST) — a request-count problem more than a CPU one |
| S6 | JSON re-encoding of fold answers per poll | responses average 5.5 KB (09-25) | < 1 ms |
| — | **Not suspects, checked:** image transcode (none — `dowiz_hub::media::prepare` sniffs 12 bytes, checks the end marker, sha256; `media.rs:180-212`); ML-KEM / seal (only `cloud/seal.rs` in the nightly path); QR (`qrcodegen`, tiny); PDF (none); MCP (`mcp.rs`, JSON only); fiscal (`SEND_ENABLED = false`) | | |

The 622 kills are S1 in the busy hours, with S3 riding on the same exhausted allowance; S2 and
S4 are the ones that will be killed **next**, as venues are added, because they run every minute
regardless of traffic.

---

## 3. Blueprint rows

Effort: S ≤ 1 lane-day, M = 2–3, L ≥ 4. "Gate" names an instrument that exists or that FT13
adds to `tools/evals/collect/cf.mjs` (today: `cf.worker_requests_day`, `cf.worker_errors_day`,
`cf.cpu_p50_us`, `cf.cpu_p99_us`, `cf.do_requests_day`, `cf.do_errors_day`,
`cf.do_response_bytes_day`). Rows are in priority order; FT13 is first in execution order
because the instrument precedes the change.

### FT13 — Instruments before changes (S)

- **Problem.** No gate today distinguishes a cron kill from a traffic kill, counts KV writes,
  or reads the object split. `cf.worker_errors_day` folds kills into errors.
- **Change.** Add to `collect/cf.mjs`: `cf.cpu_kills_day` (`status: exceededResources`),
  `cf.cron_minute_cpu_p90_us` and `cf.cron_nightly_cpu_us` (`workersInvocationsScheduled`),
  `cf.do_platform_share` (platform object requests ÷ all), `cf.do_hibernation_day` (type
  `hibernation`), `cf.kv_writes_day` (`kvOperationsAdaptiveGroups actionType: write`),
  `cf.do_rows_written_day`. Plus the one probe that settles OPEN 2: a `GET /fold/probe-cpu`
  object route that spins ≈ 25 ms once, behind the owner token, read back in
  `durableObjectsPeriodicGroups.exceededCpuErrors`.
- **Files.** `tools/evals/collect/cf.mjs`, `cf.test.mjs`, `tools/evals/rules.mjs`;
  `workers/api/src/hubdo.rs` (probe route, owner-gated).
- **Saving.** None; every other row's gate depends on it.
- **Risk.** The 1-week query window on this account; the collector already handles it.
- **Gate.** Each new metric appears in the next nightly eval with a baseline.

### FT1 — Stop shipping the catalogue to the Worker (L, split in three lane-sized parts)

- **Problem (MEASURED).** ≈ 500 KB of object bytes per Worker request in every kill hour; p50
  10–13 ms in catalogue-heavy hours vs 1–4 ms otherwise (§2.2). ~40 `load_catalog` sites.
- **Change.** The object already parses its own catalogue for `/fold/venue` (`hubdo.rs`,
  "THE VENUE'S OWN RECORD"). Extend it: `/fold/menu?locale=` (the rendered menu JSON, ≈ 70 KB,
  memoised per catalogue generation like `folded` is for the log), `/fold/product?id=`,
  `/fold/products?ids=` (ETA and preview need a handful), `/fold/location` (already `venue`),
  `/fold/supply?id=`. Then rewrite the callers to ask for answers: **1a** the four hot paths
  (`/api/menu`, `attach_one`, `attach`, manifest); **1b** ordering (`preview`, `legs`, `eta`,
  `till`, `room`); **1c** the rest, and the five `hubstore::load` (whole log) sites that are not
  export/health. `hubstore::load_catalog` stays for writers (`with_catalog`) and export.
- **Files.** `hubdo.rs`, `hubstore.rs`, `fold.rs`, `storefront.rs`, `live_eta.rs`, `owner.rs`,
  `eta.rs`, `services/ordering/*`, `services/orders/*`; `tools/gates/one-image.sh` untouched
  (writers unchanged).
- **Saving.** ≈ 10 ms → ≈ 1 ms per catalogue-pulling request (EST); object bytes to the
  Worker ÷ 7 to ÷ 500. This is the row that ends the kills in traffic.
- **Risk.** Object memory (a memo per generation, ≈ 70 KB, dropped on generation change);
  OPEN 2 if object CPU turns out to be capped at 10 ms too — the parse is the same work moved,
  but memoised, so it runs once per catalogue change rather than once per request.
- **Gate.** `cf.cpu_kills_day = 0` for 7 consecutive days; `cf.do_response_bytes_day ÷
  cf.do_requests_day < 20 KB`; `cf.cpu_p99_us` below 30,000 on a day with orders.

### FT2 — Minute cron → per-object alarms (M)

- **Problem (MEASURED).** 1440 × (3 + 3 V) object requests per day, 9.7 ms CPU p50 at V = 2
  (656/1,440 runs over 10 ms), growing with every venue whether or not anything is queued.
- **Change.** The outbox and the ebills tick live in the venue's object. When the object
  appends an outbox entry (`/fold/append` path) or an ebills plan exists, it calls
  `setAlarm(next_at_ms)`; `alarm()` drains, reschedules with `backoff_ms`, and clears when
  empty. An idle venue schedules nothing and costs nothing. Alarms are durable and
  at-least-once with 6 retries (DOC), which is the property `outbox.rs` asks for ("a write
  that landed is a message that will be delivered"). The cron expression `* * * * *` is
  removed; `record_cron` moves to the nightly. Interim step (S, if the full move waits): one
  registry read shared by both sweeps, and a `HEAD /fold/outbox` that answers 204 without
  shipping bytes.
- **Files.** `hubdo.rs` (alarm handler, `set_alarm` on append), `outbox/rails.rs`,
  `ebills/poll.rs`, `lib.rs:566-585`, `platform_health.rs`, `wrangler.toml` triggers.
- **Saving.** −8,640 object requests/day today, −(4,320 + 3,600 V) in general; −1,440 Worker
  invocations/day; the cron CPU line disappears. +1 row written per `setAlarm` (30–60 per busy
  venue-day).
- **Risk.** Alarm CPU runs under the object's limit (OPEN 2). Telegram/WhatsApp sends from
  inside the object are external subrequests: the 50 cap applies per alarm run — drain at most
  40 per run and reschedule.
- **Gate.** `cf.do_requests_day` on a day with no orders < 1,000; `cf.cron_minute_cpu_p90_us`
  absent (no such cron); `/api/owner/health` outbox depth and oldest entry unchanged in
  behaviour (the existing instrument).

### FT3 — Photos out of KV, onto R2 behind a custom domain (M)

- **Problem (MEASURED).** A photo costs 2 KV writes (4 with the small variant); one venue's
  import wrote 822 keys on 09-18 against a 1,000/day cap, so onboarding is limited to 1–3
  venues a day. Every `/media/*` read is a **Worker request** (the Cache API hit still invokes
  the Worker) — 18 per cold storefront visit (COSTS blueprint 2026-09-20, Playwright), and 2 KV
  reads on a miss.
- **Change.** Bucket `dowiz-media` on R2 with a custom domain (`media.dowiz.org`, the zone is
  already on Cloudflare); the upload handler writes one object with `httpMetadata.contentType`
  (1 Class A op, 1 M/month free); menu JSON emits `https://media.dowiz.org/<sha256>.<ext>`.
  Reads never touch the Worker: R2 custom domains sit behind the Cloudflare cache, egress is
  free, Class B is 10 M/month. Content-addressed names keep the `immutable` cache story.
  Interim (S): store the mime in KV **metadata** instead of a second key — halves the writes.
- **Files.** `services/catalogue/media.rs`, `storefront.rs` (image URLs, manifest
  `png_size` read at `:691`), `wrangler.toml` (`r2_buckets` MEDIA), `public/_headers` and the
  storefront CSP (`img-src media.dowiz.org`), `tools/platform/attach-host.sh` (the domain).
- **Saving.** −18 Worker requests per cold visit (−1,800/day for a busy hub); KV writes per
  photo 2–4 → 0; KV reads → 0. Photos leave the 1 GB KV cap for the 10 GB R2 one.
- **Risk.** Public bucket — photos are already public and content-addressed, so nothing new
  is exposed; the courier/learn videos stay in `dowiz-learn` (gated, unchanged). Migration:
  copy 156 keys once (Class A 156 ops).
- **Gate.** `cf.kv_writes_day` on an import day ≤ number of photos (interim) then 0;
  Playwright cold-visit count: Worker requests per visit ≤ 3 (`tools/evals/collect/static.mjs`
  already counts responses).

### FT4 — Login under the wall (M)

- **Problem (ESTIMATED).** argon2id m = 8 MiB t = 3 in wasm is 20–60 ms per verify; a miss
  costs the same by design; `upgrade_hash` may run a second hash. Every login is over 10 ms
  and lives on the allowance.
- **Change.** First **measure** (an `#[ignore]` test printing the wasm-side time is not
  possible in CI; measure with one login against the deployed Worker while reading
  `cf.cpu_p99_us` for that minute, or via the dashboard's invocation log). Then move the verify
  into the platform object: `POST /fold/login` with the identity table already in the object's
  memory, argon2 run there, the answer a signed claim. The Worker keeps HMAC (`auth::verify`),
  which is microseconds. Keep `verify_password_constant_work` semantics inside the object. Do
  not lower the parameters further: the auth.rs note already records what m = 8 MiB gave up.
- **Files.** `hubdo.rs`, `accounts.rs:190-200, 570-582`, `services/identity/staff.rs`,
  `auth.rs` (unchanged primitives).
- **Saving.** Removes the last route that cannot fit 10 ms in the Worker. Login volume is
  small (a few per venue-day), so this is a correctness row, not a capacity row.
- **Risk.** OPEN 2 (object CPU). If the object is also capped at 10 ms, the remaining honest
  option is WebCrypto `PBKDF2-SHA256` with ≥ 600,000 iterations (OWASP), which runs in native
  code, plus re-hash on next login — a change of primitive that needs its own decision.
- **Gate.** `cf.cpu_kills_day = 0` on a day with ≥ 10 logins (count from `identity` sessions).

### FT5 — Nightly cron fans out to the objects (M)

- **Problem (MEASURED).** 210 ms CPU at 2 venues in one cron invocation (21× the limit);
  ≥ 1 external S3 subrequest per venue against the 50-per-invocation cap; the whole log is
  loaded and rewritten in the Worker for `rotate`.
- **Change.** The cron sends each venue's object one message (`POST /fold/night`) and returns.
  The object rotates its own log (it already holds the bytes), prunes idempotency and errlog,
  writes the witness, exports, gzips and puts to S3 from inside `alarm()` (set for "now" so the
  cron's request returns in microseconds). `record_cron` counts venues messaged.
- **Files.** `cloud.rs:566-700`, `hubdo.rs`, `hubstore.rs:1061` (`rotate` moves object-side),
  `witness/`, `idempotency/mod.rs:202`, `errlog.rs`.
- **Saving.** Cron CPU 210 ms → < 1 ms; per-venue work runs under the object's limits; the
  external-subrequest cap becomes per venue (S3 put + rotation ≈ 3–5 of 50) instead of per night.
- **Risk.** OPEN 2; a night job that throws retries up to 6 times (DOC) — make it idempotent
  by generation, which `rotate` already is (`archives_pending/marked`).
- **Gate.** `cf.cron_nightly_cpu_us < 10,000`; `platform_health` cron record `venues` equals
  the registry count; `dowiz-offsite` object count grows by one per venue per night
  (`r2StorageAdaptiveGroups`).

### FT6 — Ping auto-response on the object (S)

- **Problem (MEASURED).** The client pings every 25 s (`live.js:42`); each ping is a
  `hibernation`-type object invocation (107–321/day now, one per 25 s per open socket =
  1,728/day per socket) that wakes the object and counts as a request. The object has no
  timers or alarms of its own (grep: none), so the ping is the only thing waking it — operator
  idea 3 confirmed: hibernation is used and works (218 s active on a 15,983-request day).
- **Change.** `set_websocket_auto_response({"t":"ping"} → {"t":"pong"})` in `accept()`
  (`hubdo.rs:506-533`). The docs: auto-response messages "will not incur additional wall-clock
  time, and so they will not be charged", and the object stays hibernated.
- **Files.** `hubdo.rs`; `live.js` unchanged (the exact string must match).
- **Saving.** −1,728 object requests/day per open console or courier socket; a busy hub with
  2 sockets 12 h a day saves ≈ 3,500/day.
- **Risk.** `lastHeard` on the client is fed by pongs; auto-pongs still arrive, so `due()`
  behaviour is unchanged.
- **Gate.** `cf.do_hibernation_day ≈ 0` while `activeWebsocketConnections > 0`.

### FT7 — Socket-first polling on all three surfaces (S)

- **Problem (MEASURED/EST).** With a live socket the surfaces already poll every 90 s
  (`QUIET_MS`); without it, the tracking sheet polls every 12 s (`track.js:37`), the courier
  every 12 s, the console every 15 s. A tracked order (40 min) is ≈ 200 polls without a socket,
  ≈ 27 with one, ≈ 6 if only events drove it. Busy-hub profile B's 15,000 requests/day is
  mostly this.
- **Change.** (a) Make the socket the normal path: reconnect with the existing backoff, and
  poll only when `due()`; (b) raise the no-socket fallbacks to 30 s (tracking), 30 s
  (courier), 30 s (console with live orders) — the socket carries the events, the poll is the
  safety net; (c) after FT1, the poll itself is one object request, not three.
- **Files.** `public/store/track.js`, `public/courier/app.js:1156`, `public/admin/app.js:314`,
  `public/admin/core.js:27-30`.
- **Saving.** Profile B 15,000 → ≈ 4,000 Worker requests per hub-day (EST from the intervals).
- **Risk.** A courier's ETA depends on GPS over the socket; if sockets fail behind a proxy the
  30 s poll still works. The 09-24 `max activeWebsocketConnections = 2` with three surfaces in
  QA suggests sockets do not always stay up — measure before relying on them (gate).
- **Gate.** Playwright: requests during one tracked order ≤ 40; `cf.worker_requests_day` per
  order (from `/api/owner/health` order count) ≤ 150.

### FT8 — The platform registry read once per minute, not once per request (S)

- **Problem (MEASURED).** The platform object served 37 % of all object requests on 09-25.
  `Place::of_slug` (every storefront request) and `owner_and_venue` (every console request)
  each read a platform image: 1–2 object requests and a `Table::load` per Worker request
  (ratio r ≈ 2 object requests per traffic request on 09-24).
- **Change.** Cache the registry's slug→venue map and the identity lookups in the **Cache
  API** for 60 s (`Cache::default()`, keyed by generation-less URL; a cache call is a
  subrequest, not a daily-capped operation), invalidated by the writers (`with_registry`)
  through `cache.delete`. Tokens already carry the venue (`claimed_venue`), so the console path
  only needs identity for membership checks — cache those too.
- **Files.** `hubstore.rs:116-200`, `owner.rs:114-170`, `identity_store.rs`, `platform_store.rs`.
- **Saving.** r ≈ 2 → ≈ 1 object request per traffic request; the platform object leaves the
  1,000 rps soft limit's neighbourhood at scale.
- **Risk.** A 60 s window after a venue is renamed or a membership revoked; revocation
  matters, so `with_identity` deletes the cache entry synchronously.
- **Gate.** `cf.do_platform_share < 0.15`.

### FT9 — The root page without the Worker (S)

- **Problem (DOC).** `run_worker_first = ["/"]` makes every storefront front door a Worker
  request, and on Free, over the cap, "these requests will receive a 429 instead of falling
  back to static asset serving". Every other asset is free.
- **Change.** Serve `/` as a static shell whose first inline script chooses the storefront or
  the platform hub by `location.hostname` (the same rule as `Place::slug_of_host`), and
  remove `run_worker_first`. The Worker keeps `serve_root` for `workers.dev`.
- **Files.** `wrangler.toml`, `public/index.html`, `lib.rs:187-216`.
- **Saving.** −1 Worker request per visit (−100/day per busy hub); removes the 429 failure on
  the front door.
- **Risk.** One extra round trip on first paint if the shells differ; keep both shells small.
- **Gate.** `tools/gates/sw-shell.sh` unchanged; Playwright root load shows 0 Worker requests.

### FT10 — Batching object writes (operator idea 1): deferred, with the reason (—)

- **Measured.** Rows written: 3,126/day mean for 2 hubs, 11,315 on an import day; the cap is
  100k/day. Each append is already **one transaction and two `put`s** (chunk 0 + the tail,
  `append_tip_bytes`), and a `put` is one row whatever its size. Rows are the **third** wall
  (§5), behind requests.
- **Durability.** The FSM's rule is "an order that landed is an order"; `outbox.rs` builds on
  "a write that landed is a message that will be delivered". A memory buffer flushed on a timer
  turns both into "probably", and an eviction (memory pressure, deploy, host move) loses the
  buffer. The Worker has already answered the customer by then. **NO** for the log, the stock
  and the outbox. GPS fixes are already memory-only by design.
- **What is legitimate later.** Derived images (`table`, `stock`) rebuilt from the log on
  restart instead of written per event — that is blueprint P1 / `one-image.sh` reaching 0, and
  it saves ≈ 1 row per event. Revisit at ≈ 40 hubs.

### FT11 — Client-side cache for read-only data (operator idea 4): small, honest (S)

- **Facts.** Requests are counted when the Worker is invoked; an `If-None-Match` that
  answers 304 is still a request (pricing page, "Only requests that hit a Worker count"). The
  menu is already edge-cached 30 s (`storefront.rs:619`) and is 1 request per visit. The
  console already draws from `replica.js` first (phase 7).
- **Change.** `ETag: <catalogue generation>` on `/api/menu` and `/api/menu/i18n` (cheap after
  FT1), IndexedDB on the storefront so a revisit paints before the network answers, and skip
  the request entirely when the stored copy is < 5 min old and the tab was just backgrounded.
- **Saving.** CPU and bytes on revisits; requests only in the "skip" case (≈ 0.2–0.5 per
  visit). Worth doing for feel; it does not move the capacity table.
- **Gate.** Playwright revisit paints the menu in < 100 ms offline.

### FT12 — Logs, assets, email: nothing to do (—)

`head_sampling_rate = 0.1` keeps Workers Logs at ≤ 10 % of requests (limit 200k/day, so fine
to ≈ 2 M requests/day); note the **3-day retention** on Free when chasing a failure. Static
assets are free and unlimited (412 of 20,000 files). Sends to the verified waitlist address are
free. D1 is unbound and can be deleted or left; it costs nothing.

---

## 4. Effort and order

| order | row | effort | why here |
|---|---|---|---|
| 1 | FT13 instruments + OPEN 2 probe | S | every gate below |
| 2 | FT2 (interim: shared registry read + 204-without-bytes), then alarms | S + M | the wall that grows with venues, not with traffic |
| 3 | FT1a hot paths | M | ends the kills in traffic |
| 4 | FT6 auto-response | S | one function call, large object-request saving |
| 5 | FT3 interim (mime in metadata), then R2 domain | S + M | onboarding limit; Worker requests per visit |
| 6 | FT8 registry cache | S | halves object requests per traffic request |
| 7 | FT5 nightly fan-out | M | before the third venue with a bucket |
| 8 | FT7 socket-first polling | S | the busy-hub request line |
| 9 | FT4 login in the object | M | correctness under load |
| 10 | FT1b, FT1c | M + M | the long tail of catalogue readers |
| 11 | FT9, FT11 | S + S | front-door 429, feel |

Total ≈ 14–20 lane-days, three lanes → ≈ 1–1.5 weeks of the box, gates permitting.

---

## 5. Capacity on Free: today and after each row

Per hub-day inputs. Profile **A** = today's per-hub traffic, QA included (MEASURED
09-19..09-25 ÷ 2, cron removed): 1,349 Worker requests; r = 2.0 object requests per traffic
request (09-24, ESTIMATED); 1,563 rows written; 37.5 KV writes; 523 KV reads. Profile **B** =
busy hub (COSTS model, ESTIMATED): 15,000 Worker requests, 30 orders, 1,000–1,500 rows written
(≈ 30 rows per delivered order + outbox + alarms; the COSTS doc's 11,300 scaled rows with polls,
which is the wrong basis — rows scale with orders), 100 cold visits × 18 photos.

Formulas (per day, V hubs): Worker = 1,440 (cron) + w·V ≤ 100k; DO = 4,320 + 3,600·V (cron)
+ r·w·V ≤ 100k; rows = rows·V ≤ 100k; minute-cron CPU ≈ 3.7 + 3·V ms (EST, 09-23 vs 09-25),
"consistently over" taken as p50 > 15 ms.

| state | binding line, profile A | hubs A | binding line, profile B | hubs B |
|---|---|---:|---|---:|
| **today** | minute-cron CPU (EST), then DO requests 4,320 + 6,300·V | **≈ 4–6** (15 by DO) | DO requests 4,320 + 33,600·V | **2** |
| + FT13 | unchanged | 4–6 | unchanged | 2 |
| + FT2 (cron → alarms) | DO requests 2,698·V | 37 | DO requests 30,000·V | 3 |
| + FT1a (no catalogue in the Worker) | same counts; kills → 0 | 37 | same | 3 |
| + FT6 (auto-pong) | same (sockets rare in A) | 37 | DO −3,500·V → 26,500·V | 3 |
| + FT3 (photos to R2) | Worker 1,349 → ≈ 1,100·V | 37 (DO) | Worker 15,000 → 13,200·V | 3 (DO) |
| + FT8 (r 2 → 1) | Worker and DO tie at ≈ 1,100·V | **≈ 74** → rows written 1,563·V binds | DO 13,200·V | 7 |
| + FT5, FT4 | unchanged counts | 64 (rows) | unchanged | 7 |
| + FT7 (socket-first) | ≈ same for A | **64** | Worker ≈ 2,200·V, DO ≈ 2,200·V, rows 1,500·V | **≈ 45** |
| + FT9, FT11 | −100·V Worker | 64 | Worker 2,100·V | 45–47 |

Read the A column as "hubs like today's two, dogfooding included" and the B column as
"restaurants doing 30 deliveries a day with the console open all service". Real venues will
sit between them.

---

## 6. Where Free ends

- **Numerically:** after every row, ≈ **45 busy hubs** (Worker and object requests at
  ≈ 2,200 per hub-day each against 100k/day) or ≈ **64 hubs at today's mixed traffic** (rows
  written at ≈ 1,563 per hub-day against 100k/day). Past that, Workers Paid ($5/month, then
  $0.30 per extra million requests) is unavoidable, and at that size the bill is the $5 floor
  until ≈ 100 hubs (COSTS doc §1.6).
- **Structurally, and no row changes this:** the Free caps are **daily, account-wide, and
  fail closed**. At 100,000 Worker requests every storefront on `*.dowiz.org` answers 1027 or
  429 until 00:00 UTC; at 100,000 object requests every `put` and `fetch` to every venue's
  object fails "with an error" (DOC, DO pricing page); at 1,000 KV writes the next photo
  upload fails. One crawler, one load test, one lane's Playwright run spends the budget for
  every paying venue at once. On Paid the same event costs cents.
- **CPU:** the 10 ms limit is soft and per isolate, which is why the crons survive today and
  why the kills come in the busiest hour. After FT1/FT2/FT4/FT5 no route in the Worker needs
  more than a few milliseconds, so this stops being a wall — **provided OPEN 2 holds** (object
  CPU at 30 s). If it does not, the login and the night job have nowhere left to run under
  Free, and that alone ends it.
- **Recommendation.** Do FT13, FT2, FT1a, FT6, FT3 regardless of plan: they are the four
  largest lines of the Paid bill as well (requests, object requests, rows, KV). Stay on Free
  while every venue is a dogfood or a trial. Move to Paid the day a venue pays, for the
  fail-closed reason, not for capacity.

---

## 7. Sources

Cloudflare documentation, fetched 2026-09-26 (page's own "Last updated" in brackets):

- Workers limits — https://developers.cloudflare.com/workers/platform/limits/ (Sep 5, 2026):
  "Requests 100,000/day · CPU time 10 ms · Subrequests 50/request · Cron Triggers per account 5
  · CPU time per Cron Trigger 10 ms · Subrequests to internal services 1,000 · Static Asset
  files per Worker version 20,000 · Cache API calls per request 50".
- Workers pricing — https://developers.cloudflare.com/workers/platform/pricing/ (Aug 28, 2026):
  "Free 100,000 per day · 10 milliseconds of CPU time per invocation"; "Requests to static
  assets are free and unlimited"; "WebSocket connections made to a Worker are charged as a
  request … WebSocket messages routed through a Worker do not count as requests"; Workers Logs
  "Workers Free 200,000 per day, 3 Days"; KV "Keys read 100,000/day · Keys written 1,000/day ·
  Keys deleted 1,000/day · List requests 1,000/day · Stored data 1 GB"; Queues "10,000
  operations/day"; D1 "5 million rows read/day · 100,000 rows written/day · 5 GB"; "Only
  requests that hit a Worker will count against your limits".
- Durable Objects pricing — https://developers.cloudflare.com/durable-objects/platform/pricing/
  (Aug 25, 2026): "Requests 100,000/day · Duration 13,000 GB-s/day"; "If you exceed any one of
  the free tier limits, further operations of that type will fail with an error"; "Workers Free
  plan can only create and access SQLite-backed Durable Objects"; "Rows written 100,000/day ·
  Rows read 5 million/day · SQL stored data 5 GB"; "Each setAlarm() is billed as a single row
  written"; "a 20:1 ratio is applied to incoming WebSocket messages … for billing purposes";
  "auto-response messages … will not incur additional wall-clock time, and so they will not be
  charged".
- Durable Objects limits — https://developers.cloudflare.com/durable-objects/platform/limits/
  (Jun 1, 2026): "CPU per request 30 seconds (default) / configurable to 5 minutes"; FAQ
  "Durable Objects are Worker scripts, and have the same per invocation CPU limits as any
  Workers do"; "Storage per account 5GB (Free)"; "soft limit of 1,000 requests per second" per
  object.
- Durable Objects alarms — https://developers.cloudflare.com/durable-objects/api/alarms/
  (Apr 21, 2026): "guaranteed at-least-once execution … up to 6 retries"; one alarm per object.
- KV limits — https://developers.cloudflare.com/kv/platform/limits/ (Apr 21, 2026): "Writes to
  different keys 1,000 per day · Writes to same key 1 per second · Storage 1 GB".
- Static assets billing — https://developers.cloudflare.com/workers/static-assets/billing-and-limitations/
  (Apr 23, 2026): "Requests to static assets are free and unlimited"; "When using
  run_worker_first … If you exceed your free tier request limits, these requests will receive a
  429".
- R2 pricing — https://developers.cloudflare.com/r2/pricing/ (Aug 7, 2026): "Storage 10 GB-month
  · Class A 1 million/month · Class B 10 million/month · Egress free".
- Queues pricing — https://developers.cloudflare.com/queues/platform/pricing/ (Apr 21, 2026).
- Workers Logs — https://developers.cloudflare.com/workers/observability/logs/workers-logs/
  (Aug 11, 2026).
- Email limits — https://developers.cloudflare.com/email-service/platform/limits/ (Sep 25, 2026;
  `/email-routing/limits/` redirects here): "Sends to verified destination addresses are always
  free"; Free-plan email Workers "count toward the standard Workers CPU and memory limits".

Measurements: GraphQL queries and outputs are in the lane scratchpad
(`scratchpad/ft/*.json|out`, session-local); the datasets used are `workersInvocationsAdaptive`,
`workersInvocationsScheduled`, `workersAssetsRequestsAdaptiveGroups`,
`workersSubrequestsAdaptiveGroups`, `durableObjectsInvocationsAdaptiveGroups`,
`durableObjectsPeriodicGroups`, `durableObjectsSqlStorageGroups`, `kvOperationsAdaptiveGroups`.
Code references are to `workers/api/src/` unless a crate path is given.

---

## OPEN

1. **Which URLs were killed** is in Workers Logs, dashboard-only, 3-day retention, 10 %
   sample. The operator can read it: Workers & Pages → dowiz-api → Observability, filter
   `$workers.outcome = exceededCpu`. Nothing on this box can.
2. **Durable Object CPU on Free**: 30 s (limits table) or 10 ms (limits FAQ). Measured 0
   `exceededCpu` in 50,155 requests with folds at 246 ms wall, which favours 30 s, but no request
   is proven to have crossed 10 ms of CPU. FT13's probe settles it; FT1, FT2, FT4, FT5 depend on
   the answer.
3. **QA traffic is inside profile A.** The A column is an upper bound on a quiet venue's cost.
4. **argon2 wasm timing** is estimated, not measured (S3).
5. **The minute cron's CPU slope per venue** (≈ 3 ms) is from two days with different sweeps
   installed; one day with a third venue would pin it.
6. **`inboundWebsocketMsgCount` reads 0** every day while `hibernation` invocations are
   107–321 and pings are sent every 25 s: either the metric does not count hibernation-API
   messages or sockets never live 25 s. FT6's gate reads the invocation type instead.
7. **DO stored bytes per venue** (≈ 4.8 MB) come from one day's `durableObjectsSqlStorageGroups`
   maximum divided by three objects; the platform object is not separated.
8. **Whether the 20:1 WebSocket ratio applies to the Free daily count** or only to Paid
   billing. The docs say "billing only"; FT6 makes it moot for pings.
