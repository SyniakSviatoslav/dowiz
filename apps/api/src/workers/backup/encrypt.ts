import crypto from 'node:crypto';
import { Transform } from 'node:stream';

export interface EncryptionMeta {
  iv: string; // base64
  algorithm: 'aes-256-gcm';
  keyId: string;
  authTag?: string; // base64; populated after the cipher stream ends (getAuthTag())
}

export function createEncryptionStream(keyBase64: string): { stream: Transform; meta: EncryptionMeta; getAuthTag: () => string } {
  if (!keyBase64) throw new Error('Encryption key is required');
  
  const key = Buffer.from(keyBase64, 'base64');
  if (key.length !== 32) throw new Error('Encryption key must be exactly 32 bytes for AES-256-GCM');

  const iv = crypto.randomBytes(12);
  const cipher = crypto.createCipheriv('aes-256-gcm', key, iv);
  
  return {
    stream: cipher,
    meta: {
      iv: iv.toString('base64'),
      algorithm: 'aes-256-gcm',
      keyId: 'primary'
    },
    getAuthTag: () => {
      // Must be called AFTER the stream has ended
      return cipher.getAuthTag().toString('base64');
    }
  };
}

/**
 * LC7 fix 7 — keyId → keyring lookup, FAIL LOUD on an unknown keyId.
 *
 * A restore reads the keyId from the R2 manifest (manifest.encryption.keyId) and resolves it to
 * its base64 AES-256 key here. Key rotation is supported via the optional BACKUP_KEYRING env — a
 * JSON map `{ "<keyId>": "<base64key>" }`. The default 'primary' keyId (what the writer stamps)
 * falls back to the single BACKUP_ENCRYPTION_KEY env. An UNKNOWN keyId is fatal: we refuse to
 * "restore" with the wrong/only key and silently produce garbage. Reads process.env directly
 * (not the Zod schema) so the operator-gated secret VALUE can land later without a code change.
 */
export function resolveBackupKey(keyId: string): string {
  const keyring: Record<string, string> = {};

  const raw = process.env.BACKUP_KEYRING;
  if (raw) {
    let parsed: unknown;
    try {
      parsed = JSON.parse(raw);
    } catch (err) {
      throw new Error(`BACKUP_KEYRING is not valid JSON: ${(err as Error).message}`);
    }
    if (parsed && typeof parsed === 'object') {
      for (const [k, v] of Object.entries(parsed as Record<string, unknown>)) {
        if (typeof v === 'string') keyring[k] = v;
      }
    }
  }

  const primary = process.env.BACKUP_ENCRYPTION_KEY;
  if (primary && keyring.primary === undefined) keyring.primary = primary;

  const key = keyring[keyId];
  if (!key) {
    throw new Error(
      `Unknown backup keyId '${keyId}': not present in BACKUP_KEYRING or BACKUP_ENCRYPTION_KEY. Refusing to restore with an unverified key.`,
    );
  }
  return key;
}

export function createDecryptionStream(keyBase64: string, ivBase64: string, authTagBase64: string): Transform {
  const key = Buffer.from(keyBase64, 'base64');
  const iv = Buffer.from(ivBase64, 'base64');
  const authTag = Buffer.from(authTagBase64, 'base64');

  const decipher = crypto.createDecipheriv('aes-256-gcm', key, iv);
  decipher.setAuthTag(authTag);
  return decipher;
}
