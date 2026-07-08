// Bebop store tests — content-addressed, hash-chained, durable. RED+GREEN.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { ContentStore } from './store.ts';

function tmpDir(): string {
  const d = path.join(os.tmpdir(), `bebop-store-${Math.random().toString(36).slice(2)}`);
  fs.mkdirSync(d, { recursive: true });
  return d;
}

test('GREEN: appended events form a hash chain and verifyChain() passes', () => {
  const s = new ContentStore(tmpDir());
  s.appendEvent('cause-a', 'do X');
  s.appendEvent('cause-b', 'do Y');
  assert.equal(s.eventCount, 2);
  assert.equal(s.verifyChain(), true);
});

test('RED: tampering with a stored event breaks verifyChain() (falsifiable integrity)', () => {
  const dir = tmpDir();
  const s = new ContentStore(dir);
  s.appendEvent('cause-a', 'do X');
  s.appendEvent('cause-b', 'do Y');
  // reopen, mutate event[0].data on disk, reload
  const ef = path.join(dir, 'events.jsonl');
  const lines = fs.readFileSync(ef, 'utf8').trim().split('\n');
  const ev0 = JSON.parse(lines[0]);
  ev0.data = 'MALICIOUS';
  lines[0] = JSON.stringify(ev0);
  fs.writeFileSync(ef, lines.join('\n') + '\n');
  const s2 = new ContentStore(dir);
  assert.equal(s2.verifyChain(), false); // chain rejects the tamper
});

test('GREEN: a piece is addressable by sha256 and round-trips', () => {
  const s = new ContentStore(tmpDir());
  const bytes = new TextEncoder().encode('hello bebop mesh');
  const p = s.putPiece(0, bytes);
  const got = s.getPiece(0)!;
  assert.equal(got.hash, p.hash);
  assert.equal(new TextDecoder().decode(Buffer.from(got.bytes, 'base64')), 'hello bebop mesh');
});

test('GREEN: store survives restart (durable across separate instances)', () => {
  const dir = tmpDir();
  const s1 = new ContentStore(dir);
  s1.appendEvent('cause-a', 'persisted');
  s1.putPiece(0, new TextEncoder().encode('survive'));
  // a brand-new instance over the SAME dir reloads everything
  const s2 = new ContentStore(dir);
  assert.equal(s2.eventCount, 1);
  assert.equal(s2.verifyChain(), true);
  assert.equal(new TextDecoder().decode(Buffer.from(s2.getPiece(0)!.bytes, 'base64')), 'survive');
});
