# RESEARCH-C — Living-Interface Arc: Real Build Status + Roadmap Wiring (2026-07-19)

> **What this is.** A ground-truth audit of the `living-interface-2026-07-16/` arc (the GPU
> neural-field / sonification / living-memory-viz work), verified against **live code in this
> worktree** (`research/equations-thermo-eigenvector-2026-07-19`, off `main`), not against the
> arc's own self-description. Prompted by the operator pasting the "Real-Time GPU Neural-Field
> Rendering + Signal Sonification" report — which is **byte-identical** to
> `docs/design/living-interface-2026-07-16/EXTERNAL-RESEARCH-gpu-neural-field-sonification.md`
> (saved 2026-07-16, already carried through a full synthesis → blueprint pass). This doc
> answers: is any of it **built** or all **blueprint**, is **W21/wgpu** still blocked, is the
> arc **wired into the master roadmap**, and what is the **smallest correct fix**.
>
> **Method:** Read-only (`Read`/`Grep`/`Glob`/`find`) — no git, no code edits. Every claim below
> carries a `file:line` or an explicit "searched X — 0 hits".

---

## Task 1 — Real current build status: BLUEPRINT-ONLY (arc), but two hard prerequisites landed in the KERNEL via sibling arcs

### 1a. The living-interface arc itself is fully unimplemented — confirmed

Not one deliverable of the arc (R-DEV / R-VENDOR / R-SON / R-LM / the engine field-UI GPU render
loop) exists as code. Evidence, each a real search:

| Expected artifact (per the blueprints) | Live search result | Verdict |
|---|---|---|
| `engine/src/gpu.rs` (W21 field-UI render loop) | `find … gpu.rs` → only `kernel/src/render/gpu.rs` (a **different** arc, §1b); **no `engine/src/gpu.rs`** | absent |
| Any `.wgsl` shader anywhere | `find … -name '*.wgsl'` (excl. node_modules/target) → **0** | absent |
| Engine `gpu` feature wired to a real sink | `engine/Cargo.toml:51` → `gpu = []` **EMPTY**; comment `:45-50` still says *"wgpu is absent from the cargo cache and every Cargo.lock (verified 2026-07-16)"*; `engine/src/bridge.rs:260` still returns `Err("gpu adapter not built — wgpu uncached")` | stubbed |
| AudioWorklet / audio DSP (R-SON) | grep `AudioWorklet\|audioWorklet\|AudioContext` over `*.rs/*.ts/*.js/*.mjs` (excl. node_modules/target/attic) → **0**; no `audio/` crate; no `karplus`/`bowed`/`faust` DSP (the two `karplus`-regex hits are false positives in `kernel/src/foodcourt.rs` + `typed_metrics.rs`, unrelated) | absent |
| `spectral_embedding.rs` / `coords_2d` / `coords_3d` helper (R-LM's "ONE net-new kernel primitive", FE-12) | `find … spectral_embedding.rs` → **0**; grep `coords_2d\|coords_3d\|spectral_embedding` → **0** as a helper | absent |
| New crates `brand-resolve/` (R-VENDOR P0-1), `field-math/` (RW-01), `audio/` (RW-09 3rd artifact) | `ls` → **none exist** | absent |
| Root `[workspace]` Cargo.toml (P02 §2.1 / consolidation §4) | no root `Cargo.toml`; each crate is still a peer directory | absent |

**Conclusion: the arc is 100% blueprint-only.** This matches the arc's own honest framing —
`G11-FAST-PATH-CONSOLIDATED.md:8` (*"this arc has not started implementation"*) and its §7 Q2 flag
that the arc's visible slice (Phase 7 sonification + Phase 8 memory-viz) is off the G11 critical
path (growth-substrate track, not customer-order path).

### 1b. BUT — two of the arc's hardest stated prerequisites have independently landed in the kernel (via other arcs, since 2026-07-16)

The blueprints were written 2026-07-16; the kernel has moved since. Two things the arc's docs treat
as *net-new work it must do* now **exist in `kernel/`**, delivered by sibling arcs:

**(i) Real headless wgpu bring-up — landed as DELIVERY P38, not the living-interface arc.**
`kernel/src/render/gpu.rs` (`kernel/src/render/mod.rs` gates it) is a live
`wgpu::Instance → Adapter → Device → Queue` init, `#[cfg(feature = "gpu")]`, degrade-not-crash typed
`GpuError`. Header verbatim (`render/gpu.rs:1`): *"P38 O18a — REAL minimal headless GPU bring-up …
This is NOT a stub."* Cargo wiring: `kernel/Cargo.toml:63` `gpu = ["dep:wgpu", "dep:pollster"]`,
`:142` `wgpu = { version = "30.0.0", optional = true }`, `:145` `pollster` optional. Its blueprint is
`docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P38-webgpu-render-engine.md` (dated 2026-07-18).
**Caveat:** this is *device bring-up only* — no render pipeline, no surface, **0 `.wgsl` shaders**, no
field-frame draw. It proves wgpu links and a GPU context can be created; it does not render anything.

**(ii) The eigenvector solve R-LM needs (FE-12) now exists in-kernel.** R-LM/P08 §1.4 stated the
kernel had **eigen*values* only** and that a Laplacian-eigenvector source was the *one net-new
primitive* the layout needed (to be vendored from bebop2 `field.rs`). That gap is now closed
natively: `kernel/src/householder.rs` computes eigen*vectors* (`eigh_contig`, sign-fixed basis,
`:378-476`); `kernel/src/spectral.rs:251` `eigh() -> Decomp`; `:269` `topk_symmetric() -> (basis,
values)` (sign-fixed, descending |λ|); `:424` `topk_symmetric_in` (arena twin). Per MEMORY this is
the **Eigenvector refactor Phase 28 rung-1, commit `03ac0fefe`, 2026-07-18**. **Only the thin
`coords_2d/coords_3d` embedding wrapper on top remains** — R-LM's hard dependency is now ~90% met
in-kernel and no longer needs the bebop2 vendor route the blueprint assumed.

**Net for Task 1:** the *arc* ships nothing, but its two most-cited blockers (wgpu-uncached;
missing eigenvector solve) have both dissolved underneath it via P38 O18a + the eigenvector refactor.
The blueprints' dependency/blocker framing is therefore **stale**.

---

## Task 2 — W21 / wgpu blocker: UNBLOCKED at the crate level (kernel), engine still carries the stale stub

**The premise of `BLUEPRINT-W21-field-ui-gpu-blocked.md` is now false.** W21's STATUS line
(`W21:3`) reads *"BLOCKED OFFLINE (wgpu uncached — verified 2026-07-16) … The `wgpu` crate is NOT
in the cargo cache and absent from every Cargo.lock."* Live check contradicts this:

- **wgpu IS now cached and locked:** `grep '^name = "wgpu"' **/Cargo.lock` → **FOUND in
  `kernel/Cargo.lock`**. `kernel/Cargo.toml:142` pins `wgpu = "30.0.0"`. The kernel `gpu` feature
  compiles a real bring-up (§1b). So the *"air-gapped, can't `cargo add wgpu`"* ceiling is broken —
  the one-time network fetch happened.
- **The engine (the arc's own W21 target) has NOT consumed it.** `engine/Cargo.toml:51` is still
  `gpu = []` EMPTY, its comment still asserts *"wgpu absent from every Cargo.lock (verified
  2026-07-16)"*, and `engine/src/bridge.rs:248-260` still returns the honest
  `Err("gpu adapter not built — wgpu uncached")`. The living-interface field-UI render loop
  (`engine/src/gpu.rs`, FE-04/05/08–17) remains 0% — the unblock landed in a *different* crate.

**R-DEV's framing is confirmed accurate and now realized.** `R-DEV-…md §2.1/§6` and
`LIVING-INTERFACE-ROADMAP.md` §"R-DEV" both argue the *real* W21 blocker was *"one-time `cargo add
wgpu`… a network/operator-gate, NOT a technical impossibility"* (W21 conflated "wgpu uncached" with
"no GPU"; software-Vulkan/Lavapipe was always viable once the crate is present). Reading those files
directly: the claim holds verbatim, and it has now come true — the operator's one-time network grant
(the **O18a graphics-unlock**, see Task 5) was given, wgpu 30 is fetched, and the kernel proves it
builds headless. **W21 is no longer a hard offline ceiling; it is a "consume the now-available crate
in `engine/`" task.**

**Staleness to flag:** both `W21:3` and `MASTER-ROADMAP …:584` (the P38 row: *"GPU path 0%… O18a
graphics-unlock (hard, environment-gated)"*) still describe the gate as **unmet**. They predate the
kernel wgpu landing (fresh, ~2026-07-18/19) and are now stale on the crate-availability fact.

---

## Task 3 — Roadmap-index disconnection: CONFIRMED (with one refinement)

Direct grep of the two target index docs (not trusting the prior shallow finding):

**`docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`** — searched
`living-interface | living interface | R-LM | R-SON | neural-field | sonification | memory-viz`:
- The arc name and its growth deliverables are **ABSENT**. The only `P07`/`P08` hits (`:117`, `:118`)
  are the **sovereign roadmap's own** Phase 7 *"Money-Law Closure"* and Phase 8 *"Typed Local
  Observability"* — unrelated to the living-interface P07-sonification/P08-viz blueprints.
- **Refinement (important):** the arc's *engine substrate* **is** wired in. The FE/RW/DZ arcs that
  the living-interface roadmap extends are absorbed into **DELIVERY P38a/P38b** (`…:964-966`), with
  **FE-12 "spectral φ₂φ₃ embedding" explicitly absorbed** (`:966`) and DZ-01..12 → P38b. So the
  render engine is registered as P38; it is specifically the **living-interface *name* + its four
  new designs (R-LM viz, R-SON sonification, R-DEV dev/CI, R-VENDOR brand) + the two off-path P07/P08
  blueprints** that never appear.

**`docs/design/GROUND-TRUTH-2026-07-17.md`** — searched the same terms **plus `wgpu`**: **0 hits of
any kind.** It is a 60-line `git`-state snapshot (main=`9f78b91d5`, feat/* branch inventory) and
mentions neither the living-interface arc, nor P38, nor wgpu, nor the eigenvector work.

**Where the arc *is* referenced:** only `CORE-ROADMAP-INDEX.md:63` (Layer G) and `:129` (an "easy to
overlook" callout) — matching the prior finding — **and** today's
`ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md:84` which already lists *"Living Interface (feeds
P16/Layer G) | …LIVING-INTERFACE-ROADMAP.md (+ BLUEPRINT-P07/P08 sonification/viz) | ✅"* and maps
FE/DZ/RW → P38 (`:93-95`). So at the *index* level the gap-audit already considers it "covered" via
CORE-ROADMAP-INDEX + P38 — but the two **primary** roadmap docs the operator reads
(MASTER-ROADMAP and GROUND-TRUTH) still carry **no trace of it**.

---

## Task 4 — Concrete minimal wiring fix (proposal only — not applied here)

The arc is well-designed; do **not** redesign it. Two surgical, append-only edits register its
existence + current status + real (now-dissolved) dependency, matching the terse style each target
doc already uses. Written ready-to-apply for a downstream pass.

### Edit A — MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md

Append a new `## 20.` status-ledger section (same append-only pattern as the existing `## 19. Perf,
physics and mesh research wave — status-ledger registration (2026-07-19)`). Draft prose:

> ## 20. Living-interface arc — status-ledger registration (2026-07-19)
>
> Appended after §19, same append-only rule. Registers the `docs/design/living-interface-2026-07-16/`
> arc (GPU neural-field render + sonification + 3-D living-memory viz + GPU-less dev/CI + brand→GPU
> pipeline). **Source of truth (do not re-derive): `living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md`**
> (12-phase plan) + its six execution blueprints (`BLUEPRINT-P00/P01/P02/P06/P07/P08`,
> `G11-FAST-PATH-CONSOLIDATED.md`). Already index-rowed in
> [CORE-ROADMAP-INDEX.md](CORE-ROADMAP-INDEX.md) §Layer-G and
> [ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md](ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md); this row
> closes its absence from *this* roadmap and from GROUND-TRUTH.
>
> - **Relationship to phases:** the arc's *engine substrate* (FE-01..17 / RW-01..12 / DZ-01..12) is
>   **already owned by DELIVERY P38a/P38b** (§10.5.3) — this arc adds nothing to that. Its four
>   *new* designs are: **R-VENDOR** (brand→GPU token pipeline; folds into P38a's FE-05 table),
>   **R-DEV** (Lavapipe GPU-less CI + the CSP `'wasm-unsafe-eval'` fix), and the two
>   **off-G11-critical-path growth-substrate deliverables** — **R-SON sonification** (`BLUEPRINT-P07`,
>   a Rust/wasm DSP AudioWorklet, "never load-bearing") and **R-LM living-memory viz**
>   (`BLUEPRINT-P08`, owner-only 3-D diagnostic of the hub's own memory graph). Per the arc's own §8
>   operator ruling, P07/P08 stay **off** the first-real-order path.
> - **Status: BLUEPRINT-ONLY (0% code).** Verified 2026-07-19: no `engine/src/gpu.rs`, no `.wgsl`,
>   no AudioWorklet/`audio` crate, no `spectral_embedding` coords helper, no `brand-resolve`/
>   `field-math` crate, no root workspace.
> - **Two blockers the blueprints cite have since DISSOLVED (register, don't re-plan):** (1) the
>   **W21 wgpu-uncached offline ceiling is broken** — the O18a graphics-unlock was granted; `wgpu
>   30.0.0` is now in `kernel/Cargo.lock` and `kernel/src/render/gpu.rs` is a real headless
>   `Instance→Device` bring-up (P38 O18a). The **engine** still carries the stale empty `gpu = []`
>   stub, so W21 is now "consume the available crate in `engine/`", not "wait for network". (2) the
>   **eigenvector solve R-LM/FE-12 needed now exists in-kernel** (`spectral::eigh`/`topk_symmetric`,
>   commit `03ac0fefe`, Phase-28) — only the thin `coords_2d/coords_3d` wrapper remains.

*(If a single line is preferred over a section, add one row to the §10.2 P31–P53 index or a footnote
on the P38 row (`…:584`) pointing to `LIVING-INTERFACE-ROADMAP.md` and stating "blueprint-only;
P07-son/P08-viz off G11 path; wgpu-uncached blocker dissolved via O18a — see §20".)*

### Edit B — GROUND-TRUTH-2026-07-17.md

Add one bullet to the "Next waves" list (or a "Corrections/updates" note), since this doc currently
has **zero** GPU/wgpu content and is the operator's live state snapshot:

> - **Graphics/GPU state (added 2026-07-19):** the **O18a graphics-unlock** landed — `wgpu 30.0.0`
>   is now cached and in `kernel/Cargo.lock`; `kernel/src/render/gpu.rs` builds a real headless wgpu
>   context under `feature="gpu"` (P38). The eigenvector solve landed too (`spectral::eigh`/
>   `topk_symmetric`, `03ac0fefe`). **Still blueprint-only:** the `engine/` field-UI GPU render loop
>   (engine `gpu = []` remains empty) and the entire `living-interface-2026-07-16/` arc (R-SON audio,
>   R-LM viz, R-VENDOR brand→GPU). W21's "wgpu uncached" premise is obsolete.

Both edits are pure registration — no scope, no new phase, no code. A downstream blueprint pass owns
any actual sequencing.

---

## Task 5 — The wgpu network-gate vs P06 (report only, no speculation)

**The wgpu network-gate is the "O18a graphics-unlock", a fully separate operator decision from P06.**
Evidence in code/docs:
- `kernel/Cargo.toml:57` labels the `gpu` feature *"P38 O18a graphics unlock"*; `kernel/src/render/gpu.rs:1`
  and `render/mod.rs:1` both say *"P38 O18a"*.
- `BLUEPRINT-P38-webgpu-render-engine.md` (its §0 ground-truth table) describes the *"O18a
  `graphics-unlock` gate … hard, environment-gated, ONE-TIME, shared with P17"* — the trigger is
  *"operator grants network `cargo add wgpu`"* (`W21:26`). **Nothing anywhere ties O18a to P06.**
- **P06** (per MEMORY) is the *key_V HybridSigner signed done-gate*, closed 2026-07-18 (`58987d79d`) —
  a crypto/mesh-identity item, unrelated to graphics.

**What the artifacts indicate:** independently of P06, the O18a graphics-unlock has **evidently also
been granted** — because `wgpu 30.0.0` is now present in `kernel/Cargo.lock` and the kernel builds a
live wgpu context, which the roadmap/W21 still describe as *pending*. So P06 (crypto) and O18a
(graphics network-gate) are two distinct operator decisions; the graphics gate has moved to
"granted/landed" in the kernel. I report only what the code and docs show; I do not infer operator
intent beyond that.

---

## Source ledger (all verified live this pass, 2026-07-19)

- **Arc docs (read in full):** `living-interface-2026-07-16/` — `EXTERNAL-RESEARCH-gpu-neural-field-sonification.md`,
  `R-LM-…`, `R-SON-…`, `R-DEV-…`, `R-VENDOR-…`, `LIVING-INTERFACE-ROADMAP.md`,
  `BLUEPRINT-P07-sonification-phase0.md`, `BLUEPRINT-P08-living-memory-viz-phase0.md`,
  `G11-FAST-PATH-2Q-AUDIT.md`, `G11-FAST-PATH-CONSOLIDATED.md`; companions
  `BLUEPRINT-W21-field-ui-gpu-blocked.md`, `CORE-ROADMAP-2026-07-17/BLUEPRINT-P38-webgpu-render-engine.md`.
- **Live code (build status):** `engine/Cargo.toml:45-51` (`gpu = []` empty), `engine/src/bridge.rs:248-260`
  (honest stub), `kernel/Cargo.toml:57-63,142-145` (`gpu`+wgpu 30), `kernel/src/render/{mod,gpu}.rs`
  (real headless bring-up), `kernel/src/spectral.rs:251,269,424` + `householder.rs:378-476` (eigenvector
  solve), `kernel/Cargo.lock` (wgpu present). Searches: `.wgsl`=0, `AudioWorklet`=0, `spectral_embedding`/
  `coords_3d` helper=0, `brand-resolve`/`field-math`/`audio` crates=0, root workspace=0.
- **Index docs (roadmap wiring):** `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` (arc absent;
  P38a/P38b at `:964-966`, FE-12 at `:966`, P38 row `:584`, §19 registration pattern `:2795`),
  `GROUND-TRUTH-2026-07-17.md` (0 hits, 60 lines), `CORE-ROADMAP-INDEX.md:63,129`,
  `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md:84,93-95`.

*End RESEARCH-C. Read-only audit — no product code, no roadmap doc, and no git state edited; the
Task-4 edits are proposals for a downstream pass.*
