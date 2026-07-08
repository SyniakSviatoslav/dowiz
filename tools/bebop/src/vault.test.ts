// Bebop vault tests — encrypted-at-rest identity, fail-closed. RED+GREEN.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { createIdentity } from './crypto.ts';
import { lock, unlock, saveVault, loadBlob, createOrUnlock } from './vault.ts';

function tmp(): string {
  return path.join(os.tmpdir(), `bebop-vault-${Math.random().toString(36).slice(2)}.json`);
}

// ── GREEN: round-trip ──

test('GREEN: lock → unlock round-trips the identity and verifies the self-certifying id', () => {
  const id = createIdentity();
  const blob = lock(id, 'correct horse battery staple');
  const back = unlock(blob, 'correct horse battery staple');
  assert.equal(back.id, id.id);
  assert.deepEqual([...back.pqPublic], [...id.pqPublic]);
  assert.deepEqual([...back.pqSecret], [...id.pqSecret]);
  assert.deepEqual([...back.edSecret], [...id.edSecret]);
});

// ── RED: wrong passphrase fails closed ──

test('RED: unlock with the WRONG passphrase throws (no silent decrypt)', () => {
  const id = createIdentity();
  const blob = lock(id, 'right-passphrase');
  assert.throws(() => unlock(blob, 'wrong-passphrase'), /vault: decryption failed/);
});

// ── RED: tampered blob fails closed ──

test('RED: a tampered ciphertext fails authentication (AEAD tag)', () => {
  const id = createIdentity();
  const blob = lock(id, 'pass');
  const ct = hexToBytesSafe(blob.ct);
  ct[0] ^= 0xff; // flip a bit
  const tampered = { ...blob, ct: bytesToHexSafe(ct) };
  assert.throws(() => unlock(tampered, 'pass'), /vault: decryption failed|id mismatch/);
});

// ── GREEN: file persistence + node boot ──

test('GREEN: createOrUnlock boots a node from disk and keeps the SAME identity across restarts', () => {
  const file = tmp();
  const a = createOrUnlock(file, 'node-pass');
  const b = createOrUnlock(file, 'node-pass'); // reload from vault
  assert.equal(b.id, a.id, 'identity persists across restart');
  fs.unlinkSync(file);
});

test('RED: createOrUnlock with wrong passphrase on an existing vault throws (no override)', () => {
  const file = tmp();
  createOrUnlock(file, 'good-pass');
  assert.throws(() => createOrUnlock(file, 'bad-pass'), /vault: decryption failed/);
  fs.unlinkSync(file);
});

// tiny hex helpers (avoid importing from the module-under-test internals)
function hexToBytesSafe(h: string): Uint8Array {
  const out = new Uint8Array(h.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(h.slice(i * 2, i * 2 + 2), 16);
  return out;
}
function bytesToHexSafe(b: Uint8Array): string {
  return [...b].map((x) => x.toString(16).padStart(2, '0')).join('');
}
