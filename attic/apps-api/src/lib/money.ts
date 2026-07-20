export function toMinorUnit(amount: number, _currency: string): number {
  if (!Number.isInteger(amount)) {
    throw new Error('Amount must be an integer');
  }
  return amount;
}

export function roundHalfUp(value: number, minorUnit: number): number {
  // Using BigInt internally to avoid float issues for rounding
  // Scale everything by 10
  const scaled = BigInt(Math.round(value * 10));
  const absScaled = scaled < 0n ? -scaled : scaled;
  const remainder = absScaled % 10n;
  
  let result = absScaled / 10n;
  if (remainder >= 5n) {
    result += 1n;
  }
  
  return Number(scaled < 0n ? -result : result);
}

export function applyTax(subtotal: number, taxRate: number, priceIncludesTax: boolean, _minorUnit: number): number {
  if (!Number.isInteger(subtotal)) {
    throw new Error('subtotal must be an integer (minor units)');
  }
  if (subtotal === 0 || taxRate === 0) return 0;

  // Keep all arithmetic on the monetary value in integer/BigInt space (RED LINE:
  // zero float arithmetic on money). The tax rate is a configuration input, not
  // money, so it is parsed once into integer micro-units (6 dp of precision).
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

export function computeLineTotal(productPrice: number, modifierPrices: number[], quantity: number): number {
  let unitTotal = productPrice;
  for (const modPrice of modifierPrices) {
    unitTotal += modPrice;
  }
  return unitTotal * quantity;
}

export function assertNonNegative(total: number): void {
  if (total < 0) {
    throw new Error('Total cannot be negative');
  }
}
