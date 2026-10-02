# What the kitchen reaches in the hub (2026-09-27)

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** Merged `c9ccbfad` (W-KACCESS); the kitchen hub itself `2d652dfc`. Open kitchen rows K9/K13/K14/K15 + ST-A3/ST-A9/ST-P3 are Wave N2-C in `docs/design/ROADMAP-2026-09-22.md`. Nothing below was changed.


Lane W-KACCESS. Operator, 2026-09-26: "роль кухні має мати доступ до більшості ііч хаба, окрім тих
які непотрібні, наприклад payments, загальна аналітика і тд" -- the kitchen role reaches MOST of
the hub, except what it does not need: payments and money settings, the venue's general (revenue)
analytics, customers' personal data, staff admin, legal, data and safety, API keys, billing / tax /
fiscal.

This note decides every screen, every Venue (More) row, every W-WIRE tile and every owner/staff route
family, with one line of why. The machine copy of the route table is
`workers/api/src/services/identity/staff/access.rs` (`ROUTES`); its test fails when `lib.rs` grows an
`/api/owner/*` or `/api/staff/*` route without a row here. The machine copy of the screen table is
`workers/api/public/admin/access.js` (`SECTION_CAPS`, `TILE_CAPS`); its test fails when `more.js` or
`wire.js` grows a row without a decision.

## 0. The rule in one paragraph

The kitchen is `Preset::Kitchen = {advance, catalog, stock}` (`crates/dowiz-hub/src/caps.rs`). NO NEW
CAPABILITY is added: every screen the kitchen gains is opened by one of those three words.
`advance` is the pass (the board, the printer, the floor and the bookings to cook for), `catalog` is the
menu, `stock` is the shelf and its numbers. Everything that is money, a customer, a person on the
staff, a key, a legal text or the venue's data safety stays the owner's. Where a screen is shared,
the HUB removes what the kitchen must not see before it answers (redaction is server-side; the
console only stops drawing an empty column). The kitchen never writes to a screen it only reads.

Words: **yes** = reads and writes; **read** = reads only, the write stays the owner's; **no** = not shown,
and the route refuses a kitchen token (403; another venue's token is 404).

## 1. The console's tabs (`admin/app.js` TABS, filtered by `admin/access.js` tabsFor)

| Tab | Kitchen | Why |
|---|---|---|
| orders | no | The owner's queue carries names, phones, addresses and money (`GET /api/owner/orders`). The kitchen's order list is the board below, which the hub strips (`GET /api/staff/kitchen`). |
| kitchen (the board) | yes | Its job: new / preparing / ready, seen, all-day counts, stop-list switch. PII-free by construction. |
| menu | yes | Operator Q8: the kitchen fully manages the menu (dishes, categories, photos, translations, the 86, import). |
| stock (ingredients & stock) | yes | Operator Q8: receive, count, write off, produce; supplies and recipes; the kitchen numbers button. |
| couriers | no | Couriers' personal data, their pay and hiring. |
| more (Venue) | yes, filtered | Shown when at least one row below is open to the person; only those rows are drawn (phone list and desktop sidebar alike). |

## 2. The Venue (More) rows (`admin/more.js` GROUPS)

| Row | Kitchen | Why |
|---|---|---|
| inbox | no | WhatsApp/Instagram threads with customers: personal data. |
| bookings | read | The kitchen preps for tonight's covers. The hub answers time, party size, table, occasion and status -- **no name, no phone, no actions** (`access::bookings_for_kitchen`). Confirm/decline/seat stay the owner's. |
| floorPlan | read | Which table is which ("table 4" on a ticket). Drawing the plan is the owner's. |
| tableQr | no | Printing table codes is a front-of-house setup task. |
| promos, posts, campaigns, social | no | Marketing: prices, customers, the venue's public voice. |
| analytics | no | The venue's general numbers: revenue. (The kitchen's own numbers are under stock, below.) |
| customers | no | Personal data. |
| staff | no | Staff admin. |
| exceptions | no | Voids, comps, refunds, pay-outs: money. |
| learn | yes | The lessons for the screens this person can open (kitchen, menu, stock, assistant). |
| venue | no | The venue's name, phone, address: the owner's record. |
| hours | read | A cook needs to know when the kitchen closes. Changing them (and the time zone) stays the owner's. |
| deliveryTerms, deliveryArea, payments | no | Delivery prices and money. |
| branding | no | The venue's look. |
| features | no | Venue switches (stamp card, stock refusals): owner decisions. |
| preview | yes | Opening the storefront as a guest sees it (dish photos, 86'd dishes) writes nothing. |
| integrations, channels, ebills, cloud | no | Keys, webhooks, fiscal link, backups. |
| notifications | no | WhatsApp/Telegram tokens are secrets. Kitchen Telegram groups: OPEN (section 6). |
| printer | yes | The kitchen ticket printer: its name (the one setting `print.kitchen`) and the tickets in its queue. Making a printer key mints a venue API key -- that button stays the owner's. |
| assistant | no (row) | The row is the AI endpoint's settings (a token). The assistant ITSELF is on every screen for the kitchen already (`/api/staff/assist`), with the person's own agent key. |
| mcp | no (row) | The row lists and revokes EVERY holder's agent keys. A cook's own key is minted from the assistant panel (`/api/staff/mcp/keys`). |
| apiKeys | no | Venue API keys. |
| activation | no | The owner's go-live checklist (payments, legal). |
| health | no | Backups, error counts, rotation: data and safety. |
| dpa | no | Legal. |

## 3. The W-WIRE tiles (`admin/wire.js` TILES)

| Tile | Kitchen | Why |
|---|---|---|
| messages (order threads) | no | Customers' own words and names. OPEN (section 6). |
| wallets | no | Money. |
| history (older orders) | no | Archived orders with contact data. OPEN (section 6) -- a stripped history is one more route. |
| tax | no | Tax / fiscal. |
| catWords (category names in every language) | yes | Translations are the menu (`catalog`). |
| brand | no | The venue's look. |
| safety (restore, rekey, rotate) | no | Data and safety. |
| graph (what the assistant knows) | no | The owner's knowledge graph mixes customers and money. |

## 4. The route families (`lib.rs`; the machine table is `access.rs` ROUTES)

`guard.rs` names the families; a staff token must hold ANY word of the family, and must belong to the
venue it names (404 otherwise). An owner's token takes the owner's path unchanged.

| Family (`guard.rs`) | Words | Routes | Kitchen |
|---|---|---|---|
| PASS | advance | `POST /api/owner/orders/:id/action`, `POST /api/staff/orders/:id/kitchen-ack`, `GET /api/staff/kitchen`, **`GET /api/owner/print/jobs`**, **`GET /api/owner/floorplan`**, **`GET /api/owner/reservations`** (redacted), **`GET/POST /api/owner/settings`** (only `print.kitchen`) | yes |
| MENU | catalog | products (create, edit, delete, image, clear), categories (list, set, delete), `POST /api/owner/i18n`, `POST /api/owner/menu/import`, `POST /api/owner/supplies*`, `POST /api/owner/recipes/import`, `GET /api/owner/products` | yes |
| SHELF | stock | `GET /api/owner/stock`, `POST /api/owner/stock/:kind` (received, count, produced, as-is), `GET /api/owner/stock/waste` | yes |
| BIN | open_till or stock | `POST /api/owner/stock/wasted` | yes |
| NUMBERS | stock or catalog | **`GET /api/owner/analytics/kitchen`** -- revenue, margin and food-cost (a share of revenue) removed for staff (`access::numbers_for_kitchen`) | yes (redacted) |
| ASKERS | advance, catalog or stock | `POST /api/staff/assist` | yes |
| (own person) | any staff | `GET/POST /api/staff/mcp/keys`, `/revoke`; `POST /api/staff/login`, `/claim`, `/password` | yes |
| ROOM | take_orders / take_payment / void / open_till | `/api/staff/room`, amend, pay, refund, returned, transfer, move, till/*, floor (sittings), guest, aggregator | no (the kitchen holds none of those words) |
| OWNER | owner only | everything else under `/api/owner/*`: orders list, dashboard, analytics, exceptions, customers/*, promotions, campaigns, posts, couriers/*, staff/*, location, place, zones, branding/*, logo/*, features, notify, telegram/*, inbox/*, threads, integrations/*, ebills/*, fiscal/*, receipt, dpa/*, backup/*, restore, hub/rotate, health, history, apikeys/*, mcp/keys (owner), graph, assist (owner), wallet/legs/*, tables/qr, reservations action, floorplan write | no |

**Bold** = opened to the kitchen by this lane. Everything else in the PASS/MENU/SHELF/BIN/ASKERS rows was
already open (lanes W-KITCHEN, W-INV).

### Redaction, where a screen is shared

| Screen | What the kitchen does not get | Where |
|---|---|---|
| Kitchen numbers | `revenue`, `margin`, `marginPortion`, `foodCostPm` (a ratio to revenue) -- in totals, by day and by dish. It keeps orders, dishes sold, portion cost, cost of goods, usage, waste, drift, yields, supplier prices, cover and reorder. The answer says `"scope": "kitchen"`; the view drops the empty columns. | `access::numbers_for_kitchen`, `admin/kitchen-view.js` |
| Bookings | `name`, `phone`; `next` is empty (no actions). Keeps time, party, table, zone, occasion, status. | `access::bookings_for_kitchen`, `admin/bookings.js` read-only |
| Settings | Only `print.kitchen` is read or written; no secret, no other key is ever sent to a staff token. | `access::KITCHEN_SETTINGS`, `settings.rs` |
| Order board | Contact, address, prices, payments (already, `GET /api/staff/kitchen`). | unchanged |

## 5. Signing in without the room app

`/admin/` gains "I have a staff code": email, the code from the invite, a password (new account) or the
account's password (existing one) -> `POST /api/staff/claim` -> the console opens with that person's
tabs. The same hub, the same door the room app uses; the cook never needs `/room/`.
`admin/signin.js` `claim()` is pure and tested beside `signIn()`.

## 5b. Staff passwords (operator item, 2026-09-27)

Production had no way to change a member of staff's password: a hash was written only by
`/api/staff/claim`.

| Route | Who | Rules | UI |
|---|---|---|---|
| `POST /api/staff/password` `{email, old_password, new_password}` (main `282d2c45`, `staff::staff_password`) | the person | new password >= `MIN_PASSWORD_CHARS` (400), then the old one with constant work (401); a fresh argon2 hash; EVERY open staff session of the person ends (at each venue they work) and the console signs in again with the new password | profile sheet -> "Change password" (staff only), `admin/password.js` openOwn |
| `POST /api/owner/staff/:id/password` `{new_password}` (`staff::password::owner_reset`; route line handed back) | the owner of the venue | `owner_and_venue`; the person must be a member of staff AT THAT VENUE (403 otherwise, so another venue's owner is 403); an account that is a member ANYWHERE ELSE (an owner elsewhere, staff elsewhere) is refused 403 -- the password opens those venues too; length 400; the person's sessions at the venue end | staff card -> "Set a new password", `admin/password.js` openReset |

## 6. OPEN -- needs the operator

1. **Order messages (threads) for the kitchen?** A guest writing "no sesame" after ordering matters to
   the cook, but a thread carries the customer's words and name. Proposed: a read-only, name-free view of
   the thread on the ticket. Not built.
2. **Kitchen Telegram groups?** The kitchen could CHOOSE which events its own group receives, but the
   bot token is the owner's secret and `notify.telegram.*` holds it. Proposed: the owner connects the
   group; the kitchen gets no Telegram screen. Not built.
3. **Older orders (history) for the kitchen?** Needs a stripped `history` route (the archive carries
   contact data). Not built; the board covers the live service.
4. **May the kitchen set the venue "busy"/"closed" when the pass is overwhelmed?** Today the header chip is
   read-only for staff. A kitchen "busy" is a common KDS feature; it is also a switch that stops sales.
5. **Printer key.** Pairing a new printer mints a venue API key; kept owner-only. If the kitchen should pair
   printers alone, a printer-only key kind is needed.
