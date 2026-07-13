// new-dep-scan.test.mjs — DoD: the dep-baseline must survive an ephemeral cloud container.
//
// scripts/new-dep-scan.mjs writes its baseline to loops/runs/dep-baseline.json. loops/runs/* is
// gitignored by default (only metrics.jsonl/routing.jsonl/registry.json were carved out) — so on
// the scheduled cloud maintainer (fresh checkout every firing, no persistent disk between runs)
// `--bump` never survived to the next day: every run saw "no baseline yet" and re-bumped a
// first-time baseline forever, so a REAL newcomer dependency could never be detected. Confirmed on
// 2 consecutive runs (2026-07-12, 2026-07-13) before .gitignore added the same carve-out already
// used for registry.json.
//
// Tests the ACTUAL repo .gitignore content in an isolated scratch checkout — not the live
// working tree — so a file already `git add`-ed in this session's index (which masks gitignore
// matching regardless of pattern content) can't produce a false negative.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

const REPO_ROOT = new URL('..', import.meta.url).pathname;
const REAL_GITIGNORE = readFileSync(join(REPO_ROOT, '.gitignore'), 'utf8');

function scratchRepo(t) {
  const dir = mkdtempSync(join(tmpdir(), 'ndep-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  spawnSync('git', ['init', '--initial-branch=main', dir], { encoding: 'utf8' });
  return dir;
}

function isIgnored(dir, gitignoreContent, relPath) {
  writeFileSync(join(dir, '.gitignore'), gitignoreContent);
  mkdirSync(join(dir, 'loops', 'runs'), { recursive: true });
  writeFileSync(join(dir, relPath), '{}');
  const r = spawnSync('git', ['check-ignore', relPath], { cwd: dir, encoding: 'utf8' });
  return r.status === 0; // 0 = ignored, 1 = trackable
}

test('loops/runs/dep-baseline.json is NOT gitignored in the real repo .gitignore (must survive an ephemeral container, like registry.json)', (t) => {
  const dir = scratchRepo(t);
  assert.equal(isIgnored(dir, REAL_GITIGNORE, 'loops/runs/dep-baseline.json'), false,
    'loops/runs/dep-baseline.json must NOT be gitignored — the cloud maintainer loses it every run otherwise');
});

test('RED proof: the pre-fix .gitignore (no dep-baseline.json carve-out) DOES ignore it', (t) => {
  const dir = scratchRepo(t);
  const preFix = REAL_GITIGNORE.replace('!loops/runs/dep-baseline.json\n', '');
  assert.notEqual(preFix, REAL_GITIGNORE, 'test setup bug: carve-out line not found to strip');
  assert.equal(isIgnored(dir, preFix, 'loops/runs/dep-baseline.json'), true,
    'without the carve-out, loops/runs/* must still catch dep-baseline.json (sanity check that the pattern match itself works)');
});
