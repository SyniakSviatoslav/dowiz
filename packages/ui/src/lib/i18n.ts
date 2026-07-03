import { safeStorage } from '../utils/safeStorage.js';
import { catalog } from './i18n-catalog.js';
export type Locale = 'sq' | 'en' | 'uk';

const STORAGE_KEY = 'dos_locale';

let currentLocale: Locale = (typeof window !== 'undefined'
  ? (safeStorage.get(STORAGE_KEY) as Locale) || 'sq'
  : 'sq') as Locale;

// Locale-major view DERIVED from the key-major catalog (the single source of truth).
// Add/edit strings in i18n-catalog.ts, never here.
function fromCatalog(cat: Record<string, Partial<Record<Locale, string>>>): Record<Locale, Record<string, string>> {
  const out: Record<Locale, Record<string, string>> = { sq: {}, en: {}, uk: {} };
  for (const key in cat) {
    const e = cat[key]!;
    (['sq', 'en', 'uk'] as const).forEach((l) => { if (e[l] !== undefined) out[l][key] = e[l]!; });
  }
  return out;
}
const messages: Record<Locale, Record<string, string>> = fromCatalog(catalog);


const listeners: Array<() => void> = [];

export function t(key: string, fallback?: string, options?: Record<string, any>): string {
  return translate(currentLocale, key, fallback, options);
}

// Stateless translation lookup — used by I18nProvider for reactive t()
export function translate(locale: Locale, key: string, fallback?: string, options?: Record<string, any>): string {
  const hit = messages[locale]?.[key];
  // Dev-only: a missing key silently falls back to English/raw key in prod — warn loudly in dev so
  // gaps surface during development. The CI parity gate (scripts/i18n-parity.mjs) is the hard stop.
  if (hit === undefined && (import.meta as any)?.env?.DEV) {
    console.warn(`[i18n] missing key "${key}" for locale "${locale}" — add it in i18n-catalog.ts`);
  }
  let str = hit || fallback || key;
  if (options) {
    for (const [k, v] of Object.entries(options)) {
      str = str.replace(new RegExp(`\\{\\{${k}\\}\\}`, 'g'), String(v));
    }
  }
  return str;
}

export function setLocale(locale: Locale, persist = true): void {
  currentLocale = locale;
  // `persist` is false when the storefront applies the TENANT default_locale on first
  // load: that is not the user's own choice, so it must not be written to storage (where
  // it would later shadow a different tenant's default). An explicit pick (LanguageSwitcher)
  // persists as before.
  if (persist && typeof window !== 'undefined') {
    safeStorage.set(STORAGE_KEY, locale);
  }
  if (typeof document !== 'undefined') document.documentElement.lang = locale;
  listeners.forEach(fn => fn());
}

export function getLocale(): Locale {
  return currentLocale;
}

export function getLocales(): Array<{ code: Locale; name: string; displayCode: string }> {
  return [
    { code: 'sq', name: messages.sq?.['language.name'] || 'Shqip', displayCode: 'SQ' },
    { code: 'en', name: messages.en?.['language.name'] || 'English', displayCode: 'EN' },
    { code: 'uk', name: messages.uk?.['language.name'] || 'Ukrainska', displayCode: 'UA' },
  ];
}

export const SUPPORTED_LOCALES: readonly Locale[] = ['sq', 'en', 'uk'];

export function isLocale(value: unknown): value is Locale {
  return value === 'sq' || value === 'en' || value === 'uk';
}

/** The user's OWN stored language choice, or null if they have never picked one. A stored
 *  value is an explicit preference that overrides the tenant default. */
export function getStoredLocale(): string | null {
  return typeof window !== 'undefined' ? safeStorage.get(STORAGE_KEY) : null;
}

/**
 * Resolve the initial storefront locale. Precedence:
 *   1. the user's own stored choice (explicit preference ALWAYS wins);
 *   2. the tenant's configured default_locale (honored when the user has no preference) —
 *      this is the bug fix: the storefront used to ignore default_locale and always fall
 *      back to the hard-coded module default;
 *   3. the first supported locale, then 'sq'.
 * A candidate is accepted only if it is in `supported` (when a non-empty list is given), so
 * a language the menu cannot render is never selected. Pure — no storage/DOM — so it is
 * unit-testable and reused by the storefront on first menu load.
 */
export function resolveInitialLocale(opts: {
  stored?: string | null;
  tenantDefault?: string | null;
  supported?: readonly string[];
}): string {
  const { stored, tenantDefault, supported } = opts;
  const allowed = (l: string | null | undefined): l is string =>
    !!l && (!supported || supported.length === 0 || supported.includes(l));
  if (allowed(stored)) return stored;
  if (allowed(tenantDefault)) return tenantDefault;
  return (supported && supported[0]) || tenantDefault || stored || 'sq';
}

export function subscribeToLocale(fn: () => void): () => void {
  listeners.push(fn);
  return () => {
    const idx = listeners.indexOf(fn);
    if (idx >= 0) listeners.splice(idx, 1);
  };
}
