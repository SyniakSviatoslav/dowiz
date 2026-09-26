# Menu, ingredients, stock in/out, batches and consumption analytics — research and plan

Date: 2026-09-26. Read-only analysis of `/root/dowiz` at HEAD (the uncommitted `crates/dowiz-hub/src/hub/` split does not touch
`stock.rs`, `stock/cost.rs`, `recipe.rs` or the owner routes; every path below was read from the working copy, which equals
HEAD for those files). Every claim marked **verified** was read in code at the cited line; anything else is marked
**unverified**. No code, gates or deploys were run.

Vocabulary: приход = `Received`; розхід = `Consumed`/`Served`/`Wasted`; інвентаризація = `Stocktake`; партія = lot;
брутто/нетто/вихід = gross / net (after cleaning) / out (after cooking, on the plate).

---

## 1. Current state (verified, with file:line)

### 1.1 Supplies (what the kitchen buys)

Stored as one JSON record per supply in the catalogue image (`Catalog::set_supply`, `crates/dowiz-hub/src/catalog.rs:155-165`).
Shape is fixed by `record()` in `workers/api/src/services/operations/supplies.rs:148-174`:

| key | meaning | source |
|---|---|---|
| `id`, `name`, `kind`, `category`, `unit` | kind ∈ `food_ingredient, condiment, packaging, utensil` (`recipe.rs:31`); unit ∈ `g, ml, unit` (`recipe.rs:33`) | supplies.rs:151-157 |
| `kcalPer100, proteinPer100, fatPer100, carbsPer100` | per 100 g/ml, or per ONE piece (`basis_of`, recipe.rs:53-55) | supplies.rs:158-161 |
| `costPerBasis` | minor units per basis — a STATIC list price typed by the owner | supplies.rs:162 |
| `weightPerUnit` | grams per piece, only for `unit` supplies | supplies.rs:163 |
| `lowAt` | manual reorder threshold in base units | supplies.rs:155 |
| `nutritionConfirmed`, `active`, `supplier` (free text, "a note, until F3's receipts name one") | supplies.rs:164-168, :51-53 |

Unit change after creation is refused (supplies.rs:98-108) — correct, the ledger's integers would change meaning.
Absent keys are dropped, not stored as null (supplies.rs:143-147) because the catalogue image spends a cell per byte.

**Not present on a supply:** any yield/loss %, pack/conversion (kg, box, 500 g bottle), expiry/shelf-life, storage zone, tax
rate, allergens (allergens live on the dish only, `crates/dowiz-hub/src/allergens.rs`), supplier price list, a `prep`/semi-finished kind.

### 1.2 Recipes (BOM) and what follows from them

- Stored `bom` is lean `[{supply, qty}]` only (`crates/dowiz-hub/src/catalog/bom.rs:22-26`; written by `set_bom`,
  `workers/api/src/recipe/apply.rs:52-101`). `qty` is ONE portion, integer, in the supply's base unit, ≤ `QTY_MAX` 100 000 (recipe.rs:43).
- On read, every line is re-scaled from the supply AS IT IS NOW (`lines_of_stored`, recipe.rs:188-200; `line_of`, :88-113).
  Per line: nutrition = per100 × qty/100 (food kinds only), `cost` = `costPerBasis × qty/basis` (the static list price), `weight_g` =
  **`qty` for g, `qty × 1.0` for ml (`ML_TO_G`, :46), `weightPerUnit × qty` for pieces** (:95-99).
- `derive()` (:134-158) sums per portion: kcal/protein/fat/carbs, `nutrition_complete` (every food line has kcal), `cost`
  (only if every line has one), `weight_g` (only if every food line has one), and the customer's ingredient name list.
- `set_bom` writes `nutrition`/`weightG`/`ingredients` onto the dish unless the owner typed them (`Typed`, apply.rs:17-34), and `cost` (apply.rs:98).
- Taste: five axes, levels 1-3, authored per dish, never derived (recipe.rs:39-41, 229-244).
- Modifiers/options (`crates/dowiz-hub/src/modifiers.rs`) price the basket; **an option has no BOM and never touches stock**
  (no `bom` in modifiers.rs; `bom_lines` are built from `catalog.product(&it.product_id)` only, `workers/api/src/storefront.rs:1239-1243`).
- Import (`crates/dowiz-hub/src/import/recipes.rs`, `.../recipes/cards.rs`): the RECIPES file accepts `dish, ingredient, qty|gross, [unit, net,
  yield, prepack, batch, from, to]`. Gross is the reserved quantity (cards.rs:192-195: "only a net quantity; the reserved quantity is gross");
  **net and yield are parsed and then dropped** — `DraftLine.net/yield_` (recipes.rs:96-102) never reach `BomLineIn` (`lines_of`,
  `workers/api/src/services/catalogue/import/bulk.rs`, fn `lines_of` → `{supply, qty}` only). Semi-finished products are FLATTENED to
  leaf supplies in exact fractions at import (`recipes/flatten.rs:1-6`, cards.rs:3-8); a leaf that is not a whole base unit refuses the dish.

### 1.3 The stock ledger (`crates/dowiz-hub/src/stock.rs`)

Event family (stock.rs:85-129) and the fold rules (`decide` :233-354, `apply` :407-481):

| event | fields | effect | who writes it |
|---|---|---|---|
| `Received` | item, qty | on_hand += qty | owner route (`services/operations/stock.rs:124`); MCP `stock_move` (`mcp/tools.rs:71`) |
| `Reserved` | item, qty, order_id | reserved += qty; refused with typed `OutOfStock` if qty > available (= the automated 86) | `command/place.rs:130-133` via `reservations_for` (stock.rs:1238-1268) |
| `Consumed` / `Released` | item, qty, order_id | settle a reservation (I3 linkage checked) | `command/advance.rs:127-133`, `command/refund.rs:286-290` via `settle` (stock.rs:1271-1284) |
| `Wasted` | item, qty, reason ∈ {spoiled, dropped, unsold, returned, staff_meal}, by | on_hand −= qty; cannot eat a reservation | owner/`open_till` staff route (stock.rs:125-136 in ops) |
| `Stocktake` | item, observed, stocktake_id, by | on_hand := observed (basis reset); refused below reserved | owner route; id = `st_{now_ms}` PER LINE (ops stock.rs:139) |
| `Served` / `Unserved` | item, qty, order_id | till import, may go negative / void puts back | `ebills/import.rs:149-152, :185` |
| `Returned` | item, qty, order_id, resell, by, chosen_by | food back from a door | `command/refund/returned.rs:84-93` |

Properties verified: integer `Qty` (:25); fold is pure and item-sorted (:484-490, :215-222); reservations of one basket are one
decision (`append_all`, :1057-1074); signer required for new `Wasted`/`Stocktake`/`Returned` (`signed`, :504-513); stranded holds are
surfaced (:497-501, ops stock.rs:85-91); chained content ids (:951-957). Test counts: 26 in stock.rs, 7+6+7 in
`stock/{tests,served_tests,returned_tests}.rs`, 5 in `services/operations/stock/tests.rs`, 6 in waste tests, 11 in `recipe/tests.rs`,
16 in `import/recipes/tests.rs`.

Storage: an `EvLog` (`bebop_store::evlog`) in its own image (`IMAGE_STOCK`), born at 64 KiB and doubled by `grow()` when full
(stock.rs:1013-1031; `hubstore.rs:577-593`); it never refuses (`usage_of_kind(.., grows=true)` → ceiling `i64::MAX`, `lib.rs:95`).
`DEFAULT_STOCK_BYTES` 8 MiB is "roughly a year" (stock.rs:858) — an estimate, not a measurement.

### 1.4 Cost (`crates/dowiz-hub/src/stock/cost.rs`) — built, tested, and UNREACHABLE

- `Price { unit_cost, per, supplier, doc }` rides on the same `received` record as extra JSON fields (:37-45, `priced_payload` :173-185);
  `decode` ignores them so the shelf fold is unchanged (:5-10). `receive_priced` (:200-210) is the write door.
- `CostBook` is a **weighted moving average per supply, in millionths of a minor unit (i128)** folded over the raw log (:12-30, :56-162):
  a priced receipt joins at its price; unpriced receipts/resold returns/voids join at the current average; draws leave at the average;
  a stocktake re-bases the pool at the average. `dish_cost(bom)` (:122-131), `stamp` (:228-231) and law-8 `rebuild` (:234-236). 7 tests.
- **Nothing in `workers/api/src` or the console calls `receive_priced`, `cost_book`, `stamp` or writes `unit_cost`** (grep over
  `workers/api/src`, `workers/api/public`, `e2e`, `tools`: zero hits outside `stock/cost.rs`). Commit `9fd5292f` (E1) added only the two hub
  files; `0dd8a6d3` "wire … cost" touched `stock.rs` only. The doc comment in `catalog/bom.rs:11-12` ("a dish's cost AT SALE is stamped
  from the ledger's own purchase fold") describes an intent, not the tree: `command/place.rs` computes no cost and the order envelope
  carries none (`grep '"cost"' workers/api/src/command workers/api/src/fold.rs workers/api/src/storefront.rs` → nothing).
- What the console shows as "food cost" is the static `costPerBasis` sum (`admin/menu.js:281-293, :320`), not a purchase-based cost.

### 1.5 Owner routes and console

Routes (`workers/api/src/lib.rs:396-403`): `GET /api/owner/stock` (levels + stranded, folds the whole log: ops stock.rs:28-93),
`POST /api/owner/stock/:kind` (received | wasted | stocktake; body `{item, qty|observed, reason}`, signer = the authenticated
principal, 404 unknown supply, 409 on refusal: :95-207), `GET /api/owner/stock/waste` (waste.rs:154-168), `POST /api/owner/supplies`,
`POST /api/owner/supplies/:id/retire`, `POST /api/owner/{supplies,recipes}/import[?apply=1]` (bulk.rs:28-34).

Console (`workers/api/public/admin/stock.js`, 142 lines): supply list with gauge against `lowAt` (:53-83), supply sheet (:86-121,
fields: name, id, kind, category, unit, nutrition, cost per basis, weight per piece, min level), movement sheet (:123-142: three
chips received/wasted/counted, ONE quantity field, waste reason chips). Recipe editor lives in `admin/menu.js:264-372`.
`admin/bulk.js` is the CSV import. **There is no screen for the waste report** (no `stock/waste` fetch anywhere in `public/admin`),
no purchases list, no stocktake list, no supplier, no lot, no cost history. `GET /api/owner/analytics` (`services/analytics/fold.rs:36-53`)
reports orders/revenue/channels/top dishes and **no cost or margin**.

### 1.6 Where the CPU goes today

Every placement in the Durable Object loads the order-log image, the stock image and the settings image, then `decide()` folds the
stock log from genesis (`hubdo.rs:614-660`; `StockLog::append_all` → `self.ledger()` → `events()` decodes every record with the
minijson scanner, stock.rs:932-940, :1033-1035). `StockLedger` keeps `levels`, `open`, `served` as `Vec`s with linear `find`
(:209-216, :218-231, :371-405), so a fold is O(events × live items). `GET /owner/stock`, the waste report and any cost book do the same
walk again; `cost_book_at` walks the raw payloads a second time (cost.rs:190-224). **Unmeasured** how many events fit in the Free
plan's 10 ms; the structure guarantees it is bounded by a number of events, not by time, and the ledger is currently near-empty on
every venue (memory: opening stock never entered), so nothing has hit it yet.

### 1.7 Live data

dubin-sushi: 75 supplies, 73 recipes, 165 dishes (memory `dubin-sushi-inventory-applied-2026-09-25`, not re-verified here);
no opening stocktake. By the code this means: `available` is 0 for every supply (an unknown item folds to the default level,
stock.rs:218-224), a `Reserved` is refused when `qty > available` (stock.rs:281-289), `reservations_for` emits a `Reserved` for
every `bom` line of every ordered dish (stock.rs:1238-1268, fed from `storefront.rs:1239-1243`), and the refusal reaches the
customer as **409** `Refused::Stock("salmon: 40 wanted, 0 available")` (`crates/dowiz-hub/src/room/mod.rs:44, :71`). So **every one
of the 73 dishes that received a recipe on 2026-09-25 should be un-orderable on the live venue right now** — no flag switches the
ledger off (grep for a stock feature/setting: none; `features.rs:188` only bans the word in generated copy). Whether that is
happening live is **unverified** (no probe was run) and is the first thing to check (§5, R0). An opening stocktake (R1) is the fix.

### 1.8 Defects and gaps found

1. **Cost is dead code end to end** (§1.4): no priced receipt can be written from any surface; no stamp on any order; the console's
   "food cost" is the list price. Every margin figure is therefore hand-typed and stale by construction.
2. **Stock events carry no time.** `encode` writes no timestamp (stock.rs:526-577) and `evlog::Record` has none
   (`crates/bebop-store/src/evlog.rs:86-92`); waste.rs:41 says so ("the stock log carries no clock"). Nothing per-period (usage per
   week, price trend, days of cover, expiry) can be computed from the stock log alone.
3. **`Received` has no supplier, document, lot, expiry** on the reachable path (ops stock.rs:124). `supplier` on the supply is a note.
4. **No batch/lot concept anywhere**; no FIFO/FEFO; no expiry; no traceability from an order to what it was made from.
5. **Stocktake is one event per line with a fresh id per line** (`st_{now_ms}`, ops stock.rs:139); no session, no `expected`
   captured, no drift reported to anyone — §4.3 of the design (`docs/design/DELIVERY-EDGE-CASES-…:342-374`) specifies a typed
   discrepancy report `{item, drift, stocktake_id, window}`; it was never built.
6. **Waste has a reason but no value**, and the report has no UI (§1.5).
7. **Decimal quantities are refused with a serde message.** The movement sheet uses `inputmode: decimal` and posts `Number`
   (stock.js:128, :137-138); the body is `Option<i64>` (ops stock.rs:100) → "bad request body: invalid type" 400 for `1.5`.
   There is no kg/l/pack entry; the owner must type grams.
8. **Gross/net/yield are lost at import** (§1.2) and absent from the editor; dish `weightG` = Σ gross grams, which over-states the
   plate weight of anything trimmed and under-states rice.
9. **Semi-finished products are flattened**, so a sauce made in batches cannot be stocked, counted or wasted as itself.
10. **Options never consume stock** (§1.2) — an "extra salmon" modifier is free in the ledger.
11. **A `Refused::Stock` at checkout is written nowhere** (place.rs:132 returns and the DO drops both images, hubdo.rs:655-658):
    lost sales from stock-outs are invisible.
12. **`ML_TO_G = 1.0`** (recipe.rs:46) — soy sauce, mirin, mayonnaise are 1.05-1.2 g/ml; small, but it is a weight the storefront prints.
13. The `hub/` split in the working tree is unrelated but blocks a clean build until finished (noted, not analysed).

---

## 2. What established systems do (outside research)

- **Poster POS** (the incumbent most Ukrainian operators know). Tech card lines carry **брутто and нетто**; a paper-clip toggle
  derives netto from a per-ingredient **loss %** (or a chosen cooking method); stock is written off in **brutto**, the dish's
  **вихід** is netto; expansion is a negative loss (rice −100 = doubling); an ingredient inside a modifier is written off at the
  modifier's gross and a semi-finished product inside a modifier applies its own losses. Cost = **weighted average of deliveries**,
  averaged between full stocktakes; no manual cost edits; if stock went negative the last known average is used; no delivery history →
  cost 0 and the card is flagged. Stocktake result = actual − planned (sales, production, transfers, write-offs); small gram-level
  deviations are normal; investigate by largest deviation first. Sources:
  https://knowledge-base.joinposter.com/uk-ua/how-to-specify-gross-and-net-weight-manually ;
  https://knowledge-base.joinposter.com/uk-ua/how-the-cost-is-calculated ;
  https://knowledge-base.joinposter.com/uk-ua/how-to-prepare-a-month-end-report ;
  https://knowledge-base.joinposter.com/uk-ua/faqs-working-with-inventory-in-poster (unopened, search snippet: surplus/shortage costed at WAC).
- **iiko / Syrve.** Tech card has **losses at cold processing and at hot processing** as % per line; the "direction of recalculation"
  lets the cook enter the finished output and have gross/net derived; write-off of ingredients or of a finished prep ("акт
  приготування") when a dish is made in production; semi-finished products (`getPrepared`) expand to leaves. Sources:
  https://ru.iiko.help/articles/iikochain-7-7/topic-250/a/h2_1748669673 ; https://lemma-group.ru/articles/kak-sozdat-ili-izmenit-tekhnologicheskuyu-kartu-v-iiko/ ;
  https://open-s.info/blog/tehnologicheskie_karti_iiko/ (search snippets; pages not opened).
- **Toast xtraCHEF.** Two yields per ingredient: **Yield** (loss is waste → adjusts true cost, e.g. 10 oz ribeye → 8 oz = 80 %) and
  **Consumable portion** (the trim is reused elsewhere → cost not adjusted); a **default yield per product**, overridable per recipe
  line; a **density / mass-to-each** converter for unit conversions. AvT report: actual consumption = opening + purchases − closing,
  theoretical = recipe qty × sold, variance = actual − theoretical, per item. Sources:
  https://support.toasttab.com/en/article/xtraCHEF-Recipe-Yield-and-Usable-Yield ;
  https://support.toasttab.com/en/article/xtraCHEF-Get-Started-With-Actual-vs-Theoretical-Analysis-Reports
- **MarketMan / Apicbase / Restaurant365 / Crunchtime / meez / Supy.** Food cost variance % = (actual − theoretical)/revenue; well-run
  operations hold **< 2 %**, > 3 % is "investigate"; break variance down per ingredient because proteins hide behind dry goods; par
  level = (weekly usage + safety stock)/deliveries per week, or ADU × lead time + safety. Sources:
  https://www.marketman.com/page/actual-vs-theoretical-food-cost-report ; https://get.apicbase.com/food-cost-variance/ ;
  https://supy.io/blog/learn-theoretical-food-cost-vs-actual-guide ; https://www.getmeez.com/blog/actual-vs-theoretical-food-costs ;
  https://www.marketman.com/page/how-to-calculate-restaurant-inventory-par-levels ;
  https://www.unleashedsoftware.com/blog/par-levels-in-inventory-management-with-formula-examples/
- **Costing method.** FIFO and weighted average are both accepted (IAS 2; LIFO is not); WAC is "the middle ground", easier to run,
  and what Poster uses; FIFO needs lots and gives lot-exact COGS. Sources: https://www.synergysuite.com/blog/top-three-methods-for-inventory-costing/ ;
  https://www.sculpturehospitality.com/blog/fifo-vs-lifo-vs-wac-what-restaurant-inventory-costing-method-is-best ;
  https://www.marketman.com/blog/inventory-valuation-for-restaurants
- **Lightspeed / Square for Restaurants**: Square itself only records sales; ingredient tracking, par alerts and variance come from
  an add-on such as MarketMan (https://www.marketman.com/page/square-restaurant-inventory-management). Lightspeed: not researched (unverified).

### 2.1 Albanian / EU context that bears on the design

- **Allergens**: the dish already carries the EU-14 with a three-way declaration (`allergens.rs`); nothing to add for stock.
- **Traceability**: Reg. (EC) 178/2002 art. 18 = one step back / one step forward per lot
  (https://food.ec.europa.eu/system/files/2016-10/gfl_req_guidance_rev_8_en.pdf). Albania's Ministry of Agriculture instruction
  "On the indicators identifying the lot" makes an **"L" lot code on every food label mandatory from 1 January 2026**, monitored by AKU
  (https://albaniandailynews.com/news/new-food-security-measures-introduced-denaj). So every delivery a Durrës kitchen receives after
  that date carries a lot code the receiving screen can copy. Law 9863/2008 "On food" requires HACCP self-control for every food
  business (https://leap.unep.org/en/countries/al/national-legislation/law-no-9863-food); WIPO Lex lists it as repealed
  (https://www.wipo.int/wipolex/en/legislation/details/17650) while a Dec-2025 paper still cites it — **which law is in force today is
  unverified**; the HACCP obligation is not in doubt.
- **Raw fish**: Reg. (EC) 853/2004 Annex III Section VIII Ch. III D requires a freezing treatment of fishery products to be eaten raw
  (−20 °C ≥ 24 h, or −35 °C ≥ 15 h — figures from the regulation text, not re-read here) and records of it; Italian guidance expects
  "products, quantities and date of each freezing operation" to be logged (https://www.foodtimes.eu/food-system/fish-downed-or-frozen/).
  For a sushi venue the lot record is therefore also the parasite-treatment record: `lot.frozen: {at, hours, temp}` or `supplierTreated: true`.

---

## 3. Recommended target model

Design rule kept throughout: **the ledger's fold reads what it reads today** (`item`, `qty`, `order`, `observed`, …). Everything new
rides as extra JSON fields on the same records (the pattern `cost.rs` already uses, cost.rs:5-10) or as new record kinds that old
readers skip (`decode` returns `None` for an unknown `k`, stock.rs:627 — **verify** that every caller of `events()` tolerates a skipped
record: `StockLog::events` uses `filter_map`, so yes). No SQL, one image per venue, integers only.

### 3.1 Supply record — additions

```
cleanPm:  int   ‰ of gross that remains after cold processing (whole salmon → fillet 550; peeled onion 900); default 1000
cookPm:   int   ‰ of net that remains after hot processing (rice dry → cooked 2200; seared tuna 850); default 1000
nutritionBasis: "raw" | "cooked"   which weight the per-100 figures describe (tables are usually raw); default "raw"
packs:    [{name:"кг", qty:1000}, {name:"ящик 5 кг", qty:5000}]   receiving/price entry conversions to the base unit
shelfDays: int   default expiry offset for a lot received without a date (optional)
kind: + "prep"   a semi-finished product the kitchen makes (has its own bom + batch, §3.4)
supplier: keep as the DEFAULT supplier; the real one is on each receipt
```
Consumable-portion style "trim reused" (xtraCHEF) is expressed by a `Produced` event that yields two outputs (§3.4), not by a flag.

### 3.2 Recipe line — gross / net / out (the operator's new requirement)

Where the weights live: **both**. The supply carries the defaults (`cleanPm`, `cookPm`); a line may override with explicit
numbers, because a cut varies per dish (a nigiri slice vs. a tartare) and because measured yields (§3.4) are per supply.

```
bom line: { supply, qty, net?, out? }
  qty = GROSS in base units  (unchanged; what the ledger reserves and what import already treats as gross)
  net = after cleaning       (default: round(qty × cleanPm / 1000))
  out = after cooking / on the plate (default: round(net × cookPm / 1000))
derived per line, never stored: cleanLoss = 1 − net/qty; cookLoss = 1 − out/net; yieldPm = out×1000/qty
```
Rules:
- **Stock consumption uses `qty` (gross)** — `bom_of` (stock.rs:1211-1234) and `reservations_for` are untouched.
- **Dish `weightG` = Σ `out` of food lines** (today Σ `qty`, recipe.rs:95-99, :143-147).
- **Nutrition per portion**: if `nutritionBasis = raw`: kcal = kcalPer100 × net/100 (calories are in the edible net part; cooking
  loss is mostly water so the per-portion total is conserved); if `cooked`: kcal = kcalPer100 × out/100. Add
  **per-100 g-as-served = Σ kcal × 100 / Σ out** to the dish view: this is the number that rises with cooking loss and is what a
  label prints. Protein/fat/carbs the same way.
- **Cost per line = purchase cost × gross** (unchanged: the kitchen paid for the gross); show "cost per usable g" = cost/out as the
  yield-adjusted price (Apicbase/xtraCHEF sense) on the supply sheet and the line.
- Import: `cards.rs` already reads `gross|qty`, `net`, `yield` → pass them through `BomLineIn` instead of dropping them; a yield
  column of 55 % becomes `out = qty × 55/100` when `net` is absent (Poster/iiko exports put the combined loss there).
- Editor: gross field; net and out shown derived (Poster's paper-clip: linked = derived from the supply's ‰; unlinked = typed).
- Exact changes in `recipe.rs`: `Line` gains `net: i64, out: i64`; `line_of` computes them from the supply defaults or the stored
  line; weight rule at :95-99 becomes `out` (for `unit` supplies: `out = weightPerUnit × qty × cleanPm × cookPm`); nutrition scaling
  at :91-93 uses net or out per `nutritionBasis`; `bom_json`/`bom.rs::to_json` write `net`/`out` only when they differ from the
  defaults (keeps the catalogue image lean: ≈ +0-24 bytes per line, 73 recipes ≈ +10 KiB worst case against a 10 MiB ceiling);
  `lines_of_stored` reads both the new and the old lean lines; `derive` sums `out`. The console's mirror in `menu.js:281-293` must change identically.

### 3.3 Stock records — additions (all optional on the wire, old records fold as before)

```
every new record:      at: ms (the Worker's one clock, ctx.data.now_ms), by where a person acted
received:  + unit_cost, per (already in cost.rs), supplier, doc (invoice no.), lot (the label's L-code or a hub-minted id),
             expiry (local day), pack {name, count} (as typed; qty stays base units), frozen {at, hours, tempC} | treated: true
wasted:    + lot (optional: which batch spoiled), value_micro (derived at write from the cost book; stored so reports need no re-fold)
stocktake: + session (one id for the whole count), expected (the folded on_hand at write), so drift = observed − expected is a row, not a re-fold
consumed/served: + lots [{lot, qty}] OPTIONAL — explicit pick by the kitchen (raw fish); absent = derived FEFO (§3.5)
new kinds:
  snapshot   { at_len, levels[], open[], served[], pools[] }  — the checkpoint (§3.6); the fold starts at the newest one
  produced   { out: supply, out_qty, lot?, inputs: [{supply, qty, lot?}], by, at }  — a prep batch or a fish breakdown (§3.4)
  refused    { item, wanted, available, at }  — NOT a ledger event (does not move stock); a telemetry row for lost sales, or written to errlog instead (decision)
```

### 3.4 Semi-finished products (напівфабрикати) and measured yield

Two honest modes, chosen per supply, because a sushi kitchen does both:
- **Stocked prep** (`kind: prep`, `stocked: true`): the prep is a supply with its own `bom` and `batch` (e.g. spicy mayo 1 000 ml from
  5 lines). The kitchen records a **`Produced`** act: inputs leave (gross), the prep enters (net out), its lot is minted with the
  production time and `shelfDays`. Dishes list the prep as a normal line; orders reserve the prep. **Measured yield = out_qty /
  Σ inputs**; expected = the prep's recipe batch. The same event records a **fish breakdown**: `Produced { out: salmon-fillet 2 750 g,
  inputs: [salmon-whole 5 000 g], lot: from the whole fish's lot }` → measured cleanPm 550 vs. the supply's declared 580 → a yield-
  variance row; "adopt as default" is an owner click that writes `cleanPm` on the supply (a catalogue write, never automatic).
- **Expanded prep** (`stocked: false`): today's behaviour (flatten at import / at reservation) for preps nobody counts.
  Flattening moves from import time to reservation time only if the operator wants recipes to keep naming the prep; otherwise leave
  `flatten.rs` as is and only add the stocked mode. (Decision, §6.)

### 3.5 Lots (партії), FEFO, traceability

- A lot is not a table; it is what the fold makes of `received[lot]`, `produced[lot]` and the draws. Lot fold: per supply, open lots
  ordered by (expiry, receive order); each draw (`consumed`, `served`, `wasted`, stocktake shrink) is allocated **FEFO** across open
  lots unless the record names its lots explicitly. Output: remaining per lot, expired-with-remaining, and the order → lots and
  lot → orders maps (traceability, one step back = `supplier`+`doc`, one step forward = order ids / table / customer hash).
- Derived FEFO is only true if the kitchen rotates FEFO — which HACCP already requires. For raw fish the operator may prefer an
  explicit pick at `PREPARING` (a lot chooser on the kitchen screen); the field exists either way.
- Stocktake with lots: the count is per supply (as today); the fold shrinks the OLDEST lots first (so drift lands on what should have
  been used) — the same rule Poster documents as "result = actual − planned" per product, with lot allocation as an internal detail.

### 3.6 Costing recommendation: WAC as the book cost, lots for traceability and an optional FIFO view

Recommend **weighted average (already built in `cost.rs`)** as the cost every stamp, margin and valuation uses, because: (a) it is what
Poster — the system the operator's peers and accountants in Ukraine know — computes, so figures are comparable; (b) it needs no lot
bookkeeping in the money fold and is integer-exact in millionths (cost.rs:29-30); (c) at a small venue with 1-3 deliveries a week per
fish, FIFO makes two orders ten minutes apart show different margins for the same roll, which reads as noise, not insight; (d) IAS 2 and
Albanian practice accept both. Keep **FIFO-by-lot as a report** (§4, A7) computed from the lot fold when lots carry prices, for the
accountant and for "what did that expensive salmon actually cost us". Poster's "average between full stocktakes" is exactly what the
`Stocktake` re-basing in cost.rs:152-159 does, plus a `snapshot` reset per §3.7.

Stamping: `place.rs::decide` gains `items[i].cost = stamp(stock, bom).cost` and `cost_at = stamp.at` per line (BLIND-SPOTS §2.10, verbatim
the plan); `rebuild` (cost.rs:234) is the law-8 gate. The stamp must come from the same in-memory fold as the reservation to stay
within one object turn (§3.7).

### 3.7 Fitting the append log with bounded CPU (Free plan, 10 ms)

1. **One pass, one struct.** Fold levels, open holds, served map, cost pools and lot state in a single walk (a `StockState` that
   `apply`s each raw record once); today levels and cost are two walks with two decoders.
2. **Snapshot record = checkpoint.** A `snapshot` record holds the whole `StockState` at its position. `EvLog::walk` is newest-first
   (stock.rs:925-931), so the reader stops at the first snapshot it meets and applies only the tail, oldest-first: the fold cost becomes
   O(events since the last snapshot). Written (a) at every stocktake session close (natural: the count re-bases everything anyway —
   Poster's "cost averages between full inventories"), (b) automatically when the tail exceeds K records (say 2 000), inside the
   same object turn that appended. Determinism gate: `fold(all) == fold(snapshot + tail)` over generated histories, byte for byte.
3. **Chained ids stay tamper-evident**: the snapshot is just a record in the chain; a rebuild from genesis must reproduce it (law 8).
4. **Lot and per-period reports never run on the placement path**: they are owner GETs over the tail + snapshot; anything by period
   older than the newest snapshot reads the earlier snapshots' aggregates (each snapshot also stores per-period sums since the previous one:
   received/consumed/wasted qty and value per supply) so a 30-day report is a sum of ≤ 30 snapshot deltas plus one tail, not a year walk.
5. **Archive**: the stock log grows without a ceiling (§1.3); a monthly snapshot lets a later row move records older than N snapshots to
   R2 (the same shape as the order-log archives, `hubstore.rs:1202-1235`) without changing any fold.
6. Bytes: a dated, priced, lotted `received` is ≈ 150 bytes vs ≈ 45 today; at 5 events/order and 100 orders/day the tail between weekly
   snapshots is ≈ 3 500 events ≈ 250 KB — still one image read per placement (the DO reads the whole image, hubdo.rs:632-643), so the
   archive row matters within months, not years. **All unmeasured**; the R7 row includes the measurement.

---

## 4. Analytics catalogue

Formulas use: `R` = received qty/value, `C` = consumed + served (recipe-driven draws), `W` = wasted, `S_k` = stocktake k
(`observed`, `expected`), `WAC_t` = cost book average at time t, sold(d) = quantity of dish d in `Placed` items not later voided.

| # | report | formula | events needed | UI home |
|---|---|---|---|---|
| A1 | Shelf now (exists) | fold | all | Stock list (exists) |
| A2 | Waste by reason / signer / item, **valued** | Σ W qty; value = Σ qty × WAC_at | `wasted`, `returned{resell:false}`, order `Amended{dropped}` (waste.rs:105-138); value needs R3 | Stock → "Списання" (route exists, no screen) |
| A3 | Stocktake variance (shrinkage) | per line drift = observed − expected; value = drift × WAC; per session Σ, top ± items; drift trend per item across sessions | `stocktake{session, expected}` (R1) | Stock → "Інвентаризації" |
| A4 | Theoretical vs actual usage per supply, per window between two counts | actual = S_prev.observed + R − S_now.observed; theoretical = Σ C; explained = Σ W; **unexplained = actual − theoretical − explained**; as % of theoretical and in money | R1, R2, R3 | More → Numbers → "Розхід інгредієнтів" |
| A5 | Food cost % per dish and per period | dish: stamped cost / unit_price; period: Σ stamped line costs / revenue; also theoretical (recipe × WAC now) vs stamped | R4 (+R3) | dish sheet (replace the static number, menu.js:320) + Numbers |
| A6 | Margin per dish, menu engineering | margin = price − stamped cost; quadrant by popularity (sold/d) × margin; "stars/plowhorses/puzzles/dogs" | R4 + `Placed` | Numbers → dishes |
| A7 | Purchases by supplier / period, supplier price trend, FIFO-by-lot COGS view | Σ R value by supplier/doc; unit_cost per base unit over time per supply; COGS_FIFO from lot fold | R2, R3, R8 | Stock → "Постачання" |
| A8 | Top consumed supplies by qty and value | Σ C per supply, × WAC | R2 (dates) | Stock → "Розхід" |
| A9 | Days of cover, reorder suggestion | ADU = Σ C over last 14 d / 14; days = available / ADU; par = ADU × (lead + cycle) + 25 % safety (MarketMan/Unleashed); order = par − available; `lowAt` stays as a manual floor | R2 | Stock list badge ("≈ 3 дні"), order sheet |
| A10 | Lots: expiring within N days, expired with stock, lot → orders, order → lots (HACCP), freezing-treatment log | lot fold (§3.5) | R8 | Stock → "Партії"; CSV export for AKU/HACCP |
| A11 | Yield: measured vs expected per prep / fish | measured = out/Σ in per `produced`; expected = supply ‰; trend | R9 | Stock → "Виробництво" |
| A12 | Inventory valuation | Σ available × WAC at a snapshot; opening/closing for the accountant | R3, R7 | Numbers |
| A13 | Lost sales from stock-outs | count of `refused` per item and hour | R14 | Stock list ("відмов: n") |
| A14 | Nutrition per 100 g as served (label) | Σ kcal × 100 / Σ out | R6 | dish sheet, storefront |

---

## 5. Phased roadmap (small shippable rows, by value to an owner)

Each row: files · the test that proves it · risk · [D] = needs an operator decision. Rows follow the repo rules: one image
write per request, no SQL, coverage ratchet, old images must fold identically.

**Phase 0 — see whether the ledger is already biting**
- **R0 Probe** (no code): on the live venue, place a test order for a dish that has a recipe and read the refusal, and `GET /api/owner/stock`.
  If the 73 recipe dishes are being refused at checkout, R1 is urgent (an opening count fixes it); if not, note why (unverified today).

**Phase 1 — make the existing ledger usable (2-4 rows, days)**
- **R1 Stocktake session**: `POST /api/owner/stock/stocktake` takes `{session?, lines:[{item, observed}]}`, folds ONCE, appends all
  lines as one decision (`append_all`) with one `session` id, `at`, and `expected` per line; response returns drift per line. Console:
  "Інвентаризація" sheet listing every active supply with `expected` prefilled, decimal + pack input (fixes defect 7), summary of drift
  after save. Files: `services/operations/stock.rs`, `stock.rs` (decode tolerates the new fields; the design's §4.3 drift report),
  `admin/stock.js`, `i18n.js` (uk/en/sq). Test: hub test "an image written before sessions folds to the same ledger" (extend
  `stock/tests.rs:82`); handler test that a 3-line count with one short line writes nothing; drift equals observed − folded on_hand.
  Risk: low. This row also unblocks the opening count for dubin-sushi (75 lines, one request).
- **R2 Dated records**: `at` on every new record; `reservations_for`, `settle`, ebills import and the owner routes pass `now_ms`
  (the clock the Worker already reads once). Files: `stock.rs` encode/decode, `command/place.rs`, `command/advance.rs`,
  `command/refund*.rs`, `ebills/import.rs`, `services/operations/stock.rs`. Test: old fixture folds identically; a new record round-trips
  `at`; `stock/cost/tests.rs` unchanged. Risk: low-medium (signature change across six call sites; the one-image gate must stay green).
- **R3 Priced, sourced receiving**: `POST /stock/received` accepts `{qty | pack:{name,count}, unit_cost, per, supplier, doc, lot, expiry}`
  → `receive_priced` (extend `Price` with lot/expiry/at); supplier and pack pickers in the console (last used per supply); `GET /owner/stock`
  adds `wac` per supply next to `costPerBasis`. Files: `stock/cost.rs`, `services/operations/stock.rs`, `admin/stock.js`, `i18n.js`.
  Test: the BLIND-SPOTS §2.10 CHECK through the handler (1 000 then 1 200 per kg → WAC 1 100; refused price writes nothing). Risk: low.
  [D] price entry per kg/per pack (both via `per`); whether `doc` is required.
- **R5 Waste screen with value**: list + totals (route exists) + value via the cost book. Files: `admin/stock.js` or new `waste.js`,
  `services/operations/waste.rs`. Test: waste tests gain `value`. Risk: low.

**Phase 2 — the recipe model the operator asked for (1-2 weeks)**
- **R6 Gross / net / out** as §3.2: supply `cleanPm`, `cookPm`, `nutritionBasis`; line `net`/`out`; `weightG = Σ out`; nutrition by basis;
  per-100 g-as-served; import passes `net`/`yield`; editor shows брутто/нетто/вихід with linked/unlinked. Files: `recipe.rs`,
  `recipe/apply.rs`, `catalog/bom.rs`, `services/operations/supplies.rs`, `services/catalogue/import/bulk.rs`, `import/recipes/cards.rs`,
  `admin/menu.js`, `admin/stock.js`, `i18n.js`, `e2e/gates/recipes.mjs` (report gains "dishes whose weight is net"). Test (recipe/tests.rs):
  salmon 100 g gross, cleanPm 550 → net 55, out 55, weight 55, kcal = kcalPer100 × 0.55; rice 100 g dry, cookPm 2200 → out 220;
  a stored lean line without `net`/`out` reads with the supply defaults; `bom_of` still returns `{supply, qty}`; an explicit `out` beats
  the default. Risk: medium — every dish with a recipe changes its published `weightG` on its next save (expected, but say so in the
  release note); the catalogue image grows by ≤ ~10 KiB. [D] `nutritionBasis` default (raw is the safe default for tables).
- **R4 Cost stamp on the order line** (BLIND-SPOTS §2.10): `decide()` stamps `items[i].cost`/`cost_at` from the same fold; `rebuild`
  gate; the dish sheet and A5 read stamps. Files: `command/place.rs`, `hubdo.rs`, `fold.rs` (carry_over), `rebuild.rs`, `storefront.rs`
  (envelope shape), `admin/menu.js`. Test: an order placed between two receipts stamps the earlier WAC; `cost::rebuild` reproduces every
  stamp over a fixture log. Risk: medium (adds a cost-book pass to the placement path → do R7 first or fold both in one pass).
- **R7 One-pass fold + snapshot checkpoint** as §3.7. Files: `stock.rs`, `stock/cost.rs` (merge into `StockState`), `hubdo.rs` untouched if
  `ledger()`/`stamp()` keep their signatures. Test: property test `fold(all) == fold(snapshot ∘ tail)` (levels, open, served, pools) over
  seeded random histories incl. stocktakes and grows; determinism I4; a micro-benchmark that prints events-per-10 ms (recorded in
  `docs/measurements`). Risk: medium — a snapshot that forgets `open` or `last_avg` silently corrupts every later fold; the property test is the guard.

**Phase 3 — batches and prep (2-3 weeks)**
- **R8 Lots** as §3.5: fields on `received`/`wasted`/`consumed`; lot fold (`stock/lots.rs`); expiry list; traceability both ways; CSV
  export; kitchen lot pick optional. Files: `stock.rs`, new `stock/lots.rs`, `services/operations/stock.rs` (+ `GET /stock/lots`),
  `admin/stock.js`, room/kitchen surface for the explicit pick. Test: FEFO allocation is deterministic and total-preserving; an
  expired lot with remaining is flagged; order→lots equals lots→order inverted; explicit lots override FEFO. Risk: medium; depends on R2, R3.
  [D] derived FEFO vs explicit pick for raw fish; freezing-treatment fields.
- **R9 Stocked preps and `Produced`** as §3.4: `kind: prep` with `bom`+`batch`; `Produced` (new variant → `waste.rs:69` exhaustive match
  forces a decision whether it is waste: no); "Виробництво / Розбирання" sheet; yield variance (A11); "adopt measured yield" button.
  Files: `stock.rs`, `recipe.rs`, `services/operations/{stock,supplies}.rs`, `import/recipes/{cards,flatten}.rs` (stocked preps are not
  flattened), `admin/stock.js`. Test: produce 1 whole salmon (5 000 g) → fillet 2 750 g; a dish using fillet reserves fillet; measured
  cleanPm 550 vs declared 580 yields one variance row; inputs' lots propagate to the output lot. Risk: medium-high (import semantics).
  [D] which preps are stocked; whether the kitchen will record breakdowns.

**Phase 4 — analytics (each ≤ 1 week once the data exists)**
- **R10** `GET /api/owner/stock/report?days=7|30` → A4, A8, A9, A3 trend; Numbers pane + Stock badges. Files: new
  `services/operations/report.rs` (pure fold + handler + tests), `admin/more.js`/`stock.js`. Test: fixture history → known numbers
  (incl. "unexplained" = actual − theoretical − waste). Risk: low.
- **R11** A5/A6 into `services/analytics/fold.rs` (`Report` gains `food_cost`, per-dish margin). Test: fold over stamped orders. Risk: low.
- **R12** A7/A12 purchases, price trend, valuation. Risk: low.
- **R13** [D] Options with BOM (`modifiers.rs` option gains `bom`; `storefront.rs:1239` adds chosen options' lines). Test: an "extra
  salmon" option reserves 20 g more. Risk: low-medium.
- **R14** [D] `refused` telemetry for A13 (errlog row or a non-ledger record). Risk: low.

Suggested order: R0, R1, R2, R3, R5, R6, R7, R4, R8, R10, R9, R11, R12, R13, R14. R1-R3 alone turn the ledger on with real costs;
R6 is the operator's explicit ask and is independent of the ledger rows, so it can run in a parallel lane (file-disjoint from R1-R3
except `admin/stock.js` and `i18n.js`).

---

## 6. Open questions for the operator

1. **Cost method**: WAC for every book figure (as Poster), with FIFO-by-lot only as a report — agreed? Does the accountant need FIFO COGS?
2. **Fish**: do you buy whole fish and break it down (needs R9 `Produced` + yield acts), or fillets/loins (a lot on the receipt is enough)?
3. **Nutrition tables**: are your kcal/protein figures per 100 g raw or cooked, and from which source? (Sets `nutritionBasis`.)
4. **Yields**: do you already have a loss % per ingredient (from a Poster/iiko export or your own weighing)? If not, R9 measures them.
5. **Stocktake cadence and who counts**: fish daily, dry goods weekly? Should `open_till` staff be allowed to count (today counts are
   owner-only, ops stock.rs:150-152) — and should counts be taken with the store paused (design §4.3 step 5)?
6. **Receiving**: prices per kg or per pack/box; is the invoice number required on every delivery; should expiry be mandatory for fish?
7. **Raw fish lots**: derived FEFO, or an explicit lot pick in the kitchen at PREPARING (HACCP-friendlier, one more tap per order)?
   Do you freeze in-house (record the −20 °C/24 h treatment per lot) or buy treated?
8. **Modifiers**: do extras (extra salmon, extra sauce) need to draw stock?
9. **Preps**: which semi-finished products are made in batches and should be counted as themselves (sauces, sushi rice, marinated fish)?
10. **Retention**: how long must stock records stay hot in the hub image before archiving to R2 (affects the snapshot/archive cadence)?
11. **Which venue and when for the opening count** (dubin-sushi: 75 supplies), and whether to switch the automated 86 on for all
    73 recipe dishes at once or per category.
