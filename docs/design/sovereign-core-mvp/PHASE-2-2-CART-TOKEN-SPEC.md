# Phase 2.2: Cart-Token Specification v0

**Status:** Specification for Checkout via Sovereign Core  
**Gate:** Money-council review + approval BEFORE implementation  
**Depends on:** Phase 0b-5 (kernel::decide), Phase 1.2 (event log)

## Summary

The direct checkout flow (`POST /api/orders` with `x-dowiz-cutover: true`) uses a **server-priced cart** model:
- **Client sends:** item IDs + modifier IDs + quantities ONLY
- **Server computes:** prices from menu DB, applies location tax policy, adds delivery fee
- **Result:** order with full totals, immutable after creation

This spec governs the request/response contract, price authority, and invariants.

## Request Contract: `POST /api/orders` (Hub Checkout)

**Header:** `x-dowiz-cutover: true` (signals kernel::decide path)

**Body:**
```json
{
  "location_id": "uuid",
  "type": "pickup|delivery",
  "customer": {
    "phone": "+1...",
    "name": "Alice"
  },
  "items": [
    {
      "product_id": "uuid",
      "quantity": 2,
      "modifiers": [
        {
          "modifier_id": "uuid",
          "quantity": 1
        }
      ]
    }
  ],
  "delivery_details": {
    "address": "...",
    "instructions": "..."
  },
  "payment_method": "cash|card|..."
}
```

**Forbidden fields** (if present, 400 VALIDATION_FAILED):
- `subtotal`, `tax_total`, `delivery_fee`, `total` (CLIENT PRICE FIELDS)
- `discount_total` (handled server-side only)

## Price Computation (Server Authority)

### Step 1: Validate Items & Look Up Menu Prices
- For each `(product_id, modifier_id)` pair: fetch current price from `products` + `modifier_groups`
- If any product/modifier missing or inactive: 404 PRODUCT_NOT_FOUND
- **Price source:** always from DB, never from request

### Step 2: Compute Subtotal
```
subtotal = SUM(product_price * quantity + SUM(modifier_price * modifier_qty))
```
- All prices as `i64` (minor currency units, e.g., cents)
- Integer arithmetic only (no floats in core)

### Step 3: Apply Location Tax Policy
```
tax_rate = location.tax_rate (e.g., 0.20 for 20%)
tax_total_gross = subtotal * tax_rate  // gross VAT (stored on order)
tax_charged = compute_charged_tax(tax_total_gross, location.price_includes_tax)
```
- **price_includes_tax=true:** `tax_charged = tax_total_gross - (subtotal - discount)`
  - Inclusive pricing: the menu price already contains tax
  - Tax is extracted, not added
- **price_includes_tax=false:** `tax_charged = tax_total_gross`
  - Exclusive pricing: tax is added to the menu total
- **Guard:** if `tax_rate < 0 || tax_rate > 1.0`: 400 INVALID_TAX_RATE
- **Lemma (LC1):** `tax_charged ≤ tax_total_gross` (no double-taxation under either mode)

### Step 4: Compute Delivery Fee
```
delivery_fee = compute_delivery_fee(
  delivery_address,
  location.delivery_fee_flat,
  location.delivery_tier[distance_m],
  ...
)
```
- Deterministic: same address + location → same fee
- For **pickup** type: delivery_fee = 0

### Step 5: Compose Total (Conservation Invariant)
```
total = subtotal + tax_charged + delivery_fee - discount_total
```
- **Invariant (CONSERVATION):** `total ≥ 0 && all terms ≥ 0`
- **Proof:** money-council + proptest suite (Hard Truth Layer 3)

## Response: 201 Created

```json
{
  "id": "uuid",
  "location_id": "uuid",
  "type": "pickup|delivery",
  "status": "PENDING",
  "subtotal": 2500,
  "tax_total": 500,
  "tax_charged": 500,
  "delivery_fee": 350,
  "discount_total": 0,
  "total": 3350,
  "currency_code": "USD",
  "created_at": "2026-07-07T18:30:00Z",
  "...": "other order fields"
}
```

**Invariants returned:**
- `total = subtotal + tax_charged + delivery_fee - discount_total`
- `tax_charged ≤ tax_total_gross`
- All amounts ≥ 0

## Idempotency: Request Hash

Every request is hashed deterministically (from item IDs, quantities, modifiers, location, type):
```
request_hash = SHA256(canonical_request_json)
```

If the same request arrives twice (network retry), a UNIQUE constraint on `(location_id, request_hash)` ensures:
- **First call:** creates order, returns 201
- **Second call (same hash):** returns 200 with the existing order (no duplicate charge)

## Adversarial Test Suite (RED Proofs)

### Test 1: Client-Injected Price Fields → Refused
**Scenario:** Client POSTs with `subtotal: 1000` (lower than computed)  
**Expected:** 400 VALIDATION_FAILED; server-computed total is used, never client value  
**RED Proof:** Remove validation → 400 becomes 201 → test FAILS

### Test 2: Double-Create via Same Request Hash → COUNT = 1
**Scenario:** Send identical requests concurrently  
**Expected:** Both receive 201 (first) + 200 (retry), but `COUNT(orders WHERE request_hash=$1) = 1`  
**RED Proof:** Remove UNIQUE constraint → COUNT = 2 → test FAILS

### Test 3: Conservation Invariant Across All Orders
**Scenario:** Query entire `orders` table  
**Expected:** `SUM(total) = SUM(subtotal + tax_charged + delivery_fee - discount) ± 0`  
**RED Proof:** Compute tax incorrectly → invariant diverges → test FAILS  
**Gate:** Proptest over arbitrary carts + independent i64 recalculation (never re-call `compose_total`)

## Feature Flag: `hub_checkout`

**Default:** OFF (feature toggles to kernel::decide path)  
**Behavior:**
- When OFF: `x-dowiz-cutover: true` is ignored; old Node path used
- When ON: kernel::decide path is live; old path unavailable
- **Transition:** Ramp ON gradually after staging validation

## Edge Cases & Error Codes

| Scenario | Code | HTTP |
|----------|------|------|
| Missing product ID | PRODUCT_NOT_FOUND | 404 |
| Invalid tax rate | INVALID_TAX_RATE | 400 |
| Modifier not found | MODIFIER_NOT_FOUND | 404 |
| Location not found | LOCATION_NOT_FOUND | 404 |
| Client sends price field | VALIDATION_FAILED | 400 |
| Delivery beyond radius | OUT_OF_DELIVERY_AREA | 400 |

## Deployment Checklist

- [ ] Money-council sign-off (this spec)
- [ ] Hard Truth Layer 3 proptests green (conservation + LC1)
- [ ] Adversarial test suite green (all three RED proofs)
- [ ] `/reliability-gate` validation on staging (full L0–L11 proof)
- [ ] Feature flag `hub_checkout` staged (default OFF)
- [ ] x-dowiz-cutover assertion on /api/orders POST
- [ ] Replay-parity job green (Phase 1.2)

**Ready for:** Production deployment after staging validation.
