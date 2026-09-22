# The last mile: what a waiter's hands, a kitchen's printer and an owner's old spreadsheet need from a POS that lives in a browser on Cloudflare Workers

**Date:** 2026-09-22. **HEAD read:** `bc6db359` ("tables: a table is free or taken only FOR A SLOT", 2026-09-22).
**Tree state at time of reading:** `git status --short | wc -l` = 13 — the three sibling blueprints of this
session (`BLUEPRINT-CRM-CONSENT-LOYALTY`, `BLUEPRINT-EBILLS-INTEGRATION`, `BLUEPRINT-TAX-PRICE-CHANNEL`,
all untracked), `workers/api/src/ebills/` (untracked), and a tables lane's working files. **This document
changes nothing in the tree.** `BLUEPRINT-POS-THE-ROOM-2026-09-22.md` and `BLUEPRINT-STACK-AND-DEPENDENCIES-
2026-09-22.md`, which `ROADMAP-2026-09-22.md` names as IN FLIGHT, **do not exist on disk yet** (measured:
`ls docs/design/BLUEPRINT-POS* docs/design/BLUEPRINT-STACK*` → "No such file"). Where this document
touches a waiter, a tab or a split bill, it designs the INTERACTION and inherits that lane's data model
unread; §7 says so again.

**Method.** Every "today" statement names a path and the line where it was read. `measured` means a command
was run on this box on 2026-09-22 and its output is quoted; `hypothesis` means it was not. External facts —
browser support tables, printer protocols, what iiko, R-Keeper and Poster export — carry a source in §8, and
every one of them was fetched today rather than remembered; where a fetch failed, §7 says which. Where the
brief's framing and the tree disagree, §0 says so first.

**The brief translated.** The operator says the stack is a foundation and names three things a restaurant
buys: (1) an interface a stressed waiter can drive in "3 seconds and 2 taps", (2) the physical world —
kitchen printers, drawers, scanners, card readers, fiscal registers — reached from a browser with no native
app and no local agent, and (3) a way in from the system they already run. The constraint that shapes all
three: **dowiz is a browser on a Cloudflare Worker.** §1 measures what exists; §2 is the research, with the
gamedev material the operator asked for applied to a waiter rather than a player; §3 is the design; §4 the
refusals with re-entry conditions; §5 the gates; §6 the order of work with a CHECK each; §7 the open questions.

---

## 0. The brief's framing, checked against the tree

| # | The brief says | Verdict | Evidence |
|---|---|---|---|
| 1 | `public/lib/replica.js` and `public/lib/outbox.js` | **Path wrong, substance right.** The files are `workers/api/public/lib/replica.js` and `workers/api/public/lib/outbox.js`; `public/lib/` at the repo root does not exist. The header quoted ("a PREDICTION in exactly the sense game netcode means") is at `workers/api/public/lib/replica.js:14-18`. | `find . -name replica.js` (measured) |
| 2 | "`public/lib/outbox.js` queues a courier's tap offline" | **True, and further along than the brief knows.** The queue is in IndexedDB with the key minted at tap time (`outbox.js:21-25, 77-80`), the courier app wraps every action in `tapped()` (`courier/app.js:337-358`), four server routes carry `idempotency::guard` (`idempotency/mod.rs:193-200`), and a CI gate pairs the two halves so a queueable route without a guard is refused (`tools/gates/idempotent.sh:1-27`, `ci.yml:73`). | as cited |
| 3 | "`crates/dowiz-hub/src/import.rs` and `POST /api/owner/menu/import`" | **True.** Route at `workers/api/src/lib.rs:347`; handler `services/catalogue/import.rs`; parser `dowiz_hub::import::from_csv` with 17 tests (measured `grep -c '#[test]'`). §1.5 lists exactly what it covers. | as cited |
| 4 | "`a18025d4`-era commits show a menu import that refused a missing price column" | **Half right.** `a18025d4` is the status-vocabulary commit ("a customer who COLLECTED their order could never leave a note"); the price refusal is in the parser itself, `import.rs:251-257` (header must carry name AND price) and `:137-166` (`parse_price` refuses any fractional or ambiguous price), pinned by `a_missing_price_column_is_an_error_not_an_empty_menu` (`import.rs:474`) and `fractional_prices_are_refused_not_guessed` (`:373`). The standard the brief names is real; the commit is the wrong one. | `git show --stat a18025d4` (measured) |
| 5 | "the kernel decides, the Durable Object is the single writer, commands execute in one object turn" | **True since this morning, and `BLUEPRINT-ARCHITECTURE-EVOLUTION` is stale on it.** `command/place.rs`, `advance.rs`, `assign.rs` execute in the object (`hubdo.rs:650, 747`), `one-image` reads 0, D1 is gone (`ROADMAP-2026-09-22.md` §0, §2). That blueprint's §1.2 ("D1 is not gone … 103 handles") and §1.4 describe the tree as it was at `c5c640cc`. | `ROADMAP-2026-09-22.md:20-29` |
| 6 | "`workers/api/src/errlog.rs` exists because a sampled trace lost nine of ten 500s; `otel.rs` puts one span per request" | **True, with the honesty already written into the file:** `otel.rs:3-12` says the header USED to claim child spans and never had any — "ONE SPAN PER REQUEST, the root. That is all." Any hardware bridge span would be the first child span in the crate. | `otel.rs:1-27`, `errlog.rs:1-14` |
| 7 | "No native app and no local agent today" | **True.** `grep -rniE 'escpos\|thermal\|printer\|webusb\|web serial\|navigator\.(serial\|usb\|bluetooth)\|cash drawer\|barcode\|fiscal'` over `workers/api/src`, `workers/api/public`, `crates/` finds nothing in product code; the hits are docs, the courier crate's battery model and one shell spike (measured). And the edge headers **switch WebUSB off for every page**: `Permissions-Policy: … usb=() …` (`workers/api/public/_headers:42`). | measured grep; `_headers:42` |
| 8 | "the interface has to be … 2 taps" | **The console is already at one tap for the common step, and one of its buttons is under the size token.** The gold step button on an order row fires the transition directly (`admin/orders.js:96-98, 143-150`); `.act` has `min-height:40px` (`admin/admin.css:165`) while the tap token is `--tap: 44px` (`lib/tokens.css:96`) and the mobile audit's floor is 44 (`e2e/kit-regression/mobile.mjs:29`). That audit **is not in CI** (measured: `grep -n mobile.mjs .github/workflows/ci.yml` → nothing). | as cited |
| 9 | "the CRM lane is deciding the consent model" | **Decided on paper:** a `consent` LogImage, `c`/`w` kinds, one unticked box at checkout, and the rule "anything that is not about an order this person placed … is marketing and requires a `given` consent" (`BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22.md` §3.2). §3.3 here inherits it verbatim. | that document, lines 387-414 |
| 10 | (implicit) the tax lane may own the printer | **The tax lane recommends AGAINST ESC/POS and a receipt printer** because "the venue's receipts are printed by its fiscal terminal (EBILLS §1.5)", re-entry "a venue without a fiscal terminal that prints, or a courier-side proof-of-delivery slip" (`BLUEPRINT-TAX-PRICE-CHANNEL-2026-09-22.md` §4 item 7). That is about the FISCAL receipt. The KITCHEN ticket is a different document with no fiscal content, and today it is a Telegram message (§1.3). This document is about that ticket, and agrees with the tax lane about the receipt. | that document, §3.8 and §4 |

Two more corrections the design leans on:

- **Albania's tablets are half iPads.** StatCounter, tablet OS share, Albania, August 2026: **iOS 50.04 %,
  Android 49.95 %** (fetched today, §8). Any recommendation that needs Web Bluetooth, WebUSB or Web Serial
  excludes half the tablets in the country; §3.1 says so at each step.
- **The console has no first-run tour; the courier app does.** `lib/guide.js` ("the first-run tour and the
  per-control hints, ONE module for every surface", `guide.js:1-9`) is imported by `courier/app.js:11` and by
  nothing under `public/admin/` (measured grep). The onboarding-as-tutorial work in §3.2 starts from a module
  that exists and a surface that does not use it.

---

## 1. Ground truth

### 1.1 The surfaces, and what a person's hands do on them today

Four served surfaces under `workers/api/public/`: `admin/` (the owner's console, 8 modules, 2,147 lines by
`wc -l`), `courier/` (one 1,100-line app), `store/` (the storefront) and `kit/` (the customer's installable
app). There is **no waiter surface**: `Principal` is exactly `Owner | Courier | Customer`
(`workers/api/src/auth.rs:338-346`), which is A4 of the roadmap, in flight elsewhere.

| Measured | Console | Courier | Note |
|---|---|---|---|
| Taps to advance an order | **1** — the row's gold `.act.pri` button carries `data-act` and `doAction` posts on click (`orders.js:96, 143-150`) | **1** (`tapped()`), plus a 90 %-of-track swipe to finish a delivery (`courier/app.js:1050-1061`) | already at the brief's target for the one-step case |
| Confirmation sheets | **8** call sites of `confirm()` (`couriers.js:76, app.js:133, stock.js:119, orders.js:143, more.js:57, more.js:434, menu.js:158, menu.js:246`; the sheet itself `core.js:194-207`) | **1** native `window.confirm` (`app.js:1061`, the keyboard/AT fallback for the swipe) | reject/cancel on an order asks for a reason inside the sheet (`orders.js:143`), which is the good kind (§2.7.3) |
| Undo | **0** (`grep -rni undo` → one unrelated landing string) | 0 | — |
| Haptics | `navigator.vibrate` at 5 sites: new-order ring `[80,60,80]` (`admin/app.js:229`), `12` ms on an action (`orders.js:148`), courier `[120,60,120]` (`app.js:220`), storefront tab and dish | — | present, uncoordinated |
| Tap target | token `--tap: 44px` (`tokens.css:96`), used 6× in `admin.css`, 8× in `courier.css`, 16× in `store.css`; **`.act` is 40px** (`admin.css:165`) | `--tap` | the audit that would catch it (`mobile.mjs:29-32`, floors 44/24/16/12) is not in CI |
| Languages | 3 — `sq` default, `en`, `uk` (`admin/i18n.js:1-8`); `vocabulary.sh` refuses a status missing in any (`ci.yml:55`) | 3 | new UI copy must land in three |
| Status never colour alone | the row shows a dot AND the word (`orders.js:84`, `data-t-st`) | — | true by construction on this row; not a gate |
| Reduced motion | honoured in `guide.js:37` (`calm()`) | — | — |
| `:active` feedback | 4 rules (`admin.css`), 5 (`courier.css`) | | |
| Busy state | `busy(el, fn)` **disables the button** and swaps its label for a spinner while the request runs (`core.js:186-192`) | `attend` sweeps an edge | §2.7.4: a disabled button DROPS the early tap |

### 1.2 The prediction and the queue — the netcode that already exists

- **Reads predict.** The console draws `replica.js` before any request, applies socket events locally, and
  "the next successful read replaces the lot. A replica that argued with the server would be a second fold"
  (`replica.js:9-18`). An unknown order is NOT invented from a delta (`:68-71`). Staleness is announced after
  15 minutes (`STALE_MS`, `:28`).
- **Writes survive.** `outbox.js`: IndexedDB, `seq` autoincrement is the order (`:105-111`), `MAX_QUEUE = 64`
  refusing the NEW tap rather than evicting the oldest (`:44-59`), a 409 is "an ANSWER — it is dropped and
  reported" (`:27-32`), `Retry-After` on the in-flight 409 is the one exception (`:82-95`), and a store that
  cannot keep a queue REFUSES at the tap (`:34-38`). The e2e gate runs a naive client beside it in the same
  browser so "the defect existed" is measured in the same run (`e2e/kit-regression/outbox.mjs:19-25`, `ci.yml:123`).
- **The server side.** `idempotency::guard(place, header, principal, route, body, now_ms)` with the five rules
  (`idempotency/mod.rs:12-30`): full response stored, scoped to `(venue, principal, route, key)`, same key +
  different body = 409, in-flight = 409 + `Retry-After`, fails OPEN. `tools/gates/idempotent.sh` pairs every
  `tapped(\`…/verb\`)` in the courier app with a `"courier.<verb>"` guard and says plainly what it cannot see:
  "a route queued by some future surface that does not use `tapped`" (`:22-27`). **A waiter surface is exactly
  that future surface**; §5 G7 extends the gate before the surface exists.
- **Service workers never cache `/api/`** (`kit/sw.js:6-9`; `public/sw.js:3-8` "NETWORK FIRST, ALWAYS").

### 1.3 The kitchen ticket today: a text, an outbox, and a minute

The ticket is `notify::order_text(envelope, lines, currency, venue) -> String` (`workers/api/src/notify.rs:89`):
venue and short id, the customer line, then **the first line is where it goes** — `🥡 pickup`, `🍽 table 7`,
or the address (`:120-129`, tests `:234-259`), the lines with unit and gross, fee, discount, tip, total and
payment kind. It is rendered at enqueue time, deliberately: "the drain runs minutes later and must not re-read
a catalogue that has changed" (`outbox.rs:69-72`).

The entry is written **in the same object turn as the `Placed` event** (`outbox.rs:14-18` — "a write that
landed is a message that will be delivered; a write that did not is an order that was not placed"), keyed by
order id + kind so a retried command is one message (`:62-66`), with `MAX_TRIES = 6` over a 10 s → 10 min
backoff (`:37-57`). The drain is the minute cron (`wrangler.toml:138` `crons = ["17 3 * * *", "* * * * *"]`;
`lib.rs:466-480`; `outbox/rails.rs:34-60`), which today knows two rails, `telegram` and WhatsApp, and treats
an unconfigured rail as "not failed — left where it is" (`rails.rs:48-56`). Depth and the oldest entry are on
`/api/owner/health` (`outbox.rs:22-26`).

**What this means for a printer:** the queue, the retry, the idempotency, the render-at-enqueue rule and the
health surface a kitchen printer needs **already exist**; what is missing is a third rail and a way for a
printer to reach it. The latency floor of the cron path is 0-60 s (the minute tick) plus the first backoff;
a printer that POLLS the Worker (§2.4) bypasses the cron entirely and pays only its own poll interval.

### 1.4 What the edge lets a page reach

`_headers:59` (CSP): `connect-src 'self' wss://*.dowiz.org https://tiles.openfreemap.org https://api.stripe.com
https://nominatim.openstreetmap.org` — **no private address, no `http:`, and `upgrade-insecure-requests`**.
`_headers:42`: `Permissions-Policy: camera=(self), microphone=(), payment=(), geolocation=(self), usb=(),
magnetometer=(), interest-cohort=()` — WebUSB is denied to the page and its frames; `serial` and `bluetooth`
are not named (the browser default for a top-level same-origin page is allow). So a LAN printer at
`http://192.168.1.50` is unreachable from any dowiz page **by our own policy** before any browser rule applies,
and that is correct: the policy documents what the app earned (`_headers:9-13`), and §3.1 changes it only for
the one origin a chosen path needs.

### 1.5 Migration today: what `import.rs` covers, exactly

`dowiz_hub::import::from_csv(text) -> MenuDraft` is pure — "No file I/O, no HTTP, no catalogue writes"
(`import.rs:9-12`) — and the handler is preview-by-default (`services/catalogue/import.rs:12-15`). Covered:

| Rule | Where | Test |
|---|---|---|
| Ambiguity is refused, not guessed; a refused row is named by its spreadsheet row number | `import.rs:14-17, 264-280` | `a_bad_row_is_named_and_the_rest_still_import` `:458` |
| A price with a decimal point or comma-as-decimal is refused ("this menu's currency has no subunit, so write it as a whole number") | `:147-163` | `fractional_prices_are_refused_not_guessed` `:373` |
| Thousands separators incl. U+202F; a currency word either side | `:126-129, 142` | `whole_prices_parse` `:357` |
| Header must carry name and price, else one warning and an empty draft | `:251-257` | `:474` |
| Excel's semicolon dialect detected on the header line; BOM stripped | `:204-210, 215` | `:418, :428` |
| Headers in en / sq / uk / ru (`category, kategoria, категорія, розділ` …) | `:213-229` | `:428` |
| Deterministic slug, non-ASCII kept — so a second import is an update | `:92-122` | `:344, :490` |
| Duplicates named; same name in two categories is two dishes | `:317-331` | `:504, :516` |
| Handler: applying an empty draft is refused (400), because "applying an empty draft would wipe a working menu because of a wrong separator" | `services/catalogue/import.rs:65-73` | — |
| Handler: an existing dish KEEPS photo, size, modifier groups and **allergens** the file has no column for | `:87-108` | — |
| Handler: `?retire=true` marks dishes not in the file unavailable with a note; `menu_version` bumps | `:110-127` | — |

**Not covered, measured by reading:** supplies, recipes/BOM, modifier groups, allergens, translations, photos,
cooking time, price per channel (the tax lane's B3), customers. There is **no** `/api/owner/customers/import`
and no `/api/owner/recipes/import` (`lib.rs:324-330, 347`; measured grep for `import` in `lib.rs` → one route).

### 1.6 The recipe model a tech card must land in

- A dish's bill of materials is `"bom":[{"supply":"salmon","qty":40}, …]`, read by `bom_of` into
  `BomLine { supply: String, qty: Qty }` where `Qty = i64` in the supply's base unit — "a conserved quantity
  that can be 0.30000000000000004 is not conserved" (`stock.rs:23-26, 943-972`). `qty` is ONE portion's use;
  `reservations_for` multiplies and sums per supply and sorts so two hubs agree byte for byte (`:985-1010`).
  A dish with no `bom` reserves nothing and "is not an error" (`:951-957`). 26 tests (measured).
- A supply is `{id, name, unit ∈ {g, ml, unit}, kind ∈ {food_ingredient, condiment, packaging, utensil},
  category, lowAt, kcalPer100/proteinPer100/fatPer100/carbsPer100 (floats, per 100 g/ml or per 1 piece),
  costPerBasis (integer, refused if negative), weightPerUnit, nutritionConfirmed, active}`
  (`services/operations/supplies.rs:60-104`; `recipe.rs:23-31`, `PER_MASS = 100`, `QTY_MAX = 100_000`).
- **There is no nested recipe.** `bom_of` reads flat supply lines; a semi-finished product (a sauce, a
  stock, a marinade) does not exist as a thing a dish can reference. `recipe.rs:1-17` restores "the old
  DeliveryOS model" and says the hub reads only `bom[].supply` and `bom[].qty`.
- **There is one quantity per line.** Not gross/net, not a cooking loss, not a yield. The console's recipe
  editor writes `bom: recipeDraft.map(l => ({ supply, qty }))` (`admin/menu.js:199`).
- Nutrition per dish is either authored (`nutrition`) or derived from the lines in the browser
  (`menu.js:269-276`), with `ML_TO_G: f64 = 1.0` as the one float — a weight, not money (`recipe.rs:38-39`).

### 1.7 Customers today, as the CRM lane measured them

One phone, four handles (`BLUEPRINT-CRM…` §1.1): `customer_key` = HMAC-SHA256 over the digits-only phone
(`services/customers/handlers.rs:18-22`), a `people` record keyed by `sha256_hex(raw phone)` that nothing
reads, a WhatsApp `wa_id`, a booking's `contact_phone` as typed. Two spellings of one number are two keys
(national `069…` vs `+355 69…`). The customer list row is a FOLD — `Row { key, name, phone, orders, spent,
last_at }` (`roll.rs:10-20`) — and the CRM design keeps it so: the record holds "only what a fold cannot"
(`note, tags, allergens, usual_table, lang, birthday MM-DD`), never name/phone/orders/spent (§3.1 there). No
consent record exists anywhere (§0 row 2 there). **An import lands on that design or on nothing.**

### 1.8 Where the tree disagrees with its documents (in this area)

| Document says | Tree shows | Evidence |
|---|---|---|
| `BLUEPRINT-ARCHITECTURE-EVOLUTION` §1.2/§1.4/§1.5: D1 present, decisions in the Worker, no offline writes | all three closed today | `ROADMAP-2026-09-22.md` §0-2 |
| `BLUEPRINT-RESILIENCE-AND-EVOLUTION` §7: "This product has no POS surface … no receipt printer driver … the 'kitchen ticket' is a pane in the owner console" | still true; the ticket is ALSO a Telegram text through the outbox now | `:395-405`; `notify.rs:89` |
| `RESILIENCE` §7 table: "Kitchen printer answers after 10 s — defect #20, open" (half-open socket suppresses polling for 90 s) | not re-verified here | `:423` |
| `otel.rs` header once claimed child spans | corrected in the file itself | `otel.rs:7-12` |
| `mobile.mjs:1-24` describes itself as an audit with published floors | not in CI; the `.act` finding above is exactly what it would report | measured |

---

## 2. Research, made concrete for this codebase

### 2.1 What a browser can reach, by platform (fetched 2026-09-22)

| API | Chrome desktop | Chrome Android | Safari / iPadOS / iOS (every iOS browser is WebKit) | Firefox | What it reaches |
|---|---|---|---|---|---|
| **Web Serial** | 89+ | **152+** (caniuse; MDN says the Android support shipped in the 148 beta, April 2026) | **no, no version**; WebKit's position is "opposed" (fingerprinting) | 151+ | USB-serial and Bluetooth-SPP printers, scales, some drawers |
| **WebUSB** | 61+ | yes | **no** | no | USB printers by vendor/product id — **and our own `usb=()` denies it** (§1.4) |
| **Web Bluetooth** | 56+ | yes | **no**; the only route is a third-party App-Store browser (Bluefy, WebBLE) that ships its own BLE stack — i.e. not Safari, not the installed PWA | no | BLE 58/80 mm printers; libraries exist (`WebBluetoothReceiptPrinter`, `printer-js`) |
| **`fetch` to a LAN address from an https page** | blocked as mixed content **unless** the target is a private-IP literal / `.local` / `targetAddressSpace:"local"`, in which case Chrome ≥ 142 asks a **Local Network Access** permission and, if granted, "relax[es] mixed content blocking for local network requests"; Chrome 145 splits it into `local-network` and `loopback-network` | same rule (the blog does not name Android; hypothesis: same engine, same rule) | **no LNA; a self-signed cert per printer, installed per device** (Odoo's documented ritual; and "creating another certificate causes devices using the previous one to lose HTTPS access") | no LNA | Epson ePOS-Print (HTTP/HTTPS, port 8043 for TLS), Star WebPRNT |
| **HID keyboard wedge** (a USB/BLE barcode scanner typing into a focused field) | yes | yes | **yes** | yes | every scanner sold as "keyboard emulation"; zero driver |

Global usage share for Web Serial is 76.21 % (caniuse, August 2026 StatCounter) — a desktop number that
says nothing about a Durrës dining room where half the tablets are iPads (§0).

**The two facts that decide §3.1:** nothing a browser exposes reaches a printer from an iPad; and the one
thing that does reach a LAN printer from Chrome (LNA) is a per-site, per-device permission prompt that Safari
does not have and that Chrome only introduced two months before this document.

### 2.2 Speaking to a printer on the LAN, directly

- **Raw TCP 9100 is impossible from a page.** No browser API opens a socket; `fetch` is HTTP. This is not a
  limit to design around; it is the end of that road.
- **Printers that speak HTTP** (Epson TM-i / TM-m series with ePOS-Print XML; Star with WebPRNT) accept a
  `POST` of XML from a page. Over `http://` that is mixed content from `https://dubin-sushi.dowiz.org`
  (blocked everywhere except Chrome ≥ 142 with LNA granted). Over `https://` the printer presents a
  self-signed certificate that every tablet must install first; Odoo's own manual for exactly this path is a
  per-OS, per-browser page of steps, with a footnote that Chromium 142 made it unnecessary — on Chromium.
- **Cost, stated as hypotheses:** a per-device certificate install by a restaurant owner (or by us over
  the phone) on every new tablet; a printer whose IP changed after a router reboot (DHCP) silently breaking
  the CSP allow-list; and a support call whose first question is "which browser". None of that is a code
  cost; all of it is a support cost, which is the cost the brief asks about.
- **What it buys:** the only browser path that prints when the venue's internet is down — but only from a
  Chrome tablet, only after the prompt, and only while `connect-src` names the printer's address.

### 2.3 A local bridge (a print agent on a PC or a Raspberry Pi behind the counter)

The shape is a small program on the LAN (QZ Tray, PrintNode's client, or ours) that either exposes
`localhost` to the page (then the page needs the same LNA/`loopback-network` permission as §2.2, and a
certificate for `wss://localhost` on Safari) or polls dowiz for jobs and speaks ESC/POS to the printer over
TCP/USB. The second variant works from every tablet including iPads because the TABLET never talks to it.

- **Who installs it:** somebody must own a machine that stays on. In a Durrës sushi bar that is the owner's
  laptop or nothing. Hypothesis: the install-and-keep-running cost is the whole cost; a Pi in a box that we
  ship pre-imaged is the only version an owner will keep.
- **What it costs in support:** an agent is a second deployable with its own version, its own updates and its
  own logs on a machine we cannot see. The observability rule (`errlog.rs`, "the error is ALSO a record") is
  hard to keep from a process we do not host: every failure it has must be POSTed back as a record or it is a
  console line on a laptop nobody opens — the exact shape `unreached.py:1-16` catalogues ("a capability that
  EXISTS, COMPILES and IS TESTED … and no live path reaches it").
- **When it is the right answer:** the day a venue has a printer that speaks nothing but 9100 and no internet
  a poll can cross. §4 keeps it with that re-entry condition.

### 2.4 A cloud relay where the PRINTER polls dowiz (Star CloudPRNT, Epson Server Direct Print)

Both major vendors ship printers that need no page and no agent: the printer itself is an HTTP client.

- **Star CloudPRNT** (fetched: protocol guide and the POST-poll reference). The printer `POST`s its status
  JSON — `status, printingInProgress, statusCode, printerMAC, uniqueID, clientAction` — every *polling time*
  (a setting on the printer's web page beside *Server URL*, *HTTP response timeout*, *User Name* / *Password*
  "not necessary unless requested by the CloudPRNT server", and HTTPS trust level / NTP / cipher settings).
  The server answers `{ jobReady, mediaTypes, jobToken, clientAction, deleteMethod, claimQueue }`; the printer
  `GET`s the job in one of `text/plain`, `text/vnd.star.markup`, `application/vnd.star.line`,
  `application/vnd.star.starprnt`, `image/png`, `application/pdf`; then `DELETE`s it with a result code.
  **`text/plain` is a media type**, which means the ticket that exists (`order_text`) prints with no encoder
  at all; the drawer kick and the cut are one markup tag each in `text/vnd.star.markup` later. A newer
  variant, *CloudPRNT Next*, uses MQTT instead of polling (out of scope: a Worker is not an MQTT broker).
- **Epson Server Direct Print** (fetched: Epson's overview; the manual PDF downloaded but unreadable on this
  box — §7). "The TM-Intelligent printer sends an HTTP request to the Web server periodically, such as every
  60 seconds … the Web application returns a response that has print data included … in ePOS-Print XML …
  When printing is finished, the TM-Intelligent sends a print completion request." Same shape, XML body.
- **What it buys:** works from every tablet and phone, including every iPad, because the tablet is not
  involved; no certificate, no permission prompt, no agent, no version to update; the print ACK is an HTTP
  request we receive, so "printed" becomes a record, not a hope; the job is served from the venue's own
  `outbox` image by the object that already holds it.
- **What it costs, stated as hypotheses.** (a) Latency = the poll interval; a 5-10 s poll is a 5-10 s ticket,
  which is inside what a kitchen notices and far under today's 0-60 s cron floor (§1.3). (b) Requests: one
  poll every 10 s is 8,640 requests/day per printer, 259,200/month; the memory record for hub cost says
  request COUNTS are what bind (`dowiz-hub-unit-costs`), so this must be priced before a second printer is
  offered — `x-trace-id` on every response already makes it countable. (c) **When the venue's internet is
  down there is no ticket, which is exactly when a kitchen ticket matters most.** The mitigation is not a
  second printer path; it is that the console IS the kitchen display and already draws from its replica
  with no network (`replica.js:9-12`), and the outbox prints the backlog when the line returns, stamped as
  late (§3.1 step 4). A venue whose kitchen cannot see a tablet needs §2.5 on an Android tablet, or §4's
  bridge — and must be told that plainly at sale time. (d) Printer choice: CloudPRNT/SDP hardware costs more
  than a 58 mm BLE printer (hypothesis; no price read).

### 2.5 The one offline path that is browser-only: Web Bluetooth from an Android tablet

`WebBluetoothReceiptPrinter` and `printer-js` print ESC/POS or StarPRNT to BLE printers from Chrome on
Android. `requestDevice` needs a user gesture, and reconnection after a reload is the printer's business
(hypothesis: `getDevices()` is behind a flag on most channels; not verified). It is the only way a page prints
while the internet is down, it costs no server request, and **it excludes every iPad**. It is second in §3.1
for that reason and that reason alone.

### 2.6 The other devices, one line each

- **Cash drawer.** Kicked by the receipt printer (a pulse on its DK port), so it comes free with whichever
  printer path is chosen: one `<drawer>`/`<pulse>` element in Star markup or ePOS XML. No separate driver.
- **Barcode / QR scanner.** Buy the keyboard-wedge kind; it works on an iPad. The whole software problem is
  FOCUS: a scan types into whatever field is focused, so the surface needs one always-focused hidden input
  with a fast-typing detector (a scanner types a 13-digit EAN in < 50 ms; a person does not), and an
  unknown code must be a refusal that names the code — `deny_unknown_fields` is the same rule one layer down
  (`RESILIENCE` §7 table, last row).
- **Card readers.** Stripe Terminal needs a native or JS SDK plus a reader model list; SumUp/bank terminals in
  Albania are standalone. Recommend recording the payment KIND and the terminal's reference on the order
  (the tax lane's `Paid` event that names the fiscal bill, EBILLS §0.5) and NOT integrating a reader now (§4).
- **Fiscal register.** Albanian fiscalisation is software: a sale is reported to the tax authority's CIS with
  `iic`/`fic` codes and printed by the venue's own fiscal terminal (`pointOfSale.printing: THERMAL,
  printerSize: SIZE_80`, EBILLS §1.5 via TAX §0 row 4). The ebills lane owns that seam. **There is no fiscal
  register to drive**, and building a driver would be building the wrong country's POS.

### 2.7 Gamedev, applied — and where the analogy breaks

**2.7.1 Marking (radial) menus.** Kurtenbach & Buxton, CHI '94: with a stylus, experts "stopped looking at
the menu at all and went entirely blind … Using a mark on average was 3.5 times faster than selection using
the menu"; the menu pops only if the user pauses, so a novice and an expert use the same control. Callahan
et al. show radial faster than linear for eight-item menus once the layout is known. Kurtenbach's thesis
puts the accuracy cliff beyond eight items and recommends 4 or 8 with items on the compass points.
- **Maps onto:** the actions on a TABLE or a BILL are a small fixed set — *add a round, split, move, pay,
  void* — and they recur two hundred times a shift, which is the case marking menus were built for.
  Press-and-hold on a table, flick up = add round (repeat the last drinks), flick right = pay, flick left =
  split, flick down = more (the linear list). A novice waits 150 ms and sees the four labels; a waiter in
  week two flicks without looking. That is "2 taps" collapsed into one gesture with a visible fallback.
- **Does NOT map onto:** choosing a DISH. The menu is 165 items (memory `sushi-durres-menu-source`); a
  radial menu cannot hold it and a hierarchic one would be eight-by-eight blind strokes nobody learns in a
  season. Dish entry stays a grid with the category rail under the thumb (2.7.2) and a search that accepts
  a scanner (2.6).
- **Where it breaks:** the 150 ms hold is SLOWER than a visible button for someone who has never used it;
  the win is only for the repeated set. So the four actions are also visible buttons on the open bill; the
  gesture is an accelerator, never the only path — and a gate refuses an action reachable by gesture alone
  (§5 G4).

**2.7.2 Fitts's law and the thumb.** Time to acquire a target grows with distance and shrinks with size; on
a held tablet the reachable arc is the bottom third and the side of the holding hand. Hoober's 1,333
observations: 49 % one-handed, 36 % cradled, 75 % of interactions thumb-driven — phones, but the posture of
a waiter with a plate in the other hand is the one-handed one. Apple's 44 pt and Material's 48 dp are the
minimums; WCAG 2.2 SC 2.5.8's 24 px is the floor `mobile.mjs:29-32` already encodes.
- **Maps onto:** the primary action (the gold step) at the bottom, full width, `--tap` or larger; the
  category rail along the thumb edge; nothing consequential in the top corners (that is where "close",
  "settings", "sign out" belong — `admin/index.html:27-29` already puts them there). The `.act` 40 px must
  become the token (§6 item 1). Hand-side is a per-device setting, not a guess.
- **Where it breaks:** a KDS on the pass is not held; it is a screen a cook bumps with a knuckle. Fitts
  there means big rows and one bump per ticket, not a thumb zone.

**2.7.3 Undo over confirmation.** NN/g: "the only sensible reaction is 'of course I want to do the thing I
just told you to do,' and they hit Yes without further thinking"; habituation makes the dangerous dialogue
the one that gets clicked through; for the genuinely dangerous, require a NON-standard action (type the
word). A List Apart's older formulation — "never use a warning when you mean undo" — is the game rule:
a game lets you act and shows the consequence; it asks only when the act is irreversible AND rare.
- **Maps onto the money path exactly**, because dowiz's money is a ledger of reversals, not edits: an Earn
  and its Reversal net to zero (`money.rs:121-131` per the tax lane §2.7), `Refunding → CompensatedRefund`
  exists, and a void with a reason and a person is A6 of the roadmap. So "undo" on a bill line is a
  compensating event, which is what the FSM wants anyway. And **the outbox gives undo for free on the
  client side**: a tap that is queued with a commit delay is a tap that can be withdrawn before it is sent —
  the same `seq` row, deleted, and nothing ever reached the server. §3.2.5 sets the window.
- **The eight `confirm()` sheets today** split cleanly: reject/cancel an order (with reason — keep, it is the
  "non-standard action" form), delete a dish/category/promo/key, deactivate a courier, retire a supply,
  close the venue. Deletions become soft (the catalogue already has `available:false` +
  `unavailableNote`, `services/catalogue/import.rs:113-116`); "close the venue" is a toggle with a visible
  state, not a question. **What keeps a confirmation:** money leaving the venue (a refund) and erasure —
  rare, irreversible, and NN/g's "type the amount" form, never Yes/No.
- **Where it breaks:** a game's undo is instantaneous and free. A kitchen that has already started cooking
  is a reversal with a cost; the window must be short (seconds), visible (a shrinking bar on the pending
  row), and the ticket must not print until the window closes. That is a design constraint on §3.1 step 3,
  not a nicety: **the commit delay is the same number for the outbox and for the printer.**

**2.7.4 Input buffering and forgiving input.** Street Fighter 6 accepts a follow-up input up to 4 frames
early (a 5-frame window; 7/8 for dashes and reversals); a motion input has an 11-frame window between its
directions. The design intent is that a correct intention at slightly the wrong moment still counts; the
known cost is that an over-lenient buffer produces the WRONG special move from a stray input. Celeste's
"coyote time" (jump accepted a few frames after leaving the ledge) is the same idea in the other direction.
- **Maps onto:** `busy()` (`core.js:186-192`) DISABLES the button during the request, so the second tap of a
  greasy double-tap, or the tap that lands 80 ms before the sheet has finished opening, is dropped on the
  floor. The correct shape is the opposite: accept the tap into the pending state and let the idempotency
  key make the duplicate a no-op — the key is minted at tap time (`outbox.js:21-25`) precisely so that a
  repeat is the same tap. Buffer the intent; never disable the primary. A tap during the sheet's open
  animation is applied when the sheet is ready (the "early input"); a tap within ~120 ms of the previous
  on the same control is the same tap (the "double"), which is what `Idempotency-Key` already says on the
  server. Coyote time maps onto a swipe: the courier's 90 %-of-track rule (`app.js:1050-1052`, "the last few
  pixels are where a thumb runs out of screen") IS coyote time and should be the pattern for every swipe.
- **Where it breaks:** the SF6 complaint is the warning — too much leniency turns a stray tap into a
  wrong action. A buffered tap must land on the control that was under the finger at tap time, never on
  whatever slid under it after a layout shift; the pending row must not move while its window is open.

**2.7.5 Readability under stress — the HUD.** A game HUD shows state at a glance in peripheral vision: few
elements, redundant encoding (shape + colour + position + sound), no reliance on hue alone (about 8 % of
men see red and green as one colour). WCAG 1.4.1 says the same for software. `palette.rs` already refuses a
palette that fails AA — "CONTRAST IS ENFORCED, NOT REPORTED" (`:15-19`), `AA_TEXT = 4.5`, `AA_LARGE = 3.0`
(`:115-118`), `ensure_contrast` walks the colour until it passes (`:129`), and `contrast_report` lists every
pair that must hold (`:391-400`). The order row shows a dot AND the status word (`orders.js:84`) and
`vocabulary.sh` refuses a status without a word in three languages and a colour in three stylesheets.
- **Maps onto:** one more encoding, POSITION/SHAPE — a pending row is offset and hatched, not only amber;
  a refused row carries an icon; the new-order ring is already sound + vibration (`admin/app.js:220-229`).
  Big numerals in tabular figures (`.stat b`, `admin.css:135`, already `tabular-nums`). §5 G5 turns "never
  colour alone" into a gate over the status pill.
- **Where it breaks:** a HUD may flash and animate to draw the eye; a console open eleven hours must not.
  One pulse on arrival (`orow.fresh`, `orders.js:82`) and then stillness.

**2.7.6 Latency and feedback — the netcode.** Client-side prediction + server reconciliation is what
`replica.js` states and what `outbox.js` does for writes. The extension to a waiter's terminal is direct: a
table's bill is a fold over its events; the terminal applies the waiter's tap to its local fold immediately
and marks the row `pending` until the server's event arrives over the socket (`live.js`) or the `?since=`
poll; a refusal (409 from the FSM or the stock ledger's `OutOfStock`, `stock.rs:15-19`) rolls the prediction
back and says why on the row. Nothing here is new machinery: it is `replica.apply` (`replica.js:72-99`) over
table events instead of order events, with `outbox.js` in front of it.
- **Where it breaks:** a game reconciles positions, which are continuous and forgiving; a bill is money,
  which is exact. The prediction must never SHOW a total the server has not confirmed as anything other than
  pending — the rule `replica.js:14-18` already states ("a replica that argued with the server would be a
  second fold"), and the kit's fourth money copy (memory) is what happens when a client computes money.
  So the terminal predicts LINES and STATE, and draws the TOTAL only from the server's last fold, with the
  pending lines listed beneath it un-totalled.

**2.7.7 Onboarding as a tutorial.** A game teaches by having you do the thing in a safe room with the real
controls; the first level is the tutorial. `guide.js` is that room's narration: one declarative table, the
tour and the "?" hint read the same row so "neither can drift from the other" (`guide.js:4-9`), steps whose
element is off-screen are skipped (`:30-33`), state in `localStorage` (`:22-29`), reduced motion honoured.
- **Maps onto a restaurant's first evening:** (a) the console and the waiter surface get the tour the courier
  app already has; (b) a TRAINING venue — a second object, `__training:<venue>`, seeded from the real
  catalogue, on which the first evening's practice orders are placed, whose kitchen ticket prints with a
  banner, and which is discarded at close. Not a flag on real orders: a `training` flag inside the live log
  would have to be excluded from eight conservation laws, the analytics fold and the customer fold, and the
  first one somebody forgets is the day's takings. A separate object costs nothing to fold and cannot leak.
- **Where it breaks:** a waiter is at work, not at play. A tutorial that takes ten minutes on a Friday is
  skipped; the tour must be resumable (it is, `:24-28`) and the training venue must be one tap from the
  login, not a setting.

**The general break, stated once.** A game mechanic is judged by whether it is engaging; a POS control by
whether it is faster and less error-prone than the plain button. "Juice" (spring animations, particles,
screen shake) buys engagement at the cost of time and is out. Every mechanic above is kept only for the
measured reason beside it, and each one has a visible plain-button path for the person who never learns it.

### 2.8 What the incumbents export

| System | How the data leaves it | Auth / where it runs | What a recipe looks like | Customers | Fetched from |
|---|---|---|---|---|---|
| **Poster** (cloud; iPad/Android POS; Ukrainian-origin, common in the region) | Management console → *Products → Dishes/Ingredients → Export* to CSV/XLS/XLSX; a REST API v3 with an account access token from the console (or OAuth for published apps) | cloud, token per account | `menu.getProduct.ingredients[]`: `ingredient_id, ingredient_name, structure_brutto, structure_netto, structure_unit, structure_type (1 = ingredient, 2 = semi-finished), structure_lock, pr_in_clear/cook/fry/stew/bake` (processing-loss flags); `cost`, `cost_netto` "in kopecks"; price per `spot` "in kopecks"; `group_modifications`/`modifications`; `weight_flag`. `menu.getIngredients`: `ingredient_unit: kg / p / l`, numbers as strings (`"443.45000"`), `ingredient_left`, `ingredient_weight`. Semi-finished products via `menu.getPrepacks` with their own `structure_brutto/netto` | `clients.getClients`: `client_id, firstname, lastname, phone, phone_number ("digital format"), email, birthday, client_sex, bonus (kopecks), total_payed_sum (kopecks), discount_per, client_groups_id/name, address, comment, card_number, loyalty_type (points/discount), date_activale, ewallet`; filter by `phone` "in international format" | the docs mirror on GitHub (`joinposter/docs`), `api-php` README, help-centre import/export pages |
| **iiko** (RMS on-prem server + iikoOffice; iikoCloud for delivery) | iikoOffice exports reports and nomenclature to Excel; **iikoServer API** `GET /resto/api/v2/assemblyCharts/getAll?dateFrom&dateTo&includeDeletedProducts&includePreparedCharts` and `getAllUpdate?knownRevision` (iiko 6.0+); iikoCloud `nomenclature` gives groups/products/sizes/modifiers with revision delta but NO recipes | iikoServer API is on the venue's server (`https://host:port/resto/…`), needs a licence and a login; iikoCloud is a cloud API key | `AssemblyChartDto { id, assembledProductId, dateFrom, dateTo, assembledAmount (норма закладки), productSizeAssemblyStrategy: COMMON \| SPECIFIC, items[]: { productId, amountIn (брутто — "именно это поле участвует в расчете списаний"), amountMiddle (нетто), amountOut (выход), amountIn1..3/amountOut1..3 (проработки, in kg), packageCount, productSizeSpecification, storeSpecification }, technologyDescription, appearance, organoleptic }`; `BigDecimal` throughout; **one chart per product per date**; a "prepared" chart (`getPrepared`) is the recipe flattened to leaf ingredients with `amount = ROUND(PRODUCT(ROUND(amountIn/assembledAmount, 4)), 3)` | iikoCard (loyalty) is a separate API; not fetched | `ru.iiko.help` tech-card page (via search index; the site refused a direct connection), SDK READMEs |
| **r_keeper 7** (on-prem; UCS) | the **XML interface** on the cash server: `https://{IP}:{HTTPDataPort}/rk7api/v0/xmlinterface.xml`, Basic auth with an employee's login, UTF-8; `<RK7Query><RK7CMD CMD="GetRefData" RefName="MENUITEMS" onlyActive="1"/></RK7Query>` returns dishes with `Ident, Code, Name, CategPath, PortionWeight, Status` and prices; `GetOrderMenu` for what a station sells now. Recipes ("калькуляционные карты") live in **StoreHouse 5**, which exports reports to Excel/Word and documents as XML | LAN, licensed XML interface, employee credentials | StoreHouse calculation cards: not fetched field by field (§7) | r_keeper CRM / loyalty — not fetched | `docs.restera.com` (the redirect target of `docs.rkeeper.com`), UCS StoreHouse pages |

**Three consequences for dowiz.** (1) Poster is the one with a cloud API reachable from a Worker with a
token the owner can paste; iiko and r_keeper live on a server in the restaurant, so for them the realistic
door is a FILE the owner exports, not an API dowiz calls. CSV first, therefore, for all three. (2) Every one
of them stores a recipe with MORE than dowiz's `{supply, qty}`: gross vs net vs yield, semi-finished
products, processing losses, dated versions. §3.3 says what is kept and what is declared lost. (3) Every one
of them exports money as a decimal or as "kopecks" — a number that must be scaled to the venue's minor unit
with the same refusal `parse_price` already has: lek has no subunit (`import.rs:131-136`), so "kopecks" from
a Poster account set to ALL are either ×1 or a defect, and the import must ASK, not guess.

---

## 3. The design

### 3.1 Hardware: the order, and why the first step is a printer that phones us

**Step 1 — FIRST: a `print` rail on the venue's outbox, served to a printer that polls.**

- **Change.** (a) A third `kind` in `outbox::Entry` — `"print"` — enqueued in the same object turn as
  `Placed` beside the Telegram entry, `text` = `order_text` (it already is), `to` = a printer id from
  `settings` (`print.kitchen`, `print.bar` — the routing the roadmap's A8 wants is one field per line's
  category, decided at enqueue in the object, so a bar ticket and a kitchen ticket are two entries).
  (b) Three Worker routes under `/api/print/`: `POST /poll` (the CloudPRNT status body; answers
  `jobReady`/`mediaTypes`/`jobToken` from the venue's `outbox` — an object read, no cron), `GET /job/:token`
  (answers `text/plain` today, `text/vnd.star.markup` when the cut and the drawer land), `DELETE /job/:token`
  (the ack: `Verdict::Sent` with the printer's result code; a non-zero code is `Verdict::Keep` with the
  code in the entry and, after `MAX_TRIES`, `Abandon` → an `errlog` record the console shows). The Epson
  SDP shape (`ConnectionType`/`ResponseFile` form fields, ePOS-Print XML body) is a second encoder over the
  same three routes, added when a venue owns one. (c) Auth: the printer's *User Name / Password* fields are
  a venue API key (`services/identity/keys.rs`, "shown ONCE and stored HASHED", `:1-4`; a year's life,
  `:18-23`) with the label the owner typed ("kitchen printer"); the key resolves the venue, so `Place::
  of_authorised` holds and the `one-venue` gate keeps counting.
- **Why first.** It is the only path that works from every tablet in the country (§2.1, §0); it needs no
  agent, no certificate and no prompt; the queue, retry, idempotency-at-the-queue and health surface exist
  (§1.3); the ACK is an HTTP request WE receive, so "printed" is a record — the observability rule met by
  construction rather than by a bridge's good behaviour; and `text/plain` means zero bytes of ESC/POS are
  written on day one.
- **The latency.** Poll interval on the printer, recommended 5 s (hypothesis: the vendor default is unknown,
  §7). Against today's 0-60 s cron floor that is an improvement, not a regression. The commit-delay window of
  §3.2.5 is honoured because the entry's `next_at_ms` is set to `now + window` at enqueue — the printer
  cannot see a job the waiter can still withdraw.
- **When the internet is down.** No ticket (stated plainly at sale). The console draws its replica; when the
  line returns the backlog prints in order with a `LATE +4 min` first line, because a kitchen that receives
  six tickets at once must know which were four minutes ago. That line is a rule in `order_text`'s caller,
  tested.
- **Cost.** Requests: see §2.4 — measure one printer for a week on the live venue via `x-trace-id` counts
  before offering a second. Code: the rail (~150 lines, pure verdict logic in `outbox.rs`'s shape, I/O in
  `rails.rs`), three routes (~200 lines), settings keys, one `errlog` place name. Every number a hypothesis.
- **Cash drawer:** one markup tag in the same job, behind `print.drawer_on_cash = true`. **Scanner:** the
  focus rule of §2.6, in the waiter surface, no server change. **Card readers, fiscal register:** §4.

**Step 2 — SECOND: Web Bluetooth from the console, Android only, for the venue with no fixed line.**

- **Change.** A `lib/bleprint.js` (~200 lines, hypothesis) that pairs a BLE printer on a user gesture and
  writes ESC/POS for the SAME `text` the outbox holds — the console subscribes to its own `outbox` entries
  of kind `print` over the socket and prints them locally, then `DELETE`s the job exactly as a CloudPRNT
  printer would. **One queue, two printers**: the object never knows which printed, and a venue can have
  both (the BLE one on the pass for the outage, the polling one for the day). The ESC/POS encoder is the
  smallest one that cuts and kicks: `ESC @`, text, `GS V`, `ESC p`. Nothing else.
- **Why second, and the sentence the operator asked for:** *this step assumes an Android tablet; it excludes
  every iPad in Albania — half the tablets — and it is offered to a venue only as the outage path beside
  step 1, never instead of it.*
- **`Permissions-Policy`** stays `usb=()`; `bluetooth` is not named and stays default. No CSP change.

**Step 3 — the "printed" state on the console.** A ticket entry has four states the console draws: queued,
printing (a `jobToken` was handed out), printed (the `DELETE` arrived), failed (abandoned, with the code).
A venue with no printer configured never sees the column. This is the "visible pending state" of §2.7.3
applied to paper.

**Step 4 — later, and only if a venue asks:** the LAN-direct path (§2.2) as a third encoder behind LNA on
Chrome, with the printer's private-IP literal added to `connect-src` PER VENUE by the settings (the
`_headers` file is static; a per-venue CSP means the Worker sets it on the console's HTML, which the file's
own header explains is where a header must come from for a dynamic page — `_headers:3-7`). Not built now.

### 3.2 The waiter's hands: the interaction grammar

Everything here designs the INTERACTION over the data model A4-A8 will define (open tab, `Amended`,
split/transfer/void, shift). Where a term below (a "round", a "split") lands on that model, the CHECK in §6
is written against its route names once they exist, and §7 lists the dependency.

**3.2.1 The two-tap contract, written down.** The brief's number is a contract, so here it is as a table
the e2e gate can execute (§5 G2):

| Intention | Taps (novice path) | Gesture (expert path) | Time budget |
|---|---|---|---|
| Add a dish to a table | tap table → tap dish (category rail preselects the last-used category) | — | 3 s |
| Add a round (repeat the table's last drinks) | tap table → tap "round" | hold table → flick ↑ | 2 s |
| Send to kitchen | 0 — a line is sent when its commit window closes (§3.2.5); "send now" is a tap on the pending bar | — | — |
| Split the bill | tap bill → tap "split" → tap the seats/lines → tap "done" | hold bill → flick ← → tap seats → done | 6 s |
| Take payment | tap bill → tap "pay" → tap kind (cash/card/…) | hold bill → flick → → tap kind | 3 s |
| Void a line | swipe line → tap reason (from the closed list) | — | 3 s; undo window follows |
| Move a table | tap table → tap "move" → tap destination | hold → flick ↓ → move → destination | 4 s |

Every gesture row has a tap row; §5 G4 refuses a build where one does not.

**3.2.2 Layout by the thumb.** Primary action full-width at the bottom at `--tap` or more; category rail on
the holding-hand edge (a per-device `hand` setting, default right); destructive things top-far; the bill's
total top-centre in tabular numerals; nothing consequential within 8 px of the screen edge where a case
bezel eats the tap. `.act` moves to `min-height: var(--tap)` (§6 item 1). The three profiles of
`mobile.mjs:34-38` become gate profiles plus one tablet (`iPad (768)`, `Galaxy Tab (800)`).

**3.2.3 The marking menu, bounded.** Four directions, never eight; labels appear after 150 ms of hold; a
flick before that is blind; the same four are visible buttons on the open bill. Reduced-motion users get
the labels immediately with no radial animation.

**3.2.4 Buffer, never disable.** `busy()` keeps its spinner and loses `el.disabled = true` on primary
actions; a second tap within 120 ms on the same control is folded into the first (same key); a tap during a
sheet's opening is applied when the sheet is ready. The idempotency key already makes the server side of
this safe (`idempotency/mod.rs:12-30`); this is the client half. The pending row must not move for the
window's duration (no re-sort while pending), which is the stray-input lesson of §2.7.4.

**3.2.5 Undo instead of "are you sure".** Every consequential tap goes through the outbox with a
`commit_at = queued_at + WINDOW` and is drawn as a pending row with a shrinking bar. Tapping the bar
withdraws it (`seq` deleted; nothing was sent). `WINDOW` is one number in one place — **5 s** (hypothesis:
long enough to see a mistake, short enough that a kitchen does not wait; to be measured on the training
venue in the first week) — and the SAME number sets the print job's `next_at_ms` so a withdrawn line never
reaches paper. After the window, undo is a compensating event with a reason: a voided line, a reversal on
the ledger. **The eight `confirm()` sheets** are retired in that order (§6 item 4): deletions become
soft-deletes with undo; venue close becomes a visible toggle; reject/cancel keeps its reason sheet (it IS
the non-standard action); refund and erasure keep a typed-amount confirmation forever.

**3.2.6 Prediction with an honest total.** The terminal applies its own taps to a local fold of the table's
events, marks them pending, and reconciles by `?since=` + socket exactly as `replica.apply` does for orders
(`replica.js:72-99`). The TOTAL is only ever the server's last fold; pending lines are listed beneath it
un-totalled. A refusal (409, `OutOfStock` naming the ingredient — `command/mod.rs:46-57`) rolls the line
back and says why on it, in the waiter's language.

**3.2.7 Readability.** Status = word + shape + colour on every pill (G5); the palette stays server-derived
and AA-enforced (`palette.rs:15-19`); numerals tabular; one pulse on arrival, then still; ring + vibration
on a new kitchen ticket as today.

**3.2.8 The first evening.** (a) `guide.js` on the console and the waiter surface with a table of ≤ 8 steps
each; (b) the training venue `__training:<venue>` — one tap from the login ("practise"), seeded from the
live catalogue at open, its tickets printed with `TRAINING` as the first line, deleted at close (its object
storage `deleteAll`); (c) the tour's last step places one real order on the training venue and shows the
ticket print — the tutorial is the level.

**3.2.9 What is refused as decoration.** Spring physics on sheets, particle feedback, screen shake, sound on
every tap, streaks/achievements for staff (the CRM lane's "a tier is a rating of a participant" applies to
waiters as much as diners — `CLAUDE.md:102`).

### 3.3 Migration: menu (done), recipes (the hard part), customers (the careful part)

**3.3.1 The doorway shape is already right; keep it for the next two.** Pure parser in `dowiz_hub::import`,
preview by default, warnings by row number, deterministic ids, refuse-not-guess, keep what the file has no
column for, apply refused on an empty draft. `import.rs` is 540 lines with its tests; the two parsers below
go in `crates/dowiz-hub/src/import/recipes.rs` and `import/customers.rs` (the file-size ratchet, 300 lines
hard, `tools/gates/file-size.sh:19-25`).

**3.3.2 Supplies and recipes — `import::recipes::from_csv(text, unit_hint) -> RecipeDraft`.**

Two CSVs, because every source exports two things: an INGREDIENTS file (`name, unit, cost, [category,
kcal/100, …]`) → supplies; a RECIPES file (`dish, ingredient, qty, [unit, brutto/netto, prepack]`) → `bom`.
Poster's export and iiko's `getPrepared` both come out in that shape; r_keeper's StoreHouse export must be
checked (§7). Rules, each with its RED test:

| Rule | Why (the defect it prevents) | Test |
|---|---|---|
| A unit is `g`, `ml` or `unit` after scaling: `kg → g ×1000`, `l → ml ×1000`, `p/pcs/шт/copë → unit`; any other word is refused naming the row | `recipe.rs:25` closes the set; a silent "kg" read as "g" under-reserves by a thousand | `a_kilogram_becomes_a_thousand_grams`, `an_unknown_unit_is_refused` |
| A quantity that is not whole IN THE BASE UNIT is refused ("0.0425 kg" → 42.5 g → refused; "0.043 kg" → 43 g ok); a quantity over `QTY_MAX` refused | `Qty = i64` (`stock.rs:23-26`); rounding here is the "relative error that hides from relative consumers" (memory) | `a_fractional_gram_is_refused_not_rounded` |
| Gross (brutto / `amountIn`) is the quantity kept, because it is what the incumbents write off (iiko: "именно это поле участвует в расчете списаний"); net and yield are stored on the line as `net`, `yield` keys the hub ignores (`recipe.rs:13-14`: "every other key on a line is ours") so nothing is lost and the ledger's number is the one that matches the old system's stock counts | switching a venue to net would make dowiz's counts disagree with the counts the owner trusts, on day one | `gross_is_the_reserved_quantity`, `net_and_yield_survive_on_the_line` |
| A semi-finished product (Poster `structure_type = 2`, iiko a chart whose product is itself a chart's item) is FLATTENED to leaf supplies at import, scaled by its own `assembledAmount`/yield — exactly iiko's `getPrepared` formula — and the flattening is REPORTED per dish ("sauce X expanded into 4 lines") | `bom_of` is flat by design (§1.6); a prepack imported as a supply named "sauce X" reserves a thing nobody restocks | `a_prepack_is_flattened_and_reported`, `a_prepack_cycle_is_refused` |
| A recipe line naming a supply that is not in the supplies file is refused by row, and the dish is imported WITHOUT a `bom` (it sells, it does not reserve — `stock.rs:951-957`) rather than with a partial one | a partial BOM is a wrong reservation with a correct-looking ledger | `an_unknown_supply_leaves_the_dish_without_a_recipe` |
| Cost: "kopecks"/cents scaled to the venue's minor unit with `parse_price`'s refusal for a fraction of a lek; a source currency that is not the venue's is refused whole ("this file is in UAH; this venue trades in ALL") | the kit's fourth money copy; `money.js:10-16` "a second [currency] inside it would be a rounding error with a customer's name on it" | `a_foreign_currency_is_refused_not_converted` |
| Dated tech cards (iiko `dateFrom/dateTo`): the card valid TODAY is imported; the others are counted in a warning | the ledger has no dated recipes; importing history would import contradictions | `only_the_card_in_force_today_is_imported` |
| Nutrition per 100 accepted as floats (`kcalPer100` etc. are floats already, `supplies.rs:79-83`), never used in a reservation | — | existing shape |
| A second import is an update by deterministic id (`slug`, `import.rs:92-122`); a supply that vanished from the file is `active:false`, never deleted | stock history references it | `a_vanished_supply_is_retired_not_deleted` |

Handler: `POST /api/owner/recipes/import?apply=true` — same flags, same summary shape, writes supplies
then products' `bom` inside ONE `with_catalog` closure (one image; the gate stays 0).

**3.3.3 Customers — `import::customers::from_csv(text) -> CustomerDraft`, and the rule that shapes it.**

The CRM lane's sentence is the law here: *anything that is not about an order this person placed … is
marketing and requires a `given` consent for that purpose on that channel, at the moment of the send*
(§3.2 there). **A migrated customer has given dowiz nothing.** Whatever box they ticked in Poster was a
sentence they never saw from us, for a controller that was not us. So:

| May be imported | Into | May NOT be imported | Why |
|---|---|---|---|
| phone (normalised to E.164 — the CRM's §1.1 spelling defect is the first thing an import must not repeat), name | the `people` record's key (`customer_key`) and, for name, NOTHING — name lives in orders and there are no orders; it goes in `note` prefixed "imported name:" until their first order writes the real one | `bonus`, `ewallet`, `total_payed_sum`, `discount_per`, `client_groups` | a balance is money owed by the old system, not a fact in our ledger (the CRM refuses a persisted counter of anything, §4 there); a group is a tier ("a rating of a participant") |
| allergens (mapped to the EU-14 codes `allergens.rs` refuses unknowns of, `:12-14`) | `allergens` | free-text health notes | CRM §3.1 "not held" |
| `comment` → `note` (≤ 280 chars) | `note` | `client_sex`, birthday YEAR | never held (CRM §4) |
| birthday `MM-DD` | `birthday` | email | no email channel exists (CRM §4 "Email as a channel") |
| — | — | **any consent flag from the source** | consent is to a wording (`wording_id`) we can produce; theirs is not ours |

And the record carries `provenance: { system: "poster", imported_at_ms, file_hash }` — so the console can
show "imported, no consent" on the row, so an erasure request can name the batch, and so §5 G8 can prove that
no `c` entry in the `consent` log has an `evidence` that is an import.

**What may be SENT to a migrated customer:** exactly what may be sent to anyone without consent — a message
about an order they place with us, to the contact they give for it, while it is live (CRM §3.2). Nothing
else, until the checkout box on their first dowiz order writes a `given`. The ePrivacy "soft opt-in" for
existing customers is not confirmed for Albanian law (CRM §7) and the import does not rely on it.

**3.3.4 Adapters, in order.** (1) CSV/XLSX-as-CSV for all three systems (their consoles export it; the
owner does it once). (2) Poster's API, because it is the one a Worker can call with a pasted token:
`menu.getProducts` + `menu.getProduct` per dish + `menu.getIngredients` + `menu.getPrepacks`, mapped into the
SAME `RecipeDraft` so the parser's tests are the adapter's tests; `clients.getClients` into `CustomerDraft`.
(3) iiko and r_keeper by file only; their APIs are on a server in the restaurant that a Worker cannot reach
and should not be given credentials to.

---

## 4. Recommended AGAINST, each with its re-entry condition

| Item | Why not | Re-enter when |
|---|---|---|
| **WebUSB or Web Serial as the printer path** | absent on every iPad (§2.1); our own `usb=()` denies it; Web Serial on Android shipped this year | never as the primary; as a third encoder if a venue has an Android till and a USB-only printer AND step 2 is insufficient |
| **A local print agent / bridge** (QZ Tray-style, ours or theirs) | a second deployable on a machine we do not host; its failures are console lines unless it phones home; somebody must keep a PC on (§2.3) | a venue with a 9100-only printer and no line a poll can cross; then ship it as a pre-imaged Pi that speaks the SAME `/api/print/` protocol as a CloudPRNT printer, so the object never learns a fourth thing |
| **An ESC/POS encoder as the first deliverable** | `text/plain` prints on CloudPRNT; the ticket is text already; ESC/POS is bytes with a page-width, code-page and cut model per vendor | step 2 (BLE), and only the four commands named there; a fuller encoder when a receipt (the tax lane's `receipt_text`) must print on a non-fiscal printer |
| **LAN-direct printing from the page (ePOS/WebPRNT over LNA or a self-signed cert)** | a per-device certificate on Safari, a per-site prompt on Chrome, a private IP in a CSP, a DHCP lease that breaks it; none of it visible to us when it fails (§2.2) | a venue that insists on printing with the internet down from an iPad — then per-venue CSP from the Worker and the cert ritual, documented as a paid setup |
| **A card-reader integration** (Stripe Terminal, SumUp SDK) | needs an SDK or a native app; Albanian venues run standalone bank terminals; the fiscal seam belongs to ebills | the first venue on Stripe whose terminal the platform must reconcile; then Terminal's JS SDK on the console, never on the storefront |
| **A fiscal-register driver** | there is no register to drive; Albanian fiscalisation is software via CIS and the venue's own terminal prints (§2.6) | never; the ebills lane owns the seam |
| **Eight-direction or hierarchic marking menus** | accuracy falls past eight; dish choice does not fit; novices are slower (§2.7.1) | never for dishes; eight for the bill only if four is measured short |
| **Yes/No confirmation dialogues** | habituation (§2.7.3); the eight today are retired in §6 item 4 | never as Yes/No; the typed-amount form stays for refund and erasure |
| **Disabling a control while its request runs** | drops the early and the doubled tap (§2.7.4); the idempotency key already makes the duplicate safe | never |
| **A `training` flag on live orders** | eight conservation laws, two folds and a witness to exempt it from; the first omission is the day's takings | never; a training object (§3.2.8) |
| **Importing loyalty balances, groups/tiers, sex, birth year, email, or any source consent flag** | §3.3.3, and the CRM lane's refusals it inherits | balances: never (a balance is the old system's debt); consent: never (not our wording) |
| **Nested recipes (prepacks) in the hub model** | `bom_of` is flat by design; flattening at import loses nothing the ledger needs and keeps the fold one function | a venue that RE-EDITS a sauce used by forty dishes weekly and asks for it; then a `prepack` record and a flatten-at-save, still one `bom` per dish in the image |
| **Guessing a fractional gram, an unknown unit, a foreign currency, a "kopeck" scale** | the `parse_price` standard (`import.rs:14-17`) | never |
| **Reading iiko/r_keeper over their APIs from the Worker** | on-prem servers, LAN credentials, a licence per interface (§2.8) | a venue that hosts its server publicly AND asks — and even then a read-only employee |
| **MQTT (CloudPRNT Next)** | a Worker is not a broker; polling is the protocol the object already serves | a printer fleet whose polling bill (§2.4 c) exceeds a broker's |

---

## 5. The gates

Shape: `tools/gates/<name>.sh` (+ Python where braces must be matched, `one-venue.sh`'s shape), a
`.baseline` that may only fall (`one-venue.sh:56-70`: refuse on a rise, demand the baseline be lowered on a
fall), wired beside the ten in `ci.yml:40-86`, comments stripped before counting (the epitaph trap,
`ROADMAP-2026-09-22.md` §5), and **a proof that it fires both ways before it is trusted**
(`e2e/gates/conservation.prove.mjs:1-16`).

| Gate | Counts | Baseline | Red proof | Green proof |
|---|---|---|---|---|
| **G1 `print-ack` (conservation law 9, `e2e/gates/conservation.mjs`)** | for every `outbox` entry of kind `print` older than `MAX_TRIES`' span: it is `printed` (a `DELETE` was recorded) or `abandoned` with an `errlog` record naming the printer and the code; a `print` entry in no state is the finding | 0 | a stub with one `print` entry, no ack, no errlog row → red naming the order | the healthy stub → green; a venue with no printer key → "not measured", never red |
| **G2 `two-taps` (e2e, `e2e/gates/taps.mjs`)** | Playwright drives each row of §3.2.1's table on the training venue at 390 px and 768 px with `hasTouch`; counts `pointerdown` events to reach the server call; refuses a row above its tap count; measures wall time against the budget and REPORTS it (not refused — a CI box is not a tablet) | per row of the table | remove the category-rail preselection in a scratch copy → "add a dish" needs 3 → red | HEAD after §6 item 6 → green |
| **G3 `tap-size` (`mobile.mjs` promoted into CI)** | controls under 44×44 CSS px per surface, three phone profiles + two tablets, hit-tested while visible (`mobile.mjs:40-60`) | today's count on the console and courier app — **not measured** (§7); measure, then ratchet to 0 | `.act` at 40 px on HEAD is the red case (`admin.css:165`) | after §6 item 1 |
| **G4 `gesture-has-a-button.sh`** | every `flick:` / `hold:` binding in the waiter surface's action table names a `data-act` that also appears on a visible `<button>`; a gesture-only action is the finding | 0 | add `flick: 'archive'` with no button in a scratch copy → red | HEAD |
| **G5 `never-colour-alone.sh`** | every status pill (`.st`, `.pill`, `[data-st]`) in the three surfaces' render code carries a text node (`data-t-st`, `data-t`) or an `aria-label`; a pill whose only content is a dot or a background is the finding | 0 on the console today (`orders.js:84`); others to count | a scratch `<span class="st"><i class="dot"></i></span>` → red | HEAD |
| **G6 `confirm-count.sh`** | `confirm(` call sites across `workers/api/public` (the console's sheet and the native one) | **9** (measured: 8 + 1) | add one → red | falls with §6 item 4; the two that remain (refund, erasure) are named in an allow-list with the typed-amount form |
| **G7 `idempotent.sh`, widened** | today it pairs `tapped(` in `courier/app.js` with `"courier.<verb>"` guards and says what it cannot see (`:22-27`). Widen to a list of `(surface file, route prefix)` pairs — `(courier/app.js, courier.)`, `(waiter/app.js, waiter.)`, `(admin/orders.js, owner.)` — so the waiter surface is refused a queueable route without a guard on the day it is born | 0 missing | a `tapped(\`/waiter/tabs/${id}/void\`)` with no `"waiter.void"` guard → red | HEAD (the pair list has one entry today) |
| **G8 `no-consent-from-import`** | native: `import::customers` produces no `consent` entries and no `c` record with `evidence` mentioning an import; grep: `consent` never appears in `import/customers.rs` outside the doc comment that says why | 0 | insert a `consent: { given: true }` on the draft in a scratch copy → red | HEAD |
| **G9 `import-refuses` (native tests, `cargo test -p dowiz-hub import::`)** | the RED tests of §3.3.2 and §3.3.3, each named for its refusal; a mutant that makes `parse_qty` round instead of refuse must be killed (`cargo mutants` already runs on kernel files, `ci.yml:94-95`) | all green | the mutant | HEAD after §6 item 8 |
| **G10 `hardware-policy.sh`** | `Permissions-Policy` in `_headers` and every `navigator.usb/serial/bluetooth` reference in `workers/api/public` agree: a feature the policy denies must not be called, and a feature called must be named as allowed; `connect-src` must contain no private-IP literal unless a `print.lan` settings key exists in the tree | consistent today (`usb=()`, no callers) | add `navigator.usb.requestDevice` to a scratch copy → red | HEAD |

---

## 6. Order of work, each with its CHECK

Costs are hypotheses. Nothing here was timed. TDD throughout: the RED test first, named for the defect;
pure logic in `dowiz-hub` or a `services/*/` pure file, I/O in one port file per area (`services/mod.rs:3-9`).

1. **`.act` to the token; `mobile.mjs` into CI (G3).** `admin.css:165` `min-height: var(--tap)`; run the
   audit on the console and courier app, write the baseline, ratchet.
   **CHECK:** `node e2e/kit-regression/mobile.mjs` reports 0 controls under 44 on the console's live queue;
   `ci.yml` runs it; the red proof is HEAD~1.
2. **The `print` rail (§3.1 step 1), pure half first.** In `outbox.rs`: `kind = "print"`, `next_at_ms =
   now + WINDOW` at enqueue, `Verdict` from a printer result code — all pure, tested beside the 14
   `command/tests.rs` tests. Then `rails.rs`: `waiting_print(place)`, the three routes, the API-key auth,
   the `LATE +n min` first line, the `errlog` place name `print.abandoned`.
   **CHECK:** native tests green; `curl -u <key>: -X POST https://<venue>/api/print/poll -d '{"status":"…"}'`
   answers `jobReady:true` after a test placement on the training venue and `jobReady:false` after the
   `DELETE`; `/api/owner/health` shows the entry move queued → printed; G1 red/green proved.
3. **A real printer.** One Star mC-Print3 (or any CloudPRNT model) on the live venue's LAN, *Server URL* =
   `https://<venue>/api/print/poll`, *User Name* = the key, *Polling time* = 5 s.
   **CHECK:** a real order on the training venue prints within 10 s (stopwatch, three trials, the numbers in
   the commit); pulling the printer's Ethernet for 2 min and restoring prints the backlog with `LATE` lines;
   the week's request count per printer read from traces and written into §2.4 (c) as a measurement.
4. **Undo over confirm (§3.2.5), console first.** The outbox gains `commit_at` and `withdraw(seq)` (pure
   decision + IndexedDB), the pending bar, and `busy()` loses `disabled` on primaries. Retire the eight
   sheets in this order: dish/category/promo/key deletion → soft delete + undo; courier deactivate and
   supply retire → toggle + undo; venue close → visible toggle; reject/cancel keep the reason sheet; then G6's
   baseline falls 9 → 3 → (with refund/erasure allow-listed) 2.
   **CHECK:** `e2e/kit-regression/outbox.mjs` gains a case: a tap withdrawn inside the window sends 0
   requests; a tap after the window sends 1; a doubled tap sends 1 under 1 key. G6 falls in the same commit.
5. **Guide on the console (§3.2.8 a) and the training venue (§3.2.8 b).** `createGuide` on `admin/app.js`
   with ≤ 8 steps in three languages; `__training:<venue>` object created on "practise", seeded from the
   catalogue, deleted at close; the print rail stamps `TRAINING`.
   **CHECK:** Playwright completes the tour on a fresh `localStorage`; a training order never appears in
   `/api/owner/history` of the live venue (the conservation audit's counts are unchanged before/after).
6. **The waiter surface's interaction layer (§3.2.1-3.2.4, 3.2.6-3.2.7) — after A4-A8 land.** The action
   table (taps + gestures), the four-direction marking menu, the thumb layout, the predicted table fold over
   `replica.apply`, G2/G4/G5/G7 with their proofs.
   **CHECK:** G2 passes every row of §3.2.1 at 390 and 768 px; G7's pair list has the waiter entry and the
   gate is red when one guard is removed; G5 red on a scratch dot-only pill.
7. **BLE printing from the console (§3.1 step 2), Android only.** `lib/bleprint.js`, the four-command
   encoder, the `DELETE` ack from the browser, the sale-time sentence in the onboarding copy.
   **CHECK:** on one Android tablet, a ticket prints with the tablet's mobile data off and Wi-Fi off after
   the order was placed from a phone on mobile data — i.e. the ticket that arrived over the socket before the
   outage still prints; and the same job is NOT printed twice when the CloudPRNT printer also polls (the
   `DELETE` races and one wins — the object's turn serialises it).
8. **Recipes import (§3.3.2).** `import/recipes.rs` with the nine RED tests; the route; the console's
   preview with the flattening report.
   **CHECK:** `cd crates/dowiz-hub && cargo test import::recipes` green with every refusal test present
   (G9); a Poster export of the live venue's 165 dishes previews with a warning count that is read and
   explained in the commit message; the `one-image` gate stays 0.
9. **Customers import (§3.3.3) — after the CRM lane's §3.1-3.2 exist.** `import/customers.rs`, E.164
   normalisation, provenance, G8.
   **CHECK:** an imported row shows "imported, no consent" on the console; `consent::state` for every
   imported key is `None`; a campaign built after the import selects 0 of them (the CRM's G1 test extended
   by one fixture).
10. **Poster API adapter.** Token in settings (`integrations.*` keys), a pure mapper from the JSON shapes in
    §2.8 into the same drafts.
    **CHECK:** the mapper's tests use recorded JSON fixtures; the drafts are byte-identical to the CSV path's
    for the same account export (a `diff` in the test).

**Order rationale.** 1 is a measured defect against a token that already exists. 2-3 change what a kitchen
receives and are the operator's first question; they depend on nothing the other lanes are writing. 4 and 5
change what a hand does on the surface that exists, before the surface that does not. 6 and 7 wait for the
room. 8-10 are the migration in the order the incumbents make it possible.

---

## 7. What could not be determined, and what would settle it

| Claim or number | Status | What would settle it |
|---|---|---|
| The data model for tabs, splits, voids and shifts (A4-A8) | `BLUEPRINT-POS-THE-ROOM-2026-09-22.md` is not on disk; §3.2 designs the interaction over it unread | read it when it lands; rename the route stems in G2/G7 to its names |
| Whether the stack lane re-opens CRDTs for "an offline POS till" (`ROADMAP-2026-09-22.md` §4) | its blueprint is not on disk | none of §3 needs a CRDT: the printer path polls the object, and the terminal's queue is the outbox's sequence + idempotency + a refusing FSM, the argument `outbox.js:12-19` already makes |
| The number of controls under 44 px on the console and courier app today | not measured (the audit points at the live venue and needs a Chromium; not run for a document) | `node e2e/kit-regression/mobile.mjs` on the box, then the baseline of G3 |
| Star's default polling time and the Epson SDP interval bounds | Star's manual pages fetched do not state a default; Epson's overview says "periodically, such as every 60 seconds"; the SDP manual PDF was downloaded (1.4 MB) but no PDF text tool exists on this box | `apt-get install poppler-utils` and `pdftotext` the saved file; or read the printer's web page once one is on the bench |
| The request cost of one polling printer per month | hypothesis 259,200 requests at 10 s (§2.4 c) | one printer for a week on the live venue; count by `x-trace-id` |
| Whether Chrome for Android applies Local Network Access the same way as desktop | the Chrome blog does not say | test on one Android tablet with a printer on the LAN before §3.1 step 4 is ever offered |
| Web Bluetooth reconnection after a reload without a new gesture (`getDevices()`) | not verified | try on the tablet in §6 item 7 |
| `WINDOW` = 5 s | hypothesis | measure withdrawals vs. complaints on the training venue in week one |
| What StoreHouse 5 actually exports for a calculation card, column by column | not fetched (UCS pages list Excel/XML export, not the columns) | one export from a real r_keeper venue; until then r_keeper is "menu by XML/CSV, recipes by hand" |
| Poster's number formats in the CSV EXPORT (vs. the API's "kopecks" and string decimals) | the help-centre pages describe the import template, not the export's columns | one export from a Poster account; the `parse_price`-style refusal covers the unknown either way |
| Whether Poster's `phone_number` is E.164 without `+` or with a national prefix | "digital format" is all the doc says | one export; the normaliser refuses a number it cannot place in a country |
| Albanian law on ePrivacy-style soft opt-in for existing customers | not confirmed (CRM §7) | counsel; §3.3.3 does not rely on it |
| `RESILIENCE` defect #20 (a half-open socket suppresses polling for 90 s) — still open? | not re-verified | `live.js`'s quiet-period constant and a probe that half-opens the socket |
| The `hubdo.rs` route strings the object answers (`/fold/…`) | grep for literal strings found none; command executions cited by their `decide` calls (`:650, :747`) | read the object's dispatch once before adding the `outbox` read path for `/api/print/poll` |

---

## 8. Sources outside the tree (all fetched 2026-09-22 unless marked)

- StatCounter, *Tablet Operating System Market Share Albania*, August 2026: iOS 50.04 %, Android 49.95 % — https://gs.statcounter.com/os-market-share/tablet/albania
- caniuse, *Web Serial API* (August 2026 data): Chrome 89+, Chrome for Android 152, Safari and Safari iOS not supported, Firefox 151+, Samsung Internet not supported, 76.21 % global — https://caniuse.com/web-serial
- MDN, *Web Serial API* (Android support in the Chrome 148 beta, April 2026; WebKit position "opposed") — https://developer.mozilla.org/en-US/docs/Web/API/Web_Serial_API
- MDN, *WebUSB API* — https://developer.mozilla.org/en-US/docs/Web/API/WebUSB_API
- Chrome for Developers, *New permission prompt for Local Network Access* (Chrome 142; secure context; mixed-content relaxation for private-IP literals, `.local`, `targetAddressSpace:"local"`; Chrome 145 split into `local-network` / `loopback-network`) — https://developer.chrome.com/blog/local-network-access
- Bluefy – Web BLE Browser (App Store; the third-party route on iOS) — https://apps.apple.com/us/app/bluefy-web-ble-browser/id1492822055
- Star Micronics, *CloudPRNT Protocol Guide* and *Polling the server (POST)* reference (status/`jobReady`/`mediaTypes`/`jobToken`/`deleteMethod`/`claimQueue`; media types incl. `text/plain`, `text/vnd.star.markup`) — https://star-m.jp/products/s_print/sdk/StarCloudPRNT/manual/en/protocol-guide.html and …/protocol-reference/http-method-reference/server-polling-post/index.html
- Star Micronics, *mC-Print3 online manual — CloudPRNT settings* (Server URL, Polling time, HTTP response timeout, User Name/Password, HTTPS settings) — https://www.star-m.jp/products/s_print/mcprint3/manual/en/settings/settingsCloudPRNT.htm
- Star Micronics, *WebPRNT vs. CloudPRNT* — https://starmicronics.com/blog/webprnt-cloudprnt-comparison/
- Epson, *Server Direct Print — TM-Intelligent* (overview; the manual PDF `server_direct_print_um_en_revk.pdf` downloaded, not readable here) — https://download4.epson.biz/sec_pubs/pos/reference_en/technology/server_direct_print.html
- Epson, *ePOS-Print XML User's Manual* — https://files.support.epson.com/pdf/pos/bulk/epos-print_xml_um_en_revk.pdf
- Odoo 19 documentation, *Self-signed certificate for ePOS printers* ("Since the Chromium 142 update, using a self-signed certificate is no longer required"; "Generating a self-signed certificate should only be done once") — https://www.odoo.com/documentation/19.0/applications/sales/point_of_sale/hardware_network/epos_ssc.html
- NielsLeenheer/WebBluetoothReceiptPrinter; defuj/printer-js (BLE ESC/POS from the browser) — https://github.com/NielsLeenheer/WebBluetoothReceiptPrinter , https://github.com/defuj/printer-js
- Kurtenbach & Buxton, *User Learning and Performance with Marking Menus*, CHI '94 (3.5× faster with marks; experts go blind) — https://www.billbuxton.com/MMUserLearn.html ; Kurtenbach, *The Design and Evaluation of Marking Menus* (thesis) — https://www.research.autodesk.com/app/uploads/2023/03/the-design-and-evaluation.pdf_recHpUp1v9dc1n2CJ.pdf ; Big Medium, *Touch Means a New Chance for Radial Menus* — https://bigmedium.com/ideas/radial-menus-for-touch-ui.html
- Hoober, *How Do Users Really Hold Mobile Devices?*, UXmatters 2013 (1,333 observations; 49 % one-handed; 75 % thumb) — https://www.uxmatters.com/mt/archives/2013/02/how-do-users-really-hold-mobile-devices.php
- Apple HIG 44 pt, Material 3 48 dp, WCAG 2.2 SC 2.5.8 — as cited in `e2e/kit-regression/mobile.mjs:18-23` (not re-fetched)
- Nielsen Norman Group, *Confirmation Dialogs Can Prevent User Errors — If Not Overused* — https://www.nngroup.com/articles/confirmation-dialog/ ; A List Apart, *Never Use a Warning When You Mean Undo* — https://alistapart.com/article/neveruseawarning/
- SuperCombo Wiki, *Street Fighter 6 / Game Data* (4-frame buffer, 7 for dashes/reversals; 11-frame quarter-circle window) — https://wiki.supercombo.gg/w/Street_Fighter_6/Game_Data ; EventHubs on SF6 input leniency (the over-lenient-buffer complaint) — https://www.eventhubs.com/news/2023/jun/17/sf6-input-trouble-breakdown/
- Poster developer docs mirror, `joinposter/docs` — `en/web/menu/getProduct.md`, `getIngredients.md`, `getPrepack(s).md`, `en/web/clients/getClients.md` — https://github.com/joinposter/docs ; `joinposter/api-php` README (access token from the console; OAuth for apps) — https://github.com/joinposter/api-php ; Poster help centre, *How to import dishes* / *How to import ingredients* (export to CSV/XLS/XLSX from the console) — https://help.joinposter.com/en/articles/5359195-how-to-import-dishes , https://knowledge-base.joinposter.com/en/how-to-import-ingredients
- iiko, *Технологические карты* (iikoServer API: `assemblyCharts/getAll`, `getAllUpdate`, `getPrepared`; `AssemblyChartDto` fields; the flatten formula) — https://ru.iiko.help/articles/api-documentations/~tekhnologicheskie-karty (read via the search index; a direct connection was refused) ; iikoCloud `nomenclature` (revision delta; no recipes) — https://github.com/salesduck/iiko-cloud-api
- r_keeper 7 XML interface (`https://{IP}:{HTTPDataPort}/rk7api/v0/xmlinterface.xml`, Basic auth, UTF-8) — https://docs.restera.com/translate/r_keeper-7-xml-interface-105349121.html ; *Getting Up-to-date Menu* (`GetRefData RefName="MENUITEMS" onlyActive="1"`, `GetOrderMenu`) — https://docs.restera.com/translate/getting-up-to-date-menu-106398127.html ; UCS StoreHouse 5 (calculation cards; export to Excel/Word/XML) — https://ucs.ru/products/r_keeper/storehouse/
- Sibling documents in this tree, read in full for what they decided: `BLUEPRINT-ARCHITECTURE-EVOLUTION-2026-09-22.md` (house style), `BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22.md` (§1.1, §3.1, §3.2, §4, §5, §7), `BLUEPRINT-TAX-PRICE-CHANNEL-2026-09-22.md` (§0, §2.7, §3.8, §4, §5), `BLUEPRINT-EBILLS-INTEGRATION-2026-09-22.md` (§0), `BLUEPRINT-RESILIENCE-AND-EVOLUTION-2026-09-21.md` (§7), `ROADMAP-2026-09-22.md`.
