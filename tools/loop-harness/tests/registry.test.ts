import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { registerLoop, readRegistry, type RegistryLoop } from '../src/registry.js';

const tmp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'reg-'));
const loop = (id: string): RegistryLoop => ({ id, goal: 'g', trigger_tags: ['a', 'b'], scope_class: 'A', security_carveout: ['auth'], status: 'active' });

test('registerLoop — adds a loop; readRegistry returns it', () => {
  const dir = tmp();
  registerLoop(dir, loop('be-polish'));
  const all = readRegistry(dir);
  assert.equal(all.length, 1);
  assert.equal(all[0]!.id, 'be-polish');
});

test('registerLoop — upsert by id (no duplicate); preserves registered_at', () => {
  const dir = tmp();
  registerLoop(dir, { ...loop('qa'), registered_at: 'T0' });
  registerLoop(dir, { ...loop('qa'), registered_at: 'T9', last_success: 'now' });
  const all = readRegistry(dir);
  assert.equal(all.length, 1, 'deduped by id');
  assert.equal(all[0]!.registered_at, 'T0', 'original registered_at preserved');
  assert.equal(all[0]!.last_success, 'now');
});

test('readRegistry — absent file → []', () => {
  assert.deepEqual(readRegistry(tmp()), []);
});
