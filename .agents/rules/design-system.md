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
---
