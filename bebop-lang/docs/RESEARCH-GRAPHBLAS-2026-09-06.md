Status: 2026-09-06 (session 18) -- research report by a Fable fork, read-only over /root/dowiz/bebop-lang at HEAD c96bc1b
(register-model worker in flight; "after register model" numbers are the RESEARCH-DEPS/RESEARCH-TENSOR forecasts). Box facts
reused: Cortex-A78 under proot, DRAM ~12 GB/s stream (one A78 saturates it), random line miss ~100 ns dependent / 10-20 ns
pipelined, no SVE, NEON 2 x i64 lanes without gather and without 64-bit integer multiply. Measured store rows reused: K6 scan
18.4 ms per 1M rows (18 ns/row, codegen-bound); G8 BFS queue 187-240 ns per edge slot, frontier (Beamer push/pull, alpha 14)
45 ns/slot = 4.3x, sqlite recursive CTE 10.8 us/edge = 57x; edge log 30 us amortised per logged edge with a 747 ms max stall
(the O(N) L0 rebuild); neighbour query after the log ~1.1 us (3 slices); G7 insert 1M + CSR + commit 880 ms vs sqlite 15.1 s;
durable commit 506 us; kill -9 100/100; compaction 795 ms per ~120 MB. Web: SuiteSparse README (JIT kernels cached in
~/.SuiteSparse, "compiles JIT kernels" at run time -- fetched); GraphBLAS-Pointers (LAGraph algorithm list, C API v2.0.0, MAGiQ,
Graphulo, pggraphblas, "relational joins and sparse matrix multiplication" papers -- fetched); RedisGraph paper abstract
arXiv:1905.01294 ("represents connected data as adjacency matrices" -- fetched); Kepner/Chaidez associative arrays
(arXiv:1501.05709, 1606.05797 "Associative Array Model of SQL, NoSQL, and NewSQL" -- search results). From memory (marked M):
SuiteSparse formats/pending tuples/zombies (TOMS 2019 paper returned 403), Deep et al. SIGMOD 2020 "Fast Join-Project Query
Evaluation using Matrix Multiplication", SPORES VLDB 2020, FalkorDB delta matrices, Okasaki 1998, Beamer SC 2012.

# GraphBLAS + чисто функціональні оновлення тензорів над стором bebop: чи це тензорно-графова БД замість SQL

## 0. Терміни й позиція

**GraphBLAS** тут означає рівно одне: граф = розріджена матриця над напівкільцем, а обхід/агрегат = добуток
матриця×вектор або матриця×матриця над вибраним напівкільцем (Kepner & Gilbert 2011; C API v2.0.0,
graphblas.org/docs/GraphBLAS_API_C_v2.0.0.pdf; SuiteSparse:GraphBLAS = еталонна реалізація, LAGraph = алгоритми поверх
неї). Стор bebop уже є цим у зародку: CSR `rp/ci/vv` (csr.bp, sgraph.bp), `csr_from_edges`/counting sort = побудова
матриці з COO, frontier-BFS на бітмапах = SpMSpV над `or-and` (45 ns/slot, T117), `spmv_fp` = `plus-times` над Q32
(spectral.bp:29), tombstone-бітмапа = структурна маска, zone-map = маска блоків, `csr_scan(idx,k)` = extract рядка.
**Вердикт: GraphBLAS — не нова архітектура для bebop, а НАЗВА того, що стор робить, плюс ~8 операцій, яких бракує
(mxm, eWiseAdd/Mult, select, reduce по модах, assign), і дисципліна «напівкільце — параметр компіляції».**

**Purely functional tensor updates** тут означає: матриця ніколи не змінюється на місці; оновлення повертає НОВУ версію,
що ділить із попередньою незмінені блоки (Okasaki 1998, persistent data structures; функціональні масиви як дерева
чанків; append-only лог як канонічна персистентна структура). Стор bebop уже є цим на рівні об'єктів (LANG-DB §4b-4c:
append-only арена, root swap двох суперблоків, «never in place» mvcc.bp:48) і на рівні блоків (§9.4 block-CoW 4 KB);
чого бракує — це щоб МАТРИЦЯ була таким об'єктом (блочна таблиця рядків + дельти), а не «перебудувати L0 цілком»
(T117 stage 2: 747 ms stall). **Вердикт: функціональність — не властивість, яку треба додати, а властивість, яку треба
ПЕРЕНЕСТИ з об'єктів на матриці: row-block CoW + дельта-матриці, злиття при commit = те, що SuiteSparse робить
pending tuples у пам'яті, тільки персистентно.**

**«Замінити SQL».** Чесно й одразу: над стором можна побудувати компільовану тензорно-графову БД, що б'є SQL-реалізації
на сканах, графах і повторюваних формах запитів на 1-2 порядки (уже виміряно 9.9x/57x/17x) і зрівнюється з Rust; але
вона не замінює SQL як КЛАС там, де клас виграє: багато писачів, довільні n-way join'и без знання форми, стандартна
поверхня. **Вердикт: заміна SQL для аналітики/графів/агрегатів над відомими схемами — так, після §4; для OLTP і ad-hoc
SQL-поверхні — ні, і це не змінює жодна алгебра.**

## 1. GraphBLAS над стором

### 1.1 Об'єкти, операції, напівкільця → що є, чого бракує

Об'єкт `GbMatrix` у сторі = один store-об'єкт `{n, m, nnz, fmt, ref rp, ref ci, ref vv, ref mask, gen}` (h0/h1 як §4a);
`GbVector` = dense-масив (n комірок) або sparse (відсортовані індекси + значення) — бітмапа фронтиру вже є другим
форматом. Формати SuiteSparse (M): sparse CSR/CSC, hypersparse (список непорожніх рядків), bitmap, full, з автоперемиканням
за щільністю; iso-valued (усі значення рівні → зберігати одне). Для bebop: CSR + bitmap (є) + hypersparse лише як
«рядки з ненульовим degree» (zone-map за рядками) + iso (маска `or-and` без `vv` — sgraph уже так робить: CI без ваг).

| GraphBLAS op | семантика | механізм у bebop сьогодні | бракує | рядків (шаблон кернела) |
|---|---|---|---|---|
| `mxv` / `vxm` (push) | y<mask> = A·x над напівкільцем | frontier SpMSpV `or-and` (sgraph2 phase p, 45 ns/slot); `spmv_fp` plus-times (spectral.bp:29, ~30 ops/nnz через schoolbook fp_mul) | напівкільце як параметр; dense/sparse x; вихід у CSR-рядок або dense | 60 |
| `vxm` (pull) | те саме по стовпцях | pull-фаза Beamer над A^T (sgraph зберігає обидва напрямки = A і A^T як два CSR) | descriptor `transpose` = «взяти інший CSR» | 0 (той самий шаблон) |
| `mxm` (SpGEMM) | C<M> = A·B | немає | Gustavson по рядках: для рядка i: для k у A[i,:]: для j у B[k,:]: SPA[j] ⊕= a_ik ⊗ b_kj; SPA = dense акумулятор n + список торкнутих (це і є hash join, §1.2) | 120 |
| `eWiseAdd` / `eWiseMult` | union / intersection двох розріджених рядків | немає | merge двох відсортованих рядків (2 вказівники), ci вже відсортовані (`csr_from_edges` контракт) | 60 |
| `apply` | f(a_ij) поелементно | fold-цикли по `vv` є ad hoc | шаблон над `vv` з маскою | 20 |
| `select` | залишити (i,j,v) за предикатом | tombstone-бітмапа (структурне видалення, §9.4), zone-map skip | предикат по значенню → новий CSR: 2 проходи (count, fill) = `csr_build` | 40 |
| `reduce` (по рядках / стовпцях / до скаляра) | ⊕ вздовж моди | `sum_words`, degree = `rp[i+1]-rp[i]` | по рядках = один прохід; по стовпцях = reduce над A^T | 30 |
| `extract` (рядок/підматриця) | A[i,:] / A[I,J] | `csr_scan(idx,k)` = `ci[rp[k]..rp[k+1])` (є, nnidx/T100) | підматриця = select за рядками + `apply` перенумерації | 40 |
| `assign` / `subassign` | A[I,J] = B | немає в матричній формі; edge log + L0 (§9.3) | §2: row-block CoW + дельта | 150 (§2) |
| `transpose` | A^T | sgraph build пише обидва напрямки (counting sort) | `csr_build` над (dst,src) — є | 0 |
| `kronecker` | A ⊗ B | немає | не потрібен для W; пропустити | 0 |
| маска / комплемент | `C<M>`, `C<!M>` | tombstone-бітмапа, visited-бітмапа BFS | маска як параметр шаблону: `and` слова бітмапи в циклі | 10 |
| accumulator | C = C ⊕ (op) | немає | eWiseAdd результату в наявну версію = злиття дельти (§2) | входить у eWiseAdd |
| descriptor | transpose/replace/structural | ad hoc | 3 прапорці генератора | 10 |

**Напівкільця, що потрібні** (⊕ монойд, ⊗ бінарна op; кожне = один згенерований кернел на op): `or-and` / `any-pair`
(досяжність, BFS, існування join'а; `any` дозволяє зупинку на першому — SuiteSparse використовує саме `any-pair` для
BFS, M); `plus-times` (PageRank/SpMV; у bebop — над Q32 fixed-point: ⊗ = `umulh`-клас, див. §4 builtin); `min-plus`
(SSSP); `plus-second` / `max-first` (join із переносом payload'у правої/лівої сторони); `plus-pair` (triangle counting:
`C<A> = A·A` над plus-pair, потім reduce до скаляра / 6). Разом 6 напівкілець × ~6 op = ≤ 36 кернелів, кожен ~30-120
рядків згенерованого .bp; генеруються, не пишуться руками.

### 1.2 Реляційна алгебра як лінійна (асоціативні масиви Кепнера)

Таблиця з ключем k = розріджена матриця `T[k, rowid]` (або `T[k, attr]` як асоціативний масив; Kepner & Chaidez
2015: «associative arrays ... bridge spreadsheets, databases, matrices, and graphs», arXiv:1501.05709; «Associative Array
Model of SQL, NoSQL, and NewSQL» arXiv:1606.05797). Колонки = вектори SoA (у сторі: `arr i64` на колонку — nn.bp уже
так). Відповідність:

| реляційна op | лінійна op | кернел bebop (specialise-then-run) |
|---|---|---|
| selection σ_p | mask/select: M = p(колонки) → бітмапа; результат = `C<M>` | один злитий цикл: zone-map skip блоку → `cmp_mask` builtin по колонці → бітмапа; predicate = константа компіляції |
| projection π | extract колонок | нуль байтів (SoA: не читати колонку — це і є проєкція) |
| join R ⋈_k S | `R^T[rid,k] · S[k,sid]` над `any-pair` (існування) або `plus-second` (payload) | SpGEMM = «побудувати CSR S по k (counting sort, є), для кожного rid пройти рядок S[k]» = hash join з CSR замість хеш-таблиці |
| group-by k, agg ⊕ | reduce вздовж моди rowid після перенумерації ключа: `G[k,rid] · v` над (⊕, second) | ключ → bucket (counting sort, `csr_build`) або dense акумулятор розміру #груп (Q1: 6 груп → 6×8 регістрів/комірок, без сортування) |
| transitive closure / reachability | повторний `mxv` над `or-and` з accumulate | frontier BFS (є) |
| top-k / order-by | select за порогом + radix sort i64 | radix 4×16 біт (~120 рядків), замість selection sort у csr.bp |
| count distinct | reduce над `or-and` потім popcount | бітмапа + `cnt` (hvham-прецедент) |

**Q6-подібний** (`sum(l_extendedprice*l_discount) where shipdate in [a,b) and discount in [c,d] and quantity < q`):
у GraphBLAS-термінах `reduce(eWiseMult(price, discount)<M>)`, M = ∧ трьох select-масок. Згенерований кернел — один
цикл по блоках: `zone-map(shipdate,quantity) skip` → `cmp_mask` на 3 колонках → для живих бітів `madd acc, price, disc`
(Q32 → `umulh`) → `sum64`. Це рівно §1b-fusion RESEARCH-DEPS без IR: fusion безкоштовний, бо кернел єдиний. Очікування
після регістрової моделі: 3 колонки × 8 B = 24 B/row → DRAM-стеля 2 ns/row; sqlite 158 ns/row (VDBE 14 кроків) →
**~50-80x**, Rust (LLVM auto-vec над SoA) ~1.5-2 ns/row → ~1x.

**Q1-подібний** (group by returnflag, linestatus; 8 агрегатів sum/avg/count): 6 груп → ключ = `flag*3+status` →
dense акумулятори [6][8] у сторі/регістрах; кернел: один прохід, `madd` на групу; `avg` = sum/count після циклу. Ніякого
сортування: reduce вздовж моди з відомою малою кардинальністю. sqlite: ~300 ns/row (група через B-tree temp), bebop
~5-10 ns/row → **~30-50x**; Rust ~1x.

**2-way join** `R(k,a) ⋈ S(k,b)` при |R|=|S|=1M, ключі 1M з множинністю ~1: `CSR_S = csr_build(S by k)` (3 проходи по 24 MB
≈ 5-10 ms на DRAM-швидкості; сьогодні 50-100 ms); потім для кожного r: `ci_S[rp_S[k_r]..rp_S[k_r+1])` — один випадковий
рядок на r: ~1 line miss (10-20 ns пайплайновано) + вихідні пари. Це ТОЙ САМИЙ доступ, що hash join (bucket = рядок CSR,
без ланцюжків колізій, без resize) — паритет із Rust `HashMap`-join за пам'яттю (обидва ~2 miss/probe), і виграш над
sqlite indexed nested loop (1-2 us/probe через B-tree seek + VDBE) **20-50x**. Вибух кардинальності при множинності m:
вихід = Σ m_r·m_s — однаково для SQL і для SpGEMM (Gustavson рахує рівно те саме); GraphBLAS не рятує від поганого
join'а, він робить його передбачуваним (`rp` дає точну множинність ДО виконання = точна кардинальність для планувальника,
NOPOINTERS §2.2).

**Multi-hop / шаблон Cypher** `(a)-[R1]->(b)-[R2]->(c)`: `R1·R2` над `any-pair` (RedisGraph: «represents connected data as
adjacency matrices», одна матриця на тип зв'язку, Cypher-патерн → алгебраїчний вираз; arXiv:1905.01294). Для bebop:
матриця на relation = один CSR у сторі; ланцюжок = `mxv` від фронтиру (не `mxm`: `mxm` матеріалізує всі шляхи —
використовувати лише коли потрібна вся C).

### 1.3 Ранг 3+: гіперребра, час як мода

CSF (compressed sparse fiber) = вкладений CSR: `rp` на кожну моду; COO з порядком мод = сортування за (m1,m2,m3) —
counting sort по старшій моді, стабільно. Стор: «ще один рівень `rp`» = ще один `arr i64` + `ref` у заголовку матриці;
нічого нового в §4a. **Чи W (order log) має ранг > 2?** Подія = (order_id, from_state, to_state, t). Вигляди: (1)
`order → events` — CSR за order (rp по order, ci = індекси подій) — ранг 2; (2) FSM `from × to` — 12×12, `A^12 == 0`
нільпотентна, бітмаски (ordfsm.bp) — ранг 2 dense; (3) `t` — лог append-only і вже впорядкований за часом → час = zone-map
(min/max t на блок) над (1), не мода. Ранг 3 (time × order × state) потрібен лише для запитів «стан усіх замовлень на
момент t» разом із «історія одного замовлення» — і тоді це ДВА CSR з різним порядком мод (індекс за t-bucket + індекс за
order), тобто вибір індексу = вибір порядку мод = рішення планувальника specialise-then-run. **Вердикт: CSF не потрібен
для W; «мода» = ще один CSR.**

## 2. Чисто функціональні оновлення тензорів

### 2.1 Що вже персистентне

Об'єкти: append-only + root swap + `never in place` = кожна генерація g — повна версія стору, читач із mapping'ом бачить
рівно g, поки живе mapping (§4c; G5/G6 виміряно). Блоки: §9.4 block-CoW 4 KB для бітмап і чанкованих масивів. Матриці —
НІ: sgraph stage 2 оновлює через edge log → повна перебудова L0 (O(N) на батч, 747 ms stall, 30 us/edge amortised) — це
«персистентність через копію всього», найдорожчий із можливих функціональних масивів.

### 2.2 Row-block CoW: матриця як персистентний вектор блоків

Структура (усе — store-об'єкти, `ref`-и object-relative):
```
GbMatrix v_g : {n, m, nnz, fmt, ref blocktab, ref mask, gen}
blocktab      : 2 рівні, fan-out 512: root[ceil(n/64/512)] -> page[512] -> ref RowBlock   (1M рядків: root 32 refs, 32 pages)
RowBlock b    : {rp_local[65], ci[...], vv[...]}  = 64 рядки; середній deg 10 -> ~650 комірок ~ 5 KB
```
Це Clojure/Okasaki-стиль персистентний вектор (M; PersistentVector 32-way trie) із блоком-листком = сегмент CSR.
`assign(A, i, j, v)` / вставка ребра: скопіювати RowBlock b(i) з одним зайвим елементом (5 KB append), скопіювати page
(4 KB) і root (256 B), новий заголовок матриці (64 B) → **~10 KB append ≈ 1-2 us при DRAM/append-швидкості**, старі
версії неторкані. Читання версії g: заголовок g → blocktab g → блок; `extract(i)` = 3 залежні `ldr` (root, page, block) +
рядок — проти 2 сьогодні (`rp`, `ci`): +1 miss на випадковий рядок (~+10-20 ns), 0 на послідовному скані (блоки
префетчаться, page-refs L1-hot).

**Батчі = дельта-матриці = log-structured tensor.** Транзакція накопичує дельту D_g як COO (append, O(1) на ребро,
контракт §9.3 «лог O(1»), відсортовану при commit (counting sort по рядках — `csr_build` над батчем ≤ 2^16 ребер: ≤ 1 ms);
commit = `A_{g+1} = A_g ⊕ D_g` через eWiseAdd ЛИШЕ над торкнутими блоками: для кожного блоку з ребрами в D — merge
рядків (2 вказівники) у новий блок; page/root CoW. Вартість батчу з B ребер, що торкають T блоків: T × ~5 KB + 4 KB × T/512
+ 256 B ≈ 5 KB·T; для 10^4 ребер, рівномірно по 1M рядках, T ≈ 10^4 → 50 MB append?? — ні: це гірше за stage-2. Тому
**трирівнева схема** (уже спроєктована §9.3, лише з блоками замість L0-перебудови): tail (COO, ≤ 4096), L0 (маленька
матриця з блоків, перебудовується цілком — O(|L0|) ≤ 2^18 ребер ≈ 3 проходи по 4 MB ≈ 1-2 ms після регістрової моделі),
L1 (велика, оновлюється блочним merge при злитті L0 → L1 раз на 2^18 ребер: торкнуто T ≤ min(2^18, n/64) блоків ≈ 16k
блоків × 5 KB = 80 MB append раз на 2^18 ребер ≈ 300 B/ребро amortised ≈ 25-50 ns/ребро на DRAM-швидкості; компакція
збирає старі блоки). **Read amplification**: point query = L1-блок + L0-рядок + tail-скан (≤ 32 KB) = 3 зрізи, як сьогодні
(1.1 us виміряно; після регістрової моделі ~0.4-0.6 us); скан/`mxv` читає L1 і L0 і застосовує tail як маску/дельту
(eWiseAdd на льоту). **Тобто функціональні оновлення = §9.3 tiered CSR + row-block CoW замість повної перебудови: stall
747 ms → ≤ 2 ms (L0) / фоновий merge (L1); amortised 30 us → ~0.1-0.3 us на ребро.**

### 2.3 MVCC, snapshot isolation, time-travel — безкоштовно

Версія матриці = `gen` у заголовку = генерація commit'у (§4a h1). Читач із root g бачить A_g; писач будує A_{g+1}, root
swap публікує. Time-travel «A as of g»: два суперблоки дають лише g і g-1; довша історія = `prev: ref GbMatrix` у
заголовку (mvcc.bp `prev`, §4d «history — user's choice per type»): ланцюжок версій живе до компакції, компакція зберігає
його, якщо тип оголошує `prev` (Datomic «as of»), інакше старі блоки — сміття (§4e Cheney не скопіює недосяжне = GC
недосяжних версій; tombstones §9.4 — структурне видалення всередині версії). Ізоляція: один писач (LMDB/SQLite-клас,
NOPOINTERS §2.1), читачі без локів; серіалізованість писача тривіальна; snapshot для читачів — за побудовою.

### 2.4 Порівняння з SuiteSparse pending tuples / zombies

SuiteSparse (M, TOMS 2019/2023 «non-blocking mode»): `setElement`/`assign` у порожню позицію → pending tuple (COO-список,
не в CSR), видалення → zombie (індекс помічений, лишається в структурі); «assembly» (sort + merge у CSR) відкладено до
першої операції, що потребує повної матриці; JIT-кернели на (op, type, semiring) кешуються в `~/.SuiteSparse` (README,
fetched — це specialise-then-run буквально: SuiteSparse ПРИЙШОВ до генерації кернелів у рантаймі). Наша схема — те саме
(tail = pending tuples, tombstone = zombies, merge = assembly), з двома відмінностями: (1) персистентно й версійно
(SuiteSparse збирає на місці й без історії); (2) блочний merge замість повної збірки (SuiteSparse збирає всю матрицю —
для нас це рівно stall 747 ms). FalkorDB/RedisGraph тримають «delta-plus/delta-minus» матриці над базовою (M; пошук не
підтвердив терміни) — та сама трирівнева ідея.

## 3. Чи це заміна SQL

**Що дає GraphBLAS-над-стором, чого SQL-рушії не мають:** (1) одна структура для таблиць і графів (таблиця = матриця
`k×rowid`, зв'язок = матриця на relation) — join і обхід є одна операція; (2) fusion за побудовою: кернел на (op, semiring,
mask, fmt) компілюється за 50 ms і memo'їться — SuiteSparse JIT + HyPer у одному; (3) точні кардинальності з `rp` замість
гістограм; (4) версійні, position-independent матриці (снапшоти, time-travel, checkpoint = memcpy); (5) нуль залежностей,
268 KB компілятор, self-host.

**Чого не дає:** (1) ad-hoc поверхні (SQL-текст, драйвери, BI) — DSL над асоціативними масивами, не SQL; (2) евристик
оптимізатора для n-way join'ів (n ≥ 5: k! порядків, кореляції) — SpGEMM має ту саму задачу порядку добутків
(chain-matrix ordering) і ту саму експоненційність; (3) захисту від вибуху кардинальності; (4) багатьох писачів; (5)
латентності < 50 ms на першій появі нової форми запиту (sqlite prepare 20-50 us).

**Пам'ять і SpGEMM проти hash join.** Обидва memory-bound: probe = випадковий рядок/бакет (1-2 miss), вихід — послідовний
запис. Gustavson SPA (dense акумулятор n комірок = 8 MB на 1M — L2-переповнення) проти хеш-таблиці з відкритою адресацією
(16 B/slot, load 0.5 = 32 MB) — CSR-bucket компактніший (8 B/nnz + rp), без resize, без колізій; сортування ключа
(counting sort) — 3 стріми замість random insert'ів. Очікування: **0.7-1.5x проти Rust hash join, 20-50x проти sqlite**
(§1.2). Це та сама «~1x проти найкращого Rust, порядок проти інтерпретатора» лінія всіх звітів сесії.

**Література як доказ існування:** RedisGraph/FalkorDB — комерційна графова БД, де Cypher виконується як GraphBLAS-вирази
над SuiteSparse (arXiv:1905.01294, fetched abstract); MAGiQ — SPARQL над RDF як матрична алгебра (VLDB 2018 demo /
EuroSys 2019, GraphBLAS-Pointers); Graphulo — лінійна алгебра над Accumulo (D4M, «100,000,000 database inserts per
second», arXiv:1406.4923); pggraphblas — GraphBLAS у Postgres; Deep et al. SIGMOD 2020 «Fast Join-Project Query Evaluation
using Matrix Multiplication» і SPORES VLDB 2020 (sum-product ↔ реляційні тотожності, equality saturation) — реляційна
алгебра як лінійна формально й із вимірами (M для змісту). Що література НЕ показує: GraphBLAS-БД, що виграє в OLTP.

**Прогноз на цьому боксі** (M = виміряно, D = похідне, G = здогад; «після §4» = регістрова модель + gb.bp + u32 + NEON
builtin-и; проти sqlite native і проти Rust CSR/std):

| робоче навантаження | сьогодні | після §4 | vs sqlite | vs Rust CSR / std | доказ |
|---|---|---|---|---|---|
| BFS 1M/10M (frontier `or-and`) | 45 ns/slot M | 25-35 ns (register model −push/pop; u32 −трафік) | 57x M → **150-300x** D (CTE 10.8 us/edge) | ~1x D (Beamer-клас 10-50 ns/edge) | T117, LANG-DB §9.2 |
| PageRank 10 ітер (plus-times, Q32) | ~30 ops/nnz (schoolbook fp_mul) ≈ 50-100 ns/nnz G | `umulh` builtin + madd: 16 B/nnz → 1.5-3 ns/nnz D | нема SQL-аналога без CTE-циклу: 100x-клас G | ~1-1.5x D | spectral.bp:29 |
| triangle counting (`C<A>=A·A` plus-pair) | немає | Gustavson з маскою: memory-bound, ~10-20 ns/nnz·deg G | 3-way self-join sqlite: 100x-клас G | 1-2x D | LAGraph TC (M) |
| 2-way join 1M×1M (any-pair / plus-second) | немає (nnidx = 1-D bucket join) | csr_build 5-10 ms + probe 10-20 ns/row ≈ 20-40 ms D | indexed nested loop 1-2 us/probe: 20-50x D | 0.7-1.5x D | §1.2 |
| Q6 filter+sum (3 предикати) | скан 18 ns/row M (K6-клас) | 2-3 ns/row D (DRAM-стеля 24 B/row) | 158 ns/row: 50-80x D | ~1x D | K6, §1.2 |
| Q1 group-by 6 груп × 8 агрегатів | немає | 5-10 ns/row D | ~300 ns/row G: 30-50x | ~1x D | §1.2 |
| single-row update (row-block CoW) | 30 us amortised, stall 747 ms M | 0.1-0.3 us amortised, stall ≤ 2 ms D | sqlite UPDATE WAL ~5-10 us native G: 20-50x | Rust in-place Vec: 0.1x (без версій) — не той клас | §2.2 |
| point «neighbours of v» | 1.1 us (3 зрізи) M | 0.4-0.6 us D | 9x native M | ~1x | T116/T117 |

Порядок величини проти SQL-реалізацій — на кожному рядку, крім point/update; проти Rust — паритет усюди (той самий
кремній, ті самі промахи). Це і є «тензорно-графова БД»: не швидша за Rust-програму, написану під конкретний запит, а
така, що ГЕНЕРУЄ цю Rust-якості програму за 50 ms під будь-яку форму над відомою схемою.

## 4. План інтеграції

| # | компонент | що | рядків | гейт | залежності |
|---|---|---|---|---|---|
| 1 | `gb.bp` (prelude) | GbMatrix/GbVector як store-об'єкти (§2.2 layout), формати CSR/bitmap/iso, маски, extract, transpose (= csr_build), degree/reduce по рядках | ~300 | G9a: round-trip матриці через дві бази (G2-стиль), fold == python oracle | store.bp, u32 пізніше (4b-4) |
| 2 | `gen_gb.bp` генератор кернелів | шаблони mxv (push/pull, Beamer-перемикач), mxm (Gustavson+SPA), eWiseAdd/Mult, select, apply, reduce; параметри (op, semiring, mask, fmt, transpose) → `.bp` файл → `bebop.bin compile` → digest-memo (T108) | ~400 + шаблони ~500 | G9b: LAGraph-стиль folds BFS/PR/TC/CC/SSSP == python oracle (stdlib); кожен кернел — конструкт | item 6 specialise-then-run (це його узагальнення), `umulh` builtin (~8 слів, plus-times над Q32), NEON `cmp_mask`/`sum64` (item 13B) |
| 3 | функціональні оновлення | tail COO + L0 + L1 з row-block CoW (2-рівнева blocktab), eWiseAdd-merge при commit, компакція старих блоків, `prev` для time-travel | ~300 | G9c: 1M single-row updates: amortised ≤ 0.5 us, max stall ≤ 10 ms, folds == oracle after every 10^4; kill -9 під час merge (G5-харнес) | §9.3/§9.4, sgraph2 як oracle-код |
| 4 | DSL над асоціативними масивами | `q { from T where p group by k agg s join U on k }` → AST → планувальник (шлях доступу + порядок join ≤ 4) → gen_gb | ~450 (= NOPOINTERS §2.2 планувальник + генератор) | G9d: Q6/Q1/join folds == sqlite (той самий SF 0.1) | 2 |
| 5 | multi-core | row-range partition `mxv`/`mxm` по 3 A78 через clone/setaffinity (nn4-патерн), reduce злиття | ~100 | рядок ×1.4-2.2 (DRAM-межа) | pool.bp |
| 6 | твіни | 2-way join (§4 нижче), Q6/Q1 vs sqlite, BFS/PR/TC vs Rust CSR (petgraph-free, ручний CSR) і vs sqlite CTE | ~300 .bp + ~200 py/rs | рядки в REPORT-honest / RESULT-sgraph | rust_once/ |

Разом **~2 000-2 500 рядків bebop за 6-8 chain-комітів, нуль залежностей** (усе в поверхні bebop; python/Rust лише як
оракули й близнюки, як досі). Субстрат виконання: регістрова модель (item 2: −push/pop у кожному кернелі), T52 csel
(маски без гілок), LIN — не застосовний (не рекурентності), індексна модель 4b (u32 `ci` = 2x трафіку BFS/scan;
арена-відносні refs = блоки без вказівників — уже так у сторі), NEON builtin-и (13B: `cmp_mask` для select, `sum64` для
reduce), плоский IR (item 12) — не потрібен (кернели генеруються плоскими).

**Місце в ROADMAP:** після item 6 (specialise-then-run twin — gb.bp це його загальна форма: один генератор замість
одного кернела) і разом з 14a-3 (компільований планувальник: DSL §4-4 і є той планувальник); item 3 (G9c функціональні
оновлення) — це STORE PULL «stage 3» sgraph (замінює O(N) L0-перебудову, закриває stall 747 ms); u32 (4b-4) — перед
твінами BFS/scan, бо він дає останній 2x.

**Перший вирішальний twin (measured-first, до будь-якого коду gb.bp):** **2-way join як SpGEMM** — `R(k,a)`, `S(k,b)`,
1M×1M, два розподіли ключів (uniform, Zipf 1.1 з 1 % важких ключів), вихід = count + checksum пар: (i) bebop: csr_build(S)
+ probe-цикл (ручний, ~60 рядків, БЕЗ генератора — щоб виміряти механізм); (ii) sqlite 3.46 native: `SELECT count(*),
sum(a*b) FROM R JOIN S USING(k)` з індексом і без (hash/nested loop за планом sqlite); (iii) Rust `std::collections::HashMap`
join і Rust sort-merge join (rust_once/). Гейт тези: bebop ≥ 10x sqlite native І ≥ 0.7x кращого Rust на обох розподілах.
Якщо друга умова не виконується більш ніж у 2x — теза «тензорно-графова БД» звужується до «графова + сканова БД» (усе одно
цінна: BFS/scan/агрегати), а join'и лишаються за csr-bucket'ами, як сьогодні. Другий twin (одразу після): G9c
single-row updates проти sqlite WAL UPDATE (§3 таблиця) — він вирішує, чи «purely functional» не коштує порядку на записі.

## 5. Докази

- Repo: docs/LANG-DB-DESIGN.md §4a-4h (:142-306), §9.2 (:533-573: GraphBLAS identity, push/pull, per-nnz physics), §9.3
  (:575-606: CSR mutation, tiered CSR pick), §9.4 (:608-636: block-CoW, tombstones), §9.5 (:638-670: verdicts, G8 spec);
  selfhost/std/csr.bp:1-60 (rp/ci/vv contract, counting sort, selection sort to replace), sgraph.bp:14-73 (build both
  directions, bfs_from), spectral.bp:29 (spmv_fp plus-times over Q32), ordfsm.bp:1-25 (W = 12-state FSM, A^12 == 0, reach
  masks), selfhost/prelude/store.bp:100-198 (st_begin/alloc/commit/snapshot/get/put/ref/link), mvcc.bp `prev`;
  HISTORY.md:2841 (T117 stage 1/2: queue 187-240 ns, frontier 45 ns/slot alpha 14, sqlite CTE 10.8 us/edge 57x, log 30
  us/edge, stall 747 ms, tombstone 131 ms, compaction 795 ms, neighbours ~1.1 us), :2767-2840 (STORE PULL G1-G8 rows);
  bench/vs_rust/RESULT-sgraph.md, RESULT-sbench.md, bench/tq_sqlite/RESULT.md (9.9x scan, 13.8x window, nn4 2.21x);
  docs/RESEARCH-NOPOINTERS-SQL-2026-09-06.md §2.1-2.2 (single-writer model, planner = compiler, ~1 100 lines), §1.2 (u32);
  docs/RESEARCH-TENSOR-2026-09-06.md §2 (NEON facts: no .2d mul, no gather, K6 DRAM ceiling 2 ns/row), §3.
- Fetched: raw.githubusercontent.com/DrTimothyAldenDavis/GraphBLAS/stable/README.md («compiles JIT kernels and places
  them in ~/.SuiteSparse», PreJIT); raw.githubusercontent.com/GraphBLAS/GraphBLAS-Pointers/master/README.md (LAGraph:
  BFS/PageRank/TC/CC/SSSP/BC/LP/SCC/k-core; MAGiQ VLDB 2018/EuroSys 2019; Graphulo; pggraphblas; C API v2.0.0
  graphblas.org/docs/GraphBLAS_API_C_v2.0.0.pdf); arxiv.org/abs/1905.01294 (RedisGraph abstract); search: arXiv:1501.05709,
  1606.05797, 1712.00802 «Polystore Mathematics of Relational Algebra», 1406.4923 (D4M/Accumulo inserts), Deep et al.
  SIGMOD 2020, SPORES VLDB 2020 (titles from search results).
- From memory (M): SuiteSparse formats (sparse/hypersparse/bitmap/full, iso) and pending tuples/zombies/assembly (TOMS
  2019 «Algorithm 1000», 2023 «Algorithm 1037»; dl.acm.org returned 403); FalkorDB delta matrices; Okasaki 1998; Clojure
  PersistentVector 32-way trie; Beamer et al. SC 2012 direction-optimising BFS (alpha 14 / beta 24, as LANG-DB §9.2 cites);
  Gustavson 1978 row-wise SpGEMM; HyPer/Umbra.

VERDICT: GraphBLAS над стором = назва для того, що стор уже робить (CSR, counting sort, frontier SpMSpV 45 ns/slot, spmv_fp,
tombstone-маски) плюс ~8 операцій як згенеровані кернели з напівкільцем-параметром (gb.bp ~300 + генератор ~900 рядків,
специализація за 50 ms = SuiteSparse JIT + HyPer); чисто функціональні оновлення = перенести персистентність з об'єктів на
матриці: tail COO + L0 + L1 з row-block CoW (2-рівнева blocktab, ~10 KB append на оновлення, злиття при commit) замість
повної L0-перебудови — stall 747 ms → ≤ 2 ms, 30 us → 0.1-0.3 us/ребро, snapshot/time-travel безкоштовно (~300 рядків);
реляційна алгебра як лінійна (Kepner): select = маска, join = SpGEMM над CSR-bucket = hash join без хеш-таблиці, group-by =
reduce по моді; прогноз після §4: BFS 150-300x, Q6 50-80x, Q1 30-50x, join 20-50x, update 20-50x проти sqlite, ~1x (0.7-1.5x)
проти Rust усюди; це замінює SQL для аналітики/графів/повторюваних форм над відомою схемою, НЕ для OLTP з багатьма
писачами й ad-hoc SQL-поверхні; ранг > 2 для W не потрібен (мода = ще один CSR); ~2 000-2 500 рядків, 6-8 комітів, нуль
залежностей, після ROADMAP item 6, разом із 14a-3; перший вирішальний twin — 2-way join як SpGEMM vs sqlite vs Rust hash/sort-merge
(гейт ≥ 10x sqlite І ≥ 0.7x Rust), другий — single-row updates vs sqlite WAL.
