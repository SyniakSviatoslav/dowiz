# BLUEPRINT-P01 — Vendor brand-token pipeline (one canonical `resolve()`, DOM+GPU, drift-proof)

> **Status:** execution-ready blueprint (design detail, not implementation). Arc: living-interface.
> Date: 2026-07-16. **Planning only — this document writes/edits no product code, CI, or canon.**
>
> **Scope:** one continuous piece of work that lands across three roadmap phases —
> **Phase 1** (Brand Token Source of Truth, Wave 0), the brand slice of **Phase 3** (GPU wiring),
> and the brand slice of **Phase 4** (contrast gate + live push). It turns R-VENDOR's already-designed
> recommendations (P0-1…P0-6) into named crates, files, wire formats, and migration steps.
>
> **Primary source (already-designed):** `R-VENDOR-brand-pipeline-wgpu-extension.md`.
> **Roadmap slotting:** `LIVING-INTERFACE-ROADMAP.md` §4 (Phase 1/3/4 rows) + §5 (J4/J5/J8).
> **Not re-litigated (locked prerequisites, cited by ID):** the token 3-tier model (DZ-02), Sea/Sheet
> two-layer grammar (DZ-01), FE-05 (design-token GPU table), FE-06 (MSDF text), FE-09/`money_guard`.

---

## 1. Current-state evidence (what exists, cited precisely)

**The 3-tier token model (DZ-02 / `RESEARCH-CONSPECT.md:67-77`, LOCKED).** A vendor owns exactly **5 T1
inputs** — accent, ink, paper, type-pair, radius — Hick's-law-limited, "не per-component overrides." **T2
DOWIZ-FIXED** (the Sea physics, `--spectral`, `--sea-tint`, `--sea-backdrop = color-mix(brand-bg 12%,
#060402)`, `--money-ink`/`--price-red #C21A1F` role-locked hue-only, `--ease-*`, scales/status) is never
overridable. **T3 DOWIZ-INTERNAL** is the product's own chrome (Warm Cosmo-Noir, `[data-skin=paper]`). The
load-bearing property is **"coherence by construction — not enough brand-authorable surface to diverge"**;
two brands share ~95%, differing only in the 5 T1 tokens.

**The persisted state is the 5 inputs; the served artifact is fully derived.** `location_themes`
(`technical-reference.md:90-102`, ADR-015) is a **typed table**, not a jsonb blob:
`primary_color, accent_color, background_color, text_color, font_heading, font_body, border_radius, …,
css_hash, css_generated_at`. `presets.json` (`.agents/skills/deliveryos-theme/resources/presets.json`)
today ships **fully-resolved 15-token sets** keyed by preset name (`--brand-primary`, `-hover`, `-light`,
`--brand-accent`, `--brand-bg`, `--brand-surface`, `--brand-surface-raised`, `--brand-text`,
`--brand-text-muted`, `--brand-border`, `--brand-font-heading`, `--brand-font-body`, `--brand-radius`,
`--brand-radius-sm`, `--brand-radius-btn`), using ~6 curated fonts (Inter, DM Serif Display, DM Sans,
Cormorant Garamond, Playfair Display, Yeseva).

**The production bake pipeline already exists — ADR-016, ground everything in it, invent no parallel path.**
`technical-reference.md:415` (ADR-016): *"CSS → static file → CDN. SSR links `/cdn/themes/{id}/{hash}.css`.
Zero DB queries for theme on render."* The mechanism (`technical-reference.md:74-78`, §4):

1. Owner saves → `PATCH /api/v1/location/:id/theme` → **Zod validation** (`.strict()` on `brand_config`,
   ADR-020 Radix/WCAG check) → save to `location_themes`.
2. **BullMQ job → generates a static CSS file → uploads to Cloudflare CDN → updates `css_hash`.**
3. SSR `/s/:slug`: Fastify reads location+theme → injects `<link rel=stylesheet
   href=/cdn/themes/{id}/{hash}.css>` → **zero DB queries for theme**.
4. Admin/Courier PWA: `GET /api/v1/location/:id/brand` → CSS variables →
   `document.documentElement.style.setProperty(...)`.
5. `css_hash` changes on every update → Cloudflare serves the new file.

The task's Phase-1 crate **slots into step 2**: the BullMQ CSS generator becomes a thin caller of the new
`resolve()` and additionally emits the GPU table (§3). ADR-022 confirms **white-label is client-side only**
(owner/admin surfaces merely echo the accent as chrome), which is why the living-memory viz stays T3 (§4
boundary note).

**FE-05 already decided CPU pre-resolution — the exact line (`BLUEPRINTS-FIELD-UI.md:169-193`):**

> "GPU token table: color-mix **pre-resolved CPU** → concrete RGBA (recompute on tenant switch); …
> Bind0 UBO {screen, dpr, time, **theme_tokens**} (theme switch = **1 uniform write**)."

So the GPU never runs `color-mix()`; theme swap is one `writeBuffer`. This blueprint supplies the anti-drift
contract FE-05 left implicit.

**The engine is CPU-authoritative and has no color yet.** `engine/` (`dowiz-engine`) is **zero-dep by
mandate**; its `[features] gpu = []` is an **empty stub by design** — `bridge.rs::gpu::new_gpu` returns
`Err("gpu adapter not built — wgpu uncached")` until the one-time `cargo add wgpu` (roadmap Phase 0). `SdfShape`
(`scene.rs:29-44`) carries **geometry only** — no fills, no brand color, no token table exist yet.
`money_guard.rs:15-23` makes `Money(i64)` deliberately **not** implement `FieldValue`, so
`Spring<Money>`/`interpolate(Money, …)` is a **compile error**; this must survive the port unchanged.

---

## 2. The `resolve()` crate design (Phase 1 — R-VENDOR P0-1/P0-2)

**New crate: `/root/dowiz/brand-resolve/`** — a sibling directory of `engine/`, `kernel/`,
`agent-governance-wasm/`, `wasm/` (the repo has no root workspace; each crate is a peer directory).
Package `dowiz-brand-resolve`. This is the vendored-style zero-dep pattern the future `field-math` (RW-01)
will also use — a small deterministic crate compiled **native (server bake) + wasm32 (client preview)**.

```
brand-resolve/
  Cargo.toml        # crate-type = ["rlib", "cdylib"]; NO external deps in default build;
                    # [features] wasm = ["dep:wasm-bindgen"]  (JS glue ONLY, like kernel's wasm feature)
  src/lib.rs        # pub fn resolve(&T1Inputs) -> ResolvedTokens; re-exports
  src/input.rs      # T1Inputs — the 5 owner-touched tiers, 1:1 with location_themes columns
  src/resolved.rs   # ResolvedTokens — every derived value, canonical field order; canonical_bytes()
  src/css.rs        # ResolvedTokens::to_css() -> String   (resolved literals; NEVER color-mix()
  src/gpu.rs        # ResolvedTokens::to_gpu_table() -> [u8; 288]  (linear-RGBA + oklch; §3/§5)
  src/hash.rs       # token_hash([u8;32]) over canonical_bytes(); vendored SHA-256 (~90 LOC, no dep)
  src/presets.rs    # the 7 presets defined as 5-INPUT sets (not resolved token sets); fixture
  src/color.rs      # srgb<->linear (§5), oklch<->srgb; f64 math, f32 emit; deterministic
```

**Zero-dep, deterministic, native==wasm bit-identical.** No external crates in the default build (matches
`engine`); the only `wasm` feature dep is `wasm-bindgen` glue (matches `kernel`). All color math is
`f64`-internal → `f32`-emit, scalar-only, so native and wasm produce **bit-identical** bytes — the same
`scalar == SIMD bit-identical` determinism law the engine already tests.

**`T1Inputs` (the only owner-authorable surface).** A `#[repr(C)]` struct mapping 1:1 to `location_themes`
columns: `accent: Srgb8`, `ink: Srgb8`, `ink_muted: Srgb8`, `paper: Srgb8` (bg), `surface: Srgb8`,
`surface_raised: Srgb8`, `font_heading: FontId`, `font_body: FontId`, `radius_px: u16`. `FontId` is a
`u16` index into a **curated font enum** (§3.3), never a free string — this is what structurally forbids
"auto-generate brand" from proposing anything but valid inputs.

**`resolve()` is the ONLY implementation of the transform.** It computes, once, every derived value DZ-02
describes: `--brand-primary-hover/-strong/-light`, ink AA adjustments, `--sea-tint` (= accent), `--sea-backdrop`
(= `mix(paper 12%, #060402)`, computed in oklch not browser color-mix), the `--spectral` oklch stop list
(terracotta→accent→gold), radius-derived sheet geometry (sheet-rise 26px, `-sm`, `-btn`), and pass-through
of the T2 role-locked `--money-ink`/`--price-red`. It **absorbs the branding editor's current 3→10/preset
expansion** so exactly one derivation exists.

**Two serializations + one hash from one `ResolvedTokens`:**
- **(i) `to_css()`** → the existing CDN CSS file, but with the **hard new rule: resolved literals only,
  zero `color-mix(`** (`--sea-tint: #c8502e;`, not `--sea-tint: color-mix(...)`). The browser color engine
  leaves the derivation loop entirely, so it can never disagree with the GPU.
- **(ii) `to_gpu_table()`** → a fixed 288-byte binary (32-byte header + 256-byte UBO payload, §3), the SAME
  values as **linear-light RGBA** (plus oklch for Sea/spectral).
- **(iii) `token_hash()`** → 32-byte digest over `ResolvedTokens::canonical_bytes()`, the single authority
  both serializations descend from.

### `token_hash` wire format (the drift tripwire — exact)

`canonical_bytes()` is a fixed-order little-endian byte serialization of `ResolvedTokens`, hashed in the
**display-space source-of-truth** representation (not the derived linear values, so it is stable across GPU
layout changes): each color as an `sRGB u8` quad (RGBA), each oklch stop as 4×`f32` LE `(L,C,H,speed)`, each
radius as `u16` px, each font as `u16 FontId`. `token_hash = SHA-256(canonical_bytes())` — a ~90-LOC
vendored SHA-256 (zero-dep) so the crate stays offline-clean; if the kernel later exposes its
content-id hash primitive, reuse that family for consistency with `event_log` content-ids.

The hash is **stamped into both artifacts**:
- CSS: a leading comment line `/* dowiz-token-hash: <64-hex> */`.
- GPU table: the first 32 bytes of the 288-byte binary are the raw digest (header); the UBO upload uses
  bytes `[32..288]`.

**Cross-check (P0-2 gate, runs in the bake job and in CI):** re-run `resolve(stored_T1)`, recompute
`token_hash`, and assert it equals **both** the hash in the served CSS comment **and** the GPU-table header.
Mismatch ⇒ **the bake fails** (DOM≠GPU is unrepresentable-by-construction, then caught belt-and-braces by
the hash). This is the same "one boundary, enforced two ways" pattern as `money_guard`.

### `presets.json` absorption (no third resolution path)

Today presets are 15-token resolved sets (a *second* implicit derivation). **Migration:** redefine the 7
presets as **5-input `T1Inputs` sets** embedded in `src/presets.rs`, and keep the current `presets.json`
**as a golden fixture**. A crate test re-derives each preset (`resolve(preset.inputs)`) and asserts the
output reproduces the current `presets.json` **byte-for-byte**. After that lands, a derivation-logic change
updates presets and custom brands identically; presets can never drift from DOM or GPU.

### "Auto-generate brand" constraint (operator hard rule — no AI theming)

The branding checklist's `POST /owner/brand/generate` (DZ-09) must **return type `T1Inputs`** — a proposal of
the 5 owner-editable values only — which then flows through the deterministic `resolve()`. Because the
return type is structurally the 5 inputs (font as `FontId` enum, colors as `Srgb8`), it **cannot** emit a
per-component theme. It is an input-assist to the 5 tokens, never a theming engine.

---

## 3. GPU wiring design (Phase 3 — R-VENDOR P0-3/P0-5)

**Consumer: FE-05's Bind0 UBO `theme_tokens`.** The 256-byte payload of `to_gpu_table()` is a fixed
`array<vec4<f32>, 16>` (std140/std430-aligned; 16-byte lanes). A mirrored Rust `#[repr(C, align(16))]`
struct in `brand-resolve/src/gpu.rs` and the WGSL `ThemeTokens` struct share one **golden byte-layout test**.
Slot map (all colors **linear-light RGBA** unless noted oklch):

| vec4 | contents | vec4 | contents |
|---|---|---|---|
| 0 | `sea_tint` (linear) | 8 | `paper` (linear) |
| 1 | `sea_backdrop` (linear) | 9 | `surface` (linear) |
| 2 | `accent` (linear) | 10 | `surface_raised` (linear) |
| 3 | `accent_hover` | 11 | `money_ink` (T2 role-locked) |
| 4 | `accent_strong` | 12 | `price_red` (T2 `#C21A1F`) |
| 5 | `accent_light` | 13 | `spectral_stop_0` (oklch L,C,H,speed) |
| 6 | `ink` (linear) | 14 | `spectral_stop_1` (oklch L,C,H,speed) |
| 7 | `ink_muted` (linear) | 15 | scalars: `radius, radius_sm, radius_btn, bloom_threshold` |

**Theme swap = exactly 1 uniform write.** On tenant switch or live update the client does one
`queue.writeBuffer(theme_tokens_ubo, 0, &table[32..288])`. **No re-tessellation**: because `SdfShape`
geometry is color-free (`scene.rs`), a token change never touches the vertex/index buffers — the token UBO is
read by the fragment stage only. This realizes FE-05's "theme switch = 1 uniform write" verbatim.

**Where the wiring lives.** The upload sink is the engine `gpu` feature (`bridge.rs`, today the empty
`new_gpu` stub), unblocked by the Phase-0 `cargo add wgpu`. To keep `engine`'s **default build zero-dep**,
the engine does **not** compile-depend on `brand-resolve`: the 256-byte table crosses the boundary as opaque
`&[u8]` (produced native/wasm by `brand-resolve`), and the engine just uploads it. The `#[repr(C)]` layout is
kept in sync by the shared golden byte-layout test, not by a crate dependency.

**Fonts / MSDF (P0-5).** T1 fonts are **curated, not free input** — the `FontId` enum is exactly the ~6
fonts already in `presets.json`. For each, a **server bake job (sibling of the CSS bake, ADR-016)**
pre-bakes an **MSDF atlas** covering Latin + Albanian `ë/ç` + Cyrillic `uk` + tabular-nums (FE-06). The
`FontId` in the GPU table selects the atlas; a vendor cannot pick an arbitrary GPU font (stricter than CSS,
consistent with ADR-014 curated presets). This closes R-VENDOR §3.3 — the GPU text path breaks on
un-baked fonts, so free-text fonts are disallowed on the GPU path by construction.

**Boundary invariants preserved (R-VENDOR §3).** The Sea consumes **only scalars** — it never ingests
vendor imagery. The vendor **logo** is the single raster asset and lives on the **Sheet** (an `<img>` on the
opaque Sheet today; one textured quad on a GPU Sheet later) — it may never become a source `S` in
`MÜ+ΓU̇+c²LU=S`.

---

## 4. Contrast-gate + live-push design (Phase 4 — R-VENDOR P0-4/P0-6)

### 4a. Bloom-aware contrast gate (P0-4)

**The gap:** the existing WCAG check (ADR-020, Zod+contrast) validates **flat** ink-on-paper contrast only.
The GPU Sea adds emissive HDR + selective bloom (the Phase-3 net-new bloom pass), which raises effective
backdrop luminance and can wash out text drawn **on the Sea** — a brand can pass flat AA yet be inaccessible
once bloomed.

**Design.** Add `brand-resolve/src/contrast.rs`:
`fn bloom_contrast_gate(&ResolvedTokens) -> Result<(), ContrastReject>`, called in the **same BullMQ bake
job**, right after `resolve()`, extending (not replacing) the Zod+WCAG gate:

1. Take the on-Sea backdrop = `--sea-backdrop` (`mix(paper 12%, #060402)` — already very dark, low-tint).
2. Composite it through the **T2-FIXED bloom curve** (threshold → multi-mip blur → tonemap) at the brand's
   **max Sea energy/amplitude**. Because the bloom curve and Sea physics are T2-fixed (identical every
   brand), the post-bloom backdrop luminance is **deterministically computable** from the resolved tokens +
   the fixed curve at save time — no GPU needed.
3. Check WCAG AA of **on-Sea ink** (ink / ink-muted where they render directly on the Sea) against the
   **post-bloom** luminance, not flat `--brand-bg`.

**Scope precisely (smaller than it looks):** only text **drawn directly on the Sea** — the Act-1 hero
headline over full-bleed Sea, tracking step-pills, spectral-edge-adjacent text. **Sheet text is opaque
paper** — bloom does not bleed under it, and flat AA already covers it. On failure: **reject go-live** (or
fall back on-Sea ink to a guaranteed-AA T2 ink). RED→GREEN: a brand passing flat AA but failing post-bloom
on-Sea contrast is rejected.

### 4b. Live brand-update push (P0-6)

**The requirement:** owner edits accent mid-session → live customer Sea migrates, **no page reload**, money
frozen. A reload-only path would regress the shipped `postMessage` live-preview (DZ-09) and violate the
"reactive to change" requirement.

**Mechanism** (cheap, because theme swap is already 1 uniform write):
1. Bake completes → emit a small `brand_config updated` event over the **existing WS transport** (later the
   bebop `Transport::recv()` stream) carrying the new `css_hash`/`token_hash` (not a scene).
2. Client `GET /api/v1/location/:id/brand` → fetches the new resolved GPU table (§3).
3. **One `writeBuffer`** into the token UBO + re-derive spectral.
4. Apply through the already-specified **SPREAD / heat-kernel theme-swap** over **FE-08 ζ=1 critical
   damping** (monotone, no overshoot) — colors migrate, no hard flash mid-transaction.

**Money stays frozen (🔴 red-line, two-way):** `--money-ink`/`--price-red` are T2 role-locked **hue-only**;
a brand edit can shift the money ink *hue* but the **money value never interpolates** — `money_guard`'s
`Money`-is-not-`FieldValue` compile barrier is untouched (a color swap is not a value tween). **Fallback:**
embed mode / no-WS / offline → apply on next navigation (those contexts are already non-live).

---

## 5. Colour-space handling (linear vs sRGB — the subtle brand-wide bug)

**The failure mode (R-VENDOR §2.4/§5d):** CSS operates in sRGB/oklch **display** space; GPU **bloom and
additive blending must happen in LINEAR light**. If a resolved sRGB hex is naively reinterpreted as linear on
the GPU, the Sea tint renders visibly wrong (too dark/bright) relative to the DOM — a **different** drift
than derivation drift, and a **brand-wide** one.

**Handling (folded into `resolve()`, done once, CPU-side, deterministically):**
- `to_css()` emits **sRGB display literals** (hex) + oklch (for spectral/sea-tint the CSS side interpolates
  in oklch).
- `to_gpu_table()` applies the **standard sRGB→linear EOTF once, per channel** (piecewise: `c/12.92` for
  `c ≤ 0.04045`, else `((c+0.055)/1.055)^2.4`; alpha stays linear), emitting **linear-RGBA f32**. This lives
  in `brand-resolve/src/color.rs`, `f64` math → `f32` emit, so native==wasm bit-identical.
- Spectral stops and `--sea-tint` travel as **oklch** (the Sea/spectral shaders interpolate in oklch by
  design) and are resolved once. The **tone-map/bloom pipeline is T2-fixed**, so every brand gets an
  identical luminance response — the vendor cannot alter the transfer curve, only the input color.

**Why this is a J5 real-device item, not just CI:** linear-vs-sRGB wrongness is exactly the class of bug
that can **pass on Lavapipe (software raster) yet look wrong on a real GPU** (f32 rounding, filtering, blend
order differ). The colour-space correctness item therefore requires the **one manual real-device pass** per
shader-touching change (R-DEV §5.1a), not only the CI golden.

---

## 6. Acceptance criteria (falsifiable, all three phase-slices)

**Phase 1 — token source of truth (P0-1/P0-2):**
1. Same `T1Inputs` ⇒ **bit-identical** `ResolvedTokens` / CSS / GPU-table bytes on **native == wasm32**.
2. Re-deriving each preset's 5 inputs through `resolve()` reproduces the current `presets.json` **byte-for-
   byte** (golden fixture in `src/presets.rs`).
3. `grep 'color-mix('` over generated T1/T2 CSS output = **0**.
4. `token_hash` cross-check: `resolve(stored_T1)` re-derivation matches the hash in the served CSS comment
   **and** the GPU-table header **bit-for-bit**, or the bake **fails**.
5. `POST /owner/brand/generate` returns type `T1Inputs` (5 values); it is structurally incapable of emitting
   a per-component theme (no AI theming).

**Phase 3 — brand on GPU (P0-3/P0-5):**
6. Storefront card rendered on GPU vs the CSS version: **pixel-diff < threshold**.
7. Theme switch = **exactly 1 `writeBuffer`** into the `theme_tokens` UBO — **not** a re-tessellation
   (vertex/index buffers untouched; verified by buffer-write trace).
8. Every curated font renders **crisp at 3 scales in sq/en/uk** from its pre-baked MSDF atlas (FE-06 gate);
   a non-curated `FontId` is rejected before bake.
9. GPU-table layout matches the WGSL `ThemeTokens` struct (shared golden byte-layout test passes).

**Phase 4 — contrast gate + live push (P0-4/P0-6):**
10. A brand that **passes flat WCAG AA but fails post-bloom on-Sea contrast is rejected** (RED→GREEN);
    Sheet-only brands are unaffected (opaque paper).
11. Owner edits accent mid-session → the **live customer Sea migrates** over FE-08 ζ=1 easing, **money value
    unchanged**, **no reload**; embed/offline falls back to next navigation.
12. Money-never-tween survives the port: `Spring<Money>` / `interpolate(Money, …)` remains a **compile
    error** (`money_guard`); a brand edit shifts money-ink hue only, never a money value.

---

## 7. J5 / J8 dependency — CI goldens MUST pin to `token_hash`

This pipeline is also a **prerequisite for stable CI goldens** (roadmap Phase 0/3), per the roadmap's
explicit J5↔J4/J8 cross-reference (§5):

- **J8 (brand source ↔ all consumers):** `resolve()` is the single source for **five** consumers — DOM
  Sheet, GPU Sea, the branding-editor live preview (same wasm `resolve()`, bit-identical to the eventual
  bake), `presets.json`, **and the CI golden**. The CI golden is *also* a consumer and must pin to
  `token_hash`.
- **J5 (CI software-raster ↔ real GPU):** R-DEV's Lavapipe golden asserts a pixel-hash / SSIM. **If the
  brand-token source is not canonical, every `resolve()`-logic change silently invalidates every golden** —
  the golden chases a moving target. Worse, R-VENDOR's `token_hash` tripwire (DOM literals == GPU table) **is
  itself a CI check** running in the same Lavapipe harness R-DEV designs.
- **Sequencing consequence (why Phase 1 precedes Phase 3, reinforcing §4's rule):** **Phase 1
  (`resolve()` + `token_hash`) must land before any Lavapipe golden that includes brand color is locked
  (Phase 3+), and every such golden must record the `token_hash` it was rendered against.** A golden whose
  `token_hash` no longer matches the current `resolve(stored_T1)` is *stale by definition* and must be
  regenerated — turning "silent golden rot" into an explicit, detectable mismatch. This is a **second,
  independent reason** the token-source phase precedes GPU rendering.
- **Colour-space caveat (J5, §5):** the linear-vs-sRGB item can pass on Lavapipe and look wrong on a real
  GPU, so its correctness needs the **real-device pass**, not just the CI golden.

**Boundary note (not this blueprint's build):** the living-memory viz is **T3 / unbranded** (R-VENDOR §4;
converges with R-LM F-3) — its marks never read the 5 T1 tokens; only the ambient Sea it floats over inherits
the owner Shell's `--sea-tint`, which needs no special rule. It is a separate blueprint (roadmap Phase 8) and
is noted here only to fix the boundary: a MESH view of N hubs renders peers in the viewing hub's own
neutral/spectral palette, never each peer's brand.

---

*End BLUEPRINT-P01. Design detail only — no product code, CI config, or canon edited. Extends R-VENDOR
P0-1…P0-6 and FE-05/FE-06/FE-09/DZ-01/DZ-02 without re-litigating their decided content; grounds Phase 1 in
the existing ADR-016 bake pipeline (extends the BullMQ CSS generator, invents no parallel path).*
