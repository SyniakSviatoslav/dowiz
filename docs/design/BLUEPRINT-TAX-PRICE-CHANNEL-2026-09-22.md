# Tax, price per channel, and the channel itself: what the order path carries today, what it must carry, and the seam a fiscal integration plugs into

**Date:** 2026-09-22. **HEAD read:** `b949528e` ("dine-in: an order placed at a table", 2026-09-22).
**Tree state at time of reading:** `git status --short` = 12 lines — 6 modified (`crates/dowiz-hub/src/lib.rs`,
`workers/api/src/booking.rs`, three `public/store/*` files, `.impeccable/config.json`) and 6 untracked, among them
`docs/design/BLUEPRINT-EBILLS-INTEGRATION-2026-09-22.md` and `workers/api/src/ebills/` (`mod.rs` 389 lines,
`tests.rs` 323 lines, by `wc -l`). Two other lanes were writing while this was read; line numbers in
`booking.rs` and `dowiz-hub/src/lib.rs` are from the working tree. Nothing cited below lives in a file those lanes own,
except the ebills document, which is quoted as a document.

**Method.** Every "today" statement names a path and the line where it was read. `measured` means the command was
run on this box on 2026-09-22 and its output is quoted; `hypothesis` means it was not. Four of the brief's premises
were re-run rather than believed, and §0 says which held. External facts (tax law, other vendors) carry a source in
§9; where a source is a vendor's blog rather than a statute, it is marked as such. The ebills lane's document was
read in full and is built on, not repeated: the wire protocol, the poller and the inbound mapping are that lane's;
what this document owns is the money law, the price model, the channel field and the OUTBOUND seam.

---

## 0. The four premises, re-run

| Brief said | Measured | Verdict |
|---|---|---|
| 1. "There is no tax on the order path at all. `tax_rate` exists in `json_bridge.rs:239`, but the Worker never calls it." | `grep -rn json_bridge workers/api/src` → nothing. `grep -rn 'apply_tax(\|compute_order_total(\|estimate_order_total(' workers/api/src` → nothing. The envelope is `delivery_fee`, `tip`, `total` (`storefront.rs:1017-1019`), then `discount`, `promo`, `total` patched in the object (`command/place.rs:147-149`). | **Holds** — and understates it. A complete tax law exists one crate down (`money.rs:267` `apply_tax`, `domain.rs:207` `compute_order_total`, `money.rs:404` `estimate_order_total`) with a GENERATED integer twin (`eqc_gen.rs:30,50`) and an exact-parity test (`money.rs:565`). Its only reachable consumer is `kernel/src/wasm.rs:53,61` via `json_bridge`, which is served to no browser (ARCHITECTURE-EVOLUTION §1.5). The law is not missing; it is unreachable, and its entry point takes an `f64` (§1.2). |
| 2. "One price per dish, three channels … in Albania dine-in and takeaway are taxed differently, so this is a wrong figure." | One price: `catalog.rs:104-112` stores the product JSON; `pricing.rs:126` reads `p.price` and nothing else; `fulfilment::fee` (`fulfilment.rs:81-87`) is the only thing that differs by kind. The Albanian claim: standard 20 %, reduced 6 % only for accommodation, agritourism restaurant services excluding drinks, books, a few named others ([tvsh.al], [openaccountants]); the ebills lane MEASURED `"vat":"VAT_20"` on a coffee served at table 12 (EBILLS §1.5). Germany had the dine-in 19 % / takeaway 7 % split and abolished it on 2026-01-01 ([VATupdate]). | **One price per dish holds. The tax premise is WRONG for Albania today**: dine-in and takeaway are both 20 %. Price-per-kind is still worth building — as a COMMERCIAL fact (§2.5), not a tax fact — and the rate must still live on the LINE, because rates differ by item class (drinks at agritourism venues; ebills' own vocabulary is `VAT_0|VAT_6|VAT_10|VAT_20`, EBILLS §1.5) and change by date. |
| 3. "There is no `channel` on an order." | `storefront.rs:929-935`: `json_api::place_order_at(id, None, lines, created_at_ms, Some("storefront".into()))`. The kernel stores it (`domain.rs:112` `channel: Option<String>`, re-emitted at `json_api.rs:170`); the delta fold knows it as one of the kernel's explicit nulls (`fold.rs:33-36`). The ebills mapper writes `"channel": "ebills"` (`ebills/mod.rs:261`). | **Wrong in letter, right in substance.** Every storefront order carries `channel: "storefront"` today. What does not exist: a closed set (`known()`), a reader (`channel::of`), a compatibility rule for absence, any second route that sets it, anything that READS it (`grep -rn '"channel"' workers/api/src` outside the messaging files → only `ebills/`), and the set that a marketplace or a console order would need. |
| 4. "No receipt and no fiscalisation." | `grep -rn -i 'tvsh\|\bvat\b\|fiscal\|escpos\|receipt' --include=*.rs workers/api/src crates kernel/src` → only `ebills/mod.rs` (untracked, this session) and two comments in `services/catalogue/media.rs:51,124`. The closest thing to a receipt is the kitchen ticket `notify::order_text` (`storefront.rs:1121-1126`) and the owner's client-side CSV (`public/admin/orders.js:60`). | **Holds.** The venue's fiscal receipts are printed by ebills' own terminal (`pointOfSale.printing: "THERMAL", printerSize: "SIZE_80"`, EBILLS §1.5), which is why §4 recommends against ESC/POS here. |

One more fact the brief did not have: **"money is exact integer arithmetic, currency-typed, overflow-checked, zero
floats EVER" is the header of `money.rs:1`, and lines 267 and 272 of the same file multiply an `f64`.** §1.2.

---

## 1. Ground truth

### 1.1 The tax law that exists, and who can reach it

| Function | Where | Signature | Reached from |
|---|---|---|---|
| `apply_tax` | `crates/dowiz-core/src/money.rs:267` | `(subtotal: i64, tax_rate: f64, price_includes_tax: bool) -> Result<i64, String>` | `domain.rs:212`, `money.rs:409`, `temporal_tmr.rs:243`, its own tests |
| `compute_order_total` | `domain.rs:207` | `(subtotal, tax_rate: f64, price_includes_tax, fee: Option<i64>)` — "tax on the subtotal only (no discount)" (`domain.rs:201`) | `Order::recompute_total` (`domain.rs:186`), tests |
| `estimate_order_total` | `money.rs:404` | over `OrderTotalConfig { fee, tax_rate: f64, price_includes_tax, min_order_value }` (`money.rs:361-366`); "Tax is on the subtotal (not subtotal+fee)" (`money.rs:402-403`) | `json_bridge.rs:243` only |
| `apply_tax_exclusive_int`, `apply_tax_inclusive_int` | `eqc_gen.rs:30,50` | `(sub: i64, rate_micro: i64) -> Result<i64, &'static str>` — **already integer** | the parity test only (`money.rs:565-657`); "the authority flip (callers switching to them) is W4-L1, operator docket R-4 — NOT done here" (`eqc_gen.rs:18-20`) |

Measured: `grep -rn 'apply_tax\|compute_order_total\|estimate_order_total\|OrderTotalConfig' workers/api/src` →
nothing. The Worker's total is `subtotal + fee + tip` (`storefront.rs:1016`), rewritten as
`subtotal − cut + fee + tip` when a promo is redeemed (`place.rs:149`). The stored envelope's `total` is the
number Stripe is handed (`storefront.rs:1154-1157, 1270`).

### 1.2 The rate is a float, and the float is (today) harmless — measured

`money.rs:272`: `let rate_micro = crate::math::round(tax_rate * 1_000_000.0) as i128;` — an `f64` multiply and a
hand-written `round` (`math.rs:214-226`, trunc/fabs on `f64`) inside the file whose first line is the red line.
`domain.rs:189,208` and `json_bridge.rs:239` (`field_f64(&v, "tax_rate")`) carry the same `f64` to the boundary,
where it is parsed from a JSON decimal (`"tax_rate":0.20`, `json_bridge.rs:528`). `MANIFESTO.md:18` C5 says
"Integer-only money (`i64` minor units, no `From<f64>`)"; `CLAUDE.md:98` says "zero floats, ever".

**Does the float produce wrong numbers?** Measured (Python, same IEEE-754 double as Rust `f64`, `math::round`
re-implemented line for line): for every basis-point rate `k/10000, k ∈ 0..=10000` and every per-mille rate,
`round(rate * 1e6) == k*100` — **0 failures**. So today's conversion is lossless for every rate a tax authority
publishes; the parity test's own grid (`money.rs:626-628`) relies on the same round trip. The defect is not a wrong
figure. It is (a) a type on the wrong side of a line the repo says is absolute, (b) a rate that enters as a
decimal literal and so has to be converted at all, and (c) the generated twin — which is the deterministic,
target-independent artefact — being the SHADOW while the float law is the authority. §3.1 flips that.

f64 tokens on the money path, measured (`grep -c f64`, non-test by `awk` up to `#[cfg(test)]`): `money.rs` 3
(lines 267, 332 `convert_all_to_eur_cents`, 363), `domain.rs` 9, `services/ordering/*.rs` 0, `command/*.rs` 0,
`storefront.rs` 0, `json_bridge.rs` 46 (the whole JS bridge), `temporal_tmr.rs` 1. Those are the baselines §5's
gate starts from.

### 1.3 One price, one pricer, and what the price already knows

`pricing.rs:101-104` `price_basket(lookup, want) -> Result<Basket, Refusal>`: per line `unit_price = (price +
chosen.delta).max(0)` (`:136`), `subtotal += unit_price * quantity` (`:137`). The browser's own `unit_price` "is
discarded before it can be believed" (`:87-89`). There is no argument for the fulfilment kind and no field for a
rate. `Line` carries `product_id, name, modifier_ids, quantity, unit_price` (`:28-39`) — the name is a SNAPSHOT
"because the catalogue is not versioned per order" (`:30-32`), which is exactly the historical-correctness rule §2.6
asks for, already applied to one field.

The venue's money settings are on the `location` record, not in the settings image: `free_delivery_threshold`,
`min_order`, `currencyCode` (`storefront.rs:145-149`, written by `owner.rs:1393-1399`); `delivery_fee` likewise
(`hubstore.rs:1351`, `platform.rs:236`). The hub's declared settings keys (`crates/dowiz-hub/src/settings.rs:169-336`)
are `ai.*`, `social.*`, `notify.*`, `cloud.s3.*` — **no tax key**, and "THE KEY SPACE IS CLOSED … an unknown
setting is refused" (`services/venue/settings.rs:3-5, 62-64`). The currency is `ALL` when unstated
(`services/venue/mod.rs:19-30`), and **lek has no minor unit**: `money_text(1500, "ALL") == "1500 ALL"`
(`notify.rs:227-228`); the kit once drew 1500 lek as `$15.00` (`public/kit/data.js:56-63`). Every rounding in this
document is therefore to a whole lek, which makes the tie cases of §3.2 common rather than academic.

### 1.4 The channel that exists

Kernel: `Order.channel: Option<String>` (`domain.rs:112`), free text; test values `"web"` and `"app"`
(`json_api.rs:322, 437`); an analytics `orders_by_channel` in the kernel (`analytics.rs:97`) reached only through the
unserved bridge (`json_bridge.rs:183`). Worker: `Some("storefront")` on the one placement route
(`storefront.rs:934`); `"ebills"` in the unwired mapper (`ebills/mod.rs:261`). It survives status transitions
because the kernel re-emits it, not because `HUB_OWNED` lists it (`hubstore.rs:1322-1367` — it does not; the list
is for fields "the KERNEL … knows nothing about", `:1310`). Nothing in `workers/api/src` reads it.

### 1.5 The precedent for a closed set with a compatibility rule

`fulfilment.rs` is the template this document copies: `KINDS` (`:39`), `known` (`:41`), `needs` (`:56`), `fee`
(`:81`), and `of(envelope)` — "ABSENT MEANS `delivery`, and that is a compatibility rule with a date on it … orders
restored from an archive or an old backup may not [carry a kind]" (`:91-103`), tested at
`fulfilment/tests.rs:66` (`an_order_from_before_this_existed_is_a_delivery`). Its JS twin `public/lib/fulfilment.js`
is a hand copy and says so: "the fulfilment kinds live in the WORKER, not the kernel, so there is nothing for
[`gen-vocab`] to read yet. Until `KINDS` moves into `dowiz-core`, this file is a hand copy" (`:11-16`). The
kernel's own `Fulfilment` enum has two variants, `Delivery { address, fee }` and `Pickup { code }`
(`domain.rs:68-76`) — no `dine_in`; the Worker carries the third kind in the envelope beside the kernel's order
(`storefront.rs:940-941`).

### 1.6 Where the discount is decided, and why that decides where the tax is

`command/place.rs:136-150`: the object parses the envelope, redeems the code against `input.subtotal`, and patches
`discount`, `promo`, `total` into the envelope IN THE SAME TURN as the append (`:153`). The header is explicit:
"PRICING STAYS IN THE WORKER AND ONLY THE TAIL MOVES … recomputing them here would be a second pricer"
(`:46-51`). `Promo::discount` floors a percentage — `(subtotal * value) / 100` in `i128` (`promo.rs:200-208`).
Whatever computes tax on a discounted base must therefore run where the discount is known: inside `decide`, after
line 146 — or the tax stored on the order is the tax of a basket the customer did not pay for. §3.4.

### 1.7 The log, and the one property it gives this design for free

`bebop-store/src/evlog.rs:1-12`: "An append-only, content-addressed event log … Each record is its own object
carrying an object-relative ref to its predecessor"; cells 3..10 of every record are its content-id and the
previous record's content-id (`:17-20`). `Hub::append(kind, order_id, order_json, seq, actor_pubkey)`
(`dowiz-hub/src/lib.rs:364-371`). An order envelope, once appended, cannot be edited without breaking the chain
that the conservation audit walks. So "the rate the order was charged at" is a fact the order can CARRY and the
chain can PROVE, which is a stronger guarantee than any effective-dated rate table can give (§2.6).

### 1.8 The delta trap, named once

Every status transition stores `delta(old, merged)` where `merged` is the kernel's order plus exactly the
`HUB_OWNED` keys; "anything not on this list is ERASED by the next status change" (`hubstore.rs:1308-1313`).
`payment_intent`, `amount_received` and `stripe_event` were each lost this way once (`:1327-1338`). **A `tax`
block, a `fiscal` block and per-line `vat_ppm` all have to be on that list**, or the first "confirm" deletes the
figures the receipt was issued from. Per-line fields survive because `"items"` is carried whole (`:1350`).

---

## 2. Research

### 2.1 A rate without floats: what the integer basis has to be

Three integer bases are in use in real systems; the question is which one loses nothing for every rate a venue
on this platform could face.

| Basis | 20 % | 5.5 % (FR reduced) | 8.875 % (New York City combined sales tax) | Verdict |
|---|---|---|---|---|
| per-mille (‰) | 200 | 55 | 88.75 — **not an integer** | rejected |
| basis points (1/10 000) | 2 000 | 550 | 887.5 — **not an integer** | rejected; US combined rates have three decimals of a percent |
| parts per million (1/10⁶) | 200 000 | 55 000 | 88 750 | **chosen** |
| rational `(num, den)` | (1,5) | (11,200) | (71,800) | exact, but a second arithmetic (gcd, cross-multiplication) in a zero-dependency `no_std` crate for no case ppm fails on |

ppm is also **what this repo already uses twice**: `MONEY_SCALE_MICRO = 1_000_000` (`money.rs:19`) and the generated
organs' `rate_micro: i64` (`eqc_gen.rs:30,50`) — and `rates.rs:19-24` for FX: "INTEGER PARTS PER MILLION, not a
float. The rate arrives as `0.0109` and is stored and shipped as `10900`". Choosing ppm means the money law's
representation does not change; only its entry point does.

**Wire and config form.** The rate is an integer everywhere, including the boundary: `"vat_ppm": 200000`, never
`"tax_rate": 0.20`. A JSON integer parses to `i64` with no intermediate float; a JSON decimal parses to `f64` and
then has to be converted — the step that is lossless today (§1.2) and unnecessary tomorrow. Display derives
`"20%"` from `200000` (`ppm / 10_000` and a remainder), never the reverse.

### 2.2 Where to round — per line, per rate group, or per invoice

What the law and the fiscal formats actually say, as far as could be read:

- **HMRC VAT Notice 700 §17.5-17.6** (the only rounding rule found written as a rule): the VAT on an invoice may be
  rounded to a whole penny (down, for invoice traders); if VAT is calculated at line level or per rate the
  rounding must be arithmetic (up and down), not down; the concession does not extend to retailers' schemes
  ([GOV.UK]). The EU VAT Directive does not fix a rounding rule; member states do, and most permit invoice-level
  rounding (hypothesis from the HMRC text and general practice; no second statute was read).
- **The Albanian fiscal message** carries THREE levels at once. From the technical specification's own index
  (search snippet of the tatime.gov.al PDF, the PDF itself refused the connection twice — §7): "3.1.72 Invoice
  Items I UPB (Item Unit Price Before VAT) … 3.1.92 Invoice SameTaxes SameTax PriceBefVAT". From the sister
  specification of the same vendor family (Montenegro, 100 pages, fetched and inflated on this box — the XSD is
  embedded): per item `UPB`, `UPA`, `R`, `PB`, `VR`, `VA`, `PA` (unit price before/after VAT, rebate, price
  before VAT, rate, VAT amount, price after VAT); per rate group `SameTax { NumOfItems, PriceBefVAT, VATRate,
  VATAmt }`; per invoice `TotPriceWoVAT, TotVATAmt, TotPrice`; and every one of those is `DecimalSType` /
  `DecimalNegSType`, "Decimal number with two numbers after decimal point"
  (`pattern "-?([1-9][0-9]*|0)\.[0-9]{2}|0"`). **Whether the CIS validates `VATAmt == Σ VA` (per-line
  authority) or `VATAmt == PriceBefVAT × rate` (per-group authority) is NOT stated in the XSD and was not
  determined** (§7, first row). The field names are the same in the Albanian document's index, so the shape is
  established; the validation rule is not.
- **What ebills does**, measured by the other lane: `"totalVatAmount": 83.3333333333` on a 500-lek line at
  `VAT_20` (EBILLS §1.5) — the platform keeps the UNROUNDED per-line VAT and rounds only when it renders. The
  two-decimal fiscal message must then be produced from those, so ebills rounds at message time, not at line
  time. That is the per-group/per-invoice discipline, observed rather than documented.

**Decision (argued in §3.2):** compute per RATE GROUP on the group's discounted base, round once per group, sum the
groups. Per-line VAT is derived for display and never summed. Why not per line as the authority: with whole-lek
prices, per-line rounding drifts by up to ⌈n/2⌉ lek on n lines (§3.2 example 1), which the invoice total then
has to absorb somewhere, and the fiscal message has no field for "rounding difference". Why not per invoice: an
invoice with lines at two rates must state the base and the VAT of each rate (`SameTaxes`), so per-group is the
coarsest level the message allows. Per-group is also what a `Σ` over the log can re-derive from the order alone.

### 2.3 Discounts: the taxable base is the discounted amount

EU VAT Directive art. 79(b) excludes "price discounts and rebates allowed to the customer and obtained by him at
the time of the supply" from the taxable amount (widely stated; not re-read from the Directive text for this
document — hypothesis as to article number, not as to the rule). The Albanian message has a per-item `R` (rebate)
and the ebills record has `discount` and `discountReduceBase` per line (EBILLS §1.5), i.e. the format assumes the
rebate reduces the base unless told otherwise. So: **tax on the discounted amount**, and a basket-level discount
(a fixed promo, `promo.rs:205`) must be ALLOCATED across rate groups before tax is computed — largest-remainder
allocation so that the allocations sum to the discount exactly (§3.2, equation 3). A percent promo allocates
itself (each group's share is its own percentage), but `Promo::discount` floors the whole-basket figure
(`promo.rs:204`), so even a percent promo is applied as an allocated integer, not re-derived per group — one
figure on the order, one figure the customer was told.

**The tip** is a voluntary gratuity and stays outside the taxable base and outside the fiscal document
(hypothesis for Albanian practice; certain in the sense that ebills' sale has no tip field, EBILLS §1.5). **The
delivery fee** is a service the venue supplies and is taxable; whether it follows the food's rate (ancillary
supply) or the standard rate is jurisdiction-specific — hypothesis; the design gives it its own rate field so the
answer is a setting, not a code change (§3.3).

### 2.4 Inclusive versus exclusive: which number is stored

Albania and the EU display tax-inclusive consumer prices; the US displays exclusive. The kernel already models
the switch as `price_includes_tax: bool` (`money.rs:267,364`), and the two legs of `apply_tax` differ in which
quantity is derived: inclusive derives the NET from the gross (`money.rs:284`), exclusive derives the TAX from the
net (`:294`).

**Decision:** the stored catalogue price is **the number on the menu** — gross in Albania, net in a US venue — and
the venue's `tax.prices_include` flag says which. The ORDER copies the flag and stores BOTH the base and the tax
per group (§3.4), so an order is readable without knowing the venue's current flag. The alternative (store net
always, derive the menu price) fails on the first Albanian menu: 250 lek gross at 20 % is 208.33 net, which is
not a whole lek, so the stored number could not be the number the customer sees, and every price would be
displayed through a rounding. "Choosing wrong makes every historical order unreadable" is answered by not making
history depend on the choice at all: the order carries its own flag.

### 2.5 Price per channel in the systems that solve it

| System | Mechanism | Keyed by | Source |
|---|---|---|---|
| Toast | **Price levels** — a named set of prices per item, "combined with other pricing strategies, such as size pricing, menu-specific pricing, or time-specific pricing"; **Dining options** (dine-in, takeout, delivery) are a separate axis that "informs the kitchen whether they should plate or package" | price level ↔ menu/dining option | [Toast price levels], [Toast dining options] |
| Lightspeed Restaurant (via Deliverect) | **Price levels** managed in Lightspeed; "set the price level for delivery menus and set the price level for eat-in menus" | menu type | [Deliverect K-Series] |
| Odoo POS | **Pricelists** — "Multiple prices per product" (fixed price per product per list) or "Advanced price rules (discounts, formulas)"; one pricelist per POS config; a third-party module keys a pricelist per floor/table | POS / floor / customer | [Odoo pricelists], [Odoo floor pricelist] |
| Square | Menus per channel ("control where your menu is sold across your POS, online ordering, kiosks, and delivery apps"); channel-specific PRICES not confirmed from the page read | channel (visibility) | [Square multi-channel] |

All four are **price LISTS** (an explicit second number per item), not modifiers and not percentage rules, and
Odoo's own documentation lists the formula variant as the "advanced" case. The reason is visible in lek: a 7 %
uplift on 850 is 909.5 — a rule creates non-whole prices and a rounding per line, a list creates none. A list
also survives an import (one more optional column per kind) and a promotion (the promo applies to the priced
subtotal of the kind actually ordered; nothing to reconcile). §3.5.

### 2.6 Historical correctness: stamp at placement, and keep a schedule only for the future

Two patterns exist. **Effective-dated rate tables** (a `rates` table with `valid_from`; the order stores a
reference and the rate is looked up at read time) — common in ERPs. **Stamping** (the order carries the rate and
the amounts it was charged; nothing is looked up later) — universal in invoicing, because an invoice IS the
record. The invoicing answer is the right one here for three reasons: (1) a fiscal receipt already issued must
reproduce byte-for-byte a year later, and a lookup is a second source of truth that can move; (2) this log is
content-chained (§1.7), so a stamped figure is also a PROVEN figure; (3) the catalogue image is rewritten whole and
holds no history (`catalog.rs:70-87`), so an effective-dated table would have to be a new store.

What stamping cannot do is change the rate at 00:00 on the day a law changes while the owner is asleep — Germany
2026-01-01 is the case. So the settings hold the CURRENT rate and an optional short SCHEDULE of future changes
(`[{since_ms, ppm}]`); the pricer picks the entry in force at `ctx.data.now_ms` (the one clock, `clock.sh`,
baseline 0). Past changes are not kept in settings; the log has them.

### 2.7 What gamedev contributes, and where it stops

**Deterministic lockstep bans floats for the reason MANIFESTO C2 does.** "Desync bugs in lockstep multiplayer
games and broken replay systems almost always trace back to … floating-point arithmetic producing different
results on different hardware" ([cppcat]; [Gaffer on Games] on why IEEE results still vary by compiler and
FPU mode). Photon Quantum "completely replaces all usages of floats and doubles" with a **Q48.16** fixed-point
type ([Photon Quantum FP]); the Age of Empires lineage moved the simulation to integers ([gamedeveloper.com]).
That settles the DETERMINISM question the same way this repo already did.

**But binary fixed point gives the wrong answer to the REPRESENTATION question.** Measured: `0.2` in Q16 is
`13107/65536 = 0.1999969482421875`. A game does not care (1000 × that = 199.997, and the physics rounds).
A tax authority does: 20 % of 100 000 000 lek in Q16 is 19 999 695 lek — **305 lek short**. A tax rate is a
DECIMAL fraction and needs a DECIMAL scale; ppm is that, Q16 is not. This is the one precise contribution of the
analogy: it names the trap that "just use fixed point" walks into.

**Versioned content.** Path of Exile keeps "legacy" items: an item's values are stamped at creation, a balance
patch does not touch existing items unless the patch notes say "retroactive", and a player may CHOOSE to re-roll
a legacy item to current values with a Divine Orb ([PoE wiki], [PoE forum]). That is stamping (§2.6) with an
explicit opt-in to re-derive. **Where the analogy breaks:** the opt-in. A fiscal receipt is never re-derived; it
is REVERSED by a second document that references the first (ebills `sales-cancel`, `iicReference`, EBILLS §1.5-1.6),
and the reversal is itself fiscalised. dowiz already has the shape for that — an Earn leg and its exact Reversal
netting to zero (`money.rs:121-131`), `Refunding → CompensatedRefund` — and this document adds nothing to it except
that the reversal must carry the same `tax` block as the original, negated, so the VAT return nets as well as the
ledger. And the second break: a game needs identical results across MACHINES at one tick; a receipt needs identical
results across TIME. The second is stronger, and it is why the rate goes on the record and not in a table.

### 2.8 Channel as a first-class field; how a marketplace order differs

Toast separates **dining option** (how the food reaches the guest — the thing `fulfilment.kind` is) from **order
source** (POS device, online ordering, third-party) ([Toast dining options]). The two are orthogonal: a WhatsApp
order can be a pickup; a Glovo order is always a delivery but is not dowiz's delivery.

A marketplace order differs from a first-party one on four axes, each of which is a boolean or a number the
platform must know per channel — not prose:

| Axis | First-party (storefront, console, messaging) | Marketplace (Glovo, Wolt, Bolt Food) | Evidence |
|---|---|---|---|
| Who priced it | dowiz's pricer, `price_trusted: true` | the marketplace; dowiz records what the customer paid | the kernel's `price_trusted` rule exists for this (`domain.rs:118-121`) |
| Commission | none | 22-35 % of the order, "on the total order amount including taxes" | vendor blogs ([Slant], [Menuviel], [Foxifood]) — **hypotheses**, no contract read |
| Who fiscalises | the venue (via its fiscal device / ebills) | depends on agent-vs-principal: a platform acting as principal accounts for VAT on the full price (Bolt's UK ride-hailing position, [Upper Tribunal]); food marketplaces in Albania — **not determined** | §7 |
| Who owns the customer | the venue (`contact`, the `Revealed` audit) | the platform; contact details masked | [Wavicle], [PYMNTS] — hypotheses |
| Stock | reserved at placement (`place.rs:124-127`) | drawn the same way, same ledger | by construction, `fulfilment.rs:24-29` |

So the channel model needs a **profile** per channel (§3.6), and the closed set is the key into it.

---

## 3. The design

### 3.1 The rate type, and the authority flip

In `crates/dowiz-core/src/money.rs`:

```rust
/// A tax rate in parts per million. 20 % = 200_000. Never a float, never a decimal string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RatePpm(pub u32);          // u32: 4 294 % is the ceiling; a rate above 100 % is refused at parse

pub fn tax_of(base: i64, rate: RatePpm, inclusive: bool) -> Result<i64, String>
```

`tax_of` IS `eqc_gen::apply_tax_exclusive_int` / `apply_tax_inclusive_int` (`eqc_gen.rs:30,50`), called with
`rate.0 as i64`. That is the authority flip `eqc_gen.rs:18-20` records as not done: the generated organ becomes
the law and the hand-written `apply_tax(…, f64, …)` becomes a thin adapter for its two remaining callers
(`temporal_tmr.rs:243`, `json_bridge.rs:243`) until they are moved or deleted, then it is deleted. **The parity
test survives the flip unchanged in meaning**: today it pins float-law == organ; after the flip it pins
adapter(float) == organ, and when the adapter goes, its fixture grid (`money.rs:626-657`) becomes the organ's own
regression table with `rate_micro` literals. No expected value changes, because the organ's arithmetic is what
`apply_tax` already does (`money.rs:284,294` are the same two expressions).

Why `u32` and not `i64`: a negative rate is the one input both laws guard against
(`money.rs:280-283`, `eqc_gen.rs:74-76`, `red_tax_negative_rate_is_err_not_divzero` at `money.rs:540`); an unsigned
type makes that guard a parse-time refusal instead of a runtime one. The organ keeps its `i64` parameter; the
widening is free.

### 3.2 The summariser: equations, rounding, and two worked examples in lek

`money::summarise` — pure, `no_std` + `alloc`, in `dowiz-core` so both the Worker and the object can call it (the
crate is linked into the same WASM, `workers/api/Cargo.toml:26-29`):

```
input:  lines[i] = (amount_i, rate_i)      amount_i = unit_price × quantity as PRICED (gross if inclusive, net if not)
        discount D ≥ 0  (the whole-basket cut, promo.rs:200-208)
        inclusive: bool
        fee, fee_rate                        the delivery fee and its own rate

(1) group by rate:            G_r = Σ amount_i over lines with rate_i = r
(2) allocate the discount:    d_r = ⌊D·G_r / Σ_r G_r⌋, then hand the remainder D − Σ d_r one lek at a time to the
                              groups with the largest fractional part (largest remainder), so Σ d_r = D exactly
(3) the base of each group:   B_r = G_r − d_r
(4) the tax of each group, once, in i128:
      inclusive:   net_r = ⌊(B_r·10⁶ + (10⁶ + r)/2) / (10⁶ + r)⌋ ;   tax_r = B_r − net_r
      exclusive:   tax_r = ⌊(B_r·r + 5·10⁵) / 10⁶⌋
    — the quantity that is ROUNDED is named: the NET for inclusive, the TAX for exclusive. That is what
      money.rs:284 and :294 do today and what the organs do; it is stated here so nobody "fixes" it (example 2).
(5) the fee is its own group at fee_rate, same equation.
(6) totals:  tax = Σ tax_r ;  inclusive: total = Σ B_r + fee + tip ;  exclusive: total = Σ B_r + tax + fee + tip
```

**Example 1 — per line vs per group (the naive answer differs).** Three coffees at 250 lek, 20 % inclusive.
Per group: `B = 750`, `net = ⌊(750·10⁶ + 600 000)/1 200 000⌋ = 625`, `tax = 125`. Per line, rounded and summed:
`net(250) = ⌊(250·10⁶ + 600 000)/1 200 000⌋ = 208` (208.33 rounds down), `tax = 42`, three lines → **126**.
Exact is 125. Measured (Python, integer arithmetic as above). The 1-lek difference has no field to live in on a
fiscal message whose `VATAmt` is one number per rate.

**Example 2 — which quantity you round decides a tie.** The same basket with a 10 % promo: `D = 75` (`promo.rs:204`
floors `750·10/100 = 75`), `B = 675`. Exact net is 562.5, exact tax 112.5 — a tie, and with whole-lek prices at
20 % every gross that is 3 mod 6 is a tie. Round the NET half-up (the law, `money.rs:284`): `net = 563`,
**tax = 112**. Round the TAX half-up instead: **113**. Truncate the net: `562`, tax **113**. Three rules that
each sound like "round half up", two answers, and the fiscal message carries `PB`, `VA` and `PA` for the same
line, so they have to add up. The equation in (4) names the rounded quantity; the gate in §5 (G3) refuses a
summary whose `Σ B_r + Σ tax_r` does not equal what the customer paid.

**Example 3 — why ppm and not Q16** is in §2.7: 20 % of 100 000 000 lek is 305 lek short in binary fixed point.

### 3.3 Where a rate comes from, in order of precedence

1. **The line.** A product record gains `vat_ppm` (integer, optional). The pricer copies it onto the `Line`.
2. **The venue default.** New declared settings keys (the key space is closed, `settings.rs:3-5`, so these are
   additions to `dowiz_hub::settings::KNOWN`): `tax.default_ppm`, `tax.prices_include` (`"true"`/`"false"`),
   `tax.delivery_fee_ppm`, `tax.schedule` (a JSON list `[{"since_ms":…, "ppm":…}]`, at most a handful of
   entries — the future only, §2.6). A product with no `vat_ppm` uses the default in force at `now_ms`.
3. **Nothing else.** No rate by fulfilment kind (§4, item 4). If a jurisdiction ever taxes by kind, it is one
   more settings key `tax.kind_ppm.<kind>` resolved at step 2 — no schema change, because the rate is resolved
   per line by the pricer anyway.

A venue with no `tax.default_ppm` set is a venue that has not been configured for tax; its orders carry NO
`tax` block, and the fiscal seam (§3.7) refuses them. That is not a silent zero — `NoRate` is a named refusal
and an owner-visible state — and it is the transition rule for the live venues: nothing changes for them until
the owner sets one key.

### 3.4 What the order carries

Per line, in `items[]` (survives transitions because `items` is carried whole, `hubstore.rs:1350`):
`"vat_ppm": 200000`. Per order, a new hub-owned key — **added to `HUB_OWNED` (`hubstore.rs:1322`) in the same
commit, §1.8** — :

```json
"tax": {
  "inclusive": true,
  "groups": [ { "rate_ppm": 200000, "base": 675, "tax": 112, "lines": 1 } ],
  "fee":    { "rate_ppm": 200000, "base": 300, "tax": 50 },
  "total": 162,
  "discount_allocated": [ 75 ]
}
```

For an inclusive venue `total` on the envelope does not change meaning: it was the gross and it stays the gross,
which is why every historical order remains correct and why no migration is needed. For an exclusive venue
`total` gains `tax.total` (equation 6). The block is computed **inside `command::place::decide`, after the
discount is patched** (`place.rs:146-149`), by calling `dowiz_core::money::summarise` on the lines the Worker
priced and the cut the object decided. This is the tail of pricing that the file's own header says moves
(`place.rs:46-51`): the lines are not re-priced, the tax of the given lines after the given cut is computed once,
where the cut is known. Adding it to the Worker as well would be a second summariser and the exact defect
`a18886d4` closed.

The ebills mapper writes `vat_rate_pct` (`ebills/mod.rs:300-302, 352`). **Handed to that lane as text, not
edited here:** spell it `vat_ppm` so the log has one vocabulary for a rate; `VAT_20` → `200000` is the same
one-line parse.

### 3.5 Price per fulfilment kind: a list, not a rule

Product record: `"price": 900, "prices": { "delivery": 950, "dine_in": 900 }` — `prices` optional, every key
optional, a missing key falls back to `price`. `pricing::price_basket` gains one argument, `kind: &str`, and reads
`prices[kind]` before `price` (`pricing.rs:126`). The promo preview (`preview.rs:28`) passes the same `kind`, so
the quote and the checkout keep agreeing — the two-pricer lesson is "one function, one more argument", not a
second function.

| Interaction | Rule | Where it already is |
|---|---|---|
| Free-delivery threshold | measured against the KIND-priced subtotal; nothing changes | `fulfilment::fee(kind, subtotal, free_over, fee)` (`fulfilment.rs:81-87`) already takes the subtotal it is given |
| Minimum order | against the kind-priced subtotal | `storefront.rs:881` |
| Promo `min_order` | against the kind-priced subtotal | `promo.rs:219` |
| Percent promo | on the kind-priced subtotal | `promo.rs:204` |
| Menu import | three optional CSV columns `price_delivery`, `price_pickup`, `price_dine_in`; absent = fall back | `services/catalogue/import.rs:12`, `dowiz_hub::import::from_csv` |
| The storefront menu | returns `price` AND `prices`; the client shows the price for the kind selected; a kind change re-renders the basket total from `prices`, and the server re-prices at checkout regardless (`pricing.rs:87-89`) | `storefront::menu` (`storefront.rs:193`) |
| A marketplace | NOT a key in `prices` — a marketplace order arrives priced; §3.6 profile `priced_by_us: false` | §4 item 5 |

### 3.6 `channel`: the closed set, who sets it, and the compatibility rule

`workers/api/src/services/ordering/channel.rs`, the mirror of `fulfilment.rs`:

```rust
pub const ALL: [&str; 5] = ["storefront", "console", "whatsapp", "instagram", "ebills"];
pub fn known(c: &str) -> bool
pub fn of(envelope: &Value) -> &str          // absent or unknown ⇒ "storefront"
pub struct Profile { pub first_party: bool, pub priced_by_us: bool, pub fiscalised_by_us: bool,
                     pub owns_customer: bool, pub commission_ppm: u32 }
pub fn profile(c: &str) -> Profile
```

- **Absent means `storefront`**, with the date on it: until this module exists the only route that appends
  `Placed` is `storefront::place` (measured: the only non-test `EventKind::Placed` append is `place.rs:153`, reached
  from `storefront.rs:1131`), and it has written `"storefront"` since the kernel call at `storefront.rs:934`. The
  kernel writes an explicit `null` on orders that never had one (`fold.rs:33-36`); `of` treats `null`, absent
  and an unknown word alike, exactly as `fulfilment::of` does (`fulfilment.rs:97-103`).
- **Set by the handler from the principal, never from the body.** `storefront::place` → `"storefront"`; an owner
  console route that types in a phone or walk-in order (none exists today — §7) → `"console"`; the messaging
  webhook (`channels.rs`) when it ever places an order → `"whatsapp"` / `"instagram"`; the ebills poller →
  `"ebills"`. A `channel` in the request body is discarded before it is believed, the `unit_price` precedent
  (`pricing.rs:87-89`).
- **Marketplaces** are added to `ALL` by the commit that integrates one, with a `Profile` whose `priced_by_us`
  and `fiscalised_by_us` are the two facts §2.8 could not determine and that the integration must.
- **What breaks if it is absent on old records:** nothing — that is what the compatibility rule is for. What
  breaks if a NEW record lacks it: the analytics fold (`fold.rs:134-138` counts by kind today; a by-channel
  tally is the obvious next column) would file it under `storefront`, which is true of every order that exists
  at the time of writing and false of the first ebills import. G4 in §5 is the ratchet: no `Placed` event after
  the gate's `since` may lack the field.
- **Kernel vs Worker.** `KINDS` and `ALL` both belong in `dowiz-core` so `tools/gen-vocab` can emit them and
  `fulfilment.js:11-16`'s hand copy can go; that is one move for both and is scheduled after the channel exists
  (§6, item 7), not before, so the module is born with a caller (`unreached.py` counts a `pub fn` no shipping
  code calls).

### 3.7 The seam a fiscal integration plugs into

Owned here: the shape. Owned by the ebills lane: the wire (`workers/api/src/ebills/**`), the session, the poller,
the inbound mapping, and — explicitly out of that lane's scope by the operator's instruction (EBILLS §5 item 6) —
pushing dowiz orders INTO a fiscal platform. This seam is what such a push would call.

```rust
// workers/api/src/services/fiscal/mod.rs — PURE. No fetch, no clock, no provider name.
pub struct Document {                      // one fiscal document for one order
    pub order_id: String,
    pub uuid: [u8; 16],                    // derived from order_id + venue: the idempotency key of every retry
    pub issued_at_ms: i64,                 // ctx.data.now_ms, passed in
    pub kind: Kind,                        // Cash | NonCash — TypeOfInv in the spec family
    pub payment: Payment,                  // from PAYMENT_KINDS (storefront.rs:649, 1038)
    pub inclusive: bool,
    pub lines: Vec<Line>,                  // name, qty, unit_as_priced, rebate, base, rate_ppm, tax, gross
    pub groups: Vec<Group>,                // rate_ppm, n_lines, base, tax  — SameTaxes, by another name
    pub total_base: i64, pub total_tax: i64, pub total: i64,
    pub table: Option<String>,             // fulfilment.table for dine_in
    pub channel: &'static str,
}
pub enum Refusal { NoTax, AlreadyFiscalised { by: String }, Untrusted, NotTaken, Currency(String), Lines(String) }
pub fn document(envelope: &Value, venue_currency: &str, now_ms: i64) -> Result<Document, Refusal>
pub fn receipt_text(doc: &Document, codes: Option<&Codes>) -> String   // what a printer or a screen shows
```

Rules the seam enforces, each a native test:
1. **No `tax` block → `Refusal::NoTax`.** This is the rule that makes a tax-free order impossible to fiscalise once
   tax exists, and with G2 (§5) it makes a tax-free order impossible to PLACE at a configured venue.
2. **`external.fic` present (an order that CAME from ebills, EBILLS §3.2 (1)) → `AlreadyFiscalised`.** Fiscalising
   it again is a double invoice to the tax authority.
3. **`price_trusted: false` → `Untrusted`.** The kernel's rule that an untrusted order is never charged
   (`domain.rs:118-121`) extends to never being invoiced.
4. **Status not `took_money()` (`order_machine.rs:92-94`) → `NotTaken`**; a refund produces a second `Document`
   whose lines and groups are the negation of the first and whose `reference` names it — the ledger's
   Earn/Reversal shape (`money.rs:121-131`) applied to the VAT.
5. **The result is recorded as a `Noted` event** (`EventKind::Noted = 5`, "a fact was ADDED to an order without
   its status moving", `dowiz-hub/src/lib.rs:83-91`) with payload `{ "fiscal": { "provider", "uuid", "iic",
   "fic", "inv_ord_num", "issued_at_ms" } }`, and `"fiscal"` joins `HUB_OWNED` (§1.8). No new `EventKind`. The
   codes' names are the Albanian ones because that is the venue; a second provider maps its own into the same
   three slots or adds a fourth.
6. **Retries re-send the same `uuid`** — the platform's own idempotency key (EBILLS §1.5 "The idempotency key is
   `uuid`"), so a lost response cannot produce two invoices. The Worker-side guard is `idempotency::guard`, the
   courier precedent (`idempotent.sh`).

The provider adapter that turns a `Document` into ebills' `POST /api/sales` (or into a fiscal device's XML) is
one file that touches a port, per the `services/mod.rs:3-9` rule, and it is the other lane's when the operator
opens that scope.

### 3.8 The receipt

`receipt_text` is a rendering of `Document`: the lines with unit and gross, one line per rate group ("TVSH 20 %:
base 675, VAT 112"), the total, the payment kind, the table, and — when the `Noted` event exists — the fiscal
codes and the verification URL in the shape the ebills lane recorded (`…/invoice-check/#/verify?iic=…&tin=…`,
EBILLS §1.5). It is text for the same reason `notify::order_text` is text: the console, a PDF, a WhatsApp message
and a thermal printer all take text, and ESC/POS is an encoding of it that this document recommends against
building now (§4 item 7).

---

## 4. Recommended AGAINST, each with its re-entry condition

1. **Basis points, per-mille, or a decimal string as the rate.** Basis points cannot hold 8.875 % (§2.1); a string
   is a float with extra steps. Re-entry: none.
2. **A rational or decimal library in `dowiz-core`.** The crate is zero-dependency `no_std` (ARCHITECTURE-EVOLUTION
   §1.1) and ppm loses nothing for any published rate. Re-entry: a jurisdiction whose rounding rule cannot be
   written as one integer division with a named rounded quantity — none known.
3. **Per-line rounding as the authority.** Drifts by up to ⌈n/2⌉ lek (§3.2 example 1). Re-entry: the Albanian CIS
   is shown to validate `VATAmt == Σ VA` per line (§7 row 1) — then per-line becomes the law, `summarise` rounds
   each line and the group is the sum, and G3 changes its equation. This is the single most important thing §7
   asks someone to find out.
4. **A tax rate keyed by fulfilment kind.** The brief's premise for Albania is false (§0 row 2); Germany, the market
   that had the split, removed it. Re-entry: a law that reinstates one, met by one settings key (§3.3 item 3).
5. **Price lists keyed by marketplace.** No marketplace integration exists; a marketplace prices its own menu. Re-entry:
   the first integration, which will need `priced_by_us: false` (§3.6) far more than it needs a price list.
6. **Effective-dated rate tables with full history.** The log carries history and proves it (§1.7); a table would be
   a second, unproven copy. Re-entry: none; a future-only schedule is kept (§2.6).
7. **ESC/POS and a receipt printer.** The venue's receipts are printed by its fiscal terminal (EBILLS §1.5). Re-entry:
   a venue without a fiscal terminal that prints, or a courier-side proof-of-delivery slip.
8. **Reviving `Order::recompute_total` / `compute_order_total` (`domain.rs:186-222`).** Both take an `f64`, both compute
   tax on the undiscounted subtotal ("no discount", `domain.rs:201`), and neither is on the Worker path. Re-entry:
   when the kernel's `Order` carries fulfilment and discount at this boundary ("until the aggregate's new fields
   reach this boundary", `storefront.rs:940-941`) — then `summarise` becomes the aggregate's method and these two
   are deleted, not rewritten.
9. **Retroactive re-pricing or re-taxing of any placed order.** Never (§2.6, §2.7). A wrong rate is fixed by a
   reversal and a new document, which is also how the tax authority wants it.
10. **A float at the "boundary".** `"tax_rate": 0.20` is refused by the settings parser; `tax.default_ppm` takes an
    integer string. The one remaining money float, `convert_all_to_eur_cents(_, rate: f64)` (`money.rs:332`), is
    display-only and `rates.rs` already ships ppm; moving it is item 8 of §6.
11. **Moving the whole pricer into the object.** Only the tax-after-discount tail moves (§3.4). Re-entry: none; the
    object serialises a venue's commands and the pricer's catalogue lookups are the Worker's to keep cheap.

---

## 5. Gates

Shape: a script under `tools/gates/`, a `.baseline` file, refuse on a rise, demand the baseline be lowered on a
fall (`one-venue.sh:56-70`), and **a proof that it fires both ways before it is trusted** — the
`conservation.prove.mjs` rule: "a gate is triggered before it is trusted … THE GREEN CASES ARE HALF THE PROOF"
(`e2e/gates/conservation.prove.mjs:1-16`). Each gate below names its red case and its green case.

| Gate | Counts | Baseline (measured) | Ratchet | Red proof | Green proof |
|---|---|---|---|---|---|
| **G1 `float-money.sh`** | non-test `f64` tokens in `crates/dowiz-core/src/{money,domain}.rs`, `workers/api/src/services/ordering/**`, `workers/api/src/command/**` — counted the `clock.sh` way (`clock.sh:15-23`: the SITE that decides, so `as f64`, `f64::from`, `: f64` and `_f64(` are all one token class) | `money.rs` 3 + `domain.rs` 9 + 0 + 0 = **12** | to 0 after §6 items 1, 8 and 4-against-8 | append `let _r: f64 = 0.2;` to a scratch copy of `pricing.rs` on a branch → exit 1 naming the file | the tree at HEAD → exit 0, `12 (baseline 12)` |
| **G2 `taxed-placement` (native test + runtime refusal)** | `command::place::decide` refuses an envelope with no `tax` block when the venue has `tax.default_ppm` set (`Refused::Untaxed`); accepts one without when the venue has no rate (§3.3, the transition rule) | 0 refusals expected on today's venues (no key set) | n/a — a runtime rule, tested both ways in `command/tests.rs` (14 tests there today) | envelope without `tax`, venue with a rate → `Err(Untaxed)` | same envelope, venue without a rate → `Ok`; envelope with `tax`, venue with a rate → `Ok` |
| **G3 conservation law 8 (`e2e/gates/conservation.mjs` + `.prove.mjs`)** | for every `Placed` since the venue's `tax.since_seq`: `Σ groups.tax == tax.total`, `Σ discount_allocated == discount`, and inclusive ⇒ `Σ groups.base + fee.base + tip == total`, exclusive ⇒ `+ tax.total`; and `tax_of(base, rate, inclusive)` recomputed from the stored `base` and `rate_ppm` equals the stored `tax` (the stamp is re-derivable, §2.6) | 0 violations | stays 0 | a stubbed `/api/owner/backup/cloud` with one order whose `tax.total` is off by 1 → the sentence names the order | the healthy stub → green; a venue with `since_seq` unset → "not measured", never red (`conservation.prove.mjs:12-16`) |
| **G4 `channel-closed.sh`** | (a) every `"channel"` literal written in `workers/api/src` (outside `channels.rs`/`notify.rs`/`mcp.rs`/`integrations.rs`, which are the messaging vocabulary) is a member of `channel::ALL` — the `vocab.sh` rule-3 shape ("a hand copy came back"); (b) every `Placed` append site passes through a function that sets it | (a) 2 sites today: `storefront.rs:934` (value only), `ebills/mod.rs:261`; (b) 1 site (`place.rs:153`) | (a) to 0 literals outside `channel.rs`; (b) stays 1 | add `"channel": "fax"` to a scratch handler → exit 1 | HEAD after §6 item 6 → exit 0 |
| **G5 eqc parity (`money.rs:565`, exists)** | exact integer equality, law vs organ, over the fixture list and the grid | passes today (`cargo test -p dowiz-core apply_tax_generated_parity`, not re-run for this document — §7) | unchanged; after the flip the "law" side is the adapter and then the fixture table | change one `+ b / 2` in `eqc_gen.rs` → the grid fails at the first tie | HEAD → passes |
| **G6 `vocab.sh` (exists)** | the generated `public/lib/vocab.js` is fresh | — | — | once `KINDS`/`ALL` move into `dowiz-core` (§6 item 7), `gen-vocab` emits them and `fulfilment.js:22`'s hand copy is refused the way a status hand copy is (`vocab.sh` rule 3) | — |

G2 is the one the brief asked for: "make a tax-free order impossible once tax exists". It is a runtime refusal
in the object rather than a static count, because the fact it guards ("this venue has a rate") is per venue and
lives in an image, not in the source tree. G3 is what makes G2 honest over time: an order that carries a `tax`
block whose numbers do not add up is caught by the nightly walk, the same walk that catches an edited ledger.

---

## 6. Order of work, each with its CHECK

1. **`RatePpm` + `tax_of` + `summarise` in `dowiz-core/src/money.rs`; the authority flip.** `apply_tax(f64)`
   becomes an adapter over `tax_of`. Tests: the existing parity test unchanged; `summarise` tests for examples 1
   and 2 of §3.2 with the literal lek figures (750 → 125 not 126; 675 → 112 not 113); largest-remainder allocation
   sums exactly; a two-rate basket; `inclusive` and exclusive totals; overflow near `i64::MAX` refused, never
   wrapped (`red_tax_i128_overflow_is_err_not_panic` shape, `money.rs:552`).
   **CHECK:** `cd crates/dowiz-core && cargo test --offline` green; G1 falls from 12 to 10 (`money.rs:267,363`
   remain as the adapter until item 8); `tools/eqc-rs` regenerates nothing (the organs are unchanged).
2. **Settings keys** `tax.default_ppm`, `tax.prices_include`, `tax.delivery_fee_ppm`, `tax.schedule` in
   `dowiz_hub::settings::KNOWN`; a pure `tax::in_force(schedule, default, now_ms) -> Option<RatePpm>`.
   **CHECK:** `POST /api/owner/settings` with `"tax_rate": "0.20"` → 400 "unknown setting"; with
   `tax.default_ppm = "200000"` → 200; `in_force` at one ms before and after a scheduled `since_ms` gives the two
   rates (native test, no clock).
3. **`vat_ppm` on the product record and the `Line`.** `price_basket` copies it; absent → the venue default from
   item 2; no default → lines carry `None` and item 4 writes no `tax` block.
   **CHECK:** `services/ordering/tests.rs` (10 tests today) gains: a line with its own rate, a line falling back,
   a venue with no rate producing `None` and no refusal at this layer.
4. **The `tax` block in `command::place::decide`, after the discount; `"tax"` into `HUB_OWNED`; G2.**
   **CHECK:** `command/tests.rs`: a promo basket's stored `tax.groups[0].base` equals `subtotal − cut`; the
   `Untaxed` refusal both ways (§5 G2); a placed order advanced through `CONFIRMED` still carries `tax`
   (the `HUB_OWNED` trap, tested the way `payment_intent`'s loss was closed). Live: one order on a test venue with
   a rate set, read back via `/api/owner/history`, the block present after "confirm".
5. **G3 as conservation law 8**, with its `.prove.mjs` cases. **CHECK:** `node e2e/gates/conservation.prove.mjs`
   lists the new red and green cases by name; `ci.yml` unchanged (line 86 already runs it).
6. **`channel.rs`** with `ALL`, `of`, `profile`; `storefront::place` and the ebills mapper (handed back as text)
   use it; G4 with its baseline. **CHECK:** `sh tools/gates/channel-closed.sh` → `2 (baseline 2)` before,
   `0 (baseline 0)` after; the fold's new by-channel tally shows every existing order as `storefront`
   (the compatibility rule, tested as `fulfilment/tests.rs:66` is).
7. **Move `KINDS` and `ALL` into `dowiz-core`; `gen-vocab` emits both; delete the hand copy in `fulfilment.js`.**
   **CHECK:** `sh tools/gates/vocab.sh` green; `grep -rn "\['delivery', 'pickup', 'dine_in'\]" workers/api/public`
   → nothing.
8. **Delete the `f64` entry points**: `apply_tax(f64)` after moving `temporal_tmr.rs:243` and `json_bridge.rs:243`
   to `tax_of`; `convert_all_to_eur_cents(_, f64)` to a ppm parameter (`rates.rs` already ships one);
   `compute_order_total`/`recompute_total` deleted per §4 item 8 or left for the kernel-aggregate work with a
   comment naming this document. **CHECK:** G1 at 0 for `money.rs`; `domain.rs` at whatever its non-money `f64`
   count is (the 9 include geo, which G1 does not cover — the baseline names the exact lines).
9. **Price per kind** (`prices` on the product, the `kind` argument, the import columns, the menu response).
   **CHECK:** the promo preview and the checkout give the same subtotal for a `dine_in` basket with a `prices.dine_in`
   override (the `a18886d4` test, with one more argument); an import with the three columns round-trips through
   `to_bytes`/`load` (`catalog.rs:196` shape).
10. **`services/fiscal/mod.rs`: `Document`, `document()`, `receipt_text()`, the six refusals**, `"fiscal"` into
    `HUB_OWNED`. No route, no fetch — and, per `unreached.py`, no `pub` until a caller exists (EBILLS §7's own
    rule). **CHECK:** native tests for each refusal; `document()` of the §3.2 example-2 order yields one group
    `{200000, 675, 112}`; a refund order yields the negation and a `reference`.
11. **Documents.** `CLAUDE.md:98`'s "zero floats, ever" gains the line "rates are `RatePpm`; `f64` is refused at
    the settings parser"; `MANIFESTO.md:18` C5 unchanged (it was right).

**Order rationale.** 1-2 are the representation and cost nothing on the live path; 3-4 make every new order at a
configured venue carry its tax and are inert for the two live venues until an owner sets one key; 5 is the proof
that 4 stays true; 6-7 are the channel and the vocabulary hygiene it needs; 8 pays the red-line debt; 9 is the
commercial feature and is independent of tax; 10 is the seam and is the last thing built because it needs
everything before it and is the only item whose consumer is another lane's.

---

## 7. What was NOT determined, and the command or act that would settle it

| Question | Status | What settles it |
|---|---|---|
| Does the Albanian CIS validate the per-rate-group `VATAmt` as `Σ` of per-item `VA` (per-line authority) or as `PriceBefVAT × VATRate` rounded (per-group)? | **not determined**; the XSD embedded in the sister spec types every amount as two-decimal and states no relation | read `https://www.tatime.gov.al/shkarko.php?id=14063` §3.1.72-3.1.95 (refused this box's connection twice: `ECONNREFUSED 134.0.43.56:443`); or submit one two-line test invoice to the CIS test environment through a certified provider and observe the refusal message |
| Whether ebills rounds the 2-decimal message from the unrounded per-line VAT it stores (`83.3333333333`) | inferred from one measured record (EBILLS §1.5) | compare `totalVatAmount` on a multi-line sale against `Σ` of its lines' `totalVatAmount`, both from `GET /api/sales/{id}` — a read the other lane's allow-list permits |
| Is a voluntary tip outside the VAT base in Albania? Does a delivery fee take the food's rate or the standard rate? | hypothesis (§2.3) | Law 92/2014 art. on the taxable amount, or the venue's accountant; the design makes each a setting |
| Food-delivery marketplaces in Albania: agent or principal for VAT on the food? Who issues the fiscal invoice to the consumer? | not determined (§2.8); the Bolt UT decision is ride-hailing, UK | a marketplace's merchant agreement, when one is signed |
| Marketplace commission figures (22-35 %) | vendor blogs, hypotheses | the same agreement |
| Does an owner-console route that places an order on a customer's behalf exist? | not checked (§3.6 assumes not) | `grep -rn 'command::send(&place, "place"' workers/api/src` — one hit today (`storefront.rs:1131`) means no |
| The parity test passes at HEAD | not re-run for this document | `cd crates/dowiz-core && cargo test --offline apply_tax_generated_parity` |
| Stripe's minor unit for `ALL` | `stripe.rs:14-15, 70` sends `amount=<lek>&currency=all`; Stripe treats ALL as a two-decimal currency (hypothesis from Stripe's zero-decimal list, which does not name ALL) — if so, 1500 lek is sent as 15.00 lek | one PaymentIntent on a test key and its `amount` in the dashboard; not this document's scope, but it is the kit's fourth-copy defect at the rail and belongs to whoever owns `stripe.rs` |
| The `f64` count in `domain.rs` that is money vs geo | 9 non-test tokens counted, not classified | `grep -n f64 crates/dowiz-core/src/domain.rs` and name each line in G1's baseline |
| Whether Square prices differ per channel | its page confirms per-channel menus, not per-channel prices | the Square Online item-level page, not read |
| The kernel's `orders_by_channel` (`analytics.rs:97`) vocabulary (`"web"`, `"app"`) vs the Worker's `"storefront"` | two vocabularies, neither read by anything served | resolved by §6 item 7: one `ALL` in `dowiz-core`, and the kernel's tests use its members |

---

## 8. What was written

This file only. No source file was touched; the two things handed to other lanes as text are the `vat_ppm`
spelling for `ebills/mod.rs:352` (§3.4) and the `channel::of` call for its `"channel": "ebills"` (§3.6).

## 9. Sources outside the tree

- Albania rates: [tvsh.al — VAT rate in Albania 2026](https://tvsh.al/articles/vat-albania/) (6 % list: accommodation, agritourism restaurant services excluding drinks, books, named others); [openaccountants — albania-vat.md](https://github.com/openaccountants/openaccountants/blob/main/skills/international/albania/albania-vat.md) (Law 92/2014, Law 87/2019; hotel restaurants at 20 %).
- Germany: [VATupdate — Germany cuts restaurant food VAT to 7 % from 2026, unifying dine-in, takeaway and delivery](https://www.vatupdate.com/2025/12/30/germany-cuts-restaurant-food-vat-to-7-from-2026-unifying-dine-in-takeaway-and-delivery-rates/); [Stripe — VAT in the restaurant industry in Germany](https://stripe.com/resources/more/vat-in-restaurant-industry-germany) (the 2024 19 %/7 % split).
- Rounding: [GOV.UK — VAT guide (Notice 700) §17.5-17.6](https://www.gov.uk/guidance/vat-guide-notice-700); [HMRC VATREC12020 — rounding at retailers](https://www.gov.uk/hmrc-internal-manuals/vat-trader-records/vatrec12020).
- Fiscal message shape: [tatime.gov.al — Fiscalization service technical specifications](https://www.tatime.gov.al/eng/c/320/327/fiscalization-service-technical-specifications) (index page; the PDF `shkarko.php?id=14063` refused the connection); the Montenegro sister specification `fiskalni_servis_-_tehnicka_specifikacija_v2.pdf` (poslodavci.org), fetched, content streams inflated on this box, XSD types quoted in §2.2.
- Vendors: [Toast — Using price levels](https://doc.toasttab.com/doc/platformguide/adminUsingPriceLevels.html); [Toast — Dining options](https://doc.toasttab.com/doc/platformguide/adminDiningOptions.html); [Deliverect — Lightspeed K-Series settings (price levels per menu type)](https://help.deliverect.com/en/articles/9755768-lightspeed-restaurant-k-series-settings); [Odoo 19 — POS pricelists](https://www.odoo.com/documentation/19.0/applications/sales/point_of_sale/pricing/pricelists.html); [Odoo apps — POS floor pricelist](https://apps.odoo.com/apps/modules/16.0/sh_pos_floor_pricelist); [Square — Manage your menus across locations and sales channels](https://squareup.com/help/us/en/article/8553-manage-your-menus-across-locations-and-sales-channels).
- Gamedev: [Photon Quantum — Fixed Point (Q48.16)](https://doc.photonengine.com/quantum/current/manual/quantum-ecs/fixed-point); [Gaffer on Games — Floating Point Determinism](https://gafferongames.com/post/floating_point_determinism/); [cppcat — Deterministic physics in C++](https://cppcat.com/deterministic-physics-engine/); [Game Developer — Online multiplayer the hard way](https://www.gamedeveloper.com/game-platforms/online-multiplayer-the-hard-way); [Path of Exile wiki — unique items with legacy variants](https://pathofexile.fandom.com/wiki/List_of_unique_items_with_legacy_variants); [PoE forum — Item nerfs retroactive?](https://www.pathofexile.com/forum/view-thread/565767).
- Marketplaces (hypotheses): [Slant — Glovo vs Wolt for restaurant owners](https://blog.slantco.com/glovo-vs-wolt-comparison-for-restaurant-owners/); [Menuviel — Bolt Food fees](https://blog.menuviel.com/bolt-food-fees-and-commissions-for-restaurants/); [Foxifood — cut delivery commission](https://www.foxifood.com/blog/reduce-delivery-commission-fees/); [Wavicle — Who owns the customer?](https://wavicledata.com/blog/food-delivery-who-owns-the-customer/); [PYMNTS — restaurants want their customers back](https://www.pymnts.com/news/delivery/2026/why-restaurants-want-their-customers-back-from-delivery-platforms/); [Upper Tribunal — Bolt Services Ltd (agent vs principal)](https://assets.publishing.service.gov.uk/media/67e140d1c6194abe97358caa/Bolt_UT_Decision__final_.pdf); [Crowe — VAT: agent vs principal](https://www.crowe.com/uk/insights/avp-vat).
