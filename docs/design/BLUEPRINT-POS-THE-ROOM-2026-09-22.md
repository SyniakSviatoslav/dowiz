# POS, the room: a waiter, an open check, a bill that can be amended and split, a till that is audited, and what to refuse

**Date:** 2026-09-22. **HEAD read:** `b949528e` ("dine-in: an order placed at a table", 2026-09-22).
**Tree state at time of reading:** 9 paths in `git status --short` (7 modified, 613 insertions / 28 deletions;
`crates/dowiz-hub/src/tables.rs` and `workers/api/src/ebills/` untracked). Those belong to the table-booking
and ebills lanes and were read but not cited for line numbers except where marked *(lane, uncommitted)*.
`crates/dowiz-hub/src/lib.rs` is modified by the booking lane; its diff is one line (`pub mod tables;`), so
the `EventKind` citations below are from HEAD and hold in the working tree.

**Measured on this box, 2026-09-22:** `cd workers/api && cargo test --lib --offline` → **228 passed** in 2.53 s;
`cd crates/dowiz-hub && cargo test --offline` → **337 passed** (lib) in 3.37 s. Both exit 0.

**Method.** Every "today" statement names a path and a line. `measured` means a command was run here and its
output quoted; `hypothesis` means it was not. Costs are hypotheses unless a number is cited. Where the brief's
list of facts and the tree disagree, §1.7 says so. Sibling documents: `BLUEPRINT-ARCHITECTURE-EVOLUTION-2026-09-22.md`
(whose P1–P5 have since landed: `9a0f73c7`, `ae6406f0`, `2b057edb`, `d7b97ed7`, `4a7b32c0`, `f4eb7d86`,
`cb4ef70b`), `BLUEPRINT-EBILLS-INTEGRATION-2026-09-22.md` (the venue's real till, measured from the outside), and
`ROADMAP-2026-09-22.md` items A4–A8, which this document answers, and whose §4 asks it to re-examine the
offline-till CRDT re-entry "rather than inherit the answer".

**The brief translated.** dowiz takes orders in the room now (`b949528e`) and has none of the six things a room
needs: a till, a waiter, an amendable order, an open tab, a void with a name on it, print routing. The question
is what closes each, in dowiz's own terms — an append-only chained log folded into orders, commands executed by
one object in one turn, integer money, capabilities not scores — and what to refuse. §2 is the model; §3 answers
`Amended`; §4 is the research and the gamedev seams, including where they do not hold; §5 the refusals; §6 the
gates; §7 the order of work; §8 what the tree could not tell.

---

## 1. Ground truth

### 1.1 The six facts, checked

| # | The brief says | Verified | Nuance the design uses |
|---|---|---|---|
| 1 | No till as a shift; a courier has one | **true.** `courier.rs:155-214` (`POST /api/courier/shift`), record under `K_SHIFT = "shift"` (`courier.rs:230`) keyed by the courier's own id so "two open shifts is not a state this layout can represent" (`courier.rs:176-179`); `deliveries` and `cash_collected` bumped at deliver (`courier.rs:598-605`). Nothing for a waiter or cashier: `grep -rn 'till\b\|cashier\|drawer' workers/api/src` → no handler (measured) | The shift is NOT the source of the numbers. Earnings are "FOLDED FROM THE ORDERS, not from a shifts table. The table was a second place the same numbers lived" (`courier.rs:687-690`), tips "kept APART from the float" (`courier.rs:709-711`). A till must copy that discipline, not the record (§2.5) |
| 2 | No waiter role | **true.** `auth.rs:338-348`: `Principal` is `Owner | Courier | Customer`. Below it the hub token's `Role` is `Owner | Courier | Customer | Refresh` (`crates/dowiz-hub/src/token.rs:23-32`); the roster's `Person { id, role, name, active }` (`roster.rs:40-47`) carries that role | The whole session machinery a waiter needs exists for the courier: invite codes (`roster.rs:308,371`), sessions (`roster.rs:426`), and the live check at token time — revoked, expired, "no longer at this location" (`auth.rs:500-522`). `DECISIONS.md:297-298` already rules a staff vocabulary: "Owner / Kitchen / Counter-Manager" |
| 3 | An order cannot be amended; `EventKind` has no `Amended` | **true.** `crates/dowiz-hub/src/lib.rs:62-97`: `Placed=1, Advanced=2, Paid=3, Revealed=4, Noted=5, Checkpoint=6`; `from_byte` refuses anything else (`lib.rs:117-125`) and an unreadable record is **quarantined**, which is conservation law 6 (`lib.rs:475-487`; `conservation.mjs:129-152`) | Every mutating event is already a **delta** (`fold.rs:1-45`, `DELTA_MARK = "_d"`, deletions in `"_x"`), including `Paid` (`stripe.rs:291`). The fold replaces any field a delta names, `items` included; the browser folds with the same `merge` (`replica.js:100-118`). Mechanically, an amendment is a delta the log can already carry. What is missing is a KIND that says money changed (§3) |
| 4 | No open tab, split bill, transfer | **true** for the first and third. For the second, note: `Paid` "is its own fact… an order can be paid while still PENDING" (`lib.rs:67-71`); nothing in the hub refuses a second `Paid` on one order — the only guard is Stripe's `payment_status == "paid"` early return (`stripe.rs:279-283`). So partial payment is neither modelled nor refused today | §2.4 makes N payments per check the model and adds the refusal (Σ ≤ total) |
| 5 | No void with a reason and a person | **true** for orders. Precedents exist in the same tree: `rejection_reason` recorded with the `REJECTED` edge (`advance.rs:99-101`); `StockEvent::Wasted { reason: WasteReason }` with a closed reason set `Spoiled | Dropped | Unsold` (`stock.rs:29-34, 65-66`) | A void is an amendment with a reason and a signer (§2.6); the reason set is closed the way `WasteReason` is |
| 6 | No print routing, no tip distribution | **true.** One channel: "a Telegram bot the VENUE owns" (`notify.rs:3`), one chat (`settings` key `notify.telegram.chat`, `notify.rs:196`), text rendered at enqueue into `outbox::Entry { id, kind, text, to }` (`outbox.rs:64-76`), queued by the object in the placing turn (`hubdo.rs:928`, `enqueue_bell`). Tip: "THE TIP IS THE COURIER'S, passing through" (`status.rs:63-73`, `venue_took = total − tip`) | Routing is a second `Entry` with a different `to` and a subset of the lines (§2.7). A dine-in tip belongs to nobody yet; §5 refuses to decide whose |

### 1.2 What the venue does today, measured from outside (the ebills blueprint)

The real till is on ebills.al, and `BLUEPRINT-EBILLS-INTEGRATION-2026-09-22.md` measured it read-only:

- **A course is fiscalised the moment it is rung up**, as its own sale (`summaryInvoice:false` + `saleUnitOrder`);
  the bill that closes the table is a **second** fiscalised sale whose lines are the union of the courses
  ("260 + 4600 = 4860, 2 + 2 = 4 lines", §1.6). The ebills lane therefore maps a course to an **order** and a bill
  to a **`Paid` fact, NOT an order** (`workers/api/src/ebills/mod.rs:167-171` *(lane, uncommitted)*).
- **The table is the live unit**: 18 tables, `status: ACTIVE | OCCUPIED`, an unpaid `orderTotal`, and the
  server's name on the table (§1.6). 452 bills in 30 days ≈ 15 sittings a day (§1.6; arithmetic mine).
- **A shift refuses to close over an unpaid table** (`{"errorKey":"unpaidSales"}`, §2), and the day opens with a
  `tcr-cash-balances` record of type `INITIALIZE` (§2) — a float. That is a till with an open/close and a rule.
- A cancellation is a **fiscal act** (`PUT /api/sales-cancel/{id}`, §5.3), and "the wire shape of a cancelled
  sale was not observed".

So the venue's own practice already answers the check-lifecycle question: **round = order, sitting = check,
bill = payment fact**, and the fiscal receipt is the point past which nothing is rolled back (§4.5).

### 1.3 The machinery a room would reuse

- **Commands in the object.** `command/mod.rs:1-33`: "the object holds every image in `mem`. It can therefore do
  all the fallible work against copies IN MEMORY and write only once everything has succeeded, so there is no
  state to undo." Three exist — `place`, `advance`, `assign` (`hubdo.rs:1258-1282`) — each a pure `decide` over
  `dowiz_hub` types with 14 native tests (`command/tests.rs`, measured `grep -c '#\[test\]'`). A command
  carries **no generation header** because "the read and the write are two statements inside one object turn"
  (`mod.rs:87-92`). `Refused` names its status: `Stock 409, Promo 400, NotFound 404, Conflict 409, Append 500`
  (`mod.rs:49-70`).
- **The kernel decides status, the Worker owns money.** `apply_event_logic` recomputes `subtotal` from `items`
  and sets `total = subtotal` on every transition (`json_api.rs:108-117`, the forged-total tamper test at
  `362-383`), and then `hubstore::carry_over` copies the OLD `items`, `total`, `tip`, `discount`, `promo`,
  `delivery_fee` back over it (`hubstore.rs:1350-1376`, `HUB_OWNED`) — with the comment "the kernel does not
  change `items` on a status transition". So for a stored envelope the money authority is the Worker's one
  pricer, `services::ordering::pricing::price_basket` (`pricing.rs:100-103`), plus the `total = subtotal − cut +
  fee + tip` line in `place::decide` (`place.rs:149`). **An amendment must go through `price_basket` or it is the
  third pricer** (`a18886d4` was the second).
- **Stock is per order id and settles by status.** `reservations_for(order_id, bom_lines)` at placement
  (`stock.rs:985`, `place.rs:117-120`); `settle(ledger, order_id, consume)` at `PREPARING` (consume) or
  `REJECTED | CANCELLED` (release) and nowhere else, "because by then the consumption already happened"
  (`advance.rs:56-62`). The ledger's own invariants are named I1–I4 (`stock.rs:95,102,159,333`), and `Stocktake`
  "resets the basis (I2): everything before this count stops contributing to on_hand. Reservations survive"
  (`stock.rs:332-337`). That is the shape of a till count.
- **The client is already a predicting replica with an outbox.** `replica.js:14-18`: "a PREDICTION in exactly
  the sense game netcode means"; socket events are applied by kind (`ORDER_KINDS = new Set([1, 2, 3, 5])`,
  `replica.js:62` — a **hand copy** of `EventKind::is_order`, `lib.rs:102-108`; measured: the only one,
  `grep -rn '1, 2, 3, 5' workers/api/public tools`). The courier's `tapped()` mints an `idempotency-key` at tap
  time and queues on `offline` (`courier/app.js:337-345`, `outbox.js:1-30`); `idempotent.sh` pairs the client's
  queued verbs with server guards and says plainly it covers `courier.*` only (`idempotent.sh:24-27`).
- **The audit.** `e2e/gates/conservation.mjs` runs eight laws against the live venues and `conservation.prove.mjs`
  proves each fires red and stays green against a stubbed platform (`e2e/gates/conservation.prove.mjs:1-17`; in CI at `ci.yml:86`).
  Law 8 refolds every projection from the bytes and crosses the log with the ledger (`rebuild.rs:1-22`).
- **Tables as furniture** are the booking lane's: `dowiz_hub::tables` — zones, `MAX_TABLES = 200`, `DWELL_MIN = 90`,
  "`occ` IS NOT A PROPERTY OF A TABLE" (`crates/dowiz-hub/src/tables.rs:1-17, 24-38` *(lane, uncommitted)*); a
  booking names `(zone_id, table_n)` (`booking.rs:351-356, 383-384`). An order names its table as a free string,
  `fulfilment.table` (`storefront.rs:98, 1028`), which `fulfilment::needs("dine_in") == Needs::Table` makes
  mandatory (`services/ordering/fulfilment.rs:38, 52-58`). §8 names the spelling gap.

### 1.4 What the gates look like, so the new ones look the same

Eleven gates run in CI (`ci.yml:40-86`): `no-sql`, `file-size`, `one-venue`, `one-image`, `clock`, `vocabulary`,
`vocab`, `paths`, `idempotent`, `unreached`, and the conservation proof. Baselines (measured, `cat
tools/gates/*.baseline`): `clock 0`, `file-size over=40 worst=1737`, `no-sql 0`, `one-image bodies=0`,
`one-venue handlers=0`, `paths 0`, `unreached 4`. The shape is one script per rule that counts a thing, refuses
if the count rose, and refuses if it fell without the baseline being lowered in the same commit
(`one-venue.sh:66-75`). `unreached.py` exists for exactly the failure this document must not add: "a capability
that EXISTS, COMPILES and IS TESTED… and no live path reaches it" (`unreached.py:12-16`).

### 1.5 The kitchen's status is per ticket, and that decides the model

`allowed_next` (`order_machine.rs:112-128`): `Pending → {Confirmed, Rejected, Cancelled}`, `Confirmed →
{Preparing, InDelivery, Refunding}`, `Preparing → {Ready, Refunding}`, `Ready → {InDelivery, PickedUp, Refunding}`,
`InDelivery → {Delivered, Refunding}`, `Refunding → {CompensatedRefund}`; six terminals. The owner's intents map
one-to-one onto edges (`owner.rs:561-574`: confirm, reject, preparing, ready, collected, cancel). `CANCELLED` is
reachable from `PENDING` only — an order the kitchen has accepted cannot be ended except through `Refunding`
(memory `dowiz-order-fsm-has-no-exit`; the audit's law 2 "measures the size of that hole", `conservation.mjs:78-87`).

`status` is one field on one order. A table that has ordered a second round while the first is cooking is, in
kitchen terms, one ticket at `PREPARING` and one at `PENDING`. **One order cannot be both.** Every POS that keeps
one growing order per table (Square's order with `version`, Odoo's one order per table, Loyverse's open ticket)
keeps kitchen state somewhere else — per "send" or per line. dowiz keeps it on the order and pins the graph to
a golden signature (`FSM_GOLDEN_SIGNATURE`, `order_machine.rs:547-558`: 12 vertices, 14 edges, acyclic, μ = 4,
ρ = 0, 11 reachable). Moving the kitchen state off the order to make one order grow would be a rewrite of the
thing the repo is most careful about. So the model follows the venue's own practice (§1.2) instead.

### 1.6 The golden signature: what moves it and what does not

`fsm_graph_report()` is `{vertices, edges, is_acyclic, cyclomatic, spectral_radius, reachable_from_pending,
reachable_states, topological_len}` (`order_machine.rs:454-486`); `apply_event` re-verifies it on **every fold**
and refuses with `Invalid` if it drifted (`domain.rs:344-349`). A new status or a new edge moves `vertices` or
`edges` and therefore `cyclomatic`, `reachable_*`, `topological_len`; a `Reopen` edge (`PickedUp → Pending`, say)
moves `is_acyclic` and `spectral_radius`. The cost of any of those is a re-key with a recorded rationale
(`order_machine.rs:539-546`) plus the four browser vocabularies that `vocab.sh` now generates (`b7019114`).
**Nothing in this document adds a status or an edge.** An `EventKind` is not a vertex; the signature is about
`allowed_next`, and `Amended` never calls it.

### 1.7 Where the tree disagrees with its documents, and one law that is blind

| Claim | Tree | Evidence |
|---|---|---|
| Conservation law 3: "the money on an order is its lines plus fees minus its discount" | the envelope's total is `subtotal + fee + tip` (`storefront.rs:1015-1019`) and `subtotal − cut + fee + tip` after a promo (`place.rs:149`); the gate computes `lines + fee − discount` with **no tip term** (`conservation.mjs:59-70`) | a tipped order breaches law 3 by exactly its tip, OR no live order has ever carried a tip. Which, the tree cannot say (§8). Either way the law is wrong as written and is item 0 of §7 |
| `ARCHITECTURE-EVOLUTION` §1.4: idempotency on one route | `idempotency::guard` on `courier.*` and `owner.order_action` (`d7b97ed7`; `idempotent.sh:24-27`) | landed since |
| `ARCHITECTURE-EVOLUTION` §1.5: no write queue on any surface | `public/lib/outbox.js` (`2b057edb`) | landed since |
| Brief, fact 4: "no split bill" | true as a feature; but the hub already admits N `Paid` events per order with no sum check (§1.1 row 4) | the refusal is the missing half, not the event |
| `ROADMAP-2026-09-22.md:97`: "the open tab, and `Amended` as an event kind" | the tab is not an event kind; §2.2 and §3 argue it is an id on the envelope plus one new kind for money-changing deltas | this document |

---

## 2. The model

### 2.1 Vocabulary, fixed once

| Word | Is | Is not |
|---|---|---|
| **Round** | one `Placed` order: what the kitchen makes as one ticket, with its own FSM status | a line |
| **Sitting** (the check, the tab) | every round sharing one `sitting_id`, from the table being opened to the bill being settled | an order; an aggregate with its own image |
| **Table** | `fulfilment.table` on each round; the sitting's current table | a state holder — "occ is not a property of a table" (`tables.rs:9-12`) |
| **Bill** | the sitting's total: Σ(round totals after amendment) − Σ(voids) − Σ(comps); paid by one or more `Paid` facts until Σ amounts == bill | a second order (the ebills lane's rule, `ebills/mod.rs:169`) |
| **Till** (drawer) | a period opened with a float by a person, into which cash payments fall, and closed with a count | a person's shift; a second copy of the cash numbers |
| **Amendment** | a delta on a round that changes its `items` and therefore its money, signed by a person, before the kitchen has it | a status transition; a rollback |
| **Void** | an amendment that removes a line, with a closed reason and a signer, allowed after the kitchen has it only with a capability | a `CANCELLED` status |
| **Comp** | a discount on a line the guest received, with a reason and a signer | a void |

### 2.2 The sitting: an id on the envelope, not a new aggregate

A `sitting_id` (a ULID minted by the Worker when a table is opened, like `order_id`, `lib.rs:9-11` "identity and
the clock come from the EDGE") is written into every round's envelope beside `fulfilment.table`. The sitting is
then a **projection**: `orders_state(hub)` grouped by `sitting_id` (`hubstore.rs:1268-1280`, one pass, already
memoised per generation in `orders_view`, `hubdo.rs:423-438`). It costs no image, no new fold rule, and no new
event: opening a table is placing its first round; the table is occupied while any round of its newest sitting
is not terminal or unpaid; closing is the `Paid` that brings Σ payments to the bill.

Why not a `sitting` image or a `Sitting` event: every projection is "rebuilt from the log, never a second source
of truth" (`lib.rs:10-14, 19-21`), and law 8 checks exactly that (`rebuild.rs`). A second image for the check
would be the class `253e1ece` closed — an aggregate whose status can disagree with its own history — reopened
for the room. The ebills lane reached the same shape from the other side: a bill's courses are joined "by table
+ window" because that is what the wire gives (`EBILLS` §1.6); a `sitting_id` is that join made explicit.

**What the kitchen sees:** exactly what it sees today. Each round is a ticket with a status the owner advances
(`owner.rs:561-574`) and a bell queued in the placing turn (`hubdo.rs:928`), whose first line already says
"🍽 table 7" (`notify.rs:121-125`, `b949528e`). A second round is a second bell for the same table. No change.

### 2.3 The round: `Placed`, then `Amended` until the kitchen has it, then status only

- **Placement** is `command::place` with `fulfilment.kind = "dine_in"`, `fulfilment.table`, `sitting_id`, by a
  staff principal (§2.8) or, later, by a customer through A9's QR. The stock reservation is the same
  `reservations_for` on the same ledger — the shared shelf `b949528e` proved by test.
- **Amendment** is a new command, `amend` (§3), legal while `status ∈ {PENDING, CONFIRMED}`: the Worker re-prices
  the new line set with `price_basket` and the fee/tip/promo rules it already applies at placement, sends
  `AmendIn { order_id, location_id, base_seq, items, subtotal, discount, total, by, reason?, now_ms }`, and the
  object's `decide` (a) refuses if the order's newest event `seq` ≠ `base_seq` (§4.4), (b) refuses if the status
  is past `CONFIRMED`, (c) releases the old reservations and reserves the new set against the ledger in memory
  (`Released` for lines that shrank, `Reserved` for lines that grew — I3 holds because it is one order id), (d)
  appends `Amended` as a delta of `{items, subtotal, discount, total, amended: [{by, reason, at}]}`, and (e)
  broadcasts. Nothing is written until (a)–(d) succeed — the `place` pattern (`hubdo.rs:604-617`).
- **After `PREPARING`** an order's lines are what was cooked. A guest who changes their mind then is a **new
  round** (a `Placed`) and, if the kitchen agrees to drop the first, a **void** on the first (§2.6). Never an
  `Amended` without the `void` capability, because the ledger has already `Consumed` (`advance.rs:56-62`) and
  the audit's law 8 would otherwise see stock consumed for lines the order no longer has.

### 2.4 Paying the bill: N `Paid` facts, one conservation rule

`Paid` already exists as "money arrived… its own fact" (`lib.rs:67-71`) and is written today only by the Stripe
webhook (`stripe.rs:270-292`, the one site, measured `grep -rn 'EventKind::Paid' workers/api/src`). Cash is
recorded on the courier's `deliver` as `cash_collected` (`courier.rs:566-593`) with no `Paid` at all. The room
needs a `pay` command that a person executes:

`PayIn { sitting_id, amount, method: cash | card | …, by, till_id?, covers?: [order_id…], fiscal?: {…}, now_ms }`
→ the object folds the sitting's bill and the payments so far, **refuses if `paid + amount > bill`** (the missing
half of §1.1 row 4), appends one `Paid` per round the payment covers (the payment split across rounds in
proportion, remainder to the last, integer division stated as an equation in the code), and, when
`paid == bill`, marks each round `payment_status: paid` — the field Stripe already sets (`stripe.rs:286`).
`took_money` and `venue_took` (`status.rs:50-73`) keep working unchanged because they read status and `tip`.

**Split by seat, by item, evenly, by amount — which are different data models.** Only one is: none of them.
Every split is `pay` called N times with N amounts; "by item" and "by seat" are the client choosing the amount
from lines it labels (a seat is a per-line label the client keeps, `covers` is advisory), "evenly" is
`bill div N` with the remainder on the last payer, "by amount" is the guest's number. What IS a different model
is **moving lines to another check** — Toast's "split checks" creates checks; Loyverse's "split ticket separates
its items into multiple new tickets" — and that is a transfer (§2.9), not a payment. Keeping the two apart is
what makes the conservation rule one line: **Σ `Paid.amount` over a sitting == the sitting's bill, at close, and
never more**. Loyverse's own known defect — a payment split between cash and card counted wholly as cash, so
"the Expected cash amount [is] not balanced when closing shift" — is a method-attribution bug that one law over
`Paid.method` catches (§6, G5).

### 2.5 The till: a LogImage of what the order log cannot know, and a fold across the two

The courier shift's lesson (`courier.rs:687-690`) is that the numbers live in the orders and the shift record is
"a second place the same numbers lived" that drifted. A till that copied the shift's counters would be that
second copy again — and the operator's brief says so: "should not become a second one". So:

- **Cash sales are never written to the till.** They are folded from the order log: `Paid { method: cash,
  amount, till_id, at }` per round, plus the courier's `cash_collected` handed in (below).
- **The till log holds only what the order log cannot know.** A `dowiz_hub::logimage::LogImage` named `till`
  (the append-only shape for "things that are not orders", `logimage.rs:1-16`) with a closed event set:
  `Opened { till_id, float, by, at }`, `PayIn { amount, reason, by, source?: shift:<courier_id>, at }`,
  `PayOut { amount, reason, by, at }`, `Counted { observed, by, at }`, `Closed { by, at }`. One open till per
  venue at a time (a second `Opened` while one is open is refused — the shift's "not a state this layout can
  represent", `courier.rs:176-179`); a per-register `till_id` is written from day one so a second drawer costs
  nothing later.
- **Expected cash** = `float + Σ cash Paid in [opened, closed] + Σ PayIn − Σ PayOut − Σ cash refunds` — a pure
  function over the two images, in the object, in `rebuild`'s neighbourhood (`rebuild.rs:1-22`). **X report** =
  that fold at any moment (a `Counted` without a `Closed` is a blind count against it — the app shows the
  expected figure only AFTER the count is entered, which is what "blind" means and what Odoo's "theoretical
  balance… then the real closing balance… and the difference" does after the fact). **Z report** = `Counted` +
  `Closed`; `over_short = observed − expected` is recorded, never adjusted away, and the next `Opened` float is
  a new basis — the `Stocktake` rule, I2 (`stock.rs:332-337`): the count resets on-hand, and open promises
  (unpaid tables) survive it.
- **The courier's drawer empties into the till.** At the end of a courier shift the `cash_collected` on it
  (`courier.rs:602-603`) is handed over as a `PayIn { source: shift:<courier_id>, amount }`; the audit checks the
  hand-in equals the shift's figure (§6, law 9c). Two drawers, one shape, one law; the courier shift is not
  changed.
- **Tips stay out of the drawer**, as the courier's already do (`courier.rs:709-711`): a cash tip at the table is
  the room's, not the float's, and `expected` excludes it.
- **A cash `Paid` outside any open till is a breach**, not a default — ebills' "cannot close over unpaid sales"
  in the direction dowiz can enforce: it can refuse the payment (409 "open the till first") and the audit names
  any that got through.

### 2.6 Void and comp: the record outranks the permission

Toast's vocabulary is the right one and is adopted: a **void** removes an item "the guest did not receive"; a
**comp** "waives the charge for an item the guest received but is not paying for". In dowiz:

- **Void before the kitchen** (`PENDING | CONFIRMED`) = `Amended` removing the line, stock `Released`, `reason`
  from a closed set (`Mistake | GuestChanged | Unavailable | Dropped | Other(text)` — the `WasteReason` pattern,
  `stock.rs:29-34`), `by` = the signer's person id. Any staff with `take_orders` may.
- **Void after the kitchen** (`PREPARING | READY`) = `Amended` removing the line, **no stock event** (the
  ingredients were `Consumed` at `PREPARING` and are gone whether the plate was dropped or refused; the reason
  says which), `reason` mandatory, and the command refused unless the signer holds `void` (§2.8). The line's
  money leaves the bill; law 3 holds because `items` and `total` moved together in one delta.
- **Comp** = the line stays; `discount` grows by the line's amount; `adjustments: [{ kind: "comp", line, amount,
  reason, by, at }]` is appended to the envelope in the same `Amended`. Law 3 keeps its one `discount` term;
  the audit trail is the `adjustments` list, in the order log, tamper-evident by the chain (`lib.rs:384-385`).
- **Why the record matters more than the permission.** A permission decides who may; a record decides what an
  owner can find out afterwards. Toast's "Sales Exception Report… voided payments and items for re-opened checks"
  exists because voids on closed checks are where cash leaves a restaurant. In dowiz the void is an event in the
  chained log under the order it removed money from, with `by` and `reason`; `Hub::history(order_id)`
  (`lib.rs:505-514`) is the report, and `rebuild` (law 8) proves the served bill equals the fold of it. A void
  with no `by` is refused in `decide` (§6, G3) — the one rule that must never be waived, because an anonymous
  void is the thing that cannot be audited.

### 2.7 Print routing: a station on the line, a second entry in the same outbox

The kitchen's bell is one `outbox::Entry` per order, `id = order_id + kind` "so the same event enqueued twice by
a retried command is ONE entry" (`outbox.rs:65-67`), `to` = the one chat (`notify.rs:196`). Routing is:

- a `station` field on the catalogue product (`kitchen | bar`, owner-editable in the catalogue image, default
  `kitchen`), which travels on the line the way the dish's name already does (`storefront.rs`, "THE DISH'S NAME
  TRAVELS WITH THE LINE");
- at `enqueue_bell`, one rendered text **per station present on the order**, with only that station's lines,
  `Entry.id = order_id + kind + station`, `Entry.to` = `notify.telegram.chat.<station>` if set, else the default
  chat. A venue that has set no bar chat gets one bell exactly as today.

No new image, no new event; an `Amended` that adds a bar line queues a bar entry with the delta's lines and a
kitchen entry only if kitchen lines changed. The Telegram chat IS the printer at this venue; ESC/POS is §5.

### 2.8 The waiter: one new principal, capabilities on the token

`Principal::Staff { person_id, active_location_id, session_id, caps }`, minted through the same roster
(`roster.rs:308,371,426`) and checked at token time by the same live-row read the courier gets — revoked,
expired, "no longer at this location" (`auth.rs:500-522`). `belongs_to` (`auth.rs:358-363`) gains one arm. The
hub token's `Role` gains `Staff` (`token.rs:23-32`); the `Person.role` (`roster.rs:42`) records it.

`caps` is a closed set of capability names carried in the signed token — **a signed capability, never a score**
(`CLAUDE.md:102`) — and `DECISIONS.md:297-298`'s three presets are exactly three cap sets:

| Preset (DECISIONS) | caps | may |
|---|---|---|
| Kitchen | `advance` | move a round through the kitchen edges |
| Waiter (a fourth word, needed) | `take_orders, take_payment` | place, amend before the kitchen, void before the kitchen, take a payment into the open till |
| Counter-Manager | `take_orders, take_payment, void, open_till` | everything above, void after the kitchen, comp, open/count/close the till |
| Owner | all | as today |

A cap is checked in the Worker before the command is sent (the roster is in the venue's `people` image, which the
Worker already reads for the courier), and the signer's `person_id` is written into every `Amended`, `Paid`,
till event and `PayIn/PayOut`. There is no "approval" flow: a waiter who lacks `void` asks a person who has it to
sign the void on their own device. A second signature field (`approved_by`) is a screen, not a data model, and
is not built.

### 2.9 Transfer between tables, and moving lines between checks

- **Move a whole sitting to another table**: `Amended { fulfilment.table }` on every non-terminal round of the
  sitting, one command, one turn. The `sitting_id` does not change; the bill does not change.
- **Move lines from one round to another** (a guest joins another table; "split into two checks"): `transfer
  { from_order_id, to_order_id | new sitting, lines, by }`. Both orders are in the same log image in the same
  object turn; the command builds the two `Amended` deltas and refuses the whole thing if either order is past
  `CONFIRMED` or belongs to another venue. **Conservation: Σ line totals across the two orders is unchanged**
  (§6, G4). Stock does not move: reservations are per order id, so the lines' reservations are `Released` on
  the source and `Reserved` on the destination in the same turn — I3 holds on both ids.
- **Across venues: refused.** A sitting is a venue's; two venues are two objects; that is the two-writer case
  §5 declines to solve.

---

## 3. The `Amended` question, answered

**Decision: one new kind, `EventKind::Amended = 7`, a delta that may change `items`, `subtotal`, `discount`,
`total`, `fulfilment.table` and `adjustments`, and nothing else; written only by the object's `amend`,
`transfer` and (for comps) `amend` commands; legal only while the round is `PENDING | CONFIRMED` unless the
signer holds `void`.**

### 3.1 Why not `Noted`

`Noted` is defined as "a fact was ADDED to an order without its status moving: the customer's note, a courier's
proof of delivery" (`lib.rs:81-88`), and it already carries `courier_id` (`assign.rs:110-118`). Every reader
that asks "did this order's money change?" — the promo use-count (`promo_fields.rs:103`), the analytics fold
(`analytics/fold.rs:116`), the customer list, law 3 — would have to open every `Noted` delta and look for
`total`. A kind byte exists precisely so a record "can be routed without parsing the JSON behind it"
(`lib.rs:59-61`). An amendment that changes money under a kind that promises not to is the defect class
`a18025d4` in a new coat.

### 3.2 Why not lines as their own events

The alternative — each line an `Order`, a round being N orders — makes N reservations, N kitchen tickets, N
status machines per round, and turns the sushi set into eight tickets at the pass. The ledger is keyed by order
id (`stock.rs:60-64`), the bell by order id (`outbox.rs:65-67`), the kitchen's status by order id. Per-line
kitchen state is what print routing is for (§2.7). **Re-entry:** a venue whose bar and kitchen advance the same
round independently — then a line needs its own status, and that is a second FSM, not this one.

### 3.3 The cost, counted

| Where | Change | Cost | Gate |
|---|---|---|---|
| `crates/dowiz-hub/src/lib.rs:62-97, 102-108, 117-125` | `Amended = 7`; `is_order` includes it; `from_byte` maps it | 3 lines + tests | hub suite (337) |
| **Golden signature** `order_machine.rs:547-558` | **none.** Not a status, not an edge; `apply_event` is never called | 0 | `green_live_signature_matches_golden` stays green (`order_machine.rs:1029-1036`) |
| Folds: `hubstore::orders_state` (`1268-1280`), `Hub::orders/history` (`549-567, 505-514`), `rebuild` | none beyond `is_order`: the delta merge is kind-agnostic (`fold.rs`) | 0 | law 8: a fresh fold of an amended log equals the served projection |
| `replica.js:62` `ORDER_KINDS = new Set([1, 2, 3, 5])` | must become `[1, 2, 3, 5, 7]` — and this is the hand copy `vocab.sh` cannot see, because `gen-vocab` reads `dowiz_core` (`tools/gen-vocab/src/main.rs`, measured `grep EventKind` → none) and `EventKind` is `dowiz_hub`'s | one line, plus §6 G1 so it is the last time | G1 |
| `hubdo.rs:1258-1282` dispatch, `broadcast` (`454-470`) | `amend`, `transfer`, `pay` arms; broadcast already routes by `is_order` | ~40 lines | `one-image` stays `bodies=0` |
| `command/amend.rs`, `command/pay.rs`, `command/transfer.rs` | pure `decide` each, ≤300 lines (`file-size` HARD) | 3 files, ~200 lines each (hypothesis), tests in `command/tests.rs` | file-size ratchet; `unreached` baseline 4 not raised |
| **The twin server** `tools/native-spa-server` | reads the same log through `dowiz_hub`; a binary built before `Amended` **quarantines** every kind-7 record (`lib.rs:475-487`) and law 6 goes red on it — correctly. It is outside CI (`ARCHITECTURE-EVOLUTION` §1.1) | deploy order: readers before writers, everywhere the log is read | law 6, and the P8 decision the sibling document left open |
| Archives and the nightly witness | records are moved verbatim (`lib.rs:580-582`); the witness counts records and tips, not kinds | 0 | law 7 |
| The kernel | none. `apply_event_logic` recomputes `subtotal` from the amended `items` on the next transition and `carry_over` restores the Worker's `total` — the same path a promo already rides (`hubstore.rs:1345-1364`) | 0 | the forged-total test (`json_api.rs:362`) still passes because the Worker never trusts wire totals either: `amend` re-prices with `price_basket` |
| Clients (console, waiter app) | draw `amended[]` and `adjustments[]`; the delta fold is shared | UI | none |

### 3.4 One rule the command must carry that the other three do not: a version

`place/advance/assign` need no generation "because the read and the write are two statements inside one object
turn" (`mod.rs:87-92`). That is true of the WRITE; it is not true of the DECISION a waiter made on a tablet
thirty seconds ago from a copy of the order. Two waiters amending one round from two tablets is Square's problem,
and Square's answer is `order.version`: "your request must include the order.version property, which must be
set to the current version of the order or your request returns an error". `AmendIn.base_seq` = the `seq` of the
newest event the tablet folded (`Event.seq`, `lib.rs:283-290`; the socket carries it); `decide` refuses
`Conflict("this order changed while you were editing it")` when the projection's newest `seq` differs. An
`advance` does not need it — the FSM already refuses a stale edge — and a `pay` does not need it because Σ ≤
bill is checked against the live fold. Only an edit to a value needs a version. §4.4 is why.

---

## 4. The research, and what dowiz takes from each

### 4.1 Open tab / check lifecycle

| System | Check ↔ order | Second round | Kitchen sees |
|---|---|---|---|
| **Square** (Orders API) | one `Order`, updated with a sparse body and `version`; optimistic concurrency; a new idempotency key per update | lines added to the same order | whatever the merchant's KDS diffs |
| **Toast** | a **check** is the payment unit; "order by seat" labels lines; "split checks" creates checks; voids "prompt for the check to void" | lines added; "sent" lines are the kitchen's | per send |
| **Lightspeed K-Series** | an order per table with **courses**; "transfer to another seat / course / table" per item; bar tabs with a card pre-auth | lines added, courses fired | per course |
| **Odoo POS** | one order per table; a session with cash control around it | lines added; "send" prints the diff | per send |
| **Loyverse** | an **open ticket**; split = "separate its items into multiple new tickets"; merge; sync across devices "in real time… must be connected to the internet"; offline tickets "will not sync… until the Internet connection is restored"; no stated conflict rule | lines added to the ticket | per save |
| **ebills.al** (the venue, measured) | a **course** is a fiscalised sale; the **bill** is a second fiscalised sale referencing them | a new sale | n/a (no kitchen states) |

Every general POS keeps one growing document per table and hangs kitchen state off "sends". dowiz's kitchen
state is the order's status (§1.5), and the venue's own fiscal platform already treats a round as a sale. **dowiz
follows ebills, not Square: round = order, sitting = id, bill = payments.** What it takes from Square is the one
thing the round needs that ebills does not have — a version on an edit (§3.4). What it takes from Lightspeed is
the vocabulary of transfer (seat / course / table are three destinations of one operation, §2.9) and the
knowledge that a bar tab with a pre-auth is a payment-rail feature (§5).

### 4.2 Split bill

Toast and Loyverse split **checks** (move lines; conservation across documents); Square splits **payments**
(N payments against one order; conservation against a total). Both are needed and they are different
operations: §2.4 (payments) and §2.9 (transfer). The presentation choices — by seat, by item, evenly — are all
"choose an amount" on the payment side; only "these lines are now a separate check" crosses to the transfer side.
Loyverse's cash/card attribution bug is the reason `Paid.method` is a first-class field checked by law 9.

### 4.3 Void and comp

Toast: void reasons "must be configured… before staff can select them"; permission 3.32 lets a role void
same-day payments without a manager; the Sales Exception Report exists to find voids on reopened checks. Square:
a line removed from an open order is just an update (the version protects against races, not against people).
dowiz: the reason set is closed (a `WasteReason`-shaped enum), the signer is on the event, the event is in the
chained log, `history(order_id)` is the exception report, and the permission is a cap on the token (§2.6, §2.8).
The record is what an owner reads at midnight; the permission only decides who could write it.

### 4.4 Authority and lockstep: two tablets on one table

Lockstep (every peer runs every input in order) needs everyone to agree on the order of inputs; an authoritative
server (Overwatch, and every rollback design: "rollback's reconciliation system works because a dedicated,
authoritative server holds the ground truth") makes the order whatever the server received. A Durable Object is
that server for one venue by construction (`hubdo.rs:13-18`), and its `broadcast` is the state update with
interest management ("the oldest trick in multiplayer networking", `hubdo.rs:439-452`). Two tablets do not need
to agree with each other; each predicts (`replica.js:14-18`), sends a command, and folds what comes back.

Where that is not enough is an **edit to a value** built from a stale copy: waiter A removes the tuna, waiter B
adds a beer to the copy that still had the tuna, B's amend lands second and puts the tuna back. That is not a
race in the object — B's write is serialised — it is a **lost update** between two decisions. `base_seq` (§3.4)
is the authoritative server's "your input was for tick 100 and we are at 104": refused, re-fold, re-edit. The
alternative is to send the OPERATION (add beer) rather than the state (the new line set); dowiz's commands
already send intents, not states, for status (`advance.rs:34-37`, "`next` IS A STATUS AND THE ACTION IS NOT
SENT"). For amendments the intent form (`add line`, `remove line`, `set qty`) is the better wire shape precisely
because it commutes more often — two adds never conflict — and the `base_seq` refusal is then needed only for
`remove` and `set qty` on a line another device changed. §7 item 2 chooses the intent form.

### 4.5 Command pattern, replay, and where the game analogy stops

A check IS a stream of commands folded — that is `decide → Event`, `state = fold(events)` (MANIFESTO C3), and it
is why an amendable order costs one event kind rather than a new store. Rollback netcode's move — predict, and on
a misprediction discard the local branch and re-simulate from the server's state — is exactly what the tablet
must do when the object refuses an `amend` with `Conflict`: the predicted line disappears, the replica re-reads,
the waiter is told "the kitchen already started it". `outbox.js` already has the rule for that: "it never
retries a refusal" (`outbox.js:27-30`). The waiter app is the courier app's `tapped()` with amendments in it.

**Where it stops, said plainly:**

1. **The server never rolls back.** A game discards the wrong branch; the log is append-only and chained
   (`lib.rs:384-385`), so a wrong branch on the server side is history with a **compensating** event after it —
   the double-entry rule of `kernel/src/money.rs` ("refunds net to exactly zero", `CLAUDE.md:99-101`). An
   `Amended` that removes a line is a compensation, not an undo; the line's earlier presence is still in the log.
2. **A fiscal receipt is a confirmed frame.** GGPO rolls back a handful of frames; nothing rolls back past the
   frame both sides confirmed. Here the confirmed frame is `Paid` with a fiscal reference (`fic` on ebills,
   `EBILLS` §3.2 (4)); after it, removing money from a bill is `PUT /api/sales-cancel/{id}` — a fiscal act whose
   wire shape "was not observed" — and inside dowiz it is `Refunding → CompensatedRefund` on a round, never an
   `Amended`. The `amend` command's status guard (`PENDING | CONFIRMED`, or `void` cap up to `READY`) and its
   refusal of any round with `payment_status: paid` are that confirmed frame in code.
3. **Money is not a game object.** A duped item is a balance bug; a duped payment is theft. The Diablo II dupes
   came from a duplicate inventory held during a trade and a close path that did not clean it up; the fix
   class is "resolve (read-only validation), then commit (mutation), and only retire the source after the
   destination accepted". dowiz's commands have that shape already — everything in memory, one write at the end
   (`place.rs:98-104`) — and, because both sides of a transfer are in ONE image in ONE turn, there is no
   duplicate to leak. The analogy holds for the mechanism and fails for the stakes: a game can let a dupe stand
   until the next patch; a till cannot, which is why law 9 runs nightly against the live venues and not in a test.

### 4.6 Till reconciliation

Odoo: opening balance, closing balance, "theoretical balance, the real closing balance (what you have just
counted) and the difference", a configurable maximum difference, and a "cash difference loss" journal entry.
Loyverse: "Expected cash amount is calculated from cash sales, refunds, and pay in/pay out… Actual cash amount is
the amount counted physically", with the difference per shift. ebills: a float record at open, refusal to close
over unpaid tables. All three are the same equation; §2.5 writes it once and §6 makes it a law. The one thing
none of the three do that this repo already does for stock is **refold from the bytes and diff** (law 8); the
till gets that too, because a till whose expected figure is a running counter is Loyverse's bug waiting.

---

## 5. Recommended AGAINST, with the re-entry condition

| | Against | Why, in this tree's terms | Re-entry |
|---|---|---|---|
| A | **One growing order per table** (the Square/Odoo/Loyverse shape) | `status` is per order (§1.5); a table at `PREPARING` and `PENDING` at once is unrepresentable without moving kitchen state off the order and re-keying the golden signature | never for this FSM; it is the FSM |
| B | **Lines as their own orders** | N tickets, N reservations, N bells per round (§3.2) | a venue whose bar and kitchen advance one round independently |
| C | **A `VOIDED` status, a `Reopen` edge, or `CANCELLED` from `CONFIRMED`** | each moves the golden signature (§1.6); a void is a money delta, not a lifecycle state; ending an accepted round is `Refunding` and the refund route the FSM memo already names as missing (`dowiz-order-fsm-has-no-exit`) | the refund route, which is a different blueprint and is needed regardless of the room |
| D | **A CRDT for the offline till** (the roadmap's re-entry, re-examined) | Taking ORDERS with the hub unreachable needs no merge: a tablet's taps on one table are a sequence, the outbox replays them in order with tap-time keys, and a stale edit is refused by `base_seq` — a deterministic merge by construction, as `outbox.js:12-20` says for the courier. Taking MONEY with the hub unreachable is different in kind: a `Paid` predicted locally against a bill the object later folds differently is cash taken for the wrong number, and no merge law makes that right; a receipt printed from a prediction is a fiscal act on a guess. So the sharpened condition is not "an offline POS till" but **"a till that must take payment and print a receipt with the hub unreachable"**. That is the two-writer case: the merged fold over two order logs (the G-Set in `mesh_replication.rs:209`) is the easy half, and the rule for a merged fold whose bill disagrees with a receipt already given — refuse-at-fold or compensate — is the design that does not exist. `DECISIONS.md:369` fences CRDTs out of money; the fence stands | the operator states that the room must keep taking payments through an outage longer than the outbox holds (a number to be set, §8), and accepts that the reconciliation rule is compensation |
| E | **Tip distribution** (pools, points, per-hour shares) | it is payroll policy, differs per venue and per country, and the record it needs — Σ `tip` per person per till period, folded from `Paid.by` — is cheap and is what §7 builds; the rule is a pure function the owner would supply later | a venue asks, with its rule written as an equation over integers |
| F | **Printer drivers (ESC/POS, network printers)** | the venue's printer is a Telegram chat (`notify.rs:3`); the outbox already retries and dedupes; a driver is a second delivery channel with a device on the venue's LAN, which the edge cannot reach | a venue with a printer at the pass and no phone; then it is a LAN bridge that polls the outbox, not a Worker feature |
| G | **A second shift record for cashiers** | the courier shift is a drawer that empties into the till (§2.5); a `cashier_shift` beside it is the "second place the same numbers lived" | none |
| H | **A `sitting` image or `Sitting` events** | a second aggregate that can disagree with the log; the class `253e1ece` closed (§2.2) | a venue at a volume where grouping 15 sittings a day from a memoised projection is measurable — it is not (`EBILLS` §1.6: 452 bills / 30 days) |
| I | **Card-present at the table** (Stripe Terminal, tap-to-pay, Lightspeed-style pre-auth tabs) | a payment rail; `Paid` accepts a `card` method today from the webhook only (`stripe.rs`); a card payment recorded by hand at the table is a `Paid { method: card, by }` with no rail behind it and is allowed, as it is on ebills (`CARD_ON_POS`) | the operator's Stripe keys and wallets (memory `dubin-sushi-storefront-2026-09-18`) |
| J | **Approval flows** (`approved_by`, a manager PIN prompt) | trust is the cap on the signer's own token; the void is signed by whoever holds `void` on their device (§2.8) | a venue with one shared device, where "whose token" is not a person — then a PIN re-signs the cap, which is a screen |

---

## 6. The gates each step adds

Each is a ratchet or a law, in the shape of §1.4, and each is proved to fire both ways before it is trusted
(the `prove.mjs` rule, memory `bebop-instruments-that-measure-nothing`).

| | Gate | Red when | Green when | Both-ways proof |
|---|---|---|---|---|
| **G0** | law 3 gains a tip term: `total == lines + fee − discount + tip` (`conservation.mjs:59-70`) | a live order's total is not that | it is | prove case: an order with `tip: 200` and the right total is green; the wrong total by 200 is red |
| **G1** | `tools/gates/event-kinds.sh` — the set of kind bytes for which `EventKind::is_order` is true, read from `lib.rs`, must equal `ORDER_KINDS` in `replica.js:62` (and any later copy the grep finds) | a kind is added to one and not the other | equal | add `7` to one side only → red; both → green. Better: `gen-vocab` gains `dowiz_hub` as a dependency and emits `ORDER_KINDS` into `vocab.js`, and `vocab.sh` (`ci.yml:62`) covers it — then G1 is a diff, not a grep |
| **G2 — law 9, the till** | `conservation.mjs` law 9 over a new `/api/owner/health` block `till: { open, periods[] }` computed by the object from the two images: **(a)** for every closed period, `float + cash_paid + pay_in − pay_out − cash_refunds == counted + over_short`; **(b)** no cash `Paid` timestamp falls outside every period; **(c)** every `PayIn { source: shift:<id> }` equals that courier shift's `cash_collected`; **(d)** an open period older than 24 h is "the drawer nobody closed" (the witness rule, `conservation.mjs:161-165`) | any of a–d, with the period and the order named | none | `prove.mjs` cases: a balanced period; a period short by 100; a cash `Paid` at 03:00 with no period; a hand-in of 4,500 against a shift of 5,000; an open period 30 h old; and — ABSENT IS NOT ZERO (`prove.mjs:12-16`) — a Worker without the `till` field is green |
| **G3** | native tests in `command/tests.rs` (the amend rule): `Amended` with no `by` → `Refused`; with a stale `base_seq` → `Conflict`; on a round at `PREPARING` without `void` → `Conflict`; on a round with `payment_status: paid` → `Conflict` regardless of caps; after a refusal both images are byte-identical to before (the `place` proof, `hubdo.rs:604-617`) | any passes | all refused | the tests are the proof; the positive twin (a legal amend lands, stock `Released`/`Reserved` net to the new lines, and `ledger.stranded()` is empty) is beside each |
| **G4** | `transfer` conservation: Σ line totals over `{from, to}` before == after; a transfer to a round past `CONFIRMED` or another venue leaves both untouched; `stranded()` empty after any refusal | not conserved | conserved | native tests, plus law 8 on a log with transfers |
| **G5** | `pay` conservation: `paid + amount > bill` → `Conflict`; Σ `Paid.amount` at close == bill exactly; `1000 div 3` = 333, 333, 334 with the remainder on the last; `Paid.method` is one of the closed set (a cash-and-card split is two `Paid`s) | over-payment accepted, or a sum off by one | exact | native tests; law 9 catches the method bug live |
| **G6** | `one-image.baseline` stays `bodies=0`; every room command is an object command | a Worker handler touches two images | 0 | existing gate |
| **G7** | `file-size` (`over=40 worst=1737`) does not rise; `amend/pay/transfer/till` are new files ≤300 lines | a file crosses 300 or the worst grows | as today | existing gate |
| **G8** | `unreached.baseline` stays 4; every new `pub fn` has a route in the same commit | a room capability nobody calls | 4 | existing gate; it is the one this document is most likely to trip, so each §7 item names its route |
| **G9** | `idempotent.sh` learns a second pairing: the waiter app's `tapped()` verbs ↔ `staff.<verb>` guards; the header says it "cannot see a route queued by some future surface that does not use `tapped`" (`idempotent.sh:24-27`) — the waiter app must use `tapped` | a queued verb with no guard | paired | the gate's own shape: rename a guard → red |
| **G10** | `vocabulary.sh` (`ci.yml:55`) covers the reason set and the cap names in sq/en/uk | a reason with no word | all worded | existing gate |

---

## 7. The order of work, with each item's CHECK

**0. Fix law 3 (G0).** Gates outrank rows (memory `gates-must-improve-not-degrade`); a conservation law that
cannot see tips is a law that will be switched off the first evening a table tips. Half a day (hypothesis).
**CHECK:** `node e2e/gates/conservation.prove.mjs` with the two new cases; `node e2e/gates/conservation.mjs`
against both venues, and its output says whether any live order carries a tip (§8, row 1).

**1. The staff principal (A4).** `Role::Staff`, `Principal::Staff { caps }`, roster invites reused, `belongs_to`
arm, `staff_at()` beside `courier_at()`. Routes: `POST /api/staff/login`, and `place` accepting a staff token
for `dine_in`. No new image. **CHECK:** native — a staff token for venue A is refused at venue B
(`belongs_to`); a token without `advance` is refused by `order_action`; `unreached` stays 4; `one-venue` stays 0.
Live — a waiter places a `dine_in` round at table 7 and the bell says "🍽 table 7".

**2. `sitting_id`, `Amended`, and the `amend` command in intent form (A5).** `EventKind::Amended = 7`;
`AmendIn { order_id, location_id, base_seq, ops: [Add{line} | Remove{line_id} | SetQty{line_id, qty}], by,
reason?, now_ms }`; the Worker re-prices the resulting line set with `price_basket` and sends the priced
envelope fields alongside; `decide` does status guard → `base_seq` → stock delta → append → broadcast. G1 in the
same commit (the `replica.js` copy). Deploy order: any reader of the log (the twin server, if kept) before the
first `Amended` is written. **CHECK:** G3; law 8 green on a venue after an amend (`/api/owner/health.rebuild.stale`
empty); law 6 green (nothing quarantined) — and, deliberately, law 6 RED on a reader built before this commit,
which is the proof the kind byte is load-bearing.

**3. `pay` and the split (A6, first half).** `Paid { amount, method, by, till_id?, covers? }` per round; the
Σ ≤ bill refusal; `payment_status: paid` at close; the sitting projection on `/api/owner/orders` (grouped view,
same memo). **CHECK:** G5; the customer's tracking sheet for a dine-in round shows paid; law 3 unchanged (a
payment does not change a total).

**4. The till and law 9 (A7).** `till` LogImage with the five events; `open/count/close/pay-in/pay-out` routes
under `staff.*` with `open_till`; the `expected` fold in the object; the `health.till` block; the courier
hand-in as `PayIn { source }`. `pay { method: cash }` refused with no open till. **CHECK:** G2 both ways
(the six prove cases), then live: open with a float, take two cash payments, count blind, close — the health
block's `over_short` is the number the drawer says, and the conservation run prints `CONSERVATION HOLDS`.

**5. Void and comp (A6, second half).** `Remove` with a `reason` and, past `CONFIRMED`, the `void` cap; `Comp{line_id,
reason}` as an op that grows `discount` and appends to `adjustments`. **CHECK:** G3's void cases; law 3 holds with
a comp (the discount term); `history(order_id)` shows the void with `by` and `reason`.

**6. Transfer (A6, third half).** `transfer` between rounds, and "move sitting to table". **CHECK:** G4; law 8.

**7. Print routing (A8, first half).** `station` on the product; per-station entries in `enqueue_bell`.
**CHECK:** native — a round with a kitchen line and a bar line yields two entries with disjoint lines and distinct
ids, and a venue with no bar chat yields one; live — the bar's chat receives only its lines.

**8. The tip record (A8, second half, the half that is built).** A fold `tips_by_person(period)` over
`Paid.by` × the rounds' `tip`, served under the till's Z report. No distribution. **CHECK:** Σ over people == Σ
`tip` over the period's rounds, as a native test on a synthetic log.

**Order rationale.** 0 is a gate and gates come first. 1 is the person every later event names in `by`; nothing
signed can be built before there is a signer. 2 is the one change to the storage vocabulary and the one with a
deploy-order constraint, so it goes early and alone. 3 before 4 because the till's law folds `Paid`s, and there
must be `Paid`s to fold; 4 before 5 because a void after payment is a refund (§4.5 (2)) and the refusal that says
so needs `payment_status`. 5 and 6 are the same command with more ops. 7 and 8 touch nothing above them. A9 (the
QR onto the storefront) stays OPEN: the customer token is minted per order (`storefront.rs:1222-1233`, "the
subject is the order") and a sitting needs a token per sitting, which is a small change to `Claims::Customer`
that this document notes and does not design.

---

## 8. What could NOT be determined from the tree

| Question | Why it matters | The command or person that settles it |
|---|---|---|
| Does any live order carry a `tip > 0`? | decides whether law 3 is broken live or merely blind (§1.7) | `node e2e/gates/conservation.mjs` after G0, or `curl …/api/owner/orders` and `jq '[.[] \| select(.tip>0)] \| length'` with the owner token in `/root/.dowiz_owner` |
| Does the twin server quarantine an unknown kind, or skip it? | the deploy-order constraint in §3.3 assumes `dowiz_hub::quarantined()`; if the twin has its own reader it may silently drop kind 7 | `grep -n 'from_u8\|from_byte\|quarantined' tools/native-spa-server/src/*.rs`; and the P8 decision (keep in CI or delete) |
| How will `fulfilment.table` be spelt once the booking lane lands? | today a free string (`storefront.rs:98, 1028`); the lane's floor names `(zone_id, table_n)` (`booking.rs:383-384`) — a sitting projection grouped by a string that a booking spells differently is a table that appears twice | the booking lane's final `tables.rs` API; one function `tables::key(zone, n) -> String` used by both |
| What will the ebills lane name the fiscal block on `Paid`? | §4.5 (2) keys "the confirmed frame" on it; `EBILLS` §3.2 (4) proposes `uuid/fic/invOrdNum/paymentMethod` in the `Paid` payload | the ebills lane's commit; this document's `Paid.fiscal?` should be that shape verbatim |
| The operator's word for the fourth staff preset | `DECISIONS.md:297-298` names three; the room needs a waiter who is not a Counter-Manager (§2.8) | the operator |
| The outage the room must survive | decides §5 D's re-entry: the outbox holds taps, not payments; how long is "the hub is unreachable" before the venue stops trading? | the operator; then a number in `outbox.js`'s bound |
| Whether a `Staff` socket should get the console's interest set | `TAG_CONSOLE` hears every order including addresses (`hubdo.rs:439-452`); a waiter needs the room's rounds, not a delivery customer's doorbell | read `live.rs` and decide a `TAG_ROOM` that filters `fulfilment.kind == dine_in` |
| CPU cost of a sitting-grouped projection at the live venue | assumed negligible (15 sittings/day, §1.2); the object memoises per generation | `otel` spans on `/api/owner/orders` before and after item 3 |
| Whether the venue wants the kitchen's `PREPARING` for a round rung up at the table at all | ebills has no kitchen states (`EBILLS` §3.1); a waiter may want `Placed → PICKED_UP` in one tap for a drink | the operator; if so, it is `order_action` intents, not a new edge (`Ready → PickedUp` exists) |

**Sources consulted for §4** (read 2026-09-22): Square — [Update Orders](https://developer.squareup.com/docs/orders-api/manage-orders/update-orders),
[Optimistic Concurrency](https://developer.squareup.com/docs/build-basics/common-api-patterns/optimistic-concurrency);
Toast — [Voiding checks](https://doc.toasttab.com/doc/platformguide/adminVoidingOrders.html),
[Void Items, Payments, and Checks](https://support.toasttab.com/en/article/Voiding-Items-Payments-and-Checks),
[Order by Seat](https://support.toasttab.com/en/article/Order-by-Seat),
[Split Checks](https://support.toasttab.com/en/article/Splitting-Checks-by-Item-1492811097734);
Lightspeed — [Transferring items and orders to other tables](https://k-series-support.lightspeedhq.com/hc/en-us/articles/10032856591643-Transferring-items-and-orders-to-other-tables),
[Creating and managing bar tabs](https://k-series-support.lightspeedhq.com/hc/en-us/articles/4408089985179-Creating-and-managing-bar-tabs);
Odoo — [Cash control](https://www.odoo.com/documentation/14.0/applications/sales/point_of_sale/shop/cash_control.html);
Loyverse — [Shift Management](https://help.loyverse.com/help/shift-management-loyverse-pos),
[Splitting Open Tickets](https://support.loyverse.com/en/articles/1033305-splitting-open-tickets),
[Open Tickets Synchronization](https://help.loyverse.com/help/tickets-synchronizations),
[the cash/card split defect](https://loyverse.town/topic/5214-when-customer-split-their-payment-into-cash-and-credit-the-whole-amount-is-added-to-total-cash-payment-which-make-the-expected-cash-amount-to-be-not-balanced-when-closing-shift/);
netcode — [Client-Side Prediction and Server Reconciliation (Gambetta)](https://www.gabrielgambetta.com/client-side-prediction-server-reconciliation.html),
[Netcode Architectures Part 2: Rollback (SnapNet)](https://www.snapnet.dev/blog/netcode-architectures-part-2-rollback/),
[Overwatch netcode deep dive](https://edgegap.com/blog/game-backend-deep-dive-overwatch-2016-netcode-architecture-rollback);
inventory — [Diablo II dupes walkthrough](https://gist.github.com/amtal/bf941bde443eefc7d4626fd439d7f480),
[atomic container transfer (resolve/commit)](https://github.com/jm-sky/seedvale/pull/110).
