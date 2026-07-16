# BLUEPRINT P02 — GPU Engine Foundation (Living-Interface Phase 2)

> **Scope:** the execution-ready blueprint for **Phase 2** of `LIVING-INTERFACE-ROADMAP.md` (§4 row 2, §8
> fast-path). Phase 2 originates **no new design** — it is pure integration: it sequences **six already-designed
> items (RW-01, FE-01, FE-02, FE-03, RW-05, RW-10)** into one working, tested GPU rendering foundation that
> every later GPU-consuming phase (3 render-primitives, 4 field-dynamics, 5 spectral, 6 Sea&Sheet, and the
> order-critical **9a**) builds on. **Planning only — this document edits no code, CI, or config.**
>
> **Depends on Phase 0** (`BLUEPRINT-P00-dev-ci-deploy-enablement.md`): `wgpu` must be *linkable* (`cargo add
> wgpu` behind the `gpu` feature) and the Lavapipe smoke-CI must exist before Phase 2's work is verifiable.
> Phase 0 makes `wgpu` *compile*; **Phase 2 wires it to a live device** — the two are a clean hand-off (P00 §2
> step 3 explicitly leaves the real sink to "FE-01/U1 = Phase 2").
>
> **Sources (all read in full, current-state re-verified live against HEAD 2026-07-16):**
> `LIVING-INTERFACE-ROADMAP.md` §1/§3/§4/§5/§8; `rust-engine-rewrite/BLUEPRINTS-RUST-ENGINE-REWRITE.md`
> (RW-01/05/09/10); `field-ui-engine/BLUEPRINTS-FIELD-UI.md` (FE-01/02/03, boundary FE-16);
> `BLUEPRINT-P00-dev-ci-deploy-enablement.md`.

---

## 1. Current-state evidence (verified live, 2026-07-16)

The Phase-2 RED baseline is subtle: **FE-01/02/03 are already *built* — as headless CPU Rust — but nothing
reaches a real GPU.** Six facts, each re-checked against the working tree:

**1.1 — The `engine/` crate exists and FE-01/02/03 are implemented CPU-side.** `engine/src/lib.rs:15-25`
declares `bridge` (FE-01), `widget_store` (FE-02, "SoA DOD store + ParticlePool ring"), `loop_` (FE-03,
"Fixed-timestep accumulator loop"), plus `field_frame`, `scene`, `sdf`, `motion`, `money_guard`, `zerocopy`.
~51 `#[test]` across the crate (16 in `bridge.rs` alone). `engine/Cargo.toml` has a single dependency
(`dowiz-kernel = { path = "../kernel" }`), `default = []`, and `gpu = []` (**empty stub feature**, comment
"verified 2026-07-16"). So the compute foundation is real and offline-clean; the missing piece is the display sink.

**1.2 — The real gap = no path reaches a real GPU (framing correction).** The task cites
`physics-ui-capture-blueprint.md`'s claim that `bridge.rs::upload_once()` "only COUNTS a hypothetical call,
never reaches a real GPU buffer." **Half of that is now stale, half is exactly right — re-verify precisely:**
- `VertexBridge::upload_once()` (`bridge.rs:157`) no longer *only counts* — **W20 upgraded it to a real CPU
  staging copy** (`self.staging.extend_from_slice(view)`, `:164-165`) plus `write_buffer_calls += 1`. A sibling
  `upload_to<S: GpuUploadSink>(sink)` (`:174`) drives a **real** sink in exactly one `write_buffer` call.
- But the **only** sink that exists is `HeadlessSink` / `HeadlessGpu` (`:251`) — an owned CPU mirror `Vec<f32>`.
  The `#[cfg(feature = "gpu")] pub mod gpu` (`:209`) is an honest boundary: `new_gpu(...)` (`:220`) returns
  `Err("gpu adapter not built — wgpu uncached")`, and `upload_to_gpu(...)` (`:233`) calls `upload_once()` then
  returns the same `Err`. `wgpu` is **not** a dependency.
- **Accurate Phase-2 framing:** the zero-copy CPU→staging path is real and falsifiable; what is missing is a
  concrete `GpuUploadSink` backed by a real `wgpu::Queue`, and a real `wgpu::Device`/`Surface`/`Buffer` behind
  it. **Phase 2 implements that sink and flips `new_gpu` from `Err` to a real device.** The `GpuUploadSink`
  trait seam (`:174`) already exists precisely so this is an additive change, not a rewrite.

**1.3 — `field-math` is NOT vendored (RW-01 RED).** No `field-math/` directory and no `field.rs` anywhere in
the tracked tree. RW-01's "vendor bebop2 `field.rs`/`chebyshev.rs`/`fft.rs`/`algebra.rs`" is entirely undone.

**1.4 — There is no Cargo workspace (RW-01 RED).** No root `Cargo.toml`, no `[workspace]`. `engine/`,
`kernel/`, `wasm/`, `agent-governance-wasm/`, and four `tools/*` are **standalone crates** wired by path deps.
RW-01's "promote `kernel/` into a workspace `crates/`" has not happened.

**1.5 — Kernel test count is 402, not 37.** RW-01's GATE says "existing 37 tests green"; live `kernel/src`
carries **402 `#[test]`**. Use 402 as the "stays GREEN throughout" baseline; the blueprint's 37 is stale.

**1.6 — A proto-shell `wasm/` crate exists but is NOT the RW-05 zero-copy shell.** `dowiz-wasm`
(`wasm/src/lib.rs`) already binds engine+kernel to JS via `wasm-bindgen` (pinned `=0.2.95`, the only offline
CLI). But its exports **return owned `Vec`s** — `compose_field → Vec<u8>` (`:57`), `vertex_field → Vec<f32>`
(`:112`) — which wasm-bindgen **copies out**, and it **mixes JSON** (`knowledge_map`/`lookup_tag`/`related_docs`
→ `String`, `:153-168`). That is a copy-and-JSON boundary, the opposite of RW-05's "<10 numeric exports, zero
JSON, `Float32Array` view over `memory`." RW-05 is a real refactor, not done.

**1.7 — `web/` is Astro/Svelte with a kernel loader but no engine island.** `web/serve.mjs` (dev static
server, sends no CSP — matches P00 §1.2), `web/src/lib/kernel/kernel_client.mjs` (existing kernel wasm loader),
`web/dist/_astro/dowiz_kernel_bg.*.wasm` (built kernel artifact). **No `web/src/lib/engine/`** (RW-10's target)
exists. `particle-cloud.js` is not in the tracked tree (`apps/web` is quarantined/untracked — roadmap §7 C8).

**1.8 — The RW-09/RW-01 third-`audio`-artifact amendment was NOT applied to the blueprint file.** Re-verified:
`BLUEPRINTS-RUST-ENGINE-REWRITE.md` RW-01 TARGET still reads *"Два wasm artifacts (dowiz_kernel JSON +
dowiz_engine numeric)"* (two, not three), its crate-scaffold list has **no `audio` crate**, and RW-09 (`:215`)
still lists "Audio" as a single shim. Roadmap §3 records the amendment but it is a **pending doc-edit**, not
applied. **Phase 2 must treat it as a live dependency edge, not as done** (§2.3 below).

---

## 2. RW-01 — workspace setup + `field-math` vendoring sequence

**Dependency order for the whole phase** (each waits only on strictly earlier items):

| Step | Item | What it delivers | Waits on |
|---|---|---|---|
| A | **RW-01** | Cargo workspace + vendored `field-math` + `audio` placeholder | Phase 0 |
| B | **FE-02 / FE-03** slot-in | SoA store + fixed-timestep confirmed compiling/green under the workspace | A |
| C | **FE-01** real-device wiring (native) | `WgpuSink` + real `wgpu::Device/Queue/Surface/Buffer`, `new_gpu` no longer `Err`; Lavapipe-provable | A, Phase 0 |
| D | **RW-05** | single `wasm-bindgen` shell: <10 zero-JSON numeric exports incl. `on_event` | A, B, C |
| E | **RW-10** | build toolchain + Astro island (browser WebGPU device) + ≤2MB budget | D |

**2.1 — Create the workspace (least-disruptive).** Add a **root `Cargo.toml` with `[workspace]`,
`resolver = "2"`, `members = ["kernel","engine","wasm","field-math","audio", …]`** listing the crates **in
place** — do **not** physically relocate `kernel/` into `crates/`. RW-01's "promote into `crates/`" is a
directory aesthetic; moving the 402-test kernel would churn every path dep (`engine`, `wasm`,
`agent-governance-wasm` all `path = "../kernel"`) for zero functional gain and risks the "stays GREEN"
invariant. A root `[workspace]` over the existing paths satisfies RW-01's intent (one lockfile, one
`cargo build`/`cargo test`, shared target dir) at minimal blast radius. Keep `engine/Cargo.toml`'s
`default = []` / `gpu = ["dep:wgpu"]` (from P00) intact so the default workspace build stays offline-clean.

**2.2 — Vendor `field-math` (the RW-01 keystone, done-test #1).** Create a new `field-math/` crate,
`#![no_std] + alloc`, **zero external deps**, `wasm32`-clean. **Copy** (cross-repo vendor rule — copy, never
cross-reference) bebop2's `field.rs` / `chebyshev.rs` / `fft.rs` / `algebra.rs` **together with their test
modules verbatim**. The GATE is that the vendored `field.rs` re-runs its bebop2 suite (RW-01 cites
`field.rs:346-521`, the `LaplacianSpectrum`/eigensolver tests) **GREEN unchanged** — this is exactly Phase-2
done-test #1. This crate is also the substrate Phase 5 (FE-12) needs for its eigenvector→coords helper, so
vendoring it now is load-bearing beyond Phase 2. Confirm `cargo build --target wasm32-unknown-unknown -p
field-math` is green headless.

**2.3 — The `audio`-crate amendment (dependency, not assumption — task point).** Roadmap §4's Phase-2 row says
RW-01 is *"now amended to scaffold the `audio` crate,"* but §1.8 confirms the amendment is **not** in the RW
blueprint file. Ruling, cheapest-sufficient:
1. **Apply the §3 doc-amendment first** (it is named a precondition): edit `BLUEPRINTS-RUST-ENGINE-REWRITE.md`
   RW-01 TARGET "two → three wasm artifacts" + add `audio` to the scaffold list, and split RW-09's "Audio"
   shim. This is a doc edit with no code — do it before the workspace lands so the member list is correct.
2. **Scaffold an *empty* `audio/` crate** in the workspace `members` (a `lib.rs` stub, `#![no_std]+alloc`, no
   DSP, **no `wasm-bindgen`**). This costs nothing, gives the workspace its final shape, and avoids a later
   member-list churn.
3. **Do NOT build the audio DSP or the third wasm-bindgen artifact here.** The `AudioWorkletGlobalScope`
   third-artifact work is **Phase 7**, which is **off the G11 critical path** (§8). Phase 2 pre-scaffolds the
   directory; Phase 7 fills it. This keeps the amendment honored without dragging off-path work onto the fast path.

**2.4 — FE-02/FE-03 slot-in (step B).** These modules already exist and pass. The Phase-2 work is confirmation,
not construction: verify `widget_store` (`WidgetStore` hot/warm/cold SoA + `ParticlePool` ring, zero-alloc
steady state) and `loop_` (`DT = DT_STABLE`, `MAX_FRAME`/`MAX_SUBSTEPS` guards, `alpha = accum/DT`
interpolation) compile and stay green under the new workspace, and that their output arrays feed
`VertexBridge`'s staging buffer (§4).

---

## 3. FE-01 — the real-device wiring (adapter · device · queue · surface · buffer)

**What FE-01 already specifies (do not re-blueprint) vs. what is genuinely new integration glue.**

FE-01's TARGET STATE + ALGORITHM block already fix the **data path** and the **`GpuUploadSink` seam**: Rust
writes a flat `Vec<f32>` into linear memory → the consumer reads a `Float32Array` **view** (no copy) → **one**
`writeBuffer`. Crucially, FE-01's algorithm block **already splits two contexts**:
```
JS (browser):  const view = new Float32Array(wasm.memory.buffer, ptr, len);  // NO COPY
               queue.writeBuffer(gpuBuffer, 0, view);                          // 1 upload
Native wgpu:   bytemuck::cast_slice(linear_memory[ptr..]) → write_buffer       // TRUE zero-copy CPU-side
```
So FE-01 specifies **what crosses** and the **single upload**. It does **not** specify adapter/device/surface
*acquisition* — that is the new glue Phase 2 owns.

**3.1 — The dual-context model (the central architectural clarification).** There are **two owners of "the
GPU," one shared seam** — and resolving this is what makes done-test #2 (0 JSON, one `writeBuffer` from a view)
*and* done-test #3 (≤2MB) both achievable:

- **Native context (CI-headless + operator's local visual loop): Rust `wgpu`, behind `feature = "gpu"`.**
  `wgpu::Instance::new` → `request_adapter` (`power_preference: LowPower`, `force_fallback_adapter: true` so
  Lavapipe binds — exactly P00 §3's `WGPU_ADAPTER_NAME=llvmpipe`) → `request_device` (core WebGPU feature set,
  no exotic limits — J5) yielding `Device` + `Queue`. **Surface:** an offscreen `wgpu::Texture`
  (`RENDER_ATTACHMENT | COPY_SRC`) for CI (no window), or a real windowed `Surface` locally. **Buffer:**
  `device.create_buffer` (`VERTEX | COPY_DST`, `size = MAX_PARTICLES * stride * 4`). **The new sink:**
  `struct WgpuSink { queue, buf }; impl GpuUploadSink for WgpuSink { fn write_buffer(&mut self, off, data) {
  self.queue.write_buffer(&self.buf, (off*4) as u64, bytemuck::cast_slice(data)); } }`. **Flip `new_gpu`**
  (`bridge.rs:220`) from the `Err` stub to build `WgpuSink` and return a wired `VertexBridge`. This is precisely
  the slot P00's `engine/tests/gpu_smoke.rs` reserves (adapter request → offscreen render → `copyTextureToBuffer`
  → `mapAsync` readback → SSIM-vs-golden). **`bytemuck` is the one new native dep** (small, offline-cacheable
  alongside `wgpu`); gate it behind `feature = "gpu"` next to `wgpu` so the default build stays zero-dep.

- **Browser context (production): JS-owned WebGPU, zero Rust `wgpu` in the browser wasm.**
  `navigator.gpu.requestAdapter()` → `requestDevice()`; `canvas.getContext("webgpu")` configured with the
  preferred format, `alphaMode`, and `device`; a `GPUBuffer` (`VERTEX | COPY_DST`). The RW-05 shell exposes the
  numeric view; the Astro-island glue does `device.queue.writeBuffer(buf, 0, view)` — **that JS call is the one
  `writeBuffer` of done-test #2.** Keeping WebGPU in JS (not `wgpu`-in-wasm) is faithful to FE-01's algorithm
  block, upholds the determinism firewall (FE contract #6: authoritative physics CPU-side, GPU = dumb display,
  so nothing forces `wgpu` into the browser), and is what keeps the browser bundle under 2 MB (done-test #3) —
  a `wgpu`-wasm engine would risk the budget and duplicate the device JS already provides. **The unifying
  abstraction is `GpuUploadSink`: the native side is a Rust impl over `wgpu::Queue`; the browser side is the
  identical one-`writeBuffer` contract implemented in JS over the zero-copy view.**

  *(Recorded fork, criterion-decided: if a later phase genuinely needs positions to stay resident on the GPU via
  a WGSL compute integrator — FE-01's "опційно" fast path, FE-04+ — a `wgpu`-in-wasm variant may be revisited.
  For Phase 2 it is out of scope; the ≤2MB gate (done-test #3) is the falsifiable arbiter if it is ever
  reconsidered.)*

**3.2 — Minimal Phase-2 render pipeline (prove the pipe, not the art).** Phase 2 proves
`device → buffer → draw → present/readback` end-to-end, nothing more: `create_shader_module` over a **trivial
embedded WGSL** (a clear + fullscreen-triangle, or a single instanced unit-quad) → `create_render_pipeline` →
a render pass that binds the vertex buffer and issues one draw → **present** (browser `Surface`) or
**`copyTextureToBuffer` + `map_async` readback** (CI offscreen). The **real particle billboard shader +
full-RGBA colour fix** is FE-04 = **Phase 3** — Phase 2 must not scope it. Surface config: format from
`surface.get_capabilities(&adapter).formats[0]`; size = canvas client size × dpr (driven by the shell's
`resize(w,h,dpr)` export); `present_mode: Fifo`; reconfigure on resize.

---

## 4. FE-02 / FE-03 integration — SoA store + fixed-timestep driving the device

The connection is a single, already-half-built pipe; Phase 2 attaches its far end to a real sink:

```
FE-03 loop (fixed DT=DT_STABLE, accumulator, MAX_FRAME/MAX_SUBSTEPS)
   └─ integrates FE-02 SoA arrays (WidgetStore/ParticlePool) CPU-side          [authoritative]
        └─ render(lerp(prev, curr, alpha)) writes flat f32 → VertexBridge staging  [engine/src/bridge.rs]
             └─ ONE upload_to(sink) per RENDERED frame → GPU buffer                 [WgpuSink | JS writeBuffer]
                  └─ render pass draws the vertex buffer → present / readback        [§3.2]
```

**Concretely:** the frame path currently terminates in `HeadlessSink`/`HeadlessGpu` (the offline test double).
Phase-2 work = at the render boundary, call `upload_to(&mut wgpu_sink)` (native) or hand the `vertex_view()`
slice to the JS `writeBuffer` (browser) — **without** changing the physics. `HeadlessSink` **stays** as the
default-build test double so the 51 engine tests and the "`json_parse_calls == 0` / `write_buffer_calls == 1`
per frame" gate remain green offline. FE-02/FE-03 never move onto the GPU in Phase 2 — the WGSL compute
integrator is FE-01's optional fast path (deferred).

**FE-16 / "degrade gracefully" boundary (task point — check and note).** Phase-2 done-test #3 says the island
"degrades gracefully (WebGL2/no-GPU fallback per FE-16)." **FE-16 is NOT Phase 2** — the roadmap slots FE-16
into **Phase 4** (row 4: FE-07/08/09/14/**16**), and the FE blueprint places it in Хвиля 4. The **full WebGL2
compute-parity fallback + simd128/scalar bit-identical path is Phase 4.** Phase 2's obligation is the
**narrower mount-time degrade** already mandated by RW-01 contract #3 ("degrade коли WASM/WebGPU absent") and
RW-10 ("island mounts + degrades; SSR menu stays DOM"): if `requestAdapter()` yields no WebGPU device, the
island **must not crash the page** — it falls back to the existing DOM/Svelte render or a static state. That is
the Phase-2 line; WebGL2 render parity is explicitly deferred to Phase 4. State this boundary in the island
glue so Phase 4 has a defined seam to fill.

---

## 5. RW-05 shell + RW-10 build/mount sequence

**5.1 — RW-05: the single `wasm-bindgen` entry point (step D).** Turn the numeric surface of the existing
`dowiz-wasm` crate (§1.6) into the RW-05 shell — the **one** `wasm-bindgen` crate in the engine, **<10 exports,
zero JSON in the frame loop**:
`memory`, `engine_new()`, `tick(frame_ms: f32) -> u32` (dirty-bits for FE-14), `instance_ptr() / instance_len()`
and `widget_ptr() / widget_len()` (so JS builds `new Float32Array(memory.buffer, ptr, len)` — a **view, no
copy**), `on_pointer(px, py)`, `on_event(kind: u32, count: u32)`, `set_flags(bits: u32)`, `resize(w, h, dpr)`.
The current `vertex_field`/`compose_field` return owned `Vec`s (wasm-bindgen copies them out) — **replace them
with the `ptr`/`len` export pair**; this is precisely what makes done-test #2 *real* rather than "built but
uncalled" (a `Vec`-returning export can never be a zero-copy view). The existing JSON functions
(`knowledge_map`/`lookup_tag`/`related_docs`) are **transactional, not per-frame** — keep them off the frame
loop (they belong on the kernel/JSON plane, RW-05 "OUT OF SCOPE: kernel JSON `wasm.rs` stays"). The shell must
build under the pinned `wasm-bindgen = "=0.2.95"` CLI.

**5.2 — RW-10: build toolchain + Astro island + ≤2MB budget (step E).** Repeat the kernel toolchain
(`cargo → wasm-bindgen --target web → ES module + .wasm + ~1KB loader`) into **`web/src/lib/engine/`**. Ship the
WGSL via `include_str!` — note that in the JS-owns-WebGPU design (§3.1) the shader **text** is handed to the JS
glue for `createShaderModule`, not compiled by naga-in-wasm, which keeps the shell lean. **Size budget ≤2MB
gzip** via `opt-level = "z"` + `lto` + `panic = "abort"` + `wasm-opt -Oz` + `strip` + `talc` allocator +
feature-gated locales. **Astro island** (`client:only`, behind the same mount as the kernel islands): `onMount`
→ import loader → create canvas → `engine_new()` → **create the browser WebGPU device + context + `GPUBuffer`
(§3.1 browser context)** → arm rAF (`tick()` → read `Float32Array` view → `queue.writeBuffer` → draw). If no
WebGPU: degrade (§4). `astro.config` gains a Vite wasm stanza; **SSR menu stays DOM**. **This island is the one
place the browser `GPUDevice` is created and fed the shell's zero-copy view** — it is where FE-01 (browser
context), RW-05 (shell exports), and RW-10 (island) meet.

---

## 6. Acceptance criteria — numbered checklist (matches all 3 falsifiable done-tests)

**DT1 — Vendored `field-math` re-runs bebop2 tests GREEN unchanged.**
- **AC-1a** `field-math/` crate exists: `#![no_std] + alloc`, zero external deps, in the workspace `members`.
- **AC-1b** `cargo test -p field-math` GREEN with the bebop2 test modules **copied verbatim** (RW-01
  `field.rs:346-521` eigensolver suite passes unchanged).
- **AC-1c** `cargo build --target wasm32-unknown-unknown -p field-math` green (wasm32-clean).

**DT2 — Frame-loop profile: 0 `JSON.parse`, one `writeBuffer` from a `Float32Array` view (the zero-copy bridge
is REAL, not built-but-uncalled).**
- **AC-2a** `new_gpu` (`bridge.rs:220`) **no longer returns `Err`** — it builds a real `wgpu::Device`/`Queue`
  and returns a `VertexBridge` wired to a `WgpuSink` (native, `feature = "gpu"`).
- **AC-2b** The `gpu_smoke` CI test (P00 slot) renders one offscreen frame through the real device, reads it
  back, and matches the SSIM golden — proving `device → buffer → draw → readback` end-to-end under Lavapipe.
- **AC-2c** Frame-loop profiler asserts `json_parse_calls == 0` and `write_buffer_calls == 1` per rendered
  frame, on **both** the native (`WgpuSink`) and default (`HeadlessSink`) builds.
- **AC-2d** Browser path: the Astro island reads `new Float32Array(memory.buffer, ptr, len)` (a **view**, from
  RW-05's `instance_ptr/len`) and issues exactly one `device.queue.writeBuffer(...)` per frame — no `Vec`
  copy-out, no `JSON.parse`.

**DT3 — `dowiz-engine` wasm ≤2MB gzip; Astro island mounts + degrades; kernel tests stay GREEN throughout.**
- **AC-3a** The RW-05 shell wasm (built with the RW-10 flags) is **≤2MB gzip**.
- **AC-3b** The Astro island mounts (`client:only`), creates the browser WebGPU device, and renders; with no
  WebGPU it **degrades without crashing the page** (mount-time degrade, §4) and SSR menu stays DOM.
- **AC-3c** `cargo test` across the workspace keeps **all 402 kernel `#[test]` GREEN** and the ~51 engine tests
  GREEN — kernel logic untouched (RW-01 contract #1).

**AC-4 — Scope discipline (guards against over-build).**
- Default (non-`gpu`) workspace build still pulls **zero external crates** (offline-clean preserved).
- **No FE-04 particle billboard shader / full-RGBA colour fix** (Phase 3); Phase-2 WGSL is the trivial
  clear/quad prove-the-pipe shader only.
- **No WGSL compute integrator** (FE-01 optional fast path, deferred) — physics stays CPU-side.
- **No full FE-16 WebGL2 compute-parity fallback** (Phase 4) — only mount-time degrade.
- **No `Cross-Origin-Opener-Policy`/`Cross-Origin-Embedder-Policy`** (Phase 10, Decision B).
- **No audio DSP or third wasm-bindgen artifact** — the `audio` crate is an empty placeholder (§2.3); DSP is
  Phase 7.
- `money_guard.rs` and kernel money logic untouched (FE-09 is Phase 4; red-line).

---

## 7. What this unblocks (every later GPU-consuming phase: 3, 4, 5, 6, 9a)

Phase 2 is the foundation the roadmap's fast path `0 → 1 → 2 → 3 → 4 → 5 → 6 → 9a` stands on — it is squarely
**on the G11 critical path**:

- **Phase 3 (Render Primitives + Brand-on-GPU):** FE-04 (particle → wgpu, blue-hardwire fix, RGBA), FE-05 (SDF
  + design-token `theme_tokens` UBO), FE-06 (MSDF text) all need the **live device + vertex-buffer pipeline +
  the `GpuUploadSink` seam** this phase builds; R-VENDOR P0-3 writes into FE-05's Bind0 UBO on this device.
- **Phase 4 (Field Dynamics + Guards):** FE-07/08/09/14 run on the CPU-side fixed-timestep loop; **FE-16
  completes the WebGL2/scalar fallback whose mount-time seam Phase 2 defines** (§4).
- **Phase 5 (Spectral Embedding):** the FE-12 eigenvector→coords helper is built on the **`field-math`
  eigensolver Phase 2 vendors** (RW-01, §2.2) — the one net-new kernel primitive R-LM/DZ layout needs.
- **Phase 6 (Sea & Sheet Backbone):** DZ-01..06 render on **the engine + RW-05 shell + RW-10 Astro island**
  delivered here; the `on_event(kind,count)` export is the impulse stream DZ-04/05 drive.
- **Phase 9a (Order-Critical Product Surface — G11):** DZ-07 (client menu/checkout/track) and DZ-08 (courier)
  transitively ride 3→4→5→6; **the customer order UI is drawn by the GPU foundation started here.** Without
  Phase 2's live device, shell, and island, none of the downstream customer surface renders at all.

**Off the G11 path (not unblocked-toward here):** Phase 7 (sonification — needs the amended third `audio`
artifact, §2.3) and Phase 8 (living-memory viz) are deferred; Phase 2 only **pre-scaffolds the empty `audio`
crate** so the workspace shape is final. Phase 10 (COOP/COEP + SAB) is untouched.

---

*End BLUEPRINT P02. Planning only — no product code, CI, or config edited. Every current-state citation
re-verified live against HEAD on 2026-07-16 (engine modules, `bridge.rs` gpu stub, absent `field-math`/workspace,
402 kernel tests, `dowiz-wasm` copy-and-JSON surface, unapplied RW-09/RW-01 amendment). The dual-context device
model, workspace/vendor sequence, shell refactor, island mount, and acceptance checklist above are prescriptions
for an implementation pass, not applied changes.*
