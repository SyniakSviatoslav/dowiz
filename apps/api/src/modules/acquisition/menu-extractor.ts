import type { MenuDraft, DraftProduct } from './provisioning.js';

// P6-3 — MenuExtractor: a thin adapter over the existing AiOcrParser port. It does NOT call the AI
// itself (deps point inward); it (a) transforms the parser's canonical draft into the provisioning
// MenuDraft shape, and (b) applies the H4 no-fabrication gate — turning the parser's ADVISORY
// confidence into a HARD state verdict, because there is NO human in the loop pre-claim (unlike the
// owner-upload path, where the owner reviews the draft before publish).
//
// We type against the STRUCTURAL SUBSET we read (ParseResult / CanonicalMenuDraft are not re-exported
// from the shared-types root). The real AiOcrParser result is structurally assignable to these.

export interface ExtractIssue { severity: 'error' | 'warning'; code: string; message: string }
export interface ExtractCanonicalCategory { externalKey: string; name: string }
export interface ExtractCanonicalProduct {
  externalKey: string;
  categoryKey: string;
  name: string;
  description?: string;
  price: number;
  attributesJson?: Record<string, unknown>;
}
export interface ExtractCanonicalDraft {
  categories: ExtractCanonicalCategory[];
  products: ExtractCanonicalProduct[];
}
export interface ExtractParseResult {
  draft: ExtractCanonicalDraft;
  issues: ExtractIssue[];
  summary: { valid: number; low_confidence_count?: number };
}

/** Group the parser's flat canonical draft (products keyed by categoryKey) into nested categories. */
export function toMenuDraft(canonical: ExtractCanonicalDraft): MenuDraft {
  interface Bucket { name: string; sort_order: number; products: DraftProduct[] }
  const byKey = new Map<string, Bucket>();
  canonical.categories.forEach((c, i) => byKey.set(c.externalKey, { name: c.name, sort_order: i, products: [] }));
  // Products referencing an unknown category get a synthetic "Menu" bucket (never dropped silently).
  const fallback: Bucket = { name: 'Menu', sort_order: byKey.size, products: [] };
  for (const p of canonical.products) {
    const bucket = byKey.get(p.categoryKey) ?? fallback;
    bucket.products.push({
      name: p.name,
      price: p.price,
      description: p.description ?? null,
      attributes: p.attributesJson ?? null,
    });
  }
  const categories: Bucket[] = [...byKey.values()];
  if (fallback.products.length) categories.push(fallback);
  return { categories: categories.filter((c) => c.products.length > 0) };
}

export type ExtractionVerdict = 'ENRICHED' | 'LOW_QUALITY' | 'MANUAL_REVIEW';
export interface ExtractionDecision {
  verdict: ExtractionVerdict;
  reason?: string; // REQUIRED for LOW_QUALITY/MANUAL_REVIEW (state-machine REQUIRES_REASON)
  draft?: MenuDraft; // only for ENRICHED
}

/**
 * H4 no-fabrication gate. Maps a parse result to a HARD verdict (NOT advisory):
 *  - any PII_DENSE error (the C1 fail-closed) → MANUAL_REVIEW (a human looks before anything proceeds)
 *  - 0 valid items → MANUAL_REVIEW (nothing to write)
 *  - any other error-severity issue → LOW_QUALITY
 *  - any low-confidence items → LOW_QUALITY (a low-confidence scraped menu, unreviewed, must not publish)
 *  - else → ENRICHED with the transformed draft
 * ENRICHED is the ONLY door to PROVISIONED (state-machine), so "low confidence cannot write" is structural.
 */
export function classifyExtraction(result: ExtractParseResult): ExtractionDecision {
  const errors = result.issues.filter((i) => i.severity === 'error');
  const piiDense = errors.find((i) => i.code === 'PII_DENSE');
  if (piiDense) return { verdict: 'MANUAL_REVIEW', reason: `PII guard tripped: ${piiDense.message}` };

  if ((result.summary.valid ?? 0) === 0) {
    return { verdict: 'MANUAL_REVIEW', reason: 'parser produced 0 valid items' };
  }
  if (errors.length > 0) {
    return { verdict: 'LOW_QUALITY', reason: `extraction had ${errors.length} error(s): ${errors[0]!.message}` };
  }
  const lowConf = result.summary.low_confidence_count ?? 0;
  if (lowConf > 0) {
    return { verdict: 'LOW_QUALITY', reason: `${lowConf} low-confidence item(s); below the no-fabrication write threshold` };
  }
  return { verdict: 'ENRICHED', draft: toMenuDraft(result.draft) };
}
