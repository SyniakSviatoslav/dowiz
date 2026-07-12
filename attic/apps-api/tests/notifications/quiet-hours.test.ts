import test from 'node:test';
import assert from 'node:assert/strict';
import { evaluateQuietHours } from '../../src/notifications/quiet-hours.js';

const OVERNIGHT = { from: 22, to: 8 };
const DAYTIME = { from: 9, to: 17 };

test('quiet-hours evaluation', async (t) => {
  await t.test('null / disabled window is never quiet', () => {
    assert.equal(evaluateQuietHours(new Date('2026-06-22T03:00:00Z'), 'UTC', null).quiet, false);
    assert.equal(evaluateQuietHours(new Date('2026-06-22T03:00:00Z'), 'UTC', { from: 5, to: 5 }).quiet, false);
  });

  await t.test('overnight window wraps midnight (from > to)', () => {
    // boundaries: from inclusive, to exclusive
    assert.equal(evaluateQuietHours(new Date('2026-06-22T22:00:00Z'), 'UTC', OVERNIGHT).quiet, true);
    assert.equal(evaluateQuietHours(new Date('2026-06-22T03:00:00Z'), 'UTC', OVERNIGHT).quiet, true);
    assert.equal(evaluateQuietHours(new Date('2026-06-22T08:00:00Z'), 'UTC', OVERNIGHT).quiet, false);
    assert.equal(evaluateQuietHours(new Date('2026-06-22T12:00:00Z'), 'UTC', OVERNIGHT).quiet, false);
    assert.equal(evaluateQuietHours(new Date('2026-06-22T21:59:00Z'), 'UTC', OVERNIGHT).quiet, false);
  });

  await t.test('daytime window (from < to)', () => {
    assert.equal(evaluateQuietHours(new Date('2026-06-22T12:00:00Z'), 'UTC', DAYTIME).quiet, true);
    assert.equal(evaluateQuietHours(new Date('2026-06-22T08:00:00Z'), 'UTC', DAYTIME).quiet, false);
    assert.equal(evaluateQuietHours(new Date('2026-06-22T17:00:00Z'), 'UTC', DAYTIME).quiet, false);
  });

  await t.test('window is evaluated in the location timezone', () => {
    // 20:30 UTC. Etc/GMT-2 is a fixed UTC+2 (no DST) → 22:30 local → inside overnight window.
    const inst = new Date('2026-06-22T20:30:00Z');
    assert.equal(evaluateQuietHours(inst, 'Etc/GMT-2', OVERNIGHT).quiet, true, 'UTC+2 → 22:30 quiet');
    assert.equal(evaluateQuietHours(inst, 'UTC', OVERNIGHT).quiet, false, 'UTC → 20:30 not quiet');
  });

  await t.test('missing/invalid timezone falls back to default and flags it', () => {
    const inst = new Date('2026-06-22T03:00:00Z');
    const nullTz = evaluateQuietHours(inst, null, OVERNIGHT);
    assert.equal(nullTz.tzFallback, true);
    const badTz = evaluateQuietHours(inst, 'Not/AZone', OVERNIGHT);
    assert.equal(badTz.tzFallback, true);
    // a real timezone does not flag fallback
    assert.equal(evaluateQuietHours(inst, 'UTC', OVERNIGHT).tzFallback, false);
  });

  await t.test('secondsUntilEnd points at the window end (held startAfter)', () => {
    // 22:30 local (Etc/GMT-2), window ends at 08:00 → ~9.5h = 34200s
    const d = evaluateQuietHours(new Date('2026-06-22T20:30:00Z'), 'Etc/GMT-2', OVERNIGHT);
    assert.equal(d.quiet, true);
    assert.equal(d.secondsUntilEnd, 34200);
    // not quiet → 0
    assert.equal(evaluateQuietHours(new Date('2026-06-22T12:00:00Z'), 'UTC', OVERNIGHT).secondsUntilEnd, 0);
    // always at least 60s when quiet (never a zero/negative startAfter)
    const edge = evaluateQuietHours(new Date('2026-06-22T07:59:00Z'), 'UTC', OVERNIGHT);
    assert.equal(edge.quiet, true);
    assert.ok(edge.secondsUntilEnd >= 60);
  });
});
