import test from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { encryptPII, decryptPII } from '../src/lib/pii-cipher.js';

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

  await t.test('throws on tampered ciphertext', () => {
    const cipher = encryptPII('secret-data');
    
    // Tamper with ciphertext (middle part)
    cipher[15] = cipher[15] ^ 1;
    
    assert.throws(() => {
      decryptPII(cipher);
    }, { message: 'Unsupported state or unable to authenticate data' });
  });

  await t.test('handles empty string', () => {
    const cipher = encryptPII('');
    assert.equal(cipher.length, 0);
    const decrypted = decryptPII(cipher);
    assert.equal(decrypted, '');
  });

  await t.test('different IVs yield different ciphertexts', () => {
    const plain = 'same-text';
    const c1 = encryptPII(plain);
    const c2 = encryptPII(plain);
    assert.notDeepEqual(c1, c2);
  });

  await t.test('null input returns null (distinct from empty string)', () => {
    assert.equal(decryptPII(null), null);
    assert.notEqual(decryptPII(null), '');
  });

  await t.test('cross-tenant: ciphertext from one key rejected by another key', () => {
    const cipher = encryptPII('tenant-a-secret');
    // Rotate to a different key (simulates tenant B having a different key)
    const originalKey = process.env.COURIER_PII_ENCRYPTION_KEY;
    process.env.COURIER_PII_ENCRYPTION_KEY = crypto.randomBytes(32).toString('base64');
    delete (globalThis as any)['DOWIZ_PII_ENCRYPTION_KEY_BUFFER'];
    assert.throws(() => decryptPII(cipher), { message: 'Unsupported state or unable to authenticate data' });
    // Restore
    process.env.COURIER_PII_ENCRYPTION_KEY = originalKey;
    delete (globalThis as any)['DOWIZ_PII_ENCRYPTION_KEY_BUFFER'];
  });
});
