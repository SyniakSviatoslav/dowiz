# BLUEPRINT — Phase 16: PRODUCT UI REBUILD (deterministic wasm Sea & Sheet + i18n + WCAG)

> One phase of the 19-phase master roadmap (`R2-MERGED-PHASE-ROADMAP.md` §2, row 16).
> **Anchors:** D4, E12, E41, E43, E44, F49. **Depends on:** Phase 4 (kernel graph-math + wasm
> exports, incl. `field_frame::compose`), Phase 13 (delivery-on-protocol order/PoD/payout spine).
> **i18n gate hosted by** Phase 1's CI (SCOPE RULE: canonical-repo DEV-TIME fence, not a runtime
> control). **Parallel-safe with** Phase 14, 15.
> Execution substrate: the `DOWIZ-INTERFACES-PLAN.md` + `BLUEPRINTS-DOWIZ-INTERFACES.md` DZ-01..12
> work units on the FE-01..17 field-UI engine. This document is the phase blueprint; it writes and
> edits **no UI code**.

---

## 0. Framing (load-bearing, read first)

This is **not a re-plumb of an existing product. It is a REBUILD.** The entire live product surface
was deleted:

- `79ef316f6` + `db766de47` (2026-07-13, "remove legacy JS/TS thin-layer, kernel is now sole source
  of truth") deleted **`apps/web`** (Storefront/Admin/Courier SPA), **`packages/ui`** (including ALL
  of i18n), **`packages/domain`**, **`packages/shared-types`**.
- `fce5738b0` quarantined `apps/api`, `apps/worker`, `packages/db`, `fly.toml` into `attic/`.
- **Verified this session:** `git ls-files 'apps/*'` returns **0 files at HEAD** and **0 on
  `origin/main`**. `apps/web/` on disk holds only stale `dist/` + `node_modules/`.

The honest framing R1-D fixed and this phase inherits: **rebuild-on-kernel-and-mesh with
feature-inventory preservation.** Nothing that historically shipped may be silently lost. Two docs
already on disk are the behavioral **oracle** and the reconciliation **ledger** — they are used
directly here, not re-derived:

1. `DeliveryOS-As-Built-Summary-v1.md` — what shipped (audit 2026-06-04: 92 unique Playwright tests
   ×3 breakpoints, 18 screens + 4 map components, the L0-L11 order lifecycle, cash cycle, courier
   flow, owner admin, anonymizer/GDPR).
2. `docs/design/dowiz-interfaces/{DOWIZ-INTERFACES-PLAN, BLUEPRINTS-DOWIZ-INTERFACES,
   RESEARCH-CONSPECT}.md` — the **26-page inventory** with per-screen "no feature is lost" master
   checklists, the Sea & Sheet design language, and the DZ-01..12 falsifiable work units.

**Anchor merges that bind this phase (from R2 §1):** E12 ≡ F49 — the i18n rebuild is **one build**
serving both the general locale gate and the courier-app locale requirement; there is no separate
courier-i18n. E43 (web-first responsive) and E44 (WCAG-AA) are **not later passes** — they are
page-level acceptance criteria on every rebuilt page. E42's referent (`bebop2/delivery-domain`) is
Phase 13's, not re-anchored here.

**SCOPE RULE stamp (ARCHITECTURE.md §0):** every CI gate defined below — the wasm-math grep gate,
the i18n parity gate, the WCAG/axe gate — applies to **the canonical repo + operator's own build
ONLY**. It is a DEV-TIME fence. At runtime every hub is a sovereign Hydra (M5/M9/M11): it may fork
this UI, ship any locale set, ignore any gate, or replace the shell wholesale. None of these gates
is a control over what an autonomous hub chooses to ship.

---

## 1. Current-state evidence (what exists, what is recoverable, what is absent)

**What already exists and is green (the substrate this phase builds ON):**

- The kernel-driven `web/` shell. `web/README.md` states the invariant verbatim: *"This shell never
  re-implements geo/spectral/FSM math in JS/TS. The kernel is the single source of truth;
  `src/lib/kernel/kernel_client.mjs` only decodes the kernel's flat-bridge protocol and fails closed
  on malformed input."* Its test file `web/src/lib/kernel/kernel.test.mjs` carries the 20 kernel-math
  ("VbM" = verify-by-math) assertions (W17). The shell today renders geo route-snap, spectral drift
  ρ/γ/λ₂, and the FSM signature — a **math debug page, not a product page.**
- The wasm math surface. `kernel/src/wasm.rs` exports **24 `_js` entry points** — `place_order_js`,
  `apply_event_js`, `estimate_order_total_js`, `fsm_graph_report_js`, the `geo_*_js` family, the
  `spectral_*_js` family. These are the only sanctioned source of money/geo/FSM arithmetic for the
  UI.

**What is ABSENT (the gap this phase closes):**

- **Zero product pages** on the shell. The 26-page inventory lives only in the design docs.
- **`field_frame::compose` is NOT exported from `wasm.rs`** (grep for `compose`/`rgba` in `wasm.rs`
  returns 0 this session). The deterministic RGBA physics frame — proven bit-identical across runs by
  `engine/src/field_frame.rs:299-323` (`compose_returns_deterministic_frame`) — **cannot reach a
  canvas yet.** This export is **Phase 4's deliverable** (R2 row 4: "`compose` reachable from node
  via wasm with bit-identical frames across two runs"). Phase 16 **consumes** it; it does not build
  it. This is the hard dependency edge on Phase 4.
- **Zero i18n.** The canonical `web/` UI is `lang="en"`, no framework, no locale files, no gate
  (R1-E §E12, grep-verified across web/kernel/engine).
- **Zero accessibility work.** No product markup exists, so nothing to audit yet (R1-D §E43/E44).

**What is recoverable from git (asset, not rewrite):**

- The full i18n catalog. **Confirmed working this session:**
  `git show db766de47~1:packages/ui/src/lib/i18n-catalog.ts` and
  `git show db766de47~1:packages/ui/src/lib/i18n.ts`. Structure: `Locale = 'sq' | 'en' | 'uk'`
  (Albanian is `sq`, Ukrainian `uk` displayed "UA"), **default `sq`**; the catalog is **key-major**
  and the single source of truth; `i18n.ts` derives the locale-major view via `fromCatalog`;
  `t()/translate()` warn loudly on a missing key in dev and fall back to English/raw key in prod.
  R1-D/R1-E audit it at **~1291 catalog keys, ≈631 fully populated per locale** — these strings were
  already paid for. **Honesty note:** the exact populated count must be **re-counted from the
  recovered file** during the rebuild, not trusted from any prompt or memory (the source docs
  themselves cite "1291", "631×3", inconsistently).
- **Honesty note on the parity script:** the catalog header claims *"Enforced by
  `scripts/i18n-parity.mjs`"*, but `git show db766de47~1:scripts/i18n-parity.mjs` returns *does not
  exist in db766de47~1* — the exact historical script is **not recoverable at that rev**. The parity
  **contract** is fully specified by the catalog header ("every key must carry en+sq+uk, no TODO
  drafts"), so the gate is **rebuilt from the contract**, not restored byte-for-byte. Do not claim to
  have "recovered" the script.

---

## 2. Feature-inventory reconciliation methodology (the anti-silent-loss mechanism)

The done-test is a **reconciliation pass**, not a page count. Methodology:

**Step 1 — Build the canonical feature ledger.** Extract every shipped capability from the two
oracles into a single tracked file, `docs/design/dowiz-interfaces/RECONCILIATION-LEDGER.md`, one row
per feature. Sources, in precedence order: (a) `DeliveryOS-As-Built-Summary-v1.md` §1 capability
table + §9 test-coverage table (the 92 tests enumerate real behaviors) + §7 GO/NO-GO areas; (b) the
per-screen master checklists in `DOWIZ-INTERFACES-PLAN.md` §7 and `BLUEPRINTS-DOWIZ-INTERFACES.md`
DZ-07/08/09 (which are already reconciled against the deleted code — e.g. modifier groups, OTP flow +
full error matrix, WS live tracking + 30s watchdog, SwipeToComplete resets-on-failure, owner
lifecycle actions, dormant AllergenEditor publish-gate). Each row: `feature | source-screen | oracle
citation | disposition | evidence`.

**Step 2 — Enumerate the target pages against the checklist (do not trust the prompt's "26").**
Per the checklist the count decomposes as **3 client** (MenuPage, CheckoutPage, OrderStatusPage) +
**7 courier** (Login, Invite, Shift, Tasks, Delivery, Earnings, History) + **16 owner** (Shell,
Dashboard/Orders, MenuManager, Analytics, CRM, Promotions, Branding, Settings, Couriers,
Onboarding/Activation, Auth/AuthCallback, Supplies, FlowTest, …) = **26**. Role *shells* (client
storefront chrome, courier BottomTabBar, owner 11-nav) are chrome that wraps pages, not separate
pages. **The exact enumeration is settled by counting the checklist, not this blueprint** — if the
audited count differs from 26, the ledger is authoritative and the delta is recorded.

**Step 3 — Assign every feature a disposition, exactly one of three:**
- **REBUILT** → maps to a specific rebuilt page + the DZ unit that carries it + a passing test.
- **DROPPED** → an explicit, dated, **operator-signed** row in the ledger with a written reason.
  Nothing reaches DROPPED without operator sign-off. Silent absence is a RED reconciliation.
- **DEFERRED-WITH-OWNER** → carried by another phase (e.g. PoD *backend* → Phase 13; splat
  reconstruction → Phase 17; CRDT cross-device sync → explicitly deferred per DZ-06 out-of-scope).
  The row names the owning phase so it is visibly not lost, merely relocated.

**Step 4 — Reconcile the known redesign flags** (DOWIZ-INTERFACES-PLAN §7.3 "⚠️" list) as ledger
rows with dispositions, not as loose notes: (a) the **4 legacy money-tween sites**
(ClientLayout/EarningsPage/DashboardPage/AnalyticsPage) → REBUILT as `<Money>` snap (§4); (b) the
**dormant AllergenEditor publish-gate + ReadinessIndicator** → REBUILT and wired live; (c) **branding
3/10 tokens** → REBUILT as the 5-token T1 editor; (d) **supplies localStorage-only** → REBUILT on the
local-first event-log; (e) **courier `rating` read + no-presence map** → reconciled against
NO-COURIER-SCORING (a policy question flagged to the operator, since the *display* of a rating may
itself violate the structural gate — DROPPED-or-REBUILT is an operator ruling, recorded).

**Reconciliation is falsified** iff any oracle feature has no disposition, OR any DROPPED row lacks
an operator signature. The ledger is a living CI-checkable artifact for the phase's life.

---

## 3. Wasm-first UI architecture + CI grep-gate design (operationalizes D4)

**The architecture.** Every page follows the `web/` shell pattern: DOM/Svelte view layer for
structure and text; **all money, geo, and FSM arithmetic is a call into `kernel/src/wasm.rs` via
`kernel_client.mjs`**, which decodes the flat-bridge protocol and fails closed on malformed input.
The view layer holds **zero** business arithmetic. This is D4's "deterministic physics/math wasm"
and V2's Trait-as-Port discipline enforced at the UI boundary.

Concretely, the boundary maps to existing exports: order placement → `place_order_js`; every
lifecycle transition → `apply_event_js` (the FSM is the kernel's, never a JS state chart); totals /
fees / tax → `estimate_order_total_js`; the tracking stepper's vertex/edge/acyclic signature →
`fsm_graph_report_js`; courier route-snap / distance / arrival → the `geo_*_js` family; the Sea
render → `field_frame::compose` once Phase 4 exports it. **`<Money>` renders the integer-cent value
the kernel returns and has no tween prop** — count-up animation is structurally unreachable
(DZ-02, plan §2.5).

**The CI grep gate (canonical-repo DEV-TIME fence, hosted by Phase 1).** A deterministic script
(e.g. `scripts/ci-no-client-math.sh`) added to `.github/workflows/ci.yml` that fails RED if the
rebuilt UI tree contains client-side money/geo/FSM arithmetic. Design:

- **Deny patterns** over `web/src/**` (view + client glue only, excluding `kernel_client.mjs` and
  `*.test.*`): arithmetic on price/total/fee/tax/cents identifiers; `Math.*` geo primitives
  (`haversine`, `sqrt`+lat/lng, bearing math); hand-rolled transition tables / status switch-maps
  that decide legality; number-animation on price nodes (the `<Money>` no-tween ESLint rule, DZ-02).
- **Allow only** the single decode seam (`kernel_client.mjs`) and pure presentation (formatting a
  kernel-returned integer into a locale string is display, not math — see §5).
- **RED→GREEN falsifier for the gate itself:** plant one line of client-side fee arithmetic on a
  probe branch → CI RED; remove it → GREEN. The gate must catch a *planted* violation, mirroring
  Phase 1's probe-branch discipline.

Honest boundary: the gate is a **grep**, not a theorem — it catches the common re-implementation
patterns, not an adversarial obfuscation. That is sufficient for a DEV-TIME fence on the operator's
own build; it is explicitly not a runtime guarantee about any hub.

---

## 4. Sea & Sheet layer design (references Phase 4's `compose` export)

The design language is fixed by `DOWIZ-INTERFACES-PLAN.md`: two layers stacked per screen, every
role, every brand.

- **Sea (ambient layer, dowiz-owned, brand-tinted).** One continuous field under the content on
  every screen: `M Ü + Γ U̇ + c²L U = S`. It carries **arrival**, **transitions** (dive / sheet-rise),
  **tracking** (amplitude + colour mature terracotta→gold with `OrderStatus`), **feedback** (Green's
  function: tap→ripple, success→heat bloom, error→high-λ shake), and **focus** (a single potential
  well `V(x)` driving scale/brightness/blur). **This layer is rendered by Phase 4's
  `field_frame::compose` RGBA export blitted to a canvas.** Phase 16 wires the export to the canvas
  and drives its source term `S(t)` from kernel `OrderStatus`; it does **not** implement the
  integrator (that is `engine/src/field_frame.rs`, deterministic, proven). The tracking page
  (`OrderStatusPage`, Act 3) is where the Sea "matures" — the most distinctive dowiz moment (DZ-04).
- **Sheet (content layer, brand-owned, crisp).** The opaque surface that rises over the Sea and holds
  decisions: menus, cards, forms, tables, words, prices. Rendered by the SDF content pipeline
  (FE-05/06). Owns brand identity via the 5 T1 tokens.
- **Spectral edge (the seam).** The single dowiz signature: an oklch `terracotta→accent→gold`
  gradient on every Sheet rim, ring-wipe of every dive, tracking thread; speeds up when the surface
  is attending (6s→1.4s).

**Three-act universal shell (DZ-01).** arrive → choose → receive = three URL states with working
back-navigation, inherited by all three roles (client `/s/:slug` → menu → track; owner `/admin` →
manage → act; courier `/courier` → tasks → deliver). Reduced-motion degrades the Sea to a static
tinted gradient with state still legible via pills/colour/text (a coherence rule, §6).

**Determinism boundary.** The Sea is generous motion; **money never tweens** and lives on the Sheet
in `<Money>`. The field↔state boundary (FE-09) means the Sea *presents* kernel-decided integers and
*cannot* interpolate them — enforced three ways (boundary + no-tween component + ESLint rule), which
is what retires the 4 legacy money-tween sites by construction.

**Dependency honesty:** until Phase 4 lands the `compose` export, the Sea layer is **blocked** — the
pages can be built with a static-gradient Sea fallback (which is also the reduced-motion path, so it
is not throwaway work), and the live field render switches on when the export arrives. The GPU/video
tier is **not** in this phase (Phase 17 owns wgpu + video after GPU-unlock); the Sea here is the
CPU-composed deterministic RGBA frame, WebGL2/canvas-blit, no wgpu dependency.

---

## 5. i18n recovery + parity-gate design (E12 ≡ F49)

**Recover, don't rewrite.** Step 1 is `git show db766de47~1:packages/ui/src/lib/i18n-catalog.ts >
<target>` to recover the ~1291-key, ≈631-populated-per-locale catalog verbatim (sq/en/uk). These
translated strings are a paid-for asset; the phase **extends and audits** them, it does not author
them from scratch.

**Serving-layer decision (flag to operator / DECART).** The two R1 reports diverge on mechanism:
- **R1-D §F49:** "Zero-dep JS lookup is fine (i18n is display, not math — D4's no-JS-math invariant
  is untouched)."
- **R1-E §E12:** "Rust-native locale table (std-only preferred; any new dep ⇒ DECART per E62), wired
  into web/ wasm UI."

Both agree the **strings recover from git** and that i18n is **display, not arithmetic** — so it does
**not** cross the §3 wasm-math gate either way. The reconciliation: recover the catalog as a
locale-agnostic key-major data table; serve it to the `web/` shell as static data. Per the
rust-native-default rule and R1-E's preference, the **default target is a std-only Rust-native locale
table** compiled into / shipped alongside the wasm surface; a zero-dep JS key-major lookup is the
acceptable fallback, and **any new dependency triggers a DECART report**. The choice is recorded in
the phase's DECART log; the strings port identically regardless.

**Parity gate (rebuilt from contract, hosted by Phase 1's CI).** A deterministic script (e.g.
`scripts/i18n-parity.mjs` or a Rust equivalent) that fails RED unless **every catalog key carries all
three locales** (en + sq + uk), with **no TODO/draft placeholders** — the exact contract stated in
the recovered catalog header. Design:
- Load the key-major catalog; for each key assert presence + non-empty for `sq`, `en`, `uk`.
- Emit the offending `key × locale` cells on failure (dev-loud, mirrors the historical `translate()`
  dev-warn behavior).
- **RED→GREEN falsifier:** deliberately remove one key from one locale → gate RED; restore → GREEN
  across all three. This is a literal done-test item (§8).
- **SCOPE RULE:** this is a **canonical-repo DEV-TIME blocking gate ONLY** (ARCHITECTURE.md §5). A
  hub may ship any locale set it wants; the gate never blocks hub autonomy.

**E12 ≡ F49:** there is exactly **one** i18n build. The courier app consumes the `courier.*`
namespace of the same catalog; no separate courier-i18n exists or is built.

---

## 6. WCAG-AA + responsive as page-level acceptance (E43/E44, not a later pass)

Accessibility and responsiveness are **acceptance criteria attached to each rebuilt page**, gated per
page, not a trailing cleanup sprint. R1-D §E43/E44 fixed this framing: both are "properties of pages
that don't exist yet."

**The hybrid-DOM accessibility architecture (DZ-11, FE-15).** A canvas-first UI is invisible to a
screen reader, so the design mandates a **parallel hidden semantic DOM mirror**: dishes as real
`<button>`s with `role`/`aria-label`/`tabindex`, reconciled per-frame from the field widget list; a
transparent `<input>` overlay for forms (preserving IME/autofill/mobile-keyboard, `type=email/tel`);
and the public `/s/:slug` SSR menu stays real DOM. This is architecture, not a temporary shim
(DZ-11 contract). Permanent losses (e.g. browser Ctrl+F over canvas text) are **documented**, not
hidden.

**Per-page WCAG-AA acceptance.** Each rebuilt page must pass an automated **axe (or equivalent) scan
with zero critical WCAG-AA violations**, added as a CI job (canonical-repo DEV-TIME fence). Coverage
targets the historical a11y baseline the As-Built summary already fixed (the "40 a11y lint errors"
remediation, the `MapLibreBase.tsx` `textContent` XSS fix, ErrorBoundary): contrast (the oklch token
system must validate AA — DOWIZ-INTERFACES-PLAN §8.1 already flags "AA validated" tokens), focus
order, keyboard operability of the semantic mirror, `aria-live` for order-status changes, and
reduced-motion (coherence rule 9: reduced-motion never loses meaning — the Sea degrades to a static
state that remains legible).

**Responsive acceptance (E43, web-first).** Each page passes a **viewport matrix** — the historical
suite ran **3 breakpoints** (mobile-first, 77% mobile market); the rebuild re-institutes at least the
same three (mobile / tablet / desktop) plus the owner **split-pane** density behavior (φ₂φ₃ spectral
layout collapsing to a single column below the tablet breakpoint, DZ-01/plan §6) and the courier
`position:fixed` bottom-bar embed fix (As-Built P1-14). The Playwright E2E loop (§8) runs across the
breakpoint matrix, matching the historical 92×3 discipline.

---

## 7. Address-picker + courier photo-capture UI (backend-ownership handoff to Phase 13/17)

Both flows ship as part of the **courier-facing (and checkout) pages** here, because this is where
they have a UI to live in. **This phase builds their UI surface only; the backend logic is owned
elsewhere and must not be duplicated here.**

- **Address-picker v1 (checkout `MapWithPin` + "My Location", and the GS §2.6 pin/floor UI).** The
  UI is already fully specced in the Gaussian-Splatting synthesis (GS P0.1 six-item acceptance:
  pin-drop / floor-slice / open-space degrade / <1° bearing / 0-360 seam / LOS rectangle) — **reuse
  verbatim, do not re-derive.** The picker consumes the **six GS geo functions that Phase 4 exports**
  through `wasm.rs` (`storey_height_m` … `los_clear`); the UI never computes geometry itself (§3
  gate applies). Backend handoff: the picked address **feeds Phase 13's PoD geo-signal** and **Phase
  17's splat-reconstruction bootstrap**. This phase renders the picker and emits the captured
  pin/floor/bearing; it does **not** own splat reconstruction (Phase 17) or PoD verification
  (Phase 13).
- **Courier photo-capture flow (`DeliveryPage` entry-photo → thumbnail → fullscreen; checkout
  entrance-photo).** One capture flow, **two consumers** (R1-D §F42/§0): the photo is (a) PoD
  evidence and (b) the splat-bootstrap image set. Phase 16 builds the **capture, preview, retake, and
  local-first hold** (photo queued in the outbox, uploaded when `navigator.onLine` — DZ-06); it does
  **not** build PoD signing/verification (Phase 13, edge ML-DSA multi-signal k-of-n) or the
  `SplatReconstructionJob`/Modal pipeline (Phase 17). The UI emits a signed-frame-ready capture
  artifact to the Phase 13 boundary and nothing more.

**Handoff contract (explicit, to prevent scope bleed):** Phase 16 delivers UI + captured payload at a
typed boundary; Phase 13 owns PoD/settlement/order-spine backend; Phase 17 owns splat/GPU/video.
Any PoD or settlement *math* rendered on these pages traces to a kernel/mesh call, never a client
re-implementation (§3).

---

## 8. Acceptance criteria (numbered checklist; the Playwright E2E loop is item 5)

The phase is **done** iff every item below is demonstrably GREEN (RED→GREEN shown where a falsifier is
named). Items 1-9 are the falsifiable done-test; 10-12 are the standing invariants.

1. **Feature-inventory reconciliation GREEN.** `RECONCILIATION-LEDGER.md` exists; every feature drawn
   from `DeliveryOS-As-Built-Summary-v1.md` + the DOWIZ-INTERFACES master checklists has exactly one
   disposition (REBUILT / DROPPED / DEFERRED-WITH-OWNER); **no oracle feature is undispositioned**;
   every DROPPED row carries a dated **operator signature** and a written reason. *Falsifier:* an
   undispositioned or unsigned-drop row fails the reconciliation check.
2. **All target pages rebuilt on the shell.** The page count matches the checklist enumeration
   (3 client + 7 courier + 16 owner = 26, or the audited count with the delta recorded); each page
   maps to its DZ unit and carries a passing test.
3. **Wasm-math grep gate GREEN, and RED on a planted violation.** `scripts/ci-no-client-math.sh`
   passes on the clean tree; a planted line of client-side money/geo/FSM arithmetic on a probe branch
   turns it RED; removal returns GREEN. Every such calculation in the shipped tree traces to a
   `kernel_client.mjs` → `wasm.rs` call.
4. **Sea layer live off Phase 4's `compose`.** Once Phase 4 exports `field_frame::compose`, the Sea
   canvas renders **bit-identical RGBA frames across two runs** of a fixed scenario (inherits
   `compose_returns_deterministic_frame`); before the export lands, the static-gradient/reduced-motion
   Sea fallback renders and is the acceptance the export later upgrades.
5. **Playwright E2E full-loop GREEN against the wasm UI on a real mesh hub (not a mock).** One order
   traverses **client places order → courier captures PoD → owner sees settlement**, driven end-to-end
   by the wasm UI over a **real Phase-13 mesh hub**: client builds a cart (modifier groups, live
   `<Money>` from `estimate_order_total_js`) → checkout (address-picker pin, OTP flow, idempotent
   place via `place_order_js`) → order folds to a signed frame on the hub → courier accepts the task,
   navigates (geo route-snap), captures the delivery photo, SwipeToComplete emits the PoD →
   `apply_event_js` advances the FSM to `Delivered` → owner dashboard shows the settled order and the
   payout entry. Runs across the ≥3-breakpoint viewport matrix. *Falsifier:* any leg failing, or the
   loop passing only against a mock, fails the item.
6. **i18n parity gate GREEN ×3, RED on a removed key.** With the recovered catalog fully populated,
   the parity gate is GREEN across sq/en/uk; deliberately removing one key from one locale turns it
   RED; restoring returns GREEN.
7. **Accessibility scan clean.** An automated axe (or equivalent) scan reports **zero WCAG-AA
   critical violations** across every rebuilt page; the hidden semantic-DOM mirror is
   screen-reader-navigable; reduced-motion keeps state legible.
8. **Responsive matrix GREEN.** Every page passes the ≥3-breakpoint viewport matrix; owner split-pane
   collapses correctly; courier bottom-bar embed fix holds.
9. **Redesign-flag reconciliations landed.** The 4 money-tween sites are `<Money>` snap (no-tween
   ESLint rule GREEN, tween attempt RED); AllergenEditor publish-gate wired live; branding 5-token T1
   editor live; supplies on the local-first event-log; NO-COURIER-SCORING rating/no-presence question
   resolved by operator ruling and recorded.
10. **Determinism invariant (D4):** zero client-side money/geo/FSM arithmetic anywhere in the shipped
    UI (item 3, standing).
11. **Money invariant:** `<Money>` is mono + tabular + integer-from-kernel + never-tween + never-round,
    everywhere money is shown (DZ Appendix B rule 2, standing).
12. **F50 no-central-service invariant:** no rebuilt flow reintroduces a mandatory central service —
    the E2E loop (item 5) must complete on a **solo hub with zero non-hub services running**
    (local-first render loop never touches the server on the hot path; server/mesh = async sync peer
    via outbox, DZ-06).

---

## 9. Honest register (flags carried out of this phase)

- **Hard dependency on Phase 4's `compose` export** (verified absent from `wasm.rs` today) and on
  Phase 13's real order/PoD/payout spine — the live Sea render and E2E item 5 cannot pass without
  both; the static-Sea fallback keeps pages buildable meanwhile, item 5 forbids a mock.
- **i18n serving-layer DECART** — Rust-native table (R1-E, default) vs zero-dep JS lookup (R1-D);
  recorded, not silently chosen. The **historical parity script is not recoverable** at
  `db766de47~1`; the gate is rebuilt from the catalog contract.
- **NO-COURIER-SCORING vs displayed rating** — whether showing a courier rating violates the
  structural gate is an **operator ruling**, flagged, not resolved here.
- **Exact page/key counts** ("26" pages, "1291"/"631" keys) must be re-counted from the checklist and
  the recovered catalog, never trusted from prompt or memory.
- **All gates are SCOPE-RULE DEV-TIME fences** — canonical repo only; no gate here controls what any
  autonomous hub ships.

---

*Blueprint for Phase 16 of the R2 master roadmap. Sources read in full: ARCHITECTURE.md,
R2-MERGED-PHASE-ROADMAP.md, R1-D §D-4 + phases D-1..D-5, R1-E §E12 + phase E-3,
DeliveryOS-As-Built-Summary-v1.md, DOWIZ-INTERFACES-PLAN.md, BLUEPRINTS-DOWIZ-INTERFACES.md.
Load-bearing facts (deletion commits, `git ls-files apps` = 0 at HEAD + origin/main, i18n catalog
recovery, `web/` shell + 24 wasm exports, `compose` absence) re-verified against the live tree
2026-07-16. This document plans; it changes no product code.*

---

## 10 — Planning-protocol completion appendix (2026-07-17, decorrelated pass)

> Independent grounding/DECART/doubt pass per `AGENTS.md` Detailed Planning Protocol + the 2-question
> ritual, run by an agent decorrelated from the one that wrote §0-§9. This blueprint entered with the
> weakest citation density of the three assigned (one line-numbered citation in 396 lines), so this
> appendix's priority — per its own assignment — is grounding, not merely auditing. Read-only against
> `/root/dowiz`; nothing edited outside this appendix. **Headline finding: §1/§9's "hard dependency on
> Phase 4" for the Sea layer's `compose` export is stale** — see 10.1.

### 10.1 — Citation verification + new grounding (the Rust UI-surface inventory this blueprint needed)

**The one existing citation is imprecise.** §1 cites `engine/src/field_frame.rs:299-323` for
`compose_returns_deterministic_frame`. Live read: `compose` itself is at line **193**
(`pub fn compose(scene: &Scene, eq: &FieldEquilibrium, w: usize, h: usize, steps: usize) -> Vec<u8>`)
and the determinism test is at lines **321-342**, not 299-323 — off by roughly twenty lines. The
substance holds (`assert_eq!(a, b, "compose must be bit-deterministic across calls")` is real and
present), so this is a citation-hygiene correction, not a substantive one, but it is the document's
*only* line-numbered citation and it was wrong — a fact worth weighing against how much else in this
document has no citation to check at all.

**The load-bearing correction: `field_frame::compose` is already reachable from JS today, via a
sibling crate this document never mentions.** §1 states *"`field_frame::compose` is NOT exported from
`wasm.rs` (grep for `compose`/`rgba` in `wasm.rs` returns 0 this session)... This export is **Phase 4's
deliverable**... Phase 16 **consumes** it; it does not build it. This is the hard dependency edge on
Phase 4."* The `wasm.rs`-scoped grep is accurate as far as it goes — but it is the wrong scope. A
repo-wide check finds `/root/dowiz/wasm/` (crate `dowiz-wasm`, `Cargo.toml` description: *"exposes the
engine field_frame.compose RGBA + VertexBridge graph-Laplacian field to JS via wasm-bindgen (zero TS
math)"*), which:

- exports `#[wasm_bindgen] pub fn compose_field(circles: &[f64], w: usize, h: usize, steps: usize) ->
  Vec<u8>` (`wasm/src/lib.rs:56-58`), a direct wasm-bindgen wrapper around `engine`'s `compose`;
- exports a stateful `#[wasm_bindgen] pub struct FieldSim` with `new`/`step`/`frame`/`width`/`height`
  methods for a live render loop (`wasm/src/lib.rs:64-111`);
- carries its own passing determinism test, `wasm_compose_deterministic` (mirrors the engine-level test
  exactly: two `FieldSim`s with identical inputs produce bit-identical frames);
- is **tracked in git at current HEAD** (`git ls-files wasm/` lists `wasm/Cargo.toml`, `wasm/src/lib.rs`,
  and a **built demo artifact** `wasm/demo/pkg/dowiz_wasm_bg.wasm` + `wasm/demo/smoke.mjs`, a headless
  Node smoke test that instantiates `FieldSim` and steps it 30 times) — this is not aspirational or
  work-in-progress, it is committed, compiled, and has a passing offline smoke test today;
- `web/src/lib/kernel/kernel_client.mjs`'s own header comment even says *"Runtime block copied verbatim
  from in-repo `wasm/demo/pkg/dowiz_wasm.js`"* — the `web/` shell's authors already knew this crate
  exists; they copied its wasm-bindgen glue boilerplate, but did not wire its `compose_field`/`FieldSim`
  exports into `app.mjs`. `grep -rn "compose_field|FieldSim|dowiz_wasm" web/src web/index.html` (excluding
  the one comment) returns zero calls.

**What this means for §1/§4/§9's dependency framing:** the *capability* Phase 4 is described as owing
(§1: *"`compose` reachable from node via wasm with bit-identical frames across two runs"*) **already
exists and already passes that exact bar**, in a different crate than the one this document grepped.
The real, narrower gap is that `web/`'s live JS shell does not yet call it — a wiring task, not a
cross-phase math dependency. §4's "Dependency honesty" paragraph and §9's "Hard dependency on Phase 4's
`compose` export (verified absent from `wasm.rs` today)" should read: *verified absent from
`kernel/src/wasm.rs` specifically; present and tested in the sibling `wasm/` crate; the remaining work is
wiring `web/app.mjs` to `wasm/demo/pkg/dowiz_wasm.js`'s `FieldSim`, not waiting on Phase 4.* This does not
mean Phase 4 has no remaining scope (it may still own other exports), but the Sea-layer-specific
blocker this document leans on throughout §4/§9/AC-4 is not accurate as written.

**A second stale figure, propagated across three documents.** §1 states web's `kernel.test.mjs` carries
*"the 20 kernel-math (VbM) assertions (W17)."* Live read: `kernel.test.mjs` is 32 lines with **4 test
cases / 5 `assert()` calls** (spectral radius, malformed-input fail-closed, fsm report, geo progress).
The repo's own retro doc, `docs/design/W22-RETRO-GOVERNANCE-2026-07-16.md:14`, independently confirms
this: *"`node web/src/lib/kernel/kernel.test.mjs` → **4 ok**... EXIT=0."* The "20 assertions" figure
appears not only here but also in `R1-D-product-on-protocol-gap-analysis.md:71` and
`BLUEPRINT-P17-demo-splat-gpu-unlock.md:25` — the same wrong number copy-pasted forward through the
pipeline. **A related, previously uncited gap:** `kernel_client.mjs` today binds only **3 of the
kernel's 24 `_js` exports** (`spectral_radius_js`, `geo_progress_flat_js`, `fsm_graph_report_js`) — not
`place_order_js`/`apply_event_js`/`estimate_order_total_js`, the exact functions §3/§8-item-5's checkout
and order-lifecycle flow will need to call. The "math debug page" (§1) is real but narrower than "the
kernel's wasm surface is wired" implies; most of the 24 exports are unbound in JS today.

**Deletion/quarantine chain — mostly matches, two refinements.** `git cat-file -t` confirms `79ef316f6`
and `db766de47` are both real commits with **identical diff content** (267 files, -48358/+278) and an
identical message — they are the same logical change on two refs (one an `origin/*` remote-tracking
pointer, one a local branch pointer not itself an ancestor of current HEAD), not two separate deletions
as "commits `79ef316f6` + `db766de47`" could be read to imply. Separately, `fce5738b0` (the quarantine
commit) is real, but **`attic/` was not left in a quarantined state** — a later commit
(`f9ab28ff1`/`e1505e1d9`, *"drop ALL JS/TS (per operator)... Remove entire JS/TS surface (web/, packages/,
spikes/, attic/, tools JS)..."*) fully deleted `attic/` (including the quarantined `apps/api`) the next
day, recoverable only via a named backup branch, `backup/pre-drop-js-20260715-161134` (confirmed to
exist via `git for-each-ref`). §0/§1 never claim `attic/` is retrievable today, so nothing here
contradicts the document's conclusion — but a reader could reasonably infer "quarantined" means
"still there, just out of the way," and it is not; `git ls-files 'attic/*'` = 0 and the directory does
not exist on disk. Worth one added sentence.

**i18n recovery — fully re-verified, exactly as claimed.** `git show db766de47~1:packages/ui/src/lib/
i18n-catalog.ts` and `...i18n.ts` both recover cleanly; `Locale = 'sq'|'en'|'uk'`, default `'sq'`,
key-major catalog confirmed verbatim. `git show db766de47~1:scripts/i18n-parity.mjs` correctly fails
("does not exist in db766de47~1") — the blueprint's "not recoverable at that rev" claim is accurate, not
just plausible.

**As-Built summary — mostly accurate, one stale label.** `DeliveryOS-As-Built-Summary-v1.md` confirms
"92 unique tests × 3 breakpoints = 276 total" and "Frontend React PWA (18 screens, 4 map components)"
verbatim. **"L0-L11" (§1, line 32) does not appear anywhere in the As-Built summary itself** — that
document instead describes a "10-state machine." The repo's own audit,
`docs/audit/2026-06-18/reliability-gate-SKILL-reconciled.md:18`, has already reconciled this exact label
downward: *"Stages L0–L11 → L0–L9 mapped to real code."* §1's "L0-L11 order lifecycle" phrase reuses a
label the repo's own governance has already superseded — a small but genuine staleness in the one
narrative section of this document that reads as most authoritative.

**Canon check (D4, SCOPE RULE).** The SCOPE RULE is verbatim at `ARCHITECTURE.md:23` as cited. **D4 is
not defined in `ARCHITECTURE.md`** (`grep -n "D4"` → zero hits there); it is defined at
`R1-D-product-on-protocol-gap-analysis.md:55` (*"D4 — Product UI determinism: dowiz UI = deterministic
physics/math wasm"*). §0's header does not explicitly claim D4 lives in `ARCHITECTURE.md`, so this is not
a contradiction, only a gap worth closing: a reader following "Canon is ARCHITECTURE.md" (a phrase this
document's siblings use) could look in the wrong file for D4's definition.

### 10.2 — DECART

**(a) i18n serving layer — a DECART is named but not executed; done here.** §5 correctly identifies the
choice (R1-D: zero-dep JS lookup vs. R1-E: Rust-native locale table) and correctly defers to "the
phase's DECART log," but no comparison table exists anywhere in this document or, as far as this pass
found, anywhere else in the roadmap.

| Option | For | Against |
|---|---|---|
| **Rust-native locale table, compiled into/shipped alongside the wasm surface (std-only)** | Zero new dependency; consistent with D4's "no JS math" ethos extended to data, not just arithmetic; one build artifact instead of two parallel lookup implementations | The catalog is currently TypeScript (`i18n-catalog.ts`) — porting ~1291 keys × 3 locales into a Rust-embeddable form (e.g. a build-time-generated `phf` map or a flat binary table) is nontrivial one-time work with no existing precedent in this repo |
| **Zero-dep JS key-major lookup (recovered catalog served as static JSON/JS, no framework)** | Minimal new work — the recovered `i18n-catalog.ts`/`i18n.ts` port almost directly; zero new Rust code; i18n is display, not arithmetic, so it never crosses the D4/wasm-math gate either way | Two parallel string-lookup implementations conceptually exist (kernel math is Rust/wasm, locale strings are JS) — not a *violation* of D4, but a design seam the Rust-native-default rule (per this assignment's hard constraint) would prefer collapsed |
| **CHOSEN — per the rust-native-default rule this assignment operates under: Rust-native locale table is the correct default; zero-dep JS lookup is the acceptable, lower-effort fallback if the port proves costly.** | — | Case against, honestly: this is a preference, not a proof — nothing in the recovered catalog or this repo demonstrates the Rust-native path is actually cheaper to build than it looks; if the practical port cost turns out high, the JS fallback is not a compromise, it is the correct call, and should be recorded as such rather than treated as second-best by default. |

**(b) The `web/` shell's assumed reintroduction of Svelte — a real, unresolved contradiction with the
project's own zero-dep precedent. Flagged with evidence; not resolved or rewritten here, per this
assignment's instruction.** §3 states: *"Every page follows the `web/` shell pattern: DOM/**Svelte**
view layer for structure and text..."* Live evidence bearing directly on this:

- `web/package.json` today has **no `dependencies` or `devDependencies` key at all** — it is a
  deliberately zero-dependency vanilla-JS shell (`"description": "Kernel-driven field UI — all
  geo/spectral/FSM math computed in the Rust dowiz-kernel wasm. This shell only renders."`).
- The original product frontend **was** Astro/Svelte-based, and its removal is the deletion this
  blueprint's §0 documents (`79ef316f6`'s own commit message: *"remove legacy JS/TS thin-layer"*,
  covering an Astro/Svelte `web/` predecessor per that commit's stat).
- After a same-day full JS/TS purge (`f9ab28ff1`, §10.1 above) deleted even the freshly-rebuilt shell,
  it was rebuilt **again**, the next day, as the current zero-dependency `web/` — i.e. the zero-dep
  choice for this exact shell has already been made, deliberately, **twice** in this repo's history.
- Introducing Svelte means introducing a JS compiler + runtime dependency (a real, new external tool
  choice) into a surface whose own `package.json` self-describes as dependency-free, and would be the
  **third** reversal of the same decision.

**This is exactly the tension this assignment asked to be surfaced, not silently resolved.** The
blueprint's design intent for a component-templating layer is legible and not unreasonable on its own
terms (Svelte is commonly chosen for exactly the DOM-reconciliation problem §6's "hidden semantic DOM
mirror" describes) — but it is asserted once, in passing, with no DECART, against a repo that has
twice chosen the opposite. Per this assignment's explicit instruction, this is recorded as a conflict for
the operator/next planning pass to resolve, not rewritten here: either (i) §3's "Svelte" is a loose word
for "component-templating discipline" and the actual implementation stays zero-dep vanilla JS/DOM
(consistent with precedent, no DECART needed), or (ii) Svelte is genuinely intended, in which case it
needs the same DECART table treatment as (a) above — including at least one rejected zero-dep
alternative (e.g., a small hand-rolled DOM-diffing helper, which is what the current `web/` shell already
implies it would need anyway for the semantic-DOM mirror in §6) — before this phase is buildable as
written.

### 10.3 — Two-question doubt audit

**Q1 — least confident about, concrete:**

1. **The Phase-4 dependency correction (10.1) is the load-bearing one** — I am confident `wasm/`'s
   `compose_field`/`FieldSim` exist, are tracked, and pass a determinism test matching the engine-level
   one; I am less confident about *why* `web/`'s shell doesn't already call them (a deliberate
   phase-sequencing choice? an oversight? abandoned mid-build?) — I did not find a comment or commit
   message explaining the gap, only its existence.
2. **The Svelte/zero-dep contradiction (10.2b)** is presented as a live tension, not resolved — I
   genuinely do not know which reading (loose word vs. literal dependency) the original blueprint author
   intended, and did not find internal evidence in §3-§8 pointing definitively either way.
3. **The "26 pages" checklist derivation (§2 Step 2)** — I verified the three source documents exist and
   sampled their content, but did not personally re-derive the 3+7+16=26 count against
   `DOWIZ-INTERFACES-PLAN.md`'s full per-screen checklist (a 39KB document); the blueprint's own text
   already says "the exact enumeration is settled by counting the checklist, not this blueprint," which
   is honest, but I have not done that count either, so it remains genuinely open.
4. **`kernel_client.mjs`'s narrow binding (3 of 24 `_js` exports)** — I confirmed the current state but
   did not assess how much work remains to bind the other 21 (some, like `place_order_js`, likely need
   richer JS-side plumbing than the three already bound simple functions); this could be a materially
   larger gap than "just wire compose in" implies for the rest of the wasm surface.
5. **The `attic/` full-deletion finding (10.1)** — I confirmed the backup branch
   `backup/pre-drop-js-20260715-161134` exists as a ref, but did not check it out or verify its contents
   are complete/intact; "recoverable via backup branch" is confirmed to the level of "a ref with that
   name exists," not "checking it out actually restores a working `apps/api`."
6. **The "20 assertions" stale figure's blast radius** — I found it copy-pasted in three documents but
   did not search the *entire* `docs/design/` tree for further copies; there may be more.
7. **I did not verify the DZ-01..12 work-unit documents' internal consistency** with this blueprint's
   §4/§6/§7 claims about them (Sea/Sheet, three-act shell, hybrid-DOM accessibility) — I confirmed the
   three source documents exist and sampled one-line summaries, not a claim-by-claim cross-check.

**Q2 — the biggest thing this pass might be missing:** the specific failure shape here — *"this
capability is described as blocked on a future phase, when a repo-wide (not file-scoped) check would
have shown it already exists"* — is **the same failure shape this roadmap's own
`SELF-CRITIQUE-2Q-DOUBT-AUDIT.md` §3 already found and formally investigated for llama.cpp/GPU-unlock**
(there: "self-host LLM" was bundled with a `cargo add wgpu` trigger that was actually about an unrelated
graphics crate; here: "Sea-layer `compose`" was bundled with "Phase 4's deliverable" when a sibling crate
already ships it). That prior investigation's own root-cause diagnosis — *"the error entered at the
canon and was reinforced rather than challenged at each downstream link"* — applies here nearly
verbatim: R1-D wrote the "absent, Phase-4-gated" framing, R2's phase table restated it, this blueprint
inherited it, and nothing in three passes re-ran the grep at repo scope instead of file scope. Given that
the roadmap has now made this exact mistake shape at least twice (once caught, once — until this pass —
uncaught), the biggest miss is not this one instance; it is that **there is still no standing check
(a script, a CI step, a "before citing X is absent, grep the whole repo not just the obvious file"
habit) that would catch the *next* instance of this same error automatically.** That is a process gap
this document cannot fix by itself, but it is the pattern a fresh reader would spot fastest: two
independent "assumed-blocked, actually-already-built" errors in one 19-phase roadmap, both caught only by
a decorrelated re-read, neither by the roadmap's own machinery.

### 10.4 — Anu & Ananke check

**Anu.** The "hard dependency on Phase 4" claim (§1/§4/§9) **fails Anu as written**: it is asserted from
a citation (`grep compose|rgba` in `kernel/src/wasm.rs` returns 0) that is true but insufficiently scoped
to support the conclusion drawn from it ("this export is Phase 4's deliverable... hard dependency edge").
A repo-wide grep — the same kind of check this document's own §0 uses correctly elsewhere (e.g. "`git ls
-files 'apps/*'` returns 0 files" is exactly the right scope for that claim) — would have surfaced
`wasm/`. This is the clearest single Anu failure found across all three assigned blueprints this pass
reviewed: a conclusion that does not survive being re-derived at the correct scope. The Svelte claim
(§3) similarly fails Anu in a smaller way — it is asserted, not derived from or reconciled against the
`web/package.json` evidence that was sitting one directory away and contradicts it directly.

**Ananke.** The feature-inventory reconciliation ledger (§2) is a genuine Ananke strength: it makes
"every shipped feature gets a disposition, every DROPPED row needs an operator signature" a structural
requirement (AC-1's falsifier), not a hope. The wasm-math CI grep gate (§3, AC-3) is a second real
structural win — a planted-violation RED→GREEN falsifier is exactly the kind of check whose own
existence forces the invariant, rather than relying on a future developer's diligence not to
reintroduce client-side math. But this appendix's own headline finding is itself an Ananke gap the
document doesn't name: **nothing in this blueprint's structure would have caught "Phase 4 dependency is
stale" before an implementer wasted real calendar time waiting on Phase 4** — there is no built-in
"before treating any capability as absent, re-grep the whole repo, not just the obvious crate" step
anywhere in §1's methodology or in the AC-4 acceptance criterion itself. Given that this exact failure
shape has now occurred at least twice in this roadmap (10.3/Q2), a standing mechanical habit or check
would move this from "caught by a decorrelated pass, this time" to "structurally can't happen" — which
is precisely the distinction Ananke asks a plan's own organization, not a future reader's care, to
guarantee.

---

*Appendix sources (2026-07-17): live grep/read against `/root/dowiz` HEAD `cc3d5c916`;
`engine/src/field_frame.rs` (lines 193, 321-342); `wasm/Cargo.toml`, `wasm/src/lib.rs` (lines 56-58,
64-111); `web/package.json`, `web/src/lib/kernel/{kernel_client.mjs,kernel.test.mjs}`;
`docs/design/W22-RETRO-GOVERNANCE-2026-07-16.md:14`; git history for `apps/web`/`packages/ui`/`attic/`
(commits `79ef316f6`, `db766de47`, `fce5738b0`, `f9ab28ff1`/`e1505e1d9`, branch
`backup/pre-drop-js-20260715-161134`); `DeliveryOS-As-Built-Summary-v1.md`;
`docs/audit/2026-06-18/reliability-gate-SKILL-reconciled.md:18`; `ARCHITECTURE.md:23`;
`R1-D-product-on-protocol-gap-analysis.md:55`; `SELF-CRITIQUE-2Q-DOUBT-AUDIT.md §3` (the analogous
llama.cpp/GPU-unlock precedent). No code or canon changed.*
