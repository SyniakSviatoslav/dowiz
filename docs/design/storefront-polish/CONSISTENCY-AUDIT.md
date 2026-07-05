# Storefront Design-Consistency Audit

**Scope:** customer storefront only — `/s/:slug` menu, product detail, compare panel, cart, checkout, order status.
**Branch:** `fix/audit-remediation` · **Mode:** READ-ONLY audit → prioritized fix-list for a later implementation lane.
**Goal:** make the whole service read "professional and trustworthy, like a real creative agency designed it," with enforced font + padding consistency.
**Guard-rail:** none of these fixes touch money / authz / RLS. `PriceDisplay` appears below only for its **visual font-weight** — never its amount/format logic.

Surfaces & primary files audited:
- `packages/ui/src/theme/tokens.css` — design tokens (type scale, spacing, radius, weights) — **already exists, well-formed**
- `packages/ui/src/theme/fonts.ts` — per-tenant font allowlist / pairings — **clean, not a source of drift**
- `packages/ui/tailwind.config.ts` — token→utility bridge (`text-step-*`, `spacing`, `borderRadius`)
- `apps/web/src/index.css` — global base (`body { font-family: var(--brand-font-body) }`)
- `apps/web/src/routes/ClientLayout.tsx` — header + cart drawer
- `apps/web/src/pages/client/MenuPage.tsx` — menu list, category sections, product detail modal (~2600 lines)
- `packages/ui/src/components/client/ProductCard.tsx` — the list card
- `apps/web/src/pages/client/MenuComparePanel.tsx` + `apps/web/src/components/client/DishStats.tsx` — compare panel + nutrition viz
- `apps/web/src/pages/client/CheckoutPage.tsx` + `apps/web/src/pages/client/checkout/*` — checkout
- `apps/web/src/pages/client/OrderStatusPage.tsx` — post-order status

---

## The core finding (why it looks inconsistent)

**The design system is good; adoption is partial.** `tokens.css` already ships a modular type scale (`--text-2xs`…`--text-3xl`), a spacing scale (`--space-*`), paired line-heights, weights (`--weight-*`), and a radius scale — and `tailwind.config.ts` bridges them (`text-step-*`, remapped `spacing`, `borderRadius`). The storefront just doesn't use them uniformly:

| Signal | Measure |
|---|---|
| Stock Tailwind `text-{xs..3xl}` still in use | **103** occurrences |
| Modular `text-step-*` in use | **113** occurrences |
| The two scales **disagree at xl/2xl/3xl** | `text-xl`=20px vs `text-step-xl`=22px · `text-2xl`=24px vs `text-step-2xl`=28px · `text-3xl`=30px vs `text-step-3xl`=36px |
| Half-step spacing (`*-N.5`, off the 8pt token grid) | **99** occurrences |
| Distinct card corner-radii for "a card" | **12px / 16px / 24px** across screens |
| Distinct primary-CTA signatures | **5** (heights 44/46/48/56px, 2 radii, 2 weights, 3 text sizes) |
| Global heading-font rule | **none** — headings silently inherit the **body** font unless each call-site overrides |

Because stock `text-2xl` (24px) and `text-step-2xl` (28px) are *different sizes*, the same role (page title) is literally 4px bigger on one screen than another. Because there is no global heading-font rule, the **same dish name** renders in the heading font on the card and the body font in the detail modal.

---

## TOP 5 HIGHEST-IMPACT FIXES

1. **Add a global heading-font rule** so `h1/h2/h3` in the storefront default to `--brand-font-heading`. Root cause: `apps/web/src/index.css:15` sets only `body { font-family: var(--brand-font-body) }`, with **no** heading rule. Result: the dish-name title is heading-font on the card (`ProductCard.tsx:130`) but body-font in the detail modal (`MenuPage.tsx:1445`) and compare panel (`MenuComparePanel.tsx:86,128`). The `MenuPage.tsx:1444` comment even claims the modal matches the card — it does not. *Quick-win.*

2. **Unify the primary-CTA button** into one component/token set. Today the "Add to cart / Place order / Checkout" family has **5 signatures**: `ClientLayout.tsx:195` (`h-12 rounded-full font-bold text-sm`), `ClientLayout.tsx:285` (`h-12 rounded-full font-bold text-base`), `CheckoutPage.tsx:737` (`h-14 rounded-full font-bold text-base`), `MenuPage.tsx:1695` (`h-[46px]` + `--brand-radius-btn` + `font-bold text-step-sm`), `OrderStatusPage.tsx:454,620` (`min-h-11 --brand-radius-btn font-semibold`). Two radii (`rounded-full` 9999px vs `--brand-radius-btn` 78px), two weights, three sizes. *Larger-refactor.*

3. **Collapse the two parallel radius families and pick one card radius.** `--brand-radius`(12px)/`--brand-radius-sm`(8px) duplicate `--radius-lg`(12px)/`--radius-md`(8px) — same pixels, two names — and card containers pick different ones: checkout card `--brand-radius` 12px (`CheckoutPage.tsx:640`), detail-modal inner cards `rounded-xl` 16px (`MenuPage.tsx:1497`), `DishStats` full container `rounded-2xl` 24px (`DishStats.tsx:377`), list card `rounded-xl` 16px (`ProductCard.tsx:76`). Introduce `--radius-card` and use it everywhere a card renders. *Larger-refactor.*

4. **Migrate stock `text-xl/2xl/3xl` → `text-step-*` for headings** so the page-title/section-heading roles stop being two sizes. Concrete drift: page `h1` is `text-step-2xl` (28px) in `CheckoutPage.tsx:551` but stock `text-2xl` (24px) in `OrderStatusPage.tsx:549`; detail-modal title is stock `text-xl` (20px) in `MenuPage.tsx:1445` while checkout section headers are `text-step-xl` (22px). *Quick-win per file, medium in aggregate.*

5. **Normalize card internal padding + section-header margins onto the token grid.** `ProductCard.tsx:122` uses `p-2.5`(10px)/`p-3.5`(14px) — off-grid Tailwind fractional defaults, not `--space-*` — while every other card uses `p-4`(16px) (`MenuPage.tsx:1497`, `CheckoutPage.tsx:640`, `DishStats.tsx:377`). Checkout section headers drift `mb-3`/`mb-4`/`mb-6` for the identical `text-step-xl font-semibold` element (`CheckoutPage.tsx:641` mb-3, `.../checkout/*Section.tsx` mb-4, `DeliveryDetailsSection.tsx:51` mb-6). *Quick-win.*

---

## TYPOGRAPHY findings

> Root causes: (a) no global heading-font rule → headings inherit body font unless overridden; (b) two live type scales (`text-*` stock vs `text-step-*`) that disagree at xl/2xl/3xl; (c) ad-hoc weight choices for the same role.

### T1 — No global heading-font rule; heading-vs-body font is applied per call-site *(quick-win — highest impact)*
- `apps/web/src/index.css:15` — only `body { font-family: var(--brand-font-body) }`; there is no `h1,h2,h3 { font-family: var(--brand-font-heading) }`. In `tokens.css` the only heading-font rule is scoped to `[data-skin="paper"]` (line 371), which the client storefront does **not** use.
- Consequence — the **same dish name** in two fonts:
  - `packages/ui/src/components/client/ProductCard.tsx:130` — title **explicitly** `fontFamily: 'var(--brand-font-heading)'`
  - `apps/web/src/pages/client/MenuPage.tsx:1172` — category header **explicitly** heading font
  - `apps/web/src/pages/client/MenuPage.tsx:1445` — detail-modal title **no override** → **body font** (and comment at `:1444` wrongly claims it matches the card)
  - `apps/web/src/pages/client/MenuComparePanel.tsx:86` — compare dish name, **no override** → body font
  - `apps/web/src/pages/client/MenuComparePanel.tsx:128` — compare panel heading, **no override** → body font
  - `apps/web/src/pages/client/OrderStatusPage.tsx:694,720` — section `h2`s, **no override** → body font
- **Fix:** add a storefront-scoped `h1,h2,h3 { font-family: var(--brand-font-heading) }` (or a `.type-heading` utility) so headings are heading-font by default; then delete the per-call-site `fontFamily` inline styles. Removes ~9 inline overrides and closes the card↔modal mismatch.

### T2 — Two type scales coexist and disagree *(quick-win per file; medium in aggregate)*
- Stock Tailwind `text-*` = 103 uses; modular `text-step-*` = 113 uses. Divergent sizes: `text-xl`=1.25rem(20px) vs `--text-xl`=1.375rem(22px); `text-2xl`=1.5rem(24px) vs `--text-2xl`=1.75rem(28px); `text-3xl`=1.875rem(30px) vs `--text-3xl`=2.25rem(36px).
- Per-file stock:step split shows the uneven migration: `MenuPage.tsx` 35:29 · `CheckoutPage.tsx` 13:5 · **`OrderStatusPage.tsx` 21:3** · **`ClientLayout.tsx` 13:1** · `ProductCard.tsx` 1:6 · `MenuComparePanel.tsx` 3:6. OrderStatus and ClientLayout barely adopted the scale.
- Same-role, different-size instances:
  - Page `h1`: `CheckoutPage.tsx:551` `text-step-2xl` (28px) vs `OrderStatusPage.tsx:549` `text-2xl` (24px).
  - Detail-modal title `MenuPage.tsx:1445` `text-xl` (20px) vs checkout section header `text-step-xl` (22px) — a title should not be *smaller* than a section header.
- **Fix:** migrate heading-tier stock classes (`text-xl/2xl/3xl`) to `text-step-*`; leave body-tier (`text-xs/sm/base/lg`, identical in both scales) as-is to minimize churn. Ban new stock `text-xl/2xl/3xl` via lint (see Enforcement).

### T3 — Dish-name / heading rendered ≥4 different ways *(larger-refactor — the "one heading system" case)*
| Role instance | size | weight | font | file:line |
|---|---|---|---|---|
| List card title (photo) | `text-step-sm` | `font-semibold` | heading | `ProductCard.tsx:130` |
| List card title (photoless) | `text-step-base` | `font-semibold` | heading | `ProductCard.tsx:130` |
| Category section header | `text-lg` | `font-bold` | heading | `MenuPage.tsx:1172` |
| Detail-modal title | `text-xl` | `font-bold` | **body** | `MenuPage.tsx:1445` |
| Compare dish name | `text-sm` | `font-bold` | **body** | `MenuComparePanel.tsx:86` |
| Checkout section header | `text-step-xl` | `font-semibold` | heading | `CheckoutPage.tsx:641` + `checkout/*Section.tsx` |
| OrderStatus section header | *(none→base)* | `font-semibold` | **body** | `OrderStatusPage.tsx:694,720` |
- Weight drift for the same tier: `font-bold` vs `font-semibold`. Overall weight counts across storefront: `font-semibold` 52, `font-bold` 45, `font-medium` 28, plus `font-black` 2 and `font-extrabold` 1.
- **Fix:** define semantic type roles (Display / Title / Section / Body / Label / Caption / Micro — see token proposal) and map each element to a role; a "dish name" is one role everywhere.

### T4 — Price weight drifts 800 ↔ 900 *(quick-win — visual weight only, not money logic)*
- `ProductCard.tsx:166` `fontWeight: 800` · `MenuPage.tsx:1458` `text-xl font-black` (900) · `MenuComparePanel.tsx:88` `text-base font-black` (900) · `MenuPage.tsx:1707` `font-extrabold` (800) · `DishStats.tsx:165` kcal `fontWeight: 800`.
- **Fix:** one price weight token (`--weight-price: 800`) applied via `PriceDisplay`'s own wrapper; do **not** touch amount/format logic.

### T5 — Uppercase section-label sub-headers use stock `text-xs`, inconsistent bottom margin *(quick-win)*
- `text-xs font-semibold uppercase tracking-wider` appears in `DishStats.tsx:260` (SectionHeading) and inline in `MenuPage.tsx:1498,1534,1559,1587` — internally consistent styling but stock `text-xs` (not `text-step-xs`), and margin drifts `mb-3` (1498, 1587) vs `mb-2.5` (1534, 1559).
- **Fix:** promote to a shared `<SectionLabel>` (already half-done in `DishStats.tsx:258`) using `text-step-xs`; fix `mb` to one token step.

### T6 — Inline `fontFamily` / `fontSize` scattered as inline styles *(quick-win, cleanup)*
- Inline `fontFamily: 'var(--brand-font-heading)'`: `ClientLayout.tsx:179`, `OrderStatusPage.tsx:549`, `CheckoutPage.tsx:530,551,641`, `MenuPage.tsx:1172,1736`, `checkout/OrderSummarySection.tsx:39`, `checkout/PaymentSection.tsx:32`, `checkout/ContactInfoSection.tsx:62`, `checkout/DeliveryDetailsSection.tsx:51`, `ProductCard.tsx:130,155`.
- Inline `fontSize` in px/rem instead of a scale token: `ProductCard.tsx:178` (`0.85rem`), `:188` (`0.7rem`); `MenuComparePanel.tsx:66,77,94` (`0.7`–`0.72rem`); `MenuPage.tsx:1439` (`0.6rem`), `:1468` (`0.75rem`), `:1509,1730` (`0.7rem`); `DishStats.tsx:159,164` (size-relative), `:198` (`0.85rem`). These sub-`--text-2xs` glyph sizes have no token — several are icon glyphs (acceptable) but the text ones should map to `--text-2xs`.
- **Fix:** once T1 lands, the heading `fontFamily` inline styles are redundant — remove them. Introduce `--text-micro` (10px) if a sub-11px text tier is genuinely needed, else round up to `--text-2xs`.

**No arbitrary `text-[Npx]` size classes exist** — that discipline already holds. The drift is stock-vs-step and inline-vs-token, not raw pixels.

---

## SPACING findings

> There **is** a spacing scale: `tokens.css:160` (`--space-0`…`--space-16`) and `tailwind.config.ts:88` remaps the numeric keys (`p-4`→`var(--space-4)` etc.), so whole-step utilities are token-backed and **no `p-[Npx]` arbitrary values exist**. The drift is (a) heavy reliance on Tailwind's *fractional* half-steps that fall through to defaults (not token-backed), and (b) the same semantic element choosing different values.

### S1 — Card internal padding is off-grid and inconsistent *(quick-win)*
- List card: `ProductCard.tsx:122` `p-2.5`(10px, photo) / `p-3.5`(14px, photoless) — **off the token grid** (Tailwind fractional defaults; `--space-*` has no 2.5/3.5).
- Every other card: `p-4`(16px) — `MenuPage.tsx:1497` (taste card), `MenuPage.tsx:1558` (allergen card), `CheckoutPage.tsx:640` (payment card), `DishStats.tsx:377` (`p-4 sm:p-5`).
- Modal body: `MenuPage.tsx:1427` `p-5`(20px). Compare sheet: `MenuComparePanel.tsx:120` `p-4 pb-6`.
- **Fix:** define `--card-pad` and use it for all card interiors; move the list card to the grid (16px, or a deliberate `--card-pad-compact`).

### S2 — 99 half-step (`*-N.5`) spacing usages, off the 8pt rhythm *(larger-refactor — rhythm normalization)*
- Top offenders: `gap-1.5`×21, `mb-1.5`×15, `py-0.5`×9, `mt-0.5`×9, `gap-0.5`×9, `px-1.5`×6, `gap-2.5`×5, `py-1.5`×4, `mb-2.5`×3, `py-3.5`×2, `px-3.5`×2, `px-2.5`×2 … These resolve to 2/6/10/14px via Tailwind defaults (a 2px grid) and are **not** `--space-*`-backed.
- These aren't "wrong" (they're on a 2pt grid) but they proliferate micro-decisions that read as unpolished. A tightened set (0 / 2 / 4 / 8 / 12 / 16 / 24 / 32) removes most half-steps.
- **Fix:** add `--space-0_5`(2px) and `--space-1_5`(6px) tokens **only if kept**, and prune the rest toward whole steps during the card/component refactor. Not a blind find-replace — do it element-by-element with the component unification (S1, C-series).

### S3 — Off-8pt integer spacing *(quick-win)*
- `MenuPage.tsx:1165` `mb-7`(28px) section gap (Tailwind default; not in `--space-*`) · plus isolated `py-9`, `px-7`. `--space-*` jumps 6(24px)→8(32px), so 28px has no token.
- **Fix:** snap section gap to `--space-6`(24) or `--space-8`(32).

### S4 — Section-header bottom-margin drift for the identical element *(quick-win)*
- `text-step-xl font-semibold` section header uses `mb-3` (`CheckoutPage.tsx:641`), `mb-4` (`checkout/OrderSummarySection.tsx:39`, `PaymentSection.tsx:32`, `ContactInfoSection.tsx:62`), `mb-6` (`checkout/DeliveryDetailsSection.tsx:51`).
- **Fix:** one `--heading-gap` (e.g. `--space-4`) for section-header→content.

### S5 — Uppercase-label bottom-margin drift *(quick-win)* — see T5 (`mb-3` vs `mb-2.5`).

---

## CARD-SYSTEM findings

> There is no shared Card primitive; each surface hand-rolls a container. The result is three radii, three shadows/borders styles, two alert-row styles, and five button signatures for what should be one card system.

### C1 — "A card" has three different corner radii *(larger-refactor)*
- List card `rounded-xl` 16px (`ProductCard.tsx:76`) · detail-modal inner cards `rounded-xl` 16px (`MenuPage.tsx:1497,1558`) · checkout card `--brand-radius` 12px (`CheckoutPage.tsx:640`) · `DishStats` full container `rounded-2xl` 24px (`DishStats.tsx:377`) · compare sheet `rounded-t-2xl` (`MenuComparePanel.tsx:120`).
- Compounded by **two parallel radius token families for the same pixels**: `--brand-radius`(12)=`--radius-lg`(12), `--brand-radius-sm`(8)=`--radius-md`(8). Storefront radius counts: `rounded-full`×49, `--brand-radius-sm`×24, `--brand-radius`×19, `rounded-xl`×9, `--brand-radius-btn`×5, `rounded-lg`×4, `rounded-2xl`×4, `rounded-md`×3.
- **Fix:** introduce semantic `--radius-card`, `--radius-control`, `--radius-pill`; alias the legacy `--brand-radius*` to them; migrate call-sites. Pick **one** card radius (recommend 16px).

### C2 — Card surface treatment (border/shadow/bg) is inconsistent *(larger-refactor)*
- List card: `border` + `bg var(--brand-surface)` + motion box-shadow from JS variants (`ProductCard.tsx:34-37,76,79`).
- Taste card: `bg var(--brand-surface)`, **no border, no shadow** (`MenuPage.tsx:1497`).
- Checkout card: `border` + `boxShadow: var(--elev-1)` + `bg var(--brand-surface)` (`CheckoutPage.tsx:640`).
- DishStats full: `border: 1px solid var(--brand-border)`, no shadow (`DishStats.tsx:377`).
- **Fix:** one `<Card>` primitive (surface + border + `--elev-1` + `--radius-card` + `--card-pad`); variants for `flat`/`raised`.

### C3 — The product-card *family* is visually unaligned *(larger-refactor)*
The three components that render a dish don't share a type/spacing contract:
- **List card** (`ProductCard.tsx`): title `text-step-sm/base` semibold **heading** font; desc `text-step-2xs`; padding `p-2.5/3.5`; radius `rounded-xl`; price `fontWeight 800`.
- **Detail modal** (`MenuPage.tsx:1425-1711`): title `text-xl` bold **body** font (`:1445`); desc `text-sm leading-relaxed` (`:1476`); body `p-5`; inner cards `rounded-xl p-4`; price `text-xl font-black` (900, `:1458`).
- **Compare column** (`MenuComparePanel.tsx:84-106`): name `text-sm` bold **body** font (`:86`); price `text-base font-black` (900, `:88`); gaps `gap-2`.
- Net mismatches a "consistent card system" would unify: **title font** (heading vs body), **title size** (step-sm/base vs text-xl vs text-sm), **price weight** (800 vs 900), **radius** (16 vs mixed), **padding** (10/14 vs 20 vs 16).
- **Fix:** a shared `DishTitle`, `DishPrice`, `DishMeta`, and `Card` used by all three; the modal becomes the card "expanded," visibly the same object.

### C4 — Alert / banner rows are two different components *(quick-win)*
- MenuPage banners: `MenuPage.tsx:1013` `px-4 py-3.5 rounded-2xl border-2` · `:1063` `px-4 py-3.5 rounded-2xl border-2` · `:1080` `px-4 py-3 rounded-xl border`.
- Checkout alerts: `CheckoutPage.tsx:671,699` `p-4 rounded-[--brand-radius] border` · `:722` `px-4 py-3 rounded-[--brand-radius] border`.
- Same semantic (inline alert) but `rounded-2xl border-2` (Menu) vs `--brand-radius border` (Checkout), and `py-3.5` vs `py-3`.
- **Fix:** one `<Alert variant="danger|warning|info">` with fixed radius/border/padding.

### C5 — Primary CTA button: 5 signatures *(larger-refactor)* — see TOP-5 #2. Heights 44/46/48/56px; radius `rounded-full`(9999) vs `--brand-radius-btn`(78px); weight bold vs semibold; text `sm`/`base`/`step-sm`. Secondary/pill buttons also drift: `MenuPage.tsx:1044` `px-3.5 h-9 rounded-full text-step-xs font-bold`, `:1291` `px-4 h-9 rounded-full text-step-2xs font-bold`, `MenuComparePanel.tsx:129` `px-3 h-9 rounded-full text-step-2xs font-semibold`.
- **Fix:** `<Button size="lg|md|sm" variant="primary|secondary|ghost">` with tokenized height/radius/weight/size; replaces all hand-rolled CTAs.

---

## PROPOSED TOKENS (fit the existing `tokens.css` structure)

Most primitives already exist. The additions are **semantic/role tokens** (component-level) plus two rhythm cleanups. Drop into `:root` in `packages/ui/src/theme/tokens.css`.

### Type scale — already present; formalize as semantic roles
The primitive scale at `tokens.css:134-158` is sound. Add a **role layer** so implementers reference a role, not a raw size, and each role pins size + weight + line-height + font:

```css
:root {
  /* ── Optional sub-2xs tier (only if a real <11px TEXT need survives; else round to --text-2xs) ── */
  --text-micro: 0.625rem;   /* 10px — retire ad-hoc inline 0.6–0.7rem TEXT (not icon glyphs) */

  /* ── Semantic type ROLES (size · line-height · weight) — map every element to ONE of these ── */
  /* Display / hero            */ --type-display-size: var(--text-3xl);  --type-display-lh: var(--leading-tight); --type-display-weight: var(--weight-bold);
  /* Page title (h1)           */ --type-title-size:   var(--text-2xl);  --type-title-lh:   var(--leading-tight); --type-title-weight:   var(--weight-bold);
  /* Section heading (h2)      */ --type-section-size: var(--text-xl);   --type-section-lh: var(--leading-snug);  --type-section-weight: var(--weight-semibold);
  /* Card / dish title (h3)    */ --type-cardtitle-size: var(--text-lg); --type-cardtitle-lh: var(--leading-snug); --type-cardtitle-weight: var(--weight-semibold);
  /* Body                      */ --type-body-size:    var(--text-base); --type-body-lh:    var(--leading-normal);
  /* Secondary / meta          */ --type-label-size:   var(--text-sm);   --type-label-lh:   var(--leading-normal); --type-label-weight: var(--weight-medium);
  /* Caption / badge           */ --type-caption-size: var(--text-xs);   --type-caption-lh: var(--leading-normal);
  /* Micro (pills, allergen)   */ --type-micro-size:   var(--text-2xs);  --type-micro-lh:   var(--leading-normal);

  /* ── One price weight (visual only — never touches money logic) ── */
  --weight-price: 800;
}

/* ── Storefront global heading font (fixes T1). Scope to the client shell so
   /admin + /courier paper skin still wins by proximity. body already = body font. ── */
:where(h1, h2, h3, h4) { font-family: var(--brand-font-heading); }
```

> Implement roles as either Tailwind component classes (`.type-title`, `.type-section`, `.type-card-title`, `.type-body`, `.type-label`, `.type-caption`, `.type-micro`) or extend `tailwind.config.ts` `fontSize` with `title`/`section`/`card-title` keys. Prefer classes so weight + line-height + font travel together (a raw `text-*` can't carry weight).

### Spacing scale — already an 8pt grid; add the two half-steps you actually keep, prune the rest
The scale at `tokens.css:160-171` is correct. Only add the fractional steps if S2 keeps them:

```css
:root {
  --space-0_5: 2px;   /* only if retained after S2 prune */
  --space-1_5: 6px;   /* only if retained after S2 prune */
  /* (do NOT add 2.5/3.5/7/9 tokens — snap those to whole steps instead) */

  /* ── Semantic component spacing/radius (the "card system") ── */
  --card-pad:        var(--space-4);   /* 16px — all card interiors (fixes S1) */
  --card-pad-compact: var(--space-3);  /* 12px — dense list card, if a compact variant is wanted */
  --card-gap:        var(--space-3);   /* 12px — grid gap between cards */
  --section-gap:     var(--space-6);   /* 24px — between menu sections (fixes S3 mb-7) */
  --heading-gap:     var(--space-4);   /* 16px — heading → its content (fixes S4) */

  --radius-card:    var(--radius-xl);  /* 16px — the ONE card radius (fixes C1) */
  --radius-control: var(--radius-md);  /* 8px  — inputs, small controls */
  --radius-pill:    var(--radius-full);/* buttons/badges that are pills */

  --btn-h-lg: 56px;  --btn-h-md: 48px;  --btn-h-sm: 40px;  /* fixes C5 heights */
  --btn-radius: var(--radius-pill);                        /* pick ONE button radius */
}

/* Alias the legacy duplicate radius family so old call-sites converge (fixes C1 duplication) */
:root {
  --brand-radius:     var(--radius-card);     /* was 12px → now 16px card radius */
  --brand-radius-sm:  var(--radius-control);  /* was 8px  → controls */
  --brand-radius-btn: var(--btn-radius);      /* was 78px → pill */
}
```

> Aliasing `--brand-radius*` to the new semantic tokens converges the 19 `--brand-radius` + 24 `--brand-radius-sm` + 5 `--brand-radius-btn` call-sites with zero per-site edits — a cheap first move before the component refactor.

---

## ENFORCEMENT (so it doesn't drift back)

There is already an eslint plugin at `tools/eslint-plugin-local` (per CLAUDE.md self-improvement loop). Add advisory-then-blocking rules:
1. **Ban stock heading sizes in the storefront:** disallow `text-xl|text-2xl|text-3xl` in `apps/web/src/pages/client/**` and `packages/ui/src/components/client/**` — require `text-step-*` / a `.type-*` role class.
2. **Ban inline `fontFamily`/`fontSize`** string literals in storefront JSX (allow icon-glyph `fontSize` via an explicit `/* glyph */` exception).
3. **Ban off-grid spacing:** flag `*-2.5`, `*-3.5`, `*-7`, `*-9`, `p-[…px]` in storefront components.
4. **Ban raw `rounded-{md,lg}`** in favor of `--radius-card`/`--radius-control` semantic tokens.

A `packages/ui/src/theme/tokens.css` snapshot test can assert the role/component tokens exist (red→green guardrail per the repo's ratchet rule).

---

## Quick-win vs larger-refactor summary

| Item | Class | Effort |
|---|---|---|
| T1 global heading font | Typography | **quick-win** (1 CSS rule + delete ~9 inline styles) |
| T2 stock→step heading migration | Typography | quick per file / medium aggregate |
| T4 price weight token | Typography | **quick-win** |
| T5/S5 section-label size + margin | Typo/Spacing | **quick-win** |
| T6 remove inline font styles | Typography | **quick-win** (after T1) |
| S1 card padding to grid | Spacing | **quick-win** |
| S3 off-grid integers | Spacing | **quick-win** |
| S4 heading-gap token | Spacing | **quick-win** |
| C4 alert component | Card-system | **quick-win** |
| Radius family aliasing | Card-system | **quick-win** (token alias, zero call-site edits) |
| T3 semantic type roles | Typography | larger-refactor |
| S2 half-step prune | Spacing | larger-refactor |
| C1 one card radius | Card-system | larger-refactor |
| C2 Card primitive | Card-system | larger-refactor |
| C3 dish-card family unification | Card-system | larger-refactor |
| C5 Button primitive | Card-system | larger-refactor |

**Sequencing recommendation:** land all quick-wins + radius aliasing first (immediate visible lift, low risk), then the semantic role/component tokens (T3, C1–C3, C5) as one "storefront card system" refactor lane, with the eslint guardrails added last to lock it.
