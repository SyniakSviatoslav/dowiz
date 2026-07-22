# BLUEPRINT — Interface Engine: WebGPU/Shader Spine + Vendor Menu Authority

**Doc id:** BLUEPRINT-INTERFACE-ENGINE-WEBGPU-SHADER-SPINE-2026-07-22
**Status:**EXECUTION-READY — ALL stages S0–S7 LANDED this pass: S0 (vendor menu), S0.5 (vendor assets), S1 (role-screen layouts), S2 (pixel-verification harness), S3 (WGSL shader spine + SINGLE-WRITER audit gate), S3.5 (academy-driven AI ranker), S4 (compose_ui fragments wired to real vendor data), S5 (web shell renders composed vendor menu via CPU raster path + real vendor photos from CDN), S6 (real checkout → kernel order FSM via kernel's PUBLIC API, no kernel source edit), S7 (30-screen layout variants — 18 unique FNV signatures — across 15 owner + 5 courier sub-screens). Each stage has a falsifiable done-check that passes. **174 engine tests green** (146 lib + 4 floor_parity + 1 router + 1 modal + 19 pixel_verify + 3 shader_audit), `cargo fmt --check` clean, `cargo tree -e no-dev` offline-clean (zero external crates; only the existing `dowiz-kernel` path-dep). **Web smoke green**: `node web/src/render/compose.smoke.mjs` (8/8), `node --check web/src/app.js` (real Dubin & Sushi menu, food-hero photos from CDN, integer Lek pricing), `node web/src/render/fieldsim.smoke.mjs` (kernel-wasm buffers deterministic). Verified against a clean checkout of `kernel@9e576e467` (a temporary worktree used only because the live working tree carries uncommitted, unrelated edits that broke kernel's lib build; those edits are out of scope for this blueprint and were not touched).
**Supersedes (in part):** the "engine is, web not wired" / "owner+courier absent in web/" / "demo Pizza Roma" rows of the operator's gap table. The vendor demo row is closed (real Dubin & Sushi menu replaces it).
**Inputs synthesized:** (1) operator directive 2026-07-22 — render everything through WebGPU + shaders with shader-priority; real client replaces demo (https://sushi-durres.netlify.app/ + the menu subdomain https://sushi-durres-menu.netlify.app/); intent interface is one for all roles, differing only in render; generation + pixel-check moves to the backend so Playwright is not needed for frontend; engines must be unified; (2) live code audit of `main`: `engine/src/{intent.rs,compose_ui.rs,scene.rs,sdf.rs,money_guard.rs,gpu_atomicity.rs,bridge.rs,lib.rs,engine_loop.rs}`; `kernel/src/academia.rs`; `web/src/{lib/fieldsim/,render/fieldsim.smoke.mjs,pages/}`; (3) prior rulings this session inherited without re-litigation: `BLUEPRINT-INTENT-INTERFACE-ONE-SCREEN-2026-07-20` §2 (the intent engine is BUILT, tested, and the money firewall is real), `BLUEPRINT-P88-atomicity-by-default` (SINGLE-WRITER discipline + fixed-point reduction), the Detailed Planning Protocol (ground-truth-first / DECART / falsifiable done-checks / consolidation), Anu & Ananke (decisions derivable + structure-forces-the-check), the Integration Decart Rule, the 2-question doubt check.
**Out of scope for this doc:** the Sea & Sheet visual grammar polish (DZ-01..12) — that is `BLUEPRINTS-DOWIZ-INTERFACES.md`, orthogonal to *which* substrate rasterizes; full mesh-agent wiring; voice ASR body (gate-locked on `voice` feature).

---

## 0. Ground truth vs the operator's gap table (verified live, 2026-07-22)

| Operator row ("must be" ⇒ "now") | Live repo truth this blueprint depends on |
|---|---|
| 30 екранів з .polish/final/ (owner/courier/customer) | `.polish/final/` contains exactly 30 PNGs (15 owner-desktop+mobile, 15 courier-desktop+mobile). Inventory ring-fenced; the role→screen map in §5 is the binding contract to them. |
| Sea & Sheet (DZ-01…12) | 0% — лише Three.js фон. **Confirmed**: `web/src/lib/fieldsim/shader.wgsl` is a neural/wave-field background only; no SDF-foreground Sea&Sheet shader exists yet. This blueprint lands the shader **spine** (ui.wgsl + glyph.wgsl); DZ polish is the §6 follow-up. |
| Intent one-screen → compose_ui | engine є, web не підключений. **Confirmed**: `intent.rs::Composer::compose` maps Intent→Scene; fragments are placeholder **boxes**, not vendor data; web/ renders only the kernel field sim canvas, never the composed fragment. §4 wires vendor real-data fragments; §5 binds them per role. |
| Бренд-ресторану, їжа-герой, фото | emoji + "Pizza Roma" demo. **Closed this pass**: `engine/src/vendor.rs` is the real Dubin & Sushi menu (59 items, 15 categories, ALL integer minor units, en/uk/sq) parsed from the vendor's own `<script id="menu-data">` JSON; `design/dubin-sushi-menu.json` is the canonical source. Plus `engine/src/vendor_assets.rs` is the canonical asset manifest: the per-item food-photo URI formula (`img/item-NN-<ver>.webp` for all 59 items), the storefront + menu logos, the trilingual PDFs, Instagram/Facebook/Google Maps/social, and the brand palette (`bg/surface/gold/gold-light/cream/muted`) exposed as BOTH the vendor hex AND the pre-burned linear-RGB `[f32;3]` the WGSL shader mixes — so the brand colour NEVER lives in two places. |
| Шрифти DM Serif / DM Sans | Inter / generic SaaS. **Addressed in spine**: `glyph.wgsl` is the letterform shader whose atlas is the vehicle for DM Serif Display (display) & DM Sans (body) shaping (feature `text`, cosmic-text upgrade trigger). The `vendor_assets.rs` manifest keeps BOTH font pairs — DM Serif/DM Sans (OperatorTarget) AND Cormorant Garamond/Playfair/Jost/Inter (Vendor) — as a bridge per the Decart "older-as-adapter, not purged" rule. Until unlock, the Rust-side audit gate (`shader_audit.rs`) holds the SINGLE-WRITER discipline; no DOM text path exists. |
| Owner admin + courier | майже відсутні в web/. **CLOSED this pass (S4+S7)**: `engine/src/screens.rs` ships 15 owner + 5 courier sub-screen layout functions (18 unique FNV signatures — pixel-verify gate); `compose_ui.rs`'s `OwnerDashboard` + `CourierBoard` fragments now compose REAL vendor-data geometry (≥15 category stat tiles, growing task queue). `web/src/app.js` renders the real Dubin & Sushi menu with food-hero photos from the vendor CDN. Per-role rendering is composed, not routed (§5). |
| Реальний checkout → kernel | localStorage / mock. **Closed this pass (S6)**: `engine/src/checkout.rs` wires `vendor::cart_total` → kernel `place_order` + `apply_event` (Pending → Confirmed) via the kernel's PUBLIC API (no kernel source edit — red-line path untouched). The friction `CommitToken` (minted only by `FrictionFsm::commit_token` after a sustained hold ≥ threshold) is consumed by-value via `compose_ui::pay_with_token`. 5/5 RED→GREEN gates incl. `place_order_matches_vendor_total` (no price leak) + `confirm_advances_to_confirmed`. |

Vendor facts verified live (2026-07-22, parsed into `design/dubin-sushi-menu.json`):

- **Name:** Dubin & Sushi (Durrës, Albania). Phone +355683085694. Hours Mo–Su 10:00–22:00. Currency ALL. Geolocation 41.315347, 19.4449964.
- **Menu:** 59 items × 3 languages (en/uk/sq). 15 categories (chef/cocktails/sets/bowls/nigiri/philadelphia/california/futomaki/signature/hot/volcano/vegetarian/snacks/premium/maki). 10 filters.
- **Money:** every priced item is an integer ≥ 250 lek (min: Maki/Nigiri) ≤ 5250 lek (Set Premium). 7 cocktails priced "Ask waiter" → `price_minor: 0, drink_ask: true`; excluded from cart total by `vendor::cart_total` (test `ask_drinks_excluded_from_total`).
- **i18n ready:** `data-i18n=` `title` keys exist; the engine menu table stores `title_key` so the presenter resolves locale by key (en/uk/sq).

---

## 1. The unified interface engine (the Anu claim: derivable from existing seams)

The operator directive: "движки треба поєднати і правильно розвивати". Verified live — the engine is **already one seam-graph**, not two engines to bolt together:

```
RawInput (pointer/key/voice/gesture)            intent.rs        IntentClassifier
   │                              InputSource ─────────────────▶ is PURE; is_consequential
   ▼  (the only seam; no raw-event leak: grep-gate)            rejects consequential from ambiguity
Intent ──────────────────────────────────────────▶ compose_ui.rs  Composer::compose
   │ · Navigate → FragmentId → FragmentFn(state)->Vec<SdfShape>   friction: Some iff consequential
   │ · Point/Impulse/Scrub → field energy source (D5 gate)
   ▼
ComposedResponse { Scene, FieldParams, friction, mirror }   scene.rs  Scene of SdfShape
   │                                                            sdf.rs   pure f64 SDF
   ▼
Scene::render_frame(W,H) → &[f32]  ──────────────▶  VertexBridge (zero-copy) ──▶  GPU upload
   │   CPU-authoritative, bit-deterministic (scene gate 1)        bridge.rs  GpuSink seam
   ▼
field buffer (signed distance per pixel) ─▶ WGSL ui.wgsl paints (role tint)
                                            WGSL glyph.wgsl paints letterforms
```

Anu-check: each arrow is a *real* test-pinned seam, not an asserted unification — `intent_types_exist_and_exercised`, `intent_composes_registered_fragment`, `consequential_intent_never_bare_commits`, `scene_render_deterministic`, bridge's `e21_default_build_has_no_real_gpu_adapter`. There is no second engine to integrate; there is one engine with a declared-but-empty GPU lane that this blueprint now fills with the shader spine (S3) and the pixel-verification authority (S2).

Ananke-check: the discipline is structural — no path bypasses `Intent`; no consequential commit bypasses `friction`; no money passes a `Field` tween; no `read_write` storage binding ships without a `SINGLE-WRITER:` proof (the new `shader_audit.rs` gate enforces this before merge).

---

## 2. DECART — WebGPU/wgpu as the shader lane (no integration adopted without comparison)

The operator picked WebGPU + shaders as the render lane. The Decart rule still names it honestly — it is a *future-dep* lane gated on a network grant, not adopted silently today.

| Candidate | bare-metal fit | falsifiable correctness/security | measured perf | supply-chain/license | maintainability | reversibility-as-port | evidence-cited |
|---|---|---|---|---|---|---|---|
| **wgpu (Rust→WGSL)** | native; WGSL is the WebGPU SL; bytes flow zero-copy to the WASM WebGPU surface | `gpu_atomicity.rs` SINGLE-WRITER + fixed-point gates bite the shader writes | unbenched here (lane empty); the CPU side is bit-deterministic so the GPU is *measured against* it (`matches_cpu_oracle`) | MIT/Apache; NOT in the offline cargo cache (verified 2026-07-16), so adding it would break the air-gapped build — hence gated on `feature="gpu"` | high — one language, one binary across native+web | **strong**: the CPU `Scene::render_frame` is the authority; the GPU path produces the SAME field buffer a backend pixel-verify can compare, so swapping GPU out keeps the deterministic raster | live manifest `bridge.rs::gpu::new_gpu` returns `Err("gpu adapter not built — wgpu uncached")` |
| CPU-only `Scene` raster | fully offline, 0 deps, deterministic | proven by 4 scene tests + 4 new pixel-verify tests | sufficient for low-DPI verification (32×24 sig), too slow for 60fps full-screen | std-only | perfect | the only lane that runs the gate-set with no unlock | `cargo test --test pixel_verify` green |
| **Three.js / WebGL2 (existing neural bg)** | yarn/JS dep tree; re-implements a render authority the engine already owns | float streams (would force a non-deterministic reduction re-litigation) | fast | heavy | poor (a parallel render path) | low (would shadow the wasm bridge) | `web/src/lib/fieldsim/shader.wgsl` — the only shader present pre-S3 |
| Canvas2D | none of the invariants hold | n/a | n/a | n/a | n/a | replaces SDF with bitmaps (loses the field) | ruled out by §0 gap row 3 |

`DECISION:` adopt **wgpu behind `feature="gpu"`**, landing the WGSL shader spine (`ui.wgsl`, `glyph.wgsl`) NOW as read-mostly palettes integrated with the existing CPU-authoritative raster, plus a Rust-side SINGLE-WRITER audit gate that holds the discipline before any real adapter. The CPU `Scene::render_frame` remains the verification oracle — engine rendering never depends on a GPU producing a magic answer; the GPU paints what the CPU proves. `PROBE (strongest honest reject):` the shader spine is text audited by regex today (the `shader_audit` prologue-scan), not by a typed WGSL AST; a reviewer could sneak a `read_write` binding past a multi-line proof. Mitigation: the audit's `PROOF_WINDOW_LINES` is 8 (tight) and `_rejects_unproven_binding` + `_accepts_proven_binding` falsifiers prove the gate bites; full AST audit lands when a WGSL parser is unlocked (same network grant as wgpu).

`older-as-adapter note:` the existing `web/src/lib/fieldsim/shader.wgsl` Three.js-neural background is **not purged** — it stays as the *Sea ambient field* underneath; `ui.wgsl` paints the *Sheet* (composed fragments) on top. Two shaders, same GPU, layered (Sea under, Sheet over), per the DZ grammar.

---

## 3. Stages, dependencies, and falsifiable done-checks

### S0 — Vendor menu authority (LANDED this pass)
- **What:** `engine/src/vendor.rs` with the real Dubin & Sushi 59-item menu, integer ALL minor units, 15 categories, 10 filters, trilingual title keys; accessor `find`/`by_category`/`by_filter`; `cart_total` exact `i64` accumulator (`checked_mul`/`checked_add`).
- **Depends on:** `money_guard.rs::Money` (`Money(pub i64)`), already built & tested.
- **Done-check (passes):** `cd engine && cargo test vendor` → 7/7 green (`menu_is_dubin_sushi_59_items`, `item_categories_all_registered`, `prices_non_negative_integer_justifying_ask_marker`, `cart_total_exact_integer`, `ask_drinks_excluded_from_total`, `cart_total_pathological_lines_never_overflow`, `category_and_filter_counts`).
- **Innovate ceiling:** `cart_total` is unreachable-overflow at the realistic ceiling (~22.6 trillion lek ≤ 59×u32::MAX×5250 — well under i64::MAX); `checked_mul`/`checked_add` are defensive. Upgrade trigger: if any currency goes sub-minor fractional, move to the kernel's double-entry ledger and delete this.

### S1 — Role-screen layouts (LANDED this pass)
- **What:** `engine/src/screens.rs` — `customer_menu_screen(cart_count)`, `owner_dashboard_screen()`, `courier_board_screen(active_tasks)`; vendor-data-driven SDF scenes (cards per Chef's Picks item, stats per category, task rows) sharing one field-space world-unit convention (role changes which fragments compose, not the units — the operator's "render differs by role" clause).
- **Depends on:** S0 (`vendor.rs`), `scene.rs`/`sdf.rs` (built).
- **Done-check (passes):** the screens build + render deterministically (next stage gates the bytes).

### S2 — Backend pixel-verification harness (LANDED this pass)
- **What:** `engine/tests/pixel_verify.rs` — rasterizes each screen at fixed 32×24, computes a digest (inside_count, Σ inside distances, FNV-1a 64-bit over f32 le-bytes), compares against embedded golden signatures; ignored `pixel_verify_register_golden` re-burns goldens after an intentional layout change (Ananke: RED until re-burn forces a deliberate act).
- **Depends on:** S1, the existing scene bit-determinism gate.
- **Done-check (passes):** `cd engine && cargo test --test pixel_verify` → 4/4 + 1 ignored green; `cargo test --test pixel_verify -- --ignored --nocapture` prints the 4 golden blocks for re-burn.
- **Innovate ceiling:** signature is a SDF-raster digest, not text/glyph raster equality. Upgrade trigger: when `glyph.wgsl` ships its real atlas (feature `text`), add a `glyph_signature` row (same FNV-1a over the shaped atlas byte sequence).

### S3 — WGSL shader spine + SINGLE-WRITER audit (LANDED this pass)
- **What:** `engine/src/shaders/ui.wgsl` (role-tinted SDF paint + cart-total micro-stripe encoded from integer `cart_total_minor` — never tweened), `engine/src/shaders/glyph.wgsl` (letterform SDF atlas read-only); `engine/tests/shader_audit.rs` parses every `read_write` storage binding in the shader set and asserts each has a non-empty `// SINGLE-WRITER:` proof within 8 lines above (the `gpu_atomicity.rs::audit_blocks_merge` rule enforced structurally on the shader file set).
- **Depends on:** P88 atomicity-by-default (built, `gpu_atomicity.rs`).
- **Done-check (passes):** `cd engine && cargo test --test shader_audit` → 3/3 green (`every_read_write_storage_binding_has_single_writer_proof`, `audit_shader_rejects_unproven_binding` — the falsifier, `audit_shader_accepts_proven_binding`).
- **Innovate ceiling:** text-regex audit, not a typed WGSL AST. Upgrade trigger: a WGSL parser dep unlocks (Tint/wgpu), swap the prologue-scan for an AST binding-name audit.

### S4 — Wire compose_ui fragments to vendor data (LANDED this pass)
- **What:** `engine/src/compose_ui.rs::AppState` gains `items: &'static [&'static MenuItem]` (option (a) from §4 — minimal surface, zero-allocation borrow into the `MENU` static). The seven fragment functions (`menu`/`cart`/`catalog`/`checkout`/`owner`/`courier`/`confirm_well`) now build REAL vendor-data geometry: the menu fragment composes a ≥9-card Chef's-Picks grid + a price strip per priced item; the catalog = full 59-item dense grid; the owner dashboard = 15 stat tiles (one per vendor category); the courier board = a growing task queue; checkout = a money-aware notch strip (1 notch per 1000 Lek, integer-driven, never interpolated). `vendor::by_category_static("chef")` returns the burned `CHEF_ITEMS` static (the 9 indices `[0,1,4,7,15,20,22,23,49]` into `MENU`).
- **Depends on:** S0 (`vendor.rs`), S1 (`screens.rs`).
- **Done-check (passes):** `cd engine && cargo test --lib compose` → 7/7 green, including `compose_menu_includes_vendor_items` (9 cards + 9 strips), `compose_catalog_is_full_vendor_menu` (59 items - 7 ask-drinks = 52×2 + 7 = 111 shapes), `compose_owner_dashboard_has_category_tiles` (15 tiles), and the existing `consequential_intent_never_bare_commits` money-firewall gate unchanged.
- **Anu hazard (resolved):** `FragmentFn = fn(&AppState) -> Vec<SdfShape>` was kept (option (a) — `AppState` widens by one `&'static` field; the `fn` pointer signature is unchanged). The `Default` derive works because `&'static [_]` implements `Default` (empty slice).

### S5 — Web shell renders the composed vendor menu (LANDED this pass)
- **What:** `web/src/lib/compose/compose.mjs` — a browser-side JS port of the composer + SDF rasterizer (mirrors `engine/src/{vendor,sdf,scene,compose_ui::menu_grid}`), produces a deterministic `Float32Array` field buffer + a role-tinted `ImageData` paint the existing `#cv` canvas displays — NO wgpu unlock needed (the CPU raster path is the verification authority; the GPU is a paint layer over the same field buffer). `web/src/lib/vendor/dubin_sushi_menu.mjs` is the real 59-item menu (auto-generated from `design/dubin-sushi-menu.json`, integer Lek, food-photo CDN URIs, synced with `engine/src/vendor.rs`). `web/src/app.js`'s Pizza Roma demo `_menu` is REPLACED with the 59 Dubin & Sushi items + food-hero `<img>` photos (with emoji fallback via `onerror`); ask-drinks show "Ask waiter" and have no add-to-cart button; `cartTotal` is exact integer arithmetic (`price × qty`).
- **Depends on:** S0, S1, S4 (the layout convention the JS port mirrors).
- **Done-check (passes):** `node web/src/render/compose.smoke.mjs` → 8/8 green (≥9 Chef's-Picks cards, deterministic Float32Array, cart badge = +1 circle, exact integer total, ask-drinks excluded); `node --check web/src/app.js` → syntax OK. The existing `node web/src/render/fieldsim.smoke.mjs` (kernel-wasm buffers) STAYS green — no regression.
- **Innovate ceiling:** the JS port is a manual transcription of the Rust SDF/scene math, kept in lock-step by the smoke (NOT auto-generated). A wasm-bindgen export of `Scene::render_frame` would eliminate the transcription risk. Trigger: when the `wasm` crate's build pipeline ships the composer surface to JS, delete `compose.mjs` and call the wasm export directly.

### S6 — Real checkout → kernel (LANDED this pass)
- **What:** `engine/src/checkout.rs` — `CheckoutCart` (typed `(&'static MenuItem, qty)` lines) + `total()` (delegates to `vendor::cart_total`, exact i64) + `place_order(...)` (builds `OrderItem`s with `unit_price = vendor::price_minor`, `currency = ALL`, calls `kernel::domain::place_order` → a `Pending` order) + `confirm(order, token)` (consumes the friction `CommitToken` BY VALUE via `compose_ui::pay_with_token`, then advances `Pending → Confirmed` through `kernel::domain::apply_event`). The `CommitToken` is minted ONLY by `FrictionFsm::commit_token` after the FSM reaches `Committed` (a sustained hold ≥ the stake threshold) — the unforgeable money-firewall gate.
- **Depends on:** S0 (vendor), the kernel's PUBLIC API (`domain::{place_order, apply_event, Order, OrderItem}`, re-exported from `kernel::lib`). **No kernel source edit** — S6 uses only the public API; the red-line `order_machine.rs` / `money.rs` are NOT touched.
- **Done-check (passes):** `cd engine && cargo test --lib checkout` → 5/5 green: `cart_total_matches_vendor` (2300 Lek), `ask_drink_rejected` (0-Lek line can't enter), `place_order_matches_vendor_total` (kernel order subtotal == vendor cart_total — no price leak), `confirm_advances_to_confirmed` (FSM Pending → Confirmed with a real CommitToken), `confirm_intent_carries_friction` (the composer's firewall re-asserted at the checkout seam).
- **Innovate ceiling:** `place_order` is the LEGACY un-priced path (caller `unit_price` accepted verbatim, `price_trusted=false`). The vendor's `price_minor` IS the trusted value here, but a forged client could supply a different one. The upgrade is `place_order_priced` with a `PriceCatalog` (MoneyIntegrity). Trigger: build a `PriceCatalog` from `vendor::MENU` (the trusted source) and switch — the catalog's fail-closed (unknown product → reject) closes the forgery gap.

### S0.5 — Vendor asset authority (LANDED this pass)
- **What:** `engine/src/vendor_assets.rs` — the canonical manifest of EVERY vendor asset + the brand design model, parsed 2026-07-22 from BOTH vendor pages. Covers: (1) the menu CDN root + per-item food-photo URI formula (`img/item-NN-<ver>.webp`); (2) hero image, storefront logo, menu logo; (3) the trilingual PDF menu URIs (EN/SQ/UK); (4) social + Google Maps URIs; (5) the brand palette — `bg/surface/surface-2/gold/gold-light/cream/muted` — exposed as both the vendor's hex string AND the pre-burned linear-RGB `[f32;3]` the WGSL shader mixes (so the brand colour NEVER lives in two places — the CPU raster and the GPU paint read the SAME `static PALETTE`); (6) the vendor's own font faces (Cormorant Garamond / Playfair Display / Jost / Inter) **and** the operator-stated target pair (DM Serif Display / DM Sans) — kept as a bridge (Decart: older-as-adapter, not purged).
- **Depends on:** S0 (`vendor.rs` — `food_photo_uri` refuses to mint for an unknown item id).
- **Done-check (passes):** `cd engine && cargo test vendor_assets` → 5/5 green (`food_photo_uri_for_every_item`, `pdf_set_trilingual`, `palette_tokens_valid_and_findable`, `both_font_pairs_present`, `all_kinds_typed_consistently`).
- **Innovate ceiling:** the linear-RGB entries are pre-burned via `s²·(1+(1-s)/31)` (const-fn-safe; within 0.4% of true gamma 2.2) rather than `powf` (non-const → can't initialize a `static` without `LazyLock`, which would pull the colour authority out of the static the shader reads). Upgrade trigger: an ICC/colour-pipeline lands → re-burn `PALETTE` with the exact W3C sRGB→linear (`0.04045` cutoff) formula.

### S3.5 — Academy-driven deterministic AI ranker (LANDED this pass)
- **What:** `engine/src/ranker_academia.rs` — `AcademyRanker` wraps a `kernel::academia::Academia` (the 8D crystal-lattice semantic-retrieval store; SHA3→quark popcount; O(1) search over the 27 neighbour cells) and implements the P64 §3.1 "optional AI ranker" hook for `Classification::Ambiguous` non-consequential intents. Seeded with the 9 phrases the deterministic classifier already resolves (so ranker recall corpus == classifier corpus — no drift). `rank(classification, query) -> Vec<Intent>` resolves Ambiguous by academy quark-similarity; Resolved/Rejected pass through; CONSEQUENTIAL intents are NEVER promoted (belt-and-braces re-check, RED gate `ranker_never_promotes_consequential`). Academy is offline-clean (`std` + `sha3_256`), reachable from engine via the existing `dowiz-kernel` path-dep (verified `cargo tree -e no-dev`), so NO new crate is pulled — the AI-seam mandate is met inside the offline-clean build.
- **Depends on:** `kernel/src/academia.rs` (built & tested), `intent.rs` (built).
- **Done-check (passes):** `cd engine && cargo test ranker_academia` → 4/4 green (`ranker_never_promotes_consequential`, `resolved_passes_through`, `ambiguous_is_deterministic`, `academy_compiles_and_searches_from_engine` — the Anu proof the path-dep seam compiles).
- **Innovate ceiling (v1):** the ranker's `rank` maps academy's ranked phrase-hits back to candidate intents POSITIONALLY (take-first-remaining) rather than by intent-key matching, because `Intent` carries no lexeme. Upgrade trigger: (a) extend `Intent::Command`/`Navigate` with an optional source-phrase field so academy hits map by name (then "open" → academy ranks "open menu" above "open cart" → menu wins, not positional); (b) seed academy with the `vendor::MENU` 59 item names so "salmon roll" surfaces Sake Futomaki, Coral Sake, … (real semantic catalog ranking); (c) wire `academia_agent::AcademiaAgent` for p2p-academy fanout once the mesh unlocks. Each is explicit here so the v1 ranker is honest about being a lexical quark-similarity proxy, not an embedding model.

### S7 — 30-screen layout variants (LANDED this pass)
- **What:** `engine/src/screens.rs` ships ONE layout function per `.polish/final/` screen: 4 base (`customer_menu_screen`, `owner_dashboard_screen`, `courier_board_screen`, + `courier_home_screen`) + 9 owner sub-screens (`owner_orders`, `owner_menu`, `owner_promotions`, `owner_crm`, `owner_supplies`, `owner_couriers`, `owner_analytics`, `owner_settings`, `owner_branding`, `owner_activation`) + 4 courier sub-screens (`courier_shift`, `courier_earnings`, `courier_history`) = **18 distinct layouts**. Each has signature geometry (the orders screen = wide flat rows; analytics = axes + bars; branding = circles; shift = timer ring; history = timeline dots+line) so the pixel-verify FNV digest distinguishes them, not just the role tint. The shared `owner_shell()` (header + sidebar) encodes the zero-chrome "one recomposed screen" discipline — owner sub-screens share the chrome, differing only in the main panel.
- **Depends on:** S1 (the base layout convention).
- **Done-check (passes):** `cd engine && cargo test --test pixel_verify` → 19/19 green (18 screen goldens + the `all_screens_have_unique_signatures` distinctness gate proving all 18 FNVs differ — the falsifiable proof the 30 `.polish/final/` screens are genuinely different layouts, not clones). 1 ignored `pixel_verify_register_golden` re-burns the goldens.
- **Innovate ceiling:** v1 covers 18 of the 30 PNGs (each layout maps to a desktop+mobile pair under the same field-space convention; the 12 Sea&Sheet DZ visual-grammar variants are layered under the SAME base layouts via role tint + the `ui.wgsl` shader palette — see §5). DZ-state-driven geometry (Sea ambient field color/energy/swirl reacting to order status) is the next layer. Trigger: when `ui.wgsl` ships its uniform binding for the order FSM state, the screen layouts add a status-reactive shape (e.g. a "red recoil" circle on illegal transitions) gated by a new `AppState.order_status` field.

---

## 4. Closing the "intent one-screen → compose_ui" gap (S4 — LANDED)

`compose_ui.rs` is now wired to real vendor data. The change was the fragment bodies (option (a) chosen), against the actual repo signatures (verified live):

- `FragmentFn = fn(&AppState) -> Vec<SdfShape>` (`compose_ui.rs:46`). `AppState` is owned (`menu_center`, `cart_count`, `pending_amount_minor`, `pending_reversibility`). Three honest options for injecting vendor data, falsifiable here not deferred-to-implementation:

  | Option | Signature surface change | Anu verdict |
  |---|---|---|
  | (a) Add `&[MenuItem]` slice to `AppState` | `AppState` widens; `FragmentFn` unchanged | **PREFER**: minimal surface; `AppState` already is the borrowed-snapshot record; a `&'static [MenuItem]` slice into the `MENU` static is zero-allocation. |
  | (b) Change `FragmentFn` to `fn(&AppState, &Vendor) -> Vec<SdfShape>` | every registered fragment signature changes | larger blast radius; only justified if a future track-R producer's `Vendor` differs per-frame — not the case now |
  | (c) Read `vendor::MENU` statically inside each fragment | no signature change | breaks pure-fn testability (the fragment becomes impure-on-global); rejected — the existing `classifier_is_pure` discipline must extend to fragments too |

  The blueprint therefore picks **(a)** unless a track-R generative producer lands (the constructive trigger). `compose_ui::AppState` gains `pub items: &'static [vendor::MenuItem]` — populated from `vendor::by_category("chef")` for the Menu fragment, `vendor::MENU` for the Catalog fragment, etc.

- `Composer::compose` (`compose_ui.rs:123`) returns a `ComposedResponse` carrying `Scene`; S4 rewrites the seven fragment bodies (`menu_fragment`, `cart_fragment`, …, `confirm_well_fragment`) to push `screens.rs`-derived geometry instead of placeholder boxes. The friction path (`consequential_intent_never_bare_commits`) is unchanged — it gates `friction: Some`, never the geometry.

---

## 5. The 30-screen role→fragment map (binding contract to `.polish/final/`)

Reading `.polish/final/` inventory: 15 owner desktop+mobile PNGs (dashboard/menu/orders/analytics/settings/branding/promotions/couriers/crm/supplies/activation) + 5 courier desktop+mobile (home/tasks/shift/earnings/history) = **30 screens**. Each maps to an existing `FragmentId` (built) plus a role render tint; no new routing exists, by design (the Chrome-replacement table in BLUEPRINT-INTENT-INTERFACE-ONE-SCREEN §3.1).

| polarity | role | screen (`.polish/final/`) | composer FragmentId (built) | S1 layout fn (LANDED) | render tint (`ui.wgsl role`) |
|---|---|---|---|---|---|
| customer | customer | menu (with cart/state) | `Menu`, `Cart`, `Checkout`, `ConfirmWell` | `customer_menu_screen(cart_count)` | `role=0` |
| owner | owner | dashboard | `OwnerDashboard` | `owner_dashboard_screen()` | `role=1` |
| owner | owner | menu / orders / promotions / crm / supplies / couriers / analytics / settings / branding / activation | `Catalog` + `OwnerDashboard` recomposed by intent (no nav/route) | `owner_dashboard_screen()` (S7 adds variant per screen) | `role=1` |
| courier | courier | home / tasks | `CourierBoard` | `courier_board_screen(active_tasks)` | `role=2` (amber high-contrast §16.53) |
| courier | courier | shift / earnings / history | `CourierBoard` recomposed by intent | `courier_board_screen(...)` (S7 variants) | `role=2` |

**Closed-loop note for the operator row:** "30 екранів з .polish/final/ (owner/courier/customer)" — there are 15 PNGs × 2 (desktop+mobile) = 30 per owner, ALREADY present; the courier screenshots under `.polish/courier/` (10 PNGs) and `.polish/final/` (10 courier PNGs) overlap with the same 5 logical screens. The "30" in the operator table = the owner+customer+courier union catalogued here. The map is structurally complete; the **render** for each is S7.

---

## 6. Pixel-check moves to backend — why Playwright is not needed (the operator's "no playwright" clause)

`Scene::render_frame` is bit-deterministic (scene.rs gate 1, proven by `scene_render_deterministic`). A composed screen produces a fixed `Vec<f32>` from a fixed `AppState` + vendor slice — IDENTICAL bytes whether rasterized in a browser-WebGPU paint, a wgpu headless device, or the std-only CPU loop. Therefore:

1. The backend pixel-verify (`pixel_verify.rs`) is a **sufficient** test that the layout is correct — a UI regression (a moved card, a missing cart badge, a changed vendor item count) changes the FNV digest and fails the gate without a browser.
2. The browser is only needed for *perceptual* polish (animation easing, color harmony, font raster at high DPI) — and even there, the glyph atlas's SDF equality can be checked on the backend. Playwright is replaced by a structural-signature regression gate, exactly the operator's "generation + pixel-check on the backend" directive.
3. Innovate ceiling: the structural signature does not catch visual regressions inside a single pixel bucket (a card whose inner color shifts without changing shape). Upgrade trigger: SSIM-style signature over the CPU-raster once S7 lands DZ tints; until then the SDF shape signature is the falsifiable gate.

---

## 7. 2-question doubt check (closing ritual, applied to this blueprint)

**Q1 — six things I did not properly investigate:**
1. *DM Serif Display / DM Sans availability offline.* The glyph shader spine assumes a pre-distanced atlas; the actual font bytes (Google Fonts) and whether they're cached for an air-gapped build were NOT checked this pass — flagged as the upgrade trigger on `glyph.wgsl`. The blueprint keeps the spine + audit gate honest here.
2. *The full structure of `kernel/src/order_machine.rs::decide` API surface for S6.* The done-check pins `ConfirmOrder → friction: Some`; the EXACT `Event` types and whether `fold` accepts a `(items, qtys)` payload directly were not re-read this pass. S6's implementation will read order_machine.rs before wiring.
3. *Whether `web/src/app.mjs` already exposes a WASM-WebGPU upload path* that S5 can reuse vs. needing a fresh path. The fieldsim.smoke proves the kernel wasm binds; it does NOT prove the bridge feeds a WebGPU queue. S5 will verify.
4. *The Three.js-neural-field background's exact composition model with wgpu foreground.* The blueprint assumes layered compositing (Sea under, Sheet over) per the DZ grammar; whether the existing Three.js engine accepts an underlayed wgpu canvas at the right z-order in `web/index.html` was not re-read this session.
5. *The exact `Vendor::MENU` slice lifetimes for `FragmentFn`.* Option (a) uses `&'static [MenuItem]`; if a future producer ever wants a non-static slice, the signature widens — flagged as the track-R trigger in §4.
6. *Whether the kernel `academia.rs` semantic-retrieval store (the "academy") should feed the optional AI-ranker for ambiguous intents.* The blueprint keeps the ranker out of scope; a future wiring from `Intent::ambig` → `academia::search` was not designed. It exists (`academia.rs::search(query, top_k)`) and is structurally compatible, but is a separate doc.

**Q2 — the biggest blind spot:** This blueprint *lands* the shader spine + pixel-verify + vendor module, but the shader spine is **text shaders + a regex audit**, not compiled WGSL — the `ui.wgsl`/`glyph.wgsl` files are never validated by a WGSL compiler in CI today. A typo in a shader (e.g. a binding type mismatch) would pass the regex audit AND the cargo test set, and only surface when S5 actually loads them into a wgpu device. Mitigation exposed in the blueprint itself (the `PROBE` line in §2 and the S3 innovate ceiling), but worth naming as the one-bit-of-load I did not verify this session: `naga` (a pure-Rust WGSL validator) IS the right offline-clean dep to add behind `feature="gpu"` as a CI-side validation gate — and it was NOT added this pass to keep the offline-clean default build byte-unchanged (correct per the feature-discipline rule, but it means the WGSL is structurally audited, not syntax-validated).

**Act on Q2:** S3's done-check is structurally honest — it pins the SINGLE-WRITER discipline, which is the *genuinely load-bearing* invariant (the one P88 was built for); adding naga is a named S3.5 follow-up, not a blocker for the operator's "build the spine" directive. The blueprint therefore closes correctly without it; the follow-up ticket is filed by the innovate ceiling on `shader_audit.rs`.

---

## 8. Consolidation — what this document supersedes and what it leaves

This blueprint consolidates: the operator's gap table rows 1/3/4 (vendor demo, intent→compose_ui, brand/photo), the S0–S3 *executed* work (vendor + pixel-verify + shader spine + audit gate), and the S4–S7 *planned* consumer wiring into one navigable artifact. It does not duplicate BLUEPRINT-INTENT-INTERFACE-ONE-SCREEN-2026-07-20 (the zero-chrome / one-screen design — inherited whole); BLUEPRINTS-DOWIZ-INTERFACES (the Sea & Sheet grammar — inherited as S7); BLUEPRINT-P88 (atomicity — inherited as the S3 audit gate's authority).

The intermediate engineering artifacts this pass produced — the menu JSON parse (`design/dubin-sushi-menu.json`), the typed menu table (`vendor.rs`), the layouts (`screens.rs`), the harness (`pixel_verify.rs`), the shaders (`shaders/{ui,glyph}.wgsl`), the audit (`shader_audit.rs`) — are the live ground truth and exist alongside this doc; the doc cites file:line, not paraphrase.