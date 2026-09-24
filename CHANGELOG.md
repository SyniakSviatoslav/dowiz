# Changelog

All notable changes to the dowiz kernel + product are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/) + CalVer `YYYY.MM.PATCH`.

## [2026.09.0] — 2026-09-24

The first tagged release since 2026.07.0 (2026.07.3 was never tagged). Full notes, grouped as
features, fixes, security and infrastructure: [`docs/releases/v2026.09.0.md`](docs/releases/v2026.09.0.md).
Tags carry a `v` prefix from this release on (`release.yml` publishes a GitHub Release for `v*`).

### Added
- A live restaurant system on Cloudflare: one Worker, one Durable Object per venue holding its
  records as bebop images, four apps (storefront, owner console, room app, courier app) in
  Albanian, English and Ukrainian.
- Guests: delivery and pickup with one pricer, kernel-computed waiting time, live tracking, table QR
  ordering into the table's open bill, bookings without an account, recorded marketing consent.
- Owner: orders and refunds, menu import with preview, stock ledger with recipes, cost and waste,
  staff roles, bookings and floor plan, customer cards with masked contacts and forget-in-place,
  consented WhatsApp campaigns, exceptions report, analytics in the venue's time zone, health pane,
  integrations with "prove it" checks, the venue as an MCP server.
- Room: six table states, rounds, amend in intent form, split payments in two currencies with tips,
  the till with a blind count; amend and pay decided in wasm, byte-identical to the server.
- Courier: one job at a time, offline taps replayed with an idempotency key, refused-at-the-door.
- eBills import (read direction only); the fiscal sender is built and switched off.
- Gates with mutation proofs and `tools/gates/run-all.sh`; the conservation audit; per-role live
  walks; the four-reader bebop parity gate.
- CI: `ci.yml` rewritten for the live product, `health-cron.yml`, `key-flows.yml`, `mutations.yml`,
  `release.yml`, Dependabot; issue and PR templates, CODEOWNERS.
- Documentation: README, `docs/architecture.md`, `docs/testing.md`, `docs/operations.md`,
  `docs/code-quality.md`, `docs/wiki/`.

### Changed
- D1 and all SQL removed; each venue's records live in its own object.
- The Worker reads the clock once per request; each handler writes one image.
- Nightly copies are kept 21 days (7 daily, then weekly).

### Fixed
- One pricer instead of two; a retried order placed once; a replayed courier tap answered, not
  refused; the venue's own midnight instead of a summer constant; append logs grow instead of
  refusing; truncated images refused; money formatted by one formatter everywhere; stylesheets no
  longer dropped by the CSP.

### Security
- Two unauthenticated write routes deleted and three route families closed (`727bc591`); 37 owner
  routes that acted on another venue fixed and gated; an order id alone no longer reads an order.

## [2026.07.3] — 2026-07-22

### Added
- Full UX flow audit and fix pass (25 friction points identified, 12 P0-P3 fixed)
- 8-layer improvement: notifications, polling, mobile UX, transitions, role confirm, cart feedback, shift metrics, real analytics
- Telemetry suite: oracle.mjs, markov.mjs, vitals.mjs, health.mjs, telegram.mjs
- Markov Friction Predictor (4A): freeze detection at 4× expected time, inline hints
- Invisible Assistant (4C): behavioral adaptation via state timing tracking
- ETA display on order detail and courier tasks
- Owner menu management: edit price, hide/show items, add new items
- Address/phone validation with live field highlighting
- Security: CSP meta tag, sanitize() helper, rateLimit() decorator
- Accessibility: aria attributes, aria-live toast, role/tablist/tab nav
- Keyboard shortcuts: k (courier), t (theme), r (reload)
- Dynamic seed data: generateSeedOrders(), generateSeedTasks(), generateSeedHistory()
- Error boundary in renderContent() with reload button
- Battery optimization: pause Three.js + SDF loops on page hidden

### Fixed
- Checkout no longer claims success before API responds
- Courier actions now sync _orders.status (pickup→in-delivery, deliver→done)
- Owner order chain complete through delivered
- Orders, courierTasks, earnings now persist across sessions
- Earnings history populated from actual deliveries
- Cart quantity +/- buttons added
- Stale CONTEXT-INDEX.md and MEMORY-MAP.md rewritten
- DeliveryOS-As-Built-Summary-v1.md archived
- CONVENTIONS.md rewritten as plain text
- Cart no longer auto-opens on every add
- Analytics timeline uses real order data, not Math.random()

## [2026.07.0] — 2026-07-18

### Added
- `KERNEL_PROTO_VERSION` in-code wire version constant (kernel/src/lib.rs).
- Fail-closed drift gate: NaN/±inf + ragged (index-leak) operators rejected as
  `Unstable` before indexing (`classify_drift` + `Mat::from_vecvec_checked`).
- `CompensatedRefund` FSM compensation edge with mandatory ledger reversal
  (money nets to exactly zero; no un-reversed refund).
- `order_from_in` server-authoritative subtotal/total recompute (forged client
  total cannot survive a fold — E1 closed).
- Resource caps on untrusted-JSON `_js` entry points (Box::leak OOM, harmonic
  `n`, payload, log bounds).
- `compute_order_total` / `apply_tax` overflow-safe (checked_add/checked_mul).
