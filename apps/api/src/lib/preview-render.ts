// P6-3 (council D + H3) — the honest labeled preview render for a SHADOW tenant. Server-rendered for
// BOTH humans and bots (the React storefront can't load a shadow: read_public_menu returns null for it).
//
// Binding guards baked in here:
//  • H3 — GENERIC OG: the real restaurant name appears ONLY in the on-page body, NEVER in <title>/og:*
//    metadata, so a pasted /s/:slug link does NOT unfurl the unconsented venue's identity in chat.
//  • noindex (kept) + a server-authoritative banner "preview mockup built from your public site — not a
//    live store" + a kill/claim CTA. Never-orderable: there is no cart/checkout on this page at all.
//  • Full descriptions are rendered (operator decision D-render = FULL DESCRIPTIONS, recorded override).
//  • Allergens never reach here — read_preview_menu strips attributes.bom for unconfirmed place rows.

interface PreviewProduct {
  name: string;
  description?: string | null;
  price: number;
  is_available?: boolean;
}
interface PreviewCategory {
  name: string;
  products?: PreviewProduct[];
}
export interface PreviewMenu {
  slug: string;
  name: string;
  is_preview?: boolean;
  currency?: { code?: string; minor_unit?: number };
  categories?: PreviewCategory[];
}

function escapeHtml(s: unknown): string {
  return String(s ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function fmtPrice(minor: number, code: string, minorUnit: number): string {
  const value = minorUnit > 0 ? (minor / 10 ** minorUnit).toFixed(minorUnit) : String(minor);
  return `${value} ${escapeHtml(code)}`;
}

/** Render the labeled, never-orderable, noindex, generic-OG preview HTML for a shadow tenant. */
export function renderShadowPreview(menu: PreviewMenu): string {
  const code = menu.currency?.code ?? 'ALL';
  const minorUnit = menu.currency?.minor_unit ?? 0;
  const banner = 'This is a preview mockup built from this restaurant’s public website — it is NOT a live store and cannot take orders.';

  const sections = (menu.categories ?? [])
    .map((c) => {
      const items = (c.products ?? [])
        .map(
          (p) => `
        <li class="item">
          <div class="item-row"><span class="item-name">${escapeHtml(p.name)}</span>
            <span class="item-price">${fmtPrice(p.price, code, minorUnit)}</span></div>
          ${p.description ? `<p class="item-desc">${escapeHtml(p.description)}</p>` : ''}
        </li>`,
        )
        .join('');
      return `<section class="cat"><h2>${escapeHtml(c.name)}</h2><ul>${items}</ul></section>`;
    })
    .join('');

  // H3: title + OG are GENERIC — no real name. The real name is only in the page body, behind the banner.
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta name="robots" content="noindex, nofollow" />
  <title>Restaurant preview · Dowiz</title>
  <meta property="og:title" content="Restaurant menu preview · Dowiz" />
  <meta property="og:description" content="An unclaimed menu preview on Dowiz." />
  <meta property="og:type" content="website" />
  <style>
    body{font-family:system-ui,sans-serif;margin:0;color:#1a1a1a;background:#faf8f4}
    .banner{background:#fff3cd;border-bottom:1px solid #e6d8a8;padding:12px 16px;font-size:14px}
    .wrap{max-width:680px;margin:0 auto;padding:16px}
    h1{font-size:22px;margin:16px 0 4px}
    .cat h2{font-size:17px;border-bottom:1px solid #eee;padding-bottom:4px;margin-top:24px}
    ul{list-style:none;padding:0;margin:0}
    .item{padding:10px 0;border-bottom:1px solid #f0eee8}
    .item-row{display:flex;justify-content:space-between;gap:12px}
    .item-name{font-weight:600}.item-price{white-space:nowrap;color:#555}
    .item-desc{margin:4px 0 0;color:#666;font-size:14px}
    .cta{margin:24px 0;padding:14px;background:#fff;border:1px solid #e6e2da;border-radius:8px;text-align:center}
  </style>
</head>
<body>
  <div class="banner" role="note">${escapeHtml(banner)}</div>
  <div class="wrap">
    <h1>${escapeHtml(menu.name)}</h1>
    ${sections || '<p>No menu items available.</p>'}
    <div class="cta"><strong>Is this your restaurant?</strong><br/>You can claim this preview to edit or remove it.</div>
  </div>
</body>
</html>`;
}
