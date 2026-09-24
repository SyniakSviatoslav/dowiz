# Audit — bugs, blind spots and the tests that would have caught them (2026-09-24)

**Scope.** A read-only audit of `/root/dowiz` at HEAD `5abeb0ac`. It was re-checked at `1dbb181a`, which changed
docs only (`git diff --stat 5abeb0ac 1dbb181a` touches 4 files under `docs/design/`). Nothing in the tree was edited,
committed or deployed, and no one logged in to production.

**Method.**
- Traced real paths across roles: guest → storefront → object → console / room / courier.
- Four lanes: room and payments; booking, customers and campaigns; stock, courier, eBills, outbox and auth; and
  the cross-cutting gates run in this lane itself.
- Proofs come from `node` against the shipped modules, from a scratch cargo crate that depends on
  `crates/dowiz-hub` (run via `slot.sh audit`, outside the repo), and from greps against `HEAD`.
- The scratch probes live at `/tmp/claude-0/-root/2bd4b866-4cdd-4908-9891-de3ce4677585/scratchpad/`:
  - `probe/tests/probe.rs`
  - `dst.mjs`
  - `due.mjs`
  - `routes2.py` (the route contract)
  - `sw.py` (the service-worker shell)
  - `i18n.mjs` (i18n parity)

**Labels.**
- **PROVED**: a command was run and its output is quoted.
- **CODE** (code-read): the path is traced line by line, but the scenario itself was not run.
- **INFERRED**: the trigger depends on something outside the tree, such as Meta's behaviour or cron timing.

File paths are relative to `workers/api/` unless they start with `crates/`.

**Where the code is already sound, so no one re-audits it:**
- `SEND_ENABLED=false` holds. It is defined once (`src/fiscal/mod.rs:40`) and checked at the only two entry points,
  `fiscal/rail.rs:59` and `fiscal/routes.rs:44`.
- There are no CSP-dropped inline styles, scripts or `on*=` handlers anywhere in `public/`: the greps return 0 hits.
- i18n key parity is complete. The four apps have 0 keys missing across sq/en/uk once each module's `WORDS` are
  merged (`i18n.mjs`).
- Double-booking one table is guarded inside the object's turn with a ±90-minute window.
- A double refund, a double guest confirm and a double stamp redeem are each refused by the FSM or by the object turn.
- All 39 `Place::of_any` sites are paired with a venue-scoped principal check.
- The QR signature binds the venue and compares in constant time.
- The tips day uses the venue's time zone.

---

## 1. Defects

Severity: **H** means money, privacy or safety, or a whole role blocked. **M** means a wrong number or a stuck
flow that has a workaround. **L** means cosmetic or needs a hostile insider. The table puts PROVED highs first.

### 1a. Proved

| # | Sev | Defect | Concrete failing scenario | Where | Proof |
|---|---|---|---|---|---|
| D1 | **H** | **A refused room write keeps its idempotency key claimed for 24 h.** Retrying the same key answers 409 "still running", and the outbox spins on it, blocking every tap behind it. | A waiter's queued pay first gets a 5xx and is retried with the same key. The handler then refuses it (for example "payment sum exceeds the total"). From then on every retry gets 409 + Retry-After for 24 h. `lib/outbox.js:274` treats that 409 as "still running" and does not spend a try, so the queue behind it never drains. In the console, `admin/refund.js:35` mints one key per sheet, so after a failed first "Refund" every re-tap in that sheet is refused. | Handlers `return` on `Err` without `idem.done`: `src/services/orders/room/pay.rs:79-80`, `room/handlers.rs:145`, `room/transfer.rs:62,106`, `room/till.rs:139`, `room/guest_round.rs:108`, `services/orders/refund.rs:64,110`. The 409 comes from `src/idempotency/mod.rs:153-163`. | PROVED: `git grep -n "done(&place, code" HEAD -- workers/api/src` → only `booking/create.rs:223` and `:229` record a refusal. `pay.rs:82` calls `idem.done(&place, 200, …)` only on success. |
| D2 | **H** | **The courier app cannot open offline.** Its shell omits `/lib/vocab.js`, which `/lib/money.js` imports statically. This is the same class of bug found in the storefront today. | A courier kills the app underground and reopens it. `app.js` fails at link time, so the page is blank: no address and no queued "delivered" tap. The fetch handler caches only `SHELL` keys, so `vocab.js` is never cached even online. | `public/courier/sw.js:24-37`; `public/lib/money.js:18` `import { CURRENCIES, DECIMALS } from './vocab.js'` | PROVED: `python3 sw.py` → `== /courier/sw.js entry /courier/app.js graph 8` / `in graph, NOT in shell: ['/lib/vocab.js']`. The other three shells (`/sw.js`, `/room/sw.js`, `/kit/sw.js`) came out complete. |
| D3 | **H** | **Across the DST change, a booking made before 25 Oct 2026 for a date after it lands one hour early.** The offset in force *now* is applied to a later day. | On 24 Oct a guest books Mon 26 Oct at 19:00. The slot is stored as 17:00Z, which is 18:00 in Tirane. The guest's own list (`booking-mine.js:40`, which formats with the slot's own offset) shows 18:00. The owner console does the same for phone bookings. `dayRange = from + 1440` also drops one hour of 25 Oct, which is 25 hours long. | `public/store/booking.js:77,82-83` (`tzOffset = offsetMinutes(tz, now())` → `slotAt(now(), tzOffset(), day, minute)`); `public/admin/bookings.js:53-56,125` | PROVED: `node dst.mjs` → `offset used 120 -> slot renders in Tirane as 26/10/2026, 18:00` / `offset on the day 60` |
| D4 | **H** | **A paid PENDING round can then be rejected, and its money disappears.** It cannot be refunded. | A guest's QR round (PENDING) is paid at the table, and then staff reject it. `sitting::paid` and `bill` drop REJECTED rounds (`took_money=false`). `refund` refuses PENDING (`command/refund.rs:243`), and REJECTED→REFUNDING is not an FSM edge. The cash is in the drawer (see D12) with no record that it is owed back. | `crates/dowiz-hub/src/room/pay.rs:97` refuses only CANCELLED/REJECTED/REFUNDING/COMPENSATED_REFUND; `src/services/orders/room/guest_round.rs:23`, `src/command/advance.rs:79-100` reject without reading `payments[]` | PROVED (pay half): probe → `PROBE p1 pay on PENDING -> Ok(String("paid"))`. The reject half is CODE. |
| D5 | **H** | **A menu re-import puts stopped dishes back on sale and blanks every description.** | The owner uploads a two-column `name,price` sheet to change prices. Every dish stopped for "keg empty" goes back on sale, `unavailableNote` is nulled, and every storefront description becomes `""`. | `crates/dowiz-hub/src/import.rs:312-314` (missing Available → `true`, missing Description → `""`); `src/services/catalogue/import.rs:136-144` overwrites both unconditionally | PROVED: scratch `from_csv("name,price\nSake Nigiri,900")` → `import id="menu-sake-nigiri" available=true description="" category="menu"`. The test fixture (`import/tests.rs:9,58-68`) always has an Available column. |
| D6 | M | **The console and the importer give one dish two ids.** A re-import therefore duplicates hand-made dishes, and `retire=1` retires the originals, which carry the recipe and photo. | The owner adds "Sake Nigiri" by hand (id `sake-nigiri`), then imports a menu (id `sushi-sake-nigiri`). The storefront shows two dishes. With retire, the one with the recipe, photo and allergens goes off sale. | `src/catalog_edit.rs:36-47` vs `crates/dowiz-hub/src/import.rs:299-305` | PROVED: same probe → `console id="sake-nigiri"` / `with category id="sushi-sake-nigiri"` |
| D7 | M | **The same partial payment can be taken twice.** Pay carries no `base_seq` and no dedupe. | Two waiters split a bill and both take the same 700, or one waiter re-taps while offline (a fresh key per tap). Both are accepted until the total is reached. | `crates/dowiz-hub/src/room/pay.rs` (no seq check); `public/room/net.js` mints a new key per tap; `lib/outbox.js:203` stores `tag` but never uses it | PROVED: probe → `PROBE p3 pay 700 at 5000 -> ok=true` / `pay 700 at 5001 -> ok=true` / `payments=2 paid=1400` |
| D8 | M | **An amend down to exactly the amount paid leaves the round "unpaid" for ever.** | A 1200 round has 1000 paid, and a line is voided so the total becomes 1000. `payment_status` stays null. The pay screen shows 0 owed, and any further payment is refused. The round stays amendable and escapes `transfer::sitting`'s paid check. | `crates/dowiz-hub/src/room/rules.rs:137` refuses only `paid > total`; only `pay.rs:224` writes `"paid"` | PROVED: probe → `PROBE p2 after amend: total=1000 paid=1000 payment_status=null` / `next pay of 1 -> Err(Conflict("payment sum 1001 exceeds the total 1000"))` |
| D9 | M | **An unconfirmed guest QR round is counted in the table's bill and offered for payment.** The table code never expires. | A photo of table 4's QR, taken last week, adds a 4200 round to tonight's party at table 4. The waiter's "due" jumps from 1500 to 5700 before anyone has confirmed the round. | `public/room/logic.js` (sittingDue counts PENDING); `src/services/orders/room/table_link.rs:31` (HMAC over `loc\|zone\|n`, no time or sitting) | PROVED: `node due.mjs` → `sittingDue = 5700` / `pay offered on unconfirmed guest round = true` |
| D10 | M | **A scheduled ("later") delivery uses the phone's time zone, not the venue's.** The booking flow was fixed for this (`booking-time.js:13-16`); checkout was not. | A diner whose phone is set to Kyiv (a `uk` storefront user) picks 19:00, and the kitchen gets 18:00. | `public/store/checkout.js:280-283,333-338` (`new Date(f.value)` parses in local time) | PROVED: `TZ=Europe/Kyiv node -e "new Date('2026-09-24T19:00')…"` → `Europe/Kyiv phone picks 19:00 -> 18:00 in Durres` (with `TZ=Europe/Tirane` → `19:00`) |
| D11 | M | **The storefront's "open now / open until" uses a fixed +120 from 25 Oct.** It reads `state.loc.tzOffsetMinutes`, which the hub never sends. | From 25 Oct, 22:30 local reads as 23:30. The venue shows "closed" an hour early and "open" an hour late in the morning. | `public/store/state.js:340-344`, used by `public/store/venue.js:30,157` | PROVED: `git grep -n tzOffsetMinutes HEAD -- workers/api/src crates/dowiz-hub/src` → no writer. The only reader is `hubstore.rs:911` (a record override, `tz_offset_minutes`). `store/booking.js:74` says so in a comment and was fixed. `state.js` was not. |

### 1b. Code-read (traced line by line, not run)

| # | Sev | Defect | Concrete failing scenario | Where |
|---|---|---|---|---|
| D12 | **H** | **A refund returns nothing to a wallet and never reaches the drawer.** The till's `cash_payments` has no status filter. | A wallet-paid round is refunded, and the SPEND stays in the ledger. A cash round that was refunded, rejected or cancelled stays in `expected`, so the drawer closes "short" by exactly the money handed back. A round paid in EUR cash on an ALL order gives no per-pile refund figure. | `src/command/refund.rs` (`git grep -n wallet HEAD -- workers/api/src/command/refund.rs` → 0 hits); `src/command/till/fold.rs:126` |
| D13 | **H** | **A waiter can debit any customer's wallet by typing its id.** | `PayBody.wallet` goes straight to `wallet::id64(user)`, and the only check is the balance. The read side (`wallet_who`, `wallet.rs` ~407) refuses the same waiter, so they can spend a balance they are not allowed to see. | `src/services/orders/room/pay.rs:31`, `src/command/pay/wallet.rs:50` |
| D14 | **H** | **A door pass still admits after the booking is cancelled, declined or marked no-show, and one pass admits any number of people.** | A guest takes a pass while CONFIRMED and then cancels, and the scanner still answers `ok:true`. `verify_pass` never loads the bookings image and never compares `nonce`. The route is unauthenticated, and on a venue's first call it writes `pass_key` to settings. | `src/booking/pass.rs:148-183` (verified at HEAD: only `pass::decode` + `pass::verify(key, …, window, now)`) |
| D15 | **H** | **"Forget this customer" misses alias spellings, bookings and queued messages.** | Forget K. Orders under the linked spelling K2 (`069…` ↔ `+355…`) keep the name and phone. K2's consent is not withdrawn. The person reappears under K through the alias. The bookings image keeps `contact_name` and `contact_phone`. Queued campaign messages keep `to`, and the drain re-checks only the witness key. | `src/services/customers/forget.rs:52-75`, `src/hubdo/forget.rs:66-90`, `src/services/campaigns/send.rs:74-77,167-173` |
| D16 | **H** | **An order the owner assigns can never be accepted.** The courier is stuck: "Take" fails, "End shift" is refused, and the owner cannot reassign. | `assign` writes the assignment row. `accept` refuses when any row exists, the caller's own included ("another courier took this order", verified at `courier.rs:441-457`). `tasks` still sends `offerEndsMs`, so the app shows the offer. The shift cannot end (`courier.rs:241`), and no route removes an assignment. | `src/command/assign.rs:106-111`, `src/courier.rs:157-165,241,442-457`, `public/courier/app.js:659` |
| D17 | **H** | **A dish goes on sale with its allergens undeclared, bypassing the publish gate.** | "Add dish" in the console sends no `available`. `create_product` defaults it to `true` (verified at `catalog_edit.rs:105`) with no allergens, and the gate lives only in `update_product` (`owner.rs:926-938`). The menu import is the same (`services/catalogue/import.rs:136-141`). | as stated |
| D18 | M-H | **Any principal of a venue can read any customer thread, and any customer token can post into any thread.** Since `2cfab80b`, anyone can mint a customer token by booking with no account. | Book a table anonymously to get a `Claims::Customer` valid for up to about 61 days. With order X's id (which couriers and staff all see), read or post as CUSTOMER in X's thread. A courier reads every conversation. | `src/social.rs:118-126` (only `principal_at`), `:205-217,236-246` (no `order_id == thread_id`); `src/booking/create.rs:54-72`; `src/auth.rs:585-586`. By contrast, `src/live.rs:91-106` binds correctly. |
| D19 | M-H | **Editing a recipe never updates the dish's published ingredients, kcal or weight.** A bulk recipe import does the opposite and overwrites values the owner typed by hand. | The owner adds shrimp to a roll's recipe and the storefront ingredients still omit shrimp. The dish-sheet save sends the stale prefilled values back, and they are marked as typed. Separately, `bulk.rs:247` calls `set_bom(…, Typed::default())` and ignores `nutritionDerived:false`. | `public/admin/menu.js:143-152,191-204,309-314`; `src/owner.rs:909-913`; `src/recipe/apply.rs:47-65`; `src/services/catalogue/import/bulk.rs:247` |
| D20 | M-H | **Outbox drains are unbounded and write verdicts only at the end.** A drain longer than 60 s is re-read by the next cron, which re-sends everything. A campaign also queues ahead of kitchen bells. | A 400-message campaign: duplicate paid WhatsApp sends, duplicate kitchen tickets, and kitchen bells that wait behind marketing. | `src/outbox.rs:138-140`, `src/outbox/rails.rs:55,113` (INFERRED timing) |
| D21 | M-H | **WhatsApp bells sent outside the 24 h window are marked sent but never arrive.** | Meta answers 200 and reports 131047 later in a `statuses` webhook. Nothing reads `statuses`. | `src/outbox/rails.rs:71`, `src/channels.rs:~117,247-330` (INFERRED from Meta's documentation) |
| D22 | M | **Outbox entries for a channel that is not configured never send and never expire.** Once the 1 MiB image is full, new bells and printer tickets are dropped with only a `console_error!`. | A Telegram chat id set without a token, or WhatsApp settings removed after a campaign was queued. | `src/outbox/rails.rs:68-75`, `src/hubdo.rs:710` |
| D23 | M | **A dine-in order with a typed table and no QR code is accepted.** A9's signature is only enforced when a code is sent. | `POST /api/public/locations/:slug/orders {fulfilment:{kind:"dine_in",table:"salla:4"}}` with no `table_link` and no staff token passes `Needs::Table` (`storefront.rs:801-806`). `placer::guest` returns `Ok(None)` (`placer.rs:109`), and the order is placed as an anonymous cash dine-in on any table, outside any sitting. | `src/storefront.rs:801-806,1138-1142`; `src/services/orders/room/placer.rs:109,134-136` |
| D24 | M | **Anyone who types a regular's phone number spends that regular's stamp card**, and then sees the count. | `storefront.rs:1247-1250` passes the unverified `contact.phone` to `loyalty::at_placement`, which spends the whole alias circle's card. `GET /api/order/:id/stamps` then shows the balance. | `src/storefront.rs:1247-1250`, `src/services/loyalty/handlers.rs:67-110` |
| D25 | M | **A stamp is earned for a sitting even when one's own round in it was rejected.** | A guest's QR round joins a stranger's sitting and staff reject it. When the stranger pays, the guest still gets the stamp (`e.0 \|= me` whatever the status). | `src/services/loyalty/stamps.rs` (`count`) |
| D26 | M | **Consent is read under one key, withdrawn under one key and sent under any linked key.** | Consent is filed under `069…` (K2), which is linked to K. The console shows K as "not consented", but the campaign sends. The owner withdraws on K and the campaign still sends on K2's witness. There is also no console UI for `/consent` or `/forget` (see §2, B1). | `src/services/customers/handlers.rs:112`, `src/services/campaigns/audience.rs:81-83`, `src/services/customers/consent_routes.rs:43-46` |
| D27 | M | **Saving the floor plan ignores live bookings.** | The owner renumbers table 4 to 7, removes a zone or lowers seats. Live holds keep the old ids, a guest can then book the "new" table 4 at the same slot, and the room does not draw the orphaned hold. | `src/booking/floor.rs:127-152` |
| D28 | M | **Retrying a booking after a lost response makes a second booking.** The first booking's token is lost, and the guest has no way to find it. | `store/booking.js:337-341` mints a fresh `requestId` on every tap. The first booking keeps its table and one of the phone's three slots. | `public/store/booking.js:337-341` |
| D29 | M | **The server never checks opening hours or the slot grid on a booking, and never applies the free-cancel window.** | A direct POST books 03:00 on a closed day. `cancellation_is_free` has no caller outside `crates/dowiz-core/src/reservation.rs`. | `crates/dowiz-core/src/reservation.rs:359-383` |
| D30 | M | **A paid round that is still READY keeps its sitting "open" while the floor shows the table Free.** The next QR guest joins the previous party's check and can read their rounds. | The table is marked cleared, so the floor shows Free. `sitting::open` is still true (READY is not terminal), and a waiter cannot move the round to "collected" because that needs `Cap::Advance`. `live_sitting` then hands the next guest the old sitting, and `/api/order/:id/sitting` shows the old party's rounds. | `src/command/floor.rs:129`, `src/command/sitting.rs:73`, `src/owner.rs:570` |
| D31 | M | **The till closes against a stale count.** | The Z report uses the last X-count but computes `expected` up to now, so a sale after the count shows as a false shortfall. | `src/services/orders/room/till.rs:214-218` |
| D32 | M | **Moving a sitting does not check that the target table is free, or that it exists on the plan.** | Move A onto B's table: the floor keeps only the newest sitting there, so A vanishes and cannot be cleared. A typo in the table name sends the rounds to `unplaced`. | `src/command/transfer/sitting.rs:59` |
| D33 | M | **A CONFIRMED order that goes straight to IN_DELIVERY keeps its stock reservation for ever.** | Stock is settled only at PREPARING (consume) or REJECTED/CANCELLED (release). The courier's `write_status` does not settle stock. | `src/command/advance.rs:59-65`, `src/courier.rs:334-360,425,540` |
| D34 | M | **A room round with a tip or a comp cannot be fiscalised.** | A tip raises `total`, and `ebills_body.rs:198` refuses because `gross != doc.total`. Today's tip-at-pay feature therefore makes every tipped round unfiscalisable. The refusal is loud, but nothing can recover the round. | `src/fiscal/ebills_body.rs:198` |
| D35 | M | **The console supply form can change a supply's unit.** The CSV importer refuses exactly this change. | A supply goes from `g` to `unit`, and every stock count and recipe snapshot is read in the wrong unit. | `src/services/operations/supplies.rs:86-110` vs `crates/dowiz-hub/src/import/recipes/supplies.rs:95-101` |
| D36 | M | **A courier's cash entry records EUR or USD amounts 100 times too small.** | The field is prefilled with minor units (1250), and a courier types `12.50`. `parseInt` gives 12, and the server records a shortfall of 1238. Lek is unaffected. | `public/courier/app.js:1118-1124`, `src/courier.rs:608-614` |
| D37 | M | **A fiscal retry more than a day later, or at a till with more than 500 sales in two days, can register a second invoice.** This is latent while sending is off. | Reconcile-before-retry searches only one page of 500 sales covering yesterday and today. | `src/fiscal/ebills_fire.rs:152`, `src/ebills/state.rs:210-211`, `src/fiscal/ebills_sender.rs:121` |
| D38 | L-M | **The same phone number has three different keys in three modules.** | Bookings use `canonical_digits`. The customer list uses `customer_key`, which does not fold `069` into `+355`. The wallet uses raw `customer_key(phone)`. One person therefore has one booking identity, two wallets and two customer rows. | `src/booking/guest.rs:92-99`, `src/services/customers/handlers.rs:30-37`, `src/wallet.rs:377-380` |
| D39 | L-M | **Deactivating a courier at one venue logs them out of every venue.** A venue that tries to invite a courier who works elsewhere gets a 409, which reveals that the courier does. | | `src/services/courier/hiring.rs:79-81,224-259` |
| D40 | L | Minor items: | | |
| | | - Any staff role, whatever its capabilities, can confirm or cancel bookings and read guests' phones. | | `booking/guest.rs:55` |
| | | - The client chooses `userId` on a booking, and it is stored. | | `booking/create.rs` |
| | | - The till's currencies are hard-coded as `['ALL','EUR']`. | | `room/logic.js:76` |
| | | - Clearing a recipe leaves its derived nutrition published. | | `recipe/apply.rs:47-52` |
| | | - The bootstrap seed rewrites whole dish records. | | `bootstrap.rs:179` |
| | | - The console writes back display values that depend on the viewer's language. | | `admin/app.js:199-211` |
| | | - Courier earnings are counted by the order's creation time, not its delivery time. | | `courier.rs:767,773` |
| | | - Fiscal codes are lost when the order is missing from the view. | | `hubdo/fiscal/send.rs:131,146` |

---

## 2. Blind spots, and the grep that shows each has no coverage

| # | Behaviour nothing exercises | Evidence (run at HEAD) | Would have caught |
|---|---|---|---|
| B1 | **Routes with no UI caller.** Features that exist on the server, but no one can reach them from the console or the storefront. | `python3 routes2.py` "ROUTES WITH NO CLIENT LITERAL" includes:<br>- `POST /api/owner/customers/:key/forget`<br>- `POST /api/owner/customers/:key/consent`<br>- `POST /api/owner/customers/rekey`<br>- `GET /api/owner/stock/waste` (A11's report)<br>- `POST /api/owner/restore`<br>- `GET /api/owner/history`<br>- `POST /api/owner/hub/rotate`<br>- `POST /api/owner/logo/clear`<br>- `POST /api/owner/i18n`<br>- `POST /api/owner/branding/preset`<br>- `POST /api/owner/zones`<br>- `GET /api/owner/graph`<br>- the threads, wallet and pass routes (reached from `/kit` only)<br><br>Confirmed by `git grep -n -F "/forget" HEAD -- workers/api/public` → 0 callers. | D15, D26 (GDPR erasure has no button) |
| B2 | **Service-worker shells vs the static module graph.** | There is no gate. `sw.py` found D2 in one run. | D2, and today's storefront `booking.js` miss |
| B3 | **A refused write through the idempotency layer, then a retry.** | `git grep -n "done(&place, code" HEAD` → booking only. `ls public/lib \| grep outbox.*test` → nothing (`lib/outbox.js` has no test). | D1 |
| B4 | **Venue-day arithmetic across a DST change**, composed through the screen code rather than the helper alone. | `booking-time.test.mjs:83-85` asserts `offsetMinutes` on the day, but `:35-38` tests `slotOf` only with a fixed `OFF`. There is no test of `store/booking.js`, `admin/bookings.js` or `checkout.js scheduledAt`. | D3, D10, D11 |
| B5 | **Pay on PENDING, then reject; refund of a wallet or cash payment against the ledger and the till.** | `git grep -n PENDING HEAD -- crates/dowiz-hub/src/room/` in the pay tests → 0. `refund/tests.rs` mentions `wallet` once, as `wallet: None`. | D4, D12 |
| B6 | **Import without the optional columns; console ids vs importer ids.** | `services/catalogue/import/tests.rs:9` fixture always has `Available`, and line 67 asserts `available: true` after a re-import. | D5, D6 |
| B7 | **Principal ↔ object binding** (the customer token's `order_id` equals the thread id; the courier owns the assignment; staff capabilities on booking moves). | `social.rs` has no `tests.rs`. `git grep -c "#\[test\]" HEAD -- workers/api/src/social.rs` → 0. `principal_at` is tested only for booking/guest. | D13, D18, D40 (staff caps) |
| B8 | **Owner-assign followed by courier accept.** | `offerEndsMs` appears in no test, and the courier handlers (tasks/accept/pickup/deliver/refused/shift/earnings) have none. | D16 |
| B9 | **Pass verify after a cancellation; pass reuse.** | No test outside `pass::verify`'s own window cases. | D14 |
| B10 | **Forget with aliases, bookings or queued messages.** | `grep -n alias src/services/customers/forget/tests.rs` → 0. | D15 |
| B11 | **Stock settlement on the CONFIRMED→IN_DELIVERY path.** | `command/tests.rs:245` asserts only that those statuses are "quiet". | D33 |
| B12 | **Outbox drain concurrency and WhatsApp `statuses`.** | `git grep -n '"statuses"' HEAD -- workers/api/src` → 0. There is no lease or in-flight mark. | D20, D21, D22 |
| B13 | **Till close after a sale made after the count; `cash_payments` with REJECTED or refunded rounds.** | `grep -n "counted_at\|stale" src/command/till/tests.rs` → 0. | D12, D31 |
| B14 | **The allergen gate on create and import.** | `git grep -ln "create_product" HEAD -- '*tests*'` → 0. | D17 |
| B15 | **Console JS behaviour: the dish-sheet save round trip and courier cash entry.** | No `*.test.mjs` exists under `public/admin` or `public/courier`. | D19, D36 |

---

## 3. Test-system improvements

Today's escapes share one shape. **Each piece was tested alone, and the defect sat on the seam between two pieces**:
- a JS caller and a Rust route;
- the storefront's clock and the hub's time zone;
- a role's write and another role's read;
- the success path and the idempotency layer on refusal;
- the SW list and the import graph;
- the console's id and the importer's id.

Every test sat on one side of a seam. The improvements below each test a seam.

### 3a. New gates, in the `tools/gates/*.sh` shape (script + baseline + `.prove.sh`)

Each gate follows the house rules:
- It runs on `HEAD` or on a `ROOT` argument, so the `.prove.sh` can plant a defect in a scratch copy.
- The clean tree and a harmless variant must pass. **The green cases are half the proof.**
- Each planted defect must refuse.
- Exit 2 when the gate could not read its own input, because a gate that finds nothing to measure has measured
  nothing.

**G-route: `route-contract.sh`, JS routes ↔ Rust routes.** Would have caught: the kit's `orders/:id` 404 (already
fixed by hand), today's "no bookings screen" class, and B1.

- **Input.**
  - The router literals in `src/lib.rs`: `.(get|post|put|delete)_async("…")`, with `:param` → `[^/]+`.
  - Every path literal in `public/**/*.{js,html}` (tests excluded) under the prefixes `/api/…`, `${API}/…`,
    `api('/…')`, `post('/…')`, `call('/…')`, `c.write('/…')`.
- **Normalisation.** The prototype `routes2.py` implements the first two.
  - `${expr}` and `'…' + expr + '…'` joins become one segment.
  - A trailing `+ q()` or `?…` is a query and is stripped.
  - `const base = \`${API}/…\`` bindings are resolved one level, so that `${base}/…` is followed. `store/booking-mine.js`
    needs this.
- **Refuse (a).** A client path that matches no route. Allowlist: static asset paths under
  `/courier/`, `/platform/`, `/kit/`.
- **Refuse (b).** The method disagrees: the client's `method:` literal, or the helper's default, against the route's
  verb.
- **Ratchet (c).** Routes with no client caller, counted as `unreached_routes=N` in `route-contract.baseline`. The
  baseline lists them by name and may only shrink, except for machine callers named in the baseline (webhooks,
  `/api/print/*`, `/api/mcp`, `/healthz`).
- **Prove.**
  - Clean tree → rc 0.
  - Rename `/owner/reservations` in `admin/bookings.js` → rc 1.
  - Delete the `.get_async("/api/owner/floorplan"` line → rc 1.
  - Add a route with no caller → rc 1 (the ratchet).
  - Send `method:'PUT'` to a POST route → rc 1.

**G-shell: `sw-shell.sh`, service-worker shell completeness.** Would have caught: D2, and today's storefront
`booking.js` miss.

- **Input.** For every `public/**/sw.js`, its entry is the module the matching `index.html` loads with
  `<script type="module" src>`.
- **Walk.** Static `import … from` / `export … from` edges, both relative and absolute, with comments stripped.
  Dynamic `import()` is out, which matches the comments in the SWs. The prototype is `sw.py`.
- **Refuse.**
  - (a) A module in the graph that is missing from `SHELL`.
  - (b) A `SHELL` entry that does not exist at `HEAD`, which would become a 404 at install.
  - (c) `SHELL_CACHE` unchanged while the `SHELL` list or any listed file changed since the base commit. Otherwise
    clients keep the old shell.
- **Prove.**
  - Clean tree (after D2's fix) → rc 0.
  - Add `import './x.js'` to `lib/money.js` → rc 1 for **all four** shells.
  - Remove `'/room/till.js'` from `room/sw.js` → rc 1.
  - List `/lib/nope.js` → rc 1.

**G-idem: `idem-done.sh`, every refusal records its verdict.** Would have caught D1.

- For each handler that calls `idempotency::begin`, or its helper, and then gets `Decision::Proceed`, **every
  `return` after that point** must be preceded by `idem.done(` in the same block, or by a helper that calls it.
- The count is kept by brace matching (the `unreached.py` technique). Baseline `unrecorded_returns=N` is a ratchet
  down to 0.
- **Prove.**
  - Clean tree → rc 0.
  - Delete `idem.done` from `booking/create.rs:229` → rc 1.
  - After the fix, delete it from `room/pay.rs` → rc 1.

**G-venue-day: `venue-clock.sh`, no browser clock or offset for the venue's day.** Would have caught D3, D10 and D11.

- In `public/{store,admin,room,courier}/**/*.js`, refuse any of the following outside `lib/booking-time.js`:
  - `tzOffsetMinutes`;
  - `new Date(<datetime-local value>)`;
  - `getHours()` / `getDate()` / `setHours(` on a value that feeds a request body.
- Also refuse `offsetMinutes(…, now())` whose result is later combined with a non-zero day index. The fix is
  `offsetMinutes(tz, candidateMs)`.
- Baseline is a ratchet.
- **Prove.**
  - Reintroduce `state.loc.tzOffsetMinutes` in `store/booking.js` → rc 1.
  - Planting `new Date(f.value).getTime()` in `checkout.js` → rc 1. This is the current state of the code, so it
    stays RED until D10 is fixed.

**G-principal: `principal-binds.sh`, a principal is bound to the object, not only to the venue.** Would have caught
D13, D18 and D40.

- Every handler that calls `principal_at(` / `Place::of_any(` and then takes an object id from the path (`:id`,
  `:key`, `:token`), or a person id from the body (`wallet`, `userId`, `user`), must also contain one of the named
  binders:
  - `order_id == id`
  - `courier_id ==`
  - `caps.allows(`
  - `owner_and_venue(`
  - `wallet_who(`
- Baseline lists the known exceptions by handler name.
- **Prove.**
  - Remove the `order_id == id` arm in `lib.rs`'s `/api/order/:id` → rc 1.
  - After the fix, `social::messages` without its binder → rc 1.

**G-partial: `partial-write.sh`, a write never rebuilds a record from a subset.** Would have caught the F1 recipe
wipe, D5, D8 (the seed), and D19's half.

- For every `set_product(` / `put(K_PRODUCT` site outside `catalog_edit::create_product`, the value must be built
  as `merge(existing, patch)` (a named helper), not from a fresh `json!({…})`.
- Import paths must pass `Option` fields through as `None` = keep.
- **Prove.** Replace the merge in `owner::update_product` with a fresh object → rc 1.

**G-i18n (keep what exists).** `i18n.mjs`, run today, shows 0 gaps. Promote it to `i18n-parity.sh` with two rules:
- `missing_in_any_lang=0`, a hard refusal.
- `t('prefix_' + x)` must enumerate its suffixes from a named constant (`FLOOR_STATES`, `METHODS`, and so on) that
  the gate checks against every language.
- **Prove.** Delete `floor_state_dirty` from `room/i18n.js` `uk` → rc 1.

### 3b. Cross-role e2e walks (roadmap F6's harness)

Each walk drives **two or more roles** and reads every step back through the other role's API. It closes every TEST
order in `finally`.

| Walk | Steps | Catches |
|---|---|---|
| W1 guest-QR ↔ waiter ↔ till | 1. Scan a code and place a round.<br>2. The waiter sees PENDING; due must **exclude** it.<br>3. Pay, then reject: this must be refused, or produce a refund.<br>4. Close the till: expected must equal counted. | D4, D9, D12, D30 |
| W2 owner assign ↔ courier | 1. The owner assigns an order.<br>2. The courier's `tasks` shows it as theirs; accept or pickup succeeds.<br>3. End the shift. | D16, D33 |
| W3 refused write ↔ retry | 1. With the network cut mid-response, pay over the total.<br>2. Retry with the same key: expect 4xx replayed, never 409 for 24 h.<br>3. The outbox drains. | D1, D7 |
| W4 booking across DST | 1. At the clock seam (the injected `now`), book 26 Oct 19:00 from 24 Oct.<br>2. Guest list, console and the room's held table must all say 19:00. | D3 |
| W5 forget ↔ alias ↔ campaign | 1. Place under `069…`, link to `+355…`, consent, queue a campaign.<br>2. Forget.<br>3. Customer list, bookings image and the outbox all hold no phone; the drain sends 0. | D15, D26 |
| W6 console dish ↔ import ↔ storefront | 1. Add a dish by hand with a recipe.<br>2. Import a two-column price sheet.<br>3. The storefront shows one dish, still stopped, description kept, recipe kept; the allergen gate holds. | D5, D6, D17, D19 |
| W7 offline shells | In each app (store / room / courier), load online, go offline, reload: the app's own offline line renders. | D2 |

### 3c. Property tests over folds

These run in `crates/dowiz-hub`, as proptest-style loops with a seeded RNG, 300 seeds each. Following
`gates-that-count-skips-as-passes`, each also asserts the **number of executed cases**, not only zero failures.

- **P1 money conservation per sitting.** Over random sequences of place / amend / pay / tip / refund / reject /
  transfer:

  `Σ payments − Σ refunded == drawer delta + wallet delta`,

  and `payment_status == "paid"` ⇔ `Σpaid == total`. Catches D4, D8, D12.
- **P2 floor ⇔ sitting agreement.** For every table, `floor.state == Free` ⇒ `live_sitting(table) == None`. Catches
  D30 and D32.
- **P3 stock conservation.** For every order that reaches a terminal status, `reserved == consumed + released`,
  across every FSM path the kernel allows, including CONFIRMED→IN_DELIVERY. Catches D33.
- **P4 import idempotence and preservation.** `apply(import(x)) ; apply(import(x))` is a no-op. For every column
  absent from the file, the stored field is unchanged. Catches D5 and D6.
- **P5 key canonicality.** For every spelling pair of one number, the booking key, the customer key and the wallet
  key agree, or the alias resolves them. Catches D38.
- **P6 venue-day.** For every instant in 2026 and every day index 0..60, `slotOf(now, tz, n, m)` rendered in `tz`
  equals `m`. Catches D3.

### 3d. Mutation targets

Each mutation below must turn at least one test RED. Measure with the house proof shape, or with `cargo-mutants`
scoped to these functions.
- `idempotency::begin`: flip the `done` check.
- `room/pay.rs:97`: remove one status from the refused set.
- `rules.rs:137`: `>` → `>=`.
- `till/fold.rs::cash_payments`: drop the status filter (this one survives today).
- `table_link::verify_table`: skip `plan.find`.
- `booking/pass.rs::verify_pass`: return `ok` for a cancelled booking (this one survives today).
- `forget::find`: skip alias resolution (this one survives today).
- `audience.rs:81`: take only `row.key`.
- `courier.rs::accept`: allow the owner's row.

**Surviving mutants are the blind-spot list, measured.**

---

## 4. Wave G, the prioritised fix list

Rows are proposed, and the roadmap is not edited here. Sizes are as in Wave F: S is at most a lane-day, M a few days,
L a week.

| | Goal | Fixes | CHECK | Size |
|---|---|---|---|---|
| **G1** | Every refusal after `begin` records its verdict (`idem.done(status, error)`). The outbox treats a replayed 4xx as final. | D1 | `idem-done.sh` rc 0 plus its prove script; the W3 walk | S |
| **G2** | The courier shell lists `/lib/vocab.js`, and `sw-shell.sh` guards all four shells. | D2 | `sw-shell.sh` plus prove; the W7 walk | S |
| **G3** | Venue-day everywhere:<br>- `offsetMinutes(tz, candidate)` per day in store and admin bookings;<br>- `todayAt` from `loc.tz`;<br>- checkout "later" parsed as venue time;<br>- `dayRange` from two midnights. | D3, D10, D11 | P6; `venue-clock.sh`; the W4 walk | S |
| **G4** | Pay refuses PENDING, or reject refuses a paid round and routes it to a refund. Amend-to-paid stamps `paid`. Pay carries `base_seq`. Guest PENDING rounds are excluded from due. | D4, D7, D8, D9 | P1; the W1 walk | M |
| **G5** | Refund reverses the wallet SPEND and moves cash out of `expected` (via `took_money`), per currency pile. The till close needs a count at or after the last cash event. | D12, D31 | P1; the W1 walk | M |
| **G6** | Principal binding:<br>- threads bind `order_id == thread_id`, and staff need caps;<br>- the room's wallet debit needs the customer's token or a code;<br>- the booking token is scoped to reservations;<br>- staff booking moves need a cap. | D13, D18, D40 | `principal-binds.sh` plus prove | M |
| **G7** | The pass verifier reads the booking (CONFIRMED/SEATED only, `nonce` equal) and marks the pass used. The pass key is minted on the owner's first write, not on an anonymous verify. | D14 | Tests: cancel → `ok:false`; a second scan → `ok:false` | S |
| **G8** | Forget walks the alias circle, the bookings image and the queued outbox. Consent is read, written and witnessed on the alias circle. Console buttons for Forget and Withdraw consent. | D15, D26, B1 | the W5 walk; `route-contract` ratchet down by 2 | M |
| **G9** | Owner assign produces a row the courier's accept or pickup accepts as their own; an unassign route; `tasks` sends no offer for an assigned order. | D16 | the W2 walk; courier handler tests | S |
| **G10** | Catalogue writes merge: import leaves absent columns alone, one id scheme (or a match by name before insert), and the allergen gate runs on create and import. | D5, D6, D17 | P4; `partial-write.sh`; the W6 walk | M |
| **G11** | Recipe ↔ published values. The dish sheet re-derives from the recipe unless a field was edited in this session. Bulk import honours `nutritionDerived:false`. Clearing a recipe clears its derived values. | D19, D40 (clear) | the W6 walk; `recipe/apply` tests | S |
| **G12** | The outbox gets a lease per drain, a cap per drain, a priority (bells over campaigns), verdicts written per message, and a TTL plus an error-log row for unroutable entries. WhatsApp `statuses` are read into the verdict. | D20, D21, D22 | Tests: two overlapping drains send once; 131047 → failed | M |
| **G13** | A9 closed. A dine-in order without staff needs a valid `table_link`; `live_sitting` ignores paid sittings; codes carry a rotation epoch the owner can bump. | D23, D30 | Test: an unsigned dine-in → 400; P2 | S |
| **G14** | Stock settles on every terminal path (CONFIRMED→IN_DELIVERY→DELIVERED consumes). | D33 | P3 | S |
| **G15** | Stamps: redemption needs the order's verified customer; a rejected own round earns nothing. | D24, D25 | `stamps` tests for both | S |
| **G16** | The floor-plan save refuses to orphan a live hold or lower seats below a booked party, naming the booking. Booking create re-checks hours and the grid; `cancellation_is_free` is wired in. The booking retry reuses its `requestId` until it succeeds. | D27, D28, D29 | `floor` tests; a booking-create test at 03:00 → 400 | M |
| **G17** | `route-contract.sh` plus baseline, and the uncalled owner routes either get a UI or are deleted: `stock/waste`, `restore`, `history`, `rotate`, `logo/clear`, `i18n`, `branding/preset`, `zones`, `graph`, `rekey`. | B1 | `route-contract.sh` rc 0, with `unreached_routes` at its new floor | M |
| **G18** | Fiscal:<br>- tips and comps carried as document lines;<br>- reconcile searches the document's own date and pages to the end;<br>- a codes write that misses its order keeps the queue entry. | D34, D37, D40 (fiscal) | `ebills_body` tests with a tip; a reconcile mock that honours range and paging | M |
| **G19** | Courier cash in major units, parsed per currency decimals (`lib/money.js`). A deactivation scoped to one venue. Earnings by delivery time. | D36, D39, D40 | node test on the cash parser; hiring tests | S |
| **G20** | One phone canonicaliser for bookings, customers and the wallet, and the console supply form refuses a unit change. | D38, D35 | P5; a supplies test | S |

**Order.**
1. G1, G2 and G3 first. They are proved, small, and each hits a role at the first bad moment: a waiter's queue, a
   courier offline, a booking after 25 Oct, which is 31 days away.
2. Then G4, G5, G9 and G7, the money and a blocked role.
3. Then G6, G8 and G10, privacy and the catalogue.
4. The gates in §3a land **with** the row they guard, RED first on the unfixed tree, so each gate is triggered
   before it is trusted.
