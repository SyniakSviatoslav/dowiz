Status: 2026-09-06 (session 18) -- research report by a Fable fork, read-only over /root/dowiz/bebop-lang at HEAD 44ae047
(register-model worker in flight; every "after register model" number is the RESEARCH-DEPS/RESEARCH-TENSOR forecast, not a
measurement). Box facts reused: Cortex-A78 under proot, DRAM ~12 GB/s, DRAM miss ~100 ns, page fault 2.5-7 us, msync of one
page ~100 us, rename ~270 us, f2fs mounted `fsync_mode=nobarrier` (this session re-checked /proc/mounts: `barrier` is on for
data writes, but `fsync_mode=nobarrier` means fsync/msync return without a device cache flush -- LANG-DB-DESIGN §3, and the
f2fs kernel doc fetched today: nobarrier "doesn't issue flush command"). sqlite 3.46.1 present (python module); duckdb absent
(no apt package, no pip module) -- a DuckDB twin cannot run here without adding a dependency.

# Частина 1. Bebop без вказівників: індексна модель, її ціна і виграш. Частина 2. Чого бракує стору, щоб сперечатися з SQL як класом

## 1.0 Де в bebop вказівники сьогодні

| конструкція | що це фізично | база / регістр | час життя | клас пастки | джерело |
|---|---|---|---|---|---|
| `[i64]` з `zeros(n)` | абсолютна адреса в анонімній арені 256 MB | x27 курсор / x28 кінець; адреса = x27 до bump'а | процес (ніколи не звільняється) | exit 80 (арена вичерпана); читання за межею UNDEFINED, SIGSEGV = 82 | LANGUAGE.md «Memory model», emit_zeros |
| `[e0, e1, ...]`, enum ctor, struct literal | абсолютна адреса у frame heap активації | x14 bump від sp+1024, 16 KiB на активацію | до `ret`; у `while` -- до back-edge, якщо `loop_alloc_safe` доведе, що вказівник не тікає (T42/T43) | exit 81; use-after-release при витоку з циклу (c34) | LANGUAGE.md, emit_array_lit, loop_alloc_safe bebop.bp:3044 |
| `str` літерал | абсолютна адреса в data-секції .bin через `adr` | PC-relative | образ процесу, read-only | -- | emit_str bebop.bp:2630 |
| `char(s,i)` | `ldrb` за адресою s+i | будь-який raw pointer (літерал, sys_readbuf, mmap) | -- | wild read = 82 | emit_char_fn |
| символи 9+ і temp-slot-и | `[x15,#k*8]`, x15 = sp+256 | x15, caller-saved, перев'язується callee | активація | -- | REGISTER-MODEL-BLUEPRINT §1.1 |
| store-об'єкти | зміщення в комірках від mmap-бази; `ref` = object-relative offset | база = результат `st_open`/`st_map_ro` (будь-яка), `st_cells/st_addr` -- єдині два касти i64<->[i64] у дереві | файл; читач тримає mapping | crc32 у h1; torn superblock відкидається | LANG-DB-DESIGN §4a-4c, store.bp:84-176 |
| CSR `rp[]/ci[]`, nnidx, sgraph | індекси (позиції в масиві) -- ВЖЕ індексна форма | база = адреса масиву (вказівник) | масив | -- | csr.bp:1-12, sgraph.bp:3-13 |
| sys_* буфери, argv | вказівники на комірки (bytes-in-cells: 1 байт на i64) | -- | -- | 8x пам'яті на IO-шляху | LANGUAGE.md builtins, sys_slurp `round16(len*8)` |
| потоки pool.bp | стеки 64 KiB в арені, futex-адреси | x27-арена | процес | clone-fn <= 8 живих символів | T45/T115 |

Разом: три «світи» адрес (арена, frame heap, store) плюс read-only образ; типи парсяться й відкидаються, тому
компілятор не відрізняє вказівник у арену від вказівника у frame heap від цілого (LANG-DB §2, «F-F»). Стор --
єдине місце, де вказівників уже немає (G2: одна й та сама структура читається з двох баз у одному процесі).

## 1.1 Індексна модель: механізм

**Означення.** Кожне посилання = ціле `idx` у іменовану арену/таблицю; адреса = `base(arena) + idx*8`
обчислюється лише всередині виразу, що читає/пише, і ніколи не зберігається в комірку. Це правило LANG-DB
§4b («ref values are offsets, materialised to addresses only inside the expression that uses them») підняте
з файлу на всю пам'ять.

**Форми AArch64 (усі по одному слову, `as` -> `objdump` за L1):**

| доступ | вказівник (сьогодні) | індекс з базою в регістрі | індекс без бази в регістрі |
|---|---|---|---|
| `a[i]` | `ldr xd,[xa,xi,lsl #3]` (1) | `add t,xa,xi ; ldr xd,[xB,t,lsl #3]` (2) або, якщо `a` -- сам індекс елемента 0 і `i` const: `ldr xd,[xB,#(a+i)*8]` неможливо (a динамічне) → 2 | `ldr xB,[xG,#arena*8] ; add ; ldr` (3) |
| `a[c]` (c const) | `ldr xd,[xa,#c*8]` (1) | `add t,xa,#c ; ldr xd,[xB,t,lsl #3]` (2) | 3 |
| `s.f` (struct) | `ldr xd,[xs,#f*8]` (1) | `add t,xs,#f ; ldr xd,[xB,t,lsl #3]` (2) | 3 |
| `p.next` (ref) | `ldr xd,[xp,#f*8]` (1) -- повертає вказівник | `add t,xp,#f ; ldr xd,[xB,t,lsl #3]` (2) -- повертає індекс | 3 |
| `[u32]` elem | -- | `ldr wd,[xB,xi,lsl #2]` (1, якщо `a` -- база таблиці, `i` -- елемент) | -- |

Один зайвий `add` на доступ, коли масив -- індекс першої комірки. Він зникає лише якщо (а) база масиву
живе в регістрі як SYM (тоді це і є вказівник у регістрі) або (б) майбутній плоский IR виносить `base+a` з
циклу (LICM, RESEARCH-DEPS §3 -- на самокомпіляції ~0, на кернелах-сканах реально).

**Скільки баз живуть.** Регістри вільні від усього в регістровій моделі: x17, x18 (x16 -- scratch parallel
move; x18 -- «platform register», під Linux-userspace вільний, Android bionic не використовує його в
чужому коді, але seed.S/entry_stub треба перевірити). Отже ОДНА глобальна база арени в x17 коштує нуль
слів: `ldr xd,[x17,xidx,lsl #3]`, і «індекс = вказівник - база», де база = `sys_arena_base()` (builtin є).
Друга база (стор) -- x18 або SYM-регістр функції, що з ним працює. Третьої фіксованої немає: типізовані
таблиці (1.4 крок 2) або живуть в арені x17 з індексом-початком у SYM, або платять `ldr` бази з таблиці
баз (`[x17,#-8*k]`, +1 слово, +4 такти латентності на ланцюжку).

**Ціна у кернелах** (після регістрової моделі, RESEARCH-TENSOR §4): K6 scan ~5-8 ns/row при ~12 словах у
циклі з 3 доступами -> +3 `add` = +3 слова, ≈ +0.5-1 ns/row (+10-20 %) поки скан codegen-bound; при
DRAM-стелі 2 ns/row OoO ховає їх повністю (0 %). BFS: доступи `rp[v]`, `ci[j]` -- по одному `add`, латентно
на ланцюжку pointer-chasing +1 такт на ~100 нс промаху = 0. K1H-K8H: масивів нема -- 0. Самокомпіляція:
`pos[0]`, `n[0]`, `insns[n[0]]`, `fntab[...]` -- тисячі доступів, +1 слово кожен ≈ +5-8 % слів у гарячих
парсерних циклах поки немає SROA (RESEARCH-DEPS §1 таблиця «потребує IR»); з SROA `[0]`-комірок -- ~0.

## 1.2 Переваги з числами

**(b) Пропускна здатність: u32-індекси.** CSR 10M ребер: `ci` 80 MB (i64) -> 40 MB (u32), `rp` 8 -> 4 MB.
Frontier-BFS сьогодні 45 ns/edge-slot (T117 stage 2, Beamer), queue-BFS 187-240 ns -- це промахи, не
байти: u32 дає 2x менше ліній лише там, де читання послідовне (pull-фаза сканує рядки `ci`: 2x менше
трафіку -> оцінка 45 -> 30-35 ns/slot, derived), на random `rp[v]` -- 0. K6 scan: 24 B/row (u,v,cell i64) ->
12 B/row (u32 x3): DRAM-стеля 2 ns/row -> 1 ns/row = ще 2x ПІСЛЯ того, як регістрова модель + NEON-builtin
доведуть скан до стелі (RESEARCH-TENSOR §4: ~80-90x vs sqlite -> ~150x; vs Rust з u32 у обох -- 1x).
Файл стору: G7 «2.1-2.5x LOSS» по розміру (40 B/record vs sqlite 18 B) стає ~1x з u32-комірками там, де
значення вміщаються. Ціна: тип `[u32]` (T48-клас), `ldr w`/`str w` форми (по одному слову), bpref-паритет
(u32 wrap!), обмеження 4G елементів на таблицю (для стору 1M-10M -- не обмеження; для арени 256 MB -- ніколи).

**(b') Bytes-in-cells.** Байтова арена + handle `(off, len)` замість комірки-на-байт: sys_read/sys_slurp
у 8x менше пам'яті, `crc32x` замість `crc32` по комірках, парсер читає `ldrb` (це вже `char(s,i)`), а
`str` стає ЗНАЧЕННЯМ (сьогодні «strings as values» -- явно поза мовою, LANGUAGE.md «What is NOT»): один
i64 = `off<<32 | len` (4 GB байтової арени, 4 GB довжини) -- рядки як значення без копій і без GC, бо арена
не звільняється. Це найбільший виграш поверхні мови з усієї частини 1 і той самий raw-byte шлях, якого
вимагає RESEARCH-DEPS §6d-8 (zero-copy ~1x проти найкращого Rust, 3-10x проти serde-owned).

**(c) Безпека і пастки.** Bounds check = `cmp xidx,xlen ; b.hs trap` (2 слова, гілка передбачувана, ~0 тактів
на OoO поза ланцюжком; len -- заголовок таблиці або SYM). У K6-циклі 3 доступи -> +6 слів: при
codegen-bound ~+15 %, при DRAM-стелі ~0. Арена степеня двійки: `tbnz xidx,#k,trap` -- 1 слово. Вибір:
checked за замовчуванням у debug-збірці й у стор-коді, unchecked у кернелах за прапорцем -- рішення
оператора (D11-клас). TRAP-80 лишається (bump-арена кінчається), TRAP-81 (frame heap) ЗНИКАЄ, якщо агрегати
(літерали масивів, ctor'и, struct-літерали) живуть у типізованих таблицях арени, а не у x14 -- і разом з
ним: 16 KiB фрейм (B4 стає тривіальним: фрейм = 80 + 8*marks + 8*slots, глибока рекурсія 16 KiB -> < 1 KiB
на активацію, TRAP-82 «deep recursion» відсувається у 20x), `emit_prologue`'s `add x14,sp,#1024`, T118-trap
слова (7 слів на кожен літерал), `count_word(mov x0,x14)`-скан. АЛЕ чесно: `loop_alloc_safe` НЕ зникає, а
переїжджає -- арена не звільняється, тому літерал у тілі циклу на 1M ітерацій = 16 MB арени; потрібен
mark/reset арени на back-edge (genarena.bp mark/reset уже є) з тим самим escape-сканом, що сьогодні
(«чи тікає індекс з ітерації»). Різниця: помилка escape-скану сьогодні = use-after-release у frame heap
(тихо), з індексами = читання перезаписаної комірки арени (так само тихо) -- безпека не росте, поки
індекс не типізований поколінням (generation у заголовку таблиці + перевірка = ще 2 слова).
Alias-аналіз для плоского IR: індекс у таблицю A не аліасить індекс у таблицю B -- це те, що Rust отримує
з ownership (`noalias` з 1.54), а bebop отримає з ТИПУ таблиці (T48 `[T]`/`ref T`), не з індексів як таких:
два індекси в одну арену аліасять так само, як два вказівники. Тобто виграш (c) = «типізовані таблиці»,
індекси -- лише їх представлення.

**(d) Персистентність і спільне використання.** Стор уже позиційно незалежний (G2). Індексна модель робить
таким і RAM: образ арени (використана частина, `x27 - base`) = файл через `sys_export` без жодної
трансформації -- checkpoint процесу за ~DRAM-швидкості (40 MB ~ 10-30 ms, LANG-DB §4e), відновлення =
`sys_mmap` за будь-якою адресою + один `ldr` кореня. Це і є single-level store у програмному сенсі
(RESEARCH-TENSOR §7-5), уже реалізований для стору й розширюваний на арену. Спільний доступ: кілька
процесів/ядер мапують той самий файл індексів без релокації (nn4-шардинг по ядрах працює вже; між
процесами -- те саме через MAP_SHARED). Crash-consistency зводиться до «який корінь-індекс опубліковано»
= root swap 2 суперблоків (частина 2).

## 1.3 Ціна й недоліки (чесний список)

1. +1 `add` на доступ до масиву/поля, поки база не в регістрі й немає LICM: +5-8 % слів самокомпіляції,
   +10-20 % на codegen-bound сканах, 0 на DRAM-стелі та на K1H-K8H.
2. Тиск на регістри: одна глобальна база безкоштовна (x17); кожна додаткова арена/таблиця в одній функції
   = SYM-регістр (з 8) або +1 слово + 4 такти на доступ.
3. Узагальнений код «над будь-яким масивом» (csr_from_edges бере `es/ed/ew/rp/ci/vv` -- 6 масивів) мусить
   передавати (таблиця, індекс) або тримати всі в одній арені; в одній арені -- жодного виграшу по aliasing.
4. u32 обмежує 4G елементів на таблицю і вимагає нового типу з wrap-семантикою в bpref.
5. Frame heap -> арена: витік у циклах без mark/reset; mark/reset потребує того самого escape-аналізу.
6. Bounds checks: 1-2 слова на доступ; вимикати треба вміти (кернели).
7. Дві семантики адреси в мові одночасно під час міграції (вказівник у регістрі, індекс у комірці) --
   рівно та «третя річ», проти якої LANG-DB §6 застерігає («absolute pointers creeping back»): без T48-типів
   G2-двобазовий гейт -- єдиний ловець.
8. Стек викликів, LR, sp -- лишаються адресами (ISA); «повністю без вказівників» = без них у ДАНИХ.

## 1.4 Міграційний шлях (measured-first, три кроки, кожен -- один коміт через chain)

| крок | що | рядків | гейт | взаємодія |
|---|---|---|---|---|
| 0 (безкодовий) | census: `tools/check_abi.py`-стиль скан «`str` регістра, похідного від x27/x14, у комірку мапи стору» (LANG-DB §6 рядок «absolute pointers creeping back») + перф-рядок `ptr_stores` | ~60 python | інваріант у battery | нічого не змінює в кодогені |
| 1 | арена-відносна адресація: x17 = `sys_arena_base()` у entry_stub (seed не міняється: builtin є), `zeros` повертає ІНДЕКС комірки, `a[i]`/`a[i]=v` = `add t,a,i ; ldr/str [x17,t,lsl 3]`, `sys_*` builtins додають базу самі, `sys_export` арени = checkpoint. bpref: масиви = індекси (він уже моделює арену як список). Семантика програм не міняється (індекс -- це i64) | ~150 emitter + ~40 bpref + 2 конструкти (c69_index_roundtrip: `zeros` -> export -> mmap за іншою базою -> fold; c70_ptrfree: жодного `str` абсолютної адреси в комірку, перевірка census) | chain --codegen GREEN; std_golden 99; K6/sgraph folds; перф: bin_words, k6 ns/row (очікування +10-20 % до LICM), K5 +5-8 % | регістрова модель: `add` -- ще один REG-тег, база x17 -- нова фіксована; плоский IR (item 12) потім знімає `add` через LICM |
| 2 | агрегати в типізовані таблиці арени: `[e0..]`, enum ctor, struct literal -> `zeros`-подібна алокація з mark/reset на back-edge замість x14; x14, T118-слова, exit 81, `count_word(mov x0,x14)`, `real_alloc`-факт -- видалити; B4 фрейм = 80 + 8*marks + 8*slots | ~250 emitter (−200 видалених) + LANGUAGE.md «Frame heap» + bpref | c33/c34/c40 переморожені; c67_deeprec (рекурсія 10^5 глибини не падає: TRAP-82 зсув); fuzz TRAP-81 клас = 0 | B4 (item 9) стає частиною цього кроку; `loop_alloc_safe` лишається як «чи можна reset» |
| 3 | байтова арена + `str` як значення `(off<<32|len)`: `char(s,i)` = `ldrb [x18? або x17+off]`, `str_len` = `s & 0xffffffff`, `sys_readbuf`/`sys_mmap` повертають handle, літерали = handle у data-секцію, raw-парсер (RESEARCH-DEPS §6d-8), `crc32x` по байтах; sys_read у комірки лишається як legacy | ~300 emitter/builtins + ~80 bpref + LANGUAGE.md («strings as values» стає правдою) | c68_strval (рядок як значення через виклик і масив), ingest-twin 100 MB (частина 2.3) | T48 типи (`str`, `[u32]`) -- цей крок і є місце, де вони потрібні; u32-таблиці -- окремий 4-й коміт (~200) |

Порядок відносно ROADMAP: крок 0 -- будь-коли (python); кроки 1-3 -- після item 4 (LIN) і до item 12
(плоский IR), бо IR має бачити вже індексну модель (alias по таблицях), а B4 (item 9) зливається з
кроком 2. Оцінка сумарно ~700 рядків bebop.bp нетто (з видаленням frame heap ~−200), три chain-коміти.

---

# Частина 2. Проти SQL як класу: що є, чого бракує, що міряти

## 2.1 Цілісність, ACID, стійкість до збоїв

**Інвентар (усе виміряно або гейтоване у дереві):** append-only арена в файлі, ніколи не перезаписується
крім альтернативного суперблока (LANG-DB §4b-4c); commit = запис ІНШОГО суперблока з generation+1 і crc32
останньою коміркою (root swap, LMDB-патерн: «readers do not block writers, writers do not block readers»,
symas.com/lmdb, fetched); crc32x у h1 кожного об'єкта (T109b, 8 B/такт); читач = mapping (снапшот без
локів, без reader-table, бо сторінки не перевикористовуються; munmap = release); single writer через
`sys_atomic_add` + futex у процесі (G6: 4 писачі x 10^4 інкрементів, 0 втрат) або O_EXCL lock-файл між
процесами; G5 `scrash`: SIGKILL у випадковий момент x100 -> 0 збоїв, generation ∈ {last, last-1}
(T113); durable commit = `sys_msync` доданого діапазону + суперблока: 506 us проти sqlite WAL
synchronous=NORMAL 78 us (без fsync на commit) і FULL 567 us (T116: «same class as fsync-per-commit»);
compaction = Cheney у `<store>.tmp` + rename (G4), migration table (G3); F2 atomic publish для артефактів
(cli_compile bebop.bp:5277: tmp + MAP_SHARED + rename). Ізоляція фактично: snapshot isolation для читачів,
serializable для писача (один писач). Це РІВНО модель SQLite (WAL: «there can only be one writer at a
time», sqlite.org/wal.html, fetched) і LMDB (single writer, MVCC readers) -- тобто «SQL як клас» у його
наймасовішій реалізації сам single-writer. Багатописачевий MVCC (Postgres) -- інший клас, див. «never».

**Чого бракує, і мінімальний дизайн без залежностей:**

| # | прогалина | що є | мінімальний дизайн | рядків | twin / гейт |
|---|---|---|---|---|---|
| A1 | доказ power-loss (не лише kill -9) | f2fs `fsync_mode=nobarrier`: msync повертається без flush кешу пристрою; PMU/відрізання живлення нема (LANG-DB §3) | (i) torn-write емулятор: після кожного commit'а харнес копіює файл, а потім у копії обнуляє/обрізає випадкову ПІДМНОЖИНУ сторінок, змінених після останнього msync-бар'єра (модель «лінійний, неатомарний запис сектора» SQLite atomiccommit, fetched: «if any part of the sector gets changed, then either the first or the last bytes will be changed»); reopen мусить обрати попередній валідний суперблок і crc-чистий ланцюг; (ii) порядок записів: суперблок пишеться ПІСЛЯ msync даних (є в st_commit_sync); додати `fsync` каталогу після rename компакції (renameat без fsync dir -- ім'я може не пережити збій; один builtin `sys_fsync(fd)` 6 слів); (iii) рядок «durable on a `barrier`/`strict` box» -- forward-port, як §6 каже | ~150 python (розширення scrash.sh) + ~30 bebop (sys_fsync) | G5b: 1000 trials torn pages -> 0 невалідних reopen'ів; sqlite WAL той самий харнес (обрізання wal-файлу) для чесності: обидва мають вижити |
| A2 | WAL / redo | не потрібен: append-only + root swap = after-image log без checkpoint-back (§4c); журнал = друга «правда» (T73 закон) | нічого; лише документувати еквівалентність: «commit record» = суперблок, «checkpoint» = compaction | 0 | -- |
| A3 | group commit / пропускна commit'ів | 506 us на durable commit = 2-3 msync по ~100 us | batch: N транзакцій -> один msync діапазону + один суперблок (є природно: append-only); API `st_commit_batch` | ~40 | commits/s durable: очікувано ~2000/s -> ~20-50k/s у батчах по 100; sqlite FULL ~1700/s |
| A4 | ізоляція для кількох писачів | один писач; stm.bp (конфлікт = нільпотентний добуток) для арени | партиційні писачі: одна арена = один писач-потік, транзакція над кількома арені = 2PC у одному процесі через futex; або STM-валідатор при commit (read-set перевірка generation) -- лише якщо W цього потребує | 150-300 | не робити до W (order log -- один писач за визначенням) |
| A5 | recovery-час | вибір суперблока O(1) | виміряти: reopen після kill = 590 us (T116) vs sqlite WAL recovery (replay frames, ms) | 0 | рядок у sbench |
| A6 | детекція torn object поза курсором | crc32 h1 (лише при walk) | `st_verify(root)` фоновий прохід (Cheney-walk без копії) | ~60 | G4-стиль fold |
| A7 | обмеження регіону (Crotty mmap): датасет > RAM, truncation, 100 потоків | задокументовано §6 | лишити як межу застосовності: <= RAM, <= 8 потоків | 0 | -- |

«LMDB out of thesis» (D14 item 7) означає лише: теза ROADMAP більше не обіцяє ВИМІРУ проти LMDB (нема
скрипта); дизайн і далі копіює механізми LMDB (i)-(iv) (LANG-DB §1). Для аргументу «проти класу» LMDB
важливий інакше: він доводить, що single-writer MVCC над mmap без WAL -- індустріальний стандарт
(OpenLDAP), а не самодіяльність. ARIES (Mohan 1992: WAL + REDO/UNDO + fuzzy checkpoints; з пам'яті) --
модель для in-place сторінкових БД; append-only + root swap не потребує ні UNDO (нічого не перезаписано),
ні REDO (дані вже на місці, лише корінь), що і є LMDB-аргумент проти ARIES-складності.

## 2.2 Довільні декларативні запити

**Що є:** запит = `.bp`-функція над `ref T`/CSR (LANG-DB §4f: «No planner: the plan is the index chosen by
the programmer, exactly as in MUMPS»); індекс = CSR-bucket (T100 4.0 us, T116 2.7 us на 3x3-вікно); zone-maps і
tombstone-бітмапи (§9.5); T32 qjit; digest-memo кернелів (T108); компіляція кернела 50 ms (RESEARCH-DEPS §5.4).

**Specialise-then-run як планувальник.** Запит компілюється: (текст запиту або AST) -> `gen_query.bp`
пише `.bp`-кернел під схему/предикати/агрегати -> `bebop.bin compile` 50 ms (memo за digest: повтор 0) ->
run. Планувальник вироджується у (а) вибір шляху доступу на таблицю: скан / bucket по ключу / zone-map-skip
(3 варіанти, вибір за селективністю з `rp`-лічильників -- точна кардинальність bucket'а відома, не
оцінка), (б) порядок join'ів для k <= 4 таблиць: перебір k! <= 24 планів з вартістю = сума
(розмір входу x ціна доступу) -- тривіальний пошук, (в) fusion -- безкоштовний, бо кернел ЄДИНИЙ цикл.
Це рівно теза HyPer (Neumann, VLDB 2011 «Efficiently compiling efficient query plans for modern hardware»:
data-centric компіляція конвеєрів у LLVM замість інтерпретації операторів; PDF не розпарсився -- з
пам'яті: порядки над VDBE-подібними інтерпретаторами, 2-5x над векторизованими) і Umbra (Neumann &
Freitag, CIDR 2020: адаптивна компіляція, «flying start»; з пам'яті). DuckDB/Velox -- векторизована
інтерпретація без JIT, конкурентна з компіляцією на аналітиці: доказ, що обидва шляхи б'ють VDBE (sqlite:
11 ns/крок, 14 кроків/рядок, LANG-DB §8). bebop уже має рівно «компільований» шлях і платить 50 ms за
нову форму -- це його місце в цій таблиці.

**Мінімальна реляційна алгебра над стором (рядки bebop):**

| оператор | механізм | рядків | є? |
|---|---|---|---|
| scan + filter + project | згенерований цикл, zone-map skip, tombstone-маска | 80 (генератор) | скан є (nn.bp), zone-map §9.5 |
| bucket lookup / range | CSR `rp[k]..rp[k+1]`, сусідні bucket'и | є (nnidx, csr_scan) | так |
| hash join над індексами | radix-partition по ключу (counting sort -- це csr_build!) + probe: join = «побудувати CSR по ключу меншої сторони, сканувати більшу» | 150 | csr_build є |
| group-by | ключ -> bucket (counting sort) або radix для i64-ключів; агрегати sum/count/min/max в один прохід | 120 | частково (nnidx build) |
| order-by / top-k | radix sort i64 (4 проходи по 16 біт) / heap top-k | 120 | selection sort у csr.bp (замінити) |
| планувальник (а)-(б) | таблиця вартостей + перебір | 200 | ні |
| генератор `.bp` з AST запиту | шаблони рядків | 250 | ні (morph.bp -- прецедент публікації) |
| поверхня запиту | НЕ SQL: `.bp`-DSL (`q { from t where p group by k agg s }`) або JSON-подібний AST | 200 (парсер) | ні |

Разом ~1 100 рядків .bp -- і це «повноцінний реляційний планувальник роздує базу» оператора: ні, це 20 %
bebop.bp, бо планувальник над відомими індексами з точними кардинальностями -- маленький пошук, а
виконавець -- сам компілятор. Стелі чесно: нова форма запиту = 50 ms (rustc-клас латентності зникає, але
sqlite prepare = 20-50 us -- на one-shot ad-hoc запитах з унікальною формою sqlite виграє латентність на
3 порядки; виграш bebop -- на повторюваних формах і на всьому, що триває > 50 ms).

**Де SQL як клас виграє й далі:** (1) OLTP з багатьма писачами (Postgres-клас MVCC, блокування рядків); (2)
довільні join'и по 10+ таблицях без знання форми -- евристики оптимізаторів за 40 років (гістограми,
кореляції); (3) NULL/типи/строгі семантики стандарту, tooling, драйвери, BI; (4) one-shot запити з
унікальною формою (50 ms компіляції > усього запиту на малих даних); (5) вбудовуваність без компілятора в
рантаймі (bebop.bin 268 KB -- теж вбудовуваний, але це «компілятор у продакшені»).

**Twin:** TPC-H-подібні Q6 (filter+sum по 4 предикатах) і Q1 (scan + group by 4 групи x 8 агрегатів) над
lineitem SF 0.1 (600k рядків) у сторі vs sqlite 3.46.1 (python ctypes, prepared, VM_STEP; native = minus
ctypes floor, правило §8) vs DuckDB -- НЕ встановлюваний тут (перевірено: нема apt-пакета, нема pip-модуля;
додати = залежність), тому рядок DuckDB -- з опублікованих чисел, позначений як «не тут». Гейт: Q6 >= 10x
sqlite native, Q1 >= 5x, плюс рядок «перший запит включно з компіляцією» і «повторний (memo)». Це і є
ROADMAP item 6 (specialise-then-run twin) у конкретній формі.

## 2.3 Сирий ввід-вивід, bytes-in-cells

**Що є:** `sys_readbuf(fd, len)` (raw read у IO-зону), `sys_mmap` будь-якого файлу, `char(s,i)` = `ldrb` над
будь-яким raw-вказівником, `crc32x` над сирими байтами комірок, `sys_export` = mmap-запис без sys_write;
стор = mmap-сторінки без трансформації (це і є «slotted pages без slotted pages»: об'єкт = заголовок 2
комірки + payload, довжина при даних). **Чого бракує:** `sys_read`/`sys_slurp`/`str_to_cells`/`crc32` -- усе
на комірку-на-байт (8x пам'яті й пропускної: sys_slurp `round16(len*8)`); парсери (bebop.bp сам!) читають
`str` через `char`, але зберігають байти в комірках; нема `str` як значення.

**Дизайн (= крок 3 частини 1):** байтова арена (mmap файлу або IO-зона) + handle `(off,len)` в одному
i64; парсер над handle'ами через `char`/NEON `scan` builtin (RESEARCH-DEPS §1b(3)); crc на сторінку: таблиця
crc32x по 4 KB (100 MB = 25 600 x 4 B; обчислення ~10-15 ms при 8 B/такт) у зоні суперблока -- перевірка
лише сторінок, які читаються; буферний пул = page cache ядра (LMDB-аргумент; межі -- Crotty, §6). Слотовані
сторінки не потрібні: об'єкти append-only, компакція замість дефрагментації.

**Twin (один вимір закриває дві заявки оператора, §6d-8/9):** 100 MB line-oriented записів (id,u,v,cell +
текстове поле) -> CSR у сторі: bebop raw (handle + char/scan) vs bebop cells (sys_slurp + str_to_cells) vs
sqlite `.import`-еквівалент (python executemany у транзакції; T116 показав insert 1M = 15 s через ctypes,
~0.3-0.5 s native оцінка LANG-DB §5) vs Rust memmap2 + winnow (best) vs Rust serde-owned (common). Гейт:
ms/MB, maxrss (raw path <= 1.5x розміру файлу проти 8x у cells), minor faults; чесний прогноз: raw ≈
Rust-best 1-1.5x, cells 4-8x гірше за raw, sqlite import 10-30x гірше за raw.

## 2.4 Синтез

**Можна захищати вже (виміряно, гейти зелені):** аналітичні скани 9.9x vs sqlite (стеля 80-90x після
регістрової моделі + NEON-builtin, 150x з u32); граф BFS 57x vs recursive CTE, frontier 4.3x зверху; point
lookup ~9x native (13.8x через ctypes); insert 17x, update-batch 6.4x, reopen 6.1x (T116); latency-to-result
з компіляцією 27x vs rustc, 50 ms на кернел; kill -9 стійкість 100/100 (G5), атомарний багатооб'єктний
commit (root swap), snapshot isolation читачів без локів (G6), durable commit того самого класу, що sqlite
FULL (1.1x); position-independent образ (G2), еволюція схеми (G3), компакція (G4); нуль залежностей,
self-host, 268 KB компілятор. Це аргументи проти SQL-РЕАЛІЗАЦІЙ в аналітиці й проти класу «інтерпретовані
плани над B-деревами» (VDBE-податок і miss-податок, LANG-DB §8 -- дві з чотирьох «податків» реальні).

**Можна захищати після (з гейтом):** (1) durability на power-loss -- після G5b torn-write харнесу + fsync
каталогу (A1) і рядка на `barrier`-боксі; (2) ad-hoc запити -- після компільованого планувальника + Q1/Q6
twin (2.2), із чесним застереженням про 50 ms на нову форму; (3) ingest/пам'ять -- після raw-byte шляху й
100 MB twin (2.3); (4) розмір файлу (2.1-2.5x програш) -- після u32-комірок; (5) compaction 0.7x vs VACUUM --
профілювати st_get/st_put (T117 build 44.8 s «to be profiled» -- той самий overhead бібліотечних викликів,
який регістрова модель зменшить, а індексна модель + inline (плоский IR) приберуть).

**Не заявляти ніколи:** багатописачевий OLTP Postgres-класу (single writer -- це дизайн, як у SQLite/LMDB);
стандартну SQL-поверхню й екосистему (драйвери, BI, ORM); довільні 10-way join'и без знання форми на
рівні зрілих оптимізаторів; one-shot ad-hoc з унікальною формою на малих даних (50 ms компіляції); датасети
понад RAM (mmap-режим, Crotty); «zero-copy 50-100x проти Rust» (RESEARCH-DEPS §6d-8: ~1x проти найкращого
Rust).

**Ранжування частини 2 за «аргумент на рядок bebop», з гейтом і місцем у ROADMAP:**

| # | пункт | рядків | аргумент, який відкриває | гейт | місце |
|---|---|---|---|---|---|
| 1 | G5b torn-write harness + `sys_fsync` каталогу (A1) | ~150 py + ~30 bp | «переживає збій живлення в моделі SQLite atomiccommit», не лише kill -9 | 1000 trials, 0 невалідних reopen; той самий харнес над sqlite WAL | STORE PULL, одразу після T117 stage 2; не залежить від items 2-13 |
| 2 | raw-byte шлях + `str` як значення + 100 MB ingest twin (2.3 = частина 1 крок 3) | ~380 bp + ~80 bpref | ingest 10-30x vs sqlite import, пам'ять 8x -> 1x, рядки як значення (поверхня) | ms/MB, maxrss, 5 рядків twin | після item 4 (LIN), перед item 12 (IR) |
| 3 | компільовані Q6/Q1 кернели + генератор + мінімальний планувальник (2.2) | ~1 100 bp | «ad-hoc аналітика без SQL-інтерпретатора», закриває item 6 twin | Q6 >= 10x, Q1 >= 5x sqlite native; рядки «перший/повторний» | = ROADMAP item 6, розширений |
| 4 | u32-комірки в сторі/CSR (частина 1, (b)) | ~200 bp + тип | розмір файлу 1x, BFS/scan трафік 2x | G7 size row <= 1.2x sqlite; K6 ns/row | після кроку 2 частини 1, разом із T48 |
| 5 | агрегати в арені замість frame heap (частина 1 крок 2) + B4 | ~250 bp (−200) | TRAP-81 зникає, рекурсія 20x глибша, checkpoint арени | c67_deeprec, fuzz TRAP-81 = 0 | = ROADMAP item 9 (B4), злитий |
| 6 | арена-відносна адресація x17 (частина 1 крок 1) | ~190 | образ процесу = файл; підготовка alias-типів для IR | c65/c66, K5 +5-8 % прийняти або LICM | після item 4, перед item 12 |
| 7 | group commit (A3), st_verify (A6), recovery row (A5) | ~100 | commits/s, самоперевірка | sbench рядки | STORE PULL, після 1 |
| 8 | multi-writer (A4) | 150-300 | лише якщо W вимагає | -- | не зараз (W = order log, один писач) |

Порядок: 1 (python, паралельно з чим завгодно) -> після регістрової моделі/csel/LIN: 6 -> 5 -> 2 -> 4 ->
3 -> 7; 8 -- не планувати. Сумарно ~2 000 рядків bebop за 6-7 chain-комітів дають три «after»-аргументи
(durability, ingest, ad-hoc), і жоден не додає залежності.

## 3. Докази

- Repo: docs/LANGUAGE.md (Memory model, builtins, «What is NOT»: strings as values, bounds checks);
  docs/LANG-DB-DESIGN.md §2 inventory (:68-98), §3 physics (:102-138: faults 2.5-7 us, msync 100 us,
  rename 270 us, `fsync_mode=nobarrier`, crc32 2.42 GB/s, DRAM 12 GB/s), §4a-4h design, §5 gates, §6
  risks («absolute pointers creeping back», Crotty regime), §8 four taxes (VDBE 11 ns/step, 158 ns/row,
  seek ~1 us), §9.4-9.5 (block-CoW updates, tombstones); HISTORY.md STORE PULL (:2767-2845: T109-T117 --
  G1-G8 numbers: G5 100/100 SIGKILL, G6 40000, G7 17x/22.8x/30.7x/6.4x/6.1x, size 2.5x loss, compaction
  0.7x, durable 506 us vs 78/567 us; G8 BFS 57x, frontier 45 vs 192 ns/slot, build 44.8 s «to be
  profiled»); HISTORY.md:2479-2480 (D14: LMDB/native Rust out of the thesis sentence; W = order log);
  bench/tq_sqlite/RESULT.md (9.9x scan, 13.8x window, nn4 2.21x); selfhost/std/csr.bp:1-12 (rp/ci/vv
  indices), sgraph.bp:3-13; selfhost/prelude/store.bp:84-247 (st_open/begin/alloc/commit/snapshot/
  commit_sync/compact); bebop.bp cli_compile:5277-5292 (F2 atomic publish), emit_sys_msync:1406,
  emit_sys_export:5458, emit_sys_mmap:5502, renameat:5536-5540; docs/TRAPS.md (80/81/82/89);
  docs/REGISTER-MODEL-BLUEPRINT.md §1.1 (register map: x16 scratch; x17/x18 unused);
  docs/RESEARCH-DEPS-2026-09-06.md §6d-8/9 (zero-copy / zero-alloc honest ratios), §1b(3) scan builtin,
  §3 flat IR; docs/RESEARCH-TENSOR-2026-09-06.md §2 (K6 DRAM ceiling 2 ns/row, 24 B/row), §7-5
  (single-level store reading).
- This session: /proc/mounts (f2fs `barrier` on, `fsync_mode=nobarrier`); `python3 -c "import sqlite3"`
  3.46.1; duckdb: no apt package, no module; 24 processes.
- Fetched: symas.com/lmdb («readers do not block writers, writers do not block readers», «full ACID
  semantics with MVCC», «requires no ... logs, or crash recovery procedure»); sqlite.org/wal.html («there
  can only be one writer at a time», end mark = snapshot, checkpoint starvation, synchronous NORMAL vs
  FULL); sqlite.org/atomiccommit.html (linear non-atomic sector writes, first/last bytes rule, fsync
  reliance, «flush and fsync primitives are broken on some versions»); kernel.org f2fs doc
  (`fsync_mode=nobarrier`: «doesn't issue flush command»; `nobarrier`: «no cache_flush commands are
  issued but f2fs still guarantees the write ordering»). From memory (PDF failed to parse / not fetched):
  Neumann VLDB 2011 HyPer data-centric compilation; Neumann & Freitag CIDR 2020 Umbra; Mohan et al. 1992
  ARIES; Crotty et al. CIDR 2022 «Are You Sure You Want to Use MMAP»; Hack/Goos not relevant here.

VERDICT: pointer-free bebop = «індекси в даних, вказівники лише в регістрах»: одна глобальна база x17 робить індекс безкоштовним
у формі `ldr [x17,idx,lsl 3]`, ціна +1 add на масивний доступ до LICM (K5 +5-8 %, скани +10-20 % до DRAM-стелі, кернели 0),
виграш = frame heap/TRAP-81 геть (B4 тривіальний), u32-таблиці 2x трафіку й розмір файлу 1x, байтова арена + `str` як значення
(8x пам'яті IO геть, strings as values), образ процесу = файл (single-level store на арену); три chain-коміти ~700 рядків після
LIN і до плоского IR. Проти SQL як класу: сьогодні захищаються скани/граф/індекс/латентність/kill-9/snapshot-readers/zero-deps;
після (1) torn-write харнесу + fsync dir (~180 рядків), (2) raw-byte ingest twin (~460), (3) компільованих Q6/Q1 + планувальника
(~1 100) -- durability, ingest і ad-hoc; ніколи -- multi-writer OLTP, стандартна SQL-поверхня, one-shot унікальні форми < 50 ms,
датасети > RAM; жодна залежність не додається.
