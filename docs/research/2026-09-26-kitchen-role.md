# The kitchen role (роль кухні): who, what screen, what permissions, what workflow

Research, 2026-09-26. READ-ONLY; nothing in the tree was changed. Every code claim below was
checked against `/root/dowiz` at HEAD `92219a57` (the working tree's half-finished `crates/dowiz-hub`
split is ignored; `caps.rs` and `stock.rs` were read with `git show HEAD:`). The inventory DATA MODEL
and consumption ANALYTICS are covered in `docs/research/2026-09-26-menu-ingredients-stock.md`; this
note is only about the ROLE. Anything not verified is marked **(unverified)**.

## 0. The one-paragraph answer

The kitchen role already exists as a PRINCIPAL and has no SCREEN. `dowiz_hub::caps::Preset::Kitchen`
holds exactly one capability, `advance`; a cook can log in (`POST /api/staff/login`), can open the live
socket, can bump an order (`confirm | preparing | ready`) and can tap "seen" — and cannot read the list
of orders they are supposed to bump, because the two list routes ask for `take_orders` or an owner. The
room app tells a kitchen login "the kitchen follows orders on the kitchen screen" and there is no
kitchen screen. Stock in/out is owner-only except write-offs, which need `open_till` (the till holder,
not the cook). So the work is: one new capability word for stock, one read route shaped for a pass, one
new surface `/kitchen/` built from the room app's skeleton, and the yield test the operator just asked
for. Nothing in the order FSM or the stock ledger needs to change.

## 1. Current state, with file:line

### 1.1 Principals and capabilities

| Thing | Where | What it says |
|---|---|---|
| Token claims | `workers/api/src/auth.rs:42-88` | `Claims::{Owner, Courier, Customer, Staff}`; `Staff { sub, active_location_id, jti, caps, iat, exp }` — `caps` is a signed comma list |
| Live principal | `auth.rs:352-368` | `Principal::Staff { person_id, active_location_id, session_id, caps: Caps }` |
| Staff arm of `authenticate_token` | `auth.rs:588-611` | re-reads the membership row AND the session row on every call; token can only lose rights |
| Capability law | `crates/dowiz-hub/src/caps.rs` (HEAD) | closed set of FIVE caps: `advance, take_orders, take_payment, void, open_till`; four presets; **`Preset::Kitchen => [Advance]`** |
| Decision behind it | `DECISIONS.md:297-298`; `docs/design/BLUEPRINT-POS-THE-ROOM-2026-09-22.md:297-318` (§2.8) | "Owner / Kitchen / Counter-Manager" + the operator's fourth word Waiter (2026-09-23) |
| Room's door | `auth.rs:392-409` `room_admits` | staff pass on `caps.allows(need)`; owner passes everything; 404 for another venue |
| Route guard used by staff routes | `workers/api/src/courier.rs:55-73` `staff_at(req, ctx, venue, need)` | a staff token is asked for `need`; any other token falls back to `owner_at` |
| Roster admin | `lib.rs:322-324`; `services/identity/staff_admin.rs` | `GET /api/owner/staff`, `POST /api/owner/staff/invite`, `POST /api/owner/staff/:id`; console offers `['kitchen','waiter','counter-manager']` (`public/admin/staff.js:13`) |
| Staff login | `services/identity/staff.rs` (`POST /api/staff/login`, `/claim`) | **email + password**; venue from Host; session row `ssession` bound by `jti`; TTL `STAFF_TTL_MS = 12 h` (`staff_rules.rs:30`); invite 7 d; min password 8 |
| MCP keys per staff | `lib.rs:433-435`; `mcp/tools.rs:106,108` | an `advance` key gets `order action {confirm,preparing,ready}` and `kitchen-ack` only |
| Tests already pinning the kitchen | `auth.rs:787-808` | "kitchen keeps advance", "a kitchen token minted with open_till is narrowed" |

### 1.2 What a kitchen principal can reach today

| Route | Guard | Kitchen (`advance`)? |
|---|---|---|
| `POST /api/owner/orders/:id/action` (`owner.rs:530`) | `staff_at(.., Cap::Advance)` — comment: "THE KITCHEN MOVES ROUNDS TOO" | **yes**: `confirm, reject, preparing, ready, collected, cancel` map at `owner.rs:560-571`; the kernel refuses illegal edges |
| `POST /api/staff/orders/:id/kitchen-ack` (`lib.rs:328`; `services/orders/kitchen_ack.rs:33`) | `Cap::Advance` | **yes** — writes `kitchen.seen {by, at}` once (`command/kitchen_ack.rs:109-111`) |
| `GET /api/live` (`live.rs:99-107`) | staff with `Advance` OR `TakeOrders` get `TAG_CONSOLE` | **yes** — the FULL console stream (every order delta incl. `contact`, `fulfilment.address`) |
| `GET /api/staff/room` (`services/orders/room/handlers.rs:27`) | `Cap::TakeOrders` | **no** |
| `GET /api/owner/orders` (`owner.rs:353` `owner_beside`) | owner only | **no** |
| `POST /api/owner/stock/:kind` received / stocktake (`services/operations/stock.rs:147-152`) | owner only | **no** |
| `POST /api/owner/stock/wasted` (`stock.rs:154-169`) | `Cap::OpenTill` ("the drawer's holder is who bins the stock at midnight") | **no** |
| `GET /api/owner/stock`, `/stock/waste` (`lib.rs:396-398`) | owner | **no** |
| `POST /api/owner/products/:id` — the 86 switch `available` (`admin/menu.js:135,201`; `owner.rs:790` `owner_at`) | owner | **no** |
| `POST /api/staff/orders/:id/refund` (`services/orders/refund.rs:34,89`) | `Cap::Void` | **no** (correct) |

**Gap 1 (the blocker):** a kitchen can WRITE to orders it cannot READ. There is no list route a
kitchen token passes. The room app confirms the design intent and the hole: `public/room/app.js:1-3`
"The kitchen has its own screen", `room/app.js:90` `if (!S.loc || S.role === 'kitchen') return;`,
`room/i18n.js:16,60` `kitchenNoRoom`. There is no `public/kitchen/` directory; `grep -rli kitchen
public/` finds only console, kit and lib files.

**Gap 2 (privacy):** the only read a kitchen HAS — the socket — is the widest one. `live.rs:97-98`
says the console tag carries "every live order of the venue" and admits `advance` to it. A cook's
tablet at the pass receives customer names, phones and delivery addresses it has no use for
(`storefront.rs:1119-1130` writes `contact{name,phone}` and `fulfilment.address{line,note,parts,lat,lon}`
onto the envelope). The privacy registry lists the order log's subjects as customers
(`privacy/registry/venue.rs`), and a KDS is a new RECIPIENT of that data inside the venue.

**Gap 3 (stock):** `stock.rs:147-152` keeps `received` and `stocktake` owner-only and `wasted` behind
`open_till`. The operator's stated wish — кухня робить приход/розхід — is exactly the three movements
`movement()` already builds (`stock.rs:117-142`: `received {item, qty}`, `wasted {item, qty, reason ∈
Spoiled|Dropped|Unsold|Returned|StaffMeal, by}`, `stocktake {item, observed, stocktake_id, by}`), refused to
the kitchen at the door. No new event kind is needed for receive/waste/count.

**Gap 4 (yield):** no `yield` field exists anywhere (`grep -rni yield` over `recipe.rs`, `stock.rs`,
`catalog`: only unrelated uses). `recipe.rs:1-19`: a supply has kind, unit (g/ml/unit), nutrition per
100, cost, weight per piece; a BOM line is `{supply, qty}` per portion. Raw-vs-cooked weight has no
home — that is the other researcher's model; §5 here is the CAPTURE workflow.

### 1.3 Kitchen-facing machinery that already exists (reuse, do not rebuild)

- **Seen / printed / unconfirmed** — `command/kitchen_ack.rs`: `UNSEEN_AFTER_MS = 180_000`, owed at
  `PENDING|CONFIRMED`; the owner's health pane lists unseen tickets. A KDS that opens a ticket should
  call this, which turns "seen" from a tap into a fact of display.
- **Print rail** — `print_rail.rs:1-55`: CloudPRNT-style printer polls `/api/print/poll`, `DELETE` =
  printed, order carries `kitchen.printed|print_failed`; one setting `print.kitchen`
  (`admin/printer.js`). A KDS and a printer coexist: printed ≠ seen by design (`print_rail.rs:21-26`).
- **Stations** — `bell_route.rs:27-56`: a product carries `station ∈ {kitchen, bar}` (absent =
  kitchen); lines split per station; Telegram gets one message per station when
  `notify.telegram.chat.bar` is set. The KDS filter "my station" is this field, already on every line.
- **Kitchen profile** — `eta.rs:64-75`: `kitchenStations`, `defaultCookingMin`, `perExtraPortionMin`
  on the location record; `live_eta::stamp` writes ETA stamps at each transition. The ticket timer's
  "expected" line comes from here for free.
- **Order FSM** — `crates/dowiz-core/src/order_machine.rs:112-128`: `Pending→{Confirmed,Rejected,Cancelled}`,
  `Confirmed→{Preparing,InDelivery,Refunding}`, `Preparing→{Ready,Refunding}`,
  `Ready→{InDelivery,PickedUp,Refunding}`, `InDelivery→{Delivered,Refunding}`, `Refunding→{CompensatedRefund}`.
  The "no exit past PENDING" defect is FIXED at HEAD: `command/refund.rs` emits `REFUNDING →
  COMPENSATED_REFUND` in one turn (needs `void`, not the kitchen's business).
- **Shelf settlement** — `command/advance.rs:57-63`: `PREPARING` consumes, `REJECTED|CANCELLED`
  release, everything else leaves the shelf alone; and `advance.rs:85-95` refuses cancel/reject on a
  round with money on it. The kitchen's "start" tap IS the consumption moment.
- **Realtime** — `live.rs` + `hubdo.rs:450-500` broadcast; `lib/live.js` client (ping 25 s, dead 55 s,
  poll fallback); console `admin/app.js:292-300` uses the socket only as a nudge (`onEvent →
  refreshNow()`), poll 15 s / 60 s idle (`admin/core.js:27,30`); room polls 20 s (`room/app.js:32`).
  `RECENT_KEEP = 256` deltas answer `?since=` (`hubdo.rs:98`).
- **Replica + shell** — `lib/replica.js:1-14` ("a kitchen's console is the one screen in this system
  that must not go blank"), `room/sw.js` network-first shell, `lib/outbox.js` for queued taps.
- **Ring** — the console already synthesises a two-note ring on a new order (`admin/app.js:236-242`).
- **Dark theme** — `lib/tokens.css:237-239` `data-theme`; the room HUD has `themeBtn`
  (`room/index.html:27`).
- **Automatic 86** — `stock.rs` HEAD:15-19: an order reserving more than available is REFUSED with
  `OutOfStock`; that refusal is the stop-list. A manual 86 is the product's `available` flag.

### 1.4 Gates a new surface must satisfy (`tools/gates/run-all.sh`)

| Gate | Scope today | For `/kitchen/` |
|---|---|---|
| `design-gate` (`scripts/design_gate.py:5-16`) | storefront, admin, courier, platform, landing — **room is NOT listed** | add `"kitchen": ("kitchen/index.html", "kitchen/app.js")` on the first commit (the room's omission is a pre-existing gap worth a row of its own) |
| `ui-adoption.sh` | admin courier kit room store | add `kitchen`; build only from `/lib/ui` (`button, card, chip, field, list, segmented, sheet, toast, time, money, empty, skeleton, badge, tabs`) |
| `tap-size.sh` (F37, `--tap: 44px`) | courier, kit | add kitchen; the KDS needs LARGER than 44 px (see §6) |
| `sw-shell.sh` (G2) | admin/courier/kit/room/storefront | `kitchen/sw.js` must list every statically imported module |
| `learn.sh` | anchors-{admin,courier,room,store}.txt + 61 lessons (owner 36, waiter 12, courier 7, guest 6) | `docs/learn/anchors-kitchen.txt` + lessons `K1..Kn` in sq/en/uk with `data-tour="kitchen.<control>"` |
| `personal-data.sh` (P1) | registry rows | new recipient row: the KDS device as a place PII is DISPLAYED; and the stock image row (`venue.rs:143-148` "NotPersonal") is already wrong-ish — `Wasted.by` / `Stocktake.by` carry a person id |
| `principal-binds.sh` (G6) | every route | the new list route and stock routes bind to `location_id` of the token AND the object |
| `one-venue.sh` (F2) | owner routes | reuse `location_of(req)` + `staff_at` exactly as `waste_signer` does (`stock.rs:158-169`) |
| `file-size.sh` (F1 ratchet) | Rust + JS | new files ≤ 300 lines; `app.js` of the room is the pattern (modules per screen) |
| `vocabulary.sh` (F4) | clients speak `OrderStatus` | KDS columns must use the twelve kernel words, never "cooking" |
| `no-scoring.sh` (F36) | everywhere | NO per-cook speed ranking on the KDS (Poster sells "chef performance"; DECISIONS OD-8 forbids it) |
| 100 % coverage (operator rule 2026-09-24) | all | RED-then-GREEN test per row; `node --test` for `kitchen/*.test.mjs`, `cargo llvm-cov` for routes |

## 2. Role definition

### 2.1 Who

The kitchen role is a PLACE more than a person: the pass, the sushi bar, the hot line. In Durrës the
venue has a sushi bar and a hot kitchen (`bell_route` already splits `kitchen | bar`). One person may
hold the roster word `kitchen` at several venues (membership rows are per venue, `identity_store.rs`).
The role is deliberately NOT a rank: `caps.rs` forbids ordering caps, and F36 forbids scoring.

### 2.2 Permissions matrix (proposed)

`+` = holds today, `NEW` = proposed, `–` = must not.

| Action | Owner | Counter-Manager | Waiter | Courier | **Kitchen** |
|---|---|---|---|---|---|
| See open tickets (items, qty, modifiers, table/kind, time, station) | + | + (`room`) | + (`room`) | – | **NEW** `kitchen_view` route, PII-stripped |
| See customer name / phone / address | + | + | + | + (address only, own job) | **–** (table number and fulfilment kind only) |
| See prices, bill, payments | + | + | + | – | **–** (not on the ticket) |
| `kitchen-ack` (seen) | + | – | – | – | + |
| confirm / preparing / ready (`advance`) | + | – | – | – | + |
| reject / cancel (`advance` today!) | + | – | – | – | **remove from the KDS UI** (keep the cap; the route already refuses when paid). Operator decision Q1 |
| collected (READY → PICKED_UP) | + | – | – | – | + (the pass hands the bag over) |
| amend / add lines | + | + | + | – | – |
| void / comp / refund | + | + | before kitchen | – | – |
| till | + | + | – | – | – |
| stock: received (приход) | + | – | – | – | **NEW** cap `stock_move` |
| stock: wasted (списання) | + | + (`open_till`) | – | – | **NEW** `stock_move` |
| stock: stocktake (інвентаризація) | + | – | – | – | **NEW** `stock_move` |
| see levels / low list / all-day counts | + | + | – | – | **NEW** (read half of `stock_move`) |
| 86 a dish from the line (`available=false`) | + | – | – | – | **NEW** cap `stop_list` — or PROPOSE only (Q2) |
| yield test: record raw/cleaned/cooked weights | + | – | – | – | **NEW** `stock_move` (it is a stock fact) |
| yield: change a recipe's EXPECTED yield | + | – | – | – | – (kitchen proposes, owner approves — operator's rule) |
| add / edit supplies, recipes, prices | + | – | – | – | – |
| staff roster, invites | + | – | – | – | – |
| print settings, integrations, reports | + | – | – | – | – |
| MCP key for own role | + | + | + | + | + (already, `mcp::staff_mint`) |

Two new capability words, both in `caps.rs` (one variant + one bit + one row in `Preset::caps`, exactly
what the module header says a fifth word costs):

- `stock_move` (bit 0x20): received / wasted / stocktake / yield-test, and the stock read. Kitchen and
  Counter-Manager hold it; Waiter does not. The existing `wasted` guard becomes `OpenTill OR StockMove`
  (the midnight bin is still the manager's).
- `stop_list` (bit 0x40): flip `available` on a product and set `unavailableNote`. Whether Kitchen holds
  it directly, or only a "propose 86" that the owner confirms, is **Q2** — the console's current 86
  is an owner edit of the catalogue (`owner.rs:790`), and a cook 86-ing "salmon" at 20:00 is the
  single most valuable kitchen action in a sushi venue (it stops oversell before the ledger would).

Presets after the change: `Kitchen = {advance, stock_move[, stop_list]}`; `CounterManager` gains
`stock_move`; `Owner = ALL`. The five existing tests in `auth.rs:787-838` and `caps.rs` tests pin the
narrowing; add twins for the new words.

### 2.3 Login and device model

Today: personal email + password, 12 h session, revocable per device (`staff_rules.rs:14-17`). For a
pass tablet that is wrong in two ways: (a) nobody types an email with wet gloves; (b) a shared tablet
means one login for a shift and every `by` reads the same person.

Recommendation (Square's shape: a device code for the DEVICE, a 4-digit personal passcode per team
member — https://squareup.com/help/us/en/article/8357-require-passcodes-at-point-of-sale,
https://squareup.com/help/us/en/article/8339-set-up-device-codes):

1. **Device session** (already possible): the owner logs the tablet in once with a kitchen account
   ("pass-1@venue"), session TTL for a kitchen device raised from 12 h to e.g. 7 days
   (`STAFF_TTL_MS` is per claim, so a second constant `KITCHEN_DEVICE_TTL_MS` costs nothing). The owner
   ends it from `POST /api/owner/staff/:id` (suspend) or by session id.
2. **Who-tapped PIN** (phase 2): a 4-digit PIN per kitchen person, stored as a digest on the
   membership row, entered on the tablet only for WRITES that matter for audit (waste, stocktake,
   receive, yield). The tablet keeps the device token; the PIN selects the `by`. Server side this is a
   `by_pin` field on the stock routes checked against the venue's roster — NOT a second token
   system. Bumping a ticket needs no PIN (speed beats attribution; Toast/Square do not ask either).
3. **Never** a customer-style QR token for staff: it has no session row and cannot be revoked
   (`auth.rs:51-56` says why the courier's `jti` is mandatory).

**Q3 for the operator:** one shared tablet per station with PIN-for-writes (recommended), or personal
phones with personal logins (what the waiter has)?

### 2.4 Audit trail

Every write the kitchen makes is already a signed fact: `Advanced` events carry `by` via
`order_action`'s `who` (`owner.rs:530-533`), `kitchen.seen {by, at}`, `Wasted {by}`, `Stocktake {by}`,
`Received` (**no `by` today** — `stock.rs` HEAD:87 `Received { item, qty }`; the other researcher should
add a signer, since a delivery received is exactly what an owner wants attributed). The roadmap's
yield event must carry `by` from day one (`stock::signed` requires it for human-caused events,
HEAD:504-516). Nothing on the KDS is a score: "ticket age" is a property of the ticket, "average
fulfilment" (Toast) is a property of the STATION, and neither is stored per person.

## 3. Screens (text wireframes)

All screens: `/kitchen/` on the venue's host, `lib/tokens.css` + `lib/ui`, dark theme default
(`data-theme="dark"` unless the device says otherwise), three languages via the room's `i18n.js`
shape, network-first shell `kitchen/sw.js`, replica-first draw, outbox for taps, ring on a new ticket
(reuse `admin/app.js:236-242`). Landscape 10" tablet; also works on a phone in portrait (one column).

### 3.1 KDS board (`kitchen.board`)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ ● LIVE  Sushi Durrës · KUZHINA          [Kitchen ▾][Bar]   AllDay  ⟲Recall  ☾ │  HUD: sync chip, station filter, all-day, recall, theme, lang
├────────────────┬────────────────┬────────────────┬────────────────┬──────────┤
│ #A17  Tavolina 4│ #A18  MARRJE   │ #A19  DËRGESË  │ #A20  Tavolina 2│          │  ticket head: short id, TABLE or PICKUP/DELIVERY (no name, no address)
│ 02:10  PENDING │ 06:40 CONFIRMED│ 11:05 PREPARING│ 00:30  PENDING │          │  age mm:ss, kernel status word; head colour by age (see §6)
│────────────────│────────────────│────────────────│────────────────│          │
│ 2× Philadelphia│ 1× Sake nigiri │ 3× Dragon roll │ 1× Miso        │          │  qty × name, modifiers indented, station icon per line
│    – no sesame │ 1× Tuna nigiri │ 1× Edamame     │ 2× Gyoza       │          │
│ 1× Miso        │ 2× Gyoza       │                │                │          │
│ ⚠ note: alergji│                │ ▮ ETA 18:42    │                │          │  fulfilment.note shown; ETA stamp from live_eta
│────────────────│────────────────│────────────────│────────────────│          │
│ [   START    ] │ [   START    ] │ [   READY    ] │ [   START    ] │          │  ONE big bump button per ticket (≥ 88 px tall, full width)
└────────────────┴────────────────┴────────────────┴────────────────┴──────────┘
  oldest left → newest right; a ticket taller than the column continues in the next column ("CONTINUED", Toast)
```

Rules: opening the board with a ticket visible fires `kitchen-ack` once per ticket (seen = displayed,
not tapped — Q4). START = `preparing` (consumes stock); READY = `ready`; a READY pickup ticket shows
`HANDED OVER` = `collected`; delivery tickets at READY leave the board (courier's). No reject/cancel
on this screen (Q1). Long-press a ticket → detail sheet with the recipe (Poster/Toast both show the
recipe on the ticket; dowiz has `lines_of_stored`). Course pacing (BLIND-SPOTS P3-5) is out of scope
for sushi; the hold/fire pair is already `CONFIRMED` (held) → `PREPARING` (fired).

### 3.2 All-day counts (`kitchen.allday`) — a sheet over the board

```
┌ All day · open tickets ─────────────┐
│ Philadelphia  ×7   Dragon roll ×4   │   sum of qty per product across PENDING/CONFIRMED/PREPARING
│ Sake nigiri   ×5   Gyoza       ×6   │   (Fresh: "ingredient all-day counts" is a paid tier; dowiz gets it from bom_of for free)
│ ── ingredients ──                    │
│ salmon 1 240 g  rice 3.1 kg  nori 22│   bom_of(product) × qty, folded — the prep list's live half
└─────────────────────────────────────┘
```

### 3.3 Stock: receive (`kitchen.receive`, приход)

```
┌ Receive delivery ───────────────────────────────┐
│ Supplier: [Butrinti Fish ▾]   Doc/invoice: [____]│   optional; supplier is free text today (Q5 – model)
│ ┌ item ───────────┬ qty ──┬ unit ┬ temp °C ┬ ✓ ┐ │
│ │ Salmon fillet   │ 4 200 │  g   │  2.0    │ ✓ │ │   temp field only for supplies flagged chilled/frozen (HACCP)
│ │ Nori            │   200 │ unit │   –     │ ✓ │ │
│ │ + add item…                                  │ │
│ └──────────────────────────────────────────────┘ │
│ ⚠ Salmon 2.0 °C ok (≤ 5 °C)   ✗ over 5 °C → "reject / accept with note"                          │
│ [ PIN 1234 ]   [    RECEIVED ✓    ]              │   one Received event per line; by = PIN person; photo of the invoice optional (R2)
└──────────────────────────────────────────────────┘
```

FDA Food Code 2022 §3-202.11: refrigerated TCS food received at ≤ 5 °C (41 °F)
(https://www.fda.gov/media/164194/download). Raw fish for sushi: parasite-destruction freezing
−20 °C ≥ 7 days or −35 °C ≥ 15 h (§3-402.11; text at
https://app.leg.wa.gov/wac/default.aspx?cite=246-215-03425). EU 853/2004 Annex III §VIII has the same
−20 °C/24 h rule for fishery products eaten raw **(unverified in this session; Albania aligns with EU
food law)**. The receive screen should ask for the supplier's freezing declaration once per
fish supply, not per delivery.

### 3.4 Stock: waste (`kitchen.waste`, списання)

```
┌ Write off ──────────────────────────────┐
│ [ Salmon fillet ▾ ]   qty [ 350 ] g      │   item picker = supplies with a level > 0, biggest first
│ why:  (Spoiled) (Dropped) (Unsold)       │   the FIVE WasteReason words, as chips, required
│       (Returned) (Staff meal)            │
│ [📷 photo]  note [________]              │   photo optional, stored in R2 under the venue (Winnow/Leanpath photograph every disposal)
│ [ PIN ]        [   WRITE OFF   ]         │   → POST /api/owner/stock/wasted with the kitchen's new cap
└──────────────────────────────────────────┘
```

### 3.5 Stock: stocktake (`kitchen.count`, інвентаризація)

```
┌ Count · Fri 26 Sep · fridge 1 ─────────────────┐
│ Salmon fillet    expected 3 850 g   counted [____]│   expected = ledger available; entering a number
│ Tuna loin        expected 1 200 g   counted [____]│   shows the delta live; blank = not counted (no event)
│ Rice (cooked)    expected   –       counted [____]│
│ …                                                 │
│ ⚠ 3 items differ by > 10 %                        │
│ [ PIN ]   [ SAVE COUNT (12 items) ]               │   one Stocktake event per counted line, same stocktake_id
└───────────────────────────────────────────────────┘
```

### 3.6 Prep list (`kitchen.prep`)

```
┌ Prep · today ───────────────────────────────────┐
│ From bookings + yesterday's same weekday:        │   forecast = orders of the last N same weekdays (fold over the log) + today's bookings (booking.rs)
│ rice (cooked)   need ~9 kg   have 2 kg   → cook 7 kg │
│ salmon (cleaned) need ~3.2 kg have 3.8 kg  ok       │
│ tamago          need ~40 pcs  have 12    → make 28  │
│ [ mark done ✓ ]                                  │   "done" is a client note; the STOCK truth is the receive/yield event that follows
└──────────────────────────────────────────────────┘
```

Phase 3; depends on the other researcher's prepared-item model (rice cooked is an OUTPUT of rice raw).

### 3.7 86 toggle (`kitchen.stoplist`)

```
┌ Stop list ─────────────────────────────┐
│ [search dish…]                          │
│ Dragon roll        ● on sale   [ 86 ]   │   tap → available=false + note "kitchen 20:14"; storefront hides it within its next menu read
│ Philadelphia       ○ 86'd 19:50 [ back ]│   (edge cache: the store reads the catalogue with ?fresh=1 trap in memory; verify propagation)
│ Salmon nigiri      ● auto (stock 0)     │   the ledger's OutOfStock refusal, shown, not editable
└─────────────────────────────────────────┘
```

### 3.8 Yield / butchery test (`kitchen.yield`) — the operator's new requirement

```
┌ Yield test · Salmon, whole ──────────────────────────────┐
│ batch: [Butrinti 26 Sep ▾]   (or "prep session 26 Sep 09:00") │
│ 1  RAW IN (AP)        [ 4 200 ] g   ⚖ from scale / typed   │   the big number pad; "⚖" reads a BLE scale later (phase 4, optional)
│ 2  after cleaning (EP)[ 2 940 ] g   → 70 %  (expected 68 %)│   live % vs the recipe's expected yield
│ 3  after cooking      [ _____ ] g   → –   (only for cooked items, e.g. rice, tamago)          │
│ trim to stock?  (skin 180 g → "salmon skin")  (bones 1 080 g → waste: Trim)   │   Q6: add WasteReason::Trim?
│ note [__________]   [ PIN ]   [  RECORD  ]                 │
└────────────────────────────────────────────────────────────┘
  later: "70 % differs from expected 68 % — propose new expected? [propose]" → owner sees it in the console's supplies sheet and accepts or not
```

Workflow: **per batch** (a delivery lot / one fish), not per prep session, because that is what
yield-test practice measures — "doing yield tests on several of the same items and taking an average
will give you the best idea of your standard yield" (https://psu.pb.unizin.org/hmd329/chapter/ch7/;
https://opentextbc.ca/basickitchenandfoodservicemanagement/chapter/yield-testing/). A prep SESSION
(morning rice, 5 kg raw → 11 kg cooked) is the same screen with step 3 filled and step 2 skipped. The
arithmetic is fixed: `EP = AP − waste`, `yield % = EP / AP × 100`, `EP cost = AP cost × 100 / yield %`
(https://www.getmeez.com/blog/the-benefits-of-yield-testing-in-cooking). The kitchen RECORDS; the
recipe's EXPECTED yield changes only when the owner accepts a proposal (operator's rule).

Scale entry UX: one field active at a time, a full-height number pad (keys ≥ 64 px), unit fixed by the
supply, a "same as last" key, no decimals for grams, and the % updates as you type. A BLE kitchen scale
via Web Bluetooth is Chrome/Android only **(unverified whether any scale the venue owns speaks a
documented GATT profile)** — leave it as an optional phase.

## 4. Workflow integration with the FSM and the ledger

| Moment | Who taps | FSM edge | Stock | Record |
|---|---|---|---|---|
| Order placed (store / waiter) | – | `PENDING` | `Reserved` per BOM line (`place`) | `Placed` |
| Ticket shows on the KDS | device | – | – | `kitchen.seen {by, at}` (once) |
| Owner/kitchen accepts | either | `PENDING→CONFIRMED` | – | `Advanced` |
| **START** on the pass | **kitchen** | `CONFIRMED→PREPARING` | **`Consumed`** (`advance.rs:57-63`) | `Advanced` |
| **READY** | **kitchen** | `PREPARING→READY` | – | `Advanced`; courier pool / waiter notified via the socket |
| Handed over at the counter | kitchen or waiter | `READY→PICKED_UP` | – | terminal |
| Delivery | courier | `READY→IN_DELIVERY→DELIVERED` | – | courier's |
| Something went wrong after START | owner / counter-manager | `→REFUNDING→COMPENSATED_REFUND` | consumed stays consumed; `Returned` optional | `refund` (needs `void`) |
| Delivery arrives | kitchen | – | `Received` | new signer field (model row) |
| Bin | kitchen | – | `Wasted {reason, by}` | as today |
| Count | kitchen | – | `Stocktake` | as today |
| Yield test | kitchen | – | new event (model row): AP in, EP out, cooked out, trim → waste/secondary stock | proposal to owner |

The kitchen OWNS `CONFIRMED → PREPARING → READY` and `READY → PICKED_UP` for collection. Consumption is
recorded at START and nowhere else — this is already the law and the KDS must not add a second
consumption on READY. Whether a venue wants the kitchen to also CONFIRM (skip the owner's phone) is a
venue setting, not a capability (`advance` already allows it) — **Q7**.

## 5. Realtime and cost on the Free plan

Facts (`docs/design/BLUEPRINT-FREE-TIER-2026-09-26.md:97-120`, memory `dowiz-costs-free-plan-and-bebop`):
Worker 100 000 req/day and **10 ms CPU** per request (1.4 % of requests are killed at exactly 10 ms
today); DO 100 000 req/day is the first cap to trip; a socket upgrade is one Worker request; outgoing
broadcasts are NOT requests (`hubdo.rs:445-449`); each INCOMING socket message is one DO request; the
client's 25 s ping is 1 728 DO requests/day per open socket (FT6: `set_websocket_auto_response` is
NOT yet in `hubdo.rs` — grep finds none).

How KDS updates should arrive:

1. **Socket, applying the delta locally** — not the console's "nudge then GET" (`admin/app.js:297`),
   which spends a Worker request + an object read per event per screen. `live.js` already delivers
   `{t:"event", kind, orderId, payload, generation}`; `lib/replica.js` already folds deltas. A KDS with
   two screens (bar + kitchen) then costs: 2 upgrades + pings. Ship FT6 first or the two kitchen
   sockets alone add ~3 500 DO requests/day (3.5 % of the daily cap) for nothing.
2. **The kitchen read route answers `?since=`** exactly like `/api/owner/orders` (`owner.rs:262-275`),
   so a reconnect after a wifi drop is one small request, and a cold open is one full list.
3. **Poll fallback at 30 s while the socket is down** (room uses 20 s; the KDS has the socket, so its
   poll can be slower), 0 when the screen is hidden.
4. **CPU**: the kitchen list must NOT load the catalogue image (the 538 KB `load_catalog` is what
   trips the 10 ms kills at dinner, FT1). Product names already ride on the line
   (`storefront.rs:1050-1052`); modifier NAMES do not (`modifier_ids` only, `:906`) — either stamp
   modifier names at placement (one model row; the print rail must already resolve them for the
   ticket text) or let the KDS hold the catalogue it read once at login, in the replica.
5. **Order of magnitude**: a busy night of 150 orders × ~4 transitions = 600 events; on sockets that
   is 0 extra requests; on the console's nudge pattern it is 600 × screens Worker requests. Either
   is far under the daily cap; the pings and the cron are what bind, not the tickets.

## 6. What the KDS market does (for the design, with sources)

| Pattern | Where seen | Take for dowiz |
|---|---|---|
| Ticket header colour by AGE, thresholds in minutes (yellow/red) | Square KDS setup: "Set the times after your Yellow timer and Red timer activate" https://squareup.com/help/us/en/article/7944-get-started-with-square-kds-android ; Toast: "Different ticket heading colors based on the age of the ticket" https://doc.toasttab.com/doc/platformguide/platformKDSOverview.html ; Fresh "On-Time / Caution / Late" https://www.fresh.technology/kds-features/on-time-caution-late-ticket-headers | two venue settings `kds.warnMin`, `kds.lateMin`; default from `defaultCookingMin` (`eta.rs:66`) |
| Status colours: blue = time to cook, green = ready, grey = served, red = overdue with negative minutes | Syrve https://www.syrve.com/en-gb/blog/how-syrve-works-part-4-the-kitchen-display-system | show "−3 min" past the ETA stamp rather than a bare age |
| Bump / recall last fulfilled; "complete for all devices" vs "only this device" | Toast overview (recall); Square (complete/recall scope) | one Recall button = `?since` replay of the last READY ticket, read-only; bump is global (it is an FSM edge) |
| All-day view / item summary / ingredient all-day counts | Toast all-day view; Square video https://www.youtube.com/watch?v=zGv8q9U-KXQ ; Fresh https://www.fresh.technology/features/ingredient-all-day-counts | §3.2, folded from `bom_of` |
| 86 from the KDS | Square video; Fresh comparison https://www.fresh.technology/kds/fresh-vs-square | §3.7, `stop_list` cap (Q2) |
| Prep vs expediter stations; assembly lines; station pausing | Toast https://support.toasttab.com/en/article/Grid-KDS-Overview ; Fresh | dowiz: `station` on the line is enough for two screens; no expo screen for a sushi bar |
| Courses / hold-and-fire | Poster Kitchen Kit https://joinposter.com/en/tour/addons/kitchenkit ; Syrve courses view | out of scope (BLIND-SPOTS P3-5 "Low — sushi") |
| Recipe on the ticket | Poster ("Dish recipe on KDS"); Toast "Viewing recipes" | long-press → recipe sheet from `lines_of_stored` |
| Waiter notified when ready | Poster | already: the socket carries `Advanced`; the room app should toast READY (a row for the room, not the kitchen) |
| Dark mode "at bars, reduce eye strain"; bump bars; grid vs dynamic layout | Toast new KDS | dark default; grid layout with a fixed max tickets per screen; bump bar = keyboard keys 1–9 (free) |
| Preview tickets ("fire on next") | Toast | no: dowiz's round is placed whole; a preview would leak the waiter's unsent basket |
| Chef performance reports | Poster | **no** (F36 no-scoring) |
| Every disposal weighed, photographed, timestamped; manual logs miss > 30 % | Winnow https://www.winnowsolutions.com/ ; Leanpath https://www.leanpath.com/ ; comparison https://orbisk.com/blog/food-waste-solutions-comparisons/ | photo optional on waste; the honest fix for under-logging is the yield test + stocktake delta, not a camera over the bin |
| Price points | Poster $7/screen/mo; Fresh $16–27/screen/mo; Lightspeed KDS $30/screen (`BLUEPRINT-PLATFORM-TOP-TIER:504`) | a KDS is a paid add-on everywhere; dowiz ships it inside the hub |

UX for a wet, greasy pass (own synthesis of the above + `BLUEPRINT-LAST-MILE:335-336` "a KDS on the
pass is not held; it is a screen a cook bumps with a knuckle"): one action per ticket, full-width,
≥ 88 px tall (double the F37 token); no swipe gestures (a wet swipe is a tap); no confirm dialogs —
UNDO for 8 s instead (NN/g's rule the last-mile blueprint already adopts); 22–28 px item text readable
from a metre; sound on new ticket and on a ticket turning red; the age counter ticks locally, never
asks the server; the screen never blanks (replica + shell) and says how old it is; three languages
switchable from the HUD like the room (`room/index.html:26`); Albanian is the default at the venue,
Ukrainian for the operator, English for the rest.

## 7. Roadmap — small shippable rows

Each row: files, proving test, gates, risk. **[OP]** = needs an operator decision first. Box rule: one
lane at a time on the heavy (cargo) rows.

| # | Row | Files touched | Proving test (RED first) | Gates | Risk |
|---|---|---|---|---|---|
| K0 | Register `/kitchen/` in every gate before the first commit: `design_gate.py` SURFACES (+ add the missing `room`), `ui-adoption.sh`, `tap-size.sh`, `sw-shell.sh`, `learn.sh` anchors file | `scripts/design_gate.py`, `tools/gates/*.sh`, `docs/learn/anchors-kitchen.txt` | each gate goes RED on an empty `kitchen/index.html` for the right reason, then GREEN | all | none; the room's absence from the design gate is a finding on its own |
| K1 | **Kitchen read route** `GET /api/staff/kitchen?location_id=&since=` — `staff_at(Cap::Advance)`; returns open orders at `PENDING..READY`, lines `{name, quantity, modifier_ids, station}`, `fulfilment.{kind, table, note}`, `kitchen.{seen,printed}`, ETA stamps, `created_at`; STRIPS `contact`, `fulfilment.address`, prices, payments | new `workers/api/src/services/orders/kitchen_view.rs` (+ pure `command/kitchen_view.rs` projection), `lib.rs` route, `mcp/tools.rs` (`kitchen` tool for `advance` keys) | pure: the projection of an order with a phone and an address contains neither string; route: a waiter token is 403, another venue's kitchen 404; `?since` returns the delta | principal-binds, one-venue, personal-data (new recipient row), vocabulary, coverage | low; pure fold over `hubstore::orders` (the same memoised projection `room_view` reads) |
| K2 | **Socket tag for the kitchen**: `TAG_KITCHEN`; the object sends the STRIPPED delta to kitchen sockets (`broadcast` gains a second, projected payload), `live.rs:99-107` picks the tag by cap | `live.rs`, `hubdo.rs:450-500`, `hubdo.rs:134-152` | an `advance`-only token's socket never receives a frame containing `"phone"`; a `take_orders` token still gets the console tag | personal-data, coverage | medium: `broadcast` is shared by every event; keep the projection pure and tested |
| K3 | **`/kitchen/` surface v1 — the board** (§3.1): login (device account), board, START/READY/HANDED OVER, seen-on-display, ring, age colours, dark default, sq/en/uk, replica + shell + outbox, poll fallback | `public/kitchen/{index.html, app.js, board.js, logic.js, net.js, i18n.js, kitchen.css, sw.js}` (copy the room's module split), `docs/learn/anchors-kitchen.txt` | `logic.test.mjs`: `ageClass(created, now, warn, late)`, `bumpFor(status, kind)` gives `preparing|ready|collected` and never `reject`; `css.test.mjs` tap ≥ 88 px; a snapshot of the board with 0/1/40 tickets | design-gate, ui-adoption, tap-size, sw-shell, vocabulary, learn (anchors present, lessons pending), coverage | medium: the biggest row; split board/logic into ≤ 300-line files |
| K4 | Lessons K1–K4 (first shift, the board, colours and undo, stop list) in sq/en/uk + tour anchors | `docs/learn/lessons/kitchen/K*.yaml`, `tools/learn/build-lessons.mjs` inputs | `learn.sh` GREEN with every anchor present; recorder produces the clips (R2 `dowiz-learn`) | learn | low |
| K5 | **`stock_move` capability**: `Cap::StockMove` (0x20), `Preset::Kitchen` and `CounterManager` gain it; `stock.rs::signer_for` accepts `StockMove` for all three kinds (waste: `OpenTill OR StockMove`); `GET /api/owner/stock` accepts `staff_at(StockMove)` | `crates/dowiz-hub/src/caps.rs` (HEAD version — coordinate with the split), `auth.rs` tests, `services/operations/stock.rs`, `mcp/tools.rs` | caps: `"advance,stock_move"` at `kitchen` keeps both, `"stock_move"` at `waiter` is refused; route: a kitchen token records `received` and `stocktake`, `by` = its person id | principal-binds, one-venue, coverage; `no-sql` untouched | low in code, **[OP]** Q3 on PIN attribution before shipping receive/count |
| K6 | **Stock screens** (§3.3–3.5) in `/kitchen/`: receive (with temperature field for chilled supplies), waste (reason chips, optional photo), count (delta live) | `public/kitchen/{stock.js, receive.js, waste.js, count.js}`; photo → existing media upload path **(unverified: whether `services/catalogue/media.rs` can store a non-product photo; if not, a small `stock_photo` R2 key)** | `stock.test.mjs`: a waste with no reason is not sent; temp > 5 °C flags; a count leaves uncounted lines out of the POST | ui-adoption, tap-size, personal-data (photo may show a person — registry row), coverage | medium |
| K7 | `Received` gains a signer (`by`) and an optional `temp_c`, `supplier`, `doc` — **model row, other researcher's file**; the KDS sends them | `crates/dowiz-hub/src/stock.rs` encode/decode (+ old records still decode) | fold of an old `Received` record without `by` still reads | event-kinds, record, coverage | shared with the inventory lane; sequence after their model decision |
| K8 | **Yield test** (§3.8): new stock event (AP in, EP out, cooked out, trim lines) + `POST /api/staff/stock/yield` (`StockMove`) + the screen with the number pad; the recipe's `expected_yield` is READ here and never written | model: other researcher; here: `public/kitchen/yield.js`, `services/operations/yield.rs` | pure: `yield_pct(4200, 2940) = 70`; a test with EP > AP is refused; the route refuses a body that carries `expected_yield` (`deny_unknown_fields`) | coverage, principal-binds, float-money (percent as integer ‰, like `catalogue 844‰`) | **[OP]** Q6 per batch vs per session confirmed; low code risk |
| K9 | **Yield proposal → owner approval**: a `yield_proposal` record (venue image) written by the kitchen when the measured yield differs from expected by > N %; the console's supplies sheet lists proposals with accept/decline; accept = the owner's existing supply edit | `services/operations/yield.rs`, `public/admin/stock.js` or `supplies.js`, `privacy/registry/venue.rs` (proposal names a person) | a proposal cannot change `expected_yield` by itself (test the fold); accept writes the supply with the owner's `by` | record, personal-data, coverage | low |
| K10 | **`stop_list` capability + 86 screen** (§3.7): `Cap::StopList`; `POST /api/staff/products/:id/available` (only the flag + note, never price); storefront propagation checked live | `caps.rs`, new route in `services/catalogue/`, `public/kitchen/stoplist.js`, `admin/menu.js` shows "86'd by kitchen 20:14" | route: the body `{available:false, note}` with a `price` field is 400; a kitchen token flips it; store's next menu read hides the dish (live check, not a unit test) | principal-binds, vocabulary, coverage; watch the edge-cache `?fresh=1` trap (memory) | **[OP]** Q2: direct 86 or propose-only |
| K11 | FT6 ping auto-response (`set_websocket_auto_response`) — prerequisite for two more always-on sockets | `hubdo.rs accept()` | measured DO `hibernation` invocations/day drop for an idle open socket | — | low; already specified in FREE-TIER FT6 |
| K12 | Kitchen device sessions: `KITCHEN_DEVICE_TTL_MS` (7 d) for the `kitchen` preset; owner can end a device from the roster sheet (session list per person) | `services/identity/staff_rules.rs`, `staff.rs`, `staff_admin.rs`, `admin/staff.js` | a kitchen token's `exp − iat` is 7 d, a waiter's 12 h; ending the session makes the next `kitchen-ack` 401 | coverage | **[OP]** Q3 |
| K13 | Who-tapped PIN for stock writes (`by_pin` checked against a digest on the membership row) | `staff_rules.rs`, `stock.rs` routes, `public/kitchen/pin.js` | a wrong PIN is refused with constant work (`verify_opaque`); the event's `by` is the PIN's person, not the device's | personal-data (PIN digest = credential row), coverage | **[OP]** Q3 |
| K14 | Prep list v1 (§3.6): all-day ingredient counts from open tickets + bookings' expected covers; forecast from the last 4 same weekdays (a fold, no stored counter) | `command/prep.rs` (pure), `services/operations/prep.rs`, `public/kitchen/prep.js` | pure: 3 open tickets × BOM = the expected grams; bookings add covers × average basket | coverage, no-scoring | medium; depends on the prepared-item model for "rice cooked" |
| K15 | Room app: toast + sound when a round the waiter placed turns READY (the socket already carries it) | `public/room/app.js`, `i18n.js` | `logic.test.mjs`: the READY transition of "my" round produces a notice | ui-adoption, coverage | low; a waiter-side row that completes the kitchen loop |

Suggested order: K0 → K1 → K3 (a usable board in three rows; K2 can follow because K1's poll path
already strips PII — until K2 lands the KDS should NOT open the socket, since the console tag leaks
the address) → K11 → K2 → K4 → K5 → K6 → K7/K8/K9 (with the inventory lane) → K10 → K12/K13 → K14 →
K15.

## 8. Open questions for the operator

- **Q1 — May the kitchen reject/cancel?** Today `advance` allows `reject` and `cancel` from PENDING
  (`owner.rs:561,571`). Proposed: hide both on the KDS; only the owner/console rejects (the customer
  gets the reason). Keeping the cap as-is costs nothing; the UI decides.
- **Q2 — 86 from the line: direct or proposed?** Direct is what every KDS sells and what stops
  oversell; "propose" keeps the menu the owner's. Recommendation: direct for `available=false` with
  a mandatory note, owner-only to put it back on sale — i.e. the kitchen can only close, never open.
- **Q3 — Device model.** One shared tablet per station (device account, 7-day session, 4-digit PIN for
  stock writes), or personal logins on phones? Recommendation: shared tablet + PIN.
- **Q4 — "Seen" = displayed or tapped?** Auto-ack when the ticket is drawn on a live screen makes the
  health pane honest about the WIFI; a tap makes it honest about the COOK. Recommendation: displayed
  (the tap is START anyway).
- **Q5 — Supplier and invoice on a receive?** Free-text supplier + optional invoice photo now; a
  supplier list is a model question (other researcher).
- **Q6 — Yield per batch (per fish / per lot) or per prep session?** Recommendation: per batch for
  fish (butchery test), per session for cooked outputs (rice, tamago); the same screen. Add a
  `Trim` waste reason, or book trim to a secondary supply ("salmon skin")?
- **Q7 — Should the kitchen CONFIRM incoming orders**, or does the owner's phone keep that step? The
  cap allows it; a venue setting `kds.confirms` would hide/show the button.
- **Q8 — Which station names?** `kitchen | bar` exist. Sushi bar vs hot kitchen vs bar (drinks) would
  be three; `Station::from_wire` is a closed set (`bell_route.rs:50-56`), so adding one is a small,
  explicit change — but each station is a tablet.
- **Q9 — Photos on waste**: wanted (Winnow-style evidence) or noise? Costs a KV/R2 write per photo
  (KV writes are the tightest Free cap at 1 000/day, `FREE-TIER:114`; R2 is the right store).

## 9. Things this note could not verify

- Whether the print rail resolves modifier NAMES for the ticket text (needed for the KDS lines) — not
  read; `storefront.rs:906` shows only `modifier_ids` on the line.
- Whether the media path can store a non-product photo (K6).
- The EU 853/2004 freezing rule as applied in Albania (cited from general knowledge, not fetched).
- Poster's Ukrainian "кухонний екран" page specifics beyond the English tour page (the search returned
  the RU/KG page; not scraped).
- Whether any BLE scale at the venue exposes a readable GATT weight characteristic.
