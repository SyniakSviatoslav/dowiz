# OPUS-R7 — Demo-recording + pixel-diff pipeline: neko + openmontage + Lightpanda, honestly evaluated (2026-07-18)

> **Research document — writes no product code.** Investigates the operator's proposed pipeline
> (**neko + openmontage + Lightpanda** to browse dowiz's own demos, record video, and do
> **pixel-by-pixel** render comparison over time, **explicitly without Playwright**) against live
> web/GitHub sources this pass and against the repo's own ground truth. Follows the R6 precedent
> (`OPUS-R6-NAMED-TECH-ANALYSIS-2026-07-18.md`): each named tool is verified as a concrete artifact,
> the four-way hypothesis is not forced into coherence, and the closest existing relative in the
> codebase (P63 SP-6) is checked before proposing anything new. Every load-bearing claim is a live
> quote or a fresh `file:line`, not memory.

---

## 0. One-line verdict table

| Question | Verdict | Evidence |
|---|---|---|
| Can **Lightpanda** capture screenshots / frames? | **NO — by construction.** No rendering engine ⇒ no pixels to capture. | Lightpanda's own blog + README (§1) |
| Is **neko** a fit for an automated pixel-diff pipeline? | **NO — wrong category** (human co-browsing VM; its only automation path is *running Playwright inside it*). | neko README/docs (§2) |
| Is **openmontage** a fit? | **NO — wrong category** (video-production studio; never captures/diffs external browser frames). | OpenMontage README/guides (§3) |
| Do the three compose into one coherent pipeline for this need? | **NO — refuted, same as R6's four-way.** They don't connect; the "recording" leg has no working tool among them. | §1–§3, §7 |
| Is there a real native-Rust / ffmpeg alternative? | **YES** — `dssim` / `image-compare` / `pixelmatch` / `dify` + the `image` crate; ffmpeg only if video is genuinely wanted (it isn't). | §4 |
| Can the wgpu engine already export frames natively, no browser at all? | **YES — already shipping on the CPU path**; the GPU path is one readback away. | `field_frame.rs`, `wasm/src/lib.rs`, `gpu.rs` (§5) |
| Move away from Playwright for this specific need? | **Partly — but not toward these tools.** Native Rust frame-diff is *more* ad-fontes than any browser for dowiz's OWN renders; Playwright stays for the real-browser-rung capture leg (it's already SP-6's driver). | §6 |
| Net recommendation | **Extend P63 SP-6; do NOT invent a parallel pipeline or adopt neko/openmontage/Lightpanda for this.** | §7 |

---

## 1. Lightpanda's real screenshot capability (verified, not assumed)

**This is the load-bearing question, and the answer is an unambiguous NO.** Lightpanda's defining
architectural choice — the same one that makes it ~9–16× lighter than headless Chrome (R6 §8) — is
that **it has no rendering engine**. It builds the DOM tree and runs JavaScript (V8), and stops
there. There is no CSS layout, no font/image fetch-for-display, no paint, no compositing — and
therefore **nothing to screenshot**.

Lightpanda's own blog states this directly (verbatim, fetched this pass):

> "You can't take screenshots of pages that aren't rendered."

Its comparison table answers **"No"** to *"Fetch and parse CSS to apply styling rules," "Calculate
layout," "Fetch images and fonts for display," "Paint pixels to render the visual result,"* and
*"Composite layers…"* — and it has a dedicated **"No Visual Regression Testing"** section. The blog's
own escape hatch is to *"fall back to Headless Chrome when [you] need [screenshots]."*

The GitHub README's implemented-feature list confirms it by omission — it lists *CORS, HTTP loader,
HTML parser, DOM tree, JS (v8), DOM APIs, Ajax (XHR/Fetch), DOM dump, CDP/websockets server, click,
input forms, cookies, custom headers, proxy, network interception* — and **no screenshot, no render,
no paint**, under the banner *"No graphical rendering engine."* Status is **Beta / "work in
progress… You may still encounter errors or crashes."**

**Resolving the one contradictory source.** A third-party CDP-compatibility write-up claimed the
`Page`/`Target`/`Browser` domains "handle … screenshot correctly." This conflates *CDP command
surface* with *pixel output*. Even if a `Page.captureScreenshot` handler is wired into Lightpanda's
CDP layer, it cannot return a meaningful image because there is no rasterizer behind it — the same
source notes coordinate-based interactions don't work "because there are no real coordinates … no
visual layout information." The authoritative signal is Lightpanda's own "you can't take screenshots
of pages that aren't rendered." **Lightpanda cannot perform the "recording" half of the operator's
pipeline at all.** This does not diminish R6's separate, still-valid recommendation (Lightpanda as a
JS-rendering *read/extract* engine candidate for P42 §11.7) — extraction and pixel-capture are
different jobs, and Lightpanda does exactly one of them.

*Sources:* [lightpanda.io/blog — what is a true headless browser](https://lightpanda.io/blog/posts/what-is-a-true-headless-browser)
· [github.com/lightpanda-io/browser](https://github.com/lightpanda-io/browser).

## 2. Honest verdict on neko's fit — POOR (wrong category, confirmed)

neko is a **real desktop browser (Chromium/Firefox/…) in Docker, WebRTC-streamed to humans** for
co-browsing/watch-party/remote-support (R6 §3, re-confirmed). Two capabilities surfaced this pass
that deserve honest treatment rather than a hand-wave:

1. **A REST API (OpenAPI 3.0) and a `/screenshot.jpg?pwd=<admin>` endpoint** exist, plus session
   recording for audit. But the screenshot is **JPEG** (lossy, block-DCT artifacts) and works "only
   for unlocked rooms." **Lossy JPEG is disqualifying for pixel-by-pixel diffing** — you cannot
   assert a ΔE≤0.02 tolerance against a bit-deterministic oracle through a lossy codec that
   introduces its own per-8×8-block noise. It is a human-convenience thumbnail, not a regression
   instrument.
2. **The documented automation path is "install Playwright or Puppeteer" *inside* neko.** That is the
   tell: neko is **not the automation layer** — it is the heavyweight browser+desktop-VM you would
   *drive with Playwright*. So "neko for the pipeline" collapses to one of two things: (a) use its
   lossy JPEG endpoint (unfit for pixel-diff), or (b) run Playwright inside a GB-scale Docker
   desktop-VM — which **defeats the "without Playwright" goal**, and reintroduces exactly the
   Docker/desktop-VM weight the roadmap's microVM+WASM arc is retiring.

neko is a genuinely good tool for its actual purpose (streaming a live browser to a person). For an
**unattended pixel-diff pipeline it is the wrong category**, precisely as R6 found for the original
four-way hypothesis. No adoption for this need.

*Sources:* [github.com/m1k1o/neko](https://github.com/m1k1o/neko) · [neko.m1k1o.net](https://neko.m1k1o.net/).

## 3. Honest verdict on openmontage's fit — POOR (wrong category, confirmed)

OpenMontage is an **agentic video-*production* studio** — script → asset-gen → voice-over → edit →
composite → FFmpeg render — orchestrated by an AI coding assistant, on a Python + Node/Remotion
stack (R6 §4, re-confirmed). Checked specifically for any browser-frame-capture surface this pass:
its only "browser" is **Remotion's own `localhost:3000` dev-server preview of the video being
authored** (scrub the timeline, check transitions before the final FFmpeg render). That is a preview
of OpenMontage's *own composition*, not a mechanism to browse **dowiz's demos** and capture/diff
their rendered frames. Its FFmpeg use is the *final video render*, not frame-extraction-for-
comparison. It has **no pixel-diff, no visual-regression, no external-page-capture capability of any
kind.** It is simply a different domain — and, per §16.4, the agent-studio shape is itself off-thesis
for dowiz. No relevance to this use case; confirmed, not merely assumed.

*Sources:* [github.com/calesthio/OpenMontage](https://github.com/calesthio/OpenMontage) ·
[AGENT_GUIDE.md](https://github.com/calesthio/OpenMontage/blob/main/AGENT_GUIDE.md).

## 4. The real native-Rust / ffmpeg alternatives (what actually does this job)

The pixel-diff half has mature, native-Rust answers — no browser required to *compare* two frames:

| Crate | What it computes | License | Fit |
|---|---|---|---|
| **`dssim` / `dssim-core`** (kornelski) | Multiscale **SSIM** in L\*a\*b\*, human-perception-approximating; returns `1/SSIM − 1` (0 = identical, unbounded = worse) | **AGPL-3.0** or commercial | Strong for a *perceptual* ΔE gate; **AGPL is license-compatible with dowiz's AGPLv3 open-source goal** (memory `open-source-goal-adr020`), so the usual AGPL blocker doesn't apply here |
| **`image-compare`** | SSIM + RMS + a hybrid metric, grayscale + **RGB/RGBA** | permissive (crates.io) | Best drop-in if a permissive license is preferred over dssim's AGPL |
| **`pixelmatch`** (crate) | Direct Rust port of mapbox/pixelmatch (YIQ perceptual per-pixel threshold + anti-alias tolerance) | permissive | Matches the "pixelmatch-equivalent" the operator asked about, 1:1 |
| **`dify`** (jihchi) | Fast pixel-by-pixel diff, inspired by pixelmatch + odiff | **MIT** | CLI + lib; good for a quick per-pixel count |
| **`image`** (already an ecosystem staple) | Raw RGBA buffer access | permissive | The **ponytail path**: a per-pixel ΔE diff over two `Vec<u8>` buffers is ~30 lines of stdlib Rust — no crate at all if the metric is simple |

**ffmpeg's real role here: essentially none.** ffmpeg extracts frames *from a recorded video*
(`ffmpeg -i in.mp4 -vf fps=… out_%04d.png`). It is only needed if you first **record video** — and
you only record video if you cannot get frames any other way. For dowiz you *can* get frames another
way (§5), so **the entire record-video-then-extract-frames leg is avoidable**, which removes the one
piece neko/openmontage/ffmpeg would have contributed. Note also: **no video/recording/ffmpeg
infrastructure exists in the repo today** (`grep` for capture/screenshot/readback across `engine/`
+ `kernel/src/render/` returns only the native `Vec<u8>` composers below) — adopting a video pipeline
would be *adding* a dependency class to avoid a problem the engine doesn't have.

*Sources:* [github.com/kornelski/dssim](https://github.com/kornelski/dssim) ·
[crates.io/crates/image-compare](https://crates.io/crates/image-compare) ·
[crates.io/crates/pixelmatch](https://crates.io/crates/pixelmatch) ·
[github.com/jihchi/dify](https://github.com/jihchi/dify).

## 5. The wgpu engine ALREADY exports frames natively — no browser at all (the decisive finding)

The most *ad fontes* observation in this whole investigation: **dowiz's own engine already emits
rendered frames as raw RGBA byte buffers, in-process, deterministically.** A browser-recording
pipeline to compare dowiz's own renders is not merely heavy — it is **unnecessary**.

- `engine/src/field_frame.rs:229` — `pub fn frame_rgba(&self) -> Vec<u8>`: maps the current field to
  an **RGBA8 bitmap** (`len == w*h*4`), non-finite values render black (never NaN bytes).
- `engine/src/field_frame.rs:255` — `pub fn compose(scene, eq, w, h, steps) -> Vec<u8>`: rasterize →
  evolve `steps` → return the final RGBA8 frame. **Bit-deterministic** — asserted identical across
  calls (`compose_returns_deterministic_frame`, cited at `field_frame.rs:430`, bit-equality at
  `:447–450` per P63 §0). This determinism is exactly what turns a pixel-diff into a *real gate*
  rather than a flaky screenshot compare.
- `wasm/src/lib.rs:57` — `compose_field(circles, w, h, steps) -> Vec<u8>`, plus
  `FieldSim::frame() -> Vec<u8>` (`:96`): the **same** RGBA bitmap crossed to JS for the live canvas.

So comparing "renders over time" for the CPU/field substrate is literally: call `compose()` twice
(or once now vs. a stored golden), and diff two `Vec<u8>`s with any crate in §4. **Zero browser, zero
video, zero ffmpeg, zero Docker VM.** Pure Rust, deterministic, CI-runnable.

**The one thing native export does NOT yet reach — and where a browser *is* legitimately needed.**
`kernel/src/render/gpu.rs` brings up a **real headless GPU context** (live `wgpu::Instance` → real
`Adapter` → `Device`/`Queue`, typed `GpuError::NoAdapter` degrade, `:53–80`) — but it has **no
texture→buffer readback** (`grep` confirms no `copy_texture_to_buffer` / `map_async` anywhere). It
renders to no surface and reads back no pixels *today*. Capturing **what the real GPU actually
painted** (the WebGPU and WebGL2 rungs of the FE-16 ladder, in a real browser) is the genuinely
browser-shaped part of the job — and it is precisely what **P63 SP-6 already designs** (§6). The
native `compose()` export is the *oracle*; a real-browser capture is what you diff *against* it.

## 6. Playwright vs. the alternatives — honest verdict for THIS specific need

**Framing correction first (the operator's own caveat).** The no-Node/TS constraint governs the
*shipped product runtime*, not test/dev tooling — and Playwright-**Python** exists, so "avoid
Node/TS" is not by itself a reason to drop Playwright here. More importantly, **Playwright is already
this repo's e2e/visual/a11y tool**: P58 owns the shared Playwright accessibility-tree harness
(`web/tests/a11y/harness.mjs`), and **P63 SP-6 already specifies `web/tests/floor-parity.spec.mjs` as
a Playwright driver** that forces each render rung (WebGPU-disabled / WebGL2-only / CPU-only via the
FE-16 flags) and captures the on-screen result. Playwright is not unavailable or disliked here; it is
load-bearing, already-adopted infrastructure.

**Playwright's real strengths for this need.** Built-in `page.screenshot()` (lossless PNG),
`expect(page).toHaveScreenshot()` visual comparison with its own tolerance, video recording, and — the
part that matters most — it drives a **real browser engine**, so it captures **what the GPU actually
painted**. For the "browse dowiz's own demos in a real browser and verify the pixels" leg, that is
exactly right, and no tool in the operator's trio can do it: Lightpanda paints nothing (§1), neko's
only real automation *is* Playwright-inside-a-VM (§2), openmontage is unrelated (§3).

**Where something *better than Playwright* exists — and it is native Rust, not these tools.** For
dowiz's **own CPU/field renders**, routing through *any* browser (Playwright included) is a detour:
the engine already hands you the deterministic `Vec<u8>` (§5). Diffing those buffers in pure Rust is
faster, hermetic, has no browser flake, and is maximally ad fontes. So the honest split is:

| Leg of the need | Best tool | Why |
|---|---|---|
| Compare dowiz's **own CPU/field renders** over time (golden regression) | **Native Rust** (`compose()` → §4 diff) | Deterministic, no browser, no flake — more ad fontes than any browser |
| Verify the **real GPU rungs** (WebGPU/WebGL2 in a browser) match the oracle | **Playwright** (SP-6's existing driver) | Only a real browser paints real GPU pixels; already the repo's tool |
| "Record video + neko + openmontage + Lightpanda" | **None of them** | Lightpanda can't screenshot; neko is a human VM you'd drive with Playwright; openmontage is video-editing |

**Verdict:** Moving away from Playwright *toward neko + openmontage + Lightpanda* for this use case
would be **discarding a working, already-integrated tool for strictly worse ones** — one of which
(Lightpanda) cannot do the core task at all. The *legitimate* "move away from Playwright" is narrower
and in the opposite direction: for dowiz's **own** renders, prefer **native Rust frame-diffing** over
any browser — which is already the CPU-oracle half of SP-6. Playwright stays for the real-browser-rung
capture. Nothing here justifies the operator's trio.

## 7. Recommended pipeline — extend P63 SP-6, do not invent a parallel one

**The operator's need is not new; it is SP-6.** "Pixel-by-pixel analysis to compare renders over
time" is *exactly* SP-6's floor-parity method: a **per-pixel perceptual ΔE / (1−SSIM) diff of each
render rung against the bit-deterministic `compose()` oracle, gated at ΔE ≤ 0.02**
(`BLUEPRINT-P63-shell-platform-spike.md` §3.6, `PARITY_PERCEPTUAL_DELTA_MAX = 0.02`), delivered as a
durable, importable harness (`engine/tests/floor_parity/` + `web/tests/floor-parity.spec.mjs`). The
right move is a **small extension of SP-6**, not a new blueprint and not the trio.

**Concrete extension notes to fold into P63 SP-6 (research-level, operator-gated — no code):**

1. **Name the native diff crate.** SP-6 currently states the *metric* (ΔE / 1−SSIM) but not the
   implementation. Recommend: **`image-compare`** (permissive, RGB/RGBA SSIM+RMS) as the default, or
   **`dssim`** if a stronger perceptual multiscale-SSIM is wanted (its AGPL-3.0 is *compatible* with
   dowiz's AGPLv3 goal — worth a one-line DECART note, not a blocker). For the simplest per-pixel
   count, a ~30-line hand-rolled diff over the `image` crate (or `pixelmatch`/`dify`) suffices — the
   ponytail default. This satisfies the reuse-first bar without adding a browser.

2. **Add the "over time" (temporal) dimension SP-6 doesn't yet state explicitly.** SP-6 diffs *rungs
   against the oracle at one point in time*; the operator also wants *drift across commits*. Handle it
   the way P63 §4.2 already handles device rows — a **git-versioned golden corpus**: store golden RGBA
   frames (or their hashes) for the demo scenes; each run diffs new `compose()` output against the
   golden and appends a `VerdictRecord` (move-not-delete, `captured_utc`). No video, no ffmpeg — a
   golden-frame baseline is the standard, hermetic visual-regression shape. This is the entire
   "record for comparison" need, met without recording anything.

3. **Add a browser-free "engine self-diff" rung** in `engine/tests/floor_parity/`: diff `compose()`
   across commits directly in Rust (the §5 path). This is the most ad-fontes rung and needs no
   Playwright at all — Playwright's SP-6 role narrows to *only* the real-GPU-browser rungs (WebGPU/
   WebGL2), which is the one thing native export can't reach until `gpu.rs` grows a
   `copy_texture_to_buffer` readback (itself SP-1/SP-6 GPU-capture work).

4. **Record the trio's non-fit explicitly** in the SP-6 §7 links, the way R6 recorded the four-way
   refutation: Lightpanda (can't screenshot — §1) is declined *for this leg* (its P42 §11.7
   read/extract candidacy is untouched); neko and openmontage are declined outright for pixel-diff.
   This keeps the "capability follows recorded need, not novelty" discipline.

**Do NOT** stand up a neko+openmontage+Lightpanda pipeline: it has no working "record" tool
(Lightpanda), no lossless automated capture (neko's JPEG), no relevant capability (openmontage), and
it would add Docker-VM + Python/Node/Remotion weight to duplicate — worse — a gate the repo already
designs natively. The honest output of this exercise mirrors R6's: **the three named tools do not
compose into a coherent pixel-diff pipeline; the need is already owned by P63 SP-6 and is best served
by native-Rust frame-diffing + the existing Playwright browser-rung driver.**

---

## 8. What to carry forward

- **Load-bearing fact:** **Lightpanda cannot capture pixels** (no rendering engine — its own docs) —
  it disqualifies the whole "record with Lightpanda" premise.
- **neko** = human co-browsing VM whose only real automation is *Playwright-inside-Docker* + a lossy
  JPEG endpoint → **wrong tool** for unattended pixel-diff.
- **openmontage** = video-production studio, **no external-page capture** → **wrong category**.
- **The engine already exports deterministic RGBA frames** (`field_frame.rs:229/:255`,
  `wasm/src/lib.rs:57/:96`) → compare dowiz's own renders in pure Rust, **no browser/video/ffmpeg**.
- **Native diff crates that actually do the job:** `image-compare` (default), `dssim` (perceptual,
  AGPL-compatible-here), `pixelmatch`/`dify` (MIT), or ~30 lines over `image`.
- **Playwright stays** — it is already SP-6's browser-rung driver + P58's a11y harness; the real-GPU
  paint is the one leg native export can't reach.
- **Action:** extend **P63 SP-6** with (a) a named native diff crate, (b) a git-versioned golden-frame
  temporal baseline, (c) a browser-free engine self-diff rung, (d) an explicit "trio declined for this
  need" note. **No new blueprint; no adoption of neko/openmontage/Lightpanda for this.**
