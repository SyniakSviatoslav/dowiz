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
});
