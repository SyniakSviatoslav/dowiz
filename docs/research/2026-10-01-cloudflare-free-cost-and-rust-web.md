# Cloudflare Free: cost architecture, IndexedDB, server rendering, all-Rust web — research report

Lane W-FRESEARCH, 2026-10-01, session 3e1fb658, model Fable. A research lane: no product code changed.

Operator decisions this report is bound by (2026-10-01): stay on Workers FREE; `tools/native-spa-server`
is deleted (no VPS twin); approved for design: (1) deploy + heavy gates from GitHub Actions, (3) production
observability + Telegram alerts, (4) weekly backup-restore drill, (5) dinner-peak load test on qa-durres.

Every number is tagged **MEASURED** (command + date), **CITED** (URL + date read) or **ESTIMATE** (basis).

Status: COMPLETE 2026-10-01 16:20 UTC. Sections 0–8 written; the measurement arm ran after "BOX FREE": §5.2's Leptos wasm build (slot `fres`, 4 m 06 s), §8 U1 (one GraphQL read), U4 (repo is public), U5 (restore route exists), U6 (half), five live reads on qa-durres. Still open: the per-object DO split (two connect timeouts) and U10/U11's phone measurement. R2 (native-spa-server deletion) was done by main the same day.

## 0. Одна сторінка для оператора (UA)

**Де ми.** Акаунт на Workers Free. Хвилинний cron уже прибрано (FT2, `92f96221`), тож стеля більше не росте з кількістю закладів — вона росте з ТРАФІКОМ. Три стіни, усі добові, на весь акаунт, і всі «закриваються» о 00:00 UTC для всіх закладів разом: запити до об'єктів (100k/день) → запити до Worker (100k/день) → записані рядки (100k/день). За сьогоднішнім профілем «зайнятого» закладу (30 замовлень, консоль 12 год) Free тримає **≈ 3 зайняті заклади**; після рядків нижче — **≈ 50**. 10 мс CPU досі вбиває запити у вечірній пік там, де каталог (538 KB) ще їздить через Worker (37 місць, gate `dataflow` = 38).

**Головний принцип цього звіту:** на Free запит рахується в момент, коли він дійшов до Worker — кеш не допомагає («Workers run before the cache», CITED). Безкоштовні лише статичні файли і читання з R2 через свій домен. Тому: **усе, що можна читати, має читатися не з Worker; а коли ліміт вичерпано — вітрина має показувати меню, а не порожню сторінку.**

**Що робити, в якому порядку (по 3 лейни одночасно, ≈ 3 тижні):**
1. ~~R2 — видалити `tools/native-spa-server`~~ — **зроблено main 1 жовтня** (разом із `crates/dowiz-hub/src/subs.rs`).
2. **A2+A3+A4 — polling через сокет (30 с запас), auto-pong на об'єкті, реєстр у кеші 60 с** (2 дні). Найбільший виграш на день роботи: зайнятий заклад з 15 000 → ≈ 4 000 запитів/день; мінус 3 500 запитів до об'єкта на кожен відкритий сокет; мінус половина запитів до об'єкта на кожен запит.
3. **R3 — деплой і важкі гейти з GitHub Actions** (2 дні, Opus; чекає на секрет `CLOUDFLARE_API_TOKEN` від вас). `tools/deploy/deploy.sh` уже вміє все, крім «не через slot.sh». Репо **публічне** (перевірено через GitHub API 1 жовтня): хвилини Actions безкоштовні, раннери 4 vCPU/16 GB, ARM-раннери для bebop-батареї доступні.
4. **A1+C3 — вітрина читає з R2 (`cdn.dowiz.org`), а не з Worker** (4 дні, Opus): об'єкт закладу публікує меню/i18n/налаштування/фото в R2 при кожній зміні каталогу; `/` стає статичним шеллом (`run_worker_first` геть). Холодний візит: ≈ 22 запити до Worker → 0. І це єдиний захист від «ліміт вичерпано — всі вітрини чорні».
5. **R4 — спостереження + Telegram** (3 дні). На Free немає ні алертів Cloudflare, ні Tail Workers; логи — лише в дашборді, 3 дні. Тому три джерела: (а) Worker сам пише помилки в outbox платформного об'єкта, alarm шле в платформний Telegram-бот (дайджест, ≤ 20/хв); (б) GitHub Actions кожні 15 хв перевіряє `/api/version` і `/` на всіх хостах (≈ 576 запитів/день); (в) нічна таблиця `cf.*` з порогами (60 % від лімітів).
6. **A5 — dataflow 38 → 0** (3 дні, 2×Haiku): кінець 1102 у вечірній пік.
7. **B — IndexedDB** (2 дні): блоки каталогу по `k64`, замовлення (7 днів), чернетки, черга (як є). Повторний візит = 0 запитів до Worker. Персональні дані на спільних планшетах: очищення при виході, 7 днів, рядок у `personal-data`.
8. **R5 — щотижнева перевірка бекапу** (2 дні): тижнева — без запису в продакшн (розпакувати експорт qa-durres у CI, `chain_check`, порівняти лічильники з `health`); місячна — справжнє відновлення у тимчасовий хаб `drill-<міс>`, ціль < 20 хв.
9. **R6 — навантажувальний тест у вечірній пік** (2 дні, з GitHub Actions, не з телефона): профіль «зайнятого закладу» за 60 хв ≈ 3 600 запитів до Worker / 7 000 до об'єкта (3,6 % / 7 % добових лімітів), відмова, якщо день уже > 40 %, стоп на 10 000. Міряє kills, p99, рядки на замовлення.
10. **A8 — login і нічна робота всередині об'єкта** — лише після проби «CPU об'єкта 30 с чи 10 мс» (OPEN 2 досі відкритий).

**Що це економить.** Грошей — нічого (рахунок і так $0); ємності — з ≈ 3 до ≈ 50 зайнятих закладів на одному Free-акаунті; і головне — вітрини не падають, коли ліміт вичерпано. **Коли таки доведеться платити $5:** якщо `cf.worker_requests_day` або `cf.do_requests_day` перевищить **80 000** у день із платними закладами, або онбординг другого закладу за день упреться в 1 000 записів KV (поки не зроблено A1). Назвати треба саме це число.

**Ризики.** CSP (один заголовок уже ламав усі сторінки); R2-домен і токен з правами на R2 (є з 26.09); навантажувальний тест на спільних лімітах — тому запобіжники зашиті в скрипт; GitHub `schedule` може запізнюватись і вимикається після 60 днів без пушів у публічному репо.

**Rust-фронт без JS (D).** Реально, але не зараз і не весь. **Виміряно на нашому тулчейні (1.96.1, профіль Worker, bindgen 0.2.128 + wasm-opt -O):** Leptos hello-world — **66 KB raw / 28 KB gzip**, сторінка у формі вітрини (165 страв із JSON, 4 мови, кошик у мінорних одиницях) — **138 KB raw / 61 KB gzip + 5,7 KB gzip glue ≈ 67 KB** — стільки ж, скільки сьогоднішній JS-шелл (≈ 70 KB gzip); компіляція+інстанціація **0,9–1,6 мс** на A78 — старт не проблема; холодна збірка 4 хв 06 с на боксі; залишаться 3 вендорні скрипти (MapLibre, Stripe.js — за контрактом Stripe, Telegram-віджет або OIDC) і ≈ 2–5 KB руками (SW + лоадер). Гейти переносяться за ≈ 4 дні; `body-fields` взагалі зникає (типи спільні). Але сервер відповідає `json!` у 2 750 місцях — спільні типи потребують спочатку типізувати відповіді (10–20 днів). Разом ≈ 45–60 лейн-днів і **нуль впливу на ліміти Cloudflare**. Рекомендація: після рядків вище — кімната (room) і кур'єр на Rust як доказ (≈ 7 днів), типізація відповідей паралельно, вітрина — коли буде C3 (islands роблять її маленькою), консоль — лише якщо перші два вклалися в оцінку. Таблиця вимірів — §5.2.

## 1. Current cost / limit map on Free — what binds first, at how many venues

### 1.1 What landed since the FREE-TIER blueprint (FT1–FT13), checked against the tree

MEASURED 2026-10-01 (`git log --since=2026-09-26`, `grep` over `workers/api`, worktree at `4dcbc0e1`):

| Row | Status | Evidence |
|---|---|---|
| FT2 minute cron → per-object alarms | **LANDED** | `92f96221` "node-local timers", `d7279037`; `wrangler.toml` has ONE cron `17 3 * * *`; `cf.mjs` counts `type: alarm` |
| FT1a catalogue not shipped to the Worker (hot paths) | **PARTIAL** | `9fa24c24` `/fold/menu` projection (15–20 ms → ~70 µs per read); `d830cccb` `/fold/preps`; `load_catalog(` still has **37** call sites, `dataflow` gate baseline 38 |
| FT3 photos to R2 | NOT | `MEDIA` is still the KV namespace (`wrangler.toml:111`, `media.rs:48,149,237`); no `r2_buckets` MEDIA |
| FT4 login argon2 in the object | NOT | `auth.rs:250` `Params::new(8*1024, 3, 1)` runs in the Worker |
| FT5 nightly fan-out to objects | NOT | the nightly cron still loops venues in the Worker |
| FT6 ping auto-response | NOT | 0 hits for `auto_response` in `workers/api/src` |
| FT7 socket-first polling | NOT | `track.js:38` `POLL_MS = 12_000`; `courier/app.js:1157` 12 s on shift; `admin/core.js:28` `POLL_MS = 15_000` |
| FT8 registry cached 60 s | NOT | `cache.put` only in `storefront.rs`, `media.rs`, `rates.rs` |
| FT9 root without the Worker | NOT | `wrangler.toml:97` `run_worker_first = ["/"]` |
| FT11 ETag + IndexedDB menu | NOT | IndexedDB is used by `lib/outbox.js` only; `replica.js` is `localStorage` |
| FT13 instruments | PARTIAL | `cf.mjs` has `cf.do_alarms_day`, `cf.cron_runs_day`; no `cf.cpu_kills_day`, no `cf.kv_writes_day`, no `cf.do_rows_written_day`, no object-CPU probe |

So the wall that grew with venue COUNT (the minute cron) is gone; every wall that grows with TRAFFIC is still there.

### 1.2 The limits, as read today (CITED, developers.cloudflare.com, read 2026-10-01; the page's own "Last updated" in brackets)

| Line | Free limit | Over the limit |
|---|---|---|
| Workers requests | **100,000/day**, reset 00:00 UTC (limits, Sep 5 2026) | **Error 1027**; `run_worker_first` paths get **429** instead of falling back to assets (static-assets billing, Apr 23 2026) |
| Workers CPU | **10 ms per invocation**, "built-in flexibility" for infrequent overruns (limits) | the invocation is killed (1102) |
| Subrequests | 50 per request; 6 simultaneous connections (limits) | fetch fails |
| Static assets | **"free and unlimited"**; a matched file is served "without invoking Worker code" (static assets, Jul 3 2026); 20,000 files, 25 MiB each; default `Cache-Control: public, max-age=0, must-revalidate` + ETag (headers, Sep 22 2026) | never |
| Cache | **"Cloudflare Workers run before the cache"** (how the cache works, Jul 6 2026) — a cached response on a Worker route still costs a Worker request; the Cache API runs inside the Worker (cache, Aug 14 2026); HTML/JSON are not cached by default (default cache behavior, Sep 14 2026); Cache Rules: 10 on Free (cache rules, Aug 14 2026) | — |
| Durable Objects | **100,000 requests/day**, 13,000 GB-s/day, 5 M rows read, **100,000 rows written/day**, 5 GB; `setAlarm()` = 1 row written; auto-response messages not charged (DO pricing, Sep 30 2026). CPU "30 seconds (default)" per request AND the FAQ "same per invocation CPU limits as any Workers" (DO limits, Jun 1 2026) — still the OPEN 2 contradiction | "further operations of that type will **fail with an error**" |
| KV | 100,000 reads/day, **1,000 writes/day**, 1 write/s per key, 1 GB (KV limits, Apr 21 2026) | the write fails |
| R2 | 10 GB, **1 M Class A** (PutObject…), **10 M Class B** (GetObject…) per month, egress free; a custom domain "allows you to use Cloudflare Cache"; the zone must be in the same account (R2 pricing Oct 1 2026; public buckets Sep 25 2026) | monthly, not daily |
| Workers Logs | 200,000 events/day, **3-day** retention, dashboard only, `invocation_logs` toggle (Sep 30 2026); real-time logs max 10 tail clients (Jun 25 2026); **Tail Workers are Paid-only** (Jun 25 2026) | sampled |
| Notifications | no Workers alert type is listed as available on Free (notification-available, Apr 24 2026) | — |
| Snippets | **not available on Free** (Aug 14 2026) — the "cheap path before the Worker" does not exist on this plan | — |
| Rate limiting rules | **1 rule** on Free, IP only, 10 s period, fields Path + Verified Bot (Aug 25 2026) | — |
| Workers Builds | Free **3,000 min/month, 1 concurrent build, 20-minute timeout**, 2 vCPU / 8 GB (May 29 2026) | — |

### 1.2b Live reads on qa-durres (MEASURED 2026-10-01 ~15:50 UTC, five GETs + two HEADs, read-only)

`GET /` 200, 2,538 B (br) in 0.38 s; **`GET /api/version` → 404** (the `d830cccb` build that serves it is not live: the deploy is still blocked by the uplink — memory `deploy-upload-fetch-failed`); `GET /app.js` 200, 5,464 B br, `cf-cache-status: HIT`, `cache-control: public, max-age=0, must-revalidate`, weak ETag — exactly the static-assets default the headers page describes, so every revisit of a static file is a conditional request answered at the edge for free; `/api/menu` → 404 on this host (`cache-control: private, no-store`): the menu path is `/api/public/locations/<slug>/menu` in the kit (not probed further — budget); `/manifest.webmanifest` 200 in 1.3 s. The box's connect time to Cloudflare was 0.25–1.3 s on these; two GraphQL POSTs to `api.cloudflare.com` timed out at connect.

### 1.3 What binds first, and at how many venues

**Measured last 24 h (U1, §8):** 1,250 Worker requests, **18,386 object requests** (of which 1,506 alarms), 77 object errors, 250 MB object→Worker bytes, CPU p50 6.8 ms / p99 84 ms. The object line is at **18 % of its daily cap with two live hubs plus QA**, and it is 14.7× the Worker line — the 09-26 model's r ≈ 2 is wrong for today's traffic mix. The per-object split could not be read from the box (connect timeouts); it is the first number R4's nightly table must print.

Inputs: the FREE-TIER blueprint's MEASURED per-hub profile A (1,349 Worker requests per hub-day with cron removed; r ≈ 2 object requests per traffic request; 1,563 rows written/hub-day; 37.5 KV writes) and profile B (busy: 15,000 Worker requests, ≈ 1,500 rows written, 100 cold visits × 18 photos), both ESTIMATE-carried from 2026-09-26 (re-measuring needs the GraphQL token through a shell — §8).

With FT2 landed, the formulas lose the cron terms: Worker = w·V ≤ 100k; DO ≈ r·w·V + alarms ≤ 100k; rows = 1,563·V (A) or ≈ 1,500·V (B) ≤ 100k; KV writes 37.5·V ≤ 1,000 on a quiet day, **one photo import ≈ 822 writes** (MEASURED 09-18).

| Line | Today (ESTIMATE from the 09-26 profiles, FT2 applied) | Binds at |
|---|---|---|
| **DO requests** (r ≈ 2) | A: 2,698·V; B: ≈ 30,000·V | **A ≈ 37 hubs; B ≈ 3 hubs** — FIRST |
| Worker requests | A: 1,349·V; B: 15,000·V | A 74; B 6 |
| Rows written | ≈ 1,500·V | ≈ 64–66 |
| KV writes | 37.5·V steady; 822 per onboarding | **1–2 onboardings per day**, steady 26 hubs |
| CPU 10 ms | the 538 KB catalogue still crosses the hop on 37 sites, `argon2` on every login, the nightly at ≈ 105 ms/venue | **tripping now** on the remaining paths (622 kills over 09-16..09-25, MEASURED then) |

The order of walls is unchanged from the 09-26 blueprint: **object requests → Worker requests → rows written**, and every one is a daily, account-wide, fail-closed cap. The number to watch is `cf.do_requests_day` (collector exists).

### 1.4 The one property no row removes

Every cap above is per ACCOUNT and resets at 00:00 UTC. One crawler, one load test, one QA sweep spends the day's budget for every venue at once. On Free the only shields are: **1 rate-limiting rule** (IP, 10 s window, path match) — enough to stop one runaway client, not a distributed crawl; static assets and R2 reads, which have no daily cap; and taking the storefront's read path off the Worker (§2 A1–A3), so that the day a cap trips, browsing still works and only writes fail. That last one is the design principle of §2: **a cap that trips must degrade to read-only, never to blank.**

## 2. Architecture decisions A — ranked by requests/CPU saved per lane-day

The principle, from §1: on Free, **a request is counted the moment it reaches the Worker, cache or not**, so the only ways to make a request cheaper are (a) not to send it, (b) to answer it from a static asset or an R2 custom domain, or (c) to make it one object request instead of two. Everything below is one of those three. Effort: S ≤ 1 lane-day, M 2–3, L ≥ 4. "Saved" is per busy hub-day (profile B) unless stated; the ratio is what ranks the row.

| # | Decision | Saves (ESTIMATE, basis named) | Effort | Ratio |
|---|---|---|---|---|
| **A1** | **The storefront's whole read path off the Worker**: `/` becomes a static shell (FT9); the venue's menu, i18n, settings and photos are published by the hub object to **R2 behind `cdn.dowiz.org`** at every catalogue generation (one `PutObject` per changed file, Class A 1 M/month) and read from there with `immutable` content-addressed names. The shell reads `cdn.dowiz.org/v/<slug>/manifest.json` (tiny, `max-age=30`) and then the named blocks | A cold visit today ≈ 1 root + 1 menu + 1 i18n + 1 manifest + 18 photos ≈ **22 Worker requests** (COSTS 2026-09-20 Playwright count: 40 responses, 18 `/media`, 20 static) → **0** for browsing; only placement/tracking remain. B: 100 visits × 22 = **−2,200 Worker req/hub-day** and the matching object reads | M (object: write-on-generation ~120 lines; shell ~80 lines JS; `_headers`/CSP `img-src connect-src cdn.dowiz.org`; `attach-host.sh` for the R2 domain) | ~900/day per lane-day; also the **fail-closed defence**: when a cap trips, every storefront still renders |
| **A2** | FT7 socket-first polling with 30 s fallbacks on all three surfaces | B 15,000 → ≈ 4,000 Worker req/hub-day (FT7's own estimate from the intervals) | S | ~11,000 per lane-day — the biggest single ratio, pure JS |
| **A3** | FT6 `set_websocket_auto_response(ping→pong)` | −1,728 object req/day per open socket; ≈ −3,500/hub-day for a console + courier | S (one call in `hubdo.rs` `accept()`) | ~3,500 per lane-day |
| **A4** | FT8 registry/identity through the Cache API for 60 s | r 2 → 1: halves object requests per traffic request; B after A2: ≈ −4,000 object req/hub-day | S | ~4,000 per lane-day |
| **A5** | **Finish FT1 (dataflow 38 → 0)**: the remaining `load_catalog(`/`hubstore::load(` readers ask the object for the derived answer | ends the 10 ms kills in traffic (MEASURED 09-24: every kill-hour had ≥ 100 MB object→Worker bytes); CPU p50 10–13 ms → 1–4 ms on those paths | M×2 (W-DF did 3 sites in one lane; ~12 sites per lane-day at that pace) | correctness, not count: this is the row that stops customers seeing 1102 at dinner |
| **A6** | **Tracking by socket only for the customer**: `track.js` opens `/api/live` with the order token (tag `order:<id>` already exists, phase 6) and polls only when `due()` | a 40-min tracked order: ≈ 200 polls → ≈ 27 (socket up) — included in A2's number but the customer surface is the one with no socket today | S | part of A2 |
| **A7** | **Writes batch into the object's own turn** (blueprint P1): placement = ONE object command that appends, holds stock and updates the table in one turn (`one-image.sh` 3 → 0) | −2 object requests and −2 rows written per placement; −1 per courier action; B: ≈ −150 object req and −100 rows/hub-day | L (the P1 row, ~400 lines) | low ratio; do it for `one-image`, not for the cap |
| **A8** | **Nightly in the object (FT5)** and **login in the object (FT4)** | CPU rows: 210 ms → <1 ms cron; login off the allowance | M + M | correctness; both depend on OPEN 2 (object CPU) — run FT13's probe first |
| **A9** | **Photos: FT3's interim** (mime in KV metadata: 2 writes → 1) until A1 moves them to R2 | onboarding limit 1–2 → 2–4 venues/day; A1 makes it 0 KV writes | S | one-liner, then obsolete |
| **A10** | **Protect the caps**: the one Free rate-limiting rule on `/api/*` per IP per 10 s (block at, say, 60); `robots.txt` + `X-Robots-Tag: noindex` on `/api`; the storefront shell's menu read never retries faster than 30 s | prevents one client from spending the day's budget; the rule is zone-level and runs before the Worker | S (dashboard/API: `Workers Routes` token cannot do it — operator or a zone-rules token) | insurance |
| **A11** | **Workers Builds instead of the phone for the UPLOAD only** (see §6 item 1): Free 3,000 min/month, 1 concurrent, **20-minute timeout** (CITED May 29 2026) | removes the 15 KB/s uplink from the deploy path (MEASURED: a 1.8 MB upload hung 6 min); GitHub Actions keeps the gates | — | the §6 design chooses GitHub Actions for everything because of the 20-minute cap; Workers Builds is the fallback if GitHub minutes run out |
| **A12** | **Do NOT**: move photos into DO images (S2 of the COSTS doc: adds object requests); batch the order log in memory (FT10: durability); buy Paid for capacity (operator: Free); Snippets (not on Free); Cache Rules for `/api` (Workers run before the cache; a rule changes nothing for a route the Worker owns) | — | — | refused with the number |

**Order by ratio:** A2 → A3 → A4 → A1 → A5 → A9/A10 → A6 → A8 → A7. A1 is fourth by ratio but FIRST by what it protects (the fail-closed property), which is why §6's lane order puts it with the CI lane: both are "the product keeps working when a thing we do not control fails".

**Capacity after A1–A5 (ESTIMATE, same profiles):** B hub-day ≈ 1,800 Worker req (placement, tracking polls when due, console/courier polls when due, logins), ≈ 1,800 object req, ≈ 1,300 rows → **≈ 50 busy hubs** per account, bound by Worker and object requests together; A profile ≈ 65 (rows). Same ceiling as the 09-26 blueprint's "45 busy / 64 quiet" — the gain of A1 is not capacity, it is that **browsing never fails closed**.

**CPU per request after A5:** every remaining Worker path is HMAC verify + one object round trip + a JSON pass-through (R1/R2 are bytes-out already): 1–3 ms (MEASURED light hours on 09-24: p50 1.0–1.7 ms, p90 4–17 ms). The 10 ms wall stops being a wall for traffic; only argon2 (A8) and the nightly (A8) remain above it, and both go into the object once OPEN 2 is settled.

## 3. Client data in IndexedDB (B)

### 3.1 What the device holds today (MEASURED 2026-10-01, by reading the files)

| Store | Where | Shape | Persistence | Surface |
|---|---|---|---|---|
| `dowiz.replica.v1` | **`localStorage`** (`lib/replica.js:25-40`) | `{venue, generation, at, orders[]}`; `apply(changes, generation)` folds `/fold/changes?since=` deltas with the same `_d`/`_x` merge as `fold.rs`; unknown order ⇒ `null` ⇒ full read | survives reloads; `forget()` on logout | console |
| `dowiz.outbox` | **IndexedDB** (`lib/outbox.js:40-42`), store `queue`, `keyPath: seq autoIncrement` | `{route, method, body, key, queued_at, tries, tag, last_status}`; `MAX_QUEUE = 64`, refuses the NEW tap when full; key minted at tap time; drains in order, stops at the first unresolved entry; 409-with-Retry-After waits, other 4xx dropped and reported | `forget()` on sign-out | courier, room (one DB name per app) |
| shell | Cache API via `sw.js` (`SHELL_CACHE 'dowiz-shell-2026-09-27-ru'`), "NETWORK FIRST, ALWAYS", the static module graph only | per deploy version | storefront (+ `kit/sw.js`, courier) |
| order tokens, language, drafts | `localStorage` (`store/state.js`, not re-read here) | — | — | storefront |

So reads survive an outage on one surface (console), writes on two (courier, room), and the storefront keeps nothing but its shell: a customer who reopens the menu underground sees the venue's offline line, not the menu (`sw.js` header says so deliberately: "a stale price is the failure this product exists to prevent").

### 3.2 What to keep on the device, per surface

The rule that decides it: **keep what the server already names by generation; never keep a price without the generation it was true at.** The DAG blueprint gives the names: a catalogue generation, DG7 columnar blocks with `K64` keys, `/fold/changes?since=`. On the device that becomes four stores in ONE database per app (`dowiz.<app>`), all keyed by venue:

| Store | Key | Value | Who writes | Bound |
|---|---|---|---|---|
| `blocks` | `(venue, block_id)` | the block's bytes + `k64` + `generation` + `at` | A1's manifest fetch (`cdn.dowiz.org/v/<slug>/manifest.json` names the blocks) | ≤ 2 MB per venue (the 538 KB catalogue is 8× amplified; DG3 v2 packing ≈ 70 KB per language) |
| `orders` | `(venue, order_id)` | the folded order JSON + `generation` | the replica fold (today's `replica.js` moved from `localStorage`) | last 200 orders or 7 days, whichever is smaller |
| `drafts` | `(venue, draft_id)` | basket / amendment in progress, `updated_at` | the surface, on every change | ≤ 20; dropped at 7 days |
| `queue` | `seq` | the outbox entry (unchanged) | `outbox.js` | 64 (as today) |

Storefront: `blocks` (menu, i18n, settings, the venue's own record) + `drafts` (the basket) + `orders` (the customer's own, by order token) + `queue` (placement is NOT queued: a placed order needs the server's price and stock in the same turn; the basket is the draft, the tap is online-only — the courier's taps are idempotent transitions on an order that exists, a placement is not). Console: `blocks` + `orders` + `queue` (owner actions ARE idempotent transitions — P2 extends `idempotency::begin` to `owner::order_action`). Courier: `orders` (its own) + `queue`. Room: `drafts` + `queue`.

### 3.3 How it syncs

1. **Blocks (catalogue, i18n, settings)** — pull, by manifest. The shell reads the manifest (one R2 read, `max-age=30`); for each block whose `k64` differs from the stored one it fetches the block (R2, `immutable`). The browser recomputes `k64` over the bytes (`crates/bebop-wasm` reader, DW1's "replica disagrees" line) — a block that does not hash is dropped, loudly. **Zero Worker requests** for a menu that has not changed; one R2 read per changed block when it has.
2. **Orders** — `GET /fold/changes?since=<generation>` through the Worker (one request), applied with `replica.apply`; a gap or an unknown order triggers the full `/fold/orders` read exactly as today. Events over the socket are applied as predictions (today's rule). This path stays on the Worker because it is per-principal and authorised.
3. **Queue** — drains as today; the `orders` store applies the queued tap as a prediction (status advanced locally with a `pending` mark) so the courier sees the tap, and the next `changes_since` replaces it.
4. **Drafts** — never synced; a draft is this device's.

Cost in requests: a storefront revisit within the manifest TTL = 0 Worker + 0 R2 (the manifest is in the HTTP cache); after it = 0 Worker + 1 R2; a console reopen = 1 Worker (`changes_since`). That is FT11's "feel" gain plus A1's request gain in one design.

### 3.4 Eviction and quota (CITED web.dev "Storage for the web", read 2026-10-01)

- Chrome/Chromium: "an origin can use up to 60% of the total disk space"; eviction is **best-effort, whole-origin, least-recently-used first** when the device runs low. Safari: ≈ 1 GB then a prompt in 200 MB steps, and **"a seven-day cap on all script writable storage"** for sites that are not installed to the home screen. Firefox: 10% of disk, 2 GB per eTLD+1 group.
- Low-end Android (ESTIMATE from the Chrome rule): a 32 GB phone with 2 GB free gives an origin ~1.2 GB of quota, but the eviction trigger is *device* low-space, which such phones hit routinely. Design for **≤ 5 MB per venue per app** so the origin is never the biggest LRU candidate, and for the whole origin disappearing: every store is a cache of something the server holds, except `queue`, which already REFUSES rather than silently losing a tap (`outbox.js:34-38`).
- Ask `navigator.storage.persist()` on the courier and console (installed, long-lived) and read `navigator.storage.estimate()` into `/api/owner/health`'s client gauges; never on the storefront (a prompt on a customer's phone is a lost order).
- Eviction we do ourselves, at open: blocks of other venues than the current host (the storefront's origin is per venue, so this is the console's case), orders older than 7 days, drafts older than 7 days, and anything whose `generation` is below the manifest's `min_generation` (the server's way of saying "this layout changed").

### 3.5 Privacy on shared devices

The console and the room run on shared tablets; the courier phone is personal but changes hands. Rules, each with the check that enforces it:

- `orders` holds names, phones and addresses: it is **personal data on the device** and must be in the `personal-data` registry as a retention row (`tools/gates/personal-data.sh` counts registered subjects; the client store is a new subject class "device cache"). Retention: 7 days or sign-out, whichever first — the same as `replica.forget()` today.
- **Sign-out clears every store of that app** (today: `replica.forget()` and `outbox.forget()` — keep that contract; one `clear()` per store in the same transaction).
- The storefront holds the customer's OWN orders only, keyed by order token; a shared family phone sees the orders placed from it, which is the status quo of `tokenFor(order.id)` in `localStorage` (`track.js:75`).
- No credentials in IndexedDB: tokens stay where they are; the outbox already asks `authorize()` at drain time, not at tap time, so a tap never replays under a later person's token (`outbox.js:139-144`).
- The DG10 crypto-shredding key never reaches the device: a `forget` on the server leaves the device holding a copy until its 7-day retention passes — say so in the DPIA row; or, the stricter option, `changes_since` carries `Forgotten` events and the replica deletes the order on receipt (DW1's "replica is a subscriber" makes this free).

### 3.6 RED-first checks for the B lane

1. `node --test` on a new `lib/store.js` (IndexedDB wrapper): open fails ⇒ every read answers "no store", every write answers `{ok:false, reason:'nostore'}`; quota abort during `put` ⇒ the transaction's `complete` never fires and the caller is told (the `tx` pattern in `outbox.js:119-131`, reused, not copied).
2. Playwright (`e2e/kit-regression/`): storefront cold visit, then network off, reload ⇒ the menu paints from `blocks` within 100 ms with the "offline — prices as of <time>" line; a changed `k64` in a corrupted block ⇒ "replica disagrees" visible and the block not drawn.
3. `sw-shell.sh` unchanged (the shell list grows by `lib/store.js`); `personal-data` baseline +1 subject class, `.prove.sh` fires when the registry row is removed.
4. Acceptance numbers: Playwright revisit Worker requests = **0** (menu), console reopen = **1** (`changes_since`); `navigator.storage.estimate().usage` for one venue ≤ **5 MB** after a 165-dish catalogue in 4 languages (the sushi fixture).

## 4. Smaller payloads and server rendering (C)

### 4.1 What a storefront visit costs today (MEASURED 2026-10-01 on the tree; wire counts from the 09-20 Playwright run)

- `store/index.html` loads `/app.js` + 3 stylesheets + `store/store.css` + `lib/ui/ui.css`; the static import graph from `/app.js` is **40 modules, 1,276,666 B** on disk including the dynamic imports (`lib/map` is 1,023,946 B of that — MapLibre, loaded only when the map opens). The shell itself (static imports) is ≈ 250 KB raw before gzip (ESTIMATE: graph minus `lib/map`). Admin: 92 modules, 783,573 B; courier: 14 modules, 169,271 B; room: 4 modules, 39,984 B.
- The menu is JSON from `/api/menu` (edge-cached 30 s, 1 Worker request) rendered by `store/menu.js`; i18n from `/api/menu/i18n`; 18 photos through `/media/*` (each a Worker request + KV). All of the HTML a customer sees is built in the browser.
- The Worker wasm is **4,747,270 B raw / 1,840,961 B gzip** (`/root/dowiz/workers/api/build/index_bg.wasm`, built Sep 30, `gzip -c | wc -c`); it never reaches a browser.

### 4.2 The four ways to render, costed on Free

| Option | Who renders | Requests per view (Worker / object / R2) | CPU per view | Changes per menu edit | Verdict |
|---|---|---|---|---|---|
| **C0 today**: client render from `/api/menu` JSON | browser | 1 / 1 (cached 30 s at the edge, still a Worker request on every hit — Workers run before the cache) / 0; + 18 media | Worker ≈ 1 ms after R2 (bytes pass-through); browser: parse + build 165 cards | none | the baseline |
| **C1 SSR in the Worker per request** (Rust templates, `worker` crate) | Worker | 1 / 1 / 0 | template fill of a 165-dish page: ESTIMATE 1–3 ms (string building; the 9fa24c24 projection answers the JSON in ~70 µs, HTML would be the same order), inside 10 ms; argon-free paths only | none | **AGAINST**: the same request count as C0 and more CPU; buys first paint only |
| **C2 SSR memoised in the object** (`/fold/menu.html?locale=`, rendered once per generation like `/fold/menu`) | object | 1 / 1 / 0 | Worker ≈ 0.5 ms pass-through; object ≈ 70 µs after the first | one render per generation | better than C1, still 2 counted requests per view; the step before C3 if R2 is refused |
| **C3 prerender at publish into R2 behind `cdn.dowiz.org`** (A1): the object writes `menu.<lang>.html` fragment + `manifest.json` + blocks at every generation | object, at publish time | **0 / 0 / 1** (and 0 R2 on a cache hit at the edge — UNVERIFIED whether cached custom-domain reads are billed as Class B; §8) | 0 in the Worker; browser inserts a fragment (no card building) | 1 Class A op per changed file: ~6 per edit; a venue editing 10×/day ≈ 1,800 Class A/month → the 1 M/month cap holds ≈ 500 such venues | **FOR** — the only option with 0 counted requests |
| **C4 prerender into static assets** (`public/`) | CI at deploy | 0 / 0 / 0 (static assets are free, unlimited, served without the Worker) | 0 | **a `wrangler deploy`**: assets are part of a Worker version; a menu edit would need a CI run (minutes, GitHub minutes, the 20,000-file cap shared by every venue: 165 dishes × 4 languages × N venues) | **AGAINST for venue content**; **FOR for platform pages** (`dowiz.org` landing, learn, wiki, the storefront SHELL): those change only at deploy |
| **C5 HTML streaming** (`Response::from_stream`, `wasm-streams` is already in the dependency graph) | Worker | same as C1 | **no CPU saving**: only wall time waits are free; streaming moves bytes earlier, it does not render them cheaper | — | not a cost lever; useful once for the long export/health pages (C1 class) |
| **C6 islands** (static HTML + small hydrated regions) | C3 for the HTML, browser for the islands | as C3 | browser runs only the islands' code | as C3 | the shape C3 already has: the basket, the order sheet and tracking are the islands; the menu is static. Whether the islands are JS or Rust/wasm is §5 |

### 4.3 Payload sizes (ESTIMATE, basis: the sushi fixture of 165 dishes)

- Prerendered menu fragment: 165 cards × ≈ 400 B of markup ≈ 66 KB raw, **≈ 12 KB gzip/brotli** (markup compresses 5–6×). The JSON it replaces is ≈ 70 KB raw (FT1's number) — bytes on the wire are about equal; what moves is the **browser work on a low-end phone** (no JSON parse, no 165 `innerHTML` builds before first paint) and the Worker request.
- Photos: content-addressed names let `Cache-Control: public, max-age=31536000, immutable` do its job (the KV path already caches 30 s); `?variant=small` (FT3) ≈ 20–40 KB each; 18 of them ≈ 0.5 MB per cold visit, the dominant bytes either way — **that is the payload to shrink first**, by serving the small variant on the list and the full one on the card only.
- The shell: 250 KB of JS raw is ≈ 70 KB gzip (ESTIMATE 3.5×); it is cached by `sw.js` after the first visit. Splitting `store/menu.js`'s card builder out (C3 makes it unnecessary for the first paint) and deferring `sea.js`/`particle-cloud.js` behind `requestIdleCallback` is the cheap trim; MapLibre stays dynamic.

### 4.4 What C3 costs to build, and its gates

- **Owns:** `workers/api/src/hubdo/menu.rs` (render the HTML fragment per locale beside the JSON projection; the template is a pure function in `crates/dowiz-hub` with tests, so the bytes are golden-pinned), `+workers/api/src/hubdo/publish.rs` (write manifest + blocks + fragments + photos to R2 `CDN` binding on generation change; retried by the object's alarm on failure — the same outbox shape as `rails.rs`), `wrangler.toml` (`r2_buckets` CDN), `public/index.html` + `+public/store/shell.js` (host → slug → manifest → fragment; falls back to today's `/api/menu` path when the manifest is missing — so a venue whose publish failed still renders, through the Worker), `public/_headers` (CSP `connect-src img-src https://cdn.dowiz.org`), `tools/platform/attach-host.sh` (the R2 custom domain), `tools/gates/dataflow.sh` unchanged.
- **RED first:** `menu::tests::fragment_bytes_pinned` (golden), `publish::tests::only_changed_files_are_written` (a generation that changes one dish writes one fragment per locale and the manifest, not the photos), Playwright `e2e/kit-regression/storefront-r2.mjs`: cold visit Worker request count **0** for browsing, then placement **1**.
- **Acceptance:** Worker requests per cold storefront visit ≤ 1 (from ≈ 22); `cf.worker_requests_day` on a browse-only day on qa-durres < 50; the menu paints with `/api/*` blocked at the proxy (the fail-closed property, Playwright route-blocking).
- **Risks:** R2 publish lag (a dish price edit shows after the manifest's `max-age=30` plus the write — say "published" in the console only after the manifest write returns); the R2 custom domain needs the zone's DNS write that `attach-host.sh` already does for Workers domains, and a token with `Workers R2 Storage:Edit` (the deploy token got R2 write on 2026-09-26, memory `dowiz-operator-unblocks-2026-09-26`); CSP changes are the one header that broke every page once (memory `dowiz-csp-blocked-every-stylesheet`) — the four greps from that note are the check.
- **Effort:** M–L (≈ 3–4 lane-days: object publish 1.5, shell 1, R2 domain + CSP + Playwright 1).

## 5. All-Rust web front-end, no hand-written JS (D) — verdict with measured basis

### 5.1 What there is to replace (MEASURED 2026-10-01, `find`/`du` over `workers/api/public`)

| Surface | Entry | Modules in the static+dynamic graph | Bytes on disk | Notes |
|---|---|---|---|---|
| storefront | `/app.js` | 40 | 1,276,666 (of which `lib/map` 1,023,946 — MapLibre, dynamic) | ≈ 250 KB without the map; `sea.js`, `particle-cloud.js` are WebGL |
| console | `/admin/app.js` | 92 | 783,573 | the largest surface; replica, i18n, 20+ screens |
| courier | `/courier/app.js` | 14 | 169,271 | outbox (IndexedDB), GPS, socket |
| room | `/room/app.js` | 4 | 39,984 | already shares the amend/pay deciders through `crates/bebop-wasm` (D7 phase 1) |
| landing | `platform/index.html` + `landing-words.js` | — | 24,623 + 37,858 (+13.5 MB video, 795 KB img) | content, not an app |
| whole tree | | 236 `.js` (3,220,924 B) + 77 `.mjs` (440,523 B, mostly tests) + 30 `.css` (525,259 B) + 9 `.html` | 20 MB incl. `platform/video` | 26,220 JS + 4,880 MJS + 5,813 CSS lines (COSTS doc, 09-26) |

### 5.2 Measured basis (wasm size and startup)

**CITED (read 2026-10-01):** `book.leptos.dev/islands.html`: a Leptos "Hello, world" is **274 KB** of wasm as a hydrated app, **24 KB** in islands mode with nothing interactive, **166 KB** with one island — "your WASM binary grows as a function of the amount of interactivity"; islands "require server-side rendering". Secondary (pistack.xyz 2026-08-29, one dashboard built three times, gzip): **Leptos 86 KB, Yew 180 KB, Dioxus 210 KB**; the Leptos README: "Leptos tends to prioritize holistic web performance (streaming HTML rendering, smaller WASM binary sizes)". The krausest js-framework-benchmark page is script-rendered and could not be read from here (U-list); an HN thread cites "Leptos is even worse than React in memory consumption and startup metrics" without numbers.

**MEASURED on this box's A78 cores (binary-size research 2026-09-27 §7.2, Node 26.4 / V8 14.6):** the 4,296,128 B Worker wasm takes **91–94 ms CPU to import + compile + instantiate**. That is the one startup number we own: ≈ 22 ms per MB of wasm on this core class, before tier-up.

**MEASURED here (slot `fres`, 2026-10-01 15:49–16:01 UTC, session scratchpad `fres-leptos/`):** a Leptos **0.8.10** CSR crate (`leptos_dom 0.8.8`) with two bins — `hello` (one counter) and `menu` (165 dishes serde-parsed from JSON, four languages, a basket with integer-minor-unit totals) — built with the repo's pinned `1.96.1` for `wasm32-unknown-unknown`, `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"` (the Worker's profile). Cold `cargo build --release`: **4 m 06 s** on the box (slot held 248 s). Pre-bindgen, pre-opt: **`hello.wasm` 1,331,948 B, `menu.wasm` 1,449,977 B** — i.e. the product's shapes (serde_json + a 165-row `<For>` + signals) add **118 KB** to the framework floor. The post-`wasm-bindgen 0.2.128` + `wasm-opt -O` (+ a `--strip-debug --strip-producers` variant) sizes, gzip, and the Node 26 compile+instantiate CPU on the A78 are in the table below (task `bmy90z90a`; filled in the next edit).

| Artefact (MEASURED 2026-10-01 16:15 UTC, `measure.mjs` under Termux Node 26.4 / V8 14.6, median of 5) | raw B | gzip -9 B | compile+instantiate CPU ms (A78) |
|---|---:|---:|---:|
| `hello.wasm` straight from cargo (debug/name sections still in) | 1,331,948 | 315,452 | 19.6 |
| `hello_bg.wasm` after `wasm-bindgen --target web` | 118,812 | 41,240 | 1.06 |
| **`hello_opt.wasm` after `wasm-opt -O`** (what would ship) | **66,467** | **27,971** | **0.93** |
| `hello_strip.wasm` (`-O --strip-debug --strip-producers`) | 66,341 | 27,880 | 1.9 (noise: runs 1.2–3.4) |
| `menu.wasm` straight from cargo | 1,449,977 | 359,000 | 19.9 |
| `menu_bg.wasm` after `wasm-bindgen` | 226,901 | 82,671 | 1.47 |
| **`menu_opt.wasm` after `wasm-opt -O`** (what would ship) | **137,985** | **60,818** | **1.61** |
| `menu_strip.wasm` | 137,859 | 60,721 | 1.57 |
| glue `hello.js` / `menu.js` (generated, `--target web`, unminified) | 22,485 / 26,390 | — / 5,696 | — |

Three readings. (1) **The cargo output is not the number**: bindgen + `wasm-opt -O` take 1.45 MB to 138 KB; the citations' "274 KB hello world" is a pre-opt or older-toolchain figure — on this toolchain and profile the floor is **66 KB raw / 28 KB gzip**. (2) **A storefront-shaped page is 138 KB raw / 61 KB gzip + 5.7 KB gzip of glue ≈ 67 KB gzip on the wire** — the same size as today's JS shell (≈ 70 KB gzip, ESTIMATE §4.3), not 2–3× it. (3) **Startup is ≈ 1–2 ms** of compile+instantiate on this core class for either bin (the 4.3 MB Worker took 91–94 ms: the cost is linear in bytes), so on an A53-class phone it is single-digit milliseconds — not a factor. Caveats: `wasm-opt -O` here was the Worker's flag set (`--all-features`), not `-Oz` (the strip research found `-Oz` worse for gzip anyway); the menu bin renders but was not driven in a browser (no DOM test on this box: the measurement is bytes and instantiate CPU, not frame time); the DOM work of 165 `<li>` creations through `web-sys` on first render is the UNMEASURED part (U11).

**ESTIMATE replaced by the measurement:** a full Leptos CSR storefront (menu + basket + checkout + tracking, without the map and the Sea) ≈ 3–4× the `menu` bin's own code ≈ **250–400 KB raw / 110–170 KB gzip** — still in the band of today's JS, and in islands mode on C3's HTML well under it.

(The pre-measurement ESTIMATE from the citations — 400–700 KB raw, 2–3× today's shell — was **wrong by 3×**; the measured floor and slope above supersede it.)

### 5.3 What must stay JS, by contract or by absence of a Rust path

| Part | Why | Rust-side shape |
|---|---|---|
| wasm-bindgen glue | generated, not hand-written: ≈ 30 KB (the Worker's `index.js` is 33,154 B) | none; "no HAND-WRITTEN JS" holds, "no JS" does not |
| `sw.js` (shell cache) | a service worker is a JS entry by the platform; the logic is ≈ 100 lines | keep as hand JS, or generate it from the build (the shell list is the build's file list — `sw-shell.sh` becomes trivial) |
| MapLibre GL JS (`lib/map`, 1 MB) | no production Rust vector-tile renderer for browsers; `maplibre-rs` is experimental | call through `js-sys`/`web-sys` bindings from Rust; the library stays |
| Stripe.js | "it should always be loaded directly from `https://js.stripe.com`, rather than included in a bundle or hosted yourself" (CITED docs.stripe.com/js/including) | `web-sys` bindings; the script tag stays |
| Telegram login | the widget is a JS library; the alternative is OIDC redirects (CITED core.telegram.org/widgets/login: "Telegram supports the standard OpenID Connect protocol") | OIDC removes the JS; the Worker already does HMAC verification |
| WebGL Sea, particle cloud | can be Rust (`web-sys` WebGL2), but nothing on this box can see a frame (memory `webgl-renders-nothing-on-the-box`) | port last; it is decoration |
| camera / QR | `getUserMedia` through `web-sys`; QR decode in Rust (`rqrr`) works in wasm | Rust |
| voice | `SpeechRecognition` through `web-sys` | Rust |
| IndexedDB | `web-sys` has the raw API; `rexie`/`idb` crates wrap it | Rust; the outbox's ordering and refusal rules port 1:1, with their tests |

So an all-Rust client ships ≈ 2–5 KB of hand JS (a loader + the SW) plus three vendor scripts. That is a true statement of the limit, not a blocker.

### 5.4 How the JS-based gates carry over

| Gate | Today | With a Rust client | Work |
|---|---|---|---|
| design gate (`scripts/design_gate.py`, reads HTML + linked CSS) | pages are HTML files | markup lives in `view!` macros; run the gate on **rendered** HTML: a `cargo run --bin render-all` (Leptos SSR of every route to a scratch dir) — C3 produces exactly that for the menu | M |
| `langs.mjs` | JS dictionaries (`uk:` keys) + Rust `"uk" =>` arms | dictionaries move to `crates/dowiz-hub/src/lang.rs`'s neighbours; rules `set`/`parity`/`files` already read Rust; rule `keys` becomes a Rust test | S |
| `icons.mjs` | scans `icon('x')` in JS | add `icon("x")`/`Icon::X` to `PATS`; or an `enum Icon` generated from `icons.css` so a missing icon is a compile error | S |
| `body-fields.mjs` | JS senders vs Rust `deny_unknown_fields` structs | **replaced by the type system** when the client uses the same structs; keep for any JS left | 0 |
| `ui-reach.sh` | route strings in public JS/HTML | route strings in the client crate (`format!("/api/orders/{id}")`); extend the scanner to `.rs` under the client crate, or generate a `routes.rs` enum both sides use and make the gate a Rust test | M |
| `sw-shell.sh` | module import closure vs the SW list | the shell is `index.html` + glue + `.wasm` + css: the list is the build output | S |
| `tap-size.sh` | CSS scan | unchanged | 0 |
| `strict-body`, `personal-data`, `one-image`, `dataflow`, `file-size` | Rust side | unchanged; `file-size`'s 300-line cap applies to the client crate (good: views stay small) | 0 |
| `browser-units` CI job (`node --test` over 77 `.mjs`) | node | `cargo test` for pure modules + `wasm-bindgen-test` in headless Chromium for DOM code (a browser on the runner: fine on GitHub; on the box headless Chromium exists but is UNVERIFIED with `wasm-bindgen-test`) | M |
| `flows.sh` (Playwright) | drives the DOM | unchanged | 0 |
| `vocab.sh` / `tools/gen-vocab` | generates `lib/vocab.js` from the kernel | obsolete for Rust surfaces (they import `dowiz_core` directly); kept while JS remains | 0 |

### 5.5 Shared types with the Rust backend — where the "clients 1" score really comes from

`crates/dowiz-core` is `#![no_std]`, zero deps; `crates/dowiz-hub` is bytes-in/bytes-out; `crates/bebop-wasm` already compiles both to wasm32 for the browser (CI job "bebop-wasm with the room deciders"). A Rust client gets the FSM, the money law, the lang set, the room deciders, the block codecs and the `deny_unknown_fields` request structs for free. **What it does not get:** the response shapes — the Worker builds answers with `serde_json::json!` at 2,750 sites (COSTS §2) rather than typed structs, so a typed client would need the server's responses typed first (the same refactor the binary-size report's row 5 asks for, for a different reason). That is 10–20 lane-days of server work before a Rust client can be "shared types" rather than "a Rust client parsing `Value`". P4 (gen-vocab) remains the cheap bridge until then.

### 5.6 Incremental path, surface by surface

1. **Room** (4 modules, 40 KB): the smallest, already shares the deciders; port to a Leptos CSR mount inside the existing page; proves the toolchain, the CI browser job and the gate changes. 2–3 lane-days.
2. **Courier** (14 modules, 169 KB): the outbox and the socket in Rust; the GPS/visibility rules have tests to port 1:1. 4–5 lane-days.
3. **Storefront as islands on C3**: the menu is prerendered HTML (no client code); the basket, checkout, tracking are islands; the map and the Sea stay JS. 6–8 lane-days, AFTER A1/C3.
4. **Console** (92 modules, 783 KB): last; 15–20 lane-days; the replica/IndexedDB layer from §3 is reused.
5. **Landing**: do not port; it is content (HTML + 38 KB of words).
Plus the server-side typing (5.5): 10–20 lane-days, and gates (5.4): ≈ 4 lane-days.

**Total ESTIMATE: ≈ 45–60 lane-days** (basis: 26k JS lines at 800–1,200 ported-and-gated lines per lane-day — the Worker's 63.8k lines landed at ≈ 1.8k per lane-day greenfield with tests; a port under pixel/flow gates is slower), i.e. **4–6 weeks of the whole 3-lane box doing nothing else**, and it moves **no Cloudflare number** (static assets are free either way; the Worker is already Rust).

### 5.7 Verdict

**Realistic: yes. Now: no — and not as a whole.** The measured and cited basis: (a) size — MEASURED on this toolchain: a Leptos floor of 66 KB raw / 28 KB gzip and a storefront-shaped page of 138 KB raw / 61 KB gzip (+ 5.7 KB gzip glue), i.e. **the same wire size as today's JS shell**, and smaller still as islands on prerendered HTML; (b) startup — MEASURED 0.9–1.6 ms compile+instantiate on an A78 (≈ 22 ms per MB, linear), single-digit ms on a low-end phone — not a factor; (c) the JS that remains is three vendor scripts plus ≈ 2–5 KB of loader/SW — "no hand-written JS" is reachable, "no JS" is not; (d) the gates carry with ≈ 4 lane-days of work and one gate (`body-fields`) is retired by the type system; (e) the shared-types benefit needs the SERVER's responses typed first (10–20 lane-days), which is the actual prerequisite and the actual prize ("clients 1 → 3"); (f) cost ≈ 45–60 lane-days for zero effect on the Free-plan caps.

**Recommendation:** after the §6 rows, do steps 1–2 (room, courier: ≈ 7 lane-days) as the proof, with the server typing (5.5) as its own lane in parallel; decide the storefront when C3 exists (islands make it cheap); the console only if steps 1–2 came in under their estimates. Measure, at each step, the three numbers this section names: wasm gzip bytes, Node compile+instantiate ms on the box (proxy), and Lighthouse TTI on one real low-end phone (U11).

## 6. The six approved items as rows

The six operator answers of 2026-10-01 (memory `dowiz-operator-decisions-2026-10-01`): (1) stay on Free, (2) delete `tools/native-spa-server`, (3) deploy + heavy gates from GitHub Actions, (4) production observability + Telegram alerts, (5) weekly backup-restore drill, (6) dinner-peak load test. Each row: owned files, deps, RED-first checks, acceptance numbers, risks, order. The GitHub numbers are CITED from docs.github.com, read 2026-10-01: GitHub Free = **2,000 Actions minutes/month for private repositories, 500 MB artifact storage; "the use of standard GitHub-hosted runners is free" in public repositories**; Linux 2-core at $0.006/min beyond that; a job may run **6 hours**; 20 concurrent jobs; cache 10 GB/repo; `schedule` no faster than **every 5 minutes**, "can be delayed during periods of high loads", and in a public repository "automatically disabled when no repository activity has occurred in 60 days"; environments on Free "only … for public repositories".

### R1 — Stay on Free: the design constraints that every other row inherits (—)

Not a lane; a rule set. (a) Every row budgets its own requests against the daily caps and refuses to run when `cf.do_requests_day` or `cf.worker_requests_day` is already above 40 % (the collector exists: `tools/evals/collect/cf.mjs`). (b) Nothing new runs in the Worker above ~3 ms CPU; heavy work goes to the object (OPEN 2 pending), to R2, or off-platform (GitHub Actions). (c) Reads that can be static or R2 are (§2 A1). (d) No new Cloudflare product that is Paid-only: Snippets, Tail Workers and Health Checks (Free: 0 checks) are all CITED Paid-only. **The one measured number that would force Paid**: `cf.worker_requests_day` or `cf.do_requests_day` crossing **80,000** on a day with paying venues, or a second venue onboarding in one day blocked by the 1,000 KV writes (until A1/FT3). Say that number, not "capacity", when it happens.

### R2 — Delete `tools/native-spa-server` — **DONE by main 2026-10-01** (plus the twin-only `crates/dowiz-hub/src/subs.rs`); the row is kept as the record of what the deletion had to touch (S, Haiku)

- **Owns:** `tools/native-spa-server/` (10,721 lines, MEASURED 09-26), `.github/workflows/ci.yml:46,62-63` (the rust-cache workspace entry and the test step), `CLAUDE.md` crate map line, `docs/design/BLUEPRINT-ARCHITECTURE-EVOLUTION-2026-09-22.md` P8 (append "DECIDED 2026-10-01: deleted"), `docs/design/BLUEPRINT-NATIVE-SPA-SERVER-LISTENER-HARDENING-2026-07-20.md` (header "superseded"), `scripts/verify-hub.sh` if it lists the crate, `tools/gates/paths.sh` baseline if a doc cites a deleted path.
- **Deps:** none. **RED first:** `tools/gates/paths.sh` must stay green after the deletion (every doc that cites `tools/native-spa-server/...` is edited in the same commit, or the gate names it); `grep -rn "native-spa-server" --include=*.yml --include=*.sh --include=*.toml` → 0 outside docs.
- **Acceptance:** CI `product` job green without the step; `git log --name-status` shows D for every file (memory `gitignore-ate-a-move`: check `--name-status`, not `--stat`); the 80 shared function names (memory `native-spa-server-is-a-second-worker`) now exist once.
- **Risk:** a doc or script that referenced it silently (the `paths.sh` gate catches docs; scripts are the grep). **Order:** first — it is one commit and removes a CI job that costs minutes every push.

### R3 — Deploy + heavy gates from GitHub Actions (M, Opus for the workflow, Haiku for the script moves)

**Design.** Two workflows beside `ci.yml`:

1. `deploy.yml` — `on: workflow_run: {workflows: [CI], types: [completed], branches: [main]}` plus `workflow_dispatch`; runs only when `conclusion == success`. One job, `environment: production`, steps: checkout the exact `head_sha`; `dtolnay/rust-toolchain@master` 1.96.1 + `wasm32-unknown-unknown`; `Swatinem/rust-cache` for `workers/api`; `npm ci` in `workers/` (wrangler is already a tracked lockfile there); **`tools/deploy/deploy.sh` with `DEPLOY_REPO=$GITHUB_WORKSPACE DEPLOY_WORK=$RUNNER_TEMP DEPLOY_SLOT="" DEPLOY_TOKEN_FILE=<written from secrets>`** — the script already takes every external as an override (its header lists them) and already does steps 1–6 (refuse, copy, gates, previous, upload, verify + `probe_crud` + advisory `flows`); the only change is that `slot.sh` becomes a no-op on a runner (`DEPLOY_SLOT=env` or a one-line `[ -n "$DEPLOY_SLOT" ] || SLOT=` guard). Secrets: `CLOUDFLARE_API_TOKEN` (the deploy token's value), `CLOUDFLARE_ACCOUNT_ID`, `DOWIZ_QA_OWNER_EMAIL`/`_PASSWORD` for `_probe_crud.mjs` (today from `/root/.dowiz_owner`), `ALERT_TG_TOKEN`/`ALERT_TG_CHAT` for the failure message. On FAIL the job prints the rollback command (deploy.sh already does) and posts it to Telegram; it never rolls back by itself (the script's rule).
2. `heavy.yml` — `on: schedule: '17 2 * * *'` + `workflow_dispatch`: the bebop battery (`bebop-lang/tools/battery.sh`, AArch64-only: needs an arm runner — `ubuntu-24.04-arm`, **free for public repositories; "these labels will not work in private repositories, and the workflow will fail if added"** (CITED github.blog changelog 2025-01-16, read 2026-10-01). If the repo is private, the row runs the Rust half only and SAYS the bebop half is unmeasured, exactly as `run-all.sh --cargo` does today), `cargo test` for every crate, the 360-program sweep, `tools/evals` nightly collectors with the analytics token. Artifacts: the gate table and the numbers, 7-day retention.

- **Owns:** `+.github/workflows/deploy.yml`, `+.github/workflows/heavy.yml`, `tools/deploy/deploy.sh` (slot guard, token from env instead of file when `DEPLOY_TOKEN_FILE` is unset), `tools/deploy/test.sh` (+2 cases: no slot, env token), `tools/deploy/verify.mjs` unchanged, `docs/testing.md` (job → proof table), `.github/workflows/ci.yml` (nothing; `product` loses R2's step).
- **Deps:** the operator adds the repository secrets (blocked on that today); R2 first (one job fewer).
- **RED first:** `tools/deploy/test.sh` gains the two cases and is RED before the script change; `deploy.yml` is first run with `workflow_dispatch` and `DEPLOY_MODE=--dry-run` against a branch (builds, gates, no upload) — the run must print "DRY-RUN OK <sha>"; then one real run with `--verify` only.
- **Acceptance:** `GET https://qa-durres.dowiz.org/api/version` reports the pushed commit within **30 min** of the CI green on main (ESTIMATE: a private-repo runner is 2 vCPU / 8 GB, a public-repo runner 4 vCPU / 16 GB — CITED; cargo test ≈ 10 min warm-cached, worker-build ≈ 6–8 min — the box's A78 took 5 m 26 s, MEASURED 09-27); the box runs **zero** deploys afterwards (`ls /root/.cache/dowiz-deploy/passed` stops growing); minutes per deploy ≤ **30**, so 30 deploys/month ≈ 900 of the 2,000 free minutes if the repo is private (or 0 if public — §8 has the command).
- **Risks:** (i) a public repository runs `pull_request` workflows from forks without secrets — fine, `deploy.yml` is `workflow_run` on main only; (ii) `wrangler deploy` reconciling `routes` is still impossible with this token (wrangler.toml header) — unchanged; (iii) a deploy from a runner changes nothing about the 429-on-root risk; (iv) the 60-day inactivity rule disables `heavy.yml`'s schedule in a public repo — a push resets it; (v) rust-cache misses cost +15 min; (vi) GitHub `schedule` delays — the nightly is not time-critical.
- **Order:** 2nd (after R2), the day the secret exists.

### R4 — Production observability + Telegram alerts, on Free (M, Opus for the object alarm, Haiku for the workflow)

What Free gives and does not: Workers Logs are dashboard-only, 3 days, 10 % sampled here; **no Workers notification type is listed for Free; Tail Workers are Paid-only**; the GraphQL analytics API works with the existing read-only token (`cf.mjs`). So alerts have three sources, none of them a Cloudflare alert:

1. **The product reports itself.** `loud!`/`worker_errors` (memory: never fired a row) become an **alert outbox on the `__platform` object**: a Worker error path appends `{class, venue, route, status, at}` (one object request, one row) and the object arms its alarm (the `hubdo/timer.rs` shape); the alarm drains to a platform-owned Telegram bot (`ALERT_TG_TOKEN`/`ALERT_TG_CHAT` as Worker secrets — a PLATFORM bot, distinct from the venue-owned notification bots) with a digest rule: one message per `(class, venue)` per 10 min, ≤ 20/min per group (Telegram's own limit, CITED core.telegram.org/bots/faq). Also from inside the Worker on the two outcomes it cannot see from a route: a `scheduled()` nightly that found a venue with no backup, and an object alarm that retried 6 times.
2. **An external watchdog**: `watch.yml`, `schedule: '*/15 * * * *'` (96 runs/day × ≈ 20 s ≈ 32 min/day ≈ 960 min/month — within 2,000 if private; every 5 min would be ≈ 2,900 and needs a public repo): `GET /api/version` and `GET /` on every live host (the registry list is read once from `dowiz.org/api/public/locations`, cached in the job), compares the live commit with `origin/main` HEAD (drift), posts to Telegram on any non-200, on drift, and once a day as a heartbeat ("watch alive"). Request cost: ≈ 3 hosts × 2 × 96 = **576 Worker requests/day** (0.6 % of the cap).
3. **The daily numbers**: `evals-nightly` already collects `cf.*`; add thresholds in `tools/evals/rules.mjs` (`cf.worker_requests_day > 60,000`, `cf.do_requests_day > 60,000`, `cf.cpu_kills_day > 0` (FT13's new collector), `cf.worker_errors_day > 0`, `cf.do_errors_day > 0`, `cf.do_rows_written_day > 60,000`, `cf.kv_writes_day > 600`) and a Telegram post of the table with a red mark per crossed line.

- **Owns:** `+workers/api/src/hubdo/alerts.rs` (outbox + drain, pure decision in `crates/dowiz-hub/src/alerts.rs` with tests: digest window, cap per minute), `workers/api/src/errlog.rs`/`loud!` call sites (append instead of log-only), `wrangler.toml` (no new binding; secrets by `wrangler secret put`), `+.github/workflows/watch.yml`, `tools/evals/collect/cf.mjs` (+ `cf.cpu_kills_day`, `cf.kv_writes_day`, `cf.do_rows_written_day`), `tools/evals/rules.mjs`, `+tools/evals/notify-telegram.mjs` (≤ 60 lines, tested with a fake fetch).
- **Deps:** R3 (the workflow runner and the secrets); FT13's collectors are inside this row.
- **RED first:** `alerts::tests::{digest_collapses_repeats, cap_20_per_minute, drain_marks_sent_once}`; `notify-telegram.test.mjs` (a 429 from Telegram is honoured with its `retry_after`); `watch.yml` first run against a deliberately wrong host must post "FAIL" — the alarm has to be heard once (`run-all.sh`'s rule).
- **Acceptance:** an owner-gated `GET /api/owner/probe-error` on qa-durres (raises one `loud!`) reaches Telegram within **2 min**; a stopped host alerts within **15 min**; the nightly table arrives every day; `worker_errors` stops being the instrument that never fired (memory `dowiz-fable-audit-2026-09-21`).
- **Risks:** Telegram as the only channel (add e-mail through the existing `send_email` binding to the verified address as the second channel — free, CITED); alert storms on a cap trip — the digest rule and a hard 50/day ceiling per class; the platform object becomes a hotter object (one request per error, bounded by the digest). **Order:** 3rd.

### R5 — Weekly backup-restore drill (M, Haiku; the restore half Opus)

The nightly already exports every venue with a bucket to R2 `dowiz-offsite` (7 objects, 17 MB on 09-25, MEASURED) — gzip, sealed where the seal key is set (operator unblocked the seal key 2026-09-26; whether qa-durres' export is sealed is UNVERIFIED — §8). Two arms:

- **Weekly, no production write** (`drill.yml`, `schedule: '0 4 * * 1'`): fetch yesterday's qa-durres export from R2 through the S3 API (secrets `R2_ACCESS_KEY_ID`/`R2_SECRET_ACCESS_KEY`, read-only key scoped to `dowiz-offsite`); gunzip; (unseal with `DRILL_SEAL_KEY` if sealed); open it with `crates/dowiz-hub` natively (`+crates/dowiz-hub/src/bin/drill.rs` or a `#[test]` with an env path): `Hub::chain_check()`, fold the log, count orders, read the catalogue's `K256`; compare with the venue's `/api/owner/health` counts captured at export time (the nightly writes a `<date>.health.json` beside the export — a 20-line addition to `cloud.rs`). Any mismatch posts to Telegram; a match posts the one-line "drill ok: N orders, chain ok, catalogue K256 unchanged".
- **Monthly, a real restore** (`workflow_dispatch` or `0 5 1 * *`): create a scratch hub `drill-<yyyymm>` through the two-step hub creation (memory `dowiz-platform-on-dowiz-org`), `POST /api/owner/restore` (the import route — exists? UNVERIFIED, §8; if not, R5 adds it, owner-gated, refusing a non-empty venue), then the same checks against the LIVE restored hub (`/api/owner/health`, `/api/public/locations/drill-…/menu` byte-compared to qa-durres' menu), then delete the hub. Request cost ≈ 50 Worker + 100 object + ≈ 1,000 rows written (an import day, like 09-18's 11,315 rows for photos — photos are NOT restored in the drill; they are content-addressed in KV/R2 and checked by key existence only).

- **Owns:** `+.github/workflows/drill.yml`, `workers/api/src/cloud.rs` (health sidecar), `+crates/dowiz-hub/src/bin/drill.rs` (+ tests on the committed golden images), `+tools/drill/restore.mjs` (the monthly arm), `docs/testing.md`.
- **RED first:** `drill.rs` on a golden image with one flipped byte must exit non-zero naming the chain break; the weekly workflow's first run against a deliberately truncated file must post FAIL.
- **Acceptance:** 4 green weekly runs in a row with the numbers printed; one monthly restore that serves qa-durres' menu byte-identically from the scratch host; time-to-restore recorded (**target < 20 min** end to end).
- **Risks:** the seal key in GitHub secrets (an encrypted secret on a public repo is still only as safe as the org's access list — restrict `drill.yml` to the `production` environment); the monthly arm writes to production (operator approved on 2026-10-01; keep it `workflow_dispatch` until two manual runs were clean). **Order:** 4th.

### R6 — Dinner-peak load test on qa-durres (M, Opus)

**Not from the box** (15 KB/s uplink, 32-process cap): from a GitHub Actions job (`load.yml`, `workflow_dispatch` only, `environment: production`), Node + `fetch` + one Playwright page per surface — no k6. Profile B compressed into **60 minutes at 17:30 UTC** on a weekday: 100 storefront visits (cold, with photos), 30 cash orders placed, the console socket open and advancing every order through PENDING→…→DELIVERED, 2 couriers on shift with a GPS fix every 10 s, a tracking page per order polling as the real one does.

- **Budget (ESTIMATE from the intervals):** 100 × 22 (before A1) + 30 × (1 placement + 6 actions + 27 tracking polls + 3 courier taps) + console/courier polls ≈ **3,600 Worker requests, ≈ 7,000 object requests, ≈ 1,000 rows written**: 3.6 % / 7 % / 1 % of the daily caps. **Guards:** the job reads `cf.worker_requests_day` and `cf.do_requests_day` first and refuses above 40,000; it counts its own requests and aborts at **10,000**; every request carries `X-Dowiz-Load: <run-id>`; it never runs while another lane or the nightly is active (17:30 UTC is 19:30 Durrës — the peak hour, and the nightly is at 03:17).
- **Measures:** client-side latency percentiles per route (placement, action, poll, menu), socket uptime per surface, order correctness (every order reaches DELIVERED; `/fold/rebuild` `stale = [] stranded = []`; `e2e/gates/conservation.mjs`), and from GraphQL for that hour: CPU p50/p99, `exceededResources` kills, object requests, **rows written per delivered order** (closes COSTS OPEN 4), bytes object→Worker. A second arm, ×3 for 15 min, is the stress arm — still under the 10,000 self-cap.
- **Acceptance:** 0 kills at the B rate; placement p99 < **1 s** wall from the runner; `cf.do_requests` for the hour within ±20 % of the budget (the model is right or the model is wrong, either is a result); rows per delivered order written into `docs/measurements/`.
- **Cleanup:** the load orders are real rows in qa-durres' log — placed with `channel: "load"` and a customer note `LOAD <run-id>` so reports exclude them; the nightly rotation archives them; the photos are not re-uploaded (the catalogue is not touched).
- **Owns:** `+.github/workflows/load.yml`, `+tools/load/run.mjs` (+ `.test.mjs`: the budget arithmetic and the abort against a counting stub, the `outbox.mjs` pattern), `tools/evals/collect/cf.mjs` (an hour-window query), `docs/measurements/LOAD-<date>.md`.
- **RED first:** `run.mjs --dry` against the stub proves the counts and the abort before any request reaches Cloudflare. **Risks:** the shared daily caps (the guards above are the whole design); a QA venue with real couriers signed in — run with the venue's load-only courier credentials (sushi-durres has no usable courier credential, memory `courier-login-venue-from-host`; qa-durres' are in `/root/.dowiz_owner`). **Order:** 5th, after R4 (so the run is observed) and after A2/A3 land (so the test measures the polling we will ship, not the one we are removing — or run it twice, before and after, which is the better experiment).

### Lane order and models (the answer the card asks for)

| # | Lane | Model | Days | Why here |
|---|---|---|---|---|
| 1 | ~~R2 delete native-spa-server~~ **DONE by main 2026-10-01** | — | — | the slot goes to lane 4 (A1+C3) a wave earlier |
| 2 | A2 + A3 + A4 (+A6): socket-first polling, auto-pong, registry cache | Haiku (A2/A3), Opus (A4) | 2 | the three best ratios, all small, pure JS + one `hubdo` call |
| 3 | R3 deploy from Actions | Opus | 2 | blocked on the operator's secret; start the day it exists |
| 4 | A1 + C3: storefront read path to R2 (`cdn.dowiz.org`), root without the Worker | Opus | 4 | the fail-closed defence; needs the R2 domain |
| 5 | R4 observability + Telegram | Opus (alarm) + Haiku (workflow) | 3 | before the load test |
| 6 | A5 dataflow 38 → 0 | Haiku ×2 (12 sites each) | 3 | ends the kills |
| 7 | B: IndexedDB stores | Haiku | 2 | after A1 names the blocks |
| 8 | R5 backup drill | Haiku (weekly), Opus (restore) | 2 | after R3/R4 |
| 9 | R6 load test | Opus | 2 | last; run before/after lane 2 if the dates allow |
| 10 | A8 (FT4/FT5 in the object) after FT13's object-CPU probe | Opus | 3 | gated on OPEN 2 |

Three lanes at once (box cap): {1,2,3} → {4,5,6} → {7,8,9} → {10}. ≈ 24 lane-days, ≈ 3 weeks of the box.

## 7. Conflicts with existing blueprints (numbered, both sides quoted)

| # | This report | The existing text | Resolution proposed |
|---|---|---|---|
| **K1** | §2 A1 / §4 C3: "the venue's menu, i18n, settings and photos are published by the hub object to R2 … read from there" — zero Worker requests for browsing | FREE-TIER FT11: "Requests are counted when the Worker is invoked; an `If-None-Match` that answers 304 is still a request … Worth doing for feel; it does not move the capacity table"; FT3 moved only PHOTOS to R2 | FT11 was right about the Worker path and did not consider R2 for the menu. A1 supersedes FT11's scope; FT3 becomes a part of A1 |
| **K2** | §2 A2–A4 ranked above A1 by ratio; §6 lane order puts A1 fourth | FREE-TIER §4 order: "FT13, FT2, FT1a, FT6, FT3, FT8, FT5, FT7, FT4, FT1b/c, FT9/FT11" — FT7 (socket-first) eighth | FT2 landed; with the cron gone, polling is the largest remaining line, so FT7 moves to the front. Both documents agree FT13 (instruments) precedes measurement-gated rows; this report folds FT13 into R4 |
| **K3** | §6 R3 keeps `tools/deploy/deploy.sh` as the deploy engine, run by GitHub Actions | `deploy.sh` header: "Every heavy step runs through bebop-lang/tools/slot.sh (one compute slot, waits under the process cap)"; quality-queue row 2 "Auto-deploy from main with a post-deploy version check (today the operator runs /root/dowiz-deploy.sh and it dies at the box's process cap)" — merged as d830cccb, on the box | The script's overrides (`DEPLOY_SLOT`, `DEPLOY_TOKEN_FILE`…) were written for tests; R3 makes them the runner's path. The box keeps `--verify` only |
| **K4** | §5: all-Rust web front-end — verdict below | ARCHITECTURE-EVOLUTION: "WASM monolith: server 4, clients 1"; P4 "Generate the client vocabulary from the kernel — the cheap half of direction 4"; `tools/gen-vocab` exists and `vocab.sh` gates it | P4 is the shared-types mechanism this report keeps; §5 says where a Rust client adds to it and where it does not |
| **K5** | §3: `replica.js` moves from `localStorage` to IndexedDB `orders`; blocks keyed by `k64` | BEBOP-DAG DW1: "`workers/api/public/lib/replica.js` … the browser recomputes the node and checks `K64`"; DW1 depends on DG2, DG7, DG9 (DG9 merged 9361706b with reds, then fixed in c4c9f1e9) | No conflict in substance; the B lane IS DW1's client half plus storage. DW1's "replica disagrees" line is B's RED test 2 |
| **K6** | §2 A7 / one-image 3 → 0 ranked LOW by ratio | ARCHITECTURE-EVOLUTION: "THE PLAN, in order: P1 move the order decision INTO the object … kills the hop, the whole-image promo write, and the two-image saga" — P1 FIRST | P1 is right for correctness (the saga whose compensation only logs) and wrong as a Free-cap lever (−2 object req/placement). Keep P1's priority on its own merits; do not sell it as a cost row |
| **K7** | §1.2: DO CPU "30 s default" vs FAQ "same per invocation CPU limits as any Workers" still unresolved | FREE-TIER OPEN 2 (same contradiction, 2026-09-26) and FT13's probe "`GET /fold/probe-cpu` … spins ≈ 25 ms once" | Nothing landed. R4 includes the probe; A8 waits on it |
| **K8** | §6 R6 load test from GitHub Actions | Operator decision 2026-10-01: "dinner-peak load test on qa-durres (careful: Free daily caps are shared by all venues)" — no venue named for the runner | The runner choice (not the box) is this report's; the guard numbers (40 % pre-check, 10,000 self-cap) are new |
| **K9** | §4 C4 "AGAINST for venue content" (static assets need a deploy per edit) | Card wording: "prerender at publish time into static assets (free, unlimited) or R2" | Static assets are per Worker version (CITED static assets Jul 3 2026: served per deploy; no out-of-band asset API found — §8); R2 is the publish-time target |
| **K10** | §2 A12 "Cache Rules for `/api` change nothing" | Memory `dowiz-inventory-recipes-2026-09-19`: "edge-cache `?fresh=1` trap" and `storefront.rs:619` menu edge cache 30 s — the Cache API path exists and saves CPU | Agreed: the Cache API saves CPU and object requests, never the Worker request. A12 refuses zone Cache RULES, not the Cache API |

## 8. What could NOT be verified, and the command that settles each

| # | Claim | Status | Command |
|---|---|---|---|
| U1 | Today's per-hub request profile | **MEASURED 2026-10-01 ~15:50 UTC, last 24 h, `cf.mjs measure()` over the analytics token:** Worker requests **1,250**, errors 0, subrequests 2,414, CPU p50 **6,757 µs**, p99 **84,024 µs**; DO requests **18,386**, DO errors **77**, alarms **1,506**, cron runs 1 (0 minute), DO→Worker bytes **250,066,690**. So **14.7 object requests per Worker request** (the 09-26 model assumed r ≈ 2) and 13.6 KB per object answer: the day's object traffic is NOT driven by browser requests — it is alarms (1,506), lane probes/flows against qa-durres, and something that answers 250 MB from the objects. The per-object split (`durableObjectsInvocationsAdaptiveGroups` by `objectId`,`type`) was attempted twice and both POSTs died on the box's connect timeout (`UND_ERR_CONNECT_TIMEOUT`, then an empty body) — memory `deploy-upload-fetch-failed`; the query is saved at the session scratchpad `q.json`, re-run it with `curl --connect-timeout 40` from a better link. Until then §1.3's profiles stand, with this caveat: **the first thing R4's nightly table must show is DO requests by object and type** |
| U2 | Whether cached reads on an R2 custom domain are billed as Class B | UNVERIFIED (the public-buckets page says the custom domain "allows you to use Cloudflare Cache", not how ops are counted) | after A1: `r2OperationsAdaptiveGroups` for one day vs the storefront's Playwright request count; the ratio is the answer |
| U3 | Object CPU limit on Free (30 s vs 10 ms) | OPEN since 2026-09-26 | FT13's owner-gated `GET /fold/probe-cpu` spinning ≈ 25 ms once, then `durableObjectsPeriodicGroups.exceededCpuErrors` |
| U4 | Repository visibility | **SETTLED, MEASURED 2026-10-01:** `gh` is not installed; `git remote -v` → `github.com:SyniakSviatoslav/dowiz`; unauthenticated `GET api.github.com/repos/SyniakSviatoslav/dowiz` → `private: false, visibility: public, default_branch: main`. So: **Actions minutes free, 4 vCPU / 16 GB runners, `ubuntu-24.04-arm` allowed** (the bebop battery can run in `heavy.yml`); environments available; and the 60-day inactivity rule applies to schedules | — |
| U5 | A restore route for R5's monthly arm | **SETTLED, MEASURED:** `lib.rs:481` `.post_async("/api/owner/restore", services::operations::restore)`; `cloud.rs:6` "`GET /api/owner/backup` downloads, so `POST /api/owner/restore` reads it" | — |
| U6 | Sealed exports | **HALF-SETTLED, MEASURED:** `cloud.rs:359-365`: the bundle key is `<prefix>/<venue>/<stamp>.json[.gz][.sealed]`; a `.sealed` one "opens only with `tools/seal-open` and the platform's secret key". Whether qa-durres' copies carry `.sealed` is a listing of `dowiz-offsite` (R2 API, read-only) — R5's weekly job lists the prefix first and branches on the suffix; the secret key then has to be a GitHub environment secret | `r2 object list` on `dowiz-offsite` with the deploy token |
| U7 | Static assets can be updated without a Worker version | **SETTLED, CITED** (direct-upload, Aug 10 2026): the assets-upload-session flow "does create a new Worker version" (completion token → script deployment); unchanged files skip re-upload. So C4 stays "a deploy per edit" | — |
| U8 | Cloudflare Health Checks on the Free plan | **SETTLED, CITED** (health-checks, Aug 14 2026): Free **0** checks; Pro 10. The external watchdog (R4 arm 2) is the only free probe | — |
| U9 | GitHub-hosted runner specs | **SETTLED, CITED** (runners reference, read 2026-10-01): public repositories **4 vCPU / 16 GB**, private **2 vCPU / 8 GB**, 14 GB SSD both; `ubuntu-24.04-arm` listed for both (but the changelog says the arm labels fail in private repos — U4 decides) | — |
| U10 | Leptos wasm size and startup on THIS product's toolchain | **MEASURED, §5.2** (Leptos only; Dioxus/Sycamore/Yew remain CITED secondary: 210 / — / 180 KB gzip for one dashboard) | the same crate with a second framework feature, through `slot.sh` |
| U11 | First paint of the prerendered menu on a low-end phone | cannot be measured on this box (WebGL/GPU dead; the box is an A78) | Lighthouse on a Moto G-class phone against `cdn.dowiz.org` after A1 |
| U12 | `inboundWebsocketMsgCount` reads 0 while pings are sent (FREE-TIER OPEN 6) | still open | after A3: `cf.do_hibernation_day` by `type` |
