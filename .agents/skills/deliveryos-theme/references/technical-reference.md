# DeliveryOS — Technical & Architecture Reference
> v2.0 · Agent reference document · Read when: implementing logic, DB schema, API routes, or white-label system

---

## 1. Business economics

| Clients | MRR | Infra cost | Net/month |
|---------|-----|------------|-----------|
| 2 | $56 | $25 | +$31 |
| 13 | $364 | $25 | +$39 (breakeven) |
| 120 (6 mo) | $3,360 | $50 | ~$3,010 |
| 230 (1% TAM) | $6,440 | $80 | ~$5,180 (after 15% VAT) |
| 500 (3.5%) | $14,000 | $150 | ~$11,050 |

---

## 2. White-label architecture decisions

| Decision | Rationale |
|----------|-----------|
| Curated presets (not full editor) | 5–8 ready themes → 20% work gives 80% value. GloriaFood confirmed: no churn from missing full editor. |
| White-label only on client side | Courier = internal tool. Admin = B2B. Logo + primary color shown everywhere via CSS vars. |
| CSS Custom Properties everywhere | No hardcoded colors in components. Tailwind config via `var(--brand-*)`. |
| Pre-approved Radix Colors palette | Not hex input — choice from Radix Colors palettes. WCAG contrast auto-check. |
| iframe + postMessage for embed | Shadow DOM Web Component deferred. iframe simpler, more reliable, easier to maintain. |
| Pull-based Google Sheets | CSV endpoint with API key, owner imports themselves. Zero OAuth tokens on our side. |
| Custom domain: architecture now, implementation post-MVP | `custom_domain` field in locations + Host middleware in first migration. |
| No cookies — only localStorage/sessionStorage | Cross-domain iframe cookies blocked by Safari. All cart state in localStorage. |

---

## 3. CSS Variables — complete spec

```css
:root {
  /* Colors — all via CSS variables */
  --brand-primary:        #E63946;  /* buttons, accents */
  --brand-primary-hover:  #C1121F;  /* hover state */
  --brand-primary-light:  #FFF0F1;  /* accent backgrounds */
  --brand-accent:         #F1FAEE;  /* secondary accent */
  --brand-bg:             #FFFFFF;  /* page background */
  --brand-surface:        #F8F9FA;  /* card background */
  --brand-text:           #1D1D1D;  /* primary text */
  --brand-text-muted:     #6C757D;  /* secondary text */
  --brand-border:         #DEE2E6;  /* borders */

  /* Typography */
  --brand-font-heading:   'Inter', system-ui, sans-serif;
  --brand-font-body:      'Inter', system-ui, sans-serif;

  /* Shape */
  --brand-radius:         12px;  /* sharp:0 | rounded:12px | pill:24px */
  --brand-radius-sm:      6px;
}

/* Tailwind — all colors via variables */
theme: { extend: { colors: {
  primary:            'var(--brand-primary)',
  'primary-hover':    'var(--brand-primary-hover)',
  accent:             'var(--brand-accent)',
  surface:            'var(--brand-surface)',
} } }
```

**RULE: Zero hardcoded colors in any component. Only `var(--brand-*)`. Violation = refactor.**

---

## 4. How theme technically works

| Step | Detail |
|------|--------|
| 1. Owner saves theme | `PATCH /api/v1/location/:id/theme` → Zod validation → save to `location_themes` |
| 2. CSS file generation | BullMQ job → generates static CSS file → uploads to Cloudflare CDN → updates `css_hash` |
| 3. SSR render `/s/:slug` | Fastify reads location + theme → injects `<link rel=stylesheet href=/cdn/themes/{id}/{hash}.css>` → zero DB queries for theme |
| 4. Admin + Courier PWA | `GET /api/v1/location/:id/brand` → CSS variables → React: `document.documentElement.style.setProperty(...)` |
| 5. Cache invalidation | `css_hash` changes on every theme update → Cloudflare auto-serves new file |

---

## 5. Database schema

### Core tables

**`organizations`** → `id, name, owner_id → users`

**`locations`** → `id, org_id, name, address, delivery_radius_km, lat/lng, phone, status, menu_version, custom_domain (NULL until custom domain MVP), widget_enabled`

**`location_themes`** (separate table — NOT jsonb in locations):
```sql
id, location_id, preset_id, logo_url, cover_url, favicon_url,
primary_color, accent_color, background_color, text_color,
font_heading, font_body,
border_radius,    -- 'sharp' | 'rounded' | 'pill'
menu_layout,      -- 'grid' | 'list'
hero_style,       -- 'cover' | 'minimal' | 'none'
powered_by_hidden,  -- Business tier only
custom_css,         -- Business tier only, DOMPurify sanitized
admin_logo_url,     -- Business tier only
admin_primary_color, -- Business tier only
css_hash, css_generated_at
```

**`categories`** → `id, location_id, name, position`

**`products`** → `id, category_id, name_al, name_en, description_al, description_en, price, image_url, is_available, unavailable_until, allergens text[], calories, nutrition jsonb, voice_aliases text[], position`

**`users`** → `id, email, role (owner/courier/admin), created_at`

**`couriers`** → `id, location_id, user_id, name, phone, status (online/offline/busy)`

**`customers`** → `id, phone (normalized, UNIQUE), name, created_at`

**`orders`** → `id, location_id, customer_id, courier_id, type, status (enum), rejection_reason, delivery_address, delivery_lat/lng, total, timeout_at, scheduled_at, pickup_code, payment_method, payment_status, courier_sequence, created_at, confirmed_at`

**`order_items`** → `id, order_id, product_id, name (snapshot), quantity, price_snapshot`

**`promotions`** → `id, location_id, type (8 types), code, discount_value, rules jsonb, min_order, max_uses_total, is_active`

### White-label tables

**`webhook_endpoints`** (Pro+):
```sql
id, location_id, url, events text[], secret (HMAC), is_active, failure_count, last_triggered_at
```

**`api_keys`** (Business):
```sql
id, location_id, key_hash (SHA256, raw key never stored), key_prefix,
name, scopes text[], last_used_at, expires_at
```

**`domain_verifications`** (architecture ready, post-MVP implementation):
```sql
id, location_id, domain, cname_target, verified_at, ssl_status
```

### TimescaleDB hypertables

| Table | Chunk | Purpose |
|-------|-------|---------|
| `courier_locations` | 1 day | `courier_id, order_id, lat, lng, accuracy, ts` |
| `delivery_traces` | — | `order_id, trace jsonb, distance_km, duration_min` |
| `order_metrics` | 7 days | `location_id, order_count, revenue, avg_delivery_min, ts` |
| `order_ratings` | 7 days | `order_id, food_score, delivery_score, comment, ts` |
| `ai_usage_log` | 1 month | `location_id, feature, tokens_in, tokens_out, cost_usd` |

---

## 6. Full tech stack

| Layer | Technology | Reason |
|-------|-----------|--------|
| Backend runtime | Node.js + Fastify + TypeScript | Fast start, WebSocket native, BullMQ in same process |
| Database | Supabase (TimescaleDB + PG 15) | Managed + TimescaleDB + RLS + auto-backups |
| Cache / Queue | Redis (4 clients) | BullMQ(db0), cache+idempotency(db1), pub(db2), sub(db2) |
| Queue | BullMQ | Timeouts, scheduled orders, CSS generation |
| Realtime | WebSocket own (ws) | <50ms latency. Supabase Realtime — DISABLED |
| Validation | Zod (.strict() everywhere) | Shared schemas between API and web |
| Auth | JWT 15min + refresh rotation 7d | RBAC: owner / courier / admin |
| Frontend | React 18 + TypeScript (PWA) | One codebase, three routes |
| SSR (public) | Fastify + Vite SSR | SEO for `/s/:slug`, themes via CSS variables |
| State | Zustand + TanStack Query | Local state + server state |
| Component library | shadcn/ui + Tailwind CSS | Headless, customization via CSS variables |
| Animations | Motion (Framer Motion) | AnimatePresence for order statuses |
| Tables | TanStack Table v8 | CRM, analytics, virtualization |
| Charts | Recharts | Revenue, courier stats, heatmap |
| Drag & Drop | dnd-kit | Menu order, floor plans (post-MVP) |
| Maps | MapLibre GL + OSM | Free, WebGL animations for geo |
| GPS (courier) | navigator.geolocation + Wake Lock | PWA everywhere |
| Storage (client) | localStorage + sessionStorage | NO cookies. Safari 3rd party cookies blocked |
| Email | Resend | Confirmation, receipt ($0 to 3k/mo) |
| Geocoding | Geocode.maps.co | Free to 1M requests/mo |
| Payments MVP | Cash on delivery | Zero integration |
| Payments MVP+ | 2Checkout (Verifone) | Confirmed for Albania |
| AI features | Claude API (Sonnet 4) | Dish descriptions, menu analysis, chat assistant, voice |
| Theme CDN | Cloudflare CDN | Static CSS theme files, zero DB queries on render |
| Deploy | Fly.io (backend) + Supabase | Compute separate from storage |
| Security CDN | Cloudflare | DDoS, WAF, rate limiting, SSL, routing |
| Monorepo | pnpm workspaces | apps/api + apps/web + packages/shared-types + packages/ui |

---

## 7. Architecture — one Fastify server

```
/api/*              → Fastify routes (WebSocket, BullMQ, everything)
/s/:slug            → Fastify route → Vite SSR → CSS variables from brand_config
/s/:slug?embed=true → embed mode (no fixed, no WS, localStorage cart)
/admin/*            → Fastify static → React SPA
/courier/*          → Fastify static → React SPA
```

Host header middleware: if `Host != deliveryos.com` → find location by `custom_domain`
(currently always DeliveryOS domain, logic ready for custom domain post-MVP)

---

## 8. Security rules

| Area | Measure |
|------|---------|
| SQL Injection | Parameterized queries ONLY. Never string concat. |
| Input validation | Zod `.strict()` on all schemas. `brand_config`: Zod + WCAG contrast check. |
| Auth | JWT 15min + refresh token rotation 7d. RBAC: owner/courier/admin. |
| Tenant isolation | Supabase RLS from first migration. Owner A cannot see orders B (404 not 403). |
| Geo data | Courier geo only accessible to participants of active order. |
| UUID generation | `crypto.randomUUID()` — Node.js crypto. Never `Math.random()`. |
| Webhook verify | `crypto.timingSafeEqual()` for HMAC signatures. Never `===`. |
| Custom CSS | DOMPurify on server for `custom_css` field (Business tier). |
| Embed CORS | `Access-Control-Allow-Origin: *` only for embed routes. |
| API keys | SHA256 hash stored, raw key shown once on creation. |
| Cookies | NOT USED. Exclusively localStorage/sessionStorage. |
| Security headers | @fastify/helmet: CSP, HSTS, noSniff, xssFilter. |
| Rate limiting | Cloudflare WAF + @fastify/rate-limit + WS geo rate limit (1 msg/3s). |

---

## 9. Project structure (monorepo)

```
deliveryos/
├── apps/
│   ├── api/                    # Fastify backend
│   └── web/                    # React PWA (all three roles)
├── packages/
│   ├── shared-types/           # TypeScript types + Zod schemas
│   ├── ui/                     # Shared component library (CSS variables everywhere)
│   └── config/                 # ESLint, TypeScript, Tailwind (with brand vars)
├── migrations/                 # node-pg-migrate
└── docs/

apps/api/src/modules/
├── orders/       (routes, service, schema, state-machine, timeout.worker)
├── menu/         (routes, service, menu-version)
├── couriers/     (routes, service, geo.service)
├── locations/    (routes, service, hours.service)
├── customers/    (routes, service)
├── analytics/    (routes, service — TimescaleDB)
├── ai/           (assistant, menu-analysis, price-intel, tips, voice)
├── promotions/   (routes, service, combo.service)
├── branding/     (routes, service, theme-renderer, css-generator.worker)
├── domains/      (routes, service — architecture, post-MVP implementation)
├── webhooks/     (routes, service, delivery.worker — BullMQ)
├── api-keys/     (routes, service, middleware)
└── integrations/ (export.service — CSV/JSON pull endpoints)

apps/web/src/routes/
├── customer/     (MenuPage, CartPage, CheckoutPage, OrderStatusPage, EmbedMenuPage)
├── admin/        (Dashboard, Orders, Menu, Couriers, Analytics, CRM,
│                  Settings, Settings/Branding, AIAssistant, Promotions)
└── courier/      (TasksPage, ActiveDeliveryPage)
```

---

## 10. GPS and geo-streaming (courier)

```
Courier PWA → watchPosition every 5s → WS → Redis SET courier:geo:{id} EX 30 → broadcast → MapLibre GL

Validation rules:
- accuracy > 100m      → reject position (before sending)
- speed > 150 km/h     → reject position
- Heartbeat every 15s, 30s without geo → 'position unavailable'
- Wake Lock API        → screen stays on during active delivery
- Page Visibility API  → warning if app goes to background
- GPS permission check on shift start → blocking modal if denied
```

---

## 11. Circuit breaker matrix

| Service | Criticality | On failure |
|---------|-------------|-----------|
| PostgreSQL | CRITICAL | 503 — order not created |
| Redis (BullMQ) | CRITICAL | 503 — timeout not guaranteed |
| Redis (geo cache) | NON-CRITICAL | Map without courier position |
| Resend email | NON-CRITICAL | Silent log + retry later |
| Geocode.maps.co | NON-CRITICAL | Address as text |
| WebSocket push | NON-CRITICAL | Client sees on reconnect |
| Claude API | NON-CRITICAL | AI features degrade gracefully |
| 2Checkout | NON-CRITICAL | Fallback to cash |
| CSS CDN (themes) | NON-CRITICAL | Fallback to default DeliveryOS theme |

---

## 12. Idempotency and storage

```
Idempotency: every POST → X-Idempotency-Key header (UUID on client)
Server stores key in Redis db1 with TTL 24 hours

Client-side storage — ONLY localStorage/sessionStorage, NEVER cookies:
- Cart state:       localStorage  (persistent between sessions)
- JWT access token: sessionStorage (cleared on tab close)
- Refresh token:    localStorage + httpOnly via API endpoint
- Embed cart:       localStorage  prefix 'dos_embed_{locationId}_'
- Theme cache:      sessionStorage (until next open)
```

---

## 13. Embed widget spec

```html
<!-- Owner inserts on their website: -->
<script src='https://cdn.deliveryos.com/widget/v1.js'></script>
<delivery-widget location='sushi-durres' lang='al'></delivery-widget>

<!-- Or via iframe directly: -->
<iframe src='https://order.deliveryos.com/sushi-durres?embed=true'
  style='width:100%;border:none;' id='dos-widget'></iframe>
<script>
  window.addEventListener('message', (e) => {
    if (e.data.type === 'dos:height') {
      document.getElementById('dos-widget').style.height = e.data.height + 'px';
    }
  });
</script>
```

**Embed mode technical constraints:**
- `?embed=true`: no `position: fixed` (iOS Safari bug)
- No sticky elements — "Add to cart" button becomes inline
- Cart state: localStorage (not cookies — Safari blocks 3rd party cookies in iframe)
- postMessage for dynamic iframe height resize
- Separate bundle: no unnecessary JS (no MapLibre, no WS for embed)
- CORS: widget endpoint allows any Origin

---

## 14. Google Sheets integration (pull-based, no OAuth)

```
GET /api/v1/export/orders?key={api_key}&format=csv&from=2024-01-01
GET /api/v1/export/customers?key={api_key}&format=csv
GET /api/v1/export/analytics?key={api_key}&format=json&period=30d

In Google Sheets: Data → Import → URL → paste link
Auto-refresh: Sheet → Extensions → Apps Script → setInterval hourly
Zero OAuth tokens. Zero Google API on our server. Full simplicity.
```

---

## 15. Branding settings page — three tabs

| Tab | Contents |
|-----|---------|
| **Appearance** | Logo/cover/favicon upload, preset picker (5–8 with preview), color from Radix Colors palette (not hex input), border radius (sharp/rounded/pill), menu layout (grid/list). Right: live mobile preview with real menu data. |
| **Domain & Embed** | Current URL status (subdomain), CNAME instruction (Business, visible but inactive until MVP launch), embed code copy-paste, QR poster with menu link. |
| **Integrations** | Google Sheets pull endpoint with API key (generate/revoke), copy URL for import, Test connection button, Webhooks list (Pro+), CSV/JSON export buttons, Zapier/Make integration instructions via webhook URL. |

---

## 16. Promotions system — 8 types

Types: percentage discount, fixed discount, free item, buy-X-get-Y, combo, happy hour, promo code, minimum order discount.

Key components:
- Custom rule builder (jsonb rules) via `RuleBuilder` UI
- Happy hour: auto-activation/deactivation via BullMQ schedule
- Combo editor with substitutes

---

## 17. AI features (Claude API — Sonnet 4)

All AI features degrade gracefully when Claude API is unavailable.

| Feature | Description |
|---------|-------------|
| Dish description generation | Streaming, from name + category + optional photo |
| Menu analysis | Weak positions, price intelligence, competitor comparison |
| Chat assistant | Owner asks questions about their business data |
| Voice order input | Web Speech API + Claude parsing |
| Proactive tips | Daily tips based on analytics patterns |
| Opening checklist | AI-generated daily prep list |

Usage logged to `ai_usage_log` TimescaleDB hypertable.
Cost: ~$0.84/client/month at normal usage.

---

## 18. Infrastructure costs (~$25–40/month)

| Service | Cost | Detail |
|---------|------|--------|
| Fly.io (backend + Redis) | ~$15 | shared-cpu + managed Redis |
| Supabase Pro | $25 | PostgreSQL + TimescaleDB + backups + pooler |
| Cloudflare | $0 | Free tier + themes CDN (static CSS files) |
| Cloudflare for SaaS | $0 now | Custom domains: $0.10/hostname — activates post-MVP |
| Sentry | $0 | Free: 5k errors/month |
| UptimeRobot | $0 | 3 monitors: API + Web + DB health |
| Resend | $0 | Free: 3k emails/month |
| Geocode.maps.co | $0 | Free: 1M requests/month |
| Claude API | ~$1–5 | ~$0.84/client/month at active use |
| Radix Colors | $0 | Open source. Pre-approved palettes for theme presets. |

---

## 19. Architecture Decision Records (key decisions)

| ADR | Decision | Rationale |
|-----|----------|-----------|
| 001 | Monolith | One Fastify process. Microservices = premature optimization. |
| 003 | PWA everywhere | One React codebase. iOS GPS tested in practice. |
| 006 | Own WebSocket | Supabase Realtime DISABLED. <50ms latency for geo. |
| 010 | Redis 4 clients | DB0 BullMQ, DB1 cache+idempotency, DB2 pub+sub (SEPARATE clients!) |
| 014 | Curated presets | No full editor. GloriaFood confirmed: no churn without it. 20% work = 80% value. |
| 015 | location_themes separate table | Typed columns, versioning, theme analytics. Not jsonb blob. |
| 016 | CSS → static file → CDN | SSR links `/cdn/themes/{id}/{hash}.css`. Zero DB queries for theme on render. |
| 017 | iframe + postMessage | Shadow DOM Web Component deferred. iframe simpler, iOS-compatible. |
| 018 | No cookies | Exclusively localStorage/sessionStorage. Safari blocks 3rd party cookies in iframe. |
| 019 | Pull-based Google Sheets | CSV endpoint with API key. Zero OAuth. Owner imports via 'Import from URL'. |
| 020 | Radix Colors | Open source, WCAG-verified palettes. Not hex input — pre-approved choice. |
| 021 | Custom domain architecture now | Field + Host middleware + domain_verifications in first migration. Post-MVP implementation. |
| 022 | White-label client side only | Courier = internal tool. Admin = B2B. Logo+primary color everywhere via CSS vars. |

---

## 20. Critical reminders (read before every session)

```
✗ Supabase direct connection (5432) for migrations and analytics pool
✗ Redis Pub/Sub: SEPARATE clients for pub and sub
✗ Supabase Realtime — DISABLED
✓ Zod .strict() on ALL input schemas including brand_config
✓ crypto.randomUUID() (Node.js), not a library
✓ timingSafeEqual for webhook signature verification
✓ CSS variables: var(--brand-*) everywhere, ZERO hardcoded colors
✓ ZERO cookies — exclusively localStorage/sessionStorage
✓ location_themes — separate table, NOT jsonb in locations
✓ CSS generation: BullMQ worker → Cloudflare CDN → css_hash
✓ Embed mode: no fixed positioning, localStorage cart, postMessage height
✓ Google Sheets: pull endpoint, NOT OAuth push
✓ Custom domain: field exists in DB, Host middleware exists, Cloudflare for SaaS — post-MVP
```
