# BLUEPRINT P58 — a11y-mirror-everywhere + shared accessibility-tree harness (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §9). Wave **W1**,
> foundation blueprint **P58** as scoped by `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5's W1
> table and its cross-cutting finding **X1** (the most detailed cross-cutting item in the
> synthesis — read in full this pass). Depends on canon-diff **P38-rev**
> (`BLUEPRINT-P38-webgpu-render-engine.md` §12) as its render/mirror base; format precedent:
> `BLUEPRINT-P51-open-map-routing.md` (whose §4.7 a11y-tree assertion for ONE surface — the map —
> is the harness this blueprint **generalizes into the shared gate**). Research substrate:
> `OPUS-R1-INTERFACE-RENDERING-2026-07-18.md` §2 (AccessKit: production-ready native,
> **web/canvas backend planning-only**).
>
> **This blueprint OWNS two shared contracts** (SYNTHESIS §5 swarm-dispatch, single-owner rule):
> (1) the **synthetic ARIA-textbox convention** for live text-editing state on web, and (2) the
> **shared Playwright accessibility-tree harness**. P57 (canvas text input, written in parallel)
> references the ARIA-textbox convention **provisionally** and MUST reconcile against §2/§4.3
> here; every surface blueprint (P69/P70/P71/P73, plus P57's a11y half and P51's map) imports the
> harness (§4.5) as a Definition-of-Done gate rather than re-deriving one.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads. The single most load-bearing finding:
**there is no semantic (role/name/state) layer anywhere in the engine today** — the renderer
consumes geometry + physics only, so the "same state feeds render and mirror" invariant X1 asks
for (§M1, §M6) has to be *created*, it cannot merely be cited.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| The hidden-DOM semantic mirror `web/src/lib/a11y_mirror.mjs` **does not exist yet** — it is a P38 **G6 build item**, not landed | `find web -iname '*a11y*'` → 0 files; `web/src/lib/` holds only `kernel/` + `fieldsim/` | **VERIFIED — P58 is the first blueprint that actually builds the mirror; P38 specs it, P58 generalizes + lands it** |
| No `web/tests/` directory exists; there is **no** Playwright a11y spec anywhere | `ls web/tests` → absent; repo grep `a11y-mirror.spec` → 0 | **VERIFIED — the shared harness (§M5) is greenfield infrastructure** |
| No `accesskit` crate anywhere in code or `Cargo.toml` deps | `grep -rl accesskit --include=*.rs --include=*.toml .` → 0 | **VERIFIED — native adapter (§M4) needs a crate grant (AK-unlock, §2), like P38's O18a** |
| The ONLY a11y annotation in the current web surface is a placeholder Svelte canvas label | `web/src/components/FieldSim.svelte:147-148` (`<canvas role="img" aria-label="…WebGPU">`) | **VERIFIED — ad-hoc single-node labeling; the mirror replaces it with a full semantic tree** |
| `web/src/app.mjs` — 204 lines, console-only, binds kernel `_js` exports; its header defers DOM/FieldSim to a "separate work unit" | `web/src/app.mjs` (`wc -l` = 204) | VERIFIED — the mirror + harness extend this surface, do NOT rewrite it (P38 §10 T4 charter) |
| Engine `Scene`/`SdfShape` carry **geometry only** — no `role`, `name`, `state`, or `WidgetId` field exists | `engine/src/scene.rs:29` (`SdfShape` enum — pure SDF variants), `:71` (`Scene`), `:88`/`:168` (`add`/`render_to_bridge`) | **VERIFIED — the semantic model (§M1) is genuinely new; the renderer today cannot answer "what widget is this"** |
| `WidgetStore` is SoA **physics** (positions/velocities), no semantic identity | `engine/src/widget_store.rs:14` (`WidgetStore`), `:54` (`integrate`), `:68` (`ParticlePool`) | VERIFIED — semantics ride *beside* physics, keyed by a new `WidgetId` |
| `compose(scene, eq, w, h, steps) -> Vec<u8>` — the bit-deterministic render oracle (P38 §0) | `engine/src/field_frame.rs:218` | VERIFIED — the render leg of "one state ⇒ frame AND mirror"; the mirror is its semantic sibling |
| Money renders integer→integer via `TweenGuard`, never interpolated; `Money(i64)` implements no `FieldValue` | `engine/src/money_guard.rs:18` (`Money`), `:54`/`:60` (`TweenGuard::present_money`), `:72` (`jump`) | VERIFIED — a Status/Alert node showing a price is TEXT from `present_money`, never a tweened value (§4.1) |
| FE-14 settle gate `engine/src/settle.rs` is **not landed yet** — a P38 **G5 build item** | `ls engine/src/settle.rs` → absent | VERIFIED — the mirror piggybacks it when it lands; §M2 states the seam + the pre-settle interim |
| `Intent`/`FieldPos`/`InputSource` — **0 grep hits in code** (P38b DoD-1 baseline) | repo-wide grep this pass | VERIFIED — keyboard/focus intents route through P38-rev's `InputSource` (§12.2 constraint 3), owned by P64; P58 consumes the trait |
| P38-rev §12.1 **struck** the transparent-`<input>` overlay; §12.3 reaffirms FE-15 (mirror base) + FE-16 (fallback ladder) as the shared imported base; the RW-11 grep gate now permits `createElement` **only** inside `a11y_mirror.mjs` | `BLUEPRINT-P38-webgpu-render-engine.md:656-692` (§12.1), `:739-750` (§12.3) | **VERIFIED — no DOM `<input>`/`contenteditable` on any platform; web live-edit a11y is P58's synthetic ARIA-textbox** |
| P51's a11y precedent = "the a11y tree contains the attribution node whenever a map frame is composed" (one surface, one assertion) | `BLUEPRINT-P51-open-map-routing.md:374-375` (§4.7), `:463` (DoD row M7) | **VERIFIED — this is the ONE-surface harness P58 generalizes (§M5); P51 retro-imports the shared gate** |
| AccessKit ground truth: native adapters ready (`windows`/`macos`/`unix`/`android`/`winit`), text caret/selection via `text_selection` attr + `SetTextSelection` action; **web/canvas adapter planning-only, "probably most difficult of all," no timeline** | `OPUS-R1-INTERFACE-RENDERING-2026-07-18.md:89-96`, `:108-119` | **VERIFIED — the two-platform split (§1) is forced by upstream, not chosen; solving AccessKit-web is explicitly NOT this blueprint's job** |
| No sibling W1 blueprint (P57/P59/P60/P61/P62/P63/P64) exists on disk yet — written in parallel | `ls …/CORE-ROADMAP-2026-07-17/ | grep -E 'P5[7-9]|P6[0-4]'` → 0 | VERIFIED — P58 defines the ARIA-textbox convention **first**; P57 reconciles to it (§4.3), not the reverse |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. The two platform paths — kept explicit, forced by upstream (grounded in R1 §2)

R1 verified AccessKit's web/canvas backend is "planning-only … no timeline, funding-dependent."
This is not a taste call; it partitions the product into two a11y paths that must both exist and
must stay explicit. **Solving AccessKit's own web-backend roadmap gap is out of scope** (§3
anti-scope) — this blueprint routes *around* it honestly.

| Path | Targets | Mechanism | Text caret/selection a11y | Hand-rolling |
|---|---|---|---|---|
| **(a) Native** | winit desktop (per P39-rev's `winit+wgpu+AccessKit` candidate, X3), Android | **AccessKit crate directly** (`accesskit` + `accesskit_winit` + `accesskit_android`) — production-ready on exactly these targets | **Free & correct** — text-run nodes + `text_selection` attribute + `SetTextSelection` action; the screen reader announces caret/selection natively | **Zero** — the tree is pushed, the platform AT does the rest |
| **(b) Web (WASM)** | any browser (the §16.8 zero-friction path) | **Hand-rolled hidden semantic DOM mirror** (P38 FE-15, generalized to every screen) kept in sync with live wgpu scene/widget state; SR-only, zero painted pixels | **Synthetic ARIA-textbox** (§M3) — a hidden `role=textbox` with manually-driven caret/selection announcers; **honestly weaker** than a native editable element (recorded, not hidden — the native clients are the strong-a11y path) | **All of it** — no crate to lean on; this is the only option until an AccessKit web backend ships |

**The one architectural move that makes both paths cheap and keeps them from drifting** (X1, made
concrete in §M1/§M6): both consume ONE **presentation-free `A11yTree`** derived by a pure function
`mirror(&SemanticScene)`. Native serializes it into an `accesskit::TreeUpdate`; web reconciles it
into hidden DOM. Neither path re-derives semantics from geometry — so a screen that is accessible
on native is accessible on web *from the same source*, and brand-preview parity (§M6) falls out
structurally.

**Buildability split (the lane structure, §10):** path (b) — web mirror, ARIA-textbox convention,
and the entire Playwright harness — is **buildable TODAY with zero network unlock**: it rides the
DOM plus the CPU render floor that already exists (`wasm/src/lib.rs:57` `compose_field`), and a11y
is renderer-independent by construction (P38 §3.6). Only path (a)'s native adapter needs the
**AK-unlock** crate grant (§2). This is deliberate: the web a11y story — the harder, unsolved-upstream
one — does not wait on any gate.

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── engine/src/semantics.rs — NEW module: the ONE semantic source of truth ──
// The accessible tree is a PURE FUNCTION of this scene; the GPU frame is ANOTHER
// function of the same scene (compose(), field_frame.rs:218). One SemanticScene
// ⇒ one frame AND one mirror — this is the draft-parity keystone (§M6).

pub type WidgetId = u32;   // stable per-widget identity across frames (reconcile key)

/// AccessKit-aligned role set (Wave-0 subset). Each name maps 1:1 onto
/// `accesskit::Role` so the native adapter (§M4) is a table lookup, not a
/// translation layer — and onto an ARIA role token for the web mirror (§M2).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Group, Heading, Label, Button, Link, Image, List, ListItem,
    Status,     // order-state / progress — web: aria-live="polite"  (§4.1 money-as-text)
    Alert,      // errors / money-action confirmation — web: aria-live="assertive"
    TextInput,  // the synthetic ARIA-textbox surface (§M3); native: text field w/ selection
}

/// Semantic state. `value_text` carries formatted money via money_guard
/// (present_money → String), NEVER a tweened value (money_guard.rs:60/:72).
pub struct NodeState {
    pub disabled: bool, pub selected: bool, pub busy: bool,
    pub value_text: Option<String>,   // e.g. "DELIVERED", "$12.40", "3 items"
}

/// Live text-editing state. Produced by P57's cosmic-text editor; the a11y WIRE
/// SHAPE is owned HERE (§4.3) so the web mirror and the native adapter agree on
/// one representation. Byte offsets are grapheme-aligned by P57's buffer.
pub struct EditState {
    pub text: String,       // current buffer contents (Latin+Cyrillic Wave-0)
    pub caret: usize,       // byte offset of the cursor
    pub sel_anchor: usize,  // selection anchor; sel_anchor == caret ⇒ no selection
    pub composing: bool,    // ALWAYS false Wave-0 (no IME, §0.2-2); reserved for v2
}

pub struct SemanticNode {
    pub id: WidgetId,
    pub role: Role,
    pub name: String,           // accessible name (label / order id / item name)
    pub bounds: [f32; 4],       // x,y,w,h in SCREEN space — projection-FLATTENED (§M6-3)
    pub focusable: bool,
    pub tab_index: u32,         // keyboard/tab order; 0 = not in the tab ring
    pub state: NodeState,
    pub edit: Option<EditState>,// Some ⇔ Role::TextInput; drives the ARIA-textbox (§M3)
    pub children: Vec<WidgetId>,// tree structure (List→ListItem, Group→…)
}

/// The semantic scene — the SAME state the renderer consumes for widgets. NO
/// theme token, NO view/projection matrix, NO render floor is a field of this
/// type (they live in P38's FrameUniforms) — that absence is what makes §M6 hold.
pub struct SemanticScene { pub nodes: Vec<SemanticNode>, pub root: WidgetId }

/// The accessible tree — the ONE artifact both platform paths consume.
/// Flattened in tab/DOM order; `mirror()` is the sole producer.
pub struct A11yTree { pub nodes: Vec<A11yNode> }
pub struct A11yNode { pub id: WidgetId, pub role: Role, pub name: String,
                      pub state: NodeState, pub bounds: [f32; 4],
                      pub focusable: bool, pub tab_index: u32, pub edit: Option<EditState>,
                      pub depth: u16, pub parent: Option<WidgetId> }

/// THE keystone (§M1/§M6). Pure, deterministic, presentation-free. Because no
/// token/matrix/floor is an input, the output is INVARIANT under every
/// presentation-only swap — the falsifiable draft-parity guarantee.
pub fn mirror(scene: &SemanticScene) -> A11yTree;

// ── Web mirror constants (web/src/lib/a11y_mirror.mjs, §M2) ──────────────────
// Mirror root: SR-only, zero painted pixels (the standard clip technique, P38 §2).
pub const MIRROR_ROOT_STYLE: &str =
    "position:fixed;clip-path:inset(50%);width:1px;height:1px;overflow:hidden;";
pub const MIRROR_NODE_BUDGET_DEFAULT: usize = 256; // per-screen cap (§4.2 axis; surface BP may TIGHTEN, never loosen without a note)
pub const RECONCILE_BUDGET_MS: f64 = 0.5;          // change-frames only (fits P38 §6's 0.5 ms mirror slice)

// ── Synthetic ARIA-textbox convention (§M3) — the OWNED web contract ─────────
// Element:   <div role="textbox" aria-multiline={0|1} aria-label=NAME
//                 aria-describedby=HINT_ID aria-readonly=false>
//            text content == EditState.text verbatim (NOT contenteditable — struck, P38 §12.1)
// Machine-checkable STATE (AT announcement is SR-dependent; STATE is not) — the
//   harness asserts on THESE, so caret/selection is falsifiable cross-browser:
pub const ATTR_CARET: &str = "data-caret";          // = EditState.caret (byte)
pub const ATTR_SEL_ANCHOR: &str = "data-sel-anchor";// = EditState.sel_anchor (byte)
pub const ATTR_TEXT: &str = "data-text";            // = EditState.text (round-trip pin)
// Announcers (synthesize what a native editable element would emit):
pub const CARET_ANNOUNCER_LIVE: &str = "polite";    // emits grapheme/word crossed on caret move
pub const SELECT_ANNOUNCER_LIVE: &str = "assertive";// emits "selected N: <text>" on selection change
// IME/composition slot: reserved, EMITS NOTHING Wave-0 (Latin+Cyrillic need no
// composition, §0.2-2). Populating it is v2 work, explicitly out of scope (§3).

// ── engine/src/a11y_native.rs — NEW, #[cfg(feature = "a11y_native")] (§M4) ───
// AK-unlock manifest (ONE network grant, all together — the P38-O18a discipline):
//   accesskit          — the tree schema + Action handling (SetTextSelection)
//   accesskit_winit    — desktop bridge (winit event loop ↔ platform AT)
//   accesskit_android  — Android bridge
//   (accesskit_windows/macos/unix are pulled transitively by accesskit_winit;
//    NO other a11y dep is authorized by this manifest.)
pub fn to_tree_update(tree: &A11yTree) -> accesskit::TreeUpdate;   // A11yTree → AccessKit
pub fn apply_action(a: accesskit::ActionRequest, ed: &mut EditState); // SetTextSelection → caret/sel
```

Rejected alternatives (DECART one-liners): **wait for AccessKit-web** — rejected: "planning-only,
no timeline" (§0); the product cannot gate launch a11y on an upstream unfunded item — the
hand-rolled mirror IS the AccessKit role-tree pattern by hand on web, revisited only if a web
backend ships. **Hidden `contenteditable`/`<input>` to get IME + caret for free** — rejected:
struck by operator ruling (P38 §12.1 / SYNTHESIS §0.2-2); no editable DOM element exists on any
platform. **Per-surface bespoke a11y harness** (each surface writes its own Playwright a11y test) —
rejected: that is exactly the re-derivation X1 forbids; one shared harness (§M5), imported. **A
second mirror for brand-preview** — rejected: the preview is a uniform-buffer swap on the same
SemanticScene (R5 §4.3), so the mirror is already correct; standing up a second is scope violation
(§M6 makes this un-representable, not merely discouraged). **Deriving semantics from `SdfShape`
geometry** — rejected: geometry cannot answer "is this a button" (§0); semantics is a first-class
authored layer beside geometry, not an inference.

---

## 3. Scope — what P58 owns vs deliberately does NOT

**P58 owns (build items §M1–§M7):**

| Item | Content |
|---|---|
| M1 | `engine/src/semantics.rs`: the shared `SemanticScene`/`SemanticNode`/`Role`/`EditState` model + the pure `mirror(&SemanticScene) -> A11yTree` derivation — the ONE source both paths consume |
| M2 | `web/src/lib/a11y_mirror.mjs`: FE-15 generalized — hidden-DOM semantic mirror for **every** screen; keyed/diff reconcile; SR-only root; settle-gated; `A11yTree` crossed the wasm boundary via P38 G7 ptr/len |
| M3 | The **synthetic ARIA-textbox convention** (web live text-editing) — OWNED here; consumed by P57 (§4.3) |
| M4 | `engine/src/a11y_native.rs` (`#[cfg(feature="a11y_native")]`): AccessKit adapter — `A11yTree`→`TreeUpdate`, caret/selection via `text_selection` + `SetTextSelection`, winit + Android wiring (AK-unlock) |
| M5 | The **shared Playwright accessibility-tree harness** (`web/tests/a11y/harness.mjs` + spec): role/name/state, keyboard tab-order, live-edit caret/selection, no-visible-DOM, stale-node, mirror-node budget, presentation-invariance — the importable DoD gate |
| M6 | The **draft-parity invariant** (X1): `mirror()` is invariant under every presentation-only swap (brand tokens · view/projection · render floor) — stated as an architectural invariant with a falsifiable test, so brand-preview (P70) is accessible **by construction** |
| M7 | The **surface-integration contract**: how P69/P70/P71/P73 + P57 + P51 import the harness and supply a per-screen a11y manifest; mirror-node budget as a per-blueprint DoD line; the FE-16 floor line |

**P58 explicitly does NOT own:**

- **NOT solving AccessKit's web/canvas backend.** That upstream gap (R1 §2) is real, unfunded, and
  not this blueprint's job (or the repo's) to close. P58 routes around it with the hand-rolled
  mirror; if AccessKit-web ever ships, §M2 names it as a future swap-in (§4.5). A diff that
  vendors or forks a speculative AccessKit-web is out of scope.
- **NOT the text-editing engine.** Cursor motion, selection extension, clipboard, shaping,
  grapheme clustering are **P57** (`cosmic-text`). P58 owns only the a11y **wire shape**
  (`EditState`, §2) and its ARIA-textbox mapping (§M3); P57 populates it. Byte-offset semantics
  are P57's authority — P58 asserts on them, never redefines them.
- **NOT IME / non-Latin composition.** Latin+Cyrillic Wave-0 need no composition events (§0.2-2);
  the ARIA-textbox reserves a composition slot that emits nothing. v2's IME a11y is a bounded
  future unit, not a Wave-0 unknown (§16.58 RTL/IME-deferred boundary).
- **NOT raising the mobile-web soft keyboard.** No editable DOM element ⇒ no standard mechanism;
  that is P63's named spike (SYNTHESIS §0.2-2 / X2), with voice as the honest interim on that one
  platform combination. P58's ARIA-textbox is a **reader** surface (announces state), not a
  keyboard-raising element.
- **NOT the intent/keyboard runtime.** Tab/focus/arrow **intents** normalize through P38-rev's
  `InputSource`→`Intent` (§12.2 constraint 3), owned by **P64**. P58 consumes focus/selection
  intents and reflects them in the tree; it does not build the input router.
- **NOT a visible-DOM anything.** The mirror is SR-only, zero painted pixels; the RW-11 grep gate
  (P38 §12.1) — `createElement` permitted ONLY inside `a11y_mirror.mjs` — is inherited and
  enforced by §M5's no-visible-DOM assertion. Any painted DOM widget is a scope violation.
- **NOT a second semantic/design-token system.** The SemanticScene is authored beside P38's
  geometry/token pipelines by the surface blueprints; P58 defines its shape, not its content.

**Hard gate, stated honestly:** M1/M2/M3/M5/M6/M7 land with **zero network unlock** (DOM + the
existing CPU floor). Only M4 (native AccessKit) is blocked on the **AK-unlock** crate grant — one
event, manifest pinned in §2. Nothing else waits on it, and the web a11y path (the upstream-unsolved
one) ships first.

---

## 4. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

### M1 — the shared semantic model + the pure mirror derivation (`engine/src/semantics.rs`)

The keystone. Today the renderer answers "what colour is this pixel" but not "what widget is
this" (§0). M1 adds the semantic layer *beside* geometry, keyed by `WidgetId`, and the pure
`mirror(&SemanticScene) -> A11yTree` (§2). The derivation flattens the child tree into tab order,
copies role/name/state/bounds/edit, and computes `depth`/`parent` — **no presentation input
touches it** (that absence is load-bearing, §M6). `mirror` is deterministic and allocation-bounded
(one `Vec` sized to `scene.nodes.len()`).

**Spec-first, event-shaped (item 3):** reconcile is modeled as a diff event stream —
`NodeAdded{id}` / `NodeRemoved{id}` / `NodeUpdated{id, field}` / `CaretMoved{id, caret}` /
`SelectionChanged{id, anchor, focus}` — computed by comparing successive `A11yTree`s keyed by
`WidgetId`. Tests assert on the event sequence, not only the end tree (matches the kernel's
`decide`/fold law).

**RED→GREEN:** `mirror_is_pure_and_deterministic` — same `SemanticScene` twice ⇒ byte-identical
`A11yTree` (RED today: no `mirror` exists); `diff_emits_keyed_events` — mutate one node's `name`,
assert exactly one `NodeUpdated{that id, name}` and nothing else. **Adversarial:** a `SemanticScene`
with a `children` cycle (id A→B→A) ⇒ typed refusal (`MirrorError::Cycle`), never an infinite loop
(bounded traversal with a visited-set, asserted); a `TextInput` node with `edit: None` ⇒ typed
refusal (a text field MUST carry `EditState`); duplicate `WidgetId` ⇒ typed refusal
(`MirrorError::DupId`) — the reconcile key must be unique or diffing is undefined.

### M2 — the web hidden-DOM mirror, generalized to every screen (`web/src/lib/a11y_mirror.mjs`)

FE-15's map-shaped mirror (P38 §3.6) becomes universal. The module receives the `A11yTree` across
the wasm boundary as **ptr/len** (P38 G7 conventions — frame-scoped view, re-derive every reconcile,
never cache; the detached-buffer rule is P38 §3.7's, cited not re-proven) and reconciles it into a
hidden DOM subtree under a mirror root styled `MIRROR_ROOT_STYLE` (§2 — SR-only, zero painted
pixels). Reconcile is **keyed and diff-based** (by `WidgetId`): added nodes create one `<div
role=… aria-label=…>` (focusable nodes get `tabindex` in `tab_index` order), removed nodes are
detached, updated nodes patch only the changed attribute. Money in a Status/Alert node renders as
`value_text` (already formatted via `present_money`, §4.1) — text, never a tween.

**Settle-gated (item 15):** reconcile runs **only on tree change**, piggybacking FE-14's settle
gate when it lands (`engine/src/settle.rs`, a P38 G5 item — absent today, §0) so a dormant UI has a
dormant mirror. **Interim, until G5 lands:** reconcile on `A11yTree` hash change (FNV-1a of the
serialized tree) — same "only on change" semantics, no dependency on the unlanded gate; a named
TODO keyed "P38-G5" swaps the hash trigger for the settle signal, not a silent gap.

**RED→GREEN (Playwright, via §M5 harness, local wasm path — no GPU unlock):** drive an order to
`DELIVERED`; assert a `role=status` node whose accessible name contains the order id and
"DELIVERED" (P51 §4.7 pattern, now for the general case). **Adversarial:** remove one widget from
the SemanticScene mid-test ⇒ its mirror node is gone by next reconcile (**stale-node-leak** check —
the classic mirror bug); mutate 1 node out of 200 ⇒ exactly one DOM patch, 199 nodes untouched
(reconcile is a diff, not a rebuild — asserted via a MutationObserver count); force
`navigator.gpu = undefined` ⇒ CPU-floor renderer draws AND every a11y assertion still passes
unchanged (a11y is renderer-independent **by construction**, the FE-16 floor line §M7).

### M3 — the synthetic ARIA-textbox convention (web live text-editing) — OWNED here

Because no `<input>`/`contenteditable` exists (P38 §12.1) and AccessKit-web does not exist (§0),
web must **synthesize** a textbox from a plain hidden element. This is the convention P57 references
provisionally and reconciles against (§4.3). The spec:

1. **Element:** `<div role="textbox" aria-multiline={0|1} aria-label=NAME aria-describedby=HINT
   aria-readonly="false">`. Its accessible **value** is the `EditState.text` mirrored verbatim
   (also pinned in `data-text` for the harness). It is **not** contenteditable — the browser will
   not natively announce caret movement, so we synthesize it (steps 2–3).
2. **Caret announcer:** a visually-hidden `aria-live="polite"` region owned by the field. On
   `CaretMoved`, it emits the grapheme (character-nav) or word (word-nav) the caret **crossed** —
   the same information a native input announces while arrowing. The caret byte offset is exposed
   as `data-caret` (machine-checkable, §M5).
3. **Selection announcer:** a visually-hidden `aria-live="assertive"` region. On `SelectionChanged`,
   it emits `"selected N characters: <selected text>"` (native SR selection semantics). Anchor/focus
   byte offsets are exposed as `data-sel-anchor` (focus == `data-caret`).
4. **Value updates:** on `NodeUpdated{edit.text}` the textbox value and `data-text` update together;
   an `aria-live` value change is suppressed during rapid typing (announce on settle, not per
   keystroke — avoids announcement flooding, the known synthetic-textbox failure mode).
5. **IME/composition:** the reserved slot **emits nothing** Wave-0 (`composing` is always `false`,
   §2). v2 (non-Latin) work populates it; out of scope here (§3).

**Honest limitation, recorded (X1):** synthetic announcement is weaker and more screen-reader-
dependent than a native editable element — the machine-checkable `data-*` state (not the AT prose)
is the falsifiable contract; the **native clients are the strong-a11y path** for text editing.

**RED→GREEN (harness):** type `"hi"` then arrow-left into a canvas text field; assert
`data-text=="hi"`, `data-caret=="1"`, and the caret announcer's last emission was `"h"`. Select the
whole word; assert `data-sel-anchor=="0"`, `data-caret=="2"`, and the selection announcer emitted
`"selected 2 characters: hi"`. **Adversarial:** paste a 10 kB string ⇒ value mirrored once, no
per-character announcement storm (announcement debounced to settle); a caret move with **no** text
change ⇒ caret announcer fires, value announcer silent (the two channels are independent);
`composing==true` injected ⇒ typed refusal Wave-0 (must be false until v2 — an IME event reaching
the Wave-0 path is a bug, caught RED).

### M4 — the native AccessKit adapter (`engine/src/a11y_native.rs`, AK-unlock)

`to_tree_update(&A11yTree) -> accesskit::TreeUpdate` maps each `A11yNode` onto an
`accesskit::Node` (the `Role` enum is a 1:1 lookup, §2). A `TextInput` node becomes an AccessKit
text field with a **text-run** child and a `text_selection` attribute built from `EditState`
(anchor/focus positions) — so the platform screen reader announces caret movement and selection
**natively, for free** (R1 §2). `apply_action` handles the `SetTextSelection` action request by
writing back into `EditState` (the AT can move the caret; P57's editor consumes the update). Wiring:
`accesskit_winit` on desktop (winit event loop ↔ platform AT), `accesskit_android` on Android.

**RED→GREEN (native, on AK-unlock, `#[cfg(feature="a11y_native")]`):** `tree_update_roundtrip` —
an `A11yTree` with one `Status` + one `TextInput` produces a `TreeUpdate` whose node roles/names
match, and the text field's `text_selection` equals `EditState`'s caret/anchor;
`set_text_selection_writes_back` — a synthetic `SetTextSelection` action mutates `EditState` to the
requested range. **Adversarial:** `EditState.caret` beyond `text.len()` ⇒ typed clamp-or-refuse
(byte offsets must be valid — P57's invariant, re-checked at the boundary); a `Role` with no
AccessKit equivalent (none in the Wave-0 set, but the exhaustive `match` has no `_` arm so adding a
`Role` variant without a mapping is a **compile error**, not a runtime skip). **CI gap named
honestly (§4.6):** no native-AT CI runner exists; `#[cfg(feature="a11y_native")]` tests run on
operator/developer hardware until P45 provides one — each doubles as the AK-unlock marker.

### M5 — the shared Playwright accessibility-tree harness (`web/tests/a11y/harness.mjs`)

Generalizes P51's single assertion ("attribution node present when a map frame composes",
§4.7) into a reusable, importable module. It exports assertion helpers that operate on Playwright's
**accessibility-tree snapshot API** + the mirror's machine-checkable `data-*` state — so every
future surface asserts a11y without re-deriving one:

| Helper | Asserts |
|---|---|
| `assertRoleNameState(page, {role, name, state})` | a mirror node with that role/name and state flags (`disabled`/`selected`/`busy`/`value_text`) exists |
| `assertTabOrder(page, [id…])` | pressing Tab N times lands focus on the focusable mirror nodes in `tab_index` order (keyboard-order verification) |
| `assertCaret(page, fieldId, {caret, selAnchor})` | the synthetic ARIA-textbox's `data-caret`/`data-sel-anchor` equal the expected byte offsets (live-edit caret/selection, cross-browser deterministic) |
| `assertLiveEdit(page, fieldId, keystrokes, expected)` | type a keystroke sequence into a canvas field; assert the mirror's caret/selection/value nodes update per keystroke (the live-edit scenario R1 §2 requires) |
| `assertNoVisibleDom(page)` | every element except `<canvas>` + the mirror root paints 0 pixels (`getBoundingClientRect` empty or clipped); the mirror root itself paints 0 px |
| `assertMirrorNodeBudget(page, max)` | mirror node count ≤ `max` (default `MIRROR_NODE_BUDGET_DEFAULT`; the per-blueprint DoD budget, §4.2) |
| `assertPresentationInvariance(page, swap)` | snapshot the tree, apply a presentation-only swap (`swap ∈ {brand, viewproj, floor}`), re-snapshot, assert **byte-identical** (the §M6 falsifier) |
| `a11yGate(page, manifest)` | runs a surface's whole per-screen manifest (roles/names/states + tab order + budget + no-visible-DOM) in one call — the single line a surface blueprint's DoD imports |

**RED→GREEN:** the harness ships with a **self-test** (`web/tests/a11y/harness.self.spec.mjs`)
against a fixture SemanticScene exercising every helper, so a regression in the *harness itself* is
caught (the harness is refereed, item 20 P7). **Adversarial (the harness must catch these, proving
it has teeth):** a deliberately stale mirror node ⇒ `assertRoleNameState` for a removed node must
FAIL; a scrambled tab order ⇒ `assertTabOrder` FAILS; a caret off-by-one ⇒ `assertCaret` FAILS; a
tree that drifts under a brand swap ⇒ `assertPresentationInvariance` FAILS. Each teeth-test runs
once with the defect injected, asserts red, restores — a harness that cannot fail is not a gate.

### M6 — the draft-parity invariant (X1), stated as math with a falsifier

**Invariant (architectural, not prose):** `mirror(&SemanticScene)` is a **pure function of the
SemanticScene alone**. No design token, no view/projection matrix, and no render floor is an input
to it (§2 — those live in P38's `FrameUniforms`, a *different* type). Therefore the `A11yTree` is
**invariant under every presentation-only swap**:

1. **Brand/theme swap** (R5 §4.3): brand preview is a `queue.write_buffer` of `theme_tokens` — it
   changes pixels, not the SemanticScene ⇒ the mirror is byte-identical draft vs live. **Brand-preview
   accessibility is correct by construction, not by separate work** (this is X1's "don't solve it
   twice"; P70's owner brand-preview inherits it — it writes zero a11y code of its own).
2. **View/projection swap** (P38-rev §12.2 AR/VR constraint 1): flat-ortho vs perspective vs stereo
   changes only the matrix; `bounds` are projection-flattened at the boundary but role/name/state
   are unchanged ⇒ the tree is identical. A11y-in-AR/VR is the same tree from the same source.
3. **Render-floor swap** (FE-16): WebGPU → WebGL2 → CPU `compose_field` changes the render path,
   not the semantics ⇒ identical tree (the M2 `navigator.gpu=undefined` adversarial is this case).

**Falsifiable test:** `a11y_tree_invariant_under_presentation_swap` (native, pure-Rust) — build one
`SemanticScene`, produce `A11yTree` under two brand token sets, two matrices, and two floors;
`assert_eq!` all trees. Web mirror equivalent via `assertPresentationInvariance` (§M5). **The
guarantee is structural:** because `mirror`'s signature takes only `&SemanticScene`, a presentation
input *cannot* reach it without a type change — drift is un-representable, not merely tested-against.
**Not-done clause:** any `mirror` overload/variant that accepts a token, matrix, or floor argument =
NOT done (it would break the invariant by construction); brand-preview shipping its own a11y path =
NOT done (§M6-1 makes it redundant).

### M7 — the surface-integration contract (how every surface imports the gate)

Each surface blueprint (**P69** storefront, **P70** owner, **P71** courier, **P73** landing, plus
**P57**'s a11y half and **P51**'s map) does exactly three things — no re-derivation:

1. **Author a `SemanticScene`** for its screens beside its P38 geometry (roles/names/states/tab
   order/edit fields). This is the surface's own content; P58 owns only the shape.
2. **Import `a11yGate(page, manifest)`** (§M5) into its DoD, supplying a per-screen manifest of
   expected roles/names/states + tab order + a **mirror-node budget** (its DoD line; default
   `MIRROR_NODE_BUDGET_DEFAULT`, tightened per screen — a checkout wizard with 40 controls declares
   40-ish, not 256, so mirror bloat is caught).
3. **Carry the FE-16 floor line** verbatim: "renders correctly, and a11y passes identically, on the
   WebGL2 and CPU floors" — the standing WebGL2-floor gate (SYNTHESIS §3.2 rationale 7), which §M6-3
   already guarantees for the a11y half.

**P51 retro-import (named, so nothing double-owns):** P51 §4.7's map-attribution assertion becomes
one instance of `assertRoleNameState` in the shared harness; P51's DoD row M7 cites §M5 instead of
its own bespoke check. **P48/P52 supersession:** their a11y is subsumed by P70/P71 which own the
owner/courier surfaces (SYNTHESIS §5 canon-diffs row P48/P52) — those blueprints author the
SemanticScene and import the gate.

---

## 5. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 5.1 Hazard-safety as math (item 6)

Reachability arguments, not prose:
- **Mirror drift is un-representable, not merely tested.** `mirror` takes `&SemanticScene` and
  nothing else (§2), so a presentation change (brand/matrix/floor) *cannot* alter the tree without a
  type change — the "accessible in draft, inaccessible in preview" failure mode is structurally
  absent (§M6), not policied.
- **A stale/leaked mirror node is a tested-unreachable state**, not a hope: the keyed diff (§M2)
  plus the stale-node adversarial (§M2/§M5) make "node removed from scene but present in DOM" a RED
  test, turning a heisenbug class into a deterministic failure.
- **A11y-absent is degrade-never-crash.** The mirror is additive: with `a11y_native` off (default
  build) native pushes no tree; with JS disabled the web mirror is absent — in both cases the order
  flow (kernel `decide`/fold) is untouched (no shared mutable state across the boundary; the tree is
  read-only output). "No mirror ⇒ broken order" is unreachable.
- **Money in a Status/Alert node is text, never a tween.** `value_text` is a `String` from
  `present_money` (`money_guard.rs:60`); `Money` implements no `FieldValue`, so a tweened money
  announcement does not compile — the render-side law (P38 §4.1) extends to the a11y surface.
- **Text-field caret safety:** byte offsets crossing the wasm/native boundary are re-validated
  against `text.len()` (§M4 adversarial); an out-of-range caret is clamped-or-refused, never used to
  index — no panic path from a hostile `EditState`.

### 5.2 Schemas & scaling axes (item 8)

`SemanticScene`/`A11yTree`: axis = **mirror nodes per screen**; typical screen ≤ 10², budget
`MIRROR_NODE_BUDGET_DEFAULT` = 256 (§2), enforced per-blueprint (§M7). Break point: a screen needing
> 256 semantic nodes (a very dense list) → virtualize the list (mirror only the visible window +
`aria-setsize`/`aria-posinset` for the full count — the standard large-list a11y pattern, named not
built). Reconcile cost: **O(changed nodes)** by the keyed diff, not O(scene) — `RECONCILE_BUDGET_MS`
= 0.5 ms on change-frames (§6). Announcement rate (ARIA-live): debounced to settle (§M3 step 4) — the
axis is keystrokes/sec, bounded by human typing; break point is paste (handled by value-on-settle,
not per-char). `mirror()` allocation: one `Vec` of `scene.nodes.len()` — linear, no hidden axis.

### 5.3 Isolation (item 11), mesh awareness (item 12), living memory (item 15)

**Isolation:** the mirror is a **state consumer**, never a mutator — it reads `SemanticScene` and
writes only its own DOM/AccessKit tree; a mirror panic (web) or adapter fault (native) cannot
corrupt an order (P38 §4.3 bulkhead, inherited). The web mirror lives in the DOM, structurally
disjoint from the GPU surface — no surface-conflict class (the winit+wgpu+AccessKit native path,
X3, avoids even the webview). `a11y_native` is a cargo feature: the default build compiles zero
AccessKit code, so an adapter regression cannot break kernel/engine CI. **Mesh:** entirely
**node-local** — a11y is pure presentation of local kernel/scene state; **zero transport payloads
originate here** (no gossip, no wire, no SyncFrame entry). **Living memory:** the mirror is hot
while the UI moves and dormant on settle (FE-14) — demote-never-delete applied to a11y compute; the
tree is recall-by-recompute from `SemanticScene` (never stored), matching the "store S(t), not
frames" principle.

### 5.4 Rollback / self-healing vocabulary (item 13, used precisely)

**Self-Termination leg claimed:** typed `MirrorError` (cycle/dup-id/text-input-without-edit, §M1),
the presentation-invariance guaranteed by `mirror`'s signature (§M6), the no-`_`-arm exhaustive role
match (§M4), the un-compilable money tween. **Self-Healing leg claimed narrowly:** the keyed diff
reconcile is genuine error-correction — each change regenerates the tree from the current
SemanticScene, so a transiently-wrong DOM self-corrects to source-of-truth on the next reconcile;
claimed for the **presentation tree only**, not for state. **Snapshot-Re-entry: NOT claimed** — the
tree is derived, never stored; recovery = recompute from `SemanticScene` (re-derivation, not
snapshot restore). Mechanical rollback: every module is additive (`semantics.rs`, `a11y_native.rs`,
`a11y_mirror.mjs`, `web/tests/a11y/`) — deletion + `a11y_native` off restores today's exact tree.

### 5.5 Error-propagation gates (item 14) + Linux discipline (item 9) + tensor/spectral (item 16)

**Named gates that turn this blueprint's bug classes into compile/CI failures:** the
presentation-invariance test (mirror drift), the stale-node adversarial (leak), the no-visible-DOM
Playwright assertion + the RW-11 `createElement`-only-in-`a11y_mirror.mjs` grep gate (canon drift),
`assertTabOrder` (keyboard-order regression), `assertCaret`/`assertLiveEdit` (caret desync), the
exhaustive `Role` match (native role-mapping omission = compile error), the `composing==false`
Wave-0 guard (IME leaking into the Latin/Cyrillic path). **Linux-discipline verdicts:**
**ALREADY-EQUIVALENT** — one mirror, one harness (P38 §12.3's "exactly one mirror implementation");
**REINFORCES** — feature-gated native hardware/AT access with a software (web-DOM + CPU-floor)
floor, the kernel-module discipline; **EXTENDS** — the shared cross-surface harness and the
presentation-invariance test are new gate classes this repo adds for GPU-canvas a11y;
**GAP** honestly named — no native-AT CI runner and no AccessKit-web backend exist (§M4 / §3), both
recorded, neither hand-waved. **Item 16 (tensor/spectral/eqc): N/A, stated not decorated** — a11y is
discrete symbolic semantics (roles/names/states), not field/spectral math; inventing a spectral
representation for an accessibility tree would be exactly the ritual math the Anu/Ananke discipline
forbids. eqc-rs: no closed-form organ appears in this phase.

---

## 6. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| M1 | no `semantics.rs`; `mirror` absent | `mirror_is_pure_and_deterministic`; `diff_emits_keyed_events`; cycle/dup-id/text-input-no-edit refused | determinism + diff-event tests (ledger row) |
| M2 | no `a11y_mirror.mjs` (§0); no web tests | `role=status DELIVERED` node present; stale-node gone next reconcile; 1-of-200 single-patch; `navigator.gpu=undefined` a11y unchanged | stale-node + renderer-independence tests (ledger row) |
| M3 | no ARIA-textbox; caret un-assertable | `data-caret`/`data-sel-anchor` correct; caret + selection announcers fire; paste no-storm; `composing` refused | caret/selection assertion + announcement-debounce tests |
| M4 | no native adapter | `tree_update_roundtrip`; `set_text_selection_writes_back`; out-of-range caret refused; role match exhaustive (compiles) | roundtrip + set-selection tests (AK-unlock; operator hardware) |
| M5 | no harness; surfaces would each re-derive | self-test green; every teeth-test (stale/scrambled/off-by-one/brand-drift) goes RED with defect injected | harness self-test + 4 teeth-tests (ledger row) |
| M6 | invariant unstated, un-tested | `a11y_tree_invariant_under_presentation_swap` (brand · viewproj · floor) byte-identical; web `assertPresentationInvariance` green | presentation-invariance test (ledger row) |
| M7 | surfaces have no shared gate to import | P51 retro-imports the gate; a fixture surface manifest runs through `a11yGate` green; mirror-node budget enforced | the import contract is exercised by ≥1 surface's DoD |

**Not-done clauses:** any DOM `<input>`/`contenteditable` for text entry on any platform = NOT done
(P38 §12.1 grep gate); a `mirror` variant accepting a token/matrix/floor = NOT done (breaks §M6 by
construction); brand-preview shipping its own a11y path = NOT done (§M6-1 redundant); a surface
blueprint writing a bespoke a11y test instead of importing §M5 = NOT done (X1 re-derivation); a
harness teeth-test that cannot go RED = NOT done (a gate that can't fail isn't one); waiting on
AccessKit-web for the web path = NOT done (§3 — the hand-rolled mirror is the Wave-0 answer).

---

## 7. Benchmark plan (item 10) — reconcile cost + harness runtime, measured

Criterion + `bench_track` baseline discipline (P-A §6 / P51 §7, reused, zero new infra):
`semantics/mirror_derive_256` (the pure `mirror()` over a 256-node scene — target < 0.1 ms, the
"tree derivation is free" claim made falsifiable), `semantics/diff_1_of_256` (single-node change ⇒
one event — the O(changed) claim, target < 0.05 ms). Web-side: the `a11y_mirror.mjs` reconcile
budget `RECONCILE_BUDGET_MS` = 0.5 ms/change-frame is asserted by an instrumented reconcile counter
in the M2 spec (benchmark-as-test, P38 §6 discipline — the number is a tripwire, not a report). All
added RED-commit-first so baselines auto-seed to `BENCH_HISTORY.md`. **Telemetry:** mirror node
count + reconcile time per screen ride the existing native-trackers hooks (P-H's lane), so a
mirror-bloat regression (a surface exceeding its budget) surfaces automatically, not only at review.
Harness runtime is itself budgeted (the shared gate runs on every surface's CI — a slow harness
taxes every blueprint): the self-test asserts total harness wall-time per screen < 5 s headless.

---

## 8. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 (W1 scope) + **X1** (the cross-cutting reasoning this
blueprint implements) + §3.2 rationale 7 (WebGL2 floor gate) · `BLUEPRINT-P38-webgpu-render-engine.md`
§2/§3.6 (FE-15 mirror base, §2 node shape), §12.1 (struck `<input>`, the RW-11 grep gate), §12.2
(AR/VR view-matrix constraint consumed by §M6-2), **§12.3** (FE-15/FE-16 as the shared imported
base) · `BLUEPRINT-P51-open-map-routing.md` §4.7 (the ONE-surface a11y-tree precedent this
generalizes) · `OPUS-R1-INTERFACE-RENDERING-2026-07-18.md` §2 (AccessKit native-ready / web
planning-only — the fact that forces §1's two paths) · `docs/regressions/REGRESSION-LEDGER.md` (five
rows named in §6) · `HERMETIC-ARCHITECTURE-PRINCIPLES.md` (§9). **Consumers (write-order downstream,
each imports §M5 + authors a SemanticScene):** P57 (a11y half — reconciles to §M3), P69, P70 (brand
preview parity by §M6), P71, P73; P51 retro-imports; P48/P52 superseded by P70/P71. Memory:
`physics-ui-capture-quantum-math-arc-2026-07-14` (store S(t) not frames — the mirror is
recall-by-recompute, §5.3) · `field-ui-engine-arc-2026-07-13` (FE-15 provenance) ·
`dowiz-interfaces-design-arc-2026-07-13` (DZ-11 a11y hybrid — one implementation) ·
`anu-ananke-strict-discipline-feedback-2026-07-17` (§5.5's honest "spectral N/A", no ritual math) ·
`test-integrity-rules-2026-06-27` (money-as-text red-line, §5.1). Supersedes: nothing — additive;
generalizes P38 FE-15 and P51 §4.7 under one owner.

---

## 9. Hermetic principles honored (item 20 — load-bearing only)

- **P1 MENTALISM** (spec is source, code derived): the `SemanticScene` is the authored source; both
  the GPU frame (`compose`) and the accessible tree (`mirror`) are *derived* from it — the mirror is
  never hand-authored per screen, it is a function of the semantic model (§M1).
- **P2 CORRESPONDENCE** (one concept, one primitive): ONE `A11yTree` for both platform paths, ONE
  `mirror` derivation, ONE shared harness, ONE ARIA-textbox convention — "as above (SemanticScene),
  so below (frame, native tree, web DOM)."
- **P6 CAUSE-AND-EFFECT** (determinism as law): `mirror` is pure and deterministic; the
  presentation-invariance test (§M6), the keyed-diff event stream (§M1), and the caret assertions
  (§M3) each carry a falsifier — no a11y claim is un-checkable.
- **P7 GENDER** (paired verification, no self-certification): the render is refereed by the
  accessibility tree (an independent artifact), not by its own pixels; the harness is refereed by
  its own teeth-tests (§M5) — a gate that must be able to fail before it may certify anything.

(P3/P4/P5 not load-bearing here; not claimed decoratively.)

---

## 10. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites; the "no semantic layer exists" finding; mirror/harness/accesskit all confirmed-absent) |
| 2 DoD | §6 |
| 3 spec/event-driven TDD | §2 spec-first; §M1 reconcile-as-event-stream; §4 RED-first per item |
| 4 predefined types/consts | §2 |
| 5 adversarial/breaking tests | §M1–§M6 (cycle, dup-id, stale node, single-patch, gpu-undefined, paste-storm, composing-refused, brand-drift, off-by-one caret, scrambled tab order) |
| 6 hazard-safety as math | §5.1 (drift un-representable by signature; leak tested-unreachable; degrade-never-crash; un-compilable money tween) |
| 7 links docs/memory | §8 |
| 8 scaling axes | §5.2 (mirror nodes/screen, O(changed) reconcile, announcement rate — each with a named break point) |
| 9 Linux discipline | §5.5 (all four verdict classes incl. two honest GAPs) |
| 10 benchmarks+telemetry | §7 |
| 11 isolation/bulkhead | §5.3 |
| 12 mesh awareness | §5.3 (node-local, zero transport originates here) |
| 13 rollback/self-heal vocabulary | §5.4 (two legs claimed precisely, one refused) |
| 14 error-propagation gates | §5.5 (named gates), §M1/§M4 (typed refusals, exhaustive match) |
| 15 living memory | §5.3 (settle-gated hot/dormant, recall-by-recompute) |
| 16 tensor/spectral + eqc reuse | §5.5 (honestly N/A — a11y is discrete semantics, not field math; no ritual invocation) |
| 17 regression ledger | §6 (five rows named) |
| 18 agent-executable instructions | §11 |
| 19 reuse-first | §0/§1 (FE-15 mirror base, P51 harness precedent, AccessKit-on-native, existing CPU floor all reused); §2 (five rejected alternatives) |
| 20 Hermetic citations | §9 |

---

## 11. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Two lanes. **Lane A (buildable TODAY, no network):** T1–T5 — the entire web a11y path + the shared
harness ride the DOM and the existing CPU floor (`wasm/src/lib.rs:57`). **Lane B (blocked on
AK-unlock — do NOT `cargo add accesskit*` without the operator's network grant):** T6.

1. **T1 (M1 — the model is the contract).** Create `engine/src/semantics.rs` with the §2 types
   verbatim (`WidgetId`/`Role`/`NodeState`/`EditState`/`SemanticNode`/`SemanticScene`/`A11yTree`)
   and the pure `mirror(&SemanticScene) -> A11yTree`. Register `pub mod semantics;` in
   `engine/src/lib.rs`. Write RED first: `mirror_is_pure_and_deterministic`, `diff_emits_keyed_events`,
   and the cycle/dup-id/text-input-no-edit refusal tests. Acceptance: `cargo test -p engine semantics`
   green; ledger rows added.
2. **T2 (M6 — the invariant, immediately after the model so it can't regress).** Add
   `a11y_tree_invariant_under_presentation_swap` in `semantics.rs` tests: one `SemanticScene`, vary
   brand tokens / matrix / floor as *external* values (they are NOT inputs to `mirror`, that's the
   point), `assert_eq!` all resulting trees. Acceptance: green; the test compiles ONLY because
   `mirror` takes `&SemanticScene` — if someone adds a presentation arg, this file stops compiling
   (the guard).
3. **T3 (M2 — the web mirror).** Create `web/src/lib/a11y_mirror.mjs`: receive the `A11yTree` as
   ptr/len across the wasm boundary (follow P38 G7 frame-scoped-view rules exactly — re-derive every
   reconcile, never cache), reconcile keyed-by-`WidgetId` into a hidden DOM subtree under a
   `MIRROR_ROOT_STYLE` root. Trigger on `A11yTree` hash change now; leave a TODO keyed "P38-G5" to
   swap for the settle signal when `engine/src/settle.rs` lands. Extend `web/src/app.mjs`, do NOT
   rewrite it. Acceptance: the §M2 Playwright assertions green headless (via T5's harness).
4. **T4 (M3 — the ARIA-textbox convention).** Implement the §M3 element + caret/selection announcers
   + `data-*` state in `a11y_mirror.mjs` for `Role::TextInput` nodes. This is the convention P57
   consumes — record it as the authoritative spec (a short `web/src/lib/A11Y-TEXTBOX-CONVENTION.md`
   note beside the module) so P57's a11y half reconciles to a written contract, not to code
   archaeology. Acceptance: `assertCaret`/`assertLiveEdit`/announcement-debounce tests green.
5. **T5 (M5 + M7 — the shared harness, the deliverable other blueprints import).** Create
   `web/tests/a11y/harness.mjs` exporting every §M5 helper, `harness.self.spec.mjs` (self-test +
   the four teeth-tests, each injecting a defect and asserting RED), and a one-page
   `web/tests/a11y/README.md` showing a surface blueprint how to author a manifest and call
   `a11yGate` (the §M7 contract). Retro-fit P51 §4.7's attribution assertion as one `assertRoleNameState`
   instance. Acceptance: self-test + teeth-tests green; P51's map assertion runs through the shared
   gate; add the five §6 ledger rows to `docs/regressions/REGRESSION-LEDGER.md`.
6. **T6 (M4 — native adapter, FIRST task after AK-unlock).** `cargo add accesskit accesskit_winit
   accesskit_android` under `feature = "a11y_native"` (the §2 manifest is the complete authorized
   list; check the gate record before attempting the network op). Implement `engine/src/a11y_native.rs`
   per §M4 (`to_tree_update`, `apply_action`); wire `accesskit_winit` on the P39-rev winit desktop
   shell and `accesskit_android` on Android. Un-ignore + green `tree_update_roundtrip` and
   `set_text_selection_writes_back` on native hardware. The default build must still compile zero
   AccessKit code (feature off). Acceptance: `cargo test -p engine --features a11y_native` green on
   native hardware.
