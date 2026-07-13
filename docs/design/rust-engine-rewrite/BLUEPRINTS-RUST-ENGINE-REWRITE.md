# DOWIZ RUST-ENGINE РЕРАЙТ — БЛЮПРИНТИ
## Implementation-ready робочі одиниці для агентів-виконавців

> Похідний від [RUST-ENGINE-REWRITE-PLAN.md](RUST-ENGINE-REWRITE-PLAN.md). Стоїть на двигуні FE-01..17
> ([field-ui-engine](../field-ui-engine/)) і дизайні DZ-01..12 ([dowiz-interfaces](../dowiz-interfaces/)).
> RW-блюпринти = **рерайт-механіка** (delete дублікати / port pure-logic / crate scaffold / thin-shell межа),
> що дротує TS/JS-інвентар у Rust-крейти. Джерела: per-file інвентар + crate-архітектура. Стан звірено `file:line`.

## 0. КОНТРАКТ ВИКОНАВЦЯ

1. **Kernel = authority.** Ніколи не переписувати kernel-логіку; тільки reuse/re-consume. JS-дублікати kernel
   → DELETE (не re-port).
2. **Дві межі за частотою.** Transactional → JSON (kernel, unchanged). Per-frame → numeric memory-views
   (engine, zero-JSON). Ніколи JSON у frame-loop.
3. **Keep-running.** Кожен island mounts behind SAME Astro mount, lazy `client:only`, degrade коли WASM/WebGPU
   absent. Big-bang forbidden.
4. **Money red-line.** kernel integer authority; JS money-дублікати delete; per-frame money = presented never
   interpolated.
5. **field-math floor.** `#![no_std]`+alloc, zero deps, wasm32-clean; vendor bebop2 (cross-repo rule), тести
   лишаються зелені.
6. **RED→GREEN.** Кожен RW має falsifiable gate.

## 1. ХВИЛІ

```
ХВИЛЯ 0 (crate scaffold + zero-risk deletes) ──────────────
  RW-01 dowiz-engine Cargo workspace + field-math vendor   dep FE-01/02/03
  RW-02 DELETE channel.js → kernel exports (independent)    🟢 zero-risk
  RW-03 DELETE money.ts + utils transition-table (dup)      🔴 money red-line
ХВИЛЯ 1 (particle port = island #0) ──────────────────────
  RW-04 particle-cloud.js → store::ParticlePool+render      dep FE-04, RW-01
  RW-05 shell crate + <10 numeric exports zero-JSON          dep FE-01, RW-01
ХВИЛЯ 2 (port pure-logic → kernel crates) ────────────────
  RW-06 geo-anim → kernel/src/geo.rs (+ delivery-zone)       PORT pure
  RW-07 cart consolidate (2 impls) → kernel/src/cart.rs      PORT+DELETE-dup
  RW-08 messenger + formatMoney → Rust utils                 PORT pure
ХВИЛЯ 3 (thin-shell + build + view migration) ────────────
  RW-09 thin-shell boundary (bootloader/rAF/DOM-forward)     KEEP-THIN codify
  RW-10 build toolchain + Astro island mount + size budget   dep RW-05
  RW-11 view→wgpu-field migration (per-island, DZ-07..09)    dep RW-04..10
  RW-12 legacy apps/web dedup + kill money-tween             🔴 (FE-17)
```

---

## ХВИЛЯ 0 — CRATE SCAFFOLD + ZERO-RISK DELETES

### RW-01 — `dowiz-engine` Cargo workspace + field-math vendor
**Depends** FE-01/02/03 · **Est** L

**CURRENT (звірено):** `kernel/` = standalone crate (не workspace), cdylib+rlib, wasm-bindgen+serde, modules
analytics/domain/money/order_machine/wasm → 191KB. bebop spectral у окремому репо (cross-repo rule).

**WHY:** [план §1] нативний двигун = workspace крейтів; field-math = strict no_std floor.

**TARGET:** promote `kernel/` у workspace `crates/`; scaffold crates: field-math (vendor bebop2 field.rs/
chebyshev.rs/fft.rs/algebra.rs — `#![no_std]`+alloc zero-deps), field, state, store, render, input, tokens,
i18n, shell. Dep graph: shell→render/field/store/state/input; state→kernel (untouched); field→field-math.
Тільки shell+kernel cdylib. Два wasm artifacts (dowiz_kernel JSON + dowiz_engine numeric).

**GATE:** vendored field-math re-runs bebop2 tests GREEN unchanged (field.rs:346-521); `cargo build wasm32`
green headless; kernel untouched (existing 37 tests green); field-math wasm32-clean no_std.

**ACCEPTANCE:** ☐ workspace ☐ field-math no_std vendor tests green ☐ dep graph ☐ 2 wasm artifacts ☐ kernel
untouched. **OUT OF SCOPE:** не reference bebop cross-repo (vendor); render/field impl (later RW).

### RW-02 — DELETE channel.js → kernel exports
**Depends** — · **Lane** independent · **Est** S · 🟢 zero-risk

**CURRENT (звірено):** channel.js 146 LOC byte-mirror analytics.rs; kernel exports `channel_ledger_js`
(wasm.rs:285)/`reduce_anomalies_js` (:293) вже shipped, glue kernel.js:50-53 wraps; sole consumer
OwnerDashboard.svelte:17 (comment "until WASM-binding wave" — landed).

**WHY:** [план §3.1] чистий дублікат; kernel authority; JS mirror risks drift.

**TARGET:** DELETE channel.js. Rewire OwnerDashboard.svelte:17,45-60: замінити `createLedger()` loop на
`channelLedger(SAMPLE_EVENTS)`; `ledger.funnel(sel)` → `result.funnel[sel] ?? ZERO_FUNNEL` (client 10-stage
zero fallback бо funnel only seen channels). ⚠️ channel_ledger_logic Box::leak per event (harmless one-shot,
НЕ per-frame).

**GATE:** OwnerDashboard renders identical output from kernel export (RED before: from channel.js; GREEN
after: from channel_ledger_js); channel.js file gone; grep 0 imports.

**ACCEPTANCE:** ☐ channel.js deleted (−146) ☐ OwnerDashboard rewired ☐ zero-stage fallback ☐ 0 new Rust.
**OUT OF SCOPE:** не викликати channel_ledger_js per-frame (transactional only).

### RW-03 — DELETE money.ts + utils transition-table
**Depends** — · **Est** S · 🔴 money red-line

**CURRENT (звірено):** money.ts 86 LOC = mirror money.rs (applyTax/fee-ladder/estimateOrderTotal, declared
"SERVER stays source of truth"); packages/ui/src/utils/index.ts ~30 LOC = ORDER_TRANSITIONS+assertTransition
mirror order_machine.rs.

**WHY:** [план §0] kernel money+order-machine = single authority; JS mirrors delete.

**TARGET:** DELETE money.ts + transition-table block у utils/index.ts. Repoint callers (PriceDisplay
formatMoney, checkout estimate) до kernel money via WASM. PORT (not delete) parse/ETA helpers у utils
(`parseALL`/`calcETA`/`normalizePhone`/`generateIdempotencyKey`) → Rust util або keep-thin if browser-bound.

**GATE:** RED — money computed in JS (money.ts) → GREEN kernel money authority; order-transition validated via
kernel not JS mirror; grep money.ts imports = 0.

**ACCEPTANCE:** ☐ money.ts deleted (−86) ☐ transition-table deleted ☐ callers → kernel ☐ parse/ETA ported.
**OUT OF SCOPE:** 🔴 не міняти kernel money-логіку; checkout view (DZ-07).

---

## ХВИЛЯ 1 — PARTICLE PORT (ISLAND #0)

### RW-04 — particle-cloud.js → store::ParticlePool + render::particles
**Depends** FE-04, RW-01 · **Est** L

**CURRENT (звірено):** particle-cloud.js 319 WebGL2; ⚠️ GLSL hardwires blue=1.0 (:52) → delivered рожевий,
dispatch_failed синьо-фіолетовий; dead a_seed attr (:42). Consumer CourierTrack.svelte:15,71-73 one-shot burst.

**WHY:** [план §3.2, FE-04] decorative island #0; wgpu port fixes latent color bug.

**TARGET:** `store::ParticlePool` (SoA ring MAX=4096: pos_x/y+vel_x/y+life+max_life+color[[f32;4]], drop seed,
widen meta→RGBA, `inst` flat staging→ONE writeBuffer, steady alloc=0) + `render::particles` (WGSL: point
sprite→instanced billboard quad, VS size=2+life01·6·energy ndc-flip, FS r>0.25 discard a=(1−4r)·life01
additive SrcAlpha,One; naga runtime, WebGL2 cross-compile) + VOCAB Rust table verbatim + physics integrator
(damp 0.92^(dt/16.6), pointer repulsion, semi-Euler, swap-remove, energy decay, burst; RNG xorshift32; SIMD
f32x4+scalar bit-identical) + shell exports (on_event(kind:u32,count), on_pointer(px,py), instance_view). Swap
CourierTrack backend; try/catch:77-81 = WebGL2 degrade. DELETE particle-cloud.js after parity.

**GATE:** RED — WebGL2 original renders delivered pink/blood blue-purple (hardwired blue); GREEN — wgpu port
full-RGBA correct gold/red; visual parity else; ring zero-alloc steady; compute==CPU bit-identical.

**ACCEPTANCE:** ☐ ParticlePool SoA ring ☐ WGSL billboard ☐ full-RGBA color-fix ☐ ONE writeBuffer ☐ VOCAB
verbatim ☐ physics port ☐ particle-cloud.js deleted (−319). **OUT OF SCOPE:** не міняти VOCAB values.

### RW-05 — shell crate + <10 numeric exports zero-JSON
**Depends** FE-01, RW-01 · **Est** M

**CURRENT (звірено):** kernel wasm.rs = JSON boundary (JSON.parse per-frame катастрофа); rust-core = raw
C-ABI *mut f64 shared ArrayBuffer pattern.

**WHY:** [план §2] per-frame межа = numeric memory-views zero-JSON.

**TARGET:** `shell` crate (ЄДИНИЙ wasm-bindgen у engine): exports memory, engine_new(), tick(frame_ms)->u32
(dirty-bits FE-14), instance_ptr/len + widget_ptr/len (Float32Array view zero-copy), on_pointer(px,py),
on_event(kind:u32,count:u32), set_flags(bits), resize(w,h,dpr). <10, zero-JSON. JS reads NO copy NO parse
(Float32Array(memory.buffer,ptr,len)→writeBuffer). Holds linear-memory staging buffers.

**GATE:** RED — per-frame JSON.parse (kernel pattern); GREEN — zero JSON.parse у frame-loop (profile), one
writeBuffer from view; <10 exports.

**ACCEPTANCE:** ☐ shell cdylib <10 exports ☐ zero-JSON per-frame ☐ Float32Array view ☐ dirty-bits tick.
**OUT OF SCOPE:** kernel JSON wasm.rs stays (transactional).

---

## ХВИЛЯ 2 — PORT PURE-LOGIC → KERNEL CRATES

### RW-06 — geo-anim → kernel/src/geo.rs (+ delivery-zone)
**Depends** — · **Est** M · PORT pure

**CURRENT (звірено):** geo-anim.ts 134 = PURE (haversineMeters/lerpLatLng/bearingDeg/emaNext/
progressAlongRoute equirect/etaSeconds/shouldSnap/isArriving), zero DOM, unit-test seam; delivery-zone.ts 59
= pure ray-cast point-in-polygon. Consumers: use-courier-marker, use-delivery-eta (rAF glue над цим).

**WHY:** [план §0] pure domain math → Rust; ideal port (no DOM).

**TARGET:** new `kernel/src/geo.rs`: port geo-anim functions 1:1 + delivery-zone ray-cast. Export via WASM
для marker kinematics (courier field flow) + zone check. rAF-glue (use-courier-marker/use-delivery-eta) stays
thin-shell over ported math. Reuse for DZ-06 env-signal GPS→field flow.

**GATE:** RED→GREEN — geo functions Rust == TS (parity on fixture: haversine/lerp/bearing/ETA to tolerance);
ray-cast point-in-polygon parity.

**ACCEPTANCE:** ☐ geo.rs pure port ☐ delivery-zone ray-cast ☐ WASM export ☐ parity vs TS ☐ rAF-glue thin.
**OUT OF SCOPE:** marker rAF render (thin-shell).

### RW-07 — Cart consolidate (2 impls) → kernel/src/cart.rs
**Depends** — · **Est** M · PORT+DELETE-dup

**CURRENT (звірено):** ДВІ cart impls: apps/web CartProvider.tsx 122 + packages/ui use-cart.ts 141 (add/
update/total/persist/dedupe); cartReconcile.ts 63 pure (reprice/drop drifted). Overlaps kernel pricing.

**WHY:** [план §0] cart domain (mutation+totals+reconcile) → kernel; one authority.

**TARGET:** new `kernel/src/cart.rs`: cart state machine (add/update(0=remove)/clear, dedupe product+options,
total/itemCount via kernel money, reconcileToMenu re-price/drop drifted). DELETE one of two impls; both React
wrappers become thin-shell over kernel cart (localStorage persist + cross-tab = thin). Storefront cartLines/
subtotal → read kernel.

**GATE:** RED — 2 cart impls (drift risk); GREEN — one kernel cart, both wrappers thin over it; reconcile
re-prices drifted; total via kernel money (integer).

**ACCEPTANCE:** ☐ cart.rs single authority ☐ 2 impls → 1 ☐ reconcile ported ☐ total kernel-money ☐ wrappers
thin. **OUT OF SCOPE:** localStorage/cross-tab (thin-shell); checkout (DZ-07).

### RW-08 — messenger + formatMoney → Rust utils
**Depends** — · **Est** S · PORT pure

**CURRENT (звірено):** messenger.ts 38 pure (deep-link TG/WA/Viber regex normalize); formatMoney/formatALL
(packages/shared-types) money formatting.

**WHY:** [план §0] pure string/format logic → Rust (formatMoney belongs with money.rs).

**TARGET:** port messenger deep-link builder → Rust util (pure string); formatMoney/formatALL → `money.rs`
(money formatting = money authority). Callers (PriceDisplay, message links) via WASM.

**GATE:** RED→GREEN — deep-link output Rust==TS (parity); formatMoney Rust==TS (integer-cent → display string).

**ACCEPTANCE:** ☐ messenger ported ☐ formatMoney→money.rs ☐ parity. **OUT OF SCOPE:** view (thin-shell).

---

## ХВИЛЯ 3 — THIN-SHELL + BUILD + VIEW MIGRATION

### RW-09 — Thin-shell boundary (codify irreducible JS)
**Depends** — · **Est** M · KEEP-THIN codify

**CURRENT (звірено):** 15 Web API категорій ~9k LOC scattered (Push/Geolocation/WebSpeech/Vibration/Audio/
NetworkInfo/matchMedia/WebSocket/fetch/storage/History/File/Clipboard/WebGL/MapLibre).

**WHY:** [план §4] чесний irreducible мінімум; Rust invokes through shims.

**TARGET:** consolidate thin-shell into a clear boundary module: wasm bootloader+feature-detect, lazy rAF arm/
cancel (FE-14), DOM event→on_pointer forwarders, browser-API shims (Push/getUserMedia/WebSpeech/WebXR/clipboard/
file/History), service worker (sw.js stays). Document each as "cannot be Rust". Dedup: apps/web hooks.ts vs
packages/ui hooks (DELETE-dup); safeStorage ×2 (DELETE-dup).

**GATE:** thin-shell surface enumerated + each justified cannot-Rust; dedup removed (RED: 2 impls; GREEN: 1);
Rust invokes browser APIs through shims.

**ACCEPTANCE:** ☐ thin-shell boundary module ☐ each shim justified ☐ dup hooks/storage removed. **OUT OF
SCOPE:** a11y-mirror/input-overlay (FE-15).

### RW-10 — Build toolchain + Astro island + size budget
**Depends** RW-05 · **Est** M

**CURRENT (звірено):** kernel builds cargo→wasm-bindgen --target web→ES-module+.wasm+~1KB loader (kernel.js);
astro.config integrations:[svelte()].

**WHY:** [план §5] no JS bundler for engine logic; size budget.

**TARGET:** engine repeats kernel toolchain → `web/src/lib/engine/`; WGSL include_str! naga runtime; size ≤2MB
gzip (opt-level="z"+lto+panic=abort+wasm-opt -Oz+strip+talc+feature-gate cosmic-text sq/en/uk). LEAN first
slice FE-04 on web-sys (small island), wgpu at Wave 1. Astro island client:only onMount import loader→canvas→
engine_new+arm rAF; astro.config +Vite wasm stanza.

**GATE:** engine wasm ≤2MB gzip (RED: over budget); Vite bundles only loader not logic; Astro island mounts +
degrades; SSR menu stays DOM.

**ACCEPTANCE:** ☐ wasm-bindgen toolchain ☐ ≤2MB gzip ☐ WGSL runtime ☐ island mount+degrade ☐ SSR DOM stays.
**OUT OF SCOPE:** trunk (Astro not Rust-only).

### RW-11 — View → wgpu-field migration (per-island)
**Depends** RW-04..10 + DZ-07..09 · **Est** XL

**CURRENT (звірено):** ~18k LOC React/Svelte view rendering (client/courier/owner). Canonical web/ Svelte
islands = beachhead.

**WHY:** [план §6] view layer = what engine becomes; island-by-island Gain−Loss>0.

**TARGET:** replace view rendering with wgpu-field per-island (per DZ-07/08/09 Sea&Sheet): Svelte island →
Rust field island behind SAME Astro mount, lazy client:only, degrade to Svelte/WebGL2. Order: decorative
first (particle done) → cards/lists → dashboards → forms hybrid last. Each feature from master-checklist
preserved. Money via kernel `<Money>` (RW-03).

**GATE:** each migrated island: full feature-checklist preserved (RED: missing); degrade path works; money
snap not tween; SSR menu DOM. Per-island RED→GREEN.

**ACCEPTANCE:** ☐ per-island view→field ☐ feature-checklist preserved ☐ degrade ☐ money snap ☐ SSR DOM. **OUT
OF SCOPE:** forms/a11y stay hybrid DOM permanently (FE-15).

### RW-12 — Legacy apps/web dedup + kill money-tween
**Depends** — · **Est** L · 🔴 (FE-17)

**CURRENT (звірено):** legacy apps/web React SPA 17.5k LOC; 4 money-tween sites (ClientLayout:154/EarningsPage:
47-176/DashboardPage:421/AnalyticsPage:262 via AnimatedNumber/CountUpPrice); duplicate hooks/storage.

**WHY:** [план §6, FE-17] legacy last; money-tween red-line.

**TARGET:** kill 4 money-tween (AnimatedNumber/CountUpPrice → `<Money>` snap); dedup hooks.ts vs packages/ui,
safeStorage ×2; delete devBootstrap/mockData from prod. React SPA replaced by engine islands last (superseded
by canonical web/).

**GATE:** RED — money count-up tween; GREEN — integer snap (grep money-bound AnimatedNumber = 0); dedup
removed; mock/dev deleted from prod.

**ACCEPTANCE:** ☐ 4 money-tween killed ☐ dedup ☐ mock/dev removed. **OUT OF SCOPE:** 🔴 не чіпати money-обчислення.

---

## ДОДАТОК A — RW → ІНВЕНТАР → ПЛАН

| RW | Що | Вердикт-джерело | План § |
|----|-----|----------------|--------|
| 01 | workspace + field-math | scaffold | §1 |
| 02 | DELETE channel.js | DELETE dup | §3.1 |
| 03 | DELETE money.ts + utils | DELETE dup 🔴 | §0 |
| 04 | particle→wgpu | PORT + color-fix | §3.2 |
| 05 | shell numeric exports | zero-JSON | §2 |
| 06 | geo→kernel | PORT pure | §0 |
| 07 | cart consolidate | PORT+DELETE-dup | §0 |
| 08 | messenger+formatMoney | PORT pure | §0 |
| 09 | thin-shell codify | KEEP-THIN | §4 |
| 10 | toolchain+island+size | build | §5 |
| 11 | view→wgpu-field | view migration | §6 |
| 12 | legacy dedup+money-tween | 🔴 FE-17 | §6 |

## ДОДАТОК B — ІНВАРІАНТИ

1. Kernel authority; JS-дублікати delete (channel.js/money.ts/utils/2nd-cart), ніколи re-port.
2. Дві межі за частотою: transactional→JSON (kernel unchanged), per-frame→numeric memory-views (zero-JSON).
3. Money kernel-integer authority; per-frame presented never interpolated; 4 legacy tween killed.
4. field-math no_std floor zero-deps wasm32-clean; vendor bebop (cross-repo rule); tests green.
5. Keep-running island-by-island; lazy client:only; degrade WASM/WebGPU absent; big-bang forbidden.
6. Thin-shell = irreducible browser-API membrane (15 Web APIs); forms/a11y/SSR-menu DOM permanent.
7. particle wgpu port fixes hardwired-blue bug (visual output legitimately changes — full RGBA).
8. No JS bundler for engine LOGIC (Vite bundles only ~1KB loader; logic = prebuilt .wasm).
9. size ≤2MB gzip engine; opt-z+lto+wasm-opt+strip+talc.
10. Домен що думає → Rust; JS стискається до мембрани + data (locale 3830).

---
*Кінець блюпринтів. 12 RW-01..12, 4 хвилі, стоять на engine FE-01..17 + design DZ-01..12. Джерело:
RUST-ENGINE-REWRITE-PLAN.md + per-file інвентар + crate-архітектура. Критерій — buildable + kernel-authority +
zero-risk deletes first + чесна thin-shell межа. Автор синтезує; виконують агенти.*