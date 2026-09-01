# Bebop — THE Roadmap (single source of truth)

This file supersedes and replaces PLAN_B.md, MASTER-FINISH-PLAN.md,
ROADMAP_SELFHOST.md, docs/ZERO_C_CHARTER.md and SWEEP-B3-3.md — all removed.
BUGFIXES.md stays (bug journal), AGENTS.md stays (process laws), bench/
reports stay (evidence).

## Terminal goal

**Bebop is a post-von-Neumann self-hosting agent language — a single living
mathematical structure that maps directly to silicon. It erases the boundary
between memory, compiler, text and processor architecture. There are no
traditional instruction lines, no syntactic sugar, no virtual machines,
no garbage collectors, no intermediate interpreters.**

### What the language IS (the target state)

- **Post-von-Neumann substrate**: no program counter, no call stack, no
  sequential fetch-execute loop. The processor is an asynchronous event
  dispatcher scanning dense bit arrays of activity via hardware `tzcnt`/`popcnt`
  + SVE2, with threshold accumulation (Σ w_i x_i > θ). Code "lives" only
  where a spike fires.
- **Holographic memory topology + ranked arenas**: a single immutable linear
  arena, zero-copy mmap, 64B cache-line aligned. Information is not broken
  into isolated cells — it is packed into rank-4 word-tensors (`.bt`) and
  CSR adjacency matrices. Through spectral smearing via FWHT, the program
  structure is spectrally distributed across the entire tensor topology.
  Modification or deletion does not break the system — it smoothly
  redistributes spectral fingerprints across the whole space, eliminating
  the concepts of "dangling pointers" and segmentation faults entirely.
- **Spectral engine + eigentime**: no classical execution loop. Time is
  measured not by clock ticks but by eigentime — discrete iterations of
  Hotelling deflation and dominant eigenvalue (λ) stabilization. The compiler
  maintains continuous spectral invariant checks on the CSR graph; if the
  global spectral gap (γ) is violated, the system instantly prunes invalid
  branches before execution.
- **Multi-tier spectral stack**:
  * Micro (FWHT / Hadamard / Walsh / Haar): instantaneous bit-level binding
    and event routing via pure integer add/sub — no multiplication, no float.
  * Meso (NTT): exact polynomial and cyclic convolutions over Z_p with
    absolute bit-precision and zero approximation drift.
  * Macro (KLT): background spectral deflation, eigenvector computation,
    eigentime arena stabilization.
- **Hardware fusion on ARM SME/SVE2**: tensor multiplication, spectral
  deflations, and quantized attention transforms map directly to ARM SME
  matrix tiles (ZA); Hamming distances and parallel masking use SVE2
  variable-length vectors — maximum bus throughput with no external libraries.
- **Reversible logic**: all arena mutations via reversible gates (Toffoli/
  Fredkin) implemented as pure bitwise XOR/mask operations. Every arena
  state is fully reversible at the bit level — instant rollback and
  self-healing without snapshots or extra memory.
- **Multiversal superposition**: all potential agent logic states are held
  as a weighted superposition of hypervectors (hv4096). Deterministic
  collapse to reality occurs at the intersection of eigenvectors.

### Current repo state (verified 2026-09-01)

`seed/seed.S` (frozen AArch64 loader, no libc, 1496B) + `bebop.bin`
(self-hosting compiler, fixpoint bb2 == bb3) + `*.bp` sources + `*.bin`
artifacts. **Zero C.** All 4 active gates green (15/15 std_golden,
parity 9/0/0, construct 20/20, pool 5/5).

## Current verified state (2026-09-01)

**Historical note (Step 1):** the "call-in-loop execution divergence" (c19/c20)
was a RUNNER defect, not an emitter defect — exec_words took the manifest's
LAST fn offset as the entry. Fixed via explicit entry word offset; parity
driver + construct parity now check VALUE parity too (interp run == native
exec), not just word bytes. Two emitter gaps remain (blessed by checksum-only
self_check):
- struct-literal VALUES (`pt { x: 1, y: 2 }`): enum-ctor fallback parses field
  names as variables — field access works, construction does not.
- string-literal VALUES: emit_str placeholder (mov #0).

- **M1** seed loader: DONE — k1..k7 run through seed.bin, outputs identical
  to the interpreter; zero C at runtime.
- **M3** self-bootstrap: DONE — full selfsource compiled by itself is
  byte-identical to the interpreter's output (67816/67816 words); selfcompile
  fingerprint 236065248692568 == word-sum; self_check = 0.
- **M4** CLI-in-.bp: DONE — `bebop.bp` = compiler + CLI
  (`compile/size/version`). seed loader v4 passes argc/argv (x0=argc,
  x1=arena-copied argv). Verified: `version`=1000000, `size k1c.bin`=94,
  `compile k1/k7` byte-identical vs compilewords, CLI-compiled k7 executes →
  3939697352, unknown cmd → 64.
- **M5** std twins: CORE DONE — `bench/vs_rust/std_golden.sh` gate: **15/15
  PASS** encompassing the spectral stack (wht, haar, ntt, spectral, hv, csr,
  bt) + numeric/std gates. Emitter fix shipped: `is_alpha` uppercase fix —
  self_check=0, self_bootstrap 236271528687723.
- **M7** Zero-C: DONE — `native/src` (175 files) deleted; repo = seed +
  bebop.bp + bebop.bin + bench/ + selfhost/ + docs; full gate suite green
  without C compiler: std_golden 15/15, parity_driver kernels 9/0/0 (+1
  skip) + constructs 20/0/0, construct_parity 20/20 MATCH (words+values),
  pool_parity 5/5. bebop.bin self-replication: fixpoint stable (bb2==bb3,
  sha256 `3b720370a2…`). Interp dies at M7 per charter (only the seed JIT
  runtime remains).

## Known Issues (Documented)

1. **Pool test fntab divergence**: bebop.bin pool tests (par_sum/par_merge/
   par_compile) return 0 on the JIT due to interp/JIT fntab scan-budget
   divergence on sys_clone/futex paths. Interp is retired (M7); JIT itself is
   functionally correct (fixpoint stable, all gates green).
2. **CLI exec mprotect**: CLI compilation of selfsrc (116KB) segfaults due to
   exec builtin mprotect(EACCES) under proot W^X. Boot path (self_bootstrap)
   works. Low priority — boot path is the primary delivery channel.

## Design laws (inviolable, every swarm)

Branchless Σ k·(k==N)·expr; no_std; O(n); atomic/lock-free; vector-first
(NEON, scalar fallback); hypervectors where possible; living memory.
Per-fn bind budget ≤ 128 (overflow = trap, never silent); literals ≥ 2^63
must use the normalized-half emit path; nested ifs inside let-statements and
plain-var assignment inside `let _ =` are banned (index cells + single-level
guards only); capacity asserts on every fixed table.

## Coordination

Every milestone: commit + push to origin/main. Full verification gate after
each batch: self_check, self_bootstrap parity, parity driver, construct
corpus, fuzz, bench — evidence in commit messages.

## Fundamental layers (F1–F6, binding architecture)

F1. Syscall ABI emitter table — machine-verified (LAW L1). AUDIT 2026-08-31: 131 words
    across 17 emitters disassembled — ALL register numbers correct.
    **LAW: movz x8,#N = 3531603968 + N*32 + 8 (Rd=8!). The formula WITHOUT +8
    (SELFHOST_FIXES_SUMMARY) is WRONG — it produces movz x0,#N (wrong register).**
F2. Artifact I/O: mmap-export (ftruncate+mmap MAP_SHARED+stores+munmap), atomic
    publish via renameat(tmp,out); tmp = agent argv-argument; rename(x,x) = no-op.
    Zero sys_write on the critical path (proot syscall-site flakes immunized).
F3. Memory: arena + strided tensor-views (rank-2/3/4 over the same arena).
F4. Canonical artifact: .bt rank-4 word-tensor; text-codec = agent-authoring, canon = .bt-tensor.
F5. Control flow: branch-mode while/if with proper patching, depth_sim = 0 (outside run_program).
F6. Verification: self_fixpoint in RAM (bb2==bb3), artifact-vs-artifact (zero C-oracle).

# ═══ ОПЕРАТОРНЕ РІШЕННЯ 2026-09-01: NEO-ФУНДАМЕНТ (MANDATORY, найпріоритетніше) ═══
РІШЕННЯ ОПЕРАТОРА (перевизначає "Fixed execution order" і "Terminal goal" нижче):
оновлення АРХІТЕКТУРИ роадмапу є НАЙПРІОРИТЕТНІШИМ саме зараз. Ніяких скорочень
чи опціональності — все перелічене ОБОВ'ЯЗКОВЕ (mandatory) до впровадження.
Інваріант golden-vector determinism / JIT==interp fixed-point parity може бути
ВІДПУЩЕНИЙ (released) у напрямках, де він конфліктує з новими фундаментами.
Базовий шар (арени, .bt тензори, HDC, спектральна геометрія, SPIKE-DISPATCHER,
нуль-розгалужувальні ядра, плоскі 64B-масиви потенціалів, event-driven рантайм)
ЗАЛИШАЄТЬСЯ нижнім фундаментом — всі нові надбудови лягають ВЕРХОМ на нього.

## NEO-ФУНДАМЕНТ: 8 ОБОВ'ЯЗКОВИХ ОПОРНИХ КОЛОН (впроваджувати саме в цьому порядку)
Нижні колони — це основні низові рішення; верхні — надбудови. Жодна не є
опціональною. Порядок: спочатку та, що дає найширше перевикористання.

**N1. FWHT — Швидке перетворення Волша-Адамара (як числовий / VSA-зсувний базис)**
- Чистий ADD/SUB metелик (butterfly), НУЛЬ множень — ідеально під i64 liнiйну арену.
- Рідна мова для HV: рандомізоване ортонормування, проєкції, згортання станів
  без важких матричних множень; переклад гіпервекторів в інші базиси.
- Апаратна синергія: butterfly → NEON vadd/vsub пари, SVE2 VL-agnostic.
- Branchless / детермінований, фіксований крок по лінійній арені, нуль mispredicts.
- Реалізація: selfhost/std/wht.bp — fwht(x,n) in-place метелик (wht_pow2 /
  wht_invert / wht_encode). Гейт wht у std_golden.sh (15/15): dispatch e1/n8
  Walsh row (word 85) + самозворотний round trip (wht_pow2→wht_invert відновлює
  8 клітинок точно); JIT (seed) == інтерп == 85001. N1 CORE DONE 2026-09-01.
  Разом із N1b (Haar/DWT, haar.bp — див. MULTITIER SPECTRAL STACK + статус унизу)
  закриває МІКРО-рівень спектрального стеку.

**N2. Реверсивна / Консервативна логіка (Reversible / Conservative Logic)**
- Повна відмова від деструктивних операцій: жодне ребро арени не стирається
  беззворотно; всі інструкції/мутації — через зворотні вентилі (Toffoli/Fredkin),
  реалізовані чистими побітовими операціями (XOR/mask).
- Актуальна зворотність часу на рівні бітів у самій архітектурі компілятора:
  кожен стан арени можна «відкрутити назад» без копій/знімків — 0-оверхед дебаг,
  миттєвий відкат агента до будь-якого попереднього стану, самовідновлення.
- Реалізація: selfhost/std/rev.bp — оборотні примітиви (CNOT/Toffoli маски),
  journal/reversible-операційні конструкції поверх арени.

**N3. Ring-VSA / HDC з кольоровими кільцями Адамара (цілiсна алгебрична система)**
- Не розмежування код/дані/типи/інструкції: БУДЬ-ЯКИЙ синтаксичний елемент,
  CSR-граф, SNN-спайк = єдиний 4096-бітний гіпервектор.
- WHT = ЄДИНА алгебрична група для bind (зв'язування) і bundle (суперпозиції).
- Компілятор НЕ транслює код — виконує гомоморфне згортання сутностей у єдиний
  голографічний простір арени; пошук/виконання = Hamming-відстань.
- Реалізація: розширення hv.bp до кільцевої алгебри (bind=bundle через WHT-grupu).

**N4. Bit-Level Petri Nets (bітові асинхронні мережі Петрі)**
- Заміна навіть евристичних черг подій: паралельні матриці маркування
  (Token-Passing Petri Nets), маповані в бітові масиви арени.
- Активність = матриця інцидентності переходів; спрацювання транзитів — не гілки,
  а одна апаратна побітова операція (AND масок → tzcnt). Тисячі паралельних
  гілок логіки за кілька тактів без черг і викликів функцій.
- Реалізація: selfhost/std/petri.bp — бітові переходи, інцидентність, tzcnt диспетчер.

**N5. LSM / Reservoir Computing (Liquid State Machines) у лінійній арені**
- Часова динаміка і пам'ять агента — через постійний випадковий але фіксований
  резевуар пов'язаних вузлів у CSR-арені, зафіксований спектральними інваріантами.
- Вхідні спайки збуджують «рідкий» стан; FWHT миттєво проєктує багатовимірний слід
  у вихідний проспект рішень. Адаптація в реальному часі БЕЗ «навчання»/градієнта —
  структура мови сама є динамічним процесором часу.
- Реалізація: selfhost/std/lsm.bp — резевуар-rs, фіксація спектральними інваріантами.

**N6. Голографічна нелокальність пам'яті (Holographic Memory Manifolds)**
- Кожен фрагмент рангового словотензору кодується через WHT так, що містить
  мікро-відбиток ВСІЄЇ програми (голограма: розбирання не знищує картину).
- Арену можна обрізати будь-де — спектральний рушій (власні вектори) повністю
  відновлює і виконує всю логіку з будь-якого збереженого шматка.
- Зникає «битий вказівник» / «втрата контексту»: інформація розмазана глобально.
- Реалізація: WHT-кодування словотензорів, spectral-recovery поверх N1+N5.

**N7. Multiversal Superposition Branching (багатосвітове паралельне виконання)**
- Ліквідація послідовного вибору шляхів: ВСІ можливі гілки логіки обчислюються
  одночасно в одному бітовому масиві суперпозицій через SME/SVE2 векторні інструкції.
- Усі альтернативні майбутні стани = зважена сума гіпервекторів; колапс (вибір
  реальності) — автоматично, коли спектральний дефлятор (Hotelling) знаходить
  злам власного значення λ, який відсікає хибні гілки маскуванням пам'яті.
- Реалізація: superposition-масиви, spectral-collapse поверх N1+N6+SS-15/16.

**N8. Spacetime Metric Code / Global Boundary Execution (просторово-часове замикання)**
- Знищення «часу виконання» як послідовності кроків: програма = глобальна крайова
  задача на поверхні CSR-графа (Лаплас/теплорозподіл).
- Рантайм миттєво знаходить стаціонарний стан усієї системи як єдину математичну
  рівновагу; минуле/теперішнє/майбутнє узгоджуються глобально через спектральний
  інваріант за один апаратний прохід = «кристалізація матриці».
- Реалізація: Laplacian-boundary виконання поверх N6+N7+SS-18.

## Наслідки щодо інваріантів (ОПЕРАТОРНЕ рішення)
- **Golden-vector determinism / JIT==interp fixed-point parity: RELEASED** там, де
  конфліктує з N1-N8. Новий числовий базис (WHT) і надбудови мають власні golden /
  самоперевірки ПІД НОВИМИ домовленостями; старий fixed-point оракул більше не
  є обов'язковим джерелом правди для цих шарів.
- **Порядок виконання**: N1→N2→N3→N4→N5→N6→N7→N8 лягає ВЕРХОМ на вже закритий
  нижній шар (арени/.bt/HDC/spectral/spike-dispatcher). Нижній шар не видаляється —
  він стає підкладкою. Жодна колона не є опціональною.

# ═══ АРХІТЕКТУРНИЙ СИНТЕЗ (2026-09-01): пост-фон-нейманівська VM + ARM SME/SVE2 ═══
Концепція "мова за межами фон-неймана" (ліквідація PC → Spike Dispatcher, заміна
branching → порогові суми, зникнення змінних → плоскі 64B-масиви потенціалів,
самокомпіляція → спектральне налаштування CSR-графа) — це НЕ окрема ціль, а
формулювання того, що вже закладено в нижніх шарах: плоскі арени (F3), нуль
розгалужень у гарячих ядрах (LAW branchless), i64 fixed-point 2^32 як єдина
числова правда, .bt ранг-4 словотензори (F4), .bt як канон артефакту. Вона
підтверджує напрям, а не змінює порядок виконання. (NEO-фундамент N1-N8 розширює
цей синтез: WHT = числовий/VSA базис замість або поряд із fixed-point; реверсивні
вентилі знищують деструктивні операції; кільця Адамара роблять алгебру єдиною;
Petri/LSM/голограма/superposition/spacetime — надбудови над ним.)

Точковий мапінг апаратних розширень (фундамент ПЕРШЕ, надбудова друга):

1. **ARM SME (Tiles ZA)** — першокласне апаратне залізо для NEO-фундаменту.
   Внутрішній добуток v1·v1^T + SpMV у Hotelling deflation / power iteration
   (Ф2/SS-6/SS-15/SS-16) лягає на матричні плити SME; N7 (multiversal
   superposition) використовує SME/SVE2 для одночасного обчислення всіх гілок.
   За ОПЕРАТОРНИМ рішенням 2026-09-01 інваріант golden fixed-point parity для
   цих шарів RELEASED — SME має власні golden/самоперевірки під новим базисом,
   а не валідується обов'язково проти старого i64 fixed-point оракула.
2. **ARM SVE2 (VL-agnostic)** — для hv4096 HDC (Ф1) та бітових спайків. Поточний
   Ф1 вже канонічний на NEON (SWAR popcount/hamming, golden-точен). SVE2 = forward
   порт тієї ж детермінованої цілої арифметики (popcnt/tzcnt не змінюють чисел) —
   можна коли фікс-ширина NEON стане вузьким місцем. Безпечно ТОЧНО: побітові
   операції LVA-інваріантні, паритет зберігається.
3. **Bit-Manipulation (biti/dispatch)** — для event-driven Spike Dispatcher
   (tzcnt/lzcnt/popcnt + Base+SpikeIndex*Stride). Це верхня надбудова (агентний
   рантайм-диспетчер), ПІСЛЯ того як ядро (компілятор/спектральний шар) стабільне.
   Порядок: спочатку нижчі шари, диспетчер — частина Part B/B2 backend tier.

Підсумок для роадмапу: SME/SVE2/bit-manip — частина NEON/backend tier (B2) та
опційних прискорювачів. Вони НЕ випереджають і не замінюють: (1) фикс-поінт як
єдину правду, (2) golden-детермінізм, (3) JIT==інтерп паритет. Ніякий апаратний
FP-прискорювач не стає оракулом. Це зберігає "фундаментальні низові рішення
пріоритетнішими за верхні надбудови" — SME/SNN/SVE2-надбудови лягають ВЕРХОМ на
вже закриті нижні шари, ніколи не змінюючи їхні інваріанти.

# ═══════════════════════════════════════════════════════════════════════
# SPECTRAL SINGULARITY LAYER — Kalman/Vector Calculus/LC/FIR/QLoRA/Transformer
# (максимальна проєкція: арени + .bt тензори + HDC + спектральна геометрія)
# ═══════════════════════════════════════════════════════════════════════

## SS-1. NEON Kalman Filter (нуль-алокаційний, арени, real-time)
- Фільтр Калмана як чиста .bp бібліотека на лінійних аренах: нуль malloc, нуль heap
- Матричні операції (predict/update) через NEON 2×2 systolic tiles (tile2x2 патерн)
- Детермінована затримка: фіксована кількість тактів на ітерацію (WCET-гарантія)
- Застосування: сенсорні стани дронів/боївок без ROS/C++/CUDA
- Emit: `kalman_predict(state F Q)`, `kalman_update(state H R z)` — .bp fns
- Верифікація: golden KAT (еталонні stани) vs C-еталон ДО Zero-C
- Done-check: 1000 ітерацій → PVC-стани = C-еталон, 0 дрифтів

## SS-2. Vector Calculus як статичні інваріанти (rot/div/grad → граф-топологія)
- Тотожності (∇·∇f = ∇²f, ∇×∇f = 0) → перевірки збереження структури CSR-графа
- Диференціальні оператори = бітові маски на ранг-4 тензори (не символьна математика!)
- Компілятор статично верифікує: після тензорного перетворення структури — інваріант збережено
- Вбудовується в спектральну верифікацію як додаткові аксіоми структурної цілісності
- Done-check: графова дивергенція = 0 для коректних AST-перетворень

## SS-3. LC Resonance → тактування агентних циклів (jitter-free)
- Цикл обробки = електронний LC-контур: L = латентність, C = ємність арени
- Резонансна частота f₀ = 1/(2π√LC) → цільовий інтервал між ітераціями фікспоінту
- Агент планує ітерації на резонансній частоті → мінімальний джиттер без ОС-планувальника
- Реалізація: clock_ms() + NEON-компенсація дрифту (PID-регулятор на .bp)
- Done-check: джиттер < 1% на 1000 циклів (без ОС-планувальника)

## SS-4. FIR-фільтр як заборона циклічних залежностей (BIBO-стабільність)
- FIR: тільки forward flow — жодного зворотного зв'язку → BIBO-гарантія структурно
- Компілятор ЗАБОРОНЯє while-цикли з невідомою глибиною в агентному коді (еміторіальний рівень)
- while → bounded masked iteration (Ф4) — домен обмежений, ризик нескінченності = 0
- Додатково: IRR-фільтри дозволені ТІЛЬКИ з формальним доказом збіжності (спектральна верифікація)
- LAW: агентний код без FIR-обмеження = відхиляється на емісії

## SS-5. Calculus bounding (Teylor/mean-value для мутаційного коду)
- Тheorema про середнє значення + ряд Тейлора → автоматичні bounding boxes для мутацій
- Компілятор доводить: Δ(вихід) ∈ [f(a)-ε, f(b)+ε] для any мутації CSR-графа
- Інтеграція: спектральна верифікація (boundingBox(prop) — пропозиція для кожної мутації)
- Done-check: golden мутації — bounding box містить фактичний результат

## SS-6. Matrix Decompositions на аренах (LU/QR/SVD/Power Method)
- Port: dowiz-core spectral.rs → .bp (Faddeev-LeVerrier, Durand-Kerner, Householder eigh)
- topk_symmetric (power method + Hotelling deflation) — i64 fixed-point (2^32 scale)
- DecompCache: content-addressed (FNV-64) кеш спектрів — recomputes falsifier
- par порт: NEON matvec (Ф5 einsum) для matmul-примітива в power iteration
- Done-check: golden-спектри з householder_spectral_parity.rs — парність з .bp портом

## SS-7. QLoRA 4-bit агентна еволюція
- Ваги агентних стратегій = 4-бітні матриці у фіксованих аренах
- Low-rank адаптери (A·B де rank << dim) → оновлення логіки на живому залізі
- DecompCache зберігає квантовані стани — FNV-64 ключ, лoàd через mmap
- NEON: 4-бітне розпакування (shift/mask) + матвек за 1 цикл на 16 елементів
- Done-check: агент-стратегія оновлюється за < 1ms, 0 malloc

## SS-8. Sinc інтерполяція (без фазових спотворень)
- sinc(x) = sin(πx)/(πx) як ідеальний інтерполянт для тензорної телеметрії
- NEON: векторизований обчислення sinc через наближення (ехактно до 4 знаків)
- Фільтрування сенсорних потоків без фазового спотворення = критично для Kalman (SS-1)
- Done-check: sinc-інтерполяція vs точне значення — похибка < 0.1%

## SS-9. Transformer Attention на ARM64 NEON (нуль фреймворків)
- Q,K,V = ранг-4 .bt тензори в лінійних аренах (64B-алігновані)
- Self-Attention: hv4096 Hamming distance через vcnt (замість softmax+float)
- bind = XOR, bundle = мажоритарний — лічені такти, без CUDA/PyTorch
- Позиційні енкодинги: топ-k власні значення + Фідлерерів вектор (спектральні, layout-інваріантні)
- Маскування: геометричні стріди ранг-4 (верхній трикутник = stride-skip, нуль розгалужень)
- KV-cache: DecompCache (FNV-64 ключ, квантовані low-rank стани, нуль malloc)
- Done-check: attention(Q,K,V) на .bt = узгоджений з C-еталоном, < 1ms на 128 токенів

## SS-10. Нормалізації та стрід-оптимізація (cache-line aligned)
- Layer Norm / Instance Norm = стрід-геометрія ранг-4 (.bt) під кеш-лінії 64B
- Розташування арен: гарячі тензори (attention scores) — L1-resident; холодні (KV-cache) — L2/L3
- Zero false sharing: кожен NEON-канал = окрема кеш-лінія, work-stealing без блокування кешу
- Done-check: bench attention-проходу — L1 hit rate > 95%

## SS-11. Поколінна арена з MAP_NORESERVE пагінацією
- mmap(NULL, size, PROT_READ|WRITE, MAP_PRIVATE|ANONYMOUS|NORESERVE) — віртуальний простір
- Виділення = зміщення вказівника (bump) — детермінована затримка, нуль GC
- Скидання = mprotect(PROT_NONE) — миттєвий звільнення сторінок, ядро деалокує
- Нові покоління: старий стан → mprotect(READ_ONLY) → нова арена = bump від base
- Done-check: 1M alloc/free циклів — 0 syscall (окрім mmap), 0 фрагментації

## SS-12. NEON бітові матриці (switch/case → паралельні бітові сітки)
- Патерн-матчинг: всі умови пакуються в щільні бітові сітки (128-bit NEON регістри)
- Оцінка десятків станів за 1 такт (CCMP/CCMP-ланцюги або bif/bit)
- Заміна switch/case в emit_call_or_ctor диспетчері → бітові маски
- Done-check: диспетчер 23 builtins через бітову матрицю — < 10 тактів

## SS-13. Позиційно-незалежні DecompCache блоки
- Кешовані AST-графи + скомпільований код = позиційно-незалежні бінарні блоки
- Збереження/завантаження = нуль-копіювальний mmap (диск або /dev/shm)
- Скомпільований код = raw bytes в арені без релокацій — миттєвий cold-start
- Відмінність від Ф6: кеш НЕ-content-addressed (позиційний), а RELOCATABLE (PIE-стиль)
- Done-check: cold-start компілятора < 5ms (mmap + jump)

## SS-14. Direct-threaded code в аренах (без dispatch loop)
- Інструкції = прямі посилання на наступний обробник (no dispatch loop)
- Максимізація I-cache L1: інструкції в лінійній арені, наступний обробник = сусідня адреса
- Застосування: інтерпретатор .bt-тензорних операцій (якщо потрібен), агентні state machines
- Done-check: threaded код vs switch-dispatch — ≥ 2× на I-cache-intensive навантаженні

## ═══ SPECTRAL COORDINATE SYSTEM (інтеграція eigen в єдину систему) ═══

## SS-15. Власні вектори = єдина система координат
- Всі стани/поняття проєктуються на ортонормальний базис Q (власні вектори оператора зв'язків)
- Інваріантність до пам'яті: зсуви байтів компенсуються спектральним базисом
- Пошук = проєктування гіпервектора на домінуючі власні вектори (не вказівник!)
- Координати = спектральні проєкції, layout-інваріантні (немає вказівників)

## SS-16. Власні значення = метрики контролю потоку
- γ = 1 - |λ₂| (спектральний гап) → перемикач логіки: γ < поріг → граф розпадається
- Фідлерерів вектор → автоматичне розпаралелювання (знак = біпартиція графа)
- На рівні сирого заліза: NEON power method → λ₁,λ₂ → умова на γ → NEON біпартиція
- Интеграція: Ф4 маскований потік (γ-домени) + Ф6 work-stealing (Фідлерерів біпартиція)

## SS-17. Eigentime (час = спектральна ітерація)
- Синхронізація = кількість ітерацій Hotelling deflation (не годинник!)
- Фікспоінт λ₁ стабілізується → ядра в енергоефективний стан (WFI/WFE)
- Агентний сигнал → ΔA → нова ітерація → новий λ → continued execution
- Забирає потребу в ОС-планувальнику, таймерах, перериваннях

## SS-18. Спектральна самореплікація (мутація через ΔA)
- Агент змінює логіку = матричне збурення CSR-графа ΔA
- Перевірка: spectral_drift(A₀, A₀+ΔA) → DriftClass (spectral.rs:800 порт)
- Дрифт в межах γ → автоматична фіксація (mmap snapshot); поза → .bt дамп
- Замінює текстову компіляцію: еволюція = чисті математичні стрибки спектру

## ═══ Ф0.3 BOOTSTRAP: ЗАКРИТО (2026-09-01) ═══
Блокер "bebop.bin крахує на pristine+3fn" ЗАКРИТО. Корінь: НЕ контекст-залежний
крах компілятора і НЕ proot-стелі I/O — emit_sys_read/emit_sys_write рухають
байти через жорстко зафіксований 8192-байтовий scratch (x28-8192), довжину
беручи з аргументу: будь-який len > 8192 = запис за межі арени (тиха корупція
в маплені сторінки читалась як "стеля ~291KB write / ~112KB read"; SIGBUS/SIGSEGV
на unmapped). Фікс: chunked write/read у cli (≤8192 на виклик, staging-буфер),
без змін емiтерів (потік слів стабільний). Плюс: перезбір bebop.bin (старий був
від Aug 29), розм-імена emit_sys_emit_sys_* виправлені, регресія C-парсера
(strequals/str_to_cells) обійдена на .bp-рівні. Повний журнал — BUGFIXES.md.
ГЕЙТИ: parity 9/0/0, construct 20/20, std_golden 7/7, self_check 0,
фікс-поінт ДВА ПОКОЛІННЯ байт-в-байт (bebop.bin == selfA == selfB),
pristine+1/+2/+3fn компілюються, +3fn-бінарник компілює k2/k7 правильно.
Наступні: GOLDEN-вектори спектрів з C/Rust (до Zero-C) → Ф1 HDC → Ф2 Tensor Arena.

# ═══ GOLDEN + Ф1 HDC: ЗАКРИТО (2026-09-01) ═══
GOLDEN: bench/vs_rust/spectral_golden/golden.txt — еталони з Rust (dowiz-core)
ДО Zero-C: topk_symmetric (6 графів, 32 iters, i64 fixed-point 2^32 + f64 bits),
Householder eigh референс (power-vs-Householder паритет), LeVerrier charpoly,
HDC-секція (code/bind/bundle/permute/hamming/popcount). Генератор — окремий
cargo-проєкт, regen-верифікований байт-в-байт.
Ф1 HDC-ядро: selfhost/std/hv.bp — канонічний twin hypervector.rs:
splitmix64 code(seed), bind (XOR), bundle (мажоритарний, ties→0),
permute (біт-ротація D=1024), SWAR popcount/hamming. Гейт hv у std_golden.sh
(8/8): ланцюг golden-векторів відтворений ТОЧНО (4427592702613580868)
через повний self-hosted pipeline (bebop.bin → seed), обидва рушії згодні.
LAW: hv_bundle out не може аліювати з рядком vs (w-loop пише out[w], поки
k-цикл читає vs[k*16+w]). Наступні: SPECTRAL topk_symmetric .bp порт → Ф2.

# ═══ SPECTRAL topk_symmetric .bp ПОРТ: ЗАКРИТО (2026-09-01) ═══
selfhost/std/spectral.bp: power+Hotelling над CSR spmv у i64 fixed-point 2^32 —
fp_mul (schoolbook 32/16-бітний спліт, точний 64-біт), isqrt (біт-за-бітом,
без ділення), LCG-старт з константами оракула, знак = перший |компонент| > 2^-16
додатній, сортування desc |λ|. Гейт spectral у std_golden.sh (9/9):
B6_bridge k=3 iters=32, frozen = Σ|λ_bp−λ_golden| = 184684 fp-одиниць
(~8e-6 відносно на кожне λ — чесний розрив fixed-point truncation vs f64).
JIT == інтерп точно (детермінована цілісна арифметика).
LAW: `>>` у Bebop ЛОГІЧНИЙ (u64) на обох рушіях — abs перед будь-яким зсувом
можливо-від'ємного значення (fp_mul ділить на модулі; normalize_fp квадратує
|x_i|>>14). Пропущений негативний зсув мовчки зіпсував першу спробу паритету.
Далі: Ф2 Tensor Arena + .bt ранг-4 → spectral coordinate system (SS-15).

# ═══ ХВОСТИ ЗАКРИТІ (2026-09-01) ═══
1. run_program ret −1 holdout (M3-era, "located-unfixed") — ЗАКРИТО за
   коренем: unresolved emit_call не проковтував список аргументів → pos
   застрягав усередині виклику → зайвий pop + фантомний push → ldp x29,x30
   читав сміття + SP-крип 16Б/виклик. Фікс: лексичний skip аргументів
   (глибина дужок + рядки + //), оба твіни. L0 depth-gate ЗЕЛЕНИЙ уперше
   (0 прапорців на обох потоках); depth_sim lsl#12 декодування виправлене.
2. C-парсер split-brain — ЗАКРИТО. bp_parse ніколи не парсив тіла (тільки
   ріжe на items); єдиний body-парсер (bp_parse_fn_decl → expr_parse)
   розширений до поверхні .bp-рівня: chained-discard-assign, Str~I64
   (pointer-duality), array_get на i64, syscall-буфери [i64].
   Гейт: check ok-count == ^fn-count (Makefile де-хардкоджений).
3. Interp-твіни sys_ftruncate/munmap/mmap/rename — ЗАКРИТО (L5): Term
   отримав e,f слоти (6 арг mmap), bp_syscall4, eval/check/subst/norm/conv/
   termination гілки. Паритет-проба: interp == JIT == 720000.
Стан: bebop.bp 112/112 check-ok, expr_compile.bp 110/110, self_check 0,
parity 9/0/0, construct 20/20, std_golden 9/9, фікс-поінт ×2 байт-в-байт.
Далі: Ф2 Tensor Arena + .bt ранг-4 → Ф3 VS-AST (item⊗role).

# ═══ ПРІОРИТЕТ РЕАЛІЗАЦІЇ (після Ф0.3 bootstrap) ═══
```
Ф0.3 (bootstrap) ─┬─► Ф1 (HDC) ──► Ф2 (.bt+CSR) ──► SS-6 (spectral.rs порт)
                   │                                     │
                   ├─► SS-1 (Kalman NEON) ──────────────┤
                   ├─► SS-4 (FIR bounded loops) ─────────┤
                   │                                     ▼
                   └─► SS-15 (spectral coords) ──► Ф4 (маски) ──► SS-9 (Attention NEON)
                                                         │
                    Ф6 (store/multicore) ────────────────┤
                                                         ▼
                    SS-2 (spectral inv.) ──► Ф8 (Zero-C)
                                                         │
                                                         ▼
                    SS-15..18 (spectral coord/flow/time/mutation) ──► SS-7 (QLoRA)
                                                         │
                                                         ▼
                                                    Ф9 (agent introspection)
```

## ПЕРШИЙ КОД: SS-6 (Matrix Decompositions порт) — бо це фундамент для SS-15/16/18
- Power Method + Hotelling deflation в i64 fixed-point (2^32 scale) — ФУНДАМЕНТ
- CSR spmv (матвек) через NEON (2×2 systolic) — ФУНДАМЕНТ для все графового аналізу
- DecompCache (FNV-64 ключ, recomputes falsifier) — ФУНДАМЕНТ для кешування
- Ці 3 примітиви = спільний залежність для SS-1, SS-9, SS-15, SS-16, SS-18
- БЕЗ НИХ: немає спектральної верифікації, немає layout-інваріантності, немає біпартиції

# ═══ Ф2 CORE PRIMITIVES: ЗАКРИТО (2026-09-01) ═══
CSR twin: selfhost/std/csr.bp — from_edges (per-row bucketing, selection
sort, adjacent-duplicate merge — wrapping sums order-independent = exact
Rust parity) + csr_spmv (канонічний порядок сумування). Гейт csr у
std_golden.sh (frozen -6945622865743784444 — структурний фолд 5 golden-графів).
.bt ранг-4 кодек: selfhost/std/bt.bp — Ф4 канон v1 ("BT4R", u32 version/rank,
dims[4], dense i64 LE data; 28B header): bt_pack / bt_fnv (FNV-1a 64
фінгерпринт) / bt_unpack (валідація magic+version+rank) / bt_offset
(ранг-4 stride view). Гейт bt (frozen -5708805812714944038 — roundtrip
фолд проти Rust golden 220-байтного потоку). std_golden 11/11, JIT ==
інтерп на обох.
LAW (csr.bp): cross-loop стан — тільки в клітинках ([0] буфери); два
рушії розходяться на rebind-видимості крізь гнізда циклів. І objekt-урок:
ім'я temps не має конфліктувати з клітинками (sort-temps `tc` затер лічильник
merge — мовчаний nnz=0).
Далі: .bt store (mmap-export + atomic renameat publish) → spectral coordinates (SS-15)

## N1b МІКРО-ТІР (Haar/DWT): ЗАКРИТО (2026-09-01)
selfhost/std/haar.bp — цілочисельний Haar/DWT (MULTITIER micro-tier): чистий
ADD/SUB (нуль множень, нуль float), in-place len{N/2..1} суми/різниці пари,
x[0]=DC=Σклітинок, деталі від грубих до тонких = Resolution Hierarchies для
SNN-спайків кільцевого буфера. haar_invert = точний обернений (кожне ділення
на 2 exact: a+b і a-b завжди парні) → round trip lossless для будь-якого i64.
Гейт haar у std_golden.sh (15/15): dispatch e1/n8 → word=41 (знак-біти 0,3,5)
+ self-inverse round trip [3,1,4,1,5,9,2,6] відновлює 8 клітинок точно.
РАЗОМ N1 (wht) + N1b (haar) закривають МІКРО-рівень мультитір-стеку.

## N1c МЕЗО-ТІР (NTT): ЗАКРИТО (2026-09-01)
selfhost/std/ntt.bp — number-theoretic transform над Z_p, p=998244353=2^23·119+1,
primitive root 3 (finite-field exact arithmetic, i64 чистий, нуль float). Ф2: n=8
ramp [1..8]: forward spectrum (центрований: [36, -103943349, 346334868, 201631260,
-4, -201631268, -346334876, 103943341]) → знак-біти word=141 (0,2,3,7).
Гейт ntt у std_golden.sh (15/15): ntt_inv(ntt(x)) == x exact round trip (8/8 клітин)
+ circular convolution ramp ⊛ reverse(ramp) NTT-multiply-then-invert ==
[176,156,144,140,144,156,176,204] (oracle: незалежний Python NTT, той самий MOD).
LAW (ntt.bp): invert-параметр у if..then(блок)..else НЕ виконував скалювання 1/n
(L1-пастка складних блоків в if-гілці) — інверсна стежка винесена в окрему
прямолінійну ntt_inv (як wht_invert/haar_invert); result був 8x input.
МАКРО (KLT) = SPECTRAL tier topk_symmetric/Hotelling (spectral.bp, SS-6).
Відхилені DFT/FFT/DCT/DST/Z-Transform/DHT — див. MULTITIER SPECTRAL STACK вище.
