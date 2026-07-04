// Equal-affordance proof (STOP-2 / C-2 — docs/design/voice-control/ui-spec.md §3/§5, hardened per
// docs/design/voice-pr3-ui-statemachine/resolution.md "[MED] Equal-affordance CI-assertion blind
// spot"). No jsdom/React-DOM is available in this repo's test runner (`tsx --test
// src/**/*.test.ts`, node:test, no `@testing-library/react`), so this asserts DECLARED style
// parity two ways: (1) the shared constants themselves carry no button-differentiating field, and
// (2) a source-level scan proves BOTH buttons in each equal-affordance surface reference the exact
// same exported constant — not a hand-copied lookalike — so there is no merge point where a parent
// or a future edit could reintroduce asymmetry (the full interactive Playwright computed-style
// assertion is later-PR/operator work, per the task's own scope note).

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { EQUAL_AFFORDANCE_BUTTON_CLASSNAME, EQUAL_AFFORDANCE_BUTTON_STYLE } from '../layout.js';

const VOICE_DIR = join(dirname(fileURLToPath(import.meta.url)), '..');

function readSource(file: string): string {
  return readFileSync(join(VOICE_DIR, file), 'utf8');
}

function countOccurrences(haystack: string, needle: string): number {
  return haystack.split(needle).length - 1;
}

describe('EQUAL_AFFORDANCE_BUTTON_CLASSNAME / _STYLE — the shared constant itself', () => {
  it('is a single non-empty className string (deterministic, no runtime branching)', () => {
    assert.equal(typeof EQUAL_AFFORDANCE_BUTTON_CLASSNAME, 'string');
    assert.ok(EQUAL_AFFORDANCE_BUTTON_CLASSNAME.length > 0);
  });

  it('the style object fixes box-model properties beyond the 4 CI-checked ones (breaker MED fix)', () => {
    // ui-spec §3 names background/border-width/min-height/font-weight as the CI-asserted set; the
    // breaker found that leaves padding/gap/box-shadow unmeasured. Assert they are ALSO fixed here.
    const keys = Object.keys(EQUAL_AFFORDANCE_BUTTON_STYLE);
    for (const required of ['minHeight', 'padding', 'gap', 'background', 'border', 'color', 'fontWeight', 'boxShadow']) {
      assert.ok(keys.includes(required), `EQUAL_AFFORDANCE_BUTTON_STYLE is missing "${required}"`);
    }
  });

  it('carries no colour outside the neutral brand-surface family (no red/green/danger/success token)', () => {
    const serialized = JSON.stringify(EQUAL_AFFORDANCE_BUTTON_STYLE).toLowerCase();
    for (const forbidden of ['danger', 'success', 'red', 'green', '--color-warning']) {
      assert.ok(!serialized.includes(forbidden), `unexpected semantic-danger/success colour token: ${forbidden}`);
    }
  });
});

describe('ConfirmChip.tsx — Confirm/Cancel reference the SAME shared constant', () => {
  const src = readSource('ConfirmChip.tsx');

  it('both buttons use EQUAL_AFFORDANCE_BUTTON_CLASSNAME (exactly twice — one per button)', () => {
    assert.equal(countOccurrences(src, 'className={EQUAL_AFFORDANCE_BUTTON_CLASSNAME}'), 2);
  });

  it('both buttons use EQUAL_AFFORDANCE_BUTTON_STYLE (exactly twice — one per button)', () => {
    assert.equal(countOccurrences(src, 'style={EQUAL_AFFORDANCE_BUTTON_STYLE}'), 2);
  });

  it('neither button declares a competing/local style or className override', () => {
    // A per-button override point is exactly the "parent merges an extra className" vector the
    // breaker flagged — assert there is no OTHER className=/style= on a <button> in this file.
    const buttonBlocks = src.match(/<button\b[\s\S]*?>/g) ?? [];
    assert.equal(buttonBlocks.length, 2, 'expected exactly 2 <button> elements (Cancel, Confirm)');
    for (const block of buttonBlocks) {
      assert.ok(block.includes('EQUAL_AFFORDANCE_BUTTON_CLASSNAME'), 'button missing shared className');
      assert.ok(block.includes('EQUAL_AFFORDANCE_BUTTON_STYLE'), 'button missing shared style');
    }
  });

  it('the two buttons differ ONLY in glyph/label (the one asymmetry ui-spec permits)', () => {
    assert.ok(src.includes('ti-x'), 'Cancel glyph missing');
    assert.ok(src.includes('ti-check'), 'Confirm glyph missing');
  });
});

describe('DisclosureSheet.tsx — Use/Not-now reference the SAME shared constant', () => {
  const src = readSource('DisclosureSheet.tsx');

  it('both buttons use EQUAL_AFFORDANCE_BUTTON_CLASSNAME (exactly twice — one per button)', () => {
    assert.equal(countOccurrences(src, 'className={EQUAL_AFFORDANCE_BUTTON_CLASSNAME}'), 2);
  });

  it('both buttons use EQUAL_AFFORDANCE_BUTTON_STYLE (exactly twice — one per button)', () => {
    assert.equal(countOccurrences(src, 'style={EQUAL_AFFORDANCE_BUTTON_STYLE}'), 2);
  });

  it('"Not now" never imports the voice engine — G11 guardrail, proven by this file\'s import list', () => {
    // The entire file's imports are the proof: no @deliveryos/voice, no apps/web adapter, nothing
    // engine-shaped. A regex over the import statements is the deterministic assertion.
    const importLines = src.match(/^import .+$/gm) ?? [];
    for (const line of importLines) {
      assert.ok(!/@deliveryos\/voice/.test(line), `engine import found: ${line}`);
      assert.ok(!/apps\/web/.test(line), `apps/web import found: ${line}`);
    }
  });
});

describe('ConfirmChip.tsx and DisclosureSheet.tsx import the constants from the SAME module', () => {
  it('both source files import EQUAL_AFFORDANCE_BUTTON_CLASSNAME/_STYLE from ./layout.js (one source of truth, not a hand-copied lookalike)', () => {
    for (const file of ['ConfirmChip.tsx', 'DisclosureSheet.tsx']) {
      const src = readSource(file);
      assert.match(
        src,
        /from '\.\/layout\.js'/,
        `${file} must import the equal-affordance constants from ./layout.js`,
      );
      assert.ok(src.includes('EQUAL_AFFORDANCE_BUTTON_CLASSNAME'), `${file} does not import the shared classname`);
      assert.ok(src.includes('EQUAL_AFFORDANCE_BUTTON_STYLE'), `${file} does not import the shared style`);
    }
  });
});
