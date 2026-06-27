// Pure subdomain-routing resolver extracted from server.ts's onRequest hook.
// A tenant subdomain (margherita.dowiz.org) is rewritten internally to the
// storefront route /s/:slug, EXCEPT for reserved subdomains and any request
// that already targets an API/asset/app path. Kept side-effect-free (host + url
// in, rewritten url or null out) so the dense reserved-path boolean is testable;
// the hook just assigns request.raw.url when this returns non-null.

const RESERVED_SUBDOMAINS = ['www', 'api', 'app'];
const APP_PATH_PREFIXES = ['/api/', '/public/', '/s/', '/admin', '/courier', '/dashboard'];
// A path ending in a file extension (.css, .js, .png, ...) is a static asset, not a tenant route.
const ASSET_EXT_RE = /\.\w{2,5}(\?|$)/;

/**
 * @returns the internal storefront url (`/s/:slug` + preserved query) for a
 * tenant subdomain request, or null when the request must pass through unchanged.
 */
export function resolveSubdomainRewrite(hostname: string, url: string): string | null {
  const host = hostname.split(':')[0] ?? ''; // strip port
  if (!host.endsWith('dowiz.org')) return null;

  const parts = host.split('.');
  if (parts.length < 3) return null; // needs <slug>.dowiz.org

  const slug = parts[0] ?? '';
  if (RESERVED_SUBDOMAINS.includes(slug)) return null;
  if (APP_PATH_PREFIXES.some((p) => url.startsWith(p))) return null;
  if (ASSET_EXT_RE.test(url)) return null;

  // Preserve the query string; the base host only matters for URL parsing.
  const urlObj = new URL(url, `http://${hostname}`);
  return `/s/${slug}${urlObj.search}`;
}
