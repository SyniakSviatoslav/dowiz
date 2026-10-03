# What one dowiz hub costs to run, recomputed (2026-10-03)

Lane W-COST, research only. Nothing was built, deployed or committed; no owner login was used. The only
network calls were read-only: Cloudflare GraphQL analytics (token `/root/.cf_analytics_token`, never
printed), `GET /api/version` on the three hosts, `GET cdn.dowiz.org/v/qa-durres/manifest.json`, the GitHub
repo metadata, and the Cloudflare docs pages below. The model is
`/tmp/claude-0/-root/b9eada30-066d-42d2-8eff-2b2b38ef0e02/scratchpad/cost/model.py` (+ `mixed.py`); the raw
GraphQL answers are `w.json`, `o.json`, `o7.json`, `dq.json`, `pq.json` in the same folder.

Labels: **M** = MEASURED today (command/dataset named), **C** = CITED (official page, read 2026-10-03, the
page's own "Last updated" given), **H** = HYPOTHESIS (an assumption, with its basis), **MEM** = carried from
a memory note and not re-verified.

---

## 0. Коротко (UA)

**Скільки коштує один хаб сьогодні.** На Free — **$0** за Cloudflare; єдиний реальний платіж — домен
dowiz.org ≈ **$0,93/міс** (MEM, $11,20/рік), спільний для всіх закладів. На Workers Paid — **$5,93/міс** за
перший хаб (база $5 + домен), і **$0,02–0,44/міс за кожен наступний** понад включені обсяги (тихий / типовий /
зайнятий заклад, сьогоднішній код). Після всіх запланованих оптимізацій і нових функцій — **$0,02–0,27/міс**
маржинально без платформного AI, і до **$0,47/міс** у зайнятого закладу, якщо AI (Workers AI) оплачує платформа.

**Скільки закладів витримає Free (рахується для всіх разом, обнуляється о 00:00 UTC).** Сьогодні першою
падає стеля **100 000 запитів до Durable Objects на добу**: ≈ **69 тихих** (10 замовлень/день) або
≈ **14 типових** (60/день) або ≈ **5 зайнятих** (200/день); у п'ятницю (×1,5) — 46 / 9 / 3. Після всього
запланованого — 106 / 22 / 9, а з двома новими виправленнями з §7 — **245 / 52 / 18**. Коли стеля падає, падає
**кожен** заклад (замовлення, консоль), а не лише той, що її з'їв.

**Головні знахідки вимірювання (7 днів, 26.09–02.10):**
1. **95 % усіх запитів до DO, 96 % записаних рядків і 99,7 % GB-s дає ОДИН об'єкт**, будильник якого спрацьовує
   **щохвилини 24 год** (1 449/добу) і щоразу кличе «runner», а той ≈ 10 разів читає заклад: ≈ 17 300 запитів
   DO і 6 500 рядків на добу — це 17 % добової стелі Free, і реальних замовлень за цим немає. Схоже на
   опитування каси eBills (`timer::ebills_next`: щохвилини, поки «відчинено», а без годин роботи — завжди),
   але назву закладу аналітика не дає — перевірити `/api/owner/health`.
2. **10 мс CPU зараз не вбиває**: 0 kills з 29.09 (до того 121–222/добу), хоч p99 CPU Worker-а 23–94 мс.
   Це «гнучкість» Free, яку Cloudflare не обіцяє. **CPU об'єкта на Free НЕ обмежений 10 мс**: p99,9 = 89 мс
   при 0 `exceededCpuErrors` — відкрите питання OPEN 2 закрито вимірюванням. Важку роботу (argon2, річна
   аналітика, шифрування push) треба тримати в об'єкті.
3. **Нові витрати, яких не було в старих звітах:** Web Push (W-PUSH) іде через ту саму чергу outbox → кожен
   запуск черги коштує ≈ 12 запитів DO (виміряно) — це найбільша нова лінія; dowiz-watch (ще НЕ задеплоєний)
   робить 4 запити на заклад кожні 5 хв → 1 152 запити Worker на заклад на добу, якщо внести туди всі заклади;
   нічний cron обходить усі заклади в ОДНОМУ виклику (ліміт Free 50 підзапитів / 10 мс CPU на виклик).

**Три найбільші статті витрат** (на Paid, 1000 зайнятих закладів, сьогодні): записані рядки DO ($113/міс),
запити DO ($78), R2 Class B ($67) / запити Worker ($51). Після оптимізацій — **Workers AI ($197)**, якщо платить
платформа; без нього — рядки, запити DO, R2.

**Три оптимізації з найбільшою економією:** (1) прибрати хвилинний цикл будильника / виконувати чергу в самому
об'єкті без «runner» (−10 запитів DO на кожен запуск; ×2,3 до місткості Free для зайнятих закладів);
(2) A4+BN4 (кеш реєстру + одна команда в об'єкті: r 2,0 → 1,1); (3) A1/C3+BN3 (вітрина з R2 і IndexedDB:
браузинг майже без Worker-а) — разом з A3 (auto-pong). AI має йти ключем власника (OpenRouter), бо 10 000
нейронів/добу Free — на весь акаунт.

**Платити $5 доведеться**, коли `cf.do_requests_day` стабільно > 80 000 — за цією моделлю це ≈ 55 тихих /
11 типових / 4 зайнятих закладів сьогодні. Старі числа виправлено в §8.

---

## 1. What was measured today

### 1.1 Account and deployment (M, 2026-10-03)

| Fact | Value | How |
|---|---|---|
| Deployed build | `fba9ee0f` built 2026-10-03T08:26Z on qa-durres, sushi-durres, dubin-sushi | `GET /api/version` |
| BN1A+BN1B (dataflow 38 → 0) and BN2 (menu published to R2) | deployed (they are ancestors of `fba9ee0f`); `cdn.dowiz.org/v/qa-durres/manifest.json` → 200 | git log + GET |
| Socket-first polling (A2/A6) | effectively landed: `lib/live.js` `due()` holds polls back until the socket is quiet > 90 s (`QUIET_MS`), used by `track.js:293`, `courier/app.js:1220` | code read |
| A3 auto-pong | NOT landed: 0 hits for `auto_response`; clients send an app-level `{"t":"ping"}` every 25 s (`live.js:42`) | grep |
| A4 registry cache, FT9 static root | NOT landed: `run_worker_first = ["/"]` (`wrangler.toml:97`) | grep |
| dowiz-watch | committed (`59cc27a1`), **NOT deployed**: 0 invocations of any `dowiz-watch` script through 2026-10-03 15:00Z; `dowiz-watch.…workers.dev/status` → 404 | GraphQL + GET |
| Repo | public (`private:false`), so GitHub Actions minutes are free | GitHub API |
| Plan | Workers Free (kills at exactly 10,000 µs on 09-26..09-28) | GraphQL |

### 1.2 Seven days, 2026-09-26 .. 2026-10-02 (M, `workersInvocationsAdaptive`, `durableObjectsInvocationsAdaptiveGroups`, `durableObjectsPeriodicGroups`)

| Day | Worker req | Worker CPU s | CPU p50/p90/p99 ms (success) | kills (10 ms) | DO http | DO alarm | DO rows written | DO GB-s |
|---|---:|---:|---|---:|---:|---:|---:|---:|
| 09-26 | 4,266 | 71.2 | 7.5 / 34.6 / 195 | 121 | 16,578 | 0 | 6,701 | 38 |
| 09-27 | 4,164 | 47.0 | 2.9 / 21.1 / 136 | 222 | 19,086 | 0 | 3,542 | 301 |
| 09-28 | 3,822 | 29.3 | 3.5 / 13.0 / 96 | 215 | 23,707 | 306 | 6,207 | 503 |
| 09-29 | 2,267 | 12.1 | 1.5 / 12.4 / 37 | **0** | 16,242 | 1,476 | 6,479 | 656 |
| 09-30 | 2,471 | 29.2 | 7.1 / 23.3 / 89 | 0 | 20,921 | 1,467 | 8,281 | 689 |
| 10-01 | 664 | 2.9 | 1.5 / 11.7 / 23 | 0 | 14,434 | 1,494 | 5,772 | 680 |
| 10-02 | 1,271 | 11.9 | 5.1 / 21.0 / 94 | 0 | 16,747 | 1,449 | 6,769 | 690 |
| **7 days** | **18,925** | **203.7** | mean **10.8 ms/req** | 558 | 127,715 | 6,192 | **43,751** | **3,557** |

Other lines, same window (M): Worker subrequests 43,172 (2.28 per request); DO rows read 99,372; WebSocket
messages out 1,276, in 0; DO `exceededCpuErrors` 0, `exceededMemoryErrors` 0; KV 475 reads / 32 writes,
storage 158 keys / 10.4 MB; R2 dowiz-cdn 152 PutObject + 153 GetObject (first day of BN2), dowiz-learn 232
PutObject, dowiz-offsite 14 PutObject. R2 stored (10-02): dowiz-learn 53.1 MB, dowiz-offsite 17.9 MB, dowiz-images
4.8 MB, dowiz-cdn 0.15 MB. `durableObjectsStorageGroups` is **empty** for this account (DO stored bytes not
measurable). `aiInferenceAdaptiveGroups`: empty (no Workers AI use). Other scripts: `academia-mesh` 4 req/day.

### 1.3 Who the traffic is (M, per object, `objectId` dimension, 09-29..10-02 daily means)

| Object (id prefix) | Role (inferred) | DO requests/day | rows written/day | GB-s/day |
|---|---|---:|---:|---:|
| `d144c156…` | a venue hub whose **alarm fires every minute, 24 h** | 14,374 http + 1,449 alarm | **6,565** | 359 |
| `0c2ee72a…` | its `cron~<venue>` runner (http = exactly the alarm count) | 1,449 | 0 | 319 |
| `dc1b6cc6…`, `a541cb2b…`, `d7765a58…`, `8b2ca736…` | the other hubs / registry | ≈ 920 together | ≈ 260 together | ≈ 1 |

On 10-02 the minute loop was **95 % of all DO requests, 96 % of rows written and 99.7 % of DO GB-s**. Per run
it costs (M): **≈ 11.9 DO requests** (1 alarm + 1 runner + ≈ 9.9 reads of the venue; the Worker's own
subrequests that day were only 2,366, so ≥ 12k of the venue's 14.4k http came from the runner), **4.5 rows
written, 7.1 rows read, 0.47 GB-s**, alarm wall p99 ≈ 12 s (the runner waits on an external fetch). Cause
(H): the eBills till link — `timer::ebills_next` re-arms every minute while the venue is "open", and a venue
with no opening hours is always open (`cron/timer.rs:65-90`, `MINUTE`); an outbox entry that keeps failing is
ruled out by its abandonment (`MAX_TRIES`). Which venue it is needs `/api/owner/health` (not used here).

**Venues and orders in the window.** Three venue hosts are live (sushi-durres, dubin-sushi, qa-durres) plus the
platform; 5 hub objects and 3 runner objects answered. The number of ORDERS cannot be read with the analytics
token, and this lane did not log in. Everything points to QA/test traffic dominating: qa-durres carried the
live-proof runs of 10-03 (rows 3–58 committed today), the flows gate and lane probes; the real venues' catalogue
and inventory were reset on 09-27 (MEM); no hour-of-day dinner peak exists in the data. **Treat the measured
day as ≈ 0 real customer orders and ≈ 1–4k test Worker requests plus one runaway alarm.** This is why §3
builds the per-hub profile bottom-up from code and measured UNIT costs instead of dividing the day by venues.

### 1.4 CPU (M)

- Worker: 0 `exceededResources` since 09-29 (121–222/day before), while success p99 is 23–94 ms and p90 12–23 ms.
  Free "built-in flexibility for infrequent overruns" (C, limits page) is currently absorbing it. It is not a
  contract; the 10 ms risk is dormant, not gone.
- **Durable Objects on Free are NOT capped at 10 ms**: DO http CPU p99 50–69 ms, p99.9 up to 89.6 ms, alarm p99.9
  15.7 ms, with 0 `exceededCpuErrors` over 7 days. This settles FREE-TIER OPEN 2 / cost-map U3 / K7: the
  object follows the 30 s default (C, DO limits Jun 1 2026). Consequence: argon2 (A8/FT4), the W-HIST 365-day
  analytics read (debug: 152 ms parse, 95 ms warm — it already runs in the object via `/fold/analytics`) and
  W-PUSH's RFC 8291 encryption (runs in the DO alarm) are safe in the object and unsafe in the Worker.

---

## 2. Prices and Free limits (C, read 2026-10-03)

Sources: developers.cloudflare.com `/workers/platform/pricing/` (Last updated Oct 2 2026),
`/workers/platform/limits/` (Sep 5 2026), `/durable-objects/platform/pricing/` (Sep 30 2026),
`/durable-objects/platform/limits/` (Jun 1 2026), `/r2/pricing/` (Oct 1 2026), `/workers-ai/platform/pricing/`
(Oct 1 2026), `/kv/platform/pricing/` (Apr 21 2026).

| Line | Free (daily caps are ACCOUNT-wide, reset 00:00 UTC, "further operations … fail") | Paid ($5/mo per account) |
|---|---|---|
| Worker requests | 100,000/day | 10 M/mo incl., +$0.30/M; static assets free and unlimited |
| Worker CPU | 10 ms per invocation (HTTP and Cron) | 30 M CPU-ms/mo incl., +$0.02/M; 30 s default |
| Subrequests | 50 per invocation; 1,000 to internal services | 10,000 |
| DO requests (HTTP, RPC, alarms, WS msgs at 20:1; outgoing WS and protocol pings free) | 100,000/day | 1 M/mo incl., +$0.15/M |
| DO duration | 13,000 GB-s/day | 400k GB-s/mo incl., +$12.50/M GB-s |
| DO rows read / written (`setAlarm()` = 1 row) | 5 M / **100,000 per day** | 25 B / 50 M incl.; +$0.001/M read, +$1.00/M written |
| DO SQL storage | 5 GB total | 5 GB-mo incl., +$0.20/GB-mo |
| DO billing unit | — | usage above the allowance is **rounded up to the next 1 M unit** |
| KV | 100k reads, **1,000 writes**/day, 1 GB | 10 M reads, 1 M writes incl.; +$0.50/M reads, +$5/M writes, +$0.50/GB |
| R2 (monthly, both plans) | 10 GB, 1 M Class A, 10 M Class B, egress free | $0.015/GB-mo, $4.50/M A, $0.36/M B (rounded up) |
| Workers AI | 10,000 neurons/day; beyond that needs Paid | 10,000/day free, then $0.011 per 1,000 neurons |
| Workers Logs | 200,000 events/day, 3-day retention | 20 M/mo incl., +$0.60/M (Observability pricing from Dec 1 2026) |
| Cron Triggers | 5 per account | 250 |

Workers AI neuron rates used (C): `m2m100-1.2b` 31,050/M tokens in and out; `qwen3-30b-a3b-fp8` 4,625 in /
30,475 out per M tokens; `whisper-large-v3-turbo` 46.63 per audio minute; `bge-m3` 1,075/M tokens.

---

## 3. The model

### 3.1 Venue sizes and fixed assumptions

| Input | Quiet | Typical | Busy | Label |
|---|---:|---:|---:|---|
| Orders/day N | 10 | 60 | 200 | operator brief |
| Storefront visits/day | 40 | 240 | 800 | H: 4 visits per order |
| Staff HTTP requests/day not tied to an order (boots, screens, reconnects) | 60 | 150 | 300 | H |
| Couriers on shift / kitchen board open | 1 / 0 | 2 / 1 | 4 / 1 | H |
| DO stored | 5 MB | 15 MB | 40 MB | H (storage dataset empty) |
| R2 stored per venue (photos + 7 nightly copies) | 20 MB | 20 MB | 20 MB | H (M: 17.9 MB offsite for ~3 venues) |
| Service window | 600 min | 600 min | 600 min | H |

Measured unit costs used everywhere (M, §1): 2.0 DO requests per Worker request that reaches a hub (2.28
subrequests/request measured, including external fetches); one outbox/alarm run = 11.9 DO requests + 4.5 rows;
an ordinary DO request ≈ 0.0008 GB-s; Worker CPU mean 10.8 ms/request. A notification drain is assumed to take
0.10 GB-s (H: push/Telegram answer in 100–300 ms; the measured 0.47 belongs to the slow till link).

### 3.2 Scenario A — TODAY (fba9ee0f) plus what merges today

Per order (H, from the route list in `public/store/*.js`, `admin`, `courier`): customer 12 Worker requests
(eta, reach, place, read-back, socket upgrade, ≈ 5 fallback polls, stamps/chat/feedback), console 5, courier 4 ×
0.7 delivery share, push subscribe 0.3; browse 2 per visit (`/` is still `run_worker_first`, plus one API call;
the menu, words and photos come from R2 since BN2). Rows per order 16 (H: 6 appends × 2 chunks + W-STOCK hold
+ idempotency/promo/table). Notification events per order 4.5 (H: 1 to staff on placement + W-PUSH ≈ 3.5:
status changes to a subscribed customer, courier assignment); events coalesce into ≤ 1 drain run per minute
(`RUN_GAP_MS`). App-level socket pings billed at 20:1 (A3 not landed). R2 Class B 8 per visit (H: 22 objects
on a cold uncached visit, edge cache + BN3A IndexedDB on repeats). Nightly 8 DO requests/venue, 60 rows/venue
(nightly + catalogue edits). W-HIST cube and 365-day read run in the object (CPU, not requests); W-INT2
WhatsApp status webhooks only when the venue enables WhatsApp (an optional line, not in the base).

### 3.3 Scenario B — after ALL planned optimisations, plus planned features that add load

Optimisations applied: A1/C3/BN2/BN3 (browse 0.1 Worker requests per visit; FT9 static root), A2/A6 fallback
polls 5 → 2, A3 auto-pong (pings free), A4 + BN4 (r 2.0 → 1.1; one command per placement), BN6 (rows per order
16 → 8, H), BN1/BN5/typed responses (Worker CPU 10.8 → 1.5 ms mean, H from the BN report's 0.3–1 ms floor), A8
(nightly in the objects: 3 DO/venue), AX/DG/DW rows (CPU inside the object; no request change, H).
Load added: dynamic menu Layer 0 (1 MET subrequest + 2 R2 Class A per venue-day; ranking on device = 0
requests), Workers AI (100 / 300 / 600 neurons per venue-day for quiet/typical/busy, H: translation drafts on
edits ≈ 11 neurons per dish edit, long-tail Q&A ≈ 18 neurons per question with qwen3-30b-a3b, sentiment per
review ≈ 3 neurons, optional whisper stock counts), +20 Worker requests/day for AI/translate calls,
SMS gateway status webhooks 0.5 per order, live-proof every 6 h (4 × 400 Worker / 800 DO / 200 rows, H),
dowiz-watch every 5 min watching platform + 3 named venues (13 probes/tick = 3,744 Worker req/day, M from
`probes.js`), GitHub heartbeat every 10 min (144/day).

**Scenario C = B + two fixes this report recommends (not on any roadmap yet):** the outbox drain runs inside the
venue object's own alarm turn instead of alarm → runner → ≈ 10 reads back (11.9 → 2 DO requests and 4.5 → 2 rows
per run), and the till-link poll stops polling 24/7 (§7).

### 3.4 Per venue-day results (model output)

| Scenario | Size | Worker req | DO req | rows written | GB-s | Worker CPU-ms | drain runs | R2 Class B | AI neurons |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| A today | quiet | 341 | 1,318 | 415 | 5 | 3,683 | 43 | 320 | 0 |
| A today | typical | 1,836 | 6,363 | 1,998 | 27 | 19,829 | 217 | 1,920 | 0 |
| A today | busy | 5,920 | 17,158 | 5,358 | 60 | 63,936 | 466 | 6,400 | 0 |
| B planned | quiet | 240 | 838 | 361 | 6 | 360 | 48 | 160 | 100 |
| B planned | typical | 1,130 | 4,053 | 1,607 | 27 | 1,695 | 236 | 960 | 300 |
| B planned | busy | 3,520 | 9,658 | 3,855 | 56 | 5,280 | 487 | 3,200 | 600 |
| C (+2 fixes) | quiet | 240 | 363 | 241 | 5 | 360 | 48 | 160 | 100 |
| C (+2 fixes) | typical | 1,130 | 1,716 | 1,017 | 25 | 1,695 | 236 | 960 | 300 |
| C (+2 fixes) | busy | 3,520 | 4,840 | 2,638 | 53 | 5,280 | 487 | 3,200 | 600 |

Platform load per day, independent of venue count (H except where marked): A ≈ 5,700 Worker / 7,900 DO /
600 rows (QA traffic ≈ 1,500 W measured order of magnitude + dowiz-watch once deployed); B/C ≈ 7,300 / 11,100 /
1,400 (adds live-proof). That fixed block is **8–11 % of the daily DO cap before the first customer**.

Optional per-venue line, measured: **a till link (eBills) polling every minute** = 17,240 DO requests, 6,520 rows,
681 GB-s per 24 h (half with 12 h opening hours). One such venue spends 17 % of the account's daily DO cap.

---

## 4. Cloudflare FREE

### 4.1 How many hubs fit before a daily cap fails ALL venues

Same-size fleets, average day and a 1.5× peak day (H: Friday/Saturday). The binding cap is in brackets; D = DO
requests/day. The Workers AI cap is listed separately because exceeding it fails AI calls only, not orders.

| Scenario | Quiet (avg / peak) | Typical | Busy | Mixed fleet 60/30/10 % | Workers AI 10k/day lasts for |
|---|---|---|---|---|---|
| A today | **69 / 46** (D) | **14 / 9** (D) | **5 / 3** (D) | 20 / 13 (D) | — (no AI) |
| B planned | 106 / 71 (D) | 22 / 15 (D) | 9 / 6 (D) | 33 / 22 (D) | 100 quiet / 33 typical / 16 busy |
| C (+2 fixes) | 245 / 163 (D) | 52 / 35 (D) | 18 / 12 (D) | 73 / 49 (D) | same |
| any + one till link polling 12 h | 9 | 6–8 | 3–6 | — | — |

The order the caps fall in is the same in every scenario: **DO requests → Worker requests ≈ DO rows written →
everything else**. Second-cap headroom for reference (A, typical): Worker 51 hubs, rows 50, GB-s 484, R2 Class B
(monthly) 171, DO storage 333. In B/C, Worker requests become the second wall (typical 82, busy 26).

What fails, and how (C): a Worker over 100k/day answers **error 1027** (and `run_worker_first` paths 429); a DO
over its cap makes "further operations of that type fail with an error" — placement, console, courier, tracking
for every venue at once, until 00:00 UTC. After A1/C3 the storefront still RENDERS from R2 when the caps are
spent (BN2's fallback is the opposite direction: R2 missing → Worker), but ordering does not.

### 4.2 Other walls on Free that do not scale with traffic

| Wall | Binds at | Label |
|---|---|---|
| **Nightly cron loops every venue in ONE invocation** (`cloud::nightly`, `lib.rs:584-603`): ≥ 4–6 DO/R2 subrequests per venue (settings, idempotency sweep, error prune, rotation, backup) | if R2/DO calls count against the 50-per-invocation limit: **≈ 8–12 venues**; if only against the 1,000 "internal services" limit: ≈ 160–250 venues. Plus 10 ms CPU per cron invocation (FT5 measured ≈ 105 ms/venue before BN1). Venue-owned S3 copies and the planned MET fetch are EXTERNAL subrequests (50 cap) | C (limits) + M (code); which counter applies is UNVERIFIED |
| dowiz-watch probes 4 URLs per watched venue per tick | 50 subrequests per tick → ≤ 12 watched venues; and 1,152 Worker + ≈ 1,440 DO requests per watched venue per day — watching 100 venues would spend 115k Worker requests/day, more than the cap | M (code) + C |
| KV writes 1,000/day | one photo import ≈ 822 writes → **1 onboarding/day** (A9 halves it; A1 photos-to-R2 removes it) | MEM (09-18) |
| Workers AI 10k neurons/day account-wide | one 165-dish menu translated into 3 languages ≈ 59k tokens ≈ **1,840 neurons = 18 % of the day**; AI features degrade for all venues after that | C + H |
| Cron Triggers: 5 per account | dowiz-api uses 1, dowiz-watch 1 | C |
| Workers Logs 200k events/day | sampled at 0.1 (`wrangler.toml:141-145`), not a failure, logs just stop | C |
| **10 ms Worker CPU** | dormant: 0 kills since 09-29 though p99 is 23–94 ms; at scale more invocations sit above 10 ms and Cloudflare's tolerance is undocumented. After BN1/BN5/A8 the mean is ≈ 1.5 ms (H) and the risk is the remaining argon2 login and any Worker-side fold | M + H |
| Nominatim (reverse geocoding, browser) | usage policy: absolute maximum 1 request/s per application (`docs/research/2026-10-03-live-proof-plan.md:421-423`); at ≈ 2 lookups per order that is ≈ 43k orders/day ≈ 200 busy hubs | C (via the plan) + H |

### 4.3 Monthly $ on Free

**$0 per hub.** The platform pays only the domain, ≈ $0.93/month (MEM: $11.20/yr), shared: $0.93 at 1 hub,
$0.09 at 10, $0.01 at 100. Email Routing and `send_email` are $0; GitHub Actions are $0 (public repo, M);
MET Norway, open.er-api, OpenFreeMap, Nominatim, FCM/Mozilla/Apple push are $0. Free is $0 until a cap breaks —
then it is not "more expensive", it is **down for every venue for the rest of the UTC day**.

---

## 5. Workers PAID ($5/month base + usage)

### 5.1 $/month per hub (total bill ÷ hubs), same-size fleets

| Scenario | Size | 1 hub | 10 hubs | 100 hubs | 1000 hubs | marginal per extra hub (overage rates) |
|---|---|---:|---:|---:|---:|---:|
| A today | quiet | 5.93 | 0.59 | 0.07 | 0.02 | 0.036 |
| A today | typical | 5.93 | 0.62 | 0.09 | 0.11 | 0.158 |
| A today | busy | 5.93 | 0.67 | 0.23 | 0.39 | 0.440 |
| B planned | quiet | 5.93 | 0.59 | 0.06 | 0.04 (0.01 w/o AI) | 0.061 (0.027 w/o AI) |
| B planned | typical | 5.93 | 0.61 | 0.14 (0.08 w/o AI) | 0.15 (0.06 w/o AI) | 0.208 (0.108) |
| B planned | busy | 5.93 | 0.64 | 0.27 (0.11 w/o AI) | 0.41 (0.22 w/o AI) | 0.467 (0.266) |
| C (+2 fixes) | busy | 5.93 | 0.61 | 0.25 | 0.35 (0.16 w/o AI) | 0.407 (0.206) |

Mixed fleet (60 % quiet, 30 % typical, 10 % busy), whole bill: A $5.93 / $6.08 / $8.21 / **$71.60** at 1 / 10 /
100 / 1000 hubs; B $5.93 / $6.08 / $10.81 / $110.20 ($43.32 without platform AI); C $5.93 / $5.93 / $10.21 /
$103.60 ($36.72 without platform AI). All include the domain.

### 5.2 What the 1000-hub bill is made of (busy fleet, $/month)

| Line | A today | B planned | C |
|---|---:|---:|---:|
| Base + domain | 5.93 | 5.93 | 5.93 |
| DO rows written | **113.00** | 68.00 | 31.00 |
| DO requests | 78.15 | 43.95 | 22.05 |
| R2 Class B | 66.60 | 31.68 | 31.68 |
| Worker requests | 51.04 | 29.17 | 29.17 |
| Worker CPU | 38.31 | 2.62 | 2.62 |
| DO duration (GB-s) | 25.00 | 25.00 | 25.00 |
| DO storage / KV / R2 storage | 9.16 | 9.16 | 9.16 |
| **Workers AI (if the platform pays)** | 0 | **197.30** | 197.30 |
| **Total** | **387** | **413** (216 w/o AI) | **354** (157 w/o AI) |

A till link polling 24 h adds $0.54/month per venue on Paid (524k DO requests, 198k rows, 20.7k GB-s).

**When Paid becomes necessary:** at the §4.1 capacities (e.g. ≈ 14 typical venues today). On the day it is
bought, the bill is $5.93 and stays there until roughly 100 mixed hubs.

---

## 6. Other costs, one line each

| Item | Cost | Label |
|---|---|---|
| Domain dowiz.org | $11.20/yr ≈ $0.93/mo; venue subdomains free; a venue's own domain is the venue's cost | MEM |
| Egress | $0 (Workers, R2) | C |
| Email Routing + `send_email` (waitlist, dowiz-watch alerts) | $0 | C + MEM (`dowiz-waitlist-mail-needs-email-routing`) |
| GitHub Actions | $0 — public repo (M today); if it ever goes private: 2,000 min/mo free, and a 5-minute watch alone would be ≈ 2,900 min | M + C (10-01 report) |
| Web Push (FCM, Mozilla autopush, Apple, WNS) | $0 per message; cost is the drain's DO requests (§3) | C (MEM) |
| SMS | platform $0 on the planned "venue's own Android phone as gateway" path; the venue pays its SIM plan. A commercial SMS API was not priced (not planned) | H |
| WhatsApp / Telegram / OpenRouter | venue-owned keys → $0 to the platform (Meta's per-conversation fees fall on the venue) | MEM |
| MET Norway, open.er-api, OpenFreeMap, Nominatim | $0; obligations are attribution, a User-Agent, caching, and Nominatim's 1 req/s | C (research reports 10-03) |
| Stripe (if enabled) | 1.5 % + €0.25 per EEA card order — ≈ 2,500× the infrastructure cost of that order | MEM (09-26) |
| Development (agent lanes) | the largest real spend; not on any Cloudflare bill | — |

---

## 7. Top 3 cost drivers and top 3 optimisations

**Cost drivers (Paid, 1000 busy hubs):**
1. **DO rows written** (A: $113) — 16 rows per order plus 4.5 per drain run; every chunk `put` and every `setAlarm` is a row.
2. **DO requests** (A: $78) — r = 2 per Worker request, 11.9 per drain run, socket pings at 20:1.
3. **R2 Class B + Worker requests** (A: $67 + $51) — storefront visits and staff/customer HTTP.
After the roadmap: **Workers AI** becomes the #1 line ($197) if the platform pays for it; it should not (owners' keys).

On FREE the driver is the same one everywhere: **DO requests/day**, and today 95 % of them are one minute loop.

**Optimisations ranked by saving:**

| # | Change | Saves | On a roadmap? |
|---|---|---|---|
| 1 | **Stop the 24/7 minute loop and run the outbox drain inside the venue object** (no alarm → runner → ≈ 10 reads back; poll the till link only in opening hours, and back off when nothing changed) | today: −17,000 DO req and −6,500 rows per day (−17 % of the Free DO cap) for the one venue; per drain run −9.9 DO req, −2.5 rows; Free capacity for busy venues 9 → 18 (B → C) | **NO — new.** BN4's "alarms consolidated" reduces the alarm COUNT, not the runner hop |
| 2 | **A4 + BN4** (registry/identity cached 60 s; one command per placement in one object turn) | r 2.0 → 1.1: ≈ −3,900 DO req per busy venue-day; rows per order 16 → 8 with BN6 | yes (A4 CRITICAL row; BN4) |
| 3 | **A1/C3 + BN3 + A3** (static root FT9, prerendered fragment on R2, IndexedDB replica, auto-pong) | browse 2 → 0.1 Worker req per visit (−1,500 W per busy day), R2 Class B per visit 8 → 4, socket pings 1,000+ billed DO req per busy day → 0 | yes (BN2 landed; FT9, BN3, A3 queued) |
| (4) | Keep platform AI off by default; owners bring OpenRouter keys; cap Workers AI per venue (≈ 300 neurons/day) | $0–197/month at 1000 hubs; protects the shared 10k/day | decided by operator (owner keys) |
| (5) | dowiz-watch: probe the platform + qa-durres + ONE sampled venue, not every venue | −1,152 Worker and ≈ −1,440 DO requests per extra watched venue per day | NO — new (its config lists 3 venues today) |
| (6) | Nightly fan-out to per-venue alarms (A8/FT5) before the venue count reaches ≈ 8 | removes the single-invocation wall (§4.2) | yes (A8), but its urgency is new |

---

## 8. Corrections to earlier numbers

| Earlier claim | Where | Correct (today) | Why it changed |
|---|---|---|---|
| "$5.93/mo per hub; Free's first limit at ≈ 4 venues" | memory `dowiz-hub-unit-costs` (09-20) | $5.93 is the PAID bill for the first hub; on Free the hub is $0 + the shared domain. Free holds ≈ 69 quiet / 14 typical / 5 busy today | the 09-20 note assumed Paid and whole-image polling (fixed by phases 1–7 and socket-first polling) |
| "$704/mo at 1000 busy hubs, DO rows written $294 the top line" | `COSTS-NOW-AND-BEBOP-2026-09-26.md` §1.6 | $387 at 1000 hubs of **200** orders/day (that report's busy = 30 orders); rows $113 | it scaled rows written ×7.25 with requests (its own OPEN 4); 15,000 Worker req per 30-order day assumed polling that is gone. Per order the old model was ≈ 15× too high on rows |
| "Free holds ≈ 3 busy hubs today, ≈ 50 after A1–A5" | `2026-10-01-cloudflare-free-cost-and-rust-web.md` §0/§2 | for the same 30-order venue: ≈ 25 today, 38 after the planned rows, 50 with the two new fixes | socket-first polling (A2/A6) has effectively landed in `live.js`; the "3" used the 15,000-request polling profile |
| "14.7 DO requests per Worker request — not browser-driven, unexplained" | same report U1 | explained: one object's minute alarm → runner → ≈ 10 reads, 95 % of DO requests | per-object query (`objectId`) ran today |
| "10 ms kills tripping now (622)" | 09-26 / 10-01 reports | 0 kills 09-29..10-03 although p99 is 23–94 ms | BN1 and the `/fold/*` projections took the catalogue off the hop; Free tolerance absorbs the rest |
| "DO CPU on Free: 30 s or 10 ms? (OPEN 2)" | FREE-TIER blueprint, 10-01 report K7/U3 | ≥ 89 ms per invocation observed with 0 `exceededCpuErrors` → not 10 ms | `durableObjectsInvocationsAdaptiveGroups` quantiles |
| "≈ 200–300 busy hubs per Free account after BN1–BN7" | `2026-10-01-fundamental-bottlenecks…` §8.2 | ≈ 50 (30-order venues) even after the two new fixes; ≈ 18 for 200-order venues | it moved staff actions onto the socket (I3, not planned as a row), had no Web Push drains, no platform probes, and assumed the drain cost 1 request |
| "$0.0002 per order" | 09-26 §1.5 | $0.00007 (A, busy, marginal $0.44 ÷ 6,080 orders/month) | fewer requests per order |

---

## 9. Not verified, and the command that settles each

| # | Open | Settle with |
|---|---|---|
| U1 | Which venue owns the minute loop, and whether it is the till link | `/api/owner/health` on each venue (owner credential) — or `GET /fold/…` timer state; then fix the cadence |
| U2 | Real orders and visits per venue (the model's N and 4 visits/order) | R4/BN8 nightly table by venue; a week of production after launch |
| U3 | Rows written per delivered order (16 → 8 assumed) | R6 load test or the BN4 collision probe with `cf.do_rows_written_day` |
| U4 | Whether nightly's R2/DO calls count against the 50-subrequest limit | a nightly with ≥ 12 venues on qa (or the same loop in a test Worker); read `exceededResources` for the cron |
| U5 | DO stored bytes (storage dataset empty) | `/api/owner/health` image gauges; or GraphQL `durableObjectsStorageGroups` once it populates |
| U6 | R2 Class B on cached custom-domain reads | after a week of BN2: `r2OperationsAdaptiveGroups` GetObject vs storefront visits |
| U7 | Workers AI neurons per venue-day | the `[ai]` binding's budget meter once W-AI lands; `aiInferenceAdaptiveGroups` |
| U8 | Domain renewal price | the registrar's renewal line |
| U9 | Whether app-level socket pings are billed (analytics show 0 inbound WS messages while clients ping every 25 s; `hibernation` invocations exist) | A3 makes it moot; until then `inboundWebsocketMsgCount` by day vs open sockets |

Reproduce: `node q.mjs <query>.gql '<vars>'` in the scratchpad folder above (reads the token file, never prints
it), then `python3 model.py` and `python3 mixed.py`.
