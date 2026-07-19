# ГІДРАВЛІЧНИЙ КОНТУР СЕНСУ — v2.0
## Інженерний план кібернетичної системи закритого циклу на основі L5-стека dowiz/bebop
### Синтез: схема v1.0 × реальний код × математична верифікація. Єдиний критерій істини — математична реалізовність.

> Цей документ — не переказ схеми v1.0 і не її похвала. Це **проєкт побудови**, у якому кожен вузол
> гідравлічного контуру (i) прив'язаний до вже наявного коду вашого L5-стека з оцінкою готовності у %,
> (ii) має **виправлену** математику там, де формальне дослідження показало, що аналогія схеми
> математично хибна або неповна, і (iii) несе falsifiable RED-тест, що падає, коли код неправильний.
>
> Джерела синтезу: 6 глибоких аналізів коду (L5-контроль, жива пам'ять, фізика поля, bebop2-матядро,
> kernel+governance, оркестрація) + 6 математичних досліджень ключових мостів (контракція/Банах,
> спектральні моди дрейфу, well-posedness інтейку, персистентність, ентропійний бюджет,
> ортогональність+PID). Усе — виключно код і математика; документація ігнорувалась за вашою вимогою.

---

## 0. АНОТАЦІЯ ТА СІМ МАТЕМАТИЧНИХ ВИПРАВЛЕНЬ СХЕМИ v1.0

Схема v1.0 — концептуально сильна: вона правильно вловила, що керувати треба **геометрією хаосу
на межах**, а не кроками всередині; що потрібен зовнішній еталон-«земля»; що ентропію треба **відводити,
а не накопичувати**; і що людина — це деградація пропускної здатності, а не штатний вузол. Ці інтуїції
підтвердились математично. Але сім її несучих аналогій при строгій перевірці виявились або **хибними**,
або **неповними**. План v2.0 їх виправляє — інакше система «збігалася б» до неправильної відповіді з
виглядом успіху (найгірший клас відмови, бо детектори мовчать).

| # | Схема v1.0 казала | Математика каже (дослідження) | Наслідок для плану |
|---|---|---|---|
| **В1** | Манометр = `1 − cos` до еталону; Банах дає збіжність при k<1 | `1 − cos` **не метрика** (порушує нерівність трикутника) → Банах до неї **не застосовний**. Метрика — `d_g = arccos(⟨u,v⟩)` (геодезична на сфері) або хордова `√(2(1−cos))` | Усі манометри similarity рахують `arccos`, не `1−cos`. Контракцію доводити лише в геодезичній кулі радіуса <π/2 |
| **В2** | `k` вимірюється лін.регресією `log(Δn)`, ціль `k∈[0.5,0.8]` | Скалярний `k̂` — **лише необхідна** умова. Достатня = `ρ(J)<1`, де J — якобіан оберту, оцінюваний **DMD** (не PCA). `[0.5,0.8]` — не оптимум, а alarm-межі; оптимум розв'язує `c'(k)/c(k)=1/(k ln k)` | `k`-метр = DMD-спектральний радіус через `lyapunov::spectral_radius`, а не регресія. Band — аларм, не ціль |
| **В3** | Спектральна діагностика = PCA по Δ-embedding, `λ≈1` резонанс | PCA-по-Δ **математично хибний**: згортає двобічну вісь стійкості на `\|μ−1\|²`, сліпий до `μ=1` (акумуляція!) і до ротації. Правильно — **DMD** (Koopman): `Ã=U*X'VΣ⁻¹`, `eig(Ã)→μ_i` дає модуль (стійкість) І фазу (частоту) | Детектор дрейфу = online DMD (rank-1 RLS), не PCA |
| **В4** | Промпт = крайова задача; well-posed ⇒ унікальний розв'язок | Правильний **скінченний** аналог — **Тихоновська коректність** обмеженої мінімізації, не PDE-Hadamard. UNSAT/under-determined/non-reproducible-verify — **вирішувані** до запуску; але контракція α чорного ящика — **лише runtime** | Інтейк-компілятор = детермінований `admit()` (UNSAT-драбина + DOF + purity), але не претендує на доказ стійкості |
| **В5** | Персистентність = TDA (топологічний фільтр сигнал/шум) | Це **не** персистентна гомологія, а **аналіз виживання**. Теорема стабільності НЕ переноситься (і хибна під парафразом). Але геометрична нуль-модель дає **кількісний** поріг `D*=⌈log_p α⌉` | Persistence-таблиця = survival test з α-порогом + Hungarian matching, не баркод |
| **В6** | Аксіома збереження сенсу (константа) | **Хибна.** Правильний закон — **Data Processing Inequality**: `I(еталон;вихід)` — супермартингал, монотонно спадає під внутрішньою рефлексією, росте лише від зовнішнього заземлення. Леджер ентропії — **бюджет** (`0≤D≤H_max`), не `Σ=0` | Ренормалізатор = rate-distortion@0; ентропійний леджер на integer-бітах money.rs, не ledger.rs Σ=0 |
| **В7** | Генератор⊥критик (гармонічна спряженість); PID-регулятор | Ортогональність = вимірна умова **∂q/∂v=0** (paraphrase-invariance). Governor PID з `Kd=1.5` **порушує критерій Джурі** (нестабільний на інтегруючому об'єкті). Крос-модельний критик — математична **необхідність** (self-preference bias B≈0.52) | PID-регулятор перепроєктувати (filtered derivative γ≈0.8, ζ=1); критик — обов'язково інша модель |

**Головний висновок семи виправлень.** Схема v1.0 описувала контур, який **вимірює себе собою** у чотирьох
критичних точках (cosine-манометр, PCA-моди, внутрішня згода критика, «збереження» сенсу). Математика в
кожній з цих точок каже те саме: **внутрішнього сигналу недостатньо — потрібна зовнішня земля**. Це не
філософія: це Data Processing Inequality (В6), це необхідність декорельованого критика (В7), це домінування
зовнішнього verify над персистентністю (В5), це неможливість сертифікувати контракцію на інтейку (В4).
L5-архітектура bebop уже втілює цей принцип у своєму девізі **«advisor proposes, kernel decides»** — і саме
тому вона є правильним фундаментом, а не схема сама по собі.

---

## 1. ІНВЕНТАР АКТИВІВ: ЩО ВЖЕ ПОБУДОВАНО У ВАШОМУ L5-СТЕКУ

Перш ніж проєктувати, зафіксуймо ground truth: кібернетична система закритого циклу — це **на 60–70% уже
наявний код**, розкиданий по трьох деревах (`/root/dowiz`, `/root/bebop-repo/crates/bebop`,
`/root/bebop-repo/bebop2`). План v2.0 — це переважно **інтеграція + виправлення дефектів + добудова 3–4
відсутніх примітивів**, а не будівництво з нуля.

### 1.1 Матриця активів (кожен рядок — реальний код, з оцінкою готовності до ролі в контурі)

| Роль у контурі (P&ID) | Наявний код | Файл | Готовність | Головний дефект/gap |
|---|---|---|---|---|
| **Замкнутий регулятор** (насос+тріада+сепаратор) | `resonator.rs` / `resonator.ts` | bebop2 core; dowiz agent-governance | **90%** (control shape) | **Виправлено (перевірено 2026-07-19):** Rust-версія зареєстрована в `lib.rs` (`#[cfg(feature="host")] pub mod resonator;`, підтверджено на `bebop-repo` `main`, 544 рядки, 6 тестів, `cargo check -p bebop2-core` чисто) — більше НЕ мертвий код; TS-порт vs Rust розходження (4 місця) не переперевірялись цим проходом |
| **Стабілізатор / bounded envelope** | `stabilizer.rs` (Lyapunov freeze, tanh-saturate, forbidden-wall, σ-gate) | crates/bebop | **85%** | dt≤0 fail-open (D1); freeze flapping без dwell (D16) |
| **Field veto** (червона лінія фізикою) | `field.rs` + rust-core CSR heat-kernel | crates/bebop; rust-core | **90%** (blast-radius) | keyword→node bypassable («s3cret»); toy 6-node graph |
| **WORM-еталон / tamper-evident** | `audit.rs` (SHA256 hash-chain) | crates/bebop | **90%** | wiring.rs імпортує СЛАБКИЙ AuditLog (D6) |
| **Чекпойнт-батарея / відкат** | `agentic_git.rs` (content-addressed, verify_integrity) | crates/bebop | **80%** | snapshot lossy (drop salience/layer/attic) |
| **Жива пам'ять / persistence-substrate** | `memory.rs` LivingMemory (attic move+restore) | crates/bebop | **55%** | decay = hash-lottery mod 7 (salience ігнорується) |
| **Детермінований сепаратор / decide-law** | `order_machine.rs` (allowed_next, 3-tier error, fold) | dowiz kernel | **85%** | немає Context-параметра, події = голі стани |
| **Integer-бюджет / леджер** | `money.rs` (i64 minor, i128, half-up) + `ledger.rs` (Σ=0) | dowiz kernel; crates/bebop | **75%** | unchecked arithmetic; dead guards |
| **Спектральне ядро** (DMD, eigenvectors) | `field.rs` LaplacianSpectrum (Jacobi З eigenvectors) + `kalman.rs` (SpectralKalman) | bebop2 core | **70%** | Jacobi СИМЕТРИЧНИЙ → misreports комплексні μ |
| **FFT / circulant / spectral** | `fft.rs` (radix-2, circulant_eigenvalues) | bebop2 core | **100%** (transform) | — |
| **Kalman-згладжування шумних сигналів** | `kalman.rs` (covariance propagation) | bebop2 core | **40%** | немає measurement update (H/R/gain) |
| **VSA / семантична подібність** | `vsa.rs` (bind/unbind FFT), `algebra.rs` (cosine, project) | bebop2 core | **80%** | немає permutation, cleanup memory |
| **Детермінований RNG / відтворюваність** | `rng.rs` (ChaCha20, same seed⇒same stream) | bebop2 core | **100%** | from_seed gated (fail-closed, за задумом) |
| **Signed/tamper-evident checkpoints** | `sign.rs` Ed25519 + `pq_dsa.rs` ML-DSA-65 + TLV | bebop2 core; proto-cap | **95%** | proto-crypto ladder = placeholder |
| **Error-patterns auto-learning** (макро-контур) | `error_patterns.rs` (15 markers, persist) + governance TS mirror | crates/bebop; agent-governance | **70%** | first-occurrence-only; markers hard-coded |
| **Машина станів прогону / оркестрація** | `loops/*.yaml` (12 петель) + registry | dowiz loops | **75% design / 40% enforcement** | preconditions прозові; CERTIFIED статуси ungated |
| **Детермінований done-gate** | `metric-core/run-checks.mjs` (hard/soft, exit-code) | dowiz metric-core | **85%** | check-contracts = echo OK placeholder |
| **Escalation ladder / fuses** | `loop-detector.sh` (N=3 signature) + tier1-3 + hooks | dowiz .claude | **80%** | guard-bash.sh НЕ підключений |
| **LLM-суддя (декорельований)** | `eval-layer/openrouter_judge.py` (окрема модель, temp 0) | dowiz eval-layer | **65%** | judge model не pinned per-run |
| **PID-регулятор authority** | `governor.rs` (Kp=1.4,Ki=0.22,Kd=1.5) | crates/bebop | **75% / НЕСТАБІЛЬНИЙ** | Kd=1.5 порушує Джурі |
| **Active inference / EFE** | `active_inference.rs` + bebop2 `active.rs` | crates/bebop; bebop2 | **50%** | лише perception half (немає policy) |

### 1.2 Що доведеться **добудувати** (примітиви, яких немає в жодному дереві)

Ці 6 примітивів — єдиний справжній «новий код». Усе інше — інтеграція + виправлення.

1. **General real eigensolver (Francis QR)** для несиметричного `r×r` `Ã` — щоб DMD читав **комплексні** μ
   (ротаційні моди дрейфу). Наявний Jacobi лише симетричний. Мінімальне (r≤10, `O(r³)`). [дослідж. R2]
2. **Online DMD rank-1 RLS** (Sherman–Morrison) для оновлення оператора оберту **всередині** циклу без
   зберігання історії снапшотів. Прив'язка до `kalman::matmul/invert`. [R2]
3. **Інтейк-компілятор** `admit(spec) → Result<Witness, IntakeError>` — UNSAT-драбина (структурна →
   arc-consistency → SMT QF_LIA) + DOF/ентропія + purity-scan verify. 1:1 lift `order_machine.rs`
   decide-law. [R3]
4. **Persistence-таблиця з Hungarian matching** + survival-test `D*=⌈log_p α⌉` + re-entry stitching через
   attic. Claim-екстрактор (SPO-канонізація + embedding). [R4]
5. **Ентропійний бюджет-леджер** на integer-бітах: `C(x)=|gzip|·8`, `D_t=clamp₀(D_{t-1}+ΔH_in−ΔH_out)`,
   invariant `0≤D_t≤H_max`. Прив'язка до `money.rs`. [R5]
6. **Ортогонометр + перепроєктований PID**: paraphrase-invariance test (∂q/∂v), corr-Goodhart-детектор
   (Fisher-z + CUSUM), filtered-derivative PID (γ≈0.8) на approval-rate. [R6]

**Оцінка обсягу нового коду:** ~6 модулів, кожен зі своїм RED→GREEN тест-набором. Решта плану —
з'єднання наявних вузлів через один регулятор (`resonator`) і виправлення ~40 задокументованих дефектів.

### 1.3 П'ять місць, де L5-архітектура СИЛЬНІША за схему v1.0 (підняти в ядро плану)

Схема v1.0 не знала про ці конструкції; вони строго сильніші за її відповідники і стають несучими v2.0:

1. **«Advisor proposes, kernel decides»** (`wiring.rs`) — LLM-пропозиція не голосує в рішенні `proceed`;
   вона обрізається bounded envelope (`stabilize_step` tanh-saturate), а рішення про допуск приймає
   **детермінований** кон'юнктивний шар. Схема мала лише «критик оцінює» — це слабше.
2. **Field veto домінує L5-оптимізм** (`field.rs` + `wire()`) — червоні лінії задачі перевіряються
   **незалежним** heat-kernel blast-детектором, який LLM не може переголосувати (`proceed = field==Permit
   ∧ ... — кон'юнкція`). Це строго сильніше за hard-constraints Діріхле.
3. **Fail-closed freeze** (`stabilizer.rs`) — нестабільне поле ⇒ **вся адаптація заморожена** (система
   деградує в READ-ONLY, не вимикається). Сильніша форма запобіжника, ніж ABORT.
4. **Agentic-git пам'яті** (`agentic_git.rs`) — контент-адресований ланцюг снапшотів = чекпойнт-батарея
   **з криптографічною цілісністю** (`verify_integrity` ловить tamper). Сильніше за §4.8 схеми.
5. **Ансамблева σ-незгода** (`stabilizer::consensual_aggregate`) — N паралельних L5-пропозицій; якщо
   std-dev розкиду > поріг ⇒ **ігнор LLM**, падіння в ground_state. Прямий захист від синхронної галюцинації
   ансамблю — у схемі відсутній.

Ці п'ять — **ядро v2.0**: контур керується не «критиком, що оцінює», а **детермінованим шаром, який
обрізає, вето, заморожує і падає в землю**, тоді як LLM залишається лише джерелом пропозицій. Це і є
кібернетика замкнутого циклу в строгому сенсі: контролер (детермінований) + спостерігач (LLM-радник),
розділені так, що радник ніколи не змінює правила гри.

---

## 2. ВИПРАВЛЕНА МАТЕМАТИЧНА ОСНОВА

Це несуча конструкція v2.0. Для кожного стовпа: **(а)** що каже строга математика (з дослідження),
**(б)** точні формули у реалізовній формі, **(в)** який код їх втілює, **(г)** RED-фальсифікатор, що
падає, коли реалізація неправильна. Порядок відповідає семи виправленням §0.

### 2.1 Контракція і збіжність: Банах на правильній метриці + DMD-спектр (В1, В2)

**Модель.** Один повний оберт контуру — оператор `T: X → X` (генерувати → критикувати → супервізувати →
ре-ін'єкція еталону), `x*` — нерухома точка (ідеал), `e_n = d(x_n, x*)` — скалярна похибка.
`resonator.rs` уже емітить саме цей ряд як `checkpoints[].error`.

**(а) Метрика — arccos, не 1−cos.** Банах вимагає **повного метричного простору** і справжньої метрики.
`1 − cos` порушує нерівність трикутника → Банах до неї не застосовний, і оцінка `k` в ній математично
беззмістовна. Використовуємо геодезичну на сфері `S^{d-1}`:

```
d_g(u, v) = arccos(⟨u, v⟩) ∈ [0, π]        — Є метрикою (велике коло)
або хордову  d_c(u, v) = ‖u − v‖ = √(2(1 − cos))  — теж метрика (евклідова на сфері)
```

Обидві **order-preserving** відносно `1−cos` → логіка порядку в `resonator` (best checkpoint, rising
streak) **не міняється**; змінюється лише ratio-арифметика `k̂`, яка стає валідною. Контракцію доводимо
**лише в геодезичній кулі радіуса < π/2** навколо `x*` (уникаємо cut locus / антиподів, де arccos
негладка). Саме ре-ін'єкція еталону `λ` тримає `x_n` у цій кулі.

**(б) Скалярний k̂ — необхідна умова з довірчим інтервалом.** `y_n = ln e_n = ln C + n·ln ρ + η_n`; OLS
на асимптотичному хвості `[n₁,n₂]`:

```
k̂ = exp(b̂),  b̂ = S_ny / S_nn (slope);  se(b̂) = s/√S_nn
ВЕРДИКТ (не точкова оцінка!):  upper(CI_k) = exp(b̂ + t_{α/2,N−2}·se(b̂)) < 1
```

Ламається на `e_n→0` (log-singularity: обрізати `e_n < 10·floor`, floor = `config.delta_threshold`, або
weighted LS `w_n=e_n²`) і non-geometric transients (fit лише монотонний хвіст).

**(в) Достатня умова — ρ(J)<1 через DMD.** Скаляр викидає напрямок; справжній критерій — спектральний
радіус якобіана оберту. Оцінка **online без autodiff** через difference-snapshot DMD (`Δ_n = x_{n+1}−x_n`
скасовує невідомий `x*`, бо `T` афінний біля нерухомої точки: `Δ_{n+1} = J·Δ_n`):

```
X = [Δ_0 … Δ_{m-1}],  X' = [Δ_1 … Δ_m]
X = U Σ V*   (thin SVD, truncate Gavish–Donoho τ = 2.858·σ_median)
Ã = U* X' V Σ⁻¹        (r×r reduced operator, НІКОЛИ не форм A ∈ ℝ^{n×n})
ρ̂ = max_i |λ_i(Ã)|    → lyapunov::spectral_radius(row_major(Ã), r)
```

**Обов'язковий de-bias:** OLS `Ã=YX†` biased-toward-0 (errors-in-variables) → `ρ̂` хибно <1. Fix:
forward-backward `μ_fb = √(μ_f/μ_b)` або TLS-DMD; self-check `ρ_f·ρ_b ≈ 1` на стаціонарному сегменті.

**Код-gap:** `lyapunov::eigenvals` — **симетричний** Jacobi sweep → misreports комплексні DMD-eigenvalues
(2-cycle μ≈−1). Потрібен general real eigensolver (Francis QR на tiny `Ã`) — примітив №1 з §1.2.

**Число k — не ціль, а аларм.** `n(ε,k)=⌈ln ε/ln k⌉`; при сталій ціні оберту менше `k` завжди краще.
Реальний оптимум розв'язує `c'(k)/c(k) = 1/(k ln k)` (cost elasticity) — властивість **цієї** задачі. Band
`[0.5,0.8]` = alarm-межі: `k̂≥1` дивергенція (hard stop), `>0.9` повільно (blow fuse), дуже мале+дорого
overspend.

**(г) КОМПОЗИТНИЙ вердикт «converged»** (кожен пункт спростовний окремим RED-конструктом — non-normal
transient, noise-bias, biased-fixed-point, cosine-mirage, attracting-2-cycle):

```
CONVERGED ⟺  (1) метрика = arccos у кулі <π/2
           ∧ (2) upper(CI_k) < 1 на truncated tail
           ∧ (3) ρ̂ < 1 з de-biased reduced DMD
           ∧ (4) ‖Ĵ‖₂ обмежена (немає non-normal transient, що переб'є fuse)
           ∧ (5) ¬DriftAccumulator::is_chaotic ∧ termination==Converged ∧ final_error < ε
```

Будь-який одиничний пункт spoofable; кон'юнкція — falsifiable вердикт. Усе прив'язане до наявних
примітивів (`resonator.checkpoints`, `is_chaotic`, `Termination`, `lyapunov::spectral_radius`, SVD σ_max).

### 2.2 Спектральні моди дрейфу: DMD, не PCA (В3)

**Що не так зі схемою.** §2.3 v1.0 пропонує «PCA по Δ-embedding, `λ≈1` резонанс». Це математично хибний
інструмент. Підставивши лінійну модель `Δ_k=(A−I)x_k`, коваріація Δ має вигляд `ρ_i(PCA)=|μ_i−1|²·s²`:
PCA **згортає двобічну вісь стійкості на однобічну** `|μ−1|`. Наслідки, кожен доказовий:
- `μ=1.5` (нестабільна) і `μ=0.5` (згасаюча) дають однакове `|μ−1|=0.5` — **не розрізнити, який бік 1**;
- **сліпа до `μ=1`** (акумуляція, Δ-variance=0) — а це і є verbosity/tone-drift, найважливіша мода;
- `C_Δ` симетрична PSD → дійсні власні числа, **немає фази** → сліпа до ротації (комплексні μ).

**Правильний інструмент — DMD** (data-driven Koopman): `μ_i` несе **і модуль** (стійкість), **і фазу**
(частоту). Класифікація:

```
|μ_i| < 1 (σ_i<0)  → damped     — контур сам гасить, ігнор
|μ_i| ≈ 1 (σ_i≈0)  → resonant   — нейтральна акумуляція, монітор
|μ_i| > 1 (σ_i>0)  → UNSTABLE   — геометричний рознос, ВТРУЧАННЯ
λ_i = ln(μ_i)/Δt = σ_i + iω_i    (σ=growth, ω=oscillation frequency)
```

**Реалізація на наявних примітивах:**
- **Path A** (дійсні моди — verbosity, factual decay): симетризувати `Ã_s=(Ã+Ãᵀ)/2` → `field::jacobi_eigen`
  (reuse as-is), `ν_max > 1 ⇒ growth`. Сліпа до ротації, але дешева — перший скрин.
- **Path B** (ротаційні моди — tone oscillation): для `r=2` закрита форма `μ=τ/2±√((τ/2)²−δ)`,
  `|μ|=√det(Ã)`, unstable ⟺ `det(Ã)>1` (без eigensolver!); для `r>2` — Francis QR (примітив №1).
- **Project/damp мода:** `c_i=algebra::project(d, Φ)`; корекція `d − c_iφ_i`; ін'єкція `−φ_i` у промпт
  (напр., якщо `φ_i` вздовж length-observable ⇒ «скоротити»).
- **Online (всередині циклу):** Zhang–Rowley rank-1 RLS: `γ=1/(1+x̃ᵀPx̃)`,
  `Ã_{k+1}=Ã_k+γ(ỹ−Ãx̃)x̃ᵀP`, `P_{k+1}=P−γ(Px̃)(Px̃)ᵀ`; exponential forgetting `P/ρ`; `O(r²)/оберт` —
  **плоско за довжиною циклу**. Примітив №2.

**Verbosity = 1-D DMD.** `L_{k+1}=a·L_k+b` (`L`=довжина); `μ=a`; `a<1` settles `L*=b/(1−a)`, `a=1` ramp,
`a>1` runaway. **Схемний `Ln=len/len` детектор false-alarms** коли `b>0` (settling дає ratio>1) — фікс:
регресувати афінну пару, тестувати `a`, не ratio; augment state `[x;1]` (control-DMD) ловить `μ=1`.

**FFT vs DMD.** FFT (`fft.rs` circulant) = стаціонарний shift-invariant, полюси **на** одиничному колі
`|μ|=1` (limit cycle — tone ping-pong). DMD = нестаціонарний, полюси **поза** колом (акумуляція/розпад).
Правило: `mathx::classify_trajectory` LimitCycle → FFT; Divergent/drift → DMD; FFT валідний ⟺ **усі
`|μ_i|=1`**. DMD строго домінує (дає і ω, і σ).

**RED:** повільна вихідна спіраль `A=ρ·R(θ)`, `ρ=1.02`, `θ=0.05`: DMD `|μ|=√det=1.02>1` UNSTABLE ловить;
PCA-по-Δ ранжує її **останньою** (`‖A−I‖≈0.054`, безпечний згасаючий transient `μ=0.3` має `|μ−1|=0.7`
~13× більше) — відкидає як шум. Assert: `DMD_flag≠∅ ∧ DMD_flag⊄PCA_flag`.

### 2.3 Промпт як крайова задача = Тихоновська well-posedness (В4)

**Скінченний аналог Hadamard.** Не PDE, а **Тихоновська коректність обмеженої мінімізації**:
розв'язок = нерухома точка `F` у feasible-множині `M`. Три умови:

```
F1 SAT   — M(spec) непорожня + objective bounded below       (existence)  ¬F1 → осциляція
F2 DET   — унікальний мінімізатор, residual DOF d ≤ 0         (uniqueness) ¬F2 → random convergence
F3 COND  — verify pure/reproducible/Lipschitz, no knife-edge  (stability)  ¬F3 → chaos/non-terminate
```

Діріхле = immutable reference (`π_R(x)=r*` const), Нейман = drift-cap `‖ΔV‖≤δ`, Робін = adaptive budget
`α·d+β·‖ΔV‖≤B_n`.

**Гейт = одна `admit(spec) → Result<Witness, IntakeError>`** — 1:1 lift `order_machine.rs` decide/fold:
static authority (`T∧P` єдиний), unknown⇒rejected never coerced, fold stop-at-first + return position,
5-tier typed taxonomy зі стабільними кодами. Замінює **прозові** preconditions у `loops/*.yaml`.

**UNSAT-драбина (cheapest-first):**
```
Tier A  структурна O(n): type conflict, empty enum ∩, empty interval a>b, required∧forbidden,
        const∉range, cardinality min>max
Tier B  arc-consistency AC-3 O(#c·|dom|³): domain wipeout ⇒ UNSAT (sound, incomplete)
Tier C  SMT QF_LIA/QF_LRA (decidable, NP): SAT⇒Witness model; UNSAT⇒unsat core;
        TIMEOUT/nonlinear ⇒ Undecidable→human (FAIL-CLOSED: ніколи не claim SAT на timeout)
```

**Under-determined:** DOF `d=|free|−|binding|` (pinned=singleton adm); ентропія `H=Σ log|adm(f)|`
(`H=0 ⟺ singleton ⟺ determinate`); «#M≥2» через AllSAT-2 (model m1 → blocking clause ¬(≈m1) → resolve;
SAT distinct m2 ⇒ under-determined). Placeholder = unpinned required field = `d>0` — те, що ловить M1.

**Чесна межа (В4).** Інтейк вирішує лише **структурну** коректність. Контракція `α` чорного ящика `F` —
властивість runtime, **не декларована specom**, не сертифікована до запуску. Інтейк = necessary filter,
не sufficient stability. Стійкість забезпечує **Нейман-clamp** (§2.7) + runtime Lyapunov-монітор (§2.1).

**RED:** `verify` з impure-oracle (RNG/clock/network/LLM/mutable) проходить F1/F2/структуру, але terminal
predicate = random variable ⇒ немає stable fixed point. Detector: (1) static purity scan (denylist, як
clippy disallowed-methods) + (2) dynamic idempotence probe (`K≥2` evals під perturbed nuisance env, різні
біти ⇒ NonReproducibleVerify) ⇒ FORCE human-bypass, fail-closed.

### 2.4 Персистентність = аналіз виживання, не TDA (В5)

**Чесний вирок.** §2.9 v1.0 — **не** персистентна гомологія (немає симпліціального комплексу, немає
гомології, «фільтрація» немонотонна — claim'и відроджуються). Це **survival/duration filter** над point
process claim'ів. Barcode-грамматика («довга смуга = сигнал») переноситься як scoring-евристика; **теорема
стабільності НЕ переноситься** — і, гостріше, її висновок **хибний**: один sub-threshold парафраз дає
discontinuous death+rebirth (реальна PH provably stable; claim-barcode provably UNSTABLE). Причина:
barcode = функція **генератора**, не input-data — немає `‖f−g‖_∞`, що зв'язує з істиною.

**Що переноситься строго — survival analysis.** Ідентичність claim'ів `t→t+1` = assignment problem:
**Hungarian** max-weight bipartite `O(N³)` (N≈10 trivial), ребра `w=cosine≥τ`, `<τ ⇒ death+birth`. Не
greedy (spurious rebirth). `τ*` = Bayes/equal-error суміші `p(sim|same)/p(sim|diff)`; `NOISE_FLOOR=0.35`
лише floor (замало для identity). **Re-entry stitching обов'язковий:** gap≤w rematch зберігає original
birth — bebop `attic move+restore` = точно цей примітив (non-destructive REQUIRED).

**Кількісний поріг (нуль-модель).** Noise-claim виживає кожен turn незалежно з ймовірністю `p` ⇒
geometric: `P(D≥d)=p^d`, p-value `=p^D`. Сигнал iff `p^D≤α`:

```
D* = ⌈ln α / ln p⌉ = ⌈log_p α⌉        (напр., p=0.5,α=0.05 ⇒ D*=5; α=0.01 ⇒ D*=7)
Bonferroni (N тестів):  D*_Bonf = D* + ⌈log_{1/p} N⌉      (N=100,p=0.5,α=0.05 ⇒ 11)
POWER CEILING:  D ≤ n−1 ⇒ certifiable лише якщо n ≥ D*+1   (Bonferroni N=100 ⇒ n≥12)
                нижче — ABSTAIN «insufficient iterations», НЕ «no signal»
```

`p̂` = geometric MLE `mean_span/(mean_span+1)` або 2-component EM (noise geometric + core heavy-tail).

**Anomaly formalized** (§2.9 v1.0 «пізня стійка ознака»): у `(birth, duration)`-площині (shear of
birth-death) `CORE={b<b_thr, D≥D*}`; `ANOMALY={b≥b_thr, D*≤D≤n−1−b}` (трикутник, не rectangle:
feasibility `b_thr≤n−1−D*`); `b_thr=⌈β·n⌉`, `β∈(0.5,0.8]`. Emergence vs entrenched-hallucination —
геометрія **не** розрізняє → verify/human, ніколи auto-accept.

**Пам'ять = фільтр.** bebop mod-7 hash-eviction **математично неправильний** (`MI(kept,persistence)=0`,
нульова фільтрація). Правильно: `keep iff persistence_score(c) > θ=D*`, else attic (non-destructive). Soft
online-естиматор: `s_{t+1}=s_t·exp(−Δt/τ)+boost·seen`; half-life `t_½=τ·ln2`; min-salience PQ eviction;
`τ≈n/ln(s_max/s_min)`. **Три наявні bebop-примітиви** (salience field, attic move, restore) → коректний
фільтр **одним рядком** зміни decay-логіки.

**RED (найважливіший для всієї системи).** Early-born false claim `h`; autoregressive loop re-emits його
кожен turn ⇒ `D=n−1` MAX, p-value `=p^{n−1}` найзначніший ⇒ фільтр labels `h` як CORE + pins max salience
**реінфорсить помилку**. Persistence вимірює **generator fixed points, не world**; вісь
generator-stable/transient **ортогональна** true/false. Тому:

```
КОМПОЗИЦІЯ (AND-gate):  Accept(c) = P(c) ∧ V(c)
  P = persistence filter (rejects generator NOISE, cheap, no oracle) — ADVISORY prefilter
  V = external verifier   (tests/schema/typecheck/fact/cross-model)  — AUTHORITY, decides truth
```

Небезпечна множина `{stable}∩{false}` = entrenched hallucination — тільки `V` ловить. §2.9 v1.0
«anomaly→human» правильне, але **under-scoped** (ловить лише пізні; early-persistent hallucination —
найгірша — сходить з max score). `V` мусить домінувати **всі** persistent claims. Bind: `enrich.rs`
confidence gate + `agentic_git.verify_integrity` (tamper-evidence на record того, що верифіковано).

### 2.5 Ентропійний бюджет = DPI + integer-леджер (В6)

**Аксіома збереження сенсу — математично хибна.** Data Processing Inequality: Markov `X→Y→Z ⇒
I(X;Z)≤I(X;Y)`; post-processing **не створює** інформації. При внутрішній рефлексії
`R→x_0→x_1→…→x_t` — Markov-ланцюг, тому:

```
I(R; x_t) ≤ I(R; x_{t-1}) ≤ … ≤ I(R; x_0)     — сенс МОНОТОННО СПАДАЄ
I(R; x_t) = I(R; x_0) − L_t + G_t
  L_t ≥ 0 (processing loss, DPI — кожен внутрішній оберт)
  G_t ≥ 0 (grounding injection — ЛИШЕ зовнішнє O_t ⟂̸ R: retrieval/tool/verify/human)
```

Це **точно** Huang et al. ICLR 2024: intrinsic self-correction без зовнішнього сигналу не може додати
`I(R;·)` — DPI забороняє. «Збереження сенсу» ⇒ **«монотонний витік сенсу, поповнюваний лише заземленням»**
(відкрита дисипативна система). Це і є математичне обґрунтування Аксіоми 3 схеми (зовнішнє заземлення) —
але як **теорема**, не постулат.

**Яка ентропія — компресійна довжина.** Order-0 Shannon (`knowledge.rs`) **дискваліфікована** (maximized
by noise). Два види «heat»: **verbosity heat** = `ΔC` (compression-length delta, `C(x)=|gzip|·8` bits,
integer, deterministic) → дренує **ренормалізатор**; **confabulation heat** = semantic entropy `SE`
(кластери за bidirectional entailment, Farquhar/Gal Nature 2024) → дренує **лише зовнішнє заземлення**.

**Ренормалізатор = rate-distortion @ D=0 на claim-manifold:**
```
R(x) = argmin_{x̂} L(x̂)  s.t.  claims(x̂) = claims(x);   floor = H(claims)
Алгоритм: C0 = claims_extract(x) [ОКРЕМИЙ non-generative verifier]; x' = LLM_rewrite;
RED: C1⊉C0 (dropped)⇒rollback; C1⊋C0 (new/hallucination)⇒rollback; L(x')≥L(x)⇒no-op; idempotent
```

**Леджер = БЮДЖЕТ, не Σ=0** (ентропія не conserved: source + external sink). Bind `money.rs` integer bits:
```
D_t = clamp₀(D_{t-1} + ΔH_in − ΔH_out),  D_0=0;  invariant  0 ≤ D_t ≤ H_max
ΔH_in = max(0, L(x_raw) − L(x_{t-1}));  ΔH_out = max(0, L(x_raw) − L(R(x_raw)))
ALARM D_t > H_max ⇒ HeatOverflow → force hard-renorm / re-ground / halt
```
Bind `ledger.rs` machinery (SHA id, idempotent replay, reject-on-violation) АЛЕ invariant `0≤D_t≤H_max`,
**не** `Σ=0`.

**Стаціонарність (нескінченна робота).** `D_t` — random walk з дрейфом `μ_t=E[ΔH_in]−E[ΔH_out]`.
Строго від'ємний `μ≤−δ<0` ⇒ nonneg supermartingale **CONVERGES a.s.** (Foster–Lyapunov ergodic,
`E[D_∞]≤c²/2δ`); positive ⇒ `D_t→∞` overflow a.s.; **zero `μ=0` null-recurrent STILL overflows a.s.**
(потрібна СТРОГА негативність). Умова живучості: `⟨ΔH_out⟩ > ⟨ΔH_in^gen + ΔH_in^crit⟩`.

**RED:** `H(out)≤H(in)` alone gamed by `R(x)=""` (max reduction, `I=0`). DPI: `I(claims;x̂)≤I(claims;x)`
ceiling; pin floor `I(claims;x̂)≥I(claims;x)` ⇒ pincer to EQUALITY. `R_cheat` що дропає persistent claim
(price line / allergen / negation polarity): `L↓` але `claims⊉` ⇒ guardrail MUST reject. Gate = **H↓ ∧
claim-set-equality**, ніколи H↓ alone.

### 2.6 Ортогональність генератор⊥критик + перепроєктований PID (В7)

**Ортогональність — вимірна умова ∂q/∂v=0.** Коші-Ріман discharged: `u(x)=cos(x,r)` (meaning coord),
`v(x)` (style). Генератор тече `⟂∇u` (`g∈ker(Du)`); ортогонально ⟺ `⟨g,∇q⟩=0 ⟺ ∇q∥∇u ⟺ ∂q/∂v=0` (критик
інваріантний вздовж DOF генератора). Виродження = `∇q` набуває `v`-компоненти («агенти згоджуються у своїх
помилках»).

**Естиматор (paraphrase-invariance):** `e_⊥=e_style−proj_∇u(e_style)` (Gram-Schmidt, `algebra::project`);
для binary `q`: `N` meaning-preserving парафраз, `flip_rate=(1/N)Σ1[q(x'_i)≠q(x)]` orthogonal ⟺ `≈0`
(meaning const ⇒ будь-яка variance = `∂q/∂v≠0`); enforce `|ρ_gc|≤τ_orth≈0.1`. `resonator.reflect` бачить
output+reference, **не reasoning** — structurally good, але не гарантує `∂q/∂v=0`; тест — enforcement.

**Goodhart-детектор corr(q, Δs).** `q_t=a·s_t+b·style_t+ε`; healthy `a>0,b=0`⇒corr>0; degenerate `b≠0`
⇒corr→0; reward-hack ⇒corr<0. Diff-series `r_Δ=corr(Δq_t,Δs_t)` sliding W; point-biserial для binary;
Fisher-z lower bound `r_lower=tanh(artanh(r̂)−z/√(W−3))` ALARM `r_lower<r_min≈0.3–0.5`; CUSUM
`C_t=max(0,C_{t-1}+(r_min−r_t))>h`. Це = перетин reward-overoptimization point `d*` (Gao–Hilton–Schulman,
`R_gold(d)=d(α−βd)` peak `d*=α/2β`, `d≈total_drift`). **`s_t` МУСИТЬ бути held-out/decorrelated** — не той
метрик, що loop оптимізує.

**Крос-модельний критик — математична необхідність.** `Q_g=Q*+η_shared+η_g`, `Q_c=Q*+η_shared+η_c`;
**reward-hack headroom = `Cov(Q_g,Q_c)−Var(Q*) = Var(η_shared)`**. hacking-gain `≤ Var(η_shared)`; `=0 ⟺
Cov(η_g,η_c)=0 ⟸ independent models`. Self-preference bias `B=P(Y'=1|S=1,Y=1)−P(Y'=1|S=0,Y=1)`; GPT-4
`B≈0.520` (Wataoka arXiv:2410.21819; judge scores lower-perplexity/self-like higher `∝−log p_judge` = max
`η_shared`). `eval-layer/openrouter_judge.py` (окрема модель, temp 0) = **correct decorrelation surface**.

**PID нестабільний — перепроєктувати.** `governor.rs` `Kd=1.5` на інтегруючому об'єкті `G=Kg/(z−1)`:
char `z³+a₂z²+a₁z+a₀`, `a₀=Kg·Kd`. Критерій Джурі: `|a₀|=1.5>1` **ПОРУШЕНО**; `Kg(2Kp+Ki+4Kd)=9.02≥4`
**ПОРУШЕНО** — **дві умови fail, контур нестабільний**. Static-plant margin лише `0.053` (весь з'їдений
Kd). Binary derivative kick: `e_t∈{−0.9,+0.1}`, flip `Δe=±1.0`, D-term `=±1.5` > весь authority budget
[0,1] ⇒ bang-bang; один reject slams authority to 0.

**Фікс:**
```
Filtered derivative:  D(z) = (1−γ)·Kd·(z−1)/(z−γ),  pole z=γ;  γ≥0.765 caps kick≤0.2;  γ≈0.8
Act on approval-rate p_t ∈ [0,1]  (НЕ raw {0,1} verdicts — D well-posed лише на p_t)
Critical damping ζ=1:  Kp_crit = 2√(Ki(1+KgKd)/Kg) = 1.483   (governor ζ≈0.944, 6% under-damped)
                       Q = 1/(2ζ);  ζ<1 oscillatory; ζ>1 converges-to-banality; ζ=1 fastest non-overshoot
Gain scheduling = annealing:  M₁ hard ⊂ M₂+soft ⊂ M₃+style;  β(p)∝1/|M(p)|;  E(p)=Kp(p)|M(p)| bounded
                              (style ніколи не out-competes hard = scheduling-level orthogonality)
```

**RED:** adversarial `G*: x_{t+1}=x_t+α·e_v−γ·e_u` (climbs style rewarded, drifts off ref) + `C*` same
model B≈0.52 ⇒ `q↑` monotone while `s=cos(x,r)↓` ⇒ `corr(q,Δs)<0`. Detector 3 gates (any fires): (1)
`r_Δ<0` internal; (2) external-verify `V` decorrelated dominance (accept iff `q↑ AND V non-decreasing`,
else `rollbackToBest`); (3) paraphrase-invariance floor `ρ_gc>τ_orth ⇒ freeze` (`lyapunov_guard`). Runnable:
RED `q_t=t,s_t=−t⇒r_Δ=−1` fire; GREEN `q_t=s_t+noise⇒r_Δ>0.9` pass.

### 2.7 Нейман-clamp: єдиний runtime-гарант стійкості, який інтейк дати не може

Дискретна Нейман-умова `∂u/∂n=g` = per-step cap на приріст. `max_delta_embedding_per_iter=δ` — точний
discrete-normal-derivative cap. Clamp = **евклідова проєкція на δ-кулю** (= bebop `saturate`/`io_guard`):

```
V_clamped = V(x_n) + s · min(1, δ/r),   s = V(x̃)−V(x_n),  r = ‖s‖    (L2; saturate = L∞)
```

1-Lipschitz nonexpansive → **тільки зменшує** conditioning, ніколи не збільшує. Length-cap так само. Робін:
`‖ΔV‖≤(B_n−α·d)/β` — крок стискається з накопиченням дрейфу; budget exhausted ⇒ freeze pure Діріхле.
**Це і є те, що робить контракцію α чорного ящика керованою на runtime** — перетворює unknown-Lipschitz
map на rate-limited. Інтейк = detection; clamp = enforcement.

---

## 3. ЦІЛЬОВА АРХІТЕКТУРА: ШІСТЬ ШАРІВ КОНТУРУ

Система розкладається на шість шарів. Кожен шар = **наявний кодовий актив + добудова**, і кожен реалізує
конкретну частину гідравлічного P&ID. Розділення шарів відповідає принципу «контролер (детермінований) /
спостерігач (LLM-радник)»: L0–L2 і L4–L5 — детерміновані (математика, код, гейти); лише L3 містить LLM,
і той обрізаний bounded envelope.

```
┌──────────────────────────────────────────────────────────────────────────────────────┐
│ L5  МЕТА-КОНТУР (тижні, асинхронний)  — еволюція еталонів (Робін) + error-patterns      │
│     harvest + Council retro. Людське вето. Односторонній ентропійний фільтр вгору (DPI) │
│     Код: error_patterns.rs (70%) + loops registry + librarian-патерн. Add: eталон-diff  │
├──────────────────────────────────────────────────────────────────────────────────────┤
│ L4  ОРКЕСТРАЦІЯ (машина станів прогону)  — INTAKE→PRIMED→SPIN→{CONVERGED|ABORT|BRANCH|   │
│     BYPASS}→DELIVER + манiфольд гілок + асинхронна вежа + fuses + escalation ladder     │
│     Код: loops/*.yaml (75%) + metric-core (85%) + tier1-3 + hooks (80%). Add: admit()   │
├──────────────────────────────────────────────────────────────────────────────────────┤
│ L3  АКТУАЦІЯ (єдиний LLM-шар)  — Генератор (T-annealed) + Критик (крос-модель, ∂q/∂v=0) │
│     під BOUNDED ENVELOPE (tanh-saturate) + ансамбль σ-незгода ⇒ ігнор                   │
│     Код: wiring.rs L5-gate (85%) + stabilizer.rs (85%) + eval-layer judge (65%)         │
├──────────────────────────────────────────────────────────────────────────────────────┤
│ L2  ВИМІРЮВАННЯ (панель приладів)  — DMD k-метр (ρ через spectral_radius) + arccos      │
│     манометр + persistence survival-test + ентропійний budget + ортогонометр +          │
│     Kalman-згладжування. Код: field.rs/kalman.rs/fft.rs/algebra.rs (70%). Add: online DMD│
├──────────────────────────────────────────────────────────────────────────────────────┤
│ L1  ПАМ'ЯТЬ («земля»)  — WORM-еталон (hash-chain) + persistence-таблиця claims (Hungarian│
│     +survival) + agentic-git чекпойнт-батарея. Код: audit.rs (90%) + agentic_git.rs (80%)│
│     + memory.rs (55%). Fix: destructive-decay → salience-weighted; wiring weak-AuditLog  │
├──────────────────────────────────────────────────────────────────────────────────────┤
│ L0  ФІЗИКА (детермінований kernel)  — сепаратор (decide-law) + запобіжники + integer-    │
│     бюджет + T-scheduler + чекпойнт-дерево + field-veto (heat-kernel blast) + Нейман-clamp│
│     Код: order_machine.rs (85%) + money.rs (75%) + field.rs veto (90%) + resonator (90%) │
└──────────────────────────────────────────────────────────────────────────────────────┘
       ▲ земля всіх шарів = WORM-еталон (L1); манометри кожного холона заземлені до неї
```

### 3.1 L0 — Фізика (детермінований kernel)

**Роль:** усе, що приймає рішення, — тут, і жоден LLM сюди не входить. Це «фізичний закон» контуру.

- **Сепаратор** = `order_machine.rs` decide-law, узагальнений з `OrderStatus` на стадії прогону
  (`Drafted→Critiqued→Refined→{Accepted|Rejected|Branch}`). Статична таблиця `allowed_next` — **єдина**
  legality-authority; типізована 3-tier помилка; `fold_transitions` зупиняється на першому illegal +
  повертає позицію. Фінальне рішення про збіжність (§2.1 композитний вердикт) — **детермінований код за
  показами приладів**, LLM постачає лише один сигнал.
- **Запобіжники** = `stabilizer` fuses + `loop-detector` N=3 + token-fuse ДО виклику API + timeout. Усі —
  детермінований код, жодного LLM.
- **Integer-бюджет** = `money.rs` дисципліна: токени = minor units, ентропія = біти (§2.5), half-up в
  одному місці, `assert_non_negative`. Виправити unchecked arithmetic (checked_mul/checked_add).
- **T-scheduler** (редукційний клапан §4.6 схеми) = annealing `T(iter,role)`: gen 0.9→0.6→0.3, критик 0.1,
  валідатор 0.0, гілки {0.5,0.8,1.0}. Синхронізований з gain-scheduling L3 (§2.6 B6).
- **Field-veto** = `field.rs` heat-kernel blast (виправити keyword-map на семантичний класифікатор +
  реальний граф залежностей замість toy-6-node).
- **Нейман-clamp** (§2.7) = проєкція на δ-кулю перед кожним commit.

### 3.2 L1 — Пам'ять («земля»)

**Роль:** єдина точка істини, до якої заземлені манометри всіх холонів. Реалізує аксіому зовнішнього
заземлення строго (DPI §2.5: сенс росте лише звідси).

- **WORM-еталон** = `audit.rs` SHA256 hash-chain (**не** слабкий `research_patterns::AuditLog` — виправити
  D6 у wiring.rs). Дві форми: повний текст (ре-ін'єкція) + вектор `V_ref` (манометри). Незмінний протягом
  прогону (hash-lock); еволюція лише між прогонами через L5 з людським підписом.
- **Persistence-таблиця** (§2.4) = Hungarian matching + survival `D*` + re-entry через attic. Замінює
  наївний claim-tracking. Три наявні bebop-примітиви (salience, attic move, restore) → фільтр одним рядком.
- **Чекпойнт-батарея** = `agentic_git.rs` content-addressed ланцюг з `verify_integrity` (виправити lossy
  snapshot: додати salience/layer/attic у стан). Політика: best-so-far (діод) + last-2 + fork-корені.

### 3.3 L2 — Вимірювання (панель приладів)

**Роль:** перетворити шумні LLM-сигнали на детерміновані числа з довірчими інтервалами. Це те, що робить
контур **спостережуваним** (без цього немає керування).

- **k-метр** = online DMD `ρ̂` через `lyapunov::spectral_radius` (§2.1/2.2), НЕ регресія, НЕ PCA.
- **Манометр similarity** = `arccos(cos(V(x),V_ref))` (§2.1), НЕ `1−cos`.
- **Persistence survival-test** = `D*=⌈log_p α⌉` + Bonferroni (§2.4).
- **Ентропійний budget** = `D_t=clamp₀(...)`, `0≤D_t≤H_max` (§2.5).
- **Ортогонометр** = `corr(Δq,Δs)` Fisher-z + CUSUM + paraphrase flip_rate (§2.6).
- **Kalman-згладжування** = `kalman.rs` (додати measurement update H/R/gain для fusion шумного
  quality-сигналу — примітив розширення; 40%→80%).

### 3.4 L3 — Актуація (єдиний LLM-шар, під bounded envelope)

**Роль:** джерело пропозицій. LLM тут — виконавчий механізм, обрізаний детермінованим envelope. Тріада:

- **Генератор** (T=0.7–1.0 annealed): бачить task+inputs+constraints+фідбек, НЕ бачить рубрики критика
  (анти-Гудхарт).
- **Критик** (крос-модель, T≈0.1): бачить вихід+еталон, НЕ бачить reasoning генератора; рубрика в
  reference-terms (pins ∇q∥∇u); paraphrase-invariance enforced. Обов'язково **інша модель** (§2.6 A3).
- **Bounded envelope** = `stabilize_step` tanh-saturate: LLM-дельта обрізається до max-delta/tick;
  ансамбль σ-незгода > поріг ⇒ ігнор LLM, падіння в ground_state. Fail-closed freeze на нестабільне поле.

### 3.5 L4 — Оркестрація (машина станів прогону)

**Роль:** склеює вузли в один прогін; керує ритмом, гілками, вежею, запобіжниками, ескалацією.

- **Машина станів** = `loops/*.yaml` схема + `metric-core` done-gate (детермінований, exit-code).
- **Intake-компілятор** = `admit()` (§2.3) — замінює прозові preconditions.
- **Манiфольд гілок** (§4.11 схеми) = N гілок від чекпойнта з різними T/роллю/seed; спільний сепаратор;
  Оккам-тайбрейкер; архів для graft.
- **Асинхронна вежа** = tier2-overnight патерн (pre-computation у фон, роздача з кешу).
- **Escalation ladder** = `loop-detector` N=3 → self-divergence → specialist → stronger model → council →
  human. **Виправити:** підключити guard-bash.sh.

### 3.6 L5 — Мета-контур (еволюція еталонів)

**Роль:** повільний зовнішній контур, що еволюціонує еталони (умова Робіна) між прогонами. Людське вето.

- **Еволюція еталонів** = журнал дерева прогонів → які класи задач збігаються за 2 оберти, де систематично
  рветься verify → оновлення еталонів (з людським підписом, ніколи всередині кільця — bounded
  self-modification).
- **Error-patterns harvest** = `error_patterns.rs` (виправити first-occurrence-only + hard-coded markers).
- **Council retro** = cause-critic + pattern-critic + ratchet-critic → ратчет-артефакти (guardrails).

**Односторонній ентропійний фільтр вгору** (ієрархія холонів §12 схеми, тепер строго = DPI): кожен рівень
пропускає нагору **лише низькочастотну стабілізовану складову** (персистентні ознаки, агреговані метрики),
а високочастотний шум гасить у собі. Це буквально Data Processing Inequality між рівнями: агрегація —
lossy channel, тому вгору йде тільки те, що пережило усереднення. Верхні рівні повільні/асинхронні; нижні
швидкі/синхронні; жоден не втручається в частотну смугу іншого (та сама конструкція, що робить літак
керованим: швидкі внутрішні контури стабілізації + повільний зовнішній контур навігації).

---

## 4. ПОВНИЙ P&ID КОНТУРУ v2.0 (ВИПРАВЛЕНИЙ)

```
                          ІН'ЄКЦІЯ ТВОРЦЯ (єдиний вхід)
                                    │
                                    ▼
                    ┌───────────────────────────────┐
                    │ L4: admit(spec) (§2.3)        │  UNSAT-драбина + DOF + purity-scan
                    │ Тихоновська well-posedness    │  → REJECT/Undecidable→human ДО насоса
                    └───────────────┬───────────────┘
                                    │ Witness (компільований еталон)
              ┌─────────────────────┴──────────────────────┐
              ▼                                            │ ре-ін'єкція еталону
   ╔══════════════════════════╗                            │ до КОЖНОГО вузла напряму
   ║ L1: WORM-ЕТАЛОН (audit.rs)║════════════════════════════╡ («швидка дзиґа» §2.8:
   ║ SHA256 hash-chain, V_ref  ║══════════╗                 │  ω=частота ре-ін'єкції,
   ╚══════════════════════════╝          ║                 │  I=повнота, τ=збурення)
                                          ║                 │
   ┌── ОСНОВНИЙ КОНТУР (resonator.rs, за год. стрілкою) ────┼──────────────────┐
   │                                      ║                 │                  │
   │  ┌──────────┐  ┌─────────────┐  ┌────────────────┐     │                  │
   │  │ L0 НАСОС │─▶│ L0 T-scheduler│▶│ L3 ГЕНЕРАТОР   │     │                  │
   │  │ такт+    │  │ annealing    │  │ T=0.9→0.6→0.3  │     │                  │
   │  │ mutex    │  │ 0.9→0.3      │  │ (bounded env)  │     │                  │
   │  └────┬─────┘  └─────────────┘  └───────┬────────┘     │                  │
   │       ▲                                 │              │                  │
   │       │                                 ▼              │                  │
   │       │                        ┌──────────────────┐    │                  │
   │       │                        │ L3 КРИТИК        │◀═══ ре-inject (constraints,
   │       │                        │ КРОС-МОДЕЛЬ T=0.1 │    │  schema, verify)
   │       │                        │ ∂q/∂v=0 enforced │    │                  │
   │       │                        │ tanh-saturate    │    │                  │
   │       │                        │ σ-незгода⇒ignore │    │                  │
   │       │                        └───────┬──────────┘    │                  │
   │       │                                │               │                  │
   │       │         ┌──────────────────────┤               │                  │
   │       │         ▼                       ▼               │                  │
   │  ┌──────────┐ ┌──────────────┐  ┌──────────────────┐    │                  │
   │  │L0 ЗВОРОТ.│ │L0 РЕНОРМАЛІЗ.│  │ L0 СЕПАРАТОР     │◀═══ ре-inject (verify,
   │  │КЛАПАН    │◀┤rate-distort@0│◀─┤ decide-law       │    │  drift_limits)
   │  │best ckpt │ │H↓∧claims=    │  │ КОМПОЗИТ 5-clause│    │                  │
   │  │(діод)    │ │ preserved    │  │ (§2.1)           │    │                  │
   │  └────┬─────┘ │ENTROPY BUDGET│  │ Vn ПЕРШЕ, потім  │    │                  │
   │       │       │0≤D_t≤H_max   │  │ внутр. метрики   │    │                  │
   │       │       │→дренаж назовні│ └──┬──────┬────────┘    │                  │
   │       │       └──────────────┘    │      │             │                  │
   │       │          ще оберт ◀────────┘      │ ЗБІГСЯ      │                  │
   │       └───────────────────────────┐       │ (5-clause) │                  │
   │                                   │       ▼            │                  │
   └────────────────────────────────────┼──────┼────────────┘                  │
                                        │      ▼                               │
   L2 ПАНЕЛЬ ПРИЛАДІВ (щооберту):        │  ┌──────────────┐                    │
   • DMD k-метр ρ̂ (spectral_radius)     │  │ L4 КЛАПАН    │                    │
   • arccos манометр (НЕ 1−cos)         │  │ ВИДАЧІ       │──▶ ПРОДУКТ          │
   • persistence survival D*            │  │ best ckpt +  │  (єдиний вихід)     │
   • ентропійний budget D_t             │  │ журнал дерева│                    │
   • ортогонометр corr(Δq,Δs)           │  └──────────────┘                    │
   • Kalman-згладжування                │                                      │
                                        │                                      │
   ДОПОМІЖНІ (поза кільцем):             │                                      │
   ┌────────────────────┐ ┌─────────────┴──────┐ ┌──────────────────────────┐   │
   │ L1 РОЗШИР. БАК     │ │ L4 ЗАПОБІЖНИКИ     │ │ L4 БАЙПАС ЛЮДИНИ         │◀──┘
   │ scratch, дренується│ │ max_iter/token-fuse│ │ (3 тригери §4.9:         │
   │ ренормалізатором   │ │ ДО API/timeout/    │ │  late-persist anomaly,   │
   │                    │ │ divergence ABORT   │ │  non-repro verify,       │
   │ L1 ЧЕКПОЙНТ-БАТАРЕЯ│ │ guard-bash (FIX!)  │ │  manual stop)            │
   │ agentic_git verify │ │ escalation N=3     │ │                          │
   └────────────────────┘ └────────────────────┘ └──────────────────────────┘

   L4 МАНiФОЛЬД ГІЛОК: N суб-контурів від чекпойнта {T=0.5,0.8,1.0} → спільний сепаратор
   (одна verify, чесне змагання на землі) → Оккам-тайбрейкер → злиття/архів для graft

   L5 МЕТА (тижні, async): журнал дерева → error-patterns harvest → еволюція еталонів
   (Робін, людське вето) → односторонній ентропійний фільтр вгору (DPI)
```

**Порядок оберту:** Насос (такт, mutex) → T-scheduler (annealing фази) → Генератор (розширення, bounded) →
Критик (крос-модель, ортогональне стискання, σ-незгода) → Сепаратор (КОМПОЗИТНИЙ 5-clause вердикт: спершу
зовнішній `verify`, потім внутрішні DMD-метрики) → [якщо ще оберт] Ренормалізатор (rate-distortion@0,
дренаж ентропії, claim-preserving) → Зворотний клапан (best-checkpoint діод) → назад до насоса. Еталон
живить кожен вузол напряму (не циркулює кільцем).

---

## 5. ДЕФЕКТИ КОДУ, ЯКІ МУСИМО ВИПРАВИТИ ПЕРШИМИ (пріоритезовано)

Аналіз коду виявив ~40 дефектів. Не всі критичні для контуру. Ось ті, що **блокують** правильну роботу
кібернетичної системи, у порядку пріоритету. Кожен має RED→GREEN гейт.

### 5.1 🔴 Червоний клас (блокують математичну коректність — виправити ДО будь-якої інтеграції)

1. ~~**`resonator.rs` не зареєстрований у `lib.rs`** (bebop2) — єдиний оркестратор стеку = **мертвий
   код**, 6 тестів не бігають.~~ **ЗАКРИТО (перевірено 2026-07-19):** `#[cfg(feature="host")] pub mod
   resonator;` вже стоїть у `bebop2/core/src/lib.rs` на `main` гілки `bebop-repo`; `resonator.rs`
   компілюється (`cargo check -p bebop2-core` чисто) і несе 6 `#[test]`. Точний комміт, що це
   привніс, не встановлено (історія `bebop-repo` `main` була переписана через "clean-slate publish"
   — `f38f2c5`), але поточний стан дерева підтверджений живо, не з пам'яті. Rust-оригінал тепер
   доступний як reference, до якого можна звіряти TS-порт — це звіряння (4 задокументовані
   розходження) цим проходом НЕ виконувалось.
2. **`1−cos` замість `arccos`** (knowledge.rs, resonator метрика) — Банах беззмістовний (§2.1 В1). Замінити
   метрику similarity на `arccos` або хордову. Ordering-логіка не міняється. RED: тест, де `1−cos` спадає,
   а `arccos` росте (cosine-mirage) — має ловитись.
3. **`lyapunov::eigenvals` симетричний Jacobi** — misreports комплексні DMD-eigenvalues (2-cycle μ≈−1,
   ротаційні моди §2.2). Додати Francis QR для несиметричного `Ã`. RED: attracting 2-cycle → `ρ̂≈1` має
   ловитись, а симетричний Jacobi його пропускає.
4. **coherence.rs / field_active знак дифузії** (D1/D3) — інтегрують `+cLu` (анти-дифузія `exp(+Lt)`) при
   заявленому `exp(−Lt)`. Використовувати **виключно** Chebyshev spectral path (`field_spectral`,
   правильний знак). RED: mass-conservation тест на дифузії — анти-дифузія його порушує.
5. **Governor PID Kd=1.5 порушує Джурі** (§2.6 В7) — нестабільний на інтегруючому об'єкті. Перепроєктувати:
   filtered derivative γ≈0.8 на approval-rate, Kp→1.48 (ζ=1). RED: `q_t=t, s_t=−t ⇒ r_Δ=−1` детектор має
   fire; бінарний kick ±1.5 > budget має бути обрізаний.
6. **Ентропійний леджер: `Σ=0` замість бюджету** (§2.5 В6) — ентропія не conserved. Використати `money.rs`
   integer-біти з invariant `0≤D_t≤H_max`, НЕ `ledger.rs` Σ=0. RED: `R_cheat` що дропає claim — `L↓` але
   `claims⊉` має rejected.

### 5.2 🟠 Помаранчевий клас (security/correctness — виправити перед прод-довірою)

7. **wiring.rs імпортує СЛАБКИЙ AuditLog** (D6) — «tamper-evident ledger» насправді plain Vec без
   hash-chain. Переключити на `audit::AuditLog` (SHA256). RED: mutated entry має fail verify.
8. **memory.rs decay = hash-lottery mod 7** (salience ігнорується) — `MI(kept,persistence)=0`, нульова
   фільтрація (§2.4). Замінити на salience-weighted min-PQ eviction. RED: high-persistence claim і
   length-1 noise мають РІЗНУ ймовірність eviction.
9. **field.rs keyword-veto bypassable** («s3cret» обходить) — семантичний класифікатор + реальний граф
   залежностей замість toy-6-node. RED: obfuscated red-line task має все ще vetoed.
10. **guard-bash.sh не підключений** — dangerous-command veto (deploy/git-push-main/rm-rf) мертвий.
    Підключити в settings.json. RED: `fly deploy` у sandbox має block.
11. **agentic_git snapshot lossy** — дропає salience/layer/attic → rollback втрачає метадані. Додати повний
    стан. RED: checkpoint→rollback має відновити salience.
12. **money.rs unchecked arithmetic** — overflow panic/wrap. `checked_mul`/`checked_add`. RED: near-i64-max
    inputs мають Err, не panic.

### 5.3 🟡 Жовтий клас (робасність — виправити при добудові)

13. `Ln=len/len` false-alarm при `b>0` → афінна регресія `a` (§2.2).
14. tier2 sentinel auditor STUB → реальний Claude-виклик.
15. eval-layer `--dry-run` fake scores у той самий файл → окремий файл.
16. check-contracts = echo OK placeholder → реальна перевірка.
17. CERTIFIED loops без proof-reports на диску + status flip ungated → serious-gate на loops/*.
18. stabilizer dt≤0 fail-open (D1) → fail-closed на malformed dt.
19. resonator TS 4 розбіжності з Rust (fail-open metric, немає initial ckpt/chaos-term/implicit rollback)
    → звірити з Rust-оригіналом після unlock #1.

---

## 6. ПЛАН ПОБУДОВИ ПО ФАЗАХ (кожна фаза = робоча система + RED-гейт)

Порядок — від грубого до тонкого (як gain-scheduling §2.6). Кожна фаза замикається **тільки** коли її
RED→GREEN гейт зелений. Жодна фаза не «done» без proof-артефакту (Mandatory Proof Rule).

### Фаза 0 — Розморозити ядро (1–2 дні)
- Зареєструвати `resonator.rs` у bebop2 lib.rs (дефект #1). **GATE:** 6 тестів resonator біжать зелено.
- Замінити `1−cos → arccos` (дефект #2). **GATE:** cosine-mirage RED-тест ловиться.
- Звірити TS-порт із Rust-оригіналом (дефект #19). **GATE:** обидва дають однаковий вердикт на пакеті.

### Фаза 1 — Лінійний контур (тиждень 1)
Приймання (`admit()` мінімальна: Tier-A UNSAT + DOF) → WORM-еталон → 1 генератор → 1 крос-модель критик →
детермінований сепаратор (спершу verify, потім `arccos` манометр + `Δn`) → запобіжники (max_iter,
token-fuse ДО API). Без гілок, без DMD, без ренорму. Це вже дає більшу частину виграшу рефлексії
(Reflexion/Self-Refine — ~20 п.п.). **GATE:** прогін на 20-задачному пакеті, `termination` коректний,
token-fuse спрацьовує RED на overspend, best-checkpoint видача.

### Фаза 2 — Панель приладів + DMD k-метр (тиждень 2)
Online DMD (примітив №2) → `lyapunov::spectral_radius` k-метр (примітив №1 Francis QR) → композитний
5-clause вердикт (§2.1) → ортогонометр `corr(Δq,Δs)` (§2.6). Тепер видно **справжній** k свого контуру.
**GATE:** slow-spiral RED (§2.2) — DMD ловить `|μ|=1.02>1`, PCA пропускає; adversarial `q↑,s↓` RED —
ортогонометр `r_Δ<0` fire.

### Фаза 3 — Ентропійний теплообмінник + persistence (тиждень 3)
Ренормалізатор rate-distortion@0 (примітив №5) з claim-preservation RED → ентропійний budget-леджер
(integer-біти) → persistence survival-таблиця (примітив №4: Hungarian + `D*` + attic re-entry). Контур
виживає на довгих прогонах (стаціонарність §2.5). **GATE:** `R_cheat` claim-drop RED rejected;
entrenched-hallucination RED — persistence сама пропускає, verify ловить (§2.4 AND-gate).

### Фаза 4 — Манiфольд гілок + асинхронна вежа (тиждень 4)
N паралельних гілок від чекпойнта {T=0.5,0.8,1.0} → спільний сепаратор → Оккам-тайбрейкер → архів для
graft → асинхронна вежа (pre-computation). Глибока якість без латентності (ширина замість глибини §7
схеми). **GATE:** BRANCH-рішення на застряглому контурі дає дисперсію результатів; вежа роздає з кешу за
0мс.

### Фаза 5 — Мета-контур + калібрування (постійно)
Error-patterns harvest → еволюція еталонів (Робін, людське вето) → калібрувальний пакет 200–500 задач
(з навмисно-правильними випадками для лову over-correction). Заміри розподілу `k` по класах задач,
ReflectionAccuracy критика, частки відкатів (>30% = дефект конструкції §6.2 схеми). **GATE:**
калібрувальний прогін дає розподіл `k`; self-preference bias `B` крос-моделі < self-`B`.

---

## 7. FMEA v2.0 (ВИПРАВЛЕНА — з математичними детекторами)

| Відмова | Механізм | Детектор (математичний) | Автодія | Код |
|---|---|---|---|---|
| Дивергенція (k>1) | позитивний feedback | DMD `ρ̂≥1` (spectral_radius) | ABORT + best ckpt | resonator+lyapunov |
| Прихована дивергенція (non-normal) | ρ<1 але ‖J‖≫1 transient | `‖Ĵ‖₂>1` (SVD σ_max) + is_chaotic | ABORT до fuse-overrun | DMD §2.1 |
| Ротаційний дрейф (tone spiral) | комплексна μ, мала step | DMD `\|μ\|=√det>1` (Francis QR) | damp `−φ_i` | §2.2 |
| Verbosity drift | геом. ріст довжини | 1-D DMD `a>1` (афінна регресія, НЕ ratio) | renorm вздовж моди | §2.2 |
| Гудхарт (критик captured) | reward-hacking | `corr(Δq,Δs)<0` Fisher-z+CUSUM | freeze + external verify | §2.6 |
| Entrenched hallucination | early-persist false | persistence сама НЕ ловить → V dominance | rollback (V authority) | §2.4 |
| Некоректна задача | ill-posed intake | `admit()` UNSAT/DOF/purity | REJECT до насоса | §2.3 |
| Non-reproducible verify | impure oracle | idempotence probe K≥2 | FORCE human-bypass | §2.3 |
| Закипання (ентропія) | H_debt > cap | `D_t>H_max` budget | hard-renorm/reground | §2.5 |
| Over-correction | хибнопозит. критик | best-ckpt діод + P-gain | видача з best | resonator |
| Гідроудар (конкурентний) | подвійний запуск | mutex/lease | відмова другому | L0 |
| Синхронна галюц. ансамблю | shared error | σ-незгода > поріг | ігнор LLM→ground | stabilizer |
| Cosine-mirage | 1−cos не метрика | arccos recompute | — (метрика виправлена) | §2.1 |

---

## 8. ЧЕСНІ МЕЖІ ТА ЩО СВІДОМО НЕ БУДУЄМО

**Що математика реалізувати НЕ дозволяє (не обіцяємо):**
1. **Сертифікацію контракції на інтейку.** Lipschitz-фактор `α` чорного ящика LLM — властивість runtime
   (модель+температура+траєкторія), не функція specа (§2.3 В4). Інтейк вирішує лише структурну коректність;
   стійкість забезпечує Нейман-clamp + runtime-монітор.
2. **Персистентність як детектор істини.** Вона вимірює generator fixed points, не world (§2.4). Ортогональна
   до true/false. Тільки зовнішній verify адьюдикує.
3. **Внутрішню самокорекцію як джерело сенсу.** DPI забороняє (§2.5): `I(еталон;вихід)` росте лише від
   зовнішнього заземлення. Контур без виконуваного verify примусово отримує людський байпас.
4. **Kalman як повний фільтр без measurement-update.** Наявний код — лише covariance propagation (40%);
   fusion шумного quality потребує добудови H/R/gain.

**Що свідомо не будуємо:**
- Самомодифікацію еталону всередині кільця (bounded self-modification: лише між прогонами, людське вето).
- LLM-суддю як єдиний verify там, де є детермінована перевірка (схема/тест/компілятор пріоритетні).
- «Нескінченний» контур у прямому сенсі: нескінченне — **дерево між прогонами** (L5), кожен прогін
  скінченний і забюджетований.
- PCA-по-Δ як резонанс-тест (§2.2 доведено хибним; лишити лише як дешевий direction-screen).
- `1−cos`, `Σ=0`-ентропійний-леджер, `Kd=1.5`-PID — усі три математично неправильні, замінені.

---

## 9. КРИТЕРІЙ ІСТИНИ: ТАБЛИЦЯ МАТЕМАТИЧНОЇ РЕАЛІЗОВНОСТІ

Ваш єдиний критерій правильності — математична можливість реалізації. Ось повний реєстр: кожен компонент,
чи реалізовний, яким апаратом, і що його спростовує.

| Компонент | Реалізовний? | Математичний апарат | RED-фальсифікатор |
|---|---|---|---|
| Контракція/збіжність | ✅ так | Банах на `arccos` + DMD `ρ(J)` + CI | 5 конструктів (§2.1) |
| Спектральні моди дрейфу | ✅ так | DMD/Koopman (не PCA) | slow-spiral (§2.2) |
| Well-posedness інтейку | ✅ частково | Тихонов + SMT QF_LIA + AC-3 | impure verify (§2.3) |
| Контракція чорного ящика на інтейку | ❌ ні (runtime only) | — (Нейман-clamp замість) | — |
| Персистентність сигнал/шум | ✅ так | survival `D*=⌈log_p α⌉` + Hungarian | entrenched-halluc (§2.4) |
| Персистентність як істина | ❌ ні (ортогональна) | — (verify dominance) | early-persist false |
| Ентропійний дренаж | ✅ так | DPI + rate-distortion@0 | R_cheat claim-drop (§2.5) |
| Збереження сенсу (конст.) | ❌ ні (хибно) | DPI (монотонний витік) | — |
| Ентропійний бюджет | ✅ так | integer money.rs, `0≤D_t≤H_max` | overflow drift (§2.5) |
| Ортогональність ген⊥критик | ✅ так | `∂q/∂v=0` paraphrase-invariance | flip_rate>τ (§2.6) |
| Goodhart-детекція | ✅ так | `corr(Δq,Δs)` Fisher-z+CUSUM | q↑s↓ adversarial (§2.6) |
| Крос-модельний критик | ✅ так (необхідний) | `Var(η_shared)` decorrelation | B≈0.52 self-preference |
| PID-стабільність | ✅ так (після фіксу) | Джурі + ζ=1 + filtered deriv | Kd=1.5 Джурі-violation |
| Bounded envelope | ✅ так | tanh-saturate 1-Lipschitz | delta > max clamp |
| Field-veto | ✅ так | heat-kernel blast > TOLERANCE | red-line не Permit |
| Signed checkpoints | ✅ так (95%) | Ed25519+ML-DSA-65 TLV | tamper-RED |
| Детермінований сепаратор | ✅ так | decide-law (order_machine) | illegal transition |
| Нейман-clamp | ✅ так | проєкція на δ-кулю (1-Lipschitz) | ‖ΔV‖>δ не clamped |
| Стаціонарність (∞ робота) | ✅ так | Foster-Lyapunov негат. дрейф | μ≥0 overflow |

**Підсумок реалізовності:** з 19 компонентів **15 повністю реалізовні** наявним+добудованим апаратом, **1
частково** (well-posedness — структурне так, стійкість ні), **3 математично неможливі як заявлено у схемі**
(контракція на інтейку, персистентність-як-істина, збереження-сенсу) — і для кожного з трьох є коректна
заміна (Нейман-clamp, verify-dominance, DPI). Жоден компонент не спирається на непідтверджену магію.

---

## 10. ПІДСУМКОВИЙ ПАСПОРТ ПРОЄКТУ v2.0

**Система:** замкнутий кібернетичний контур обробки промптів на L5-стеку dowiz/bebop. Один вхід
(`admit()`-скомпільований еталон), один вихід (best-checkpoint клапан видачі), нульові неконтрольовані
витоки, керований дренаж ентропії за DPI, детермінований контролер + LLM-спостерігач під bounded envelope.

**Несуча математика (виправлена):** Банах на **arccos**-метриці (не 1−cos) → **DMD/Koopman** спектр дрейфу
(не PCA) → **Тихоновська** well-posedness інтейку (не PDE-Hadamard) → **survival analysis** персистентності
(не TDA) → **Data Processing Inequality** для ентропії (не збереження) → **∂q/∂v=0** ортогональність +
**Джурі-стабільний** PID (не Kd=1.5) → **Нейман-проєкція** на δ-кулю як runtime-гарант.

**Несуча інженерія (наявний код):** `resonator` регулятор (розморозити) + `stabilizer` bounded envelope +
`field.rs` heat-kernel veto + `audit.rs` WORM hash-chain + `agentic_git` чекпойнт-батарея + `order_machine`
decide-law + `money.rs` integer-бюджет + bebop2 `field/kalman/fft/algebra` спектральне ядро + `loops`+
`metric-core` оркестрація + crypto `Ed25519/ML-DSA-65` signed checkpoints.

**Головні числа:** метрика = `arccos` у кулі <π/2; `k` = DMD `ρ̂` (аларм-band [0.5,0.8], не ціль); DMD
truncate `τ=2.858·σ_median`; persistence `D*=⌈log_p α⌉` (power n≥D*+1); ентропійний budget `0≤D_t≤H_max`,
стаціонарність `⟨ΔH_out⟩>⟨ΔH_in⟩`; PID filtered-derivative `γ≈0.8`, `ζ=1` (`Kp≈1.48`); критик крос-модель
(B≈0.52 self-bias); композитний 5-clause вердикт; 2–3 оберти sync / ширина замість глибини async; renorm
`H↓∧claims-preserved`; відкати ≤30%.

**Обсяг роботи:** ~60–70% коду вже існує; ~6 нових примітивів (Francis QR, online DMD, admit(), persistence
survival, entropy-budget, ортогонометр+PID); ~19 пріоритетних дефектів до виправлення (6 червоних блокують
математичну коректність). 5 фаз, кожна = робоча система з RED-гейтом.

**Ключовий інваріант.** Потік не виходить за межі контуру. Хаос лишається паливом. Межі визначають форму
розв'язку. **Але сенс тече лише з зовнішньої землі всередину — ніколи навпаки** (DPI). Контур, що вимірює
себе собою, дрейфує; контур, заземлений до WORM-еталону і зовнішнього verify, — збігається.

---
*Кінець документа. Версія 2.0. Жива структура — переглядати після кожного калібрувального циклу
макро-контуру (L5). Синтез: 6 аналізів коду + 6 математичних досліджень; критерій — математична
реалізовність; кожен несучий вузол прив'язаний до наявного коду і несе falsifiable RED.*

