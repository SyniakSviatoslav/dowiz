# DeliveryOS — Product Context
> v3.0 · Agent reference document · Read when: building any screen, component, or logic

---

## 1. What DeliveryOS is

SaaS platform for independent food businesses to run their own delivery channel.
**Not a marketplace. Not an aggregator.** The restaurant owns the customer relationship.

**Core pitch:** "Connect your own delivery in 10 minutes — no Glovo, no Poster, no programmer.
Your delivery, your customers, your data. Zero commission."

---

## 2. Three roles

| Role | What they do | Interface |
|------|-------------|-----------|
| **Client** | Orders food via link or iframe. Installs nothing. | `/s/:slug` PWA |
| **Owner** | Manages menu, orders, couriers, analytics | `/admin/` SPA |
| **Courier** | Receives tasks, runs active delivery | `/courier/` PWA |

---

## 3. Market — Albania as entry point

| Metric | Value | Why it matters |
|--------|-------|----------------|
| Total venues | ~15,300 | #1 per capita globally |
| Real TAM | ~5,000–6,000 | Food venues with delivery potential |
| Month 1 target | 50 venues | Clients already lined up |
| Cash on delivery | 77% | Online payment is secondary |
| Mobile share | 70.88% | Mobile-first is mandatory |
| Growth | 12.7% annual | |

Scale path: Kosovo → North Macedonia → Bosnia. Same model, just localization.

---

## 4. Pricing

| Plan | Price | Couriers | Key features |
|------|-------|----------|-------------|
| **Starter** | $19/mo | up to 2 | Orders, menu, basic analytics, CRM |
| **Pro** ★ | $39/mo | up to 5 | Full analytics, geo-tracking, AI features, webhooks |
| **Business** | $59/mo | unlimited | Multi-location, API access, full white-label, custom domain |

Rules: zero % on transactions (subscription only), 14-day trial no card, owner pays 2Checkout fees directly.

---

## 5. Product principles (immutable)

| Principle | Detail |
|-----------|--------|
| Tool for owner | Not marketplace. Owner gets FULL access to all their data. |
| Zero friction for client | Client installs nothing. Link → order. |
| Confirmation mandatory | No order exists without venue confirmation. |
| Reliability via minimalism | Fewer moving parts = less that breaks at 19:00. |
| Fallback to contacts | If platform is down → client sees venue phone. |
| Simplicity = less support | Every extra feature is a potential call at 19:00. |

---

## 6. Onboarding — 8 steps, ~10-14 min total

Route: `/onboarding` (separate from settings, Shopify Setup Guide pattern)

| Step | Name | Time |
|------|------|------|
| 1 | Venue (name, phone, address) | 2 min |
| 2 | First dish (name, price, photo optional) | 3 min |
| 3 | First courier (name, phone) | 1 min |
| 4 | Delivery zone (polygon draw or radius km) | 2 min |
| 5 | Branding (logo + color, skippable) | 2 min |
| 6 | Preview (3 screens: owner / client / courier) | 1 min |
| 7 | Link and script (copy iframe or share link) | 1 min |
| 8 | Test order (owner places order, sees full flow) | 2 min |

Requirements: always-visible progress indicator, skip option per optional step,
save-and-continue-later, inline hints (not separate docs),
after test order → dashboard with real data (not empty state).

---

## 7. Order types (MVP)

- `delivery` — standard flow with courier
- `pickup` — self-pickup, no courier, `pickup_code` generated
- `scheduled_delivery` — delivery at chosen time (timeslots)

---

## 8. Order state machine — 10 statuses

| Status | Who transitions | Trigger | Auto |
|--------|----------------|---------|------|
| `PENDING` | System | POST /orders | BullMQ timeout job starts |
| `CONFIRMED` | Owner | confirm action | Timeout cancelled |
| `PREPARING` | Owner | assign courier | Push to courier |
| `READY` | Owner | ready action | WS push to courier |
| `IN_DELIVERY` | Courier | pickup action | Courier number → client |
| `DELIVERED` | Courier | deliver action | Analytics write, trace saved |
| `REJECTED` | Owner | reject + reason | WS push → client |
| `CANCELLED` | System (BullMQ) | Timeout after N min | WS push → client |
| `SCHEDULED` | System | Scheduled order | BullMQ activation 15 min before |
| `PICKED_UP` | Owner | pickup (pickup type only) | — |

**Key flow details:**
- Confirmation timeout: 5/10/15/30 min, configurable by owner
- Busy mode: timeout ×2, venue status unchanged
- On rejection → ask "add dish to stop-list?"
- Every POST → `X-Idempotency-Key` header (prevents double orders)
- 30 min after DELIVERED → BullMQ job → feedback form
- Status page stays active 7 days after DELIVERED
- Reorder: check `is_available` + prices before confirming

---

## 9. White-label system — two UI layers

| Layer | What it is | Brand |
|-------|-----------|-------|
| **Owner Layer** | Admin panel, settings, analytics | DeliveryOS brand. Utilitarian dense UI. Ref: Shopify Admin + Stripe. |
| **Tenant Layer** | Client menu, embed widget | Fully controlled by restaurant CSS variables. Ref: Wolt. |

### CSS Variables — Rule #1 (NEVER violate)

```css
:root {
  --brand-primary:        #C1121F;
  --brand-primary-hover:  #9B0D17;
  --brand-primary-light:  #FFF0F1;
  --brand-accent:         #F5F0E8;
  --brand-bg:             #FFFFFF;
  --brand-surface:        #F8F9FA;
  --brand-text:           #1A1A1A;
  --brand-text-muted:     #6B7280;
  --brand-border:         #E5E7EB;
  --brand-font-heading:   'DM Serif Display', serif;
  --brand-font-body:      'DM Sans', sans-serif;
  --brand-radius:         12px;
  --brand-radius-sm:      6px;
  --brand-radius-btn:     24px;  /* pill buttons */

  /* Semantic — NEVER override per theme */
  --color-success:  #059669;
  --color-warning:  #D97706;
  --color-danger:   #DC2626;
  --color-info:     #2563EB;
}
```

**Zero hardcoded colors in any component. Only `var(--brand-*)`. Violation = refactor.**

---

## 10. Theme presets (6 curated)

| Preset | Primary | Character |
|--------|---------|-----------|
| **Crimson Classic** | `#C1121F` | Restaurant, pizza, meat — default |
| **Ocean Fresh** | `#0D9488` | Seafood, healthy food |
| **Midnight Urban** | `#F97316` | Burgers, street food (dark bg) |
| **Sage Garden** | `#4D7C0F` | Vegetarian, organic |
| **Royal Gold** | `#B45309` | Premium, sushi (dark bg) |
| **Coral Breeze** | `#DB2777` | Desserts, café |

Switching theme = replacing `:root` CSS variable values only. Zero DOM changes.

---

## 11. White-label by plan

| Feature | Starter | Pro | Business |
|---------|---------|-----|----------|
| Logo + venue name | ✓ | ✓ | ✓ |
| Primary color (Radix palette) | ✗ | ✓ | ✓ |
| Preset themes (5–8) | ✗ | ✓ | ✓ |
| Rebrand client page | ✗ | ✓ | ✓ |
| Rebrand admin + courier | ✗ | ✗ | ✓ |
| Custom domain | ✗ | ✗ | ✓ (post-MVP) |
| Embeddable widget (iframe) | ✗ | ✗ | ✓ |
| Hide "Powered by DeliveryOS" | ✗ | ✗ | ✓ |
| Custom CSS override | ✗ | ✗ | ✓ (DOMPurify sanitized) |
| REST API + API keys | ✗ | ✗ | ✓ |
| Google Sheets pull endpoint | ✗ | ✓ | ✓ |
| CSV / JSON export | ✓ | ✓ | ✓ |

---

## 12. Embed widget constraints

`?embed=true` activates embed mode. Critical differences from normal mode:

- **No `position: fixed`** — iOS Safari bug with fixed elements in iframes
- **No `sticky` elements** — cart button becomes inline, not floating
- **Cart state**: `localStorage` with prefix `dos_embed_{locationId}_` (never cookies)
- **postMessage** → parent page adjusts iframe height dynamically
- **CORS**: `Access-Control-Allow-Origin: *` for embed routes only
- **Separate lighter bundle**: no MapLibre, no WebSocket

---

## 13. Storage rules (NEVER use cookies)

```
Cart state:          localStorage  (persists between sessions)
JWT access token:    sessionStorage (cleared on tab close)
Checkout state:      sessionStorage (between steps)
Embed cart:          localStorage  prefix 'dos_embed_{locationId}_'
Theme cache:         sessionStorage
```

Safari blocks 3rd party cookies in iframes — this is why no cookies anywhere.

---

## 14. UX rules by role

### All screens
- Skeleton screens instead of spinners for loading content
- Empty states: not blank screens, show next-step hints
- Offline banner with venue phone as fallback (always visible)
- Inline form errors (under the field, not after submit)
- Toast notifications: success / error / warning / info (shadcn Sonner)

### Admin — during peak hours
- Sound alert for new order (iOS audio context fix required)
- Critical alerts non-dismissible
- Busy mode: timeout ×2, status unchanged
- LocationStatusToggle: large and visible, confirmation modal on close
- OrderCard: pulse animation for PENDING, slide-in for new order

### Courier interface
- Map takes majority of screen, UI as bottom sheet on top
- One main CTA button at a time
- Minimum font size: 16px for anything read while moving
- Wake Lock API: screen stays on during active delivery
- Page Visibility API: warning if app goes to background
- GPS: reject if accuracy > 100m or speed > 150km/h
- Sound + vibration on new task

### Client page
- Cart summary bar at bottom (not FAB) — Wolt pattern
- DeliveryZoneGate: check zone before showing menu
- ClosedOverlay: if venue closed, show phone + hours
- StopListBadge: gray overlay + reason (don't just hide the dish)

---

## 15. Design references by role

| Context | Primary reference | What to copy |
|---------|------------------|-------------|
| Owner admin | Shopify Admin | Orders list, Products, Home dashboard, Onboarding checklist |
| Owner admin | Square Dashboard | QuickStats layout for offline businesses |
| Orders realtime | Deliverect | Order management kanban |
| Courier | Uber Eats Driver | Button sizes 56px+, map dominates, one action per screen |
| Client menu | Wolt | CategoryNav, ProductCard, Cart summary bar bottom |
| Analytics | Stripe Dashboard | Information density, chart style, metric cards |

---

## 16. WebSocket rooms

```
order:{id}      → client + owner + courier (order status updates)
location:{id}   → owner (all active couriers on map)
courier:{id}    → courier (their tasks)
```

Auth: first message `{ type: 'auth', token: jwtToken }`. Server verifies JWT.
No auth within 5s → auto-close with 1008.

---

## 17. Technology stack (relevant for mockup)

**Frontend:**
- React 18 + TypeScript (PWA, single codebase, three routes)
- shadcn/ui + Tailwind CSS (headless, customized via CSS variables)
- Motion (Framer Motion) for AnimatePresence on order statuses
- MapLibre GL + OSM (free, WebGL geo animations)
- TanStack Table v8 (CRM, analytics)
- Recharts (revenue charts, heatmap)
- dnd-kit (menu drag-and-drop order)

**Key architectural decisions:**
- One Fastify process: API + WebSocket + BullMQ + Vite SSR
- No cookies anywhere — localStorage/sessionStorage only
- CSS → static file → Cloudflare CDN (zero DB queries for theme on render)
- Embed mode: separate lighter bundle, no MapLibre, no WS

---

## 18. Priority build order

1. `src/screens/01-menu-client.html` — client menu `/s/:slug`
2. `src/screens/02-admin-dashboard.html` — owner dashboard `/admin/`
3. `src/screens/03-branding-settings.html` — branding `/admin/settings/branding`
4. `src/screens/04-courier-delivery.html` — courier `/courier/delivery/:id`
5. `src/screens/05-admin-menu.html` — menu management `/admin/menu`
