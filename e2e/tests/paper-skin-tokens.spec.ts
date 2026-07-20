import { test, expect } from '@playwright/test';

// GUARDRAIL (regression ledger #13) — a CSS block comment whose prose contains the literal
// `*/` closes the comment early, and the browser then SILENTLY DROPS the rule that follows
// (error-recovery swallows it). This is invisible to grep/build/typecheck — only a live-DOM
// computed-style read catches it. It is exactly how the [data-skin="paper"] token block was
// dropped (commit a0a28c05 → fixed same commit). This test asserts, against the REAL
// tokens.css parsed by a real browser, that the paper scope still applies — and proves the
// check is sharp by showing the bugged shape drops the block (the red arm).
const TOKENS = 'packages/ui/src/theme/tokens.css';

// Pin light scheme — the aged-paper night variant deliberately re-maps these to dark values.
test.use({ colorScheme: 'light' });

test('paper-skin token block survives CSS parsing (no */-in-comment drop)', async ({ page }) => {
  await page.setContent('<div data-skin="paper" id="t"></div>');
  await page.addStyleTag({ path: TOKENS });
  const v = await page.evaluate(() => {
    const cs = getComputedStyle(document.getElementById('t')!);
    return { paperBg: cs.getPropertyValue('--paper-bg').trim(), brandBg: cs.getPropertyValue('--brand-bg').trim() };
  });
  // GREEN arm: the real file's [data-skin="paper"] rule applies on the element (cream paper).
  // (Chromium resolves var() within computed custom-property values, so --brand-bg reads the
  // re-mapped #F4ECDB, not the literal var(--paper-bg).)
  expect(v.paperBg, 'paper scope sets --paper-bg (block not dropped)').toBe('#F4ECDB');
  expect(v.brandBg, 'paper scope re-maps --brand-bg → cream').toBe('#F4ECDB');
});

test('the guardrail is sharp — a */-in-comment shape DOES drop the next rule (red arm)', async ({ page }) => {
  // A minimal reproduction of the bug class: a comment with `*/` in its prose, then a rule.
  const bugged = `
    /* note about --ink-*/--paper-* tokens that closes early */
    [data-skin="paper"] { --paper-bg: #F4ECDB; }
  `;
  await page.setContent('<div data-skin="paper" id="t"></div>');
  await page.addStyleTag({ content: bugged });
  const dropped = await page.evaluate(() =>
    getComputedStyle(document.getElementById('t')!).getPropertyValue('--paper-bg').trim(),
  );
  // The browser drops the [data-skin="paper"] rule → --paper-bg never set. If this ever STOPS
  // being empty, the bug class changed and the green-arm assertion above must be re-derived.
  expect(dropped, 'the bugged comment shape drops the following rule (proves the check bites)').toBe('');
});
