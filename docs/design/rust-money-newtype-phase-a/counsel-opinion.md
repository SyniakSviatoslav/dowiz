# Counsel Opinion — Rust `domain` money newtype `Lek(i64)` (Phase A scaffold)

- **Date:** 2026-07-04 · **Counsel:** DeliveryOS Radник (advisory — non-blocking)
- **Reviews:** `docs/design/rust-money-newtype-phase-a/proposal.md`, `docs/adr/ADR-rust-money-newtype.md`
- **Verdict at a glance:** No ETHICAL-STOP. Design is ethically and aesthetically sound — it actively
  *serves* the money-correctness and server-authoritative red lines. Two forward cautions and one open
  question worth carrying into Phase B. Friction here is deliberately light: the proposal is small,
  honest, and well-reasoned, so my footprint should be too.

---

## 1. Reasoning by lens (only what is substantive)

**Fairness / distribution.** A money type that makes wrong money *uncompilable* protects everyone,
but it protects the weaker parties most: the customer and the courier are the ones least able to
detect or contest a silent miscalculation (a wrong charge, a wrong cash reconciliation). Correctness
that can't be routed around is a fairness dividend that lands where it should. I affirm this.

One real fairness edge, though — the **directional asymmetry of an unsigned type**. `Lek` can express
"user owes / user pays" magnitudes natively, but "the platform owes *back*" — a refund exceeding an
order, store credit, a courier payout owed, a correction that dips below zero — is by construction
*not a `Lek`*. That is a defensible modelling choice (a magnitude is not a signed balance; the DB
enforces `CHECK >= 0` on 16 tables; refunds/settlements are explicitly Phase B). But it means the
system's *native expressiveness* is asymmetric in the platform's favour, and the path of least
resistance for a future dev who hits `checked_sub → Err(Negative)` on a legitimate refund is to floor
it to zero. If that happens, a real person silently loses money they were owed. Not a stop — a
forward flag, and the seed of my open question (§5).

**Honesty / consent.** Strong alignment with "server authoritative for price/status." No dark
pattern is even reachable in a scalar. The API *tells the truth about its own semantics* — refusing
`Add`/`Sub` because their presence would advertise infallible arithmetic is an honesty choice, not
just an ergonomic one. The distinct `NegativeQuantity` vs `Negative` error is honesty at the message
level (the UI, downstream, can say true things about what went wrong).

**Care / harm — which failure touches a person.** The residual harm surface is not construction or
overflow (those are correctly handled); it is the `checked_sub → Err(Negative)` path once a Phase-B
caller uses it. A discount or refund computation that legitimately dips below zero returns `Err`, and
a careless caller could abort a valid discounted checkout or drop a refund. This is a Phase-B
call-site concern, not a Phase-A type concern — but it is the honest answer to "which failure hurts a
real person," and it belongs in the record so the order/settlement surface handles it as a *business
outcome*, never as a floor-to-zero.

**Dignity / autonomy (courier).** N/A directly — no surveillance, coercion, or agency in a scalar.
Indirectly positive: cash-as-proof reconciliation depends on money correctness, so a type that cannot
wrap or clamp reduces the chance a courier is wrongly flagged as short on cash. A dignity dividend of
correctness.

**Privacy / PII.** Genuinely N/A. A scalar integer carries no customer data, no credential, no tenant
dimension. The proposal's refusal to put `location_id` inside the value type is the right call and the
right reason (category error, not added isolation). Nothing here touches anonymize-not-delete or
zero-PII-in-AI.

**Long horizon / strategy.** Reversibility is excellent (isolated worktree, deletable = full
rollback). The real strategic exposure is *lock-in of the call-site ergonomics*: the checked-Result
API with no operators propagates `?`-verbosity to every money site built in Phase B. That is the
correct trade for money — but it creates pressure to escape into raw `i64` (risk O-1). A safety type
that call sites quietly route around is worse than no type, because it manufactures a false sense of
safety (convergence theater). So the strategic counsel is: treat the clippy/dylint "no raw-i64 money
param" rule (O-1) as **earlier than a "later ratchet"** — ideally landing alongside the first Phase-B
money call-site, not after several. The longer the escape hatch stays open, the more optional the
safety becomes.

Second-order honesty with the operator: this artifact's entire value is *contingent on the Rust
rebuild program actually shipping*. A perfect money type in a rebuild that stalls is polish, and the
launch trigger (first real paid order) rides the current Node system, not this. That is a
program-level bet already made above this proposal — I only name it so the contingency is conscious.

**Aesthetics / conceptual integrity.** This is the strongest dimension and I want to affirm it
plainly. "Make illegal states unrepresentable," parse-don't-validate, errors-as-values, and "schema
rich, runtime minimal" are applied coherently, not as slogans. The catch of the
`#[serde(transparent)]` `Deserialize` hole — a negative wire value wrapping into a valid-looking
`Lek(-100)` without the constructor check — is a genuinely sharp find; it is the kind of thing that
separates a coherent design from a merely plausible one, and closing it with a validating
`Deserialize` keeps the invariant true *at the trust boundary*. The domain-width (i64) vs
storage-width (i32) split is clean and honestly documented as a supersession, not a silent override.
Elegance here is the real kind (fewer runtime failure modes), not the seductive kind.

**Epistemic.** The load-bearing unexamined assumption is **"single-currency, forever."** It is named
(O-3) but not *examined* — it is treated as a contingency to bolt on later, when it is actually the
premise that makes the whole `Lek(i64)` shape correct. If it breaks, it breaks call-site-wide, not
type-internally. I don't think it should change the decision (see steel-man), but it deserves to be a
*conscious* bet, not a deferred footnote.

## 2. ETHICAL-STOPs

**None.** I probed the one place a money-amount type could plausibly touch fairness/exploitation: the
**non-negativity invariant** and its effect on refunds / amounts-owed (§1 fairness + care). It does
not rise to a grounded red-line crossing, and I want the reasoning on record rather than a bare "none":

- The DB already enforces `CHECK >= 0` on all 16 money columns; a magnitude-only value type is
  faithful to the schema it models, not a new restriction.
- Refunds, credits, and payout balances are *explicitly* out of Phase A scope and live at the
  order/settlement/ledger layer (the current system already has a refund-due trigger + reconciler and
  settlements catch-up — so a home for directional money exists in principle).
- This is design-time only, no runtime, no PII, no coercion, no surveillance, no tenant surface.

So the honest concern is a **forward caution**, not a crossed line — recorded in §5, not as a stop.
An ETHICAL-STOP here would be friction on a smell, which the mandate forbids.

## 3. Non-blocking aesthetic / strategic advice

1. **(Strategic, higher priority) Pull the raw-`i64`-money lint forward (O-1).** An escape hatch that
   stays open makes the safety type optional in practice. Landing it with the first Phase-B money
   call-site preserves the whole point of the seam.
2. **(Strategic) Give directional/signed money an explicit, named home *before* the first refund path
   is coded in Rust** — not as part of this scalar, but as a conscious Phase-B design item, so
   "refund exceeds order" is never resolved by flooring to zero. (This is the open question, §5.)
3. **(Aesthetic, minor) `checked_mul_qty(qty: i64)` takes a raw `i64`.** A quantity is a different
   *kind* than money, and a bare `i64` qty gently undercuts the "no raw integers near money" ethos and
   admits e.g. an absurd-but-typechecking qty. YAGNI says a `Qty` newtype is out of scope for Phase A
   and I agree — but note it as a candidate so the eventual choice is deliberate, not accidental.
4. **(Aesthetic, affirm) The serde asymmetry is correct taste** — validate on the way in, emit bare on
   the way out. O-5's unit-test guardrail against a reintroduced transparent `Deserialize` is the
   right and sufficient catch; no need to gild it.

## 4. Steel-man of a rejected option

**A2 — operator overload + `debug_assert!` (rejected).** The proposal's rejection is *correct*, but it
wins partly by conflating two separable things, and the honest steel-man is worth recording so the
decision rests on the real tradeoff.

The fatal flaw the proposal names is real and specific: `debug_assert!` compiles *out* in release, so
the invariant is unenforced in production. That kills the `debug_assert` *enforcement mechanism*
outright. But it does **not** kill the *ergonomics argument*, which the proposal folds into the same
rejection. `?`-verbosity is not free: it accrues at every one of the dozens of money folds in Phase B
(a subtotal is a fold of `checked_add`), and ceremony-heavy code has its own safety cost — reviewers
miss bugs in noisy code. A stronger A2 proponent would not defend `debug_assert`; they would propose
**infix operators over already-validated `Lek` operands, with `overflow-checks = true` in the release
Cargo profile** — so `+`/`-` *panic on overflow in production* rather than wrap, and, because the i64
overflow boundary (~9.2e18) is unreachable by any real DeliveryOS money value, that panic branch is
effectively dead code while the 99.99% path gets clean infix ergonomics.

Why the Result API still likely wins (so this stays a steel-man, not a reversal): (a) for
`checked_sub`, going below zero is a *legitimate business error* you positively want as a handled
value, not a panic — infix `-` cannot express that cleanly; (b) a handled `Result` composing into the
single `IntoResponse` error enum beats a panic-inside-a-handler even for the "impossible" overflow;
(c) uniformity — one API for all four ops is more coherent than "infix for add, method for sub." But
the proposal slightly *overstates* its dominance by rejecting A2 wholesale rather than conceding that
A2's ergonomics critique survives and only its enforcement mechanism is fatal. Recording this keeps
the decision honest: Result-API is chosen over a *strong* alternative on genuine grounds, not over a
strawman.

*(Briefly, the multi-currency steel-man for R2/Decimal feeds §5 rather than repeating here: adopting a
currency-carrying representation now would make an eventual multi-currency move a type-internal change
instead of a call-site-wide migration — countered, correctly, by YAGNI and by the fact that a currency
tag on an always-Lek value is the same category error as `location_id` in the type.)*

## 5. The open question nobody asked

**Where does *directional* money live in the Rust rebuild, and who owns ensuring it exists before the
first refund/payout path is built?**

The proposal asks, carefully, how a `Lek` behaves when arithmetic *would* go negative (answer:
`Err`). It does **not** ask where the *legitimate* negative / owed quantity lives — the refund that
exceeds an order, the store credit, the courier balance the platform owes. Because `Lek` is unsigned
by invariant, the system's native vocabulary can say "user pays X" fluently but must reach for some
*other, not-yet-designed* construct to say "platform owes user X." If that construct is missing or
more awkward than flooring to zero when the first Rust refund path is coded, the asymmetry resolves
against the customer and the courier — the two parties least able to notice.

This is not a Phase-A change and not a criticism of `Lek(i64)` — the scalar should stay a magnitude.
It is a request that the Phase-B design *consciously name the home for signed/owed money* (a
`SignedLek` / `Delta`, a ledger entry type, or an explicit direction+magnitude), and that the S5
orders/money council own it, so that "refund exceeds order" is always a modelled business outcome and
never a silent floor. The human decision I'd ask for: **is that home a Phase-B deliverable with a
named owner now, or a latent risk we're accepting until it bites?**
