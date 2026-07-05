import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// GUARDRAIL — docs/design-review/audit-frontend-2026-07-03.md, PromotionsPage.tsx lane.
//   S4 (silent mutation failures): the toggle-active and delete handlers used to fail
//     silently (console-only) while the UI gave the owner no feedback that anything
//     had gone wrong.
//   S5 (no focus trap on any hand-rolled modal): the edit-promotion modal was a
//     hand-rolled `fixed inset-0 z-50` overlay with aria-modal="true" but no real
//     Tab-trap / Escape / restore-focus behavior.
//
// Neither handler was optimistic (state only changes after the request resolves), so
// there is no rollback logic to extract into a pure helper here — unlike
// menu-manager-audit-fixes.test.ts's replaceProductInCategories arm. This repo's test
// runner (tsx --test) has no jsdom/testing-library (see AGENTS.md), so — mirroring
// packages/ui/src/theme/css-comment-integrity.test.ts and
// apps/web/src/pages/admin/menu-manager-audit-fixes.test.ts — this is a
// source-content regression test: readFileSync + anchored slices + regex, plus a red
// arm proving the detectors actually detect the bug shape (not vacuously true). The
// DOM-level behavior (actual focus trap / Tab-cycling / Escape) is Playwright's job.

const DIR = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(DIR, 'PromotionsPage.tsx'), 'utf8');

/** Slice the source between two anchors (both must appear, endAnchor after startAnchor). */
function windowAfter(startAnchor: string, endAnchor: string): string {
  const start = SRC.indexOf(startAnchor);
  assert.ok(start >= 0, `anchor not found in PromotionsPage.tsx: "${startAnchor}"`);
  const end = SRC.indexOf(endAnchor, start);
  assert.ok(end > start, `end anchor "${endAnchor}" not found after "${startAnchor}"`);
  return SRC.slice(start, end);
}

// --- S5: modal migration -----------------------------------------------------------

test('S5: no raw fixed-inset-0-z-50 modal wrapper remains (edit modal migrated to ResponsiveDialog)', () => {
  assert.equal(
    SRC.includes('fixed inset-0 z-50'),
    false,
    'a hand-rolled modal overlay reappeared — route it through ResponsiveDialog (packages/ui/src/components/molecules/ResponsiveDialog.tsx) instead',
  );
  assert.match(
    SRC,
    /import\s*\{[^}]*\bResponsiveDialog\b[^}]*\}\s*from\s*'@deliveryos\/ui'/,
    'ResponsiveDialog must be imported from @deliveryos/ui',
  );
  const usages = SRC.match(/<ResponsiveDialog\b/g) ?? [];
  assert.equal(usages.length, 1, `expected exactly 1 <ResponsiveDialog> usage (the edit-promotion modal); found ${usages.length}`);
});

test('S5: the edit modal is keyed off `editing` state, clears it on close, and still renders PromotionForm', () => {
  const block = windowAfter('<ResponsiveDialog', '{rmDialog}');
  assert.match(block, /open=\{!!editing\}/, 'ResponsiveDialog must be open exactly when `editing` is set');
  assert.match(block, /onClose=\{\(\) => setEditing\(null\)\}/, 'closing the dialog must clear `editing`');
  assert.match(block, /<PromotionForm\s+initial=\{editing\}/, 'the same PromotionForm + initial={editing} must be rendered inside');
});

// --- S4: silent mutation failures ---------------------------------------------------

test('S4: handleToggleActive catch block now toasts on failure (was console-only)', () => {
  const block = windowAfter('const handleToggleActive = async', 'const handleDelete = async');
  assert.match(
    block,
    /catch\s*\(err[^)]*\)\s*\{[\s\S]*showToast\(/,
    'the toggle-active PATCH catch must call showToast, not just console.*',
  );
});

test('S4: handleDelete catch block now toasts on failure (was console-only)', () => {
  const block = windowAfter('const handleDelete = async', 'const now = new Date');
  assert.match(
    block,
    /catch\s*\(err[^)]*\)\s*\{[\s\S]*showToast\(/,
    'the delete catch must call showToast, not just console.*',
  );
});

test('useToast is wired up (import + hook call)', () => {
  assert.match(
    SRC,
    /import\s*\{[^}]*\buseToast\b[^}]*\}\s*from\s*'@deliveryos\/ui'/,
    'useToast must be imported from @deliveryos/ui',
  );
  assert.match(
    SRC,
    /const\s*\{\s*showToast\s*\}\s*=\s*useToast\(\)/,
    'useToast() must be called and destructured to showToast',
  );
});

// --- Red arm: prove the detectors above are sharp, not vacuously true --------------

test('sharp arm: the catch-block detector correctly REJECTS the old silent-catch shapes', () => {
  const consoleOnlyToggle = `const handleToggleActive = async (p) => {\n    try {\n      await apiClient(x);\n    } catch (err) {\n      console.error('[Promotions] toggle failed:', err);\n    }\n  };\n  const handleDelete = async`;
  assert.equal(
    /catch\s*\(err[^)]*\)\s*\{[\s\S]*showToast\(/.test(consoleOnlyToggle),
    false,
    'a console-only catch must NOT satisfy the showToast detector',
  );

  const consoleOnlyDelete = `const handleDelete = async (p) => {\n    try {\n      await apiClient(x);\n    } catch (err) {\n      console.error('[Promotions] delete failed:', err);\n    }\n  };\n  const now = new Date`;
  assert.equal(
    /catch\s*\(err[^)]*\)\s*\{[\s\S]*showToast\(/.test(consoleOnlyDelete),
    false,
    'a console-only catch must NOT satisfy the showToast detector',
  );

  const fixed = `const handleToggleActive = async (p) => {\n    try {\n      await apiClient(x);\n    } catch (err) {\n      showToast('failed', 'error');\n    }\n  };\n  const handleDelete = async`;
  assert.match(
    fixed,
    /catch\s*\(err[^)]*\)\s*\{[\s\S]*showToast\(/,
    'the detector passes the fixed (toast-on-catch) shape',
  );
});

test('sharp arm: the fixed-inset-0-z-50 detector correctly flags the old hand-rolled modal shape', () => {
  const bugged = `<div className="fixed inset-0 z-50 flex items-end sm:items-center justify-center" role="dialog" aria-modal="true">`;
  assert.equal(bugged.includes('fixed inset-0 z-50'), true, 'detector flags the old hand-rolled overlay shape');
});
