import { test } from 'node:test';
import assert from 'node:assert/strict';
import { recordComplaint } from '../src/modules/acquisition/claim.js';

// CC4 — declines + complaints are structured log events so decline-without-complaint is a computable
// health signal (no migration). This proves the complaint event shape; the decline event
// (acquisition.shadow_declined) is emitted by declineAndErase on the same pattern.
test('recordComplaint emits a structured acquisition.complaint event', () => {
  const lines: string[] = [];
  const orig = console.log;
  console.log = (...a: unknown[]) => { lines.push(String(a[0])); };
  try {
    recordComplaint('ChIJ_test_place', 'owner sent a C&D');
  } finally {
    console.log = orig;
  }
  const evt = lines.map((l) => { try { return JSON.parse(l); } catch { return null; } }).find((e) => e?.event === 'acquisition.complaint');
  assert.ok(evt, 'a complaint event was emitted');
  assert.equal(evt.place_id, 'ChIJ_test_place');
  assert.equal(evt.note, 'owner sent a C&D');
  assert.ok(typeof evt.at === 'string' && evt.at.length > 0, 'timestamped');
});

test('recordComplaint without a note still emits (note=null)', () => {
  const lines: string[] = [];
  const orig = console.log;
  console.log = (...a: unknown[]) => { lines.push(String(a[0])); };
  try {
    recordComplaint('ChIJ_x');
  } finally {
    console.log = orig;
  }
  const evt = lines.map((l) => { try { return JSON.parse(l); } catch { return null; } }).find((e) => e?.event === 'acquisition.complaint');
  assert.ok(evt);
  assert.equal(evt.note, null);
});
