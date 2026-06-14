# SSR Architecture and Edge Caching

DeliveryOS implements a robust, hyper-performant server-side rendering (SSR) strategy combined with aggressively immutable edge caching to deliver sub-50ms responses to end users.

## 1. Cache Key Strategy
The fundamental rule of our caching architecture is that **we do not use URL purges**.
Instead, we rely on a custom header injected by the origin server: `X-Menu-Version`.

The Cache Key at the edge (Cloudflare) is formulated as:
`hostname + uri + custom_header["X-Menu-Version"]`

When a restaurant owner updates a menu item, the `menu_versions` table is incremented atomically. The next request to `/s/:slug` goes to the origin (because the cache TTL might have expired, or `stale-while-revalidate` is running in the background), and the origin returns the new HTML along with the new `X-Menu-Version`. Because the `X-Menu-Version` header is part of the cache key, this automatically creates a cache miss and caches the new payload under the new version.

### Cloudflare Cache Rule Definition
```text
Match: hostname = dowiz.org AND starts_with(uri, "/s/")
Action: Cache eligible
Edge TTL: 86400 (24h)
Browser TTL: 60
Cache key: hostname + uri + custom_header["X-Menu-Version"]
```

## 2. All-Locales Payload
To prevent cache fragmentation and reduce origin load, the SSR payload includes **all** translations for the supported locales in a single HTML document. 

The HTML includes `data-text-[locale]` attributes for all dynamic strings. A tiny inline `<script>` toggle iterates through these elements and updates `textContent` purely on the client-side.
- **Zero additional network requests** when switching languages.
- **Progressive Enhancement**: Without JS, the site gracefully degrades and displays the menu in the `default_locale`.

## 3. SEO & JSON-LD
Every SSR response contains a dynamically constructed `application/ld+json` script block that maps the `CanonicalMenu` into strict `schema.org/Restaurant` and `schema.org/Menu` entities.
Hreflang tags are populated dynamically based on `locations.supported_locales`.
