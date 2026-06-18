import type { CurrencyCode } from '@deliveryos/shared-types';

const STORAGE_KEY = 'dos_currency';

let currentCurrency: CurrencyCode = (typeof window !== 'undefined'
  ? (localStorage.getItem(STORAGE_KEY) as CurrencyCode) || 'ALL'
  : 'ALL') as CurrencyCode;

const listeners = new Set<() => void>();

export function getCurrency(): CurrencyCode {
  return currentCurrency;
}

export function setCurrency(code: CurrencyCode) {
  currentCurrency = code;
  if (typeof window !== 'undefined') {
    localStorage.setItem(STORAGE_KEY, code);
  }
  listeners.forEach(fn => fn());
}

export function subscribeToCurrency(fn: () => void): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

export function getCurrencies(): { code: CurrencyCode; name: string; symbol: string }[] {
  return [
    { code: 'ALL', name: 'Lek', symbol: 'L' },
    { code: 'EUR', name: 'Euro', symbol: '\u20AC' },
  ];
}
