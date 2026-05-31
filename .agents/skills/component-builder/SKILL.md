---
name: component-builder
description: >
  Use this skill when building individual UI components for DeliveryOS:
  ProductCard, OrderCard, StatusBadge, CartFAB, CategoryNav, Toggle,
  Button variants, Input states, SkeletonCard, or any reusable UI element.
  Trigger phrases: "component", "card", "button", "badge", "build a",
  "create a", "ProductCard", "OrderCard", "atom", "molecule".
---

# DeliveryOS Component Builder

## Goal
Produce self-contained HTML+CSS component snippets that use only CSS variables
from tokens.css and follow DeliveryOS interaction patterns.

## Component anatomy rules
Every component must have:
1. Default state — visible, functional
2. Hover state — CSS :hover with transition 0.15s ease
3. Active/pressed state — transform scale(0.98)
4. Disabled state — opacity 0.5, pointer-events none
5. Dark mode — works automatically via CSS variable inheritance

## ProductCard spec
```html
<article class="product-card" data-available="true">
  <div class="product-img">
    <!-- 4:3 ratio, object-fit cover, radius var(--brand-radius-sm) -->
  </div>
  <div class="product-body">
    <h3 class="product-name"><!-- DM Sans 14px/500 --></h3>
    <p class="product-desc"><!-- 12px muted, 2-line clamp --></p>
    <div class="product-footer">
      <span class="product-price"><!-- var(--brand-primary) 16px/600 --></span>
      <button class="btn-add"><!-- + pill 32px --></button>
    </div>
    <div class="allergens"><!-- 10px pills --></div>
  </div>
</article>
<!-- StopList variant: add class "unavailable" → gray overlay + disabled btn -->
```

## OrderCard spec
Left border 3px color = var(--status-{state}).
Always contains: order number + timestamp + items summary + total + action buttons.
Timer for PENDING: red text when < 3 minutes remaining.

## Button variants
Primary:   bg var(--brand-primary), white text, radius var(--brand-radius-btn)
Outline:   border 1px var(--brand-primary), var(--brand-primary) text, transparent bg
Ghost:     transparent bg+border, var(--brand-text-muted) text
Danger:    bg var(--color-danger), white text
Disabled:  opacity 0.5, cursor not-allowed, pointer-events none

## StatusBadge spec
10 variants — use CSS classes: .status-pending .status-confirmed etc.
PENDING gets @keyframes pulse on the color dot.

## Animation rules
- Wrap all in @media (prefers-reduced-motion: no-preference)
- CartFAB bounce: scale 1→1.12→1 on item add
- New OrderCard: slideInLeft 0.2s ease
- PENDING badge pulse: opacity 1→0.4→1 1.2s infinite
- Skeleton shimmer: background-position 200%→-200% 1.5s infinite

## Reference implementations
See examples/ folder for ProductCard.html and OrderCard.html.

## Constraints
- No inline style colors — only CSS variables or Tailwind utilities that map to vars
- Component file = snippet only, no full HTML document
- Always provide the stop-list / disabled variant alongside default
---
