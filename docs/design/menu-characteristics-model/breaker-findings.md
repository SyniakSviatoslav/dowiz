# Breaker Findings — Menu Characteristics Model

**Seat:** Breaker (Triadic Council). Attack-only, grounded @ HEAD. No fixes.
**Verdict:** NOT CONVERGED. Open **2 CRITICAL, 3 HIGH**, 3 MED, 3 LOW.
The red-line (allergen safety to diet-restricted people) is where it breaks hardest.

---

## CRITICAL

### C1 · B-SEC/B-DATA · The allergen "hide-X" lens is a false-safety promise — with a near-empty data denominator it returns the WHOLE menu as "safe"
**Break scenario.** The SHIP filter (§5.2 "hide-allergen X (exists)") hides a dish *only on confirmed presence* and keeps "unknown" visible (§7 fail-safe, proposal:112,133). Presence is read from `bomToNutrition(p).allergens` (`MenuPage.tsx:99-113,192-193`). But the qualifying-data denominator is near zero:
- `menu-seed.ts` and every seed file contain **0** bom/allergen rows → the 49-product demo has **0/49** products with any allergen data.
- The acquisition path WRITE-STRIPS allergens to `[]` (`provisioning.ts:50-58`), so scraped `place` products carry an **empty** allergen array even after the bom is served.
- Only owner recipe-built dishes with allergen-tagged supplies have data → the proposal itself concedes "a minority" (§2).

So a nut-allergic customer selects "hide nuts" → `allergens.includes('nuts')` is false for ~all products → **every dish stays visible and reads as the curated safe set.** A filter that returns the entire menu as safe is strictly worse than no filter: it manufactures an implied safety guarantee out of missing data. The §7 fail-safe ("unknown stays visible") is the bug-as-feature — when ~95%+ of rows are "unknown," "visible" reads as "passed the safety filter."
**Violated invariant:** "never imply safe / absence never derived" (ADR Red lines:50). The UI's silence under a *safety lens* IS a derived absence claim.
**Evidence:** `apps/web/src/pages/client/MenuPage.tsx:99-113,192-193`; `apps/api/src/lib/menu-seed.ts` (0 bom); `apps/api/src/modules/acquisition/provisioning.ts:50-58`.

### C2 · B-CONSIST/B-SEC · The per-product `confirm-allergens` flip asserts review without verifying any allergen was authored → "confirmed, data-rich, zero warnings"
**Break scenario.** `confirm-allergens` runs `UPDATE products SET allergens_confirmed = true … RETURNING id` and nothing else (`menu-confirm.ts:18-24`); its own comment states the AI write-stripped `bom[].allergens=[]` and the owner is *expected* to author allergens first — but nothing enforces it. Confirming flips the read-gate (`mig 072:121`) to **serve the bom**, whose macros survive but whose `allergens` are still `[]`. Result: a confirmed scraped dish that genuinely contains peanuts renders **L2 macro chips (looks reviewed/data-rich) + zero allergen chips**. A customer who scans allergens first sees a clean, confident card → eats it. The confirm act is a liability signal ("owner reviewed allergens") with no allergen content behind it.
**Violated invariant:** owner-authority/honesty-anchor — a "confirmed" state must be backed by real declared data; here a real number (kcal) qualifies the card while the safety field is structurally empty. "We never claim absence" is hollow when confirm + macros = an implied all-clear.
**Evidence:** `apps/api/src/routes/owner/menu-confirm.ts:18-24`; `packages/db/migrations/1790000000072_c2-read-gate.ts:121`; strip at `provisioning.ts:50-58`.

---

## HIGH

### H1 · B-CONSIST · Dropping modifier recompute leaves an actively stale label; the "base dish" caption is a passive note against an active badge
**Break scenario.** Modifiers carry `price_delta integer` only — no nutrition, no allergen columns (`mig 1780338982010:15-23`; served shape `mig 065:60-93`). A "light / 420 kcal" base salad + a selected "bacon + double cheese" modifier still shows **"light / 420 kcal"**; a modifier that introduces nuts → still **no allergen chip**; a vegan-framed base made non-vegan by a modifier keeps the framing. §7's mitigation is a static "for the base dish" caption — a passive footnote the calorie/allergen-constrained customer reads *after* they've already trusted the prominent chip. The patch itself flagged this as "where honesty is hardest + most important"; dropping it is the largest scope retreat and leaves the chip lying at exactly the moment of customization.
**Violated invariant:** facts-not-verdict / honest degradation — the chip asserts a property of a dish the customer is no longer ordering.
**Evidence:** `packages/db/migrations/1780338982010_menu_modifiers.ts:15-23`; `proposal.md:23,134`.

### H2 · B-SEC/B-ANTIPATTERN · L2 thresholds are EU/Albanian-regulated nutrition claims, shipped client-side at scale on behalf of every tenant, gated only by a code-comment honor system
**Break scenario.** "light" / "low-calorie" are legally-defined claims (EU Reg 1924/2006; Albanian transposition). The design ships **client-side** derivation (Option B, proposal:71-75) and gates correctness on "every threshold cites a verified per-market anchor" (R-3) — but that gate is a *comment convention*; no `CHARACTERISTIC_RULES` module, no CI denylist, and no anchor exist in the tree today (`grep deriveCharacteristics/CHARACTERISTIC_RULES` → 0 hits). If a threshold is wrong or unverified at flip-on, the platform emits an unlawful nutrition claim across every storefront simultaneously. Compounds the spine inversion (see attack 3): L2 is **platform-derived**, so the platform — not the owner — is the asserter, inverting the "owner=authority, platform=conduit" spine for exactly the regulated layer. "Describe the dish, not the body" is not a shield: the word "light" is itself the regulated term regardless of surrounding wording, and an honesty-anchor (a real number) only converts a wrong threshold into a *confident* wrong claim.
**Violated invariant:** "no regulated body-effect claims" + "owner authority for declarations" (ADR:51,53).
**Evidence:** `proposal.md:71-78,165`; no rule module present (`apps/`, `packages/` grep clean).

### H3 · B-DATA · Supply Library is localStorage-only → per-device macro truth → the regulated "light" claim is non-reproducible and flips across the owner's own devices
**Break scenario.** Nutrition flows from `dos_supplies` in browser `localStorage` (`SupplyLibraryPage.tsx:25,48-59`), snapshotted into `bom` at edit-time (`RecipeEditor.tsx:90-93`). Owner builds Dish X on device A (supplies present) → bom has kcal → card shows "light." Owner edits the same dish on device B (empty/different `dos_supplies`) → re-snapshot writes empty or divergent macros → the **same product's** badge flips light↔nothing↔hearty depending on which device last saved, and the customer sees the last-write-wins value. R-2 "accept for SHIP" while L2 derivation *trusts the bom it displays* means a legally-regulated claim rests on per-device, non-reproducible data with no server source of truth.
**Violated invariant:** read-after-write consistency / honesty-anchor (the "real number" is device-dependent).
**Evidence:** `apps/web/src/pages/admin/SupplyLibraryPage.tsx:25,48-59`; `apps/web/src/pages/admin/RecipeEditor.tsx:90-93`; `proposal.md:18,164`.

---

## MEDIUM

### M1 · B-CONSIST · Per-axis directional arrows are micro-verdicts that re-import the verdict + regulated-claim problem the design claims to avoid
"No global winner" (§4.2) but each axis shows ↑/↓ "lighter/cheaper/faster." An arrow on the lightness axis IS a per-axis verdict, and "lighter wins" rests on the same unverified, regulated "light" threshold as H2 — so the comparison view smuggles back both the verdict and the regulated-claim hazards through the back door. **Evidence:** `proposal.md:92-96`.

### M2 · B-DATA · Allergen-presence layer is near-dead → its rare appearance is *more* trusted (scarcity = signal)
Because seed=0 and scrape-stripped (C1), allergen chips will almost never render. The "minority qualify" framing (§2) understates: for the *allergen* axis specifically it is near-zero. When a chip finally appears it carries outsized authority — and its absence everywhere else trains customers that "no chip = checked & clean," directly feeding C1/C2. **Evidence:** `menu-seed.ts` (0), `provisioning.ts:50-58`, `proposal.md:52`.

### M3 · B-ANTIPATTERN · "hide-allergen (exists)" / "seam partly exists" overstates readiness of the most dangerous surface
The existing filter is a **SHOW-CONTAINS** predicate (`includes(filterAllergen)`, `MenuPage.tsx:192-193`), i.e. the inverse of the proposed safe-direction "hide-X" lens. The dangerous hide-filter is **net-new code**, not "exists." Citing it as already-built understates the build/verification surface of the exact lens that creates the false-safety in C1. **Evidence:** `apps/web/src/pages/client/MenuPage.tsx:192-193`; `proposal.md:109,112`.

---

## LOW

### L1 · B-SEC · "owner declaration" liability anchor is vaporware today
The design leans on owner allergen/diet *declaration* for liability-shift (§8), but `AllergenEditor` is **not imported** into `MenuManagerPage.tsx` and `handleSaveProduct` persists only `taste/stockCount/recipeLines` (`MenuManagerPage.tsx:326-335`). The only persisted allergen act is the C2 boolean flip. The owner-authority liability shield does not exist in the SHIP data path. **Evidence:** grep `AllergenEditor` in MenuManagerPage → not imported; `MenuManagerPage.tsx:326-335`.

### L2 · B-SCALE/scope · Client-only derivation will drift from any future server use
`deriveCharacteristics` is "reusable server-side verbatim" only in aspiration — no shared module exists. When scale forces a server-side filter/comparison/cart re-check, the threshold logic forks → card and server disagree (the exact drift Option-B-vs-A was meant to prevent). **Evidence:** `proposal.md:71-75,99-100`; no module in tree.

### L3 · i18n · al label vocabulary risks body-effect drift; allergen al-set is only partially present
Taste/allergen al strings exist (`i18n-catalog.ts:1372-1379,1091-1098`) but the L2 character vocabulary (light/hearty/rich…) is not yet translated; "light" → Albanian renderings can read as a body/health effect rather than a dish description, and only a subset of the 14 EU allergens has confirmed al keys. Flag for per-market legal+linguistic review before flip-on. **Evidence:** `packages/ui/src/lib/i18n-catalog.ts:1091-1098,1370-1379`; `packages/shared-types/src/allergens.ts:1-4`.

---

## Net verdict
**NOT CONVERGED.** The presentation-layer / zero-migration / pure-function spine is sound and the absence-never red line is honored *in derivation*; but the customer-facing **safety semantics invert** it: with a near-empty allergen-data denominator (seed 0/49 + scrape-strip + confirm-flip-without-author), UI silence under a *safety lens* becomes a derived all-clear (C1, C2). L2 ships a regulated claim client-side, platform-asserted, on non-reproducible per-device data, gated only by a comment (H2, H3), and the modifier-recompute drop leaves chips actively stale at customization time (H1).
**Open: 2 CRITICAL, 3 HIGH, 3 MED, 3 LOW.**

---

# RE-ATTACK round 2 (regression check on the RESOLVE round-1 fix-set)

**Seat:** Breaker, RE-ATTACK r2. Grounded @ HEAD. Attacks the round-1 fixes only (resolution.md + updated proposal.md) for NEW holes. No fixes.
**Verdict:** RESOLVE round 1 did **NOT converge** — the H1/BC-2 fix introduced a new **CRITICAL** (allergen-warning suppression) and the M1 fix reopened the lightness-verdict it closed. **New: 1 CRITICAL, 4 HIGH, 2 MED, 1 LOW.**
**HEAD grounding:** `grep CHARACTERISTIC_RULES|deriveCharacteristics|ALLERGEN_FILTER|char_hidden|"info not provided"|guardrail-names` → **0 hits in apps/packages/tools** (all SHIP code + all 5 guardrails + the marker are prose). `prep_time_minutes` is a single `int NOT NULL DEFAULT 15` — **no min/max/range column** (`1790000000065_products-prep-time.ts:337`).

---

## CRITICAL

### R2-C1 · B-SEC · The H1/BC-2 "suppress, not caption" fix SUPPRESSES a true allergen-PRESENCE WARNING on modifiable dishes — strictly more dangerous than the stale label it replaced (NEW danger introduced by the round-1 fix)
**Break scenario.** The H1/BC-2 fix says: on a product whose modifier groups can change the relevant axis, the "**regulated / diet / allergen-sensitive** label is **SUPPRESSED, not captioned**" (`resolution.md:36,132`; `proposal.md:33,150`). "Allergen-sensitive label" is **never defined as distinct from the allergen-PRESENCE chip** ("contains nuts"). Take a base dish that genuinely **contains nuts** and has modifier groups (e.g. "add sauce" / "remove topping") that touch the allergen axis. Under suppress-on-modifier the **"contains nuts" warning is removed from the card.** The base dish still contains nuts (a modifier can *add* others; it does not retract the base ingredient, and "remove nuts" never clears cross-contamination). A nut-allergic customer sees a card with **no allergen warning** → reads as nut-free → orders. Suppressing a *reassurance* (light/vegan) on customization is safe; suppressing a *warning* hides a true positive — anaphylaxis-grade. This **reopens the absence-never red line from the warning side**: not "missing data read as safe" (C1) but "a present, true warning actively deleted by the suppression rule."
**Guardrail makes it worse.** Guardrail #4 (`resolution.md:198`, `proposal.md:205`) is specified as: "unit assertion that a product with axis-changing modifier groups emits **no regulated/diet/allergen-sensitive label**." As written this guardrail would **codify the deletion** of the "contains nuts" chip — the gate is pointed in the dangerous direction.
**Spec ambiguity is itself the finding.** `proposal.md:148` declares the presence chips + "info not provided" marker a "first-class default UI state," but `proposal.md:150` then suppresses "allergen-sensitive" labels on modifiers with no carve-out for the presence-warning, and never says which of {keep chip / show nothing / show "info not provided"} a modifiable nut dish renders. Downgrading a true "contains nuts" to either silence or "unknown" is a safety inversion in all three readings.
**Violated invariant:** allergen presence-only / **absence-never** / honest-degradation — a true present warning must never be removed by a presentation rule. (ADR Red lines:50.)
**Evidence:** `resolution.md:36,129-136,198`; `proposal.md:30,33,148,150,205`.

---

## HIGH

### R2-H1 · B-CONSIST · The M1 fix is self-contradictory — it bans the "lighter wins" verdict but permits a ↓ arrow on the kcal macro axis, which IS the lightness verdict (reopens M1/H2)
**Break scenario.** The M1 fix lists "**raw macro numbers**" among the arrow-eligible **non-regulated** axes ("Directional arrows (↑/↓/≈) ONLY on … raw macro numbers shown **as the number**," `resolution.md:38,154`; `proposal.md:102`) — yet the very next line gives the example "protein **↑** on A, **kcal ↓** on B" (`proposal.md:103`). A **↓ arrow on calories** reads as "B is the lighter/better choice" — it is exactly the "lighter wins" regulated-lightness verdict the design **deferred** by gating "light" (H2). The arrow re-imports the regulated-claim hazard the resolution claims M1 removed; the word "light" is gated but its directional verdict ships under the label "raw macro." `proposal.md:102` ("as the number, not a verdict") and `proposal.md:103` ("kcal ↓") directly contradict each other.
**Violated invariant:** facts-not-verdict + no platform-asserted regulated claim — a directional calorie arrow is a lighter-is-better micro-verdict on the regulated axis.
**Evidence:** `resolution.md:38,154`; `proposal.md:102-103`.

### R2-H2 · B-SEC · The descriptive/regulated split is drawn on VOCABULARY, but regulation is on MEANING — "protein-forward"/"filling" are regulated claims by synonym, and they ship NOW (no gate)
**Break scenario.** The split routes "protein-forward" (relative) into **descriptive (ships now, no owner gate)** while "source of protein" stays **regulated/gated** (`resolution.md:112`, `proposal.md:81`). A customer (and an EU/AL regulator under 1924/2006) hears the **same nutrient-content message** from "protein-forward" as from "source of protein" — regulation evaluates the conveyed message, not the exact token; synonym routing is the precise loophole the regime forecloses. "**filling**"/"**hearty**" (descriptive, `resolution.md:112`) asserts **satiety = a body effect** ("keeps you full") — a near-synonym of a phrase the design's own body-effect denylist bans (`proposal.md:91`). So at least two "descriptive, ships-now" labels are regulated/body-effect claims in disguise, emitted platform-side across every tenant with **no owner-authority gate** — reopening H2 on the surface the resolution declared safe to ship. Doc-drift confirms the boundary isn't propagated: `proposal.md:54` still lists "**light**" and "**protein-rich**" (regulated phrasings) in the flat SHIP vocabulary count, unmarked as gated.
**Violated invariant:** no regulated body-effect claims + owner-authority for declarations (ADR:51,53).
**Evidence:** `resolution.md:112`; `proposal.md:54,81,91`.

### R2-H3 · B-SEC · Comparison composition row + the §6 2-3 curation cap re-open "silence reads as clean" on SHIP surfaces (the marker is not pinned everywhere it claims to be)
**Break scenario (two SHIP surfaces where the C2/M2 marker is absent):**
(a) **Comparison view.** §4.2 specifies the composition row as "**presence chips per dish; no cleaner verdict**" (`proposal.md:105`) and the macro convention "missing → '—', never inferred" (`proposal.md:103`). The comparison composition row is **never bound to render the "info not provided" marker.** Put dish A ("contains nuts") beside dish B (no allergen data) → B shows blank/"—" → **B reads as "doesn't contain nuts" = clean**, in the highest-stakes side-by-side context. The marker the resolution calls "default everywhere" (`resolution.md:39`) is not specified into this row.
(b) **Card curation cap.** §6 shows "at most **2-3** chips" ranked by priority (`proposal.md:139`). The mandatory "info not provided" marker is **not exempted from the cap.** A dish with taste + 2 descriptive chips fills the slots → the allergen marker is dropped → a card that *looks* fully-characterized silently omits the allergen surface → silence-reads-as-clean. The resolution asserts the marker is "first-class default, not a per-lens option" (`resolution.md:39`) but never carves it out of the curation cap or pins it into the comparison row.
**Violated invariant:** data-absence-never-reads-as-safe (the M2 fix's own red line, `resolution.md:62`).
**Evidence:** `proposal.md:103,105,139`; `resolution.md:39`.

### R2-H4 · B-OPS/B-ANTIPATTERN · The 5 guardrails are prose, not gates — and the SHIP allergen-safety rests on guardrail #5, which is unbuilt and unbound to the SHIP flag (the exact failure mode flagged this session)
**Break scenario.** All five guardrails (`resolution.md:189-200`, `proposal.md:201-206`) are titled "**to author**" and **0 exist in tree** (grep clean). The SHIP allergen-presence layer's *only* safety net is guardrail **#5 (no-clean-from-empty)** — yet (i) it is unbuilt, and (ii) the resolution **never makes the SHIP flag flip contingent on #5 being red→green** ("before the respective flag flips" is prose, not a binding). A "regulated-subset-off" #2 described as gating on a missing anchor table is, until the test exists, just a **flag default** (`proposal.md:195` sub-flags) — flippable without the anchors, which is precisely how §5/§6 work was burned this session (a claimed guardrail that wasn't built). Worse, guardrail #4 as specified (R2-C1) is pointed at the dangerous direction. A resolution that lists guardrails as the enforcement of its red lines while none are falsifiable artifacts has not actually moved the red lines from prose to gate.
**Violated invariant:** "a fix is not done without a deterministic guardrail" (harness ratchet) — applied to the design: the safety claims are unenforced.
**Evidence:** `resolution.md:189-200`; `proposal.md:201-206`; HEAD grep (0 guardrail/rule-module hits).

---

## MEDIUM

### R2-M1 · B-DATA/UX · The "info not provided" marker becomes wallpaper at 0/49 coverage — re-creating the no-signal state it was meant to fix
**Break scenario.** Demo coverage is 0/49 (and acquisition write-strips allergens) → the marker renders on **~49/49 cards**. A control that fires on **100% of instances carries zero discriminating bits** and trains banner-blindness within a single session; the customer learns to skip the allergen row entirely. When the rare real "contains nuts" chip eventually appears (M2's scarcity problem from the other side), it lands in a row the eye has already tuned out — the omnipresent marker **inherits its ignorability to the rare warning** sitting in the same row. The C2/M2 fix's stated purpose ("silence reads as unchecked, not clean") is defeated by habituation: a marker on every card reads as no-signal, the same end-state as a blank.
**Violated invariant:** the M2 remedy's own goal (defuse scarcity-as-signal) — undermined, not met, at the live denominator.
**Evidence:** `resolution.md:39,62`; `proposal.md:30,148`; `menu-seed.ts` (0 bom), `provisioning.ts:50-58`.

### R2-M2 · B-DATA · The comparison prep-time "RANGE" fabricates a spread from a single stored integer (false precision in the inventing-uncertainty direction)
**Break scenario.** M1/§4.2 present prep-time as a "**RANGE** … not a false-precision single number" (`resolution.md:38`, `proposal.md:106`). But the only stored field is `prep_time_minutes int NOT NULL DEFAULT 15` — a **single integer, no min/max column** (`1790000000065_products-prep-time.ts:337`). A range can only be **synthesized** (e.g. `±X` or `×factor`) from that one number → the range endpoints are **fabricated data**, the inverse false-precision: inventing a spread the data does not contain. "Mirroring delivery-ETA" does not supply real min/max here. Either the comparison shows a fabricated range (fabrication red line) or it shows the single number it called false-precision — the fix has no honest third option on current data.
**Violated invariant:** never-fabricate (a synthesized spread is invented data) / honesty-anchor (the range endpoints have no qualifying number).
**Evidence:** `proposal.md:106`; `resolution.md:38`; `1790000000065_products-prep-time.ts:337`.

---

## LOW

### R2-L1 · B-ANTIPATTERN/scope · On real/seed data the reduced SHIP scope is ~95% empty — its only universally-visible net-new artifact is a 100%-coverage disclaimer
**Break scenario.** Quantifying SHIP against the live denominator: descriptive L2 needs a `bom` → **0/49** (demo 0 bom; scraped `place` write-stripped, no bom until owner rebuilds). Allergen-presence chips need `bom[].allergens` → **0/49**. Comparison macros need both dishes to carry a number → **0** comparable macro pairs. Taste is owner-entered/optional/sparse (R-4). So on a real menu the entire L2 + allergen-presence + macro-comparison surface renders **nothing**; the only universally-rendering element is the "info not provided" marker (R2-M1 wallpaper) plus whatever sparse taste exists. The SHIP "characteristics model" — card curation engine, comparison view, derivation module, marker — is built for data that is **near-absent on real menus**; its net user-visible effect today is **adding a disclaimer to ~100% of dishes**. Honest (degrades to silence), but the surface/benefit ratio is the finding: a comparison view + curation logic for ~0 qualifying chips is surface built ahead of data.
**Violated invariant:** back-of-envelope / "runtime minimal" — building presentation surface for a denominator that is near-zero by construction.
**Evidence:** `proposal.md:53` (minority qualify), `resolution.md:5` (0 bom, write-strip); HEAD seed/scrape facts.

---

## Net verdict (RE-ATTACK round 2)
**RESOLVE round 1 did NOT converge.** The fix-set introduced a **new CRITICAL** danger: the H1/BC-2 "suppress, not caption" rule, by folding the undefined "allergen-sensitive label" into suppression, **deletes a true "contains nuts" warning on modifiable dishes** (R2-C1) — strictly more dangerous than the stale label it replaced, and guardrail #4 as written would enforce it. The M1 fix **reopened its own lightness verdict** via the kcal ↓ arrow (R2-H1). The descriptive/regulated split is a **synonym loophole** that ships regulated/body-effect claims now (R2-H2). The C2/M2 marker is **not pinned into the comparison row or exempted from the curation cap**, reopening silence-as-clean on SHIP surfaces (R2-H3), and at 0/49 coverage the marker is **wallpaper** (R2-M1). All five guardrails are **prose, 0 in tree, unbound to the flag** (R2-H4) — the failure mode flagged this session.
**New this round: 1 CRITICAL, 4 HIGH, 2 MED, 1 LOW.** The standout regression is R2-C1 (warning-suppression): the round-1 fix made the allergen surface *less* safe than before it.

---

# RE-ATTACK round 3 (convergence gate — regression check on the RESOLVE round-2 fix-set)

**Seat:** Breaker, RE-ATTACK r3. Grounded @ HEAD. Attacks the **round-2** fixes only (resolution.md §"RESOLVE round 2" + updated proposal.md) for NEW CRITICAL/HIGH. No fixes.
**Verdict:** RESOLVE round 2 did **NOT converge**. The taste-first **coverage gate** (R2-M1/Q5 fix) re-violates the **"never-suppress-a-warning"** red line that the R2-C1 fix had just established, and **contradicts** the two SHIP-gating safety guardrails (#5, #4-positive). Plus the register denylist (#6) has live synonym survivors, and the partial-declaration completeness gap is a demonstrable comparison-row inversion.
**HEAD grounding:** rule module / guardrails / marker still **unbuilt** (grep ~0 in apps/packages/tools — all SHIP code + 6 guardrails + marker are prose, design-time). Detail modal renders allergens **only** when `bomToNutrition(detailProduct).allergens.length > 0` (`MenuPage.tsx:1078`) — **no completeness caveat, nothing on empty**. `prep_time_minutes` still a single `int` (`…065:337`).

---

## HIGH

### R3-H1 · B-SEC/B-OPS · THE KEY ATTACK — the coverage gate (R2-M1/Q5) re-suppresses the allergen WARNING, and CONTRADICTS guardrails #5 + #4-positive; the warning/marker is NOT exempted from the gate
**Direct answer to the convergence question: the allergen-PRESENCE warning is NOT exempt from the coverage gate.** The resolution explicitly bundles **"allergen-presence chips AND the universal 'info not provided' marker"** *into* the band that "does **NOT render** … until that tenant's data clears a per-tenant minimum coverage" (`resolution.md:344-346`; `proposal.md:29`). So a tenant at, say, 3/49 allergen coverage — one dish of which genuinely **contains nuts** (owner-built `bom[].allergens=['nuts']`) — is **below threshold → the whole band does not render → the at-a-glance "contains nuts" warning chip is hidden.** Warning visibility is now decoupled from per-dish risk: the **same nuts dish** shows the chip in a high-coverage tenant and is **silent** in a low-coverage tenant, purely because of *unrelated* dishes' data density.

**This re-violates the round-2 red line.** `resolution.md:397` lists as an intact red line: *"never-suppress-a-warning (R2-C1 carve-out + guardrail #4-positive)."* The R2-C1 fix carved the warning OUT of the *modifier* suppression path; the R2-M1 fix then puts it back UNDER a *coverage* suppression path. Two round-2 fixes contradict on the same red line.

**The guardrail binding is internally contradictory (the convergence-blocker).** The SHIP flag "must not flip on until **#1, #4-positive, #5, #6** are red→green" (`resolution.md:339-340`). But:
- **#5** asserts (unconditional, across card/detail/comparison): *empty/absent `bom[].allergens` renders the "info not provided" marker, never nothing* (`resolution.md:334`). Below the coverage gate the band — including the marker — **does not render** → #5's assertion is **FALSE below the gate**.
- **#4-positive** asserts: *a modifiable dish with `bom[].allergens ⊇ {X}` MUST render "contains X"* (`resolution.md:333`). Below the gate the presence chips do not render → #4-positive's assertion is **FALSE below the gate**.

So the implementer cannot author #4-positive/#5 **red→green** without first choosing: (a) scope the guardrails "above-gate only" → the below-gate warning-suppression path is **unguarded** (no guardrail asserts the warning ever appears there), or (b) make them unconditional → the coverage gate **cannot legally hide the band**. The resolution specifies **both** as binding and reconciles neither. The R2-H3 invariant ("marker on **every** SHIP surface, **never nothing**", `resolution.md:289`) is in flat contradiction with the R2-M1 gate ("band incl. marker does **not render** below threshold", `resolution.md:344-346`).
**Why HIGH and not CRITICAL:** the verified floor (`MenuPage.tsx:1078`) means a *non-empty* nuts dish still surfaces "contains nuts" one-tap-away in the detail modal below the gate = the pre-feature status quo, so it is not *worse than today* and there is no false "clean band" shown. But the resolution **relies on that floor without guardrailing it**, the stated red line is literally broken for the chip surface, and the SHIP-flag binding is self-contradictory → not converged.
**Violated invariant:** never-suppress-a-warning / data-absence-never-reads-as-safe (`resolution.md:395-397`); harness "guardrail = deterministic + the fix is bound to it" (the SHIP flip is bound to guardrails that contradict a SHIP rule).
**Evidence:** `resolution.md:289,333-334,339-340,344-346,357-360,397`; `proposal.md:29,151,206,210-211`; floor at `apps/web/src/pages/client/MenuPage.tsx:1078`.

### R3-H2 · B-SEC · The register denylist (#6) is a TOKEN list; the SHIP descriptive set still carries satiety/energy/nutrient-content survivors — R2-H2's synonym loophole recurs on new synonyms
After dropping "filling"/"protein-rich", the descriptive SHIP set is "**hearty-portion / generous-portion, rich, carb-forward, protein-forward** (only if it clears the register)" (`proposal.md:55,82`). Survivors that carry a denylisted **meaning** the token-list (`proposal.md:82`) does not catch:
- **"hearty"** = the exact synonym of the dropped **"filling"** — "a hearty meal" *fills you up / is substantial / satisfying* = the **satiety/body-effect** register R2-H2 banned. Appending "-portion" does not neutralize "hearty"; "hearty" itself is the satiety claim. The dropped term came back through its own synonym — the precise failure R2-H2 claimed to fix.
- **"rich"** = **energy-density** ("rich" food = high fat/calories) and the head of the regulated nutrient-content form **"rich in"** (`proposal.md:82` denylists "rich in" but ships bare "rich"). A customer hears the energy/indulgence claim either way.
- **"carb-forward"** = **nutrient-content by synonym** ("high in carbs"), identical "-forward" construction to **"protein-forward"** — yet "protein-forward" is gated *pending human review* while **"carb-forward" ships UNGATED** (`proposal.md:82`). Same construction, two verdicts: an internal inconsistency that proves the split is not meaning-based.

**#6 cannot be authored green as specified.** A "denylist test over rendered vocabulary in al+en" is a **token/substring** match — it will not catch "hearty"/"rich"/"carb-forward" conveying a banned register, and "protein-forward **only if it clears the nutrient-content register on review**" (`proposal.md:82`) **punts to human review**, which is not a deterministic guardrail. A SHIP-gating guardrail that requires human judgment to decide its own inputs cannot be red→green.
**Violated invariant:** no regulated/body-effect claims (regulation is on conveyed meaning, not token); deterministic-guardrail.
**Evidence:** `proposal.md:55,82,212`; `resolution.md:298-307,335`.

### R3-H3 · B-SEC · Partial allergen declaration reads as a COMPLETE profile — the marker fires only on EMPTY data, so a non-empty-incomplete dish beside a "contains nuts" dish reads "no nuts" (comparison inversion via the third door)
The R2-H3 fix pins the marker into the comparison row "for a no-data dish" and guardrail #5 fires on **"empty/absent `bom[].allergens`"** (`resolution.md:289,334`). Neither covers the **partial** case: a non-empty but incomplete declaration. Owner declared milk only (`bom[].allergens=['milk']`, never reviewed for nuts) → `length>0` → **no marker fires** → the row renders "**declared to contain milk**". Beside a dish that declares "contains nuts", a nut-allergic customer reads the milk dish as **"milk only ⇒ no nuts ⇒ safe for me"** — the C1/M2 inversion, in the highest-stakes side-by-side context, through a door the round-2 marker does not guard. Verified: the only allergen surface that exists today renders non-empty lists with **no completeness caveat** (`MenuPage.tsx:1078`). The Q1 framing ("declared to contain" + **ambient** "confirm with the venue", `proposal.md:31,151`) is the sole counter, but it is low-salience prose overridden by the high-salience visual "milk | nuts", and the system has **no concept of review-completeness** — non-empty `bom[].allergens` is treated as *the* profile. The "unchecked" signal (the marker) is structurally unavailable for partial data. Counsel flagged this as a round-2 residual; it is a concrete safety inversion on real data (partial declarations are the *expected* owner behavior — list the obvious allergen, forget the rest).
**Violated invariant:** data-absence/data-incompleteness-never-reads-as-clean; allergen presence-only/absence-never (a partial list implies an absence claim for everything not listed).
**Evidence:** `resolution.md:289,334`; `proposal.md:31,104,106,151`; `apps/web/src/pages/client/MenuPage.tsx:1078`.

---

## MEDIUM

### R3-M1 · B-OPS · Guardrail #1's "positive-only" clause is not a falsifiable lint/grep — only "filter absent from SHIP" is checkable
#1 is specified as "lint/grep: fail if any allergen-axis predicate **removes** dishes (hide/only-safe) or `…_ALLERGEN_FILTER` is wired to anything but a **positive-only view**" (`resolution.md:332`; `proposal.md:207`). A grep cannot decide the *semantics* of a predicate (whether a filter hides-to-safe vs shows-contains) nor verify "positive-only." For SHIP this is adequate-by-luck (the filter is deferred entirely, so the checkable assertion collapses to "no allergen filter is wired into the rendered set" — greppable), but the forward "positive-only" guarantee it claims to enforce is **prose, not an artifact**. As a permanent ratchet it overstates what the gate can mechanically catch.
**Evidence:** `resolution.md:332`; `proposal.md:207`.

---

## Cleared this round (regression check — round-2 fixes that DID hold)

- **R2-C1 (modifier path) — HOLDS.** The carve-out + guardrail #4-positive correctly stop the *modifier* suppression path from deleting a warning. (The warning is re-exposed only via the *new* coverage path — R3-H1, a different mechanism.)
- **R2-H1 (kcal arrow) — HOLDS.** Arrows restricted to price + prep-time; macros numbers-only (`proposal.md:103`). No residual lightness-verdict.
- **R2-M2 (prep-time range) — HOLDS.** Point estimate "~N min"; no fabricated spread (`proposal.md:107`). Confirmed single-int source.
- **Taste-first SHIP (attack #5) — CLEAN.** Taste is owner-entered (owner authority, not platform-asserted), profile-not-chip "never claiming a body effect" (`proposal.md:67`), comparison "explicitly not a winner" (`proposal.md:106`), missing axes → "—" never inferred. No net-new liability surface. This is the genuine minimal ship. (Minor: the "richness" taste axis sits adjacent to the "rich" descriptive chip of R3-H2 — watch for conflation; low.)

---

## Net verdict (RE-ATTACK round 3)

**NOT CONVERGED.** The round-2 fix-set repeated the round-1/round-2 pattern: a fix introduced a new danger.
**Open: 3 HIGH, 1 MED.**
- **R3-H1 (the key attack):** the **coverage gate is NOT exempt for the allergen warning/marker** — it hides a true "contains X" chip for below-threshold tenants, re-violating the **"never-suppress-a-warning"** red line the R2-C1 fix established, and the two SHIP-gating safety guardrails **#5 and #4-positive are specified UNCONDITIONAL against a gate that hides their subject** → they cannot be authored red→green without choosing between the gate and the guardrail; the resolution binds the SHIP flag to **both** and reconciles **neither**. Held below CRITICAL only by the un-guardrailed detail-modal floor (`MenuPage.tsx:1078`), which keeps it "not worse than today" for non-empty data.
- **R3-H2:** register denylist (#6) survivors — **"hearty"** (=dropped "filling"), **"rich"** (energy-density), **"carb-forward"** (ungated, same construction as gated "protein-forward"); a token-test cannot catch them and "protein-forward only if it clears the register on review" punts to human judgment → #6 is not authorable green.
- **R3-H3:** partial (non-empty-incomplete) allergen declarations read as a **complete profile** — the marker fires only on EMPTY data → "milk only ⇒ no nuts" comparison inversion; round-2 R2-H3/Q1 do not close it.

**Decisive flag (per the convergence brief):** the coverage-gate-vs-allergen-warning question resolves **against** convergence — the warning is **not** exempt from the gate (explicit in `resolution.md:344-346`), the gate **can** hide a real allergen warning chip, and no guardrail makes the warning/marker gate-exempt. Until the architect EITHER exempts the allergen-presence chips + marker from the coverage gate (and guardrails it) OR explicitly scopes #4-positive/#5 and guardrails the detail-modal floor as the below-gate safety surface, the SHIP-gating guardrail set is self-contradictory and the design has not converged.

---

# RE-ATTACK round 4 (convergence gate — regression check on the RESOLVE round-3 fix-set)

**Seat:** Breaker, RE-ATTACK r4. Grounded @ HEAD. Attacks the **round-3** fixes ONLY (resolution.md §"RESOLVE round 3" + updated proposal.md) for NEW CRITICAL/HIGH. No fixes.
**Verdict:** RESOLVE round 3 did **NOT converge**. The round-3 fix closed the warning-suppression door (warning now ungated from modifier + coverage + cap — confirmed below) but **opened a new one by the same pattern**: it protected the warning by **asymmetrically coverage-gating the *no-data marker*** (the "we don't know" signal) while leaving the **warning ungated and on the card**. On a partially-authored menu the visible warning chips convert the *absence* of a chip on a no-data dish into a **contrast-read of "allergen-free"** — a clean-read that did **not** exist pre-feature and that the round-3 door-check explicitly (and wrongly) declared impossible ("no clean-read sits behind any door").
**HEAD grounding (unchanged, design-time):** rule module / guardrails / marker / reliance-bound still **unbuilt** (grep ~0 in `apps/`,`packages/`,`tools/` for `CHARACTERISTIC_RULES|deriveCharacteristics|char_hidden|not a complete allergen list`). Detail modal still renders allergens **only** when `bomToNutrition(detailProduct).allergens.length > 0` (`MenuPage.tsx:1078`) — the round-3 floor fix is spec, not code. `prep_time_minutes` still single `int`.

---

## Per-vector results (the five questions put to this seat)

### 1. Is the WARNING ungated from EVERY suppression door? — YES, all three closed and mutually consistent. (Confirmed plainly.)
- **(a) modifier-suppress (R2-C1 carve-out):** still excludes the warning. `proposal.md:35,154` + guardrail #4-positive (`resolution.md:733`) assert a modifiable dish with `bom[].allergens ⊇ {X}` MUST render "contains X". CLOSED.
- **(b) coverage gate (R3-H1):** warning ungated — gate re-scoped to descriptive-axis only; warning renders on card+detail+comparison ALWAYS (`proposal.md:30`). CLOSED.
- **(c) §6 2–3 curation cap:** warning exempt — the allergen-presence row is "a separate, always-rendered safety row, never evicted by taste/descriptive chips" (`proposal.md:143`). 3 taste/descriptive chips CANNOT evict a "contains nuts" chip. CLOSED.
**The authored allergen WARNING is droppable on no surface.** That specific question converges. **But the fix that achieved (b) is itself the new door (R4-H1).**

### 2. Detail-floor invariant + guardrail #5 — authorable red→green now? — YES, and the round-3 self-contradiction is resolved.
#5 is now concrete + falsifiable and no longer contradicts the gate: (a) detail-empty ⇒ "info not provided" **unconditional**; (b) card-non-empty ⇒ "contains X" **unconditional**; (c) no surface clean-from-empty — with the *card no-data marker* explicitly carved out as the only gate-able element (`resolution.md:734`, `proposal.md:213`). The R3-H1 contradiction ("#5 unconditional vs gate hides the band") is gone because the gated element (card marker) is now excluded from the unconditional clauses. **#4-positive, #5, #6 (subset check), #1 (filter-absent) are all now authorable red→green.** No residual *unauthorable* guardrail. (The old R3-H2 punt-to-human and R3-M1 positive-only-grep problems are fixed.) **Caveat:** #5(c) guards a *single dish's* empty data; it does **not** assert anything about the cross-dish contrast in R4-H1 — so it goes green while the new door is open.

### 3. Empty v1 allowlist + dormant gate — coherent? — Structurally yes; no divide-by-zero / stray frame. BUT it is the precondition for R4-H1.
No path divides by allowlist size (coverage denominator = total dishes, not allowlist size); empty allowlist ⇒ descriptive coverage = 0/N for every tenant ⇒ gate closed for all ⇒ no descriptive chips **and no card-level no-data marker** render anywhere; no empty band frame (band renders nothing). Detail floor renders "info not provided" unconditionally — not stray, intended. So "taste-first SHIP" = **taste + ungated warning + detail floor + reliance bound**, descriptive band dormant. **Coherent — but NOT safe**, because the dormant gate means the no-data marker NEVER reaches the card at v1 (see R4-H1): the card can show "contains X" (warning) or blank, but never "we don't know."

### 4. Reliance bound (R3-H3) — does "not a complete list" defuse partial-reads-as-complete in comparison? — Partially, and it carries its own tension. (MED residual, not a blocker on its own.)
Surface-attaching the caveat to every cell is the best a presence-only system can do and it does weaken the literal "milk ⇒ no nuts" read. Two residuals: (i) a "not a complete list" note on **100% of cells** is the **same 100%-coverage habituation** the design used to justify gating the no-data marker as "wallpaper" — the design treats the same omnipresence as *noise* (marker, gated) and as *signal* (caveat, mandatory), an inconsistent stance; at-glance the high-salience "milk | nuts" visual still dominates the low-information caveat. (ii) The caveat only rides **where an allergen surface exists**; a no-data dish below the dormant gate has no card surface to carry it (feeds R4-H1). Exhaustiveness-implication is **mitigated, not killed** — acceptable as the irreducible floor of presence-only, so MED, but the noise/signal inconsistency should be recorded.

### 5. NEW door (moved risk)? — YES → **R4-H1** below.

---

## HIGH

### R4-H1 · B-SEC · THE ROUND-4 EQUIVALENT — the round-3 fix protected the warning by coverage-gating the *no-data marker*; on a partially-authored menu the visible warning chips make a no-data card read "allergen-free" by CONTRAST (a clean-read the round-3 door-check declared impossible)
**The moved risk (the pattern repeats).** Round 2 hid the warning; round 3 fixed that by **ungating the warning** AND **moving the no-data marker behind the descriptive coverage gate** as "noise" (`proposal.md:29,152`; `resolution.md:583-584`). The risk did not close — it **moved from the warning to the unknown-state signal.**

**Break scenario (the expected real-world state, not an edge case).** A tenant authors allergens on the *obvious* dishes only — `bom[].allergens=['nuts']` on 5 of 40 dishes, nothing on the other 35 (the documented expected owner behavior, per R3-H3: "list the obvious allergen, forget the rest"). At v1 the descriptive allowlist is EMPTY → the descriptive coverage gate is closed for every tenant (§ attack-3) → **the card-level no-data marker renders on NO card.** But the warning is ungated:
- the 5 nut dishes show **"declared to contain nuts"** on the card,
- the 35 no-data dishes show **nothing** on the card.

A nut-allergic customer scans the 40-dish menu, sees 5 flagged and 35 blank, and reads **"the 35 unflagged dishes are nut-free."** The 35 are actually **unknown**. This is `data-absence-reads-as-clean` at the **card/menu-scan surface** — a SHIP surface.

**Why the round-3 door-check is wrong.** R3-H1's door-check dismissed this as "byte-identical to today's pre-feature card … manufactures no new clean-read at the card level" (`resolution.md:606-608`) and concluded "**no clean-read sits behind any door**" (`:620`). That is **false**: the door-check evaluated a no-data card *in isolation*. It is **not** byte-identical to today, because **today no card carries any allergen chip** (allergens render only in the detail modal, `MenuPage.tsx:1078`) — there is no contrast. The round-3 act of putting the warning chip on the card (the very fix for R3-H1(b)) is precisely what converts "blank" from "no information" (today, uniform) into "no allergen" (post-fix, by contrast with visible warnings). The habituation the door-check called protective ("warning chip appears iff authored allergen" — TRUE) produces a **false inference** ("no warning ⇒ safe") exactly because the disambiguating "we don't know" signal (the no-data marker) was gated away on the same surface.

**Why the reliance bound (R3-H3) does not save it.** The bound rides "on the allergen surface — card affordance + detail" (`resolution.md:654`). A no-data dish below the dormant gate has **no card allergen surface** → no card-level caveat → nothing on the card tells the customer "unknown." (If the architect instead puts the caveat on *every* card including no-data ones, that universal caveat IS the no-data marker the design just gated away as wallpaper — contradiction; and it reintroduces the 100%-coverage habituation of attack-4. Either branch is incoherent.)

**Two-denominator residue (reinforces this).** The dissolution ungated the *warning* but left the *no-data marker* — an **allergen-axis safety signal** — gated by the **descriptive denominator**. So the visibility of the "we don't know about this dish's allergens" signal depends on how many *descriptive* chips the tenant has, a quantity unrelated to allergen data. A tenant with rich allergen authoring but no descriptive chips (every tenant at v1) never shows the marker on cards. The two-denominator category error survives for the marker.

**Why HIGH, not CRITICAL.** The detail-floor fix means the truth ("allergen info not provided") is **one tap away** for any no-data dish, and no fabricated "allergen-free" badge is shown — so it is not a total inversion and matches the severity the council itself assigned R3-H1. But at the **at-a-glance card-scan** level it is **worse than today** (today: no warning chips, no contrast, no clean-read), the clean-read is **manufactured by the round-3 fix**, and it crosses the stated red line *"data-absence never reads as a clean allergen state on any surface"* (`resolution.md:714`) at the card surface — a surface the resolution explicitly claims is clean.
**Violated invariant:** data-absence-never-reads-as-clean / allergen presence-only / absence-never (`resolution.md:714`); and the round-3 self-assertion "no clean-read sits behind any door" (`resolution.md:620`) is falsified.
**Evidence:** `proposal.md:29,30,152`; `resolution.md:583-584,606-608,620,654,714`; floor at `apps/web/src/pages/client/MenuPage.tsx:1078`.

---

## MEDIUM

### R4-M1 · B-UX · The mandatory per-cell reliance bound is the same 100%-coverage "wallpaper" the design gates the no-data marker to avoid — inconsistent treatment of identical omnipresence
The design gates the card no-data marker because a signal firing on ~100% of cards "carries zero discriminating bits = wallpaper" (`resolution.md:618`, R2-M1). It simultaneously mandates the "not a complete allergen list — confirm with the venue" bound on **every** allergen cell/surface, always (`resolution.md:652-661`). Same 100%-coverage property, opposite verdict (noise → gate it; caveat → mandate it). At comparison glance the persistent caveat habituates and is overridden by the high-salience "milk | nuts" visual — the exact override R3-H3 said it was fixing. Exhaustiveness-implication is **mitigated, not killed**; acceptable as the presence-only floor, but the noise/signal inconsistency should be recorded so a future reader does not treat the caveat as a solved discriminator.
**Evidence:** `resolution.md:618,652-661`; `proposal.md:107,152`.

---

## Cleared this round (round-3 fixes that DID hold)
- **R3-H1 warning-ungating (the warning itself) — HOLDS.** Warning is droppable on no door: modifier (R2-C1/#4-positive), coverage (descriptive-only gate), cap (§6 separate row). The three doors are closed and mutually consistent. *(The fix's side effect — gating the no-data marker — is the new door R4-H1; the warning's own protection is sound.)*
- **R3-H2 allowlist inversion — HOLDS.** #6 is now a deterministic subset check (rendered descriptive set ⊆ closed reviewed allowlist); human judgment moved upstream, per label; v1 allowlist empty → descriptive band dormant. Authorable red→green. "hearty"/"rich"/"carb-forward" cannot ship until reviewed. The R3-H2 synonym-survivor attack is closed.
- **R3-M1 #1 scope — HOLDS.** #1 now claims only "no allergen filter wired into the rendered set" (greppable); positive-only is a recorded re-enable design rule, not an overstated grep. Honest.
- **Detail-floor invariant + #5/#4-positive authorability — HOLDS.** The R3-H1 guardrail self-contradiction is resolved (card marker carved out as the only gate-able element). All SHIP-gating guardrails are now individually authorable and internally consistent.
- **Two-denominator (for the warning) — HOLDS.** Warning + detail floor are ungated, so no arithmetic suppression of a warning. *(Residue survives for the no-data marker — folded into R4-H1.)*

---

## Net verdict (RE-ATTACK round 4)
**NOT CONVERGED.** The pattern held a fourth time: the round-3 fix moved the risk rather than closing it. It correctly **ungated the warning from all three doors** (confirmed), but did so by **coverage-gating the no-data marker** (the unknown-state signal) on the same card surface while leaving the warning visible — so on the *expected* partially-authored menu, visible warning chips make an unflagged no-data card read "allergen-free" **by contrast**, a clean-read that is **new vs today** and that the round-3 door-check explicitly declared impossible.
**Open: 1 HIGH (R4-H1), 1 MED (R4-M1).**
**The convergence-blocker (R4-H1), and the shape of a fix (architect's call, not mine):** the design must either (a) make the **unknown-state signal symmetric with the warning** — i.e. the per-dish "allergen info not provided" marker is **ungated on the card** wherever any allergen warning can appear on the same menu (so blank never means clean by contrast), and guardrail it; **or** (b) prove the at-a-glance card-scan cannot manufacture a contrast-clean while the no-data marker is gated — which the current "byte-identical to today" argument does not, because the ungated warning chip is itself net-new on the card. Until then the red line *"data-absence never reads as a clean allergen state on any surface"* is broken at the card surface, and the design has not converged.

---
---

# RE-ATTACK round 5 — convergence gate (attack ONLY the RESOLVE round-4 fix)

**Seat:** Breaker, RE-ATTACK r5. Read-only grounding; no product change.
**Re-grounded @ HEAD (verified this round):** `apps/web/src/pages/client/MenuPage.tsx:1078` — `{(bomToNutrition(detailProduct).allergens.length > 0) && (…)}`: allergens render **only in the detail modal, only on `length>0`, nothing on empty, no completeness caveat**, and **no card carries any allergen chip**. Round-4's baseline claim is TRUE. Rule module / guardrails / marker / reliance-bound still unbuilt (grep ~0) — design-time.

## What round 4 actually fixed — and the contrast door IS genuinely closed (confirm plainly)

The round-4 move (DETAIL-FLOOR-ONLY for v1 + the card-allergen **all-or-nothing** weld, guardrail #7) **closes the R4-H1 contrast door without relocating the risk** — the first fix in this design that does so. Walked in PAIRS at the card level:

- **Vector 1 — card contrast: CLOSED.** v1 elevates *no* allergen element to the card (no warning chip, no marker). Two dishes — A=`bom[].allergens=['nuts']`, N=no-data — render **identical** cards (uniform allergen-silence). No card asserts an allergen state, so no blank reads "nut-free by contrast." This is byte-for-byte today's baseline (`:1078`, no card chip), which the breaker itself certified contrast-free. **Removing** the surface to a proven-clean baseline cannot manufacture a contrast that did not exist there. Round 4 *removes*, does not *relocate* → no asymmetric seam → the structural precondition for the rounds-1–4 door (a moved half vs a stationary counterpart) is absent. The later card unit welds warning+marker (#7), so even when on, A→warning / N→marker / P→partial+bound — no blank at any coverage level. **The contrast door converges.**
- **Vector 4 — #7 + #5 authorable/deterministic: YES.** #7 ("card warning chip present while a no-data card renders no marker ⇒ RED") is a concrete, falsifiable, deterministic assertion (v1 unit-OFF ⇒ both absent ⇒ trivially green; permanent ratchet). #5(a)/(b) detail-floor are concrete + unconditional. #1 (filter-absent grep) / #6 (allowlist subset) are deterministic. The flip is explicitly bound to {#1,#4-positive,#5,#6,#7}. No *unauthorable* guard remains. The round-3 self-contradiction is gone.
- **Vector 3 — detail wording: HONEST.** Empty → "allergen info not provided — ask the restaurant" reads as *no info / confirm with venue*, not a fabricated "contains none." Non-empty → "declared to contain X" (provenance-framed, not "contains"). Neither asserts absence. No fabricated-absence read.

**So vectors 1 and 4 are closed; vector-3 *wording* is closed; there is NO 5th *relocation* door.** Round 4's specific claim ("closes without relocating") holds.

## But the convergence GATE (0 open HIGH) is not met — one HIGH on a different axis

The round-4 *guardrail table* was rebuilt ("round-4 form", `resolution.md:962-970`). Two spec-mandated allergen-honesty protections are **specified in prose but NOT transcribed into the ship-gating guardrail set {#1,#4-positive,#5,#6,#7}** to which the flip is bound. The design's own flip contract (`resolution.md:993`: "Building #1, #4-positive, #5, #6, #7 red→green is the implementer's gate to flip") therefore **green-lights builds that omit them** — both anaphylaxis-grade. This is precisely the failure-class R2-H4 declared must not exist ("guardrails as artifacts; flips bound to them — so safety is not left to implementer care"). Vector 3's real question — *is the reliance bound actually attached, not just specified?* — answers: **specified, not bound; detachable at ship time with all five gates green.**

## HIGH

### R5-H1 · B-CONSIST/B-SEC · Two spec-mandated allergen protections (R3-H3 partial-incompleteness bound; R2-H3 comparison explicit-marker) are ABSENT from the round-4 ship-gating guardrail set — the flip contract green-lights anaphylaxis-grade builds

Read every ship-gating guard (`resolution.md:962-970`): #1 no-hide-filter; #4-positive (detail warning renders; card-unit warning rides *with* marker; no reassuring chip); #5 (a) detail-empty⇒marker (b) detail-non-empty⇒"declared to contain X" (c) **no surface renders a clean state from empty**; #6 descriptive-allowlist subset; #7 card all-or-nothing. **None asserts (i) the "not a complete allergen list — confirm with the venue" reliance bound, nor (ii) the comparison no-data cell's explicit "allergen info not provided" text.**

**Instance (i) — partial-as-exhaustive (R3-H3), un-bound.** Owner authors the *obvious* allergen only: `bom[].allergens=['milk']` on a dish that also contains nuts ("list the obvious, forget the rest" — the documented expected behavior, R3-H3). #5(b) green: detail renders "declared to contain milk". The "not a complete list" bound — the *only* thing R3-H3 specifies to stop this reading as exhaustive — is in no gate. A build renders "declared to contain milk" with **no incompleteness caveat**, passes {#1,#4,#5,#6,#7}, **flips**. Nut-allergic customer reads "milk listed, no nuts ⇒ safe for me" → eats → anaphylaxis. The "declared to contain" framing softens *provenance*, it does **not** convey *incompleteness*; the human default read of a one-item allergen list is "this is the list." R3-H3 itself identified this exact hazard and mandated the persistent caveat — round 4 left it as a recorded principle, never bound.

**Instance (ii) — comparison clean-by-contrast (R2-H3), un-bound by a POSITIVE assertion.** Comparison (build-order #2, `proposal.md:98-107`) renders allergen cells. Spec §4.2 (`:107`) requires the no-data cell render the marker "NEVER blank/'—'" and every cell carry the bound. But the only ship-gating clause naming comparison is **#5(c) — a NEGATIVE** ("no surface renders a *clean state* from empty"). A literal #5(c) (assert no cell contains "free"/"none"/"safe") **passes a "—" cell** — a dash contains no clean word. "—" beside "declared to contain nuts" reads **nut-free by contrast** at the highest-stakes pair surface. The detail floor got a precise POSITIVE artifact (#5a: empty⇒"info not provided" text); the comparison no-data cell did **not** — asymmetric guardrail rigor. Comparison is **not** under #7 (card-only weld), so nothing else catches it. (If comparison is deferred behind `…_COMPARISON` and *out* of v1, instance (ii) is deferred too — but the brief and `proposal.md:98` place comparison in v1 build order; resolve the ambiguity, see R5-L1.)

**Why HIGH, not MEDIUM:** the harm is demonstrable and **permitted by the design's own ship contract**, not merely by hypothetical implementer error — the resolution enumerates {#1,#4,#5,#6,#7} as *the* flip gate and treats that set as the safety contract (R2-H4). A safety requirement outside that set is, by the design's own rule, ship-optional. Both instances are anaphylaxis-grade (exhaustive-read / clean-by-contrast). **Why not CRITICAL:** the spec *does* mandate both protections (prose), the detail floor still shows *something* truthful, and closing it is a pure guardrail-binding edit (no architecture change) — matches the severity class of the prior allergen-honesty findings.
**Violated invariant:** *data-absence/partial-data never reads as a clean or exhaustive allergen state on any surface* (`resolution.md:944,716`); *guardrails as artifacts, flips bound to them* (R2-H4, `resolution.md:993`).
**Evidence:** `resolution.md:962-970,993`; bound principle un-guarded at `resolution.md:652-661` (R3-H3) and `proposal.md:107,152` (comparison + reliance bound); detail floor `MenuPage.tsx:1078`.

## LOW

### R5-L1 · B-ANTIPATTERN · v1 scope incoherence — is COMPARISON in v1 or deferred?
The brief's v1 = "taste on card + allergen detail-floor + **comparison**(...)". The resolution defers comparison behind `…_COMPARISON` (door-check row 4: "deferred `…_COMPARISON`; spec", `resolution.md:897`) yet names it in a SHIP-gating guard (#5c). If comparison ships in v1, R5-H1(ii) is **live at v1**; if deferred, it is a deferred-surface concern. The flip-binding set {#1,#4,#5,#6,#7} does not list a `…_COMPARISON`-specific gate. Resolve explicitly which surfaces flip at v1 and bind each surface's allergen guard accordingly.
**Evidence:** `proposal.md:98-107,201`; `resolution.md:897`.

## Cleared this round (round-4 fixes that DID hold)
- **R4-H1 contrast door — CLOSED (no relocation).** Card returns to today's uniform-silence baseline; A and N cards identical; later card unit welds warning+marker (#7). The structural asymmetry the door requires is unbuildable. Genuinely the first close-without-relocate. Confirmed via paired walk.
- **R4-M1 omnipresent card caveat — DISSOLVED.** v1 card carries no allergen element ⇒ no omnipresent card note; the bound rides only where allergen content renders (detail; later card unit + comparison). The noise/signal inconsistency on the card is gone. (It re-surfaces *only* as the comparison/partial binding gap R5-H1, a different axis.)
- **#7 authorability + flip-binding — HOLD.** Concrete, deterministic, ratcheted; flip bound to {#1,#4,#5,#6,#7}.
- **Detail-floor wording + #5(a)/(b) — HOLD.** Honest "info not provided"; unconditional; #5(a) is a precise positive artifact.

## Net verdict (RE-ATTACK round 5)
**NOT CONVERGED — but the loop's recurring door IS closed.** The contrast/relocation door that opened in rounds 1→2→3→4 is **genuinely shut** by round 4 (remove-not-relocate + the all-or-nothing weld) — that specific failure pattern has converged and I will not invent a relocation finding to continue it. **The 5th door is a different class:** a **guardrail-binding completeness hole** — two spec-mandated allergen protections (R3-H3 partial-incompleteness bound; R2-H3 comparison explicit-marker) sit outside the round-4 ship-gating guardrail set {#1,#4-positive,#5,#6,#7}, so the design's own flip contract green-lights anaphylaxis-grade builds (partial-list-as-exhaustive; comparison "—" clean-by-contrast). **Open: 1 HIGH (R5-H1, two instances), 1 LOW (R5-L1).** Convergence gate (0 CRIT/HIGH) **not met**. Shape of the close (architect's call): add the two missing assertions to the bound set — **#5 extended**: (d) detail/comparison non-empty allergen surface MUST render the "not a complete list — confirm with venue" bound; (e) comparison no-data cell MUST render the explicit "allergen info not provided" text (positive, not the #5c negative) — and bind the flip to them. No architecture change; pure guardrail edit. Re-confirm after.

---

# RE-ATTACK — FULL BUILD round (ADR-0014, 2026-06-30)

**Target:** `proposal.md` (FULL BUILD) + `docs/adr/0014-menu-characteristics-model.md`, grounded @ HEAD.
**Verified against:** `apps/web/src/pages/client/MenuPage.tsx`, `packages/ui/src/lib/characteristics.ts`, `packages/ui/src/lib/__tests__/characteristics.test.ts`, `packages/db/migrations/1780338982010_menu_modifiers.ts`, `read_public_menu` (mig `1790000000033_localize-modifiers.ts:59-117`).
**Verdict:** NOT CONVERGED. **2 CRITICAL · 2 HIGH · 4 MED · 1 LOW.** The hardest break is a *present-tense production* allergen false-negative the design asserts is "deferred / fixed by construction."
**Regression vs round 5:** prior rounds' detail-floor convergence (#5/#7 weld) still holds at the modal; the new CRITICALs are on the **card pipeline + the live filter**, surfaces round-1..5 treated as not-yet-built. They are built.

## CRITICAL

### FB-C1 · B-SEC / B-CONSIST · The allergen filter the proposal calls "DEFERRED ENTIRELY" is LIVE, unflagged, over a recipe-only denominator — false-negative safety read.
Proposal §8 / ADR step 3 / R-6 / guardrail #1 assert the allergen filter "stays **DEFERRED**", is "**FIXED by construction**", and that "guardrail #1 forbids any dish-removing allergen predicate." All three are false against shipped code. `MenuPage.tsx:193-194`:
```js
if (filterAllergen) {
  result = result.filter(p => bomToNutrition(p).allergens.includes(filterAllergen));
}
```
A dish-removing allergen predicate — **no `VITE_*` gate**, persisted to `localStorage` (`:145-166`), visible chip UI (`:680-696`). Guardrail #1 (`no-hide-allergen`/`dish-removing`) **does not exist in tree** (grep → zero hits; only `characteristics.test.ts` #2/#5/#6/#8 exist).
**Break.** The predicate reads `bomToNutrition(p).allergens` — the **recipe (bom) set only** — never `computeAllergenSurface`, so it ignores owner `declared_allergens`. A dish with `allergen_status:'listed', declared_allergens:['milk']` and no bom line returns `allergens:[]`; with `filterAllergen='milk'` that real-milk dish is **dropped from the result**. A milk-allergic user locating-and-avoiding milk is shown a set that *excludes a real milk dish* — partial declaration read as exhaustive. (Polarity trap too: tapping "milk" *shows* milk dishes, but the chip reads as a generic allergen toggle.)
**Violated red-line.** Allergen presence-only / absence-never; "no dish-removing allergen predicate" (#1, claimed-in-force, not built); §12 duty-stays-with-owner. The surface's central safety claim is contradicted by prod code.

### FB-C2 · B-CONSIST / B-SEC · Card vs modal compute the allergen surface from DIFFERENT functions; quick-add bypasses both.
Card pipeline feeds `allergens: nutrition.allergens` (`:854`) = bomToNutrition, **recipe-only**. Modal uses `computeAllergenSurface(attributes, bomAllergens)` (`:1118`) = **declared ∪ recipe**. Two derivations of one safety fact ⇒ the FULL build's "one pure derivation, surfaces cannot disagree" (§1/§7) is already false at the card.
**Break.** Owner declares `peanuts` (`listed`, no peanut bom line). Modal: "Declared to contain: peanuts." Card `allergens` prop = `[]` → no peanut signal. **Quick-add** (`:865`): `if (!product.modifier_groups?.length)` adds to cart with `options:{}` — **no modal, no allergen surface at all**. Scan → quick-add → checkout a peanut dish with zero peanut signal. The proposed `…_CARD_ALLERGEN` unit, wired onto this `allergens` prop, inherits the recipe-only blind spot and defeats its own ALL-OR-NOTHING (#7) upstream.
**Violated red-line.** Single source of truth; presence-not-absence (card reads clean for a declared allergen).

## HIGH

### FB-H1 · B-ANTIPATTERN / B-DATA · Contract B is impossible without re-versioning the 🔴 hot-path the proposal forbids.
`read_public_menu` (SECURITY-DEFINER hot-path, mig `1790000000033:63-66`) emits each modifier as `jsonb_build_object('id',…, 'name',…, 'price_delta', m.price_delta)` — **id/name/price_delta only**, the sole path modifiers reach the client (no modifier endpoint exists). Contract B (§6/§9) recomputes from `base ∪ Σ selected-option deltas`, needing the §5 `kcal/proteinG/fatG/carbsG/allergens` columns **in the client payload** → the only delivery is bumping that `jsonb_build_object`.
**Break.** §5/§9 assert "Hot-path untouched — `read_public_menu` not re-versioned," "served through the same tenant-scoped path," and §2 budgets "+0 endpoints." All three are mutually exclusive with Contract B — which requires re-versioning the 🔴 TOTAL-BLAST-RADIUS fn used to *reject Option B* in §3. Contract B cannot ship as specified; its data prerequisite is understated (not "additive columns" — a hot-path re-version).
**Violated red-line.** "Hot-path `read_public_menu` untouched" (§6 spine LAW); "+0 endpoints"; Option-B rejection rationale.

### FB-H2 · B-SEC · Decoupled descriptive-coverage and allergen-coverage gates let positive badges render while allergens stay suppressed — wholesome-by-omission card.
§8 step 1: descriptive-coverage gate "governs no allergen element"; allergen-coverage gate keyed separately. Nothing couples them.
**Break.** A tenant with broad bom/taste authoring (descriptive gate passes) but near-zero owner allergen declarations (allergen gate dark) shows cards with 2-3 curated positive chips and **no allergen unit**. A curated-positive card with no allergen signal reads "wholesome/safe" — the absence-implied-safe outcome §12 names worst. The two-gate independence is the mechanism.
**Violated red-line.** "'Fewer chips' must never read as safe" (§12); presence-not-absence.

## MED

### FB-M1 · B-CONSIST · "Modifiable" is undecidable from data → Contract A suppresses ALL reassurance on every dish with any modifier group.
`modifiers` (mig `1780338982010`) carry only `name/price_delta/available/sort_order` — no axis-impact signal. "Could change the relevant axis" collapses to "has ≥1 modifier group." A dish whose only modifier is "extra napkins"/"no onion" loses every future descriptive/regulated badge — the at-a-glance layer goes blank on the customizable dishes it targets. Latent behind empty `DESCRIPTIVE_ALLOWLIST` (`characteristics.ts:46`); manifests on populate. Safe direction (suppression), but self-defeating utility.

### FB-M2 · B-CONSIST / B-SEC · `char_hidden` has no second-surface application and ships raw in the public payload.
§4 subtracts `char_hidden` "at render" via `deriveCharacteristics`, but `compareDishes`/`selectDescriptiveLabels` (`characteristics.ts:51,143`) take **no `char_hidden` param** → compare-view and filter show labels the card hides. Guardrail #9 does not exist (grep `char_hidden` → zero hits). `attributes` rides the `jsonb` passthrough through `read_public_menu` → `char_hidden` ships raw; "hide" is client-side cosmetic, not authoritative, and any surface that forgets the subtraction resurfaces the true label.
**Violated.** "Surfaces cannot disagree"; hide presented as authoritative but unenforced server-side.

### FB-M3 · B-DATA · Sort-by-protein/energy over a half-empty bom denominator buries "unknown" in the "zero" band.
`bomToNutrition` returns `protein:0` for no-bom dishes (`:100`). A `sort-by-protein` lens cannot distinguish "0 g (known)" from "no bom (unknown)" — both numeric 0. On a partially-authored tenant (~half no bom), no-bom dishes are **mis-ranked as low-protein**, not dropped. §8's "no-data dish shows 'no data', never dropped" does not survive a numeric sort — absence read as a measured value.

### FB-M4 · B-FAIL (UX) · Compare long-press collides with the card tap-to-modal, iOS native long-press, and scroll.
Card binds `onClick → handleProductClick` (modal, `:859`) and renders an image, on a 2-col mobile grid (`:827`). R-8 adds a visible affordance for a11y but not gesture collision: a long-press on the card image fires iOS context-menu/image-save/text-select and competes with scroll-hold; on release `onClick` may also open the modal. The gesture is unreliable on its target surface.

## LOW

### FB-L1 · B-DATA · §5 migration idempotency asserted but unspecified; down() convention empty.
Existing modifier migration `down()` is `{}` (forward-only). New additive `ADD COLUMN` needs `IF NOT EXISTS` for the asserted idempotency; re-asserting FORCE on `modifiers` is belt-and-suspenders (ADD COLUMN doesn't drop RLS). Low blast radius alone; the real coupling is FB-H1 (columns inert without the hot-path bump).

## Net verdict (RE-ATTACK FULL BUILD)
**NOT CONVERGED.** Convergence gate (0 CRIT/HIGH) not met: **2 CRITICAL + 2 HIGH.** The decisive shift vs rounds 1-5: those rounds reasoned about *un-built* surfaces; this round grounds two CRITICALs in **production code** — the live recipe-only allergen filter (`MenuPage.tsx:193-194`, contradicting "deferred/fixed") and the card↔modal allergen-derivation split with a quick-add bypass. FB-H1 is an internal contradiction (Contract B vs hot-path-untouched). Guardrails #1 and #9 that the design leans on **do not exist in tree** at this writing. Architect owns the shape of the close.
