# dowiz as a DAG: what is already dataflow, where the cycles are, and how to recompute only what changed

Research, 2026-09-27. Lane W-RESEARCH, READ-ONLY: nothing in the tree was changed, built or deployed.
Every code claim was checked against `/root/lanes/w-research` (worktree at `5b77de64`) by `sed`/`grep` on this
date; the line numbers are from that tree. Labels: **MEASURED** = a command run here or a number quoted from a
dated measurement document; **DOC** = a vendor page fetched on 2026-09-27 (URL and its "Last updated" beside it);
**EST** = arithmetic on measured inputs, inputs named; **(unverified)** = not checked. The operator's brief (A) —
"pure dataflow / reactive graph execution" — is treated as a hypothesis and tested clause by clause in §9.

Companion documents from the same lane: `docs/research/2026-09-27-sel4-wasm.md`,
`docs/research/2026-09-27-performance-paths.md`, `docs/research/2026-09-27-binary-size.md`.

---

## 0. The answer in ten lines

1. **At the storage layer dowiz already is a dataflow system.** Every venue state is `fold(events)` over an
   append-only, content-chained log, never a stored counter: orders (`workers/api/src/hubstore.rs:1246-1275`
   `orders_state`, `crates/dowiz-hub/src/room/delta.rs:89-113` `fold`/`fold_one`), stock
   (`crates/dowiz-hub/src/stock.rs:567-570` `fold`), money (`workers/api/src/wallet.rs:9-16` "a balance is
   replayed, never stored"), analytics (`workers/api/src/services/analytics/fold.rs:1-8` "NO ANALYTICS STORE"),
   the relation graph (`crates/dowiz-hub/src/graph.rs:10-25` "FOLDED, NOT STORED").
2. **The order FSM is a compile-time DAG with a test that refuses a cycle.** `allowed_next`
   (`crates/dowiz-core/src/order_machine.rs:100-124`), `FSM_ADJ` bitmask adjacency (`:216-222`), `topological_order`
   (`:324`), spectral radius proved 0 (`:409-427`), and the RED/GREEN tests at `:858`, `:1018-1022`, `:1259-1266`
   make a future `Reopen` edge fail the golden signature.
3. **What is NOT dataflow is the edges.** The graph's edges are hand-sequenced awaits inside 206 route handlers
   (`grep -c` on `workers/api/src/lib.rs`; 1,411 `.await` sites under `workers/api/src`, MEASURED) and there is
   exactly ONE memoised projection, `folded` (`workers/api/src/hubdo.rs:208`), dropped whole on every log write
   (`:1120`). So every write costs a full refold on the next read: O(events), not O(change).
4. **The business cycles are all already broken**, each in one of the two ways brief (A) names: a
   **per-transaction DAG** inside one object turn (placement writes log then stock from in-memory copies,
   `hubdo.rs:1623-1747`; the room's `write_both`, `workers/api/src/hubdo/room.rs:84-96`) or a **time barrier in an
   immutable log** (rotation's `Checkpoint` record, `crates/dowiz-hub/src/lib.rs:103-109`; `HOT_KEEP_MS`,
   `hubstore.rs:1015`; the outbox's `next_at_ms`, `workers/api/src/outbox.rs:302-304`; the amendment's `base_seq`,
   `crates/dowiz-hub/src/room/amend.rs:4-13`). §2 lists all ten.
5. **The 10 ms wall is not the fold; it is the edge carrying a source node.** The kills cluster where the Worker
   pulls the 538 KB catalogue image through ~40 `load_catalog` call sites (MEASURED, 40 today) and in the minute
   cron; the fold itself runs in the object where 0 of 50,155 requests exceeded CPU while whole-log folds took
   246 ms wall (`docs/design/BLUEPRINT-FREE-TIER-2026-09-26.md` §1.2, §2.3). The DAG rule that ends the kills:
   **an edge across the hop carries a derived node, never a source node.**
6. **Incremental recompute is one data structure away.** The 256-entry `recent` ring
   (`hubdo.rs:100-117` `changes_since`) is already a dirty set — served to clients and never used by the object's
   own fold. Keying the memo per order and applying `fold_one` on append turns the per-write cost from O(E) to
   O(one order's events) and the poll to a byte copy (§3).
7. **Timers become node-local.** The minute cron is a global tick that costs 1440 × (3 + 3V) object requests a day
   (MEASURED); Durable Object alarms are at-least-once with 6 retries (DOC) and are exactly "a node schedules its
   own next evaluation" (§4.3, = FT2).
8. **bebop is the bytes and the second reader, not the scheduler** (§5). Its store format is what the persisted
   nodes are; `bebop-wasm`'s four-reader fold is the node hash; running bebop code in the Worker stays AGAINST
   (`docs/design/BLUEPRINT-BEBOP-IN-WASM-2026-09-23.md` §0.6).
9. **Adopt the ideas, not the engines.** Salsa's revision/durability trick is dowiz's per-image generation;
   DBSP's "compute proportional to the change" is `fold_one` applied to the dirty set; the Dataflow Model's
   watermark is `HOT_KEEP_MS` and should become the till's closing rule. None of the three libraries fits a Worker
   with a 10 ms budget and twelve projections (§6).
10. **Plan (§7):** instruments → projections in the object → alarms → persisted/byte projections → declared
    edges. The first three rows (R1 incremental orders memo, R2 `/fold/menu` memo, R3 the `dataflow` gate) are
    each under 200 lines and each has a measurable gate.

---

## 1. Today's data flow, drawn as a graph

### 1.1 Nodes

**Source nodes** — images, one Durable Object per venue (`workers/api/wrangler.toml` `[[durable_objects.bindings]]
name = "HUB"`, `class_name = "HubImages"`; `hubdo.rs:190-218`), each stored as 96 KiB chunks with a `Meta
{ generation, chunks, len }` written last (`hubdo.rs:124-127`, `:1878-1911`):

| Image | Type | Where it is named | Grows by |
|---|---|---|---|
| `log` | `dowiz_hub::Hub` over EvLog v2 (`crates/bebop-store/src/evlog.rs:24-33`) | `hubdo.rs:133` `LOG_IMAGE` | one chained record per event (`crates/dowiz-hub/src/lib.rs:397-420`) |
| `stock` | `dowiz_hub::stock::StockLog` | `hubstore.rs` `IMAGE_STOCK` | one `StockEvent` per reserve/consume/release/receive |
| `catalog`, `settings`, `posts` | KV images | `hubstore.rs:1486-1487` `IMAGES` | whole-image rewrite (writers only) |
| `outbox`, `idem`, `ops`, `people`, `i18n`, `ebills` | `dowiz_hub::table::Table` | `outbox.rs:257`, `workers/api/src/idempotency/mod.rs:30-33`, `hubstore.rs:814-838` | "rewritten eagerly on every put — O(n)" (`crates/dowiz-hub/src/table.rs` header) |
| `audit`, `consent`, ledger | `dowiz_hub::logimage::LogImage` | `hubstore.rs:842`, `hubdo/room.rs:60` | append-only, chained (`crates/dowiz-hub/src/logimage.rs:1-32`) |
| platform: registry, sessions, identity | `Table` in the `__platform` object | `workers/api/src/platform_store.rs` | per request read whole into the Worker (OPT §A.2) |

**Derived nodes** — folds, all pure, all (today) recomputed from a source:

| Projection | Function | Memoised? | Consumers |
|---|---|---|---|
| orders view (newest first) | `hubstore::orders_state` (`:1253`) over `Hub::events_oldest_first` + `fold_one` | yes, one memo keyed by log generation (`hubdo.rs:424-438`) | `/fold/orders`, `/fold/order?id=`, `place`, `advance`, `assign`, analytics, promo count |
| one order | `room::delta::fold` (`:89`) | via the memo above | `/api/order/:id`, courier, tracking |
| stock ledger | `StockLedger::fold` (`stock.rs:567`) | no — refolded per command | `place::decide`, `advance::decide`, rebuild's `stranded` |
| venue record | `Catalog::load(...).location()` | no (`hubdo.rs:2062-2078` `/fold/venue`) | owner dashboard ("today" at the venue's midnight) |
| analytics report | `services::analytics::fold::fold` (`:98`) | no | owner dashboard |
| relation graph | `graph::Graph::of(hub, catalog)` (`graph.rs:179`) | no; "~20 ms against ~150 ms of storage" (`graph.rs:22-23`) | assist / MCP |
| outbox depth | `outbox::depth` (`:383`) | no | `/api/owner/health` |
| rebuild report | `rebuild::compare` (`workers/api/src/rebuild.rs:571`) | no, by design | conservation audit, law 8 |
| changes window | `recent` ring, 256 (`hubdo.rs:100`, `changes_since` `:117`) | in memory, cleared on any whole-image write (`:1954`) | `?since=` on the consoles, `replica.js` |
| courier positions | `positions` map, `POSITION_KEEP_MS` | in memory only, on purpose (`hubdo.rs:2036-2048`) | live ETA |

**Sinks / subscribers**: hibernatable sockets tagged `console` / `courier` / `courier:<id>` / `order:<id>` / kitchen
(`hubdo.rs:142-155`, `:468`, `:1956`), the console's `localStorage` replica
(`workers/api/public/lib/replica.js:1-25`), Telegram / WhatsApp / the print rail via the outbox drain
(`workers/api/src/outbox/rails.rs:56`, `:225`), the fiscal queue (`hubdo/fiscal.rs`), the nightly S3 copy
(`workers/api/src/cloud.rs`), and two inbound external edges: Stripe webhooks and the ebills.al poller
(`workers/api/src/ebills/poll.rs:1-9`).

### 1.2 Edges

```
 client ──HTTP──▶ Worker handler (auth, venue from Host, pricing) ──command::send──▶ venue object /fold/<cmd>
                                                                                        │
              ┌─────────────────────────────────────────────────────────────────────────┤ one turn
              │  image(log) ─┐                                                          │
              │  image(stock)─┼─▶ decide(&mut hub, &mut stock, &listed, ...)  (pure)    │
              │  orders_view ─┘        │ Ok                                              │
              │                        ▼                                                 │
              │        put_image(log) ─▶ put_image(stock) ─▶ enqueue_bell ─▶ outbox      │
              │                 │                                  │                     │
              │                 ▼ broadcast (after the write)      ▼ (minute cron / alarm)
              │           sockets: console, courier, order:<id>   drain ─▶ Telegram/WhatsApp/print
              └──────────────────────────────────────────────────────────────────────────┘
 poll ──▶ Worker ──▶ /fold/orders ──▶ folded memo (gen match) or refold(log)  ──▶ JSON ──▶ console/replica
 nightly cron ──▶ per venue: rotate(log → log@<gen> + Checkpoint) ─▶ backup ─▶ witness
 minute cron  ──▶ per venue: cron~<venue> runner object ──▶ drain_venue, ebills tick, fiscal minute
```

- **Command edge**: Worker → object, no generation header, "the read and the write are two statements inside one
  object turn" (`workers/api/src/command/mod.rs:218-228`). The object holds every image in `mem`
  (`hubdo.rs:196`), so a command's fallible work runs on in-memory copies and writes only on `Ok`
  (`command/mod.rs:178-183`; `hubdo.rs:1667-1671` "NOTHING HAS BEEN WRITTEN").
- **Subscription edge**: `broadcast` runs "AFTER THE WRITE LANDED, never before" (`hubdo.rs:1583-1586`,
  `:1744`); only order kinds travel, a non-order kind sends `moved` and clears the window (`:455-475`).
- **Effect edge**: an outbound message is WRITTEN into the venue's `outbox` image in the same turn as the event
  (`outbox.rs:238-242`: "a write that landed is a message that will be delivered; a write that did not is an
  order that was not placed"); the drain is separate and retries by `next_at_ms` (`:276-284`, `:345-354`).
- **Timer edge**: `#[event(scheduled)]` reads one clock (`lib.rs:576-594`), then `cron::minute` sends ONE request
  per venue to a `cron~<venue>` runner object, "a separate instance of the same class, so it can call the venue's
  own object without calling itself" (`workers/api/src/cron.rs:21-25`, `:457-478`).
- **Verification edge**: `rebuild` refolds from bytes, ignores the memo, and crosses the log against the stock
  ledger (`hubdo.rs:924-961`; `rebuild.rs:1-13`).

### 1.3 The parts that already ARE dataflow (with the line that proves each)

| Property | Where | Evidence |
|---|---|---|
| Append-only, content-chained source | `evlog.rs:6-10` | "an append allocates ONE object, relinks the root, and commits. That is O(1) per insert" |
| id commits to the record before it | `crates/dowiz-hub/src/lib.rs:412-416` | `content_id_chained(&prev, &payload)`; `chain_check` (`:710`) |
| State = fold | `hubstore.rs:1253`, `delta.rs:89`, `stock.rs:567`, `wallet.rs:11-16`, `workers/api/src/services/analytics/fold.rs:1-8`, `graph.rs:10-19` | each header says "never stored" in its own words |
| Events are deltas, old images still fold | `delta.rs:13-19`; test `workers/api/src/fold.rs:47-86` | `"_d": true` marks a delta; unmarked = snapshot replaces |
| Pure deciders, native tests | `command/place.rs:117`, `command/advance.rs:82`, `command/assign.rs:86`, `room/amend.rs:207`, `room/pay.rs:146` | "bytes in, bytes out, no clock and no I/O — exercised by ordinary `cargo test`" (`place.rs:271-275`) |
| Memo keyed by version | `hubdo.rs:201-208`, `:428-437` | "Keyed by generation so it cannot go stale" |
| Effects as data | `outbox.rs:286-329` `Entry`, `:331-354` `Verdict`/`after_attempt`; `hubdo/routed.rs:81-87` `Owed` | idempotent id `(order, channel)` (`:288-292`) |
| Rebuild-and-diff | `rebuild.rs:518-543`, `:571-636` | `stale`, `stranded`, `unheld`; "EMPTY IS THE ONLY ACCEPTABLE ANSWER" |
| Same fold on the client | `replica.js:14-18`; `crates/bebop-wasm/src/lib.rs:26-29` `decide` feature | the tablet runs `amend`/`pay` deciders offline ("MOVED FROM `workers/api/src/command/amend.rs` (D7 phase 1)", `amend.rs:23-25`) |
| The clock is an input | `lib.rs:239-241` `Req { now_ms }`; 9 `Date::now` sites left (MEASURED; the `clock` gate allows four places) | `place::PlaceIn.now_ms`, `stock.set_clock(input.now_ms)` (`hubdo.rs:1654`) |
| The FSM is acyclic, and tested so | `order_machine.rs:100-124`, `:216-222`, `:324`, `:1259-1266` | `green_reopen_edge_flips_gate_fields` |
| Log rotation = time barrier | `lib.rs:103-109` `Checkpoint = 6`, `:621` `rotate`; `hubstore.rs:1015` `HOT_KEEP_MS = 30 days`, `:1046-1056` | "MOVED VERBATIM — same ids, same `prev` links — so `chain_check` still verifies" |

### 1.4 What is NOT dataflow today

1. **Edges are imperative and unnamed.** Nothing in the tree lists "projection P depends on images {A, B}". The
   dependency lives in each handler's sequence of awaits. The `rebuild` audit knows two edges (log → orders,
   stock → ledger) and no others.
2. **One memo, invalidated whole.** `put_image` sets `folded = None` on any write to `log` (`hubdo.rs:1120`), so a
   placement, an `advance`, a `Noted` ack or a rotation each cost the next poll a full refold. With `HOT_KEEP_MS`
   at 30 days, a venue at 30 orders/day × 6 events holds ≈ 5,400 hot events (EST); the graph fold measured
   ≈ 20 ms for the same walk (`graph.rs:22-23`), which is fine in the object (budget 30 s DOC) and was the whole
   problem when it ran in the Worker (budget 10 ms).
3. **In-memory state does not survive eviction.** "After a brief period of inactivity, the Durable Object will be
   evicted, and all in-memory state will be lost" (DOC,
   https://developers.cloudflare.com/durable-objects/examples/durable-object-in-memory-state/), and the object
   was MEASURED hibernating (218 s active in a 15,983-request day, FREE-TIER §0.7). So `mem` and `folded` are
   rebuilt on every wake: one `get_multiple` of every chunk (`hubdo.rs:1482-1496`) plus a full fold. The memo is
   a cache, not a checkpoint.
4. **Whole source images still cross the hop.** 40 `load_catalog(` call sites and 5 `hubstore::load(` sites
   (MEASURED: `workers/api/src/exceptions.rs:69`, `cloud.rs:643`, `services/customers/handlers.rs:222`,
   `services/engagement/assist.rs:253`, `services/operations/waste.rs:203`); the platform registry is `Table::load`ed
   whole in the Worker per request (`platform_store.rs:159-170`, OPT §A.2).
5. **Timers are a global tick.** `crons = ["17 3 * * *", "* * * * *"]` (`wrangler.toml`); no `set_alarm` anywhere
   under `workers/api/src` (MEASURED, grep = 0).
6. **Two folds of the same thing can disagree** and the system relies on a nightly diff to say so: memo vs
   fresh (`rebuild`), the browser replica vs the server ("a replica that argued with the server would be a second
   fold, and two folds disagree eventually", `replica.js:14-18`).
7. **Brief (A)'s "native fit with tensor ops" has nothing to attach to.** No tensor op is on any request path;
   the spectral machinery in `dowiz-core` (`order_machine.rs:409-427`) is a proved constant. This clause of the
   brief is inert for dowiz.
8. **The node evaluator is one 410 KB function.** MEASURED (`tools/evals/collect/wasm.py` over the 2026-09-27
   bundle, through `slot.sh`): `<HubImages as DurableObject>::fetch` (`hubdo.rs:1170-1512`) is 409,879 bytes —
   every `/fold/*` command's future inlined into one `match`, the largest function in a 3.79 MB code section.
   The declared-edge table of §7 (Phase 4) is also the split of that dispatcher into one function per node
   (`2026-09-27-binary-size.md` §4 row 5).

---

## 2. The cycles, and how each is already broken

Brief (A) says the hard part is cycles in business processes and offers two remedies: a DAG per transaction, or
time barriers fixed in immutable event logs. Both exist in the tree. The rule that emerges: **nothing in dowiz
mutates a node in place; a "cycle" is always a new record that points backwards, and the only forward-pointing
structure is the derived memo.**

| # | Business cycle | Where it lives | How it is broken today | Which remedy |
|---|---|---|---|---|
| 1 | order created → cancelled → "restored" | `order_machine.rs:118-122`: `Cancelled => &[]`, `Rejected => &[]` | there is no restore edge; the FSM refuses it and the golden-signature test fails on a `Reopen` (`:1259-1266`). A restored order is a NEW order id with its own `Placed` | per-transaction DAG, by construction |
| 2 | order past PENDING must end (refund) | `workers/api/src/hubdo/refund.rs:4-13`, `command/refund.rs` | a forward path `→ REFUNDING → COMPENSATED_REFUND`, both edges in one turn when no money was taken; "what is owed back is what was taken, read off the order's own evidence" | per-transaction DAG |
| 3 | order ↔ stock (reserve / consume / release) | `command/advance.rs:59-62` `settlement`; `stock.rs:1455` `settle` | log and stock decided on copies, written log-first-then-stock in one turn, the order chosen so the recoverable failure is the visible one (`hubdo.rs:1612-1622`); `stranded` is the cross-node conservation check (`rebuild.rs:505-510`) | per-transaction DAG + nightly diff |
| 4 | order → outbox → kitchen → ack → order | `hubdo.rs:1702-1724` enqueue; `hubdo/kitchen_ack.rs` | the ack is a new `Noted` event, not a mutation; the outbox entry id is `(order, channel)` so a replay overwrites (`outbox.rs:288-292`); retries by `next_at_ms`, abandoned after 6 (`:268`, `:341`) | time barrier in an immutable log |
| 5 | payment ↔ status | `lib.rs:80-84` `Paid = 3` "NOT a status transition" | paying is its own fact; the FSM never sees it; refund reads `payments[]` / `amount_received` | append, no edge |
| 6 | courier / venue settlement | `wallet.rs:9-16`; `TxRow.reverses: Option<String>` (`:34-36`) | postings journal; a reversal is a NEW posting that names the old one; balance = sum; "the ledger orders by its own append order, not by a timestamp a writer supplied" | immutable log with backward references |
| 7 | two tablets amend one round | `room/amend.rs:4-13` | intents commute; `Remove`/`SetQty`/`Comp` carry `base_seq` and a stale one is refused ("tick 100 vs 104") | per-transaction serialisation (single writer) + version barrier |
| 8 | history grows without bound | `lib.rs:103-109` `Checkpoint`; `hubstore.rs:1015`, `:1040-1056` `rotate` | finished orders older than 30 days move verbatim to `log@<generation>`; the hot log starts with a `Checkpoint` naming the archive's tip; `keep` is asked per ORDER, never per event | time barrier (a watermark) |
| 9 | an object calling itself | `cron.rs:14-25`, `:429-432` `runner_name` | the runner is `cron~<venue>`, `~` is in no location id, so a runner can never be the venue's own object | topology rule |
| 10 | external till → orders (ebills import) | `ebills/poll.rs:1-9`; `hubdo/ebills.rs:89` `ebills_import` | inbound edge, bounded per firing, mapped to `Placed`/`Paid` under external ids; the poller state is its own `Table` | append + idempotent id (detail unverified) |

Two cycles are NOT yet closed and the DAG makes both visible:

- **Rows 3 and 8 together**: a rotation moves an order's events out of the hot log while the stock ledger may
  still hold a reservation for it. `rotate`'s `keep` decides per order by `is_over` (`hubstore.rs:1053-1056`); a
  live order is kept, so the leak needs `rebuild`'s `stranded` on the archive too. Today `stranded` is checked
  against the HOT log only (`hubdo.rs:942-950`). → R5 in §7.
- **The till (A7) has no watermark.** The X/Z report as the ninth conservation law needs a rule for an event
  whose `seq` is before the close but which arrives after it (a courier's cash `Paid` from a dead zone). This is
  exactly the Dataflow Model's late-data question (§6.3); recommend the rule "late events post to the next
  shift and the Z report names them" before A7 lands.

---

## 3. Incremental recompute: the design

### 3.1 What exists, precisely

- `folded: RefCell<Option<(i64, Vec<OrderView>)>>` — one key (the log generation), one value (the whole list),
  filled by `orders_view` (`hubdo.rs:424-438`), cleared by `put_image` on `LOG_IMAGE` (`:1120`).
- `recent: RefCell<Vec<Change>>`, kept to `RECENT_KEEP = 256` (`:100`, `:483-484`), answered by
  `changes_since(recent, since)` → `Some(changes)` or `None` = "ask for the list" (`:117`). This is a dirty set with
  a generation on each entry. It is used only by clients.
- `fold_one(state, raw)` (`delta.rs:100-113`), "exposed so a caller folding many orders in one pass does not have
  to re-walk a history per order" — the incremental step already has a name.
- `StockLedger::apply(ev)` inside `fold` (`stock.rs:567-570`) — the incremental step for the ledger.

### 3.2 The projection node

Replace the single memo with a projection that knows its own dirty set:

```
struct Orders {
    generation: i64,                              // the log generation this is current at
    by_id: HashMap<String, (u64 /*newest seq*/, serde_json::Value /*fold*/)>,
    newest_first: Vec<String>,                    // the order the consoles render in
    bytes: Option<Vec<u8>>,                       // the serialised list, made once per generation
}
```

- **On `append` (the single-event path, `hubdo.rs:1560-1594`)**: if `orders.generation == expected`, apply
  `fold_one` to `by_id[order_id]` (or insert), move the id to the front, set `generation = next`, drop `bytes`.
  Cost: one JSON parse of the delta. O(1) in the log size.
- **On a command that appends k events to an in-memory `hub` (`place`, `advance`, `assign`, the room)**: the
  object knows k = `hub.len() - before` (it already computes `events = hub.len()`, `:1673`); take the k newest from
  `hub.events()` (newest-first, `lib.rs:521-525`) and apply them oldest-first. Still O(k).
- **On a whole-image write that is not an append (rotation, import, `forget`)**: drop the projection; the next
  read refolds. These are nightly or rare.
- **On a cold object**: refold once from bytes (today's cost, ≈ 20 ms EST from `graph.rs:22-23`), then stay
  incremental for the object's life.
- **The poll path**: `/fold/orders` answers `bytes` (a copy) — no fold, no `serde_json::to_string`. The Worker
  passes the body through instead of `Response::from_json` → parse → re-serialise (today
  `hubstore::orders` deserialises into `Vec<OrderView>` and `owner::orders` re-encodes it).

The same shape for the other derived nodes:

| Node | Key (input generations) | Incremental step | Dirty set |
|---|---|---|---|
| stock ledger | `stock` generation | `StockLedger::apply(ev)` per appended `StockEvent` | events appended in the turn |
| menu (rendered JSON per locale) | `catalog` generation × locale | none; recompute per catalogue write (writers are rare) | whole |
| venue record | `catalog` generation | none | whole |
| analytics report | `log` generation × zone × window | per day bucket: add the changed order's day; a refund subtracts | the order ids in the dirty set |
| relation graph | `log` gen × `catalog` gen | keep lazy; build on first ask, drop on either write | whole |
| outbox depth | `outbox` generation | none (a Table put rewrites the image anyway) | whole |

### 3.3 Invalidation rule

A memo carries the tuple of input generations it was folded from; it is valid iff that tuple equals the current
one. If invalid and the node has a dirty set for the delta between the two generations, apply the incremental
step; otherwise refold. This is Salsa's revision + durability idea (§6.1) reduced to a tuple compare: dowiz has
about twelve projections, so the "dependency graph" is a twelve-row table, and a hand-written table is
debuggable where a framework is not (`docs/design/BLUEPRINT-MODULAR-ARCHITECTURE-2026-09-21.md` §9 makes the same
argument against a bus).

### 3.4 Checkpoints: persist the projection, or not

A persisted projection (`proj:orders@<gen>` as its own image, written in the same `put_image` transaction) would
let a read-only wake load the projection's chunks instead of the log's, and skip the fold. Costs and the reason
to defer it:

- +1 row written per order event. Rows written is the third Free-plan wall (100,000/day, DOC; 11,315 measured on
  an import day, FREE-TIER §1.2), and every daily cap fails closed for every venue at 00:00 UTC (§0.6 there).
- The projection is a second account of the log. Its correctness must then be under `rebuild` (law 8), which is
  cheap (`stale = []` already exists) but must run against the persisted copy, not the memo.
- The cold-read saving is real only if a wake serves reads without writes. Unmeasured: the ratio of read-only
  wakes. **Gate before building:** `do_reads_per_wake` from the analytics API for one week; persist only if a
  wake is mostly polls.

Recommendation: R1 in memory first; R4 persisted only after the measurement, and then as a bebop KV image so the
four readers can check it (§5).

### 3.5 Correctness under the DAG

`rebuild` (`hubdo.rs:924-961`) is the property test of the whole scheme: "refold from the BYTES, ignoring the memo,
and compare" (`rebuild.rs:9-12`). Extend it to every incremental node (ledger memo, analytics memo, persisted
projection) and keep it READ-ONLY ("a gate that fixes what it finds is a gate whose findings nobody ever sees",
`hubdo.rs:917-919`). The nightly conservation audit already runs it as law 8; `stale = []` and `stranded = []`
become the DAG's invariant, per venue, per night.

---

## 4. Cutting Worker CPU under the Free plan's 10 ms

MEASURED (FREE-TIER §1.2, §2, 2026-09-24..26): Worker success p50 2.7 ms, p90 20 ms, p99 91 ms; 622 kills at
exactly 10,000 µs; kill hours have ≥ 100 KB of object bytes per Worker request; the minute cron p50 9.7 ms with
656 of 1,440 runs over 10 ms; the object: 0 `exceededCpu` in 50,155 requests, wall p99 246 ms. DOC (limits page,
Sep 5 2026): Workers Free CPU 10 ms per request and per Cron Trigger; Durable Objects "CPU per request 30 seconds
(default)" (DO limits page, Jun 1 2026) while its FAQ says objects follow the Workers plan — the two sentences
conflict and FT13's deliberate over-10-ms probe is the way to settle it (OPEN 2 there).

The DAG restates the Free-tier blueprint as three rules and shows why each one cuts CPU:

1. **Worker = edge, object = node.** The Worker authenticates, resolves the venue from the Host, prices (pure,
   small) and forwards. It never evaluates a node. Today `storefront::menu`, `attach_one`, the manifest and ~40
   other sites evaluate the catalogue node in the Worker (FT1). Expected: 10 → 1 ms per such request (FT1 EST).
2. **An edge carries a derived node, never a source.** `/fold/menu`, `/fold/product?id=`, `/fold/products?ids=`,
   `/fold/venue` (exists) replace `load_catalog`; `/fold/orders` answers bytes. The parse moves to the object,
   memoised per generation, so it runs once per catalogue change instead of once per request.
3. **A timer is a node's own alarm.** No global tick; an idle venue schedules nothing (§4.3). The cron CPU line
   (9.7 ms p50, growing with V) disappears from the Worker.

Two more CPU sinks the graph view exposes:

- **Double serialisation on every poll**: object `Response::from_json` → Worker `res.json()` → Worker
  `Response::from_json`. With byte memos the Worker forwards the body. EST saving: the p50 of `/api/owner/orders`
  (unmeasured per route — Workers Logs are dashboard-only on Free, FREE-TIER OPEN 1).
- **argon2id login** (EST 20–60 ms in wasm, FREE-TIER §0.2c) is a node evaluation and belongs in an object
  (FT4), not in the Worker.

### 4.3 Alarms as node-local timers

DOC (alarms page, Apr 21 2026): one alarm per object; `setAlarm(ms)` replaces the previous; "guaranteed
at-least-once execution ... retried using exponential backoff, starting at 2 second delays for up to 6 retries";
"Only one instance of `alarm()` will ever run at a given time per Durable Object instance"; the handler may
reschedule itself. That is the outbox's own contract (`outbox.rs:243-248`, `MAX_TRIES = 6`, `backoff_ms`). The
object already holds the entries; `enqueue_bell` would call `set_alarm(min next_at_ms)` and `alarm()` would run
`drain` for this venue and reschedule. The ebills tick and the fiscal minute (`cron.rs:482-487`) move the same
way. Saving MEASURED-derived: −8,640 object requests/day at V = 2, −1,440 Worker invocations/day (FT2). Risk: the 50
external-subrequest cap per invocation applies per alarm run — drain at most ~40 and reschedule (FT2).

---

## 5. bebop's role in the DAG

- **The bytes.** `crates/bebop-store` (2,241 lines, "ZERO dependencies: std only") defines the image the persisted
  nodes are: EvLog v2 packs eight payload bytes per cell (`evlog.rs:24-33`; "A 330-byte event is 345 cells in v1
  and 54 in v2", `:35-38`), a commit is superblock + PartTab (21 cells per commit, memory
  `bebop-parttab-21-cells`), and an append touches chunk 0 and the tail (`hubdo.rs:1887-1893`).
- **The second reader.** `crates/bebop-wasm/src/lib.rs:1-16`: the KV root hash and the log fold are "byte-for-byte"
  the same across `kv.bp`, `bebop_store`, dowiz-core and `oracle.py`; CI runs the four-reader gate
  (`.github/workflows/ci.yml:54-57`, `:71-72`). In DAG terms that is a **node hash an independent reader can
  recompute** — the property a persisted projection (R4) must have before it is trusted.
- **Not the scheduler, not in the Worker.** `docs/design/BLUEPRINT-BEBOP-IN-WASM-2026-09-23.md` §0.2-0.6: bebop's
  emitter is AArch64-only, its builtin surface is twelve Linux syscalls, all three routes to "bebop in wasm" are
  AGAINST; `docs/measurements/COSTS-NOW-AND-BEBOP-2026-09-26.md` §0.5: bebop CPU is 2.6–10.6× Rust. The DAG runtime
  is the Rust in `hubdo.rs` and `crates/dowiz-hub`; bebop.bin reads the same bytes natively on a machine that has
  one.
- **On the Box.** `tools/native-spa-server/src/hub.rs:40-58` keeps the same images as files (`orders.store`,
  `stock.store`, ...) with "ONE process is ONE writer ... a mutex around the store is the whole of that problem"
  (`:8-11`). The graph is identical; only the edge transport changes (see `2026-09-27-sel4-wasm.md`).
- **What bebop would gain from the DAG**: the undone Kv packing (phase 4b, memory `dowiz-hub-seven-phases`) is a
  source-node change that every reader must move on in one commit (repo rule 10) — the declared-edge table (§7
  Phase 4) is where such a change is listed before it is made.

---

## 6. Against the literature, briefly

### 6.1 Salsa (rust-analyzer)
"Every query is used like a function K → V ... results are memoized ... when you make changes to the inputs,
we'll figure out when we can re-use these memoized values" (https://github.com/salsa-rs/salsa README). The
durability trick — "a memo stores one current result, not a history"; inputs tagged by how often they change so
a revision of a rarely-changing input does not re-verify everything
(https://rust-analyzer.github.io/blog/2023/07/24/durable-incrementality.html) — is dowiz's per-image
generation: the catalogue changes weekly, the log changes per order, and a menu memo keyed by the catalogue
generation is never touched by an order. **Adopt the idea; not the crate**: Salsa serves thousands of queries
with dynamic dependency tracking; dowiz has ~12 static projections and a Worker where every crate is code bytes
(`2026-09-27-binary-size.md`).

### 6.2 DBSP / Feldera
DBSP (Budiu et al., VLDB 2023, https://www.vldb.org/pvldb/vol16/p1601-budiu.pdf) incrementalises any relational
query so that work is proportional to the change (Z-sets, streams of deltas). dowiz's `"_d"` deltas are already
a change stream and `fold_one` is the incremental operator for one order. The two aggregates an owner reads
(revenue per day, top dishes; `workers/api/src/services/analytics/fold.rs:15-30`) are Z-set folds: +order on `Placed`, −order on
`COMPENSATED_REFUND`. **Hand-roll those two**; Feldera is a server-shaped engine and nothing that ships in a Worker.

### 6.3 The Dataflow Model and watermarks
Akidau et al., VLDB 2015 (https://research.google/pubs/the-dataflow-model-a-practical-approach-to-balancing-correctness-latency-and-cost-in-massive-scale-unbounded-out-of-order-data-processing/):
event time vs processing time; a watermark is the assertion "no event older than t will arrive" (Begoli et al.,
"Watermarks in Stream Processing Systems", VLDB 2021, http://www.vldb.org/pvldb/vol14/p3135-begoli.pdf); Flink
implements the same (https://nightlies.apache.org/flink/flink-docs-stable/docs/concepts/time/). dowiz's `seq` on
each record is the event time at append (`hub.append(kind, id, payload, ev.clock, ...)`, `hubdo.rs:1578`) and
`HOT_KEEP_MS` is a 30-day watermark for rotation. The till's Z close is the second watermark the product needs
(§2, after the table). No engine is needed: the watermark is one number per venue in the settings image.

### 6.4 The actor as the node host
DOC (DO limits page): "Each individual Object is inherently single-threaded"; soft limit 1,000 requests/s per
object; in-memory state between requests (until eviction). That is the reason the per-transaction DAG works
without a lock, and the reason the platform object (`__platform`, one for all venues) is the one node whose
request concentration the graph must watch (OPT §A.2: ≈ 9 platform requests/s at 200 venues, EST).

---

## 7. Migration plan, phased, with gates

Each phase names its gate as a number that already has, or gets, a collector under `tools/evals`.

| Phase | What | Gate (must hold to advance) |
|---|---|---|
| 0 Instruments (= FT13) | `cf.cpu_kills_day`, `cf.do_response_bytes_day / cf.do_requests_day`, per-route p50 where the analytics API allows; a new gate `dataflow` (R3) with today's baseline | baselines written; the over-10-ms object probe answers OPEN 2 |
| 1 Projections in the object | R1 incremental orders memo; R2 `/fold/menu` + `/fold/product(s)` memoised per catalogue generation, hot readers rewritten (FT1a); byte pass-through on polls | nightly `rebuild.stale = []` on every venue for 7 days; `cf.cpu_kills_day = 0` for 7 days; `dataflow` count on poll paths → 0 |
| 2 Node-local timers (= FT2) | outbox drain, ebills tick, fiscal minute as per-object alarms; the `* * * * *` cron removed; `record_cron` to the nightly | `cf.do_requests_day` on a no-order day < 1,000; outbox `Depth.oldest_ms` unchanged in behaviour |
| 3 Cross-object inputs versioned (= FT8) and, if the §3.4 measurement says so, persisted projections (R4) | registry read once per minute keyed by its generation; `proj:*` images under `rebuild` | platform-object requests per Worker request < 0.1; rows written per delivered order ≤ 4 |
| 4 Declared edges | one table in `workers/api/src/hubdo.rs`'s neighbourhood: `(inputs, projection, step)`; a test that every `/fold/*` route names its inputs; `rebuild` iterates the table; `unreached.py`-style refusal of a projection nothing reads | the table's row count equals the `/fold/*` route count; law 8 covers every row |
| 5 The Box | the same graph on `tools/native-spa-server` with file images and a mutex; edge transport only changes | `2026-09-27-sel4-wasm.md` §4 |

### The first three rows, lane-sized

**R1 — incremental orders memo** (`workers/api/src/hubdo.rs`; ≈ 150 lines, tests beside `changed_chunks`'s at
`:243-343`). Replace `folded` with the `Orders` projection of §3.2; `append` applies `fold_one`; commands apply
their k newest events; whole-image writes drop it. Tests: (a) a delta history and a snapshot history give the
same projection after N appends (the property `fold.rs:47-86` already states, now through the memo); (b)
`rebuild` after 100 random appends reports `stale = []`; (c) a whole-image write drops the memo and the next
read refolds. Mutation proof: skip the `fold_one` on append and watch (b) go red. Gate: `stale = []` nightly.

**R2 — the catalogue as a projection** (`hubdo.rs` + `workers/api/src/storefront.rs` + `workers/api/src/live_eta.rs`;
≈ 200 lines). `/fold/menu?locale=` and `/fold/products?ids=` memoised per catalogue generation, rendered once,
answered as bytes; the four hot readers (`/api/menu`, `attach_one`, `attach`, manifest) pass the body through.
`hubstore::load_catalog` stays for writers and export. Gate: `cf.cpu_kills_day = 0` for 7 consecutive days and
`cf.do_response_bytes_day / cf.do_requests_day < 20 KB` (FT1's numbers).

**R3 — the `dataflow` gate** (a new script and its `.prove.sh` under `tools/gates`, ≈ 60 lines, same shape as
`tools/gates/one-image.sh`). Counts `hubstore::load(` and `load_catalog(` call sites outside an allowlist of
writers/export/nightly (`with_catalog`, `seed_*`, `export`, `cloud.rs`), strips comments first (the epitaph trap,
`docs/design/ROADMAP-2026-09-22.md` §5), baseline = today's 45, ratchet down. Prove it fires by adding one call
site in a scratch copy. Gate: the number may only fall.

**R5 (queued behind R1)** — `rebuild` checks `stranded` against archives too: `archive_orders`
(`hubstore.rs:1212`) already reads `log@<gen>`; cross its terminal orders against the ledger's open reservations.

---

## 8. What was NOT verified, and the command that would settle each

- The object's real CPU cap (OPEN 2): a deliberate 15 ms fold in `/fold/rebuild` on the QA hub, read back from
  `durableObjectsInvocationsAdaptiveGroups.exceededCpuErrors`.
- The share of read-only wakes (§3.4): `durableObjectsInvocationsAdaptiveGroups` by object and method for a
  week; no token on this box reads Workers Logs (FREE-TIER OPEN 1).
- The 20 ms fold figure is the graph's, not `orders_state`'s; time `orders_state` on the 5,830-cell live log via
  `/fold/rebuild`'s wall time before and after R1.
- Row 10 of §2 (ebills idempotency by external id) was read from headers only.
- Nothing in the plan needs a new crate, a new language or a new platform; that claim is by construction of §7,
  not by a build.

---

## 9. Brief (A), clause by clause

| Clause | Verdict for dowiz |
|---|---|
| "no function colouring / async hell" | The 1,411 awaits are I/O at the edges (storage, stubs, fetch). A DAG does not remove them; it keeps nodes pure (already true: five `decide` functions, 317 hub tests) and confines awaits to `put_image`/`send`. The earlier verdict AGAINST a general effect system (`docs/design/BLUEPRINT-ARCHITECTURE-EVOLUTION-2026-09-22.md` P7) stands; the DAG is its cheaper form |
| "incremental computation — only nodes downstream of a change recompute" | True at the level of one order once R1 lands; true across projections once §3.2's table is the code. Today: one memo, dropped whole |
| "native fit with tensor ops" | Inert: no tensor op on any request path (§1.4.7) |
| "cycles: DAG per transaction or time barriers in immutable event logs" | Both already used; §2 lists ten cycles and the remedy each one has. Two gaps named (archive `stranded`, the till's watermark) |
| "dowiz має прийняти DAG архітектуру" | Accepted as **make the graph explicit and evaluate nodes where the bytes are** — not as a framework. The measurable form is §7 |
