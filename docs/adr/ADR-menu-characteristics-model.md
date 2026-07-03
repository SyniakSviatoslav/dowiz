# ADR — Menu Characteristics Model

- **Status:** Accepted (Council-resolved, **RESOLVE round 5 — CONVERGED; convergence gate met (0 CRIT/HIGH)**) — **v1 SHIP = L1 taste (card) + L2 descriptive (closed allowlist, EMPTY in v1 → dormant) + allergen DETAIL-FLOOR (detail modal only; NO card allergen element) ONLY; COMPARISON and FILTER are DEFERRED to their own flags (`…_COMPARISON` / `…_FILTER`), NOT in v1 (R5-L1).** The R3-H3 partial-as-exhaustive reliance bound is now a **SHIP-gating guardrail (#5d):** a non-empty allergen declaration rendering without the "not a complete list — confirm with the venue" caveat is RED. The R2-H3 comparison explicit-marker is bound to the **`…_COMPARISON`** flip (#8): a comparison allergen cell that is blank/"—" is RED. Both were spec-mandated but previously unbound (R5-H1). SHIP = **taste-first**. **v1 elevates NO allergen element to the card (DETAIL-FLOOR-ONLY, R4-H1):** the card allergen surface = today's behavior (allergens detail-only); the authored warning + the no-data state both live in the always-on **detail modal** (floor: empty → "allergen info not provided", never blank) + the reliance bound; the warning is never less visible than today (STOP-3 honored). **INVARIANT: the card allergen surface is ALL-OR-NOTHING** — the card never shows an authored warning unless it also shows the no-data marker on no-data cards (warning + marker are one inseparable unit, gated/ungated **together, never split**; the at-a-glance card unit is a LATER allergen-coverage-gated feature). The **descriptive** coverage gate is **descriptive-axis ONLY** (one denominator) and governs **no** allergen element; the descriptive vocabulary is a **closed human-reviewed allowlist** (empty for v1, R3-H2); the allergen presence layer carries a persistent **"not a complete allergen list — confirm with the venue"** reliance bound wherever it renders so partial declarations never read as exhaustive (R3-H3); safety-inverting surfaces gated/deferred. STOP-1 + STOP-2 + STOP-3 all discharged. See `docs/design/menu-characteristics-model/resolution.md` §RESOLVE round 4.
- **Date:** 2026-06-28
- **Slug:** `menu-characteristics-model`
- **Design doc:** `docs/design/menu-characteristics-model/proposal.md`
- **Relates to:** acquisition read-gate (mig 068/070/072), prep-time/ETA (mig 065), `products.attributes` model.

## Context

The storefront already ships raw dish facts to the client inside `products.attributes` —
taste (`attributes.taste`, owner-entered 0..3 axes), macros (summed from `attributes.bom[]`),
allergen presence (`bom[].allergens`), ingredients, and `prep_time_minutes`. They are shown
only as raw figures in a per-dish detail modal. Customers cannot scan a menu through their own
lens. We want an honesty-backed **characteristics layer** that maps these existing numbers to
human, at-a-glance labels (with exact figures one tap away), plus a comparison view and a filter
built on the same layer.

Grounding (see proposal §0) established what is real vs assumed:
- **L1 taste — EXISTS** end-to-end as owner-entered data; presentation only (NOT derived, NOT zero owner work, but zero new plumbing).
- **L2 macros — DERIVABLE** by folding `bom`, but only for products that have a recipe (`bom`); no `fibre` field.
- **L3 allergen presence — DERIVED** from `bom` already; absence is (correctly) never derived.
- **L3 diet (vegan/halal/organic) — ABSENT**: no field, no UI. Genuinely new owner-entered declaration.
- **Modifiers — carry only `price_delta`**: nutrition/allergen recompute on a modifier is **not honestly buildable** today.

## Decision

Adopt a **presentation layer over existing dish data**, built in order
**characteristics layer → comparison (a view on it) → filter (a seam behind it)**, with three levels:

1. **L1 TASTE (sensory).** Render the existing owner taste profile as a minimal visual mini-profile. Zero liability. In comparison it is shown as **two profiles side-by-side, never a winner.**
2. **L2 DISH CHARACTER (derived, number-anchored) — SPLIT into two classes (BC-1).** A pure function `deriveCharacteristics(product)` runs at the **client edge** over already-served `attributes` (NOT inside hot-path `read_public_menu`); `CHARACTERISTIC_RULES` is split:
   - **DESCRIPTIVE (SHIP) — a CLOSED, HUMAN-REVIEWED ALLOWLIST (R3-H2):** the descriptive vocabulary is a small, fixed, closed set; each label is individually reviewed (EN+AL) to clear the regulated/satiety/energy/health/nutrient-content register **before** it enters. A token denylist cannot catch conveyed meaning ("hearty"=satiety, "rich"=energy-density, "carb-forward"=nutrient-content), so the polarity is inverted: human judgment moves upstream (once per label), the test stays a **deterministic subset check** (guardrail #6: rendered descriptive set ⊆ allowlist). **"hearty"/"rich"/"carb-forward"/"protein-forward" do NOT ship until each clears review → the v1 allowlist is EMPTY** (taste-first; the descriptive band renders nothing and the coverage gate is dormant until a reviewed label exists). NEEDS-HUMAN: allowlist contents + AL review.
   - **REGULATED (GATED OFF):** legally-defined claims (light / low-energy / source-of-protein, EU 1924/2006 + AL). NOT platform-asserted — requires honesty-anchor **AND** owner authority (opt-in/confirm, not opt-out) **AND** a verified per-market legal-threshold table **AND** server-authoritative supplies. A real number is necessary but NOT sufficient. Red-on-disk guardrail keeps it off until the verified-anchor table exists; supplying the anchors is **NEEDS-HUMAN**.
3. **L3 COMPOSITION & DIET (owner declaration) — v1 allergen surface is DETAIL-ONLY (R4-H1).** Allergen **presence** chips (derived from `bom` today): the authored **WARNING** ("declared to contain X") and the no-data state both live in the **always-on detail modal** in v1 — **no allergen element renders on the card** (= today's behavior, no contrast). **Detail floor (INVARIANT + guardrail #5):** the detail modal ALWAYS renders the allergen truth — "declared to contain X" or an explicit **"allergen info not provided — ask the restaurant"** — regardless of coverage (this changes `MenuPage.tsx:1078`, which today renders the section only on `length>0`); the warning is never less visible than today (STOP-3). **CARD ALL-OR-NOTHING INVARIANT (guardrail #7):** the card never shows an authored warning unless it ALSO shows the no-data marker on no-data cards — warning + marker on the card are **one inseparable unit, gated/ungated together, NEVER split** (so a no-data card can never read "allergen-free by contrast" with a sibling warning card). The at-a-glance **card** allergen unit (warning + marker together) is a **LATER allergen-coverage-gated** feature, keyed on the **allergen** denominator (not the descriptive one). The presence layer ALWAYS carries a **persistent, surface-attached "not a complete allergen list — confirm with the venue"** reliance bound **wherever it renders allergen content** (detail; later card unit + comparison cells — **not** on otherwise-silent v1 cards, R4-M1) so a **partial** declaration (`['milk']`) never reads as the exhaustive profile (R3-H3); in comparison no cell may imply exhaustiveness. `allergens_confirmed` is read as provenance only, **never as "allergen-reviewed."** **Diet** (vegan/halal/organic) is an **owner DECLARATION** on `attributes.diet` — **deferred** (field does not exist; new owner UI + liability). Platform never derives or fabricates a composition claim.

**Comparison** compares exactly two dishes via the same `deriveCharacteristics` + raw macros: directional
arrows (↑/↓/≈) **ONLY on price and prep-time** (R2-H1) — **NO arrow on calories or any regulated/derived
axis** (a ↓ on kcal IS the lightness verdict); macros render **side-by-side as the raw number only**; taste as
paired profiles; composition as presence chips (a no-data dish renders the "info not provided" marker
explicitly, **never blank/"—"** — R2-H3); **no global winner**; prep time as the **single value "~N min"**
(`prep_time_minutes` is one int — no min/max column; a synthesized range fabricates uncertainty — R2-M2).

**Filter** is a client-side predicate over the same derivation (sort-by-protein/energy derivable now;
only-vegan deferred with the diet field). The **allergen FILTER is DEFERRED ENTIRELY from SHIP (STOP-1/C1)**:
over the near-empty live denominator a hide/safe transform manufactures an implied safe set — a guardrail
forbids any hide/safe allergen predicate. If it ever returns it is **positive-only** ("show dishes that
contain X"), never hide→safe, behind markers + a non-dismissible coverage disclosure + an honest denominator
+ a human-set coverage threshold (**NEEDS-HUMAN** to re-enable). Sort-by-"lightness" is deferred with the
regulated subset (sort over raw kcal is fine; a "light" verdict is not).

**Derivation site:** client-side pure function (reusable server-side verbatim later), NOT a
re-version of `read_public_menu`. **Storage:** SHIP scope needs **no migration**; owner hide-list
(`attributes.char_hidden`) and the deferred `attributes.diet` ride the existing `jsonb` column
under FORCE-RLS. **Flag:** `MENU_CHARACTERISTICS_ENABLED` (default off) with sub-flags for
comparison, filter, and diet-declaration.

## Red lines preserved

- **Allergen presence-only / absence-never AND data-absence-never-reads-as-clean (incl. by CONTRAST — R4-H1) AND never-suppress-a-warning (across ALL mechanisms — R3-H1).** Absence ("nut-free") is never shown, derived, or stored — **and never manufactured by contrast** between a card's blank and a sibling card's warning. An authored allergen warning is **NEVER suppressed by ANY mechanism — modifier (R2-C1) OR coverage (R3-H1) OR curation cap (R2-H3)**; its unconditional floor is the **detail modal** (always; never less visible than today — STOP-3). The **descriptive coverage gate is descriptive-axis ONLY** and governs **no** allergen element. **v1 puts no allergen element on the card (DETAIL-FLOOR-ONLY, R4-H1):** the warning + the no-data state + the reliance bound live in the always-on detail floor (`MenuPage.tsx:1078` fix: empty → "allergen info not provided", never blank); the card = today's behavior (uniform silence, no contrast). **CARD ALL-OR-NOTHING (guardrail #7):** the card never shows an authored warning unless it ALSO shows the no-data marker on no-data cards — warning + marker on the card are one inseparable unit, gated/ungated **together, never split** (the at-a-glance card unit is a LATER allergen-coverage-gated feature). **Data-absence never renders as a clean allergen state on any surface:** detail renders "declared to contain X" or "allergen info not provided"; the card asserts nothing in v1 (no false-clean chip, no contrast); every comparison cell renders explicitly; **partial declarations carry a persistent "not a complete allergen list — confirm with the venue" bound (R3-H3)** so a partial list never reads as exhaustive. NO hide/safe allergen filter ships (deferred, guardrail-forbidden). **The allergen-PRESENCE WARNING is NEVER suppressed — including on modifiable dishes (R2-C1/STOP-2):** modifiers carry no allergen data, so a modifier can only ADD allergens, never honestly remove a base one → a base-dish warning is always conservatively true; suppression scopes to *reassuring* labels only. If an allergen lens ever returns it is positive-only (contains-X), never hide→safe.
- **No platform-asserted regulated claims.** Regulated nutrition claims (light/low-energy/source-of-protein) are owner-authority-gated + verified-anchored + server-sourced; they stay OFF until prerequisites exist (red-on-disk guardrail). The descriptive vocabulary stays on dish description; body-effect phrasing ("keeps you full", "healthy", "good for you") is denylisted (CI red).
- **Honesty anchor = a real number is required (necessary, not sufficient).** No qualifying number → no label. Borderline values (no notable margin) → no label. For a regulated label, the number alone does NOT entitle the claim — owner authority + legal threshold also required.
- **Owner authority for declarations.** Allergen/diet declarations are owner-owned; platform is conduit. `allergens_confirmed` is provenance, not "reviewed." (Wiring the owner `AllergenEditor` is the prerequisite for true L3 declarations — deferred/flagged.)
- **Hide-not-fabricate (asymmetry).** Owner may suppress a true label; may never add an unsupported one, nor flip presence→absence.
- **Facts, not a verdict.** No global "winner"/"healthier" in comparison.
- **Never fabricate composition.** Diet/allergen claims are never inferred from ingredients (no auto-"vegan").
- **Facts-not-verdict in comparison.** No global winner; directional arrows **only on price and prep-time** — never on calories or any regulated/derived axis (a kcal ↓ arrow is the lightness verdict — R2-H1).
- **Hot-path untouched.** `read_public_menu` is not re-versioned for presentation; honesty read-gate (`allergens_confirmed`) remains upstream and authoritative.

## Carried principle — where the safety duty lands

The layer must **NOT silently migrate the legal food-safety / nutrition-claim duty from owner → platform.**
The honest-by-construction data layer meets a dishonest-by-omission data set; reliance is real the moment a
customer trusts the filter or a "light" badge. The design choices — positive-only allergen view, "info not
provided" markers, owner-gated regulated claims, coverage disclosure + honest denominator, server-authoritative
regulated supplies — keep duty **and** authority with the owner (the food-business-operator who legally holds
them). A future reader must not re-cross this quietly. Secondary caveat: partial L2 coverage tracks data
provenance (owner-built `bom` vs scraped `place`), so "fewer chips" must not be read as "less healthy dish."

**Carried reliance principle (presence-only floor, R3-H3).** A presence-only allergen system can never assert
completeness — a non-empty `bom[].allergens` is *what was declared*, not *the exhaustive truth*. Therefore the
presence layer ALWAYS carries the reliance bound on the surface itself: "**declared to contain …**" + a
persistent "**not a complete allergen list — confirm with the venue**". A partial declaration must never read
as exhaustive (the "milk only ⇒ no nuts" inversion is forbidden), and no comparison cell may imply
exhaustiveness. A future reader must not strip this bound to "clean up" the surface — it is the honesty floor,
not decoration.

## Consequences

**Positive.** Zero new DB connections/queries/migrations for SHIP scope; pure deterministic
derivation (trivially testable, idempotent, no cascade); comparison and filter cannot disagree
with the card (one source of truth); instant rollback via flag; degrades to today's behavior when
data is absent.

**Negative / costs.**
- "Computed, zero owner work" holds only for L2/allergen-presence **where a `bom` exists**; **L1 taste is owner-entered** and **L3 diet is genuinely new owner work** — both honestly acknowledged, diet deferred.
- **Modifier recompute for nutrition/allergens is out of honest scope** until modifiers carry deltas; SHIP characterizes the base dish and annotates "base dish" rather than lie.
- Supply-Library nutrition is `localStorage`-only; product numbers are snapshots and don't retro-correct.
- Thresholds require verified per-market legal anchors before any L2 label ships.

## Migration

- **SHIP scope: none.** Presentation over existing `attributes` served by `read_public_menu`.
- **Owner hide-list:** additive `attributes.char_hidden: string[]` (enum-subset, Zod-validated) — no migration (`jsonb` passthrough).
- **Deferred diet:** additive `attributes.diet: {vegan?,vegetarian?,halal?,organic?}` (owner declaration) — no migration; needs owner UI + validation + i18n.
- **Deferred fibre:** would require a `fibreG` line field on `bom` + Supply Library (real data change) — flagged, not in scope.
All additive `jsonb` keys are backward-compatible (unknown keys ignored).

## Flag / rollout

1. **Ship first — TASTE-FIRST; v1 = taste + descriptive band + allergen detail-floor ONLY (R5-L1):** L1 taste mini-profile ships independently (the only populated, zero-liability surface). The **descriptive band** (L2 descriptive chips only) renders **only above a per-tenant DESCRIPTIVE-coverage gate** (ONE denominator; governs **no** allergen element; threshold = NEEDS-HUMAN, product value; **dormant at v1 because the descriptive allowlist is empty**). **v1's allergen surface is DETAIL-ONLY (R4-H1):** no allergen element on the card; the warning + no-data marker + reliance bound live in the always-on **detail** floor (the `MenuPage.tsx:1078` fix). **COMPARISON and FILTER are DEFERRED to their own flags below — NOT in v1.** v1's allergen value is honestly "today's detail, made honest"; the at-a-glance menu-scan safety upside lives behind the deferred `…_CARD_ALLERGEN` unit, not v1 (Counsel (b)). The flag must not flip until ship-gating guardrails #1, #4-positive, #5, **#5d**, #6, #7 are red→green (R2-H4). Curation 2–3 (allergen surface exempt — R2-H3). On modifiable products the **reassuring** label suppresses (BC-2) but the **allergen-presence WARNING is NEVER suppressed** (R2-C1/STOP-2; never by coverage — R3-H1; never by the contrast asymmetry — R4-H1/#7). Presence layer carries a **persistent surface-attached** "declared to contain" + "not a complete list — confirm with venue" bound wherever allergen content renders, now SHIP-gated by #5d (Counsel Q1, R3-H3, R5-H1(i); **not** on otherwise-silent v1 cards, R4-M1).
   - **Later (`…_CARD_ALLERGEN`):** the at-a-glance **card** allergen UNIT (warning + no-data marker render **together**, guardrail #7) turns on above a per-tenant **allergen-coverage** threshold — keyed on the **allergen denominator alone AND an ABSOLUTE authored-dish floor** (not a ratio-only test, so a low-N tenant with 2 authored dishes cannot trip it on — Counsel (a)); NEEDS-HUMAN; OFF in v1 so the card never reads "clean by contrast".
2. **Then (`…_COMPARISON`) — DEFERRED, NOT in v1 (R5-L1):** two-dish comparison; arrows on **price + prep-time only** (R2-H1); macros side-by-side as numbers; prep-time as a point estimate, not a range (R2-M2). **Bound to guardrail #8 (R5-H1(ii)):** every allergen cell renders its explicit state (warning / "allergen info not provided" / partial + #5d bound) and **never blank/"—"** — the flip cannot occur until #8 is red→green.
3. **Then (`…_FILTER`):** non-allergen lenses only (sort-by-protein/energy). **Allergen filter NOT in this step** (deferred, STOP-1/C1).
4. **NEEDS-HUMAN gate — regulated L2 subset (BC-1):** OFF until a verified per-market legal-anchor table exists in-tree + owner opt-in/confirm authority is wired + server-authoritative supplies (R-2) + al linguistic/legal review (R-10). Red-on-disk guardrail enforces off.
5. **NEEDS-HUMAN gate — allergen filter (`…_ALLERGEN_FILTER`, STOP-1/C1):** OFF until a human records a coverage-threshold + positive-only design (contains-X, never hide→safe) + markers + non-dismissible coverage disclosure + honest denominator. Guardrail forbids any hide/safe allergen predicate.
6. **Separate track (`…_DIET_DECLARATION`) — NEEDS owner data that does not exist:** diet declaration field + owner UI (wire `AllergenEditor`, R-9) + liability copy + "only-vegan" filter.
7. **Out of scope until data exists (R-1):** honest modifier recompute for nutrition/allergens (`MODIFIER_NUTRITION_ENABLED`) — replaces BC-2 suppression when modifiers carry deltas.
8. **Route-semantics fix (DEFER-FLAG, R-8):** `confirm-allergens` should require authored allergen content before it asserts review — owner of `menu-confirm.ts`; this layer is robust regardless.

## Guardrails (red→green before each flag flips — SPECIFIED, NOT BUILT; design-time)

Flag flips are **bound** to these (R2-H4): SHIP (v1 = taste + descriptive band + allergen detail-floor) must not flip until **#1, #4-positive, #5, #5d, #6, #7** are red→green. **Each deferred surface's allergen guard travels with its own flag** (no over-gating of v1; no deferred surface ships without its guard): **#8** gates `…_COMPARISON`; **#2** gates the regulated flag; the positive-only design rule gates the `…_ALLERGEN_FILTER` re-enable.
1. **No-hide-allergen** (STOP-1/C1; R3-M1) — grep: no allergen-axis filter predicate is wired into the rendered set (filter ABSENT from SHIP). "Positive-only" is a recorded re-enable design rule, NOT this grep. Gates SHIP, permanent ratchet.
2. **Regulated-subset-off** (BC-1) — gates the *regulated* flag; red-on-disk until anchor table exists.
3. **Body-effect denylist** (keep) — regulated body-effect phrasing → CI red. All flags.
4. **Warning-never-suppressed / positive complement** (R2-C1/STOP-2; R3-H1; R4-H1) — a dish with a base allergen MUST render "declared to contain X" in the **detail modal regardless of modifier state AND coverage** (the unconditional floor); when the card allergen unit is ON, the card warning renders **with** the marker (never alone — #7); reassuring chips suppressed on modifiable dishes. Gates SHIP.
5. **No-clean-from-empty + detail floor** (C2/M2; R3-H1; R4-H1) — detail modal renders "info not provided" on empty `bom[].allergens` **unconditionally** (regardless of coverage — the `MenuPage.tsx:1078` fix); detail renders "declared to contain X" on non-empty unconditionally; no surface renders a clean state from empty data. The detail floor + warning are unconditional; the card allergen surface is governed by #7. Gates SHIP.
5d. **Reliance-bound-rendered** (NEW — R5-H1(i)) — on the v1 **detail floor**, the rendered allergen state carries the persistent **"not a complete allergen list — confirm with the venue"** bound (EN+AL) in BOTH branches; **a non-empty allergen declaration rendering without the incompleteness caveat is RED** (positive assertion; surface-attached). Binds the R3-H3 carried principle into the flip contract — the only protection against partial-as-exhaustive ("milk only ⇒ no nuts") was prose; now a SHIP gate. Gates SHIP.
6. **Descriptive-ALLOWLIST** (R2-H2 → R3-H2, inverted) — rendered descriptive vocabulary (EN+AL) ⊆ the closed human-reviewed allowlist (deterministic subset check; v1 empty). Gates SHIP.
7. **Card-allergen ALL-OR-NOTHING** (anti-contrast, R4-H1) — a build where the **card** renders an allergen **warning** chip while a no-data **card** renders **no marker** is RED; warning + no-data marker on the card are one inseparable unit (both render or neither, at every coverage level). v1 (card allergen unit OFF) is trivially green; permanent ratchet so the contrast door can never be (re)introduced. Gates SHIP.
8. **Comparison-allergen explicit-both** (NEW — R5-H1(ii)) — every comparison allergen cell renders its **explicit** state (warning / "allergen info not provided" / partial + #5d bound) and **never blank/"—"**; #7's all-or-nothing logic extends to comparison cells (no cell empty while another asserts). A blank/"—" cell reads nut-free by contrast at the highest-stakes pair surface → the assertion is **positive** (assert the explicit text), not the #5(c) negative. **Gates `…_COMPARISON` (NOT SHIP — comparison deferred from v1, R5-L1).**
See `resolution.md` §RESOLVE round 5 §"Final guardrail table".
