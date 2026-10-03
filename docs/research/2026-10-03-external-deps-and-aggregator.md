# External dependencies (rates, tiles, geocoder) and the aggregator: reliability, cost, and a real Wolt integration

Lane W-EXTRES (Opus, research only). 2026-10-03. Tree: main `34aa2bc0`.
Tags: **MEASURED** = a command run today with its output, mostly a read-only GET against live production or the provider. **CITED** = `file:line` in this tree. **SOURCE** = an external page with its URL, read on 2026-10-03 unless another date is given. **GUESS** = reasoned but not measured. Treat every GUESS as a hypothesis.
Nothing in this lane wrote to production. The only requests sent to production were GETs.

---

## 0. Відповідь оператору (uk, одна сторінка)

**Курси валют.** Працюють, MEASURED: `qa-durres/api/public/rates` → `EUR 10861 ppm`, `stale:false`, `asOf 2026-10-03 00:02 UTC`. Знайдено чотири дефекти:
1. Невдала відповідь (`stale:true`, тільки лек) кешується на годину, так само як вдала. Одна хвилинна збійка постачальника означає годину без EUR у цьому дата-центрі (`rates.rs:112-115`).
2. Немає тайм-ауту ні у Worker, ні в браузері, а перший рендер вітрини **чекає** на курс, якщо відвідувач раніше обрав EUR (`app.js:127`).
3. Умови open.er-api вимагають підпис «Rates By Exchange Rate API». Його в коді немає (0 збігів).
4. Резервного джерела немає.

**Рекомендація.** Курс зберігати як щоденний знімок у R2: його пише вже наявний нічний cron, з двома джерелами і з «останнім добрим» значенням та його віком. Власник може задати свій курс вручну. Вартість ≈ 0: 2 підзапити на добу і 1 запис у R2.

**Плитки мапи (OpenFreeMap).** Працюють, MEASURED: 4 стилі відповідають 200 за 0.11–0.65 с, плитка Дурреса — 114 КБ. SLA немає. У серпні 2025 сервіс уже частково ліг під навалою трафіку Wplace. Коли плитки недоступні:
- у кур'єра мапа зникає, але адреса і дзвінок лишаються;
- у клієнта на листі «обрати на мапі» сірий прямокутник без пояснення (`address.js` не обробляє `error`).

**Рекомендація.** OpenFreeMap лишається основним джерелом. Резервне джерело — власний PMTiles-витяг Дурреса й Тирани (≈10–20 МБ, GUESS) на `cdn.dowiz.org`. MEASURED: CDN уже віддає `206 Range` з CORS. Мапа перемикається на резерв автоматично. Кур'єрський service worker кешує плитки. Вартість 0 $ (R2 free 10 ГБ).

**Геокодер (Nominatim).** Працює, MEASURED: reverse для Дурреса → «Rruga Sulejman Kadiu». Але є важливий факт: **у коді немає пошуку адреси взагалі**, лише зворотне визначення вулиці за піном. «Клієнт не знайде адресу на мапі» — це відсутність функції, а не збій. Номери будинків в OSM для Дурреса майже відсутні: MEASURED, 614 об'єктів з `addr:housenumber` на весь муніципалітет. Пін — це правда, вулиця — підказка.

**Рекомендація** (найнадійніше перше):
1. Локальний список вулиць закладу: MEASURED, 743 вулиці Дурреса, 10 КБ gzip. Пошук іде в браузері і працює, навіть коли все зовнішнє лежить.
2. Photon (komoot) як пошук за кнопкою з `bbox` Албанії. MEASURED: `lang=sq` → 400, тож слати треба `default`.
3. Тайм-аут 4 с і Photon як резерв для reverse.
4. Пін без геокодування, як і зараз.

Платні сервіси не потрібні.

**Агрегатор — головне.** Поточний стан чесно:
- **моку Wolt немає**, і адаптера Wolt теж немає;
- є **ручне введення** замовлення з планшета платформи (кнопка «Замовлення з платформи» на екрані «Замовлення», `orders.js:161`);
- рядок F4-mock, дозволений 09-26, так і не був побудований;
- на екрані «Інтеграції» агрегатора немає зовсім.

Факти про ринок станом на 2026-10-03:
- **Wolt** — єдиний міжнародний агрегатор в Албанії: 7 міст, серед них Тирана і Дуррес (MEASURED).
- **Glovo** у квітні 2024 відмовився від запуску в Албанії. Це виправляє попередній блюпринт, де Glovo «у Тирані».
- **Bolt Food** в Албанії немає.
- **Baboon** (місцевий) API не має.

Wolt підключає лише **POS- і middleware-провайдерів**: dowiz подається як POS. Підключення йде через OAuth 2.0: заклад сам натискає «Підключити Wolt», логіниться у Wolt, і dowiz отримує токени. Вебхуки підписані HMAC-SHA256 (`WOLT-SIGNATURE`). Є dev-середовище, тобто пісочниця.

**Що має зробити оператор:**
1. Подати заявку на інтеграцію як POS-провайдер на developer.wolt.com: Getting started → integration request form.
2. Прийняти T&C і мінімальні вимоги Wolt.
3. Отримати `client_id` і `client_secret` (OAuth) для dev та prod.
4. Зареєструвати redirect URL `https://dowiz.org/api/wolt/callback` і webhook URL `https://dowiz.org/api/hooks/wolt` з власним секретом підпису (≥128 біт).
5. Отримати тестовий заклад у dev-середовищі Wolt.
6. Попросити в акаунт-менеджера Wolt Albania пілот на одному закладі (sushi-durres або dubin-sushi).

**Що будуємо зараз, без ключів:** картку «Wolt» на екрані «Інтеграції» чотирма мовами з такими елементами:
- статус-лампа;
- «останнє замовлення … хв тому»;
- кнопка «Ввести вручну»;
- кнопка «Підключити», яка чесно пише «очікує схвалення Wolt»;
- вимикач «пауза».

Далі — вхідний вебхук із перевіркою підпису і з durable-вхідною чергою в DO, бо Wolt повторює вебхук лише тричі по 5 с і потім губить. Повний план рядків — у §6.

---

## 1. Rates: `services/ordering/rates.rs` → open.er-api.com

### 1.1 Current state

| aspect | fact | tag |
|---|---|---|
| Route | `GET /api/public/rates?base=XXX` (`lib.rs:340`) → `rates()` (`rates.rs:45`) | CITED |
| Upstream | `https://open.er-api.com/v6/latest/{base}` (`rates.rs:75`), read through `crate::edge::fetch` (`edge.rs:230`). That is a bare `worker::Fetch`, with **no timeout or AbortSignal and no retry** | CITED |
| Workers wall time | For an HTTP request the duration is "no limit … as long as the client remains connected" ([Workers limits](https://developers.cloudflare.com/workers/platform/limits/)). A hanging upstream therefore hangs the request, though it uses no CPU | SOURCE |
| Edge cache | `caches.default` keyed by `https://rates.dowiz/{base}` (`rates.rs:58-61`), `cache-control: public, max-age=3600` (`:113`). **Every outcome is cached for an hour, the stale identity answer included** (`:82`, `:112-115`). The Cache API is per data centre, so each colo fetches once an hour | CITED |
| What is shipped | Only `ALL`, `EUR`, `USD` in integer ppm (`:88-95`). `asOf` comes from the upstream's `time_last_update_utc` | CITED |
| Live answer | First GET: 200 in 0.48 s, `ppm {ALL:1000000, EUR:10861, USD:12295}`, `asOf "Sat, 03 Oct 2026 00:02:32 +0000"`, `stale:false`. Second GET: `cf-cache-status: HIT` in 0.20 s, but **`cache-control: public, max-age=14400`**. The browser TTL is rewritten from 3600 to 14400 on a HIT. GUESS on the cause: the zone's Browser Cache TTL setting | MEASURED |
| Upstream live | `open.er-api.com/v6/latest/ALL` → 200 in 0.14 s, 166 currencies, `time_next_update_utc "Sun, 04 Oct 2026 00:14:22"`. EUR 0.010861 → **92.07 ALL/EUR** | MEASURED |
| Cross-check | Bank of Albania official 02.10.2026: **EUR/ALL 91.98, USD/ALL 81.80** ([BoA](https://www.bankofalbania.org/Markets/Official_exchange_rate/)). er-api differs by +0.1 % on EUR and −0.6 % on USD (81.33). Fine for a display-only reading | SOURCE + MEASURED |
| Browser client | `lib/money.js:91-101` `loadRates`: plain `fetch`, no timeout. On failure it falls back to `{ppm:{[base]:1e6}, stale:true}`. `formatter` shows the **venue's real currency** when the ppm is missing (`money.js:73-84`), so a failure never invents a number. `exactCharge` appends ` ⚠` when stale (`store/state.js:150`) | CITED |
| First paint | `public/app.js:127` `await resolveCurrency()` runs **before the loader dissolves**. `resolveCurrency` (`state.js:141-144`) awaits `loadRates` only when the visitor's remembered currency is not the base. **A returning EUR visitor waits on Worker → er-api with no timeout.** Owner console (`admin/core.js:115`) and courier (`courier/app.js:41`) do the same at their own boot | CITED |
| Service worker | `public/sw.js:156-170`: `/api/*` is never cached (W-SYSCHECK row 48), so offline there is no rate and lek is shown | CITED |
| CSP | Not needed: the browser calls same-origin `/api`, and only the Worker talks to er-api | CITED |
| Coverage | Base is the venue's `currencyCode`. All three live venues are `ALL` (MEASURED: menus on qa-durres, sushi-durres, dubin-sushi). The display set is `ALL/EUR/USD` (`lib/vocab.js:93-95`). The UI languages sq/en/uk(/ru) are not involved | MEASURED |
| Attribution | The provider's terms require `<a href="https://www.exchangerate-api.com">Rates By Exchange Rate API</a>`. `grep -rni "exchangerate-api\|Rates By" public src` → **0** | SOURCE + MEASURED |

### 1.2 Failure modes

| mode | likelihood | evidence | what the user sees today |
|---|---|---|---|
| er-api returns 429 for a while | LOW | "After 20 minutes the rate limit will finish". It is triggered by heavy polling, and one request per colo per hour is far below that ([docs/free](https://www.exchangerate-api.com/docs/free)) | stale identity, **pinned for 1 h** in that colo (and up to 4 h in the browser, per the measured `max-age=14400`). The price shows in lek with ⚠ |
| er-api down or 5xx | LOW-MED. No SLA, no status page found; free endpoint. GUESS | — | same as above |
| er-api hangs (TCP accepted, slow body) | LOW, high impact | there is no timeout anywhere in the path (§1.1) | **the storefront loader never dissolves** for a visitor with a non-base preference, until the browser gives up |
| Access withdrawn for missing attribution | LOW | terms require attribution, and we show none | permanent stale |
| Data stale by up to 24 h | CERTAIN, by design | "Updates Once Per Day" | the honest `asOf` is shipped but **not shown** anywhere |
| ALL absent from a source | — | ECB's reference set has no ALL: MEASURED `eurofxref-daily.xml` 2026-10-02 has USD 1.1225 and no `ALL`. Frankfurter-style ECB mirrors are therefore useless as a second source for lek | — |

### 1.3 Options, ranked by reliability then cost (Cloudflare Free)

| # | option | reliability | cost on Free | latency |
|---|---|---|---|---|
| **R-A** | **Daily snapshot in R2, written by the existing cron** (`crons = ["17 3 * * *"]`, `wrangler.toml:177`, after er-api's ~00:02 UTC update). The cron reads er-api first and `@fawazahmed0/currency-api` second. **MEASURED** `latest.currency-api.pages.dev/v1/currencies/all.min.json` → `date 2026-10-03`, eur 0.010878562, usd 0.012242002, which is within 0.2 % / 0.4 % of er-api. The fallback host is `cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/…`. The cron writes `rates/ALL.json` to the `CDN` bucket **only when a source parsed and passed sanity** (ppm in a ±20 % band of the last good value). Otherwise last-known-good stays, and its `asOf` ages honestly | survives any single provider outage for days, with age visible | 2-4 subrequests/day, 1 R2 Class A/day (free 1 M/month), 0 Worker requests per visit if the storefront reads `cdn.dowiz.org/rates/ALL.json` directly (CSP `connect-src` already has `https://cdn.dowiz.org`, `_headers:66`) | one CDN GET; MEASURED cdn.dowiz.org ≈ same-origin |
| R-B | Owner-set manual rate per venue (`rates.manual.EUR` ppm in settings), which wins over R-A and is labelled "rate set by the venue" | as reliable as the owner | 0 | 0 |
| R-C | Keep the per-request Worker fetch, but add a 3 s timeout, cache stale for 60 s instead of 3600, render first and repaint when rates arrive, and show `asOf` plus attribution | removes the hang and the 1-h pin | unchanged (≤ 1 subrequest per colo per hour) | first paint no longer waits |
| R-D | Bank of Albania as a source | authoritative for ALL, but **no machine feed** ("No specific XML, JSON, or CSV feed URL", [BoA page](https://www.bankofalbania.org/Markets/Official_exchange_rate/)). Scraping HTML is fragile, so use it only as the human reference shown in the owner's manual-rate sheet | — | — |
| R-E | Paid FX API | not needed; display-only | $ | — |

**Recommendation:** ship R-C now (S, removes the two user-visible defects), then R-A + R-B (M). Numbers:
- one cron run/day with ≤ 4 subrequests (≤ 50 allowed) and < 1 ms CPU (GUESS);
- a 6 KB JSON parsed;
- R2 ≈ 1 KB;
- per visit: 0 Worker requests when read from the CDN;
- the visitor sees "€ ≈ … · rate of 03.10, 00:02 UTC · Rates By Exchange Rate API" or "rate set by the venue".

### 1.4 Contract (to pin in tools/live-proof, operator rule 2026-10-03)

```
LINK rates.upstream.primary
  producer : open.er-api.com   version: path /v6/ (free "open access" endpoint)
  request  : GET https://open.er-api.com/v6/latest/ALL   (no key)
  response : { result:"success", base_code:"ALL", time_last_update_unix:int, time_last_update_utc:str,
               time_next_update_unix:int, rates:{ <ISO4217>: number>0, ... } }   must contain EUR, USD
  terms    : once-a-day data; 429 for 20 min on abuse; attribution required; no redistribution
LINK rates.upstream.fallback
  producer : @fawazahmed0/currency-api  version: v1 (CDN path)
  request  : GET https://latest.currency-api.pages.dev/v1/currencies/all.min.json
             (fallback https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@latest/v1/currencies/all.min.json)
  response : { date:"YYYY-MM-DD", all:{ eur:number>0, usd:number>0, ... } }
LINK rates.snapshot (ours)
  producer : cron 03:17 UTC → R2 dowiz-cdn key rates/ALL.json
  consumer : storefront/console/courier lib/money.js; GET /api/public/rates (reads the same object)
  schema v2: { v:2, base:"ALL", ppm:{ALL:1000000, EUR:int>0, USD:int>0}, decimals:{ALL:0,EUR:2,USD:2},
               asOf:"RFC3339", source:"er-api"|"currency-api"|"manual", fetchedAt:"RFC3339", stale:bool }
  invariant: stale == (now - asOf > 48 h); ppm within ±20 % of previous snapshot or the write is refused (loud)
```

---

## 2. Map tiles: OpenFreeMap

### 2.1 Current state

| aspect | fact | tag |
|---|---|---|
| Library | MapLibre GL **v5.9.0**, self-hosted `/lib/map/maplibre-gl.js` (UMD), loaded on demand (`store/ui.js:151-161`, `courier/app.js:134-153`, `kit/screens/map.js:45-58`) | MEASURED/CITED |
| Styles | storefront address picker and venue map: `styles/liberty` (`store/address.js:31`, `store/venue.js:25`). Tracking map: `styles/positron` (`store/track-map.js:16`). Courier and kit: `styles/bright` / `styles/dark` (`courier/app.js:111`, `kit/screens/map.js:15-16`) | CITED |
| Live | `liberty` 200 in 0.65 s (43 KB), `positron` 0.14 s, `bright` 0.13 s, `dark` 0.11 s, all `cache-control: public, max-age=86400`, `access-control-allow-origin: *`, `server: cloudflare`. TileJSON `planet` → `tilejson 3.0.0`, OpenMapTiles schema `version 3.16.0`, build `20260927_080001_pt`, maxzoom 14. The Durrës z14 tile `14/9076/6123.pbf` → 200 in 0.22 s, **114 081 B**, `max-age=315360000`. Glyphs and sprites also come from `tiles.openfreemap.org` | MEASURED |
| Caching | browser HTTP cache only. `courier/sw.js:100-102` and `public/sw.js:159-162` ignore cross-origin requests except `cdn.dowiz.org`, so **no service-worker tile cache** exists | CITED |
| Timeouts / retries | MapLibre's own (it retries nothing on a style failure) | — |
| When down: courier | `map.on('error')` before load → `map.remove()` (`courier/app.js:171`). The sheet keeps the address and the call button. After load, missing tiles leave a grey or partial map | CITED |
| When down: tracking map | slot hidden on an early error (`track-map.js:96`) | CITED |
| When down: address picker | **no `error` handler** (`address.js:175-181`). The sheet shows an empty box with a pin that cannot be seen. "Use my location" still sets coordinates, and reverse-geocoding still runs. The typed street is enough to order (`checkout.js:362`) | CITED |
| When down: venue map | grey box, no message unless the library itself fails (`venue.js:229-239`) | CITED |
| CSP | `img-src … https://tiles.openfreemap.org`; `connect-src … https://tiles.openfreemap.org`; `worker-src 'self' blob:` (`_headers:66`). Live CSP on qa-durres is identical (MEASURED) | CITED/MEASURED |
| Coverage | planet-wide. Albania detail = OSM. Venue coordinates: sushi-durres and dubin-sushi `41.315347, 19.4449964`; **qa-durres `lat: null`**, so its maps open on the code's Durrës fallback (`address.js:33`) | MEASURED |

### 2.2 Failure modes

| mode | likelihood | evidence |
|---|---|---|
| OpenFreeMap degraded | MED over a year | Aug 2025: Wplace sent ~100 000 req/s and 3 billion requests in 24 h; "some tiles were not loading", 96 % served ([Gigazine 2025-08-12](https://gigazine.net/news/20250812-openfreemap-survived-100000-requests-wplace-live/)) |
| No SLA | CERTAIN | "I don't offer SLA guarantees or personalized support" ([openfreemap.org](https://openfreemap.org/)) |
| Usage limits / keys | none | "no limits on the number of map views or requests", "no registration … no API keys", commercial use "Yes" ([openfreemap.org](https://openfreemap.org/)) |
| CORS | OK | `access-control-allow-origin: *` (MEASURED) |
| Courier on a weak 3G edge | HIGH in practice, GUESS | a z14 city tile is ~114 KB and there is no SW cache, so every shift re-downloads |
| Weekly planet rebuild changes the tile URL | certain, harmless | the tilejson points at a dated path; the style resolves it |

### 2.3 Options

| # | option | size / cost | reliability |
|---|---|---|---|
| **T-A** | **Keep OpenFreeMap primary; add a PMTiles fallback on `cdn.dowiz.org`** built from a **Protomaps** extract, so the fallback comes from a different producer. `pmtiles extract https://build.protomaps.com/<date>.pmtiles durres-tirana.pmtiles --bbox=19.38,41.24,19.95,41.42 --maxzoom=14` runs in a weekly GitHub Action; no Java and no Planetiler. Self-host glyphs and sprite (`protomaps-themes-base`). Load `pmtiles.js` (GUESS ~30 KB). Switch the style on ≥ 3 tile or style errors within 5 s, or on a failed style fetch | Size, GUESS: the bbox ≈ 25 × 10 z14 tiles; at ~20-30 KB mean that is **≈ 10-20 MB** with lower zooms. Whole Albania bbox (19.26-21.06 E, 39.64-42.66 N): z14 = 15 189 tiles (MEASURED count); a 16-tile random sample gave median 3.5 KB, mean 15 KB (one urban 166 KB tile), so z0-14 ≈ **70-300 MB** (GUESS; Protomaps says each zoom roughly doubles the size, Berlin z0-15 = 84 MB, [docs](https://docs.protomaps.com/guide/getting-started)). R2 free: 10 GB storage, 10 M Class B/month, free egress ([R2 pricing](https://developers.cloudflare.com/r2/pricing/)). MEASURED: `cdn.dowiz.org/v/qa-durres/manifest.json` with `Range: bytes=0-99` → **206**, `accept-ranges: bytes`, per-origin ACAO, `cf-cache-status: DYNAMIC`. Range reads already work; each is a Class B op (≈ 50-150 per map session, GUESS, i.e. ≥ 66 000 sessions/month free) | two independent producers; ours has no third party in the path |
| T-B | Self-hosted primary, OpenFreeMap as fallback | same storage; 100 % of map traffic becomes Class B | best control, but we own the pipeline's freshness |
| T-C | **Courier SW tile cache**: `courier/sw.js` cache-first for `tiles.openfreemap.org/planet/**.pbf`, `fonts/**`, `sprites/**`, LRU-capped at ~1500 tiles (~30 MB, GUESS) | 0 $ | a courier who rode Durrës yesterday has a map today even with OFM down. Note: a PMTiles 206 response cannot be put in the Cache API; cache only the OFM z/x/y tiles |
| T-D | Error UX: every map surface gets `on('error')` → "map unavailable — type the street, the courier gets your words" (4 languages), never an empty box | 0 | removes the silent grey box |
| T-E | MapTiler / Stadia / Mapbox | key in the browser; free tiers capped (≈100k loads/month class, GUESS) | adds an account and a quota; rejected |

**Recommendation:** T-D (S) → T-C (S) → T-A (M). Expected result: with OFM down, the courier and the customer still see Durrës and Tirana streets. Cost $0 and 0 extra Worker requests: tiles never pass through the Worker, which matters on the Free 100 000 req/day cap.

### 2.4 Contract

```
LINK map.style (OpenFreeMap)   version: OpenMapTiles schema 3.16.0, TileJSON 3.0.0, MapLibre style spec v8
  GET https://tiles.openfreemap.org/styles/{liberty|positron|bright|dark} → 200 application/json
      { version:8, sources:{ openmaptiles:{type:"vector", url:"https://tiles.openfreemap.org/planet"} , ...},
        glyphs:"https://tiles.openfreemap.org/fonts/{fontstack}/{range}.pbf", sprite:str, layers:[...] }
  GET https://tiles.openfreemap.org/planet → { tilejson:"3.0.0", tiles:[".../planet/<build>/{z}/{x}/{y}.pbf"], maxzoom:14 }
  GET tile z14 over Durrës (9076/6123) → 200 application/vnd.mapbox-vector-tile, size > 10 000 B, ACAO "*"
LINK map.fallback (ours)  version: PMTiles v3, Protomaps basemap schema v4 (GUESS: pin the build date)
  GET https://cdn.dowiz.org/map/durres-tirana-<YYYYMMDD>.pmtiles  Range: bytes=0-16383 → 206, magic "PMTiles" 0x03
```

---

## 3. Geocoder: Nominatim

### 3.1 Current state

| aspect | fact | tag |
|---|---|---|
| What is geocoded | **Reverse only**: pin → street (`address.js:30`, `:92-111`). There is **no forward search** anywhere in `public/**`. A customer finds their door by panning from the venue (or the Durrës fallback) or by "use my location" (`address.js:205-213`) | CITED |
| Call | `GET https://nominatim.openstreetmap.org/reverse?format=jsonv2&lat&lon&zoom=18&accept-language=<page lang>`, once per settled pin (`dragend`/`click`), never per drag frame | CITED |
| Cache | browser `localStorage`, keyed by a ~50 m cell, TTL 30 days; only non-empty answers are kept (`address.js:63-89`, `:107-108`). No Worker or DO cache, a deliberate choice explained at `:55-62` | CITED |
| Timeout / retry | none / none (`:95-110`); failure → `null` → empty line | CITED |
| When down | the "found" line stays empty. Typed street, house, floor and note still work; a pin is optional; an order needs a typed address (`checkout.js:362`), and the geo travels if pinned (`:372`) | CITED |
| CSP | `connect-src … https://nominatim.openstreetmap.org` (`_headers:66`) | CITED |
| User-Agent / Referer | browsers cannot set UA. `Referrer-Policy: strict-origin-when-cross-origin` (`_headers`) sends the origin as Referer, which the policy accepts ("valid HTTP Referer **or** User-Agent") | CITED + SOURCE |
| Live | reverse at sushi-durres coordinates → 200 in 0.43 s, `road: Rruga Sulejman Kadiu`, `city: Bashkia Durrës`, `postcode 2001`, **no house number**. With an `Origin` header → `access-control-allow-origin: *`. `/status` → `software_version 5.3.0`, `data_updated 2026-10-03T05:34:49Z` | MEASURED |
| Address data quality in Durrës | Overpass, Bashkia Durrës: **1 687 named highway ways / 743 unique street names**; objects with `addr:housenumber`: **614** in the whole municipality | MEASURED (overpass-api.de, osm_base 2026-10-03T07:25Z) |
| Coverage | Albania. `accept-language` follows the page (sq/en/uk/ru); Nominatim returns local names when it has no translation | CITED |

**Implication:** house-number geocoding is close to useless in Durrës (614 numbered objects). The pin is the address and the street name is a confirmation. "The client can't find the address on the map" therefore means **there is no way to jump to a street**. That is a missing feature, not an outage.

### 3.2 Failure modes and policy

| mode | likelihood | evidence |
|---|---|---|
| Policy breach → block | LOW at today's volume | "absolute maximum of 1 request per second"; "Auto-complete search … you must not implement such a service"; "Results must be cached"; for apps "the sum of traffic by all your users should not exceed the limits" ([OSMF Nominatim policy](https://operations.osmfoundation.org/policies/nominatim/)). Today: ≤ a few reverse lookups per checkout, cell-cached, so a few hundred per day across all venues (GUESS). **A forward search-as-you-type on Nominatim would breach the policy; never build it** |
| OSMF infrastructure outage | LOW-MED, no SLA | OSM had a multi-day outage in Dec 2024 ([Map Room, 2024-12](https://www.maproomblog.com/2024/12/openstreetmap-outage-resolved/)) |
| Hang | LOW | no timeout (§3.1); only the line stays on "finding address…" |

### 3.3 Options, with the cost of the alternatives

| # | option | cost | notes |
|---|---|---|---|
| **G-A** | **Venue street list, local**: a cron/publish step writes `v/<slug>/streets.json` ([name, lat, lon] per unique street name inside the venue's area) to R2. The checkout gets a search field that filters it in the browser (accent-insensitive: ë→e, ç→c), flies the map to the street, and the customer drops the pin | **MEASURED: Durrës = 743 names, 28.6 KB raw, 10.2 KB gzip.** 1 CDN GET per checkout that opens the map; 0 third-party calls; works offline (the shell SW already caches `cdn.dowiz.org` objects, `sw.js:159-160`) | the most reliable; source data = OSM via Overpass or a Geofabrik extract (`albania-latest.osm.pbf` 51 MB, [Geofabrik](https://download.geofabrik.de/europe/albania.html)), ODbL attribution |
| **G-B** | **Photon** (komoot) forward search **on submit** (Enter or button), not per keystroke, with `bbox=19.26,39.64,21.06,42.66&lat=<venue>&lon=<venue>&limit=5` | free, fair use: "extensive usage will be throttled", no availability guarantee ([photon.komoot.io](https://photon.komoot.io/)). MEASURED: 200 in 0.22 s, ACAO `*`, `version 1.3.0`, import 2026-09-26; `bbox` works (returned Durrës results); **`lang=sq` → 400 "Supported are: default, de, en, fr"**, so send `lang=en` for en and `default` for sq/uk/ru | CSP `connect-src` must add `https://photon.komoot.io` |
| **G-C** | Reverse hardening: `AbortSignal.timeout(4000)`; on failure try Photon `/reverse?lat&lon` (MEASURED 200, returned `street: Rruga Sulejman Kadiu`) | 0 | — |
| G-D | DO geocode cache keyed by normalised query | adds a Worker request and a DO request per lookup, against the Free 100k/day and DO daily caps ([dowiz-costs memory]; Workers Free 100 000/day, [limits](https://developers.cloudflare.com/workers/platform/limits/)) | **rejected**: the browser cache already absorbs the repetition, and the comment at `address.js:55-62` is right |
| G-E | Pin drop without geocoding | already allowed; make it explicit in copy ("street not found — put the pin on your door and describe the entrance") | 0 |
| G-F | Geoapify | free 3 000 credits/day, 5 req/s, autocomplete included ([freetier.co](https://freetier.co/directory/products/geoapify-location-platform); [Geoapify autocomplete](https://apidocs.geoapify.com/docs/geocoding/address-autocomplete/)) | key exposed in the browser (origin-restricted); same OSM data, so no better in Durrës |
| G-G | LocationIQ | free 5 000/day, 2 req/s, autocomplete included ([agentdeals](https://agentdeals.dev/vendor/locationiq)) | same OSM data |
| G-H | Mapbox Temporary Geocoding | free 100 000/month, then $0.75/1 000 ([Mapbox evaluation](https://www.mapbox.com/geocoding/evaluation), [woosmap 2026](https://www.woosmap.com/blog/mapbox-pricing)) | "temporary" results may not be stored (GUESS on clause wording; read Mapbox ToS before use) |
| G-I | Google Places Autocomplete | since 2025-03-01: 10 000 free Essentials calls/month per SKU, $2.83/1 000 per keystroke without session tokens ([Google March 2025](https://developers.google.com/maps/billing-and-pricing/march-2025)) | its terms tie results to a Google map (GUESS, verify), which conflicts with our OSM map; most expensive |

**Recommendation:** G-A + G-C + G-E (S-M), then G-B (S).
- Expected third-party calls per checkout: 0-1 Photon plus 0-2 Nominatim reverse, both cached.
- Worker requests added: 0.
- Latency: local filter < 5 ms (GUESS), Photon ≈ 0.2 s (MEASURED).

### 3.4 Contracts

```
LINK geo.reverse.primary  Nominatim 5.3.0, API "reverse" format=jsonv2
  GET https://nominatim.openstreetmap.org/reverse?format=jsonv2&lat=41.315347&lon=19.4449964&zoom=18&accept-language=sq
  → 200 { place_id:int, lat:str, lon:str, display_name:str,
          address:{ road?:str, pedestrian?:str, house_number?:str, city?|town?|village?:str, postcode?:str, country_code:"al" } }
  probe invariant: address.country_code=="al" && (road||pedestrian) non-empty; ACAO "*" when Origin sent
LINK geo.reverse.fallback / geo.search  Photon 1.3.0
  GET https://photon.komoot.io/api/?q=<text>&bbox=19.26,39.64,21.06,42.66&lat=..&lon=..&limit=5&lang=<en|default>
  GET https://photon.komoot.io/reverse?lat=..&lon=..
  → 200 GeoJSON FeatureCollection { features:[{ geometry:{type:"Point",coordinates:[lon,lat]},
          properties:{ name?|street?:str, housenumber?:str, city?:str, countrycode:"AL", osm_key, osm_value } }] }
  refusal: lang outside {default,de,en,fr} → 400 (MEASURED) — the client must map sq/uk/ru → "default"
LINK geo.streets (ours)  schema v1
  R2 dowiz-cdn v/<slug>/streets.json → { v:1, slug, source:"osm", osmBase:"RFC3339", bbox:[w,s,e,n],
                                          streets:[[name:str, lat_udeg:int, lon_udeg:int], ...] }  (integers, MANIFESTO C2)
```

---

## 4. The aggregator (`command/aggregator.rs`): the operator's main item

### 4.1 What exists, honestly

| piece | where | what it does |
|---|---|---|
| Rules | `workers/api/src/command/aggregator.rs` (237 lines) | `Entry {channel, external_id, lines[{product_id, quantity, unit_price}], discount, total}` (`:38-56`). The id is `<channel>-<external_id>` (`:67-84`), and the platform id **is** the idempotency key. Refuses `total ≠ lines − discount` (`:158-163`). Born `CONFIRMED`, `payment: platform`, `paid`, `price_trusted:false`, **pickup only**; a marketplace the venue delivers for is refused as "not modelled" (`:200-204`). `bell()` prints "WOLT <number>" for Telegram/print (`:213-234`) |
| Object half | `hubdo/aggregator.rs` (22 lines) | same content = `existing:true`; different content = named conflict (`same_entry`, `aggregator.rs:96-128`) |
| HTTP | `POST /api/staff/orders/aggregator` (`lib.rs:391`) → `services/orders/aggregator.rs:21-115`: staff `Cap::TakeOrders`, idempotency-key guard, dishes resolved via `/fold/basket`, unknown dish → 400 |
| UI | `public/admin/aggregator.js` (90 lines): a sheet with platform select `['wolt','glovo','baboon']` (`:16`), platform number, dishes, discount, total, and a live "adds up" check. Opened from the **Orders** screen header button `#oAgg` "Platform order" (`orders.js:161`, `:183`). i18n in sq/en/uk (`i18n.js:48,177,289`) and ru (`i18n-ru.js:40`) |
| Tests | `command/aggregator/tests.rs` (201 lines, 10 `#[test]`s): money law, refusal shapes, ledger reservation, out-of-stock refusal, bell text, the conflict twin. **All in-memory**; there is no live proof |
| Channel set | `services/ordering/channel.rs:32-34` `WOLT, GLOVO, BABOON`; "no adapter speaks to any of them yet" (`:29-31`) |
| Wolt adapter / mock | **none.** `grep -rn "WOLT-SIGNATURE\|hooks/wolt\|woltapi" workers/api/src` → 0. Roadmap F4 (`ROADMAP-2026-09-22.md:337`) and "F4-mock" (`:540`, operator-approved 2026-09-26) are **not built**. The operator's sentence "it works against a mock, not real Wolt" overstates it: what exists is **manual entry**, and nothing talks to Wolt or to any mock of Wolt |
| Integrations screen | `public/admin/more.js:618-627` lists telegram, whatsapp, instagram, webhook, cloud, mcp, stripe, ai. **No aggregator row.** Backend `src/integrations.rs:34-60` reports no aggregator state |

### 4.2 Which platforms actually operate in Albania (2026-10-03)

| platform | in Albania? | evidence | partner API | verdict |
|---|---|---|---|---|
| **Wolt** | **Yes, 7 cities**: Tirana, Durrës, Elbasan, Fier, Sarandë, Shkodër, Vlorë | MEASURED: `wolt.com/en/alb` lists `/en/alb/{durres,elbasan,fier,sarande,shkoder,tirana,vlora}`; `wolt.com/en/alb/durres` 200 (≈1 258 venue/name entries in the page). Launch: Tirana 2024-03-13 with 130+ venues ([albaniatech](https://albaniatech.org/wolt-officially-enters-the-albanian-market/)); Durrës March 2025 ([albaniatech](https://albaniatech.org/wolt-expands-to-durres-bringing-seamless-delivery-to-the-coastline/)); 700+ merchants at year one ([albaniatech](https://albaniatech.org/wolt-celebrates-first-anniversary-in-albania-a-year-of-growth-innovation-and-impact/)). `wolt.com/en/alb/wolt-drive` exists | Order, Menu, Venue APIs + webhooks + OAuth 2.0 (§4.3) | **The only real target.** Build it |
| **Glovo** | **No** | "Glovo suspends plans to launch in Albania" ([SeeNews, 2024-04-19](https://SeeNews.com/news/glovo-suspends-plans-to-launch-in-albania-855204)). MEASURED: `glovoapp.com/al/en/tirana/` and `/durres/` redirect to `glovoapp.com/en`. **Corrects** BLUEPRINT-LAUNCH-GAPS §2.2 ("Tirana; limited expansion to Durrës") | Delivery Hero Partner API v2 (in that blueprint) | drop from the Albanian UI; keep the channel word |
| **Bolt Food** | **No** | "Bolt's delivery arm doesn't operate in Albania" ([himara.net](https://himara.net/blog/bolt-in-albania)); Wikipedia country list as of 2023 (blueprint) | Bolt Stores API elsewhere | out of scope |
| **Baboon** (local) | Yes (Tirana, Durrës, Vlorë) | blueprint §2.2 sources | **none public** | manual entry only |
| **Deliverect / Otter / middlewares** | Global; Albanian presence UNVERIFIED | Deliverect has a two-way Wolt integration ([deliverect.com/integrations/wolt](https://website-dev.deliverect.com/integrations/wolt)); pricing is quote-only ([Deliverect FAQ](https://deliverect.com/en/faq)) | dowiz would integrate once as a "POS" into Deliverect and reach Wolt through it | **Not recommended.** It adds a monthly per-location fee (GUESS €60-120) and a second vendor, while Wolt is the only platform worth reaching. Revisit only if Wolt refuses dowiz as a POS provider |

### 4.3 The real Wolt integration as of 2026 (SOURCE: developer.wolt.com, read 2026-10-03)

**Who may integrate.** "This is for Restaurant Point of Sale (POS) and Middleware providers (MWP)." Individual venues may not. The steps are:
1. Submit the integration request (a form);
2. Wolt sends "minimum requirements and T&Cs";
3. test credentials;
4. sandbox "that mirrors our live system";
5. demo/QA;
6. "launch at a single pilot venue first".

Partners must implement OAuth 2.0 + Order + Menu + Venue APIs before sandbox testing ([Getting started — restaurant](https://developer.wolt.com/docs/getting-started/restaurant)). Integration fees are not published (UNVERIFIED; ask Wolt).

**How a venue connects: Self-Service Integration Onboarding (SSIO), OAuth 2.0 authorization code** ([SSIO](https://developer.wolt.com/docs/authenticationssio), [Authentication 2.0](https://developer.wolt.com/docs/authentication20)):
- The integration URL is dev `https://developer.development.dev.woltapi.com/integrate`, prod `https://developer.wolt.com/integrate`. Its parameters are `client_id`, `redirect_url` (pre-registered), `state` (≥ 8 chars), and optionally `venue_name`, `venue_address`.
- The merchant signs in with their Wolt account, "selects exact venue", and accepts the T&C. The redirect then returns `code` (single-use, valid 1 h), `state` and `scope` (offline).
- Tokens come from dev `https://integrations-authentication-service.development.dev.woltapi.com/oauth2/token` or prod `https://integrations-authentication-service.wolt.com/oauth2/token`.
  - `grant_type=authorization_code`: `code, redirect_uri, client_id, client_secret`.
  - `grant_type=refresh_token`: `refresh_token, client_id, client_secret`.
- **Access token: 1 h. Refresh token: 30 days, single-use, rotated.** Reusing one "will immediately deactivate and revoke the newly issued access and refresh tokens".
- The Wolt venue id is in the JWT payload: `integration.venue_id`.
- API base: `https://pos-integration-service.wolt.com` (e.g. `/orders/{order_id}`). Auth: `Authorization: Bearer <access JWT>`.

**Webhook** ([Webhook](https://developer.wolt.com/docs/webhook)):
- Payload: `{ id, type:"order.notification", order:{ id, venue_id, status, resource_url }, created_at }`. Statuses: `CREATED, PRODUCTION, READY, CANCELED, COURIER ARRIVAL, PICK-UP-COMPLETED, DELIVERED, REVIEW`. Venue events: `VENUE_OFFLINE_ALERT_TRIGGERED/RECOVERED`, `REJECTION_ALERT_*`, `OPENING_HOURS_UPDATED`.
- Signature: header `WOLT-SIGNATURE` = HMAC-SHA256, **hex**, over the raw body, keyed by the webhook client secret (≥ 128 bits, which the partner provides).
- The webhook must answer **200**. Wolt retries 3 times, 5 s apart, and after that the event is gone.
- The partner gives Wolt the webhook URL, the secret and the trigger set (`CREATED, PRODUCTION, READY, DELIVERED, CANCELED`).

**Order API** ([Order API](https://developer.wolt.com/docs/api/order)):
- `GET /v2/orders/{orderId}` (recommended, the "most comprehensive payload").
- `PUT /orders/{id}/accept`, `/self-delivery/accept`, `/reject` (with a reason), `/ready`, `/delivered`, `/confirm-preorder`.
- Money: `{amount:int minor units, currency}`.
- **No list/poll endpoint is documented.** Reconciliation after a lost webhook is therefore impossible, and the manual sheet stays as the fallback.

**Menu API** ([Menu API](https://developer.wolt.com/docs/api/menu)): `POST /v1/restaurants/{venueId}/menu` (full push), `GET /v2/venues/{venueId}/menu`, `PATCH /venues/{venueId}/items` (availability/price), `PATCH /venues/{venueId}/items/inventory`, `PATCH /venues/{venueId}/options/values`. Items are keyed by `external_data` / `sku` / `gtin`, and in the v2 order they come back as `pos_id`/`sku`.

**Venue API** ([Venue API](https://developer.wolt.com/docs/api/venue)): `GET /venues/{venueId}/status`, `PATCH /venues/{venueId}/online` (ONLINE/OFFLINE, optional `until` = **pause**), `PATCH /venues/{venueId}/opening-times`, `PUT …/special-opening-times`, `GET|PATCH …/delivery-provider`.

**Sandbox:** the dev environment `*.development.dev.woltapi.com`, plus a test venue issued by Wolt. A public Postman collection exists.

**Wolt Drive** (couriers for dowiz's own storefront orders) exists and has an Albanian page. Its API uses `/shipment-promises` and `/deliveries`, credentials come "from the local Wolt Drive team", and JWT-signed status webhooks ([Wolt Drive](https://developer.wolt.com/docs/wolt-drive)). It is out of scope here; noted as a later G3 "multi-fleet" option.

### 4.4 Design decisions this changes from BLUEPRINT-LAUNCH-GAPS §2.3 (2026-09-24)

1. **Credentials are platform-wide, tokens are per venue.** dowiz holds one `client_id`/`client_secret` (Worker secrets `WOLT_CLIENT_ID`, `WOLT_CLIENT_SECRET`) and one webhook secret (`WOLT_WEBHOOK_SECRET`). Each venue holds only its **refresh token** and `wolt.venue_id` in its own settings image, written by the callback and never read back to the browser. The blueprint's "per-venue `client_secret`/`api_key`" predates Authentication 2.0.
2. **One webhook URL for the integration:** `https://dowiz.org/api/hooks/wolt`. It routes by `order.venue_id` through a `wolt.venue_id → slug` index in the `__platform` DO, written at connect. GUESS: Wolt registers one URL per integration; if Wolt allows one per venue, `https://<slug>.dowiz.org/api/hooks/wolt` works with the same code.
3. **Answer 200 only after a durable write, and do the work in the DO alarm.** The blueprint answered 5xx on fetch failure and relied on three retries 5 s apart. That loses the order on a 15-second blip. Instead:
   - verify the signature;
   - append the raw event to the venue DO's `wolt.inbox`;
   - answer 200 (≈ 2 DO requests, < 2 ms CPU, GUESS);
   - the DO alarm then fetches `/v2/orders/{id}`, places the order, and PUTs accept, retrying with backoff through the existing outbox (`outbox/rails.rs`).
   - **Nothing acknowledged to Wolt is ever lost while R2/DO are up.**
4. **Token refresh is single-flight inside the venue DO** (its turn serialises it). A second concurrent refresh would revoke both (SOURCE above). The access token lives in DO memory/storage with `exp`. The nightly 03:17 cron (or a DO alarm at day 20) refreshes any refresh token older than 20 days, so a quiet venue never hits the 30-day wall. The console shows "Wolt link expires in N days" when below 7.
5. **Currency minor units: UNVERIFIED, must be checked in the sandbox.** Wolt sends `amount` in ISO minor units, and ISO 4217 gives ALL **2** decimals (qindarka), while dowiz's `Currency::ALL` has **0** (`decimals:{ALL:0}`, MEASURED in `/api/public/rates`). If Wolt sends lek ×100, the mapper must divide by 100 and **refuse** any amount not divisible by 100. Otherwise a 900-lek dish becomes 90 000 lek: the "1500 lek drawn as $15.00" class of bug ([kit-had-a-fourth-money-copy] memory). The first sandbox order settles this; until then the mapper refuses ALL amounts it cannot prove.
6. **Self-delivery orders** (`delivery.self_delivery:true`, Wolt "Hybrid") are refused today by `aggregator.rs:200-204`. Phase 1 places only Wolt-courier orders. A self-delivery order is **rejected with a reason** before acceptance, and the console says why. Modelling them (they have an address and coordinates in `delivery.location`) is a later row.
7. **Glovo is removed from the Albanian sheet** (`aggregator.js:16`), and the channel word stays (no deletions). The sheet offers `wolt` and `baboon`.

### 4.5 The simplest owner flow (console, "no backend without UI")

**Where:** Settings → **Integrations** gets one card per aggregator: `wolt`, `baboon`. Each card is also a row in the existing `INTEGRATIONS` list (`more.js:618`), so the "Check all" proof button covers it. The Orders screen keeps its "Platform order" button.

```
┌ Wolt ───────────────────────────────── ● green / ● amber / ● red / ○ grey ┐
│ Connected · venue "Sushi Durrës" (Wolt id 64f…)                            │
│ Last order WOLT 4F2K · 12 min ago · 2 400 L                                │
│ Menu sent 3 Oct 10:14 · 165 dishes · 0 not matched                        │
│ [ Pause 30 min ▾ ]  [ Send menu ]  [ Enter order by hand ]  [ ⋯ Disconnect ]│
└────────────────────────────────────────────────────────────────────────────┘
```

Status-light rules, computed by the server and never by the browser:

| light | condition |
|---|---|
| grey | `WOLT_CLIENT_ID` absent. Card shows "Waiting for Wolt partner approval" + [Enter by hand] |
| amber | client present, venue not connected → [Connect Wolt]; or connected and paused; or refresh token < 7 days |
| green | connected, last token refresh OK, last inbox item placed |
| red | last refresh failed, OR an inbox item older than 2 min not placed, OR an outbox PUT abandoned ("Wolt was not told") |

Copy in four languages (keys `ag_*`, added to `admin/i18n.js` sq/en/uk and `i18n-ru.js`):

| key | sq | en | uk | ru |
|---|---|---|---|---|
| ag_connect | Lidh Wolt | Connect Wolt | Підключити Wolt | Подключить Wolt |
| ag_waiting | Në pritje të miratimit nga Wolt | Waiting for Wolt partner approval | Очікує схвалення партнерства Wolt | Ожидает одобрения партнёрства Wolt |
| ag_last | Porosia e fundit | Last order | Останнє замовлення | Последний заказ |
| ag_none | Asnjë porosi ende | No orders yet | Замовлень ще немає | Заказов пока нет |
| ag_menu | Dërgo menunë | Send menu | Надіслати меню | Отправить меню |
| ag_pause | Pushim | Pause | Пауза | Пауза |
| ag_byhand | Fut me dorë | Enter by hand | Ввести вручну | Ввести вручную |
| ag_notTold | Wolt nuk u njoftua | Wolt was not told | Wolt не отримав відповідь | Wolt не получил ответ |
| ag_expires | Lidhja skadon pas {n} ditësh | Link expires in {n} days | Зв'язок спливає через {n} дн. | Связь истекает через {n} дн. |

(The sq and ru strings are drafts; have a native speaker review them.) ASCII quotes only in JS strings (COMMON-RULES 11).

### 4.6 Contracts

```
LINK agg.status (ours, new)   GET /api/owner/integrations → adds
  "aggregators": {
    "wolt":   { "mode": "off"|"manual"|"connected", "partner": bool,          // partner = WOLT_CLIENT_ID present
                "venueId": str|null, "paused": bool, "pausedUntilMs": int|null,
                "lastOrderMs": int|null, "lastOrderRef": str|null, "lastTotal": int|null,   // venue minor units
                "menuSentMs": int|null, "menuItems": int|null, "unmatched": int,
                "tokenExpiresInDays": int|null, "inboxWaiting": int, "lastError": str|null,
                "light": "grey"|"amber"|"green"|"red" },
    "baboon": { "mode": "manual", "lastOrderMs": int|null, "lastOrderRef": str|null, "light": "grey"|"green" } }
  POST /api/owner/integrations/check {which:"wolt"} → { ok:true, detail:{ venueId, online:bool } }  // GET /venues/{id}/status
LINK agg.manual (exists)       POST /api/staff/orders/aggregator   Cap::TakeOrders, header idempotency-key
  req  { location_id:str, channel:"wolt"|"baboon"|"glovo", external_id:/^[A-Za-z0-9_-]{1,64}$/,
         lines:[{product_id:str, quantity:1..99, unit_price:int>=0}], discount:int>=0, total:int }  total==Σ−discount
  resp 200 { order:{ order_id:"<channel>-<external_id>", status:"CONFIRMED", payment:"platform", price_trusted:false, ... },
             existing:bool }   409 conflict on same id/other content; 400 dish not on menu
LINK wolt.connect (new)        GET /api/owner/wolt/connect (owner) → 302
  https://developer.wolt.com/integrate?client_id=..&redirect_url=https://dowiz.org/api/wolt/callback&state=<HMAC(slug,nonce,exp)>
  GET /api/wolt/callback?code&state&scope → token POST (authorization_code) → store {refresh, venue_id from JWT integration.venue_id}
  → index __platform wolt.venue_id→slug → 302 https://<slug>.dowiz.org/admin/#integrations
LINK wolt.hook (new)           POST https://dowiz.org/api/hooks/wolt     Wolt webhook (no version header; payload "order.notification")
  verify: hex(HMAC_SHA256(WOLT_WEBHOOK_SECRET, raw_body)) == header WOLT-SIGNATURE (constant time) else 401, nothing written
  body  { id:str, type:"order.notification", order:{ id:str, venue_id:str, status:enum, resource_url:url }, created_at:RFC3339 }
  → unknown venue_id: 200 + loud("wolt.unknown_venue") (no retries needed; nothing to place)
  → known: append inbox {event_id:id, order_id, status, at} (idempotent on id) → 200
LINK wolt.order (Wolt v2)      GET https://pos-integration-service.wolt.com/v2/orders/{id}  Bearer
  used fields: id, order_number, venue.id, type (instant|preorder), pre_order, delivery.self_delivery,
               items[].{pos_id|sku, count, item_price.unit_price.amount, name}, basket_price.total.amount,
               basket_price.price_breakdown.total_discounts.amount, *.currency, consumer_comment, pickup_eta
  map → aggregator::Entry{ channel:"wolt", external_id:order_number|id, lines:[{product_id:pos_id, quantity:count,
        unit_price:unit_price.amount/ALL_SCALE}], discount:total_discounts/ALL_SCALE, total:basket_total/ALL_SCALE }
        ALL_SCALE ∈ {1,100} — PINNED BY THE FIRST SANDBOX ORDER (§4.4.5); fees.* never become lines
LINK wolt.status_out (Wolt v1) PUT /orders/{id}/accept | /reject {reason} | /ready | /confirm-preorder   via outbox kind "wolt"
  FSM CONFIRMED→accept, READY→ready, dowiz cancel before accept→reject; Wolt CANCELED→refund route, method "platform"
LINK wolt.menu (Wolt v1)       POST /v1/restaurants/{venueId}/menu  (external_data = dowiz product_id); PATCH /venues/{venueId}/items
LINK wolt.pause (Wolt)         PATCH /venues/{venueId}/online { status:"OFFLINE"|"ONLINE", until?:RFC3339 }   (body fields UNVERIFIED; pin from sandbox)
LINK wolt.token (OAuth 2.0)    POST https://integrations-authentication-service.wolt.com/oauth2/token
  form: grant_type=refresh_token&refresh_token&client_id&client_secret → { access_token(JWT,1h), refresh_token(30d, single-use), expires_in }
```

### 4.7 Cost on Cloudflare Free (GUESS from the shapes above)

Per Wolt order:
- ≈ 4-5 webhooks (CREATED, PRODUCTION, READY, DELIVERED, maybe CANCELED), each costing 1 Worker request + 1-2 DO requests;
- 1 GET order and 1-2 PUTs as subrequests from the DO;
- about 1 token refresh per hour of activity.

At 100 Wolt orders/day per venue that is ≈ 500 Worker + ≈ 1 000 DO requests/day, which is 0.5 % of the 100 000/day Worker cap. Webhook CPU: HMAC over < 1 KB + a JSON parse, well under the 10 ms Free CPU limit. The placement work runs in the DO alarm through `/fold/basket` (no catalogue image load, BN1), the path that already places manual entries.

### 4.8 What the operator must obtain (exact list)

1. **Wolt partner application** as a *POS provider* (dowiz = the venue's POS/ordering system), via the integration request form linked from [developer.wolt.com → Getting started (restaurant)](https://developer.wolt.com/docs/getting-started/restaurant). Have a legal entity name, a contact, and the venues (sushi-durres, dubin-sushi) ready as pilot candidates.
2. Accept Wolt's minimum requirements and T&C. Ask explicitly:
   - (a) integration fee, if any;
   - (b) whether the webhook URL is per integration or per venue;
   - (c) whether ALL amounts are sent in qindarka (×100);
   - (d) the `PATCH /online` body;
   - (e) whether an order-list endpoint exists for reconciliation.
3. Receive the **dev credentials**: `client_id`, `client_secret` for `*.development.dev.woltapi.com`, plus a **test venue**.
4. Register with Wolt:
   - redirect URL `https://dowiz.org/api/wolt/callback`;
   - webhook URL `https://dowiz.org/api/hooks/wolt`;
   - the webhook secret (generate 32 random bytes, hex; keep it only in `wrangler secret put WOLT_WEBHOOK_SECRET`);
   - trigger set `CREATED, PRODUCTION, READY, DELIVERED, CANCELED`.
5. After QA: the **prod** `client_id`/`client_secret`, and the venue owner (a Wolt merchant account holder) clicks **Connect Wolt** in the console. That click is the SSIO consent; no key is ever typed.
6. Worker secrets to set: `WOLT_CLIENT_ID`, `WOLT_CLIENT_SECRET`, `WOLT_WEBHOOK_SECRET`, `WOLT_ENV` (`dev`|`prod`).

---

## 5. What a live probe can and cannot prove today

| link | live-provable now? | probe |
|---|---|---|
| rates via Worker | YES | `curl https://qa-durres.dowiz.org/api/public/rates?base=ALL` → schema §1.4, `stale:false`, `asOf` < 30 h old |
| rates upstreams | YES | the two GETs in §1.4 |
| tiles OFM | YES | style + tilejson + z14 Durrës tile (§2.4) |
| tiles fallback | after MP2 | `Range: bytes=0-16383` on the PMTiles object → 206 + magic |
| Nominatim / Photon | YES (one request each, cached) | §3.4 |
| aggregator manual entry | YES, with an owner/staff token on **qa-durres** | POST the manual route with `idempotency-key`, read the order back from `/api/owner/orders`, and check `aggregators.wolt.lastOrderMs` |
| Wolt hook signature refusal | after AG2 | unsigned POST → 401, and the inbox count is unchanged |
| Wolt end to end | **NEEDS-KEY** (Wolt dev credentials) | sandbox order → console within 10 s → Wolt shows accepted. A self-signed fixture is a mock and is never reported as proof (live-proof rule) |

---

## 6. Phased plan (lane-sized rows)

Order: first what removes user-visible defects without keys, then the aggregator surface, then Wolt behind the key.

| row | goal | files (owned) | acceptance | live probe on qa-durres | size |
|---|---|---|---|---|---|
| **RT1** | Rates: no hang, no 1-h stale pin, attribution, `asOf` shown | `workers/api/src/services/ordering/rates.rs` (+`rates/tests.rs`), `public/lib/money.js`, `public/store/state.js`, `public/app.js` (one line: do not `await` rates before first paint), store i18n (4 langs) | upstream fetch wrapped in a 3 s abort; stale cached ≤ 60 s (`max-age=60`); test: upstream `Err` → `stale:true` and cache-control 60; positive twin `max-age=3600`; `grep -c "Rates By Exchange Rate API" public` ≥ 1 | GET rates → 200 schema-valid; storefront with `localStorage dowiz.currency=EUR` paints before rates arrive (browser walk: loader gone < 1.5 s with rates blocked by route interception) | S |
| **RT2** | Rates snapshot in R2 with two sources + last-known-good | `workers/api/src/cron.rs` (one call), new `src/services/ordering/rates/snapshot.rs` (+tests), `rates.rs` reads R2 first | cron writes `rates/ALL.json` v2 (§1.4); a ±20 % jump refused loud; er-api `Err` → currency-api used, `source` says so; both `Err` → object untouched | after a cron run, `curl https://cdn.dowiz.org/rates/ALL.json` → v2, `asOf` today; `/api/public/rates` equals it | M |
| **RT3** | Owner manual rate | settings key `rates.manual.*`, `public/admin/` currency sheet (4 langs), `rates.rs` | manual wins and is labelled `source:"manual"`; positive and negative tests | set a manual EUR on qa-durres → storefront shows "rate set by the venue" | S |
| **MP1** | No silent grey map | `public/store/address.js`, `store/venue.js`, `store/track-map.js`, `courier/app.js`, `kit/screens/map.js`, i18n | every `new Map` has `on('error')`; ≥ 3 tile errors in 5 s → visible message "map unavailable — type the street" (4 langs) | browser walk with `tiles.openfreemap.org` blocked: message shown, order placeable with a typed street | S |
| **MP2** | PMTiles fallback on cdn.dowiz.org | new `.github/workflows/map-extract.yml` (weekly `pmtiles extract` from Protomaps, upload to R2), new `public/lib/map/pmtiles.js` (vendored) + `public/lib/map/fallback-style.json` + glyphs/sprite, `store/ui.js` (`loadMapLib` registers the protocol), `_headers` (nothing new if served from cdn.dowiz.org) | object ≤ 25 MB; style switch on failure; MapLibre renders Durrës streets from the fallback | Range GET → 206 + `PMTiles` magic; browser walk with OFM blocked counts > 0 tile ranges from cdn.dowiz.org and > 0 rendered features (judge by request counts, per memory `storefront-map-was-umd-import`) | M |
| **MP3** | Courier tile cache | `public/courier/sw.js` | cache-first for `tiles.openfreemap.org` tiles, fonts and sprites; LRU cap 1500; SHELL version bumped | courier walk: second load with network off shows a map of the previous area | S |
| **GC1** | Reverse geocode hardening | `public/store/address.js`, `_headers` (+`https://photon.komoot.io`), `e2e/kit-regression/_local.mjs` reads the CSP from `_headers` (already does) | 4 s timeout; Photon reverse on failure; `lang` mapping sq/uk/ru → `default` | pin on qa-durres resolves a street with Nominatim blocked (browser walk) | S |
| **GC2** | Venue street list + search box | new `crates/dowiz-hub` or `tools/streets/` builder (Overpass or Geofabrik → JSON), publish step in `src/hubdo/publish.rs` (`v/<slug>/streets.json`), `public/store/address.js` search field, i18n | Durrës list ≈ 743 names, ≤ 15 KB gz; accent-insensitive match ("durres" finds "Durrës"); choosing a street flies the map there | `curl https://cdn.dowiz.org/v/qa-durres/streets.json` → schema §3.4; walk: type "Sulejman", pick, and the pin lands within 300 m of `41.3153, 19.4447` | M |
| **GC3** | Photon forward search on submit | `public/store/address.js` | at most 1 Photon request per submit; none per keystroke (test counts requests) | walk: unknown-locally text → Photon results listed | S |
| **AG0** | **Aggregator visible: Integrations card + truthful sheet** | `public/admin/more.js` (INTEGRATIONS row), new `public/admin/aggregators.js` + `aggregators-i18n.js` (sq/en/uk/ru), `src/integrations.rs` (`aggregators` block, light rules §4.5), `public/admin/aggregator.js` (platform list `wolt`,`baboon`) | `/api/owner/integrations` returns `aggregators.wolt.mode=="manual"`, `light=="grey"`, `lastOrderMs` from the newest `wolt-*` order; the card renders in 4 langs; the [Enter by hand] button opens the existing sheet; gates `ui-reach`, `langs` GREEN | owner login on qa-durres → GET integrations → schema §4.6; enter one manual Wolt order → `lastOrderRef` equals it (read back) | S |
| **AG1** | Pause and menu export without keys | `aggregators.js`, `src/integrations.rs` | "Pause" in grey mode explains that it pauses the Wolt tablet manually (copy only); "Send menu" disabled with reason | walk shows reasons | XS (fold into AG0) |
| **AG2** | Wolt webhook intake, durable | new `src/wolt/{hook,inbox,sig}.rs` (+tests), `src/hubdo/wolt.rs`, `lib.rs` (1 route, hand-back) | wrong/missing `WOLT-SIGNATURE` → 401 and nothing written (named RED test, then GREEN); same event id twice → one inbox row; unknown venue → 200 + loud; inbox gauge in `aggregators.wolt.inboxWaiting` | unsigned POST to `https://dowiz.org/api/hooks/wolt` → 401 (MEASURED after deploy); end to end is **NEEDS-KEY** | M |
| **AG3** | Wolt connect (SSIO OAuth) + token keeper | `src/wolt/{oauth,token}.rs`, `__platform` index `wolt.venue_id→slug`, `aggregators.js` [Connect] | state HMAC-bound to slug + expiry; refresh single-flight in the DO; a 20-day-old refresh token is rotated by the nightly cron; refresh failure → red light and `lastError` | **NEEDS-KEY**: dev env, connect qa-durres to the Wolt test venue, light amber→green, `check` → `GET /venues/{id}/status` 200 | M |
| **AG4** | Order in → placed → accepted | `src/wolt/{fetch,map}.rs`, outbox kind `wolt` in `src/outbox/rails.rs`, `command/aggregator.rs` (unmapped line flagged, not refused; self-delivery rejected with reason) | `ALL_SCALE` pinned by a recorded sandbox payload stored as a fixture; fees never lines; unmapped `pos_id` flagged; accept PUT retried by the outbox; abandoned → "Wolt was not told" | **NEEDS-KEY**: sandbox order → console WOLT badge within 10 s; Wolt dev shows accepted; read back via `/v2/orders/{id}` `order_status` | M |
| **AG5** | Status out + cancel in | `src/wolt/push.rs`, FSM hook in the advance path (hand-back) | READY→`/ready`; dowiz reject before accept→`/reject{reason}`; Wolt `CANCELED`→refund route, stock hold released | **NEEDS-KEY**: sandbox cancel → order ends refunded on qa-durres | S-M |
| **AG6** | Menu sync + pause | `src/wolt/menu.rs`, `aggregators.js` | push with `external_data=product_id`; 86 → `PATCH items`; pause → `PATCH /online` with `until` | **NEEDS-KEY**: `GET /v2/venues/{id}/menu` item count = dowiz dish count; pause shows OFFLINE in `GET /venues/{id}/status` | M |
| **LP-EXT** | Live-proof contracts for every external link above | `tools/live-proof/contracts/{rates,tiles,geo,wolt}.json` + probes (lands on W-LIVE's harness) | each probe validates the live response against §1.4/§2.4/§3.4/§4.6; no key → `NEEDS-KEY`, never PASS | nightly `live-proof.yml` run green for rates/tiles/geo; Wolt rows NEEDS-KEY until AG3 | S |

**Row list:** RT1, RT2, RT3, MP1, MP2, MP3, GC1, GC2, GC3, AG0(+AG1), AG2, AG3, AG4, AG5, AG6, LP-EXT.
- Lanes that can run in parallel (file-disjoint): {RT1}, {MP1, MP3}, {AG0}.
- Then: {RT2, GC1}, {MP2}, {AG2}.
- AG3-AG6 wait for the Wolt dev credentials.

---

## 7. Not verified, and what would settle it

| item | why open | how |
|---|---|---|
| Why a cache HIT on `/api/public/rates` returns `max-age=14400` | zone setting not readable with our tokens | CF dashboard → Caching → Browser Cache TTL |
| Wolt: webhook URL per integration or per venue; ALL minor units; `/online` body; order-list endpoint; integration fee | not in public docs | Wolt's credentials pack / account manager (§4.8 step 2) |
| Whether Wolt accepts dowiz as a "POS provider" | Wolt's decision | the application |
| Exact PMTiles size for the Durrës–Tirana bbox | `pmtiles extract` not run (compute/network budget of a research lane) | `pmtiles extract https://build.protomaps.com/<date>.pmtiles x.pmtiles --bbox=19.38,41.24,19.95,41.42 --maxzoom=14 && ls -l x.pmtiles` |
| R2 Class B billing for edge-cached reads | the doc does not say | R2 metrics after MP2 runs a week |
| Whether a cached `cdn.dowiz.org` range read is billed | `cf-cache-status: DYNAMIC` today, so every read is an R2 op | same |
| How many manual Wolt entries exist live | needs an owner token (login is a POST) | `GET /api/owner/orders` filter `wolt-` on sushi-durres |
| Deliverect in Albania, and its price | quote-only | Deliverect sales |

Sources read 2026-10-03:
- OSMF [Nominatim policy](https://operations.osmfoundation.org/policies/nominatim/)
- [OpenFreeMap](https://openfreemap.org/), [Gigazine 2025-08-12](https://gigazine.net/news/20250812-openfreemap-survived-100000-requests-wplace-live/)
- [ExchangeRate-API free](https://www.exchangerate-api.com/docs/free), [Bank of Albania](https://www.bankofalbania.org/Markets/Official_exchange_rate/)
- Cloudflare [R2 pricing](https://developers.cloudflare.com/r2/pricing/), [Workers limits](https://developers.cloudflare.com/workers/platform/limits/)
- [Geofabrik Albania](https://download.geofabrik.de/europe/albania.html), [Protomaps](https://docs.protomaps.com/guide/getting-started), [Photon](https://photon.komoot.io/)
- Geoapify/LocationIQ/Mapbox/Google pages as linked in §3.3
- Wolt: [restaurant getting started](https://developer.wolt.com/docs/getting-started/restaurant), [SSIO](https://developer.wolt.com/docs/authenticationssio), [Auth 2.0](https://developer.wolt.com/docs/authentication20), [Webhook](https://developer.wolt.com/docs/webhook), [Order](https://developer.wolt.com/docs/api/order), [Menu](https://developer.wolt.com/docs/api/menu), [Venue](https://developer.wolt.com/docs/api/venue), [Drive](https://developer.wolt.com/docs/wolt-drive)
- [SeeNews Glovo 2024-04-19](https://SeeNews.com/news/glovo-suspends-plans-to-launch-in-albania-855204), [himara.net Bolt](https://himara.net/blog/bolt-in-albania), albaniatech Wolt articles linked in §4.2
