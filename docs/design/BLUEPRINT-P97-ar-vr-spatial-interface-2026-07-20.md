# BLUEPRINT P97 — AR/VR spatial interface: consolidated (2026-07-20)

> **Consolidation blueprint, not a from-scratch design.** AR/VR content already existed in this
> corpus in four separate places, none of them a first-class roadmap item. This document is the
> single synthesis of all four, reconciled where they overlap, and is now the canonical home for
> "what is dowiz's AR/VR strategy." It does not re-derive facts the sources already established —
> it cites them, resolves the one real scope question between them (near-term product AR vs.
> long-term spatial-computing readiness), and gives the item a falsifiable DoD of its own. Format
> follows the `BLUEPRINT-P64-intent-engine-friction-voice.md` / `BLUEPRINT-P38-webgpu-render-engine.md`
> precedent (ground truth → scope → build items → acceptance → dependencies → anti-scope).

---

## 0. Ground truth — the four sources, re-verified live this pass

| # | Source | What it actually contains | Cite (verified this pass) |
|---|---|---|---|
| S1 | DZ-12 — "Cross-platform (WebGL2 fallback / native / AR panel)" | A P38b sub-unit (not a first-class item): field→off-screen-texture→curved-panel AR, native OpenXR Quest first, WebXR-WebGPU deferred until `XRGPUBinding` lands, ray→panel→`FieldPos` = the *same* `Intent` as a 2D pointer | `docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md:344-354` |
| S2 | P38-rev §12.2 — "AR/VR insurance constraints — four HARD requirements" | A canon-diff on the render engine itself: (1) view/projection-matrix-driven pipeline end-to-end, (2) `FieldPos` 3D end-to-end (never truncated), (3) all input routed through `InputSource` (pointer/voice/gesture/**future XR controller**), (4) exactly one XR seam where a backend (OpenXR/WebXR) supplies per-eye matrices + pose `Intent`s. Structural insurance **paid now**; the actual XR backend stays Track-R | `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P38-webgpu-render-engine.md:694-737` |
| S3 | §17.5 ruling — "Build in AR/VR/new-form-factor readiness now" | Operator ruling: reverses "not priority yet," ties the extension explicitly to the *ad fontes* physics/math foundation (§16.42) extending naturally to spatial interfaces, named as real added scope to the already-large UI research program | `docs/design/ROADMAP.md:3091-3099` (§17.5) |
| S4a | `BLUEPRINT-SPATIAL-STOREFRONT-VOICE-HUB-SYNTHESIS-2026-07-20.md`, Lane B (Display) | D-B1 (browser `<model-viewer>` → AR Quick Look/Scene Viewer, **BUILD, Phase 1, first ship**), D-B2 (RoomAlive-style projected AR, **REJECTED**), D-B3 (dumb short-throw projector as a "hub display mode," **BUILD, Phase 5**), D-B4/D-B5 (light-field/swept-volume/Pepper's-ghost, **REJECTED as product**), the USDZ/glTF format-duality gap (O4, open), Phase 1's falsifiable acceptance (A1.1-A1.4) | `docs/design/BLUEPRINT-SPATIAL-STOREFRONT-VOICE-HUB-SYNTHESIS-2026-07-20.md` §2 (decision register D-B1–D-B5), §4, §8 Phase 1/5, §9 O1/O3/O4 |
| S4b | `BLUEPRINT-INTENT-INTERFACE-ONE-SCREEN-2026-07-20.md` §4.2, §7 | "Report 2" (native AR compositing) triaged into **3 buckets**: (1) already-shipping via S4a's `<model-viewer>`, (2) the real native-camera-compositing stack — **deferred behind an explicit, unmade native-companion-app decision**, (3) non-AR-specific techniques (curl noise, LUT brand-grading, degrade-to-match-camera) reusable in the browser today, independent of any camera. Build-sequence Stage A/B/C names the AR-adjacent Stage-B/C gating precisely | `docs/design/BLUEPRINT-INTENT-INTERFACE-ONE-SCREEN-2026-07-20.md:141-230` |

**One correction this pass makes explicit:** the task that produced this blueprint cited S4b's
bucket/stage content as if it lived inside S4a. It does not — S4a (spatial-storefront-voice-hub)
covers Lane B's *display* decisions (D-B1–D-B5) and Phase 1/5 acceptance; the "3 buckets" / "Stage
A/B/C" native-AR-compositing triage is S4b (intent-interface), which explicitly cross-references
S4a's Phase 1 ruling rather than repeating it. Both are real, in-tree, and both are folded in here
— this is a reconciliation, not a transcription error carried forward.

**Code-grounded facts used below (re-verified live):**
- `engine/Cargo.toml` declares `gpu`/`webgl`/`webgpu`/`splat` as deliberate empty feature seams;
  the default engine build is zero-external-crate (`dowiz-kernel` only) — confirmed by direct read
  this pass.
- `Intent`/`FieldPos`/`InputSource` are the P38 §11.2 grammar; `FieldPos { u, v, w }` is already
  declared 3D (P38-rev §12.2 constraint 2 makes that load-bearing, not decorative).
- P64 (`BLUEPRINT-P64-intent-engine-friction-voice.md`) is the concrete implementation of P38-rev
  §12.2 constraint 3 — `VoiceSource`/gesture sources already exist as the `InputSource` model an
  eventual XR-controller source would follow with **zero** change to the intent surface (P38-rev
  §12.2 constraint 4).

---

## 1. Why this is one item, and the two-track reconciliation

Reading S1–S4b together surfaces one real structural fact: **"AR" in this corpus names two
genuinely different capabilities that happen to share a name and a four-letter acronym.**

- **Track 1 — Product AR (ships first, customer-facing, no headset).** A customer points a phone
  at a menu item and sees a 3D model of the dish on their table via the browser, using
  Google's `<model-viewer>` web component auto-routing to platform-native AR (Quick Look on iOS,
  Scene Viewer on Android). Zero dowiz render-engine involvement — it is a vendored web component
  outside the wgpu pipeline entirely (S4a D-B1, ratified as decision O3).
- **Track 2 — Spatial-computing readiness (structural insurance, backend deferred).** dowiz's own
  field-UI render engine (Sea & Sheet, wgpu) is architected so that the *entire interface* — not
  one 3D asset — can eventually be viewed as a floating curved panel inside an XR headset (Quest
  via OpenXR first, browser via WebXR once `XRGPUBinding` ships), with the same `Intent` model
  driving a hand-ray the way it drives a mouse pointer today (S1, S2).

These are not competing designs to pick between — **both are real, both are already-ruled BUILD
or insurance-paid decisions, and they are deliberately NOT unified into one mechanism today.**
Track 1 exists entirely outside the wgpu engine (a pinned static JS asset, S4a's own "repo-
discipline note," O3); Track 2 exists entirely inside it (the four structural constraints are
render-core requirements, S2). The one thing that *would* unify them — rendering a `<model-viewer>`-
class object AR experience natively inside the wgpu engine instead of via a browser web component —
is not proposed anywhere in the four sources and is not proposed here either; it would need its own
justification against Track 1's already-shipping, zero-effort path. This blueprint records that
non-unification as a deliberate scope boundary (§4), not an oversight.

**What makes this one first-class item rather than two:** every one of the four scattered sources
frames itself as "AR" work, every one of them cites the *ad fontes* physics-first UI philosophy
(§16.42) as its rationale, and an operator auditing "dowiz's AR story" needs one door, not four.
The item is P97; its two tracks are named and scoped separately inside it so neither is diluted by
the other's constraints (Track 1 has no wgpu dependency; Track 2 has no `<model-viewer>`/JS
dependency).

**Explicitly out of this item's scope** (stays exactly where it already lives): the asset-capture
pipeline that produces the 3D dish models Track 1 displays (spatial-storefront-voice-hub Lane A —
capture happens on the owner's phone, dowiz only ingests/stores/serves; zero overlap with AR
display mechanics); the dense-retrieval/agent lanes (Lane C) and voice lanes (Lane D) of the same
synthesis document; human/body capture (Lane E, rejected outright, not an AR concern). Those stay
registered under the existing "product-surface wave" entry in `CORE-ROADMAP-INDEX.md` §7 — this
item does not re-home them.

---

## 2. Track 1 — Product AR: browser `<model-viewer>`, ships first

### 2.1 What ships (Phase 1, restated as the canonical acceptance — source S4a §8)

**Scope:** asset upload/validation/storage for USDZ + GLB per menu item; storefront item page
gains an AR badge via a vendored, pinned, single-file `<model-viewer>` static asset in `web/` (no
npm dependency tree, no build step — consistent with `web/`'s "renders only, never re-implements
math" charter, since the AR viewer computes no dowiz-authoritative state); a zero-JS Quick-Look
link path on iOS Safari as a fallback with no JS at all; a capture-guidance doc for owners (single
object, 30-60s phone capture).

**Explicit non-scope:** any reconstruction compute on dowiz servers; splat rendering (mesh-only —
system AR viewers are mesh-based, USDZ/glTF, not 3D-Gaussian-Splat); whole-room capture.

**Falsifiable acceptance (RED→GREEN, unchanged from S4a — this is the canonical restatement, not
a rewrite):**
- **T1.1** On a physical iPhone (Safari): item page → AR badge → Quick Look places the dish mesh
  in the room. No app installed.
- **T1.2** On a physical Android (Chrome): same flow via Scene Viewer with the GLB.
- **T1.3** An item with only one format shows the badge only on the platform it can serve — the
  degraded state is explicit, not broken (see the O4 gap, §2.2).
- **T1.4** `web/` remains build-step-free; the viewer component is a pinned static file;
  `cd kernel && cargo tree -e no-dev` output is byte-identical before/after (kernel untouched,
  asserted anyway); `cd engine && cargo tree -e no-dev` likewise unchanged (Track 1 never touches
  the render engine).

### 2.2 The one open engineering gap (O4, carried forward unresolved)

iOS AR Quick Look consumes USDZ; Android Scene Viewer consumes glTF/GLB. Apple's on-device capture
emits USDZ natively; there is no dowiz-side conversion tool yet. **Ruled (S4a §9 O4): invest in a
conversion step** (candidate landscape: Apple's own `usdzconvert`/USD toolchain, `gltf-transform`,
Blender's Python API as a batch-conversion backend) rather than accepting the degraded per-platform
default or pushing dual-capture onto owners. This is real, unscoped follow-on tooling work — not
designed further here; the falsifier (T1.3) already covers the honest interim state.

### 2.3 Native AR compositing — triaged, not built (S4b's 3-bucket disposition)

The "should dowiz build a real native camera-AR app" question is resolved into three disjoint
buckets, restated here as the AR item's own registry (not re-derived — S4b §4.2 is authoritative):

| Bucket | Content | Disposition |
|---|---|---|
| **1 — already shipping without it** | The customer's first AR moment (§2.1) needs none of a native compositing stack. Report 2's own evidence *closes* the browser path for true live camera compositing (WebXR `immersive-ar` absent from iPhone Safari; 8th Wall's hosted SLAM shut down; WebXR↔WebGPU binding still an unstable Editor's Draft) — which is exactly why Track 1 uses `<model-viewer>`'s Quick-Look/Scene-Viewer routing instead of attempting live camera AR in-browser. | **No action** — already the shipped design. |
| **2 — Report 2's actual native stack** | Zero-copy camera import (`CVPixelBuffer`/`AHardwareBuffer`), 6DoF planar tracking or ARKit/ARCore VIO, depth-only proxy-geometry occlusion, stencil-buffer portal, degrade-to-match-camera compositing — all requiring a **native companion app** (Rust + wgpu behind thin Swift/Kotlin FFI shells). | **Deferred behind an explicit, unmade native-companion-app product decision** (distribution/maintenance/support cost, not an engineering call). Zero code, zero blueprint work, until decided. Named as open decision D2 in §7. |
| **3 — non-AR-specific reusable techniques** | Curl-noise ambient particles, LUT-based brand-token palette grading, "degrade-to-match" as a shader discipline, 2.5D layered planes over full 3D. All apply to the *already-loaded* storefront page, independent of any camera. | **Adopted, but owned by the Sea/Sheet render substrate (P38b / `BLUEPRINT-INTENT-INTERFACE-ONE-SCREEN-2026-07-20.md` §5, Stage B), not by this AR item.** Cross-referenced here so the AR reader knows where they live; not duplicated. |

Bucket 2 is the only genuinely open AR decision left in Track 1. Until the native-companion-app
question is answered, **web-first stands, `<model-viewer>` is the whole AR story**, and no Stage-C
work (S4b's own naming) proceeds.

### 2.4 Adjacent, honestly not AR: the projector "hub display mode" (D-B3, Phase 5)

Recorded for completeness because it was one of the four scattered sources' concerns (§17.5's
"new-form-factor" framing), but stated plainly: a plain $200-400 short-throw/pico projector
rendering a wall-legible owner-hub view (order queue, courier positions, FSM-state warnings) is
**not augmented reality** — it is a second dumb display surface with all interactivity staying on
phone/keyboard/voice. It survives S4a's own honesty pass specifically *because* the taxonomy there
(D-B2/D-B4/D-B5 rejections) ruled out every display technology that could plausibly be called AR/
volumetric. Included here as Track 1's adjacent, lower-priority Phase 5 item so it is not
orphaned by this consolidation; its acceptance criteria are S4a §8 Phase 5 A5.1/A5.2, unchanged.

---

## 3. Track 2 — Spatial-computing readiness: the render-core insurance

This track is **owned by `BLUEPRINT-P38-webgpu-render-engine.md` §12.2** — the four structural
constraints are render-core requirements and stay specified there as the single source of truth;
this section registers Track 2 as part of the P97 item and states the sequencing gate DZ-12 adds
on top, without re-deriving the constraint text (append-only discipline — this repo does not fork
canon into two places that can drift).

**The four constraints, by reference (full text: P38 §12.2, `BLUEPRINT-P38-webgpu-render-engine.md:702-731`):**
1. View/projection-matrix-driven pipeline end-to-end — `FrameUniforms` gains `view_proj`, 2D screen
   becomes the orthographic-identity configuration of the *same* matrix path, never a hardcoded
   second path.
2. `FieldPos` is 3D end-to-end — the already-declared `w` field must survive unchanged from intent
   through the vertex buffer, no stage collapses to `(u, v)`.
3. All input routes through `InputSource` — pointer, keyboard, voice, gesture, **and any future XR
   controller** normalize to `Intent`; P64's `VoiceSource`/gesture sources are the concrete proof
   this seam works before an XR controller ever exists.
4. Exactly one XR seam — the single extension point where OpenXR (native) or WebXR (web) supplies
   per-eye `view`/`projection` matrices and pose-derived `Intent`s; the render core consumes them
   unchanged, no restructuring when the backend eventually lands.

**DZ-12's sequencing gate on top of those four (`BLUEPRINTS-DOWIZ-INTERFACES.md:344-354`):** once
the seam exists, the concrete backend order is **native OpenXR (Meta Quest) first**, **WebXR-WebGPU
deferred until `XRGPUBinding` ships** (an unstable Editor's Draft per S4b's own citation — the same
fact both S1 and S4b independently landed on). The interaction contract is fixed now, ahead of any
backend: a hand-controller ray hitting the curved panel produces the *same* `Intent` a 2D pointer
click would (`ray → panel → FieldPos`), so no separate input-handling code path is ever written for
the XR case.

**What is NOT built by Track 2 (Track-R, explicit — P38 §12.2's own framing, restated):** no XR
device support ships in Wave-0. The four constraints are *shape* requirements on the render core so
that adding OpenXR/WebXR later requires zero restructuring — they are not a feature build. A build
that ships a 2D-only transform path, a `FieldPos` truncated at any stage, an input handler bound to
a concrete event type outside `InputSource`, or more than one (or zero) XR seams is **NOT done**,
per P38 §12.2's own not-done clause — inherited here verbatim, not weakened.

---

## 4. Reconciliation — why Track 1 and Track 2 stay separate (stated as a decision, not a gap)

Nothing in S1-S4b proposes rendering Track 1's dish-AR experience natively inside the wgpu engine
instead of via `<model-viewer>`, and this blueprint does not propose it either. The honest reason:
Track 1's browser path is **already production-viable, zero-effort, and ships today**; folding it
into the wgpu render core would mean re-solving camera compositing, 6DoF tracking, and platform AR
session APIs from scratch inside dowiz's own renderer — exactly Bucket 2's rejected/deferred scope
(§2.3), for a capability the platform browsers already give away for free. **The two tracks share
one thing on purpose: the `Intent`/`InputSource` contract (§3, constraint 3) is general enough that
if a native companion app (Bucket 2) is ever built, its camera-ray input would plug into the exact
same seam an XR controller would** — so choosing Track 2's insurance now is not wasted even if
Bucket 2 stays unbuilt indefinitely. No code exists for either backend today; this is recorded as
architecture, not implementation.

---

## 5. Consolidated rejection ledger (for future-proposal collision detection)

| Rejected / adjacent-not-AR | Why | Source |
|---|---|---|
| RoomAlive-style interactive projected AR appliance | Its one commercialization (Lightform LF2) shut down 2022; dowiz would become the failed integrator | S4a D-B2 |
| Light-field (Looking Glass), swept-volume (Voxon), laser-plasma displays as product | Screen-you-look-at / showroom-priced / lab-only respectively | S4a D-B4 |
| Pepper's-ghost / spinning-LED "holo-fans" | Front-arc-only 2D illusion, marketed as volumetric — fails the honesty bar | S4a D-B5 |
| Server-side reconstruction (COLMAP/gsplat/OpenSplat/3DGS training) | GPU-bound; no server GPU (C1) — belongs to the capture Lane A, not this item, noted for completeness | S4a §3 (out of this item's scope, §1) |
| Native camera-AR compositing (Bucket 2) | Not rejected — **deferred** behind an unmade native-companion-app product decision | S4b §4.2, this doc §2.3 |
| WebXR-WebGPU backend | Not rejected — **sequenced after OpenXR**, blocked on an unstable spec (`XRGPUBinding`) | S1, S4b |
| Rendering Track 1's dish-AR inside the wgpu engine instead of `<model-viewer>` | Not proposed by any source; would re-solve an already-free platform capability | This doc, §4 |

---

## 6. Dependencies

**Consumes:**
- **P38** (`BLUEPRINT-P38-webgpu-render-engine.md` §11.2/§12.2) — the `Intent`/`FieldPos`/
  `InputSource` grammar and the four structural constraints Track 2 is built on; P97 does not
  redefine them.
- **P64** (`BLUEPRINT-P64-intent-engine-friction-voice.md`) — proof-of-pattern that `InputSource`
  absorbs a non-pointer modality (voice) with zero change to the intent surface; the model an XR
  controller source would follow later.
- **P58** (a11y mirror) — Track 2's eventual XR panel must still satisfy the same mirror/fallback
  contract as the 2D surface; named, not designed here (P58 owns the mechanism).
- **Spatial-storefront-voice-hub Lane A** (capture) — supplies the USDZ/GLB assets Track 1 displays;
  zero overlap in mechanism, a hard data dependency.

**Feeds:**
- **P69** (customer storefront & checkout) — the AR badge lives on the item Detail fragment.
- Any future native-companion-app decision (Bucket 2) would consume Track 2's `InputSource` seam
  for camera-ray input.

---

## 7. Open decisions carried forward (restated, not re-litigated)

- **D1 — Asset-format conversion tooling (O4, §2.2):** RULED to build a USDZ↔glTF conversion step;
  the concrete tool choice is unscoped follow-on work.
- **D2 — Native companion app, ever? (Bucket 2, §2.3):** genuinely unmade. Until decided: web-first
  stands, zero Stage-C/Bucket-2 work in any direction.
- **D3 — Hardware posture for the projector display mode (O1):** RULED — dowiz tests specific
  projector/display models and publishes a "known good" list; no inventory/reseller commitment.
- **D4 — `web/` JS precedent (O3):** RULED and ratified — `<model-viewer>` is the first JS
  component in `web/` since the 2026-07-15 drop, vendored/pinned/zero-build-step.

---

## 8. Acceptance criteria for the P97 item as a whole (falsifiable, "verified not claimed")

A build that claims P97 "done" must satisfy, together:
- **Track 1:** T1.1-T1.4 (§2.1) all green on physical hardware (not simulator/emulator only) — a
  claim of T1.1/T1.2 passing in a headless test harness without a physical-device run is **not**
  acceptance; the falsifier is a real iPhone/Android in front of the interface.
- **Track 2:** the four P38 §12.2 falsifiers (§3) all green — the ortho-identity bit-for-bit render
  match, the non-zero-`w` round-trip test, the `InputSource`-only grep gate, and the single-XR-seam
  structural check. No XR backend needs to exist for these to pass; they gate the *shape*, not a
  feature.
- **Consolidation itself:** the four original scattered sources (S1-S4b) each carry a pointer to
  this document (§9) rather than an orphaned, un-cross-referenced AR mention; `ROADMAP.md`'s
  summary and `CORE-ROADMAP-INDEX.md` both carry a P97 row.

Anything short of both tracks' falsifiers plus the reachability requirement is **PLAN**, not DONE —
consistent with every other blueprint in this corpus.

---

## 9. Where the four sources now point (registration record)

- `docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md` DZ-12 — pointer added: "consolidated
  under P97."
- `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P38-webgpu-render-engine.md` §12.2 — pointer added
  after the four constraints: "registered as P97 Track 2."
- `docs/design/ROADMAP.md` §17.5 — pointer added: "P97 is this ruling's first-class home."
- `docs/design/BLUEPRINT-SPATIAL-STOREFRONT-VOICE-HUB-SYNTHESIS-2026-07-20.md` Lane B header —
  pointer added: "Lane B's AR/display decisions are registered as P97 Track 1; this section is
  preserved as the source research, not duplicated."

Registered in `docs/design/ROADMAP.md` (Part I, new §16) and `docs/design/CORE-ROADMAP-INDEX.md`
(new §11).
