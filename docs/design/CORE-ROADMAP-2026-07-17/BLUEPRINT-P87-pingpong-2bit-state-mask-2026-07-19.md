# BLUEPRINT P87 — Minimal-bit-depth (2-bit) ping-pong companion state-mask plane (2026-07-19)

> **Standalone DESIGN blueprint (dowiz `engine/` physics-render + the future `gpu` feature).** One
> coherent, independently-buildable unit against the 20-point contract in
> `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Research source:
> `docs/research/OPUS-PHYSICS-WAVE-ALGORITHMS-AND-TENSOR-SYNTHESIS-2026-07-18.md` §B3 (near-settle is
> a few slow modes), `OPUS-PERF-RGB-GPU-TEXTURE-PACKING-2026-07-18.md` §5 (the full-float packing this
> reinterprets), `OPUS-PINGPONG-SHADOW-COPY-PROPAGATION-2026-07-18.md` §4.2/§4.3 (shared-pair cadence +
> per-cell shadow gate). Divergence recorded in `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md`
> §2 row 5 (operator item B) and §4.3. Format precedent:
> `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`. Grounding tree: `/root/dowiz` at HEAD, read
> live this pass.
>
> **One sentence:** realize the operator's "ping-pong at 2-bit depth" as a **2-bit-per-cell state-mask
> plane** — a packed `u32`/`R32Uint` companion (16 cells per word, ≤ 1/16 the float plane's bandwidth)
> encoding per-cell lifecycle flags `{settled, active, source, invalid}` that ride the float field's
> **shared** ping-pong swap — driving lazy settle-skip and a per-cell shadow-frame validity gate,
> **NOT** 2-bit physics (which is physically meaningless and REJECTED).

---

## VERDICT (stated up front, per session discipline)

**GO-WITH-CONDITIONS — and measure-first; the plane is CUT if it shows no measured win.** The 2-bit
mask is a real, coherent structure (§4) with a clean precision-ladder home (§5). But its value is
**empirical**, and the operator's own standard applies: *"if metrics show I'm wrong I'll admit it."*
Two honest splits govern the build:

1. **The GPU-bandwidth leg is the real bet** (P38 §4.2-gated): a 1/16-width mask that gates texel
   fetch / ROP for settled regions on large grids. If the P81 grid-swept bench (§7) shows a frame-time
   win at high settle fractions, it ships; if not, it is cut and the negative result logged (§6 DoD).
2. **The CPU-authority leg is likely a negative result, and that is stated up front** (§4.1): a
   conservative bit-identical settle-skip on the CPU oracle path must check the cell **and its 4-cell
   Neumann neighborhood** for exact-zero delta before skipping — a check that costs about as much as
   the 5-point stencil it would skip, so the branch rarely pays. The DoD accepts "CPU leg dropped with
   the measurement recorded" as a valid outcome. **The GPU leg is where the win, if any, lives.**

The **2-bit-physics literal reading is REJECTED** (§0.1, mirrored in `SYNTHESIS §7` NEW row): a
4-level amplitude cannot carry the PDE (the CFL/energy math and the bit-exact oracle collapse). No
part of this unit quantizes the physics field; the 2-bit plane is *lifecycle metadata beside* the
float physics.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." Every claim read from source **this pass** (`/root/dowiz`, HEAD).

### 0.1 The physics field is f32 and MUST stay f32 — 2-bit physics is impossible here

`FieldFrame` holds `u`, `u_prev`, `lap_scratch`, `next_scratch` as `Vec<f32>` (`field_frame.rs:159-168`)
and integrates the semi-implicit scheme `U_next = (U + dt·(Γ·U̇ + c²·L·U) + dt·S)/(1 + dt·M)` in **f64
per-cell arithmetic** (`field_frame.rs:207-216`). The stability guard (`assert_stable`,
`field_frame.rs:70-98`) and the Lyapunov energy certificate (`field_energy.rs:289-320`) are defined on
this continuous amplitude. A 2-bit (4-level) amplitude cannot represent `U̇ = (U−U_prev)/dt` or the
Dirichlet energy `½c²⟨U,L₊U⟩` — the scheme and its oracle would be meaningless. **Therefore the 2 bits
are NOT the field; they are a companion lifecycle mask.** (Recorded as a rejected literal reading so no
future pass "completes" it.)

### 0.2 Ping-pong is the engine's native idiom — the mask shares the field pair's swap

`FieldFrame::step` rotates buffers with two `std::mem::swap`s (`field_frame.rs:217-221`): `u_prev ← u`,
`u ← next_scratch`. Per `OPUS-PINGPONG…` §4.2 (applied): a companion plane updated by the **same step at
the same cadence** as the float field **shares that pair's swap** (same stencil footprint + cadence +
stability ⇒ shared pair). The mask is a **channel-plane of the field's ping-pong unit**, never an
independently-evolved texture with its own pair.

### 0.3 The settle regime is real and measured (the settle-mask's justification)

`step_reduces_magnitude_toward_source_equilibrium` (`field_frame.rs:378-425`) proves the field
**converges**: after 3000 steps on a fixed disk source the field is finite and bounded (`:410`), and
**300 further steps change it by `max_delta < 1e-2`** (`:412-424`) — i.e. most cells are effectively
settled long before the animation is "done". R14 B3's "near-settle field is dominated by a few slow
modes" is exactly this regime: the bulk of cells stop changing while a few slow modes finish. That is
the population the `settled` bit marks and the lazy-skip targets.

### 0.4 The validity bit's gate already exists — the energy/Lyapunov tolerance

`field_energy.rs` holds the per-step energy tolerance `TOL_E = 1e-6` (`field_energy.rs:165`) and the
`lyapunov_nonincreasing` certificate (`field_energy.rs:302-305`). A GPU-shadow cell whose local energy
delta exceeds tolerance is exactly the `invalid` case (R16 §4.3): mark it, fall back to the CPU
authority value for that cell. The gate is a per-cell instance of an already-tested global check.

### 0.5 The value range that fixes the flag/validity encoding

`frame_rgba` clamps `|v|` to `[0,1]` for display (`field_frame.rs:234`); the field's working amplitude
is O(1). This bounds the fixed-point/tolerance math the `invalid` bit needs — no unbounded range to
encode.

### 0.6 The GPU plane does not exist in the default build (the gate)

`engine/Cargo.toml:36` webgpu flags empty; `bridge.rs:214-218` pins no-GPU default. The **GPU mask
plane** (a real `R32Uint` texture riding the float pair) is P38 §4.2-gated. The **CPU-side mask + the
settle/validity logic** are buildable now on the CPU field for the measure-first bench.

---

## 1. Prior-art map — adopt, don't invent (standard §2 item 19)

| Prior art | What it is | How P87 uses it — and what it does NOT take |
|---|---|---|
| **Packed bitset planes / occupancy masks** (GPGPU: a low-bit-depth companion texture gating work on a high-precision one — e.g. sparse-voxel occupancy, tile "dirty" masks) | 1–2 bits/cell metadata steering compute over expensive full-precision state | **Adopt** as the 2-bit lifecycle plane (`R32Uint`, 16 cells/word). **NOT taken:** using the mask to *store* the value — it stores flags, the float plane stores the field. |
| **Ping-pong / double-buffering** (`field_frame.rs:217-221`) | disjoint read/write buffers per step | **Adopt with a SHARED swap** — the mask rotates with the float pair (§0.2), never a second pair. |
| **Dirty-region / lazy-evaluation** (retained-mode UIs; incremental solvers) | skip recompute where nothing changed | **Adopt on the GPU presentation path freely; on the CPU authority path ONLY under a conservative bit-identity proof** (§4.1). |
| **Shadow-then-promote with per-cell fallback** (`OPUS-PINGPONG…` §4.3) | compute a cheap shadow, promote only cells within tolerance, else fall back to authority | **Adopt** as the `invalid` bit's fallback gate (§4.2), reusing the `TOL_E` energy tolerance (§0.4). |
| **RGBA32F/RG32F multi-channel field state** (`OPUS-PERF-RGB-GPU-TEXTURE-PACKING` §5) | pack `(vx,vy,p,ρ)` / complex `(re,im)` into wide float texels | **Reconciled, not overturned:** that is the *float authority/presentation* layer; the 2-bit mask is a *third, lower* rung (§5), not a replacement. R13's full-float assumption and the operator's low-bit direction are both right, at different layers. |

**What P87 invents:** nothing runtime beyond a packed-mask helper. It composes the existing ping-pong,
the existing energy tolerance, and a standard occupancy-mask idiom.

---

## 2. Scope — what P87 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P87 OWNS

1. **The 2-bit lifecycle-flag encoding** (`{settled, active, source, invalid}`) and the packed
   `u32`/`R32Uint` layout (16 cells/word) that shares the float field's ping-pong swap (§3, §4.3).
2. **The settle-mask lazy-skip** on the GPU presentation path (skip freely) and — conditionally, and
   only if it benches positive — the conservative bit-identical skip on the CPU authority path (§4.1).
3. **The per-cell `invalid` validity gate** for the GPU shadow-frame fallback (§4.2).
4. **The precision-ladder contract** (f32 authority / f16 presentation / 2-bit flags) as P87's
   single-owner contract (§5) — no other doc may redefine it.
5. **The measure-first DoD** — the plane exists only if it shows a measured win, else it is cut (§6).

### 2.2 P87 does NOT own (anti-scope)

- **The physics field precision.** The field stays f32/f64 (§0.1). P87 never quantizes `U`.
- **`money.rs` / the CPU determinism oracle.** The excluded red-line
  (`SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` §5); no mask, no fixed-point, no skip logic
  ever touches money or the bit-exact oracle definition. The mask only *reads* the oracle to *prove*
  its CPU-leg skip is bit-identical — it never alters it.
- **The channel-lease/pair-pool allocation** — that is **P86**. The mask plane *leases a channel slot*
  through P86's `SlotArena<ChannelAlloc>`; P87 does not design that registry.
- **The atomicity rule for the mask-update shader** — that is **P88** (§4.3); P87's mask-update shader
  *inherits* it (the mask write is class-(c) single-writer under the shared ping-pong, or an atomic
  set where a scatter marks `invalid`).
- **The swap-parity "which buffer is current" tag** — real but **tiny**: a per-pair scalar carried as
  a field on P86's `PingPongPair`, not a plane. Named here so the interpretation space is honestly
  covered (`SYNTHESIS §4.3` item 3), owned by P86.

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs:** `FieldFrame::step`/ping-pong (`engine/src/field_frame.rs`); the `compose()` oracle
(`field_frame.rs:255-262`) for the CPU-leg bit-identity proof; `field_energy.rs`'s `TOL_E` +
`lyapunov_nonincreasing` for the `invalid` gate; **P81** (engine bench harness — the DoD substrate);
**P86** (the channel-lease the mask plane rides); **P88** (the atomicity rule the mask shader inherits);
P38 §4.2 (the GPU leg). **Consumers:** the GPU render path (settle-skip) and the shadow-frame gate
(validity fallback).

### 2.4 Honest reconciliation (standard §2 item 6)

The research assumed full-float `RG32F`/`RGBA32F` state; the operator asked for 2-bit ping-pong. **Both
are right at different layers** (§5): the field/presentation state is float (unchanged); the 2-bit plane
is a *new lower rung* of lifecycle metadata. P87 does not overturn the float packing — it adds a
companion. The literal "2-bit physics" is the only reading rejected, and it is rejected on physics, not
preference (§0.1).

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

```rust
// engine/src/state_mask.rs  (NEW; the CPU-side mask + logic always-compiled; the R32Uint texture
//                            leg #[cfg(feature = "gpu")])

/// 2-bit per-cell lifecycle flag. Exactly 4 states — the whole point of the 2-bit width.
/// Packed 16 cells per u32 (2 bits × 16 = 32). Encoded as a u8 with values 0..=3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CellFlag {
    Active   = 0b00, // evolving normally — full stencil work
    Settled  = 0b01, // |Δ| below settle epsilon for this cell AND its Neumann neighborhood
    Source   = 0b10, // an SDF source/attractor cell (driven; never skipped)
    Invalid  = 0b11, // GPU shadow failed the energy tolerance → fall back to CPU authority value
}

/// Packed 2-bit mask plane over a w×h grid. `words.len() == ceil(w*h / 16)`.
/// Shares the float field's ping-pong: rotated by the SAME swap, never a second pair (§4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateMask {
    words: Vec<u32>,   // 16 cells / word
    w: usize,
    h: usize,
}

/// Settle epsilon: a cell is a `Settled` CANDIDATE when its per-step |ΔU| ≤ this. The CPU-authority
/// skip additionally requires the 4-neighborhood to be EXACTLY 0.0 (conservative; §4.1). Pinned to
/// the convergence test's observed near-settle margin (field_frame.rs:412-424 uses max_delta < 1e-2
/// as "near equilibrium"); the SETTLE candidate epsilon is looser than the CPU-skip exact-zero rule.
pub const SETTLE_EPSILON: f32 = 1e-3;

/// The mask plane's memory MUST be ≤ 1/16 the float plane's (2 bits vs 32 bits). Asserted in test.
pub const MASK_BITS_PER_CELL: usize = 2;
pub const FLOAT_BITS_PER_CELL: usize = 32;
```

```rust
impl StateMask {
    pub fn new(w: usize, h: usize) -> Self { /* zeroed = all Active */ }
    #[inline] pub fn get(&self, i: usize) -> CellFlag { /* unpack 2 bits */ }
    #[inline] pub fn set(&mut self, i: usize, f: CellFlag) { /* pack 2 bits */ }
    /// Recompute flags from the last step's per-cell delta + source set. Runs at the field cadence.
    pub fn update(&mut self, delta: &[f32], source: &[f32]) { /* §4.4 */ }
    /// The shared swap: the mask rides the float pair. Called from the SAME rotation site.
    pub fn swap_with(&mut self, other: &mut StateMask) { std::mem::swap(&mut self.words, &mut other.words); }
}
```

---

## 4. Build items — spec → RED test → code, each with adversarial cases (standard §2 items 2, 3, 5)

### 4.1 M1 — settle-mask lazy skip (the determinism split — the load-bearing correctness point)

- **Spec.** A cell may be marked `Settled` when its per-step `|ΔU| ≤ SETTLE_EPSILON`. **Skip rule
  differs by path:**
  - **GPU presentation path:** a `Settled` cell may skip its stencil update **freely** — this path is
    already non-bit-identical to the CPU oracle by canon (P38 §3.1), so a slightly-stale settled texel
    is acceptable.
  - **CPU authority path:** a `Settled` cell may be skipped **ONLY if its update is provably
    bit-identical to not skipping** — i.e. the cell's value **and** all 4 Neumann neighbors' values are
    **exactly** unchanged (delta `== 0.0`) across the previous step, so `laplacian_into` (which reads
    the 4 neighbors, `field_frame.rs:147-151`) would produce the identical `next` value. Otherwise the
    oracle (`compose()`) dies. This is the conservative mask.
- **HONEST expected result (stated up front):** the CPU-leg neighborhood exact-zero check costs ~5
  reads + compares — comparable to the 5-point stencil it skips — so the CPU leg is **likely a wash or
  a loss**. The DoD (§6) accepts "CPU leg measured, found non-winning, dropped, result logged" as a
  valid completion. The GPU leg is the real bet.
- **RED `red_cpu_skip_is_bit_identical_to_oracle`:** run `compose()` normally vs a mask-skipping CPU
  variant over identical scene/eq/steps → **byte-identical RGBA** (`field_frame.rs:427-451` pattern).
  RED if the skip ever diverges (proves the conservative rule is actually conservative); GREEN only
  when the exact-zero-neighborhood rule holds.
- **Adversarial `red_settled_cell_reactivates_on_neighbor_change`:** a settled cell whose neighbor
  becomes active (a new source arrives) must **not** be skipped the next step — its Laplacian changed.
  Assert the mask clears `Settled → Active` when any neighbor's delta ≠ 0. RED against a naive
  self-only settle check (which would wrongly keep skipping and freeze a stale value).

### 4.2 M2 — per-cell `invalid` validity gate (the shadow-frame fallback)

- **Spec.** On the GPU shadow path, a cell whose local energy delta exceeds `TOL_E` (§0.4) is marked
  `Invalid`; the compositor uses the **CPU authority value** for `Invalid` cells (per-cell shadow
  comparison, R16 §4.3). This never affects the CPU oracle — it only decides which of {GPU-shadow,
  CPU-authority} the presentation reads per cell.
- **RED `red_invalid_cell_falls_back_to_authority`:** inject a cell whose shadow value violates the
  energy tolerance → it is marked `Invalid` and the presented value equals the CPU-authority value,
  not the bad shadow. RED (no per-cell gate today), GREEN after.
- **Adversarial `red_invalid_gate_non_vacuous`:** a shadow that is *within* tolerance everywhere marks
  **zero** cells `Invalid` (the gate does not spuriously fall back) — mirrors the non-vacuousness
  discipline of `field_energy.rs`'s `energy_gate_catches_anti_diffusion`.

### 4.3 M3 — shared-pair swap + packed layout (cadence rule)

- **Spec.** The `StateMask` is rotated by the **same** swap site as the float field
  (`field_frame.rs:217-221`), never a second ping-pong pair. Layout: 16 cells/`u32`, so the plane is
  exactly `MASK_BITS_PER_CELL / FLOAT_BITS_PER_CELL = 1/16` the float plane's memory.
- **RED `red_mask_shares_float_swap_cadence`:** advance N steps; assert the mask's "current" buffer
  parity always equals the float field's current parity (they swap together). RED against an
  independently-swapped mask (which would desync flags from values).
- **RED `red_mask_memory_is_one_sixteenth`:** assert `mask.words.len() * 32 ≤ (w*h*32) / 16 + 32`
  (rounding slack) — the ≤ 1/16 memory claim, machine-checked.

### 4.4 M4 — flag recompute from delta + source

- **Spec.** `StateMask::update(delta, source)`: `source[i] != 0 ⇒ Source`; else `|delta[i]| ≤
  SETTLE_EPSILON` **and** no neighbor active ⇒ `Settled`; else `Active`. `Invalid` is set only by the
  shadow gate (M2), never by `update`.
- **RED `red_source_cell_never_settles`:** a driven source cell keeps `Source` even at zero delta — it
  must never be skipped (it anchors the field). RED against a settle check that ignores the source set.

### 4.5 WebGL2 / CPU-floor fallback — the GPU texel-skip leg is WebGPU-compute-scoped (FE-16 floor)

P38's FE-16 fallback ladder is **WebGPU → WebGL2 → CPU `compose_field`** (§0.6;
`BLUEPRINT-P38-webgpu-render-engine.md` §3.6, §12.3) — a courier's mid-tier phone may reach only the
WebGL2 rung, which has **no compute shaders and no atomics** (core WebGL2 / GLES 3.0). P87 already
splits a CPU-authority leg from a GPU leg (§4.1); this subsection makes the fallback rung explicit and
carries the P38 §12.3 floor line the audit found missing (META-GAP-AUDIT-2026-07-19 §1 G2). The GPU
leg — the `R32Uint` mask plane, texel-fetch/ROP skip, and the `invalid`-scatter set — is
**WebGPU-compute-only**: the mask-update needs a compute shader, and the `invalid` scatter needs an
atomic set. Neither exists on WebGL2. Plainly:

- **WebGPU rung (post-OD-11):** the full mask plane runs — settle texel-skip + the shadow `invalid`
  gate, its mask-update shader carrying P88's `// SINGLE-WRITER:` proof (shared ping-pong) or an atomic
  set (the scatter, §4.2).
- **WebGL2 and CPU-floor rungs:** the field runs on the **single-threaded CPU** `FieldFrame::step`
  path (the P38 §3.1 authority), presented by a WebGL2 fragment-raster quad or canvas2d `putImageData`.
  The **only** settle optimization available on these rungs is the **CPU-authority settle-skip leg
  (§4.1)** — the conservative exact-zero-neighborhood bit-identical skip — and only **if it benched
  positive** (the honestly-expected outcome is that it is cut, §6 D-BENCH). There is **no GPU
  texel-skip and no atomic scatter on WebGL2**; the 2-bit mask is either a pure-CPU optimizer or
  absent. This is **option (i)** of the audit's G2 choice: the compute leg is WebGPU-only and the
  WebGL2 floor never runs it. Because the CPU writer is single-threaded, **atomicity (the `invalid`
  scatter's atomic set) is moot on these rungs** — there is no concurrent writer.

**DoD addition — the FE-16 floor line (P38 §12.3 standing gate), previously absent (audit G2):**
**D-WEBGL2** — settle/mask behavior is verified **CORRECT on the WebGL2 and CPU floors**, not only on
WebGPU. Falsifier: the CPU oracle stays bit-identical (D-NOREG) whether the mask is present as a
pure-CPU optimizer or absent; and a forced-`navigator.gpu = undefined` degrade path (P38 §3.6 pattern)
composes the field correctly via the CPU path with **no compute-atomics dependency reachable**. A build
whose settle/render correctness on WebGL2 **depends on** the GPU mask plane, or that makes texel-skip a
*requirement* rather than a WebGPU-only optimization, = **NOT done**.

---

## 5. The precision ladder P87 fixes in canon (standard §2 item 16, the single-owner contract)

Reconciling `SYNTHESIS §2` row 5: three rungs, each right at its layer.

| Rung | Precision | Where | Owner |
|---|---|---|---|
| **Authority** | **f32 field / f64 per-cell arithmetic** | `field_frame.rs:207-216`, the CPU oracle | unchanged (pre-existing) |
| **Presentation** | **f16 / fixed-point** GPU state | R14 B2, the GPU display texels | P38 / P88 (fixed-point reduction) |
| **Lifecycle flags** | **2-bit** per-cell mask | this unit's `StateMask` | **P87 (this contract)** |

The operator's low-bit direction and the research's full-float assumption are **both right, at
different layers**. This table is P87's single-owner contract — no other document redefines the ladder.

---

## 6. DoD — falsifiable, RED→GREEN, measure-first (standard §2 item 2)

| # | Done when… | Falsifier (RED test / check) |
|---|---|---|
| D1 | the CPU-authority settle-skip is proven bit-identical to `compose()` (or the CPU leg is dropped with the measurement recorded) | `red_cpu_skip_is_bit_identical_to_oracle` + `red_settled_cell_reactivates_on_neighbor_change` |
| D2 | the `invalid` per-cell gate falls back to authority on tolerance breach, and never spuriously | `red_invalid_cell_falls_back_to_authority`, `red_invalid_gate_non_vacuous` |
| D3 | the mask shares the float pair's swap and costs ≤ 1/16 the float plane | `red_mask_shares_float_swap_cadence`, `red_mask_memory_is_one_sixteenth` |
| D4 | source cells never settle/skip | `red_source_cell_never_settles` |
| **D-BENCH** | **a MEASURED frame-time/bandwidth win from mask-skip on large grids**, else the plane is CUT | §7 grid-sweep {128²,256²,512²} × settle {0%,50%,90%}; **if no win, cut the plane + log the negative result (§8-style)** |
| D-NOREG | the float field oracle stays bit-identical (the mask never alters `U` or `compose()`) | `compose_returns_deterministic_frame`, `allocfree_step_byte_identical` stay green |
| D-WEBGL2 | settle/mask behavior is CORRECT on the WebGL2 + CPU floors (the GPU texel-skip/scatter leg is WebGPU-compute-only; those rungs run the single-threaded CPU path, mask as pure-CPU optimizer or absent) — §4.5 | CPU oracle bit-identical with mask present-or-absent + forced-`navigator.gpu=undefined` composes via the CPU path with no compute-atomics dependency reachable (P38 §12.3 floor line) |

**The measure-first gate is binding:** per the operator's standard, if D-BENCH shows no win the plane
does **not** ship and the negative result is recorded — exactly as B4 walked back a shipped
optimization when the numbers did not hold.

---

## 7. Benchmarks + telemetry (standard §2 item 10) — the substrate for the bet

P81's engine bench harness is the substrate (the engine crate has zero benches today, Ledger P81).

| Bench | Measures | Sweep |
|---|---|---|
| `bench_field_step_no_mask` | baseline `step()` cost/frame | grid {128²,256²,512²} |
| `bench_field_step_settle_skip_cpu` | CPU-leg skip cost/frame | grid × settle-fraction {0%,50%,90%} |
| `bench_field_step_settle_skip_gpu` | GPU-leg texel-fetch/ROP saved (when P38 §4.2 lands) | grid × settle-fraction |
| `bench_mask_update_overhead` | the `StateMask::update` + neighborhood-check cost | grid |

The bet is confirmed iff the skip saving at high settle fractions exceeds `bench_mask_update_overhead`
on large grids. Telemetry: emit per-frame `{settled_fraction, skipped_cells, frame_ns}` through the
P38 render metrics seam so a regression (mask overhead exceeding its saving) surfaces automatically.

---

## 8. AI/system-hazard safety, grounded in math (standard §2 item 6)

- **Oracle corruption** (the mask silently changing an authoritative field value): unrepresentable —
  the CPU-leg skip is admitted **only** when the skipped update is *bit-identical* to computing it
  (exact-zero neighborhood, §4.1), proven by `red_cpu_skip_is_bit_identical_to_oracle`. The `invalid`
  gate (M2) only chooses *which precomputed value to present*, never mutates `U`.
- **Frozen-stale cell** (a cell wrongly kept `Settled` while its neighborhood moves): caught by
  `red_settled_cell_reactivates_on_neighbor_change` — the settle bit clears on any neighbor delta.
- **Money/oracle red-line:** the mask is lifecycle metadata on the *presentation* field; it never
  encodes money, never enters `money.rs`, never redefines the determinism oracle (§2.2). Reachability
  argued from the exact-zero skip rule + the P38 §3.1 GPU/CPU split.

---

## 9. Cross-cutting obligations (standard §2 items 8, 9, 11–15, 20)

- **Schemas & scaling axis (item 8):** axis = **cells/grid**; the packed plane is `ceil(w·h/16)` words
  and holds to any grid the float plane holds to (it is strictly 1/16 of it). Break point: none of its
  own — it scales exactly as the field does.
- **Isolation / bulkhead (item 11):** the mask is a **read-only optimizer** of the render path — losing
  or corrupting it degrades to "compute every cell" (the current behavior), never a wrong field. A mask
  bug cannot corrupt state (it gates *work*, not *values*, except the bit-identity-gated CPU skip).
- **Mesh awareness (item 12):** **N/A** — node-local render metadata, zero transport payload (P38 §4.4).
- **Rollback / self-healing as math (item 13):** **Self-Termination** = the exact-zero skip predicate
  (an incorrect skip is unrepresentable, not caught by a supervisor). **Snapshot-re-entry** = the mask
  is recomputable from the field delta at any time (`update`), so a lost mask self-heals by
  recomputation. **Self-healing (error-correcting): NOT claimed** — it is metadata, not redundancy.
- **Error-propagation / smart index (item 14):** the bug class (a skip that diverges from the oracle)
  is a **test-time** failure (`red_cpu_skip_is_bit_identical_to_oracle`), not a runtime drift.
- **Living-memory awareness (item 15):** the mask is the field's temporal access pattern made explicit
  — hot cells `Active`, dormant cells `Settled` (demote-never-delete applied to compute, per
  `physics-ui-capture-quantum-math-arc-2026-07-14`). Deliberately ephemeral, recomputed, not persisted.
- **Tensor/spectral (item 16):** the settle regime *is* the spectral fact (few slow modes dominate
  near-settle, §0.3) — P89 (field eigenmodes) is the modal exploitation of the same regime; P87 is the
  cheap per-cell mask complement. Reuses the energy operator's `TOL_E`, invents no new math.
- **Linux discipline (item 9):** **EXTENDS** — a per-cell dirty/settle mask is a new discipline for the
  GPU-vs-CPU path; **REINFORCES** the feature-gated-GPU-with-CPU-floor pattern; **ALREADY-EQUIVALENT**
  on the bit-identity oracle gate (reuses `compose()`).
- **Hermetic principles (item 20):** **Correspondence** — the mask *corresponds* to the field's motion
  ("as the field settles, so the mask marks it"); it is derived, never asserted. **Vibration/Rhythm**
  — the mask breathes with the field at the same cadence (shared swap), the lifecycle a rhythm of
  Active↔Settled.

---

## 10. Standard-compliance map (all 20 points — standard §2)

| # | Item | Where |
|---|---|---|
| 1 | Ground truth `file:line` | §0 |
| 2 | Falsifiable DoD | §6 (incl. measure-first D-BENCH) |
| 3 | Spec→test→code, event-modeled | §4 (each M spec→RED→code; flags as per-step events) |
| 4 | Predefined types & constants | §3 (`CellFlag`, `StateMask`, `SETTLE_EPSILON`) |
| 5 | Adversarial/breaking tests | §4.1/§4.2 (reactivation, non-vacuous gate) |
| 6 | Hazard-safety from math | §8 (exact-zero skip; honesty split) |
| 7 | Links to docs & memory | §12 |
| 8 | Schemas with scaling axis | §9 (cells/grid; scales as the field) |
| 9 | Linux engineering discipline | §9 (EXTENDS/REINFORCES/ALREADY-EQUIVALENT) |
| 10 | Benchmarks + telemetry | §7 (grid×settle sweep; settled-fraction telemetry) |
| 11 | Isolation / bulkhead | §9 (read-only optimizer; degrades to compute-all) |
| 12 | Mesh awareness | §9 (N/A, node-local) |
| 13 | Rollback/self-heal as math | §9 (self-termination; recompute self-heal; no error-correction) |
| 14 | Error-propagation / smart index | §9 (bit-identity test-time gate) |
| 15 | Living-memory awareness | §9 (Active/Settled temporal pattern) |
| 16 | Tensor/spectral | §9 (settle = slow-mode regime; P89 complement) |
| 17 | Regression tracking | §6 D-NOREG + D-BENCH result logged in REGRESSION-LEDGER |
| 18 | Clear worker instructions | §12 |
| 19 | Reuse-first, upgrade-if-needed | §1 (occupancy-mask/ping-pong/shadow all adopted), §2.2 anti-scope |
| 20 | Hermetic principles | §9 (Correspondence, Rhythm) |

---

## 11. Rollout sequencing (per the master ledger)

Master Ledger §3 wave-4 (GPU-gated) + §4 item 12. **P87 sequences AFTER P88 → P86**, and its GPU leg
is P38 §4.2-gated:

1. **P88 first** (the atomicity rule the mask-update shader inherits).
2. **P86 next** (the channel-lease the mask plane rides).
3. **P81** provides the bench substrate.
4. **P87 CPU-side** (`StateMask` + settle/validity logic + the measure-first CPU bench) is buildable
   once P81 lands — this delivers the D-BENCH verdict for the CPU leg **without** P38 §4.2.
5. **P87 GPU leg** (`R32Uint` plane, texel-fetch skip, shadow gate) lands **only after** P38 §4.2 and
   inherits P88's rule + R16's shared-pair rule.

---

## 12. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` §2 row 5, §4.3 (this unit's sketch), §5
  (money/oracle exclusion), §7 (2-bit-physics REJECTED row).
- `MASTER-STATUS-LEDGER-2026-07-19.md` §1 (P87 row), §3 wave-4, §4 item 12 (precision ladder = single
  owner).
- `docs/research/OPUS-PHYSICS-WAVE-ALGORITHMS-AND-TENSOR-SYNTHESIS-2026-07-18.md` §B3 (slow-mode
  settle), `OPUS-PERF-RGB-GPU-TEXTURE-PACKING-2026-07-18.md` §5, `OPUS-PINGPONG-SHADOW-COPY-PROPAGATION-2026-07-18.md`
  §4.2/§4.3 (recovered — restore per Ledger §0).
- `BLUEPRINT-P88-atomicity-by-default-physics-gpu-2026-07-19.md` §4.3 (the rule the mask shader inherits).
- `BLUEPRINT-P86-…` (the channel-lease the plane rides) · `BLUEPRINT-P38-webgpu-render-engine.md` §3.1/§4.2.
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Memory:
  `physics-ui-capture-quantum-math-arc-2026-07-14.md`, `performance-priority-over-minimal-change-2026-07-17.md`.

**Existing code this blueprint edits/extends (exact targets, dowiz):**
- **NEW** `engine/src/state_mask.rs` — §3 `CellFlag`/`StateMask`/constants; settle/validity/update
  logic (always-compiled CPU side); the `R32Uint` texture leg `#[cfg(feature = "gpu")]`.
- **EDIT** `engine/src/field_frame.rs` — call `StateMask::update` at the field cadence and
  `swap_with` at the SAME swap site (`:217-221`); a mask-gated CPU-skip variant behind a flag for the
  bench (guarded so `compose()` stays bit-identical by default).
- **REUSE unchanged** `field_frame.rs::compose` (the bit-identity oracle), `field_energy.rs` (`TOL_E`,
  Lyapunov), P86's `ChannelAlloc`/`PingPongPair`, P88's atomicity rule.
- **DO NOT TOUCH** `kernel/src/money.rs`, the determinism oracle definition, or the field's f32/f64
  arithmetic (§0.1 — no 2-bit physics).

**For the worker with zero session context — exact acceptance path:**
1. Build the **CPU-side `StateMask`** + settle/validity/update logic + its RED tests first (no GPU
   feature). This proves the correctness invariants (D1–D4) and is independent of P38 §4.2.
2. **Run the measure-first D-BENCH** on the CPU leg via P81. If the CPU settle-skip shows no win (the
   honestly-expected outcome), **drop the CPU leg and record the measurement** — this is a valid
   completion, not a failure.
3. Do **not** build the GPU `R32Uint` plane until P38 §4.2 is operator-approved and P86 lands the
   channel-lease. When both hold, the GPU leg leases a channel, rides the float pair's swap, and its
   mask-update shader carries the P88 `// SINGLE-WRITER:` proof (shared ping-pong) or an atomic set
   (the `invalid` scatter).
4. Record the D-BENCH verdict (win → ship; no win → cut + log) in `docs/regressions/REGRESSION-LEDGER.md`.
5. Anti-scope: never quantize the physics field; never skip a CPU-authority cell without the exact-zero
   neighborhood proof; never let the mask redefine or touch the money/oracle path.

---

## 13. Open operator-decision points

| # | Decision | Blocks / affects | Default if unruled |
|---|---|---|---|
| OD-P87-1 | **P38 §4.2** GPU-compute decision (operator-owned) | P87's GPU leg (the real bet) | CPU leg only; GPU plane not built |
| OD-P87-2 | If D-BENCH CPU leg shows no win: cut the CPU leg entirely, or keep it dormant behind a flag | P87 CPU scope | Cut + log the negative result (measure-first default) |
| OD-P87-3 | `SETTLE_EPSILON` value (default 1e-3 for the `Settled` candidate; the CPU-skip rule is exact-zero regardless) | settle fraction, D-BENCH sensitivity | 1e-3 (looser candidate, strict skip) |

---

*Cross-references: `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` (§2 row 5/§4.3/§5/§7) ·
`MASTER-STATUS-LEDGER-2026-07-19.md` (P87 row, wave-4) · `BLUEPRINT-P88-atomicity-by-default-physics-gpu-2026-07-19.md`
(§4.3 inherited) · `BLUEPRINT-P86-…` (channel-lease) · `BLUEPRINT-P38-webgpu-render-engine.md`
(§3.1/§4.2) · `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 · source: `engine/src/field_frame.rs`,
`engine/src/field_energy.rs` · memory: `physics-ui-capture-quantum-math-arc-2026-07-14.md`,
`performance-priority-over-minimal-change-2026-07-17.md`.*
