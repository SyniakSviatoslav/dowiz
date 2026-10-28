# R-STORE-VS-SQL — bebop-store проти SQLite: виміри (2026-10-27)

Статус: **частковий**. Звіт написано 2026-10-28 лише з уже наявних сирих результатів, без
жодного нового запуску: лейн двічі втрачав сесію, а потім отримав правило «жодних обчислень,
бокс потрібен P0-лейну». Метрики 1, 2, 3, 6, 7 і частина 5 виміряно. Метрики 4, 8, 9, 10
(на поточному бінарнику) та 11 — **не виміряно**; причину кожної наведено в §5, точні
команди для дозапуску — в §6.

Джерело всіх чисел — `RES|…` рядки в
`/tmp/claude-0/-root/97ffb566-8be5-47e7-a223-f3a5b6569e89/scratchpad/storebench/results/*.txt`
(кожен файл закінчується `## rc=0`). Бінарник harness `storebench` зібрано 2026-10-27 17:22,
sha256 `5b23803aebbd8e7e…`; ним отримано всі числа, крім рядків 1e3×{16,256,4096}, 1e5×16 і
частини 1e5×256 (`matrix.sh`, 16:56–17:14, попередня збірка того ж коду — UNVERIFIED, що код
harness між збірками не змінився в бенчованих шляхах; див. §5).

Позначення: N — кількість ключів, VS — розмір value в байтах. Ключ — `key%012d` (15 B).
Усі часи — **медіана** (у дужках p95 або mean, де це важливо), кількість прогонів `runs`
вказана. Ядро: `taskset -c 5` (Cortex-A78), бокс — Android + proot, частоти плавають.

---

## 1. Зведена таблиця

| # | Метрика (N×VS) | bebop | SQLite (налаштування) | Співвідношення | Переможець |
|---|---|---|---|---|---|
| 1a | get, гарячий, 1e3×16 (7 прогонів) | 445 ns (`KvIn` zero-copy); 159 ns (`Kv` декодований) | 658 ns (WAL, один read txn); 611 ns (cache 1 GiB + mmap) | 1.4–4.1× | bebop |
| 1a | get, гарячий, 1e5×16 (7) | 1 594 ns `KvIn`; 941 ns `Kv` | 2 159 ns (txn); 1 150 ns (tuned) | 0.72–2.3× | змішано: `KvIn` програє tuned SQLite 1.4× |
| 1a | get, гарячий, 1e5×256 (7) | 1 733 ns `KvIn`; 1 312 ns `Kv` | 5 155 ns (txn); 1 784 ns (tuned) | 1.03–3.9× | bebop |
| 1a | get, гарячий, 1e4×4096 (7) | 869 ns `KvIn` | 34 739 ns (txn); 9 418 ns (tuned) | 10.8–40× | bebop |
| 1a | get, гарячий, 1e6×16 (5) | 3 080 ns `KvIn`; 1 499 ns `Kv` | 3 212 ns (txn); 1 321 ns (tuned) | `KvIn` 0.43× tuned | **SQLite tuned** (2.3× проти `KvIn`) |
| 1b | get autocommit (кожен get — власний txn), 1e3×16 | 873 ns (`KvIn::reopen`+get) | 552 893 ns (WAL, звичайні локи); 796 ns (DELETE journal, EXCLUSIVE) | 0.9–633× | залежить від локів; див. §2.1 |
| 2 | bulk load 1e3×16 (7) | 9.2 ms | 25.6 ms (DELETE/FULL, 1 txn) | 2.8× | bebop |
| 2 | bulk load 1e3×4096 (7) | 266 ms | 67.3 ms (DELETE/FULL) | 0.25× | **SQLite** (4×) |
| 2 | bulk load 1e5×16 (7) | 436 ms | 327 ms (DELETE/FULL); 223 ms (WAL, відсортований вхід) | 0.75× | **SQLite** |
| 2 | bulk load 1e5×256 (7) | 363 ms | 844 ms (WAL rowid); 1 780 ms (DELETE/FULL WITHOUT ROWID) | 2.3–4.9× | bebop |
| 2 | bulk load 1e4×4096 (7) | 1 419 ms | 1 472 ms (WAL rowid) | 1.04× | нічия |
| 2 | bulk load 1e6×16 (3) | 2 597 ms (вкл. sort 427 ms) | 13 551 ms (DELETE/FULL, випадковий порядок); 2 633 ms (відсортований вхід, sort не враховано) | 5.2× / 1.0× | bebop (випадковий вхід) |
| 3 | prefix scan ~100 ключів, 1e6×16 (5) | 8.6 µs | 21.9 µs (range у read txn) | 2.5× | bebop |
| 3 | prefix scan ~100, 1e4×4096 (7) | 8.4 µs | 355 µs | 42× | bebop (див. застереження §2.3) |
| 3 | full scan 1e6×16 (5) | 17.8 ms `KvIn`; 7.5 ms `Kv` | 175.9 ms | 9.9–23× | bebop |
| 3 | count 1e6×16 (5) | 0.26 µs (`KvIn.len()`, root cell) | 20 267 µs (`count(*)`) | ~78 000× | bebop (O(1) проти O(N)) |
| 5 | розмір файлу після load, 1e6×16 (вміст 31.0 MB) | 63.0 MB (2.03×) | 41.3 MB (1.33×, WITHOUT ROWID) | 0.66× | **SQLite** |
| 5 | розмір, 1e4×4096 (вміст 41.1 MB) | 41.4 MB (1.008×) | 46.9 MB (1.14×) | 1.13× | bebop |
| 5 | розмір після 50% updates / після compaction / VACUUM | не виміряно | не виміряно | — | — (§5) |
| 6 | open + перший get, cold, 1e3×16 (7) | 1.23 ms (mmap + `reopen` без CRC); 1.43 ms (з CRC) | 22.9 ms | 16–19× | bebop |
| 6 | open + перший get, cold, 1e6×16 (5) | 3.76 ms (без CRC); **183 ms** (mmap + `KvIn::open`, CRC усього); 361 ms (`Catalog::load` шлях) | 11.3 ms | 3.0× / **0.06×** | без CRC — bebop; з перевіркою — **SQLite 16×** |
| 7 | put, durable, 1e3×16 (5×200) | медіана 407 µs, **mean 929 µs** (3 fsync/op + compaction кожні 33 op) | медіана 223 µs, mean 814 µs (WAL/FULL, EXCLUSIVE) | 0.55× / 0.88× | SQLite |
| 7 | put, durable, 1e6×16 (3×200) | медіана 531 µs, **mean 50 683 µs** (1.89 MB записано на op) | медіана 269 µs, mean 399 µs (WAL/FULL, EXCLUSIVE) | **0.008× за mean** | **SQLite ~127×** |
| 7 | put, без fsync, 1e6×16 (3×2000) | медіана 21 µs, mean 24 395 µs | медіана 11.9 µs, mean 18 µs (WAL/OFF) | ~0.0007× за mean | **SQLite** |
| 7 | delete, durable, 1e6×16 (3×200) | медіана 4 611 µs, mean 61 475 µs | медіана 233 µs, mean 400 µs (WAL/FULL) | ~0.007× | **SQLite** |
| 4 | read amplification / compaction / WAL checkpoint | не виміряно | не виміряно | — | — (§5) |
| 8 | конкурентні читачі + 1 writer | не виміряно | не виміряно | — | — (§5) |
| 9 | join/aggregate (DG8, каталог), Worker wasm | не виміряно | не виміряно | — | — (§5) |
| 10 | integrity: час verify / що ловить | тільки ранній прогін (UNVERIFIED, §2.6) | — | — | — |
| 11 | код / бінарник | 7 952 рядків `.rs` у `crates/bebop-store/src` (3 690 без тестів і коментарів); бінарник не виміряно | SQLite 3.46.0 bundled; не виміряно | — | — (§5) |

Однією фразою: **bebop швидший на читанні з уже відкритого образу (get, scan, count) і на
відкритті без перевірки; SQLite швидший на всьому, що пише по одному запису, на відкритті з
перевіркою цілісності великих образів і на просторі для малих values.**

---

## 2. Розбір по метриках

### 2.1 Point get (метрика 1)

Контрольна сума `acc` (сума по прочитаних values) збігається на всіх 8 варіантах у кожному
файлі (`checksums_equal = 1`), тож обидві сторони реально читають ті самі values.

| N×VS | `Kv` (Vec у RAM) | `KvIn` zero-copy, відкритий раз | `KvIn::reopen`+get | SQLite WAL, у txn | SQLite tuned, у txn | SQLite WAL autocommit | SQLite WAL EXCLUSIVE autocommit | SQLite DELETE EXCLUSIVE autocommit |
|---|---|---|---|---|---|---|---|---|
| 1e3×16 | 159 | 445 | 873 | 658 | 611 | 552 893 | 181 096 | 796 |
| 1e3×256 | 186 | 443 | 855 | 799 | 708 | 496 441 | 194 475 | 952 |
| 1e3×4096 | 871 | 467 | 850 | 15 352 | 6 531 | 546 798 | 242 854 | 15 427 |
| 1e5×16 | 941 | 1 594 | 2 016 | 2 159 | 1 150 | 341 641 | 161 767 | 2 502 |
| 1e5×256 | 1 312 | 1 733 | 2 168 | 5 155 | 1 784 | 687 775 | 329 915 | 5 210 |
| 1e4×4096 | 1 463 | 869 | 1 384 | 34 739 | 9 418 | 619 485 | 408 332 | 27 862 |
| 1e6×16 | 1 499 | 3 080 | 3 078 | 3 212 | 1 321 | 350 455 | 182 525 | 3 250 |

(ns/op, медіана, 7 прогонів; 1e6 — 5 прогонів.)

Чому:
- `KvIn` шукає ключ бінарним пошуком по індексу, але кожен крок іде через комірку-посилання на
  окремий об'єкт ключа (`crates/bebop-store/src/kv/zc.rs:204` `find`). Harness рахує
  прочитані cells: 146 на get при 1e3, 238 при 1e5, 277 при 1e6. Це ~log2(N) непрямих
  переходів, кожен — потенційний cache miss; B-tree SQLite з кешем 1 GiB + mmap робить ~3–4
  переходи по сторінках, тому при 1e6 tuned SQLite випереджає `KvIn` у 2.3×.
- При великих values (4096) SQLite програє в 10–40×: value 4 KiB не влазить у leaf-сторінку
  4 KiB і йде в overflow-ланцюжок, а bebop віддає зріз без копіювання.
- Стовпці autocommit показують **податок proot на fcntl-локи**: кожен read txn у SQLite бере і
  знімає лок; під proot `fcntl(F_SETLK)` коштує ~140 µs (коментар у
  `storebench/src/common.rs:168`, вимір того ж лейну). На реальному Linux цього стовпця не
  було б. Тому чесне порівняння — колонки «у txn» і «tuned». Чому WAL+EXCLUSIVE autocommit все
  одно ~180 µs, а DELETE+EXCLUSIVE — ~0.8 µs, лейн не розібрав (UNVERIFIED; ймовірно, доступ
  до wal-index під proot).

### 2.2 Bulk load (метрика 2)

bebop: sort + `Kv::compacted_bytes_fit` (`kv.rs:361`) + запис tmp/fsync/rename/fsync
каталогу. SQLite: один txn, prepared INSERT, випадковий порядок, з close/checkpoint.

| N×VS | bebop всього (з них sort / fit) | SQLite WAL/NORMAL WITHOUT ROWID | WAL rowid+autoindex | DELETE/FULL (дефолт) WITHOUT ROWID | WAL, відсортований вхід |
|---|---|---|---|---|---|
| 1e3×16 | 9.2 (0.07 / 0.12) | 78.2 (p95 888) | 33.2 | 25.6 | 36.6 |
| 1e3×256 | 16.2 (0.11 / 1.5) | 68.6 | 60.3 | 33.3 | 43.2 |
| 1e3×4096 | 266 (0.12 / 9.4) | 109.9 | 96.2 | 67.3 | 107.0 |
| 1e5×16 | 436 (26 / 24) | 430.6 | 373.6 | 327.5 | 223.0 |
| 1e5×256 | 363 (34 / 91) | 3 363 | 844 | 1 780 | 746 |
| 1e4×4096 | 1 419 (2.3 / 168) | 2 509 | 1 472 | 1 794 | 1 569 |
| 1e6×16 | 2 597 (427 / 407) | 20 996 | 18 276 | 13 551 | 2 633 |

(ms, медіана; 7 прогонів, 1e6 — 3.)

Чому: bebop будує весь образ у пам'яті одним проходом після сортування, тому його вартість
≈ O(N log N) sort + лінійна серіалізація + один послідовний запис. SQLite з випадковим
порядком вставки розщеплює сторінки B-tree; з відсортованим входом він майже рівний bebop
(1e6×16: 2 633 проти 2 597 ms). Решта часу bebop поза sort/fit — це запис і fsync файлу; на
1e3×4096 це ~256 ms на 4 MB, що підозріло повільно порівняно з SQLite (67 ms на 4.7 MB) —
причину не розібрано (UNVERIFIED; можливо, tmp+rename+dir-fsync під proot).

Інкрементний шлях `Kv::put` × N у випадковому порядку + compaction дорогий:
1e5×16 — 11 483 ms (проти 436 ms для sort-шляху, 26×), 1e5×256 — 12 704 ms. `Kv::put`
вставляє в відсортований Vec бінарним пошуком (`kv/zc.rs:24` коментар), тобто кожна вставка
— memmove O(N), разом O(N²). На 1e6 цей рядок свідомо не запускався.

### 2.3 Scan і count (метрика 3)

| N×VS | prefix ~100 ключів: bebop / SQLite (µs) | full scan: `KvIn` / `Kv` / SQLite (ms) | count: bebop (µs) / SQLite (µs) |
|---|---|---|---|
| 1e3×16 | 7.5 / 18.7 | 0.021 / 0.003 / 0.149 | 0.26 / 3.4 |
| 1e3×256 | 7.8 / 21.9 | 0.022 / 0.004 / 0.180 | 0.36 / 9.5 |
| 1e3×4096 | 7.6 / 293.8 | 0.022 / 0.005 / 1.657 | 0.26 / 14.8 |
| 1e5×16 | 9.1 / 22.6 | 2.12 / 0.84 / 17.5 | 0.31 / 2 100 |
| 1e5×256 | 9.4 / 44.5 | 2.16 / 1.07 / 35.9 | 0.26 / 18 486 |
| 1e4×4096 | 8.4 / 355.3 | 0.214 / 0.073 / 20.95 | 0.31 / 2 851 |
| 1e6×16 | 8.6 / 21.9 | 17.8 / 7.5 / 175.9 | 0.26 / 20 267 |

Контрольні суми full-scan і prefix збігаються на всіх трьох шляхах.

Чому: ключі в образі відсортовані, prefix — бінарний пошук початку плюс послідовний прохід
(`kv/zc.rs:289` `kv_prefix_in`). `count` у bebop — одна комірка в root, O(1); SQLite
`count(*)` обходить усе дерево, O(N).

**Застереження (важливе):** `acc` у scan — це `len(value) + один байт ключа`
(`storebench/src/kvb.rs:465,497,523`). Байти value не читаються ні на одній стороні. Для
bebop довжина лежить в індексі, тож 41 MB values при 1e4×4096 взагалі не торкаються
(0.214 ms — це ~190 GB/s, фізично неможливо для реального читання). SQLite змушений пройти
overflow-ланцюжки, щоб віддати blob. Тож 42× і 98× для VS=4096 — це «зріз без читання» проти
«матеріалізації»; для споживача, що реально читає всі байти, розрив буде значно меншим (не
виміряно).

### 2.4 Open / startup (метрика 6)

| N×VS | bebop mmap+`reopen` (без CRC) | bebop mmap+`KvIn::open` (CRC усіх масивів) | bebop read+`Kv::load_checked` (`Catalog::load`) | SQLite open+prepare+перший get |
|---|---|---|---|---|
| warm 1e3×16 | 0.52 | 0.84 | 0.72 | 21.8 |
| cold 1e3×16 | 1.23 | 1.43 | 1.24 | 22.9 |
| cold 1e3×4096 | 1.81 | 13.6 | 16.8 | 27.1 |
| cold 1e5×16 | 4.04 | 23.3 | 35.4 | 13.5 |
| cold 1e5×256 | 4.60 | 105.8 | 143.4 | 80.7 |
| cold 1e4×4096 | 5.15 | 66.5 | 155.0 | 19.7 |
| warm 1e6×16 | 0.67 | 144.3 | 324.4 | 7.4 |
| cold 1e6×16 | 3.76 | 183.4 | 360.7 | 11.3 |

(ms, медіана, 7 прогонів, 1e6 — 5; «cold» = `posix_fadvise(DONTNEED)`, `fadvise_ok = 1`.)

Чому: `KvIn::open` (`kv/zc.rs:84–97`) рахує CRC root і всіх чотирьох масивів, перевіряє
delta-ланцюжок і робить index pass — це O(байтів образу), тому на 63 MB це 144–183 ms.
`KvIn::reopen` (`kv/zc.rs:101`) пропускає все це і коштує майже константу — але дає
**неперевірені** дані (див. §2.6: при зіпсованій комірці індексу ключ мовчки зникає).
SQLite перевіряє лише заголовок і схему, а сторінки — ліниво, тому його open майже не
залежить від N. Дивно, що SQLite open при 1e3 (22 ms) повільніший, ніж при 1e6 (7–11 ms):
ймовірно, шум proot `open()`/fcntl (~300 µs на open під proot, `common.rs:269`) і порядок
прогонів — UNVERIFIED.

Висновок для dowiz: zero-copy open без перевірки — справжня перевага bebop (3–30×), але
тільки якщо цілісність образу гарантує щось інше (наприклад, CRC перевірено один раз при
записі, а носій довірений).

### 2.5 Durability: put/delete по одному запису (метрика 7)

Що саме синхронізує bebop (прочитано в коді):
- `Store::commit` (`crates/bebop-store/src/lib.rs:856–866`): нові об'єкти → `sync_data`,
  сторінка PartTab → `sync_data`, superblock останнім → `sync_data`. **3 fsync на commit.**
- delta-ланцюжок: `append_delta` у крейті комітить лише в пам'ять (`commit_bytes`); дисковий
  шлях у harness (`storebench/src/common.rs:256–300`, `BbFile::apply`) відтворює порядок
  `Store::commit` з тими ж трьома `sync_data`.
- Коли ланцюжок досягає `MAX_DELTAS = 32` (`kv/delta.rs:50`), або мертвих комірок більше ніж
  live/`DEAD_DIV`=4 (`kv/delta.rs:52`), або арена повна (`Why::Full`), `append_delta`
  повертає `Compact` — і весь образ переписується (tmp + fsync + rename + fsync каталогу).
- `Kv::commit_into` (`kv.rs:300`) переписує всі чотири масиви в арену при кожному commit;
  арена не звільняється: після 50 commit при образі 71 KB вона займає 3.2 MB (45×), при
  образі 4.15 MB — 211 MB.

Пристрій: 4 KiB `pwrite`+`fdatasync` — медіана 96–246 µs.

Put, durable, µs/op (медіана; mean у дужках):

| N×VS | bebop delta+compaction [3 fsync] | bebop whole-image rewrite | SQLite WAL/FULL EXCL | SQLite DELETE/FULL EXCL | SQLite WAL/NORMAL EXCL | SQLite WAL/FULL звичайні локи |
|---|---|---|---|---|---|---|
| 1e3×16 | 407 (929) | 6 548 | 223 (814) | 368 | 9.2 | 1 331 |
| 1e3×256 | 472 (1 219) | 9 654 | 193 (261) | 343 | 10.0 | 1 075 |
| 1e3×4096 | 476 (11 274) | 46 865 | 251 (367) | 447 | 40.7 | 1 460 |
| 1e5×16 | 386 (4 240) | 76 772 | 212 (379) | 367 | 13.2 | 1 244 |
| 1e5×256 | 490 (50 544) | 1 740 289 | 268 (399) | 483 | 21.9 | 1 456 |
| 1e4×4096 | 578 (23 184) | 417 240 | 315 (809) | 546 | 80.9 | 1 193 |
| 1e6×16 | 531 (50 683) | 690 526 | 269 (399) | 479 | 17.8 | 959 |

Без fsync: bebop delta медіана 10–89 µs, але mean 74 µs (1e3×16) → 989 (1e5×16) → 6 888
(1e5×256) → 9 309 (1e4×4096) → 24 395 µs (1e6×16); SQLite WAL/OFF медіана 9–44 µs, mean
11–153 µs.

Записано байтів на op (bebop delta): 2.6 KB (1e3×16), 190 KB (1e5×16), 910 KB (1e5×256),
1.25 MB (1e4×4096), 1.89 MB (1e6×16). SQLite WAL пише ~1 сторінку + WAL-кадр на op.

Чому: медіана bebop (3 fsync) ≈ 2× медіани SQLite WAL/FULL (1 fsync на commit у WAL) — це
просто кількість fsync. Але **mean** визначається compaction: кожен 33-й запис переписує весь
образ, тож середня вартість ≈ розмір_образу/33 на запис і росте лінійно з N. На 1e6×16 це
~127× гірше за SQLite. Для dowiz (образ хаба сотні KB) це прийнятно; для загального
embedded-сховища з мільйонами ключів — ні.

Delete поводиться так само (1e6×16 durable: bebop медіана 4.6 ms, mean 61 ms; SQLite 233 µs,
mean 400 µs). На малих N delete частіше тригерить compaction через `Why::Dead`.

### 2.6 Integrity (метрика 10) — лише ранній прогін, UNVERIFIED

Метрика `integ` на поточному бінарнику не запускалась. Є один прогін 2026-10-27 15:47 на
ранній збірці (`results_v1_prootbug/integ_1000_16.txt`, 1e3×16, 5 прогонів), до виправлення,
що дало назву каталогу. Числа звідти — тільки орієнтир:
- час перевірки 1e3×16: `KvIn::open` 0.136 ms, `Kv::load_checked` 0.177 ms; SQLite
  `quick_check` 0.481 ms, `integrity_check` 0.490 ms;
- по одній названій зіпсованій комірці на клас (правило оператора, без фазерів):
  - bebop C1 (байт value), C2 (байт ключа), C3 (комірка індексу) — `KvIn::open` відмовляє з
    `BadCrc`; **але `KvIn::reopen` (без CRC) на C3 мовчки втрачає ключ**;
  - bebop C4 (байт live superblock) і C5 (обрізаний до 90%) — відмова `NotKv`;
  - bebop C4b (байт superblock на v3-образі з 2 deltas) — **тихий відкат до попередньої
    генерації**, жоден шлях не помічає;
  - SQLite C3 (тип сторінки), C4 (magic), C5 (обрізання) — виявлено;
  - SQLite C1/C2 (байт value/ключа) — `quick_check` і `integrity_check` кажуть `ok`, SELECT
    повертає правильне значення. Harness сам зазначив «2 copies of the key in the file», тобто
    ймовірно зіпсовано не живу копію — цей результат **не доводить**, що SQLite пропускає
    пошкодження payload (хоча він справді не має CRC на payload). UNVERIFIED.

### 2.7 Простір (метрика 5)

| N×VS | вміст (key+value) | bebop | SQLite WITHOUT ROWID | SQLite rowid+autoindex |
|---|---|---|---|---|
| 1e3×16 | 31 000 | 71 320 (2.30×) | 49 152 (1.59×) | 77 824 |
| 1e3×256 | 271 000 | 311 320 (1.15×) | 348 160 (1.28×) | 331 776 |
| 1e3×4096 | 4 111 000 | 4 151 320 (1.01×) | 4 702 208 (1.14×) | 4 644 864 |
| 1e5×16 | 3 100 000 | 6 308 320 (2.03×) | 4 182 016 (1.35×) | 6 668 288 |
| 1e5×256 | 27 100 000 | 30 308 320 (1.12×) | 33 697 792 (1.24×) | 31 981 568 |
| 1e4×4096 | 41 110 000 | 41 438 320 (1.008×) | 46 911 488 (1.14×) | 46 354 432 |
| 1e6×16 | 31 000 000 | 63 008 320 (2.03×) | 41 349 120 (1.33×) | 66 863 104 |

(байти; після свіжого load; SQLite після checkpoint, `wal=0`.)

Чому: образ складається з 8-байтових cells; ключ і value — окремі об'єкти з двома
заголовковими cells h0/h1 (довжина, digest і CRC; `Store::alloc`, `lib.rs:785–787`) і вирівнюванням до 8 B, плюс комірки
індексу. Звідси ~32 B фіксованої надбудови на запис (63 B/запис при 31 B вмісту). Для
великих values надбудова тоне (1.008×), а SQLite платить за неповне заповнення сторінок і
overflow (1.14×).

Після 50% updates, після compaction, після 50% deletes, після VACUUM — **не виміряно**
(`space` не запускався).

---

## 3. Де bebop виграє / програє / не може конкурувати

**(a) для dowiz** (образ хаба — сотні KB, читання з Worker, рідкі записи):
1. Виграє: open без перевірки + get із відкритого образу (1e3: 1.2 ms cold open проти 23 ms;
   get 445 ns проти 611–658 ns). Це саме профіль «прочитати меню з образу».
2. Виграє: prefix scan і count — 2.5× і O(1).
3. Виграє: простір при values ≥ 256 B (1.01–1.15× проти 1.14–1.28×).
4. Програє, але терпимо: durable put — медіана 2× гірша (3 fsync проти 1); при образі
   ~100 KB compaction кожні 33 записи коштує мілісекунди.
5. Ризик: `reopen` без CRC може мовчки втратити ключ; пошкоджений superblock на v3 тихо
   відкочує генерацію (UNVERIFIED, ранній прогін).

**(b) для загального embedded-сховища:**
1. Не може конкурувати: записи при великому N — mean put 50 ms при 1e6 (×127 проти SQLite),
   бо compaction переписує весь образ; інкрементний `Kv::put` — O(N²).
2. Програє: open з перевіркою цілісності — O(розміру образу), 183 ms на 63 MB проти 11 ms.
3. Програє: get при 1e6 проти налаштованого SQLite (3.1 µs проти 1.3 µs).
4. Програє: простір для малих values (2.03× проти 1.33×).
5. Відсутні функції: транзакції на кілька ключів поза одним delta, SQL/join/aggregate,
   конкурентні writers — порівняння не виміряне (§5).

---

## 4. Відтворення

- Дата: 2026-10-27 15:56–20:11 (UTC+1 у логах, UTC у заголовках файлів).
- CPU: 8 ядер (Cortex-A55 + Cortex-A78), вимір на ядрі 5 (A78), `nice 5` через
  `/root/dowiz/bebop-lang/tools/slot.sh` (ядра 4,5,6). Android, proot-distro, Linux
  6.17.0-PRoot-Distro.
- rustc 1.93.1 (01f6ddf75 2026-02-11), `--release` opt-level 3.
- SQLite 3.46.0 (rusqlite =0.32.1, `bundled`). Налаштування: `SQ_DEFAULT` DELETE/FULL,
  `SQ_WAL_NORMAL`, `SQ_WAL_FULL`, `SQ_WAL_OFF`, `SQ_TUNED` (WAL/NORMAL, cache 1 GiB, mmap
  2 GiB), `excl()` = `locking_mode=EXCLUSIVE` (`storebench/src/common.rs:172–178`).
- bebop-store: `/root/dowiz/crates/bebop-store` за path-залежністю; останній commit, що
  торкався крейта, — `d4bd2114` (2026-10-10).
- Harness: `/tmp/claude-0/-root/97ffb566-8be5-47e7-a223-f3a5b6569e89/scratchpad/storebench/`
  (`src/kvb.rs`, `common.rs`, `integ.rs`, `cat.rs`, `g7.rs`, `dl.rs`), `run1.sh <cmd> <N> <VS>
  [runs]` (пише в `results/<cmd>_<N>_<VS>.txt`), `matrix.sh`, `matrix2.sh`, `matrix3.sh`,
  зведення `python3 summ.py results`.
- Збірка: `PATH=/root/.cargo/bin:$PATH PKG_CONFIG_LIBDIR=/usr/lib/aarch64-linux-gnu/pkgconfig:/usr/share/pkgconfig CARGO_TARGET_DIR=/root/.cache/storebench-target cargo build --release --offline --bin storebench`.

---

## 5. Що не виміряно і чому; що UNVERIFIED

Не виміряно:
- **1e6×16 delete без fsync** (bebop і SQLite) — процес вбито `rc=137` phantom-process killer
  під час цього кроку (36 процесів при ліміті 32, `matrix3.log` 20:11). Решта рядків
  `put_1000000_16.txt` збережена.
- **1e6×256, 1e5×4096** — свідомо прибрано з матриці: ~2 GB вільної RAM на боксі
  (`matrix2.sh`, коментар), замість 1e5×4096 виміряно 1e4×4096.
- **`Kv::put` × N інкрементний load при 1e6** — не запускався (O(N²), §2.2).
- **`Kv::commit_into` для образів ≥ 4 MB** — пропущено за умовою harness (`image_len < 4<<20`).
- **Метрика 4** (read amplification при зростанні ланцюжка, вартість compaction, WAL
  checkpoint) — `readamp 100000 256` ніколи не запускався: `matrix2.sh` вбито `rc=137` на
  `load 1000000 16`, а потім діяло правило «без обчислень».
- **Метрика 5 після updates/compaction/deletes/VACUUM** — `space 100000 256`,
  `space 1000000 16` не запускались (та сама причина).
- **Метрика 8** (читачі + writer) — `conc 100000 256` не запускався.
- **Метрика 9** (каталог dowiz через проекції/DG8 проти SQL, Datalog `dl`, gate G7, Worker
  wasm проти D1) — `cat`, `g7`, `dl`, `g7_bebop.sh`, `dl_bench.sh`, `wasm_time.mjs` не
  запускались. Існуючі виміри з пам'яті (`dowiz-hub-unit-costs.md` тощо) не використано,
  бо картка дозволяє їх лише після перевірки.
- **Метрика 10 на поточному бінарнику** — `integ 100000 256`, `integ 1000000 16` не
  запускались; є лише ранній 1e3×16 (§2.6).
- **Метрика 11** — бінарники `mini_bebop`/`mini_sqlite`/`mini_none` (профіль `mini`) не
  зібрано; розмір бінарника невідомий. Рядки коду крейта пораховано (`wc -l`).

UNVERIFIED:
- Числа 1e3×{16,256,4096}, 1e5×16, 1e5×256 (get/open/scan/load) отримано попередньою
  збіркою harness (до 17:22); бенчовані шляхи мали бути ті самі, але це не перевірено diff-ом.
- Повільний запис bebop при load 1e3×4096 (~256 ms на 4 MB).
- Причина ~180 µs на WAL+EXCLUSIVE autocommit get.
- SQLite open 1e3 повільніший за 1e6.
- Усі integrity-результати (§2.6), особливо SQLite C1/C2.
- Співвідношення VS=4096 у scan (§2.3) — харнес не читає байти values.

---

## 6. Що лишилось запустити (для main, коли бокс вільний)

Усе через один `slot.sh` на виклик, не вкладаючи `slot.sh` у `slot.sh`; кожен виклик
< ~9 хв; перебудова не потрібна (бінарник sha `5b23803aebbd8e7e`). З каталогу
`/tmp/claude-0/-root/97ffb566-8be5-47e7-a223-f3a5b6569e89/scratchpad/storebench`:

```
S=/root/dowiz/bebop-lang/tools/slot.sh
$S sb-m4a ./matrix3.sh "put 1000000 16 3"          # дозаміряти delete без fsync; спершу прибрати старий put_1000000_16.txt у results_killed/
$S sb-m4b ./matrix3.sh "readamp 100000 256 7" "space 100000 256 1"
$S sb-m4c ./matrix3.sh "space 1000000 16 1"
$S sb-m4d ./matrix3.sh "integ 100000 256 5" "integ 1000000 16 5"
$S sb-m4e ./matrix3.sh "cat 0 0 7" "g7 0 0 5" "dl 0 0 5"
SB_CORE=4,5,6 $S sb-m4f ./matrix3.sh "conc 100000 256 1"
$S sb-m4g ./g7_bebop.sh                              # > results/g7_bebop.txt
BEBOP_TMP=$PWD/bl/tmp $S sb-m4h taskset -c 5 /root/dowiz/bebop-lang/bench/vs_rust/dl_bench.sh   # > results/dl_bebop.txt
$S sb-m4i sh -c 'for i in 1 2 3; do taskset -c 5 node wasm_time.mjs /root/dowiz/crates/bebop-wasm/target/wasm32-unknown-unknown/release/bebop_wasm.wasm data/cat.img 200 7; done > results/wasm.txt 2>&1'
# метрика 11:
$S sb-m4j env PATH=/root/.cargo/bin:$PATH CARGO_TARGET_DIR=/root/.cache/storebench-target cargo build --profile mini --offline --bins && ls -l /root/.cache/storebench-target/mini/mini_*
```

Перед кожним кроком: `df -h /` (на 2026-10-28 вільно 5.6 GB; стоп при < 3 GB) і перевірити,
що жоден інший лейн не тримає слот (крок 1e6 перевищив ліміт процесів, коли поруч працювали
інші процеси).
