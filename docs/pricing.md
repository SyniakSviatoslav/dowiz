# Server Pricing Engine (Phase 2, Stage 10)

## Overview
DeliveryOS pricing calculation is strictly **Server-Side Wins**. The client submits product IDs, modifier IDs, and quantities. The server derives the final price mathematically and transactionally ignores any pricing inputs provided by the client payload.

## Formula
\`\`\`text
Line Item Total = (Product.price + SUM(Modifier.price_delta)) * Quantity
Subtotal = SUM(Line Item Totals)
Total = Subtotal + Delivery Fee + Tax - Discount
\`\`\`

## Modifiers Pricing Rule
Modifier prices are calculated **per unit**. If an order item has a quantity of `2` and includes a modifier with `price_delta = 150`, the total cost added by that modifier is `150 * 2 = 300`. This is to accommodate volume adjustments efficiently.

## Delivery Fee Logic
Calculated conditionally through Location configurations:
1. **Min Order Value:** If `subtotal < location.min_order_value`, the request is rejected with `422 MIN_ORDER_NOT_MET`.
2. **Free Delivery Threshold:** If `subtotal >= location.free_delivery_threshold`, the delivery fee is set to `0`.
3. **Delivery Tiers:** The server calculates the Haversine distance (`lib/geo.ts`) from `delivery.pin` to `location.lat/lng`. It finds the tier where `dist <= tier.max_distance_km` and applies `tier.fee`. If no tier is matched, it rejects with `422 NOT_DELIVERABLE`.
4. **Fallback Flat Fee:** If no tiers are configured, or coordinates are missing, it uses `location.delivery_fee_flat`.
5. **Not Configured:** If no fee resolution is possible, it rejects with `422 DELIVERY_NOT_CONFIGURED`.

## Rounding Rule
All monetary variables in DeliveryOS are represented in **minor units** (e.g. cents) as integers. Floating point arithmetic is prohibited for price derivation. Computations involving fractional ratios (like Taxes) resolve using integer `roundHalfUp(value)` leveraging `BigInt` under the hood to completely avoid JS floating-point issues (`lib/money.ts`).

## Error Codes
- `422 PRODUCT_UNAVAILABLE`: Item out of stock or disabled.
- `422 PRODUCT_NOT_FOUND`: Product doesn't exist for location.
- `422 MODIFIER_UNAVAILABLE`: Modifier disabled or unmapped to product.
- `422 MODIFIER_MIN_NOT_MET`: Product requires selection for modifier group.
- `422 MODIFIER_MAX_EXCEEDED`: Exceeded max choices in modifier group.
- `422 IDEMPOTENCY_KEY_REUSED`: Hash canonicalization mismatch for key.
