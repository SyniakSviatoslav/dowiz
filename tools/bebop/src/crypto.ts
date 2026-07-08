// Bebop crypto — post-quantum, self-certifying identity for independent nodes (CORE principle).
//
// OSS-only, zero external services, WASM-ready: @noble/post-quantum (ML-KEM / ML-DSA), @noble/hashes
// (sha256), @noble/curves (ed25519). This is the auth primitive for a MESH of autonomous nodes: there
// is no central session server. A node's identity IS its PQ keypair; its node-id is the hash of its
// public key (self-certifying namespace, IPNS/IPS-style). Auth = signature over content.
//
// Hybrid by default (NIST + operator guidance): classical Ed25519 runs alongside ML-DSA so today's
// classical attacks are covered AND tomorrow's quantum threat is mitigated. Secrets never leave the
// local vault (see vault.ts, Phase 2) and are never transmitted.

import { sha256 } from '@noble/hashes/sha2.js';
import { ml_kem768 } from '@noble/post-quantum/ml-kem.js';
import { ml_dsa65 } from '@noble/post-quantum/ml-dsa.js';
import { ed25519 } from '@noble/curves/ed25519.js';

export function sha256hex(data: string | Uint8Array): string {
  const bytes = typeof data === 'string' ? new TextEncoder().encode(data) : data;
  return Buffer.from(sha256(bytes)).toString('hex');
}

// ── Self-certifying node identity ──

export interface NodeIdentity {
  pqPublic: Uint8Array; // ML-DSA-65 public key
  pqSecret: Uint8Array; // ML-DSA-65 secret key (NEVER transmitted)
  edPublic: Uint8Array; // Ed25519 public key (classical hybrid)
  edSecret: Uint8Array; // Ed25519 secret key (NEVER transmitted)
  id: string; // self-certifying node id = hash(pqPublic || edPublic)
}

/** Create an identity. If `seed` is given it drives Ed25519 deterministically; PQ keys are random. */
export function createIdentity(seed?: Uint8Array): NodeIdentity {
  const edSecret = seed ? seed.slice(0, 32) : ed25519.utils.randomSecretKey();
  const edPublic = ed25519.getPublicKey(edSecret);
  const { publicKey: pqPublic, secretKey: pqSecret } = ml_dsa65.keygen();
  const id = nodeIdFromPublic(pqPublic, edPublic);
  return { pqPublic, pqSecret, edPublic, edSecret, id };
}

/** Derive a node id from public keys alone (anyone can compute it; it needs no secret). */
export function nodeIdFromPublic(pqPublic: Uint8Array, edPublic: Uint8Array): string {
  return sha256hex(Buffer.concat([Buffer.from(pqPublic), Buffer.from(edPublic)]));
}

/** Reconstruct the public keys from the secret keys (a node can recover its public identity). */
export function publicFromSecret(pqSecret: Uint8Array, edSecret: Uint8Array): { pqPublic: Uint8Array; edPublic: Uint8Array } {
  return { pqPublic: ml_dsa65.getPublicKey(pqSecret), edPublic: ed25519.getPublicKey(edSecret) };
}

// ── Sign / verify (auth = signature over content; hybrid fail-closed) ──

export interface Signature {
  pq: Uint8Array;
  ed: Uint8Array;
}

export function sign(identity: NodeIdentity, message: Uint8Array): Signature {
  // noble API order: sign(message, secretKey)
  return {
    pq: ml_dsa65.sign(message, identity.pqSecret),
    ed: ed25519.sign(message, identity.edSecret),
  };
}

export function verify(
  pqPublic: Uint8Array,
  edPublic: Uint8Array,
  message: Uint8Array,
  sig: Signature,
): boolean {
  try {
    const edOk = ed25519.verify(sig.ed, message, edPublic);
    const pqOk = ml_dsa65.verify(sig.pq, message, pqPublic);
    return edOk && pqOk; // both must validate
  } catch {
    return false;
  }
}

// ── KEM (post-quantum key exchange) for private node-to-node channels ──
// Uses ML-KEM-768 (FIPS 203). Distinct keypair from the ML-DSA signing identity.

export interface KemKeys {
  publicKey: Uint8Array; // 1184 bytes
  secretKey: Uint8Array; // 2400 bytes
}

export function createKemKeys(): KemKeys {
  return ml_kem768.keygen();
}

export function kemEncapsulate(pqKemPublic: Uint8Array): { cipherText: Uint8Array; sharedSecret: Uint8Array } {
  return ml_kem768.encapsulate(pqKemPublic);
}

export function kemDecapsulate(pqKemSecret: Uint8Array, cipherText: Uint8Array): Uint8Array {
  // noble API order: decapsulate(cipherText, secretKey)
  return ml_kem768.decapsulate(cipherText, pqKemSecret);
}
