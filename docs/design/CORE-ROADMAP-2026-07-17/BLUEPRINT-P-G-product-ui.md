# BLUEPRINT P-G — Product/UI on kernel (Wave 2, 2026-07-17)

> Wave-2 phase blueprint for **P-G** of `CORE-ROADMAP-STANDARD-2026-07-17.md` (§3 row P-G),
> written against the §2 twenty-point contract. **Primary grounding: the Wave-1 audit
> `P-G-audit-product-ui-post-decommission.md` — adopted, not re-derived.** Its corrected scope
> statement governs this document: P-G is a **greenfield build-out of the `web/` beachhead toward
> the no-DOM physics-UI vision** — NOT a migration (there is no old app to migrate from, commit
> `5675c349b`) and NOT a backend re-plumb (none exists, none is wanted: `web/` is backend-less /
> local-first through the wasm kernel; audit §4).
>
> **Branch note (verified this pass):** live HEAD is `f01f9bb6b` on `feat/p19-growth-engine`
> (the session header's `feat/harness-llm-backend` snapshot is stale). All cites below were
> re-verified against this working tree, per STANDARD §2 item 1.

---

## 0. Scope (honest, binding)

Three gaps, three fixes, in this phase — nothing more:

| Gap (audit §5-§6) | Fix (this blueprint) |
|---|---|
| **G1** — `compose_field`/`FieldSim` exist in the sibling `wasm/` crate but `web/` never calls them | §4: wire `FieldSim` into `app.mjs` + the `#cv` canvas (exact call sequence) |
| **G2** — only 3 of 24 kernel `_js` exports bound in `kernel_client.mjs` | §5: bind the remaining 21, order/money first (`place_order_js`, `apply_event_js`, `estimate_order_total_js`) |
| **G3** — `app.mjs` is console-only; zero DOM; product pages = 0 | §6: first real DOM pass — populate the existing cards, live `#tick`, ONE minimal order surface |

**Explicitly OUT of this phase** (deferred with owner, per P16 §2 Step 3 discipline): the 26-page
inventory, sq/en/uk i18n recovery, WCAG-AA + semantic-DOM mirror, Sea & Sheet full design language,
the address-picker/photo-capture flows — all remain `BLUEPRINT-P16-product-ui-rebuild.md` +
`LIVING-INTERFACE-ROADMAP.md` scope. **Money charge-authority flip — explicitly gated, NOT here**
(§8). This is a UI-BUILD phase: minimal but REAL product surface, not a full redesign.

---

## 1. Ground truth (STANDARD §2.1 — every claim verified this pass, live tree)

Adopted from the Wave-1 audit and **re-verified fresh**, plus two new findings (1.g, 1.h):

- **a.** `web/` = 7 tracked files, zero dependencies (`web/package.json` has no
  `dependencies`/`devDependencies` key). `web/src/app.mjs` (35 lines) is console-only: binds the
  kernel, calls exactly `spectral_radius_js` / `fsm_graph_report_js` / `geo_progress_flat_js`,
  logs, `process.exit(1)` on failure (`app.mjs:18-34`). Zero DOM references (whole file read;
  no `document`/`canvas`/`addEventListener`). **Audit G3 confirmed current.**
- **b.** `web/src/lib/kernel/kernel_client.mjs` binds **3 of 24** exports (`:56-66`), with a
  fail-closed multivalue decoder `decodeRet` (`:49-55`) and a hand-rolled wasm-bindgen-0.2.95
  runtime copied from `wasm/demo/pkg/dowiz_wasm.js` (`:2-3`). **Audit G2 confirmed current.**
- **c.** `kernel/src/wasm.rs` exports 24 `_js` entry points (grep this pass: `place_order_js:284`,
  `apply_event_js:296`, `estimate_order_total_js:408`, `boot_verify_fsm_js:326`,
  `channel_ledger_js:307`, `reduce_anomalies_js:315`, `fsm_graph_report_js:424`, 10× `geo_*`,
  6× `spectral_*`, `harmonic_centrality_js:723`). JS-boundary orders reconstruct
  `price_trusted: false` — fail-closed (`wasm.rs:156-158`).
- **d.** The field capability ships TODAY in the sibling crate: `wasm/src/lib.rs:56-59`
  `compose_field`, `:64-109` stateful `FieldSim` (`new`/`step`/`frame`/`width`/`height`),
  `:111-114` `vertex_field`; determinism proven at `:203-211` (`wasm_compose_deterministic`) and
  `:289-301` (`wasm_fieldsim_deterministic`); liveness at `:263-284` (frame evolves, bytes finite).
  Built artifact tracked: `wasm/demo/pkg/{dowiz_wasm.js,dowiz_wasm_bg.wasm}`; headless smoke
  `wasm/demo/smoke.mjs` steps a `FieldSim` 30× (`:12-16`). **Audit G1 confirmed current.**
- **e.** `web/serve.mjs` `ROOT` is the **repo root** (`serve.mjs:9` — inline `// web/` comment is
  wrong, audit already flagged), `.wasm` MIME correct (`:16`), `/pkg-web/*` → `kernel/pkg-web/*`
  (`:25-26`). Consequence: **`/wasm/demo/pkg/dowiz_wasm.js` is already servable with zero
  serve.mjs changes.**
- **f.** `web/src/lib/kernel/kernel.test.mjs` = 4 tests / 5 asserts (spectral value, malformed
  fail-closed, fsm ok, geo ok) — the W17 green gate. **This phase EXTENDS it, never replaces it.**
- **g. NEW:** `kernel/pkg-web/dowiz_kernel.js` (3.5 KB, read in full this pass) contains **only
  init machinery — zero per-export shims** (`import * as __wbg_star0 from 'wbg'`, `__wbg_init`,
  `initSync`; no `place_order_js` wrapper exists in it). A naive implementer who tries
  `import { place_order_js } from '.../dowiz_kernel.js'` fails. The ONLY working binding path is
  the `kernel_client.mjs` hand-rolled ABI pattern (raw `wasm.<export>` + `passStringToWasm0` +
  `decodeRet`). §5 is written for that reality.
- **h. NEW:** `boot_verify_fsm_js` (`wasm.rs:319-331`: *"Call once at web-kernel init, before any
  order is placed"*) is **never called** by `web/` today — the fail-closed boot gate exists but is
  unwired. Fixed in §6 (W-3 step 1).

---

## 2. Correction of P16's stale Phase-4-dependency claim (explicit, required)

`BLUEPRINT-P16-product-ui-rebuild.md` §1/§4/§9 and acceptance item 4 claim the Sea layer has a
**"hard dependency on Phase 4's `compose` export"** (*"verified absent from `wasm.rs` today"*).
**That claim is STALE and is corrected here:** the grep was file-scoped to `kernel/src/wasm.rs`;
the capability already exists, is git-tracked, compiled, and determinism-tested in the sibling
`wasm/` crate (ground truth 1.d). P16's own §10.1 appendix and the Wave-1 audit §5.1 both
established this; this blueprint operationalizes it: **the Sea render is a wiring task in
`web/app.mjs` (§4), not a cross-phase math dependency. Nothing in P-G waits on Phase 4.**

Two related P16 carry-overs, resolved for this phase only:
- **Svelte tension (P16 §10.2b):** `web/` stays **zero-dep vanilla JS/DOM** — the zero-dep choice
  was made deliberately twice in repo history; adopting any framework requires its own DECART and
  is not needed for §6's scope. (STANDARD §2.19 reuse-first.)
- **"20 assertions" figure:** stale everywhere it appears; the real count is 4 tests / 5 asserts
  (ground truth 1.f). This document uses only re-counted figures.

---

## 3. The UI–kernel boundary spec — predefined types (STANDARD §2.3, §2.4, §2.8)

**Spec precedes test precedes code.** New file `web/src/lib/kernel/contracts.mjs` (zero-dep,
JSDoc-typed — the honest "predefined types" form for a no-TS surface; the **authoritative** types
remain the Rust structs in `wasm.rs`, and each JS typedef cites the struct it mirrors):

| JS typedef (contracts.mjs) | Mirrors (authoritative) | Shape |
|---|---|---|
| `KernelResult<T>` | `decodeRet` contract (`kernel_client.mjs:49-55`) | `{ok:true, value:T} \| {ok:false, err:string}` |
| `OrderOut` | `wasm.rs:163-175` `order_to_out` | `{id, customer_id, status, items:[{product_id, modifier_ids, quantity, unit_price}], subtotal, total, created_at_ms, channel, cash_pay_with}` |
| `EstimateOut` | `wasm.rs:364-373` | `{fee_known, delivery_fee, tax_total, total, min_not_met}` — `tax_total/total` may be `null` (overflow degrade, never a fabricated zero) |
| `ProgressOut` | `wasm.rs:439-444` | `{remaining_m, snapped:{lat,lng}, segment_index}` |
| `LedgerOut` | `wasm.rs:109-119` | `{orders_by_channel:[[ch,n]], funnel:{ch:[[status,n]]}, anomalies}` — BTreeMap ⇒ deterministic key order |
| `FsmReport` | `wasm.rs:412-415` doc | `{vertices, edges, is_acyclic, cyclomatic, spectral_radius, reachable_from_pending, reachable_states, topological_len}` |
| `SpectralFlat` | `wasm.rs:736-744` layout doc | `[rho, gap, fiedler, drift_code, n, re₁, im₁, …]` flat array |

**Validators (fail-closed):** one `validate<T>(parsedJson)` per typedef — field presence, type,
and for every money field `Number.isSafeInteger(x)` — returning `KernelResult`, never throwing.
A shape mismatch renders the ERR state, exactly like a kernel rejection. This is the "typed
wrapper around each `_js` export's JSON contract, not raw untyped JSON passing."

**FSM-legality rule (load-bearing):** the UI holds **no transition table**. Status *names* are a
display constant; **legality is decided only by `apply_event_js`** — the UI offers candidate
actions and renders the kernel's `Err` on an illegal press (kernel = sole FSM authority; a JS
status chart would violate D4 and the grep gate, §7). *Named optional follow-up (kernel-side, not
DoD here): an `fsm_edges_js` export so the UI can gray out illegal actions — until then illegal
actions are offered and correctly refused.*

**Scaling axes (§2.8):** (i) JSON-string exports scale with payload size and are **forbidden
inside the rAF loop** — per-frame data uses the flat protocols (`geo_progress_flat_js`,
`spectral_flat_js`) and `FieldSim.frame()` bytes; the JSON seam is for event-rate calls (user
actions, ≤ tens/sec). (ii) Money integers cross serde→JSON→JS `Number`: exact iff
`|amount| ≤ 2^53−1` minor units — the validator's `isSafeInteger` check IS the boundary, and a
violation degrades (ERR), never rounds. (iii) `FieldSim` cost is O(w·h·steps): sim runs at
reduced resolution (§4), scaling knob = sim resolution, breakpoint = when 1 step + blit exceeds
the frame budget (§10 bench).

---

## 4. G1 fix — wire `FieldSim` into `app.mjs` (the Sea beachhead)

**Exact call sequence** (all verified against `smoke.mjs:2-16` and `wasm/src/lib.rs:74-109`;
zero `serve.mjs` changes needed per ground truth 1.e):

1. **Import the field crate's own glue** (unlike the kernel, `dowiz_wasm.js` DOES export shims):
   `import initField, { FieldSim } from '../../wasm/demo/pkg/dowiz_wasm.js';` — the relative
   specifier resolves in BOTH environments (browser URL `/wasm/demo/pkg/…` via repo-root serving;
   Node file path).
2. **Init, environment-split** (mirrors `app.mjs:5-13` `loadBytes`): browser → `await initField();`
   (default fetch of `dowiz_wasm_bg.wasm` relative to the glue, correct MIME per `serve.mjs:16`);
   Node → `await initField({ module_or_path: bytes })` with `readFileSync` bytes (Node fetch has no
   `file://`, `smoke.mjs:4-7`).
3. **Order of operations:** `bindKernel(bytes)` FIRST (math authority), then kernel calls produce
   the scene, then `initField`. Kernel wasm and field wasm are **two separate instances** — the
   isolation boundary of §9.
4. **Build the sim from kernel-derived state:**
   `const sim = new FieldSim(new Float64Array(circles), SIM_W, SIM_H);` with `SIM_W=200, SIM_H=40`
   (¼ of the 800×160 `#cv`, `index.html:25`). `circles` = flat `[cx,cy,r,…]` derived from kernel
   outputs only (e.g. the `geo_progress_flat_js` snapped position and route endpoints mapped to
   sim coordinates) — a **display projection of kernel values, zero new math in JS**.
5. **Render loop (browser only):** per `requestAnimationFrame`:
   `sim.step();` → `const rgba = sim.frame();` →
   `off.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(rgba), sim.width(), sim.height()), 0, 0);`
   → `cvCtx.drawImage(off, 0, 0, 800, 160);` (offscreen canvas at sim resolution, scaled blit).
   Wrapped in `try/catch`: a throw stops the loop and paints the static-gradient fallback (§9) —
   the same degrade path P16 §4 defines for reduced-motion, so it is not throwaway work.
6. **Headless path (Node, keeps CI green):** no DOM — step 30×, assert `frame().length ===
   SIM_W*SIM_H*4` and step-30 ≠ step-0 (exactly `smoke.mjs:12-16`'s proven pattern).
7. **`#tick` (`index.html:26`)** advances the demo courier parameter and re-derives circles from
   fresh kernel geo calls, then one `sim.step()` + blit when the rAF loop is off (reduced-motion).

---

## 5. G2 fix — bind the remaining 21 exports in `kernel_client.mjs`

**Method (dictated by ground truth 1.g):** there is no generated per-export glue to copy — every
binding follows the existing hand-rolled pattern (`kernel_client.mjs:56-66`): encode string params
with `passStringToWasm0`, call the raw `wasm.<export>`, decode with `decodeRet`. Each binding is
proven by a RED→GREEN test (§7) with a known value — **the ABI is verified by test, never assumed.**

**ABI variants to handle (from the live signatures, `wasm.rs`):**
- `Option<String>` params (`place_order_js` `customer_id`/`channel`): pass `[0,0]` for
  null/undefined, else `passStringToWasm0` — confirmed by the known-value test (native oracle:
  `wasm.rs:789-798` `place_order_round_trip`).
- `i64` params (`estimate_order_total_js` `subtotal`; `geo_is_out_of_order_js`): raw wasm i64
  requires **`BigInt`** at the JS call site — wrapper accepts a safe integer, converts, documents.
- `Result<u64,_>` return (`reduce_anomalies_js`): non-string multivalue layout; **low priority**
  because `LedgerOut.anomalies` already carries the same number through `channel_ledger_js`'s JSON
  (`wasm.rs:109-119`) — bind it last, or consume anomalies via the ledger.
- Plain `f64`/no-arg/usize+String: same as the three existing wrappers.

**Priority waves (each lands separately, previous tests stay green):**
- **Wave A (unblocks cart/checkout/lifecycle — the audit's top order):** `place_order_js`,
  `apply_event_js`, `estimate_order_total_js`, **plus `boot_verify_fsm_js`** (ground truth 1.h —
  the boot gate must be callable before any order call).
- **Wave B (product-adjacent):** `channel_ledger_js`, `spectral_flat_js` (one call fills the whole
  spectral card, §6), `geo_haversine_js`, `geo_lerp_js`, `geo_eta_js`, `geo_is_arriving_js`,
  `geo_should_snap_js`, `geo_progress_js`.
- **Wave C (completes the surface):** `geo_bearing_js`, `geo_point_in_polygon_js`,
  `geo_is_out_of_order_js`, `spectral_eigenvalues_js`, `spectral_radius_js` (already bound),
  `spectral_gap_js`, `spectral_algebraic_connectivity_js`, `spectral_classify_drift_js`,
  `harmonic_centrality_js`, `reduce_anomalies_js`.

Every wrapper returns `KernelResult` through the §3 validator for its output type. The frozen
signatures of the 3 already-bound exports do not change (never-break-userspace, §11).

---

## 6. G3 fix — the first real DOM pass (minimal but REAL product surface)

**W-3 work unit, in `app.mjs` (DOM code guarded by `typeof document !== 'undefined'` so the Node
headless path and all existing tests keep running unchanged):**

1. **Boot gate:** after `bindKernel`, call `boot_verify_fsm_js()`; on `Err`, render a fatal banner
   and refuse to enable any order action (fail-closed boot, `wasm.rs:319-331`).
2. **Populate the existing debug cards** (`index.html:21-39`): `#rem`/`#snap`/`#seg` from
   `geo_progress_js` (`ProgressOut`); `#rho`/`#gap`/`#fie`/`#drift` from ONE `spectral_flat_js`
   call; `#fsm`/`#acyc` from `fsm_graph_report_js` (`FsmReport`). ERR state renders the `.bad`
   class with the kernel's error string — cards never stay `–` and never throw.
3. **Live `#tick` + canvas:** §4 steps 4-7 (the Sea beachhead on `#cv`).
4. **ONE minimal order surface** (a new card, same zero-dep style): fixture item list → **Place
   order** → `place_order_js` → render `OrderOut.status` + money line from
   `estimate_order_total_js` (integer minor units, formatted for display only — formatting is
   display, not math) → candidate status actions → `apply_event_js` per press → status advances on
   `Ok`, kernel error rendered on `Err` with state unchanged. Money **never tweens** (snap render;
   FE-09/DZ-02 red-line carried verbatim).

That is the complete G3 scope: cart-less fixed-item placement + full lifecycle + kernel-priced
display. Everything beyond it is P16's inventory, deferred with owner (§0).

---

## 7. Spec/event-driven TDD plan + DoD (STANDARD §2.2, §2.3, §2.5, §2.17)

**Order:** contracts.mjs typedefs (§3) land first → tests RED → bindings/wiring turn them GREEN.
Lifecycle tests assert on **event sequences** folded through `apply_event_js` (the kernel's own
decide/fold law), not just end-state. **All tests extend `kernel.test.mjs` — the existing 4 stay
untouched and green.**

| # | Test (RED before, GREEN after) | Asserts |
|---|---|---|
| T5 | `place_order_js` round trip | `ok`, `status==="PENDING"`, `subtotal === Σ qty·unit_price` (oracle: `wasm.rs:789-798`) |
| T6 | Event sequence fold | PENDING → CONFIRMED → … folded stepwise via `apply_event_js`; each step validates as `OrderOut` |
| T7 | **Adversarial:** illegal transition | e.g. DELIVERED → PENDING ⇒ `{ok:false}`, prior state object unchanged |
| T8 | `estimate_order_total_js` known cfg | exact integer total; `min_not_met` behavior |
| T9 | **Adversarial:** overflow degrade | near-i64-max subtotal ⇒ `tax_total: null` rendered as degrade, never a fabricated 0 (`wasm.rs:364-373`) |
| T10 | **Adversarial:** malformed order JSON | `apply_event_js('not json', …)` ⇒ `{ok:false}` (fail-closed seam) |
| T11 | `boot_verify_fsm_js` | `"OK"` on the live graph |
| T12 | FieldSim headless liveness | 30 steps: `len === w·h·4`, step-30 ≠ step-0 (mirrors `smoke.mjs:12-16`) |
| T13 | FieldSim determinism at the JS boundary | two sims, N steps, bit-identical bytes (mirrors `lib.rs:289-301`) |
| T14 | **Chaos:** corrupt wasm bytes | flip one byte of the field-crate bytes ⇒ init throws ⇒ app takes the static-fallback path and the kernel/order path still completes (bulkhead proof, §9). Kernel-bytes corruption ⇒ fail-closed exit (authority loss is fatal by design) |
| T15 | Validator fail-closed | a hand-built wrong-shape "order" ⇒ validator returns `{ok:false}` (the UI can never render unvalidated kernel-shaped data) |
| G | **Grep gate (intentionally-failing falsifier):** deny client math (`haversine`, eigen, hand transition tables, arithmetic on price/total/fee identifiers) AND raw `wasm.` access outside `kernel_client.mjs`, over `web/src/**` excluding the seam + tests. Plant one violation ⇒ RED; remove ⇒ GREEN (formalizes the prose check in `kernel.test.mjs:2-4`; P16 §3 design, honest boundary: a grep, not a theorem) |

**DoD:** T5–T15 + G green; the original 4 tests green; 24/24 exports bound and each covered by at
least one known-value assert; cards + tick + order surface function in a real browser (manual pass
recorded; optional Playwright smoke via the existing tooling is a bonus, not a gate — no new dep).
**Regression tracking:** T7, T9, T10, T14 registered permanently in
`docs/regressions/REGRESSION-LEDGER.md` (append rows in its existing format, named
`PG-illegal-transition`, `PG-money-overflow-degrade`, `PG-malformed-order`, `PG-field-bulkhead`).

---

## 8. Money / RLS safety (Batch 9 note — carried verbatim in intent)

- **Dual authority stands; this phase does NOT flip write/charge authority.** The kernel money
  surface is a **display mirror** by its own declaration (`wasm.rs:334-336`: the server fee ladder
  *"stays authoritative for what is CHARGED; this mirror drives what the client SEES"*). Batch 9
  (`18-BATCH9-…-audit.md:87-90`, §B.4 T2) is explicit: collapsing to one kernel authority is
  **money red-line, bit-identical parity-gated, and never flipped before the display leg is proven**
  — and `CORE-ROADMAP-STANDARD` §3 row P-G lists the flip as *"explicitly gated, not Wave 1."*
  This blueprint binds and displays; it changes no authority.
- **Post-decommission honesty:** with `apps/api` deleted there is currently no live charge path on
  this branch at all — which makes flipping authority *tempting* and is exactly why the gate
  matters: the flip is a separate, operator-gated decision tied to the P06 `key_V` product-safety
  story (audit §6 P06 note), not a side effect of UI wiring.
- **Fail-closed price trust:** every order crossing the JS boundary is reconstructed
  `price_trusted: false` (`wasm.rs:156-158`). The UI renders integer minor units only; **no RLS
  surface exists or is created in this phase** (no DB, no tenant boundary in `web/`).

**Hazard argument (STANDARD §2.6, structural not prose):** the unsafe state S* = "UI shows a
money/legality value not produced by kernel math." Reaching S* requires (i) client arithmetic on
money/geo/FSM — machine-caught by gate G; or (ii) bypassing the seam — raw `wasm.` outside
`kernel_client.mjs`, also caught by G; or (iii) rendering unvalidated kernel-shaped data —
unrepresentable through §3's validators (T15). Residual: a kernel-side bug, owned by the kernel's
own parity tests, outside UI reach.

---

## 9. Isolation, rollback, self-healing — stated as math (STANDARD §2.11, §2.13)

- **Bulkhead:** kernel wasm and field wasm are **two separate WebAssembly instances with disjoint
  linear memories** — a field-crate trap cannot corrupt kernel state by construction. Failure
  polarity is directional: **authority failure (kernel) fails closed** (no order surface); 
  **presentation failure (field) degrades open to the static-gradient canvas** with cards and the
  order surface alive (T14).
- **Snapshot Re-entry (the claimed recovery class):** `FieldSim` state is a pure function of
  `(circles, step_count)` — proven bit-deterministic (`lib.rs:289-301`) — so recovery =
  re-instantiate + re-step: cheap regenerative recovery from inputs, no stored frames.
- **Self-Termination (invariant boundary):** `decodeRet`'s `{ok:false}` + §3 validators make
  "render untrusted data as trusted" unrepresentable at the seam — a hard boundary, not a
  supervisor decision.
- **Self-Healing (redundant/error-correcting): NOT claimed** — no redundancy exists in this layer;
  saying so honestly per §2.13.

---

## 10. Mesh, telemetry, benchmarks (STANDARD §2.10, §2.12)

- **Mesh:** everything in P-G is **node-local**. The only network I/O is fetching the two wasm
  binaries at boot (audit §4). No gossip, no transport dependency, zero hot-path payload budget.
  A sync peer remains a future async concern (P-B/P-D territory).
- **Benchmarks:** measure on the dev box and record real numbers in the execution report:
  (a) `FieldSim.step()+frame()` at 200×40 (target ≤ ~4 ms; the O(w·h·steps) knob in §3 governs);
  (b) blit cost; (c) one JSON seam call (`place_order_js`) end-to-end. Before/after = headless
  timing of the §4 loop vs. nothing (greenfield: "before" is the absence, so record absolute
  numbers as the new baseline).
- **Telemetry hook:** a perf line on the page (last step ms / frame ms / fps EMA) rendered like
  any other card — regressions show up on every open, not only at review. Value-based tests stay
  the hard gates (timing asserts are advisory — no flaky time-based CI).

---

## 11. Standard-compliance closeout (remaining §2 items, explicit)

- **§2.7 Links:** grounded on `P-G-audit-product-ui-post-decommission.md`; builds toward
  `BLUEPRINT-P16-product-ui-rebuild.md` + `living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md`
  (whose §8 operator ruling — commercial-delivery-first, order path never waits on decorative
  phases — this phase's minimal-order-surface scope directly serves); safety per
  `18-BATCH9-product-layer-kernel-wasm-migration-audit.md`; memory:
  `physics-ui-capture-quantum-math-arc-2026-07-14`, `field-ui-engine-arc-2026-07-13`,
  `rust-engine-rewrite-arc-2026-07-13`, `sovereign-architecture-19-phase-roadmap-2026-07-17`.
- **§2.9 Linux discipline** (verdict framework from `BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-
  ADOPTION-2026-07-17.md`, reused): *never break userspace* → EXTENDS (3 bound signatures frozen;
  all new bindings additive); *incremental patches* → REINFORCES (waves A/B/C land separately,
  suite green between); *no regressions* → ALREADY-EQUIVALENT (W17 gate stays green throughout).
- **§2.14 Smart index:** the bug class this phase can introduce = Rust-struct ↔ JS-consumer JSON
  drift → caught at CI time by the known-value tests (T5-T11) + validators (T15); client-math
  reintroduction → caught by gate G. Both are CI-time, not runtime surprises.
- **§2.15 Living memory:** order state is event-shaped and derived by folding through
  `apply_event_js` — no flat mutable store; page-lifetime only. Durable event-log persistence is
  P-B's seam (cross-ref `internal-retrieval-living-memory-arc-2026-07-14`), named, not built here.
- **§2.16 Tensor/spectral:** pure reuse — `spectral_flat_js` flat protocol + `FieldSim` are the
  already-built machinery; no new closed-form math is authored, so no `eqc-rs` involvement (stated
  honestly rather than performed).
- **§2.20 Hermetic principles** (`HERMETIC-ARCHITECTURE-PRINCIPLES.md`): **P1 Mentalism** — §3's
  contracts derive from the Rust structs (spec is source; JS mirrors are cited derivations);
  **P2 Correspondence** — one concept, one primitive: kernel is the sole math authority, gate G
  makes a parallel JS implementation a CI failure; **P3 Vibration** — the render rate has a single
  named authority (the rAF loop driving `FieldSim.step`; JSON seam explicitly excluded from it);
  **P4 Polarity** — the seam's two poles (`ok`/`err`) collapse safe-directed: every ambiguity
  collapses to `{ok:false}`; **P6 Cause-and-Effect** — determinism as law: T13 pins bit-identical
  frames at the JS boundary; **P7 Gender** — no self-certified done: every claim has a paired
  RED→GREEN falsifier (T5-T15, G) plus the recorded manual browser pass.

---

## 12. Agent execution order (STANDARD §2.18 — zero-prior-context executable)

| Unit | Files | Do | Accept when |
|---|---|---|---|
| **W-0** | `web/src/lib/kernel/contracts.mjs` (new) | §3 typedefs + validators | T15 RED→GREEN; file has zero imports |
| **W-1a** | `web/src/lib/kernel/kernel_client.mjs`, `kernel.test.mjs` | Bind Wave A (§5) following the `:56-66` pattern; write T5-T11 first (RED) | T5-T11 GREEN; original 4 tests GREEN |
| **W-2** | `web/src/app.mjs` | §4 call sequence exactly; keep the headless path | T12-T14 GREEN; `node web/src/app.mjs` exits 0 |
| **W-3** | `web/src/app.mjs`, `web/index.html` (order card only) | §6 DOM pass | Manual browser pass: cards live, tick animates `#cv`, order places + advances + refuses illegal, money snaps |
| **W-1b/c** | `kernel_client.mjs`, `kernel.test.mjs` | Waves B, C with one known-value test each | All 24 bound, suite GREEN |
| **W-4** | `kernel.test.mjs` (gate G), `docs/regressions/REGRESSION-LEDGER.md` | Gate + ledger rows | Planted violation ⇒ RED; removal ⇒ GREEN; 4 rows appended |

Constraints binding every unit: no new dependency (any exception ⇒ DECART); never edit
`kernel/src/wasm.rs` or `wasm/src/lib.rs` in this phase (UI-build only; kernel follow-ups are
named, not smuggled); never touch money authority (§8); `kernel.test.mjs` is extended, never
replaced.

---

*Wave-2 blueprint for P-G. Sources verified this pass (live HEAD `f01f9bb6b`): all of `web/`
(`app.mjs`, `index.html`, `serve.mjs`, `kernel_client.mjs`, `kernel.test.mjs`, `package.json`),
`kernel/src/wasm.rs` (export surface + money/FSM/geo/spectral contract bodies),
`kernel/pkg-web/dowiz_kernel.js` (read in full — init-only, no shims), `wasm/src/lib.rs` (in
full), `wasm/demo/smoke.mjs`, `docs/regressions/` (ledger exists). Grounding docs read in full:
`CORE-ROADMAP-STANDARD-2026-07-17.md`, `P-G-audit-product-ui-post-decommission.md`,
`BLUEPRINT-P16-product-ui-rebuild.md` (incl. §10 appendix), `LIVING-INTERFACE-ROADMAP.md`
(incl. §8 ruling). This document plans; it changes no product code.*
