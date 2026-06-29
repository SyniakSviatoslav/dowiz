import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { recordProvenUpgrade, readProvenUpgrades } from '../src/proven-upgrades.js';

const tmp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'gene-'));
const input = (id = 'repo-perf:tune:x:8') => ({
  id, patch_ref: 'set x = 8 (mechanical; git-revert)',
  metric_before: 1000, metric_after: 850, speedup_pct: 15,
  revert: 'git checkout -- config.x', provenance: 'config-tune detector | lenses pass',
});

test('recordProvenUpgrade — new gene → version 1, count 1, metric+revert persisted', () => {
  const dir = tmp();
  const g = recordProvenUpgrade(dir, input(), 'T0');
  assert.equal(g.version, 1);
  assert.equal(g.count, 1);
  assert.equal(g.first_proven, 'T0');
  assert.equal(g.last_proven, 'T0');
  assert.equal(g.metric_before, 1000);
  assert.equal(g.metric_after, 850);
  assert.equal(g.speedup_pct, 15);
  assert.equal(g.revert, 'git checkout -- config.x');
  assert.equal(readProvenUpgrades(dir).length, 1);
});

test('recordProvenUpgrade — re-proven bumps version+count, keeps first_proven (append-only ledger)', () => {
  const dir = tmp();
  recordProvenUpgrade(dir, input(), 'T0');
  const g2 = recordProvenUpgrade(dir, { ...input(), metric_after: 800, speedup_pct: 20 }, 'T1');
  assert.equal(g2.version, 2);
  assert.equal(g2.count, 2);
  assert.equal(g2.first_proven, 'T0', 'first_proven preserved');
  assert.equal(g2.last_proven, 'T1');
  assert.equal(g2.speedup_pct, 20, 'newer measurement supersedes');
  assert.equal(readProvenUpgrades(dir).length, 1, 'deduped by id');
});

test('recordProvenUpgrade — persisted JSON is durable + parseable', () => {
  const dir = tmp();
  recordProvenUpgrade(dir, input('a'), 'T0');
  recordProvenUpgrade(dir, input('b'), 'T0');
  const onDisk = JSON.parse(fs.readFileSync(path.join(dir, 'proven-upgrades.json'), 'utf8'));
  assert.equal(onDisk.length, 2);
  assert.deepEqual(onDisk.map((g: { id: string }) => g.id).sort(), ['a', 'b']);
});

test('readProvenUpgrades — absent file → []', () => {
  assert.deepEqual(readProvenUpgrades(tmp()), []);
});
