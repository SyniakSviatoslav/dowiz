// Bebop vault — encrypted-at-rest identity store (deferred item, now built).
//
// A node's PQ identity MUST survive restart without ever being transmitted. The vault encrypts the
// full NodeIdentity with XChaCha20-Poly1305 (AEAD) — key derived from a passphrase via scrypt.
// OSS, zero external services, auditable (@noble/ciphers + @noble/hashes).
//
// Secrets NEVER leave this module in cleartext. On unlock we re-derive the self-certifying node id
// from the recovered public keys and assert it matches — a tampered or wrong-key blob fails closed.

import { xchacha20poly1305 } from '@noble/ciphers/chacha.js';
import { scrypt } from '@noble/hashes/scrypt.js';
import { randomBytes, bytesToHex, hexToBytes, utf8ToBytes, bytesToUtf8 } from '@noble/ciphers/utils.js';
import fs from 'node:fs';
import type { NodeIdentity } from './crypto.ts';
import { nodeIdFromPublic, createIdentity } from './crypto.ts';

const NONCE_LEN = 24; // XChaCha20 nonce
const SALT_LEN = 16;
const DK_LEN = 32;

export interface VaultBlob {
  v: 1;
  salt: string; // hex
  nonce: string; // hex
  ct: string; // hex ciphertext+tag
}

function deriveKey(passphrase: string, salt: Uint8Array): Uint8Array {
  return scrypt(utf8ToBytes(passphrase), salt, { N: 2 ** 15, r: 8, p: 1, dkLen: DK_LEN });
}

/** Encrypt an identity under a passphrase → a portable, self-describing blob. Secrets stay local. */
export function lock(identity: NodeIdentity, passphrase: string): VaultBlob {
  const salt = randomBytes(SALT_LEN);
  const nonce = randomBytes(NONCE_LEN);
  const key = deriveKey(passphrase, salt);
  const plain = utf8ToBytes(JSON.stringify({
    pqPublic: bytesToHex(identity.pqPublic),
    pqSecret: bytesToHex(identity.pqSecret),
    edPublic: bytesToHex(identity.edPublic),
    edSecret: bytesToHex(identity.edSecret),
    id: identity.id,
  }));
  const ct = xchacha20poly1305(key, nonce).encrypt(plain);
  return { v: 1, salt: bytesToHex(salt), nonce: bytesToHex(nonce), ct: bytesToHex(ct) };
}

/** Decrypt a blob. Throws on wrong passphrase OR tampered ciphertext (AEAD fail-closed). */
export function unlock(blob: VaultBlob, passphrase: string): NodeIdentity {
  const salt = hexToBytes(blob.salt);
  const nonce = hexToBytes(blob.nonce);
  const key = deriveKey(passphrase, salt);
  let plain: Uint8Array;
  try {
    plain = xchacha20poly1305(key, nonce).decrypt(hexToBytes(blob.ct));
  } catch {
    throw new Error('vault: decryption failed (wrong passphrase or tampered blob)');
  }
  const j = JSON.parse(bytesToUtf8(plain));
  const identity: NodeIdentity = {
    pqPublic: hexToBytes(j.pqPublic),
    pqSecret: hexToBytes(j.pqSecret),
    edPublic: hexToBytes(j.edPublic),
    edSecret: hexToBytes(j.edSecret),
    id: j.id,
  };
  // self-certifying integrity check: re-derive id from recovered public keys; must match stored id
  const derived = nodeIdFromPublic(identity.pqPublic, identity.edPublic);
  if (derived !== identity.id) throw new Error('vault: identity id mismatch after unlock (tamper)');
  return identity;
}

export function saveVault(path: string, blob: VaultBlob): void {
  fs.writeFileSync(path, JSON.stringify(blob));
}

export function loadBlob(path: string): VaultBlob {
  return JSON.parse(fs.readFileSync(path, 'utf8'));
}

/**
 * Node boot path: if a vault exists at path, unlock it; else create a fresh identity, lock it, and
 * save. Returns the live NodeIdentity. Secrets are never transmitted — only ever at rest here.
 */
export function createOrUnlock(path: string, passphrase: string, seed?: Uint8Array): NodeIdentity {
  if (fs.existsSync(path)) {
    return unlock(loadBlob(path), passphrase);
  }
  // create a fresh identity, lock it, and save
  const id = createIdentity(seed);
  saveVault(path, lock(id, passphrase));
  return id;
}
