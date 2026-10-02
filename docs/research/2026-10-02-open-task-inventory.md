# Open task inventory — 2026-10-02 (lane W-INVENTORY, read-only, Fable)

> **Applied to ROADMAP-2026-09-22 on 2026-10-02 per operator decision** ("усе в роадмап, усе потрібно, ніяких видалень, усе обновити"): every OPEN / PARTIAL / QUEUED row below is now an approved row in `docs/design/ROADMAP-2026-09-22.md` (NEXT, Wave BN, the approved queue, Wave N0-N4); the OBSOLETE groups were NOT archived or deleted — each file got a dated STATUS banner instead. One correction found while applying: LE-14 / §2 item 11 (`tools/bpref.py` pre-A26) is already fixed (`8e438580`; `bpref.py:119-135`), so N0.4 is a re-check, not a fix.

main = `41b0d1d4` (`cov: route seam + in-memory platform; Worker 62.8% -> 90.0%`). No code was edited; every status below was checked against `git log`/`git grep` on this tree on 2026-10-02 (the command or commit is in the evidence column). Where a check was coarse the row says **verify**.

Status legend: **DONE** (+commit or code) · **IN-FLIGHT** (a lane holds it) · **QUEUED** (operator-approved, no lane yet) · **OPEN** (not queued) · **OBSOLETE** (+why) · **NEEDS-OPERATOR** · **PARTIAL** (half landed; the open half is named).

---

## 0. Відповідь оператору (одна сторінка)

**Скільки відкритого.** Переглянуто 11 джерел: живий роадмап `docs/design/ROADMAP-2026-09-22.md` (80 рядків), DAG-блюпринт (DG1-26, DW1-9), два звіти 2026-10-01 (A1-A12/B/C/R1-R6, BN1-BN8), чотири дослідження 2026-09-26 і чотири 2026-09-27, `bebop-lang/ROADMAP.md` (147 рядків), три аудити (09-21, 09-24, 09-27), 36 файлів пам'яті, 15 карток лейнів, 24 нотатки лейнів, маркери в коді, бейзлайни гейтів. Після дедуплікації (§4) — **228 рядків**, з них:

| статус | кількість |
|---|---|
| DONE (підтверджено комітом або кодом) | 96 |
| IN-FLIGHT (W-DGSWITCH, W-BN7, W-BN1A, W-TELEM-handback, деплой 41b0d1d4) | 7 |
| QUEUED (схвалено оператором, лейна ще немає) | 31 |
| **OPEN — не в черзі** | **55** |
| PARTIAL (половина є, друга половина в OPEN/QUEUED) | 12 |
| NEEDS-OPERATOR | 14 |
| OBSOLETE (закрити) | 13 груп (≈ 70 липневих документів + 13 модулів dowiz-core) |

**По областях (OPEN, не в черзі):** приватність/MCP Wave P — 13 (P4, P5, P7, P10, P11, P12, P13, P14, P15-ген2, P16, P17, P18, P19); гігієна Worker (спліти F8/F10/F11/F13, dead_code D6, FLOWS_BLOCKING, bw_block-фіча, сміття на qa-durres) — 8; аудиторські дірки (D13, D21, D34, O1, O7, O8, O14, catch-up window) — 8; кухня/Telegram/склад з досліджень 09-26 (K9, K13, K14, K15, TG-A5, TG-A6, A3-варіанс) — 7; bebop-компілятор (A18-2, A19-1b, A25-2, B6-таймінг, B7-explain, bpref-оракул, F5, F9) — 8; безпека/PQ (F15 X-Wing, F21, seL4 §4 рядки 1, 2, 4) — 5; інше (F5 інвайт кур'єра, scheduled_for, фільми, binary-size 4/5/8) — 6.

**Найважливіше з того, що НЕ в черзі (топ-10 за цінністю, §3 дає порядок):**
1. **`tools/bpref.py` досі моделює мову до A26** (`&&`=`&`, `||`=`|`) — оракул і компілятор перевіряють різні мови; «гейти важливіші за рядки». 1 lane-day.
2. **FLOWS_BLOCKING=1 за замовчуванням** — баг, через який браузерний гейт був advisory, виправлено 4d2d4f33, а деплой і далі пропускає червоні flows. 0.2 lane-day.
3. **Чотири файли Worker понад 1 100 рядків** (hubdo 1598, hubstore 1596, owner 1511, storefront 1117 — F8/F10/F11/F13); BN4 і BN1 пишуть у ті самі файли, спліт перед ними дешевший, ніж після. 3 lane-days.
4. **D21 WhatsApp `statuses`** — повідомлення поза 24-годинним вікном позначаються надісланими і не доходять (0 входжень `"statuses"` у коді). 1 lane-day.
5. **D34 чайові/компи у фіскальному документі** — `ebills_body` відмовляє, коли є чайові; латентно, поки SEND_ENABLED=false, але блокує увімкнення фіскалізації. 1 lane-day.
6. **Wave P: P4 (30-денний клок запитів), P5 (експорт даних клієнта), P10 (ROPA), P12 (ранбук витоку), P13 (DPIA — схвалено 09-26, не поставлено)** — законодавчі обов'язки з датою (Law 124 Art. 31 ≈ січень 2027). 5 lane-days разом.
7. **D13 офіціант списує будь-який гаманець за id** (G6 з аудиту 09-24; у `command/pay` немає перевірки токена/коду). 0.5 lane-day.
8. **O7 `rate_ppm` з клієнта** — каса закриває раунд за 1 цент; потрібен курс/коридор у налаштуваннях. 0.5 lane-day.
9. **F5 інвайт кур'єра доставляється, а не лише показується** (0 входжень rail/outbox у `hiring.rs`). 1 lane-day.
10. **DG18 причина суперлінійної компіляції** + **DG23 B6 1.00x** — Wave M схвалена, але лейна немає; обидва — чисті вимірювання. 1 lane-day.

**Застаріле, що треба закрити (§1 блок OBSOLETE):**
- `MASTER-ROADMAP-MVP-2026-07-12.md` (сам каже SUPERSEDED), `docs/design/ROADMAP.md` (липневий P01-P56), `CORE-ROADMAP-INDEX.md`, `CORE-ROADMAP-2026-07-17/`, `sovereign-roadmap-2026-07-16/`, ~60 `BLUEPRINT-*-2026-07-*.md` і `BLUEPRINT-ITEM-*`, `BLUEPRINT-W13..W22`, `BLUEPRINT-P97/P98/P102`: усі цілять в архітектуру, якої в дереві немає (rusqlite лише в `tools/deep-clean`; dtn7/bp7/quinn/pgrust/wgpu/native-spa-server — 0 входжень). Пропозиція: перенести в `docs/archive/2026-07/` одним комітом (потрібна згода — §2 №8).
- DG20, DG25, DG26 (тензори, SIMD) — зняті оператором 09-28 (C-1), лишаються лише як запис.
- Блюпринт FREE-TIER FT3/FT4/FT5/FT6/FT7/FT8/FT9/FT11 — не застарілі, але **повністю поглинуті** рядками A1-A9/BN2/BN3 (дублікати, §4).
- Картки W-WIRE, W-RU, W-KITCHEN, W-TG, W-INV, W-QA, W-UX, W-APPLE, W-AUDIT, W-KACCESS у `.claude/lanes/` — усе змерджено; W-PERF P1/P2 закрито b6f42cbd (P3 set_clock — див. рядок); W-VIDEO замінено рішенням «одна англійська версія» і 13 опублікованими уроками (лишається «33 фільми due»).
- **`crates/dowiz-core`: 230 модулів, 150 218 рядків; 13 найбільших липневих модулів (academia_p2p 5192, causal 2677, capability_cert 2250, spectral 2023, hub_provisioning 1971, prompt_enrich 1863, stem 1771, predictor 1460, orchestrator 1420, reverse_engineer 1308, hub_supervisor 1351, csr 1354, evals 1393) не згадуються ні у Worker, ні в hub** (`git grep` по `dowiz_core::<mod>` і `<mod>::` = 0 для кожного). Це ≈ 25 000 рядків, які компілюються в кожен `cargo test`. Пропозиція — рядок «W-REAP-CORE» (§2 №9, потрібне рішення, бо це видалення).

**Що потребує рішення оператора зараз:** 14 пунктів у §2; найтерміновіші — №5 (ігнор wasmtime RUSTSEC-2026-0316 спливає **2026-10-14** і тоді `deps-audit` зупинить кожен деплой), №3 (секрет CF для R3 — лейн чекає), №6 (`scheduled_for_ms` тихо губиться: передзамовлення «на пізніше» — це фіча чи прибрати поле з чекаута?).

---

## 1. Inventory table

Sizes are ESTIMATES in lane-days (S ≤ 1, M 2-3, L ≥ 4), based on the pace of the nearest finished row.

### 1.1 In flight and approved queue (recorded, not re-planned)

| id | source | item | status | evidence | size | depends on |
|---|---|---|---|---|---|---|
| DGSW | memory quality-queue-09-30; /root/lanes/w-dgswitch/.dgsw-notes.md | DAG bebop side: d99_cap memo reuse, dagfull store arm, DG8→st_proj binding, §5 switch checklist | IN-FLIGHT | notes 2026-10-02: step 1 d99_cap FIXED (gen4 339a6010), battery pair next, then restore bebop.bin 8a0b4325 | M | — |
| BN7 | bottlenecks §9 | store micro-fixes (Kv::get binary search, Catalog::load over View, from_bytes memcpy) | IN-FLIGHT | .bn7-notes: 65 tests green; kv_get_10k 526 ns (was 21,600); kv_get_165 371 ns vs 200 limit still red | S | — |
| BN1A | bottlenecks §9 BN1 | 15 of 38 dataflow sites → /fold/* | IN-FLIGHT | .bn1a-notes: 15 sites listed, none moved yet | M | — |
| TELEM | memory additions-10-01 §3 (DW6) | telemetry lab as a service | IN-FLIGHT (done-unmerged) | .telem-notes: hand-backs = merge `telemetry = []` feature beside `shred`; telemetry.prove.sh GREEN run owed; BN8 counters not started | S to merge | box quiet |
| DEPLOY | task brief | deploy 41b0d1d4 | IN-FLIGHT | w-fresearch 10-01 saw `/api/version` 404 (d830cccb not live); box cannot resolve the host today | — | uplink |
| BN1B | bottlenecks §9 | remaining 23 dataflow sites | QUEUED | dataflow.baseline `sites=38`; target 0 | M | BN1A |
| BN2 | bottlenecks §9 (= A1/C3) | publish-on-generation to R2, content-addressed blocks + manifest, CSP | QUEUED | no `r2_buckets` for media in wrangler.toml (cost §1.1 FT3 NOT) | L (4) | R2 custom domain (operator) |
| BN3 | bottlenecks §9 (= FT11/B + DW1 start) | device replica by k64 + browser block reader (IndexedDB) | QUEUED | IndexedDB only in `lib/outbox.js` | M (3) | BN2 |
| BN4 | bottlenecks §9 (= arch-evo P1 + A7) | one command, one object turn (`POST /fold/command`) | QUEUED | one-image.baseline still names `campaigns::handlers::send_now` | L (4) | hubdo split (F10) advisable first |
| BN5 | bottlenecks §9 (= binary-size row 7) | Worker wasm diet (twiggy; drop chrono/icu/idna/url) | QUEUED | wasm 4,747,270 B (w-fresearch 10-01) | S (2) | — |
| BN6 | bottlenecks §9 (= W1) | cart as indices, payload v3 | QUEUED | — | L (5) | BN1, BN4 |
| BN8 | bottlenecks §9 | instruments (`cf.cpu_kills_day`, per-route CPU, …) | QUEUED | .telem-notes: NOT started | S | TELEM merged |
| REFUND | memory quality-queue-09-30 row 5; dowiz-order-fsm-has-no-exit | refund/cancel after PENDING + Stripe money back | QUEUED | 8e188016 route exists; `/v1/refunds` 8 hits in 5 files — verify whether a real Stripe refund is issued | M | — |
| PRRO | quality-queue-09-30 row 8 | Ukraine ПРРО 36 h offline + full offline sale | QUEUED | 0 commits, 0 code hits for `prro|ПРРО|offline_sale` | L | — |
| TYPED | additions-10-01 §2 | typed responses: ts-rs .d.ts + `@ts-check` + tsc gate + json! ratchet | QUEUED | 0 hits `ts_rs|ts-rs|.d.ts`; 2,355 `json!` sites | L | — |
| R3 | cost report §6 | deploy + heavy gates from GitHub Actions | QUEUED / **NEEDS-OPERATOR** | `.github/workflows` has ci/key-flows/mutations/release/health-cron/evals-nightly, none runs wrangler deploy; CF token secret missing | M (2) | operator secret |
| R4 | cost report §6 | production observability + Telegram alerts on Free | QUEUED | `alert.rs` exists (ru commit); no alarm-driven alert on worker errors found (verify) | M (3) | R3 |
| R5 | cost report §6 | weekly backup-restore drill | QUEUED | `restore` route at operations/mod.rs:278; no drill script (`drill` only in docs) | M (2) | R3/R4 |
| R6 | cost report §6 (= F20 soak) | dinner-peak load test on qa-durres | QUEUED | `soak` 2 files in tools/e2e (partial harness) | M (2) | R4 |
| VOICE | additions-10-01 §1 | offline on-device voice research (Fable) | QUEUED | — | research | — |
| DW1 | ROADMAP-09-22:304; DAG BP §4 | browser replica as graph subscriber | QUEUED | = BN3's second half | M | DG5, BN3 |
| DW2 | ROADMAP-09-22:305 | personal-data labels as `personal(N)` rule | QUEUED | DG8 landed `personal` rule 8300b51b; edge table + gate rewrite open | M | DG8 |
| DW3 | ROADMAP-09-22:306 | gates as graph queries (`edges.json`) | QUEUED | — | M | DW2 |
| DW4 | ROADMAP-09-22:307 | conservation and stock-vs-orders as per-edge invariants | QUEUED | — | M | DW2 |
| DW5 | ROADMAP-09-22:308 | provenance "why is this number X" | QUEUED | — | S | DW2 |
| DW6 | ROADMAP-09-22:309 | per-node CPU/bytes accounting | QUEUED (= TELEM) | see TELEM | — | — |
| DG10w / DW8 | DAG BP §4; w-dg10 notes | crypto-shredding Worker wiring (seal at place-order, key-table image, registry row, conservation law `chain.redacted == declared`) | QUEUED | b29310c4 hub side only (`feature shred`, default off); w-dg10: "Worker wiring NOT owned → OPEN"; law has 1 hit in tools/evals (verify it is the law) | M | — |
| WAVE-M | DAG BP §6 | DG18 superlinear-compile cause, DG19 fold wall on QA hub, DG23 B6 scan | QUEUED | DG17 DONE (c4c9f1e9 "edit wall median 124 ms"), DG21 DONE (w-dg10 step 2 KAT), DG22 DONE (01783eb4 "sorryAx 0"); 18/19/23 untouched | S each | DG19 needs the QA hub + a deploy |
| A1-A5 | cost report §2 | A1 storefront read path to R2 (= BN2), A2 socket-first polling, A3 auto-pong, A4 registry Cache API 60 s, A5 dataflow 38→0 (= BN1) | QUEUED | FT7/FT6/FT8 NOT per cost §1.1 (`track.js:38 POLL_MS=12_000`; 0 hits `auto_response`; `cache.put` only in storefront/media/rates) | A2+A3+A4+A6 = 2 | — |
| A6 | cost report §2 | customer tracking by socket only | QUEUED (part of A2) | — | — | A2 |
| B | cost report §3 | IndexedDB stores per surface (= BN3) | QUEUED | — | — | — |
| C | cost report §4 (C3) | publish-time prerender to R2 (= BN2) | QUEUED | — | — | — |
| A8 | cost report §2 | FT4 login argon2 + FT5 nightly into the object | QUEUED (gated on OPEN 2 object-CPU probe) | `auth.rs:250 Params::new(8*1024,3,1)` runs in the Worker | M+M | BN8 probe |
| A9 | cost report §2 | FT3 interim: mime in KV metadata (2 writes → 1) | OPEN (not in the approved list; obsolete once BN2 lands) | — | XS | — |
| A10 | cost report §2 | protect the caps: zone rate-limit rule on `/api/*`, `X-Robots-Tag` | **NEEDS-OPERATOR** (zone rule = dashboard) | — | XS | operator |

### 1.2 Live roadmap `docs/design/ROADMAP-2026-09-22.md` — Phases A-D (labels there are stale "IN FLIGHT" from 09-22)

| id | source | item | status | evidence | size | depends on |
|---|---|---|---|---|---|---|
| A2-A3 | ROADMAP-09-22:94-95 | tables as data; booking from storefront | DONE | 7496e129 "floor: six table states as a fold"; `booking/` split (booking.rs 194 lines) | — | — |
| A4 | :96 | waiter role (`Staff` principal) | DONE | `Principal::Staff` 94 hits; `/room/` surface | — | — |
| A5-A7 | :97-99 | open tab/Amended, split/transfer/void, till X/Z | DONE | `command/transfer.rs`, `command/till.rs:221 counted_at`; 0fa5d37b "till and tips fixes" | — | — |
| A8 | :100 (L63) | print routing by station, tips per person | DONE | 1fb88ab5 "a station per Telegram group"; `station` 207 hits | — | — |
| A9 | :101 (L66) | QR onto the storefront | DONE | b1d256a8 "table QR: a signed link per table" | — | — |
| B1-B6 | :107-112 | tax as integer, inclusive/exclusive, price per channel, history, `channel`, fiscal seam | DONE | `tax_of` 65 hits (c496c2d9 uses it); `channel` 93 hits | — | — |
| B7 | :113 (L70) | ebills.al import + real sender | DONE (sender built, **switched off** by operator) | EBILLS §8.1 (09-23); memory fiscal-send: `SEND_ENABLED=false` (11 hits) | — | operator decision to arm |
| B8 | :114 | the receipt (format, printing) | DONE (printer tickets) — verify receipt *format* row | d8969b01 "a printer pulls it"; `receipt` 95 hits | — | — |
| C1-C6 | :120-125 (L64, L69, L71) | customer record, consent, erasure, aliases, stamp card, campaigns | DONE | 52df6b97 (consent log, forget); `Linked|alias` 142 hits; `stamps` 130 hits; `send_now` in one-image.baseline | — | — |
| D1 | :131 | stack audit blueprint | DONE | BLUEPRINT-STACK-AND-DEPENDENCIES-2026-09-22.md | — | — |
| D2 | :132 | courier PWA service worker | DONE | sw-shell.baseline `missing=0` ("guards all four shells") | — | — |
| D3 | :133 | `Currency::minor_units()` in the kernel | DONE | `minor_units` 6 hits in 5 files | — | — |
| D4 | :134 | `KINDS` into dowiz-core, `lib/fulfilment.js` generated | DONE (verify generation step) | `KINDS` 34 hits; tools/gen-vocab exists | — | — |
| D5 | :135 | `unreached` baseline to zero | DONE | unreached.baseline `0`; 827f7902 | — | — |
| D6 | :136 (L67) | remaining `#[allow(dead_code)]` to zero | **OPEN** | `allow(dead_code)` 66 hits in 34 files under workers/api/src + crates (some are tests; count by hand before the lane) | S | — |

### 1.3 Wave F — launch gaps (ROADMAP-09-22 §Wave F, 2026-09-24)

| id | source | item | status | evidence | size | depends on |
|---|---|---|---|---|---|---|
| F1 | :213 | bulk CSV import of supplies/recipes | DONE | 8a9995c6; `crates/dowiz-hub/src/import.rs` | — | — |
| F2 | :214 | recipe templates + suggestion | DONE (verify templates dir) | `fn suggest|templates` 12 hits | — | — |
| F3 | :215 | supplier invoice as one receipt | DONE | c0262881 "raw is received with its `total`" | — | — |
| F4 | :216 | (marked DONE in the doc) | DONE | doc | — | — |
| F5 | :217 | courier invite **delivered** by the venue's rail | **OPEN** | 0 hits `rail|outbox|Entry` in `services/courier/hiring.rs` | S | — |
| F6 | :218 | walk harness `e2e/walk/` | DONE | 34 files; c25e4ae2 live-walk scripts; b965858f flows gate | — | — |
| F7 | :219 | QA venue qa-durres | DONE | all flows/probes run on it | — | — |
| F8 | :220 | split `hubstore.rs` (1671) | **OPEN** | `wc`: hubstore.rs **1596** | S | — |
| F9 | :221 | split hub `lib.rs` | DONE | 4dcbc0e1 (1,655 → 99; now 101) | — | — |
| F10 | :222 | split `hubdo.rs` (1560) | **OPEN** | hubdo.rs **1598** (binary-size row 5 names its 409 KB `fetch`) | M | do before BN4 |
| F11 | :223 | split `owner.rs` (1477) | **OPEN** | owner.rs **1511** | S | — |
| F12 | :224 | split `stock.rs` | DONE | stock.rs 249 | — | — |
| F13 | :225 | split `storefront.rs` (1391) | **OPEN** | storefront.rs **1117** | S | BN1A touches it (`storefront.rs:456`) — sequence |
| F14 | :226 | split `booking.rs` | DONE | booking.rs 194 | — | — |
| F15 | :227 | `pq::hybrid` X-Wing-exact + draft vectors; dwzseal v2 | **OPEN** (memory PQ = option B: make ML-KEM real) | 0 hits `X-Wing|SHA3|label` in `pq/hybrid.rs`; `kem.rs` KAT exists | M | — |
| F16 | :228 | switch the seal on (keygen off-platform, `BACKUP_SEAL_PK`) | **NEEDS-OPERATOR** (approved 09-26 "seal key"; the key is the operator's act) | `BACKUP_SEAL_PK` 3 hits in workers/api (code half present) | XS | operator |
| F17 | :229 | README/landing words on the sealed copies | **NEEDS-OPERATOR** (conflicts with memory legal-texts "never claim post-quantum") | 0 hits `ML-KEM|X-Wing|sealed` in README.md | XS | F15, F16 |
| F18 | :230 | per-venue DO alarms replace crons | DONE | 92f96221, d7279037; one cron `17 3 * * *` | — | — |
| F19 | :231 | wildcard route | DONE | `dowiz.org/*` in wrangler.toml; memory operator-unblocks F19 token half | — | — |
| F20 | :232 | soak on the preview Worker (10/50/200 venues) | PARTIAL → merged into R6 | `soak` 2 files | — | R6 |
| F21 | :233 | registry's real capacity test (`K_LOC` until `arena_full`) | **OPEN** | 0 hits `arena_full|K_LOC` in platform_store.rs | XS | — |

### 1.4 Wave P — privacy and MCP (ROADMAP-09-22 §Wave P, 2026-09-24)

| id | source | item | status | evidence | size | depends on |
|---|---|---|---|---|---|---|
| P1 | :258 | personal-data registry + gate | DONE | 52df6b97; personal-data.baseline `unaccounted=0` | — | — |
| P2 | :259 | erasure reaches every store; law `redacted = declared` | PARTIAL | forget routes exist (`/customers/:key/forget`, `reforget`); w-dg10: conservation.mjs has no `chain.redacted == declared` law (1 hit for `redacted` in tools/evals — verify) | S | DG10w |
| P3 | :260 | backups 21 d + erasure register replayed on restore | DONE | `cloud.rs:274 KEEP_WEEKLY_MS = 21 d`; `reforget` route | — | — |
| P4 | :261 | privacy-request intake with a 30-day clock | **OPEN** | only hit for `due_at` is a cron timer test | M | — |
| P5 | :262 | access/portability export, `restricted` flag, "forget me on this device" | **OPEN** | `fn export` only = venue backup (hubstore.rs:1472) and otel | M | — |
| P6 | :263 | retention jobs (24/12 months) | PARTIAL — verify | `retention` 48 hits in 7 files; operator approved retention 09-26; no commit titled retention | S | — |
| P7 | :264 | rights for couriers, staff, owners, prospects | **OPEN** | forget routes exist for customers only | S | — |
| P8, P9 | :265-266 | notice, DPA, processors | DONE | 52df6b97; `docs/privacy/DPA-v1-*.md` ×4, `PROCESSORS.md` | — | — |
| P10 | :267 | ROPA rendered from the registry | **OPEN** | 0 hits `\bropa\b|ROPA` in src/docs/privacy | S | P1 |
| P11 | :268 | minimisation (no contact in assistant facts, ticket phone only for delivery) | OPEN — verify | `contact` 5 hits in services/engagement | S | — |
| P12 | :269 | breach runbook with `aware_at` | **OPEN** | 0 hits in docs/privacy and src/privacy | S | — |
| P13 | :270 | DPIA screening (courier location, CRM) — operator approved 09-26 | **OPEN** (approved, never queued) | DPIA appears only in the blueprint | S | — |
| P14 | :271 | OAuth 2.1 AS in the Worker | **OPEN** | 0 hits `oauth` in workers/api/src | L | — |
| P15 | :272 | `/mcp` for every role, two protocol generations | PARTIAL | 85b1d1d7 MCP for every role; `mcp.rs:36 PROTOCOL_VERSION = "2025-06-18"` only — the 2026-07-28 generation open | S | — |
| P16 | :273 | tool catalogue covers every route + gate | OPEN — verify | `catalogue` 5 hits in mcp/gates | S | — |
| P17 | :274 | MCP resources and prompts | **OPEN** | 0 hits `morning_check|close_the_day|eighty_six` | S | P15 |
| P18 | :275 | scopes with step-up, two-phase dangerous actions | **OPEN** | 0 hits `insufficient_scope|two_phase|dangerous` | M | P14 |
| P19 | :276 | one scripted e2e session per client (claude -p / codex exec / opencode run) | **OPEN** | 4 hits, docs only | S | qa-durres |
| P20 | :277 | connected agents in the console | PARTIAL | `admin/mcp.js` + 92219a57 setup snippets; grants list/revoke not found | S | — |

### 1.5 DAG programme (DAG BP + bebop ROADMAP Phase G) — status of every row

| id | status | evidence |
|---|---|---|
| DG1 | DONE | 0f745db5 (13.3 s → 0.79 s) |
| DG2 | DONE | d36a32cb |
| DG3 | DONE | 36f72b69 (4.25 MB → 543 KB) |
| DG4 | DONE | 9ac97c69 + c4c9f1e9 (memo image, edit wall 124 ms) |
| DG5 | DONE (store projections) / dagfull store arm IN-FLIGHT | 64d2d5cd; W-DGSWITCH |
| DG6 | DONE | 01783eb4 (bebop.bin 8a0b4325, Dag.lean sorryAx 0) |
| DG7 / DW7 | DONE | c496c2d9 (165-dish decode 8,066 ns) |
| DG8 | DONE; st_proj binding IN-FLIGHT | 8300b51b; W-DGSWITCH |
| DG9 | DONE; hand-back: Cargo `block = []` feature instead of `--cfg bw_block` | 9361706b + c4c9f1e9 follow-up; .dg9-notes:30 | **OPEN** XS |
| DG10 hub / DW8 Worker | DONE hub side / QUEUED Worker side | b29310c4; w-dg10 notes |
| §5 switch checklist | IN-FLIGHT | W-DGSWITCH |
| DG17, DG21, DG22 | DONE | c4c9f1e9 124 ms; w-dg10 step 2 KAT; 01783eb4 sorryAx 0 |
| DG18, DG19, DG23 | QUEUED (Wave M) | no lane |
| DG24 | blocked (no board) | DAG BP §6 |
| DG20, DG25, DG26, DW9-SIMD | OBSOLETE (withdrawn, C-1, 7cdcf0ab) | — |
| DG11-16 | never existed (numbering gap) | DAG BP line 7 |
| DAG BP §9 `KEEP_GENS = 16` hypothesis | OPEN measurement (compiles per battery) | DAG BP §9 | XS |

### 1.6 Audits

| id | source | item | status | evidence | size |
|---|---|---|---|---|---|
| G1-G3 | AUDIT-09-24 §4 | idem verdicts, courier shell, venue-day | DONE | idem-done/sw-shell/venue-clock baselines all 0 | — |
| G4, G5, G7, G9 | §4 | pay/reject rules, refund→wallet+till, pass verifier, owner-assign accept | DONE | 0fa5d37b (Wave G) | — |
| D13 (G6) | §1b:68 | waiter can debit any wallet by id | **OPEN** | no token/code check found in `command/pay/mod.rs` (`wallet` guard grep empty) | S |
| D18 (G6) | :73 | any principal reads any thread; any customer token posts anywhere | verify (principal-binds.baseline says none open) | principal-binds gate green | — |
| D15 (G8) | :70 | forget walks aliases, bookings, outbox | DONE (verify outbox) | aliases + forget in code; L64 | — |
| D5/D6/D17 (G10) | — | catalogue import merge, allergen gate on create/import | DONE | 8a9995c6 import preview; 8300b51b allergen rule finding | — |
| D19 (G11) | :74 | recipe ↔ published values | DONE | d24e6da2 "a supply or card edit re-derives every dish" | — |
| D20 (G12) | :75 | outbox lease per drain | DONE | `lease` 31 hits in outbox | — |
| D21 (G12) | :76 | WhatsApp `statuses` webhook read | **OPEN** | 0 hits `"statuses"` | S |
| D22 (G12) | :77 | outbox TTL for unconfigured channels | DONE (verify) | `ttl|expires` 10 hits in outbox | — |
| D23, D30 (G13) | :78,85 | dine-in without staff needs `table_link`; `live_sitting` ignores paid | DONE | b1d256a8 (A9 closed) | — |
| D33 (G14) | :88 | stock settles on IN_DELIVERY path | DONE (verify) | `IN_DELIVERY` 29 hits in stock/command | — |
| D24, D25 (G15) | :79-80 | stamps need verified customer | DONE (verify) | L71 stamp card lane | — |
| D27, D29 (G16) | :82,84 | floor save vs live holds; hours + `cancellation_is_free` | DONE | `cancellation_is_free` 7 hits; `orphan|live hold` 13 hits | — |
| G17 | §4 | `route-contract.sh` + uncalled owner routes get a UI | DONE | ui-reach.baseline 0; c3f23373 W-WIRE | — |
| D34, D37 (G18) | :89,92 | tips/comps as fiscal lines; reconcile before retry | **OPEN** (D34), PARTIAL (D37 `reconcile` 29 hits) | 0 hits `tip` in `ebills_body.rs` | S |
| D36, D39, D40 (G19) | :91,94 | courier cash major units; deactivation per venue | DONE (D36: `lib/money.js` imported in courier/app.js; fa2e7ace) / verify D39 | — |
| D38, D35 (G20) | :93,90 | one phone canonicaliser; unit change refused | DONE | `canonical_digits` 23 hits; unit-change 5 hits | — |
| O1 | docs/audit/2026-09-27 | Stripe webhook stamps paid on a CANCELLED order | **OPEN** | stripe.rs:288 comment only | S |
| O2-O6, O9-O11 | same | — | DONE | fa2e7ace, ed3be201 | — |
| O7 | same | `rate_ppm` from the client | **OPEN** | fx.rs has no band/venue rate | S |
| O8 | same | `logimage::keep` drops quarantine | OPEN — verify | `quarantin` 4 hits in logimage.rs | XS |
| O12, O13 | same | cosmetic truncations; `Response::error(..).unwrap()` | OPEN (cosmetic) | — | XS |
| O14 | same | kit prints `$` + cents/100 when currency missing | **OPEN** | memory kit-had-a-fourth-money-copy | XS |
| FA-1 | memory fable-audit-09-21 | backups 8 copies, not 7 d + 4 w | DONE | cloud.rs:265-277 three-week rule (P3) | — |
| FA-2 | same | hibernated sockets outlive their principal | DONE (verify) | `get_websockets_with_tag` 6 hits (hubdo.rs, hubdo/host.rs) | — |
| FA-3 | same | catch-up window holds one change (`RECENT_KEEP` dead) | **OPEN — verify** | `RECENT_KEEP = 256` at hubdo.rs:107; `put_image` body (986-1003) no longer touches `recent` — may be fixed by phase-6 rework | XS |
| FA-4 | same | `chain_check` called by nothing | DONE | 2 non-test callers | — |
| FA-5 | same | pickup order has no terminal state; couriers offered collections | DONE (state) / verify tasks filter | `PickedUp` terminal in order_machine.rs:24,74; no `pickup` filter found in `services/courier/tasks*` | XS |
| FA-6 | same | FSM exit + Stripe money back | = REFUND (queued) | — | — |

### 1.7 Research rows 2026-09-26 / 09-27 (kitchen, stock, Telegram, Russian, perf, size, seL4)

| id | source | item | status | evidence | size |
|---|---|---|---|---|---|
| K0-K7, K12 | kitchen-role.md:447-460 | kitchen surface, board, caps, stock screens | DONE | 2d652dfc, c9ccbfad | — |
| K8 | :455 | yield test event + screen | DONE | `yield` 45 hits | — |
| K9 | :456 | yield proposal → owner approval | **OPEN** | 0 hits | S |
| K10 | :457 | stop list / 86 as its own cap | DONE (via kitchen access) — verify cap | W-KACCESS | — |
| K11 | :458 | FT6 ping auto-response | = A3 (queued) | — | — |
| K13 | :460 | who-tapped PIN for stock writes | **OPEN** | 0 hits `by_pin` | S |
| K14 | :461 | prep list + forecast | **OPEN** | 0 hits | M |
| K15 | :462 | room toast when my round is READY | **OPEN** (verify) | logic.js maps READY to kitchen; no toast found | XS |
| TG-A0..A3 | telegram-groups.md:275-278 | webhook, groups model, event sources, owner matrix | DONE | 0801fb8d, c3f23373, 1fb88ab5 | — |
| TG-A4 | :279 | digests + quiet hours | DONE | `digest_rail.rs`, 37 hits | — |
| TG-A5 | :280 | inline actions (`ready`, `86`, `cancel`) | **OPEN** | 0 hits `callback_query` | M |
| TG-A6 | :281 | pacing, 403 auto-mute, HTML opt-in | PARTIAL | `mute|pacing` 3 hits | S |
| RU-B0..B6 | :370-376 | Russian everywhere | DONE | a3cc8f86 (langs gate); B4 ru films OBSOLETE (operator: English only) | — |
| ST-R4, R7 | menu-ingredients-stock.md (W-PERF P1/P2) | cost at placement; stock checkpoint | DONE | b6f42cbd | — |
| ST-P3 | W-PERF-CARD P3 | `set_clock` at remaining stock writes (room, refund, ebills) | PARTIAL | refund has clock_tests; room.rs 1 hit; ebills not found | XS |
| ST-A3 | :343 | stocktake variance valued | PARTIAL | 4 hits | S |
| ST-A9 | :349 | days of cover / reorder | PARTIAL (`lowAt` exists) | — | S |
| ST-A10 | :350 | lots, expiry | DONE | 36 hits `expir` in stock | — |
| ST-A2, A4-A8, A11-A14 | :342-354 | analytics rows (waste valued, theoretical vs actual, food cost %, menu engineering, supplier trend, top consumed, valuation, lost sales, nutrition/100 g) | DONE in part (93396cbf "kitchen numbers", d4bff908 analytics through ПФ) — unverified per row | — | — |
| PP-P0 | performance-paths.md:141 | Workers Paid | OBSOLETE (operator: FREE) | memory operator-decisions-10-01 | — |
| PP-P1..P4 | :142-145 | = BN1, FT2 (DONE), A4, A3 | dup | — | — |
| PP-P5 | :146 | opt-level s vs z | DONE (measured, keep `-O`) | binary-size §7.2 (W-STRIP) | — |
| BS-1, BS-2 | binary-size.md §4 | wasm baseline; strip names | DONE | b6f42cbd, 96c2b790 | — |
| BS-3 | §4 | `-Oz` | DONE (rejected, measured) | binary-size §7.2 | — |
| BS-4 | §4 | try `opt-level = "s"` | **OPEN** (one probe) | — | XS |
| BS-5 | §4 | split `HubImages::fetch` (409 KB fn) | **OPEN** (= F10) | — | — |
| BS-6 | §4 | byte pass-through on poll/menu | DONE | 9fa24c24, e6ab7182 | — |
| BS-7 | §4 | drop url/idna/icu | = BN5 | — | — |
| BS-8 | §4 | `wasm-snip` panic/fmt paths | **OPEN** | — | S |
| BS-9, 10 | §4 | panic-unwind / immediate-abort std | deferred (toolchain pin) | — | — |
| SEL4-1 | sel4-wasm.md §4 | declared write manifest per module | **OPEN** | — | M |
| SEL4-2 | §4 | secrets out of the storefront module | **OPEN** | — | S |
| SEL4-3 | §4 | `--panic-unwind` evaluation | deferred (= BS-9) | — | — |
| SEL4-4 | §4 | sign the chain (`actor_pubkey` zero on every append) | **OPEN** | hubdo.rs:1578 per doc | M |
| SEL4 B1/B2 | §5 | seL4 Box / native seL4 | OBSOLETE until re-entry condition | — | — |
| BWU | backend-without-ui.md §2.1 | 15 orphan routes get a UI | DONE | c3f23373; ui-reach 0 | — |

### 1.8 Loose ends from commit messages, lane notes and memory

| id | source | item | status | evidence | size |
|---|---|---|---|---|---|
| LE-1 | d78f496d body | checkout's `scheduled_for_ms` dropped silently | **NEEDS-OPERATOR** | `hubstore/carry.rs:56` now *declares* the drop; the storefront still offers "later" | S if feature |
| LE-2 | tools/deploy/deploy.sh:174 | `FLOWS_BLOCKING` default 0 "until the rename-back bug is fixed" | **OPEN** — bug fixed 4d2d4f33, default never flipped | XS |
| LE-3 | 44b33c9d, d4bff908 | 33 films due; owner probe re-run pending; lessons O22a/O22b not filmed | **OPEN** | memory autopilot-09-27 | S |
| LE-4 | a7fcbd4d | rebuild path re-folds the all-raw bom only | documented limit (OPEN-low) | cost.rs | S |
| LE-5 | .flows-notes:6, .verify-notes:12 | qa-durres litter: 16 "QA Guest Arbenita" orders, leftover qa-pf categories | **OPEN** housekeeping (verify-lane deleted the categories) | — | XS |
| LE-6 | .strict-notes:22 | eta `EtaBody`, waitlist `Join`, social `SendBody` left non-strict on purpose | decided (not a task) | — | — |
| LE-7 | .cov-notes:96 | coverage.sh skipped in run-all; "its own CI job (ci.yml text in hand-back)" | **OPEN** | — | XS |
| LE-8 | memory deps-audit-needs-weekly-refresh | advisory-db weekly refresh; wasmtime RUSTSEC-2026-0316 ignore expires **2026-10-14** | **NEEDS-OPERATOR** | gate refuses run-all → every deploy | XS |
| LE-9 | .deps-notes:39 | Dependabot leftovers: rustls in `kernel` (handed back), mesh-adapter unresolvable | **NEEDS-OPERATOR** (delete the legacy crates?) | 553abec7 fixed the rest | S |
| LE-10 | markers grep | code markers: 23 hits, of which **1 real** `TODO(operator)` at `crates/dowiz-core/src/markov.rs:209`; the 12 `OPEN:` are test fixtures/grammar words, the 11 `XXX` are mktemp templates and currency tests | OPEN-low | — | XS |
| LE-11 | memory courier-login-venue-from-host | sushi-durres has no usable courier credential | human task (venue) | — | — |
| LE-12 | memory dubin-sushi-inventory-applied | dubin inventory WIPED 09-27 on operator order; "the new person fills it in" | human task (venue) | — | — |
| LE-13 | memory bebop-clone-kept-symbol-limit | A19 step 1b (kept-symbol count by hand) | **OPEN** (bebop) | — | S |
| LE-14 | memory bpref-models-the-pre-a26-language | `tools/bpref.py` still `&&`→`&`, `||`→`\|` | **OPEN** (bebop gate quality) | — | S |
| LE-15 | memory f6-bitblaster | `bb_mul64` capacity | OPEN (bebop, design) | — | M |
| LE-16 | memory d5b-witness-buffer-segv | `use` splicing SIGSEGV | OPEN (bebop) | — | M |
| LE-17 | bebop ROADMAP rows | A11 rest (harness dogfooding), A18 step 2 lint, A25 step 2, A29 (conditional), B2 (thesis twins, UNMET), B6 timing arm, B7 `explain` SIGSEGV, C1 steps 3-4, C5, C6 (do not start), D4 (not landed), F5 translation validation, F9 precondition | OPEN (bebop) — 13 rows | ROADMAP.md status words (see notes) | S-M each |
| LE-18 | bebop ROADMAP:208 | design-bound T-rows: T61, T68-T70, T85→T86, T73, T76, T49/T50, T56, T59 | **NEEDS-OPERATOR** ("operator decision first") | — | — |
| LE-19 | ebills §8.3 | poller role narrowest (`ROLE_REPORT` may be refused) | OPEN-low | — | XS |
| LE-20 | evals blueprint §9 | run `traffic.mjs` baseline; ETA backtest; nightly soak | PARTIAL (evals-nightly.yml exists; ETA backtest not found) | — | S |
| LE-21 | hub-cost §6 / memory seven-phases | phase 3a line codebook — deliberately not done | decided | — | — |
| LE-22 | file-size.baseline | `over=20 worst=1653` → 300-line rule | OPEN ratchet (= F8/F10/F11/F13 + 16 others) | — | — |
| LE-23 | float-money.baseline | `6` → target 0 | OPEN ratchet | — | S |
| LE-24 | ui-adoption.baseline | residue `admin.button=1 admin.field=1 …` | OPEN ratchet | — | XS |
| LE-25 | dowiz-core probe | 13 July-era modules (≈25 k lines) referenced by neither Worker nor hub | **NEEDS-OPERATOR** (deletion) | `git grep dowiz_core::<mod>` = 0 each | M |

### 1.9 OBSOLETE (close; evidence per group)

| group | files | why |
|---|---|---|
| OB-1 | `MASTER-ROADMAP-MVP-2026-07-12.md` | header: SUPERSEDED 2026-07-17; its successor file no longer exists |
| OB-2 | `docs/design/ROADMAP.md` (July P01-P56), `CORE-ROADMAP-INDEX.md`, `CORE-ROADMAP-2026-07-17/`, `sovereign-roadmap-2026-07-16/`, `ROADMAP-*-2026-07-*.md`, `WAVE-CLOSEOUT-P57-P74` | the architecture they plan (rusqlite nodes, dtn7/BPv7, QUIC bearer, pgrust, Astro) has 0 code in workers/crates (rusqlite only in `tools/deep-clean`) |
| OB-3 | `BLUEPRINT-*-2026-07-*.md`, `BLUEPRINT-ITEM-*`, `BLUEPRINT-W13..W22`, `BLUEPRINT-P97/P98/P102`, `BLUEPRINT-NATIVE-SPA-SERVER-*`, `BLUEPRINT-P-NATIVE-PGRUST-*`, `OPUS-*` research (07-18/19) | same; native-spa-server deleted b1865597 |
| OB-4 | DECISIONS.md D0-D16 open sub-questions (July) | they rule on the July design; no live follow-up names them |
| OB-5 | DG20, DG25, DG26, DW9-SIMD | withdrawn by operator C-1 (7cdcf0ab) |
| OB-6 | FREE-TIER FT3/4/5/6/7/8/9/11/13 as rows | superseded by A1-A9 / BN2 / BN3 / BN8 (see §4) |
| OB-7 | lane cards W-WIRE, W-RU, W-KITCHEN, W-TG, W-INV, W-QA, W-UX, W-APPLE, W-AUDIT, W-KACCESS, W-PERF (P1/P2), W-VIDEO (as written), L62-L71 | all merged (c3f23373, a3cc8f86, 2d652dfc, 0801fb8d, 93396cbf, 2c904e8a, 37b4bf94, cde39b66, 965fbff4, c9ccbfad, b6f42cbd, 35044138; L62-L71 see §1.2) |
| OB-8 | PP-P0 Workers Paid | operator: FREE |
| OB-9 | RU-B4 Russian film cuts | operator: one English version |
| OB-10 | openspec/ (one archived change), specs/ (README + client-checkout) | no live items |
| OB-11 | `/root/lanes/audit-report.md` (2026-09-14 bebop audit) | its 35 OPEN T-rows are the LE-18 list; the rest re-verified in ROADMAP.md 09-14 |
| OB-12 | memory "still open" entries older than their fix: order-fsm (fixed 8e188016), lean blockers (stale per the file itself), bebop-andand (A26 landed) | the files say so |
| OB-13 | dowiz-core July modules (LE-25) | pending the operator's word |

---

## 2. Items needing an operator decision

| # | item | options |
|---|---|---|
| 1 | **R3 CI deploy** — the Cloudflare token as a GitHub repo secret | (a) add the secret → lane starts; (b) keep deploying from the box (uplink 11-16 KB/s, 88eec18f retries) |
| 2 | **A10 zone rate-limit** on `/api/*` per IP (Free: one rule) | (a) operator sets it in the dashboard; (b) a zone-rules token for the box; (c) skip |
| 3 | **F16 seal on** — generate the seal keypair off-platform, set `BACKUP_SEAL_PK` | (a) now (approved 09-26); (b) after F15 X-Wing exactness |
| 4 | **F17 / PQ wording** — README says nothing; memory forbids public PQ claims | (a) say only "sealed off-site copies (hybrid KEM + AES-GCM)"; (b) say nothing until F15 is KAT-gated; (c) full X-Wing sentence |
| 5 | **wasmtime RUSTSEC-2026-0316** ignore expires 2026-10-14 → deps-audit refuses run-all → every deploy | (a) bump wasmtime before 10-14; (b) extend the ignore with a date; (c) drop the wasmtime feature (W-TELEM's fuel lab uses it) |
| 6 | **`scheduled_for_ms`** (order for later) is silently dropped by `carry.rs` | (a) build the feature (kitchen board + ETA honour it); (b) remove the field from the checkout UI |
| 7 | **Wave P legal rows** P4, P5, P10, P12, P13 — authorised without a lawyer 09-24; DPIA approved 09-26; none queued | (a) queue after BN1-BN5 (≈5 lane-days); (b) before BN (legal date Jan 2027 for Art. 31) |
| 8 | **Archive the July corpus** (OB-1..OB-3, ~70 files) into `docs/archive/2026-07/` | (a) move in one commit (paths gate must stay 0); (b) leave and mark each header SUPERSEDED |
| 9 | **W-REAP-CORE** — delete the 13 unreferenced July modules in `crates/dowiz-core` (≈25 k lines, verify by `cargo build` after removal) | (a) a Fable lane with a build-and-test proof; (b) keep as a library |
| 10 | **bebop design-bound T-rows** (T61 pool library, T68-T70 QTT/contracts/effects, T73 snapshot/rollback, T76 living memory, T49/T50, T56, T59, T85→T86) | (a) park formally under HISTORY `## PARKED`; (b) pick one per wave |
| 11 | **bpref oracle** speaks the pre-A26 language (LE-14) | (a) fix the oracle (gates outrank rows); (b) accept the divergence and delete the `no_andand` gate |
| 12 | **Fiscal SEND_ENABLED** stays false (import-only) | unchanged unless the operator says arm; D34 (tips) must land first |
| 13 | **Legacy crates with unresolvable advisories** (mesh-adapter; rustls in `kernel`) | (a) delete the crates; (b) keep the ignores with dates |
| 14 | **Model routing**: "усі нові лейни лише Fable" (2026-10-02) vs the queue's Haiku/Opus labels | confirm Fable for all §3 rows |

---

## 3. Proposed order for OPEN-NOT-QUEUED items, after the approved queue

Three file-disjoint lanes per wave; every row RED-first; owned files named so lanes do not collide. Nothing here pre-empts BN1-BN8 / refunds / ПРРО / typed / R3-R6 / DW1-6, which stay first in the order the operator gave.

**Wave N0 — cheap, zero-risk hygiene (fits into gaps between BN lanes; one Fable lane, 1 day total)**
- N0.1 `FLOWS_BLOCKING=1` default — owns `tools/deploy/deploy.sh` (LE-2).
- N0.2 Cargo `block = []` feature for bebop-wasm (DG9 hand-back) — owns `crates/bebop-wasm/Cargo.toml`, `gate.sh`.
- N0.3 `coverage.sh` as its own CI job — owns `.github/workflows/ci.yml` (LE-7).
- N0.4 bpref oracle `&&`/`||` (LE-14) — owns `bebop-lang/tools/bpref.py` + its parity gate.
- N0.5 qa-durres litter cleanup + probe re-run (LE-5, LE-3 probe half) — owns nothing in the tree.

**Wave N1 (3 lanes, disjoint)**
| lane | rows | owns |
|---|---|---|
| N1-A "splits" | F10 hubdo split (+BS-5), then F8 hubstore, F11 owner, F13 storefront — **sequence after BN1A/BN1B merge** since they edit storefront.rs/hubdo | `workers/api/src/{hubdo,hubstore,owner,storefront}.rs` → `*/` dirs; `tools/gates/file-size.baseline` |
| N1-B "money holes" | D13 wallet guard, O7 `rate_ppm` band, O1 late Stripe paid, D34 tips in fiscal lines, LE-23 float-money 6→0 | `workers/api/src/command/pay/**`, `crates/dowiz-hub/src/room/pay/fx.rs`, `workers/api/src/stripe.rs`, `crates/dowiz-hub/src/ebills_body.rs` |
| N1-C "channels" | D21 WhatsApp `statuses`, TG-A5 inline actions, TG-A6 pacing/mute, F5 invite delivered | `workers/api/src/{meta,telegram}*.rs`, `workers/api/src/outbox/**`, `workers/api/src/services/courier/hiring.rs` |

**Wave N2 (3 lanes, disjoint)**
| lane | rows | owns |
|---|---|---|
| N2-A "privacy law" | P4 request intake + clock, P5 export/restriction, P7 staff/courier rights, P10 ROPA, P12 breach runbook, P13 DPIA, P11 minimisation | `workers/api/src/privacy/**`, `docs/privacy/**`, `workers/api/public/admin/privacy*.js` |
| N2-B "MCP depth" | P15 second protocol generation, P16 catalogue gate, P17 prompts/resources, P18 two-phase dangerous, P20 grants list, P19 e2e per client | `workers/api/src/mcp*.rs`, `workers/api/public/admin/mcp.js`, `tools/gates/mcp-*.sh`, `e2e/mcp/**` |
| N2-C "kitchen 2" | K9 yield proposal, K13 PIN, K14 prep list, K15 READY toast, ST-A3 variance valued, ST-A9 reorder, ST-P3 set_clock | `workers/api/src/services/operations/stock*.rs`, `workers/api/public/kitchen/**`, `workers/api/public/room/app.js`, `workers/api/src/hubdo/{room,ebills}.rs` |

**Wave N3 (3 lanes, disjoint)**
| lane | rows | owns |
|---|---|---|
| N3-A "PQ real" | F15 X-Wing exact + vectors, F16/F17 after the decisions, SEL4-4 sign the chain | `crates/dowiz-core/src/pq/**`, `workers/api/src/cloud.rs`, README |
| N3-B "worker surface" | SEL4-1 write manifest, SEL4-2 secrets out of storefront module, D6 dead_code, F21 capacity test, BS-4/BS-8 size probes | `workers/api/src/{lib,storefront,platform_store}.rs` (after N1-A), `workers/api/Cargo.toml` |
| N3-C "bebop open rows" | DG18/DG23 measurements (if Wave M still unassigned), A19 1b, A25 step 2, B7 explain SIGSEGV, B6 timing arm, A18 lint | `bebop-lang/**` only |

**Wave N4 — after the operator's §2 answers:** P14 OAuth 2.1 (L), W-REAP-CORE (M), archive move (S), scheduled orders (S/M), T-rows.

---

## 4. Duplicates merged (same work under different names)

| canonical | also known as |
|---|---|
| BN1 (A5, FT1a, "dataflow 38→0", W-DF continuation, quality-queue row 6 item 2) | BN1A/BN1B lanes |
| BN2 (A1, C3, FT3, FT9, binary-size "static shell") | "storefront read path to R2", "publish-time prerender" |
| BN3 (B IndexedDB, FT11, DW1 first half, hub-cost phase 7) | "device replica" |
| BN4 (arch-evo P1, A7, hub-cost "one turn", one-image 3→0) | "command surface on HubImages" |
| BN5 (binary-size row 7, "wasm diet") | — |
| BN8 (FT13 instruments, DW6/W-TELEM counters) | — |
| A2+A3+A4+A6 (FT7, FT6, FT8, K11, PP-P3, PP-P4) | "socket-first polling", "auto-pong", "registry cache" |
| A8 (FT4, FT5) | "argon2/nightly in the object" |
| R6 (F20 soak, evals §9 nightly soak) | "load test" |
| F10 (binary-size BS-5 "split HubImages::fetch") | — |
| DG10 Worker wiring (DW8, P2's `redacted = declared` law, "seal at place-order") | — |
| DW6 (W-TELEM, "telemetry service") | — |
| DG19 (DW9 first half) | "fold wall on the QA hub" |
| REFUND (quality-queue row 5, FA-6, order-fsm memory, W-REFUND) | — |
| F8/F9/F10/F11/F12/F13/F14 (quality-queue row 4 "36 files over 300", file-size ratchet LE-22) | W-SPLIT did F9 |
| D6 dead_code (ROADMAP D6, L67 card) | — |
| K11 = A3; PP-P1 = BN1; PP-P2 = F18 (done); BS-6 = DAG R2/R5 (done) | — |
| TG-A4 digests = `digest_rail.rs` (done) | — |
| Wave M (DG18/19/23) = quality-queue row 6 last item | — |

---

## 5. Sources covered / not readable

Read: all files named in the brief except where noted. `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` does not exist (inlined into `docs/design/ROADMAP.md` on 2026-07-20 per CORE-ROADMAP-INDEX). `docs/research/2026-09-27-dag-architecture.md` and `2026-09-28-bebop-dag.md` have no id-tagged row tables; their recommendations are the DG/DW rows and were taken from there. `bebop-lang/docs/BUG-LEDGER-WEEK.md` has no OPEN markers. Live `/api/version` could not be read from the box today (DNS); the last read (w-fresearch, 2026-10-01) was 404. The three parallel reader agents were refused by the session's classifier (transient), so every extraction above was done by grep from this lane; coarse probes are marked **verify**. Counts in §0 are from this table and are exact for the rows listed, estimates for the OBSOLETE file groups.
