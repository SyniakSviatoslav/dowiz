# Blueprint — no SQL: bebop, Rust, wasm and nothing else

**2026-09-21, operator directive.** D1 is removed. Twenty-nine relational
tables and roughly thirty indexes move into bebop-store images held in Durable
Objects. Nothing in this system writes or reads SQL afterwards.

This is not a storage swap for its own sake. Three of this platform's defect
classes are SQL's shape showing through:

- **Six "venue chosen by a guess" defects** existed because a venue was a
  *column* that a query had to remember to filter on. `ORDER BY created_at_ms
  LIMIT 1` is a sentence that only a table can say.
- **`content_i18n` has no venue column at all**, and `d1-bind-limit-silent-i18n`
  records 187 ids in one `IN()` silently failing at the 100-bind ceiling.
- **`worker_errors` has never received a row**, and `sqlite_sequence` is how we
  know — an instrument whose liveness had to be inferred from the database's own
  bookkeeping.

`id_from_name(location_id)` gives each venue its own object and its own bytes.
**A venue that cannot be named cannot be read.** Tenancy stops being a filter and
becomes structural.

---

## 1. What "bebop everywhere" can and cannot mean

**bebop-lang compiles to raw AArch64 machine words**, loaded by a 1.5 KB assembly
seed. It cannot run inside a Cloudflare Worker, and pretending otherwise would be
the most expensive sentence in this document.

So "bebop everywhere" means what it already means at the one seam that works
today, extended to everything:

- **bebop owns the format and the schema.** `selfhost/std/kv.bp` creates the
  layouts; the digests are sha256 and that stays on the bebop side, so the Rust
  half never reimplements sha256 to write data.
- **The format is pointer-free on purpose** — nothing in the file is an address,
  every `ref` is an object-relative cell offset — which is precisely what lets
  Rust and bebop read and write the same bytes.
- **`crates/bebop-store` (zero dependencies) is the wasm half.** It compiles to
  wasm32 with the Worker and has no supply chain.
- **The four-way fold check is the contract between the halves**: bebop,
  bebop-store, `InMemoryStore` and python must fold the same entries to the same
  value. It has already caught a real defect, and the wrong side was Rust's.

**Rust and wasm carry the request. bebop carries the truth about the bytes.**

---

## 2. The two shapes, and choosing between them

`bebop-store` offers exactly two, and the choice per table is the whole design.

| | **Kv image** | **EvLog image** |
|---|---|---|
| Layout | sorted keys, `KIDX/KBLOB/VIDX/VBLOB` | chain of one-object records |
| Append cost | **O(n)** — rewritten eagerly on every put | **O(1)** — measured constant at 47 cells |
| Ceiling | doubling loop against `DEFAULT_*_BYTES` | capacity it was created with; then refuses |
| Reads | key lookup, prefix scan (keys are sorted) | fold, memoised per generation |
| For | small, mutable, keyed sets | anything that grows without bound |

**The rule that follows: anything that grows with usage is an EvLog. Anything
bounded by the venue's own size is a Kv.** Getting this backwards is how the
catalogue acquired the same 8× amplification the order log had.

And the gauge rule from `bebop-ceiling-not-capacity`: a Kv image's honest
denominator is its **ceiling**, never its current capacity — `used/capacity` on a
compacted image is a sawtooth that read 829 per mille on a venue that was under
10 % full.

---

## 3. Where every table goes

Twenty-nine tables, three homes. **The home is the tenancy boundary**, which is
the point of the exercise.

### 3.1 The venue's object — `id_from_name(location_id)`

Everything a venue owns. After this migration a venue's entire existence is its
own object's bytes, and a cross-tenant read requires naming the other tenant.

| Image | Shape | Holds | Replaces |
|---|---|---|---|
| `log` | EvLog | orders and their lifecycle | `orders` **(already done)** |
| `stock` | EvLog | movements, supplies | **(already done)** |
| `catalog` | Kv | categories, products, modifier_groups, modifiers, **content_i18n** | 5 tables + 5 indexes |
| `settings` | Kv | venue settings, features, branding, hours, zones, `venue_pass_keys` | 1 table |
| `posts` | Kv | posts | **(already done)** |
| `people` | Kv | `customers`, the venue's couriers and their roster rows | `customers`, `courier_locations` |
| `threads` | EvLog | `threads`, `thread_messages`, `channel_messages` | 3 tables + 4 indexes |
| `bookings` | EvLog | `reservations`, `reservation_events` | 2 tables + 3 indexes |
| `ledger` | EvLog | `ledger_tx`, `ledger_postings` | 2 tables + 3 indexes |
| `audit` | EvLog | `courier_audit_log`, reveals, **the venue's `worker_errors`** | 2 tables |

### 3.2 The platform object — `id_from_name("__platform")`

The same `HubImages` class, a different name. **No new Durable Object class and
no new migration**: the class is already `new_sqlite_classes` and its storage API
is the key/value one either way.

| Image | Shape | Holds | Replaces |
|---|---|---|---|
| `registry` | Kv | `locations`, `organizations`, host→venue, slug→venue | 2 tables |
| `identity` | Kv | `users`, `memberships`, `platform_admins` | 3 tables + 1 index |
| `sessions` | Kv | `auth_refresh_tokens`, `courier_sessions`, `owner_api_keys` | 3 tables + 5 indexes |
| `couriers` | Kv | `couriers` (phone→id), `courier_invites` | 2 tables + 3 indexes |
| `waitlist` | EvLog | `waitlist` | 1 table |
| `errors` | EvLog | platform-level `worker_errors` | 1 table |

### 3.3 Nothing

`sqlite_sequence` — SQLite's own bookkeeping, which this platform was reduced to
reading as an instrument. It goes with the database.

---

## 4. Indexes become keys, and that is the whole query language

There is no query planner and no `WHERE`. **Every access path is a key that was
written on purpose**, and the sorted-key property of the Kv layout gives prefix
scans for free.

```
identity image
  user/<id>                         → the record
  user.email/<lowercase email>      → <id>          UNIQUE: presence check before write
  member/<location_id>/<user_id>    → role|status   the memberships_lookup_idx, as a prefix
  member.by_user/<user_id>/<loc>    → role|status   the other direction, written together
  admin/<user_id>                   → level

sessions image
  refresh/<token_hash>              → the record
  refresh.family/<family_id>/<created_at_ms>/<id> → ""   the family scan, as a prefix
  apikey/<hash>                     → location_id|scopes
  apikey.loc/<location_id>/<hash>   → ""            the owner's list, as a prefix
```

**Four rules make this safe rather than a pile of hand-maintained denormalisation:**

1. **Every index entry is written in the same put as its record.** One image, one
   transaction, and the object is single-threaded — so a record without its index
   is not a state this can reach.
2. **A uniqueness constraint is a presence check inside the object.** Safe for
   exactly the same reason, and it is the one thing a Worker holding an image
   could never do correctly.
3. **An index is derivable.** Every image carries a `rebuild_indexes()` that
   recomputes every derived key from the records, and a gate asserts that a
   rebuild changes nothing. That is the conservation audit applied to storage.
4. **A transaction touches ONE image.** Two images in the same object are two
   puts and two generations. Anything needing both is a saga and must be named
   one — which is the aggregate rule from the hardening plan, arriving as a
   storage constraint.

---

## 5. What is genuinely lost, and what replaces it

Stated plainly, because a migration document that lists only wins is a sales
document.

| Lost with SQL | What replaces it | Honest cost |
|---|---|---|
| ad-hoc `WHERE` over any column | a key written on purpose, or a fold | a new access path is a code change and a backfill, not a query |
| `JOIN` across tables | the join is done where the data lives, in one object | cross-object joins become explicit sagas — which they always were |
| `ORDER BY` on any column | sorted keys (one order, chosen) or a fold that sorts | a second sort order is a second index |
| the query planner | nothing; every path is explicit | **this is a win**: no plan can silently change |
| `sqlite_sequence` as an instrument | the EvLog's own record count and chain tip | **a win**: it was never an instrument, it was an accident |
| D1's 100-bind ceiling | gone | **a win**: it failed silently and cost i18n for every venue |
| migrations as `.sql` files | `grow()`, version in the root's 8th cell, upcasters on read | the discipline is stricter and already written down |

**The one thing to watch:** the platform object is a single serialisation point
for every login on the platform. At two venues this is free. The sharding plan,
named now and deferred deliberately: images that grow with the number of *people*
(`identity`, `sessions`) shard as `__platform:<nibble>` on the first hex digit of
the key's sha256; `registry` never shards because it must answer "which venue is
this host" in one read.

---

## 6. The migration, family by family

Each family is expand → backfill → verify → contract, and the tree deploys at
every step. Never more than one family in flight.

**Step A — the storage layer.** `platform.rs`: the bebop-image accessor over the
platform object, the same shape `hubstore` already has for a venue, plus the Kv
record/index helpers and `rebuild_indexes`. No behaviour change, nothing reads it
yet.

**Step B — per family, in this order.** Ordered by blast radius ascending, so the
mechanism is proved on something that cannot lock anyone out:

1. `waitlist`, `worker_errors` — append-only, nobody's login depends on them.
2. `content_i18n` + catalogue — **fixes the missing venue column structurally**
   and removes the 100-bind failure.
3. `threads`, `channel_messages`, `reservations`, `ledger` — venue-scoped folds.
4. `customers`, `couriers`, `courier_*` — the courier login path.
5. `registry` (`locations`, `organizations`) — host→venue resolution.
6. `identity` + `sessions` — **last, because a bad cutover locks everyone out.**

**Per family:**
- write the image's records, keys and `rebuild_indexes`, with its `tests.rs`;
- **backfill** from D1 in a one-shot admin route, idempotent, resumable;
- **verify**: a fold of both sides compared entry by entry, loud on any
  difference — the four-way check's discipline, applied per family;
- **cutover**: read bebop. For families 5 and 6 only, one deploy of **read bebop,
  fall back to D1, log every mismatch loudly** before the fallback is removed;
- **contract**: delete the D1 code, the table and its indexes.

**Step C — the last commit.** Remove `[[d1_databases]]` from `wrangler.toml`,
delete `workers/api/migrations/`, delete every `.prepare(` in the tree. A gate
asserts the string `prepare(` and the word `SELECT` do not occur in
`workers/api/src/**` — the file-size ratchet's mechanism, applied to SQL.

---

## 7. The gates this adds

| # | Fitness function | Kind |
|---|---|---|
| F26 | **no SQL**: `prepare(`, `SELECT`, `INSERT INTO` do not occur in `workers/api/src/**` | atomic · triggered |
| F27 | `rebuild_indexes()` changes nothing, for every Kv image | atomic · triggered |
| F28 | **four-way fold** over each migrated family: bebop, bebop-store, the live image, python | holistic · triggered |
| F29 | backfill is idempotent: running it twice yields the same image bytes | holistic · triggered |
| F30 | every image's gauge is measured against its **ceiling**, and never falls as the image grows | atomic · triggered |
| F31 | a transaction touches exactly one image | atomic · triggered |

---

## 8. Why this is the right time

The hardening plan's phase 1 builds `platform/` — `Ctx`, `Ports`, `Fault`,
`Handler`. **`Ports` is where a database handle would have gone.** Building that
abstraction over D1 and then replacing D1 underneath it would mean designing the
port twice, and the second design would be shaped by the first mistake.

The two plans therefore merge: **`Ports` has no SQL handle. It has a venue image
port and a platform image port, and that is the only storage a service can see.**
