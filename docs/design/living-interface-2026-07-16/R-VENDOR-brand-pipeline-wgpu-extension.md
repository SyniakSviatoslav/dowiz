# R-VENDOR — Vendor brand pipeline, ported to the wgpu/WebGPU render engine

> Status: **architecture research (design, not implementation)**, 2026-07-16. Arc: living-interface.
> Scope: extend the existing "Sea & Sheet" per-tenant brand system (today a CSS-token pipeline on the
> DOM layer) so the **new production wgpu/WebGPU renderer** draws each vendor's brand **deterministically**
> (no AI theming) and **cannot mismatch** the DOM layer. Companion reads: `dowiz-interfaces/RESEARCH-CONSPECT.md`
> (Sea & Sheet, TOKEN 3 TIERS), `dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md` DZ-02/DZ-09,
> `field-ui-engine/BLUEPRINTS-FIELD-UI.md` FE-05/FE-06/FE-09, `physics-ui-capture-blueprint.md` §2/§4,
> `internal-retrieval-living-memory-blueprint.md`.
>
> This document **confirms and extends** decisions already made in the corpus. It does **not** redesign
> Sea & Sheet or the token 3-tier model — those are locked. It answers one new question: *how do the 5
> vendor-owned tokens reach the GPU without drifting from the DOM, and where does the brand boundary sit
> on the GPU?* Every claim is grounded against on-disk source; citations are `file:line` or section names.

---

## 1. Current-state grounding (what already exists, cited precisely)

### 1.1 The token 3-tier model (LOCKED — do not redesign)

`RESEARCH-CONSPECT.md` "TOKEN 3 TIERS" (line 69) and `BLUEPRINTS-DOWIZ-INTERFACES.md` DZ-02 (lines 69-90):

- **T1 BRAND-OWNED — exactly 5 inputs the owner touches** (Hick's-law-limited, "НЕ per-component overrides"):
  1. accent — `--brand-primary` (+ auto `-hover`/`-strong`/`-light` derived, AA-validated)
  2. ink — `--brand-text` / `--brand-text-muted` (AA-validated pair)
  3. paper — `--brand-bg` / `--brand-surface` / `--brand-surface-raised`
  4. type-pair — `--brand-font-heading` / `--brand-font-body` (2 max: serif + rounded-sans)
  5. radius — `--brand-radius` (sheet-rise 26px derives)
- **T2 DOWIZ-FIXED — the Sea + grid, never overridable**: `--spectral` (oklch gradient + speeds),
  `--sea-backdrop` = `color-mix(brand-bg 12% into #060402)`, `--sea-tint` = `brand-primary`,
  `--field-c/gamma/mass` physics params, `--font-mono` + `--money-ink`/`--price-red #C21A1F`
  (hue-only brand shift, **role-locked**), `--ease-snap`/`--ease-tide`, `--space`/`--text`/`--tap`/
  `--status` (10×4). "DOWIZ-FIXED — lifecycle identical every brand."
- **T3 DOWIZ INTERNAL BRAND — the product's own chrome** (marketing/login/settings/owner-tool-frame):
  Warm Cosmo-Noir (`#d69a3d` / `#061b1a` / `#f5efe5`, `[data-skin=paper]`).

The load-bearing property (`RESEARCH-CONSPECT.md` CROSS-BRAND, line 77): **"Coherence BY CONSTRUCTION
— not enough brand-authorable surface to diverge."** Two brands share ~95% (identical Sea physics, law,
ζ-motion, components, scales, status, money, spectral identity); they differ **only in the 5 T1 tokens.**
One accent → 4 placements (Sheet direct / Sea tint / spectral re-derive / backdrop 12%) "coherent zero
manual" (line 69).

### 1.2 The two-layer grammar (LOCKED)

`RESEARCH-CONSPECT.md` TWO-LAYER MODEL (line 67), DZ-01:

- **LAYER A — DOWIZ AMBIENT (Sea / Field)**: dowiz-owned, **brand-TINTED never brand-authored**. Rendered
  by the field-UI engine (`MÜ+ΓU̇+c²LU=S`). Carries arrival, transitions, tracking, feedback, focus.
- **LAYER B — BRAND CONTENT (Sheet / Paper)**: brand-owned, tenant-themed. **Opaque** surface (DZ-01
  "opaque `--brand-surface`") carrying menu/cart/forms/tables/decision + the brand identity.
- **SEAM = SPECTRAL EDGE**: `--spectral` re-derived per brand, the *single* dowiz mark on a brand Sheet.

Assignment rule: ambient/transition/tracking/feedback → Sea; content/word/price/decision → Sheet;
number/money → Sheet, **never moves**.

### 1.3 How theming works **in production today** — it already server-derives

This is the most important and most-overlooked grounding fact. The corpus describes T2 derivation as CSS
`color-mix()` (design language), but the **shipped mechanism** is server-side pre-baking:

`.agents/skills/deliveryos-theme/references/technical-reference.md` §4 "How theme technically works" +
ADR-015/016/020/022 (§19):

| Step | Detail (verbatim from §4) |
|------|---------------------------|
| 1 | Owner saves theme → `PATCH /api/v1/location/:id/theme` → **Zod validation** → save to `location_themes` |
| 2 | **BullMQ job → generates a static CSS file → uploads to Cloudflare CDN → updates `css_hash`** |
| 3 | SSR `/s/:slug`: Fastify reads location+theme → injects `<link href=/cdn/themes/{id}/{hash}.css>` → **zero DB queries for theme** |
| 4 | Admin + Courier PWA: `GET .../brand` → CSS variables → `document.documentElement.style.setProperty(...)` |
| 5 | `css_hash` changes on every update → Cloudflare serves new file |

- **ADR-014 / §2**: curated presets, **not a full editor** ("20% work = 80% value; GloriaFood confirmed
  no churn"). `presets.json` holds **7 fully-resolved token sets** — every preset lists all 15 tokens
  explicitly, using only ~6 curated fonts (Inter, DM Serif Display, Cormorant Garamond, DM Sans,
  Playfair Display, Yeseva). The presets are *already* the "server pre-resolved" artifact.
- **ADR-015 / §5**: `location_themes` is a **typed table** (`primary_color, accent_color, background_color,
  text_color, font_heading, font_body, border_radius, …`), not a jsonb blob. The **persisted state is the
  5 T1 inputs**; the served artifact is the fully-derived CSS.
- **ADR-020 / §8**: colors chosen from **Radix Colors (WCAG-verified palettes), not free hex**; `brand_config`
  runs a **WCAG contrast check** (Zod + contrast). This is the existing accessibility gate — but it
  validates **flat** ink-on-paper contrast only (see §5c).
- **ADR-022 / §2**: **white-label is client-side ONLY.** "Courier = internal tool. Admin = B2B. Logo +
  primary color shown everywhere via CSS vars." The owner/admin surfaces are dowiz chrome that merely
  *echoes* the accent; they are not vendor storefronts. (Load-bearing for §4.)

**Takeaway:** the production pipeline already resolves T1→(full token set) **once, server-side, at save
time**, and the client consumes a fully-resolved artifact. The DOM never runs the derivation at render
time. This is the model the wgpu port should mirror — not invent a second one.

### 1.4 The engine's stance: CPU-authoritative, deterministic, GPU = display

`physics-ui-capture-blueprint.md` §2 + both blueprint appendices (Invariant #3/#6):

- The `engine/` crate is **pure-CPU, zero-dep**; wgpu is explicitly out of scope of that crate today.
  `engine/src/scene.rs` + `sdf.rs` rasterize an SDF scene into a **flat, bit-deterministic `Vec<f32>`**
  ("two calls with identical inputs yield identical `Vec<f32>` bytes", `scene.rs:120`) fed zero-copy to
  the GPU upload path. **There is no color/token handling in the engine yet** — `SdfShape` carries
  geometry only (`scene.rs:29-44`); fills, brand colors, and the token table do not exist.
- **`engine/src/money_guard.rs`** already realizes FE-09: `Money(i64)` deliberately does **not** implement
  the `FieldValue` trait, so `interpolate(Money, …)` / `Spring<Money>` is a **compile error**
  (`money_guard.rs:19-23, 92-95`); `TweenGuard::present_money` is the runtime mirror. Money-never-tween is
  enforced by construction and must survive the port unchanged.
- **Determinism invariant** (BLUEPRINTS-FIELD-UI Appendix B #3): "authoritative compute CPU-side (WASM
  f64→f32); GPU = display; scalar == SIMD bit-identical." Color derivation is *authoritative compute* — by
  this invariant it belongs CPU-side, not in a fragment shader.

### 1.5 FE-05 already answers the core question

`BLUEPRINTS-FIELD-UI.md` FE-05 "SDF shape pipeline + design-token GPU table" (lines 169-193) is the piece
the task expected to be load-bearing, and it is. Its TARGET STATE already says, verbatim:

> "GPU token table: color-mix **pre-resolved CPU** → concrete RGBA (recompute on tenant switch); …
> Bind0 UBO {screen,dpr,time,**theme_tokens**} (theme switch = **1 uniform write**)."

So FE-05 **already chose CPU-side pre-resolution** and a single-uniform-write theme swap. The GPU never
runs `color-mix()`. This research confirms that choice, ties it to the production server-bake model (§1.3),
and specifies the anti-drift contract FE-05 left implicit.

---

## 2. The wgpu port design — server-derives vs client-derives

### 2.1 Decision: **SERVER-DERIVES (canonical). The client renders from a fully-resolved token table it is handed. One Rust `resolve()` is the single source; it is the ONLY implementation of the transform.**

This is option (a) from the task framing, hardened so the "second implementation" risk it warns about is
eliminated *by construction*, and reconciled with the existing production model (§1.3) and FE-05 (§1.5).

**The mechanism (one canonical function, two serializations):**

```
                       ┌─────────────────────────────────────────────┐
  owner saves 5 T1  ─► │  brand::resolve(T1) -> ResolvedTokens        │  ← THE ONLY derivation
  (location_themes)    │  pure Rust, f64→f32, deterministic, no deps  │     (native + wasm target)
                       └───────────────┬─────────────────────────────┘
                                       │ emits, at save time:
              ┌────────────────────────┼─────────────────────────────┐
              ▼                        ▼                              ▼
   (i) CSS file: RESOLVED    (ii) GPU token table: the SAME     (iii) css_hash / token_hash
       literals only —           values as LINEAR-RGBA f32          (cache key, cross-check,
       NO live color-mix()        (+ oklch for the Sea/spectral)     drift tripwire)
       → served to DOM Sheet      → served to wgpu Sea/Sheet
```

- **`resolve(T1) -> ResolvedTokens`** lives in one Rust module (proposed: a small `brand-resolve` crate
  reused by both the server bake job and, wasm-compiled, the client — mirroring how `field-math` (RW-01)
  is vendored). It computes every derived value **once**: `--brand-primary-hover/-strong/-light`, ink AA
  adjustments, `--sea-tint`, `--sea-backdrop` (the `color-mix(brand-bg 12%, #060402)`), the `--spectral`
  stop list (oklch terracotta→accent→gold), status/money role-locked tokens (pass-through), radius-derived
  sheet geometry. This is the same "5 inputs → many derived values" transform DZ-02 describes; the point is
  it exists **exactly once**.
- **Serialization (i)** is the existing CDN CSS file (§1.3) — but with one new hard rule (§5a): it must
  contain **resolved literals** (`--sea-tint: #c8502e;`), never live `color-mix()` calls. The browser's
  color-mix engine is then *never in the derivation loop*, so it cannot disagree with the GPU.
- **Serialization (ii)** is new: the same `ResolvedTokens`, laid out as a flat `f32` buffer of **linear-light
  RGBA** (plus a few oklch triples the Sea/spectral shaders want), sized to fit FE-05's Bind0 UBO
  `theme_tokens` field. This is what `queue.writeBuffer` uploads. It ships alongside the CSS under the same
  `css_hash` so DOM and GPU are versioned together.
- The client **never derives at render time.** It fetches the resolved table (same `GET .../brand`
  endpoint, extended to return the numeric table or a URL to it) and does one `writeBuffer` into the token
  UBO. Theme switch = 1 uniform write (FE-05). No per-frame math, no WGSL color-mix.

### 2.2 Why this beats pure client-derivation (option b)

The task's option (b) — client re-derives T2 from T1 in WGSL/wasm — is rejected as the *canonical* path for
four grounded reasons:

1. **It creates the exact drift the operator fears.** A WGSL/Rust color-mix on the client and the CSS
   `color-mix()` on the DOM are *two implementations* of the same transform. oklch↔linear↔sRGB conversions,
   rounding, and gamma handling differ between a browser's CSS engine and a hand-written shader. Result:
   "the DOM-layer Sheet and the GPU-layer Sea rendering different colors for the same brand" — the precise
   failure mode named in the task. Server-derive-once makes this **unrepresentable**: there is one number,
   served to both.
2. **The determinism invariant already forbids it.** "Authoritative compute CPU-side; GPU = display"
   (§1.4). Deriving brand colors is authoritative compute. Putting it in a fragment shader violates a
   locked engine invariant.
3. **FE-05 already chose CPU pre-resolution** (§1.5). Client-derivation would contradict a locked blueprint.
4. **The production model already server-bakes** (§1.3). Client-derivation would be a *new second model*
   sitting beside the CDN CSS generator — two ways to theme, guaranteed to diverge over time.

### 2.3 What the single canonical function buys us for free (offline / live-preview)

Because `resolve()` compiles to **both** native (server) and wasm (client), the client *can* run it — but
**only** for the one case where the server has not yet baked: the branding editor's **live preview before
save**. Today that preview is a DOM postMessage into an iframe (DZ-09, technical-reference §4 step 4). On
the GPU, the same wasm `resolve()` feeds the same token UBO, so **the preview is bit-identical to the
artifact the server will bake** — because it is the same code, not a second implementation. This is the
"self-sufficient offline" benefit of option (b) captured *without* the drift cost:

- **Canonical render path** = consume server-resolved table (option a). Simple client, no drift.
- **Preview / offline path** = same wasm `resolve()`. Cannot diverge from the eventual bake.

This also satisfies the local-first invariant (render loop never needs the server): the resolved table is
cached client-side like any other menu/scene state, and `resolve()` is available offline for a first-run
preview if the CDN artifact hasn't arrived.

### 2.4 Colour-space correctness (a concrete correctness item, not optional)

CSS operates in sRGB/oklch *display* space. GPU **bloom and additive blending must happen in LINEAR light**
(external research §2: "emissive HDR (>1.0) … additive blending … tone mapping *last*"). If the resolved
CSS hex is naively reinterpreted as linear on the GPU, the Sea tint renders visibly wrong (too dark/bright)
relative to the DOM — a *different* drift than derivation drift. Therefore `resolve()` must emit the GPU
table in **linear RGBA** (sRGB→linear applied once, CPU-side, deterministically), and the tone-map/bloom
pipeline is T2-fixed so every brand gets the same luminance response. The oklch values for `--spectral` and
`--sea-tint` travel as oklch (the Sea/spectral shaders interpolate in oklch by design) and are likewise
resolved once.

### 2.5 What the GPU actually needs from a brand: only scalars

The wgpu Sea consumes, per brand, **only** the resolved numeric tokens: a handful of RGBA/oklch colors, two
font *identifiers* (not glyphs — see §3.3), and one radius scalar. That is the entire brand surface the
field engine ever sees. It never needs arbitrary vendor imagery (§3). This keeps the token UBO tiny (a
Bind0 field, FE-05) and the theme swap a single write.

---

## 3. Sea / Sheet content boundary confirmation (Q2)

### 3.1 The boundary holds: Sea = scalar tokens only; vendor content lives on the Sheet

`RESEARCH-CONSPECT.md` LAYER A is explicit: the Sea is **"brand-TINTED never brand-authored."** The field
physics is identical for every vendor; the *only* per-vendor inputs are the resolved scalar tokens (§2.5).
**The wgpu field engine never consumes vendor imagery, logos, or arbitrary art.** Confirmed against the
per-screen master checklists (DZ-07/08/09): no Sea-layer feature requires vendor raster/vector content.

### 3.2 The vendor LOGO is Sheet content (the one raster asset), and stays out of the field

The logo is the only vendor-supplied non-scalar asset in the whole system (branding: "logo upload ≤2MB
png/jpeg/svg", DZ-09; storefront hero + checkout + admin chrome show it). Its placement:

- **On the DOM/2D Sheet** (the default two-layer split, DZ-01): the logo is an `<img>` on the opaque Sheet.
  The GPU Sea is untouched. This is the current and recommended arrangement.
- **If/when the Sheet itself goes GPU** (the pure-no-DOM direction in `physics-ui-capture-blueprint.md` §4):
  the logo becomes a **single textured quad on the Sheet layer** — one `wgpu::Texture` uploaded from the
  rasterized logo — composited over, but never fed *into*, the field physics. It is still Sheet content; the
  Sea remains scalar-only. SVG logos rasterize to a texture at upload time (CPU, deterministic), not parsed
  in-shader.

**Flag (real boundary caveat):** the logo is the single point where a vendor raster asset can reach GPU
memory, and *only* on the Sheet layer. The invariant to hold: **a vendor texture may live on the Sheet quad;
it may never become a source `S` in `MÜ+ΓU̇+c²LU=S`.** The field stays vendor-content-free; the Sheet may
carry exactly one vendor texture (the logo). No DZ-07/08/09 screen violates this.

### 3.3 Fonts are a real new constraint the wgpu port imposes (flag)

T1 includes a type-pair. On the DOM this is a webfont (`--brand-font-heading/body`, Google Fonts). On the
GPU, FE-06 (MSDF text) needs the font as a **parsed outline / pre-baked MSDF atlas**, covering the vendor's
glyph set (Latin + Albanian ë/ç + Cyrillic uk + tabular-nums). Consequences:

- A vendor **cannot** pick an arbitrary free-text font for the GPU path: each font needs a server-baked
  atlas before it can render. This is *stricter* than the CSS version.
- This is already consistent with **ADR-014 (curated presets, not a full editor)** and `presets.json`
  (only ~6 fonts in use). **Recommendation:** T1 fonts are a **curated set** with pre-baked MSDF atlases,
  not free input. This is a constraint, not a regression — it matches the existing curated-preset stance.
  The atlas bake is a server job analogous to the CSS bake (§1.3).

---

## 4. Living-memory view — tier assignment (Q3)

**Verdict: T3 (dowiz-internal chrome, unbranded). The visualization's marks never read the 5 T1 tokens.**

Note: the R-LM living-memory-visualization doc does not exist on disk yet
(`living-interface-2026-07-16/` holds only `EXTERNAL-RESEARCH-gpu-neural-field-sonification.md`). The
relevant existing design is `internal-retrieval-living-memory-blueprint.md` (living-memory = DB→pgrust,
personalized-PageRank recall). The argument below is grounded on that + the access/branding rules.

**Why T3, concretely:**

1. **It renders dowiz's own operational/introspection data, not vendor content.** The memory graph,
   PageRank recall, and order-processing traces are dowiz IP (the "moat" Sea-side, RESEARCH-CONSPECT). The
   Sheet/content rule is "Sheet wears the **content-owner's** brand" — and the content owner here is dowiz,
   not the vendor. So even the content layer stays T3.
2. **ADR-022 settles it: white-label is client-side (customer storefront) ONLY.** Owner/admin surfaces are
   "internal tool / B2B" that merely echo the accent as chrome; they are not brand storefronts. A diagnostic
   introspection view is the *most* internal of internal tools. Tinting the diagnostic *marks* terracotta
   would be a category error — it implies the vendor authored/owns dowiz's cognition surface.
3. **Cross-tenant operability requires a fixed palette.** An operator debugging 50 vendors must see the
   *same* viz palette every time; node/edge/heat colors that shift per vendor would make comparison and
   anomaly-spotting harder — an operational cost with no product benefit.
4. **Bloom + arbitrary vendor accent = legibility risk on a data view** (§5c). Diagnostic marks must stay
   legible; binding them to an unvalidated vendor color under emissive bloom risks washing out signal. The
   fixed T2/T3 palette (status colors + spectral + cosmo-noir) is contrast-controlled by dowiz.

**The one refinement (owner sees their own slice):** the task asks whether an owner viewing *their own*
orders' processing trace warrants brand-tinting. Answer: **the diagnostic marks stay T3; only the ambient
Sea the view floats over may carry the owner's tint** — and that is already true of *every* owner screen
(DZ-09: "venue accent tints Sea"). So an owner-facing living-memory diagnostic, rendered inside the owner
Shell, sits over a `--sea-tint` backdrop like all owner screens, but the **visualization itself** (graph
nodes, edges, glow, neural-field marks) reads only T2/T3, never T1. This is exactly "Sea tints active venue,
content stays with its owner (= dowiz here)." Net: **T3 for the viz; ambient Sea tint inherited from the
owner Shell is fine and requires no special rule.**

---

## 5. Friction / joint-risk map

### (a) ★ MOST IMPORTANT — token DRIFT between the DOM/CSS Sheet and the GPU/WGSL Sea

**Risk:** two independent implementations of "derive T2 from T1" (browser `color-mix()` vs a shader/Rust
color mix) drift over time → DOM Sheet and GPU Sea show *different colors for the same brand*. This is the
central sync-error the operator raised.

**Prevention (drift-impossible-by-construction), from §2:**
1. **One canonical Rust `resolve(T1)`** — the *only* implementation of the transform, native + wasm.
2. **The served CSS carries RESOLVED literals, never live `color-mix()`.** The browser color engine is
   removed from the derivation loop entirely. (Concrete rule; easy to lint: grep the generated CSS for
   `color-mix(` → must be **0** in T2/T1 output.)
3. **DOM and GPU read the SAME resolved values** under the same `css_hash`/`token_hash`. GPU gets linear
   RGBA emitted by the same function.
4. **`token_hash` as a runtime tripwire:** ship a cheap dev/CI cross-check that re-derives from the stored
   T1 and asserts the served CSS literals and the GPU table agree bit-for-bit (mirrors the engine's existing
   `scalar == SIMD bit-identical` determinism test). If DOM≠GPU, the hash mismatches → fail the bake.

This converts the friction from "a class of bugs we hope not to hit" into "a single function that cannot be
bypassed." It is the same pattern the money guard uses (one boundary, enforced two ways) and the same
pattern ADR-016 already uses for CSS (bake once, serve static).

### (b) Owner changes brand mid-session while a customer has a live wgpu scene

**Question:** live token-update push vs page-reload-to-apply?

**Answer: live push — and it is cheap, because FE-05 already made a theme swap = 1 uniform write.**
- The existing branding editor *already* does live preview with **no reload** (postMessage, DZ-09,
  technical-reference §4). A reload-only wgpu path would be a **regression** against shipped behavior and
  against the operator's "reactive to change" requirement.
- Mechanism: a `brand_config updated` event over the existing transport (WS today; the bebop
  `Transport::recv()` stream later) → client fetches the new resolved table → one `writeBuffer` into the
  token UBO + re-derive spectral. No new streaming complexity beyond a small event; the payload is one small
  buffer, not a scene.
- **Do it smoothly via existing motion, not a hard cut:** apply the swap through the already-specified
  **SPREAD / heat-kernel theme-swap** ("items DIFFUSE from tap … theme-swap", RESEARCH-CONSPECT MOTION ζ
  VOCAB). Colors migrate over ζ-motion; no jarring flash mid-transaction.
- **Money is exempt and safe:** `--money-ink`/`--price-red` are T2 role-locked (hue-only), and money never
  tweens (money_guard). A brand edit does not change the money value; only its ink hue could shift, and that
  is a color swap, not a value interpolation — the guard is untouched.
- **Fallback:** embed mode / no-WS / offline → apply on next navigation or reload. Acceptable because those
  contexts are already non-live (technical-reference §13 embed: no WS).

### (c) Accessibility — bloom can wash out contrast that flat CSS never does

**Risk:** the existing WCAG check (§1.3, ADR-020) validates **flat** ink-on-paper contrast. The GPU Sea
adds **emissive HDR + selective bloom** (external research §2). Bloom raises effective luminance of the
tinted field and can wash out text drawn *on* the Sea in ways a flat CSS color cannot. An arbitrary 5-token
vendor brand could pass flat AA yet be **inaccessible once bloomed**.

**Scope the risk precisely (it is smaller than it first looks):**
- The Sheet is **opaque** (`--brand-surface`, DZ-01). Text on the Sheet sits on opaque paper, *not* on the
  Sea; bloom on the Sea does not bleed under opaque Sheet text. Flat AA already covers this.
- The real exposure is **text/marks drawn directly on the Sea**: the Act-1 hero headline over the full-bleed
  Sea, tracking step-pills, spectral-edge-adjacent text.

**Recommended new gate (a brand-config go-live gate the CSS-only design didn't need):**
- Add a **bloom-aware contrast validation**, scoped to on-Sea text, that checks ink contrast against the
  **post-bloom composited backdrop luminance at the brand's max Sea energy/amplitude** — not the flat
  `--brand-bg`. Because the Sea backdrop is `color-mix(brand-bg 12%, #060402)` (a very dark, low-tint
  backdrop) and bloom is T2-fixed, this is computable deterministically from the resolved tokens + the fixed
  bloom curve at brand-save time. Fail → brand cannot go live (or on-Sea text falls back to a guaranteed-AA
  ink). This extends the existing Zod+WCAG gate rather than replacing it.
- This gate runs in the same server bake job as `resolve()` — same place, same determinism.

### (d) Other joint risks found

- **Colour-space mismatch (§2.4):** sRGB/oklch (CSS) vs required linear-light (GPU bloom/blend). Folded into
  `resolve()` emitting linear RGBA once. Flagged because a naive "reuse the hex on the GPU" would ship a
  subtle, brand-wide wrongness.
- **Font atlas coverage (§3.3):** a curated-font requirement the wgpu path imposes; free-text fonts break the
  GPU text path until an atlas is baked. Constrain T1 fonts to a curated set (already the ADR-014 stance).
- **"Auto-generate brand" must terminate in the 5 T1 inputs (operator constraint #2 — no AI theming).** The
  branding master checklist has "auto-generate brand from website/logo → POST /owner/brand/generate"
  (DZ-09). This is compatible **only** if it *proposes* the 5 T1 values for the owner to accept/edit, then
  flows through the deterministic `resolve()` — never a per-component AI theme. Flag: keep this feature an
  *input-assist* to the 5 tokens, not a theming engine. Anything else violates "deterministic
  token-derivation only."
- **`presets.json` is fully-resolved but `resolve()` must still own presets.** Today presets ship as
  complete token sets. To keep one source of truth, presets should be defined as **5 T1 inputs** and run
  through the same `resolve()` (with the resolved set cached), so a change to derivation logic updates
  presets and custom brands identically. Otherwise presets become a *third* resolution path that can drift
  from both DOM and GPU.

---

## 6. Phase-0 recommendation

The rendering math is proven; the branding port is a **discipline** problem (one derivation, versioned
artifacts), not a research problem. Smallest set of moves that makes the wgpu path brand-correct and
drift-proof, ordered by dependency:

- **P0-1 — Extract the single canonical `resolve(T1) -> ResolvedTokens`** into a small zero-dep Rust crate
  (`brand-resolve`, native + wasm32-clean, like `field-math`). It **absorbs** the existing derivation
  (the branding editor's 3→10 / preset expansion) so there is exactly one implementation. Emits (i) CSS
  literals, (ii) linear-RGBA + oklch GPU table, (iii) `token_hash`. Gate: same T1 → bit-identical outputs
  (native == wasm), and re-deriving a preset's 5 inputs reproduces `presets.json` exactly.
- **P0-2 — Serve the GPU token table beside the CSS**, under the same `css_hash`, from the existing bake job
  (extend the BullMQ css-generator, ADR-016). Gate: generated CSS contains **0** `color-mix(` for T1/T2
  output; DOM literals and GPU-table values agree (the `token_hash` cross-check).
- **P0-3 — Wire the GPU token table into FE-05's Bind0 UBO** (`theme_tokens`), theme swap = 1 `writeBuffer`.
  Gate (FE-05's own): render a Storefront card vs the CSS version, pixel-diff < threshold; theme switch =
  1 uniform write, not a re-tessellation.
- **P0-4 — Add the bloom-aware contrast gate** (§5c) to the brand-save path, extending the existing Zod+WCAG
  check. Gate: a brand that passes flat AA but fails post-bloom on-Sea contrast is **rejected** (RED→GREEN).
- **P0-5 — Curate the T1 font set** (§3.3) and pre-bake MSDF atlases per curated font (Latin+ë/ç+Cyrillic+
  tnum) in the same bake pipeline. Gate: every curated font renders crisp at 3 scales in sq/en/uk (FE-06's
  gate).
- **P0-6 — Live brand-update push** (§5b) as a small transport event → refetch table → SPREAD/heat-kernel
  swap. Gate: owner edits accent mid-session → the live customer Sea migrates over ζ-motion, money unchanged,
  no reload; embed/offline falls back to next-nav.
- **Living-memory viz (P-later):** implement as **T3** (§4) — marks read only T2/T3; if hosted in the owner
  Shell it inherits the Shell's ambient `--sea-tint`, no special rule.

**Invariants this design must not break** (all pre-existing): money never a field value / never tweens
(money_guard); authoritative compute CPU-side, GPU = display; one Intent/one derivation path; coherence by
construction (owner touches only 5 T1 tokens); Sea brand-tinted-never-authored; the field never carries a
vendor texture (Sheet may carry the logo only).

---

### Appendix — source map (grounded on-disk, 2026-07-16)

| Claim | Source |
|-------|--------|
| Token 3 tiers; owner touches 5 T1; coherence by construction | `dowiz-interfaces/RESEARCH-CONSPECT.md` lines 67-77; `BLUEPRINTS-DOWIZ-INTERFACES.md` DZ-02 |
| FE-05: color-mix pre-resolved CPU → RGBA; theme switch = 1 uniform write | `field-ui-engine/BLUEPRINTS-FIELD-UI.md` lines 169-193 |
| Money never tween — compile-time `FieldValue` exclusion + runtime guard | `engine/src/money_guard.rs:19-23, 55-68, 92-95` |
| Engine CPU-authoritative, deterministic Vec<f32>, no color yet | `engine/src/scene.rs:120-138`, `engine/src/sdf.rs`; Invariant #3 |
| Production server-bakes theme CSS → CDN; typed `location_themes`; curated presets; WCAG check; white-label client-side only | `.agents/skills/deliveryos-theme/references/technical-reference.md` §4/§5/§8, ADR-014/015/016/020/022 |
| Presets are fully-resolved sets, ~6 curated fonts | `.agents/skills/deliveryos-theme/resources/presets.json`, `tokens.css` |
| Pure-no-DOM direction; AccessKit; hidden input; wgpu sole graphics dep | `physics-ui-capture-blueprint.md` §3/§4 |
| Bloom/emissive HDR aesthetic; WebGPU+WebGL2; contrast/bloom concerns | `living-interface-2026-07-16/EXTERNAL-RESEARCH-gpu-neural-field-sonification.md` §2 |
| Living-memory = DB→pgrust, personalized-PageRank recall (introspection) | `internal-retrieval-living-memory-blueprint.md` |

*End R-VENDOR. Architecture research only — no code written or edited.*
