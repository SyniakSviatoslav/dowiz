Status: 2026-09-06 (session 18) -- research report by a Fable fork, read-only over /root/dowiz/bebop-lang at HEAD 9e53878
(B5) with the register-model worker in flight. Box: 4x Cortex-A55 (0xd05) + 4x Cortex-A78 (0xd41); honest.sh and
chain.sh pin to the A78 cores 4-6 (tools/chain.sh:23). Cycle figures use Cortex-A78 latencies (add/sub 1, add-shifted 2,
mul/madd 3 with 1-cycle accumulate forwarding, sdiv 8-14, cset/csel 1, ldr 4, str->ldr forwarding ~5, mispredict ~13)
at ~2.8 GHz (K4: Rust 3.25 ms / 2M iter = 4.5 cycles at 2.8 GHz matches its 4-cycle chain); they are estimates, the
honest.sh rows are the measurements. Rust twins rebuilt here with rustc 1.96.1 `-O` and disassembled (§5.3).

# Ланцюжки залежностей, бекенди без залежностей і що з цього треба bebop

## 0. Два різні «ланцюжки залежностей»

**(а) Ланцюжок залежностей даних у циклі (CPU).** У прогнозі швидкості йшлося саме про це: у K4
`v = (v + i*7)*3 - 11` кожна ітерація читає `v` попередньої, тому мінімальний час ітерації = сума
латентностей інструкцій на шляху `v -> v'`, скільки б не було вільних портів. LLVM -O3 (§5.3) емітить
`add v,v,t ; add v,v,v,lsl #1 ; sub v,v,#11` (`i*7` зведено до running `sub t,t,#7` поза ланцюжком) =
1+2+1 = **4 такти/ітерацію**, виміряно 4.5 (3.25 ms на 2M). Сьогоднішній bebop (14 слів, §5.1): той
самий ланцюжок плюс `mov x1,x0 ; mov x0,x19` на шляху = +2 такти → 6.5 = 4.6 ms, збігається з
виміряним 4.60. Це і є «як у LLVM»: після регістрової моделі bebop емітить той самий ланцюжок і
впирається в ту саму межу. Зсунути межу може лише **алгебра** (§1b: складання лінійної рекурентності
за k ітерацій), і саме цього LLVM на цих кернелах не робить.

**(б) Ланцюжок залежностей збірки/архітектури (LLVM).** Те, про що йдеться в питанні: `Value.h`/
`Instruction.h` → мільйони рядків, шаблони, віртуальний поліморфізм, універсальний бекенд. Це
проблема *компілятора як артефакту* (час збирання, розмір, залежності), а не швидкості згенерованого
коду. Для bebop вона не стоїть: bebop.bp — 5.3 тис. рядків однією мовою, компілюється сам собою за
1.5 с, залежностей нуль, seed-лоадер 200 рядків асемблера. Отже питання не «як утекти від LLVM», а
**«які з ідей QBE/Cranelift/плоского IR/Salsa додають оптимізаційну силу bebop, не додаючи
залежностей»**, і — §6 — де за це Rust платить так, що різниця стає порядком.

## 1. Де насправді йде час у кернелах після регістрової моделі

Джерело: дизасемблювання поточних kernels під HEAD bebop.bin (§5.1), Rust-близнюків (§5.3), латентності A78.

| кернел | Rust honest | bebop зараз | критичний шлях (такти/ітер) | після регістрової моделі | що робить LLVM у близнюку | що ще лишається |
|---|---|---|---|---|---|---|
| K4 `v=(v+i*7)*3-11` | 3.25 ms (4.5 т) | 4.60 (6.5 т: +2 `mov` на шляху) | add→add-lsl→sub = 4 | **~3.3 ms, 1.0x** | strength reduction `i*7`; без unroll, без складання рекурентності | лише §1b (4)+(2): 1.7-2x над Rust |
| K1H `s=s*3+i` | 1.12 ms (3.1 т) | 2.03 (5.7 т) | add-lsl(2)→add(1) = 3 | **~1.1-1.2 ms, 1.0-1.05x** | нічого (5 слів, без unroll) | §1b: до 3x над Rust |
| K3H `a=a*3+x*2+y*3` | 0.257 ms (8 т вимір.) | 0.68 (21 т: два str→ldr ланцюжки + шаффли) | add→add-lsl→add = 4 | **~0.15-0.30 ms, 0.6-1.2x** | реасоціація `(a+y)*3 + 2x` + LICM `2x`; без unroll | §1b: 2-3x над Rust |
| K2H fib(25) | 0.39 ms (4.5 т/виклик) | 0.87 (10 т/виклик) | bl/ret + stp/ldp + sub sp + push/pop | **~0.6-0.7 ms, 1.5-1.8x** | другий рекурсивний виклик → цикл з акумулятором (одне `bl` на активацію, §5.3) | потрібна tail-recursion→loop трансформація (IR-клас) |
| K8H LCG + branchy | 0.069 ms (9.7 т вимір.) | 0.31 (43 т) | madd(3)→(tst→csel)→add; bebop: +push/pop 5, +8 слів констант, +mispredict ~7 сер. | **~0.09-0.11 ms, 1.3-1.6x** | LICM обох 64-біт констант, `madd`, `tst`+`csel` (без гілки) | T52 csel → ~1.0-1.2x; хостинг констант −8 слів |

Що ще може зробити **однопрохідний прямий емітер** (без IR), і скільки це дає (рядки §1b додано):

| техніка | де в bebop | виграш | ціна |
|---|---|---|---|
| вибір інструкцій на тегах (madd/msub, add-lsl, lsl/lsr/asr #imm, cmp #imm, cbz/cbnz) | REGISTER-MODEL-BLUEPRINT §2 — у коміті, що йде | K4 6.5→4.5 т, K3H 21→~5 т, K1H 5.7→3 т | вже оплачено |
| loop rotation | B5, залендено | −1 слово/цикл, ~0 ms | 0 |
| `csel` для чистих if-гілок (T52) | ROADMAP item 4; на тегах FLAGS/REG | K8H mispredict ≈55 % → ~1.0-1.2x проти Rust-csel | ~60 рядків + текстова перевірка «чистості» гілок |
| хостинг інваріантних констант циклу | K8H: 2×(movz+3 movk) = 8 слів/ітер (§5.2); B5-скан тіла на літерали > 16 біт → callee-saved регістри | K8H −8 слів (~−3 т пропускної); поза K8H малий | ~80 рядків |
| літерал-пул (`ldr x,[pc-rel]`) замість movz/movk | та сама латентність, −3 слова на 64-біт константу | bin_words −1-2 % | ~40 рядків |
| §1b(1) незалежні акумулятори | лише чисті редукції (`acc = acc + f(i)`): `sum_words`, checksum-цикли; K1H-K8H — рекурентності, не редукції | кернели 0; редукції корпусу 2-3x (обмежено `ldr`) | входить у (2) |
| §1b(2)+(4) unroll ×k зі складанням лінійної рекурентності на тегах | K1H, K3H, K4, K8H(x), хеш-цикли `h = h*131 + ch` | **K1H 3→1 т/ітер (×4), K4 4.5→~2.3, K3H 4→~2, K8H 3→1.5** — 1.5-3x над Rust -O3, який цього не робить (§5.3) | ~300 рядків (тег LIN, §1b) |
| §1b(3) NEON — builtin-рівень | `hvham/hvham2/crc32` є; новий builtin `scan(s,pos,class)` для `skip_ws/read_ident/skip_string` | кернели 0 (немає 64-біт NEON mul); самокомпіляція −10-25 % (оцінка) | ~40 слів + 30 рядків |
| §1b(5) software pipelining | потребує IR + модель латентностей; A78 out-of-order сам перекриває ітерації | 0 на A78 | відхилити |
| unrolling ×2 без алгебри | не змінює ланцюжок | 0 | не робити |

Що **потребує IR** (граф значень на функцію), і скільки це дало б:

| оптимізація | потреба | виграш на K1H-K8H | виграш на самокомпіляції |
|---|---|---|---|
| LICM виразів (не лише констант) | def-use граф + домінування | 0 | ~0: парсер — `char(s,pos)` з мутабельним `pos` |
| GVN/CSE | те саме | 0 | 2-5 % (оцінка): повторні `char(s, pos[0]+k)` |
| планування інструкцій | граф залежностей + латентності | ~0 (A78 OoO, ROB 160) | 0 |
| SROA `[0]`-комірок (`pos[0]`, `n[0]`, `i[0]`) | alias-аналіз (escape-скан як `loop_alloc_safe` є) | 0 | **10-20 %** (найбільший невзятий приз) |
| інлайнинг (`slen`, `is_alpha`, `is_digit`; fib) / tail-recursion→loop | граф викликів | K2H 1.6x → ~1.0x | 5-10 % |

## 1b. Техніки приховування латентності ланцюжка

Базовий факт для всіх п'яти: арифметика bebop — i64 з wraparound (кільце Z/2^64), тож **add/sub/mul
асоціативні й дистрибутивні ТОЧНО**; div/mod, зсуви вправо, порівняння — ні. bpref.py рахує в тому
самому кільці, тому будь-яке переставлення add/mul зберігає паритет байт-у-байт. Rust-близнюки —
`wrapping_*` з тими самими властивостями, і LLVM -O3 у жодному з п'яти близнюків не розгортає цикл
і не складає рекурентність (§5.3) — «ratio vs Rust» нижче чесний.

| № | техніка | легально в bebop | однопрохідний емітер на тегах | потребує IR §3 | K1H | K3H | K4 | K8H | K2H | попадання в корпус | рядки bebop.bp |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | незалежні акумулятори (acc1..acc4, ланцюжок RAW розбито; OoO перекриває) | так для комутативних редукцій `acc = acc ⊕ f(i)` (⊕ = add/mul/and/or/xor); K1H/K3H/K4/K8H — це `s = a*s + b(i)`, НЕ редукції → техніка (1) сама по собі не застосовна, потрібна (4) | так: `while` уже пре-сканує текст (B5); тіло виду `let acc = acc + <вираз без acc>` → k копій тіла з `acc_j`, `i` зсунуто, злиття після циклу | ні | 0 | 0 | 0 | 0 | 0 | `sum_words`, `count_word`, checksum/crc-цикли std_tests: 2-3x на них, обмежено `ldr` 2/такт | входить у 2 |
| 2 | unroll ×k + незалежні акумулятори | так (k ділить лічильник; хвіст — окремий цикл) | так, як (1); сам по собі unroll без алгебри нічого не дає на рекурентностях | ні | 0 без (4) | 0 без (4) | 0 без (4) | 0 без (4) | 0 | редукції: як (1) | ~150 (unroll+хвіст) |
| 4 | алгебраїчна реасоціація → **складання лінійної рекурентності**: `s = a*s + b(i)` за k кроків = `s = a^k*s + B(i)`, B афінна в `i` → 1 madd + 1 add на k ітерацій | так: точне в кільці для add/mul/sub; заборонено при div/mod/`>>`/cmp у рекурентності | так: новий тег `LIN(a, b, c)` = `a*s + b*i + c` над (символ, лічильник); тіло = лети афінних форм → композиція k кроків = множення матриць 3×3 у компайл-таймі; емісія: `movz a^k` hoisted, `madd s, s, ck, t` де `t = B(i)` поза ланцюжком | ні (форма обмежена, але вона і є кернельною) | ×2: 9s + 4i − 1 → 4 т/2 іт = 2 т/іт; ×4: 81s + 40i − 18 → 4 т/4 іт = **1 т/іт (3x над Rust 3.1 т)** | ×2: 9a + 8x + 12y − 3 → **~2 т/іт (Rust вимір. 8 т)** | ×2: 9v + 84i − 65 → ~2.3 т/іт (**1.8-2x над Rust**) | x: ×2 = M²x + (MC+C): 1 madd/2 іт → 1.5 т/іт, але кожен x потрібен для біта → два madd від x (незалежні) + csel | 0 | K1H/K3H/K4/K8H, LCG-генератори у std_tests (багато), хеші `h*131+ch` у `read_ident` (короткі ідентифікатори — ефект малий) | ~300 (тег LIN + композиція + hoist констант) |
| 3 | NEON (2×64-біт lanes на A78, 2 NEON-пайпи) | так, лише через builtin (hvham/crc32 — прецедент); авто-векторизація потребує IR + аналіз залежностей | builtin-рівень: `scan(s, pos, class)` — cmeq по 16 байт, перший не-збіг; `sum64(cells, n)` — addp | так для auto-vec — відхилити | 0 (NEON не має 64-біт mul; рекурентність не векторизується) | 0 | 0 | 0 | 0 | парсерні цикли самокомпіляції (`skip_ws`, `read_ident`, `skip_string`, `skip_line_comment`, `collect_fns`): 1 байт/~5 т → 16 байт/~3 т; **K5 −10-25 %** (оцінка, міряти диференційно) | ~40 слів + 30 |
| 5 | software pipelining / modulo scheduling | так | ні (потрібна модель латентностей і II) | так, і навіть тоді: A78 out-of-order (ROB 160) перекриває ітерації апаратно, коли ланцюжок дозволяє; допомагає лише на in-order A55 (кернели там не міряються) або коли `ldr` стоїть у ланцюжку (pointer chasing у BFS — там доречніший prefetch-builtin) | 0 | 0 | 0 | 0 | 0 | 0 | відхилити |

Що робить LLVM у близнюках (§5.3, за кернелом): K4 — strength reduction (`i*7` → running sub), без
unroll, без (4); K1H — нічого; K3H — (4) у межах ОДНІЄЇ ітерації (`(a+y)*3 + 2x`) + LICM `2x`, без
unroll; K8H — LICM констант, `madd`, `tst`+`csel`; K2H — другий виклик перетворено на цикл з
акумулятором. Тобто LLVM не складає рекурентність за k ітерацій ні в одному близнюку — техніка (4)
на тегах є **єдиною з п'яти, що дає виграш над Rust на K1H-K4 (1.8-3x)**, і вона точна лише завдяки
wraparound-семантиці мови. Гейт для неї — той самий honest.sh (ms/rep на близнюку), тому її треба
оцінювати за попаданнями в корпус: 4 з 5 кернелів, LCG/хеш-цикли std_tests; самокомпіляція ~0.

## 2. Огляд підходів

| підхід | що це | ціна (рядки/проходи/залежності) | що в bebop уже є | що додало б | вердикт |
|---|---|---|---|---|---|
| **LLVM** | універсальний SSA-IR + mid-end (~100 проходів) + TableGen-бекенди; C++ | ~10 M рядків, години збирання, залежність від clang/cmake; нездатний до self-host у bebop | нічого спільного, і це правильно | ті 5 % на кернелах, яких bebop не добере, і жодного разу не складання рекурентності | **відхилити** (AGENTS.md: self-host, нуль залежностей) |
| **QBE** (c9x.me/compile: «70 % of the performance of industrial optimizing compilers in 10 % of the code», single SSA IL; copy elimination, SCCP, DCE, «registerization of small stack slots», linear RA with hinting, loop-aware spilling, amd64 addressing modes) | мінімальний SSA-бекенд у C, ~15 тис. рядків | як залежність — ні (C, не self-host); як *зразок* — так: список його проходів = рівно те, чого бракує bebop (SCCP ≈ fold на тегах, copy-elim ≈ retarget, registerization ≈ SROA `[0]`-комірок, addressing modes ≈ `ldr [base,idx,lsl 3]`) | тег-модель дає 3 з 6 без IR | SROA/inline потребують §3 | **взяти як мірило**: якщо є IR, він робить саме цей список, не більше |
| **Cranelift** | Rust; CLIF на індексних аренах (`Value`/`Inst` = u32), ISLE-DSL вибору інструкцій, e-graph mid-end (aegraph, 2022), regalloc2 | ~200 тис. рядків Rust; cargo-екосистема | індексна форма (fntab/insns/stab — u64-індекси в арені) | e-graph — надлишкове; «правила вибору як таблиця» = §2 blueprint | **відхилити як залежність; запозичити «IR = масиви індексів»** |
| **Плоский IR на індексах** (LuaJIT SSA IR 2.0: лінійний масив 64-біт інструкцій, 16-біт посилання, константи нижче bias; FOLD/CSE/DCE/LOOP; wiki.luajit.org/SSA-IR-2.0 — не завантажено, self-signed cert) | IR = масив рядків `{op, a, b, aux}`, блоки = діапазони | `insns[]` — плоский масив машинних слів; `fntab[2000+3i]` — плоский масив тегів; .bin = масив слів | **єдиний кандидат на IR для bebop** — §3 | **розглянути після регістрової моделі, лише під SROA/інлайн/CSE** |
| **Salsa / rustc queries** | кожен прохід — чиста функція від хешу; кеш у пам'яті; інкрементальність | ідея, не бібліотека | `.becache` (memo всього виходу за байтами компілятора+джерела), гейт-memo `BEBOP_MEMO`, факти на fn | per-fn memo (хеш тексту fn → слова; `bl`-офсети залежать від layout — memo слів, не адрес) | **низький пріоритет**: K5 1.5 с, теплий .becache 0.07 с |
| **Direct-to-machine** | текст → машинні слова за 1-2 проходи, свій лоадер/арени | це bebop | все | — | **база; не міняти** |
| Go SSA backend | генеричний SSA + `.rules` (rewrite-правила → Go), ~50 проходів | ~100 тис. рядків | — | нічого понад QBE-список | приклад, не мета |
| Zig self-hosted (x86_64 без LLVM) | AIR (плоскі індекси) → машинний код напряму; оптимізацій майже немає | ~50 тис. рядків/бекенд | те саме, що bebop, з IR-шаром | підтверджує: індексний IR + прямий емітер = робочий self-host | приклад на користь §3 |
| TCC | однопрохідний C → машинний код, value stack (`vtop/vpop`) = bebop до сьогодні | ~30 тис. рядків C | bebop-до-регістрової-моделі = TCC-схема | TCC лишився 1.5-3x повільніший за gcc -O2 — ціна відсутності SROA/інлайну | попередження для реальних програм, не для кернелів |
| LuaJIT trace IR | байткод за 1 прохід, IR для гарячих трас | ~50 тис. рядків | — | JIT — не про bebop (AOT, self-host) | ні |

## 3. Що в bebop уже є графом, і ескіз плоского IR на функцію

**Графи, що вже є у репо** (плоскі масиви індексів, без вказівників):

- Стор: CSR (`selfhost/std/csr.bp`: `rp[n+1]` row pointers, `ci[nnz]` column indices, `vv[nnz]`
  weights; `sgraph.bp`: 1M вузлів / 10M ребер як `RP{arr}` + `CI{arr}` у сторі, BFS/nbr, G8/T117).
  Ребро = позиція в `ci`, вузол = діапазон `rp[v]..rp[v+1]`.
- Компілятор: `fntab` (`[cnt, names, offsets, srcpos]` + enum-зона + ft_cache + факти `1500+i` +
  вікно `2000+3i`), `stab` — трійки `(name, reg, srcpos)`, `insns[]`, `starts[]/sizes[]`.
- Вікно операндів: список тегів `kind/p0/p1` по 3 комірки — вироджений IR-рядок (`REG r,p` тримає
  індекс слова-виробника — ребро def→use).

**Ескіз per-fn плоского IR у поверхні bebop** (форма CSR):

```
// ir[4*k .. 4*k+3] = {op, a, b, aux}; a,b = індекси рядків або -1-ci для констант cpool[];
// aux = imm / cond / slot. Блоки = діапазони: blk[2*j] = перший рядок, blk[2*j+1] = термінатор.
// Символ v після SSA-конверсії = останній рядок, що його визначив; phi лише на входах while /
// join if — по одному рядку на живий символ (як у QBE).
// Проходи (кожен — один while по рядках, O(n)):
//   fold   op(const,const) -> const                      (є на тегах уже)
//   lin    LIN-композиція рекурентностей §1b(4)           (є на тегах уже, якщо зроблено там)
//   cse    hash(op,a,b,aux) -> попередній рядок            (таблиця 4096)
//   sroa   cell c := ldr/str [sym,#0] без escape -> значення з phi (escape-скан = loop_alloc_safe)
//   inline callee <= 8 рядків без циклів -> копія рядків зі зсувом індексів
//   tre    `f(x) = g(f(x-1), ...)` з асоціативним g -> цикл з акумулятором (K2H)
//   dce    лічильник uses, зворотний прохід
//   emit   той самий емітер регістрової моделі, вхід = рядки, не текст
```

Оцінка: ~600-900 рядків bebop.bp (SSA-конверсія ~200, проходи по 60-120, емісія з рядків ~200), один
масив на функцію в арені. Планувальна/емісійна проходки B1 зливаються (розмір fn відомий з рядків) —
мінус ~150 рядків і мінус клас багів «дві проходки не зійшлися». Розблоковує: K2H → ~1.0x (tre),
**самокомпіляція 15-30 %** (SROA + inline + CSE; оцінка, міряти диференційно: pcprof під proot
ненадійний). На K1H/K3H/K4/K8H IR не потрібен — §1b(4) робиться на тегах.

## 4. Рекомендація (ранжовано за оптимізацією на нуль доданих залежностей; measured-first за D11/D14)

1. **Дозавершити регістрову модель** (у польоті): K1H/K4 → 1.0x, K3H → ≤1.2x, K2H → ~1.6x, K8H →
   ~1.4x. 80 % доступного виграшу на кернелах.
2. **T52 `csel` на тегах**: K8H → ~1.0-1.2x. ~60 рядків.
3. **§1b(4): складання лінійної рекурентності на тегах (тег LIN, unroll ×2/×4)** — measured-first:
   близнюк уже є (K1H/K3H/K4, Rust -O3 цього не робить, §5.3); гейт `k1h_ms <= 0.5 x Rust`,
   `k4_ms <= 0.6 x Rust`, паритет bpref на std_tests (LCG-генератори). ~300 рядків. Це єдиний
   доведений шлях **обігнати Rust на K1H-K4** (1.8-3x), і він існує лише тому, що мова має
   wraparound-семантику й компілятор без залежностей може змінити політику циклів одним комітом.
4. **§6 top-1: «specialise-then-run» twin pair** (measured-first, код лише після twin+gate):
   bebop генерує й компілює скан під конкретну схему (50 ms) проти Rust-generic скану з runtime-
   схемою; гейт — ms до результату включно з компіляцією. Очікування 5-30x (§6d).
5. **Хостинг 64-біт констант із тіл `while`** (S): K8H −8 слів/ітер. Робити лише якщо після 1-2 K8H > 1.2x.
6. **NEON `scan` builtin** для парсерних циклів — measured-first: спершу диференційний вимір K5
   (замінити один `skip_ws` вручну); поріг 10 %.
7. **B4** (фрейм за фактами): TRAP-82 = 0, RSS рекурсії; 0 ms.
8. **Плоский per-fn IR (§3)** — лише під самокомпіляцію після виміру SROA/inline вручну в одному
   гарячому циклі; поріг 15 %; форма CSR, рівно QBE-список проходів + tre.
9. **Salsa-подібний per-fn memo** — відкласти.

**Відхилити**: LLVM/Cranelift/QBE як залежності; e-graph mid-end; планувальник інструкцій і
software pipelining (A78 OoO); unrolling без алгебри; авто-векторизація (потрібен IR + залежності,
NEON без 64-біт mul); trace-JIT.

## 5. Докази

5.1 Поточний K4 під HEAD bebop.bin (objdump; loop = 14 слів):
```
2c: mov x0,x20 ; lsl x1,x0,#3 ; sub x0,x1,x0     (i*7 = mulc_try 2^3-1)
38: mov x1,x0 ; mov x0,x19 ; add x0,x0,x1          (два mov на ланцюжку v)
44: add x0,x0,x0,lsl #1 ; sub x0,x0,#0xb ; mov x19,x0
50: mov x0,x20 ; sub x0,x0,#1 ; mov x20,x0
5c: cmp x20,#0 ; b.gt 0x2c                          (B5: bottom test)
```
K1H: 10 слів, `mov x0,x19 ; add-lsl ; mov x1,x20 ; add ; mov x19,x0` — два зайві `mov` на шляху.
K3H: 24 слова, `sub sp/str/ldr/add sp` ×2 = 8 слів, два str→ldr ланцюжки. K2H fib: 25 слів, один
push/pop навколо другого `bl`, два `bl` на активацію.

5.2 Поточний K8H (38 слів у циклі): `mov x0,#0x7f2d ; movk ×3` і `mov x0,#0x814f ; movk ×3` —
8 слів інваріантних констант LCG щоітерації; `mul ; sub sp ; str ; ... ; ldr ; add sp ; add`;
`lsr x0,x0,x1` через `mov x1,#0x3c`; `cmp x22,#1 ; b.ne` — гілка ~50 % mispredict (K8 control:
0.15 ms з передбачуваним бітом проти 0.31-0.34).

5.3 Rust-близнюки, зібрані тут (`rustc 1.96.1 -O`, honest.sh:19; `-O` = `opt-level=3`, overflow-checks
off, thin-local LTO, 16 codegen units, target-cpu = базовий armv8 — doc.rust-lang.org/rustc/codegen-options,
завантажено), дизасембльовано `objdump -d`:
```
k4h  8d60: add x14,x20,x12 ; sub x13,x13,#1 ; sub x12,x12,#7 ; add x14,x14,x14,lsl #1 ; cmp x13,#1 ;
           sub x20,x14,#0xb ; b.gt        -- i*7 як running sub; ланцюжок v: add, add-lsl, sub = 4 т
k1h  8d58: add x12,x13,x13,lsl #1 ; sub x11,x11,#1 ; cmp x11,#1 ; add x13,x11,x12 ; b.gt   -- 3 т
k3h  8d58: add x13,x20,x12 ; subs x12,x12,#1 ; add x13,x13,x13,lsl #1 ; add x20,x13,x11 ; b.gt
           -- (a+y)*3 + 2x: реасоціація в межах ітерації, lsl x11,x10,#1 hoisted (LICM)
k8h  8d94: madd x14,x14,x12,x13 ; sub x16 ; tst x14,#0x1000000000000000 ; csel x17,x15,x14,eq ;
           cmp ; add x15 ; add x20,x17,x20 ; b.gt   -- константи x12/x13 hoisted, csel без гілки
k2h  fib:  stp x29,x30 ; stp x20,x19 ; mov x29,sp ; cmp x0,#2 ; b.ge ; ... ;
           loop: sub x0,#1 ; bl fib ; mov x8,x0 ; cmp x20,#3 ; sub x0,x20,#2 ; add x19,x8,x19 ; b.hi loop
           -- другий виклик fib(n-2) перетворено на цикл: ОДНЕ bl на активацію (тому 4.5 т/виклик)
```
Жоден близнюк не розгорнутий і не складає рекурентність за k ітерацій. Близнюки міряють in-process
(`Instant`, reps=100) — старт процесу в ratio не входить. Масивів у близнюках немає → bounds/overflow
перевірок немає; `libgcc_s` злінковано (unwind), `panic=unwind` за замовчуванням.

5.4 Старт процесу під proot (медіана 15 запусків): Rust `k0` (println одного числа, 4.3 MB, NEEDED
libc.so.6 + libgcc_s.so.1) **16.3 ms**; bebop `seed c01.bin` **6.8 ms**; `bebop.bin version` 7.8 ms.
Компіляція: `rustc -O k4h.rs` (14 рядків) **1.34 s**, `bebop compile k4.bp` **0.05 s** (27x); bebop
самокомпіляція 230 KB джерела **1.53 s** (холодний .becache).

5.5 Репозиторій: bench/tq_sqlite/RESULT.md (sqlite scan 183.2 ms python / ~158 ms native проти
bebop nn.bp 18.4 ms = 8.6-9.9x; nn4.bp 1 A78 → 3 A78 через `sys_setaffinity`: 219 → 99 ms = 2.21x);
docs/LANG-DB-DESIGN.md:19-20 («100x is reachable on the scan class only (Rust-quality codegen: 112x
measured), never on DRAM-resident random point lookups»; BFS проти sqlite recursive CTE 15-40 ns проти
~1.5 us/ребро, гейт G8); HISTORY.md:1718 (T45: clone/futex/LSE builtins портовано, pool_parity 5/5),
HISTORY.md кінець («K1-K4 6-12x faster than Rust» відкликано: «today 1.2-5.8x slower»);
docs/DECISIONS-RESEARCH-2026-09-06.md §3.3 B3 («K4 is latency-bound on add;add-shifted;sub and will
move little in ms» — підтверджено §0а); bench/vs_rust/REPORT-honest.md:74-82; selfhost/std/csr.bp:1-12,
sgraph.bp:3-13; tools/chain.sh:23; /proc/cpuinfo; bebop.bp `cache_hit/cache_write`.

5.6 Зовнішнє: QBE — https://c9x.me/compile/ (завантажено). rustc codegen options —
https://doc.rust-lang.org/rustc/codegen-options/index.html (завантажено: `-O` = opt-level 3;
overflow-checks вимкнені без debug-assertions; codegen-units 16; thin-local LTO; target-cpu базовий).
Mutable `noalias` увімкнено за замовчуванням з Rust 1.54 (PR https://github.com/rust-lang/rust/pull/82834
«Enable mutable noalias for LLVM >= 12»; issue https://github.com/rust-lang/rust/issues/54878;
https://users.rust-lang.org/t/rust-1-54-0-is-here/62927). LuaJIT SSA IR 2.0 — wiki.luajit.org/SSA-IR-2.0
(не завантажено). Cranelift (CLIF/ISLE/aegraph/regalloc2), Go SSA `.rules`, Zig self-hosted, TCC —
з пам'яті, у вебі не перевірялись.

## 6. Прорив: як перегнати Rust на порядок — дослідити сам Rust

**Чесно спочатку.** На K1H-K4 (чисті скалярні рекурентності) 10x фізично неможливо: обидва
компілятори емітять той самий ланцюжок на тому самому кремнії; HISTORY.md уже відкликав «K1-K4 6-12x
faster than Rust». Максимум на цьому класі — §1b(4): 1.8-3x, і то лише тому, що LLVM не складає
рекурентність (§5.3). Порядок величини треба шукати там, де Rust платить за універсальність.

**(a) Що насправді платять honest-близнюки.** `rustc -O` = opt-level 3, overflow-checks off, thin-local
LTO, 16 CGU, `panic=unwind` + `libgcc_s`, target-cpu базовий (без `crc`, без LSE → атомарні операції
через ldxr/stxr, crc32 програмний — це важливо для crc/atomic-близнюків, яких ще немає). У K1H-K8H
масивів немає → bounds/overflow-перевірок нуль; unwind-таблиці є, але на гарячий шлях не впливають;
пролог fib 3 слова + `mov x29`, епілог 2 (проти bebop 4+4 із 16 KiB фреймом). Вимір in-process →
ratio чесний і НЕ містить старту процесу. Висновок: на кернелах Rust не платить нічого зайвого;
різниця 1.4-4.5x сьогодні — це bebop-стек-машина, і вона зникає з регістровою моделлю.

**(b) Фіксовані витрати Rust, яких у bebop немає** (виміряно §5.4 або з документації):
- старт процесу під proot: 16.3 ms проти 6.8 ms (2.4x): динамічний лінк libc+libgcc_s, 4.3 MB
  бінарник проти 652 B, std-ініціалізація; проот робить кожен syscall дорогим (ptrace) — bebop
  робить 5 syscall'ів на старт;
- компіляція: 1.34 s на 14 рядків проти 0.05 s (27x); 5.3k рядків bebop за 1.53 s — rustc такого
  обсягу з -O компілює десятки секунд (не міряно тут; клас порядку величини — K5 уже це фіксує);
- malloc + page faults проти mmap-арени/frame heap: у кернелах нуль, у алокаційно-важких програмах
  (парсери, побудова графів) ідіоматичний Rust платить malloc/free + CoW faults 3.5 us/сторінку
  (LANG-DB-DESIGN §4); Rust із bump-алокатором — паритет;
- мономорфізація + 16 CGU: cross-fn fusion лише в межах CGU/через thin LTO; bebop компілює всю
  програму як один юніт (але й не інлайнить — §3);
- `noalias` (з 1.54, PR #82834): LLVM отримує alias-факти від borrow checker'а — перевага Rust над
  bebop у будь-якому майбутньому IR; bebop-комірки `[i64]` можуть аліасити, SROA потребує escape-скану.

**(c) Де репо вже міряє порядок величини і чому.** K6: 18.4 ms проти sqlite 158-183 ms (8.6-9.9x) —
без VDBE/декодування записів, прямий cell-layout, і LANG-DB-DESIGN §5 називає стелю: «100x on the scan
class only (Rust-quality codegen: 112x measured)», тобто **близнюк тут — sqlite, не Rust**; Rust-scan
над тим самим SoA-масивом дав би ~1x. BFS проти recursive CTE: 15-40 ns проти ~1.5 us/ребро (40-100x) —
знову проти SQL-рушія. Ядра: nn4.bp 1→3 A78 = 2.21x усередині процесу (`sys_setaffinity`, clone/futex
builtins) — Rust `std::thread` робить те саме, паритет. K5: 27x на компіляції — проти rustc, справжній
Rust-порядок.

**(d) Кандидати на 10x проти Rust** (механізм, чому Rust повільний, чим bebop його уникає, що міряти):

| # | домен | чому Rust повільний | bebop | очікуване ratio | сила доказів | twin, який треба зробити |
|---|---|---|---|---|---|---|
| 1 | compile-then-run / specialise-then-run (скан або запит під конкретну схему, форму таблиці, константи) | rustc-латентність 1.3 s мінімум → спеціалізація per-query неможлива; generic код тримає stride/схему в runtime | 50 ms на компіляцію кернела, константи запечено в immediates, `.becache` для повторів | **5-30x на latency-to-result; 1.5-3x на самому скані** | сильна для латентності (§5.4), відсутня для скану | Rust generic scan (runtime schema, 1M рядків) проти bebop-згенерованого скану; гейт: ms до результату включно з компіляцією; другий рядок — без компіляції |
| 2 | лінійні рекурентності / LCG / хеші | LLVM не складає рекурентність за k ітерацій (§5.3) | тег LIN (§1b) | **1.8-3x** на K1H/K3H/K4 | сильна (дизасемблювання) | близнюки є; гейт k1h ≤ 0.5x, k4 ≤ 0.6x Rust |
| 3 | алокаційно-важкі програми (парсери, побудова CSR, JSON-подібне) | ідіоматичний Rust: malloc/free, `Vec` перевиділення, CoW faults | арена + frame heap, нуль free | 2-5x проти ідіоматичного; ~1x проти bump-алокатора | слабка (twin немає) | Rust `serde_json`-подібний парсер vs bebop-парсер над 10 MB; чесний twin — Rust з `bumpalo` |
| 4 | NEON-builtin там, де LLVM не векторизує | popcount-редукції LLVM векторизує сам (`cnt`+`addv`); crc32 — без `-C target-feature=+crc` програмний (табличний ~1 B/т) | `hvham` NEON, `crc32` апаратний 8 B/т | crc: 5-8x проти generic-target Rust, ~1x проти `+crc`; hamming ~1-2x | середня (T109 ~1 cell/т) | crc32 над 1 MB: Rust generic vs `+crc` vs bebop; звітувати обидва |
| 5 | in-process multi-core без планувальника | `std::thread` = clone + futex так само | clone/futex/atomic builtins | ~1x | сильна (паритет) | не робити |
| 6 | syscall-free гарячі шляхи під proot | proot робить syscall ~10-30 us; Rust std робить їх більше (алокатор, println буфери, stdio locks) | mmap-арена, буферизований sys_write | 2x на старті, 1x у циклах | середня (§5.4) | лише як рядок у REPORT-honest |
| 7 | data layout (CSR, cells, без pointer chasing) | Rust *може* так само (SoA, індекси) | так за замовчуванням | 1x проти тюнінгованого Rust; 5-10x проти ідіоматичних `Vec<Node>`/`Box` | середня (sgraph vs CTE, не vs Rust) | BFS 1M/10M: Rust `petgraph` (ідіоматичний) vs Rust CSR vs bebop sgraph |
| 8 | **zero-copy parsing над mmap** (заявка оператора: 50-100x на важких текстових/бінарних потоках) | `serde_json` в owned DOM: malloc на кожне поле/рядок, UTF-8 валідація, копії; Python `json` — ще й об'єктна купа. АЛЕ Rust уміє zero-copy: `memmap2` + `winnow`/`nom` над `&[u8]`, `simd-json` borrowed, `rkyv` (LANG-DB-DESIGN w15) — проєкція на байти без алокацій | уже є раба-байт шлях: `str` = raw pointer, `char(s,i)` = zero-extended byte load (bebop.bp:1466), `sys_readbuf` (raw read у IO-зону), `sys_mmap` (5462), літерали через `adr` у data-секції; **чесно: `sys_read`/`sys_slurp`/`str_to_cells`/`crc32` — bytes-in-cells, 1 байт на i64-комірку = 8x пам'яті й пропускної** (sys_slurp: `round16(len*8)`), тож zero-copy парсер у bebop мусить іти raw-шляхом, не cells-шляхом; найближчий вимір у репо — K6 nnidx `ldr` in place (9.9x проти sqlite VDBE-декодування) і LANG-DB-DESIGN:19 «zero-copy is worth 10-100x against serde/JSON and 1.0x against sqlite\'s page cache on bytes moved» | **~1x проти найкращого Rust** (memmap2 + winnow/simd-json borrowed: ті самі байти, той самий `ldrb`/NEON-скан, у Rust ще й NEON авто-векторизація сканування); 3-10x проти звичайного (`serde_json` → owned структури зі `String`); 50-100x лише проти Python/JS DOM-парсерів — заявка не підтверджується проти Rust | середня для «vs owned DOM» (LANG-DB §5 M1), відсутня для «vs Rust zero-copy» | twin: 100 MB line-oriented записів (int-поля, рядки) через mmap: bebop raw-byte парсер (char/скан-builtin §1b(3)) vs Rust `memmap2`+`winnow` (best) vs `serde_json` owned (common) vs Python `json` (контекст); гейт: ms/MB, maxrss; звітувати всі три рядки |
| 9 | **zero allocation, no GC** (заявка оператора: 5-20x на рутинних операціях) | GC-рантайми (Go/JS/Python): write barriers, GC-паузи, заголовки об'єктів; Rust: без GC, `malloc/free` (system або jemalloc) ~20-50 ns на виклик + перевиділення `Vec`, CoW page faults 3.5 us/сторінку на першому дотику (LANG-DB §4) — і арени доступні (`bumpalo`, `typed-arena`, `Vec::with_capacity`) | арена x27/x28 (bump, `zeros`) + frame heap x14 (bump на активацію, T43 reset у циклах) — рівно «один інкремент вказівника»; **чесно: арена ніколи не звільняється** (exit 80 на кінці арени; повторне використання лише через store-компакцію або вихід процесу), frame heap 16 KiB/активацію, і кожен перший дотик сторінки — той самий page fault, що і в Rust | **~1-1.3x проти найкращого Rust** (bumpalo / with_capacity: той самий bump + ті самі faults); 2-5x проти звичайного Rust (`Vec<Vec<T>>`, `Box` на вузол, `String` на поле) на алокаційно-важких шляхах; 5-20x правдоподібно лише проти GC-рантаймів — клас порівняння треба називати явно | середня: репо має nnidx build (zeros + counting sort, не хронометровано окремо, tq_sqlite RESULT:4) і фізику faults (LANG-DB §4), але жодного Rust-близнюка | twin: побудова 1M дрібних записів у CSR (10M ребер): bebop `zeros` + counting sort vs Rust `Vec<Vec<u32>>` push (common) vs Rust двопрохідний CSR `with_capacity` (best) vs `bumpalo`; гейт: ms, maxrss, minor faults (python `resource`), три рядки в одному звіті |

Ранжування за очікуваним ratio × силою доказів (проти НАЙКРАЩОЇ Rust-формулювання, бо саме її міряє
honest.sh): **1 (specialise-then-run, 5-30x на latency-to-result)** і **2 (LIN на тегах, 1.8-3x)** —
обидва в §4 як measured-first; 8 (zero-copy над mmap) і 9 (zero allocation) — **~1x проти найкращого
Rust**, 3-10x / 2-5x проти звичайного Rust, 50-100x / 5-20x лише проти Python/JS/GC-рантаймів: заявлені
оператором цифри не підтверджуються проти Rust і мають звітуватись трьома рядками (best / common /
GC-мова) в одному twin-звіті, інакше це повтор історії «K1-K4 6-12x faster than Rust»; 3 і 7 — той самий
клас «10x проти ідіоматичного, 1x проти тюнінгованого»; 4 — залежить від target-feature близнюка; 5-6 —
паритет. Порядок вимірювання: twin 1 → twin 2 → один спільний twin для 8+9 (парсер, що будує CSR із
mmap-файлу: обидві заявки в одному вимірі, три Rust-рядки).

**(e) Що стратегічно дає відсутність залежностей.** Політика кодогенерації Rust зафіксована LLVM:
ротацію циклу, вибір `csel`, unroll-евристики, складання рекурентностей змінити з Rust-програми не
можна — це коміт у 10-мільйонний проєкт із багатогодинним bootstrap'ом і чужим review. У bebop за одну
сесію: B5 (ротація циклів для всіх ~200 while-сайтів: ~100 рядків, один коміт, chain 2-3 хв), регістрова
модель (заміна всієї моделі значень: один коміт, ~1000 рядків diff, gate той самий chain). Ціна зміни
політики — хвилини на fixpoint і той самий battery; це і є актив, який робить §1b(4) і §6d-1
досяжними взагалі.

## 7. Bash у tooling bebop: чим замінити — власним рішенням чи наявним

**(a) Інвентар (read-only, 2026-09-06).** tools/: 16 sh (526 рядків) + 11 py (1 887 рядків); bench/:
23 sh (2 053) + 124 py (6 257). Разом **39 shell-скриптів / 2 579 рядків і 135 python-файлів / 8 144
рядки** — python уже є прийнятою залежністю репо, і вона в 3x більша за bash-шар (perf.py 362, check_abi.py
261, census, check_words, fuzz_batch, pre-commit/journal linter). Що роблять sh: chain.sh (45 рядків:
gen2→gen3→gen4 + battery + perf), battery.sh (45: 8 лейнів паралельно, підсумок через `grep -E | tail`),
std_golden.sh (769: 99 тестів, 109 викликів `seed`), construct_parity.sh (136), invariants.sh (63),
reap.sh (31, proc-cap gate), std_par.sh (J=3 шарди), fuzz.sh/fuzzd.sh, honest.sh/bench_pinned.sh
(хронометраж), решта — one-shot гейти. Зовнішні команди у всіх sh (входжень у тексті): `seed` 412,
`tail` 140, `python3` 83, `grep` 50, `awk` 43, `taskset` 36, `date` 27, `timeout` 22, `cut` 21,
`md5sum` 20, `sort` 15, `sed` 12. Структуровані дані справді сплющуються в текст: кожен лейн друкує
рядок-підсумок, battery.sh витягує його `grep -E`-ом і звіряє regex-ом (`line()`), perf.py читає csv.

**(b) Реальна ціна, виміряна під proot (ptrace на кожен syscall).** Мікробенчмарк ×200 послідовно:

| примітив | ms на виклик |
|---|---|
| bash `$(echo)` (fork без exec) | 2.1 |
| bash → `/bin/true` (fork+exec, dyn libc) | 8.5 |
| python `subprocess.run(['/bin/true'])` | 8.0 |
| `seed c01.bin` (bebop-процес: статичний, 5 syscall'ів на старт) | 4.3 |
| `grep -c` на малому файлі | 21 (динамічний лінк + ugrep) |
| `python3 -c pass` | 85 |

Chain (perf.csv, 2026-09-06): `chain_wall` 18 s (CG=1) / 38 s (CG=0 + повний battery), `chain_cpu`
20-50 s, `gate_run_ms` 37 085 ms = сума 149 не-memo прогонів (≈250 ms кожен, з них ~10 ms spawn), або
0 при 100-200 memo-hit'ах; лейн perf.py +60-70 s (хронометраж кернелів, не оркестрація). Оцінка кількості
spawn'ів на один chain із скриптів: std_golden 99 тестів × (compile + run + порівняння + 3-6 `$(…)`/`grep`/
`md5sum`) ≈ 600-900; construct_parity 52 × ~6 ≈ 300; parity/pool/diag ≈ 150; python-старти 6-10 (×85 ms);
`taskset`/`timeout` обгортки подвоюють exec на кожен запуск → **≈1 500-2 500 fork/exec на chain**, по
4-9 ms → **≈10-18 s CPU, ≈4-6 s wall на 3 ядрах**.

**(c) Частка оркестрації.** З ~30 s battery-wall: реальна робота (compile+run 149 гейтів) ≈ 37 s CPU /
3 ядра ≈ 12 s wall; spawn+текст ≈ 4-6 s wall; решта — послідовні хвости лейнів (inv.sh закінчує останнім,
+10 s) і python-старти (~1 s). **Стеля будь-якої заміни bash — 15-25 % battery-wall і 0 % на gen-компіляціях
(3 × 1.5 s) та на perf-лейні (60-70 s чистого хронометражу).** При повному memo (gate_run_ms = 0) частка
оркестрації зростає до ~50 %, але абсолютно це ті самі 5-8 s. Заміна, що спавнить ті самі процеси (just,
nu, osh), не зменшує нічого: spawn — це ptrace-ціна proot, а не bash.

**(d) Що bebop уже має для варіанту (4) і чого бракує.** Є (`grep '^fn emit_sys_' bebop.bp`): open, read,
write, close, exit, clock_ms, arena_base/end, **clone (сирий: flags + stack_top — `sys_clone(17, 0)` =
fork із SIGCHLD, тобто fork процесу вже є)**, cond_set, futex_wait_guard/wake, atomic_add,
exit_thread_guard, setaffinity, msync, readbuf, slurp, ftruncate, munmap, export, mmap, rename; argv —
контракт M4 seed'а (argc/argv скопійовано в арену), envp — ні; `sha256_words` (cas_verify) для хешів.
Бракує для оркестрації (кожен — за шаблоном `emit_sys_*`: 8-20 ручних слів + 15-30 рядків bebop.bp +
рядок у words.objdump + allowlist check_abi + заглушка bpref + конструкт-гейт ≈ 40-60 рядків на builtin):

| builtin | syscall | слів | навіщо |
|---|---|---|---|
| `sys_execve(path, argv, envp)` | 221 | ~10 | запуск seed/python3 |
| `sys_wait4(pid, status)` | 260 | ~10 | rc/сигнал дитини (SIGILL/SIGBUS тестів) |
| `sys_pipe2(fds)` + `sys_dup3(a,b)` | 59, 24 | ~8+6 | захоплення stdout лейну без tmp-файлів (або писати у файли — уже можна) |
| `sys_getdents64(fd, buf)` | 61 | ~10 | перелік std_tests/ (або статичний список у .bp) |
| `sys_fstat/statx` | 80/291 | ~10 | розмір/mtime для memo (є обхід: sys_slurp + порівняння) |
| `sys_kill(pid, sig)` | 129 | ~6 | timeout-и без `timeout(1)` |
| `sys_unlinkat/mkdirat` | 35/34 | ~8 | tmp-дерева |
| `run(bin)` — mmap PROT_EXEC + `blr` | mmap є | ~15 | **виконати .bin у тому ж процесі без fork/exec** — єдина річ, що прибирає spawn взагалі (потрібен fork для ізоляції краху тесту) |

Разом ≈ 8 builtins ≈ 350-450 рядків bebop.bp + ~80 слів. Ескіз `chain.bp` (те, що робить 45-рядковий
chain.sh): `gen(bin, src, out)` = fork + execve seed (або **in-process: `cli_compile` — це функція самого
bebop.bp, gen2/gen3/gen4 можна викликати без жодного процесу, якщо chain.bp злінкований з компілятором**
через `use`); fixpoint = `sys_slurp` обох .bin + поцільне порівняння (md5 не потрібен); battery fan-out =
3 `sys_clone`-потоки (по A78-ядру, `sys_setaffinity`), кожен fork+exec'ає лейни або, для std_golden, компілює
й запускає тести in-process через `run()` під fork-ізоляцією; perf.py/census/check_abi/check_words лишаються
python (8 k рядків ніхто не переписує) → 4-6 execve python3 (0.5 s); reap-gate = читання /proc через
getdents+read (або лишити `reap.sh --check` як один exec). Оцінка **150-250 рядків chain.bp + 300-500
рядків std-runner**; правила, що лишаються: proc-cap (тепер точний: chain.bp знає своїх дітей), reap після
задач (менше сиріт, бо wait4 свій), pkill-пастка зникає (kill по pid). Що ламається: memo `BEBOP_MEMO`
(треба переписати), словесні підсумки лейнів (battery.log — контракт для агентів і людей; лишити текст),
кожен лейн — це sh зі своїм regex-контрактом → переписувати всі 39 скриптів або обгортати їх execve'ом
(тоді виграш = 0).

**(e) Залежності.** `command -v just nu osh` — жодного; apt (Ubuntu ports arm64 у цьому proot): `just`
1.45.0-1 доступний, `nushell`/`oils-for-unix` — відсутні; Termux `pkg` з цього proot недосяжний. Усі три —
Rust/Python-екосистемні бінарники: just — Rust, «just a command runner» (github.com/casey/just, завантажено;
кожен рядок рецепта виконується `sh -c`, тобто spawn той самий + ще один `sh`; залежності між рецептами є,
паралелізм — лише атрибутом `[parallel]` на залежностях у нових версіях, без `-j` для довільного DAG — з
пам'яті, не перевірено); Nushell — Rust, «external programs are spawned as separate processes, their output
appears as text by default», «does not execute bash scripts natively» (nushell.sh/book, завантажено) → повна
міграція 2.6 k рядків; Oils/osh — bash-сумісний парсер + YSH зі структурованими даними, Python→C++
транслятор, збірка з tarball (oils.pub повернув 403; з пам'яті). AGENTS.md: нуль залежностей для
компілятора; для tooling репо вже прийняв python (8 k рядків) і coreutils/ugrep/objdump/as — це треба
сказати чесно: питання не «залежність чи ні», а «ще одна, заради чого».

**(f) Матриця рішень** (overhead removed з (c); рядки міграції з (a); ризик — battery = safety net, історія
T77/T96 показала, що баг harness'а коштує днів):

| варіант | прибирає overhead | нові залежності | self-host чистота | міграція, рядків | ризик для гейтів | вердикт |
|---|---|---|---|---|---|---|
| bash (статус-кво) | 0 | 0 | tooling ≠ компілятор; ок | 0 | 0 | **лишити зараз** |
| just | 0 (ті самі spawn'и + `sh -c`) | 1 (apt є) | ні | ~200 (Justfile-обгортка) | низький, але виграш 0 | відхилити |
| Nushell | ≤5 % (менше `grep/awk/cut` через таблиці; spawn'и ті самі) | 1 (не в apt) | ні | 2 600 (переписати все) | **високий** (кожен regex-контракт) | відхилити |
| osh/YSH | 0-3 % | 1 (не в apt, збірка) | ні | 0 для osh (сумісний) → тоді й виграш 0 | середній (сумісність 39 скриптів) | відхилити |
| python (уже є) для текстових шматків | ≤5 % (85 ms на старт з'їдає виграш, якщо не один процес на лейн) | 0 нових | ні | ~300 (battery.sh `line()` + підсумки) | низький | лише як заміна `awk`-сміття, не мета |
| bebop-native, повний (chain.bp + усі лейни) | 15-25 % battery (стеля (c)); до ~40 % з in-process `run()` | 0 (dogfooding) | **так** — і це єдиний аргумент | 800-1 200 (builtins + chain.bp + runner) + переписати/обгорнути 39 скриптів | **високий**: fixpoint-критичний harness переписується мовою, чий компілятор він гейтить (циклічна довіра: зламаний компілятор ламає свій же гейт — потрібен frozen golden-runner, як golden bebop-f86bee7.bin) | не зараз |
| bebop-native, лише std-runner (in-process compile+run 99 тестів під fork-ізоляцією, викликаний з battery.sh як один лейн) | 10-20 % battery (найбільший лейн: ~700 spawn'ів → ~100) | 0 | так, локально | ~350 builtins + ~300 runner | середній (один лейн, звіряється з std_golden.sh байт-у-байт паралельно, старий лейн лишається як oracle до 3 зелених chain'ів) | **кандидат після виміру** |

**Рекомендація (ранжовано).** 1) Bash лишається оркестратором: 45+45 рядків chain/battery — це не гальмо,
гальмо — proot-spawn (4-9 ms) і хронометраж. 2) **Перший крок, без коду (measured-first):** один
інструментований chain на вільній коробці — `strace -f -e trace=execve -c` (або `bash -x` + підрахунок)
для точної кількості exec'ів і `perf.py`-рядки `chain_spawns`, `chain_spawn_ms`; гейт рішення: якщо
spawn+текст < 20 % battery-wall — питання закрите до зміни платформи (без proot spawn ≈ 0.5 ms і частка
падає нижче 5 %). 3) Якщо > 20 %: bebop-native **std-runner як один лейн** (builtins execve/wait4/pipe2/
kill + `run()`; runner звіряється зі std_golden.sh байт-у-байт три chain'и поспіль, старий лейн — oracle);
це і dogfooding, і єдине місце, де spawn'и реально зникають. 4) Повний chain.bp — лише після (3) і після
рішення про golden-runner (frozen bin для гейту, щоб компілятор не гейтив себе своїм же зламаним runner'ом).
5) just/Nushell/osh — відхилити: залежність без усунення жодного spawn'а; python для текстових підсумків —
лише як прибирання `awk`-шматків усередині наявних лейнів, не як міграція.

VERDICT: top = дозавершити регістрову модель + T52 csel + тег LIN (§1b(4): складання лінійної рекурентності на тегах, ~300 рядків,
1.8-3x над Rust -O3 на K1H/K3H/K4 — LLVM цього не робить, дизасемблювання §5.3); очікувані відношення до Rust після регістрової
моделі: K4 1.0x, K1H 1.0-1.05x, K3H 0.6-1.2x, K2H 1.5-1.8x, K8H 1.3-1.6x → 1.0-1.2x із csel; після LIN: K1H ~0.35x, K4 ~0.55x,
K3H ~0.3-0.5x; порядок величини проти Rust — лише в класі specialise-then-run (5-30x на latency-to-result, twin спершу); zero-copy над mmap і
zero-allocation (заявки 50-100x / 5-20x) — ~1x проти найкращого Rust (memmap2+winnow, bumpalo), 2-10x проти звичайного, порядок лише проти
Python/JS/GC — міряти одним спільним twin (парсер → CSR з mmap-файлу, три Rust-рядки) до будь-якої заявки; плоский per-fn IR (CSR-форма, QBE-список +
tre) — лише під самокомпіляцію після виміру (поріг 15 %), ~600-900 рядків; LLVM/Cranelift/QBE як залежності — відхилити.; §7: bash лишається оркестратором (стеля заміни 15-25 % battery-wall, spawn = proot-ptrace 4-9 ms, не bash), перший крок = інструментований підрахунок exec'ів + перф-рядки chain_spawns (гейт 20 %), потім bebop-native std-runner як один лейн (execve/wait4/pipe2/kill/run builtins, ~650 рядків, старий лейн як oracle); just/Nushell/osh — відхилити.
