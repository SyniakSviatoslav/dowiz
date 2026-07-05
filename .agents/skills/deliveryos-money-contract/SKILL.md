---
name: deliveryos-money-contract
description: >-
  DeliveryOS money & rounding contract. ALWAYS load BEFORE adding or editing any code that touches
  prices, totals, currency, rounding, cart/checkout amounts, delivery-fee, tax, discounts, or any
  monetary value — even if money isn't the explicit ask. Handles the ALL-minor-units integer rule,
  centralized half-up rounding, EUR-as-display-only never-in-math invariant, and PriceDisplay / formatMoney
  / formatALL / fmtPrice formatting rule. Ships a deterministic checker (`scripts/check-money.mjs`) —
  run it on every money-related diff before declaring done. Violations: integer money fields used
  as float, EUR leaking into order math, rounding outside the central utility, ad-hoc formatting.
---

# DeliveryOS Money & Rounding Contract

## Core Invariants (🔴 never violated)

### 1. Integer ALL minor units
- **All** monetary values in `orders`, `order_items`, `products`, `delivery_tiers`, `settlements`, `courier_payouts` are stored as **integer ALL minor units** (ALL = 0 decimal currency).
- **Never** store or compute money as `float`/`decimal`/`number` with fractional ALL.
- The only exception: EUR display conversion results are ephemeral (never persisted, never in math).

### 2. Centralized half-up rounding
- Rounding uses JavaScript's `Math.round()` (half-up by IEEE 754 default).
- All rounding goes through `formatMoney()` in `packages/shared-types/src/utils.ts`.
- **No** ad-hoc `Math.round()`, `toFixed()`, or manual rounding on monetary values outside this utility.
- `formatALL()` is a convenience wrapper for `formatMoney(amount, 'ALL')`.

### 3. EUR is display-only — never in order math
- EUR conversion `amount * rate` is computed only for display in `formatMoney()`.
- The result **never** flows into `subtotal`, `total`, `delivery_fee`, `tax`, or `POST /orders` payload.
- The `subtotal` and `total` fields in the `CreateOrderInput` schema are always in ALL minor units.
- EUR display is ephemeral: computed on render, never persisted in DB or API payloads.

### 4. Formatting through shared utilities only
- Use `formatMoney(amount, currency, rate?)` for any user-visible monetary string.
- Use `formatALL(amount)` as shorthand for ALL-only display.
- Use `PriceDisplay` React component for frontend price rendering.
- Use `fmtPrice()` only for raw price-string construction (avoid in new code).
- **Never** write ad-hoc `{amount} ALL`, `amount.toFixed(0)`, or template-literal price strings.

## Checklist (run against every money diff)

- [ ] New monetary field stored as **integer** (not float/decimal)?
- [ ] Rounding uses central `formatMoney()` utility (not ad-hoc)?
- [ ] EUR conversion result never flows into order math or DB?
- [ ] API payloads use ALL minor units, not EUR?
- [ ] Frontend display uses `PriceDisplay` / `formatMoney` / `formatALL`?
- [ ] No ad-hoc `toFixed()` or `Math.round()` on monetary values outside formatMoney?

## Check Script

```bash
node .agents/skills/deliveryos-money-contract/scripts/check-money.mjs [path]
```

Run without arguments to scan the entire repo, or pass a specific path/file to scan only that diff.
