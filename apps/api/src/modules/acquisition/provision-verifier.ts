import type { Pool } from 'pg';
import { renderShadowPreview, type PreviewMenu } from '../../lib/preview-render.js';

// P6-6 — ProvisionVerifier. Before a shadow is offered for claim (VERIFIED), verify the EXTERNAL BOUNDARY:
// the exact HTML a visitor would get at /s/:slug must satisfy every binding invariant. The preview is
// server-rendered STATIC HTML (renderShadowPreview), so this server-side render-check verifies the real
// output deterministically — no browser needed. (The deploy-time Playwright spec
// e2e/tests/p6-provision-verify.spec.ts re-asserts the same invariants against the live staging URL.)

export interface VerifyChecks {
  served: boolean; // read_preview_menu returns the shadow (owner_id NULL + closed + published_at NULL)
  hasItems: boolean; // the menu actually has products
  banner: boolean; // honest "demo — not yet live" label renders
  noindex: boolean; // robots noindex present (unfurl ≠ search index)
  richOg: boolean; // real name IS in og:title + per-venue card is og:image (operator directive 2026-07-06)
  neverOrderable: boolean; // no cart/checkout/add-to-cart affordance
}
export interface VerifyResult {
  ok: boolean;
  checks: VerifyChecks;
  failed: string[];
}

/** Verify the rendered preview for a shadow slug against every invariant. Pure given the DB row. */
export async function verifyShadowPreview(pool: Pool, slug: string): Promise<VerifyResult> {
  const res = await pool.query('SELECT read_preview_menu($1) AS m', [slug]);
  const menu = (res.rows[0] as { m: PreviewMenu | null }).m;

  const checks: VerifyChecks = {
    served: !!menu,
    hasItems: false,
    banner: false,
    noindex: false,
    richOg: false,
    neverOrderable: false,
  };

  if (menu) {
    const baseUrl = process.env.APP_BASE_URL || 'https://dowiz.fly.dev';
    const html = renderShadowPreview(menu, { ogImageUrl: `${baseUrl}/og/${slug}.png`, baseUrl });
    const itemCount = (menu.categories ?? []).reduce((n, c) => n + (c.products?.length ?? 0), 0);
    checks.hasItems = itemCount > 0;
    checks.banner = /ende jo dyqan aktiv/i.test(html);
    checks.noindex = /<meta name="robots" content="noindex/i.test(html);
    // rich OG (operator directive 2026-07-06, replaces the old generic-OG/H3 gate): the real name IS in
    // og:title and the per-venue card IS the og:image, so a pasted link unfurls as a product card.
    const title = html.match(/<title>([^<]*)<\/title>/)?.[1] ?? '';
    const ogTitle = html.match(/<meta property="og:title" content="([^"]*)"/)?.[1] ?? '';
    const name = menu.name ?? '';
    checks.richOg = !!name && title.includes(name) && ogTitle.includes(name) && /<meta property="og:image"/.test(html);
    checks.neverOrderable = !/add to cart|checkout|\bcart\b/i.test(html);
  }

  const failed = (Object.keys(checks) as (keyof VerifyChecks)[]).filter((k) => !checks[k]);
  return { ok: failed.length === 0, checks, failed };
}
