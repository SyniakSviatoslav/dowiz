# Reflection — LC1 inclusive-tax double-charge + the mirror-oracle that certified it

- **Date:** 2026-07-03
- **Change:** money red-line hotfix (`orders.ts` total composition, `packages/ui/src/lib/money.ts`
  mirror + `chargedTax` field, `fee-parity.test.ts` oracle correction, new independent-constant
  vectors + property tests, `OrderSummarySection.tsx` M7 receipt, i18n key). Council: audit-fix-money
  RESOLVED v2, breaker re-attack PASSED.

## CONTEXT
The 2026-07-03 six-lane audit named LC1 the "single scariest live bug": with `price_includes_tax=true`
(the schema DEFAULT), `applyTax` correctly EXTRACTS the embedded VAT, then `orders.ts:511` ADDED it to
the total again → every taxed inclusive order overcharged by `r/(1+r)` (16.7% at 20% VAT). The client
mirror (`money.ts:84`) did the same, and `fee-parity.test.ts` was GREEN throughout.

## DECISIONS
1. Fix the composition, not the extraction: `chargedTax = price_includes_tax ? 0 : taxTotal`; keep
   `taxTotal` persisted/displayed as the informational VAT figure (ADR D1). 2. Add `chargedTax` to the
   FE estimate so the receipt renders inclusive VAT as "Incl. VAT" informational, never an addend (M7).
3. Prove with hand-derived literal vectors (zero-import file) + a definitional property test
   (`inclusive ⇒ total === subtotal + fee`), NOT another mirror.

## WHERE
`apps/api/src/routes/orders.ts:511`; `packages/ui/src/lib/money.ts` `estimateOrderTotal`;
`apps/api/tests/fee-parity.test.ts:67` (the oracle); `OrderSummarySection.tsx:53`.

## WHY (causal — two roots, distinct)
- **Why the bug EXISTED:** `applyTax` is overloaded — its return value means two different things
  (added-tax when exclusive, extracted-tax when inclusive), but the single composition line treated the
  return as unconditionally additive. A function whose output changes semantic role by a boolean flag,
  consumed by a caller that ignores the flag, is a latent double-count by construction.
- **Why the bug SURVIVED (the load-bearing lesson):** `fee-parity.test.ts` was a **mirror-oracle** — it
  computed its "expected" total as `sub + fee + serverApplyTax(...)`, i.e. from the *same* implementation
  formula under test, and asserted the client mirror equalled the server. A test that derives its
  expectation from the code it tests can only ever prove the two sides AGREE; it is structurally blind to
  a composition error consistent across both sides. It passed before and after the fix. The bug wasn't
  missed by weak coverage — it was *certified* by a test that measured self-consistency and called it
  correctness.

## CONFIDENCE
HIGH on both roots. Red→green verified mechanically: reintroducing `chargedTax = taxTotal` fails exactly
the 4 inclusive assertions + the property test. The mirror-oracle diagnosis is verified by inspection —
line 67 literally fed the implementation's output back as the expected value.

## NEXT-TIME
Any money/pricing test whose expected value comes from calling the module under test (or its mirror) is
not proof — it is a drift detector wearing a correctness label. Oracle independence = hand-derived
literal constants (with the arithmetic shown) OR a definitional invariant that names no implementation
output. The money council already encodes this as the M4 zero-import-vector-file ratchet + P2 property
test; this incident is the concrete instance that ratchet exists to stop from recurring.

## LINK
[[audit-remediation-orchestration-2026-07-03]] · ADR-audit-fix-money D1/M4/M5/M7 ·
docs/design/audit-fix-money/resolution.md · fee-parity mirror-lock also flagged as arch F14 +
test-integrity mirror-lock class in AUDIT-SYNTHESIS-2026-07-03.md.
