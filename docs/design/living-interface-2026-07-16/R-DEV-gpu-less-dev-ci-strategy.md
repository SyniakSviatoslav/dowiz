# R-DEV — GPU-less dev + CI strategy for the wgpu/WGSL living interface

> Living-interface arc, 2026-07-16. **Research/design only — nothing installed or configured.**
> Grounds the "how do we develop and CI-test real WGSL/wgpu without a GPU, and where do we host the
> live preview" question in the *actual* repo + deploy state, not a green-field guess. Companion to
> `physics-ui-capture-blueprint.md` (wgpu = sole graphics dep), `field-ui-engine/BLUEPRINTS-FIELD-UI.md`
> (FE-16 fallback), `BLUEPRINT-W20/W21` (the existing offline-ceiling), and
> `EXTERNAL-RESEARCH-gpu-neural-field-sonification.md` (the COOP/COEP dependency).
>
> Tags: **VERIFIED** (checked against repo/live source) · **RESEARCHED** (web-sourced, cited) ·
> **INFERENCE** (reasoned, flagged for a 1-line confirm).

---

## 1. Current deployment / CI state (grounded, not assumed)

### 1.1 What actually renders, and where the GPU lives
- **Zero wgpu/WGSL code exists in the repo today** (VERIFIED — `find … -name '*.wgsl'` = 0 hits;
  `physics-ui-capture-blueprint.md:56` "ZERO wgsl / wgpu / WebGPU code exists"). The `wgpu` crate is
  **not in the cargo cache and not in any `Cargo.lock`** (VERIFIED — `BLUEPRINT-W21` STATUS line;
  `engine/Cargo.toml:22-31`).
- **`engine/` (`dowiz-engine`) is pure-Rust, zero-external-dep, headless-testable** (VERIFIED —
  `engine/Cargo.toml`: only dep is the local-path `dowiz-kernel`; `default = []`). The GPU sink is a
  **feature gate that is deliberately empty**: `gpu = []`, and `bridge.rs::gpu::new_gpu` returns the
  honest `Err("gpu adapter not built — wgpu uncached")` (VERIFIED — W20 blueprint + Cargo.toml comment).
- **W20/W21 are the existing "GPU-less" pattern.** W20 completed the CPU staging path
  (`upload_once` does a real host-side vertex-buffer copy, falsifiable headless: "1 logical upload,
  0 GPU json, 0 GPU calls") plus a `HeadlessGpu` **mock** that satisfies the GREEN gate. W21 (the real
  `wgpu::Device`/`Surface` render loop, FE-04/05/08–17) is **documented CEILING: BLOCKED OFFLINE**, with
  an explicit non-goal: *"No software-raster GPU emulation (impossible without a GPU binding crate)."*
  — **That non-goal is the specific thing this document revisits.** W20/W21 blocked on *"`wgpu` not in
  cache → can't `cargo add` air-gapped"*; that is a **package-availability** constraint, **not** a
  "we can't run wgpu without a physical GPU" constraint. Those are two different blockers and only the
  first one is real here.

### 1.2 CI today (VERIFIED — `.github/workflows/`)
- `ci.yml`: post-"drop js" (2026-07-15) the JS/TS+pnpm stack was removed; CI now runs only (a) a bash
  **telemetry self-test** and (b) **`eqc` math proofs** (sympy → emit Rust → `rustc -O` → run, assert
  exit 0). **There is no GPU, no wgpu, no browser step in the real CI.**
- `visual.yml`: a Playwright **visual-regression** suite that boots the *legacy* API+SPA (`pnpm -r build`,
  `apps/web`) and diffs screenshots **inside the pinned `mcr.microsoft.com/playwright:v1.60.0-jammy`
  image** so "rendering (fonts/AA/GPU) is byte-identical to how baselines were locked" (VERIFIED —
  `visual.yml:143-156`). **INFERENCE:** this workflow is **stale/vestigial** — it targets `apps/web`
  which is now **untracked** (`git ls-files apps/web` = 0; only build artifacts remain on disk), and the
  pnpm stack it drives was removed. It is not a wgpu harness and its "pinned renderer" is a DOM-screenshot
  determinism trick (CPU font rasterization), not GPU shader testing. It is a *precedent for the pattern*
  (pin the renderer image for byte-deterministic pixels) but not a base to extend.

### 1.3 Deploy topology today (VERIFIED — corrected against stale memory)
- **The old Fly.io + Supabase centralized server was DROPPED** (VERIFIED — `Dockerfile:3-6`: *"the legacy
  centralized server (apps/api + apps/worker, Fly, Supabase) was DROPPED. This image serves ONLY the
  static SPA"*; MANIFESTO/DECISIONS D1, 2026-07-13). **`fly.toml` and `wrangler.toml` are NOT
  git-tracked** (VERIFIED — `git ls-files` finds neither; the only Cloudflare/Fly hits are docs + the
  bundled `wrangler`/`workers-best-practices` skills). The `deploy-topology.md` memory note (dated
  2026-06-19, describing two Fly apps) **predates the drop and is stale for the current arc** — treat it
  as history, not ground truth.
- **Current production serving path = a single static-binary web server** (VERIFIED):
  - `Dockerfile` (DK-04/DK-08): **zero-OCI `scratch` image** = one compiled native-Rust **axum** binary
    (`tools/native-spa-server/`) + the SPA `dist` + CA certs. No nginx, no node runtime.
  - `tools/native-spa-server/src/lib.rs`: SPA fallback, `/assets` immutable long-cache, and the **exact
    security headers ported 1:1 from `docker/nginx-default.conf`** (locked by RED tests in
    `tests/integration.rs`).
  - The canonical **field-UI shell** is `web/` (Astro build + `serve.mjs`, a **zero-dep Node static
    server**) whose own description is telling: *"Kernel-driven field UI — all geo/spectral/FSM math
    computed in the Rust dowiz-kernel wasm. **This shell only renders.**"* (VERIFIED — `web/package.json`).
- **The production security headers, verbatim** (VERIFIED — `docker/nginx-default.conf:16-25` /
  `native-spa-server/src/lib.rs:36-48`):
  ```
  Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
      img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none';
      base-uri 'self'; form-action 'self'; object-src 'none'; upgrade-insecure-requests
  X-Frame-Options: DENY
  X-Content-Type-Options: nosniff
  Referrer-Policy: strict-origin-when-cross-origin
  Permissions-Policy: geolocation=(), microphone=(), camera=(), payment=()
  ```
  **There is NO `Cross-Origin-Opener-Policy` and NO `Cross-Origin-Embedder-Policy` header, and
  `script-src` is `'self'` with no `'wasm-unsafe-eval'`.** Both matter — see §5.3. (The dev `serve.mjs`
  sets *no* CSP at all, only MIME types — which is exactly why the existing kernel wasm loads fine in
  dev but would be a different story behind the production CSP.)

**Bottom line for §1:** the operator's stated architecture is already the built architecture. The server
generates/streams state and **only serves static bytes**; the kernel math is WASM-on-CPU; "this shell
only renders" in the *visitor's* browser. Nothing in the production path needs a server GPU. The gap is
purely (a) a **dev/CI way to build+prove wgpu/WGSL without a physical GPU**, which W21 wrongly declared
impossible, and (b) **two missing / too-strict HTTP headers** that the wasm+wgpu+sonification work needs.

---

## 2. Software-rasterizer dev/CI design (concrete tooling + setup)

### 2.1 The key correction to W21
W21's non-goal — *"No software-raster GPU emulation (impossible without a GPU binding crate)"* — conflates
two things. **wgpu IS the GPU binding crate**, and wgpu is explicitly designed to run against a **software
Vulkan/GL adapter** when no hardware adapter is present. wgpu's *own* CI does exactly this. So once the
`wgpu` crate is fetched (the real W21 blocker — a one-time network `cargo add`), a GPU-less Hetzner box
**can** compile and run real WGSL end-to-end. The blocker is "wgpu isn't cached," **not** "there's no GPU."

### 2.2 Chosen approach: **Mesa Lavapipe (software Vulkan) as the primary CPU adapter; DX12-WARP/SwiftShader as reference precedent**
- **What it is:** Lavapipe (`lvp`) is Mesa's **software Vulkan driver** — a full CPU-side Vulkan
  implementation built on the `llvmpipe` Gallium rasterizer (RESEARCHED — Mesa docs; Igalia "Current
  state of Lavapipe" Vulkanised-2025). wgpu's native Linux backend is Vulkan, so wgpu enumerates the
  Lavapipe ICD as a normal (CPU) adapter — no code change, just an installed driver + an env var.
- **Precedent (strong):** wgpu's own test matrix runs on **D3D12 WARP (Windows software adapter) and
  Lavapipe for Vulkan / llvmpipe for GLES on Linux** (RESEARCHED — gfx-rs "wgpu alliance with Deno";
  wgpu CI docs). This is the exact "prove shaders on CPU in CI" workflow we want, from the crate's own
  maintainers. The two software Vulkan options in the ecosystem are **Lavapipe** and Google's
  **SwiftShader**; Lavapipe is the one wgpu CI standardizes on.
- **Why Lavapipe over SwiftShader here:** Lavapipe ships as a Debian/Ubuntu apt package
  (`mesa-vulkan-drivers`), tracks recent Vulkan (1.4-class) so modern WGSL features resolve, and is the
  path wgpu CI is tuned against. SwiftShader is the better fit only for the *browser* leg (it's what
  Chromium/Dawn use headlessly — §4.2), not the native-Rust leg.

### 2.3 Concrete setup (documentation only — do NOT run here)
On the headless Hetzner dev box / a GPU-less GitHub `ubuntu-latest` runner:
```bash
# 1. Install the software Vulkan (Lavapipe) + software GL (llvmpipe) drivers.
apt-get install -y mesa-vulkan-drivers libgl1-mesa-dri libegl1-mesa-dev libxcb-xfixes0-dev
#    (mesa-vulkan-drivers provides /usr/share/vulkan/icd.d/lvp_icd.*.json)

# 2. Point the Vulkan loader at the Lavapipe ICD only (deterministic adapter selection),
#    and force software GL as a belt-and-suspenders for the GL backend.
export VK_DRIVER_FILES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json   # newer loaders
# export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json # older loaders (alias)
export LIBGL_ALWAYS_SOFTWARE=1

# 3. Tell wgpu to accept the CPU adapter. In code, request an adapter WITHOUT
#    requiring HighPerformance, and allow fallback:
#    wgpu::RequestAdapterOptions { power_preference: LowPower,
#                                  force_fallback_adapter: true (or accept AdapterType::Cpu), .. }
#    Env override for the examples/tests: WGPU_ADAPTER_NAME=llvmpipe  (or WGPU_BACKEND=vulkan)
```
(RESEARCHED — Mesa/ArchWiki env vars; wgpu-on-lavapipe apt list from the gfx-rs wiki as surfaced in
search: `libegl1-mesa-dev libgl1-mesa-dri libxcb-xfixes0-dev mesa-vulkan-drivers`.)

### 2.4 What the CI test actually asserts (correctness, not fps)
The design intent is exactly the tradeoff the task frames: **CI proves the shader compiles and produces
the right pixels for a still frame — not that it hits 60fps.** Concretely, matching the W21 RED→GREEN
already written (*"a headless/CI GPU-raster smoke renders one field-frame frame (pixel hash
deterministic)"*):
1. `cargo build -p dowiz-engine --features gpu` **links wgpu** (RED today: unknown dep).
2. Boot wgpu against the Lavapipe adapter **headless** (offscreen `wgpu::Texture` render target — **no
   surface/window needed**; this is the standard headless-render + `copyTextureToBuffer` + `mapAsync`
   readback pattern).
3. Render **one field-frame** (VertexBridge → vertex buffer → pipeline), read the pixels back, assert a
   **deterministic pixel/content hash** (or an SSIM/tolerance compare against a locked golden).
4. Also compile-check every WGSL module (a `wgpu`/`naga` `create_shader_module` over each `.wgsl` fails
   fast on a bad shader without even rendering — a cheap first gate).

**Performance / correctness characteristics (RESEARCHED):** software rasterizers are **bit-deterministic
enough for shader-logic and still-frame correctness** but **1–2 orders of magnitude slower** than a GPU
and **explicitly non-conformant** ("lavapipe is not a conformant vulkan implementation, for testing
purposes only" — the loader literally warns this). That is acceptable **because CI's job here is
compile+correctness of a still frame, with fps validation deferred to real-device testing (§4)** — the
same division wgpu's own project uses.

---

## 3. Live-preview hosting design (production needs no server GPU — confirmed)

### 3.1 The reasoning is sound (VERIFIED against the architecture)
WebGPU/WebGL2 execute **in the visitor's browser against the visitor's GPU**. The server ships static
bytes (HTML + JS glue + the `.wasm` kernel + `.wgsl`/compiled shaders) and never touches a graphics API.
This is not a claim to defend — it's already how `web/` is built: *"This shell only renders"* on the
client; the Rust kernel math runs as WASM on the client CPU; the server is `serve.mjs` / the axum
static binary. **A GPU-less Hetzner VPS is a complete non-issue for the production serving path.** The
GPU-less box only bites for (a) local visual iteration by a developer and (b) CI — both addressed above
and below.

### 3.2 Hosting recommendation — grounded in what exists
There are two already-real static-serving surfaces; **prefer the in-repo one, keep Cloudflare for edge:**
- **Primary (in-repo, zero new infra): the native-Rust axum static server** (`tools/native-spa-server`),
  the current production artifact. Serving a wasm+wgpu build is just "more static files." Two header
  edits are needed (§5.3) — and because these are **plain static assets**, the same build drops onto any
  static host unchanged.
- **Edge/CDN: Cloudflare.** Note precisely **what Cloudflare is already used for** (VERIFIED — repo +
  memory): **DNS + wildcard `*.dowiz.org` (tenant `slug.dowiz.org` subdomains, `docs/embed/subdomain.md`),
  a WAF challenge-bypass rule for the OpenBebop GitHub webhook (`webhook.dowiz.org` tunnel), and R2 as
  the backup bucket (`docs/deployment-plan.md` §2).** So Cloudflare = **DNS/WAF/Tunnel/R2 today, NOT
  Pages/Workers hosting the app.** `docs/deployment-plan.md` §4 *proposes* Cloudflare Pages for the old
  React PWA, but that plan targets the dropped stack and was never the field-UI path. **Recommendation:**
  do **not** introduce Cloudflare Pages as new hosting just for the preview — serve the static build from
  the existing axum binary behind the Cloudflare edge you already run. If a throwaway per-branch preview
  URL is later wanted, Cloudflare Pages is a reasonable *addition* (it supports the `_headers` file the
  cross-origin-isolation config needs — §5.3), but it is **net-new infra**, not "already deployed."

### 3.3 What the operator needs for local visual iteration vs. what CI validates
- **Local visual iteration (needs a real GPU):** the operator opens the locally-served dev build
  (`web/ serve.mjs` or `wrangler`/any static server) **in a browser on a device that HAS a GPU — their
  own laptop or phone.** WebGPU shipped in all major browsers by late 2025 (Chrome/Edge 113+, Safari 26,
  Firefox 141+/145+) (RESEARCHED — sonification research §1; TestMu WebGPU support table). The dev box
  being GPU-less is irrelevant: it serves; the laptop renders. For live reload, the box runs the static
  server and the laptop points at `http://<box-ip>:4173` (or a Cloudflare tunnel, already in use for the
  webhook).
- **CI validates (headless, no GPU):** **correctness, not fidelity** — shader compiles, still-frame pixel
  hash is stable, VertexBridge → GPU buffer path executes. CI does **not** assert 60fps or "looks right on
  a real driver"; that is §4's job.

---

## 4. The real-device-testing gap + honest recommendation

A GPU-less dev box + Lavapipe CI together **cannot** prove the shader **looks right or performs
acceptably on a real end-user GPU**. The unavoidable gaps:
- **Driver divergence:** Lavapipe is non-conformant and CPU-only; real GPUs differ in precision (f32
  rounding, `fwidth`/derivative accuracy — load-bearing for the SDF analytic-AA in FE-05), texture
  filtering, and feature availability. A shader can be pixel-correct on Lavapipe and subtly wrong on a
  Mali/Adreno/Apple GPU.
- **Timing:** software raster gives **zero** signal on frame budget. The 60fps/30fps design targets
  (FE-16 "30fps design budget") can only be measured on real hardware.

### 4.1 Options, honestly scoped
| Option | What it gives | Cost / effort | Fit for Phase-0 |
|---|---|---|---|
| **Manual on operator's own devices** (laptop + 1–2 phones) | Real driver + real fps on the devices that matter most; catches gross wrongness immediately | ~free; minutes/iteration; not automated | **Primary. Sufficient now.** |
| **Playwright + headed Chromium + xvfb on a GPU-having runner** | Automated screenshot/perf on a real driver, if you add a GPU runner | Needs a GPU CI runner (self-hosted, or GH larger/GPU runners); headless Chromium does **not** use the GPU by default — you must run **headed under xvfb** (RESEARCHED — Dave Snider "Playwright with GPU Actions"; createIT) | **Defer.** Only worth it once WGSL churns enough to need regression automation. |
| **`--enable-unsafe-swiftshader` / `--enable-unsafe-webgpu` headless Chromium** | Runs WebGPU/WebGL headless on **CPU SwiftShader** — a *second* software path, browser-side | Cheap, no GPU needed | Useful as a **browser-side compile/smoke** mirror of the native Lavapipe test — but it is **still software**, so it does **not** close the real-driver gap (RESEARCHED — Chromium SwiftShader docs; note WebGPU-headless remains flaky). |
| **BrowserStack / LambdaTest real-device cloud** | Real iOS/Android/desktop GPUs + browsers on demand | Paid — entry tiers roughly **$29–$40/mo**, real-device/automation tiers higher; WebGPU only on browser versions they stock (RESEARCHED — LambdaTest/BrowserStack 2025 comparisons; TestMu WebGPU support) | **Defer to pre-launch cross-device QA.** Overkill at Phase-0. |

### 4.2 Recommendation
**Phase-0: manual testing on the operator's own laptop + phone is the honest, sufficient real-device
gate**, backed by (native) Lavapipe CI for shader compile/still-frame correctness. Add an optional
headless-Chromium `--enable-unsafe-swiftshader` smoke to catch "does it even initialize a WebGPU context
in a browser." **Defer** the GPU-runner Playwright path and the paid device clouds until (a) WGSL changes
frequently enough that manual checking is a bottleneck, or (b) you approach a public launch and need a
device matrix. Do not buy device-cloud minutes to validate a still-experimental renderer.

---

## 5. Friction / joint map

### 5.1 (a) "Looks correct in the emulator, breaks on real GPUs" — the mitigation
**Real risk** (§4). Concrete mitigations, cheapest first:
1. **Determinism firewall (already an invariant — reuse it):** the project's own rule is *"Authoritative
   compute CPU-side (WASM f64→f32); GPU=display. scalar==SIMD bit-identical"* (BLUEPRINTS-FIELD-UI §0.6,
   Appendix B.3). Keep **all** physics/layout/spectral math on the CPU kernel; the shader is a **dumb
   display** of CPU-computed vertices. This shrinks the "shader logic" surface that a software rasterizer
   might mis-verify to near-zero — most of the math never runs on the GPU at all.
2. **Golden-image tolerance, not bit-exactness, across drivers:** lock goldens rendered on a *real* GPU
   (the operator's laptop), compare CI-Lavapipe output with an SSIM/pixel-tolerance threshold — never an
   exact hash across different rasterizers (they will differ in AA/precision by design).
3. **Stay inside the WebGPU/WGSL core feature set** (no vendor extensions); this is also what the
   WebGL2-fallback discipline (FE-16) forces anyway.
4. **One real-device pass per shader-touching change** (the manual laptop/phone check) before it's
   "done." Cheap, catches the class of bug software raster can't.

### 5.2 (b) CI cost/time — software raster is slow
- **Scope CI to compile + a handful of small still frames**, not animation or large particle counts. A
  WGSL `create_shader_module` compile-check is milliseconds; a single offscreen field-frame at a modest
  resolution (e.g. 512×512, low particle N) under Lavapipe is seconds, not minutes.
- **Realistic budget:** target **≤ 3–5 min** for the whole GPU-smoke job; hard **`timeout-minutes: 10`**
  ceiling (the existing `visual.yml` already sets `timeout-minutes: 25` for a much heavier suite, so
  10 is comfortably conservative). If a frame test approaches the budget, cut resolution / particle
  count — CI proves *logic*, real devices prove *scale*.
- **Known CI pitfall to design around (RESEARCHED):** Lavapipe has a history of **hanging the runner /
  heap corruption** in long CTS-style runs (gfx-rs/wgpu issue #1974; also on Haswell). Mitigation: keep
  our render tests **tiny and few** (we are not running the full WebGPU CTS), pin the Mesa version, and
  set a per-test timeout so a hang fails fast instead of burning the whole job's wall-clock.
- **Gating:** run the GPU-smoke job **only on paths that touch `engine/` or `*.wgsl`** (mirror
  `visual.yml`'s `paths:` filter) so the common case pays nothing.

### 5.3 (c) Deployment / CSP header requirements — **the cross-cutting blocker** 🔴
This is the highest-leverage finding and it affects **other agents' designs, especially sonification.**
The current production headers (§1.3) are **too strict in two ways** for the living-interface + sonification
work:

1. **No cross-origin isolation → `SharedArrayBuffer` is unavailable.** The sonification design's own
   caveat: *"`SharedArrayBuffer` requires cross-origin isolation (COOP/COEP headers) — plan hosting
   accordingly"* (EXTERNAL-RESEARCH §Caveats + §6). The `AudioWorklet` ⇄ compute-readback ⇄ WASM-DSP path
   uses `SharedArrayBuffer` + `Atomics` for glitch-free audio. Today there is **no `Cross-Origin-Opener-Policy`
   and no `Cross-Origin-Embedder-Policy`** on any response, so `crossOriginIsolated` is `false` and
   `SharedArrayBuffer` is disabled. **Required (RESEARCHED — Cloudflare Pages/Workers COOP/COEP docs):**
   ```
   Cross-Origin-Opener-Policy: same-origin
   Cross-Origin-Embedder-Policy: require-corp
   ```
   Add these to `native-spa-server`'s `SECURITY_HEADERS` (and to a Cloudflare `_headers` file if Pages is
   ever used). ⚠️ `require-corp` makes every cross-origin subresource need CORP/CORS — for a `default-src
   'self'` app that ships all assets same-origin this is low-risk, but any third-party embed
   (fonts/analytics/iframes) must then send CORP or it breaks.

2. **`script-src 'self'` (no `'wasm-unsafe-eval'`) blocks WebAssembly instantiation in Chromium.**
   (INFERENCE — HIGH confidence, needs a 1-line browser confirm.) Chromium-family browsers gate
   `WebAssembly.compile`/`instantiate` behind `'unsafe-eval'` or the narrower `'wasm-unsafe-eval'` in
   `script-src`; with bare `'self'` a `.wasm` module throws a CSP `CompileError`. The **existing kernel
   wasm dodges this only because dev `serve.mjs` sends no CSP at all** — behind the *production* axum/nginx
   CSP the kernel wasm (and any wgpu-wasm build) would be blocked. **Required:**
   ```
   Content-Security-Policy: … script-src 'self' 'wasm-unsafe-eval'; …
   ```
   This is latent **today** for the kernel wasm, independent of the GPU work — flag it now.
   **Verify** by loading the production-header build in Chrome and watching for a CSP wasm violation.

3. **Minor:** if WGSL/textures are ever loaded via `fetch` from a CDN host, `connect-src`/`img-src` would
   need that host; the all-same-origin static build keeps `connect-src 'self'` fine.

### 5.4 Joint map (who depends on what)
```
 GPU-less Hetzner box ─┬─ (dev)  serves static bytes ───────────► operator's laptop/phone GPU renders  (§3.3)
                       └─ (CI)   cargo build --features gpu
                                 + Lavapipe headless still-frame ─► shader compile + pixel-hash  (§2.4)
                                        │ correctness only, NOT fps
                                        ▼
 real-device gap ───────────────► manual laptop/phone  (Phase-0)  ─► real driver + real fps  (§4.2)
                                   [defer: GPU-runner Playwright, BrowserStack]

 production serving path (axum static binary / Cloudflare edge)
       │  MUST add:  COOP:same-origin + COEP:require-corp   ──────► unlocks SharedArrayBuffer (SONIFICATION dep) 🔴
       │  MUST add:  script-src … 'wasm-unsafe-eval'         ──────► unblocks kernel wasm + wgpu-wasm  🔴
       └─ no server GPU anywhere on this path (confirmed §3.1)
```

---

## 6. Phase-0 recommendation (minimum viable dev/CI to unblock the arc)

**Do these; skip the rest.**
1. **Unblock W21's real blocker, once:** perform the one-time network `cargo add wgpu` (operator gate,
   already the documented trigger in `BLUEPRINT-W21`) so `wgpu` enters the cache/lockfile. This — **not**
   any GPU — is what W20/W21 were actually blocked on. Correct the W21 non-goal line: *software-raster
   GPU emulation via Lavapipe is exactly how CI will run wgpu without a GPU.*
2. **Native GPU-smoke CI job (Lavapipe):** on a GPU-less runner, `apt-get install mesa-vulkan-drivers …`,
   set `VK_DRIVER_FILES=…/lvp_icd…json` + `LIBGL_ALWAYS_SOFTWARE=1`, request a fallback/CPU adapter, and
   run: (a) `create_shader_module` compile-check over every `.wgsl`, (b) one offscreen field-frame render
   → readback → **pixel-hash / SSIM-vs-golden**. Gate it on `engine/**` + `**/*.wgsl` paths;
   `timeout-minutes: 10`; keep frames tiny (Lavapipe-hang mitigation).
3. **Local visual loop:** developer serves the static build from the GPU-less box; **iterates visually on
   their own GPU-having laptop/phone** (browser WebGPU, late-2025 baseline). No new infra.
4. **Two header edits (the cross-cutting unblock for sonification + wasm):** add
   `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: require-corp`, and
   `'wasm-unsafe-eval'` to `script-src` in `native-spa-server`'s `SECURITY_HEADERS` (RED-test-locked, so
   update the test golden too). This is a prerequisite the sonification agent's design **depends on** —
   flag it to them.
5. **Hosting:** keep serving the static build from the **existing axum binary behind the Cloudflare edge
   you already run** (DNS/WAF/Tunnel/R2). **Do not** stand up Cloudflare Pages as if it were existing
   infra; it's an optional future add for throwaway preview URLs (and it does support the `_headers`
   COOP/COEP config if you go there).

**Explicitly defer:** GPU-having CI runners + headed-Chromium/xvfb Playwright; BrowserStack/LambdaTest
device clouds; any real-fps regression automation. Manual device testing is the honest Phase-0 gate.

---

### Sources
**Repo (VERIFIED):** `engine/Cargo.toml`, `docs/design/BLUEPRINT-W20-vertexbridge-gpu-gate.md`,
`docs/design/BLUEPRINT-W21-field-ui-gpu-blocked.md`, `docs/design/physics-ui-capture-blueprint.md`,
`docs/design/field-ui-engine/BLUEPRINTS-FIELD-UI.md`, `.github/workflows/{ci,visual}.yml`, `Dockerfile`,
`docker/nginx-default.conf`, `tools/native-spa-server/src/lib.rs`, `web/{package.json,serve.mjs}`,
`docs/embed/subdomain.md`, `docs/deployment-plan.md`,
`docs/design/living-interface-2026-07-16/EXTERNAL-RESEARCH-gpu-neural-field-sonification.md`.
**Web (RESEARCHED):**
- wgpu CI runs on WARP + Lavapipe/llvmpipe: [gfx-rs "wgpu alliance with Deno"](https://gfx-rs.github.io/2021/09/16/deno-webgpu.html); apt list + hang caveat: [gfx-rs/wgpu #1974](https://github.com/gfx-rs/wgpu/issues/1974), [#1551](https://github.com/gfx-rs/wgpu/issues/1551).
- Lavapipe = Mesa software Vulkan on llvmpipe: [Mesa LLVMpipe docs](https://docs.mesa3d.org/drivers/llvmpipe.html), [Igalia "Current state of Lavapipe" (Vulkanised 2025)](https://www.vulkan.org/user/pages/09.events/vulkanised-2025/T5-Lucas-Fryzek-Igalia.pdf), [airlied lavapipe FAQ](https://airlied.blogspot.com/2020/08/vallium-software-swrast-vulkan-layer-faq.html); env vars + `mesa-vulkan-drivers`/`vulkan-swrast`: [Vulkan ArchWiki](https://wiki.archlinux.org/title/Vulkan), [pkgs.org mesa-vulkan-drivers](https://pkgs.org/download/mesa-vulkan-drivers).
- SwiftShader (browser CPU path) + Chromium flags: [Chromium SwiftShader docs](https://chromium.googlesource.com/chromium/src/+/main/docs/gpu/swiftshader.md), [Support GPU in headless (crbug 40540071)](https://issues.chromium.org/issues/40540071).
- Playwright + real GPU needs headed+xvfb: [Dave Snider "Playwright with GPU Actions"](https://davesnider.com/posts/gputests), [createIT headless WebGL](https://www.createit.com/blog/headless-chrome-testing-webgl-using-playwright/).
- WebGPU browser baseline + device clouds: [TestMu WebGPU support](https://www.testmuai.com/learning-hub/webgpu-browser-support/), [LambdaTest vs BrowserStack 2025](https://www.devassure.io/blog/lambdatest-vs-browserstack/).
- COOP/COEP for SharedArrayBuffer on static hosts: [Cloudflare Pages Headers](https://developers.cloudflare.com/pages/configuration/headers/), [Cloudflare Workers security headers](https://developers.cloudflare.com/workers/examples/security-headers/), [aboutweb.dev cross-origin isolation](https://aboutweb.dev/blog/cross-origin-isolation-requirements-sharedarraybuffer-cloudflare-worker/).
</content>
</invoke>
