# Backend without UI — route audit, 2026-09-26

Read-only audit of `HEAD` = `5fa49ea6` ("stock: an ingredient nobody has counted never refuses an order").
Committed code only (`git show HEAD:`); the working tree's half-split `crates/dowiz-hub` was not read.
Operator rule under test: "усе що є на бекенді, має бути підключено, видно та мати змогу використовуватись на UI також".

Method: every `.get/.post/.put/.delete(_async)` in `workers/api/src/lib.rs` (192 registrations: 190 with a
named handler, `/healthz` and `/api/order/:id` as closures), matched against every literal, template and
helper-built path in `workers/api/public/**` (`admin/core.js api()/post()` → `/api` + path; `room/net.js`;
`courier/app.js`; `kit/data.js call()` → `/api/public/locations/<slug>` + path; `store/state.js API`;
`platform/app.js`; `lib/money.js loadRates`). Guards come from the handler files (`Place::of_authorised`
= owner; `courier::staff_at(Cap)` = owner OR staff holding the cap; `principal_at` = any principal of the
venue; customer = order/booking token). Rows marked (unverified) were not driven live.

## 1. Summary

| Class | Count | Notes |
|---|---:|---|
| USED — a screen calls it | 170 | includes 7 "USED, but only from the owner console" where the natural user is staff (§2.2) and 3 half-orphans (rows 12, 14, 15: the guest side is wired, the venue side is not) |
| MACHINE-ONLY | 9 | webhooks (3), printer daemon (3), MCP RPC, bootstrap, health probe |
| ORPHAN — no UI caller at all | 13 | 1 covered by W-INV (`stock/waste`), 1 to hand to W-KITCHEN (`graph`), 11 open; §2.1 lists them with the 3 half-orphans (15 rows) |
| **Total registered** | **192** | 170 + 9 + 13 |

Beyond routes: 8 declared settings keys have no console control (§4.1), 1 hub module is dead
(`stock/cost.rs`, W-INV wires it), 3 hub modules have no worker caller (§4.2), and one platform-wide
switch (`fiscal::SEND_ENABLED = false`) is a compile-time constant no UI can flip (§4.3).

Not a route problem but found on the way: the kit's `add-money` screen (`kit/data.js:252 topUp`) calls
`POST …/wallet/topup`, which since the red-team pass is **owner-only** (`wallet.rs:274`, 403 for a
customer) — the screen exists, the door does not open, and the owner has no screen for it either.

## 2. ORPHANS, ranked by value to a real restaurant owner

Ranking: what an owner of Dubin Sushi / Sushi Durrës would miss first.

### 2.1 Open orphans (nobody is building these)

| # | Route | Who should use it | Screen / control | Size |
|---|---|---|---|---|
| 1 | `GET/POST /api/public/locations/:slug/threads/:id[/messages]` — **venue side** | waiter (room), owner | The customer writes from the kit chat (`kit/screens/chat.js`); `party.rs` admits staff with Advance/TakeOrders and the owner, but **no venue screen reads or answers** the order thread. Needs a "Messages" tab on the order sheet in `/room` and `/admin/orders.js`, badge on new message, reply box. | M (2 files + live tag) |
| 2 | `POST /api/public/locations/:slug/wallet/topup` (owner-only) | owner / counter | The kit shows "Add money" and gets 403. Owner needs a "Record top-up" control on the customer card (`admin/customers.js`) with amount + rail reference; or hide the kit screen until a payment webhook signs top-ups. | S |
| 3 | `POST /api/owner/i18n` | owner | Category headings have **no translation UI** at all (dish translations ride on `POST /owner/products/:id`; `admin/menu.js:198`). Add sq/en/uk name fields to the category sheet (`admin/menu.js openCategories`) and post through this route. | S |
| 4 | `GET /api/owner/history` (`?archive=log@n`) | owner | After rotation the hot log is not the whole history; the console's Orders → History tab (`admin/orders.js` view.mode 'history') only folds the live log. Add an "Older…" picker listing archives from `/owner/health` and loading one through this route. | M |
| 5 | `GET /api/owner/stock/waste` | owner, kitchen | Waste report by reason/value/signer. **Covered by W-INV (I4 Waste screen)** — do not re-propose. | — |
| 6 | `POST /api/owner/branding/extract` | owner | "Suggest colours from a photo": file input on the Branding sheet (`admin/more.js openBranding`) → swatches → pick → existing `POST /owner/branding`. | S |
| 7 | `POST /api/owner/branding/preset` | owner | "A whole look at once": preset chips on the Branding sheet. Trivial to wire; the route exists and the sheet already saves three fields separately. | XS |
| 8 | `POST /api/owner/logo/clear` | owner | The logo can be set (`admin/more.js:493`) but never removed. One "Remove" button beside the file input. | XS |
| 9 | `POST /api/owner/restore` | owner | Restore a downloaded backup into an EMPTY venue. The Health sheet has "Backup" (`more.js:527`) but no "Restore" file input; today this is a curl in a runbook. | S |
| 10 | `GET /api/owner/graph?q=` | owner (debug) | "What the assistant was shown." Best as a collapsible "sources" panel under each answer in the coming `admin/assistant.js` — **hand to W-KITCHEN (K5)** rather than a separate screen. | S |
| 11 | `POST /api/owner/customers/reforget` | owner (runbook) | Last step of the PITR runbook, idempotent. A "Re-apply forget requests" button on Health, guarded by confirm. Low frequency; still must be reachable without curl. | XS |
| 12 | `POST /api/owner/customers/rekey` | owner (one-shot migration) | Idempotent migration of old phone-hash cards. Either a one-time "Repair customer keys" button on Health, or run once per venue and delete the route. | XS |
| 13 | `POST /api/owner/hub/rotate` | owner | The nightly cron does this; an owner button ("Archive finished orders now") on Health is honest to the rule but low value. | XS |
| 14 | `GET /api/platform/errors` | platform operator | The platform console (`platform/app.js`) lists hubs and waitlist only; the worker-500 black box (`errlog`, platform-wide) has **no reader**. Add an "Errors" pane to `platform/hub.html`. | S |
| 15 | `POST /api/public/locations/:slug/pass/verify` — **staff side** | host / waiter | The kit's guest can verify its own pass (`kit/data.js:238`), but the person at the door — the waiter in `/room` — has no scanner/entry control. Add "Verify pass" (code field) to the room's floor screen. (unverified whether `verify_pass` admits a staff token) | S |

### 2.2 USED only from the owner console, though the natural user is staff

Not orphans by the letter of the rule, but the operator's "має бути видно" fails for the person who does the work:

| Route | Only caller today | Natural user | Lane |
|---|---|---|---|
| `POST /api/staff/orders/:id/kitchen-ack` | `admin/orders.js:210` | kitchen | **W-KITCHEN K4** (board "seen" on tap) |
| `POST /api/staff/orders/aggregator` | `admin/aggregator.js:83` | counter / waiter | open — add to `/room` |
| `POST /api/staff/orders/:id/refund` | `admin/refund.js:32` | counter-manager (Cap::Void) | open — add to `/room` pay sheet |
| `POST /api/staff/orders/:id/returned` | `admin/orders.js:232` | counter / courier desk | open |
| `POST /api/owner/stock/:kind` (received/wasted/stocktake) | `admin/stock.js:140` | kitchen | **W-KITCHEN K2 guards + W-INV screens** |
| `GET /api/live` | admin, courier, store — **not `/room`** | waiter | W-KITCHEN adds a kitchen tag; the room still polls (`room/app.js:92`) |
| `GET /api/order/:id/stamps` | store only | waiter at the table | minor |

## 3. MACHINE-ONLY (no UI needed, and why)

| Route | Why no screen |
|---|---|
| `GET /healthz` | uptime probe; returns "ok" |
| `POST /api/bootstrap` | `BOOTSTRAP_SECRET`-gated seed used by scripts and hub creation (`platform::create_hub` two-step) |
| `POST /api/webhooks/stripe` | Stripe signs it; payments then appear in Orders |
| `GET/POST /api/webhooks/meta` | Meta verify + inbound WhatsApp/Instagram; surfaced in Inbox (`/owner/inbox`); the URL is shown at `admin/more.js:386` |
| `POST /api/print/poll`, `GET/DELETE /api/print/job/:token` | polled by the kitchen printer with an API key minted at `admin/printer.js:52`; the queue is visible via `/owner/print/jobs` |
| `POST /api/mcp` | JSON-RPC for agents (Claude Code etc.); `GET /api/mcp` describe IS shown in three consoles |
| `GET /api/learn/manifest`, `GET /api/learn/media/*key` | fetched by the wiki (`wiki/wiki.js:147`, `wiki-core.js:95`) — counted USED below, listed here because they are asset gates, not controls |
| `scheduled` (`17 3 * * *` nightly `cloud::nightly`; every minute `outbox::sweep`, `ebills::poll::sweep`, `fiscal::rail::sweep`) | cron; their effects show in Cloud, Ebills and Fiscal sheets |

## 4. Backend capabilities with no route or no UI

### 4.1 Declared settings keys (`crates/dowiz-hub/src/settings/known.rs`, 35 keys) with no console control

The Settings sheets write keys by name (`admin/more.js`, `printer.js`, `stamps.js`, `exceptions.js`); the
generic pane the file's doc promises ("the settings pane can be BUILT from it") does not exist — `known`
is fetched (`more.js:345`) and never rendered. Unedited keys:

| Key | Effect if set | Who covers |
|---|---|---|
| `notify.telegram.chat.bar` | second Telegram target for the bar station | **W-TG** (groups model supersedes it) |
| `notify.whatsapp.status` | opt-in to paid WhatsApp status pushes (OFF by default, $) | open — one toggle on the Notifications sheet |
| `tax.default_ppm`, `tax.prices_include`, `tax.delivery_fee_ppm`, `tax.schedule` | orders carry a `tax` block; validated by `tax_cfg::validate` | **open — no tax UI at all**; a "Tax" sheet with rate (%, converted to ppm), inclusive toggle, delivery-fee rate, schedule editor |
| `fiscal.since_ms` | fiscalisation configured from this instant; `GET /owner/fiscal` reports "not configured" without it | open — the Fiscal sheet (`admin/fiscal.js`) arms/disarms but has **no field for the start date**; today only a raw `POST /owner/settings` sets it |
| `ai.*`, `social.*`, `notify.telegram.token/chat`, `notify.whatsapp.token/phone_id/to/verify`, `notify.meta.secret`, `cloud.s3.*`, `alerts.exceptions.*`, `print.kitchen`, `loyalty.stamps.*` | — | all edited (verified by grep) |

Feature flags (`crates/dowiz-hub/src/features.rs`: tips, promo, feedback, allergen_filter, ar, sea, voice, ai.enabled, social.enabled) are all switchable — the Features sheet renders whatever `/owner/features` returns (`more.js:505-510`).

### 4.2 Hub modules with no worker caller

| Module | State | Who covers |
|---|---|---|
| `stock/cost.rs` (`stamp`, `rebuild`, weighted-average cost) | referenced only by a comment in `catalog/bom.rs`; `unit_cost` appears nowhere in the worker or UI | **W-INV I2** |
| `graph.rs` | used by `assist.rs` (route #164, orphan above) | W-KITCHEN assistant panel |
| `subs.rs` ("who to tell", Telegram chat ids in KV) | no worker caller found (unverified: may be reached via `crate::subs` inside the hub only) | **W-TG** groups model likely replaces it |
| `forget.rs` (top-level) | only self + `consent/forget.rs`; the route side (`/customers/:key/forget`, `reforget`) is wired | — |
| `minijson.rs` | internal helper | — |

### 4.3 Switches no UI can flip

| Switch | Where | Note |
|---|---|---|
| `fiscal::SEND_ENABLED = false` | `workers/api/src/fiscal/mod.rs:40` | compile-time; operator decision 2026-09-24 (import-only). The Fiscal sheet lets an owner "arm" a venue whose sends are globally off — the sheet should say so. |
| `fiscal.ebills.armed`, `fiscal.ebills.cancel_armed` | `ebills_arm.rs` (not in `known.rs`, by design) | edited via `/owner/fiscal/ebills` — fine |
| Stripe / Telegram platform secrets | `wrangler` secrets | infra, correct |

## 5. Full route table

Guard legend: **pub** public; **cust** customer/booking token bound to the order; **owner** venue owner (`of_authorised`/`owner_at`); **staff(Cap)** owner or staff holding the cap via `staff_at`; **any** any principal of the venue (`of_any`/`principal_at`); **courier**; **platform** platform-admin token; **key** API key; **hook** webhook signature.

| # | Method | Path | Handler | Guard | Class | UI caller |
|---|---|---|---|---|---|---|
| 1 | GET | /healthz | closure | pub | MACHINE | uptime probe |
| 2 | GET | /api/public/locations/:slug/menu | storefront::menu | pub | USED | app.js:70, kit/data.js:45, room/menu.js:21, admin/app.js:213 |
| 3 | GET | /manifest.webmanifest | storefront::manifest | pub | USED | store/index.html:18 |
| 4 | GET | /privacy | privacy::notice::serve | pub | USED | store/consent.js:32, kit/screens/privacy.js:24 |
| 5 | GET | /dpa | privacy::dpa::page | pub | USED | admin/dpa.js:22, platform/hub.html:102 |
| 6 | POST | /api/public/locations/:slug/orders | storefront::place | pub | USED | store/checkout.js:361, room/open.js:45, kit/data.js:266 |
| 7 | GET | /api/public/locations/:slug/reservations | booking::list | cust | USED | kit/data.js:233 |
| 8 | POST | /api/public/locations/:slug/reservations | booking::create | pub | USED | store/booking.js:346, admin/bookings.js:146, kit/data.js:235 |
| 9 | GET | /api/public/locations/:slug/reservations/:id | booking::detail | cust | USED | store/booking-mine.js:31, kit/data.js:234 |
| 10 | POST | /api/public/locations/:slug/reservations/:id/action | booking::action | cust | USED | store/booking-mine.js:92, kit/data.js:236 |
| 11 | GET | /api/public/locations/:slug/reservations/:id/pass | booking::issue_pass | cust | USED | kit/data.js:237 (kit/screens/pass.js) |
| 12 | POST | /api/public/locations/:slug/pass/verify | booking::verify_pass | any (unverified) | USED (guest) / ORPHAN (staff) | kit/data.js:238; no room/admin caller |
| 13 | GET | /api/public/locations/:slug/tables | booking::availability | pub | USED | store/booking.js:105 |
| 14 | GET | /api/public/locations/:slug/threads/:id | social::messages | cust / staff(Advance,TakeOrders) / owner | USED (kit) / **ORPHAN (venue side)** | kit/data.js:242; no venue screen |
| 15 | POST | /api/public/locations/:slug/threads/:id/messages | social::send | same | USED (kit) / **ORPHAN (venue side)** | kit/data.js:243 |
| 16 | GET | /api/public/locations/:slug/wallet | wallet::balance | cust / owner | USED | kit/data.js:250 |
| 17 | GET | /api/public/locations/:slug/wallet/statement | wallet::statement | cust / owner | USED | kit/data.js:251 |
| 18 | POST | /api/public/locations/:slug/wallet/topup | wallet::top_up | **owner only** | **ORPHAN** | kit/data.js:252 calls as customer → 403; no owner control |
| 19 | POST | /api/public/locations/:slug/eta | eta::quote | pub | USED | store/eta.js:39, kit/data.js:311 |
| 20 | POST | /api/promo/check | preview::promo_check | pub | USED | store/checkout.js:305 |
| 21 | GET | /api/public/reach | zones::reach | pub | USED | store/checkout.js:320 |
| 22 | GET | /api/public/rates | rates::rates | pub | USED | lib/money.js loadRates ← store/state.js:136, admin/core.js:112, courier/app.js:40 |
| 23 | POST | /api/voice | voice::voice | any (staff caps in decide.rs) | USED | admin/voice.js:52, room/voice.js:114, courier/app.js:716 |
| 24 | POST | /api/owner/zones | zones::set_zones | owner | USED | admin/zones.js:144 |
| 25 | POST | /api/owner/floorplan | booking::set_plan | owner | USED | admin/floorplan.js:152 |
| 26 | GET | /api/owner/floorplan | booking::get_plan | owner | USED | admin/floorplan.js:161, bookings.js:123 |
| 27 | GET | /api/owner/reservations | booking::venue_day | owner | USED | admin/bookings.js:70 |
| 28 | POST | /api/owner/reservations/:id/action | booking::venue_action | owner | USED | admin/bookings.js:114 |
| 29 | POST | /api/owner/branding/extract | brand_extract::extract_branding | owner | **ORPHAN** | — |
| 30 | POST | /api/order/:id/feedback | feedback::feedback | cust | USED | store/track.js:75 |
| 31 | POST | /api/bootstrap | bootstrap::seed | BOOTSTRAP_SECRET | MACHINE | scripts / hub creation |
| 32 | POST | /api/waitlist | waitlist::join | pub | USED | platform/landing.js:546 |
| 33 | GET | /api/platform/waitlist | waitlist::list | platform | USED | platform/app.js:104 |
| 34 | GET | /api/platform/errors | platform::errors | platform | **ORPHAN** | — |
| 35 | GET | /api/platform/hubs | platform::hubs | platform | USED | platform/app.js:78 |
| 36 | POST | /api/platform/hubs | platform::create_hub | platform | USED | platform/app.js:141 |
| 37 | POST | /api/webhooks/stripe | stripe::webhook | hook | MACHINE | Stripe |
| 38 | POST | /api/auth/login | accounts::owner_login | pub | USED | admin/app.js:53, platform/app.js:49 |
| 39 | POST | /api/auth/refresh | accounts::owner_refresh | refresh token | USED | admin/core.js:95, wiki/wiki.js:37 |
| 40 | POST | /api/auth/logout | accounts::owner_logout | owner | USED | admin/app.js:67 |
| 41 | POST | /api/courier/auth/login | accounts::courier_login | pub | USED | courier/app.js:506 |
| 42 | POST | /api/courier/auth/claim | accounts::courier_claim | invite code | USED | courier/app.js:529 |
| 43 | POST | /api/staff/login | staff::staff_login | pub | USED | room/app.js:78 |
| 44 | POST | /api/staff/claim | staff::staff_claim | invite code | USED | room/app.js:78 |
| 45 | GET | /api/owner/staff | staff_admin::list_staff | owner | USED | admin/staff.js:22, app.js:232 |
| 46 | POST | /api/owner/staff/invite | staff_admin::invite_staff | owner | USED | admin/staff.js:46 |
| 47 | POST | /api/owner/staff/:id | staff_admin::set_staff | owner | USED | admin/staff.js:76,87 |
| 48 | GET | /api/staff/room | room::handlers::room_view | staff(TakeOrders) | USED | room/app.js:92 |
| 49 | POST | /api/staff/orders/:id/amend | room::handlers::amend | staff(TakeOrders; Void for after-kitchen) | USED | room/sheet.js:87, room/voice.js:54 |
| 50 | POST | /api/staff/orders/:id/pay | room::pay::pay | staff(TakePayment) | USED | room/pay.js:120, room/voice.js:58 |
| 51 | POST | /api/staff/orders/:id/kitchen-ack | kitchen_ack::kitchen_ack | staff(Advance) | USED (owner console only) | admin/orders.js:210 — kitchen board: W-KITCHEN |
| 52 | POST | /api/print/poll | print::poll | key (owner API key) | MACHINE | printer daemon; URL shown admin/printer.js:32 |
| 53 | GET | /api/print/job/:token | print::job | key | MACHINE | printer daemon |
| 54 | DELETE | /api/print/job/:token | print::ack | key | MACHINE | printer daemon |
| 55 | GET | /api/owner/print/jobs | print::jobs | owner | USED | admin/orders.js:78, printer.js:30 |
| 56 | GET | /api/owner/wallet/legs | legs::audit | owner | USED | admin/exceptions.js:81 |
| 57 | POST | /api/owner/wallet/legs/repair | legs::repair | owner | USED | admin/exceptions.js:92,94 |
| 58 | POST | /api/staff/orders/aggregator | aggregator::enter | staff(TakeOrders) | USED (owner console only) | admin/aggregator.js:83 |
| 59 | POST | /api/staff/orders/:id/refund | refund::refund | staff(Void) | USED (owner console only) | admin/refund.js:32 |
| 60 | POST | /api/staff/orders/:id/returned | refund::returned | staff(Void) | USED (owner console only) | admin/orders.js:232 |
| 61 | POST | /api/staff/orders/:id/transfer | room::transfer::transfer | staff(TakeOrders) | USED | room/transfer.js:61 |
| 62 | POST | /api/staff/sittings/:id/move | room::transfer::move_sitting | staff(TakeOrders) | USED | room/transfer.js:90 |
| 63 | POST | /api/staff/till/open | room::till::open | staff(OpenTill) | USED | room/till.js:94,115 |
| 64 | POST | /api/staff/till/count | room::till::count | staff(OpenTill) | USED | room/till.js:95,115 |
| 65 | POST | /api/staff/till/close | room::till::close | staff(OpenTill) | USED | room/till.js:58,115 |
| 66 | POST | /api/staff/till/pay_in | room::till::pay_in | staff(OpenTill) | USED | room/till.js:50,115 |
| 67 | POST | /api/staff/till/pay_out | room::till::pay_out | staff(OpenTill) | USED | room/till.js:50,115 |
| 68 | GET | /api/staff/till/tips | room::till::tips | staff(OpenTill) | USED | room/till-view.js:67 |
| 69 | GET | /api/staff/floor | room::floor::get | staff(TakeOrders) | USED | room/floor.js:20 |
| 70 | POST | /api/staff/floor/:sitting/cleared | room::floor::post_cleared | staff(TakeOrders) | USED | room/floor.js:21 |
| 71 | GET | /api/owner/tables/qr | table_qr::list | owner | USED | admin/tableqr.js:75 |
| 72 | GET | /api/owner/tables/:zone/:n/qr.svg | table_qr::one | owner | USED | admin/tableqr.js:62 |
| 73 | POST | /api/staff/orders/:id/guest | guest_round::confirm | staff(Advance/TakeOrders) | USED | room/guest.js:34 |
| 74 | GET | /api/order/:id/sitting | guest_round::sitting_bill | cust | USED | store/table.js:71 |
| 75 | GET | /api/order/:id/stamps | loyalty::order_stamps | cust | USED | store/stamps.js:43 |
| 76 | GET | /api/owner/orders | owner::orders | owner | USED | admin/app.js:190,196 |
| 77 | POST | /api/owner/orders/:id/action | owner::order_action | owner (Cap::Advance for staff) | USED | admin/orders.js:196, voice-plan.js:25 |
| 78 | POST | /api/owner/orders/:id/assign | owner::assign_courier | owner | USED | admin/orders.js:244 |
| 79 | GET | /api/owner/couriers/:id | courier::console::courier_detail | owner | USED | admin/couriers.js:61 |
| 80 | GET | /api/owner/dashboard | owner::dashboard | owner | USED | admin/app.js:204 |
| 81 | POST | /api/owner/products | catalog_edit::create_product | owner | USED | admin/menu.js:240 |
| 82 | POST | /api/owner/products/:id/delete | catalog_edit::delete_product | owner | USED | admin/menu.js:168 |
| 83 | GET | /api/owner/categories | catalog_edit::list_categories | owner | USED | admin/menu.js:226 |
| 84 | POST | /api/owner/categories | catalog_edit::set_category | owner | USED | admin/menu.js:258,259 |
| 85 | POST | /api/owner/categories/:id/delete | catalog_edit::delete_category | owner | USED | admin/menu.js:260 |
| 86 | POST | /api/owner/products/:id | owner::update_product | owner | USED | admin/menu.js:218, voice-plan.js:28 |
| 87 | POST | /api/owner/location | owner::update_location | owner | USED | admin/app.js:150,154; more.js:267,302,318,336,498 |
| 88 | POST | /api/owner/i18n | owner::write_translations | owner | **ORPHAN** | — (category translations have no UI) |
| 89 | GET | /api/owner/analytics | analytics::analytics | owner | USED | admin/more.js:194 |
| 90 | GET | /api/owner/exceptions | exceptions::exceptions | owner | USED | admin/exceptions.js:39 |
| 91 | GET | /api/owner/promotions | promotions::promotions | owner | USED | admin/more.js:105 |
| 92 | POST | /api/owner/promotions | promotions::set_promotion | owner | USED | admin/more.js:110,131 |
| 93 | POST | /api/owner/promotions/:code/delete | promotions::delete_promotion | owner | USED | admin/more.js:111 |
| 94 | GET | /api/owner/activation | activation::activation | owner | USED | admin/more.js:513 |
| 95 | GET | /api/owner/branding | brand::branding | owner | USED | admin/more.js:483 |
| 96 | POST | /api/owner/branding | brand::set_branding | owner | USED | admin/more.js:497 |
| 97 | POST | /api/owner/branding/preset | brand::set_preset | owner | **ORPHAN** | — |
| 98 | GET | /api/owner/campaigns | campaigns::list | owner | USED | admin/campaigns.js:40 |
| 99 | POST | /api/owner/campaigns | campaigns::define | owner | USED | admin/campaigns.js:75 |
| 100 | GET | /api/owner/campaigns/:id | campaigns::report | owner | USED | admin/campaigns.js:81 |
| 101 | POST | /api/owner/campaigns/:id/preview | campaigns::preview | owner | USED | admin/campaigns.js:100 |
| 102 | POST | /api/owner/campaigns/:id/send | campaigns::send_now | owner | USED | admin/campaigns.js:110 |
| 103 | GET | /api/owner/customers | customers::customers | owner | USED | admin/more.js:217, customers.js:110 |
| 104 | POST | /api/owner/customers/:key/reveal | customers::reveal_customer | owner | USED | admin/more.js:240 |
| 105 | POST | /api/owner/customers/:key/forget | forget::forget_customer | owner | USED | admin/customers.js:148 |
| 106 | GET | /api/owner/customers/reveals | customers::reveals | owner | USED | admin/more.js:249 |
| 107 | PUT | /api/owner/customers/:key/record | record_routes::put_record | owner | USED | admin/customers.js:77 |
| 108 | POST | /api/owner/customers/rekey | record_routes::rekey | owner | **ORPHAN** | — (one-shot migration) |
| 109 | POST | /api/owner/customers/reforget | forget::run::reforget | owner | **ORPHAN** | — (PITR runbook) |
| 110 | POST | /api/owner/customers/:key/consent | consent_routes::owner_act | owner | USED | admin/customers.js:140 |
| 111 | POST | /api/owner/customers/:key/link | alias_routes::link | owner | USED | admin/customers.js:119 |
| 112 | POST | /api/owner/customers/:key/unlink | alias_routes::unlink | owner | USED | admin/customers.js:105 |
| 113 | GET | /api/public/consent/wordings | consent_routes::wordings | pub | USED | store/consent.js:22 |
| 114 | GET | /api/owner/stock | stock::stock | owner (owner_beside) | USED | admin/stock.js:45, menu.js:280 |
| 115 | POST | /api/owner/stock/:kind | stock::stock_move | staff(OpenTill) | USED (owner console only) | admin/stock.js:140 — kitchen: W-KITCHEN K2 / W-INV |
| 116 | GET | /api/owner/stock/waste | waste::waste_report | owner | **ORPHAN — covered by W-INV (I4)** | — |
| 117 | POST | /api/owner/supplies | supplies::set_supply | owner | USED | admin/stock.js:118 |
| 118 | POST | /api/owner/supplies/:id/retire | supplies::retire_supply | owner | USED | admin/stock.js:120 |
| 119 | POST | /api/owner/supplies/import | bulk::import_supplies | owner | USED | admin/bulk.js:9 |
| 120 | POST | /api/owner/recipes/import | bulk::import_recipes | owner | USED | admin/bulk.js:9 |
| 121 | GET | /api/owner/products | bulk::owner_products | owner | USED | admin/menu.js:330 |
| 122 | GET | /api/owner/features | settings::features | owner | USED | admin/more.js:505 |
| 123 | POST | /api/owner/features | settings::set_feature | owner | USED | admin/more.js:510 |
| 124 | GET | /api/owner/settings | settings::settings | owner | USED | admin/more.js:160,345,382,425,543; printer.js:30; stamps.js:20 |
| 125 | POST | /api/owner/settings | settings::set_setting | owner | USED (27 of 35 keys) | see §4.1 |
| 126 | POST | /api/owner/notify/test | notify::test | owner | USED | admin/more.js:375 |
| 127 | GET | /api/owner/inbox | channels::inbox | owner | USED | admin/more.js:450 |
| 128 | GET | /api/owner/inbox/:peer | channels::thread | owner | USED | admin/more.js:463 |
| 129 | POST | /api/owner/inbox/:peer | channels::reply | owner | USED | admin/more.js:468 |
| 130 | GET | /api/owner/backup/cloud | cloud::status | owner | USED | admin/more.js:425 |
| 131 | POST | /api/owner/backup/cloud | cloud::push | owner | USED | admin/more.js:444 |
| 132 | GET | /api/webhooks/meta | channels::webhook_verify | hook | MACHINE | Meta |
| 133 | POST | /api/webhooks/meta | channels::webhook | hook (app secret) | MACHINE | Meta |
| 134 | GET | /api/owner/integrations | integrations::status | owner | USED | admin/more.js:625 |
| 135 | GET | /api/owner/dpa | privacy::dpa::read | owner | USED | admin/dpa.js:31 |
| 136 | POST | /api/owner/dpa/accept | privacy::dpa::accept | owner | USED | admin/dpa.js:43 |
| 137 | GET | /api/learn/manifest | learn::manifest | any role token | USED | wiki/wiki.js:147 |
| 138 | GET | /api/learn/media/*key | learn::media | any role token | USED | wiki/wiki-core.js:95 |
| 139 | POST | /api/owner/integrations/check | integrations::check | owner | USED | admin/more.js:634 |
| 140 | GET | /api/owner/ebills | ebills::routes::status | owner | USED | admin/ebills.js:23 |
| 141 | POST | /api/owner/ebills/config | ebills::routes::config | owner | USED | admin/ebills.js:84,89 |
| 142 | POST | /api/owner/ebills/map | ebills::routes::map | owner | USED | admin/ebills.js:73,77 |
| 143 | GET | /api/owner/fiscal | fiscal::routes::status | owner | USED | admin/fiscal.js:23 |
| 144 | POST | /api/owner/fiscal/ebills | fiscal::routes::set | owner | USED | admin/fiscal.js:61 |
| 145 | GET | /api/owner/orders/:id/receipt | fiscal::routes::receipt | owner | USED | admin/fiscal.js:78 |
| 146 | GET | /api/mcp | mcp::describe | pub | USED | admin/mcp.js:52, room/mcp.js:50, courier/mcp.js:49 |
| 147 | POST | /api/mcp | mcp::rpc | role key | MACHINE | agents (Claude Code etc.) |
| 148 | GET | /api/staff/mcp/keys | mcp::staff_list | staff | USED | room/mcp.js:50 |
| 149 | POST | /api/staff/mcp/keys | mcp::staff_mint | staff | USED | room/mcp.js:40 |
| 150 | POST | /api/staff/mcp/keys/revoke | mcp::staff_revoke | staff | USED | room/mcp.js:44 |
| 151 | GET | /api/courier/mcp/keys | mcp::courier_list | courier | USED | courier/mcp.js:49 |
| 152 | POST | /api/courier/mcp/keys | mcp::courier_mint | courier | USED | courier/mcp.js:39 |
| 153 | POST | /api/courier/mcp/keys/revoke | mcp::courier_revoke | courier | USED | courier/mcp.js:43 |
| 154 | GET | /api/owner/mcp/keys | mcp::owner_list | owner | USED | admin/mcp.js:52 |
| 155 | POST | /api/owner/mcp/keys/revoke | mcp::owner_revoke | owner | USED | admin/mcp.js:61 |
| 156 | POST | /api/owner/menu/import | import::import_menu | owner | USED | admin/menu.js:108 |
| 157 | GET | /api/owner/couriers | courier::console::couriers | owner | USED | admin/couriers.js:23, app.js:231 |
| 158 | POST | /api/owner/couriers/invite | hiring::invite_courier | owner | USED | admin/couriers.js:47 |
| 159 | POST | /api/owner/couriers/:id/uninvite | hiring::uninvite_courier | owner | USED | admin/couriers.js:34 |
| 160 | POST | /api/owner/couriers/:id/active | hiring::set_courier_active | owner | USED | admin/couriers.js:81 |
| 161 | GET | /api/owner/posts | posts::posts | owner | USED | admin/more.js:140 |
| 162 | POST | /api/owner/posts/draft | posts::draft_post | owner | USED | admin/more.js:148 |
| 163 | POST | /api/owner/posts/:id/approve | verdict::approve_post | owner | USED | admin/more.js:154 |
| 164 | POST | /api/owner/posts/:id/reject | verdict::reject_post | owner | USED | admin/more.js:155 |
| 165 | GET | /api/owner/graph | assist::graph | owner | **ORPHAN** (hand to W-KITCHEN K5) | — |
| 166 | GET | /api/live | live::connect | any (staff Advance/TakeOrders) | USED | lib/live.js:59 ← admin/app.js:293, courier/app.js:1169, store/track.js:191; not /room |
| 167 | GET | /api/owner/health | operations::health | owner | USED | admin/more.js:528 |
| 168 | GET | /api/owner/history | operations::history | owner | **ORPHAN** | — |
| 169 | POST | /api/owner/hub/rotate | operations::rotate_now | owner | **ORPHAN** | — (cron does it) |
| 170 | GET | /api/owner/backup | operations::backup | owner | USED | admin/more.js:527,536 |
| 171 | POST | /api/owner/restore | operations::restore | owner | **ORPHAN** | — |
| 172 | POST | /api/owner/assist | assist::owner_assist | owner | USED | admin/more.js:559, voice.js:61 |
| 173 | POST | /api/courier/assist | assist::courier_assist | courier | USED | courier/app.js:755,824 |
| 174 | GET | /api/owner/apikeys | keys::list_api_keys | owner | USED | admin/more.js:475 |
| 175 | POST | /api/owner/apikeys | keys::create_api_key | owner | USED | admin/more.js:480, printer.js:52 |
| 176 | POST | /api/owner/apikeys/revoke | keys::revoke_api_key | owner | USED | admin/more.js:478 |
| 177 | POST | /api/owner/products/:id/image | media::set_product_image | owner | USED | admin/menu.js:186,187 |
| 178 | POST | /api/owner/products/:id/image/clear | media::clear_product_image | owner | USED | admin/menu.js:193 |
| 179 | POST | /api/owner/place | place::set_place | owner | USED | admin/more.js:268,303 |
| 180 | POST | /api/owner/logo | media::set_venue_logo | owner | USED | admin/more.js:493 |
| 181 | POST | /api/owner/logo/clear | media::clear_venue_logo | owner | **ORPHAN** | — |
| 182 | GET | /media/:name | media::media | pub | USED (unverified line) | image URLs carried in menu JSON, drawn by store/kit/admin |
| 183 | GET | /api/courier/tasks | courier::tasks | courier | USED | courier/app.js:548 |
| 184 | POST | /api/courier/shift | courier::shift | courier | USED | courier/app.js:1059 |
| 185 | POST | /api/courier/orders/:id/accept | courier::accept | courier | USED | courier/app.js:668,886 |
| 186 | POST | /api/courier/orders/:id/pickup | courier::pickup | courier | USED | courier/app.js:930 |
| 187 | POST | /api/courier/orders/:id/deliver | courier::deliver | courier | USED | courier/app.js:1036 |
| 188 | POST | /api/courier/orders/:id/refused | courier::refused | courier | USED | courier/app.js:1020 |
| 189 | POST | /api/courier/position | courier::position | courier | USED | courier/app.js:428 |
| 190 | GET | /api/courier/earnings | courier::earnings | courier | USED | courier/app.js:854 |
| 191 | GET | /api/courier/history | courier::history::courier_history | courier | USED | courier/app.js:862 |
| 192 | GET | /api/order/:id | closure (lib.rs:477) | cust / owner-of-venue / assigned courier / staff(Advance,TakeOrders) | USED | store/track.js:207,279; store/state.js:262; kit/orders.js:86 |

Cron (not routes): `scheduled` in lib.rs — `17 3 * * *` → `cloud::nightly`; every other minute → `outbox::sweep`, `ebills::poll::sweep`, `fiscal::rail::sweep`.

## 6. Caveats

- Counts are by static grep of committed `public/**`; no route was driven live in this audit. `/media/:name` and `pass/verify`'s staff admission are marked unverified.
- The kit (`/kit/`) is a full second storefront with chat, wallet and passes; whether any live venue links to it was not verified (`store/index.html` has no reference to it). If it is not linked, routes 7–17 lose their only caller and become orphans too.
- Lane coverage is read from the cards in `.claude/lanes/W-{KITCHEN,INV,TG}-CARD.md` (2026-09-26), not from their trees; a card is a plan, not a delivery.
