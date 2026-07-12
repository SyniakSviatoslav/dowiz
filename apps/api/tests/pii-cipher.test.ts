import test from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { encryptPII, decryptPII } from '../src/lib/pii-cipher.js';

// Must match GLOBAL_PII_KEY_VAR in src/lib/pii-cipher.ts — the cached key buffer.
const KEY_CACHE_VAR = 'DOWIZ_PII_ENCRYPTION_KEY_BUFFER';

test('PII Cipher Helper', async (t) => {
  // Setup env
  process.env.COURIER_PII_ENCRYPTION_KEY = crypto.randomBytes(32).toString('base64');

  await t.test('roundtrips plaintext correctly', () => {
    const plain = 'test@example.com';
    const cipher = encryptPII(plain);
    assert.notEqual(cipher.toString('utf8'), plain); // Should not contain plaintext
    const decrypted = decryptPII(cipher);
    assert.equal(decrypted, plain);
  });

  await t.test('throws on tampered ciphertext body', () => {
    const cipher = encryptPII('secret-data');

    // Tamper with ciphertext body (index 15 is past the 12-byte IV).
    cipher[15] = cipher[15] ^ 1;

    assert.throws(() => {
      decryptPII(cipher);
    }, { message: 'Unsupported state or unable to authenticate data' });
  });

  await t.test('throws on tampered auth-tag region', () => {
    const cipher = encryptPII('secret-data');

    // Last byte lives in the 16-byte GCM auth tag.
    cipher[cipher.length - 1] = cipher[cipher.length - 1] ^ 1;

    assert.throws(() => {
      decryptPII(cipher);
    }, { message: 'Unsupported state or unable to authenticate data' });
  });

  await t.test('throws on tampered IV', () => {
    const cipher = encryptPII('secret-data');

    // First byte lives in the 12-byte IV prefix.
    cipher[0] = cipher[0] ^ 1;

    assert.throws(() => {
      decryptPII(cipher);
    }, { message: 'Unsupported state or unable to authenticate data' });
  });

  await t.test('handles empty string (null-sentinel contract)', () => {
    // encryptPII('') short-circuits to an empty buffer; decryptPII treats an
    // empty/absent buffer as "no value" and returns null (string | null). All
    // call sites coalesce that null (`|| ''` / `|| 'Unknown'`), so empty/absent
    // PII reads back as null, not ''.
    const cipher = encryptPII('');
    assert.equal(cipher.length, 0);
    const decrypted = decryptPII(cipher);
    assert.equal(decrypted, null);
  });

  await t.test('different IVs yield different ciphertexts', () => {
    const plain = 'same-text';
    const c1 = encryptPII(plain);
    const c2 = encryptPII(plain);
    assert.notDeepEqual(c1, c2);
  });

  await t.test('decrypts legacy base64-in-bytea format', () => {
    // Old format: encryptPII returned a base64 STRING inserted into a bytea
    // column, so pg hands back a Buffer of ASCII base64 chars. decryptPII must
    // detect and re-decode that legacy shape.
    const plain = 'legacy@example.com';
    const binary = encryptPII(plain);
    const legacyBytea = Buffer.from(binary.toString('base64'), 'ascii');
    assert.equal(decryptPII(legacyBytea), plain);
  });

  await t.test('throws when COURIER_PII_ENCRYPTION_KEY is missing', () => {
    const savedEnv = process.env.COURIER_PII_ENCRYPTION_KEY;
    const savedCache = (globalThis as any)[KEY_CACHE_VAR];
    delete process.env.COURIER_PII_ENCRYPTION_KEY;
    delete (globalThis as any)[KEY_CACHE_VAR];
    try {
      assert.throws(() => encryptPII('x'), {
        message: 'Missing COURIER_PII_ENCRYPTION_KEY environment variable. Required for PII encryption.',
      });
    } finally {
      process.env.COURIER_PII_ENCRYPTION_KEY = savedEnv;
      (globalThis as any)[KEY_CACHE_VAR] = savedCache;
    }
  });

  await t.test('throws when key is not exactly 32 bytes', () => {
    const savedEnv = process.env.COURIER_PII_ENCRYPTION_KEY;
    const savedCache = (globalThis as any)[KEY_CACHE_VAR];
    process.env.COURIER_PII_ENCRYPTION_KEY = crypto.randomBytes(16).toString('base64');
    delete (globalThis as any)[KEY_CACHE_VAR];
    try {
      assert.throws(() => encryptPII('x'), {
        message: 'COURIER_PII_ENCRYPTION_KEY must be exactly 32 bytes when decoded from base64. Got: 16 bytes',
      });
    } finally {
      process.env.COURIER_PII_ENCRYPTION_KEY = savedEnv;
      (globalThis as any)[KEY_CACHE_VAR] = savedCache;
    }
  });
});
