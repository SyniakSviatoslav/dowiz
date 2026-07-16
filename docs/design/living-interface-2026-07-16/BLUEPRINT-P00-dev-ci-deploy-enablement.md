# BLUEPRINT P00 — Dev/CI + Deploy Enablement (Living-Interface Phase 0)

> **Scope:** the execution-ready blueprint for **Phase 0** of `LIVING-INTERFACE-ROADMAP.md` (§4 row 0).
> Turns `R-DEV-gpu-less-dev-ci-strategy.md` Phase-0 recommendations 1–4 into concrete file targets,
> commands, and migration steps. **Planning only — this document edits no code, CI, or config.** It is the
> Wave-0 enablement layer: it depends on nothing, is parallel-safe with Phase 1, and unblocks *every*
> wasm-shipping and GPU-rendering phase downstream (J6 in the roadmap risk-map).
>
> **Sources (both read in full):** `LIVING-INTERFACE-ROADMAP.md` §1 (prereq map), §2 (Decision A/B),
> §4 (Phase-0 row), §5 J5/J6; `R-DEV-gpu-less-dev-ci-strategy.md` §1–§6. Every current-state claim below
> was re-verified live against the working tree on 2026-07-16 (citations in §1), not paraphrased from R-DEV.

---

## 1. Current-state evidence (verified live, 2026-07-16)

Three independent RED states must exist for Phase 0 to be well-founded. All three verified against HEAD:

### 1.1 wgpu dependency state — RED (uncached, empty feature, zero shaders)
- **`engine/Cargo.toml:22-31`** — `[features] default = []` and `gpu = []` (empty). The only dependency is
  `dowiz-kernel = { path = "../kernel" }` (line 20). The comment at 27-31 states verbatim that `wgpu` "is
  absent from the cargo cache and every Cargo.lock (verified 2026-07-16)" and that the `gpu` feature stays
  empty until `cargo add wgpu` succeeds over the network.
- **`engine/src/bridge.rs:200-242`** — `#[cfg(feature = "gpu")] pub mod gpu` exists as an honest boundary.
  `new_gpu(...)` (line 220) returns `Err("gpu adapter not built — wgpu uncached")` (line 227);
  `upload_to_gpu(...)` (line 233) returns the same error (line 240). The gated test
  `new_gpu_returns_honest_err_when_wgpu_uncached` (line 352) asserts that error — so it will need updating
  once the real adapter lands (that is Phase 2 / FE-01 work, **not** Phase 0; Phase 0 only makes the crate
  fetchable).
- **`find … -name '*.wgsl'` = 0 hits** (live). **`grep 'name = "wgpu"'` across every `Cargo.lock` = 0 hits**
  (live). Confirms R-DEV §1.1: zero WGSL, wgpu in no lockfile.
- **Layout note (load-bearing for the build command):** `engine/` is a **standalone crate, not a workspace
  member** — no root `Cargo.toml` with `[workspace]` exists (live: the only Cargo manifests are
  `engine/`, `kernel/`, `wasm/`, `agent-governance-wasm/`, and four under `tools/`, none declaring a
  workspace). Therefore `cargo build -p dowiz-engine` only resolves when cwd is `engine/` or via
  `--manifest-path engine/Cargo.toml`. The done-test command must run from `engine/`.

### 1.2 CSP gap — RED (latent production bug today)
- **`tools/native-spa-server/src/lib.rs:36-48`** — the `SECURITY_HEADERS` const. Line 39 CSP value:
  `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:;
  font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self';
  object-src 'none'; upgrade-insecure-requests`. **`script-src 'self'` has no `'wasm-unsafe-eval'`; there
  is no `Cross-Origin-Opener-Policy` and no `Cross-Origin-Embedder-Policy`.** This const is applied to
  every response by the `security_headers` middleware (lib.rs:51-58) and by `health_response()`
  (lib.rs:115-122).
- **`docker/nginx-default.conf:23`** — the byte-identical CSP source-of-truth the axum const mirrors
  ("ported 1:1", per lib.rs:8 and 31). Any Phase-0 edit must touch **both** so they stay in lockstep.
- **`tools/native-spa-server/tests/integration.rs:209-215`** — test `r4_security_headers_present_and_exact`
  asserts the CSP string with `assert_eq!` **exactly**, across three paths (`/`, `/assets/app.js`,
  `/deep/nested/route`). This is the RED lock: any change to the header string turns this test red until the
  golden literal at line 213 is updated in the same commit.
- **Why this is a bug now, not a new feature (roadmap §2 Decision A / R-DEV §5.3.2):** Chromium gates
  `WebAssembly.compile`/`instantiate` behind `'wasm-unsafe-eval'` in `script-src`. The existing kernel wasm
  loads only because the *dev* server (`web/serve.mjs`) sends no CSP at all. Behind this production CSP the
  current kernel wasm — and every wasm this arc ships (wgpu-wasm engine, R-SON audio DSP, RW-05 shell) —
  throws a CSP `CompileError`. The fix is one token; the downside for a `default-src 'self'` same-origin app
  is nil.

### 1.3 CI state — RED (no GPU/wgpu/shader job exists)
- **`.github/workflows/ci.yml`** — two jobs only: `telemetry-selftest` (bash) and `eqc-proofs`
  (sympy→emit-Rust→`rustc -O`→run). No GPU, no wgpu, no browser, no `engine/` build. Triggers on push to
  `main` + `feat/kernel-fsm-graph-analysis` and PR to `main`.
- **`.github/workflows/visual.yml`** — a stale Playwright DOM-screenshot suite (`timeout-minutes: 25`)
  targeting **untracked** `apps/web/**` (R-DEV §1.2: vestigial). It is **not** a wgpu harness, but it is the
  in-repo **precedent** for the two mechanics Phase 0 reuses: a `paths:` trigger filter (visual.yml:7-13)
  and `concurrency: cancel-in-progress` (16-18).
- **No `wgpu`/`lavapipe`/`mesa`/`gpu-smoke` workflow exists** (live grep across `.github/` = 0). Phase 0
  therefore **adds a new workflow file**, it does not extend an existing one.

---

## 2. Recommendation 1 — one-time `cargo add wgpu` + feature-flag plan

The real W21 blocker is **package availability, not GPU absence** (R-DEV §2.1). Fix it once, online.

**Migration steps (in order):**
1. **Operator-gated network fetch** (the documented trigger already in `BLUEPRINT-W21`): from `engine/`, run
   `cargo add wgpu` (pin an explicit version, e.g. `cargo add wgpu@22` — pin the current stable to keep the
   lockfile reproducible and the Lavapipe apt/Mesa pairing predictable). This is the **only** step that
   needs the network; everything downstream is offline-clean again.
2. **Gate wgpu behind the existing `gpu` feature, never `default`.** In `engine/Cargo.toml`, add wgpu as an
   **optional** dependency and make the `gpu` feature enable it:
   ```toml
   [dependencies]
   dowiz-kernel = { path = "../kernel" }
   wgpu = { version = "22", optional = true }   # display sink only; opt-in

   [features]
   default = []                # unchanged — offline-clean default build stays zero-external-dep
   gpu = ["dep:wgpu"]          # was `gpu = []`; now pulls wgpu ONLY under --features gpu
   ```
   This preserves the invariant in `engine/Cargo.toml:6-8` and `bridge.rs:200-208`: the **default build has
   zero external crates**, so `cargo test` (the kernel/engine offline gate) never pulls a GPU toolchain. Only
   `--features gpu` links wgpu.
3. **Leave `bridge.rs::gpu::new_gpu` as the honest stub for Phase 0.** Replacing the `Err("gpu adapter not
   built — wgpu uncached")` stub with a real `wgpu::Device`/`Queue` sink is **FE-01/U1 = Phase 2**, not
   Phase 0. Phase 0's contract is narrower: prove wgpu *links* and the Lavapipe *harness* runs. Do not scope
   the real sink into this phase.
4. **Correct the stale non-goal.** `BLUEPRINT-W21`'s line *"No software-raster GPU emulation (impossible
   without a GPU binding crate)"* is factually wrong (wgpu *is* the binding crate and runs on Lavapipe). Mark
   it superseded by this blueprint. (Doc edit, left for the implementation pass — not code.)

**Falsifiable done-test (1):** from `engine/`, `cargo build -p dowiz-engine --features gpu` links wgpu and
compiles. RED today (uncached ⇒ resolution error); GREEN after steps 1–2.

---

## 3. Recommendation 2 — the Mesa Lavapipe software-Vulkan GPU-smoke CI job

**New file: `.github/workflows/wgpu-smoke.yml`** (a sibling of `ci.yml`, *not* a job inside it — it needs its
own `paths:` gate and apt install, and must stay off the common-case critical path). This is a proven
pattern, not novel: wgpu's own CI runs on Lavapipe/llvmpipe + D3D12-WARP (R-DEV §2.2).

**Job structure (concrete):**
```yaml
name: wgpu GPU-smoke (Lavapipe)

on:
  pull_request:
    paths:                          # mirror visual.yml's paths-gate: common case pays nothing
      - 'engine/**'
      - '**/*.wgsl'
      - '.github/workflows/wgpu-smoke.yml'
  push:
    branches: [ "main", "feat/kernel-fsm-graph-analysis" ]
    paths:
      - 'engine/**'
      - '**/*.wgsl'

concurrency:                        # newer push cancels in-flight (from visual.yml precedent)
  group: wgpu-smoke-${{ github.ref }}
  cancel-in-progress: true

jobs:
  gpu-smoke:
    runs-on: ubuntu-latest          # GPU-LESS runner; Lavapipe is CPU-side
    timeout-minutes: 10             # HARD ceiling (roadmap done-test); Lavapipe-hang mitigation
    steps:
      - uses: actions/checkout@v4

      - name: Install software Vulkan (Lavapipe) + software GL (llvmpipe)
        run: |
          sudo apt-get update
          sudo apt-get install -y mesa-vulkan-drivers libgl1-mesa-dri \
            libegl1-mesa-dev libxcb-xfixes0-dev
          # provides /usr/share/vulkan/icd.d/lvp_icd.x86_64.json

      - uses: dtolnay/rust-toolchain@stable

      - name: Build engine with the gpu feature (done-test 1)
        working-directory: engine
        run: cargo build -p dowiz-engine --features gpu

      - name: WGSL compile-check + one offscreen field-frame (done-test 2)
        working-directory: engine
        env:
          VK_DRIVER_FILES: /usr/share/vulkan/icd.d/lvp_icd.x86_64.json  # newer loaders
          VK_ICD_FILENAMES: /usr/share/vulkan/icd.d/lvp_icd.x86_64.json # older loaders (alias)
          LIBGL_ALWAYS_SOFTWARE: '1'
          WGPU_ADAPTER_NAME: llvmpipe        # force the CPU adapter deterministically
          WGPU_BACKEND: vulkan
        run: cargo test -p dowiz-engine --features gpu --test gpu_smoke -- --nocapture
```

**The test binary it drives: new `engine/tests/gpu_smoke.rs`** (gated `#![cfg(feature = "gpu")]`), which does
exactly the four things R-DEV §2.4 enumerates:
1. Request an adapter with `power_preference: LowPower` + `force_fallback_adapter: true` (accept
   `AdapterType::Cpu`) so wgpu binds the Lavapipe ICD — no window/surface needed.
2. **`create_shader_module` compile-check over every `.wgsl`** discovered under the repo (glob `**/*.wgsl`).
   This is the cheap first gate; it fails fast on a bad shader without rendering. **Today the glob is empty
   (§1.1), so the loop is a no-op that becomes load-bearing the moment Phase 2/3 add shaders** — wiring it
   now means no shader ever lands without a compile gate.
3. Render **one offscreen frame** into a `wgpu::Texture` render target (a minimal clear + a single
   fullscreen-triangle pass through a trivial embedded WGSL is sufficient in Phase 0 — it proves device
   acquisition + `copyTextureToBuffer` + `mapAsync` readback works under Lavapipe end-to-end). The **real
   field-frame** golden is locked the instant FE-01's `VertexBridge → vertex buffer → pipeline` exists
   (Phase 2); the harness slot is reserved here so Phase 2 fills it in, not re-scaffolds it.
4. Read pixels back and assert a **deterministic pixel-hash / SSIM-vs-golden** — never a bit-exact hash
   *across rasterizers* (J5 / R-DEV §5.1: Lavapipe f32 rounding and `fwidth` differ from real GPUs; goldens
   are locked on the operator's real GPU and compared with an SSIM tolerance).

**Cost / hang mitigations baked in (R-DEV §5.2 / J5):** tiny frames only (e.g. 512×512, low N), pinned Mesa
via the apt install, `timeout-minutes: 10` hard ceiling, per-test timeout so a Lavapipe hang (gfx-rs/wgpu
#1974) fails fast instead of burning wall-clock. CI proves **logic and still-frame correctness, not fps** —
fps is deferred to Recommendation 3's manual device pass.

**Falsifiable done-test (2):** the `wgpu-smoke` job goes GREEN — `create_shader_module` passes over every
`.wgsl` and the offscreen render→readback→hash matches golden — and is gated on `engine/**` + `**/*.wgsl`,
`timeout-minutes: 10`.

---

## 4. Recommendation 3 — the local visual loop (operator's own GPU)

No new infra, no code. The GPU-less Hetzner box **serves**; a GPU-having device **renders** (R-DEV §3.1/§3.3).

- The developer serves the static field-UI build from the existing static server (`web/serve.mjs`, or the
  `native-spa-server` axum binary) on the box; the **operator opens it in a WebGPU browser on their own
  laptop/phone** (`http://<box-ip>:4173`, or the Cloudflare tunnel already used for `webhook.dowiz.org`).
- **One manual real-device pass per shader-touching change** is the honest Phase-0 real-GPU gate (J5 mitigation
  4). **Do NOT** over-scope this blueprint into automated real-GPU CI — GPU-runner Playwright/xvfb and
  BrowserStack/LambdaTest are explicitly **deferred to Phase 10** (roadmap §4 row 10; R-DEV §4.2). Phase 0
  documents the loop; it automates nothing here.

There is **no acceptance test** for this recommendation beyond "the operator can load the served build on a
real device and see the frame" — it is a workflow, not a gate, by deliberate design.

---

## 5. Recommendation 4 — the CSP header fix (Decision A only)

**Critical scope split (roadmap §2):** R-DEV rec 4 bundles three header edits (`'wasm-unsafe-eval'` + COOP +
COEP). The roadmap **splits them**: only **`'wasm-unsafe-eval'` (Decision A) ships in Phase 0**; **COOP/COEP
(Decision B) is deferred to Phase 10** behind a MapLibre/R2 CORP-proxy migration (enabling `COEP:
require-corp` now would break cross-origin map tiles and R2 photos). **This blueprint adds exactly one CSP
token and no COOP/COEP header.** Anyone bundling them is forced into a false binary.

**Exact diff — one token, three synchronized edit sites (all in the same commit so RED→GREEN):**

1. **`tools/native-spa-server/src/lib.rs:39`** (the `SECURITY_HEADERS` const):
   ```
   - "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; …"
   + "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; …"
   ```
   (`… ` = the remainder of the CSP unchanged: `img-src 'self' data: https:; font-src 'self'; connect-src
   'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'; object-src 'none';
   upgrade-insecure-requests`.)

2. **`docker/nginx-default.conf:23`** — the same `script-src 'self'` → `script-src 'self'
   'wasm-unsafe-eval'` edit, keeping the axum const's "ported 1:1" invariant intact.

3. **`tools/native-spa-server/tests/integration.rs:213`** — update the `assert_eq!` golden literal to the new
   CSP string. This is mandatory: the test is the RED lock (§1.2). Editing lib.rs without editing the test
   turns `r4_security_headers_present_and_exact` red; editing both in one commit is the RED→GREEN transition.

**Migration order:** edit integration.rs:213 golden → edit lib.rs:39 → edit nginx-default.conf:23 → run the
test (§6 test 3a). All three in one commit.

**Explicitly NOT edited in Phase 0:** no `Cross-Origin-Opener-Policy`, no `Cross-Origin-Embedder-Policy` line
added anywhere. Those are Phase 10.

**Falsifiable done-test (3):** loading a build carrying the **production** header config in Chrome throws
**no** CSP wasm `CompileError` when instantiating the kernel wasm. RED before the edit (the current
`script-src 'self'` blocks `WebAssembly.instantiate`); GREEN after.

---

## 6. Acceptance criteria — numbered checklist (matches roadmap §4 row-0 done-tests)

**AC-1 — wgpu links (done-test 1).**
`cd engine && cargo build -p dowiz-engine --features gpu` exits 0 and the build graph contains `wgpu`.
- Verify: the command exits 0; `cargo tree -p dowiz-engine --features gpu | grep -q '^.*wgpu'` succeeds.
- Guard: `cargo build -p dowiz-engine` (no `--features gpu`) still pulls **zero** external crates
  (`cargo tree -p dowiz-engine` shows only `dowiz-kernel`) — the offline-clean default is preserved.
- RED baseline (today): resolution fails — wgpu uncached, `gpu = []` empty.

**AC-2 — Lavapipe headless smoke (done-test 2).**
The `.github/workflows/wgpu-smoke.yml` `gpu-smoke` job is GREEN on a GPU-less runner.
- 2a: `create_shader_module` compile-check runs over every `**/*.wgsl` (0 today; loop wired and passing).
- 2b: one offscreen frame renders under the Lavapipe adapter, is read back, and its pixel-hash/SSIM matches
  the locked golden (golden locked on the operator's real GPU, compared with tolerance — never bit-exact
  cross-rasterizer).
- 2c: the job is `paths:`-gated on `engine/**` + `**/*.wgsl` and carries `timeout-minutes: 10`.
- Verify locally: `VK_DRIVER_FILES=…/lvp_icd.x86_64.json LIBGL_ALWAYS_SOFTWARE=1 WGPU_ADAPTER_NAME=llvmpipe
  cargo test -p dowiz-engine --features gpu --test gpu_smoke` exits 0.
- RED baseline (today): no such workflow/test exists; no adapter would bind.

**AC-3 — CSP no longer blocks wasm (done-test 3).**
A build served with the production `SECURITY_HEADERS` instantiates the kernel wasm in Chrome with **no** CSP
`CompileError` in the console.
- 3a: `cargo test -p native-spa-server --test integration r4_security_headers_present_and_exact` passes with
  the new golden (proves the header the server actually sends now contains `'wasm-unsafe-eval'`).
- 3b: manual — serve the build, open Chrome DevTools console, confirm the kernel wasm loads with no
  `Content Security Policy … 'wasm-unsafe-eval'` violation (R-DEV §5.3.2 verify step).
- RED baseline (today): `script-src 'self'` alone ⇒ `WebAssembly.instantiate` throws a CSP `CompileError`.

**AC-4 — scope discipline (guards against over-build).**
- No `Cross-Origin-Opener-Policy` / `Cross-Origin-Embedder-Policy` header added (Decision B is Phase 10).
- No real `wgpu::Device` sink in `bridge.rs` (that is FE-01/Phase 2); `new_gpu` still returns its honest stub.
- No GPU-runner Playwright / device-cloud automation added (Phase 10).

---

## 7. What this unblocks (roadmap J6 — widest blast radius)

Phase 0 is Wave-0 with **no dependencies** and is **parallel-safe with Phase 1** (brand token source). It is
the gate under every downstream wasm/GPU phase — the roadmap's J6 joint ("production HTTP headers ↔ browser
runtime") is the widest-blast-radius joint in the whole arc even though its fix is one header token:

- **The CSP fix (AC-3)** unblocks **every wasm-shipping phase**: the existing kernel wasm (fixes a latent
  production bug *today*), the wgpu-wasm engine (Phase 2, J1/J4), the R-SON audio DSP third artifact
  (Phase 7, J3 — which specifically needs `'wasm-unsafe-eval'` for its own separate wasm instance), and
  RW-05's shell. Without it, none of them instantiate behind the prod CSP.
- **The wgpu fetch + Lavapipe CI (AC-1/AC-2)** unblocks FE-01/U1's real `wgpu::Device` wiring (Phase 2) and
  becomes the **correctness harness for every WGSL-touching phase after** (the `create_shader_module` gate +
  still-frame golden). It also corrects the W21 "software-raster impossible" ceiling.
- **The local visual loop** is the honest real-GPU gate for Phases 2–9 until automated real-device CI arrives
  in Phase 10.

Ship Phase 0 first and every later slice is *shippable at all*; skip it and every wasm artifact this arc
produces dies silently behind the production CSP, and no shader can be built or regression-tested without a
physical GPU. That is why it is Phase 0, "regardless of everything else in this arc" (roadmap §2 Decision A).

---

*End BLUEPRINT P00. Planning only — no product code, CI, or config edited. Every current-state citation
re-verified live against HEAD on 2026-07-16. Feature-flag, CI-job, and header changes above are prescriptions
for an implementation pass, not applied changes.*
