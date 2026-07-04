# Design Proposal — Rust `domain` crate money newtype `Lek(i64)` (Phase A scaffold)

- **Date:** 2026-07-04 · **Author:** System Architect · **Status:** PROPOSED — Round-1 breaker + counsel RESOLVED (STOP-DESIGN-B; no code). Resolution: `./resolution.md`.
- **Scope:** ONE file — `rebuild/crates/domain/src/money.rs`. Pure invariant core: NO IO, no sqlx, no tokio.
  Lands as isolated scaffold in a git worktree; nothing merged, no DB writes, not yet wired to any call site.
- **Companion ADR:** `docs/adr/ADR-rust-money-newtype.md` · **Frame:** `docs/design/rebuild-plan/06-complete-rebuild-stack.md`,
  `.../REBUILD-MAP.md` §1/§2, `.../inventory/12-data-layer.md` §9/§10-R3.
- **Red-line:** 🔴 money/correctness. This is the seam that makes "negative money" and "silent overflow"
  *uncompilable* rather than runtime-guarded. It is design-time only; the runtime lands in Phase B S5.

---

## 1. Problem + non-goals

**Problem.** The live schema stores all money as Postgres `integer` (int4) minor units with `CHECK (>= 0)`
across **16 tables** — no floating-point or numeric money survives (inventory/12 §1 "Money reality check";
§10 R3). The Node/TS layer carries money as a bare `number`, so non-negativity and overflow-safety are
enforced only at the DB CHECK and scattered app guards — not in the type. The Rust rebuild's first money
artifact is a domain newtype that pulls those invariants into the compiler: a value that is negative, or an
arithmetic that silently wraps/panics, must not be *representable*.

**In scope (this decision):**
- The pure value type + its constructor + its arithmetic API + its wire (serde) contract + its unit-test DoD.
- Resolving the internal doc conflict on width/name/sqlx-coupling (see §3, §4).

**Non-goals (explicitly out — do NOT build here):**
- **No persistence.** No `sqlx::Type`, no decode/encode, no column mapping. That is Phase B S5 (§5).
- **No call-site adoption.** No order/payment/courier code touches this type yet (§10 risk O-1).
- **No division / percentage math.** The stored money fields are absolute integers (`subtotal`,
  `delivery_fee`, `tip_amount`, `discount_total`, `total`, `price_snapshot`, `amount_minor` …); add + sub +
  multiply-by-quantity cover Phase A. Percentage/basis-point math (which introduces a rounding-mode
  decision) is deferred (§10 risk O-4). YAGNI — "schema rich, runtime minimal."
- **No multi-currency.** Single-currency system (Albanian Lek). `exchange_rates` (numeric) is a latent seam,
  not a money-value type today (§10 risk O-3).

## 2. Back-of-envelope (why the sizing driver is *value magnitude*, not throughput)

**Throughput is a non-driver.** DeliveryOS is a multi-tenant food-delivery platform for Albania: tens–low
hundreds of venues; peak order intake is low hundreds/min platform-wide; a handful of money ops per order
(subtotal build, fee, tip, discount, total). Each op is one or two integer instructions with zero allocation.
There is no numeric hot loop. So the type's *speed* is irrelevant — the sizing question is **how big can a
value get**, because that decides the integer width.

**Connection budget: N/A.** This crate opens zero connections. The `API + worker + analytics + migrations`
connect budget does not apply — it is a pure CPU/compile-time type with no pool handle. (Persistence, when it
lands in Phase B, rides the existing `PgPool` A/B plan from inventory/12 §7 — it adds no new pool.)

**Scale correction (Round-1 breaker H-1 — this number was wrong by 100× in the first draft).** Albanian Lek
is stored at **scale 0** — whole Lek, no live subunit (the qindarka is defunct): the currency minor-unit
defaults to `0` (`apps/api/src/lib/preview-render.ts:47` and `ssr-renderer.ts:304` `?? 0`; `csv-parser.ts:12`
`currencyMinorUnit = 0`; the tax helper's `_minorUnit` is dead code, `apps/api/src/lib/money.ts:1`). So for
today's single-currency system **"minor unit" == "whole Lek"** — there is no ×100. The first draft computed
the aggregation row as Lek×100 and asserted a single location-year ≈ 1.46e10 minor units; at scale 0 the true
figure is **1.46e8**, which *fits* int4. The corrected envelope, and the corrected i64 justification:

**Width envelope — the actual decision input (scale-0 corrected):**

| Quantity | Magnitude (whole Lek == minor units at scale 0) | Fits int4 (2.147e9)? | Fits i64 (9.2e18)? |
|---|---|---|---|
| One order total / line item (persisted VALUE, DB `CHECK ≥ 0`) | ≤ ~1e5–1e6 typical; int4-bounded by the column | ✅ by construction | ✅ |
| `price × quantity` intermediate | real catering ~5e3 × ~1e3 = **5e6** (fits); adversarial-but-non-overflowing qty (e.g. 1e3 × 3e6 = **3e9**) | ⚠️ **adversarial case exceeds int4** | ✅ |
| In-domain **aggregation** (one location's order totals, a year) | 400k Lek/day × 365 = **1.46e8 Lek** | ✅ **fits int4 (~14.7× headroom)** | ✅ |
| ~14 location-years, or cross-location / platform revenue rollup, settlement/payout sums | 1e11–1e13 | ❌ **exceeds int4** | ✅ |

**Reading of the envelope (corrected).** A single location-year sum **fits** int4 — so the first draft's
headline "a single location-year already blows int4" was false under scale-0 Lek. i64 over i32 still wins, but
on the *honest* set of reasons, not that one:
1. **Rollups actually exceed int4.** ~14 location-years of a naive running sum, and any cross-location /
   platform revenue rollup or settlement/payout sum (1e11–1e13), overflow int4. The domain type is exactly
   where these sums happen; an i32 type would return `Err` on a *legitimate* platform rollup.
2. **Adversarial-but-non-overflowing intermediates.** A crafted `price × qty` can cross int4 (~3e9) while
   being a well-formed i64 — an i32 type would `Err` on it as "too small," muddying "i32 overflowed" with
   "this money is wrong."
3. **Multi-currency headroom is not free at i32.** The moment any *scale-2* currency enters (the latent
   `exchange_rates` seam, O-3), every value regains the ×100 — a single scale-2 location-year is ~1.46e10,
   the very number the first draft wrongly attributed to Lek, and it blows int4. i64 absorbs that future
   without a domain-type change.
4. **Defensive headroom is free at the domain layer.** The storage width (int4) is enforced *separately* at
   the Phase-B write boundary (§5), so widening the domain type to i64 costs nothing at rest. i64 moves the
   overflow boundary to ~9.2e18, a magnitude no real or plausibly-adversarial DeliveryOS value reaches — so a
   `checked_*` `Err` reliably means "genuine bug/attack," which is what we want an error to mean. At i32 that
   same `Err` would too often mean "the width was too small," which is a false signal on a 🔴 money type.

**Conclusion (unchanged decision, corrected reasoning):** width = **i64** for the domain type; the int4
storage width is enforced separately, at the write boundary in Phase B (§5). Consistent with REBUILD-MAP §1
(`Lek(i64)`); supersedes inventory/12 §9's `Minor(i32)` — reconciled in §4, not silently overridden.

## 3. Options (≥2, with the named concept for each)

Two orthogonal axes: **representation** (what holds the value) and **arithmetic API** (how values combine).

### Axis 1 — representation

- **R1 · integer minor units (CHOSEN).** *Concept: fixed-point integer money ("store the smallest unit").*
  Exact; matches the only money representation in the live schema (16 tables, all int minor + `CHECK ≥ 0`);
  zero impedance at the DB boundary except a fail-loud int4 narrowing on write. No rounding error can enter
  because there is no fraction and (Phase A) no division.
- **R2 · `rust_decimal::Decimal`.** *Concept: arbitrary-precision base-10 decimal.* Correct for money too,
  but heavier (128-bit + scale), and since the DB column is `integer` a Decimal domain type would need a
  conversion + a rounding-mode decision *anyway* on every read/write — reintroducing exactly the rounding
  question integer minor units eliminate. Justified only for genuinely fractional rates: inventory/12 §9
  reserves Decimal for the ONE such column, `exchange_rates.rate`. Not money.
- **R3 · `f64` / `f32`.** *Concept: binary floating point.* **Rejected categorically** (inventory/12 §10 R3:
  "Any Rust type other than a checked integer newtype is a regression"). Binary floats cannot represent 0.10
  exactly; sums drift; comparisons lie. The proposed design's deliberate absence of any `From<f64>/From<f32>`
  is the compile-time enforcement of this rejection — float construction must not compile.

**Width sub-choice (i32 vs i64) within R1:**
- *i32* — matches int4 exactly; the type *is* the column, so a Phase-B sqlx type could decode with zero
  narrowing. But intermediates and sums overflow int4 (§2) → spurious `Err`s on valid money.
- *i64 (CHOSEN)* — headroom for arithmetic/aggregation; cost is a fail-loud int4 narrowing at the persistence
  write boundary (Phase B). The narrowing is a *feature*: a computed value that can't fit int4 also can't
  satisfy the DB range, so catching it at the app boundary with a typed error beats a Postgres `22003`.

### Axis 2 — arithmetic API

- **A1 · Checked-Result API (CHOSEN).** *Concept: errors-as-values / total functions / "make illegal states
  unrepresentable" / parse-don't-validate.* `checked_add`/`checked_sub`/`checked_mul_qty` each return
  `Result<Lek, MoneyError>`; **no `std::ops::Add`/`Sub` impls at all** (their presence would advertise
  infallible, panicking/wrapping semantics). Overflow and sign violations become values the caller must
  handle — they compose with axum's `IntoResponse` error enum (REBUILD-MAP §1: 68 codes → one enum).
  Cost: `?`-verbose call sites. Benefit: on money, a handled error beats both a panic and a silent wrap.
- **A2 · operator overload + `debug_assert!`.** *Concept: ergonomic infix + fail-fast-in-debug.* Impl `Add`/
  `Sub`, `debug_assert!` non-negativity, use `checked_*` internally, panic on overflow. **Rejected:**
  `debug_assert!` is compiled *out* in release, so in production the invariant is unenforced and arithmetic
  either **panics** (overflow-checks on → an availability incident inside a request handler) or **silently
  wraps** (overflow-checks off → a wrong charge, a 🔴 correctness regression). A money type that can wrap in
  release is exactly the failure this whole seam exists to prevent.
- **A3 · saturating arithmetic.** *Concept: total functions via clamping.* `saturating_add`/`_sub`/`_mul`
  never error and never panic. **Rejected:** they clamp *silently* — an order total that should be X becomes
  `i64::MAX` or floors at 0 with no signal. Silent wrong money with no error is worse than a handled error;
  it violates "server authoritative for price/status" + fail-loud.

### Wire (serde) contract — a defect found in the proposed design, and its fix

The proposed design was `#[derive(… Serialize, Deserialize)] #[serde(transparent)]`. **`#[serde(transparent)]`
makes `Deserialize` bypass the constructor** — it decodes the inner `i64` and wraps it directly, so a
negative JSON integer (`-100`) would deserialize into `Lek(-100)` **without** going through `Lek::new`,
violating the core non-negativity invariant on the wire. The proposed test list (float/string rejected)
does not cover this because transparent derive *can't* reject it. **Fix (refinement, part of this proposal):**
keep `Serialize` emitting a bare integer, but **hand-write `Deserialize`** to route through the fallible
constructor — deserialize an `i64`, call `Lek::new`, and map `MoneyError` to `serde::de::Error::custom`. This
keeps parse-don't-validate true at the trust boundary: a `Lek` is *always* non-negative, including one built
from JSON. Add a unit test that `-1` (bare integer) is rejected at deserialization (§ DoD). Float rejection
(`100.5`, and `100.0` — serde_json refuses a fractional token into `i64`) and string rejection (`"100"`)
already hold via the inner `i64` deserialize; the negative case is the one the transparent derive dropped.

**KNOWN LIMITATION — bare-integer `Serialize` and the f64 cliff (Round-1 breaker H-2; accepted for Phase A,
must-solve before any browser-touching cutover).** `Serialize` emits a **bare JSON integer**. Rust-to-Rust
this is exact — `serde_json` round-trips any `i64` losslessly (the DoD adds a test pinning a value **above
2^53**, e.g. `Lek(9_000_000_000_000_000)`, through `to_string`→`from_str` to prove it). But **JavaScript**
`JSON.parse` decodes JSON numbers to IEEE-754 `f64`, which loses integer precision above **2^53 ≈ 9.007e15**.
Any value in `(9.007e15, i64::MAX]` — constructible without an `Overflow` `Err` — would **silently round** when
parsed by the existing Node/TS layer or a browser. This is *silent wrong money*, reintroduced at a cross-
language wire boundary, in exactly the headroom band i64 exists to hold. Two facts bound the risk for Phase A,
so we **accept it now and do not block**: (a) **no JS consumer touches this type in Phase A** — it is an
isolated, un-wired scaffold with zero call sites; (b) at the corrected scale-0 magnitudes (§2), real Lek
rollups top out ~1e11–1e13, comfortably below 2^53, so only an adversarial/bug value in `(9e15, i64::MAX]`
demonstrates corruption. The **must-solve-before-cutover** obligation is recorded as risk **O-7** (§10) and
owned by the S5 lead: at the *first* JSON boundary a browser or the Node layer touches, either (i) emit money
as a **string** over that boundary (BigInt-safe) and teach `Deserialize` to accept the string form there, or
(ii) prove every value crossing it is bounded into the f64-safe range. Phase A ships the Rust-exact contract
plus this documented obligation; it does **not** ship the cross-language mitigation (no boundary exists yet).

## 4. Decision + why it dominates (ADR-format → docs/adr/ADR-rust-money-newtype.md)

**Decision:** `Lek(i64)` — **R1 integer minor units, i64 width, A1 checked-Result API, validating
`Deserialize` + bare-integer `Serialize`, and deliberately no `From<f64>/From<f32>` and no `Add`/`Sub`
traits.** Errors via `enum MoneyError { Negative(i64), NegativeQuantity(i64), Overflow { op: &'static str,
lhs: i64, rhs: i64 } }` — the `Overflow` variant **carries the operation name and both operands** (Round-1
breaker M-1: a payload-less `Overflow` is the least-diagnosable variant yet the one the design most wants to
alert on; `Display` renders e.g. `money overflow in mul_qty: 1000 * 9000000000000`). Distinct
`NegativeQuantity` so a bad qty and a bad amount give different messages; all implement `std::error::Error` +
`Display` so the enum slots into the one Rust error enum. Constructor `Lek::new(minor_units: i64) ->
Result<Self, MoneyError>` rejects negatives; `checked_add` / `checked_sub` (also rejects negative results) /
`checked_mul_qty(qty: i64)` return `Result<Lek, MoneyError>`. **A `pub const ZERO: Lek = Lek(0)`** is exposed
so call sites that must *explicitly* floor a legitimate over-discount to zero write a visible, greppable
`.unwrap_or(Lek::ZERO)` — a deliberate choice over a hidden saturating method (M-5; see §7). **Derives:
`Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize` — `Hash` is dropped** (Round-1 breaker L-2:
no Phase-A consumer and money-as-map-key is a smell; re-add only when a keyed collection needs it).

**Implementation constraint pinned for the implementer (Round-1 breaker M-2).** `checked_mul_qty` MUST detect
a negative quantity with a **`qty < 0` comparison and early-return `Err(NegativeQuantity(qty))` BEFORE any
multiplication or magnitude normalization**. It MUST NOT use `qty.abs()` / `qty.unsigned_abs()` anywhere — for
`i64::MIN` those overflow (panic with overflow-checks on — the exact A2-rejected in-handler panic — or wrap to
`i64::MIN` with checks off, a silent sign flip). The DoD pins `checked_mul_qty(i64::MIN)` →
`Err(NegativeQuantity(i64::MIN))`, proving multiplication is never reached.

**Why it dominates the alternatives:**
1. **Matches the only money representation in the live schema.** 16 tables, all int minor units + `CHECK ≥ 0`
   (inventory/12 §1, §10 R3). Zero representation impedance; the sole boundary cost is a fail-loud int4
   narrowing on write (§5), which is strictly better than a Postgres `22003` at the driver.
2. **Errors-as-values, never panic, never silent wrap.** A panicking money op in a request handler is an
   availability incident (A2 in release with overflow-checks on); a wrapping one is a wrong charge (A2 with
   checks off; A3 by clamping). A1 makes both caller-handled `Result`s that compose with `IntoResponse`.
3. **i64 fits in-domain arithmetic/aggregation; int4-width is enforced where it belongs.** §2 shows a single
   location-year sum exceeds int4 — an i32 domain type would `Err` on valid money. i64 reserves `Err` for
   genuine overflow; the persistence write narrows to int4 with a typed failure (§5).
4. **The invariant holds even from the wire — but it is a *sign* guarantee, not an *authority* one (Round-1
   breaker M-3).** Validating `Deserialize` closes the transparent-derive hole, so no `Lek` in the process —
   constructed or deserialized — is ever negative. It guarantees **non-negativity (sign) only**; it does
   **not** make a deserialized `Lek` *trustworthy money*. A hostile client can still post `{"total": 1}` and
   get a perfectly valid `Lek(1)` — well-formed, but not authoritative. **Server-authority over the *amount*
   stays where it lives today** — at the order-create transaction (server recomputes the charge; the cash-422
   backstop; `packages/ui/src/lib/money.ts` "the SERVER stays the single source of truth for what is
   CHARGED") — and is **out of scope for this scalar**. This proposal explicitly does *not* license a Phase-B
   call site to trust a wire `Lek` as a charge because "it's a validated `Lek`"; a validated `Lek` is a
   well-formed magnitude, never an authoritative one.
5. **Resolves the internal doc conflict, explicitly.** REBUILD-MAP §1 says `Lek(i64)`; inventory/12 §9 says
   `Minor(i32)` + `#[sqlx(transparent)]`. This proposal adopts REBUILD-MAP's `Lek(i64)` for the **domain**
   type and re-reads inventory/12 §9's `i32` as the **storage width**, not the domain-type width — the two
   meet at the Phase-B decode/encode boundary (§5). Documented supersession, not a silent override.

## 5. Data / migrations

**N/A — no schema is touched.** This is a pure Rust value type; the DB is UNCHANGED (06-doc: "code-only
against the live schema; data never migrates"). No migration, forward-only or otherwise, is created or run.

**Phase-B persistence note (recorded, NOT built here).** When `Lek` is wired to sqlx in Phase B S5, the int4
column ↔ i64 domain bridge is:
- **Read (decode):** the `integer` column decodes to `i32`; map via `TryFrom<i32> for Lek` (i.e.
  `#[sqlx(try_from = "i32")]`-style), which routes through `Lek::new` and so re-checks non-negativity at the
  decode boundary too. (A transparent `i64` newtype would map to BIGINT and *fail* to decode an int4 column —
  a concrete reason the domain type and any sqlx wiring are distinct concerns.)
- **Write (encode):** narrow the i64 value with `i32::try_from(value)`; on `Err`, a typed overflow error, not
  a silent truncation — a value that can't fit int4 also can't satisfy the column's range, so it is a bug
  worth failing loudly. This narrowing is the accepted cost of the i64 domain width (§10 risk O-2).
- The DB `CHECK (>= 0)` remains the belt-and-suspenders backstop under the type invariant.

## 6. Consistency + idempotency

**N/A for this type — and the rationale matters.** `Lek` is an immutable, `Copy` scalar value; it holds no
state, so there is no read-modify-write, no cross-request ordering, no CAP tradeoff, and nothing to make
idempotent. Consistency/idempotency for money live one layer up, at the *order-create transaction* — the
`idempotency_keys` PK `(location_id, key)` + claim-first single-txn contract (ADR-0007 §4; inventory/12 §4).
Those are honored by the orders/money surface in Phase B S5, not by the value type. Keeping them out of the
scalar is deliberate: conflating value-semantics with transaction-semantics is a category error that would
bloat the type past need.

## 7. Failure + degradation (every path is caller-handled; zero panic, zero cascade)

The type has no external calls, so there is no timeout/fallback matrix in the classic sense — its "external
failure surface" is arithmetic overflow and invalid input. Failure-first design:

| Trigger | Behavior | Why not worse |
|---|---|---|
| Negative construction (`Lek::new(-1)`) | `Err(MoneyError::Negative(-1))` | Not `panic!`, not clamp-to-0 (silent) |
| `checked_add` overflow (near i64::MAX) | `Err(MoneyError::Overflow { op: "add", lhs, rhs })` | Not wrap (A3/A2-release), not `panic!`; **carries op+operands** so the alert is diagnosable <1 min (M-1) |
| `checked_sub` goes negative | `Err(MoneyError::Negative(result))` | A refund/discount that would go below zero is a caught business error, not an underflow. **No clamp-to-zero method** (M-5): a legitimate over-discount → free order is floored *explicitly* at the call site via `.unwrap_or(Lek::ZERO)`, never by a silent saturating primitive. This matches production `assertNonNegative` (throws, does not clamp — `apps/api/src/lib/money.ts:32`); the clamp-vs-error choice stays visible at the business layer, not hidden in the type. |
| `checked_mul_qty(negative qty)` | `Err(MoneyError::NegativeQuantity(qty))` | Distinct from a negative amount for a clear message |
| `checked_mul_qty(i64::MIN)` | `Err(MoneyError::NegativeQuantity(i64::MIN))` | `qty < 0` early-returns **before** any multiply/`abs` — never panics or wraps on the `abs`-of-`MIN` trap (M-2) |
| `checked_mul_qty` overflow | `Err(MoneyError::Overflow { op: "mul_qty", lhs: self.0, rhs: qty })` | Not wrap, not `panic!`; carries op+operands (M-1) |
| Wire: negative / float / string JSON | `Err` at `Deserialize` (via `de::Error::custom`) | Bad input never becomes a live `Lek` — but note (M-3) this checks *sign*, not *authority*; the server still owns the amount |

**No cascade:** every failure is a local `Result` the caller must handle at the point of use. In Phase B this
means an axum handler maps `MoneyError` to a 4xx/5xx via the shared error enum — never an unhandled panic
that would 500 the request or (worse) poison a shared worker. The degradation story for the *feature* that
uses money (checkout) is designed at the order surface, not here.

## 8. Security + tenant isolation

- **No PII, no secrets, no tenant dimension *in the scalar itself*.** `Lek` is a context-free integer; it
  carries no customer data and no credential. The Ethics Charter (no military/warfare use; commons) is
  trivially satisfied by a currency value type. **Narrowing (Round-1 breaker L-1):** this is *not* a blanket
  "money is never sensitive" waiver. A bare amount is weak on its own, but once a `Lek` is **joined to
  customer/order context downstream**, an order total becomes financial data under the project's GDPR /
  claim-check posture (menu-only into AI prompts, queues, analytics egress). Redaction is therefore a
  **call-site / egress-boundary** concern, not a type property — the type stays a plain scalar (`Debug` remains
  derived, needed for test assertions and dev logs; `MoneyError` still carries the amount for diagnosability).
  The obligation to redact amounts at the known AI/queue/analytics choke points is recorded as risk **O-8**
  (§10), owned by the S5 / egress lead; Phase A neither ships nor needs that seam (no egress path exists yet).
- **Tenant isolation is N/A *by design*.** Money values are tenant-agnostic; tenant scoping is a property of
  the *row/query* layer (RLS FORCE + txn-scoped GUCs, inventory/12 §2/§7), not of a scalar. Putting a
  `location_id` inside the money type would be a category error and would not add isolation — it would just
  couple the value to context it should not know.
- **The one untrusted boundary is serde**, and that is exactly why the validating `Deserialize` (§3) is a
  security-relevant refinement: it prevents a hostile/negative wire value from entering as a valid `Lek`.

## 9. Operability

- **A compile-time guarantee replaces a runtime guard.** Today (Node/TS) negative-money and overflow are
  caught, if at all, at the DB `CHECK` and ad-hoc app guards. The newtype makes negative-money
  *unconstructable* and silent overflow *uncompilable* (no `Add`/`Sub`, no `From<f64>`) — a class of runtime
  guard becomes a type property. That is the operability win: fewer runtime failure modes to observe.
- **Observability (<1 min):** `MoneyError` implements `Display` + `Error` and names the failing op/value, so
  when a `checked_*` `Err` does surface at a Phase-B call site it maps cleanly into the one error enum and is
  loggable/alertable without extra plumbing. (No metric/telemetry of its own — it is a pure type.)
- **Health degraded-vs-down:** N/A (no runtime, no health surface).
- **Rollback:** trivial — the file is isolated scaffold in a worktree; deleting it is a full rollback. No
  migration, no deploy, no data touched.
- **Flag / scaling-gate:** none needed. The type is *dark by absence* — nothing calls it until Phase B S5
  wires the first money call-site. "Schema rich, runtime minimal": the invariant seam exists now; runtime
  enforcement activates when the first call-site adopts `Lek`.

## 10. Open / accepted risks (each with owner + justification)

| # | Risk | Disposition | Owner |
|---|---|---|---|
| **O-1** | No compile-time enforcement that call sites use `Lek` instead of raw `i64` yet — a Phase-B route could still pass a bare `i64` around money. | **Accept now / lint pulled FORWARD (Counsel §3.1).** Lands with the **first** real money call-site in Phase B S5 — **not** a "later ratchet." An escape hatch left open makes the safety type optional in practice (convergence theater); the clippy/dylint "no raw-`i64` money param" rule must land *alongside* the first `Lek` signature, not after several. | S5 orders/money council lead |
| **O-2** | i64 domain width exceeds int4 storage width → a `Lek` can hold a value that cannot persist. | **Accept.** The Phase-B write boundary narrows via `i32::try_from` with a typed overflow error (§5). A value exceeding int4 also violates the DB range, so failing loud at the app edge is correct, not a regression. | S5 lead |
| **O-3** | Single-currency assumption (all money = Albanian Lek). `exchange_rates` (numeric) is a latent multi-currency seam. | **Accept / contingency.** Every money *value* is Lek today. If multi-currency ever ships, `Lek` gains a sibling type or a currency tag; not built now (YAGNI). | Product |
| **O-4** | No division / percentage / basis-point method → in-domain %-based tips/discounts/fees are not expressible. | **Accept / defer.** The relevant DB fields are stored as absolute integers; add+sub+mul_qty suffice for A. A half-up rounding method is introduced only if/when %-math is computed in-domain (Phase B). | S5 lead |
| **O-5** | A future edit could reintroduce `#[serde(transparent)]` on `Deserialize`, silently dropping the wire non-negativity check. | **Mitigate with a guardrail.** The DoD includes a unit test asserting `-1` JSON is rejected at deserialization (red→green per the repo's self-improvement loop); it fails if transparent derive returns. | Crate maintainer |
| **O-6** | Internal doc conflict REBUILD-MAP (`Lek(i64)`) vs inventory/12 §9 (`Minor(i32)`). | **Resolved.** This proposal adopts `Lek(i64)` for the domain type and re-reads §9's `i32` as the storage width (§4.5, §5). inventory/12 §9's line should be annotated as "storage width; domain type is `Lek(i64)`" when next edited. The frame docs (`06-…`, `REBUILD-MAP`, `inventory/12`) live in the **main tree, not this scaffold worktree** (confirmed: `docs/design/rebuild-plan/**` glob returns 0 files here; breaker L-3) — a worktree-sync fact, not a doc-integrity defect; the reconciliation is verifiable against the main checkout and re-checkable when this branch merges. | This proposal / merge lead |
| **O-7** | **f64 cliff at a cross-language JSON boundary (breaker H-2).** Bare-integer `Serialize` silently rounds any value in `(2^53, i64::MAX]` when parsed by JS (`JSON.parse`→f64) — the existing Node/TS layer and every browser. Rust-to-Rust is exact. | **Accept for Phase A (no JS consumer, un-wired scaffold) / MUST-SOLVE before cutover.** DoD adds a >2^53 exact-round-trip test proving the Rust contract. At the **first** browser/Node-touching JSON boundary, either emit money as a **string** (BigInt-safe, teach `Deserialize` the string form there) or prove every crossing value is f64-safe-bounded. Do not wire a JS consumer to bare-int money without this. | S5 orders/money lead |
| **O-8** | **Amount + downstream context = financial data (breaker L-1).** The scalar is context-free, but a `Lek` joined to customer/order context and egressed un-redacted (AI prompts, queues, analytics) is GDPR-relevant financial data; the type carries no redaction signal. | **Accept / defer to egress boundary.** Redaction is a call-site/egress concern, not a scalar property; enforced at the known choke points in Phase B, not in the value type. No egress path exists in Phase A. | S5 / egress lead |
| **O-9** | **`as i32` write-narrowing footgun (breaker M-4).** The Phase-B write path must use `i32::try_from` (a `Result`); the always-available `value as i32` cast silently wraps a >int4 `Lek` to a **negative** i32 → a `CHECK (>= 0)` violation or, on any un-checked path, negative money persisted. No guard exists in Phase A. | **Defer-flag → MISSING for the Phase-B write-boundary lane.** No i32 storage-boundary code exists yet. When it lands: `i32::try_from` only, and a clippy/dylint rule banning `as`-casts on money (bundled with the O-1 lint) must land in the **same** lane as the first write path. | S5 lead |
| **O-10** | **No home for *directional / owed* money (Counsel §5 open question).** `Lek` is unsigned by invariant, so "platform owes user X" (refund > order, store credit, courier payout) is *not expressible* as a `Lek`. The path of least resistance for a future dev hitting `checked_sub → Err(Negative)` on a legitimate refund is to floor to zero — silently costing the customer/courier money they were owed. | **Named Phase-B deliverable (recommended) — HUMAN DECISION flagged (§10a).** Not a Phase-A change (the scalar stays a magnitude). Recommend a conscious Phase-B design item — a `SignedLek`/`Delta`/ledger-entry type — designed *before* the first Rust refund/payout path, so "refund exceeds order" is a modelled business outcome, never a silent floor. | S5 orders/money council lead |

## 10a. Human decision flagged (Counsel §5 open question — no ETHICAL-STOP, but a conscious bet)

Counsel raised **no ETHICAL-STOP**, but one open question is genuinely a human/product call, not an
architect's to close unilaterally: **where does *directional/owed* money live, and is it a Phase-B
deliverable with a named owner *now*, or a latent risk accepted until it bites?** (O-10). The fairness stake
is real — the unsigned invariant makes "user pays" fluent and "platform owes back" awkward, and awkwardness
resolves against the customer and courier, the two parties least able to notice a floored refund.

**Architect recommendation (for the human to accept or override):** make it a **named Phase-B deliverable**
owned by the S5 orders/money council, designed *before* the first Rust refund/payout path — a `SignedLek` /
`Delta` / ledger-entry type. Do **not** accept it as latent risk: a missing directional type is exactly the
condition under which "refund exceeds order" gets silently floored to zero. This does not change the Phase-A
scalar (it stays a magnitude). **Owner of the human decision:** product + S5 lead.

*(Counsel §4 steel-man, recorded for honesty: the rejection of A2 — operator overload — is correct, but A2's
**ergonomics** critique survives its dead **enforcement mechanism** (`debug_assert!` compiles out). The
Result-API still wins on genuine grounds — `checked_sub`-below-zero is a business error you want as a handled
value not a panic; `Result` composes into the one `IntoResponse` enum; one uniform API across all four ops —
not over a strawman. No design change; the `?`-verbosity cost is real and is the reason the O-1 lint is pulled
forward, so the safety is never quietly routed around.)*

*(Counsel §3.3, Qty newtype: **agreed YAGNI** — `checked_mul_qty(qty: i64)` keeps a raw `i64` quantity in
Phase A. A `Qty` newtype is noted as a Phase-B candidate so the eventual choice is deliberate, not accidental;
not built now.)*

## DoD — proof deferred to implementation (STOP-CODE-A), test list specified now

Per the Mandatory Proof Rule, the *change* (code) must ship with programmatic proof; this design doc is not a
behavior change, so proof is deferred to the code step. The unit-test set that will prove `Lek` red→green:
1. Construction: `new(0)`, `new(1)`, `new(i64::MAX)` → `Ok`; `new(-1)` → `Err(Negative)`. `Lek::ZERO ==
   Lek::new(0).unwrap()` (M-5 explicit-floor primitive exists).
2. `checked_add(i64::MAX, 1)` → `Err(Overflow { op: "add", .. })`; a normal add → `Ok(sum)`.
3. `checked_sub` going below zero → `Err(Negative)`; a normal sub → `Ok`. (No clamp method exists — the
   over-discount floor is `x.checked_sub(d).unwrap_or(Lek::ZERO)` at the call site, M-5.)
4. `checked_mul_qty` overflow → `Err(Overflow { op: "mul_qty", .. })`; negative qty (`-5`) →
   `Err(NegativeQuantity(-5))`; normal → `Ok`.
5. **`checked_mul_qty(i64::MIN)` → `Err(NegativeQuantity(i64::MIN))`** — pins the `abs`-of-`MIN` landmine
   shut: the `qty < 0` guard returns before any multiply, so no panic/wrap (M-2).
6. serde round-trip: `Lek(1500)` ⇄ bare JSON integer `1500`.
7. **serde exact-i64 round-trip above 2^53:** `Lek(9_000_000_000_000_000)` → `to_string` → `from_str` →
   equal — proves the Rust-to-Rust contract is lossless in the i64 headroom band (H-2; the *cross-language* JS
   f64 hazard is the documented O-7 must-solve-before-cutover, out of scope for this Rust-only test).
8. Deserialize rejects float `100.5` **and** `100.0`, string `"100"`, **and negative `-1`** (O-5 guardrail).
9. `MoneyError` `Display`/`Error` renders the failing op **and operands** — the `Overflow` case asserts the
   rendered string contains the op name and both operands (M-1 diagnosability).

(Phase B adds: `TryFrom<i32>` decode round-trip; the `i32::try_from` write-narrowing rejection test; the
`as`-cast-on-money lint (O-9); the raw-`i64`-money-param lint landing with the first call-site (O-1); and, at
the first JS-touching JSON boundary, the string-encoding-or-f64-bounded mitigation + its test (O-7).)
