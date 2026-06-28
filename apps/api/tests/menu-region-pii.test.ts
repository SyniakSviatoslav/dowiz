import { test } from 'node:test';
import assert from 'node:assert/strict';
import { extractMenuRegion, scanResidualPii } from '../src/lib/menu-region.js';
import { PiiRedactor } from '../src/lib/pii-redactor.js';

// P6-3 (council C1) — REDACTION-RECALL GUARDRAIL for the scrape→AI path. The binding invariant
// (ADR-0011 / operator decision #4, NOT waived): no third-party person-PII reaches the external LLM.
// STATED RECALL FLOOR (this test fails red below it):
//   • emails + phones in the menu region → 100% redacted by PiiRedactor.
//   • anchored person-names ("Chef Maria Hoxha", "Owner: Jeton") → 100% redacted.
//   • a page whose About/Team/Reviews/footer carry names → those regions are DROPPED by the
//     allowlist, OR the residual is flagged PII-dense (fail-closed) — never silently sent.

const redactor = new PiiRedactor();

const SCRAPED_PAGE = `
<html><head><title>Trattoria</title></head>
<body>
  <header><a href="/">Home</a> <span>+355 69 123 4567</span></header>
  <h1>Menu</h1>
  <div class="item">Margherita Pizza — 850</div>
  <div class="item">Caesar Salad — 600</div>
  <h2>About Us</h2>
  <p>Meet our chef Maria Hoxha, who trained in Napoli. Owner: Jeton Berisha.</p>
  <h2>Reviews</h2>
  <blockquote>"Best burek in Tirana, the staff are lovely!" — Arben K.</blockquote>
  <footer>Contact: info@trattoria.al · +355 4 222 3333</footer>
</body></html>`;

test('C1: extractMenuRegion keeps menu items, drops About/Reviews/footer (where names live)', () => {
  const region = extractMenuRegion(SCRAPED_PAGE);
  // menu survives
  assert.match(region, /Margherita Pizza/, 'menu item kept');
  assert.match(region, /Caesar Salad/, 'menu item kept');
  // non-menu name/PII regions dropped
  assert.doesNotMatch(region, /Maria Hoxha/, 'chef bio dropped by allowlist');
  assert.doesNotMatch(region, /Jeton Berisha/, 'owner name dropped by allowlist');
  assert.doesNotMatch(region, /Arben/, 'testimonial attribution dropped');
  assert.doesNotMatch(region, /info@trattoria\.al/, 'footer email dropped (footer element removed)');
});

test('C1: PiiRedactor redacts anchored names + emails + phones (recall floor)', () => {
  const cases = [
    'Meet our chef Maria Hoxha today',
    'Owner: Jeton Berisha',
    'Prepared by Arben K.',
    'Reach us at info@trattoria.al',
    'Call +355 69 123 4567 to book',
  ];
  for (const c of cases) {
    const { text } = redactor.redact(c);
    assert.match(text, /\[REDACTED\]/, `must redact PII in: ${c}`);
  }
  assert.doesNotMatch(redactor.redact('Meet our chef Maria Hoxha').text, /Maria Hoxha/);
  assert.doesNotMatch(redactor.redact('Owner: Jeton Berisha').text, /Jeton Berisha/);
});

test('C1: a menu item name (TitleCase, no trigger) is NOT redacted (no false positives)', () => {
  const { text } = redactor.redact('Caesar Salad 600\nMargherita Pizza 850');
  assert.match(text, /Caesar Salad/, 'menu item not a false-positive name');
  assert.match(text, /Margherita Pizza/);
});

test('C1: scanResidualPii FAILS CLOSED on residual testimonial/bio text', () => {
  const leaky = 'Margherita 850\n"The food here is absolutely incredible and fresh" — Arben K.';
  const scan = scanResidualPii(leaky);
  assert.equal(scan.dense, true, 'residual testimonial → fail closed');
  assert.ok(scan.score >= 1);
  // clean menu text is not flagged
  const clean = scanResidualPii('Margherita Pizza 850\nCaesar Salad 600\nTiramisu 400');
  assert.equal(clean.dense, false, 'clean menu text passes');
});

test('C1: end-to-end — region + redact leaves NO seeded person-PII (the binding)', () => {
  const region = extractMenuRegion(SCRAPED_PAGE);
  const { text } = redactor.redact(region);
  for (const pii of ['Maria Hoxha', 'Jeton Berisha', 'Arben', 'info@trattoria.al', '69 123 4567']) {
    assert.doesNotMatch(text, new RegExp(pii.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')), `seeded PII leaked: ${pii}`);
  }
});
