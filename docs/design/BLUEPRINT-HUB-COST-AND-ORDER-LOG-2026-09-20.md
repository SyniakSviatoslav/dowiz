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

### Phase 2 — the object IS the hub (effort 2 days, risk medium)

The Durable Object owns a live `Hub` (and `StockLog`) in memory; the Worker sends
COMMANDS and reads PROJECTIONS; the image never crosses the Worker↔object hop again.

- `hubdo.rs`: `POST /append {kind, order_id, payload}` → `Hub::append` + persist (chunk
  diff) + bump memo; `GET /orders?lod=console|courier|track&since=<generation>` → the
  fold, cached per generation, filtered per LOD; `GET /order/:id`; `GET /catalog` stays
  bytes for `with_catalog` writers but `GET /catalog/product/:id` and `/catalog/location`
  serve JSON from a memo. Snapshot+delta: `since` returns only events after that
  generation, with the new generation in a header.
- `hubstore.rs`: `with_hub` becomes `append(place, ev)`; readers call the projection.
  `load()` remains for export/import/health.
- Readers: `owner.rs orders/dashboard`, `courier.rs tasks`, `lib.rs /api/order/:id`,
  `eta.rs quote`, `live_eta.rs` — each drops its `load()`.
- Tests (workers/api, native): the projection of a fixture image equals
  `Hub::load(bytes).orders()`; `since` returns exactly the appended events; a generation
  bump invalidates the memo; LOD `track` never contains another customer's address.
- Live proof: `/api/owner/health` gains `objectReads`, `bytesShipped`, `chunksWritten`
  counters kept in the object; a console hour ships < 1 MB where it shipped 400 MB.

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

### Phase 4 — EvLog v2 in bebop-store (effort 3 days, risk HIGH: a written format)


- Record: 8 payload bytes per cell; the tip written in the same commit as the record (one
  root); `actor_pubkey` present only when non-zero (a flag bit in cell 0); `prev` and the
  content id kept; the content id NOW hashes `prev` too, so the chain is tamper-evident by
  cascade (today `stock.rs:723` claims it and `lib.rs:488` does not deliver it).
- Version cell in the EVLOG root; `walk`/`read_at` dispatch on it; `grow()` migrates v1 →
  v2 on the next doubling; `Kv` entries pack the same way (catalogue 538 KB → ≈ 70 KB).
- Repo rules: the ORACLE for `append_is_constant_cost` (`evlog.rs:321`, golden 47) is
  re-derived first; the bebop-lang reader of this format (`bebop-lang/`, `bpref`) moves in
  the same commit, per `bebop-lang/AGENTS.md` ("every reader, oracle, golden and harness
  model in the SAME commit"). `arch_check` and `invariants.sh` stay green.
- Tests: v1 fixture image loads, folds identically, and after one append is v2; cells per
  event = 12 + ⌈P/8⌉; the 3,000-order regression in `tests/log_growth.rs` runs in a fraction
  of the image.

### Phase 5 — a bounded hot log with cold history (effort 2 days, risk medium)

- `EventKind::Checkpoint = 6` with a LOUD test that `events()` returns it (today unknown
  kinds are skipped silently, `lib.rs:471`).
- `Hub::rotate(keep)` next to `grow()`: replay only events of orders that are open or
  closed within 30 days, preceded by a `Checkpoint` naming the archived image's tip and
  generation. Object stores the outgoing image under `log@<generation>` (never on the hot
  path; served by `/api/owner/history?before=`); the nightly S3 bundle carries it too.
- Trigger: `usage().used_cells` above a threshold on a write, or the nightly cron.
- Tests: every open order folds identically before and after; a closed-old order is in the
  archive and not in `orders()`; a second rotation chains checkpoints.

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
