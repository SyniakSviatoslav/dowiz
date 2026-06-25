# Mobile UI/UX Polish — iteration 1 findings (390px)

> SENSE: `audit/mobile-polish-i1/m-*.png` (19 surfaces, 390×844, staging, icons rendering).
> DIAGNOSE: 3 vision agents vs the Mobile Rubric (`loops/mobile-polish.yaml`), verify-before-fix.

## Verified REAL (fix — FE-only, token-conformant), ranked
| # | Finding | Surfaces | Sev | Fix |
|---|---------|----------|-----|-----|
| 1 | **Inputs <16px → iOS zoom-on-focus** (shared Input/Select/Textarea/SearchInput are `text-sm`=14px) | all forms | High | shared controls → `text-base md:text-sm` (16px mobile, 14px desktop) — one place, fixes all |
| 2 | **Chrome buries content** — welcome banner + 5-tile KPI grid push first order ~73% down | admin Orders/Dashboard | High | hide/collapse welcome banner on mobile; KPI grid → compact row |
| 3 | **Zero-value KPI painted alert-amber** (`0 Në pritje`) | admin Orders | High | zero/empty KPI → neutral `--brand-text-muted`, reserve amber for warnings |
| 4 | **Chip/tab rows hard-clip mid-word, no scroll affordance** ("Përbë…", "Beverages", "VEZË") | Supplies, Menu, Orders filters, storefront tabs+sort | Med | right-edge fade-mask + scroll-snap padding on horizontal chip/tab rows |
| 5 | **Checkout sticky CTA overlaps content** (occludes Dërgo/Merr + address hint) | storefront checkout | Med | scroll-container `padding-bottom` = CTA height + `env(safe-area-inset-bottom)` |
| 6 | **CRM toolbar squeezes search to icon-only square** | admin CRM | Med | stack: search full-width row, sort+Export below on mobile |
| 7 | **Couriers primary "+ Shto Postier" stranded top-right** | admin Couriers | Med | bottom sticky / FAB on mobile |
| 8 | **Tap targets <44px** — row edit/delete/chevron icons ~36px | Menu, Couriers, Orders | Med | ≥44px hit area (min-h/min-w) without enlarging the glyph |
| 9 | **Supplies info banner eats a card of vertical space** | admin Supplies | Low-Med | dismissible / single-line on mobile |
| 10 | **Courier empty-state card hugs top third, large dead void below** | courier tasks/earnings/history | Low | vertically center the empty card in the viewport |
| 11 | **Lang pills (SQ/EN/UA) heavy header chrome on 390px** | all (LanguageSwitcher full) | Low | collapse to a dropdown on mobile |

## VERIFY (confirm before asserting)
- Checkout input font ≥16px (covered by #1 once shared controls bump).
- Activation sticky CTA safe-area inset (covered by the safe-area pattern in #5).

## FLAG-ONLY (NOT mobile bugs — do not fix here)
- Demo seed data: CRM all-E2E/QA fixtures + masked phones; 60 placeholder "Pa telefon" couriers w/ "?" avatars; "E2E Seam" orders; blank product thumbnails.
- No food photography (MEDIA off + unseeded) — the oversized empty modal hero is downstream of this.
- Couriers "60 online" semantics, delivery-fee sourcing — business logic (flagged in the main findings doc).

## Already-good (confirmed, do not touch)
- Icons render everywhere (self-host holds). Storefront cart/modal bottom-sheets, success toast, courier
  empty states, promotions empty state, analytics no-data KPI — all intentional & branded.

## Iteration-1 ACT scope (this pass)
Highest-value, bounded, verified: **#1 (iOS zoom), #3 (zero-KPI color), #4 (chip scroll-fade), #5 (checkout
overlap), #2 (mobile banner/KPI collapse)**. Deferred to iter 2: #6 CRM toolbar stack, #7 couriers FAB,
#8 tap-target sweep, #10 courier empty-state centering, #11 lang dropdown.
