# Where does `frame_rgba`'s "RGB-like packing" actually generalize?

**Date:** 2026-07-18
**Author:** Opus (research/audit pass)
**Scope:** `engine/src/field_frame.rs::frame_rgba` vs the kernel's matrix / sparse /
SIMD data-layout work. Honest verdict per candidate. No code changed.

---

## TL;DR

The operator's hope is that `frame_rgba`'s interleaved-multi-channel byte packing
is a general perf technique worth spreading to "other nested matrices, tensors,
or functions." **It is not.** Precisely dissected:

1. `frame_rgba` is **AoS / interleaved** (4 channels contiguous *per pixel*), and
   it is a **1→4 derived colormap of a single scalar**, not 4 independent scalars
   packed together. Its interleaving + fixed `A=255` + RGBA byte order is dictated
   by the **display/blit buffer format** (canvas `ImageData` / a future `wgpu`
   texture upload expects interleaved RGBA8). It is a *boundary-format constraint*,
   not a cache/SIMD optimization.
2. The codebase **already has** the real structural twin of this pattern —
   `engine/src/zerocopy.rs::ParticleBuffer` (`[x,y,vx,vy,life] * N`, stride 5) —
   and it exists for the **same reason**: it crosses the WASM→GPU boundary as one
   interleaved vertex record. Both AoS buffers in the tree sit at a GPU boundary.
3. Where the kernel wanted genuine **CPU compute throughput** on batched
   independent units, it deliberately chose **SoA** (`kernel/src/simd.rs`, the
   Kalman/softmax AVX2 lane) or **contiguous-flatten** (`kernel/src/mat.rs`),
   **not** channel-interleaving. Those are the load-bearing perf patterns.
4. The one genuinely actionable internal improvement (spectral `evecs:
   Vec<Vec<f64>>` → one contiguous buffer) is the **`mat.rs` lesson**, not the
   RGBA lesson. Applying RGBA-style component-interleaving there would be
   *actively harmful*.

So the transferable lesson is the boring one: **know your access pattern before
choosing AoS vs SoA.** There is no "spread the RGBA trick" win.

---

## 1. What `frame_rgba` actually does (precise)

`engine/src/field_frame.rs:229-249`. Input: `self.u` — a single scalar field,
`Vec<f32>` of length `w*h`. Output: `Vec<u8>` of length `w*h*4`.

```rust
for i in 0..n {
    let v = self.u[i];
    let mag = if v.is_finite() { v.abs().min(1.0) } else { 0.0 };
    let b = (mag * 255.0) as u8;                       // (a) branchless float→u8 quantize
    let (r, g, bl) = if v >= 0.0 {                      // (b) sign → warm/cool BRANCH
        (b, (b as u32 * 128 / 255) as u8, (b as u32 * 40 / 255) as u8)
    } else {
        ((b as u32 * 40 / 255) as u8, (b as u32 * 128 / 255) as u8, b)
    };
    out[i * 4]     = r;                                 // (c) INTERLEAVED store, stride 4
    out[i * 4 + 1] = g;
    out[i * 4 + 2] = bl;
    out[i * 4 + 3] = 255;                               // (d) constant alpha
}
```

Three things matter for the reuse question:

- **(a) The float→u8 quantize** `(mag * 255.0) as u8` is a genuine branchless
  clamp: `mag` is pre-clamped to `[0,1]`, and Rust's `f32 as u8` saturates, so no
  `if` is needed. The channel ratios `(b as u32 * 128 / 255) as u8` are integer
  fixed-point scaling. These *are* nice branchless tricks — but they are scalar
  quantization tricks, orthogonal to the memory layout, and are only meaningful
  when the destination is an **8-bit** display channel.
- **(c) The interleaving is stride-4 AoS**: pixel `i`'s four bytes are contiguous
  at `out[4i .. 4i+4]`. This is exactly the RGBA8 texture/`ImageData` memory
  format. Nothing computes over "all R bytes" or "all G bytes" as a column — the
  consumer is a GPU blit / `putImageData` that reads the whole interleaved buffer.
- **(d) The 4 channels are all functions of ONE input scalar** `v`. This is a
  colormap (sign→hue, magnitude→brightness), i.e. a **1→N fan-out**, not "N
  independent quantities that happened to be packed." That distinction is fatal
  to the generalization: the candidates the operator named (Kalman state+cov,
  eigenvector components, order money fields) are **N genuinely-independent
  quantities**, a different problem entirely.

**Verdict on `frame_rgba` itself:** display-buffer-specific. The interleaving is
imposed by the output format; the branchless quantize is 8-bit-channel-specific.
Neither is a reusable "make the kernel faster" primitive.

---

## 2. The three kernel layouts, contrasted

| Site | Layout | What it stores | Why that layout |
|------|--------|----------------|-----------------|
| `field_frame.rs::frame_rgba` | **AoS / interleaved** (stride 4) | 4 derived channels of 1 scalar | GPU/blit RGBA8 format constraint |
| `zerocopy.rs::ParticleBuffer` | **AoS / interleaved** (stride 5) | `[x,y,vx,vy,life]` per particle | WASM→GPU vertex-record boundary |
| `mat.rs::Mat` | **contiguous flat, row-major** | homogeneous scalars `data[i*ncols+j]` | kill `Vec<Vec>` pointer-chase; linear matmul walk |
| `csr.rs::Csr` | **SoA triplets** (`row_ptr`, `col_idx ‖ val`) | sparse (col,weight) pairs | type-homogeneous runs; deterministic SpMV |
| `simd.rs` (Kalman/softmax lane) | **SoA** (`xs`,`ps`,`qs`,`rs` as `[f64;4]`) | one field per lane across the batch | one AVX2 register = one field × 4 lanes |

Two observations that decide the whole question:

- **`mat.rs` is NOT the RGBA pattern.** It flattens a 2-D matrix of *homogeneous*
  scalars into one buffer. Its lesson is "one contiguous `Vec` beats
  `Vec<Vec<f64>>` pointer-chasing" (stated verbatim in its own module doc). That
  is a real and widely-applicable lesson — but it is *contiguity*, not
  *multi-channel interleaving*. RGBA's stride-4 packing of heterogeneous channels
  is a strictly different idea.

- **`csr.rs` deliberately chose SoA over AoS.** Its builder assembles `Vec<(usize,
  f64)>` AoS tuples in scratch, then **splits them into parallel `col_idx` / `val`
  arrays** for the stored form (`csr.rs:88-108`). That is a conscious AoS→SoA
  conversion at the storage boundary — the opposite direction from "spread RGBA
  interleaving." SoA won there because `usize` indices and `f64` weights are
  different widths and each is scanned as a homogeneous run.

**So the kernel already made the AoS-vs-SoA decision correctly, per site, on the
merits.** `frame_rgba` and `ParticleBuffer` are AoS because a GPU boundary demands
it; `mat`, `csr`, and `simd` are contiguous/SoA because CPU compute demands it.

### AoS vs SoA — the one-line rule this codebase already embodies

- **Interleaved / AoS** wins when the consumer touches *all fields of one element
  together* (a pixel is blitted whole; a vertex is fetched whole with a stride).
- **SoA** wins when the consumer touches *one field across many elements* (an
  AVX2 register holds `x` for 4 couriers; a matmul walks one row of scalars).

`frame_rgba` and `ParticleBuffer` are the former. Everything the operator wants to
speed up is the latter.

> Terminology nit worth fixing: `zerocopy.rs:22` labels ParticleBuffer's layout
> "SoA-record layout" while describing "`[x,y,vx,vy,life]` contiguous per particle
> (stride 5)" — that description **is AoS/interleaved**, not SoA. Even the authors
> conflated the two terms, which is exactly the confusion this audit resolves.

---

## 3. Per-candidate verdict (the honest part)

### Candidate A — N-courier Kalman state (`x, P, Q, R` per courier)
**Current:** `Vec<TrustEstimate>`, each wrapping `KalmanFilter { x: Vec<f64>, p:
Mat, … }` — an **array of rich structs** (AoS-of-structs). `simd.rs::kalman_batch_step`
already gathers these into SoA `[f64;4]` lanes *on the fly* for the AVX2 step and
scatters back.
**Right layout:** **SoA**, and the codebase already proves it (bit-identical +
`speedup >= 1.0` gate in `simd.rs`). If anything is ever optimized further, it is
persisting a SoA store (parallel `xs/ps/qs/rs` Vecs) to delete the per-call gather —
**not** interleaving `x,P,Q,R` contiguously per courier, which would scatter every
SIMD lane load.
**RGBA interleaving here:** ❌ actively wrong — it fights the SIMD lane.
**Caveat:** the persistent-SoA refactor touches per-courier filter authority
(`apply_event_with_trust`, the NO-COURIER-SCORING red line). The current
AoS-of-structs + on-the-fly-SoA is a defensible middle; the gather cost is small.
Not urgent.

### Candidate B — Spectral eigenvectors (`evecs: Vec<Vec<f64>>`, k vecs × length n)
**Current:** `spectral.rs:280` (and `:421`, `:400`, `:529`) — `k` separate heap
allocations, pointer-chased. **Access pattern (verified `spectral.rs:300-360`):**
every consumer walks **one whole eigenvector contiguously** — `for v in
evecs.iter() { for i in 0..n { proj += v[i]*x[i] } }` (Gram-Schmidt / Hotelling
deflation / Rayleigh quotient are all full-length dot products).
**Right layout:** the **`mat.rs` contiguous-flatten lesson** — one `Vec<f64>` of
`k*n`, eigenvector `m` at `[m*n .. (m+1)*n]`. Each dot product then walks a
contiguous run; `k` allocs collapse to 1.
**RGBA interleaving here:** ❌ actively harmful — interleaving components
(`evec0[0], evec1[0], … , evec0[1], …`) would stride-scatter every dot product,
the exact opposite of what the whole-vector access wants.
**This is the one genuine, actionable win in the audit — and it is NOT the RGBA
lesson.** It is "kill `Vec<Vec>`," the same move `mat.rs` already made for dense
matrices. The sibling `Vec<Vec<f64>>` surfaces in `spectral.rs::matmul`,
`absorbing.rs`, and `hydra.rs::topology_adjacency` are the same opportunity.

### Candidate C — Order money fields (`delivery_fee, tax_total, total, …`)
**Current:** `money.rs::OrderTotalEstimate` — `Option<i64>` fields on **one**
struct, computed once per order estimate. There is no hot array of these being
bulk-processed.
**Right layout:** a plain struct. Neither AoS-interleave nor SoA applies; there is
no batch to lay out.
**RGBA interleaving here:** ❌ N/A — pure over-engineering (ponytail/YAGNI). No
reuse. (These are also `i64` minor-units on purpose — money never touches the f32
display path.)

### Candidate D — `geo::RouteProgress` (`remaining_m, snapped:(f64,f64), segment_index`)
Single struct per `progress_along_route` call, not batch-iterated. Same as C:
plain struct, no layout change warranted. ❌ N/A.

### Candidate E — `geo` polygon footprints (`&[Vec<(f64,f64)>]`)
The `(f64,f64)` points are **already AoS tuples** (x,y interleaved), and that is
**correct** — LOS/segment tests (`los_clear`, `orient`, `on_segment`) consume both
coordinates of a point together, so the point is the logical unit. ✅ already the
right layout; no change. (A worked example of AoS being correct when the element
is touched whole — the same reason `frame_rgba`/`ParticleBuffer` are AoS.)

---

## 4. Verdict summary

| Candidate | Best layout | Is it the RGBA lesson? |
|-----------|-------------|------------------------|
| Kalman N-courier state | SoA (already done in `simd.rs`) | No — opposite |
| Spectral eigenvectors `Vec<Vec<f64>>` | contiguous-flatten (`mat.rs` lesson) | No — and interleaving would hurt |
| Order money fields | plain struct | N/A |
| `RouteProgress` | plain struct | N/A |
| geo footprint points | AoS (already correct) | Coincidentally yes — because points are touched whole |

**Bottom line for the operator:** `frame_rgba`'s technique does **not** have much
to teach the rest of the kernel. Its interleaving is a display/GPU-blit format
requirement (and its structural twin `ParticleBuffer` already occupies the only
other place that constraint exists — a GPU boundary). The reusable, load-bearing
data-layout patterns in this tree are the kernel's own **SoA** work (`simd.rs`,
`csr.rs`) and **contiguous-flatten** work (`mat.rs`) — and the single actionable
improvement found (eigenvector storage) is the `mat.rs` lesson, where RGBA-style
component-interleaving would be a regression, not a win. The only thing that
transfers is the discipline: **choose AoS vs SoA by the access pattern, not by
analogy to a buffer that looks superficially similar.**
