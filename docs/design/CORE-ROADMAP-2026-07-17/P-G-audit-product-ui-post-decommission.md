# P-G Wave-1 Audit — Product/UI on the kernel, post-decommission (2026-07-17)

> **RECONSTRUCTION NOTICE.** This is the Wave-1 Opus ground-truth audit for **Layer P-G** of
> `CORE-ROADMAP-STANDARD-2026-07-17.md` (§3 row P-G). The original was lost before commit — an
> uncommitted on-disk file, wiped when a separate consolidation session merged ~20 `feat/*`
> branches onto `main` (root cause confirmed; the untracked `CORE-ROADMAP-2026-07-17/` dir had no
> git-recovery path — see `BLUEPRINT-P-I-consolidation.md` G6). Its load-bearing content had
> already been adopted verbatim into `BLUEPRINT-P-G-product-ui.md` §0–§1; this document restores
> the full audit and **re-verifies every citation fresh against current `main`** so the reference
> from `CORE-ROADMAP-INDEX.md` and `BLUEPRINT-P-G-product-ui.md` resolves.
>
> **Live tree verified this pass:** branch **`main`**, HEAD **`caba2203c`** ("docs: land
> CORE-ROADMAP Layer A-I execution structure"). This supersedes the branch note in
> `BLUEPRINT-P-G` (`f01f9bb6b` on `feat/p19-growth-engine`) and the session-header
> `feat/harness-llm-backend` snapshot — both stale; the reconcile-vs-reality rule (STANDARD §2.1)
> requires re-verifying `git`, not trusting a pasted status. All `file:line` cites below were
> re-read on `caba2203c`. Working tree clean except `kernel/Cargo.lock`.

---

## §0 — Corrected scope (READ THIS FIRST): greenfield build-out, NOT a migration

The original Batch 9 audit (`bebop2-mesh-tensor-hermetic-2026-07-17/18-BATCH9-product-layer-kernel-wasm-migration-audit.md`)
framed P-G as a **migration** of the live `apps/api` + `apps/web` product onto the kernel/WASM
substrate, with an elaborate RLS-preservation story (its §D). **That framing is now stale and is
corrected here.** Between that audit and this pass the operator **decommissioned the entire
Node/TS product layer**:

- Commit **`5675c349b`** — *"chore: decommission outdated Node/TS workspace (apps/web, packages/*)
  + Docker"* — removed the old React/Vite storefront and every `packages/*` workspace.
- Commit **`db766de47`** — *"refactor: remove legacy JS/TS thin-layer, kernel is now sole source
  of truth."*
- **Verified this pass:** `git ls-files apps/` → **0 files**; `git ls-files packages/` → **0
  files** (`packages/` survives only as an empty on-disk dir). Matches
  `GROUND-TRUTH-2026-07-17.md` ("No decommissioned `packages/`/`apps/` resurrected in `main`,
  grep-confirmed 0"). There is **no old app to migrate from**.

**Corrected charter for P-G:** a **greenfield build-out of the new top-level `web/` beachhead**
toward the no-DOM physics-UI vision (`docs/design/living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md`,
`physics-ui-capture-quantum-math-arc`, `field-ui-engine-arc`). It is **not** a migration and
**not** a backend re-plumb. Consequently the Batch 9 §D RLS/`NOBYPASSRLS`/tenant-GUC analysis is
**out of scope for this beachhead** — there is no DB session-adapter here to preserve or break
(see §6). This inverts the highest-risk item of the original audit into a non-item.

---

## §1 — Method

Zero edits (audit only; Fable writes `BLUEPRINT-P-G` against STANDARD §2). Read in full, this
pass, on `main`@`caba2203c`: `web/index.html`, `web/src/app.mjs`, `web/src/lib/kernel/kernel_client.mjs`,
`web/serve.mjs`, `web/package.json`, `web/src/lib/kernel/kernel.test.mjs`, `wasm/src/lib.rs`,
`kernel/pkg-web/dowiz_kernel.js`, `scripts/build-kernel-wasm.sh`. Kernel export surface counted
from `kernel/src/wasm.rs`; the **compiled** `kernel/pkg-web/dowiz_kernel_bg.wasm` export table was
parsed byte-for-byte (WASM section-7 dump) — surfacing the stale-binary finding in §7.

---

## §2 — Ground-truth inventory (STANDARD §2.1; every claim cited, verified this pass)

**`web/` = 7 tracked files, zero dependencies.** `web/package.json` has **no** `dependencies` /
`devDependencies` key (name `dowiz-field-ui`, `"type":"module"`, two scripts: `serve`, `test`).
Tracked set: `README.md`, `index.html`, `package.json`, `serve.mjs`, `src/app.mjs`,
`src/lib/kernel/kernel.test.mjs`, `src/lib/kernel/kernel_client.mjs`.

**Exactly ONE network call exists in `web/src`.** `app.mjs:8`:
`fetch('/pkg-web/dowiz_kernel_bg.wasm')` — loading the WASM binary itself. No HTTP API call, no
`WebSocket`, no `EventSource`, no `XMLHttpRequest` anywhere in `web/`. The Node path
(`app.mjs:11-12`) `readFileSync`s the same binary off disk. **`web/` is backend-less and
local-first: all geo/spectral/FSM math executes in-browser through the WASM-compiled kernel**
(this is the §0 charter, made concrete).

**`web/serve.mjs`** is a zero-dep static file server: `ROOT` is the **repo root** (`serve.mjs:9`;
the inline `// web/` comment is wrong — flag), `.wasm` served as `application/wasm` (`:16`),
`/pkg-web/*` mapped to `kernel/pkg-web/*` (`:25-26`, no wasm duplication). Consequence:
`/wasm/demo/pkg/dowiz_wasm.js` (the sibling crate, §5.1) is **already servable with zero
serve.mjs changes**.

**Kernel WASM export surface = 24 `_js` entry points** (`kernel/src/wasm.rs`, grepped this pass;
each is a `#[wasm_bindgen] pub fn` over a pure `Result<String,String>` logic fn, no DB/IO):

| Domain | Exports (line) |
|---|---|
| Order FSM | `place_order_js:284`, `apply_event_js:296`, `boot_verify_fsm_js:326`, `fsm_graph_report_js:424` |
| Money | `estimate_order_total_js:408` |
| Analytics | `channel_ledger_js:307`, `reduce_anomalies_js:315` |
| Geo/route (10) | `geo_haversine_js:484`, `geo_lerp_js:499`, `geo_bearing_js:513`, `geo_progress_js:531`, `geo_progress_flat_js:550`, `geo_eta_js:562`, `geo_should_snap_js:576`, `geo_is_arriving_js:588`, `geo_point_in_polygon_js:601`, `geo_is_out_of_order_js:615` |
| Spectral (6) | `spectral_eigenvalues_js:661`, `spectral_radius_js:666`, `spectral_gap_js:671`, `spectral_algebraic_connectivity_js:677`, `spectral_classify_drift_js:682`, `spectral_flat_js:776` |
| Graph | `harmonic_centrality_js:723` |

**`scripts/build-kernel-wasm.sh` is intact** (`set -euo pipefail`, fail-closed): compiles
`kernel` to `wasm32-unknown-unknown --release`, then emits two `wasm-bindgen` glue targets —
`kernel/pkg/` (`--target nodejs`) and `kernel/pkg-web/` (`--target web --no-typescript`).
**`kernel/pkg-web/` is built on disk** (`dowiz_kernel_bg.wasm` 321 KB, `dowiz_kernel.js` 3.5 KB) —
but stale relative to source (§7).

---

## §3 — The three gaps (the corrected top-3; adopted into `BLUEPRINT-P-G` §0)

| Gap | Finding | Nature |
|---|---|---|
| **G1** | `compose_field` + stateful `FieldSim` exist, are git-tracked, compiled, and determinism-tested — but `web/app.mjs` never calls them | **pure wiring gap** (NOT a Phase-4 math dependency; corrects P16 — §8) |
| **G2** | Only **3 of 24** kernel `_js` exports are bound in `kernel_client.mjs`; `place_order_js`/`apply_event_js`/`estimate_order_total_js` unbound | **blocks cart/checkout/money flows** |
| **G3** | `app.mjs` is console-only — never touches the DOM; `index.html`'s cards/canvas/button are inert; **zero product pages** | **no real product surface yet** |

---

## §4 — Backend-less / local-first confirmation (the scope invariant)

The single `fetch` in §2 is the whole network surface. There is no server round-trip in any
product path: order placement, FSM transitions, money estimation, geo route-progress, and spectral
drift are **all** kernel exports meant to run in-browser against local state. This is the
`LIVING-INTERFACE-ROADMAP` end-state substrate — the `web/` Astro/Svelte-free, zero-dep beachhead
that already loads the kernel via `kernel_client.mjs`. **Nothing in P-G reintroduces a backend**;
persistence/sync, when it comes, is a later mesh-transport concern (Layer P-E), not a UI-phase
dependency. The RLS/tenant-GUC hazard that dominated Batch 9 §D **does not apply here** — the
kernel touches no DB, and there is no TS session-adapter on this beachhead to delete (§6).

---

## §5 — Detailed gap analysis

### §5.1 — G1: the field capability already ships (in the sibling `wasm/` crate)

The single biggest planning trap is that there are **two** WASM modules, and the physics-UI
capability lives in the **second** one:

| Module | Crate | Output dir | Loaded by `web/`? |
|---|---|---|---|
| `dowiz_kernel` | `kernel/` | `kernel/pkg-web/` | **Yes** — `app.mjs` → `kernel_client.mjs` |
| `dowiz_wasm` | `wasm/` | `wasm/demo/pkg/` | **No** |

`wasm/src/lib.rs` (git-tracked, `5bbd00272`) exports, over the kernel/engine field math:
- `compose_field(circles, w, h, steps) -> Vec<u8>` (`:56-59`) — kernel-computed field → RGBA8 the
  GPU blits; deterministic.
- stateful **`FieldSim`** (`:64-109`): `new(circles,w,h)` rasterizes the SDF source, `step()`
  advances one physics timestep (for a rAF loop), `frame()` returns `w*h*4` RGBA bytes,
  `width()`/`height()`.
- `vertex_field(count, edges) -> Vec<f32>` (`:111-114`) — graph-Laplacian `y = L·x`.
- plus `knowledge_map`/`lookup_tag`/`related_docs` (W7 spine fields).

Proven in-crate (host `cargo test`, 8 tests): `compose` is bit-deterministic (`:203-211`),
`FieldSim` evolves and stays finite (`:263-284`), `FieldSim` is bit-deterministic across sims
(`:289-301`), Laplacian matches the engine's known −0.55 triangle value (`:191-201`). The compiled
artifact is tracked (`wasm/demo/pkg/{dowiz_wasm.js,dowiz_wasm_bg.wasm}`), and the **headless smoke
test passes**: `wasm/demo/smoke.mjs` (`:12-16`) constructs a 64×64 `FieldSim`, steps it 30×, and
asserts `frame().length === 64*64*4` ("W12 SMOKE PASS").

**Verified absent from the kernel WASM:** `compose_field`/`FieldSim`/`vertex_field`/`knowledge_map`
are **not** in `kernel/pkg-web/dowiz_kernel_bg.wasm` (byte-grep, this pass) — they are `dowiz_wasm`,
a distinct binary. So G1's fix is **load a second WASM module** in `web/` (trivially servable per
§2) and drive `FieldSim` from `app.mjs`'s rAF loop into `#cv` — a **wiring** task, not a math
build. `serve.mjs` needs no change.

### §5.2 — G2: only 3 of 24 exports bound

`kernel_client.mjs` (66 lines) binds exactly **three** exports — `spectral_radius_js` (`:56-59`),
`geo_progress_flat_js` (`:60-63`), `fsm_graph_report_js` (`:64-66`) — behind a fail-closed
multivalue decoder `decodeRet` (`:49-55`, `{ok:false}` on kernel reject). `app.mjs` imports only
those three (`app.mjs:3`). The **money/order surface is entirely unbound**: grep of `web/` for
`place_order_js`/`apply_event_js`/`estimate_order_total_js` → **zero hits**. Until they are bound,
cart/checkout/lifecycle cannot exist — this is what actually blocks the product, not the field
render. Binding order (audit recommendation): order/money first
(`place_order_js`→`apply_event_js`→`estimate_order_total_js`), then the remaining geo/spectral/
analytics for the display cards. **Constraint on the fix: see §7(g)** — the binding must follow the
hand-rolled ABI pattern, not a glue import.

### §5.3 — G3: `app.mjs` is console-only; the DOM is inert

`app.mjs` (35 lines) binds the kernel and `console.log`s ρ, the FSM report, and geo-progress, then
`process.exit(1)` on any failure (`:18-34`). **It contains no `document`, no `canvas` handle, no
`addEventListener` — zero DOM references** (whole file read). Meanwhile `index.html` defines a full
card shell that **nothing populates**: `#rem`/`#snap`/`#seg` (route), `<canvas id="cv">` +
`<button id="tick">advance</button>`, `#rho`/`#gap`/`#fie`/`#drift` (spectral), `#fsm`/`#acyc`
(FSM) — every value renders the literal placeholder `–`, the canvas is never drawn, the button has
**no handler**. The index also displays fields `app.mjs` doesn't even compute (spectral gap γ,
Fiedler λ₂, drift class, snapped, segment). **Product pages = 0.** G3's fix is the first real DOM
pass: populate the existing cards, wire `#tick`, and stand up one minimal order surface.

---

## §6 — Money-authority + RLS note (why Batch 9 §D does not gate this beachhead)

On the decommissioned product, money had **dual authority** (TS charge vs kernel display,
`18-BATCH9-…-audit.md:87-90`, §B.4 T2) and RLS lived in a TS session-adapter that a naive deletion
could break (Batch 9 §D). **Neither hazard exists on `web/`:** there is no TS charge path and no DB
adapter — the kernel is already the sole authority, and it touches no database, so it can neither
enforce nor bypass RLS. The money **charge-authority** question is therefore not a UI-wiring
side-effect; it belongs to the kernel-money-authority story (`feat/rw-03-kernel-money-authority`
intent) and the **P06 `key_V` signed done-gate** (memory:
`sovereign-architecture-19-phase-roadmap-2026-07-17.md`) — explicitly **gated, not part of P-G**.
`estimate_order_total_js` gives the client a safe **preview**; flipping any charge decision stays
behind P06. STANDARD §2.6 hazard note: `place_order_js` reconstructs `price_trusted: false` at the
JS boundary (`wasm.rs:156-158`) — fail-closed by type, so an unbound-then-bound money path cannot
silently trust a client price.

---

## §7 — Two new ground-truth findings (this pass) + a stale-binary corollary

**(g) `kernel/pkg-web/dowiz_kernel.js` is init-only — zero per-export shims.** Read in full (3.5
KB): it contains only `__wbg_load`, `__wbg_get_imports`, `initSync`, `__wbg_init` (default export),
and — critically — `import * as __wbg_star0 from 'wbg'`, a **bare specifier that resolves in
neither Node nor the browser** without an import map. There is **no `place_order_js` wrapper, no
per-export shim of any kind** in it. A naive implementer who writes
`import { place_order_js } from '.../dowiz_kernel.js'` **fails**. The only working binding path is
the one `kernel_client.mjs` already uses: a **hand-rolled wasm-bindgen-0.2.95 ABI**
(`passStringToWasm0` `:19-24`, `getStringFromWasm0` `:14-17`, raw `wasm.<export>(p,l,…)`,
`decodeRet` `:49-55`), with the runtime block copied from `wasm/demo/pkg/dowiz_wasm.js`
(`:2-3`). **G2's fix (§5.2) must extend that hand-rolled shim, not glue-copy `dowiz_kernel.js`.**
Still current on `main`@`caba2203c`.

**(h) `boot_verify_fsm_js` exists but is never called — the fail-closed boot gate is unwired.**
Defined `wasm.rs:326` (header: *"Call once at web-kernel init, before any order is placed"*) and
re-exported `kernel/src/lib.rs:218`, but a repo-wide grep (`--include=*.mjs,*.js,*.rs,*.html,*.ts`,
docs excluded) finds it **only** at those two definition sites — **zero call sites**; nothing in
`web/` invokes it. The boot gate that would fail-closed if the FSM graph is corrupt is present in
the kernel but **not wired into the web boot path**. Still current on `main`@`caba2203c`. Fix:
call it immediately after `bindKernel` and render a fatal banner on `Err`.

**Corollary — the on-disk `pkg-web` binary is STALE (strengthens (g)/(h)).** Parsing the export
section of `kernel/pkg-web/dowiz_kernel_bg.wasm` (built Jul 16) yields **only 22** `_js` func
exports — **`boot_verify_fsm_js` and `harmonic_centrality_js` are ABSENT from the compiled
binary**, though both exist in `wasm.rs` source (24). So finding (h) is doubly true: the boot gate
is neither called **nor even present** in the shipped wasm. **Prerequisite for G2/(h): re-run
`scripts/build-kernel-wasm.sh`** to regenerate `kernel/pkg-web/` at 24 exports before binding the
full surface. (This is a real build step, not a doc note.)

---

## §8 — Correction of P16's stale Phase-4-dependency claim (required)

`BLUEPRINT-P16-product-ui-rebuild.md` (§1/§4/§9, acceptance item 4) claims the Sea render layer has
a **"hard dependency on Phase 4's `compose` export"**, *"verified absent from `wasm.rs` today"*
(`:70-73`, `:213`). **That claim is STALE.** The grep behind it was scoped to
`kernel/src/wasm.rs` — but the capability was never going to be there: `compose_field`/`FieldSim`
live in the **sibling `wasm/` crate** (§5.1), where they are git-tracked, compiled, and
determinism-tested **today**. **The Sea render is a wiring task in `web/app.mjs` (G1), not a
cross-phase math dependency. Nothing in P-G waits on Phase 4.** (P16's own §10.1 appendix already
conceded this; this audit makes it the operative finding.)

---

## §9 — Prioritized build order (audit recommendation; `BLUEPRINT-P-G` turns this into the plan)

0. **Rebuild `kernel/pkg-web/` via `scripts/build-kernel-wasm.sh`** so the compiled surface = 24
   exports (§7 corollary). Blocking prerequisite for steps 1 and 3.
1. **G2 — bind the order/money exports first** (`place_order_js`, `apply_event_js`,
   `estimate_order_total_js`), extending the hand-rolled ABI in `kernel_client.mjs` per §7(g); then
   the remaining geo/spectral/analytics exports for the display cards. Keep `decodeRet` fail-closed.
2. **G3 — first real DOM pass:** populate the existing `index.html` cards from bound exports, wire
   the `#tick` button, stand up ONE minimal order surface. `app.mjs` stops being console-only.
3. **(h) — wire the boot gate:** call `boot_verify_fsm_js()` right after `bindKernel`; fatal banner
   on `Err`. (Needs step 0 — it is absent from the current binary.)
4. **G1 — wire the field render:** load `wasm/demo/pkg/dowiz_wasm.js` (already servable, §2), drive
   `FieldSim.step()/frame()` from a rAF loop into `#cv`. No `serve.mjs` change, no Phase-4 wait.
5. **Extend the green gate:** `web/src/lib/kernel/kernel.test.mjs` (the existing W17 gate) grows
   with each newly-bound export and a headless `FieldSim` liveness assertion mirroring
   `smoke.mjs:12-16`; never replaced.

**Explicitly deferred (not P-G):** money charge-authority flip (P06-gated, §6); the full
26-page/i18n/WCAG/Sea-&-Sheet inventory (stays `BLUEPRINT-P16` + `LIVING-INTERFACE-ROADMAP`);
mesh persistence/sync (Layer P-E). This phase is minimal-but-real product surface, not a redesign.

---

## Links & memory (STANDARD §2.7)

- **Standard/contract:** `CORE-ROADMAP-STANDARD-2026-07-17.md` (§3 row P-G, §2 twenty-point).
- **Blueprint that adopts this audit:** `CORE-ROADMAP-2026-07-17/BLUEPRINT-P-G-product-ui.md`
  (§0–§1 quote this doc; §2 corrects P16; §4–§6 fix G1/G2/G3).
- **Index / crosswalk:** `CORE-ROADMAP-INDEX.md` (this file resolves its P-G "MISSING ON DISK"
  row); `BLUEPRINT-P-I-consolidation.md` G6 (the loss event).
- **Superseded/corrected:** `bebop2-mesh-tensor-hermetic-2026-07-17/18-BATCH9-product-layer-kernel-wasm-migration-audit.md`
  (migration framing → greenfield; §D RLS story → N/A on this beachhead);
  `sovereign-roadmap-2026-07-16/BLUEPRINT-P16-product-ui-rebuild.md` (Phase-4 `compose` "hard
  blocker" → wiring task).
- **Vision:** `docs/design/living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md`;
  memory arcs `physics-ui-capture-quantum-math-arc-2026-07-14`, `field-ui-engine-arc-2026-07-13`.
- **Blocker:** P06 `key_V` — memory `sovereign-architecture-19-phase-roadmap-2026-07-17.md`.
- **Ground truth:** `GROUND-TRUTH-2026-07-17.md` (apps/packages grep-0).
