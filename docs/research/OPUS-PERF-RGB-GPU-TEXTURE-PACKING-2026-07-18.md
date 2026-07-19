# GPU-compute texture packing (RGBA32F/RG32F) — does it help dowiz's field engine?

**Date:** 2026-07-18
**Author:** Opus (research/audit pass — *recreated 2026-07-19 after the original was
accidentally deleted; same investigation redone from source, not reconstructed from memory*)
**Scope:** GPU-compute texture packing — using `RGBA32F`/`RG32F` **storage/sampled**
textures as the read/write medium of a physics compute kernel — as a *distinct*
question from `engine/src/field_frame.rs::frame_rgba()`'s CPU-side RGBA8 **display**
packing (which the sibling audit `OPUS-PERF-RGB-PACKING-REUSE-2026-07-18.md` already
dismissed). No code changed. All GPU work here is gated behind the P38 §4.2 operator
network-grant decision.

---

## TL;DR

`frame_rgba()` packs bytes because the **display blit** demands interleaved RGBA8 —
that is a boundary-format constraint and does **not** generalize to compute (settled in
the sibling doc). This doc asks the genuinely different question the operator raised:
**can multi-channel float textures (`RGBA32F`/`RG32F`) reduce texture I/O and improve
locality for the *physics compute itself*?**

The answer is a **three-regime** verdict, not a yes/no:

- **(a) Genuinely helps** — for the **2-D grid field** workload once it is (i) GPU-resident
  and (ii) multi-channel: pack `(U, U̇)` in `RG32F`, complex spectral `(re, im)` in `RG32F`,
  a true vector field `(vx, vy, p, ρ)` or spatially-varying PDE coefficients `(Γ, c², M, S)`
  in `RGBA32F`. Real wins: one texel fetch delivers the whole per-cell state (fewer binds,
  better cache locality), **free hardware bilinear** for semi-Lagrangian sub-cell sampling,
  and — critically — on the **WebGL2 budget-device floor it is the *only* GPGPU mechanism
  that exists** (no compute shaders there).
- **(b) Real but not yet applicable** — dowiz has **no GPU compute pipeline at all today**
  (`gpu`/`webgl`/`webgpu` cargo features are EMPTY by design, `wgpu` uncached, W21 BLOCKED),
  and the field is still a **single scalar `U`** (nothing to pack — a 1-channel field on GPU
  is an `R32F` texture, and packing it into RGBA wastes 3 channels). On WebGPU specifically,
  storage **buffers** are often the better default, and the `read_write` format restriction
  forces ping-pong on any packed float texture — so this is the right tool for the *grid-field*
  workload only, deferred until that workload is both GPU-resident and multi-channel.
- **(c) Doesn't apply** — `frame_rgba()` itself (RGBA8 **display** format, a 1→4 colormap of one
  scalar — the premise); the **particle/vertex path** (`ParticleBuffer`, `VertexBridge`) which
  crosses the boundary as an interleaved **vertex/storage buffer**, not a texture, and whose
  compute wants **SoA**; and the **kernel's CPU compute** (`mat.rs`/`csr.rs`/`simd.rs`), which
  never touches a texture.

The unifying rule: **the packed-float-texture is the correct medium for a 2-D grid PDE that
lives on the GPU; SoA storage buffers are correct for particles/graphs; interleaved RGBA8 is a
display constraint.** dowiz has correctly not built any of the GPU legs yet, so (a) is a
*design*, not a *win in hand*.

---

## 0. Why this is NOT the CPU-side question

The sibling doc `OPUS-PERF-RGB-PACKING-REUSE-2026-07-18.md` asked: *does `frame_rgba`'s
interleaved-byte packing teach the kernel anything?* It answered **no** — that interleaving is
imposed by the **display/blit output format**, and where the kernel wants throughput it correctly
chose **SoA** (`simd.rs`, `csr.rs`) or **contiguous-flatten** (`mat.rs`). That conclusion is about
**CPU** data layout and an **8-bit final-output** buffer.

This doc is a different axis entirely:

| | Sibling doc (CPU display packing) | This doc (GPU-compute texture packing) |
|---|---|---|
| Medium | `Vec<u8>`, host memory | `RGBA32F`/`RG32F` GPU texture (device memory) |
| Role | **final output** consumed by a blit | **intermediate compute I/O**, read+written by a kernel each step |
| Bit depth | 8-bit quantized (lossy) | 32-bit float (physics-exact) |
| "Packing" means | interleave 4 derived channels of **1 scalar** for the canvas | co-locate **N independent physics scalars** per grid cell |
| Question | reusable CPU perf trick? (no) | reduce texture binds / improve locality on GPU? (regime-dependent) |

`frame_rgba` is a **1→4 fan-out** (sign→hue, magnitude→brightness) — one input scalar becomes
four display bytes. GPU-compute packing is an **N→1** co-location — N genuinely-independent
physics quantities share one texel. These are not the same operation, and the sibling doc's "no"
does not answer this doc's question. **Confirmed from source below.**

---

## 1. What `frame_rgba()` actually is (confirmed from source)

`engine/src/field_frame.rs:229-249`. Input: `self.u` — a **single scalar field**, `Vec<f32>` of
length `w*h` (`field_frame.rs:159-168`). Output: `Vec<u8>` of length `w*h*4`.

```rust
// field_frame.rs:229
pub fn frame_rgba(&self) -> Vec<u8> {
    let n = self.w * self.h;
    let mut out = vec![0u8; n * 4];
    for i in 0..n {
        let v = self.u[i];
        let mag = if v.is_finite() { v.abs().min(1.0) } else { 0.0 };
        let b = (mag * 255.0) as u8;                 // float→u8 quantize (LOSSY, 8-bit)
        let (r, g, bl) = if v >= 0.0 {               // sign → warm/cool colormap
            (b, (b as u32 * 128 / 255) as u8, (b as u32 * 40 / 255) as u8)
        } else {
            ((b as u32 * 40 / 255) as u8, (b as u32 * 128 / 255) as u8, b)
        };
        out[i * 4] = r; out[i*4+1] = g; out[i*4+2] = bl; out[i*4+3] = 255; // interleaved RGBA8
    }
    out
}
```

The module's own doc comment states the intent precisely (`field_frame.rs:5-7`): *"Authoritative
compute stays CPU-side … The GPU (future `feature = "gpu"` adapter) would **blit** the `Vec<u8>`
this module produces."* And `compose()` (`field_frame.rs:255-262`) is documented as *"the single
call a future `wgpu` **blit** would consume."*

So `frame_rgba` is unambiguously a **final display frame**: 8-bit, lossy, RGBA-interleaved because
that is what a canvas `ImageData` / a `wgpu` swap-chain texture upload expects. It is a **1→4
colormap of one scalar**, not multiple physics channels. It is a compute **output**, never a
compute **input**. This is exactly why it does not generalize to GPU-compute packing — you would
never feed an 8-bit lossy colormap back into a physics stencil.

**The physics state, by contrast** (`field_frame.rs:198-222`, `step()`): the integrator holds `u`,
`u_prev`, and two scratch buffers, all `Vec<f32>`, and evolves the operator equation
`M·U̇ = -ΓU̇ - c²·L·U + S` on the CPU. **This** is the buffer a GPU-compute texture would carry —
and today it is a **single scalar channel**.

---

## 2. dowiz's actual GPU/compute pipeline today (the honest state)

I searched `engine/src`, `apps/web/src`, and the design docs. The ground truth:

**There is no GPU compute pipeline. There are no shaders.** No `.wgsl` files exist in the repo;
no `createComputePipeline`/`GPUTexture`/`texStorage`/`RGBA32F` usage in `apps/web/src`; the only
`compute` hits in `engine/src` are the CPU `LaplacianField::compute` method and doc prose.

The engine is **deliberately CPU-side and offline-clean** (`engine/Cargo.toml:7-9`): *"NO
dependencies — offline-clean by mandate. GPU is a display surface; authoritative compute is
CPU-side (scalar == SIMD bit-identical). wgpu/cosmic-text are OUT OF SCOPE here."* Every GPU-adjacent
cargo feature is **EMPTY by design** (`Cargo.toml:45-57`):

- `gpu = []` — *"`wgpu` is absent from the cargo cache and every Cargo.lock … the real adapter is
  an honest `Err("gpu adapter not built — wgpu uncached")` stub"* (confirmed at
  `engine/src/bridge.rs:253-261`).
- `webgl = []`, `webgpu = []`, `splat = []` — *"P11 §5 (E23 scaffolding) … EMPTY by design — no deps pulled."*

The current "GPU boundary" is a **CPU mock**: `bridge.rs::HeadlessGpu` / `VertexBridge::upload_once`
perform a real CPU staging copy and count exactly one `writeBuffer`, zero JSON — the FE-01 zero-copy
contract — without any real device (`bridge.rs:277-303`, tests `bridge.rs:311-329`). The blueprints
confirm the ceiling: **W15** (`docs/research/BLUEPRINT-W15-wgpu-shell.md`) ships only a trait shell;
**W21** (`docs/design/BLUEPRINT-W21-field-ui-gpu-blocked.md`) is titled *"BLOCKED OFFLINE (wgpu
uncached)"* and states *"Nothing renders to a GPU anywhere … the only demonstrable render path"* is
the CPU field-frame.

**What state would a GPU compute kernel carry, if the pipeline existed?** Two shapes in the tree:

1. **The 2-D grid field** (`field_frame.rs`): `U` (+ `U_prev` for the backward-difference `U̇`),
   source `S` from the SDF, all `w*h` scalars. This is the natural **texture** workload —
   a stencil PDE over a regular grid. Currently **1 channel**.
2. **Particles** (`engine/src/zerocopy.rs::ParticleBuffer`, `bridge.rs::VertexBridge`):
   `[x, y, vx, vy, life]` per particle (`zerocopy.rs:21-24`), staged as an interleaved **vertex/
   storage buffer** for the `queue.writeBuffer` upload. This is a **buffer** workload, not a texture
   one. Note `bridge.rs` also computes a per-frame field via the kernel's graph Laplacian
   (`apply_field`, `field: Vec<f64>`) — also a single channel.

So: the only workload for which "pack multiple physics channels into a float texture" is even the
right *shape* is the grid field — and it is presently a single scalar with no GPU home.

---

## 3. Established GPU-compute texture-packing precedent (real sources)

Packing simulation channels into float-texture color channels is a **canonical, decades-old GPGPU
technique**, not a novelty. Primary sources:

- **GPU Gems, Ch. 38 — "Fast Fluid Dynamics Simulation on the GPU"** (Mark Harris, NVIDIA). The
  reference text for exactly this: *"Because textures usually have three or four color channels, they
  provide a natural data structure for vector data types with two to four components."* The 2-D
  velocity field is stored with *"the red channel [containing] the magnitude in x, and the green
  channel … the magnitude in y."* State fields (velocity, pressure, dye, vorticity) are kept in
  textures, and the solver uses **ping-pong / render-to-texture**: *"RTT requires the use of two
  textures to implement feedback … The swap … is merely a swap of texture IDs."*
  <https://developer.nvidia.com/gpugems/gpugems/part-vi-beyond-triangles/chapter-38-fast-fluid-dynamics-simulation-gpu>
  → This is the direct precedent for packing `(vx, vy, p, ρ)` into `RGBA32F` and evolving it in place.

- **WebGL2 Fundamentals — "GPGPU"**. Establishes that **WebGL2 has no compute shaders**: GPGPU is done
  by treating *"a texture … [as] a 2D array of values,"* writing via framebuffer attachment (fragment
  shader) and reading via `texelFetch`. It packs particle position `x,y` into an **`RG32F`** texture's
  red/green channels and **ping-pongs two framebuffer-attached textures** each frame, keeping all state
  on the GPU. (Also documents the mobile fallback: `floatBitsToInt`/`uintBitsToFloat` when float
  textures are unavailable.) <https://webgl2fundamentals.org/webgl/lessons/webgl-gpgpu.html>
  → Confirms that on the WebGL2 floor, **float-texture packing IS the GPGPU mechanism**, not an optional optimization.

- **`gpu-io`** (Amanda Ghassaei) — a production WebGL2 GPGPU library for *"running physics simulations
  … in a web browser,"* built on *"2D spatially-distributed state (i.e. fields) stored in textures using
  fragment shaders,"* with **GPULayers of configurable components** (i.e. pack N values per texel) and
  **framebuffer ping-ponging**. It explicitly notes *"all the computation happens in a fragment shader"*
  because WebGL1/2 lack compute — WebGPU support is *"planned"*.
  <https://github.com/amandaghassaei/gpu-io>
  → A living reference for the exact packing/ping-pong pattern dowiz's WebGL2 fallback would use.

- **WebGPU Fundamentals — "Storage Textures"**. On the WebGPU (primary) path, storage-texture formats
  include **`rgba8`, `rgba16float`, `rg32float`, `rgba32float`**. Two facts that shape the design:
  (1) a storage texture *"is still a texture — you can use it … as a storage texture [in one shader] and
  as a regular texture (with samplers, mip-mapping) in another,"* i.e. you get **free hardware
  filtering** on the same packed data; (2) only **`r32float`/`r32sint`/`r32uint` support `read_write`**
  in a single shader — every multi-channel float format is **read-only OR write-only**, which **forces
  ping-pong** for packed state. <https://webgpufundamentals.org/webgpu/lessons/webgpu-storage-textures.html>
  → Both the upside (free bilinear on packed channels) and the binding constraint (packed = ping-pong) are real.

- **Operator-supplied in-repo research** `docs/design/living-interface-2026-07-16/EXTERNAL-RESEARCH-gpu-neural-field-sonification.md`
  (§1, §6): WebGPU is baseline as of Jan 2026, **WebGL2 remains the fallback with no compute stage —
  "emulate via transform feedback / float-texture ping-pong"**; WGSL is f32-only and one packs into
  `rgba16float` or a `u32` via `pack4x8unorm` to halve bandwidth; `RGBA32F` textures are the classic
  Shadertoy particle/"falling-sand" state medium. This is dowiz's own filtered corpus and it says the
  same thing the external references do.

**Established, not speculative.** The technique is real. The question is purely *where in dowiz it lands*.

---

## 4. Three-regime verdict

### (a) Genuinely helps — the 2-D grid field, once GPU-resident and multi-channel

Where GPU-compute texture packing is *correct and valuable* for dowiz is the **grid field PDE**
(`field_frame.rs`), on the day it moves to the GPU and grows past one channel:

- **`(U, U̇)` in `RG32F`.** The integrator's backward-difference velocity term needs both the current
  field and its predecessor at every cell (`field_frame.rs:208-213`: `udot = (u - uprev)/dt`). Packing
  `U` and `U̇` (or `U` and `U_prev`) into one `RG32F` texel means **one texture fetch per cell delivers
  both operands** of the update, instead of binding + sampling two separate textures. Fewer binds,
  one memory transaction per cell, better cache locality — a genuine, measurable win on the stencil hot loop.
- **Complex spectral `(re, im)` in `RG32F`.** If per-cell spectral/phase quantities are ever evaluated
  on the GPU, a complex number is the textbook 2-channel texel.
- **A true vector field `(vx, vy, p, ρ)` in `RGBA32F`.** If the field model becomes a fluid/flow field
  (the neural-field / living-interface direction), this is exactly GPU Gems 38 — packing is the *natural*
  representation, not an optimization bolt-on.
- **Spatially-varying coefficient texels `(Γ, c², M, S)` in `RGBA32F`.** Today `Γ, c², M` are scalar
  constants (`FieldEquilibrium`, `field_frame.rs:39-49`) and `S` is a separate SDF buffer. If media become
  heterogeneous (per-cell damping/wave-speed/mass + source), one `RGBA32F` fetch delivers all four PDE
  coefficients for a cell in a single transaction — clean and cache-friendly.
- **Free hardware bilinear sub-cell sampling.** This is the one thing a *texture* gives that a storage
  *buffer* does not (WebGPU storage-texture note above): semi-Lagrangian advection / sub-cell probing reads
  the packed field at fractional coordinates and the hardware interpolates all channels for free. For a
  field engine that will want smooth sampling for motion/blur, this is a real reason to prefer packed
  textures over buffers for the grid state specifically.
- **The WebGL2 floor makes it non-optional.** On budget devices without WebGPU, there are **no compute
  shaders at all** — float-texture ping-pong is the *only* GPGPU mechanism (WebGL2 Fundamentals; the
  in-repo research). So if dowiz wants *any* GPU physics on the WebGL2 target, packing state into
  `RGBA32F`/`RG32F` textures isn't a speedup to consider — it is the mechanism, full stop.

**Why (a) is real and not just the rejected CPU trick:** the sibling doc rejected *8-bit display
interleaving as a CPU throughput pattern*. This is *32-bit float co-location of independent physics
channels as GPU compute I/O* — a different medium, a different bit depth, a different purpose, backed by
GPU Gems 38 and every browser-GPGPU reference. The rejection there does not touch this.

### (b) Real but not yet applicable — pipeline maturity + WebGPU-vs-WebGL2

Everything in (a) is **correct and inapplicable today**, for four concrete reasons rooted in the actual code:

1. **No GPU compute pipeline exists.** `gpu`/`webgl`/`webgpu` features are EMPTY by design; `wgpu` is
   uncached and absent from every Cargo.lock (`Cargo.toml:45-57`, `bridge.rs:233-261`); W21 is BLOCKED.
   There is nowhere to *put* a packed compute texture. This is gated behind the **P38 §4.2** operator
   network-grant decision (the same gate P86/P87 in the SYNTHESIS roadmap sit behind). Until that flips,
   packing is a **design**, not an implementation.
2. **The field is a single scalar — nothing to pack yet.** `field_frame.rs` evolves one `Vec<f32>` field
   `U`. A 1-channel field on the GPU is an **`R32F`** texture; packing it into `RGBA32F` wastes three
   channels and *increases* bandwidth. The multi-channel wins in (a) only materialize once the field model
   actually grows channels (a future design — P86 names the `ChannelLease` registry, P87 the state-mask
   plane). Packing ahead of that is premature (ponytail/YAGNI).
3. **On WebGPU, storage *buffers* are often the better default.** When the pipeline lands WebGPU-first, a
   structured compute kernel can use storage buffers with arbitrary struct layout — frequently simpler and
   more flexible than textures. The texture-packing advantages (free bilinear, 2-D spatial cache locality,
   4-channel-natural-vector) are strongest for the **2-D-grid stencil** workload and for the **WebGL2
   fallback**; they are *not* a general "always pack into textures" mandate. So even post-unblock, packed
   float textures are the right tool for the *grid field*, and SoA storage buffers for particles/graphs.
4. **`read_write` format limits force ping-pong.** Only `r32*` formats are `read_write` in a single WGSL
   shader; `rgba32float`/`rg32float` are read-only OR write-only (WebGPU Fundamentals). So any *packed*
   float state is inherently a **ping-pong** (two textures, swap per step) — which is fine (it mirrors the
   CPU `step()`'s existing double-buffer swap at `field_frame.rs:220-221`) but is a real design constraint,
   not a free lunch.

Net: (b) is "yes, and here's the exact shape — but only after P38 §4.2, only for the grid field, and only
once it's multi-channel." dowiz has correctly **not** built it early.

### (c) Doesn't apply

- **`frame_rgba()` itself** — the DISPLAY output (`field_frame.rs:229`). RGBA8, lossy, a 1→4 colormap of one
  scalar, dictated by the canvas/`wgpu`-swapchain blit format. It is a compute *output*, never an input; you
  cannot and would not feed it back into a stencil. This is the premise of the task and §1 confirms it from
  source. GPU-compute packing simply is not what this buffer is.
- **The particle / vertex path** — `ParticleBuffer` `[x,y,vx,vy,life]` (`zerocopy.rs:21-24`) and
  `VertexBridge`'s staging (`bridge.rs:69-84`) cross the boundary as an **interleaved vertex/storage
  buffer** for `writeBuffer`, not as a texture. Its compute (integration, forces) wants **SoA storage
  buffers** — the same SoA conclusion the sibling CPU audit reached, and the reason the external research
  notes "vertex pulling beats instancing." Forcing particle state into an `RGBA32F` texture would be a
  category error unless dowiz deliberately adopts texture-based GPGPU particles (a position-texture), which
  it has not designed. Texture packing does not apply here.
- **The kernel's CPU compute** — `mat.rs` (contiguous-flatten), `csr.rs` (SoA triplets), `simd.rs` (SoA AVX2
  lanes), `spectral.rs`. These never touch a texture or a GPU; they are CPU throughput workloads already
  laid out correctly per the sibling doc. GPU texture packing is irrelevant to them.
- **Scalar structs** — `money.rs::OrderTotalEstimate`, `geo::RouteProgress`. No grid, no batch, no GPU. N/A.

---

## 5. Concrete mapping (what packs where, and when)

| dowiz surface | File:line | Right GPU medium | Pack into a float texture? | Regime |
|---|---|---|---|---|
| Grid field `U` (today, 1 ch) | `field_frame.rs:159-222` | `R32F` texture | No (1 channel — nothing to pack) | (b) not yet |
| Grid field `(U, U̇)` state | `field_frame.rs:208-221` | `RG32F` ping-pong | **Yes** — one fetch = both update operands | **(a)** when GPU-resident |
| Complex spectral `(re, im)` per cell | (future) | `RG32F` | **Yes** — natural 2-channel texel | **(a)** when it exists |
| Vector field `(vx,vy,p,ρ)` | (future, fluid direction) | `RGBA32F` ping-pong | **Yes** — GPU Gems 38 canonical | **(a)** when it exists |
| Per-cell coeffs `(Γ,c²,M,S)` | `field_frame.rs:39-49` (today scalar) | `RGBA32F` | **Yes** if media become heterogeneous | **(a)** when heterogeneous |
| Display frame | `field_frame.rs:229` (`frame_rgba`) | `RGBA8` swapchain / blit | No — it's the *output* format | **(c)** premise |
| Particles `[x,y,vx,vy,life]` | `zerocopy.rs:21-24`, `bridge.rs:69` | SoA **storage buffer** | No — buffer + SoA, not a texture | **(c)** |
| Kernel CPU compute | `mat.rs`/`csr.rs`/`simd.rs` | (CPU only) | No — never a texture | **(c)** |

**Recommendations (all gated behind P38 §4.2, none actionable offline):**

1. When W21 unblocks, the grid field's GPU home should be a **ping-pong pair of `RG32F` textures**
   carrying `(U, U̇)` (or `(U, U_prev)`), not two separate `R32F` textures — that is the first real
   packing win and it mirrors the existing CPU double-buffer swap exactly.
2. Keep particles on **SoA storage buffers**; do not texture-pack them.
3. Reserve `RGBA32F` for the day the field is a genuine vector/heterogeneous-coefficient field; until then
   an `R32F` scalar texture is correct and packing is premature.
4. Budget the **WebGL2 fallback separately**: there, packed float-texture ping-pong is the *only* GPGPU path,
   so the WebGPU storage-buffer design must have a texture-ping-pong twin for the grid field (the WebGPU
   `read_write` limit already pushes the grid design toward ping-pong, so the two paths converge).

**Bottom line:** GPU-compute texture packing is a **real, established technique with a genuine home in
dowiz's field engine** — the 2-D grid PDE, packed as `RG32F`/`RGBA32F` ping-pong textures with free
hardware bilinear, and the sole GPGPU mechanism on the WebGL2 floor. But that home **does not exist yet**
(no GPU pipeline, single-channel field, P38 §4.2 gate), it is the right tool for the **grid field only**
(particles want SoA buffers), and it never describes `frame_rgba`'s display output. The technique is
sound; dowiz is correctly not paying for it before the pipeline that would use it exists.

---

## Sources

- Mark Harris, "Fast Fluid Dynamics Simulation on the GPU," *GPU Gems* Ch. 38, NVIDIA —
  <https://developer.nvidia.com/gpugems/gpugems/part-vi-beyond-triangles/chapter-38-fast-fluid-dynamics-simulation-gpu>
- "GPGPU," WebGL2 Fundamentals — <https://webgl2fundamentals.org/webgl/lessons/webgl-gpgpu.html>
- Amanda Ghassaei, `gpu-io` (WebGL2 GPGPU physics library) — <https://github.com/amandaghassaei/gpu-io>
- "Storage Textures," WebGPU Fundamentals — <https://webgpufundamentals.org/webgpu/lessons/webgpu-storage-textures.html>
- In-repo corpus: `docs/design/living-interface-2026-07-16/EXTERNAL-RESEARCH-gpu-neural-field-sonification.md`
  (§1 WebGPU-vs-WebGL2, §6 WGSL f32 / pack4x8unorm); sibling audit
  `docs/research/OPUS-PERF-RGB-PACKING-REUSE-2026-07-18.md`; blueprints
  `docs/research/BLUEPRINT-W15-wgpu-shell.md`, `docs/design/BLUEPRINT-W21-field-ui-gpu-blocked.md`;
  roadmap `docs/design/CORE-ROADMAP-2026-07-17/SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` (P86/P87/P38 §4.2).
- Code: `engine/src/field_frame.rs`, `engine/src/bridge.rs`, `engine/src/zerocopy.rs`, `engine/Cargo.toml`.
</content>
