# Evals · nightly · 2026-09-26 · 85b1d1d7-dirty

Deployed version: UNVERIFIED (no route or token reports the deployed version id). Hosts: https://sushi-durres.dowiz.org, https://dubin-sushi.dowiz.org. 180 indicators, 15 breaches, 16 unverified. Run 67 s on node v22.22.1.

## Breaches

| Indicator | Value | Rule | Why |
|---|---|---|---|
| `live.root.ttfb_ms` | 125 ms | plus25 | 125 > 72 x 1.25 — status 200; cf-cache-status HIT |
| `live.root.total_ms` | 139 ms | plus25 | 139 > 81 x 1.25 |
| `health.sushi_durres.unheld` | 1 orders | zero | 1 != 0 |
| `health.dubin_sushi.verdict_ok` | 0 bool | min | 0 < limit 1 — verdict watch |
| `health.dubin_sushi.worst_used_permille` | 844 permille | max | 844 > limit 800 |
| `health.dubin_sushi.image.catalog.used_permille` | 844 permille | max | 844 > limit 800 |
| `ux.store.fcp_ms` | 1,172 ms | plus25 | 1172 > 796 x 1.25 |
| `ux.store.csp_violations` | 2 violations | zero | 2 != 0 — directives: script-src-elem |
| `ux.admin.csp_violations` | 2 violations | zero | 2 != 0 — directives: script-src-elem |
| `ux.room.boot_wire_bytes` | 185,711 bytes | ratchet | 185711 > baseline 185607 |
| `ux.room.csp_violations` | 2 violations | zero | 2 != 0 — directives: script-src-elem |
| `ux.courier.boot_wire_bytes` | 136,415 bytes | ratchet | 136415 > baseline 136386 |
| `ux.courier.csp_violations` | 2 violations | zero | 2 != 0 — directives: script-src-elem |
| `cost.cron_share_permille` | 842 permille | max | 842 > limit 500 |
| `cost.do_measured_over_modelled_permille` | 1,740 permille | max | 1740 > limit 1200 |

## Unverified

- `order.api_requests` — traffic.mjs places a real order and has no clean-up for a walk that stops half way; run it by hand against sushi-durres and pass EVALS_TRAFFIC_JSON (source: e2e/evals/traffic.mjs)
- `order.api_bytes` — traffic.mjs places a real order and has no clean-up for a walk that stops half way; run it by hand against sushi-durres and pass EVALS_TRAFFIC_JSON (source: e2e/evals/traffic.mjs)
- `order.p95_ms` — traffic.mjs places a real order and has no clean-up for a walk that stops half way; run it by hand against sushi-durres and pass EVALS_TRAFFIC_JSON (source: e2e/evals/traffic.mjs)
- `order.cells_per_order` — traffic.mjs places a real order and has no clean-up for a walk that stops half way; run it by hand against sushi-durres and pass EVALS_TRAFFIC_JSON (source: e2e/evals/traffic.mjs)
- `platform.total_bytes` — answered 404: needs a platform administrator's token (PLATFORM_EMAIL/PLATFORM_PASSWORD) and the H5 route deployed (source: GET /api/platform/health)
- `platform.errors` — answered 404: platform administrators only (source: GET /api/platform/errors)
- `product.sushi_durres.eta_bias_s` — the ETA backtest needs the promised time beside DELIVERED; the order record carries `eta` on too few orders and no promise timestamp (source: GET /api/owner/orders (sushi_durres), folded to distributions)
- `product.dubin_sushi.ready_to_in_delivery.p50_s` — no value: 0 orders (source: GET /api/owner/orders (dubin_sushi), folded to distributions)
- `product.dubin_sushi.ready_to_in_delivery.p90_s` — no value (source: GET /api/owner/orders (dubin_sushi), folded to distributions)
- `product.dubin_sushi.in_delivery_to_delivered.p50_s` — no value: 0 orders (source: GET /api/owner/orders (dubin_sushi), folded to distributions)
- `product.dubin_sushi.in_delivery_to_delivered.p90_s` — no value (source: GET /api/owner/orders (dubin_sushi), folded to distributions)
- `product.dubin_sushi.created_to_delivered.p50_s` — no value: 0 orders (source: GET /api/owner/orders (dubin_sushi), folded to distributions)
- `product.dubin_sushi.created_to_delivered.p90_s` — no value (source: GET /api/owner/orders (dubin_sushi), folded to distributions)
- `product.dubin_sushi.eta_bias_s` — the ETA backtest needs the promised time beside DELIVERED; the order record carries `eta` on too few orders and no promise timestamp (source: GET /api/owner/orders (dubin_sushi), folded to distributions)
- `product.dubin_sushi.booking_arrived_permille` — no value (source: seated or completed / created)
- `cost.plan` — Free vs Paid is not in the analytics dataset the token reads (source: Cloudflare dashboard)

## cf

| Indicator | Value | Unit | Baseline | Δ | Rule | Status | Note |
|---|---:|---|---:|---:|---|---|---|
| `cf.worker_requests_day` | 2,333 | requests | — |  | trend | ok | 12106 subrequests |
| `cf.worker_errors_day` | 0 | errors | — |  | zero | ok |  |
| `cf.cpu_p50_us` | 7,739 | µs | 7,807 | -68 | plus25 | ok |  |
| `cf.cpu_p99_us` | 69,549 | µs | 60,736 | +8,813 | plus25 | ok | CPU per invocation (not memory: no analytics dataset reports isolate memory) |
| `cf.do_requests_day` | 11,899 | requests | — |  | trend | ok |  |
| `cf.do_errors_day` | 0 | errors | — |  | zero | ok |  |
| `cf.do_response_bytes_day` | 169,210,314 | bytes | — |  | trend | ok |  |

## cost

| Indicator | Value | Unit | Baseline | Δ | Rule | Status | Note |
|---|---:|---|---:|---:|---|---|---|
| `cost.modelled_worker_requests_day` | 2,521 | requests | — |  | trend | ok |  |
| `cost.modelled_do_requests_day` | 6,840 | requests | — |  | trend | ok |  |
| `cost.cron_share_permille` | 842 | permille | — |  | max 500 | **breach** | 842 > limit 500 |
| `cost.plan` | — | plan | — |  | trend | **unverified** |  |
| `cost.marginal_venue_month_micro_usd` | 37,272 | µ$ | — |  | max 1,000,000 | ok |  |
| `cost.all_in_venue_month_micro_usd` | 3,003,939 | µ$ | — |  | trend | ok |  |
| `cost.logs_month` | 6,999 | lines | — |  | max 20,000,000 | ok |  |
| `cost.do_measured_over_modelled_permille` | 1,740 | permille | — |  | max 1,200 | **breach** | 1740 > limit 1200 |

## health

| Indicator | Value | Unit | Baseline | Δ | Rule | Status | Note |
|---|---:|---|---:|---:|---|---|---|
| `health.sushi_durres.verdict_ok` | 1 | bool | — |  | min 1 | ok | verdict ok |
| `health.sushi_durres.errors` | 0 | records | 0 | 0 | ratchet | ok | {} |
| `health.sushi_durres.outbox.waiting` | 0 | messages | — |  | trend | ok |  |
| `health.sushi_durres.outbox.oldest_ms` | 0 | ms | — |  | max 600,000 | ok |  |
| `health.sushi_durres.outbox.failing` | 0 | messages | — |  | zero | ok |  |
| `health.sushi_durres.quarantined` | 0 | records | — |  | zero | ok |  |
| `health.sushi_durres.stranded` | 0 | orders | — |  | zero | ok |  |
| `health.sushi_durres.unheld` | 1 | orders | — |  | zero | **breach** | 1 != 0 |
| `health.sushi_durres.witness_intact` | 1 | bool | — |  | min 1 | ok |  |
| `health.sushi_durres.backup_sealed` | 1 | bool | — |  | min 1 | ok | X25519+ML-KEM-768 / AES-256-GCM, dwzseal v1 |
| `health.sushi_durres.rails_open` | 0 | rails | — |  | zero | ok |  |
| `health.sushi_durres.ebills.failures` | 0 | count | — |  | zero | ok |  |
| `health.sushi_durres.kitchen_unseen` | 1 | orders | — |  | trend | ok |  |
| `health.sushi_durres.events` | 322 | events | — |  | trend | ok |  |
| `health.sushi_durres.worst_used_permille` | 521 | permille | — |  | max 800 | ok |  |
| `health.sushi_durres.worst_growing_permille` | 967 | permille | — |  | trend | ok |  |
| `health.sushi_durres.image.catalog.used_cells` | 67,854 | cells | — |  | trend | ok | 521‰ of 130048 (fixed) |
| `health.sushi_durres.image.catalog.used_permille` | 521 | permille | — |  | max 800 | ok |  |
| `health.sushi_durres.image.log.used_cells` | 17,285 | cells | — |  | trend | ok | 544‰ of 31744 (grows) |
| `health.sushi_durres.image.log.used_permille` | 544 | permille | — |  | trend | ok |  |
| `health.sushi_durres.image.posts.used_cells` | 38 | cells | — |  | trend | ok | 0‰ of 64512 (fixed) |
| `health.sushi_durres.image.posts.used_permille` | 0 | permille | — |  | max 800 | ok |  |
| `health.sushi_durres.image.settings.used_cells` | 570 | cells | — |  | trend | ok | 17‰ of 31744 (fixed) |
| `health.sushi_durres.image.settings.used_permille` | 17 | permille | — |  | max 800 | ok |  |
| `health.sushi_durres.image.stock.used_cells` | 6,933 | cells | — |  | trend | ok | 967‰ of 7168 (grows) |
| `health.sushi_durres.image.stock.used_permille` | 967 | permille | — |  | trend | ok |  |
| `health.dubin_sushi.verdict_ok` | 0 | bool | — |  | min 1 | **breach** | 0 < limit 1 |
| `health.dubin_sushi.errors` | 0 | records | 0 | 0 | ratchet | ok | {} |
| `health.dubin_sushi.outbox.waiting` | 0 | messages | — |  | trend | ok |  |
| `health.dubin_sushi.outbox.oldest_ms` | 0 | ms | — |  | max 600,000 | ok |  |
| `health.dubin_sushi.outbox.failing` | 0 | messages | — |  | zero | ok |  |
| `health.dubin_sushi.quarantined` | 0 | records | — |  | zero | ok |  |
| `health.dubin_sushi.stranded` | 0 | orders | — |  | zero | ok |  |
| `health.dubin_sushi.unheld` | 0 | orders | — |  | zero | ok |  |
| `health.dubin_sushi.witness_intact` | 1 | bool | — |  | min 1 | ok |  |
| `health.dubin_sushi.backup_sealed` | 1 | bool | — |  | min 1 | ok | X25519+ML-KEM-768 / AES-256-GCM, dwzseal v1 |
| `health.dubin_sushi.rails_open` | 0 | rails | — |  | zero | ok |  |
| `health.dubin_sushi.ebills.failures` | 0 | count | — |  | zero | ok |  |
| `health.dubin_sushi.kitchen_unseen` | 0 | orders | — |  | trend | ok |  |
| `health.dubin_sushi.events` | 234 | events | — |  | trend | ok |  |
| `health.dubin_sushi.worst_used_permille` | 844 | permille | — |  | max 800 | **breach** | 844 > limit 800 |
| `health.dubin_sushi.worst_growing_permille` | 822 | permille | — |  | trend | ok |  |
| `health.dubin_sushi.image.catalog.used_cells` | 109,875 | cells | — |  | trend | ok | 844‰ of 130048 (fixed) |
| `health.dubin_sushi.image.catalog.used_permille` | 844 | permille | — |  | max 800 | **breach** | 844 > limit 800 |
| `health.dubin_sushi.image.log.used_cells` | 137,055 | cells | — |  | trend | ok | 524‰ of 261120 (grows) |
| `health.dubin_sushi.image.log.used_permille` | 524 | permille | — |  | trend | ok |  |
| `health.dubin_sushi.image.posts.used_cells` | 19 | cells | — |  | trend | ok | 0‰ of 64512 (fixed) |
| `health.dubin_sushi.image.posts.used_permille` | 0 | permille | — |  | max 800 | ok |  |
| `health.dubin_sushi.image.settings.used_cells` | 169 | cells | — |  | trend | ok | 5‰ of 31744 (fixed) |
| `health.dubin_sushi.image.settings.used_permille` | 5 | permille | — |  | max 800 | ok |  |
| `health.dubin_sushi.image.stock.used_cells` | 5,897 | cells | — |  | trend | ok | 822‰ of 7168 (grows) |
| `health.dubin_sushi.image.stock.used_permille` | 822 | permille | — |  | trend | ok |  |

## live

| Indicator | Value | Unit | Baseline | Δ | Rule | Status | Note |
|---|---:|---|---:|---:|---|---|---|
| `live.root.ttfb_ms` | 125 | ms | 72 | +53 | plus25 | **breach** | 125 > 72 x 1.25 |
| `live.root.total_ms` | 139 | ms | 81 | +58 | plus25 | **breach** | 139 > 81 x 1.25 |
| `live.root.status_ok` | 1 | bool | — |  | min 1 | ok | status 200 |
| `live.file.store_css.ttfb_ms` | 47 | ms | 77 | -30 | plus25 | ok | status 200; cf-cache-status HIT |
| `live.file.store_css.total_ms` | 76 | ms | 93 | -17 | plus25 | ok |  |
| `live.file.store_css.status_ok` | 1 | bool | — |  | min 1 | ok | status 200 |
| `live.file.icons_css.ttfb_ms` | 59 | ms | 53 | +6 | plus25 | ok | status 200; cf-cache-status HIT |
| `live.file.icons_css.total_ms` | 68 | ms | 58 | +10 | plus25 | ok |  |
| `live.file.icons_css.status_ok` | 1 | bool | — |  | min 1 | ok | status 200 |
| `live.file.i18n_js.ttfb_ms` | 41 | ms | 67 | -26 | plus25 | ok | status 200; cf-cache-status HIT |
| `live.file.i18n_js.total_ms` | 51 | ms | 71 | -20 | plus25 | ok |  |
| `live.file.i18n_js.status_ok` | 1 | bool | — |  | min 1 | ok | status 200 |
| `live.file.ui_css.ttfb_ms` | 49 | ms | 43 | +6 | plus25 | ok | status 200; cf-cache-status HIT |
| `live.file.ui_css.total_ms` | 54 | ms | 45 | +9 | plus25 | ok |  |
| `live.file.ui_css.status_ok` | 1 | bool | — |  | min 1 | ok | status 200 |
| `live.file.booking_js.ttfb_ms` | 42 | ms | 70 | -28 | plus25 | ok | status 200; cf-cache-status HIT |
| `live.file.booking_js.total_ms` | 49 | ms | 73 | -24 | plus25 | ok |  |
| `live.file.booking_js.status_ok` | 1 | bool | — |  | min 1 | ok | status 200 |
| `live.file.state_js.ttfb_ms` | 40 | ms | 48 | -8 | plus25 | ok | status 200; cf-cache-status HIT |
| `live.file.state_js.total_ms` | 45 | ms | 50 | -5 | plus25 | ok |  |
| `live.file.state_js.status_ok` | 1 | bool | — |  | min 1 | ok | status 200 |
| `live.boot_files_revalidating` | 6 | files | 6 | 0 | ratchet | ok |  |
| `live.api.menu.ttfb_ms` | 41 | ms | 79 | -38 | plus25 | ok | status 200; cf-cache-status HIT |
| `live.api.menu.total_ms` | 50 | ms | 87 | -37 | plus25 | ok |  |
| `live.api.menu.status_ok` | 1 | bool | — |  | min 1 | ok | status 200 |
| `live.api.menu_fresh.ttfb_ms` | 108 | ms | 162 | -54 | plus25 | ok | status 200; cf-cache-status - |
| `live.api.menu_fresh.total_ms` | 125 | ms | 172 | -47 | plus25 | ok |  |
| `live.api.menu_fresh.status_ok` | 1 | bool | — |  | min 1 | ok | status 200 |
| `live.api.manifest.ttfb_ms` | 74 | ms | 100 | -26 | plus25 | ok | status 200; cf-cache-status - |
| `live.api.manifest.total_ms` | 86 | ms | 109 | -23 | plus25 | ok |  |
| `live.api.manifest.status_ok` | 1 | bool | — |  | min 1 | ok | status 200 |
| `live.api.consent.ttfb_ms` | 34 | ms | 47 | -13 | plus25 | ok | status 200; cf-cache-status - |
| `live.api.consent.total_ms` | 38 | ms | 51 | -13 | plus25 | ok |  |
| `live.api.consent.status_ok` | 1 | bool | — |  | min 1 | ok | status 200 |
| `live.api.rates.ttfb_ms` | 39 | ms | 73 | -34 | plus25 | ok | status 200; cf-cache-status HIT |
| `live.api.rates.total_ms` | 45 | ms | 88 | -43 | plus25 | ok |  |
| `live.api.rates.status_ok` | 1 | bool | — |  | min 1 | ok | status 200 |
| `live.api.menu.edge_hit` | 1 | bool | — |  | min 1 | ok | cf-cache-status HIT |
| `live.photos.products` | 77 | products | — |  | trend | ok |  |
| `live.photos.small_permille` | 0 | permille | 0 | 0 | floor | ok |  |
| `live.photos.mean_bytes` | 140,979 | bytes | 140,979 | 0 | ratchet | ok |  |

## order

| Indicator | Value | Unit | Baseline | Δ | Rule | Status | Note |
|---|---:|---|---:|---:|---|---|---|
| `order.api_requests` | — | requests | — |  | ratchet | **unverified** |  |
| `order.api_bytes` | — | bytes | — |  | ratchet | **unverified** |  |
| `order.p95_ms` | — | ms | — |  | plus25 | **unverified** |  |
| `order.cells_per_order` | — | cells | — |  | ratchet | **unverified** |  |

## platform

| Indicator | Value | Unit | Baseline | Δ | Rule | Status | Note |
|---|---:|---|---:|---:|---|---|---|
| `platform.total_bytes` | — | bytes | — |  | ratchet | **unverified** |  |
| `platform.errors` | — | records | — |  | ratchet | **unverified** |  |

## product

| Indicator | Value | Unit | Baseline | Δ | Rule | Status | Note |
|---|---:|---|---:|---:|---|---|---|
| `product.sushi_durres.orders_7d` | 63 | orders | — |  | trend | ok | {"storefront":59,"console":3,"wolt":1} |
| `product.sushi_durres.created_to_confirmed.p50_s` | 10 | s | — |  | trend | ok | 35 orders |
| `product.sushi_durres.created_to_confirmed.p90_s` | 124 | s | 124 | 0 | plus25 | ok |  |
| `product.sushi_durres.confirmed_to_preparing.p50_s` | 1 | s | — |  | trend | ok | 34 orders |
| `product.sushi_durres.confirmed_to_preparing.p90_s` | 4 | s | — |  | trend | ok |  |
| `product.sushi_durres.preparing_to_ready.p50_s` | 3 | s | — |  | trend | ok | 33 orders |
| `product.sushi_durres.preparing_to_ready.p90_s` | 9,604 | s | — |  | trend | ok |  |
| `product.sushi_durres.ready_to_in_delivery.p50_s` | 2 | s | — |  | trend | ok | 27 orders |
| `product.sushi_durres.ready_to_in_delivery.p90_s` | 40,292 | s | — |  | trend | ok |  |
| `product.sushi_durres.in_delivery_to_delivered.p50_s` | 0 | s | — |  | trend | ok | 28 orders |
| `product.sushi_durres.in_delivery_to_delivered.p90_s` | 16 | s | — |  | trend | ok |  |
| `product.sushi_durres.created_to_delivered.p50_s` | 44 | s | — |  | trend | ok | 28 orders |
| `product.sushi_durres.created_to_delivered.p90_s` | 42,745 | s | — |  | trend | ok |  |
| `product.sushi_durres.abandoned` | 4 | orders | 4 | 0 | ratchet | ok |  |
| `product.sushi_durres.rejected_7d` | 9 | orders | — |  | trend | ok |  |
| `product.sushi_durres.eta_coverage_permille` | 0 | permille | — |  | trend | ok |  |
| `product.sushi_durres.eta_bias_s` | — | s | — |  | trend | **unverified** |  |
| `product.sushi_durres.bookings_7d` | 7 | bookings | — |  | trend | ok | {"DECLINED":3,"CANCELLED_BY_GUEST":2,"CANCELLED_BY_VENUE":2} |
| `product.sushi_durres.booking_arrived_permille` | 0 | permille | — |  | trend | ok |  |
| `product.dubin_sushi.orders_7d` | 1 | orders | — |  | trend | ok | {"storefront":1} |
| `product.dubin_sushi.created_to_confirmed.p50_s` | 17 | s | — |  | trend | ok | 1 orders |
| `product.dubin_sushi.created_to_confirmed.p90_s` | 17 | s | 17 | 0 | plus25 | ok |  |
| `product.dubin_sushi.confirmed_to_preparing.p50_s` | 4 | s | — |  | trend | ok | 1 orders |
| `product.dubin_sushi.confirmed_to_preparing.p90_s` | 4 | s | — |  | trend | ok |  |
| `product.dubin_sushi.preparing_to_ready.p50_s` | 4 | s | — |  | trend | ok | 1 orders |
| `product.dubin_sushi.preparing_to_ready.p90_s` | 4 | s | — |  | trend | ok |  |
| `product.dubin_sushi.ready_to_in_delivery.p50_s` | — | s | — |  | trend | **unverified** | 0 orders |
| `product.dubin_sushi.ready_to_in_delivery.p90_s` | — | s | — |  | trend | **unverified** |  |
| `product.dubin_sushi.in_delivery_to_delivered.p50_s` | — | s | — |  | trend | **unverified** | 0 orders |
| `product.dubin_sushi.in_delivery_to_delivered.p90_s` | — | s | — |  | trend | **unverified** |  |
| `product.dubin_sushi.created_to_delivered.p50_s` | — | s | — |  | trend | **unverified** | 0 orders |
| `product.dubin_sushi.created_to_delivered.p90_s` | — | s | — |  | trend | **unverified** |  |
| `product.dubin_sushi.abandoned` | 1 | orders | 1 | 0 | ratchet | ok |  |
| `product.dubin_sushi.rejected_7d` | 0 | orders | — |  | trend | ok |  |
| `product.dubin_sushi.eta_coverage_permille` | 0 | permille | — |  | trend | ok |  |
| `product.dubin_sushi.eta_bias_s` | — | s | — |  | trend | **unverified** |  |
| `product.dubin_sushi.bookings_7d` | 0 | bookings | — |  | trend | ok | {} |
| `product.dubin_sushi.booking_arrived_permille` | — | permille | — |  | trend | **unverified** |  |

## ux

| Indicator | Value | Unit | Baseline | Δ | Rule | Status | Note |
|---|---:|---|---:|---:|---|---|---|
| `ux.store.fcp_ms` | 1,172 | ms | 796 | +376 | plus25 | **breach** | 1172 > 796 x 1.25 |
| `ux.store.boot_requests` | 49 | requests | 49 | 0 | ratchet | ok |  |
| `ux.store.boot_wire_bytes` | 259,295 | bytes | 259,300 | -5 | ratchet | **improved** | lowered 259300 -> 259295 |
| `ux.store.console_errors` | 2 | errors | 2 | 0 | ratchet | ok | Executing inline script violates the following Content Security Policy directive 'script-src 'self' https://js.stripe.co |
| `ux.store.page_errors` | 0 | errors | — |  | zero | ok |  |
| `ux.store.csp_violations` | 2 | violations | — |  | zero | **breach** | 2 != 0 |
| `ux.store.contrast_failures` | 31 | elements | 31 | 0 | ratchet | ok | approximate (not axe): of 689 text elements |
| `ux.admin.fcp_ms` | 1,156 | ms | 1,844 | -688 | plus25 | ok |  |
| `ux.admin.boot_requests` | 33 | requests | 33 | 0 | ratchet | ok |  |
| `ux.admin.boot_wire_bytes` | 249,619 | bytes | 251,122 | -1,503 | ratchet | **improved** | lowered 251122 -> 249619 |
| `ux.admin.console_errors` | 2 | errors | 2 | 0 | ratchet | ok | Executing inline script violates the following Content Security Policy directive 'script-src 'self' https://js.stripe.co |
| `ux.admin.page_errors` | 0 | errors | — |  | zero | ok |  |
| `ux.admin.csp_violations` | 2 | violations | — |  | zero | **breach** | 2 != 0 |
| `ux.admin.contrast_failures` | 1 | elements | 1 | 0 | ratchet | ok | approximate (not axe): of 10 text elements |
| `ux.room.fcp_ms` | 468 | ms | 496 | -28 | plus25 | ok |  |
| `ux.room.boot_requests` | 44 | requests | 44 | 0 | ratchet | ok |  |
| `ux.room.boot_wire_bytes` | 185,711 | bytes | 185,607 | +104 | ratchet | **breach** | 185711 > baseline 185607 |
| `ux.room.console_errors` | 2 | errors | 2 | 0 | ratchet | ok | Executing inline script violates the following Content Security Policy directive 'script-src 'self' https://js.stripe.co |
| `ux.room.page_errors` | 0 | errors | — |  | zero | ok |  |
| `ux.room.csp_violations` | 2 | violations | — |  | zero | **breach** | 2 != 0 |
| `ux.room.contrast_failures` | 0 | elements | 0 | 0 | ratchet | ok | approximate (not axe): of 8 text elements |
| `ux.courier.fcp_ms` | 420 | ms | 408 | +12 | plus25 | ok |  |
| `ux.courier.boot_requests` | 32 | requests | 32 | 0 | ratchet | ok |  |
| `ux.courier.boot_wire_bytes` | 136,415 | bytes | 136,386 | +29 | ratchet | **breach** | 136415 > baseline 136386 |
| `ux.courier.console_errors` | 2 | errors | 2 | 0 | ratchet | ok | Executing inline script violates the following Content Security Policy directive 'script-src 'self' https://js.stripe.co |
| `ux.courier.page_errors` | 0 | errors | — |  | zero | ok |  |
| `ux.courier.csp_violations` | 2 | violations | — |  | zero | **breach** | 2 != 0 |
| `ux.courier.contrast_failures` | 0 | elements | 0 | 0 | ratchet | ok | approximate (not axe): of 12 text elements |
