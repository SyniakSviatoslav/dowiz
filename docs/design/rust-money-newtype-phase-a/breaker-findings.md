# Breaker findings — Rust `domain` money newtype `Lek(i64)` (Phase A scaffold)

- **Target:** `docs/design/rust-money-newtype-phase-a/proposal.md` + `docs/adr/ADR-rust-money-newtype.md`
- **Breaker:** System Breaker DeliveryOS · **Date:** 2026-07-04 · **Round:** 1
- **Mode:** design-time attack (no code exists yet). Severities reflect "if implemented and later wired as-specified,"
  since Phase A itself is an isolated, no-IO, no-call-site worktree scaffold (blast radius *now* = zero).
- **Read-only grounding used:** live money columns (`packages/db/migrations/1790000000083_payments-ledger.ts`),
  the money scale config (`apps/api/src/lib/money.ts`, `apps/api/src/lib/preview-render.ts`,
  `packages/ui/src/lib/money.ts`, `packages/ui/src/lib/currency.ts`), and the (absent) frame docs.
- **Not restated as findings** (already self-identified by the proposal): the `#[serde(transparent)]`
  constructor-bypass on deserialize, and the i64-vs-i32 width conflict with inventory/12 §9. Findings below go past these.

Ranked: **HIGH ×2 · MEDIUM ×5 · LOW ×3.** No CRITICAL — nothing in Phase A reaches production and both HIGHs have a
realistic-magnitude escape hatch; flagging either CRITICAL would be severity inflation. No fixes proposed (architect fixes).

---

## HIGH

### H-1 · B-SCALE · The ADR's load-bearing sizing number is wrong by 100× — a single location-year *fits* int4
- **Finding.** §2's decisive input — *"a single location's annual order totals ≈ **1.46e10** minor units already
  exceeds int4's 2.147e9"* (proposal §2 row 3; ADR "Width i64, not i32") — assumes money is stored at scale 2
  (Lek ×100 minor units). The live system stores Albanian Lek at **scale 0** (whole Lek, no subunit): the currency
  minor-unit defaults to `0` (`apps/api/src/lib/preview-render.ts:47` `menu.currency?.minor_unit ?? 0`;
  `csv-parser.ts` `currencyMinorUnit = 0`), and the server tax helper's `_minorUnit` param is **dead code** —
  `packages/ui/src/lib/money.ts` states verbatim *"The server's `_minorUnit` param is dead code there, so it is
  intentionally absent here."* The proposal's own §1/non-goals scope the type to **single-currency Albanian Lek**,
  whose qindarka subunit is defunct → scale 0.
- **Recomputed number.** 400k Lek/day × 365 = 146,000,000 Lek = **1.46e8 minor units** (minor == Lek at scale 0),
  not 1.46e10. int4 max = 2.147e9 ⇒ **fits int4 with ~14.7× headroom.** The true int4 crossover for aggregation
  needs ~**14 location-years** or a cross-location platform rollup — not "a single location-year."
- **Break.** The ADR's headline justification for choosing i64 over i32 is false under its own single-currency-Lek
  premise. The two premises are mutually inconsistent: the §2 table computes "minor units" as Lek×100 while §1
  scopes to Lek (scale 0). A reviewer approving *"i64 because a single location-year already blows int4"* is
  approving on a 100×-wrong number. (The i64 choice may still survive on platform-rollup + headroom grounds —
  but the *stated reasoning* does not hold.)
- **Invariant violated.** Back-of-envelope must add up on a 🔴 money-red-line ADR ("ignores back-of-envelope" =
  reject). Load-bearing number is off by two orders of magnitude.

### H-2 · B-CONSIST / B-DATA · `Serialize` as a bare JSON integer silently corrupts values in (2^53, i64::MAX] — the exact i64 headroom band the width was chosen to protect
- **Finding.** The type is i64 *specifically* to hold large aggregations/intermediates up to the ~9.2e18 ceiling,
  and the design argues a `checked_*` `Err` there "reliably signals a genuine bug or attack" (§2 conclusion).
  But `Serialize` emits a **bare JSON number** (proposal §3 wire contract; ADR "Serialize emits a bare JSON
  integer"). JavaScript `JSON.parse` — i.e. **the entire existing Node/TS layer** (which "carries money as a bare
  `number`", proposal §1) **and every browser client** — decodes JSON numbers to IEEE-754 `f64`, which loses
  integer precision above **2^53 = 9,007,199,254,740,992 ≈ 9.007e15**.
- **Break (concrete).** A valid, non-overflowing `Lek` in (9.007e15, 9.2e18] is *constructible without any
  `Overflow` Err* — e.g. a crafted catering line `price 1000 × qty 1e13 = 1e16` passes `checked_mul_qty` (1e16 <
  i64::MAX), yields a legitimate `Lek(1e16)`, then **silently rounds** when serialized to any JS consumer (1e16
  is not f64-exact). That is *silent wrong money* — the precise class the type claims to make impossible —
  reintroduced at the wire boundary, and it is **not** caught by the overflow guard (the value is below i64::MAX).
  So the ~1024× band between the domain ceiling (9.2e18) and the JS-safe ceiling (9.007e15) is a silent-corruption
  blind spot in exactly the range i64 was picked to preserve.
- **Compounding.** The standard JS mitigation for >2^53 integers — send them as **strings** (BigInt→string) — is
  **rejected** by the hand-written `Deserialize` (it deserializes `i64`; a JSON string `"..."` errors). So the type
  can neither safely *emit* large values to JS nor *accept* the string-encoded workaround.
- **Realistic-magnitude caveat (why HIGH not CRITICAL).** Real Albanian-Lek rollups top out ~1e11–1e13 (< 2^53),
  so the demonstrable corruption needs an adversarial/bug value in (9e15, 9.2e18] — but the design *itself* invokes
  "attack" values as the reason for i64, so that band is in-scope by the design's own argument.
- **Invariant violated.** "No silent wrong money" / server-authoritative correctness — the seam's raison d'être.

---

## MEDIUM

### M-1 · B-OPS · `MoneyError::Overflow` carries zero diagnostic payload, contradicting the <1-min observability claim
- **Finding.** §9 asserts `MoneyError` "names the failing op/value ... loggable/alertable without extra plumbing."
  But the enum is `Negative(i64)`, `NegativeQuantity(i64)`, **`Overflow`** — the `Overflow` variant carries
  *neither* the operands *nor* which operation (add / sub / mul_qty). `Overflow` is precisely the variant that
  signals "genuine bug or attack" (the entire stated reason for i64 in §2), yet it is the **least diagnosable**:
  a Phase-B log line reads `MoneyError: Overflow` with no amount and no op.
- **Break.** On the one error the design most wants to alert on, the "<1 min visibility, no extra plumbing" claim
  is false — an operator sees `Overflow` and cannot tell an add from a catering-mul, nor the magnitudes involved,
  forcing exactly the "extra plumbing" (stack traces, call-site logging) §9 says is unneeded.
- **Invariant violated.** B-OPS: failure must be diagnosable <1 min; observability claim not met for the key variant.

### M-2 · B-FAIL · `checked_mul_qty(i64::MIN)` is an implementation landmine the DoD does not pin
- **Finding.** DoD item 4 tests "negative qty → `Err(NegativeQuantity)`" with an unspecified value (a suite will
  naturally use `-5`). The design does not specify *how* negative qty is detected. A natural implementation that
  normalizes or messages via `qty.abs()` / `qty.unsigned_abs()`-style logic hits the classic trap:
  **`i64::MIN.abs()` overflows** → **panics** with overflow-checks on (an availability incident inside a request
  handler — the exact A2-rejected failure mode) or **wraps to `i64::MIN`** with checks off (silent wrong sign).
- **Break.** A `qty < 0` early-return avoids it, but the spec leaves detection unspecified and the test list does
  **not** force `qty = i64::MIN`, so a green test suite (passing the `-5` case) can ship a handler-panic on
  `checked_mul_qty(i64::MIN)`. The type designed to forbid money panics can panic at its one multiply.
- **Invariant violated.** B-FAIL / "never panic in a request path" — the A1-over-A2 rationale (§3) defeated by an
  unpinned edge in the design's own DoD.

### M-3 · B-CONSIST / B-SEC · "The invariant holds even from the wire" conflates type-validity with server-authority
- **Finding.** ADR why-point 4 and §8 present the validating `Deserialize` as *the* security refinement — "no
  `Lek` in the process ... is ever negative", "prevents a hostile/negative wire value from entering." True, but it
  guarantees **only non-negativity (sign)**, never **server-authority (amount)**. Nothing in the type stops a
  Phase-B handler from `Deserialize`-ing a client-supplied `total: Lek` and trusting it *because "it's a validated
  Lek."*
- **Break.** A hostile client posts `{"total": 1}`; it deserializes into a perfectly valid `Lek(1)`. The
  reassuring "invariant holds from the wire" framing invites a call site to treat a wire `Lek` as trustworthy
  money, when it is only sign-checked. The live system already guards this at the order txn + a cash-422 backstop
  (`packages/ui/src/lib/money.ts` "the SERVER stays the single source of truth for what is CHARGED"); the ADR
  language risks eroding that discipline at the new type boundary.
- **Invariant violated.** 🔴 "server authoritative for price/status" — client total must never be trusted; a
  validated `Lek` is not an authoritative `Lek`.

### M-4 · B-DATA · Phase-B write-narrowing footgun: `as i32` truncation → negative money in a `CHECK (>= 0)` int4 column
- **Finding.** §5 correctly specifies the write path as `i32::try_from(value)` (a `Result`). But nothing gates
  the always-available `value as i32` cast, which **silently wraps**: `2_147_483_648_i64 as i32 == -2_147_483_648`.
  The clippy/dylint rule against raw-money mistakes is explicitly **deferred** (O-1), so Phase A ships no guard
  against this. The live money columns are `integer NOT NULL CHECK (amount_minor >= 0)`
  (`1790000000083_payments-ledger.ts:32-34`) with a residual invariant `refunded <= captured <= amount`.
- **Break.** A single careless `as i32` at the very write boundary the design defers turns a >int4 `Lek` into a
  **negative** i32 → either a Postgres `23514` check violation (best case, loud) or, on any path lacking the
  CHECK, a **negative money value persisted** — the exact "negative money" the entire newtype exists to make
  unrepresentable, reached at the one boundary Phase A does not build.
- **Invariant violated.** 🔴 non-negative money; the fail-loud-narrowing promise (§5) depends on hand-discipline
  with no guardrail in place.

### M-5 · B-ANTIPATTERN · No clamp-to-zero primitive: `checked_sub` is error-only, so a legitimate over-discount checkout must be hand-guarded
- **Finding.** Money math needs "floor the total at 0" for a promo/voucher exceeding subtotal (the schema has a
  distinct `discount_total` and derived `total`, `apps/api/src/lib/order-persistence.ts:26-27`). The design
  rejected A3 saturating (§3) *and* makes `checked_sub` return `Err(Negative)` on a below-zero result — leaving
  **no total-function** for clamp-to-zero. Callers must hand-roll `(...).checked_sub(discount).unwrap_or(Lek::ZERO)`.
- **Break.** subtotal 500, discount 600 → `subtotal.checked_sub(discount)?` → `Err(Negative(-100))` → a careless
  `?` fails a *legitimate free-order* checkout. (Parity caveat: today's `assertNonNegative` in
  `apps/api/src/lib/money.ts:32` also throws on negative total, so this is **not a regression** — but the newtype
  had the chance to model the clamp-vs-error choice explicitly and instead makes clamp *inexpressible*, pushing a
  business rule back into per-call-site error handling — the scattered-guard pattern the type claims to eliminate.)
- **Invariant violated.** B-ANTIPATTERN: the value type under-models a real domain operation; correctness now
  depends on every call site pre-clamping (re-scattering the guard).

---

## LOW

### L-1 · B-OPS / B-SEC · §8's blanket "no PII / nothing sensitive" + derived `Debug` + value-carrying `MoneyError` = money amounts in logs with no redaction seam
- **Finding.** §8 declares `Lek` categorically "No PII, no secrets ... nothing sensitive" and uses that to waive a
  redaction posture. `Debug` is derived and `MoneyError::Negative(i64)`/`NegativeQuantity(i64)` carry the raw
  amount, which §9 says is "loggable/alertable." This project runs a claim-check / PII-egress posture (menu-only
  into AI prompts and queues, per project memory). Declaring the money type "not sensitive" removes any
  **type-level signal to redact** amounts when a `Lek` flows into those known egress choke points (AI, queues,
  analytics exports).
- **Break.** A bare amount alone is weak PII (hence LOW), but the over-broad "not sensitive" waiver could license
  downstream un-redacted egress of order totals joined to customer context — financial data the project's GDPR
  path otherwise protects.
- **Invariant violated.** Claim-check / no-untracked-PII-egress posture; the §8 claim is stronger than warranted.

### L-2 · B-ANTIPATTERN · `Ord` / `PartialOrd` / `Hash` derived with no stated consumer (mild over-derive / YAGNI)
- **Finding.** The derive set includes `PartialOrd, Ord, Hash` (ADR "Derives"). For a single-field integer newtype
  these are **correct** (lexicographic-on-one-field == i64 order; Hash consistent with Eq) — **no correctness bug**.
  But no Phase-A need is stated for ordering money or using it as a `HashMap`/`BTreeSet` key.
- **Break.** None functional — flagged only against the proposal's own YAGNI/"runtime minimal" stance: shipping an
  ordering + hashing surface with no caller invites later misuse (money as map key) for zero present benefit.
- **Invariant violated.** B-ANTIPATTERN (mild): API surface beyond need. Cheap and defensible; hence LOW.

### L-3 · Doc-integrity · The canon frame docs the ADR claims to *reconcile* are absent from this worktree — the central "resolves the conflict" claim is unverifiable
- **Finding.** The ADR's headline is *"Reconciles REBUILD-MAP.md §1 (`Lek(i64)`) vs inventory/12 §9
  (`Minor(i32)`)"* and both proposal & ADR cite `docs/design/rebuild-plan/06-complete-rebuild-stack.md`,
  `.../REBUILD-MAP.md`, `.../inventory/12-data-layer.md` as frame. **None of these files exist** in the worktree
  (`docs/design/rebuild-plan/` is absent; a repo-wide grep for `Minor(i32)`, `inventory/12`, `REBUILD-MAP` returns
  zero hits outside this proposal). The "16 tables, all int4 + `CHECK ≥ 0`" claim is only *partially* verifiable —
  the pattern is confirmed on the money SoT table (`payments`, `1790000000083`), but the count "16" and
  "no bigint money anywhere" cannot be checked here.
- **Break.** A reviewer cannot verify the reconciliation (O-6), the "16 tables" sizing base, or the `§9 Minor(i32)`
  text being superseded — the decision's entire evidentiary base is out of reach of the artifact set. May be a
  worktree-isolation artifact, but as delivered the ADR asserts a reconciliation against documents that aren't present.
- **Invariant violated.** Verifiability / DoD: a 🔴 decision's cited grounding must be checkable.

---

## Vectors swept (per breaker matrix)
- **B-SCALE:** H-1 (100× sizing error). Connection budget correctly N/A (no pool). Throughput correctly a non-driver.
- **B-FAIL:** M-2 (i64::MIN mul panic landmine). Overflow/negative paths are `Result` (sound in principle).
- **B-CONSIST:** H-2 (wire silent-corruption), M-3 (wire-validity ≠ authority). Idempotency correctly deferred to order txn.
- **B-SEC:** M-3, L-1. No cross-tenant/secret surface in a scalar (correctly N/A).
- **B-DATA:** H-2, M-4. Integer-not-float representation is **correct** (confirmed against live int4+CHECK schema) — no finding, a strength.
- **B-OPS:** M-1 (Overflow no payload), L-1. Rollback trivial (correct). Flag-by-absence (correct).
- **B-ANTIPATTERN:** M-5 (no clamp primitive), L-2 (over-derive). Otherwise disciplined YAGNI (no division/multi-currency).
- **Doc-integrity:** L-3 (frame docs absent).

---

## RE-ATTACK — Round 2 (regression pass over `resolution.md`)

- **Date:** 2026-07-04 · **Round:** 2 (regression-only). Scope: (a) did any of the 10 dispositioned fixes
  introduce a NEW hole; (b) any remaining unresolved CRITICAL/HIGH. Accept-risk / defer-flag items with
  recorded owners (H-2→O-7, M-2, M-4→O-9, M-5, L-1→O-8, L-3, O-1, O-10) are NOT re-litigated — only checked
  for a broken fix.

### New hole introduced by a fix

**[MEDIUM] B-DATA/verifiability · The H-2 interim guardrail constant is BELOW 2^53 — the ">2^53 round-trip"
test does not exercise the band it names.**
- **Finding.** The H-2 fix adds DoD test #7 "serde exact-i64 round-trip **above 2^53**: `Lek(9_000_000_000_000_000)`"
  (proposal §3 line 152, DoD #7 lines 349–351, O-7 line 305; resolution delta #6 line 76 — four cross-refs, all
  asserting the value is `> 2^53`). It is not. `2^53 = 9_007_199_254_740_992`; the chosen constant
  `9_000_000_000_000_000` is **below 2^53 by 7,199,254,740,992** (verified). Every integer below 2^53 is
  f64-exact, so the constant sits *outside* the (2^53, i64::MAX] headroom band that the entire H-2 discussion —
  and this very test — exists to cover.
- **Break.** The guardrail added specifically to pin the i64-headroom losslessness claim is mislabeled and
  ineffective at its stated job: it proves a sub-2^53 value round-trips (trivially true, and true even in JS
  f64), not a headroom-band value. Because the path is Rust-only `to_string`→`from_str` (serde_json i64 parse is
  lossless for *all* i64) the test passes regardless, so there is **no production-money hole** — but it delivers
  false assurance about exactly the band H-2 flagged. This repeats the H-1 failure mode (a wrong load-bearing
  number on a 🔴 money artifact); to actually exercise the band the constant must be ≥ `9_007_199_254_740_993`.
- **Invariant violated.** Guardrail must test what it claims / back-of-envelope must add up on a 🔴 money seam.
  Not CRITICAL/HIGH: un-wired Phase-A scaffold, zero blast radius, Rust-only path passes; the real JS f64 risk
  remains correctly deferred to O-7.

### Minor note (not a hole — no decision impact)

- **[LOW/nit] B-SCALE · H-1 reframe "~14 location-years overflow int4" is really ~15.** `14 × 1.46e8 = 2.044e9
  < int4 max 2.147e9` (does not overflow); `15 × 1.46e8 = 2.19e9` overflows. Tilde-hedged, order-of-magnitude
  correct, and the i64 decision does not rest on the exact crossover — the corrected §2 table's other rows
  (cross-location/platform rollup 1e11–1e13; adversarial `1e3 × 3e6 = 3e9`; scale-2 future 1.46e10) are all
  arithmetically sound and independently exceed int4. No change required.

### Fixes that hold (no new hole)

- **H-1** reframe — §2 numbers recomputed and sound (loc-year 1.46e8 fits int4 ~14.7×; adversarial mul 3e9
  exceeds int4/fits i64; scale-2 1.46e10). Decision unchanged, reasoning now honest. HOLDS.
- **M-1** `Overflow { op, lhs, rhs }` + `Display` — diagnosable variant achieved; amounts-in-error is the
  already-accepted O-8 posture, not a new leak. HOLDS.
- **M-2** `qty < 0` early-return before any multiply, no `.abs()/.unsigned_abs()`, `i64::MIN` DoD pin — landmine
  closed. HOLDS.
- **M-3** doc: sign-not-authority — over-claim removed; server-authority stays at the order txn. HOLDS.
- **M-5** `pub const ZERO = Lek(0)` — inner value 0 is invariant-valid, explicit greppable floor; no saturating
  method added. HOLDS.
- **L-1** doc narrowing + O-8 defer; **L-2** `Hash` dropped, `Ord/Eq` kept — derived `Serialize` on a single-field
  tuple struct still emits a bare integer (serde `newtype_struct`), so removing `#[serde(transparent)]` does not
  change the wire form; **L-3** worktree-sync accept. All HOLD.

### Verdict

**RE-ATTACK: no new CRITICAL/HIGH; all decision-level fixes hold — with ONE new MEDIUM defect in the H-2
interim guardrail** (its ">2^53" test constant `9_000_000_000_000_000` is below 2^53, so it does not exercise
the headroom band it names; Rust-only path passes regardless, no production hole). No remaining unresolved
CRITICAL/HIGH: H-1 corrected, H-2 residual JS-f64 risk correctly deferred to O-7 with owner. No CRITICAL exists
(un-wired scaffold, zero blast radius now).
