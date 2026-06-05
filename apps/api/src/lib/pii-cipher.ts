import crypto from 'node:crypto';

// The key must be exactly 32 bytes (256 bits) for AES-256-GCM.
// We expect it to be provided as a base64 string in env.
let ENCRYPTION_KEY: Buffer | null = null;

function getKey(): Buffer {
  if (ENCRYPTION_KEY) return ENCRYPTION_KEY;
  const envKey = process.env.COURIER_PII_ENCRYPTION_KEY;
  if (!envKey) {
    throw new Error('Missing COURIER_PII_ENCRYPTION_KEY environment variable. Required for PII encryption.');
  }
  const keyBuf = Buffer.from(envKey, 'base64');
  if (keyBuf.length !== 32) {
    throw new Error('COURIER_PII_ENCRYPTION_KEY must be exactly 32 bytes when decoded from base64.');
  }
  ENCRYPTION_KEY = keyBuf;
  return ENCRYPTION_KEY;
}

const ALGORITHM = 'aes-256-gcm';
const IV_LENGTH = 12; // 96 bits is recommended for GCM
const AUTH_TAG_LENGTH = 16;

/**
 * Encrypts a plaintext string into a Buffer containing: [IV (12 bytes)] + [Ciphertext] + [Auth Tag (16 bytes)]
 */
export function encryptPII(plaintext: string): string {
  if (!plaintext) return '';
  
  const key = getKey();
  const iv = crypto.randomBytes(IV_LENGTH);
  const cipher = crypto.createCipheriv(ALGORITHM, key, iv);
  
  let encrypted = cipher.update(plaintext, 'utf8');
  encrypted = Buffer.concat([encrypted, cipher.final()]);
  const authTag = cipher.getAuthTag();
  
  // Format: IV + Ciphertext + AuthTag
  return Buffer.concat([iv, encrypted, authTag]).toString('base64');
}

/**
 * Decrypts a Buffer containing [IV] + [Ciphertext] + [Auth Tag] back into plaintext.
 */
export function decryptPII(ciphertextBase64: string | null): string | null {
  if (!ciphertextBase64) return null;
  
  const key = getKey();
  const ciphertextBuf = Buffer.from(ciphertextBase64, 'base64');
  
  if (ciphertextBuf.length < IV_LENGTH + AUTH_TAG_LENGTH) {
    throw new Error('Invalid ciphertext length');
  }
  
  const iv = ciphertextBuf.subarray(0, IV_LENGTH);
  const authTag = ciphertextBuf.subarray(ciphertextBuf.length - AUTH_TAG_LENGTH);
  const encrypted = ciphertextBuf.subarray(IV_LENGTH, ciphertextBuf.length - AUTH_TAG_LENGTH);
  
  const decipher = crypto.createDecipheriv(ALGORITHM, key, iv);
  decipher.setAuthTag(authTag);
  
  const decrypted = Buffer.concat([decipher.update(encrypted), decipher.final()]);
  return decrypted.toString('utf8');
}
