# What dowiz costs to run now, and what an all-bebop rewrite would cost (2026-09-26)

Lane COST, read-only. Nothing was built, deployed or committed. Every number is labelled:

- **MEASURED**: read from a Cloudflare API, a file, or a `wc` count on this date.
- **ESTIMATED**: a model built on measured inputs. The assumptions are stated next to it.
- **MEMORY**: carried over from a memory note and not re-verified today.

Prices come from https://developers.cloudflare.com/workers/platform/pricing/ ("Last updated Aug 28, 2026"),
fetched live on 2026-09-26. Every Cloudflare rate below is quoted from that page.

---

## 0. The answer in five lines

1. **Cloudflare charges nothing for dowiz today. The account is on the Workers FREE plan.** All 622 of
   dowiz-api's `exceededResources` kills in the window stopped at exactly **10,000 µs of CPU**, the
   Free plan's per-invocation limit, on every day they occurred (MEASURED, §1.3). The only recurring
   cost we can prove is the domain, **≈ $0.93/mo** ($11.20/yr, MEMORY).
2. **That free plan is costing us requests.** 1.4 % of dowiz-api's requests (622 of 44,209) were
   killed at the 10 ms CPU limit. The fix is the $5/mo Workers Paid plan, not a rewrite.
3. **On Paid, dowiz stays at the $5 floor until roughly 100 hubs at today's traffic, or 10 to 30 busy
   hubs.** At 1000 hubs the ESTIMATED bill is **$54/mo** at today's per-hub traffic, or **≈ $700/mo**
   if every hub is busy all day. That is $0.05 to $0.70 per hub per month.
4. **Running cost per order is ≈ $0.0002 (≈ 0.015 lek).** If card payment is turned on, Stripe's fee on
   one 1500-lek order (≈ €0.48) is about **2,500 times** the whole infrastructure cost of that order.
5. **A bebop rewrite does not lower the lines where the money goes.** Requests are set by client polling.
   Durable Object rows written are set by the chunk layout, which is already bebop's store format. KV
   holds opaque photos. The one line bebop would change is CPU, and it would get **worse**, by 2.6× to
   10.6× (bebop is that much slower than Rust on compute today). The VPS saving is real, but the Rust
   `native-spa-server` that already exists can deliver it. Rewrite effort is **≈ 140 to 270 agent
   lane-days, or ≈ 100 to 150 person-weeks**, gated by five missing language/runtime features.

---

## 1. Current costs: measured

### 1.1 What exists on the account (MEASURED, read-only GETs with `/root/.cf_deploy_token`)

| Resource | Value | Source |
|---|---|---|
| Worker scripts | `dowiz-api` (created 2026-09-15T22:11Z), `academia-mesh`, `initial-survey` (the last two are not dowiz) | `GET /accounts/{id}/workers/scripts` |
| Usage model | `standard` | `GET .../workers/account-settings` |
| DO namespace | `dowiz-api_HubImages` (SQLite-backed, `new_sqlite_classes`) | `GET .../workers/durable_objects/namespaces` |
| KV namespace | `dowiz-media` (photos, content-addressed) | `GET .../storage/kv/namespaces` |
| R2 buckets | `dowiz-images`, `dowiz-learn`, `dowiz-offsite`, `academia-matrix` | `GET .../r2/buckets` |
| D1 | `dowiz`, 2,113,536 B. **Unbound** since `cf834d24`; the rows remain (MEMORY: no-SQL migration) | `GET .../d1/database` |
| Hostnames | `dowiz.org`, `sushi-durres.dowiz.org`, `dubin-sushi.dowiz.org` → dowiz-api | `GET .../workers/domains` |
| Plan / subscriptions | **not readable**: both tokens return `10000` on `/subscriptions` and `7003` on `/workers/subscription` | GET |

### 1.2 Traffic, 2026-08-27 → 2026-09-26 (MEASURED, GraphQL with `/root/.cf_analytics_token`)

**The window is really 11 days.** `dowiz-api` was created 2026-09-15, and its first row in the dataset
is that day. Every monthly figure below extrapolates the **last 7 full days (09-19..09-25)**, which
cover **2 live hubs plus all QA, lane and dogfood traffic**. There is no way to separate the two (OPEN 1).

`workersInvocationsAdaptive`, script `dowiz-api`, whole window:

| status | requests | CPU sum (ms) | CPU p50 / p99 (ms) | subrequests |
|---|---:|---:|---:|---:|
| success | 43,213 | 466,584 | 2.76 / 102.5 | 51,660 |
| exceededResources | **622** | 7,897 | **10.00 / 75.0** | 415 |
| scriptThrewException | 203 | 143 | 0.62 / 2.08 | 0 |
| clientDisconnected | 116 | 1,595 | 6.9 / 73.2 | 238 |
| responseStreamDisconnected | 55 | 325 | 4.8 / 24.7 | 159 |
| **total** | **44,209** | **476,544** | | 52,472 |

The last 7 days (MEASURED): **28,970 Worker requests**, **240,916 CPU-ms** (mean **8.32 ms/request**;
p25 1.3, p50 2.7, p75 7.7, p90 20.1, p99 91.3, p99.9 198 ms).

Per day (MEASURED, dowiz-api):

| date | Worker req | CPU s | DO req | DO bytes per response (KB) | DO GB-s | DO rows written |
|---|---:|---:|---:|---:|---:|---:|
| 09-16 | 5,699 | 141.0 | 1,377 | 469 | 1.75 | 1,113 |
| 09-18 | 6,628 | 54.9 | 3,437 | 309 | 6.78 | 7,612 |
| 09-19 | 7,495 | 62.3 | 2,755 | **575** | 3.24 | 1,184 |
| 09-20 | 1,378 | 8.3 | 268 | 359 | 0.56 | 8 |
| 09-21 | 7,031 | 53.1 | 4,937 | 146 | 5.72 | 2,656 |
| 09-22 | 2,849 | 12.3 | 1,656 | 44 | 2.61 | 639 |
| 09-23 | 2,070 | 9.3 | 7,380 | 11.5 | 14.89 | 3,186 |
| 09-24 | 5,821 | 79.2 | 16,069 | 119 | 27.92 | 11,315 |
| 09-25 | 2,326 | 16.4 | 11,570 | **5.5** | 18.10 | 2,892 |

A side finding: **bytes per DO response fell about 100×, from ≈ 575 KB to ≈ 5.5 KB**, after the
"object folds its own log" phases went live (MEMORY: seven-phases). The whole-image-per-poll problem
from the 09-20 cost note is gone on the wire. The fall is not monotonic. On 09-24 responses averaged
119 KB (image paths: export, health, graph_facts, placement).

Durable Objects, whole window (`durableObjectsInvocationsAdaptiveGroups` + `durableObjectsPeriodicGroups`),
MEASURED:

- **50,155 DO requests**: success 49,994, clientDisconnected 154, exception 1, stream-disconnected 6.
- DO response bytes: 6.71 GB, carried into the Worker. This is not billed.
- **Duration 83.17 GB-s**, cpuTime 168.9 s, activeTime 649.7 s.
- **Rows read 54,445; rows written 30,605.**
- WebSocket messages out 777, in 0.
- `exceededCpu` 0, `exceededMemory` 0.
- `durableObjectsStorageGroups` returns **empty**, so stored bytes are NOT measured (OPEN 2).

KV `dowiz-media` (MEASURED): 12,355 reads (818 MB), 826 writes (49 MB) in the window. Storage 156 keys,
**10,426,277 B**.

R2 (MEASURED, `r2StorageAdaptiveGroups`, 2026-09-25):

| bucket | objects | bytes |
|---|---:|---:|
| dowiz-images | 65 | 4,784,998 |
| dowiz-offsite | 7 | 17,221,874 (nightly copies) |
| dowiz-learn | **0** | 0 (created, nothing uploaded yet) |
| eu_dowizassets | 2 | 93,115 |

Operations: 6 PutObject, 3 ListObjects, 1 PutBucket, 3 ListBuckets.

### 1.3 Evidence that the account is on the Free plan (MEASURED, inference labelled)

`exceededResources` rows per day: **09-16: 232, 09-18: 111, 09-24: 277**. On every one of those days
CPU p25, p50, p75 and p90 are all **exactly 10,000 µs**. Two further kills on 09-15 were at 109 ms and
357 ms, before the script settled. 10 ms is the Free plan's "10 milliseconds of CPU time per invocation".
The Paid limit is 30 s by default (pricing page). Successful requests do reach p90 = 20 ms, which fits
the Free plan's burst allowance. The API gives no way to read the plan (§1.1), so this is **inferred
from a measurement, not read**. The 09-20 memory note had already recorded the wrangler `10097` error
that forced `new_sqlite_classes`. That is the same fact seen from the other side.

**Consequence:** 1.4 % of all dowiz-api requests, and 2.9 % on the three days it happened, were killed.
Which routes they hit cannot be read from this dataset (OPEN 3). The usual suspects are argon2 on
login and whole-image folds (`export`, `health`, placement).

### 1.4 Other recurring costs found in the repo

| Item | Cost to the platform | Evidence |
|---|---|---|
| Domain dowiz.org | **$11.20/yr ≈ $0.93/mo** | MEMORY (dowiz-hub-unit-costs, 2026-09-20), not re-verified |
| Email (waitlist) | $0. Cloudflare Email Routing + `send_email` binding | `wrangler.toml` |
| Voice | **$0.** Speech is the browser's `SpeechRecognition` (`public/lib/voice.js:19`). Azure/voice lessons were dropped (MEMORY: wave-L decisions) | code |
| LLM / AI assist | **$0 to the platform.** `assist.rs` calls an endpoint the VENUE configures (`ai.endpoint`, `ai.model`). No platform key exists | `workers/api/src/assist.rs:90-118` |
| WhatsApp / Instagram / Telegram | **$0 to the platform.** Tokens live in venue settings (`notify.whatsapp.*`, venue-owned bot), so Meta's per-message fees fall on the venue | `integrations.rs:54-144` |
| Stripe | $0 today (keys `STRIPE_*` are secrets and payments are not live; MEMORY). If enabled: **1.5 % + €0.25** standard EEA card, 3.15 % + €0.25 international, +2 % FX (https://stripe.com/ie/pricing, fetched 2026-09-26). This is a pass-through fee per order | `stripe.rs:48` |
| S3 off-site copies | R2 `dowiz-offsite` 17 MB, inside the 10 GB free tier. A venue may point `cloud.s3.*` at its own bucket, at its own cost | `cloud.rs:49` |
| R2 dowiz-learn videos | 0 B stored today. At 10 GB free, then $0.015/GB-mo, lesson videos stay under $1/mo unless they exceed ~70 GB | R2 API |
| OTEL export | Unset (`OTEL_EXPORTER_OTLP_*` are optional secrets) | code |
| Fiscal (eBills) | $0. `SEND_ENABLED=false`, import-only (MEMORY) | |

Not a running cost, but the largest spend by far: **development**. That means agent lanes and the
operator's Claude usage. 651 commits since 2026-08-01, and 63,817 Worker lines written in 12 days
(`git log --numstat`). None of it is measurable from this box, and none of it appears on a Cloudflare bill.

### 1.5 Current monthly cost, split three ways

| | Free plan (today) | Workers Paid (recommended) |
|---|---|---|
| **Fixed platform** | domain ≈ **$0.93/mo** | $5.00 + $0.93 = **$5.93/mo** |
| **Per hub (marginal)** | $0 until a daily Free limit trips (below) | **$0** until about 100 hubs at today's traffic; beyond that **≈ $0.05/hub/mo** (today's per-hub traffic) to **≈ $0.70/hub/mo** (busy hub). See §1.6 |
| **Per order (marginal)** | $0 | **≈ $0.0002 ≈ 0.015 lek** (ESTIMATED: 250 Worker req + 300 DO req + 2,080 CPU-ms + 30 rows written per delivered order, mostly the tracking sheet polling for about 40 min; priced at overage rates) |

Free-plan daily ceilings against today's MEASURED per-hub load (last 7 days ÷ 2 hubs: 2,069 Worker
req, 3,188 DO req, 37.5 KV writes per hub-day):

| Free limit (per day) | per hub today | hubs until it trips |
|---|---:|---:|
| Worker requests 100,000 | 2,069 | ~48 |
| DO requests 100,000 | 3,188 | **~31** |
| KV writes 1,000 | 37.5 | ~26 |
| CPU 10 ms/invocation | n/a | **already tripping (622 kills)** |

### 1.6 Projection to 10 / 100 / 1000 hubs on Workers Paid (ESTIMATED)

There are two per-hub profiles.

**A, "today".** The last 7 MEASURED days ÷ 2 hubs, ×30.4. Per hub-month: 62.9k Worker req, 523k CPU-ms,
96.9k DO req, 159 GB-s, 47.5k DO rows written, 17k KV reads. Storage: 5.2 MB of KV photos (measured)
and 20 MB of DO storage (assumed).

**B, "busy".** 30 orders a day, console open 12 h, courier app 8 h, 100 storefront visits. That comes to
**15,000 Worker req/hub-day**, 7.25× profile A. Every other line is scaled by the same factor. The
rows-written scaling is the weakest assumption in this model (OPEN 4).

The model is `scratchpad/cost/model.py` (lane scratch), with the pricing page's rates and allowances:
10M req, 30M CPU-ms, 1M DO req, 400k GB-s rounded up per million, 50M rows written, 10M KV reads,
1 GB KV, 5 GB DO SQL, and 20M log events at `head_sampling_rate = 0.1`.

| hubs | A: total $/mo | A: $/hub | B: total $/mo | B: $/hub | the biggest line in B |
|---:|---:|---:|---:|---:|---|
| 2 | 5.00 | 2.50 | 5.06 | 2.53 | none |
| 10 | 5.00 | 0.50 | 6.06 | 0.61 | DO requests $0.90 |
| 100 | 6.75 | 0.07 | 34.24 | 0.34 | Worker req $10.68, DO req $10.39, CPU $6.98 |
| 1000 | **53.77** | **0.05** | **703.52** | **0.70** | **DO rows written $294**, Worker req $134, DO req $105, CPU $75, KV reads $57 |

The domain adds $0.93/mo to every row.

**Where the money goes at scale:** in order, **(1) DO rows written** (every chunk `put` is one billed
row, at $1.00/M), **(2) Worker requests** (client polling, at $0.30/M), **(3) DO requests** ($0.15/M), and
**(4) CPU** ($0.02/M ms). Storage, logs and duration stay small. DO duration is under 2 % because the
object hibernates (MEASURED 83 GB-s in 10 days, against 400k GB-s included per month).

---

## 2. What is not bebop today (MEASURED, `wc -l`)

bebop is already the **storage format**. Every DO image is a bebop store, read and written by the Rust
crate `bebop-store` (MEMORY: bebop-is-dowiz-database). Everything that computes is Rust or JS:

| Component | Language | Lines (non-test) | Tests | Runs where |
|---|---|---:|---:|---|
| `workers/api/src`: 362 files, 170 `/api` routes, 1,349 `.await`, 2,750 `json!`/`serde_json` sites in 282 files | Rust → wasm32 | 47,516 | 16,301 | Worker + DO |
| `crates/dowiz-hub/src` (log, fold, table, redact) | Rust, no_std-ish | 16,839 | ~3,175 | linked by Worker and native server |
| `crates/bebop-store/src` (the format in Rust) | Rust | 2,322 | | Worker |
| `kernel/src` (FSM/money authority, `json-api`) | Rust | 16,728 | | Worker |
| `crates/dowiz-core/src` (the part linked by the Worker was not measured) | Rust | 150,218 total | | Worker (part) |
| `tools/native-spa-server` (a second twin of the Worker, axum) | Rust | 10,721 | | a machine |
| `workers/api/public` JS 26,220 + MJS 4,880 + CSS 5,813 + HTML 820 | JS/CSS | **37,733** | | browsers |
| KV `dowiz-media` | JPEG bytes | 10.4 MB | | KV |
| **bebop itself**: `bebop.bp` compiler 9,122 lines (393 KB); `selfhost/` 34,394 lines of .bp; `std/kv.bp` 117 lines (owns schema creation) | bebop | | | the box (aarch64) |

## 3. What "rewrite everything onto bebop" could mean, and what each version costs

### 3.1 Blockers that gate any rewrite (verified where cheap)

| # | Blocker | Status | Source |
|---|---|---|---|
| B1 | **No wasm backend.** The live compiler emits AArch64 words only | `TASKS.md:103` T94 "WASM direct binary emitter" **OPEN** (the 22/22 wasm parity in `bench/vs_rust/REPORT.md` belongs to the LEGACY pipeline and is marked SUPERSEDED) | TASKS.md |
| B2 | **No x86_64 backend.** Most VPSes are x86 | `TASKS.md:93` T91 **OPEN** | TASKS.md |
| B3 | **No networking.** `bebop.bp`'s syscall emitters are open/read/write/mmap/clone/futex/fsync/rename…, with **no socket/bind/listen/accept** | `grep -o 'emit_sys_[a-z_]*' bebop.bp` | MEASURED |
| B4 | **`str` cannot be built at runtime** (a zeros() buffer used as str = SIGSEGV). Every JSON, HTTP and HTML response is a runtime string | MEMORY: bebop-str-cannot-be-built-at-runtime | |
| B5 | **No JSON, HTTP, TLS, UTF-8 or argon2 in std.** `ls selfhost/std` has no json/http/net module. sha256 exists | MEASURED | |
| B6 | Clone keeps ≤ 8 symbols (silent child loss); multi-core gives 1.00× on B6 | MEMORY: bebop-clone-kept-symbol-limit, bebop-multicore-gives-nothing | |
| B7 | The compiler source is near its own slurp cap. Raised to 1.2 MB, but a 100k-line port grows the compiled unit, and `use` splicing had a SIGSEGV (D5b) | MEMORY: bebop-slurp-cap-headroom, d5b-witness-buffer-segv | |
| B8 | Compute speed: bebop/Rust = **2.6× (fib) to 10.6× (arith chain)**. The FASTPATH "≥1.0× vs Rust" done-check is **UNMET** (no register allocator) | `bench/vs_rust/REPORT-630.md` (CURRENT, 2026-09-03) | MEASURED file |
| B9 | Where bebop is faster: store ops. PK lookup 190 ns vs sqlite 6,542 ns; hash-probe 0.38× of Rust's time (grouped join) | `bench/vs_rust/RESULT-sbench.md`, `ROADMAP.md` D2 | MEASURED file |

### 3.2 Scenarios

| Scenario | What changes | Running cost vs today (Paid, 1000 busy hubs = $704/mo) | One-time effort |
|---|---|---|---|
| **S0: status quo.** Rust Worker, bebop images via `bebop-store` | nothing | **$704/mo** (B), $54 (A) | 0 |
| **S1: bebop compiled to wasm inside the Worker**, replacing the Rust in `workers/api` + `dowiz-hub` + `kernel` | Only the CPU line moves. Requests, DO requests, rows written, KV and storage are identical, because clients poll the same routes and the DO writes the same chunks | CPU $75 → **$227 (×3) to $758 (×10)**, so the total is **$855 to $1,386/mo**: **worse by 22 % to 97 %** (B8 ratios applied to Worker CPU; wasm-vs-native ratio not measured). At profile A: $54 → $75 to $148 | Port ≈ 81k non-test Rust lines + 20k test lines, **plus B1, B4 and B5** (wasm emitter, runtime strings, JSON, UTF-8, HMAC/argon2, SigV4, QR, a JS shim for fetch/DO bindings). ESTIMATE below |
| **S2: bebop images replacing KV blobs** | KV holds only 156 opaque, content-addressed JPEGs (10.4 MB). Putting them in DO images moves bytes into the 128 MB-memory object and turns edge-cached KV reads into DO requests | KV at 1000 B hubs is $57 reads + $2 storage. Moving it adds a similar or larger DO-request line and risks the DO memory limit. **Saves nothing; recommend AGAINST** | small, but negative value |
| **S3: bebop native server on a VPS** (one process, mmap store, no Workers/DO) | Every Cloudflare usage line disappears. TLS via a Cloudflare proxy or Tunnel (free). Photos go to disk | Hetzner CAX11 (Arm64, matches bebop's only backend) **€5.99/mo** excl. IPv4 (https://docs.hetzner.com/general/infrastructure-and-availability/price-adjustment/, price from 15 June 2026). Two boxes for failover plus backup ≈ **€15 to €20/mo** at 1000 hubs. B-load is 456M req/mo ≈ 174 req/s average, so this is feasible only if a request costs well under 5 ms of one core (bebop is single-core, B6). **Saves ≈ $680/mo at 1000 busy hubs, $35/mo at 1000 today-hubs, $0 below ~100 hubs** | S1's port, **plus B2** if not Arm, **plus B3** (sockets, epoll, an HTTP/1.1 parser, WebSocket), plus ops: HA, replication, deploys, monitoring. You lose the edge, per-venue isolation by construction, and Cloudflare's DDoS layer |
| **S3′: the same VPS with the EXISTING Rust `native-spa-server`** | same as S3 | **same saving as S3** | Bring the twin (10,721 lines, 80 shared function names) up to Worker parity. That is a fraction of S3, with no new-language risk. **This is the control S3 must beat, and bebop adds nothing to the saving itself** |
| **S4: front ends to bebop** | Browsers run JS or wasm, and bebop has no wasm emitter (B1) | $0 either way: static assets are free and unlimited | not possible today |

### 3.3 Rewrite effort (ESTIMATED; assumptions stated)

LOC to port (MEASURED): Worker 47,516 + hub 16,839 + kernel 16,728 = **81,083 non-test Rust lines**,
plus ≈ 19.5k test lines. Two exclusions:

- `bebop-store`'s 2,322 lines become unnecessary, since bebop does its own store.
- `native-spa-server`'s 10,721 lines are deleted in S3 or kept in S1.

The linked slice of `dowiz-core` (150k lines in total) is **not measured** and is excluded. That makes
this a lower bound.

Platform prerequisites (none of these exist): wasm emitter or x86 backend, runtime strings, JSON,
UTF-8, HTTP and sockets (S3 only), HMAC-SHA256, argon2id, base64, a date/time and timezone rule set,
QR code, S3 SigV4, and ML-KEM-768 (the operator's PQ decision, MEMORY). Rough size: **15k to 25k bebop lines**.

Velocity assumptions:

- **Rust:** the Worker's 63.8k lines landed in 12 days (MEASURED, `git log --numstat`), about 5k lines a
  day across 3 lanes.
- **bebop:** 93.5k .bp lines added in the last 30 days (MEASURED), but that is compiler, tests and gate
  work with no product surface. For product code in bebop, assume **0.5k to 1k gated lines per
  lane-day**, 3 to 10 times slower than Rust. Reasons: B4 to B7 traps, no library ecosystem, and the
  lane-failure history (MEMORY: lane-verdicts-need-triggering, bpref-models-the-pre-a26-language).

| Part | Lines | Lane-days (0.5k to 1k lines/day) |
|---|---:|---:|
| Prerequisites | 15k to 25k | 30 to 50 (compiler work is slower; B1 alone is a new backend) |
| Port, non-test | 81k | 81 to 162 |
| Port, tests + oracle twins | 20k | 20 to 40 |
| Cut-over and migration (images already bebop, so no data migration) | | 10 to 20 |
| **Total** | **≈ 115k to 125k** | **≈ 140 to 270 lane-days** |

With the 3-lane cap, that is **≈ 2 to 4 calendar months** of the box doing nothing else. For humans,
at ≈ 1k ported lines per person-week into an immature language, it is ≈ **100 to 150 person-weeks**.

## 4. Does a bebop rewrite move the line where the money is?

**No.** The money line today is **$0** (Free plan) plus the domain. At 1000 busy hubs it is DO rows
written, then requests. Neither depends on the language:

- **Requests** are how often browsers poll. They move with WebSocket push (phase 6 exists; the poll is
  kept on purpose) and with longer intervals. That is a JS change.
- **DO rows written** are one row per 96 KiB chunk `put`. They move with fewer chunk writes per event
  (batch appends, or a smaller hot log). That is a layout change in the Rust `hubdo`, and the format is
  already bebop.
- **CPU** is the only language-bound line. bebop makes it worse today (B8), and it is the fourth line.

The only large saving on the table, ≈ $680/mo at 1000 busy hubs, comes from **leaving Workers for a
VPS** (S3/S3′). That is an architecture decision independent of bebop, and the existing Rust native
server gets it for a fraction of the effort. Below ~100 hubs there is nothing to save: the bill is the
$5 floor.

**What to do instead, in cost order:**

1. Move to Workers Paid ($5/mo). This stops the 1.4 % CPU kills.
2. Find which routes hit 10 ms (OPEN 3).
3. Cut polling on the tracking sheet and the console now that WebSockets exist.
4. Batch DO chunk writes before 100 hubs.

---

## OPEN

1. **Real traffic vs QA/lane traffic** cannot be separated. The per-hub profile A includes dogfooding,
   so it is probably an OVER-estimate of a quiet venue.
2. **DO stored bytes are not measured.** `durableObjectsStorageGroups` is empty for this account, so
   20 MB/hub is an assumption. `/api/owner/health` has image gauges but needs owner credentials. Not
   used by this read-only lane.
3. **Which routes the 622 CPU kills hit.** `workersInvocationsAdaptive` has no path dimension. This
   needs Workers Logs or the observability API, which no token here can read.
4. **Rows written per order.** The busy profile scales rows written with requests (7.25×). A direct
   count per delivered order would firm up the $294 line, which is the largest one at 1000 hubs.
5. **Plan status is inferred, not read.** A billing-read token or one look at the dashboard settles it.
6. **bebop→wasm CPU ratio** is unknown. B8's ratios are native AArch64 vs Rust native, and T94 does not exist.
7. **Domain price** ($11.20/yr) is MEMORY, not re-verified. Registrar not checked.
8. The linked size of `dowiz-core` in the Worker is not measured, so the port LOC is a lower bound.
