# ГІДРАВЛІЧНИЙ КОНТУР СЕНСУ v2.0 — БЛЮПРИНТИ РЕРАЙТУ
## Implementation-ready робочі одиниці для агентів-виконавців

> **Що це.** Похідний від [HYDRAULIC-LOOP-v2-PLAN.md](HYDRAULIC-LOOP-v2-PLAN.md) каталог блюпринтів. Кожен
> блюпринт — **самодостатня робоча одиниця для одного агента**: точний файл, звірений поточний стан,
> математично обґрунтована ціль, реалізовний алгоритм, і falsifiable RED→GREEN гейт. Автор блюпринтів не
> кодить — він синтезує; кодять агенти-виконавці за цими специфікаціями.
>
> **Джерела.** 6 глибоких аналізів коду + 6 математичних досліджень (конспекти:
> [CODE-ANALYSIS-CONSPECT.md](CODE-ANALYSIS-CONSPECT.md), [MATH-RESEARCH-CONSPECT.md](MATH-RESEARCH-CONSPECT.md))
> + план v2.0. Поточний стан кожного цільового файлу звірено на диску перед написанням (line-числа можуть
> дрейфувати після перших правок — орієнтуйся на **символи й поведінку**, не на номери рядків).

---

## 0. ЯК ЧИТАТИ І ВИКОНУВАТИ ЦЕЙ ДОКУМЕНТ

### 0.1 Контракт виконавця (обов'язковий для КОЖНОГО блюпринта)

1. **RED→GREEN або не зроблено.** Жоден блюпринт не «done» без детермінованого гейту, доведеного
   **червоним, тоді зеленим**. Гейт указано в кожному блюпринті. Заборонено: skip/.only/inflated-timeout/
   `expect(true)`/закоментована асерція (test-integrity red-lines проєкту).
2. **Grounding перед правкою.** Прочитай цільову секцію файлу перед редагуванням (правило Read-before-edit).
   Якщо звірений тут поточний стан **розійшовся** з тим, що на диску — зупинись, познач розбіжність, не
   вгадуй (згідно з `ask-dont-guess`).
3. **Скоуп жорсткий.** Кожен блюпринт має розділ «OUT OF SCOPE / НЕ ЧІПАТИ». Не роби feature-creep. Червоні
   лінії (money/auth/RLS/migrations/crypto) — окремий gate, не чіпати без явного дозволу.
4. **Математична коректність — критерій істини.** Якщо реалізація не відповідає формулі з блюпринта —
   неправильна реалізація, не переписуй формулу (правило test-failures = code is wrong).
5. **Не послаблюй наявні гейти.** Ніколи не роби cheat-green; кожен новий guardrail — монотонний ратчет.

### 0.2 Шаблон блюпринта

```
### BP-NN — Назва
Layer · Priority(🔴/🟠/🟡) · Depends-on · Parallel-lane · Est-size
TARGET FILES     — точні шляхи
CURRENT STATE    — звірено (символ + поведінка)
WHY              — математичне обґрунтування (посилання на план §)
TARGET STATE     — точні сигнатури + інваріанти
ALGORITHM        — implementation-ready псевдокод
RED→GREEN GATE   — falsifiable тест (точна асерція)
ACCEPTANCE       — чекліст
OUT OF SCOPE     — що НЕ чіпати
```

### 0.3 Пріоритети

- 🔴 **Червоний** — блокує математичну коректність контуру. Без цього система «збігається» до неправильної
  відповіді з виглядом успіху. Робити першими.
- 🟠 **Помаранчевий** — security/correctness. Виправити перед прод-довірою.
- 🟡 **Жовтий** — робасність. При добудові.

---

## 1. ГРАФ ЗАЛЕЖНОСТЕЙ І ХВИЛІ ВИКОНАННЯ

Блюпринти згруповано у 5 хвиль. Усередині хвилі позначено паралельні смуги (collision-free — різні файли,
без спільного стану) — їх можна дати різним агентам одночасно. Між хвилями — бар'єр (наступна залежить від
попередньої).

```
ХВИЛЯ 0 (розблокування ядра) ───────────────────────────────────────────
  BP-01 resonator unlock (bebop2 lib.rs)        lane-A  🔴 [1 рядок]
  BP-02 arccos/geodesic metric (algebra.rs)     lane-B  🔴
  BP-22 resonator TS↔Rust reconcile             lane-A  🔴 (після BP-01)
        ↓ бар'єр: ядро-регулятор живий і на правильній метриці

ХВИЛЯ 1 (червоний клас — математична коректність) ──────────────────────
  BP-03 Francis QR eigensolver (lyapunov.rs)    lane-A  🔴
  BP-04 diffusion sign fix (coherence.rs+active) lane-B 🔴
  BP-05 PID redesign (governor.rs)              lane-C  🔴
  BP-06 entropy-budget ledger (новий модуль)    lane-D  🔴
        ↓ бар'єр: усі несучі формули коректні

ХВИЛЯ 2 (нові примітиви L2/L3/L4) ──────────────────────────────────────
  BP-07 online DMD (новий модуль)               lane-A  🔴 (dep BP-03)
  BP-08 admit() intake compiler (новий модуль)  lane-B  🔴
  BP-09 persistence survival table (новий)      lane-C  🔴 (dep BP-13)
  BP-10 orthogonometer + Goodhart (новий)       lane-D  🔴
  BP-11 renormalizer rate-distortion (новий)    lane-E  🔴 (dep BP-06)
        ↓ бар'єр: панель приладів і актуація повні

ХВИЛЯ 3 (помаранчевий клас — security/correctness) ─────────────────────
  BP-12 wiring strong AuditLog (wiring.rs)      lane-A  🟠
  BP-13 salience-weighted decay (memory.rs)     lane-B  🟠
  BP-14 field-veto semantic (field.rs)          lane-C  🟠
  BP-15 guard-bash wiring (settings.json)       lane-D  🟠
  BP-16 agentic_git full snapshot (agentic_git) lane-E  🟠
  BP-17 money checked arithmetic (money.rs)     lane-F  🟠 (RED-LINE gate)
        ↓ бар'єр: цілісність і безпека

ХВИЛЯ 4 (інтеграція 6 шарів) ───────────────────────────────────────────
  BP-18 wire resonator у 6-шаровий контур       lane-A  (dep усі вище)
  BP-19 instrument panel L2 (агрегація)         lane-B  (dep BP-02,07,09,10)
  BP-20 orchestration state-machine (loops)     lane-C  (dep BP-08)
  BP-21 Kalman measurement-update (kalman.rs)   lane-D
  BP-23 yellow-class batch (дрібні фікси)       lane-E  🟡
```

**Правило паралелізації:** різні lane-и однієї хвилі — різні файли, нуль спільного мутабельного стану →
безпечно давати різним агентам. Інтеграційні точки (Хвиля 4) — після фан-ауту, робить lead.

---

## ХВИЛЯ 0 — РОЗБЛОКУВАННЯ ЯДРА

### BP-01 — Розморозити resonator у bebop2
**Layer** L0 · **Priority** 🔴 · **Depends-on** — · **Lane** A · **Est** 1 рядок + перевірка

**TARGET FILES:** `/root/bebop-repo/bebop2/core/src/lib.rs`

**CURRENT STATE (звірено):** `resonator.rs` існує (500 рядків, повний closed-loop регулятор, 6 тестів), але
у `lib.rs` **немає** `pub mod resonator` — grep підтвердив 0 згадок. Модуль ніколи не компілюється, 6 тестів
не бігають. Host-gated блок модулів — рядки 289–304 (`field`, `vsa`, `algebra`, `kalman`, `lyapunov`,
`chebyshev`, `fft`, `active`).

**WHY:** [план §5.1 #1] — це єдиний закінчений замкнутий регулятор стеку; він мертвий код. Розморозка =
найдешевший unlock усієї системи. Після цього Rust-оригінал (строга метрика, initial checkpoint,
chaos-termination, baked-in rollback) стає reference для звірки TS-порту (BP-22).

**TARGET STATE:** після рядка 304 (`pub mod active;`) додати:
```rust
#[cfg(feature = "host")]
pub mod resonator; // closed-loop controller: generate→reflect→supervise, Lyapunov freeze, rollback-to-best
```

**RED→GREEN GATE:**
- RED (до): `cargo test -p bebop2-core --features host resonator::` → «module not found» / 0 tests.
- GREEN (після): усі **6** resonator-тестів біжать і зелені: `converging_loop_resonates_under_epsilon`,
  `runaway_loop_frozen_by_lyapunov_guard`, `runaway_loop_blows_fuse_when_guard_off`,
  `guard_off_still_diverges`, `reference_reinjection_prevents_drift`, `rollback_returns_best_checkpoint`.

**ACCEPTANCE:** ☐ 6/6 тестів зелені ☐ `cargo build --features host` без warning про unused module ☐
wasm32-збірка (без host) не зачеплена (resonator host-gated).

**OUT OF SCOPE:** не міняти логіку resonator.rs (лише реєстрація); реконсиляція з TS — окремо BP-22.

---

### BP-02 — Геодезична (arccos) метрика в algebra + AngularMetric для resonator
**Layer** L2 · **Priority** 🔴 · **Depends-on** — · **Lane** B · **Est** S

**TARGET FILES:** `/root/bebop-repo/bebop2/core/src/algebra.rs`; `/root/bebop-repo/crates/bebop/src/knowledge.rs`;
(споживач) `/root/dowiz/agent-governance/resonator.ts` — додати `AngularMetric`.

**CURRENT STATE (звірено):** `algebra.rs:32` має `cosine_similarity(a,b)` (zero-guard 1e-12, clamp [−1,1]),
але **немає** кутової/геодезичної відстані. `knowledge.rs:62` `cosine()` на 256-dim byte-histogram, NOISE_FLOOR
0.35. `resonator.ts:61` `L2Metric` (Euclidean). **Нюанс:** `1−cos` НЕ вживається в resonator.ts безпосередньо
(він generic над `Metric<S>`); `1−cos` фігурує у формулах Sn/Δn **схеми** — тому завдання не «замінити», а
**додати правильну метрику** і використати її там, де манометр similarity→distance.

**WHY:** [план §2.1 В1] `1−cos` **не метрика** (порушує нерівність трикутника) → Банах беззмістовний. `arccos`
(геодезична на сфері) і хордова `√(2(1−cos))` — метрики. Обидві order-preserving → логіка порядку не
міняється, лише ratio-арифметика `k̂` стає валідною.

**TARGET STATE:** у `algebra.rs` додати дві чисті функції:
```rust
/// Geodesic (angular) distance on the unit sphere. IS a metric (unlike 1−cos).
/// d_g ∈ [0, π]. Clamp cos into [−1,1] before acos to avoid NaN at the poles.
pub fn geodesic_distance(a: &[f64], b: &[f64]) -> f64 {
    (cosine_similarity(a, b)).clamp(-1.0, 1.0).acos()
}
/// Chordal (Euclidean-on-sphere) distance = √(2(1−cos)). Also a metric; cheaper, no acos.
pub fn chordal_distance(a: &[f64], b: &[f64]) -> f64 {
    (2.0 * (1.0 - cosine_similarity(a, b))).max(0.0).sqrt()
}
```
У `resonator.ts` додати `AngularMetric: Metric<number[]>` з `distance = arccos(clamp(cos))`, щоб контур міг
працювати на семантичній сфері, а не лише L2.

**ALGORITHM / ІНВАРІАНТ:** контракцію доводити **лише в геодезичній кулі радіуса < π/2** навколо `x*` (cut
locus / антиподи, де arccos негладка). Ре-ін'єкція еталону тримає `x_n` у цій кулі.

**RED→GREEN GATE:** cosine-mirage RED-тест (план §2.1 RED-D):
- Побудувати трійку `x_a, x_b, x_c` де `(1−cos)` **спадає** step-to-step, а `arccos` **росте**.
- RED: детектор «significant decrease» на `1−cos` каже «convergence», на `arccos` каже «divergence».
- GREEN: `geodesic_distance` дає монотонну відстань, яка задовольняє нерівність трикутника на випадковому
  пакеті трійок: `d_g(a,c) ≤ d_g(a,b)+d_g(b,c)` (100 випадкових трійок, tol 1e-9).

**ACCEPTANCE:** ☐ triangle-inequality тест зелений для geodesic+chordal ☐ NaN-guard на антиподах
(`acos(±1.0)` не NaN) ☐ AngularMetric у resonator.ts + unit-тест.

**OUT OF SCOPE:** не чіпати `cosine_similarity` (він коректний); не видаляти L2Metric (лишається дефолт для
не-семантичних станів).

---

### BP-22 — Реконсиляція resonator TS-порту з Rust-оригіналом
**Layer** L0 · **Priority** 🔴 · **Depends-on** BP-01 · **Lane** A · **Est** M

**TARGET FILES:** `/root/dowiz/agent-governance/resonator.ts` (+ `resonator.test.ts`)

**CURRENT STATE (звірено):** TS-порт розходиться з Rust-оригіналом у **4** семантично навантажених місцях
[конспект R-D/E].

**WHY:** [план §5.3 #19] після розморозки (BP-01) Rust — reference; TS дає **слабші** гарантії і в одному
місці може тихо «збігтися» на неправильному стані.

**TARGET STATE — усунути 4 розбіжності:**
1. **Fail-open метрика.** Rust `L2Metric` на mismatch довжин → `INFINITY` («never silently converge»); TS
   ітерує `Math.min(a.length,b.length)` → mismatch тихо конвергує. **Fix:** TS L2Metric на mismatch → повернути
   `Infinity`.
2. **Немає initial checkpoint.** Rust seed `checkpoints[0]=initial`; TS пушить лише в циклі → `max_iterations=0`
   читає `checkpoints[0]` порожнього масиву (undefined) і не може відновити стартовий стан. **Fix:** seed
   `checkpoints[0]` перед циклом.
3. **Немає chaos-termination.** Rust має `is_chaotic` stop (rising_streak≥n && total>1e-9); TS накопичує
   `totalDrift` але не зупиняється на осциляцію. **Fix:** портувати `DriftAccumulator.is_chaotic` + третю
   гілку termination.
4. **Немає implicit rollback.** Rust повертає best checkpoint; TS повертає last committed (rollback opt-in).
   **Fix:** `final_state/final_error` = best checkpoint (як Rust); лишити `rollbackToBest()` як явний API.

**RED→GREEN GATE:** порт-parity — прогнати обидві версії (Rust via FFI/фікстура, TS) на однаковому пакеті з
6 сценаріїв resonator-тестів; вердикт (`Converged/Fused/Stalled`) і `final_error` мають збігатися до 1e-9.
Спеціальний RED: `max_iterations=0` не кидає (наразі кидає undefined); attracting 2-cycle → TS дає `Stalled`
(наразі крутиться до fuse).

**ACCEPTANCE:** ☐ 4 розбіжності усунені ☐ parity-тест зелений ☐ `max_iterations=0` не кидає.

**OUT OF SCOPE:** не міняти публічний API-контракт (типи `LoopConfig/Actors/Metric`); Actors лишаються sync
(async-порт — інша робота, поза цим блюпринтом).

---

## ХВИЛЯ 1 — ЧЕРВОНИЙ КЛАС (МАТЕМАТИЧНА КОРЕКТНІСТЬ)

### BP-03 — General real eigensolver (Francis QR) для несиметричного Ã
**Layer** L2 · **Priority** 🔴 · **Depends-on** — · **Lane** A · **Est** M

**TARGET FILES:** `/root/bebop-repo/bebop2/core/src/lyapunov.rs` (додати шлях); опційно `kalman.rs`
(`real_eig` має ту саму ваду).

**CURRENT STATE (звірено):** `lyapunov.rs:19` `fn eigenvals(a,n)` — коментар «via the Jacobi method (mirrors
the kalman path)»: це **симетричний** Jacobi-sweep (зануляє `a_pq` припускаючи `a_pq=a_qp`). `spectral_radius`
(:91), `stability_margin` (:72) споживають його. Для **несиметричного** `Ã` (reduced DMD-оператор) із
**комплексними** власними числами (ротаційні моди) Jacobi тихо повертає неправильні значення.

**WHY:** [план §2.1/§2.2, дослідж. R1/R2] DMD-оператор оберту загалом несиметричний; його цікаві моди
(ротація, attracting 2-cycle μ≈−1) мають комплексні власні числа. Симетричний Jacobi їх **misreports** — і
контур пропускає нестабільність (найгірший false-green: k̂ каже «збіглося», а є 2-cycle).

**TARGET STATE:** додати функцію, що повертає **комплексні** власні числа несиметричного дійсного `r×r`:
```rust
/// Complex eigenvalues of a general real r×r matrix (r small, ≤ ~16).
/// r==1: trivial. r==2: closed form μ = τ/2 ± √((τ/2)²−δ), τ=trace, δ=det;
///        complex pair when (τ/2)²<δ, then |μ|=√δ.
/// r>2:  real Schur via Francis double-shift QR → read 1×1 (real μ) and 2×2 (complex pair) blocks.
pub fn eigenvalues_general(a: &[f64], n: usize) -> Vec<Complex>;
/// Spectral radius from the general path (для несиметричного Ã).
pub fn spectral_radius_general(a: &[f64], n: usize) -> (f64, bool); // (ρ=max|μ|, ρ<1)
```
Наявний симетричний `spectral_radius` **лишити** (він коректний для симетризованого `(Ã+Ãᵀ)/2` — Path A
з §2.2). Новий — для Path B (ротація).

**ALGORITHM:**
- `r=2`: `τ=a₀₀+a₁₁; δ=a₀₀a₁₁−a₀₁a₁₀; disc=(τ/2)²−δ`; `disc≥0`→два дійсних `τ/2±√disc`; `disc<0`→пара
  `τ/2 ± i√(−disc)`, `|μ|=√δ`. Дешевий stability-read: **unstable ⟺ det(Ã)>1**.
- `r>2`: Hessenberg-редукція → Francis double-shift QR iterations до real Schur form (tol на subdiagonal
  1e-14, ≤ 30·r iterations) → 1×1 блоки = дійсні μ, 2×2 блоки = комплексні пари (закрита форма вище).

**RED→GREEN GATE:** attracting-2-cycle RED (план §2.2):
- Матриця `Ã=[[0,1],[1,0]]` (swap, eig ±1) → RED: симетричний `spectral_radius` дає щось `<1` (misreport);
  `spectral_radius_general` дає `ρ=1.0` (флагує non-contraction).
- Slow-spiral GREEN: `Ã=1.02·[[cos.05,−sin.05],[sin.05,cos.05]]` → `|μ|=√det=1.02>1` UNSTABLE, точність 1e-6.

**ACCEPTANCE:** ☐ 2-cycle і spiral RED→GREEN зелені ☐ `eigenvalues_general` збігається з симетричним на
симетричних матрицях (regression) ☐ немає panic на defective/repeated eigenvalues.

**OUT OF SCOPE:** не чіпати наявний симетричний Jacobi (він потрібен для Path A + Kalman covariance).

---

### BP-04 — Виправити знак дифузії (anti-diffusion) у coherence + field_active
**Layer** L0/L2 · **Priority** 🔴 · **Depends-on** — · **Lane** B · **Est** S

**TARGET FILES:** `/root/bebop-repo/crates/bebop/src/coherence.rs`; `/root/bebop-repo/rust-core/src/lib.rs`
(`field_active`, якщо використовується); (правильний еталон) `bebop2 core/src/chebyshev.rs`,
`crates/bebop field_spectral`.

**CURRENT STATE (звірено):** `coherence.rs:4` docstring обіцяє `u(t)=exp(−L·t)·u0` (дифузія), але `:39` `acc =
deg[i]*u[i]` мінус сусіди = `+L·u` (не `−L·u`), і `:51` `u[i] += dt*coeff*lu[i]` інтегрує `u̇=+cLu` =
`exp(+Lt)` = **анти-дифузія** (зростання). Плюс `dt=t.max(1e-3)` → `steps=1` завжди (один Euler-крок).
`rust-core field_active` (D3) — та сама вада знаку.

**WHY:** [план §5.1 #4, дослідж. R2] позитивний seed штовхає **негативну** амплітуду на сусідів; кожен
споживач `propagate_wave`/`wave_probe` біжить на інвертованому ядрі. Тести проходять лише бо порівнюють
**відносні** магнітуди/детермінізм, а не знак/mass-conservation.

**TARGET STATE:** два варіанти (обрати один, задокументувати):
- **(A, мінімальний):** виправити знак — `u[i] -= dt*coeff*lu[i]` (інтегрувати `−cLu`), і правильний
  багатокроковий Euler (`steps=ceil(t/dt_stable)`, `dt_stable=0.02` — B11 corridor).
- **(B, кращий):** **депрекувати** `coherence::propagate`/`field_active` для дифузії; переспрямувати всіх
  споживачів на **Chebyshev spectral** (`chebyshev::spectral_propagate` / `field_spectral`) — правильний
  знак, mass-conserving, детермінований libm. Лишити coherence лише для `interfere` (superposition
  `(ψ₁±ψ₂)²`, яка знак-агностична).

**Рекомендація:** варіант B (Chebyshev вже перевірений mass-conservation тестом; §2.2 каже
«використовувати виключно Chebyshev spectral path»).

**RED→GREEN GATE:** mass/decay RED:
- RED: seed `[1,0,0,0]` на 4-node path після одного оберту — анти-дифузія дає `[1.5,−0.5,…]` (сума росте,
  сусіди від'ємні); assert «дифузія зберігає невід'ємність сусідів і не збільшує масу» падає.
- GREEN: після фіксу/переспрямування `Σu ≈ 1` (mass-conserving, tol 1e-2), сусіди ≥ 0, амплітуда seed
  спадає.

**ACCEPTANCE:** ☐ mass-conservation тест зелений ☐ усі споживачі `propagate_wave`/`wave_probe` на правильному
ядрі ☐ `interfere` не зачеплено.

**OUT OF SCOPE:** не чіпати Chebyshev (він правильний); не міняти superposition-математику.

---

### BP-05 — Перепроєктувати PID governor (Джурі-стабільний + filtered derivative)
**Layer** L3 · **Priority** 🔴 · **Depends-on** — · **Lane** C · **Est** M

**TARGET FILES:** `/root/bebop-repo/crates/bebop/src/governor.rs`

**CURRENT STATE (звірено):** `governor.rs` `GovConfig` default: `kp=1.4, ki=0.22, kd=1.5, u_min=0.0,
u_max=1.0, target_quality=0.9, dead_ic=0.02`. `error = quality − 0.9`; `quality∈{0,1}` бінарний.
Дефекти: `u_min/u_max/dead_ic` **не читаються** (немає output-clamp перед інтегруванням у authority);
`kd=1.5` на бінарному сигналі = derivative kick.

**WHY:** [план §2.6 В7, дослідж. R6] На інтегруючому об'єкті `G=Kg/(z−1)` критерій **Джурі**:
`|a₀|=Kg·Kd=1.5>1` **ПОРУШЕНО**, `Kg(2Kp+Ki+4Kd)=9.02≥4` **ПОРУШЕНО** — дві умови fail, контур
**нестабільний**. Бінарний `Δe=±1.0` → D-term `±1.5` > весь authority budget [0,1] ⇒ bang-bang; один reject
`Kp·(−0.9)=−1.26` slams authority to 0.

**TARGET STATE:**
1. **Filtered derivative** (low-pass, EMA): замінити чистий `Kd·(e_t−e_{t-1})` на
   ```
   d_t = γ·d_{t-1} + (1−γ)·Kd·(p_t − p_{t-1}),  γ = 0.8   // pole z=γ; caps flip-kick ≤ 0.2
   ```
   де `p_t` — **approval-rate** (rolling frequency ∈ [0,1]), **не** raw `{0,1}` verdict (D well-posed лише на
   `p_t`).
2. **Активувати output-clamp:** реально використати `u_min/u_max` перед інтегруванням `u` у `authority`
   (bumpless), і `dead_ic` як deadband. Наразі мертві.
3. **Critical damping ζ=1:** `Kp_crit = 2√(Ki(1+KgKd)/Kg)`; при поточних Ki=0.22, Kd (після фіксу) —
   налаштувати `Kp≈1.48` (governor зараз ζ≈0.944, 6% недодемпфований). `Q=1/(2ζ)`.
4. **Gain scheduling** (опційно, узгодити з T-scheduler): `Kp(phase)` спадає, `dim M(phase)` росте
   (hard→soft→style), `E(p)=Kp(p)|M(p)|` bounded — style ніколи не out-competes hard.

**RED→GREEN GATE (детермінований, без LLM):**
- Джурі-тест: обчислити `a₀=Kg·Kd`, перевірити `|a₀|<1` і `Kg(2Kp+Ki+4Kd)<4` — RED на поточних (1.5,9.02),
  GREEN на нових.
- Kick-тест: один reject → `|Δauthority| ≤ 0.2` (не slam to 0). RED на `kd=1.5`, GREEN на filtered.
- Reward-hack RED (з BP-10): `q_t=t, s_t=−t ⇒ r_Δ=−1` детектор fire.

**ACCEPTANCE:** ☐ Джурі-умови GREEN ☐ kick ≤ 0.2 ☐ u_min/u_max/dead_ic реально впливають ☐ ζ≈1.

**OUT OF SCOPE:** governor.rs — не інтегрувати ще в `wire()` (це окремо, BP-18); лише перепроєктувати
регулятор + тести.

---

### BP-06 — Ентропійний бюджет-леджер (integer-біти, не Σ=0)
**Layer** L0/L2 · **Priority** 🔴 · **Depends-on** — · **Lane** D · **Est** M

**TARGET FILES:** новий модуль (запропоновано `/root/dowiz/kernel/src/entropy_budget.rs` або
`crates/bebop/src/entropy_ledger.rs`); reuse-патерни з `money.rs` (integer) і `ledger.rs` (SHA-id, idempotent
replay).

**CURRENT STATE (звірено):** `ledger.rs:77` інваріант «Σ balance == 0» (i128, amount>0, TigerBeetle-style
conservation). `money.rs`: i64 minor units, i128 intermediates, half-up. Ентропійного обліку **немає**.

**WHY:** [план §2.5 В6, дослідж. R5] Ентропія **не conserved** (source + external sink) → `Σ=0` — неправильна
модель. Правильна — **бюджет** (монотонно-споживаний ресурс з cap), як токен-бюджет. «Збереження сенсу» =
Data Processing Inequality (`I(еталон;вихід)` супермартингал).

**TARGET STATE:** integer-бітовий бюджет (bind money.rs дисципліну: біти = minor units):
```rust
pub struct EntropyBudget { h_max: i64, debt: i64 /* D_t, integer bits */ }
// L(x) = compression length = gzip(x).len() * 8   (integer bits, deterministic)
impl EntropyBudget {
    // ΔH_in = max(0, L(x_raw) − L(x_prev)); ΔH_out = max(0, L(x_raw) − L(renorm(x_raw)))
    fn step(&mut self, dh_in: i64, dh_out: i64) -> Result<(), HeatOverflow> {
        self.debt = (self.debt + dh_in - dh_out).max(0);   // clamp₀ (heat не банкується)
        if self.debt > self.h_max { return Err(HeatOverflow); }
        Ok(())
    }
}
```
Reuse `ledger.rs` machinery: SHA-256 entry id, **idempotent replay** (re-apply `(id,ΔH_in,ΔH_out)` = no-op),
reject-on-violation. **Інваріант — `0 ≤ D_t ≤ H_max`, НЕ `Σ=0`.**

**ALGORITHM (стаціонарність, для монітора L2):** `D_t` — random walk, дрейф `μ=E[ΔH_in]−E[ΔH_out]`. Живучість
нескінченної роботи ⟺ **строго від'ємний дрейф** `⟨ΔH_out⟩ > ⟨ΔH_in⟩` (Foster–Lyapunov; `μ≥0` → overflow
a.s.). Монітор рахує ковзний `μ̂` і алармить при `μ̂ ≥ 0`.

**RED→GREEN GATE:**
- RED: `R_cheat` що дропає persistent claim — `L↓` але `claims⊉` (з BP-11) → renormalizer має rejected; сам
  budget: подати послідовність з `μ>0` → `D_t` монотонно росте → `HeatOverflow` спрацьовує (RED→ALARM).
- GREEN: `μ<0` послідовність → `D_t` збігається (bounded), overflow не спрацьовує; idempotent replay =
  no-op.

**ACCEPTANCE:** ☐ integer-only (жоден float не торкається `D_t`) ☐ `0≤D_t≤H_max` інваріант ☐ overflow-reject
☐ idempotent replay ☐ стаціонарність-монітор алармить на `μ̂≥0`.

**OUT OF SCOPE:** не чіпати `ledger.rs` (грошовий Σ=0 лишається для грошей); compression-функцію `L(x)`
абстрагувати за trait (gzip-impl — окремо).

---

## ХВИЛЯ 2 — НОВІ ПРИМІТИВИ (L2 ВИМІРЮВАННЯ / L3 АКТУАЦІЯ)

### BP-07 — Online DMD (rank-1 RLS) — детектор дрейфу всередині циклу
**Layer** L2 · **Priority** 🔴 · **Depends-on** BP-03 · **Lane** A · **Est** L

**TARGET FILES:** новий модуль `bebop2 core/src/dmd.rs`; reuse `kalman::matmul/invert`, `field::jacobi_eigen`
(для Gram-SVD), BP-03 `eigenvalues_general`, `algebra::project`.

**CURRENT STATE (звірено):** DMD немає ніде. Наявні цеглинки: `field.rs LaplacianSpectrum` (Jacobi з
eigenvectors), `kalman.rs SpectralKalman` (real_eig з eigenvector accumulation, matmul, Gauss-Jordan invert),
`algebra::project/reconstruct/cosine`.

**WHY:** [план §2.2, дослідж. R2] PCA-по-Δ математично хибний (сліпий до `μ=1` акумуляції і ротації). DMD дає
модуль (стійкість) + фазу (частоту) кожної моди. Online-версія оновлює оператор оберту **без** зберігання
історії — `O(r²)/оберт`, плоско за довжиною циклу → реалізовно **всередині** контуру.

**TARGET STATE:**
```rust
pub struct OnlineDMD { r: usize, a_tilde: Vec<f64> /*r×r*/, p_inv: Vec<f64> /*r×r*/, u_basis: Vec<f64> /*n×r*/ }
impl OnlineDMD {
    // POD coords x̃ = U*x; на нову пару (x̃_k, ỹ_k=U*x_{k+1}):
    // γ = 1/(1 + x̃ᵀ P x̃);  Ã += γ(ỹ − Ã x̃) x̃ᵀ P;  P -= γ (P x̃)(P x̃)ᵀ   (Sherman–Morrison)
    fn update(&mut self, x_k: &[f64], x_next: &[f64]);
    // ρ̂ = max|μ_i(Ã)|  через eigenvalues_general (BP-03)
    fn spectral_radius(&self) -> f64;
    // моди φ_i для damp: c_i = project(drift, Φ); корекція drift − c_i φ_i
    fn modes(&self) -> Vec<(Complex, Vec<f64>)>;
}
```
- **Difference-snapshot** режим: годувати `Δ_n=x_{n+1}−x_n` (скасовує невідомий `x*`).
- **Truncation:** Gavish–Donoho `τ=2.858·σ_median` (unknown noise) при побудові POD-базису.
- **De-bias:** forward-backward `μ_fb=√(μ_f/μ_b)`; self-check `ρ_f·ρ_b≈1` на стаціонарному сегменті.
- **Exponential forgetting** для time-varying дрейфу: `P ← P/ρ_forget` перед update.
- **1-D спецкейс (verbosity):** афінна регресія `L_{k+1}=a·L_k+b`, `μ=a`; augment state `[x;1]` ловить `μ=1`.

**RED→GREEN GATE:** slow-spiral RED (план §2.2): `A=1.02·R(0.05)` → online DMD після ~4 update дає `ρ̂=1.02>1`
UNSTABLE; PCA-по-Δ ранжує спіраль **останньою** (`‖A−I‖=0.054` << harmless transient `μ=0.3` `|μ−1|=0.7`).
Assert: `DMD_flag≠∅ ∧ DMD_flag⊄PCA_flag`. Verbosity: `L_{k+1}=1.3·L_k` → `a=1.3>1` runaway ловиться, а
`Ln`-ratio при `b>0` false-alarm — показати різницю.

**ACCEPTANCE:** ☐ spiral RED→GREEN ☐ de-bias self-check ☐ `O(r²)/оберт` (профіль) ☐ 1-D verbosity mode.

**OUT OF SCOPE:** не будувати full `A∈ℝ^{n×n}` (лише reduced `Ã`); batch-DMD не потрібен (online достатньо).

---

### BP-08 — admit() — детермінований інтейк-компілятор (well-posedness gate)
**Layer** L4 · **Priority** 🔴 · **Depends-on** — · **Lane** B · **Est** L

**TARGET FILES:** новий модуль `/root/dowiz/kernel/src/intake.rs` (Rust) АБО TS-eq в agent-governance;
reuse decide-law патерн з `order_machine.rs`.

**CURRENT STATE (звірено):** `loops/*.yaml` preconditions — **прозові** рядки, невиконувані. `order_machine.rs`
має decide-law (static `allowed_next`, 3-tier typed error, `fold_transitions` stop-at-first+position).

**WHY:** [план §2.3 В4, дослідж. R3] Некоректна задача не має розв'язку — жодна кількість обертів не
виправить. Інтейк вирішує **структурну** коректність (не контракцію α — вона runtime). 1:1 lift decide-law.

**TARGET STATE:**
```rust
pub fn admit(spec: &EtalonSpec) -> Result<Witness, IntakeError>;
pub enum IntakeError {
    Unsatisfiable      { tier, rule_or_core, fields },  // F1 fail
    UnderDetermined    { free_fields, dof, entropy },   // F2 fail
    IllConditioned     { kind /* KnifeEdge | FeasibleSliver */ },
    NonReproducibleVerify { source },                   // F3 verify fail
    Undecidable        { reason },                      // SMT timeout / nonlinear → human
}
```
Патерн decide-law: static authority (`T∧P` єдиний), unknown⇒rejected never coerced, fold stop-at-first +
return position, стабільні коди.

**ALGORITHM — три детерміновані перевірки cheapest-first:**
1. **UNSAT-драбина:** Tier A структурна `O(n)` (type conflict, empty enum ∩, empty interval `a>b`,
   required∧forbidden, const∉range, cardinality) → Tier B arc-consistency AC-3 (domain wipeout⇒UNSAT) → Tier
   C SMT QF_LIA/QF_LRA (SAT⇒Witness model; UNSAT⇒unsat core; TIMEOUT/nonlinear⇒Undecidable→human,
   **fail-closed: ніколи не claim SAT на timeout**).
2. **Under-determined:** DOF `d=|free|−|binding|` (pinned=singleton adm); ентропія `H=Σ log|adm(f)|`
   (`H=0⟺determinate`); `#M≥2` via AllSAT-2 (model m1 → blocking clause `¬(≈m1)` → resolve; distinct m2 ⇒
   under-determined).
3. **Non-reproducible-verify:** (a) static purity-scan (denylist RNG/clock/network/LLM/mutable — як clippy
   disallowed-methods) + (b) dynamic idempotence probe (`K≥2` evals під perturbed nuisance env; різні біти ⇒
   NonReproducibleVerify) → FORCE human-bypass.

**RED→GREEN GATE:**
- UNSAT RED: spec з `minimum:10, maximum:5` → Tier-A `Unsatisfiable{empty interval}`.
- Under-determined RED: spec з required-полем без constraint → `UnderDetermined{dof>0}`.
- Impure-verify RED: `verify = rand()>0.5` → `K=3` evals різні → `NonReproducibleVerify`; GREEN: `verify =
  (total==sum(lines))` → 3 однакові → pass.

**ACCEPTANCE:** ☐ 3 RED-класи ловляться ☐ fail-closed на SMT-timeout ☐ Witness повертається на well-posed
☐ детермінізм (fixed solver seed).

**OUT OF SCOPE:** не претендувати на доказ контракції (лише структура); SMT-solver — за trait (можна почати
з Tier A+B без повного SMT, Tier C — окремий інкремент).

---

### BP-09 — Persistence survival table (Hungarian + D*-тест + attic re-entry)
**Layer** L1/L2 · **Priority** 🔴 · **Depends-on** BP-13 · **Lane** C · **Est** L

**TARGET FILES:** новий модуль `crates/bebop/src/persistence.rs`; reuse `knowledge::cosine`,
`memory::{attic, restore}`, `enrich::confidence-gate`.

**CURRENT STATE (звірено):** claim-tracking немає. `knowledge.rs` cosine + NOISE_FLOOR 0.35; `memory.rs`
attic move+restore (129) — точний re-entry примітив.

**WHY:** [план §2.4 В5, дослідж. R4] Це **не** TDA, а **survival analysis**. Дає **кількісний** поріг
сигнал/шум замість вібу. Персистентність вимірює generator fixed points (не істину) → мусить бути ADVISORY
prefilter, а не authority.

**TARGET STATE:**
```rust
pub struct Claim { id, birth: u32, last_seen: u32, embedding: Vec<f64> }
pub struct PersistenceTable { claims: HashMap<Id, Claim>, iter: u32 }
impl PersistenceTable {
    // Hungarian max-weight bipartite match S_t ↔ S_{t+1}, edges w=cosine≥τ;
    // <τ ⇒ death+birth; gap≤w rematch через attic ⇒ stitch (birth зберігається)
    fn ingest(&mut self, claims_now: Vec<Claim>);
    // D = last_seen − birth; сигнал iff p^D ≤ α:
    // D* = ⌈log_p α⌉;  Bonferroni D*_Bonf = D* + ⌈log_{1/p} N⌉;  power: n ≥ D*+1 інакше ABSTAIN
    fn is_signal(&self, c: &Claim, p: f64, alpha: f64, n_claims: usize) -> Verdict; // Core|Noise|Abstain|Anomaly
}
```
- **Matching:** Hungarian `O(N³)` (N≈10 trivial), НЕ greedy. `τ*` = Bayes/equal-error суміші
  `p(sim|same)/p(sim|diff)`; NOISE_FLOOR 0.35 лише floor.
- **Anomaly region:** `(birth,duration)` shear; `CORE={b<b_thr,D≥D*}`; `ANOMALY={b≥b_thr,D*≤D≤n−1−b}`
  (трикутник), `b_thr=⌈β·n⌉`, β∈(0.5,0.8] → verify/human, ніколи auto-accept.
- **p̂:** geometric MLE `mean_span/(mean_span+1)` або 2-component EM.

**RED→GREEN GATE:** entrenched-hallucination RED (план §2.4, найважливіший):
- RED: early-born false claim, re-emitted кожен turn → `D=n−1` MAX, `p-value=p^{n−1}` найзначніший → таблиця
  сама labels CORE (пропускає). Assert: **persistence сама НЕ ловить** — тому потрібен AND-gate з verify.
- GREEN: `Accept(c) = P(c) ∧ V(c)` — persistence rejects noise (`D<D*`), external verify rejects false;
  entrenched-hallucination ловиться V-шаром.
- Power RED: `n < D*+1` → verdict `Abstain` («insufficient iterations»), НЕ «no signal».

**ACCEPTANCE:** ☐ Hungarian (не greedy) ☐ `D*=⌈log_p α⌉` + Bonferroni ☐ attic re-entry stitch ☐ AND-gate
з verify ☐ Abstain при недостатньому n.

**OUT OF SCOPE:** persistence — **advisory** (ранжує що verify); ніколи не адьюдикує істину сама.

---

### BP-10 — Ортогонометр + Goodhart-детектор (крос-модельний критик)
**Layer** L2/L3 · **Priority** 🔴 · **Depends-on** — · **Lane** D · **Est** M

**TARGET FILES:** новий модуль `crates/bebop/src/orthogonality.rs` або TS в agent-governance; reuse
`algebra::{project, cosine}`, `eval-layer/openrouter_judge.py` (крос-модельна поверхня).

**CURRENT STATE (звірено):** ортогонометра немає. `resonator.reflect` бачить output+reference, НЕ reasoning
(structurally good, але не гарантує ∂q/∂v=0). eval-layer judge = окрема модель, temp 0.

**WHY:** [план §2.6 В7, дослідж. R6] Виродження пари генератор⊥критик = «агенти згоджуються у своїх
помилках». Крос-модельний критик **математично необхідний** (`hacking-gain ≤ Var(η_shared)`; self-preference
bias B≈0.52).

**TARGET STATE:**
```rust
// A. Paraphrase-invariance (∂q/∂v=0):
//    e_⊥ = e_style − proj_∇u(e_style); для binary q: N meaning-preserving paraphrases,
//    flip_rate = (1/N)Σ 1[q(x'_i)≠q(x)]; orthogonal ⟺ flip_rate ≈ 0; enforce |ρ_gc| ≤ τ_orth≈0.1
fn orthogonality(critic, x, paraphrases: &[X]) -> f64; // flip_rate / ρ_gc
// B. Goodhart detector corr(Δq, Δs):  s = held-out/decorrelated similarity (НЕ той метрик що loop оптимізує)
//    r_Δ = corr(Δq_t, Δs_t) sliding W; Fisher-z lower bound r_lower=tanh(artanh(r̂)−z/√(W−3));
//    ALARM r_lower<r_min≈0.3–0.5; CUSUM C_t=max(0,C_{t-1}+(r_min−r_t))>h
fn goodhart_alarm(q_series, s_series, w: usize) -> Alarm;
```

**RED→GREEN GATE:** reward-hack RED (план §2.6):
- RED: adversarial `q_t=t, s_t=−t ⇒ r_Δ=−1` → Goodhart-детектор fire; GREEN: `q_t=s_t+noise ⇒ r_Δ>0.9` pass.
- Orthogonality RED: критик, чутливий до style-осі → `flip_rate` високий на meaning-preserving парафразах →
  freeze; GREEN: reference-term rubric → `flip_rate≈0`.
- Cross-model: виміряти self-preference `B` для same-model критика vs different-model → different значно
  нижче.

**ACCEPTANCE:** ☐ Goodhart RED→GREEN ☐ paraphrase-invariance ☐ `s` — decorrelated (не loop-метрик)
☐ крос-модель обов'язкова для критика.

**OUT OF SCOPE:** сам генератор/критик LLM-виклики — не тут (тут лише вимірювач); інтеграція у freeze —
BP-18.

---

### BP-11 — Ренормалізатор (rate-distortion@0, claim-preserving)
**Layer** L0 · **Priority** 🔴 · **Depends-on** BP-06 · **Lane** E · **Est** M

**TARGET FILES:** новий модуль `crates/bebop/src/renormalizer.rs`; reuse claim-екстрактор з BP-09,
entropy-budget BP-06.

**CURRENT STATE (звірено):** ренормалізатора немає. `enrich.rs` має SEAL store (trigger→correction),
confidence gate.

**WHY:** [план §2.5 В6, дослідж. R5] Ренормалізатор = єдиний вузол, що виводить назовні рівно одне: шум.
Rate-distortion @ D=0 на claim-manifold: `R(x)=argmin L(x̂) s.t. claims(x̂)=claims(x)`, floor `H(claims)`.

**TARGET STATE:**
```
Renorm(x, claims_extract, L):
    C0 = claims_extract(x)            # ОКРЕМИЙ non-generative verifier (не той LLM, що rewrite)
    x' = LLM_rewrite(x, "compress; keep every claim; drop filler & repetition")
    C1 = claims_extract(x')
    if C1 ⊉ C0:  return ROLLBACK(x)   # dropped claim → FAIL
    if C1 ⊋ C0:  return ROLLBACK(x)   # new/hallucinated claim → FAIL
    if L(x') ≥ L(x): return x         # no compression → idempotent no-op
    assert claims(x') == C0           # I(claims;x')=I(claims;x)
    return x'                          # H↓ AND claims preserved
```
Ентропійний облік: `ΔH_out = L(x)−L(x')` кредитується у budget-леджер (BP-06). Періодичність: кожні K=2
оберти або за тригером термометра ентропії.

**RED→GREEN GATE (найважливіший для ентропії):**
- RED: `R_cheat` що дропає persistent claim (price line / allergen / negation polarity flip) — `L↓` але
  `claims⊉C0` → guardrail **MUST reject/rollback**. Gate = `H↓ ∧ claim-set-equality`, ніколи H↓ alone.
- GREEN: чесний rewrite (drop filler) — `L↓` І `claims=C0` → accept, `ΔH_out>0` у budget.

**ACCEPTANCE:** ☐ claim-preservation RED→GREEN ☐ `claims_extract` окремий від rewrite-LLM ☐ idempotent
`R(R(x))=R(x)` ☐ кредитує budget-леджер.

**OUT OF SCOPE:** semantic-entropy (SE, confabulation heat) — дренує зовнішнє заземлення, не ренормалізатор;
тут лише verbosity/redundancy heat.

---

## ХВИЛЯ 3 — ПОМАРАНЧЕВИЙ КЛАС (SECURITY / CORRECTNESS)

### BP-12 — wiring.rs → міцний (hash-chained) AuditLog
**Layer** L1 · **Priority** 🟠 · **Depends-on** — · **Lane** A · **Est** S

**TARGET FILES:** `/root/bebop-repo/crates/bebop/src/wiring.rs`

**CURRENT STATE (звірено):** `wiring.rs:23` `use crate::research_patterns::{AuditLog, TargetScope}` — це
**СЛАБКИЙ** AuditLog (plain `Vec<(u64,String)>`, без hash-chain). `audit::AuditLog` — міцний (SHA256
hash-chain, `verify()` ловить tamper). wiring мітить його «LAYER 5: AUDIT (tamper-evident ledger)» — брехня.

**WHY:** [план §5.2 #7, конспект A/B D6] аудит-запис 3-шарового циклу **не** tamper-evident — comment/behavior
mismatch + security-gap на red-line-релевантній поверхні.

**TARGET STATE:** переключити import на `crate::audit::AuditLog` (SHA256 hash-chain). Звірити API-сумісність
(`new()`, `record(seq,msg)`); якщо сигнатури різняться — адаптувати виклики у `wire()` (рядки ~178–313 у
тестах теж).

**RED→GREEN GATE:** tamper RED — мутувати записаний entry після `wire()` → `audit.verify()` має повернути
`false` (наразі слабкий AuditLog не має verify, тому tamper непомітний). GREEN: чистий ланцюг verify=true;
мутований — false з індексом розриву.

**ACCEPTANCE:** ☐ import = `audit::AuditLog` ☐ tamper RED→GREEN ☐ усі wiring-тести (6) зелені після адаптації.

**OUT OF SCOPE:** не міняти `research_patterns::AuditLog` (може вживатись деінде для не-security логів);
TargetScope лишається з research_patterns.

---

### BP-13 — memory.rs: salience-weighted decay (замість hash-lottery)
**Layer** L1 · **Priority** 🟠 · **Depends-on** — · **Lane** B · **Est** M

**TARGET FILES:** `/root/bebop-repo/crates/bebop/src/memory.rs`

**CURRENT STATE (звірено):** `tick()` (:141): `target=(clock%7)`, `nodes.retain` (:146) евіктить вузли де
`FNV(concept)%7==target` → attic (non-destructive, restore :129). `salience` (:21) **зберігається але
tick його не читає** — decay = hash-lottery, кожен вузол evicted ≤7 тіків незалежно від важливості.

**WHY:** [план §2.4/§5.2 #8, дослідж. R4] `MI(kept, persistence)=0` — нульова фільтрація. Персистентний фільтр
**вимагає** retention монотонну в persistence-score. Три наявні примітиви (salience, attic, restore) → фільтр
одним рядком decay-логіки.

**TARGET STATE:** замінити hash-lottery на **salience-weighted eviction**:
```rust
// Soft online estimator (оновлюється при seen/tick):
//   s_{t+1}(c) = s_t(c)·exp(−Δt/τ) + boost·seen;  half-life t_½ = τ·ln2;  τ ≈ n/ln(s_max/s_min)
// Eviction: keep iff salience > θ (= D* з persistence, BP-09), else attic (non-destructive).
// або min-salience priority-queue: евіктити найменш-salient коли розмір > cap.
pub fn tick(&mut self) {
    self.clock += 1;
    for n in self.nodes.values_mut() { n.salience *= (-1.0 / self.tau).exp(); }  // exponential forgetting
    // evict below-threshold → attic (зберегти restore-шлях)
}
pub fn reinforce(&mut self, concept: &str, boost: f64); // seen ⇒ salience += boost
```

**RED→GREEN GATE:** filtering RED (план §2.4): high-persistence claim (seen кожен tick, boost) і length-1
noise (seen раз) → RED на старому коді: **однакова** ймовірність eviction (mod-7 lottery); GREEN на новому:
high-salience переживає, noise → attic; `MI(kept, persistence) > 0`.

**ACCEPTANCE:** ☐ salience реально впливає на eviction ☐ non-destructive (attic+restore збережено)
☐ half-life коректний ☐ high-persistence переживає, noise евіктиться.

**OUT OF SCOPE:** не видаляти attic/restore (вони load-bearing для re-entry BP-09); не міняти node-структуру.

---

### BP-14 — field.rs: семантичний field-veto (замість keyword-bypass)
**Layer** L0 · **Priority** 🟠 · **Depends-on** — · **Lane** C · **Est** M

**TARGET FILES:** `/root/bebop-repo/crates/bebop/src/field.rs`

**CURRENT STATE (звірено):** `field_gate_verdict` — keyword→node map (`secret|auth|money|migrat|rls→node4`),
toy 6-node plan graph, veto iff `out[4]>TOLERANCE=0.10`. Blast-фізика **реальна**, але task→node mapping
тривіально обходиться («s3cret», «rotate credentials» → node1 impl → Permit).

**WHY:** [план §5.2 #9, конспект C D8] фізичне вето сильне лише як text→node mapping перед ним; toy-граф — не
реальний граф залежностей.

**TARGET STATE:** дві незалежні добудови:
1. **Семантичний класифікатор** замість substring: embedding-similarity задачі до red-line-концептів (cosine
   до набору еталонних red-line-описів > поріг), не keyword-match. Обфускація («s3cret») лишається близькою
   в embedding-просторі.
2. **Реальний граф залежностей** замість toy-6-node: приймати CSR реального графа (файли/модулі/секрети) —
   CSR-pipeline уже це вміє (`field_build`), лише годувати справжній граф.

**RED→GREEN GATE:** obfuscation RED: task «rotate the deploy s3cr3ts» → наразі Permit (bypass); після фіксу —
все ще vetoed (`out[secrets_node]>0.10`). GREEN: benign task («update docs») → Permit (no over-veto).
Zero-false-positive на безпечному корпусі.

**ACCEPTANCE:** ☐ obfuscated red-line vetoed ☐ benign не over-vetoed ☐ реальний граф приймається
☐ fail-closed на sim-degradation збережено.

**OUT OF SCOPE:** не чіпати heat-kernel blast-математику (вона коректна); TOLERANCE=0.10 — калібрувати
окремо, не в цьому блюпринті.

---

### BP-15 — Підключити guard-bash.sh (мертвий hook)
**Layer** L4 · **Priority** 🟠 · **Depends-on** — · **Lane** D · **Est** S

**TARGET FILES:** `/root/dowiz/.claude/settings.json`; `/root/dowiz/.claude/hooks/guard-bash.sh`

**CURRENT STATE (звірено):** `guard-bash.sh` існує (DANGER regex: fly deploy/secrets, supabase, wrangler, git
push main, git push --force, pnpm migrate:up, pnpm add/remove, npm install, rm -rf /), exit 2 hard block,
але **не зареєстрований** у settings.json hooks → dangerous-command veto **мертвий**. tier1-run.sh навіть
стверджує «dangerous bash still vetoed by guard-bash» — наразі неправда.

**WHY:** [план §5.2 #10, конспект F дефект 1] небезпечні команди у автономних tier-ах не блокуються.

**TARGET STATE:** зареєструвати `guard-bash.sh` як PreToolUse hook на `Bash` у settings.json (поряд з наявними
PreToolUse). Формат — як інші зареєстровані hooks.

**RED→GREEN GATE:** `fly deploy` / `git push origin main` / `rm -rf /` як Bash-команда → hook exit 2, команда
заблокована (RED до реєстрації = проходить; GREEN після = блок). Безпечна команда (`ls`, `git status`) —
проходить.

**ACCEPTANCE:** ☐ guard-bash у settings.json PreToolUse Bash ☐ danger-команди block ☐ safe-команди pass
☐ tier1-run.sh твердження тепер істинне.

**OUT OF SCOPE:** не розширювати DANGER-regex (лише підключити наявний); не чіпати інші hooks.

---

### BP-16 — agentic_git.rs: повний (non-lossy) snapshot
**Layer** L1 · **Priority** 🟠 · **Depends-on** — · **Lane** E · **Est** S

**TARGET FILES:** `/root/bebop-repo/crates/bebop/src/agentic_git.rs`

**CURRENT STATE (звірено):** `snapshot()` захоплює лише `concept→payload` (LIVE nodes), дропає
`layer/entities/topic/salience` і весь attic; `replay` скидає все в `Short`. Docstring «reconstructs the EXACT
memory state» — **перебільшення**. Також dead-branch у `verify_integrity` (обидві гілки if/else однакові).

**WHY:** [план §5.2 #11, конспект B дефект 5] rollback втрачає метадані + cold-tier; чекпойнт не відновлює
повний стан (критично для persistence-фільтра BP-09, який залежить від salience/attic).

**TARGET STATE:** розширити snapshot до **повного** стану вузла (`concept, payload, layer, entities, topic,
salience`) + attic; `replay` відновлює всі поля (не reset у Short). Прибрати dead-branch у `verify_integrity`.

**RED→GREEN GATE:** roundtrip RED: node з `salience=0.9, layer=Long` → commit → replay → наразі salience=0,
layer=Short (RED); після фіксу — salience=0.9, layer=Long збережені. Attic-node теж переживає commit→replay.

**ACCEPTANCE:** ☐ повний стан у snapshot ☐ attic захоплено ☐ replay відновлює всі поля ☐ dead-branch прибрано
☐ hash-детермінізм збережено (sorted serialize).

**OUT OF SCOPE:** не міняти hash-функцію (FNV32 детермінований); не міняти commit-chain структуру.

---

### BP-17 — money.rs: checked arithmetic + прибрати dead guards [RED-LINE]
**Layer** L0 · **Priority** 🟠 (money = 🔴 red-line gate) · **Depends-on** — · **Lane** F · **Est** S

**TARGET FILES:** `/root/dowiz/kernel/src/money.rs` (+ `domain.rs` де `unit_price*quantity`, `subtotal+tax+fee`)

**CURRENT STATE (звірено):** `unit*quantity` (:64) unchecked; `subtotal+tax+fee` unchecked → overflow
panic(debug)/wrap(release). `tax as i64` (:55), `rounded as i64` (:85) — silent truncation. Dead guards:
`to_minor_unit` (:8) `if amount != amount` (NaN-fossil, завжди false на i64); `round_half_up` (:17) =
**identity** (`value*10%10==0` завжди, `rem>=5` unreachable).

**WHY:** [план §5.2 #12] money — red-line; unchecked arithmetic + silent truncation = грошовий баг. Dead
guards вводять в оману (виглядають як захист, не захищають).

**TARGET STATE:** `checked_mul`/`checked_add` з `Result`/`Err` на overflow; `i128→i64` cast з range-check
(`i64::try_from`); прибрати/замінити dead guards (`to_minor_unit` NaN-check, `round_half_up` identity —
або видалити, або реалізувати справжнє округлення з документованою семантикою).

**RED→GREEN GATE:** overflow RED: `unit_price=i64::MAX, quantity=2` → наразі panic/wrap; після — `Err(Overflow)`.
`round_half_up` — якщо лишається, RED-тест що доводить, що `rem>=5` гілка **досяжна** (інакше видалити).

**ACCEPTANCE:** ☐ checked arithmetic (Err не panic) ☐ i128→i64 range-checked ☐ dead guards
прибрані/виправлені ☐ **RED-LINE:** money-тести (18) зелені + жоден money-інваріант не послаблено.

**OUT OF SCOPE:** 🔴 RED-LINE — НЕ міняти rounding-семантику half-up без окремого approval; не чіпати SCALE=1e6;
цей блюпринт лише про overflow-safety + dead-code, не про грошову логіку.

---

## ХВИЛЯ 4 — ІНТЕГРАЦІЯ 6 ШАРІВ

### BP-18 — Змонтувати resonator у 6-шаровий контур (wire)
**Layer** усі · **Priority** — · **Depends-on** BP-01,02,03,05,06,10 · **Lane** A · **Est** L

**TARGET FILES:** новий інтеграційний модуль (запропоновано `crates/bebop/src/loop_runtime.rs`); композиція
над `resonator`, `wiring::wire` (L5-gate), `stabilizer`, `field.rs` veto, `governor` (BP-05).

**CURRENT STATE:** `resonator` (регулятор) і `wiring::wire` (field-veto+bounded-envelope+σ-gate+memory+audit)
існують **окремо**; не з'єднані в один tick. `wire()` обчислює bounded delta але **не актуює** (L5 output не
впливає на proceed).

**WHY:** [план §3, §4] кібернетика замкнутого циклу = resonator tick, де `generate/reflect/supervise` дротяться
через L3 (LLM під bounded envelope) + L0 сепаратор (композитний 5-clause вердикт) + L1 ре-ін'єкція еталону.

**TARGET STATE:** один `run_loop(spec) -> Product`, що реалізує машину станів
`INTAKE→PRIMED→SPIN→{CONVERGED|ABORT|BRANCH|BYPASS}→DELIVER` (план §8):
- `admit(spec)` (BP-08) → WORM-еталон (audit hash-chain) → resonator loop;
- кожен tick: T-scheduler → генератор (bounded via `stabilize_step`) → крос-модель критик (∂q/∂v, BP-10) →
  **композитний 5-clause сепаратор** (§2.1: verify ПЕРШЕ, потім DMD ρ̂/arccos/is_chaotic) → ренормалізатор
  (BP-11, кожні K=2) → best-checkpoint діод;
- field-veto + σ-незгода домінують (proceed = кон'юнкція);
- fuses (max_iter/token-ДО-API/timeout) + escalation (BP-15/N=3).

**RED→GREEN GATE:** end-to-end на калібрувальному пакеті: (1) well-posed задача → CONVERGED з best-checkpoint;
(2) ill-posed → REJECT на admit (не витрачає токени); (3) reward-hack сценарій → freeze+rollback (не
false-green); (4) red-line задача → field-veto ABORT.

**ACCEPTANCE:** ☐ машина станів повна ☐ composite вердикт (verify-first) ☐ field-veto домінує ☐ best-checkpoint
видача ☐ fuses спрацьовують ☐ 4 end-to-end сценарії зелені.

**OUT OF SCOPE:** манiфольд гілок (BP-окремо/§4.11) — після лінійного контуру; async-вежа — окремо.

---

### BP-19 — Instrument panel L2 (агрегація приладів)
**Layer** L2 · **Priority** — · **Depends-on** BP-02,07,09,10 · **Lane** B · **Est** M

**TARGET FILES:** новий `crates/bebop/src/instrument_panel.rs`; агрегує BP-02/07/09/10 + Kalman (BP-21).

**TARGET STATE:** структура, що щооберту емітить:
```
Panel { sim_arccos, k_dmd (ρ̂), delta_n, persistence_D_star, entropy_debt_D_t,
        orthogonometer_r_delta, len_ratio_affine_a, kalman_smoothed }
```
+ аларм-логіка (план §13 виправлена): k-метр `ρ̂∈[0.5,0.8]` аларм-band, ортогонометр `r_Δ<0` fire, ентропія
`D_t>H_max`, persistence late-anomaly. Прив'язка до OTLP-spans (як metric-core).

**RED→GREEN GATE:** кожен прилад має свій RED (успадковано з BP-02/07/09/10); панель агрегує і жоден аларм не
губиться (тест: інжектувати кожну відмову → відповідний аларм fires).

**ACCEPTANCE:** ☐ усі 8 приладів ☐ аларм-band коректні ☐ per-iteration емісія ☐ OTLP-експорт.

**OUT OF SCOPE:** не дублювати математику приладів (лише агрегація).

---

### BP-20 — Orchestration state-machine + виконувані preconditions
**Layer** L4 · **Priority** — · **Depends-on** BP-08 · **Lane** C · **Est** M

**TARGET FILES:** `/root/dowiz/loops/*.yaml` (додати machine-executable preconditions-ref); metric-core.

**CURRENT STATE (звірено):** loop-card preconditions — прозові; CERTIFIED статуси ungated (serious-gate
exempt `loops/*`); check-contracts = `echo OK` placeholder.

**TARGET STATE:** замінити прозові preconditions на посилання на `admit()` (BP-08); додати програмний гейт на
DRAFT→CERTIFIED (не ungated file-edit); замінити check-contracts placeholder на реальну перевірку.

**RED→GREEN GATE:** ill-posed loop-spec → admit REJECT (не dispatch); DRAFT loop → не dispatch без CERTIFIED-гейту.

**ACCEPTANCE:** ☐ preconditions виконувані ☐ CERTIFIED gated ☐ check-contracts реальний.

**OUT OF SCOPE:** не переписувати M1–M11 рубрику (вона добра); лише зробити її enforcement детермінованим.

---

### BP-21 — Kalman measurement-update (для fusion шумного quality)
**Layer** L2 · **Priority** 🟡 · **Depends-on** — · **Lane** D · **Est** M

**TARGET FILES:** `/root/bebop-repo/bebop2/core/src/kalman.rs`

**CURRENT STATE (звірено):** `kalman.rs` — covariance propagation `P_k=A P Aᵀ+Q` **тільки** (немає H/R/gain/
innovation/state-mean). 40% фільтра.

**WHY:** [план §1.1, дослідж. R1] fusion шумного reflector-quality сигналу у стан-оцінку потребує measurement
update. Будівельні блоки (matmul/transpose/invert/eigen) вже є.

**TARGET STATE:** додати measurement update: `K = P Hᵀ(H P Hᵀ + R)⁻¹`; `x += K(z − Hx)`; `P = (I−KH)P`. Стан-mean
propagation `x = Ax`. Використати наявний Gauss-Jordan invert.

**RED→GREEN GATE:** noisy-signal RED: подати шумний quality навколо істинного → Kalman-згладжений має нижчу
variance, ніж raw; gain `K` спадає при converging covariance.

**ACCEPTANCE:** ☐ H/R/gain/innovation ☐ state-mean propagation ☐ variance-reduction тест.

**OUT OF SCOPE:** не чіпати covariance-propagation (він коректний, лишається як predict-step).

---

### BP-23 — Жовтий батч (дрібні робасність-фікси)
**Layer** різні · **Priority** 🟡 · **Depends-on** — · **Lane** E · **Est** S (кожен)

Компактні фікси, кожен зі своїм RED→GREEN; можна одному агенту послідовно:
1. **stabilizer dt≤0 fail-open (D1)** → fail-closed на malformed dt (наразі `dt≤0⇒V̇=0⇒adaptation дозволена`;
   має бути refuse). RED: `dt=−1` дозволяє рух → після: заморожує.
2. **Ln ratio false-alarm** → афінна регресія `a` замість `len/len` (§2.2). RED: settling `b>0` дає ratio>1
   (false runaway) → після: `a<1` коректно.
3. **eval-layer `--dry-run` fake scores** у той самий файл → окремий файл (false-green вектор). RED: dry-run
   не має забруднювати `deepeval-result.json`.
4. **tier2 sentinel auditor STUB** (verdict NO-GO, 0 findings) → реальний Claude-виклик АБО чесний
   «not-implemented» замість фейкового вердикту.
5. **coherence index-clamp (D2)** `u[b.min(n-1)]` тихо ремапить out-of-range edge → skip edge + error.
6. **active_inference `advise` panic на b[a≥1]** (D9) → валідувати всі `b[a]`, не лише `b[0]` (fail-closed
   None, не panic).

**ACCEPTANCE (кожен):** ☐ RED→GREEN ☐ скоуп не розповзся.

---

## ДОДАТОК A — МАТРИЦЯ БЛЮПРИНТ → ПЛАН → ДОСЛІДЖЕННЯ

| BP | Система | План § | Дослідж. | Тип |
|----|---------|--------|----------|-----|
| 01 | resonator unlock | §5.1#1 | R-D | реєстрація |
| 02 | arccos metric | §2.1 В1 | R1 | нова функція + метрика |
| 03 | Francis QR eigen | §2.2 | R1/R2 | новий шлях |
| 04 | diffusion sign | §5.1#4 | R2 | фікс знаку |
| 05 | PID redesign | §2.6 В7 | R6 | перепроєкт |
| 06 | entropy budget | §2.5 В6 | R5 | новий модуль |
| 07 | online DMD | §2.2 | R2 | новий модуль |
| 08 | admit() intake | §2.3 В4 | R3 | новий модуль |
| 09 | persistence survival | §2.4 В5 | R4 | новий модуль |
| 10 | orthogonometer | §2.6 В7 | R6 | новий модуль |
| 11 | renormalizer | §2.5 В6 | R5 | новий модуль |
| 12 | wiring AuditLog | §5.2#7 | A/B-D6 | фікс import |
| 13 | salience decay | §5.2#8 | R4 | фікс decay |
| 14 | field-veto semantic | §5.2#9 | C-D8 | добудова |
| 15 | guard-bash wiring | §5.2#10 | F | реєстрація |
| 16 | agentic_git snapshot | §5.2#11 | B | розширення |
| 17 | money checked | §5.2#12 | E | safety (red-line) |
| 18 | wire 6-layer loop | §3,§4 | усі | інтеграція |
| 19 | instrument panel | §3.3 | R1/R2/R4/R6 | агрегація |
| 20 | orchestration | §5.3 | F | enforcement |
| 21 | Kalman measurement | §1.1 | R1 | розширення |
| 22 | resonator reconcile | §5.3#19 | R-D/E | реконсиляція |
| 23 | yellow batch | §5.3 | різні | дрібні фікси |

## ДОДАТОК B — ІНВАРІАНТИ, ЯКІ ЖОДЕН БЛЮПРИНТ НЕ СМІЄ ПОРУШИТИ

1. **verify ПЕРШЕ, внутрішні метрики ПОТІМ** (DPI §2.5: внутрішня згода без зовнішнього verify нічого не
   важить). Сепаратор не сміє приймати CONVERGED без зовнішнього verify.
2. **Persistence/critic — ADVISORY; verify/детермінований код — AUTHORITY** (§2.4/§2.6). LLM-сигнал ніколи не
   вирішує сам.
3. **Bounded self-modification** — еталон WORM протягом прогону; еволюція лише між прогонами з людським вето.
4. **Крос-модельний критик** — генератор і критик РІЗНІ моделі (§2.6 A3, математична необхідність).
5. **Best-checkpoint діод** — видача завжди з best, не з last iteration (§2.1).
6. **Fail-closed скрізь** — malformed вхід ⇒ refuse, не proceed (виправити наявні fail-open: dt≤0,
   Floyd-guard, active_inference).
7. **Integer-only облік** — токени/ентропія у мінімальних одиницях, жоден float не торкається балансу/боргу.
8. **RED-LINE gate** — money/auth/RLS/migrations/crypto: окремий human-approval, ніколи не в загальному потоці.

---
*Кінець документа блюпринтів. 23 робочі одиниці, 5 хвиль, кожна з паралельними смугами і RED→GREEN гейтами.
Джерело: HYDRAULIC-LOOP-v2-PLAN.md + 6 аналізів коду + 6 математичних досліджень. Критерій приймання —
математична коректність + falsifiable proof. Автор синтезує; виконують агенти.*
