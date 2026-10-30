# R-EMBED-STORE — bebop-store як самостійна вбудована бібліотека («малий верифікований pointer-free KV»)

Status: 2026-10-27, read-only дослідження + архітектура + специфікація (лейн R-EMBED-STORE, Fable). У дереві нічого не змінено, жодної git-команди запису не виконано. Кожне твердження про код несе `file:line`, перевірений сьогодні читанням; кожна зовнішня теза несе URL; кожне число, якого я не міряв, позначене **UNVERIFIED**. Єдине, що я запускав: `cargo test --lib --release` у `crates/bebop-store` через `slot.sh` — **107 passed, 0 failed, 2 ignored, 1.83 s** (лог у сесійному scratchpad, файл `bs-test.log`; `slot.sh` звільнив слот з rc=0 через 17 с).

Ціль, як її поставив оператор: сховище bebop має стати **окремою вбудованою бібліотекою класу SQLite/LMDB (не сервером)**, якою можуть користуватися люди поза dowiz: «маленьке, верифіковане, pointer-free KV». Документ відповідає на чотири питання: (1) що є насправді, (2) що знає світ, (3) якою має бути цільова архітектура, (4) як до неї дійти фазами — і де частина цілі є поганою ідеєю.

---

## 0. Короткий підсумок (12 рядків)

1. **Ядро формату вже є і воно справді pointer-free**: комірки по 8 байт, два суперблоки на різних 4 KiB-сторінках, bump-арена самоописних об'єктів з `crc32` у заголовку, `ref` = зміщення в комірках відносно об'єкта (`crates/bebop-store/src/lib.rs:1-24`). Його читають чотири незалежні реалізації й гейт `crates/bebop-wasm/gate.sh:1-24` змушує їх зійтися на одному числі.
2. **Протокол коміту правильний за формою, але не доведений і не повний**: три `fdatasync` у Rust (`lib.rs:843-853`), два `msync` у bebop (`selfhost/prelude/store.bp:883-889`); суперблок пишеться останнім. Але Rust-письменник **не пише якір `anc`** (`lib.rs:833`, `proj.rs:32`), тому Rust-образ після збою перевіряється або повним обходом арени (bebop), або **взагалі не перевіряється** (Rust-читач не має `st_reopen_verify`; `Kv::load_checked` перевіряє crc кореня, чотирьох масивів і ланцюжка — `verify.rs:85-101`, але **не crc PartTab**).
3. **Жодного тесту на збій посеред запису в Rust немає**. Є лише `bench/vs_rust/scrash.sh` для bebop (SIGKILL у випадкову мить, 100 спроб) — доводить process-crash, не power-loss, і не для Rust-шляху.
4. **Паралельності немає й не описано**: у бібліотеці — жодного блокування файлу; «single writer» тримається тим, що Durable Object однопотоковий (`workers/api/src/hubdo/store.rs:120-170`). Поза dowiz ця гарантія зникає.
5. **Образ живе цілком у пам'яті** (`Store { cells: Vec<i64> }`, `lib.rs:58-60`; `from_bytes` копіює, `lib.rs:341-386`); є `View<'a>` без копії (`lib.rs:118-152`). Стелі: 10 MiB на KV-образ у хабі (`crates/dowiz-hub/src/catalog.rs:47`), 512 MiB `MAX_PAD_CELLS` (`lib.rs:76`). Це не LMDB-клас масштабу.
6. **KV = відсортований масив + ланцюжок дельт ≤ 32**, компакція переписує все (`kv/delta.rs:43-52`, `kv.rs:316-331`): амортизована вартість запису O(n/32) — чудово для 165 страв, погано для 10⁶ ключів.
7. **Lean доводить лише абстрактну карту** (`formal/Bebop/KvDelta.lean:274-316`): жодного байта, жодного crc, жодного збою. 26 аксіом `Syscalls.lean` кажуть лише «повертає 0 або -errno і не чіпає арену» (`Syscalls.lean:214-218`) — вони **не моделюють файл узагалі**, тож на них не можна побудувати жодну теорему про довговічність.
8. **Рекомендована архітектура** (§3): п'ять крейтів — `bebop-image` (формат, `no_std`+`alloc`, нуль залежностей), `bebop-kv` (розкладка KV v2/v3 + транзакції), `bebop-log` (append-log), `bebop-io` (file/mmap-ro/memory бекенди, lock-файл, рівні довговічності), `bebop-ffi` (C ABI + ctypes). Адаптер Durable Object/R2 **лишається в dowiz**. Один письменник + знімкові читачі через незмінні покоління; серіалізовані write-txn, snapshot isolation для читачів; протокол коміту з двома `fdatasync` і якорем; відновлення O(розмір останнього коміту).
9. **Специфікація** (§4): 26 критеріїв `AC-ES-*` і 31 тест/гейт `T-ES-*`, включно з **переліченим** (не фаззованим) crash-harness над симульованим диском, кросреалізаційною конформацією чотирьох читачів, трьома Lean-поставками й бенчмарком проти SQLite/redb/LMDB.
10. **План** (§5): 8 фаз, ~28–31 днів одного Opus-лейну послідовно, ~16–18 календарних днів при трьох файлово-роз'єднаних лейнах; P8 (індекс для >10⁵ ключів) — окреме рішення оператора.
11. **Погана ідея** (§6): змагатися з LMDB за сирий throughput; multi-writer; MAP_SHARED-запис у бібліотеці; C ABI до стабілізації API; публікувати під AGPL як «бібліотеку для людей».
12. **Рішення оператора** (§7): 10 питань, кожне з рекомендацією.

---

## 1. Що є сьогодні: формат і протокол запису, як їх бачить код

### 1.1 Формат на диску (перевірено по коду)

| Елемент | Де визначено | Факт |
|---|---|---|
| Комірка | `lib.rs:144-147`, `:341-347` | `i64` little-endian, 8 байт; все адресується у комірках |
| Суперблок A / B | `lib.rs:31-33` | комірки `0..16` і `512..528` → різні 4 KiB-сторінки |
| Поля суперблока | `lib.rs:8-13`, bebop `store.bp:81-100` | 0 magic `BEBOPST1` (`lib.rs:29`), 1 version=2, 2 generation, 3 **ref на PartTab** (не на корінь даних), 4 `arena_used` (абсолютний індекс), 5 PROJTAB, 6 migration_table, 7 live_cells, 8 superseded_cells, 9 `anc`, 10–11 нуль, 12 arena capacity (комірок), 13–14 нуль, 15 crc32 комірок 0..14 |
| Вибір суперблока | `lib.rs:197-216` (`pick_in`) | валідний (magic+crc) з більшою генерацією; **якщо новіший валідний, але не вміщується (`sb_fits_in`, `:170-194`), образ ВІДМОВЛЯЄТЬСЯ**, а не читається попередня генерація (`:518-526`, тест `:961-974`) |
| PartTab | `lib.rs:689`, `:809-822` | об'єкт на 19 комірок у хвості сторінки *іншого* суперблока: `pt = (512 - sb) + 16`; корінь даних у сирій комірці `pt+18`, курсор `pt+19`, генерація `pt+20` (`:219-227`) |
| Арена | `lib.rs:35` | з комірки 1024, bump-алокатор, append-only |
| Об'єкт | `lib.rs:16-20`, `:746-761`, `:774-778` | `h0 = (digest_lo32 << 32) \| len_cells`, `h1 = (crc32(payload) << 32) \| generation_lo32`, далі payload; `ref` = `target - this_obj`, 0 = null |
| CRC | `lib.rs:22-24`, `crc.rs` | zlib CRC-32 над LE-байтами комірок; табличний |
| «Довжина — це заява» | `lib.rs:235-238`, `:261-277` | `obj_cells` обмежує заяву образом; `follow` з `checked_add`; читач за межею читає 0 |
| Обрізаний образ | `lib.rs:410-421`, `:366-383` | `to_bytes_trimmed` ріже на `arena_used`; `from_bytes` добиває нулями лише вгору від `arena_used` і не більше `MAX_PAD_CELLS` |

**Розкладки над форматом** (усі — об'єкти в арені, корінь через PartTab):

- **KV v1/v2/v3** (`kv.rs:1-32`, `kv/delta.rs:9-37`): корінь `{n, ref KIDX, ref KBLOB, ref VIDX, ref VBLOB, VERSION, [ref DELTA, D]}`; ключі відсортовані (`kv/zc.rs:25-29`); v2 пакує 8 байт у комірку; v3 = v2-база + ланцюжок записів `{ref NEXT, OP, KLEN, VLEN, key.., value..}`, найновіший перший, `D` = заява про довжину ланцюжка, перевіряється обходом (`kv/delta.rs:123-162`). Дайджести розкладок прибиті константами, sha256 лишається на боці bebop (`kv.rs:59-65`).
- **EvLog v1/v2** (`evlog.rs:12-45`): корінь `{n, ref LAST, has_tip, tip0..3, VERSION}`, запис з content-id, prev-id, `ref` на попередній — ланцюжок хешів.
- **PROJ/PROJTAB/OUT** (`proj.rs:1-35`): мемо проєкцій DAG-рантайму, з суперблока (комірка 5).

### 1.2 Протокол запису — три письменники, три шляхи на «диск»

**(a) Rust → файл** (`lib.rs:658-672` коментар, `:843-853` код):
1. `begin` бере курсор і генерацію з живого PartTab, а не з суперблока (`:711-723`), і перевіряє, що курсор усередині арени та не позаду `arena_used` (`:733-740`);
2. `alloc`+`put_cell`+`link`+`seal` будують об'єкти в пам'яті від `tx.mark` до `tx.cursor`;
3. `stage_commit` пише PartTab у хвіст сторінки іншого суперблока і сам інший суперблок у пам'яті (`:798-839`); **комірка 9 (`anc`) = 0, 10–11 = 0** (`:833`);
4. `commit`: `write_cells([mark, cursor))` → `sync_data` → `write_cells(PartTab, 21)` → `sync_data` → `write_cells(SB, 16)` → `sync_data`.

Властивість, на яку спирається код: між будь-якими двома кроками падіння лишає генерацію k−1 читабельною, бо нічого з того, що називає старий суперблок, не перезаписано.

**(b) bebop → mmap MAP_SHARED** (`store.bp:435-468`, `:875-908`): `st_commit_x` рахує `anc = (mark << 32) | crc32(cells[mark..cursor))` і пише його в комірку 9 (`:465-466`); `st_commit_sync` = `msync([mark,cursor))` → `st_commit` → `msync(сторінки 0–1)`. Два бар'єри замість трьох, бо PartTab і суперблок лежать на одній сторінці і їх валідність перевіряється crc. `st_reopen_verify` (`:188-214`) на кожному відкритті: якщо `anc != 0` — перевірити crc PartTab і crc діапазону останнього коміту (`st_anc_ok`, `:205-214`); якщо 0 — обмежений обхід арени від курсора іншого суперблока; при провалі — **затерти magic у відображенні** й перейти на попередню генерацію, яку перевірити повністю.

**(c) Rust → Durable Object (dowiz)** (`workers/api/src/hubdo.rs:43` `CHUNK = 96 KiB`; `hubdo/atomic.rs:1-26`; `hubdo/store.rs:120-170`): образ цілком у пам'яті DO, `commit_bytes` (`lib.rs:432-435`) лише стадіює, далі `to_bytes_trimmed`, диф по чанках проти копії в пам'яті, один атомарний `put(entries)` чанків + мета (`Meta{generation, chunks, len}`). Атомарність тут дає **Cloudflare**, а не формат. Коментар `lib.rs:426-431` («whole-image PUT … no partial state») описує попередній світ: сьогодні пишуться лише змінені чанки, і атомарність спирається на W-ATOMIC.

### 1.3 Читання, перевірка, компакція (як є)

- `Kv::load_checked` (`verify.rs:85-101`): crc кореня → crc чотирьох масивів → версія ≤ 3 → `chain_in` → crc кожного запису → декодування. `KvIn::open` робить те саме без копії, `reopen(Checked)` — без crc для вже перевірених байтів (`kv/zc.rs:11-24`).
- `EvLog::chain_scan` (`verify.rs:137-158`): корінь з поганим crc — відмова; запис з поганим crc — **карантин**, якщо ланцюжок усе ще віддає рівно `n` записів і `prev`-id сходиться. `ChainMark` (`verify/since.rs:1-30`) не перехешовує те, що вже доведено в тій самій пам'яті.
- Компакція KV (`kv.rs:316-357`): свіжий образ, подвоєння від 16 KiB до стелі, запис усіх записів один раз, `to_bytes_trimmed`. Тригери в письменнику дельт (`kv/delta/write.rs:260-279`): `MAX_DELTAS = 32`, `superseded*4 > live`, база v1, арена повна.
- Хаб: `Catalog::to_bytes` (`crates/dowiz-hub/src/catalog.rs:121-142`) спершу пробує `append_delta`, будь-яка відмова → компакція. П'ять образів на хаб: `log`, `catalog`, `settings`, `posts`, `stock` (+ `floor`, `archive`, `__platform`) — усі через одну `HubImages`.

### 1.4 Де документ і код розходяться (знайдено сьогодні)

| # | Твердження | Факт | Наслідок |
|---|---|---|---|
| D1 | `lib.rs:12` — комірки 10 «commit-object», 11 «heads-table» | Rust зануляє 9..12 (`lib.rs:833`); bebop пише 10–11 нулями (`store.bp:91-96`), а `st_commit_head` читає `sb+10` (`store.bp:1043`) | дві розкладки суперблока співіснують у коментарях; жоден читач їх не перевіряє |
| D2 | `proj.rs:32` і `lib.rs:795-798`: «цей крейт пише `anc` = 0» (свідомо) | так і є | **кожен Rust-образ відкривається bebop-ом через обхід арени**, а Rust-читач не має жодного еквівалента `st_reopen_verify` → після збою на (a) торн-об'єкт останнього коміту виявить лише `Kv::load_checked`, і то тільки якщо він досяжний з кореня |
| D3 | `lib.rs:426-431`: «whole-image PUT … old image stays readable until the new one lands» | DO пише **змінені чанки** (`hubdo/store.rs:131-146`) | гарантія тепер належить `hubdo/atomic.rs`, не формату; коментар застарілий |
| D4 | `LANG-DB-DESIGN.md:270-279` §4g: «kill -9 anywhere: consistent by construction» | доведено лише для bebop MAP_SHARED (`scrash.sh`); для Rust-шляху (a) **жодного тесту**, для `Store::create` (`lib.rs:283-301`) — `std::fs::write` без `fsync` і без `fsync` каталогу | створення файлу не довговічне; після power-loss файл може бути порожнім при валідній «пам'яті» про нього |
| D5 | `bebop-ceiling-not-capacity.md`: «superseded_cells ніколи не пишеться» | пишеться з W-CRC/W-DELTA (`kv.rs:226-238`, `write.rs:326`; тест `superseded_cells_are_written_and_compaction_clears_them`) | нотатка застаріла (у ній самій є CORRECTION) |
| D6 | `kv/zc.rs:33` «ключі порівнюються як сирі байти; `Kv` декодує з втратами» | `delta.rs:168` `String::from_utf8_lossy` | два читачі **можуть розійтися** на не-UTF-8 ключі; сьогодні ніхто такого не пише, але зовнішня бібліотека мусить або заборонити не-UTF-8 ключі, або зробити ключі `&[u8]` скрізь |
| D7 | `alloc` пише `h1 = next_gen` (`lib.rs:757`), `seal` маскує нижні 32 біти (`:777`) | генерація в `h1` = `gen mod 2³²`; `proj` порівнює `input_gen` з цим полем (`proj.rs:16-18`) | межа 2³² комітів на образ — треба записати як ліміт |
| D8 | Rust `pick`/`sb_fits` читають `pt+18..20`, не перевіряючи crc PartTab (`lib.rs:181-191`, `:713-723`) | bebop `st_anc_ok` перевіряє `st_check(pt)` (`store.bp:210`) | один перекинутий біт у PartTab (поза `pt+18`) → Rust-читач вірить курсору; `begin` відмовить лише якщо курсор поза ареною або позаду `arena_used` (`:733-740`). **UNVERIFIED**, чи є тест на біт у `pt+19` |
| D9 | ROADMAP.md bebop-lang має заголовок 2026-09-13 (`ROADMAP.md:3`) з допискою 2026-10-02 | рядки про B1/B5 стор не відбивають W-CRC/W-ZC/W-DELTA/W-HUBCRC | для зовнішнього читача єдиний актуальний опис формату — коментарі в `lib.rs`/`kv.rs`/`delta.rs` |

### 1.5 Що бібліотеці бракує проти класу SQLite/LMDB (чесний список)

1. Моделі паралельності (блокування файлу, читачі під час компакції, багатопотоковість у процесі).
2. Транзакційної семантики як контракту (сьогодні: «одна `Tx` = один коміт», ізоляція не названа).
3. Доказу або хоча б тесту атомарності коміту під збоєм для Rust-шляху.
4. Сторінкового/mmap-доступу для образів понад десятки MiB.
5. Індексу, що масштабується за ключами (сортований масив + 32 дельти — це не індекс).
6. Відокремлення формату від продукту: `CatalogView`, `hub_image`, `Meta`, `LogImage`, `proj`/`nodekey` живуть упереміш із форматом.
7. Стабільного публічного API, C ABI, документа формату з номером версії та політикою міграцій.
8. Рівнів довговічності як параметра, а не як трьох різних функцій (`commit`, `commit_bytes`, `st_commit_sync`).

---

## 2. Дослідження: prior art і що з нього пасує pointer-free образу з ланцюжком дельт

### 2.1 Вбудовані сховища

| Система | Механізм коміту | Паралельність | Що взяти | Що не брати | Джерело |
|---|---|---|---|---|---|
| **LMDB** | COW B+tree, **дві мета-сторінки** чергуються, читачі без блокувань через mmap, один `fsync` (або два без `MDB_NOMETASYNC`) | 1 письменник + N читачів, таблиця читачів у lock-файлі | модель «два слоти + crc/txn-id», reader table для компакції, `mdb_copy -c` як компакція | mmap-запис (MAP_SHARED) як основний шлях письменника: саме це робить відновлення після `fsync`-помилки неможливим (див. 2.2) | Chu, «MDB: A Memory-Mapped Database and Backend for OpenLDAP» (2011), https://www.openldap.org/pub/hyc/mdb-paper.pdf ; https://www.symas.com/lmdb |
| **SQLite** | rollback journal або **WAL**; документ «Atomic Commit» описує припущення про сектори, торн-записи, порядок `fsync` | 1 письменник (WAL: читачі не блокують письменника) | формулювання припущень про пристрій (атомарність сектора, «powersafe overwrite»), WAL-checkpoint як окрема операція, тестовий режим із симульованим збоєм | VDBE/SQL-поверхня (не наша ціль) | https://www.sqlite.org/atomiccommit.html ; https://www.sqlite.org/wal.html ; https://www.sqlite.org/psow.html ; тестування збоїв https://www.sqlite.org/testing.html |
| **bbolt** (BoltDB) | COW B+tree, дві мета-сторінки, `fdatasync` на даних, потім мета | 1 письменник + N читачів, read-txn тримають старі сторінки → freelist | простота API (`Update/View`), freelist-досвід як антиприклад (довгі read-txn роздувають файл) | freelist-сторінки (у нас bump-арена + Cheney) | https://github.com/etcd-io/bbolt |
| **redb** | COW B+tree, метадані з контрольними сумами (XXH3), опційний 2-phase commit, `Durability::{None,Eventual,Immediate}` | 1 письменник + N читачів, savepoints | **рівні довговічності як enum**, чистий Rust без `unsafe` у ядрі формату, документ `design.md` | B+tree на сторінках 4 KiB (за D.3 звіту R-BEBOPDB — не для масштабу dowiz; див. Q2) | https://github.com/cberner/redb ; https://docs.rs/crate/redb/0.6.1/source/docs/design.md |
| **sled** | lock-free Bw-tree + лог; відомий довгою бетою і проблемами стабільності | багато письменників | — (як приклад, що «новий алгоритм» без доказу коштує років) | архітектуру | https://github.com/spacejam/sled |
| **fjall** | LSM (keyspaces = окремі LSM-дерева), один журнал, MVCC-знімки, single-writer txn опційно | 1 письменник на txn | ярусні відсортовані прогони як природне узагальнення «база + дельти» | компакційний планувальник у фоновому потоці (у нас має бути явний виклик) | https://github.com/fjall-rs/fjall |
| **Badger** | LSM із відділенням ключів від значень (WiscKey) | багато | вартість value-log GC як попередження | — | https://github.com/dgraph-io/badger |
| **Kyoto/Tokyo Cabinet** | hash/B+tree у файлі, in-place | 1 | — | in-place оновлення без журналу (сам клас помилок, якого формат уникає) | https://dbmx.net/kyotocabinet/ |

**Висновок 2.1.** Формат bebop уже є «LMDB-подібною мета-парою + append-only арена без B+tree». Найближчий родич за духом — redb (Rust, чисті типи, рівні довговічності, два слоти метаданих, crc), найближчий за структурою даних — fjall (ярусні відсортовані прогони), найкращий зразок документа про припущення — SQLite `atomiccommit.html`.

### 2.2 Crash-consistency: що відомо про диски й fsync

- **ALICE / «All File Systems Are Not Created Equal»** (Pillai et al., OSDI'14): файлові системи відрізняються атомарністю й упорядкуванням; додатки покладаються на властивості, яких їм ніхто не обіцяв. Метод: записати трасу системних викликів, перелічити стани після збою, перевірити інваріант. https://www.usenix.org/conference/osdi14/technical-sessions/presentation/pillai — **це і є модель для нашого харнесу** (§4.3): перелічення, не рандом.
- **CrashMonkey / B³** (Mohan et al., OSDI'18): bounded black-box crash testing — малі послідовності операцій, усі точки збою, знайдено 10 багів у btrfs/F2FS. https://www.usenix.org/conference/osdi18/presentation/mohan — підтверджує, що **обмежене перелічення ловить реальні баги**, що узгоджується з правилом оператора «без фаззера».
- **«Can Applications Recover from fsync Failures?»** (Rebello et al., ATC'20): після помилки `fsync` ext4/XFS/Btrfs позначають сторінки чистими; PostgreSQL, LMDB, LevelDB, SQLite, Redis — усі відновлюються недостатньо. https://www.usenix.org/conference/atc20/presentation/rebello . Висновок для нас: **помилка `fdatasync` = образ у невідомому стані → відкинути копію в пам'яті і перечитати з диска** (dowiz уже робить це для DO: `hubdo/store.rs:150-157`); для mmap-письменника цього зробити не можна чесно — ще одна причина не писати через MAP_SHARED у бібліотеці.
- **PostgreSQL «fsyncgate» (2018)**: той самий клас; виправлення — PANIC при помилці fsync. https://lwn.net/Articles/752063/
- **Torn writes**: сектор 512 B/4 KiB — єдина межа атомарності, яку можна припускати, і то не всюди (SQLite «powersafe overwrite»). Наш суперблок = 128 байт усередині одного сектора; PartTab = 168 байт у тій самій сторінці на зміщенні 128 — обидва в межах першого 512-байтового сектора сторінки (комірки 512..533 → байти 4096..4264 **UNVERIFIED** проти вирівнювання конкретної ФС, тому crc лишається єдиним аргументом).
- **f2fs на цьому боксі** змонтовано з `fsync_mode=nobarrier` (`LANG-DB-DESIGN.md:3-4`, `RESEARCH-NOPOINTERS-SQL:3-5`): **power-loss тут недоказовий**, лише process-crash. Це означає, що crash-harness має бути **симульованим диском**, а не реальними kill-ами.

### 2.3 Верифіковані сховища

| Робота | Що доведено | Інструмент | Урок для нас | Джерело |
|---|---|---|---|---|
| **FSCQ** (SOSP'15) | crash-safety ФС у Crash Hoare Logic | Coq | специфікація «після збою стан = один із префіксів» формулюється як пост-умова збою | https://pdos.csail.mit.edu/papers/fscq:sosp15.pdf |
| **DFSCQ** (SOSP'17) | `fdatasync`/`fsync` семантика з відкладеним записом, tree-sequence spec | Coq | **модель диска з буфером незаписаних блоків і бар'єром** — те, що треба замість `sys_fsync_spec` | https://pdos.csail.mit.edu/papers/dfscq:sosp17.pdf |
| **Perennial / GoJournal** (SOSP'19, OSDI'21) | паралельний crash-safe журнал, логічно атомарні crash-специфікації | Coq/Iris | якщо колись буде паралельність письменника — це рівень складності; не для фази 1 | https://www.usenix.org/conference/osdi21/presentation/chajed |
| **VeriBetrKV** (OSDI'20, «Storage Systems are Distributed Systems») | верифікований Bε-tree KV, crash-safety через state machines | Dafny | **довести протокол як машину станів із переходом «crash»**, а реалізацію — як refinement | https://www.usenix.org/conference/osdi20/presentation/hance |
| **Armada** (PLDI'20) | refinement паралельних програм через шари | Dafny | шаруватий доказ: абстрактна карта → байти → диск | https://www.microsoft.com/en-us/research/publication/armada-low-effort-verification-of-high-performance-concurrent-programs/ |
| **PoWER / CapybaraKV** (OSDI'25) | crash consistency + виявлення пошкодження в Verus; «corruption-detecting Boolean» для атомарного оновлення контрольної суми | Verus | **найближчий до нашої задачі**: KV, crc, атомарність слота, доказ у Rust-діалекті | https://people.csail.mit.edu/nickolai/papers/leblanc-power.pdf |

### 2.4 Інструменти верифікації Rust ↔ Lean

| Інструмент | Що дає | Статус/обмеження | Пасує? |
|---|---|---|---|
| **Aeneas** | переклад safe-Rust у чисту функціональну модель; бекенд **Lean** (також F*, Coq, HOL4); використовується на SymCrypt/крипто в Microsoft/Google (2025–26) | без `unsafe`, без interior mutability; цикли → рекурсія з паливом; `Vec<i64>` і зріз-арифметика підтримуються | **ТАК для `bebop-image`/`bebop-kv` ядра**: воно вже `forbid(unsafe_code)`-подібне і без залежностей; дає Lean-визначення декодера, які можна з'єднати з `KvDelta.lean`. Джерела: https://lean-lang.org/use-cases/aeneas ; ICFP'22 https://www.microsoft.com/en-us/research/publication/aeneas-rust-verification-by-functional-translation/ |
| **Verus** | SMT-доведення анотованого Rust, включно з `unsafe`-подібними шарами через ghost-стан; PoWER показав crash-consistency | власний діалект анотацій у коді; не Lean | придатний як альтернатива для протоколу диска; суперечить правилу «Lean — лише крос-перевірка» не більше, ніж Lean |
| **Kani** | bounded model checking Rust (CBMC) | обмежені довжини | добре для `pick_in`/`sb_fits_in`/`follow_in` на малих образах (наприклад, 1100 комірок); не замінює доказ |
| **Creusot** | дедуктивна верифікація через Why3 | власні анотації | без переваг проти Aeneas для нашої цілі (Lean уже в дереві) |

**Вибір.** Ядро формату → Aeneas → Lean (одна мова доказу з уже наявним `KvDelta.lean`, гейт `tools/gates/lean.sh` уже рахує аксіоми). Протокол диска → ручна Lean-модель (DFSCQ-стиль) і refinement-теорема; Kani — як дешева «сітка» для функцій бібліотеки до появи доказу. Verus — запасний варіант, якщо Aeneas не перетравить `Vec`-арифметику в `stage_commit` (**UNVERIFIED**: Aeneas на цьому коді не запускався).

---

## 3. Цільова архітектура

### 3.1 Поділ на крейти

```
bebop-image   (no_std + alloc, 0 deps)  — комірки, суперблоки, PartTab, об'єкти, crc, Cells/Image, pick/verify/recover-логіка над абстрактним "Device"
bebop-kv      (no_std + alloc)          — розкладка KV v2/v3, ReadTxn/WriteTxn, scan, compaction policy, Lean-twin tests
bebop-log     (no_std + alloc)          — EvLog v1/v2, карантин, ChainMark
bebop-io      (std)                     — FileDevice (pwrite+fdatasync), MmapRoDevice (читачі), MemDevice, lock-файл, Durability
bebop-ffi     (std, cdylib)             — C ABI + python/ctypes обгортка; оракул oracle.py лишається НЕЗАЛЕЖНИМ читачем, не біндінгом
(поза бібліотекою, в dowiz) dowiz-hub   — CatalogView, LogImage, StockLog, Settings/Posts/Roster, hub_image/Meta, Chunks+W-ATOMIC адаптер DO/R2
(поза бібліотекою, в bebop-lang) proj/nodekey — DAG-рантайм; bebop-wasm — Worker-читач над bebop-image/-kv/-log
```

Принципи: `bebop-image` не знає про файли (`Device` трейт: `read_cells(off, n)`, `write_cells(off, &[i64])`, `barrier()`, `len()`); усе, що сьогодні в `lib.rs` як вільні функції над `Cells`, переїжджає туди без змін семантики; `Store`/`View` лишаються як дві реалізації `Cells`. Жоден крейт бібліотеки не імпортує `dowiz-*`. Гейт: `cargo tree -e no-dev` у кожному з п'яти — нуль зовнішніх крейтів (принцип уже є в `Cargo.toml:5-12`).

### 3.2 Публічний API (Rust)

```rust
pub struct Db<D: Device>;                 // один письменник на процес + lock-файл
impl<D: Device> Db<D> {
  pub fn open(dev: D, opts: Options) -> Result<Db<D>, Refusal>;   // recover() усередині
  pub fn create(dev: D, opts: Options) -> Result<Db<D>, Refusal>;
  pub fn begin_read(&self) -> ReadTxn<'_>;                        // знімок = (sb, generation, root, arena_used)
  pub fn begin_write(&self) -> Result<WriteTxn<'_>, Refusal>;     // Err(Refusal::WriterBusy) якщо вже є
  pub fn compact(&self, into: D) -> Result<(), Refusal>;          // Cheney-копія живих об'єктів у НОВИЙ пристрій; publish = rename (FileDevice)
  pub fn verify(&self, depth: Verify) -> Result<Report, Refusal>; // Anchor | Reachable | WholeArena
  pub fn snapshot_bytes(&self) -> Vec<u8>;                        // to_bytes_trimmed поточного покоління
}
impl ReadTxn<'_> { get(&[u8]) -> Option<&[u8]>; scan(range) -> Iter; len(); generation(); fold_root() -> u64 }
impl WriteTxn<'_> { get; put(&[u8], &[u8]); remove(&[u8]); scan (бачить власні записи); commit(Durability) -> Result<Generation, Refusal>; abort() }
pub enum Durability { None, Data, Full }
pub enum Refusal { NoSuperblock, Truncated{claimed,have}, BadCrc{obj,want,got}, UnknownVersion(i64), ChainBroken{at}, PartTabBad{at}, AnchorBad{mark,used}, ArenaFull{need,capacity}, KeyTooLong, ValueTooLong, WriterBusy, Locked{by}, Io(ErrorKind) }
```

Ключі — `&[u8]`, не `&str` (закриває D6; `Kv` з `String` лишається як зручний декодер поверх). Значення — `&[u8]`. Жодного `panic!` на шляху з байтів образу — це вже правило коду (`lib.rs:548-567`), стає контрактом (AC-ES-05).

### 3.3 Модель паралельності

- **Один письменник на файл**: `bebop-io` бере `flock(LOCK_EX|LOCK_NB)` на `<path>.lock` (не на самому файлі — `rename` при компакції міняє inode); у процесі — `Mutex<WriterSlot>`; другий `begin_write` → `WriterBusy` одразу (без очікування; очікування — справа застосунку).
- **Читачі без блокувань, знімкові**: `ReadTxn` фіксує суперблок і `arena_used`; оскільки арена append-only й нічого з того, що називає старий суперблок, не перезаписується **до компакції**, читач бачить незмінне покоління довільно довго. Це і є MVCC-читачі, які дає сам формат (не «MVCC» у сенсі multi-writer, якого D.3 звіту R-BEBOPDB забороняє).
- **Компакція ≠ перезапис**: нова копія у новий файл → `fdatasync` → `rename` → `fsync` каталогу. Старі читачі тримають старий fd/mmap, inode живе до закриття (LMDB-/Datomic-патерн з `LANG-DB-DESIGN.md:283-288`). Лічильник читачів старого покоління потрібен лише для **in-process** `MemDevice`/`Store`, де компакція замінює буфер — там `Arc<[u8]>` на знімок.
- **Багатопроцесний режим** (кілька процесів читають, один пише): читачі мають бачити оновлений суперблок → `MmapRoDevice` перечитує сторінки 0–1 на `begin_read`; після `rename` нове покоління — у новому файлі, тому читач, який відкрився давно, має перевіряти inode при `begin_read` і переоткриватися (`Refusal::Superseded` → клієнт переоткриває). У фазі 1 — лише **один процес**; багатопроцесність — Q4.

### 3.4 Транзакції та ізоляція

- Write-txn: серіалізовані (один письменник) → **Serializable** тривіально. Read-txn: **Snapshot** одного покоління. Усередині write-txn — read-your-writes через overlay у пам'яті (сьогодні `Catalog.dirty` робить це на боці dowiz, `catalog.rs:65-69`; переїжджає всередину `WriteTxn`).
- Один коміт = один набір дельт + новий корінь (або компакція). Немає часткових комітів: або генерація k+1 цілком, або k.
- Відкат = `abort` (курсор назад, `st_abort`-еквівалент `store.bp:909-912`); після збою — автоматично, бо суперблок не торкнутий.
- Немає nested txn, savepoints, multi-image txn (2PC `st_commit_2pc` у bebop `store.bp:729` **не переноситься**).

### 3.5 Рівні довговічності й протокол коміту

Порядок запису одного коміту (`Durability::Data`), два бар'єри замість трьох:

```
1. pwrite(arena[mark..cursor))             — нові об'єкти (кожен із власним crc)
2. pwrite(PartTab @ (512-sb)+16, 21 cells)  — crc у h1 + crc payload[15]
3. fdatasync                                — бар'єр 1: усе, що називатиме новий суперблок, на пристрої
4. pwrite(SB' @ 512-sb, 16 cells)           — gen+1, ref PartTab, used=cursor, anc=(mark<<32)|crc(arena[mark..cursor)), crc15
5. fdatasync                                — бар'єр 2: точка коміту
```

- Чому один бар'єр між 2 і 4 достатній: SB' валідний лише з crc; якщо він досяг диска, бар'єр 1 гарантує, що PartTab і об'єкти досягли раніше. Rust сьогодні робить три `sync_data` (`lib.rs:847-851`); третій між PartTab і SB потрібен лише якщо не довіряти, що запис 128 байт у той самий сектор, що й PartTab, упорядкований — а він упорядкований бар'єром 3, бо йде після нього. Доказ — AC-ES-20/Lean L2.
- `Durability::None`: кроки 1,2,4 без бар'єрів → process-crash consistent (page cache когерентний), не power-safe; це рівень dowiz-DO (бар'єр робить Cloudflare).
- `Durability::Full`: `Data` + при `create`/`compact`: `fsync(file)` після початкового запису і `fsync(dir)` після `rename`.
- Якір **обов'язковий** для Rust-письменника (закриває D2): відкриття коштує O(розмір останнього коміту), не O(арени).
- Групування: N комітів з `Durability::None` + один `sync()` = аналог `st_commit_batch` (`store.bp:904-908`); семантика — «після збою видно префікс групи або нічого з неї» (**перевірити** в харнесі: T-ES-23).

### 3.6 Відновлення (`open` → `recover`)

```
pick: валідні (magic, crc15) суперблоки; беремо більшу генерацію; sb_fits
if anc != 0: PartTab crc ok && crc(arena[mark..used)) == anc  → OK
else:        обмежений обхід арени від курсора ІНШОГО валідного суперблока (st_reopen_lo-правило) або від 1024
if fail:     інвалідація цього суперблока В ПАМ'ЯТІ (не на диску — бібліотека-читач не пише; `repair` утиліта пише, як st_reopen_verify) → pick іншого → ПОВНИЙ обхід fallback
if both fail: Refusal::NoSuperblock{a: why, b: why}
after pick:  курсор = used з PartTab; усе за курсором — сміття, наступний коміт його перепише
```

Відмінність від bebop (`store.bp:188-204`): bebop затирає magic у MAP_SHARED-відображенні прямо з читача; бібліотечний читач цього робити **не повинен** (read-only mmap, можливо чужий файл) — тому інвалідація в пам'яті + явний `repair()` у письменника.

### 3.7 Версіонування формату й міграції

- Суперблок комірка 1 = версія формату контейнера (нині 2 скрізь: `lib.rs:291`, `store.bp:83`). Комірки 13–14 **мусять бути нулем** за S-1 (oracle.py відмовляє), тож бітова маска можливостей **не може** з'явитися без підняття версії. Пропозиція: версія 2 лишається «те, що є», версія 3 = «обов'язковий `anc` + комірка 13 = features bitmask (біт 0: ключі довільні байти; біт 1: 64-бітна довжина об'єкта…)». Читач v3 читає v2; письменник пише ту версію, яку має образ (правило з `kv.rs:20-25`), підняття — лише при компакції.
- Розкладки (KV v1/2/3, EvLog v1/2) версіонуються своїм полем у корені — як сьогодні; невідома версія → `UnknownVersion`.
- Міграція = тільки при компакції (Cheney-копія, `LANG-DB-DESIGN.md:233-252`); у бібліотеці немає «міграції на місці».
- Нормативний документ формату `docs/FORMAT-BEBOP-IMAGE-v2.md` — відсутній сьогодні, стає поставкою P0 (гейт: oracle.py читає тільки за документом — він це вже заявляє, `oracle.py:4-7`).

### 3.8 Ліміти (запропоновані, треба зафіксувати як константи з тестами на межі)

| Ліміт | Значення | Звідки |
|---|---|---|
| Об'єкт | ≤ 2³²−1 комірок (поле len у `h0`) | `lib.rs:16` |
| Генерацій | ≤ 2³²−1 (нижні 32 біти `h1`) | D7 |
| Ключ | ≤ 65 535 байт | як fjall; `base_lens` уже обрізає на 1<<16 (`write.rs:209`) |
| Значення | ≤ 1 GiB | обмежено `Vec` в одному об'єкті |
| Образ у пам'яті (`Store`) | ≤ 512 MiB | `MAX_PAD_CELLS`, `lib.rs:76` |
| Образ через `MmapRoDevice`/`FileDevice` | ≤ 2⁴⁰ байт (офсети `i64` у комірках) | формат; **практично не міряно** |
| Ланцюжок дельт | ≤ 32 до компакції (параметр) | `delta.rs:50` |
| Ключів у KV v3 | **рекомендовано ≤ 10⁵** (компакція O(n) кожні 32 записи) | §1.5 п.5; більше — P8 |

### 3.9 Модель помилок

Усі відмови — іменовані варіанти `Refusal` з даними (комірка, want/got, claimed/have). Правило коду: жодного індексування зрізу за числом з образу без `get`/`checked_*` (сьогодні виконано вручну; стає гейтом `T-ES-06`: `#![deny(clippy::indexing_slicing)]` у `bebop-image`/`bebop-kv` + `forbid(unsafe_code)`). Помилка `fdatasync` → `Refusal::Io` **і** письменник переходить у стан `Poisoned`: наступний `begin_write` → `Refusal::Poisoned`, поки не зроблено `reopen()` (урок ATC'20).

### 3.10 C ABI і Python

- `bebop-ffi`: `bebop_open(path, opts*) -> *Db`, `bebop_get(db, k, kl, out*, outl*) -> status`, `bebop_begin/put/remove/commit/abort`, `bebop_scan_*`, `bebop_verify`, `bebop_last_refusal(db, buf, len)` — коди статусу = ті самі, що в `bebop-wasm/src/abi.rs` (один словник статусів для wasm і C).
- Python: тонкий `ctypes` модуль у `bebop-ffi/python/` (не біндінг-крейт, без PyO3 — нуль залежностей). `oracle.py` **не** стає обгорткою: він лишається незалежним читачем, інакше чотири читачі стануть трьома.

### 3.11 Що виїжджає з dowiz, що лишається

| Переїжджає в бібліотеку | Лишається в dowiz / bebop-lang |
|---|---|
| `lib.rs` формат, `crc.rs`, `verify.rs` (без `EvLog`-частини → `bebop-log`), `kv.rs`, `kv/*`, `evlog.rs`, `evlog/subject.rs` (як generic prefix-scan по заголовках), тести з названою коміркою | `dowiz-hub/src/{catalog,kvload,logimage,log,stock,settings,roster,post,table,quarantine,usage}.rs`, `Catalog.dirty` (стає `WriteTxn`), `hub_image`/`Meta`/чанки/W-ATOMIC, `proj.rs`+`nodekey.rs` (DAG-рантайм — у `bebop-lang` або окремий `bebop-proj`), `bebop-wasm/decide*` |
| golden-фікстури `bebop-wasm/fixtures/kv*.store` як конформаційний набір | `block.rs` (DG7/DG9 колонковий блок — це розкладка dowiz; можна винести пізніше як `bebop-block`) |

---

## 4. Специфікації: критерії прийняття й тести

Нумерація: `AC-ES-NN` — критерій; `T-ES-NN` — тест або гейт, який його доводить. Кожен тест детермінований; **жодних фаззерів** — лише названі комірки й **перелічені** точки збою (правило оператора `no-fuzzers-in-this-repo.md`; див. Q1).

### 4.1 Формат і читач (`bebop-image`)

- **AC-ES-01** Образ, записаний будь-яким із письменників (Rust v2/v3, bebop `kv.bp`, компакція), читається `bebop-image` до того самого `(generation, root, arena_used)` і того самого `fold_root`, що й oracle.py і wasm-читач. — **T-ES-01**: `gate.sh` розширено п'ятим образом (новий API, v3 з 7 дельтами) і п'ятим читачем (C ABI через ctypes); «4/4» стає «5/5», skip не рахується згодою.
- **AC-ES-02** Усі довжини/зміщення/лічильники з образу обмежуються образом; жодного `panic`/`abort` на будь-яких байтах. — **T-ES-02**: каталог із 24 названих пошкоджень по одній комірці (magic A, crc15 A, gen B > A з used поза образом, `pt+18` від'ємний, `pt+19` позаду `arena_used`, `pt+19` за ареною, `h0.len` з бітом 33, `h1.crc` ±1, `ref` на себе, `ref` назад (петля), KIDX off за межею blob, VERSION=4, D на 1 більше/менше, OP=3, VLEN≠0 при remove, порожній образ, 8 байт, `MIN_BYTES-8`, обрізання на 1 комірку нижче `used`, обрізання на півкомірки, capacity абсурдна) — кожне дає **названу** `Refusal`, тест названо за коміркою. Існуючі 10 тестів (`kv/tests.rs`, `kv/delta/tests.rs`, `verify/tests.rs`) входять сюди.
- **AC-ES-03** PartTab перевіряється crc при `pick` (закриває D8). — **T-ES-03**: біт у `pt+19` → `Refusal::PartTabBad{at}` ще до `begin`.
- **AC-ES-04** Новіший валідний суперблок, що не вміщується, = відмова, не fallback (збережено). — **T-ES-04**: наявний `a_cut_one_cell_short_is_refused_not_read_as_the_previous_generation` (`lib.rs:961-974`) переноситься.
- **AC-ES-05** `#![forbid(unsafe_code)]`, `#![no_std]` + `alloc`, нуль залежностей у `bebop-image`/`bebop-kv`/`bebop-log`. — **T-ES-05**: CI-крок `cargo build --target thumbv7em-none-eabi` (або `wasm32-unknown-unknown` без std) + `cargo tree -e no-dev | wc -l == 1` для кожного.
- **AC-ES-06** Жодного індексування зрізу значенням з образу. — **T-ES-06**: `clippy::indexing_slicing` як `deny` у трьох крейтах; винятки лише в `#[cfg(test)]`.
- **AC-ES-07** `View` і `Store` відповідають однаково на всіх образах T-ES-02. — **T-ES-07**: `view_tests::the_view_and_the_store_answer_alike` над усім каталогом.
- **AC-ES-08** Нормативний документ формату v2 існує і oracle.py читає тільки за ним. — **T-ES-08**: ревʼю-гейт: кожна константа документа (`MAGIC`, 512, 1024, 16, 19, зміщення PartTab) має тест, який її виводить (`derive`-тест, як `digests_are_st_digest_of_their_layouts`).

### 4.2 Письменник, транзакції, ізоляція (`bebop-kv`, `bebop-io`)

- **AC-ES-10** `WriteTxn` бачить власні put/remove (`get`/`scan`), `ReadTxn`, відкритий до коміту, не бачить їх до/після. — **T-ES-10**: три txn у визначеному порядку; перевірка `generation()` читача не змінюється.
- **AC-ES-11** Другий `begin_write` у процесі → `WriterBusy`; другий процес → `Locked{by}` через lock-файл. — **T-ES-11**: тест із `std::process::Command` на власний бінарник (один форк, детерміновано).
- **AC-ES-12** `abort` і падіння процесу до коміту не змінюють жодного байта, досяжного зі старого суперблока. — **T-ES-12**: порівняння `snapshot_bytes()` до/після аборту; після симульованого збою (T-ES-20) — те саме.
- **AC-ES-13** Коміт = або генерація k+1 з усіма операціями, або k без жодної (атомарність на рівні `pick`). — **T-ES-13**: харнес §4.3, інваріант «replay(k+1) = replay(k) + ops або = replay(k)».
- **AC-ES-14** Після `fdatasync`-помилки письменник `Poisoned`; наступний `begin_write` відмовляє; `reopen` відновлює через `recover`. — **T-ES-14**: `FaultDevice` з ін'єкцією `EIO` на 1-му/2-му бар'єрі (дві названі точки).
- **AC-ES-15** Компакція у новий пристрій + publish не змінює `fold_root`, `len`, порядок ключів; старий `ReadTxn` дочитує старе покоління. — **T-ES-15**: наявний `lean_tests.rs` + читач, відкритий до компакції.
- **AC-ES-16** Ліміти §3.8 відмовляють іменовано на межі й приймають на межі−1. — **T-ES-16**: по одному тесту на ліміт (ключ 65 535/65 536; значення на межі — через `MemDevice` з обмеженою capacity, не реальним 1 GiB).
- **AC-ES-17** Rust-письменник пише `anc`; bebop `st_reopen_verify` відкриває Rust-образ шляхом якоря (не обходом). — **T-ES-17**: образ із Rust → `kv.bin` через `st_open_verify_all` і `st_open`; обидва дають той самий fold; у bebop-лічильнику обходу (додати повернення `n` з `st_verify_range` у лог) — 0 об'єктів пройдено при `anc != 0`. **UNVERIFIED**, чи `kv.bp` відкриває через `st_open` (треба прочитати `kv.bp`).
- **AC-ES-18** Групове комітування: після збою видно префікс групи. — **T-ES-18**: харнес з N=3 комітами `Durability::None` + `sync()`; усі точки збою перелічені.

### 4.3 Crash-recovery харнес (перелічений, не фаззований)

`SimDisk: Device` записує кожну операцію як `Write{sector, bytes}` або `Barrier`. Модель збою (ALICE-клас, DFSCQ-семантика):

- Диск = масив секторів по 512 B. Запис довжиною L покриває ⌈L/512⌉ секторів; кожен сектор пишеться атомарно **або** рветься на дві половини (режим `Torn`: перша половина нова, друга стара).
- Між двома бар'єрами записи — множина **W**; після збою кожен сектор із W незалежно або старий, або новий (persisted subset). До останнього бар'єра — все нове.
- **Перелічення**: для одного коміту `Durability::Data` маємо вікно A = {об'єкти, PartTab} (до бар'єра 1) і вікно B = {SB'} (до бар'єра 2). Точки збою: (i) до будь-якого запису; (ii) усередині вікна A: кожна підмножина секторів A (для коміту з ≤ 6 секторами це ≤ 64 станів) × {ціло, torn на кожному секторі}; (iii) після бар'єра 1, усередині B: {SB старий, SB новий, SB torn}; (iv) після бар'єра 2. Для тестового коміту з 3 дельтами (≈ 5 секторів) це **≤ 64·6 + 3 + 2 ≈ 400 станів**, кожен із порядковим номером у назві провалу. Для великого коміту (120 секторів) підмножини обмежуються **названими класами**: «усе крім першого», «усе крім останнього», «лише перший», «лише останній», «парні», «непарні», «порожньо», «усе» — 8 класів × torn-позиція ∈ {перший, останній} — і це записується як явне обмеження харнесу, не як «випадкові 1000».
- **Інваріант у кожному стані**: `recover()` повертає `Ok` з генерацією k або k+1; `replay(стан) ∈ {replay(k), replay(k)+ops}`; `verify(WholeArena)` на обраній генерації — `Ok`; наступний коміт із цього стану успішний і дає k+1 або k+2.
- **T-ES-20** харнес над `commit(Data)` для чотирьох сценаріїв (перший коміт у свіжий образ; дельта на v3; компакція-у-нове + rename; видалення останнього ключа). **T-ES-21** те саме з `Durability::None` — очікувано **знаходить** стани з генерацією k−1 при бар'єрі тільки в кінці (харнес має вміти провалитися: RED-доказ як у `gate.sh --prove`). **T-ES-22** торн-запис суперблока → завжди попередня генерація. **T-ES-23** група з 3 комітів (AC-ES-18). **T-ES-24** `FileDevice` над реальним файлом: той самий тест, але з «краш» = відмова писати далі після точки N (без kill — процес-краш на f2fs nobarrier нічого не доводить) + окремо `scrash.sh`-подібний SIGKILL-прогін **лише як димовий**, не як доказ.

Якщо оператор вважає перелічення підмножин «фаззером» — див. Q1; запасний варіант — лише 8 названих класів у всіх випадках.

### 4.4 Кросреалізаційна конформація

- **AC-ES-30** П'ять читачів (bebop `kv.bp`, `bebop-image` native, wasm32, oracle.py, C ABI) дають один `fold_root` і одну відмову (той самий код статусу) на кожному з фікстурних образів: `kv.store` (v1), `kv2.store` (v2), новий `kv3.store` (v3, 7 дельт у 3 комітах, один remove), `kv3c.store` (після компакції), `torn-sb.store` (SB' зіпсовано → генерація k), `torn-pt.store` (PartTab зіпсовано, `anc` ok → відмова суперблока, генерація k). — **T-ES-30**: `gate.sh` з таблицею `образ × читач × очікуване`; відсутній читач = «NOT MEASURED», не згода.
- **AC-ES-31** Письменники теж конформні: образ із `bebop-kv` після 3 комітів байт-у-байт дорівнює образу з `kv.bp` для тих самих ops (окрім `h1.generation`-полів? — ні: вони теж детерміновані). — **T-ES-31**: golden із SHA-256 двох образів; розбіжність називає першу комірку. **UNVERIFIED**: чи `kv.bp` сьогодні пише дельти (у `delta.rs:13-14` сказано, що kv.bp навчили *читати* v3; писати — не сказано). Якщо ні — критерій звужується до «bebop читає Rust-образ», а повна двостороння конформація = окрема bebop-поставка.
- **AC-ES-32** Статуси відмов — один словник (`abi.rs` ↔ C ABI ↔ oracle.py). — **T-ES-32**: таблиця в документі формату + тест, що перелічує `Refusal` і коди.

### 4.5 Lean-поставки

- **AC-ES-40 (L1, байтовий replay)**: у `formal/Bebop/KvBytes.lean` визначено `decodeV2 : Image → Option (List (Bytes × Bytes))`, `chain : Image → Nat → Option (List Rec)`, `readV3` і доведено `readV3 (encodeV3 base deltas) = KvDelta.replayFrom (ofList base) deltas` для довільних base/deltas, що задовольняють ліміти; функції отримані **з Rust через Aeneas** або транскрибовані з `kv/decode.rs`, `kv/delta.rs:123-162` із Rust-twin-тестом (як `lean_tests.rs:1-14`), який прибиває транскрипцію до коду на фіксованому наборі образів. — **T-ES-40**: `lake build` без `sorry`, `lean.sh` census не додає аксіом (поточний `lean.axioms`: 31).
- **AC-ES-41 (L2, атомарність коміту)**: `formal/Bebop/Disk.lean` — модель диска §4.3 (сектори, вікна між бар'єрами, torn), `commitProtocol : State → List Op` (п'ять кроків §3.5), `crash : List Op → Set Disk`, `recover : Disk → Option Gen`, теорема `∀ d ∈ crash (commitProtocol s), recover d ∈ {some k, some (k+1)} ∧ view (recover d) ∈ {view k, apply ops (view k)}`. Це **замінює** `sys_write_spec`/`sys_fsync_spec`/`sys_msync_spec` для стору конкретною моделлю; аксіоми лишаються для решти мови. — **T-ES-41**: той самий харнес T-ES-20 читає перелік станів із Lean-екстракту (`#eval crash …` у малому екземплярі) і порівнює з переліком SimDisk — модель і харнес мусять перелічити **одну й ту саму множину** (взаємний контроль).
- **AC-ES-42 (L3, відносна несуперечливість аксіом)**: `formal/Bebop/SyscallsModel.lean` — конкретний `State` і реалізація кожного з 26 `sys_*`, для яких усі 26 специфікацій доведені як теореми (`theorem sys_fsync_spec_holds : … := ⟨0, s, …⟩`). Оскільки специфікації вимагають лише «результат ∈ {0, -errno} ∧ арена незмінна», тривіальна модель їх задовольняє — і це **сам по собі результат**: аксіоми несуперечливі, але **нічого не кажуть про файл** (чесно записати в README формального каталогу). — **T-ES-42**: `lean.sh` проходить; у `lean.axioms` рядків не додано; `results.txt` оновлено.
- **AC-ES-43** Жодна Lean-поставка не є load-bearing для гейтів бібліотеки (ROADMAP-теза: Lean — крос-оракул). — **T-ES-43**: `gate.sh`/харнес/конформація зелені без встановленого Lean.

### 4.6 Продуктивність

Бенчмарк `es-bench` (criterion 0.5.1 — є в кеші реєстру; rusqlite 0.32 — є; **redb/heed(LMDB) у кеші немає → потрібна мережа, Q8**). Чотири ряди, по одному числу кожен, на двох машинах (цей бокс і x86 CI-раннер; бокс під proot має ~273 µs на syscall — `RESEARCH-NOPOINTERS-SQL`, тож ряди з fsync тут нерепрезентативні й позначаються):

| Ряд | Набір | Ціль (RTM = відношення до найкращого з redb/LMDB/SQLite на тій самій машині) |
|---|---|---|
| B1 point get, гаряче | 10⁴ ключів × 256 B, 10⁶ випадкових get | ≤ 1.5× RTM (сьогодні zero-copy 1.5 µs на re-read, `f8d26eae`) |
| B2 put без fsync (`None`/`synchronous=OFF`/`MDB_NOSYNC`) | 10⁵ put по 256 B, батчі по 32 | ≤ 3× RTM (компакція кожні 32 — чесно врахувати) |
| B3 durable commit | 10³ комітів по 1 put, `Data`/`FULL`/`MDB_SYNC` | ≤ 1.2× RTM (2 бар'єри проти 1–2 у них) |
| B4 open+verify після 10⁵ комітів | — | ≤ 2 ms з якорем на x86 (O(останній коміт)); без якоря — назване число обходу |
| B5 scan 10⁴ ключів за префіксом | — | ≤ 1.5× RTM |

- **AC-ES-50** Усі п'ять рядів виміряні на обох машинах і записані в `docs/measurements/` з командою; жодна ціль не «виконана» без числа конкурента з тієї самої машини. — **T-ES-50**: `bench_track.py`-подібний гейт на регрес ≥ 10 % проти baseline по B1/B4/B5 (детерміновані); B2/B3 — лише звіт.
- **AC-ES-51** Пам'ять читача: `ReadTxn::get` через `MmapRoDevice` не алокує понад розмір значення. — **T-ES-51**: `count-allocs`-лічильник (є у kernel як feature) або `GlobalAlloc`-обгортка в тесті.

---

## 5. Фазовий план

Розміри — дні одного Opus-лейну; на боксі ≤ 3 лейни (`DOWIZ-COMMON-RULES`), heavy через `slot.sh`. Файлова роз'єднаність вказана явно.

| Фаза | Зміст | Вихідний критерій | Розмір | Залежить від | Файли |
|---|---|---|---|---|---|
| **P0 Аудит і заморозка формату** | документ формату v2 (§3.7), закрити D1–D9 (anc у Rust-письменнику, crc PartTab у `pick`, `create` з fsync, виправити коментарі), ліміти як константи, каталог T-ES-02 (24 названі комірки) | T-ES-02/03/08 зелені; `gate.sh` 4/4; `run-all` зелений; dowiz-hub без змін поведінки (golden `kv/golden_tests.rs` незмінний) | 2–3 | — | `crates/bebop-store/src/{lib,verify,proj}.rs`, `docs/FORMAT-*.md` |
| **P1 Розділ на крейти** | `bebop-image`/`bebop-kv`/`bebop-log`/`bebop-io` як окремі каталоги `crates/`; `bebop-store` стає фасадом-реекспортом (щоб `dowiz-hub`, `bebop-wasm`, `kernel` не мінялися в тому самому коміті); `Device` трейт; `no_std` для трьох | T-ES-05/06/07; wasm ratchet `bytes.baseline` не росте (або ріст названий і прийнятий); 107 тестів переїхали й зелені | 4 | P0 | нові каталоги + `bebop-store/src/lib.rs` (фасад) |
| **P2 Протокол коміту v2 + відновлення + харнес** | `FileDevice` (pwrite+fdatasync, 2 бар'єри, `anc`), `recover()` §3.6, `SimDisk`, перелічений харнес §4.3, `Poisoned`, lock-файл | T-ES-12/13/14/17/20–24; T-ES-21 доводить, що харнес уміє червоніти | 4–5 | P1 | `bebop-io/*`, `bebop-image/src/recover.rs`, `bebop-kv/tests/crash/*` |
| **P3 Публічний API** | `Db`/`ReadTxn`/`WriteTxn`/`scan`/`compact(into)`/`verify(depth)`, `Durability`, `Refusal`, ключі `&[u8]`; `Catalog.dirty` → `WriteTxn`; `dowiz-hub` переходить на API (але DO-адаптер лишається) | T-ES-10/11/15/16; dowiz `run-all` зелений; flows на qa-durres у Playwright (правило 10-12) | 4 | P1 (паралельно з P2 до злиття) | `bebop-kv/src/{db,txn,scan}.rs`, `dowiz-hub/src/catalog*.rs`, `kvload.rs` |
| **P4 Конформація** | фікстури v3/компакція/torn, `gate.sh` 5 образів × 5 читачів, оновити oracle.py і `kv.bp`-читач, словник статусів | T-ES-30/32; AC-ES-31 — у межах того, що `kv.bp` уміє писати | 3 | P2, P3 | `bebop-wasm/{gate.sh,oracle.py,fixtures}`, `selfhost/std/kv.bp` (читач) |
| **P5 Lean L1–L3** | `KvBytes.lean`, `Disk.lean` + теорема атомарності, `SyscallsModel.lean`; спроба Aeneas на `bebop-image` (якщо не піде — транскрипція + twin-тест) | T-ES-40/41/42/43; `lean.sh` зелений; `lean.axioms` без нових рядків | 5–6 | P2 (модель мусить збігатися з харнесом) | `bebop-lang/formal/Bebop/*.lean`, `bebop-kv/src/kv/delta/lean_tests.rs` |
| **P6 FFI + Python + документація** | `bebop-ffi` cdylib, ctypes-модуль, README-и п'яти крейтів, приклади, `CHANGELOG`, ліцензійні заголовки (Q3) | T-ES-01 (5-й читач), `cargo doc` без warnings, приклад `examples/kv.rs` і `examples/kv.py` запускаються в CI | 3 | P3 | `crates/bebop-ffi/*` |
| **P7 Бенчмарк** | `es-bench` §4.6, обидві машини, baseline, регрес-гейт | T-ES-50/51; таблиця в `docs/measurements/` | 3 | P3; мережа для redb/heed (Q8) | `crates/es-bench/*`, `.github/workflows/*` |
| **P8 (опційно) Індекс для >10⁵ ключів** | або багаторівневі відсортовані прогони (L0 дельти → L1 прогони → база; компакція рівнями), або COW B+tree поверх об'єктів | B1/B2/B5 на 10⁶ ключів ≤ 3× RTM; усі читачі навчені (ще одна версія розкладки) | 6–10 | P3, P4, рішення Q2 | `bebop-kv/src/levels/*`, усі читачі |

**Разом без P8:** 28–31 днів послідовно. **Паралельно (3 лейни, файлово роз'єднані):** хвиля 1: P0 (один лейн, 2–3 д; решта чекає, бо все залежить від заморозки); хвиля 2: P1 (один лейн; ділить файли з усім — інші чекають 4 д); хвиля 3: P2 ∥ P3 ∥ P5-підготовка (Lean-модель диска не чіпає Rust) — 5 д; хвиля 4: P4 ∥ P6 ∥ P5-завершення — 5–6 д; хвиля 5: P7 — 3 д. ≈ **19–21 календарних днів** при ідеальному злитті; реалістично з merge-паузами — 4 тижні.

**Ризики.** (R1) Aeneas не перетравлює `Vec<i64>`-арифметику → L1 транскрипцією (вже є прецедент `lean_tests.rs`), +2 д. (R2) P1 ламає wasm-ratchet через реекспорт (прецедент: +507 B від делегування через трейт, `lib.rs:86-89`) → фасад мусить реекспортувати, не обгортати. (R3) `kv.bp` не пише v3 → AC-ES-31 односторонній (сказано). (R4) Розрив поведінки хаба при переїзді `Catalog.dirty` всередину txn → тримати golden-образи `catalog/delta_tests.rs` незмінними. (R5) Бокс: кожен `cargo test` нового крейту — компіляція; `-j2` уже в `~/.cargo/config.toml`; лейни з паралельною компіляцією пауза перед `run-all` (`runall-killed-by-lane-threads.md`). (R6) Публікація на crates.io потребує імен (`bebop-*` можуть бути зайняті — **UNVERIFIED**) і ліцензії (Q3).

---

## 6. Чесна частина: що є поганою ідеєю, і що не перевірено

**Погані ідеї (рекомендую не робити):**
1. **Змагатися з LMDB/redb за throughput як головну ціль.** Ряд B2 структурно програє (компакція O(n) кожні 32 записи), і це не лікується без P8; цінність бібліотеки — не швидкість, а (а) формат, який читають п'ять незалежних реалізацій і один Python-файл на 300 рядків, (б) відмова замість паніки на будь-яких байтах, (в) машинно-перевірений протокол. Так і позиціонувати.
2. ~~Multi-writer / MVCC у сенсі паралельних письменників.~~ **ВІДКЛИКАНО 2026-10-29** (оператор: «писачів має бути багато, не розумію звідки рішення лише про одного»): рішення «один письменник» ніколи не виносилось оператору; COW-дерево P8 робить оптимістичні паралельні транзакції природними — див. P19/P20 у §12.
3. **MAP_SHARED-запис у бібліотеці.** Після помилки `fsync` не можна знати, що на диску (ATC'20); bebop-код хай лишається таким, як є (він — ще один конформний письменник), але `bebop-io` пише `pwrite`.
4. **C ABI до стабілізації Rust API** — друга поверхня, яку доведеться міняти; тому P6 після P3.
5. **«Верифікована» в маркетингу раніше, ніж L2 доведено над байтами.** Сьогодні доведено лише абстрактну карту; слово «verified» на README без L1+L2 — та сама помилка, що «Lean builds clean» при суперечливих аксіомах (`bebop-lean-never-compiled.md`).
6. **Викладати бібліотеку під AGPL-3.0** (ліцензія `bebop-wasm/Cargo.toml:5`), якщо мета — «щоб люди поза dowiz користувалися»: вбудовані сховища під AGPL практично не вбудовують.

**Частина цілі, яка є сумнівною взагалі:** ринок вбудованих KV зайнятий зрілими системами з десятиліттями crash-тестування (SQLite має ~100 % MC/DC і симульовані збої — https://www.sqlite.org/testing.html). Самостійна бібліотека виправдана як **спосіб затвердити власну базу dowiz** (стандартний API, харнес, доказ, п'ять читачів) із зовнішньою поверхнею «на здачу» — не як продукт, що має когось витіснити. Якщо оператор очікує саме продукту, треба сказати це прямо й планувати P8 і багатопроцесність як обов'язкові, а не опційні.

**Не перевірено (UNVERIFIED) у цьому документі:** чи `kv.bp` пише v3 (лише читає — за `delta.rs:13-14`); чи є тест на біт у `pt+19` у Rust; вирівнювання суперблока/PartTab щодо 512-байтових секторів на конкретних ФС; Aeneas на цьому коді; наявність імен `bebop-*` на crates.io; числа з `RESEARCH-BEAT-SQL`/R-BEBOPDB (SQLite put 146–261× швидше за компакційний put — виміряно 2026-10-05 іншим агентом, мною не відтворено); чи `st_open` викликається з `kv.bp` (для T-ES-17); чи CI x86 має доступ до мережі для redb/heed.

---

## 7. Рішення, які має ухвалити оператор

| # | Питання | Моя рекомендація |
|---|---|---|
| **Q1** | Чи є **перелічений** crash-харнес (§4.3: підмножини секторів одного коміту, ≤ ~400 станів із номером у назві) порушенням правила «без фаззерів»? | **Ні**: кожна точка збою названа, детермінована, без генератора й без тисяч ітерацій; це CrashMonkey-клас, не cargo-fuzz. Якщо «так» — обмежитися 8 названими класами на коміт (теж у §4.3). |
| **Q2** | Індекс для масштабу (P8): багаторівневі відсортовані прогони чи COW B+tree? D.3 звіту R-BEBOPDB відкинув B+tree «при одній венуї на образ»; для зовнішньої бібліотеки масштаб інший. | **Спершу прогони** (узагальнення того, що є; усі читачі переживають це як ще одну версію), B+tree — лише якщо ціль ≥ 10⁶ ключів із випадковими put. Рішення після P7, з числами. |
| **Q3** | Ліцензія та імена крейтів для публікації (`bebop-wasm` — AGPL-3.0). | MIT OR Apache-2.0 для `bebop-image/-kv/-log/-io/-ffi`; AGPL лишити для продукту. Імена перевірити на crates.io перед P1. |
| **Q4** | Багатопроцесний доступ (один письменник + читачі з інших процесів) — у межах цієї роботи чи пізніше? | Пізніше: фаза 1 = один процес; `Locked{by}` + `Superseded` закладені в API, щоб додати без зламу. |
| **Q5** | Писати `anc` у Rust-письменнику негайно (P0; сумісно: 0 і ≠0 читають усі) і чи піднімати версію суперблока до 3 заради features-маски у комірці 13 (S-1 нині вимагає нуль)? | `anc` — так, у P0. Версію 3 — **не зараз**; потреба у масці з'явиться лише з P8. |
| **Q6** | Чи входять `bebop-log` (EvLog) і `proj`/`nodekey` до публічної бібліотеки? | `bebop-log` — так (друга розкладка, окремий крейт); `proj`/`nodekey` — ні (DAG-рантайм dowiz/bebop-lang). |
| **Q7** | Пріоритет Lean: L2 (атомарність під моделлю диска, нове) перед L1 (байтовий replay)? | Так: L2 закриває єдину недоведену гарантію, L1 підтверджує те, що вже прибито golden-ами; L3 — дешевий, робиться разом із L2. |
| **Q8** | Дозвіл на мережу для `redb`/`heed` у бенчмарку (їх немає в кеші реєстру; SQLite і criterion є). | Так, у P7; інакше ряди B1–B5 лише проти SQLite. |
| **Q9** | bebop-ова реалізація (`store.bp`, MAP_SHARED, `msync`) лишається як є — «п'ятий конформний письменник», не переписується під протокол §3.5? | Так; конформація T-ES-30/31 тримає їх разом; `st_commit_sync` уже має той самий інваріант порядку. |
| **Q10** | Чи це продукт («витіснити LMDB») чи затвердження власної БД із зовнішньою поверхнею? | Друге. Якщо перше — P8 і Q4 стають обов'язковими, бюджет ×1.7, і треба окремий лейн на документацію/підтримку поза dowiz. |

---

## Додаток A. Джерела

Код (усе перевірено читанням 2026-10-27): `crates/bebop-store/src/{lib,kv,crc,verify,evlog,proj}.rs`, `kv/{delta,zc,decode}.rs`, `kv/delta/{write,lean_tests}.rs`, `kv/zc/{image,overlay}.rs`, `verify/since.rs`; `crates/bebop-wasm/{Cargo.toml,gate.sh,oracle.py,src/lib.rs}`; `crates/dowiz-hub/src/{catalog,kvload,logimage,lib}.rs`; `workers/api/src/hubdo.rs`, `hubdo/{atomic,store}.rs`; `bebop-lang/selfhost/prelude/store.bp`, `selfhost/std/scrash.bp`, `bench/vs_rust/scrash.sh`; `bebop-lang/formal/Bebop/{KvDelta,Syscalls}.lean`; `tools/gates/{lean.sh,lean.axioms}`; `bebop-lang/docs/{LANG-DB-DESIGN,RESEARCH-BEAT-SQL-2026-09-08,RESEARCH-NOPOINTERS-SQL-2026-09-06,CERTIFICATE-FORMAT}.md`, `bebop-lang/ROADMAP.md`; `docs/research/2026-10-05-bebop-db-gaps-strengths-tensor-graph.md` §D.3; нотатки пам'яті, перелічені в завданні.

Зовнішні: LMDB https://www.openldap.org/pub/hyc/mdb-paper.pdf · SQLite https://www.sqlite.org/atomiccommit.html , https://www.sqlite.org/wal.html , https://www.sqlite.org/psow.html , https://www.sqlite.org/testing.html · bbolt https://github.com/etcd-io/bbolt · redb https://github.com/cberner/redb , https://docs.rs/crate/redb/0.6.1/source/docs/design.md · sled https://github.com/spacejam/sled · fjall https://github.com/fjall-rs/fjall · Badger https://github.com/dgraph-io/badger · Kyoto https://dbmx.net/kyotocabinet/ · ALICE https://www.usenix.org/conference/osdi14/technical-sessions/presentation/pillai · CrashMonkey https://www.usenix.org/conference/osdi18/presentation/mohan · fsync failures https://www.usenix.org/conference/atc20/presentation/rebello · fsyncgate https://lwn.net/Articles/752063/ · FSCQ https://pdos.csail.mit.edu/papers/fscq:sosp15.pdf · DFSCQ https://pdos.csail.mit.edu/papers/dfscq:sosp17.pdf · GoJournal/Perennial https://www.usenix.org/conference/osdi21/presentation/chajed · VeriBetrKV https://www.usenix.org/conference/osdi20/presentation/hance · Armada https://www.microsoft.com/en-us/research/publication/armada-low-effort-verification-of-high-performance-concurrent-programs/ · PoWER/CapybaraKV https://people.csail.mit.edu/nickolai/papers/leblanc-power.pdf · Aeneas https://lean-lang.org/use-cases/aeneas , https://www.microsoft.com/en-us/research/publication/aeneas-rust-verification-by-functional-translation/ · Verus https://github.com/verus-lang/verus · Kani https://github.com/model-checking/kani · Creusot https://github.com/creusot-rs/creusot

---

## 12. Фази швидкості (додано 2026-10-29 на рішення оператора «добавити усе в план, чого бракує, щоб зробити швидшим»)

Звідки: виміри `R-STORE-VS-SQL-2026-10-27.md` (bebop проти SQLite на боксі), `R-BINREC-2026-10-28.md` §1–§8 (склад байтів, сегмент-прототип), лейни 2026-10-28/29 (W-KVPERF, W-BLOCKVIEW, LV-INC-A, LV-PCD-C, W-SPACE). P8 з «опційної» стає **обов'язковою**. Розміри — дні одного лейну, як у §5. Кожна фаза — критерій із числом проти конкурента або контролю з того самого процесу.

| Фаза | Що бракує сьогодні (виміряно) | Зміст | Вихідний критерій | Дні | Залежить від |
|---|---|---|---|---|---|
| **P8 Індекс для запису (обов'язкова)** | запис 11–19× повільніший за SQLite; mean durable put на 1e6 ≈ 127× (переписування всього образу кожні 32 зміни, `MAX_DELTAS`) | **COW-дерево на наявному DAG об'єктів** (персистентне B+tree або HAMT, розгалуження 64–256, вузли = звичайні bebop-об'єкти з відносними посиланнями): запис = O(log n) нових вузлів, решта спільна з попереднім поколінням; знімки = корені; перевірка лише нових вузлів. Запасний варіант — рівневі відсортовані прогони (LSM), якщо читання через дерево не вкладеться в B1 ≤ 1.5× RTM (оператор 2026-10-29: «а як же графова структура і dag») | B2 ≤ 3× RTM на 1e6; durable put mean ≤ 5× медіани; усі 4 читачі вміють нову розкладку; старі образи читаються | 6–10 | P3, P4, Q2 |
| **P9 Інкрементальна компакція** | компакція = O(образу) | зливати лише зачеплені прогони/сторінки (частина рівневої схеми P8, але окремий критерій) | одна компакція ≤ O(розміру рівня), не образу; виміряно на 1e5/1e6 | 2–3 | P8 |
| **P10 Лінива перевірка при відкритті** | open+verify 183 мс проти 11 мс SQLite на 1e6 (crc усього образу) | `anc` (уже пишеться з P0) → відкриття перевіряє лише останній коміт; повна перевірка — явна (`verify(depth)`) або при відновленні | B4 ≤ 2 мс з якорем на x86; на боксі назване число; T-ES з корупцією старого коміту все одно ловиться `verify(Full)` | 2 | P2 |
| **P11 Груповий коміт з одним бар'єром** | 3 fsync на коміт (арена, PartTab, суперблок) | безпечний варіант з W-ES-P2 (спершу надійно k+N−1, потім сторінка надійного суперблока); пачка N комітів = 2 бар'єри, не 3N | B3 ≤ 1.2× RTM; харнес точок збою зелений для N=1..8; Lean-модель (Disk.lean) розширена на групу | 2–3 | P2, P5 |
| **P12 Апаратний CRC** | crc — slicing-by-8 у софті; домінує у verify і в W-SPACE pack (6.6 мс на 1.9 МБ у wasm32) | ARMv8 `crc32*` / x86 PCLMUL через `core::arch` з софт-запасом, або `crc32fast` (MIT/Apache) для байтових шляхів; wasm32 лишається софтом | verify 1e6 ×3+ швидше на нативі; байти crc незмінні (golden); 4 читачі згодні | 1–2 | — |
| **P13 Повторне використання місця** | арена ніколи не звільняється (50 комітів образу 71 КБ → 3.2 МБ); 9–15 % порожніх комірок | список вільних сторінок / GC поколінь при компакції, без порушення pointer-free і атомарності | ріст арени ≤ 2× живих даних при довгій серії комітів; харнес точок збою зелений | 3–4 | P8 |
| **P14 Шлях читання через mmap** | читач копіює образ у `Vec<i64>` перед відкриттям | `MmapRoDevice` (AC-ES-51 уже вимагає: get не алокує понад значення) | T-ES-51 зелений; B1 без копіювання на холодному відкритті | 2 | P2, P3 |
| **P15 Компактний формат запису (EVLOG v3 + запечатані сегменти)** | 421 Б/подію журналу; ~70 % стиснутого розміру — похідні хеші/id у кожному записі | корінь не копіюється в кожен коміт, `prev` не дублюється (EVLOG v3); архіви — колонкові сегменти по 512 подій без per-record id, один Merkle-корінь, zstd зі словником (R-BINREC §8: 12.9 / 9.3 Б/подію) | байти/подію виміряно на qa-durres; decode+verify сегмента ≤ 5 мс у wasm32; witness/backup якорі на корені сегмента; 4 читачі | 12–24 (S0–S5) | P2, migration-law для логу (W-P0-LOG) |
| **P16 Швидкий sha256 для Merkle-кореня** | корінь над 165 стравами 191 мкс проти цілі 20 (`nodekey::sha256` алокує й доповнює побайтово) | sha256 без алокацій, блоками; апаратні SHA-інструкції (ARMv8 SHA2, x86 SHA-NI) з запасом | корінь 165 листків ≤ 20 мкс на нативі; ті самі байти | 1 | — |
| **P18 Синк через різницю піддерев** | планшет синкає хвіст подій, не знаючи, які частини стану вже однакові | після P8: обмін коренями, далі лише піддерева з різними хешами (Merkle-diff); той самий механізм для бекапів-інкрементів | байти синку при 1 зміні з 1e4 ключів ≤ O(log n) вузлів; офлайн-flow F12 зелений | 3–4 | P8, P15 (хеші піддерев: sha256 з P16, якщо між незалежними сторонами) |
| **P19 Багато письменників: оптимістичні паралельні транзакції** | один письменник (рішення §6 цього документа, ніколи не виносилось оператору); оператор 2026-10-29: «писачів має бути багато» | будь-яка кількість `WriteTxn` над своїми знімками COW-дерева (P8), path-copy без блокувань; при коміті — перевірка read/write-множин проти комітів після знімка; неперетинні зміни зливаються (Z-set-дельти комутують), перетинні — іменована відмова `Refusal::Conflict` з повтором; послідовна лише заміна кореня (бар'єр, групується `Writer::flush`) | N письменників на неперетинних ключах: пропускна здатність ×≥0.7N до межі fsync; конфліктні — рівно одна перемагає, решта Conflict; харнес точок збою з 2–8 письменниками зелений; Lean: серіалізовність (кожна історія комітів еквівалентна послідовному порядку) | 8–12 | P8, P11, P3 |
| **P20 Кілька процесів-письменників** | lock-файл тримається весь час `Writer` | lock лише на час коміту (заміна кореня); інші процеси перечитують корінь і ребейзять транзакцію; виявлення застарілого lock за pid | 2+ процеси пишуть в один файл; харнес зі SIGKILL кожного процесу в кожній точці; T-ES-11b на x86 CI | 4–6 | P19, P2 |
| **P17 Типізований fold / бінарний payload** | serde_json 1.74 мкс/подію; прототип бінарного декодера 3.43 мкс — виграє лише з типізованим fold | бінарний payload разом із fold прямо в типізовані структури | лише коли холодний wake Worker наблизиться до 10 мс CPU; ціль ≤ 0.5 мкс/подію | 8–12 | P15 |

**Порядок (рекомендація):** P12 і P16 — одразу, паралельно з P1 (маленькі, не чіпають формат). P10 і P11 — разом із доробкою P2. P14 — разом із P3. Далі P8 → P9 → P13 (одна хвиля; з COW-деревом P9 і P13 здебільшого випадають самі: старі вузли лишаються в минулих поколіннях, а GC поколінь — це обхід досяжних вузлів від живих коренів) → P18 → P19 → P20. P15 — після P4 (потрібна конформація 5×5). P17 — відкладено до сигналу CPU.

**Разом (P8–P17):** ~39–61 день одного лейну; у 3 лейни ~15–25 днів. Без P15/P17: ~19–25 днів.

**Чого НЕ буде:** MAP_SHARED-запис у бібліотеці (§6 п.3). (Пункти «multi-writer» і «не змагатися за throughput» відкликано 2026-10-29: див. P19/P20 і §12.1.)


### 12.1 Ціль оператора 2026-10-29: «швидкість у пріоритеті; бібліотека має бути кращою за SQLite»

Це замінює формулювання §4.6 / §6 «не набагато гірше за конкурентів». Критерій готовності бібліотеки тепер: **кожен ряд B1–B5 і es-bench (get_hot, get_cold, prefix, scan, unsynced put, durable commit, delete, open+verify, bulk load) ≤ 1.0× SQLite на x86 CI-раннері** (той самий процес, ті самі дані, SQLite у найкращому для нього режимі: WAL, синхронність відповідна ряду), плюс ≤ 1.5× найкращого з redb/LMDB. Бокс під proot міряє лише відносні регреси.

Що з цього вже виконано (виміри 2026-10-27/29): get (×5–22 швидше), prefix scan, count O(1), cold get (0.41× найкращого), розмір бінарника, виявлення пошкоджень.
Що закриває решту, у порядку пріоритету: **P8** (COW B+tree, запис/видалення) → **P11** (груповий коміт) + новий **`Durability::Normal`** (аналог `synchronous=NORMAL`: fsync лише на контрольній точці, втрата після збою ≤ останні незакомічені на диск транзакції, але без розірваного стану — харнес точок збою доводить) → **P10** (ліниве відкриття) → P19/P20 (паралельні письменники — тут SQLite відстає принципово). P12/P16 уже написано.
Порядок хвиль після P3: P8 першим (усі інші фази швидкості стоять на ньому).


### 12.2 Головна ціль бібліотеки (рішення оператора 2026-10-30, замінює критерій §12.1)

**Файл, який зливається без сервера, доводить свою історію і читається без бібліотеки.**

Це три властивості одного об'єкта: local-first, перевірювана історія, самоопис. Критерій §12.1 «кожен ряд <= 1.0x SQLite» знижено до вимоги «достатньо швидко». Ціль P19 ×0.7N не відкликано.

Спершу пишеться модель даних: окремий підписаний ланцюг на кожен пристрій, злиття як підписана подія, дерево KV як робочий стан, який можна стискати, журнал як збережена історія.

Відкриті конфлікти:

- злиття проти доказу: історія є DAG, а не лінійний ланцюг;
- компакція проти історії: жодна компакція не робить історію невідновною;
- незмінність проти права на видалення: окремий ключ на суб'єкта;
- межа доказу: запис не змінили після підпису; це не доказ правдивості, а офлайн-годинникам вірити не можна, тому потрібні зовнішні якорі;
- керування ключами: втрата пристрою, відкликання, ротація; підпис лише ML-DSA;
- ціна самоопису: режим рівня архіву, а не кожного образу.

Вперед виходять P18, контракт склеювання і самоопис. Критерій готовності — демонстрація: два офлайн-пристрої зливають файл і конфлікт названо; сторонній аудитор перевіряє історію лише за файлом і підписами; чистий Node читає файл вбудованим читачем.
