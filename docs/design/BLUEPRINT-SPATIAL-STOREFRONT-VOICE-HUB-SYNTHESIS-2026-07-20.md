# BLUEPRINT — Capture → Asset → Display → Voice: The Spatial Storefront and the Talking Hub

**Status: RESEARCH SYNTHESIS / PLAN. No code has been written. Nothing in this document is built. This is the reconciliation of five completed Opus research passes into one reviewable plan; the operator reviews this before anything is blueprinted further or implemented.**

**Date:** 2026-07-20
**Inputs:** Five Opus research passes — (P1) photogrammetry/volumetric capture, (P2) AR/volumetric display, (P3) SMPL/GVHMR/FaceAnything human capture, (P4) RAG+CAG/agentic retrieval vs. the actual dowiz retrieval stack, (P5) local voice control and voice-answering hub agent. All external facts and citations in this document come from those passes. Repo facts (file paths, identifiers) were re-verified against the live tree at `/root/dowiz` on 2026-07-20.
**Intended landing path if approved:** `docs/design/BLUEPRINT-SPATIAL-STOREFRONT-VOICE-HUB-SYNTHESIS-2026-07-20.md`

---

## 0. The one-sentence thesis

An owner points a phone at a dish for sixty seconds and gets a 3D asset; a customer taps one badge on the storefront and sees that dish on their own table through the browser with no app install; the owner talks to the hub, and a local agent — running through the exact same capability-gated tool path as every typed command — retrieves, acts, answers out loud, and interrupts only for events the owner asked to be warned about.

That is one pipeline, not five features: **capture → asset → display → agent → voice**. The five research passes each examined one segment of it. Four segments have a buildable, honestly-costed path. One (human body/face capture) mostly does not, and this document says so plainly rather than rounding it up.

---

## 1. Constraint frame — the invariants every verdict below was tested against

Every decision in this document was checked against six standing constraints. Where a research finding collided with one of these, the constraint won.

| # | Constraint | Source of authority |
|---|---|---|
| C1 | **CPU-only servers.** Hub/server hardware is Hetzner-class, no GPU. Anything CUDA-mandatory cannot run server-side, period. | Established repo constraint; confirmed as the binding limit in P1 (reconstruction) and P3 (all three human-capture models). |
| C2 | **Rust-first, minimal-dep, feature-gated.** The default kernel build is pure-`std` and serde-free; heavy/external functionality goes behind off-by-default Cargo features with a documented rationale (DECART). | `CLAUDE.md` feature discipline; `kernel/Cargo.toml` header docs. |
| C3 | **No async runtime in the agent lane.** `llm-adapters` is synchronous-only ("no tokio, per 2026-07-15 operator mandate"); the `ToolPort` trait is a blocking `fn invoke`; time-bounded work already uses a watchdog thread + `recv_timeout` (`agent-loop/src/lib.rs:25-27`, `TOOL_TIMEOUT_MS`). P5 confirmed this is also the idiomatic pattern for real-time audio (`cpal` is callback-based on a dedicated OS thread bridged via `std::sync::mpsc`; async inside a realtime audio callback is actively discouraged). | P5 code reading + operator mandate. |
| C4 | **The compile firewall stands.** `agent-facade` is the only agent crate that imports `dowiz-kernel`; `agent-loop` imports only `agent-facade` and structurally cannot name kernel mutation. Every new tool proposed here goes through `ToolPort` on `agent-facade` — nothing tunnels around it. | `CLAUDE.md` crate map; P4/P5 code reading. |
| C5 | **DenyByDefault red-lines apply to every input modality.** `kernel/src/ports/agent/scope.rs` (`RedLinePolicy::DenyByDefault`, red-lines Ledger/Auth/Secret/Migration, verified at `scope.rs:268`). P5's central architectural finding: a voice-issued command flows through the same `ChatRequest → ToolPort` path as a typed one, so **voice is a new input modality, never a privilege escalation.** This is not a design goal to be achieved — it falls out of the cascaded architecture chosen in §5, and a test must prove it (§8, Phase V acceptance). | `scope.rs` + P5. |
| C6 | **No courier scoring, rating, or reputation — ever.** CI-enforced (`no-courier-scoring` job); routing enums omit `Ord`/`PartialOrd` by design. P3 found that using body/face capture to track or measure courier performance would directly violate this. That finding is treated as a hard rejection, not a trade-off. | `DECISIONS.md` D0; P3. |

One additional factual constraint from P3: the launch market is inside GDPR jurisdiction (DECISIONS.md D12), so persistent biometric templates carry regulatory exposure on top of the invariant conflict. This is stated once as a legal fact and used as such.

---

## 2. Decision register

Every decision is falsifiable: each has a verdict, a reason traceable to a research pass, and (for BUILD items) an acceptance criterion in §8. "GATED" means buildable but blocked on a named prerequisite. "OPERATOR" means this synthesis deliberately does not decide.

| ID | Decision | Verdict | Basis |
|----|----------|---------|-------|
| D-A1 | Reconstruction happens **on the capture device**, never on dowiz servers | **BUILD (as policy)** | P1: on-device is where the field moved (Apple Object Capture on-device iOS; PocketGS arXiv:2601.17354, ~4 min on iPhone; Mobile-GS arXiv:2603.11531, 116 fps on Snapdragon); C1 forbids server CUDA anyway |
| D-A2 | Server-side SfM→MVS / 3DGS training pipeline (COLMAP, gsplat, OpenSplat) | **REJECTED** | P1: state-of-the-art reconstruction is Python/CUDA-bound; C1 (no server GPU) |
| D-A3 | Rust-native 3DGS *training* (brush) as a product dependency | **REJECTED (revisit ≥12 mo)** | P1: brush is one experimental project (Burn + WebGPU, browser mode Chrome/Edge-only) — research-grade, not shippable |
| D-A4 | Scope self-serve capture to **single objects** (dishes, products), 30–60 s phone capture | **BUILD** | P1: realistic for a non-technical owner today; whole-room capture needs pro/LiDAR gear — out of self-serve scope |
| D-B1 | Customer storefront AR via **browser-based `<model-viewer>`** → AR Quick Look (iOS, USDZ) / Scene Viewer (Android, glTF/GLB) | **BUILD — Phase 1, first ship** | P2: strongest, cheapest, most production-ready fit; zero app install, zero new hardware |
| D-B2 | Interactive depth-aware projected AR (RoomAlive-style) for the owner hub | **REJECTED** | P2: proven research (Microsoft Research, UIST 2014) whose one commercial appliance (Lightform LF2) shut down in 2022; dowiz would have to become the hardware/calibration integrator that already failed commercially |
| D-B3 | "Projected on another surface" delivered as a **projector-legible hub display mode** (plain short-throw/pico projector as a dumb second screen) | **BUILD — Phase 5** | P2: dumb projectors ($200–400) are cheap, buyable, real; interactivity stays on phone/keyboard/voice |
| D-B4 | Light-field (Looking Glass), swept-volume (Voxon VX2), laser-plasma displays | **REJECTED as product; Looking Glass = OPERATOR option for showroom marketing** | P2: Looking Glass $199–$1500+ is a screen you look *at*, not projectable; Voxon ~$6,800 showroom-grade; laser-plasma lab-only |
| D-B5 | Pepper's-ghost / spinning-LED "holo-fans" | **REJECTED** | P2: front-arc-only 2D illusion, not volumetric; marketing one as a "hologram" fails the repo's honesty bar |
| D-C1 | Wire the already-present local embeddings (Ollama embed port) into a real **dense vector index + working rerank** | **BUILD — Phase 2** | P4: this is the single biggest retrieval-quality gap — the model is wired, the index is missing |
| D-C2 | Expose a **read-only retrieval ToolPort** through `agent-facade` | **BUILD — Phase 3** | P4: the agent physically cannot query kernel retrieval today (only 2 tools exist); every agentic-RAG technique is blocked on this one missing piece |
| D-C3 | Literal CAG (KV-cache preloading, arXiv:2412.15605) | **REJECTED; its spirit adopted** | P4: poor fit (small-context Ollama models; the firewall has no KV-cache seam; determinism tension). Spirit → prompt-level compacted-index preload + finishing the named-but-unbuilt `SemanticOk` cache layer |
| D-C4 | Self-RAG / CRAG-style multi-hop agentic retrieval (arXiv:2310.11511, arXiv:2401.15884, survey arXiv:2501.09136) | **GATED on D-C2** | P4: all blocked by the missing ToolPort; buildable after Phase 3 with the existing bounded 4-iteration loop |
| D-V1 | Cascaded voice pipeline: wake-word → VAD-gated STT → **existing unmodified agent-loop/ToolPort path** → chunked TTS; threads + channels, no async | **BUILD — Phase 4** | P5: matches C3/C4/C5 exactly; the codebase already uses this concurrency pattern |
| D-V2 | Full-duplex speech-to-speech model (Kyutai Moshi) | **REJECTED** | P5: GPU-bound and architecturally incompatible with the capability-gated ToolPort model — it would fuse understanding and acting, destroying the gate |
| D-V3 | STT engine: whisper.cpp/whisper-rs baseline; evaluate Moonshine (arXiv:2602.12241) head-to-head | **BUILD (bake-off inside Phase 4)** | P5: whisper.cpp mature CPU baseline; Moonshine claims sub-200 ms on a Raspberry Pi, ~5× faster than Whisper — claim must be reproduced on hub hardware, not trusted |
| D-V4 | TTS engine: **Kokoro-82M** via the Rust `ort` ONNX runtime; Piper as fallback only | **BUILD** | P5: Kokoro-82M (Jan 2025, Apache-2.0, ~327 MB, ~90 ms first-audio, real-time on CPU, no Python); Piper works but "sounds like a 2015 GPS" |
| D-V5 | Proactive (unprompted) spoken alerts | **BUILD — Phase 6, under a strict allowlist policy** | P5: least-solved piece industry-wide (patent-stage at Google/Amazon, no open reference stack); UX literature (Nielsen Norman Group, Smashing Magazine) is clear on alert fatigue. dowiz has the one asset that makes it tractable: a deterministic event-sourced order FSM as the alert source |
| D-V6 | Sync (non-async) **streaming read** of LLM responses in `llm-adapters` | **BUILD — inside Phase 4** | P5: transport currently hardcodes `"stream": false` (`llm-adapters/src/transport.rs:50`); without streaming, spoken-response latency is dominated by full CPU token generation before the first syllable |
| D-H1 | SMPL / GVHMR / FaceAnything deployment in any dowiz surface | **REJECTED as-is** | P3: all three are Python/PyTorch/CUDA-bound (C1), non-commercially licensed (SMPL patent US10395411B2 + paid Meshcapade license; GVHMR weights inherit the SMPL gate; FaceAnything CC-BY-NC, ~15 GB checkpoint), and the courier-analytics use case violates C6 outright |
| D-H2 | Any body/face/pose analytics applied to couriers | **REJECTED — red-line, not a trade-off** | P3 + C6: CI-enforced no-scoring rule; persistent biometric templates additionally conflict with local-first/no-tracker invariants and GDPR |
| D-H3 | One-time consented courier liveness/auth check | **OPERATOR (and if pursued: a purpose-built verification tool, not these three models)** | P3: named as the only plausibly-legitimate slice, explicitly noting a different tool class fits better |
| D-H4 | Stylized avatar / retail try-on via licensed SMPL | **OPERATOR (default: drop)** | P3: still hits the Meshcapade commercial-license gate; no current product surface demands it |

---

## 3. Lane A — Capture: the phone does the math, the hub stores the result

### 3.1 What the research settled

P1's core finding is a division of labor that happens to align perfectly with dowiz's hardware reality. The classical server pipeline (SfM → MVS via COLMAP, or 3D Gaussian Splatting training per Kerbl et al., SIGGRAPH 2023, arXiv:2308.04079, via gsplat/OpenSplat) is GPU-bound — unusable under C1. But the field moved reconstruction onto the capture device in 2025–2026: Apple Object Capture runs fully on-device on iOS; PocketGS (arXiv:2601.17354) reconstructs in ~4 minutes on an iPhone; Mobile-GS (arXiv:2603.11531) renders splats at 116 fps on a Snapdragon. Rust-native *rendering* of the resulting assets is mature (web-splat, gauzilla, bevy_gaussian_splatting); Rust-native *training* is one experimental project (brush, on Burn + WebGPU, with a Chrome/Edge-only browser mode) and is not a foundation to ship on (D-A3).

### 3.2 The dowiz position

**dowiz never reconstructs. dowiz ingests, validates, stores, and serves.** The owner's phone (or a third-party capture app on it) produces the asset; the dowiz asset pipeline is a content pipeline, not an ML pipeline. This keeps Lane A entirely out of the kernel — no new kernel code, no new features, no new deps in the default build. The server-side work is: upload endpoint, format/size validation, storage, and delivery to the storefront (Lane B).

Scope is deliberately narrow per D-A4: **single objects** — dishes, products — captured in 30–60 s by a non-technical owner. Whole-room/interior capture requires pro/LiDAR gear (P1) and is explicitly out of self-serve scope; if a merchant wants an interior scan, that is a services engagement, not a product feature.

### 3.3 The honest gaps

- **Asset format duality.** iOS AR Quick Look consumes USDZ; Android Scene Viewer consumes glTF/GLB (P2). Apple's on-device capture emits USDZ natively; the Android-side path and any conversion between formats is an unresolved engineering question this synthesis flags rather than hand-waves (§9, O4). Until resolved, the realistic v1 is: owners on iPhones produce USDZ; a GLB is required for the Android AR badge to light up; items lacking one format simply don't show the AR badge on that platform. Degraded, honest, shippable.
- **Splat vs. mesh.** 3DGS splats render beautifully in the Rust/browser viewers P1 found, but the *system* AR viewers in Lane B (Quick Look/Scene Viewer) are mesh-based (USDZ/glTF). Phase 1 therefore targets **mesh assets** (which Object Capture produces). Splat assets are a Phase-5+ option for the in-page (non-AR) viewer only, where web-splat-class renderers apply.

---

## 4. Lane B — Display: the browser is the volumetric display; the projector is a second monitor

### 4.1 The taxonomy, stated without marketing

P2's central service was demolishing the word "hologram" into four honest categories: (a) light-field displays (Looking Glass, $199–$1,500+) — real, but a screen you look *at*, not projectable; (b) swept-volume displays (Voxon VX2, ~$6,800) — real volumetric light in a small enclosed volume, showroom-grade pricing; (c) laser-plasma aerial voxels — lab-only, not purchasable; (d) Pepper's-ghost and spinning-LED "holo-fans" ($150–400) — the cheap retail illusion, front-arc-only, 2D, not volumetric. None of these is the customer-facing product (D-B4, D-B5).

### 4.2 The customer storefront wow-moment: browser AR, Phase 1, first ship

P2's strongest finding: Google's `<model-viewer>` web component auto-routes to **AR Quick Look on iOS** (built into Safari — zero app install, USDZ) and **Scene Viewer / WebXR on Android** (glTF/GLB). This is production infrastructure that already exists on every customer's phone. The one hard gotcha: **WebXR live-camera SLAM does not work in iOS Safari** — iOS customers get the "tap to place" Quick Look experience, not a live in-page camera overlay. That is acceptable: "see this dish on your table" *is* the wow-moment, and Quick Look delivers it.

This is the highest wow-per-effort item in the entire synthesis: no new hardware, no kernel changes, no ML, no GPU, assets supplied by Lane A. It goes first (§8, Phase 1).

**Repo-discipline note (must be decided, not slipped in):** `web/` is a zero-dependency render-only shell. `<model-viewer>` is a JavaScript component. The compliant integration is a **vendored, pinned, single-file static asset** in `web/` (no npm dependency tree, no build step), used strictly for rendering — consistent with `web/`'s "renders only, never re-implements math" charter, since an AR viewer computes no dowiz-authoritative state. On iOS specifically, Quick Look is link-triggerable from Safari with no JS at all, which offers a zero-JS fallback path. Flagged as O3 in §9 because it is the first-ever JS asset added to `web/` since the JS drop, and the operator should ratify that precedent explicitly.

### 4.3 "Projected on another surface": what the owner actually gets

The operator's request named projection onto another surface for the owner hub. P2's finding here is a clean fork:

- **True projected AR** — depth-aware, touch-interactive projection mapping (RoomAlive, Microsoft Research, UIST 2014) — is *proven research* whose only commercial appliance attempt, the **Lightform LF2, shut down in 2022**. Building this means dowiz becomes a hardware + calibration integrator, i.e., re-attempting the exact business that already failed. **Rejected (D-B2).**
- **A plain short-throw/pico projector ($200–400)** as a dumb second display for a purpose-designed hub view is cheap, real, and buyable today. **Adopted (D-B3, Phase 5).**

So the deliverable is a **"hub display mode"**: a projector-legible rendering of the owner hub — live order queue, courier positions, FSM-state warnings — designed for the medium (high contrast, large type, dark background, no fine pointer targets, legible at wall scale). All *interaction* with that display happens through the owner's phone or, once Phase 4 lands, **voice** — which is exactly the input modality a wall projection wants, and is why Lane B's endpoint and Lane D's endpoint converge on the same desk. Whether dowiz *recommends or bundles* a specific projector is an operator decision (§9, O1) — dowiz has never been a hardware vendor.

---

## 5. Lane C — The agent's substrate: finish the retrieval system dowiz already half-has

### 5.1 What actually exists (P4 read the code; re-verified against the tree)

This lane is unusual: the research pass's main discovery was that **dowiz is further along than its own roadmap discourse assumes, in every layer except the two that matter most.**

Present and real:
- Hybrid sparse retrieval: byte-trigram + BM25 (`kernel/src/retrieval/index.rs`, `bm25.rs`).
- Graph diffusion: personalized PageRank over a wikilink graph (`kernel/src/retrieval/ppr.rs`, `diffusion.rs`, on the CSR representation in `kernel/src/csr.rs`) — deterministic, reproducible, GraphRAG-adjacent already.
- A local embedding model, wired: the embed port in `kernel/src/ports/llm.rs` served by `llm-adapters/src/ollama.rs` (nomic-embed-text / qwen3-embedding).
- A response cache (`llm-adapters/src/cache.rs`).
- A bounded, synchronous, 4-iteration plan-act-observe executor (`agent-loop/src/lib.rs`).

Absent, and blocking everything downstream:
1. **No dense retrieval.** The embeddings feed only a semantic-leak-detection gate. There is no vector index of any kind, and the rerank port returns `Unsupported` everywhere (`kernel/src/ports/llm.rs:377` documents `Err(Unsupported)` as the current universal answer).
2. **No retrieval ToolPort.** `agent-facade` exposes exactly two tools — `ReadOrderStatusTool` and the feature-gated `WebFetchTool` (`agent-facade/src/lib.rs:59,130`). The agent **physically cannot query the kernel's own retrieval system.**
3. The cache is exact-match only; the `SemanticOk` Layer-B policy is named in `cache.rs:129` but unbuilt.

### 5.2 The verdicts

**D-C1 (Phase 2): build the dense layer the model is already waiting for.** The corpus is small (memory/docs scale, thousands of documents, not millions), so v1 is a **brute-force exact cosine scan** — pure-`std`, deterministic, no ANN crate, no new dependency, index rebuilt from the embed port's outputs. This is deliberately the minimum: an ANN structure (HNSW-class) is a *gated later step* justified only by a measured latency failure of the exact scan, per the repo's performance-priority-with-evidence rule. Simultaneously, implement the rerank port for at least one backend so `Err(Unsupported)` stops being the universal answer. Fusion of dense scores with the existing BM25+PPR signals must remain deterministic (fixed weights or reciprocal-rank fusion with total-order tie-breaking) — the retrieval layer stays replayable even though the embedding model itself is a feature-gated external.

**D-C2 (Phase 3): the read-only retrieval ToolPort.** One new tool in `agent-facade` — the only crate allowed to import the kernel — exposing query-in, ranked-passages-out. Read-only by construction: it re-exports no mutation symbols, so the firewall's guarantee ("agent-loop cannot name kernel mutation") is preserved verbatim. Scope-wise it is a plain capability, subject to the same `DenyByDefault` admission as every tool. P4's key structural insight is that **every** agentic-retrieval technique in the literature — Self-RAG (arXiv:2310.11511), CRAG (arXiv:2401.15884), the Agentic RAG survey's decompose/verify/multi-hop loops (arXiv:2501.09136) — is blocked by this one missing tool. Build the tool; the techniques become incremental prompt/loop work inside the existing 4-iteration bound rather than architecture.

**D-C3: CAG — reject the letter, keep the spirit.** Literal CAG (arXiv:2412.15605, "Don't Do RAG": preload a stable corpus into a frozen KV-cache) fails three dowiz-specific tests per P4: Ollama's small-context local models can't hold the corpus; the agent-facade/agent-loop firewall has no seam through which raw KV-cache state could pass without violating it; and KV-cache reuse is in tension with the determinism ethos. What survives is the *principle* — don't re-retrieve what is small and stable — implemented at two legitimate levels: (a) inject the compacted memory index into the system prompt (prompt-level preload), and (b) finish the `SemanticOk` semantic-cache layer that `cache.rs` already names, using the same embeddings from D-C1. Both land inside Phase 2–3 scope.

### 5.3 Why this lane sits in the middle of the build order

Lane C is not the wow. It is the reason the wow in Lane D is *useful*: a voice interface to an agent with two tools is a demo; a voice interface to an agent that can retrieve from the hub's own operational memory and order corpus is a product. Phases 2–3 are therefore sequenced as the prerequisite spine between the AR ship (Phase 1) and the voice ship (Phase 4).

---

## 6. Lane D — Voice: a new mouth and ears on an unmodified nervous system

### 6.1 The architecture, fixed by constraints before preference

P5's decisive finding is that the right voice architecture for dowiz is not a taste question — it is forced by C3/C4/C5:

```
[mic, dedicated OS thread (cpal callback)]
   → std::sync::mpsc →
[wake-word (openWakeWord) → VAD gate (Silero VAD / silero-vad-rust via ort)]
   → [STT: whisper.cpp/whisper-rs  vs  Moonshine — bake-off]
   → ChatRequest into the EXISTING agent-loop, UNMODIFIED
        (same ToolPort tools, same scope.rs DenyByDefault admission,
         same 4-iteration bound, same watchdog-thread timeouts)
   → [chunked TTS: Kokoro-82M via `ort`]
   → [speaker thread] → std::sync::mpsc → playback
```

- **Threads + channels, no async** — not merely policy compliance: `cpal` (RustAudio) is itself synchronous/callback-based, audio belongs on a dedicated OS thread bridged via `std::sync::mpsc`, and async inside a realtime audio callback is actively discouraged (P5). The codebase already proves the pattern at `agent-loop/src/lib.rs:25-27` (watchdog thread + `recv_timeout`, std-only).
- **Cascaded, not full-duplex.** Kyutai Moshi-class speech-to-speech models are GPU-bound and — the deeper problem — architecturally fuse comprehension with action, which is incompatible with a capability-gated ToolPort model where every action must pass an admission check (D-V2). The cascade keeps a hard, testable boundary: audio in, `ChatRequest` out; nothing about being spoken grants a request anything a typed request lacks.
- **Voice is not an escalation — provably.** A spoken "refund order #42" produces the same `ChatRequest → ToolPort` flow as a typed one and hits the same `RedLinePolicy::DenyByDefault` wall at `scope.rs` (Ledger/Auth/Secret/Migration). Phase 4's acceptance suite includes a RED→GREEN test asserting exactly this denial (§8).

### 6.2 Component selections, with the numbers the research actually gave

| Slot | Selection | Cost/maturity facts (from P5, unrounded) |
|---|---|---|
| Wake word | openWakeWord | Standard CPU building block (Home Assistant ecosystem) |
| VAD | Silero VAD via its Rust port (silero-vad-rust, `ort`) | Standard; also built into whisper-cpp-plus-rs |
| STT | whisper.cpp/whisper-rs **baseline**; Moonshine challenger | whisper.cpp: mature, CPU-capable; candle can run Whisper as a ~22 MB static pure-Rust binary. Moonshine (arXiv:2602.12241): *claims* sub-200 ms on a Raspberry Pi, ~5× faster than Whisper — a vendor-adjacent claim to be reproduced on hub hardware, not shipped on faith |
| TTS | **Kokoro-82M** via `ort` | Apache-2.0, ~327 MB, ~90 ms first-audio latency, real-time on CPU, no Python. Piper is the fallback but is honestly described as sounding "like a 2015 GPS" — below the product bar for a talking hub |
| Reference architecture | Home Assistant "Year of the Voice" (wake-word→STT→agent→TTS; Wyoming protocol) | The canonical fully-local open reference; dowiz mirrors its stage structure without adopting Wyoming (in-process channels suffice on one hub) |
| Barge-in | Acoustic echo cancellation required | P5: without AEC the assistant's own voice self-triggers the wake/VAD stage. v1 may ship half-duplex (mic gated closed during playback) with AEC as the v1.1 upgrade — stated as a limitation, not hidden |

**Crate placement:** a new edge crate (working name `voice-adapters`), sibling to `llm-adapters`, importing **only `agent-facade`** — the audio stack (cpal, `ort`, whisper bindings) never appears in the kernel, engine, or default build graph. Every model file and audio dependency sits behind an off-by-default feature with a DECART rationale (C2). Disk budget on the hub is real and stated: ~327 MB TTS + STT model + VAD/wake models — order of half a gigabyte of model weights, all local, none leaving the device (local-first preserved: audio is captured, transcribed, and synthesized on the hub; nothing is sent to any third party).

### 6.3 The one real transport gap (D-V6)

`llm-adapters/src/transport.rs:50` hardcodes `"stream": false`. For text chat this is fine; for voice it means the owner waits for *complete* CPU token generation before the first syllable — on a CPU-only hub, this is the dominant term in perceived latency, dwarfing the ~90 ms TTS and sub-second STT stages. The fix P5 scopes is a **synchronous streaming read** (blocking chunked reads on the existing thread model — no tokio), feeding sentence-boundary chunks to Kokoro so speech begins while generation continues. This is a real change to a load-bearing adapter and is called out as its own reviewed work item inside Phase 4, not smuggled in.

### 6.4 Proactive voice: the least-solved piece, and dowiz's one unfair advantage

P5 is blunt: genuinely *unprompted* spoken alerts have no widely-adopted open reference stack — the pattern is mostly patent-stage at Google/Amazon — and the UX literature (Nielsen Norman Group, Smashing Magazine) documents alert fatigue and mid-conversation interruption as real failure modes. The stated bar: **interrupt only for what the user explicitly asked to be warned about, or what is costly enough to miss.**

dowiz can clear that bar where generic assistants cannot, because the alert source is not a heuristic — it is the **deterministic, event-sourced order FSM** (`kernel/src/order_machine.rs`). P5's own example is exactly the target: "courier hasn't picked up order #42 in 20 minutes." Design consequences, adopted as policy:

1. **Allowlist, not classifier.** Spoken alerts fire only for FSM-derived event classes on an explicit, owner-visible allowlist; v1 ships with a minimal default set (operator to ratify contents — §9, O6) and per-owner opt-in/out per class.
2. **The alert path is read-only.** The proactive channel observes events and speaks; it holds no action capabilities. If the owner responds ("reassign it"), that response enters the normal Phase-4 command path with normal gating. The interrupt channel and the action channel never merge.
3. **Rate-bounded and suppressible** ("quiet" state), because the failure mode is fatigue, not silence.

This is Phase 6 — deliberately last, because it is the only phase where the industry gives dowiz no working reference to stand on.

---

## 7. Lane E — Human capture: the honest "mostly no"

This is the cluster where the correct synthesis output is a rejection with reasons, not a plan. P3's findings converge from three independent directions, any one of which would alone suffice:

1. **Licensing.** SMPL (Loper et al., SIGGRAPH Asia 2015) is patented (US10395411B2) and non-commercial without a **paid Meshcapade license**; the CC-BY "SMPL-Body" subset excludes key blendshapes. GVHMR (Shen/Pi/Xia et al., SIGGRAPH Asia 2024, arXiv:2409.06662) publishes non-commercial weights that *also inherit* SMPL's commercial gate. FaceAnything (Kocasari et al., TU Munich/Niessner lab, targeting ECCV 2026) is CC-BY-NC. Nothing in this cluster is commercially deployable as-is.
2. **Hardware.** All three are Python/PyTorch/CUDA-bound. GVHMR's headline speed (~280 ms network inference for a 45 s clip) is on an RTX 4090, and its *full* pipeline needs ViTPose+DPVO+SMPL; FaceAnything carries a ~15 GB checkpoint. None runs on a courier's phone or a CPU-only hub (C1). The only Rust/WASM-reachable fragment is SMPL's forward linear-blend-skinning math — a mesh deformer with no legal parameters to feed it.
3. **Invariants.** The obvious delivery-domain application — tracking or measuring a courier's body or performance — is a **direct violation of the CI-enforced no-courier-scoring rule (C6)**. Persistent biometric templates for re-identification additionally conflict with local-first/no-tracker invariants and create GDPR exposure in the launch market. These are not costs to weigh; they are lines the repo has already drawn.

**What survives, conditionally (P3's own narrow slices):**
- *One-time consented courier liveness/auth* — legitimate in principle, but P3 explicitly notes a **different, purpose-built verification tool** fits better than any of these three research systems. If the operator wants this, it is a fresh, separately-scoped evaluation (§9, O7) — not a salvage of this cluster.
- *Stylized (non-photorealistic) avatar animation or retail try-on* — conceptually clean of the biometric problem, but still behind the Meshcapade commercial gate, and no current dowiz surface demands it. Default: **drop**; revival requires the operator to fund the license against a concrete product need (§9, O2).

**Net verdict: D-H1/D-H2 rejected; D-H3/D-H4 parked as operator questions. No build phases exist for this lane.** Recording the rejection is itself the deliverable — the next person who proposes "pose-estimate the couriers" should hit this section and the CI gate at the same time.

---

## 8. Phased build order, with falsifiable acceptance per phase

Ordering principle, derived from the research's own cost/maturity findings: **ship the zero-hardware production-ready wow first (P2's browser AR); build the agent's substrate second (P4's two named gaps, in their stated priority order); put the new modality on top third (P5's cascade); do the industry-unsolved piece last (proactive voice); keep the hardware-adjacent display mode decoupled at the end.** Phases 1 and 2 touch disjoint surfaces (`web/` + asset serving vs. kernel retrieval + adapters) and may run as parallel lanes; everything after Phase 2 is a dependency chain.

### Phase 1 — AR storefront (customer wow; no new hardware; no kernel changes)
**Scope:** Asset upload/validation/storage for USDZ + GLB per menu item; storefront item page gains an AR badge via vendored `<model-viewer>` (or the zero-JS Quick Look link path on iOS); capture guidance doc for owners (single object, 30–60 s, per D-A4).
**Explicit non-scope:** any reconstruction compute on dowiz servers (D-A2); splat rendering (mesh-only, §3.3); whole-room capture.
**Acceptance (falsifiable):**
- A1.1 On a physical iPhone (Safari): item page → AR badge → Quick Look places the dish mesh in the room. No app installed.
- A1.2 On a physical Android (Chrome): same flow via Scene Viewer with the GLB.
- A1.3 An item with only one format shows the badge only on the platform it can serve — the degraded state is explicit, not broken.
- A1.4 `web/` remains build-step-free; the viewer component is a pinned static file; `cd kernel && cargo tree -e no-dev` output is byte-identical before/after (no new kernel deps — trivially true since the kernel is untouched, and asserted anyway).

### Phase 2 — Dense retrieval + rerank + semantic cache (D-C1, D-C3-spirit)
**Scope:** Exact-cosine vector index in `kernel/src/retrieval/` (pure-`std` over vectors obtained through the existing embed port; embedding acquisition stays feature-gated per C2); deterministic fusion with BM25/PPR; rerank port implemented for ≥1 backend; `SemanticOk` cache layer finished in `llm-adapters/src/cache.rs`; compacted-index prompt preload.
**Acceptance:**
- A2.1 RED→GREEN: a query that trigram/BM25 provably misses (paraphrase, zero lexical overlap) and dense retrieval hits — committed as a regression test.
- A2.2 Rerank on the wired backend returns ranked results; `Err(Unsupported)` remains the documented-valid answer only for backends genuinely lacking it.
- A2.3 Same corpus + same query ⇒ byte-identical ranking across runs (determinism holds through fusion).
- A2.4 Semantic cache: a paraphrased repeat query hits Layer-B; the exact-match layer's behavior is unchanged.
- A2.5 Default build stays clean: serde/ML dep count in `cargo tree -e no-dev` unchanged.

### Phase 3 — Read-only retrieval ToolPort (D-C2), then agentic loops (D-C4)
**Scope:** One new `ToolPort` tool in `agent-facade` (query → ranked passages; no mutation symbols exported); registered through the same admission/scope machinery as existing tools; then CRAG/Self-RAG-style retrieve-verify-retry prompting *within* the existing 4-iteration bound — no loop-architecture changes.
**Acceptance:**
- A3.1 RED→GREEN: a question answerable only from the corpus, which the agent could not answer with the prior two-tool surface, now answered with the passage cited in the observation trace.
- A3.2 Firewall audit: `agent-loop` still compiles with zero ability to name kernel mutation (the existing structural guarantee, re-asserted post-change).
- A3.3 The tool is deniable: with its capability withheld by scope, invocation fails closed.

### Phase 4 — Voice command loop (D-V1, D-V3, D-V4, D-V6)
**Scope:** New edge crate `voice-adapters` (cpal + wake-word + VAD + STT + Kokoro-via-`ort`), threads+channels only, importing only `agent-facade`; sync streaming read in `llm-adapters` with sentence-chunked TTS; STT bake-off (whisper-family vs. Moonshine) on actual hub hardware with recorded numbers; half-duplex barge-in v1 (mic gated during playback), AEC scheduled as v1.1.
**Acceptance:**
- A4.1 End-to-end: spoken "what's the status of order 42?" → wake → STT → agent-loop → `ReadOrderStatusTool` → spoken answer, fully offline, on the CPU-only hub. Wall-clock measured and recorded (target: first audible syllable within a small number of seconds; the measured number, not the hope, goes in the phase report).
- A4.2 **The privilege-escalation test (load-bearing):** spoken command targeting a red-line capability (ledger/refund) is **denied** by `scope.rs` `DenyByDefault`, with a test asserting the voice path and the typed path produce the same denial for the same request.
- A4.3 Streaming: first TTS audio begins before LLM generation completes (observable: audio-start timestamp < generation-end timestamp).
- A4.4 STT bake-off report: latency + WER-proxy numbers for both engines on hub hardware; Moonshine's sub-200 ms/5× claims marked reproduced or not reproduced.
- A4.5 `grep tokio` across the new crate and touched adapters: zero hits. Default builds of kernel/engine unchanged.

### Phase 5 — Projector display mode (D-B3) [decoupled; can slide]
**Scope:** A hub view designed for wall projection (contrast, type scale, glanceable order/courier/FSM-warning layout) rendered by the existing `web/` shell; works on any HDMI display — a plain $200–400 short-throw/pico projector is one option, not a requirement (P2). No interactivity on the projected surface (D-B2 stands); voice (Phase 4) and phone are the inputs.
**Acceptance:** A5.1 Legibility check at projection scale (defined viewing distance, defined minimum type size); A5.2 the view is display-only — it carries no session with mutation rights, so a passerby at the wall can *see* and cannot *do*.

### Phase 6 — Proactive spoken alerts (D-V5) [last; industry-unsolved]
**Scope:** FSM-event → allowlist filter → rate limiter → TTS, as a read-only observer channel per §6.4; per-owner class opt-in; quiet mode.
**Acceptance:**
- A6.1 RED→GREEN on the canonical case: injected "not picked up in N minutes" event class ⇒ spoken warning; an off-allowlist event ⇒ silence, asserted.
- A6.2 The alert channel demonstrably holds no ToolPort action capabilities (structural + test).
- A6.3 Rate bound: an event flood produces bounded speech, not a monologue.

---

## 9. Open decisions for the operator

These are deliberately **not** decided by this synthesis.

- **O1 — Hardware posture.** Does dowiz *recommend* (docs), *certify* (tested-with list), or *bundle* consumer hardware — specifically a $200–400 short-throw/pico projector for Phase 5, and optionally a Looking Glass ($199+) as showroom marketing (D-B4)? dowiz has never been a hardware vendor; each step up that ladder adds support surface. Phase 5's software is identical under all three answers, so this can be decided late — but bundling would need lead time.
- **O2 — The SMPL question, binary.** Fund a Meshcapade commercial license against a concrete future surface (stylized avatars / try-on, D-H4), or drop the human-capture cluster entirely? Default in this document: **drop**; nothing in Phases 1–6 depends on it.
- **O3 — The `web/` JS precedent.** Ratify vendoring `<model-viewer>` as a pinned static render-only asset (first JS component added since the 2026-07-15 JS drop), or mandate the zero-JS path (iOS Quick Look links + Android intent flow) at the cost of a less uniform cross-platform UX? Phase 1 is written to survive either ruling.
- **O4 — Asset-format pipeline ownership.** USDZ (iOS) vs. GLB (Android) duality (§3.3): accept the per-platform degraded state indefinitely, require owners to supply both, or invest in a conversion step (which would need its own tooling evaluation — none was researched in these passes, so none is proposed here)?
- **O5 — Streaming transport change (D-V6).** Approve modifying `llm-adapters/src/transport.rs` from hardcoded `"stream": false` to sync streaming — a real change to a load-bearing adapter that also affects non-voice consumers — or confine voice v1 to non-streaming latency (worse spoken UX, zero transport risk)?
- **O6 — The proactive-alert allowlist v1.** Which FSM event classes are speak-worthy by default? This is a product-judgment call (the fatigue literature says fewer is safer), and per the standing rule that operator sets direction, the initial set is the operator's to name.
- **O7 — Courier liveness/auth (D-H3).** Pursue a one-time consented verification capability at all? If yes, it is a *new* scoped research task for a purpose-built tool (P3: the three researched models are the wrong instruments), with GDPR/biometric handling as a first-class requirement — not an add-on to any phase here.
- **O8 — Voice-data retention.** The pipeline is fully local, but should the hub retain *anything* — audio never; transcripts as agent events, or nothing beyond the resulting `ChatRequest`? Local-first narrows but does not answer this; it touches the privacy posture and is the operator's call before Phase 4 ships.

---

## 10. Consolidated rejection ledger

For future-proposal collision detection; each entry cites its basis.

| Rejected | Why (source) |
|---|---|
| Server-side reconstruction (COLMAP/gsplat/OpenSplat/3DGS training) | CUDA-bound; no server GPU (P1, C1) |
| brush (Rust-native 3DGS training) as product dependency | Single experimental project; Burn+WebGPU; browser mode Chrome/Edge-only (P1) |
| Self-serve whole-room/interior capture | Needs pro/LiDAR gear (P1) |
| RoomAlive-style interactive projected AR appliance | Its one commercialization (Lightform LF2) shut down 2022; dowiz would become the failed integrator (P2) |
| Holo-fans / Pepper's-ghost marketed as volumetric | Front-arc 2D illusion; fails honesty bar (P2) |
| Voxon/laser-plasma volumetric displays as product | ~$6,800 showroom-grade / lab-only respectively (P2) |
| Literal CAG KV-cache preloading | Small-context local models; no firewall seam for KV state; determinism tension (P4, arXiv:2412.15605) |
| ANN index in Phase 2 | Unjustified dependency before a measured exact-scan latency failure (C2 + repo evidence discipline) |
| Kyutai Moshi / full-duplex speech-to-speech | GPU-bound; fuses comprehension with action, bypassing ToolPort gating (P5, C4/C5) |
| tokio/async anywhere in the voice path | Operator mandate; cpal's own idiom is threads+channels (C3, P5) |
| SMPL/GVHMR/FaceAnything deployment as-is | CUDA-mandatory + non-commercial licenses (US10395411B2, Meshcapade gate, CC-BY-NC) (P3, C1) |
| Any courier body/face/pose analytics | Direct violation of the CI-enforced no-scoring red line; biometric templates conflict with local-first + GDPR (P3, C6) |
| Voice as a privilege channel | Structurally impossible in the chosen cascade; enforced by test A4.2 (P5, C5) |

---

## 11. Source register

**Papers/projects (all from the five passes):** 3D Gaussian Splatting — Kerbl et al., SIGGRAPH 2023, arXiv:2308.04079 · PocketGS arXiv:2601.17354 · Mobile-GS arXiv:2603.11531 · COLMAP · gsplat · OpenSplat · brush (Burn+WebGPU) · web-splat · gauzilla · bevy_gaussian_splatting · Apple Object Capture · RoomAlive — Microsoft Research, UIST 2014 · Lightform LF2 · Looking Glass · Voxon VX2 · `<model-viewer>` / AR Quick Look / Scene Viewer / WebXR · SMPL — Loper et al., SIGGRAPH Asia 2015; patent US10395411B2; Meshcapade; SMPL-Body; SMPL-X · GVHMR — Shen/Pi/Xia et al., SIGGRAPH Asia 2024, arXiv:2409.06662; ViTPose; DPVO · FaceAnything — Kocasari et al., TU Munich/Niessner lab, targeting ECCV 2026 · CAG arXiv:2412.15605 · Self-RAG arXiv:2310.11511 · CRAG arXiv:2401.15884 · Agentic RAG survey arXiv:2501.09136 · whisper.cpp / whisper-rs / whisper-cpp-plus-rs · Moonshine arXiv:2602.12241 · candle · Piper · Kokoro-82M · `ort` · Home Assistant "Year of the Voice" / Wyoming protocol · openWakeWord · Silero VAD / silero-vad-rust · cpal (RustAudio) · Kyutai Moshi · Nielsen Norman Group / Smashing Magazine (voice-alert UX).

**Repo files verified live (2026-07-20):** `kernel/src/retrieval/{index,bm25,ppr,diffusion}.rs` · `kernel/src/csr.rs` · `kernel/src/ports/llm.rs` (embed port; rerank `Err(Unsupported)` documented at line 377) · `kernel/src/ports/agent/scope.rs` (`RedLinePolicy::DenyByDefault`, line 268) · `agent-facade/src/lib.rs` (`ReadOrderStatusTool` line 59; `WebFetchTool` line 130) · `agent-loop/src/lib.rs` (`TOOL_TIMEOUT_MS`, watchdog thread + `recv_timeout`, lines 25–27) · `llm-adapters/src/transport.rs` (`"stream": false`, line 50) · `llm-adapters/src/cache.rs` (`SemanticOk` named-unbuilt, line 129) · `llm-adapters/src/ollama.rs` · `kernel/src/order_machine.rs`.

---

*End of synthesis. Nothing above is built. Phases 1–6 and open decisions O1–O8 await operator review.*
