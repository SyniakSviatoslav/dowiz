// health-gate.test.mjs — proves the fail-closed contract deterministically.
// Run: node --test tools/health-gate.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkDiskFree, checkVolumeMount, checkKernel, evaluate } from './health-gate.mjs';

test('FAIL-CLOSED: disk check fails when FORCE_FAIL=disk injected', async () => {
  const prev = process.env.FORCE_FAIL;
  process.env.FORCE_FAIL = 'disk';
  try {
    const r = checkDiskFree();
    assert.equal(r.ok, false);
    assert.equal(r.name, 'disk-free');
  } finally {
    process.env.FORCE_FAIL = prev;
  }
});

test('FAIL-CLOSED: volume check fails when FORCE_FAIL=volume injected', async () => {
  const prev = process.env.FORCE_FAIL;
  process.env.FORCE_FAIL = 'volume';
  try {
    const r = checkVolumeMount();
    assert.equal(r.ok, false);
    assert.equal(r.name, 'volume-mount');
  } finally {
    process.env.FORCE_FAIL = prev;
  }
});

test('FAIL-CLOSED: kernel check fails when FORCE_FAIL=kernel injected', async () => {
  const prev = process.env.FORCE_FAIL;
  process.env.FORCE_FAIL = 'kernel';
  try {
    const r = await checkKernel();
    assert.equal(r.ok, false);
    assert.equal(r.name, 'kernel-test');
  } finally {
    process.env.FORCE_FAIL = prev;
  }
});

test('FAIL-CLOSED: evaluate() returns ok=false when ANY check is forced to fail', async () => {
  const prev = process.env.FORCE_FAIL;
  process.env.FORCE_FAIL = 'disk';
  try {
    const r = await evaluate();
    assert.equal(r.ok, false);
    assert.equal(r.checks.length, 3);
    assert.ok(r.checks.some((c) => c.ok === false));
  } finally {
    process.env.FORCE_FAIL = prev;
  }
});

test('GREEN: disk check passes with minFreePct=0 (any non-negative free space)', () => {
  const r = checkDiskFree({ minFreePct: 0 });
  assert.equal(r.ok, true);
  assert.equal(r.name, 'disk-free');
});

test('GREEN/behavioral: volume check correctly flags "/" as NOT a separate mount', () => {
  // '/' has the same device as '/', so it must be reported as not-a-separate-volume.
  const r = checkVolumeMount({ path: '/' });
  assert.equal(r.ok, false);
  assert.match(r.detail, /not a separate mount/);
});

test('SHAPE: evaluate() returns {ok, checks[]} with name/ok/detail on each', async () => {
  const prev = process.env.FORCE_FAIL;
  process.env.FORCE_FAIL = 'kernel'; // force a known, deterministic (fail) outcome
  try {
    const r = await evaluate();
    assert.equal(typeof r.ok, 'boolean');
    assert.equal(r.checks.length, 3);
    for (const c of r.checks) {
      assert.equal(typeof c.name, 'string');
      assert.equal(typeof c.ok, 'boolean');
      assert.equal(typeof c.detail, 'string');
    }
  } finally {
    process.env.FORCE_FAIL = prev;
  }
});
