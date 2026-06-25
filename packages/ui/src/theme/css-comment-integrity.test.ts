import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// GUARDRAIL (regression ledger #13, cheap static arm) — complements the live-DOM E2E
// (e2e/tests/paper-skin-tokens.spec.ts) with a fast, browser-free check.
//
// A CSS block comment is terminated by the FIRST `*/`. If `*/` appears in comment PROSE
// (e.g. "--ink-*/--paper-*"), the comment closes early and the browser silently drops the
// next rule. Detector: strip canonical comments (`/* ... */`, non-greedy = up to first `*/`);
// in well-formed CSS every `/*`/`*/` is consumed in a matched pair, so ANY leftover `/*` or
// `*/` marker means a comment opened/closed where it shouldn't have. This is exactly the
// fingerprint of the bug (a dangling intended-terminator survives the strip).
const THEME_DIR = join(dirname(fileURLToPath(import.meta.url)));

function strippedHasDanglingMarker(css: string): boolean {
  const withoutComments = css.replace(/\/\*[\s\S]*?\*\//g, '');
  return withoutComments.includes('*/') || withoutComments.includes('/*');
}

test('theme CSS has no */-in-comment-prose drops (no dangling comment markers)', () => {
  const cssFiles = readdirSync(THEME_DIR).filter((f) => f.endsWith('.css'));
  assert.ok(cssFiles.length > 0, 'found theme CSS files to check');
  for (const f of cssFiles) {
    const css = readFileSync(join(THEME_DIR, f), 'utf8');
    assert.equal(
      strippedHasDanglingMarker(css),
      false,
      `${f}: a comment marker (/* or */) survives comment-stripping — a comment likely closed ` +
        `early (a literal */ in prose). The browser will drop the next rule. See ledger #13.`,
    );
  }
});

test('the static check is sharp — the bug shape is detected (red arm)', () => {
  // The exact shape that broke tokens.css: `*/` inside comment prose, then a rule.
  const bugged = `/* adds --ink-*/--paper-* tokens, see docs */\n[data-skin="paper"]{--paper-bg:#F4ECDB}`;
  assert.equal(strippedHasDanglingMarker(bugged), true, 'detector flags the */-in-prose shape');
  // A correctly-phrased comment (no */ in prose) is clean.
  const clean = `/* adds ink, paper and display tokens, see docs */\n[data-skin="paper"]{--paper-bg:#F4ECDB}`;
  assert.equal(strippedHasDanglingMarker(clean), false, 'detector passes clean comment prose');
});
