import { test } from 'node:test';
import assert from 'node:assert/strict';
import { classifyExtraction, toMenuDraft } from '../src/modules/acquisition/menu-extractor.js';
import type { ExtractParseResult, ExtractCanonicalDraft } from '../src/modules/acquisition/menu-extractor.js';

type ParseResult = ExtractParseResult;
type CanonicalMenuDraft = ExtractCanonicalDraft;

// P6-3 (council H4) — the no-fabrication gate must be ENFORCED, not advisory: a low-confidence or
// error-laden scrape (no human in the loop pre-claim) must NOT reach ENRICHED → PROVISIONED → publish.

const emptyDraft: CanonicalMenuDraft = { categories: [], products: [] };
function result(over: Partial<ParseResult>): ParseResult {
  return {
    draft: emptyDraft,
    issues: [],
    summary: { valid: 1 },
    ...over,
  };
}

test('H4: a clean high-confidence parse → ENRICHED with a draft', () => {
  const draft: CanonicalMenuDraft = {
    ...emptyDraft,
    categories: [{ externalKey: 'c1', name: 'Pizza' }],
    products: [{ externalKey: 'p1', categoryKey: 'c1', name: 'Margherita', price: 850 }],
  };
  const d = classifyExtraction(result({ draft, summary: { valid: 1 } }));
  assert.equal(d.verdict, 'ENRICHED');
  assert.equal(d.draft?.categories?.[0]?.name, 'Pizza');
  assert.equal(d.draft?.categories?.[0]?.products?.[0]?.price, 850);
});

test('H4: 0 valid items → MANUAL_REVIEW (nothing to write, never fabricate)', () => {
  const d = classifyExtraction(result({ summary: { valid: 0 } }));
  assert.equal(d.verdict, 'MANUAL_REVIEW');
  assert.ok(d.reason && d.reason.length > 0, 'reason required');
  assert.equal(d.draft, undefined);
});

test('H4: an error-severity issue → LOW_QUALITY (not a silent write)', () => {
  const d = classifyExtraction(result({ issues: [{ code: 'PARSE_ERROR', message: 'bad row', severity: 'error' }] }));
  assert.equal(d.verdict, 'LOW_QUALITY');
  assert.match(d.reason ?? '', /error/);
});

test('H4: low-confidence items → LOW_QUALITY (no human in the loop pre-claim)', () => {
  const d = classifyExtraction(result({ summary: { valid: 3, low_confidence_count: 2 } }));
  assert.equal(d.verdict, 'LOW_QUALITY');
  assert.match(d.reason ?? '', /low-confidence/);
});

test('H4: the C1 PII_DENSE fail-closed → MANUAL_REVIEW', () => {
  const d = classifyExtraction(result({ issues: [{ code: 'PII_DENSE' as any, message: 'names present', severity: 'error' }] }));
  assert.equal(d.verdict, 'MANUAL_REVIEW');
  assert.match(d.reason ?? '', /PII/);
});

test('toMenuDraft: groups products by category, orphan → Menu bucket, empty categories dropped', () => {
  const canonical: CanonicalMenuDraft = {
    ...emptyDraft,
    categories: [{ externalKey: 'c1', name: 'Pizza' }, { externalKey: 'c2', name: 'Empty' }],
    products: [
      { externalKey: 'p1', categoryKey: 'c1', name: 'Margherita', price: 850, attributesJson: { bom: [{ ingredient: 'cheese', allergens: ['milk'] }] } },
      { externalKey: 'p2', categoryKey: 'unknown', name: 'Mystery', price: 100 },
    ],
  };
  const draft = toMenuDraft(canonical);
  const names = draft.categories!.map((c) => c.name);
  assert.ok(names.includes('Pizza'), 'Pizza kept');
  assert.ok(!names.includes('Empty'), 'empty category dropped');
  assert.ok(names.includes('Menu'), 'orphan product bucketed, not dropped');
  // attributes carried through (allergens stripped later at write, not here)
  const pizza = draft.categories!.find((c) => c.name === 'Pizza');
  assert.equal((pizza!.products![0].attributes as any).bom[0].ingredient, 'cheese');
});
