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
});

test('PiiRedactor - IBAN', () => {
  const redactor = new PiiRedactor();
  const input = 'My IBAN is AL28020011111234567890123456 please pay';
  const res = redactor.redact(input);

  assert.strictEqual(res.redactions.length, 1);
  assert.strictEqual(res.redactions[0].kind, 'iban');
  assert.ok(!res.text.includes('AL28020011111234567890123456'));
});

test('PiiRedactor - ignores prices', () => {
  const redactor = new PiiRedactor();
  const input = 'Pizza +355 ALL is 1000';
  const res = redactor.redact(input);

  assert.strictEqual(res.redactions.length, 0); // Too short to be phone
  assert.strictEqual(res.text, input);
});
