/**
 * Menu-parse hallucination grounding (ADR-0011 B2, B7-corrected). Dev/staging-gated by
 * MENU_GROUNDING_ENABLED — the enrichment is held behind the B5/B12 RLS gate (import_sessions
 * FORCE + strengthened verify:rls) and ships dark until then.
 *
 * A parsed price is `grounded` iff the OCR text contains a price-token that NORMALIZES to the same
 * integer minor value — compared via the SAME normalizer the parser uses (extracted verbatim from
 * ai-ocr-parser priceOf). NOT a substring: a substring false-flags `"1.200 Lek"` (reads as 1.2) and
 * false-passes `price:1` (the char "1" appears everywhere). An ungrounded price is a likely
 * hallucination → flagged for the owner draft-review; never auto-published.
 */

/**
 * Extract a trailing price token from one line and normalize it to integer minor units.
 * Verbatim logic from ai-ocr-parser priceOf (the single source of price normalization, B7).
 */
export function extractTrailingPriceMinor(
  line: string,
  minorUnit: number,
): { minor: number; index: number; raw: string } | null {
  // Currency-tagged or trailing number: "... 8.50 EUR", "... 800 Lek", "... €12", "... 1.200".
  const re = /(?:€|£|\$|lek|all|eur|usd|gbp)?\s*([0-9][0-9.,\s]*[0-9]|[0-9])\s*(?:€|£|\$|lek|all|eur|usd|gbp)?\s*$/i;
  const m = line.match(re);
  if (!m || !m[1]) return null;
  const raw = m[1];
  let num = raw.replace(/\s/g, '');
  if (num.includes('.') && num.includes(',')) {
    num = num.lastIndexOf(',') > num.lastIndexOf('.')
      ? num.replace(/\./g, '').replace(',', '.')
      : num.replace(/,/g, '');
  } else if (num.includes(',')) {
    num = /,\d{3}$/.test(num) ? num.replace(/,/g, '') : num.replace(',', '.');
  } else if (/\.\d{3}$/.test(num)) {
    num = num.replace(/\./g, ''); // "1.200" → 1200 (thousands)
  }
  const value = Number(num);
  if (!isFinite(value) || value <= 0 || value > 1_000_000) return null;
  const minor = Math.round(value * Math.pow(10, minorUnit));
  if (minor <= 0) return null;
  return { minor, index: m.index ?? line.lastIndexOf(raw), raw: m[0] };
}

/** Set of all normalized minor-unit price values present in the OCR text (per line). */
export function collectOcrPriceMinors(ocrText: string, minorUnit: number): Set<number> {
  const minors = new Set<number>();
  for (const line of ocrText.split('\n')) {
    const p = extractTrailingPriceMinor(line, minorUnit);
    if (p) minors.add(p.minor);
  }
  return minors;
}

export interface GroundableItem {
  externalKey?: string;
  name?: string;
  price: number; // integer minor units (parser already normalized — ai-ocr-parser:756)
}

export interface GroundingResult {
  groundedCount: number;
  ungrounded: { externalKey?: string; name?: string; price: number }[];
}

/**
 * Mark each item grounded iff its (already-minor) price is among the OCR's normalized price tokens.
 * Both sides are integer minor units → an exact set lookup (no tolerance — money is exact, B1/ADR-0011).
 */
export function computeGrounding(items: GroundableItem[], ocrMinors: Set<number>): GroundingResult {
  const ungrounded: GroundingResult['ungrounded'] = [];
  let groundedCount = 0;
  for (const it of items) {
    if (typeof it.price === 'number' && ocrMinors.has(it.price)) groundedCount++;
    else ungrounded.push({ externalKey: it.externalKey, name: it.name, price: it.price });
  }
  return { groundedCount, ungrounded };
}

/** Convenience: ground `items` directly against raw OCR text. */
export function groundItems(
  items: GroundableItem[],
  ocrText: string,
  minorUnit: number,
): GroundingResult {
  return computeGrounding(items, collectOcrPriceMinors(ocrText, minorUnit));
}
