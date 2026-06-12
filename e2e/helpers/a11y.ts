import type { Page } from '@playwright/test';

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

export async function checkTouchTargets(page: Page): Promise<{ tooSmall: number; tooClose: number }> {
  return page.evaluate(() => {
    const buttons = document.querySelectorAll('button, a, [role="button"], input, select, textarea');
    let tooSmall = 0;
    let tooClose = 0;
    const MIN_SIZE = 44;
    for (const btn of buttons) {
      const rect = btn.getBoundingClientRect();
      if (rect.width < MIN_SIZE || rect.height < MIN_SIZE) tooSmall++;
      // Check proximity to other targets
      for (const other of buttons) {
        if (other === btn) continue;
        const oRect = other.getBoundingClientRect();
        const gapX = Math.max(0, oRect.left - rect.right, rect.left - oRect.right);
        const gapY = Math.max(0, oRect.top - rect.bottom, rect.top - oRect.bottom);
        if (gapX < 8 && gapX > 0 && gapY < 8 && gapY > 0) tooClose++;
      }
    }
    return { tooSmall, tooClose };
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

export async function checkAriaLive(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    return document.querySelectorAll('[aria-live], [role="alert"], [role="status"]').length > 0;
  });
}
