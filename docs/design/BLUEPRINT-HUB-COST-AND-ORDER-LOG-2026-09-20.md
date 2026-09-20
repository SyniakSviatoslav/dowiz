# Blueprint: the cheapest hub — where every byte and every request goes, and how to remove most of them

Date: 2026-09-20, revision 2 (after the operator's brainstorm request). Status: phase 1
SHIPPED (af5b0fc), phase 1.5 items 1 (part), 3 and 4 SHIPPED, the rest of 1.5–7 DESIGNED.
Numbers are MEASURED unless marked *estimate*.
Calculator: https://claude.ai/artifact/XLTpcL2RXZ1z52yeJC1UnY

## 0. The measured state (one venue, 30 orders/day, 10 h open, 1 courier, 5 visits per order)

| Fact | Value | Source |
|---|---|---|
| Cloudflare plan | Workers Free, zone Free | `wrangler.toml` (error 10097 forced `new_sqlite_classes`); zone API |
| One cold storefront visit | 40 responses: 20 static (free), 1 root, 1 API, **18 `/media` photos = 2.69 MB, 150 KB each** | Playwright on `sushi-durres.dowiz.org` |
| Photo pipeline | stored as uploaded, no resize, no thumbnail; grid cards load the full photo | `extra.rs` upload, `store/menu.js` |
| One `/media` photo | 2 KV reads, no edge cache | `extra.rs` media (fixed in phase 1) |
| One delivered order | D1 12 reads + 5 writes; DO 12 GET + **8 PUT of the whole image** | `hubstore::with_hub` call sites |
| Status poll | every 12 s; loaded the log twice + catalogue | `store/track.js`, `live_eta.rs:299` (fixed) |
| Owner console | every 15 s, 2 requests | `admin/core.js` |
| Courier app | every 12 s; GPS row per fix, never pruned | `courier/app.js`, `courier.rs` (fixed) |
| Live venue log | 5830 cells / 10 orders at generation 21 → **583 cells = 4.66 KB per EVENT** | `/api/owner/health` |
| Events per delivered order | 6, each carrying the WHOLE order JSON | `domain.rs:715`, `lib.rs:431` |
| Cell packing | **one payload byte per 8-byte cell** | `bebop-store/src/evlog.rs:146` |
| Record header | 35 cells = 280 B per event (2 obj + 15 record + two 9-cell roots: two commits per append) | `evlog.rs`, `dowiz-hub/src/lib.rs:364` |
| Image on the wire | ALWAYS full capacity (zeros included); doubles; never compacted | `bebop-store/src/lib.rs:103,132` |
| Catalogue image | 67,242 cells = 538 KB for 165 dishes (JSON would be ≈ 70 KB): same 8× | `/api/owner/health` |
| Workers Logs | enabled, unsampled: ≥ 1 event per request | `wrangler.toml [observability]` |
| Notifications | Telegram (free) and WhatsApp (Meta: per message from 2026-10-01) on every order | `notify.rs`, `channels.rs` |
| Nightly backup | base64 of all five FULL-CAPACITY images, a new object every night, never rotated | `cloud.rs`, `hubstore::export` |

Durable Object DURATION is not billed between polls: the object has no sockets, timers or
outbound fetches, so it is hibernatable within 10 s of every response (Cloudflare,
"Lifecycle of a Durable Object"). What binds is REQUEST COUNT (DO 100k/day free, 1M/month
included on paid) and the order log, which is rewritten whole per append and read whole per
poll and grows 28 KB per delivered order — 300 MB a year at 30 orders/day, into a Worker
with 128 MB.

## 1. Estimated savings by phase (the deliverable of this document)

Per venue, 30 orders/day. "$/venue-month" is the marginal cost at overage rates, i.e. what
the NEXT venue costs once the included amounts are used; "free" = venues the free plan
holds; "$5" = venues the paid plan's included amounts hold before usage is billed.

| Phase | requests/day (Worker + DO) | $/venue-month | venues free | venues in $5 | KB per delivered order | hot image after a year | MB per cold visit | KB per poll |
|---|---|---|---|---|---|---|---|---|
| 0 today | 41,880 | 1.73 | 4 | 1.4 | 28 | 299 MB | 2.69 | 4,300 |
| 1 shipped | 33,930 | 1.02 | 5 | 1.6 | 28 | 299 MB | 2.69 | 4,300 |
| 1.5 quick wins | 27,800 | 0.32 | 6 | 2.0 | 28 | 299 MB | 0.5 | 4,300 |
| 2 the object IS the hub | 19,800 | 0.27 | 9 | 3.7 | 28 | 299 MB | 0.5 | 10 |
| 3 deltas + codebook | 19,800 | 0.27 | 9 | 3.7 | 2 | 21 MB | 0.5 | 10 |
| 4 format v2 | 19,800 | 0.26 | 9 | 3.7 | 0.8 | 9 MB | 0.5 | 10 |
| 5 bounded hot log | 19,800 | 0.26 | 9 | 3.7 | 0.8 | ≈ 1 MB | 0.5 | 10 |
| 6 push, not poll | 3,900 | 0.06 | 45 | 20 | 0.8 | ≈ 1 MB | 0.5 | 0 |
| 7 local-first replicas (wild) | 1,100 | 0.03 | 167 | 67 | 0.8 | ≈ 1 MB | 0.5 | 0 |

Two lines the first model missed and this one carries: **Workers Logs** (unsampled, ≥ 1
event per request: at 40 venues ≈ 60M events/month = $25, the largest line on the bill
by then) and **WhatsApp** (Meta bills service and utility messages per message from
2026-10-01, $0.004–0.046 each: two per order at 30 orders/day is $7–80 per venue per
month — up to 40× the whole Cloudflare bill). Neither is a byte problem; both are a
setting.

## 2. Brainstorm — every place a byte or a request hides

Ordered by leverage. ✔ = in a phase below.

**Requests.** Polling is 65 % of all requests (console 2 × every 15 s, courier every 12 s,
tracking every 12 s per live order) ✔6. Adaptive tick: back off to 60 s when the queue is
empty, 5 s while an order is in flight ✔1.5. The dashboard totals every 4th poll ✔1.
`owner_at` runs the membership query twice per owner action (`owner.rs:24,41`) ✔1.5. The
storefront root goes through the Worker only to pick a host — unavoidable, 1 per visit.

**Bytes on the wire to the phone.** Photos: 150 KB each, 18 per first paint, stored as
uploaded. Resize at upload to two sizes (grid 320 px ≈ 12 KB, sheet 1024 px ≈ 45 KB), serve
by `srcset`; 2.69 MB → ≈ 0.5 MB per cold visit ✔1.5. Lazy-load below the fold (present in
`menu.js`; verify it applies to the grid). A menu "atlas": one KV blob with every thumbnail
of the menu, one request, one edge-cache entry — game-dev's sprite sheet — worth it only if
per-photo caching proves insufficient. Fonts, CSS, JS: static, free, cached — leave.

**Bytes between Worker and object.** Whole image both ways on every read and write ✔2.
`to_bytes()` ships the FULL CAPACITY, zeros included: a freshly doubled 8 MB image is 8 MB
of mostly nothing. Persist `1024 + arena_used` cells and re-pad on load from superblock
cell 12 (capacity) ✔1.5 — halves write transfer on average, no format change (the on-disk
`Store::open` path can keep writing full files).

**Bytes in the log.** Six full JSON envelopes per order ✔3; names and descriptions copied
from the catalogue into every line ✔3a; one payload byte per cell ✔4; 280 B of header per
event, two commits per append, a 32-byte actor key that is always zero ✔4; nothing ever
leaves the hot image ✔5. The catalogue and settings images have the same 8× ✔4 (Kv
entries pack the same way).

**Writes to D1.** GPS a row per fix ✔1 (throttle + prune); positions have no business in
D1 at all once a socket exists — they are ephemeral state, kept in the object's memory ✔6.
Every fix a courier sends is 1 auth read + 1 insert; over a socket it is one message.

**CPU.** `orders()` was O(events × orders) ✔1; `order(id)` walks the whole log; every
reader folds from bytes ✔2 (fold once per generation in the object). `serde_json` per event
per poll ✔2/3.

**Logs.** `head_sampling_rate = 0.1` in `[observability]` — one line, and a 10× cut on the
line that dominates at 40 venues ✔1.5. Keep 100 % for `console_error!` by logging errors to
a D1 table the console reads (loud failures stay loud; the memory of "instruments that
measure nothing" applies).

**External services.** WhatsApp → default OFF for status pushes; Telegram (free) and Web
Push (VAPID, free, the storefront is already an installable PWA) carry them; WhatsApp only
inside a customer-initiated window ✔1.5. Nominatim reverse geocoding: free but 1 req/s
policy — cache by 50 m grid cell in KV, or resolve against the venue's delivery polygon on
the device ✔1.5. Nightly S3 bundle: gzip the JSON (a full-capacity image of mostly zeros
compresses ≈ 50×) and rotate (7 daily + 4 weekly) ✔1.5 — the venue's bucket, the venue's
bill, but it grows unbounded today.

## 3. Ideas from game networking (the operator asked; they fit unusually well)

A delivery hub is a small multiplayer game: a few entities, positions, state transitions,
many observers, unreliable phones. Game netcode solved these constraints twenty years ago.

| Game technique | What it becomes here | Phase |
|---|---|---|
| Snapshot + delta compression (Quake 3: send the diff against the last acked state) | The object keeps per-client "last seen generation"; a poll or push carries only events since it | 2, 6 |
| Dead reckoning (send velocity, extrapolate; correct only when the error exceeds a threshold) | Courier sends a fix only when the straight-line prediction is off by > 20 m; the map extrapolates between fixes | 1.5 (client), 6 |
| Quantisation + bit-packing (Fiedler, "Snapshot Compression": 10× typical) | Positions to 1 m inside the delivery zone (2 × 15 bits), status/time/courier in one cell | 4 |
| Tick rate scaling | Poll every 60 s when idle, 5 s in flight; push makes the tick zero | 1.5, 6 |
| Interest management / area of interest | A client receives only its topic: the customer its order, the courier its run, the console the queue | 2, 6 |
| Level of detail | Three projections, not one: tracking sheet (status + ETA), courier (assigned + open), console (full); archived orders at "far LOD" — a count and a link | 2, 5 |
| Mipmaps | Grid thumbnails vs dish-sheet photos | 1.5 |
| Texture atlas | One menu thumbnail blob | optional |
| Occlusion culling | Fold and send only live orders; closed ones are not in the hot path | 5 |
| Lockstep / deterministic replay from inputs | The kernel already replays identically everywhere: clients can fold the log themselves | 7 |
| Client-side prediction + reconciliation | The console shows a status change at once and reconciles with the object's answer | 2 |
| Baked lighting (precompute at build) | ETA profile per zone computed nightly, not per request | 1.5 |
| Entity-component / structure-of-arrays | Columnar fold in the object: one array per field, cache-friendly and compressible | 2 |

## 4. Three wild ideas (they look impossible; they are the direction)

**W1 — Zero-byte orders: the cart is a tensor over the catalogue.** A cart is a sparse
vector over the venue's 165 dishes: `(index: 8 bits, qty: 4 bits, modifiers: 8 bits)` per
line, the address an index into the customer's saved addresses, the price snapshot the
catalogue's version number (prices are re-derived by the deterministic kernel, exactly, as
`storefront.rs:762` already does). A `Placed` event becomes ≈ 3 cells of payload; with the
v2 header, a delivered order ≈ 15 cells = 120 bytes. Two hundred times smaller than today.
"Impossible" because the receipt must survive a menu change — it does: the catalogue is
itself a versioned image, and the version is in the event.

**W2 — The phone is the hub, Cloudflare is the mirror.** "bebop IS dowiz's database" and
the kernel is local-first by decision D0. The console device in the kitchen holds the
authoritative hot log (≈ 1 MB after phase 5); the object is a relay and a backup that
accepts appends while the phone is away and hands them over when it returns. Devices
exchange hashed heads, not images; the `actor_pubkey` slot every caller leaves zero becomes
real (each device signs its appends). Cloudflare usage per venue collapses to relay
messages; the platform costs $5 flat however many venues. "Impossible" because phones go
offline — which is exactly what an append-only log with a mirror tolerates.

**W3 — The customer never talks to the server about their order.** After placement the
tracking sheet subscribes to the venue's peer set over WebRTC (bebop2's mesh, in the browser)
and folds status events locally; the object is contacted only when no peer answers.
Polling disappears not because it got cheaper but because there is nothing to poll. This is
W2 seen from the customer's side and is the actual end state of phase 7.

## 5. The blueprints

Each phase: scope, files, the test that is RED before and GREEN after, the live number
that proves it, effort (*estimate*), risk. Order is by leverage ÷ risk.

### Phase 1 — shipped (af5b0fc)

DO writes only changed chunks (`hubdo.rs changed_chunks`, 5 tests: an append = chunk 0 +
tail); `attach_one` reuses the loaded hub; `/media` behind `caches.default`; `orders()`
HashSet; `StockLog::write` numbers from the root counter; `with_hub`/`with_stock` skip the
save when `len()` did not move (not the generation: `grow()` restarts it); GPS one fix per
10 s / 20 m and a nightly prune > 48 h; dashboard totals every 4th poll. Proof: dowiz-hub
261 tests, workers/api 35 (5 new), wasm check, design gate; `/media` `cf-cache-status: HIT`
on the second fetch.

### Phase 1.5 — quick wins (effort ½ day, risk low)

Status 2026-09-20 night: items 1 (SAMPLING ONLY), 3 and 4 are shipped; the rest stand.

1. PART SHIPPED (ccd9298). `[observability] head_sampling_rate = 0.1` is live. The other
   half is NOT: errors are still only in the sampled trace, and nothing writes a
   `worker_errors` D1 row (kept 7 days) that the console can read. Test when it lands: a
   probe error appears in the table.
2. Photos: resize at upload with the `image` crate (already wasm-safe) to 320 px and
   1024 px JPEG q72, both content-addressed; `srcset` in `menu.js` and the dish sheet.
   `menu.js` ALREADY emits the `srcset` (ccd9298) and it is inert until the upload path
   makes the small file — that is the whole of what is left.
   Test: an upload yields two keys; the grid requests the small one (Playwright: bytes per
   cold visit < 0.6 MB).
3. SHIPPED (ccd9298). Adaptive polling: console 15 s live / 60 s idle, courier 12 s on
   shift / 60 s off, tracking 12 s moving / 30 s before. An idle venue with an open console
   falls from 480 requests an hour to 120.
4. SHIPPED (this commit). `to_bytes_trimmed()` ships the cells the arena has actually
   used; `from_bytes` re-pads to capacity from superblock cell 12, refusing to pad an image
   cut BELOW `arena_used` (truncated, not trimmed) or one whose capacity cell claims more
   than `MAX_PAD_CELLS` (512 MiB — a restore must not choose the reader's allocation).
   The Kv images trim the same way inside `compacted_bytes`.
   MEASURED, and smaller than this document estimated: a fresh 4 MiB hub is 1,072 bytes
   instead of 4,194,304; ten delivered orders (60 events) in a hub born at 64 KiB are
   165,664 bytes instead of 262,144 — **63 %**, not the "half on average" guessed above.
   The saving is a sawtooth, because what is dropped is the slack left by the last
   doubling: ≈ 50 % the moment an image doubles, ≈ 0 % just before the next one. It is the
   FRESH venue and the doubling that this wins, not the busy one. Tests: `bebop-store`
   `the_trim_lands_on_the_arena_cursor`, `a_truncated_image_is_left_as_it_arrived`,
   `an_absurd_capacity_is_not_padded_to`; `dowiz-hub` `a_trimmed_log_reloads_into_the_same_hub`,
   `a_trimmed_log_keeps_appending`, `a_days_orders_on_the_wire_full_against_trimmed`.
5. `owner_at` → `owner_and_venue` (one membership read).
6. Nightly bundle gzipped and rotated (7 daily + 4 weekly). Test: object size of a fresh
   venue < 20 KB. (The trim in item 4 already shrinks the bundle: it base64s whatever the
   object stores, which is now the trimmed image.)
7. WhatsApp status pushes default off; Web Push (VAPID keys as Worker secrets, subscription
   stored on the customer row) for the tracking sheet; Telegram stays. Test: a placed
   order produces a push, no Meta call unless `notify.whatsapp.status = on`.
8. Nominatim results cached in KV by 50 m cell for 30 days.

### Phase 2 — the object IS the hub — SHIPPED

The Durable Object folds its own log and answers with ORDERS; the Worker sends EVENTS.
The image crosses the hop only where something genuinely needs the bytes.

- `hubdo.rs` gained a second kind of route. `/img/...` still hands over bytes — the
  catalogue, the settings, a backup, anything whose reader is not this object.
  `/fold/...` hands over answers: `GET /fold/orders` (every order, folded, newest first),
  `GET /fold/order?id=` (one), `GET /fold/generation` (the guard, without the list),
  `POST /fold/append` (one event, under the same `x-generation` guard a whole-image
  write carried). The projection is MEMOISED by generation and dropped on every write
  to the log, so a venue whose consoles, couriers and customers all poll folds its log
  once per change rather than once per request.
- `hubstore.rs` gained the Worker's half: `orders`, `order`, `log_generation`,
  `append_for` (read the order, decide, append, retry on 409 — the contract `with_hub`
  had) and `append_blind` (a placement or an audit record, which depend on nothing the
  log already says).
- Converted: every list reader (owner queue and dashboard, courier tasks and wallet,
  eta quote, analytics, customers, promo check, courier history, both assistants),
  every single-order reader (`/api/order/:id`, courier `load_order`, owner assign) and
  every writer but one (`lib.rs` place and advance, owner advance and assign, courier
  advance and claim, stripe paid, feedback, reveal).
- STILL TAKES THE IMAGE, and each for a reason written at the call site: the storefront's
  PLACEMENT, because a promotion's last use must be counted and spent in one guarded
  breath; the REVEALS audit route, because `reveals()` reads the events the fold skips;
  the owner ASSISTANT, because `graph_facts` walks the hub itself; and health, export,
  import and the catalogue writers, which are about bytes by definition.
- Found while converting: the dashboard fetched the CATALOGUE beside the log "because
  the readiness count needs it" and then never touched it. Every console poll paid for
  a second image to satisfy a comment.
- Tests: `hubdo.rs` 1 (the projection's JSON shape is a contract — `kind` travels as the
  byte the log stores, and must name the same kind on the other side), on top of the
  four `hubstore` fold tests that phase 3 added and that the object now runs inside
  itself. The object's own plumbing is not natively testable; what it does is
  `hubstore::orders_state`, which is.

### Phase 3 — deltas — SHIPPED

An event carries WHAT CHANGED. The fold lives in `workers/api/src/fold.rs`, not in
`dowiz-hub`: that crate's `minijson` says plainly that it is not a general JSON parser
and is never pointed at untrusted documents, and an order envelope holds a customer's
own words. `dowiz-hub` hands out events (`history`, `events_oldest_first`, both
order-kind only); the Worker folds them with `serde_json`.

- A delta is MARKED (`"_d": true`); anything unmarked is a snapshot that REPLACES the
  state. So every image written before this change folds to exactly what `Hub::order`
  used to return — the newest envelope — and there is no migration and no moment where
  the two disagree. `a_history_of_snapshots_folds_to_the_newest_snapshot` is that
  guarantee as a test.
- The delta is DERIVED (`fold::delta(old, new)`), not declared by each caller. A list of
  "fields this transition changes" drifts from what the kernel actually returned, and the
  first thing it drops is the field somebody added last week. A key the new state lost
  becomes an explicit `null`, so a delta can delete as well as add.
- Writers: `lib.rs` advance, `owner.rs` advance and assign, `courier.rs` advance and
  claim, `stripe.rs` paid. Readers: `hubstore::order_state` / `orders_state`, which
  replaced every `hub.order()` / `hub.orders()` call in the Worker (22 sites).
  `orders_state` folds the whole log in ONE pass — folding per order would restore the
  O(events × orders) shape phase 1 removed.
- MEASURED: three orders, twelve events — 2,619 cells of envelopes against 1,545 of
  deltas, **58 %**. Not the 90 % the payloads alone suggest, because what is left is the
  RECORD HEADER: 15 cells plus two object headers and a nine-cell root per commit, about
  26 cells whatever the payload says, and then one payload byte per eight-byte cell.
  That header is phase 4's business.
- Tests: `fold.rs` 9 (the snapshot/delta equivalence, deletion, nested merge, arrays
  replaced whole, a damaged payload skipped, the marker never reaching a consumer),
  `hubstore.rs` 4 (the same equivalence through `orders_state`, a `Noted` assignment
  reaching the fold, an audit record being neither an order nor part of one).

**NOT DONE, and deliberately: the line codebook.** The plan had lines carry ids only,
with names and photos re-derived from the catalogue at render. With deltas the name is
written ONCE per order rather than six times, so what is left to save is about thirty
bytes a line — while the cost is that a receipt stops being self-contained: rename a
dish and last month's order re-renders under the new name, because this catalogue is not
versioned per order. That trade is bad at this size. Revisit only if a venue's catalogue
becomes versioned.

### Phase 4 — EvLog v2 — SHIPPED

A written format changed, so every reader moved with it. There is exactly one reader of
this layout — `crates/bebop-store/src/evlog.rs` — which was checked rather than assumed:
nothing in `bebop-lang/`, no oracle script and no `.bp` file names EVLOG, and `dowiz-hub`,
the kernel's `bebop_event_store` and the Worker all go through the `EvLog` API.

- **v2 record**: `12 + (4 if the actor is named) + ceil(P/8)` cells, against v1's
  `15 + P`. The payload is packed EIGHT BYTES TO A CELL; the 32-byte `actor_pubkey` that
  every dowiz caller leaves zero is behind a flag bit and costs nothing when absent.
- **v2 root**: 8 cells, the eighth being the VERSION. A root with no version cell is v1,
  which is what every image in production looks like. `walk`, `read_at` and `append`
  dispatch on it, so a v1 log keeps being read and appended to as v1 — a mixed chain is
  never created.
- **The migration is a replay.** `Hub::grow` and `StockLog::grow` init a FRESH store and
  copy the chain into it, so an old image becomes v2 the next time it doubles. Nothing is
  rewritten in place and no migration step runs anywhere.
- **The tip travels with the record.** `append_tip_bytes` allocates the record and the new
  root in ONE transaction. That removes a whole root per event AND removes a defect class
  by construction: "the record fitted but the tip update did not" (the order log refused
  order 2450 with 56 cells free) cannot happen when both are allocated in one tx — either
  everything is committed or the arena cursor has not moved, so a retry after growing
  cannot double-count.
- **The content id now commits to the previous one.** `stock.rs` said "editing any event
  changes every content id after it" and the code hashed the payload alone, so it did not.
  `content_id_chained(prev, payload)` makes it true, and `Hub::chain_check()` is the walk
  that checks it: `chained`, `legacy` (written before the cascade — every image in
  production, and not an alarm) and `broken`.

**MEASURED.**

| | v1 | v2 |
|---|---|---|
| a 330-byte event | 356 cells | 70 cells |
| an append with its tip (21-byte payload, named actor) | 47 + 9 cells | 31 cells |
| ten delivered orders, full envelopes, on the wire | 165,664 B | **38,032 B** |
| 20,001 order events | 67,108,864 B image | **16,777,216 B** |

And the interaction with phase 3, which moved: deltas were 41 % off the v1 envelope and
are 24 % off the v2 one. The deltas did not get worse — the baseline got better, because
v2 stopped the payload being what an event costs. Together: 2,619 cells → 444, **83 %**,
and neither change reaches that alone.

**Tests.** `bebop-store` 31 (6 new: a v1 log reads and appends as v1 at exactly its old
cost; a replay into a fresh store is v2 and byte-identical in content; payloads survive at
0, 1, 7, 8, 9, 63, 64, 65 and 330 bytes; an unnamed actor takes no cells; the tip in one
commit costs one root less; the version survives the byte round trip). `dowiz-hub` 259 + 7
+ 2 (3 new: the cascade verifies, an edited byte in the image is found, a pre-cascade log
reads as legacy rather than broken). `workers/api` 56. Kernel `bebopdb` 4.

**NOT DONE: the Kv packing.** The catalogue's 538 KB has the same 8× amplification, and
the same fix would work — but that layout is created and read by
`bebop-lang/selfhost/std/kv.bp` as well as by Rust, and this repo's rule 10 says a written
format is not changed until every reader moves in the SAME commit. That is a bebop-lang
change on a different toolchain; it is phase 4b and it is not this.

### Phase 5 — a bounded hot log with cold history — SHIPPED

The hot log holds what is live and what is recent; everything else becomes its own image.

- `EventKind::Checkpoint = 6`, and it is VISIBLE: `decode` used to drop an unknown kind
  without a sound, so a mark in the log would have been a mark nobody could see. A test
  asserts `events()` returns it, that `is_order()` is false for it, and that it never
  appears in `orders()`.
- `Hub::rotate(keep)` returns the image as it stood — for the caller to store cold — and
  leaves a fresh one holding a CHECKPOINT (`tip=<hex> events=<n> bytes=<n>`) and every
  event of every order `keep` kept. `keep` is asked once per ORDER, never per event: an
  order half of whose events survived would fold to a lie.
- THE RECORDS MOVE VERBATIM — same ids, same `prev`, same payloads — so `chain_check`
  still verifies every one of them. An id commits to the id before it, and that id is a
  VALUE in the record rather than a pointer into the image, so a gap in the walk is what
  the checkpoint announces rather than damage.
- Worker: `hubstore::rotate` keeps an order if its status is live (PENDING … IN_DELIVERY,
  whatever its age — a PENDING order forty days old is a problem, and archiving it would
  be hiding one) or its last event is within `HOT_KEEP_MS` (30 days). The archive is
  written FIRST, under `log@<generation>`, and the hot image second: if the second write
  fails the venue has one extra copy of its history rather than none of it, and a retry
  lands on the same archive id, which the object refuses to overwrite.
- Reachable, or it would be deletion with extra steps: `GET /api/owner/history` lists the
  archives, `?archive=log@<n>` folds one, `POST /api/owner/hub/rotate` runs it now, and
  the nightly cron runs it for every venue before the backup.
- The nightly bundle carries each archive ONCE (`archives_pending` / `archives_marked`,
  marked only after the PUT lands), and `import` restores archives beside the five fixed
  images — a restore that dropped them would put a venue back with its live orders and no
  history.
- `is_archive_id` is a checked name, not a trusted one: the id arrives in a query
  parameter and reaches the object's storage, so anything but `log@<digits>` is refused.
  Fourteen refusals in the test, including `log@1/../settings` and `m:log@1`.

**Tests.** `dowiz-hub` 264 lib (5 new: a kept order is untouched and a moved one is in the
archive; the checkpoint is an event the log returns and not an order; a second rotation
chains and the hot image keeps exactly one mark; a rotated log still verifies on both
sides of the cut; every event of a kept order travels with it). `workers/api` 57 (1 new:
the archive-name check). MEASURED in the rotation test: 198 arena cells hot against 436
archived.

### Phase 6 — push, not poll (effort 3 days, risk medium)

- The object accepts WebSockets with the Hibernation API (`state.accept_web_socket`, in
  workers-rs 0.8.5), tagged by topic (`console`, `courier:<id>`, `order:<id>`); it
  hibernates between messages, so duration stays unbilled; each incoming message is one
  request, outgoing broadcasts are not.
- On every append the object broadcasts the delta to the interested topics (interest
  management); clients apply it (client-side prediction already renders the optimistic
  state).
- Courier GPS goes over the socket into object memory (dead reckoning on the client: send
  only when the prediction is off by > 20 m); `live_eta::fixes` reads the object;
  `courier_positions` in D1 is retired (kept 48 h for the audit log only).
- Fallback: the existing polling endpoints remain for clients without a socket.
- Tests: a fixture object with two sockets receives one broadcast per append; a
  hibernated object wakes on a message and answers from the memo.
- Live proof: requests/day for an idle open venue < 500 (today ≈ 10,000).

### Phase 7 — local-first replicas (the wild end; effort weeks; risk research)

W2 + W3: devices hold the hot log, sign their appends with the actor key, exchange heads
over bebop2 (the PQ mesh, `mesh-adapter/`), and treat the object as relay + mirror. Depends
on phase 4's real hash chain and phase 5's bounded hot image. Deliverable of the first
step: the console folds the log it receives over the socket (phase 6) and survives a
15-minute outage without a request. Everything after that is the manifesto.

## 6. What is deliberately NOT done

Poll rates are not simply lowered below what liveness needs: phase 6 removes the poll.
Hypervectors are not used for the log (lossy; money is exact by D0) — they belong to
retrieval and taste. Encrypting the order does not save bytes; encrypting the PERSON
(address, phone, to the venue's key) is a privacy feature to design into phase 3's payload,
size-neutral.
