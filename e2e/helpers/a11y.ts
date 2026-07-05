import { expect, type Page } from '@playwright/test';

/**
 * Sense 1 · Accessibility tree gate (Non-Pixel Verification Net).
 *
 * 🔴 axe reads the COMPUTED a11y-tree (what a screen-reader gets) — never a
 * pixel. This is the authority for a11y verdicts; the vision layer no longer
 * scores C_a11y (see docs/operating-model/ui-build-verification-loop.md).
 *
 * Hard gate: zero WCAG 2.0/2.1 A+AA violations. `disableRules` is permitted
 * ONLY when a rule is genuinely wrong for the context — document why at the
 * call-site. Scan AFTER interactions too (open modal/drawer, hover, dark-mode):
 * the default scan misses interactive-state contrast/focus.
 */
export async function expectNoA11y(page: Page, disableRules: string[] = []): Promise<void> {
  const AxeBuilder = (await import('@axe-core/playwright')).default;
  const r = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
    .disableRules(disableRules)
    .analyze();
  const summary = r.violations.map(
    (v) => `${v.id} (${v.impact}, ${v.nodes.length}×): ${v.help}`,
  );
  expect(r.violations, `axe violations:\n${summary.join('\n')}`).toEqual([]);
}

export interface A11yIssue {
  id: string;
  impact: 'critical' | 'serious' | 'moderate' | 'minor';
  description: string;
  help: string;
  nodes: number;
  html?: string;
}

export async function checkAxe(page: Page): Promise<A11yIssue[]> {
  const AxeBuilder = (await import('@axe-core/playwright')).default;
  const results = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
    .analyze();
  return results.violations.map(v => ({
    id: v.id,
    impact: v.impact as A11yIssue['impact'],
    description: v.description,
    help: v.help,
    nodes: v.nodes.length,
    html: v.nodes[0]?.html?.substring(0, 120),
  }));
}

export function checkFocusPage(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const el = document.activeElement;
    return el !== null && el !== document.body && window.getComputedStyle(el as Element).outlineStyle !== 'none';
  });
}

export async function checkTouchTargets(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const buttons = document.querySelectorAll('button, a, [role="button"], input, select, textarea');
    const issues: string[] = [];
    const MIN_SIZE = 44;
    for (const btn of buttons) {
      const rect = btn.getBoundingClientRect();
      if (rect.width < MIN_SIZE || rect.height < MIN_SIZE) {
        issues.push(`size:${btn.tagName.toLowerCase()} ${Math.round(rect.width)}x${Math.round(rect.height)}`);
      }
    }
    for (const btn of buttons) {
      const rect = btn.getBoundingClientRect();
      for (const other of buttons) {
        if (other === btn) continue;
        const oRect = other.getBoundingClientRect();
        const gapX = Math.max(0, oRect.left - rect.right, rect.left - oRect.right);
        const gapY = Math.max(0, oRect.top - rect.bottom, rect.top - oRect.bottom);
        if (gapX < 8 && gapX > 0 && gapY < 8 && gapY > 0) {
          issues.push(`proximity:${btn.tagName.toLowerCase()}<->${other.tagName.toLowerCase()}`);
        }
      }
    }
    return issues;
  });
}

export async function checkFormLabels(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const issues: string[] = [];
    const inputs = document.querySelectorAll('input, select, textarea');
    for (const input of inputs) {
      const id = input.getAttribute('id');
      const label = id ? document.querySelector(`label[for="${id}"]`) : null;
      const ariaLabel = input.getAttribute('aria-label');
      if (!label && !ariaLabel) {
        const name = input.getAttribute('name') || input.getAttribute('placeholder') || '(unnamed)';
        issues.push(`Missing label for input: ${name}`);
      }
    }
    return issues;
  });
}

export async function checkAriaLive(page: Page): Promise<number> {
  return page.evaluate(() => {
    return document.querySelectorAll('[aria-live], [role="alert"], [role="status"]').length;
  });
}
