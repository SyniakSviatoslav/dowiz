# SPEC: semi-finished products (напівфабрикати / ПФ) — a tree of tech cards, 2026-09-29

**Status.** Design for lane W-PF (tree `/root/lanes/w-pf` at main `c62cebbe`), written BEFORE the code from the
operator's request (verbatim in the lane card) and the code as it stands. Every path below exists in the tree
unless written `+like/this.rs` (created by this lane). Numbers are MEASURED where a command is quoted, else DERIVED
from the source cited.

**The request, in one paragraph.** Today a recipe line can only name a RAW supply, so a roll cannot say "130 g of
seasoned rice" — it must say dry rice, water, vinegar, salt, sugar, each scaled by hand and wrong the day the rice
batch changes. The operator wants three kinds of item — raw / semi-finished (ПФ) / dish — a ПФ with its own card
(components with gross quantities, a net yield, the derived ratio K), ПФ inside ПФ to any depth, RECURSIVE write-off
of raw leaves at the sale of a dish (130 g of rice-ПФ → 61.9 g dry rice, 12.38 g vinegar, 0.77 g salt, 2.32 g
sugar), and a cost that follows the raw purchase price through the tree with nothing re-saved.

---

## a) Where the type lives

| type | record | key | has a card |
|---|---|---|---|
| raw (Сырьё / Товар) | a supply, `catalog` key `supply:<id>` | `kind` ∈ `food_ingredient, condiment, packaging, utensil, resale` (`workers/api/src/recipe.rs` `KINDS`) | no |
| ПФ (semi-finished) | **a supply** with `"kind":"prep"` and a `"card"` | new kind `prep` | yes: `{"lines":[{"item":"<supply id>","qty":<gross>}...],"yield":<net>}` |
| dish (Блюдо) | a product, key `product:<id>`, with `bom` | — | its `bom`: `[{supply, qty, net?, out?}]`, unchanged in shape |

**Why a ПФ is a supply and not a fourth directory.** A dish's `bom` line names a `supply` id, and everything that
reads a recipe reads that id: the ledger (`crates/dowiz-hub/src/stock.rs` `bom_of`), the columnar `bom` block
(`crates/dowiz-hub/src/block/encode.rs` `project`, `block/view.rs` `Catalogue::bom_of`), the owner's recipe view
(`recipe.rs` `lines_of_stored`), the kitchen numbers (`services/analytics/kitchen.rs` `dishes_of`), the nomenclature
delete (`services/operations/supplies/delete.rs`). A ПФ that IS a supply is nameable by every one of them with **no
format change** to the dish record, the `bom` block or the catalogue key space; the one new thing any reader meets is
a supply record whose `kind` is `prep`. The console already has ONE directory with a kind filter
(`public/admin/stock.js` chips over `KINDS`); the ПФ is one more chip in it, which is the operator's "one directory".

**Card lines.** `item` is a supply id — raw OR another ПФ. `qty` is GROSS, an integer in the ITEM's base unit
(`g`/`ml`/`unit`, `recipe::UNITS`), bounded `1..=QTY_MAX` (100 000, `import::recipes::QTY_MAX` = `recipe::QTY_MAX`),
at most `LINES_MAX` = 40 lines. `yield` is the NET output of the whole batch, an integer in the ПФ's own base unit,
`1..=QTY_MAX`.

**K = yield / Σ gross is DERIVED and shown, never stored.** Σ gross is taken in GRAMS with the rule
`recipe/weights.rs` already applies to a line: g and ml are grams 1:1, a piece weighs its supply's `weightPerUnit`.
So "2 eggs in a sauce measured in grams" is allowed: the egg line is `2 unit`, the write-off takes 2 eggs, the cost
takes 2 × the egg's price; only K needs the egg's weight, and when the egg supply has no `weightPerUnit` **K is
shown as unknown ("—"), not refused** — a kitchen can weigh the batch and write off correctly before it has weighed
an egg. Decided: `weightPerUnit`, never a refusal.

**The ПФ's own base unit** is `g` or `ml` in practice; `unit` is allowed (a "rice ball 20 g" made 50 at a time), with
the same `weightPerUnit` rule for its weight.

**Water and anything nobody counts:** a raw supply with a new optional flag `"untracked": true` (cost 0 unless given).
It is a real line on the card (it is in the recipe and it is in K), it is NOT a leaf of the write-off (nothing is
reserved), and it is left out of the where-used delete list's shelf consequences. Decided: a flag on a raw item, not a
special id.

## b) Recursion, cycles, depth

A ПФ line may name a ПФ, to any depth up to `DEPTH_MAX` = 6 ПФ levels under a dish. **Cycles are refused at save**
(`+crates/dowiz-hub/src/prep.rs` `check_card`), direct (`A → A`) and indirect (`A → B → A`), with the path named in the
refusal text exactly as the importer names it (`import/recipes/flatten.rs`: `"semi-finished products name each
other: a → b → a"`). Also refused at save: an unknown item, a line naming the card itself, a quantity or yield out of
bounds, more than `LINES_MAX` lines, a duplicate item, and a card that would put ANY chain through it past
`DEPTH_MAX` (height below + longest chain of users above).

**Why 6, with the arithmetic.** The expansion is exact (§c) in `i128` rationals; a line contributes a factor
`qty / yield` with both ≤ 10^5, so a chain of `d` ПФ levels has numerator and denominator ≤ 10^(5d) before reduction;
times the portion's own line (≤ 10^5) and the micro scaling (10^6): 10^(5·6+5+6) = 10^41 > i128's 1.7·10^38 in the
WORST case with no gcd reduction — which is why every multiplication is `checked_*` and an overflow is a loud
refusal, never a wrapped number — and ≈ 10^30 in any real kitchen (yields like 1000 and 2100 reduce). No real card is
six deep: sauce-in-sauce-in-marinade-in-dish is three. 6 leaves headroom and keeps every chain readable on a phone.

## c) Write-off at sale — exact through the tree, one rounding per leaf, no drift

**The expansion.** `+crates/dowiz-hub/src/prep.rs` `expand(lines, lookup)` walks a dish's `bom` down through ПФ
cards to RAW leaves with `Rat` (`import/recipes/num.rs`, the importer's exact fraction — reused, not rewritten:
`flatten.rs` now walks through the SAME `prep::tree` recursion), sums per leaf over every path that reaches it, and
converts each leaf ONCE to a **micro-quantity** `uq = round_half_up(rat × 10^6)` in the leaf's base unit, per ONE
portion. The operator's example, per one Philadelphia (130 g of ПФ 2):

    share of the batch  = 130 / 2100 = 13/210
    dry rice            = 1000 × 13/210 = 61.904 761…  g  → uq 61 904 762
    mitsukan (ПФ 1)     =  250 × 13/210 = 15.476 190…  g  (not a leaf: expanded)
      vinegar           =  800/1000 × 15.476… = 12.380 952… g → uq 12 380 952
      salt              =   50/1000 × …       =  0.773 809… g → uq    773 810
      sugar             =  150/1000 × …       =  2.321 428… g → uq  2 321 429
    water               = 1100 × 13/210 = 68.095… g (untracked: not reserved)

**The ledger stays in whole base units.** A finer ledger base (milli-grams) was considered and rejected: every
record ever written (`qty` in `reserved`/`consumed`/…), every `GET /api/owner/stock` number, every typed delivery and
count, every checkpoint and every report would change meaning or need a version flag — the exact rewrite of the
format the module's rule forbids ("a venue's shelf is the fold of its whole history, none of it may stop folding",
`stock/meta.rs`). What changes instead is the WRITER of order lines.

**The carried remainder, as a fold (never a stored counter).** A `reserved` or `served` record written for a leaf
may carry one extra integer key, `"uq"`, the exact micro-quantity of that draw, riding INSIDE the record the way
`unit_cost` rides on `received` (`stock/meta.rs`: `decode` still reads `item`/`qty`/`order`, so the shelf's fold is
untouched). A new fold beside the cost book, `+crates/dowiz-hub/src/stock/carry.rs`, keeps per item
`carry = Σ uq − 10^6 · Σ qty` over the draws it has seen; a record WITHOUT `uq` counts as `uq = qty × 10^6`, so every
record ever written moves the carry by exactly zero and **every old log folds with carry 0 everywhere**. The write
door for order lines (`StockLog::append_draws`, `+stock/carry.rs`) folds the tail as `commit` does and books

    qty = round_half_up((carry + uq) / 10^6) − 0       (the whole units owed so far, minus those already booked)

then appends `reserved {item, qty, order, uq}`. `released` (a cancel) subtracts the SAME `uq` and `qty` it reserved
(the fold remembers the open `(order, item)` → `(uq, qty)`), so a cancel undoes the fraction exactly; `consumed`
leaves the carry as it is (the food was used); `unserved` (a till void) undoes a `served` the same way. The
checkpoint (`stock/checkpoint/codec.rs`) stores the carry as one more section, `X`, **written only when non-empty**,
so every checkpoint already on a live log still verifies byte for byte (`verify_checkpoints`).

**Proof of no drift** (acceptance 1, `+crates/dowiz-hub/src/prep/tests.rs`): after N sales the booked total for a
leaf is `round(N · uq / 10^6)`, by induction on the carry invariant `−0.5·10^6 < carry ≤ 0.5·10^6`; the ONLY error
against the exact value is the per-portion micro rounding, ≤ 0.5·10^-6 per portion → over 1 000 sales ≤ 0.0005 g,
inside one base unit by three orders of magnitude. Salt: 1 000 sales book 774 g (exact 773.81), not 1 000 g (+30 %,
which is what rounding every sale to whole grams did), and not 0 g (what truncation did).

**Zero-quantity order lines.** A first sale of 0.77 g books 1 g; a draw of 0.3 g books 0 g and MUST still be a record
(else the carry forgets it and every later draw of that leaf rounds to 0 for ever — proved in a test). §4's "Always >
0" is therefore relaxed for the four order-linked kinds only (`reserved`, `consumed`, `released`, `served`,
`unserved`: `qty ≥ 0`); a zero hold sits in `open` so its `consumed`/`released` twin still matches. Every other kind
(`received`, `wasted`, `produced`) keeps `> 0`.

**Where the expansion happens.** In the Worker, where the catalogue is loaded, at the three places that build
`bom_lines: Vec<(product JSON, qty)>` for the object (`storefront.rs:885`, `services/orders/aggregator.rs:57`,
`ebills/import.rs:145`): the product JSON handed to the object is `prep::for_ledger(lookup, product_json)`, the same
record with its `bom` replaced by the leaf list `[{"supply":"salt","uq":773810}, …]`. `stock::bom_of` reads a `uq`
line into `BomLine { supply, qty: round(uq/10^6), uq }` (a stored dish never carries `uq`, so the block projection
of a catalogue is byte-identical — `block/tests.rs` `fixtures_are_the_encoder_output` is the proof). The object then
calls `stock::draws_for(order_id, bom_lines)` (sums `uq × ordered` per leaf, sorted) and `append_draws` instead of
`reservations_for` + `append_all_costed` (`command/place.rs:137`, a two-line hand-back; the cost stamp gets the same
book and `at`). Until the hand-backs land, a dish with a ПФ line reserves the ПФ id itself as an uncounted item
(negative, "needs a count", never a refusal) — degraded, not broken.

**ПФ in stock (batch cooked ahead).** OUT OF SCOPE here, on purpose: the operator's request says expand to raw at
sale, and a stocked ПФ needs a `produced` act per batch (inputs leave, ПФ enters — `StockEvent::Produced` already
exists), a lot per batch and a `stocked` flag read by the expander. The seam is left clean: `expand` takes a
`stocked(id) -> bool` predicate that is constant `false` today; when a ПФ is stocked it becomes a leaf of the
expansion and the ledger reserves it as itself.

## d) Cost, nutrition, weight — derived on read, exact inside, rounded once

**Cost of one base unit of a ПФ** = Σ over its raw leaves of (leaf quantity per ПФ unit × leaf price), exact in
rationals, rounded ONCE to `costMicroPerUnit` (i64, MICRO minor units per base unit, the cost book's own scale,
`stock/cost.rs` `MICRO`). Shown per kg / per l (× 1000) or per piece, and per line of the card. **Nothing is stored**:
a raw price change re-costs every ПФ and dish on the next read.

Two price sources, as today: the catalogue's list price `costPerBasis` for the owner's recipe view and the dish's
derived `cost` (`recipe.rs`), and the ledger's weighted average for the stamp at placement (`stock/cost.rs`
`CostBook::dish_cost`, which now multiplies by `uq` instead of `qty`, so an integer line stamps exactly what it did
and a leaf line stamps its fraction; `command/place/cost.rs` is unchanged).

**How `recipe.rs` learns a ПФ's numbers without a second derivation path.** `set_bom` / `lines_of_stored` receive a
`supply(id) -> Option<JSON>` closure; for a record whose `kind` is `prep`, `+workers/api/src/recipe/prep.rs` derives
the ПФ's `costMicroPerUnit`, `kcalPer100`/`proteinPer100`/`fatPer100`/`carbsPer100` (Σ raw nutrition of the batch ÷
yield × 100, on the edible part as `nutrition_scale` already does) and hands `line_with` a HYDRATED record with those
keys, `nutritionBasis: "cooked"` (a yield IS the cooked weight) and no `cleanPm`/`cookPm` (a ПФ line's out is its
qty, unless the dish line typed net/out). `line_with` prefers `costMicroPerUnit` when present (integer math), so a
ПФ line's cost is exact and a raw line's is what it was. The same hydration feeds `GET /api/owner/preps` and the
`/api/owner/stock` row (hand-back §i), so the console draws the same numbers the hub stores.

**"No re-save" for the dish's stored numbers.** A dish record stores `cost`, `nutrition`, `weightG` at `set_bom` time
(the public menu prints two of them). Two things make them follow a change without the owner re-saving the dish:
(1) `recipe::hydrate` — the owner's read — now refreshes `cost` and, where marked `…Derived: true`, `nutrition`
and `weightG` from today's lines; (2) a supply write (`POST /api/owner/supplies`, `POST /api/owner/preps`) re-runs
`set_bom` on every dish whose tree reaches the item, in the SAME catalogue write. (2) also closes an existing edit
bug: a raw's kcal or list price edited today left every dish's stored nutrition and cost stale until the dish was
re-saved (item 7 below).

## e) Deleting and editing an item other cards use

- `GET /api/owner/supplies/:id/uses` (+`services/operations/preps.rs`) answers `{ preps: [{id,name}], dishes:
  [{id,name}] }` — **transitive**: every ПФ whose tree reaches the item and every dish whose tree reaches it (so
  deleting salt names Mitsukan, Rice seasoned AND Philadelphia).
- `POST /api/owner/supplies/delete` (W-NOM's hard delete, `0367317f`) keeps its contract — ids leave the list, every
  recipe, the shelf's fold — with one new step in front: when any id is used by a ПФ card or a dish and the body
  does not say `"confirmUses": true`, it answers **409 with the uses list** and writes nothing. The console shows the
  list and asks; with the confirmation the lines leave the dish recipes (re-derived, as W-NOM does) AND the ПФ cards
  (re-checked; a card left with no lines keeps its yield and costs "unknown"). No dangling line, nothing removed from
  a card the owner has not seen named.
- Retire (`/retire`, `active:false`) stays reversible and stays out of the list; a retired item still expands (the
  kitchen still uses it — W-NOM's "honest outcome").
- Renaming or editing a raw or a ПФ re-derives everything (§d (2)); a unit change is refused as today (audit D35).

## f) Storage, byte cost, every reader

- A card costs the catalogue image (a cell per byte, `catalog/bom.rs` header) `"card":{"lines":[…],"yield":N}` =
  29 bytes + the yield's digits + ~26 per line (`{"item":"vinegar","qty":800}` is 28, `{"item":"salt","qty":50}`
  24); Mitsukan is 112 bytes MEASURED (`prep/tests.rs` `the_card_round_trips_and_a_raw_record_has_none`), so a
  venue with 30 ПФ of 6 lines ≈ 6 KB ≈ 0.6 ‰ of the 10 MiB ceiling (`catalog.rs` `DEFAULT_CATALOG_BYTES`). Absent
  keys stay absent (the `record()` rule).
- The columnar `bom` block: schema UNCHANGED (`block/schema.rs` `BOM`); a dish line naming a ПФ is interned like any
  supply id. `bom_of` (JSON) and `Catalogue::bom_of` (block) still give equal lines (`bom_of_block_equals_json`).
- Readers of a recipe, each named and its behaviour with a ПФ line: `stock::bom_of` (reads it as a line; plus the new
  `uq` form), `stock::bom_of_product`, `block::encode::project` / `view::Catalogue::bom_of` (interns the id),
  `stock::reservations_for` (old door: reserves the ПФ id; kept for callers not yet on `draws_for`),
  `+stock::draws_for` / `+append_draws` (the exact door), `command/place/cost.rs stamp_lines` (via `dish_cost`, now
  `uq`-exact), `ebills/import.rs` (served lines; hand-back to `for_ledger`), `services/orders/room/handlers.rs
  recipes()` (a filter on non-empty `bom`; unchanged), `recipe::lines_of_stored` / `hydrate` / `apply::set_bom`
  (hydrated ПФ record), `services/analytics/kitchen.rs dishes_of` (sees the ПФ as one line — hand-back names the
  expander), `supplies/delete.rs remove_supplies` (now also the cards), `ingredients_reset.rs` (ПФ are supplies:
  wiped with them), `catalog/bom.rs` (writer; unchanged).
- An old image (no ПФ, no `card`, no `uq`) reads byte-identically: no reader's output changes for a record that has
  none of the new keys; the block fixtures and the checkpoint codec's old-body test are the two byte-level proofs.

## g) The importer

`import/recipes` flattens a prepack card at import to raw leaves (`flatten.rs`), exactly. **Kept.** Reasons: (1) its
inputs are Poster/iiko exports, migrations of a flat file, not the operator's kitchen editing cards; (2) turning them
into real ПФ items needs the recipe import to WRITE supplies, which lives in `services/catalogue/import/bulk.rs` —
W-CRUD's file this lane may not touch — and a half-done version (draft names a ПФ id no supply has) would make
`set_bom` refuse every imported dish, a degradation; (3) flattening's math is the same `Rat` the tree uses, and after
this change `flatten.rs` runs on the shared `prep::tree` walker, so there is ONE recursion. The seam for the
follow-up: `RecipeDraft` gains `preps: Vec<DraftPrep>` (card + yield) written by the bulk import through
`supplies::record` before the dishes; the lines then name the ПФ id and are not flattened.

## h) UI (operator: maximally simple, on a phone, four languages)

One directory: the Ingredients & stock screen (`public/admin/stock.js`) gets the kind chip **Semi-finished** (`prep`,
icon `chef-hat`) beside the raw kinds, and a **Dishes → Menu** chip that opens the menu tab (dishes are edited there;
words Сырьё / ПФ / Блюдо in sq/en/uk/ru: `kind_raw`, `kind_prep`, `kind_dish` in `+public/admin/prep-i18n.js`).
A ПФ row shows name · yield · K · cost per kg (or l, piece) · how many dishes use it; no shelf levels and no
delivery/count buttons (it is not stocked). Tapping it opens the ПФ card (`+public/admin/prep.js` +
`+prep-view.js` + `+prep-logic.js`, pure logic tested in `+prep-logic.test.mjs`, `+prep-view.test.mjs`):

- **the card**: lines with gross qty, unit and cost per line; yield; K (per mille, as %); cost per kg/l and per unit;
  **where used** — ПФ and dishes, each dish with a "one sale takes" button;
- **the editor** (`Add semi-finished` on the screen, `Edit` on the card): name, id, unit chips, group, a searchable
  picker over raw items AND other ПФ, a quantity per line in the item's unit, the yield, and LIVE K, total batch cost
  and cost per kg as the fields change (the same arithmetic as the hub: `+prep-logic.js` mirrors
  `prep.rs` with BigInt rationals; the hub's answer wins after save);
- **"what one sale takes off the shelf"** for a dish: `GET /api/owner/products/:id/takes` drawn as a list of raw
  leaves with three-decimal quantities and the cost of the portion; reachable from the ПФ card's where-used list, and
  (hand-back) one button in the dish sheet of `menu.js`;
- the dish recipe editor (`menu.js`, W-CRUD) already lists every supply of `/api/owner/stock` with a kind filter, so a
  ПФ is pickable the day it is a supply; its `costPerBasis`/`kcalPer100` in that answer come from the hand-back
  hydration in `services/operations/stock.rs` (until then its preview cost reads "unknown" and the hub's `set_bom`
  still stores the exact one).

## i) What was measured (2026-09-29, this tree)

`cd crates/dowiz-hub && cargo test --lib --offline` → `test result: ok. 552 passed; 0 failed; 5 ignored`;
`cd workers/api && cargo test --lib --offline` → `test result: ok. 1324 passed; 0 failed; 6 ignored`;
`node --test` over the six console suites of this screen → `# pass 30 / # fail 0`; the example's write-off
(`prep/tests.rs one_philadelphia_expands_to_the_operators_grams`), 1 000 sales → 774 g of salt booked, carry bounded
(`stock/carry/tests.rs a_thousand_sales_book_the_exact_total_not_a_thousand_grams`), the cycle `b → a → b` refused
with its path, the depth cap above and below, where-used through two cards, the byte-identical block fixtures and
the old checkpoint bodies (`fixtures_are_the_encoder_output`, `the_codec_round_trips_hostile_names_and_every_list`).

## j) Hand-backs (files this lane does not own), summarised — exact text in the verdict

`workers/api/src/lib.rs` (4 route lines), `storefront.rs:888` / `services/orders/aggregator.rs:59` /
`ebills/import.rs:145` (`for_ledger`), `command/place.rs:137-141` (`draws_for` + `append_draws`),
`services/operations/stock.rs` (hydrate a `prep` row), `services/analytics/kitchen.rs` (expand through ПФ),
`public/admin/menu.js` (the "one sale takes" button).

---

## k) Second pass (lane W-PF2, tree `/root/lanes/w-pf2` at `620e2464`)

Rows R1–R5 of the lane card. Paths that exist unless written `+new`.

### k.1 The importer creates real ПФ (R1) — supersedes §g

§g's seam is built. `import::recipes::Opts` gains `preps: bool`; the Worker's bulk import passes `true`, so a
prepack card of the RECIPES file becomes a `+import/recipes/preps.rs` `DraftPrep` (id = the card's slug, name,
unit = its batch unit, lines = `(item, gross)`, yield = the batch) in `RecipeDraft::preps`, CHILDREN FIRST (a fixpoint
over "every ПФ it names is already made"), and a dish whose card uses it gets a `bom` line NAMING the ПФ. The Worker
(`+services/catalogue/import/bulk/preps.rs`) writes each through the console's own card writer
(`operations::preps::{check, save}`: unknown item, cycle, depth, bounds, re-derivation of every dish it reaches)
BEFORE `set_bom` writes the dishes, in the same object turn. The dry run judges the dish rows against a copy of the
catalogue already holding the ПФ (no false "unknown supply") and lists them (`preps: [{id, name, unit, yield, lines,
new, k}]`); the console draws them (`public/admin/pf2-view.js prepsBlock`).

- **Not guessed.** A prepack that cannot be a real card — a line or batch that is not a whole number of its unit,
  more than 40 lines, a refused row, a cycle, more than 6 levels — is not created; a warning names it, and each dish
  using it is FLATTENED through it as before (or refused by the same rule). A real ПФ removes one refusal: 7 g of a
  sauce whose leaves would be 5.6 g of mayo was refused flattened and is now a line of 7 g (the carry books it).
- **Idempotent.** A second import finds the ПФ it made (`bulk/preps.rs supplies_of` marks catalogue ПФ `kind: prep`),
  says "update", and writes the same records (test `the_same_file_twice_changes_nothing`). A RAW supply of that name
  is still a clash ("is both a supply and a semi-finished product").
- **A flat file reads byte for byte as before**: `as_json` of the old reader was captured from the unmodified tree and
  pinned (`import/recipes/preps/tests.rs FLAT_BEFORE`); `preps` is written only when non-empty; the catalogue after
  Apply equals the `preps: false` reading's (`a_flat_file_writes_what_it_wrote_before_this_row`).
- **Defect closed on the way.** A supplies import with "retire missing" retired every ПФ (a ПФ is in no ingredients
  file). The existing-supplies list handed to the reader now holds raw items only
  (`a_supplies_file_never_retires_a_semi_finished_product`).

### k.2 ПФ kept ready: the production act (R2) — supersedes §c's "OUT OF SCOPE"

**Two new records** (`stock.rs`, every reader re-derived in the same change: ledger, `decide`, signer, encode/decode,
cost book, carry, journal value, lots, stock view, waste, kitchen analytics):

    {"k":"cooked","item":<input>,"qty":Q,"into":<ПФ>,"act":<id>,"by":<signer>[,"uq":…]}
    {"k":"made","item":<ПФ>,"qty":OUT,"planned":P,"gross":G,"act":<id>,"by":<signer>[,"value":V,"lot","expiry"]}

`POST /api/owner/stock/cooked {item, qty, out?, lot?, expiry?}` (the existing `/api/owner/stock/:kind` route, same
SHELF guard, no new route) — `+services/operations/stock/cook.rs` → `+dowiz_hub::stock::act` `StockLog::cook`:

- `qty` = what the CARD makes from what went in (`planned`); the inputs are the card × `planned / yield`, expanded
  exactly by `+prep/stocked.rs plan` (a READY ПФ inside the card is taken first), rounded once per leaf, whole units
  booked from the carry (`Cooked` joins the carry fold like a consumed draw; 100 batches of 10 g mitsukan take 50 g of
  salt, not 100 or 0). Water (untracked) never leaves.
- `out` = what came off the stove, WEIGHED (default `planned`). The ПФ's `on_hand += out` and it is COUNTED from here
  (a measurement, like a delivery). `gross` = the card's grams at `planned` (g/ml 1:1, a piece its `weightPerUnit`;
  0 when unknown). **Loss on cooking = `gross − out`**, shown in the answer (`lossG`, measured `yieldPm` vs the card's
  `cardPm`), in the stock view's movement row and in the kitchen analytics' `yields` (stage `batch`).
- **Cost = WAC of what was drawn.** `value` = Σ leaf `uq` × the leaf's weighted average at that moment, rounded once,
  stamped on the `made` record; the cost book adds it to the ПФ's pool (`qty += out`), so the ПФ's average is the raw
  cost actually used over what came out. An unpriced input: no `value`, the batch joins at the ПФ's average (unknown
  until something priced reaches it) — a partial sum is never a cost.
- **Inputs never refuse** (`Cooked` may drive a level negative, "needs a count"): what was cooked is recorded, as
  `Consumed`/`Served` are. `Made` needs `planned > 0`, `gross ≥ 0`, `out > 0`. All records of an act are ONE commit.

**A sale takes the ready ПФ first; the rest is EXPANDED, not refused.** Decided: a batch running out mid-service does
not stop the kitchen — the cook makes the next from raw, which is exactly what the expansion books; refusing would 86
a dish whose raw items are on the shelf, against the standing rule that only a MEASURED raw shortage refuses. How:
`prep::for_ledger` now embeds the dish's tree (`"tree": {lines, cards, untracked}`) beside the all-raw `bom`; the
ledger's own door (`stock::draws_for` attaches the basket, `+stock/basket.rs`; `append_draws` / `append_served_draws`
re-plan in the venue's object where the shelf is) walks the basket's tree top-down in topological order: each ПФ's
WHOLE need over the basket is split into "from the shelf" (as far as `+basket::ready_micro` allows: whole units
available, the carry included, so a reservation never exceeds what is available and is never refused) and "from its
card". With nothing ready, the answer is `prep::expand`'s leaf for leaf and the records are identical
(`nothing_cooked_ahead_books_what_the_raw_expansion_booked`). **No call site changed**: `command/place.rs` and
`ebills/import.rs` still call `draws_for` + `append_draws`. The cost STAMP at placement still prices the all-raw
`bom` (the recipe's cost at today's averages), not the ready batch's own average — OPEN, one line in
`command/place/cost.rs` if the operator wants the batch's.

**Stock view, stock-health, analytics.** The `/owner/stock` row of a ПФ already carried its level; the console now
shows it (`pf_onShelf`, "a sale takes the ready batch first") and a "Cook a batch" button on the ПФ card
(`public/admin/prep.js openCook`). Stock-health's stranded holds list ПФ like any item (names from the same rows).
Kitchen analytics: `cooked` (raw into batches; also counted in `drawn`, it is food used) and `made` per item, and a
`batch` row in `yields` per act.

### k.3 Kitchen analytics through ПФ (R3)

`services/analytics/kitchen.rs dishes_of` gives each dish its raw `leaves` (`prep::expand`, millionths per portion)
when a line names a ПФ; `sales::fold` counts ingredient use from them, summed in millionths over the window and
rounded ONCE (1 000 Philadelphias: 774 g of salt, 61 905 g of dry rice; the ПФ is not an ingredient row; water is
not one); `portion_cost` prices the leaves exactly (average, else list price). A recipe of raw lines is counted as
before (`kitchen/prep_tests.rs`, each with its raw twin).

### k.4 Lessons and probe (R4, R5)

Lessons `docs/learn/lessons/owner/O22a.yaml` (the directory, the editor, "one sale takes") and `O22b.yaml` (the
production act), sq/en/uk/ru, anchors `pf.add, pf.name, pf.addLine, pf.yield, pf.save, pf.takes, pf.cook,
pf.cookQty, pf.cookOut, pf.cookSave`; films are the operator's to render. `e2e/kit-regression/_probe_pf.mjs` steps
5b (act + a sale from the ready batch) and 5c (import creates a ПФ, idempotent), qa-durres only.
