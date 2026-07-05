# Counsel Opinion — Menu Characteristics Model

> COUNSEL seat, Triadic Council. Advisory (non-blocking) except a grounded ETHICAL-STOP.
> Pairs with `proposal.md` (ARCHITECT) + `docs/adr/ADR-menu-characteristics-model.md`.
> Verdict line at the bottom. The human decides.

## Verdict (one line)

**ONE ETHICAL-STOP** — the **allergen filter** as specified manufactures a *perceived* absence
claim over **known-incomplete data** for the highest-stakes user, and (uniquely among the lenses)
omits the "no data" marker exactly where life depends on it. It is a **STOP-as-friction, not a
veto**: a precise, cheap remedy lifts it. Everything else holds the grounded lines; I record two
strong binding conditions (L2 regulated-claim authority; static-badge-under-customization) and the
usual non-blocking advice.

---

## 1. Reasoning by lens (load-bearing only)

### CARE / SAFETY — the allergen filter over known-incomplete data (the sharpest)

The data layer is honest by construction, and I affirm it: absence is never derived, `hide-allergen`
hides **only** on *confirmed presence*, unknown stays visible, no "free of" label exists. The
false-negative that kills someone ("this dish is safe" when it isn't) is **structurally impossible at
the data layer.** That is real and good.

But the red line is **allergen-safety / never-imply-absence**, and a filter is not a data structure —
it is a *promise about a transformation*. Walk the highest-stakes path against the actual data:

1. A scraped `place` product has its allergens write-stripped at acquisition and `bom`-stripped by the
   read-gate until the owner confirms. For these — likely a **majority** at launch — the layer sees
   *nothing*.
2. The allergic customer taps **"hide dishes containing nuts."** This is a positive action verb. It
   promises: *the nut dishes are now gone.*
3. What actually happens: only the **confirmed-nut** dishes leave. Every **unknown-nut** dish stays —
   and here is the crossing — §5.2 makes the allergen lens the **one lens that does NOT mark "no
   data."** Other lenses show no-data dishes *as* "no data"; the allergen lens silently mixes the
   unchecked dishes into the visible set, **indistinguishable from genuinely-checked-clear dishes.**

So the lens that most needs the "no data" marker is the only one that omits it. The remaining set,
after an explicit hide-action, is read by the person whose life depends on it as *"these passed the
filter = these are safe."* The data never claimed absence — but the **interaction** manufactures a
perceived-absence claim the data cannot back, for the exact population where coverage is weakest. That
is the allergen-safety line crossed at the **perception layer**, even though the data layer is clean.

Test against the deliver-v2 precedent and it is decisive: there, the dangerous direction (cash) got
**protected friction** (alert), never a silent record — *"the constrained person must see the truth at
the moment of exposure."* The allergic person at the moment of filtering is precisely that constrained
person at maximal exposure. The honest design therefore *requires* friction here, not silence.

**This is the STOP** (§2). The remedy is cheap and lifts it — so it is friction, not a veto.

### HONESTY / AUTHORITY — L2 "light" inverts the spine (strong, STOP-adjacent)

The spine is *owner = authority, system gives facts not a verdict.* L3 honors it (owner declares; the
platform is conduit). **L2 inverts it.** "320 kcal" is a fact. **"light" is a categorization against a
threshold — a judgment the platform makes about the owner's dish, shown under the owner's menu, which
the owner never reviewed.** And "light" is not a casual word: it is a **legally-defined nutrition claim**
(EU 1924/2006 + Albania). The architect's R-3 gate is good but answers the wrong question — it verifies
*"is the threshold correct,"* not *"who is allowed to make this claim."* Under food law the nutrition-
claim duty sits with the **food business operator (the owner)**, not a conduit. Shipping derived
regulated claims at scale on behalf of tenants who never saw them is the platform **becoming the
asserter** of a claim the spine says only the owner may make. The `char_hidden` opt-out is the wrong
asymmetry for a *regulated* claim: default-asserted with owner-suppress is opt-out; a legal claim wants
owner **authority** (opt-in / confirm), not opt-out.

This stops short of a STOP only because the layer is flag-off, R-3 already gates thresholds, and
body-effect phrasing is denylisted — so harm cannot ship by accident. But it is the second-sharpest
edge → **Binding condition BC-1.** Split the vocabulary: **descriptive** terms (hearty, rich, protein-
forward as *relative* description) are facts-rendered and fine platform-side; the **regulated subset**
(light, low-energy, "source of protein") must not be **platform-asserted** without owner authority.

### HONESTY — the dropped modifier recompute (the static badge under customization)

Dropping nutrition/allergen recompute is **correct** (the data does not exist; a false "still light"
is worse than none). The honesty hole is the residual: a "light" badge survives a heavy add-on at the
**moment of customization — exactly when the constrained customer is most exposed.** The design's
mitigation has a soft seam: §7 says annotate "for the base dish" **(or** suppress the macro chips**)**.
That "(or)" is where a clean rule went optional. A *caption* on a regulated/diet claim is the
**soft-confirm-as-trap** analogue — it is read past by the very low-literacy/anxious user it is meant
to protect. For the future diet track this is acute: a "vegan" frame on a dish you can add a non-vegan
modifier to is not a calorie matter — vegan/halal is ethical/religious, and a caption-that-still-claims
is a dignity wound. → **Binding condition BC-2:** on products with modifier groups, the regulated/diet
subset must **suppress**, not annotate. Caption is acceptable only for non-regulated descriptive chips.

### JUSTICE — who it falls on, and the "one tap away" defense

The "exact figures are one tap away" defense **does not hold for the person who most needs the
warning.** The badge is the glance layer; the detail is the navigate layer; the person who trusts the
glance is *by definition* the one who will not navigate — the low-literacy, the anxious, the elderly,
the allergic-in-a-hurry. This is the same justice geometry as deliver-v2 C2 (the powerless cannot reach
the evidence). The consequence is a design rule, not just a worry: **safety-critical truth (allergen
coverage, regulated-claim caveats) must live at the glance layer, never be deferred to the tap.** This
grounds both the STOP and BC-1/BC-2.

### REGULATED CLAIMS / STRATEGY

R-3 (every threshold cites a verified per-market anchor; un-anchored = CI-red) is the right gate and
genuinely well-made. The residual the gate does not touch: a *legally-anchored* "light" is still a
**claim made by the platform, at scale, about a tenant's food, unreviewed by the operator who legally
owns the liability.** The anchor makes the claim *true*; it does not make the platform *entitled* to
make it. That is the BC-1 authority gap, restated in legal terms.

### AESTHETICS / COHERENCE (affirm)

"Schema rich, runtime minimal" is honored cleanly: zero migrations, a pure deterministic
`deriveCharacteristics`, comparison/filter as *views* over one source so they cannot disagree. §0 —
leading with *"the patch's framing is partially false"* — is exemplary anti-motivated-reasoning; the
architect grounded against source and named what breaks ("zero owner work" is false for L1/L3-diet)
rather than selling the patch. The "no global winner, directional arrows only" comparison is the
facts-not-verdict spine rendered as UI. This is good work; the STOP is about one UX seam, not the
architecture.

---

## 2. ETHICAL-STOP

**STOP-1 — The allergen filter must not ship without coverage-honesty friction at the glance layer.**

- **Grounded line crossed:** allergen-safety / never-imply-absence — at the *perception* layer. The
  filter's action-verb framing + the §5.2 omission of a "no data" marker on the allergen lens (the one
  lens that omits it) manufactures a perceived-absence claim over known-incomplete data (write-stripped
  / unconfirmed `place` products) for the highest-stakes user. The data never claims absence; the
  *interaction* does.
- **Why it is a STOP and not a binding condition:** every other gap in this design is *omission* (under-
  informative). This one is *commission* — the UI performs an action that asserts a safety
  transformation the data cannot back, to the person whose life depends on it. That is the allergen red
  line's actual purpose, not its letter.
- **The remedy that LIFTS it (this is friction, not a veto):**
  1. On the allergen lens, **mark unknown-data dishes visibly** ("allergen info not provided — confirm
     with the venue") — the same "no data" honesty §5.2 already gives every *other* lens; remove the
     allergen-lens exception.
  2. A **non-dismissible coverage disclosure** when the allergen filter is active: *"This hides only
     dishes we know contain X. Dishes without allergen data are still shown. Always confirm with the
     venue."* Friction proportional to stakes — the deliver-v2 cash→alert pattern, pointed at allergens.
  3. State the **coverage denominator honestly** (it is a minority at launch) so the remaining set is
     never read as a vetted-safe set.

With (1)–(3) the filter is honest and the STOP lifts. The human records the decision to ship the
filter with this friction (or to defer the allergen filter until owner allergen-confirmation coverage
is materially complete). **Pause the allergen-filter flag (`…_FILTER`) until one of those is recorded.**
The L1/L2/allergen-**presence** chips and the comparison view do **not** depend on this and are clear
to proceed.

---

## 3. Binding conditions (human records a decision; not vetoes)

- **BC-1 — Regulated L2 claims need owner authority, not just a correct threshold.** Split
  `CHARACTERISTIC_RULES` into **descriptive** (platform-renderable: hearty, rich, protein-forward as
  relative description) and **regulated** (light / low-energy / source-of-protein). The regulated subset
  must not be **platform-asserted** without owner authority — owner opt-in/confirm, *not* the
  opt-out `char_hidden`. R-3 verifies the number; this verifies the *entitlement to claim*.
- **BC-2 — On modifiable products, suppress (don't annotate) the regulated/diet subset.** Close the
  §7 "(or)" seam: a caption on a regulated/diet claim is a soft-confirm trap read past by the user it
  protects. Descriptive chips may keep the "base dish" caption; regulated/diet chips suppress.

## 4. Non-blocking advice (aesthetic / strategic)

- Recover the rejected Option A *for the regulated subset only* (see steel-man): if "light" et al ever
  ship, derive them **server-authoritative** so the claim is a recorded, audited, owner-tied fact, not
  an ephemeral client render. Descriptive labels stay client-edge — that split is the honest one.
- The "no data" marker (STOP remedy #1) should be the default *everywhere*, not a per-lens decision —
  partial coverage is the whole layer's defining condition; make honesty about it a first-class UI state.
- Strategically: name in the ADR that L2 coverage correlates with **data provenance** (owner-built
  recipes have `bom`; scraped/imported dishes do not), so a future reader does not mistake "fewer chips"
  for "less healthy dish" (see Q).

## 5. Steel-man of a rejected option (≥1)

**Option A — derive inside `read_public_menu` (server, single source) — rejected for SHIP, rightly for
the descriptive labels, but its kernel is exactly what the regulated subset needs.** The architect
rejected it on hot-path blast-radius + localization cost — both real. But strip those away and Option A
offered the one thing the chosen client-edge design *cannot*: for a **regulated nutrition claim**, the
project's own spine is **server-authoritative**. A server-side derivation makes the "light" claim a
**recorded, versioned, auditable server fact tied to the owner's confirmed data** — you can answer
*"what did we claim, about which dish, to whom, on what data, under which threshold version"*. The
client-edge function cannot: the claim is computed and shown with no server record that it was ever
made. For a *descriptive* chip that auditability is over-engineering (Option C territory). For a
*regulated* claim shipped at scale on a tenant's behalf, the absence of a server record of the claim is
a genuine liability and honesty gap. So the honest synthesis is **not** "client-edge for everything" —
it is **client-edge for descriptive, recover Option A for the regulated subset.** Rejecting Option A
wholesale trades away the audit trail of exactly the claims that carry legal weight.

## 6. The question nobody asked

**The layer creates customer reliance that it explicitly disclaims responsibility for — and nobody
asked where the safety duty LANDS when the honest-by-construction layer meets the dishonest-by-omission
data.** §1 declares the layer is "not a nutrition database, not a health/medical claim engine." Yet the
moment an allergic customer uses the filter or a diabetic trusts "light," they *rely* on it — and that
reliance silently migrates the de-facto safety duty **from the owner (who legally holds it and never
entered the data) toward the platform (which built the filter and disclaims the duty).** When a
low-coverage allergen filter contributes to a reaction, the duty has been transferred by the UI to the
party that refuses it, over data the party that holds it never supplied. Nobody mapped that liability
landing.

A second, quieter one rides with it: because L2 coverage tracks **data provenance** (owner-built `bom`
vs scraped `place`), the *presence or absence of chips* becomes a silent signal that correlates with
how the owner onboarded, not with the dish. A health-conscious customer who avoids un-chipped dishes —
or trusts chipped ones — is unknowingly sorting by acquisition path. Partial coverage is not neutral;
it invisibly rewards hand-built menus and penalizes imported ones, in the customer's eye, on a health
axis. Worth asking before launch: *does the coverage gap itself bias the customer, and against whom?*

---

**Final verdict: ETHICAL-STOP (STOP-1: allergen-filter coverage-honesty) — a friction, not a veto;
the cheap remedy lifts it. The rest of the layer (L1 taste, L2 descriptive chips, allergen-presence
chips, comparison) is clear on the values axis, subject to BC-1 and BC-2. The human records the
allergen-filter decision before the `…_FILTER` flag flips.**

---

## RE-EXAMINE round 2 (post-fix — re-grounded against resolution.md RESOLVE round + updated proposal.md)

> COUNSEL seat. Reading the RESOLVE-round dispositions, the reduced SHIP scope, and the five guardrails
> to author. Confirming whether STOP-1 / BC-1 / BC-2 / the carried duty principle landed, and whether
> the de-scoping opened anything new. A STOP is only a grounded crossing.

### Q1 — STOP-1 discharged? **YES for the ship path; the commission is gone.**

The architect chose the stronger of the two paths I offered: not ship-with-friction but **defer the
allergen filter entirely**, with a permanent red-on-disk guardrail (#1) forbidding any hide/only-safe
allergen predicate, and the "info not provided" marker promoted to a first-class default UI state (#5,
no-clean-from-empty). My STOP was specifically the *commission* — an action-verb interaction asserting a
safety transform the data cannot back. With no filter and no hide/safe predicate able to ship by code,
**that commission cannot occur.** The grounded crossing is cleared for SHIP. Re-enable is correctly a
recorded NEEDS-HUMAN gate (positive-only + markers + coverage disclosure + honest denominator +
human-set threshold). Discharged.

**Residual care concern in the SHIPPING surfaces (non-STOP, advisory).** The marker fires on
*empty/absent* `bom[].allergens`. It does **not** fire on a *non-empty-but-partial* declaration — a dish
where the owner authored a `bom` listing milk on one line and simply never characterized the rest. That
dish shows "contains: milk", no marker, and is **indistinguishable from a dish whose allergens were
fully reviewed.** This is the within-dish completeness gap: presence-only honesty can certify *what is
declared present*, never *that the absence of a chip means reviewed-and-clear for that allergen*. It is
the floor of any presence-declaration system, the duty stays with the owner, and it is omission not
commission — so **not a STOP.** But it carries the same scarcity-as-signal residue I flagged at M2, now
*inside* a single card. Recommendation (non-blocking): frame the presence row as **"declared to
contain"** with the ambient "always confirm with the venue" reliance-bound, so a sparse chip list never
reads as a complete allergen profile. The reliance-bounding language that the resolution attached to the
(deferred) filter disclosure should ride the **presence layer** too — in SHIP scope the presence chips +
marker are the *only* allergen surface, so that is where the "confirm with venue" framing must live.

### Q2 — BC-1 landed? **YES, structurally; owner=authority is restored for the regulated layer.**

The descriptive/regulated split is real, the regulated subset is OFF behind all three locks (honesty-
anchor + owner-authority opt-in + verified-anchor table), and guardrail #2 is red-on-disk until the
anchor table exists. The platform no longer asserts a regulated claim on a tenant's behalf — the FBO
asserts or it does not ship. "Facts-not-verdict / owner=authority" is restored for the regulated layer.

**Is the descriptive-vs-regulated split honest, or does a descriptive label quietly assert a regulated
claim? Two seams remain (non-blocking, BC-1-flavored refinements):**

1. **"filling" / "hearty" verges on a body-effect (satiety) claim.** The body-effect denylist (guardrail
   #3) already bans "keeps you full." "Filling" is a near-synonym of that banned phrase wearing a
   descriptive coat. "Hearty" / "generous portion" is a clean dish-description; "filling" asserts an
   effect on the eater's body. Recommend the descriptive table drop "filling" in favor of a
   portion/intensity word, and that the denylist test explicitly cover the satiety register
   ("filling", "fills you up", "satisfying" if used as a hunger claim).

2. **The §2 enumeration still mixes regulated terms into the casual label list.** `proposal.md:54` lists
   the L2 vocabulary as "*light, hearty/filling, protein-rich, carb-forward, rich, light-bite*" — that
   line was **not** updated to the split. "light" and "light-bite" are the regulated terms that are
   gated OFF; "protein-rich" reads as the regulated "high in protein / source of protein," not as the
   sanctioned relative descriptor "protein-forward." This is doc-level enumeration drift, not the rule
   table, and the gate (#2) would still block a regulated emission — but it is exactly the "descriptive
   label quietly asserting a regulated claim" risk the question probes, and it should be corrected so the
   *rendered* descriptive chip can never surface as "protein-rich"/"light-bite". Tighten the descriptive
   renderings and align §2 with §3.4. Not a STOP (the gate + denylist + flag-off mean nothing regulated
   ships by accident), but a real honesty refinement.

### Q3 — BC-2 landed AND safe? **LANDED for reassurance — but the WORDING introduces an ambiguity that, under a live reading, would suppress an allergen WARNING. This is a grounded crossing → residual ETHICAL-STOP-2.**

The intent is correct and safe: a reassuring/regulated/diet label (light, vegan) on a modifiable dish is
**suppressed, not captioned** — closing the soft-confirm "(or)" seam. For *reassurance*, suppression is
the protective choice (a modifier could break the reassurance, and modifiers carry no nutrition/allergen
deltas — R-1 — so an honest post-modifier recompute is impossible). Good.

**But the rule and guardrail #4 are written as "regulated / diet / allergen-sensitive label is
SUPPRESSED" (resolution §H1/BC-2; proposal §7; guardrail #4: "emits no regulated/diet/allergen-sensitive
label").** The phrase **"allergen-sensitive label" is ambiguous** and one of its readings is dangerous:

- *Safe reading:* it means reassuring allergen claims ("free-from", "doesn't contain X") — which do not
  exist in SHIP scope anyway. Suppress those: fine.
- *Dangerous reading:* it includes the **allergen-presence WARNING chip ("contains nuts")**. A developer
  implementing guardrail #4 literally — "a product with axis-changing modifier groups emits no
  allergen-sensitive label" — would **suppress the 'contains nuts' warning on every modifiable dish.**

That is a **care crossing of a grounded red line** (allergen-safety / the constrained person must see the
truth at the moment of exposure). And it is uniquely indefensible here: because modifiers carry **no
allergen data** (R-1), a modifier can only ever *add* allergens, never honestly *remove* a base-dish
allergen. Therefore a base-dish allergen **warning is always conservatively true** and must **never** be
suppressed for modifiability — suppressing it manufactures absence-by-omission for the highest-stakes
user, which is the exact harm STOP-1 was about, re-entering through the BC-2 fix's wording.

This is the crossing the fix **accidentally risks introducing**. It is cheap to close (one carve-out
sentence + scoping the guardrail), so it is **friction, not a veto** — but as written, the guardrail
could *encode* the dangerous behavior, so I record it as a residual STOP, not merely advice.

**Remedy that lifts STOP-2:** state explicitly, in the rule and in guardrail #4, that suppression applies
**only to reassuring claims** (regulated nutrition + diet + any "free-from"/reassurance), and that
**allergen-presence WARNINGS are NEVER suppressed** — a modifiable dish still shows "contains X" (a
modifier can only add allergens, never remove them, since modifiers carry no allergen deltas). Guardrail
#4 must assert the positive complement too: *a modifiable dish that has a base-dish allergen still
renders its presence chip.* With that carve-out recorded, BC-2 is both landed and safe.

### Q4 — "Where the safety duty lands" — addressed? **YES, now structurally honored by the ship scope, not aspirational.**

In round 1 the duty-migration risk was live *because* the high-reliance surfaces (the filter, the
platform-asserted "light") were in scope — reliance is what migrates the de-facto duty owner→platform.
The de-scoping removes exactly those surfaces: no filter, regulated claims gated to owner-authority, and
the only allergen surface is presence chips + "info not provided" marker that visibly point the duty back
at the owner ("ask the restaurant"). The principle is recorded as a carried ADR principle (§12) so it
cannot be re-crossed quietly, with the secondary provenance-bias caveat also recorded. The mechanisms
(positive-only view, markers, owner-gating, coverage disclosure, server-authoritative regulated supplies)
are now *load-bearing structure of the ship scope*, not promises. Honored — subject to the Q1 note that
the reliance-bounding "confirm with venue" language must actually attach to the presence layer in SHIP,
not only to the deferred filter's disclosure.

### Q5 — New value/honesty/care concern opened by the de-scoping? **YES — a strategic/honesty concern (non-blocking): the SHIP feature is near-invisible at launch coverage, and the rare chip is over-trusted.**

The de-scoping is correct on safety, but it sharpens a value question. Grounded against the live
denominator: demo allergen coverage is **0/49**, `bom` exists only on owner-built recipe dishes
(scraped/`place` products have none), and descriptive L2 *also* requires `bom`. So in SHIP scope, L2
descriptive + allergen-presence render for **almost nothing** at launch; the realistically-visible
surface is **taste** (owner-entered, itself optional/sparse) plus comparison/sort over near-empty macros.

Two honesty effects of shipping a mostly-invisible layer:

- **Rare-chip over-trust.** When 2 of 49 cards carry a chip and 47 are blank, the chipped cards gain
  disproportionate salience and read as "specially characterized / better," when the chip's presence
  tracks **data provenance** (hand-built vs scraped), not dish merit. This is the M2 scarcity-as-signal
  and the §6 "question nobody asked" provenance-bias — now the *dominant* visible behavior of the layer
  rather than an edge case. It is recorded in §12, which is right.
- **Is shipping it honest?** Degrade-to-silence is honest in the *narrow* sense (no false claim is made).
  But shipping a characteristics layer that is invisible on the overwhelming majority of cards, whose
  main observable effect is to make a handful of provenance-privileged dishes stand out, is questionable
  *value-to-noise* against the launch trigger (first real paid order). It is closer to polish than to the
  trigger.

This is **not a STOP** (no false claim; silence is honest). It is a strategic/aesthetic recommendation:
(a) consider gating the layer's *visibility* on a per-tenant minimum coverage so a 2-chip menu does not
turn chips into an unintended "premium" signal; or (b) **ship taste-first** (the one field carrying
genuine owner intent and the most coverage) and hold descriptive-L2 + presence chips until `bom` coverage
is materially real — letting the layer earn its visibility rather than debut as mostly-blank-with-a-few-
provenance-flags.

### RE-EXAMINE round 2 — bottom line

- **Residual ETHICAL-STOP(s):**
  - **STOP-2 (NEW, friction not veto)** — the BC-2 suppress-on-modifier rule + guardrail #4 use the
    ambiguous phrase "allergen-sensitive label"; under its live reading it would **suppress allergen-
    presence WARNINGS on modifiable dishes** (a care crossing, since modifiers carry no allergen deltas
    and can only add allergens). **Lifts on a recorded carve-out:** suppression is for *reassuring* claims
    only; allergen-presence warnings are **never** suppressed, and guardrail #4 must positively assert
    that a modifiable dish with a base allergen still renders its presence chip.
  - **STOP-1 — DISCHARGED** for SHIP (filter deferred + red-on-disk no-hide guardrail + first-class
    marker). Re-enable remains a correctly-recorded NEEDS-HUMAN gate.
- **Binding conditions hold as recorded rule:** **YES.** BC-1 landed (descriptive/regulated split + owner-
  authority + verified-anchor + guardrail #2); BC-2 landed *for reassurance* and becomes fully safe once
  STOP-2's carve-out is recorded. The NEEDS-HUMAN gates (coverage threshold; verified legal anchors;
  Albanian regulated-term legal review) are recorded as rule, not aspiration.
- **Non-blocking (do not gate ship):** (1) "declared to contain" framing + "confirm with venue" on the
  presence layer (Q1 within-dish gap); (2) drop "filling", fix §2 enumeration so "light/light-bite/
  protein-rich" cannot surface as descriptive renderings (Q2); (3) per-tenant coverage floor for layer
  visibility, or ship taste-first (Q5).

---

## RE-EXAMINE round 3 — FULL BUILD (re-grounded against proposal.md FULL BUILD + ADR-0014 draft)

> COUNSEL seat. The FULL build un-defers the surfaces v1 held back — populated L2 card badges, the
> card allergen unit, two-dish compare, the characteristic filter, and live modifier recompute. I
> re-walk only what the FULL build NEWLY introduces, and confirm whether the round-1/2 STOPs stayed
> discharged through the expansion. A STOP is still only a grounded crossing; **armed-on-flip** where
> the design gates but a human act could still cross.

### Carried STOPs — did the expansion re-open them? **No regressions; both stayed discharged.**

- **STOP-1 (allergen-filter commission)** — STILL DISCHARGED. The FULL build keeps the allergen filter
  DEFERRED (`…_ALLERGEN_FILTER`, NEEDS-HUMAN), keeps guardrail #1 as a permanent ratchet forbidding any
  dish-removing allergen predicate, and ships the filter step (#8) with **non-allergen, non-regulated
  lenses only** — a no-data dish shown as "no data", never silently dropped. The commission cannot ship
  by code. Re-enable stays a positive-only, coverage-disclosed, human-thresholded recorded gate.
- **STOP-2 (warning-never-suppressed carve-out)** — STILL DISCHARGED, now load-bearing. Proposal §6
  Contract A states it in the protective form: *"the allergen-presence WARNING is NEVER suppressed…
  suppression is asymmetric: it removes reassurance, never a warning,"* and guardrail #4-positive
  ("warning-never-suppressed + reassuring-suppressed-on-modifiable") gates the card. The ambiguous
  "allergen-sensitive label" phrasing round-2 flagged is gone. This is the single most important
  carry-forward and it held.

### What the FULL build NEWLY introduces — three armed-on-flip ETHICAL-STOPs

> All three are **armed-on-flip standing gates**, not crossings already made — the design gates each.
> They pause the *launch act* of the named flag and require a recorded human decision. Friction
> proportional to harm; none blocks a conscious human or the build.

**FB-STOP-1 — Regulated subset flip without verified-from-text anchors + a named legal sign-off.**
- **Line (if flipped early):** misleading-about-health — "light / low-energy / source of protein" are
  regulated EU 1924/2006 (+ AL transposition) claims; figures reconstructed from memory rather than
  transcribed from the verified regulation text mislead about health at scale, in the platform's voice,
  for tenants who never asserted them.
- **Status:** NOT crossed — gated three ways (R-3 NEEDS-HUMAN; guardrail #2 red-on-disk OFF;
  `REGULATED_ANCHORS` empty + `citation`/`verifiedBy` required; owner opt-in authority; server-
  authoritative supplies prereq R-2). This is round-1 BC-1 fully structuralized — affirmed.
- **Arms on:** any proposal to flip the regulated subset. **Needs a recorded human decision owned by a
  named legal/Counsel signatory** — not a revise-and-proceed. The architecture is right; the *act* of
  asserting a legal health claim on a tenant's behalf is the human-gated part.

**FB-STOP-2 — `…_CARD_ALLERGEN` flip, OR a curation interaction that lets the positive badge-stack read as allergen safety.**
- **Line (if shipped wrong):** false-negative about food safety — the worst outcome in the system. Two
  sub-paths:
  - **(a) Coverage/contrast (already gated):** the card allergen unit stays OFF until real allergen
    coverage + the absolute authored-dish floor exist; ALL-OR-NOTHING (guardrail #7) makes "clean by
    contrast" between cards structurally impossible. Affirmed — honest by construction.
  - **(b) Within-card juxtaposition (the NEW seam the FULL build opens, NOT yet explicitly guarded):**
    the FULL build puts a curated stack of 2–3 *reassuring descriptive* badges on the same card as the
    allergen unit. Each element is individually honest, but a confident positive-badge stack sitting
    beside an allergen unit in its **no-data floor state** can read, to a fast/hungry/anxious eye, as
    "good dish, nothing flagged = safe." Honest atoms, misleading gestalt — the perception-layer
    crossing of the original STOP-1, re-entering through visual hierarchy rather than a filter.
- **Status:** (a) gated; (b) NOT yet covered by a named guardrail. **REVISE:** add a guardrail/visual
  assertion that the allergen unit (warning OR floor marker) is **never visually subordinated to,
  crowded out by, or pushed below the primary card read by the descriptive badge-stack** — the curation
  cap governs descriptive badges only and can never displace or de-emphasize the allergen surface.
  Guardrail #7 covers card-vs-card; this covers **badge-stack-vs-allergen-unit within one card.** A
  recorded human decision gates the `…_CARD_ALLERGEN` flip; this within-card guardrail should be
  red→green before it.

**FB-STOP-3 — `char_hidden` vocabulary not provably disjoint from the allergen / reassurance surface.**
- **Line (if vocabulary leaks):** owner HIDE approaching the hiding of a *warning* — the new owner hide
  feature is the round-1 "hide-not-fabricate asymmetry" given a live control surface; if its validated
  vocabulary ever admitted an allergen token, or a label whose *removal* manufactures a reassuring
  absence, subtractive control becomes warning-suppression, which the spine forbids absolutely.
- **Status:** NOT crossed — design scopes `char_hidden` to the descriptive/taste enum, routes allergen
  through a separate `computeAllergenSurface` with no absence field, and specifies guardrail #9
  (`char_hidden` ⊆ derived vocabulary + subtractive-only). The asymmetry is genuinely strong: a false
  badge has nowhere to live (no `char_added`, no union path). Affirmed.
- **Arms on:** defining `char_hidden`'s allowlist. **REVISE guardrail #9 to assert, as a permanent
  ratchet, that the validated `char_hidden` vocabulary is a strict subset of DESCRIPTIVE/TASTE labels
  and can NEVER contain (i) any allergen token, (ii) any "free-from"/reassurance token, or (iii) any
  label whose removal would manufacture a reassuring absence.** A code/guardrail revision (lower
  friction than FB-STOP-1/2's human-legal sign-off), but airtight before the hide feature ships — the
  boundary is the entire integrity of the asymmetry.

> The **modifier-delta migration** (§5, 🔴 RED-LINE in `packages/db/migrations/`) is **not** a Counsel
> ETHICAL-STOP — it is Breaker/architect territory (forward-only, idempotent, re-assert ENABLE+FORCE
> RLS, integer half-up, staging-before-boot). On the *values* axis it is clean: gated dark behind
> `MODIFIER_NUTRITION_ENABLED`, and Contract B's monotonicity (allergen union never removes a base
> allergen; reassurance suppressed if any option lacks a delta — guardrail #10) is the honest contract.
> One note: Contract B must not ship on *partial* modifier coverage as if complete — the partial-data
> guard already says so; hold it.

### Non-blocking (FULL build — aesthetics · strategy · dignity)

- **Aesthetics — the curation cap is now doing double duty.** 2–3 "only when notable" is both the
  honesty mechanism (a wall of badges reads as completeness/reassurance) AND the visual quality bar.
  Treat it as an invariant, not a tunable; the day someone wants "richer cards," that pressure is the
  same gestalt risk as FB-STOP-2(b). Borderline → silence stays a feature.
- **Strategy — compare/filter is the genuine retention differentiator, and the honesty floor IS the
  growth strategy, not a tax.** "Scan the menu through MY lens" at +0 DB is real moat. The localized
  small dictionary (~35–40 strings, sq/en/uk, hand-translated, no auto-translation) is both a quality
  signal in the Albanian market and a red-line guard — auto-translating an allergen/claim label would be
  a quiet crossing. Hold the hand-translation line.
- **Dignity — facts, not a verdict, for the eater too.** "No global winner, arrows only on price/prep,
  never on kcal" protects the *customer* from a platform telling them what to eat. When someone asks for
  a "wellness score" or a green-leaf "good choice" badge, that is Option C's inversion aimed at the
  customer instead of the owner — refuse it the same way. And §12's "fewer chips ≠ less healthy dish" is
  the dignity line for **data-poor (often smaller/scraped) restaurants** — don't let a provenance gap
  render as a quality verdict against them (Q5 / the visibility concern, restated).

### Steel-man of a rejected option (FULL build — honest strongest case)

**Option B (server-computed + cached) — for the REGULATED subset only.** The proposal rejects B
wholesale, and for descriptive/taste/compare/filter that rejection is correct (re-versioning the 🔴
hot-path `read_public_menu` for a sub-ms pure compute, SQL-localizing thresholds, cache thrash — all
real, all damning). But B carries one argument the proposal underweights, and it bites hardest exactly
where the stakes are highest: **forensic auditability of a regulated claim.** Under Option A
(client-derived), *what a customer actually saw when they relied on "light"* is a function of the
shipped bundle version + flag state + anchor-table version **at view time** — reconstructing "what
regulated claim did we render, for which dish, to whom, on what data, under which threshold version,
when a complaint or regulator arrives" is genuinely hard; the claim is computed and shown with **no
server record it was ever made.** For a feature whose entire liability thesis is "the food-business-
operator owns the claim," being able to prove exactly what the platform rendered on their behalf is not
nothing. The honest synthesis is **not** "A everywhere" — it is **A for descriptive/taste/compare/
filter (right), and recover B's kernel for the regulated subset IF it ever ships**: derive the
regulated claim against owner-confirmed, server-authoritative supplies (already an R-2 prerequisite) and
**emit a server-side audit record of the claim** — a sidecar log, NOT a hot-path `read_public_menu`
re-version, so the architect's blast-radius objection is fully preserved. Rejecting B wholesale trades
away the audit trail of precisely the claims that carry legal weight.

### The question nobody asked (FULL build)

**We have proven honesty to ourselves in code — have we proven it to an actual allergic person's
EYES?** Every FULL-build guardrail enforces honesty at the *data* layer: presence-only, reliance bound
attached, ALL-OR-NOTHING, no-clean-by-contrast, floor unconditional, warning-never-suppressed. But the
entire safety model rests on an **untested human-perception assumption** — that a real at-risk customer
*reads* "allergen info not provided — ask the restaurant" as **"this could contain anything"** rather
than glossing a tidy, badge-rich, confidently-designed card as **"if it were dangerous they'd have said
so."** The FULL build *raises* this stake, because it adds the very thing that makes a card look
confidently complete: a populated badge stack next to the allergen unit (FB-STOP-2b). A clean, badge-rich
UI *feels* safe; that feeling is exactly the false-negative we forbade in the data but never tested in
the human. The missing perspective is the allergy sufferer reading the floor copy under real conditions
— scanning fast, hungry, trusting the brand — **not** the engineer confirming the string renders.
**Before `…_CARD_ALLERGEN` launches, validate the floor + reliance-bound copy AND the badge-stack-beside-
allergen-unit gestalt with actual allergy-affected users, not only a guardrail assertion that the text
is present.** Honest data meeting a misreading human still hospitalizes someone — and the red→green proof
cannot see it. This is the one gap no amount of code discipline closes; it needs a human in a usability
seat, once, before the allergen card ships.

### RE-EXAMINE round 3 — bottom line

- **ETHICAL-STOPs (armed-on-flip, friction not veto):**
  - **FB-STOP-1** — regulated subset flip needs verified-from-text anchors + a named legal/Counsel
    sign-off (recorded human decision; the architecture already gates it — the *act* is human-gated).
  - **FB-STOP-2** — `…_CARD_ALLERGEN` flip needs a recorded human decision AND a NEW within-card
    guardrail: the allergen unit (warning/floor) is never displaced or de-emphasized by the descriptive
    badge-stack (honest atoms must not compose into a misleading "safe" gestalt). (a) gated; (b) REVISE
    — author it.
  - **FB-STOP-3** — `char_hidden` allowlist must be provably disjoint from allergen/reassurance tokens;
    REVISE guardrail #9 to a permanent ratchet before the hide feature ships.
  - **Carried STOP-1 / STOP-2 — STILL DISCHARGED** through the FULL-build expansion (filter deferred +
    #1; warning-never-suppressed + #4-positive). No regression.
- **Steel-man:** recover Option B's kernel — a server-side audit record of the *claim* — for the
  regulated subset only, if it ever ships (sidecar log, not a hot-path re-version).
- **The one question:** validate the allergen floor copy + the badge-stack gestalt with real
  allergy-affected users before the allergen card launches — code proves data-honesty, not perception.
