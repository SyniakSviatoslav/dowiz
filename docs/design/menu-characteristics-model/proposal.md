# Design Proposal — Menu Characteristics Model (FULL BUILD)

**Slug:** `menu-characteristics-model`
**Seat:** System Architect (Triadic Council). Design-time only — NO production code.
**Status:** DRAFT (FULL BUILD). **Extends, does not replace,** the RESOLVED v1 (taste-first detail-floor) — round-by-round history is preserved in `resolution.md`, `breaker-findings.md`, `counsel-opinion.md`, and the Accepted `docs/adr/ADR-menu-characteristics-model.md`. **Draft ADR for this build: `docs/adr/0014-menu-characteristics-model.md`.**
**System spine (repeats, LAW):** owner = authority for declarations · facts not a verdict · hide-but-never-fabricate · presence-not-absence for allergens · honesty-anchor = a real number must qualify · hot-path `read_public_menu` untouched.

> **What "FULL BUILD" means here.** v1 shipped exactly three dormant/floor surfaces behind `VITE_MENU_CHARACTERISTICS_ENABLED`: L1 taste chips (live), an L2 descriptive band (empty allowlist → renders nothing), and the allergen **detail-floor** (`MenuPage.tsx:1089-1144`). The FULL build un-defers the surfaces v1 held back — **populated L2 character badges on the card, the card allergen unit, two-dish comparison, the characteristic filter, and (the hardest) honest live recompute on modifier selection** — each behind its own sub-flag, each gated red→green, **without weakening a single v1 red-line.** Where the FULL build needs data that does not yet exist (modifier nutrition/allergen deltas, owner diet declaration, verified legal anchors), that data is named as an explicit prerequisite, not assumed.

---

## 1. Problem + non-goals

**Problem.** The raw facts already reach the storefront in `products.attributes` (`taste`, `bom[]` macros + allergens, `prep_time_minutes`, `modifier_groups`). v1 made them honest one-tap-away in the detail modal. Customers still cannot **scan** a menu through their own lens at a glance ("which is light?", "which has more protein?", "show me declared-vegan", "compare these two"), and the moment they add a heavy modifier the displayed character silently lies (a "light" base dish is no longer light). The FULL build delivers the at-a-glance layer, the two-dish comparison, and the filter — all on the same single pure derivation, so no two surfaces can disagree. **Live-honest recompute on modifier selection (Contract B) is DEFERRED** — modifiers carry no nutrition/allergen deltas today and delivering them honestly is itself a 🔴 hot-path concern (§6, FB-H1); v1 ships **Contract A** (suppress reassurance on modifiable dishes, never the warning).

> **STEP 0 — a standalone allergen safety fix lands first.** Re-grounding found that the live storefront *already* (a) runs a recipe-only allergen filter that ignores owner `declared_allergens` (`MenuPage.tsx:193-194`) and (b) renders a recipe-only card allergen chip (`:854`) and a quick-add path (`:865`) that diverge from the modal's `computeAllergenSurface` — two allergen red-line false-negatives **already in production**. Before, or independent of, any characteristics flag, **STEP 0** converges every allergen surface onto the single `computeAllergenSurface` function and removes the unguarded card chip (v1 = detail-floor-only). See §8 STEP 0 + guardrail #12; full disposition in `resolution.md` §"RESOLVE — FULL BUILD round".

**Non-goals.**
- Not a nutrition database, not a health/medical claim engine, not a recommender. Facts, not verdicts.
- Not a "winner" in comparison — no global ranking, no "healthier".
- Not deriving allergen ABSENCE — ever, on any surface, including by contrast.
- Not re-versioning the hot-path `read_public_menu` (`mig …072`, 🔴 TOTAL BLAST RADIUS) for a presentation concern.
- Not turning the owner into a verdict author: owner gets **HIDE**, never **threshold-tune** (§ Owner control).

---

## 2. Back-of-envelope (quantified)

Scale (live-grounded): demo = 49 products; cap **≤ ~200 products/location**, ≤ a few hundred active locations near-term. Menu read is already cached behind `read_public_menu` + SSR/CDN. The whole characteristics layer is a **pure function over the already-served payload** → **+0 DB connections, +0 queries, +0 endpoints** for everything except the one optional modifier-delta migration (which adds columns, no new runtime pool).

| Cost centre | Quantity | Cost |
|---|---|---|
| Products / menu | demo ≈ 50; cap ≈ 200 | bound |
| **Badge-compute / card** | fold `bom` (≤ ~15 lines) + ≤ ~6 threshold compares + curation rank | **O(bom) ≈ 20–100 ops, sub-ms**; already paid by `bomToNutrition` |
| Full-menu render | 200 cards × O(bom) | ≈ 4–20k ops, **< 1 ms**; no new cost vs today |
| **Modifier recompute** | per option toggle, **one open dish only** | O(bom + selected modifiers) ≤ ~30 ops, sub-ms; bounded by human tap rate (~1/s); never touches the menu grid |
| **Compare-view (mobile)** | exactly 2 dishes, one screen, single render | 2 × (≤15 bom + ≤5 taste + allergen union) ≈ 60 ops; trivial |
| **Filter pass** | one client predicate over loaded menu | O(products) ≤ 200 per toggle/keystroke, **< 1 ms**, zero backend |
| **i18n labels (fixed, sq/en/uk, hand-translated)** | 14 EU allergens + 5 taste axes + ≤6 reviewed descriptive + ~3–5 diet + ~8 reliance/marker/compare UI strings | **≈ 35–40 fixed strings**; no auto-translation (red line) |
| **Connection budget** | API + worker + analytics + migrations pools | **unchanged**; modifier-delta migration is additive columns on an existing FORCE-RLS table, no new pool |

---

## 3. The real choice — where the L2 derivation + thresholds come from (≥2 named options)

The v1 derivation **site** decision (Option B: pure client module, not `read_public_menu`) stands. The FULL-build question is narrower and sharper: **where do the threshold values + the characteristic set come from** once the band is populated and recomputed live?

- **Option A — Client-derived, hard-coded thresholds in the shared lib.** *Concept: pure projection at the edge + a single versioned config constant.* `deriveCharacteristics(product, selection)` is a pure TS function in `packages/ui/src/lib/characteristics.ts` (it already exists); descriptive thresholds + the closed allowlist live in a versioned `CHARACTERISTIC_RULES` constant; the **regulated** subset's numbers come from `REGULATED_ANCHORS` (a verified-legal-anchor table, human-supplied, today empty).
  *Tradeoffs:* one canonical implementation reused by card + compare + filter verbatim → surfaces cannot drift; iterating a descriptive threshold is a code change + a guardrail run, not a migration; no hot-path risk; thresholds are reviewable in a diff. Cost: a threshold change ships with a deploy (acceptable at this cadence; thresholds are near-static and legally anchored). **CHOSEN.**
- **Option B — Server-computed + cached characteristic set on the menu payload.** *Concept: derive-at-source / materialized read-model.* `read_public_menu` (or a sidecar fn) emits a precomputed `characteristics[]` per product.
  *Tradeoffs:* one server source of truth; but it re-versions a ~150-line SECURITY-DEFINER **🔴 TOTAL-BLAST-RADIUS** fn for a **sub-millisecond pure compute**, puts thresholds in SQL (hard to localize sq/en/uk, hard to iterate, hard to legally review), and the cache invalidates on every `bom`/modifier edit. It buys nothing the pure client fold doesn't already give, and the live modifier recompute is inherently client-state anyway (the selection lives in the open modal, not on the server). **Rejected** — over-engineering against back-of-envelope; "schema rich, runtime minimal" violated.
- **Option C — Owner-tunable thresholds.** *Concept: per-tenant threshold dials in the owner UI.*
  *Tradeoffs:* maximal owner flexibility — and it is **the wrong asymmetry, fatally.** A tunable "light below N kcal" dial lets the owner **manufacture a verdict** ("everything on my menu is light") with no number ever changing — exactly the honesty-anchor inversion the whole design forbids, and for the **regulated** subset it hands the legal-claim threshold to a marketing dial instead of the law. Owner authority is over **declarations** (allergen/diet) and over **HIDE**, never over the numeric bar that licenses a platform-rendered character. **Rejected** — breaks honesty-anchor + relocates the regulated-claim duty into a UI control.

**Decision: Option A.** Derivation is a pure function of the already-served payload + current selection, thresholds are versioned platform config (descriptive) or a verified legal-anchor table (regulated), and the owner's only lever is subtractive (HIDE). Comparison and filter are *views/seams over the same function* — never a second threshold source (a chip that says "protein-forward" and a comparison that disagrees is structurally impossible).

### 3.1 Where the thresholds actually come from (and the open item)
- **Descriptive labels** (e.g. `protein-forward`, `carb-forward`) are **relative-composition** statements, not regulated numeric claims: a label qualifies when one macro **dominates the dish's own energy mix by a notable margin AND clears an absolute floor** — e.g. "protein-forward" when protein-kcal is the largest share *and* the dish is not trivially small. These cuts are **platform distinctiveness thresholds**, deliberately phrased to stay on *dish description*, never on energy/satiety/health register (guardrail #6's closed allowlist). **They must be set by human review, not from memory** — the exact margins are a NEEDS-HUMAN product+nutrition value before the allowlist is populated.
- **Regulated labels** (`light`, `low-energy`, `source of protein`…) are **legal nutrition claims** whose numbers are **defined by EU Reg (EC) 1924/2006 Annex + the Albanian transposition** (per 100 g / per portion, NRV-referenced). 🔴 **These exact figures MUST be transcribed from the verified regulation text, NEVER reconstructed from memory** — `REGULATED_ANCHORS` carries a `citation` + `verifiedBy` and stays INERT until a human supplies a signed-off anchor. **OPEN ITEM (R-3) / FB-STOP-1 (NEEDS-HUMAN): the regulated subset stays red-on-disk OFF (guardrail #2) until a NAMED legal/Counsel signatory records anchors transcribed verbatim from the regulation text (`citation` + `verifiedBy`), owner opt-in authority is wired, and server-authoritative supplies (R-2) exist. No numbers are invented here.** Steel-man recorded: if it ever ships, emit a server-side audit log of each regulated claim (what/which dish/what data/which threshold version) — a sidecar, NOT a hot-path re-version.

---

## 4. Owner control — the hide/fabricate asymmetry (and where HIDE is stored)

The owner's lever is **subtractive only**, and the storage shape enforces it:

- **HIDE a derived badge — ALLOWED.** Stored as `products.attributes.char_hidden: string[]` — an **enum subset of the closed `DESCRIPTIVE_ALLOWLIST ∪ TASTE_AXES` set**, validated server-side (Zod `.strict`, string-only, every element ∈ that closed set). It rides the existing `jsonb` `attributes` column that owner CRUD already passes through (`products.ts`) → **no migration.** The subtraction happens in the **one shared derivation** every surface calls — `deriveCharacteristics`/`selectDescriptiveLabels` and `compareDishes` all take and apply `char_hidden` (FB-M2), so a hidden label can never resurface on the compare-view or filter. The owner can quiet a true "carb-forward" they dislike; the surface only ever loses a *real* descriptive/taste label. **Guardrail #9 (permanent ratchet, FB-STOP-3):** `CHAR_HIDDEN_VOCAB ⊆ (DESCRIPTIVE_ALLOWLIST ∪ TASTE_AXES)` AND `∩ ALLERGEN_TOKENS = ∅` (the 14 EU allergens are a disjoint namespace) AND `∩ REASSURANCE/FREE_FROM_TOKENS = ∅` (no absence/free-from vocabulary exists) AND `computeAllergenSurface` takes **no `char_hidden`** argument — so a subtraction can never touch a warning, and no label whose removal manufactures a reassuring absence is hidable. `char_hidden` rides the `jsonb` payload raw, which is safe (it can only ever hide a non-safety label).
- **ADD a badge the numbers don't support — FORBIDDEN by construction.** `char_hidden` can only *remove* from the derived set; there is no `char_added` field and no code path that unions an owner string into the rendered characteristics. A would-be false badge has **nowhere to live**.
- **Flip allergen presence → absence — FORBIDDEN.** `computeAllergenSurface` returns `{ known[], hasInfo }`; an owner "none" attestation yields `hasInfo:false` (→ the floor), **never** a "free-from" claim. There is no absence field, on purpose.
- **Regulated subset:** owner authority here is an **opt-in/confirm** (the food-business-operator *asserts* the legal claim), gated behind a verified anchor — it is *not* `char_hidden` and *not* a threshold dial.

Asymmetry restated: **HIDE is a subset operation on truth; ADD has no representation.** This is the data-shape encoding of "hide-but-never-fabricate".

---

## 5. Data / migrations (forward-only, additive, RLS FORCE, integer) — RED-LINE items flagged

| Item | Shape | Migration? | Notes |
|---|---|---|---|
| Owner **hide-list** | `attributes.char_hidden: string[]` (enum-subset, Zod-validated) | **None** — `jsonb` passthrough | subtractive-only; the asymmetry of §4 |
| **L2 character set / thresholds** | code config `CHARACTERISTIC_RULES` + `DESCRIPTIVE_ALLOWLIST` | **None** — not data | versioned constant, reviewable in diff |
| **Regulated anchors** | `REGULATED_ANCHORS` table-in-code (citation + verifiedBy) | **None** — not data, NEEDS-HUMAN to populate | red-on-disk OFF until populated (guardrail #2) |
| **Diet declaration** (vegan/halal/organic) | `attributes.diet: {vegan?,vegetarian?,halal?,organic?}` owner declaration | **None** — `jsonb` passthrough | new owner UI + liability copy + i18n; **genuinely new owner work** — defer track `…_DIET_DECLARATION` |
| **Modifier nutrition/allergen deltas** (for honest live recompute) | additive columns on the modifier-option table: `kcal,proteinG,fatG,carbsG int` + `allergens text[]` | **🔴 YES — forward-only additive migration in `packages/db/migrations/` (RED-LINE glob)** | table is tenant-scoped → **must keep RLS `ENABLE+FORCE`** (the modifier table already carries it; the migration must not regress it); integer macros (half-up, consistent with `RecipeEditor` rounding); gated behind `MODIFIER_NUTRITION_ENABLED` |

**RED-LINE discipline:** the modifier-delta migration touches `packages/db/migrations/` — it is forward-only, atomic, idempotent (**`ADD COLUMN IF NOT EXISTS` per column**, FB-L1), must re-assert `ENABLE ROW LEVEL SECURITY` + `FORCE ROW LEVEL SECURITY` on the altered table (belt-and-suspenders — `ADD COLUMN` does not drop RLS, but the migration asserts it per discipline), `down() = {}` (project forward-only convention), and must be applied to staging DB *before* the app boots (boot-guard FATAL-exits otherwise). It is the **only** schema change in the whole FULL build, and it is gated dark until owner data entry exists. **Coupling (FB-H1/FB-L1):** the columns are **INERT without a client delivery path** — they ship with the Contract B track together with the **lazy per-product detail fetch** (§6), never alone and never via a `read_public_menu` re-version. Every other surface is presentation over existing `attributes`. All additive `jsonb` keys are backward-compatible (unknown keys ignored).

---

## 6. Live recompute on modifier selection — the hardest honesty case

This is the surface the brief most wants ("a heavy add-on must drop *light*; an option can INTRODUCE an allergen") and the one where the data does not yet exist. Grounded fact: **modifiers carry only `price_delta`** (`mig 1780338982010:15-23`) — no nutrition, no allergens. Therefore there are **two honest contracts**, selected by `MODIFIER_NUTRITION_ENABLED`:

**Contract A — data absent (today, default).**
- `deriveCharacteristics(product, selection)` characterizes the **base dish only**.
- On any dish whose modifier groups *could* change the relevant axis, **every reassuring/positive label is SUPPRESSED, not captioned** — descriptive, regulated, diet, any "free-from". A "for the base dish" caption on a reassuring claim is a soft-confirm trap; suppression deletes only labels a modifier could *falsify*.
- **The allergen-presence WARNING is NEVER suppressed.** Modifiers carry no allergen data → a modifier can only *add* an allergen, never honestly *remove* a base one ("remove cheese" never clears cross-contamination) → a base-dish warning is always conservatively true. Suppression is asymmetric: it removes reassurance, never a warning. Taste chips remain (sensory, not a body claim). Price/prep recompute (already live) is untouched.

**Contract B — data present (`MODIFIER_NUTRITION_ENABLED`, after the §5 migration). DEFERRED — not in v1.**
> **Honest delivery cost (FB-H1).** `read_public_menu` (🔴 hot-path) emits modifiers as `{id,name,price_delta}` only and is the **sole** path modifiers reach the client. Contract B's recompute needs the §5 `kcal/protein/fat/carbs/allergens` deltas **in the client** — so it is **not** "additive columns" alone; it needs a **delivery path**. We do **NOT** re-version the hot path (that is the exact 🔴 fn whose blast radius rejected Option B in §3). **Delivery path (chosen): a lazy per-product detail fetch on modal-open** — the modal already lazily fetches `detailMedia` on open (`MenuPage.tsx:136`), so the deltas ride that same per-dish read, never the menu grid. The recompute is client-state in the open modal anyway, so a per-dish fetch suffices; the grid "+0 endpoints" budget and the hot-path spine LAW are preserved. Extending the media endpoint vs a sibling is decided in the Contract B ADR pass.
- Selection state lives in the open detail modal. On each toggle, the pure function recomputes:
  - **Macros** = base `bom` fold **+ Σ (selected option deltas)** → thresholds re-evaluate **live**. A heavy add-on that pushes the dish past the bar **drops** "protein-forward"/any descriptive label; a protein add-on may *introduce* a number-anchored label — honest, because the number now qualifies for the actual selection.
  - **Allergen surface** = conservative **UNION** (`base ∪ selected-option allergens`). Selecting an allergen-bearing option **introduces** that allergen live; **no selection can ever remove a base allergen** (presence-only, monotone-up; removal options don't clear the base warning).
- **Honesty monotonicity (the contract):** the function is pure & idempotent — same `(product, selection)` → same output. A reassuring label may only *qualify* against the real recomputed number; an allergen may only be *added* by selection, never subtracted. **Partial-data guard:** if **any** selectable option on the dish lacks a nutrition delta, the recomputed *reassuring* label is **suppressed** (can't honestly recompute from a hole); the allergen union still adds every *known* option allergen (conservative), and the floor + reliance bound cover the unknowns.
- **Cross-surface consistency:** comparison and filter operate on a **defined selection** (base, unless a selection is explicitly carried) and call the *same* function → a card chip and a comparison cell can never disagree.

---

## 7. Failures + degradation (failure-first)

Every input is optional and untrusted; degradation is designed before the happy path.

- **No `bom` / missing macro** → no L2 badge, no derived allergen (NEVER a guessed badge). Detail simply omits the section. **A missing-data card must never render anything that reads as "no allergens".**
- **Missing allergen data** (no `bom`, empty `bom[].allergens`, or read-gate strip) → the allergen surface renders the explicit **"allergen info not provided — ask the restaurant"** floor (`MenuPage.tsx:1139`), **unconditionally** (regardless of coverage / `allergens_confirmed`), exempt from the curation cap, and **never** "allergen-free" — including never **by contrast** with a sibling card's warning (the card allergen unit is ALL-OR-NOTHING: warning + marker render together or neither, guardrail #7).
- **Partial declaration** (`['milk']`, non-empty but incomplete) → the persistent, surface-attached **reliance bound** ("declared to contain… not a complete allergen list — confirm with the venue", EN+sq) is **always** attached wherever allergen content renders (guardrail #5d), so a partial list never reads as exhaustive ("milk only ⇒ no nuts" is forbidden).
- **No taste / no diet field** → no mini-profile / no diet chip; the "declared-vegan" lens shows an honest empty state, never a fabricated one.
- **Modifier data absent / partial** → Contract A / the partial-data guard of §6 (suppress reassurance, never the warning).
- **No `bom` for a comparison dish** → the cell renders the explicit floor marker, **never blank/"—"** (a blank beside "declared to contain nuts" reads nut-free by contrast — guardrail #8).
- **External calls:** none added. No timeout/circuit/cascade surface — a pure client function over already-fetched data. The whole layer flips off instantly via flag and reverts to today's behavior.

**Consistency + idempotency:** `deriveCharacteristics` / `computeAllergenSurface` / `compareDishes` are pure, deterministic, side-effect-free → trivially idempotent; one source of truth for card, compare, and filter.

---

## 8. Build order — STEP 0 (safety fix) + flag-gateable steps

STEP 0 is a standalone safety fix (no flag). Each later step is its own `VITE_` sub-flag, shippable dark and launched separately. The v1 `VITE_MENU_CHARACTERISTICS_ENABLED` already gates the detail-floor; the FULL build adds:

0. **STEP 0 — allergen single-source-of-truth (NO FLAG — lands first / independent).** Fixes two allergen red-line crossings already in production (FB-C1, FB-C2). The **unified-surface contract:**
   - **One function, every surface.** The menu **filter** (`MenuPage.tsx:194`), the **card** allergen prop (`:854`), the **quick-add** path (`:865`), the **detail modal** (`:1118`, already compliant), and (later) **comparison** cells ALL derive allergens from the single pure `computeAllergenSurface(attributes, bomAllergens)` → conservative `{ known: declared ∪ recipe, hasInfo }`. **`bomToNutrition().allergens` is NEVER read for a safety decision again** (macro display is not a safety read).
   - **Filter converged + honest.** Predicate becomes `computeAllergenSurface(p.attributes, bomToNutrition(p).allergens).known.includes(filterAllergen)` so a **declared-only** allergen dish is never dropped from a "contains X" view. The live filter is **NOT "deferred entirely"** — it ships; it carries the non-dismissible coverage disclosure + honest denominator, OR is gated OFF behind `…_ALLERGEN_FILTER` (the recorded keep-vs-gate human decision, FB-C1).
   - **No bare card warning in v1.** The unguarded recipe-only card chip (`:854`) is **removed** and folded under the gated all-or-nothing `…_CARD_ALLERGEN` unit (#7). **v1 = detail-floor-only**; v1 allergen truth lives at the detail floor + the #5d reliance bound. No card warning ⇒ no "clean by contrast" (R4-H1) and no "safe by omission" (FB-H2).
   - Gating guardrail: **#12** (single-allergen-source — no recipe-only allergen read; no bare card warning; declared-only dish retained + surfaced). Permanent ratchet.
1. **L2 BADGES on the card** (`VITE_MENU_CHARACTERISTICS_ENABLED`, extending v1).
   - Populate `DESCRIPTIVE_ALLOWLIST` with **human-reviewed** labels (EN+sq+uk register-cleared) — until then the band stays dormant (safe default).
   - Card **descriptive band** renders above a per-tenant **descriptive-coverage** gate (one denominator, descriptive-axis only — governs no allergen element). Curation 2–3, "only when notable" (margin past the anchor; borderline → silence).
   - **Card-gestalt invariants (FB-H2 / FB-STOP-2b):** (i) **presence coupling (#13)** — a card NEVER renders a reassuring descriptive badge unless the card's allergen unit is in a determinate state (warning OR no-data marker) on the same card; (ii) **layout (#14)** — the allergen unit occupies a fixed reserved slot **at or above the descriptive badge container in DOM/reading order**, at **≥ the prominence** of any descriptive badge, never truncated/evicted/shrunk by the 2–3 cap (the cap governs descriptive badges only). Honest atoms must not compose into a misleading "safe" gestalt.
   - Card **allergen unit** (`…_CARD_ALLERGEN`): warning + no-data marker render **together** (ALL-OR-NOTHING, guardrail #7) above an **allergen-coverage** gate keyed on the **allergen denominator + an absolute authored-dish floor** (low-N tenant can't trip it). OFF until real allergen coverage → no "clean by contrast". **Launch-gate:** human perception validation (floor copy + badge-stack gestalt with allergy-affected users) before the flip.
   - Gating guardrails red→green before flip: #1, #4-positive, #5, #5d, #6, #7, #12; for the card unit also #13, #14.
2. **COMPARE-VIEW** (`VITE_MENU_CHARACTERISTICS_COMPARISON`).
   - Exactly **two** dishes, one mobile screen. **Entry = a visible "compare" affordance ONLY** (discoverable + accessible). The **long-press gesture is removed** (FB-M4: it collides with card tap-to-modal, iOS native long-press, and scroll-hold, and is unneeded); a power-path gesture, if ever wanted, must first pass a gesture-collision test.
   - Delta per axis via a **directional indicator on price + prep-time ONLY** (non-regulated, customer-neutral). **NO arrow on kcal or any regulated/derived axis** (a ↓ on kcal *is* the "lighter wins" verdict). Prep-time shown as **"~N min"** point estimate (`prep_time_minutes` is one int — synthesizing a range fabricates uncertainty); if a real range source appears later it can replace the point.
   - Macros side-by-side as raw numbers (missing → explicit "no data", never inferred, never a bare blank beside a value). **Taste = two profiles side-by-side, NOT a winner.** Composition = presence chips; **every allergen cell explicit** (warning / floor / partial + reliance bound), **never blank** (guardrail #8). **No global winner.**
3. **FILTER** (`VITE_MENU_CHARACTERISTICS_FILTER`).
   - Client predicate over the loaded menu (no backend, O(≤200)). SHIP lenses = **non-allergen, non-regulated only**: sort-by-protein / sort-by-energy (raw macros) + taste facets + (once the diet field exists) **declared-vegan**. A dish with no data for the active lens is shown in an **explicit "no data" group** (guardrail #15) — a no-bom dish is checked by `bom` presence, **never** folded into the numeric-0 band (FB-M3); never silently dropped as if it failed.
   - **Allergen filter** — the live "contains X" view is converged in STEP 0; whether it stays (with disclosure) or is gated OFF behind `…_ALLERGEN_FILTER` is the recorded human decision. Any **hide→safe** transform stays forbidden (guardrail #1, permanent ratchet); re-enable of a richer allergen lens is **positive-only** ("show dishes declared to contain X"), never hide→safe, behind markers + a non-dismissible coverage disclosure + an honest denominator + a human-set coverage threshold. **Sort-by-"lightness" deferred** with the regulated subset (sort over raw kcal is fine; a "light" verdict is not).

---

## 9. Security + tenant isolation

- **No new tenant table** in the whole build. The one schema change (modifier deltas, §5) adds columns to an **existing FORCE-RLS** tenant table — the migration must re-assert `ENABLE+FORCE`, never regress it, and the columns are served through the same tenant-scoped path. RLS surface otherwise unchanged.
- **No PII** anywhere in the characteristics layer (menu-only) — consistent with the null-PII-to-derivation spine; nothing here is eligible to reach an LLM or analytics with customer data.
- **Owner control is subtractive-only** (§4): `char_hidden` is an enum-subset, server-validated; no fabrication path exists.
- **Diet declaration**, when built, is an owner declaration on `attributes` under existing FORCE-RLS — owner owns liability; the read path must **never derive** vegan/halal/organic from ingredients (a false "vegan" is the diet analogue of a false "nut-free").
- **Hot-path untouched:** `read_public_menu` (🔴) is not re-versioned; the honesty read-gate (`allergens_confirmed`) stays upstream and authoritative; the layer reads `confirmed` as provenance, never as "reviewed".

---

## 10. Operability

- **Health:** no new runtime; nothing to be "down". **Degraded = a product shows fewer/zero chips** (observable in the same menu render). The modifier-delta migration is the only thing that can FATAL the boot-guard if unapplied — standard staging-first discipline covers it.
- **Observability (< 1 min):** a dev-only assertion/log when a label is suppressed for a missing anchor citation (catches an un-anchored rule reaching prod) and when `char_hidden` contains a non-vocabulary token (catches a fabrication attempt). Pure functions → deterministic unit tests are the primary signal.
- **Rollback / flag-gate:** every surface is independently flag-gated, default off; flipping off reverts to the prior behavior with **zero data change, instant.** No flag flips until its gating guardrails are red→green (step 8). The descriptive allowlist + regulated anchors being empty is a second, data-level safety default underneath the flags.
- **Scaling gate:** none — no new connections/queries; the filter and recompute are client-bound and O(≤200)/O(≤30).

---

## 11. Open / accepted risks (owner per item)

- **R-0 — STEP 0: two allergen red-line crossings are already in production** (recipe-only live filter ignoring `declared_allergens`; card/quick-add diverging from the modal's `computeAllergenSurface` — FB-C1/FB-C2). *Decision:* **FIX as a standalone safety fix that lands first/independent of every flag** — unify on `computeAllergenSurface`, remove the unguarded card chip (v1 detail-floor-only), guardrail #12. *Owner:* frontend + architect.
- **R-1 — Honest live modifier recompute needs data that does not exist, and delivering it honestly is a 🔴 hot-path concern.** Modifiers carry only `price_delta`; the deltas can only reach the client via `read_public_menu` (🔴) or a new read path (FB-H1). *Decision:* **v1 ships Contract A only (suppress reassurance, never the warning); Contract B DEFERRED behind `MODIFIER_NUTRITION_ENABLED` — §5 migration + owner delta entry + a LAZY per-product detail fetch (NOT a hot-path re-version).* The proposal no longer claims live modifier honesty for v1. *Owner:* product/data + architect.
- **R-2 — Supply-Library nutrition is `localStorage`-only → per-device, non-reproducible.** *Decision:* **server-authoritative supplies is a PREREQUISITE for the REGULATED subset**; descriptive + taste + allergen-presence ship on current data (low-stakes for non-regulated chips). *Owner:* product/data.
- **R-3 — Regulated thresholds are EU/AL legal claims; exact numbers must NOT come from memory.** *Decision:* **gate + NEEDS-HUMAN** — regulated subset red-on-disk OFF until a verified per-market anchor table (citation + `verifiedBy`) exists in-tree AND owner opt-in authority is wired. *Owner:* Counsel/legal + architect.
- **R-4 — Descriptive thresholds (margins) are product judgments, not law, and could surface a borderline label.** *Decision:* "notable margin" rule + a guardrail test on threshold margins; allowlist populated only after EN+sq+uk register review. *Owner:* product + Counsel/i18n.
- **R-5 — Diet declaration is genuinely new owner work** (contradicts "computed, zero owner work"). *Decision:* defer to `…_DIET_DECLARATION` track with its own owner UI + liability copy; never blocks the rest. *Owner:* product.
- **R-6 — Allergen filter false-safety.** *Decision:* **DEFER ENTIRELY** + guardrail #1 forbids any dish-removing allergen predicate; re-enable is positive-only + coverage-disclosed + human-thresholded (NEEDS-HUMAN). *Owner:* Counsel/legal + product.
- **R-7 — Card allergen unit on a partially-authored menu reads "clean by contrast".** *Decision:* **FIXED by construction** — ALL-OR-NOTHING card unit (guardrail #7) keyed on the allergen denominator + an absolute authored-dish floor; OFF until real coverage. *Owner:* product.
- **R-8 — Long-press entry to compare collides with card tap-to-modal / iOS native long-press / scroll, and fails a11y as the only path.** *Decision:* **the visible "compare" affordance is the SOLE entry; the long-press gesture is REMOVED** (FB-M4). A power-path gesture, if ever wanted, must first pass a gesture-collision test. *Owner:* frontend/design.
- **R-11 — Card badge-stack-as-safety gestalt + decoupled coverage gates** (FB-STOP-2b / FB-H2). *Decision:* **card coupling invariant (#13: no reassuring badge without a determinate allergen unit) + layout invariant (#14: allergen unit never subordinated/below/crowded by the badge stack) + a launch-gate human perception validation** before `…_CARD_ALLERGEN`. *Owner:* product/design + architect.
- **R-12 — Perception not proven to allergic users** (Counsel open question). *Decision:* **NEEDS-HUMAN launch-gate** — record a usability validation of the floor copy + badge-stack gestalt with actual allergy-affected users before the `…_CARD_ALLERGEN` flip; no guardrail closes this. *Owner:* product/design.
- **R-13 — Sort-by-macro buries no-bom "unknown" in the numeric-0 band** (FB-M3). *Decision:* **FIXED** — explicit "no data" group keyed on `bom` presence, never numeric 0 (guardrail #15). *Owner:* frontend.
- **R-9 — `confirm-allergens` route flips review without authored content.** *Decision:* this layer never reads `confirmed` as `reviewed` (robust regardless); the route-semantics fix is **DEFER-FLAG** to the owner of `menu-confirm.ts`. *Owner:* API/backend.
- **R-10 — i18n regulated/descriptive Albanian drift.** *Decision:* **DEFER-FLAG** — per-market linguistic + legal review gates the regulated subset and the allowlist population (NEEDS-HUMAN). *Owner:* Counsel/legal + i18n.

---

## 12. Carried principle — where the safety duty lands (LAW, do not re-cross quietly)

The layer must **NOT silently migrate the legal food-safety / nutrition-claim duty from owner → platform.** An honest-by-construction data layer meeting a dishonest-by-omission data set creates real reliance the moment a "light" badge or a filter is trusted. Duty + authority stay with the owner (the food-business-operator): positive-only allergen view (no platform-asserted safe set) · "allergen info not provided" markers (silence is *unchecked*, not *clean*) · owner-gated + legally-anchored regulated claims · coverage disclosure + honest denominator · server-authoritative regulated supplies · subtractive-only owner control. Secondary caveat: partial L2 coverage tracks **data provenance** (owner-built `bom` vs scraped `place`) — "fewer chips" must never read as "less healthy dish". A future reader must not strip the reliance bound or open a hide→safe filter to "clean up" the surface — these are the honesty floor, not decoration.

---

## BACK-OF-ENVELOPE — SUMMARY (6 lines)
1. Scale: demo ≈50 products, cap ≈200/location, few-hundred locations; menu already cached + SSR/CDN.
2. Badge-compute: O(bom ≤15) ≈20–100 ops/card, sub-ms; full menu ≈4–20k ops <1 ms (already paid by `bomToNutrition`).
3. Modifier recompute: one open dish per toggle, O(bom+selected) ≤30 ops, human-rate; never touches the grid.
4. Compare (mobile): 2 dishes, one screen, ≈60 ops, single render — trivial.
5. Filter pass: one client predicate, O(≤200) per toggle, <1 ms, zero backend.
6. Budget: +0 DB connections / queries / endpoints; i18n ≈35–40 fixed strings (sq/en/uk, hand-translated); one optional additive FORCE-RLS modifier-delta migration (RED-LINE, gated dark).

## OPEN-RISKS
- **R-1** live modifier recompute needs modifier nutrition/allergen deltas (don't exist) → Contract A now / Contract B behind `MODIFIER_NUTRITION_ENABLED` — *owner: product/data.*
- **R-2** supply nutrition is `localStorage`-only (non-reproducible) → server-authoritative supplies prereq for regulated — *owner: product/data.*
- **R-3** regulated thresholds are EU/AL law, exact numbers NOT from memory → verified anchor table NEEDS-HUMAN, red-on-disk OFF — *owner: Counsel/legal + architect.*
- **R-4** descriptive margins are product judgment → "notable margin" rule + threshold-margin guardrail + EN/sq/uk review — *owner: product + i18n.*
- **R-5** diet declaration is genuinely new owner work → deferred `…_DIET_DECLARATION` track — *owner: product.*
- **R-6** allergen filter false-safety → DEFERRED, guardrail #1 forbids dish-removing allergen predicate; re-enable positive-only + NEEDS-HUMAN — *owner: Counsel/legal + product.*
- **R-7** card allergen unit "clean by contrast" → ALL-OR-NOTHING (guardrail #7) + absolute authored-dish floor, OFF until coverage — *owner: product.*
- **R-8** compare long-press-only fails a11y → visible "compare" affordance mandatory alongside the gesture — *owner: frontend/design.*
- **R-9 / R-10** `confirm-allergens` route semantics + Albanian regulated/descriptive drift → DEFER-FLAG to route owner / NEEDS-HUMAN i18n+legal review.
