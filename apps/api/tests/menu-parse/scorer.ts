/**
 * Menu-parse eval scorer (ADR-0011 B1) — deterministic, field-level. Blocks a MEASURED cascade-swap
 * regression on the committed fixture set; does NOT replace human review (B9 — fixtures are
 * self-authored; an independent oracle is future work).
 *
 *   price            = EXACT integer-minor-unit match, ZERO tolerance (money is exact, B1/ADR-0010).
 *   item-recall      = expected products found in actual (matched by normalized name — externalKeys
 *                      are LLM-assigned and unstable across model swaps).
 *   modifier-struct  = expected modifier-groups whose (minSelect,maxSelect,required) match in actual.
 */
import type { CanonicalMenuDraft, CanonicalProduct, CanonicalModifierGroup } from '@deliveryos/shared-types';

export interface Thresholds {
  priceExact: number;
  itemRecall: number;
  modifierStructure: number;
}

// Version-controlled thresholds (ADR-0011 B1). Tighten only with evidence; never loosen to pass.
export const THRESHOLDS: Thresholds = {
  priceExact: 1.0, // zero tolerance
  itemRecall: 0.95,
  modifierStructure: 0.9,
};

export interface EvalReport {
  priceExact: { matched: number; total: number; rate: number };
  itemRecall: { found: number; total: number; rate: number };
  modifierStructure: { matched: number; total: number; rate: number };
  pass: boolean;
  failures: string[];
}

const norm = (s: string | undefined): string => (s ?? '').trim().toLowerCase().replace(/\s+/g, ' ');

function indexByName<T extends { name: string }>(items: T[]): Map<string, T> {
  const m = new Map<string, T>();
  for (const it of items) if (!m.has(norm(it.name))) m.set(norm(it.name), it);
  return m;
}

export function scoreParse(
  expected: CanonicalMenuDraft,
  actual: CanonicalMenuDraft,
  thresholds: Thresholds = THRESHOLDS,
): EvalReport {
  const actualProducts = indexByName<CanonicalProduct>(actual.products ?? []);
  const expProducts = expected.products ?? [];

  // Item recall — every expected product must appear in actual (by name).
  let found = 0;
  // Price exact — over the FOUND products, the integer-minor price must match exactly (zero tolerance).
  let priceMatched = 0;
  for (const ep of expProducts) {
    const ap = actualProducts.get(norm(ep.name));
    if (!ap) continue;
    found++;
    if (ap.price === ep.price) priceMatched++;
  }

  const actualGroups = indexByName<CanonicalModifierGroup>(actual.modifierGroups ?? []);
  const expGroups = expected.modifierGroups ?? [];
  let structMatched = 0;
  for (const eg of expGroups) {
    const ag = actualGroups.get(norm(eg.name));
    if (ag && ag.minSelect === eg.minSelect && ag.maxSelect === eg.maxSelect && ag.required === eg.required) {
      structMatched++;
    }
  }

  const rate = (n: number, d: number) => (d === 0 ? 1 : n / d);
  // Price rate is measured over FOUND items (a missing item is an item-recall failure, not a price one),
  // so a single wrong price among present items still drops priceExact below 1.0 → fails.
  const report: EvalReport = {
    itemRecall: { found, total: expProducts.length, rate: rate(found, expProducts.length) },
    priceExact: { matched: priceMatched, total: found, rate: rate(priceMatched, found) },
    modifierStructure: { matched: structMatched, total: expGroups.length, rate: rate(structMatched, expGroups.length) },
    pass: false,
    failures: [],
  };

  const failures: string[] = [];
  if (report.priceExact.rate < thresholds.priceExact)
    failures.push(`price-exact ${(report.priceExact.rate * 100).toFixed(1)}% < ${thresholds.priceExact * 100}% (zero tolerance)`);
  if (report.itemRecall.rate < thresholds.itemRecall)
    failures.push(`item-recall ${(report.itemRecall.rate * 100).toFixed(1)}% < ${thresholds.itemRecall * 100}%`);
  if (report.modifierStructure.rate < thresholds.modifierStructure)
    failures.push(`modifier-structure ${(report.modifierStructure.rate * 100).toFixed(1)}% < ${thresholds.modifierStructure * 100}%`);
  report.failures = failures;
  report.pass = failures.length === 0;
  return report;
}
