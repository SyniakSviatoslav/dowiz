# DeliveryOS — Design System

## Identity
SaaS delivery platform for independent restaurants. Two UI layers:
- **Owner Layer** (admin): utilitarian, dense, Shopify Admin + Stripe reference
- **Tenant Layer** (client-facing): warm, food-premium, Wolt reference
- **Courier Layer**: readable one-handed, Uber Eats Driver reference

## Critical Rule #1 — Zero hardcoded colors
ALL colors via CSS custom properties. No hex values in components.
`var(--brand-primary)` everywhere. Violation = refactor.

## CSS Variables (Crimson Classic preset — default)
```css
:root {
  --brand-primary:       #C1121F;
  --brand-primary-hover: #9B0D17;
  --brand-primary-light: #FFF0F1;
  --brand-accent:        #F5F0E8;
  --brand-bg:            #FFFFFF;
  --brand-surface:       #F8F9FA;
  --brand-text:          #1A1A1A;
  --brand-text-muted:    #6B7280;
  --brand-border:        #E5E7EB;
  --brand-font-heading:  'DM Serif Display', serif;
  --brand-font-body:     'DM Sans', sans-serif;
  --brand-radius:        12px;
  --brand-radius-sm:     6px;
  --brand-radius-btn:    24px;
  --color-success:       #059669;
  --color-warning:       #D97706;
  --color-danger:        #DC2626;
  --color-info:          #2563EB;
}
```

## Typography
- Headings (client menu only): DM Serif Display, 36px mobile / 48px desktop
- All UI (admin, courier, forms): DM Sans, 14px default / 12px small / 16px large
- Code/keys: JetBrains Mono 11px

## Spacing — 4px base unit
Use only: 4 / 8 / 12 / 16 / 20 / 24 / 32 / 48 / 64px

## Border Radius
- Cards, modals: var(--brand-radius) = 12px
- Buttons (primary): var(--brand-radius-btn) = 24px (pill)
- Inputs: 8px
- Small elements: var(--brand-radius-sm) = 6px

## Tap targets (courier UI)
- Standard actions: min 48px height
- Critical actions (Delivered, Pickup): min 56px height

## Component States (always implement all 4)
Button: default / hover (primary-hover) / active (scale 0.98) / disabled (opacity 0.5)
Input: default / focus (border: brand-primary) / error (border: danger) / success
Toggle: ON = brand-primary fill / OFF = #D1D5DB

## Order Status Colors (semantic, not brand)
PENDING:     warning  #D97706  (pulse animation)
CONFIRMED:   info     #2563EB
PREPARING:   amber    #F59E0B
READY:       teal     #0D9488
IN_DELIVERY: blue     #3B82F6  (live badge)
DELIVERED:   success  #059669
REJECTED:    danger   #DC2626
CANCELLED:   danger   #DC2626
SCHEDULED:   purple   #7C3AED
PICKED_UP:   teal     #0D9488

## Storage rule
NEVER cookies. Only localStorage / sessionStorage.
Embed mode cart: localStorage prefix 'dos_embed_{locationId}_'

## Embed mode constraints
?embed=true → NO position:fixed, NO sticky elements, NO external links
postMessage for iframe height resize

## Visual references by screen

### Client Menu (/s/:slug)
- Primary ref: Wolt mobile app — editorial warmth, generous card spacing
- Secondary ref: Ottolenghi restaurant websites — DM Serif Display usage
- Anti-pattern: Generic food delivery app (Glovo) — avoid flat minimal cards

### Admin Dashboard (/admin/)
- Primary ref: Shopify Admin — information density, order management
- Secondary ref: Linear — subtle animations, status colors
- Anti-pattern: Corporate dashboard (SAP) — avoid heavy borders and tables

### Courier Screen (/courier/delivery/:id)
- Primary ref: Uber Eats Driver app — tap target sizes, map dominance
- Anti-pattern: Anything where action button < 56px