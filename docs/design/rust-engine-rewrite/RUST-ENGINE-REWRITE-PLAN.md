# DOWIZ — ПЛАН РЕРАЙТУ ІНТЕРФЕЙСУ НА НАТИВНУ RUST/WASM v1.0
## Усі залишки TS/Node/JS → нативна Rust/WASM бібліотека, щоб двигун був справді растовим
### Синтез: вичерпний per-file інвентар × архітектура нативного крейта × field-UI двигун × kernel/bebop Rust. Критерій — buildable, збереження домену, чесна thin-shell межа.

> **Що це.** План перетворення інтерфейсного шару dowiz на **нативну Rust/WASM бібліотеку**: детермінований
> домен (order-machine, money, channel, cart, geo) стає Rust; рендеринг стає wgpu; JS лишається лише як
> **незводима thin-shell мембрана** браузерних API. Стоїть на плечах двигуна
> [field-ui-engine](../field-ui-engine/) (FE-01..17) і дизайну [dowiz-interfaces](../dowiz-interfaces/) (DZ-01..12).
>
> **Джерела.** Вичерпний per-file інвентар (~33.6k LOC, ~205 файлів) + архітектура нативного крейта (crate
> workspace, channel.js/particle-cloud.js порти, thin-shell межа, toolchain, migration). Конспект:
> [rust-rewrite/INDEX.md]. Кожна прив'язка звірена на диску (`file:line`).
>
> **Незламний контракт.** Домен, що «думає», стає Rust; JS лишається лише мембраною браузерних API + data-шаром.
> Money-authority = kernel integer (JS-дублікати видаляються). Local-first: per-frame boundary = numeric
> memory-views, ніколи JSON.

---

## 0. ГОЛОВНИЙ РЕЗУЛЬТАТ: НАСКІЛЬКИ RUST ВЖЕ Є ДВИГУНОМ

**Кількісний вирок (з інвентаря ~33.6k LOC інтерфейсного коду):**

```
Детермінований DOMAIN (order-machine/money/channel/cart/geo/anomaly/ETA):
  • ВЖЕ Rust у WASM (kernel/*.rs)        1,390 LOC Rust — вже authority, вже compiled 191KB
  • АБО <1k LOC pure JS/TS що портується 1:1 (geo-anim 134, cart 141, cartReconcile 63, messenger 38)

ДУБЛІКАТИ Rust-kernel → DELETE outright:  ~230 LOC (channel.js 146, money.ts 86, utils-transition-table)

Незводима browser-glue (KEEP-THIN-SHELL): ~9,000 LOC (15 Web API, no WASM equivalent)

DATA (locale/tokens/config):              ~5,700 LOC (i18n-catalog 3830 сам, motion, tokens.css, palette)

Regenerated wasm-bindgen glue:            ~700 LOC (auto-emitted)

React/Svelte VIEW rendering:              ~18,000 LOC = ШАР, ЩО ДВИГУН МАЄ СТАТИ
```

**Наративний вирок.** Логіка, що «думає», = **1.4k рядків Rust сьогодні**. Усе, що JS зараз обчислює сам
(cart totals, money, order transitions, anomaly folding, ETA/geo), **collapses у kernel**, лишаючи JS = тонка
мембрана браузерних API + document/locale data. Тобто «зробити двигун растовим» — це не 33k рядків перепису:
це **видалити ~230 дублікатів, портувати ~1k pure-logic, і замінити ~18k view-rendering на wgpu-поле**, тоді
як домен уже растовий.

**П'ять істин синтезу:**

1. **channel.js — чистий дублікат, DELETE = 3 рядки.** Byte-for-byte порт `analytics.rs`; kernel уже exports
   `channel_ledger_js`/`reduce_anomalies_js` у shipped wasm; єдиний споживач OwnerDashboard.svelte з коментарем
   «until WASM-binding wave» (уже landed). Delete = rewire на наявний export, 0 нового Rust.

2. **particle-cloud.js → wgpu порт ВИПРАВЛЯЄ латентний баг.** GLSL hardwires `blue=1.0` → `delivered`
   рендериться **рожевим**, `dispatch_failed` «кров» — **синьо-фіолетовим**. Full-RGBA у wgpu-порті (FE-04) не
   косметика — це виправлення бага (visual output legitimately changes). Плюс мертвий `a_seed` attribute
   (declared+uploaded, ніколи не читається) зникає.

3. **kernel вже authority; JS-дублікати видаляються.** channel.js + money.ts + utils-transition-table = ~230
   LOC що дублюють Rust — delete outright. Дві cart-реалізації (CartProvider + use-cart) → одна `kernel/src/cart.rs`.

4. **Дві wasm-межі, розділені частотою оновлення.** kernel JSON boundary (transactional order placement —
   ПРАВИЛЬНИЙ, лишається) vs engine numeric memory-views (per-frame — новий, zero-copy, zero-JSON). Розділяюча
   лінія = **частота**, не тип даних. JSON per-frame = катастрофа (V8 JSON.parse 240KB 60×/с).

5. **Toolchain уже працює.** Наявний `cargo build wasm32 → wasm-bindgen --target web → ES-module + .wasm +
   ~1KB loader` (kernel вже так робить). Двигун повторює це verbatim. Vite бандлить лише thin loader, ніколи
   логіку. «No JS bundler for engine logic» досягнуто наявним шляхом.

---

## 1. АРХІТЕКТУРА: `dowiz-engine` CARGO WORKSPACE

Наразі `kernel/` — **standalone crate** (не workspace), cdylib+rlib, → `dowiz_kernel_bg.wasm` 191KB. bebop
spectral math — у **окремому репо** (cross-repo rule: НЕ reference з dowiz → **vendor**).

**Пропозиція:** promote `kernel/` у workspace `crates/` + додати engine-крейти. **Два wasm-артефакти, розділені
частотою:** `dowiz_kernel` (JSON transactional, unchanged) + `dowiz_engine` (numeric per-frame).

| Crate | Шар | Призначення | Reuse / New |
|---|---|---|---|
| **`kernel`** (є) | E3 | discrete-state authority: decide/fold (order_machine), integer money (money.rs), ChannelLedger (analytics.rs); keeps own JSON wasm.rs transactional | 100% reuse untouched |
| **`field-math`** | E2-math | VENDORED no_std spectral core: Laplacian eigenmodes + Chebyshev + FFT (blur) + algebra; `core`+`alloc`, **zero deps** | vendor bebop2 field.rs (DT_STABLE=0.02) / chebyshev / fft / algebra — тести вже зелені |
| **`field`** | E2 | 5 аспектів: layout (stress-majorization), motion (critical spring), feedback (Green's), focus (potential well), structure (spectral φ₂φ₃) | reuse field-math; new FE-07/08/10/11/12 |
| **`state`** | E3 | thin adapter kernel + **money-never-tween type boundary** (FE-09: money = discrete channel, ніколи field channel) | reuse kernel; new facade+guard |
| **`store`** | E1 | DOD SoA WidgetStore (hot/warm/cold) + ParticlePool ring (FE-02) + fixed-timestep accumulator DT=0.02 (FE-03) | pattern particle-cloud ring; new |
| **`render`** | E4 | wgpu pipelines: particle billboards (FE-04 GLSL→WGSL), SDF rounded-box (FE-05), MSDF text cosmic-text (FE-06); single pass painter-order | new; deps wgpu+cosmic-text |
| **`input`** | E5 | Intent/FieldPos/InputSource; pointer→field-force; event→source impulse | new |
| **`tokens`** | E4-data | design-token GPU table: color-mix pre-resolved CPU→RGBA, 4-layer cascade → 1 UBO write | new |
| **`i18n`** | E4-data | locale key tables sq/en/uk feeding shaper (~1296 keys) | data-only |
| **`shell`** | E5/E0 | **ЄДИНИЙ** crate з wasm-bindgen; <10 numeric/memory-view exports; integrates all; holds linear-memory staging | new; cdylib; pattern rust-core zero-copy ABI |

**Dep graph:**
```
shell (cdylib · wasm-bindgen · ЄДИНА JS-межа)
 ├─ render ── tokens, i18n, field-math
 ├─ field  ── field-math          (spectral substrate; no_std+alloc, 0 deps)
 ├─ store
 ├─ state  ── kernel              (reuse: order-machine+money+analytics — UNTOUCHED)
 └─ input
kernel (cdylib+rlib · own JSON wasm.rs — TRANSACTIONAL order placement only)
```
Тільки `shell`+`kernel` = cdylib; engine crates = rlib. `field-math` = **strict floor**: `#![no_std]`+alloc,
zero deps, wasm32-clean (bebop2 field.rs вже meets — `use alloc::vec::Vec`).

---

## 2. ДВІ WASM-МЕЖІ (частота, не тип)

### 2.1 kernel JSON boundary — TRANSACTIONAL (правильний, лишається)
`place_order_js`/`apply_event_js`/`channel_ledger_js`/`reduce_anomalies_js` — JSON String both ways. Few
calls/session. Правильно для transactional. **Не чіпати.**

### 2.2 engine numeric memory-views — PER-FRAME (новий, zero-JSON)
Engine WASM export surface (<10, zero-JSON):
```
memory        : WebAssembly.Memory                 // вже exported
engine_new()                                        // build store+field+pools+pipelines
tick(frame_ms: f32) -> u32                          // fixed-timestep advance; dirty/should_render bits (FE-14)
instance_ptr() -> u32 ; instance_len() -> u32       // Float32Array view particle instances (zero-copy)
widget_ptr()   -> u32 ; widget_len()   -> u32       // Float32Array view SDF rect instances
on_pointer(px: f32, py: f32)                        // one-line DOM forward
on_event(kind: u32, count: u32)                     // burst/feedback impulse (enum, НЕ string)
set_flags(bits: u32)                                // reduced_motion | webgl2 | ...
resize(w: f32, h: f32, dpr: f32)
```
JS reads frame **NO copy NO parse**: `new Float32Array(memory.buffer, instance_ptr(), instance_len())` →
`writeBuffer`. = rust-core ABI (`*mut f64` same ArrayBuffer). **Розділяюча лінія = частота оновлення, не тип
даних.**

---

## 3. ДВА ФЛАГМАНСЬКІ ПОРТИ

### 3.1 channel.js → DELETE (proven byte-mirror)
- Header: «port of kernel/src/analytics.rs»; line-by-line: STAGE_ORDER=funnel stage_order, ALLOWED_NEXT=
  allowed_next, assertTransition=assert_transition, ingest dup-lock=ChannelLedger::ingest, reduceAnomalies
  BTreeMap-fold=reduce_anomalies.
- Rust вже shipped: `channel_ledger_js` (wasm.rs:285), `reduce_anomalies_js` (:293), у .d.ts. Glue вже wraps
  (kernel.js:50-53).
- Verdict **DELETE, не port** (dashboard TRANSACTIONAL, once at mount ~16 events → JSON correct). Rewire
  OwnerDashboard:17 → `channelLedger(SAMPLE_EVENTS)`; `funnel[selectedChannel] ?? ZERO_FUNNEL`.
- **Deliverable: −146 LOC, ~3-line Svelte rewire, 0 нового Rust.**

### 3.2 particle-cloud.js → повний Rust/wgpu порт (FE-04)
**Deliverable:** `store::ParticlePool` + `render::particles` + 3 shell exports; delete 319 LOC; swap
CourierTrack:15,71-73.
- **GLSL 300es → WGSL:** point sprites (`gl_PointSize`) не в WGSL → **instanced billboard quad** (4 verts
  tri-strip). VS: `life01=life/maxLife`, `size=2+life01·6·energy`, corner expand pixel-space, ndc y-flip. FS:
  `d=uv·0.5`, `r=dot(d,d)`, `r>0.25 discard`, `a=(1−4r)·life01`, additive blend SrcAlpha,One. naga compiles
  WGSL runtime; WebGL2 fallback cross-compiles WGSL→GLSL auto.
- **Full RGBA fixes bug:** `Inst {pos vec2, life vec2, color vec4}` — виправляє hardwired-blue (delivered
  рожевий→gold, blood синьо-фіолетовий→red).
- **Ring SoA:** MAX=4096, pos_x/y+vel_x/y+life+max_life+color[[f32;4]] (drop dead seed, widen meta→RGBA),
  `inst` flat interleaved staging → ONE writeBuffer (collapse 3 bufferSubData), steady alloc=0.
- **VOCAB Rust table** verbatim (не міняти значення); palette runtime-mutable.
- **Physics integrator** (ms-calibrated, під FE-03 fixed loop dt_ms=20ms): damp `0.92^(dt/16.6)`, pointer
  repulsion, semi-implicit Euler, life swap-remove, energy `0.95^decay`, burst. RNG Math.random→xorshift32.
  SIMD f32x4 + scalar bit-identical.
- **Dual path:** WebGPU compute (positions stay GPU) / WebGL2 CPU step. RED→GREEN: visual parity EXCEPT
  corrected colors; blue!=1.0 proven; compute==CPU bit-identical.

---

## 4. НЕЗВОДИМА THIN-SHELL JS (чесний мінімум)

Усе, що торкається браузерного API без WASM-еквіваленту, або worker/document scope:

| Відповідальність | Чому не Rust | Наявний актив |
|---|---|---|
| wasm bootloader + feature-detect (requestAdapter/SIMD/WebGL2) | wasm не бутстрапить себе; adapter probe = JS async | kernel.js:10-14 ready() |
| lazy rAF arm/cancel (FE-14) | requestAnimationFrame = DOM API | particle :255-260 |
| DOM event → on_pointer (one-liner) | pointer events на DOM nodes | particle :279-285 |
| browser-API shims Rust invokes through: Push/getUserMedia/WebSpeech/WebXR/clipboard/file/History/permissions | permission/gesture-gated, no wasm binding | push.js isolated |
| a11y semantic DOM mirror (FE-15) | AccessKit no web backend 2026; screen readers read DOM | hand-roll |
| transparent `<input>` overlay (IME/autofill/mobile-kbd) | canvas can't summon mobile keyboard; IME DOM-only | hand-roll |
| service worker (sw.js) | separate thread, install/push events | web/public/sw.js |
| WebSocket/fetch/token-refresh/localStorage | browser APIs | apiClient/useWebSocket/auth/safeStorage |

**15 Web API категорій ~9k LOC** лишаються JS назавжди. Це архітектура (Rust owns compute+state+render; JS
owns browser-chrome membrane), не борг.

---

## 5. TOOLCHAIN (no JS bundler for engine logic)

- **Target** wasm32-unknown-unknown; field-math `--no-default-features` no_std; shell/render link wgpu+cosmic-text.
- **wasm-bindgen (as today), НЕ trunk** (Astro/Svelte SSR ≠ Rust-only-app). Path: `cargo build wasm32` →
  `wasm-bindgen --target web` → ES-module+.wasm committed `web/src/lib/engine/` + ~1KB loader `import init`.
  Vite бандлить ЛИШЕ thin loader, ніколи логіку (logic=prebuilt .wasm). = «no JS bundler for engine logic».
- **WGSL:** `include_str!` embedded, naga compiles runtime (removes GLSL compile), WebGL2 cross-compiles WGSL→GLSL.
- **Size budget ≤2MB gzip** dowiz_engine (kernel 191KB; wgpu+cosmic-text = cost centers). Levers: `opt-level="z"`
  + lto + panic=abort + wasm-opt -Oz + strip + talc allocator + feature-gate cosmic-text sq/en/uk (MSDF atlas =
  DATA, не wasm). **LEAN first slice:** FE-04 particle alone doesn't need full wgpu → raw web-sys WebGL2/WebGPU
  keeps island small; adopt wgpu at Wave 1 (SDF+text).
- **Astro SSR stays** SEO /s/:slug DOM; engine = `client:only` svelte island, onMount import loader→canvas→
  engine_new+arm rAF (= CourierTrack:67-93 backend swapped); astro.config +Vite wasm stanza.

---

## 6. МІГРАЦІЯ (island-by-island, canonical web/ beachhead)

Big-bang forbidden; Gain−Loss>0; text-input never pure-GPU. Keep-running invariant: кожен Rust-island mounts
behind SAME Astro mount point, lazy `client:only` WASM, degrade to Svelte/WebGL2 коли WASM/WebGPU absent
(CourierTrack:77-81 pattern). Два wasm-модулі coexist; kernel ніколи не rewritten, лише re-consumed.

1. **Workspace + field-math + store + shell skeleton (Wave0 FE-01/02/03).** Promote kernel→crates/, vendor
   bebop2 (re-run tests green), store ring zero-alloc + fixed loop DT=0.02. No app change; engine wasm builds
   green headless.
2. **DELETE channel.js EARLY (independent).** Transactional read; kernel exports ship. Rewire OwnerDashboard.
   Proves «kernel authority, JS mirrors deleted» smallest change. −146 LOC.
3. **Particle port = island #0 (FE-04).** Decorative (rank#1), one island. store::ParticlePool +
   render::particles + 3 exports. Keep running: CourierTrack static text stays, canvas backend swap. DELETE
   particle-cloud.js after parity gate.
4. **Render primitives (Wave1 FE-05 SDF+tokens / FE-06 MSDF).** One Storefront card pixel-parity vs CSS.
5. **Field dynamics (Wave2 FE-07 layout / FE-08 motion / FE-09 money 🔴).** One island Svelte→field. FE-09
   codifies existing integer-static invariant (not behavior change).
6. **Semantics+hybrid (Wave3-4 FE-10..16).** Green's/potential-well/spectral + permanent `<input>` overlay +
   a11y DOM mirror. Forms/a11y/SSR-menu stay DOM permanently.
7. **Legacy apps/web LAST (FE-17 🔴).** React SPA untouched till end; 4 money-tween killed separate red-line.

---

## 7. ПІДСУМКОВИЙ ПАСПОРТ

**Система:** нативна Rust/WASM бібліотека інтерфейсного двигуна dowiz. Домен (order/money/channel/cart/geo)
= Rust; рендеринг = wgpu; JS = thin-shell мембрана браузерних API. Стоїть на field-ui двигуні (FE-*) + dowiz
дизайні (DZ-*).

**Несуча основа:** `dowiz-engine` Cargo workspace (kernel reuse + field-math vendored no_std + field/state/
store/render/input/tokens/i18n/shell); дві wasm-межі (JSON transactional + numeric per-frame); наявний
wasm-bindgen toolchain.

**Головні числа:** домен «що думає» = 1.4k Rust вже; delete ~230 дублікатів (channel.js/money.ts/utils);
port ~1k pure-logic (geo/cart/messenger); ~9k irreducible thin-shell (15 Web APIs); ~5.7k DATA (locale 3830);
~18k view→wgpu-поле; engine <10 exports zero-JSON; size ≤2MB gzip; DT=0.02.

**Обсяг:** не 33k рерайт — а delete дублікати + port pure-logic + replace view-rendering wgpu-полем (домен уже
растовий). One Cargo workspace + one vendored no_std math crate (tests green) + one cdylib shell + two deletes
(channel/particle) + existing toolchain reused verbatim.

**Ключовий інваріант.** Логіка, що думає, стає Rust; JS лишається мембраною браузерних API + data. Money-
authority = kernel integer (дублікати видаляються). Per-frame boundary = numeric memory-views ніколи JSON.
Домен уже растовий; «зробити двигун растовим» = видалити дублікати, портувати pure-logic, замінити view-шар
wgpu-полем — тоді як kernel лишається authority, а JS стискається до тонкої мембрани.

---
*Кінець плану. Версія 1.0. Блюпринти — у BLUEPRINTS-RUST-ENGINE-REWRITE.md. Стоїть на docs/design/
field-ui-engine/ + dowiz-interfaces/. Синтез: per-file інвентар + crate-архітектура; критерій — buildable +
збереження домену + чесна thin-shell межа.*