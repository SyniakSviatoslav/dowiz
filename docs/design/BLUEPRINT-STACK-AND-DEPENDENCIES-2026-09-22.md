# Stack and dependencies: seven groups of proposed crates and patterns, and one claim about bebop, tested against this tree

**Date:** 2026-09-22. **HEAD read:** `7b0c9871` ("roadmap: one short entry point", 2026-09-22 23:30).
**Tree state at time of reading:** one untracked file in `docs/design/` (the ebills lane's blueprint); nothing
else in `docs/design/` modified. The POS lane's `BLUEPRINT-POS-THE-ROOM-2026-09-22.md`, which
`ROADMAP-2026-09-22.md:96-100` names as IN FLIGHT, **did not exist when the tree was first read** and
appeared at 23:46 while this document was being written; §4 was written from the roadmap rows A4-A8 and
then checked against it (§4.4). The two lanes reached the same verdict on CRDTs independently, and the POS
lane's re-entry condition is the sharper one; it is adopted in §0.

**Method.** Every "today" statement names a path and a line where it was read. Every number says how it was
obtained: `measured` means a command was run on this box and its output quoted; `hypothesis` means it was
not, and a verdict resting on a hypothesis says so beside it. Nothing here was taken from a document about
the code when the code was readable. Three documents outside the tree were consulted (an outside stack
analysis, its SQLite-based ordering argument, and a third analysis calling bebop a "tensor-graph language and
database"); every line of them was treated as a hypothesis to test, and the ones that failed are recorded
in §2, §5 and §7 with the command that refuted them.

This document is the decart comparison `.claude/CLAUDE.md:32-34` requires ("No silent adoption. Any new
dependency/crate/API/transport/backend swap must pass a decart comparison") for everything it recommends —
which, after testing, is very little.

---

## 0. Verdicts

| Group | Crate / pattern | Verdict | Basis | Re-entry condition (for AGAINST) |
|---|---|---|---|---|
| A | `serde` + `serde_json` | **ALREADY HAVE IT** — at the HTTP boundary only | measured: `workers/api/Cargo.toml:29-30`; kernel `json-api` feature; deliberately NOT in `dowiz-hub` (`minijson.rs:3-8`) | — |
| A | `rkyv` | **AGAINST** | reasoning §3 + one measurement (fold is memoised per generation, `hubdo.rs:416,624`); rkyv itself **not measured** (no network on this box) | a measured fold where record parse, not I/O, dominates: time `/fold/orders` cold vs memoised on a 20k-event log |
| A | `nutype` | **AGAINST** (preference; policy) | the pattern is hand-written already (`order_machine.rs:35-50`, `money.rs:21-45`); proc-macro in a zero-dep crate | never for `dowiz-core`; see the one real gap it would have caught, §2.3 (`Qty` is an alias) |
| A | `validator` | **AGAINST** (preference; policy) | 22 `deny_unknown_fields` sites + refusing parsers (`import.rs:137`) do this without a crate | — |
| A | `thiserror` | **AGAINST** (policy: shrink-only allowlist) | measured: `thiserror` enters the kernel lock only via `wgpu`/`sqlx` under off-by-default features | — |
| B | `proptest` | **ALREADY HAVE IT** (dev-only) — **OPERATOR QUESTION** for anything new | measured: dev-dep in `kernel/Cargo.toml:224`, `crates/dowiz-core/Cargo.toml:51`; 5 live suites | §6 |
| B | `insta` | **AGAINST** (preference, no measurement) | the repo's snapshot is a hand-written golden (`order_machine.rs:578-600`, `fsm_boot.rs:13`) | byte-stable rendered outputs (receipts, e-bill XML) needing regression files |
| C | `worker` | **ALREADY HAVE IT** | `workers/api/Cargo.toml:14` (0.8) | — |
| C | `wasm-bindgen` | **ALREADY HAVE IT** (transitively; used as `worker::wasm_bindgen`) | measured: lock reverse-deps; `hubdo.rs:39`, `lib.rs:66-70` | do NOT add a direct line: two versions already coexist in the tree (`wasm/Cargo.toml:17` pins `=0.2.95`; Worker lock has 0.2.128) |
| C | `gloo` | **AGAINST** (irrelevant) | no Rust runs in a browser today: the client is plain JS (`workers/api/public/**`); `dowiz-wasm` is loaded by no live page (measured grep) | the till ships the decider to the browser as wasm — then a measured decart `gloo-*` vs direct `web-sys` |
| D | `yrs` / `automerge` (CRDTs) | **AGAINST**, re-examined for the till in §4 | `DECISIONS.md:369`; `outbox.js:12-21`; `BLUEPRINT-POS-THE-ROOM-2026-09-22.md` §5 D reached the same verdict independently; both crates are `std`-only (hypothesis) so they could never enter the core anyway | **a till that must take payment and print a receipt with the hub unreachable** (the POS lane's condition, §4.4), or two devices amending one open tab while cut off from the hub and from each other — and even then the merge of money is a decision (refuse-at-fold vs compensate), not a library |
| E | `jiff` (replace `chrono`) | **AGAINST** (nothing to replace) | measured: `chrono` is in the Worker lock only via `worker`; zero source uses; time enters the kernel as `i64` (`json_api.rs:238-244`); `tz.rs:13-20` refuses a tzdb on purpose | the first venue in a zone whose rule is not the EU rule (`tz.rs` returns `None` and refuses on write) — and then it is "one more rule vs jiff + tzdb", measured |
| E | `ulid` / `uuid` v7 | **AGAINST** | §5: there is no B-tree; the log is chronological by construction (`evlog.rs:6-10`); ids are random on purpose (`lib.rs:61-65`) | a `Table` that needs id order rather than key order — which `table.rs:4-7` forbids by design |
| F | `tracing` + `tracing-subscriber` + `tracing-wasm` | **AGAINST** | §7: it would have removed the *excuse* for the defect, not the defect; the kernel already removed `tracing` once (`fdr/mod.rs:124-129`); bundle cost **not measured** (no network) | spans must cross an `await` into `dowiz-hub` and the `Req`-threaded `Trace` (the fix §7 names) proves unworkable — measure that first |
| G | type-state order lifecycle | **AGAINST** | §8: every transition arrives as a string (`json_api.rs:278`); the FSM is *data* with a golden fingerprint and a generated client vocabulary — type-state erases both | a Rust-only caller path holding an `OrderStatus` value, counted (today: 0 outside tests) |
| — | "bebop is a tensor-graph database" | **UNSUPPORTED** by the tree | §9: bebop is an integer-only AArch64 systems language (`LANGUAGE.md:5-10`); the product uses its *file format* only (`crates/bebop-store`); no bebop code can execute in a Worker | — |

An **ADOPT** with no measurement beside it would be a preference. There is no ADOPT in this table; the
things the outside analysis got right are already in the tree, and the things it got wrong are wrong for
reasons the tree wrote down before the analysis was made.

---

## 1. Ground truth: the constraints every hypothesis was tested against

### 1.1 The dependency policy, as built (and where the framing was off)

- **`crates/dowiz-core` is `#![no_std]`** (`src/lib.rs:17`, `#![cfg_attr(not(test), no_std)]`) with **one
  optional dependency**, `serde`, behind `json-api` (`Cargo.toml:11-19`: "zero deps — pure core::, no alloc,
  no std" at `:12`, then `serde = { …, optional = true }` at `:13`, `json-api = ["dep:serde"]` at `:19`).
  `proptest` is a dev-dependency (`Cargo.toml:51`).
- **`kernel/` (`dowiz-kernel`)** declares six optional deps — `wasm-bindgen`, `serde`, `sqlx`, `bebop-store`,
  `tokio`, `wgpu`+`pollster` — each behind a feature (`wasm`, `json-api`/`pq`, `pgrust`, `bebopdb`, `gpu`);
  `default = ["std"]`. **Measured:** `cd kernel && cargo tree --offline -e no-dev | grep -c -E
  "serde|tracing|proptest|thiserror|sqlx"` → **`0`**. The default graph is clean, as `CLAUDE.md:55-58` says.
- **`crates/dowiz-hub`** depends on `bebop-store` and `sha2` only (`Cargo.toml:12-19`).
  **`crates/bebop-store`** has an empty `[dependencies]` (`Cargo.toml:13`) — "ZERO dependencies: std only,
  plain file I/O, and a table-free CRC-32" (`Cargo.toml:9-11`).
- **`workers/api`** (the deployed Worker) carries `worker 0.8`, `serde`, `serde_json`, `hmac`, `sha2`,
  `base64`, `subtle`, `argon2`, `password-hash`, `getrandom`, `futures-util`, and the kernel with `json-api`
  (`Cargo.toml:14-43`). `[profile.release]` (`:49-64`): `opt-level = "z"`, `lto = true`, **`strip = false`**
  (`:64`).
- **`ZERO-DEP-ALLOWLIST.txt` is not "in three crates".** It is in **25** crate directories (measured:
  `find . -name ZERO-DEP-ALLOWLIST.txt` outside `node_modules`/`target`/`.claude`), and
  `scripts/zero-dep-crates.txt` rosters **24** of them for `scripts/zero-dep-gate.sh` (three checks: actual
  `cargo tree -e no-dev --locked --offline` names ⊆ allowlist; allowlist shrinks monotonically; `Cargo.lock`
  hash unchanged — `zero-dep-gate.sh:9-14`). **The kernel is rostered. `crates/dowiz-core`,
  `crates/dowiz-hub`, `crates/bebop-store` and `workers/api` are not** — there is no allowlist file under
  `crates/` at all. So the four crates this document is mostly about are held to zero deps by their own
  `Cargo.toml` comments and by review, not by the gate. That is a gap worth a row somewhere, not a reason to
  relax.
- **`deny.toml` exists** (`yanked = "deny"`, `wildcards = "deny"`, an explicit licence allow-list,
  `unknown-registry = "deny"`) **but I could not find `cargo-deny` running anywhere.** Measured: `grep -c -i
  deny .github/workflows/ci.yml` → `0`; there is no `.husky/` directory (`ls .husky` → no such file);
  `which cargo-deny` → nothing. `CLAUDE.md:125-126` describes a pre-commit that runs `cargo-deny check` when
  a `Cargo.*` is staged; that hook is not in the tree. Every licence in this document was checked by hand
  against the allow-list (all candidates are MIT / Apache-2.0 / Unlicense — hypothesis from crate metadata,
  not run through the tool). **The framing's "deny.toml + cargo-deny gate" is a file, not a gate**, and
  under the operator's "gates outrank rows" priority this is the most useful single line in this section.

### 1.2 Feature discipline and decart

`CLAUDE.md:115-121`: new heavy or external-dep functionality goes behind an off-by-default feature with a
header comment stating what it pulls in and how to verify the default graph stays clean. `.claude/CLAUDE.md:32-34`:
no silent adoption; a decart report in the change. Every AGAINST below therefore also says what the *feature-gated*
version would cost, so a later ADOPT-BEHIND-A-FEATURE has its numbers.

### 1.3 Purity: what "no clock, RNG, network or float" actually is in the tree

- `MANIFESTO.md:15` (C2): no clock/RNG/env/floats/network vocabulary reaches the kernel. `:18` (C5):
  integer-only money, `i64` minor units, no `From<f64>`.
- **Time enters as an argument:** `place_order_at(id, customer_id, items_json, created_at_ms: i64, channel)`
  (`crates/dowiz-core/src/json_api.rs:238-244`). The Worker reads the clock **once** per request into
  `Req { now_ms }` (`workers/api/src/lib.rs:244`), and `tools/gates/clock.sh` counts every other site
  (its header, lines 1-25, records that the honest count was 93, not the 27 a blueprint estimated).
- **Money:** `money.rs:1` "RED LINE: zero float arithmetic on monetary values". **The framing's "zero floats
  ever" is slightly stronger than the file:** `apply_tax(subtotal: i64, tax_rate: f64, …)` (`:267`) takes
  an `f64` *rate* from config and converts it once, `round(tax_rate * 1_000_000.0)` (`:272`), after which
  everything is `i128` with checked ops; `convert_all_to_eur_cents(amount_all: i64, rate: f64)` (`:332`) is
  the same shape. No monetary *amount* is ever a float. `ROADMAP-2026-09-22.md:107` (B1, "A tax rate as an
  integer") is already scheduled to close even that seam; it belongs to the tax lane, not this one.
- **Randomness enters at the edge:** `edge_id()` (`lib.rs:61-71`) takes an order id from
  `crypto.randomUUID`, **fail-closed** — "an order id that can repeat is a primary-key collision between two
  customers" (`:63-65`). The kernel never mints one. §5 depends on this.
- **The FSM is data:** `allowed_next` is a `const fn` table (`order_machine.rs:112-127`),
  `assert_transition` (`:173-198`) checks it and `debug_assert`s it against a compile-time bitmask built by
  independent code; a golden fingerprint (`verify_fsm_signature`, `:578-600`; `kernel/tests/fsm_boot.rs:13`)
  refuses drift. `spectral_radius() -> f64` (`:425`) is a report over that graph, not on the decision path.

### 1.4 Where state lives — the fact the SQLite premise gets wrong

- `tools/gates/no-sql.sh:33-39` counts `.prepare(`, `.d1(`, `[[d1_databases]]` bindings and migration files
  under `workers/api` and asserts the sum stays at its baseline, which is zero; the header (`:1-8`) records
  the ratchet from 118 statements to none. **Its scope is `workers/api` only.**
- **State is bebop images in a Durable Object,** one object per venue: `workers/api/src/hubdo.rs:1-34`
  ("NO SQL REACHES THIS MODULE. Storage here is a key/value map addressed by exactly the key we ask for";
  chunks of 96 KiB, `:42`; meta written last, `:31-34`). Reads come from the object's memory
  (`:362-367`); writes are chunked `storage().put` (`:1007-1033`).
- **The image format** is `crates/bebop-store` (2,222 lines measured: `lib.rs` 800, `evlog.rs` 971,
  `kv.rs` 451): two superblocks, an append-only arena of self-describing objects, refs as *object-relative*
  cell offsets, CRC-32 over little-endian cells (`lib.rs:1-25`). Two layouts sit on it: `EvLog` — O(1)
  append, chained content ids, **v2 packs eight payload bytes per cell** after a measured 583-cell (4.66 KB)
  event became 54 cells (`evlog.rs:6-10, 36-40`); and `Kv` — sorted keys, `(offset, len)` index arrays
  (`kv.rs:1-9`).
- **`dowiz-hub` is pure** — "Bytes in, bytes out. No filesystem, no network, no clock, no randomness"
  (`crates/dowiz-hub/src/lib.rs`, as quoted in `BLUEPRINT-ARCHITECTURE-EVOLUTION-2026-09-22.md` §1.1);
  `Table` (`table.rs:1-36`) is records plus the keys that find them in one `Kv`: **"There is no query
  planner here and no `WHERE`: every access path is a key that was written on purpose"**, and a prefix
  scan over sorted keys "is the whole of `ORDER BY` and `LIKE 'x%'` that this product ever needed" (`:3-7`).
  Layout `r/<kind>/<id>`, `x/<index key>`, `i/<kind>/<id>` (`:26-31`); the `Kv` is rewritten on every put,
  so a `Table` is for venue-sized sets and anything that grows with usage is an `EvLog` (`:32-36`).
- **Where the SQLite premise came from, charitably:** the tree's own canon still says it.
  `DECISIONS.md:19` ("peer nodes, each running the Rust/WASM kernel + a local SQLite DB") and `:79` ("Each =
  local SQLite + kernel"); `docs/adr/0008-local-sqlite-pq-at-rest.md:3` is `Status: PROPOSED`;
  `kernel/Cargo.toml` still declares `sqlx` behind `pgrust`; `tools/deep-clean/Cargo.toml:10` has
  `rusqlite` (a tool). An analyst reading `DECISIONS.md` would conclude SQLite is the store. It is not, and
  has not been since the 2026-09-21 directive (`BLUEPRINT-NO-SQL-BEBOP-EVERYWHERE-2026-09-21.md`). **Amending
  D1/D5 is a one-line follow-up for whoever owns `DECISIONS.md`;** until then every outside analysis will make
  this mistake.

### 1.5 The Worker bundle, measured, so cost claims below have a denominator

The built artefact on this box (`workers/api/build/`, 2026-09-22 22:20): `index_bg.wasm` **2,426,605 B**
(2,369.7 KiB) + `index.js` 31,958 B; `gzip -9`: **904,804 B** (883.6 KiB) + 8,312 B. The framing's
"2423 KiB / 899 KiB" is the deploy tool's figure (not found in any log on the box); it agrees within 2 %.

Section sizes (`wasm-objdump -h`): Code 1,824,804 B; Data 234,831 B; **custom `name` section 348,560 B
= 14.4 % of the file, shipped because `strip = false`** (`workers/api/Cargo.toml:64`). No crate discussed
here would move the bundle as much as that one line; it is outside this lane's remit and is reported, not
changed.

Per-crate share of function bodies (the box's `wasm-objdump` cannot parse this module's Code section —
"expected valid local type" — so body sizes were read directly from the section's LEB128 size prefixes and
joined to the `name` section; 3,703 bodies, 1,819,648 B; script in the session scratchpad, not the tree):

| crate (leading path segment) | bytes | share |
|---|---|---|
| `dowiz_api_worker` (handlers; largest single body `storefront::place` closure, 35,736 B) | 958,184 | 52.7 % |
| `core` (all monomorphisations) | ≈343,900 | 18.9 % |
| `js_sys` | 104,201 | 5.7 % |
| `worker` | ≈53,200 | 2.9 % |
| `dowiz_hub` | 45,712 | 2.5 % |
| `serde_json` + `serde_core` | ≈54,700 | 3.0 % |
| `url` | 30,355 | 1.7 % |
| `dowiz_core` | 25,090 | 1.4 % |
| `alloc` | 23,507 | 1.3 % |
| `bebop_store` | 15,103 | 0.8 % |
| `argon2` + `password_hash` + `blake2` | ≈18,800 | 1.0 % |

`chrono` does not appear in the top 24 — pulled by `worker` and dead-code-eliminated (hypothesis from its
absence, consistent with zero source uses). **The whole storage stack (`dowiz_hub` + `bebop_store` +
`dowiz_core`) is 86 KB of a 1.82 MB code section;** the handlers are half of it. Any "add a crate to shrink
the bundle" argument has to beat that.

---

## 2. Group A — serialisation and types

### 2.1 `serde` / `serde_json` — ALREADY HAVE IT, at the boundary, and deliberately not below it

Present in the Worker (`workers/api/Cargo.toml:29-30`) and in the kernel behind `json-api` (Worker enables
it, `:25`). **Not** in `dowiz-hub`, by a written decision: `minijson.rs:3-8` — the token payload is signed,
"a signed payload must have a byte-for-byte stable encoding, and a derive's output is stable only until the
derive changes"; the hand writer fixes field order so the signed bytes are a property of the file, not of a
dependency version. `:12-14`: anything from the network is parsed by `serde_json` at the HTTP boundary.
This is exactly the boundary the outside analysis recommends, already drawn, plus one reason it did not
give. Nothing to do.

### 2.2 `rkyv` — AGAINST (a competitor to the seam, not a better version of it)

The question was: is `rkyv` a better version of something this repo built, a competitor, or irrelevant?

**It is a competitor, and adopting it would cut the one property the store exists for.** `bebop-store`'s
manifest says why the format exists: "the format is POINTER-FREE by design — 'nothing in the file is an
address', every ref is an object-relative cell offset — which is exactly what lets a second language read
and write the same file" (`Cargo.toml:9-11`). `rkyv` archives are *also* pointer-free (relative pointers),
but their layout is defined by Rust type derives; a record archived by `rkyv` inside a bebop object is
opaque to `selfhost/prelude/store.bp` and `selfhost/std/kv.bp`, which own schema creation on the bebop side
(`kv.rs:9-10`: "bebop creates the schema … this module reads and writes the data through the documented
pointer-free format"). Two zero-copy formats, one of which only one language can read, is strictly worse
than one.

**Where `rkyv` would actually compete is one level down, and that level is memoised.** Inside an image,
records are JSON strings (`logimage.rs:31-34`: payload `[kind_len][kind][subject_len][subject][json]`;
`table.rs:27`: "the record, as JSON"). So the "zero copy" the repo has is at the *image* level; each record
is parsed when folded. That is `rkyv`'s pitch — read a record without parsing it. But the order fold is
**memoised per generation** in the object (`hubdo.rs:416` "from the memo when the generation matches";
`:624` "memoised against this generation"), so record parsing is paid once per write, not once per read,
and every poll of the queue is a field access. `rkyv` removes a cost that is already amortised away.

**Measurement:** `rkyv` itself was **not measured** — it is in no lockfile in the tree and this box has no
registry access (`cargo tree --offline` fails on any uncached crate). What *is* measured: EvLog v2's 8
bytes/cell packing (`evlog.rs:36-40`), the 86 KB storage-stack code share (§1.5), and the E4 lane's
finding that a store-backed decode's floor is memory and indirection, not arithmetic (1-2 ns/slot for the
arithmetic; recorded in the session memory `d4-floor-is-not-arithmetic`). A parse-free record format
attacks the 1-2 ns, not the 51 ns.

**Re-entry:** a measured cold fold of a realistic log (20,001 order events fit in 16.8 MB per the hub cost
blueprint) where `serde_json` parse time exceeds the object's I/O — `time` the `/fold/orders` route cold vs
memoised. Not run here. If that day comes, the answer is more likely "a binary record layout in bebop's own
object grammar" (which `evlog.rs` v2 already started) than an `rkyv` derive.

### 2.3 `nutype` and `validator` — AGAINST, and the one gap they would have caught

Both are proc-macro crates (build-time `syn`/`quote`; zero runtime bytes). The objection is not cost but
policy and redundancy:

- The repo's validated-newtype pattern is hand-written and refuses unknown values instead of mapping them:
  `OrderStatus::from_str` (`order_machine.rs:35-50`, "Unknown strings are rejected (never silently
  mapped)"), `Currency::from_code` (`money.rs:21-45`), `parse_price` refusing fractional and junk input
  (`import.rs:137`, tests `:357-385`), 22 `#[serde(deny_unknown_fields)]` sites in the Worker (measured
  `grep -c`). `nutype`'s `#[nutype(validate(…))]` generates the same `TryFrom`; the saving is ten lines per
  type, paid for with a proc-macro in a crate whose `Cargo.toml:12` says "zero deps".
- `validator` needs `serde` and, for its common validators, `regex`/`idna`; the repo retired regex from the
  kernel on purpose (`BLUEPRINT-ITEM-05-regex-retirement-2026-07-19.md`). Its shape — annotate a struct,
  call `.validate()` after deserialising — is the "validate after construct" model the refusing parsers
  above were written to avoid.

**The one thing the outside analysis is right about, by accident:** `pub type Qty = i64;`
(`crates/dowiz-hub/src/stock.rs:26`) is an *alias*, not a newtype, so a quantity and a minor-unit price can
be added without a compile error. That is exactly the class `nutype` prevents. It is closed by one
`pub struct Qty(i64)` with the four ops it needs — by hand, in the stock lane's file, not by a crate. Flagged
for whoever owns `stock.rs`; not changed here.

### 2.4 `thiserror` — AGAINST on policy, not on cost

Measured: `thiserror` is in `kernel/Cargo.lock` only as a dependency of `gpu-allocator`, `naga`, `wgpu-*`
(the `gpu` feature) and `sqlx-*` (the `pgrust` feature); it is absent from the default graph (§1.1). The
repo has 78 `pub enum …Error` types with 44 hand-written `Display` impls across `dowiz-core` and `dowiz-hub`
(measured `grep -c`). `thiserror` would delete perhaps 400 lines of `impl Display` at the price of a
proc-macro dependency in the two crates whose value is "zero deps" — and for the kernel, a *growth* of
`kernel/ZERO-DEP-ALLOWLIST.txt`, which `zero-dep-gate.sh:11-12` makes RED unless the gate itself is edited
in the same reviewed diff. Runtime cost is zero; the verdict is the allowlist's, not mine. A preference,
labelled as one: the hand `Display` impls in this tree carry sentences (`TransitionError::message`,
`order_machine.rs:155`) that `#[error("…")]` attributes would flatten.

---

## 3. Group B — correctness and testing

### 3.1 `proptest` — ALREADY HAVE IT, and the operator's rule and the tree need reconciling (§6)

Measured: `proptest = "1.11"` is a dev-dependency of both `kernel` (`Cargo.toml:224`) and `dowiz-core`
(`Cargo.toml:51`), with live `proptest!` blocks in `kernel/src/token_bucket.rs:188`,
`crates/dowiz-core/src/ports/payment.rs:660` (`with_cases(400)`), `ports/payment_provider.rs:1233`
(`with_cases(400)`), `retrieval/pattern.rs:489`, and `kernel/tests/json_oracle.rs:228`
(`with_cases(2000)`, "proptest differential fuzz"). There is also `kernel/tests/invariant_fuzz.rs` — a
"fuzz-harness" by its own header (`:1-3`) that is in fact a fixed list of 15 `f64` edge values (`:13-29`),
i.e. deterministic. And `kernel/tests/bebop_property_tests.rs` (behind `pq`). None of this is in
`dowiz-hub` or `bebop-store`. See §6 for the question.

### 3.2 `insta` — AGAINST (preference, no measurement)

Not in any lockfile. The repo's snapshot mechanism is a **hand-written golden**: `FSM_GOLDEN_SIGNATURE`
compared field by field by `verify_fsm_signature_against` (`order_machine.rs:578-600`), asserted at boot
(`kernel/tests/fsm_boot.rs:13, 44`); bebop-lang's `std_golden.sh` with Python oracles; the hub's
rebuild-and-diff projections (roadmap 2026-09-22, "Rebuild-and-diff" row). `insta`'s model — accept
whatever the code emitted on first run into a `.snap` file, review with `cargo insta` — is the shape of
"an instrument that measures nothing" this session has already catalogued five of (session memory
`bebop-instruments-that-measure-nothing`): a snapshot accepted from a wrong first run locks the wrong
answer in. A golden written by hand cannot do that. Dev-only, so bundle cost is nil; the cost is the review
workflow and a `similar`/`console`/`serde` dev-graph. **Re-entry:** byte-stable rendered documents — a
receipt, the ebills lane's fiscal XML — where the expected bytes are large enough that a hand golden is
impractical; even then `tests/fixtures/*.expected` compared with `assert_eq!` is the zero-dep version of the
same test.

---

## 4. Group D — CRDTs, and the till: the re-entry condition, re-examined properly

### 4.1 What is written down, and what now exists

- `DECISIONS.md:369` (O4): "content-address-only for money/order; CRDT fenced out of those, open for
  knowledge-wiki".
- `BLUEPRINT-ARCHITECTURE-EVOLUTION-2026-09-22.md:340-355` (P6): AGAINST, because a Durable Object is one
  writer per venue; the stock ledger's fold over an append-only log is already a join-semilattice; the courier
  position map is already LWW; and the named re-entry: "two writers that cannot share an object — a POS till
  that must take orders with the hub unreachable … When one does, the merge of two append-only logs of
  signed, content-addressed events IS the G-Set in `mesh_replication.rs`; what has to be DESIGNED is the rule
  when the merged fold drives stock negative (refuse-at-fold vs. compensate). That is a decision, not a
  library."
- `ROADMAP-2026-09-22.md:145-149` repeats the fence and says the condition "may now be met" and that this
  lane was told to re-examine rather than inherit.
- **What exists since P6 was written (all measured):**
  - `workers/api/src/outbox.rs` — the venue-side effect outbox: the effect is written in the same turn as
    the event (`:14-19`), the drain retries with backoff and abandons loudly (`:20-26`).
  - `workers/api/src/idempotency/mod.rs` — `Idempotency-Key` with the five rules (`:13-33`): stored
    full response; key scoped to `(venue, principal, route, key)`; **different body under the same key is
    409**; a concurrent retry is 409 + `Retry-After`; fails open. Guarding `courier.rs:348, 478, 543`,
    `owner.rs:547`, `storefront.rs:911` (measured call sites).
  - `workers/api/public/lib/outbox.js` (345 lines; the framing's `public/lib/outbox.js`) — the client
    write queue. Its header is the argument, already in the tree: "**WHY THIS NEEDS NO CRDT** … A courier's
    actions on ONE order are a SEQUENCE, not a set of concurrent edits … The server's FSM refuses an illegal
    edge as an error rather than a silent no-op, and an `Idempotency-Key` makes the replay of the same tap
    the same answer. A queue that drains IN ORDER, replays idempotently, and believes the refusal is a
    deterministic merge already" (`:12-21`); the key is minted at tap time, not drain time (`:22-26`);
    **a refusal is never retried — "409 is an ANSWER"** (`:28-33`).
  - The object-side command surface: `command::{place, advance, assign}::decide` (`command/advance.rs:79`,
    `place.rs:112`, `assign.rs:86`), one turn for the transition and the stock settlement
    (`advance.rs:11-15`), the kernel still deciding (`:17-20`).
  - The G-Set: `crates/dowiz-core/src/mesh_replication.rs:26-33, 209-213` — ingest is idempotent by
    content id, union is commutative/associative/idempotent, two nodes diverge offline and converge on pull.

### 4.2 Is a till inside or outside the fence?

**Inside.** A till writes orders and money — the two things O4 names. So the question is not "may a till
use a CRDT" (no), but whether a till *needs* one, i.e. whether `queue + Idempotency-Key + refusing FSM`
fails to cover some till behaviour. Take the roadmap's rows one at a time (A4-A8, `ROADMAP-2026-09-22.md:96-100`):

| Till behaviour (roadmap) | Shape of the write | Covered by what exists? | What a CRDT would add |
|---|---|---|---|
| A4 waiter takes an order at a table, hub unreachable | a NEW order: id minted locally (CSPRNG, as `edge_id` does at the edge), events queued, replayed through `place` on reconnect | yes — same shape as `outbox.js`; conflict = stock refused at replay → surfaced to the till, as `:28-33` surfaces 409 to a courier | nothing: there is no concurrent edit of one document |
| A5 open tab, `Amended` events | append-only amendments to ONE order from ONE device | yes — a sequence, refused if illegal | nothing |
| A5/A6 **two devices amend the same open tab** (waiter's phone + the till), both cut off from the hub | concurrent edits on one document — **the true CRDT shape** | partly: two append logs merge as a G-Set (`mesh_replication.rs`); the *order* of amendments is what a CRDT would fix automatically — and what money must NOT fix automatically | automatic convergence of a void against a payment, or of two splits of one bill: convergence without a refusal is the failure mode O4 fences out |
| A6 split bill, transfer between tables, void with a reason | money-bearing amendments | yes, if serialised at one decider; **undecided** if two deciders act while apart (P6's "refuse-at-fold vs compensate") | a merged number nobody decided |
| A7 the till as a shift, X/Z report | a fold over the shift's events; a conservation law | yes — a fold is deterministic over a merged G-Set once order is fixed | nothing |

The row that matters is the third. Everything else is the courier's queue with a different noun. And for
that row the honest statement is: **a CRDT is the wrong tool precisely there**, because what it does —
converge without asking — is what a bill must not do. `DECISIONS.md:369` already says this for money;
`outbox.js:28-33` says it for orders ("a client that retried that would hammer production").

### 4.3 What a till actually needs, and what it costs — none of it is a crate

1. **A decider on the till.** `outbox.js` holds *requests*, not state; a till that must price a basket,
   check stock and open a tab while cut off needs the venue's images and the same pure `decide` functions
   locally. Those functions exist (`command/*::decide`, `dowiz-hub`), and the storage stack is small
   (**86 KB of wasm code**, §1.5 — measured share in today's bundle). What is not yet true: the `command`
   modules live in `workers/api` and import `serde_json`/`worker` types; moving `decide` into `dowiz-hub` so
   it can compile for a browser or the native twin (`tools/native-spa-server`, already a second Worker per
   the session memory) is a **refactor of three files, not a dependency**. Cost: hypothesis, a day; the
   decart is "which crate owns `decide`", and it should be answered with the POS lane (its blueprint, §4.4
   below, keeps the object as the authority and does not yet place a decider on the tablet — it predicts
   with `replica.js` and folds what comes back).
2. **One authority per partition, named.** While cut off, the till is the venue's single writer for the
   room and the hub is the single writer for online customers. That is *two objects*, each single-writer,
   which is P1's model applied twice — not two writers on one object. Reconnection is a G-Set merge of two
   content-addressed logs followed by a **replay through the hub's FSM in the till's order**, with refusals
   returned to the till operator as answers. `mesh_replication.rs` is the merge; `advance.rs:79` is the
   refusal.
3. **The decision P6 named, still undecided:** when the merged fold drives stock negative (the storefront
   sold the last portion online while the till sold it in the room), refuse-at-fold or compensate. This
   document does not decide it; it records that it is the *only* open question a till adds, and it is a
   product rule.
4. **The one case to design out rather than merge:** two devices amending one tab while apart from each
   other. The coarse rule is a tab owned by one device at a time (a lock carried in the tab's last event;
   transfer is an event; a second device's amendment while it is held is refused, exactly as a 409). The POS
   lane chose the finer rule, and it is the better one — see §4.4. Either turns the third row of the table
   into the first. If a room ever needs simultaneous editing of one tab by two waiters with no connectivity
   between them, **that** is a re-entry condition, stated precisely — and even then the answer for the
   *money* fields is a refusal, with a CRDT at most for non-money fields (notes, seat labels), which is what
   O4's "open for knowledge-wiki" already permits.

### 4.4 Checked against the POS lane's blueprint, which appeared while this was being written

`BLUEPRINT-POS-THE-ROOM-2026-09-22.md` (23:46) re-examined the same re-entry independently and lands in the
same place, with two mechanisms this document had not named and one sharper condition:

- **A version on value edits, not on status changes** (§3.4): `place/advance/assign` need no generation
  because read and write are one object turn, but "it is not true of the DECISION a waiter made on a tablet
  thirty seconds ago from a copy of the order"; `AmendIn.base_seq` is the newest event `seq` the tablet
  folded, and `decide` refuses `Conflict` when the projection has moved. An `advance` does not need it (the
  FSM refuses a stale edge); a `pay` does not (Σ ≤ bill is checked against the live fold). This is Square's
  `order.version`, i.e. optimistic concurrency at the single decider — the opposite of a CRDT.
- **Intent-form amendments** (§4.4): send `add line` / `remove line` / `set qty`, not the new line set,
  "precisely because it commutes more often — two adds never conflict", so `base_seq` is needed only for
  `remove` and `set qty` on a line another device changed. That is the finer version of item 4 above:
  commutativity where it is free, refusal where it is not, and no library in between.
- **The sharpened condition** (§5 D): "Taking ORDERS with the hub unreachable needs no merge … Taking MONEY
  with the hub unreachable is different in kind: a `Paid` predicted locally against a bill the object later
  folds differently is cash taken for the wrong number, and no merge law makes that right; a receipt printed
  from a prediction is a fiscal act on a guess. So the sharpened condition is not 'an offline POS till' but
  **a till that must take payment and print a receipt with the hub unreachable**." Its re-entry: the
  operator states the room must keep taking payments through an outage longer than the outbox holds, and
  accepts compensation as the reconciliation rule. Its §8 leaves the outage length to the operator.

Both documents agree that the fence at `DECISIONS.md:369` stands, that the G-Set merge is the easy half, and
that what does not exist is a *rule* for a merged fold that disagrees with a receipt already given. This
document adds only §4.3 item 1 — that the decider must be able to run on the till, which is a crate-ownership
refactor — and the measured 86 KB storage-stack share that says it is cheap to ship.

**Cost of the crates, for the record (hypothesis, not measured — no network):** `yrs` and `automerge` are
`std` Rust and could never enter `dowiz-core`/`dowiz-hub`; they would live in the Worker or a browser wasm.
Published wasm builds are in the hundreds of KB (`yrs`) to over a megabyte (`automerge`), i.e. between a
third and two thirds of today's entire code section, to solve a problem the fence says must be solved by
refusal. **Verdict stands: AGAINST**, with the re-entry condition sharpened from "an offline POS till" to
"two devices amending one open tab while cut off from the hub and from each other, and a product decision
that the merge must not refuse".

---

## 5. Group E — time and identity

### 5.1 `jiff` replacing `chrono` — AGAINST, because there is nothing to replace

Measured: `chrono` is in exactly one lockfile, `workers/api/Cargo.lock`, and its only dependant is
`worker` (tomllib reverse-dep parse). `grep -rn chrono` over `workers/api/src`, `kernel/src`, `crates/*/src`
hits only module names (`chronos.rs`, `chronos_topology.rs`) — zero uses. The product's time model is:
the Worker reads `Date::now()` once into `Req { now_ms: i64 }` (`lib.rs:244`; `clock.sh` gate at 0), the
kernel takes `created_at_ms: i64` (`json_api.rs:238-244`), and local time is `crates/dowiz-hub/src/tz.rs`
— "INTEGER MINUTES, no floats, no date library, no clock" (`:22-24`), the EU DST rule as arithmetic
(`:13-20`, `Dst::Eu` at `:35-40`), and **a zone whose rule is not in the file is refused on write rather
than guessed** (`:17-20`). `jiff`'s value is a Temporal-style API over a tzdb; a Worker isolate has no
`/usr/share/zoneinfo`, so it would be `tzdb-bundle-always` — a bundled database (hypothesis: hundreds of
KB) for a product that serves venues in one DST regime and would rather refuse a zone than carry the world's.
**Re-entry:** the first venue whose zone `tz.rs::zone()` returns `None` for; the decart then is "one more
`Dst` variant vs jiff + bundled tzdb", with the bundle measured.

### 5.2 `ulid` / `uuid` v7 — AGAINST; what survives without a B-tree

The outside analysis's argument was "so SQLite stores events in chronological order" — a B-tree index on a
random primary key fragments, a time-ordered key appends. Tested against the tree:

- **There is no B-tree.** Events are appended to an `EvLog` whose records chain by object-relative ref
  (`evlog.rs:6-10`: "an append allocates ONE object, relinks the root, and commits. That is O(1) per
  insert"); `logimage.rs:10-13` says it directly — every D1 table it replaced "had an index on `(something,
  created_at_ms DESC)`, which is what an append-only log IS". Chronological order is the storage order by
  construction; the id plays no part in it.
- **`Table` keys are written on purpose** (`table.rs:3-7`) and sorted; if an index must be time-ordered,
  the time goes in the *key* (`x/<kind>/<at_ms>/<id>`), not in the id. That is already how the sorted `Kv`
  gives "ORDER BY". A time-ordered id would be a second, weaker copy of that.
- **Ids are random on purpose.** `lib.rs:61-65`: the CSPRNG id replaced a kernel `AtomicU64` counter that
  collided the moment it left one process. A v7 UUID keeps 74 random bits, which is enough — but it also
  publishes the creation instant in every id, and an order id appears in customer-facing URLs and courier
  tasks. Nothing in the tree needs that, and the kernel must not mint time (C2) — the id would have to be
  composed at the edge from `Req.now_ms` plus CSPRNG bytes, which is a 20-line function if it were ever
  wanted, not a crate.
- **What survives:** one thing — a *human-sortable* id in logs and support tickets. It is a convenience,
  not an ordering guarantee, and the log already carries `atMs` in each record (`errlog.rs:80`).
  **Verdict AGAINST**; re-entry: a `Table` that must be ordered by id rather than by a purpose-written key,
  which `table.rs` forbids by design.

---

## 6. `proptest` — a question for the operator, not a decision here

The operator's standing rule (2026-09-21, recorded in this session's memory as "No fuzzers in this repo"):
the hub fuzzer `crates/dowiz-hub/src/fuzz_tests.rs` — a seeded generator running ~20k corrupted images
inside `cargo test`, minutes long, one case asking for an 8 GiB allocation, which on a 7.5 GB phone under
the phantom-process killer is the whole session — was deleted, and defects are to be locked with **one named
corrupted cell**: "fixed cut points, not random ones. No `cargo-fuzz`, no seeded generators, no loops of
thousands of iterations in the test suite."

**What the tree contains today (§3.1, measured):** five `proptest!` suites in `kernel`/`dowiz-core`, bounded
at 400 and 2,000 cases, over *pure functions with small inputs* (a GCRA rate limiter against a mutex model;
ledger fold equals fold-derived; a pattern matcher against a reference; a JSON round-trip). They pre-date the
rule and are not in the image-parsing crates. A "seeded generator" and a "loop of thousands of iterations"
describes them literally; "a fuzzer over corrupted images that can ask for 8 GiB" does not.

**My view, marked as such:** property tests over pure kernel functions with bounded, shrinking inputs are
the *cheap* half of the verification pyramid the repo already relies on (`json_oracle.rs:184` calls its own
suite a "differential fuzz", and it has been green for months). The failure the operator hit came from
generating *images* — inputs whose size is under the generator's control — not from generating *integers*.
The line I would draw: **no generated inputs in `dowiz-hub` or `bebop-store` (anything that parses bytes it
did not write), keep the existing kernel suites, and any new `proptest` block must state its case count
and its largest input in its header.**

**Questions for the operator, verbatim:**
1. Does "no fuzzers" cover the five existing bounded `proptest` suites in `kernel`/`dowiz-core`? If yes,
   they should be converted to named cases and `proptest` removed from both `[dev-dependencies]` (the
   dev-graph shrinks; the default graph is unaffected, §1.1).
2. If no, is the line above (never over image bytes; header states cases and largest input) the rule?

No new `proptest` use is recommended by this document either way.

---

## 7. Group F — `tracing`: would it have prevented the defect in `afdf15cd`, or renamed it?

### 7.1 The defect, as the commit records it

`afdf15cd` ("warnings: the SQL era's corpses, and a tracer that described spans it never emitted"): the
`otel` header "said it traces 'the request, the kernel call inside it, each store read and write'.
`Trace::child` has no caller anywhere in this crate and never has, so every trace this product has ever
exported is ONE SPAN … `traceparent()` renders correctly and nothing sends it, so a call to Telegram or
Stripe starts a new trace instead of continuing this one." The corrected header is now `otel.rs:1-36`;
`child` is marked "NO CALLER TODAY" at `:102-106` (measured: still no caller outside `otel.rs`).

The header also says **why** it was never wired (`:14-21`): "the `Trace` lives in the router's own scope and
the store calls are several frames down inside the handlers, so spanning them means threading the trace
through every handler signature or putting it somewhere a Worker isolate can reach from both."

### 7.2 Renamed, not prevented — but it would have removed the excuse

Separate the two things that went wrong:

1. **A header claimed spans that were not emitted.** `tracing` does not emit a span for a store read until
   someone writes `#[instrument]` on it or opens `span!` around it. With `tracing`, the header could have
   made the identical claim and `child` would have been `info_span!("store.get")` with no call site. The
   *defect* — an instrument described as measuring something it did not — is renamed, not prevented. The
   thing that prevents it is what the session's memory already names: an instrument must be exposed by a
   command that shows it measuring (`bebop-instruments-that-measure-nothing`), e.g. a test that exports a
   trace for a request that touches the store and asserts `spans.len() > 1`. That test costs zero crates.
2. **The wiring was never done because context could not reach the call site.** This is the part `tracing`
   solves by construction: its dispatcher holds the current span in a thread-local, and `Instrument` carries
   it across `await`s, so a store call several frames down can open a child of the request span without a
   parameter. In a single-threaded Worker isolate that works. **So `tracing` would have removed the reason
   the author gave for not wiring it.** But the tree already has a cheaper carrier for the same context:
   `Router::with_data(Req { now_ms })` (`lib.rs:244`) is a per-request value every handler reaches through
   `RouteContext::data` — P3 built it for the clock; putting the `Trace` in it is option (a) of the header's
   own list. Cost: the handler signatures already take the context; the store calls that need a span are in
   `hubstore`/`hubdo` (dozens, not hundreds). Hypothesis: a day, no dependency.

### 7.3 What it would cost, and what the kernel already decided

- **The kernel removed `tracing` once.** `kernel/src/fdr/mod.rs:124-129`: `SpanObserver` is "the
  kernel-owned replacement for `tracing_subscriber::Layer`'s span hook" (P83), a trait with one method,
  `on_span_close(name: &'static str, dur_us: u64)`. `otel.rs:24-28` notes the consequence honestly: kernel
  spans have no trace id and no parent, so they cannot be stitched in without widening that trait — "A span
  with a fabricated parent is worse than no span." Re-adopting `tracing` in the Worker would put the two
  halves of the product on two span models again, which is the state P83 ended.
- **`tracing-wasm`** writes to the browser `console` and `performance` APIs; it is not an OTLP exporter. The
  OTLP/HTTP builder in `otel.rs` (which works, one span at a time) would stay and become a custom `Layer`.
  So the adoption is `tracing` + `tracing-subscriber` (registry) + a hand-written layer — the exporter is not
  saved, only the context plumbing.
- **Bundle:** **not measured** — no registry access on this box. For scale, the entire storage stack is
  86 KB and `serde_json` is 55 KB of a 1.82 MB code section (§1.5); a `tracing-subscriber` registry with
  per-span field storage is, as a hypothesis, of the order of `serde_json`, and every `#[instrument]` adds
  code to the 52.7 % that is already handlers. Telemetry that must never fail a request (`otel.rs:29-32`)
  also gains a dependency that *can* panic in a subscriber.
- **`tracing` is already in three lockfiles transitively** (measured): `kernel` via `sqlx-core`/`sqlx-postgres`
  under `pgrust`; `tools/native-spa-server` via `axum`/`h2`/`tower`; `mesh-adapter`/`node` via `quinn`. None
  in a default build. If the native twin ever needs the same spans as the Worker, it already links the crate;
  that is the strongest re-entry argument and it is about the twin, not the Worker.

**Verdict: AGAINST.** Do the two zero-crate things first: carry `Trace` in `Req`, and add the test that fails
when a store-touching request exports one span. Re-entry: spans must cross an `await` into a `dowiz-hub`
call whose duration is the number wanted, *and* the `Req`-carried trace is shown (measured) not to reach it.

---

## 8. Group G — the type-state pattern: `dispatch_order()` cannot compile for an unpaid order

### 8.1 What the FSM is, and where its callers are

- "Paid" is not a state. `OrderStatus` has twelve variants (`order_machine.rs:14-31`); money is considered
  moved once an order is `Confirmed` (`:24-26`, on `Refunding`: "reachable from any post-commitment state
  (money has moved after `Confirmed`)"), and `took_money()` (`:92-94`) derives it from status. The
  property the analysis wants — no dispatch before payment — is `allowed_next(Pending) = [Confirmed,
  Rejected, Cancelled]` (`:115`): `Pending → InDelivery` is refused at runtime, tested, and fingerprinted.
- **Every transition arrives as a string.** `apply_event_logic(order_json: &str, next_status: &str)`
  (`json_api.rs:278`) parses both; the object's `command::advance::decide` (`advance.rs:79`) calls it with a
  status that came off the wire (`:26-30`: "`next` IS A STATUS AND THE ACTION IS NOT SENT"). Measured: no
  product code outside `dowiz-core` holds an `OrderStatus` value across a transition; `PhantomData` appears
  in the core only in `optical.rs` and `lut.rs`, unrelated. **A type-state `Order<Paid>` is erased at the
  first `&str`, which is every caller.** The compile-time proof would cover zero call sites that see the
  network.

### 8.2 What the pattern would cost, and what it would break

- Twelve states become twelve types; the `const fn` table at `:112-127` becomes impl blocks; `fold_transitions`
  (`:201-215`), the topological report, `reachable`, `has_cycle`, `cyclomatic_number`, the golden
  fingerprint (`:578-600`) and the generated client vocabulary (`tools/gen-vocab`; roadmap row "Client
  vocabulary … generated from the kernel — all four hand copies were short by `COMPENSATED_REFUND`") all
  work because **the FSM is data**. Type-state makes it code, and the repo's two real FSM defects this
  month were not "an illegal edge compiled": they were a *missing* edge (no exit past `PENDING`, session
  memory `dowiz-order-fsm-has-no-exit`) and folds *spelling statuses by hand* (fixed by `took_money`, a
  method on the enum). The first is a table row; the second is exactly the thing a data-FSM with one method
  fixes and a twelve-type FSM spreads back out.
- Where type-level guards DO pay in this tree is money, and it is already done: `Currency` on every amount
  (`money.rs:21-25`, M5: "a cross-currency operation is a caught error, not a silent unit confusion").

**Verdict: AGAINST.** Re-entry: a Rust-only path that holds `OrderStatus` values from decision to write with
no string in between (count the call sites; today 0), at which point a *narrow* typestate on that one path
could be weighed against keeping the table — and even then the table stays the source of truth.

---

## 9. What bebop is, and the three "tensor" claims

### 9.1 What the tree says bebop is

- `bebop-lang/README.md:3-6`: "A self-hosting, integer-only language **for AArch64** with no C in the
  toolchain: a 1.5 KB assembly loader (`seed/seed.S`) runs `bebop.bin`, which compiles `.bp` source to raw
  machine words; `bebop.bin` is itself compiled from `bebop.bp` and reaches a byte-exact fixpoint."
- `bebop-lang/docs/LANGUAGE.md:5-10`: "a small, integer-only, self-hosting language compiled straight to
  AArch64 machine words … There is no runtime library, no garbage collector, **no strings beyond literals,
  and no types at run time**: every value is a 64-bit integer (`i64`), and an array is the address of a run
  of i64 cells. **Types are parsed and discarded by the compiler**." `:20`: struct literals work for the
  FIRST declared struct only (A22). `:29`: 14 parameters; a 15th "is NOT a compile error today: the fn gets a
  `brk #8` prologue and the program exits 8 silently".
- `bebop-lang/ROADMAP.md` thesis: it "compiles itself, proves itself, and **is its own database**" — and
  the database half is `docs/LANG-DB-DESIGN.md §4 + §9.5`, whose first line reads "**the design is a
  PROPOSAL pending operator decisions**".
- Its defect record is a compiler's: 833 lines in `BUGFIXES.md` and 2,961 in `HISTORY.md` (measured
  `wc -l`) — operator precedence, spill machinery above eight live symbols, scratch-zone overlap, register
  protocol (`AGENTS.md` T2 list).

**What dowiz uses of it: the file format, and nothing that executes.** `crates/bebop-store` reads and writes
the store layout that `selfhost/prelude/store.bp` (1,200 lines) and `selfhost/std/kv.bp` (117 lines) define;
`dowiz-hub` folds Rust over the cells. **No bebop code runs in the product,** and none can: the product is a
Cloudflare Worker running wasm32 (`workers/api/Cargo.toml:9`, `crate-type = ["cdylib"]`; `worker-build`
per `wrangler.toml`), and `bebop.bin` emits
AArch64 machine words for a Linux seed. "Queries execute natively on the data structure" is therefore
false for the deployed product in the strongest sense: the only code that executes over these images is
Rust, and its query language is `table.rs:3-4` — "no query planner, no `WHERE`: every access path is a key
that was written on purpose".

**Where "tensor" is real in bebop-lang, and what it is:** the T100 gate `bench/tq_sqlite/run.sh` — "tensor-query
latency vs sqlite" — is a **nearest-neighbour lookup over one million LCG points through a 1024×1024 grid
of cell buckets in CSR form** (`RESULT.md:1-12`: bebop indexed 2.0 µs vs sqlite C-API 57.2 µs). It is a
spatial grid index, benchmarked. `docs/LANG-DB-DESIGN.md §9` is titled "**The operator's proposals** (A)
dimensional descent, (B) graph = tensor, (C) CRUD on it — mechanism by mechanism" (`:474`), and the
analyst's verdicts are on the record: "'dimensional descent' is the T100 grid index under another name (a
2-level arithmetic router = grid file; more levels = a B-tree with shifts; wins on uniform keys, loses on
skew)" (`:20`, `:527-531`); "graph = tensor is exact (GraphBLAS)" — BFS is iterated sparse-matrix ×
sparse-vector at "10-40 ns per nonzero, memory-bound" (`:536-548`). So the "tensor-graph database" is the
operator's own proposal, examined in the tree, reduced by that examination to a grid index plus CSR, and
**not built into anything the product runs.**

### 9.2 The three claims, one at a time

**(1) "Price is a multidimensional tensor over (Channel × Fulfilment × TimeOfDay × ClientSegment)."**
Today a product's price is one integer in its catalogue record (the test fixture at `catalog.rs:200-201`
shows the shape, `{"name":"Sake Futomaki","price":900}`), plus modifier deltas (`modifiers.rs:201`,
`price(groups, chosen) -> Priced`, integer `delta`), plus `promo.rs`; per-channel price is
`ROADMAP-2026-09-22.md:109` (B3) **IN FLIGHT at the tax lane**, and
"channel" is already an argument to `place_order_at` (`json_api.rs:243`). Four coordinates → one integer
is a map with a four-part key: in `Table` terms `x/price/<product>/<channel>/<fulfilment>/<slot>/<segment>`,
one key per *exception* to the base price. A dense tensor is the wrong shape: for Sushi Durrës's 165
dishes (session memory), 3 channels × 3 fulfilments × 24 slots × 4 segments = 864 cells = 6.9 KB per dish,
**1.1 MB per venue image** (arithmetic on today's counts) to hold values that are 95 % the base price,
against a handful of purpose-written keys. What is genuinely hard — precedence when a promo, a channel
price and a segment price all apply; historical correctness (B4, "last March's receipt shows last March's
rate") — is a **rule**, which no data structure supplies. **Cost to build "on bebop":** not possible in the
Worker (§9.1). **Cost in Rust:** a key layout and a lookup with a stated precedence — the B3 work the tax
lane owns. **Basis for the claim: none.**

**(2) "Stock is tensor subtraction of a recipe matrix."** `stock.rs` is an integer ledger: `decide`
(`:180`) refuses an event against the current level, `fold` (`:344`) replays Reserve/Consume/Release events;
recipes are read from the catalogue record (`:951-954`). "Recipe matrix × order vector" *is* the loop that
exists, and with 3-8 nonzeros per dish the matrix form does the same multiplications with more zeros. The
open stock problems in the tree are semantic — an order abandoned at `CONFIRMED` holds its reservation for
ever; `Stocktake` refuses `observed < reserved` (session memory `dowiz-order-fsm-has-no-exit`); P6's
refuse-vs-compensate — and a subtraction of any rank does not touch them. **Basis: the arithmetic is the
same; the word adds nothing.**

**(3) "Courier ETA and routing execute natively on the data structure, so Dijkstra is not needed."**
Three facts: (a) the product's ETA is straight-line metres in integer micro-degrees (`eta.rs:294`
`straight_line_m`; `zone.rs:16-20`: "THE EARTH IS FLAT HERE, and that is a measured choice"), fed to
`estimate(items, ahead, distance_m, k)` (`eta.rs:219-224`), with the explicit note "a caller with a
routing service should pass that instead" (`:290-293`); (b) **the kernel already has a zero-dependency
CSR Dijkstra / A* / contraction-hierarchy router with OSM ingestion** (`crates/dowiz-core/src/router.rs:1-12`)
and it has **no product caller** (measured: only the `pub use` at `kernel/src/lib.rs:615` and a span
instrument); (c) bebop-lang's graph kernels are CSR/BFS benches on AArch64. The claim conflates BFS-as-SpMSpV
(unweighted hops, Boolean semiring) with shortest path on a weighted road graph: SpMSpV over a (min,+)
semiring *is* Bellman-Ford — it computes what Dijkstra computes with O(V·E) work instead of O(E log V).
"Dijkstra is not needed" is true only in the sense that a slower algorithm exists. And no road graph is in
any image. **Cost to get real routing:** an OSM extract for Durrës turned into CSR (a data pipeline;
hypothesis: tens of MB raw, a few MB as CSR, per venue region) plus the router that exists — nothing to do
with the language. **Basis: a Dijkstra exists, unused, in Rust; nothing executes "natively" in bebop.**

**Plainly:** bebop is a small integer-only systems language with its own compiler, and a pointer-free
image format that Rust reads. That format is a real asset (it is why SQL could go). The pricing tensor, the
recipe tensor and the native router are not in the tree, are not proposals in the tree's own bebop design
beyond a grid index and a BFS bench, and could not execute where the product runs. **A roadmap that
budgets on them budgets on nothing**, and the cheapest correction is to route every pricing/stock/routing
row to the lane that owns the Rust file named above.

---

## 10. What was NOT verified, and the command that would settle it

- `rkyv`, `yrs`, `automerge`, `jiff`, `tracing-subscriber` bundle costs: **not measured** (no registry on
  the box; `cargo tree --offline` fails on uncached crates). Settle with, on a networked box: add each behind
  a feature in a scratch copy of `workers/api`, `worker-build --release`, and diff `index_bg.wasm` and its
  gzip against §1.5.
- The fold-time premise of §2.2: `time curl …/fold/orders` against a venue object cold vs warm, on a log of
  the hub-cost blueprint's 20,001-event size.
- Whether `command::{place,advance,assign}::decide` are pure enough to move into `dowiz-hub` (§4.3 item 1):
  `grep -n "worker::\|Date::now\|env" workers/api/src/command/*.rs` and read what remains.
- The deploy-tool size figure "2423 KiB / 899 KiB" was not found in any log on the box; the local build
  agrees within 2 % and is what §1.5 cites.
- `cargo-deny` not running anywhere (§1.1) — settle by `grep -rn deny .github/ && which cargo-deny`; if both
  are still empty after this document, the licence gate is a document.

## 11. Corrections to the framing this lane was given

1. `ZERO-DEP-ALLOWLIST.txt` is in 25 crates (24 rostered), not three; `crates/dowiz-core`, `dowiz-hub`,
   `bebop-store` and `workers/api` are **not** rostered.
2. `deny.toml` exists; a `cargo-deny` gate does not run anywhere I can find (§1.1).
3. "Zero floats ever" in money: an `f64` *rate* enters once at `apply_tax`/`convert_all_to_eur_cents`
   (`money.rs:267, 332`) and is converted to micro-units before any arithmetic; amounts are never floats.
   B1 is scheduled to close the seam.
4. The JS outbox is `workers/api/public/lib/outbox.js`.
5. The POS blueprint named by the roadmap did not exist at first read and appeared at 23:46; §4 was
   written from rows A4-A8 and then checked against it (§4.4) — same verdict, sharper condition, adopted.
6. The SQLite premise has a source in the tree (`DECISIONS.md:19, :79`; ADR-0008 PROPOSED; `sqlx` behind
   `pgrust`); it is wrong about the product and the canon should be amended so the next analysis does not
   inherit it.
7. `tracing` is already in three lockfiles transitively and in no default build; `proptest` is already a
   dev-dependency with five suites; `wasm-bindgen` is already used directly through `worker`. Three of the
   proposed "adoptions" describe the tree as it is.
