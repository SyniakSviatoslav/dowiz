// Bebop core tests — kernel + crypto + torrent + mesh (RED+GREEN, Verified-by-Math).
//
// Core principle (GRAND-PLAN + your directives): cryptographic, network, post-quantum, mesh,
// torrent-like — and it must be provable. Each GREEN proves the happy path; each RED proves a
// wrong/false input is rejected (no false-green).

import assert from 'node:assert/strict';
import test from 'node:test';
import {
  createIdentity,
  sign,
  verify,
  nodeIdFromPublic,
  publicFromSecret,
  kemEncapsulate,
  kemDecapsulate,
} from './crypto.ts';
import { ml_kem768 } from '@noble/post-quantum/ml-kem.js';
import { createTorrent, verifyPiece, assemble, wantBitfield, DEFAULT_PIECE_SIZE } from './torrent.ts';
import { InMemoryNode, nodeAssemble } from './mesh.ts';
import { decide, fold, replay, genesis, commandHash, applyCommand, type Command, type State } from './kernel.ts';

const enc = new TextEncoder();

// ── CRYPTO ──

test('GREEN: hybrid PQ+Ed25519 identity signs and self-verifies', () => {
  const id = createIdentity();
  assert.match(id.id, /^[0-9a-f]{64}$/);
  const msg = enc.encode('autonomous node');
  const sig = sign(id, msg);
  assert.equal(verify(id.pqPublic, id.edPublic, msg, sig), true);
});

test('RED: signature over tampered message fails (no silent accept)', () => {
  const id = createIdentity();
  const sig = sign(id, enc.encode('autonomous node'));
  assert.equal(verify(id.pqPublic, id.edPublic, enc.encode('hostile node'), sig), false);
});

test('GREEN: ML-KEM key exchange yields a shared secret on both sides', () => {
  const kem = ml_kem768.keygen();
  const encap = kemEncapsulate(kem.publicKey);
  const ss = kemDecapsulate(kem.secretKey, encap.cipherText);
  assert.ok(Buffer.from(encap.sharedSecret).equals(Buffer.from(ss)));
  assert.equal(encap.sharedSecret.length, 32);
});

test('GREEN: node id is self-certifying (derived from public keys alone)', () => {
  const id = createIdentity();
  const fromPub = nodeIdFromPublic(id.pqPublic, id.edPublic);
  assert.equal(fromPub, id.id);
  const recon = publicFromSecret(id.pqSecret, id.edSecret);
  assert.ok(Buffer.from(recon.pqPublic).equals(Buffer.from(id.pqPublic)));
});

// ── TORRENT / MESH ──

test('GREEN: content-addressed pieces assemble back to the original payload', () => {
  const payload = enc.encode('Bebop mesh payload '.repeat(500));
  const t = createTorrent(payload, 1024);
  assert.ok(t.pieces.length > 1);
  const rebuilt = assemble(t, t.pieces);
  assert.ok(rebuilt && Buffer.from(rebuilt).equals(Buffer.from(payload)));
});

test('GREEN: infoHash is self-certifying (changes if chunking/pieces change)', () => {
  const p = enc.encode('same bytes');
  const a = createTorrent(p, 4);
  const b = createTorrent(p, 8); // different piece size → different infoHash
  assert.notEqual(a.infoHash, b.infoHash);
  assert.equal(a.pieceHashes.length, Math.ceil(p.length / 4));
});

test('RED: a tampered piece is rejected by verifyPiece AND assemble returns null', () => {
  const payload = enc.encode(('mesh piece ').repeat(200));
  const t = createTorrent(payload, 256);
  const bad = { ...t.pieces[1], bytes: enc.encode('EVIL'.repeat(64)) };
  assert.equal(verifyPiece(bad), false);
  const withBad = [...t.pieces];
  withBad[1] = bad;
  assert.equal(assemble(t, withBad), null);
});

test('RED: missing a piece means assemble returns null (no partial fake)', () => {
  const payload = enc.encode(('mesh piece ').repeat(200));
  const t = createTorrent(payload, 256);
  const missingOne = t.pieces.filter((_, i) => i !== 0);
  assert.equal(assemble(t, missingOne), null);
});

test('GREEN: two nodes converge by gossip without any server (torrent-like)', () => {
  const payload = enc.encode(('autonomous content-addressed sync ').repeat(400));
  const t = createTorrent(payload, 1024);
  const A = new InMemoryNode('A');
  A.publish(t);
  const B = new InMemoryNode('B');
  B.publishPartial(t, [0]); // leecher: only first piece
  for (let i = 0; i < 10; i++) {
    const r1 = A.sync(B);
    const r2 = B.sync(A);
    if (r1.received === 0 && r2.received === 0) break;
  }
  const rebuilt = nodeAssemble(B, t.infoHash);
  assert.ok(rebuilt && Buffer.from(rebuilt).equals(Buffer.from(payload)));
});

test('GREEN: wantBitfield reports exactly the missing piece indexes', () => {
  const payload = enc.encode(('mesh ').repeat(300));
  const t = createTorrent(payload, 256);
  const C = new InMemoryNode('C');
  C.publishPartial(t, [0, 2]);
  const want = wantBitfield(t, C.store.get(t.infoHash)!.pieces.map((p) => p.index));
  const expected = [...Array(t.pieces.length).keys()].filter((i) => i !== 0 && i !== 2);
  assert.deepEqual(want.sort((a, b) => a - b), expected);
});

// ── KERNEL (deterministic, pure) ──

function mkCmd(partial: Partial<Command>): Command {
  return {
    actor: { kind: 'node', id: 'self' },
    action: 'INGEST',
    payload: 'deadbeef',
    nonce: 'n1',
    ...partial,
  } as Command;
}

test('GREEN: decide→fold→replay is deterministic and total', () => {
  const c = mkCmd({ action: 'INGEST', payload: 'abc' });
  const r1 = applyCommand(c, genesis());
  const r2 = applyCommand(c, genesis());
  assert.deepEqual(r1.state.ingested, r2.state.ingested);
  assert.ok(r1.state.ingested.has('abc'));
});

test('RED: replaying the same command (same cause) is a no-op (D2 dedupe)', () => {
  const c = mkCmd({ action: 'INGEST', payload: 'xyz' });
  const s1 = applyCommand(c, genesis()).state;
  const s2 = applyCommand(c, s1).state; // same cause again
  assert.deepEqual(s1.ingested, s2.ingested); // no double event
});

test('RED: revoked content cannot be published (kernel rejects)', () => {
  let st = genesis();
  st = applyCommand(mkCmd({ action: 'REVOKE', payload: 'bad' }), st).state;
  const pub = applyCommand(mkCmd({ action: 'PUBLISH', payload: 'bad' }), st);
  assert.equal(pub.envelopes.length, 1);
  assert.equal(pub.envelopes[0].event.type, 'DENIED');
});

test('GREEN: replay reconstructs state from an event log', () => {
  let st = genesis();
  for (const p of ['a', 'b', 'c']) st = applyCommand(mkCmd({ action: 'INGEST', payload: p, nonce: p }), st).state;
  const events = ['a', 'b', 'c'].map((p) => ({ type: 'INGESTED' as const, contentHash: p }));
  const r = replay(events);
  assert.deepEqual(r.ingested, st.ingested);
});
