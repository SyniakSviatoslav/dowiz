# Counsel Opinion — Sovereign Core money boundary (GRAND-PLAN 0b-1)

Status: ADVISORY (non-blocking). Reviewer: Counsel. Subject: `proposal.md` (1A+2A+3A+4A).
Grounding read: `pricing.rs` (apply_tax, distance_km, resolve_delivery_fee), `DECISIONS.md`
(D2/D5), migration `1780338982014_location_commerce.ts:18` (`max_distance_km numeric`, no scale
cap), Grep of `*.tsx` (no owner-UI author surface for tiers found — see Open Question).

---

## 1. Reasoning by lens (only what is load-bearing)

**Fairness / stakeholders (money precision).** The one place harm could enter is systematic
mis-charge of owner or client. It does not. Tax: core's `rate_micro = round(tax_rate·1e6)` is the
SAME `round_f64_to_i64` the code ships today (pricing.rs:56) — bit-identical, zero drift. Distance:
`distance_km` already quantizes to whole meters (`(d·1000).round()/1000`, pricing.rs:319) BEFORE any
compare, so the integer-meter boundary adds zero new rounding error. Byte-parity on every realistic
input is a real claim, not a hope. **No party silently loses money.**

**Care / harm — the R1 edge, examined for direction.** The sub-meter counter-example (§2): at a
half-meter-precision tier boundary, the integer round-half-up path returns TRUE (delivery covered →
a defined tier fee) where the float returned FALSE (not covered → `NOT_DELIVERABLE`). The divergence
is (a) confined to a 0.5 m band at a >3-dp tier max, (b) below Haversine/spherical-earth accuracy, so
physically unresolvable, and (c) directionally **inclusive** — it fulfils an order at a posted tier
fee rather than rejecting it. It is NOT a mechanism that overcharges either side; it selects a
config-authored fee. This is a fair, symmetric rounding rule (2A), and 2B was correctly rejected for
"fixing an unreachable edge by breaking the reachable centre" (whole-km boundaries are the common
case). **R1 is a justly acceptable risk, not a matter requiring a human money-ruling.**

**Long horizon / strategy.** Squarely serves D2 (deterministic, replayable, wasm-pure core): a float
can no longer *compile* into the core, so a native↔wasm replay divergence on the money path becomes a
build error, not a production surprise. Fail-closed at build, zero cascade. Reversibility is high —
forward-only, revert-by-commit, no migration, no lock-in. This is not polish; it is the step that
first exercises the F2 `disallowed-types` gate the invariant depends on. Aligned with the trigger
(the real paid-order money surface), not gold-plating.

**Aesthetic / conceptual integrity.** "Thin shell adapter = single float chokepoint" is a genuine
Anti-Corruption Layer: one module answers "where do floats meet money." The core losing the ability
to *name* `f64` is honest, restraint-shaped design ("схема багата, рантайм мінімальний" respected —
pure move, no new runtime). One honest crack in the "single chokepoint" story: `distance_km` also
lives in the shim and is imported by `shifts.rs`, so floats-touching-geo is not fully centralized —
the chokepoint is for floats-touching-*money*, which is the claim that matters. Integrity holds.

**Epistemic.** The proposal's own honesty is a quality signal: it names the counter-example against
itself (§2), keeps the f64 end-to-end vectors in the shim as an **independent, non-mirror oracle**
over the real boundary (D5-friendly), and offers 4B/3B/1B as explicit fallbacks. This is
adversarial-not-confirmatory authoring. No convergence-theater smell.

## 2. ETHICAL-STOP — NONE (grounded)

I find **zero** grounded red-line crossings. Checked against the register: server-authoritative money
(preserved — `Lek` i64 end-to-end, server computes); "схема багата, рантайм мінімальний" (respected);
no PII enters the core (§8, verified — menu/price/meters only, no customer identity); no surveillance,
no courier-dignity, no cash-path, no a11y surface touched. The money-precision red line is the only
one in reach, and byte-parity + identical `round_f64_to_i64` means it is not crossed. R1 is a
theoretical, physically-unreachable, direction-benign edge — it is NOT a grounded line, so it is an
owner accept-risk, not an ETHICAL-STOP. This is friction proportional to the truth: near zero.

## 3. Non-blocking advice (aesthetic / strategic)

- **R2 / Option 4A is the one place the "strictly mechanical move" boundary is crossed** — moving
  `PricingError.code` to `domain::ErrorCode` and deleting pg.rs `pricing_code`. I judge it *net-good*
  (it removes a D5 mirror-oracle, is net-negative code), so I'd keep 4A — but name it plainly in the
  ADR as a deliberate scope-widening on a red-line file, not fold it in silently. If any reviewer
  wants the minimal red-line diff, 4B is a clean, honest fallback. Either is defensible; just be
  explicit which one and why, so a future reader doesn't read it as passive scope creep.
- **R1's compensating control is currently a phantom.** See Open Question — I'd land the ≤3-dp
  constraint AS a DB CHECK now (it is one line, forward-only, and converts accept-risk to
  proven-safe) rather than assigning it to an owner who may not exist. Cheap insurance against a
  future footgun; still non-blocking for 0b-1 itself.

## 4. Steel-man of a rejected option — Option 1B ("dissolve the shim")

**Strongest case FOR 1B.** This repo's own ethos is ponytail — "the best code is the code never
written." 1B deletes a whole file and an extra hop. The Anti-Corruption Layer is being built for
*one* money caller (pg.rs) plus one unrelated `distance_km` importer (shifts.rs); YAGNI says do not
erect an adapter layer for a single consumer. Worse, the "one file where floats meet money" benefit
is already leaky — `distance_km` lives in the shim and is imported elsewhere, so the chokepoint is
partial regardless. Inlining the conversions in pg.rs, right after the `::float8` SQL read, is
arguably the *more honest* placement: it shows floats entering the system exactly where they enter
(the DB read), instead of hiding them behind an adapter that impersonates the old f64 signatures.

**Why it still loses (and it genuinely does).** The decisive reason is not "cleanliness" — it is D5.
Keeping the shim preserves TWO independent oracles: integer vectors in the core AND f64 end-to-end
vectors in the shim, checking the same behaviour by different paths (a non-mirror double-check). 1B
collapses them into one call path and forfeits that independent oracle on the money red line.
Secondary but real: pg.rs is a health-1.0/10 hotspot on the crown-jewel INSERT; 1B raises blast
radius there and re-homes the `shifts.rs` import as unrelated collateral. So the adapter is not
gratuitous abstraction — it is a *test boundary that buys an independent oracle*, which is precisely
what D5 asks for. 1B loses on verification integrity, not on taste. Verdict upheld.

## 5. Open question no one asked

**Who actually authors a delivery tier today, and who carries R1 when that changes?** Grep of the
`*.tsx` surface finds `max_distance_km` in *no* owner-facing editor — only server, seed, migration,
and tests. If tiers are engineer/seed-authored today, R1 is not merely "operationally unreachable" —
it is *dead*, and the proposed compensating control is ceremony. But the proposal defers that control
to a "tier-config surface owner … not a 0b-1 blocker" — an owner who, per the grep, **may not exist
yet**. That is the quiet failure mode: an accepted risk with its compensating control assigned to a
role that isn't staffed becomes accepted-and-forgotten, and the unbounded-precision `numeric` column
becomes a live footgun the *day* an owner tier editor ships — with no one owning the memory that a
CHECK was owed. The question for a human: do we (a) land the `≤3-dp` DB CHECK now, closing R1
permanently and cheaply while the surface is still engineer-controlled, or (b) formally attach R1 to
the future owner-tier-editor spec's Definition-of-Done so it cannot ship without the constraint?
Leaving it as an unowned flag is the one choice that lets it rot.
