# Design-Library Landscape 2026 — Intent-Driven · WebGL/WebGPU · Generative-Without-AI

> **Date:** 2026-07-11 · **Method:** web survey (WebSearch/WebFetch fan-out, 6 research agents) with
> independent verification: npm registry API (version/publish-date/license/deps), bundlephobia API
> (gz sizes), and direct tarball measurement (`gzip -9` on dist files) where bundlephobia failed.
> Every load-bearing number is cited or marked UNVERIFIED. **Research only — nothing recommended
> here is implemented; no repo file outside this doc was touched.**
>
> **Evaluation lens (fixed):** Astro 5 + Svelte 5 islands rebuild, per-route JS working targets
> 25/35/60 kB gz (authoritative storefront ceiling 60–90 kB gz, `inventory/11-frontend-surface.md`
> §7.1; measured base today 21.6 kB gz, Svelte floor 14.3 kB gz — G05 §2.1); SSR-friendly; licenses
> compatible with the planned AGPLv3 relicense (HANDOFF.md Layer 0); zero/low deps; **no AI at
> runtime**; brand = Warm Cosmo-Noir (BRAND-BIBLE.md), server OG cards via sharp SVG→PNG.

---

## 0. Executive summary

**Top pick per category:**

| Category | Top pick | Why (one line) |
|---|---|---|
| 1 — Intent-driven | **No new library; keep native CSS tokens + (if headless needed) Bits UI** | The "intent" job is already done by `tokens.css` + Svelte 5; DTCG spec went stable 2025-10 and is a *format*, not a runtime; Bits UI is the Svelte-5-native headless layer if/when needed |
| 2 — WebGL/WebGPU | **OGL** (hero, ~14–20 kB gz tree-shaken, Unlicense) — or **raw WebGL2 quad, measured 1.5 kB gz** for a single shader | three.js full is 178 kB gz (7–8× a whole route budget); threlte 8 is superb but pulls three; WebGPU-only still loses ~16 % of users mid-2026 |
| 3 — Generative w/o AI | **A "primitives kit," not a framework: simplex-noise (1.8 kB) + culori (tree-shaken) + rough.js (8.6 kB) + blobshape (1 kB) + SVG `feTurbulence` (0 kB)** — server-side: keep sharp, add satori/takumi only if HTML-layout OG cards are wanted | Every creative-coding *framework* (p5 322.7 kB gz, paper 84 kB, two.js 47.6 kB) blows a route budget; the primitives are tiny, 0-dep, MIT/ISC, and run identically in Node for OG cards |

**Top pick per dowiz use-case (compact — full fit matrix in §4):**

| Use-case | Pick | Fallback | est. gz added |
|---|---|---|---|
| Landing hero (HorizonDrift-class, cinematic) | Raw WebGL2 fullscreen-quad shader (hand-rolled) | OGL subset | ~1.5 kB / ~14–20 kB |
| Storefront polish (particles, product flair) | CSS/SVG first; OGL island if truly needed | PixiJS only if 2D-heavy roadmap appears | 0 / ~15–25 kB |
| Per-tenant generative theming | culori (oklch ramps, tree-shaken) + tiny seeded PRNG | @thi.ng/color if deeper color math needed | ~3–6 kB (server-side: 0 client) |
| OG cards (server) | Keep sharp SVG→PNG; add satori(+satori-html) only for HTML-flex layout | @resvg/resvg-js as rasterizer swap | 0 client |
| Brand textures / grain / receipts / QR | SVG `feTurbulence` + rough.js + styled-QR via `qrcode`+own SVG post-pass | css-doodle (client-only, 36 kB — avoid) | 0–9 kB |

**Five surprises from the research (detailed in-line):**
1. **shadcn/ui switched its default from Radix to Base UI in July 2026** (Base UI v1.6.0, MIT, by the ex-Radix/MUI/Floating-UI people; "projects pick Base UI over Radix 2:1") — the Radix era is ending upstream of half the industry ([shadcn changelog](https://ui.shadcn.com/docs/changelog)).
2. **The W3C Design Tokens spec (DTCG) finally went stable** — first stable version 2025.10 on 2025-10-28, with Style Dictionary v5 / Terrazzo / Tokens Studio consuming it ([w3.org](https://www.w3.org/community/design-tokens/2025/10/28/design-tokens-specification-reaches-first-stable-version/)).
3. **Meta open-sourced Astryx (2026-06-27, MIT)** — a 150+-component React+StyleX design system explicitly built to be *agent-legible* (CLI + machine-readable manifest); the one concrete artifact behind all the "intent-driven design" discourse ([github.com/facebook/astryx](https://github.com/facebook/astryx), verified live: v0.1.4, 7.9k stars).
4. **lygia, the most popular shader library, is NOT usable commercially for free** — dual Prosperity/Patron license, non-commercial by default (verified on the repo). Everyone assumes it's open; it isn't.
5. **The OGL license is Unlicense (public domain), not MIT** — and its core+math tree-shakes to ~14 kB gz, making a sub-30 kB WebGL hero genuinely achievable ([github.com/oframe/ogl](https://github.com/oframe/ogl)).

---

## 1. Category 1 — "Intent-driven" design libraries

### 1.1 What "intent-driven" actually means in 2025–2026 (landscape map)

The term has **no single referent**. Four real clusters exist, one hype cluster does not:

- **(a) Declarative style-intent / token systems** — you declare *what* (semantic tokens, typed style objects), a compiler emits CSS. Build-time, zero/near-zero runtime. Panda CSS, StyleX, vanilla-extract, Tailwind v4 `@theme`, DTCG spec + Style Dictionary/Terrazzo, Open Props.
- **(b) Behavior-intent headless components** — you declare *what a widget is* (dialog, combobox), the lib supplies interaction/a11y behavior; you own all styling. Radix, React Aria, Base UI (React); **Ark UI/Zag.js** (state machines, cross-framework incl. Svelte); **Bits UI**, Melt (Svelte-native).
- **(c) Algorithmic/constraint layout** — you declare layout *intent* (fluid scale endpoints, layout primitives, constraints), math resolves it. Every Layout, Utopia, CUBE CSS (all 0 kB JS); Cassowary solvers (kiwi.js lineage) — effectively obsoleted for page layout by CSS Grid/`clamp()`/container queries.
- **(d) "Intent-driven UI" as a 2025–2026 movement** — overwhelmingly *discourse* (outcome-oriented UX think-pieces), with exactly three substantive artifacts: DTCG 2025.10 (a token format), Google's **A2UI v0.9** (a protocol for *agents* to declare UI intent — orthogonal to this project), and Meta's **Astryx** (agent-legible design system). No installable "intent framework" for human-authored UI emerged. A React kit literally named "Intent UI" (v3.8.4, June 2026) is a brand-name coincidence, not the paradigm.

**Honest verdict for dowiz:** interpretation (a) is already implemented in-house (`packages/ui/src/theme/tokens.css`, `[data-skin]` layers = semantic token intent, zero runtime); (c) is a set of free techniques, not dependencies; (b) is the only cluster that could ever become a dependency, and only Bits UI / Ark-Svelte fit the stack.

### 1.2 Comparison table — styling/token systems (verified 2026-07-11)

| System | Runtime JS | License | Latest (npm) | Published | Deps | Svelte/Astro fit |
|---|---|---|---|---|---|---|
| Panda CSS `@pandacss/dev` | 0 (build-time) | MIT | 1.11.4 | 2026-06-27 | 12 (dev-side) | Official Svelte + Astro guides |
| StyleX `@stylexjs/stylex` | ~0 (styleq merge) | MIT | 0.19.0 | 2026-06-16 | 3 | React-centric in practice |
| vanilla-extract `@vanilla-extract/css` | 0 core | MIT | 1.21.1 | 2026-06-30 | 11 (dev-side) | Agnostic (.css.ts → CSS) |
| Tailwind v4 (foil) | 0 | MIT | 4.3.2 | 2026-06-29 | 0 | First-class Astro/SvelteKit |
| Style Dictionary | 0 (codegen) | Apache-2.0 | 5.5.0 | 2026-06-21 | 13 | Agnostic (emits CSS vars) |
| Terrazzo `@terrazzo/cli` | 0 (codegen) | MIT | 2.4.0 | 2026-06-13 | 18 | Agnostic; DTCG-purist |
| Open Props | 0 (pure CSS) | MIT | 1.7.23 | 2026-01-31 | 0 | Perfect; 22.1 kB gz *CSS* if imported whole |
| Theme UI | runtime (emotion) | MIT | 0.17.4 | 2026-01-02 | 6 | React-only; legacy era |

Sources: npm registry API queries 2026-07-11 (versions/dates/licenses/deps as listed); [panda-css.com getting-started](https://panda-css.com/docs/overview/getting-started); [stylexjs.com](https://stylexjs.com/docs/learn/thinking-in-stylex/); [vanilla-extract status discussion](https://github.com/vanilla-extract-css/vanilla-extract/discussions/1144); [open-props.style](https://open-props.style/); bundlephobia (open-props 22.07 kB gz).

### 1.3 Per-library verdicts (the ones that matter)

- **Panda CSS** — the most mature "style-intent" compiler in 2026: zero-runtime atomic CSS, token dictionary + recipes, official Svelte/Astro installs, unusually clean tracker (~10 open issues), same team as Ark/Zag/Chakra v3. **Verdict: best-in-class, but dowiz already has a working token layer; adopting Panda would be a build-toolchain add for aesthetics of authoring, not user value. Skip unless the token layer is ever rebuilt.**
- **StyleX** — Meta-scale, near-zero runtime, now the engine under Astryx. React-in-practice despite agnostic claims (Svelte only via unofficial demos). **Verdict: wrong ecosystem for a Svelte 5 build. Skip.**
- **vanilla-extract** — strongest *typed token contract* model (`createThemeContract`); maintainers self-describe limited capacity (weekly meetings, slow features) — stable-not-accelerating. **Verdict: viable if type-checked tokens are ever wanted; not needed now.**
- **DTCG spec + Style Dictionary v5 / Terrazzo** — the real 2025–2026 news: a stable interchange *format* (2025.10, 2025-10-28) with two living build tools (SD v5.5.0 Apache-2.0 incumbent; Terrazzo MIT spec-purist, small but active, 423 stars). **Verdict: relevant to dowiz only if per-tenant theming ever needs a token *pipeline* (tenant JSON → CSS vars). File the format away; adopt no tool yet — dowiz's tenant theming is a handful of CSS custom properties, not a multi-platform token graph.**
- **Open Props** — 0-dep CSS custom-property pack, MIT, but momentum visibly stalled (no push since 2026-01-31, v2 stuck in beta). **Verdict: skip; dowiz's own tokens are more coherent than a generic pack.**
- **Theme UI** — six months between releases, superseded era. **Verdict: legacy; skip.**
- **Native CSS as the real winner:** `@property` universal; container **style queries** hit Baseline May 2026 (Firefox 151; Chrome 111+/Safari 18+) — a component can branch on an ancestor's declared custom-property intent in pure CSS ([MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Containment/Container_size_and_style_queries), [web.dev 05-2026](https://web.dev/blog/web-platform-05-2026)). **This is the platform absorbing the "intent" category — and it costs 0 kB.**

### 1.4 Behavior-intent headless systems (the only adoptable cluster)

| System | Intent model | Svelte 5 | License | Latest | Published | Notes |
|---|---|---|---|---|---|---|
| **Bits UI** | Radix-style primitives, Svelte-5-native (runes/snippets) | ✅ native | MIT | 2.18.1 | 2026-05-03 | 6 deps (floating-ui, runed, tabbable…); powers shadcn-svelte; gz per-component UNVERIFIED (bundlephobia can't build Svelte — measure in a Vite build) |
| **Ark UI Svelte** `@ark-ui/svelte` | Zag.js finite state machines — behavior defined once, framework adapters render | ✅ (Svelte 5 only) | MIT | 5.22.1 | 2026-06-06 | 67 deps (all internal `@zag-js/*`); cross-framework parity; heavier conceptual+byte payload |
| Melt (`melt` pkg) | Builder rewrite for runes | ✅ pre-1.0 | MIT | 0.44.0 | 2026-01-04 | Legacy `@melt-ui/svelte` in maintenance (last push 2025-09-30); rewrite incomplete — adoption risk today |
| Radix UI (React) | Imperative primitives | ❌ | MIT | react-dialog 1.1.19 | 2026-07-06 | WorkOS-stewarded, slowed; shadcn demoted it July 2026 |
| React Aria Components | Behavior hooks | ❌ | Apache-2.0 | 1.19.0 | 2026-06-18 | Reference-quality a11y; React-only |
| Base UI `@base-ui/react` | Refined Radix-lineage primitives | ❌ | MIT | 1.6.0 | 2026-06-18 | v1.0 GA 2025-12-11; fastest-rising; new shadcn default |

Sources: npm registry API 2026-07-11; [bits-ui.com](https://www.bits-ui.com/) / [github.com/huntabyte/bits-ui](https://github.com/huntabyte/bits-ui); [ark-ui.com Svelte announcement](https://ark-ui.com/blog/introducing-ark-ui-svelte); [melt-ui/next-gen](https://github.com/melt-ui/next-gen); [shadcn changelog — Base UI default, July 2026](https://ui.shadcn.com/docs/changelog); [Base UI v1 coverage](https://www.infoq.com/news/2026/02/baseui-v1-accessible/).

**Verdict:** dowiz's islands are few and bespoke (MenuBrowser, CartButton, LanguageSwitcher); a headless library earns its keep only when a hard widget arrives (combobox, date picker, complex dialog stack). When that day comes: **Bits UI first** (native, lean, shadcn-svelte ecosystem), **Ark-Svelte second** (if cross-framework behavior parity ever matters). Zag's "intent as state machine" is the most intellectually honest intent model in the whole category — behavior declared once as a machine, views generated per framework.

### 1.5 Algorithmic/constraint layout — techniques, not dependencies

- **Every Layout** (Pickering/Bell): ~10 CSS layout primitives (Stack/Cluster/Sidebar/Cover…), 0 kB JS, translate directly to `.astro`/`.svelte` components. 2nd ed. 2021, complete-not-evolving. **Adopt the patterns, buy nothing** ([every-layout.dev](https://every-layout.dev/)).
- **Utopia** (fluid type/space via `clamp()`): methodology + `utopia-core` (ISC, v1.6.0, 2024-09-19 — quiet because *finished*). Build-time only. **Directly compatible with the bebop token layer — a candidate for formalizing dowiz's fluid scale** ([utopia.fyi](https://utopia.fyi/)).
- **CUBE CSS**: authoring methodology, 0 kB, still the reference pairing with Every Layout ([cube.fyi](https://cube.fyi/)).
- **Constraint solvers** (kiwi.js → `@lume/kiwi` BSD-3-Clause, v0.4.4 2024-09-17): original kiwi.js unmaintained; the fork lives for canvas/diagram editors. No web revival — CSS Grid `minmax()`/`auto-fit`, `clamp()`, container queries ate this category. CSS masonry: still contested — Safari 26.4 shipped "Grid Lanes"/item-flow first; Chrome/Firefox behind flags mid-2026; fallback still required ([chrome blog](https://developer.chrome.com/blog/masonry-update)). **Verdict: no JS constraint solver belongs in a storefront.**

### 1.6 The 2025–2026 "intent movement" — honest note

Search confirms: overwhelmingly think-pieces (NN/g "outcome-oriented design", UX Collective). Concrete artifacts are exactly three — DTCG 2025.10 (format standard), **Google A2UI v0.9** (multi-vendor protocol for *agent-generated* UI declaring intent rendered by the client's design system — [developers.googleblog.com](https://developers.googleblog.com/a2ui-v0-9-generative-ui/)) and **Meta Astryx** (2026-06-27, MIT, React+StyleX, 150+ components, CLI + machine-readable manifest so humans and coding agents consume identical APIs; verified live at v0.1.4, 7.9k stars — [github.com/facebook/astryx](https://github.com/facebook/astryx)). A2UI/Astryx both assume AI-agent workflows and React respectively — **neither is adoptable for a no-AI-at-runtime Svelte product; both are worth knowing as vocabulary.**

---

## 2. Category 2 — WebGL / WebGPU libraries

### 2.1 The one number that frames everything

The whole landing route budget is **25 kB gz** (working target; hard ceiling 60–90 kB for the storefront). A *minimal tree-shaken* three.js scene measures **~129 kB gz** (first-party esbuild+gzip measurement against three@0.185.1 — Scene/Camera/WebGLRenderer/one mesh/two lights; the kitchen-sink import is 178–182 kB gz per bundlephobia). Every "real engine" is therefore 4–60× over the hero budget before a single line of effect code. The viable zone is micro-libs and raw WebGL.

### 2.2 Size/scope comparison table (verified 2026-07-11)

| Library | gz size (method) | Tree-shake | License | Latest / published | Deps | Svelte/Astro island fit |
|---|---|---|---|---|---|---|
| **three** | 178.1 kB full (bundlephobia); **~129 kB minimal scene (measured, esbuild+gzip)**; ~207 kB WebGPU/TSL path (measured) | poor (~28 % off full) | MIT | 0.185.1 / 2026-07-01 | 0 | Works as lazy island; blows any route budget |
| **@babylonjs/core** | ~1.69 MB full; **~1.47 MB minimal scene (measured)** | very poor | Apache-2.0 | 9.16.1 / 2026-07-09 | — | Disqualified (~50× hero budget) |
| **playcanvas** | ~564 kB full; **~476 kB minimal (measured)** | poor | MIT | 2.20.6 / 2026-07-06 | — | Editor-centric; disqualified |
| **pixi.js** | 245.4 kB full (bundlephobia); **~156.5 kB single-sprite app (measured)** | designed-in but floor is high | MIT | 8.19.0 / 2026-06-04 | 10 | 2D-excellent, 5× hero budget |
| **ogl** | 33.4 kB full (bundlephobia); README self-report core 8 + math 6 + extras 15 kB — **~14–20 kB realistic subset** | good (ESM, 0 deps) | **Unlicense (public domain)** | 1.0.11 / 2025-01-27 | 0 | **Best engine-tier fit for a hero island** |
| **regl** | 36.8 kB (bundlephobia) | monolithic | MIT | 2.1.1 / 2024-11-12 | 0 | Functional style; low momentum |
| **twgl.js** | 22.1 kB full (bundlephobia); base module smaller | modular | MIT | 7.0.0 / 2025-07-16 | 0 | Thin WebGL helper, viable |
| **curtainsjs** | 26.1 kB (measured: umd.min gzip -9) | monolithic | MIT | 8.1.6 / 2024-05-02 | 0 | DOM-GL bridge; author moved on |
| **gpu-curtains** | 132.5 kB (measured umd.min; ESM tree-shakes) | ESM | MIT | 0.16.3 / 2026-03-24 | 1 | WebGPU successor, pre-1.0 |
| **gl-matrix** | 13.3 kB full (bundlephobia); per-module much less | good | MIT | 3.4.4 / 2025-08-08 | 0 | Math only; pairs with raw GL |
| **@threlte/core** | +thin layer over three (scene total lands in the same 130–210 kB band) | n/a | MIT | 8.5.16 / 2026-05-25 | 0 (peers: svelte ≥5, three ≥0.160) | **Svelte-5-native; inherits three's floor** |
| **@react-three/fiber** | v9.6.1 / 2026-04-28 (context only) | — | MIT | — | 10 | React-only |
| **postprocessing** (pmndrs) | requires three (peer) | — | Zlib | 6.39.2 / 2026-06-28 | 0 | Only meaningful with three |

Sources: bundlephobia API queries 2026-07-11; first-party measurements (esbuild `--bundle --minify` + `gzip -9` per engine agent; npm-tarball dist `gzip -9` for curtains/gpu-curtains); npm registry API (versions/dates/licenses/deps); [github.com/oframe/ogl](https://github.com/oframe/ogl) README size self-report.

### 2.3 Engine-tier verdicts

- **three.js (r185, MIT)** — release cadence slowed to 8–10 weeks (0.182 Dec 2025 → 0.185 Jun 2026) but healthy. **WebGPURenderer is still officially experimental** — the manual itself says "you will encounter missing features or better performance with WebGLRenderer" — with automatic WebGL2 fallback; TSL (one node graph → WGSL+GLSL) is real but docs are thin, ecosystem young (`tsl-textures` v3.0.4, MIT, fills procedural-texture gaps). The measured WebGPU/TSL path costs ~207 kB gz, ~60 % more than the WebGL path. **Verdict: the default choice the moment dowiz ever needs a real 3D scene — as a lazily-loaded, below-the-fold island outside the route budget — and not one byte sooner** ([threejs.org WebGPURenderer manual](https://threejs.org/manual/en/webgpurenderer.html), [TSL docs](https://threejs.org/docs/TSL.html)).
- **Babylon.js 9 (Apache-2.0)** — well-resourced (Microsoft; annual majors; Gaussian-splat + Havok momentum), measured minimal scene ~1.47 MB gz. **Verdict: category error for this product. Skip.**
- **PlayCanvas (MIT, 2.20.6)** — engine is genuinely open, but the product is the cloud editor; splat-centric momentum (SuperSplat WebGPU renderer 2026). Minimal measured ~476 kB gz. **Verdict: skip.**
- **PixiJS v8 (MIT, 8.19.0)** — technically the best 2D engine (dual WebGL/WebGPU, though its own guide labels the WebGPU renderer "🚧 Experimental" and recommends WebGLRenderer for production); measured single-sprite floor ~156.5 kB gz. **Verdict: adopt only if a 2D-heavy roadmap materializes (game-like menu experiences); never for a hero effect** ([pixijs.com renderers guide](https://pixijs.com/8.x/guides/components/renderers)).

### 2.4 Bindings

- **Threlte 8 — the key Svelte fact:** `@threlte/core` 8.5.16 (2026-05-25, MIT) peer-requires **`svelte: ">=5"`** and `three: ">=0.160"` (npm registry, verified) — a Svelte-5-runes rewrite, declarative `<T>` components compiled without R3F-style runtime class lookup. `@threlte/extras` 9.21.0 (2026-06-04) is active. It adds little over three itself, which means it also *saves* nothing: a threlte hero lands in the same 130–210 kB gz band. **Verdict: the right shape for "full scenes if ever needed" on this stack; irrelevant below that tier** ([threlte.xyz/blog/threlte-8](https://threlte.xyz/blog/threlte-8/)).
- **react-three-fiber v9.6.1** (React 19) — context only; drei's last stable is 10.7.7 (2025-11-13, ~8 months quiet; newer work on an alpha channel — UNVERIFIED beyond that). **TresJS 5.8.3** (Vue, 2026-06-18) healthy. Neither fits the stack.

### 2.5 WebGPU reality, mid-2026

- caniuse global: **83.63 %** (82.17 full + 1.46 partial), fetched live 2026-07-11 ([caniuse.com/webgpu](https://caniuse.com/webgpu)).
- Chrome/Edge 113+ (Apr 2023); **Safari 26 shipped WebGPU Sept 2025** (macOS/iOS/iPadOS/visionOS 26 — the WWDC25 promise landed; [webkit.org](https://webkit.org/blog/17333/webkit-features-in-safari-26-0/)); Chrome-on-Android supported on recent hardware; Samsung Internet 24+.
- **Firefox is the contested one:** blog-level reports say Windows since FF 141 (Jul 2025) and Apple-Silicon macOS since FF 145, Linux "2026" — but caniuse's own table still marks no Firefox version as fully supported (disabled-by-default rows). Conflict noted; treat Firefox default-on status as **unresolved/partial**.
- **Verdict: WebGL fallback is still mandatory for a consumer storefront in mid-2026.** ~16 % of global traffic (older Android, Firefox variants, in-app webviews, pre-26 Safari) has no WebGPU. Every surveyed engine ships automatic WebGL2 fallback as a design default — the maintainers have already made this call. For dowiz: author effects against WebGL2 now; TSL/WebGPU becomes interesting only at the "full scene" tier.

### 2.6 Recommendation ladder (operator's three rungs)

1. **(i) Cinematic hero under ~30 kB** → **hand-rolled WebGL2 fullscreen-quad shader** (one `<canvas>`, ~100 lines of boilerplate, GLSL fbm noise inlined): **measured 1.5 kB gz** — a representative complete hero module (fullscreen-triangle, fbm drift field, amber/near-black tint uniforms, DPR resize, IntersectionObserver pause, `prefers-reduced-motion` freeze, WebGL-context-loss cleanup) written during this research minifies to 2,839 B → **1,473 B gzip** (esbuild `--minify` + `gzip -9`; source preserved at `/tmp/bpcheck/raw-hero.js`). Codrops' 2025–2026 tutorial wave teaches exactly this pattern raw, and independent references put hand-rolled quad heroes at 1–3 kB gz ([Codrops 2026-03-04](https://tympanus.net/codrops/2026/03/04/webgl-for-designers-creating-interactive-shader-driven-graphics-directly-in-the-browser/)). Zero deps, fits *inside* the 25 kB route budget alongside everything else. Fallback rung: **OGL subset** (~14–20 kB gz, Unlicense, 0 deps) when you want mesh/texture/render-target ergonomics — still fits 25 kB only if the route carries almost nothing else, comfortable under 35 kB. curtainsjs (26.1 kB, maintenance-frozen) only if scroll-synced DOM-to-GL planes become a requirement; twgl (22.1 kB full, modular) as the middle path between raw GL and OGL.
2. **(ii) Product/menu 3D or particles** → CSS/SVG first (0 kB); then a **small OGL particle island** (lazy, below-fold, ~15–25 kB); **PixiJS v8 only** if a persistent 2D-heavy surface appears — as its own deferred chunk, never in the critical path.
3. **(iii) Full scenes if ever needed** → **three (WebGL path) + Threlte 8**, lazily loaded below-fold island, explicitly outside route budgets (~130–210 kB gz); revisit the WebGPU/TSL path when three flips WebGPURenderer to non-experimental *and* Firefox resolves.

### 2.7 Shader tooling, DOM-GL bridges, and one-shot widgets (flags)

- **lygia — LICENSE TRAP (the survey's biggest gotcha):** dual **Prosperity/Patron** license. Prosperity = free for *non-commercial* only, with a single 30-day commercial trial per company; commercial use requires becoming a GitHub-sponsor "Patron" or buying a per-version license. Verified on the repo (v1.4.1, 2026-02-07, 3.4k stars — very much alive, just not free for a storefront). **Do not vendor lygia snippets into dowiz.** The Book of Shaders / IQ-article techniques (hash/noise/fbm idioms) are re-implementable from math, as done in the measured hero above ([lygia.xyz/license](https://lygia.xyz/license), [github.com/patriciogonzalezvivo/lygia](https://github.com/patriciogonzalezvivo/lygia)).
- **glslify** — confirmed dead-stale: last real release 7.1.1 (2020-09-02), last push 2022-06-29, 15 deps. ESM/Vite era passed it by. **Skip.**
- **TSL standalone** — architecturally portable (WGSL+GLSL NodeBuilders) but ships only inside the three monorepo; no decoupled package exists mid-2026. **Not usable without three.**
- **curtains.js → gpu-curtains** — curtainsjs (26.1 kB gz measured, MIT, 0 deps) works but is in maintenance freeze (last publish 2024-05-02); the author's energy went to **gpu-curtains** (0.16.3, 2026-03-24, MIT) which is a *full WebGPU engine* (~132 kB gz measured UMD; agent-measured ~213 kB with different bundling) — pre-1.0, tiny community (183 stars). **Watch, don't adopt.**
- **VFX-JS** (`@vfx-js/core` 1.1.0, 2026-06-08, MIT, 0 deps, 28.6 kB gz) — attach shader effects to DOM elements (`<img>`/video/text); the most credible *maintained* DOM-GL bridge in 2026, but consumes an entire route budget solo. **Only for a dedicated effects-heavy page.**
- **four** (published as `fourwastaken`, v0.4.3, 2026-01-11, MIT, 369 stars) — minimal three-alternative with WebGL2+WebGPU backends; interesting, too small a bus factor for production. **Watch.**
- **One-shot widgets — all fail licensing/size checks:** vanta.js (MIT but stale since 2024-03, requires *global* three → real cost 150 kB+); threejs-toys (11.5 kB gz + three on top, npm license field unclear, stale); finisher-header npm package is an *unofficial republish* of a proprietary generator — provenance murky. **Skip all three.** nanogl is GPL-2.0 — incompatible with a permissive client bundle policy; picogl stale since 2022 ([sources: npm registry + bundlephobia + repos, 2026-07-11](https://www.npmjs.com/package/vanta)).

---

## 3. Category 3 — Generative design WITHOUT AI (procedural/algorithmic)

**Boundary note (one paragraph, as scoped):** this category deliberately excludes AI-model tooling — diffusion image generators (Midjourney/SD-class), LLM UI generators (v0/Galileo-class), and AI design copilots. The reason is structural, not ideological: dowiz's generative surfaces (OG cards, tenant textures, receipts) are *runtime pipelines* that must be deterministic, reproducible per seed, license-clean, zero-marginal-cost, and offline — properties a hosted model API cannot offer and a local model can't offer within a kB or RAM budget. Procedural = a seed and math; that is the whole point. AI tools remain fine as human design-time explorers; they are out of scope here and were not researched further.

### 3.1 Landscape map

Three tiers, and the tier determines fit:
- **Frameworks** (p5, Processing lineage, canvas-sketch, paper.js, two.js) — batteries-included sketching environments. All fail the client budget; some fail Node.
- **Primitives** (simplex-noise, d3-delaunay, poisson-disk, blobshape, rough.js, color libs, thi.ng packages) — tiny, 0–low dep, run identically in browser and Node. **This is where dowiz shops.**
- **Server rasterization** (sharp/resvg/satori/takumi) — where procedural SVG becomes PNG. dowiz already operates this tier.

### 3.2 Pattern/SVG generation (verified 2026-07-11)

| Tool | Size (gz) | License | Latest / published | Deps | Node/SSR | Verdict |
|---|---|---|---|---|---|---|
| blobshape | **1.0 kB** (tarball measure ~0.97 kB) | MIT | 1.0.0 / 2020-06-12 | 0 | ✅ pure string fn | Tiny + frozen + safe: vendor-grade |
| rough.js | 8.6 kB (bundlephobia) | MIT | 4.6.6 / 2023-11-20 | 4 | ✅ works on SVG strings/node | Stable-done; hand-drawn accents |
| trianglify | 20.7 kB | **GPL-3.0** (+paid commercial) | 4.1.1 / 2020-11-01 | 3 (incl. `canvas`!) | partial | **Skip: license + dormant** — reimplement via d3-delaunay if wanted |
| css-doodle | 36.2 kB (tarball measure) | MIT | 0.51.0 / 2026-04-19 | 0 | ❌ web-component; SSR no-ops | Design-time toy for this stack |
| svg-patterns | small | ISC | 2.0.1 / 2022-05-22 | — | ✅ | Stale but usable idea-source |
| geopattern | — | MIT | 1.2.3 / 2014-12-03 | 1 | ✅ | Dormant 11 years; vendor algorithm only |
| Hero Patterns | 0 (static SVG) | **CC BY 4.0** (attribution!) | — | — | ✅ | Fine w/ credit line; not generative |
| SVG `feTurbulence` grain | **0 kB** | — | native | 0 | ⚠ see below | **Top pick for brand grain — with one server caveat** |

**The feTurbulence caveat (directly relevant to dowiz's OG pipeline):** sharp rasterizes SVG via librsvg, whose SVG *filter-primitive* support is known-limited/buggy (`lovell/sharp` [#804](https://github.com/lovell/sharp/issues/804)). Grain baked as `<feTurbulence>` inside an SVG fed to sharp may render wrong or not at all. Safe patterns: pre-bake the grain once as a PNG/WebP texture asset and composite with sharp, or rasterize via resvg instead (its feTurbulence coverage: UNVERIFIED — test first). Client-side, feTurbulence grain is fully supported and free ([CSS-Tricks "Grainy Gradients"](https://css-tricks.com/grainy-gradients/), [fffuel gggrain](https://www.fffuel.co/gggrain/)).

### 3.3 CSS Houdini Paint API — 2026 reality: dead end

caniuse (live): Chromium-only since Chrome 65 (~76 % share via Chromium), **Firefox still "under consideration" — unchanged for years; Safari disabled by default**. web.dev's Houdini primer last updated 2020-12-10; the advocacy ecosystem (houdini.how) is quiet; ishoudinireadyyet.com liveness UNVERIFIED (DNS failed in-sandbox; source repo exists unarchived). Also architecturally moot for dowiz: worklets run only in the browser engine — useless to the Node OG pipeline. **Verdict: eight-plus years post-spec, Paint API is a Chromium-only progressive enhancement. Skip** ([caniuse.com/css-paint-api](https://caniuse.com/css-paint-api), [web.dev/articles/houdini-how](https://web.dev/articles/houdini-how)).

### 3.4 Color science for generative palettes (momentum comparison)

| Lib | gz | License | Latest / published | Deps | Note |
|---|---|---|---|---|---|
| **culori** | 21.7 kB full; **tree-shakes via `culori/fn`** (default entry has side-effectful space registration — must use `/fn` to shrink) | MIT | 4.0.2 / 2025-06-27 | 0 | OKLCH/OKLab/APCA; the practical pick ([tree-shaking guide](https://culorijs.org/guides/tree-shaking/)) |
| colorjs.io | ~24.8 kB (fn.min measured) | MIT | 0.6.1 / 2026-01-15 | 0 | Verou/Lilley reference-grade; still pre-1.0 after years |
| chroma-js | 15.9 kB | BSD-3-Clause (repo LICENSE; npm shows combined tag from a bundled dep note) | 3.2.0 / 2025-11-28 | 0 | Same original author, still active; largest install base (~2.65M/wk) |
| **@texel/color** | 9.0 kB (bundlephobia; ~3.5 kB min claimed tree-shaken) | MIT | 1.1.11 / 2026-01-07 | 0 | mattdesl; fast OKLab gamut-mapping; leanest wide-gamut option |
| @thi.ng/color | (part of thi.ng, see §3.6) | Apache-2.0 | 5.8.30 / **2026-07-02** | 13 (internal) | Deepest color math; thi.ng cadence |
| apcach | — | MIT | 0.6.4 / 2023-11-13 | — | Contrast-targeted OKLCH — great *concept* for tenant a11y ramps; stale |
| **Native CSS** | **0 kB** | — | — | — | `color-mix()` 91.2 % global, relative color syntax ~88.4 % (caniuse live) — viable for client-side tenant theming NOW |

**Fit note for per-tenant theming:** the server must still precompute concrete sRGB/hex values for anything entering the SVG→sharp pipeline (librsvg/resvg won't reliably resolve `oklch()`/`color-mix()` — UNVERIFIED exact coverage, assume no). So the winning split is: **culori (`/fn` imports, ~3–6 kB of functions) or @texel/color server-side to derive tenant ramps once → emit plain hex custom properties; native CSS `color-mix()` client-side for micro-variations.** Zero client JS attributable to color.

### 3.5 Server-side generative pipelines (dowiz already lives here)

| Tool | License | Latest / published | Signal |
|---|---|---|---|
| **sharp** | Apache-2.0 | **0.35.3 / 2026-07-01** | ~70M weekly downloads; dominant; keep as the raster backbone |
| @resvg/resvg-js | **MPL-2.0** | 2.6.2 / 2024-03-26 (2.7.0-alpha.2 2026-01-28) | Spec-faithful Rust SVG; **silently skips `<text>` if no font loaded** — gotcha; slow stable cadence |
| canvas (node-canvas) | MIT | 3.2.3 / 2026-03-31 | Revived, but node-gyp/Cairo pain persists |
| **@napi-rs/canvas** | MIT | **1.0.2 / 2026-06-30** | Skia + prebuilt binaries; **13.2M wk downloads — has overtaken node-canvas (7.3M)**; the 2026 default if canvas API is ever needed |
| **satori** (Vercel) | **MPL-2.0** (not MIT) | 0.26.0 / 2026-03-20 | HTML/JSX-ish tree → SVG with flexbox+text shaping; **works without React** (plain `{type,props}` trees); 1.77M wk; satori-html helper frozen since 2022-12 |
| **takumi** (`@takumi-rs/*`) | **MIT OR Apache-2.0** | 2.0.3 / **2026-07-11** (published the day of this research) | Rust; HTML/CSS → PNG directly, skips the SVG hop; claimed 2–10× faster than satori+resvg; adopted by Nuxt OG Image v6; young but the momentum story of 2026 |

**Verdict for the OG-card generator:** dowiz's hand-authored SVG → sharp pipeline remains correct and sovereign — **no change needed**. The upgrade path, *only if* OG cards grow layout complexity (multi-line trilingual text, RTL, wrapping): add **satori** (mature, no-React usage verified in docs) or **takumi** (permissive dual license, faster, younger) as a layout front-end, keeping sharp for final compositing/format. Both are server-side; client cost 0. Sources: npm registry 2026-07-11; [github.com/vercel/satori](https://github.com/vercel/satori); [takumi docs](https://takumi.kane.tw/docs); [sharp #804](https://github.com/lovell/sharp/issues/804).

### 3.6 QR aesthetics (receipt/table-tent surfaces)

- **qrcode** (node-qrcode): MIT, 1.5.4 (2024-08-05), ~16.9M wk downloads; emits plain SVG strings server-side — **the base layer; already the right choice**. Styling approach: post-process its matrix/SVG output with a small in-house pass (rounded modules, amber-on-near-black quiet zone, center glyph) — keeps the bebop aesthetic without new deps.
- qr-code-styling: MIT, 1.9.2 (2025-04-11), 13.8 kB gz — canvas-first, needs jsdom/node-canvas server-side; community fork `qr-code-styling-node` exists. **Heavier than the job requires.**
- Canvas-free server alternatives if styling-by-hand is rejected: EasyQRCodeJS-NodeJS, `@qrgrid/server` (styled SVG strings, no DOM) — UNVERIFIED maintenance depth, check before adopting.

### 3.7 Creative-coding frameworks (verified 2026-07-11)

| Framework | gz | License | Latest / published | Momentum | Node/SSR | Verdict |
|---|---|---|---|---|---|---|
| **p5.js 2.x** | **322.7 kB** (bundlephobia p5@2.3.0) | **LGPL-2.1** | 2.3.0 / 2026-05-28 (2.0.0 shipped 2025-04-17; 1.11.13 still patched 2026-04) | Very active (pushed 2026-07-11); WebGPU renderer experimental in 2.2; deps incl. zod, colorjs.io, acorn | ❌ window-dependent, no first-party headless | Sketchbook, not a shipping dependency: 5× the largest route budget, LGPL diligence item, Node-hostile. **Prototype in it, ship the algorithm without it** |
| canvas-sketch | n/a (build-time CLI) | MIT | 0.7.8 / 2026-05-26 | Perma-"beta", sporadic compat bumps | ✅ its core use (node-canvas/headless export) | Legitimate *build-time* texture pre-renderer; never a runtime dep |
| paper.js | 84.2 kB | MIT (LICENSE.txt; GitHub SPDX shows NOASSERTION) | 0.12.18 / 2024-07-17; repo quiet since 2024-07 | ~2 yrs stale, 430 open issues | ⚠ paper-jsdom shims, SSR bundling friction ([#1483](https://github.com/paperjs/paper.js/issues/1483)) | Best-in-class béziers/booleans, wrong decade for this budget. Skip |
| **two.js** | 47.6 kB | MIT | 0.8.23 / 2025-12-22; pushed 2026-06-30 | Alive (jonobr1) | ⚠ node-canvas path documented; SVG renderer string-oriented | Only framework that is both alive and sub-60 kB — still eats an entire 60 kB tier alone. Niche pick, not default |
| Zdog | **7.3 kB** (measured tarball, confirmed twice) | MIT (package.json) | 1.1.3 / 2022-01-22; repo push 2023-07-18 | Dormant (all of Metafizzy quiet) | ⚠ canvas/SVG; SVG path usable in Node UNVERIFIED | Charming flat pseudo-3D at a tiny cost; treat as frozen — vendor/pin if ever used for a brand-asset one-off |
| rough.js | 8.6 kB | MIT | 4.6.6 / 2023-11-20; repo push 2024-07-28 | "Stable-done"; **38M downloads/mo** (Excalidraw transitive) | ✅ excellent — generator emits SVG path data, no DOM | **Adopt-grade**: sketchy accents on-brand cheap, works in the OG pipeline |

### 3.8 Noise/math/physics primitives (the actual toolkit)

| Primitive | gz | License | State | Verdict |
|---|---|---|---|---|
| **simplex-noise** | **1.8 kB** | MIT | 4.0.3 / 2024-07-26 — algorithm-complete, 1.24M dl/mo | **Adopt when needed; effectively free.** fBm = ~10 hand-rolled octave lines on top |
| **poisson-disk-sampling** | 2.3 kB | MIT | 2.3.1 / 2022-06 — stable, 1 open issue | Adopt for even scatter (texture dots, particle seeding) |
| **d3-delaunay** | 6.9 kB (+delaunator) | ISC | 6.0.4; repo pushed 2025-11 — 71M dl/mo | Adopt for Voronoi/triangulated textures — the license-clean trianglify replacement |
| lindenmayer (L-systems) | ~6.3 kB (unminified UMD, measured) | MIT | 1.5.4 / 2020 — dormant-stable | Fine frozen; or hand-roll (an L-system rewriter is ~30 lines) |
| Boids/flocking | — | — | No maintained lib exists (dormant 2016-era packages) | Hand-roll (~50–80 lines) if ever wanted |
| matter-js | 25.3 kB | MIT | 0.20.0 / 2024-06; repo quiet since 2024-08 | De-facto 2D physics standard but eats the 25 kB tier alone; only for a dedicated playful surface |
| planck | 45.7 kB (measured + bundlephobia agree) | MIT | 1.5.0; active 2026-04 | **"Lighter than matter" is marketing — it measured 1.8× heavier.** Skip |
| @dimforge/rapier2d | **~424 kB gz WASM** (measured) | Apache-2.0 | Very active | Superb engine, absurd for this product. Skip |

### 3.9 thi.ng/umbrella — the deep one, honestly

Apache-2.0 across all sub-packages; ~350-project monorepo, 215+ actively published, maintained since 2015 by **one person** (Karsten Schmidt/postspectacular, sponsor-funded) — a real, acknowledged bus-factor risk. Cadence is extreme (repo pushed 2026-07-08; `@thi.ng/geom` shipped 8 versions between 2026-04-18 and 2026-07-02; `@thi.ng/color` 5.8.30 published 2026-07-02).

Measured sizes (esbuild --minify + gzip, this research): `@thi.ng/random` **2.2 kB** full; `@thi.ng/color` 22.3 kB full → **7.3 kB** realistic slice; `@thi.ng/geom` **46.7 kB full** (composite aggregator!) → 8.9 kB for a 3-function slice; `@thi.ng/hiccup-svg` **5.3 kB** (pure data→SVG-string, zero DOM). **Honest correction to the "thi.ng is tiny" reputation: true for leaf packages, false for composites unless imports are disciplined.**

Node/SSR fit is the best of anything surveyed — random/color/geom/hiccup-svg are pure data transforms, no `window`, verified by standalone bundling. The API idiom (transducers, point-free) is a genuine ramp-up cost; the project itself concedes no "getting started" path exists beyond ~185 examples.

**Verdict: conditionally production-safe — adopt narrowly and version-pinned (random + hiccup-svg + slices of color/geom) for *server-side* texture math and OG-card SVG assembly, where its DOM-freeness is exactly right and churn is contained by the lockfile. Never as an app framework or client dependency.** The philosophical alignment with dowiz's sovereign-engineering ethos is real, but philosophy doesn't waive the bus-factor: everything it would do client-side, 10-line hand-rolled functions do for free.

Sources for §3.7–3.9: agent measurements July 2026 (npm pack / esbuild --minify / gzip -9, cross-checked against bundlephobia API where available — zdog and planck numbers matched independently to the byte); npm registry API 2026-07-11; [github.com/processing/p5.js releases](https://github.com/processing/p5.js/releases/tag/v2.0.0); [github.com/thi-ng/umbrella](https://github.com/thi-ng/umbrella); [github.com/mattdesl/canvas-sketch](https://github.com/mattdesl/canvas-sketch); [github.com/liabru/matter-js](https://github.com/liabru/matter-js); [github.com/piqnt/planck.js](https://github.com/piqnt/planck.js/); [github.com/dimforge/rapier.js](https://github.com/dimforge/rapier.js/).

**Plotter/pen-art (brief, as scoped):** the procedural-art community's energy in 2026 lives in physical plotting (vpype CLI, AxiDraw→NextDraw hardware, Drawingbots Discord, PlotterFiles) rather than general JS libs — a healthy signal that the *techniques* (Poisson scatter, flow fields, L-systems) are alive and framework-independent, which is exactly how this report recommends consuming them ([awesome-plotters](https://github.com/beardicus/awesome-plotters)).

---

## 4. Fit matrix — dowiz use-cases → recommendation + fallback + gz cost

| dowiz use-case | Recommended | Fallback | Est. client gz added | Notes |
|---|---|---|---|---|
| **Landing hero** (HorizonDrift-class cinematic, Astro island) | Hand-rolled WebGL2 quad shader (measured **1.5 kB**) with static-gradient no-WebGL fallback | OGL subset (**~14–20 kB**, Unlicense) if it grows into meshes/passes | 1.5 → 20 kB | Fits the 25 kB landing target *with* room left; IntersectionObserver pause + `prefers-reduced-motion` freeze included in the measured module |
| **Storefront polish** (`/s/:slug` — tenant world, subtle) | CSS/SVG only: `feTurbulence` grain, animated gradients, blobshape-derived shapes | Tiny OGL/simplex particle island, lazy, below-fold | **0** → ~18 kB | Storefront is the tenant's brand space (BRAND-BIBLE §1) — restraint is policy, and the 21.6 kB base leaves ~3 kB headroom at the 25 kB target anyway |
| **Per-tenant generative theming** | Server-side: culori `/fn` (or @texel/color) derives oklch→hex ramps from tenant seed color → plain CSS custom properties | @thi.ng/color slice; client `color-mix()` for micro-variation | **0 client** (~3–9 kB server) | Deterministic per tenant-id seed; a11y-checked ramps (APCA in culori) |
| **OG cards** (server, sharp SVG→PNG — exists) | Keep current pipeline; compose with simplex-noise + blobshape + rough.js + @thi.ng/hiccup-svg for richer procedural cards | satori (MPL-2.0, no-React mode) or takumi (MIT/Apache-2.0) if HTML-flex layout becomes needed | **0 client** | Beware feTurbulence-in-librsvg (§3.2 caveat): pre-bake grain textures, composite in sharp |
| **TUI/brand assets** (posters, receipts, QR, textures) | Build-time: canvas-sketch or plain Node scripts + the same primitives kit; QR = `qrcode` SVG + in-house styling pass | Zdog (7.3 kB, vendored) for a one-off pseudo-3D brand object | 0 (build-time) | Assets committed to repo; zero runtime deps |
| **Loading states / skeletons** | Pure CSS (animated gradient + grain layer already in tokens) | rough.js-drawn placeholder strokes (8.6 kB, only on a surface that already ships it) | 0 | Never spend JS on waiting |
| **Full 3D scene** (hypothetical future) | three WebGL path + Threlte 8, lazy below-fold island outside route budgets | PlayCanvas engine-only (if editor workflow ever wanted) | ~130–210 kB (deferred chunk) | Not before a concrete product need exists |

## 5. Skip list — popular things NOT worth adopting here

| Skip | Why (one line) |
|---|---|
| **three.js on the landing page** | 129 kB gz *minimal* measured — 5× the whole route target; reserve for a true 3D-scene future |
| **p5.js in production** | 322.7 kB gz, LGPL-2.1 diligence, window-bound — a sketchbook, not a dependency |
| **lygia** | Prosperity/Patron license = not free for commercial use; re-derive the 30 lines of noise math instead |
| **trianglify** | GPL-3.0 + dormant since 2020 + drags node-canvas; d3-delaunay rebuilds it license-clean |
| **Tailwind/Panda/StyleX migration** | Zero-runtime is great, but dowiz's token layer already achieves it; a toolchain swap buys authoring taste, not user value |
| **Radix (via ports) / any React headless** | Ecosystem just demoted it (shadcn→Base UI 07-2026); and React primitives don't run in Svelte anyway — Bits UI exists |
| **Ark UI Svelte as default** | Works, but 67-package Zag dep tree vs Bits UI's 6 deps for the same widgets |
| **css-doodle** | 36 kB client-only web component that can't SSR — wrong side of the wire for a generative-texture need the server already covers |
| **Houdini Paint worklets** | Chromium-only after 8 years, Firefox "considering", dormant ecosystem |
| **vanta.js / threejs-toys / finisher-header** | Stale + global-three (150 kB+ real cost) / unclear licenses / proprietary republish |
| **planck.js as "light physics"** | Measured 45.7 kB gz vs matter-js 25.3 kB — the "lighter" claim is false in practice |
| **rapier (WASM) client-side** | 424 kB gz WASM kernel; magnificent, and 17× the largest budget |
| **satori as a default** | MPL-2.0 is fine and it works React-free, but adopting it *before* OG layout complexity exists is 11 deps of speculative generality |
| **gpu-curtains / WebGPU-only anything** | ~16 % of users still have no WebGPU mid-2026; pre-1.0 libs, Firefox unresolved — author for WebGL2 now |

## 6. Verification method + unresolved items

- **Three independent evidence classes** were used: (1) npm registry API (`registry.npmjs.org/<pkg>`) for version/publish-date/license/deps — 70+ packages queried directly during this session; (2) bundlephobia API for min+gz where it would build; (3) direct measurement — npm tarball dist files and freshly bundled subsets via esbuild `--minify` + `gzip -9` — wherever bundlephobia 429'd/failed. Convergence checks passed where classes overlapped (zdog 7,274 B and planck 45.7 kB matched to the byte/decimal across two independent measurers; ogl/culori/roughjs/simplex bundlephobia numbers matched registry-era expectations).
- **Known conflicts / UNVERIFIED left standing:** Firefox WebGPU default-on status (blogs vs caniuse table disagree — treated as partial); resvg feTurbulence coverage (untested); drei post-10.7.7 alpha channel details; ishoudinireadyyet.com liveness; Zdog SVG-in-Node path; @qrgrid/server maintenance depth; Bits UI per-component gz (bundlephobia cannot build Svelte packages — measure inside the rebuild's own Vite build when relevant).
- Astryx, Base-UI-as-shadcn-default, takumi 2.0.3 (published 2026-07-11), lygia licensing, and OGL's Unlicense were each **re-verified first-hand** (live fetches of repo/changelog/registry) rather than taken from search summaries.

