import test from 'node:test';
import assert from 'node:assert';
import { PiiRedactor } from '../src/lib/pii-redactor.js';

test('PiiRedactor - basic redaction', () => {
  const redactor = new PiiRedactor();
  const input = 'Contact me at admin@deliveryos.com or call +355 69 123 4567. Address is Rruga 12. Also, here is my card 1234-5678-9012-3456 and url http://foo.com?token=123';
  const res = redactor.redact(input);

  assert.strictEqual(res.redactions.length, 4);
  assert.ok(res.text.includes('[REDACTED]'));
  assert.ok(!res.text.includes('admin@deliveryos.com'));
  assert.ok(!res.text.includes('+355 69 123 4567'));
  assert.ok(!res.text.includes('1234-5678-9012-3456'));
  assert.ok(!res.text.includes('http://foo.com?token=123'));
  assert.ok(res.text.includes('Rruga 12')); // Not redacted!

  // Machine-readable offsets: email is the first pattern + first match,
  // so its (start,end) are recorded against the original input (offsetDiff 0).
  const email = res.redactions.find((r) => r.kind === 'email');
  assert.ok(email, 'email redaction must be present');
  assert.strictEqual(email.start, input.indexOf('admin@deliveryos.com'));
  assert.strictEqual(email.end, input.indexOf('admin@deliveryos.com') + 'admin@deliveryos.com'.length);
  assert.strictEqual(email.replacement, '[REDACTED]');

  // All four expected kinds must appear, each replaced with the sentinel.
  for (const kind of ['email', 'phone', 'card', 'url'] as const) {
    const hit = res.redactions.find((r) => r.kind === kind);
    assert.ok(hit, `expected a '${kind}' redaction`);
    assert.strictEqual(hit.replacement, '[REDACTED]');
    // Offsets must be a non-empty, ordered span.
    assert.ok(hit.end > hit.start, `'${kind}' span must be non-empty`);
  }
});

test('PiiRedactor - IBAN', () => {
  const redactor = new PiiRedactor();
  const input = 'My IBAN is AL28020011111234567890123456 please pay';
  const res = redactor.redact(input);

  assert.strictEqual(res.redactions.length, 1);
  assert.strictEqual(res.redactions[0].kind, 'iban');
  assert.strictEqual(res.redactions[0].start, input.indexOf('AL28020011111234567890123456'));
  assert.strictEqual(
    res.redactions[0].end,
    input.indexOf('AL28020011111234567890123456') + 'AL28020011111234567890123456'.length
  );
  assert.strictEqual(res.redactions[0].replacement, '[REDACTED]');
  assert.ok(!res.text.includes('AL28020011111234567890123456'));
});

test('PiiRedactor - ignores prices', () => {
  const redactor = new PiiRedactor();
  const input = 'Pizza +355 ALL is 1000';
  const res = redactor.redact(input);

  assert.strictEqual(res.redactions.length, 0); // Too short to be phone
  assert.strictEqual(res.text, input);
});
