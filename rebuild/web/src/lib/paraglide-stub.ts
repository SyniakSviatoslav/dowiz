// PARAGLIDE STAND-IN — src/lib/paraglide-stub.ts
//
// Intended provenance: `@inlang/paraglide-js compile --project ./project.inlang --outdir
// ./src/paraglide` (script "gen:messages" in package.json.pending), which compiles
// messages/{sq,en,uk}.json into one tree-shakable function PER MESSAGE (Paraglide's whole
// value proposition — see REBUILD-MAP inventory 11 §5.2). Each generated function looks like:
//
//   export function cart_title(): string {
//     const locale = getLocale();
//     if (locale === 'sq') return 'Shporta';
//     if (locale === 'uk') return 'Кошик';
//     return 'Cart';
//   }
//
// BLOCKED: `@inlang/paraglide-js` could not be installed (dependency-approval gate, see
// rebuild/web/README.md). This file hand-implements the SAME call signature Paraglide 2
// generates (one function per message key, reading the active locale) so the rest of the
// scaffold (islands, layout) can import from a stable module path today, and swapping this
// file for the real `src/paraglide/messages.js` output is a pure import-path change — zero
// call-site churn — once the gate clears.
//
// The 10 keys below are verbatim from packages/ui/src/lib/i18n-catalog.ts (dotted keys renamed
// to underscores — Paraglide message keys must be valid JS identifiers, dots are not legal,
// see inventory 11 §5.2 "dynamic-key families" note). Values copied 1:1, not re-translated.
import { messages as sq } from './locale-data/sq';
import { messages as en } from './locale-data/en';
import { messages as uk } from './locale-data/uk';

export type Locale = 'sq' | 'en' | 'uk';
export const baseLocale: Locale = 'sq';
export const locales: readonly Locale[] = ['sq', 'en', 'uk'];

const tables: Record<Locale, Record<string, string>> = { sq, en, uk };

let currentLocale: Locale = baseLocale;
export function setLocale(locale: Locale): void {
  currentLocale = locale;
}
export function getLocale(): Locale {
  return currentLocale;
}

function make(key: string) {
  return (locale: Locale = currentLocale): string => tables[locale][key] ?? tables[baseLocale][key];
}

export const client_menu = make('client_menu');
export const cart_title = make('cart_title');
export const cart_empty = make('cart_empty');
export const cart_total = make('cart_total');
export const cart_checkout = make('cart_checkout');
export const cart_clear = make('cart_clear');
export const cart_increase = make('cart_increase');
export const cart_decrease = make('cart_decrease');
export const checkout_title = make('checkout_title');
export const client_closed_title = make('client_closed_title');
