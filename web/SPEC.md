# SPEC — Complete Interface

## WHY
Original design concept (`.polish/final/`) defines 30 screens across 3 roles:
- **Customer/Owner**: menu, cart, orders, dashboard, analytics, settings, branding, promotions, couriers, CRM, supplies, activation
- **Courier**: home, tasks, shift, earnings, history
- **Admin**: dashboard, orders, menu, promotions, CRM, supplies, analytics, settings, branding, activation, couriers

Current web/ has:
- `index.html` — original Pizza Roma menu (static, partial)
- Three.js neural field background (integrated in index.html)
- Missing: all admin/courier views, full interactivity, Three.js integration

## WHAT
Complete single-page application that implements ALL designed screens with Three.js WebGPU background, preserving original design language (DM Serif Display, DM Sans, Crimson Classic, Tabler Icons).

## Acceptance
1. All 30 designed screens render correctly at 390px and 1280px
2. Three.js neural field with bloom renders as background
3. Audio sonification: spike events → pentatonic tones
4. Cart + checkout flow works end-to-end
5. Admin dashboard shows real stats (from localStorage)
6. Courier view shows tasks, earnings, shift
7. No CSS framework — pure CSS with design tokens
8. i18n ready with `t()` pattern for sq/en/uk
