# bebop as what it is: data-oriented design from games, applied pattern by pattern to this tree

**Date:** 2026-09-23. **Tree read:** the working tree of `/root/dowiz` and `/root/dowiz/bebop-lang`;
no git command was run by this lane. Companion: `BLUEPRINT-BEBOP-IN-WASM-2026-09-23.md` (where
bebop can run; this document is about what it is FOR).

**Method.** As the companion: `file:line` for every claim about the tree, `measured` for numbers a
command produced here, `hypothesis` otherwise. For each pattern the same four questions: what is
the pattern, what would it change HERE, what it costs, and **whether the Rust side already has the
benefit** — because "we already do this without saying so" is the most common honest answer and
this repo has hit it several times (STACK §11.7; ARCHITECTURE-EVOLUTION P6 "naming it LWW costs a
comment, not a crate").

---

## 0. Thesis, in ten lines

1. A language with **no runtime strings, no GC, integer-only arithmetic and an arena** is not a weak
   language; it is a specific one, and the discipline built on exactly those constraints is
   **data-oriented design** (DOD) as game engines practise it: memory layout first, entities as
   indices, systems as loops, tables over branches, no allocation in the frame.
2. **The tree already practises about half of it, unnamed** (§3 scoreboard): the store is a bump
   arena with a pointer-free layout (`bebop-store/src/lib.rs:1-22`, `:497-512`); the KV is SoA
   (four parallel arrays, `kv.rs:3-7`); money, zones and ETA are fixed-point with lookup tables
   (`zone.rs:33-45`, `eta.rs:320-334`); the object's memo is a hot/cold split (`hubdo.rs:203-210`);
   features, hours, the floor plan and modifiers are data the owner edits (`features.rs:1-30`,
   `hours.rs:1-16`, `tables.rs:172`, `modifiers.rs:201`).
3. Where it does NOT: the ledgers and plans that the product asks predicates of are **AoS with linear
   search** (`stock.rs:139-150` `Vec<(String, StockLevel)>` + `.iter().find`, 13 such sites across
   `dowiz-hub`, measured), and **no bitset exists** anywhere in the hub or the Worker (measured).
4. **ECS fits the order fold exactly and the stock ledger nearly**; it does not fit the table plan,
   which is a hundred rows and a nested loop that is already the right shape (§2.2).
5. **A "frame" for a Durable Object turn is one request**: everything the turn allocates dies with it,
   and the image it mutates is the persistent arena. bebop's 16 KiB frame heap and its
   never-freed `zeros` arena are the same two lifetimes (§2.3).
6. **Bitsets buy little at this scale** and cost a vocabulary; the predicates the product asks a
   thousand times are over tens of orders, not tens of thousands (§2.4). Measured before adopted.
7. **Deterministic fixed point is already law** (MANIFESTO C2 `MANIFESTO.md:15`; `clock.sh` gate); the
   one seam is the `f64` tax/FX rate adapter (`money.rs:270-297, 345`) that B1 closes. bebop's Q32
   plan (`LANGUAGE.md` "Fractions") is the same decision reached from the other side.
8. **Data-driven beats a branch wherever the venue is the author** — price per channel (B3), the
   floor plan, hours, allergens — and loses where the LAW is the author: the order FSM, money
   rounding, tax precedence. The line is "who may change it without a deploy" (§2.7).
9. **The cheapest real gains** (§6): name what exists (comments and one doc), replace the four
   `find`-on-a-`Vec` ledgers with sorted indices only when a measurement says so, and write the
   ETA and detour tables the companion's §6 asks for. No new crate, no new language surface.
10. **AGAINST**: a general ECS framework, bitset-everything, and any move of the fold into `.bp`
    before A7/A8 land (§4).

---

## 1. The language as it is — the recorded constraints, each with its line

| Constraint | Where recorded | What it means for a design |
|---|---|---|
| every value is an `i64`; arrays are cell addresses; types parsed and discarded | `docs/LANGUAGE.md:5-10` | layout is the type system; a record is a convention about offsets |
| no runtime strings: `str` is a literal view, `[u8;N]` the buffer; a buffer built at runtime as `str` is SIGSEGV | `LANGUAGE.md` "Strings"; session memory `bebop-str-cannot-be-built-at-runtime` | text is bytes in cells; the caller owns the buffer |
| `zeros(n)` bumps a 256 MB arena and never frees; array literals live in a 16 KiB per-call frame that dies at return; `while` bodies reset per iteration | `LANGUAGE.md` "Memory model" | two lifetimes only: forever and this-frame |
| `>>` is LOGICAL, `>>>` arithmetic (inverts C) | `LANGUAGE.md` "Expressions … shifts"; memory `bebop-shift-right-is-logical` | branchless masks must be written with `>>>` for signs |
| `&&` had a precedence defect (A26 promoted; the `no_andand` gate is the stale side) | memory `bebop-andand-precedence` | predicates as arithmetic (`a * b`, `1 - x`) are the safe idiom, and the tree writes them that way (`bebop.bp:5006`) |
| a function spanning `sys_clone` keeps at most EIGHT symbols; children are lost silently past it | `AGENTS.md:356-359` | pack state into one `[i64]` — i.e. a struct of arrays by hand |
| `if` is an expression, no braces; one-cell mutables (`let x = [0]`) | `LANGUAGE.md` "Statements", "Expressions" | state lives in cells, not variables — DOD's "data, not objects" enforced by syntax |
| files cap at 800 lines; top-level functions only; derive constants in source | `AGENTS.md:347-366`; CLAUDE.md "three laws" | small named systems over one big update loop |
| 14 parameters; a 15th is a silent `brk #8` | `LANGUAGE.md:29` | context travels as one array handle, again SoA |
| integer `/` and `%` are hardware-truncating, `x/0 = 0` | `LANGUAGE.md` "Expressions" | fixed point needs explicit rounding; the kernel's money law already states its own |

None of these is an accident of a young compiler. Together they are the C-with-no-malloc that game
engines are written in for hot paths, minus the compiler's freedom to hide a layout from you.

---

## 2. The patterns, one at a time

### 2.1 SoA over AoS — and what the image format already does

**The pattern.** Store each field of N records in its own contiguous array, so a system that reads one
field touches N cells, not N records; and so an index can point at a row across all fields.

**What the store does about it, measured from the layout.** The KV image is SoA by construction:
root `KV{n, ref KIDX, ref KBLOB, ref VIDX, ref VBLOB}` — four parallel arrays, `(offset, len)` per row
into a blob, keys sorted (`kv.rs:3-7`; `kv.bp:11-16`). A read of "all keys" touches `KIDX` and
`KBLOB` and never a value (`kv.rs:116-142`). The event log is the opposite: one object per record
with its header, ids and payload inline (`evlog.rs:24-33`), because the log's system is "walk the
chain newest-first", which reads whole records. Both are right for their access pattern, which is the
DOD test.

**Where the Rust side is AoS and pays for it.** `StockLedger` is `levels: Vec<(String, StockLevel)>`
and `open: Vec<((String, String), Qty)>` (`stock.rs:139-143`), looked up by `.iter().find(|(i,_)| i == item)`
(`:146-150`) and `.iter().position` (`:166`); the same shape in `catalog.rs` (5 `Vec<(String…`),
`table.rs` (5), `roster.rs` (2), and 13 `.iter().find`/`.position` sites across `dowiz-hub/src`
(measured `grep -c`). `Kv::get` is a linear scan too (`kv.rs:147-149`) while `put`/`remove` use
`binary_search_by` (`:157-175`) — the entries ARE sorted and one accessor does not use it.

**What it would change here.** For the stock ledger: `items: Vec<String>` (sorted), `levels:
Vec<StockLevel>`, `open_order: Vec<u32>`, `open_item: Vec<u32>`, `open_qty: Vec<Qty>` with a binary
search on `items`. `decide` (`:180`) becomes an index lookup; `stranded` (`:357`) a scan of one
array.

**Cost.** ~150 lines in `stock.rs` (hypothesis), tests unchanged in intent; `items()` already sorts
for determinism (`:157-162`), so the sorted invariant is free.

**Does Rust already have the benefit?** No — but does it NEED it? Measured scale: 165 dishes at the
largest venue (memory `sushi-durres-menu-source`), 0 recipes and 0 supplies on both venues (memory
`dowiz-stock-ledger-works-but-is-off`). A linear scan over 165 strings is ~2 µs; the fold that
precedes it walks a log of thousands of records and parses JSON per order (`hubstore.rs:1268-1298`).
**The AoS ledger is not the hot path. Verdict: name it, do not rewrite it, until a venue with
recipes measures it.** The one free fix is `Kv::get` using the sort it already maintains.

### 2.2 ECS — entities as indices, components as parallel arrays, systems as loops

**Which of the three candidates fits.**

- **The order fold: fits exactly.** An order is an entity named by `order_id`; the events are
  components arriving over time (`EventKind` `Placed/Advanced/…`, `dowiz-hub/src/lib.rs:102-130`);
  `orders_state` (`hubstore.rs:1268-1298`) IS the system: one pass, `HashMap<String, usize>` from id
  to newest index, `HashMap<String, Value>` for the folded state, then sort newest-first. The only
  ECS-shaped change is the KEY: ids are `String`s hashed per event; an interned `u32` per order
  (assigned at first sighting) turns three hash maps into three `Vec`s indexed by that number.
  **Cost:** ~40 lines; the fold's output is unchanged. **Benefit:** measurable only on a long log;
  the object memoises the fold per generation (`hubdo.rs:423-437`) so it runs once per write, not
  per poll. **Verdict:** fits, cheap, not urgent; do it when the fold is timed on a 20,001-event log
  (`STACK` §10 names the missing measurement).
- **The stock ledger: fits nearly.** Entities are `(item)` and `(order, item)`; `fold(events)`
  (`stock.rs:344`) is a system; `decide` (`:180`) is the same system asked "would this event fold".
  The near-miss is that its events name items by string and its output must be byte-identical across
  folds (I4, `:157-162`), so the index must be the sorted position, assigned by a first pass. §2.1's
  layout is the ECS form.
- **The table plan: does not fit.** `availability(plan, held, party, slot, dwell)`
  (`tables.rs:276-294`) is two nested loops over zones and tables, each asking `holder(held, …)` —
  a plan is a few dozen tables (`PLAN_W/PLAN_H` `:23-24`, `MAX_SEATS 20` `:28`) and `held` is
  tonight's bookings. Components would be `occupied[t]`, `too_small[t]`; the systems would be the
  same two loops. ECS here is the code that exists with new nouns. **Verdict: already the shape;
  say so in the header.**

**Does the Rust side already have the benefit?** Systems-as-loops: yes, everywhere (`fold`,
`orders_state`, `availability`, `ledger`). Entities-as-indices: no; ids are strings hashed on every
step. That is the whole ECS gap, and it is one interning pass.

### 2.3 Arena / bump allocation and frame lifetimes — what a "frame" is for a Durable Object turn

**The store IS an arena.** `Store::alloc` bumps `tx.cursor`, writes `h0`/`h1`, never frees
(`lib.rs:495-512`); "nothing is freed" is the same rule as bebop's `zeros` (`LANGUAGE.md` "Memory
model"); reclamation is a whole-image rebuild — `Hub::grow` copies the chain into a fresh image
(`lib.rs:458-488`), `Kv::compacted_bytes` rewrites the map into a fresh one (`kv.rs:272-283`), and
bebop's Cheney compaction (`LANG-DB-DESIGN.md` §4e) is the same move.

**Two lifetimes, named.** bebop has exactly two: the arena (forever, `zeros`) and the frame (16 KiB
per call, dies at return; `while` bodies reset per iteration, `LANGUAGE.md` "Frame heap"). The
Durable Object has exactly two as well, and they map:

| bebop | Durable Object | where |
|---|---|---|
| arena (`zeros`, never freed, msync'd to the file) | the images in `mem: RefCell<HashMap<String,(Meta,Vec<u8>)>>` (`hubdo.rs:197`), written back in 96 KiB chunks (`:40-43`) | the persistent state, mutated only by the single writer |
| frame (per call; literals and ctors; released at return) | one request turn: `Hub::load(&bytes)` (`:432`), the folded `Vec<OrderView>`, the JSON, the response — all dropped when the handler returns | everything a turn allocates |
| `while`-body reset (T43) | the memo `folded: RefCell<Option<(i64, Vec<OrderView>)>>` (`:210`) — a per-generation value that survives turns but is discarded whenever the generation moves | the one thing between the two lifetimes |

**What a frame discipline would change here.** The turn allocates a fresh `Hub` from the bytes on
every fold (`Store::from_bytes` copies the image into `Vec<i64>`, `lib.rs:131-137`, 8 bytes per cell),
then drops it. The image is already in `mem` as `Vec<u8>`; a reader over `&[u8]` with no copy is the
frame-free form. **Cost:** a `Store<'a>` borrowing `&[i64]` — `from_bytes` needs an aligned view or
a `u64::from_le_bytes` per cell read; ~100 lines in `bebop-store` (hypothesis) and every `Hub`
signature. **Benefit:** one image copy per fold saved (a 4 MiB default image = 4 MiB of allocation
per cold fold, `lib.rs:60`). **Does Rust already have it?** No. **Verdict:** worth it only if a
cold-fold timing shows the copy, not the walk, dominates — measure first (`STACK` §10 row 2).

### 2.4 Bitsets and branchless code — for the predicates asked a thousand times

**The predicates.** "Is this order live" — `EventKind::is_order` (`lib.rs:117`), `is_over(status)`
derived from the kernel's `is_terminal` (`rebuild.rs:56-60`; `order_machine.rs:70`); "does it hold
stock" — `StockLedger::stranded` (`stock.rs:357`), `open` reservations; "is this table free at this
slot" — `holder(held, zone, n, slot, dwell)` (`tables.rs:253`), `holds_table(status)` (`:246`).

**How bebop writes them.** Without `&&` for years, the compiler itself is branchless by habit:
`is_cimm = (if idxk == 1 then 1 else 0) * (if idxc >= 0 then 1 else 0) * (if idxc < 4096 then 1 else 0)`
(`bebop.bp:5006`); `csel` emission for `if` when both arms are cheap (`emit_cond_csel :4665`).
Bitsets are a `[i64]` and `>>>`/`&` — which is why the shift convention matters.

**Would a bitset change anything here?** Scale, measured: a venue's live orders are tens; a plan is
tens of tables; the roster is tens of people. A `u64` per status set would turn `is_over(status)`
from a string match into a mask test — but `status` arrives as JSON text and must be parsed first
(`fold.rs`), so the mask saves the last 10 ns of a 10 µs operation. The place a bitset is genuinely
right is the one the tree lacks: **a per-table occupancy mask per slot** for the booking calendar,
if bookings ever become "show me every free table across the evening" (N tables × M slots). Today
`availability` is asked for one slot (`tables.rs:276-280`).

**Does Rust already have the benefit?** Branchless: the compiler does it where it matters, and none
of these paths is a tight loop. Bitsets: none exist (measured `grep "1u64 <<\|bitset\|count_ones"` →
only `crypto.rs:172`). **Verdict: AGAINST introducing bitsets now** (§4); re-entry is a multi-slot
availability query or a live-order count in the thousands.

### 2.5 Deterministic fixed point — which the kernel already requires

**Already law.** MANIFESTO C2 (`MANIFESTO.md:15`): "no clock/RNG/env/floats/network … reaches the
kernel"; `tools/gates/clock.sh` ratchets clock sites; money is `i64`/`i128` (CLAUDE.md "Money is
exact integer arithmetic"); coordinates are micro-degrees with a cosine TABLE (`zone.rs:33-45` "A
TABLE rather than a call, because `cos` is a float function"); ETA's `cos_scaled` and `isqrt`
(`eta.rs:320-346`) are integer.

**The seam.** `apply_tax(subtotal, tax_rate: f64, …)` and `convert_all_to_eur_cents(amount, rate: f64)`
(`money.rs:297, 345`) — "THE `f64` ADAPTER, and nothing more" (`:270-279`) — B1 on the roadmap
closes it (`ROADMAP-2026-09-22.md` Phase B); the `router.rs` port is f64 throughout (companion §6).

**bebop's version of the same decision.** `LANGUAGE.md` "Fractions: Q32 fixed point, NOT IEEE-754
and NOT posit" with `smulh`/`umulh` as "the exact-arithmetic half of a Q32 fixed-point multiply";
25 oracles and a first theorem. Two projects, one conclusion, reached independently.

**What it would change here:** nothing new — the pattern is complete on the decision path once B1
lands. **Does Rust already have it?** Yes. The honest addition is a sentence in `money.rs`'s header
naming the bebop Q32 row as the same law, so the next reader does not propose floats "because bebop
will have them".

### 2.6 Hot/cold splitting — the memoised fold is hot, the archive is cold

**Already built, three times.** (1) `folded` memo per generation (`hubdo.rs:203-210`): "re-folding an
unchanged log is the same answer computed again". (2) `Hub::rotate(keep)` moves old orders out of the
hot image and hands the whole image back "for the caller to store somewhere cold" (`lib.rs:607`),
leaving a CHECKPOINT record. (3) The nightly S3 copies (memory `dowiz-integrations-2026-09-19`).
Plus the log's own v2 layout packed 8 bytes per cell precisely to keep the hot image small
(`evlog.rs:35-38`, "583 cells — 4.66 KB — per event").

**What DOD adds.** The split is by TIME here (recent vs archived). The game-engine split is by
ACCESS: keep the fields the hot system reads in one array and everything else elsewhere. Applied to
the log: the fold reads `kind`, `order_id` and the delta JSON of every event; it does not read
`actor_pubkey`, `id`, `prev` (32 bytes each) except in `chain_check`. A **hot index image** — per
order: newest offset, status byte, placed-at — would let the poll path answer "the queue" without
walking or parsing; the object would maintain it on append. **Cost:** a new image family (~200
lines, hypothesis) and a rebuild-and-diff gate for it (the `rebuild.rs` shape). **Benefit:** the cold
fold becomes an index scan; the warm path is unchanged (the memo already serves it).
**Does Rust already have the benefit?** The memo gives it to every reader after the first per
generation; the cold cost is paid once per write. **Verdict:** the second projection is a real item
IF the cold fold is measured as the cost on the live venue; `STACK` §10 says it was not measured.
Not before.

### 2.7 Data-driven design over code — where a TABLE beats a branch, and where it does not

**Already tables, owner-edited:** feature flags as a declared registry (`features.rs:1-30`: "the
surfaces read the list rather than a hard-coded set"); opening hours as a weekly schedule the flag is
DERIVED from (`hours.rs:1-16`); the floor plan as JSON the owner saves (`tables.rs:172` `from_json`,
"rewritten whole on every owner save"); modifier groups with integer deltas (`modifiers.rs:201`);
promos (`promo.rs`); allergens (`allergens.rs`). The product is already data-driven where the venue
is the author.

**Where a table beats a branch (the roadmap's three):**

| Row | Table | Why a table | Where it does NOT go |
|---|---|---|---|
| B3 price per channel (`ROADMAP-2026-09-22.md:109`, tax lane) | one key per EXCEPTION to the base price: `x/price/<product>/<channel>[/<fulfilment>]` (STACK §9.2(1) already sized the dense form at 1.1 MB per venue and rejected it) | the venue sets it; a deploy must not be needed | **precedence** when promo, channel and segment all apply is a RULE in code with a test, never a row |
| the floor plan | exists (`tables.rs`) | same | `seats_party`, `MAX_SEATS`, "a box that leaves the room is refused" are code |
| the recipe (stock) | `recipe: [(item, qty)]` per dish read from the catalogue record (`stock.rs:951-954`) — exists | same | `decide`'s refusal on `OutOfStock` (`stock.rs:19-21`) is law |
| ETA speed and detour (companion §6) | per venue `m_per_min`, `detour_e3`, per zone optionally | measured from the venue's own deliveries; overridable | the estimate's arithmetic and its overflow checks (`eta.rs:219-258`) |

**Where a table does not beat a branch:** the order FSM (`order_machine.rs:14-70`, "Forbidden
transitions are errors"), money rounding, tax precedence, the allergen gate, consent. The tree
already states the rule: "A FLAG NEVER GUARDS CORRECTNESS" (`features.rs:16-19`). The line is
**who may change it without a deploy** — the venue for tables, the law for branches.

**Does Rust already have the benefit?** Yes, for everything the venue edits today. The gap is the two
tables the roadmap has not written (price-per-channel key layout, at the tax lane; the ETA table,
nobody's yet).

---

## 3. Scoreboard

| Pattern | Rust side has it? | Gap | Cost to close | Worth it now? |
|---|---|---|---|---|
| SoA over AoS | store: yes (KV); hub: no (13 `find` sites) | `Kv::get` ignores its own sort; ledgers are AoS | 1 line; ~150 lines | the 1 line yes; the rest after a measurement |
| ECS | systems yes; entity-as-index no | string ids hashed per event | ~40 lines interning | when the cold fold is timed |
| Arena / frame | arena yes; frame yes, with one copy per fold | `from_bytes` copies the image | ~100 lines, API change | after a cold-fold timing |
| Bitsets / branchless | branchless where it matters; bitsets none | none needed at this scale | — | AGAINST (§4) |
| Deterministic fixed point | yes; one f64 seam (B1) | router is f64 | B1 | already scheduled |
| Hot / cold | yes by time (memo, rotate, S3); not by access | no hot index image | ~200 lines + a gate | if the cold fold is the measured cost |
| Data-driven | yes where the venue is the author | B3 key layout; ETA table | tax lane; ~120 lines | B3 in flight; ETA yes |

---

## 4. Recommended AGAINST, with re-entry conditions

| # | Against | Why | Re-entry |
|---|---|---|---|
| D1 | An ECS framework crate or a general "component store" | one interning pass gives the whole benefit; a framework adds a vocabulary to the fourteen-copies problem (`ARCHITECTURE-EVOLUTION` §1.7) | never as a crate; the pass itself when §2.2's timing exists |
| D2 | Bitsets for status/stock/table predicates | tens of rows, string-parsed inputs; a mask saves nothing measurable and inverts `>>`/`>>>` habits across two languages | a multi-slot availability query or thousands of live orders |
| D3 | A borrowing `Store<'a>` now | API change across `dowiz-hub` for a copy nobody has timed | cold-fold timing shows the copy ≥ 20 % of the turn |
| D4 | Moving the fold, or any JSON, into `.bp` | no runtime strings (`LANGUAGE.md:8-9`); `fold.rs:17-22` needs a real parser | A7 step 2 + A8 tag 7 landed, and a `.bp` JSON reader passing `fold.rs`'s tests byte-for-byte |
| D5 | A dense price tensor | STACK §9.2(1): 1.1 MB per venue for 95 % base prices; precedence is a rule | never dense; the key-per-exception table is B3 |
| D6 | Applying the 800-line/`.bp` file cap and the eight-symbol rule to Rust as-is | Rust has closures and a stack; the Rust ratchet is 300 lines (`tools/gates/file-size.sh`) and it already only falls | — |

---

## 5. Gates

| Gate | Value | Fails when | Proved both ways |
|---|---|---|---|
| E1 `file-size.sh` (exists) | `over`/`worst` ratchet | a Rust file grows past 300 | yes (`tools/gates/file-size.sh:44-58`) |
| E2 `clock.sh`, `float-money.sh` (exist) | clock sites, float-on-money sites, only down | a new site | yes (`:92-99`, `:120`) |
| E3 (proposed) `linear-find.sh` | count of `.iter().find(\|(` and `.iter().position(\|(` over `dowiz-hub/src` — baseline **13** (measured) | a new AoS lookup | write the baseline, add one, watch it refuse, remove it |
| E4 (proposed) cold-fold timing | ms to `Hub::load + orders_state` on the 20,001-event log of the hub-cost blueprint; recorded, not gated, until two runs agree | — | the number moves when the log doubles |
| E5 (proposed, with the companion's G4) ETA table miss rate | `|quoted − actual| ≤ spread` on the last N deliveries | a venue's misses rise | the number moves when the speed row is edited |

---

## 6. Order of work, each with its CHECK

1. **Name what exists** — headers in `stock.rs`, `tables.rs`, `hubdo.rs` (memo = hot/cold),
   `money.rs` (Q32 is the same law): the patterns above, in one sentence each, with the file this
   document is. **CHECK:** `grep -n "data-oriented\|SoA\|hot/cold" crates/dowiz-hub/src/*.rs
   workers/api/src/hubdo.rs` finds four headers. (Main session or the owning lanes — `stock.rs` is
   not this lane's.)
2. **`Kv::get` by binary search** (`kv.rs:147-149` → `binary_search_by`, as `put` does). **CHECK:**
   `crates/bebop-store` tests green; `crates/bebop-wasm/gate.sh` still `4 of 4` (the root must not move).
3. **E3 `linear-find.sh`** with baseline 13. **CHECK:** proved both ways as in §5.
4. **The ETA table** (companion item 4): per-venue `m_per_min`, `detour_e3` folded from delivered
   orders, as settings keys. **CHECK:** companion G4 / E5.
5. **E4 cold-fold timing** on the 20,001-event log. **CHECK:** two runs within 10 %; the number
   decides D3 and §2.6.
6. **Order-id interning in `orders_state`** (§2.2), only if item 5 says the fold is the cost.
   **CHECK:** byte-identical `Vec<Event>` output on the same log; timing before/after.
7. **Hot index image** (§2.6), only if item 5 says the cold fold is the cost and item 6 was not
   enough. **CHECK:** a `rebuild.rs`-shaped diff of index vs fresh fold, empty.

---

## 7. What was NOT determined, and the command that would settle it

| Claim | Status | Command |
|---|---|---|
| The cold fold's cost on a real log (decides D3, §2.6, items 6-7) | not measured; `STACK` §10 also lists it | `time curl …/fold/orders` cold vs warm on the hub-cost blueprint's 20,001-event image; or a `#[test]` timing `Hub::load + orders_state` |
| That a linear `find` over 165 dishes is ~2 µs | hypothesis from scale | a criterion bench in `dowiz-hub` |
| The 13 `find`/`position` sites are all lookups by key (not filters) | counted, not read one by one | `grep -n "\.iter()\.find(\|\.iter()\.position(" crates/dowiz-hub/src/*.rs` and read each |
| That `csel` emission (`emit_cond_csel`) covers the predicate shapes in `store.bp` | inferred from the function's existence | `bench/vs_rust/invariants.sh` branch census on `kv.bp` |
| Whether `Store::from_bytes`'s copy is the dominant cost of a cold fold | hypothesis | the E4 timing with `from_bytes` timed separately |
| B3's key layout as the tax lane is writing it | not read (another lane's files) | read `crates/dowiz-core/src/tax/` when it lands and check it against §2.7's row |
