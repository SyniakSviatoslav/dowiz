# ПОЛЕ ЯК ІНТЕРФЕЙС — v1.0
## План побудови фізичного UI-двигуна: DOD + Rust/WASM + wgpu, де математика хвиль малює весь інтерфейс
### Синтез: наявний код dowiz/bebop/bebop2 × математика поля × реалізовність на слабкому залізі. Критерій істини — математична/технічна реалізовність.

> **Теза.** Інтерфейс — це фізичне поле. Віджети — не збережені (retained) об'єкти, а **емерджентні структури симуляції поля**: layout = рівновага сил, motion = хвильова динаміка, focus = потенційні ями, feedback = брижі (функція Гріна), structure = власні моди Лапласіана. Математичні функції для малювання **хвиль** стають фундаментом малювання **інтерфейсів загалом** — доведено строго, з чесною межею, де фізику доповнюють три дискретні шари.
>
> **Джерела синтезу.** 4 глибокі аналізи коду (фронтенд/анімація, дизайн-система, particle-cloud+фізичний субстрат, bebop2-математика+WASM-межа) + 3 математичні дослідження (immediate-mode GPU renderer, фізика-як-UI-субстрат, DOD+zero-copy+thin-shell). Конспекти: [engine-reports/INDEX.md]. Лише код і математика; поточний стан звірено на диску.
>
> **Що це НЕ.** Не «зробити канвас-анімацію красивою». Це заміна retained-DOM-моделі рендерингу на **континуальний динамічний скелет** (одне поле) + три дискретні корекції (constraint solver, SDF, стейт-машина), спроєктована так, щоб бігати на інтегрованих GPU і слабких телефонах.

---

## 0. ГОЛОВНИЙ РЕЗУЛЬТАТ І П'ЯТЬ ІСТИН СИНТЕЗУ

Дослідження дало один строгий результат і п'ять інженерних істин, на яких стоїть весь план.

**Головний результат (доведено, §R2).** Інтерфейс описується **ОДНИМ** оператором з **ОДНИМ** сертифікатом збіжності, **ОДНІЄЮ** модальною декомпозицією і **ОДНИМ** словником відчуття:

```
Керівний закон поля:   M Ü + Γ U̇ + c²·L·U = S(t)         (value/activation field)
                       M Ẍ = −∇E(X) − Γ_x Ẋ              (position/layout field)
Сертифікат:            H = ½ẊᵀMẊ + E(X),  dH/dt = −ẊᵀΓẊ ≤ 0  → LaSalle → рівновага
Модальна декомпозиція: U = Σ_k a_k(t)·φ_k,  L φ_k = λ_k φ_k   (Laplacian eigenmodes)
Словник відчуття:      a_k'' + 2ζ_kω_k a_k' + ω_k² a_k = s_k/m,  ω_k = c√(λ_k/m),  ζ_k = γ/(2√(mc²λ_k))
```

П'ять аспектів UI — це п'ять ознак **однієї** системи: layout = рівноваги `∇E=0`; motion = транзієнти; feedback = функція Гріна `U=G∗S`; focus = потенційні ями; structure = низькі моди `φ₂,φ₃`.

**Ключове осяяння:** сертифікат збіжності поля (`wave_energy`/LaSalle) — це **та сама теорема**, що доводить стабільність resonator'а з попереднього арку. І критичне демпфування `ζ=1` (найшвидший settle без overshoot — фізика доброго UI-easing) — це **те саме** критичне демпфування, що і в governor'і. Один математичний апарат керує і петлею сенсу, і малюванням інтерфейсу.

### П'ять інженерних істин

1. **`particle-cloud.js` — це вже зародок двигуна, вже DOD.** Саморобний WebGL2 particle field з 3 flat `Float32Array` (SoA ring buffer), event→physics-vocab (`{color,energy,burst,swirl}`), pointer-force, life-decay, idle-stop. Пряме перенесення у wgpu. Але сьогодні це **one-shot decorative FX** в одному компоненті — весь двигун є дельтою до перетворення цього на UI-субстрат.

2. **Фізичний субстрат bebop доведений і детермінований, але жоден вузол не з'єднаний з пікселем.** `field_physics.rs` (damped wave + Lyapunov, стабільний коридор dt≤0.02), `wavefield.rs` (FR spring layout, spectral), `stabilizer.rs` (potential well + tanh), bebop2 `field.rs` (Laplacian eigenmodes **з eigenvectors** — рідкість!). Чотири фізичні стовпи (layout/motion/focus + easing) вже є; треба їх дротувати до GPU.

3. **Дизайн-система повністю перелічувана і ESLint-примусова → відтворювана як GPU-таблиця токенів.** ~11 brand + 40 status + 8 semantic токенів, рівно 8 розмірів шрифту, 4px-сітка, жодного hex. Пружини вже параметризовані (`motion.ts` k/d = ω,ζ готові). Немає чистого GPU-аналога лише для: box-shadow (blur/erf), backdrop-blur, color-mix (CPU-resolve), Fraunces variable font (discretize).

4. **Zero-copy — найбільший важіль реалізовності.** JSON-межа (`wasm.rs`) катастрофічна per-frame (V8 `JSON.parse` 240KB тексту 60×/с). Рішення: Rust пише flat `Vec<f32>` у linear memory → `Float32Array::view` (0 copy) → `queue.writeBuffer` (1 upload). `writeBuffer` **перевершує** mapped buffers саме з WASM. Межа обирається **частотою оновлення**, не типом даних: transactional→JSON, per-frame→view.

5. **Чесна межа: це гібрид, не чистий GPU.** Фізика НЕ може тримати три класи поведінки (доведено RED-тестами): exact pixel-alignment (measure-zero), crisp edges/glyphs (band-limited), exact discrete semantics (money/selection/z-order). І a11y на web не має AccessKit-backend у 2026. Тому: **GPU для visual/animated/owner-app; DOM залишається для text-input, a11y-mirror, SSR-menu.** Хто обіцяє чистий GPU-застосунок з повним a11y у 2026 — помиляється.

---

## 1. ІНВЕНТАР: ЩО ВЖЕ Є І ЩО МІГРУВАТИ

### 1.1 Два роз'єднані фронтенд-стеки (мігруємо ВІД)

| Стек | Тех | Стан money | Анімація | Роль у міграції |
|---|---|---|---|---|
| **ЛЕГАСІ `apps/web`** | React 18 DOM SPA, framer-motion (35 файлів), 32 CSS keyframes, react-router, Zod | ⚠️ **4 tween-порушення** (ClientLayout:154, EarningsPage:47-176, DashboardPage:421, AnalyticsPage:262) через AnimatedNumber/CountUpPrice | DOM-mediated + framer parallel rAF; AnimatedNumber робить rAF→setState per-frame (найгірший path) | Несе весь a11y/routing/forms/i18n вантаж; мігрувати ОСТАННІМ, гібридно |
| **КАНОНІЧНИЙ `web/`** | Astro SSR + Svelte 5 islands на kernel WASM; WebGL2 particle cloud | ✅ integer-cent static, **NO tween** (Storefront/OwnerDashboard/CourierTrack) | Svelte fine-grained + 1 GPU loop decoupled | **BEACHHEAD** — вже має kernel WASM + money-discipline + particle DOD + no framer |

**Висновок:** канонічний `web/` — правильний плацдарм. Він уже на ~80% там (WASM-стан + один GPU-loop); бракує лише універсального рендерера (текст/форми все ще Svelte DOM).

### 1.2 Фізичний субстрат (мігруємо ЗАВДЯКИ) — чотири стовпи + рендер-шар

| Актив | Файл | Роль у двигуні | Готовність | Gap |
|---|---|---|---|---|
| **particle-cloud.js** | webgl/particle-cloud | Рендер-шаблон: SoA ring 4096, event→vocab, GLSL sprites, additive glow | 30% (тільки point sprites) | no lines/quads/SDF/text, RGB truncated (blue hardwired), no layout-binding, no persistent particles |
| **Damped wave** | field_physics.rs | Motion/activation/feedback: `m ü=c²Lu−γu̇+s`, Lyapunov `wave_energy`, стабільний dt≤0.02 | physics 60% / render 0% | ragged `Vec` tensors → flatten CSR; немає channel→pixel mapping |
| **FR spring layout** | wavefield.rs | Layout positions: `k=√(area/n)`, repulsion `k²/d`, attraction `d²/k` | 70% | O(n²), unweighted, point-only (no card-sizing/collision/anchor); **upgrade → stress-majorization** |
| **Potential well** | stabilizer.rs | Focus/snap/dock: `½Σk(θ−b)²` + `tanh` saturate + forbidden-zone bump | 80% | expose `∇V=k(θ−b)` restoring force |
| **Laplacian eigenmodes+vectors** | bebop2 field.rs | Structure (spectral embedding φ₂,φ₃) + heat-kernel transitions `Σe^{−λt}⟨u,φ⟩φ` | 85% layout / 90% heat | coords_2d helper; Lanczos для великих; frame-driver clock→t + field→visual mapping |
| **Heat-kernel matrix-free** | chebyshev.rs | Diffusion на великих графах без eigendecomp, `exp(−cLt)` qp=64 | 90% | 2-D separable FFT для blur/bloom не написаний |
| **Zero-copy CSR kernel** | rust-core lib.rs | raw C-ABI linear memory f32, deterministic libm | 55% | ВСЯ wgpu binding glue = TODO; SharedArrayBuffer; C8-fexp port |

### 1.3 Дизайн-система (відтворюємо як GPU-таблицю)

- **Джерело істини:** `tokens.css` :root (597 рядків), 4-шаровий CSS-var cascade (defaults→preset→dark→tenant-inline→paper-skin→sunlight!important) — рендерер МУСИТЬ відтворити override-stack точно.
- **Warm Cosmo-Noir default:** gold `#d69a3d` + teal `#061b1a`/`#152928` + warm off-white `#f5efe5`.
- **Повністю перелічувано + ESLint-примусово:** ~11 brand + 40 status(10×4) + 8 semantic + 3 chart. Type scale 8 steps (11-36px). Spacing 4px. Radius none..full. Пружини `motion.ts` (press{400,25}/enter{260,24}/bounce{500,18}/gentle{180,22}) = (ω,ζ) готові. `AnimatedNumber` ВЖЕ поважає reduced-motion.
- **3 locales** sq(default)/en/uk (Cyrillic), LTR, ~1296 keys.
- **Немає bebop skin** (grep) — тільки `data-skin=paper`.

---

## 2. УНІФІКОВАНА АРХІТЕКТУРА: ОДНЕ ПОЛЕ + ТРИ ДИСКРЕТНІ ШАРИ

Це несуча конструкція. П'ять аспектів UI — континуальні (поле); три класи поведінки — дискретні (не поле). Взаємодія одностороння per-frame: **поле вирішує динаміку → constraint проєктує на точні рівності → SDF рендерить чітку геометрію (поле лише модулює) → стейт-машина постачає точні значення (поле лише презентує).**

### 2.1 Поле володіє п'ятьма аспектами (§R2)

**Layout = рівновага сил (§R2.1).** Кожен card/list-item/control = частинка `x_i∈ℝ²`. Енергія `E(X)=Σ_edges ½k_ij(‖x_i−x_j‖−d0_ij)² + Σ repulsion`. **Без jitter (критично):**
- **Stress majorization (SMACOF)** замість force-integration: Guttman transform `X^{k+1}=V⁺B(X^k)X^k` — майоризація ГАРАНТУЄ `σ(X^{k+1})≤σ(X^k)` monotone, не осцилює (upgrade FR→majorization);
- **АБО** symplectic Euler + `wave_energy` Lyapunov → LaSalle → рівновага (ТА САМА теорема resonator);
- **pixel-snap ПІСЛЯ збіжності** `x_snap=g·round(x*/g)`, ніколи during (freeze при `‖∇E‖<ε_f AND ‖ẋ‖<ε_v`).

**Motion = хвиля/дифузія + критичне демпфування (§R2.2).** Heat kernel `U(t)=e^{−αLt}U₀` (monotone, no overshoot — theme swap/spread) vs damped wave (springy/ripple). Модальне розчеплення → незалежні осцилятори; одноелементне easing = `ẍ+2ζω ẋ+ω²x=ω²x_target`: ζ=1 critical (snappy `x=x_t[1−(1+ωt)e^{−ωt}]`), ζ<1 bouncy, ζ>1 sluggish. **cubic-bezier НЕ може overshoot → springs витіснили.** Designer (tension,friction,mass)=(ω,ζ,m); `motion.ts` пружини вже задають ζ. Глобальні transitions = heat-kernel stagger (delay `τ_j=dist(source,j)/√α` — ripple-out reveal автоматом).

**Feedback = функція Гріна (§R2.4).** Кожна дія інжектить source `S`; відгук = згортка з Green's function. Tap δ→ripple (Material ripple = буквально `G` 2D damped wave). Дискретна `G_ij(t)=Σφ_k(i)φ_k(j)g_k(t)`. Event→source vocab (узагальнення particle-cloud): tap=δ wave; success=Gaussian HEAT; error=high-λ shake; loading=sustained. Ripples + particles = ДВА рендери ОДНОГО поля.

**Focus = потенційні поля (§R2.3).** `V(x)=−Σ A_i exp(−‖x−c_i‖²/2σ²) + Σ bumps`. Cursor test particle `m p̈=−∇V−γṗ` + tanh saturate. **ОДНЕ V драйвить ВСЕ emphasis** (no per-component): scale/brightness(Boltzmann)/blur(DoF)/saturation як readouts. Local minima=snap/dock.

**Structure = спектральне вкладення (§R2.5).** Hall: `x_i=(φ₂(i),φ₃(i))` низькі eigenvectors L (мінімізує `xᵀLx` s.t. ⊥1). Natural tangle-free layouts, no local minima. Eigenvalue = рівень hierarchy. bebop2 field.rs ВЖЕ має eigenvectors. Decoration = same operator finer scale (Bessel drum, spherical harmonics, golden-angle phyllotaxis для particle seeding).

### 2.2 Три дискретні шари, які поле НЕ може замінити (§R2.6, доведено RED)

| Клас | Чому поле не може (математика) | Власник | RED-тест |
|---|---|---|---|
| **Exact align / equal-size / contain** | Рівність (integer) = measure-zero; smooth flow дрейфує | **Constraint solver** (Cassowary/QP via gradient projection) | `expect(colA.right).toBe(colB.left)` FAILS field-only |
| **Crisp 1px stroke / glyph / baseline** | Поле band-limited → O(σ) blob, ніколи ≤1px | **SDF шар** (analytic distance, thresholded; поле лише модулює position/scale/glow) | edge-width O(σ)≫1px |
| **Exact money / selection / z-order** | Континуум не має точного integer/selection; interpolate money = RED-LINE violation | **Дискретна стейт-машина** (kernel; поле лише ПРЕЗЕНТУЄ decided states) | field-interp total `$12.4999` mid-transition VIOLATES |
| **Text line-break / reflow** | Combinatorial (Knuth-Plass), не energy min | **Typesetting** (cosmic-text) | — |

**Наслідок для money-never-tween (ваша red-line):** межа field↔state захищає її **за конструкцією** — поле презентує вже-обчислене integer-значення з kernel і НІКОЛИ не інтерполює його. Immediate-mode redraw settled number = no count-up. Це строго безпечніше за легасі AnimatedNumber.

---

## 3. ШІСТЬ ШАРІВ ДВИГУНА (архітектура виконання)

```
┌───────────────────────────────────────────────────────────────────────────────┐
│ E5 THIN-SHELL JS (bootloader + event forwarder)                                 │
│    load wasm + feature-detect SIMD/WebGPU · raw events → wasm.on_pointer(one-line)│
│    hold GPUDevice/Queue handles + writeBuffer(view) · arm/cancel rAF (lazy)      │
│    MUST-STAY: clipboard/file-dialog/A11Y-mirror/IME-input/native-pickers/URL     │
├───────────────────────────────────────────────────────────────────────────────┤
│ E4 РЕНДЕР (wgpu, immediate-mode, single render pass painter-order)              │
│    P0 shape SDF (1 draw N inst rounded-box) → P1 text MSDF (cosmic-text shape)   │
│    → P2 icons → P3 particle-cloud custom shader (Green's tracers) → [P4 egui]    │
│    Bind0 per-frame UBO {screen,dpr,time,theme_tokens} · Bind1 static atlases     │
├───────────────────────────────────────────────────────────────────────────────┤
│ E3 ДИСКРЕТНІ КОРЕКЦІЇ (поле НЕ володіє)                                          │
│    constraint solver (Cassowary/QP align) · SDF geometry · kernel state          │
│    (money/selection/z-order) · cosmic-text line-break                            │
├───────────────────────────────────────────────────────────────────────────────┤
│ E2 ПОЛЕ (фізичний субстрат — континуальний скелет)                              │
│    layout(stress-majorization+Lyapunov) · motion(heat+damped-wave ζ) ·           │
│    feedback(Green's fn) · focus(potential wells) · structure(spectral φ₂φ₃)      │
│    Код: field_physics + wavefield(→majorization) + stabilizer + bebop2 field     │
├───────────────────────────────────────────────────────────────────────────────┤
│ E1 DOD STORE + FIXED-TIMESTEP LOOP                                               │
│    SoA WidgetStore + ParticlePool ring (hand-rolled, НЕ bevy_ecs) ·               │
│    accumulator DT=0.02=field::DT_STABLE + MAX_FRAME/MAX_SUBSTEPS guards ·         │
│    lazy-render-on-settle (physics_settled = resonator Converged, biggest battery)│
├───────────────────────────────────────────────────────────────────────────────┤
│ E0 ZERO-COPY КОМПУТ-МЕЖА                                                         │
│    Rust flat Vec<f32> у linear memory → Float32Array::view (0 copy) →            │
│    writeBuffer (1 upload) АБО WGSL compute (positions stay on GPU) ·             │
│    simd128 f32x4 integrator + scalar fallback · WebGL2 baseline+WebGPU enhance    │
└───────────────────────────────────────────────────────────────────────────────┘
```

**Дисципліна межі (§E0):** transactional/domain → JSON (`wasm.rs` unchanged); per-frame numeric → `Float32Array` view + `writeBuffer`. Обирається частотою оновлення, не типом даних.

**Battery-важіль (§E1):** `should_render = input_pending || !physics_settled || animation_active || external_change`; інакше — НЕ планувати наступний rAF, dormant. `physics_settled` = `field.rs active_diffuse` gate `|Δu|<ε` = resonator `error<delta_threshold Converged`, з hysteresis K=3 (stall_patience) і Lyapunov watchdog (divergence → force render, ніколи dormant). Interact 5% + settle 0.5s → duty cycle few %.

---

## 4. ЧЕСНА МЕЖА ГІБРИДУ ТА ПОРЯДОК МІГРАЦІЇ

### 4.1 Що залишається DOM (не мігрує) — це не тимчасовий gap, а постійна ціна

- **Text input / IME / autofill / password / mobile keyboard** — прихований DOM `<input>` overlay над GPU-намальованим полем (як egui-web). Canvas не може викликати екранну клавіатуру мобільного; focused DOM input може. IME composition = DOM-only.
- **A11y semantic tree** — паралельний прихований transparent DOM mirror (Flutter CanvasKit підхід): role/aria-label/tabindex/rect/state, reconcile per-frame з immediate widget list. **AccessKit НЕ має web-backend у 2026** (roadmap: "most difficult, no timeline") → hand-roll. Постійні втрати: Ctrl+F find-in-page, translate, reader mode.
- **Public SEO pages (`/s/:slug`)** — залишаються SSR DOM. Canvas рендерить НІЧОГО server-side; crawlers/OG-unfurl потребують HTML. GPU-takeover цілить owner/admin app за auth, де SEO нерелевантне.
- **Native pickers, History/URL, clipboard, file dialogs, permission/gesture-gated chrome** — JS-shim, Rust викликає через.

### 4.2 Decidable порядок міграції (§R1.6)

Мігрувати компонент у GPU iff `Gain − Loss > 0`, де `Gain = M·(render+memory saved)`, `Loss = A + S + (T ? ∞ : 0)`. Член `T?∞` — несучий: **будь-який компонент з text-input НІКОЛИ не pure-GPU** (гібрид overlay завжди).

```
1. Decorative/animated, zero-semantics (M≈1, A≈S≈0, T=0)  → FIRST  максимальний Gain, ~0 Loss
   particle field (done), live-order/earnings viz, charts, transitions, background
2. Visual containers (cards/lists/panels)                 → GPU + semantic-mirror nodes
3. Nav/structural chrome                                   → GPU + mirror + focus order
4. Forms & text-editor surfaces                           → HYBRID always (GPU field + transparent <input>)
5. Public SEO /s/:slug                                     → DO NOT migrate, keep SSR DOM
```

**money-never-tween ДОПОМАГАЄ тут:** immediate redraw settled number = no count-up → «money viz» = first-wave, якщо рендерить static final figure (drop AnimatedNumber/CountUpPrice).

---

## 5. ФАЗИ ПОБУДОВИ (кожна = робоча система + RED-гейт)

Порядок — від рендер-примітиву до повного поля. Плацдарм — канонічний `web/` стек. Кожна фаза замикається лише коли RED→GREEN зелений.

### Фаза 0 — Компут-межа + DOD store (тиждень 1)
Zero-copy WASM↔GPU: Rust flat `Vec<f32>` → `Float32Array::view` → `writeBuffer`; SoA `WidgetStore`+`ParticlePool` ring; fixed-timestep accumulator DT=0.02. **GATE:** порт `particle-cloud.js` фізики у Rust/WASM, той самий візуал через zero-copy шлях; 0 `JSON.parse` у frame-loop (профіль); dt-corridor тест не дивергує на симульованому stutter.

### Фаза 1 — SDF рендер + дизайн-токени (тиждень 2)
Один instanced quad SDF pipeline (`sdRoundBox` + analytic AA + border + gradient + erf-shadow); GPU-таблиця токенів (11 brand+40 status+8 semantic, color-mix pre-resolved CPU); MSDF text atlas (cosmic-text shaper, sq/en/uk + Tabler icons). **GATE:** відрендерити один реальний екран (Storefront card) піксель-у-піксель проти CSS-версії; triangle-inequality на geodesic не потрібен тут; theme-switch = 1 uniform write.

### Фаза 2 — Поле: layout + motion (тиждень 3)
Stress-majorization layout (upgrade FR) з Lyapunov-збіжністю + pixel-snap-after; critically-damped spring easing (ζ з `motion.ts`); heat-kernel stagger transitions. **GATE:** layout збігається без jitter (freeze при ε), snap точний ПІСЛЯ; easing ζ=1 монотонний no-overshoot; money НЕ інтерполюється (RED: field-interp total ніколи не показує проміжне значення).

### Фаза 3 — Поле: feedback + focus + structure (тиждень 4)
Green's-function feedback (tap ripple, event→source vocab узагальнений з particle-cloud); potential-well focus (одне V драйвить scale/brightness/blur/saturation, snap/dock); spectral embedding layout (φ₂,φ₃ з bebop2 field.rs) як initializer. **GATE:** одна дія = один field impulse (не per-component код); focus change = well spring; spectral layout tangle-free на реальному графі меню.

### Фаза 4 — Гібрид-межа + lazy-render (постійно)
Transparent `<input>` overlay для форм; a11y semantic mirror (reconcile per-frame); lazy-render-on-settle (dormant коли physics_settled + no input); WebGL2 fallback + scalar SIMD fallback. **GATE:** screen-reader читає mirror; форма приймає typed input + autofill; idle екран → rAF СПРАВДІ зупиняється (battery gate); працює на WebGL2-only пристрої.

---

## 6. РЕАЛІЗОВНІСТЬ І РИЗИКИ (чесно)

| Підпроблема | Реалізовна? | Основа |
|---|---|---|
| Immediate-mode whole-app | ✅ | egui 0.35 production; Θ(K) state vs Θ(N) DOM exact |
| Field-as-UI (layout/motion/feedback/focus/structure) | ✅ | один оператор + Lyapunov + модальна + (ω,ζ); субстрат частково є |
| Текст (sq/en/uk + icons) | ✅ | cosmic-text+rustybuzz+MSDF; Cyrillic+Albanian OK |
| SDF shapes + tokens | ✅ | closed-form sdRoundBox + erf shadow + fwidth AA |
| Zero-copy WASM↔GPU | ✅ | Float32Array view + writeBuffer (beats mapped from WASM) |
| Fixed-timestep стабільність | ✅ | accumulator DT=0.02=DT_STABLE (tested corridor) |
| Lazy-render battery | ✅ | resonator convergence detector (tested 6/6) |
| WebGPU + WebGL2 fallback | ✅ | WebGPU Baseline Jan 2026; wgpu dual-target |
| **A11y на web** | ⚠️ ГІБРИД | AccessKit no web backend 2026 → hand-roll DOM mirror; Ctrl+F/translate lost |
| **Forms / text input** | ⚠️ ГІБРИД | IME/autofill потребують real DOM `<input>` |
| **Exact align / money / crisp text** | ⚠️ 3 ДИСКРЕТНІ ШАРИ | measure-zero/band-limited/discrete — constraint/SDF/state, не поле |

**Головні ризики і мітигації:**
1. **A11y regression** — найбільша ціна. Мітигація: a11y-mirror = first-class subsystem (не afterthought); owner/admin app за auth; public menu лишається SSR DOM.
2. **Обсяг переписування** — не робити big-bang. Island-by-island за `Gain−Loss` сортом; particle = острів #0; форми/SEO лишаються DOM.
3. **Determinism на GPU** — GPU FMA ≠ CPU. Authoritative compute CPU-side (WASM f64→f32), GPU = display consumer; або accept visual (не bit) reproducibility.
4. **Слабке залізо** — WebGL2 baseline (universal) + WebGPU enhancement; simd128 + scalar fallback; 30fps design budget; gates 4 (active frame ≤33ms) + 6 (idle rAF stops) = що слабке HW реально валить.
5. **Coherence/field_active знак дифузії** (з попереднього аналізу) — використовувати ВИКЛЮЧНО Chebyshev spectral path (правильний знак), не coherence single-step.

---

## 7. ЩО СВІДОМО НЕ БУДУЄМО / ЧЕСНІ МЕЖІ

- **Чистий GPU-застосунок без DOM.** Неможливо з повним a11y у 2026 (AccessKit). Гібрид — не компроміс, а правильна архітектура.
- **Поле, що тримає точні рівності/money/crisp-text.** Три дискретні шари — за конструкцією, не «доробимо фізикою».
- **bevy_ecs.** Dynamic-composition overhead невиправданий для fixed compile-time UI component set. Hand-rolled SoA.
- **JSON у frame-loop.** Transactional лишається JSON; per-frame — тільки view.
- **Big-bang міграція.** Island-by-island, forms/SEO лишаються DOM.
- **coherence single-step дифузія.** Тільки Chebyshev spectral (правильний знак).
- **Money interpolation.** Field↔state boundary: поле презентує integer з kernel, ніколи не tween (захищає red-line).

---

## 8. ПІДСУМКОВИЙ ПАСПОРТ

**Система:** фізичний UI-двигун на Rust/WASM+wgpu, де інтерфейс — це поле `M Ü+Γ U̇+c²LU=S`, а віджети — емерджентні структури симуляції. DOD SoA, zero-copy компут-межа, fixed-timestep, lazy-render-on-settle, thin-shell JS.

**Несуча математика:** ОДИН damped-field оператор + ОДИН Lyapunov сертифікат (wave_energy/LaSalle = теорема resonator) + ОДНА Laplacian модальна декомпозиція + ОДИН (ω,ζ) словник (критичне ζ=1 easing = критичне демпфування governor). П'ять аспектів = п'ять ознак одного поля; три дискретні шари (constraint/SDF/state) — чесна межа.

**Несуча інженерія (наявний код):** particle-cloud.js (DOD ring seed) + field_physics (damped wave+Lyapunov) + wavefield (→stress-majorization) + stabilizer (potential well) + bebop2 field.rs (spectral eigenvectors + heat-kernel) + rust-core (zero-copy CSR ABI) + tokens.css (перелічувана GPU-таблиця) + resonator (convergence detector для lazy-render).

**Головні числа:** DT=0.02 (field::DT_STABLE); 30fps design / 16ms bonus; 32-64MB memory ceiling; particle cap 4096; MSDF pxRange=4; ζ=1 critical (snappy τ_s≈130ms), ζ≈0.65-0.8 fluid, ζ≈0.35 playful; golden-angle 137.5° seeding; zero-copy view+writeBuffer; lazy dormant при physics_settled hysteresis K=3.

**Обсяг:** ~40% фізики вже в коді (доведеної+детермінованої); треба wgpu binding glue (0 сьогодні), SDF шар, constraint solver bridge, MSDF text, a11y mirror, field↔state boundary. 5 фаз, beachhead = канонічний web/, island-by-island, forms/SEO лишаються DOM.

**Ключовий інваріант.** Інтерфейс — це поле; хаос симуляції — паливо; дискретні шари визначають точність. Поле малює рух, layout, feedback, focus, structure одним оператором; constraint/SDF/state тримають те, що континуум провабли тримати не може. Money тече з kernel integer-ом і ніколи не інтерполюється полем.

---
*Кінець плану. Версія 1.0. Блюпринти виконавцям — у BLUEPRINTS-FIELD-UI.md. Синтез: 4 аналізи коду + 3 математичні дослідження; критерій — реалізовність на слабкому залізі; кожен несучий вузол прив'язаний до наявного коду.*
