# Architecture evolution: where dowiz stands against the four post-microservice directions, and what to do about it

**Date:** 2026-09-22. **HEAD read:** `c5c640cc` ("extra.rs is gone", 2026-09-22).
**Tree state at time of reading:** 15 files modified and uncommitted (`git status --short | wc -l` = 15;
`git diff --stat` = 74 insertions, 135 deletions). Among them `workers/api/src/hubstore.rs` (+4/−14) and
`workers/api/src/storefront.rs` (+22/−… lines). Line numbers cited below for those two files are from the
WORKING TREE, not HEAD, and may drift by a few lines once that work lands.

**Method.** Every "today" statement names a path, and a line number where one was read. Every number says
how it was obtained: `measured` means a command was run on this box and its output quoted; `hypothesis`
means it was not. Where a design doc and the tree disagree, §1.8 names the file. Nothing here was taken
from a document about the code when the code itself was readable.

The brief translated: the operator names four convergent directions after "microservice hell" (network
latency, sagas, desynchronisation) — (1) local-first with CRDTs, (2) a stateful edge actor mesh, (3)
effect-driven/algebraic architecture, (4) an edge-compiled distributed monolith in WASM — asks where dowiz
stands on each plus event-driven and data-driven, and asks directly whether pure computation is separated
from port effects. §4 answers the last question with evidence; §2 scores the six; §3 is the plan with the
items recommended AGAINST marked as such.

---

## 1. Ground truth

### 1.1 Runtime shape

One Cloudflare Worker, one Rust crate (`workers/api/`, 18,582 lines of top-level `src/*.rs` by `wc -l`,
measured), compiled to one WASM module (`wrangler.toml:1-3` — `main = "build/worker/shim.mjs"`,
`worker-build --release`). The router is a single function, `workers/api/src/lib.rs:209-426` (`route`),
carrying **119 route registrations** (measured: `grep -c '\.get_async\|\.post_async\|\.get(' lib.rs`).
The module header states the contract: "transport and storage ONLY. Every order decision is made by
`dowiz_kernel::json_api`" (`lib.rs:1-4`).

Below the Worker, three standalone crates (no workspace — `CLAUDE.md`, "Build model"):

| Crate | Role | Purity claim in its own header | Verified? |
|---|---|---|---|
| `crates/dowiz-core/` | the FSM and money authority; `#![no_std]`, no non-optional deps (`Cargo.toml:6,13`; `serde` is optional behind `json-api`, which the Worker's `dowiz-kernel/json-api` feature turns on — serialisation, not I/O) | "No float, no I/O" (`src/order_machine.rs:11`) | `place_order_at(id, customer_id, items_json, created_at_ms, channel)` takes identity and time as ARGUMENTS (`src/json_api.rs:238-244`); `apply_event_logic(order_json, next_status)` reads no clock (`src/json_api.rs:278-284`, grep for `now`/`clock` in its body: none) |
| `kernel/` (`dowiz-kernel`) | facade; re-exports `dowiz_core::*` (`kernel/src/lib.rs:22-73`); `json-api` feature is what the Worker links (`workers/api/Cargo.toml:26`) | pure-std, serde-free by default (`kernel/Cargo.toml:19-31`) | `grep -rln 'SystemTime\|Instant::now\|rand::\|std::net' kernel/src` → 19 of 146 files, none of them on the order/money path (measured; the hits are `clock.rs`, `ct_gate.rs`, `openobserve.rs`, `living_knowledge.rs`, bins) |
| `crates/dowiz-hub/` | "One tenant's hub: the order log, over a bebop store, as PURE logic. Bytes in, bytes out. No filesystem, no network, no clock, no randomness" (`src/lib.rs:1-4`); depends only on `bebop-store` and `sha2` (`Cargo.toml:12-19`) | **317 + 7 + 1 + 1 native tests green, 3.55 s for the lib suite** (measured: `cd crates/dowiz-hub && cargo test --offline`, exit 0) |
| `crates/bebop-store/` | the pointer-free image format, "ZERO dependencies: std only" (`Cargo.toml:5-11`); `evlog.rs` is the chained append-only log (971 lines), `kv.rs` the sorted key layout | in CI (`.github/workflows/ci.yml:24`) |

**A second implementation of the same surface exists.** `tools/native-spa-server/` is 10,491 lines
(measured `wc -l src/*.rs`) with **73 `"/api/` route strings** across `api.rs`, `hub*.rs` (measured by
`grep -c`), its own `hub.rs` where "ONE process is ONE writer … a mutex around the store is the whole of
that problem here" (`src/hub.rs:8-11`), over the same `dowiz_hub` + `dowiz_kernel::json_api`
(`src/hub.rs:28-36`). **It is not in CI**: `grep -n native-spa-server .github/workflows/ci.yml` returns
nothing (measured), while `bebop-store`, `dowiz-hub`, `workers/api` and `apps/courier` are (`ci.yml:12,24-27`).

**`CLAUDE.md` cites a file that does not exist.** "`kernel/src/order_machine.rs`" (CLAUDE.md, "Kernel
authority model") — `ls kernel/src/order_machine.rs` → "No such file or directory" (measured). The FSM
is `crates/dowiz-core/src/order_machine.rs` (1,404 lines).

### 1.2 How state is stored

- **One Durable Object class, `HubImages`** (`wrangler.toml` `[[durable_objects.bindings]] name = "HUB"`,
  `class_name = "HubImages"`; `workers/api/src/hubdo.rs:191-210`). One object per venue,
  `id_from_name(location_id)` (`hubdo.rs:22-25`); one more object named `__platform` for people, venues,
  hosts (`workers/api/src/platform_store.rs:1-25`).
- **Images, not rows.** Each object stores named byte images in 96 KiB chunks under `c:<id>:<n>` with a
  `Meta { generation, chunks, len }` under `m:<id>` written LAST (`hubdo.rs:31-33, 44, 61-66, 356-360,
  618-630`). Venue images: `log`, `catalog`, `settings`, `posts`, `stock` (`hubstore.rs:31-46`), `ops`,
  `people`, `i18n` (`hubstore.rs:~975-1000`), `idem` (`idempotency.rs:40`). A missing chunk or a
  length mismatch is refused, not truncated (`hubdo.rs:396-411`), after commit `4e585256`.
- **Three storage shapes over one format:** `dowiz_hub::Hub` (the order EvLog; fold = an order's life),
  `dowiz_hub::logimage::LogImage` (append-only for things that are not orders — errors, threads,
  postings; `logimage.rs:1-16`), `dowiz_hub::table::Table` (records + hand-written index keys in one
  Kv image, index entries written in the same operation as the record; `table.rs:1-36`). The Table
  header states the cost honestly: "rewritten eagerly on every put — O(n)".
- **Events are deltas** since `fold.rs`: an `Advanced` event carries only what changed, marked `"_d"`,
  with deletions in `"_x"` (`workers/api/src/fold.rs:1-45`). The pre-delta images still fold correctly
  by construction (unmarked payload = snapshot).
- **D1 is not gone.** `wrangler.toml` still carries `[[d1_databases]] binding = "DB"`. Prepared
  statements outside `migrate.rs`: **0** (measured: `grep -c '\.prepare('` → only `migrate.rs:22`);
  `tools/gates/no-sql.baseline` = `0`. But **103 `ctx.d1("DB")` handles remain outside `migrate.rs`**
  (measured `grep -rn 'd1("DB")' workers/api/src | grep -v migrate.rs | wc -l`), e.g. `lib.rs:363`,
  `owner.rs:267`. The gate's own promise — "When the count reaches zero, the LAST commit removes
  `[[d1_databases]]`, deletes `workers/api/migrations/`" (`tools/gates/no-sql.sh:16-18`) — has not
  been kept; those 103 handles are what is left of it. Commit `8e956320` removed "ten dead D1 handles";
  the count above is what remains after it.

### 1.3 What is pure and what is effectful — counted

**Method.** A `services/` file is classed as PORT-TOUCHING if it matches
`Place::|stub\(\)|\.d1\(|durable_object|hubstore::|Fetch|worker::`, else PURE. Line totals by `wc -l`.
Await density by `grep -c '\.await'`. Clock reads by `grep -c 'Date::now'`. All measured 2026-09-22.

| Layer | Files | Lines | `#[test]` | `.await` | `Date::now` |
|---|---|---|---|---|---|
| `services/` PURE | 26 | 1,973 (1,104 excluding `tests.rs`) | 62 (all of services) | — | 1 (`services/operations/mod.rs:178`) |
| `services/` PORT | 26 | 3,801 | | 185 | |
| top-level `src/*.rs` handlers | 37 | 18,582 | 98 | 519 | 27 |
| `crates/dowiz-hub` | 27 | 15,237 incl. store | 317 lib | 0 | 0 |

Worker crate native tests: **169 passed, 2.32 s** (measured `cd workers/api && cargo test --lib --offline`).
Of those, the Durable Object's own file has 7, `hubstore.rs` 10, `fold.rs` 12, `idempotency.rs` 5 —
all of them test the PURE helpers (`changes_since`, `changed_chunks`, `merge`, `fingerprint`); the object's
I/O has no native harness.

**Reading of the numbers.** Of the request path that lives in the Worker (top-level + services,
24,356 lines), roughly 1,100 lines are pure decision code with tests and roughly 22,000 are handlers that
sequence I/O. The `services/mod.rs` header states the rule that produced the split: "at most ONE file in
[an area] touches a port, and everything else a function of its arguments" (`services/mod.rs:3-9`). That
rule is honoured in `services/` (each area's `mod.rs`/`handlers.rs` is the port file) and is not yet
applied to `storefront.rs` (1,310 lines, 7 awaits inside `place()` alone), `owner.rs` (1,514), `courier.rs`
(671), `booking.rs` (718), `accounts.rs` (955), `auth.rs` (734).

The seam that exists everywhere, named or not: **`hubstore::with_*` and `append_for` take a closure over
the image and run it under a retry loop** (`hubstore.rs:764-782` `with_catalog`; `hubstore.rs:1119-1170`
`append_for`, "`read` is given the CURRENT state … and returns the event to write"). The closure is a
function State → (State′, Out); the `with_*` is its interpreter. That IS a pure/effect split, used at
4 + 8 + 7 + 3 sites in the four handler files above (measured `grep -c`), without the vocabulary.

### 1.4 Concurrency guarantees today

- **Single writer per venue** is a platform property: "A Durable Object IS the serialisation: one instance,
  one writer, no race to guard against" (`hubdo.rs:13-18`).
- **The generation guard is kept anyway** and answers 409 (`hubdo.rs:591-600` `put_image`;
  `hubstore.rs:583-589`). Every `with_*` retries five times then fails loudly ("five attempts lost the
  generation guard", `hubstore.rs:779-781`). Inside one object the guard "cannot happen … but the
  caller's contract already says what to do" (`hubdo.rs:584-587`).
- **But the DECISION is still made in the Worker, one hop away from the state.** `append_for` GETs
  `/fold/order`, runs the closure (which calls `json_api::apply_event_logic`, `owner.rs:~577-586`), then
  POSTs `/fold/append` with the generation it read (`hubstore.rs:1128-1160`). The object's header says
  so: "THE DECISION IS STILL THE WORKER'S. The kernel decided … this is the record of it"
  (`hubdo.rs:551-553`). Two Workers can therefore race between the GET and the POST; the guard catches
  it and the retry re-decides. That is correct and it is exactly the read-modify-write-over-a-network
  that `BLUEPRINT-RESILIENCE-AND-EVOLUTION-2026-09-21.md` §1 calls "a distributed consensus problem
  invented for no reason" (rule 2: "the object decides, the Worker never does").
- **Placement still takes the whole log image** through `with_hub` so that a promotion's last use is
  counted and spent in one breath (`storefront.rs:~1080-1108`, "THE ONE WRITER THAT STILL TAKES THE
  WHOLE IMAGE … moving the redemption into the object is phase 6's business").
- **Placement is a two-image saga inside one object, with a hand-written compensation.** Stock is
  reserved in the `stock` image (`with_stock`, `storefront.rs:~1063-1072`), then the order is appended to
  `log` (`with_hub`), and if the second write fails the first is undone by a second `with_stock`
  ("THE ORDER DID NOT SURVIVE; ITS INGREDIENTS MUST NOT STAY HELD", `storefront.rs:~1110-1130`); if THAT
  fails it is only logged (`console_error!`). `courier::accept` has the same shape: `with_ops` then
  `append_for` (`courier.rs`, accept body lines 49-82 relative). The no-SQL blueprint lists gate **F31
  "a transaction touches exactly one image"** (`BLUEPRINT-NO-SQL-BEBOP-EVERYWHERE-2026-09-21.md` §7) —
  there is no `tools/gates/one-image.sh`; `ls tools/gates` shows `file-size`, `no-sql`, `one-venue` only
  (measured). F31 is a wish today, and placement violates it on every order that reserves stock.
- **Idempotency** exists at the HTTP layer with the five rules (`idempotency.rs:1-33`), keyed on
  `(venue, principal, route, key)`, stored in the venue's own `idem` image. It is called from **one
  route only**: `storefront::place` (measured `grep -rn idempotency::begin`). Courier `accept/pickup/
  deliver` and owner `order_action` are not covered — a retried "deliver" is refused by the FSM (illegal
  transition), which is a correct outcome by a different mechanism, but a retried `accept` after a lost
  response is an open question (not verified).

### 1.5 The offline story today

**Reads: real, on one surface.** The owner console keeps a replica of the queue in `localStorage`
(`workers/api/public/lib/replica.js:25, 32, 40`), draws it on boot before any request, marks it stale after
15 minutes (`STALE_MS`, line 27), and reconciles with `?since=<generation>` (`public/admin/app.js:161-189`).
The server answers from a 256-event ring in the object (`hubdo.rs:78-110` `RECENT_KEEP`, `changes_since`,
tested at lines 240-272) or says "full" and the client re-reads the list (`owner.rs:271-313`). A WebSocket
per client with hibernation tags (`hubdo.rs:112-140, 510-540`; `live.rs:1-21`; `public/lib/live.js`)
pushes events; the poll survives as the fallback ("`live()` NEVER replaces the poll; it makes it slow",
`live.js:8-14`). The header of `replica.js` states the model precisely: "a PREDICTION in exactly the sense
game netcode means … The server is still the authority" (lines 14-18).

**Writes: none, on any served surface.** `public/kit/sw.js:6-9`: "`/api/` IS NEVER TOUCHED … The worker
does not even intercept those requests". `public/sw.js:3-8` (storefront): "NETWORK FIRST, ALWAYS". The
courier PWA has an offline panel with a Retry button (`public/courier/app.js:474-506`) and nothing that
queues a tap. `offline/OfflineQueue.ts` is an IndexedDB queue with `pending → syncing → synced/failed`
(lines 4-9, `expanded-types.ts:125-134`) that **no served surface imports** (measured: the only reference
outside `offline/` is a doc comment in `engine/src/lib.rs:74`; `engine/src/offline.rs` is a Rust ring
buffer with a Ukrainian header). `apps/courier/` is a Rust crate with a `DispatchSession` FSM and a
`Requeued` event (`src/dispatch.rs:41-42`), in CI (`ci.yml:12`), and **no served surface loads any WASM**
(measured: `grep -rln '\.wasm\|WebAssembly' public/{courier,admin,store,kit}` → empty).

**CRDT: exists, off the path.** `crates/dowiz-core/src/mesh_replication.rs` implements "the G-Set CvRDT
merge: the union of two [content-id sets]" (line 209) with the test
`two_nodes_diverge_offline_reconnect_pull_identical_folded_state` (line 379). Its only consumer is
`kernel/src/brain/hydra.rs` (measured `grep -rln mesh_replication`), which is not on any request path.
`DECISIONS.md:369` (O4) already rules: "content-address-only for money/order; CRDT fenced out of those".
`crates/dowiz-core/src/wallet/mod.rs:18`: "NO CRDT — single-writer LWW is strictly correct".

### 1.6 Is there an event bus, and what does §9 refuse

There is no bus. `BLUEPRINT-MODULAR-ARCHITECTURE-2026-09-21.md` §9 refuses, in these words: "**An event
bus between in-process services.** A direct typed call is debuggable; a bus is a distributed system with
none of the benefits." It also refuses separate deployments per service, a shared `utils`, and rewriting
the kernel ("265 hub tests" — now 317, measured).

What does exist is narrower than a bus and should be named as what it is:

| Event-driven property | Today | Where |
|---|---|---|
| Append-only, content-chained log per venue | yes | `bebop-store/src/evlog.rs:1-12`; `Hub::chain_check` (`dowiz-hub/src/lib.rs:676`) |
| Fold = state | yes | `hubdo.rs:414-437` `orders_view`, memoised per generation |
| Projections | in-object only: orders view, `/fold/venue`; analytics fold in `services/analytics/fold.rs` (pure) | no durable projection store; every projection is recomputed from the log on a cold object |
| Replay | `Hub::grow`/`rotate` replay the chain into a fresh store (`evlog.rs:36-40`, `lib.rs:587`); the conservation audit (`3bc94e3a`) and the witness census (`witness/mod.rs`) compare counts and tips | no "rebuild every projection and diff" command |
| Subscribers | WebSocket fan-out after the write lands (`hubdo.rs:576-580`, "AFTER THE WRITE LANDED, never before") | best-effort; a sleeping client catches up via `?since=` |
| Outbound side effects | awaited inline after the append: `notify::order_placed(...).await` (`storefront.rs:1238`) | no outbox; a notify failure after a successful append is a lost message, not a retried one (verified by reading; not exercised) |
| Idempotent ingest | content-id chain in the log; HTTP `Idempotency-Key` on placement | not on courier/owner transitions (§1.4) |

### 1.7 The WASM monolith, as built

The Worker is one crate → one WASM module deployed to every edge PoP by one `wrangler deploy`; the FSM,
money, hub and store are linked, not called (`workers/api/Cargo.toml:26-29`). No OpenAPI, no generated
client; the services share Rust types by construction. That half of direction 4 is simply true.

The other half is not: the three browser surfaces (`public/admin`, `public/courier`, `public/store`,
`public/kit`) are hand-written JS with hand-copied vocabularies. The defects that produced:
`a18025d4` ("fourteen hand-written copies" of the status list; a collected order could never leave a
note — `services/orders/tests.rs:6-9`), the fourth money copy in the kit
(memory `kit-had-a-fourth-money-copy`), two pricers (`a18886d4`). The kernel's `wasm-bindgen` glue
(`scripts/build-kernel-wasm.sh`) is built and loaded by nothing that is served (§1.5).

### 1.8 Where the tree disagrees with its documents

| Document says | Tree shows | Evidence |
|---|---|---|
| `CLAUDE.md`: FSM at `kernel/src/order_machine.rs` | file absent; FSM at `crates/dowiz-core/src/order_machine.rs` | `ls` (measured) |
| `BLUEPRINT-MODULAR-ARCHITECTURE` §3: `Ctx { principal, venue, now_ms, ports, trace }`, `trait Handler`, closed `enum Fault` | not built: `find workers crates -name contract.rs` empty; no `struct Ctx`/`trait Handler`/`enum Fault` in `workers/api/src` | measured grep |
| `BLUEPRINT-NO-SQL` §6a: "ratchet 121 → 115 so far" | baseline is `0`; 22 statements, all in the exempt `migrate.rs` | `tools/gates/no-sql.baseline`, grep |
| `BLUEPRINT-NO-SQL` §7 F31 "a transaction touches exactly one image" | no gate; placement and courier accept touch two | `ls tools/gates`; §1.4 |
| `tools/gates/no-sql.sh:16-18`: at zero, remove the D1 binding and `migrations/` | binding present; 103 `ctx.d1("DB")` handles | `wrangler.toml`; grep |
| `BLUEPRINT-RESILIENCE` §1: "Four paths still take the whole image" | placement still does (`with_hub` for promo); the others not re-verified here | `storefront.rs` |
| `OFFLINE-RESILIENCE-SYNTHESIS-2026-07-20.md:21`: convergence "already solved at the algorithm layer" | true in `dowiz-core`; reachable from no served surface | §1.5 |
| `DECISIONS.md` D1: "each node running the Rust/WASM kernel + a local SQLite DB" | no SQLite anywhere; bebop images in a Durable Object | `kernel/src/bebop_event_store.rs:3-7` says the same |
| `docs/design/ARCHITECTURE.md` and the P-numbered roadmap | not re-read for this document; treat as historical per `CLAUDE.md` ("start at `ROADMAP.md`") | not verified |

---

## 2. Scores

**Scale, defined once.** 0 = absent. 1 = named in a document only. 2 = present in code that no request
executes. 3 = on the request path for some flows, without a gate. 4 = a property: on every relevant path
AND a gate that goes red when it regresses. A direction can score differently for reads and writes, and
the table says so rather than averaging.

| Direction | Score | Already true (cite) | Not true | What the gap has cost (real commits) |
|---|---|---|---|---|
| **1. Local-first + CRDT** | reads **3**, writes **0**, CRDT **2** | console replica in `localStorage` + `?since=` + socket (§1.5); DO per venue is "local" to the venue's traffic; G-Set CvRDT in `dowiz-core` with a two-node divergence test | no offline WRITE queue on any surface; `/api/` explicitly never cached; CRDT unreachable from product; courier app is a Rust crate nothing loads | `564f93cc`: a lost response + retry = a second order, second stock reservation, second kitchen ticket — the offline-write problem arriving through a weak connection instead of a tunnel. `courier/app.js:506` "youAreOffline" is the whole courier offline UX. |
| **2. Stateful edge actor mesh** | **3** (highest of the six) | one object per venue holding the images in memory (`hubdo.rs:191-197`), fold memoised in the object, sockets hibernate with tags, `__platform` object; tenancy "as a property of where the bytes live" (`table.rs:11-14`) | the decision is in the Worker one hop away (`hubdo.rs:551-553`); placement takes the whole image for promo; two-image sagas with hand compensations (§1.4); no `one-image` gate | the promo total computed OUTSIDE the CAS closure was what reached Stripe (`RESILIENCE` §1, last paragraph; `storefront.rs` "WHAT THE CUSTOMER IS CHARGED IS WHAT THE ORDER SAYS, and those were two different numbers"). `ddcd0548`: 37 handlers chose the venue twice in the Worker and acted on the wrong object — a defect only possible because routing is decided outside the actor. |
| **3. Effect-driven / algebraic** | **2.5** | kernel takes `id` and `created_at_ms` as arguments (`json_api.rs:238-244`); `dowiz-hub` bytes-in/bytes-out with 317 tests and zero awaits; `services/` pure files carry 62 tests with no Durable Object; `with_*`/`append_for` closures are State→(State′,Out) under an interpreter (§1.3) | no effect DESCRIPTION anywhere — effects are performed, not declared; 27 `Date::now` in handlers; `Ctx`/`Ports` not built; 22,000 lines of handlers un-testable natively (the DO has no native harness) | `5b6c680f` "six places that each invented their own" midnight; `486a5c38` analytics days 24 h apart while the venue's are not — both are the clock read ad hoc in a handler. `a18886d4` two pricers: a rule written twice because there was no one pure place for it until `services/ordering/pricing.rs`. |
| **4. Edge-compiled distributed monolith (WASM)** | server **4**, clients **1** | one crate, one WASM, one deploy, kernel+hub+store linked (§1.7); `no_std` core; `cargo-deny` and zero-dep allowlists (`ZERO-DEP-ALLOWLIST.txt` in three crates) | the browser vocabularies are hand copies; no generated types; the kernel WASM glue is served to no one; a 10,491-line twin server outside CI | `a18025d4` fourteen status copies; the kit's fourth money copy (1500 lek drawn as $15.00); `a18886d4`. Every one is "shared types with no OpenAPI" NOT reaching the clients. |
| **Event-driven** | storage **4**, property **2** | append-only chained log; fold=state; deltas (`fold.rs`); broadcast after commit; conservation audit + witness in CI (`3bc94e3a`, `5b590d3c`) | no durable projections, no rebuild-and-diff command, no outbox (notify awaited inline `storefront.rs:1238`), idempotency on one route | `253e1ece`: a `db.batch` that half-applied left a booking's status disagreeing with its own history — fixed by making the aggregate one image, which is the event-sourced answer; the same class is still open across `stock`+`log`. |
| **Data-driven** | **2** | `services/analytics/fold.rs` (pure, tested), `gauges.rs` (image ceilings), `otel.rs` one span per request with `x-trace-id` on every response (`lib.rs:124`), errlog per venue (`aa31938d`) | `head_sampling_rate = 0.1` (`wrangler.toml`) — nine of ten 500s were unrecorded until `aa31938d`; no eval loop closes on production data (`BLUEPRINT-EVALS-…` exists; not re-verified here) | "`worker_errors` had never received a row" (`lib.rs:91-99`) — the instrument measured nothing for its whole life. |

The honest one-line summary: **dowiz is an actor-per-venue system with an event-sourced store and a pure
kernel, whose decisions still run one network hop away from the actor, whose clients cannot write while
offline, and whose "shared types" stop at the WASM boundary.** Directions 2 and 4 are mostly built;
direction 1 is built for reads and absent for writes; direction 3 is practised without being named.

---

## 3. The plan, in order

Each item: the defect or capability, the change, the cost, the risk, the CHECK. Items marked
**AGAINST** are recommended against, with the reason. Costs are estimates (hypotheses) unless a number is
cited; nothing below was timed.

### P1. Move the order decision INTO the object — a command surface on `HubImages`

- **Defect answered:** the promo/total defect (§2, row 2); the two-image saga with a logged-only
  compensation (`storefront.rs` "could NOT release … after a failed placement"); the `append_for` GET→
  decide→POST race window and its five-retry loop; F31 unenforceable while it is true.
- **Change:** `POST /fold/command` on the object with a closed enum of commands — `Place { envelope,
  reservations, promo }`, `Advance { order_id, next }`, `Accept { courier_id, order_id }` — executed in
  ONE object turn over the images it already holds in `mem`: reserve stock, count and spend the promo,
  append `Placed`, broadcast. The Worker keeps authentication, parsing, pricing (pure), and becomes a
  transport for a command, which is what `lib.rs:1-4` already claims it is. `json_api::apply_event_logic`
  is in the same crate, so no new dependency crosses the boundary.
- **Cost:** `hubdo.rs` (1,006 lines) must be split first — the file-size ratchet is at `over=41,
  worst=1985` (`tools/gates/file-size.baseline`), and a command module is 300-500 lines by the shape of
  `place()` (hypothesis). The compensation code in `storefront.rs` is deleted, not moved. The
  `with_hub` whole-image write goes away for placement.
- **Risk:** the object serialises ALL of a venue's commands, so pricing must stay in the Worker (it is
  pure; keep it there) and only the state-touching tail moves. CPU time inside one object turn is the
  budget to watch; `otel` already times the request — measure before and after on the live venue.
- **CHECK:** (a) `tools/gates/one-image.sh` — brace-matched like `one-venue.sh`, counts handler bodies
  with more than one `with_*`/`append_for`/`with_ops` call; baseline = today's count (§1.4 gives
  4+8+7+3 call SITES, not bodies — the gate measures bodies), ratchet to 0. (b) The L3 collision probe
  from `RESILIENCE` §1: 20 simultaneous placements for the last portion → one 200, nineteen 409, one
  reservation, no 500. (c) `stock::stranded()` empty after a forced failure of the log append
  (a chaos toggle behind a test-only header).

### P2. An offline WRITE queue for the courier and the console, on the idempotency layer that exists

- **Capability unlocked:** a courier in a basement or a tram taps "picked up"; the tap survives. This is
  the operator's direction 1 where it is genuinely warranted, and it needs **no CRDT**: a courier's
  actions on one order are a sequence, the server FSM refuses illegal transitions
  (`CLAUDE.md`: "Forbidden transitions are errors, not silent no-ops"), and `Idempotency-Key` makes the
  replay of the same tap a no-op. Queue + idempotent replay + a refusing FSM = a deterministic merge,
  by construction of what already exists.
- **Change:** (a) extend `idempotency::begin` to `courier::accept/pickup/deliver` and
  `owner::order_action` (today it guards `storefront::place` only, §1.4); (b) a client queue in
  IndexedDB (the shape of `offline/OfflineQueue.ts`, rewritten under `public/lib/` — the original is
  orphaned and typed against a deleted app) that stores `{route, body, key, queued_at}` and drains in
  order on `online`, applying the replica's own prediction locally; (c) `kit/sw.js` rule 1 stays: reads
  are never cached; only the queue is local.
- **Cost:** one server change per route (small, the middleware exists); one client module ~200 lines
  (hypothesis); an e2e test that flips the network off (the `e2e/` tree has Playwright).
- **Risk:** a queued `deliver` replayed after the owner cancelled → 409 from the FSM; the client must
  surface that as "this order changed while you were away", not retry forever. Bound the queue (the
  orphan had `MAX_QUEUE`).
- **CHECK:** Playwright: courier taps deliver with the network off, reconnects, exactly one `Advanced`
  event in the log (read via `/api/owner/history`), no duplicate; a second replay of the same key answers
  the stored response (rule 1 of `idempotency.rs`).

### P3. Inject the clock (and the id) into every handler — the `Ctx` from MODULAR §3, built small

- **Defect answered:** `5b6c680f`, `486a5c38` (§2 row 3). Also the reason services cannot be tested at
  a chosen time.
- **Change:** `Ctx { now_ms, venue, principal, trace }` built once in `lib.rs::main` and passed down;
  `Date::now` allowed in exactly three places (`main`, `append_for`'s clock stamp, `auth`), everywhere
  else it is `ctx.now_ms`. **No `Ports` trait object and no `Handler` trait** — see P7 for why.
- **Cost:** mechanical; 27 sites (measured).
- **CHECK:** `tools/gates/clock.sh` — count of `Date::now` outside the allow-list, baseline 27,
  ratchet to 0.

### P4. Generate the client vocabulary from the kernel — the cheap half of direction 4

- **Defect answered:** `a18025d4` (fourteen copies), the kit's fourth money copy, every future copy.
- **Change:** a build step (`tools/gen-vocab`, Rust, reads `dowiz_core::OrderStatus` and the money
  formatting law) emits `public/lib/vocab.js` (status list, terminal set, currency minor units) — and
  a gate diffs the committed file against a fresh generation.
- **Cost:** a day (hypothesis). No runtime change.
- **CHECK:** `tools/gates/vocab.sh` red when the generated file is stale; grep for hand-written
  `'DELIVERED'` string sets outside `vocab.js`, ratchet to 0.

### P5. Make event-driven a property: rebuild-and-diff, and an outbox

- **Defect answered:** the class `253e1ece` closed for bookings is still open across images (stock vs
  log); a notify failure after a landed append is lost (`storefront.rs:1238`, verified by reading only).
- **Change:** (a) `/fold/rebuild` on the object: refold every projection from the log into fresh
  memory and compare with the memoised one and with the stock ledger's derived reservations; wire into
  the conservation audit (`3bc94e3a`) as law 8. (b) An `outbox` LogImage per venue: the command in P1
  appends the outbound effect (Telegram/WhatsApp/Stripe intent) in the SAME turn as the event; the
  object's alarm drains it with retries; the Worker stops awaiting `notify::` inline.
- **Cost:** (a) small — the folds are pure and exist; (b) a new image + an alarm handler (~300 lines,
  hypothesis) and the removal of inline `notify::` calls.
- **Risk:** the alarm is the first background execution in the object; its failure must be visible in
  `/api/owner/health`.
- **CHECK:** F28 four-way fold (no-SQL §7) becomes a gate; a forced notify failure leaves one row in
  `outbox` and the next alarm drains it (native test of the pure drain decision + one live probe).

### P6. CRDTs — **AGAINST**, except where named

- **Why against:** a Durable Object is one writer per venue (§1.4), so within a venue there are no
  concurrent writers to merge; `DECISIONS.md:369` already fences CRDT out of money and orders; the
  stock ledger is a fold over an append-only log, which is already a join-semilattice over content ids
  (union of records) — adding a CRDT library on top would be a second merge with the same result and
  a new vocabulary to keep honest. The courier position map is already a last-writer-wins register: each
  fix overwrites the previous one on arrival (`hubdo.rs:976-978`, `Fix { lat_e6, lng_e6, at_ms }` at
  147-151), reads drop fixes older than `POSITION_KEEP_MS` (line 746), and the object serialises
  arrivals so "last" is well-defined. Naming it LWW costs a comment, not a crate.
- **Where they ARE warranted, and are not built:** two writers that cannot share an object — a POS
  till that must take orders with the hub unreachable, or two branches writing one ledger. Neither
  exists today: a branch is a venue is an object. When one does, the merge of two append-only logs of
  signed, content-addressed events IS the G-Set in `mesh_replication.rs`; what has to be DESIGNED is
  the rule when the merged fold drives stock negative (refuse-at-fold vs. compensate). That is a
  decision, not a library. Revisit at the first venue with a second writer; not before.

### P7. A general effect system in Rust — **AGAINST**; the `services/` split plus P3 buys most of it

- **What it would cost here:** the Worker's I/O is `!Send` wasm-bindgen futures behind
  `worker::Request/Response`; an effect interpreter means either (a) an `async_trait` `Ports` object
  with a fake for every port (D1, DO stub, KV, fetch, email, clock, CSPRNG) — a second copy of each
  port to keep honest — or (b) an effect enum + a runtime loop, rewriting 519 + 185 awaits (measured)
  into a description. Both are a rewrite of the 22,000 handler lines to reach a property the kernel and
  the hub already have by construction and the services reach by file discipline (§1.3).
- **What the split already buys:** 62 service tests and 317 hub tests run natively with no network and
  no object (measured, 2.32 s and 3.55 s). The pure decision for the audit chain "can never run in a
  test on this box if it needs a Durable Object" (`5b590d3c`), and so it was written pure — the
  discipline is working.
- **What is still missing and cheap:** P3's `Ctx` (the clock), and finishing the `services/mod.rs`
  rule for the six big handler files (§1.3) — `storefront.rs::place` alone has six pure steps
  interleaved with seven awaits and is the file to do first.
- **CHECK** that the discipline holds rather than the vocabulary: the file-size ratchet already exists
  (`over=41, worst=1985`); add a `tests.rs`-per-area presence gate under `services/` and the `clock.sh`
  ratchet from P3.

### P8. The twin server — decide, do not drift

- **Defect:** 73 routes and 10,491 lines implement the same surface outside CI (§1.1). Every rule fixed
  in the Worker since `a18886d4` is either re-fixed there or silently wrong there.
- **Change:** one of two, and this document does not choose: put `cd tools/native-spa-server && cargo
  test` in `ci.yml` and port the shared rules through `dowiz-hub` (where the pure ones already live), or
  delete it. The deciding fact is whether P67 (one restaurant, one box, `src/hub.rs:3-4`) is still a
  product line; that is the operator's, not this document's.
- **CHECK:** a `ci.yml` line either way.

### P9. Documents and leftovers — the "last commit" the gate promised

- Fix `CLAUDE.md`'s FSM path; correct no-SQL §6a's ratchet numbers to the baseline; either build the
  D1 removal the gate promised (`no-sql.sh:16-18`; 103 handles, `migrate.rs`, the binding, `migrations/`)
  or write down why not.
- **CHECK:** `grep -rn 'd1("DB")' workers/api/src | grep -v migrate.rs | wc -l` → 0; `[[d1_databases]]`
  absent from `wrangler.toml`.

**Order rationale.** P1 removes the last read-modify-write over a hop and makes F31 enforceable; P2 is
the only item that changes what a courier can do in a basement and it depends on nothing but the
middleware that exists; P3 and P4 are cheap ratchets that close recurring defect classes; P5 turns the
storage format into the property. P6 and P7 are refusals with a stated re-entry condition. P8 and P9
are hygiene that the gates already ask for.

---

## 4. The operator's direct question

*"Do you use functional-programming approaches — separating pure computation from port effects — to
isolate business logic from the runtime?"*

**Yes, at three layers, by construction rather than by a framework; and no at the fourth, where it is
by discipline and incomplete.**

1. **The kernel is pure by signature.** `place_order_at(id, customer_id, items_json, created_at_ms,
   channel)` (`crates/dowiz-core/src/json_api.rs:238-244`) receives the id and the time; the Worker
   supplies them (`lib.rs:9-11`, "Identity and the clock come from the EDGE, never from the kernel").
   `apply_event_logic(order_json, next_status)` (`json_api.rs:278`) reads nothing but its arguments. The
   crate is `#![no_std]` with zero dependencies (`crates/dowiz-core/Cargo.toml:6,14`); MANIFESTO C2
   ("no clock/RNG/env/floats/network … reaches the kernel", `MANIFESTO.md:15`) is what the grep in
   §1.1 confirms for the order/money path.
2. **The hub is pure by contract.** "Bytes in, bytes out. No filesystem, no network, no clock, no
   randomness — the caller supplies the store image, the time and the ids" (`crates/dowiz-hub/src/
   lib.rs:3-5`); `Hub::append(kind, order_id, order_json, seq: u64, actor_pubkey)` takes its
   sequence stamp as a parameter (`lib.rs:363-370`) — the object passes its clock in as `seq`
   (`hubdo.rs:573`). 317 tests, 3.55 s, no I/O (measured).
3. **The Worker's write seams are pure closures under an interpreter.** `with_catalog`, `with_stock`,
   `with_hub`, `with_table`, `append_for` (`hubstore.rs:764, 743, 1028, 1119`) each take
   `FnMut(&mut State) -> Result<Out>` and own the load/retry/save. The closure is the business rule;
   the `with_*` is the effect. That is the port/effect separation in practice, and it is what let the
   witness's deciding half be "PURE and that is not an aesthetic choice" (`5b590d3c`).
4. **The handlers are where it is incomplete.** `services/` applies the rule "at most ONE file per area
   touches a port" (`services/mod.rs:3-9`) and holds 1,104 pure lines with 62 tests; the six large
   handler files outside it (`storefront.rs`, `owner.rs`, `courier.rs`, `booking.rs`, `accounts.rs`,
   `auth.rs`, together ~5,900 lines) interleave decisions with 27 direct clock reads and hundreds of
   awaits (§1.3), and the Durable Object itself has no native test harness — its 7 tests cover the pure
   helpers only.

There is no effect DESCRIPTION anywhere: effects are performed, not declared and interpreted. §3 P7
argues that is the right call here, and P3 names the one missing injection (the clock). The defects
that the incomplete fourth layer has produced are real and listed in §2, row 3; every one of them was
fixed by moving a rule into a pure file with a test, which is the mechanism this document recommends
continuing rather than replacing.

---

## 5. What was NOT verified, and the command that would settle it

| Claim in this document | Status | Command |
|---|---|---|
| The deployed Worker is built from `c5c640cc` (memory: promoted binaries have differed from source before) | not verified | `cd workers/api && npx wrangler deployments list` and compare the version id against a fresh `worker-build` |
| Latency of one `append_for` round trip vs. an in-object command (the P1 payoff) | not measured | read `otel` spans for `POST /api/owner/orders/:id/action` before/after P1, or `npx wrangler tail --format json` on the live venue |
| `notify::order_placed` is awaited inline and NOT in `wait_until` | verified by reading `storefront.rs:1238` and grepping `wait_until` in `notify.rs`/`owner.rs` (none); not exercised | force a Telegram failure on a test venue and read `/api/owner/health` |
| Courier `accept` replayed after a lost response creates a duplicate hold in `ops` | not verified (no idempotency on that route, §1.4) | fire two `POST /api/courier/orders/:id/accept` with the same token 50 ms apart against a test venue; read `ops` via `/api/owner/couriers/:id` |
| The other three "whole image" paths named in `RESILIENCE` §1 (reveals audit, `graph_facts`, export/health) still take the whole image | not re-verified | `grep -n "with_hub\|load(&place)" workers/api/src/*.rs` |
| `BLUEPRINT-EVALS-TRAFFIC-MEMORY-LATENCY-2026-09-21.md` describes something built | not read | read it and `ls tools/evals` |
| Line numbers in `hubstore.rs`/`storefront.rs` against HEAD | working tree has uncommitted edits to both (§ header) | `git stash; grep -n … ; git stash pop` — or wait for the 15-file change to land and re-cite |
| The kernel WASM glue is served to no browser surface | measured on `public/` only; `web/` (the older demo shell with its own `sw.js`) not checked for deployment | `grep -rn "kernel/pkg\|\.wasm" web/src | head` and whether `web/` is behind any host |
| `apps/courier` `DispatchSession` FSM matches the server FSM's courier edges | not checked | a test in `apps/courier/tests/` that folds the same event sequence through `dowiz_core::order_machine` and `DispatchSession` |
| 13,340 `.rs` files under the repo (from `find`, excluding `target`/`node_modules`) | the number includes vendored and research trees; not used above | `find . -name '*.rs' -not -path '*/target/*' -not -path '*/node_modules/*' -not -path './bebop-lang/*' | wc -l` |
