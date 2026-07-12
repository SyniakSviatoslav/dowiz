// Serves the React SPA (apps/web, built to public/index.html) as the canonical
// storefront for /s/:slug/* human requests. The CSP mirrors the /branding-preview
// route, which is empirically proven to run the SPA (Vite ESM + framer-motion +
// Tabler icons + map tiles + Plausible). The global CSP (security/headers.ts)
// would break the SPA (no 'unsafe-eval'/jsdelivr), so we set this explicitly —
// the global onSend only fills in a CSP when none is present.

// Crawlers/social scrapers that don't execute JS: serve them the SSR menu so SEO
// (JSON-LD/OG) is preserved. Everyone else gets the SPA.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const BOT_UA = /bot|crawl|spider|slurp|mediapartners|facebookexternalhit|embedly|quora|pinterest|whatsapp|telegram|slackbot|twitter|linkedinbot|discord|google|bing|yandex|baidu|duckduck|applebot|petalbot|semrush|ahrefs/i;

export function isBot(ua: string | undefined | null): boolean {
  return BOT_UA.test(String(ua || ''));
}

// P1-SEO: escape values before interpolating into HTML attributes / text. The SPA
// shell is hand-templated (not preact), so unlike ssr-renderer we must escape here.
// Covers both attribute (") and element (<,>,&) contexts.
export function escapeHtml(s: string): string {
  return String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

// The built SPA shell lives next to fastify-static's root. The relative depth
// differs between the source layout (apps/api/src/lib → ../../public) and the
// bundled deploy (dist/api/server.cjs → ../public, where index.html is at
// dist/public). The old single hard-coded '../../public' resolved to /app/public
// in the bundle — which doesn't exist — so readShell threw and every human page
// load fell back to the generic <title>Dowiz</title> shell. Try candidates and
// cache the first that exists, mirroring server.ts's __dirname-first resolution.
const _here = typeof __dirname !== 'undefined' ? __dirname : path.dirname(fileURLToPath(import.meta.url));
const SHELL_CANDIDATES = [
  path.join(_here, '..', 'public', 'index.html'),         // bundled: dist/api → dist/public
  path.join(_here, '..', '..', 'public', 'index.html'),   // source:  apps/api/src/lib → apps/api/public
  path.join(process.cwd(), 'dist', 'public', 'index.html'),
  path.join(process.cwd(), 'public', 'index.html'),
];
let _shellCache: string | null = null;
function readShell(): string {
  if (_shellCache != null) return _shellCache;
  for (const p of SHELL_CANDIDATES) {
    try { _shellCache = fs.readFileSync(p, 'utf8'); return _shellCache; } catch { /* try next */ }
  }
  throw new Error('SPA shell index.html not found in: ' + SHELL_CANDIDATES.join(', '));
}

interface TenantMeta {
  name: string;
  address: string | null;
  logoUrl: string | null;
  slug: string;
}

// Build per-tenant <title> + OG/Twitter tags, mirroring ssr-renderer's OgMetaTags
// so the SPA shell and the bot-SSR page advertise identical metadata.
export function buildTenantMeta(m: TenantMeta): string {
  const baseUrl = process.env.APP_BASE_URL || 'https://dowiz.fly.dev';
  const title = `${m.name} — Order Online | Dowiz`;
  const desc = m.address
    ? `Order delivery from ${m.name} at ${m.address}. View menu, prices, and place your order online.`
    : `Order delivery from ${m.name}. View menu, prices, and place your order online.`;
  const url = `${baseUrl}/s/${m.slug}`;

  const tags = [
    `<title>${escapeHtml(title)}</title>`,
    `<meta name="description" content="${escapeHtml(desc)}" />`,
    `<meta property="og:title" content="${escapeHtml(title)}" />`,
    `<meta property="og:description" content="${escapeHtml(desc)}" />`,
    `<meta property="og:url" content="${escapeHtml(url)}" />`,
    `<meta property="og:type" content="website" />`,
    `<meta property="og:site_name" content="Dowiz" />`,
    `<meta property="og:locale" content="sq_AL" />`,
    `<meta name="twitter:card" content="summary_large_image" />`,
    `<meta name="twitter:title" content="${escapeHtml(title)}" />`,
    `<meta name="twitter:description" content="${escapeHtml(desc)}" />`,
  ];
  if (m.logoUrl) {
    tags.push(`<meta property="og:image" content="${escapeHtml(m.logoUrl)}" />`);
    tags.push(`<meta name="twitter:image" content="${escapeHtml(m.logoUrl)}" />`);
  }
  return tags.join('\n    ');
}

// Inject tenant meta into the shell: replace the static <title>Dowiz</title> with the
// per-tenant tag block. If the marker is absent (shell changed), fall back to inserting
// before </head> so we never lose the SPA.
export function injectTenantMeta(shell: string, meta: string): string {
  if (shell.includes('<title>Dowiz</title>')) {
    return shell.replace('<title>Dowiz</title>', meta);
  }
  return shell.replace('</head>', `    ${meta}\n  </head>`);
}

/**
 * Serve the SPA shell with the CSP it needs. `frameAncestors` is the per-location
 * embed policy (default 'self'); pass location_themes.frame_ancestors to permit
 * widget embedding on restaurant sites.
 */
export async function serveSpaShell(reply: any, db: any, slug: string): Promise<any> {
  let frameAncestors = "'self'";
  let tenant: TenantMeta | null = null;
  try {
    // LEFT JOIN: a location may not have a theme row yet; still want name/address.
    const res = await db.query(
      `SELECT l.name, l.address, lt.frame_ancestors, lt.logo_url
         FROM locations l LEFT JOIN location_themes lt ON lt.location_id = l.id
        WHERE l.slug = $1 LIMIT 1`,
      [slug],
    );
    const row = res.rows[0];
    if (row) {
      const fa = row.frame_ancestors;
      if (Array.isArray(fa) && fa.length) frameAncestors = fa.join(' ');
      if (row.name) {
        tenant = { name: row.name, address: row.address ?? null, logoUrl: row.logo_url ?? null, slug };
      }
    }
  } catch (err: any) {
    console.debug('[spa-shell] tenant lookup failed:', err?.message);
  }

  let r2ImgSrc = '';
  const r2PublicUrl = process.env.R2_PUBLIC_URL;
  if (r2PublicUrl) {
    try { r2ImgSrc = ' ' + new URL(r2PublicUrl).origin; } catch { /* ignore */ }
  }

  const csp = `default-src 'self'; img-src 'self' data: https:${r2ImgSrc}; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com https://plausible.io; worker-src 'self' blob:; connect-src 'self' https://cdn.jsdelivr.net https://tiles.openfreemap.org https://router.project-osrm.org https://en.wikipedia.org https://plausible.io; frame-ancestors ${frameAncestors}`;

  reply.header('Content-Security-Policy', csp);
  reply.header('Cache-Control', 'no-cache, no-store, must-revalidate');
  if (frameAncestors !== "'self'") {
    try { reply.raw.removeHeader('X-Frame-Options'); } catch { /* ignore */ }
  }

  // P1-SEO: inject per-tenant <title> + OG tags so link unfurls / search snippets
  // for /s/:slug carry the restaurant identity, not the generic "Dowiz" shell.
  // On any failure, fall back to the static shell so the SPA always boots.
  if (tenant) {
    try {
      const html = injectTenantMeta(readShell(), buildTenantMeta(tenant));
      return reply.type('text/html').send(html);
    } catch (err: any) {
      console.debug('[spa-shell] meta injection failed, serving static shell:', err?.message);
    }
  }
  return reply.sendFile('index.html');
}
