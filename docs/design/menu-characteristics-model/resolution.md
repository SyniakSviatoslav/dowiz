# Resolution — Menu Characteristics Model

**Seat:** Architect (Triadic Council), RESOLVE round. Design-time only — NO production code.
**Inputs:** `breaker-findings.md` (C1,C2,H1,H2,H3,M1,M2,M3,L1,L2,L3) · `counsel-opinion.md` (STOP-1, BC-1, BC-2, "where the safety duty lands").
**Re-grounded @ HEAD:** `menu-confirm.ts:18-24` (confirm flips boolean only, no allergen-content check — C2 holds), `MenuPage.tsx:99-113` (`bomToNutrition` returns `allergens:[]` for no-bom — C1/M2 hold), `provisioning.ts:50-58` (scrape write-strips allergens), `menu-seed.ts` (0 bom rows).

---

## Verdict

**CONVERGED — to a REDUCED SHIP scope.** The architecture (presentation layer, zero migration, pure
deterministic `deriveCharacteristics`, comparison/filter as views over one source) is sound and is kept.
The convergence is achieved by **cutting the customer-facing safety-inverting surfaces out of SHIP** and
gating them behind real data + owner authority + verified legal anchors.

**Safe SHIP scope (now):** L1 taste · **descriptive** L2 chips · L3 allergen-**presence** chips (with a
first-class "allergen info not provided" marker) · comparison on **non-regulated** axes.

**Gated / deferred (NOT in SHIP):** the allergen **filter** (any hide/safe transform) · the **regulated**
L2 subset (light / low-energy / source-of-protein) · L3 **diet** (vegan/halal/organic).

**Remaining NEEDS-HUMAN (gates on the deferred surfaces, not blockers to the reduced SHIP):**
1. Coverage threshold + positive-only design sign-off before any allergen filter re-enables (STOP-1).
2. Verified per-market legal anchors (EU 1924/2006 + Albania transposition) before the regulated L2 subset.
3. Per-market linguistic/legal review of regulated-term Albanian renderings before regulated flip-on.

---

## Disposition table

| # | Sev | Finding (one line) | Disposition | What changes |
|---|-----|--------------------|-------------|--------------|
| **STOP-1 / C1** | CRIT | Allergen "hide-X" filter over near-empty data returns whole menu as implied "safe" | **FIXED (defer) + future-gated** | Allergen FILTER removed from SHIP; `…_ALLERGEN_FILTER` flag stays dark behind a coverage gate; if it ever returns it is **positive-only** (never hide→safe), with per-dish "info not provided" markers + non-dismissible coverage disclosure + honest denominator. Guardrail forbids a hide/safe allergen predicate. Re-enable is NEEDS-HUMAN. |
| **C2** | CRIT | `confirm-allergens` opens read-gate without any authored allergen content → confirmed dish reads clean | **FIXED (this layer) + DEFER-FLAG (route)** | Characteristics layer treats `allergens_confirmed` as **provenance only, not "allergen-reviewed."** A product with empty/absent `bom[].allergens` shows "allergen info not provided" **regardless of the confirmed flag.** Separately flag the confirm-route to require authored allergen content before it asserts review (not fixed here — shipped-route change). |
| **H2 / BC-1** | HIGH | L2 thresholds are EU/AL-regulated nutrition claims, platform-asserted, gated only by a comment | **FIXED (split) + NEEDS-HUMAN (anchors)** | `CHARACTERISTIC_RULES` split into **descriptive** (platform-renderable) and **regulated** (owner-authority + verified-anchor). Regulated subset is OFF until a verified-anchor table exists on disk; red-on-disk guardrail keeps it off. Honesty-anchor (number) is necessary but not sufficient — owner entitlement + legal threshold also required. |
| **H1 / BC-2** | HIGH | Dropped modifier recompute → stale "light"/allergen badge; "base dish" caption is a soft-confirm trap | **FIXED (suppress)** | On a product whose modifier groups can change the relevant axis, the **regulated / diet / allergen-sensitive** label is **SUPPRESSED, not captioned.** Descriptive/taste chips may keep the "base dish" caption. Reconciles with `MODIFIER_NUTRITION_ENABLED` (recompute + show only when modifiers carry nutrition/allergens). |
| **H3** | HIGH | Regulated labels rest on localStorage-only Supply Library → per-device, non-reproducible | **FIXED (disposition) + DEFER-FLAG (prereq)** | Server-authoritative Supply Library is a **prerequisite for the regulated subset** (recovers Counsel's Option-A kernel: server-recorded/audited for regulated claims only). Descriptive + taste + allergen-presence ship on current data. |
| **M1** | MED | Comparison arrows are micro-verdicts on unverified/regulated axes | **FIXED** | Directional arrows only on **non-regulated, well-defined** axes (price, prep-time range, raw macro numbers shown as the number). Regulated/derived axes show the number **side-by-side (fact), no "wins" arrow** until the regulated subset is owner-gated + anchored. |
| **M2** | MED | Near-dead allergen layer trains "no chip = clean" | **FIXED (via marker)** | The "allergen info not provided" marker (C1/C2 remedy) makes silence read as *unchecked*, not *clean* — directly defuses scarcity-as-signal. Marker is a first-class UI state, default everywhere, not a per-lens option. |
| **M3** | MED | "hide-allergen (exists)" overstates readiness — the hide-filter is net-new | **FIXED (doc)** | Proposal corrected: the existing `MenuPage.tsx:192-193` predicate is SHOW-CONTAINS; the safe-direction hide-filter is **net-new code** and is deferred anyway (C1). No "exists" credit claimed. |
| **L1** | LOW | Owner `AllergenEditor` not wired → liability anchor is vaporware | **DEFER-FLAG** | Wiring `AllergenEditor` into the menu-manager save is the **prerequisite for L3 owner declarations** (tracked with the deferred diet/declaration track). The SHIP layer does not claim an owner-declaration liability shield it does not have. |
| **L2** | LOW | Client-only derive will drift from future server use | **ACCEPT-RISK (descriptive) / covered (regulated)** | Accepted for **descriptive** labels (pure function, reusable verbatim server-side; drift is low-stakes for a non-regulated chip). For the **regulated** subset the H3/BC-1 server-authoritative path removes the drift. Owner: architect. |
| **L3** | LOW | al regulated-term vocabulary risks body-effect drift; allergen al-set partial | **DEFER-FLAG** | Per-market linguistic + legal review of regulated-term Albanian renderings is a **gate before the regulated subset flips on.** Descriptive/taste/allergen-presence al strings exist and ship. NEEDS-HUMAN (#3). |

---

## STOP-1 / C1 — the load-bearing decision (full justification)

**Chosen: (a) DEFER the allergen filter entirely for SHIP, AND (b) if it ever returns, positive-only.**
Not either/or — both, sequenced.

**Why defer now rather than ship with friction.** Counsel framed the remedy (markers + disclosure +
honest denominator) as cheap friction that *lifts* the STOP, leaving the human to choose ship-with-friction
vs defer. Re-grounded against the live denominator the choice collapses: demo coverage is **0/49**, the
acquisition path **write-strips** allergens to `[]`, and the read-gate strips `bom` until an owner confirm
that (C2) carries no allergen content. At launch the allergen filter would operate over a denominator that
is **near-zero by construction**, not merely incomplete. A coverage disclosure on a 1%-coverage filter is
itself a kind of false comfort ("we told you" over a tool that cannot work). The safest honest state for a
safety transform with no data is **its absence.** No filter is strictly better than a false-safe filter.
So SHIP carries **no allergen filter at all.**

**The red line, made deterministic.** The UI must NEVER let absence-of-data read as absence-of-allergen.
Two enforcement points:
1. **No hide/safe allergen transform ships.** Guardrail (red-on-disk): a CI/lint rule fails if a filter
   predicate over the allergen axis removes dishes from the visible set (a "hide" / "only-safe" semantics),
   or if `…_ALLERGEN_FILTER` is wired to anything but a positive-only view. The dangerous direction cannot
   ship by code.
2. **Silence is marked, not blank.** Every dish with empty/absent `bom[].allergens` renders an explicit
   **"allergen info not provided — ask the restaurant"** marker on its allergen surface (presence chips
   row), independent of any filter. This is the M2 remedy and makes the *presence* layer honest on its own.

**When (if ever) the filter returns — the binding shape (NEEDS-HUMAN gate).** Re-enable requires a recorded
human decision and must satisfy ALL of:
- **Positive-only semantics.** The allergen lens is a POSITIVE view ("show dishes the owner declared
  contain X"), never a "hide → the rest is safe" transform. It narrows *to* declared-contains, it never
  produces a residual "safe set."
- **No-data dishes marked, never silently in a residual.** Any dish without allergen data is shown with the
  "info not provided" marker; it is never folded into an implied-clean remainder.
- **Non-dismissible coverage disclosure** whenever an allergen lens is active over incomplete data.
- **Honest denominator** on the lens ("12 of 49 dishes have allergen info — always confirm with the venue").
- **Coverage threshold** (human-set) on owner-confirmed allergen coverage before the flag may flip per
  tenant.

**Is this still NEEDS-HUMAN after the revision?** For **SHIP: no** — deferral removes the STOP from the
ship path; the human decision is no longer a launch blocker. For **re-enable: yes** — the coverage threshold
and positive-only sign-off are an explicit recorded human decision before `…_ALLERGEN_FILTER` flips.

---

## C2 — confirm asserts review without content (this-layer fix + route flag)

The characteristics layer **does not** treat `allergens_confirmed=true` as "allergen-reviewed." The flag
is provenance/read-gate state only. A confirmed `place` product whose `bom[].allergens` is empty/absent
shows the **"allergen info not provided"** marker and **zero allergen chips that read as clean** — exactly
as an unconfirmed product would on the allergen surface. A confirm that carried no allergen content can
never produce a clean allergen surface.

**Separate (not fixed here):** the shipped `confirm-allergens` route (`menu-confirm.ts:18-24`) flips the
boolean with no check that any allergen was authored. That is a route-semantics change (require authored
allergen content, or a tri-state "reviewed: none-present" attestation, before the flip asserts review) —
**DEFER-FLAG**, raised to the owner of `menu-confirm.ts`. This layer is robust to that gap regardless,
because it never reads `confirmed` as `reviewed`.

---

## H2 / BC-1 — split the L2 vocabulary + the gate

`CHARACTERISTIC_RULES` is split into two classes:

- **Descriptive (platform-renderable now):** relative dish-description that is not a regulated claim —
  e.g. "hearty"/"filling", "rich", "protein-forward" (as relative description, not "source of protein"),
  "carb-forward". These are facts-rendered at the client edge, no owner gate. They still obey the
  honesty-anchor (a real number qualifies) and the body-effect denylist.
- **Regulated (gated OFF):** legally-defined nutrition claims — "light", "low-calorie"/"low-energy",
  "source of protein", etc. (EU 1924/2006 + Albanian transposition). These require **all three**:
  1. **Honesty-anchor** — a real qualifying number (necessary, not sufficient).
  2. **Owner authority** — owner opt-in/confirm (the owner=authority spine), NOT the opt-out
     `char_hidden`. The owner is the food business operator who legally holds the claim duty.
  3. **Verified legal threshold** — a per-market anchor in a real `CHARACTERISTIC_RULES.regulated`
     table sourced from VERIFIED legal definitions (NEEDS-HUMAN to supply).

**Gate (red-on-disk):** the regulated subset stays OFF until the verified-anchor table exists in-tree. A
guardrail test fails (CI red) if a regulated label is emittable without (a) an owner-authority flag set and
(b) a populated verified-anchor entry. The honesty-anchor alone does not unlock a regulated label — it
would only make a wrong/un-entitled claim *confident*.

---

## H1 / BC-2 — suppress, don't caption, on modifiable products

A product whose modifier groups can change the relevant axis (nutrition / diet / allergen) does **not**
render the regulated / diet / allergen-sensitive label at all — **suppress, not caption.** A "for the base
dish" caption on a safety/regulated claim is a soft-confirm trap read past by the constrained customer.
Descriptive (hearty/rich) and taste chips may remain, with the "base dish" caption acceptable for those
non-regulated chips only. When `MODIFIER_NUTRITION_ENABLED` lands (modifiers carry nutrition/allergen
deltas), honest recompute replaces suppression and the label may show post-modifier.

---

## H3 — server-authoritative supplies as a regulated-subset prerequisite

Regulated labels must not rest on non-reproducible, per-device, last-write-wins data. The localStorage-only
Supply Library means the same product's macro can flip across the owner's own devices. Therefore:
**server-authoritative (recorded/audited) supplies are a PREREQUISITE for the regulated L2 subset** — this
is the kernel of the rejected Option A, recovered for the regulated claims only (a recorded, versioned,
owner-tied server fact: "what we claimed, about which dish, on what data, under which threshold version").
**Shippable now on current data:** descriptive L2 + taste + allergen-presence (these display the `bom` they
characterize; for a non-regulated chip the per-device fragility is low-stakes and accepted, L2 finding).

---

## M1 — arrows only on non-regulated axes

Comparison shows directional arrows (↑/↓/≈) ONLY on non-regulated, well-defined axes: price, prep-time
**range**, and raw macro numbers shown **as the number** (not a "lighter wins" verdict). For regulated /
derived axes the comparison shows the numbers **side-by-side as facts, with no "wins" arrow**, until the
regulated subset is owner-gated + anchored. This keeps the facts-not-verdict red line intact and prevents
the comparison view from smuggling back the regulated-claim hazard.

---

## "Where the safety duty lands" — carried principle (recorded in ADR)

The layer must **not silently migrate the legal food-safety / nutrition-claim duty from owner → platform.**
The honest-by-construction data layer meets a dishonest-by-omission data set, and customer reliance is real
the moment the filter or a "light" badge is trusted. The design choices keep duty + authority with the
owner: **positive-only allergen view (no platform-asserted safe set), "allergen info not provided" markers
(silence is unchecked, not clean), owner-gated regulated claims (the FBO asserts, not the conduit), coverage
disclosure + honest denominator (reliance is bounded), server-authoritative regulated supplies (the claim
is recorded against the owner's confirmed data).** Recorded as a carried principle in the ADR so a future
reader cannot re-cross it quietly. Secondary note (also recorded): partial L2 coverage tracks data
provenance (owner-built `bom` vs scraped `place`), so "fewer chips" must not be read as "less healthy
dish" — a coverage caveat, not a health signal.

---

## Red lines — all intact after resolution

- Allergen presence-only / absence-never **AND** data-absence-never-reads-as-safe (markers + filter defer + guardrail).
- No platform-asserted regulated claims (owner-authority + verified-anchor gate, regulated subset OFF).
- Honesty-anchor = a real number required (kept; necessary-not-sufficient for regulated).
- Owner-authority for declarations (regulated opt-in; AllergenEditor wiring flagged as L3 prereq).
- Facts-not-verdict (no global winner; arrows only on non-regulated axes).
- Never-fabricate-composition (no auto-vegan; diet deferred).
- i18n (descriptive/taste/allergen al ship; regulated-term al review gated, L3 flag).

---

## Guardrails to author (red→green, before the respective flag flips)

1. **No-hide-allergen guardrail** (blocks STOP-1/C1 regression): CI/lint fails if any allergen-axis filter
   predicate removes dishes (hide/only-safe semantics) or if `…_ALLERGEN_FILTER` is wired to anything but a
   positive-only view. Red until the design ships positive-only; stays a permanent ratchet.
2. **Regulated-subset-off guardrail** (H2/BC-1): CI fails if a regulated label is emittable without an
   owner-authority flag + a populated verified-anchor entry. Red-on-disk until the anchor table exists.
3. **Body-effect denylist** (existing red line, keep): regulated body-effect phrasing → CI red.
4. **Suppress-on-modifier guardrail** (H1/BC-2): unit assertion that a product with axis-changing modifier
   groups emits no regulated/diet/allergen-sensitive label.
5. **No-clean-from-empty guardrail** (C2/M2): unit assertion that empty/absent `bom[].allergens` renders the
   "info not provided" marker regardless of `allergens_confirmed`, never a clean allergen surface.

---
---

# RESOLVE round 2

**Seat:** Architect, RESOLVE r2. Design-time only — NO production code.
**Inputs:** `breaker-findings.md` §RE-ATTACK round 2 (R2-C1, R2-H1..H4, R2-M1/M2, R2-L1) ·
`counsel-opinion.md` §RE-EXAMINE round 2 (STOP-2 + Q1/Q2/Q5 non-blocking refinements).
**Re-grounded @ HEAD (breaker facts, accepted):** `1790000000065_products-prep-time.ts:337` —
`prep_time_minutes int NOT NULL DEFAULT 15`, **single int, no min/max column**; grep
`CHARACTERISTIC_RULES|deriveCharacteristics|ALLERGEN_FILTER|char_hidden|"info not provided"` →
**0 hits** (all SHIP code + all guardrails + the marker are still prose — design-time, as intended).

## Verdict

**CONVERGED — to a SHIP scope reduced once more: TASTE-FIRST.** The round-1 fix-set introduced one new
CRITICAL (R2-C1 = STOP-2: the suppress-on-modifier wording deleted a true allergen WARNING) and reopened
two verdicts it had claimed closed (R2-H1 kcal arrow, R2-H3 marker not pinned). All are wording/spec/gating
defects in the resolution, not architecture defects — and all are fixed here by tightening the spec, not by
adding runtime. The architecture (pure edge derivation, zero migration, views-over-one-source) still holds.

**The decisive change:** SHIP is re-sequenced to **taste-first**. L1 taste (the only populated, zero-liability
surface) ships now. The **characteristics band** — descriptive-L2 chips, allergen-presence chips, AND the
universal "info not provided" marker — does **not render at all** until a per-tenant **coverage gate** clears
(enough real `bom`/allergen data to carry signal). This kills banner-blindness (R2-M1), rare-chip over-trust
(R2-L1), and the "0/49 surface built ahead of data" antipattern (Q5) in one move, without weakening any red line.

## STOP / blocking status

- **STOP-2 — DISCHARGED** by the R2-C1 carve-out below (a recorded wording fix + guardrail re-pointing).
- **STOP-1 — remains DISCHARGED** (round 1: filter deferred + red-on-disk no-hide guardrail).
- No new STOP opened. The remaining NEEDS-HUMAN gates are unchanged (regulated anchors; allergen-filter
  re-enable; al regulated-term review) plus one new operator value: the **per-tenant coverage threshold**
  for the characteristics band (a product/operator number, not a legal gate — see R2-M1/L1 below).

---

## Disposition table — RE-ATTACK / RE-EXAMINE round 2

| # | Sev | Finding (one line) | Disposition | What changes |
|---|-----|--------------------|-------------|--------------|
| **R2-C1 / STOP-2** | CRIT | "suppress, not caption" deletes a true allergen-PRESENCE warning on modifiable dishes; guardrail #4 would codify the deletion | **FIXED (carve-out)** | Suppression scoped to **reassuring/positive** labels ONLY (descriptive L2, diet, regulated subset, any "free-from"). **Allergen-presence WARNING is NEVER suppressed.** Guardrail #4 **rewritten to its positive complement** (a modifiable dish carrying a base allergen STILL renders its presence chip — red→green). Rule recorded: modifiers carry no allergen data (R-1) → a modifier can only ADD allergens, never honestly remove a base one → a base-dish warning is always conservatively true. STOP-2 discharged. |
| **R2-H3** | HIGH | "info not provided" marker not pinned on all SHIP surfaces (curation cap can evict it; comparison row renders blank) | **FIXED (invariant)** | (a) The allergen marker/chip is **exempt from the §6 2–3 curation cap** — a safety surface is never evicted by taste/descriptive chips. (b) The **comparison composition row must render the marker explicitly** for a no-data dish — never blank/"—". Invariant: on every SHIP surface (card, detail, comparison) absence-of-allergen-data renders an explicit "allergen info not provided", never nothing. |
| **R2-H2 + Counsel BC-1 refine** | HIGH | descriptive/regulated split is on the WORD; regulation is on MEANING — "protein-rich"/"filling" are regulated/body-effect by synonym, ship now | **FIXED (register split)** | Split is by **register/meaning, not token**. Denylisted **register** (energy/satiety/health/nutrient-content): "light", "low/high-in", "source of", "protein-rich", "filling", "keeps you full", "healthy", "good for", "fills you up", "satisfying"(as hunger) — regulated/banned regardless of synonym. Descriptive SHIP set avoids that register entirely: **drop "filling"** (→ "hearty-portion" / "generous-portion", a portion word), **drop "protein-rich"** (→ "protein-forward" only if it clears the register; else gate it). §2 enumeration aligned with §3.4 (regulated terms removed). New guardrail: a **denylist test over the rendered descriptive vocabulary** in **al AND en** (red if any descriptive label matches the regulated register). |
| **R2-H1** | HIGH | M1 self-contradiction — "kcal ↓ wins" arrow IS the regulated lightness verdict | **FIXED (no arrow on regulated/derived axes)** | **NO directional/"wins" arrow on calories or ANY regulated/derived axis.** Calories shown side-by-side as the **raw number only** (a fact) — no arrow, no colour-as-better. Arrows allowed **ONLY on price and prep-time**. Proposal example (`:102-103`) corrected. |
| **R2-M2** | MED | prep-time "RANGE" fabricated from a single int (false precision, inventing-uncertainty direction) | **FIXED (point estimate)** | Do **not** fabricate a min/max range. Show the single value honestly (**"~N min"**). A real range needs a real basis (defer to the same source delivery-ETA ranges come from, if/when it exists). The arrow on prep-time (R2-H1) is on this single value, not on an invented spread. |
| **R2-H4** | HIGH | 5 guardrails are prose, 0 in tree; the SHIP flip is not bound to them | **FIXED (gating spec, design-time)** | Each guardrail specified as a **concrete red→green artifact** (test/lint/grep) and the **flag flips bound to them**. SHIP flag **MUST NOT flip** until the **ship-gating** guardrails are authored red→green: **#1** (no hide/only-safe allergen predicate), **#4-positive** (modifiable+base-allergen still shows the chip), **#5** (no SHIP surface — card/detail/comparison — renders a clean allergen state from empty data), **#6** (descriptive-vocabulary denylist, al+en). Guard **#2** (regulated-subset-off) gates the *regulated* flag, not SHIP. Explicitly **specified-not-built** (design-time); building them red→green is the implementer's gate. |
| **R2-M1 + R2-L1 + Counsel Q5** | MED/LOW | near-empty SHIP → marker is wallpaper (banner-blindness), rare chip = provenance not merit, value-to-noise poor | **FIXED (taste-first + coverage gate)** | SHIP re-sequenced **taste-first**: L1 ships now (only populated, zero-liability surface). The **characteristics band** (descriptive-L2 + allergen-presence chips **+ the universal "info not provided" marker**) does **not render** until a **per-tenant minimum coverage** clears. Taste ships independently of the band. |
| **Counsel Q1 (non-blocking)** | — | reliance framing wrongly attached to the deferred filter | **FIXED (re-attached)** | "declared to contain" + ambient "confirm with the venue" reliance-bound **attached to the PRESENCE layer** (the only SHIP allergen surface), not the deferred filter. Presence row reads "declared to contain", never "complete allergen profile". |
| **Counsel Q2 (non-blocking)** | — | "filling" satiety claim; §2 enumeration lists regulated terms | **FIXED** | "filling" dropped (folded into R2-H2 register denylist); §2 enumeration corrected (see proposal edit). |

---

## R2-C1 / STOP-2 — the load-bearing resolve (full justification)

The round-1 H1/BC-2 fix said: on a modifiable product the "**regulated / diet / allergen-sensitive** label is
SUPPRESSED, not captioned." Read literally, "allergen-sensitive label" includes the allergen-**presence**
WARNING ("contains nuts"), and guardrail #4 ("emits no allergen-sensitive label") would **codify that
deletion**. A base dish that genuinely contains nuts + any axis-touching modifier group → warning removed →
reads nut-free → anaphylaxis-grade harm. That is strictly worse than the stale-reassurance label it replaced.

**The carve-out (recorded rule):**
- **Suppression applies ONLY to reassuring / positive labels** — descriptive L2, diet declarations, the
  regulated subset, and any "free-from"/reassurance copy. These are labels a modifier could **falsify** by
  making the dish "worse" (heavier, no-longer-vegan), so suppressing them on customization is protective.
- **The allergen-PRESENCE warning is NEVER suppressed.** Rationale: **modifiers carry no allergen data (R-1)**,
  so a modifier can only **ADD** allergens, never honestly **remove** a base one ("remove topping" does not
  clear cross-contamination). A base-dish allergen warning is therefore **always conservatively true** →
  suppressing it is never defensible.
- **Guardrail #4 is rewritten to the POSITIVE complement.** It no longer asserts "emits no allergen-sensitive
  label." It asserts: *a modifiable dish carrying a base allergen STILL renders its presence chip.* Red→green
  test: a product with axis-changing modifier groups **and** `bom[].allergens` containing X **must** show
  "contains X". The guard now **catches the deletion** instead of encoding it. (A separate assertion keeps the
  *reassuring* suppression: a modifiable dish emits no descriptive/diet/regulated chip.)

**STOP-2 discharged?** Yes. It was a wording/spec defect (an undefined term folded a warning into a suppression
rule). The carve-out is a recorded rule + a re-pointed guardrail; no architecture changes; the dangerous
direction can no longer be implemented as "guardrail-compliant".

## R2-H3 — pin the marker on every SHIP surface

The "info not provided" marker is the core safety primitive and must be unevictable:
- **Exempt from the §6 curation cap.** The 2–3-chip cap ranks taste/descriptive chips; the allergen
  marker/presence-chip row is a **separate, always-rendered safety surface**, never competing for a chip slot.
- **Comparison composition row renders the marker explicitly.** A no-data dish beside a "contains nuts" dish
  must show "allergen info not provided", never blank/"—" (a blank reads as "this one is nut-free" — the exact
  C1/M2 inversion in the highest-stakes side-by-side context).
- **Invariant (recorded):** on **every** SHIP surface — card, detail modal, comparison — absence-of-allergen-data
  renders an explicit "allergen info not provided", **never nothing**. Guardrail #5 is extended to assert this
  across all three surfaces.

## R2-H2 — split on REGISTER (meaning), not the word

Regulation (EU 1924/2006 + AL transposition) evaluates the **conveyed message**, not the token. A denylisted
**register** is regulated/banned regardless of synonym:
- **energy/lightness** — "light", "low-energy", "low-calorie", "low/high in";
- **nutrient-content** — "source of", "rich in", "protein-rich", "high-protein";
- **satiety/body-effect** — "filling", "keeps you full", "fills you up", "satisfying" (as a hunger claim);
- **health** — "healthy", "good for you", "guilt-free".

The **descriptive SHIP set must avoid that register entirely.** Concrete renderings: **drop "filling"** → a
portion/intensity word ("hearty-portion" / "generous-portion") that asserts size, not a body effect; **drop
"protein-rich"** → "protein-forward" is permitted **only if it clears the nutrient-content register** on
review — otherwise it is gated with the regulated subset. The §2 enumeration is corrected to remove
"light"/"protein-rich"/"light-bite". **Guardrail #6 (new):** a denylist test over the **rendered descriptive
vocabulary in al AND en** — red if any descriptive label, in either language, matches the regulated register.
This makes the split deterministic and language-aware (closes the al body-effect-drift risk, R-10, for the
descriptive surface).

## R2-H1 — no arrow on regulated/derived axes

A ↓ arrow on calories IS the "lighter wins" verdict the design deferred by gating "light". Therefore:
- **NO directional/"wins" arrow on calories or any regulated/derived axis** (kcal, macros, any threshold-derived
  value). They render **side-by-side as the raw number** — a fact, no arrow, no colour-as-better.
- **Arrows (↑/↓/≈) are allowed ONLY on price and prep-time** — non-regulated, well-defined, customer-neutral
  axes. The proposal example "protein ↑ / kcal ↓" (`:102-103`) is corrected to numbers-only on macros.

## R2-M2 — prep-time is a point estimate, not a fabricated range

There is only `prep_time_minutes` (one int). Synthesizing a min/max is **inventing uncertainty** (false
precision in the other direction). SHIP shows the **single value honestly ("~N min")**. A real range needs a
real basis — defer to wherever delivery-ETA ranges are sourced, or keep the point estimate. The prep-time
arrow (R2-H1) is on this single value.

## R2-H4 — guardrails as artifacts; flips bound to them (design-time spec)

These are **specified, not built** (design-time). The binding the implementer MUST honor:

| Guardrail | Concrete artifact | Gates which flip |
|---|---|---|
| **#1 No-hide-allergen** | lint/grep rule: fail if any allergen-axis predicate **removes** dishes (hide/only-safe) or `…_ALLERGEN_FILTER` is wired to anything but a positive-only view | **SHIP** (permanent ratchet) |
| **#4-positive Warning-never-suppressed** | unit test: modifiable dish + `bom[].allergens` ⊇ {X} **must** render "contains X"; AND a modifiable dish emits no descriptive/diet/regulated chip | **SHIP** |
| **#5 No-clean-from-empty** | unit test across card/detail/comparison: empty/absent `bom[].allergens` renders the marker (never blank, never clean) regardless of `allergens_confirmed` | **SHIP** |
| **#6 Descriptive-denylist** | test over rendered descriptive vocab in al+en: red if any label matches the regulated register | **SHIP** (descriptive band) |
| **#2 Regulated-subset-off** | CI: fail if a regulated label is emittable without owner-authority flag + populated verified-anchor entry; red-on-disk until anchor table exists | **regulated flag** (NOT ship) |
| **#3 Body-effect denylist** | existing red line, kept | all |

**Binding rule (recorded):** `MENU_CHARACTERISTICS_ENABLED` (and the band sub-surfaces) **must not flip on**
until **#1, #4-positive, #5, #6** are authored and proven **red→green**. #2 gates only the regulated flag.

## R2-M1 / R2-L1 / Q5 — taste-first + per-tenant coverage gate

On real/seed data: allergen-presence **0/49**, descriptive-L2 **0/49**, taste is the only populated surface.
A universal "info not provided" marker on ~100% of cards is **banner-blindness** (a control firing on 100% of
instances carries zero discriminating bits); the rare chip tracks **data provenance, not merit** (provenance
bias becomes the dominant visible behavior). Re-decided SHIP sequence:

- **Taste-first.** L1 taste ships now — it is the only populated, zero-liability surface, and is independent of
  the band.
- **Coverage gate on the band.** The characteristics band — descriptive-L2 chips, allergen-presence chips, AND
  the universal "info not provided" marker — does **NOT render** for a tenant until that tenant's data clears a
  **per-tenant minimum coverage** threshold (enough dishes carry `bom`/allergen data that the band carries
  signal rather than wallpaper). Below threshold the menu renders today's behavior (raw facts in the detail
  modal) — no band, no marker, no provenance-flag salience.
- **Taste ships independently** of the band and its coverage gate.

This is a **value/UX gate, not a safety gate**: it never lets absence read as clean (below threshold there is no
allergen surface at all — same as today; above threshold the marker is pinned per R2-H3). The threshold number
is an **operator/product decision (NEEDS-HUMAN)** — owner: product.

## Counsel non-blocking refinements — dispositions

- **Q1 reliance framing → PRESENCE layer (FIXED).** The round-1 resolution wrongly attached "confirm with the
  venue" to the deferred filter's disclosure. In SHIP scope the presence chips + marker are the **only** allergen
  surface, so the framing rides there: the presence row reads **"declared to contain"** with an ambient
  **"always confirm with the venue"** reliance-bound, so a sparse chip list never reads as a complete profile
  (closes Counsel's within-dish completeness gap).
- **Drop "filling" (FIXED).** Folded into the R2-H2 register denylist.
- **Fix §2 enumeration (FIXED).** Regulated terms removed from the descriptive count (proposal edit).

---

## Corrections to prior (round-1) dispositions — stated explicitly

- **M1 (round 1) OVERSTATED.** Round 1 listed "raw macro numbers shown as the number" among **arrow-eligible**
  axes and used the "kcal ↓" example — that arrow IS the lightness verdict (R2-H1). **Corrected:** arrows ONLY
  on price and prep-time; macros/regulated/derived axes are numbers-only, no arrow.
- **M2 / prep-time (round 1) OVERSTATED.** Round 1 called prep-time a "RANGE" to avoid false precision, but the
  data is a single int — a range would be fabricated (R2-M2). **Corrected:** point estimate "~N min".
- **C2/M2 marker "default everywhere" (round 1) was ASSERTED, not PINNED.** Round 1 said the marker is
  "first-class default, not a per-lens option" but never carved it out of the curation cap or pinned it into the
  comparison row (R2-H3). **Corrected:** explicit invariant + cap-exemption + comparison-row binding +
  guardrail #5 extended to all three surfaces.
- **BC-2 suppression (round 1) OVERSTATED safety.** The undefined "allergen-sensitive label" folded a true
  warning into suppression (R2-C1/STOP-2). **Corrected:** suppression = reassuring labels only; warning never
  suppressed; guardrail #4 re-pointed to the positive complement.
- **Counsel reliance language (round 1) attached to the wrong surface.** **Corrected:** moved to the presence
  layer (Q1).

---

## Red lines — all intact after RESOLVE round 2

- Allergen: **presence-only / absence-never** AND **data-absence-never-reads-as-clean** (coverage gate hides the
  band entirely below threshold; above threshold the marker is pinned on card/detail/comparison) AND
  **never-suppress-a-warning** (R2-C1 carve-out + guardrail #4-positive).
- No platform-asserted regulated claims (register-split + owner-authority + verified-anchor gate; guardrail #2/#6).
- Honesty-anchor = a real number required (kept; necessary-not-sufficient for regulated).
- Owner authority for declarations (regulated opt-in; AllergenEditor wiring still the L3 prereq).
- Facts-not-verdict (no global winner; **arrows only on price + prep-time**, never on a regulated/derived axis).
- Never-fabricate (no auto-vegan; **no synthesized prep-time range**; diet deferred).
- i18n (descriptive/taste/allergen al ship; descriptive denylist now enforced al+en; regulated-term al review
  still gated).

## Converged? + remaining NEEDS-HUMAN

**CONVERGED** to taste-first SHIP. No open CRITICAL/HIGH. STOP-1 and STOP-2 both discharged.

**Remaining NEEDS-HUMAN (gates, not ship blockers):**
1. **Per-tenant coverage threshold** for the characteristics band (NEW — product/operator number). Owner: product.
2. Coverage threshold + positive-only design sign-off before any allergen filter re-enables (STOP-1).
3. Verified per-market legal anchors (EU 1924/2006 + AL transposition) before the regulated L2 subset.
4. Per-market linguistic/legal review of regulated-term al renderings before regulated flip-on.

**Design-time note:** all six guardrails are **specified, not built**. Building #1, #4-positive, #5, #6 red→green
is the implementer's gate to flip `MENU_CHARACTERISTICS_ENABLED`; #2 gates the regulated flag.

---
---

# RE-EXAMINE round 3 (Counsel seat — final confirmation)

**Seat:** Counsel. Confirming the round-2 RESOLVE against the three questions put to this seat. A STOP is
only a grounded crossing of a recorded red line — not a matter of taste.

## 1. STOP-2 — fully discharged as recorded rule? **YES.**

The carve-out is complete and recorded, not merely asserted:
- Scope correctly narrowed — suppression applies **only to reassuring/positive** labels (descriptive L2,
  diet, regulated subset, "free-from"); the **allergen-PRESENCE warning is NEVER suppressed** (disposition
  R2-C1; §"R2-C1/STOP-2"; red-lines §"Red lines after RESOLVE round 2").
- Guardrail #4 is **rewritten to the positive complement** and is a real red→green artifact (R2-H4 table):
  *modifiable dish + `bom[].allergens` ⊇ {X} MUST render "contains X"*, plus a separate assertion that the
  *reassuring* chips are suppressed. The guard now **catches** the deletion instead of **codifying** it, and
  it is bound to the SHIP flip.
- The rule is grounded in a real datum (R-1: modifiers carry no allergen data → can only ADD, never honestly
  remove → a base-dish warning is always conservatively true), so it is principled, not ad-hoc.

Complete, recorded discharge. Confirmed.

## 2. NEW care crossing from the round-2 fixes? **YES — one grounded crossing (the coverage gate).**

The taste-first coverage gate (R2-M1/L1/Q5) **bundles the allergen-presence chips into the gated band**
(stated verbatim in the disposition row and §R2-M1: "descriptive-L2 chips, allergen-presence chips, AND the
universal 'info not provided' marker — does NOT render … until … coverage clears"). Consequence: a
low-coverage tenant that has *some* dishes with **authored** allergen data (e.g. 5/49 "contains nuts") has
those **true warnings de-elevated** below threshold. This re-crosses the line STOP-2 *just* grounded —
**never-suppress-a-warning** — through a different door.

Why it is a crossing and not yet discharged as recorded rule:
- The allergen-presence warning is **not explicitly exempt** from the coverage gate — it is explicitly placed
  *inside* the gated unit.
- The status-quo fallback ("raw facts in the detail modal", §R2-M1) is the only thing between the gate and
  harm, yet it is a single passing clause — **not recorded as a binding invariant and not bound to any
  guardrail.** Guardrail #5 fires only on rendered SHIP surfaces; below the gate those surfaces don't render,
  so #5 does not cover the below-threshold state. The guardrail gap is exactly at the new door.

Decisive coherence point (and the cheap fix): R2-M1's own banner-blindness argument is about the **no-data
"info not provided" marker firing on ~100% of cards** — zero discriminating bits = wallpaper. A **rare,
authored "contains X" warning is the opposite of wallpaper** — high-signal precisely because it is rare.
Folding it into the same gate is a category error: it sacrifices the one high-signal safety chip to solve a
low-signal noise problem. The gate's stated justification does **not** reach the true warning.

**Fix (one recorded rule, no architecture change):** decouple the value-gate from the safety surface. The
coverage gate may suppress the **descriptive-L2 chips and the universal "info not provided" marker** (the
actual wallpaper); a dish with **authored** allergen data renders its **presence warning regardless of tenant
coverage**. Equivalent acceptable form: record the below-threshold raw-allergen fallback as a binding
invariant + guardrail (any authored allergen datum surfaces at ≥ status-quo, and no reassuring
characterization renders below threshold). Either closes the door; neither costs runtime.

Steel-man of the option I decline (the resolution's position): *the gate removes the band uniformly, so no
reassurance context survives; below threshold the menu is byte-identical to today's chip-less production menu;
you cannot re-introduce suppression relative to a status quo that has no warnings to suppress.* Strong at the
**first order**. It fails at the **second order**: once above-threshold tenants train customers that "dowiz
flags allergens with chips," a habituated customer on a sparse-coverage menu reads *no chips → nothing flagged
→ safe* — a NEW false-clean read manufactured by the band's existence elsewhere, absent in today's uniformly
chip-less world. The harm bar is not only "worse than today in isolation"; it is "worse than the expectation
the product itself now sets." Hence the warning needs an explicit floor.

## 3. Both binding conditions + carried "safety duty stays with owner" — hold as recorded rule?

- **SHIP binding condition:** the flip-binding (`MENU_CHARACTERISTICS_ENABLED` must not flip until #1,
  #4-positive, #5, #6 are red→green) is **recorded and holds** (R2-H4 table + binding rule). Confirmed.
- **Regulated binding condition:** owner-authority + verified-anchor (guard #2/#6) gates the regulated flag —
  **recorded and holds**. Confirmed.
- **Carried principle "safety duty stays with owner":** holds **as written** (§"Where the safety duty lands",
  ADR-recorded). BUT the §2 coverage-gate crossing sits directly against it — gating a true warning shifts a
  sliver of reliance back toward a platform UX decision without a recorded floor. The principle is intact; the
  new gate needs the one recorded line above to remain *consistent* with it.

## Residual ETHICAL-STOP

- **STOP-3 (coverage-gate warning-suppression).** The per-tenant coverage gate must not hide an **authored**
  allergen-PRESENCE warning. Exempt the true-warning surface from the gate (gate only the no-data marker +
  descriptive chips), **or** record the below-threshold raw-allergen fallback as a binding invariant +
  guardrail. Grounded line: *never-suppress-a-warning* / *data-absence-never-reads-as-clean*. This is
  friction, not veto — a recorded human/implementer rule discharges it; it does **not** block the taste-first
  SHIP, which carries no allergen surface at all.

STOP-2: **discharged.** STOP-1: discharged (carried). STOP-3: **open** (new, narrow, cheaply closed).

## The question no one asked

The coverage threshold is framed as a **single per-tenant number**, but coverage is **two denominators**:
descriptive-L2 coverage and allergen coverage diverge (a tenant can have rich allergen data and sparse
descriptive data, or the reverse, per the provenance note in §"Where the safety duty lands" — owner-built
`bom` vs scraped-then-stripped `place`). A single combined number lets sparse **descriptive** coverage gate
off a well-covered **allergen** surface — the same STOP-3 crossing, arriving by arithmetic. Decide explicitly
whether the gate is one number or per-axis before the operator is handed "pick the threshold."

**Converged-from-counsel-seat: NO** — one grounded crossing (STOP-3) remains. One recorded line closes it;
re-confirm after.

---
---

# RESOLVE round 3

**Seat:** Architect, RESOLVE r3. Design-time only — NO production code.
**Inputs:** `breaker-findings.md` §RE-ATTACK round 3 (R3-H1, R3-H2, R3-H3, R3-M1) ·
`resolution.md` §RE-EXAMINE round 3 (STOP-3 + the two-denominator "question no one asked").
**Re-grounded @ HEAD (the load-bearing datum):** `MenuPage.tsx:1078` — the detail modal renders the
allergen section **only** when `bomToNutrition(detailProduct).allergens.length > 0`; **empty allergens
render NOTHING in the detail today, and a non-empty list renders with NO completeness caveat.** This is the
exact surface the floor + the reliance-bound below must change. `bom[].allergens` is the only authored
allergen source; `prep_time_minutes` is still a single `int`; seed/scrape allergen reality still 0/49; the
rule module + all guardrails + the marker are still prose (grep ~0). Design-time, as intended.

## Verdict

**CONVERGED.** The three round-3 HIGHs and the MED are all **spec/scoping defects in the round-2 resolution,
not architecture defects** — and all are fixed here by *narrowing what the coverage gate touches* and
*inverting a guardrail's polarity*, adding **zero runtime**. The architecture (pure edge derivation, zero
migration, views-over-one-source, taste-first) still holds unchanged.

**The decisive move (R3-H1/STOP-3):** the coverage gate is re-scoped to the **descriptive axis ONLY**. The
**allergen axis is removed from the gate entirely.** An authored allergen WARNING ("contains X") and the
always-on detail-modal allergen truth are **never coverage-gated**. The gate now touches only noise
(descriptive chips + the at-a-glance *no-data marker on the card*), never a warning. This discharges STOP-3
without moving the warning behind a different door (the explicit door-check is in §R3-H1 below).

## STOP / blocking status

- **STOP-3 — DISCHARGED** by the R3-H1 re-scope + the recorded detail-floor invariant + guardrail re-pointing.
- **STOP-1 — remains DISCHARGED** (allergen filter deferred + no-hide guardrail).
- **STOP-2 — remains DISCHARGED** (modifier-path warning carve-out + #4-positive).
- No new STOP opened. The new NEEDS-HUMAN is the **descriptive allowlist contents (EN+AL review)** — and it
  may be **empty for v1** (taste-first), in which case the descriptive band ships nothing and the gate is
  dormant.

---

## Disposition table — RE-ATTACK / RE-EXAMINE round 3

| # | Sev | Finding (one line) | Disposition | What changes |
|---|-----|--------------------|-------------|--------------|
| **R3-H1 / STOP-3** | HIGH | The taste-first coverage gate bundles the allergen-presence WARNING + marker into the gated band → a real "contains nuts" chip is hidden below threshold; contradicts unconditional guardrails #5 / #4-positive | **FIXED (re-scope the gate)** | **The authored allergen WARNING ("contains X", from non-empty `bom[].allergens`) is NEVER coverage-gated** — it renders at-a-glance on card + detail + comparison ALWAYS, regardless of tenant coverage. **Coverage gate scope = descriptive-L2 chips ONLY** (and the at-a-glance *no-data marker on the card*). **Below-gate safety floor = recorded INVARIANT + guardrail:** the detail modal ALWAYS renders the allergen truth — "contains X" or an explicit "allergen info not provided" — regardless of coverage. #4-positive + #5 rewritten to be authorable red→green by scoping to the *ungated warning* + the *always-on detail*, so they no longer contradict the gate. |
| **R3-H2** | HIGH | Register *denylist* (#6) is a token/substring test; survivors "hearty"/"rich"/"carb-forward" convey banned meaning; "protein-forward only if it clears review" punts to human judgment → #6 not authorable green | **FIXED (invert to allowlist)** | #6 inverted to a **closed, human-reviewed ALLOWLIST**: the descriptive SHIP vocabulary is a SMALL FIXED SET, each label individually reviewed (EN+AL) to clear the regulated/satiety/energy/health/nutrient-content register. Guardrail #6 becomes deterministic: **rendered descriptive set ⊆ reviewed allowlist** (red if any label outside it renders). "hearty"/"rich"/"carb-forward"/"protein-forward" do NOT ship until each clears review. **v1 descriptive allowlist = EMPTY** pending review (taste-first) → the descriptive band renders nothing and the gate is dormant. (NEEDS-HUMAN: allowlist contents + AL review.) |
| **R3-H3** | HIGH | Marker fires only on EMPTY allergens; a non-empty-incomplete declaration (`['milk']`) reads as complete → "milk only ⇒ no nuts" beside a "contains nuts" dish | **FIXED (carried reliance bound)** | The allergen presence layer ALWAYS carries the reliance bound: framed **"declared to contain …"** + a **persistent** "not a complete allergen list — confirm with the venue" note attached directly to the allergen surface (card affordance + detail), so a partial declaration NEVER reads as the exhaustive truth. In comparison, **neither dish's allergen cell may imply exhaustiveness.** Recorded as the carried reliance principle on the SHIP presence layer (the honest floor of any presence-only system), not on the deferred filter. |
| **R3-M1** | MED | #1's "positive-only" clause is not a falsifiable grep — only "filter absent" is checkable | **FIXED (spec correction)** | #1's SHIP-checkable part = **"no hide/only-safe allergen filter predicate ships"** (the filter is absent from SHIP → greppable: no allergen filter is wired into the rendered set). The **positive-only semantics are a recorded design rule for the deferred re-enable** (the NEEDS-HUMAN gate), **not** a SHIP grep. Spec corrected to stop claiming the grep mechanically enforces positive-only. |
| **Two-denominator** (Counsel) | — | Coverage has two diverging denominators (descriptive vs allergen by provenance); a single number lets sparse descriptive gate off well-covered allergen | **RESOLVED (dissolved)** | Since the resolve **ungates the allergen axis entirely**, the coverage gate applies to the **descriptive axis ONLY → ONE denominator (descriptive coverage).** The operator threshold gates **descriptive chips only**; allergen warnings + the detail floor are never gated. The allergen denominator is irrelevant to gating because the allergen axis is not gated. The ambiguity dissolves; the operator is handed one unambiguous number. |

---

## R3-H1 / STOP-3 — the load-bearing resolve (full justification + the door-check)

**The crossing (accepted as stated).** Round 2 placed "allergen-presence chips AND the universal 'info not
provided' marker" *inside* the coverage-gated band (`resolution.md:344-346`, `proposal.md:29`). A tenant at
3/49 allergen coverage — one dish genuinely `bom[].allergens=['nuts']` — sits **below threshold → the band
does not render → the at-a-glance "contains nuts" chip is hidden.** That re-crosses **never-suppress-a-warning**
through the *coverage* door (after R2-C1 closed the *modifier* door), and makes the two SHIP-gating guardrails
(#5, #4-positive, specified *unconditional*) un-authorable against a gate that hides their subject.

**The coherent model (verified against `MenuPage.tsx:1078`).**

1. **The authored allergen WARNING is NEVER coverage-gated.** A non-empty `bom[].allergens` → "declared to
   contain X" renders at-a-glance on **card + detail + comparison ALWAYS**, regardless of tenant coverage. A
   warning is *high-signal* — it is the **opposite** of the banner-blindness the gate exists to fix (R2-M1's
   wallpaper argument is about a no-data marker firing on ~100% of cards = zero discriminating bits; a *rare
   authored warning* is high-signal *because* it is rare). Folding the warning into the gate was a category
   error; the resolve removes it.
2. **Coverage gate scope = descriptive-L2 chips ONLY** (and the at-a-glance *no-data marker on the card* — the
   actual noise). **The ALLERGEN axis is NOT coverage-gated.**
3. **Below-gate safety floor — a recorded INVARIANT + guardrail (not a passing clause).** The detail modal
   ALWAYS renders the allergen truth — either "declared to contain X" or an explicit "allergen info not
   provided" — **regardless of coverage.** So a below-gate card may be *minimal* (taste only, no false-clean
   chip), but the one-tap detail never shows a blank/clean allergen state. This must change `MenuPage.tsx:1078`
   (which today renders the section *only* on `length > 0`) so the empty branch renders "info not provided".
4. **Guardrails rewritten so they are authorable red→green and no longer contradict the gate:**
   - **#5 (No-clean-from-empty), rewritten:** "On every surface that renders an allergen state — **card
     warning always; detail always** — empty `bom[].allergens` renders 'allergen info not provided', never
     nothing, regardless of `allergens_confirmed`; and a non-empty list never renders as a clean/complete
     state. The *card-level no-data marker* may be gated (below the descriptive gate it need not render on the
     card), but **the detail floor is unconditional.**" → assertable: (a) detail-empty ⇒ "info not provided"
     (unconditional); (b) card-non-empty ⇒ "contains X" (unconditional); (c) no surface ⇒ clean-from-empty.
   - **#4-positive (Warning-never-suppressed), rewritten:** "A modifiable dish with `bom[].allergens ⊇ {X}`
     MUST render 'contains X' on the card **regardless of modifier state AND regardless of coverage**; AND a
     modifiable dish emits no descriptive/diet/regulated chip." → assertable without choosing between gate and
     guardrail, because the warning surface is now *outside* the gate.

**Door-check — does ungating the warning + the detail-floor invariant open a NEW door (the failure pattern of
rounds 1 and 2)? Stated explicitly: NO.** Reasoning, exhaustively:
- *Card-silence below gate.* Below the descriptive gate, a **no-data** dish shows no chip on the card (the
  card-level no-data marker is gated as noise). Could that read as "clean"? It is **byte-identical to today's
  pre-feature card** (today has no allergen chips at all), so it manufactures **no new clean-read at the card
  level** — and the detail floor now shows "info not provided" where today it showed *nothing*, so below-gate
  is **strictly better than today**, never worse.
- *Counsel's second-order habituation argument.* Above-threshold tenants train customers that "dowiz flags
  allergens with chips." On a sparse menu, does "no chip ⇒ safe" get manufactured? **No — because the warning
  chip is now ungated:** a dish that contains nuts shows "contains nuts" on a sparse menu too. The habituation
  "warning chip appears iff there is an authored allergen" stays **true** at every coverage level. The only
  thing gated is the *no-data marker* (the "we don't know" signal) and *descriptive* chips — never the
  positive warning. So the expectation the product sets ("we surface declared allergens") is honored
  uniformly; the harm bar Counsel raised ("worse than the expectation the product itself now sets") is met.
- *Residual, named and bounded.* The one residual is the **gated card-level no-data marker**: above gate a
  no-data dish shows the marker on the card; below gate it does not (only in detail). This is the deliberate
  R2-M1 wallpaper trade — the card marker is noise (fires on ~100% of cards at 0/49), the detail floor is
  signal (always truthful one-tap-away). The split puts **noise behind the gate, the floor never gated.** No
  warning, and no clean-read, sits behind any door.

STOP-3 discharged: the warning is ungated; the gate touches only noise; the floor is a recorded invariant +
guardrail (#5/#4-positive rewritten to be red→green and gate-consistent).

## R3-H2 — invert the denylist to a closed reviewed allowlist

A token/substring *denylist* cannot catch *conveyed meaning* ("hearty"=satiety, "rich"=energy-density,
"carb-forward"=nutrient-content) and "protein-forward only if it clears review" punts to human judgment — so
#6 as a denylist is **not authorable red→green**. Inverted:

- **The descriptive SHIP vocabulary is a SMALL, CLOSED, human-reviewed ALLOWLIST.** Each label is individually
  reviewed (EN **and** AL) to clear the regulated/satiety/energy/health/nutrient-content register **before** it
  enters the allowlist.
- **Guardrail #6 becomes deterministic:** `rendered descriptive set ⊆ reviewed allowlist` — **red if any label
  outside the allowlist renders.** A subset check is mechanical (no human judgment at test time); the human
  judgment is moved *upstream* into populating the allowlist, exactly once per label.
- **"hearty"/"rich"/"carb-forward"/"protein-forward" do NOT ship** until each individually clears review.
- **What is in the descriptive allowlist for v1?** **Nothing pre-cleared** — per R3-H2 each candidate carries
  a survivor register, and none has passed EN+AL review. So **the v1 descriptive allowlist is EMPTY.** This is
  fine and consistent with **taste-first**: v1 ships taste (+ the ungated allergen warning + detail floor) and
  the descriptive band renders nothing until the allowlist is populated. Consequence: the coverage gate is
  **dormant at v1** (an empty allowlist renders no descriptive chips regardless of coverage); it activates only
  when a human adds the first reviewed label. (NEEDS-HUMAN: the allowlist contents + AL review.)

## R3-H3 — the carried reliance bound on the presence layer (partial ≠ complete)

The marker firing only on EMPTY `bom[].allergens` leaves the **partial** case unguarded: `['milk']` →
`length>0` → no marker → "declared to contain milk" → beside "contains nuts" reads "milk only ⇒ no nuts". The
system has no review-completeness concept; a presence-only system cannot acquire one without owner data it does
not have. The honest floor:

- **The allergen presence layer ALWAYS carries the reliance bound.** Framing is **"declared to contain …"**
  (not "contains", not "allergens:") + a **persistent, surface-attached** note: **"not a complete allergen
  list — confirm with the venue."** Attached directly to the allergen surface — **card affordance + detail** —
  not as low-salience ambient prose (the round-2 Q1 ambient line was overridden by the high-salience visual;
  this round it rides *on* the surface).
- **A partial declaration NEVER reads as the complete/exhaustive truth.** "Declared to contain milk" carries
  its own incompleteness caveat in place, so the nut-allergic customer cannot read it as a closed list.
- **In comparison, neither dish's allergen cell may imply exhaustiveness** — every cell (warning, no-data
  marker, or partial list) carries the same "declared, not complete; confirm with venue" bound; no cell reads
  as "this one is clear of X".
- Recorded as the **carried reliance principle on the SHIP presence layer** (the honest floor of any
  presence-only system), NOT on the deferred filter.

**Door-check.** Does the persistent caveat dilute the warning itself? No — it strengthens honesty uniformly:
the caveat says "list may be incomplete", which never *weakens* a present "contains X" (a warning that is also
"possibly incomplete" is still a warning, only more conservative). It closes the partial-as-complete inversion
without creating a clean-read anywhere.

## R3-M1 — correct guardrail #1's checkable scope

#1's deterministic SHIP assertion is **"no hide/only-safe allergen filter predicate ships"** — i.e. the
allergen filter is **absent** from the rendered set (greppable: no allergen-axis predicate removes dishes,
because no allergen filter is wired at all in SHIP). The **"positive-only" forward guarantee** (contains-X,
never hide→safe) is a **recorded design rule for the deferred re-enable** (the NEEDS-HUMAN gate), **not** a
mechanical SHIP grep — a grep cannot decide a predicate's semantics. Spec corrected so #1 no longer overstates
what the gate catches: it catches *presence/absence of an allergen filter*, and the positive-only shape is a
human-reviewed condition of re-enable.

## Two-denominator question — dissolved to ONE denominator

Counsel asked whether the coverage threshold is one number or per-axis, given descriptive and allergen coverage
diverge by provenance (owner-built `bom` vs scraped-then-stripped `place`). **The resolve dissolves it:** the
allergen axis is **ungated entirely** (R3-H1), so the coverage gate applies to the **descriptive axis ONLY**.
There is therefore **ONE denominator: descriptive coverage.** The operator threshold is unambiguous — it gates
**descriptive chips only**; allergen warnings + the detail floor are never gated and have no threshold. A
sparse-descriptive / rich-allergen tenant cannot have its allergen surface gated off by arithmetic, because the
allergen surface is not behind the gate. (And at v1 the descriptive allowlist is empty → the single denominator
gates an empty set → the gate is dormant until a reviewed descriptive label exists.)

---

## Corrections to prior dispositions — stated explicitly

- **R2-M1/L1/Q5 (round 2) OVERSTATED safety scope.** Round 2 bundled the allergen-presence chips + marker
  *into* the coverage-gated band, which hid a true warning below threshold (R3-H1/STOP-3). **Corrected:** the
  coverage gate is descriptive-axis-ONLY; the allergen warning + detail floor are never gated.
- **R2-H3 invariant (round 2) was in flat contradiction with the R2-M1 gate.** "Marker on every SHIP surface,
  never nothing" (`:289`) vs "band incl. marker does not render below threshold" (`:344-346`). **Corrected:**
  the *card-level no-data marker* may be gated as noise; the *detail-floor allergen truth* is unconditional;
  the *warning* is never gated. #5 rewritten to this shape (authorable red→green).
- **R2-H2 register split (round 2) was a denylist that could not be authored green.** **Corrected:** inverted
  to a closed reviewed allowlist (subset check); v1 allowlist empty.
- **Counsel Q1 (round 2) attached the reliance bound as low-salience *ambient* prose.** **Corrected:** the
  reliance bound is **persistent + surface-attached** (rides on the allergen surface itself), and now also
  closes the *partial-declaration* completeness gap (R3-H3), not just the sparse-list gap.
- **Guardrail #1 (round 2) overstated mechanical enforcement of "positive-only".** **Corrected** per R3-M1.

## Red lines — all intact after RESOLVE round 3

- **Allergen: never-suppress-a-warning — now across ALL mechanisms.** An authored allergen warning is NEVER
  suppressed by **modifier (R2-C1) OR coverage (R3-H1) OR curation cap (R2-H3)**. The warning surface is
  outside every gate.
- **Data-absence never reads as a clean allergen state on any surface.** The detail floor unconditionally
  renders "contains X" or "info not provided"; the card never shows a false-clean chip; partial declarations
  carry the "not a complete list" bound (R3-H3) so a partial never reads as exhaustive.
- No platform-asserted regulated claims (allowlist is reviewed-clear-of-register; regulated subset OFF;
  guardrail #2/#6).
- Honesty-anchor = a real number required (kept; necessary-not-sufficient for regulated).
- Owner authority for declarations (regulated opt-in; AllergenEditor wiring still the L3 prereq).
- Facts-not-verdict (no global winner; arrows only on price + prep-time).
- Never-fabricate (no auto-vegan; no synthesized prep-time range; no fabricated allergen completeness).
- i18n (taste + allergen al ship; descriptive allowlist is EN+AL-reviewed per label before it enters;
  regulated-term al review still gated).

## Updated guardrail set (red→green; SPECIFIED, NOT BUILT — design-time)

SHIP flag bound to **#1, #4-positive, #5, #6**; **#2** gates only the regulated flag.

| Guardrail | Concrete artifact (round-3 form) | Gates |
|---|---|---|
| **#1 No-hide-allergen** | grep/lint: **no allergen-axis filter predicate is wired into the rendered set** (filter absent from SHIP). The "positive-only" shape is a recorded re-enable design rule, NOT this grep (R3-M1). | SHIP (ratchet) |
| **#4-positive Warning-never-suppressed** | unit: modifiable dish + `bom[].allergens ⊇ {X}` MUST render "contains X" on the card **regardless of modifier state AND coverage**; AND a modifiable dish emits no descriptive/diet/regulated chip. | SHIP |
| **#5 No-clean-from-empty (+ detail floor)** | unit: (a) detail modal with empty/absent `bom[].allergens` renders "info not provided" **unconditionally** (regardless of coverage / `allergens_confirmed`) — fixes `MenuPage.tsx:1078`; (b) card with non-empty allergens renders "contains X" unconditionally; (c) no surface renders a clean state from empty data. | SHIP |
| **#6 Descriptive-allowlist** | unit: rendered descriptive vocabulary (EN+AL) **⊆ the closed reviewed allowlist** — red if any label outside it renders. (v1 allowlist empty → descriptive band renders nothing.) Human review is upstream, per label; the test is a mechanical subset check. | SHIP (descriptive band) |
| **#2 Regulated-subset-off** | CI: regulated label not emittable without owner-authority flag + populated verified-anchor entry; red-on-disk until anchor table exists. | regulated flag |
| **#3 Body-effect denylist** | kept (existing red line). | all |

## Converged? + remaining NEEDS-HUMAN

**CONVERGED.** No open CRITICAL/HIGH. STOP-1, STOP-2, STOP-3 all discharged. The round-3 fixes were
re-scoping + guardrail-polarity-inversion + a recorded reliance principle — **zero new runtime, zero new
door** (door-checks stated for R3-H1 and R3-H3).

**Remaining NEEDS-HUMAN (gates, not ship blockers):**
1. **Descriptive allowlist contents + AL review** (NEW — each candidate label individually cleared of the
   regulated register in EN+AL before it enters the allowlist; v1 may be empty). Owner: product + Counsel/i18n.
2. **Per-tenant descriptive-coverage threshold** (the single, now-unambiguous denominator — descriptive only).
   Owner: product. (Dormant while the allowlist is empty.)
3. Verified per-market legal anchors (EU 1924/2006 + AL) before the regulated L2 subset. Owner: Counsel/legal.
4. Per-market linguistic/legal review of regulated-term al renderings before regulated flip-on. Owner:
   Counsel/legal + i18n.
5. Coverage threshold + positive-only design sign-off before any allergen filter re-enables (STOP-1). Owner:
   Counsel/legal + product.

**Design-time note:** all guardrails remain **specified, not built**. Building #1, #4-positive, #5
(incl. the `MenuPage.tsx:1078` detail-floor fix), #6 red→green is the implementer's gate to flip
`MENU_CHARACTERISTICS_ENABLED`; #2 gates the regulated flag.

---
---

# RESOLVE round 4

**Seat:** Architect, RESOLVE r4. Design-time only — NO production code.
**Inputs:** `breaker-findings.md` §RE-ATTACK round 4 (**R4-H1** HIGH — the contrast door; **R4-M1** MED —
reliance-bound omnipresence vs the gated marker). No new Counsel RE-EXAMINE round-4 / ETHICAL-STOP filed
(the standing STOP-1/STOP-2/STOP-3 remain discharged; this round resolves a Breaker-found regression).
**Re-grounded @ HEAD (the load-bearing datum, verified this round):** `MenuPage.tsx:1078` —
`{(bomToNutrition(detailProduct).allergens.length > 0) && (…)}`: allergens render **ONLY in the detail
modal, only on `length > 0`, with NO completeness caveat, and NOTHING on empty.** **There is NO
card-level allergen surface anywhere today** (no allergen chip on any card). So today's card carries
**zero** allergen information — silence is uniform across all cards → **no contrast.** This is exactly the
baseline R4-H1 says round 3 broke, and it is the baseline this resolve returns the card to. Rule module /
guardrails / marker / reliance-bound still unbuilt (grep ~0), `prep_time_minutes` still single `int` —
design-time, as intended.

## Verdict

**CONVERGED.** R4-H1 is a **spec defect in the round-3 resolution** (an asymmetric card surface), not an
architecture defect, and it is the **fourth instance of the same failure pattern**: a fix relocated a risk
instead of closing it (r1 modifier-door → r2 coverage-door → r3 marker-gating → **r4 contrast-door**). This
round closes it in a way that **structurally cannot create a 5th door**, because the fix does not *relocate*
the safety element to a new asymmetric position — it **bonds the two halves of the card allergen surface
into one inseparable rendering unit** (warning + no-data marker), so the asymmetry the contrast bug
*requires* becomes unbuildable. Zero new runtime. The taste-first / pure-edge-derivation / views-over-one-
source / descriptive-allowlist architecture is unchanged.

**The decisive move (R4-H1): DETAIL-FLOOR-ONLY for v1 + the card-allergen-surface ALL-OR-NOTHING invariant.**
In v1 **no allergen element is elevated to the card at all** — no warning chip, no no-data marker. The card
allergen surface = **exactly today's behavior (allergens detail-only).** The authored warning *and* the
no-data state both live in the **always-on detail modal** (floor fix: empty → "allergen info not provided",
never blank) **+ the reliance bound.** The at-a-glance **card** allergen surface — warning **and** marker
**together, as one unit** — becomes a **LATER, allergen-coverage-gated feature**: when (and only when) a
tenant's allergen coverage is real, **BOTH turn on together**, so a no-data card shows the marker, never a
clean blank — no contrast at any coverage level.

## STOP / blocking status

- **STOP-3 — remains DISCHARGED, and is honored more cleanly than in round 3.** STOP-3's actual requirement
  is *the warning is never gated BELOW its current (detail) visibility.* Today the warning lives in the
  detail modal (`MenuPage.tsx:1078`). DETAIL-FLOOR-ONLY keeps the warning in the detail modal **always,
  unconditionally** (improved with the floor + reliance bound) → the warning is **never less visible than
  today**, at any coverage level. Round 3's elevation of the warning *to the card* was an **enhancement
  beyond STOP-3's floor** that backfired (the contrast door); reverting it to the detail floor returns the
  warning *to* its current location, **never below it.** STOP-3 holds.
- **STOP-1 — remains DISCHARGED** (allergen filter deferred + no-hide guardrail).
- **STOP-2 — remains DISCHARGED** (modifier-path warning carve-out + #4-positive, re-scoped to the detail
  floor below).
- **No new STOP opened.** New NEEDS-HUMAN: the **allergen-coverage threshold** for the *later* card
  allergen unit (a product value, dormant in v1 since the card unit is OFF) — not a ship blocker.

---

## Disposition table — RE-ATTACK round 4

| # | Sev | Finding (one line) | Disposition | What changes |
|---|-----|--------------------|-------------|--------------|
| **R4-H1** | HIGH | Round 3 ungated the warning onto the card but left the no-data marker gated → on a partially-authored menu the 5 warning cards + 35 blank cards make the 35 read "nut-free by contrast" (new vs today); door-check missed it by evaluating a card in isolation | **FIXED (DETAIL-FLOOR-ONLY + all-or-nothing invariant)** | **Option DETAIL-FLOOR-ONLY chosen.** v1 elevates **NO** allergen element to the card (no warning chip, no marker) — card allergen surface = today's behavior (detail-only). The authored **warning + the no-data state both live in the always-on DETAIL** (floor: empty → "allergen info not provided", never blank) + the reliance bound. **INVARIANT:** the card never shows an authored allergen warning unless it ALSO shows the no-data marker on no-data cards — warning + marker are **one unit, gated together or ungated together, NEVER split.** The card allergen UNIT becomes a **LATER allergen-coverage-gated** feature (both halves on together). **Guardrail #5 = the unconditional detail floor; NEW guardrail #7 = card-allergen all-or-nothing (anti-contrast):** a build where the card renders a warning chip but a no-data card renders no marker is **RED.** #4-positive re-scoped: warning floor is the **detail** (always); the card warning rides only with the card unit (which carries the marker). **Corrects the round-3 door-check overstatement** "no clean-read sits behind any door" — see §below. |
| **R4-M1** | MED | The "not a complete list" reliance bound on 100% of allergen cells is the same omnipresence the design gates the no-data marker to avoid — noise in one place, mandated signal in another | **RESOLVED (dissolved by DETAIL-FLOOR-ONLY)** | With v1's allergen surface = **detail-only**, the reliance bound lives **in the detail** (where the allergen truth lives), not on every card. The card carries **no allergen element at all** in v1 → **no omnipresent card note** → the noise/signal inconsistency dissolves. The bound rides the surface that actually renders allergen content (detail; later the card unit + comparison cells), exactly where reliance is being placed — never as wallpaper on cards that assert nothing. Recorded so a future reader does not re-spread the bound onto an otherwise-silent card. |

---

## R4-H1 — the load-bearing resolve (full justification + the option choice)

**The crossing (accepted as stated, and re-verified against source).** Round 3 closed the warning-
suppression doors by **ungating the warning onto the card** (`proposal.md:30` r3) while **gating the
no-data marker** behind the descriptive coverage gate as "noise" (`proposal.md:29,152` r3;
`resolution.md:583-584`). On a partially-authored menu — the **expected** owner behavior (R3-H3: "list the
obvious allergen, forget the rest"), e.g. 5/40 `bom[].allergens=['nuts']`, 35 no-data — and with the v1
descriptive allowlist empty (gate closed for all), the result is: **5 cards show "declared to contain
nuts", 35 cards show nothing.** A nut-allergic customer scanning the menu reads the 35 blanks as
**nut-free by contrast.** This is `data-absence-reads-as-clean` at the **card/menu-scan surface** — new
vs today, because today **no card carries any allergen chip** (`MenuPage.tsx:1078`, verified), so today's
silence is **uniform** and means "open detail to see," not "this is clean relative to that one." The
asymmetry — *warning on-card, marker off-card* — is the bug.

**The choice — DETAIL-FLOOR-ONLY (recommended) vs BOTH-ON-CARD-ALWAYS.**

- **BOTH-ON-CARD-ALWAYS** (warning + marker both ungated on the card always) *does* close the contrast —
  no card is ever blank on allergens, so no blank-vs-warning contrast exists. But at v1's live denominator
  (0/49 → the no-data marker fires on ~100% of cards) it **re-creates R2-M1 banner-blindness** (a signal on
  100% of cards carries zero discriminating bits = wallpaper), the exact problem round 2 solved. **Rejected
  for v1.**
- **DETAIL-FLOOR-ONLY** (chosen). v1 elevates **no** allergen element to the card. The card returns to
  **today's exact state** (uniform allergen-silence — verified clean of contrast by the breaker's own
  grounding). The warning + the no-data state both live in the **always-on detail modal** with the floor
  fix and the reliance bound. This:
  1. **Eliminates the contrast** — there is no card allergen element to be asymmetric, so no blank-vs-
     warning contrast can form on the card.
  2. **Keeps banner-blindness away** — no no-data marker on cards at all in v1 (the wallpaper never renders).
  3. **Keeps the warning at exactly today's visibility** — detail, always (STOP-3 honored; the warning is
     never gated below its current location).
  4. **Makes the card allergen surface a clean coverage-gated UNIT for later** — warning + marker turn on
     **together**, keyed on the **allergen** denominator (not the descriptive one), so a no-data card always
     shows the marker, never a clean blank, at any coverage level.

**THE INVARIANT (recorded, load-bearing).** *The card never shows an authored allergen warning unless it
ALSO shows the no-data marker on no-data cards.* Warning and no-data marker on the card are **one
inseparable rendering unit** — gated together or ungated together, **NEVER split.** In v1 the unit is
**OFF** (both absent → symmetric → no contrast). When the unit later turns on (above an allergen-coverage
threshold), **both** render (warning where authored, marker where no-data → no blank → no contrast). There
is no coverage level at which one half renders without the other.

**Why this CANNOT create a 5th door (the thing the brief demands be proven).** Every prior round's new door
was created by *relocating* a safety element to a new position **asymmetrically** (the moved half left a gap
its counterpart did not fill). DETAIL-FLOOR-ONLY does the **opposite of relocate**: it **removes** the
entire card allergen surface in v1, returning the card to the pre-feature baseline the breaker *itself
certified* has no contrast ("today no card carries any allergen chip … there is no contrast"). **Removing a
surface to a baseline that is already proven contrast-free cannot manufacture a new contrast that did not
exist at that baseline.** And for the *later* card unit, the **all-or-nothing invariant makes asymmetry
structurally impossible**: the contrast bug *requires* one half (warning) to render while the other half
(marker) does not — an inseparable unit has no such state. Guardrail #7 makes this **deterministic** (RED on
any build where a card warning renders without the marker). A door requires an asymmetry; the fix bonds the
two halves so no asymmetry can exist; therefore no door. This is the structural difference from rounds 1–3:
those fixes left a seam (a moved element vs its stationary counterpart); this fix **welds the seam shut.**

## DOOR-CHECK (exhaustive — evaluated in PAIRS, not isolation; the round-3 error corrected)

**Method correction (recorded).** The round-3 door-check evaluated a no-data card **in isolation**
("byte-identical to today's pre-feature card → no new clean-read") and concluded "**no clean-read sits
behind any door**" (`resolution.md:606-608,620`). **That conclusion was FALSE and is hereby retracted.** The
no-data card was *not* byte-identical to today, because round 3 had **also** placed a warning chip on
sibling cards — relative to which the blank read clean. **A clean-read is a relation between dishes on a
screen, never a property of one card.** The standing door-check method is therefore: **evaluate allergen
surfaces in PAIRS, at every coverage level, for every dish state.** Below, every cell is walked.

Dish states: **A** = authored-contains-X (`bom[].allergens=['nuts']`); **P** = partial declaration
(`['milk']`, non-empty but incomplete); **N** = no-data (empty/absent). Coverage levels: **0%, partial,
high.**

| Surface | v1 behavior (DETAIL-FLOOR-ONLY) | Pair test (A beside N / P beside A) | (a) warning ≥ today? | (b) any contrast clean-read? | (c) fabricated absence? |
|---|---|---|---|---|---|
| **Card** (at-a-glance) | **No allergen element at all** (warning + marker unit OFF in v1). Identical to today. | All cards uniformly allergen-silent → **no pair can contrast** (A and N cards look identical: neither asserts anything). To learn anything the customer must open detail. | **=today** (warning is detail-only today; unchanged). | **NO** — uniform silence = today; no card asserts allergen state, so no blank reads "clean" relative to anything. | **NO** — card asserts nothing. |
| **Card — LATER unit ON** (above allergen-coverage gate) | Warning **and** marker render **together** (unit). A→"declared to contain nuts"; N→"allergen info not provided"; P→"declared to contain milk" + bound. | A beside N: N shows the **marker**, not blank → reads "unknown," not "clean." P beside A: P carries the "not a complete list" bound → not "no nuts." | **≥today** (warning on card + detail). | **NO** — no card is ever blank on allergens when the unit is on (guardrail #7); every dish shows warning/marker/partial-with-bound. | **NO** — marker is "unknown," never "free." |
| **Detail modal** (always-on, all coverage levels) | A→"declared to contain nuts" + bound; P→"declared to contain milk" + "not a complete list" bound; **N→"allergen info not provided"** (the `:1078` floor fix; never blank). | Single-dish surface (no on-screen pair). Each state renders an **explicit truthful element**; none blank. Even across navigation, each dish is self-truthful. | **≥today** (always, + floor + bound; today rendered nothing on empty). | **NO** — no blank state exists to read clean; N is explicit "unknown." | **NO** — explicit unknown, never "free." |
| **Comparison row** (deferred `…_COMPARISON`; spec) | **Every cell renders its state explicitly** (R2-H3) — A→warning, N→marker, P→partial — each with the reliance bound (R3-H3). Never blank/"—". | **The highest-stakes PAIR surface, explicitly walked:** A ("contains nuts") beside N → N shows "allergen info not provided", **not blank** → no "nut-free" read. P ("milk") beside A → P carries "not a complete list" bound → "milk only ⇒ no nuts" forbidden. | **≥today** (no comparison today; warning always rendered). | **NO** — every cell rendered + bound; no blank, no exhaustiveness implication. | **NO**. |
| **Deferred allergen filter** (`…_ALLERGEN_FILTER`, STOP-1) | Not in v1. Positive-only when re-enabled; no-data dishes marked, never folded into a clean remainder. | N/A (deferred). | ≥today (warning unaffected). | **NO** (positive-only; marked no-data). | **NO**. |

**The partially-authored-menu contrast scenario, run explicitly (the R4-H1 break case).** 40-dish menu, 5
× A (`['nuts']`), 35 × N. **v1 card:** all 40 cards show **no** allergen element → the 5 A-dishes and the 35
N-dishes are **visually identical at the card level** → the customer can infer nothing about allergens from
any card → **no "nut-free by contrast" read** (the R4-H1 break is closed). To learn allergen state the
customer opens the detail, which renders "declared to contain nuts" (A) or "allergen info not provided" (N)
— **strictly better than today** (today N rendered *nothing* in the detail; now it renders the explicit
"unknown"). **Comparison-row pair (when shipped):** A-cell "declared to contain nuts" vs N-cell "allergen
info not provided" — no clean-read. **Every cell PASSES (a)/(b)/(c). The fix is done.**

## R4-M1 — the reliance-bound omnipresence dissolves under DETAIL-FLOOR-ONLY

R4-M1 is correct *against the round-3 model*: a "not a complete list" caveat on **100% of card cells** is the
same 100%-coverage omnipresence the design gates the no-data marker to avoid — the design treated identical
omnipresence as *noise* (marker → gated) in one place and *mandated signal* (caveat → always) in another.
**DETAIL-FLOOR-ONLY dissolves the inconsistency:** in v1 the card carries **no allergen element**, so there
is **no omnipresent card caveat.** The reliance bound rides only where allergen content actually renders —
the **detail** (v1), and later the **card unit** + **comparison cells**. On those surfaces the bound is not
wallpaper: it accompanies a surface that is *asserting allergen content*, where bounding the reliance is
exactly the point (a "declared to contain" claim must carry "declared, not complete"). The bound is never
spread onto a card that asserts nothing. Recorded so a future reader does not re-introduce a universal card
caveat (which would BE the no-data marker by another name — the contradiction R4-M1/R4-H1 flagged).

## Corrections to prior (round-3) dispositions — stated explicitly

- **R3-H1 card-elevation of the warning OVERSTATED the fix and created the contrast door.** Round 3 said the
  warning "renders at-a-glance on card + detail + comparison ALWAYS" (`proposal.md:30` r3) while gating the
  no-data marker — an **asymmetric card surface**. **Corrected:** in v1 the card carries **no** allergen
  element (warning detail-only = today); the card warning rides only with the card unit, which always
  carries the marker. STOP-3 is honored by the **detail** floor, not by card elevation.
- **The round-3 door-check assertion "no clean-read sits behind any door" (`resolution.md:620`) was FALSE.**
  It evaluated a no-data card **in isolation** and missed the **contrast** with sibling warning cards.
  **Retracted.** Standing method: evaluate allergen surfaces in **pairs**, never in isolation.
- **The two-denominator residue for the no-data marker (R4-H1) is closed.** Round 3 left the no-data marker
  gated by the **descriptive** denominator (a category error — an allergen signal keyed to an unrelated
  quantity). **Corrected:** the card allergen UNIT (warning + marker) is keyed to the **allergen**
  denominator and gated as a unit (later feature); the descriptive gate touches **only** descriptive chips
  and no longer governs any allergen element.

## Red lines — all intact after RESOLVE round 4

- **Allergen: never-suppress-a-warning — across ALL mechanisms, and the warning at ≥ today's visibility.**
  The warning is never suppressed by modifier (R2-C1) OR coverage (R3-H1) OR curation cap (R2-H3); its
  unconditional floor is the **detail modal** (always, = today, improved). It is not "hidden" by being
  detail-only in v1 — that is its current location, never below it (STOP-3).
- **Data-absence never reads as a clean allergen state on ANY surface — including by CONTRAST (R4-H1).** No
  card asserts allergen state in v1 (uniform silence = today); the detail floor renders explicit "unknown";
  every comparison cell renders explicitly; the later card unit renders warning + marker together (no blank).
  **The card allergen surface is all-or-nothing (guardrail #7): warning and marker are one unit, never
  split.** Partial declarations carry the "not a complete list" bound (R3-H3) so a partial never reads as
  exhaustive.
- No platform-asserted regulated claims (allowlist reviewed-clear-of-register; regulated subset OFF; #2/#6).
- Honesty-anchor = a real number required (necessary-not-sufficient for regulated).
- Owner authority for declarations (regulated opt-in; AllergenEditor wiring still the L3 prereq).
- Facts-not-verdict (no global winner; arrows only on price + prep-time).
- Never-fabricate (no auto-vegan; no synthesized prep-time range; no fabricated allergen completeness or
  absence — including no contrast-derived absence).
- i18n (taste + allergen al ship; descriptive allowlist EN+AL-reviewed per label; regulated-term al gated).

## Updated guardrail set (red→green; SPECIFIED, NOT BUILT — design-time)

SHIP flag bound to **#1, #4-positive, #5, #6, #7**; **#2** gates only the regulated flag.

| Guardrail | Concrete artifact (round-4 form) | Gates |
|---|---|---|
| **#1 No-hide-allergen** | grep/lint: no allergen-axis filter predicate wired into the rendered set (filter absent from SHIP). Positive-only = recorded re-enable design rule, not this grep (R3-M1). | SHIP (ratchet) |
| **#4-positive Warning-never-suppressed** | unit: a dish with `bom[].allergens ⊇ {X}` MUST render "declared to contain X" in the **detail modal** regardless of modifier state AND coverage (the unconditional floor); AND when the card allergen unit is ON, the card warning MUST render with the marker (never alone — see #7); AND a modifiable dish emits no descriptive/diet/regulated **reassuring** chip. | SHIP |
| **#5 No-clean-from-empty + detail floor** | unit: (a) detail modal with empty/absent `bom[].allergens` renders "allergen info not provided" **unconditionally** (regardless of coverage / `allergens_confirmed`) — the `MenuPage.tsx:1078` fix; (b) detail with non-empty renders "declared to contain X" unconditionally; (c) **no surface renders a clean state from empty data.** | SHIP |
| **#6 Descriptive-allowlist** | unit: rendered descriptive vocabulary (EN+AL) **⊆ the closed reviewed allowlist** (deterministic subset check; v1 empty → descriptive band renders nothing). | SHIP (descriptive band) |
| **#7 Card-allergen ALL-OR-NOTHING (NEW — anti-contrast)** | unit: a build where the **card** renders an allergen **warning** chip while a no-data **card** renders **no marker** is **RED** — warning and no-data marker on the card are one inseparable unit (both render, or neither does, at every coverage level). v1 (card unit OFF → both absent) is trivially green; the test stays a **permanent ratchet** so the contrast door can never be (re)introduced by elevating the warning to the card without the marker. | SHIP (ratchet) |
| **#2 Regulated-subset-off** | CI: regulated label not emittable without owner-authority flag + populated verified-anchor entry; red-on-disk until anchor table exists. | regulated flag |
| **#3 Body-effect denylist** | kept (existing red line). | all |

## Converged? + remaining NEEDS-HUMAN

**CONVERGED.** No open CRITICAL/HIGH. R4-H1 fixed (DETAIL-FLOOR-ONLY + all-or-nothing invariant + #7);
R4-M1 dissolved. STOP-1/STOP-2/STOP-3 all remain discharged. The fix is the first this design that **closes
without relocating** — it welds the card warning/marker into one unit, so the asymmetry the contrast door
requires is structurally unbuildable (proven in §"Why this CANNOT create a 5th door" + the paired
door-check).

**Remaining NEEDS-HUMAN (gates, not ship blockers):**
1. **Allergen-coverage threshold** for the **LATER card allergen unit** (NEW — warning + marker turn on
   together above it; the unit's own denominator, distinct from descriptive; dormant in v1 since the card
   unit is OFF). Owner: product.
2. Descriptive allowlist contents + AL review (v1 may be empty). Owner: product + Counsel/i18n.
3. Per-tenant **descriptive**-coverage threshold (descriptive chips only; dormant while allowlist empty).
   Owner: product.
4. Verified per-market legal anchors (EU 1924/2006 + AL) before the regulated L2 subset. Owner: Counsel/legal.
5. Per-market linguistic/legal review of regulated-term al renderings before regulated flip-on. Owner:
   Counsel/legal + i18n.
6. Coverage threshold + positive-only design sign-off before any allergen filter re-enables (STOP-1). Owner:
   Counsel/legal + product.

**Design-time note:** all guardrails remain **specified, not built**. Building #1, #4-positive, #5 (incl.
the `MenuPage.tsx:1078` detail-floor fix), #6, **#7 (card-allergen all-or-nothing)** red→green is the
implementer's gate to flip `MENU_CHARACTERISTICS_ENABLED`; #2 gates the regulated flag.

---
---

# RESOLVE round 5 — convergence-closing

**Seat:** Architect, RESOLVE r5. Design-time only — NO production code.
**Inputs:** `breaker-findings.md` §RE-ATTACK round 5 (**R5-H1** HIGH, two instances (i)+(ii); **R5-L1** LOW) ·
Counsel round-5 non-blocking notes (a)+(b), recorded below, not gated. No new ETHICAL-STOP filed
(STOP-1/STOP-2/STOP-3 remain discharged).
**Re-grounded @ HEAD (verified this round, matches the Breaker grounding):** `MenuPage.tsx:1078` —
`{(bomToNutrition(detailProduct).allergens.length > 0) && (…)}`: allergens render **only in the detail
modal, only on `length>0`, nothing on empty, no completeness caveat**, and **no card carries any allergen
chip**. Round-4's baseline claim holds. Rule module / guardrails / marker / reliance-bound still unbuilt
(grep ~0) — design-time, as intended.

## Verdict

**CONVERGED.** Both seats confirm the **architecture has converged** — the contrast/relocation door that ran
rounds 1→2→3→4 is **structurally shut** (round 4: remove-not-relocate + the all-or-nothing weld), and the
Breaker explicitly declines to invent a relocation finding. The single open item is **not architecture**: it
is a **guardrail-binding completeness gap** + a **scope ambiguity** — a pure guardrail/spec edit. This round
closes it by **tightening the gate** (promoting two already-spec-mandated allergen protections from prose into
the bound guardrail set) and by **recording the v1 scope unambiguously**. Zero architecture change, zero new
runtime.

**The two moves:**
1. **R5-L1 / v1 scope, decided first (it determines whether R5-H1(ii) is a v1 blocker):**
   **v1 SHIP = L1 taste (card) + L2 descriptive (closed allowlist, EMPTY in v1 → dormant) + allergen
   DETAIL-FLOOR (detail modal only; NO card allergen element) ONLY.** **COMPARISON and FILTER are DEFERRED to
   their own flags (`…_COMPARISON`, `…_FILTER`) and are NOT in v1.** Consequence: R5-H1(ii) (comparison) is
   **NOT a v1 ship blocker** — it gates the deferred `…_COMPARISON` flip, not SHIP.
2. **R5-H1(i) promoted to a SHIP-gating guardrail (#5d); R5-H1(ii) bound to the COMPARISON flip (#8).** Both
   were spec-mandated (R3-H3 reliance bound; R2-H3 comparison explicit-marker) but sat **outside** the bound
   set, so the design's own flip contract green-lit anaphylaxis-grade builds. They are now **bound artifacts**.

## STOP / blocking status

- **STOP-1 / STOP-2 / STOP-3 — all remain DISCHARGED** (carried; nothing in this round reopens them).
- **No new STOP opened.** R5-H1 is a binding-completeness HIGH, not a red-line crossing; the spec already
  mandated both protections — they were merely not bound. Binding them removes ship-optionality.

---

## Disposition table — RE-ATTACK / Counsel round 5

| # | Sev | Finding (one line) | Disposition | What changes |
|---|-----|--------------------|-------------|--------------|
| **R5-L1** | LOW | v1 scope incoherence — is COMPARISON in v1 (brief build-order) or deferred (`…_COMPARISON`, door-check row 4)? | **RESOLVED (decided + recorded)** | **v1 SHIP = L1 taste (card) + L2 descriptive (allowlist, empty → dormant) + allergen DETAIL-FLOOR only.** COMPARISON and FILTER are **DEFERRED behind `…_COMPARISON` / `…_FILTER`, NOT in v1.** Recorded unambiguously in proposal §0 + ADR rollout so no reader pulls comparison into v1. The "characteristics layer → comparison → filter" brief build-**order** is preserved as the *sequence*; only the layer ships in v1. |
| **R5-H1(i)** | HIGH | The R3-H3 partial-as-exhaustive reliance bound is **unbound** — a non-empty-but-incomplete declaration (`['milk']` on a dish that also has nuts) passes #5(b) as "declared to contain milk" with no completeness caveat; the bound is prose, not in the ship-gating set → flip contract green-lights "milk only ⇒ no nuts" | **FIXED (promote to SHIP-gating #5d)** | **#5(d) added to the SHIP-gating set:** every rendered allergen state on an authoritative surface (the v1 **detail floor**), whether empty (`"allergen info not provided"`) OR non-empty (`"declared to contain …"`), MUST carry the persistent **"not a complete allergen list — confirm with the venue"** reliance bound. **A build where a non-empty allergen declaration renders WITHOUT that caveat is RED.** Concrete red→green assertion (below). SHIP-gating set is now {#1, #4-positive, #5, **#5d**, #6, #7}. |
| **R5-H1(ii)** | HIGH (but comparison DEFERRED → not a v1 blocker) | The R2-H3 comparison no-data cell explicit-marker is **unbound by a POSITIVE assertion** — only #5(c) (a NEGATIVE) names comparison, and a literal `"—"` cell passes #5(c) yet reads "nut-free by contrast" beside "declared to contain nuts" | **FIXED (bind to COMPARISON flip, #8)** | Since comparison is **DEFERRED** (R5-L1), this is **NOT a v1 ship blocker** but MUST be a guardrail bound to the `…_COMPARISON` flip. **#8 (gates COMPARISON, not SHIP):** the comparison allergen surface is **all-or-nothing / explicit-both** — every dish's allergen cell renders its **explicit** state (warning / `"allergen info not provided"` / partial+bound), and **a build where any comparison allergen cell is blank/`"—"` is RED.** #7's all-or-nothing logic **extends to comparison cells.** Recorded as bound to the comparison flip so comparison cannot ship without it. |
| **Counsel (a)** non-blocking | — | The later card-allergen-unit coverage threshold (NEEDS-HUMAN) must be decided against the ALLERGEN denominator alone AND use an ABSOLUTE authored-dish floor (a ratio alone flips the unit on for a tenant with 2 authored dishes at low N) | **RECORDED (not gated)** | The `…_CARD_ALLERGEN` threshold spec is amended: gate keyed on the **allergen denominator alone** (already so since round 4) **AND** an **absolute authored-dish floor** (e.g. ≥ K dishes with authored allergen data), not a ratio-only test — so low-N tenants cannot trip the unit on. Dormant in v1 (card unit OFF); a NEEDS-HUMAN product value, not a ship gate. |
| **Counsel (b)** non-blocking | — | Don't oversell v1 as a safety feature — the at-a-glance safety upside lives behind the deferred card unit | **RECORDED (framing)** | v1's allergen value is honestly framed internally as **"today's detail, made honest"** (the detail floor adds the explicit "info not provided" + the reliance bound where today there was nothing; the warning stays at today's visibility). The at-a-glance menu-scan safety upside is a property of the **later** `…_CARD_ALLERGEN` unit, not v1. Recorded so v1 is not communicated as an at-a-glance allergen-safety feature. |

---

## R5-H1(i) — the SHIP-gating reliance-bound guardrail (#5d), full justification

**The gap (accepted as stated).** R3-H3 mandated a persistent, surface-attached "not a complete allergen
list — confirm with the venue" bound so a **partial** declaration never reads as exhaustive. Round 4 recorded
it as a **carried principle** (`resolution.md:652-661`, ADR §"Carried reliance principle") but the round-4
ship-gating set {#1, #4-positive, #5, #6, #7} contains **no assertion that the bound is rendered**. #5(b)
asserts only that a non-empty list renders "declared to contain X" — provenance framing, which softens
*who said it*, not *whether it is complete*. So a build can render "declared to contain milk" on a
nuts-and-milk dish, with no incompleteness caveat, **pass all five gates, and flip** — the exact
anaphylaxis-grade exhaustive-read R3-H3 was written to forbid. This is precisely the R2-H4 failure class
("guardrails as artifacts, flips bound to them — safety is not left to implementer care"): a safety
requirement outside the bound set is, by the design's own contract, ship-optional.

**The fix — #5(d), a concrete red→green artifact, bound to SHIP:**

> **#5(d) Reliance-bound-rendered (SHIP).** On every authoritative allergen surface in v1 — the **detail
> floor** — the rendered allergen state carries the persistent reliance bound, in BOTH branches:
> - empty `bom[].allergens` → renders `"allergen info not provided — ask the restaurant"` (the #5(a) text)
>   AND the surface carries the "not a complete list / confirm with venue" framing;
> - non-empty `bom[].allergens` → renders `"declared to contain {X}"` (the #5(b) text) **AND** the persistent
>   `"not a complete allergen list — confirm with the venue"` caveat **attached to the same surface**.
>
> **Red→green assertion:** a unit test that renders the detail modal for a product with a **non-empty**
> `bom[].allergens` and asserts the presence of the incompleteness caveat string (i18n key, EN **and** AL);
> a build where a non-empty allergen declaration renders **without** the caveat is **RED**. (Symmetric check on
> the empty branch already covered by #5(a).) The bound is surface-attached, not low-salience ambient prose
> (the round-2 Q1 ambient line was overridden by the high-salience visual — R3-H3 §; the test asserts it rides
> *on* the allergen surface element).

**Why this is the load-bearing close.** The bound is the **only** protection a presence-only system has
against partial-as-exhaustive (the system has no review-completeness datum and cannot acquire one without owner
data it does not have — R3-H3). Promoting it from prose to a bound artifact converts "the implementer should
render the caveat" into "the flip cannot occur until a test proves the caveat renders." This closes the gap at
its actual location (the flip contract), not adjacent to it.

## R5-H1(ii) — the comparison explicit-both guardrail (#8), bound to `…_COMPARISON`

**The gap (accepted as stated).** §4.2 (`proposal.md:107`) requires every comparison allergen cell render its
explicit state and the no-data cell render the marker "NEVER blank/'—'". But the only ship-gating clause
naming comparison is **#5(c) — a NEGATIVE** ("no surface renders a clean state from empty"). A literal #5(c)
(no cell contains "free"/"none"/"safe") **passes a `"—"` cell** — a dash contains no clean word — yet `"—"`
beside "declared to contain nuts" reads **nut-free by contrast** at the highest-stakes pair surface. The
detail floor got a precise POSITIVE artifact (#5a); the comparison no-data cell did not. #7 is card-only
(welds the *card* warning+marker), so nothing in the round-4 set catches the comparison cell.

**The fix — #8, a concrete red→green artifact, bound to the COMPARISON flip (NOT SHIP):**

> **#8 Comparison-allergen explicit-both / all-or-nothing (gates `…_COMPARISON`).** Every dish's comparison
> allergen cell renders its **explicit** state — warning (`"declared to contain X"`) / `"allergen info not
> provided"` / partial-list + the #5(d) reliance bound — and **never blank/`"—"`.** #7's all-or-nothing logic
> **extends to comparison cells**: across the two compared dishes, no cell is empty while another asserts a
> state.
>
> **Red→green assertion:** a unit test that renders the comparison surface for a PAIR (A = `['nuts']`, N =
> no-data) and asserts the N-cell renders the explicit `"allergen info not provided"` text (positive
> assertion, **not** the #5(c) negative) and that A's cell carries the #5(d) reliance bound; a build where any
> comparison allergen cell is blank/`"—"` is **RED.**

**Why this is bound to COMPARISON, not SHIP.** Per R5-L1, comparison is **deferred** — it is not on the wire in
v1. Binding #8 to SHIP would gate v1 on a surface v1 does not render (over-gating). Binding it to
`…_COMPARISON` means **comparison cannot flip without #8 green** — the protection travels with the surface it
protects. This is the same binding discipline R2-H4 demands, applied per-surface.

## Counsel non-blocking notes — recorded dispositions

- **(a) Card-allergen-unit threshold — allergen denominator + ABSOLUTE floor (RECORDED).** The later
  `…_CARD_ALLERGEN` threshold is keyed on the **allergen denominator alone** (a card allergen signal must
  never be gated by an unrelated quantity — the round-4 correction) **AND** must carry an **absolute
  authored-dish floor** (≥ K dishes with authored allergen data), not a ratio-only test. Rationale (Counsel):
  at low N a ratio flips the unit on for a tenant with 2 authored dishes (2/2 = 100% coverage, but two
  warnings is not an at-a-glance safety surface). The floor prevents the unit activating on statistically
  empty menus. Dormant in v1 (card unit OFF). NEEDS-HUMAN: the threshold values (K + ratio). Owner: product.
- **(b) v1 framing — "today's detail, made honest" (RECORDED).** v1's allergen value is **not** an
  at-a-glance safety feature: the card carries no allergen element (= today). v1 improves the **detail** —
  the explicit "info not provided" where today there was nothing, plus the reliance bound — and keeps the
  warning at today's visibility. The at-a-glance menu-scan safety upside is a property of the **deferred**
  `…_CARD_ALLERGEN` unit. Recorded so v1 is communicated internally as "today's detail, made honest," never
  oversold as an allergen-safety feature.

## DOOR-CHECK — does a binding edit open a new door? (the brief's explicit demand)

**Stated explicitly: NO. A binding edit can only TIGHTEN the gate; it cannot open a door.** Exhaustive
reasoning:

1. **A guardrail-binding edit adds a *failing condition* to the flip; it changes no rendered surface, no data
   path, no derivation.** #5(d) and #8 each say "a build that omits protection P is RED." Adding a RED
   condition strictly **shrinks** the set of builds that may flip — it can forbid a previously-allowed
   (unsafe) build, never permit a previously-forbidden one. The space of shippable builds after the edit is a
   **subset** of the space before. A new door requires *enlarging* the shippable set (permitting an unsafe
   render that was previously caught); a tightening edit does the opposite. Therefore no door.
2. **Neither edit relocates a safety element** (the rounds-1–4 door mechanism). #5(d) asserts the bound
   renders *where R3-H3 already placed it* (the allergen surface). #8 asserts the comparison cell renders
   *where §4.2 already placed it*. No element moves to a new asymmetric position — the edits assert that the
   already-specified position is actually populated. No moved-half/stationary-counterpart seam is created.
3. **The v1 scope decision (R5-L1) removes a surface from v1; it does not add one.** Deferring comparison
   returns v1's comparison surface to "absent" — strictly fewer surfaces to contrast, never more. (And #8
   still guards comparison whenever it ships.)
4. **No interaction with the discharged STOPs.** #5(d) strengthens the never-suppress / never-read-as-clean
   line (STOP-2/STOP-3) by making the partial-as-exhaustive caveat mandatory-to-flip. #8 strengthens the
   never-read-as-clean-by-contrast line (R4-H1/STOP-3) on the comparison surface. Both push *toward* the red
   lines, never across.

**Conclusion:** the round-5 edits are monotonic gate-tightenings. They cannot open a 5th door because a
binding edit is not a relocation and not a permission — it is a forbiddance. The brief's check passes.

## Final guardrail table (FULL — id · what it asserts red→green · which flag it gates)

SHIP flip bound to **exactly** {#1, #4-positive, #5, **#5d**, #6, #7}. Deferred surfaces gate their own flags:
**#8** → COMPARISON, **#2** → regulated, the no-hide/positive-only design → the allergen-filter re-enable.

| # | Asserts (red→green) | Gates which flag |
|---|---------------------|------------------|
| **#1 No-hide-allergen** | grep/lint: no allergen-axis filter predicate is wired into the rendered set (the allergen filter is ABSENT from SHIP). "Positive-only" is a recorded re-enable *design rule*, NOT this grep (R3-M1). | **SHIP** (permanent ratchet) |
| **#4-positive Warning-never-suppressed** | unit: a dish with `bom[].allergens ⊇ {X}` MUST render "declared to contain X" in the **detail modal regardless of modifier state AND coverage** (the unconditional floor); when the card allergen unit is ON, the card warning renders **with** the marker (never alone — see #7); a modifiable dish emits no descriptive/diet/regulated **reassuring** chip. | **SHIP** |
| **#5 No-clean-from-empty + detail floor** | unit: (a) detail with empty/absent `bom[].allergens` renders "allergen info not provided" **unconditionally** (the `MenuPage.tsx:1078` fix); (b) detail with non-empty renders "declared to contain X" unconditionally; (c) no surface renders a *clean* state from empty data (negative). | **SHIP** |
| **#5d Reliance-bound-rendered** (NEW — R5-H1(i)) | unit: on the v1 **detail floor**, BOTH branches carry the persistent "not a complete allergen list — confirm with the venue" bound (EN+AL) — empty → bound beside "info not provided"; **non-empty → bound beside "declared to contain X"**. A build where a non-empty allergen declaration renders **without** the caveat is **RED**. (Positive assertion; surface-attached, not ambient.) | **SHIP** |
| **#6 Descriptive-allowlist** | unit: rendered descriptive vocabulary (EN+AL) ⊆ the closed human-reviewed allowlist (deterministic subset check; v1 allowlist empty → descriptive band renders nothing). | **SHIP** (descriptive band) |
| **#7 Card-allergen ALL-OR-NOTHING** (anti-contrast, R4-H1) | unit: a build where the **card** renders an allergen **warning** chip while a no-data **card** renders **no marker** is **RED** — warning + no-data marker on the card are one inseparable unit (both render or neither, at every coverage level). v1 (card unit OFF) trivially green; permanent ratchet. | **SHIP** (ratchet) |
| **#8 Comparison-allergen explicit-both** (NEW — R5-H1(ii)) | unit: every comparison allergen cell renders its **explicit** state (warning / "allergen info not provided" / partial+#5d bound) and **never blank/`"—"`**; #7's all-or-nothing extends to comparison cells (no cell empty while another asserts). A blank/`"—"` cell is **RED** (positive assertion, not the #5c negative). | **`…_COMPARISON`** |
| **#2 Regulated-subset-off** | CI: a regulated label is not emittable without an owner-authority flag + a populated verified-anchor entry; red-on-disk until the anchor table exists in-tree. | **regulated flag** |
| **#3 Body-effect denylist** | regulated body-effect phrasing ("keeps you full", "healthy", "good for you") → CI red. | all |
| **(re-enable design rule) Positive-only allergen filter** | NOT a grep — a recorded human-reviewed condition of re-enable: contains-X never hide→safe + markers + non-dismissible coverage disclosure + honest denominator + human-set coverage threshold. | **`…_ALLERGEN_FILTER`** re-enable (NEEDS-HUMAN) |

**SHIP-flip binding confirmation:** `MENU_CHARACTERISTICS_ENABLED` (v1 = taste + descriptive band + allergen
detail-floor) **must not flip on** until **#1, #4-positive, #5, #5d, #6, #7** are authored and proven
red→green. **#8** gates `…_COMPARISON`; **#2** gates the regulated flag; the positive-only design rule gates
the allergen-filter re-enable. Each deferred surface's allergen guard travels with its own flag — no deferred
surface can ship without its guard, and v1 is not over-gated by a surface it does not render.

## Corrections to prior (round-4) dispositions — stated explicitly

- **The round-4 ship-gating set {#1, #4-positive, #5, #6, #7} was INCOMPLETE.** It omitted the R3-H3 reliance
  bound (recorded as a carried principle but never bound) — so the flip contract green-lit a
  partial-as-exhaustive build. **Corrected:** #5d added to the SHIP set.
- **The round-4 set named comparison only via a NEGATIVE (#5c), which a `"—"` cell passes.** **Corrected:** #8
  (a POSITIVE explicit-both assertion) added, bound to `…_COMPARISON`.
- **v1 scope was ambiguous** (brief build-order placed comparison in v1; door-check row 4 deferred it).
  **Corrected:** v1 = taste + descriptive(dormant) + allergen detail-floor ONLY; comparison + filter deferred
  to their own flags — recorded in proposal §0 + ADR rollout.

## Red lines — all intact after RESOLVE round 5

- **Allergen: never-suppress-a-warning (all mechanisms) AND data-absence/partial-data never reads as a clean
  or exhaustive allergen state on any surface — now BOUND, not merely specified.** The warning floor is the
  detail modal (always, ≥ today — STOP-3); empty → explicit "info not provided" (#5); **non-empty → carries
  the persistent incompleteness bound (#5d, SHIP-gating)** so a partial never reads as exhaustive; the card is
  all-or-nothing (#7); the comparison surface is explicit-both (#8, gates `…_COMPARISON`). No protection sits
  outside its surface's flip contract.
- No platform-asserted regulated claims (allowlist reviewed-clear-of-register; regulated subset OFF; #2/#6).
- Honesty-anchor = a real number required (necessary-not-sufficient for regulated).
- Owner authority for declarations (regulated opt-in; AllergenEditor wiring still the L3 prereq).
- Facts-not-verdict (no global winner; arrows only on price + prep-time — and comparison is deferred from v1).
- Never-fabricate (no auto-vegan; no synthesized prep-time range; no fabricated allergen completeness/absence
  — including no contrast-derived absence and no partial-as-exhaustive read).
- i18n (taste + allergen al ship; the reliance-bound caveat asserted EN+AL by #5d; descriptive allowlist
  EN+AL-reviewed per label; regulated-term al review still gated).

## Converged? + final NEEDS-HUMAN list

**CONVERGED.** No open CRITICAL/HIGH after this round: R5-H1(i) closed (#5d, SHIP-gating), R5-H1(ii) closed
(#8, bound to `…_COMPARISON`), R5-L1 resolved (v1 scope recorded), Counsel (a)/(b) recorded as non-gating
notes. The architecture was already converged; this round closed the binding-completeness gap by tightening
the gate (door-check: a binding edit cannot open a door — proven above). Both seats' "architecture has
converged" stands; the convergence gate (0 CRIT/HIGH) is now met.

**Final NEEDS-HUMAN list (gates on deferred/later surfaces, NOT v1 ship blockers):**
1. **Descriptive allowlist contents + AL register-review per label** (v1 allowlist empty → descriptive band
   dormant). Owner: product + Counsel/i18n.
2. **Per-tenant descriptive-coverage threshold** (one denominator, descriptive only; dormant while allowlist
   empty). Owner: product.
3. **Allergen-coverage threshold for the LATER card allergen unit** (`…_CARD_ALLERGEN`) — **allergen
   denominator alone + an ABSOLUTE authored-dish floor** (Counsel (a), not a ratio-only test); dormant in v1.
   Owner: product.
4. Verified per-market legal anchors (EU 1924/2006 + AL transposition) before the regulated L2 subset. Owner:
   Counsel/legal.
5. Per-market linguistic/legal review of regulated-term al renderings before regulated flip-on. Owner:
   Counsel/legal + i18n.
6. Coverage threshold + positive-only design sign-off before any allergen filter re-enables (STOP-1). Owner:
   Counsel/legal + product.

**Design-time note:** all guardrails remain **specified, not built**. Building **#1, #4-positive, #5, #5d, #6,
#7** red→green (incl. the `MenuPage.tsx:1078` detail-floor fix and the #5d non-empty-caveat assertion) is the
implementer's gate to flip `MENU_CHARACTERISTICS_ENABLED` (v1). **#8** is the gate for `…_COMPARISON`; **#2**
for the regulated flag; the positive-only design rule for the allergen-filter re-enable. This is the
converging round.

---
---

# RESOLVE — FULL BUILD round (ADR-0014, 2026-06-30)

**Seat:** Architect, RESOLVE FULL BUILD. Design-time only — NO production code.
**Inputs:** `breaker-findings.md` §"RE-ATTACK — FULL BUILD round" (**FB-C1, FB-C2** CRITICAL · **FB-H1, FB-H2**
HIGH · **FB-M1..M4** MED · **FB-L1** LOW) · `counsel-opinion.md` §"RE-EXAMINE round 3 — FULL BUILD"
(**FB-STOP-1, FB-STOP-2** [esp. sub-path b], **FB-STOP-3**, + the open question: perception not proven to
allergic users).
**Re-grounded @ HEAD (production code verified this round):**
- `MenuPage.tsx:193-194` — the menu allergen filter is **LIVE, no `VITE_*` gate**, predicate
  `bomToNutrition(p).allergens.includes(filterAllergen)` = **recipe (bom) only**, persisted to `localStorage`
  (`:145-166`). **FB-C1 holds.**
- `MenuPage.tsx:854` — the card `allergens` prop = `nutrition.allergens` (`bomToNutrition`, **recipe-only**),
  rendered **unconditionally** (no flag, no marker, no all-or-nothing). `:865` quick-add adds to cart with **no
  modal / no allergen surface** when a dish has no modifier groups. `:1118` — the **modal** uses
  `computeAllergenSurface(attributes, bomAllergens)` = **declared ∪ recipe**. Card and modal compute the same
  safety fact from **two different functions**. **FB-C2 holds.**
- `characteristics.ts:97-108` — `computeAllergenSurface` exists and returns the conservative
  `{ known: declared ∪ recipe, hasInfo }`; `compareDishes`/`selectDescriptiveLabels` take **no `char_hidden`
  param** (FB-M2 holds); `DESCRIPTIVE_ALLOWLIST`/`REGULATED_ANCHORS` empty.
- `read_public_menu` (mig `1790000000033:59-117`) emits each modifier as `{id,name,price_delta}` only — the
  sole path modifiers reach the client; no modifier endpoint exists. **FB-H1 holds.**

## Verdict

**NOT YET CONVERGED on entry — two CRITICALs are PRODUCTION bugs already shipped, not design flaws.** The
prior five rounds reasoned about *un-built* surfaces; this round grounds FB-C1/FB-C2 in **live storefront
code** that already violates the allergen red line. The architecture (pure edge derivation, views-over-one-
source, gated steps) is kept. Convergence requires a **STEP-0 SAFETY FIX** that lands **before, or independent
of, every characteristics flag** — it is the single most important output of this pass — plus honest
resolution of the Contract-B / hot-path contradiction (FB-H1) and the three Counsel revisions.

**The decisive move:** the allergen surface is **one function on every surface** (`computeAllergenSurface`),
the live recipe-only filter and the unguarded recipe-only card chip are **converged or removed**, and the
v1 card returns to **detail-floor-only** so no bare card warning manufactures the R4-H1 contrast. Contract B
(live modifier honesty) is **DEFERRED** — v1 ships **Contract A only** and the proposal stops claiming live
modifier honesty.

---

## Disposition table — FULL BUILD round

| # | Sev | Finding (one line) | Disposition | What changes |
|---|-----|--------------------|-------------|--------------|
| **FB-C1** | CRIT | Allergen filter is LIVE (unflagged), recipe-only → drops a declared-only allergen dish from "contains X"; contradicts "DEFERRED ENTIRELY / fixed by construction" | **FIXED (STEP-0 safety fix) + NEEDS-HUMAN (keep-vs-gate)** | The filter predicate is converged to `computeAllergenSurface(p.attributes, bomAllergens).known.includes(X)` (declared ∪ recipe) so a declared-only allergen dish can **never** be dropped from a "contains X" view. The proposal **stops claiming the filter is "deferred entirely"** — it is **live**. Because it is a *positive show-contains* view (not hide→safe) it does not cross STOP-1's commission line, but over a near-empty denominator it still needs the non-dismissible **coverage disclosure + honest denominator**, OR it is gated OFF behind `…_ALLERGEN_FILTER` (default off) pending the STOP-1 positive-only human sign-off. **Keep-with-disclosure vs gate-off is the recorded human decision** (it is a shipped surface; "defer" = remove). |
| **FB-C2** | CRIT | Card (recipe-only) vs modal (declared∪recipe) compute allergens from different functions; quick-add bypasses both → declared-allergen dish reads clean on the card / in cart | **FIXED (STEP-0 safety fix)** | Single source of truth: card prop, quick-add path, modal, filter, comparison ALL read `computeAllergenSurface`. The card's current **unguarded recipe-only chip (`:854`) is REMOVED from v1** and folded under the gated, all-or-nothing `…_CARD_ALLERGEN` unit (#7) — v1 = detail-floor-only, so no card renders a bare warning (no R4-H1 contrast, no wholesome-by-omission). Quick-add of a declared-allergen dish surfaces the warning once `…_CARD_ALLERGEN` ships (unit reads the unified surface); in v1 the truth lives at the detail floor. **Guardrail #12** fails on any allergen read off `bomToNutrition().allergens` or any bare card warning. |
| **FB-H1** | HIGH | Contract B needs modifier deltas in the client payload → only delivery is re-versioning the 🔴 hot-path `read_public_menu`; contradicts "+0 endpoints / hot-path untouched" + live modifier honesty | **DEFER-FLAG (Contract B) + delivery path named** | **Contract B is DEFERRED** behind `MODIFIER_NUTRITION_ENABLED`; **v1 ships Contract A only** (suppress reassurance on modifiable dishes, never the warning). The proposal **stops claiming live modifier honesty for v1**, and names Contract B's true cost honestly: it is **not** "additive columns" alone — the deltas must reach the client. **Delivery path (chosen): a LAZY per-product detail fetch on modal-open** (the modal already lazily fetches `detailMedia` on open, `MenuPage.tsx:136`) — NOT a `read_public_menu` re-version and NOT a menu-grid endpoint. Selection/recompute is client-state in the open modal anyway, so the deltas ride a per-dish read, the 🔴 hot-path and the grid "+0 endpoints" budget are **preserved**. Whether to extend the media-fetch endpoint or add a sibling is decided in the Contract B ADR pass. |
| **FB-H2** | HIGH | Decoupled descriptive-coverage and allergen-coverage gates → positive badges render while the allergen unit is dark → wholesome-by-omission card | **FIXED (presence coupling, #13)** | **Card-level coupling invariant:** a card **never** renders a reassuring descriptive badge unless the card's **allergen unit is in a determinate state on the same card** (warning OR no-data marker present). I.e. the descriptive band may not light a card whose allergen state is unknown-and-unshown. **Guardrail #13:** a build where a card shows a descriptive/reassuring badge while the card allergen unit (warning or no-data marker) is absent is **RED**. (v1: descriptive allowlist empty → band dormant → trivially green; the coupling bites when descriptive populates while `…_CARD_ALLERGEN` is dark.) |
| **FB-STOP-2(b)** | STOP (REVISE) | Card badge-stack-as-safety gestalt — a confident 2–3 descriptive badge stack beside the allergen unit in its no-data floor reads "good dish, nothing flagged = safe" | **REVISED (layout invariant + #14)** | **Layout invariant (recorded LAW):** the allergen unit (warning OR no-data floor) is NEVER subordinated to, crowded out by, truncated by, or pushed below the primary card read by the descriptive badge-stack. Spec: the allergen unit occupies a **fixed reserved slot rendered at or above the descriptive badge container in DOM/reading order**, at **≥ the visual prominence** (type weight/size, not muted) of any descriptive badge, and the 2–3 curation cap governs **only** descriptive badges and can never evict/shrink it. **Guardrail #14:** a DOM/render assertion that (a) the allergen-unit element precedes (or sits in its dedicated non-evictable slot relative to) the descriptive-badge container, (b) it is present whenever any descriptive badge is present (ties to #13), (c) its computed prominence class ≥ the descriptive badge's. Gates `…_CARD_ALLERGEN`. **Plus the launch-gate human perception validation** (the open question, below) before the flip. |
| **FB-STOP-3** | STOP (REVISE) | `char_hidden` vocabulary not provably disjoint from allergen / reassurance surface | **REVISED (#9 → permanent ratchet)** | `char_hidden`'s validated vocabulary = the **closed enumerated set `DESCRIPTIVE_ALLOWLIST ∪ TASTE_AXES`** (a fixed, in-code constant). Server-side **Zod `.strict` enum** rejects any element outside it at write. **Guardrail #9 (permanent ratchet):** a test asserting `CHAR_HIDDEN_VOCAB ⊆ (DESCRIPTIVE_ALLOWLIST ∪ TASTE_AXES)` **AND** `CHAR_HIDDEN_VOCAB ∩ ALLERGEN_TOKENS = ∅` (the 14 EU allergens are a disjoint namespace) **AND** `CHAR_HIDDEN_VOCAB ∩ REASSURANCE/FREE_FROM_TOKENS = ∅` (no absence/free-from vocabulary exists by construction) **AND** that the allergen path (`computeAllergenSurface`) takes **no `char_hidden` argument** (subtraction can never touch a warning). Any label whose *removal* could manufacture a reassuring absence is excluded because hidable labels are descriptive/taste **descriptions**, never warnings, and the allergen surface is a separate function `char_hidden` cannot reach. |
| **FB-STOP-1** | STOP (NEEDS-HUMAN) | Regulated subset flip without verified-from-text anchors + a named legal sign-off | **NEEDS-HUMAN (recorded; stays red-on-disk OFF)** | The regulated subset stays **red-on-disk OFF** (`REGULATED_ANCHORS` empty, guardrail #2). The flip is gated on a **named legal/Counsel signatory** recording anchors **transcribed verbatim from EU Reg (EC) 1924/2006 Annex (+ Albanian transposition)** — `citation` + `verifiedBy` per anchor — AND owner opt-in authority AND server-authoritative supplies (R-2). **No numbers are invented in this pass.** Owner: named Counsel/legal signatory + architect. |
| **FB-M1** | MED | "Modifiable" undecidable from data → Contract A suppresses ALL reassurance on any dish with any modifier group (even "extra napkins") | **ACCEPT-RISK (safe direction) + DEFER-FLAG (refinement)** | Suppression is the **safe direction** (over-suppression is a utility loss, not a safety loss; FB-M1 concedes). Latent in v1 (allowlist empty). The conservative collapse ("any modifier group → suppress reassurance") **stands** until an axis-impact signal exists. Refinement **DEFERRED to the Contract B / `MODIFIER_NUTRITION_ENABLED` track** (where modifier deltas land — a delta of 0 on every axis ⇒ non-impacting ⇒ no suppression). Owner: product/data. |
| **FB-M2** | MED | `char_hidden` not applied to compare/filter; ships raw in public payload | **FIXED (single-derivation subtraction) — folded into #9** | The subtraction happens in the **one shared derivation** every surface calls; `compareDishes`/`selectDescriptiveLabels` take and apply `char_hidden` so a hidden label cannot resurface in compare/filter. Shipping `char_hidden` raw in the `jsonb` payload is **accepted** — it is a descriptive/taste enum subset (#9), can only ever *hide* a non-safety label, and carries no absence semantics, so it is not a safety leak. Guardrail #9 extended: a test that a hidden label is absent from compare AND filter output. |
| **FB-M3** | MED | Sort-by-protein/energy buries "unknown" (no-bom ⇒ `protein:0`) in the numeric-0 band → no-bom dishes mis-ranked as low, not marked | **FIXED (tri-state no-data bucket, #15)** | A no-bom dish is **not** a 0-value dish. The sort/filter lens checks `bom` presence (not the `bomToNutrition` numeric 0) and places no-data dishes in an **explicit "no data" group**, never inline at the bottom of the numeric rank. **Guardrail #15:** a sort-by-macro lens over a no-bom dish renders it in the explicit "no data" group, never numerically ranked as 0. Bound to `…_FILTER`. |
| **FB-M4** | MED | Compare long-press collides with card tap-to-modal, iOS native long-press, scroll | **FIXED (drop long-press; affordance-only)** | The **visible "compare" affordance (R-8) is the sole entry** for v-compare; the **long-press gesture is removed** from the spec (it is unreliable on the card image and unneeded — the affordance is discoverable + accessible). If a power-path gesture is ever wanted it must first pass a gesture-collision test (scroll-hold / iOS context-menu / double-fire `onClick`). Bound to `…_COMPARISON`. |
| **FB-L1** | LOW | §5 migration idempotency asserted but unspecified; `down()` empty | **FIXED (spec)** | The §5 additive migration uses `ADD COLUMN IF NOT EXISTS` per column (idempotent), **re-asserts `ENABLE+FORCE` RLS** (belt-and-suspenders per red-line discipline), forward-only `down() = {}` (project convention). Recorded coupling: the columns are **INERT without the FB-H1 lazy delivery path** — the migration ships with the Contract B track, not standalone. |
| **Open question** (Counsel) | LAUNCH-GATE | Perception not proven to allergic users — honest data can still misread to a fast/hungry/trusting human | **NEEDS-HUMAN (launch-gate)** | Before `…_CARD_ALLERGEN` flips, a recorded **usability validation with actual allergy-affected users** of (a) the "allergen info not provided — ask the restaurant" floor copy and (b) the badge-stack-beside-allergen-unit gestalt. No guardrail closes this; it needs a human in a usability seat once before the allergen card ships. Owner: product/design. |

---

## STEP-0 SAFETY FIX — the unified allergen-surface contract (the load-bearing output)

**This lands FIRST, before — or independent of — every characteristics flag.** It is a standalone safety fix
for two allergen red-line violations already in production (FB-C1, FB-C2), gated by its own red→green
guardrail (#12). It does not wait on `MENU_CHARACTERISTICS_ENABLED` or any sub-flag.

**The contract (precise):**

1. **One function, every surface.** Every storefront allergen read — the menu **filter** predicate
   (`MenuPage.tsx:194`), the **card** allergen prop (`:854`), the **quick-add** cart path (`:865`), the
   **detail modal** (`:1118`, already compliant), and (when shipped) the **comparison** cells — derives the
   allergen set from the single pure function **`computeAllergenSurface(attributes, bomAllergens)`** returning
   the conservative **`{ known: declared ∪ recipe, hasInfo }`**. **`bomToNutrition(p).allergens` is NEVER read
   for a safety decision again** (it may still feed the *macro* nutrition display, which is not a safety read).
2. **Filter converged + honest.** The live predicate becomes
   `computeAllergenSurface(p.attributes, bomToNutrition(p).allergens).known.includes(filterAllergen)` so a
   **declared-only** allergen dish can never be dropped from a "contains X" view. The filter carries the
   non-dismissible **coverage disclosure + honest denominator**, OR is gated OFF behind `…_ALLERGEN_FILTER`
   (default off) — the recorded **keep-with-disclosure vs gate-off** human decision (FB-C1).
3. **No bare card warning in v1.** The card's unguarded recipe-only allergen chip (`:854`) is **removed** and
   folded under the gated all-or-nothing `…_CARD_ALLERGEN` unit (#7). **v1 = detail-floor-only**: the card
   renders no allergen element, so no visible card warning manufactures a "clean by contrast" read (R4-H1) and
   no positive badge stack reads "safe by omission" (FB-H2/#13). All v1 allergen truth lives at the **detail
   floor** with the **#5d reliance bound** ("declared to contain … / allergen info not provided — not a
   complete list, confirm with the venue", EN+AL).
4. **Reliance bound rides the unified surface.** Wherever the unified surface renders content (modal now; card
   unit + comparison later), the persistent "not a complete allergen list — confirm with the venue" caveat is
   attached (#5d / #8).

**Guardrail #12 (single-allergen-source — red→green, gates STEP-0 and SHIP, permanent ratchet):**
> A build is **RED** if (a) any allergen-bearing render path reads `bomToNutrition(...).allergens` (or any
> recipe-only allergen accessor) instead of `computeAllergenSurface`, OR (b) the **card** renders an allergen
> **warning** chip while a no-data card renders no marker (a bare card warning — extends #7 to the card prop),
> OR (c) the menu filter predicate references a recipe-only allergen accessor. Concrete: a grep/lint that the
> only allergen source in `MenuPage` storefront surfaces is `computeAllergenSurface`, plus a unit test that a
> **declared-only** allergen dish (`attributes.allergen_status='listed', declared_allergens=['milk']`, no bom)
> is **retained** by a `filterAllergen='milk'` "contains milk" view and surfaces the warning on every
> non-gated surface.

**Why STEP-0 is independent of the flags.** FB-C1/FB-C2 are live red-line crossings *today*; gating their fix
behind a feature flag would leave the production false-negative standing until launch. The convergence of the
*derivation function* is a pure refactor + a removal of an unguarded chip — zero new surface, instantly
revertible, and it makes every later flag (card unit, compare, filter) inherit the correct source for free.

---

## FB-H1 — the Contract-B / hot-path contradiction, resolved honestly

The proposal claimed both "live modifier honesty (Contract B)" and "+0 endpoints / `read_public_menu`
untouched." Grounded: `read_public_menu` emits modifiers as `{id,name,price_delta}` only, and it is the **sole
path** modifiers reach the client. Contract B's recompute needs the §5 `kcal/protein/fat/carbs/allergens`
deltas **in the client**, so naive Contract B forces a re-version of the 🔴 TOTAL-BLAST-RADIUS hot-path — the
exact fn whose blast radius was used to **reject Option B** in §3. The two claims are mutually exclusive.

**Resolution (chosen, justified):** **Contract B is DEFERRED;** v1 ships **Contract A only**. The proposal no
longer claims live modifier honesty for v1. Contract B's honest cost is named: not "additive columns" alone,
but **columns + a delivery path for the deltas**. The chosen delivery path is a **lazy per-product detail
fetch on modal-open** — the open detail modal already lazily fetches `detailMedia` on open
(`MenuPage.tsx:136`), so the modifier deltas ride that **same per-dish read**, NOT the menu-grid
`read_public_menu`. This is the boring/proven choice: the menu-grid hot path and its "+0 endpoints" budget are
**preserved** (the spine LAW holds), and the recompute is client-state in the open modal anyway, so a per-dish
fetch is sufficient — the deltas are needed only for the one dish a customer is customizing. Extending the
existing media endpoint vs a sibling endpoint is decided in the Contract B ADR pass. **Owner:** architect +
product/data.

---

## Counsel non-blocking — steel-man (regulated audit trail) recorded

Counsel's steel-man (recover Option B's kernel for the regulated subset): if the regulated subset ever ships,
emit a **server-side audit record of the regulated claim** (a sidecar log — what claim, which dish, what data,
which threshold version, when) so the FBO-owns-the-claim liability thesis is provable — **NOT** a hot-path
`read_public_menu` re-version (blast-radius objection preserved). Recorded as a **design note on the regulated
track** (with FB-STOP-1), not a v1 item.

---

## Updated guardrail set — FULL BUILD (additions/revisions only; carried set unchanged)

| # | Asserts (red→green) | Gates |
|---|---------------------|-------|
| **#9 (REVISED → ratchet)** `char_hidden` closed-vocab + cross-surface subtraction | `CHAR_HIDDEN_VOCAB ⊆ (DESCRIPTIVE_ALLOWLIST ∪ TASTE_AXES)` AND `∩ ALLERGEN_TOKENS = ∅` AND `∩ REASSURANCE/FREE_FROM = ∅`; Zod `.strict` rejects out-of-set; `computeAllergenSurface` takes no `char_hidden`; a hidden label is absent from card AND compare AND filter output. | hide feature (permanent ratchet) |
| **#12 (NEW)** Single-allergen-source + no-bare-card-warning | no allergen render path reads `bomToNutrition().allergens`; only `computeAllergenSurface`; no bare card warning without the all-or-nothing marker; declared-only dish retained by "contains X" + surfaced. | **STEP-0 + SHIP** (permanent ratchet) |
| **#13 (NEW)** Descriptive-badge ⇒ allergen-unit presence coupling | a card showing a descriptive/reassuring badge while the card allergen unit (warning or no-data marker) is absent is RED. | descriptive band + `…_CARD_ALLERGEN` |
| **#14 (NEW)** Card layout — allergen unit never subordinated | allergen-unit element precedes/equals the descriptive-badge container in DOM order, present whenever any descriptive badge is, prominence ≥ descriptive badge; curation cap governs descriptive badges only. | `…_CARD_ALLERGEN` |
| **#15 (NEW)** Sort-by-macro no-data bucket | a no-bom dish under a macro sort/filter lens renders in the explicit "no data" group, never numerically ranked as 0. | `…_FILTER` |

Carried unchanged: **#1** no-hide-allergen (SHIP ratchet) · **#2** regulated-subset-off (regulated flag) ·
**#3** body-effect denylist (all) · **#4-positive** warning-never-suppressed (SHIP) · **#5/#5d** no-clean-from-
empty + reliance bound (SHIP) · **#6** descriptive allowlist subset (SHIP) · **#7** card all-or-nothing
anti-contrast (SHIP ratchet) · **#8** comparison explicit-both (`…_COMPARISON`) · **#10** modifier-recompute
monotonicity (`MODIFIER_NUTRITION_ENABLED`) · **#11** compare arrows only price/prep (`…_COMPARISON`).

---

## Revised BUILD ORDER — STEP 0 is the safety floor

0. **STEP-0 SAFETY FIX (no flag — lands first/independent).** Converge every allergen surface onto
   `computeAllergenSurface`; converge the live filter (+ disclosure or gate-off); remove the unguarded
   recipe-only card chip (v1 = detail-floor-only). Gate: **#12** red→green. This fixes the two production
   red-line crossings (FB-C1/FB-C2) and underwrites every later step.
1. **L1 taste + L2 descriptive band** (`MENU_CHARACTERISTICS_ENABLED`) — allowlist empty in v1 → band dormant.
   Gate: **#1, #4-positive, #5, #5d, #6, #7, #12**. (Detail-floor + reliance bound; no card allergen element.)
2. **CARD allergen unit** (`…_CARD_ALLERGEN`) — all-or-nothing (warning+marker), coverage-gated (allergen
   denominator + absolute floor). Gate: **#7, #12, #13, #14** + the **launch-gate human perception
   validation**.
3. **COMPARE** (`…_COMPARISON`) — affordance-only entry (long-press removed). Gate: **#8, #11**.
4. **FILTER** (`…_FILTER`) — non-allergen lenses; no-data bucket. Gate: **#15** (+ #1 keeps the allergen filter
   positive/absent). Allergen filter re-enable = NEEDS-HUMAN positive-only sign-off.
5. **Regulated subset** — NEEDS-HUMAN: named legal signatory + verified-from-text anchors + owner authority +
   server supplies + AL legal review. Gate: **#2, #3, #6**. (Steel-man: server-side claim audit log.)
6. **Contract B / modifier deltas** (`MODIFIER_NUTRITION_ENABLED`) — §5 RED-LINE additive migration
   (idempotent, FORCE-RLS) **+ the lazy per-product delivery path** (NOT the hot path). Gate: **#10**.

---

## Red lines — all intact after the FULL BUILD round

- **Allergen presence-only / absence-never / single-source** — one function (`computeAllergenSurface`) on every
  surface (#12); recipe-only reads forbidden; no bare card warning; v1 detail-floor-only (no contrast, no
  wholesome-by-omission); reliance bound always attached (#5d/#8); warning never suppressed (modifier/coverage/
  cap/curation).
- **No platform-asserted regulated claims** — red-on-disk OFF, named legal signatory required (FB-STOP-1);
  steel-man audit log recorded for if-it-ships.
- **Hot-path `read_public_menu` untouched** — Contract B deferred; deltas ride a lazy per-product fetch, never
  the grid (FB-H1).
- **Hide-not-fabricate asymmetry** — `char_hidden` is a closed descriptive/taste enum, provably disjoint from
  allergen/reassurance, subtraction-only, applied in the one shared derivation (#9).
- **Facts-not-verdict** — no global winner; arrows only on price/prep (#11); no-data is a bucket, never a 0
  (#15).
- **Honesty-anchor; never-fabricate; i18n hand-translated** — unchanged.

## Converged? + disposition counts

**CONVERGED — to v1 = STEP-0 safety fix + taste + dormant descriptive band + allergen detail-floor.** Both
CRITICALs are resolved by STEP-0 (a standalone safety fix + #12, not gated behind any feature flag); FB-H1
resolved by deferring Contract B + naming a non-hot-path delivery; FB-H2/STOP-2(b) by the card coupling +
layout invariants (#13/#14); STOP-3 by the #9 ratchet; the rest fixed/accepted/deferred per the table.

**Disposition counts (Breaker, 9 findings):** **FIXED 7** (FB-C1, FB-C2, FB-H2, FB-M2, FB-M3, FB-M4, FB-L1) ·
**ACCEPT-RISK 1** (FB-M1, safe direction + deferred refinement) · **DEFER-FLAG 1** (FB-H1, Contract B).
**Counsel (3 STOPs + 1 open question):** **REVISE 2** (FB-STOP-2, FB-STOP-3) · **NEEDS-HUMAN 2** (FB-STOP-1
regulated anchors + named signatory; the perception-validation launch-gate).

**Remaining NEEDS-HUMAN / launch-gates (NOT v1-ship blockers; v1 = STEP-0 + taste + detail-floor):**
1. **STEP-0 keep-with-disclosure vs gate-off** for the live allergen filter (FB-C1). Owner: product + Counsel.
2. **Human perception validation** (floor copy + badge-stack gestalt with allergy-affected users) before
   `…_CARD_ALLERGEN`. Owner: product/design.
3. Named legal/Counsel **signatory + verified-from-text EU 1924/2006 (+AL) anchors** before the regulated
   subset (FB-STOP-1). Owner: Counsel/legal + architect.
4. Card-allergen-unit coverage threshold (allergen denominator + absolute floor); descriptive allowlist
   contents + AL review; per-tenant descriptive-coverage threshold; AL regulated-term review; Contract B
   delivery-path + migration. Owners as recorded.

**Design-time note:** all guardrails remain **specified, not built**. **#12 (single-allergen-source) is the
STEP-0 gate and lands first**, independent of every feature flag, because FB-C1/FB-C2 are live production
red-line crossings.
