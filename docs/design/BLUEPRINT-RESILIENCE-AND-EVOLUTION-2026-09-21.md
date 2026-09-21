# Blueprint — resilience, concurrency and evolution

**2026-09-21.** The third sibling. The [verification pyramid](./BLUEPRINT-VERIFICATION-PYRAMID-2026-09-21.md)
answers *how do we know it is right*; the [modular architecture](./BLUEPRINT-MODULAR-ARCHITECTURE-2026-09-21.md)
answers *where the code lives so that it can be asked*. This one answers the
third question, which the first thirty defects did not raise but the next thirty
will:

> **what happens when two cashiers, two clocks, two schema versions or two
> networks disagree.**

Ten layers. Each one states **what already exists in this tree, with the file
and line**, then what is actually missing, then the smallest honest version of
the missing part. Nothing is adopted because it is a famous pattern. Half of
this list turned out to be already built and not wired up, which is this
codebase's characteristic failure — see `bebop-instruments-that-measure-nothing`.

---

## 0. The inventory, before the plan

| Layer | Built? | Evidence in this tree |
|---|---|---|
| CAS / optimistic concurrency | **yes** | `hubstore.rs` generation guard, `x-generation` header (`:389`, `:428`) |
| Hash-chained audit log | **yes, dormant** | `content_id_chained` (`dowiz-hub/src/lib.rs:688`), `chain_check` (`:632`), tested hot+cold (`:916`) — **called by nothing in production** |
| Snapshots / checkpoints | **yes** | `Hub::rotate(keep)`, `EventKind::Checkpoint`, archives at `log@<generation>` (phase 5) |
| Circuit breaker | **yes, unused** | `crates/dowiz-core/src/breaker/` — 8 files, `admit() -> Result<Permit, Tripped>`; re-exported `kernel/src/breaker/mod.rs:10`; **`grep breaker workers/api/src/` is empty** |
| Distributed tracing | **yes, half-wired** | `otel.rs`, W3C `traceparent` continued not replaced; begun once at `lib.rs:70` |
| Property testing | **dependency present, mis-aimed** | `proptest = "1.11"` in `dowiz-core` and `kernel`; `kernel/fuzz` targets are `fuzz_trinary`, `fuzz_eigen` — mathematics, not money |
| Idempotency | **five hand-rolled schemes, no contract** | §2 |
| Schema upcasters | **version byte exists, no read-path chain** | EvLog v2 version in the root's 8th cell |
| Poison-pill quarantine | **no** | §5 |
| Timezone correctness | **no — and there is a live defect** | §9 |
| POS hardware ports | **no surface exists** | §7 |

**The shape of the work is therefore "wire up and gate", not "build".** Four of
the ten are finished components with no caller. That is cheaper than it looks
and more dangerous than it sounds: a dormant safety component is indistinguishable
from a working one in every document that mentions it.

---

## 1. Concurrency: who wins the last unit, and how the loser is told

### What exists

A placement is **one CAS write inside one Durable Object per venue**. The object
serialises its own calls, so two customers buying the last portion are not a
data race — they are two sequential decisions. The generation guard in
`hubstore.rs` is the optimistic lock: a write carries the generation it read at,
and the row moves only if that is still the generation on disk.

Since phase 2 the object folds its own log (`/fold/orders`, `POST /fold/append`),
so most writes are **blind appends** that cannot lose a race at all.

### What is actually exposed

**Four paths still take the whole image**, each documented where it happens:
placement's promo atomicity, the reveals audit, `graph_facts`, and export/health.
Those are read–modify–write across a network, and they are the only places where
a CAS can be lost. Placement is one of them — which is the expensive one, because
it is the path that reserves stock.

Three things are missing, and none of them is a lock:

1. **A typed, bounded retry.** A lost guard must produce `Fault::Conflict`, a
   bounded number of retries with the fresh image, and then a refusal the caller
   can act on. Today the behaviour on a lost guard is per-call-site.
2. **An arbitration rule stated once.** The rule this product should commit to is
   **"the object decides, the Worker never does"**: the last-unit decision is made
   by code running inside the object against the image it is holding, not by a
   Worker comparing an image it fetched 40 ms ago. Anything else is a distributed
   consensus problem invented for no reason.
3. **The loser's experience.** A 409 is correct and useless. The refusal carries
   `Refused { rule: "stock.insufficient", detail }` and the storefront turns that
   into *"the last one just went — here are two others"*, from data the menu
   response already has. A second cashier must never see a 500, and must never
   see a spinner that resolves into a charge.

### The tests this layer owns

- **Property (L2).** For any interleaving of *N* placements against stock *k*:
  exactly `min(N, k)` succeed, the rest are `Refused`, the folded level is
  `k - min(N,k)`, and `StockLedger::stranded()` is empty.
- **Live collision probe (L3).** Fire 20 placements for the last portion at the
  deployed hub with `Promise.all`; assert one 200, nineteen 409, one reservation,
  no 500, and the venue is still open afterwards.
- **The regression this catches** is not hypothetical: modifier pricing and promo
  discount both already live inside the CAS closure, and the promo bug shipped
  precisely because a value computed *outside* the closure was the one that
  reached Stripe.

---

## 2. Idempotency: the retry that places a second order

### What exists — five different schemes

| Site | Scheme |
|---|---|
| `booking.rs:240,276` | client `request_id` → deterministic id `id64(venue:request_id)` |
| `wallet.rs:227,268` | client `request_id`, same shape |
| `social.rs:176,235` | an idempotency key field, checked first, resend is a no-op |
| `stripe.rs:55,83` | **outbound**: `idempotency-key: <order_id>` so a retried intent is the same intent |
| `storefront.rs:1142` | the comment says the order id is the idempotency key |

### The defect that is in the tree right now

That last row is not true in the direction that matters. `place()` mints the
order id from the platform CSPRNG at **`storefront.rs:846`**:

```rust
let Some(id) = crate::edge_id() else {
    return Response::error("no platform CSPRNG for order id", 500);
};
```

The id is created **by the server, per attempt**. So a customer on a weak
connection whose response is lost, and whose app retries, gets **a second
order**: a second stock reservation, a second kitchen ticket, a second Stripe
intent keyed on the new id. Stripe's idempotency key protects the *charge*; it
cannot protect the *order*, because the key is derived from the thing that is
duplicated. The venue does not lose money — it loses a portion of food and a
courier's trip, which is worse per incident than the charge would have been.

This is the exact failure the mobile-POS analysis predicted, and the platform
has it today on its highest-value route.

### The contract

One mechanism, in `platform/`, applied by `Ctx` and never re-implemented:

- **Every unsafe route (POST/PUT/PATCH/DELETE) requires `Idempotency-Key`.**
  Routes that do not yet send one get a grace flag that may only be removed,
  never added — the same ratchet the file-size gate uses.
- The stored key is the tuple **`(venue, principal, route, key)`**. A key from
  one venue can never replay into another; that is a tenancy defect waiting to
  be written otherwise.
- **The stored value is the full response**, status and body, not a "seen"
  marker. A no-op that returns `204` where the first call returned an order and
  a customer token has not made the client whole.
- **A replay with a different body under the same key is `409`, loudly.** This is
  Stripe's own rule and it is the half that catches client bugs: a POS that
  reuses a key across two different baskets is broken, and silence would hide it.
- **TTL 24 h**, in D1 with the key hashed, swept by the nightly cron that already
  runs.
- **The in-flight case is not optional.** A retry that arrives while the first
  call is still running returns `409 Conflict` with `Retry-After`, never a second
  execution. Without this the middleware only narrows the window it was built to
  close.

### Why this must land before offline writes

An offline write queue (§6) means the **client** mints the key, hours before the
server sees it. That only works if the key is already the contract. Order:
`Idempotency-Key` → client-minted keys → offline queue → signed offline queue.

---

## 3. Schema evolution: upcasters, and the corpus that proves them

### What exists

- EvLog **v2**: the version lives in the root's 8th cell; v1 logs stay v1 and
  `grow()` is the migration.
- Events carry **deltas** marked `"_d": true`; an unmarked payload is a snapshot
  that replaces. This is already an evolution mechanism — it is why every
  pre-existing image reads identically after phase 3.
- The modular blueprint's `#[schema(name, version)]` derive, with the JSON Schema
  committed to `contracts/` and a CI compatibility check.

### What is missing: the read-path chain

A stored envelope is **never rewritten**. The log is append-only and now
hash-chained (§4), so a migration that edits history is not merely bad practice
here — it would break the chain. Evolution therefore happens **at read time**:

```rust
/// Total, pure, and free of I/O. Composed into a chain: v1→v2, v2→v3, …
/// The fold NEVER sees anything but the current version.
fn upcast(from: u16, payload: Value) -> Result<Value, Fault>;
```

Three rules make this survivable rather than a second source of drift:

1. **Upcasters are append-only too.** `upcast_v1_to_v2` is written once and never
   edited. Editing one rewrites history just as surely as editing the log.
2. **A field is added, never removed or retyped.** Removal is a version bump with
   an upcaster that supplies the default, so the *reader* decides what a missing
   field meant, with the evidence in front of it.
3. **The corpus is the proof.** `contracts/corpus/` holds real historical
   envelopes, one per version per message, committed. CI folds the whole corpus
   through the chain and compares to committed expectations. **A schema change
   that cannot upcast the corpus fails the build.**

That corpus discipline is borrowed wholesale from machine learning practice: a
frozen evaluation set, versioned with the code, that a change must be measured
against rather than argued about. It is the same instrument as a golden test,
and it is the only thing that will keep the monthly analytics export from
failing on year-old receipts.

### The tests this layer owns

- `upcast` is **total** over the corpus (property).
- `fold(upcast(corpus))` equals the committed status for every archived log.
- A synthetic "future version" envelope is **refused loudly**, never guessed at.

---

## 4. The audit chain: built, dormant, and missing its witness

### What exists — and it is more than the analysis assumed

`crates/dowiz-hub/src/lib.rs`:

- `content_id_chained(prev, payload)` at **:688** — every record's id commits to
  the record before it.
- `chain_check()` at **:632** — walks the log and verifies each id.
- Tested against both a hot log and a **reloaded archive** at :916–920.

So the "blockchain-like chain of receipts" is not a proposal here; it shipped in
phase 4. What is wrong with it is more interesting than its absence.

### Three verified gaps

1. **Nothing in production calls `chain_check`.** It is a test-only instrument,
   which by this codebase's own standard means it is not an instrument.
2. **It cannot see a deletion.** By construction it detects an *edited* record —
   an id that no longer matches its payload. A record removed from the end, or a
   log truncated, leaves a shorter but perfectly valid chain.
3. **There is no external witness.** The chain proves internal consistency to
   whoever holds the image. Anyone who can write the image can recompute the
   whole chain. Against the threat the analysis names — *editing the database
   after the fact* — an unwitnessed chain proves nothing.

### The fix, in three sizes, cheapest first

- **Nightly `chain_check` before the backup**, failing loudly through
  `Fault::Unavailable` so it lands in `worker_errors`. One call site. Closes gap 1.
- **A length-and-tip commitment.** Write `(generation, record_count, tip_hash)` to
  a D1 row on every rotation. A truncation now contradicts a row in a different
  store. Closes gap 2 for anything short of compromising both.
- **The off-site witness, which is nearly free.** The nightly S3 archive already
  travels off-site once. Put `(record_count, tip_hash, generation)` in its
  manifest and the off-site copy becomes a witness written by a different
  principal at a time the editor cannot revisit. Closes gap 3 without a chain of
  anything.

**Merkle trees are explicitly deferred.** Their advantage is proving *one*
receipt without shipping the log, which nobody has needed yet. Named here so
that when a tax inspection or a dispute does need it, the reason is on record.

---

## 5. Poison pills: one bad record must not close the restaurant

### What exists

The snapshot half is done. `Hub::rotate(keep)` plus `EventKind::Checkpoint`
means a fold does not have to start at the beginning of time, and the fold is
memoised per generation.

### What is missing

**Containment.** A record that fails to parse mid-log must not be able to take
the venue down — and, just as importantly, must not be able to disappear
quietly. The `.ok().flatten()` defect (a storage error read as "no image", which
would have orphaned a venue's entire order log) is the same mistake in a
different costume: a failure converted into an absence.

The required shape:

- A record that cannot be parsed is **skipped, counted, and recorded** in a
  quarantine list carried on the hub.
- The fold **continues from the last checkpoint**, so the venue keeps serving.
- The count and the ids are surfaced by `/api/owner/health` — which already
  carries the image gauges and the last twenty errors — and a non-zero count is
  a **failing gate**, not a warning.
- Quarantine is never automatic repair. A quarantined record is preserved
  verbatim for a human, because it is evidence.

### The test this layer owns (L5)

Inject a corrupted record into a copy of a real log; assert: the venue serves,
the order count drops by exactly one, health reports one quarantined record with
its id, and the gate goes red. A quarantine that nobody notices is a data-loss
feature.

---

## 6. Circuit breakers: the one that was already written

### What exists, unused

`crates/dowiz-core/src/breaker/` is eight files of kernel-grade work:

- the gate is **one function**, `admit() -> Result<Permit<'_>, Tripped>`;
- a gated operation takes `&Permit` **by signature**, so a call site cannot
  forget or invert the check — omitting it is a compile error;
- there is deliberately no `is_tripped()` accessor that could gate a mutation;
- "tripped-but-permitting" is unconstructible.

It is re-exported by the kernel at `kernel/src/breaker/mod.rs:10`. Its only
callers are `autonomic.rs`, `autonomic_pmu.rs` and `temporal_tmr.rs` inside
`dowiz-core`. **The Worker has never mentioned it.**

### Where it belongs

Three outbound rails can make this product look broken while it is perfectly
healthy: **Stripe**, the **AI endpoint** (`ai.endpoint`, which already shipped
defaulting to a loopback URL a Worker can never reach), and **Meta/WhatsApp**.

One breaker per rail, keyed by rail and venue, state in the Durable Object
(which is the only place per-venue state can live). A trip produces
`Fault::Unavailable { place }`, which writes its own `worker_errors` row by
construction — so tripping a breaker is automatically visible, and the "instrument
that has never fired" problem does not recur here.

**The breaker does not invent the fallback; it stops paying for the failure.**
Placement already degrades correctly when Stripe is unreachable
(`storefront.rs:1165` marks `payment_error` so the surface can offer cash). What
it does not do is stop *trying* — every customer during an outage pays the full
timeout before being offered cash. That is the breaker's actual job here.

### Local-first, stated honestly

`/lib/replica.js` (phase 7, first step) is **read-side only**: the console draws
its own copy and keeps drawing through an outage, saying *"offline — last known
queue"*. Offline **writes** do not exist. The analysis is right that they are
the endgame for a venue on a bad line, and the order is forced:

1. `Idempotency-Key` as a contract (§2) — without it an offline queue duplicates.
2. Client-minted keys.
3. A queued write with the device's own sequence number.
4. Device signing, which is only worth building once 1–3 are real.

Anything attempted out of that order produces duplicate orders under exactly the
conditions it was built to survive.

---

## 7. The hardware edge, and the POS question answered plainly

**This product has no POS surface.** A grep across `workers/api/src/` and every
client for `printer`, `fiscal`, `barcode`, `terminal`, `kiosk` returns one
comment about a terminal *state* and one particle animation. There is no cash
drawer, no receipt printer driver, no fiscal register, no card terminal — card
payments are Stripe's Payment Element inside a browser, and the "kitchen ticket"
is a pane in the owner console.

So the honest answer is not *"add virtual drivers"*. It is: **the discipline
transfers now, the devices are a market decision.**

### The discipline, adopted today

Every external actor is a **port** in `platform::Ports` with (a) a fake that is
the default in tests, (b) an explicit timeout, and (c) a two-phase state for
anything that can fail after it has had an effect. This is already the right
architecture for device drivers later; it is also the fix for failures that
exist **now**, because each hardware scenario the analysis named already has a
software twin in this tree:

| The POS scenario | Its live twin here | Status |
|---|---|---|
| Kitchen printer answers after 10 s | a half-open socket suppresses polling for 90 s | defect #20, open |
| The bank drops mid-payment | Stripe's webhook racing the client's confirmation | two-phase state exists, **untested** |
| The scanner sends garbage instead of an article number | `deny_unknown_fields` and an unknown product id | one is universal, one shipped accepting unknown option ids |
| The terminal is offline | `payment_error` → offer cash | works; no breaker (§6) |

**The second row is the one to act on.** A payment that is confirmed by the
browser and a webhook that arrives before, after, or never is a genuine two-phase
commit that this product performs several times a day and has no test for.

### If the devices are ever built

The one architectural consequence worth knowing *before* that decision: fiscal
regimes that mandate offline operation (Ukraine's ПРРО among them) typically
require a **pre-reserved range of receipt numbers usable without connectivity**.
That is a monotonic counter which must never re-issue a number and must survive
a lost response — a stronger guarantee than anything in this system today, and
one the hub object would have to own. It is a reason to build §2 properly rather
than approximately.

---

## 8. Correlation: three of the four pieces already exist

### What exists

`workers/api/src/otel.rs` is better than the analysis assumed:

- an incoming W3C `traceparent` is **continued, not replaced**, so a trace that
  started in the browser stays one trace;
- ids come from the platform CSPRNG, because a guessable trace id can be poisoned;
- **telemetry never fails a request** — every export path swallows its own errors;
- the module's own header names, honestly, what it cannot do: kernel spans cannot
  be stitched because `fdr::SpanObserver` hands out `(name, dur_us)` and nothing
  else.

A trace is begun once, at `lib.rs:70`.

### What is missing — and it is the part that makes it usable

The trace id **leaves no trace**. It is not in the response, so a customer or an
owner reporting a failure cannot name the thing that failed. It is not on the
`worker_errors` row. And it is not in the event envelope, so the **order log —
the only durable record of what happened — cannot be joined to it.**

Four lines of contract, all of which belong in `Ctx`:

1. `TraceId` on `Ctx` (already in the modular blueprint's §3).
2. Echoed as `x-trace-id` on **every** response, including errors. A 500 with no
   id is an unfindable 500.
3. Written on every `worker_errors` row, by `Fault::Unavailable`'s constructor.
4. Written into every appended event's envelope.

Then *"the black box of one order"* is one query on one id: the spans, the errors
and the events that order caused. That is exactly the utility the analysis asked
for, and it is now a join rather than a feature.

---

## 9. Time, clocks and timezones — including a defect that is live

### The question answered

> *How do you synchronise time between client terminals, the server and
> logistics timestamps, especially across short connectivity losses?*

**By not synchronising anything.** The design already refuses the problem in the
only way that works:

- the kernel has **no clock** (MANIFESTO C2) — `now_ms` is injected, so every
  decision is a function of a time that was supplied, not read;
- every stored instant is `..._at_ms`, an **i64 of UTC milliseconds**, stamped by
  the Worker;
- the Worker's clock is Cloudflare's, so there is **one authoritative clock per
  request** and no NTP problem on the server side at all.

The rule that must be written down and gated, because it is currently a habit
rather than a contract:

> **A device sends a sequence number and an idempotency key. The server supplies
> the time. No `_at_ms` field is ever populated from a request body.**

There is then no clock skew to reconcile, because a phone's clock is not an
input. When offline writes arrive (§6), the device will have to carry *something*
— and the right something is not a wall clock but
`(idempotency_key, device_seq, elapsed_since_capture_from a MONOTONIC source)`.
The server stamps `received_at_ms` and stores the device's claim separately as
`captured_before_ms`, **labelled as a claim and never merged with the server's
stamp**. Two timestamps, two meanings, never one column. (This is *Designing
Data-Intensive Applications* ch. 8 applied literally: a wall clock is a hint, a
monotonic clock is a measurement, and confusing them is how ordering breaks.)

For logistics: a courier's GPS point and a `DELIVERED` transition are stamped on
receipt by the server, not by the phone. Keep it that way and the "logistics
timestamps" question never becomes a distributed-clock question.

### The live defect

Local time is a **hard-coded constant in three places**:

| Site | Code |
|---|---|
| `owner.rs:620` | `let tz_offset_ms: i64 = 2 * 60 * 60 * 1000; // Europe/Tirane, standard time` |
| `storefront.rs:249` | `// Durrës is UTC+2. A Worker has no timezone database and needs none` |
| `extra.rs:27` | `// Europe/Tirane. A UTC day boundary would reset an owner's takings at one or…` |

**Europe/Tirane is UTC+1 in winter and UTC+2 in summer.** The constant is the
*summer* offset, and the comment calls it standard time. From the last Sunday of
October to the last Sunday of March — **from Sunday 25 October 2026, 34 days from this writing** — the
venue's "today" begins an hour early. Three things are wrong for five months of
the year: the owner's daily takings boundary, the schedule's open/closed decision
at the edges of the day, and the daily analytics bucket. Nobody has seen it
because the platform has not yet had a winter.

### The fix, which is the plan's own shape

A **pure function in the kernel**, no timezone database, no float:

```rust
/// The EU rule: +1h from 01:00 UTC on the last Sunday of March
/// to 01:00 UTC on the last Sunday of October.
pub fn offset_ms(tz: Tz, utc_ms: i64) -> i64;
```

Table-tested on both transition instants and one second either side of each, for
several years. A `venue.timezone` setting replaces the constant at all three
sites **in one commit** (repo rule 10: every reader moves together). And the gate
is a simulation: walk a year at hourly steps and assert the local day boundary
moves exactly twice.

This is also the cleanest possible demonstration of the verification pyramid's
central claim — a decision tangled into a handler was untestable and wrong; the
same decision as a function of its inputs is three lines and exhaustively testable.

---

## 10. Properties and fuzzing: the tools are installed and pointed elsewhere

### What exists

`proptest = "1.11"` is a dependency of both `crates/dowiz-core` and `kernel`.
`kernel/fuzz` is a real cargo-fuzz harness with targets `fuzz_trinary` and
`fuzz_eigen`. **Both are aimed at the mathematics. Neither touches money, stock,
tenancy or the order machine** — which is where every one of the thirty defects
lived.

### The invariants, stated against this codebase

These are laws, not examples. Each names the thing it would have caught.

| Property | Catches |
|---|---|
| `apply(a, delta(a,b)) == b` for every key any writer produces | the deleted `payment_intent`; the line that lost its `name` |
| the journal nets to zero over any sequence of top-ups, spends, refunds | a money bug in either direction |
| `stranded()` is empty after any interleaving of placements and transitions | stock held for ever by an order with no exit |
| folded stock level `== opening + deliveries − consumption` | the ledger being silently switched off |
| a customer balance never goes negative under any interleaving | concurrent spend |
| `fold(events) == stored status` for every archived log | the replay gate; divergence between the two truths |
| `upcast` is total over the corpus | the analytics export failing on old receipts |
| the `content_id_chained` chain holds under any append order | tampering |
| exactly `min(N, k)` of N concurrent placements succeed against stock `k` | §1 |

### The two fuzz targets worth adding

1. **`crates/dowiz-core/src/json.rs`** — a hand-written JSON parser is the single
   highest-value fuzz target in this repository, and it is on the path of every
   request.
2. **`Hub::load` over arbitrary bytes.** The image arrives over a network from
   the object. A corrupted image must **refuse loudly** — never panic, and above
   all never truncate silently into a shorter valid-looking log. This is a
   security property, not a robustness nicety.

---

## 11. What this adds to the pyramid

The verification pyramid has five layers. This blueprint adds one and widens two;
the amendment is recorded in that document's §7.

- **L2 widens** to hold every property in §10 and the upcaster corpus in §3.
- **L5 widens** to hold quarantine injection (§5) and breaker trips (§6).
- **L6 — adversarial and concurrent** is new: the collision probe (§1), the
  idempotency replay matrix (§2), the two fuzz targets (§10), and the year-long
  timezone simulation (§9). It is the only layer whose tests are *generated*
  rather than written, which is precisely why it finds what review does not.

---

## 12. Order of work, interleaved with the modular plan

The modular blueprint's phases stay as they are. These slot in where they are
cheapest, and four of them are wiring rather than building.

| # | Item | Size | Lands with |
|---|---|---|---|
| 1 | **The timezone function and its three call sites** | 1 day | phase 0 — it is live and dated |
| 2 | **Nightly `chain_check`** | 1 day | phase 0 |
| 3 | **`x-trace-id` on every response + on `worker_errors`** | 1 day | phase 1 (`platform/`) |
| 4 | **`Idempotency-Key` middleware**, placement first | 3 days | phase 1 — it is the `Ctx` contract |
| 5 | **The collision probe and the stock property** | 2 days | phase 1 |
| 6 | **Breakers on the three outbound rails** | 2 days | phase 1 |
| 7 | **Quarantine + health gauge** | 2 days | phase 2 |
| 8 | **The upcaster chain and `contracts/corpus/`** | 3 days | phase 1, beside the schema derive |
| 9 | **The witness in the S3 manifest** | 1 day | phase 6 (`operations/`) |
| 10 | **The two fuzz targets** | 2 days | any time; they are independent |

**Item 1 is first because it is the only one with a deadline**, and the deadline
is Sunday 25 October 2026.
