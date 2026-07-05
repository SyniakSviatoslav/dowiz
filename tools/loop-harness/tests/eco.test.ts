import { test } from 'node:test';
import assert from 'node:assert/strict';
import { computeEco } from '../src/eco.js';

test('computeEco — empty input is all zero', () => {
  const e = computeEco({});
  assert.equal(e.kwh, 0);
  assert.equal(e.gco2, 0);
  assert.equal(e.water_ml, 0);
  assert.equal(e.estimate, true);
});

test('computeEco — opus tokens yield non-zero energy/CO2/water that scale with tokens', () => {
  const a = computeEco({ 'claude-opus-4-8': 100_000 });
  const b = computeEco({ 'claude-opus-4-8': 200_000 });
  assert.ok(a.kwh! > 0 && a.gco2! > 0 && a.water_ml! > 0);
  assert.ok(Math.abs(b.kwh! - 2 * a.kwh!) < 1e-9, 'energy scales linearly with tokens');
});

test('computeEco — a smaller model has a lower factor than opus for equal tokens', () => {
  const opus = computeEco({ 'claude-opus-4-8': 100_000 });
  const haiku = computeEco({ 'claude-haiku-4-5': 100_000 });
  assert.ok(haiku.kwh! < opus.kwh!);
});

test('computeEco — prefix match handles versioned ids; unknown falls back', () => {
  const known = computeEco({ 'claude-opus-4-8-something': 100_000 });
  const unknown = computeEco({ 'mystery-model': 100_000 });
  assert.ok(known.kwh! > 0);
  assert.ok(unknown.kwh! > 0); // default factor, not zero
});
