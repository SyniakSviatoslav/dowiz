# ADR: Rust `domain` money newtype `Lek(i64)` — checked-Result, integer minor units

**Status:** PROPOSED — Round-1 breaker + counsel RESOLVED (Phase A scaffold — design-time only; no code shipped)
**Resolution:** `docs/design/rust-money-newtype-phase-a/resolution.md` (10 findings dispositioned; H-2/M-4 → Phase-B; O-10 human-decision flagged)
**Date:** 2026-07-04
**Red-line:** 🔴 money / correctness
**Supersedes:** nothing · **Reconciles:** REBUILD-MAP.md §1 (`Lek(i64)`) vs inventory/12 §9 (`Minor(i32)`)
**Companion design:** `docs/design/rust-money-newtype-phase-a/proposal.md`
**Frame:** `docs/design/rebuild-plan/06-complete-rebuild-stack.md`; `.../inventory/12-data-layer.md` §1/§9/§10-R3

## Context

The Rust rebuild (code-only against the UNCHANGED live schema) needs its first money type. The live schema
stores all money as Postgres `integer` (int4) minor units with `CHECK (>= 0)` across **16 tables**; no float
or numeric money exists (inventory/12 §1, §10 R3: "Any Rust type other than a checked integer newtype is a
regression"). Today's Node/TS layer carries money as a bare `number` — invariants live only at the DB CHECK
and scattered guards. The rebuild's opportunity is to make "negative money" and "silent overflow"
*uncompilable*, in a pure `domain` crate value type with **no IO, no sqlx, no tokio**. This lands as isolated
worktree scaffold: nothing merged, no DB writes, not yet wired to any call site.

Two canon docs conflicted: REBUILD-MAP §1 carries `Lek(i64)`; inventory/12 §9 carries `Minor(i32)` +
`#[sqlx(transparent)]`. They disagree on name, width, and sqlx-coupling — this ADR resolves it.

## Decision

Adopt a single pure-domain newtype **`Lek(i64)`**:

- **Representation:** integer minor units (fixed-point). **No `From<f64>`/`From<f32>`** anywhere — float
  construction must not compile. Decimal is reserved for genuinely fractional rates (`exchange_rates.rate`),
  not money.
- **Width i64, not i32.** *(Sizing corrected after Round-1 breaker H-1: Albanian Lek is stored at **scale 0**
  — whole Lek, no live subunit; `minor_unit ?? 0` — so "minor unit" == "whole Lek". The first draft's headline
  "a single location-year ≈ 1.46e10 minor units already exceeds int4" was wrong by 100× — at scale 0 that sum
  is **1.46e8**, which **fits** int4 with ~14.7× headroom. The decision does not change; the reasoning does.)*
  i64 over i32 is justified on the honest set of grounds: (1) **rollups that truly exceed int4** — ~14
  location-years of a running sum, and any cross-location/platform/settlement rollup (1e11–1e13); (2)
  **adversarial-but-non-overflowing `price × qty` intermediates** (~3e9) that cross int4 while being valid i64;
  (3) **multi-currency headroom** — any future scale-2 currency (the latent `exchange_rates` seam) re-adds the
  ×100, at which point a single location-year *is* ~1.46e10 and blows int4; (4) **free defensive headroom** —
  the int4 *storage* width is enforced separately at the Phase-B write boundary, so widening the domain type
  costs nothing at rest and keeps a `checked_*` `Err` meaning "genuine bug/attack" (~9.2e18 boundary) rather
  than the false "the width was too small" signal an i32 type would emit on a legitimate rollup. This adopts
  REBUILD-MAP's `Lek(i64)` and re-reads inventory/12 §9's `i32` as the storage width, not the domain-type width.
- **Checked-Result arithmetic only.** `Lek::new(minor_units: i64) -> Result<Self, MoneyError>` rejects
  negatives; `checked_add` / `checked_sub` (rejects negative result) / `checked_mul_qty(qty: i64)` each return
  `Result<Lek, MoneyError>`. **No `std::ops::Add`/`Sub` trait impls** — their presence would imply infallible,
  panicking/wrapping semantics. Errors:
  `enum MoneyError { Negative(i64), NegativeQuantity(i64), Overflow { op: &'static str, lhs: i64, rhs: i64 } }`
  implementing `Error` + `Display`, so it composes with the one Rust error enum + `IntoResponse`. The
  `Overflow` variant **carries the operation name and both operands** (Round-1 breaker M-1: a payload-less
  `Overflow` is the least-diagnosable variant yet the one most worth alerting on). **`checked_mul_qty` MUST
  detect negative qty with a `qty < 0` early-return BEFORE any multiply — never `qty.abs()`/`.unsigned_abs()`**
  (M-2: `i64::MIN.abs()` panics or wraps). A **`pub const ZERO: Lek`** is exposed so a legitimate over-discount
  is floored *explicitly and visibly* at the call site (`.unwrap_or(Lek::ZERO)`) — no silent saturating method
  (M-5: matches production `assertNonNegative`, which throws, not clamps).
- **Serde:** `Serialize` emits a bare JSON integer; **`Deserialize` is hand-written to route through
  `Lek::new`** (deserialize `i64` → `new` → map `MoneyError` to `de::Error::custom`). This closes a defect in
  the originally-proposed `#[serde(transparent)]` derive, which would have wrapped a negative wire value
  (`-100`) into a valid-looking `Lek(-100)` *without* the non-negativity check. Float (`100.5`, `100.0`) and
  string (`"100"`) rejection already hold via the inner `i64` decode; the negative case is the one the
  transparent derive silently dropped. **Two clarifications from Round-1:** (a) the validating `Deserialize`
  guarantees **sign only, never authority** — a deserialized `Lek(1)` is a well-formed magnitude, not a trusted
  charge; server-authority over the *amount* stays at the order-create txn (M-3). (b) **KNOWN LIMITATION
  (H-2):** the bare-integer form is exact Rust-to-Rust but a JS `JSON.parse` decodes to f64 and silently rounds
  values above 2^53; **no JS consumer touches this type in Phase A**, so it is accepted now and recorded as the
  must-solve-before-cutover risk O-7 (string-encode or f64-bound at the first browser-touching JSON boundary).
- **Derives:** `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord` (+ `Serialize`; hand-written
  `Deserialize`). **`Hash` is dropped** (Round-1 breaker L-2: no Phase-A consumer; money-as-map-key is a smell
  — re-add only when a keyed collection needs it).

Rejected alternatives: operator-overload + `debug_assert!` (debug asserts compile out in release → prod
either panics inside a handler or silently wraps money — a 🔴 regression); saturating arithmetic (silently
clamps → wrong money with no error signal); `f64`/`Decimal` representation (float can't represent 0.10; Decimal
reintroduces a rounding-mode decision against an integer column). Full tradeoff matrix in the companion design
§3.

## Consequences

- Negative money is unconstructable; silent overflow is uncompilable; float money does not compile. A class of
  runtime guard becomes a compile/type property.
- Every arithmetic failure is a caller-handled `Result` (no panic in a request path, no silent wrap) that maps
  to the shared error enum.
- **Accepted cost:** the i64 domain width needs a fail-loud `i32::try_from` narrowing at the Phase-B write
  boundary (a value exceeding int4 also violates the DB range → correct to fail loud). Phase-B reads decode
  int4 → i32 → `TryFrom<i32> for Lek`, re-checking non-negativity at the decode boundary.
- **N/A by design:** no migration (schema untouched); no consistency/idempotency in the scalar (those live in
  the order-create txn — `idempotency_keys (location_id, key)`, ADR-0007 §4); no tenant dimension (tenant
  scoping is a row/query property, RLS FORCE + GUCs — a `location_id` in the value type would be a category
  error); no PII/secrets.
- **Open risks (owned):** raw-`i64`-vs-`Lek` lint **pulled forward** to land with the first Phase-B call-site,
  not a later ratchet (S5 lead, Counsel §3.1); single-currency assumption (product); no division/percentage
  method yet (S5 lead); a future edit could reintroduce transparent `Deserialize` (mitigated by a red→green
  negative-JSON test). **Round-1 additions:** O-7 f64-cliff at the JS JSON boundary — must-solve before any
  browser-touching cutover (S5 lead); O-8 amount+context redaction at the egress boundary (egress lead); O-9
  `as i32` write-narrowing footgun — defer-flag MISSING, `i32::try_from`-only + `as`-cast lint in the Phase-B
  write lane (S5 lead); **O-10 no home for directional/owed money — HUMAN DECISION flagged**, recommended as a
  named Phase-B deliverable (`SignedLek`/`Delta`/ledger entry) so a refund exceeding an order is a modelled
  outcome, never a silent floor-to-zero (product + S5 lead). Full register + human-decision note: design §10 / §10a.

## Proof (DoD — deferred to the code step, STOP-CODE-A)

Unit tests to ship red→green with the implementation: construction (0 / 1 / i64::MAX ok, −1 rejected;
`Lek::ZERO == new(0)`); add-overflow at i64::MAX → `Overflow { op: "add", .. }`; sub-goes-negative rejected;
mul overflow → `Overflow { op: "mul_qty", .. }` + negative-qty rejected; **`checked_mul_qty(i64::MIN)` →
`NegativeQuantity(i64::MIN)`** (M-2 landmine pinned); serde round-trip as bare integer **plus an exact-i64
round-trip above 2^53** (`Lek(9_000_000_000_000_000)`, Rust-to-Rust lossless; H-2); deserialize rejects float
(`100.5`, `100.0`), string (`"100"`), **and negative (`-1`)**; `MoneyError` Display renders the failing op
**and operands** (M-1). Phase B adds: `TryFrom<i32>` decode + `i32::try_from` write-narrowing tests; the
`as`-cast + raw-`i64`-money lints (O-9/O-1); and the string-encode-or-f64-bound mitigation at the first
JS-touching JSON boundary (O-7).
