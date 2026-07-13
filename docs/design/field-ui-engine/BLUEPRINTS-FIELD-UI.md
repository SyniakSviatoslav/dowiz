# ПОЛЕ ЯК ІНТЕРФЕЙС — БЛЮПРИНТИ ДВИГУНА
## Implementation-ready робочі одиниці для агентів-виконавців

> Похідний від [FIELD-UI-ENGINE-PLAN.md](FIELD-UI-ENGINE-PLAN.md). Кожен блюпринт — самодостатня робоча
> одиниця: точний файл/модуль, звірений поточний стан, математично обґрунтована ціль, реалізовний
> алгоритм, RED→GREEN гейт. Автор синтезує; кодять агенти. Джерела: 4 аналізи коду + 3 математичні
> дослідження (конспект: engine-reports/INDEX.md). Поточний стан звірено на диску.

## 0. КОНТРАКТ ВИКОНАВЦЯ (для КОЖНОГО блюпринта)

1. **RED→GREEN або не зроблено.** Детермінований гейт, доведений червоним тоді зеленим. Заборонено
   cheat-green (skip/.only/inflated-timeout/expect(true)/закоментована асерція).
2. **Grounding перед правкою.** Прочитай цільову секцію; якщо звірений тут стан розійшовся з диском —
   зупинись, познач, не вгадуй.
3. **Скоуп жорсткий.** Дотримуйся «OUT OF SCOPE». Red-line (money/kernel-state) — окремий gate.
4. **Математична коректність — критерій.** Реалізація не відповідає формулі → неправильна реалізація.
5. **Гібрид, не чистий GPU.** Форми/a11y/SSR лишаються DOM — це архітектура, не тимчасовість.
6. **Determinism.** Authoritative compute CPU-side (WASM f64→f32); GPU=display. scalar==SIMD bit-identical.

## 1. ГРАФ ЗАЛЕЖНОСТЕЙ І ХВИЛІ

Плацдарм — канонічний `web/` стек (Astro/Svelte+kernel WASM+particle DOD). Big-bang заборонено;
island-by-island за `Gain−Loss>0` (`Loss = A + S + (T?∞:0)`, text-input НІКОЛИ pure-GPU).

```
ХВИЛЯ 0 (компут-фундамент) ────────────────────────────────────────────
  FE-01 zero-copy WASM↔GPU міст            lane-A  [E0]
  FE-02 SoA DOD store + ParticlePool ring  lane-B  [E1]
  FE-03 fixed-timestep accumulator loop    lane-C  [E1] (dep FE-02)
        ↓ бар'єр: numeric дані течуть zero-copy, loop стабільний
ХВИЛЯ 1 (рендер-примітиви) ─────────────────────────────────────────────
  FE-04 particle-cloud → wgpu compute/port lane-A  [E4] (dep FE-01,02)
  FE-05 SDF shape pipeline + design tokens lane-B  [E4] (dep FE-01)
  FE-06 MSDF text (cosmic-text + atlas)    lane-C  [E4] (dep FE-01)
        ↓ бар'єр: можна намалювати реальний екран піксель-точно
ХВИЛЯ 2 (поле — динаміка) ──────────────────────────────────────────────
  FE-07 layout field (stress-majorization) lane-A  [E2] (dep FE-02)
  FE-08 motion field (critically-damped)   lane-B  [E2]
  FE-09 field↔state boundary (money guard) lane-C  [E3] 🔴 red-line
        ↓ бар'єр: layout+motion без jitter, money defended
ХВИЛЯ 3 (поле — семантика) ─────────────────────────────────────────────
  FE-10 feedback field (Green's function)  lane-A  [E2] (dep FE-04)
  FE-11 focus field (potential wells)      lane-B  [E2] (dep FE-08)
  FE-12 spectral structure layout          lane-C  [E2] (dep FE-07)
  FE-13 constraint solver bridge (align)   lane-D  [E3] (dep FE-07)
        ↓ бар'єр: повне поле + exact-align
ХВИЛЯ 4 (гібрид + низьке залізо) ───────────────────────────────────────
  FE-14 lazy-render-on-settle              lane-A  [E1] (dep FE-08)
  FE-15 hybrid DOM: input overlay + a11y   lane-B  [E5]
  FE-16 WebGL2 + scalar fallbacks          lane-C  [E0]
  FE-17 kill legacy money-tweens           lane-D  🔴 (apps/web)
```

---

## ХВИЛЯ 0 — КОМПУТ-ФУНДАМЕНТ

### FE-01 — Zero-copy WASM↔GPU міст
**Шар** E0 · **Depends** — · **Lane** A · **Est** M

**TARGET:** новий Rust crate (запропоновано `engine/` під `web/` або `crates/field-engine`); wgpu binding glue
(не існує сьогодні); reuse rust-core CSR ABI патерн.

**CURRENT STATE (звірено):** `kernel/src/wasm.rs` = JSON-string boundary (per-frame катастрофа). rust-core
`lib.rs` = raw C-ABI над linear memory, `out:*mut f64` пише в той самий ArrayBuffer; `memory` експортовано
(`dowiz_kernel_bg.wasm.d.ts:3`), JS вже робить `new Uint8Array(wasm.memory.buffer)` (`dowiz_kernel.js:11`).
**ВСЯ wgpu binding glue = TODO** ("GPU/WGPU flagged next", rust-core lib.rs:7).

**WHY:** [план §E0, дослідж. R3] JSON per-frame = V8 `JSON.parse` 240KB тексту 60×/с main thread = jank.
Найбільший realizability lever.

**TARGET STATE:** Rust пише flat `Vec<f32>` у linear memory → JS `Float32Array::view(&slice)` (0 copy) →
`queue.writeBuffer(buf, 0, view)` (1 upload copy — unavoidable #2). **`writeBuffer` beats mapped buffers
саме з WASM** (mapped range = separate ArrayBuffer → друга copy). Два planes: JSON (transactional, unchanged)
+ raw numeric (per-frame). Export surface `vertex_view() -> (*const f32, usize)`.

**ALGORITHM:**
```
Rust:  fn vertex_buffer() -> &[f32]  // flat, in linear memory
JS:    const {ptr, len} = wasm.vertex_view();
       const view = new Float32Array(wasm.memory.buffer, ptr, len);  // NO COPY
       queue.writeBuffer(gpuBuffer, 0, view);                         // 1 upload
Native wgpu: bytemuck::cast_slice(linear_memory[ptr..]) → write_buffer  // TRUE zero-copy CPU-side
```
Опційно: WGSL compute integrator → storage buffer read by render pass (WebGPU only, positions stay on GPU).

**RED→GREEN GATE:** профіль frame-loop: RED (JSON path) = N `JSON.parse` calls per frame на 4096 particles;
GREEN (view path) = **0** JSON.parse у frame-loop, один writeBuffer. Візуал ідентичний.

**ACCEPTANCE:** ☐ 0 JSON у frame-loop ☐ Float32Array view (no alloc) ☐ writeBuffer шлях ☐ JSON лишається для
transactional. **OUT OF SCOPE:** не чіпати `wasm.rs` JSON (transactional correct); WGSL compute — опційний fast path.

### FE-02 — SoA DOD store + ParticlePool ring
**Шар** E1 · **Depends** — · **Lane** B · **Est** M

**TARGET:** новий `WidgetStore` + `ParticlePool` (Rust).

**CURRENT STATE (звірено):** bebop2 `field.rs` ВЖЕ SoA (modes/row_ptr/col_idx/degrees flat). Легасі
`RevealOverlay.tsx:84` = AoS `Particle[]` + rAF (anti-pattern). particle-cloud.js вже SoA ring (3 Float32Array).

**WHY:** [план §E1, дослідж. R3] cache line 64B: AoS integrate touches 8B/32B = 25% utilization; SoA = 100%,
40-60% speedup. Hand-rolled НЕ bevy_ecs (dynamic composition overhead невиправданий для fixed UI set).

**TARGET STATE:**
```rust
struct WidgetStore {  // hot/warm/cold split
    pos_x: Vec<f32>, pos_y: Vec<f32>, vel_x: Vec<f32>, vel_y: Vec<f32>,  // hot (physics tick)
    size_w: Vec<f32>, size_h: Vec<f32>,                                  // warm (render)
    color: Vec<u32>, flags: Vec<u32>, id: Vec<u32>,                      // cold (DIRTY|VISIBLE|HOVER|ANIMATING|PINNED)
}
struct ParticlePool { pos_x/y, vel_x/y: Vec<f32>, life: Vec<f32>, color: Vec<u32>, head, len }  // ring, spawn overwrites head, ZERO alloc steady
```

**RED→GREEN GATE:** integrate loop cache-utilization RED (AoS baseline) vs GREEN (SoA); ring spawn steady-state
allocation count = 0 (RED: Vec push grows; GREEN: overwrite head, capacity const).

**ACCEPTANCE:** ☐ SoA hot/warm/cold ☐ ring zero-alloc steady ☐ НЕ bevy_ecs. **OUT OF SCOPE:** ECS тільки якщо
UI стане genuinely dynamic heterogeneous (не сьогодні).

### FE-03 — Fixed-timestep accumulator loop
**Шар** E1 · **Depends** FE-02 · **Lane** C · **Est** S

**TARGET:** новий loop runtime.

**CURRENT STATE (звірено):** field.rs `DT_STABLE=0.02` guarded (`:184`), tested `b11_dt_corridor_never_diverges`
(`:490`, 0.05 divergent). Легасі — variable rAF.

**WHY:** [план §E1, дослідж. R3] variable dt на stutter → large dt → explicit integrator past stability →
explode (field 0.05 tested divergent). Fixed guarantees integrator sees ТІЛЬКИ 0.02.

**TARGET STATE:**
```rust
const DT: f32 = 0.02;  // = field::DT_STABLE
const MAX_FRAME: f32 = 0.25;  const MAX_SUBSTEPS: u32 = 5;
// clamp frame_time→accumulator; while accum>=DT && steps<MAX: prev=curr; integrate(DT); accum-=DT; steps++
// alpha=accum/DT; render(lerp(prev,curr,alpha))  // interpolation = SoA SIMD pass
```
Guards: MAX_FRAME clamp (spiral-of-death), MAX_SUBSTEPS cap, dt compile-const=DT_STABLE.

**RED→GREEN GATE:** simulated stutter (inject 50ms frame): RED (variable dt) = integrator sees 0.05 → diverge;
GREEN (fixed) = integrator sees тільки 0.02, no divergence; interpolation smooth @60fps render / 50Hz update.

**ACCEPTANCE:** ☐ DT=DT_STABLE compile-const ☐ MAX_FRAME+MAX_SUBSTEPS guards ☐ interpolated render.
**OUT OF SCOPE:** не міняти field.rs integrator (лише loop навколо).

---

## ХВИЛЯ 1 — РЕНДЕР-ПРИМІТИВИ

### FE-04 — particle-cloud → wgpu (port/compute)
**Шар** E4 · **Depends** FE-01,02 · **Lane** A · **Est** M

**CURRENT STATE (звірено):** particle-cloud.js WebGL2, 3 flat Float32Array ring 4096, GLSL 300es (VS point
size=2+life·6·energy, ⚠️ **blue hardwired 1.0**), FS soft round r>0.25 discard alpha=(1−4r)·life, additive
SRC_ALPHA,ONE. Physics: damp 0.92^(dt/16.6), pointer repulsion f=(1−d²/R²)·0.6, semi-implicit Euler.

**WHY:** [план §1.2, дослідж. A3] це вже DOD renderer template — прямий порт; але RGB truncated (2-channel).

**TARGET STATE:** wgpu port: SoA ParticlePool (FE-02) → vertex/storage buffer via zero-copy (FE-01); WGSL
шейдери (VS/FS з GLSL); **widen meta до full RGBA** (blue не hardwired); WGSL compute integrator (WebGPU) +
CPU simd128 fallback (WebGL2). Reinterpret bursts як Green's-function tracers (готує FE-10).

**RED→GREEN GATE:** візуальний parity RED→GREEN (той самий event burst виглядає ідентично); full-RGBA RED
(blue!=1.0 працює); WebGPU compute path == WebGL2 CPU path (visual parity).

**ACCEPTANCE:** ☐ wgpu port ☐ full RGBA ☐ zero-copy buffer ☐ compute+CPU dual path. **OUT OF SCOPE:** не
змінювати event VOCAB значення (color/energy/burst/swirl калібровані).

### FE-05 — SDF shape pipeline + design-token GPU table
**Шар** E4 · **Depends** FE-01 · **Lane** B · **Est** L

**CURRENT STATE (звірено):** tokens.css :root перелічувано (11 brand+40 status+8 semantic, 8 type steps, 4px).
color-mix/box-shadow/backdrop-blur = no clean GPU analog. Немає SDF рендера.

**WHY:** [план §2.2/§R1.3] всі rects/cards/buttons/borders/shadows = один instanced quad SDF pipeline.

**TARGET STATE:** instanced unit quad ×N (всі rounded-rect = 1 draw call). WGSL:
```
sdRoundBox(p,b,r) = { q=abs(p)-b+r; min(max(q.x,q.y),0)+length(max(q,0))-r }  // per-corner r by quadrant
AA: cov = 1 - smoothstep(-fwidth(d), fwidth(d), d)   // analytic, замість egui CPU feather
border: dBorder=abs(d)-t/2;  gradient: mix(c0,c1,clamp(dot(p-p0,dir)/len,0,1)) або 1×K LUT
shadow: Levien erf7 blurred-rounded-rect (r_eff=√(r_c²+1.25·r_b²)) — ZERO blur passes
```
GPU token table: color-mix **pre-resolved CPU** → concrete RGBA (recompute on tenant switch); RectInstance flat
{rect,cornerRadii,fill:u32,border,borderW,shadow,gradient,flags}. Bind0 UBO {screen,dpr,time,theme_tokens}
(theme switch = 1 uniform write). Відтворити 4-шаровий cascade override-stack.

**RED→GREEN GATE:** відрендерити Storefront card проти CSS-версії — pixel-diff < threshold; box-shadow
erf7 vs CSS elevation match; theme-switch = 1 uniform write (не re-tessellate).

**ACCEPTANCE:** ☐ sdRoundBox+per-corner+AA ☐ token table (color-mix CPU-resolved) ☐ erf shadow no blur pass
☐ cascade override-stack. **OUT OF SCOPE:** backdrop-blur (framebuffer gaussian — окремий pass, не тут);
Fraunces variable font (FE-06).

### FE-06 — MSDF text (cosmic-text shaper + atlas)
**Шар** E4 · **Depends** FE-01 · **Lane** C · **Est** L

**CURRENT STATE (звірено):** text = Tabler webfont `<i class="ti">` + Google Fonts (Inter/DM Sans/.../Fraunces
VARIABLE/Yeseva One). 3 locales sq/en/uk. Немає GPU text.

**WHY:** [план §R1.2] найважче. cosmic-text (shape+BiDi+linebreak+fallback via rustybuzz) → MSDF atlas.

**TARGET STATE:** cosmic-text = shaper+layout (owns line-break=Knuth-Plass, BiDi, fallback); MSDF atlas render:
```
median(RGB)=max(min(r,g),min(max(r,g),b)); pxRange=4; toPixels=pxRange·inverseSqrt(dx²+dy²)
alpha=smoothstep(-0.5,0.5,(median-0.5)·toPixels)  // ОДИН atlas crisp @ANY scale
```
Atlas ranges: Cyrillic(uk)+Albanian(ë/ç)+Tabler icon codepoints. tabular-nums = tnum GSUB at shaping.
Fraunces variable → discretize N opsz atlases (nearest) або bitmap (glyphon) якщо fixed scale достатньо.

**RED→GREEN GATE:** crisp text @ 3 scales (RED: single-SDF corner-round; GREEN: MSDF median sharp); sq/en/uk
render (Cyrillic+ë/ç в atlas); tabular-nums aligned digits.

**ACCEPTANCE:** ☐ cosmic-text shaper ☐ MSDF median AA ☐ 3 locales in atlas ☐ tnum. **OUT OF SCOPE:** text
INPUT/IME (FE-15 hidden DOM); color emoji (accept/exclude explicit).

---

## ХВИЛЯ 2 — ПОЛЕ: ДИНАМІКА

### FE-07 — Layout field (stress-majorization, no jitter)
**Шар** E2 · **Depends** FE-02 · **Lane** A · **Est** L

**CURRENT STATE (звірено):** wavefield.rs FR spring layout (k=√(area/n), repulsion k²/d, attraction d²/k, temp
0.1) — O(n²), unweighted, point-only, JITTERS (force-integration oscillates).

**WHY:** [план §2.1, дослідж. R2] naive force-integration осцилює (=jitter). Stress-majorization ГАРАНТУЄ
монотонний спад.

**TARGET STATE:** upgrade FR → **stress majorization (SMACOF)**: Guttman `X^{k+1}=V⁺B(X^k)X^k` (V=weighted
Laplacian, B from d_ij/‖x_i−x_j‖) — майоризація гарантує `σ(X^{k+1})≤σ(X^k)`. АБО symplectic Euler + wave_energy
Lyapunov → LaSalle. **pixel-snap ПІСЛЯ збіжності** `x_snap=g·round(x*/g)`, freeze при `‖∇E‖<ε_f AND ‖ẋ‖<ε_v`.
Keep FR-repulsion як soft overlap. Rest-length springs (d0_ij = desired gap).

**RED→GREEN GATE:** jitter RED (force-integration осцилює, ‖ẋ‖ не спадає) → GREEN (majorization σ monotone
non-increasing, converges, freeze); snap ПІСЛЯ (RED: snap during injects discontinuous gradient→jitter).

**ACCEPTANCE:** ☐ stress-majorization monotone ☐ Lyapunov freeze ☐ snap-after-converge ☐ rest-length springs.
**OUT OF SCOPE:** exact align/equal-width (FE-13 constraint solver — measure-zero, не поле).

### FE-08 — Motion field (critically-damped spring easing)
**Шар** E2 · **Depends** — · **Lane** B · **Est** M

**CURRENT STATE (звірено):** motion.ts пружини (press{400,25}/enter{260,24}/bounce{500,18}/gentle{180,22})
= (k,d), easing cubic-bezier. AnimatedNumber rAF cubic-out.

**WHY:** [план §2.1, дослідж. R2] cubic-bezier НЕ може overshoot → springs. ζ=1 critical = фізика доброго
easing = критичне демпфування governor.

**TARGET STATE:** per-property spring integrator `ẍ+2ζω ẋ+ω²x=ω²x_target`; (ω,ζ) з motion.ts (tension→ω=√(k/m),
friction→ζ=d/(2√(km))). Регіони: snappy ζ=1 ω≈30 (τ_s≈130ms), fluid ζ≈0.65-0.8, playful ζ≈0.35. Heat-kernel
stagger для global transitions (delay τ_j=dist(source,j)/√α). semi-implicit Euler (stable).

**RED→GREEN GATE:** ζ=1 monotone no-overshoot (RED: overshoot detected → ζ wrong); ζ<1 bounces; stagger
ripple-out delay ∝ graph distance.

**ACCEPTANCE:** ☐ spring integrator (ω,ζ) ☐ ζ=1 no-overshoot ☐ heat-kernel stagger ☐ (ω,ζ) з motion.ts.
**OUT OF SCOPE:** money interpolation (FE-09 — field↔state boundary НІКОЛИ не tween money).

### FE-09 — Field↔state boundary (money-never-tween guard) 🔴
**Шар** E3 · **Depends** — · **Lane** C · **Est** S · **RED-LINE**

**CURRENT STATE (звірено):** легасі 4 money-tween порушення (AnimatedNumber/CountUpPrice). Канонічний web/ =
integer-cent static (правильно). kernel = integer money source.

**WHY:** [план §2.2 RED#3, red-line] field continuum interpolate money → `$12.4999` mid-transition VIOLATES.
Money = discrete state, поле лише ПРЕЗЕНТУЄ decided integer.

**TARGET STATE:** контракт-межа: поле малює лише **вже-обчислене** integer-значення з kernel; будь-яке
value з money-type НІКОЛИ не проходить через spring/heat/interpolation. Type-level guard: money = discrete
channel, не field channel. Immediate redraw settled number = no count-up за конструкцією.

**RED→GREEN GATE:** RED — спроба анімувати money value через field → компіляційна/runtime помилка або guard
reject; field-interpolated total НІКОЛИ не показує проміжне ($12.4999). GREEN — money стрибає integer-to-integer.

**ACCEPTANCE:** ☐ money НЕ field channel ☐ no intermediate money value ☐ integer jump. **OUT OF SCOPE:** 🔴
RED-LINE — не чіпати kernel money-логіку; лише presentation boundary.

---

## ХВИЛЯ 3 — ПОЛЕ: СЕМАНТИКА

### FE-10 — Feedback field (Green's function unification)
**Шар** E2 · **Depends** FE-04 · **Lane** A · **Est** M

**CURRENT STATE (звірено):** particle-cloud event→vocab (order/delivered/dispatch_failed бурсти) — one-shot
decorative per component.

**WHY:** [план §2.1, дослідж. R2.4] кожна дія = source impulse; відгук = Green's function. ОДНЕ механізм
замість per-component feedback коду.

**TARGET STATE:** `U(x,t)=∫∫G·S`; tap δ→ripple (2D damped wave G, Material ripple); дискретна
`G_ij=Σφ_k(i)φ_k(j)g_k(t)`, `g_k=(1/mω_dk)e^{−ζ_kω_k t}sin(ω_dk t)`. Event→source vocab (узагальнення
particle VOCAB): tap=δ wave ζ0.6, success=Gaussian HEAT no-overshoot, order=impulse wave ζ<1, error=high-λ
shake, loading=sustained. Particles = tracers seeded ∝|∇U| advected by U̇. Ripples+particles = 2 рендери 1 поля.

**RED→GREEN GATE:** одна дія = один field impulse (RED: per-component feedback код; GREEN: unified source→G);
tap ripple = expanding front radius c·t; error shake decays ~4/(ζω).

**ACCEPTANCE:** ☐ Green's function feedback ☐ event→source vocab ☐ particles = field tracers ☐ unified (no
per-component). **OUT OF SCOPE:** не міняти particle renderer (FE-04); feedback лише driver.

### FE-11 — Focus field (potential wells)
**Шар** E2 · **Depends** FE-08 · **Lane** B · **Est** M

**CURRENT STATE (звірено):** stabilizer.rs potential_well `½Σk(θ−b)²` + tanh saturate + forbidden-zone bump.
Легасі: per-component hover/focus (framer whileHover).

**WHY:** [план §2.1, дослідж. R2.3] ОДНЕ V драйвить ВСЕ emphasis, no per-component focus код.

**TARGET STATE:** `V(x)=−Σ A_i exp(−‖x−c_i‖²/2σ²) + Σ B_m bumps`; cursor `m p̈=−∇V−γṗ` + tanh saturate (magnetic
pull не yank). Readouts (одне V): scale `s₀(1+β(−V)/V_max)`, brightness Boltzmann `L₀+Δexp(−V/T)`, blur DoF
`b_max|V−V_focus|/range`, saturation. Local minima=snap/dock; focus change=well spring (via FE-08). Expose
`∇V=k(θ−b)` restoring force (stabilizer gap).

**RED→GREEN GATE:** одне V драйвить scale+brightness+blur (RED: per-component; GREEN: readouts of single V);
snap lands nearest basin minimum; focus change animates via spring.

**ACCEPTANCE:** ☐ potential field V ☐ readouts (scale/bright/blur/sat) ☐ snap/dock local minima ☐ ∇V exposed.
**OUT OF SCOPE:** не дублювати V per-component.

### FE-12 — Spectral structure layout (φ₂,φ₃ embedding)
**Шар** E2 · **Depends** FE-07 · **Lane** C · **Est** M

**CURRENT STATE (звірено):** bebop2 field.rs LaplacianSpectrum eigenVECTORS column-major modes[rank*n+i] via
jacobi_eigen (O(n³)). wavefield.rs graph_laplacian_eigs = eigenVALUES only.

**WHY:** [план §2.1, дослідж. R2.5] Hall spectral: x_i=(φ₂,φ₃) = tangle-free global layout, no local minima →
initializer що dodges FR.

**TARGET STATE:** L=D−W → smallest nontrivial (λ₂,φ₂),(λ₃,φ₃) via bebop2 field.rs eigensolver (додати coords_2d
helper: x_i=center+S(modes[1*n+i], modes[2*n+i]), scale 1/√λ) → hand to FE-07 constrained majorization+snap.
Eigenvalue = hierarchy level. Lanczos для великих (O(n³) Jacobi infeasible). Decoration = same operator finer
scale (golden-angle 137.5° phyllotaxis для particle seeding).

**RED→GREEN GATE:** spectral layout tangle-free RED (FR local-minimum tangle) → GREEN (φ₂φ₃ untangled на
реальному графі меню); clusters separate along φ₂.

**ACCEPTANCE:** ☐ coords_2d(φ₂,φ₃) helper ☐ initializer для FE-07 ☐ Lanczos для великих ☐ golden-angle seeding.
**OUT OF SCOPE:** не переписувати jacobi_eigen (reuse); pixel-exactness = FE-07/FE-13.

### FE-13 — Constraint solver bridge (exact alignment)
**Шар** E3 · **Depends** FE-07 · **Lane** D · **Est** L

**CURRENT STATE (звірено):** немає. Поле дає soft arrangement (measure-zero не тримає exact equality).

**WHY:** [план §2.2 RED#1, дослідж. R2.6] equality (align/equal-width) = measure-zero для smooth field →
constraint solver.

**TARGET STATE:** `minimize E(X) s.t. AX=b (align/equal-width) CX≤d (min gaps/contain)`; constrained stress-
majorization + gradient projection (після Guttman step project на constraint polytope, small QP); equalities
exact via Cassowary incremental simplex (=Apple Auto Layout). Physics=soft global, constraint=hard pixel-exact.

**RED→GREEN GATE:** `expect(colA.right).toBe(colB.left)` RED (field-only sub-pixel drift) → GREEN (constraint
exact equal); equal-width columns exact after settle.

**ACCEPTANCE:** ☐ Cassowary/QP exact equality ☐ gradient projection after majorization ☐ physics owns soft.
**OUT OF SCOPE:** не робити ВСЕ constraint (тільки hard equalities; soft = поле).

---

## ХВИЛЯ 4 — ГІБРИД + НИЗЬКЕ ЗАЛІЗО

### FE-14 — Lazy-render-on-settle (battery)
**Шар** E1 · **Depends** FE-08 · **Lane** A · **Est** M

**CURRENT STATE (звірено):** resonator.ts:137 convergence detector (error<delta_threshold Converged); field.rs
active_diffuse gate |Δu|<eps. Немає lazy-render.

**WHY:** [план §E1, дослідж. R3] naive immediate 60fps burns battery. Найбільший battery lever.

**TARGET STATE:** `should_render = input_pending || !physics_settled || animation_active || external_change`;
none → НЕ планувати наступний rAF, dormant. `physics_settled` = max SoA step delta < ε = resonator Converged =
field active_diffuse gate. Hysteresis K=3 consecutive (resonator stall_patience). Lyapunov watchdog: divergence
→ force render never dormant.

**RED→GREEN GATE:** static екран → rAF СПРАВДІ зупиняється (RED: 60fps forever; GREEN: dormant, 0 wake-ups);
instantly responsive on touch; divergence never dormant.

**ACCEPTANCE:** ☐ dormant on settle ☐ physics_settled = resonator Converged ☐ hysteresis K=3 ☐ watchdog force-wake.
**OUT OF SCOPE:** не будувати новий convergence detector (reuse resonator).

### FE-15 — Hybrid DOM: input overlay + a11y semantic mirror
**Шар** E5 · **Depends** — · **Lane** B · **Est** L

**CURRENT STATE (звірено):** легасі a11y DOM-based (role/aria-live/sr-only/focus-visible). AccessKit no web
backend 2026. Немає гібрид-межі.

**WHY:** [план §4.1, дослідж. R1.5] canvas = zero semantic DOM. Text-input/IME/autofill cannot be ARIA-faked;
screen reader reads DOM tree.

**TARGET STATE:** (a) transparent DOM `<input>` overlay над GPU-drawn field (keep type=email/tel для autofill;
mobile keyboard summon; IME composition); (b) hidden transparent DOM **semantic mirror** (Flutter CanvasKit):
role/aria-label/tabindex/rect/state, reconcile per-frame з immediate widget list. Позначити permanent losses
(Ctrl+F/translate/reader).

**RED→GREEN GATE:** screen-reader читає mirror (RED: canvas invisible to AT; GREEN: announces role+label);
form приймає typed input + autofill + mobile keyboard; IME composition works.

**ACCEPTANCE:** ☐ input overlay (IME/autofill/keyboard) ☐ semantic mirror (role/label/focus) ☐ reconcile
per-frame ☐ permanent losses documented. **OUT OF SCOPE:** public SEO /s/:slug — окремо (лишається SSR DOM,
не мігрує взагалі).

### FE-16 — WebGL2 + scalar SIMD fallbacks
**Шар** E0 · **Depends** — · **Lane** C · **Est** M

**CURRENT STATE (звірено):** particle-cloud вже WebGL2. wgpu dual-targets. Немає fallback discipline.

**WHY:** [план §6, дослідж. R3] low-end long tail: WebGL2 baseline universal, WebGPU enhancement; simd128 +
mandatory scalar (determinism).

**TARGET STATE:** feature-detect requestAdapter SUCCESS (не object presence) → WebGPU compute path; інакше
WebGL2 CPU writeBuffer path. simd128 f32x4 integrator (pos_x[i..i+4]+=vel_x*dt) + scalar fallback (bit-identical).
Design to downlevel_webgl2_defaults (tex 2048, 4 bind groups). 30fps design budget.

**RED→GREEN GATE:** WebGL2-only пристрій рендерить (RED: assumes WebGPU→fail; GREEN: WebGL2 path works);
simd128 == scalar bit-identical (determinism RED: divergent results).

**ACCEPTANCE:** ☐ WebGL2 baseline works ☐ WebGPU enhancement (requestAdapter success) ☐ simd128+scalar
bit-identical ☐ downlevel limits. **OUT OF SCOPE:** не робити WebGPU обов'язковим.

### FE-17 — Kill legacy money-tweens 🔴
**Шар** — · **Depends** — · **Lane** D · **Est** S · **RED-LINE (money)**

**CURRENT STATE (звірено):** 4 порушення apps/web: ClientLayout:154 (AnimatedNumber cart total), EarningsPage:47-176
(CountUpPrice), DashboardPage:421 (revenue), AnalyticsPage:262 (revenue+avg). Culprit = AnimatedNumber.tsx +
CountUpPrice. Канонічний web/ чистий.

**WHY:** [план §1.1, операторська red-line money-never-tween] легасі досі анімує money 4 місцях.

**TARGET STATE:** drop AnimatedNumber/CountUpPrice для money-bound значень → static integer-cent (як
канонічний web/). Non-money counts (order counts DashboardPage:423) можуть лишити odometer.

**RED→GREEN GATE:** RED — money value показує проміжне під час зміни (tween); GREEN — money стрибає
integer-to-integer, no interpolation. Guardrail: grep money-bound AnimatedNumber = 0.

**ACCEPTANCE:** ☐ 4 сайти money static ☐ non-money counts дозволені ☐ grep guardrail. **OUT OF SCOPE:** 🔴 не
чіпати money-обчислення; лише presentation (tween→snap).

---

## ДОДАТОК A — БЛЮПРИНТ → ПЛАН → ДОСЛІДЖЕННЯ

| FE | Система | План | Дослідж |
|----|---------|------|---------|
| 01 | zero-copy міст | §E0 | R3/A4 |
| 02 | SoA DOD store | §E1 | R3 |
| 03 | fixed-timestep | §E1 | R3 |
| 04 | particle→wgpu | §E4 | A3/R1 |
| 05 | SDF + tokens | §E4 | R1/A2 |
| 06 | MSDF text | §E4 | R1/A2 |
| 07 | layout field | §2.1 | R2/A3 |
| 08 | motion field | §2.2 | R2/A2 |
| 09 | field↔state money | §2.2 RED#3 | R2 🔴 |
| 10 | feedback Green's | §2.1 | R2/A3 |
| 11 | focus wells | §2.1 | R2/A3 |
| 12 | spectral layout | §2.1 | R2/A4 |
| 13 | constraint solver | §2.2 RED#1 | R2 |
| 14 | lazy-render | §E1 | R3 |
| 15 | hybrid DOM a11y | §4.1 | R1/A1 |
| 16 | WebGL2 fallback | §6 | R3/R1 |
| 17 | kill money-tween | §1.1 | A1 🔴 |

## ДОДАТОК B — ІНВАРІАНТИ, ЯКІ ЖОДЕН БЛЮПРИНТ НЕ СМІЄ ПОРУШИТИ

1. **Поле НЕ тримає exact align/money/crisp-text** — три дискретні шари (constraint/SDF/state) за конструкцією.
2. **Money НІКОЛИ не field channel** — поле презентує integer з kernel, ніколи не інтерполює (red-line).
3. **Determinism** — authoritative compute CPU-side; GPU=display; scalar==SIMD bit-identical.
4. **Boundary kind = update frequency** — transactional→JSON, per-frame→zero-copy view. Ніколи JSON у frame-loop.
5. **Fixed dt = DT_STABLE** — integrator ніколи не бачить divergent 0.05.
6. **Lazy-render on settle** — dormant коли physics_settled (battery); divergence force-wake.
7. **Гібрид, не чистий GPU** — forms/a11y-mirror/SSR-menu лишаються DOM; big-bang заборонено.
8. **Chebyshev spectral для дифузії** — не coherence single-step (правильний знак).

---
*Кінець блюпринтів. 17 робочих одиниць, 5 хвиль, кожна з RED→GREEN гейтом. Джерело: FIELD-UI-ENGINE-PLAN.md +
4 аналізи коду + 3 математичні дослідження. Критерій — реалізовність на слабкому залізі + falsifiable proof.
Автор синтезує; виконують агенти. Гібрид чесний: поле малює динаміку, DOM тримає a11y/forms/SEO.*