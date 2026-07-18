# BLUEPRINT P57 — Canvas text input & editing (Latin + Cyrillic) (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §10). Wave **W1**,
> component **DELIVERY / interface**. Scope authority: `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md`
> §5 (W1 table, row **P57**), the closed operator ruling §0.2-2, and the two cross-cutting
> resolutions X2 (the IME ruling) and X1 (a11y-mirror-everywhere). Render substrate:
> `BLUEPRINT-P38-webgpu-render-engine.md` — this blueprint is the **replacement** the P38 §12.1
> canon-diff names when it strikes G6's transparent-`<input>` overlay. Research grounding:
> `docs/research/OPUS-R1-INTERFACE-RENDERING-2026-07-18.md` §3 (cosmic-text/parley findings),
> §2 (AccessKit / a11y-mirror). Structural template + rigor precedent:
> `BLUEPRINT-P51-open-map-routing.md`; sibling W1 blueprint (a11y convention owner): **P58**.
>
> **Binding scope is a closed operator decision — this document makes it buildable, it does not
> re-litigate it.** Fully custom text input rendered inside the wgpu canvas; **no DOM `<input>`
> on any platform**; Wave-0 supports **Latin + Cyrillic only**; scripts requiring IME composition
> (Arabic, CJK, Thai, Indic) are **explicitly deferred to v2** (§2 anti-scope). The text model is
> `cosmic-text` rendered through the existing MSDF glyph pipeline (FE-06); the editor is
> engine-owned, cosmic-text supplies buffer/cursor/selection/shaping/clipboard, the engine owns
> the event surface and the render/a11y projections.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads. The single most load-bearing finding:
**no text-input code of any kind exists yet** — cursor, selection, clipboard, keydown wiring,
shaping, glyph render: P57 starts from zero, on top of a landed, tested field-render substrate.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| **Zero text-input code anywhere.** grep `cosmic\|cursor\|caret\|selection\|clipboard\|keydown\|keyup\|textinput\|contenteditable\|<input\|graphemes` over `engine/src wasm/src web/src` → **0 hits** | repo-wide grep this pass | **VERIFIED — P57 is greenfield for the editor; only the linguistics come from a crate** |
| **Zero glyph/MSDF code anywhere** (FE-06 not yet built) | grep `glyph\|msdf` over `engine/ wasm/ web/` → 0 files (P38 §0 re-confirmed this pass) | VERIFIED — P57's render leg rides FE-06 **once it lands**, it does not pre-empt it |
| Engine is **offline-clean by mandate**; `cosmic-text` explicitly OUT OF SCOPE until a network unlock, added behind a feature later | `engine/Cargo.toml:5-7` ("wgpu/cosmic-text are OUT OF SCOPE here (added behind features later) so the default build has zero external crates"), `:22-24` | **VERIFIED — cosmic-text is in the O18a manifest (P38 §2), same one network grant as wgpu** |
| Feature-gate discipline already exists: `gpu`/`webgl`/`webgpu`/`splat` are EMPTY by design; default build pulls zero external crates | `engine/Cargo.toml:27-45` (`default = []`, each feature empty) | VERIFIED — P57 adds a `text` feature in the same shape (§3) |
| `compose(scene, eq, w, h, steps) -> Vec<u8>` — the bit-deterministic physics-state→RGBA oracle | `engine/src/field_frame.rs:218` (P38 §0's oracle, re-cited) | VERIFIED — text glyphs render **through** this frame, not beside it |
| Scene + SDF primitives exist (CPU): `SdfShape::{Circle,Box,RoundedBox,LineSegment}`, `Scene::{add,render_frame,render_into,render_to_bridge}` | `engine/src/scene.rs:29-44` (variants), `:71` (`Scene`), `:88,122,143,168` | VERIFIED — caret/selection quads are Scene geometry; glyph quads are FE-06's |
| SDF toolkit (rounded-box for the caret/selection rects, line-segment for the caret bar) | `engine/src/sdf.rs:19,29,41,53` (`sdf_circle/box/rounded_box/line_segment`), `:124` (`SdfField::rasterize`) | VERIFIED — caret geometry reuses these, no new SDF math |
| Critically-damped `Spring` (ζ=1 monotone, no overshoot) with `snappy/fluid/playful` presets | `engine/src/motion.rs:14` (`Spring`), `:29` (`new`), `:66-77` (presets), `:80` (`zeta`) | VERIFIED — caret focus-fade / blink alpha rides `Spring`, a `FieldValue` channel (never money) |
| **Money-never-tween guard LANDED and binding (🔴 RED-LINE)**: `Money(i64)` implements NO `FieldValue`; `TweenGuard::present_money` rejects fractional | `engine/src/money_guard.rs:18` (`Money`), `:22-25` (`FieldValue`, deliberately not for `Money`), `:60` (`present_money`), `:72` (`jump`) | VERIFIED — a P57 text field is a `&str`; a typed money amount NEVER enters the caret animation (§5.1) |
| SoA store + particle ring (marker/particle seeds ride this) | `engine/src/widget_store.rs:14` (`WidgetStore`), `:68` (`ParticlePool`) | VERIFIED — P38 §0 re-confirmed |
| Public engine surface exports the render primitives P57 consumes; **no editor symbol exported** | `engine/src/lib.rs:32-42` (`Scene`/`SdfShape`/`Spring`/`Money`/`TweenGuard`/`VertexBridge` exported; no `TextField`/`EditCmd`) | VERIFIED — P57 registers a new `text_input` module + exports |
| wasm surface: `compose_field`, `FieldSim{new,step,frame,width,height}`, `vertex_field`, `knowledge_map/lookup_tag/related_docs`; **no keydown/text/input export**; retrieval exports still mixed in (P38 G7 separates them) | `wasm/src/lib.rs:56-114,152-170` | **VERIFIED gap — P57 adds `text_*_js` exports per P38 G7 ptr/len conventions** |
| web app is a Svelte/Astro FieldSim render path + a console-only kernel driver binding 24 kernel `_js` exports; **no a11y mirror, no keyboard handling, no `web/tests/`** | `web/src/app.mjs:1-12` (header), `:14-40` (24 binds); tree = `web/src/{app.mjs,components/FieldSim.svelte,lib/fieldsim/*,lib/kernel/*,pages/fieldsim.astro,render/fieldsim.smoke.mjs}`; `ls web/tests` → absent | VERIFIED — P57 adds the canvas keyboard source + rides P58's `a11y_mirror.mjs` |
| O18a `graphics-unlock` gate: `cargo add wgpu`/`cosmic-text` is network, operator-granted, verified RED 2026-07-16, shared with P38/P17 | `docs/design/BLUEPRINT-W21-field-ui-gpu-blocked.md:3,26` (P38 §0 re-cited) | VERIFIED — P57's cosmic-text half inherits this exact gate (§3, §11 Lane B) |
| P38 §12.1 canon-diff **names P57 as the owner** of the struck overlay and of every typed field across P69/P70/P71/P73; tightens the grep gate to forbid any `<input>`/`contenteditable` | `BLUEPRINT-P38-webgpu-render-engine.md:656-692` (§12.1), `:739-750` (§12.3 FE-15/FE-16 shared base) | VERIFIED — P57's DoD inherits the tightened not-done clause (§6) |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. The binding decision, restated + the research it rests on (not re-opened)

### 1.1 The closed ruling (SYNTHESIS §0.2-2 / X2), restated so every item below is checkable

1. **Fully custom in-canvas text input. No DOM `<input>` on any platform** — cursor, selection,
   clipboard, keydown wiring all hand-built over the field engine (MASTER-ROADMAP §16.34, now in
   canon per P38 §12.1). The only DOM P57 touches is the **semantic a11y mirror** (P58/FE-15) —
   a *projection*, never an input.
2. **Wave-0 script scope = Latin + Cyrillic only.** These scripts need **no IME composition**, so
   the entire `keydown → EditCmd → buffer → shaped glyphs → GPU` path is self-contained (X2).
3. **IME-composition scripts (Arabic, CJK, Thai, Indic) are deferred to v2** — a scope boundary
   consistent with §16.58's RTL-deferred-to-v2 ruling, **not** a new exception (§2 anti-scope
   names it clearly and does not half-build it).
4. **Text model = `cosmic-text`** (R1 §3's first choice — the editing surface is the more mature).
   parley is the named alternative only if a `WidgetStore` layout prototype later favours it; that
   swap does not change P57's engine-owned event surface (§2 anti-scope).

### 1.2 Why cosmic-text, not a hand-rolled text model (R1 §3, cited — the *ad fontes* boundary)

R1 §3 verified that the hard **algorithmic** parts of text editing are **not greenfield**:
`cosmic-text` (pure Rust, battle-used in COSMIC desktop) supplies shaping (HarfRust), font
discovery, fallback, layout, rasterization, and a real **editing** layer — cursor management,
**selection, copy/paste**, grapheme clustering, cursor motion, wrapping. Re-implementing
HarfBuzz-class shaping and Unicode grapheme segmentation from scratch would be a genuine
**correctness regression, not a simplification** — a text-shaping crate is a first-principles
Unicode primitive in the *same category §16.43 keeps for crypto* (SYNTHESIS X2). So the *ad fontes*
line for P57 is: **cosmic-text owns the linguistics (buffer/shaping/cursor-motion/selection);
the engine owns everything downstream** — the event alphabet, the render projection onto FE-06's
MSDF glyph quads, the a11y projection, the keydown wiring, and the Latin+Cyrillic scope boundary.
Greenfield **only** for the GPU render and the input event wiring, exactly as R1 §3 recommends
("Latin-script Wave-0 text input is fully in-canvas and achievable").

The keystroke-latency bar is set by real prior art: GPUI/Zed renders a full live-editing code
surface at **2 ms keystroke-to-pixel** (R1 §1). P57's budget (§7) is measured against that class,
not asserted.

### 1.3 What is *still* genuinely custom in P57 (R1 §3, the honest residue)

- **The event wiring** — `keydown`/pointer/clipboard → a closed `EditCmd` alphabet — native
  (winit) and web (canvas + wasm), both hand-built (§3, §4.3/§4.4).
- **The GPU render** — cosmic-text's shaped layout glyphs → FE-06's MSDF glyph-quad instances;
  caret + selection rects as Scene SDF geometry (§4.5). No bespoke text renderer.
- **The a11y projection** — AccessKit text-run + selection nodes on native (production-ready per
  R1 §2); a **synthetic ARIA-textbox** mirror on web (forced by ruling §0.2-2 — the
  hidden-editable-element variant is ruled out) (§4.6). P58 owns the canonical web convention;
  P57 defines a reasonable approach and **must reconcile with P58's final convention** (§4.6, §12).
- **The Latin+Cyrillic scope boundary** — a pure classifier that refuses out-of-scope codepoints
  at the buffer edge, so v2's IME work is a *bounded future project, not a Wave-0 unknown* (X2).

Rejected alternatives (DECART one-liners): **hidden `<input>`/contenteditable overlay to get IME
+ mobile keyboard + caret-a11y "for free"** — rejected: it is exactly the overlay §16.34 forbids
and P38 §12.1 struck; it is the R1 §0 contradiction, now closed by ruling, not re-openable here.
**Hand-rolled shaping/grapheme segmentation** — rejected: correctness regression vs HarfRust/
cosmic-text (R1 §3); a Unicode primitive is *ad fontes*-kept, not re-derived. **parley Wave-0** —
not rejected, *deferred*: cosmic-text's editing surface is more mature today (R1 §3); parley's
"wants to own a full rectangle" friction against the immediate/field model is unresolved and
would be prototyped before any swap — and the swap is invisible to P57's event surface by design.
**A bespoke in-canvas soft-IME candidate UI for non-Latin** — rejected for Wave-0: large,
unprecedented (R1 §3/§9 risk #1); it is the v2 project, named not built (§2).

---

## 2. Scope — what P57 owns vs deliberately does NOT

### 2.1 P57 owns (build items §4)

| Item | Content |
|---|---|
| M1 | **Latin+Cyrillic classifier** (`text_scope.rs`, pure, no cosmic-text) — the v2 scope boundary as a testable refusal, lands **pre-unlock** |
| M2 | **`EditCmd`/`EditEvent` model + `TextField`** editor (`text_input.rs`, cosmic-text-backed, `text` feature / O18a) — buffer, cursor, selection, clipboard, applied as an event sequence |
| M3 | **Keydown wiring, native** — winit `KeyEvent` + `Ime::Commit` (dead-key Latin) → `EditCmd` via `KeyboardSource: InputSource`; clipboard through `ClipboardPort` |
| M4 | **Keydown wiring, web** — focusable `<canvas tabindex=0>` `keydown`/`copy`/`cut`/`paste` → `EditCmd` → wasm `text_apply_js`; clipboard via async Clipboard API behind the same `ClipboardPort` |
| M5 | **Render leg** — cosmic-text shaped glyphs → FE-06 MSDF glyph quads; caret bar + selection rects as Scene SDF; caret blink/fade via `Spring` **without waking the field integrator** |
| M6 | **a11y live-edit projection** — AccessKit text-run + `SetTextSelection` on native (R1 §2); synthetic ARIA-textbox mirror on web, riding P58's `a11y_mirror.mjs` (reconciles with P58's final convention) |
| M7 | **wasm exports** per P38 G7 ptr/len conventions: `text_new_js`, `text_apply_js`, `text_value_js`, `text_caret_js`, glyph-run `ptr/len` |

### 2.2 P57 explicitly does NOT own

- **NOT any DOM `<input>` / `contenteditable` / editable element on any platform** — hard ruling
  (§16.34, P38 §12.1), not a preference. A diff that creates one is a scope violation **regardless
  of test state**; falsified by the tightened grep gate (§6). The ONLY DOM P57 touches is the
  a11y ARIA-textbox mirror node — a non-editable `role="textbox"` semantic projection inside
  `a11y_mirror.mjs`, which never receives real text entry.
- **NOT non-Latin / IME-composition scripts (Arabic, CJK, Thai, Indic) — deferred to v2.** This is
  named, not half-built: `EditCmd::Insert` carries only **committed grapheme clusters**; there is
  **no preedit/composition state** in the P57 model at all. Latin+Cyrillic need none (X2). The v2
  project (a hidden composition host, decided against §16.34 only by the operator, **or** a
  bespoke in-canvas candidate UI — R1 §9 risk #1) is a bounded future blueprint, not a Wave-0
  unknown, and not touched here.
- **NOT RTL / bidi UI** — deferred to v2 alongside IME (§16.58). cosmic-text *computes* bidi runs,
  but P57 renders and cursors **LTR Latin+Cyrillic only**; a bidi caret-affinity model is v2.
- **NOT the mobile-web soft-keyboard mechanism.** Raising a soft keyboard with no editable element
  has no standard mechanism (R1 §3, X2) — this is **P63's named spike**, not P57's. Installed Tauri
  clients call native `show_soft_input` (P63/P39-rev own that binding); web-mobile falls back to
  the spike's outcome, with voice (§16.31, P64) as the honest interim on that one platform combo
  if the spike comes back empty. P57 exposes `TextField::focus()`; **what raises a keyboard is the
  shell's** (§12).
- **NOT the a11y-mirror convention or its Playwright harness** — **P58 owns them** (SYNTHESIS
  single-owner contract). P57's a11y half *cites* P58's synthetic-ARIA-textbox convention and
  imports its harness; where P58 does not yet exist, P57 defines a reasonable approach (§4.6) and
  carries an explicit **reconcile-with-P58** flag.
- **NOT the `InputSource`/`Intent` grammar** — **P64 owns it** (SYNTHESIS single-owner contract).
  P57 *contributes* one variant, `Intent::Text(EditCmd)`, and the focus-routing rule (§3); it does
  not fork the enum. This keeps the P38 §12.2 AR/VR constraint 3 intact (all input via
  `InputSource` → `Intent`, one path).
- **NOT a money-entry field.** Money is a **decided integer** presented via `TweenGuard`
  (`money_guard.rs`, 🔴 RED-LINE) — never typed into a tweened/interpolated caret. A P57 field is a
  `&str`; a user-entered numeric amount (e.g. a tip) is parsed to `i64` minor units **at the
  consumer's submit boundary** and presented via `TweenGuard::present_money`, never rendered
  through the caret animation (§5.1). P57 creates no editable `Money` surface.
- **NOT draft persistence / autofill** — **P66 owns** the on-device wallet + offline draft
  (query-before-replay). P57 exposes `value()`/`set_value()` so P66 can snapshot/restore a field;
  P57 stores nothing across sessions.
- **NOT deployment / shell selection** — P63/P39-rev decide winit-vs-Tauri and provide the native
  `ClipboardPort` + soft-keyboard impls; P57 defines the ports and consumes the verdict.

### 2.3 Two-lane build reality (mirrors P38's O18a split)

`cosmic-text` is absent from every Cargo cache (offline-clean mandate, §0) — adding it is the
**same one network grant as wgpu** (O18a, P38 §2 manifest). So P57 is two-laned exactly like P38:

- **Lane A (buildable TODAY, zero network):** M1 (Latin+Cyrillic classifier — pure), the
  `EditCmd`/`EditEvent`/`EditReject` types + the key→cmd mapping table (pure, shared by native +
  web sources), the `ClipboardPort`/`InputSource` traits, caret-blink phase logic (pure), the
  ARIA-textbox mirror spec + Playwright scaffolding (DOM-side), and every RED test — cosmic-text-
  dependent ones marked `#[ignore = "O18a"]` (the same convention P38 uses).
- **Lane B (blocked on O18a — do NOT `cargo add cosmic-text` without the operator's network
  grant; check `BLUEPRINT-W21-field-ui-gpu-blocked.md` first):** the cosmic-text `Editor`
  integration inside `TextField`, hit-testing (pointer→byte-cursor), glyph-run → FE-06 render,
  wasm `text_*_js` exports, and AccessKit text nodes.

---

## 3. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── engine/Cargo.toml — NEW feature (O18a manifest; default stays offline-clean) ──
// [features]
//   text = ["dep:cosmic-text"]     # part of the SAME one network grant as `gpu` (P38 §2)
// [dependencies]
//   cosmic-text = { version = "*", optional = true }   # added ONLY under `cargo add` at O18a
// Guard test (mirrors bridge.rs:214 e21): the DEFAULT build links NO cosmic-text (§4, §6 DoD-10).

// ── engine/src/text_scope.rs — NEW module, PURE, no cosmic-text (Lane A, lands NOW) ──
/// Wave-0 script scope — Latin + Cyrillic ONLY. Non-Latin/IME scripts are v2 (§2 anti-scope).
/// A hard classifier, not a config knob: the v2 boundary is a type-level refusal, not policy.
pub const WAVE0_SCRIPTS: &str = "Latin+Cyrillic";
/// True iff `c` is in Wave-0 scope: Basic Latin + Latin-1 Supplement + Latin Extended-A/B,
/// Cyrillic + Cyrillic Supplement, plus the always-allowed structural set (space, common
/// punctuation, digits). Everything else (CJK, Arabic, Thai, Indic, emoji, …) is OUT (v2).
pub fn in_wave0_scope(c: char) -> bool;
/// Grapheme-cluster boundary check deferred to cosmic-text at edit time; this module classifies
/// codepoints only (the scope gate), never re-implements segmentation (§1.2 ad-fontes line).

// ── engine/src/text_input.rs — NEW module, #[cfg(feature = "text")] (Lane B, O18a) ──
/// A UTF-8 BYTE offset into the buffer (cosmic-text `Cursor` maps to this). NEVER a char index —
/// byte offsets are the only stable cross-run index for grapheme-aware motion.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ByteCursor(pub usize);
/// An ordered selection: [anchor, focus] in byte offsets. `focus` is the live caret; it may be
/// < or > anchor. Collapsed (anchor == focus) ⇒ no selection, just a caret.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Selection { pub anchor: ByteCursor, pub focus: ByteCursor }

/// The CLOSED editing alphabet — every keydown / pointer / clipboard action normalizes to ONE
/// of these (standard item 3: tests assert on sequences of these, not end-state). There is NO
/// preedit/composition variant: Latin+Cyrillic commit directly (X2). `select` = extend-selection.
#[derive(Clone, PartialEq, Debug)]
pub enum EditCmd {
    Insert(String),                                   // committed grapheme cluster(s) only
    Backspace, Delete,
    MoveLeft { select: bool },  MoveRight { select: bool },
    MoveWordLeft { select: bool }, MoveWordRight { select: bool },
    MoveHome { select: bool },  MoveEnd { select: bool },
    SelectAll,
    Cut, Copy, Paste(String),                         // Paste payload arrives pre-read via ClipboardPort
    PointerDown(FieldPos), PointerDrag(FieldPos), PointerUp,   // mouse-drag selection (FieldPos 3D, P38 §12.2-2)
    Focus, Blur,
    Submit,                                            // Enter/commit intent — the CONSUMER decides meaning
}

/// What changed after `apply` — the event the a11y mirror AND the render both consume (one source).
#[derive(Clone, PartialEq, Debug)]
pub enum EditEvent {
    TextChanged { value_bytes: usize },
    CaretMoved { caret: ByteCursor },
    SelectionChanged { sel: Selection },
    Submitted,                                         // Submit surfaced (field-level; consumer acts)
    FocusChanged { focused: bool },
    Rejected(EditReject),
}
/// Typed refusal — NEVER a silent drop (self-termination leg, §5.4).
#[derive(Clone, PartialEq, Debug)]
pub enum EditReject { OutOfScope(char), ReadOnly, NothingToDelete, NoSelection }

/// Caret geometry in FIELD space (u,v,w — w carried, never truncated; P38 §12.2-2).
#[derive(Clone, Copy, Debug)] pub struct Rect { pub u: f32, pub v: f32, pub w: f32, pub h_w: f32, pub h_h: f32 }
/// One shaped glyph, already mapped to an FE-06 atlas slot — the ONLY bridge to the render.
#[derive(Clone, Copy, Debug)] pub struct ShapedGlyph { pub atlas_id: u32, pub pos: FieldPos, pub adv: f32, pub size: f32 }

/// One editable text field. Wraps a cosmic-text `Editor`; the engine owns the event surface.
pub struct TextField { /* cosmic_text::Editor + field_id: WidgetId + focused: bool
                          + blink: Spring (alpha) + read_only: bool + scope-gate */ }
impl TextField {
    pub fn new(field_id: WidgetId) -> Self;
    pub fn read_only(field_id: WidgetId) -> Self;
    /// Apply one command; returns the ordered events it produced (possibly empty). The ONLY
    /// mutator of buffer/caret/selection state.
    pub fn apply(&mut self, cmd: EditCmd, clip: &mut dyn ClipboardPort) -> Vec<EditEvent>;
    pub fn value(&self) -> &str;                       // for P66 snapshot / consumer submit
    pub fn set_value(&mut self, s: &str);              // for P66 restore / prefill (scope-gated)
    pub fn selection(&self) -> Selection;
    pub fn caret_rect(&self) -> Rect;                  // cosmic-text hit-test inverse (Lane B)
    pub fn selection_rects(&self) -> Vec<Rect>;        // one rect per visual run of the selection
    pub fn glyph_runs(&self) -> &[ShapedGlyph];        // → FE-06 MSDF glyph-quad pipeline
    /// Advance the caret blink/fade by dt; returns true iff ONLY the caret alpha changed (so the
    /// field integrator is NOT woken — §5.1 battery invariant). Blink is a localized alpha toggle.
    pub fn tick_caret(&mut self, dt: f32) -> bool;
}

// ── ports (P57 defines; SHELL provides native impl, WEB provides async-clipboard impl) ──
/// Clipboard is a PORT, not a hard dep (R3's "a port retrofitted is not a port"): native via the
/// shell (P39-rev/P63 — winit has none of its own), web via `navigator.clipboard` (async, needs a
/// user gesture, works with NO DOM widget — R1 §3).
pub trait ClipboardPort { fn read(&mut self) -> Option<String>; fn write(&mut self, s: &str); }
/// Keyboard input is an InputSource (P64-owned trait). P57 contributes ONE Intent variant.
/// P64's enum gains:  Intent::Text(EditCmd)     // cited, NOT forked (§2.2)
/// Focus-routing rule (P57-owned): while a TextField holds focus, keyboard Intents route to it as
/// EditCmds; nav Intents are suppressed for that field. ONE input path (InputRouter::tick → apply).

pub const CARET_BLINK_HZ: f32 = 1.0;            // 1 Hz on/off — localized alpha, NOT a field wake
pub const CARET_BAR_HALF_W: f32 = 0.5;          // caret bar half-width in field units (sdf_line_segment)
pub const FIELD_MAX_BYTES: usize = 4096;        // Wave-0 single-line field cap; §5.2 scaling axis
pub const WORD_BOUNDARY: &str = "unicode-word";  // cosmic-text word motion authority (never re-derived)

// ── wasm/src/lib.rs — exports per P38 G7 ptr/len conventions (Lane B) ──
// text_new_js(field_id) -> handle · text_apply_js(handle, cmd_json) -> events_json
// text_value_js(handle) -> String · text_caret_js(handle) -> "[u,v,w,h]"
// text_glyph_runs_ptr(handle) -> *const f32 · text_glyph_runs_len(handle) -> usize
//   (frame-scoped view; re-derive EVERY frame — the detached-buffer rule is P38 §3.7's, cited)
```

Rejected alternatives (DECART one-liners): **char-index cursor** — rejected: byte offsets are the
only run-stable index for grapheme-aware motion; cosmic-text's `Cursor` is byte-based (§1.2).
**a bespoke text renderer** — rejected: `ShapedGlyph` feeds FE-06's ONE glyph-quad pipeline; a
second renderer is a P38 §12.3 scope violation. **arboard as a hard native clipboard dep** —
rejected: clipboard is a `ClipboardPort` the shell fills (the shell is P63's undecided choice; a
hard-coded clipboard crate would pre-empt it and violate the port discipline). **a preedit field
in `EditCmd`** — rejected: Latin+Cyrillic need no composition (X2); a preedit variant would be
dead code that mislabels P57 as IME-capable.

---

## 4. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

### 4.1 M1 — Latin+Cyrillic classifier (pure; Lane A, lands NOW; the v2 boundary as a test)

`text_scope.rs::in_wave0_scope(c)` per §3 — the scope gate, decided by Unicode block membership,
with **zero** cosmic-text dependency (it classifies codepoints, not shaped runs). This is the
mechanism that makes "non-Latin deferred to v2" a **tested refusal**, not a prose promise.
RED→GREEN: `scope_accepts_latin_cyrillic` — `'a'`, `'Z'`, `'ё'`, `'Я'`, `'ї'`, `'—'`, `'0'`,
`' '` all in-scope; `scope_rejects_v2_scripts` — `'中'` (U+4E2D), `'あ'` (U+3042), `'ا'`
(U+0627), `'ก'` (U+0E01), `'😀'` (U+1F600) all OUT. **Adversarial (designed to break the
boundary):** the "Latin homoglyph" trap — Cyrillic `'а'` (U+0430) is in-scope, Latin `'a'`
(U+0061) is in-scope, but Greek `'α'` (U+03B1) is **OUT** (Greek is v2) — asserted explicitly so
the boundary is drawn at the *block*, not the glyph shape; combining diacritics (U+0300..U+036F)
classified as in-scope **only** when Latin-composable (precomposed forms preferred — see §5.1
dead-key note); zero-width/control codepoints (U+200B, U+0000) rejected as OutOfScope, never
inserted.

### 4.2 M2 — `TextField` editor over cosmic-text (Lane B, O18a; the event alphabet)

`TextField::apply(cmd, clip)` per §3 drives a cosmic-text `Editor`: `Insert` runs each grapheme
through `in_wave0_scope` first (out-of-scope ⇒ `Rejected(OutOfScope(c))`, buffer untouched);
`Backspace`/`Delete` at an empty boundary ⇒ `Rejected(NothingToDelete)`; `Move*{select}` maps to
cosmic-text cursor motion (`select:true` keeps the anchor, extends focus); `SelectAll` selects the
whole buffer; `Cut`/`Copy` call `clip.write(selection_text)` (Copy leaves buffer, Cut deletes +
emits `TextChanged`); `Paste(s)` inserts `clip.read()`'s payload **through the scope gate** (a
pasted `中` is refused per-grapheme, the Latin remainder inserts); `PointerDown/Drag/Up` set the
caret / drag-select via hit-test (§4.5); `Submit` emits `Submitted` (the consumer decides). Every
mutation returns the ordered `EditEvent` sequence.

RED→GREEN (event-sequence form, standard item 3): `type_hello` — `Insert("h")..Insert("o")`
yields `[TextChanged, CaretMoved]×5` and `value() == "hello"`; `type_privet_cyrillic` —
`"привіт"` round-trips byte-for-byte (Cyrillic is first-class, not a fallback); `backspace_mid`
— caret in the middle, `Backspace` removes the left grapheme, `CaretMoved` to the new byte
offset; `select_all_then_copy` — `SelectAll` then `Copy` writes the whole value to the clip port.
**Adversarial (designed to break):** paste a mixed `"cafe中文"` string ⇒ Latin `"cafe"` inserts,
`中`/`文` each surface `Rejected(OutOfScope)` (the v2 boundary holds under paste, the sharpest
injection path); insert past `FIELD_MAX_BYTES` ⇒ refused at the cap, caret unchanged (no partial
grapheme); a multi-byte grapheme (`"й"` = U+0439, or a Latin `"é"` precomposed) is deleted as ONE
unit by `Backspace`, never split into an invalid UTF-8 boundary (cosmic-text grapheme motion is
the authority — the test that fails if someone byte-decrements the cursor by hand).

### 4.3 M3 — keydown wiring, native (winit + AccessKit; Lane B for AccessKit, shell-gated)

A `KeyboardSource: InputSource` (P64 trait) consumes winit `WindowEvent::KeyboardInput`
(`KeyEvent` logical key + `ModifiersState`) **and** `WindowEvent::Ime(Ime::Commit(s))` (dead-key
**Latin** commits — a committed grapheme, NOT candidate-UI composition; this is why native handles
French/Portuguese dead keys that web cannot — §5.1). The key→`EditCmd` mapping is a **pure table**
(`key_to_cmd(key, mods) -> Option<EditCmd>`) shared with the web source (M4) so there is one
authority: `ArrowLeft`+Shift ⇒ `MoveLeft{select:true}`, `Ctrl/Cmd`+`ArrowLeft` ⇒ `MoveWordLeft`,
`Ctrl/Cmd`+`C/X/V` ⇒ `Copy/Cut/Paste`, `Ctrl/Cmd`+`A` ⇒ `SelectAll`, `Home/End` ⇒
`MoveHome/MoveEnd`, `Enter` ⇒ `Submit`, printable ⇒ `Insert`. Native clipboard = the shell's
`ClipboardPort` impl (P39-rev/P63). RED→GREEN: `key_table_maps_editing_keys` (pure, **Lane A** —
no winit needed: assert the table for the full modifier matrix); `ime_commit_inserts_deadkey`
(`Ime::Commit("é")` ⇒ `Insert("é")`, in-scope). **Adversarial:** an `Ime::Preedit` event (which
would arrive for a *non-Latin* IME) is **dropped with a counted warning**, never buffered — the
model has no preedit state (§2.2), and this asserts the v2 boundary at the native event edge;
a key with no printable value and no mapping (a bare `Alt`) ⇒ `None`, no event.

### 4.4 M4 — keydown wiring, web (focusable canvas + wasm; Lane B for wasm)

The `<canvas>` gets `tabindex="0"` (focusable, receives keyboard) — **not** an editable element.
A `web/src/lib/text/keyboard_source.mjs` binds `keydown` (reads `event.key` — for Latin+Cyrillic
the composed character arrives directly, **no IME needed**, X2 — plus `ctrlKey/metaKey/shiftKey`),
and `copy`/`cut`/`paste` clipboard events + `navigator.clipboard.readText()` behind the same
`ClipboardPort` shape (async, user-gesture-gated, works with **no DOM widget** — R1 §3). Each
event → the **same** key→cmd table (M3) → `text_apply_js(handle, cmd_json)` → `EditEvent` JSON →
update the a11y mirror (M6) + trigger a redraw. New wasm exports per §3 (P38 G7 ptr/len). RED→GREEN
(Playwright, headless): `web_type_latin_cyrillic` — dispatch `keydown` for `"привіт hello"`,
assert `text_value_js` equals it and the a11y mirror value tracks; `web_shift_arrow_selects` —
`keydown ArrowRight` with `shiftKey` extends the selection; `web_clipboard_roundtrip` — programmatic
copy then paste re-inserts. **Adversarial (the ruling's teeth):** dispatch a `keydown` whose
`event.key` is `"中"` (as a hostile synthetic layout) ⇒ `Rejected(OutOfScope)`, buffer unchanged;
assert **no `compositionstart`/`compositionupdate` listener exists** on the canvas (grep + runtime
`getEventListeners`-equivalent check) — the web path deliberately does not host composition (that
is the v2 seam, §2.2); assert the canvas is **not** `contenteditable` and there is **no `<input>`**
in the tree (the P38 §12.1 tightened grep gate, §6).

### 4.5 M5 — render leg (FE-06 glyph quads + caret/selection SDF; blink without a field wake)

Render is three layers into the **existing** frame, zero new render math: (a) **glyphs** —
cosmic-text shaped `LayoutGlyph`s → `ShapedGlyph` (atlas-mapped) → FE-06's instanced MSDF glyph-
quad path (P38 §3.3, the SAME RectInstance family — no bespoke text renderer); (b) **selection** —
one `SdfShape::RoundedBox` per `selection_rects()` run behind the glyphs (`sdf_rounded_box`,
`scene.rs:41`); (c) **caret** — a thin `SdfShape::LineSegment` bar (`sdf_line_segment`) at
`caret_rect()`, whose **alpha** rides a `Spring` (focus-fade in/out) and a 1 Hz blink phase. The
load-bearing invariant: **a blinking caret must NOT keep the field alive** — `tick_caret` toggles
only the caret quad's alpha and returns without setting `FrameSignals::animation_active` on the
field integrator, so FE-14's settle gate (P38 §3.5) stays valid and a focused-idle field draws ~0
field steps (battery — §5.1). RED→GREEN (Lane B): `caret_at_buffer_end` — after typing, the caret
rect sits at the last glyph's advance (hit-test inverse); `selection_rects_cover_range` — a 3-char
selection yields a rect spanning exactly those glyphs' bounds. **Lane A** (pure, now):
`blink_does_not_wake_field` — drive `tick_caret` for 3 s of blink cycles against a settled field,
assert the field integrator step count stays 0 (the caret alpha animates, the field does not) —
this is the test that fails if someone implements the caret as a full-field redraw.
**Adversarial:** an empty field shows a caret at offset 0 (not absent); a caret past a wrapped
line (Wave-0 single-line ⇒ no wrap, but the test pins that a value at `FIELD_MAX_BYTES` still has a
findable caret rect, not a panic).

### 4.6 M6 — a11y live-edit projection (AccessKit native; synthetic ARIA-textbox web, P58-reconciled)

**Native (production-ready, R1 §2):** build an AccessKit `Node` with role **`TextInput`**, push
the buffer as text-run content, and set the **`text_selection`** attribute (anchor+focus byte
positions map onto AccessKit's schema fields) on every `CaretMoved`/`SelectionChanged`/
`TextChanged`; handle the **incoming** `SetTextSelection` action (a screen reader moving the caret)
by translating it to `Move*`/selection `EditCmd`s — so caret motion is announced natively, both
directions. This is the strong path and needs **no** hand-rolled announcement logic.

**Web (forced synthetic, ruling §0.2-2 — the hidden-editable variant is OUT):** a hidden
`role="textbox"` node inside P58's `a11y_mirror.mjs` subtree (NOT `<input>`, NOT `contenteditable`
— it never receives entry; entry is canvas `keydown` → wasm), whose accessible **value** mirrors
the buffer and whose caret/selection is communicated via `aria-activedescendant` pointing at a
per-caret marker, updated on **every** `EditEvent`. This is honestly **weaker and more
screen-reader-dependent** than a native editable element (R1 §2, X1, X2) — recorded, not hidden;
the native clients are the strong-a11y path. **P58 owns the canonical convention**; P57 specifies
this reasonable approach and carries a hard **reconcile-with-P58** obligation (§12): if P58's final
convention differs (e.g. `aria-describedby` live-region caret announcements instead of
`aria-activedescendant`), P57's web M6 adopts it — one convention, one implementation (P38 §12.3).

**Draft-parity invariant (X1, inherited):** the a11y mirror reconciles from the **same**
`EditEvent` stream the renderer consumes — one source ⇒ mirror caret == rendered caret by
construction. RED→GREEN: native `accesskit_selection_tracks_buffer` (`#[ignore]` until the shell
wires AccessKit — P39-rev/P63); web `aria_textbox_caret_parity` — a Playwright a11y-tree snapshot
(P58's harness) asserts a `role=textbox` node whose value + active-descendant caret marker match
`text_value_js`/`text_caret_js` after a keydown sequence. **Adversarial:** type then select-all
then delete ⇒ the mirror value is empty and the caret marker is at offset 0 (no stale
announcement — the same stale-node class P38 §3.6 guards); force `navigator.gpu = undefined`
(WebGL2/CPU floor) ⇒ the a11y assertions still pass unchanged (a11y is renderer-independent by
construction — inherited from P38 §3.6).

### 4.7 M7 — wasm exports (P38 G7 ptr/len; retrieval-export separation honored)

`text_new_js/apply_js/value_js/caret_js` + glyph-run `ptr/len` per §3, following P38 G7's
frame-scoped-view + re-derive-every-frame rule exactly (the detached-buffer hazard is P38 §3.7's,
cited not re-proven). These are **cosmic-text-backed ⇒ Lane B (O18a)**. On wasm, cosmic-text loads
fonts from **committed bytes** (no `fontdb` system query on wasm32 — the same font asset FE-06's
MSDF atlas is generated from, one font authority). RED→GREEN: `wasm_text_apply_roundtrip` — a
JSON `EditCmd` in, `EditEvent` JSON out, `text_value_js` reflects it. **Adversarial:** the glyph-
run view is re-derived after a `text_apply_js` that grew wasm memory ⇒ a stale cached view is
detached (the P38 §3.7 rule is load-bearing here too, asserted).

---

## 5. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 5.1 Hazard-safety as math (item 6)

Reachability arguments, not prose:

- **A non-Latin/Cyrillic codepoint in the Wave-0 buffer is unrepresentable:** every insert path
  (`Insert`, `Paste`, `Ime::Commit`, `keydown`) passes through `in_wave0_scope` before the buffer
  mutates; an out-of-scope codepoint returns `Rejected(OutOfScope(c))` with the buffer **untouched**
  (M2/M4 adversarial tests make "a `中` reached the buffer" a tested-unreachable state). The v2
  boundary is a type-level refusal at one choke point, not scattered policy.
- **Money never tweens through a caret:** a P57 field's value is `&str`; `Money` implements no
  `FieldValue` (`money_guard.rs:22`), so `Spring<Money>` / `interpolate(money,…)` does not compile.
  A user-entered numeric amount is parsed to `i64` minor units at the **consumer's** submit boundary
  and presented via `TweenGuard::present_money` (`money_guard.rs:60`) — never rendered through the
  caret alpha/position animation (which are `FieldValue` channels). The 🔴 RED-LINE is inherited,
  not weakened.
- **A blinking caret cannot burn battery or hide field divergence:** `tick_caret` animates only the
  caret quad's alpha; it does not wake the field integrator (§4.5), so FE-14's settle gate + its
  Lyapunov watchdog (P38 §3.5) remain the sole authority on field liveness — "field diverged while
  the caret blinked" is unreachable while the energy law holds. The `blink_does_not_wake_field`
  test is the falsifier.
- **No DOM input exists:** the only DOM P57 creates is the a11y `role="textbox"` mirror node
  (created only in `a11y_mirror.mjs`); a bare `<canvas tabindex=0>` carries keyboard focus without
  being editable. "Text entry via an editable DOM element" is falsified by the tightened grep gate
  (§6) — no `<input>`, no `contenteditable`, `createElement` only in the mirror module.
- **Dead-key / AltGr honesty (the real edge, named not hidden):** most European Latin layouts emit
  the *precomposed* character directly in `event.key` (web) — those work. True **two-stroke dead-key
  composition** (accent-then-letter) uses `compositionend` even for Latin, which a bare web canvas
  cannot host — so Wave-0 **web** accepts precomposed input only; two-stroke dead-key composition on
  web is the **same class as IME and is deferred with it** (§2.2). **Native is unaffected**: winit's
  `Ime::Commit` delivers the final composed Latin grapheme (a commit, not candidate-UI), so native
  clients handle dead keys. This asymmetry is recorded (not a silent gap) and is why native is the
  stronger input path, mirroring the a11y asymmetry.

### 5.2 Schemas & scaling axes (item 8)

`TextField`: axis = **bytes/field**; Wave-0 fields (name, address, phone, note) are ≤ `FIELD_MAX_BYTES`
(4 KiB); break point = a **multi-line paragraph editor** (word-wrap + vertical caret motion), which
is a v2 unit, not Wave-0. **Concurrent focused fields = exactly 1** (single-focus invariant — a
second `Focus` blurs the first; no axis, an invariant). **Glyph atlas:** shares FE-06's atlas (one
2048² page ~4k glyphs, P38 §4.2); Latin+Cyrillic cover ~500 glyphs — comfortably one page, no growth
step in Wave-0. **a11y mirror:** O(1) node per focused field (single-focus) — no reconcile-cost axis.

### 5.3 Isolation (item 11), mesh awareness (item 12), living memory (item 15)

**Isolation:** a `TextField` is node-local with no shared mutable state; a text-field panic cannot
corrupt an order — the buffer is consumer-read **only at submit**, and the kernel `decide` validates
the submitted value (P57 is a consumer of no kernel state and a mutator of none). The render stack
inherits P38 §4.3's bulkhead. **Mesh:** text entry is **entirely node-local** — a submitted value
rides P37/P34's wire as an order-intent field (address/name), but that transport is the **consumer's**
(P69), not P57's; **zero mesh payload originates in the editor** (no gossip, no SyncFrame). **Living
memory:** draft persistence is **P66's** (query-before-replay, LWW draft — R4 §3); P57 exposes
`value()`/`set_value()` so P66 can snapshot/restore a field, but P57 stores nothing across sessions —
the temporal/topological access pattern is P66's to own (X6), cited not duplicated.

### 5.4 Rollback / self-healing vocabulary (item 13, used precisely)

- **Self-Termination leg claimed:** typed `EditReject` refusals (OutOfScope / ReadOnly /
  NothingToDelete / NoSelection); the Latin+Cyrillic classifier as an unrepresentable-state
  boundary; the single-focus invariant; the un-compilable money-tween (inherited). These are hard
  invariant boundaries, not a supervisor's decision.
- **Self-Healing: NOT claimed.** Text entry has no error-correcting redundancy — a mistyped
  character is corrected by the user, not by the system; claiming self-healing here would be loose
  use of the word (the standard forbids it).
- **Snapshot-Re-entry: NOT claimed by P57.** The buffer is authoritative state, not derived; its
  persistence/restore across a reconnect is **P66's** snapshot (query-before-replay), which P57
  merely exposes a hook for. Mechanical rollback: the `text` feature OFF restores today's exact
  build (no text editing) — mirrors P38's `gpu` feature; every symbol is module- or feature-additive.

### 5.5 Linux discipline (item 9) + tensor/spectral/eqc (item 16)

Verdicts per the adoption framework: **ALREADY-EQUIVALENT** — one glyph pipeline (FE-06 renders
both static labels and editable text; `ShapedGlyph` reuses the RectInstance family — no bespoke
text renderer); one key→cmd authority (the pure table shared by native + web sources). **REINFORCES**
— feature-gated cosmic-text with an offline-clean **default** + a guard test that the default build
links no cosmic-text (the kernel-module hardware-behind-a-flag discipline, mirroring `gpu`).
**EXTENDS** — the Latin+Cyrillic-only type boundary as a **new refusal class** for a text surface (a
discipline this repo adds: a script-scope gate at the buffer edge). **GAP** honestly named — **no
browser CI runner exists** for the canvas-`keydown` path; the web M4/M6 Playwright specs run
headless (accessibility-tree + programmatic key dispatch), but real on-device keyboard behaviour
(soft keyboard, dead keys, layout quirks) is unverified until P45/P63 provides a display runner —
each web `#[ignore = "O18a"]`/headless marker doubles as that GAP marker.
**Item 16 (tensor/spectral/eqc): NOT load-bearing, stated not decoratively invoked** — text shaping
is cosmic-text's HarfRust (a first-principles Unicode primitive, *ad fontes*-kept — §1.2); there is
no closed-form math organ here, so `eqc-rs` does not apply and no spectral machinery is summoned
(the Anu/Ananke discipline forbids ritual math — P51 §5.5 precedent).

---

## 6. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| M1 | no classifier; scope tests absent | `scope_accepts_latin_cyrillic`; `scope_rejects_v2_scripts`; Greek-homoglyph + combining-mark + control-codepoint adversarial | **scope-boundary test** (ledger row) |
| M2 | no `TextField`; event-sequence tests RED | type/backspace/select/copy sequences exact; mixed-script paste rejects per-grapheme; grapheme-atomic backspace | **Latin+Cyrillic-only + grapheme-atomic tests** (ledger row) |
| M3 | no `KeyboardSource`; key table absent | `key_table_maps_editing_keys` (pure); `ime_commit_inserts_deadkey`; `Ime::Preedit` dropped-not-buffered | preedit-drop test |
| M4 | no web keyboard source; wasm text exports absent (§0) | web type Latin+Cyrillic; shift-arrow selects; clipboard round-trip; **hostile `中` keydown rejected**; no composition listener; no `<input>`/`contenteditable` | **no-DOM-input grep gate** (ledger row) |
| M5 | no render leg; caret/blink tests RED | caret at buffer end; selection rects cover range; **`blink_does_not_wake_field`** (0 field steps) | **caret-blink-no-field-wake test** (ledger row) |
| M6 | no a11y projection | native AccessKit selection tracks buffer (`#[ignore]` until shell); web `aria_textbox_caret_parity`; stale-announcement + renderer-independent adversarial | **a11y caret-parity test** (ledger row) |
| M7 | zero wasm text exports (§0) | `wasm_text_apply_roundtrip`; glyph-run view re-derive-after-grow (detached-buffer) | detached-view test (P38 §3.7 shared) |
| default build | — | **`default_build_has_no_cosmic_text`** (mirrors `bridge.rs:214` e21): offline-clean default links no cosmic-text | offline-clean guard test |

**Not-done clauses:** any DOM `<input>`/`contenteditable`/editable element for text entry on any
platform = **NOT done** regardless of green totals (P38 §12.1 inherited); a `compositionstart`/
`update` listener hosting composition = NOT done (that is the v2 seam, not Wave-0); a non-Latin/
Cyrillic codepoint reaching the buffer = NOT done; a money value rendered through the caret
animation = NOT done (🔴 RED-LINE); a blinking caret that wakes the field integrator = NOT done
(battery); an `#[ignore = "O18a"]` test silently deleted instead of un-ignored at unlock = NOT done
(P38 convention); a second glyph renderer or a second a11y-mirror implementation = NOT done (P38
§12.3).

---

## 7. Benchmark plan (item 10) — the keystroke-latency bar, measured against GPUI

Budgets (mid-tier device class, P38 §6's 16.6 ms/frame split): **keystroke → `apply` → `EditEvent`
≤ 2 ms CPU** for a ≤256-char field (the GPUI 2 ms keystroke-to-pixel class, R1 §1 — the pixel leg
then rides FE-06's ≤ 4 ms GPU text budget, P38 §6); **shaping reflow after an insert ≤ 1 ms**
(cosmic-text incremental shape of one line); **caret blink CPU cost ≈ 0** (alpha toggle, no field
step — the `blink_does_not_wake_field` counter is the tripwire). Criterion benches
`engine/benches/text.rs`: `text_apply_insert_256`, `text_shape_reflow_line`, `text_selectall_copy`
— added **RED-commit-first** so the `bench_track` baseline auto-seeds (P-A §6 / P51 §7 discipline,
same `BENCH_HISTORY.md` append rule). Telemetry: keystroke-to-event latency + reject-count ride the
existing native-trackers hooks (P-H's lane) so an input-latency or reject-rate regression surfaces
without review. The settle target — **0 field-integrator steps on a focused-idle blinking field** —
is asserted by `blink_does_not_wake_field` over a counted 3 s window, which doubles as the CI
tripwire (the benchmark IS a test, not a report — P38 §6 pattern).

---

## 8. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §0.2-2 (the ruling), §2 X1/X2 (a11y-mirror + IME
resolutions), §5 W1 table (P57 scope + deps) · `docs/research/OPUS-R1-INTERFACE-RENDERING-2026-07-18.md`
§3 (cosmic-text/parley), §2 (AccessKit / a11y-mirror), §9 risk #1/#4 (the deferred v2 problems) ·
`BLUEPRINT-P38-webgpu-render-engine.md` §3.3 (FE-06 MSDF glyph pipeline P57 renders through), §3.5
(FE-14 settle gate the caret must not wake), §3.6/§12.3 (FE-15 mirror base + FE-16 ladder P57
imports), §11.2 (`FieldPos`/`Intent`/`InputSource`), §12.1 (the struck overlay P57 replaces),
§12.2 (AR/VR constraints P57 honors) · `BLUEPRINT-P51-open-map-routing.md` (rigor/format precedent) ·
`docs/design/BLUEPRINT-W21-field-ui-gpu-blocked.md` (O18a gate, shared) ·
`docs/regressions/REGRESSION-LEDGER.md` (five rows named in §6) · `HERMETIC-ARCHITECTURE-PRINCIPLES.md`
(§9). **Sibling W1 blueprints (contracts P57 consumes, cited not forked):** **P58** (a11y-mirror-
everywhere + the synthetic-ARIA-textbox convention + the shared Playwright a11y-tree harness — P57's
web M6 reconciles with it), **P64** (`InputSource`/`Intent` grammar owner — P57 contributes
`Intent::Text(EditCmd)`), **P63** (shell + soft-keyboard + native-clipboard spike), **P39-rev**
(shell decision → native `ClipboardPort` impl), **P66** (draft persistence hook), **P69/P70/P71/P73**
(the surfaces whose every typed field is a `TextField`). Memory: `physics-ui-capture-quantum-math-arc-2026-07-14`
(ONE Laplacian; the settle gate is the liveness authority — honored by the caret-no-wake invariant) ·
`field-ui-engine-arc-2026-07-13` (FE-06/FE-14/FE-15 substrate) · `rust-native-bare-metal-decision-2026-07-14`
(DECART tables §1/§3; cosmic-text = modern-default primitive, older = adapters) ·
`anu-ananke-strict-discipline-feedback-2026-07-17` (style; §5.5's refusal to invoke spectral math
decoratively; the honest dead-key/a11y asymmetries stated not hidden) · `test-integrity-rules-2026-06-27`
(money 🔴 RED-LINE, §5.1) · `verified-by-math-2026-07-07`. Supersedes: nothing — it **replaces** P38
G6's struck input overlay (P38 §12.1 already recorded the strike; P57 is the named home).

---

## 9. Hermetic principles honored (item 20 — load-bearing only)

- **P1 MENTALISM** (spec is source): the `EditCmd`/`EditEvent` alphabet + the Latin+Cyrillic scope
  gate (§3) precede every line of implementation; cosmic-text is bound to a spec surface, never
  free-handed into the render path.
- **P2 CORRESPONDENCE** (one concept, one primitive): one glyph pipeline (FE-06 for labels AND
  editable text); one key→cmd table (native + web); one `EditEvent` stream feeding both render and
  a11y (draft-parity by construction, X1); one clipboard port; one focus at a time.
- **P6 CAUSE-AND-EFFECT** (determinism as law): event-sequence tests (not end-state); a hostile
  mixed-script paste has an exact, tested per-grapheme outcome; the scope classifier is total over
  `char` — every refusal carries a falsifier (§4, §6).
- **P7 GENDER** (paired verification, no self-certification): the editor is refereed by cosmic-text's
  independent grapheme motion (a hand-decremented cursor fails the atomic-backspace test); the render
  is refereed by FE-06's glyph pipeline + the compose oracle; the a11y projection is refereed by the
  accessibility tree (Playwright / AccessKit), never by its own reconcile code.

(P3/P4/P5 not load-bearing here; not claimed decoratively.)

---

## 10. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites; the zero-text-input finding; the wasm/web gap) |
| 2 DoD | §6 |
| 3 spec/event-driven TDD | §3 spec-first; §4 RED-first; §4.2/§4.4 event-sequence assertions |
| 4 predefined types/consts | §3 |
| 5 adversarial/breaking tests | §4.1–§4.7 (Greek-homoglyph, mixed-script paste, grapheme-atomic backspace, hostile `中` keydown, `Ime::Preedit` drop, blink-no-wake, stale a11y node, detached view) |
| 6 hazard-safety as math | §5.1 (unrepresentable out-of-scope char, un-compilable money tween, no-DOM-input, blink-no-wake) |
| 7 links docs/memory | §8 |
| 8 scaling axes | §5.2 (each with a named break point) |
| 9 Linux discipline | §5.5 (all four verdict classes incl. an honest GAP) |
| 10 benchmarks+telemetry | §7 (GPUI-class budget, bench_track seeding, blink-counter tripwire) |
| 11 isolation/bulkhead | §5.3 (node-local; consumer-read at submit only) |
| 12 mesh awareness | §5.3 (zero payload originates in the editor) |
| 13 rollback/self-heal vocabulary | §5.4 (self-termination claimed; self-healing + snapshot refused precisely) |
| 14 error-propagation gates | §6 (named ledger rows), §5.1 (typed refusal classes) |
| 15 living memory | §5.3 (persistence is P66's; P57 exposes the hook, owns no storage) |
| 16 tensor/spectral + eqc reuse | §5.5 (honestly NOT invoked — shaping is an ad-fontes Unicode primitive) |
| 17 regression ledger | §6 (five rows named) |
| 18 agent-executable instructions | §11 |
| 19 reuse-first | §0/§1.2 (cosmic-text/FE-06/Scene/SDF/Spring/money_guard all reused; four rejected alternatives §3; the crate-vs-handroll comparison §1.2) |
| 20 Hermetic citations | §9 |

---

## 11. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Two lanes (§2.3). **Lane A (buildable TODAY, no network):** T1–T3, T6-web-scaffold. **Lane B
(blocked on O18a — do NOT `cargo add cosmic-text` without the operator's network grant; check
`docs/design/BLUEPRINT-W21-field-ui-gpu-blocked.md` first):** T4, T5, T7, and the cosmic-text half
of T6.

1. **T1 (M1 — the v2 boundary first; pure, Lane A).** Create `engine/src/text_scope.rs` per §3
   (`in_wave0_scope`, `WAVE0_SCRIPTS`), register `mod text_scope;` in `engine/src/lib.rs`. Write RED
   first: `scope_accepts_latin_cyrillic`, `scope_rejects_v2_scripts`, and the Greek-homoglyph /
   combining-mark / control-codepoint adversarial cases (§4.1). Acceptance:
   `cargo test -p dowiz-engine text_scope` green; ledger row added.
2. **T2 (M3/M4 key table — pure, Lane A).** Add the `key_to_cmd(key, mods) -> Option<EditCmd>` table
   (§3, §4.3) as a pure fn (in `text_input.rs` under a small non-feature-gated sub-module, or its own
   `keymap.rs`). RED: `key_table_maps_editing_keys` over the full modifier matrix. Acceptance: green
   (no winit/cosmic-text needed).
3. **T3 (M5 blink logic — pure, Lane A).** Implement the caret blink/fade phase (a `Spring`-driven
   alpha + 1 Hz toggle) as a pure function whose contract is "toggles caret alpha, never a field
   step". RED: `blink_does_not_wake_field` (drive 3 s against a settled field, assert 0 integrator
   steps). Acceptance: green; ledger row added; bench `text.rs` seeded RED-commit-first (§7).
4. **T4 (M2 — FIRST Lane B task after O18a).** `cargo add cosmic-text` under `feature = "text"`
   (the §3 manifest is the complete authorized addition; it is the SAME O18a grant as wgpu — do not
   add anything else). Implement `TextField`/`EditCmd`/`EditEvent` in `engine/src/text_input.rs`
   (`#[cfg(feature = "text")]`); route every insert through T1's classifier. Un-ignore the M2
   event-sequence + mixed-script-paste + grapheme-atomic tests. Keep the default build offline-clean:
   `default_build_has_no_cosmic_text` guard (mirror `bridge.rs:214`) must stay green. Acceptance:
   `cargo test -p dowiz-engine --features text text_input` green.
5. **T5 (M5 render + M7 wasm — Lane B).** Wire `glyph_runs()` → FE-06's MSDF glyph-quad pipeline
   (P38 §3.3 — reuse it, do NOT build a second renderer); caret/selection via `SdfShape` (§4.5). Add
   the wasm `text_*_js` exports (P38 G7 ptr/len + re-derive-every-frame). Acceptance: `caret_at_buffer_end`,
   `selection_rects_cover_range`, `wasm_text_apply_roundtrip` green.
6. **T6 (M4 web + M6 web a11y — scaffold Lane A, wire Lane B).** Create `web/src/lib/text/keyboard_source.mjs`
   (canvas `tabindex=0`, `keydown`/clipboard → key table → `text_apply_js`) and the ARIA-textbox
   projection **inside P58's `a11y_mirror.mjs`** (do NOT stand up a second mirror — P38 §12.3). Add
   `web/tests/text-input.spec.mjs` with the §4.4/§4.6 assertions (type Latin+Cyrillic, shift-arrow
   select, clipboard round-trip, hostile `中` reject, no-composition-listener, no-`<input>` grep,
   `aria_textbox_caret_parity`). **Import P58's Playwright a11y-tree harness — cite it, and leave a
   `RECONCILE-P58` TODO** wherever P57's ARIA convention is provisional (§4.6, §12). Acceptance:
   Playwright specs green headless.
7. **T7 (M3/M6 native — Lane B, shell-gated).** Implement `KeyboardSource: InputSource` over winit +
   `Ime::Commit`, and the AccessKit `TextInput` node + `SetTextSelection` handler (§4.3/§4.6). These
   land **with the shell** (P39-rev/P63) — mark `#[ignore]` keyed to the shell landing until then,
   never delete. Provide the native `ClipboardPort` impl through the shell's clipboard. Acceptance:
   native tests green once the shell exists; ledger rows present in `docs/regressions/REGRESSION-LEDGER.md`.

---

## 12. Dependencies & blocks (the wiring, stated once)

**Inputs P57 depends on (cited, never redefined):**

| Input | What P57 takes from it | Reconciliation obligation |
|---|---|---|
| **P38-rev** (`BLUEPRINT-P38-webgpu-render-engine.md` §12) | The strike of the DOM overlay (§12.1 — P57 IS the replacement); FE-06 MSDF glyph pipeline (§3.3 — P57 renders through it); FE-14 settle gate (§3.5 — the caret must not wake it); FE-15 mirror base + FE-16 ladder (§12.3 — imported, not re-derived); `FieldPos`/`Intent`/`InputSource` (§11.2); AR/VR constraints (§12.2 — honored: FieldPos 3D, input via InputSource) | P57 imports; if FE-06 lands with a different glyph-quad shape, T5 adapts to it |
| **P58** (a11y-mirror-everywhere) | The **synthetic-ARIA-textbox convention** and the shared **Playwright a11y-tree harness**; the draft-parity invariant (X1) | **HARD: P57's web M6 is provisional until P58 is final** — P57 defines a reasonable ARIA-textbox approach (§4.6) and carries `RECONCILE-P58` TODOs; on divergence, P57 adopts P58's convention (one convention, one implementation) |
| **P64** (`InputSource`/`Intent` grammar owner) | The `InputSource` trait + `Intent` enum + `WidgetId`; P57 **contributes** `Intent::Text(EditCmd)` + the focus-routing rule | P57 cites the variant, does not fork the enum (SYNTHESIS single-owner contract) |
| **P63 / P39-rev** (shell + spike) | The native `ClipboardPort` impl, the winit-vs-Tauri verdict, and the **mobile-web soft-keyboard mechanism** (P63 spike — NOT P57's to solve) | P57 defines the ports; the shell fills them; `TextField::focus()` is the seam, what raises a keyboard is the shell's |

**Consumers P57 feeds (every typed field across the product is a `TextField`):**

| Consumer | Fields it builds on P57 |
|---|---|
| **P69** (customer storefront & checkout) | name, address, phone, note fields in the checkout wizard (§16.34's no-interim-DOM-form is why P57 must exist before P69) |
| **P70** (owner surface) | menu-item names/descriptions, brand text fields, GDPR-tool inputs |
| **P71** (courier surface) | delivery notes / any in-app text entry (voice-primary in motion per §16.53, but text remains the fallback) |
| **P73** (dowiz.org landing + signup) | the interest form (§16.56 full-wgpu landing, no static-page exception) |

**Build ordering (SYNTHESIS §3):** P57 is a **W1** blueprint — its blueprint is written in parallel
with the rest of W1 (no file collision), and its Lane-A code (T1–T3, T6-scaffold) is buildable now;
its Lane-B code (T4/T5/T7 + cosmic-text) unlocks with O18a. P57 must be **built before any checkout
UI** (P69's address/name fields have no interim DOM form to borrow — SYNTHESIS §3.2 rationale 2).
The blocking edge is real: **P69/P70/P71/P73 cannot build a typed field until P57 exists.**
