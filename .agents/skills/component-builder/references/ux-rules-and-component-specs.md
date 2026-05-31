# DeliveryOS — UX Rules & Component Specs
> Reference for component-builder skill · Read when building any UI element

---

## Global UX rules

### Interaction basics
- Skeleton screens (not spinners) for loading content
- Empty states: always show next-step hint, never blank
- Offline banner with venue phone as fallback — always accessible
- Inline form errors: below the field, not after submit
- Toast notifications: success / error / warning / info (4 types)
- Destructive actions (close venue, reject order): confirmation modal required

### Tap targets
- Standard actions: `min-height: 44px`
- Critical courier actions (Delivered, Picked up): `min-height: 56px`
- Admin quick actions: `min-height: 36px` acceptable

### Transitions
- All animations: wrapped in `@media (prefers-reduced-motion: no-preference)`
- Standard: `transition: all 0.15s ease`
- Hover lift: `transform: translateY(-2px)` + brand-colored shadow
- No generic `opacity: 0.8` hover — use elevation or color shift

---

## Button variants

| Variant | Background | Text | Border | Radius |
|---------|-----------|------|--------|--------|
| **Primary** | `var(--brand-primary)` | white | none | `var(--brand-radius-btn)` = 24px |
| **Outline** | transparent | `var(--brand-primary)` | 1px `var(--brand-primary)` | 24px |
| **Ghost** | transparent | `var(--brand-text-muted)` | none | 8px |
| **Danger** | `var(--color-danger)` | white | none | 24px |
| **Disabled** | any + `opacity: 0.5` | — | — | `cursor: not-allowed; pointer-events: none` |

States for all variants:
- Default
- Hover: `var(--brand-primary-hover)` or darken + `transform: translateY(-1px)`
- Active: `transform: scale(0.98)`
- Disabled: `opacity: 0.5; cursor: not-allowed; pointer-events: none`

---

## Input variants

| State | Border | Background | Note |
|-------|--------|-----------|------|
| Default | 1px `var(--brand-border)` | `var(--brand-bg)` | |
| Focus | 2px `var(--brand-primary)` | white | box-shadow: 0 0 0 3px `var(--brand-primary-light)` |
| Error | 1px `var(--color-danger)` | `#FEF2F2` | error text below field |
| Success | 1px `var(--color-success)` | white | checkmark icon right |
| Disabled | 1px `var(--brand-border)` | `var(--brand-surface)` | `opacity: 0.6` |

Height: 44px. Padding: 0 12px. Border-radius: 8px.

---

## Toggle component

```css
/* ON state */
.toggle-on  { background: var(--brand-primary); }
/* OFF state */
.toggle-off { background: #D1D5DB; }
/* Transition */
transition: background 0.2s ease;
```

Size: 44px × 24px track, 20px thumb.

---

## StatusBadge — 10 variants

| Status | Color | CSS variable | Special |
|--------|-------|-------------|---------|
| PENDING | amber | `var(--status-pending)` | pulse animation on dot |
| CONFIRMED | blue | `var(--status-confirmed)` | |
| PREPARING | amber-warm | `var(--status-preparing)` | |
| READY | teal | `var(--status-ready)` | |
| IN_DELIVERY | blue | `var(--status-in-delivery)` | live dot |
| DELIVERED | green | `var(--color-success)` | |
| REJECTED | red | `var(--color-danger)` | |
| CANCELLED | red | `var(--color-danger)` | |
| SCHEDULED | purple | `var(--status-scheduled)` | |
| PICKED_UP | teal | `var(--status-picked-up)` | |

```css
/* Add to tokens.css */
--status-pending:     #D97706;
--status-confirmed:   #2563EB;
--status-preparing:   #F59E0B;
--status-ready:       #0D9488;
--status-in-delivery: #3B82F6;
--status-scheduled:   #7C3AED;
--status-picked-up:   #0D9488;

/* PENDING pulse animation */
@media (prefers-reduced-motion: no-preference) {
  .status-pending .dot {
    animation: pulse-warning 1.2s ease-in-out infinite;
  }
  @keyframes pulse-warning {
    0%, 100% { opacity: 1; }
    50%       { opacity: 0.4; }
  }
}
```

---

## ProductCard spec

```
┌─────────────────────────────────────┐
│  [PHOTO — 4:3 ratio, radius 6px]    │
├─────────────────────────────────────┤
│  Dish name              DM Sans 14/500  │
│  Description (2 lines)  12px muted   │
│  ────────────────────────────────── │
│  850 ALL ← primary 16/600    [  +  ]│
│  🌾 🥛 🥚 ← allergen pills 10px    │
└─────────────────────────────────────┘
```

**Stop-list variant:** gray overlay `rgba(255,255,255,0.7)` over photo,
badge "Тимчасово недоступно" centered, `+` button disabled `opacity: 0.4`.

**Hover:** `transform: translateY(-2px)`, `box-shadow: 0 8px 24px rgba(var(--brand-primary-rgb), 0.12)`.

**Skeleton:** gray shimmer rectangles in same proportions.

```css
@keyframes shimmer {
  0%   { background-position: -200% 0; }
  100% { background-position:  200% 0; }
}
.skeleton {
  background: linear-gradient(90deg, #f0f0f0 25%, #e8e8e8 50%, #f0f0f0 75%);
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite;
}
```

---

## OrderCard spec

Left border `3px solid var(--status-{state})` — the only visual differentiator by status.

```
┌ 3px status-color border
│  #1847 · "2 хв тому"        ⏱ 08:24 (countdown)
│  ─────────────────────────────────────────────
│  +355 69 XXX XXXX
│  Піца Маргарита ×2, Кола ×1
│  1 850 ALL
│  ─────────────────────────────────────────────
│  [PENDING badge]    [Підтвердити] [Відхилити]
└─────────────────────────────────────────────
```

**PENDING:** timer turns red when < 3 minutes remaining.
**CONFIRMED:** single action "Призначити курʼєра".
**PREPARING:** courier row (avatar + name + status + distance).
**IN_DELIVERY:** address + ETA live badge, no action buttons.

**Animation — new card arrives:**
```css
@media (prefers-reduced-motion: no-preference) {
  .order-card-enter {
    animation: slideInLeft 0.2s ease;
  }
  @keyframes slideInLeft {
    from { transform: translateX(-24px); opacity: 0; }
    to   { transform: translateX(0);     opacity: 1; }
  }
}
```

---

## CartFAB (client menu, full mode only)

```
Fixed bottom: 24px, right: 24px
Height: 48px, padding: 0 20px
Background: var(--brand-primary)
Border-radius: var(--brand-radius-btn) = 24px
Text: white DM Sans 14px
Content: "ti-shopping-cart icon · Кошик · 2 позиції · 1 700 ALL"
```

**Hidden in embed mode** (`?embed=true`).
**Bounce on item add:**
```css
@media (prefers-reduced-motion: no-preference) {
  .cart-fab-bounce { animation: cart-bounce 0.35s ease; }
  @keyframes cart-bounce {
    0%,100% { transform: scale(1); }
    50%     { transform: scale(1.12); }
  }
}
```

---

## CategoryNav (client menu)

```css
.category-nav {
  position: sticky;
  top: 56px; /* below header */
  z-index: 10;
  display: flex;
  gap: 0;
  overflow-x: auto;
  scrollbar-width: none; /* hide scrollbar Firefox */
  background: var(--brand-bg);
  border-bottom: 1px solid var(--brand-border);
}
.category-nav::-webkit-scrollbar { display: none; }

.category-item {
  padding: 12px 16px;
  white-space: nowrap;
  font-size: 14px;
  color: var(--brand-text-muted);
  border-bottom: 2px solid transparent;
  cursor: pointer;
  transition: all 0.15s ease;
}
.category-item.active {
  color: var(--brand-primary);
  border-bottom-color: var(--brand-primary);
}
```

Smooth scroll to section on click: `element.scrollIntoView({ behavior: 'smooth', block: 'start' })`.

---

## Admin sidebar

**Background: `#0F172A` — fixed, NOT a CSS variable. This is DeliveryOS brand, not tenant brand.**

```css
.sidebar {
  width: 240px;
  background: #0F172A;
  height: 100vh;
  position: sticky;
  top: 0;
}

.nav-item {
  height: 48px;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 16px;
  color: rgba(255,255,255,0.7);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s;
}
.nav-item:hover  { background: rgba(255,255,255,0.06); }
.nav-item.active {
  background: rgba(255,255,255,0.1);
  border-left: 3px solid var(--brand-primary);
  color: white;
}
```

Responsive: collapses to 64px icon-only at < 1024px. Hamburger toggle.

---

## QuickStatCard (admin dashboard)

```css
.stat-card {
  background: var(--color-background-primary); /* white */
  border: 1px solid var(--brand-border);
  border-radius: var(--brand-radius);
  padding: 20px;
}
.stat-label { font-size: 13px; color: var(--brand-text-muted); margin-bottom: 4px; }
.stat-value { font-size: 24px; font-weight: 500; color: var(--brand-text); }
.stat-trend { font-size: 12px; color: var(--color-success); } /* green for positive */
```

---

## Allergen pills

```html
<div class="allergens">
  <span class="allergen"><i class="ti ti-wheat"></i> Глютен</span>
  <span class="allergen"><i class="ti ti-droplet"></i> Молоко</span>
  <span class="allergen"><i class="ti ti-egg"></i> Яйця</span>
</div>
```

```css
.allergen {
  font-size: 10px;
  padding: 2px 7px;
  border-radius: 8px;
  background: var(--brand-surface);
  color: var(--brand-text-muted);
  display: inline-flex;
  align-items: center;
  gap: 3px;
}
```

---

## Theme switcher (required on all screens)

```html
<button id="theme-btn" onclick="cycleTheme()"
  style="position:fixed;top:16px;right:16px;z-index:9999;
         background:var(--brand-surface);border:1px solid var(--brand-border);
         border-radius:20px;padding:6px 14px;font-size:12px;
         color:var(--brand-text);cursor:pointer;
         box-shadow:0 2px 8px rgba(0,0,0,0.1)">
  🎨 Тема
</button>

<script>
const PRESETS = {
  'Crimson Classic': {
    '--brand-primary': '#C1121F', '--brand-primary-hover': '#9B0D17',
    '--brand-primary-light': '#FFF0F1', '--brand-accent': '#F5F0E8',
    '--brand-bg': '#FFFFFF', '--brand-surface': '#F8F9FA',
    '--brand-text': '#1A1A1A', '--brand-text-muted': '#6B7280',
    '--brand-border': '#E5E7EB', '--brand-radius': '12px', '--brand-radius-btn': '24px',
    '--brand-font-heading': "'DM Serif Display', serif",
    '--brand-font-body': "'DM Sans', sans-serif"
  },
  'Ocean Fresh': {
    '--brand-primary': '#0D9488', '--brand-primary-hover': '#0A7567',
    '--brand-primary-light': '#F0FDFA', '--brand-accent': '#F0F9F8',
    '--brand-bg': '#FFFFFF', '--brand-surface': '#F8FFFE',
    '--brand-text': '#134E4A', '--brand-text-muted': '#6B7280',
    '--brand-border': '#CCFBF1', '--brand-radius': '24px', '--brand-radius-btn': '32px',
    '--brand-font-heading': "'Cormorant Garamond', serif",
    '--brand-font-body': "'DM Sans', sans-serif"
  },
  'Midnight Urban': {
    '--brand-primary': '#F97316', '--brand-primary-hover': '#EA6C0A',
    '--brand-primary-light': '#FFF7ED', '--brand-accent': '#1C1917',
    '--brand-bg': '#0C0A09', '--brand-surface': '#1C1917',
    '--brand-text': '#FAFAF9', '--brand-text-muted': '#A8A29E',
    '--brand-border': '#292524', '--brand-radius': '4px', '--brand-radius-btn': '4px',
    '--brand-font-heading': "'DM Sans', sans-serif",
    '--brand-font-body': "'DM Sans', sans-serif"
  }
};

let idx = 0;
const keys = Object.keys(PRESETS);
function cycleTheme() {
  idx = (idx + 1) % keys.length;
  const p = PRESETS[keys[idx]];
  const r = document.documentElement.style;
  Object.entries(p).forEach(([k, v]) => r.setProperty(k, v));
  document.getElementById('theme-btn').textContent = '🎨 ' + keys[idx];
  localStorage.setItem('dos_theme', keys[idx]);
}
// Restore on load
const saved = localStorage.getItem('dos_theme');
if (saved && PRESETS[saved]) {
  idx = keys.indexOf(saved);
  const p = PRESETS[saved];
  const r = document.documentElement.style;
  Object.entries(p).forEach(([k, v]) => r.setProperty(k, v));
  document.getElementById('theme-btn').textContent = '🎨 ' + saved;
}
</script>
```
