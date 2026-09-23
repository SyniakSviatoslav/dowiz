# Operational blind spots: sixteen points from an outside critique, checked against the tree

**Date:** 2026-09-23. **HEAD read:** `8a964dc1` ("room: pay, and the bill split across as many payments as it takes").
**Method.** READ-ONLY on code; nothing was built or run except `grep`/`sed`/`git log`. Every "today" statement names a
`file:line` or the grep that returned nothing. `measured` = a command was run here and its output quoted; `hypothesis` = it
was not. Four coding lanes were editing concurrently, so a line number can drift by a few lines; the SYMBOL beside it is
what to search for. Legal claims cite a URL; where the only source is a vendor summary it says so. Related blueprints are
cited by section: `POS` = `BLUEPRINT-POS-THE-ROOM-2026-09-22.md`, `TAX` = `BLUEPRINT-TAX-PRICE-CHANNEL-2026-09-22.md`,
`CRM` = `BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22.md`, `EBILLS` = `BLUEPRINT-EBILLS-INTEGRATION-2026-09-22.md`,
`STACK` = `BLUEPRINT-STACK-AND-DEPENDENCIES-2026-09-22.md`, `LAST-MILE` = `BLUEPRINT-LAST-MILE-2026-09-22.md`,
`WASM` = `BLUEPRINT-BEBOP-IN-WASM-2026-09-23.md`.

**Two facts that frame every row below, both from memory notes and re-checked in the tree:**

1. **The stock ledger is correct and switched OFF.** `dowiz-stock-ledger-works-but-is-off`: 165 dishes, 0 recipes, 0
   supplies on both live venues. `place` reserves nothing for a dish with no BOM (`command/room_rules.rs:152-160`, "a line
   whose product it did not send reserves nothing, exactly as placement treats a dish with no recipe"). Every inventory
   point in the critique (P1-2, P2-1, P2-4, P3-4) is therefore about a mechanism nobody has turned on, and the first
   step for all four is the same: recipes on the dishes that matter.
2. **dowiz does not fiscalise anything today.** The venue's fiscal device is ebills.al, a cloud POS (`EBILLS` §0-§1).
   `workers/api/src/ebills/` is a pure parser with no route (`grep -n "mod ebills" workers/api/src/lib.rs` → nothing,
   measured); `workers/api/src/services/fiscal/` does not exist (`ls workers/api/src/services/` → analytics catalogue
   courier customers engagement identity operations ordering orders venue, measured). Pushing orders INTO a fiscal
   platform is out of the ebills lane's scope by the operator's instruction (`EBILLS` §5 item 6). So P1-1 and P3-2's
   fiscal halves are about a seam that is designed (`TAX` §3.7) and not built.

---

## 0. The two direct answers

### Q1 — How does the stock model close the gap between recipe-derived "virtual" stock and physical loss? Is there a WriteOff event with a reason and a person?

**There is a write-off event with a reason and WITHOUT a person, and the two physical-loss mechanisms are the right
shape.** `crates/dowiz-hub/src/stock.rs:56-69` defines six events: `Received`, `Reserved`, `Consumed`, `Released`,
`Wasted { item, qty, reason: WasteReason }` and `Stocktake { item, observed, stocktake_id }`. `WasteReason` is a closed
set `Spoiled | Dropped | Unsold` (`stock.rs:29-34`). Virtual stock is never a stored counter: `StockLedger::fold(events)`
(`stock.rs:344`) and `available = on_hand − reserved` (`stock.rs:132`). Physical loss enters in two ways:

- **`Wasted`** takes `qty` off `on_hand`, and is REFUSED into a reservation: "Wasting into a reservation would let a
  kitchen bin a portion it has already promised" (`stock.rs:253-263`).
- **`Stocktake`** sets `on_hand := observed` and resets the conservation basis (invariant I2, `stock.rs:332-337`); a
  count below what is reserved is refused so the person recounts (`stock.rs:265-274`). This is the drift closer: the
  gap between what the recipes said and what is on the shelf becomes the difference between the fold and the count,
  and the next fold starts from the count.

**Comps, voids after cooking and dropped plates — what the tree does deliberately:**

- Ingredients are `Consumed` at `PREPARING` and nowhere else (`command/advance.rs:52-62`: "PICKED_UP and DELIVERED …
  by then the consumption already happened").
- A **void after the kitchen has the round** leaves the shelf untouched by construction: `command/amend.rs` returns
  `restock && stage == Stage::BeforeKitchen` (the last line of `apply`), and `room_rules.rs:60-64` says why: "the food
  is gone whether it was eaten or dropped". The reason (`mistake | guest_changed | unavailable | dropped | other:<text>`,
  `room_rules.rs:16-22`) and the signer (`by`, mandatory, `room_rules.rs:86-88`) go on the ORDER's `Amended` event, not
  on the stock log. That is correct for `on_hand` (no double-count) and it means the WASTE REPORT — what was binned —
  must be folded from two logs: `Wasted` events plus voids-after-kitchen with reason `dropped`. No such fold exists.
- A **comp** leaves the line and the stock alone and grows `discount` (`amend.rs`, `Op::Comp`); stock was consumed at
  `PREPARING`, the food was served, so nothing is owed to the shelf. Correct.

**The gaps, precisely:**

1. **No person on `Wasted`.** The stock log's actor slot is zero for every event ("recorded by the hub rather than
   signed by a person", `stock.rs:749-755`) and the only route, `POST /api/owner/stock/:kind`
   (`services/operations/stock.rs:107-171`), is owner-only (`owner_and_venue`) and carries no `by`. So today a write-off
   is always "the owner", which at a venue where a Counter-Manager bins the tuna is a record of the wrong person.
2. **A missing reason defaults silently to `Spoiled`** (`services/operations/stock.rs:143-148`,
   `.unwrap_or(WasteReason::Spoiled)`). Under this repo's own law ("refuse, do not default", `ebills/mod.rs` header) that
   is a defect: the report will say the kitchen spoils everything.
3. **`WasteReason` has no `Returned`** (a delivery refused at the door, P1-4) and no `Comped`/`Staff meal`; the closed set
   is the right idea and is one variant short per real case.
4. **`Stocktake` sets `on_hand` only; reservations survive** — by design (I2) — so an order abandoned at `CONFIRMED`
   holds its ingredients for ever, and the venue cannot count its way out (`dowiz-order-fsm-has-no-exit`). The refund
   route the FSM memo names as missing is the fix, not a stock change.

**Answer in one sentence:** yes, there is a WriteOff (`Wasted`) with a closed reason and a Stocktake that re-bases the
fold, and the void-after-kitchen rule is right; what is missing is `by` on `Wasted`, a refusal instead of a default
reason, one or two reasons, and a waste report that reads both logs — and none of it measures anything until the venue
has recipes.

### Q2 — Does the architecture support local autonomy when the venue loses internet?

**The architecture supports it; the tree does it for reads everywhere, for writes on the courier's phone only, and for
neither the till nor a receipt.** Measured:

- **Reads survive.** The console draws `replica.js` before any request and says how old it is (`public/lib/replica.js:
  1-25`, `STALE_MS = 15 min` at `:27`); the courier app now has its own service worker (`1b978569`, `public/courier/sw.js`,
  "reopens underground and says what it is showing", roadmap D2 — LANDED, the roadmap still says OPEN). The storefront
  and kit SWs are network-first and never cache `/api/` (`public/sw.js:3`; `outbox.js` header quoting `kit/sw.js`), which
  is right for prices.
- **Writes survive on the courier's phone.** `public/lib/outbox.js`: IndexedDB queue, key minted at tap time, drains IN
  ORDER, stops at the first unresolved entry, never retries a refusal ("409 is an ANSWER"), `MAX_QUEUE = 64`, six
  transient retries. Server side: `idempotency::guard` on `courier.*`, `owner.order_action`, `staff.pay`
  (`services/orders/room/pay.rs:34-41`) and `staff.amend`. The merge is deterministic by construction: a single writer
  per venue (the Durable Object), a refusing FSM (`order_machine.rs allowed_next`), and for value edits a version
  (`AmendIn.base_seq`, `amend.rs:90-92`, "this order changed while you were editing it"). The intent form (`Op::Add |
  Remove | SetQty | Comp | Table`, `amend.rs:31-46`) makes two adds commute; only `Remove/SetQty/Comp` carry the version.
- **What does NOT exist:** a waiter/till surface at all (`ls workers/api/public/` → `admin courier kit lib platform store`,
  measured — the room routes `/api/staff/room|amend|pay` at `lib.rs:307-309` have no page); a write queue on the console;
  a **local decider** (pricing, stock check and bill live in the object; `replica.js` is "a PREDICTION … the server is
  still the authority", `:14-18`); a local receipt.
- **Merging back deterministically without conflicts** is already what `queue + Idempotency-Key + refusing FSM +
  base_seq` gives for a SEQUENCE of intents from one device against one object — `STACK` §4.2 walks the roadmap rows and
  finds one true CRDT shape (two devices amending one tab while both are cut off from the hub and from each other),
  and both `STACK` §4.3-4.4 and `POS` §5 D answer it with a refusal, not a merge; `DECISIONS.md:369` (O4) fences CRDTs
  out of money and orders and the fence stands.
- **The two things a local-first till would need that are missing** (`STACK` §4.3 item 1): (a) `command/*::decide`
  compiled where the tablet is — a crate-ownership refactor (`decide` into `dowiz-hub`, which is bytes-in/bytes-out with
  zero awaits) and the format reader in wasm already exists (`crates/bebop-wasm`, 30,892 bytes, `WASM` §0 item 8); (b) a
  product rule for the one conflict a merge cannot refuse away — the last portion sold online and in the room during
  the same outage (refuse-at-fold or compensate; `STACK` §4.3 item 3, still undecided).
- **Local receipts** are not a fiscal act on a guess in Albanian law: Law 87/2019 art. 29 lets the taxpayer issue and
  print an invoice WITHOUT the NIVF during an internet interruption and fiscalise it within 48 hours (§5, sources). So
  the objection in `POS` §5 D is not legal, it is arithmetical: a receipt printed from a bill the object may later fold
  differently. With a local decider that folds the same bytes with the same functions, the bill is not a prediction —
  which is the re-entry condition sharpened once more: **local payments and receipts are safe exactly when the tablet
  runs the object's own `decide` over the object's own last image, and the only residual conflict is stock.**

**Answer in one sentence:** yes by construction (single writer per partition, ordered idempotent replay, refusals as
answers, versions on value edits), no in the tree for the room (no surface, no local decider, no local receipt), and no
CRDT is needed or allowed.

---

## 1. The sixteen points

Severity is for ONE real venue in Durrës over the next three months: 18 tables on ebills, delivery on dowiz, EUR-paying
tourists, no recipes entered yet.

| # | Point | What the tree does today | Verdict | Severity |
|---|---|---|---|---|
| P1-1 | Fiscal offline queue, 48 h, strict order, one rejection stalls | dowiz issues no fiscal documents; `ebills/` unwired (grep `mod ebills` → nothing); seam designed `TAX` §3.7 (`Document.uuid` idempotency, `Refusal::AlreadyFiscalised`, result as `Noted{fiscal}`); nothing about deadlines or ordering. ebills wire measured: per-POS ordinal `invOrdNum`, `iic`/`fic` per sale, `iicReference` on summary bills (`EBILLS` §1.5) | **Partly right, and misdescribed.** Law 87/2019 art. 29: the clock is 48 h and runs per invoice from its issue without NIVF (§5); each invoice has its own IIC/FIC, so nothing is a "chain" that one rejection stalls — a rejected invoice is corrected by a corrective invoice referencing it (art. 32; registered invoices "cannot be deleted or cancelled"). What is right: a backlog with per-item deadlines needs a drain that does not head-block on a refusal, and a visible oldest-deadline. GAP, but only when the push exists (B7/B8) | **Low now, high the day dowiz pushes.** Today the venue's own cloud POS carries this risk, and how ebills behaves offline is not known (§5) |
| P1-2 | Inventory drift; WriteOff with reason and person | `Wasted{reason}` + `Stocktake` (`stock.rs:56-69, 253-274`); route owner-only, no `by`, default reason (`services/operations/stock.rs:107-171`); void-after-kitchen leaves shelf alone (`amend.rs`, last line of `apply`) | **Partly.** Event and reason exist; person, refusal-not-default, `Returned` reason and a two-log waste report are missing | **Med** — and ZERO effect until recipes exist (0 today) |
| P1-3 | Mixed payments, one fiscal transaction, no holes in the shift, wallet never negative | `pay.rs`: N `Paid` per round, `Σ ≤ total` refused (`:108-113`), closed methods `cash card cheque transfer gift_card other` (`:51-58`) — no `wallet`; `till_id` optional (`:29-30`) and NO "open the till first" guard (`grep -n till pay.rs` → the field only); `till.rs` exists (200 lines, five events, `expected()` at `:116-118`) with **no route and no image** (`hubdo/room.rs:25-31` dispatches `amend`,`pay` only; `grep -rn "command::till\|TillEvent" workers/api/src` outside the file → nothing). Tip-as-remainder is REFUSED (over-payment). Wallet: `crates/dowiz-core/src/wallet/mod.rs` single-writer LWW, ledger legs (`ledger_account.rs`) | **Partly.** The split and the conservation rule are landed (G5); the till is written and unreachable; wallet tender, tip-at-payment and the one-fiscal-document-per-sitting are gaps. ebills' own enum has `MULTIPLE` (`EBILLS` §1.5) and the law lists the payment means per invoice (§5) | **High** — cash+card at one table is nightly at this venue |
| P1-4 | Courier: refusal at the door, cash not returned; ReturnedToKitchen/WriteOff; per-courier cash audit | `deliver` records `short = cash_due − collected`, "never silently rounded" (`courier.rs:598-604`); earnings folded from orders, tips apart, `cashInHand`/`expectedCash` (`courier.rs:736-780`); shift close refused with a run in hand (`courier.rs:223-232`); conservation law 4 (`e2e/gates/conservation.mjs:19`). No failed-delivery action: owner intents are confirm/reject/preparing/ready/collected/cancel (`POS` §1.5, `owner.rs:561-574`); `CANCELLED` only from `PENDING`; **no route emits `Refunding`** (`grep -rn Refunding workers/api/src` → status helpers only, measured; `dowiz-order-fsm-has-no-exit`). Hand-in to a till: designed `POS` §2.5, `till.rs::pay_in` with `source`, not reachable | **Partly / GAP.** The audit half is strong; the "door refused" half has no exit from the FSM and no stock word for food that came back | **High** — a refused delivery today is an order that cannot be ended and stock held for ever |
| P1-5 | Insider fraud: cash then void; unauthorised discounts; alerting | A paid round cannot be amended (`room_rules.rs:84-97`, `changeable`); after the kitchen, `Remove`/`Comp` need `Cap::Void` (`amend.rs:108-116`; caps from the roster, `handlers.rs:136`); `by` and a closed `reason` mandatory; every amendment in the chained log under its order, `Hub::history` is the exception report (`amend.rs` header); comps grow `discount` and append `adjustments{by,reason,at}`; promo codes are a refusal engine (`promo.rs`). No report page, no alert. `tools/gates/no-scoring.sh` refuses `(staff|waiter|user|…)_(score|rating|rank|tier|reputation)`, `reputation`, `vip` | **Partly, and the "anomaly score" half is REFUSED by DECISIONS (OD-8, `CLAUDE.md` "a signed capability, never a score").** The need is met without scoring a person: an exception REPORT of events with `by` on each row, and threshold alerts on EVENT counts per venue-period (§2) | **Med** — the exposure is limited to `Void` holders (Counter-Manager/Owner); the missing piece is that the owner cannot SEE it without reading a log |
| P1-6 | Kitchen desync: "sent" vs printer out of paper; print status, ack | Bell written in the placing turn into the `outbox` image (`outbox.rs:14-19`), six retries with backoff, `Verdict::Abandon` reported on `/api/owner/health` (`:37-58, 104, 144-153`); the console replica says its age (`replica.js:27`). Telegram's 200 is the only "delivered". No printer, no "seen by a human" | **Partly; the rest is designed.** `LAST-MILE` §3.1 step 1 (a print rail served to a polling printer, the printer's `DELETE` is the ack) and step 3 (queued/printing/printed/failed on the console) | **Med** — the venue's printer IS a Telegram chat today (`notify.rs:3`); paper-out is a phone-screen problem |
| P2-1 | Chains, central kitchen: inter-warehouse transfers, batch expiry, cost by origin | One object per venue; `Place::of_authorised` + `one-venue` gate; stock ledger per venue; `Received{item, qty}` only (`stock.rs:58`); `grep -in "expir\|fifo\|batch\|supplier\|transfer" stock.rs` → nothing | **GAP by design, correctly deferred.** A second venue is the re-entry (§4) | **Low** — one venue |
| P2-2 | GDPR erasure vs append-only log; crypto-shredding | `crates/dowiz-hub/src/forget.rs`: in-place redaction keeping `id`/`prev`, kind byte `\| 0x80` (`:36`), `redact()` rewrites the image (`:61-100`), `declared()` vs tombstones = a conservation law (`:106-114`); `EventKind::Forgotten = 8` (`lib.rs:115`); readers mask the bit (`lib.rs:823`). **No route yet** (`grep -n forget workers/api/src/lib.rs` → nothing, measured) | **Solved in design, half-landed; the choice is right for this system** (judgement in §2). Crypto-shredding is the re-entry (`CRM` §2.2), not the alternative | **Low** — the statutory window is a month; `CRM` §1.2 shows the 30-day PITR floor makes no shorter promise honest |
| P2-3 | Aggregators in Durrës; menu/status/stock sync; cancellation penalties | `grep -rin "wolt\|glovo" --include=*.rs --include=*.js` → nothing (docs only); `channel` as a closed set with a per-channel profile designed `TAX` §2.8/§3.6 (B5); `price_trusted:false` is the kernel's word for "somebody else priced it" (`domain.rs:118-121`); `OutOfStock` refusal is the automatic 86 (`stock.rs`) | **GAP, with the seam designed.** Research (§5): Wolt operates in Durrës since March 2025; Glovo mostly Tirana with limited Durrës; Baboon (Albanian) Tirana/Durrës/Vlorë. Wolt has Order/Menu/Venue APIs; Glovo a Partners API. Wolt deducts compensation for merchant-attributable cancellations (Poland addendum; Albania terms not read) | **Med** — the venue may already be on Wolt; if so, its stock is drawn from a shelf dowiz cannot see |
| P2-4 | Dynamic COGS, FIFO batches, margin per dish | `costPerBasis` per supply (`recipe.rs:85`), dish `cost` derived when every food line has one (`recipe.rs:129-145`), shown as a % in the console (memory: "cost 760 = 77%"); no purchase price on `Received`, no lots | **Partly.** Static cost and margin exist; a cost that follows purchases does not. FIFO lots recommended AGAINST (§4); a weighted-average fold is the fit | **Low** (0 recipes) |
| P2-5 | Edge split-brain; local-first cashier tablet; deterministic merge | See Q2 | **Partly; architecture yes, room no; CRDT refused and not needed** | **Med** — the venue's wifi at six in the evening is real; the console already survives it for reads |
| P3-1 | Interactive floor plan with live states | `tables.rs`: zones, `Table{x,…}` on a `PLAN_W×PLAN_H` viewBox (`:22-24, 55`), `MAX_TABLES`, `DWELL_MIN`, occupancy computed per minute from bookings (`holds_table`, `:246`); `sitting.rs::room()` gives live sittings from the rounds (`handlers.rs:23-41`); ebills' floor (`OCCUPIED`, `orderTotal`) is readable (`EBILLS` §1.6). `grep -n dirty tables.rs sitting.rs` → nothing | **Partly.** Five of six states are FOLDS today (§2); `dirty` needs one mark | **Med** — 18 tables, staff already look at a floor on ebills |
| P3-2 | EUR/ALL at one till, change in the other currency, drawer in both, ebills and EUR | `Currency::{All, Eur, Usd}` (`crates/dowiz-core/src/money.rs:34-41`); `convert_all_to_eur_cents(_, rate: f64)` is the kernel's one remaining money float (`kernel/src/money.rs:332`, `TAX` §1.2); `Paid` has `amount`+`method` and no currency (`pay.rs:20-33`); `till.rs` has no currency. Law 87/2019: any currency allowed, totals in LEK at the Bank of Albania rate, the invoice states the currency and rate (§5); ebills' sale carries `currencyRate`, `currency{sellRate,buyRate}`, `exchange` (`EBILLS` §1.5) | **GAP.** The fiscal side is a field, not a problem; the till side is a per-currency fold | **High** — coastal Durrës in season; EUR cash is daily (hypothesis: no till data yet) |
| P3-3 | Granular RBAC, manager PIN, clock-in/out, labour % | `Cap` closed set `Advance TakeOrders TakePayment Void OpenTill`, no `Ord`, deny by default, unknown name refuses the set (`caps.rs:1-58`); four presets (`caps.rs:20-24`); staff sessions endable per device, token lives one 12 h shift (`staff_rules.rs:1-30`); a token without a capability is refused (`auth.rs:781`). PIN: `POS` §5 J AGAINST. `grep -rni "clock_in\|clockin\|labour" workers/api/src` → nothing | **RBAC solved; PIN refused with a stated re-entry (a shared device); clock-in and labour % GAP** | **Low-med** — a shared tablet at the pass is likely, which is the PIN re-entry |
| P3-4 | B2B procurement: delivery notes, price check, FIFO cost, supplier debt | `Received{item, qty}` (`stock.rs:58`); nothing for supplier, document, price, debt | **GAP**, after recipes and after P2-4's cost fold | **Low** |
| P3-5 | KDS with course pacing | A round IS a ticket with its own status (`POS` §1.5); `scheduled_for_ms` on the order; no course label, no hold/fire | **GAP, and mostly a kitchen-display feature**; the data model needs one word | **Low** — sushi; courses are rare at this venue (hypothesis) |

**Corrections to the roadmap the table implies:** D2 (courier cold start) is LANDED (`1b978569`); A7 (the till) is
"written, unreached" rather than IN FLIGHT; C3 (erasure) is "hub function landed, route pending"; three documents each
call something "law 9" — the till (`POS` G2), `declared == redacted` (`CRM`, `forget.rs` header) and the tax block that
actually landed as law 9 in `conservation.mjs:25-26` (`d5f0eca1`). The next law is numbered by the file, not the doc.

---

## 2. Per gap, the smallest design that fits, with a CHECK

Every design below reuses what exists: `LogImage` for things that are not orders, the stock ledger, the `Amended`/`Paid`
deltas, the `outbox` image and its verdicts, the object's one-turn commands, and the conservation audit's prove-red-then-
green rule. No new status, no new FSM edge, no CRDT.

### 2.1 Write-off with a person; refusal not default; the waste report (P1-2, Q1)

- `StockEvent::Wasted { item, qty, reason, by: String }` — `by` mandatory, encoded beside `reason` (`stock.rs:370-395`,
  `encode`/`decode`); the decoder accepts a missing `by` on OLD records as `""` (the events already written must still
  fold). `WasteReason` gains `Returned` (food back from a refused delivery) and `StaffMeal`; still closed.
- `POST /api/owner/stock/wasted` becomes a staff route under `Cap::OpenTill` (the drawer's holder is the one who bins
  stock at midnight) or `Owner`; `by` = the signer's person id; a reason outside the set → **400, never `Spoiled`**.
- **Waste report** = a pure fold, served under `/api/owner/stock/waste?period=`: `Wasted` events by reason and `by`,
  plus voids-after-kitchen with reason `dropped` folded from `Amended` events (`amended[].reason`), each row an event
  with its `by`. Two logs, one function, no new event.
- **CHECK.** Native: a `Wasted` with empty `by` is refused; `"reason":"soggy"` → 400 and the stock log's `len()` did not
  move; a decode of a pre-change record folds to the same ledger. Live on a TEST venue: bin 2 portions as
  `dropped`, void one after `PREPARING` with `dropped` → the report shows 3 rows, 2 of them stock events, and
  `available` moved by 2 only.

### 2.2 The till reachable, and cash refused outside it (P1-3, P1-4, P3-2)

- Wire `till.rs`: a `till` LogImage in the object (the `logimage.rs` shape), five routes under `staff.*` behind
  `Cap::OpenTill` (`open`, `count`, `close`, `pay-in`, `pay-out`), the `expected` fold in the object, and a
  `health.till` block. `pay.rs` gains the guard its own header already claims: `method == cash && no open till → 409
  "open the till first"`. `till_id` becomes mandatory for cash.
- **Per-currency from day one.** `Paid` gains `currency` (default the venue's), `tendered` (in that currency's minor
  units), `rate_ppm` and `amount` (venue currency, the number that enters the bill); `amount = tendered × rate_ppm /
  1_000_000` with the rounding rule stated as an equation in the code and a test on the boundary; `Opened.float`,
  `Counted.observed`, `PayIn/PayOut.amount` each carry `currency`; `expected` is a map by currency. Change given in the
  other currency is a `PayOut{currency, reason: "change", ref: order_id}` in the SAME object turn as the `Paid`. The
  rate is an owner setting (`settings` image, the venue's posted board rate); the fiscal document converts at the Bank
  of Albania rate per the law (§5) — two rates, two purposes, both recorded. `convert_all_to_eur_cents(f64)` is not
  used; `rate_ppm` follows `TAX` §3.1's integer basis.
- **Courier hand-in** = `PayIn{source: "shift:<courier_id>", amount, currency}`; the audit checks it equals that shift's
  folded cash (`courier.rs:736-780` is the fold) — `POS` G2 (c).
- **CHECK** (the `POS` G2 prove cases, plus): a EUR 40 tender at `rate_ppm 100_000_000` on a 3,600 lek bill → `amount
  4000`, `Paid` accepted, and a `PayOut{400, ALL, change}` in the same turn; `expected[EUR]` rose by 40, `expected[ALL]`
  fell by 400; law 3 unchanged (a payment never touches `total`); a cash `Paid` with no open till → 409 and both images
  byte-identical; `over_short` recorded per currency and never adjusted.

### 2.3 Tender completeness: tip at payment, wallet as a method, one fiscal document per sitting (P1-3)

- `PayIn.tip: Option<i64>`: the delta raises the round's `tip` and `total` by it in the same `Paid` event (law 3 has a
  tip term since `6965c77d`), so "remainder as tip" is `amount = bill − paid_so_far, tip = remainder` and the Σ ≤ total
  rule holds against the NEW total. Tips stay out of `expected` (the courier's rule, `courier.rs:741-744`).
- `method: "wallet"`: the object holds the ledger image too; the `pay` command appends the wallet's debit leg and the
  `Paid` in one turn, and REFUSES when the folded balance < amount — "a wallet must never go negative" is a refusal at
  the single writer, which is what `ledger_account.rs`'s double entry is for (verify the existing guard first, §5).
- One fiscal document per sitting at close (`Σ paid == bill`), with the payment means listed — the law's per-invoice
  "mënyrën e pagesës" and ebills' `MULTIPLE` (§5). This is `TAX` §3.7's `Document` with `payment: Vec<PayMethod>`
  instead of one, and it is the reason the tender fields above are on `Paid` and not on the till.
- **CHECK.** Native: three payments `cash 2000 + card 1500 + wallet 500` on a 4,000 bill → `payment_status: paid`, three
  `Paid`, one wallet leg, `history` shows them in order; a wallet short by 1 → refused, no leg, no `Paid`; a fourth
  payment → refused. The `Document` for that sitting lists three means summing to the LEK total.

### 2.4 The refused delivery, and ending an accepted order (P1-4)

- The missing piece is the **refund route**, not a stock event: `Refunding` is reachable from `Confirmed`, `Preparing`,
  `Ready`, `InDelivery` (`POS` §1.5) and nothing emits it. `owner::order_action` gains `refund { reason }` →
  `Refunding`, and `compensated` → `CompensatedRefund`; `took_money` already excludes both (`status.rs`), so no fold
  changes (memory: "when the refund route lands, no fold needs changing"). The stock consequence is decided in the same
  command: a round refunded before `PREPARING` `Release`s; after it, the ingredients were consumed, and the courier's
  "returned to kitchen" is a `Wasted{reason: Returned, by: courier}` per BOM line — or, if the owner marks it
  resellable, a `Received` — chosen by the owner, never inferred.
- The courier gets one more tap, `refused_at_door { note }`, which is the same `refund` intent under the courier's
  principal with `cash_collected: 0` and the shortfall rule (`courier.rs:602-604`) unchanged; per-courier cash audit
  (law 4) then sees a refused run as zero cash owed rather than as a run that never ended.
- **CHECK.** Native: `READY → refund` lands, `stranded()` empty after it; `IN_DELIVERY → refund` with `Returned` writes
  N `Wasted` and `available` falls by the BOM; law 2 ("no order is stranded") goes GREEN on a synthetic log that was
  RED before. Live: the ten stuck orders on sushi-durres can be ended through the route, and
  `e2e/kit-regression/drain-stuck-orders.mjs` is deleted the same day (the memo's own instruction).

### 2.5 The exception report and event alerts, without scoring a person (P1-5)

- **Report** (`/api/owner/exceptions?period=`): a pure fold listing EVENTS — voids after the kitchen, comps,
  `PayOut`s, `over_short ≠ 0` periods, cash `Paid` outside a till (should be impossible after 2.2), amendments on rounds
  older than N minutes — each row `{at, order_id, kind, reason, amount, by}`. Grouped by round and by reason. It is
  `Hub::history` across the venue with a filter; it persists nothing.
- **Alert**: the minute cron already drains an outbox; an `Entry{kind: "exception", to: owner chat}` is enqueued by the
  object when a **venue-period count** crosses an owner-set threshold ("3 voids after the kitchen in one till period").
  The alert names the events and their signers; it never carries a per-person total, a rank or a tier.
- **Why this does not collide with OD-8:** the gate refuses identifiers that pair a participant with a judgement
  (`no-scoring.sh` pattern) and the type system refuses ordering people (`caps.rs`, no `Ord`). A list of signed events is
  the record the log already is; an alert on a count of events at a venue is a conservation-style law, not a score. The
  line not to cross is a persisted or sorted number PER PERSON — and the design does not need one, because the owner
  reads names on rows, which is what Toast's Sales Exception Report is (`POS` §4.3).
- **CHECK.** `no-scoring.sh` stays at baseline; a native test that the report's JSON contains no key matching the gate's
  pattern; a prove case that the alert fires at the threshold and names the order ids and `by`.

### 2.6 Print status and "seen" (P1-6)

- Build `LAST-MILE` §3.1 step 1 and step 3 as written: `print` rail, printer polls, `DELETE /api/print/job/:token` is
  the ack → `Verdict::Sent`; console draws queued/printing/printed/failed. Add one thing: a **`seen` ack from the console
  itself** — the kitchen taps the ticket, the console POSTs `Noted{seen_by, at}` through the existing `order_action`
  idempotency; the owner console shows "sent 18:02 · seen 18:03 · printed 18:02" per round. "Sent" then means three
  things and each is a record.
- **CHECK.** Native: an entry whose `DELETE` never arrives is `failing` on health after the first backoff and `Abandon`
  after six; a round with no `seen` within N minutes is a row on the exception report (2.5). Live: pull the paper, ring
  an order, watch the console's `failed` column.

### 2.7 Floor states as folds, plus one mark (P3-1)

- `free` = no sitting live and no booking `holds_table` for this minute; `booked` = `holds_table`; `ordering` = a live
  sitting with its newest round at `PENDING|CONFIRMED`; `waiting` = at `PREPARING`; `paying` = every round `took_money`-
  eligible and `Σ paid < bill` with the newest round past `READY`; `dirty` = `Σ paid == bill` and no `Noted{cleared}` yet.
  Only the last needs a write: `Noted{cleared_by, at}` on the sitting's last round (an order event, already a kind).
  The plan is `tables.rs`'s geometry; the state is `sitting.rs`'s fold; the console overlays one on the other. The
  ebills floor poll (`EBILLS` §3.2 (5)) is a second source for the same picture while the venue rings up on ebills.
- **CHECK.** A native test that the six states are a total, disjoint function of `(bookings, rounds, now)`; a prove
  case that a table with a paid sitting and no `cleared` is `dirty`, and `free` one event later.

### 2.8 The fiscal outbox with a deadline (P1-1) — for when B8 opens the push

- Each `Document` (`TAX` §3.7) is an `outbox::Entry{kind: "fiscal", id: uuid, deadline_ms: issued_at_ms + 48 h}`. The
  drain sends by `issued_at_ms` order but **does not head-block**: a refusal (a `problem+json` from the platform) is
  `Verdict::Abandon` + an exception row + the corrective path (a second `Document` referencing the first — the art. 32
  shape and `TAX` §3.7 rule 4's negation form); the entries behind it continue. A retry re-sends the same `uuid`
  (`TAX` §3.7 rule 6).
- **Conservation law N:** every order with `took_money` at a venue with fiscalisation configured has a `Noted{fiscal}`
  OR an outbox entry whose `deadline_ms` is in the future; an entry past its deadline is a breach with the order named.
  `health.fiscal = { backlog, oldest_issued_at, first_deadline }`.
- The receipt printed locally during an outage carries every art. 29 field except the NIVF and says so ("pa NIVF");
  the fiscalised codes are printed on the copy once `Noted{fiscal}` exists (`TAX` §3.8).
- **CHECK.** Prove cases: three entries, the middle one refused → two `Sent`, one exception row; an entry 49 h old and
  unsent → law RED with its order; the same `uuid` sent twice → one invoice (the mock counts).

### 2.9 Aggregator channel (P2-3) — `channel: wolt|glovo|baboon`

- Inbound: an adapter per platform under `services/ordering/` producing a `place` command with `channel`,
  `price_trusted: false`, the platform's order id in `external{}` (`EBILLS` §3.2 (1)'s block, reused), born `CONFIRMED`
  when the platform requires acceptance in-app first. Stock is reserved on the same ledger; an `OutOfStock` refusal
  answers the platform's webhook with a rejection AND enqueues an availability update (Wolt Menu API / Glovo Partners
  API) — the automatic 86 propagated. Outbound status maps the FSM's edges to the platform's; `Rejected` after
  acceptance is the case the platform charges for, so the console asks a reason and the exception report counts it.
- **CHECK.** A native test that a `wolt` order's `total` is never recomputed by dowiz's pricer (`price_trusted:false`
  respected); a prove case that an `OutOfStock` refusal produces exactly one availability entry per SKU.

### 2.10 Cost that follows purchases (P2-4, P3-4) — after recipes

- `Received { item, qty, unit_cost: Option<i64>, supplier: Option<String>, doc: Option<String> }`; **weighted-average
  cost** as a fold over `Received` (a running average is a fold; FIFO needs lots and is refused for now, §4); the dish's
  cost at placement is STAMPED on the line the way the tax rate is (`TAX` §2.6), so last March's margin shows last
  March's cost. Supplier debt, when asked for, is an account in the kernel's double-entry ledger, not a counter.
- **CHECK.** Two receipts at 1,000 and 1,200 per kg → WAC 1,100; an order placed between them stamps 1,000; law 8's
  rebuild reproduces the stamped costs.

### 2.11 The local decider (P2-5, Q2) — the one architectural item

- Move `command::{place, amend, pay, advance}::decide` and `room_rules` into `dowiz-hub` (they are pure over `Value`
  and hub types; `STACK` §4.3 item 1 measured the storage stack at 86 KB of wasm), expose them through
  `crates/bebop-wasm`'s module beside the reader, and let a waiter surface (which does not exist yet) fold the object's
  last image locally, predict with the SAME functions, and queue intents in `outbox.js`. The residual conflict — stock
  sold twice during the outage — is refused at replay (the `OutOfStock` 409 the outbox already treats as an answer) and
  shown to the waiter as "this was sold online while we were offline"; compensation is the owner's act, not a merge.
- **CHECK.** A gate in the shape of `WASM` §9 item 1: the same amend applied by the Worker, the object and the wasm
  module over the same image yields byte-identical deltas (three deciders, one number); a replay test where two
  offline tablets each sold the last portion → exactly one `Reserved`, one 409, nothing stranded.

---

## 3. Order of work, merged into roadmap phases A-D

| Roadmap row | Item here | Why this position |
|---|---|---|
| **A7** (was IN FLIGHT; is "written, unreached") | 2.2 wire the till; cash refused outside it; per-currency fields from day one | The till is the precondition of P1-3, P1-4's hand-in and P3-2; adding `currency` later is a format change every reader must re-derive (`CLAUDE.md`, "a change to a written format") |
| **A6** (second half) | 2.3 tip at payment, `wallet` method, one `Document` per sitting | Rides on `pay.rs` as landed; the fiscal document shape is `TAX` §3.7 |
| **new A10 — the refund route** | 2.4 | Named missing since `dowiz-order-fsm-has-no-exit`; P1-4 and the stock hold are the same defect; no fold changes |
| **new A11 — write-off signer + waste report** | 2.1 | Small; blocks nothing; measures nothing until recipes exist, so it is paired with the operator entering the first 20 recipes (`LAST-MILE` §3.3) |
| **new A12 — exceptions report + event alerts** | 2.5 | Needs `by` on till/waste events (A7, A11) to be complete |
| **A8** (first half) + **new A13 — seen/printed** | 2.6, on `LAST-MILE` §3.1 steps 1 and 3 | The print rail is the LAST-MILE lane's item 1 already |
| **A2/A3** (tables lane) + **new A14 — floor states** | 2.7 | Folds over what A2 and the room already store; one `Noted` |
| **B6/B7/B8** | 2.8 fiscal outbox with deadline + law | Only when the operator opens the push scope; the law's 48 h is the number the outbox carries |
| **B5** (channel) + **new B9 — aggregator adapters** | 2.9 | After `channel` is a closed set (B5); Wolt first (present in Durrës) |
| **new E1 — cost follows purchases** | 2.10 | After recipes; before any procurement feature |
| **D2** | mark LANDED (`1b978569`) | correction |
| **new D7 — decider in the hub, in wasm** | 2.11 | The one item that changes an architectural score (`local-first writes 0`, `dowiz-architecture-evolution`); it is a refactor of crate ownership, and it is what makes an offline room lawful AND correct |
| **C3** | route for `Hub::redact` + the archive path + G3's live probe | the hub half landed (`8d7cb62a`); the route is the missing half |

**Gates first** (`gates-must-improve-not-degrade`): the till law (2.2's CHECK, `POS` G2), the fiscal deadline law (2.8),
and the "no key matching the no-scoring pattern in any report JSON" test (2.5) are written and proved red before their
features, in the `conservation.prove.mjs` shape.

---

## 4. Recommended AGAINST, and the condition to reopen

| | Against | Why, here | Reopen when |
|---|---|---|---|
| 1 | **An anomaly score per staff member** | OD-8 / `no-scoring.sh` / no `Ord` on people; a score is a rating of a participant however it is computed. 2.5 meets the need with events and venue-level counts | never as a score; a venue asks for a per-person threshold and the operator rules OD-8 differently |
| 2 | **Manager PIN prompts** (`POS` §5 J) | trust is the cap on the signer's own token; a PIN re-signs a cap on a shared device and is a screen, not a data model | one shared tablet at the pass — likely at this venue; then a PIN that re-mints a staff token for the tap, never an `approved_by` field |
| 3 | **CRDTs for the offline till** | `DECISIONS.md:369`; `STACK` §4; `POS` §5 D; Q2 above — convergence without refusal is the failure mode for money | two devices amending one tab with no connectivity between them AND a product decision that the merge must not refuse; even then, non-money fields only |
| 4 | **Crypto-shredding now** (P2-2) | no cipher in the hub; every `contact` reader would decrypt (13 files); history is already in clear; the key image is itself under the 30-day PITR. B keeps the witness's tips and costs no reader change (`CRM` §2.2) | a copy of the log the venue cannot rewrite within a month: a peer replica, an object-locked archive, a shared ledger across venues — build C on top of B then |
| 5 | **FIFO lots / batch expiry** | needs lots on `Received` and a lot on every `Consumed`; the venue has 0 recipes; WAC is a fold and answers "margin per dish" | a venue that must trace a lot for a recall, or a central kitchen (P2-1) |
| 6 | **Inter-venue transfers and a central kitchen** | one venue; a transfer is a two-object saga (an `outbox` entry that becomes a `Received` command at the other object, with a platform-level Σout == Σin law) — designable, not needed | the second venue of one owner |
| 7 | **Labour cost %** | the rate is payroll policy per venue and country (`POS` §5 E's reasoning for tips); hours are a record (`Clocked{in\|out, by, at}` in the till's LogImage), the percentage is the owner's spreadsheet | a venue supplies its rule as an equation over integers |
| 8 | **A course-pacing engine on the KDS** | rounds are tickets already; a `course` label on the round plus "hold until fired" (`CONFIRMED` held, `PREPARING` is the fire) covers the sushi venue; timers are client-side | a venue that serves tasting menus |
| 9 | **ESC/POS drivers and LAN printing first** | `LAST-MILE` §3.1: the polling printer needs no page, no agent, and its ack is a request we receive | a venue with a printer and no fixed line (then Web Bluetooth on Android, step 2) |
| 10 | **A fiscal push before the till and the tax block exist** | `TAX` §3.7 rule 1: no `tax` block → `NoTax`; a document from an untilled cash payment cannot list its means | B1-B6 landed; the operator opens the write scope on ebills |

---

## 5. What could NOT be verified, and the legal record

### 5.1 Code facts not verified (each with the command that settles it)

| Claim | Status | Settle with |
|---|---|---|
| A wallet debit beyond balance is refused today | **hypothesis.** `grep -n "negative\|insufficient" workers/api/src/wallet.rs crates/dowiz-core/src/ledger_account.rs` → only an overflow refusal at `ledger_account.rs:575` (measured); the balance check may live under another name | read `ledger_account.rs`'s debit path, or a native test that debits balance+1 |
| `till.rs` is counted by `unreached.py` | **unknown.** `tools/gates/unreached.baseline` reads `0`, yet no caller exists outside the file; the gate may not see `command/` — or it may be about to fail on the next run | `python3 tools/gates/unreached.py` (it ran past 120 s here and was stopped; do not run it beside a build) |
| Archive images are redacted too | **not found.** `grep -rn redact workers/api/src/hubstore.rs` → nothing; `CRM` §3.3 names "one on the archive path" | the CRM lane's commit for the route |
| Readers other than `dowiz-hub` understand `kind \| 0x80` and kinds 7/8 | **not verified.** Only `crates/dowiz-hub/src/lib.rs:823` masks `REDACTED_BIT` (measured); `crates/bebop-wasm`, `tools/native-spa-server` and the Python reader name neither. A reader built before `8d7cb62a` quarantines kind 7/8 records — law 6 goes red, correctly (`POS` §3.3) — but a reader that masks nothing may misclassify a redacted `Placed` as kind 129 | `grep -rn "0x7f\|REDACTED" tools/native-spa-server/src crates/bebop-wasm/src bebop-lang/tools/*.py`; run the four-way gate on an image with one redacted record |
| Any live order carries a tip, a comp or an amendment yet | **unknown**; the room routes landed on 2026-09-23 | `node e2e/gates/conservation.mjs` |
| Wolt/Glovo/Baboon terms for Albania | **not read**; the compensation clause cited is Wolt's Poland addendum | the operator's merchant contract |
| Bolt Food operates in Durrës | **hypothesis** (one directory page says "growing" in Albania; no Durrës claim found) | `https://food.bolt.eu` city list |
| How ebills.al behaves when the venue's internet is down — the venue's ACTUAL offline receipt path today | **unknown, and it is the most important operational question in P1-1**: a cloud POS needs the line; whether it prints art. 29 receipts offline and back-fills, or the staff hand-write, decides how urgent 2.8 is | ask the staff; or read ebills' SPA for an offline mode (`EBILLS` §1.1 fetched the bundle) |
| The spec's field names `IsSubseqDeliv`, `SubseqDelivType` (values such as `NOINTERNET`, `BOUNDBOOK`), `CorrectiveInv`/`IICRef`, `PayMethods`, `Currency/ExRate` | **hypothesis for Albania.** The Albanian technical spec PDFs (v01-v06, English and Albanian) are linked from the tax authority page below but could not be text-extracted on this box (glyph-encoded; `pdftotext`/`pypdf` absent, measured). The names were confirmed only in Montenegro's twin specification (same vendor lineage), and `EBILLS` §1.5 measured `currencyRate`, `sellRate/buyRate`, `exchange`, `iicReference` and a `MULTIPLE` payment method on ebills' wire, which are those fields' shadows | download `shkarko.php?id=3514` on a machine with `pdftotext` and grep the names |

### 5.2 The legal record (Albania), with sources

- **Law No. 87/2019 "Për faturën dhe sistemin e monitorimit të qarkullimit"** (18 Dec 2019). Text read from a PDF copy
  hosted by HAT Finance (streams inflated and grepped here; article numbers from the extracted headings):
  [Ligj_Nr_87](http://hatfinance.al/images/dokumenta/Ligj_Nr_87__Per_Faturen_dhe_Sistemin_e_monitorimit_te_Qarkullimit_HAT_FINANCE.pdf).
  Official page: [tatime.gov.al — Fiscalization](https://www.tatime.gov.al/eng/c/320/fiscalization).
  - **Art. 29 "Ndërprerja e lidhjes elektronike (internetit)"**: p.1 — on an internet interruption the taxpayer issues
    the invoice with every element of chapter IV **except the NIVF** and prints it on paper; p.2 — "brenda 48 orëve,
    duke filluar nga momenti i [lëshimit]…" the invoices issued under p.1 are sent to the central tax administration by
    a special protocol; p.3 — invoices so issued are considered correctly fiscalised if the process is completed within
    the deadline. (The extracted text has a gap after "momenti i"; vendor pages say "48 hours after connectivity is
    restored"; the tax authority's FAQ says both "from the interruption" and "no later than 48 hours from issuing an
    invoice without NIVF". **Treat the stricter reading — per invoice, from issue — as the design number.**)
  - **Beyond 48 h**: per the tax authority's FAQ, if reconnection is impossible within 48 h the taxpayer informs the
    administration with evidence and may continue issuing without NIVF until restored, then fiscalises everything
    ([tatime.gov.al FAQ](https://www.tatime.gov.al/c/424/444/pyetje-pergjigje-mbi-fiskalizimin);
    [logic.al summary](https://www.logic.al/pyetje-pergjigje-mbi-fiskalizimin/)).
  - **Art. 30 (number inferred from ordering; heading not extracted)**: a fiscal device that stops working must be
    repaired or replaced "brenda pesë ditëve", and the invoices issued meanwhile sent within 5 further days.
  - **Art. 31**: fiscalisation in zones where no internet connection is possible (a declared regime, not an outage).
  - **Art. 32 "Fiskalizimi i faturave korrigjuese/saktësuese"**: corrective invoices follow the VAT law; the
    2026-08-27 clarification: "Registered invoices cannot be deleted or cancelled. Corrections require a fiscalised
    corrective invoice that references the original document"
    ([VATupdate](https://www.vatupdate.com/2026/08/27/albania-clarifies-nine-common-misstatements-about-its-fiscalisation-regime/)).
  - **Currency** (chapter IV, invoice content; art. 23 for cash invoices lists "mënyrën e pagesës (kartëmonedha, kartë,
    çek), monedhën dhe kursin e këmbimit, nëse fatura nuk është e shprehur në LEK"): "Vlerat e treguara në faturë mund
    të shprehen në çfarëdo lloj monedhe, me kusht që vlera totale, vlera e tatueshme dhe vlera e pagueshme e TVSH-së të
    shprehen në … LEK, sipas kursit të këmbimit të publikuar në faqen zyrtare të Bankës së Shqipërisë" (extracted).
    Bank of Albania official rate: [bankofalbania.org](https://www.bankofalbania.org/Markets/Official_exchange_rate/).
    A EUR tender at a Durrës table is therefore a payment MEANS on a LEK invoice with the rate stated — not a EUR invoice.
  - **Payment means per invoice** are enumerated in the same article (banknotes and coins, card, cheque, bank
    transaction, payment order, electronic money, other non-cash means) — a list, which is why one invoice may carry
    several (ebills' `MULTIPLE`).
  - **Penalties**: Law No. 83/2022 from 1 Jan 2023; a repeat failure to issue a fiscalised invoice is quoted at 50,000
    lek for a non-VAT sole trader ([sherbimekontabiliteti.al](https://sherbimekontabiliteti.al/en/fiskalizimi-albania/) —
    a vendor guide; the amount is not verified against the law).
  - **Technical specification** (Functional and Technical, v01-v06, alb/eng):
    [tatime.gov.al technical specifications](https://www.tatime.gov.al/eng/c/320/327/fiscalization-service-technical-specifications);
    v07 change notes (IsBuying optional for currency exchange; IsEinvoice) per the search index, not read.
    Field names: Montenegro's twin spec
    [Fiskalni servis v02](http://poslodavci.org/site/assets/files/2937/fiskalni_servis_-_tehnicka_specifikacija_v2.pdf)
    confirms `SubseqDelivType`, `TypeOfInv` (cash/non-cash), `CorrectiveInv`, `IICRef` — hypothesis that Albania's names
    match.
  - Vendor summaries consistent with the above: [SNI](https://snitechnology.net/new-fiscalization-regulation-in-albania/),
    [EDICOM](https://edicomgroup.com/electronic-invoicing/albania),
    [Pagero/Thomson Reuters](https://europe.thomsonreuters.com/compliance/regulatory-updates/albania).

### 5.3 The market record (aggregators)

- **Wolt** entered Albania in 2024 (Tirana, 130+ restaurants) and **Durrës in March 2025**, "covering the entire city
  area" ([Reuters via TradingView](https://www.tradingview.com/news/reuters.com,2025-03-20:newsml_SEE8GBLLa:0-wolt-expands-to-albania-s-durres/),
  [SeeNews](https://seenews.com/news/wolt-expands-to-albanias-durres-1272591),
  [Wolt Durrës](https://wolt.com/en/alb/durres), [Wolt newsroom](https://press.wolt.com/en-WW/236056-wolt-enters-luxembourg-and-albania-now-operating-in-27-markets-across-emea/)).
  Integration: [Order API](https://developer.wolt.com/docs/api/order), [Menu API](https://developer.wolt.com/docs/api/menu),
  [getting started for restaurants](https://developer.wolt.com/docs/getting-started/restaurant). Merchant-attributable
  cancellation → compensation deducted; missing items → a per-order deduction (Poland addendum, hypothesis for Albania:
  [Wolt Poland marketplace addendum](https://merchant.wolt.com/en/pol/marketplace-product-addendum)).
- **Glovo**: dominant in Tirana, "limited expansion to Durrës and Vlorë" ([Nomada](https://nomada.tools/directory/food-delivery/albania));
  [Glovo Partners API](https://api-docs.glovoapp.com/partners/index.html).
- **Baboon** (Albanian): Tirana, Durrës, Vlorë ([Google Play](https://play.google.com/store/apps/details?id=al.baboon),
  [SKAN.AL](https://skan.al/blog/online-food-ordering/)). No public API found.
- **Bolt Food**: named as "growing" in Albania by one directory; Durrës presence not established (hypothesis).

### 5.4 The judgement asked for on P2-2, in five lines

Redaction-in-place (`forget.rs`) is the right choice for THIS log because its integrity is a witness on `(records, tip)`
over an FNV chain, not a cryptographic commitment (`CRM` §1.2): keeping `id`/`prev` keeps every tip the witness ever
sealed, and declaring the tombstones makes the one place the chain stops vouching for content a counted, audited fact
(`declared == redacted`). Crypto-shredding would buy dead ciphertext in the dated S3 copies — the one thing B cannot
reach — at the cost of a cipher the hub does not have, decryption in thirteen reader files, and a key image that the
same 30-day PITR restores; and it does nothing for the orders already in clear. B now, C when a copy exists that the
venue cannot rewrite within the statutory month, and never C instead of B.
