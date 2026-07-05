import { test } from 'node:test';
import assert from 'node:assert/strict';
import { CHANNEL_ALLOWLIST, DEFAULT_CHANNEL, normalizeChannel, channelFromHeader } from '../src/lib/channel.js';

// Guardrail for the QR/ATTRIBUTION `x-channel` header validation. A malformed or
// unexpected header value must NEVER throw / block order creation — it must always
// degrade to a safe allowlisted value ('web-direct' when absent, 'other' otherwise).

test('CHANNEL_ALLOWLIST — exactly the 13 canonical values', () => {
  assert.deepEqual([...CHANNEL_ALLOWLIST], [
    'web-direct', 'qr', 'nfc', 'gbp', 'apple-maps', 'instagram', 'facebook',
    'whatsapp', 'telegram-tma', 'kiosk', 'widget', 'agent', 'other',
  ]);
  assert.equal(DEFAULT_CHANNEL, 'web-direct');
});

test('normalizeChannel — missing/empty header defaults to web-direct', () => {
  assert.equal(normalizeChannel(undefined), 'web-direct');
  assert.equal(normalizeChannel(null), 'web-direct');
  assert.equal(normalizeChannel(''), 'web-direct');
  assert.equal(normalizeChannel('   '), 'web-direct');
});

test('normalizeChannel — every allowlisted value round-trips (case-insensitive, trimmed)', () => {
  for (const c of CHANNEL_ALLOWLIST) {
    assert.equal(normalizeChannel(c), c);
    assert.equal(normalizeChannel(c.toUpperCase()), c);
    assert.equal(normalizeChannel(`  ${c}  `), c);
  }
});

test('normalizeChannel — unrecognized value normalizes to other', () => {
  assert.equal(normalizeChannel('billboard'), 'other');
  assert.equal(normalizeChannel('QR-code-v2'), 'other');
  assert.equal(normalizeChannel('<script>alert(1)</script>'), 'other');
});

test('normalizeChannel — non-string / over-length input normalizes to other, never throws', () => {
  assert.equal(normalizeChannel(['qr', 'nfc'] as any), 'other');
  assert.equal(normalizeChannel(42 as any), 'other');
  assert.equal(normalizeChannel({} as any), 'other');
  assert.equal(normalizeChannel('q'.repeat(1000)), 'other');
});

test('channelFromHeader — Fastify string | string[] | undefined header shapes', () => {
  assert.equal(channelFromHeader(undefined), 'web-direct');
  assert.equal(channelFromHeader('qr'), 'qr');
  assert.equal(channelFromHeader(['nfc', 'qr']), 'nfc'); // duplicated header -> first wins
  assert.equal(channelFromHeader(['bogus']), 'other');
});
