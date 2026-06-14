import crypto from 'node:crypto';

const GLOBAL_PII_KEY_VAR = 'DOWIZ_PII_ENCRYPTION_KEY_BUFFER';

function getKey(): Buffer {
  const cached = (globalThis as any)[GLOBAL_PII_KEY_VAR];
  if (cached && Buffer.isBuffer(cached) && cached.length === 32) return cached;
  
  const envKey = process.env.COURIER_PII_ENCRYPTION_KEY;
  if (!envKey) {
    throw new Error('Missing COURIER_PII_ENCRYPTION_KEY environment variable. Required for PII encryption.');
  }
  const keyBuf = Buffer.from(envKey, 'base64');
  if (keyBuf.length !== 32) {
    throw new Error('COURIER_PII_ENCRYPTION_KEY must be exactly 32 bytes when decoded from base64. Got: ' + keyBuf.length + ' bytes');
  }
  (globalThis as any)[GLOBAL_PII_KEY_VAR] = keyBuf;
  return keyBuf;
}

const ALGORITHM = 'aes-256-gcm';
const IV_LENGTH = 12;
const AUTH_TAG_LENGTH = 16;

// Base64 alphabet for detecting legacy (base64-string-in-bytea) format
const BASE64_REGEX = /^[A-Za-z0-9+/=]+$/;

/**
 * Detects whether a Buffer is a legacy base64-encoded string stored in a bytea column.
 * Old format: encryptPII returned base64 string → inserted into bytea → pg returns Buffer of ASCII chars
 * New format: encryptPII returns raw binary Buffer → inserted into bytea → pg returns Buffer of binary data
 */
function isLegacyBase64(buf: Buffer): boolean {
  const asString = buf.toString('utf8');
  return BASE64_REGEX.test(asString);
}

/**
 * Encrypts plaintext into a raw binary Buffer: [IV (12)] + [Ciphertext] + [AuthTag (16)]
 * Stored directly in PostgreSQL bytea columns.
 */
export function encryptPII(plaintext: string): Buffer {
  if (!plaintext) return Buffer.alloc(0);
  
  const key = getKey();
  const iv = crypto.randomBytes(IV_LENGTH);
  const cipher = crypto.createCipheriv(ALGORITHM, key, iv);
  
  let encrypted = cipher.update(plaintext, 'utf8');
  encrypted = Buffer.concat([encrypted, cipher.final()]);
  const authTag = cipher.getAuthTag();
  
  return Buffer.concat([iv, encrypted, authTag]);
}

/**
 * Decrypts bytea/Buffer or base64 string back into plaintext.
 * Handles both:
 * - New format: raw binary Buffer (IV + ciphertext + authTag)
 * - Legacy format: base64 string stored in bytea → pg returns Buffer of ASCII chars
 * - String input: base64 encoded (for non-pg callers)
 */
export function decryptPII(ciphertext: Buffer | string | null): string | null {
  if (!ciphertext) return null;
  
  const key = getKey();
  let ciphertextBuf: Buffer;
  
  if (typeof ciphertext === 'string') {
    ciphertextBuf = Buffer.from(ciphertext, 'base64');
  } else {
    // Buffer from pg (bytea column)
    // Check for legacy format: base64 string stored as raw ASCII bytes in bytea
    if (isLegacyBase64(ciphertext)) {
      ciphertextBuf = Buffer.from(ciphertext.toString('utf8'), 'base64');
    } else {
      ciphertextBuf = ciphertext;
    }
  }
  
  if (ciphertextBuf.length < IV_LENGTH + AUTH_TAG_LENGTH) {
    throw new Error('Invalid ciphertext length: ' + ciphertextBuf.length);
  }
  
  const iv = ciphertextBuf.subarray(0, IV_LENGTH);
  const authTag = ciphertextBuf.subarray(ciphertextBuf.length - AUTH_TAG_LENGTH);
  const encrypted = ciphertextBuf.subarray(IV_LENGTH, ciphertextBuf.length - AUTH_TAG_LENGTH);
  
  const decipher = crypto.createDecipheriv(ALGORITHM, key, iv);
  decipher.setAuthTag(authTag);
  
  const decrypted = Buffer.concat([decipher.update(encrypted), decipher.final()]);
  return decrypted.toString('utf8');
}