# The six fundamental bottlenecks — every technique, numbers here, and where orders of magnitude are and are not

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** BN1-BN8 APPROVED by the operator on 2026-10-02 and copied as Wave BN into `docs/design/ROADMAP-2026-09-22.md` with their state: BN1A IN FLIGHT (dataflow 38 → 23 in its tree), BN7 IN FLIGHT (`kv_get_10k` 526 ns; the bebop-wasm size ratchet RED, control build owed), BN1B/BN2/BN3/BN4/BN5/BN6/BN8 QUEUED in the order §9 gives. Nothing below was changed.


Lane W-BOTTLENECK, 2026-10-01 → 2026-10-02 (sessions 3e1fb658, 748e569d), model Fable. A RESEARCH lane: no product code changed,
no deploys, no git writes. Tree read at `01783eb4` (`/root/dowiz`; the first session read `b29310c4`). Builds on
`docs/research/2026-10-01-cloudflare-free-cost-and-rust-web.md` (the cost map — not redone here; its rows are cited as A1–A12, C0–C6, R1–R6),
`docs/design/BLUEPRINT-BEBOP-DAG-2026-09-28.md` (DG rows), `docs/research/2026-09-28-bebop-dag.md` (R), `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md`
(phases 1–7, W1–W3), `docs/design/BLUEPRINT-ARCHITECTURE-EVOLUTION-2026-09-22.md` (P1–P9), `docs/measurements/COSTS-NOW-AND-BEBOP-2026-09-26.md`,
`docs/research/2026-09-27-binary-size.md`, `docs/research/2026-09-27-performance-paths.md`.

Every number is tagged **MEASURED** (command + date), **CITED** (URL + date read) or **ESTIMATE** (basis named). A parallel lane
W-TELEM builds the lab profiler (DW6); §10 names the measurements wanted from it rather than duplicating them. The operator's seven
outside-chat inputs (I1–I7) are judged inside the section they belong to and summarised in §8.4.

The lane's own measurements: six read-only GETs on `qa-durres.dowiz.org` (2026-10-02 ~08:00 UTC) and one single-file microbenchmark
through `bebop-lang/tools/slot.sh bneck` (`scratchpad/bneck/copybench.rs`, `rustc 1.93.1 -O`, A78 cores 4–6, two runs; its first run
crashed on my own `black_box` and was rerun — said here because a crash that is not reported is a number that is not trusted).

---

## 0. Одна сторінка для оператора (UA)

**Що виміряно.** За добу (cf.mjs, 2026-10-01): 1 250 запитів до Worker, 18 386 до об'єктів (14,7 на один), CPU p50 6,8 мс / p99 84 мс;
за 7 днів середнє 8,32 мс на запит. Один холодний візит у вітрину = ≈ 22 запити до Worker. Каталог — образ 538 KB, який 37 місць у коді
досі читають цілком і парсять як JSON за 6,33 мс; колонковий блок DG7 (7,9 KB) декодується за 8 мкс у дереві (2,4 мкс у дослідному зонді).
Копія того ж образу байт-у-байт коштує 70 мкс (виміряно сьогодні) — тобто дорогий не РУХ байтів, а ЇХНІЙ ФОРМАТ. Worker-wasm 4,75 MB
(1,84 MB gzip) — 92 мс compile+instantiate на A78, лінійно від розміру.

**Де 1000× є фізично, а де немає.**
- *Запити й CPU на ЧИТАННЯ вітрини:* 1000× є — але не через швидший код, а через те, що запиту не стає: меню/i18n/фото публікуються
  об'єктом у R2 (`cdn.dowiz.org`) як незмінні блоки з хеш-іменами + крихітний «корінь» (manifest); повторний візит читає IndexedDB і
  не робить жодного запиту. 22 → 0–1 запитів на візит; CPU Worker-а на читання — 0. Це ряд A1/C3 карти витрат, і це чесна половина ідеї I7 (Merkle-блоки) БЕЗ P2P.
- *CPU на запитах, які залишаються* (оформлення, логін, консоль): 10–30×, не 1000×. Підлога — HMAC + один стрибок до об'єкта + байти наскрізь
  ≈ 0,3–1 мс; сьогодні 2,7–8 мс. Решту з'їдає фізика: один стрибок Worker↔об'єкт ніякий компілятор не прибере (I4 — хибно як мережева оптимізація).
- *Запити на polling:* 15 000 → ≈ 1 800 на зайнятий заклад-день (A2–A4 карти витрат, сокети вже hibernatable) → ≈ 300 із IndexedDB і дельтами
  «з покоління N» (вікно `recent` в об'єкті вже є). 50×, далі — лише фізика присутності клієнта.
- *Байти:* каталог 538 KB → 7,9 KB блок (68×) на хоп; замовлення 6 JSON-конвертів → ≈ 120 B (W1 blueprint-а 09-20, ≈ 200×). Реально 100–200×.
- *Пам'ять:* JSON-дерево serde_json::Value на 538 KB ≈ 3–5 MB алокацій на запит (оцінка) → блок-view = 0 алокацій. Це 1000× на цьому шляху, але шлях і так зникає.
- *Затримка користувача:* RTT 50–150 мс — стеля. Холодний візит сьогодні ≈ 5–8 послідовних RTT; підлога — 1 RTT (shell із SW-кешу + manifest); повторний — 0 RTT. 5–10×, і більше ніяк.

**Порядок (після DAG і вже затвердженої черги), 3 лейни:** (1) BN1 блок замість образу на 38 місцях — кінець 1102 у пік (2 дні, Haiku×2);
(2) BN2 публікація в R2 з хеш-іменами + manifest-корінь (4 дні, Opus) — єдиний захист «ліміт вичерпано — вітрина жива»; (3) BN3 читач блоків
у браузері + IndexedDB по k64 (3 дні, Opus) — повторний візит 0 запитів; (4) BN4 = P1 команда в об'єкті (одна черга = одне замовлення, 3 → 1 запит до об'єкта; 4 дні, Opus);
(5) BN5 дієта wasm (icu/idna/url/chrono геть, argon2 в об'єкт; 2 дні, Haiku); (6) BN6 W1 «кошик як індекси» (≈ 200× байт/рядків на замовлення; 5 днів, Opus);
(7) BN7 дрібниці: `Kv::get` лінійний (345 → 150 нс), `Catalog::load` без копії (1 день, Haiku). Відмовлено з числами: P2P для клієнтів (I7),
предиктивний push на Free (I6), «compile-time fusion» як мережева оптимізація (I4), щільний SIMD (DG25 — 8× ПОВІЛЬНІШЕ), wavelet-дерева на 165–10 000 страв (I5),
bebop-ядра у Worker (2,6–10,6× повільніше за Rust).

**Вердикт I1–I7 одним рядком кожен:** I1 — правила (Datalog) так, байткод ні; I2 — так, це DG7 (копія 70 мкс — не проблема, парсер — проблема); I3 — «−99 %»
завищено: −90–95 % на зайнятий заклад, і лише з BN2+BN3; I4 — хибно про мережу, правда лише всередині одного ізоляту (і це вже зроблено на `/fold/*`);
I5 — дані замалі (7,9 KB), ні; I6 — ні на Free (push = рахований запит); I7 — хибно для клієнтів, half-true для блоків без P2P, окремий кейс для персоналу лише з локальним вузлом.

---

## 1. Bottleneck 1 — state and compute in different places (Worker ↔ Durable Object hop; MEASURED 14.7 DO requests per Worker request)

### 1.1 Physics: what a hop costs and why

A Durable Object is one single-threaded isolate at one place ("instantiated in a data center close to where the initial `get()` request is
made … do not currently change locations after they are created" — CITED, developers.cloudflare.com/durable-objects/reference/data-location,
read 2026-10-02, page dated Jun 26 2026). The Worker that fronts it runs at whichever edge the customer hit. Every `stub.fetch()` therefore pays:
(a) a serialisation of the request (structured clone / HTTP framing), (b) a network round trip inside Cloudflare's backbone — intra-metro
≈ 1 ms, Vienna↔Warsaw-class ≈ 10–25 ms (ESTIMATE, speed of light in fibre ≈ 200 km/ms over ~800 km plus switching; the two `cf-ray`
colos the box hit today were VIE and WAW, MEASURED in the probe headers), (c) the object's own turn, and (d) the response copy back into
the Worker's linear memory. The DO request itself is also the first wall on Free: 100,000/day, account-wide (CITED, DO pricing Sep 30 2026).

Why 14.7 and not 1–2 (the 09-26 model): the measured 24 h had 1,506 alarms (per-object timers, FT2), hub reads that do `Place::of_slug`
(registry) + the venue object (r ≈ 2 per traffic request, cost map §1.3), sockets (each ping that reaches the object is a request until A3's
auto-response), and the nightly's fan-out. The ratio is a traffic-mix fact, not a code constant: it moves with every row below.

### 1.2 Every technique (industry, research, other runtimes) — incl. I4 "compile-time fusion"

| Technique | Where it comes from | What it is here | Verdict |
|---|---|---|---|
| **Route the request straight to the object** (actor co-location; Orleans "grain-directed calls", Erlang `gen_server` as the server) | actor systems | A DO cannot be a route target: the docs' model is Worker → stub → object; there is no "object as origin". What CAN be done: the Worker becomes a 1-line proxy (auth + `stub.fetch(req)`), and every decision moves into the object — the P1 command surface (`POST /fold/command`, arch-evo §3) | **the real row**: 1 Worker + 1 object request per action is the floor; today placement = 3 object calls + 3 rows (cost map A7, one-image gate) |
| **RPC service bindings / JS RPC to the object** | Workers RPC | Removes HTTP framing, not the hop: "Smart Placement is currently ignored when making RPC calls" is the only locality statement (CITED, developers.cloudflare.com/workers/runtime-apis/rpc, read 2026-10-02); an RPC call to a DO is still a DO request for billing (ESTIMATE: nothing on the page says otherwise, and the pricing page counts "requests" to the object) | 1.2–2× on the serialisation share of a tiny call (ESTIMATE); 0× on the count. Not a lever on Free |
| **Collapse r (registry + venue) to 1** | caching | A4: 60 s Cache-API/memory of `Place::of_slug` | halves object requests per traffic request (cost map A4) — already a row |
| **Alarms consolidated** | timer wheels | 1,506 alarms/day (MEASURED) = 8 % of the object line; one alarm per venue per minute-of-need, not per timer | ESTIMATE −60 % of alarms; S |
| **Pings never reach the object** | hibernation API | `setWebSocketAutoResponse(ping→pong)`: "Application level auto-response messages … will not be charged" (CITED, DO pricing Sep 30 2026) | A3 — already a row; −1,728 object req/day per socket |
| **Serve reads from the edge, not the object** | CDN | the object WRITES R2 at generation change; readers never touch it (A1/C3) | the only way to 0 for browsing — §3 and §6 |
| **Location hint `eeur`/`weur`** | DO data location | hub objects for Albania/Ukraine created from the box's VIE/WAW colos already; `locationHint` is "best effort" (CITED) | no gain; one line of insurance when a venue is created from abroad |
| **I4 "a Rust macro compiles the 38 interactions into one function"** | query fusion / stream fusion (Haskell `foldr/build`, DBSP operator fusion, LINQ) | fusion removes **intermediate materialisations inside one address space**. The Worker and the object are two isolates in two machines; no compiler can remove a network boundary — it can only MOVE work across it (which is P1), or batch N calls into one (which is the command surface again). Inside ONE isolate, fusion is real and already done: `fold::menu::Memo` folds catalogue+i18n+settings once per generation and `/fold/menu` answers bytes (`workers/api/src/hubdo/menu.rs:43-61`, a hit is "three map lookups", MEASURED by reading) | **false as a network optimisation; true inside the object**, where it is spent. What remains of I4 is a typed command enum (P1) — one call carrying the whole intent |
| **"Snapshot + delta" per client** (Quake 3 netcode, hub-cost §3) | games | `recent: RefCell<Vec<Change>>` + `since=generation` already exist (`hubdo.rs:102-140`, MEASURED by reading) | the push side is §2 |

### 1.3 Numbers here

- MEASURED (cf.mjs, 24 h to 2026-10-01): Worker 1,250 / DO 18,386 (1,506 alarms) / CPU p50 6.8 ms, p99 84 ms. MEASURED (COSTS-NOW §1.2,
  7 days to 09-25): mean 8.32 ms/request; DO bytes per response fell 575 KB → 5.5 KB once the object folded its own log.
- Hop latency: ESTIMATE 1–25 ms per `stub.fetch` depending on colo pairing; placement today = 3 object calls in series (one-image gate) → 3–75 ms of
  pure hops before any work. P1 makes it one.
- Floor per remaining Worker request after A4+A5+P1: HMAC verify (≈ 10 µs) + 1 object call + bytes pass-through ≈ **0.3–1 ms CPU** (MEASURED light hours
  09-24: p50 1.0–1.7 ms; the pass-through handler in `storefront.rs:196-216` parses nothing). Today's p50 2.7 ms → ≈ 1 ms: **3×**; today's mean 8.32 ms
  (dominated by the catalogue-parsing sites and argon2) → ≈ 1 ms: **≈ 8×**. Not 1000× — on the requests that remain, the hop IS the cost.
- Object requests per Worker request: 14.7 → ESTIMATE ≈ 1.2 after A3 (pings), A4 (registry), alarm consolidation, P1 (3 → 1 per placement). **≈ 12×** on the first wall.

### 1.4 Cost, risk, composition

P1 is L (arch-evo: `hubdo.rs` split first; ~400 lines) — it is BN4 in §9. A3/A4 are S and already queued. Risk: the object serialises a venue's commands
(soft limit 1,000 req/s per object, CITED DO limits Jun 1 2026 — a venue doing 30 orders/day is 5 orders of magnitude under it). Composes with everything:
a command that enters the object once is the unit §2 pushes, §3 reads as a block, §4 folds incrementally.

---

## 2. Bottleneck 2 — pull instead of push (polling 12–20 s; cost ∝ clients × time)

### 2.1 Physics

A poll costs one counted Worker request + r object requests whether or not anything changed; cost = Σ_clients (session seconds / interval). A push costs
one message per CHANGE per interested client — and on Durable Objects the outgoing direction is free: "There is no charge for outgoing WebSocket messages,
nor for incoming WebSocket protocol pings"; incoming application messages are counted at "a 20:1 ratio" (CITED, DO pricing Sep 30 2026). A hibernated
socket costs no duration ("Billable Duration (GB-s) charges do not accrue during hibernation" — CITED, DO websockets best practices, read 2026-10-02).
So the physics on this platform is asymmetric in our favour: **state flows down for free; only what goes up is counted.**

### 2.2 Every technique — incl. I3 "client as a Datalog node" and I6 "predictive streaming"

| Technique | What it is here | Verdict |
|---|---|---|
| Hibernatable WebSocket per surface, polls only when `due()` (A2/A6) | sockets exist (`accept_websocket_with_tags`, `hubdo.rs:533`); the surfaces still poll at 12–15 s (`track.js:38`, `admin/core.js:28`) | **the biggest ratio per lane-day** (cost map: 15,000 → ≈ 4,000/busy hub-day) |
| SSE | one counted request held open through the Worker; no hibernation; duration billed | AGAINST on DO (the socket is strictly better here) |
| Delta since generation (`since=N`, Quake-3 snapshot delta) | `recent` window exists (`hubdo.rs:102-140`) | the message body of every push; already built |
| Single-writer streams, no CRDT (arch-evo P6: CRDTs AGAINST) | the object is the only writer; clients fold a totally ordered log | keeps the client fold trivial — the precondition for I3 |
| Server push of the storefront ROOT (manifest version) | a customer with the storefront open learns a price change via the same socket as tracking | only after A1/C3; 1 message per generation per open storefront |
| Web Push (VAPID) for the courier/console when the tab is closed | free, already a PWA | replaces the "keep polling in the background" case |
| **I3 — client as a full Datalog node**: DO = authoritative log + intent validator; browser runs folds/rules from snapshot + delta over the socket | `crates/bebop-wasm` already exposes `kv_view`/`log_view` (browser readers); DW1 "browser replica as a graph subscriber" is the DAG row; the rules exist in bebop (`dl*.bp`, DG8) but "relations are in memory, not store projections" (commit `8300b51b`) | **direction right, "−99 %" overstated** — see the computation in §2.3 |
| **I6 — predictive streaming** (edge Markov model pushes the next screen before the tap) | On Free every push that is an HTTP fetch is a counted request, and a speculative one is wasted 1−p of the time. Over an open socket, pushes are free — but the next screen's DATA for a storefront is the catalogue, which after A1/B is already on the device. What is left to predict is nothing | **AGAINST on Free**; the useful 10 % of it is `<link rel=prefetch>` of static/R2 assets (free, uncounted) and a service-worker warm of the next fragment |
| Request coalescing (one object turn answers many waiters) | the memo already does this inside the object; at the edge, `cache_put` 30 s on the menu (`storefront.rs:192-225`) | done where it matters |

### 2.3 Numbers here

- MEASURED (hub-cost blueprint §2, 09-20): polling = 65 % of all requests. MEASURED today: `track.js` 12 s, console 15 s, courier 12 s, `live.js` PING 25 s.
- A busy hub-day (profile B): 15,000 Worker requests → ≈ 4,000 with socket-first (cost map A2) → ≈ 1,800 with A3/A4 (cost map "capacity after A1–A5").
- **I3 computed from the cf.mjs numbers.** Take the busy profile after A1–A5 (≈ 1,800 Worker / 1,800 object requests per hub-day). What a full client-side
  Datalog node removes further: console/courier polls when `due()` (they become socket deltas: ≈ 1,200 of the 1,800 → 0 counted, since outgoing is free),
  tracking (≈ 27 polls per order × 30 orders ≈ 800 → 0). What it cannot remove: placements (30), courier actions (≈ 150 incoming socket messages ÷ 20 ≈ 8
  request-equivalents), logins/identity (≈ 50), socket upgrades (≈ 10 per device-day × 5 devices = 50), the object's own alarms (≈ 50 after consolidation).
  Result ≈ **150–300 counted requests per busy hub-day against today's ≈ 15,000 + 30,000 object requests: −98 to −99 % of the counted lines** — but **only
  on top of A1–A5 and P1**; against the cost map's post-A5 state (≈ 1,800) the Datalog node itself buys **−85 to −90 %**, i.e. ≈ 8×, not 100×. The
  "−99 % server load" is the compound of five rows, four of which are already approved; the client node is the last ≈ 8×. Honest number: **the socket
  carries it; the Datalog engine in the browser is a correctness convenience (same rules both sides), not the saving.**
- What the browser must hold for I3 (ESTIMATE from the fixture): the catalogue blocks 2.9–7.9 KB each (MEASURED, `crates/dowiz-hub/fixtures/blocks/*.dwb`:
  `menu_prices` 7,884 B, `names` 6,732 B, `bom` 7,764 B, `stock_levels` 2,956 B), the venue's live orders (≈ 30 × 120 B after W1, ≈ 30 × 2 KB today), the rule
  set (`dl_sushi.bp` is GENERATED from the menu — a few KB). Well under IndexedDB's budget (cost map §3.4). The client runtime: `bebop-wasm` plus a block
  reader (DG9's `block.rs` twin) — ESTIMATE 60–150 KB raw on the Leptos-measured slope (cost map §5.2: 22 ms/MB compile on A78; sub-millisecond here).

### 2.4 Cost, risk, composition

A2/A3/A6 are S (JS + one call in `accept()`); already queued. DW1 (browser replica as subscriber) is M–L; it is BN3 in §9 and it composes with §3 (what it
subscribes to is blocks by `k64`) and §6 (0 requests on a return visit). Risk: sockets through mobile carriers die silently (live.js's DEAD_MS exists for
this) — the 30 s fallback poll stays, and that fallback is the floor on the request count: ≈ 2 polls per minute per surface that lost its socket.

---

## 3. Bottleneck 3 — reading everything to use a little (538 KB catalogue parsed as JSON, 6.33 ms; 2,355 `json!` sites)

### 3.1 Physics

Three separable costs hide in "read the catalogue": (1) **moving** 538 KB across the hop and into wasm linear memory; (2) **parsing** it into a
`serde_json::Value` tree — one allocation per node, UTF-8 validation of every string, number parsing; (3) **touching** the one field the request needs.
MEASURED today (`scratchpad/bneck/copybench.rs`, `rustc -O`, A78, 2026-10-02, two runs): copying 538,000 B with `Store::from_bytes`'s 8-byte loop
(`crates/bebop-store/src/lib.rs:348-355`) = **69.4–71.9 µs**; a flat `memcpy` = **43.8–44.6 µs**; a byte scan = 99–103 µs. MEASURED (R §13.2, 09-28):
JSON parse to `Value` of the same catalogue = **6.33 ms** (16.5 ms at 2.5×, 146.7 ms at 25×); DG7 block decode = 2.4 µs in the research probe, **8,066 ns
in the tree's release build** (commit `c496c2d9`, with the 4-lane crc32); `bom_of` 634 ns from the block vs 3,775 ns from JSON (same commit).
So: the copy is 1 % of the parse. **The bottleneck is the FORMAT, not the movement** — which is exactly what I2 claims and what DG7 already built.

The memory side (ESTIMATE, serde_json's `Value` is 32 B per node plus heap for strings/vecs; a 538 KB document with ~165 products × ~40 fields × nested arrays
≈ 50–100k nodes): **≈ 3–5 MB of allocation per parse**, freed at the end of the request. In a 128 MB isolate that is not a wall, but it is 1 % of the isolate per
request and the allocator's time is inside the 6.33 ms.

### 3.2 Every technique — incl. I2 "database-as-heap / zero-copy" and I5 "succinct structures"

| Technique | What it is here | Verdict |
|---|---|---|
| **Columnar block = wire = memory format** (Arrow/FlatBuffers/Cap'n Proto idea; rkyv's "a pointer offset and a cast" — CITED rkyv.org/zero-copy-deserialization, read 2026-10-02) | DG7 `crates/dowiz-hub/src/block/{mod,schema,encode,decode,view}.rs`: "a reader views a field with one bounds check and one `from_le_bytes`" (module header); `/fold/menu?block=`, `/fold/products?block=` serve it (`hubdo/menu.rs:66-71`) | **built**; what is NOT done is making the 37 `load_catalog(` sites read it (dataflow baseline 38) — BN1 |
| **I2 — storage bit-identical to the wasm structs, read by offset, zero allocation** | The real limit the operator names is right: a DO `storage.get()` value or an R2/KV body is copied into linear memory at least once (no mmap in Workers; the host hands bytes across the JS↔wasm boundary — `serde_wasm_bindgen`/`Uint8Array` copy). Quantified: that copy is **44–72 µs per 538 KB (MEASURED above), 90× under the 6.33 ms parse**. A `View` over `&[u8]` (DG5's borrowing twin, `proj.rs:185`) removes the second copy (`Store::from_bytes` → `Vec<i64>`); rkyv-style in-place structs would remove a third (the block's `decode` = "validate lengths + copy out", R §13.2) — the block's `view.rs` already reads in place for `bom_of`/prices | **true, and 95 % of it is DG7 + DG5's `View`**. Own format over rkyv/FlatBuffers: keep the own format — it is the CAS unit and the four-reader twins exist (DG9); rkyv's `bytecheck` validation is the same cost shape as DG7's `check()`. Alignment: a DO body arrives at an arbitrary offset; the block pads columns to 8 (header) and reads `from_le_bytes`, which is alignment-free — correct choice for wasm |
| **Memo per generation in the object** | `fold::menu::Memo`, hit = 3 map lookups, no storage call (`hubdo/menu.rs:43-61`) | built (the 9fa24c24 row: 15–20 ms → ~70 µs per read) |
| **Edge cache 30 s on the public menu** | `storefront.rs:177-225`; MEASURED today: cold MISS TTFB 1.58 s, warm HIT TTFB 0.28 s (`age: 27`) | built; a HIT is still a counted Worker request ("Workers run before the cache", cost map) |
| **Interning + arena** (string tables; bump allocation per request) | the block's `names` column IS an interned table; a request-scoped bump allocator (`bumpalo`) for the remaining JSON paths | ESTIMATE 1.3–2× on allocation-heavy handlers; irrelevant once the parse is gone |
| **Bloom/xor filters for negative lookups** | "is this slug / this product id present" against a 165-entry sorted array | the array is 165 long: a binary search is 8 compares. Not a lever below ~10⁵ entries |
| **I5 — succinct structures (wavelet trees, roaring bitmaps, popcount/SIMD over compressed bits)** | The catalogue is **7,884 B** as a block (165 dishes) and ≈ 0.5 MB at 10k dishes (ESTIMATE, linear). Roaring bitmaps pay off on sets of 10⁵–10⁹ integers; a wavelet tree on rank/select over large sequences. Our hot queries are "price of id", "bom of id", "available(id)" — direct indexing on a 165-row column, ≈ 1 ns each. DG25's dense-SIMD measurement is the warning: dense mask simd128 **9,749 ns vs sparse scalar 1,089 ns — 8× SLOWER** at 1×, 580× at 100× (MEASURED R §12.3; DG25/26 WITHDRAWN). wasm SIMD inside a real Worker is UNMEASURED (DG20 withdrawn with DG25) — said plainly: no number exists | **the data is too small to matter**; refused with the DG25 number. Re-open only if a venue crosses ~10⁴ dishes AND a scan shows up in W-TELEM's per-node cost |
| **Zstd dictionary per venue on the wire** | brotli at the edge already gives 779 B for a 2,884 B QA menu (MEASURED today) and ≈ 12 KB for the 165-dish fragment (cost map §4.3); a venue dictionary could take another 20–40 % off small bodies (ESTIMATE, dictionary compression's usual band on <4 KB payloads) | not worth a format: the bodies that matter are already content-addressed and cached |
| **2,355 `json!` response sites** | `Response::from_json` 252 sites, `.text().await` 43 (MEASURED grep 10-01). Each is a tree build + a string copy across JS↔wasm | the typed-responses row (ts-rs, queue 10-01) is the way out; CPU share ESTIMATE 0.1–0.5 ms per response — real but second-order |

### 3.3 Numbers here

- Per catalogue read on a Worker path: 538 KB hop + 6.33 ms parse → 7.9 KB hop + 8 µs decode (tree) = **68× bytes, ≈ 780× CPU** on that path. On a placement:
  `bom_of` 3,775 ns → 634 ns (6×); the whole placement's catalogue work 41 µs → <1 µs (R §13.2).
- MEASURED today: `Kv::get` is a LINEAR scan (`crates/bebop-store/src/kv.rs:246`: `self.entries.iter().find(...)`) over entries the same file keeps SORTED
  (`set`/`remove` use `binary_search_by`, `:257,:267`). Cost: 345 ns linear vs 150 ns binary per lookup at 165 keys (both include ≈ 100 ns of `format!`). A real
  omission, a negligible number today; at 10k keys it is 20 µs vs 200 ns per lookup — the one place in §3 where size WOULD bite. BN7.
- The "31 ms `from_bytes` copy" of R §4 / the DAG blueprint's "Do NOT" row was a 2.9 MB LOG image and (by its ratio to the 5.75 ms fold) most likely a wasm/node
  measurement; the 538 KB catalogue copies in 70 µs native. The row's conclusion stands (borrow, do not copy); its number should not be reused for the catalogue.

### 3.4 Cost, risk, composition

BN1 (38 sites → block/`/fold/*`) is M×2 by W-DF's pace (3 sites per lane); Haiku-able because the block API exists. Risk: a site that needs a field the four
blocks do not carry (the JSON stays the writers' format — add a column, re-derive the twins in the same commit, the "written format" law). Composes: the block
is what §6 caches on the device and what §2 pushes by `k64`.

---

## 4. Bottleneck 4 — recompute instead of incremental (whole-history folds; Datalog 970 ns/event vs 10 µs interpreted)

### 4.1 Physics

A fold over N events is O(N) per read; an incremental view is O(Δ) per change plus O(1) per read. The theory is settled: DBSP ("DBSP: Automatic Incremental
View Maintenance for Rich Query Languages", Budiu, McSherry, Ryzhyk, Tannen, arXiv 2203.16684, CITED read 2026-10-02) gives a general incrementalisation for
relational + recursive queries; differential dataflow is its ancestor. The product's log is tiny — ≈ 5,400 events at the hot-log size (R §4) — so the absolute
numbers are milliseconds, and the question is whether milliseconds matter under a 10 ms wall. They do: the 10 ms kills clustered on exactly the paths that
folded or parsed whole things (COSTS-NOW §0: 622 kills at 10,000 µs).

### 4.2 Every technique — incl. I1 "code-to-data" (rules vs bytecode to where the data lives)

| Technique | What it is here | Verdict |
|---|---|---|
| **Memo per generation in the object** (`folded: Option<Orders>`, `hubdo.rs:212-217`) | reads hit the memo; a write refolds whole | built; refold cost 5.75 ms at 5,400 events (MEASURED R §4) |
| **Store projections + `fold_step`** (DG5, landed `64d2d5cd`) | one append extends the memo: ≤ 0.05 ms vs 5.75 ms (254×; 1,274× at 54k events — MEASURED R §4) | built in `bebop-store`; the dowiz object still refolds whole on write (`folded` is per generation, not stepped) — BN4 adopts `fold_step` |
| **Datalog on the dirty set** (DG8, landed `8300b51b`) | `dl_event_ns` 970 generated / 10,060 interpreted (MEASURED, commit msg); 82 ns Rust native (R §12.3); 40× under naive at 1×, 2,260× at 100× | built in bebop; "relations are in memory, not store projections" — the DG6 pure class closes that |
| **Answers computed at write time** (CQRS read models; materialised views) | `/fold/menu` per generation, `/fold/orders` memo — and C3: HTML fragments + blocks written to R2 at publish | the object already IS the write-time computer; C3 extends it to the edge |
| **I1 — code-to-data: ship rules / rule ids / bytecode / DAG projections to the DO or the browser** | Three carriers, judged separately. (a) **Datalog rules** (declarative, stratified, terminating by construction, E125 refusals exist): safe to ship — a rule set is data, a versioned `K64` of its text is its contract; the engine is the same bebop/Rust twin on both sides; interpreter overhead 970 ns/event is 10⁻⁵ of a request. (b) **Bebop bytecode** to the browser: the compiler exists only on the box (`bebop.bin` is a native Linux binary; no wasm emitter — DG26 withdrawn is about v128, but there is also no wasm32 backend in the tree), so the "client runtime" would be a bebop VM in wasm = a second runtime, refused by operator D-2; and arbitrary code from the server to a customer's device is the XSS surface by another name. (c) **DAG projections** (DG5 PROJ nodes): these are what the browser should receive — a memo keyed by `(key64, kind, input_gen, tip)` with a CRC; shipping the memo IS shipping the answer, and `bebop-wasm`'s `log_view`/`kv_view` can verify it | **rules yes, bytecode no, projections yes**. Interpreter overhead vs data moved: a rule ≈ 100 B vs the relation it would otherwise fetch ≈ 3–8 KB; per event 1 µs. Versioning: rule contracts by content hash + a `min_client` generation in the manifest; a client with an older rule set falls back to the server's `/fold/*` answer (never to a wrong local answer) |
| **Bebop-compiled kernels where they beat Rust** | COSTS-NOW: bebop 2.6–10.6× SLOWER than Rust on compute; DG8: generated 970 ns vs 82 ns Rust native (12×) | nowhere today. Bebop wins on the box as the store/DAG language, not as a Worker kernel |
| **Differential dataflow / DBSP engine in the object** | a full operator graph for 5 rule sets over 165 × 120 rows | over-engineered at this size; the semi-naive dirty-set loop (DG8) is the same asymptotics with ~200 lines |

### 4.3 Numbers here

- Reads: memo hit ≈ µs today (built). Writes: whole refold 5.75 ms → `fold_step` ≤ 0.05 ms: **≈ 100× at the hot-log size**, inside the object, on every placement/advance.
  This is the one place a 100× is already measured and merely unadopted on the dowiz side.
- Rules: `unavailable`/`allergen`/`fsm_ok`/`courier_may`/`personal` at ≈ 1 µs per event (MEASURED, bebop generated) — the cost of a rule layer is below noise at
  any venue size that fits one object.

### 4.4 Cost, risk, composition

Adopting `fold_step` + projections in `hubdo.rs` is part of BN4 (it touches the same write path P1 moves). Risk: the projection's correctness depends on the
anchor/tip rules (`proj.rs` header: "A memo is never trusted past its CRC") — the RED tests exist (`step_equals_full`, `redaction_drops_memo`). Composes with §2
(the stepped memo is the delta that goes down the socket) and §6 (the same memo verified on the device).

---

## 5. Bottleneck 5 — fixed per-start costs (4.7 MB wasm, JS↔wasm copies, Free 10 ms CPU / 128 MB)

### 5.1 Physics

MEASURED: `workers/api/build/index_bg.wasm` 4,747,270 B raw / 1,840,961 B gzip (built Sep 30); compile + instantiate of a 4.3 MB build = 91–94 ms CPU on an A78
under Node 26/V8 14.6 — ≈ 22 ms per MB (binary-size research §7.2; the Leptos bins confirm the slope: 66 KB → 0.93 ms, 138 KB → 1.6 ms, cost map §5.2). Platform
facts (CITED, developers.cloudflare.com/workers/platform/limits, Sep 5 2026): "no compressed size limit. Only the uncompressed bundle size counts" (64 MiB);
"a Worker must parse and execute its global scope … within 1 second"; "each isolate can consume up to 128 MB of memory, including the JavaScript heap and
WebAssembly allocations"; Free 10 ms CPU per request. The runtime page (Apr 23 2026) says only "the larger your Worker is, the longer it may take your Worker to
start" and recommends `wasm-opt`; **whether the compiled machine code is cached across isolates is not stated by Cloudflare's docs** (third-party blogs claim
it is; UNVERIFIED — §10). Whether the 92 ms is charged against a request's 10 ms is also unstated; the measured kills clustered on catalogue paths, not on first
requests, so the working assumption is that compile is outside the per-request budget (ESTIMATE from the kill pattern, COSTS-NOW §1.2).

The per-request fixed costs are the JS↔wasm crossings: request headers/body copied in, response copied out, every `serde_wasm_bindgen` conversion
(`crate::body` + the strict-body gate — memory `worker-deny-unknown-fields-inert`), `Response::from_json` 252 sites. ESTIMATE 50–300 µs per request for a small
JSON body (string copy + UTF-8 + tree) — real, second-order against 2.7 ms p50.

### 5.2 Every technique

| Technique | Here | Verdict |
|---|---|---|
| **Crate diet** — `Cargo.lock` 106 crates incl. `chrono`, 7 × `icu_*`, `idna`, `url`, `tokio`, `wasm-streams`, `strum` (MEASURED grep 10-01); `argon2` in the Worker | `twiggy top` on the build names the bytes (binary-size research §1/§4 did this once: the strip research found `-Oz` worse for gzip) | ESTIMATE 4.7 MB → 2–2.5 MB (icu/idna/url alone are ≈ 1 MB-class in wasm); compile 92 → ≈ 45 ms. **2×**, Haiku-able, BN5 |
| `wasm-opt -O` + `--strip`, `panic=abort`, `lto`, `codegen-units=1` | already set (`Cargo.toml:57-71`; `strip = false` for the names) | done; the remaining lever is what is compiled, not how |
| **Split into several Workers** (storefront reads / owner console / nightly) | each cold-starts a smaller module; service bindings between them | the storefront's reads leave the Worker entirely under A1/C3, which is the same split for free; splitting the rest buys little once the diet is done |
| **Lazy instantiation / dynamic wasm** | a second module for rare paths (export, print, QR) loaded on demand | only if the diet leaves >2 MB; ESTIMATE 20–30 % more |
| **Component model / no-JS-glue** | `workerd` speaks JS at the boundary; no component-model host today | not available; nothing to do |
| **argon2 and the nightly into the object** (A8) | argon2 `Params::new(8*1024, 3, 1)` in the Worker (`auth.rs:250`); the object's CPU limit is 30 s (CITED DO limits) vs the Worker's 10 ms — the OPEN 2 contradiction (FAQ says Workers' limits apply) must be probed first | the right place for the only CPU-heavy pure work; kills from logins end |
| **Bebop in the Worker** | 2.6–10.6× slower than Rust (COSTS-NOW); no wasm backend | no |
| **Arena/bump per request** | `bumpalo`-style for handlers that build `Value` trees | 1.3–2× on those handlers (ESTIMATE); made moot by typed responses |

### 5.3 Numbers here

Startup: 92 ms → ≈ 45 ms (diet) → ≈ 10 ms if the storefront's own Worker is ~0.5 MB (ESTIMATE on the 22 ms/MB slope). Per-request fixed cost: ≈ 0.1–0.3 ms →
≈ 0.05 ms with bytes-through responses. Memory: the parse's 3–5 MB per request → 0 on block paths. **None of this is 1000×; all of it is 2–10×, and the
10 ms wall stops being hit for a different reason (§3).**

### 5.4 Cost, risk, composition

BN5 = 2 lane-days (Haiku; `twiggy` + `cargo tree -e no-dev`, feature flags, the deps-audit gate). Risk: `url`/`idna` are pulled by `worker` itself for `Url`
parsing — some of the bytes cannot leave without replacing `req.url()?` uses; measure with `twiggy` before promising. Independent of the other rows.

---

## 6. Bottleneck 6 — physics of the mobile link (RTT 50–150 ms)

### 6.1 Physics

A mobile RTT to the nearest Cloudflare colo is 30–80 ms on LTE in Tirana/Kyiv and 100–200 ms on a congested cell (ESTIMATE; the box's own connect from
Android/Termux was 37–500 ms today, MEASURED). Nothing on the server shortens it. The only levers are **fewer serial round trips** and **more state on the device**.
MEASURED today on qa-durres: `/` HIT 2,538 B br TTFB 0.29 s; `/app.js` HIT 5,464 B TTFB 0.14 s; the menu cold MISS 1.58 s (origin + object), warm HIT 0.28 s.
A cold storefront visit today is ≈ 22 Worker requests with 3–5 serial stages (shell → app.js graph → menu + i18n → photos) ≈ 5–8 RTT before the menu is visible.

### 6.2 Every technique

| Technique | Here | Verdict |
|---|---|---|
| **Everything the first paint needs in ≤ 2 requests** (shell from the SW cache; one manifest; blocks/fragments by immutable hash) | C3/A1: `cdn.dowiz.org/v/<slug>/manifest.json` (tiny, `max-age=30`) then named blocks `immutable` | 5–8 RTT → 1–2 RTT cold, **0 RTT warm** (manifest check can be async) |
| **IndexedDB replica by `k64`** (cost map §3; FT11) | `replica.js` is `localStorage`; IndexedDB only in `lib/outbox.js` (MEASURED 10-01) | the return visit = 0 requests; BN3 |
| **Service worker: stale-while-revalidate for immutable blocks** | `sw.js` is NETWORK FIRST, ALWAYS ("a stale price is the failure this worker exists to prevent", `sw.js:3-5`) — correct for mutable URLs, wrong for content-addressed ones | with hash names the SW may cache forever; the ROOT stays network-first. Freshness moves to the 30 s manifest |
| **Speculative/optimistic UI** (client-side prediction + reconciliation, hub-cost §3) | the console shows the status change at once and reconciles; the storefront shows "placed" on the socket ack | perceived latency → 0 for the actor's own actions; no server cost |
| **Photos**: small variant on the grid (FT3), content-addressed, `immutable` | 18 × 150 KB → 18 × 20–40 KB (cost map §4.3) = the dominant bytes | 5× bytes per cold visit; on R2 = 0 counted requests |
| **I7 — P2P / WebTorrent swarm** | judged in full in §7.3 | **false for customers**; the content-addressed half is BN2 |
| **Fonts/CSS/JS** static, cached, free | already | leave |

### 6.3 Numbers here

Cold visit: ≈ 22 requests / 5–8 RTT / ≈ 0.6 MB (ESTIMATE from cost map §4.1–4.3) → ≈ 2 counted-free requests + 18 image GETs from the edge / 2 RTT / ≈ 0.5 MB
(photos dominate; the catalogue itself is 8–15 KB). Return visit: ≈ 22 → **0** requests, 0 RTT to first paint, one background manifest GET. On a 100 ms RTT
link that is ≈ 0.8 s → ≈ 0.2 s cold and ≈ 0.03 s warm: **4× cold, ≈ 25× warm** on time-to-menu; **∞ on counted requests**.

### 6.4 Cost, risk, composition

BN2 (R2 publish + manifest + CSP + Playwright) ≈ 4 lane-days (Opus — CSP is the header that broke every page once); BN3 ≈ 3. Risk: a stale price is a product
failure — the root pointer's 30 s window must be stated in the console ("published" only after the manifest write returns), and the price the customer pays is
re-derived by the kernel at placement from the catalogue generation in the order (`storefront.rs:762`'s existing rule; W1 makes it explicit) — the device's
copy is a VIEW, never the price authority.

---

## 7. Bottlenecks main missed

### 7.1 In the hot loops and the layout (small, real)

- `Kv::get` linear over sorted entries (§3.3): 345 → 150 ns; the only O(n)-per-lookup left in the store's read path. 1 line.
- `Catalog::load` = `Store::from_bytes` (copy, 70 µs at 538 KB) + `Kv::load` (which rebuilds every entry as `(String, Vec<u8>)` — ESTIMATE 165 × 2 allocations +
  copies, ≈ 50–100 µs). DG5's `View` makes both borrows. Together ≈ 0.15 ms per load — the parse that follows is 40× bigger, so fix the parse first (BN1), then this (BN7).
- `Store::from_bytes` copies 8 bytes at a time through a `[u8; 8]` (`lib.rs:348-355`): 70 µs vs 44 µs for `memcpy` + reinterpretation. 1.6× on a cost that should not exist.
- The `recent` window and `positions` are `RefCell<HashMap<String, …>>` keyed by `String` — fine at venue scale; noted so no one "optimises" them.

### 7.2 Outside the request path

- **The dev loop**: a cold Leptos build is 4 m 06 s on the box; the Worker's own build + the 1.8 MB upload on a 11–16 KB/s uplink (memory `deploy-upload-fetch-failed`)
  is the slowest loop in the project. CI/Actions (R3) is the fix and is approved; the diet (BN5) halves the upload.
- **The order payload**: six JSON envelopes per order with names copied from the catalogue (hub-cost §2) — W1 "cart as indices" ≈ 120 B per delivered order
  (ESTIMATE in that blueprint, "two hundred times smaller"). Rows written are the third wall (100,000/day) — the one wall none of §1–§6 moves; W1 and P1 move it. BN6.
- **Client rendering**: 165 `innerHTML` card builds on a low-end phone before first paint (cost map §4.3) — C3's prerendered fragment removes it; measured frame time
  is U11 (unmeasured on this box: no DOM harness).
- **The ratio itself as an instrument**: `cf.do_requests_day / cf.worker_requests_day` is not printed by `cf.mjs` (indicators MEASURED by grep: `cf.cpu_p`,
  `cf.cron_runs_day`, `cf.do_alarms_day`, `cf.do_errors_day`, `cf.do_requests_day`, `cf.do_response_bytes_day`, `cf.worker_errors_day`, `cf.worker_requests_day`);
  the number that ranks §1 should be a row with a baseline — W-TELEM.

### 7.3 I7 — P2P / WebTorrent swarm / local-first, judged for THIS product with numbers

**Setting the scene (ESTIMATE from the cf.mjs/qa profiles).** Busy profile B: 100 cold visits and 30 orders per hub-day; a dinner peak of 2 h carries ≈ 40 % of visits
→ 40 visits in 120 min; a menu visit lasts ≈ 3 min → **mean concurrency on one venue's menu ≈ 1 customer (range 0–3)**. A swarm needs ≥ 2 peers that both hold the piece;
at concurrency 1 the swarm is cold essentially always, and the "seed" is… the server. WebTorrent in a browser "can only download torrents that are seeded by a
WebRTC-capable torrent client" (CITED, webtorrent.io/faq, read 2026-10-02) — so a hybrid seed must run somewhere 24 h: that is a server again.

**What a customer actually needs per visit.** Not the 538 KB image: the menu as block/fragment ≈ 8–15 KB brotli (MEASURED block sizes 2.9–7.9 KB; cost map §4.3 fragment
≈ 12 KB gzip), plus photos ≈ 0.5 MB in small variants. The 538 KB number is the OBJECT's image, which nobody outside the object should ever fetch.

| Claim | Number | Verdict |
|---|---|---|
| "egress = $0 at any peak" | Workers: "There are no additional charges for data transfer (egress) or throughput (bandwidth)" (CITED, Workers pricing Aug 28 2026); R2: "Egress (data transfer to Internet) … Free" (CITED, R2 pricing, read 2026-10-02); static assets "free and unlimited" | **true, and already true without P2P** — egress is not a cost line on this platform. What costs is COUNTED REQUESTS, and a swarm adds them (next row) |
| signalling is "free" | every WebSocket message a peer sends to the tracker DO is counted at 20:1 (CITED, DO pricing); an ICE exchange is ≈ 4–20 candidate/SDP messages per peer pair (ESTIMATE, trickle ICE on mobile) → ≈ 0.2–1 request-equivalent per PEER, plus 1 counted request for the socket upgrade — **the same order as the single edge GET the swarm was meant to replace**, which on R2/static costs 0 | **false**: the swarm's signalling costs more counted requests than fetching the piece from the edge |
| "the more users the faster" | needs direct connectivity: ≈ 85 % of pairs connect with STUN alone, 10–25 % need TURN (CITED bloggeek.me/webrtcglossary/turn and expressturn.com, read 2026-10-02); symmetric NAT on 4G/5G carriers and CGNAT make mobile the worst case — and both venues' customers are on mobile. TURN relays are a server we would pay for | **overstated**: at concurrency ≈ 1 there is no "more users"; when there are, 1 in 5 pairs needs a relay |
| "survives a CF region outage" | the storefront shell, the signalling DO and the seed are on Cloudflare; a peer cannot discover another peer without the tracker | **false**: it survives exactly as well as the signalling path does. What DOES survive an outage is a device that holds the blocks in IndexedDB (BN3) |
| latency | ICE median ≈ 200 ms + DTLS ≈ 200 ms (CITED, fippo/Chrome stats via search, read 2026-10-02) before the first byte, after the signalling round trips; MEASURED today one edge GET: 140–290 ms total on a HIT | **worse**: ≈ 0.5–1 s to first piece vs 0.15–0.3 s to the whole menu |
| battery / data | WebRTC keeps the radio in high-power state and uploads pieces to strangers; the customer pays upload on plans at $0.27/GB (Ukraine) to $2.83/GB (Albania) (CITED cable.co.uk via ispreview, 2023/2020 figures) | a customer uploading a venue's photos for the venue's benefit, on their own plan — **not acceptable** for a storefront |
| privacy | peers learn each other's IPs (ICE candidates); IP addresses are "online identifiers" (CITED, GDPR Recital 30, gdpr-info.eu) → personal data exchanged between customers, needing a basis and a notice | **a GDPR surface created for no gain** |
| integrity vs freshness | hashes stop tampering; they do not make a stale price current — a peer can serve yesterday's block forever | the root pointer must still come from the authority every 30 s, i.e. the same manifest GET as BN2 |
| **Merkle / content-addressed blocks WITHOUT P2P** | immutable `k64`-named blocks cached forever at the edge and in IndexedDB + a tiny root manifest → only changed blocks are fetched; this is A1/C3 with hash names, which the cost map already specifies ("immutable content-addressed names") | **take it** — BN2/BN3. It gives the real wins the swarm promised (zero counted reads, offline return visits) with none of the costs |
| staff devices on one LAN (kitchen tablet, waiter phones, courier) | a better case: concurrency is 3–6 devices for 10 h, same network, and offline operation when the venue's uplink drops is a real need. But browsers cannot discover LAN peers without a signalling server (no mDNS/WebRTC local discovery in web APIs), so once the uplink is down no NEW pair can form; pairs formed before stay up. The honest shape is W2 (hub-cost §4): ONE local node (the kitchen tablet as a native/PWA-with-local-server, or the Box of DG24) that the others talk to over the LAN, with the DO as mirror | **a different product row** (offline venue), not a P2P row; defer until the "Box"/native question is decided; nothing in §9 |

**Verdict on I7:** false here for customers on every claim except egress (already free); half-true as "content-addressed blocks" — that half is §9's BN2/BN3;
a separate, later question for staff-LAN offline operation (W2).

---

## 8. Composition — which techniques multiply, the realistic end-to-end factor per metric, and where 1000× is and is not physically possible

### 8.1 What multiplies and what merely adds

Multiplicative chains (each factor is on a different term): **(remove the request) × (remove the hop) × (remove the parse) × (remove the refold)**. A1/C3 removes
the read request (×∞ on that term); P1 removes two of three hops on a write (×3); the block removes the parse (×780 CPU on that path); `fold_step` removes the refold
(×100 on writes). Additive, not multiplicative: the wasm diet (a per-isolate constant), typed responses (per-response constant), `Kv::get` (per-lookup constant).
Things that compose NEGATIVELY: a bebop VM in the browser (second runtime) with a block reader; dense SIMD with a 7.9 KB block; a swarm with a 30 s freshness root.

### 8.2 The realistic end-to-end factor per metric (ESTIMATE on the MEASURED bases above)

| Metric | Today (MEASURED) | After the approved rows (A1–A5, R-rows) | After §9 BN1–BN7 | Factor | Where 1000× is / is not |
|---|---|---|---|---|---|
| **Counted requests per cold storefront visit** | ≈ 22 | 0–1 | 0–1 (return visit 0) | **22× → ∞** | **IS possible** — by the request not existing (edge/R2/device), not by code |
| **Counted requests per busy hub-day** (Worker + object) | ≈ 15,000 + 30,000 | ≈ 1,800 + 1,800 | ≈ 300 + 400 (sockets carry state; P1 one turn; alarms consolidated) | **≈ 60×** | NOT 1000×: placements, logins, courier actions and socket upgrades are counted and are the product |
| **Worker CPU per request that remains** | p50 2.7 ms, mean 8.32 ms | p50 ≈ 1 ms (pass-through) | ≈ 0.3–0.5 ms (typed bytes-out, no parse, HMAC + 1 hop) | **≈ 5–25×** | NOT 1000×: the hop and the HMAC are the floor |
| **Worker CPU per customer READ** | 2–10 ms (menu MISS path) | 0 (R2/static) | 0 | **∞** | IS possible — same reason as row 1 |
| **Bytes across the hop per catalogue read** | 538 KB | 7.9 KB block / ≈ 70 KB JSON pass-through | 7.9 KB, or 0 (device) | **68× → ∞** | 1000× only by zero |
| **Bytes + rows per order** | 6 envelopes, ≈ 2 KB, 3 rows | 3 rows (P1: 1 turn, 1 row) | ≈ 120 B, 1 row (W1 + P1) | **≈ 17× bytes, 3× rows** | 1000× impossible: an order must be written once |
| **Memory per request on catalogue paths** | ≈ 3–5 MB parse tree (ESTIMATE) | ≈ 8 KB block view | 0 (not on the Worker) | **≈ 500× → ∞** | the one place a 1000× is in the code, and the path disappears anyway |
| **Time-to-menu on a 100 ms RTT link** | ≈ 0.8 s cold / ≈ 0.4 s warm | ≈ 0.25 s / ≈ 0.05 s | same; 0 RTT warm | **≈ 3× cold, ≈ 10–25× warm** | NOT 1000×: 1 RTT is the floor for anything new; 0 for anything already held |
| **Wasm startup** | 92 ms per isolate | 92 ms | ≈ 45 ms (diet); ≈ 10 ms if the read Worker is split out | **2–9×** | not a 1000× term; not a per-request term either |
| **Capacity on one Free account** | ≈ 3 busy hubs (DO line first) | ≈ 50 | ≈ 200–300 (bound by rows written and placements) | **≈ 100×** | the operator's "1000× more efficient" lands here as ≈ 100× venues per account, with browsing never failing closed |

### 8.3 Where 1000× is physically possible — stated plainly

- **Yes:** anything that becomes "not a request" (reads from static/R2/device), "not a parse" (block view), "not a refold" (`fold_step`), "not a hop" (compute in
  the object). These are each 100–∞ on their own term and they compose.
- **No:** the counted write path (one placement = at least 1 Worker + 1 object request + 1 row — the product's own atoms); CPU on a remaining request (HMAC + hop +
  bytes ≈ 0.3 ms floor; 2.7 ms today = 9× at most); latency below one RTT for anything the device does not already hold; rows written below one per order.
- **Not a lever at all, with the number:** bebop kernels in the Worker (2.6–10.6× slower), dense SIMD (8× slower at 1×), wavelet/roaring on 7.9 KB, a browser bebop VM
  (second runtime; and the rule interpreter is 1 µs/event anyway), P2P for customers (concurrency ≈ 1; signalling counted), predictive push on Free (counted),
  compile-time fusion across the isolate boundary (physically not a thing).

### 8.4 The seven inputs, one line each (full judgements in the sections named)

| | Verdict | Where |
|---|---|---|
| I1 code-to-data | rules yes (content-hashed, versioned, server fallback); bytecode no (second runtime, no wasm backend, code-from-server to customers); projections yes (they are the answer) | §4.2 |
| I2 database-as-heap | true; the copy is 44–72 µs per 538 KB (MEASURED), the parse 6.33 ms; DG7 + DG5 `View` are 95 % of it; own format over rkyv | §3.1–3.2 |
| I3 client as Datalog node | "−99 %" is the compound of A1–A5 + P1 + the socket; the client node itself is the last ≈ 8×; worth it for one rule set on both sides, not for the saving | §2.3 |
| I4 compile-time fusion | false across Worker↔object; true and already spent inside the object (`Memo`, `/fold/*`); what remains is a typed command (P1) | §1.2 |
| I5 succinct/SIMD | data too small (7.9 KB); DG25 measured dense 8× slower; wasm SIMD in a Worker unmeasured — said so | §3.2 |
| I6 predictive streaming | against on Free (push = counted); keep `prefetch` of free assets | §2.2 |
| I7 P2P swarm | false for customers on 7 of 8 claims; egress already free; take the content-addressed blocks (BN2/BN3); staff-LAN is W2, later | §7.3 |

---

## 9. Ranked plan (rows: owned files, RED-first, acceptance number, model, lane-days) — fits AFTER the DAG and the approved order

Order of precedence this plan respects (memory `dowiz-quality-queue-2026-09-30`, `-10-01-additions`, operator 10-01): W-COV, auto-deploy (done), browser gate
(merged), split lib.rs, refunds + Stripe, DAG (DG8 landed; "38 sites"; DG6 landed `01783eb4`), Dependabot, ПРРО + offline sale, CI/observability/restore/load
(R3–R6), typed responses. The rows below slot after those; where a row IS an approved row seen from this angle (A1/C3 → BN2; A5 → BN1; P1 → BN4) it sharpens the
acceptance number rather than adding a lane. 3 lanes at a time; every row RED-first; every acceptance is a number in a gate or an eval, never a sentence.

| # | Row | Owns | RED first | Acceptance (number) | Model | Lane-days |
|---|---|---|---|---|---|---|
| **BN1** | **38 catalogue readers → block / `/fold/*`** (= A5 finished with DG7's blocks): every `load_catalog(`/`hubstore::load(` site in the Worker asks the object for the derived answer or the `?block=` bytes; no image crosses the hop on a traffic path | `workers/api/src/{storefront,services/**,owner/**,fold/**}.rs` (the 37 sites), `tools/gates/dataflow.sh` (baseline 38 → 0, ratchet), `crates/dowiz-hub/src/block/*` (add a column only with its twins) | `dataflow.sh` red at any site; per-site test that the handler's object calls carry no `catalog` image id | `dataflow` = **0**; placement CPU p50 < **2 ms** and menu-MISS path < **3 ms** in W-TELEM's per-route row; `cf.cpu_kills_day` = 0 on a busy day on qa-durres | Haiku ×2 (Opus if a site needs a new column) | 2 + 2 |
| **BN2** | **Publish-on-generation to R2 with content-addressed blocks + root manifest** (= A1/C3, hash names made the contract; the honest half of I7): the object writes `cdn.dowiz.org/v/<slug>/<k64>.{dwb,html,webp}` `immutable` + `manifest.json` (`max-age=30`) at every generation; `/` static; shell reads manifest → blocks | `workers/api/src/hubdo/menu.rs`, `+workers/api/src/hubdo/publish.rs`, `workers/api/wrangler.toml` (CDN bucket), `workers/api/public/store/index.html`, `+workers/api/public/store/shell.js`, `workers/api/public/_headers`, `workers/api/public/sw.js` (immutable paths cache-first; root network-first), `tools/platform/attach-host.sh`, `+e2e/kit-regression/storefront-r2.mjs` | `publish::tests::only_changed_blocks_are_written`; `menu::tests::fragment_bytes_pinned`; Playwright cold visit with `/api/*` blocked | Worker requests per cold browse ≤ **1** (from ≈ 22); menu paints with `/api/*` blocked; a one-dish price edit writes **≤ 3** objects (block, fragment, manifest), not photos; Class A per edit ≤ 6 | Opus (CSP + R2 domain + shell) | 4 |
| **BN3** | **Device replica by `k64` + browser block reader** (= FT11/B + DW1 start; the browser side of I2/I3): IndexedDB holds blocks by hash and the live-orders projection; `bebop-wasm` gains the block `view` twin; the socket (A2/A6) delivers `since=generation` deltas and the manifest version | `workers/api/public/lib/replica.js` (localStorage → IndexedDB), `+workers/api/public/lib/blocks.js`, `crates/bebop-wasm/src/{lib,block}.rs` + `gate.sh`, `workers/api/public/store/track.js`, `admin/core.js` | `bebop-wasm` block fixture 4/4 readers; Playwright: second visit with the network OFF renders the menu; a delta applied == a full refetch (golden) | return visit counted requests = **0**; time-to-menu warm < **100 ms** on the Playwright CPU-throttled profile; replica size < **1 MB** per venue | Opus | 3 |
| **BN4** | **One command, one object turn, one stepped memo** (= P1 + DG5 adoption): `POST /fold/command` enum `Place/Advance/Accept`; placement reserves stock, spends promo, appends, extends the `folded` memo with `fold_step`, broadcasts — in one turn; alarms consolidated per venue | `workers/api/src/hubdo.rs` (split first — file-size ratchet), `+workers/api/src/hubdo/command.rs`, `workers/api/src/storefront.rs` (compensation code deleted), `tools/gates/one-image.sh` (bodies with >1 `with_*` → 0), `workers/api/src/hubdo/timer.rs` | one-image gate red today; L3 collision probe (20 placements, last portion → 1×200, 19×409, 1 reservation); `stock::stranded()` empty after a forced append failure | object requests per placement **3 → 1**; rows per placement **3 → 1**; refold per write 5.75 ms → ≤ **0.1 ms** (`fold_step`); `cf.do_alarms_day` ≤ 40 % of today's per venue | Opus | 4 |
| **BN5** | **Worker wasm diet**: `twiggy top` → drop `chrono`, `icu_*`, `idna`, `url` where `worker` allows, `strum`, `wasm-streams` if unused; argon2 → the object (A8) after the OPEN 2 probe | `workers/api/Cargo.toml`, `workers/api/src/{auth,…}.rs`, `tools/gates/file-size.baseline`, `docs/research/2026-09-27-binary-size.md` (append the twiggy table) | a size gate: `index_bg.wasm` raw ≤ baseline, ratchet down | raw ≤ **2.5 MB** (from 4.75), gzip ≤ **1.0 MB**; Node harness compile+instantiate ≤ **50 ms** (from 92) on the A78 (`measure.mjs` from the cost map lane); upload time on the box halves | Haiku (Opus for the argon2 move) | 2 |
| **BN6** | **W1 — the cart as indices** (order payload ≈ 120 B; prices re-derived by the kernel from the catalogue generation in the event) | `crates/dowiz-core/src/{json_api,order_machine}.rs` (payload v3 behind a version byte), `crates/dowiz-hub/src/{event,log}.rs`, `workers/api/src/storefront.rs:762` region, `bebop-lang` log twins (same commit — the written-format law), goldens | golden log fixture with both payload versions; `one-image` unchanged; `float-money` | bytes per delivered order ≤ **200 B** (from ≈ 2 KB); rows per order **1**; every receipt reproduces the same total from `(catalogue_gen, indices)` on all four readers | Opus | 5 |
| **BN7** | **Store micro-fixes**: `Kv::get` binary search; `Catalog::load` over `View` (no `from_bytes` copy); `from_bytes` as one `memcpy` + reinterpret | `crates/bebop-store/src/{kv,lib}.rs`, `crates/dowiz-hub/src/catalog.rs`, `bebop-lang` kv twin if the layout is touched (it is not) | a test that `get` on a 10k-key fixture is < 1 µs; `copybench`-style bench row in `bebop-lang/docs/PERF.md` | `Kv::get` ≤ **200 ns** at 165 keys (from 345), ≤ 1 µs at 10k; `Catalog::load` ≤ **50 µs** at 538 KB | Haiku | 1 |
| **BN8** | **Instruments** (hand to W-TELEM / DW6; listed so they are not forgotten): `cf.do_per_worker_ratio`, `cf.cpu_kills_day`, per-route CPU, object CPU per turn, cold-start count per day, Worker requests per cold visit (Playwright, counted) | `tools/evals/collect/cf.mjs` + baselines | — | each printed nightly with a baseline | Haiku | (W-TELEM) |

**Refused rows, with the number:** P2P customer swarm (concurrency ≈ 1; signalling counted at 20:1; TURN 10–25 %; §7.3); predictive push on Free (every push a counted
request); compile-time fusion across the hop (not physically possible; inside the object it is done); dense SIMD / v128 (DG25: 8× slower; DG26 withdrawn); wavelet /
roaring on 7.9 KB; a bebop VM in the browser (second runtime, operator D-2); bebop kernels in the Worker (2.6–10.6× slower); SSE (no hibernation); Paid plan as a lever
(operator: stay Free).

**Order:** BN1 ‖ BN2 ‖ BN5 (three lanes, disjoint files) → BN3 ‖ BN4 ‖ BN7 → BN6. Total ≈ 23 lane-days (BN1 4, BN2 4, BN3 3, BN4 4, BN5 2, BN6 5, BN7 1) after the approved queue.

---

## 10. What could not be verified, and the command that would (incl. the measurements wanted from W-TELEM / DW6)

| # | Not verified | Command / measurement that settles it |
|---|---|---|
| U1 | Whether wasm compile (92 ms) is charged to a request's 10 ms on Free, and whether compiled code is cached across isolates (Cloudflare's docs do not say; third-party blogs claim a cache) | W-TELEM: `cf.cpu_p99_us` split by "first request after deploy" vs steady; a `/api/version` probe 60 s after a deploy with `cf-ray` and CPU from the analytics API |
| U2 | Per-route CPU and object CPU per turn (DW6) — the numbers §1.3 and BN1/BN4 accept on | `tools/evals/collect/cf.mjs` `cf.node_cpu_us` per `/fold/*` (DW6 row); the object's `cost_us` gauge in `health` |
| U3 | The DO ↔ Worker hop latency by colo pair (1 ms or 25 ms?) | `otel` span around `stub.fetch` on qa-durres, p50/p99 per `cf-ray` colo, 1 day |
| U4 | wasm SIMD in a real Worker (DG20 withdrawn) | the DG20 row as written; only if a scan ever appears in U2 |
| U5 | Browser frame time of 165 cards vs a prerendered fragment on an A53-class phone (cost map U11) | Playwright with CPU throttling 4× on the kit-regression storefront, `performance.measure` around first menu paint |
| U6 | Mean concurrency on one venue's menu at dinner (ESTIMATE ≈ 1 in §7.3) | the R6 load test's log, or `cf.worker_requests_day` by hour on dubin-sushi for a week via the analytics API (`/root/.cf_analytics_token`) |
| U7 | Whether cached custom-domain R2 reads are billed as Class B (cost map §8 carried) | R2 metrics after one day of BN2 on qa-durres: Class B count vs edge HITs |
| U8 | The 9fa24c24 "~70 µs per read" of `/fold/menu` on the live object (measured on the box, not in the object) | W-TELEM's `cost_us` on `/fold/menu` HIT vs MISS |
| U9 | Rows per placement after BN4 really 1 (alarm `setAlarm()` is itself a row written — CITED DO pricing) | `cf.do_rows_written_day` on qa-durres over the collision probe's 20 placements: ≤ 21 |
| U10 | CJEU Breyer C-582/14 as the IP-address authority (two fetches failed from the box: curia 404/redirect, eur-lex empty) — Recital 30 cited instead | `curl -L 'https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:62014CJ0582'` from a box with a working route to eur-lex |

Lane artefacts: `/root/lanes/w-bottleneck/.bneck-notes.md` (step log with every number's source); scratchpad `bneck/copybench.{rs,out}` (the microbench), `bneck/hdr*.txt`,
`bneck/menu2.json` (the six probes). No file in the tree was changed except this report.
