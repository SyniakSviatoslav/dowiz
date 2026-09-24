# Seven launch gaps: what the tree says, what the world says, and what each one costs to close

**Date:** 2026-09-24. **HEAD read:** `b154b97c` ("fix: the 7bd2881b..0dd8a6d3 features, walked as the owner and
the waiter", 2026-09-24 10:52). **Method:** read-only on the code; no build, no login, no write to any live
system. Every claim about the tree cites `file:line` as read at this HEAD; every external claim cites a URL read
on 2026-09-24; **UNVERIFIED** marks what could not be checked from this box. `measured` means a command was run
here and its output is quoted; `derived` means arithmetic over measured inputs; `hypothesis` means neither.

**Two of the seven gaps are not what the operator's list says they are, and this document says so first:**

- **G3 (the courier credential) is already closed in the tree and in the credential file.** The invite flow
  exists end to end (`POST /api/owner/couriers/invite` → code shown once → `POST /api/courier/auth/claim`), and
  `/root/.dowiz_owner` (dated 2026-09-21 10:17, measured) carries `QA_COURIER_PHONE` / `QA_COURIER_PASSWORD`
  for a courier created on `sushi-durres` by exactly that flow (`courier-login-venue-from-host`, "RESOLVED same
  day"). What is missing is smaller than "a flow": the code is never *delivered* to the courier, only shown to
  the owner. §3.
- **G6 (ML-KEM-768 "not used on any live path") was true on 2026-09-23 morning and false by that evening.**
  Commit `8ae71778` (2026-09-23 19:34, measured) seals the nightly off-site copy with hybrid X25519+ML-KEM-768
  (`workers/api/src/cloud/seal.rs`, `crates/dowiz-core/src/pq/backup_seal.rs`). The path is live *code* but is
  in its `Off` state until `BACKUP_SEAL_PK` is set — and no document, script or config in the tree records that
  var (grep, measured: zero hits). So every nightly copy is still plain gzip and the error log says so
  (`cloud.rs:419`). The gap is a keygen, a var and a deploy, plus one combiner correction. §6.

The other five are as described. The roadmap rows are in `ROADMAP-2026-09-22.md` §Wave F; this document is the
evidence behind each row.

---

## 1. G1 — Stock: the ledger works and holds nothing

### 1.1 What exists (measured in the tree)

| Piece | Where | What it does |
|---|---|---|
| The ledger | `crates/dowiz-hub/src/stock.rs:85` (`StockEvent`), `:160` (`StockError`, `OutOfStock` = the automatic 86), `:209` (`StockLedger`, a fold, never a counter) | Received / Reserved / Consumed / Released / Wasted / Stocktake / Served / Unserved |
| A dish's recipe, as the ledger reads it | `stock.rs:1198` (`BomLine`), `:1211` (`bom_of(product_json)` reads only `bom[].supply` + `bom[].qty`), `:1238` (`reservations_for`), `:1271` (`settle`) | reservation at placement, settle at kitchen start or death |
| Cost that follows purchases | `crates/dowiz-hub/src/stock/cost.rs` (header, lines 1–30): a `received` may carry `unit_cost`, `per`, `supplier`, `doc`; weighted-average cost as a fold; the cost at placement is a `Stamp` (cost + log length) so law 8's rebuild reproduces it | **built** (BLUEPRINT-OPERATIONAL-BLIND-SPOTS §2.10), waiting for prices |
| The supply model and the derivation | `workers/api/src/recipe.rs:1–17` (kind, category, unit g/ml/unit, per-100 nutrition, `costPerBasis`, `weightPerUnit`, `lowAt`), `:79` `line_of`, `:125` `derive`, `:152` `bom_json` | a dish's kcal / weight / cost / ingredients follow from its lines |
| The routes | `workers/api/src/lib.rs:369–373`: `GET /api/owner/stock`, `POST /api/owner/stock/:kind` (received / wasted / stocktake, `services/operations/stock.rs:117–147`), `GET /api/owner/stock/waste`, `POST /api/owner/supplies`, `POST /api/owner/supplies/:id/retire`; `lib.rs:393` `POST /api/owner/menu/import` | one supply per request; one recipe per product save (`owner.rs:767, 819, 907–925`) |
| The console | `public/admin/stock.js` (141 lines: the three movements), `public/admin/menu.js:255–298` (the recipe editor, one dish at a time), `menu.js:89–101` (CSV import of the **menu**, dry run then apply) | every supply and every recipe is typed by hand, one at a time |
| The menu importer | `crates/dowiz-hub/src/import.rs:241` `from_csv`; known columns at `:215–232`: `category, name, description, price, available, id` in en/sq/uk/ru spellings; ambiguity refused, not guessed | **no supply column, no recipe column** |

**The number** (`dowiz-stock-ledger-works-but-is-off`, proved live 2026-09-21): 165 dishes on each venue, **0 with
a recipe, 0 supplies**; `e2e/kit-regression/stock-cycle.mjs` proves the whole cycle green on its own dish. The 73
dishes with a `weightG` and the 163 with a kcal figure carry hand-typed values, not derived ones.

**Why the owner has not entered them, said plainly.** 165 dishes × (say) 6 lines = ~1,000 recipe lines, each
typed through a sheet that opens one dish at a time, after first typing ~80 supplies one form at a time. That is
two working days of data entry with no visible payoff until the last line is in. The design that says "a venue
that has not modelled its ingredients reserves nothing" (`storefront.rs`, quoted in the memory note) is right;
the consequence is that the switch is never flipped.

### 1.2 Where recipes and supplies can come FROM — each source judged

| Source | Verdict | Evidence |
|---|---|---|
| **A spreadsheet the venue already has** | **YES — the primary path.** Every restaurant that costs its menu has one; the menu importer already reads CSV with tolerant headers in four languages and refuses ambiguity | `import.rs:206–232` (separator sniffing, column matching), `menu.js:89–101` (dry run → apply is the UX to copy) |
| **eBills purchase data** | **NO, today.** The read-only integration allow-lists six reads and none is a purchase (`ebills/client.rs:30–44`: Root, Account, Sales, Sale, Tables, Items). On the live tenant `/api/item-in-warehouses` answers `[]`, `/api/items` is `403`, and the business has `showInventory:false` (EBILLS §1.3 table rows at lines 142, 144 and §3 row at 386). The SPA knows a `ROLE_BUYER` (§1.2), so the platform *has* a purchasing side — this venue does not use it. **Re-entry:** the venue switches inventory on in ebills and `/api/item-in-warehouses` returns rows; then one more allow-listed read (`Path::Warehouse`) and the crosswalk that already exists (`ebills/state.rs`, `map.rs`) turns a warehouse line into a priced `received` | UNVERIFIED beyond what the blueprint measured on 2026-09-22 |
| **The eBills till catalogue** | **Partly.** 200 items, 150 matching a dowiz dish exactly by name and price (EBILLS §8.1). Useful for the *dish* side of a recipe crosswalk, useless for ingredients: the till sells dishes, not flour | measured 2026-09-23 |
| **A supplier's invoice** | **YES, as typed lines with a document number**, landing on the existing priced `received` (`cost.rs`: `unit_cost`, `per`, `supplier`, `doc`). Photo OCR is not proposed: it is an AI call on a money path, `DECISIONS.md`'s no-scoring stance applies to anything that *guesses* a number, and a mis-read quantity is a wrong cost stamped on every order until the next stocktake | `cost.rs:1–30` |
| **Recipe templates per category** | **YES, as a starting point the owner edits, never as a fact.** A sushi venue's dishes share a small alphabet (rice, nori, fish, avocado, cucumber, cream cheese, sauce, box). A template is a CSV row set per category; applying it to a dish that has no recipe writes lines the owner is shown *as a draft* and confirms | none needed; a data file |
| **Suggestions from what exists** | **YES, cheap.** For a dish without a recipe, propose the recipe of the nearest dish in the same category (normalised-name match, the same rule `ebills/map.rs` uses for the crosswalk) and scale the food lines to the dish's own `weightG` when it has one | `ebills` crosswalk suggestions (EBILLS §8.1 "suggestions by normalised name + price agreement") |

### 1.3 The design

**F1 — supplies and recipes in bulk, the way the menu already comes in.**

- `dowiz_hub::import::supplies_from_csv(text) -> SuppliesDraft` beside `from_csv` (`import.rs:241`): columns
  `name, kind, unit, kcal, protein, fat, carbs, cost, per, supplier, low_at` with the same tolerant header matching
  (`column_of`, `:215`), the same "ambiguity is a warning naming the row" rule, and prices through `parse_price`
  (`:139`) so `1.200,00` and `1200` cannot be confused. Pure: no I/O, exhaustively testable.
- `dowiz_hub::import::recipes_from_csv(text, dishes, supplies) -> RecipesDraft`: rows `dish, supply, qty`; the
  dish and the supply are matched by id when the column is an id, else by normalised name (the ebills rule);
  an unmatched name is a warning that names it and the row is left out; qty is in the supply's base unit.
- `POST /api/owner/supplies/import` and `POST /api/owner/recipes/import`, `?dry=1` first, the same shape as
  `menu/import` (`services/catalogue/import.rs`), applying through the existing writers: one `set_supply` per
  row (`services/operations/supplies.rs:51`) and, for recipes, the same `bom` path `update_product` takes
  (`owner.rs:819–925`, so `line_of` / `derive` / `bom_json` run exactly as for a hand-typed recipe). One object
  turn per image, not per row (`one-image` stays 0).
- The console: an "Import" button on the Stock pane and on the Menu pane's recipe section, copying
  `menu.js:89–101` verbatim (file input → dry run → apply, warnings listed by row).
- **The coverage instrument**, because the first coverage number this platform produced measured nothing
  (`dowiz-stock-ledger-works-but-is-off`, trap 2): `e2e/gates/recipes.mjs` reads `GET /api/owner/products`
  (the owner route carries `bom`; the public menu never does) and `GET /api/owner/stock`, and prints
  `dishes 165 · with recipe N · supplies M · dishes whose every food line has kcal K`. It is a report, not a
  ratchet, until the operator says which number a venue must reach before "stock" is switched on for it.

**F2 — templates and suggestions.**

- `tools/recipes/templates/<cuisine>/<category>.csv` in the F1 recipe format, with the supplies they need in
  `tools/recipes/templates/<cuisine>/supplies.csv`. The first set is sushi (both venues); a template row carries
  the qty for a portion of the *typical* dish of that category.
- `GET /api/owner/recipes/suggest?product=<id>`: for a dish with no `bom`, answers (a) the recipe of the nearest
  named dish in the same category that has one, scaled to the dish's `weightG` when both are known, else (b) the
  category template, else nothing. The answer is a draft: it is written only when the owner presses apply,
  through F1's route. No scoring, no ranking beyond "same category, nearest name"; the rule is one function
  with its cases as tests.

**F3 — the priced receipt as a supplier invoice.**

- `POST /api/owner/stock/receipt`: `{ supplier, doc, lines: [{ supply, qty, unit_cost, per }] }` → one
  `received` per line carrying `cost.rs`'s fields, **all in one object turn**, and **idempotent on
  `(supplier, doc)`** through `idempotency::guard` (the four routes that use it are the pattern): the same
  invoice entered twice lands once. The console's Stock pane gets a "Receipt" sheet with the supplier, the
  document number and a line table; the eBills purchase read (§1.2) is the future *source* of the same body
  and needs nothing else.
- The margin then appears where the tax already does: the line's `Stamp` at placement (`cost.rs`), and the
  waste report (`lib.rs:371`) can finally say what the waste cost.

**What is deliberately NOT proposed:** photo/OCR capture; a FIFO ledger (refused in `cost.rs`, "FIFO needs lots");
making stock mandatory before selling (the stance in `storefront.rs` stands: an unmodelled dish reserves nothing).

---

## 2. G2 — Aggregators: what operates in Albania, what has an API, what needs a signature

### 2.1 What exists (measured)

- `workers/api/src/command/aggregator.rs` (237 lines): an order typed from the platform's tablet; id
  `<channel>-<external id>` (path-safe since `b154b97c`; the colon form 404'd on every console action), so the
  platform's id **is** the idempotency key (`order_id`, `:66–80`; `existing` checked in the object's turn);
  the money is the platform's (`price_trusted: false`, `total == lines − discount` refused otherwise, so law 3
  holds); commission not recorded; PICKUP; payment `platform`. Route `POST /api/staff/orders/aggregator`
  (`lib.rs:322`), console sheet `public/admin/aggregator.js`, object half `hubdo/aggregator.rs` (22 lines).
- The closed channel set: `services/ordering/channel.rs:32–34` (`wolt`, `glovo`, `baboon`), `marketplace_word`
  at `:134`.
- The seam was designed in BLUEPRINT-OPERATIONAL-BLIND-SPOTS §2.9 (inbound adapter → `place` with `channel`,
  born CONFIRMED; `OutOfStock` → reject + availability update; status mapped outbound) and its §5.3 holds the
  market record as of 2026-09-23. This section deepens the API facts and states what is impossible.

### 2.2 The market (web, 2026-09-24)

| Platform | In Durrës? | Partner API | Credentials | Verdict |
|---|---|---|---|---|
| **Wolt** | Yes since March 2025 (§5.3 of BLIND-SPOTS: Reuters/SeeNews) | Order, Menu, Venue APIs; order webhook | Only from Wolt: "You can get credentials by contacting your Wolt account manager or technical account manager" ([Order API](https://developer.wolt.com/docs/api/order)); onboarding = an integration request form → "our minimum requirements and T&Cs" → test credentials → a sandbox "with just a few minor constraints (e.g., no real couriers)" ([Getting started](https://developer.wolt.com/docs/getting-started/restaurant), [FAQ](https://developer.wolt.com/docs/faq)) | **Buildable, blocked on Wolt's word.** Nothing can be tested before Wolt answers the form |
| **Glovo** | Tirana; "limited expansion to Durrës" (BLIND-SPOTS §5.3) | Partner API v2 (Delivery Hero infrastructure): OAuth 2.0 client credentials, `Authorization: Bearer`, stage `https://sandbox.partner.deliveryhero.io/v2`, prod `https://glovo.partner.deliveryhero.io/v2`; webhooks `RECEIVED / READY_FOR_PICKUP / DISPATCHED / CANCELED`; `PUT /chains/{chain_id}/orders/{order_id}` (ACCEPTED, DISPATCHED, READY_FOR_PICKUP); catalog `POST/PUT /chains/{chain_id}/vendors/{vendor_id}/catalog` ([redoc](https://partner-api-docs-tmp.s3.us-east-2.amazonaws.com/redoc-glovo.html), [Partners API](https://api-docs.glovoapp.com/partners/index.html)) | "Glovo generates `client_id` and `client_secret` … through the Partner Portal, under the Shop Integrations Plugin section" | **Buildable after Wolt**, same shape; dowiz would be a "chain" with one vendor per venue. The redoc host is a temporary S3 URL — the canonical page is JS-rendered and could not be read here (UNVERIFIED that it matches) |
| **Baboon** (Albanian; Tirana, Durrës, Vlorë; 1,000+ merchants) | Yes | **None found.** In-house platform; no developer page, no API mention on [Albania Tech](https://albaniatech.org/show_cases/baboon-the-most-required-home-delivery-platform-in-albania/), [ACTI](https://acti.al/baboon-en), the [Play Store](https://play.google.com/store/apps/details?id=al.baboon) | **Impossible without a bilateral agreement.** Stays manual (`aggregator.js`) |
| **Bolt Food** | **No.** Albania is not among Bolt Food's countries ([Wikipedia](https://en.wikipedia.org/wiki/Bolt_Food), list as of 2023) | Bolt Stores API exists elsewhere ([developer portal](https://developer.bolt.eu/stores)) | Out of scope; remove from any Albanian list |

### 2.3 The Wolt intake, designed against the wire

**What Wolt sends** ([Webhook](https://developer.wolt.com/docs/webhook)): a POST with `id`, `type`, and
`order { id, venue_id, status, resource_url }`, signed with **`WOLT-SIGNATURE` = HMAC-SHA256 over the raw body
with the venue's client secret**; events `CREATED, PRODUCTION, READY, DELIVERED, CANCELED`; **must be answered
`200`**, otherwise retried after 5 s and twice more at 5 s (three retries, then lost). The order's lines are
NOT in the webhook: fetch `resource_url` (develop against `/v2/orders/{orderId}`, the most complete payload).
Actions: `PUT /orders/{orderId}/accept | reject | ready | delivered`. Menu: `POST /v1/restaurants/{venueId}/menu`,
availability and price `PATCH /venues/{venueId}/items`, items carry `external_data` (the venue's own id) —
[Menu API](https://developer.wolt.com/docs/api/menu). Auth header for the Order API: UNVERIFIED (the page speaks
of a JWT from an OAuth 2.0 flow; older integrations used an API-key header) — read from the credentials Wolt
issues, not from memory.

**The pieces, each one reusing something that exists:**

1. **Per-venue secrets in the venue's own image**, exactly as ebills does (`ebills/state.rs:11–16`: one Worker
   serves every venue, so a Worker secret would bind the platform to one venue's Wolt account): `wolt` image
   with `venue_id`, `client_secret`, `api_key`, `enabled`, `last_error`. Written by an owner route, never read
   back.
2. **The public hook** `POST /api/hooks/wolt` on the venue's host (`https://<slug>.dowiz.org/api/hooks/wolt`;
   the host names the venue, `Place::slug_of_host`, `hubstore.rs:112–116`), verified by
   `WOLT-SIGNATURE` before anything is parsed — constant-time compare, refuse with 401 and write nothing.
   `order.venue_id` must equal the image's `venue_id` or the hook answers 404 loud (`loud!`).
3. **Fetch with an allow-list**, copied from `ebills/{client,judge,fetch}.rs`: `Path::Order { id }` is the only
   GET; every refusal shape is a named failure. Done *inside* the hook's invocation — Wolt gives three retries
   five seconds apart, so a fetch failure answers **5xx on purpose** to be retried, and only a mapped, placed
   order answers 200.
4. **Map → the existing command.** Wolt lines → `aggregator::Entry { channel: "wolt", external_id: order.id,
   lines, discount, total }` (`command/aggregator.rs:47–57`); a line's `external_data` is the dowiz `product_id`
   when the menu was pushed by dowiz (step 6), else the crosswalk (`ebills/map.rs`'s rule); an unmapped line
   stays `wolt:<sku>` and draws no stock, flagged on the console, **never refused** — Wolt has already taken the
   order. `total` is Wolt's item total minus Wolt's discounts; delivery and service fees are Wolt's money and are
   not lines (law 3 holds by the same refusal as today). Idempotency = `wolt-<order.id>`: a retried webhook
   returns `existing: true` and 200.
5. **Status, both ways.** After placement: `PUT …/accept` (the order is born CONFIRMED; if Wolt's venue is set to
   auto-accept the PUT is a no-op and is still sent, so one code path). FSM → Wolt: `READY` → `ready`; for a
   Wolt-courier order `delivered` is Wolt's, not ours (`courier_id` absent → no PUT); a dowiz cancel before
   accept → `reject` with the reason the console asks for (BLIND-SPOTS §2.9: this is the case Wolt charges for).
   Wolt → dowiz: `CANCELED` → the refund route (`lib.rs:323`) with `platform` as the method, so the stock hold
   is released (the FSM has that exit since `b154b97c`). Every outbound PUT goes through the **outbox** rail
   (`outbox/rails.rs`), which already retries, abandons loudly after six and shows the waiting count in
   `/api/owner/health`.
6. **Menu: dowiz is the source of truth, one direction.** `POST /api/owner/wolt/publish` pushes the catalogue
   (`POST /v1/restaurants/{venueId}/menu`) with `external_data = product_id`; an `OutOfStock` refusal or an
   owner's "hide" enqueues one outbox entry per SKU → `PATCH /venues/{venueId}/items` (the automatic 86,
   propagated — BLIND-SPOTS §2.9's prove case: exactly one entry per SKU).

**Failure modes, each with its answer:** bad signature → 401, nothing written; unknown venue → 404 + `loud!`;
fetch fails → 5xx, Wolt retries thrice, then the order is **lost to the webhook** — and there is no Wolt list
endpoint in the docs read (UNVERIFIED) — so the manual sheet stays as the fallback and a `wolt.missed` gauge
counts hook invocations that never reached placement; duplicate → 200 `existing`; item unmapped → placed with a
flag; PUT accept fails → outbox retry, and if abandoned the console shows "Wolt was not told" (the venue's
tablet still has the order, so nothing is silently lost); clock: `now_ms` from `Req`, never `Date::now()` in the
handler (`clock` gate).

**What cannot be built or tested from here:** anything past step 1 without Wolt's sandbox credentials. The form
is the operator's act; until it is answered the row is **BLOCKED (external)**, and the manual entry is the
channel. Glovo is the same design with `Authorization: Bearer` from a client-credentials exchange, a `chain_id`
and per-venue `vendor_id`, and `PUT /chains/{chain}/orders/{id}` for the outbound status. Baboon is out until a
signature exists.

---

## 3. G3 — The courier credential: the flow exists; the code is never delivered

### 3.1 What exists (measured)

| Step | Where |
|---|---|
| Owner mints an invite: phone + name → a 16-character code, **shown once**, stored hashed, TTL 7 days | `services/courier/hiring.rs:1–60` (`invite_courier`), route `lib.rs:395`; console `public/admin/couriers.js:45`; uninvite `:32`, route `lib.rs:396` |
| Courier claims: phone + code + a password of ≥ 8 → signed in on the spot | `accounts.rs:743` (`courier_claim`; "no invite" and "wrong code" are one answer, expiry told apart), route `lib.rs:305`; app screen `public/courier/app.js:525–556` |
| Courier logs in: the **host decides the venue**; a body `location_id` that disagrees is a 403 | `accounts.rs:530–640` (`venue_for_login`, `venue_of_host`), fixed and deployed 2026-09-21 (`courier-login-venue-from-host`) |
| The sushi-durres courier | `e350e12c` (`+355690000050`), created by invite + claim on 2026-09-21; credentials `QA_COURIER_PHONE` / `QA_COURIER_PASSWORD` in `/root/.dowiz_owner` (file mtime 2026-09-21 10:17, keys present — measured; values not read) |

So the sentence "the courier for sushi-durres has no usable credential" describes the morning of 2026-09-21,
before the memory note's RESOLVED paragraph. **It was not re-verified here** (a login is a write of a session
and was not attempted); the CHECK below is that verification.

### 3.2 What is actually missing

1. **Delivery.** The code exists only on the owner's screen for one render (`couriers.js:45`). The owner reads
   it out or retypes it into a chat; a courier who is not in the room gets it by a route dowiz does not control.
   The venue already has outbound rails with retries (`outbox/rails.rs`: Telegram, WhatsApp) — the invite can
   be *sent*.
2. **Where to type it.** The claim screen is reached from the login screen (`toLogin` / claim toggle,
   `app.js:537–538`); a courier sent a bare code must first find `https://<slug>.dowiz.org/courier/`, then find
   the claim form. A link can carry the phone and open the claim form directly; **it must never carry the
   code** (a link in a chat log would be the credential).
3. **A second venue.** An owner with two venues invites on the venue the console is on (`owner_and_venue`
   resolves the named location, then the token's, fixed 2026-09-21) — right, but the console does not show which
   venue the roster belongs to. One line of copy on the Couriers pane ("Couriers of <venue>") is the fix.

### 3.3 The design (F5)

- `POST /api/owner/couriers/invite` gains `send: "whatsapp" | "telegram" | null`. With a rail configured, the
  code goes out as an outbox entry to the courier's phone (WhatsApp, if the venue's Meta config is on —
  `channels.rs`) or to a Telegram user id the owner pastes; the answer says `sent: true|false` and the code is
  shown once *as well* only when nothing could send it. The outbox's six-attempt abandon is visible in health.
- The invite link: `https://<slug>.dowiz.org/courier/#claim=<phone>` opens the claim form with the phone filled;
  `app.js` reads the hash once and clears it.
- The Couriers pane names the venue.
- **CHECK, and it is a live one:** with the operator's word, `node e2e/walk/courier.mjs --claim` (F6) mints an
  invite on `sushi-durres`, follows the link in a browser, claims with a fresh password, logs in on
  `sushi-durres.dowiz.org`, sees the pool, and is then uninvited/deactivated; separately, the existing
  `QA_COURIER_*` pair logs in on `sushi-durres.dowiz.org` and gets `courier.locationId == "sushi-durres"`
  (ids, not slugs — `dubin-sushi` is `dubin-durres`).

---

## 4. G4 — Walking features live: a harness, not another test suite

### 4.1 Why this is the right shape (measured)

- `b154b97c` (this morning) lists six defects found "by doing the flow on the live venue, not by reading": a
  404 on every Wolt-order action (`wolt:` in a path), a "cancel" the machine refuses past PENDING and no screen
  calling refund, a printer that never queued, an alert that never asked after voids/comps, wallet audit routes
  with no button, a room that could not open a table so no tip was ever possible. Native tests: green throughout.
- 2026-09-21: three defects the same way (`dowiz-live-bugs-only-a-browser-found`): the socket's 500, the owner's
  second venue, the courier pool — "63 Worker tests, 265 hub tests, a clean wasm check and a green design gate
  all passed".
- The pattern is not "tests are weak". The tests prove the kernel and the object; what breaks is the seam
  between a *screen* and a *route*: an id encoded one way and routed another, a button that does not exist, a
  transport the product degrades gracefully around.

### 4.2 What this box can run (measured)

| Fact | Value |
|---|---|
| Browsers | `~/.cache/ms-playwright`: `chromium-1243`, `chromium_headless_shell-1243`, `firefox-1543`, `webkit-2359`, `ffmpeg-1011` |
| Library | `node_modules/playwright` + `playwright-core` (no `@playwright/test` — a supply-chain entry this repo refuses, `e2e/kit-regression/README.md`); node v22.22.1 |
| System chromium | none (`which chromium chromium-browser google-chrome` → nothing) |
| WebGL | renders nothing here (`webgl-renders-nothing-on-the-box`): the map and the Sea cannot be captured; judge them by request counts only |
| Already written and run against live venues | `e2e/kit-regression/cycle-full.mjs` (three contexts in ONE browser: customer, console, courier; `watch()` records pageerror / console.error / responses ≥ 400; `page.on('websocket')` counts handshakes; walks the order to DELIVERED, falling back to the API rather than leaving litter; `MARK = "QA hh:mm:ss"`), `flows-owner.mjs` (nine owner write cycles, each cleans up and says whether it managed), `surface-sweep.mjs` (every screen behind a login, writes nothing), `stock-cycle.mjs`, `run.mjs` (render / mobile / interact / pwa / domains / outbox / courier-cold), `drain-stuck-orders.mjs` (the caretaker), `e2e/gates/conservation.mjs` |
| Not walked by anything | the **room** (`public/room/*`: open a table, amend, pay, split, till open/count/close, transfer, tips), refund / returned, the printer queue, the exceptions pane, the ebills pane, the aggregator sheet, staff invite/claim, courier claim, the platform page's hub creation |

The process cap is the constraint that shapes it: one Chromium is 5–8 OS processes, so the harness launches
**one browser and N contexts**, never a browser per role, and never in the background (`box-sigkill-phantom`).

### 4.3 The design (F6)

`e2e/walk/` — one file per role, one shared harness, one coverage map, one report per run.

**`_harness.mjs`** (lifted from `cycle-full.mjs`, then tightened):
- `watch(page, tag)`: `pageerror`; `console` type `error` **from our origin** (the exclusion list is *named*:
  `CF$cv`, WebGL/GPU, `favicon`, tile 404s — and nothing else; today's list also swallows "Content Security",
  which must NOT be swallowed: `render.mjs` proves a CSP break is the defect this suite exists for); a
  `securitypolicyviolation` listener on the document for our origin; every response ≥ 400 from `HOST` unless the
  step *declares* it expects one (`expect409('stock')`); `page.on('websocket')` counted per surface and asserted
  **exactly once**.
- `api(path, opts)` with the role's token; **every UI step is followed by a read-back**: the order's status from
  `/api/owner/orders`, the till from `/api/staff/till/*`, the stock from `/api/owner/stock`, the product from
  `/api/owner/products` — "the screen updated" is never the proof (`dowiz-live-bugs-only-a-browser-found`).
- `mark()`: the order's note/customer name carries `QA-WALK <ISO>`; the harness registers every id it creates.
- `ensureClosed()` in a `finally`: every registered order is walked to a terminal state by the API — PENDING →
  cancel; CONFIRMED…IN_DELIVERY → `POST /api/staff/orders/:id/refund` then `returned` (`lib.rs:323–324`; the
  exit that did not exist when `drain-stuck-orders.mjs` was written); DELIVERED/REFUNDED stay. Then it asserts
  `rebuild.stranded == []` and that no `QA-WALK` order is live. A cleanup that fails is a FAIL line, never
  silent.
- `report`: `e2e/walk/out/<ISO>/report.json` + one screenshot per step; the exit code is the verdict and is
  *written into the report* (`lane-verdicts-need-triggering`: a quoted line without its exit code is not a
  verdict).

**The four flows**, each asserting what the feature *promised* in its roadmap row:

| Flow | Steps (each a read-back) |
|---|---|
| `guest.mjs` | menu loads (cards, 3 languages, price in lek); dish sheet; basket; checkout for delivery / pickup / dine-in (address asked only for delivery — the A1 promise); order placed → `/api/order/:id` says PENDING; tracking sheet socket opens once; refund shows on the sheet after `waiter` refunds it |
| `owner.mjs` | login on the venue host → `active_location_id` is this venue; orders pane: confirm → preparing → ready (each read back); assign courier; the Wolt sheet: enter `wolt-<n>` twice → second is `existing`; every action on that order answers < 400 (the `b154b97c` case); refund sheet on CONFIRMED (reason, note) → REFUNDING → "money handed back"; printer tile: setting on → a ticket queued (`GET` the queue); exceptions pane: threshold edit, audit dry-run then apply; ebills pane: link state shown, disconnect forgets the password; stock pane: a receipt (F3), a waste with a signer; menu pane: CSV dry run (menu, supplies, recipes); couriers pane names the venue; staff invite → code |
| `waiter.mjs` | staff login on the host; **open a table** (`room/open.js`) → a `dine_in` round exists (`/api/staff/room`); amend (add, remove with reason, comp); kitchen ack; pay cash with no open till → 409 expected; open till → pay → split; tip; transfer to another table; till count → close → `health.till.over_short` is the number typed; floor: sitting cleared |
| `courier.mjs` | login on the host (id check); shift on; the pool shows the walked order; take → pickup → deliver; earnings; `--claim` mode = §3.3's CHECK |

**`FEATURES.md`** — the coverage map: one row per roadmap id (A1–A13, B1–B9, C1–C6, F-rows as they land) →
the flow and step names that walk it, or `NOT WALKED`. A feature lane's verdict is incomplete until its row is
not `NOT WALKED`; that is the rule that turns this from a script into a harness.

**Where it runs (F7).** A walk rings a real kitchen: the bell is written by the placing turn (`outbox`), Telegram
and WhatsApp go out, and a real courier's pool changes. So the default `HOST` is a **QA venue on the platform**
(`qa-durres.dowiz.org`), created through the platform's own two steps (`POST /api/platform/hubs`,
`platform.rs:275`; `tools/platform/attach-host.sh`), seeded with the sushi menu CSV and F1's supplies/recipes,
its Telegram pointed at the operator's own chat. That venue is also G7's soak target. A `--real` flag walks a
real venue outside opening hours with the operator's word, because some seams (the ebills link, a real
courier roster) exist only there. **Frequency:** every feature lane's CHECK names its flow; a full walk of all
four before every deploy — the deploy that ships `b154b97c` should be the first.

---

## 5. G5 — The 39 files over 300 lines, and how each split is one lane

### 5.1 Measured

`sh tools/gates/file-size.sh` (2026-09-24): `39 file(s) over 300 lines (baseline 39); worst 1671 in
workers/api/src/hubstore.rs (baseline 1671)`. Scope: `workers/api/src`, `crates/dowiz-hub/src`,
`crates/bebop-store/src`, **`tests.rs` files excluded** (`file-size.sh:27–34`). The top of the list:

| Lines | File | Of which inline `#[cfg(test)]` (measured by span) |
|---|---|---|
| 1671 | `workers/api/src/hubstore.rs` | 1243–1250 (a helper) + **1327–1495 ≈ 170** |
| 1640 | `crates/dowiz-hub/src/lib.rs` | **904–1640 = 737** (three test modules) |
| 1560 | `workers/api/src/hubdo.rs` | 257–365 ≈ 108 |
| 1477 | `workers/api/src/owner.rs` | 0 |
| 1418 | `crates/dowiz-hub/src/stock.rs` | 634–840, 1103–1194, 1286–1418 ≈ **430** (plus three `mod x_tests;` already out) |
| 1391 | `workers/api/src/storefront.rs` | 0 |
| 1152 | `workers/api/src/booking.rs` | 964–1152 ≈ 189 |
| 1012 | `crates/dowiz-hub/src/roster.rs` | 584–1012 ≈ 429 |
| 971 | `crates/bebop-store/src/evlog.rs` | 500–971 ≈ 472 |
| 959 | `workers/api/src/accounts.rs` | 38 |

Two observations decide the order. **Half of the top ten is inline tests**, and the gate does not count a
`tests.rs`: moving a test module to its own file is a zero-risk cut that drops `dowiz-hub/lib.rs` from 1640
to ~900, `evlog.rs` to ~500, `roster.rs` to ~580, `stock.rs` to ~990 — with `cargo test -- --list` as the
proof it moved nothing. **The rest is sectioned already**: `booking.rs` has `// ────` rulers at 57, 264, 564,
785; `hubstore.rs` reads top-down as Place / loaders / seeds / folds / rotation / backup; `hubdo.rs` is
`impl HubImages` in two halves (storage + commands, 375–1147) and the fetch dispatcher (1170–1492).

### 5.2 The split plan, one lane per row, no behaviour change

Rust lets an inherent `impl` be spread across files in one crate, so `impl HubImages { … }` in
`hubdo/commands.rs` needs only `use super::HubImages;` — no type moves, no signature changes. Every moved
function stays reachable at its old path through `pub use` in the root file, so no caller changes. **Rule for
every split: no new file over 300 lines**, or it is the same problem in more places.

| Lane | File | What moves where | Result |
|---|---|---|---|
| **F8** | `hubstore.rs` 1671 | `Place` + `claimed_venue` (66–282) → `hubstore/place.rs`; loaders and `with_*` (323–657) → `hubstore/images.rs`; `seed_fresh_hub`, `seed_catalog` (658–732) → `hubstore/seed.rs`; fold reads + appends (733–1029) → `hubstore/fold.rs`; `HOT_KEEP_MS`… `archive_orders`, `order_state`, `orders_state` (1030–1308) → `hubstore/archive.rs`; `IMAGES`… `import` (1495–1671) → `hubstore/backup.rs`; tests (1327–1495) → `hubstore/tests.rs` (`carry.rs` already is) | root ≈ 120 lines of consts and `pub use` |
| **F9** | `dowiz-hub/lib.rs` 1640 | three test modules (904–1640) → `src/tests.rs`, `src/audit_tests.rs`, `src/usage_tests.rs` (`#[cfg(test)] mod tests;`); then `EventKind` / `HubError` / `Usage` (66–300) → `kinds.rs` + `error.rs`; `decode*`, `Quarantined`, `ChainCheck`, `content_id` (775–900) → `record.rs` | lib ≈ 480; a second cut of `Hub` (302–775) → `hub.rs` gets it under 300 |
| **F10** | `hubdo.rs` 1560 | tests → `hubdo/tests.rs`; `Change`, tags, `Fix`, `OrderView` (102–212) → `hubdo/view.rs`; `changed_chunks`, `chunk_bytes`, `image`, `put_image` (243–443, 1060–1147) → `hubdo/storage.rs`; `orders_view`, `append`, `place`, `advance`, `assign`, `rebuild`, `enqueue_bell` (444–1059) → `hubdo/commands.rs` (+ `bell.rs`); `fetch` dispatcher (1170–1492) → `hubdo/routes.rs`; websocket handlers (1492–1560) → `hubdo/sockets.rs` | root: `HubImages` struct + `DurableObject` impl delegating |
| **F11** | `owner.rs` 1477 | `orders`, `assign_courier`, `order_action` (262–611) → `owner/orders.rs`; `dashboard` (612–700) → `owner/dashboard.rs`; `update_product` + `i18n_*`, `apply_i18n`, `write_translations` (701–1204) → `owner/product.rs` + `owner/i18n.rs`; `update_location` + `hex_colour`, `clean_stage`, `deserialize_some` (1205–1477) → `owner/location.rs` | root: `owner_and_venue` and shared helpers |
| **F12** | `stock.rs` 1418 | inline tests → `stock/ledger_tests.rs`, `stock/log_tests.rs`, `stock/bom_tests.rs`; `encode`/`decode` (524–633) → `stock/codec.rs`; `StockLog` (840–1100) → `stock/log.rs`; `BomLine`, `bom_of`, `reservations_for`, `settle` (1194–1285) → `stock/bom.rs` | root: events, errors, `StockLedger` ≈ 500; second cut: `StockLedger` impl (218–503) → `stock/ledger.rs` |
| **F13** | `storefront.rs` 1391 | input types (18–191) → `storefront/input.rs`; `menu` (192–659) → `storefront/menu.rs`; `manifest`, `short_name`, `png_size` (660–748) → `storefront/manifest.rs`; `place` (749–1391, 640 lines) → `storefront/place.rs`, and inside it the address/contact validation vs. the pricing vs. the command call as three files | the only lane with a design decision (where `place` cuts); it should read the `command/place.rs` seam first |
| **F14** | `booking.rs` 1152 | along its own rulers: model + folds (1–263) → `booking/model.rs`; `detail`, `list`, `create`, `action` (174–563) → `booking/reservations.rs`; floor (564–784) → `booking/floor.rs`; pass key + base64 + `issue_pass`/`verify_pass` (785–963) → `booking/pass.rs`; tests → `booking/tests.rs` | root: routes' `pub use` |
| then | `roster.rs`, `evlog.rs`, `accounts.rs`, `cloud.rs`, `graph.rs`, `auth.rs`, `bebop-store/lib.rs`, `courier.rs` | tests out first (roster 429, evlog 472 lines of tests), then by section | the ratchet falls one file per lane |

Lanes F8–F14 are file-disjoint (each declares its `mod` lines inside the file being split, never in
`lib.rs`), so any number may run at once — subject to the box's worker cap, and with `git status` of the
main tree after each (`lanes-write-outside-their-tree`).

### 5.3 How "zero behaviour change" is proved

1. **The test list is the contract.** `cd <crate> && cargo test -- --list > before.txt` at the parent commit,
   the same after; `diff` must be empty except for the module path prefix of moved tests (a test named
   `hubstore::tests::a_log_of_deltas…` may become `hubstore::tests::a_log_of_deltas…` — identical — or gain a
   segment; the *set of test names* after stripping module paths must be byte-identical). Counts today:
   `workers/api` 221, `dowiz-hub` 317, `dowiz-core` 3,647, `native-spa-server` 166 (ROADMAP §0).
2. **Every gate reads the same numbers**: `for g in tools/gates/*.sh; do sh $g; done; python3
   tools/gates/unreached.py` — identical output except `file-size`, which must *fall* and have its baseline
   lowered in the same commit (`file-size.sh:60`).
3. **The bytes the stores write are golden already**: `bebop-store`/`dowiz-hub` golden tests (EvLog v2 was built
   "oracle-first golden", `dowiz-hub-seven-phases`) fail on any change to what is written; a split of
   `stock.rs` or `evlog.rs` that alters one byte is caught there, not by review.
4. **The move is visible as a move**: `git diff --color-moved=dimmed-zebra <parent>` must show every removed
   hunk as moved except lines matching `^\s*(mod|use|pub use|pub\(crate\) use)\b`, `#[cfg(test)]` and the file
   headers. A reviewer rejects a split whose diff contains an unmoved deletion or insertion of code.
5. **Wasm still builds** (`cargo check --target wasm32-unknown-unknown` in `workers/api`, under `slot.sh`) —
   visibility is the one thing a move can silently change (`pub(crate)` vs `pub(super)`), and the compiler is
   the gate for it. The Worker's wasm bytes are NOT expected to be identical (function order moves); the proof
   for the Worker is 1 + 2 + 5 and, after the deploy, the live read-back the roadmap already uses
   (`/api/owner/health` fields, both storefronts 200, conservation's eight laws).

**Why not a mechanical tool.** `cargo-modules`/`rust-analyzer`'s extract-module can do the move, and a lane may
use it; the proof above is what makes the result acceptable, not the tool.

---

## 6. G6 — Post-quantum: the live path exists since last night and is switched off

### 6.1 What exists (measured)

| Piece | Where | State |
|---|---|---|
| ML-KEM-768 | `crates/dowiz-core/src/pq/kem.rs:1–27`: "VERIFIED (P91.2, 2026-09-23): every ML-KEM-768 vector of NIST ACVP-Server … 80 vectors"; `kem/acvp_tests.rs`; vectors vendored under `pq/kat/acvp/` | **KAT green** (operator's first condition met). Header still lists: no constant-time review; only `_internal` (seeded) algorithms; `ek_check` (§7.2) run by `hybrid_encaps_to` |
| The hybrid | `pq/hybrid.rs:1–13` ("X25519 + ML-KEM-768, BOTH mandatory … no classical-only fallback"); `hybrid_encaps_to` (`:84–99`) refuses a malformed ML-KEM key and a low-order X25519 key; `combine` (`:152–168`) | see §6.3 — the combiner is NOT X-Wing's |
| The seal | `pq/backup_seal.rs:1–31`: hybrid KEM + AES-256-GCM DEM; file layout `DWZSEAL` v1; AES key = `SHAKE256("dowiz/backup-seal/v1" ‖ ss ‖ header[0..1173])` — every header byte (both ciphertext legs, nonce) is bound; public key text `dwzseal-pk1:`; secret = two 32-byte seeds | pure, tested |
| The live path | `workers/api/src/cloud/seal.rs:1–24`: three states **Off / On / Refused**; `PK_VAR = "BACKUP_SEAL_PK"` (var, then secret); `cloud.rs:417` `seal::apply` in the nightly push; `:419` `loud!("… is not set: this copy is NOT sealed")`; `:488–489` `sealed` + `seal.describe` on the push answer, the status route and `/api/owner/health`; `.sealed` suffix on the object key (`:354–360`) | **wired; `Off` until the var is set** |
| The opener | `tools/seal-open` (`keygen`, `pubkey`, `open`) | on this box |
| Commit | `8ae71778` "pq: ML-KEM-768 passes all 80 NIST ACVP vectors; nightly copies sealed with it" — 2026-09-23 19:34 | **Deployed? UNVERIFIED.** No deploy record after `b9bde1aa`/the courier fix was found in the tree; `BACKUP_SEAL_PK` appears in no `.md`, `.sh` or `.toml` (grep, measured: 0) |

**What is encrypted by dowiz today, on a live path: nothing.** Tokens are HS256 — HMAC, not encryption
(`auth.rs:16–33`), and a MAC over SHA-256 is not weakened by a quantum adversary in any way ML-KEM would fix.
The venue's eBills password, S3 keys and Meta tokens sit at rest in the venue's own images
(`ebills/state.rs:11–16`, by reference to `settings`'s header). The transport is Cloudflare's TLS — which
already negotiates X25519MLKEM768 with modern browsers, and is Cloudflare's to claim, not dowiz's.

### 6.2 Which live path is honest and worth it — each candidate judged

| Candidate | Verdict | Why |
|---|---|---|
| **The nightly off-site copy** | **YES — and it is the one already built.** | It is the only dowiz artefact that leaves Cloudflare: every venue's whole history including customer PII, into a third-party bucket, kept for years. "Harvest now, decrypt later" describes exactly a bucket of backups. The Worker holds only the public key; the secret exists only where the operator keeps it (`backup_seal.rs` header). Honest wording follows: "off-site copies are sealed to a hybrid X25519 + ML-KEM-768 key" |
| Per-venue secrets at rest (eBills password, S3, Meta) | **AGAINST** | The Worker must *decrypt* them to use them, so the decryption key would live in the Worker beside the ciphertext; a KEM to a key the same process holds is symmetric encryption with a slower name. What protects those secrets is the object's isolation and the routes that never read them back — already the design |
| Tokens | **No change** | HMAC-SHA256 is post-quantum as it stands; "PQ tokens" would be a claim with no adversary |
| Customer ↔ hub end-to-end (tracking sheet, room) | **AGAINST, for now** | WebCrypto has no ML-KEM; it would mean the wasm module in every browser doing a KEM over a channel TLS already protects with the same hybrid. Re-entry: the offline/mesh transport (BLIND-SPOTS §2.11, `bebop-wasm`), where dowiz *is* the transport |
| Code signing of the wasm / images | Later | `pq/codesign.rs`, `hybrid_signing.rs` exist (ML-DSA-65 is also ACVP-exact); a signed image is a different claim ("integrity"), and not the one the landing made |

### 6.3 The construction, against the current standards

- **NIST SP 800-227 final, 2025-09-18** ([NIST](https://www.nist.gov/publications/nist-special-publication-800-227-recommendations-key-encapsulation-mechanisms)):
  the hybrid is "run each component KEM in parallel and combine their outputs into a single shared secret using
  an approved key combiner" — the combiner must be a KDF over *both* secrets and should bind the ciphertexts.
- **X-Wing, draft-connolly-cfrg-xwing-kem-11 (2026-09-23)** ([datatracker](https://datatracker.ietf.org/doc/html/draft-connolly-cfrg-xwing-kem)):
  exactly X25519 + ML-KEM-768; `ss = SHA3-256(ss_M ‖ ss_X ‖ ct_X ‖ pk_X ‖ label)`, label bytes
  `5c2e2f2f5e5c` (`\.//^\`); the ML-KEM ciphertext is deliberately *not* hashed ("we do not need to mix in its
  ciphertext, see Section 6" — ML-KEM-768 is ciphertext-binding on its own); pk 1216 B, ct 1120 B, ss 32 B;
  IND-CCA in the QROM from ML-KEM-768's IND-CCA and X25519's gap-CDH; it satisfies the HPKE KEM interface.
  The draft carries **test vectors**.
- **TLS: draft-ietf-tls-ecdhe-mlkem-05** (X25519MLKEM768 = ML-KEM ss ‖ X25519 ss, 64 bytes into TLS's own KDF)
  — IESG-approved, in the RFC Editor's queue as of 2026-07 ([datatracker](https://datatracker.ietf.org/doc/draft-ietf-tls-ecdhe-mlkem/)).
  The plain concatenation is safe *there* because TLS's transcript hash binds every public value; a
  standalone KEM has no transcript, which is why X-Wing binds `ct_X ‖ pk_X` itself.

**dowiz's `combine` (`hybrid.rs:152–168`)**: `ss = SHAKE256(SHAKE256(ss_M ‖ ss_X))[..32]` (the comment says
"sorted concat"; the code hashes `a = ss_M ‖ ss_X` and *discards* `b`), and a **confirmation tag
`SHAKE256(ss)` transmitted in the clear** (`HybridCiphertext.confirm`, `:66`). Three differences from X-Wing:

1. **`ct_X` and `pk_X` are not bound at the KEM level.** For the *seal* this is repaired one layer up — the AES
   key binds the whole header (`backup_seal.rs:22–25`), so dwzseal v1 is sound as a whole — but `hybrid` as a
   reusable KEM is weaker than what it will be compared to, and it cannot be checked against X-Wing's vectors.
2. **The public tag is a hash of the shared secret.** It gives a decapsulator a cheap "wrong key" answer, which
   the AEAD tag already gives; what it costs is a public commitment to `ss` that no standard construction
   emits. Under a random-oracle assumption it leaks nothing; it is still a non-standard element that a reviewer
   will have to be argued past every time.
3. **`tag_eq` is `==` in production** (`hybrid.rs:126–133`: constant-time only under `test` or `ct-gate`) — a
   timing side channel on the receiving side, which for the seal is the operator's laptop, not the Worker.

### 6.4 The design

**F15 — make `hybrid` X-Wing-exact and gate it on the draft's vectors.** `hybrid::combine` becomes
`SHA3-256(ss_M ‖ ss_X ‖ ct_X ‖ pk_X ‖ label)` (SHA3-256 is in `keccak.rs`); the `confirm` field is dropped
(decapsulation failure is the AEAD's tag, as in X-Wing/HPKE); `hybrid_keygen` derives the X25519 and ML-KEM
seeds from one 32-byte seed with SHAKE256 as the draft's `expandDecapsulationKey` does, so the draft's
`DeriveKeyPair` vectors apply. **Gate:** `pq/kat/xwing/` vendored from the draft's appendix, `hybrid/xwing_tests.rs`
asserting keygen / encaps / decaps byte-exact, with the group count asserted (the ACVP file's shape). The seal's
KDF label goes to `dowiz/backup-seal/v2`, the version byte to 2, and `seal-open` opens both versions — v1 files
already written stay readable.

**F16 — switch it on (operator, one hour).** `tools/seal-open keygen ~/.dowiz_seal.sk` on a machine that is
NOT the Worker; `wrangler secret put BACKUP_SEAL_PK` with the printed `dwzseal-pk1:…`; deploy; after the
next 03:17 UTC nightly: `GET /api/owner/health` → `backup.seal.sealed == true` and `scheme` names the
construction; the S3 listing shows `…json.gz.sealed`; **`tools/seal-open open ~/.dowiz_seal.sk <that file>
out.json.gz` on this box opens it and `gunzip -t` passes** — the round trip is the proof, and the memory rule
(`dowiz-pq-and-investment-decisions`) is met only then: KAT green AND a live path using it.

**F17 — the words, after F16.** README Security and the landing say what is true and nothing more: "nightly
off-site copies are sealed with hybrid X25519 + ML-KEM-768 (X-Wing) and AES-256-GCM; the secret key never
exists on the platform". Not "post-quantum encryption" unqualified. `ct-gate` on in the release build of
`seal-open` (the receiver) in the same commit.

---

## 7. G7 — Dozens of venues: what binds at 10, 50, 200

### 7.1 The shape (measured)

One Worker (`dowiz-api`, `workers/api/wrangler.toml`), one Durable Object class `HubImages` (`new_sqlite_classes`),
**one object per venue** named by the venue id (`hubstore.rs:66–70, 89`) plus **one platform object** `__platform`
(`platform_store.rs:36`) holding `registry` (512 KiB ceiling, `:60`), `identity` (2 MiB), `sessions` (4 MiB),
`couriers` (1 MiB), `waitlist` (1 MiB). Per venue the image ceilings are: log 4 MiB, catalog 1 MiB, settings
256 KiB, posts 512 KiB, stock **8 MiB**, people / ops / i18n 2 MiB each, bookings 2 MiB, roster / subs 512 KiB
(`lib.rs:61`, `catalog.rs:21`, `settings.rs:26`, `post.rs:27`, `stock.rs:858`, `hubstore.rs:833–853`,
`booking.rs:121`, `roster.rs:29`, `subs.rs:28`) — ≈ 24 MiB if every image reached its ceiling; images are
trimmed on write since phase 1.5.

Two crons (`wrangler.toml [triggers]`: `17 3 * * *` and `* * * * *`; `lib.rs:518–533`). **Both fan out
serially over every venue in the registry** (`registry.all(K_LOC)`): the minute one runs `outbox::sweep`
(`outbox/rails.rs:138–163`: one `waiting()` read per venue = one DO request via `with_table`, `:15–27`) and
`ebills::poll::sweep` (`ebills/poll.rs:33–64`: one `ebills/tick` DO request per venue whether or not ebills
is enabled, `:41`, then the upstream fetches for enabled venues); the nightly (`cloud.rs:562–600`) reads
settings, sweeps idempotency keys, rotates, backs up, per venue in a loop. `rails.rs:130–134` already names
the wall: "forty object wakes a minute whether or not anything is queued … NOT built now, because … two venues
do not need one".

Onboarding a venue is two steps (`dowiz-platform-on-dowiz-org`): `POST /api/platform/hubs` (`platform.rs:275`,
registry write + five images) and `bash tools/platform/attach-host.sh <slug>` — a Workers **custom domain** per
venue, because the wildcard route needs `Workers Routes:Edit`, which `/root/.cf_deploy_token` lacks
(`attach-host.sh:4–17`). Then, by hand: owner login, menu CSV, supplies/recipes (F1), ebills link, Telegram
bot, courier invites, Stripe keys, S3 bucket for the nightly.

### 7.2 What binds, in the order it binds

Inputs: per venue after phase 6 ≈ 3,900 Worker+DO requests/day at 30 orders/day (BLUEPRINT-HUB-COST §1 table;
**modelled, not measured** — no token here can read a request count); the minute cron's own 2 DO requests per
venue per minute = 2,880/venue/day (derived from `rails.rs:138–163` + `poll.rs:33–64`); Cloudflare limits
read 2026-09-24 ([Workers limits](https://developers.cloudflare.com/workers/platform/limits/),
[DO limits](https://developers.cloudflare.com/durable-objects/platform/limits/)). Which plan the account is on is
**UNVERIFIED from this box** (the cost blueprint's fact table says Workers Free; the memory note says a $5
subscription); both columns are given.

| Bind | Free plan | Paid plan | At 10 | At 50 | At 200 | Where the number comes from |
|---|---|---|---|---|---|---|
| **Minute cron: DO requests from the sweep alone** (2/venue/min) | 100k DO req/day total | $0.15/M | 28.8k/day | 144k/day — **over free on its own** | 576k/day ≈ $2.6/mo | `rails.rs:138`, `poll.rs:33`; the ebills tick fires for venues with ebills OFF too (`:41` before `:48`) |
| **Minute cron: subrequests in one invocation** (2/venue + upstream fetches) | 50 external + 1,000 to Cloudflare services | 10,000 | 20 | 100 | 400 + ebills fetches | limits page: "Workers on the free plan remain limited to 50 external subrequests"; every ebills GET is external → **≈ 25 ebills-enabled venues is the free ceiling for the poller** |
| **Minute cron: CPU** | **10 ms per Cron Trigger** | 30 s (< 1 h interval) | the wasm fold of even one venue's outbox table is near 10 ms | over | over | limits page, "CPU time per Cron Trigger"; on free this may already be failing silently — an instrument is needed (F18) |
| **Minute cron: wall time** (serial, ~20–50 ms per DO hop) | 15 min | 15 min | < 1 s | ~3 s | ~10–20 s per firing | derived; it fits, but a slow ebills upstream (Front Door 403s, `EBILLS` §1.1) is inside the loop and one slow venue delays every venue after it |
| **Nightly cron: wall time** (5 image reads + gzip + seal + S3 PUT + witness per venue) | 15 min | 15 min | fine | ~2–4 min | **~10–15 min — at the limit** | derived at 3–4 s per venue; the loop is serial (`cloud.rs:580`) |
| **Custom domains per zone** | 100 | 100 | fine | fine | **refused at the 101st venue** | limits page "Custom domains per zone: 100 … consider using a wildcard route"; `attach-host.sh` header says exactly what unlocks it |
| **Worker requests** (3,900/venue/day) | 100k/day | 10M/mo included, $0.30/M after | 39k/day | 195k/day — over free | 780k/day ≈ 23M/mo ≈ $4/mo | cost blueprint §1 (modelled) |
| **DO requests** (≈ 1,500/venue/day of the 3,900, + the sweep) | 100k/day | 1M/mo incl., $0.15/M | fine | over free | ≈ 12M/mo ≈ $1.7/mo | same |
| **DO storage** | 5 GB/account | 10 GB/object, unlimited account; SQLite storage billed from Jan 2026 | 0.24 GB at full ceilings | 1.2 GB | 4.8 GB — near the free cap only if every image were full | ceilings above; DO limits page |
| **The platform registry** (512 KiB ceiling) | — | — | fine | fine | **UNVERIFIED**: a `K_LOC` record is ~200 B of JSON plus a slug lookup; if the `Table` layout has the log's old 8× cell amplification (phase 4b "Kv packing" was deliberately not done, `dowiz-hub-seven-phases`) the ceiling holds ≈ 300 venues, else thousands | `platform_store.rs:60`; `platform.rs:316–323`; measure it (F21) |
| **Workers Logs** | 20M events/mo free | $0.60/M over | fine | ≈ 6M/mo at `head_sampling_rate = 0.1` | ≈ 24M/mo ≈ $2/mo | `wrangler.toml [observability]`; the cost blueprint's "$25 at 40 venues" predates the sampling |
| **WhatsApp** | — | per message from 2026-10-01 | $7–80/venue/mo | ×50 | ×200 — **the largest line by far, and a setting** | cost blueprint §1 |
| **Worker memory 128 MB** | per request | per request | the four remaining whole-image reads (`dowiz-hub-seven-phases`, phase 2) are per request, so venue count does not stack them | | | not a scaling bind |
| **Onboarding hands** | 2 shell steps + ~8 console steps per venue | | 20 min/venue | a day of clicking | a week | §7.1; F19 removes step 2, F1 removes the longest console step |

**Reading the table:** on the free plan the platform is already at its limits at about 4 venues (the cost
blueprint said so); on paid, **nothing in Cloudflare's money binds before 200 venues** (all lines together
≈ $10–15/mo at 200) — what binds is *the two serial crons and the 100-domain rule*, all three of which are
dowiz's own shape, not Cloudflare's price.

### 7.3 The design

**F18 — the crons stop fanning out from the platform.** Each venue's object owns its own schedule with a
**Durable Object alarm** (`ctx.storage.setAlarm`; alarms get 15 minutes each and there is no fan-out limit
because there is no fan-out): the outbox drain runs when the *enqueue* sets the alarm (the thing
`rails.rs:130–134` said would replace the sweep once it was needed), the ebills tick sets its own next alarm
from its own plan (`poll.rs:48`: floor 60 s / sales 300 s / backoff), and the nightly is an alarm at
03:17 venue-local (`tz.rs` has the zone). The platform's minute cron then does nothing per venue and can go;
the nightly platform cron survives only to prune platform images. **Instrument first:** a `cron` record in the
platform object — `{ fired_at, venues, ms, failures }` per firing, shown on `/api/platform/health` — so the
serial loop's wall time is a number before it is replaced, and the alarm version can show the same number per
venue. **CHECK:** with the QA venues of F20 at N = 50, `cron.ms` under the sweep vs. the per-venue alarm; the
sweep's DO request count per minute falls from 2N to the number of venues with something queued (asserted from
the platform record, not from the dashboard nobody here can read).

**F19 — the wildcard route (operator, ten minutes).** Widen `/root/.cf_deploy_token` with `Workers Routes:Edit`
on `dowiz.org`; put `*.dowiz.org/*` back in `wrangler.toml` (below the `[vars]` trap noted in
`dowiz-platform-on-dowiz-org`); `attach-host.sh` becomes unnecessary and the 100-domain limit stops applying.
Then `create_hub` is the *whole* onboarding of the address, and a venue answers on its host the moment the
registry row exists. **CHECK:** `curl -sI https://qa-x.dowiz.org/` answers 200 with no attach step, and the
existing two custom domains still answer.

**F20 — a soak that touches no real venue.** The `preview` environment already has a name and a pattern
(`wrangler.toml [env.preview]`, `preview.dowiz.org`): a second Worker with its own `HUB` namespace and its own
platform object. `e2e/soak/venues.mjs` creates N venues through `POST /api/platform/hubs` (slugs `qa-<n>`),
seeds each with the sushi menu CSV and F1's supplies/recipes, then for each venue places k orders through the
storefront route with `QA-SOAK` marks and walks them with the API. Measures, all readable from here: the
platform `cron` record (F18), each venue's `/api/owner/health` gauges (image `used_per_mille`, `errors`),
p50/p95 of the storefront and console reads under N venues (one `fetch` loop, no browser), and the registry's
`used_per_mille` at N = 10 / 50 / 200. Cleanup: the preview namespace is deleted, not the venues. No production
venue is named anywhere in the script — `HOST` must match `/preview\.|qa-/` or it refuses to run.

**F21 — the registry's real capacity.** A native test in `workers/api` that writes `K_LOC` records with realistic
JSON into a `Table` at `REGISTRY_BYTES` until `arena_full` and prints the count — the "ceiling, not capacity"
lesson (`bebop-ceiling-not-capacity`) applied before the number matters. If it is in the hundreds, the fix is the
phase 4b Kv packing or a larger ceiling; either way it is known, not inferred.

**What is deliberately NOT proposed:** Workers for Platforms (dispatch namespaces) — a second billing line and a
second deploy path for a problem the wildcard route solves at zero; sharding venues across Worker scripts —
one Worker per venue is the twin-server anti-pattern in the cloud; per-venue KV/D1 — `no-sql` holds.

---

## 8. What could not be determined from this box

| Question | Why it matters | The command or person that settles it |
|---|---|---|
| Is `8ae71778` deployed, and is `BACKUP_SEAL_PK` set? | G6's live path is `Off` until both | `curl -s -H "Authorization: Bearer $OWNER" https://sushi-durres.dowiz.org/api/owner/health \| jq .backup.seal` (owner token from `/root/.dowiz_owner`) — a read, and `wrangler secret list` with the deploy token |
| Does the `QA_COURIER_*` pair still log in on `sushi-durres.dowiz.org`? | G3's premise | `e2e/walk/courier.mjs` (F6) — a login is a session write, so with the operator's word |
| Which Workers plan is the account on? | every row of §7.2's table has two columns | the Cloudflare dashboard; or `wrangler whoami` + the billing page |
| Is the minute cron already over its CPU limit on the free plan? | a cron that dies at 10 ms is a bell that never rings and an ebills poll that never runs, silently | the Workers dashboard's cron invocation status (no token here reads it); F18's `cron` record makes it readable from the platform itself |
| How many cells does one `K_LOC` record cost in the registry `Table`? | §7.2's UNVERIFIED row | F21, or `/api/platform/health` if it gains the registry gauge |
| Wolt's Order API auth header and whether an order LIST endpoint exists | §2.3 steps 3 and the "lost after three retries" failure mode | Wolt's credentials pack after the integration form |
| Whether Glovo's canonical Partner API page matches the S3 redoc read here | §2.2 | a browser on `api-docs.glovoapp.com/partners/index.html` |
| The venue's eBills purchasing side | §1.2 | the owner, in ebills: is inventory switched on; then `GET /api/item-in-warehouses` with the poller's session |

**Sources read 2026-09-24.** Wolt — [Order API](https://developer.wolt.com/docs/api/order),
[Webhook](https://developer.wolt.com/docs/webhook), [Menu API](https://developer.wolt.com/docs/api/menu),
[Getting started for restaurants](https://developer.wolt.com/docs/getting-started/restaurant),
[FAQ](https://developer.wolt.com/docs/faq); Glovo — [Partners API](https://api-docs.glovoapp.com/partners/index.html),
[Partner API redoc](https://partner-api-docs-tmp.s3.us-east-2.amazonaws.com/redoc-glovo.html);
Baboon — [Albania Tech](https://albaniatech.org/show_cases/baboon-the-most-required-home-delivery-platform-in-albania/),
[ACTI](https://acti.al/baboon-en), [Google Play](https://play.google.com/store/apps/details?id=al.baboon);
Bolt Food — [Wikipedia](https://en.wikipedia.org/wiki/Bolt_Food), [developer portal](https://developer.bolt.eu/stores);
NIST — [SP 800-227](https://www.nist.gov/publications/nist-special-publication-800-227-recommendations-key-encapsulation-mechanisms),
[announcement 2025-09](https://www.nist.gov/news-events/news/2025/09/recommendations-key-encapsulation-mechanisms-nist-publishes-sp-800-227);
IETF — [X-Wing draft-11](https://datatracker.ietf.org/doc/html/draft-connolly-cfrg-xwing-kem),
[draft-ietf-tls-ecdhe-mlkem](https://datatracker.ietf.org/doc/draft-ietf-tls-ecdhe-mlkem/),
[draft-ietf-tls-hybrid-design](https://datatracker.ietf.org/doc/draft-ietf-tls-hybrid-design/16/);
Cloudflare — [Workers limits](https://developers.cloudflare.com/workers/platform/limits/),
[Durable Objects limits](https://developers.cloudflare.com/durable-objects/platform/limits/),
[SQLite in DO GA](https://developers.cloudflare.com/changelog/post/2025-04-07-sqlite-in-durable-objects-ga/).
