export const PHONE_E164_REGEX = /^\+[1-9]\d{6,14}$/;
export const PHONE_E164_PATTERN = '[+][1-9][0-9]{6,14}';

export type CurrencyCode = 'ALL' | 'EUR';

export interface CurrencyConfig {
  code: CurrencyCode;
  symbol: string;
  locale: string;
  decimals: number;
}

export const CURRENCIES: Record<CurrencyCode, CurrencyConfig> = {
  ALL: { code: 'ALL', symbol: 'L', locale: 'sq-AL', decimals: 0 },
  EUR: { code: 'EUR', symbol: '€', locale: 'de-DE', decimals: 2 },
};

const HALF_UP = new Intl.NumberFormat('en-US', { minimumFractionDigits: 0, maximumFractionDigits: 8 });

export function formatMoney(
  amountInCents: number,
  displayCurrency: CurrencyCode = 'ALL',
  rate?: number,
): string {
  if (displayCurrency === 'ALL') {
    const all = Math.round(amountInCents / 100);
    return `${all} ALL`;
  }

  if (!rate || rate <= 0) {
    const all = Math.round(amountInCents / 100);
    return `${all} ALL`;
  }

  // Convert ALL cents to EUR (display-only)
  const allAmount = amountInCents / 100;
  const eurAmount = allAmount * rate;
  const decimals = CURRENCIES.EUR.decimals;
  const rounded = Math.round(eurAmount * Math.pow(10, decimals)) / Math.pow(10, decimals);
  return `${rounded.toFixed(decimals)} €`;
}

export function formatALL(amountInCents: number): string {
  return formatMoney(amountInCents, 'ALL');
}

export function normalizePhone(phone: string): string {
  // Simple normalization: strip all non-numeric, prepend +355 if no country code
  let clean = phone.replace(/\D/g, '');
  if (clean.length === 9) { // Assuming 9 digit albanian numbers
    clean = `355${clean}`;
  }
  return `+${clean}`;
}

export function calcETA(createdAt: string | Date, elapsedSeconds: number): string {
  // Mock logic
  const created = new Date(createdAt);
  return '15-25 min';
}
