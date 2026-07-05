import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { makeEmptyProductDraft, productDraftsEqual, type ProductFormDraft } from './MenuManagerPage.js';

// GUARDRAIL — UNDO/REDO for the menu-item edit draft (client buffer BEFORE save).
// The history reducer itself is unit-tested in packages/ui (use-history-stack.test.ts);
// this file guards the MenuManagerPage WIRING. This repo's test runner (tsx --test)
// has no jsdom, so it takes two arms (same shape as menu-manager-audit-fixes.test.ts):
//   1. REAL unit tests of the exported pure draft helpers;
//   2. anchored source-content checks that the page keeps the undo/redo wiring:
//      flag-gated UI, keyboard shortcuts, per-open history reset, snapshot effect,
//      and — critically — an UNCHANGED save contract (undo must stay client-only).
// DOM-level behavior (clicking the buttons actually reverts a field) is Playwright's
// job: e2e/tests/undo-redo.spec.ts.

const DIR = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(DIR, 'MenuManagerPage.tsx'), 'utf8');

function windowAfter(startAnchor: string, endAnchor: string): string {
  const start = SRC.indexOf(startAnchor);
  assert.ok(start >= 0, `anchor not found in MenuManagerPage.tsx: "${startAnchor}"`);
  const end = SRC.indexOf(endAnchor, start);
  assert.ok(end > start, `end anchor "${endAnchor}" not found after "${startAnchor}"`);
  return SRC.slice(start, end);
}

// --- Arm 1: pure draft helpers -----------------------------------------------------

test('makeEmptyProductDraft: returns a FRESH object each call (no shared mutable aliases)', () => {
  const a = makeEmptyProductDraft();
  const b = makeEmptyProductDraft();
  assert.notEqual(a, b);
  assert.notEqual(a.taste, b.taste, 'nested taste object must not be shared');
  assert.notEqual(a.recipeLines, b.recipeLines, 'nested recipeLines array must not be shared');
  assert.notEqual(a.declaredAllergens, b.declaredAllergens, 'nested declaredAllergens array must not be shared');
  assert.equal(a.available, true);
  assert.equal(a.prepTime, '15', 'matches the form default prep time');
});

test('productDraftsEqual: structural — equal values compare equal across different references', () => {
  const a: ProductFormDraft = { ...makeEmptyProductDraft(), name: 'Pizza', recipeLines: [], taste: { spicy: 2 } };
  const b: ProductFormDraft = { ...makeEmptyProductDraft(), name: 'Pizza', recipeLines: [], taste: { spicy: 2 } };
  assert.equal(productDraftsEqual(a, b), true, 'value-equal drafts must dedupe (else every render is a history entry)');
  assert.equal(productDraftsEqual(a, { ...b, name: 'Pasta' }), false);
  assert.equal(productDraftsEqual(a, { ...b, taste: { spicy: 3 } }), false);
  assert.equal(productDraftsEqual(a, { ...b, declaredAllergens: ['gluten'] }), false);
});

// --- Arm 2: source-content regression (page wiring) --------------------------------

test('imports useHistoryStack + UndoRedoButtons from @deliveryos/ui', () => {
  assert.match(
    SRC,
    /import\s*\{[^}]*\buseHistoryStack\b[^}]*\}\s*from\s*'@deliveryos\/ui'/,
    'useHistoryStack must come from the shared ui package',
  );
  assert.match(
    SRC,
    /import\s*\{[^}]*\bUndoRedoButtons\b[^}]*\}\s*from\s*'@deliveryos\/ui'/,
    'UndoRedoButtons must come from the shared ui package',
  );
});

test('feature flag: VITE_UNDO_REDO_ENABLED default-ON shape, and the UI render is gated on it', () => {
  assert.match(
    SRC,
    /UNDO_REDO_ENABLED\s*=\s*import\.meta\.env\?\.VITE_UNDO_REDO_ENABLED\s*!==\s*'false'/,
    "flag must be default ON (!== 'false'), disabled only by an explicit VITE_UNDO_REDO_ENABLED=false",
  );
  assert.match(
    SRC,
    /\{UNDO_REDO_ENABLED\s*&&[\s\S]{0,300}<UndoRedoButtons\b/,
    'the <UndoRedoButtons> render must be inside a {UNDO_REDO_ENABLED && ...} gate',
  );
});

test('keyboard shortcuts: Cmd/Ctrl+Z undo, Shift variant or Ctrl+Y redo, with preventDefault, gated on flag+form-open', () => {
  const block = windowAfter('const onKey = (e: KeyboardEvent)', 'removeEventListener');
  assert.match(block, /e\.metaKey\s*\|\|\s*e\.ctrlKey/, 'must accept both Cmd (mac) and Ctrl');
  assert.match(block, /'z'\s*&&\s*!e\.shiftKey/, 'plain mod+Z is undo');
  assert.match(block, /'z'\s*&&\s*e\.shiftKey\)\s*\|\|\s*key\s*===\s*'y'/, 'mod+Shift+Z or mod+Y is redo');
  const occurrences = block.match(/e\.preventDefault\(\)/g) ?? [];
  assert.ok(occurrences.length >= 2, 'both branches must preventDefault so native input undo does not double-fire');
  const effectGate = windowAfter('// Cmd/Ctrl+Z = undo', 'const onKey');
  assert.match(effectGate, /if\s*\(!UNDO_REDO_ENABLED\s*\|\|\s*!showForm\)\s*return/, 'shortcut listener must be gated on the flag AND the form being open');
});

test('history baseline: BOTH openAddForm and openEditForm reset the history to the opening draft', () => {
  const add = windowAfter('const openAddForm', 'const openEditForm');
  assert.match(add, /resetDraftHistory\(/, 'openAddForm must reset history (else undo bleeds across form sessions)');
  const edit = windowAfter('const openEditForm', 'const closeForm');
  assert.match(edit, /resetDraftHistory\(/, 'openEditForm must reset history to the loaded product baseline');
});

test('snapshot effect: every draft field feeds pushDraft (a dropped field would silently become un-undoable)', () => {
  const eff = windowAfter('pushDraft({', '});');
  for (const field of ['name:', 'price:', 'desc:', 'available:', 'stock:', 'prepTime:', 'taste:', 'recipeLines:', 'allergenStatus:', 'declaredAllergens:']) {
    assert.ok(eff.includes(field), `snapshot must include draft field "${field.replace(':', '')}"`);
  }
});

test('save contract untouched: undo/redo is client-draft only — the product save endpoints are unchanged', () => {
  assert.ok(SRC.includes('`/owner/menu/products/${editingProduct.id}`'), 'PATCH endpoint for edit must remain');
  assert.ok(SRC.includes("'/owner/menu/products'"), 'POST endpoint for create must remain');
  const save = windowAfter('const handleSaveProduct', 'const handleImageSelect');
  assert.equal(/undoDraft|redoDraft|resetDraftHistory|pushDraft/.test(save), false,
    'handleSaveProduct must not consult or mutate the history — saving and undo are independent concerns');
});

// --- Red arm: prove the detectors are sharp, not vacuously true ---------------------

test('sharp arm: the detectors correctly REJECT unwired shapes', () => {
  // Flag detector rejects a default-OFF shape (=== 'true') — that would ship the feature dark.
  assert.equal(
    /UNDO_REDO_ENABLED\s*=\s*import\.meta\.env\?\.VITE_UNDO_REDO_ENABLED\s*!==\s*'false'/.test(
      "const UNDO_REDO_ENABLED = import.meta.env?.VITE_UNDO_REDO_ENABLED === 'true';",
    ),
    false,
  );
  // Keyboard detector rejects an undo-only handler with no redo branch.
  const undoOnly = "const onKey = (e: KeyboardEvent) => { if ((e.metaKey || e.ctrlKey) && e.key === 'z') { e.preventDefault(); } }; removeEventListener";
  assert.equal(/'z'\s*&&\s*e\.shiftKey\)\s*\|\|\s*key\s*===\s*'y'/.test(undoOnly), false);
  // Gate detector rejects an ungated render.
  assert.equal(/\{UNDO_REDO_ENABLED\s*&&[\s\S]{0,300}<UndoRedoButtons\b/.test('<div><UndoRedoButtons canUndo /></div>'), false);
});
