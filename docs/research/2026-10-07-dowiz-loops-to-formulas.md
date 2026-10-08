# R-LOOPS: loops -> formulas/algorithms across dowiz (2026-10-08)

Status: DONE for release numbers (2026-10-08 11:45Z); debug twins held by the compute hold (§6).
Lane rules: no edits to /root/dowiz, no git, benches via slot.sh, cargo -j2, taskset -c 4, median of >=7.
Excluded: bebop-store/src/kv.rs (Kv::decode, blob_byte, snapshot_root_u64 — W-KVDEC), bebop-lang/ (L1).

## 0. Journal of this lane (grows as work proceeds)
- 08:48Z box up 2 min, MemAvailable 2.6 GB, load 0.1. Started inventory.

## 1. Inventory (first pass, own reading; scouts' lines merged below as they land)

Legend: A closed-form, B better algorithm, C cache/memo (decode once), D incremental, E leave.
n = production size on dubin (165 dishes, ~600 journal records, tens of orders/day).

| # | file:line | fn | what the loop does | n | how often | class | note |
|---|---|---|---|---|---|---|---|
| I1 | workers/api/src/lib.rs:302-560 `router()` | route table | 256 `insert`s into `HashMap<Method, matchit::Router>` (String pattern alloc + radix split per insert) | 256 routes | EVERY Worker request (`route()` calls `router(Router::with_data(..))`) | C | the table is constant; the per-request part is `Req{now_ms}` only. workers-rs 0.8.5 `Router` owns the data, so the table cannot be `static` as-is; a `static` matchit table of `fn` pointers + manual dispatch, or a `thread_local` built once per isolate, removes 256 inserts per request. BENCH B1 |
| I2 | workers/api/src/hubdo/journal.rs:95-150 `journal_append` + `write_image` | per catalogue write | `LogImage::load(journal bytes)` = `Store::from_bytes` (copy ~2-3 MB) + `chain_scan` crc of all ~600 records; then `edits::journal` (1 append), `to_bytes_trimmed` (copy), `changed_chunks` (memcmp over the image) | ~600 records, 2-3 MB | per catalogue write (owner price edit, import, compaction) | C+D | the object holds the bytes in `mem` but re-verifies the whole chain on every write; W-ZC's "crc once per generation" (`cat_checked`) has no twin for the journal. Keep the `LogImage` resident per generation; only the tail chunk changes on an append. BENCH B2 |
| I3 | journal.rs:100-106 | `journal_append` | `Catalog::load` TWICE (`before` from `mem`, `after` from the PUT bytes) + `state_of` ×2 (clones every entry) | 165 dishes ×2 | per catalogue write | C | `after` of write N is `before` of write N+1 at the same generation: keep the last `State` keyed by generation. Decode itself is W-KVDEC's (EXCLUDED); the `state_of`/`diff` part is measured in B2 |
| I4 | workers/api/src/hubstore.rs:623-642 `with_catalog` (30 Worker call sites) | owner edits | pulls the whole catalogue image (~0.5 MB) across the hop into the WORKER, `Catalog::load`, edit, `to_bytes`, PUT 0.5 MB back | 0.5 MB | per owner edit (price, category, supply, media, promo, zones...) | C (architectural) | reads moved into the object (W-ZC); writes did not. Under the 10 ms Worker cap this is decode + encode + two 0.5 MB copies per edit. Not benchable here without kv.rs; CITED from W-AX0 (148 ms debug per edit) |
| I5 | workers/api/src/hubdo.rs:634, :722 `place`/`advance` | `tell_orders` input | clones EVERY order's JSON `(id, json)` into a Vec, then `produce::late` (notify/route/produce.rs:53) `serde_json::from_str` on every order | day's orders (tens-hundreds × ~2 KB) | per order placed / advanced | B/E | gated by `produce::wants(&groups,"order.late")` (venues with a Telegram group only); ESTIMATE 100 × 20 µs = 2 ms inside a DO (30 s cap). Leave unless a venue has many open orders |
| I6 | workers/api/src/catalog_edit.rs:275-290 `categories_view` | owner categories | parses all 165 products, then for each category scans all products (`filter(categoryId==)`) | 165 × ~12 | per `GET /api/owner/categories` (object side) | B/E | O(c×p) with p already parsed; a one-pass count map is trivial but the parse dominates. ESTIMATE < 1 ms release. Leave |
| I7 | workers/api/src/catalog_edit.rs:97-104 `create_product` | last sortOrder | parses all 165 products to find max sortOrder in one category | 165 | per dish created (Worker side, after I4) | E | cold (a dish is born rarely) |
| I8 | workers/api/src/fold/menu.rs:112-176 `Memo::build` | menu projection | `serde_json::from_str` ×165 + `blocks_of` + per-locale `render` | 165 | ONCE per catalogue generation (memoised by `Gens`) | E (already C) | RECORDED 6.33 ms per parse in R-SPIKE; memo makes it per write not per request. Nothing to add |
| I9 | crates/dowiz-hub/src/catalog/edits.rs:93-107 `diff` | journal | two passes over 165-entry BTreeMaps with string compares; `digest` (SHA-256) only on changed keys | 165 | per catalogue write | E | O(n) string compares of ~3 KB each = ~0.5 MB memcmp; measured inside B2 |
| I10 | crates/dowiz-hub/src/catalog/edits.rs:118 `last_gen` | journal | `walk_until(|_| true)` reads one record from the tip | 1 | per write | E | already O(1) |
| I11 | crates/dowiz-hub/src/logimage.rs `entries()`/`quarantined()` | journal reads | `walk_marked` decodes every record (String::from_utf8_lossy ×3 per record) | 600 | per `catalog_history` read, per compaction | E/B | console read is `recent(n)` which still calls `entries()` on all 600 then `take(n)`; a `walk_until` with a counter would read n. ESTIMATE 600 × 3 allocs; low frequency |
| I12 | workers/api/src/hubdo.rs:235-237 `image()` | every image read on a miss of the in-place path | `hit.clone()` of the whole image (catalogue 0.5 MB, journal 2-3 MB) | 0.5-3 MB | per `catalogue()`/`image()` call not yet on `with_catalog` | C | R-SPIKE §7.6 row; `journal_append` calls `self.image(img)` and so clones 2-3 MB per write on top of I2. Counted in B2 as a memcpy |

### 1b. Worker services inventory (scout 1, read-only; paths under workers/api/src/)

Orders folded and re-parsed on request paths:

| file:line | fn | loop | n | how often | class | fix |
|---|---|---|---|---|---|---|
| fold/projection.rs:106-108 | `Orders::view` | clones every `OrderView` (with its `order_json` String) on every `orders_view()` | hot log 300-1000 orders | every DO read touching orders | C | borrow / `get(order_id)` on `slots` |
| fold/projection.rs:139 + hubstore.rs:1253 | `render` / `orders_state` | the fold holds a serde `Value`, stringifies it, every reader parses it back | N | every request | C | keep the hot fields (location_id, status, created_at_ms, sitting_id, courier_id) on the row |
| fold/projection.rs:159-160 | `folded_exactly` | re-hashes the whole older history on every command write | ~600 events | per command write | E/D | stored prefix digest |
| command/floor.rs:153-163 | `sittings` | distinct sitting ids by `ids.iter().any` (O(S²)), then `sitting::rounds` per sitting, each re-parsing EVERY order | S×N (~10^5 parses) | every staff floor poll (services/orders/room/floor.rs:96) | B | group by sitting_id in one pass. BENCH B3 |
| command/sitting.rs:44-52 | `rounds` | parses all N orders to find one sitting's rounds | N | per pay/transfer/clear/guest round, and per sitting above | B/C | index by sitting_id |
| services/orders/room/table_link.rs:106-130 | `live_sitting` | same O(S×N) (`seen.iter().any` + `rounds` per sitting) | S×N | every guest QR placement at a table (placer.rs:161) | B | one grouping pass |
| services/orders/room/placer.rs:153-159 | table placement | parses every order to filter by location, then `live_sitting` re-parses | N | per QR placement | C | parse once |
| services/orders/room/floor.rs:68-77 | `listed` | full list crosses the hop, every order parsed to filter location, parsed again by `floor()` | N | per floor poll | C | filter in the DO |
| services/orders/kitchen_ack/board.rs:111-124 | `board` | parses all orders to keep open tickets | N | kitchen board load | C/D | open-orders index |
| courier.rs:101-147 | courier queue | full list crosses, each parsed; `assigned.iter().find` per candidate | N | each courier polls every 12 s | C/D | READY/IN_DELIVERY index in the DO |
| courier.rs:808-830 | courier wallet | parses every order to sum one courier's day/week/month | N | per wallet view | C | same |
| eta.rs:135-150 | eta quote | parses every order for CONFIRMED/PREPARING | N | public storefront checkout | C/D | active-queue index |
| live_eta.rs:381-387 | `attach_one` | full list parsed for IN_DELIVERY couriers | N | customer tracking poll | C/D | busy-courier set in the DO |
| owner.rs:365-370 | owner orders (full path) | parses every order to filter location+status | N | console poll without `since` | C | filter in the DO |
| owner.rs:660-674 | owner dashboard | parses every order BEFORE the cheap `e.seq < day_start` test | N | dashboard poll | A/B | test seq first; list is newest-first so stop at the first older row. BENCH B4 |
| hubdo/room.rs:117,161,248; hubdo/refund.rs:17,114; hubdo/kitchen_ack.rs:13; hubdo/print.rs:88 | room/refund/ack/print commands | whole projection cloned, then a linear `find` for one order_id | N | per command | B/C | O(1) lookup in `projection.slots` |
| hubdo/room/till.rs:58-71 | cash | parses every order on each till command | N | per till command | C | projection's parsed rows |
| services/orders/mine.rs:35-40 | `of_venue` | parses all orders on each analytics/kitchen/forecast/week_top read | N | per request | C | same |
| services/ordering/promotions.rs:31-39 | owner promotions list | `promo_uses` -> `orders_state(hub)`: a full refold of the log PER PROMO on a hub decoded Worker-side via `load_both` | P × ~600 events | per promotions page | B/C | fold once, count uses by code. BENCH B5 |
| services/ordering/promo_fields.rs:94-108 | `promo_uses_in` | parses every order to count one code | N | each placement with a promo, and preview | C/D | uses-by-code counter in the projection |
| notify/route/produce.rs:57-70 | `late` | parses every order after every order turn; `marked.iter().any` inside | N | every order turn when a group listens to `order.late` | C/D | open-orders index |
| hubstore.rs:1025-1026 | `rotate` | `orders_state` stringifies each fold, then each is parsed again | N | nightly | C/E | cold |

The whole catalogue decoded to read one field, or decoded twice (every row is class C: `with_catalog` in place, crc memoised, exists since W-ZC):

| file:line | fn | what | how often |
|---|---|---|---|
| hubdo/fiscal.rs:47-53 | `enqueue_fiscal_with` | full `Catalog::load` (~0.5 MB) for the currency; `Settings::load` too | EVERY placement at a fiscalising venue |
| hubdo/routed.rs:127-131 | `tell_low` | full `Catalog::load` to name the supplies that crossed | every placement/turn that moved stock when `stock.low` is routed |
| hubdo/routed.rs:112 + 121 | `low_watch` / `tell_low` | `stock.ledger()` replays the whole stock journal BEFORE the turn and AGAIN after | per stock-holding order turn (class D: after = before + this turn's lines) |
| hubdo/timer.rs:47-70, 85-92 | `timer_next` | after EVERY write: parses all outbox entries, loads the fiscal Table, loads it again in `offline_overdue_next` (room/offline.rs:177-183), full catalogue decode in `ebills_next` | every image write outside an alarm |
| hubdo/timer.rs:122-131 | `own_venue` | `Catalog::load` for `location.id` | when not cached in storage |
| hubdo/ebills.rs:73-85, 101 | `ebills_tick` / `ebills_import` | full `Catalog::load` each; the tick needs only location | per alarm while the till link polls (1-15 min) |
| hubdo/fiscal/send.rs:95-97, 215 | `plan` / `receipt` | `Catalog::load` for location or currency | per fiscal firing / receipt |
| hubdo/room/till.rs:50-52 | `venue_currency` | `Catalog::load` for the currency | per till command |
| hubdo/room/offline.rs:203-205 | `offline_overdue_turn` | `Catalog::load` for location | per alarm |
| hubdo/exceptions.rs:51, 93 | exceptions read / alert | `catalogue()` + `Catalog::load` for currency and zone | per read; per pay-out/close alert |
| hubdo/stock_turn.rs:38 + hubdo/forecast.rs:89 | `stock_move` -> `expiring_surplus` -> `use_table` | `catalogue()` decoded TWICE in one request | per stock movement |
| hubdo/reads.rs:143-145 | `fold_week_top` | full `catalogue()` only for the zone | storefront "most ordered" |
| hubdo/reads.rs:69-70, 102, 128, 156 | `fold_analytics`/`kitchen`/`stock`/`haccp` | full `Catalog::load` per request | per request |
| hubdo/facts.rs:61-62, 69-70 | `fold_facts` assist/graph | `Hub::load` of the whole log + `Catalog::load`; `owner_facts` then refolds `orders_state(hub)` instead of the memoised projection | per assistant question |

Analytics, kitchen, forecast:

| file:line | fn | loop | n | how often | class | fix |
|---|---|---|---|---|---|---|
| services/analytics/handler.rs:143 | `answer_with` | `cube::fold_orders` over the whole hot log (after `of_venue` parsed it all) | N | every owner analytics read | D | hot DayCubes per day kept by the projection |
| services/analytics/forecast.rs:130 | `history_with` | same full hot refold | N | per prep forecast; per stock movement on the expiring day | D | same |
| hubdo/cube.rs:50-55 | `rows_of` | clones every DayCube in range, product maps included | ≤ ~760 days × ~50 dishes | per analytics/kitchen/forecast read | C | borrow |
| services/analytics/history.rs:172, 179-182 | `report` | `cold.clone()` again; `rows.get(&d).cloned()` per day | same | per read | C | overlay/borrow |
| services/analytics/history.rs:90-96, 176-177, 194 | `total` | steps day by day with a BTreeMap get, merges product maps, for cur, prev and every bucket | 366×3 merges | per read | B/A | `rows.range()`; cur = sum of bucket totals |
| services/analytics/handler.rs:150-155 + week_top.rs:113-119 | name closure | `cat.product(id)` + JSON parse per name shown | ~200 parses | per analytics read | C | id->name map once |
| services/analytics/forecast/plan.rs:125-127 | `dish_name` | product JSON parsed per forecast dish | ~165 | per forecast | C | same |
| services/analytics/kitchen.rs:97-110, 116-123 | `dishes_of`/`leaves_of` | parses all 165 products; `bom_of` re-parses; `prep::expand` per dish | 165 | every kitchen read; turn.rs:90 `linked_of` on every count | C | memo per catalogue generation |
| services/analytics/kitchen/report.rs:48 | `avt_of` linked | scans every dish's lines/leaves for each counted supply | ~50 × 165 × ~5 | per kitchen read | B | HashSet once |
| services/analytics/kitchen/avt.rs:157-163 | `report` | `counted.contains` inside the journal loop | ~600 × distinct | per kitchen read | B | BTreeSet |
| services/analytics/kitchen/sales.rs:70 + fold.rs:102 | `Window::bucket` | `rposition` linear over day starts per order/row | ≤62 × N | per kitchen/analytics read | B | `partition_point`. BENCH B6 |
| services/analytics/forecast/expand.rs:122-136 | `raw_by_day` | `needs()` for all dishes, then per day ahead a full cast + `needs()` re-parsing BOMs | 165 × (warn days+1) | per stock movement on the expiring day; per prep read | C/A | per-dish unit leaves once; a day is a linear combination |
| services/analytics/fold.rs:181 | `fold` | `products.iter_mut().find` per order line | lines × ≤165 | daily digest | B | HashMap |

Customers, stock, smaller:

| file:line | fn | loop | n | how often | class |
|---|---|---|---|---|---|
| services/customers/roll.rs:88-98 | `roll` | `rows.iter_mut().find` per order (O(N×C)); `key_of` = one HMAC per order | N≈1000 × C≈100s | owner customers page | B/C |
| services/customers/handlers.rs:110-118 -> consent_log.rs:106-122 | `circle_state` per row | parses every consent act per customer row | C × acts | per customers page | B/C |
| services/customers/alias.rs:68-70 | `Aliases::members` | scans all aliases, resolves each, per row | C×A | per customers page | B |
| services/customers/handlers.rs:166-174 | `reveal` | HMAC + alias resolve per order | N | per reveal | C |
| services/operations/stock/turn.rs:147-149 | `run` | `log.ledger()` and `log.journal()` each replay the full stock journal | ~600 ×2 | per stock movement | D |
| services/operations/stock/turn.rs:61-75 | `from_catalogue` | parses all supplies; on counts `linked_of` over 165 products | 165 | per movement | C |
| services/operations/stock/tell.rs:160; view.rs:161-170; moves.rs:132 | lots / sessions / count plan | `position`/`find`/`any` in loops | L², 600×S, lines² | daily / stock screen / per count | B/E |
| fold/menu_venue.rs:103 | `at` | hours re-serialised and re-parsed per request (`from_json(&h.to_string())`) + `base.clone()` | small | EVERY menu request | C |
| fold/menu_venue.rs:161-164 | `render` | per category filters all products | 20×165 | memoised per locale | E |
| owner.rs:1211-1214 | translations save | `has()` linear scan of id arrays per entry | entries × 165 | owner bulk i18n save | B |
| hubdo/push_turn.rs:82-91 | `push_owed` | parses every subscription of 3 kinds; `stale_customers` re-parses customers | tens-hundreds | every order turn | C |
| ebills/status.rs:27,43 + ebills/state.rs:263-271 | `view`/`suggest` | `norm()` of every product name per unmatched till code | U × 165 | owner ebills page | B/C |
| hubdo/cube.rs:142-150, 185-195 | `cube_catch_up`/`cube_trace` | each archive `Hub::load`-ed twice | archives | nightly | C/E |
| cron/timer.rs:93-96 | `ebills_next` | steps minute by minute asking `open_at` | ≤30 | per write and alarm while closed | A/E |
| notify/route.rs:203-205 | `next_at` | weekday by stepping days | ≤6 | per alarm | A/E (trivial) |
| services/catalogue/import.rs:53-54, 178-190 | owner import | `any` per existing product; `slug` recomputed per pair | 165² | rare | E |

### 1c. Crates and JS inventory (scout 2, read-only)

tz.rs, hours.rs, bebop-store lib.rs/evlog.rs: already O(1) or closed-form (tz `start_of_local_day_ms` checks 4 fixed candidates; `offset_minutes` is ~50 integer ops; evlog `len`/`tip` are root reads). Nothing to change there.

| file:line | fn | loop | n | how often | class | fix |
|---|---|---|---|---|---|---|
| bebop-store/src/verify.rs:185-202 | `walk_marked` | pointer walk, then a full walk, then `check_obj` crc per record -- AFTER load already crc'd the chain | log | per `Hub::events`/`LogImage::entries`/`quarantined` | C/B | keep the bad-set from load (as `StockLog` does), one walk. BENCH B7 |
| dowiz-hub/src/read.rs:38-43 | `Hub::events` | `walk_marked` + decode all | hundreds-thousands | per cold fold, per command (`hubdo.rs:951 put_log` -> `Written::Log(hub.events())`) | C/D | `walk_until` the memo's tip: O(k) per command. BENCH B7 |
| dowiz-hub/src/read.rs:80-86; room/view.rs:54-77 | `Hub::order`, `latest_seq`, `current` | decode every event then `find` | n | per call / per room write | B | early exit (`walk_until`, `history_scan`) |
| dowiz-hub/src/logimage.rs:186-195, 216-223 | `entries`, `about(kind,subject,limit)` | `walk_marked` (double walk + re-crc) + decode all, THEN filter + `take(limit)`; limit=1 at wallet.rs:308/372, channels.rs:596 | ~600 | per request on taste_routes.rs:66, customers/handlers.rs:107, campaigns/send.rs:145, chat/store.rs:91, exceptions.rs:110, channels.rs:503 | B | lazy walk, early exit at `limit`, subject filter before decode. BENCH B7 |
| dowiz-hub/src/consent/personal.rs:34-50 | `newest` | `Act::parse` of every consent act for every key | entries | per taste request, per marketing send | B | filter `e.subject` before parsing |
| dowiz-hub/src/catalog/edits/history.rs:30-36 | `recent(n)` | `quarantined()` + `entries()`: two full walks, three crc passes, decode all, `take(n)` | journal | per owner console history view | B | `walk_until` n |
| dowiz-hub/src/catalog.rs:284-292 | `entries_with_prefix` | scans every kv entry for a prefix and clones each value | ~300 entries | per `products()`/`supplies()`/`categories()`/`promos()` | B | keys are sorted: binary-search the range (`CatalogView::with_prefix` already does) |
| dowiz-hub/src/table.rs:128-157 | `Table::all`/`scan` | `kv.keys()` allocates every key, filters by prefix, then `kv.get` per match | whole table | per request (platform_store.rs:179, hubdo, fold/menu i18n) | B | prefix range |
| dowiz-hub/src/stock/ledger.rs:8-14, 24-26, 36-43 | `level`/`level_mut`/`is_counted` | linear `find`/`position` over `levels: Vec<(String, StockLevel)>` and `counted: Vec<String>` per event (M1 turned open/served into BTreeMaps; `levels` was left a Vec) | ~76 supplies × records | per stock write, per `ledger()`, twice per journal step | B | BTreeMap/BTreeSet. BENCH B8 |
| dowiz-hub/src/stock/cost.rs:111-122 | `CostBook::pool`/`avg_micro` | linear `position` over `pools` per priced event | ~76 | per journal step | B | same |
| dowiz-hub/src/stock/journal.rs:132-167 + meta.rs:111-128 | `Journal::step`, `meta_of` | per record ~25 substring scans (`minijson` field finds, each `format!`-ing its key) and ~25 String allocations | ~600 | per `journal()`/`journal_since`/`trace` | C | tokenise once |
| dowiz-hub/src/stock/lots.rs:68, 88-110, 126-129, 163 | `open_lot`/`take`/`total`/`step` | find, filter+sort, sum, retain over all open lots per step | L ≤ ~100 | per journal step | B | index lots by item |
| dowiz-hub/src/stock/journal.rs:173-179 | `journal()` | from genesis ignoring checkpoints, unpacking checkpoint blobs | ~600 | per request (operations/stock.rs:96, stock/turn.rs:149) | D | `journal_now`/`journal_since` |
| dowiz-hub/src/stock/notes.rs:49-51; storages/write.rs:23-39 | `notes(what)`, `storages()` | `raw()` walks the whole chain (checkpoint bodies included) + 2 field scans per record; several times per stock page | ~600 | per request (stock.rs:161, aliases.rs:91, suppliers.rs:133, digest.rs:75, bind.rs:211) | C/D | one walk per request; carry notes in the checkpoint |
| dowiz-hub/src/stock.rs:187-203 | `returned_lines` | `events()` from genesis, then linear find | ~600 | per refund-on-return | B | filter by order before decoding |
| dowiz-hub/src/stock/bom.rs:92-106, carry.rs:140-145, storages/bind.rs:114-119 | `reservations_for`/`draws_for`/`shares` | re-parse `bom_of(product_json)` per order line, ×3 | 3 × lines | per placement | C | parse once (`block::view::Catalogue::bom_of` exists) |
| dowiz-hub/src/graph/retrieval.rs:135-144 | `Graph::ppr` dangling loop | each dangling node adds to all n restart entries; restart is non-zero on ≤5 seeds | D × n × 12 iters | per owner-assistant question | A | iterate seeds only (bit-identical) or sum dangling mass once (changes rounding). BENCH B9 |
| dowiz-hub/src/graph/retrieval.rs:46-81 | `Graph::bm25` | re-tokenises every node per query; per term two passes over every doc | N × q | per assistant question | B/C | postings map once |
| dowiz-hub/src/graph/build.rs:156-172 | `of_with` | folds `hub.orders()` (whole log) + catalogue into a graph | history | per assistant question | C | memo per (log gen, catalog gen) |
| dowiz-hub/src/block/taste.rs:208-221, 225-238 | `top_k`, `top_k_json` | full sort then truncate; `top_k_json` parses all 165 product JSONs + allergens + sense per call | 165 | per for-you / taste request (SNN shadow compare) | B/C | `select_nth_unstable`; the block path is the cache |
| dowiz-hub/src/snn.rs:52-54, 92-105, 138-148; snn/infer.rs:63-95, 115-127 | `shipped`, `Menu::from_products`, `stalks`, `scores` | weights blob decoded (crc) per request; all 165 products parsed per request; `stalks` (165 × d≤8 × L≤4, `Vec<Vec>` per dish per layer) depends only on model+menu, recomputed per guest; `index_of` linear per guest dish | 165 | per taste request (customers/taste/snn.rs:43-48, 72-87) | C | build once per catalogue generation; flat `Vec<i64>`. BENCH B10 |
| dowiz-hub/src/forecast.rs:168-170 + forecast/plan.rs:53 | `first_sale` per dish | scans the days range per dish | 165 × ≤365 | per forecast | B | one pass |
| dowiz-hub/src/block/decode.rs:143-187 | `values`/`utf8` | validates every offset and string per `View` open | block bytes | per request opening a View | C | `Checked` per block generation |
| dowiz-hub/src/prep/uses.rs:15-45; prep.rs:220-229 | `uses_of`, `check_card` | linear finds over supplies/cards | 76 × lines | owner page | E |
| bebop-store/src/lib.rs:341-349 | `Store::from_bytes` | copies the whole image into `Vec<i64>` 8 bytes at a time | MBs | per image load | C | `View<'a>` exists; cache the Store per generation inside the DO |
| bebop-store/src/evlog.rs:57-82, 229-298 | `pack_payload`/`unpack_payload`, `read_at` | byte-at-a-time shift loops; 2-3 small Vec allocs per record | per record | every walk/append | E/C | `to_le_bytes` chunked copy; stack arrays |
| dowiz-canvas/src/* (scene.rs:129, text.rs:86, board/cards.rs:90-134) | hit / fit / grid | fixed arrays, binary search, tens of tickets | small | per tap / per frame | E | fine |
| dowiz-core tax.rs:173-240 | tax lines | linear find of the rate group per line, then sort | few | per order | E |
| dowiz-core money.rs:210-258, geo.rs:60-139, tracker.rs, retrieval/*, living_knowledge.rs | — | `ledger_sum` O(n²), polyline scans, bm25 scores every doc | — | COLD: no caller in workers/ or dowiz-hub | E |

JS (workers/api/public):

| file:line | fn | loop | n | how often | class | fix |
|---|---|---|---|---|---|---|
| admin/orders.js:93, 99 | history list / `historyAll` | `S.orders.filter(o => !liveOrders().includes(o))`: `liveOrders()` rebuilds the filtered array INSIDE the filter callback, then a linear `includes` | hundreds-thousands | per render/keystroke on the orders tab | B | Set of live ids once. BENCH B11 (node) |
| admin/orders.js:97 | search filter | rebuilds + normalises the hay string (NFD + regex) per order per keystroke | orders | per keystroke | C | cache per order |
| admin/app.js:316-322 | `loadVenue` translations | `S.products.find` inside langs × categories × products | langs × 165 × 165 | per venue load (after each dish toggle) | B | Map by id. BENCH B11 |
| admin/kitchen.js:105-108, 75-79 | ticker, `stopRows` | every 30 s full rerender incl. the 165-dish stop list (norm + localeCompare sort); per keystroke norm + sort | 165 | every 30 s / per keystroke | D/C | update ages in place; precomputed normalised names, `Intl.Collator` |
| admin/kitchen-logic.js:123-129, 155-158 | `ticketsFor`/`stationCounts` | clone+filter+sort once per station plus all (~6× per render) | tens | per render | B | one grouping pass |
| store/menu.js:222, 232-238, 349-366 | `applyFilters`, `patchTexts` | `querySelector('.sec-name')` + NFD per CARD; re-sorts and re-appends every card even when unchanged; `'az'` does 2 `findProduct` + `localeCompare` without a Collator per comparison; `$('.card[data-p=...]')` per product | 165 | per search keystroke (debounced) / per language switch | C/D | hoist per section; reorder only on sort change; one `$$('.card')` pass |
| room/canvas/loader.js:65-88 | `replay` | per draw op: destructuring, new `Uint8Array` + `TextDecoder` per text op, font string per op | ops | per frame on demand | C (minor) | read `W[]` directly |
| lib/particle-cloud.js:249-250, 277 | draw / frame | `getUniformLocation` twice per frame; `resize()` reads `clientWidth` (layout) every frame; seed buffer re-uploaded every frame | MAX particles | per frame while active | C/D | cache locations; dirty range; ResizeObserver |
| lib/dust.js:101,112 | draw | `filter` allocations + rgba template per item per frame | small | per frame | C (minor) | in-place |
| admin/stock-health.js:52; admin/menu.js:501; courier/app.js:919; store/track.js:165 | — | small `includes`/`some`/1-node intervals | small | — | E |

## 2. Measurements

Box: Android/proot aarch64, core 4 (Cortex-A78) via `taskset -c 4` inside `tools/slot.sh`; cpu4 ran at 2054-2208 MHz
during the runs (printed per run). rustc 1.93.1, `opt-level=3` release (dowiz-core at opt-level 1: it is linked, not timed).
Bench crate: `rloops/benches/rbench/` (path deps on /root/dowiz/crates/dowiz-hub and bebop-store, matchit =0.7.3 as
worker 0.8.5 pins it; nothing under /root/dowiz edited). Raw outputs: `rloops/results/rel-*.out`, `run-rel.out`,
`js_bench.out`, build logs `build-*.out`. Every number below is the MEDIAN OF 9 runs unless stated. Every replacement
asserted an identical result before timing (`EQUIV ok` lines in the raw output).

Build history (for the record): the release build of the bench crate was SIGKILLed twice by Android's phantom-process
killer (`build-core-try1`: `Killed`, `build-rel-try2.out:1099` `signal: 9`), refused once by slot.sh's process ceiling
(`build-rel-try1.out`: `28 procs still above the 26 ceiling after 300s`), and succeeded on the fourth attempt with `-j1`
and dowiz-core at opt-level 1 (`build-rel-try3.out:1093` `Finished release ... in 5m 57s`). DEBUG numbers are NOT
MEASURED: the coordinator put compute on hold ("HOLD COMPUTE ... until BOX FREE") right after the release run; the
debug build is the NEXT COMMAND in §6. Where a debug figure matters, W-AX0's CITED debug numbers are given.

### B1. Route table rebuilt per request (lib.rs:302 `router`) -- class C

MEASURED (`rel-router.out`): `B1 CURRENT build+match per request: median 200.9 us` / `B1 REPLACE match only per
request: median 0.189 us` / `B1 RATIO 1063x; build alone = 200.7 us = 2.01% of the 10 ms Free cap`. 257 routes
(`benches/routes.txt`, extracted from lib.rs), 14 probe paths (12 hits, 2 misses) agree on handler index and params.
That is 0.2 ms of CPU on EVERY Worker request before any handler runs -- 2 % of the Free cap, paid by the 1.4 % of
requests that already die at the cap. workers-rs 0.8.5's `Router` is consumed by `run`, so the fix is not a `static
Router`: it is a `static`/`OnceCell` matchit table of `fn` pointers (one per route) plus a 20-line dispatch, with the
existing `routes_table/tests.rs` conflict test kept.

### B2. The catalogue edit journal per write (hubdo/journal.rs `journal_append` + `write_image`) -- class C + D

Fixture: a 165-dish catalogue (the `measure.rs` product shape, 187 KB as stored) and a journal grown by 590 one-price
edits through the Worker's own recipe (baseline + edits, compacted once by `MAX_RECORDS`): 522 records, 688 KB, 8
chunks of 96 KiB. Also a 100-edit journal (278 records, 358 KB). MEASURED (`rel-journal.out`), phase medians in µs at
n=590: `p1 clone 87 | p2 before(decode+state_of) 605 | p3 after(decode+state_of) 574 | p4 LogImage::load 652 | p5
journal 141 | p6 to_bytes 95 | p7 changed_chunks 111 | TOTAL 2265`. The journal-only part (clone + load/crc + journal +
to_bytes + chunk diff) is 1086 µs; the two catalogue decodes (W-KVDEC's) are 1179 µs of the 2265.
REPLACEMENT A (the `LogImage` resident per generation, as `cat_checked` keeps the catalogue's crc): journal-only 318 µs
(`3.4x (1086 -> 318 us)`); REPLACEMENT B (A + the previous write's `after` State kept as this write's `before`, dropping
one decode): whole path `2.4x (2265 -> 925 us)`, i.e. `22.6% -> 9.3%` of 10 ms (object side, where the cap is 30 s, so
this is latency and DO CPU, not a kill). At n=100: 1482 -> 646 µs (2.3x). Journal bytes and changed-chunk lists are
byte-identical (`B2 EQUIV ok ... bytes 689544 B identical, changed chunks [0, 7] identical`).
CITED for the debug profile: W-AX0 measured one dubin price edit at 148 ms debug, journal 34.5 ms -- the same phases,
~30x slower; the ratio above is the release one. Note `p4 LogImage::load` is `Store::from_bytes` + `chain_scan` crc of
every record: the object re-verifies a 688 KB chain on every one-record append.

### B3. The staff floor poll: `command/floor.rs:sittings` + `sitting::rounds` -- class B

Fixture: N orders newest-first, ~1.3 KB kernel-envelope JSON each, S sittings. MEASURED (`rel-orders.out`):
`B3 n=100 S=20: CURRENT 32453 us | REPLACE 1454 us | 22.3x | share of 10 ms cap 325% -> 15%`;
`B3 n=300 S=60: CURRENT 278974 us | REPLACE 4570 us | 61.0x | share 2790% -> 46%`;
`B3 n=1000 S=150: CURRENT 2177489 us | REPLACE 13045 us | 166.9x`.
The current code parses every order once to collect sitting ids and then AGAIN per sitting inside `rounds`: S × N
serde parses at ~16 µs each. This runs in the WORKER on every `/api/staff/floor` poll (services/orders/room/floor.rs:96)
and the same shape on every guest QR placement (`table_link::live_sitting`). At 100 open orders in the hot log it is
32 ms -- three times the Free cap -- which makes it the most likely single source of the 1.4 % kills on venues that
use the room. The one-pass grouping gives the identical (sitting order, rounds order) result (`B3 EQUIV ok`).

### B4. Owner dashboard tiles: parse before the `seq < day_start` test (owner.rs:660-674) -- class A/B

MEASURED: `B4 n=300 today=50: CURRENT 4614 us | REPLACE 754 us | 6.1x | share 46.1% -> 7.5%`; `B4 n=100 today=0:
CURRENT 1427 us | REPLACE 0 us`; `B4 n=1000 today=750: 14599 -> 10840 us (1.3x)`. The list is newest-first, so
testing `seq` first and stopping at the first older row parses only today's orders. Identical tiles asserted.

### B6. `Window::bucket` rposition -> partition_point (analytics/kitchen/sales.rs:70) -- class B

MEASURED (`rel-bucket.out`): `days=7: 3.3 -> 4.6 ns/call (0.7x)`, `days=30: 9.5 -> 8.5 ns (1.1x)`, `days=62: 19.1 ->
9.7 ns (2.0x)`; per 1200 calls 23 µs -> 12 µs at 62 days. Noise against a request. NO-GO.

### B7. Log readers that re-walk and re-crc the chain (bebop-store verify.rs:185 `walk_marked`; dowiz-hub read.rs:38
`Hub::events`; logimage.rs:216 `about`) -- class C/B

MEASURED (`rel-logwalk.out`), 600 events / 463 KB: `Hub::load(crc) 1008 us | CURRENT events() 961 us | walk only 530
us | REPLACE walk+decode 647 us | 1.5x on events(); load+events 1969 -> 1655 us (19.7% -> 16.6% of 10 ms)`; at 2000
events `3254 -> 2244 us`. `events()` costs as much as the load that already verified the chain, because `walk_marked`
walks twice and runs `check_obj` on every record again. The replacement (one walk, no second crc, same decode) is
legal only with the load-time bad-set carried on the Hub (as `StockLog` does); the equivalence here is on a clean log.
`about(kind, Some(subject), 1)` (wallet.rs:308/372, channels.rs:596): `records=600: CURRENT 363 us | REPLACE walk_until
48.0 us | 8x`; `records=2000: 1201 -> 19.2 us (62x)`; `records=200: 122 -> 9.5 us (13x)`. The replacement stops at the
first matching record and decodes only it; it must still skip a crc-bad record (check the matched object), which the
bench's clean fixture does not exercise -- a test with one named corrupted cell is required.

### B9. `Graph::ppr` dangling loop (graph/retrieval.rs:135) -- class A, bit-identical

MEASURED (`rel-ppr.out`), 554-node fixture (venue + 12 categories + 76 ingredients + 165 dishes + 300 orders, 1890
edges), 12 iterations: `dangling=0: 96 -> 95 us (1.0x)`, `dangling=10: 253 -> 96 us (2.6x)`, `dangling=100: 1936 -> 109
us (17.7x)`. Iterating the seeds only (restart is zero elsewhere) is the same integer arithmetic in the same order, so
the ranking is identical; the closed form (sum the dangling mass once) changes rounding and was NOT used. How many
dangling nodes a real venue graph has is NOT MEASURED; with none, the gain is nil. Cold path (owner-assistant question).

### B11. Owner-console JS (node 22.22.1, `js_bench.out`, median of 9, `taskset -c 4`, run outside a slot: ~2 s total)

(a) admin/orders.js:93,99 `S.orders.filter(o => !liveOrders().includes(o))`: `n=300: CURRENT 3844 us | REPLACE 36 us |
105.6x`; `n=1000: 21605 -> 75 us (286x)`; `n=3000: 167 ms -> 252 us`. Main-thread, per render and per search
keystroke on the history tab. Identical arrays asserted.
(b) admin/app.js:316-322 `S.products.find` inside languages × categories × products: `dishes=165: CURRENT 1026 us |
REPLACE 226 us | loop-only 4.9x`; `dishes=500: 7972 -> 687 us (12.3x)`. Per `loadVenue` (which also runs after each
dish toggle). Identical objects asserted (JSON-equal).

### Not measured (compute hold, or not benchable in this lane)

- B5 promotions page `promo_uses -> orders_state(hub)` per promo (services/ordering/promotions.rs:39): ESTIMATE from B7a:
  each refold costs at least `Hub::load` 1.0 ms + `events()` 1.0 ms + the fold, times P promos, on the WORKER. With 5
  promos that is >= 10 ms = the cap. NOT MEASURED (`orders_state` is Worker code with workers-rs types).
- B8 stock ledger `levels: Vec` linear maps (stock/ledger.rs:8-43): NOT MEASURED; both live venues hold 0 supplies
  (memory: dowiz-stock-ledger-works-but-is-off), so n is 0 in production today.
- B10 SNN shadow path per request (snn.rs:52-105, infer.rs:63-127): NOT MEASURED; shadow mode, the budget test says < 1 ms.
- I4 Worker-side `with_catalog` (hubstore.rs:623): CITED W-AX0 148 ms debug per dubin price edit (decode 112 ms,
  journal 34.5 ms); the release decode is W-KVDEC's (998 -> 143 µs) and EXCLUDED here.
- The `Catalog::load`-for-one-field sites (scout table in §1b): the per-site cost is one decode (W-KVDEC's number, CITED)
  plus `Store::from_bytes` of ~0.5 MB; `with_catalog` already exists and is the mechanical replacement.
- Debug-profile twins of B1-B9: NOT MEASURED (hold); the command is in §6.

## 3. Ranked rows (by measured cost × frequency; Worker rows outrank object rows because of the 10 ms kill)

| # | file / fn | change | MEASURED gain (release) | where it runs, how often | risk | tests needed | verdict |
|---|---|---|---|---|---|---|---|
| 1 | workers/api/src/command/floor.rs:153 `sittings`; command/sitting.rs:44 `rounds`; services/orders/room/table_link.rs:106 `live_sitting` | parse each order once, group rounds by `sitting_id` in one pass (first-seen sitting order, rounds sorted by `created_at_ms`, stable) | 32.5 ms -> 1.45 ms at 100 orders (22x); 279 -> 4.6 ms at 300 (61x); share of cap 325% -> 15% | Worker, every staff floor poll and every QR placement at a table | low: pure function, grouping asserted identical; `rounds` keeps its signature for other callers | floor.rs tests + an equivalence test `sittings_old == sittings_new` over a 300-order fixture incl. orders without `sitting_id` and unparsable envelopes | GO (first) |
| 2 | workers/api/src/lib.rs:302 `router` | build the matchit table once per isolate (`OnceCell`/`thread_local`), route to `fn` pointers; keep `Req{now_ms}` as the per-request datum | 200.7 µs -> 0.19 µs per request (1063x); 2 % of the cap on EVERY request | Worker, every request | medium: 257 registrations change shape; the conflict check moves from runtime to the once-build (same `insert` errors) | `routes_table/tests.rs` kept; a test that every (method, path) probe resolves to the same handler as before; the browser gate | GO |
| 3 | workers/api/src/owner.rs:660-674 dashboard | test `e.seq < day_start` before parsing; `break` at the first older row (list is newest-first) | 4.6 -> 0.75 ms at 300 orders/50 today (6.1x); 1.4 ms -> 0 with nothing today | Worker, every dashboard poll | nil: identical tiles asserted; relies on newest-first order, which `orders_view` documents | tile equality test over a fixture with today/yesterday boundary rows | GO |
| 4 | crates/dowiz-hub/src/logimage.rs:216 `about(kind, subject, limit)`; consent/personal.rs:34 `newest`; catalog/edits/history.rs:30 `recent(n)` | `walk_until` with the kind/subject filter and `limit`, decoding only matches; crc-check the matched object (or carry the load-time bad-set) | 363 -> 48 µs at 600 records (8x); 1201 -> 19 µs at 2000 (62x); per wallet balance/statement and channel read | Worker + object, per request on 6 call sites | low-medium: must keep quarantine semantics | equivalence vs `entries().filter().take(n)` on a clean log AND on a log with one named corrupted cell (no fuzzers) | GO |
| 5 | workers/api/src/hubdo/journal.rs:95-150 + hubdo.rs:235 | keep the journal `LogImage` resident per generation (drop on write/compaction/rollback like `cat_checked`); keep the last `after` State as the next `before` | object-side write path 2265 -> 925 µs (2.4x); journal-only 1086 -> 318 µs (3.4x) at 522 records | object, per catalogue write (CITED debug: 34.5 ms journal of a 148 ms edit) | medium: ~0.7 MB more resident memory per object; invalidation on `v2_when_pinned`, compaction, failed `put_together` | the W-PITR2 journal tests + a test that a resident log after a rollback/compaction is re-loaded; `counters.journal_us` live before/after | GO-small (after W-KVDEC lands; the decodes are the other half) |
| 6 | crates/bebop-store/src/verify.rs:185 `walk_marked`; dowiz-hub read.rs:38 `Hub::events`; hubdo.rs:951 `put_log` | carry the load-time bad-set on `Hub`, walk once without re-crc; for `put_log`, walk only the k new records (`walk_until` the memo's tip) | `events()` 961 -> 647 µs at 600 events (1.5x), 3254 -> 2244 at 2000; the O(k) tail walk is ESTIMATE (not measured) | object, per command write and per cold fold | medium: a bebop-store API change; crc semantics must be provably unchanged | `events()` equality on clean and corrupted-cell logs; `folded_exactly` stays | GO-small |
| 7 | workers/api/public/admin/orders.js:93,99 | `const live = new Set(liveOrders())` once per `matching()`/`historyAll()` | 3.8 ms -> 36 µs at 300 orders (106x); 21.6 ms -> 75 µs at 1000 | browser main thread, per render/keystroke on the history tab | nil | node equivalence test (the bench) + the browser gate | GO |
| 8 | workers/api/public/admin/app.js:316-322 `loadVenue` | `Map` id -> product built once | 1.03 -> 0.23 ms at 165 dishes (4.9x); 8.0 -> 0.69 ms at 500 | browser, per venue load (after each dish toggle) | nil | same | GO (small) |
| 9 | services/ordering/promotions.rs:31-39 `promo_uses` | fold the log once, count uses by code into a map | ESTIMATE >= 2 ms × P promos on the Worker (from B7a); NOT MEASURED | Worker, owner promotions page | low | a count-equality test per code | GO-estimate (measure first) |
| 10 | crates/dowiz-hub/src/graph/retrieval.rs:135 `ppr` | iterate the seeds in the dangling loop | 1.0x with no dangling node; 2.6x at 10; 17.7x at 100 (bit-identical) | cold (assistant question) | nil | ranking equality (the bench's assert) | GO-tiny / NO-GO if real graphs have no dangling node (NOT MEASURED) |
| 11 | hubdo/fiscal.rs:47, routed.rs:127, timer.rs:85, room/till.rs:50, ebills.rs:73/101, exceptions.rs:51/93, reads.rs:69-156, fiscal/send.rs:95/215, room/offline.rs:203 | `with_catalog` (in place, crc memoised) instead of `Catalog::load` for one field | one decode per site: W-KVDEC's 998 -> 143 µs release CITED; ~0.5 MB `from_bytes` copy each | object, per placement / per write / per alarm | low (W-ZC pattern) | existing route tests | GO (mechanical, after W-KVDEC) |
| 12 | workers/api/src/hubstore.rs:623 `with_catalog` (30 Worker sites) | move catalogue EDITS into the object (`/fold/catalogue` write twin of W-ZC), so no 0.5 MB image crosses the hop twice per edit | CITED W-AX0: 148 ms debug per edit today; release decode EXCLUDED | Worker, every owner edit | high: architectural | the W-PITR2 journal + strict-body + browser gates | GO-plan (a lane of its own) |
| 13 | analytics/kitchen/sales.rs:70 `Window::bucket` | `partition_point` | 19 -> 10 ns/call at 62 days; 0.7x at 7 days | per order per analytics read | nil | — | NO-GO (noise) |
| 14 | crates/dowiz-hub/src/stock/ledger.rs:8-43 `levels`/`counted` | BTreeMap/BTreeSet | NOT MEASURED; n = 0 supplies on both live venues | per stock event | low | fold determinism (I4) | NO-GO now (re-rank when a venue fills its inventory) |
| 15 | crates/dowiz-hub/src/snn.rs:52-105, snn/infer.rs:63-127 | build `shipped`/`Menu`/`stalks` once per catalogue generation | NOT MEASURED; shadow mode, budget test < 1 ms | per taste request | low | shadow equality | DEFER |
| 16 | every "closed-form" candidate (tz.rs, hours.rs, cron/timer.rs:93 `ebills_next` minute stepping, notify/route.rs:203 weekday stepping, money/geo) | — | tz/hours are already O(1); the two stepping loops are <= 30 steps on a cold path | — | — | — | NO-GO: there is no loop in dowiz that a formula replaces for a measurable gain |

## 4. VERDICT

1. The operator's premise ("loops replaced by formulas") finds almost nothing in dowiz: the date/time code is already
   closed-form (tz.rs checks four fixed candidates; hours.rs sorts eight entries), the only algebraic loop that gains is
   `ppr`'s dangling loop (bit-identical seeds-only form, 1.0x on a graph without dangling nodes), and both stepping
   loops left (`ebills_next` minutes, weekday `next_at`) are <= 30 steps on cold paths. Class A rows: 1 GO-tiny, 0 GO.
2. What the sweep DID find is class B and C: repeated JSON parsing of the same order envelopes and repeated full
   decodes/crc-walks of images the object already holds. Those are the rows above, and three of them are Worker-side
   costs that exceed the 10 ms Free cap on their own: the staff floor poll (32 ms at 100 open orders, 279 ms at 300,
   MEASURED), the route table rebuilt on every request (0.2 ms = 2 % of the cap on every request, MEASURED), and the
   dashboard parsing every order before its date test (4.6 ms at 300 orders, MEASURED).
3. The object-side per-write path is 2.3 ms release at a 522-record journal and 165 dishes, of which 1.2 ms are the two
   catalogue decodes W-KVDEC owns and 1.1 ms is the journal re-verified and re-copied on every append; keeping the
   journal resident per generation takes the journal part to 0.3 ms (3.4x) with byte-identical output.
4. No instrument measured nothing this time: every row with a number has an `EQUIV ok` line and a raw output file; the
   rows without one say NOT MEASURED or ESTIMATE, and why.
5. Debug-profile numbers and the promotions refold are the two things left unmeasured, both by the compute hold.

## 5. Proposed docs/exp.journal line

`<ts> H:dowiz loops can be replaced by formulas/algorithms with a measurable gain | DID:R-LOOPS full sweep (2 scouts, ~140 inventory rows) + 9 release benches w/ equivalence asserts (rloops/results) | GOT:formulas: ~nothing (tz/hours already O(1), ppr 1.0x w/o dangling); algorithms: floor poll 32.5->1.45 ms @100 orders (22x, Worker, >cap), router 200.7->0.19 us/req (1063x), dashboard 4.6->0.75 ms, journal write 2.27->0.93 ms object-side, about(..,1) 363->48 us, admin JS 3.8 ms->36 us | VERDICT:GO rows 1-8 (floor first), NO-GO on formula hunting | COST:~4.5 h wall incl. 2 SIGKILLs + 1 slot refusal + compute hold; debug twins NOT MEASURED`

## 6. PAUSED / NEXT (compute hold from main, 2026-10-08 ~11:45Z)

DONE: inventory (§1, §1b, §1c), release benches B1-B4, B6, B7, B9, JS B11 (§2), ranked table (§3), verdict (§4).
HALF-DONE: nothing in flight; no slot held.
NEXT COMMAND (only after "BOX FREE"), debug twins of B2/B3/B4 for the W-AX0 comparison:
`cd rloops/benches/rbench && CARGO_TARGET_DIR=../../target PHANTOM_WAIT_S=3000 /root/dowiz/bebop-lang/tools/slot.sh rloops-build-dbg taskset -c 4 cargo build --offline -j1`
then `slot.sh rloops-run-dbg taskset -c 4 ../../target/debug/rbench journal 7` (and `orders 7`), outputs to `results/dbg-*.out`.
