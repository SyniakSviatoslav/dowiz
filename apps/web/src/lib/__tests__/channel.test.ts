import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { CHANNEL_ALLOWLIST, DEFAULT_CHANNEL, normalizeChannel, resolveCapturedChannel } from '../channel.js';

// Guardrail for QR/ATTRIBUTION channel capture + propagation. Covers the two PURE
// functions (allowlist validation + ?ch= resolution); captureChannel/getOrderChannel are
// thin sessionStorage side-effects exercised via these same normalizeChannel semantics
// (there is no DOM/jsdom in this test runner — see apps/web/package.json's `test` script).

describe('CHANNEL_ALLOWLIST', () => {
  it('is exactly the 13 canonical values, with web-direct as the default', () => {
    assert.deepEqual([...CHANNEL_ALLOWLIST], [
      'web-direct', 'qr', 'nfc', 'gbp', 'apple-maps', 'instagram', 'facebook',
      'whatsapp', 'telegram-tma', 'kiosk', 'widget', 'agent', 'other',
    ]);
    assert.equal(DEFAULT_CHANNEL, 'web-direct');
  });
});

describe('normalizeChannel — allowlist validation', () => {
  it('missing/empty -> web-direct (direct/organic visit)', () => {
    assert.equal(normalizeChannel(undefined), 'web-direct');
    assert.equal(normalizeChannel(null), 'web-direct');
    assert.equal(normalizeChannel(''), 'web-direct');
    assert.equal(normalizeChannel('   '), 'web-direct');
  });

  it('every allowlisted value round-trips (case-insensitive, trimmed)', () => {
    for (const c of CHANNEL_ALLOWLIST) {
      assert.equal(normalizeChannel(c), c);
      assert.equal(normalizeChannel(c.toUpperCase()), c);
      assert.equal(normalizeChannel(` ${c} `), c);
    }
  });

  it('unrecognized value -> other', () => {
    assert.equal(normalizeChannel('tiktok'), 'other');
    assert.equal(normalizeChannel('QR2'), 'other');
    assert.equal(normalizeChannel('<script>'), 'other');
  });
});

describe('resolveCapturedChannel — capture-propagate logic (?ch= -> stored value)', () => {
  it('no ?ch= param -> null (leave any prior capture alone, do not overwrite with a default)', () => {
    assert.equal(resolveCapturedChannel(''), null);
    assert.equal(resolveCapturedChannel('?embed=true'), null);
  });

  it('?ch=<allowlisted value> -> that value', () => {
    assert.equal(resolveCapturedChannel('?ch=qr'), 'qr');
    assert.equal(resolveCapturedChannel('?ch=NFC'), 'nfc');
    assert.equal(resolveCapturedChannel('?ch=telegram-tma&foo=bar'), 'telegram-tma');
  });

  it('?ch=<unknown value> -> other (never silently dropped, never blocks capture)', () => {
    assert.equal(resolveCapturedChannel('?ch=billboard'), 'other');
  });

  it('?ch= present but empty -> null (URLSearchParams sees an empty string, not "missing")', () => {
    assert.equal(resolveCapturedChannel('?ch='), null);
  });
});
