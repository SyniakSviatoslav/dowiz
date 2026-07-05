// Pure client-side MIRROR of the server's authoritative order-total math
// (apps/api/src/routes/orders.ts fee ladder + apps/api/src/lib/money.ts applyTax).
//
// Approach M (ADR-0005): the SERVER stays the single source of truth for what is CHARGED.
// This mirror only drives what the customer SEES before they commit — and its correctness is
// enforced two ways: a red→green parity guardrail (estimateOrderTotal === server total for a
// matrix of inputs) and the runtime cash-422 backstop. If the mirror ever drifts, CI goes red
// and the server still rejects an under-quoted cash amount — the charged amount never moves.
//
// RED LINE: all money is integer minor units; zero float arithmetic on monetary values.

/**
 * Mirror of apps/api/src/lib/money.ts `applyTax` (the BigInt branches, half-up). The server's
 * `_minorUnit` param is dead code there, so it is intentionally absent here.
 */
export function applyTax(subtotal: number, taxRate: number, priceIncludesTax: boolean): number {
  if (!Number.isInteger(subtotal)) {
    throw new Error('subtotal must be an integer (minor units)');
  }
  if (subtotal === 0 || taxRate === 0) return 0;

  const SCALE = 1_000_000n;
  const rateMicro = BigInt(Math.round(taxRate * 1_000_000));
  const sub = BigInt(subtotal);

  if (priceIncludesTax) {
    // net = round(subtotal * SCALE / (SCALE + rate)); tax = subtotal - net
    const denom = SCALE + rateMicro;
    const net = (sub * SCALE + denom / 2n) / denom; // half-up
    return Number(sub - net);
  }
  // tax = round(subtotal * rate / SCALE)
  return Number((sub * rateMicro + SCALE / 2n) / SCALE); // half-up
}

export interface FeeConfig {
  isPickup: boolean;
  freeDeliveryThreshold: number | null;
  deliveryFeeFlat: number | null;
  /** True if the venue uses distance tiers — those are RLS-hidden from the public /info, so the
   *  client genuinely cannot compute the fee and must NOT pre-quote an exact (cash) total. */
  hasDistanceTiers: boolean;
}

/**
 * Mirror of the server fee ladder (orders.ts:528-560). Returns `null` when the fee cannot be
 * computed client-side (distance-tiered, or delivery simply not configured) → the caller must
 * degrade to "fee confirmed at checkout" rather than show a number it can't back.
 */
export function computeDeliveryFee(subtotal: number, cfg: FeeConfig): number | null {
  if (cfg.isPickup) return 0;
  if (cfg.freeDeliveryThreshold !== null && subtotal >= cfg.freeDeliveryThreshold) return 0;
  if (cfg.hasDistanceTiers) return null; // distance-based — server-only
  if (cfg.deliveryFeeFlat !== null) return cfg.deliveryFeeFlat;
  return null; // delivery not configured
}

export interface OrderTotalConfig extends FeeConfig {
  taxRate: number;
  priceIncludesTax: boolean;
  minOrderValue: number | null;
}

export interface OrderTotalEstimate {
  /** True only when the delivery fee is computable client-side (flat/free/pickup). */
  feeKnown: boolean;
  deliveryFee: number | null;
  /** The VAT figure, for display/records. On inclusive venues this is EXTRACTED from the
   *  subtotal (already paid inside the price), not added on top. */
  taxTotal: number;
  /** The tax actually ADDED to `total`: `taxTotal` on exclusive venues, 0 on inclusive. The FE
   *  renders the tax line from this (never `taxTotal`) so it can't structurally show an inclusive
   *  VAT as an addend (ADR-audit-fix-money M7). */
  chargedTax: number;
  /** The authoritative-by-construction total, or null when the fee is unknown (tiered/unconfigured). */
  total: number | null;
  /** Mirrors the server's MIN_ORDER_NOT_MET gate (orders.ts:519 — applies to pickup AND delivery). */
  minNotMet: boolean;
}

/**
 * Mirror of the full server order-total computation (orders.ts:518-565). Tax is on the subtotal
 * (not subtotal+fee), matching the server. On inclusive venues the tax is already inside the
 * subtotal, so it contributes 0 to the charge — adding it would double-charge (LC1). `discountTotal`
 * is 0 server-side today (orders.ts:564).
 */
export function estimateOrderTotal(subtotal: number, cfg: OrderTotalConfig): OrderTotalEstimate {
  const deliveryFee = computeDeliveryFee(subtotal, cfg);
  const taxTotal = applyTax(subtotal, cfg.taxRate, cfg.priceIncludesTax);
  const chargedTax = cfg.priceIncludesTax ? 0 : taxTotal;
  const minNotMet = cfg.minOrderValue !== null && subtotal < cfg.minOrderValue;
  const feeKnown = deliveryFee !== null;
  const total = feeKnown ? subtotal + (deliveryFee as number) + chargedTax : null;
  return { feeKnown, deliveryFee, taxTotal, chargedTax, total, minNotMet };
}
