# DeliveryOS — DESIGN.md
> Machine-readable brand contract · Single source of truth · Open Design 9-section schema
> Generated from: `packages/ui/src/theme/tokens.css` + `design.md` + `DeliveryOS-Figma-Make-Prompt`
> **This DESIGN.md encodes the EXISTING design system. It does NOT create a new one.**

---

## 1. Color

### Brand Colors
| Token | Food Dark (default) | Crimson Classic | Ocean Fresh | Midnight Urban | Sage Garden | Royal Gold | Coral Breeze |
|-------|---------------------|-----------------|-------------|----------------|-------------|------------|--------------|
| `--brand-primary` | `#ea4f16` | `#C1121F` | `#0D9488` | `#F97316` | `#4D7C0F` | `#B45309` | `#DB2777` |
| `--brand-primary-hover` | `#ffa12e` | `#9B0D17` | `#0F766E` | `#EA580C` | `#3F6212` | `#92400E` | `#BE185D` |
| `--brand-bg` | `#121212` | `#FFFFFF` | `#FFFFFF` | `#0C0C0C` | `#FAFAF5` | `#0A0A0A` | `#FFFBFB` |
| `--brand-surface` | `#1e1e1e` | `#F8F9FA` | `#F8FFFE` | `#1A1A1A` | `#F5F5F0` | `#1A1A1A` | `#FFF5F5` |
| `--brand-text` | `#ffffff` | `#1A1A1A` | `#134E4A` | `#FAFAFA` | `#1A2E05` | `#FEF3C7` | `#1A1A2E` |
| `--brand-text-muted` | `#a8a8a8` | `#6B7280` | `#6B7280` | `#A3A3A3` | `#6B7280` | `#A8A29E` | `#6B7280` |
| `--brand-border` | `#2c2c2c` | `#E5E7EB` | `#CCFBF1` | `#262626` | `#D9F99D` | `#292524` | `#FBCFE8` |

### Semantic Colors (IMMUTABLE across all presets)
```
--color-success: #059669  --color-warning: #D97706  --color-danger: #DC2626  --color-info: #2563EB
```

### Status Colors (IMMUTABLE)
```
PENDING:#D97706  CONFIRMED:#2563EB  PREPARING:#F59E0B  READY:#0D9488  IN_DELIVERY:#3B82F6
DELIVERED:#059669  REJECTED:#DC2626  CANCELLED:#DC2626  SCHEDULED:#7C3AED  PICKED_UP:#0D9488
```

### Dark Mode Override
```css
@media (prefers-color-scheme: dark) {
  :root {
    --brand-bg: #0F172A; --brand-surface: #1E293B; --brand-surface-raised: #263548;
    --brand-text: #F1F5F9; --brand-text-muted: #94A3B8; --brand-border: #334155;
  }
}
```

---

## 2. Typography

| Role | Font | Weight | Size |
|------|------|--------|------|
| Client headings (menu, hero) | DM Serif Display | 400 | 36px mobile / 48px desktop |
| Admin headings | DM Sans | 600 | 18px |
| Body UI | DM Sans | 400 | 14px |
| Small / labels / meta | DM Sans | 400 | 12px |
| Code / keys | JetBrains Mono | 400 | 11px |
| Ocean Fresh headings | Cormorant Garamond | 500 | same sizes |
| Sage/Coral headings | Playfair Display | 500 | same sizes |

### Font stacks
```
--brand-font-heading: 'DM Serif Display', 'Cormorant Garamond', 'Playfair Display', serif
--brand-font-body: 'DM Sans', 'Inter', sans-serif
```

---

## 3. Spacing (4px base unit)

```
--space-0:0  --space-1:4  --space-2:8  --space-3:12  --space-4:16
--space-5:20 --space-6:24 --space-8:32 --space-12:48 --space-16:64
```

Rule: Only these values. Never 6px, 10px, 14px, 18px, 22px, 28px, etc.

---

## 4. Layout

### Grid
- Mobile: single column, 16px gutters
- Tablet (768px): 2 columns
- Desktop (1280px): max-width container, sidebar 240px + content

### Z-index layers
```
--z-dropdown:100  --z-sticky:200  --z-modal-backdrop:300  --z-modal:400  --z-toast:500
```

### Elevation
```
--elevation-1: 0 1px 3px rgba(0,0,0,0.12)
--elevation-2: 0 4px 12px rgba(0,0,0,0.15)
--elevation-3: 0 8px 24px rgba(0,0,0,0.18)
--elevation-4: 0 16px 48px rgba(0,0,0,0.22)
```

---

## 5. Components

### Buttons (4 variants + 4 states each)
| Variant | Background | Text | Border |
|---------|-----------|------|--------|
| Primary | var(--brand-primary) | #fff | none |
| Outline | transparent | var(--brand-primary) | 1px var(--brand-primary) |
| Ghost | transparent | var(--brand-text-muted) | none |
| Danger | var(--color-danger) | #fff | none |

States: default / hover (primary-hover + translateY(-1px)) / active (scale 0.97) / disabled (opacity 0.5)

### Inputs
height: 44px, radius: 8px, border: 1px var(--brand-border)
States: default / focus (brand-primary border + ring) / error (danger border) / disabled

### Toggle
ON: var(--brand-primary) fill, OFF: #D1D5DB
Size: 46x26px track, 20px thumb

### Status Badge
10 variants with colored dot + text label. PENDING and IN_DELIVERY pulse animations.

### ProductCard
4:3 image, name (14px/600), description (12px/muted, 2-line clamp), price (16px/700 primary), add button (32px circle)

### OrderCard
Left 3px status-color border, order #, time, items, total, contextual action buttons

### CartFAB
Fixed bottom-right, 48px height, brand-primary bg, bounce on item add. Hidden in embed mode.

### Admin Sidebar
240px, #0F172A bg (brand, not tenant). Mobile: bottom tab bar.

---

## 6. Motion

### Animation budget
- Card hover: translateY(-2px) + shadow, 0.2s ease
- Button press: scale(0.97), 0.15s
- Page enter: fadeSlideUp, 0.35s ease, stagger 0.05s per child
- Cart bounce: scale bounce, 0.35s
- PENDING pulse: opacity 1→0.4→1, 1.2s infinite

### Reduced motion
```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

---

## 7. Voice

### Albanian market · 77% cash · Mobile-first
- Restaurant names in original language
- Prices: `{amount} ALL` (Albanian Lek, integer)
- Status labels: Albanian (`Në pritje`, `Konfirmuar`, `Në rrugë`, `Dorëzuar`)
- Currency: ALL, no decimals in display, integer storage in DB

### Tone
- Client: warm, inviting, food-premium ("Your order is being prepared")
- Owner: utilitarian, concise ("New order #2301 - 1 806 ALL")
- Courier: direct, action-oriented ("Pickup at Dubin & Sushi")

---

## 8. Brand

### Identity
"SaaS delivery platform for independent restaurants in Albania"
"Your delivery, your customers, your data. Zero commission."

### Visual references
- Client menu: Wolt mobile app (editorial warmth, card spacing)
- Admin: Shopify Admin (information density)
- Courier: Uber Eats Driver (tap targets)

### Three surfaces
1. **Client** (menu, cart, checkout, status): warm, food-premium, DM Serif Display headings
2. **Owner** (dashboard, orders, menu, analytics, settings): utilitarian, dense, DM Sans
3. **Courier** (tasks, delivery, earnings): readable one-handed, large tap targets

---

## 9. Anti-Patterns (RED LINES)

### DO NOT:
- ❌ Write hex colors outside :root / theme classes — use CSS variables ONLY
- ❌ Use cookies — localStorage/sessionStorage only
- ❌ position:fixed in embed mode (?embed=true)
- ❌ Create a new design system / import third-party design tokens
- ❌ Change server contracts / Zod schemas / business logic for UI purposes
- ❌ Use emojis as primary UI elements — use Tabler icons (ti ti-*)
- ❌ Hardcode `rgba(234,79,22,...)` — use var(--brand-primary-light)
- ❌ Skip states — every component needs default/hover/active/disabled
- ❌ Skip dark mode — every screen must have @media (prefers-color-scheme: dark)
- ❌ Generic AI gradients/shadows/purple — use brand tokens
- ❌ Auto-fix token values or add new design systems — flag-only
- ❌ Store PII in localStorage, MessageBus payloads, or audit logs
