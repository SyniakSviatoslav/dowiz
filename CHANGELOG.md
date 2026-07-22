# Changelog

All notable changes to the dowiz kernel + product are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/) + CalVer `YYYY.MM.PATCH`.

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
