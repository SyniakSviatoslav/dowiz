---
# DeliveryOS — Always-on design rules

## CRITICAL: Zero hardcoded colors
Never write hex values (#xxxxxx) in any component or screen file.
Only CSS custom properties: var(--brand-primary), var(--brand-surface), etc.
Check: grep for any hex outside :root {} block before finishing any file.

## CRITICAL: No cookies
All state via localStorage or sessionStorage only. Never document.cookie.

## Typography rule
Client-facing headings: DM Serif Display only.
All UI (admin, courier, inputs, buttons): DM Sans only.
Never use Inter, Roboto, system-ui as primary fonts.

## Tap target rule
Every interactive element: min-height 44px.
Courier critical actions (Доставлено, Забрав): min-height 56px.

## Embed mode rule
Any element inside ?embed=true context:
  - No position: fixed
  - No position: sticky
  - No external links (target="_blank")

## 🔴 3-Language i18n rule
Every user-visible string MUST use `t('key', 'English fallback')` — zero exceptions.
- Every new key MUST be added to ALL 3 locales (`sq`, `en`, `uk`) in `packages/ui/src/lib/i18n.ts`
- No `alert('...')` with hardcoded string — always use `alert(t('key', '...'))`
- No hardcoded labels, placeholders, aria-labels, or alt text — use `t()`
- Check existing keys first before adding new ones (cross-reference with i18n.ts)
- When adding to `sq` and `uk`, the key must have meaningful translations, not English copy-paste
- Bad: `t('checkout.entrance')` without fallback → renders "checkout.entrance" raw
- Good: `t('checkout.entrance', 'Entrance')` → fallback shows if locale key missing
- All error messages visible to users must use `t()` — never raw strings
---
