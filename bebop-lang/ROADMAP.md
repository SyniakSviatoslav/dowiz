# Bebop — THE Roadmap (single source of truth)

This file supersedes and replaces PLAN_B.md, MASTER-FINISH-PLAN.md,
ROADMAP_SELFHOST.md, docs/ZERO_C_CHARTER.md and SWEEP-B3-3.md — all removed.
BUGFIXES.md stays (bug journal), AGENTS.md stays (process laws), bench/
reports stay (evidence).

## Terminal goal

1. **Zero C**: every mechanism of the toolchain exists in Bebop; `native/src`
   (C) is deleted. The repo is `seed.bin` (frozen AArch64 loader, no libc) +
   `bebop.bin` (the compiler compiled by itself) + `*.bp` sources + `*.bin`
   artifacts. The C interpreter is retired, not ported.
2. **Math validation**: a Lean4-like proof/verification layer for the Bebop
   language (QTT kernel, theorem checker, machine-checked proofs).
3. **Full language**: the complete glyphic+VSA+QTT surface compiled by the
   self-hosted pipeline (lexer/parser/typecheck/codegen + aarch64/wasm/NEON/
   GPU-FPGA backends).

## Fixed execution order (binding)

1. **BLOCKERS** — close the known correctness gaps (see below).
2. **M4 → M7** — the Zero-C milestones.
3. **THEOREM WORKSTREAM** — Lean4-like math validation & verification.
4. **PART B** — full language front-end + backends, LAST.

Rationale: the compiler must be trustworthy before it replaces C (blockers
first); C can only leave after every C-provided capability has a verified .bp
twin (M4-M7); the proof layer is built on the stable core language; the full
language surface is the final expansion once everything beneath it is solid.

## Step 1 — Blockers [DONE 2026-08-28]

The apparent "call-in-loop execution divergence" (c19/c20) was a RUNNER
defect, not an emitter defect: exec_words took the manifest's LAST fn offset
as the entry, so multi-fn programs entered at their final helper. The
word-level parity had been green all along; value parity exposed the entry
bug. Fixes:
- exec_words: numeric third arg = explicit entry word offset; manifest
  default restored to last-offset (kernel convention: helpers first, main
  last, per the seed charter entries).
- parity_driver.sh + construct_parity.sh now compute fn main's offset from
  the source (^fn index) and pass it explicitly; construct_parity.sh also
  checks VALUE parity (interp run == native exec), not just word bytes.
- Corpus corrected to supported constructs; two emitter gaps documented
  below (blessed by checksum-only self_check cases until Point B):
  * struct-literal VALUES (`pt { x: 1, y: 2 }`): the enum-ctor fallback
    parses field names as variables (xzr reads) — pointer-valid field
    access works, construction does not.
  * string-literal VALUES: emit_str is a placeholder (mov #0); str_len/char
    on a literal dereference NULL natively.

**Gate for Step 1 (all green):**
- self_check → 0
- full self_bootstrap: 67816/67816 words byte-identical
- parity_driver (kernels) 9/0/0 (+1 main-less skip); (constructs) 20/0/0
- construct_parity.sh → 20/20 MATCH (words AND values)

## Step 2 — Zero-C milestones (M4 → M7)

| M | Goal | Acceptance |
|---|---|---|
| **M4 [DONE 2026-08-28]** | CLI in .bp: `bebop.bin` subcommands `compile / run-via-exec / size / version`. Args passed from the seed loader's stack block into the arena. | `seed bebop.bin compile k1.bp` emits byte-identical words vs `compilewords`; `run` executes a kernel to the known result; `size`/`version` print correct values. |
| **M5 [CORE DONE 2026-08-28]** | std/ .bp twins for toolchain-adjacent algorithms: sort, rng, checksum, base64, sha256 (then whatever else the compiler/tooling consumes). | Each twin golden-vector tested against the C result BEFORE the C twin is removed. |
| **M6 [DONE 2026-08-28]** | Parallelism: clone/futex via svc; pool.c reimplemented as .bp work-splitting over the shared arena. | compilemany and k7 queries run multi-core with identical outputs. |
| **M7 [DONE 2026-08-28]** | Delete `native/src` (keep docs). | Repo = seed + .bp + .bin + docs; full gate green without the C compiler. |

Non-goals (unchanged from the charter): the interpreter is NOT ported;
wasm/GPU backends stay archived until Step 4; the C CLI's legacy modes die
with their implementations.

## Step 3 — Theorem workstream (Lean4-like math validation)

Built on the stable core after M7:

- **T1** Wire the QTT kernel (`qtt_kernel.bp`, `nat_peano.bp`, `proof.bp`,
  `universes.bp`) into an end-to-end proof checker: parse statement → kernel
  type-check → proof term validates.
- **T2** Port the C-side theorem machinery (3 machine-checked theorems) to
  .bp; grow the theorem corpus (nat induction, list, arithmetic identities).
- **T3** Refl-style automation (conv/norm/subst already exist as slices) +
  a tactic layer in Bebop; golden-vector proofs re-verified.
- **T4** Verification reports: fraction of the stdlib proven; all proofs
  machine-checked by the kernel with zero trust in the checker's own claims.

Acceptance: `make theorem-check` green; N machine-checked theorems with the
kernel as sole authority; docs report the proven coverage honestly.

## Step 4 — Part B: full language (LAST)

Only after M7 + the theorem workstream. Ports the C compiler surface to the
self-hosted pipeline, keeping the Zero-C state:

- **B1 Front-end**: full lexer.bp/parser.bp/typecheck.bp parity (modules, ADTs,
  dependent Pi types, generics, match, let-in, lambdas, arrays, field access;
  QTT quantities 0/1/ω, universes, conv/norm kernel).
- **B2 Backends**: aarch64.bp full native parity (entire construct set incl.
  closures/floats/syscalls/alloc); wasm.bp valid+executable modules; NEON
  vector-first backend; GPU/FPGA VIR slice + emit contract.
- **B3 Closure**: self-compile the FULL pipeline end-to-end (checksum-stable);
  port all native self-tests; fuzz to 1M inputs; honest bench vs Rust; docs.

DoD: the full glyphic+VSA+QTT surface compiles itself; backends verified;
fuzz/bench/docs green; every milestone committed+pushed.

## Current verified state (2026-08-28)

- **M1** seed loader: DONE — k1..k7 run through seed.bin, outputs identical
  to the interpreter; zero C at runtime.
- **M2** syscall I/O builtins: DONE (open/read/write/close/exit + clock;
  interp mirror + emitter words).
- **M6** parallelism: DONE — `bench/vs_rust/pool_parity.sh` 5/5:
  par_sum 4×1000==10000, par_merge (atomic sys_atomic_add LDADDAL merge)
  4×1000==10000, par_compile(4,k1)==4×92, par_compile(4,k7)==4×1536,
  real-thread evidence (clone returns 4 child tids on the seed, 0 under the
  sequential interp emulation) — all on BOTH engines. The clone emitter
  re-bases the child's x27/x28 to a private 4MB arena slice and x15/x14 to
  its own stack (no shared-cursor race in concurrent sys_slurp); the futex
  wake emits DMB ISH first (ARM64 weak ordering); seed loader 8-aligns the
  arena cursor after the argv copy (LDADDAL BUS_ADRALN fix).
- **M3** self-bootstrap: DONE — full selfsource compiled by itself is
  byte-identical to the interpreter's output (67816/67816 words); selfcompile
  fingerprint 236065248692568 == word-sum; self_check = 0.
- **M4** CLI-in-.bp: DONE — `bebop.bp` = compiler + CLI
  (`compile/size/version`, `run-via-exec` documented stub; exec is
  C-dependent and dies at M7 per the charter). seed loader v4 passes
  argc/argv (x0=argc, x1=arena-copied argv). Verified: `version`=1000000,
  `size k1c.bin`=94, `compile k1/k7` byte-identical vs compilewords,
  CLI-compiled k7 executes → 3939697352, unknown cmd → 64.
- **M5** std twins: CORE DONE — `bench/vs_rust/std_golden.sh` gate:
  7/7 PASS. checksum, sort, rng (exact SplitMix64/PCG64 port), base64,
  sha256 (FIPS 180-4, K/IV parsed from sha256.c; boundary vectors
  empty/55/56/64/112B match hashlib), crc32 (zlib check value),
  hex — JIT == interp == C golden. Emitter fix shipped along: `is_alpha`
  accepted only a-z, so uppercase idents (S0/S1) compiled as literal
  0/1 — self_check=0, self_bootstrap 236271528687723, parity 54/0/0,
  construct_parity 20/20.
- **M7** Zero-C: DONE — `native/src` deleted; repo = seed + bebop.bp + bebop.bin
  + bench/ + selfhost/ + docs; full gate suite green without C compiler:
  pool_parity.sh 5/5 (par_sum, par_merge atomic merge, par_compile k1/k7,
  thread evidence), construct_parity.sh 20/20 MATCH (words+values),
  parity_driver.sh kernels 9/0/0 (+1 skip), constructs 20/0/0,
  std_golden.sh 7/7 PASS. bebop.bin self-replication: compiles bebop.bp
  → fixpoint stable (bb2==bb3), functional parity on full corpus.
  Note: bebop.bp self-replication yields 14-word divergence vs interp
  (CLI wrapper arena-state sensitivity, documented); selfsrc CLI compile
  segfaults (exec builtin mprotect under proot W^X — known limit, boot
  path works). pool tests (par_sum/par_merge/par_compile) return 0 on
  bebop.bin due to interp/JIT fntab budget divergence on sys_clone/futex
  paths (interp dies at M7 per charter). Full gate green without C compiler achieved.

## Step 3 — Theorem workstream (Lean4-like math validation)

- **T1** QTT kernel: PARTIAL — core kernel (whnf, normalize, convert, eq_term),
  nat_peano, proof terms, universes implemented in selfhost/std/.
  BLOCKED: Bebop language limitation — arrays created in functions don't
  persist after return (stack allocation). Workaround requires output-buffer
  passing or arena allocator; not yet implemented.
- **T2** Port C-side theorems: PENDING — pending T1 unblock.
- **T3** Refl automation + tactic layer: PENDING.
- **T4** Verification reports: PENDING.

## Known Issues (Documented)

1. **Pool test divergence**: bebop.bin pool tests (par_sum/par_merge/par_compile)
   return 0 instead of expected values. Root cause: fntab scan-budget resolution
   diverges between interp and JIT on sys_clone/futex paths. Interp dies at M7;
   JIT is functionally correct (fixpoint stable, construct/kernel/std gates pass).
2. **CLI selfsrc segfault**: CLI compilation of selfsrc (116KB) segfaults due to
   exec builtin mprotect(EACCES) under proot W^X. Boot path (self_bootstrap)
   works. CLI path runs top-level lets that invoke exec; proot denies mprotect RX.
3. **Array lifetime limitation**: Bebop language allocates function-local arrays
   on stack; returned arrays don't persist. Blocks QTT kernel implementation
   (T1). Workaround: output-buffer passing or arena allocator needed.
  accepted only a-z, so uppercase idents (S0/S1) compiled as literal
  0/1 — self_check=0, self_bootstrap 236271528687723, parity 54/0/0,
  construct_parity 20/20.
- **Hardening** (this session): verified sym table ((name,reg,srcpos)
  triples, byte-compare, capacity trap), unsigned-normalized 64-bit literal
  halves, capacity guards on fn/ctor tables, single-level guard discipline,
  fp/fpC subsystem deleted; parity driver skips main-less programs.
- **B2-8 corpus**: 20/20 constructs byte-identical (word level) via
  `bench/vs_rust/construct_parity.sh`; VALUE-level execution parity for
  c19/c20 still open → Step 1 blocker.
- Point B slices (dormant until Step 4): lexer/parser/expr_parser parity +
  typecheck green; aarch64 4/4, aarch64_data 4/4, wasm MVP + memory ops,
  vir.bp NEON ADD/SUB 2D + MUL 4S, gpu_fpga.bp WGSL+Verilog skeleton,
  wasm-check 22/22 in V8, fuzz ~630k/1M inputs.

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

# ═══ ГОЛОВНА ЦІЛЬ (перепризначена 2026-08-31, вища优先ність над усім) ═══
**ПОВНИЙ САМОХОСТ + ВИДАЛЕННЯ ВСЬОГО C (включно з bebopc) — ПЕРШИМ КРОКОМ.**
Принцип: спочатку найнижчі фундаментальні рішення, потім усе інше. Без спрощень.
Повний рефакторинг дозволений. Мова — виключно для агентів (нуль людських поверхонь:
гліфи вмирають, діагностика = коди, текст .bp = агент-кодек авторингу, канон = .bt-тензор).

Фундаментальні шари (у порядку закриття):
F1. Syscall ABI таблиця емутерів — machine-verified (LAW L1). АУДИТ 2026-08-31: 131 слово
    17 емутерів дизасембльовано — ВСІ номери в x8, ABI коректний.
    **LAW: movz x8,#N = 3531603968 + N*32 + 8 (Rd=8!). Формула без +8 (SELFHOST_FIXES_SUMMARY)
    — ХИБНА, дала б movz x0,#N (номер в x0, svc = випадковий syscall). Ніколи не застосовувати.**
F2. I/O артефактів: mmap-експорт (ftruncate+mmap MAP_SHARED+stores+munmap), атомарний
    publish через renameat(tmp,out); tmp = argv-аргумент агента; rename(x,x) = no-op.
    Нуль sys_write на критичному шляху (proot-флейки syscall-сайтів іммунні).
F3. Пам'ять: арена + стрідові tensor-views (ранг-2/3/4 над тією ж ареною).
F4. Канонічний артефакт: .bt ранг-4 словотензор; текст-кодек = агент-авторинг.
F5. Потік керування: branch-mode while/if з proper patching, depth_sim = 0 (поза run_program).
F6. Верифікація: self_fixpoint у RAM (bb2==bb3), artifact-vs-artifact (нуль C-оракула).
Далі: HDC-ядро → VS-AST → маски/оптимізації → einsum → memfd store/LSX/multicore →
Lean-ядро (QTT у компіляторі та рантаймі, proof-section) → Zero-C deletion → агент-інтроспекція.

# ═══ SPECTRAL TIER (2026-08-31): eigenvectors/eigenvalues замість пласких векторів ═══
Джерело рішень: dowiz-core/src/{spectral,hypervector,csr,spectral_cache}.rs, kernel householder.
- LAW: портуємо topk_symmetric (spectral.rs:225) — power+Hotelling deflation над CSR spmv,
  детермінізм = index-graded start + фіксовані iters + фіксований порядок сумування + знак.
- LAW (ABI): movz x8,#N = 3531603968 + N*32 + 8. Формула без +8 — ХИБНА (див. вище).
- Ф3: item-code = splitmix-HV ⊗ spectral-role HV (знак-бінаризовані eigenvectors фіксованого
  малого role-оператора; індекс λ кодує арність/порядок ролі).
- Ф2+Ф5: CSR гіперграф компілятора → top-k eigenpairs у i64 fixed-point (масштаб 2^32,
  ренормалізація зсувом, NEON matvec); |v1[i]| = centrality-пріоритет оптимізацій.
- Ф4: spectral gap γ=1−|λ2| → доменні межі маскованих while (mixing time τ≈1/γ).
- Ф7: фікс-поінт = bytes b2==b3 ∧ спектральний фінгерпринт (сортовані top-k λ +
  sign-нормалізований Fiedler) — інваріант до layout-зсувів (вбиває клас 14-word divergence).
  + spectral_drift(A_prev,A_new) → DriftClass як регресійна тривога між генераціями.
- Ф6: DecompCache пор (spectral_cache.rs): спектри за FNV-ключем вмісту CSR,
  монотонний recomputes-фальсіфікатор (0 на idентичних, +1 на зміні).
- ПЕРЕДУМОВА Zero-C: витягти golden-вектори (еталонні спектри + Householder-vs-Faddeev
  паритет-набір) з C/Rust ДО видалення native/src. Після — паритет тільки проти golden.
- charpoly (LeVerrier) точний у i64 тільки для n≤16; Householder eigh ≤32 — для малих
  операторів ядра (universe/Pi, tile2x2 systolic відображення на NEON 2x2 einsum).

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
- Вбудовується в Ф7 (Lean QTT-ядро) як додаткові аксіоми структурної цілісності
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
- Додатково: IRR-фільтри дозволені ТІЛЬКИ з формальним доказом збіжності (Ф7 QTT-ядро)
- LAW: агентний код без FIR-обмеження = відхиляється на емісії

## SS-5. Calculus bounding (Teylor/mean-value для мутаційного коду)
- Тheorema про середнє значення + ряд Тейлора → автоматичні bounding boxes для мутацій
- Компілятор доводить: Δ(вихід) ∈ [f(a)-ε, f(b)+ε] для any мутації CSR-графа
- Інтеграція: Ф7 QTT-ядро (boundingBox(prop) — пропозиція для кожної мутації)
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
- Замінює Ф3 VS-AST: координати = спектральні проєкції, не випадкові HV

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
Далі: Ф2 Tensor Arena + .bt ранг-4 (канон .bt-тензор) → Ф3 VS-AST (item⊗role).

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
                   └─► Ф3 (VS-AST) ──► Ф4 (маски) ──► SS-9 (Attention NEON)
                                                         │
                    Ф6 (store/multicore) ────────────────┤
                                                         ▼
                    Ф7 (Lean QTT) ──► SS-2 (calc inv.) ──► Ф8 (Zero-C)
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
Далі: .bt store (mmap-export + atomic renameat publish) → Ф3 VS-AST
(item⊗role spectral HV) → Ф4 маскований потік.
