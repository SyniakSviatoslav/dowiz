# Breaker findings — sovereign-core-money-boundary-0b1

Role: System Breaker. Read-only against real code. No fixes proposed. Attack vectors:
money / data-integrity / deploy-mismatch / adversarial-input / anti-pattern.

Sources verified against HEAD (not just the proposal):
`rebuild/crates/api/src/routes/orders/pricing.rs`, `.../orders/pg.rs`, `.../orders/mod.rs`
(`round_f64_to_i64`), `.../courier/shifts.rs`, `rebuild/crates/domain/src/error.rs`,
`.../domain/src/lib.rs`, `.../domain/src/kernel.rs`, `.../domain/clippy.toml`.

---

## VERIFIED (proposal claims that hold — no finding)

- **Q2/#2 — all 9 error codes exist with the exact wire strings.** `domain::error::ErrorCode`
  contains `ProductNotFound, ProductUnavailable, ModifierUnavailable, ModifierMinNotMet,
  ModifierMaxExceeded, DuplicateModifier, NotDeliverable, DeliveryNotConfigured, MinOrderNotMet`,
  all under `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` (`error.rs:49-50,134-153`). The literal
  strings in `pricing.rs` (`"PRODUCT_NOT_FOUND"`, …) map 1:1. Confirmed.
- **#3 — exactly two `pricing_code` call sites.** `pg.rs:292` and `pg.rs:326`; definition `pg.rs:987`;
  test `pg.rs:1077`. Grep across the whole tree finds `pricing_code` only in `pg.rs`. Confirmed.
- **#4 — no third caller.** Whole-repo grep: `pricing::*` / `orders::pricing` appears only in
  `pg.rs` (money fns) and `shifts.rs` (`distance_km`), plus docs. No tests/ or other crate consumes
  it. Confirmed.
- **#6 — no top-level name collision & kernel is a dir.** `domain/lib.rs` exports none of
  `PricingError/DeliveryTier/FeeLocation/PricingItem/…` today; `kernel/` is already a directory
  (`kernel.rs` + `kernel/{idempotency,policy}.rs`), so `git mv → kernel/pricing.rs` + `pub mod
  pricing;` is structurally sound. Confirmed.

---

## HIGH

### [HIGH] adversarial-input / money-parity — the `apply_tax` f64 short-circuit guard is silently dropped by the shown boundary, and NO vector covers it

The current `apply_tax` (`pricing.rs:50-51`) opens with a **float-domain guard**:
`if subtotal == 0 || tax_rate <= 0.0 || !tax_rate.is_finite() { return Ok(0); }`.
This guard cannot exist in the integer core (it names `f64`). The proposal’s concrete artifacts
drop it: the core signature is `apply_tax(subtotal: i64, rate_micro: i64, incl)` (no guard shown),
and the illustrated adapter body is the bare one-liner
`domain::kernel::pricing::apply_tax(subtotal, round_f64_to_i64(tax_rate*1e6), incl)` (proposal §4).

Break scenario (traced through the real arithmetic + real `round_f64_to_i64` at `mod.rs:34-49`):
- **Negative rate.** A location row with `tax_rate = -0.2` (nullable `numeric`, read
  `tax_rate.unwrap_or(0.0)` at `pg.rs:335`; the guard exists precisely because the column is not
  trusted). OLD: guard → `Ok(0)`, order succeeds with 0 tax. NEW (literal adapter): `rate_micro =
  -200000`; core exclusive branch `1000·(-200000)+500000 = -199_500_000 /1_000_000 = -199` →
  `Lek::new(-199)` → `Err` → `pg.rs:339-340` returns `RepoError("tax negative")` → **5xx**. A
  create that used to succeed now hard-errors.
- **±Infinity rate.** Postgres the numeric type admits Infinity; cast to f64 it becomes `f64::INFINITY`.
  OLD: `!is_finite` → `Ok(0)`. NEW: `round_f64_to_i64(INF·1e6)=i64::MAX` → `checked_mul` overflow →
  `MoneyError::Overflow` → **5xx**.
- (NaN happens to survive only by luck: `round_f64_to_i64(NaN)=0` → `rate_micro=0`; do not rely on it.)

Invariant violated: **byte-parity on the money red-line** ("parity is the whole point", proposal §1).
Worse, the money-parity gate is BLIND to it: every `apply_tax` vector uses rates
`{0.0, 0.075, 0.0744, 0.0745, 0.1, 0.2, 0.0825, 1.0}` — none negative, non-finite, or the
guarded zero-via-`<=0` path (`pricing.rs:423-453`, `559-603`). The proposal even says the guard’s
re-homing is nowhere specified. So the single behavior the f64→i64 integerization is most likely to
lose is exactly the one no test can catch.

---

## MEDIUM

### [MED] anti-pattern / deploy-mismatch — `disallowed-types` does NOT catch inferred float literals, so the "no float compiles into core / fail-closed at build" claim and its RED→GREEN proof are narrower than stated

Gate 2 (`clippy -p dowiz-core --lib`, `disallowed-types = [f64,f32]`, `clippy.toml:56-59`) fires on
**type positions** (signatures, fields, generics, casts, aliases). It does **not** lint an
expression whose float type is only inferred. `let x = 1.5 * 2.0;` / `let r = 0.1 + 0.2;` are `f64`
with no `f64` token — clippy stays silent, and the wasm32 gate also accepts `f64` (float is valid
wasm). The proposal’s RED proof exercises only the named-signature path
(`pub fn leak(x: f64) -> f64`, §9), which the lint does catch — so it demonstrates the easy case and
generalizes to "any float is a hard build failure," which is false for inferred-literal float
arithmetic.

Break scenario: a future edit adds a float intermediate inside `kernel::pricing` (e.g. a "quick"
proportional-fee tweak using `let f = subtotal_as_ratio * 0.1;`) — non-deterministic native↔wasm by
the crate’s own charter — and **Gate 2 passes green**. Scoped honestly: crossing that float back
into the i64 money path still needs a named cast (`as i64`) that the workspace `as_conversions` deny
should trip, so the money boundary is doubly-fenced; but the design’s headline safety property
("cannot name f64 ⇒ cannot leak a float") is over-claimed and the verification does not test the hole.
Invariant: deterministic-replayable core (DECISIONS D2 / clippy.toml rationale).

### [MED] data-integrity / anti-pattern — `compute_order_pricing` introduces the FIRST entropy-seeded collections into the "entropy-free" core, unguarded by any gate

Grep of `domain/src/**` finds **zero** `HashMap`/`HashSet`/`std::collections` today (kernel,
policy, idempotency are all free of it). Moving `pricing.rs` brings `compute_order_pricing`, which
constructs collections *inside* the core: `let mut group_counts: HashMap<&str,i64> =
HashMap::new();` and `let mut seen = std::collections::HashSet::new();` (`pricing.rs:233,239`).
`HashMap::new()` seeds `RandomState` from OS entropy — an ambient-entropy read that the core’s
"Law 2: No entropy" forbids and that `clippy.toml disallowed-methods` (`SystemTime`/`Instant`/
`new_v4`/`env::var`) does **not** list. The wasm32 gate is a *build*, not a run, so it cannot catch a
runtime seed source either.

Today the OUTPUT is seed-independent (only `.get()`/`.entry()` by key; failure selection walks the
ordered `items`/`group_rows` slices), so replay stays byte-identical — but that is an **unenforced,
incidental** property. Break scenario: a later edit that iterates `group_counts`/a snapshot map (e.g.
to emit "all unmet groups") yields native↔wasm-divergent ordering, and no gate goes red — precisely
the silent replay divergence this whole sovereign-core exercise exists to prevent. The proposal
dismisses this as "HashMap — це std, ОК" without noting it is the crate’s first `RandomState` and
adds an unguarded exception to the entropy law.

---

## LOW

### [LOW] money — R1 (sub-meter tier divergence) ships un-gated
The proposal discloses R1 honestly and the ≤3-dp analysis is correct (verified: distance is
`round(d·1000)/1000` at `pricing.rs:319`, so the meter operand carries zero new error; the only
divergence is a >3-dp `max_distance_km` with a delivery within 0.5 m — schema-allowed, `numeric`,
no scale cap). But the compensating control (≤3-dp validation / DB CHECK) is explicitly "not
blocking this step," so the change lands with the divergent config still schema-reachable and **no
guardrail accompanying the money red-line edit**. Accepted-risk, but the guardrail-with-fix rule is
deferred to an unowned future.

### [LOW] anti-pattern — two `DeliveryTier`/`FeeLocation` types with identical names (i64 core vs f64 shim) coexist
By design the shim keeps f64 `DeliveryTier{max_distance_km}`/`FeeLocation{lat,lng,…}` while the core
gets i64 `DeliveryTier{max_distance_m}`/`FeeLocation{…}` (lat/lng removed), and `lib.rs` re-exports
the core ones. `pg.rs:308` builds the f64 shim shape from the `max_distance_km` column cast to f64.
Footgun: two same-named money-adjacent structs in the same workspace; a glob import or a future
refactor that grabs `domain::DeliveryTier` where the f64 shim shape was meant compiles into a
1000×-scale error (km vs m). No current break (pg.rs uses the explicit `super::pricing::` path).

### [LOW] anti-pattern — "single float chokepoint / single meter convention" is narrower than framed
`shifts.rs:896` computes `distance_km(lat,lng,loc_lat,loc_lng) * 1000.0` (raw `*1000`, NOT
`round_f64_to_i64`) for the geofence ping, an independent second km→m conversion using a *different*
rounding convention. It stays in the shell and is untouched, so no regression — but the design’s
"one file is where floats meet money / one meter convention" claim (§3) is true only for the money
path; distance-in-meters already has a second, unrounded convention elsewhere.

---

## Severity roll-up
- CRITICAL: none (no path produces a wrong *charge* on normal input; the HIGH is Ok(0)→5xx on
  exotic config — a parity/availability break, not a mischarge — so no inflation to CRITICAL).
- HIGH: 1 (apply_tax guard dropped + gate-blind).
- MED: 2 (disallowed-types inferred-float hole; first unguarded entropy source in core).
- LOW: 3 (R1 un-gated; dual same-name structs; meter-convention framing).
