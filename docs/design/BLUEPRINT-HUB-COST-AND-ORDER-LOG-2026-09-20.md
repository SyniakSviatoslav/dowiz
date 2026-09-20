# Blueprint: what one hub costs, and the order log that makes it cost more every day

Date: 2026-09-20. Status: phase 1 SHIPPED (this commit), phases 2–5 DESIGNED, numbers
measured unless marked *estimate*. Calculator: https://claude.ai/artifact/XLTpcL2RXZ1z52yeJC1UnY

## 0. The measured state

| Fact | Value | Source |
|---|---|---|
| Cloudflare plan | Workers Free, zone Free | `wrangler.toml` (error 10097 forced `new_sqlite_classes`); zone API `plan.name` |
| One cold storefront visit | 40 responses: 20 static (free), 1 root, 1 API, **18 `/media` photos** | Playwright on `sushi-durres.dowiz.org` |
| One `/media` photo | 2 KV reads, no edge cache (`cache.put` absent) | `extra.rs` media handler |
| One delivered order | D1 12 reads + 5 writes; DO 12 GET + **8 PUT of the whole image** | code walk, `hubstore::with_hub` |
| Status poll `/api/order/:id` | every 12 s; loaded the log **twice** + catalogue | `store/track.js`, `live_eta.rs:299` |
| Owner console | every 15 s, 2 requests (orders + dashboard) | `admin/core.js POLL_MS` |
| Courier app | every 12 s; GPS `watchPosition` POSTs a D1 row per fix, never pruned | `courier/app.js`, `courier.rs` |
| Live venue log | 5830 cells for 10 orders at generation 21 → **583 cells = 4.66 KB per EVENT** | `/api/owner/health` |
| Events per delivered order | 6 (Placed + 5 Advanced), each carrying the WHOLE order JSON | `domain.rs:715`, `lib.rs:431` |
| Cell packing | **one payload byte per 8-byte cell** | `bebop-store/src/evlog.rs:146` |
| Image on the wire | always full capacity; doubles when full; never compacted | `bebop-store/src/lib.rs:103`, `dowiz-hub/src/lib.rs:399` |

So a delivered order writes ≈ 28 KB into an image that is then rewritten whole eight
times and transferred whole on every poll. At 30 orders/day the hot image passes 25 MB in
about a month. That is the constraint. Money is not: one venue is $5.93/month all-in
(the $5 subscription is per account), ten venues are $1.88 each, and a delivery is
≈ 0.01 lek. Durable Object DURATION is not billed between polls (the object has no
sockets, timers or outbound fetches, so it is hibernatable within 10 s of every
response); the first free-plan limit is DO REQUESTS at ~4 venues.

## 1. Phase 1 — shipped in this commit

| Change | File | Effect |
|---|---|---|
| DO writes only the chunks whose bytes moved | `hubdo.rs` `changed_chunks` + 5 unit tests | an append = chunk 0 + tail chunk: **2 row writes instead of ⌈image/96 KiB⌉** (43 at 4 MB) |
| `attach_one` takes the already-loaded hub | `live_eta.rs`, `lib.rs` | status poll reads the log **once**, not twice |
| `/media` behind `caches.default` | `extra.rs` | photos cost KV reads once per POP, not per device |
| `orders()` dedups with a set | `dowiz-hub/src/lib.rs` | fold O(events) instead of O(events × orders) |
| `StockLog::write` numbers from the root counter | `stock.rs` | no full walk per delivery |
| `with_hub`/`with_stock` skip the save when nothing was appended | `hubstore.rs` | a webhook replay no longer rewrites the image |
| GPS: one fix per 10 s or 20 m; positions older than 48 h pruned nightly | `courier/app.js`, `cloud.rs` | D1 writes ÷ ~3, table bounded |
| Dashboard totals every 4th poll | `admin/app.js` | console requests −40 % |

Verification: `cd crates/dowiz-hub && cargo test`, `cd workers/api && cargo test --lib`
(35, five of them new), `cargo check --target wasm32-unknown-unknown`, design gate GREEN,
`/media` answers `cf-cache-status: HIT` on the second fetch from one POP.

## 2. Phase 2 — the object answers with orders, not with bytes

Today every reader (console, courier, tracking sheet, ETA) fetches the whole image from
the DO and folds it in the Worker: O(image) transfer and O(events) CPU per poll, growing
with the venue's life. The DO already keeps the image in `self.mem`.

Design: the DO folds once per generation and caches the projection.
`GET /img/log` stays for writers. New: `GET /orders` → the `orders()` fold as JSON;
`GET /order/:id`; both served from a `(generation, Vec<Event>)` memo invalidated by
`put_image`. Readers in `owner.rs`, `courier.rs`, `lib.rs` `/api/order/:id`, `eta.rs`,
`live_eta.rs` switch to it; `with_hub` keeps the image path. A poll becomes one small
JSON hop, constant in the venue's history. Tests: the memo returns the same list as
`Hub::load(bytes).orders()` for a fixture image; generation bump drops the memo.

Effect: DO requests per poll 2–3 → 1; Worker CPU per poll O(events) → O(orders); the
Worker never holds a 4 MB image on a read path.

## 3. Phase 3 — `Advanced` carries the change, not the order

Every transition stores the whole envelope again: 6 × the order's JSON per delivered
order. Change inside `dowiz-hub` only: the fold merges events oldest → newest onto the
`Placed` envelope (`serde_json` object merge); `Advanced` callers then pass a delta
(`status`, the stamped timestamps, `courier_id`). Old events with full JSON merge
identically, so the change is backward compatible with every image in production.
Tests: an order folded from a full-JSON history equals one folded from deltas; the
`cap.rs` history walk shrinks by ≈ 5× for the same orders.

Effect: bytes per delivered order ≈ 28 KB → ≈ 6 KB.

## 4. Phase 4 — EvLog v2: eight payload bytes per cell (bebop-store format)

`evlog.rs:146` writes one payload byte per cell and `read_at` mirrors it; the image is
8× the data. A v2 record packs 8 bytes per cell (little-endian), flagged by a version
cell in the EVLOG root so a reader tells v1 from v2 and `grow()` migrates a v1 image
into a v2 arena on its next doubling. Rules from the repo: re-derive
`append_is_constant_cost` (`evlog.rs:321`, today `47`) with an ORACLE before touching
the golden; the bebop-lang side that reads this format moves in the same commit
(see `bebop-lang/AGENTS.md`, "a change to a written format is not finished until every
reader, oracle, golden and harness model is re-derived in the SAME commit").

Effect: image ÷ ~8 for the same events; with phase 3, a delivered order ≈ 0.8 KB.

## 5. Phase 5 — a bounded hot log, with the history kept cold

No primitive exists for compaction, snapshot or truncation of an append log; `grow()`
is a verbatim replay into a fresh arena and is the machinery to reuse. `prev` is not
verified anywhere and content ids do not chain (`lib.rs:488`), so dropping head records
is mechanically safe; `EventKind::from_byte` silently skips unknown kinds, so a new
`Checkpoint` kind must be added with a loud test rather than relied on.

Design (inside `dowiz-hub`, next to `grow`): `Hub::rotate(keep: |order| bool)` replays
into a fresh image only the events of orders that are open or closed within 30 days,
preceded by one `Checkpoint` record naming the archived image's tip. The DO stores the
outgoing image under `log@<generation>` (never read on the hot path); the nightly S3
bundle already carries the full image. Trigger: `usage().used_cells` above a threshold
on a write, or the nightly cron. Tests: every open order folds identically before and
after; a closed-old order is absent from `orders()` and present in the archive;
`Checkpoint` survives `events()` rather than being skipped.

Effect: the hot image is bounded by activity, not by lifetime.

## 6. Expected numbers (30 orders/day, estimate from the measurements above)

| | today | phase 1 | + 2 | + 3 | + 4 | + 5 |
|---|---|---|---|---|---|---|
| DO row writes per order | 8 × ⌈img/96K⌉ (≈ 360 at 4 MB) | 16 | 16 | 16 | 16 | 16 |
| bytes per poll | whole image | whole image | ≈ orders JSON | same | same | same |
| bytes per delivered order | 28 KB | 28 KB | 28 KB | 6 KB | 0.8 KB | 0.8 KB |
| hot image after a year | 300 MB | 300 MB | 300 MB | 66 MB | 8 MB | ≈ 1 MB |

## 7. What is deliberately NOT done

Polling is not replaced by WebSockets: the object already hibernates between polls, so
duration is not the cost, requests are, and phase 2 makes a poll one small hop. Lower
poll rates trade liveness the console and the tracking sheet exist for.
