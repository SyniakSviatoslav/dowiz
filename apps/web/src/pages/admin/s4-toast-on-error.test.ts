import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// GUARDRAIL (audit-frontend-2026-07-03.md, class S4 — "silent mutation failures"):
// DashboardPage.tsx order-message send + BrandingPage.tsx logo upload both used to fail
// with a console-only catch, leaving the owner believing the action succeeded. There's no
// jsdom/testing-library in this repo (by convention — see AGENTS.md), so we can't render
// these components; instead this is a source-content regression test, mirroring
// packages/ui/src/theme/css-comment-integrity.test.ts's style (readFileSync + regex +
// a red-arm self-test proving the detector actually detects the bug shape).

const DIR = dirname(fileURLToPath(import.meta.url));
const dashboardSrc = readFileSync(join(DIR, 'DashboardPage.tsx'), 'utf8');
const brandingSrc = readFileSync(join(DIR, 'BrandingPage.tsx'), 'utf8');

// Extracts a top-level `const <name> = async (...) => { ... };` function body (bounded by
// the next 2-space-indented closing `};`) so assertions can't accidentally match some OTHER
// function's showToast call elsewhere in the file.
function extractFn(source: string, name: string): string {
  const re = new RegExp(`const ${name}\\s*=\\s*async[\\s\\S]*?\\n  \\};`);
  const m = source.match(re);
  assert.ok(m, `expected to find function '${name}' in source`);
  return m![0];
}

test('DashboardPage: order-message send failure surfaces a toast (S4 fix)', () => {
  const fn = extractFn(dashboardSrc, 'handleSendMessage');
  assert.match(
    fn,
    /catch \(err\) \{[\s\S]*showToast\(/,
    'handleSendMessage catch must call showToast — a console-only catch leaves the owner ' +
      'believing a failed send actually went through',
  );
});

test('BrandingPage: logo-upload skip (locationId not ready) surfaces a toast (finding #40)', () => {
  const fn = extractFn(brandingSrc, 'handleLogoUpload');
  assert.match(
    fn,
    /if \(!locationId\) \{[\s\S]*?showToast\([\s\S]*?'warning'\)/,
    'the locationId-not-ready branch must warn the owner instead of silently returning',
  );
});

test('BrandingPage: logo-upload failure surfaces a toast AND reverts the optimistic preview (finding #40)', () => {
  const fn = extractFn(brandingSrc, 'handleLogoUpload');
  assert.match(
    fn,
    /catch \(err\) \{[\s\S]*setLogoDataUrl\(''\)[\s\S]*showToast\([\s\S]*?'error'\)/,
    'the upload catch must revert the preview to the last-saved logo AND toast an error, not fail bare',
  );
});

test('BrandingPage: file-too-large no longer uses a native alert()', () => {
  assert.doesNotMatch(
    brandingSrc,
    /alert\(t\('admin\.error_file_too_large'/,
    'must use showToast, not a native alert(), for consistency with the rest of the page',
  );
});

test('BrandingPage: logo upload control is disabled until locationId resolves', () => {
  assert.match(
    brandingSrc,
    /disabled=\{logoUploading \|\| !locationId\}/,
    'the file input must stay disabled while locationId has not resolved yet, so the ' +
      'silent-skip path can never be reached through the UI',
  );
});

test('red arm — the extractor + detector are sharp, not vacuously true', () => {
  const buggedDashboard = `
  const handleSendMessage = async (orderId) => {
    try {
      await apiClient();
    } catch (err) {
      console.warn('send message failed:', err);
    }
  };
`;
  const fn = extractFn(buggedDashboard, 'handleSendMessage');
  assert.doesNotMatch(
    fn,
    /catch \(err\) \{[\s\S]*showToast\(/,
    'a bare console-only catch must NOT pass the showToast assertion (detector must actually detect)',
  );

  const buggedBranding = `
  const handleLogoUpload = async (e) => {
    if (!locationId) return;
    try {
      await apiClient();
    } catch {
    }
  };
`;
  const fn2 = extractFn(buggedBranding, 'handleLogoUpload');
  assert.doesNotMatch(fn2, /if \(!locationId\) \{[\s\S]*?showToast\(/, 'a silent locationId skip must not pass');
  assert.doesNotMatch(fn2, /catch[\s\S]*setLogoDataUrl/, 'a bare catch must not pass');
});
