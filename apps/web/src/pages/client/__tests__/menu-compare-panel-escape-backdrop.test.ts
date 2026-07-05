import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// GUARDRAIL — audit-frontend-2026-07-03.md finding #64: MenuComparePanel's
// role="dialog" aria-modal sheet had no Escape key handling and a non-focusable div
// backdrop (keyboard close was ONLY the "Done" button). Source-content regression check
// (no jsdom/testing-library in this repo — mirrors packages/ui/src/theme/css-comment-integrity.test.ts).
//
// This panel was NOT migrated onto the shared ResponsiveDialog primitive: it is
// mounted/unmounted by MenuPage.tsx inside <AnimatePresence> and relies on framer-motion's
// `exit` prop for its slide-down close animation, which ResponsiveDialog does not support
// (it returns `null` the instant `open` flips false — no exit-animation hook). Migrating
// would have silently killed that animation, so the fix instead adds the missing pieces
// directly, mirroring ResponsiveDialog's own handleKeyDown/backdrop/scroll-lock patterns
// (packages/ui/src/components/molecules/ResponsiveDialog.tsx). NOTE: this means the panel
// still has NO Tab-focus trap — only approach (a), a full ResponsiveDialog migration, would
// close that remaining gap (see the remediation report).
const FILE = join(dirname(fileURLToPath(import.meta.url)), '..', 'MenuComparePanel.tsx');

function hasEscapeHandler(src: string): boolean {
  return /addEventListener\(\s*'keydown'/.test(src) && /key === 'Escape'/.test(src);
}
function hasKeyboardOperableBackdrop(src: string): boolean {
  // Mirrors ResponsiveDialog's backdrop: role="button" + tabIndex={0} + an onKeyDown
  // that closes on both Enter and Space.
  return (
    /role="button"\s+tabIndex=\{0\}/.test(src) &&
    /onKeyDown=\{[^}]*key === 'Enter'[^}]*key === ' '/.test(src)
  );
}
function hasScrollLock(src: string): boolean {
  return /document\.body\.style\.overflow\s*=\s*'hidden'/.test(src) && /document\.body\.style\.overflow\s*=\s*''/.test(src);
}
function usesResponsiveDialog(src: string): boolean {
  return /<ResponsiveDialog\b/.test(src);
}

test('MenuComparePanel.tsx (audit #64): has an Escape-key handler', () => {
  const src = readFileSync(FILE, 'utf8');
  assert.equal(hasEscapeHandler(src), true, 'expected a document keydown listener that closes on Escape');
});

test('MenuComparePanel.tsx (audit #64): backdrop is keyboard-operable (role=button + tabIndex + Enter/Space)', () => {
  const src = readFileSync(FILE, 'utf8');
  assert.equal(hasKeyboardOperableBackdrop(src), true, 'expected the backdrop to carry role="button" tabIndex={0} + an onKeyDown closing on Enter/Space');
});

test('MenuComparePanel.tsx (audit #64): body scroll is locked while open and restored on close', () => {
  const src = readFileSync(FILE, 'utf8');
  assert.equal(hasScrollLock(src), true, 'expected document.body.style.overflow to be set to hidden on open and reset ("") on cleanup');
});

test('MenuComparePanel.tsx (audit #64): documents the approach — bespoke fix, not a ResponsiveDialog migration', () => {
  const src = readFileSync(FILE, 'utf8');
  // Approach (b) was chosen (see file-header comment): this is a deliberate choice, not an
  // oversight — assert it explicitly so a future migration to approach (a) is a conscious,
  // visible change to this guardrail rather than a silent drift.
  assert.equal(usesResponsiveDialog(src), false, 'expected MenuComparePanel to remain a bespoke sheet (approach b) — if migrated onto ResponsiveDialog (approach a), update this guardrail accordingly');
});

test('detector is sharp — flags the exact pre-fix broken shape (red arm)', () => {
  const bugged = [
    '  return (',
    '    <motion.div',
    '      className="fixed inset-0 z-modal flex items-end justify-center"',
    '      onClick={onClose}',
    '      role="dialog" aria-modal="true" aria-label={t(\'compare.title\', \'Compare dishes\')}',
    '      data-testid="compare-panel"',
    '    >',
    '      <motion.div onClick={e => e.stopPropagation()}>',
    '        <button onClick={onClose}>Done</button>',
    '      </motion.div>',
    '    </motion.div>',
    '  );',
  ].join('\n');
  assert.equal(hasEscapeHandler(bugged), false, 'detector flags the missing Escape handler in the broken shape');
  assert.equal(hasKeyboardOperableBackdrop(bugged), false, 'detector flags the non-keyboard-operable backdrop (plain onClick div) in the broken shape');
  assert.equal(hasScrollLock(bugged), false, 'detector flags the missing scroll-lock in the broken shape');

  const fixed = [
    '  useEffect(() => {',
    "    const handleKeyDown = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose(); };",
    "    document.addEventListener('keydown', handleKeyDown);",
    "    document.body.style.overflow = 'hidden';",
    '    return () => {',
    "      document.removeEventListener('keydown', handleKeyDown);",
    "      document.body.style.overflow = '';",
    '    };',
    '  }, [onClose]);',
    '  return (',
    '    <motion.div',
    '      onClick={onClose}',
    '      role="button" tabIndex={0}',
    '      onKeyDown={(e) => { if (e.key === \'Enter\' || e.key === \' \') { e.preventDefault(); onClose(); } }}',
    '    >',
    '      <motion.div onClick={e => e.stopPropagation()} role="dialog" aria-modal="true">',
    '        <button onClick={onClose}>Done</button>',
    '      </motion.div>',
    '    </motion.div>',
    '  );',
  ].join('\n');
  assert.equal(hasEscapeHandler(fixed), true, 'detector passes the fixed shape (Escape handler present)');
  assert.equal(hasKeyboardOperableBackdrop(fixed), true, 'detector passes the fixed shape (keyboard-operable backdrop present)');
  assert.equal(hasScrollLock(fixed), true, 'detector passes the fixed shape (scroll-lock present)');
});
