# Storefront Polish — Awwwards & Best-in-Class Reference Research

> Goal: make dowiz/DeliveryOS look "designed by a creative agency" — trustworthy, professional,
> consistent. This is **real research**, not invention: every reference below is a live site pulled
> from awwwards' Food & Drink category (or a well-known food/nutrition brand for the nutrition
> feature). Scores/awards are recorded only where verified from the awwwards page; where a numeric
> score is not published (nominees), that is stated. Screenshots were captured with a real headless
> Chromium (agent-browser CLI) and saved to `./refs/`.
>
> Research date: 2026-07-03. Source pool: `https://www.awwwards.com/websites/food-drink/`
> (3,744 sites in the category at capture time).

---

## 0. TL;DR (headline recommendations)

- **Fonts — KEEP Inter for all UI/body; keep the per-tenant display system.** Every premium
  reference uses a *neutral Swiss/geometric grotesque* for functional text (Huel → Suisse Int'l,
  Eatnaked → Satoshi, Caffé Milani → Owners, Ballena → Sweet Sans Pro). Inter is the free,
  self-hostable, variable-font, multi-script (Latin + Latin-Extended for `ç`/`ë` + full Cyrillic for
  Ukrainian) member of exactly that family. Switching costs multi-script coverage for no real gain.
  The upgrade is **how** Inter is used, not which font: tabular numerals for prices/macros, tighter
  tracking at display sizes, one weight ramp (400/500/600/700).
- **Product card** — adopt the **Crav Burgers** anatomy (full-bleed food photo on a rounded card,
  circular `+` quick-add top-right, name bottom-left, price bottom-right) but at **Caffé Milani's
  restraint** (category chip, generous whitespace, consistent aspect ratio, thin dividers).
- **Nutrition/ingredients** — adopt **Huel's** pattern: a row of *big-numeral + small-label* macro
  tiles, and ingredients/full nutrition behind a **progressive-disclosure accordion** — rendered
  **only when data exists**, inside the opened product detail.
- **Blocked captures:** Bucks Sauce (Vercel bot checkpoint) and Oatly's deep pages / Eatnaked's deep
  scroll (Cloudflare "verify you are human"). Routed around; hero/score data still recorded.

---

## 1. Reference gallery

Legend: **award** and **score** are transcribed from the awwwards site page. "Nominee" pages do not
publish a public numeric score, so those are marked `not published`. Fonts are read from the live
site's computed `font-family` (marked *confirmed*) or noted *unconfirmed* when blocked/obfuscated.

### 1.1 Crav Burgers — the ordering-flow reference (most on-point)
- **awwwards:** https://www.awwwards.com/sites/crav-burgers · live: **https://www.cravburgers.shop/**
- **Award / score:** **Site of the Day** (Jun 13, 2026) · **Overall 7.25 / 10**, **Jury 7.31 / 10** *(verified)*
- **Tech:** Next.js
- **Why praised (awwwards description, verbatim):** *"A playful burger ordering concept crafted with
  immersive visuals, smooth interactions, and dynamic motion design to make ordering fast, engaging,
  and irresistibly fun."*
- **Font:** `Mouse Memoirs` (Google Fonts; chunky rounded display) — *confirmed*. Used for
  everything, which is why it reads "loud/brand-forward," not "trustworthy/neutral."
- **What we take:** the **product-card + quick-add anatomy** and a confident type scale. Warm cream
  canvas (`~#F5E1D0`) + a single hot accent (red) + food photography does the heavy lifting.
- **Screens:** `refs/awwwards-crav-burgers-detail.png` (score proof), `refs/crav-01-hero.png` (hero),
  `refs/crav-04-menu.png` (menu hero + floating cart FAB), **`refs/crav-05-cards.png` (THE card
  pattern)**. Note: Crav is an ordering *concept* — it has no expanded detail / nutrition panel.

### 1.2 Bucks Sauce — hot-sauce e-commerce
- **awwwards:** https://www.awwwards.com/sites/bucks-sauce · live: https://buckssauce.com/ **(blocked)**
- **Award / score:** **Site of the Day** · **7.34 / 10** *(verified from awwwards page)*
- **Font:** *unconfirmed* — live site sits behind a **Vercel Security Checkpoint** ("Failed to verify
  your browser, Code 21") that blocks automation, so the real site could not be captured.
- **What we take:** confirms the SOTD bar for bold-product-photography food e-commerce; not used for
  layout specifics since capture was blocked.
- **Screens:** `refs/awwwards-bucks-sauce-detail.png` (score proof), `refs/bucks-01-hero.png` (the
  Vercel checkpoint, documenting the block).

### 1.3 Caffé Milani — the "clean, minimal, trustworthy" card reference
- **awwwards:** https://www.awwwards.com/sites/caffe-milani · live: **https://caffemilani.it/**
- **Award / score:** **Nominee** · score `not published` (nominee page)
- **Tech:** Shopify
- **Font:** `Owners` (OH no Type Co — humanist-geometric sans) — *confirmed*.
- **Why it matters:** the antithesis of Crav — restrained, editorial, premium. Product cards are a
  **category chip (uppercase) → centered product image → centered name → muted price**, separated by
  **thin vertical dividers** on a warm off-white (`~#F1ECE6`). Product detail is a calm two-column
  (large image / structured `Categoria:` `Formato:` labels + description).
- **What we take:** the minimal card grid, the category-chip label, tabular price treatment, and the
  "labels + values" structured detail block.
- **Screens:** `refs/milani-01-hero.png`, `refs/milani-02-product.png` (product detail),
  **`refs/milani-04-detail-notes.png` (the clean card grid)**.

### 1.4 Eatnaked — meal-delivery, ingredient chips
- **awwwards:** https://www.awwwards.com/sites/eatnaked · live: **https://eatnaked.co/** (partial — Cloudflare challenge on deep scroll)
- **Award / score:** **Honorable Mention** (May 3, 2026) · score `not published` · **Tech:** Three.js + React
- **Font:** `Satoshi Variable` (Fontshare / Indian Type Foundry — free geometric grotesque) — *confirmed*.
- **Why it matters:** a **dark, premium** food-delivery aesthetic. The hero uses **ingredient
  category chips** (`🫑 Vegetables`, `🌾 Grains`, `🍖 Proteins` — emoji + label pills) and a
  right-margin **scroll-spy nav** whose anchors include `Ingredients`. Meal cards are large rounded
  image cards with a pill CTA (`View Meals`) and a circular arrow icon.
- **What we take:** the **ingredient-chip** treatment (directly reusable for our allergen/tag/BOM
  chips) and the meal-card + section scroll-spy (we already use category scroll-anchors on `/s/:slug`).
- **Screens:** `refs/eatnaked-01-hero.png` (hero + ingredient chips), `refs/eatnaked-03-scroll.png`
  (lifestyle meal cards + scroll-spy). Deeper `Ingredients` section was gated by a Cloudflare
  "verify you are human" wall.

### 1.5 Matcha Cartel — editorial/agency-grade art direction
- **awwwards:** https://www.awwwards.com/sites/matcha-cartel · live: **https://matcha-cartel.com/** (passcode `MC26`, shown on the gate)
- **Award / score:** **Nominee** · score `not published` · **Tech:** Framer
- **Font:** *unconfirmed* (computed family reports generic `sans-serif`; Framer obfuscates the real
  face). Visually a **condensed industrial grotesque** for display + a **monospace** for UI/data labels.
- **Why it matters:** this is the clearest "**designed by an agency**" signal in the pool — a rigorous
  editorial system: a **numbered specimen grid** (`(01)…(15)`) with **crosshair/registration marks
  (`+`)** at every intersection, **monospaced micro-labels**, world-clock ticker, and a **"PHASE
  01–04" scroll-spy storytelling** section ("STONE GROUND — Dried tencha is ground using stone
  mills…"). Cold/edgy palette — we take the *structure*, not the mood.
- **What we take:** editorial rigor cues — small uppercase/mono labels, thin grid lines, tabular data
  alignment, and **process/"how it's made" storytelling** we can echo in the product detail.
- **Screens:** `refs/matcha-01-hero.png` (gate), **`refs/matcha-03-inside.png` (specimen grid +
  crosshairs)**, **`refs/matcha-04-product.png` (PHASE scroll-spy storytelling)**.

### 1.6 Ballena — fine-dining typography & editorial layout
- **awwwards:** https://www.awwwards.com/sites/ballena-fine-dining · live: **https://ballenacabo.com/**
- **Award / score:** **Nominee** · score `not published`
- **Font:** `Sweet Sans Pro` (Mark Simonson Studio — refined geometric sans) — *confirmed*.
- **Why it matters:** the "**premium & trustworthy**" end of the spectrum. Full-bleed video hero,
  centered wordmark, **wide-tracked uppercase headlines in cream over imagery** ("SHAPED BY SEA,
  GROUNDED IN LAND"), and — the agency tell — **small uppercase section labels in the left margin**
  (`ABOUT`) beside big editorial headlines, plus a card carousel with a thin progress control.
- **What we take:** wide letter-spacing on display headings, cream-on-photo overlays, and the
  **margin section-label** device for menu section headers.
- **Screens:** `refs/ballena-01-hero.png`, `refs/ballena-02-scroll.png` (margin labels + carousel).

### 1.7 Huel — nutrition & ingredients gold standard (best-in-class food brand)
- **Live:** **https://huel.com/products/huel-black-edition** (redirects to `de.huel.com`)
- Not an awwwards entry — included as the reference for the **nutrition/ingredients feature**.
- **Font:** `Suisse Int'l` (Swiss Typefaces — premium neutral grotesque) — *confirmed*.
- **Why it matters:** the cleanest macro presentation anywhere. Macros are **giant numeral + unit,
  with a small muted descriptor** stacked beside it: `40 g / Protein pro Mahlzeit`, `10 g /
  Ballaststoffe`, `26 / Vitamine & Mineralstoffe`, `Glutenfrei`. Price is a **green pill** (`Ab 2,20
  € / Mahlzeit`) beside an outline pill (`17 Mahlzeiten / Beutel`); 5-star rating + social proof
  ("4 Millionen Kunden"). Full ingredients + nutrition table live behind a **"Zutaten &
  Nährwertangaben" link → modal of per-variant accordions** — pure progressive disclosure.
- **What we take:** the **big-number macro tile**, the **price/quantity pill** treatment, and the
  **accordion/modal disclosure** so the default card stays minimal.
- **Screens:** **`refs/huel-02-macros.png` (macro tiles + price pills)**,
  `refs/huel-03-nutrition-table.png` (disclosure modal — per-variant accordions).

### 1.8 Oatly — brand voice & ingredient transparency (best-in-class food brand)
- **Live:** **https://www.oatly.com/products/oat-drink** (partial — persistent consent wall)
- **Font:** `Margo Pro` (bold quirky display) — *confirmed*.
- **Why it matters:** distinctive **brand personality as trust signal** — hand-drawn "NICE!"
  starburst badge, hatched textures, and a cookie wall that opens with copy ("We know you didn't come
  here for cookies"). It shows how *voice* can make a brand feel human without hurting clarity.
- **What we take:** a *light* touch of personality (a badge, a warm line of copy) — used sparingly so
  the storefront still reads trustworthy. The full nutrition panel sat behind a **persistent
  consent modal** that re-appears per page and blocked deep capture.
- **Screens:** `refs/oatly-02-nutrition.png` (brand-voice consent wall), `refs/oatly-03-nutrition.png`
  (bold display + "NICE!" badge behind the wall).

**Also in the pool (not captured, for future rounds):** BurgerFuel, Banh mi Viet Nam (GSAP),
La Revoltosa, WatchHouse (coffee), Imperiale Bolgheri (wine), KFC Rewards, Mr Day.

---

## 2. Typography direction

### 2.1 What the top references actually use
| Reference | Body/UI face | Family type | Multi-script? |
|---|---|---|---|
| Huel | Suisse Int'l | neutral Swiss grotesque | Latin + Cyrillic (commercial) |
| Eatnaked | Satoshi | geometric grotesque | **Latin only** (no Cyrillic) |
| Caffé Milani | Owners | humanist-geometric sans | Latin (+ limited) |
| Ballena | Sweet Sans Pro | refined geometric sans | Latin |
| Crav (playful) | Mouse Memoirs | rounded display | Latin |
| Matcha (editorial) | condensed grotesque + mono | industrial / mono | unconfirmed |

**Convergence:** the *trustworthy/premium* references all sit in one family — **neutral-to-geometric
grotesque sans for functional text**, with any character coming from a **separate display face**
used only for hero/brand/section headings. None of the premium references use a serif for UI.

### 2.2 Recommendation — KEEP Inter (with a usage upgrade), keep the tenant display system

**Keep Inter as the universal UI/body face** (`packages/ui/src/theme/tokens.css:58-59`). Rationale
grounded in the references *and* the app's constraints:

1. **It is the free, self-hostable, variable-font member of the exact family the premium references
   use.** Inter is the open equivalent of Suisse Int'l / Satoshi / Owners. Swapping to any of those
   is a paid/limited-license and/or **loses script coverage** — Satoshi (Eatnaked's face) has **no
   Cyrillic**, which breaks Ukrainian.
2. **Multi-script is a hard requirement here.** The app must render **Latin + Albanian diacritics
   (`ç`, `ë`) + Ukrainian Cyrillic (`і`, `ї`, `є`, `ґ`)**. Inter covers all three from one variable
   file; most "nicer-looking" grotesques cover only Latin. This alone settles keep-vs-switch.
3. **The gain is in usage, not the family.** Inter already ships variable + `tabular-nums` + a clean
   400–700 ramp. The references' polish comes from *disciplined* use, which we can replicate for free.

**The upgrade (do these, don't switch fonts):**
- **Tabular numerals everywhere numbers align** — prices, quantity steppers, macro tiles, ETAs:
  `font-variant-numeric: tabular-nums;` (Huel/Milani price alignment). Add a `--num-tabular` utility.
- **Tighten tracking as size grows** — display headings `letter-spacing: -0.01em to -0.02em`;
  large all-caps labels/section headers `+0.04em to +0.08em` (Ballena's wide-tracked caps). Body stays 0.
- **One type scale, applied consistently** (see §3). The single biggest "agency-grade" signal is
  *consistency*, not novelty.
- **Reserve tenant display faces for brand/product/section headings only** (they already exist:
  Playfair/Fraunces/Cormorant/Bebas/… in `fonts.ts`). Keep **all functional chrome on Inter**
  regardless of tenant — this is the consistent "chassis" that makes every tenant feel professionally
  built.

**⚠️ Multi-script finding to fix in `fonts.ts`:** several allowlisted *heading* faces are **Latin-only**
— notably **Fraunces** (default for italian/cafe/bakery cuisines) has **no Cyrillic**, and Bebas /
DM Serif / Yeseva are effectively Latin-only. When the UI language is Ukrainian, any *UI* header
rendered in a tenant's Latin-only display face will fall back mid-page. **Mitigation:** keep display
faces scoped to **Latin brand/product names**, and render **all localized UI headers (section titles,
"Cart", "Menu", empty states) in Inter**. If a Cyrillic-capable display face is wanted, add one that
ships Cyrillic (e.g. **Unbounded** or **Manrope** for headings) to the allowlist.

**Optional, low-risk enhancement (not required):** if a slightly warmer body texture than Inter is
ever wanted, **DM Sans** is already `base: true` in the allowlist and also covers Latin + Latin-Ext;
it is the safe in-house alternative. Do **not** add a new dependency (Satoshi/Geist) — it buys
nothing over Inter and risks script coverage.

---

## 3. Spacing / layout rhythm

The praised references share three habits: an **8px grid**, **generous whitespace**, and **one
radius language**. Grounded against the app's existing tokens (`tokens.css`: `--brand-radius: 12px`,
soft `--elev-*` scale, `--tap-min: 44px`).

**Base unit: 8px** (with a 4px sub-step). Every gap, pad, and margin is a multiple of 4/8.

| Token | Value | Use |
|---|---|---|
| space-1 / 2 / 3 | 4 / 8 / 12px | inline gaps, chip padding, icon-text gap |
| space-4 / 5 | 16 / 20px | **card padding** (16 mobile, 20 desktop) |
| space-6 / 8 | 24 / 32px | grid gutters, block spacing |
| space-10 / 12 | 40 / 48px | intra-section rhythm |
| space-16 / 20 | 64 / 80px | **section vertical rhythm** (desktop) |

- **Card radius: 16px** for product cards (bump from the 12px base — Crav/Eatnaked use large radii;
  keep `--brand-radius: 12px` for smaller controls, `--brand-radius-sm: 8px` for chips/inputs).
- **Card padding:** 16px mobile / 20px desktop; image sits flush to card edges (full-bleed), text
  block gets the padding.
- **Grid:** 2-col mobile → 3-col tablet → 3–4-col desktop, **gutter 16–24px**. **Consistent card
  aspect ratio is mandatory** (pick one, e.g. 1:1 or 4:3, `object-fit: cover`) — mismatched image
  heights are the #1 thing that makes a menu look un-designed.
- **Thin dividers** (`1px` at low-opacity border token) between list rows / card columns for the
  Milani/Matcha editorial feel — cheaper and calmer than boxes.
- **Whitespace budget:** section headers get ≥32px above; never let cards touch the viewport edges
  (≥16px inset). Let food photos breathe.
- **Elevation:** keep the existing soft `--elev-1/2`; hover lifts one step (`elev-1 → elev-2`) over
  `--motion-base` (240ms) with `--ease-out`. No hard borders on cards.

---

## 4. Product-card pattern (distilled spec)

A blend of **Crav's anatomy** (image-forward, quick-add) and **Milani's restraint** (chip, tabular
price, whitespace). One component, used identically across the grid.

### 4.1 Collapsed card (grid item)
```
┌───────────────────────────────┐  radius 16, --elev-1, no border
│  [CATEGORY / DIET CHIP]    (＋)│  chip: uppercase 11px, muted; ＋: 40–44px circle, brand-primary
│                               │
│        food photo             │  full-bleed, ONE fixed aspect ratio, object-fit: cover
│        (flush to edges)       │
│                               │
├───────────────────────────────┤  16/20px padding starts here
│  Product name          €12.50 │  name: Inter 600, 15–16px, clamp 2 lines; price: 600, tabular-nums
│  one-line description …        │  muted, clamp 1 line (optional)
│  · allergen/tag chips ·        │  small chips, only if present
└───────────────────────────────┘
```
- **Quick-add `＋`**: circular, top-right over the image (Crav). 44px tap target (`--tap-min`),
  `brand-primary` fill, `--color-on-primary` glyph. On tap → adds + micro-bounce; becomes a
  quantity stepper once in cart. On mobile it is always visible (no hover dependency).
- **Price**: prominent, `tabular-nums`, integer-safe (money is stored as integer minor units in this
  app — render via the existing formatter, never float math).
- **States (required):**
  - *loading* → skeleton: image block + 2 shimmer text lines (respect reduced-motion).
  - *sold-out / unavailable* → image desaturated + a "Sold out" chip; `＋` hidden/disabled.
  - *empty description / no chips* → simply omitted (no blank rows).
- **Chips**: category (Milani) and diet/allergen (Eatnaked) use the same chip primitive — uppercase
  11px, low-contrast fill, 8px radius.

### 4.2 Opened / expanded product detail (modal on desktop, sheet on mobile)
```
[ large hero image, same radius ]
Product name            (display face allowed here for brand tenants; else Inter 700)
€12.50    ★ 4.8 (128)   ← price tabular-nums + optional rating/social proof (Huel)
Short description paragraph (Inter 400, muted, ~60ch max)

▸ What's inside          ← accordion, RENDERED ONLY IF nutrition/ingredients data exists (see §5)

[ quantity −  1  + ]  [  Add · €12.50  ]   ← sticky bottom bar on mobile (thumb reach)
```
- Reuse the collapsed card's visual language (same radius, chips, tabular numerals) so open/closed
  feel like one system.
- **Add-to-cart** is a sticky bottom bar on mobile; quantity stepper mirrors the `＋` control.
- Keep the default view minimal — push nutrition/ingredients into the accordion (Huel), never a wall
  of data on open.

---

## 5. Nutrition + ingredients (BOM) display pattern

Feature rule: **shown ONLY when the product actually carries nutrition/ingredient data; hidden
entirely otherwise** (never an empty panel). Lives inside the opened product detail (§4.2). Pattern
is grounded in **Huel** (macro tiles + accordion), **Eatnaked** (ingredient chips), **Caffé Milani**
(labels + values, thin dividers).

### 5.1 Macros — "stat tile" row (Huel)
A horizontal row (desktop) / 2×2 grid or horizontal-scroll (mobile) of tiles. Each tile = **big
numeral + unit** on top, **small muted label** below. Only render tiles that have a value.
```
   520        28 g        45 g        18 g
  kcal       Protein      Carbs        Fat
```
- Numerals: Inter, weight 600–700, `tabular-nums`, ~24–28px; label: 11–12px uppercase, muted.
- Monochrome; optional single accent (e.g. protein in `--brand-primary`). No pie charts, no gauges —
  the numbers are the design (Huel).
- Tiles separated by whitespace or a hairline divider (Milani); no boxes needed.

### 5.2 Ingredients — chip wrap or clean list (Eatnaked / Milani)
- Short lists → **chip wrap** (Eatnaked's emoji+label pills), one chip per ingredient; the chip
  primitive from §4.1.
- Longer lists → a comma-separated line or a two-column list under an **"Ingredients"** label
  (Milani's calm labels + values).
- **Allergens** are emphasized, not hidden: bold the allergen token or prefix a small ⚠ chip. Wire
  this to the existing single-source allergen / Menu-Characteristics model — do **not** re-key
  allergen data here (that model is the safety source of truth).

### 5.3 Disclosure
- Wrap §5.1 + §5.2 in a single **"What's inside" accordion** (collapsed by default) so the product
  detail stays minimal when opened (Huel's modal-of-accordions, simplified to one inline accordion).
- If the data is very short (e.g. only 3 macros, no ingredient list), it may render inline expanded —
  but still only when data exists.
- Motion: expand over `--motion-base` (240ms) `--ease-out`; respect `prefers-reduced-motion`.

### 5.4 Reference screens
`refs/huel-02-macros.png` (macro tiles), `refs/huel-03-nutrition-table.png` (disclosure),
`refs/eatnaked-01-hero.png` (ingredient chips), `refs/milani-02-product.png` (labels + values).

---

## 6. Design principles (hit "creative-agency-grade, trustworthy")

1. **One neutral chassis.** All functional text — price, buttons, badges, macros, quantities — is
   Inter with tabular numerals, regardless of the tenant's display font. Consistency of the chrome is
   what reads as "professionally built."
2. **Rhythm over novelty.** One 8px grid, one card radius (16px), one type scale, one fixed image
   aspect ratio. Mismatched heights and ad-hoc spacing are the tells of un-designed UI.
3. **Let photography breathe.** Full-bleed food imagery + generous negative space (every reference).
   Never crowd cards to the viewport edge.
4. **Restraint in color.** Warm neutral surface + one brand accent + muted secondary text
   (Milani/Ballena/Huel). Reserve saturated color for one CTA and status.
5. **Progressive disclosure.** Nutrition/ingredients live behind a "What's inside" accordion; the
   default view stays calm (Huel).
6. **Show data only when it exists.** Hide empty macros/ingredients/descriptions — never render a
   blank panel or placeholder dash.
7. **Editorial cues = perceived craft.** Small uppercase/mono section & category labels, hairline
   dividers, tabular data alignment, margin section-labels (Matcha, Milani, Ballena).
8. **Purposeful micro-interaction.** Circular `＋` quick-add, soft one-step hover lift, 240ms
   `--ease-out`. Motion confirms actions; it never decorates for its own sake.
9. **A light, human touch — sparingly.** One warm line of copy or a small badge (Oatly) adds
   personality without costing trust. One per screen, not everywhere.
10. **Accessibility is non-negotiable.** AA contrast (existing tokens enforce it), 44px tap targets
    (`--tap-min`), `prefers-reduced-motion` honored, and every chosen face verified for Latin-Ext
    (`ç`/`ë`) + Cyrillic before it ships.

---

## 7. Capture log / caveats (honesty ledger)

- **Verified numeric scores:** Crav Burgers **7.25** (jury 7.31), Bucks Sauce **7.34** — both **Site
  of the Day**. All other awwwards entries are **Nominee / Honorable Mention** pages that **do not
  publish a public numeric score**; those are recorded as `not published` rather than invented.
- **Jury notes:** awwwards shows a public prose "description" only on some pages. Crav's is quoted
  verbatim (§1.1). Where no prose jury note was visible, the "why it matters" text describes the
  **observed** design, explicitly, and does not fabricate jury quotes.
- **Blocked / partial captures:**
  - **Bucks Sauce (live):** Vercel Security Checkpoint — no live capture (score still verified).
  - **Eatnaked (deep scroll):** Cloudflare "verify you are human" — hero + lifestyle captured, the
    dedicated Ingredients section was gated.
  - **Oatly (deep pages):** persistent per-page consent modal — brand-voice hero captured, full
    nutrition table gated. (Huel fully covers the nutrition pattern, so no loss.)
- **Fonts** are from live `getComputedStyle` where the site loaded (marked *confirmed*); Matcha
  Cartel (Framer-obfuscated) and Bucks Sauce (blocked) are *unconfirmed*.
- All screenshots: `docs/design/storefront-polish/refs/` (31 PNGs). Tooling: agent-browser CLI
  (headless Chromium via CDP).
