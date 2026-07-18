# OPUS-R1 — Interface & Rendering research for the full-wgpu, intent-only, no-DOM UI (2026-07-18)

> **Research document — writes no product code.** Commissioned against the binding decisions in
> `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.30/§16.34/§16.35/§16.39/§16.40/
> §16.42–§16.44/§16.50–§16.51/§16.55–§16.56/§17.5 (all treated as **closed ground truth** — not
> re-litigated). Builds forward from `BLUEPRINT-P38-webgpu-render-engine.md` (the existing render
> engine: `compose()` oracle, Scene/SDF pipelines, FE-15 hidden-DOM a11y mirror, FE-16 fallback
> ladder) and `BLUEPRINT-P51-open-map-routing.md` (the a11y-mirror precedent for one surface).
> Every external claim carries a live 2026 source URL. Where no precedent exists, this document
> says so plainly rather than inventing one.

---

## 0. The binding decisions this report designs *around* (not toward)

Restated so every recommendation below can be checked against them:

1. **Full wgpu everywhere** — menu, checkout, admin, courier, and the `dowiz.org` landing page all
   render through the field engine; DOM survives only as the invisible a11y mirror + (currently)
   an input overlay (§16.30, §16.56; P38 §1).
2. **Custom in-canvas text input, no HTML `<input>` overlay hybrid** — cursor, selection,
   clipboard, IME for non-Latin scripts, all hand-built (§16.34).
3. **Intent-driven UI, full replacement from day one, no button/menu fallback** — schedule risk
   knowingly accepted (§16.35, §16.40).
4. **Physics/math-generated typography** — glyphs from field/wave math, not font files; flagged by
   the operator himself as R&D with no known precedent (§16.39).
5. **Friction as field dynamics** — amplitude/intensity/color/rhythm, not confirm dialogs;
   multi-modal, fully audio-accessible (§16.44, §16.50).
6. **a11y-mirror on every screen, including live text-editing state** (§16.30, §16.34).
7. **SEO/AEO via bot-facing static files** (robots/manifest/sitemap/JSON-LD/llms.txt-style),
   explicitly *distinct* from the a11y mirror, zero rendered markup (§16.55).
8. **One UI paradigm for all devices**, no legacy fallback mode, but real optimization discipline
   ("без тяжких бібліотек") so budget hardware runs it (§16.51).
9. **AR/VR readiness designed in now** (§17.5).
10. ***Ad fontes*** — math/physics primitives over UI-library dependencies, **scoped to the
    UI/rendering/interaction layer only**; crypto/protocol/storage keep their vetted crates (§16.42–§16.43).

**One internal contradiction surfaced by this research, flagged for Tier-3 (not resolved here):**
§16.34 forbids an HTML `<input>` overlay, but the existing P38 blueprint (G6 / FE-16,
`BLUEPRINT-P38-webgpu-render-engine.md` §3.6) *plans* a "transparent `<input>` (type=email/tel
preserved) for IME/autofill/mobile keyboard." These cannot both stand. The overlay is precisely
how the industry gets IME + mobile soft-keyboard + native caret accessibility "for free"; removing
it (per §16.34) makes non-Latin IME the single hardest unsolved sub-problem in the whole program
(Area 3 + Area 2 below). This is Risk #1 in §9.

---

## 1. State of the art: Rust wgpu UI frameworks for a full application UI

**Production-viable wgpu-based UI stacks in 2026 (all verified this pass):**

| Stack | What it is | Web (WASM) | Desktop | Mobile | a11y | Fit for dowiz |
|---|---|---|---|---|---|---|
| **iced** ([iced.rs](https://iced.rs/), [lib.rs/iced_wgpu](https://lib.rs/crates/iced_wgpu)) | Elm-arch retained GUI on `wgpu` | yes | production | experimental | partial | Rejected by P38 §2 (retained-widget framework re-imports the abstraction the field UI replaces) |
| **egui / eframe** ([github.com/emilk/egui](https://github.com/emilk/egui), [egui-wgpu](https://docs.rs/egui-wgpu/)) | Immediate-mode GUI on `wgpu`; **AccessKit built into eframe by default** | yes | production | partial | **AccessKit (native only)** | Rejected by P38 §2; but its AccessKit integration is the reference pattern (Area 2) |
| **GPUI** ([gpui.rs](https://www.gpui.rs/), [zed.dev/blog/videogame](https://zed.dev/blog/videogame)) | Zed's hybrid immediate+retained, GPU-direct UI; **the entire Zed editor renders through it** | no (native) | **shipping weekly on macOS/Linux/Windows** | no | via platform | **The strongest real-world proof** that a full, text-heavy application UI can be GPU-rendered from scratch with no browser engine — 120 FPS target, glyph-atlas text, **2 ms keystroke-to-pixel latency** vs 12–25 ms in VS Code/Cursor. Pre-1.0. Mine for technique, do not adopt. |
| **Vello / Vello Hybrid** ([github.com/linebender/vello](https://github.com/linebender/vello)) | GPU-compute 2D vector renderer (Skia/Cairo class) on `wgpu`; backs the Xilem toolkit | yes | yes | yes | via toolkit | **Alpha** (conflation artifacts, blur/filters WIP). The reusable-technique reference for GPU vector/text if SDF ever proves insufficient (Area 4) |
| **Slint** | Declarative DSL, commercial license option | yes | production | yes | yes | Off-canon (its own DSL/runtime is exactly the superstructure *ad fontes* rejects) |

**Verdict — production-ready vs R&D:** GPU-rendered full application UI in Rust is **production-real
in 2026**, proven at the hardest end (a code editor: dense text, live editing, 120 FPS) by GPUI
shipping across three desktop OSes. What is *not* off-the-shelf is dowiz's specific shape: a
**field-engine-native, no-retained-widget, no-DOM** UI. None of iced/egui/Slint fit it (each
re-imports a widget abstraction P38 §2 explicitly rejects), and GPUI/Vello are pre-1.0 and
architecturally their own thing.

**Wave-0 recommendation:** Keep building on dowiz's **own** engine substrate — `compose()`,
`Scene`/`SdfShape`, `WidgetStore`, `zerocopy`, `VertexBridge` (P38 §0, all landed + tested) — which
is already the correct *ad fontes* shape. Do **not** adopt a framework. **Do** treat **GPUI** as the
canonical technique reference (glyph-atlas text caching, hybrid immediate/retained bookkeeping,
input-latency budget) and **Vello** as the fallback vector-text engine if the MSDF path (Area 4)
hits legibility limits. This matches P38's already-chosen direction; nothing here reopens it.
*Sources:* [iced.rs](https://iced.rs/) · [github.com/emilk/egui](https://github.com/emilk/egui) ·
[zed.dev/blog/videogame](https://zed.dev/blog/videogame) · [gpui.rs](https://www.gpui.rs/) ·
[github.com/linebender/vello](https://github.com/linebender/vello) ·
[ideaverse.ai — State of Rust Interfaces 2026](https://ideaverse.ai/blog/are-we-gui-yet-the-state-of-building-rust-interfaces-in-2026-mqcix2yy).

---

## 2. Accessibility bridging for a GPU canvas: AccessKit and the a11y-mirror

**AccessKit** ([accesskit.dev](https://accesskit.dev/), [github.com/AccessKit/accesskit](https://github.com/AccessKit/accesskit))
is the real, canonical Rust crate for this problem: a cross-platform schema for an accessibility
tree (each node has an integer id, a role — `button`/`label`/`textInput`/`status` — and optional
attributes) plus a trait for handling AT-requested **actions** (move focus, invoke, **select text**).
It is exactly the "parallel accessible tree kept in sync with a self-rendered UI" pattern §16.30
names.

**Production status (verified this pass):**
- **Ready:** `accesskit_windows`, `accesskit_macos`, `accesskit_unix` (AT-SPI), `accesskit_android`,
  `accesskit_winit`. egui/eframe ship it by default — the first immediate-mode GUI with real
  platform a11y ([egui PR #2294](https://github.com/emilk/egui/pull/2294)).
- **NOT ready:** the **web/canvas adapter is planning-only** and self-described as "**probably most
  difficult of all**," with no timeline, funding-dependent
  ([accesskit.dev/looking-back-looking-forward](https://accesskit.dev/looking-back-looking-forward/);
  reconfirmed 2026 as "planned but not yet available"). iOS adapter likewise planning-only.

This **validates P38's decision** (§0/§2/§3.6) to hand-roll a hidden-DOM semantic mirror on web
rather than wait for `accesskit`-web. The engineering split the research supports:

- **Native (Tauri/winit desktop + Android):** use the **AccessKit crate directly**. It is
  production-ready on exactly those targets; there is no reason to hand-roll there.
- **Web (WASM):** hand-rolled **hidden-DOM semantic mirror** (P38's FE-15 mechanism) is currently
  the *only* option and remains so until an `accesskit` web backend ships. Reconcile the mirror from
  the widget list, piggybacking the FE-14 settle gate so a dormant UI has a dormant mirror (P38
  §3.6 already specifies this).

**Keeping the mirror in sync with *live text-editing* state** is the genuinely harder half (§16.34's
new requirement). AccessKit's schema already models this — text-run nodes, a **text-selection**
attribute (anchor + focus positions), and a `SetTextSelection` action — so on native the cursor and
selection map onto real schema fields and screen readers announce caret movement/selection changes
natively. **On web there is no equivalent for a canvas:** a screen reader reads a hidden `<div>`,
not a canvas caret. Two sub-options, both imperfect, for Tier-3 to choose between:
  1. **Hidden contenteditable/`<input>` element** mirroring the edit buffer — gives IME + caret a11y
     + mobile keyboard **for free**, but is exactly the overlay §16.34 forbids (the §0 contradiction).
  2. **Fully synthetic ARIA `textbox` mirror** — a hidden element with `role=textbox`,
     `aria-activedescendant`/`aria-selection` updated per keystroke — honors §16.34's "no `<input>`"
     but must **manually re-implement** IME composition events and cursor-position announcements that
     the browser otherwise provides, with weaker and more screen-reader-dependent results.

**Wave-0 recommendation:** (a) adopt **AccessKit on native immediately** — it is free, correct, and
covers cursor/selection out of the box. (b) On web, ship the **hand-rolled hidden-DOM mirror** P38
already specifies, reconciled per-change; but escalate the **text-editing-state-on-web** question to
Tier-3 as a first-class design item, because it collides head-on with §16.34 (Risk #1, §9). (c)
Test the mirror the way P51 §4.7 and P38 §3.6 already do — Playwright accessibility-tree snapshots
asserting role/name/state, plus a keyboard-order pass — and **extend that harness to every screen**,
including a live-editing scenario (type into a canvas field, assert the mirror's caret/selection
node updates). *Sources:* [accesskit.dev](https://accesskit.dev/) ·
[accesskit.dev/how-it-works](https://accesskit.dev/how-it-works/) ·
[looking-back-looking-forward](https://accesskit.dev/looking-back-looking-forward/) ·
[egui PR #2294](https://github.com/emilk/egui/pull/2294).

---

## 3. Custom in-canvas text input from scratch (cursor / selection / clipboard / IME)

This is, as §16.34 concedes, one of the hardest sub-problems in UI engineering. The good news from
the research: **the hardest *algorithmic* parts are not greenfield** — two mature pure-Rust crates
solve shaping, bidi, grapheme clustering, cursor motion, and selection, and both are designed to be
paired with your own GPU renderer:

- **cosmic-text** ([github.com/pop-os/cosmic-text](https://github.com/pop-os/cosmic-text)) — pure
  Rust multi-line text: shaping (HarfRust), font discovery (fontdb), fallback, layout, rasterization
  (swash), and a real **editing** layer: cursor management, **selection, copy/paste**, bidirectional
  text, ligatures, color emoji, wrapping, optional vi-style commands (modit). Battle-used in the
  COSMIC desktop. ([DeepWiki overview](https://deepwiki.com/pop-os/cosmic-text))
- **parley** ([Linebender](https://linebender.org/)) — now uses **HarfRust**, giving
  "production-quality shaping for all scripts"; ships a `PlainEditor` with selection and IME hooks.
  The trade-off noted in the wild (via the [egui Parley PR #5784](https://github.com/emilk/egui/pull/5784))
  is that Parley wants to own a full rectangle to lay text into, which can fight an
  immediate-mode/field model — worth prototyping against dowiz's `WidgetStore` layout before
  committing.

Either gives you the text **model** (buffer, cursor, selection, shaped runs, hit-testing) while you
keep dowiz's **GPU glyph rendering** (MSDF/SDF via FE-06). This is the correct *ad fontes*-compatible
split: a text-shaping crate is not a UI-library superstructure — it is a first-principles primitive
(Unicode + font shaping) in the same category §16.43 keeps for crypto. Reimplementing HarfBuzz-class
shaping and Unicode bidi from scratch would be a genuine safety/correctness regression, not
simplification.

**What *is* still hard / genuinely custom:**
- **IME composition** for CJK/Thai/Indic/etc. On **native**, `winit` surfaces IME events
  (`Ime::Preedit`/`Commit`) and the a11y layer is AccessKit — tractable. On **web**, browser IME
  (`compositionstart`/`update`/`end`) fires only against a **focused editable DOM element**; a bare
  canvas cannot receive composition events. This is the crux of the §16.34-vs-reality tension: honest
  full non-Latin IME in a canvas on web effectively **requires** a hidden editable element to host
  composition, then mirroring its committed text into your buffer. Removing the overlay (per §16.34)
  means either accepting no-IME on web at Wave-0 for non-Latin, or building a bespoke soft-IME
  candidate UI inside the canvas driven by raw `keydown` — a large, unprecedented sub-project.
- **Clipboard** — native via `winit`/`arboard`; web via the async Clipboard API (needs a user
  gesture; works without DOM widgets).
- **Mobile soft keyboard** — no reliable way to raise it without a focused editable element on web;
  native mobile needs explicit `show_soft_input` calls.

**Wave-0 recommendation:** Build the canvas text editor on **cosmic-text** (more mature editing
surface today) or **parley** (better all-script shaping; prototype the rectangle-ownership friction
first) for buffer/cursor/selection/shaping — **greenfield only for the GPU render and the input
event wiring**, not for the linguistics. Scope **Latin-script Wave-0 text input as fully in-canvas
and achievable**; scope **non-Latin IME on web as an explicit Tier-3 research spike** with a named
fallback (a hidden composition host, decided against §16.34 only by the operator). Native IME is
tractable now. *Sources:* [github.com/pop-os/cosmic-text](https://github.com/pop-os/cosmic-text) ·
[deepwiki.com/pop-os/cosmic-text](https://deepwiki.com/pop-os/cosmic-text) ·
[Linebender / parley](https://linebender.org/) ·
[egui PR #5784 (Parley)](https://github.com/emilk/egui/pull/5784).

---

## 4. Procedural / math-generated typography (§16.39) — honest precedent check

**Finding, stated plainly: there is NO production or academic precedent for generating legible
glyphs from field/wave equations.** The operator's own framing ("no known production precedent") is
correct. The closest real work is *parametric* and *neural* type design, all of which is
outline/curve-based or interpolation-based, not field/wave-based:

- **METAFONT / MetaPost** (Knuth, 1980s) — the original parametric type system: glyphs as
  **pen-stroke and Bézier programs** with variable parameters. Real, legible, still used; but it is
  procedural *geometry* (curves under parameters), **not** glyphs emerging from a Laplacian/wave
  field. ([Metafont overview](https://grokipedia.com/page/Metafont))
- **"Parametric type design in the era of variable and color fonts"** (Thottingal, Jan 2025,
  [arxiv 2502.07386](https://arxiv.org/abs/2502.07386)) — modern MetaPost-based parametric workflow;
  two real variable fonts shipped open-source. Still outline/parameter based.
- **"Differentiable Variable Fonts"** (Oct 2025 / rev. Mar 2026, [arxiv 2510.07638](https://arxiv.org/abs/2510.07638))
  — makes variable-font **parameter→outline** mappings differentiable; demonstrates "physics-based
  text animation" and gradient font-design optimization. **Critically: the glyphs come from existing
  font outlines**, interpolated — not synthesized from equations. The "physics" is animation of
  outline control points, not glyph *generation* from a field.
- **Neural variable-font generation** (e.g. [NIV, arxiv 2606.05261](https://arxiv.org/abs/2606.05261))
  — operates on **vector glyph geometry**, trained on >1M Google-Fonts variation tuples; outputs
  standard variable-font files. Still outlines, now learned.

None of these is "draw the letter *A* as the steady state of a wave equation." That remains genuinely
unprecedented R&D — which is exactly how §16.39 records it.

**The established, GPU-native, *ad fontes*-compatible Tier-1 answer** is **SDF/MSDF text from real
font files** — precisely what P38 G3/FE-06 already plans. MSDF is resolution-independent, preserves
sharp corners (its whole reason to exist over plain SDF), and has "little overhead at runtime" once
the atlas is built. Documented caveats to design around: **thin faces and very small sizes** are
where SDF/MSDF fidelity degrades, and atlas generation has an offline cost.
([Red Blob Games SDF/MSDF guide](https://www.redblobgames.com/articles/sdf-fonts/),
[MSDF in Metal](https://medium.com/@sihaolu/performant-crisp-text-rendering-in-metal-with-multi-channel-signed-distance-field-msdf-9acd634d0052))
If MSDF's small-size legibility proves inadequate, **Vello** (Area 1) is the GPU **vector** text
fallback that renders true outlines with no atlas.

**Wave-0 recommendation (a defensible two-track structure, not a walk-back of §16.39):**
- **Tier-1 (Wave-0 critical path):** real font files rendered via **MSDF** (P38 FE-06), with **Vello
  vector text** as the pre-named escape hatch for small-size/thin-face legibility. This is
  production-proven and unblocks every text surface today.
- **Track-R (parallel, off critical path):** the procedural-glyph R&D. Sequence it honestly:
  **step 1 = parametric (MetaPost-style) glyph programs** whose parameters ride the same design-token
  UBO (a real, precedented stepping stone); **step 2 = field/wave-generated glyphs** (the
  unprecedented goal). Gate: no order-flow screen may depend on Track-R output; a Track-R glyph ships
  only after it passes the same legibility/contrast bar as the MSDF Tier-1 (an objective a11y
  contrast gate, not taste). This keeps §16.39's ambition alive without letting it block the first
  real order (the exact hedge §16.35 asks for). *Sources:* [arxiv 2502.07386](https://arxiv.org/abs/2502.07386) ·
  [arxiv 2510.07638](https://arxiv.org/abs/2510.07638) · [arxiv 2606.05261](https://arxiv.org/abs/2606.05261) ·
  [redblobgames.com/articles/sdf-fonts](https://www.redblobgames.com/articles/sdf-fonts/).

---

## 5. Local / offline voice recognition on courier phones (§16.34, §16.53)

The 2026 landscape has moved decisively past whisper-as-default for **real-time** voice:

- **whisper.cpp** — still the standard, known-good, offline, **multilingual** engine, and the safe
  baseline. But it is **batch-architected**: it processes a fixed 30-second window regardless of
  utterance length, so streaming latency is poor (benchmarks cite ~**11.3 s** for large-v3 on a live
  clip). Use small/tiny models to cut that, at accuracy cost.
- **Moonshine** ([github.com/moonshine-ai/moonshine](https://github.com/moonshine-ai/moonshine)) —
  purpose-built for **live, low-latency, edge** speech: **~107 ms** latency vs ~11 s for Whisper
  large-v3 on the same hardware, **245M params (~6× smaller** than Whisper large-v3), processes only
  the audio you give it, words appear as you speak. **The 2026 default for real-time voice
  interfaces.** Caveat to verify: Moonshine's strength is **English**; multi-language coverage is
  thinner than Whisper's — a real gap for dowiz's multi-language requirement (§16.20).
- **Others:** **Parakeet** (very low WER, edge-capable), **Voxtral** (multilingual open model),
  **Cactus** ([cactuscompute.com](https://cactuscompute.com/compare/best-whisper-cpp-alternative))
  — a mobile on-device runtime that hosts Whisper/Moonshine/Parakeet with sub-6% WER.

For a **courier phone during an active shift** (§16.53 makes voice the practical primary input in
motion), the tradeoff triangle is latency × battery × language coverage:

**Wave-0 recommendation:**
- **Command/navigation path (the hot, in-motion path):** a **Moonshine-class streaming model**,
  gated by an on-device **keyword-spotting wake** stage so the full ASR only runs on an actual
  utterance (the single biggest battery lever — continuous full ASR drains a phone). This gives the
  ~100 ms responsiveness a hands-busy courier needs.
- **Multi-language / free-form path:** **whisper.cpp small or tiny** as the multilingual fallback for
  languages Moonshine covers weakly, accepting higher latency where real-time isn't required (e.g.
  address dictation vs. a terse "accept"/"arrived" command).
- Run both **natively (Tauri side), not in WASM**, for battery and NEON/SIMD access; expose a single
  `VoiceSource: InputSource` (P38b DZ-10's port) so the model behind it is swappable per §16.52's
  BYO-model stance.
- **Benchmark the unknown:** multi-language streaming ASR accuracy **and** battery draw on a real
  mid/low-end courier phone is unproven — this is a named Tier-3 measurement, not an assumption
  (Risk #2, §9). *Sources:*
  [Moonshine vs Whisper benchmark 2026](https://modelslab.com/blog/audio-generation/moonshine-vs-whisper-asr-real-time-speech-2026) ·
  [github.com/moonshine-ai/moonshine](https://github.com/moonshine-ai/moonshine) ·
  [Northflank — best open-source STT 2026](https://northflank.com/blog/best-open-source-speech-to-text-stt-model-in-2026-benchmarks) ·
  [Cactus](https://cactuscompute.com/compare/best-whisper-cpp-alternative).

---

## 6. Cross-platform wgpu deployment reality in 2026

**Web (WASM) — production-viable now, with a mandatory WebGL2 floor.** WebGPU has reached critical
mass: shipped in all major browsers, **~82% global support** — Chrome (mature since 113;
Android 121+), Firefox (stabilized; macOS Apple-Silicon in 145+), **Safari 26** (macOS Tahoe, iOS,
iPadOS, **visionOS**). `wgpu` targets WebGPU with a **WebGL2 fallback**, so the ~18% without WebGPU
still render — which is exactly the FE-16 ladder (WebGPU → WebGL2 → CPU `compose_field`) P38 §3.6
already specifies. *Sources:*
[web.dev — WebGPU in major browsers](https://web.dev/blog/webgpu-supported-major-browsers) ·
[caniuse.com/webgpu](https://caniuse.com/webgpu) · [MDN WebGPU](https://developer.mozilla.org/en-US/docs/Web/API/WebGPU_API).

**Desktop native — production-viable, but the Tauri shell decision is real.** `wgpu` itself is mature
(it *powers Firefox*; GPUI/Zed ship a full GPU UI on macOS/Linux/Windows). The friction is
**Tauri specifically**: Tauri is a **webview** framework by default. Running a **raw wgpu surface**
inside a Tauri window is possible (raw-window-handle, Tauri v2 multi-surface) but community reports
are rough — **flickering when webview and wgpu compete for the surface**, macOS threading
constraints, and an open feature request for first-class native-wgpu support
([tauri #15213](https://github.com/tauri-apps/tauri/issues/15213),
[#9220 flickering](https://github.com/tauri-apps/tauri/issues/9220),
[discussion #10964](https://github.com/tauri-apps/tauri/discussions/10964)). For a **no-DOM, fully
custom GPU UI**, the cleaner substrate is **`winit` + `wgpu` directly** (no webview), which is what
GPUI effectively does. This surfaces a genuine architecture choice: dowiz wants a browser/webview
*only* to host the web a11y mirror + overlay — so on **desktop** a `winit`+`wgpu` app with
**AccessKit** (native a11y, no webview needed at all) is more coherent with the canon than a Tauri
webview shell. Recommend prototyping **winit+wgpu+AccessKit** as the desktop path and reserving Tauri
for where its OS-integration/packaging is worth the surface-conflict cost. *Sources:*
[tauri #15213](https://github.com/tauri-apps/tauri/issues/15213) ·
[dceddia/wgpu-tauri-experiment](https://github.com/dceddia/wgpu-tauri-experiment).

**Mobile (iOS/Android) — the roughest surface.** `wgpu` runs natively on Metal/Vulkan, but **Tauri
mobile uses the system webview** (WKWebView / Android System WebView). So a full-GPU UI on Tauri
mobile means either (a) running wgpu via **WebGPU-in-webview** (Safari 26 / Chrome-Android WebGPU —
now possible but young), or (b) a **custom native view** hosting a raw wgpu surface (more control,
more platform glue). Neither is turnkey in 2026. This is the least-settled deployment target and
needs an early spike. *Sources:*
[Tauri v2 mobile guide](https://www.oflight.co.jp/en/columns/tauri-v2-mobile-ios-android) ·
[Tauri 2.0 stable](https://v2.tauri.app/blog/tauri-20/).

**Battery for a full-shift courier GPU UI (§16.34's explicit open question).** The single most
important lever already exists in the plan: **FE-14 lazy-render-on-settle** (P38 §3.5) — *0 rAF
wake-ups per second on a settled screen*, with a Lyapunov watchdog so divergence is never invisible.
A courier's screen is static most of a ride (a map + an ETA). Combined with a **30 FPS floor** on the
WebGL2/mobile path (P38 §6 already budgets 33 ms @30fps as acceptable) and **power-state-aware
throttling**, continuous GPU render for a shift is *plausible* — but **unproven on real budget
hardware**. §16.34 itself flags this as "a genuine battery-life question the Tier-3/P52 build needs
to benchmark, not assume away." Concur — this is Risk #3 (§9): a measured, on-device, full-shift
battery benchmark is a Tier-3 gate, and the settle gate is the mechanism whose real-world savings
must be demonstrated, not asserted.

---

## 7. SEO / AEO / GEO: the bot-facing static files (§16.55)

**`llms.txt`** ([llmstxt.org](https://llmstxt.org/), proposed by Jeremy Howard, Sep 2024) is a real,
current 2025–2026 convention: a Markdown file at `/llms.txt` curating links to a site's highest-value
content — a *routing* file telling agents what's worth fetching. **But the honest 2026 reality is
that no major AI crawler consumes it:**
- Adoption is low (~2–10% of sites depending on the study), and ~40% of existing files are plugin
  stubs.
- Google **publicly declined** it (Jul 2025; Mueller compared it to the discredited keywords meta
  tag). **No** major provider — OpenAI, Anthropic, Google, Meta, Mistral — has committed to it in
  production answer surfaces. One study of 500M+ AI-bot visits over 90 days found only **408** hits
  targeting `llms.txt` directly.
- Where it *does* real work: the **agentic / B2A layer** — IDE agents (Cursor, Claude Code, Copilot,
  Windsurf), MCP servers, in-product assistants fetch it. It is a **developer-experience** signal,
  not (yet) an SEO one.

**Implication for dowiz — an important sharpening of §16.55, not a contradiction of it:** ship the
`llms.txt`-style feed (it is cheap, forward-looking, and dowiz's own product *is* agent-facing), but
**do not rely on it for crawlability**. The **load-bearing** AEO substrate for a
restaurant/menu product is **schema.org JSON-LD** — `Restaurant`, `Menu`, `MenuItem`, `Offer`,
`PostalAddress`, `OpeningHoursSpecification` — which is what Google/Bing rich results **and** the
LLM answer engines actually parse for food/menu facts, plus the universally-consumed **`robots.txt`**
and **`sitemap.xml`**, and **Open Graph** for link unfurls. All of these are pure static files with
zero rendered markup — perfectly compatible with the no-DOM canon, generated from the same kernel
order/menu state that feeds the field UI.

**Wave-0 recommendation:** the bot-facing pack = **`robots.txt` + `sitemap.xml` + per-venue
schema.org JSON-LD (Restaurant/Menu/Offer) + Open Graph/`manifest.json`**, with an **`llms.txt` feed
added as a forward-looking extra** — explicitly ranked, JSON-LD first. Generate them from kernel state
at publish time (they are facts, not interactivity — §16.55's "two audiences" framing is exactly
right). Keep them a **separate output path** from the a11y mirror, as §16.55 insists. *Sources:*
[llmstxt.org](https://llmstxt.org/) ·
[SE Ranking — llms.txt study](https://seranking.com/blog/llms-txt/) ·
[codersera — llms.txt honest guide May 2026](https://codersera.com/blog/llms-txt-complete-guide-2026/) ·
[Digital Strategy Force — does your site need llms.txt 2026](https://digitalstrategyforce.com/journal/does-your-site-need-llms-txt-to-get-cited-by-ai-search-in-2026/).

---

## 8. Spatial / AR-VR readiness for a wgpu field-UI (§17.5)

The operator's claim that the *ad fontes* physics/math foundation "extends naturally to spatial" is
**directionally credible and partly already true in dowiz's data model** — but with concrete
technical requirements, not a free lunch.

**What works today (native OpenXR + wgpu):**
- `wgpu` + **OpenXR** is real: native standalone VR on **Meta Quest 2/Pro/3/3S with hand tracking**,
  rendering into the XR compositor. Reference integrations exist:
  [philpax/wgpu-openxr-example](https://github.com/philpax/wgpu-openxr-example) (Vulkan-only) and
  [matthewjberger/wgpu-example](https://github.com/matthewjberger/wgpu-example)
  (Windows/Linux/macOS/**Web/Android/OpenXR**, incl. Quest 3). Upstream `wgpu` OpenXR support is
  still tracked as an open issue ([gfx-rs/wgpu #602](https://github.com/gfx-rs/wgpu/issues/602)) — so
  it's example/fork-grade, not a first-class `wgpu` feature yet.
- **WebXR + WebGPU** on the web: **Safari 26.2 ships WebXR integration with WebGPU rendering on Vision
  Pro**. `wgpu`'s WebXR path is via a **fork** that exposes the underlying `GPUDevice` and wraps
  external `GPUTexture`s (avoiding per-frame copies); documenting/upstreaming it is an open issue
  ([gfx-rs/wgpu #8329](https://github.com/gfx-rs/wgpu/issues/8329)). Not yet mainline.

**Why dowiz is already better-positioned than most 2D apps:** the field model is *already 3D-shaped*
— DZ-10's `FieldPos { u, v, w }` carries a **w** axis (P38b §11.2), and `geo.rs` already has FOV,
line-of-sight, floor-slice, and storey primitives (P51 §0, from the splatting arc). P38b Додаток C
already costs out camera language and depth-of-field on the existing primitives. So the 2D UI can be
architected as the **degenerate (orthographic, w=0) case of a 3D scene**, which is the single
cheapest piece of AR/VR insurance available.

**Concrete requirements to "design in now" (the low-cost, high-leverage subset):**
1. **Keep the render pipeline view/projection-matrix-driven**, never hardcode a 2D ortho transform —
   so a perspective camera and a **stereo (two-view-per-frame)** path are configuration, not a
   rewrite.
2. **Keep `FieldPos` 3D end-to-end** (already true) and ensure layout/particle/SDF math never
   silently drops `w`.
3. **Abstract input as intent** (P38b DZ-10's `InputSource`) so **hand-tracking/gaze** become new
   `InputSource` impls beside pointer/voice — no interaction rewrite.
4. Treat OpenXR (native) and WebXR (web) as **two backends behind one XR seam**, mirroring the
   existing WgpuSink/GpuUploadSink abstraction.

**Named unknowns (do not hand-wave):** stereo rendering **doubles** the per-frame GPU cost against a
**90 FPS** VR budget (vs 60 on flat) — the frame budget math changes materially; **text legibility in
VR** at reading distance is a known-hard problem the MSDF Tier-1 must be re-validated against; and the
WebXR+wgpu path is **fork-only**, so web AR/VR readiness depends on unmerged upstream work. Full AR/VR
is **post-Wave-0**; the four architecture choices above are the "designed in now" that §17.5 actually
requires. *Sources:*
[philpax/wgpu-openxr-example](https://github.com/philpax/wgpu-openxr-example) ·
[matthewjberger/wgpu-example](https://github.com/matthewjberger/wgpu-example) ·
[gfx-rs/wgpu #602](https://github.com/gfx-rs/wgpu/issues/602) ·
[gfx-rs/wgpu #8329](https://github.com/gfx-rs/wgpu/issues/8329) ·
[web.dev — WebGPU/visionOS](https://web.dev/blog/webgpu-supported-major-browsers).

---

## 9. Prioritized riskiest unknowns a Tier-3 blueprint MUST resolve before implementation

Ranked by (probability of blocking the first real order) × (cost if discovered late). Each is a real
gap this research could not close from prior art — they are design decisions, not lookups.

1. **Non-Latin IME in a no-DOM canvas, and the §16.34-vs-P38 `<input>`-overlay contradiction (§0,
   Areas 2–3).** Browser IME composition fundamentally needs a focused editable DOM element; §16.34
   forbids the overlay P38 G6 currently plans. Tier-3 must pick: (a) hidden composition host
   (violates §16.34 — operator call), (b) bespoke in-canvas soft-IME (large, unprecedented), or
   (c) Latin-only Wave-0 text with non-Latin IME as a dated deferral. **This blocks checkout/address
   entry for CJK/Indic/Thai users and cannot be discovered late.**
2. **Multi-language real-time on-device voice under a courier battery budget (Area 5).** Moonshine
   gives the latency but is English-strong; multilingual streaming ASR accuracy + battery draw on a
   real budget phone is unmeasured. §16.53 makes voice the in-motion primary input — if it's slow or
   drains the battery in one language, the courier flow breaks.
3. **Full-shift GPU-render battery on real budget hardware (Area 6, §16.34).** The FE-14 settle gate
   is the right mechanism but its real-world savings are unproven on target devices. Needs a measured
   on-device benchmark as a Tier-3 gate.
4. **a11y-mirror for LIVE text-editing state on every screen, on web, without an AccessKit web
   backend (Area 2).** No crate to lean on for web canvas caret/selection a11y; must be hand-rolled
   and kept in sync per keystroke across the whole product, then Playwright-asserted. Large surface,
   easy to get subtly wrong (stale/missing announcements).
5. **Intent-only navigation with zero button fallback: discoverability & implicit onboarding
   (§16.35/§16.40/§16.50).** No food-delivery-scale precedent for a button-less UI; the
   "onboarding embedded in the field itself" mechanism (§16.50) is itself unspecified. A first-time
   user who cannot discover how to order is a conversion-killer. Needs a concrete learnability spec +
   usability test, not just an architecture.
6. **Friction-as-field-dynamics numeric mapping (§16.44).** Unspecified: how a money amount /
   irreversibility maps to wave amplitude/color/rhythm, what gesture *completes* vs *cancels*, and how
   intuition is acquired before any learned association exists. This is the confirm/cancel safety
   surface for money actions — it must be unambiguous and audio-accessible, and it is currently a
   principle without a spec.
7. **Desktop/mobile shell decision: `winit`+`wgpu`+AccessKit vs Tauri-webview vs WebGPU-in-webview
   (Area 6).** Raw wgpu in Tauri is rough (surface conflicts, macOS threading); mobile Tauri is
   webview-only. The wrong early choice is expensive to reverse. Needs an early spike per platform.
8. **Procedural field/wave glyph generation has zero precedent (Area 4).** Must be held strictly off
   the Wave-0 critical path behind the MSDF (+ Vello) Tier-1, with an objective legibility/contrast
   gate before any procedural glyph ships. Risk is scope-creep pulling it onto the critical path.
9. **WebGL2-floor parity for the ~18% without WebGPU (Area 6).** The *entire* UI — not just ambience
   — must render on the WebGL2/CPU floor (FE-16). Any WebGPU-only visual effect silently excludes
   ~1-in-5 web users; parity must be a tested gate, not an afterthought.
10. **WebXR+wgpu is fork-only; OpenXR support is not upstream `wgpu` (Area 8).** AR/VR "readiness"
    depends on unmerged integrations. The cheap insurance (matrix-driven pipeline, 3D `FieldPos`,
    intent-abstracted input) must be locked in now so the fork risk stays post-Wave-0, not baked into
    Wave-0 assumptions.

---

## 10. One-paragraph synthesis

Nothing in the 2026 prior art forces a walk-back of any closed decision — a full application UI *can*
be GPU-rendered from scratch in Rust (GPUI/Zed proves it), WebGPU is production-ready across browsers
with a WebGL2 floor, the hard text-model algorithms are solved by cosmic-text/parley, AccessKit is
production-ready on native, and the field model is already 3D-shaped for AR/VR. The three places the
research says "build the pragmatic substrate now, keep the ambition as a parallel track" are
**typography** (MSDF real fonts Tier-1; field/wave glyphs = unprecedented R&D track), **voice**
(Moonshine streaming for commands + whisper.cpp for multilingual fallback), and **AEO** (schema.org
JSON-LD is load-bearing; llms.txt is a forward-looking extra, not a crawlability bet). The two places
it says "**there is a real, currently-unsolved problem that needs an operator-level decision**" are
**non-Latin IME in a no-DOM canvas** (which collides with the §16.34 no-`<input>` rule that P38's own
plan currently violates) and **courier battery under full-shift GPU render** — both named honestly by
§16.34 itself, both now backed with concrete 2026 evidence and a mitigation path, neither assumed
away.
