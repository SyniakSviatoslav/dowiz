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
  amount: number,
  displayCurrency: CurrencyCode = 'ALL',
  rate?: number,
): string {
  if (displayCurrency === 'ALL') {
    return `${Math.round(amount)} ALL`;
  }

  if (!rate || rate <= 0) {
    return `${Math.round(amount)} ALL`;
  }

  // Convert ALL to EUR using scaled integer arithmetic (display-only)
  const rateScaled = Math.round(rate * 1_000_000_000);
  const eurCents = BigInt(amount) * BigInt(rateScaled) * 100n;
  const scale = BigInt(10) ** BigInt(9);
  const eurCentsRounded = Number((eurCents + scale / 2n) / scale);
  const decimals = CURRENCIES.EUR.decimals;
  const rounded = eurCentsRounded / Math.pow(10, decimals);
  return `${rounded.toFixed(decimals)} €`;
}

export function formatALL(amount: number): string {
  return formatMoney(amount, 'ALL');
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

export function shortId(id: string): string {
  return '#' + id.substring(0, 4).toUpperCase();
}

export function shortIdRaw(id: string): string {
  return id.substring(0, 4).toUpperCase();
}

export function ensureCurrency(code: string | null | undefined, fallback: CurrencyCode = 'ALL'): CurrencyCode {
  if (code === 'ALL' || code === 'EUR') return code;
  return fallback;
}

export function fmtPrice(amount: number, currency: string = 'ALL', decimals?: number): string {
  const d = decimals ?? (currency === 'EUR' ? 2 : 0);
  return `${amount.toFixed(d)} ${currency}`;
}
