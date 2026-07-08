// Regression guardrail (docs/regressions/REGRESSION-LEDGER.md #50): plane-report's gate digest
// must stay parseable even when the failing gate command prints valid JSON on stdout and leaves
// stderr genuinely empty (empty string, not undefined).
//
// RED proof (pre-fix behavior): revert to `stdout + (stderr || message)` and this test's first
// case fails — the merged string gets execSync's "Command failed: …" appended after the JSON
// close-brace, so JSON.parse throws.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mergeCaptureOutput } from './capture-merge.mjs';

test('valid JSON stdout + empty stderr stays parseable (does not fall back to message)', () => {
  const stdout = JSON.stringify({ verdict: 'FAIL' });
  const merged = mergeCaptureOutput(stdout, '', 'Command failed: exit 1');
  assert.doesNotThrow(() => JSON.parse(merged));
  assert.deepEqual(JSON.parse(merged), { verdict: 'FAIL' });
});

test('valid JSON stdout + non-empty stderr appends the real stderr, not the message', () => {
  const stdout = JSON.stringify({ verdict: 'FAIL' });
  const merged = mergeCaptureOutput(stdout, 'warning: deprecated flag\n', 'Command failed: exit 1');
  assert.equal(merged, stdout + 'warning: deprecated flag\n');
});

test('no output on either stream falls back to the exec error message (e.g. ENOENT)', () => {
  const merged = mergeCaptureOutput('', '', 'Cannot find module foo.mjs');
  assert.equal(merged, 'Cannot find module foo.mjs');
});

test('undefined stdout/stderr (never captured) are treated as empty, not concatenated as "undefined"', () => {
  const merged = mergeCaptureOutput(undefined, undefined, 'spawn failed');
  assert.equal(merged, 'spawn failed');
});
