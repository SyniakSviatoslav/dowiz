# BLUEPRINT P81 — Engine bench harness (first-ever coverage for `dowiz/engine`) (2026-07-19)

> **Standalone COVERAGE blueprint (`dowiz/engine` crate).** One coherent, independently buildable
> unit against the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Research source:
> `docs/research/OPUS-PERF-BENCH-COVERAGE-MAP-2026-07-18.md` §2B + §3 (the "engine crate has ZERO
> benches, runs every frame" finding). Reconciled in `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md`
> §3.3-C2 / §5 (Tier C, Wave W2, unit **P81**). Format precedent:
> `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`. Grounding tree: `/root/dowiz/engine` at
> HEAD, read live this pass.
>
> **One sentence:** the `dowiz-engine` crate — the CPU-authoritative field-UI render engine whose
> `FieldFrame::step`/`compose` pipeline runs on *every animation frame* — has **zero** benchmark
> coverage today; P81 adds its first `criterion` harness with grid/shape/ω/nnz sweeps so a per-frame
> regression trips a gate instead of shipping silently, and pins the RED-LINE `present_money` guard.

---

## VERDICT (stated up front, per session research discipline)

**GO — mechanical, low-risk, high-leverage; but hard-gated on P75.** This is additive test/bench
code only: it changes **no** product source, preserves **all** behavior, and cannot regress the
render path (a bench binary is compiled behind `harness = false` and runs only under `cargo bench`).
The engine is the single largest wholly-unbenched hot surface in the dowiz repo (R5 §1f) and it runs
continuously all session (R5 §2B), so the coverage value is real and the risk is near-zero.

Two conditions bound it honestly:

1. **Hard prerequisite — P75 must land first.** P75 owns the `<group>/<n>` bench-id + `baseline.json`
   schema and re-architects the CI gate that currently `exit(2)`s on every fresh runner (S1 §3.1-A1,
   `bench_track.py`). P81's ~15 new baselines **must** be written into P75's fixed schema, not the
   broken one — otherwise they are recorded but never gate. P81 does not redefine the schema; it
   cites P75's.
2. **Bench-only, not a fix.** R5 §2B records one latent defect on the benched surface — a per-call
   `vec![0.; n]` heap allocation inside `VertexBridge::apply_field` (`bridge.rs:121`) "in a
   documented no-alloc loop." **P81 does NOT fix it.** P81's job is to make it *measurable*; the
   allocation-hoist is a separate, evidence-gated follow-up opened only if the `apply_field/nnz_*`
   bench proves it hot — exactly the bench-first discipline S1 applies to the contended-lock benches
   in P80 (E12) and the Performance Standing Rule requires (`.claude/CLAUDE.md:182-195`: "rewrites
   require a benchmark proving hotness").

There is no measure-first NO-GO gate here (unlike P92): coverage of a zero-covered per-frame path is
unconditionally worth the ~one-file cost; the sweeps *are* the deliverable.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> Read from `/root/dowiz/engine` this pass, not inherited from the research sketch. Two corrections
> to the research/synthesis shorthand are recorded because a correct blueprint requires them.

### 0.1 The engine crate today

| Element | Cite (verified this pass) | State |
|---|---|---|
| crate | `engine/Cargo.toml` `name = "dowiz-engine"` | pure-Rust, `[dependencies]` = only `dowiz-kernel` (path); `serde`/`cosmic-text` optional + off-by-default |
| **`[dev-dependencies]`** | `engine/Cargo.toml` (last stanza) | **EMPTY** — there is no `criterion`, no `[[bench]]`, and **no `engine/benches/` directory** (confirmed: `ls engine/benches` → absent) |
| offline-clean mandate | `engine/Cargo.toml` header comment | "NO dependencies — offline-clean by mandate … the default build has zero external crates." **This governs the harness-shape decision in §1.2 — it is a real constraint, not prose.** |
| `FieldFrame::step(&mut, source, eq)` | `engine/src/field_frame.rs:198` | per-frame integrator (5-point Laplacian stencil + integrate) |
| `laplacian_into(u, w, h, out)` | `engine/src/field_frame.rs:140` | pure inner diffusion kernel called by `step` |
| `FieldFrame::frame_rgba(&self) -> Vec<u8>` | `engine/src/field_frame.rs:229` | per-pixel hue map + **fresh `Vec` alloc per frame** |
| `compose(scene, eq, w, h, steps) -> Vec<u8>` | `engine/src/field_frame.rs:255` | the full-frame pipeline the GPU blit consumes |
| `Scene::render_frame(&self, w, h) -> Vec<f32>` | `engine/src/scene.rs:136` | SDF source buffer fold, O(w·h·shapes) |
| `Spring::step(&mut, dt)` | `engine/src/motion.rs:50` | critically-damped substep loop, ω-dependent count |
| `VertexBridge::apply_field(&mut, x)` | `engine/src/bridge.rs:121` | per-frame graph-Laplacian SpMV; **R5 §2B: `vec![0.; n]` alloc per call** (§0.2 correction) |
| `TweenGuard::present_money(amount_minor: f64) -> Result<i64, String>` | `engine/src/money_guard.rs:60` | 🔴 RED-LINE money-never-tween guard; O(1) |
| `write_into_linear(mem, offset, buf)` | `engine/src/zerocopy.rs:85` | Rust→WASM upload leg |
| `field_energy.rs` | `engine/src/field_energy.rs` | `#[cfg(test)]`-only reference oracle — **correctly NOT a bench target** (R5 §2B note) |

### 0.2 Correction A — R5's cited line for `scene::render_frame` drifted

R5 §2B cites `scene.rs:122` for `Scene::render_frame`; live it is **`scene.rs:136`** (verified this
pass). Same function, +14 lines of drift since the 2026-07-18 read. The blueprint uses the live
line. (All other R5 §2B engine cites — `field_frame.rs:{140,198,229,255}`, `motion.rs:50`,
`bridge.rs:121`, `money_guard.rs:60`, `zerocopy.rs:85` — matched live exactly.)

### 0.3 Correction B — `criterion` IS already a cached workspace dev-dep (the offline-clean tension is smaller than it looks)

The engine's "zero external crates" mandate is about the **default library build**. Dev-dependencies
are a *separate* build graph pulled only for `cargo bench`/`cargo test`, and **`criterion` is already
present and cached**: `kernel/Cargo.toml:138` `criterion = "0.5"`, wired as `kernel/benches/criterion.rs`
(`kernel/Cargo.toml:147` `[[bench]] name = "criterion"`), and `criterion` resolves in
`kernel/Cargo.lock`, `agent-loop/Cargo.lock`, `agent-adapters/Cargo.lock`, `llm-adapters/Cargo.lock`
(verified this pass). So adding `criterion` as an **engine dev-dependency** neither touches the
default lib build nor requires a network fetch. This turns §1.2's harness choice from a blocker into
a clean engineering decision (resolved in favor of criterion — see §1.2).

### 0.4 The gate this feeds (why P75 is a hard prerequisite)

`kernel/benches/bench_track.py` is a thin wrapper that delegates to `tools/telemetry/native-trackers`
`bench <crate> [--threshold N]`, parsing criterion output against a committed `baseline.json`
(default 10% regression threshold). S1 §3.1-A1 establishes this gate **cannot execute on a fresh
runner** (`native-trackers` never built there) — P75 fixes it and owns the replacement schema. P81's
baselines are meaningless until that gate runs; hence the hard dependency.

---

## 1. Prior-art map — adopt, don't invent (standard §2 item 19)

### 1.1 The two in-repo harness precedents

| Prior art | What it is | How P81 uses it |
|---|---|---|
| **`kernel/benches/criterion.rs`** (13 baseline entries, R5 §1a) | `criterion` `harness = false`; `BenchmarkGroup` + `bench_with_input` size-sweeps; deterministic seeded fixtures; auto-tracked by `bench_track.py` into `baseline.json` per `<group>/<n>` id | **The template P81 copies verbatim in structure.** Same group/sweep idiom, same `criterion_main!` registration, same deterministic-input rule. The engine harness is the kernel harness pointed at engine symbols. |
| **`bebop2/core/benches/verify_lane.rs`** (R5 §1d) | zero-dep `std::time` timing binary (a *measured number*, not a gate) | **Considered and NOT taken** (see §1.2) — it produces numbers but no `baseline.json` row, so it cannot feed P75's gate. Recorded so nobody "reconciles" P81 toward it later. |

### 1.2 Harness-shape decision (engineering, resolved here — operator need not rule)

**Decision: `criterion` dev-dep + `engine/benches/criterion.rs`, mirroring the kernel.** Rationale,
weighed honestly against the offline-clean mandate (§0.1, §0.3):

- criterion is already cached and is the *only* harness `bench_track.py`/P75's gate can parse; a
  hand-rolled `std::time` binary (the `verify_lane.rs` shape) would produce a number that **no gate
  consumes** — coverage without regression protection, which defeats the point (S1's whole Tier-A
  lesson is "a gate that carries no signal is worse than none").
- Dev-deps do not enter the default lib build, so the "zero external crates in the shipped engine"
  invariant is **preserved byte-for-byte** — verified by a DoD check (§DoD D-CLEAN) that
  `cargo tree -e no-dev -p dowiz-engine` still shows only `dowiz-kernel`.
- This is the same trade the kernel already made; P81 introduces no new pattern (item 19).

The one honest cost: `cargo bench -p dowiz-engine` now compiles criterion. That is acceptable — it is
opt-in, cached, and identical to the kernel's existing posture.

---

## 2. Scope — what P81 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P81 OWNS

1. **`engine/Cargo.toml`**: add `criterion = "0.5"` under `[dev-dependencies]` and a
   `[[bench]] name = "criterion" harness = false` stanza. Nothing else in the manifest changes.
2. **NEW `engine/benches/criterion.rs`**: the groups in §4, seeded-deterministic, `black_box`-guarded,
   registered in `criterion_main!`.
3. **The `<group>/<n>` bench-ids** for the engine surface, written into **P75's** baseline schema
   (never a new convention).
4. **A doc-comment revisit-threshold** on each sweep group (the growth-tripwire discipline S1 applies
   to `ppr`/`absorbing`/`money_ledger`): the size at which the curve would demand an algorithmic look.
5. **A NEW engine-crate CI bench job** (`bench-regression-engine`) in `.github/workflows/ci.yml`,
   modeled verbatim on P75's kernel `bench-regression` same-runner A/B job (P75 §5) but with
   `cd engine && cargo bench --bench criterion -- --save-baseline {base,pr}` as the A/B run and
   `native-trackers compare base pr --manifest engine/benches/baseline.json` (the per-crate v2
   manifest, P75 §3.2) as the gate step. Plus the committed `engine/benches/baseline.json` file itself.
   **This is new work P81 now owns explicitly — it is NOT something P75 already provides.** Ground
   truth (verified this pass): P75's *running* CI gate is **kernel-only** (P75 §5 runs
   `cd kernel && cargo bench`; P75 §2.2 states plainly *"the engine/bebop `[[bench]]` wiring for their
   own new targets — P81/P82 add those; P75 only fixes the kernel gate and defines the schema they
   conform to"*). So without this second job, P81's ~15 engine baselines are **recorded but never
   gated**, and D3 (below) is unsatisfiable. P81 **reuses** P75's `native-trackers compare` binary +
   `GateExit` exit-code contract + same-runner A/B shape **unchanged** — it adds only the second job
   pointing at the engine crate and its own `baseline.json`; it does **not** re-invent, fork, or widen
   the gate machinery (that stays P75's single-owner contract, §2.2). This item corrects
   `META-GAP-AUDIT-2026-07-19.md` finding **G3** ("no blueprint owns creating the engine-crate CI bench
   job"), rather than the prior silent assumption that P75's kernel gate would somehow run the engine.

### 2.2 P81 does NOT own (anti-scope — prevents collision & scope-creep)

- **Any product-source change to `engine/src/*`.** Zero. In particular the `apply_field` alloc-hoist
  (`bridge.rs:121`) and any `frame_rgba` alloc reuse are **out of scope** — bench-only, per the
  Standing Rule (§VERDICT.2). Opening those is a separate blueprint gated on P81's own numbers.
- **The GPU/WebGPU render surface** (`gpu`/`webgpu`/`webgl` features, all empty stubs today per
  `engine/Cargo.toml`). P81 benches the **CPU-authoritative** compute path (the engine's canonical
  authority; "scalar == SIMD bit-identical", GPU is a display sink). When P38 wires a real GPU sink,
  its bench is P38's, not P81's.
- **The kernel benches** (`kernel/benches/criterion.rs`) — that surface is **P80**'s. P81 touches only
  the engine crate; `csr::laplacian_spmv` (the kernel-side Laplacian the bridge calls into) is a P80
  target, not a P81 one. Collision-free by crate lane.
- **The bench-id/baseline schema + gate semantics** — **P75**'s single-owner contract.
- **`field_energy.rs`** — a `#[cfg(test)]` oracle, deliberately never a bench (R5 §2B).

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard input:** **P75** — but for the **`<group>/<n>` schema, the `baseline.json` v2 format, and the
reusable `native-trackers compare` gate binary + `GateExit` contract**, *not* for a ready-made engine
CI job. P75's running gate benches the kernel only (P75 §2.2/§5); the **engine `bench-regression-engine`
job is P81's own deliverable** (§2.1.5). Without both P75's schema/binary **and** P81's engine job, the
engine baselines are recorded but ungated. (This split corrects META-GAP-AUDIT G3, which flagged the
prior assumption that "the P75 gate" would run the engine crate.)
**Soft/none:** independent of P77/P79/P80/P82/P83 (disjoint files/crates — parallel-safe within Wave
W2 per S1 §5). **Substrate-for:** P87 (2-bit companion-mask) and P89 (field eigenmodes) name P81's
`field_frame`/`scene` benches as their measured before/after gate (MASTER-STATUS-LEDGER §1 P81 row) —
so P81 must land before those DoDs can be met.

### 2.4 Honest reconciliation with the physics-UI arc (standard §2 item 6)

P81 is pure instrumentation; it takes **no** position on the P86–P89 physics bets. It measures the
*current* CPU engine as-is. If P89 later replaces the diffusion step with a spectral/modal path, the
same bench-ids re-measure it — the harness is the neutral gate both sides of that bet report into
(P89's three-path table, MASTER-STATUS-LEDGER §4 row 14).

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

All live in the new bench file; no product types are added. Constants are named, never magic — the
sweep sets are the growth-curve schema.

```rust
// engine/benches/criterion.rs  (NEW)  — deterministic, seeded, no wall-clock, no RNG-from-entropy.

/// Grid sizes for the per-frame field sweep. Chosen to span a 16× area range so an O(w·h)
/// regression is visible as a slope, not a point (R5 §3: "the sweeps are the point").
const GRID_SWEEP: &[(usize, usize)] = &[(64, 64), (128, 128), (256, 256)];

/// The single "real-shape" grid for the point benches (frame_rgba/compose) — the size the
/// wasm rAF loop actually paints (`wasm/lib.rs`), so the pinned number is production-representative.
const GRID_ANCHOR: (usize, usize) = (128, 128);

/// compose() integration depth at the anchor grid — matches the FFI call site's typical steps.
const COMPOSE_STEPS: usize = 8;

/// SDF shape counts for the scene fold sweep — exposes the O(shapes) term at fixed resolution.
const SHAPE_SWEEP: &[usize] = &[1, 8, 32];

/// Spring angular-frequency sweep — exposes the ⌈ω·dt/0.1⌉ substep-loop count (motion.rs:50).
const OMEGA_SWEEP: &[f32] = &[1.0, 8.0, 32.0];
const SPRING_DT: f32 = 1.0 / 60.0; // one 60 Hz frame

/// Non-zero counts for the graph-Laplacian SpMV sweep (bridge.rs:121). n = node count;
/// nnz ≈ 2n for the field graph. Straddles small→large so the per-call alloc's share is visible.
const NNZ_SWEEP: &[usize] = &[256, 1024, 4096];

/// Deterministic fixture seed — every input buffer is filled from this LCG, never from entropy,
/// so the bench is byte-reproducible across runs/machines (the kernel-harness rule).
const FIXTURE_SEED: u64 = 0x5EED_F1E1_D0_00;
```

**Bench-id map (written into P75's `<group>/<n>` schema — P81 cites, never redefines):**

| group | ids | target (`engine/src/…`) |
|---|---|---|
| `field_step` | `/64x64`, `/128x128`, `/256x256` | `field_frame.rs:198` `FieldFrame::step` |
| `laplacian_into` | `/64x64`, `/128x128`, `/256x256` | `field_frame.rs:140` `laplacian_into` |
| `frame_rgba` | `/128x128` | `field_frame.rs:229` `FieldFrame::frame_rgba` |
| `compose` | `/128x128_s8` | `field_frame.rs:255` `compose` |
| `scene_render` | `/shapes_1`, `/shapes_8`, `/shapes_32` | `scene.rs:136` `Scene::render_frame` |
| `spring_step` | `/omega_1`, `/omega_8`, `/omega_32` | `motion.rs:50` `Spring::step` |
| `apply_field` | `/nnz_256`, `/nnz_1024`, `/nnz_4096` | `bridge.rs:121` `VertexBridge::apply_field` |
| `present_money` | `/pin` | `money_guard.rs:60` `TweenGuard::present_money` (RED-LINE) |

---

## 4. Build items — spec → RED check → code, each anti-cheat-guarded (standard §2 items 2, 3, 5)

Each bench group: **spec first, a RED check that fails before the bench exists, code, then GREEN.**
"RED" for a bench = *the id is absent from `baseline.json`* and *an injected slowdown does not trip
the gate*; "GREEN" = the id is measured and gates. The anti-cheat cases (item 5) are the heart — a
bench that does not actually exercise the hot path is the bench equivalent of a fake-green test.

### 4.1 M1 — `field_step` + `laplacian_into` grid sweep (the continuous per-frame core)

- **Spec:** `bench_with_input` over `GRID_SWEEP`; build a seeded `w·h` `source` buffer + a fixed
  `FieldEquilibrium`; time one `FieldFrame::step` (and, separately, one `laplacian_into`) per iter.
  `black_box` the input buffer and the mutated frame so LLVM cannot hoist/elide the stencil.
- **RED `red_field_step_absent`:** before the bench, `baseline.json` has no `field_step/*` id →
  P75's gate has nothing to compare → a 2× slowdown injected into `step` ships silently. This is the
  status-quo failure the bench closes.
- **GREEN:** three `field_step/*` + three `laplacian_into/*` baselines exist; the sweep shows the
  expected ~linear-in-area slope.
- **Anti-cheat `bench_actually_steps`:** assert (in a `#[test]` sibling, not the bench) that a single
  `step` on the fixture measurably changes the frame (‖after−before‖ > 0) — proving the benched call
  is real work, not a no-op the optimizer deleted. Mirrors the existing
  `step_reduces_magnitude_toward_source_equilibrium` test (`field_frame.rs:382`).
- **Revisit threshold (doc-comment):** *256×256 is the realistic ceiling for a mobile field-UI; if a
  consumer ever drives >512×512, revisit whether `step` needs tiling/SIMD — until then the scalar
  stencil is correct (matches the engine's CPU-authority mandate).*

### 4.2 M2 — `frame_rgba` + `compose` point benches (the paint + full pipeline)

- **Spec:** `frame_rgba` at `GRID_ANCHOR` (the size wasm actually paints); `compose` end-to-end at
  `GRID_ANCHOR` with `COMPOSE_STEPS`. `black_box` the returned `Vec<u8>`.
- **RED `red_frame_rgba_absent` / `red_compose_absent`:** the two ids are absent → the per-paint hue
  map + `Vec` alloc and the whole compose pipeline are ungated.
- **Anti-cheat `frame_rgba_len_is_wxhx4`:** assert the returned buffer length == `w*h*4` — a bench
  that measured a degenerate empty frame (length 0) would be a silent lie about the paint cost.
- **Honest note (recorded, not fixed):** `frame_rgba` allocates a fresh `Vec` each call (§0.1). The
  bench *exposes* the alloc share; whether to pool it is a follow-up gated on this number, not P81.

### 4.3 M3 — `scene_render` shape sweep (SDF fold)

- **Spec:** `bench_with_input` over `SHAPE_SWEEP` at `GRID_ANCHOR`; build a deterministic `Scene` of k
  SDF shapes from `FIXTURE_SEED`; time `Scene::render_frame`. `black_box` the `Vec<f32>` source.
- **RED `red_scene_render_absent`:** no `scene_render/*` id → the O(w·h·shapes) fold is ungated; a
  regression in the per-shape SDF cost is invisible.
- **Anti-cheat `scene_render_scales_with_shapes`:** assert the `shapes_32` mean is meaningfully above
  `shapes_1` (the fold is really O(shapes)); if they're equal, the shapes aren't being evaluated.
- **Revisit threshold:** *>32 concurrent SDF shapes at 128² would argue for a spatial cull; note only.*

### 4.4 M4 — `spring_step` ω sweep (motion substep loop)

- **Spec:** `bench_with_input` over `OMEGA_SWEEP` at `SPRING_DT`; time one `Spring::step`. The
  substep count is `⌈ω·dt/0.1⌉`, so higher ω = more inner iterations — the sweep exposes it.
- **RED `red_spring_step_absent`:** no `spring_step/*` id → the substep-loop cost is ungated.
- **Anti-cheat `spring_step_omega_monotone`:** assert `omega_32` ≥ `omega_1` cost (more substeps cost
  more) — proving the loop runs, not a constant-folded single step.

### 4.5 M5 — `apply_field` nnz sweep (graph-Laplacian SpMV + the alloc canary)

- **Spec:** `bench_with_input` over `NNZ_SWEEP`; build a seeded sparse field graph of n nodes
  (nnz ≈ 2n) and a seeded `x: Vec<f64>`; time one `VertexBridge::apply_field`. `black_box` the bridge
  and the field vector.
- **RED `red_apply_field_absent`:** no `apply_field/*` id → the per-frame SpMV **and its per-call
  `vec![0.; n]` alloc** (R5 §2B, `bridge.rs:121`) are ungated.
- **Anti-cheat `apply_field_touches_all_nnz`:** assert (sibling test) that `apply_field` changes every
  vertex reachable in the fixture graph — proving the SpMV visits the whole structure, not a prefix.
- **Alloc-canary note (doc-comment):** *this group is the tripwire for the `bridge.rs:121` alloc; if
  `apply_field/nnz_4096` shows the alloc as a material fraction, open a scoped alloc-hoist blueprint —
  do NOT hoist speculatively (Standing Rule; mirrors P80's contended-lock bench-first stance).*

### 4.6 M6 — `present_money/pin` (RED-LINE baseline pin, O(1))

- **Spec:** time one `TweenGuard::present_money(amount_minor)` at a fixed representative value.
- **RED `red_present_money_absent`:** no `present_money/pin` id → the money-never-tween guard has no
  perf pin.
- **Honest scope note:** this function is **O(1)** — the bench is a *presence + latency pin*, not a
  scaling curve. Its value is a canary: a change that made the RED-LINE guard disappear or balloon in
  cost shows up in the gate. The guard's *correctness* is owned by its own test
  (`present_money_rejects_fractional`, `money_guard.rs:102`); P81 only pins its cost baseline. Stated
  plainly so nobody reads more into an O(1) bench than is there.

---

## 5. DoD — falsifiable, RED→GREEN, machine-checkable (standard §2 item 2)

| # | Done when… | Falsifier (check) |
|---|---|---|
| D1 | `engine/benches/criterion.rs` exists; `cargo bench -p dowiz-engine` builds and runs all groups | the bench binary compiles under `harness = false`; run emits every `<group>/<n>` id in §3 |
| D2 | all §3 bench-ids are present in P75's `baseline.json` | `bench_track.py`/P75 gate lists them; a missing id fails the coverage assertion |
| D3 | an injected 2× slowdown in `FieldFrame::step` trips the **`bench-regression-engine`** CI job (the engine-crate gate P81 wires in §2.1.5, modeled on P75's job) RED, and clean HEAD → GREEN | inject `for _ in 0..2 { laplacian_into(...) }`, run the **engine** bench job → RED; revert → GREEN (proves the engine gate has real signal, not `\|\| true`). **NB (per META-GAP-AUDIT G3):** the CI gate P75 ships is dowiz-kernel-only, so this DoD is satisfiable **only because P81 now owns wiring the engine CI job** (§2.1.5) — it reuses P75's `native-trackers compare` + exit contract but does **not** assume P75's gate already benches the engine crate. This is also the evidence pointer Q1's benchmark-number claim-shape needs to discharge D3 (`BLUEPRINT-Q-SERIES-VERIFICATION-OBSERVABILITY-2026-07-19.md` Q1-b: "a `bench.jsonl` line … gated by the existing bench-regression CI job") — without `bench-regression-engine` that pointer would be permanently `NOT-MET`. |
| D4 | the default engine lib build stays zero-external-crate | **D-CLEAN:** `cargo tree -e no-dev -p dowiz-engine` shows only `dowiz-kernel` (no criterion leak) |
| D5 | every bench uses seeded deterministic inputs; no RNG-from-entropy, no wall-clock, no live daemon | grep the bench for `thread_rng`/`SystemTime`/network → empty; `FIXTURE_SEED` drives all buffers |
| D6 | anti-cheat sibling tests pass (each benched call does real work) | `bench_actually_steps`, `frame_rgba_len_is_wxhx4`, `scene_render_scales_with_shapes`, `spring_step_omega_monotone`, `apply_field_touches_all_nnz` all GREEN |
| D-NOREG | no `engine/src/*` product file changed; existing engine tests stay green | `git diff --stat engine/src` empty; `cargo test -p dowiz-engine` green |

---

## 6. Benchmarks + telemetry + the growth-curve gate (standard §2 item 10)

- **The benches ARE the deliverable** — there is no separate before/after number to report; P81
  *creates* the baseline the rest of the roadmap measures against. The "before" is *no coverage*; the
  "after" is a gated growth curve per surface.
- **Telemetry hook:** the baselines flow through `bench_track.py` → `tools/telemetry/native-trackers`
  → `baseline.json` (P75's fixed path), so a future regression surfaces in CI automatically, not at
  review time (item 10 + item 14). P81 adds no new telemetry mechanism — it feeds the existing one.
- **Scaling axis (item 8):** the bench inputs scale on **grid area (w·h)**, **SDF shape count**,
  **spring ω**, and **graph nnz** — the four independent cost axes of the engine. Each sweep states
  the size at which its curve would demand a code change (the revisit thresholds in §4). Nothing is
  presented as timeless.

---

## 7. Cross-cutting obligations (standard §2 items 6, 8, 9, 11–16, 20)

- **Hazard-safety (item 6):** the only hazard P81 could introduce is a **fake-green bench** (a bench
  that measures a no-op and reports "fast," hiding a real regression). Made unrepresentable by the §4
  anti-cheat sibling tests (each asserts the benched call did structural work) + `black_box` guards +
  the D3 injected-slowdown falsifier. A bench that lies fails an anti-cheat test at compile/CI time.
- **Isolation / bulkhead (item 11):** a bench binary is fully isolated — it links the engine as a dev
  target and cannot affect the shipped `cdylib`/`rlib` (D-CLEAN proves the default build is untouched).
  Its failure mode is "the bench doesn't build/run," never "the engine regresses."
- **Mesh awareness (item 12):** N/A, honestly — the engine is **node-local** render compute; it
  gossips nothing and touches no transport. Stated, not shoehorned.
- **Rollback/self-heal (item 13):** N/A as math — this is test scaffolding. Rollback = delete the
  bench file + revert the two `Cargo.toml` lines; there is no runtime state to heal.
- **Error-propagation / smart index (item 14):** the bug class P81 guards against — *a per-frame
  function silently getting slower* — becomes a **CI-time** failure via P75's gate on P81's baselines.
  The bug class P81 could *introduce* — a bench that stops exercising the real path — becomes a
  **test-time** failure via the anti-cheat siblings. Both are gated, not runtime surprises.
- **Living-memory awareness (item 15):** N/A — bench fixtures are ephemeral, seeded per run; nothing
  persisted.
- **Tensor/spectral (item 16):** the benched `laplacian_into`/`apply_field` **are** the engine's
  Laplacian-diffusion kernels; P81 measures the existing tensor/graph machinery (reuse, not
  re-derive). It adds no new spectral code — that would be P89.
- **Linux discipline (item 9):** **EXTENDS** the existing kernel criterion-harness pattern to a second
  crate; **REINFORCES** the deterministic-seeded-input + `harness = false` + `baseline.json`
  conventions; **DOES-NOT-TRANSFER** — no new tool, no new schema (P75 owns that).
- **Hermetic principles (item 20):** **Correspondence** — the bench-id is a faithful mirror of the
  function it measures ("as the function scales, so its baseline curve"); a bench that did not
  correspond to real work is exactly what the anti-cheat tests forbid. **Cause & Effect** — a
  regression has a measurable cause (a slope change on a named `<group>/<n>` id), never an
  unattributed "it feels slow."

---

## 8. Standard-compliance map (all 20 points — standard §2)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (every engine cite re-verified; the `scene.rs:122→136` drift + the criterion-already-cached correction) |
| 2 | Falsifiable DoD | §5 (D1–D-NOREG, incl. the D3 injected-slowdown gate proof) |
| 3 | Spec→check→code, event-ordered | §4 (spec-first per M; RED-before / GREEN-after per bench-id) |
| 4 | Predefined types & constants | §3 (sweep sets, seed, bench-id map named before code) |
| 5 | Adversarial / anti-cheat cases | §4 (per-M anti-cheat siblings), §7 (fake-green hazard) |
| 6 | Hazard-safety from structure | §7 (fake-green bench made unrepresentable by anti-cheat tests + black_box) |
| 7 | Links to docs & memory | §9 |
| 8 | Schemas with scaling axis | §3/§6 (grid area / shapes / ω / nnz; per-sweep revisit thresholds) |
| 9 | Linux engineering discipline | §7 (EXTENDS/REINFORCES/DOES-NOT-TRANSFER) |
| 10 | Benchmarks + telemetry | §6 (P81 *is* the baseline; feeds `bench_track.py`) |
| 11 | Isolation / bulkhead | §7 (dev-target isolation; D-CLEAN proves default build untouched) |
| 12 | Mesh awareness | §7 (N/A, node-local, stated) |
| 13 | Rollback/self-heal as math | §7 (N/A; rollback = delete file + 2 manifest lines) |
| 14 | Error-propagation / smart index | §7 (regression → CI-time gate; fake-green → test-time fail) |
| 15 | Living-memory awareness | §7 (ephemeral seeded fixtures) |
| 16 | Tensor/spectral where applicable | §7 (measures the existing Laplacian kernels; no new spectral code) |
| 17 | Regression tracking | §6 (baselines are the permanent gate; §9 REGRESSION-LEDGER note) |
| 18 | Clear worker instructions | §9 |
| 19 | Reuse-first, upgrade-if-needed | §1 (copies the kernel harness; criterion-over-std::time decided with reason) |
| 20 | Hermetic principles | §7 (Correspondence, Cause & Effect) |

---

## 9. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `docs/research/OPUS-PERF-BENCH-COVERAGE-MAP-2026-07-18.md` §1f (engine = zero benches), §2B (the
  engine hot-path table + the `bridge.rs:121` alloc finding), §3 Wave-1 engine harness sketch.
- `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.3-C2, §5 (Tier C, Wave W2, unit P81; "P75 hard").
- `MASTER-STATUS-LEDGER-2026-07-19.md` §1 (P81 = SKETCH-ONLY→this blueprint; substrate for P87/P89),
  §3 Wave-2 (P80 ∥ **P81** ∥ P83, all after P75 hard).
- **P75** (`BLUEPRINT-P75-*`, to be written) — owns the `<group>/<n>` bench-id + `baseline.json`
  schema + the working CI gate. **P81 cites it; never redefines it.**
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract).
- Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`.
- Memory: `performance-priority-over-minimal-change-2026-07-17.md` (scoped-perf directive),
  `verified-by-math-2026-07-07.md` (ship-RED, falsifiable-proof), `physics-ui-capture-quantum-math-arc-2026-07-14.md`
  (the engine's ONE-Laplacian design the benches measure).

**Existing code this blueprint edits/creates (exact targets, `dowiz` repo):**
- **NEW** `engine/benches/criterion.rs` — all §3/§4 groups, seeded, `black_box`-guarded,
  `criterion_main!`-registered.
- **EDIT** `engine/Cargo.toml` — add `criterion = "0.5"` to `[dev-dependencies]` + a
  `[[bench]] name = "criterion" harness = false` stanza. **No other manifest change; no `[dependencies]`
  change (offline-clean invariant).**
- **NEW** `engine/benches/baseline.json` — the per-crate v2 manifest (P75 §3.2) holding the §3 engine
  bench-ids; committed, so the engine gate has something to compare against.
- **EDIT** `.github/workflows/ci.yml` — add the **`bench-regression-engine`** job (§2.1.5): a second
  same-runner A/B job modeled on P75's kernel `bench-regression`, running the engine `cargo bench` and
  `native-trackers compare … --manifest engine/benches/baseline.json`. **Reuses P75's binary + exit
  contract; does not fork the gate machinery.** (Closes META-GAP-AUDIT G3 for the engine crate.)
- **DO NOT TOUCH** any `engine/src/*.rs` product file (bench-only; the `bridge.rs:121` alloc-hoist and
  `frame_rgba` pooling are explicitly deferred to evidence-gated follow-ups).

**For the worker with zero session context — exact acceptance path:**
1. **Confirm P75 has landed** (working gate + `<group>/<n>` schema). If not, STOP — P81's baselines
   are ungated until then; report the block.
2. Copy `kernel/benches/criterion.rs`'s structure; point the groups at the engine symbols in §3's map.
3. Seed every input from `FIXTURE_SEED`; `black_box` inputs and outputs; add the §4 anti-cheat sibling
   `#[test]`s.
4. Wire the two `engine/Cargo.toml` lines; run `cargo bench -p dowiz-engine` and confirm every
   `<group>/<n>` id emits; commit `engine/benches/baseline.json` with those ids.
5. **Wire the `bench-regression-engine` CI job** (§2.1.5) into `.github/workflows/ci.yml`, modeled on
   P75's kernel `bench-regression` job but pointed at the engine crate + `engine/benches/baseline.json`
   — this is P81's own deliverable, not something P75 provides (P75's gate is kernel-only). Then prove
   D3: inject a 2× slowdown into `FieldFrame::step`, run the **engine** job → RED; revert → GREEN.
6. Prove D-CLEAN: `cargo tree -e no-dev -p dowiz-engine` shows only `dowiz-kernel`.
7. Add a `docs/regressions/REGRESSION-LEDGER.md` row: "engine bench coverage established (P81);
   `field_step`/`compose`/`apply_field`/`present_money` now gated."
8. Anti-scope: do **not** fix the `bridge.rs:121` alloc or any engine source in this unit; do **not**
   invent a bench-id convention (cite P75's); do **not** add a `[dependencies]` entry.
