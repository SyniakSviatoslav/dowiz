#!/usr/bin/env node
// Tests for asset-surface-scan — crt.sh is NEVER hit here: every response is a fixture.
// Run: node --test scripts/asset-surface-scan.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  extractSubdomains, buildReport, scanDomains, main, DEFAULT_DOMAINS,
} from './asset-surface-scan.mjs';

// A crt.sh-shaped response body (raw JSON text, as the API returns it).
const certBody = (names) => JSON.stringify(
  names.map((n) => ({ common_name: n, name_value: n })),
);

test('extractSubdomains: filters to the domain, strips wildcard, dedupes, sorts', () => {
  const body = JSON.stringify([
    { common_name: 'API.dowiz.org', name_value: 'api.dowiz.org\n*.dowiz.org' },
    { common_name: 'staging.dowiz.org', name_value: 'staging.dowiz.org' },
    { common_name: 'evil.example.com', name_value: 'evil.example.com' }, // unrelated → dropped
    { common_name: 'dowiz.org', name_value: 'dowiz.org' }, // apex kept
  ]);
  const { names, parseError } = extractSubdomains(body, 'dowiz.org');
  assert.equal(parseError, false);
  assert.deepEqual(names, ['api.dowiz.org', 'dowiz.org', 'staging.dowiz.org']);
});

test('DIFF DETECTION: baseline missing a host → that host is flagged NEW', () => {
  const domain = 'dowiz.org';
  const results = { [domain]: { names: ['a.dowiz.org', 'new.dowiz.org'] } };
  const baseline = { updated_ts: 't0', domains: { [domain]: ['a.dowiz.org'] } };
  const report = buildReport([domain], results, baseline);
  assert.deepEqual(report.domains[domain].new, ['new.dowiz.org']);
  assert.equal(report.summary.total_new, 1);
});

test('DIFF DETECTION: stable set (baseline == current) → zero new', () => {
  const domain = 'dowiz.org';
  const names = ['a.dowiz.org', 'b.dowiz.org'];
  const results = { [domain]: { names } };
  const baseline = { updated_ts: 't0', domains: { [domain]: [...names] } };
  const report = buildReport([domain], results, baseline);
  assert.deepEqual(report.domains[domain].new, []);
  assert.equal(report.summary.total_new, 0);
});

test('no baseline → nothing flagged NEW (baseline must be established first)', () => {
  const domain = 'dowiz.org';
  const results = { [domain]: { names: ['a.dowiz.org'] } };
  const report = buildReport([domain], results, null);
  assert.equal(report.has_baseline, false);
  assert.deepEqual(report.domains[domain].new, []);
});

test('malformed response → clean error, not a crash', () => {
  assert.deepEqual(extractSubdomains('this is not json', 'dowiz.org'), { names: [], parseError: true });
  assert.deepEqual(extractSubdomains('{"error":"rate limited"}', 'dowiz.org'), { names: [], parseError: true });
});

test('empty response → no certs, no error', () => {
  assert.deepEqual(extractSubdomains('[]', 'dowiz.org'), { names: [], parseError: false });
});

test('scanDomains via fixture: malformed body surfaces as a per-domain error, no throw', async () => {
  const results = await scanDomains(['dowiz.org', 'dowiz.com'], {
    fixture: { 'dowiz.org': certBody(['a.dowiz.org']), 'dowiz.com': 'garbage-not-json' },
  });
  assert.deepEqual(results['dowiz.org'].names, ['a.dowiz.org']);
  assert.ok(results['dowiz.com'].error, 'malformed domain should carry an error field');
  assert.deepEqual(results['dowiz.com'].names, []);
});

test('scanDomains via fixture: domain absent from fixture → empty (no network, no throw)', async () => {
  const results = await scanDomains(['dowiz.org'], { fixture: {} });
  assert.deepEqual(results['dowiz.org'], { names: [] });
});

test('--json shape is stable (top-level + per-domain + summary keys)', async () => {
  // Isolate: fresh ROOT (no baseline) + a fixture file; capture stdout.
  const dir = mkdtempSync(join(tmpdir(), 'asset-scan-'));
  const fixturePath = join(dir, 'fx.json');
  writeFileSync(fixturePath, JSON.stringify({ 'dowiz.org': certBody(['api.dowiz.org', 'www.dowiz.org']) }));
  process.env.ASSET_SCAN_ROOT = dir;

  const out = [];
  const orig = console.log;
  console.log = (...a) => out.push(a.join(' '));
  let code;
  try {
    code = await main(['--json', '--domains', 'dowiz.org', '--test-fixture', fixturePath]);
  } finally {
    console.log = orig;
    delete process.env.ASSET_SCAN_ROOT;
  }
  assert.equal(code, 0);
  const report = JSON.parse(out.join('\n'));
  assert.equal(report.schema_version, 1);
  assert.ok(typeof report.generated_ts === 'string');
  assert.equal(report.has_baseline, false);
  assert.ok(report.domains['dowiz.org']);
  assert.equal(report.domains['dowiz.org'].total, 2);
  assert.deepEqual(report.domains['dowiz.org'].subdomains, ['api.dowiz.org', 'www.dowiz.org']);
  for (const k of ['total_subdomains', 'total_new', 'errored_domains', 'scanned_domains']) {
    assert.ok(k in report.summary, `summary.${k} present`);
  }
});

test('all domains fail → main exits non-zero (crt.sh down degradation)', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'asset-scan-'));
  const fixturePath = join(dir, 'fx.json');
  // Fixture maps every requested domain to a malformed body → all error.
  writeFileSync(fixturePath, JSON.stringify({ 'dowiz.org': 'nope', 'dowiz.com': 'nope' }));
  process.env.ASSET_SCAN_ROOT = dir;
  const orig = console.log; console.log = () => {};
  let code;
  try {
    code = await main(['--json', '--domains', 'dowiz.org,dowiz.com', '--test-fixture', fixturePath]);
  } finally { console.log = orig; delete process.env.ASSET_SCAN_ROOT; }
  assert.equal(code, 1);
});

test('DEFAULT_DOMAINS are the operator’s own hosts', () => {
  assert.ok(DEFAULT_DOMAINS.includes('dowiz.fly.dev'));
  assert.ok(DEFAULT_DOMAINS.includes('dowiz-staging.fly.dev'));
});
