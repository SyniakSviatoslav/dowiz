import { test } from 'node:test';
import assert from 'node:assert/strict';
import { validateImageKey } from '../src/lib/image-key.js';

// Guardrail for the storefront image-key validator. The data:/blob: rejection is
// the security-relevant arm — it keeps inline image payloads out of brand/logo
// columns and forces the upload pipeline.

test('validateImageKey — null/undefined pass through (cleared field)', () => {
  assert.equal(validateImageKey(undefined), undefined);
  assert.equal(validateImageKey(null), null);
});

test('validateImageKey — a plain object-storage key is returned as-is', () => {
  assert.equal(validateImageKey('brands/loc-1/logo.png'), 'brands/loc-1/logo.png');
});

test('validateImageKey — data: URL is rejected', () => {
  assert.throws(() => validateImageKey('data:image/png;base64,iVBORw0KGgo='), /upload endpoint/);
});

test('validateImageKey — blob: URL is rejected', () => {
  assert.throws(() => validateImageKey('blob:https://dowiz.app/abc-123'), /upload endpoint/);
});

test('validateImageKey — non-string is coerced to string', () => {
  assert.equal(validateImageKey(123), '123');
});
