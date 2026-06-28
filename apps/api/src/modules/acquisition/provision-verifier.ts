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
  banner: boolean; // honest "not a live store" label renders
  noindex: boolean; // robots noindex present
  genericOg: boolean; // real name is NOT in <title>/og:* (B2/H3 — no unfurl leak)
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
    genericOg: false,
    neverOrderable: false,
  };

  if (menu) {
    const html = renderShadowPreview(menu);
    const itemCount = (menu.categories ?? []).reduce((n, c) => n + (c.products?.length ?? 0), 0);
    checks.hasItems = itemCount > 0;
    checks.banner = /not a live store/i.test(html);
    checks.noindex = /<meta name="robots" content="noindex/i.test(html);
    // generic OG: the real name must NOT appear in <title> or any og:* metadata.
    const title = html.match(/<title>([^<]*)<\/title>/)?.[1] ?? '';
    const ogContents = [...html.matchAll(/<meta property="og:[^"]+" content="([^"]*)"/g)].map((m) => m[1] ?? '');
    const name = menu.name ?? '';
    checks.genericOg = !!name && !title.includes(name) && !ogContents.some((c) => c.includes(name));
    checks.neverOrderable = !/add to cart|checkout|\bcart\b/i.test(html);
  }

  const failed = (Object.keys(checks) as (keyof VerifyChecks)[]).filter((k) => !checks[k]);
  return { ok: failed.length === 0, checks, failed };
}
