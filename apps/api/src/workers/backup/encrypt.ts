import crypto from 'node:crypto';
import { Transform } from 'node:stream';

export interface EncryptionMeta {
  iv: string; // base64
  algorithm: 'aes-256-gcm';
  keyId: string;
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

export function createDecryptionStream(keyBase64: string, ivBase64: string, authTagBase64: string): Transform {
  const key = Buffer.from(keyBase64, 'base64');
  const iv = Buffer.from(ivBase64, 'base64');
  const authTag = Buffer.from(authTagBase64, 'base64');

  const decipher = crypto.createDecipheriv('aes-256-gcm', key, iv);
  decipher.setAuthTag(authTag);
  return decipher;
}
