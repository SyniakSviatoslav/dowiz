# R-BEBOP-LEVERS — чотири «важелі на порядок величини» для bebop: дослідження, архітектура, специфікації, лейн-картки

Дата: 2026-10-27. Base: `main 1d530e75` (2026-10-21). Автор: Fable-лейн R-BEBOP-LEVERS (дослідження, без змін коду, без git-write).
Фаза 0 (фундамент, НЕ дублюється тут): `R-EMBED-STORE-2026-10-27.md` (bebop-store як вбудована бібліотека: поділ на крейти `bebop-image/-kv/-log/-io/-ffi`, `Db`/`ReadTxn`/`WriteTxn` API, протокол коміту, відновлення) і `R-STORE-VS-SQL-2026-10-27.md` (bebop-store проти SQLite, числа). Усі чотири важелі нижче стоять **на** фазі 0 і посилаються на неї як на «P0».

Позначки: **MEASURED** = число з коду/гейту/звіту з командою; **FRESH** = виміряно сьогодні; **ESTIMATE** = оцінка з перших принципів (формула подана); **UNVERIFIED** = не перевірено.

---

## 0. Резюме для оператора (12 рядків)

1. **Рамку «чотири важелі» слід виправити в одному місці:** важіль 2 (local-first) у dowiz **уже наполовину збудований і не змерджений** — S0 (glue), S1 (escrow + `Stock.lean`), S2 (кухонний планшет офлайн: `tablet.wasm`, `glue_turn.rs`, UI «потрібне рішення») лежать готові у `/root/lanes/w-sheaf3` (порт на main 207cafec, усі гейти зелені за нотатками лейну), а живий QA-хаб сьогодні відповідає `sheaf/stock: "needs S0 glue (not built)"` (FRESH §1.3). Перший крок — не нове дослідження, а **merge SHEAF3** (R-GAPS рядок H, затверджений 10-10).
2. **Важіль 1 (інкрементальність) — головний і справді на порядок:** бази виміряні — `fold_step` 254× на 5 400 подій (0,05 ms проти 5,75 ms), Datalog-дельта 40× на 1× і 2 260× на 100× (ROADMAP DG5/DG8). Прогалина — не алгоритм, а **одна валюта дельт**: сьогодні три інкрементальних механізми (`fold/projection.rs`, `proj.rs` kind 1, `dl.bp`) з трьома правилами інвалідації, KV-memo (kind 2) перераховується повністю, видалення в Datalog — O(stratum), і жодної теореми «інкремент = перерахунок» поза `seminaive_eq_naive`.
3. **Z-set (мультимножина з вагами в ℤ) — валюта для всіх чотирьох:** KV-дельта `put/remove` (W-DELTA) уже є Z-set-дельтою (+1 новий, −1 старий); хвіст логу — Z-set із вагами +1; `glue` — інтеграл `base + ΔA + ΔB` з перевіркою інваріанта (`Stock.lean fold_glue` — це вже доведено); сертифікат кроку — `(OUT_old, Δ, OUT_new)`; колонковий блок — матеріалізований Z-set. Нового формату на диску не треба.
4. **Sheaf-рамка чесно:** склейка локальних секцій = перевірка рівності обмежень на перетинах = «інкрементальна консистентність» у Z-set-алгебрі (Δ на перетині = 0, інакше — обструкція = порушений інваріант). Вона дає **словник і структуру накриття** (контексти, перетини, cover manifest), але **не дає нової алгебри** понад DBSP + інваріанти; когомологія нічого не додає (S7 E4, MEASURED 0/1000). Тримаємо sheaf як іменування синку (lever 2), не як двигун обчислень.
5. **Важіль 3 (proof-carrying data):** SNARK/STARK — overkill (довірена сторона — сам DO венуї; ціни публічні; prover — секунди-хвилини). Правильна форма — certifying algorithms + Merkle-корінь + підпис ML-DSA-65 (`pq/dsa.rs` справжній, FIPS 204 KAT). Виграш — не швидкість, а довіра: чек з доказом ціни (X4 MEASURED 2,4 µs перевірка, ≤ 256 B), закриття дня стоку з witness-ом, фіскальний ланцюжок у QR. 2–4 лейн-дні.
6. **Важіль 4 (колонкова аналітика) — найслабший на цьому масштабі:** 165 страв × ~30 замовлень/день; будь-який «векторизований двигун» тут міряє свій оверхед. Варта **одна** річ: денний фактовий блок продажів (`sales.dwb`, DG7-схема) замість JSON-рядків куба + 4 скалярних ядра (sum-by, top-k, mask-filter, CSR-join) + Z-set-хвіст гарячого логу. 2–3 дні, рік історії ≈ 0,7–1,3 MB, прохід ≈ 0,1 ms (ESTIMATE). Загальний query engine — **не будувати**.
7. **Де живе кожен важіль:** request path dowiz — Rust (`bebop-store`, `dowiz-hub`, Worker, `bebop-wasm`); bebop-мова — двійник-читач (четверо читачів, `gate.sh`), Datalog-ваги (`dl.bp`) і Lean. Жоден важіль не кладе `.bp`-код на живий шлях запиту (рішення AGAINST стоїть).
8. **Ранжування (цінність × здійсненність / ціна):** (1) local-first = merge SHEAF3 + S2b (кухонний rail замовлень офлайн); (2) інкрементальність = `ZSet` + PROJ kind 3/4 + `ZSet.lean`; (3) PCD = S5 корінь + доказ на чеку; (4) колонкова — лише фактовий блок, решту відкласти.
9. **Перші три лейни (файлово роз'єднані, ≤ 3 на боксі):** LV-INC-A (`bebop-store` zset + proj kind 3/4 + 3 двійники), LV-LOC-B (S2b: кухонний rail через той самий `glue`), LV-PCD-C (опублікований корінь + доказ ціни на чеку) — після того, як main змерджить SHEAF3.
10. **Lean-поставки:** `ZSet.lean` (інкремент = перерахунок для лінійних і білінійних операторів, крок memo), `Merkle.lean` (перевірка шляху = перерахунок кореня; soundness witness-а fold-у), `Stock.lean` уже є (fold_glue, glue_comm, escrow).
11. **Де важелі НЕ допомагають:** атом запису (1 placement = 1 DO-turn + 1 row), лічені запити, латентність < 1 RTT для нового, 10 ms-kill (0 з 09-29). Кількісно це все вже сказано в `2026-10-01-fundamental-bottlenecks` §8.3 — повторюю, щоб не обіцяти 1000× там, де його фізично нема.
12. **Питання оператору — 7** (§10), кожне з рекомендацією; найважливіше: Q1 merge SHEAF3 зараз (так), Q4 ключ підпису венуї в DO (так, один ключ, ніколи не ротувати — правило 10-20).

---

## 1. Метод, джерела, що перевірено

### 1.1 Коміти, перевірені `git -C /root/dowiz log`/`show --stat` (усі на main)

| sha | дата | що |
|---|---|---|
| `8300b51b` | 2026-10-01 | DG8: Datalog rule layer (`dl.bp`, `gen_dl.bp`, `Datalog.lean`, 5 правил, оракули `dl_*.py`) |
| `01783eb4` | 2026-10-01 | DG6: L3 scheduler, `Dag.lean` (confluence, memo_sound) |
| `64d2d5cd` | 2026-09-29 | DG5: PROJ/PROJTAB, `fold_step` (`crates/bebop-store/src/proj.rs`) |
| `c496c2d9` | 2026-09-29 | DG7: колонковий каталог (`crates/dowiz-hub/src/block/*`) |
| `9361706b` | 2026-09-30 | DG9: codec round-trip gate, 4 читачі (`block.bp`, `bebop-wasm/src/block.rs`, `oracle.py --block`) |
| `9f1405d2` | 2026-10-07 | W-DELTA: KV v3 delta chain (`kv/delta.rs`, `kv/zc/overlay.rs`, `kv.bp kv3_fold`) |
| `72a5de19` | 2026-10-09 | W-DELTALEAN: `KvDelta.lean` (replay = compact, для будь-якого ланцюжка) |
| `f8d26eae` / `b11f92dd` / `aceda918` | 2026-10-06 | W-ZC (zero-copy `KvIn`, product 1 229 → 1,5 µs), W-OCHAIN (history 3 155 → 29 µs), W-TASTE (`taste.dwb` top-5 6,9 µs vs 638 µs) |
| `6ccca6ef` | 2026-10-08 | W-AX0: лічильники в DO (`hubdo/counters.rs`) |
| `ba18f58d` / `4ebe05e6` | 2026-10-06 | S7a/b/c (consistency radius, harmonic Suggest, migration-law gate); R-S7 звіт |

**НЕ на main (перевірено `grep`/`ls`):** `dl_del.bp` (AX1), `GENTAB` (AX2), `fold/analytics_view.rs` (AX5), PROJ `KIND_BLOB = 3` (AX4) — жодного; з AX-плану (`2026-10-02-ax-implementation-plan.md` §6) збудовано лише AX0 і AX3 (`fold/menu/out.rs`, early cutoff). `glue*` під `crates/dowiz-hub/src/` — відсутній на main, присутній у `/root/lanes/w-sheaf3` і `/root/lanes/w-s2`.

### 1.2 Що є на main по важелях (file:line)

- **DAG/memo:** `bebop-lang/formal/Bebop/Dag.lean:184 confluence`, `:190 memo_sound`, `:198 step_extends` (S-2 shape: fold старого + нові записи = fold конкатенації). `crates/bebop-store/src/proj.rs:1-20` — PROJ kind 1 (log fold, **розширюється одним `log_step` на запис**), kind 2 (KV snapshot, **перераховується повністю на кожен put**, S-3 у `SPEC-BEBOP-DAG-RUNTIME §6.2`). Викликів `bebop_store::proj` у `dowiz-hub`/`workers/api` поза тестами — **0** (grep): memo DO живе окремо.
- **Datalog:** `Datalog.lean:117 seminaive_eq_naive`, `:138 seminaive_lfp`, `:225 stratified_well_defined`. `selfhost/std/dl.bp:1-39` — правила як дані, dirty set, і рядок 38-39: «Relations live in memory, not yet as store projection nodes». Видалення: `dl_eval.bp` `dl_recompute_s` «correct, not fast» (AX-план A1).
- **KV-дельти:** `crates/bebop-store/src/kv/delta.rs:1-45` — v3 root `{n, KIDX, KBLOB, VIDX, VBLOB, VERSION=3, ref DELTA, D}`, запис `{NEXT, OP 1|2, KLEN, VLEN, key, value}`, `MAX_DELTAS = 32`, `DEAD_DIV = 4`; `KvDelta.lean:274 replay_compact`, `:294 compact_prefix`, `:298 compact_append_comm`, `:305 newest_put_wins`.
- **Інкрементальність у Worker (ручна):** `workers/api/src/fold/projection.rs:1-30` — проєкція замовлень застосовує рівно додані події (`Written::Appended|Tail|Log` з дайджестом; `Whole` скидає); `fold/menu/out.rs:1-22` — AX3 early cutoff за output-generation; `hubdo/deltas.rs:14 RECENT_KEEP = 256` — вікно `/fold/changes?since=` (поза вікном — `None`, «читай список»); `hubdo/cube.rs:1-30` + `services/analytics/cube.rs:1-30` — денний куб продажів = «кеш із верифікатором» (trace перераховує день з архівів і порівнює байт-у-байт); `hubdo/edges.rs:1-20` — таблиця ребер усіх `/fold/*` (law 8: кожен memo має перевірку).
- **Local-first на main:** `workers/api/public/lib/blocks.js:1-25` — меню на пристрої (IndexedDB, content-addressed `<k64>.json/.dwb` + `manifest.json`, sha256 перевіряється на кожному читанні, **ніколи замовлення** — AX7 «shred before replicate»); `hubdo/publish.rs:1-25` — публікація на R2 лише зміненого; `public/sw.js:15,110-133` — shell + фото; `public/store/taste-device.js:1-12` — профіль смаку на пристрої, ON за замовчуванням, один тап «вимкнути». `crates/bebop-wasm` — читач формату на wasm32 (`lib.rs:1-40`; `block_view` під `bw_block`; `proj` → `bw_proj`; `decide` feature = FSM кімнати).
- **PCD на main:** `crates/dowiz-hub/src/chain.rs:19 chain_check` (`id = sha256(prev ‖ payload)` — автентифікований append-only лог), crc кожного об'єкта (`bebop-store/src/verify.rs`), `verify/since.rs ChainMark`, node keys K64/K256 (`nodekey.rs`), `pq/dsa.rs` ML-DSA-65 (FIPS 204, ACVP KAT), `pq/backup_seal.rs` (X25519+ML-KEM-768, AES-GCM); `pq/hybrid_signing.rs` **витікає ключ** (memory `pq-hybrid-signing-leaks-key`, карантин `cfg(never)` затверджено). `bebop-lang/docs/CERTIFICATE-FORMAT.md` — сертифікати **резолюції** для VC компілятора (F6), не для даних; переноситься принцип «hints make checking a loop».
- **Колонки на main:** `crates/dowiz-hub/src/block/schema.rs:24-41` — 4 схеми (`menu_prices`, `bom`, `stock_levels`, `names`), типи `i64/i32/u8/u32off/u32rp/u32/CSR val`; `block/view.rs:151-182 short_scalar/short_mask` — єдині «ядра», скалярні (C-1); `taste.dwb` SoA + скорер (W-TASTE). Аналітика: `services/analytics/{cube,history,week_top,forecast,kitchen}.rs` — усе над JSON-рядками.

### 1.3 Живі числа (FRESH 2026-10-27, `GET /api/owner/health` на `qa-durres.dowiz.org`, один запит, read-only)

| образ | generation | usedCells | байти (×8) | примітка |
|---|---|---|---|---|
| `log` | 1993 | 103 835 | 830 680 | `events 1991`; `grows: true`, 798 ‰ |
| `stock` | 1747 | 70 548 | 564 384 | 542 ‰ |
| `catalog` | 2 | 658 | 5 264 | QA-каталог малий; 165-стравовий — sushi-durres |

`counters`: `wakes_total 1`, `cold_folds 1`, `since_total 0`, `proj_rows 0`, `clock: platform-ms-advances-on-io-only` (µs на платформі ≈ 0 за побудовою — `counters.rs:22-25`); `sheaf/stock: "needs S0 glue (not built)"`; `rebuild.stale []`, `quarantined []`.
Висновок: холодний wake складає 1 991 подію з 830 KB **на кожне пробудження**; за R §4 (5,75 ms на 5 400 подій, MEASURED natively) це ≈ 2 ms ESTIMATE (лінійно) — саме те, що AX4/PROJ kind 3 прибирає.

### 1.4 Де живе кожен важіль — мова bebop, Rust-крейти чи обидва

Факт: скомпільований `.bp` **не** на живому шляху dowiz; лише формат образу (Rust) і його читачі (Rust, wasm32, python, bebop — «четверо читачів»). Отже:

| Важіль | Rust (`bebop-store`/`dowiz-hub`/Worker/`bebop-wasm`) | bebop-мова (`selfhost/*`) | Lean | Чому так |
|---|---|---|---|---|
| 1 інкрементальність | **так, основне**: `zset.rs`, PROJ kind 3/4, `fold/projection.rs` на PROJ, аналітичний Z-set | двійник-читач memo (`prelude/proj.bp`), Datalog-ваги (`dl.bp`/`dl_del.bp`, AX1/AX14) | `ZSet.lean`, `Datalog.lean counting_sound` | гаряча частина — DO; Datalog уже живе в bebop і має оракули; четверо читачів — гейт формату |
| 2 local-first | **так**: `glue/*`, `stock/rights.rs`, `bebop-wasm --features tablet`, JS | лише читач `cover manifest`-kind у `kv.bp` (гейт 4/4) | `Stock.lean` (є), `Sync.lean` (rebase детермінізм) | пристрій = wasm32; bebop не має wasm-емітера (рішення DG26 WITHDRAWN) |
| 3 PCD | **так**: `merkle.rs` у `bebop-store`, `bw_verify` у `bebop-wasm`, підпис у DO | `tcheck`-подібний перевіряльник `.bp` як 4-й читач доказу (гейт, не продукт) | `Merkle.lean` | перевіряльник має бути малим і мати ≥ 2 незалежні реалізації (принцип CERTIFICATE-FORMAT §1) |
| 4 колонки | **так**: `block/kernels.rs`, `sales.dwb`, аналітика читає блоки | `block.bp` двійник ядер (`bk_sum_by`) для round-trip/parity | — (одна лема «kernel = fold», опційно) | DG7/DG9 уже так влаштовані; оператор C-1: скаляр, без SIMD |

---

## 2. Важіль 1 — Інкрементальні обчислення (головний)

### 2.1 Prior art

- **DBSP** — Budiu, Chajed, McSherry, Ryzhyk, Tannen, *DBSP: Automatic Incremental View Maintenance for Rich Query Languages*, PVLDB 16(7):1601-1614, 2023, VLDB 2023 Best Research Paper ([pdf](https://www.vldb.org/pvldb/vol16/p1601-budiu.pdf), [arXiv 2203.16684](https://arxiv.org/abs/2203.16684)). Ядро: потоки над комутативною групою (Z-sets = функції скінченної підтримки `α → ℤ`), оператори `D` (диференціювання), `I` (інтегрування), `z⁻¹`; інкрементальна версія запиту `Q^Δ = D ∘ Q ∘ I`; для **лінійного** `Q`: `Q^Δ = Q`; для **білінійного** (join): `(a ⋈ b)^Δ = Δa ⋈ b + a ⋈ Δb + Δa ⋈ Δb` (з інтегралами); `distinct`/негація — через ваги (ефективно «counting»); рекурсія — вкладені потоки. **Формалізація:** [tchajed/dbsp-theory](https://github.com/tchajed/dbsp-theory) — Lean: `incremental.lean` (еквaційна теорія інкременталізації), `circuits.lean` (загальний алгоритм інкременталізації будь-якого контуру), `relational.lean` (реляційні оператори = множинна семантика), `recursive.lean`. Продукт — [Feldera](https://feldera.com/blog/Best-Research-Paper-VLDB-2023).
- **Differential dataflow** — McSherry, Murray, Isaacs, Isard, CIDR 2013 ([MSR](https://www.microsoft.com/en-us/research/publication/differential-dataflow/)); колекції різниць за частково впорядкованим часом, вкладена ітерація; Rust-реалізація — основа Materialize.
- **IVM у Datalog** — Gupta, Mumick, Subrahmanian, *Maintaining views incrementally*, SIGMOD 1993 ([doi 10.1145/170035.170066](https://www.springerprofessional.de/doi/10.1145/170036.170066)): **Counting** (лічильник дериватцій, нерекурсивні) і **DRed** (over-delete + rederive, рекурсивні); Hu, Motik, Horrocks, *Optimised Maintenance of Datalog Materialisations*, AAAI 2018 ([arXiv 1711.03987](https://arxiv.org/pdf/1711.03987)) — гібриди Counting+DRed/B-F «іноді на порядки швидше». Семі-наївне обчислення — уже доведене в `Datalog.lean`.
- **Demand-driven** — Hammer, Khoo, Hicks, Foster, *Adapton*, PLDI 2014 ([pdf](https://people.mpi-sws.org/~hammer/adapton-pldi2014.pdf)); Salsa (rust-analyzer, [docs.rs](https://docs.rs/rust-analyzer-salsa/^0.17.0-pre.4)) — ревізії, memo залежностей, «early cutoff» за рівністю виходу. **У dowiz це вже є**: `fold/menu/out.rs` (AX3) — саме Salsa-shaped cutoff; `Dag.lean memo_sound` — його теорема.
- **Zero (Rocicorp)** — IVM (ZQL) і на клієнті, і на сервері, запити кешуються локально ([llms.txt](https://zero.rocicorp.dev/llms.txt)) — підтверджує, що «дельта як валюта» є і валютою синку (важіль 2).

### 2.2 Що є в коді (докладно)

Три інкрементальні механізми, **кожен зі своїм правилом інвалідації**:

| Механізм | Де | Дельта | Інвалідація | Теорема |
|---|---|---|---|---|
| PROJ kind 1 (log fold) | `bebop-store/src/proj.rs` | записи після `tip` | `input_gen`+`tip` у ланцюжку; інакше refold | `Dag.lean step_extends`; тест `step_equals_full` |
| PROJ kind 2 (KV snapshot) | там само | **немає** (S-3: FNV над відсортованими ключами не splice-able) | генерація → refold whole | — |
| Orders projection | `workers/api/src/fold/projection.rs` | `Written::Appended/Tail/Log` + дайджест | `Whole` скидає; `Log` — дайджест історії | тільки тести |
| Menu memo | `fold/menu.rs` + `menu/out.rs` | нема дельти; early cutoff за out-bytes | `Gens` входів | — |
| Datalog | `selfhost/std/dl.bp`, `dl_eval.bp` | dirty set (вставки); видалення — `dl_recompute_s` O(stratum) | версії DESC | `seminaive_eq_naive` |
| Куб продажів | `services/analytics/cube.rs` | день при ротації | trace = refold + compare | — |
| KV delta chain | `kv/delta.rs` | OP put/remove | `MAX_DELTAS 32`, `DEAD_DIV 4` → compact | `replay_compact` |

Виміряні бази (MEASURED, ROADMAP DG5/DG8, R §4/§12.3): `fold_step` ≤ 0,05 ms проти 5,75 ms на 5 400 подій (254×; 1 274× на 54 000); `dl_event_ns` 970 (bebop) / 82 (Rust) проти 3 316 naive (40× на 1×, 2 260× на 100×); `orders_state` 120 776 µs на 32k подій до `projection.rs`.

### 2.3 Прогалина (те, чого бракує, по пунктах)

1. **Одна валюта.** Три механізми не складаються: KV-дельта (W-DELTA) не доходить ні до Datalog (`dl.bp:38`), ні до menu memo (refold whole), ні до пристрою (`blocks.js` бере цілі `<k64>`-об'єкти — це ок для 8 KB, але це не дельта).
2. **KV kind 2 — без дельти.** Редагування ціни → refold меню + publish-план. AX3 відрізає залежних лише за рівністю байтів виходу.
3. **Datalog-видалення O(stratum)**, і ваги відсутні (AX1/AX14 не збудовані).
4. **Холодний wake** складає весь гарячий лог (FRESH: 1 991 подія, 830 KB) — AX4 (PROJ kind 3) не збудований; `proj.rs` у Worker не використовується.
5. **Теорема «інкремент = перерахунок»** є лише для семі-наївного Datalog і для `log_step` (як `step_extends`); для KV-дельт, menu memo, аналітики — немає.
6. **`/fold/changes` поза 256 подій** віддає `None` (AX2 GENTAB не збудовано) — пристрій читає весь список.

### 2.4 Дизайн: Z-set як одна валюта

**Визначення (Rust, `+crates/bebop-store/src/zset.rs` (новий), нуль залежностей):**
```
ZSet<R>  = відсортований Vec<(R, i64)>  // R = рядок як масив i64-клітинок фіксованої ширини (схема K64), вага ≠ 0
add, neg, is_zero; from_kv_chain(base: &KvIn, chain) -> ZSet<KvRow>  // put k v = +(k,v_new) −(k,v_old) (old з бази), remove = −(k,v_old)
from_log_tail(records after tip) -> ZSet<Rec> (усі ваги +1)
```
Нічого нового на диску: **KV-запис OP уже є Z-set-дельтою** (старе значення читач бере з бази, яка в нього є — `kv/zc/overlay.rs` робить це сьогодні для читання), хвіст логу — дельта з вагами +1, блок DG7 — матеріалізований Z-set (рядок = колонки). Компакція (`compact`) = інтегрування; `replay_compact` — уже доведено, що воно не змінює образ.

**Оператори (у `dowiz-hub`, скалярні):** `map/filter/project` (лінійні: `Q(Δ) = ΔQ`), `sum_by key` (лінійний), `join` по CSR (білінійний: три доданки), `distinct`/`not exists` (нелінійні → через ваги = лічильники, і це *той самий* CNT, що AX1 додає в DESC `dl.bp`). Кожен оператор має `full(zs)` і `step(acc, Δ)`; гейт — `step(full(a), Δ) == full(a + Δ)` на фікстурах і на живих образах (`dagfull`-shaped).

**PROJ kind 3 (BLOB) і kind 4 (ZSET memo)** у `proj.rs` (+ `prelude/proj.bp`, `bebop-wasm/src/proj.rs`, `oracle.py` в **одному** коміті, правило 10): kind 3 — OUT = серіалізований `Orders` (AX4 як спроєктовано), kind 4 — OUT = `ZSet` інтеграл + `input_gen/tip`; крок = `step` над `from_log_tail`/`from_kv_chain`; при `tip` поза ланцюжком — refold (правило S-4 зберігається).

**Datalog:** EDB-дельти = `from_kv_chain` спроєктовані на відношення (`stock(S,Q)`: put → `+stock(S,Q')`, `−stock(S,Q)` — рівно A-7 «retraction by key»), ваги в DESC (AX14), counting для нерекурсивних, DRed-closure для `allergen`/`personal` (AX1). Це закриває `dl.bp:38`.

**Куб/аналітика:** `analytics = Σ day-blocks (lever 4) + ZSet(hot tail)`; `trace` лишається верифікатором (law 8).

**Lean (`formal/Bebop/ZSet.lean`, шаблон — `Datalog.lean semi_invariant` + `KvDelta.lean replay_compact`):**
- `ZSet α := α → Int` (скінченна підтримка через список, як `FSet` без Mathlib); `add`, `neg`, група.
- `Linear Q := ∀ a b, Q (a + b) = Q a + Q b`; `theorem linear_incremental : Linear Q → Q (I Δs) = fold step (Q) Δs`.
- `Bilinear J`; `theorem join_incremental : J(a+Δa, b+Δb) = J a b + J Δa b + J a Δb + J Δa Δb`.
- `theorem memo_step_eq_full : ∀ chain, foldl step (full base) chain = full (base + integrate chain)` — «інкрементальний результат = повний перерахунок» для kind 4; kind 3 — як `step_extends`.
- `theorem kv_chain_is_zset : zset(replay (c ++ [d])) = zset(replay c) + delta_of d` — міст між `KvDelta.lean` і `ZSet.lean`.
- `Datalog.lean counting_sound` (з AX-плану A1): `h ∈ F ↔ CNT h > 0`.

### 2.5 Sheaf-рамка — чи додає щось до DBSP?

Чесна відповідь у три речення. (1) Склейка (`glue.rs`, S0) перевіряє `res_U(s_U) = res_V(s_V)` на перетині й інваріанти над `fold(base) + ΔA + ΔB` — у Z-set-термінах це «Δ на перетині = 0, інтеграл хвостів задовольняє інваріант»; `Stock.lean fold_glue/glue_comm` — це DBSP-лінійність fold-у стоку над адитивними подіями, доведена під іншою назвою. (2) Що sheaf **додає**: явну структуру накриття (контексти, перетини, `cover manifest`, функції обмеження як named objects) — це цінний **словник і гейти** для важеля 2 (які перетини існують, що є обмеженням, що є обструкцією), і операторове рішення 10-05 цим задоволене. (3) Що sheaf **не додає**: нової алгебри обчислень (обструкція для дискретних фактів = множина порушених інваріантів, `H¹` даних не бачить — S7 E4 MEASURED 0/1000; дифузія й гармонічне продовження — без виграшу, E2/E3). Тож: **DBSP/Z-set — алгебра; sheaf — топологія синку**; «gluing is an incremental-consistency check» — так, і саме в цьому сенсі.

### 2.6 Реалістичний виграш (із навантаженням)

| Навантаження | Сьогодні | Після | Множник | Статус числа |
|---|---|---|---|---|
| Холодний wake DO: перша відповідь `/fold/orders` | fold 1 991 подій / 830 KB (FRESH) ≈ 2 ms (ESTIMATE з 5,75 ms/5 400, лінійно) + читання образу | PROJ kind 3: читання memo + крок над записами після tip ≤ 0,5 ms (AX4 mark) | ≈ 4–10× на wake; на **CPU DO за день** ≈ wakes × 2 ms — малий абсолют | AX0 міряє (`cold_fold_us` на платформі ≈ 0 за годинником; міряти натівно) |
| Редагування ціни → menu memo → publish | refold усього меню (165 страв, 3 мови) + план: ESTIMATE 1–3 ms CPU, 3 R2-записи | kind 4 крок над 1 KV-записом: ESTIMATE 10–50 µs; publish незмінний (3 об'єкти) | ≈ 30–100× CPU кроку; **латентність для гостя не змінюється** (manifest `max-age=30`) | ESTIMATE |
| Datalog-видалення (`produced` row, `edge`) | O(stratum) `dl_recompute_s` (не виміряно, AX-план) | O(affected): ≤ 10× `dl_event_ns` (AX1 mark ≤ 10 µs) | ≈ 40× за аналогією до шляху вставок (MEASURED там) | ESTIMATE |
| Аналітика за рік | парсинг JSON рядків куба на кожне читання (`cpu_of_a_year_read`, число в `history/tests.rs`) | Σ блоків + Z-set хвоста (важіль 4) | 10–100× | ESTIMATE |
| Пристрій «що змінилось» | `/fold/changes` поза 256 → весь список | GENTAB (AX2) → завжди дельта | байти: список ≈ 70 KB → дельта ≈ 1 KB | MEASURED базa (menu memo ~70 KB) |

**Де НЕ допомагає:** атом запису (1 placement = 1 DO-turn + ≥ 1 row), лічені запити (платформа рахує запити, не µs), латентність нижче 1 RTT, 10 ms-kills (0 з 09-29), bebop на request path (2,6–10,6× повільніше за Rust — не важіль).

---

## 3. Важіль 2 — Local-first

### 3.1 Prior art

- **Ink & Switch, Kleppmann, Wiggins, van Hardenberg, McGranaghan, *Local-first software*, Onward! 2019** — сім ідеалів: без спінера, не прив'язаний до пристрою, мережа опційна, колаборація, довговічність, приватність, контроль користувача ([огляд](https://electric-sql.com/docs/reference/local-first), [Kleppmann](https://kleppmann.com/2021/06/14/emergencity.html)).
- **Automerge** — JSON-CRDT, multi-value register з `getConflicts` ([talk](https://talks.cam.ac.uk/talk/index/163276)); **Yjs** — CRDT для тексту/масивів; **cr-sqlite (vlcn)** — CRDT-колонки всередині SQLite ([Electric alternatives](https://legacy.electric-sql.com/docs/reference/alternatives)); **ElectricSQL** — sync engine Postgres → локальна БД, «shapes» ([neon](https://neon.com/blog/comparing-local-first-frameworks-and-approaches)); **Replicache / Zero (Rocicorp)** — *server-authoritative mutators + rebase* на клієнті, IVM-запити обома боками ([Zero](https://zero.rocicorp.dev/llms.txt)).
- Вибір моделі для dowiz **уже зроблений і правильний** (R-BEBOPDB G.2): CRDT — лише там, де дані є решіткою (профіль смаку, S4); **операційний лог + серверна авторитетність + rebase** (Zero/Replicache-shaped) для замовлень і стоку; **escrow** (Balegas 2015, bounded counters) для «останнього лосося». LWW за годинником — ніколи (годинникам планшетів не довіряємо).

### 3.2 Що є (і що з цього не змерджено)

На main: меню гостя на пристрої (BN2/BN3, §1.2), профіль смаку на пристрої, `/fold/changes`, `bebop-wasm` читач + `decide` (FSM кімнати в wasm), `bw_menu`/`bw_proj`.
**Збудовано, не змерджено** (`/root/lanes/w-sheaf3`, порт S0+S1+S2 на main `207cafec`; нотатки `.s0/.s1/.s2/.sheaf3-notes.md`; усі RED-мутанти червоніли й відновлені; hub lib 770 passed; `Stock.lean` 246 рядків sorryAx 0; `tablet.wasm` 304 056 B / 110 502 gz):
- `/root/lanes/w-sheaf3/crates/dowiz-hub/src/glue.rs` + `glue/{walk,delta,wire,merge,tablet,desk}.rs` — S0 glue (924 µs MEASURED на 2000+20+20), S2 merge/adopt, desk «потрібне рішення»;
- `/root/lanes/w-sheaf3/crates/dowiz-hub/src/stock/{rights,adopt}.rs` — S1 escrow-права, provisional;
- `/root/lanes/w-sheaf3/crates/bebop-wasm/src/tablet.rs` (`bw_tab_record/tail/rebase/items`, `--features tablet`) — писач планшета;
- `/root/lanes/w-sheaf3/workers/api/src/hubdo/glue_turn.rs` (`/fold/glue`, `/fold/desk`), `services/operations/stock/glue.rs` (контракт `POST /api/staff/stock/glue {location_id, ctx, base, tip, tail}` → `{tip, tail, merged, held, conflicts, grants, replay, wire_up, wire_down}`, 422/409 без запису);
- canvas `board/{desk,stock}.rs`, `public/room/canvas/tablet.js` — UI;
- `/root/lanes/w-sheaf3/bebop-lang/formal/Bebop/Stock.lean` — `fold_glue` (Т2), `glue_comm` (Т1), escrow (Т3).
Живий хаб: `sheaf/stock: "needs S0 glue (not built)"` (FRESH). R-GAPS 10-10 рядок H і операторове «усе дозволяю» = merge затверджено; порядок SHEAF3 → CV → TELEM.

### 3.3 Прогалина

1. **Merge SHEAF3** (головне; 12 ручних конфліктів уже розв'язані в лейні — `ffi.rs` коди 15/16, `field.rs Number = 6`, `F_QTY = 8`, sw.js shell `2026-10-10-sheaf3`; потрібне: повторний `diff3` проти сьогоднішнього main (10-10 → 10-21: 20 комітів, переважно flows/console), rebuild `board.wasm`, `run-all`, flows у браузері).
2. **Офлайн лише для стоку.** Кухонний rail замовлень (kitchen ack, Advance) офлайн — ні; S2b = той самий `glue` над логом замовлень з `decide`-FSM (яка вже є у `bebop-wasm --features decide`).
3. **Гість офлайн** — лише читання (вірно: замовлення = персональні дані, AX7; оплата й placement потребують хабу). Нічого не додавати.
4. **Консоль власника офлайн-редагування** — S3 (поле-рівневий merge, обидва значення при конфлікті того самого поля).
5. **Дельти для пристрою** — GENTAB (AX2) для `since=`; KV-дельти для каталогу на пристрої не потрібні: `<k64>.dwb` 7,9 KB (MEASURED) — дешевше за будь-який протокол дельт.
6. **Ізоляція орендарів:** образ планшета ключується `slug` (хост вирішує, `courier-login-venue-from-host`), `Place::of_authorised` у `glue.rs` контракту; на пристрої — ніяких байтів іншої венуї (тест: два slug-и, два образи, жодного спільного запису). Приватність: у хвості кухні немає профілю гостя (`no-tracking.sh` на хвіст — гейт S2 уже має).

### 3.4 Дизайн: дельта = одиниця синку

- **Стебла:** планшет тримає образ-лог (стік, S2; замовлення, S2b) з `prev = tip_hub` у першому записі; каталог — `<k64>` блоки (є). Усе — той самий формат, той самий `bebop-wasm`.
- **Синк:** `tail` вгору (Z-set з вагами +1), відповідь = хвіст хаба + grants + rebase-рішення; це рівно контракт S2. S2b додає `kind` подій замовлення й `decide` як «write door» планшета (як `tablet.rs` робить для стоку через `dowiz_hub::glue::tablet`).
- **Конфлікти:** таблиця G.2 без змін (escrow → rebase → людина; поле-рівневий merge для каталогу; CRDT-join для смаку).
- **Lean:** `Sync.lean` — детермінізм rebase: `merge(hub, tails in canonical order (at, ctx))` не залежить від порядку надходження при комутуючих дельтах (`glue_comm` уже є), і «`held` ⊆ tail» (жодна подія не губиться: `len == events + held` — закон S2).

### 3.5 Реалістичний виграш

| Навантаження | Сьогодні | Local-first | Множник |
|---|---|---|---|
| Повторний візит гостя, меню | 1 edge GET 140–290 ms HIT (MEASURED, bottlenecks §7.3) → **уже 0 запитів** з BN3 | те саме | **уже зібрано**; залишок — лише `manifest.json` раз на 30 с |
| Кухонний планшет, читання стоку/замовлень | DO round trip 50–200 ms + poll 12 с | локальне читання `bebop-wasm`: ≤ 1 ms incl. crc (ESTIMATE: `Kv::load` 316–713 µs MEASURED на 165 страв; стік-лог 564 KB — ChainMark) | ≈ 50–200× латентність; **доступність при падінні uplink-у** — головна цінність |
| Байти на дроті, кухня/день | весь образ при кожному read (поза memo) | ≤ 100 KB/день (mark), 62 KB MEASURED E5 | ≈ 10× |
| Продажі понад залишок офлайн | можливі | 0 / 10 000 днів (MEASURED E5), −0,65 % прийнятих | інваріант, не швидкість |

**Де НЕ допомагає:** placement/оплата (авторитет — хаб), персональні дані на спільних пристроях (AX7), гість (уже зроблено), P2P між гостями (bottlenecks §7.3 — хибно на кожному пункті, крім безплатного egress).

---

## 4. Важіль 3 — Proof-carrying data / verify замість re-execute

### 4.1 Prior art

- **Certifying algorithms** — McConnell, Mehlhorn, Näher, Schweitzer, *Computer Science Review* 5(2):119-161, 2011 ([anu](https://researchportalplus.anu.edu.au/en/publications/certifying-algorithms/)): вихід + witness, перевіряльник малий і простіший за алгоритм (LEDA). Прямо наш шаблон: `fold` стоку → рівень + witness (список рухів і біжучі суми); перевірка = `k` додавань.
- **Authenticated data structures** — Merkle (1987); Miller, Hicks, Katz, Shi, *Authenticated data structures, generically*, POPL 2014 ([pdf](https://cs.umd.edu/~mwh/papers/gpads.pdf)): будь-яка структура з хешованих вузлів дає докази під collision resistance; **наш ланцюжок `id = sha256(prev ‖ payload)` уже є автентифікованим логом** (`chain.rs`), а доказ включення = шлях від запису до tip (O(n) по ланцюжку) або Merkle-дерево над листками (O(log n), X4 MEASURED: корінь над готовими листками 6,3 µs, перевірка включення 2,4 µs).
- **Verifiable computation (SNARK/STARK)** — не реалістично тут: довірена сторона = DO венуї (сервер власника), а не хмара; нема що приховувати (ціни публічні); prover — секунди-хвилини на кожен чек, circuit-інженерія, trusted setup (Groth16) або великі докази (STARK ~10-100 KB) — проти 256 B Merkle-шляху. **Overkill; не будувати.**
- **Переноситься з `CERTIFICATE-FORMAT.md`:** текст нормативний, бінарне — похідне; перевіряльник — цикл без пошуку (hints); negative gate (`cert_neg/`) — кожна помилка формату має іменований червоний приклад.

### 4.2 Що є / 4.3 Прогалина

Є: ланцюжок sha256, crc, node keys, ML-DSA-65 (справжній), `backup_seal`, `cube trace` (верифікація **повторним виконанням**), law 8 rebuild. Нема: **опублікованого кореня** (S5), підпису венуї, доказу на чеку/у QR, witness-а закриття дня стоку, сертифіката крос-джерельної консистентності (імпорт till e-bills ↔ замовлення; `fiscal/receipt.rs` друкує «pa NIVF» без хешу).

### 4.4 Дизайн

- **Корінь:** на кожну генерацію образу — `root = merkle(leaves)`, листки = `K256` записів/ключів (нічого не змінює у форматі: корінь — PROJ kind 5 «ROOT» або `settings`-ключ; `+crates/bebop-store/src/merkle.rs`, нуль залежностей, `Sha256` уже є у `nodekey.rs`). **Підпис:** ML-DSA-65 ключ венуї в DO (секрет через `wrangler secret`, один, ніколи не ротується — правило 10-20); публічний ключ у `manifest.json` сторфронту.
- **Сертифікати (4 види, кожен ≤ 1 KB):**
  1. *Ціна на чеку* (S5): `(root_catalog_gen, sig, leaf(product:id), path)` → у QR та на сторінці `/verify`; перевіряє `bebop-wasm bw_verify` у браузері (2,4 µs) і `oracle.py` (другий читач).
  2. *Закриття дня стоку* (certifying fold): `(level_i, witness: Σ moves_i за день, tip_open, tip_close)`; перевіряльник додає `k` чисел і звіряє `tip`-и — ніякого sha256 над усім логом.
  3. *Фіскальний документ:* `id` ланцюжка документа + корінь дня у тексті чека (поряд із «pa NIVF»), QR → `/verify?doc=`.
  4. *Інкрементальний крок* (міст до важеля 1): `(OUT_old_digest, Δ, OUT_new_digest)` — клієнт з OUT_old перевіряє крок, не перераховуючи; це те, що `bw_proj` + kind 4 дають майже безплатно.
- **Крос-джерельна консистентність** (till import vs orders): witness = Z-set різниця двох проєкцій (має бути 0) — підписаний «нуль» за день.
- **Lean (`Merkle.lean`):** `verify_path leaf path root = true ↔ root = fold hash path leaf` (коректність, без аксіом); soundness доказу fold-у: `check(witness) = true → level = fold(moves)` (чиста арифметика). Колізійну стійкість у Lean **не** моделюємо аксіомою (гейт `lean.axioms` не приймає нових аксіом) — вона є гіпотезою в **статті**, не в теоремі.

### 4.5 Виграш і де не

Швидкість: перевірка 2,4 µs проти refold 2–6 ms ≈ 1000× — але це не той метрик, що продає. **Цінність** = довіра: власник доводить аудитору ціни/залишки/документи без доступу до DO; клієнт перевіряє чек; `cube trace` стає сертифікатом замість повторного fold-у архівів. **Де не:** шлях замовлення в реальному часі (нічого не додавати), ZK (нема чого ховати), SNARK.

---

## 5. Важіль 4 — Колонкова аналітика

### 5.1 Prior art

Abadi, Madden, Hachem, *Column-stores vs. row-stores: how different are they really?*, SIGMOD 2008 ([pdf](https://users.cs.utah.edu/~pandey/courses/cs6530/fall22/papers/rowvcolumn/abadi-sigmod08.pdf)) — виграш дає **executor** (векторизація, компресія, пізня матеріалізація), не сам layout; Boncz, Zukowski, Nes, *MonetDB/X100*, CIDR 2005 ([pdf](https://cs.brown.edu/courses/cs227/archives/2008/Papers/ColumnStores/MonetDB.pdf), CIDR Test-of-Time 2024) — вектори ~1000 значень, IPC; Raasveldt, Mühleisen, *DuckDB*, SIGMOD 2019 ([duckdb.org](https://duckdb.org/pdf/SIGMOD2019-demo-duckdb.pdf)) — вбудований, векторизований; Abadi, Myers, DeWitt, Madden, *Materialization strategies*, ICDE 2007; Abadi, Madden, Ferreira, *Integrating compression and execution*, SIGMOD 2006. Спільний висновок, що важить тут: **виграш колонок росте з розміром таблиці** (мільйони рядків); на 10³–10⁴ рядках executor-оверхед переважає.

### 5.2 Що є / 5.3 Прогалина

Є (§1.2): 4 схеми DG7, кодек DG9 з 4 читачами, 2 скалярних ядра (`short_scalar/short_mask`), `taste.dwb` + top-5 (6,9 µs MEASURED проти 638 µs JSON), куб продажів з верифікатором. Нема: фактового блоку продажів (куб = JSON-рядки, парсинг на читання), ядер агрегації/групування/top-k над блоками, «плану» як даних, Z-set-хвоста для гарячого дня. Масштаб (FRESH/ESTIMATE): 30 замовлень/день × 3 рядки × 365 = 33 k рядків/рік × 6 колонок × 8 B ≈ 1,6 MB (i32 де доведено — 0,8 MB); 10 венуй-років — 16 MB — усе ще «один блок на день у пам'яті DO».

### 5.4 Дизайн (мінімальний)

- `sales.dwb` схема `sales:v1(day:i32:7,dish:i64:0,qty:i32:0,gross:i64:1,cost:i64:1,order:u32:0)` — пишеться при ротації (там, де сьогодні куб робить catch-up), один блок на день, `K256` у кубі; старі рядки куба читаються як є (law: `trace` порівнює блок із refold дня).
- Ядра `+crates/dowiz-hub/src/block/kernels.rs` (скаляр, C-1): `sum_by(col_key, col_val)`, `topk(col, k)`, `mask_filter(pred)`, `csr_join(dish → category)` через `names`/CSR; усі мають форму `full(blocks)` і `step(acc, zset Δ)` (важіль 1).
- План як дані (як правила A.3): `[filter, group, agg, topk]` у 8-клітинкових рядках; `run_plan` — одна функція; **не** парсер запитів, **не** оптимізатор.
- Двійник `block.bp bk_sum_by` + `oracle.py --kernel` — гейт 4/4 (DG9-shaped).
- UI: наявна сторінка аналітики власника читає блоки; позначка «перевірено проти архівів» (trace).

### 5.5 Виграш і де не

Прохід колонки 33 k рядків ≈ 3 ns/рядок (ESTIMATE з 0,5 µs на 165-рядкову маску і 2,4 µs decode) ≈ 0,1 ms/колонку; проти парсингу JSON-рядків куба — ESTIMATE 10–100×. Але аналітику власник відкриває кілька разів на день: **виграш — не латентність, а місткість і просте «рік у пам'яті»**; ще — food cost у ту саму колонку (`cost`), що просив оператор 10-12. **Де не:** 165 страв (вже 0,5 µs), OLTP, гість, SIMD (DG20/25/26 WITHDRAWN), щільні тензори (C-1).

---

## 6. Архітектура: як чотири важелі лягають на P0

```mermaid
flowchart LR
  subgraph P0[P0: embedded store (R-EMBED-STORE)]
    IMG[bebop-image\ncells, sb A/B, PartTab, crc]
    KV[bebop-kv\nv2 base + v3 delta chain]
    LOG[bebop-log\nEvLog, chain ids, ChainMark]
    IO[bebop-io\nFileDevice / MemDevice]
  end
  subgraph Z[ZSet layer (lever 1)]
    ZS[zset.rs\nfrom_kv_chain / from_log_tail / from_block]
    OPS[ops: map filter sum_by join distinct\nfull + step]
    PROJ[PROJ kind 1,2 (є) · 3 BLOB · 4 ZSET · 5 ROOT]
  end
  subgraph HUB[dowiz-hub + DO (authority)]
    ORD[fold/projection.rs → kind 3]
    MENU[menu memo → kind 4 step]
    DL[Datalog EDB Δ ← kv chain]
    GLUE[glue/merge/desk (S0-S2)]
    KER[block/kernels.rs + sales.dwb (lever 4)]
    MER[merkle.rs + ML-DSA sign (lever 3)]
  end
  subgraph DEV[device: bebop-wasm]
    RD[reader: kv/log/block/proj]
    WR[tablet writer (stock, S2b orders)]
    VF[bw_verify: path + witness]
  end
  P0 --> ZS --> OPS --> PROJ
  PROJ --> ORD & MENU & DL & KER
  ZS -->|tail = Δ (+1)| GLUE
  PROJ -->|OUT digests| MER
  GLUE <-->|POST /api/staff/*/glue  tail ↑ / tail+grants ↓| WR
  MER -->|root, sig, path ≤ 256 B| VF
  RD -->|manifest + <k64>.dwb (є)| DEV
  FOUR[four readers gate: bebop · Rust · wasm32 · python] -.-> PROJ & KER & MER
```

**Базові абстракції (усі над P0, без нового дискового формату):**
`ZSet<R>` (валюта), `DeltaSource` (KV-ланцюжок, хвіст логу, різниця блоків), `Op { full, step }`, `Memo` (PROJ kind 3/4/5 з `input_gen/tip`), `Tail` (одиниця синку = Z-set +1), `Witness` (`Δ` + дайджести OUT, або шлях Merkle), `Plan` (рядки-дані для ядер).

**Що змінюється в dowiz, щоб кожен важіль був видимий в UI (правило `dowiz-no-backend-without-ui`):**
- Health власника: рядок «Incremental»: `proj_hits / proj_steps / refolds`, `cold_fold_us` (натівний), `since_none` → 0 після AX2.
- Консоль: після редагування ціни — «опубліковано за N ms, 3 об'єкти, крок memo» (замість мовчання).
- Кухонний планшет: банер «офлайн — N рухів чекають синку», desk «потрібне рішення» (S2 UI вже є), S2b — те саме для замовлень.
- Чек/сторінка замовлення: QR «перевірити ціну/документ» → `/verify` (працює офлайн у браузері через `bw_verify`).
- Аналітика: «рік історії, перевірено проти архівів ✓», колонка food cost.

---

## 7. Специфікації: критерії прийняття, тести, Lean, бенчмарки

### 7.1 Важіль 1 — `AC-LV-INC-*` / `T-LV-INC-*`

| ID | Критерій (вимірний) | Тест |
|---|---|---|
| AC-LV-INC-01 | `zset::from_kv_chain(base, chain)` для кожної фікстури `kv3.store` і живих образів: `integrate(base ⊕ Δ) == Kv::load(replay)` байт-у-байт | T-LV-INC-01 (`bebop-store/src/zset/tests.rs`), golden `kv/golden_tests.rs` незмінний |
| AC-LV-INC-02 | PROJ kind 3 (BLOB) і kind 4 (ZSET): `step(full(prefix), Δ) == full(prefix ⊕ Δ)` на 10⁴ подій; refold при `tip` поза ланцюжком; redaction скидає memo (S-4) | T-LV-INC-02 (`proj/tests.rs step_equals_full_kind{3,4}`, `redaction_drops_memo_kind4`) |
| AC-LV-INC-03 | Четверо читачів читають kind 3/4/5 memo до одного числа (`gate.sh proj` 4/4 на новій фікстурі `proj4.store`); `oracle.py` і `prelude/proj.bp` у тому самому коміті | T-LV-INC-03 (`crates/bebop-wasm/gate.sh`, `std_golden gate proj`) |
| AC-LV-INC-04 | Worker: `fold/projection.rs` читає/пише kind 3; `cold_fold_us_p50` натівно ≤ 500 µs на 2 000 подій (FRESH база 1 991) ; `proj_rows` на append не росте | T-LV-INC-04 (`workers/api` тест `cold_read_steps_not_refolds`), AX0 health |
| AC-LV-INC-05 | Menu memo як kind 4: редагування однієї ціни = крок над 1 записом; `out_key` незмінний при редагуванні без видимого ефекту (AX3 збережено); publish — 3 об'єкти | T-LV-INC-05 (`fold/menu/tests.rs one_price_edit_is_one_step`) |
| AC-LV-INC-06 | Datalog: ваги в DESC (CNT), counting для нерекурсивних, DRed-closure для `allergen`/`personal`; `dl_event_del_ns ≤ 10 × dl_event_ns`; `dagfull datalog` incremental == scratch після 10⁴ змішаних подій | T-LV-INC-06 (`bench/vs_rust/dagfull.sh datalog`, `dl_allergen_del.py`) |
| AC-LV-INC-07 | `/fold/changes?since=` ніколи не `None` для `since ≥ gen − 10⁴` (GENTAB) | T-LV-INC-07 (`hubdo/deltas` тест + live-proof контракт `feature-since.json`) |
| AC-LV-INC-08 | Lean: `ZSet.lean` — `linear_incremental`, `join_incremental`, `memo_step_eq_full`, `kv_chain_is_zset`; `Datalog.lean counting_sound`; `lake build` rc 0, `sorryAx 0`, `lean.axioms` без нових рядків | T-LV-INC-08 (`tools/gates/lean.sh`) |
| AC-LV-INC-09 | Мутації: (a) `step` ігнорує вагу −1 → T-01 червоний; (b) kind 4 не скидається при redaction → T-02 червоний; (c) CNT не декрементує → `dl_allergen_del` червоний | RED-first, записано в нотатках лейну |

**Бенчмарки (named, `bebop-lang/docs/PERF.md`/`perf.csv` рядки):** `proj3_cold_read_us` ≤ 500 на 2 000 подій (контроль — повний fold, той самий процес); `proj4_price_step_us` ≤ 50 (контроль — refold меню); `dl_event_del_ns` ≤ 10 000; `since_delta_bytes` ≤ 2 KB на 10 подій.

### 7.2 Важіль 2 — `AC-LV-LOC-*`

| ID | Критерій | Тест |
|---|---|---|
| AC-LV-LOC-01 | SHEAF3 на main: hub lib ≥ 770 passed; `run-all.sh` зелений; `canvas-wire` у бюджеті або прийнятий підпис; `sheaf/stock` у health = `glue: built` | T-LV-LOC-01 (`tools/gates/run-all.sh`, `e2e/flows`) |
| AC-LV-LOC-02 | Доба без зв'язку в replay (кухня, 400 рухів): усі злиті, `len == events + held`, конфлікти показані; байти ≤ 100 KB/день | T-LV-LOC-02 (`glue/tests.rs day_offline`, E5-shaped, детермінований seed, без фаззера) |
| AC-LV-LOC-03 | S2b: kitchen ack / Advance офлайн через `decide` на планшеті; після синку FSM хаба приймає або відхиляє іменованою помилкою; жодна подія не губиться | T-LV-LOC-03 (`glue/orders/tests.rs`, `bebop-wasm tests/tablet_orders.rs`) |
| AC-LV-LOC-04 | Ізоляція: два slug-и → два образи, 0 спільних записів; `no-tracking` на хвіст = 0 полів профілю | T-LV-LOC-04 (`tablet.test.mjs two_venues`, `tools/gates/no-tracking.sh`) |
| AC-LV-LOC-05 | Live (правило 10-03): Playwright на `qa-durres` — планшет офлайн (`context.setOffline(true)`), 3 рухи, онлайн, desk показує 0 або іменовані конфлікти; контракт `+tools/live-proof/contracts/feature-glue.json` зі схемою | T-LV-LOC-05 (`+e2e/flows/f12-offline-kitchen.mjs`, `+tools/live-proof/probes/feature-glue.mjs`) |
| AC-LV-LOC-06 | Lean `Sync.lean`: `merge` детермінований за канонічним порядком; `held ⊆ tail`; `Stock.lean` незмінний | T-LV-LOC-06 (`lean.sh`) |

**Бенчмарки:** `glue_2000_20_20_us` ≤ 1 000 (MEASURED 924); `tablet_local_read_us` ≤ 1 000 на 564 KB стік-логу з ChainMark; `wire_bytes_day` ≤ 100 KB.

### 7.3 Важіль 3 — `AC-LV-PCD-*`

| ID | Критерій | Тест |
|---|---|---|
| AC-LV-PCD-01 | `merkle.rs`: корінь над K256 листків; `prove(i)`/`verify(leaf, path, root)`; доказ ≤ 256 B при n ≤ 10⁴; verify ≤ 10 µs (X4 2,4) | T-LV-PCD-01 (`bebop-store/src/merkle/tests.rs`, один іменований зіпсований байт шляху → refuse) |
| AC-LV-PCD-02 | Корінь каталогу на генерацію (PROJ kind 5) + підпис ML-DSA-65 у DO; `manifest.json` несе `root, sig, pk_id`; четверо читачів читають kind 5 | T-LV-PCD-02 (`gate.sh proj`, `publish/tests.rs root_in_manifest`) |
| AC-LV-PCD-03 | Чек/сторінка замовлення: QR → `/verify` перевіряє ціну в браузері (`bw_verify`) без мережі; `oracle.py --verify` — другий читач; 100 % цін фікстури проходять; підмінена ціна — іменована відмова | T-LV-PCD-03 (`public/store/verify.test.mjs`, live-proof `feature-verify.mjs`) |
| AC-LV-PCD-04 | Witness закриття дня стоку: `check(witness) == ledger.level` для 50 голденів `stock/fold/tests.rs`; witness не містить персональних даних (`personal-data` 0) | T-LV-PCD-04 |
| AC-LV-PCD-05 | Фіскальний документ друкує `doc_id` ланцюжка і `root_day` (TAX §3.8 текст доповнено); `fiscal/receipt.rs` golden оновлено свідомо | T-LV-PCD-05 |
| AC-LV-PCD-06 | Lean `Merkle.lean`: `verify_path ↔ root = fold`, `witness_sound`; без нових аксіом | T-LV-PCD-06 |

**Бенчмарки:** `merkle_root_165_us` ≤ 20; `verify_path_us` ≤ 10; `sign_ms_dsa65` (натівно) записати — ML-DSA-65 sign ≈ сотні µs–ms ESTIMATE, один раз на генерацію, не на запит.

### 7.4 Важіль 4 — `AC-LV-COL-*`

| ID | Критерій | Тест |
|---|---|---|
| AC-LV-COL-01 | Схема `sales:v1` у `block/schema.rs` + `block.bp` + `oracle.py`; round-trip `gate.sh blocks` 4/4 на нових фікстурах (порожній, 1 день, 365 днів) | T-LV-COL-01 |
| AC-LV-COL-02 | Ядра `sum_by/topk/mask_filter/csr_join`: `full == step-fold` над Z-set-хвостом; результати == `services/analytics::fold` на 10⁴ подій incl. DST (law 10, `float-money 0`) | T-LV-COL-02 (`block/kernels/tests.rs`, `analytics/tests.rs kernels_equal_fold`) |
| AC-LV-COL-03 | Куб: день при ротації = блок; `trace` порівнює блок із refold дня байт-у-байт; старі JSON-рядки читаються | T-LV-COL-03 |
| AC-LV-COL-04 | Аналітика власника читає блоки; сторінка показує «перевірено проти архівів» і колонку food cost; `ui-reach` без нових сиріт | T-LV-COL-04 (flows F11 розширено) |
| AC-LV-COL-05 | `year_scan_us` ≤ 500 на 33 k рядків (контроль — JSON-куб у тому самому процесі) | T-LV-COL-05 (`history/tests.rs cpu_of_a_year_read` поряд) |

---

## 8. Ранжування

| # | Важіль | Цінність для dowiz | Здійсненність | Ціна (лейн-дні Opus) | Score (V×F/C) | Вердикт |
|---|---|---|---|---|---|---|
| 1 | **Local-first (staff)** | висока: операторова стратегія 10-05, офлайн венуя, escrow доведений | дуже висока: S0–S2 **збудовані**, лишився merge + S2b | merge 2 + S2b 4 = 6 | найвищий | **робити першим** |
| 2 | **Інкрементальність** | висока: cold wake, редагування, Datalog-видалення, одна теорема на все | висока: бази виміряні, формат не змінюється, шаблони Lean є | 4 + 3 + 4 (A, D, F) = 11 | високий | **робити** |
| 3 | **PCD** | середня-висока: довіра/аудит/фіскал; малий код | висока: sha256-ланцюжок, DSA, X4 є | 3 | високий (мала ціна) | **робити** після 1–2 |
| 4 | **Колонкова аналітика** | низька-середня на цьому масштабі; цінність = історія в пам'яті + food cost | висока для фактового блоку; низька для «двигуна» | 3 (блок + 4 ядра); 10+ (двигун) | низький | **звузити**: лише фактовий блок; query engine / векторизація / компресія — **не робити** |

Прямо: «колонкова аналітика як важіль на порядок» для 165 страв × 30 замовлень/день — не важіль; це 0,1 ms проти кількох ms раз на день. Беремо з неї лише те, що складається з важелем 1 (блок = матеріалізований Z-set) і виконує операторове «food cost feeds analytics».

**Порядок:** main мерджить SHEAF3 (не лейн — merge) → хвиля 1: LV-INC-A ∥ LV-LOC-B ∥ LV-PCD-C → хвиля 2: LV-INC-D (Lean) ∥ LV-COL-E ∥ LV-INC-F (Datalog) → потім AX2 (GENTAB) і S3.

---

## 9. Лейн-картки (готові до запуску; лейни не роблять git, main комітить; ≤ 3 паралельно; heavy — тільки `bebop-lang/tools/slot.sh`)

Спільне для всіх карток: правила `/root/dowiz/.claude/lanes/DOWIZ-COMMON-RULES.md`; RED-first (один іменований мутант на кожен новий тест, записаний у `.<lane>-notes.md` з `restore`); **без фаззерів** — один іменований зіпсований байт/клітинка; **без UI — не done** (рядок UI у вердикті); live-proof на `qa-durres` із контрактом (producer/consumer/schema/version/description); `export PATH=/root/.cargo/bin:$PATH`; `rc 137` = повторити раз; `.bp`-правки — ідіоми `if ... then ( ... ) else 0`, одноклітинні масиви на рівні fn, `(buf, ...) -> len` замість `-> str`, ≤ 8 kept symbols через `sys_clone`.

### LV-INC-A — ZSet + PROJ kind 3/4 у bebop-store, три двійники

- **Мета:** `+crates/bebop-store/src/zset.rs` (`ZSet`, `from_kv_chain`, `from_log_tail`, `add/neg`), `proj.rs` `KIND_BLOB = 3` (OUTB) і `KIND_ZSET = 4`, крок/refold за §2.4; двійники `selfhost/prelude/proj.bp`, `crates/bebop-wasm/src/proj.rs` (`bw_proj` kind 3/4), `crates/bebop-wasm/oracle.py`; фікстура `fixtures/proj4.store` + `.expected`.
- **Володіє:** `+crates/bebop-store/src/{zset.rs,zset/}`, `crates/bebop-store/src/{proj.rs,proj/}`, `crates/bebop-wasm/src/proj.rs`, `crates/bebop-wasm/oracle.py` (розділ proj), `crates/bebop-wasm/fixtures/proj4.*`, `crates/bebop-wasm/gate.sh` (крок proj), `bebop-lang/selfhost/prelude/proj.bp`, `bebop-lang/bench/vs_rust/std_tests/proj4*.bp`.
- **Не чіпає:** `workers/api/**`, `crates/dowiz-hub/**`, `bebop.bp`/`compiler/*`, `kv.rs`/`kv/*`, `evlog.rs`, `formal/**` (це LV-INC-D), усе з SHEAF3.
- **Команди:** `cd /root/dowiz/crates/bebop-store && /root/dowiz/bebop-lang/tools/slot.sh inca cargo test --lib`; `cd /root/dowiz/crates/bebop-wasm && /root/dowiz/bebop-lang/tools/slot.sh inca ./gate.sh` (кроки `proj`, `key`); `cd /root/dowiz/bebop-lang && tools/slot.sh inca bench/vs_rust/std_golden.sh` (gate proj4); `tools/slot.sh inca bench/vs_rust/dagfull.sh` (store arm); wasm ratchet `bytes.baseline` — ріст названий і прийнятий або 0.
- **Вихід:** AC-LV-INC-01/02/03/09 зелені; `proj4_price_step_us` і `proj3_cold_read_us` записані з контролем у тому самому процесі; старі `*.store` голдени байт-незмінні; RED-мутанти (a)(b) задокументовані.
- **Розмір:** 4 дні. **Залежності:** немає (P0 не блокує: фасад `bebop-store` лишається).

### LV-LOC-B — S2b: кухонний rail замовлень офлайн через той самий glue (після merge SHEAF3)

- **Мета:** `+crates/dowiz-hub/src/glue/orders.rs` (хвіст подій замовлень, `decide` як write door планшета, rebase через `order_machine`), `bebop-wasm` `tablet.rs` + `bw_tab_order`, `hubdo/glue_turn.rs` маршрут для `log`, `services/operations/orders/glue.rs` контракт, canvas `board/rail.rs` офлайн-банер + черга, live-proof контракт + probe, flows `f12-offline-kitchen.mjs`.
- **Володіє:** `+crates/dowiz-hub/src/glue/orders.rs` + `glue/orders/`, `crates/bebop-wasm/src/tablet.rs` (після merge SHEAF3; + `tests/tablet_orders.rs`), `workers/api/src/hubdo/glue_turn.rs` (після merge SHEAF3), `+workers/api/src/services/operations/orders/glue.rs` (+ route row у `hubdo/edges.rs` — **один рядок**, узгодити з main), `workers/api/public/room/canvas/{tablet.js,tablet.test.mjs}`, `crates/dowiz-canvas/src/board/rail*.rs` (якщо такий модуль; інакше — файл, названий у вердикті), `+e2e/flows/f12-offline-kitchen.mjs`, `+tools/live-proof/{contracts/feature-glue-orders.json,probes/feature-glue-orders.mjs}`.
- **Не чіпає:** `crates/dowiz-hub/src/glue.rs` (SHEAF3), `glue/{walk,delta,merge,wire,desk,tablet}.rs`, `stock/*`, `bebop-store/**`, `formal/**`, `sw.js` (hand-back: SHELL_CACHE bump як текст).
- **Команди:** `cd /root/dowiz/crates/dowiz-hub && slot.sh locb cargo test --lib glue::`; `cd /root/dowiz/workers/api && slot.sh locb cargo +1.96.1 check --lib --target wasm32-unknown-unknown`; `cd /root/dowiz/crates/bebop-wasm && slot.sh locb ./tablet.sh`; `node --test workers/api/public/room/canvas/tablet.test.mjs`; гейти `tools/gates/{one-venue,no-tracking,dataflow,canvas-wire,ui-reach}.sh`; live: `FLOWS_HOST=https://qa-durres.dowiz.org node e2e/flows/run.mjs f12` **після** деплою main.
- **Вихід:** AC-LV-LOC-03/04/05 зелені; `len == events + held` закон у тесті; `canvas-wire` у бюджеті або названий; UI-рядок у вердикті.
- **Розмір:** 4 дні. **Залежності:** main змерджив SHEAF3 (AC-LV-LOC-01) і задеплоїв.

### LV-PCD-C — Опублікований корінь + доказ ціни на чеку (S5)

- **Мета:** `+crates/bebop-store/src/merkle.rs` (корінь над K256, `prove/verify`, ≤ 256 B), PROJ kind 5 ROOT (двійники: `proj.bp`, `bebop-wasm`, `oracle.py --verify`), підпис ML-DSA-65 у DO (`hubdo/publish.rs`: `root, sig, pk_id` у `manifest.json`; секрет `VENUE_SIGN_SK` — операторове Q4), `bebop-wasm` `bw_verify`, сторінка `/verify` у storefront (`public/store/verify.js` + QR на чеку/сторінці замовлення), `oracle.py --verify`, live-proof контракт.
- **Володіє:** `+crates/bebop-store/src/merkle.rs` + `merkle/`, `+crates/bebop-wasm/src/verify.rs` (+ `gate.sh` крок `verify`, `oracle.py` розділ verify), `+bebop-lang/selfhost/prelude/merkle.bp` (читач-двійник), `workers/api/src/hubdo/publish.rs` (**лише** додавання полів у manifest; AX3 cutoff не рухати), `workers/api/public/store/{verify.js,verify.test.mjs}`, `workers/api/public/store/order-*` (QR-рядок), `tools/live-proof/{contracts,probes}/feature-verify.*`, `+docs/design/SPEC-PCD-2026-10-27.md` (формат сертифіката: текст нормативний, за зразком CERTIFICATE-FORMAT §2).
- **Не чіпає:** `proj.rs` (kind 5 — узгодити з LV-INC-A: **конфлікт файлів** → kind 5 додає LV-INC-A за текстом від C; C володіє лише `merkle.rs`), `fiscal/**` (AC-LV-PCD-05 — окремий крок після), `pq/**` (використовує `dsa.rs` як є), `formal/**`.
- **Команди:** `cd /root/dowiz/crates/bebop-store && slot.sh pcdc cargo test --lib merkle::`; `cd /root/dowiz/crates/bebop-wasm && slot.sh pcdc ./gate.sh` (крок verify 4/4); `node --test workers/api/public/store/verify.test.mjs`; `cd /root/dowiz/workers/api && slot.sh pcdc cargo test --lib publish::`; гейти `personal-data`, `sw-shell`, `ui-reach`; live `feature-verify.mjs` після деплою.
- **Вихід:** AC-LV-PCD-01/02/03 зелені; один іменований зіпсований байт шляху → відмова в усіх 4 читачах; UI-рядок (QR видно, `/verify` працює офлайн).
- **Розмір:** 3 дні. **Залежності:** Q4 (ключ підпису) — до старту; LV-INC-A для kind 5 (або C пише корінь у `settings` до злиття A — fallback, названий у картці).

### LV-INC-D — `ZSet.lean` + `Merkle.lean` + `Sync.lean` (Lean-лейн, не торкається Rust/bebop)

- **Мета:** теореми §2.4/§4.4/§3.4; `Bebop.lean` імпорти; `tools/gates/lean.baseline` при потребі; `lean_tests.rs`-стиль twin-тестів — hand-back текстом (володіє LV-INC-A).
- **Володіє:** `bebop-lang/formal/Bebop/{ZSet,Merkle,Sync}.lean`, `bebop-lang/formal/Bebop.lean` (рядки import), `bebop-lang/formal/Bebop/Datalog.lean` (лише `counting_sound` — узгодити з LV-INC-F: F не чіпає `.lean`).
- **Не чіпає:** усе поза `formal/`.
- **Команди:** `cd /root/dowiz/bebop-lang/formal && /root/dowiz/bebop-lang/tools/slot.sh incd lake build` (`/root/.elan/toolchains/leanprover--lean4---v4.33.1/bin`); `slot.sh incd /root/dowiz/tools/gates/lean.sh`; `grep -c sorryAx` = 0; `lake env lean ../../tools/gates/lean-axioms.lean` — без нових рядків проти `lean.axioms`.
- **Вихід:** AC-LV-INC-08, AC-LV-PCD-06, AC-LV-LOC-06; RED: один `hinside`-подібний мутант на теорему (нотатки).
- **Розмір:** 3 дні. **Залежності:** немає (може йти в хвилі 1 замість C, якщо Q4 не відповіли).

### LV-COL-E — Фактовий блок `sales.dwb` + 4 скалярних ядра + аналітика читає блоки

- **Мета:** §5.4 повністю, UI-колонка food cost, позначка trace.
- **Володіє:** `crates/dowiz-hub/src/block/schema.rs` (один рядок схеми), `+crates/dowiz-hub/src/block/{kernels.rs,kernels/}`, `crates/dowiz-hub/fixtures/blocks/sales_*.dwb` (+ `examples/generate_fixtures.rs`), `bebop-lang/selfhost/std/block.bp` (`bk_sum_by` та ін.) + `bench/vs_rust/std_tests/block_kernels.bp`, `crates/bebop-wasm/{src/block.rs,oracle.py --kernel,gate.sh blocks}`, `workers/api/src/services/analytics/{cube.rs,history.rs,week_top.rs}` (читання блоків), `workers/api/src/hubdo/cube.rs` (запис блоку при ротації), `workers/api/public/admin/analytics*.js`, flows F11.
- **Не чіпає:** `block/view.rs`, `taste*`, `proj.rs`, `fold/projection.rs`, `formal/**`.
- **Команди:** `cd /root/dowiz/crates/dowiz-hub && slot.sh cole cargo test --lib block::`; `slot.sh cole cargo test --lib analytics` у `workers/api`; `cd crates/bebop-wasm && slot.sh cole ./gate.sh` (blocks 4/4 + `--prove`); `cd bebop-lang && slot.sh cole bench/vs_rust/std_golden.sh` (gate block_kernels); гейти `float-money`, `venue-clock`, `ui-reach`; flows F11 live.
- **Вихід:** AC-LV-COL-01..05; `year_scan_us` з контролем; UI-рядок.
- **Розмір:** 3 дні. **Залежності:** LV-INC-A для `step` над Z-set-хвостом (без нього — лише `full`, названо).

### LV-INC-F — Datalog: ваги, counting, DRed-closure, EDB-дельти з KV-ланцюжка (AX1 + AX14, bebop-бік)

- **Мета:** AX-план A1 як написано + `dl_kv.bp`: `from_kv_chain → Δ_stock/Δ_bom`; `dagfull datalog` на 10⁴ змішаних подій; `dl_bench.sh dl_event_del_ns`.
- **Володіє:** `bebop-lang/selfhost/std/{dl_del.bp (new),dl_kv.bp (new),dl.bp,dl_eval.bp,gen_dl.bp}`, `bebop-lang/bench/oracles/dl_*.py` + `dl_common.py`, `bebop-lang/bench/vs_rust/{dagfull.sh (datalog arm),dl_bench.sh}`, `bebop-lang/docs/PERF.md` рядки dl_*.
- **Не чіпає:** `bebop.bp`/`compiler/*`, `prelude/*`, `formal/**` (теорема — текстом до LV-INC-D), усе в `crates/`.
- **Команди:** `cd /root/dowiz/bebop-lang && tools/slot.sh incf bench/vs_rust/dagfull.sh datalog`; `tools/slot.sh incf bench/vs_rust/dl_bench.sh`; `tools/slot.sh incf bench/vs_rust/std_golden.sh` (gate dl_*); `bebop-lang/tools/arch_check.py` file-size ≤ 800; повна `bebop-lang/tools/battery.sh` — main, після merge.
- **Вихід:** AC-LV-INC-06; `dl_event_ns ≤ 1 000` незмінний; `dl_event_del_ns ≤ 10 000`; RED (c).
- **Розмір:** 4 дні. **Залежності:** LV-INC-A (`from_kv_chain` семантика; Rust і bebop мають збігтися на фікстурі `kv3.store` — гейт 4/4).

**Файлова роз'єднаність хвилі 1 (A ∥ B ∥ C):** A — `bebop-store/{zset,proj}`, `bebop-wasm/proj`, `prelude/proj.bp`; B — `dowiz-hub/glue/orders`, `bebop-wasm/tablet`, Worker glue routes, canvas; C — `bebop-store/merkle`, `bebop-wasm/verify`, `publish.rs`, storefront verify. Спільний файл `bebop-wasm/gate.sh` і `oracle.py` — **три різні розділи**; кожен лейн править лише свій крок, main зливає. `hubdo/edges.rs` — один рядок на лейн (B і C) — hand-back текстом.

---

## 10. Питання оператору (кожне з рекомендацією)

| # | Питання | Рекомендація |
|---|---|---|
| Q1 | Мерджити SHEAF3 (S0+S1+S2) зараз, до будь-якого нового лейну важелів? | **Так.** Код готовий і перевірений у лейні; живий хаб каже «not built». Merge робить main (не лейн), з повторним `diff3` проти 1d530e75 і `run-all` + flows. |
| Q2 | Z-set як одна валюта (kind 4 memo, ваги в Datalog) — приймаємо як архітектурне рішення, щоб не множити ad-hoc інкременти? | **Так.** Без нового дискового формату; старі образи незмінні; одна теорема замість трьох. |
| Q3 | S2b (замовлення офлайн на кухонному планшеті) — будувати зараз чи лишити лише стік? | **Так, зараз** (LV-LOC-B): той самий glue, `decide` уже є в wasm; це «офлайн венуя», яку ви назвали метою 10-05. |
| Q4 | Ключ підпису венуї (ML-DSA-65) у DO для важеля 3: один платформний ключ чи по ключу на венуу? Секрет — через `wrangler secret`, без ротації (правило 10-20)? | **Один платформний ключ** `VENUE_SIGN_SK` (простіше; `pk_id` у manifest), виданий вами (бокс не створює секретів без дозволу). Без ротації. |
| Q5 | Колонкову аналітику звузити до фактового блоку + 4 ядер і НЕ будувати query engine/векторизацію/компресію? | **Так, звузити.** Виграш на цьому масштабі — не на порядок; робимо лише те, що складається з важелем 1 і дає food cost. |
| Q6 | Sheaf: лишити як словник синку (контексти/перетини/manifest) і не будувати когомологію/дифузію (S7 E2–E4: без виграшу)? | **Так.** Це узгоджується з вашим «gluing detects, does not resolve». |
| Q7 | Фіскальний документ: додавати `doc_id`/`root_day` і QR-верифікацію в текст чека (TAX §3.8) зараз, при `SEND_ENABLED=false`? | **Так, лише текст/QR** (не надсилання): це не змінює фіскальну політику import-only, а дає аудитору перевірку. |

---

## 11. Межі цього дослідження (чесно)

- Жодного cargo-білду не запускалося (диск 98 %, паралельний бенч-лейн): усі нові числа — ESTIMATE з названих MEASURED баз; один FRESH-запит — health QA-хаба (§1.3). Кожна картка вимагає контроль у тому самому процесі перед тим, як число потрапить у `PERF.md` (memory `head-may-not-reproduce-its-own-artifacts`).
- `cpu_of_a_year_read` (куб) — число є в `history/tests.rs`, тут не процитоване; LV-COL-E має зняти його першим.
- Стан `/root/lanes/w-sheaf3` оцінено з нотаток лейну і `ls`, не з прогону гейтів; merge має повторити їх.
- DBSP-формалізація (`tchajed/dbsp-theory`) — читалося README; теореми, які ми переносимо, названі за змістом, не за іменами в тому репо.
- `R-STORE-VS-SQL-2026-10-27.md` на момент написання відсутній (перевірити при merge; його числа замінюють мої ESTIMATE для рядків «cold open/put»).

## 12. Стан 2026-10-28

Що змерджено за день (числа — з нотаток лейнів, release, контроль у тому самому процесі; batch-коміт
решти лейнів нижче — **batch merge pending**, sha впише main). Живі хаби (qa-durres, sushi-durres)
досі на `905f06f7` (`/api/version`, 2026-10-28): нічого з цього не задеплоєно.

| Лейн | Коміт | Що лягло |
|---|---|---|
| P0 | `9f733d46` | заморозка формату: нормативний `FORMAT-BEBOP-IMAGE-v2.md`, каталог іменованих комірок, D2/D4/D7/D8 |
| SHEAF3 | `1eb2083b` | S0 glue, S1 escrow + `Stock.lean`, S2 кухонний планшет офлайн; Lean `ZSet`/`Merkle`/`Sync` |
| LV-INC-A | batch pending | `zset.rs`; PROJ види 3 (BLOB) і 4 (ZSET) з відбитком-tip (FORMAT §9.3); чотири читачі (native, wasm32, python, bebop) 4/4 на `proj4`/`proj4.log`; виправлено kind 2 на v3 у `proj.bp` (тепер відмова why 5) |
| LV-INC-D | `1eb2083b` (разом із SHEAF3) | Lean: `ZSet.lean` (memo_step_eq_full, лінійний/join інкремент, `kv_chain_is_zset`), `Merkle.lean`, `Sync.lean`, лічильники в `Datalog.lean`; 9 RED-мутантів rc=1 |
| LV-INC-D2 | batch pending | `DatalogZ.lean`: `zset_present_iff_pos`, `store_present_iff_pos`, `dred_sound` (+ `dred_needs_sound_keep`); 31 аксіома без змін, sorryAx 0 |
| LV-INC-F | batch pending | `dl_del.bp`: ваги/лічильники для нерекурсивних страт, DRed для рекурсивних; `dl_kv.bp`; 19/19 dl-голденів |
| LV-LOC-B | batch pending | S2b: rail замовлень кухні офлайн (`glue/orders.rs`, `tablet.rs`, Worker `operations/orders`), закон `sent == merged + held` |
| LV-COL-E | batch pending | денний блок продажів `sales:v1(day:i32:0,dish:i64:0,qty:i32:0,gross:i64:1,cqty:i32:0,cgross:i64:1,cost:i64:1)` (K64 `85978ba600000057`), 4 ядра, foodCost в аналітиці; `std_golden` 162/162 |
| W-KVPERF | batch pending | `overlay.rs`: O(D log n) відкриття v3 замість O(n); `prefix_range` бінарним пошуком; `Kv::remove_many` |

| Число | Ціль (§7) | Виміряно | Вердикт |
|---|---|---|---|
| `proj4_price_step_us` (165 страв, 1 put) | ≤ 50 | 89 (повний перерахунок ~2 350) | **НЕ ДОСЯГНУТО**; ~26× проти refold |
| `proj3_cold_read_us` (2 000 подій, +9) | ≤ 500 | 30,7 (HIT 4,2; повний 2 349) | так |
| `dl_event_del_ns` (видалення 1 факту) | ≤ 10 000 | 5 259 (legacy 5 494) | так |
| `dl_event_del5_ns` (5 фактів) | — | 13 839 проти legacy 28 906 | 2,1× |
| `dl_event_ns` (вставка) | ≤ 1 000 незмінний | 930 / 930 (HEAD / нове, 21 перемежований прогін) | без регресу |
| `year_scan` (рік продажів) | ≤ 500 µs | ~2,4 ms (JSON-куб ~7 ms у тому самому процесі) | **НЕ ДОСЯГНУТО**; 3× швидше за JSON |
| reopen + get, 1e5 ключів, 32 дельти | — | 56 µs (було 2 077 µs) | ~37× |

**Відкрите.**
- Видалення одного ключа KV за O(log n): `Kv::remove` досі `Vec::remove` (memmove n/2); виправлення
  потребує зробити `Kv.entries` приватним — поле будують struct-літералом ~20 файлів поза лейном.
  Є лише пакетне `remove_many` (один прохід).
- DRed: `dred_sound` доведено **за гіпотези K ⊆ M** (набір, що лишився після over-delete, лежить у
  новій найменшій моделі); що правило over-delete у `dl_del.bp` її забезпечує — не доведено (потрібні
  дерева виведення). `dred_needs_sound_keep` показує, що гіпотезу не можна відкинути.
- LV-PCD-C (опублікований Merkle-корінь + доказ ціни на чеку): у роботі, не в цьому батчі; підпис
  чекає на ключ венуї `VENUE_SIGN_SK` від оператора (Q4). Release-бенч кореня 165 страв — 191 µs проти
  цілі 20 (sha256 у `nodekey` повільний); перевірка 8,07 µs (≤ 10, так).
- `proj4_price_step_us`: решта після виправлення пошуку ключів у `base_get` — читання memo ~19 µs і
  дельта ~30 µs; наступний крок — zero-copy пошук бази через `KvIn`.

## Додаток A. Джерела

DBSP: https://www.vldb.org/pvldb/vol16/p1601-budiu.pdf · https://arxiv.org/abs/2203.16684 · https://github.com/tchajed/dbsp-theory · https://feldera.com/blog/Best-Research-Paper-VLDB-2023 · Differential dataflow: https://www.microsoft.com/en-us/research/publication/differential-dataflow/ · Gupta–Mumick–Subrahmanian 1993: https://www.springerprofessional.de/doi/10.1145/170036.170066 · Hu–Motik–Horrocks 2018: https://arxiv.org/pdf/1711.03987 · Adapton: https://people.mpi-sws.org/~hammer/adapton-pldi2014.pdf · Salsa: https://docs.rs/rust-analyzer-salsa/^0.17.0-pre.4 · Zero: https://zero.rocicorp.dev/llms.txt · Local-first: https://electric-sql.com/docs/reference/local-first · https://kleppmann.com/2021/06/14/emergencity.html · Automerge: https://talks.cam.ac.uk/talk/index/163276 · cr-sqlite/Replicache: https://legacy.electric-sql.com/docs/reference/alternatives · Certifying algorithms: https://researchportalplus.anu.edu.au/en/publications/certifying-algorithms/ · Authenticated data structures, generically: https://cs.umd.edu/~mwh/papers/gpads.pdf · Abadi–Madden–Hachem 2008: https://users.cs.utah.edu/~pandey/courses/cs6530/fall22/papers/rowvcolumn/abadi-sigmod08.pdf · MonetDB/X100: https://cs.brown.edu/courses/cs227/archives/2008/Papers/ColumnStores/MonetDB.pdf · DuckDB: https://duckdb.org/pdf/SIGMOD2019-demo-duckdb.pdf.
Внутрішні: `docs/research/2026-10-05-bebop-db-gaps-strengths-tensor-graph.md` (R-BEBOPDB, Part D/G), `2026-10-06-s7-sheaf-engine-arxiv-and-experiments.md` (E1–E6), `2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` (§4, §7.3, §8), `2026-10-02-ax-implementation-plan.md` (A1, A2, A4, §6), `docs/design/SPEC-BEBOP-DAG-RUNTIME-2026-09-28.md` §6, `SPEC-DATALOG-AND-CODEC-2026-09-28.md` A.5/B.3/B.4, `bebop-lang/docs/CERTIFICATE-FORMAT.md`, `R-EMBED-STORE-2026-10-27.md` (P0).
