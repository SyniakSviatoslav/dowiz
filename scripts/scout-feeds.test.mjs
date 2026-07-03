#!/usr/bin/env node
// Tests for scout-feeds — node:test, fixture Atom docs (never hammers real feeds).
// Run: node --test scripts/scout-feeds.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { writeFileSync, mkdirSync, rmSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { parseAtom, diffNewEntries, sanitizeTitle, scan, DEFAULT_WATCHLIST } from './scout-feeds.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const SCRIPT = join(HERE, 'scout-feeds.mjs');

// A fixture Atom document — shape mirrors GitHub's /<owner>/<repo>/releases.atom.
const ATOM = `<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Release notes from foo</title>
  <entry>
    <id>tag:github.com,2008:Repository/1/v3.0.0</id>
    <title>v3.0.0 — big release</title>
    <updated>2026-07-02T10:00:00Z</updated>
    <link rel="alternate" type="text/html" href="https://github.com/acme/foo/releases/tag/v3.0.0"/>
  </entry>
  <entry>
    <id>tag:github.com,2008:Repository/1/v2.0.0</id>
    <title>v2.0.0 &amp; patches</title>
    <updated>2026-06-01T10:00:00Z</updated>
    <link rel="alternate" type="text/html" href="https://github.com/acme/foo/releases/tag/v2.0.0"/>
  </entry>
</feed>`;

// A malicious/remote title carrying ANSI + C0 control chars — must be neutralized before output.
const ATOM_EVIL = `<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <id>evil-1</id>
    <title>v9\x1b[31m\x07\x00\x1b]0;pwn\x07 release\nsecond line</title>
    <updated>2026-07-02T12:00:00Z</updated>
    <link href="https://github.com/acme/foo/releases/tag/v9"/>
  </entry>
</feed>`;

test('parseAtom extracts title/updated/link/id and decodes entities', () => {
  const entries = parseAtom(ATOM);
  assert.equal(entries.length, 2);
  assert.equal(entries[0].title, 'v3.0.0 — big release');
  assert.equal(entries[0].updated, '2026-07-02T10:00:00Z');
  assert.equal(entries[0].link, 'https://github.com/acme/foo/releases/tag/v3.0.0');
  assert.equal(entries[1].title, 'v2.0.0 & patches'); // &amp; decoded
});

test('diffNewEntries: only entries strictly newer than the cursor are NEW', () => {
  const entries = parseAtom(ATOM);
  // cursor set to the older release → only v3.0.0 is new
  const fresh = diffNewEntries(entries, { lastSeen: '2026-06-01T10:00:00Z', seenKeys: [] });
  assert.equal(fresh.length, 1);
  assert.equal(fresh[0].title, 'v3.0.0 — big release');
});

test('diffNewEntries: no cursor → every entry is new (first-run baseline)', () => {
  const fresh = diffNewEntries(parseAtom(ATOM), undefined);
  assert.equal(fresh.length, 2);
  // newest-first order
  assert.equal(fresh[0].updated, '2026-07-02T10:00:00Z');
});

test('diffNewEntries: cursor at newest → nothing new', () => {
  const fresh = diffNewEntries(parseAtom(ATOM), { lastSeen: '2026-07-02T10:00:00Z', seenKeys: [] });
  assert.equal(fresh.length, 0);
});

test('sanitizeTitle strips ANSI/OSC + control chars and caps length', () => {
  const evil = parseAtom(ATOM_EVIL)[0].title;
  const clean = sanitizeTitle(evil);
  // eslint-disable-next-line no-control-regex
  assert.ok(!/[\x00-\x1f\x7f]/.test(clean), 'no raw control chars survive');
  assert.ok(!clean.includes('[31m'), 'ANSI CSI stripped');
  assert.ok(clean.includes('release'), 'legible text preserved');
  assert.ok(!clean.includes('\n'), 'newlines flattened');
  const capped = sanitizeTitle('x'.repeat(500));
  assert.ok(capped.endsWith('…[capped]'));
  assert.ok(capped.length <= 210);
});

test('scan: dead feed is tolerated — other feeds still reported', async () => {
  const okUrl = 'https://github.com/acme/foo/releases.atom';
  const deadUrl = 'https://github.com/acme/bar/releases.atom';
  const watchlist = [
    { name: 'foo', owner: 'acme', repo: 'foo' },
    { name: 'bar', owner: 'acme', repo: 'bar' },
  ];
  const fixtures = { [okUrl]: ATOM, [deadUrl]: 'ERROR' };
  const report = await scan(watchlist, { schema_version: 1, feeds: {} }, { fixtures });
  const foo = report.results.find((r) => r.repo === 'acme/foo');
  const bar = report.results.find((r) => r.repo === 'acme/bar');
  assert.equal(bar.error != null, true, 'dead feed records an error');
  assert.equal(bar.new.length, 0);
  assert.equal(foo.error, null, 'live feed unaffected by the dead one');
  assert.equal(foo.new.length, 2, 'live feed still reports its entries');
  assert.equal(report.feeds_failed, 1);
  assert.equal(report.total_new, 2);
});

test('scan: report envelope is advisory + untrusted-remote + sanitizes titles', async () => {
  const url = 'https://github.com/acme/foo/releases.atom';
  const report = await scan([{ name: 'foo', owner: 'acme', repo: 'foo' }], { schema_version: 1, feeds: {} }, { fixtures: { [url]: ATOM_EVIL } });
  assert.equal(report.advisory, true);
  assert.equal(report.content_trust, 'untrusted-remote');
  const foo = report.results[0];
  // eslint-disable-next-line no-control-regex
  assert.ok(!/[\x00-\x1f\x7f]/.test(foo.new[0].title), 'title sanitized in report');
});

test('--json output shape is stable (subprocess, fixtures)', () => {
  const url = 'https://github.com/acme/foo/releases.atom';
  const wlPath = join(HERE, '..', 'loops', 'runs', '.scout-test-watchlist.json');
  mkdirSync(dirname(wlPath), { recursive: true });
  writeFileSync(wlPath, JSON.stringify([{ name: 'foo', owner: 'acme', repo: 'foo' }]));
  try {
    const r = spawnSync(process.execPath, [SCRIPT, '--json', '--watchlist', wlPath], {
      encoding: 'utf8',
      env: { ...process.env, SCOUT_FIXTURES: JSON.stringify({ [url]: ATOM }) },
    });
    assert.equal(r.status, 0, r.stderr);
    const parsed = JSON.parse(r.stdout);
    for (const k of ['schema_version', 'generated_ts', 'content_trust', 'advisory', 'feeds_checked', 'feeds_failed', 'total_new', 'results']) {
      assert.ok(k in parsed, `envelope has ${k}`);
    }
    assert.equal(parsed.results[0].repo, 'acme/foo');
    assert.ok(Array.isArray(parsed.results[0].new));
  } finally {
    try { rmSync(wlPath); } catch { /* ignore */ }
  }
});

test('DEFAULT_WATCHLIST is seeded with load-bearing deps + adopted tools', () => {
  const names = DEFAULT_WATCHLIST.map((e) => e.name);
  for (const must of ['fastify', 'react', 'zod', 'argon2', 'jose']) assert.ok(names.includes(must), `watchlist includes ${must}`);
  for (const e of DEFAULT_WATCHLIST) { assert.ok(e.owner && e.repo, `${e.name} has owner+repo`); }
});
