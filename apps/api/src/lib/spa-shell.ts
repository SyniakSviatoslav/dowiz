// Serves the React SPA (apps/web, built to public/index.html) as the canonical
// storefront for /s/:slug/* human requests. The CSP mirrors the /branding-preview
// route, which is empirically proven to run the SPA (Vite ESM + framer-motion +
// Tabler icons + map tiles + Plausible). The global CSP (security/headers.ts)
// would break the SPA (no 'unsafe-eval'/jsdelivr), so we set this explicitly —
// the global onSend only fills in a CSP when none is present.

// Crawlers/social scrapers that don't execute JS: serve them the SSR menu so SEO
// (JSON-LD/OG) is preserved. Everyone else gets the SPA.
export const BOT_UA = /bot|crawl|spider|slurp|mediapartners|facebookexternalhit|embedly|quora|pinterest|whatsapp|telegram|slackbot|twitter|linkedinbot|discord|google|bing|yandex|baidu|duckduck|applebot|petalbot|semrush|ahrefs/i;

export function isBot(ua: string | undefined | null): boolean {
  return BOT_UA.test(String(ua || ''));
}

/**
 * Serve the SPA shell with the CSP it needs. `frameAncestors` is the per-location
 * embed policy (default 'self'); pass location_themes.frame_ancestors to permit
 * widget embedding on restaurant sites.
 */
export async function serveSpaShell(reply: any, db: any, slug: string): Promise<any> {
  let frameAncestors = "'self'";
  try {
    const res = await db.query(
      `SELECT lt.frame_ancestors
         FROM locations l JOIN location_themes lt ON lt.location_id = l.id
        WHERE l.slug = $1 LIMIT 1`,
      [slug],
    );
    const fa = res.rows[0]?.frame_ancestors;
    if (Array.isArray(fa) && fa.length) frameAncestors = fa.join(' ');
  } catch (err: any) {
    console.debug('[spa-shell] frame_ancestors lookup failed:', err?.message);
  }

  let r2ImgSrc = '';
  const r2PublicUrl = process.env.R2_PUBLIC_URL;
  if (r2PublicUrl) {
    try { r2ImgSrc = ' ' + new URL(r2PublicUrl).origin; } catch { /* ignore */ }
  }

  const csp = `default-src 'self'; img-src 'self' data: https:${r2ImgSrc}; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdn.jsdelivr.net; font-src 'self' https://fonts.gstatic.com https://cdn.jsdelivr.net; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com https://plausible.io; worker-src 'self' blob:; connect-src 'self' https://cdn.jsdelivr.net https://tiles.openfreemap.org https://router.project-osrm.org https://en.wikipedia.org https://plausible.io; frame-ancestors ${frameAncestors}`;

  reply.header('Content-Security-Policy', csp);
  reply.header('Cache-Control', 'no-cache, no-store, must-revalidate');
  if (frameAncestors !== "'self'") {
    try { reply.raw.removeHeader('X-Frame-Options'); } catch { /* ignore */ }
  }
  return reply.sendFile('index.html');
}
