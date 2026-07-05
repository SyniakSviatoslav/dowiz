import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { replaceProductInCategories } from './MenuManagerPage.js';
import type { Category, Product } from '../../hooks/useMenuData.js';

// GUARDRAIL — docs/design-review/audit-frontend-2026-07-03.md, MenuManagerPage.tsx lane.
//   S4 (silent mutation failures): a toggle/save/delete failed silently (console-only)
//     while the UI kept showing the optimistic "success" state.
//   S5 (no focus trap on any hand-rolled modal): `fixed inset-0` overlays had
//     aria-modal="true" with no real Tab-trap/Escape/restore-focus behavior.
//
// This repo's test runner (tsx --test) has no jsdom/testing-library, so this file
// takes two arms instead of a DOM render:
//   1. a REAL unit test of the extracted pure rollback-merge helper, run with plain
//      objects (no DOM needed for this part — it's pure data transformation);
//   2. a source-content check (readFileSync + anchored slices, not a full parse)
//      that the 3 modals now route through the shared ResponsiveDialog primitive
//      and the 3 previously-silent catch blocks now call showToast. The DOM-level
//      behavior (actual focus trap / Tab-cycling / Escape) is Playwright's job.

const DIR = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(DIR, 'MenuManagerPage.tsx'), 'utf8');

/** Slice the source between two anchors (both must appear, endAnchor after startAnchor). */
function windowAfter(startAnchor: string, endAnchor: string): string {
  const start = SRC.indexOf(startAnchor);
  assert.ok(start >= 0, `anchor not found in MenuManagerPage.tsx: "${startAnchor}"`);
  const end = SRC.indexOf(endAnchor, start);
  assert.ok(end > start, `end anchor "${endAnchor}" not found after "${startAnchor}"`);
  return SRC.slice(start, end);
}

function product(id: string, available: boolean): Product {
  return { id, name: id, price: 100, available, categoryId: 'c1' };
}
function category(id: string, products: Product[]): Category {
  return { id, name: id, products };
}

// --- Arm 1: pure logic (replaceProductInCategories) -------------------------------

test('replaceProductInCategories: replaces only the targeted product, leaves siblings/other categories untouched', () => {
  const before: Category[] = [
    category('c1', [product('p1', true), product('p2', true)]),
    category('c2', [product('p3', true)]),
  ];
  const after = replaceProductInCategories(before, 'c1', 'p1', product('p1', false));
  assert.equal(after[0]!.products![0]!.available, false, 'targeted product flipped');
  assert.equal(after[0]!.products![1]!.available, true, 'sibling product untouched');
  assert.equal(after[1]!.products![0]!.available, true, 'other category untouched');
  assert.notEqual(after, before, 'returns a new array (immutable update, safe for React state)');
});

test('replaceProductInCategories: rollback is the exact inverse of the optimistic apply (this is what handleToggleAvailable does on a failed PATCH)', () => {
  const original = product('p1', true);
  const optimistic = product('p1', false);
  const categories: Category[] = [category('c1', [original])];
  const applied = replaceProductInCategories(categories, 'c1', 'p1', optimistic);
  assert.equal(applied[0]!.products![0]!.available, false, 'optimistic flip applied');
  const rolledBack = replaceProductInCategories(applied, 'c1', 'p1', original);
  assert.deepEqual(rolledBack, categories, 'rollback restores the exact pre-toggle state');
});

// --- Arm 2: source-content regression (S5 modal migration + S4 toast wiring) ------

test('S5: no raw fixed-inset-0-z-50 modal wrapper remains (all 3 migrated to ResponsiveDialog)', () => {
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
  assert.equal(
    usages.length,
    3,
    `expected exactly 3 <ResponsiveDialog> usages (product preview, add/edit form, PDF import); found ${usages.length}`,
  );
});

test('S4: kitchen-busy toggle catch block now toasts on failure (was console-only)', () => {
  const block = windowAfter('const toggle = async', 'return (');
  assert.match(block, /catch\s*\(err\)\s*\{[\s\S]*showToast\(/, 'the kitchen-busy PATCH catch must call showToast, not just console.*');
});

test('S4: handleToggleAvailable catch block rolls back the optimistic flip AND toasts', () => {
  const block = windowAfter('const handleToggleAvailable = async', 'const handleAddCategory');
  assert.match(block, /catch\s*\(err\)\s*\{[\s\S]*showToast\(/, 'must call showToast on PATCH failure');
  assert.match(
    block,
    /replaceProductInCategories\(prev,\s*catId,\s*product\.id,\s*product\)/,
    'must revert to the pre-toggle `product` snapshot, not leave the flipped optimistic state showing',
  );
});

test('S4: schedule removeSchedule catch block now toasts on failure (was a bare-ignore catch)', () => {
  const block = windowAfter('const removeSchedule = async', 'const fmt = (min');
  assert.match(block, /catch\s*\(err\)\s*\{[\s\S]*showToast\(/, 'the schedule-delete catch must call showToast, not swallow the error silently');
});

// --- Red arm: prove the detectors above are sharp, not vacuously true -------------

test('sharp arm: the catch-block detector correctly REJECTS the old silent-catch shapes', () => {
  const silentIgnore = `const removeSchedule = async (id) => {\n    try {\n      await apiClient(x);\n    } catch { /* ignore */ }\n  };\n  const fmt = (min`;
  assert.equal(
    /catch\s*\(err\)\s*\{[\s\S]*showToast\(/.test(silentIgnore),
    false,
    'a bare `catch { /* ignore */ }` (no bound err, no showToast) must NOT satisfy the detector',
  );

  const consoleOnly = `const toggle = async () => {\n    try {\n      await apiClient(x);\n    } catch (err) {\n      console.debug(err);\n    } finally {}\n  };\n  return (`;
  assert.equal(
    /catch\s*\(err\)\s*\{[\s\S]*showToast\(/.test(consoleOnly),
    false,
    'a console-only catch must NOT satisfy the showToast detector',
  );

  const toastNoRollback = `const handleToggleAvailable = async (catId, product) => {\n    try {\n      await apiClient(x);\n    } catch (err) {\n      showToast('failed', 'error');\n    }\n  };\n  const handleAddCategory`;
  assert.equal(
    /replaceProductInCategories\(prev,\s*catId,\s*product\.id,\s*product\)/.test(toastNoRollback),
    false,
    'showToast alone without the rollback call must NOT satisfy the rollback detector',
  );

  const fixed = `const toggle = async () => {\n    try {\n      await apiClient(x);\n    } catch (err) {\n      showToast('failed', 'error');\n    } finally {}\n  };\n  return (`;
  assert.match(fixed, /catch\s*\(err\)\s*\{[\s\S]*showToast\(/, 'the detector passes the fixed (toast-on-catch) shape');
});
