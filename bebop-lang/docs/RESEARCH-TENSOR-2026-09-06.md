Status: 2026-09-06 (session 18) -- research report by a Fable fork, read-only over /root/dowiz/bebop-lang at HEAD a650f62
(register-model worker still in flight, so all "after register model" numbers are the forecasts of docs/RESEARCH-DEPS-2026-09-06.md,
not measurements). Box: 4x Cortex-A55 + 4x Cortex-A78; /proc/cpuinfo Features = `fp asimd evtstrm aes pmull sha1 sha2 crc32 atomics
fphp asimdhp cpuid asimdrdm lrcpc dcpop asimddp` -- NEON (asimd) yes, dot-product (asimddp) yes, **no `sve`**. A78 pipes (Arm Cortex-A78
Software Optimization Guide, documentation-service.arm.com/static/630382d1e95b0a633aff8a78, fetched via search): 4 integer ALU pipes
(2 single-cycle + 2 single/multi-cycle with multiply/divide/CRC), **2 FP/ASIMD pipes (V0, V1)**, 128-bit each. Latency model as in
RESEARCH-DEPS (add 1, add-shifted 2, madd 3, ldr 4, mispredict ~13, DRAM miss ~100 ns, DRAM stream ~12 GB/s saturated by one A78).

# «Тензорна графічна модель регістрів»: що це може означати, і що з цього дає bebop

## 0. Чотири читання терміна (і вердикт на кожне)

**(A) Графова алокація регістрів.** «Графічна» = граф інтерференції/def-use по всій функції: значення
(SSA-vals) = вузли, ребро = живуть одночасно; розфарбування у x0..x28 (з callee-saved як окремими
«кольорами», що переживають виклик) замість однопрохідного вікна + park-евристики blueprint'а.
Стандарт: Chaitin/Briggs (ітеративне розфарбування зі спілами), linear scan (QBE, Cranelift regalloc2 —
бандли live-range'ів з розщепленням), SSA-алокація Hack/Grund/Goos (граф інтерференції SSA-програми —
хордальний, оптимально фарбується за лінійний час за perfect elimination order, спіл відокремлений від
фарбування, копії лише на phi). **Вердикт: справжній механізм, потребує per-fn IR (RESEARCH-DEPS §3);
на кернелах-рекурентностях 0, на самокомпіляції +3-8 % понад плоский IR; входить у roadmap лише як
частина пункту 8 (плоский IR), не окремо.**

**(B) Тензор = векторний регістровий файл як другий вимір.** Значення = lane у v0..v31 (2×i64 або
4×i32 на 128-біт), єдина модель операндів «(файл, lane, ширина)», авто-векторизація незалежних
ітерацій (редукції, заповнення, map по масивах, побудова/скан CSR, crc/hamming). **Вердикт: на
K1H-K4/K8H (рекурентності над i64) — 0, бо NEON не має 64-біт цілого множення і рекурентність не
векторизується поперек ітерацій; на store-сканах — реальний, але стеля — DRAM (12 GB/s), і до неї
доводить уже регістрова модель + 2 lanes; авто-векторизацію відхилити (потребує IR + аналіз
залежностей), лишити builtin-рівень (hvham/crc32 — прецедент, `scan`/`sum64` — наступні).**

**(C) Тензорний/dataflow IR у ML-сенсі** (MLIR tensor dialects, XLA, TVM/Halide: граф над масивами і
loop-nest'ами: tiling, fusion scan+aggregate, розбиття по ядрах). **Вердикт: для store-кернелів
(scan, BFS, CSR build) виграш є лише там, де LLVM не може — fusion під динамічну форму запиту = домен
specialise-then-run (RESEARCH-DEPS §6d-1), і його дає не IR, а сам bebop-компілятор за 50 ms (шаблонні
кернели в поверхні bebop); ML-IR — відхилити (1500+ рядків, нуль виграшу над шаблонами).**

**(D) Інші читання.** «Граф» = вікно тегів, узагальнене до DAG (CSE: `REG r,p` уже несе ребро
def→use через індекс слова-виробника) — це той самий плоский IR (§3 RESEARCH-DEPS), 2-5 % на
самокомпіляції. «Тензор регістрів» = payload тега як трійка (file, lane, width) — це технічний шар
читання (B), корисний лише разом із builtin-векторизацією. У docs/LANG-DB-DESIGN.md §9 оператор уже
вводив «graph = tensor» для стору: там вердикт «GraphBLAS-тотожність точна; SpMSpV б'є pointer chasing
5-10x лише на фронтирах; NEON на gather'ах нічого не дає» (§9.2, §9.5) — це читання (C) на рівні даних,
і воно вже в roadmap стору (G8 `frontier`), не в компіляторі.

## 1. Читання (A): графова алокація регістрів над per-fn IR

**Модель.** Після плоского IR (рядки `{op,a,b,aux}`, блоки-діапазони, phi на входах while / join if)
кожен рядок = SSA-значення з інтервалом життя [def, last use]. Для SSA-програм граф інтерференції
хордальний (Hack/Grund/Goos, «Register Allocation for Programs in SSA-Form», CC 2006,
compilers.cs.uni-saarland.de/papers/ssara.pdf — PDF не розпарсився WebFetch'ем, твердження з пам'яті):
максимальна кількість одночасно живих значень = хроматичне число, фарбування за один прохід у порядку
домінування, без ітерацій Chaitin'а; спіл — окремий прохід ДО фарбування (знизити тиск до ≤ K), копії —
лише при розв'язанні phi (паралельне переміщення, як `vs_place_args` blueprint'а). Це і є те, що
blueprint робить жадібно й локально: вікно = «живі зараз», park = спіл «зараз», retarget = локальна
copy-coalescing одного слова.

**Що змінюється проти вікна blueprint'а:**

| аспект | вікно (blueprint, однопрохідне) | графова алокація (A) |
|---|---|---|
| рішення «в якому регістрі» | у момент появи значення, без знання майбутнього | після повної liveness: значення, що переживає виклик, ОДРАЗУ народжується в callee-saved (жодного `mov x20,x0` після `bl` у K2H: `add x0,x20,x0` стає `bl; mov`-free лише якщо результат виклику вже потрібен у cs — тут 1 mov лишається, бо `bl` повертає в x0 фіксовано) |
| копії | retarget останнього слова; `mov` при bind, коли producer не останній; парк/reload навколо викликів | coalescing усіх копій, чиї джерело й ціль не інтерферують; парки лише де тиск > 8 (у bebop.bp: ~0 функцій за оцінкою Sethi-Ullman ≤ 4) |
| символи | фіксовано x19..x26 за порядком `let`, 9+ у слоти | символ = ще одне SSA-значення: мертві після останнього use звільняють регістр, 9-й..N-й символ отримує регістр, якщо живе не одночасно з 8 іншими → **слоти зникають у більшості fn з 9-20 локалами** (use_scan, emit_body, compile_fn_at) |
| спіл | temp slots S+k, символи k | один спіл-прохід за тиском (Belady/furthest-next-use), слоти лише де тиск реально > K |
| дві проходки B1 | планувальна + емісійна мусять збігтися по n[0] | одна: розмір відомий з рядків (RESEARCH-DEPS §3: −150 рядків, −клас багів) |

**Ціна.** Понад плоский IR (600-900): liveness по блоках ~120 рядків, порядок домінування ~80,
фарбування + coalescing ~150, спіл-прохід ~120, phi-resolution = наявний parallel move → **~450-500
рядків**, і мінус ~250 рядків вікна/park/retarget, які він заміняє. Разом з IR: ~1 000-1 200 рядків
bebop.bp, один коміт-рунг ПІСЛЯ IR.

**Виграш за кернелом** (усе відносно стану «після регістрової моделі + LIN + csel»):

| кернел | що лишається після roadmap | що дає (A) | оцінка |
|---|---|---|---|
| K1H/K3H/K4 | критичний шлях = ланцюжок даних, 0 копій у циклі (retarget закриває `let v = ...`) | нічого: у циклі нема живих значень понад символи | 0 (derived) |
| K2H fib | `mov x20,x0` після першого `bl` (результат у x0 → cs) | не прибирає: ABI повертає в x0; прибирає лише tre (IR-прохід), а не алокатор | 0; tre → ~1.0x (derived) |
| K8H | після csel + LIN: ~1.0x | LICM констант робить IR-прохід, не алокатор | 0 |
| K5 самокомпіляція | після плоского IR (SROA/inline/CSE 15-30 %): у великих fn (use_scan 26 локалів, emit_body 48→ після split, compile_fn_at) — слоти символів 9+ і парки навколо ~1 300 `bl` | зникнення ~60-80 % `ldr/str [x15]` символів у fn з 9-20 локалами і половини парків: −3-8 % часу, −2-4 % bin_words | **+3-8 % K5** (guess, міряти диференційно) |

**Взаємодія:** LIN (тег над (символ, лічильник)) і csel — рівні тегів/IR-рядків, алокатор їх не
торкається; B4 отримує від (A) точний тиск замість vc (менші фрейми). Ризик: алокатор — це те, де
компілятори помиляються тихо (b4326b5-клас); Hack-схема мінімізує ітерації, але спіл-прохід + phi-move
= два нових джерела «дві проходки не зійшлися»-багів; гейт той самий chain + c-конструкти з тиском > 8.

## 2. Читання (B): NEON як другий вимір регістрів

**Факти ISA (ARMv8-A AdvSIMD на A78, без SVE):** 32 регістри v0-v31 по 128 біт; цілі арифметика на
.2d (2×i64): `add/sub/and/orr/eor/shl/ushr/sshr/cmeq/cmgt/neg/abs .2d` — так; **цілого множення на
.2d НЕМАЄ**: `mul` лише .8b/.16b/.4h/.8h/.2s/.4s; `smull/umull` = 32×32→64 (по 2 lanes), `sqdmull`,
`pmull`/`pmull2` = поліноміальне (64×64→128, для crc/GF); `smlal`/`umlal` = 32×32+64; `sdot/udot`
(asimddp) = 4×i8·i8→i32. 64-біт множення у векторі = 4 `umull`+зсуви+складання (≥8 інструкцій на 2
lanes) — гірше за 2 скалярних `mul` (1/такт кожен на 2 MAC-пайпах). Gather/scatter: **немає** (лише
`ld1/ld2/ld3/ld4` contiguous та `ld1 {v.d}[i]` по одному lane з окремою адресою = скалярний load у
вектор, 1 µop на lane); SVE gather відсутній на цьому кремнії. Редукції: `addv/addp .2d`, `cnt`+`addv`
(popcount — це hvham), `crc32x` — скалярний апаратний 8 B/такт (T109), не NEON.

**Модель операндів.** Розширення тега blueprint'а: payload `REG` → (file ∈ {x, v}, r, lane ∈ {-,0,1},
width ∈ {64, 2×64}); `vs_alloc` над двома масками (x0..x7 і v0..v7, v8-v15 callee-saved нижні 64 біт,
v16-v31 caller-saved); `vs_binop` отримує рядки для .2d-форм; matérialisation lane→x: `fmov x<d>,v<r>.d[i]`
(1 такт... на A78 `fmov`/`umov` GPR←FP має латентність 3-5 тактів і йде через V-пайп — це та «ціна
мосту», яка з'їдає виграш на коротких векторних ділянках). ~100 рядків тегів + ~200 рядків форм. Але
без ЧОГО векторизувати воно мертве: потрібен або (i) аналіз залежностей циклу (незалежність ітерацій:
кожен `arr[i] = f(arr[i], c)` без aliasing — потребує IR + escape/alias-скан, ~400 рядків, і корпус
bebop.bp майже не має таких циклів: парсер — це `char(s, pos[0])` із мутабельним `pos`), або (ii)
builtin-и з ручними NEON-словами (hvham 35 слів — прецедент; `scan(s,pos,class)` ~40 слів; `sum64(cells,n)`
~12; `fill(cells,n,v)` ~8; `cmp_mask(cells,n,c)` ~16 для зон-мап скану). (ii) — це RESEARCH-DEPS §4 пункт
6, уже в roadmap; (i) — відхилити.

**Чи відкриває LIN дво-lane форму?** Після складання `s = a^k s + B(i)` лишається ОДНА рекурентність
(один ланцюжок над одним `s`); дві незалежні рекурентності є лише в редукціях, розбитих на акумулятори
(RESEARCH-DEPS §1b(1)), і навіть тоді крок рекурентності — 64-біт `madd` → у NEON немає. K8H: `x` LCG
64-біт — те саме. Отже на K1H-K8H (B) = 0 точно, не оцінково.

**Прогноз (B) по кернелах і store-кернелах:**

| кернел | сьогодні | після регістрової моделі (RD §1) | (B) auto-vec | (B) builtin | стеля | що робить LLVM у Rust-близнюку |
|---|---|---|---|---|---|---|
| K1H/K3H/K4 | 1.8x / 2.6x / 1.4x | 1.0 / 0.6-1.2 / 1.0x | 0 (i64 рекурентність, нема .2d mul) | 0 | ланцюжок | не векторизує |
| K2H fib | 2.2x | 1.5-1.8x | 0 | 0 | виклики | не векторизує |
| K8H | 4.5x | 1.3-1.6x → 1.0-1.2 з csel | 0 | 0 | madd + csel | не векторизує (madd+csel скалярно) |
| K6 nnidx scan 1M (18.4 ms = 18 ns/row, codegen-bound, LANG-DB §3) | 9.9x vs sqlite | ~5-8 ns/row: ~30-40x vs sqlite (derived: push/pop геть із внутрішнього циклу — 4 слова з ~12) | 2 lanes: ~3-4 ns/row → ~50x (derived) | `cmp_mask`/`sum64` builtin у циклі скану: те саме ~3-4 ns/row | **DRAM: 24 B/row при 12 GB/s = 2 ns/row → ~80-90x vs sqlite** (LANG-DB §5: «112x measured with Rust-quality codegen») | Rust-скан 1.4 ns/row (auto-vec + prefetch) — (B) доводить bebop до ~1.5-2x від Rust на скані, не далі |
| BFS 10M ребер (sgraph) | 15-40 ns/edge | ~10-20 (менше слів у циклі) | 0 (gather нема, LANG-DB §9.2) | 0 | пропускна промахів: ~10 ns/edge пайплайновано | Rust те саме, ~1x |
| CSR build 1M/10M (counting sort, ~50-100 ms) | — | ~20-40 ms (derived) | 0 (scatter) ; `fill` builtin на обнулення | ~1.2x | 3 проходи по 24 MB = 5-10 ms при DRAM | Rust ~1x на тюнінгованому |
| point lookup nnidx 4.0 us | 13.8x vs sqlite C | ~3 us | 0 (35 line misses — це не обчислення) | 0 | 1.3 us з кластеризацією (LANG-DB §9.1) — store-механізм, не регістровий | — |
| crc32 1 MB / hamming | апаратний / NEON `cnt` | без змін | — | уже є | 8 B/такт / 16 B/такт | Rust без `+crc` — табличний, 5-8x повільніше; з `+crc` — 1x |
| парсерні цикли K5 (skip_ws/read_ident/skip_string, 1 байт/~5 т) | 1.56 s | ~1.0-1.2 s | 0 (мутабельний `pos`, залежність по даних) | `scan` builtin: 16 байт/~3 т → **K5 −10-25 %** (RD §1b(3), guess) | пам'ять L1 | — |

**Пропускна здатність A78:** 2 ASIMD-пайпи × 2 lanes i64 = 4 i64-операції/такт проти 4 скалярних ALU
(2 з MAC) = 4/такт — на add/sub/логіці NEON НЕ виграє нічого по пропускній на цьому ядрі; виграє лише
на ширині завантажень (`ld1` 16 B/µop проти `ldr` 8 B, 2 load-пайпи обидва) і на редукціях (`addp`)
та popcount (`cnt`). Тобто (B) — це «2x на memory-стрімах до DRAM-стелі», не «×2 обчислень».

## 3. Читання (C): loop-nest/tensor IR над масивами для стору

Що дає TVM/Halide-клас: tiling під L2 (1 MB/ядро), fusion scan+filter+aggregate в один прохід, розбиття
по ядрах. Що з цього bebop уже має або отримує дешевше: fusion — за побудовою у згенерованому кернелі
(specialise-then-run: bebop генерує `.bp` під схему/предикат/агрегат і компілює за 50 ms — це та сама
fusion без IR, RESEARCH-DEPS §6d-1, 5-30x на latency-to-result проти Rust generic); розбиття по ядрах —
`sys_clone` + `sys_setaffinity` (nn4.bp 1→3 A78 = 2.21x виміряно, але це поки codegen-bound: після
регістрової моделі скан упирається в DRAM, і 3 ядра дадуть 1.0-1.4x на стрімах, LANG-DB §3 — виграш
multi-core ЗМЕНШУЄТЬСЯ з кращим кодогеном); tiling — зона-мапи по блоках (LANG-DB §9.5) роблять те
саме на рівні даних. Rust не має такого оптимізатора теж, тому виграш «проти Rust» лишається виключно
у specialise-then-run. Ціна ML-IR у поверхні bebop: ≥1 500 рядків (індексні простори, афінні залежності,
планувальник), нуль залежностей формально досяжний, але це другий компілятор. **Відхилити;** взяти
шаблонні кернели (`gen_scan.bp`, що пише `.bp`) — ~150-250 рядків, гейт twin §6d-1.

## 4. Прогноз: сьогодні → регістрова модель → прийнятий roadmap → тензорно-графовий апгрейд

Legend: M = measured, D = derived (дизасемблювання + латентності), G = guess. Проти Rust = ms/rep
bebop / ms/rep Rust-близнюка (менше = краще; 1.0x = паритет). Roadmap = T52 csel, LIN, хостинг
констант, NEON `scan`, B4, плоский IR (SROA/inline/tre/CSE), per-fn memo (оператор: потрібен).

| метрика | сьогодні | після регістрової моделі | після roadmap | після (A)+(B)+(C) | стеля / примітка |
|---|---|---|---|---|---|
| K4 vs Rust | 1.4x M | 1.0x D | **0.55x** D (LIN ×2) | 0.55x | ланцюжок 4.5→2.3 т; (B) 0 |
| K1H vs Rust | 1.8x M | 1.0-1.05x D | **0.35x** D (LIN ×4) | 0.35x | 1 т/ітер |
| K3H vs Rust | 2.6x M | 0.6-1.2x D | **0.3-0.5x** D | 0.3-0.5x | (B) 0 |
| K2H vs Rust | 2.2x M | 1.5-1.8x D | ~1.0x D (tre у плоскому IR) | ~1.0x (A: 0) | ABI повертає в x0 |
| K8H vs Rust | 4.5x M | 1.3-1.6x D | 0.7-1.0x D (csel + LIN на x + hoist) | 0.7-1.0x | mispredict зникає з csel |
| K5 самокомпіляція | 1.56 s M | 1.0-1.2 s G | 0.7-0.9 s G (scan −10-25 %, IR −15-30 %, memo теплий 0.07 s) | **0.65-0.85 s** G (A: −3-8 %) | парсер — гілки, не арифметика |
| K6 scan vs sqlite (1M, Q=20) | 9.9x M | ~30-40x D | ~40-50x D (cmp_mask builtin) | **~80-90x** D (2 lanes + `ld1`, DRAM-стеля 2 ns/row) | Rust 1.4 ns/row: bebop ~1.5-2x від Rust |
| K6 scan vs Rust | ~13x M (18 vs 1.4 ns/row) | ~4-6x D | ~3x D | **~1.5-2x** D | без prefetch-builtin не 1.0x |
| BFS vs sqlite CTE | 40-100x M (15-40 ns/edge vs 1.5 us) | ~60-150x D | + frontier SpMSpV 5-10x на фронтирах (store, не компілятор) | те саме | gather нема; vs Rust ~1x |
| point lookup vs sqlite C-API | 13.8x M (4.0 us vs 55) | ~18x D | ~40x D з кластеризацією (1.3 us; store) | те саме | промахи, не обчислення |
| CSR build 1M/10M | ~50-100 ms G | ~20-40 ms D | ~15-25 ms D (`fill`) | ~12-20 ms G | DRAM 5-10 ms |
| RSS програм | 15.5 MB M | 15.5 MB D | −16 KiB/активацію після B4 (рекурсія: TRAP-82 = 0) | те саме | арена/mmap домінує |
| bin_words | 68 229 M | < 55 000 D | ~45-50 k G (IR: inline+dce; memo не впливає) | ~43-48 k G (A: −2-4 %) | — |
| компіляція kernel (латентність) | 50 ms M | ~40 ms G | ~35 ms G (memo: повтор 0.07 s → in-process ms) | — | Rust 1.34 s: 27-40x |

Що НЕ дає жоден апгрейд регістрів: 10x на K1H-K4 (кремній), 1.0x на K6 проти Rust без prefetch/`ld1`
у циклі (Rust 1.4 ns/row — уже DRAM-стеля), multi-core на стрімах понад 1.4x (DRAM насичує одне A78).

## 5. Рекомендація

1. **(A) графова алокація — так, але лише як другий рунг плоского IR** (roadmap пункт 8): спершу IR з
   SROA/inline/tre (виміряти K5 вручну, поріг 15 %), потім Hack-фарбування + coalescing замість вікна
   (~450 рядків, мінус вікно/park/retarget ~250). Гейт: K5 −3 % понад IR, bin_words −2 %, chain GREEN,
   конструкти з тиском > 8 (c55/c56/c62/c63). Не раніше: на кернелах 0.
2. **(B) NEON — лише builtin-рівень**, у порядку RD §4 п.6: `scan` (K5, поріг 10 %), потім `cmp_mask` +
   `sum64` для K6-скану (гейт: ns/row ≤ 4, twin Rust scan у тому ж honest-звіті), `fill` для CSR build.
   Розширення payload'а тега до (file, lane, width) — НІ, поки немає авто-векторизації; авто-векторизацію
   відхилити (i64 без .2d mul, gather нема, корпус без незалежних циклів).
3. **(C) tensor/loop-nest IR — відхилити;** specialise-then-run через шаблонні `.bp`-кернели (RD §6d-1
   twin) закриває fusion; multi-core — уже є, і його частка падає з кращим кодогеном.
4. **(D)** DAG-вікно = плоский IR; трійка (file, lane, width) — разом із (B) builtin-рівнем не потрібна.
5a. §7 (радикальні моделі виконання): усі п'ять — не цілі; беруться лише як компайл-тайм читання, які
   вже є в roadmap (плоский IR = канонічна мінімальна форма; fold/LIN/CSE = graph rewriting у
   компайл-таймі; незалежні ланцюжки §1b = «dataflow» для OoO-ядра; store = single-level store;
   specialise-then-run = «конфігурація grid'а»); нових пунктів не додають.
5. Порядок після регістрової моделі лишається як у RESEARCH-DEPS §4 (csel → LIN → specialise twin →
   hoist → NEON scan → B4 → плоский IR → **графова алокація як IR-рунг 2** → per-fn memo); дві
   measured-first позиції з цього звіту: `cmp_mask`/`sum64` для K6 (після NEON scan) і алокатор (після IR).

## 6. Докази

- /proc/cpuinfo: Features без `sve`; CPU part 0xd05 (A55) / 0xd41 (A78); 8 ядер. A78 SOG (Arm,
  documentation-service.arm.com/static/630382d1e95b0a633aff8a78): Integer Single-Cycle 0/1 + Single/Multi-cycle
  0/1 (mul/div/CRC), FP/ASIMD-0/1 — з пошуку; latencies з RESEARCH-DEPS.
- ISA: `mul` (vector) arrangements 8B/16B/4H/8H/2S/4S only; `smull/umull` 32→64; `pmull` 64→128
  polynomial; `sdot/udot` 8-bit; `ld1 {Vt.D}[index]` single-lane load; no gather in AdvSIMD (SVE
  `ld1d` gather only) — ISA knowledge, узгоджено з LANG-DB-DESIGN.md:475-480 («no SVE, NEON = 2 x i64
  lanes, no gather instruction»).
- Hack/Grund/Goos 2006 (compilers.cs.uni-saarland.de/papers/ssara.pdf, PDF не розпарсився — з пам'яті):
  chordal interference graphs of SSA programs, colouring in O(n) by dominance order with χ = max pressure,
  spilling decoupled, copies at phi. regalloc2 DESIGN.md — 404 при fetch; з пам'яті: SSA-вхід, бандли
  live-range'ів, backtracking з розщепленням, move resolution, спіл-слоти за класами.
- Repo: bebop.orig.bp emit_hvham (35 `em` слів NEON: `ldp q0,q1`, `eor`, `cnt`, `addv`), emit_crc32
  (`crc32b/crc32x` скалярні), selfhost/std/csr.bp:1-12 (rp/ci/vv, from_edges contract), sgraph.bp:3-13
  (1M/10M CSR у сторі, BFS/nbr), bench/tq_sqlite/RESULT.md (18.4 ms scan = 9.9x; 4.0 us indexed = 13.8x;
  nn4 3 A78 = 2.21x), docs/LANG-DB-DESIGN.md:18-20, 121 (DRAM 12 GB/s, 1.4 ns/row Rust, 18 ns/row bebop
  codegen-bound), 474-560 (§9: dimensional descent = grid file; graph = tensor exact; SpMSpV 5-10x on
  frontiers only; NEON adds nothing on gathers), 638-662 (§9.5 verdicts, G8 gate);
  docs/RESEARCH-DEPS-2026-09-06.md §1, §1b, §3, §4, §6d; docs/REGISTER-MODEL-BLUEPRINT.md §1-§2.

## 7. Радикальні моделі виконання проти регістрової

Мірило для всіх п'яти: кінцевий продукт — AArch64-слова від self-hosted компілятора без залежностей на
Cortex-A78/A55 під proot; будь-яка модель, що не є машинним кодом цього ядра, або емулюється (і платить
за емуляцію), або лишається компайл-тайм читанням.

| # | модель | що прибирає | ціна на реальному A78 | прибуткова емуляція? | K1H-K8H / K5 / K6-BFS | вердикт |
|---|---|---|---|---|---|---|
| 1 | **OISC / subleq** (одна інструкція `Mem[b] -= Mem[a]; if <= 0 goto c`; en.wikipedia.org/wiki/One-instruction_set_computer, techtinkering.com/articles/subleq-a-one-instruction-set-computer, arxiv.org/pdf/1106.2593 «A Simple Multi-Processor Computer Based on Subleq» — фетчено пошуком) | регістри, opcode, алокацію | кожен subleq = 2 `ldr` + `sub` + `str` + `b.le` = 5 слів, латентність через store→load forwarding ~5-6 т на крок проти 1 т `sub` у регістрі; add = 3 subleq, mul = цикл → **10-50x повільніше** за регістровий код на тих самих обчисленнях (джерела вище кажуть те саме: «many more memory accesses per operation»); L1-трафік ×4 | лише як компайл-тайм канонічна форма: 3-адресний рядок `{op,a,b,aux}` плоского IR (RESEARCH-DEPS §3) — це і є «мінімальний набір» з 12-15 op замість 1; subleq-IR потребував би зворотного відновлення `mul/madd/csel` із ланцюжків віднімань (паттерн-матчинг, який регістрова модель щойно видалила) | кернели 10-50x гірше як runtime; як IR — 0 проти плоского IR, мінус читабельність | **відхилити** як runtime; плоский IR = правильна «мінімальна форма» |
| 2 | **Dataflow** (без PC, вузол спрацьовує за готовими токенами: Manchester 1981 — cs.duke.edu/~lisa/papers/manchester-cacm1985.pdf; Monsoon ETS 1990 — dl.acm.org/doi/10.1145/325164.325117; огляд «Dataflow: Passing the Token» csg.csail.mit.edu/Users/arvind/ISCAfinal.pdf; чому не вижили — ieeexplore.ieee.org/document/9623012: matching-store вартість, транзисторний бюджет OoO зробив те саме дешевше) | програмний порядок, алокацію (токени замість регістрів) | **A78 і є dataflow-машиною над вікном ~160 µop** (ROB, renaming, wakeup/select) — регістровий код ПЕРЕТВОРЮЄТЬСЯ на dataflow апаратно; програмна емуляція (token store в пам'яті, matching) = інтерпретатор, 20-100x | так, двома читаннями, обидва вже в roadmap: (i) компілятор ЕКСПОНУЄ незалежні ланцюжки, щоб OoO мав що перекривати — §1b (LIN, акумулятори); (ii) task-level dataflow для стору: черги задач через `sys_clone`/futex (nn4 2.21x виміряно; на DRAM-стрімах стеля 1.4x) | (i) — виграш §1b (K1H 3x, K4 1.8x над Rust); (ii) K6/BFS до 1.4x на стрімах, до 2-3x на codegen-bound фазах | **прийняти як читання**, нових пунктів 0 |
| 3 | **Graph reduction / SK-комбінатори** (чисте переписування без стану; реалістичний еталон — GHC STG: Peyton Jones 1992, cs.tufts.edu/~nr/cs257/archive/simon-peyton-jones/spineless-jfp.pdf; Egel arxiv.org/pdf/2004.09843) | змінний стан, регістри як поняття мови | кожна редукція = алокація вузла в купі (закриття/thunk) + непрямий перехід (tag/enter) + GC; на скалярних циклах 3-10x повільніше за C навіть у GHC з strictness-аналізом, і без GC не існує — прямо проти арени-без-free bebop (`let`-rebind у циклах і `[0]`-комірки — антитеза) | так, у компайл-таймі: fold (CONST op CONST), LIN-композиція, CSE над DAG, `mulc`/`madd`-таблиці — це graph rewriting над IR без runtime-редукції; equality saturation (FlexC 2023 для CGRA, RESEARCH: arxiv 2508.02167 згадує) — e-graph, відхилено у RESEARCH-DEPS §2 (Cranelift aegraph) як надлишкове | runtime: 3-10x гірше + GC; компайл-тайм: = плоский IR-проходи (K5 15-30 %) | **відхилити** runtime; компайл-тайм читання = пункт 8 roadmap |
| 4 | **CGRA / spatial** (масив ALU з конфігурованими зв'язками, дані течуть просторово; огляд Liu et al. 2019 dl.acm.org/doi/fullHtml/10.1145/3357375; MLIR-компіляція arxiv.org/pdf/2508.02167) | fetch/decode, регістровий файл (значення живуть на дротах), PC | такого заліза тут немає; найближчі аналоги на A78: NEON lanes (§2: 2 пайпи × 2 lanes, без .2d mul) і OoO-вікно; емуляція CGRA = інтерпретатор графа, 20-100x | так, читанням «конфігурація grid'а = згенерувати спеціалізований кернел під запит» — specialise-then-run (RESEARCH-DEPS §6d-1, 50 ms компіляція) робить те, що CGRA робить перезаписом конфігурації за мкс; що дав би реальний CGRA store-сканам: fusion filter+aggregate без регістрового тиску й потокова обробка при DRAM-швидкості — тобто рівно DRAM-стелю 2 ns/row, яку §2 (B) обіцяє і без нього | K6 scan: те саме, що §4 таблиця «після (B)» (DRAM-стеля); BFS: 0 (промахи) | **не ціль**; читання = specialise-then-run, уже в roadmap |
| 5 | **Single-level store / capabilities** (плоский адресний простір, персистентність синхронізована з пам'яттю — IBM i / System/38 єдиний адресний простір 128-біт (з пам'яті, пошук не дав джерела); CHERI/Morello: 128-біт capability = 64-біт адреса + межі + права, semiengineering.com/capabilities-in-cap-cheri-and-morello, eprints.whiterose.ac.uk/id/eprint/231424 «Performance Characterization of the Arm Morello Platform», lwn.net/Articles/1037974) | межу load/store ↔ persist; для CHERI — небезпечні вказівники | Morello — квадро N1 2.5 GHz, тут немає; capability-регістри ≠ memory-to-memory ALU: AArch64 (і Morello) не має ALU-операцій пам'ять-пам'ять, регістри фізично лишаються; вартість CHERI: 128-біт вказівники ×2 трафіку на pointer-heavy коді (BFS ci/rp — +50 % байтів), ~0-10 % на скалярному | bebop **уже є** single-level store у програмному сенсі: mmap-арена, `st_open`/`msync`/`crc32x`, комірки адресуються однаково в пам'яті й на диску (LANG-DB-DESIGN §4); читання для компілятора: комірки стору — першокласні операнди = регістро-параметризовані `ldr/str [base,#imm]`/`[base,idx,lsl 3]` blueprint'а §3.10 + zero-copy §6d-8; capability-межі емулюються бітмапами/зон-мапами стору (§9.5) без ISA | 0 на кернелах; K6/BFS: те, що вже враховано (zero-copy ~1x vs best Rust) | **прийняти як констатацію** (уже є); CHERI — не тут |

Спільний висновок §7: жодна з п'яти не є заміною регістрової моделі на цьому залізі — усі, крім (5), у
runtime коштують 3-100x, а (5) вже реалізована програмно. Усе цінне в них — компайл-тайм або store-level
читання, які roadmap уже містить: плоский 3-адресний IR (1), експонування незалежних ланцюжків + task-
dataflow по ядрах (2), fold/LIN/CSE як graph rewriting (3), specialise-then-run (4), комірки стору як
операнди + zero-copy (5).

VERDICT: «тензорна графічна модель регістрів» розпадається на (A) графову алокацію — прийняти як другий рунг плоского IR
(~450 рядків, K5 +3-8 %, кернели 0), (B) NEON-вимір — лише builtin-рівень (scan/cmp_mask/sum64/fill; K6 скан до ~80-90x vs
sqlite = DRAM-стеля, ~1.5-2x від Rust; K1H-K8H 0, бо i64 рекурентності без .2d mul), (C) tensor/loop-nest IR — відхилити на
користь specialise-then-run шаблонів; сукупний прогноз після всього roadmap + (A)(B): K1H 0.35x, K4 0.55x, K3H 0.3-0.5x, K2H ~1.0x,
K8H 0.7-1.0x від Rust; K5 0.65-0.85 s; K6 scan ~80-90x vs sqlite; RSS без змін, bin_words ~45 k; §7: OISC/dataflow/graph-reduction/CGRA/single-level-store — жодна не є runtime-ціллю на A78 (3-100x або вже реалізовано), усі корисні читання (плоский IR, незалежні ланцюжки, compile-time rewriting, specialise-then-run, комірки стору як операнди) уже в roadmap.
