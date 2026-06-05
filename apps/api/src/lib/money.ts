export function toMinorUnit(amount: number, currency: string): number {
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

export function applyTax(subtotal: number, taxRate: number, priceIncludesTax: boolean, minorUnit: number): number {
  if (priceIncludesTax) {
    // tax = subtotal - (subtotal / (1 + taxRate))
    const tax = subtotal - (subtotal / (1 + taxRate));
    return roundHalfUp(tax, minorUnit);
  } else {
    const tax = subtotal * taxRate;
    return roundHalfUp(tax, minorUnit);
  }
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
