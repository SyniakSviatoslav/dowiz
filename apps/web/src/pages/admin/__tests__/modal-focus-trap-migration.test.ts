import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// GUARDRAIL — audit-frontend-2026-07-03.md systemic class S5 / findings #59.
// Source-content regression check (no jsdom/testing-library in this repo — see
// packages/ui/src/theme/css-comment-integrity.test.ts for the established pattern this
// mirrors). We can't render React or simulate Tab/Escape here, so instead we assert the
// STATIC SHAPE that guarantees the fix landed: the hand-rolled `fixed inset-0 z-50 ...
// role="dialog" aria-modal="true"` overlay (zero focus trap, divergent Escape/scroll-lock)
// is gone, and a `<ResponsiveDialog` usage — which DOES have a real focus trap, Escape,
// scroll-lock and focus restoration (packages/ui/src/components/molecules/ResponsiveDialog.tsx)
// — stands in its place, imported from '@deliveryos/ui'.
const ADMIN_DIR = join(dirname(fileURLToPath(import.meta.url)), '..');

const HAND_ROLLED_MODAL_RE = /fixed inset-0 z-50[^"]*"\s+role="dialog"\s+aria-modal="true"/;
const RESPONSIVE_DIALOG_USAGE_RE = /<ResponsiveDialog\b/;
const RESPONSIVE_DIALOG_IMPORT_RE = /import\s*\{[^}]*\bResponsiveDialog\b[^}]*\}\s*from\s*'@deliveryos\/ui'/;

function hasHandRolledModal(src: string): boolean {
  return HAND_ROLLED_MODAL_RE.test(src);
}
function usesResponsiveDialog(src: string): boolean {
  return RESPONSIVE_DIALOG_USAGE_RE.test(src) && RESPONSIVE_DIALOG_IMPORT_RE.test(src);
}

const targets = [
  { name: 'SupplyLibraryPage.tsx (audit #59 — edit-supply modal)', file: join(ADMIN_DIR, 'SupplyLibraryPage.tsx') },
  { name: 'CouriersPage.tsx (audit #59 — order-detail modal)', file: join(ADMIN_DIR, 'CouriersPage.tsx') },
];

for (const { name, file } of targets) {
  test(`${name}: hand-rolled fixed/inset-0 dialog overlay is gone, ResponsiveDialog is used`, () => {
    const src = readFileSync(file, 'utf8');
    assert.equal(
      hasHandRolledModal(src),
      false,
      `${file}: still contains the hand-rolled "fixed inset-0 z-50 ... role=dialog aria-modal=true" ` +
        `overlay shape — this had zero focus trap / no Escape (audit #59, S5).`,
    );
    assert.equal(
      usesResponsiveDialog(src),
      true,
      `${file}: expected a <ResponsiveDialog> usage (imported from '@deliveryos/ui') replacing the ` +
        `hand-rolled overlay — it supplies the real focus trap + Escape + scroll-lock + focus restoration.`,
    );
  });
}

test('detector is sharp — flags the exact pre-fix broken shape (red arm)', () => {
  const bugged = [
    '  return (',
    '    <div className="fixed inset-0 z-50 flex items-end sm:items-center justify-center fade-in" role="dialog" aria-modal="true">',
    '      <button type="button" className="absolute inset-0 bg-black/50" onClick={() => setEditing(null)} />',
    '      <div className="relative w-full max-w-lg"><SupplyForm /></div>',
    '    </div>',
    '  );',
  ].join('\n');
  assert.equal(hasHandRolledModal(bugged), true, 'detector flags the hand-rolled overlay shape');
  assert.equal(usesResponsiveDialog(bugged), false, 'detector finds no ResponsiveDialog usage in the broken shape');

  const fixed = [
    "import { Button, ResponsiveDialog } from '@deliveryos/ui';",
    '  return (',
    '    <ResponsiveDialog open={!!editing} onClose={() => setEditing(null)}>',
    '      <SupplyForm />',
    '    </ResponsiveDialog>',
    '  );',
  ].join('\n');
  assert.equal(hasHandRolledModal(fixed), false, 'detector passes the migrated shape (no hand-rolled overlay left)');
  assert.equal(usesResponsiveDialog(fixed), true, 'detector finds the ResponsiveDialog usage + import in the migrated shape');
});
