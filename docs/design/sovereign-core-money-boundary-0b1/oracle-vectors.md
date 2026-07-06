# Oracle vectors — sovereign-core money boundary (0b-1)

**Decorrelated, independent oracle.** Every value below is hand-derived from the **Node/TS reference
source only** (`apps/api/src/lib/money.ts`, `apps/api/src/lib/order-pricing.ts`,
`apps/api/src/routes/orders.ts` §8, `apps/api/src/lib/geo.ts`) plus first-principles arithmetic. **No
Rust under `rebuild/` was read.** This file is a non-mirror check: if it agreed with the port only
because it copied the port, it would be worthless.

Ground-truth guard, quoted verbatim from `apps/api/src/lib/money.ts:5`:

```js
if (subtotal === 0 || taxRate === 0) return 0;
```

Then `apps/api/src/lib/money.ts:11`:

```js
const rateMicro = BigInt(Math.round(taxRate * 1_000_000));
```

BigInt division truncates **toward zero** (not floor). The `+ SCALE/2n` / `+ denom/2n` terms are the
half-up bias for positive operands.

---

## A. Tax-rate guard edge cases (subtotal = 1000)

**Crux finding up front.** The Node `applyTax` guard is **only** `subtotal === 0 || taxRate === 0`.
There is **no** `taxRate <= 0` arm and **no** `!isFinite` arm in the Node source. So Node does **NOT**
short-circuit to 0 on a negative or non-finite rate. Instead:

- a **negative** rate flows through and produces a **negative tax** (undercharge), and
- **±Infinity / NaN** reach `BigInt(Math.round(rate * 1e6))`, where `BigInt(Infinity)` / `BigInt(NaN)`
  **throw `RangeError`** (`Math.round(±Inf) = ±Inf`, `Math.round(NaN) = NaN`; neither is an integer, so
  the `BigInt(...)` conversion throws before either branch runs).

| tax_rate | priceIncludesTax | Node `applyTax(1000, …)` result | derivation |
|---|---|---|---|
| **-0.2** | false (exclusive) | **-199** | `rateMicro = round(-0.2·1e6) = -200000`. `(1000·-200000 + 1000000/2) / 1000000 = (-200000000 + 500000)/1000000 = -199500000/1000000 = -199.5` → trunc-toward-zero → **-199** |
| **-0.2** | true (inclusive) | **-250** | `denom = 1000000 + (-200000) = 800000`. `net = (1000·1000000 + 800000/2)/800000 = (1000000000 + 400000)/800000 = 1000400000/800000 = 1250.5` → trunc → `1250`. `tax = 1000 - 1250 = ` **-250** |
| **0** | false | **0** | guard `taxRate === 0` → `return 0` |
| **0** | true | **0** | guard `taxRate === 0` → `return 0` (fires before the branch) |
| **+Infinity** | false | **throws `RangeError`** | not caught by guard (`Infinity !== 0`); `Math.round(Infinity·1e6)=Infinity`; `BigInt(Infinity)` → RangeError ("cannot convert Infinity to a BigInt") |
| **+Infinity** | true | **throws `RangeError`** | same throw at the `rateMicro` line, before the inclusive branch |
| **-Infinity** | false | **throws `RangeError`** | `BigInt(-Infinity)` → RangeError |
| **-Infinity** | true | **throws `RangeError`** | same |
| **NaN** | false | **throws `RangeError`** | `NaN !== 0`; `Math.round(NaN)=NaN`; `BigInt(NaN)` → RangeError ("cannot convert NaN to a BigInt") |
| **NaN** | true | **throws `RangeError`** | same |

**Contrast with the port's intended behavior (H1 / resolution.md).** The design restores `Ok(0)` for
all four exotic classes (core `rate_micro <= 0` → negative & zero; shell `!tax_rate.is_finite()` →
±Inf & NaN). That target is **byte-parity with the PRIOR Rust `apply_tax` short-circuit**
(`pricing.rs:50`: `subtotal == 0 || tax_rate <= 0.0 || !tax_rate.is_finite()`), **not** with the
current Node reference. Against the Node reference, the new Rust guard is a **deliberate divergence**:
where Node returns `-199 / -250` (negative rate) or **throws** (±Inf/NaN), the port returns `Ok(0)`.
This is defensible (production reads `tax_rate` as `unwrap_or(0.0)` off a nullable `numeric`; a
negative rate is a misconfig, and `Ok(0)` is safer than an undercharge or a 5xx), but it must be named
as an intentional guard, not sold as "byte-parity with Node." The only exotic input where Node and the
port already agree is **zero rate** (both → 0). See summary.

---

## B. `rate_micro` conversion vectors — `rate_micro = round(tax_rate · 1_000_000)`

`Math.round` is round-half-up (ties toward +∞). All six inputs are positive and land cleanly (the tiny
IEEE-754 residue of e.g. `0.075·1e6 = 75000.0000000000027` is far from any .5 tie), so:

| tax_rate | tax_rate · 1e6 (exact-ish) | round | **rate_micro** |
|---|---|---|---|
| 0.075 | 75000.000… | 75000 | **75000** |
| 0.1 | 100000.000… | 100000 | **100000** |
| 0.2 | 200000.000… | 200000 | **200000** |
| 0.0744 | 74400.000… | 74400 | **74400** |
| 0.0745 | 74500.000… | 74500 | **74500** |
| 0.0825 | 82500.000… | 82500 | **82500** |

Cross-checks proposal §6's stated set (`0.075→75000, 0.0745→74500, 0.0744→74400, 0.2→200000,
0.0825→82500`) and adds `0.1→100000`. Independently re-derived, not copied.

---

## C. Integer-meter delivery-fee vectors

**Precedence, read off `orders.ts` §8 (lines 497–525) + `order-pricing.ts` `resolveDeliveryFee`
(lines 168–184).** The integer-domain `delivery_fee_for_order(subtotal, is_pickup, location,
distance_m, tiers)` composes them in this order:

1. **MIN_ORDER_NOT_MET** first, for **both** pickup and delivery (`orders.ts:498`, *before* the
   `if (!isPickup)` branch): `min_order_value !== null && subtotal < min_order_value` → error.
2. **pickup** → fee `0`, no further checks (`orders.ts:508` gates the whole delivery block).
3. **free_delivery_threshold** (`orders.ts:509`): `!== null && subtotal >= threshold` → fee `0`
   (tier logic never runs — distance is irrelevant).
4. **tier ladder** (`resolveDeliveryFee`): if `tiers` non-empty **and** `distance_m` present, iterate
   tiers ASC by `max_distance_m`; first tier with `distance_m <= max_distance_m` → `tier.fee`; none →
   **NOT_DELIVERABLE**. Else if `delivery_fee_flat !== null` → flat. Else → **DELIVERY_NOT_CONFIGURED**.

Wire codes/messages, quoted verbatim:
- `order-pricing.ts:178` → `{ code: 'NOT_DELIVERABLE', message: 'Location out of delivery range' }`
- `order-pricing.ts:183` → `{ code: 'DELIVERY_NOT_CONFIGURED', message: 'Delivery not configured' }`
- `orders.ts:502` → `code: 'MIN_ORDER_NOT_MET'` (message `'Minimum order value not met'`)

All amounts in minor units; `distance_m` / `max_distance_m` in whole meters.

| # | scenario | subtotal | is_pickup | tiers `{max_distance_m, fee}` | distance_m | flat | free_threshold | min_order | **expected** | derivation |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | within tier 1 | 2000 | false | `[{1000,300},{5000,500}]` | 800 | null | null | null | **fee 300** | min N/A; not pickup; free N/A; `800 <= 1000` → tier1 |
| 2 | between t1 & t2 → t2 | 2000 | false | `[{1000,300},{5000,500}]` | 1390 | null | null | null | **fee 500** | `1390 <= 1000`? no; `1390 <= 5000`? yes → tier2 |
| 3 | beyond last tier | 2000 | false | `[{1000,300},{5000,500}]` | 7000 | null | null | null | **NOT_DELIVERABLE** | `7000<=1000` no, `7000<=5000` no → no tier covers |
| 4 | no tiers + flat | 2000 | false | `[]` | null | 250 | null | null | **fee 250** | tiers empty → skip ladder; `delivery_fee_flat = 250 !== null` → flat |
| 5 | no tiers + no flat | 2000 | false | `[]` | null | null | null | null | **DELIVERY_NOT_CONFIGURED** | tiers empty; flat null → configured-error |
| 6 | free threshold met | 5000 | false | `[{1000,300}]` | 9999 | null | 3000 | null | **fee 0** | `5000 >= 3000` → free; tier ladder (and the 9999 out-of-range distance) never evaluated |
| 7 | pickup | 2000 | true | `[{1000,300}]` | 9999 | null | null | null | **fee 0** | pickup short-circuits; tiers/distance ignored |
| 8a | min not met (delivery) | 500 | false | `[{1000,300}]` | 500 | null | null | 1000 | **MIN_ORDER_NOT_MET** | `500 < 1000` → error before any fee resolution |
| 8b | min not met (pickup) | 500 | true | `[]` | null | null | null | 1000 | **MIN_ORDER_NOT_MET** | min check precedes the pickup fee=0 → error fires for pickup too |

Boundary note (not a listed vector, but load-bearing): the compare is `<=`, so `distance_m` **exactly
equal** to a tier's `max_distance_m` selects that tier (e.g. `distance_m=1000`, tier `{1000,300}` →
fee 300). This is the equality edge the km→m round-half-up in the shim must preserve.

---

## D. Cross-check of 3 existing `ORDER_TOTAL_VECTORS`

Re-derived independently from `money.ts` formulas + composition
`total = subtotal + deliveryFee + (priceIncludesTax ? 0 : taxTotal)`.

**D1 — inclusive (the LC1 case).** `subtotal=1200, taxRate=0.2, incl=true, fee=250`.
- `rateMicro = round(0.2·1e6) = 200000`; `denom = 1000000 + 200000 = 1200000`.
- `net = (1200·1000000 + 1200000/2)/1200000 = (1200000000 + 600000)/1200000 = 1200600000/1200000 = 1000.5` → trunc → `1000`.
- `taxTotal = 1200 - 1000 = 200`; `chargedTax = 0` (inclusive); `total = 1200 + 250 + 0 = 1450`.
- Vector says `expectedTax 200, expectedChargedTax 0, expectedTotal 1450`. **MATCH.**

**D2 — exclusive.** `subtotal=1000, taxRate=0.2, incl=false, fee=200`.
- `rateMicro = 200000`; `tax = (1000·200000 + 1000000/2)/1000000 = (200000000 + 500000)/1000000 = 200500000/1000000 = 200.5` → trunc → `200`.
- `taxTotal = 200`; `chargedTax = 200` (exclusive); `total = 1000 + 200 + 200 = 1400`.
- Vector says `expectedTax 200, expectedChargedTax 200, expectedTotal 1400`. **MATCH.**

**D3 — zero rate (inclusive).** `subtotal=1000, taxRate=0, incl=true, fee=250`.
- guard `taxRate === 0` → `taxTotal = 0`; `chargedTax = 0`; `total = 1000 + 250 + 0 = 1250`.
- Vector says `expectedTax 0, expectedChargedTax 0, expectedTotal 1250`. **MATCH.**

All three vectors reproduced from first principles — no discrepancy.

---

## Vector count

- **A:** 10 tax-guard results (5 rates × 2 modes).
- **B:** 6 `rate_micro` conversions.
- **C:** 9 delivery-fee outcomes (8 scenarios; #8 split pickup/delivery).
- **D:** 3 cross-checked composition vectors.

**Total: 28 hand-derived expected values.**
