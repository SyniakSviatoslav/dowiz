import { h } from 'preact';
import render from 'preact-render-to-string';
import htm from 'htm';
import { LRUCache } from 'lru-cache';
import { getImageUrl } from './image-url.js';
import { formatMoney, ensureCurrency as ensureCurr } from '@deliveryos/shared-types';

// The menu page is hydrated by its dedicated, stable client bundle
// (apps/api/src/client/menu/app.ts → built by build-client.js → served at
// /dist/menu/app.js), referenced directly in the body below — the same way the
// cart/checkout/status routes reference their bundles. A prior version scanned
// the Vite index.html for a script tag and returned '' in production, so the
// menu shipped with no script, never hydrated, and customers could not order.
const html = htm.bind(h);

interface MenuData {
  menu_version: number;
  default_locale: string;
  supported_locales: string[];
  currency: { code: string; minor_unit: number };
  location: {
    name: string;
    address: string | null;
    public_phone: string | null;
    hours: Record<string, { open: string; close: string }[]> | null;
    geo: { lat: number; lng: number } | null;
  };
  categories: CategoryData[];
}

interface CategoryData {
  id: string;
  sort_order: number;
  available_names: Record<string, string>;
  products: ProductData[];
}

interface ProductData {
  id: string;
  price: number;
  available: boolean;
  image_key: string | null;
  attributes: Record<string, any> | null;
  available_names: Record<string, string>;
  available_descriptions: Record<string, string>;
}

const cache = new LRUCache<string, { html: string; slug: string }>({
  max: 50,
  ttl: 60_000,
});

// NOTE: do NOT hand-escape values interpolated into the html`` (htm + preact)
// templates below — preact-render-to-string already escapes text and attribute
// interpolations. Manual escaping here caused double-encoding (e.g. a venue
// name "X & Y" rendered as "X &amp;amp; Y" in <title>/OG/body).

function getName(item: { available_names: Record<string, string> }, locale: string, fallback: string): string {
  return item.available_names?.[locale] || item.available_names?.['en'] || fallback;
}

function getDesc(item: { available_descriptions?: Record<string, string> }, locale: string): string | null {
  return item.available_descriptions?.[locale] || item.available_descriptions?.['en'] || null;
}

function buildJsonLd(menu: MenuData, slug: string, baseUrl: string): string {
  const loc = menu.location;
  const menuUrl = `${baseUrl}/s/${slug}`;
  const parts: any[] = [];

  parts.push({
    '@context': 'https://schema.org',
    '@type': 'Restaurant',
    name: loc.name,
    url: menuUrl,
    servesCuisine: 'Albanian',
    priceRange: '€',
    address: loc.address ? {
      '@type': 'PostalAddress',
      streetAddress: loc.address,
    } : undefined,
    geo: loc.geo ? {
      '@type': 'GeoCoordinates',
      latitude: loc.geo.lat,
      longitude: loc.geo.lng,
    } : undefined,
    openingHoursSpecification: loc.hours ? buildHours(loc.hours) : undefined,
  });

  const menuItems: any[] = [];
  for (const cat of menu.categories || []) {
    for (const prod of cat.products || []) {
      menuItems.push({
        '@type': 'MenuItem',
        name: getName(prod, menu.default_locale, 'Item'),
        description: getDesc(prod, menu.default_locale) || undefined,
        offers: {
          '@type': 'Offer',
          price: (prod.price / 100).toFixed(2),
          priceCurrency: menu.currency.code === 'EUR' ? 'EUR' : 'ALL',
        },
      });
    }
  }

  parts.push({
    '@context': 'https://schema.org',
    '@type': 'Menu',
    name: `${loc.name} Menu`,
    description: `Menu for ${loc.name}`,
    hasMenuItem: menuItems.slice(0, 30),
  });

  parts.push({
    '@context': 'https://schema.org',
    '@type': 'BreadcrumbList',
    itemListElement: [
      { '@type': 'ListItem', position: 1, name: 'Home', item: baseUrl },
      { '@type': 'ListItem', position: 2, name: loc.name, item: menuUrl },
    ],
  });

  if (menuItems.length > 2) {
    parts.push({
      '@context': 'https://schema.org',
      '@type': 'FAQPage',
      mainEntity: [
        {
          '@type': 'Question',
          name: `What are the delivery hours for ${loc.name}?`,
          acceptedAnswer: {
            '@type': 'Answer',
            text: `Check the ${loc.name} menu page for current operating hours and delivery availability.`,
          },
        },
        {
          '@type': 'Question',
          name: `What payment methods does ${loc.name} accept?`,
          acceptedAnswer: {
            '@type': 'Answer',
            text: `${loc.name} accepts cash on delivery.`,
          },
        },
      ],
    });
  }

  // Emitted raw (unescaped) into a <script> via dangerouslySetInnerHTML so the
  // JSON-LD stays valid/parseable. Escape <, >, & to \uXXXX (still valid JSON,
  // parses back to the same chars) to make a </script> breakout impossible.
  return JSON.stringify(parts.length === 1 ? parts[0] : parts)
    .replace(/</g, '\\u003c')
    .replace(/>/g, '\\u003e')
    .replace(/&/g, '\\u0026');
}

function buildHours(hours: Record<string, { open: string; close: string }[]>): any[] {
  const dayMap: Record<string, string> = {
    mon: 'Monday', tue: 'Tuesday', wed: 'Wednesday', thu: 'Thursday',
    fri: 'Friday', sat: 'Saturday', sun: 'Sunday',
  };
  const result: any[] = [];
  for (const [shortDay, periods] of Object.entries(hours)) {
    const dayName = dayMap[shortDay.toLowerCase()];
    if (!dayName) continue;
    for (const p of periods) {
      result.push({
        '@type': 'OpeningHoursSpecification',
        dayOfWeek: dayName,
        opens: p.open,
        closes: p.close,
      });
    }
  }
  return result;
}

function formatPrice(price: number, currencyCode: string, minorUnit: number): string {
  const actualPrice = minorUnit === 0 ? price : price / Math.pow(10, minorUnit);
  const cur = ensureCurr(currencyCode, 'ALL');
  return formatMoney(actualPrice, cur);
}

function ProductCard({ product, locale, currencyCode, minorUnit }: {
  product: ProductData;
  locale: string;
  currencyCode: string;
  minorUnit: number;
}) {
  const name = getName(product, locale, 'Product');
  const desc = getDesc(product, locale);
  const imgUrl = getImageUrl(product.image_key);
  const price = formatPrice(product.price, currencyCode, minorUnit);

  return html`
    <div class="product-card" data-product-id="${product.id}">
      ${imgUrl ? html`<img class="product-image" src="${imgUrl}" alt="${name}" loading="lazy" />` : html`<div class="product-image-placeholder"></div>`}
      <div class="product-info">
        <h3 class="product-name">${name}</h3>
        ${desc ? html`<p class="product-desc">${desc}</p>` : null}
        <div class="product-foot">
          <span class="product-price">${price}</span>
          <button class="product-add" aria-label="Add to cart" onclick=${`DowizMenu.addToCart(event, '${product.id}', ${Number(product.price) || 0})`}>+</button>
        </div>
      </div>
    </div>
  `;
}

function MenuSection({ category, locale, currencyCode, minorUnit }: {
  category: CategoryData;
  locale: string;
  currencyCode: string;
  minorUnit: number;
}) {
  const catName = getName(category, locale, 'Category');
  return html`
    <section class="menu-section" data-category-id="${category.id}">
      <h2 class="category-title">${catName}</h2>
      <div class="product-grid">
        ${category.products.map((p: ProductData) => html`<${ProductCard} product=${p} locale=${locale} currencyCode=${currencyCode} minorUnit=${minorUnit} />`)}
      </div>
    </section>
  `;
}

function HreflangLinks({ slug, supportedLocales, defaultLocale, baseUrl }: { slug: string; supportedLocales: string[]; defaultLocale: string; baseUrl: string }) {
  const links: any[] = [];
  for (const loc of supportedLocales) {
    links.push(html`<link rel="alternate" hreflang="${loc}" href="${baseUrl}/s/${slug}?locale=${loc}" />`);
  }
  links.push(html`<link rel="alternate" hreflang="x-default" href="${baseUrl}/s/${slug}" />`);
  return links;
}

function OgMetaTags({ loc, slug, baseUrl }: { loc: { name: string; address: string | null }; slug: string; baseUrl: string }) {
  const title = `${loc.name} — Order Online | Dowiz`;
  const desc = loc.address ? `Order delivery from ${loc.name} at ${loc.address}. View menu, prices, and place your order online.` : `Order delivery from ${loc.name}. View menu, prices, and place your order online.`;
  const url = `${baseUrl}/s/${slug}`;

  return [
    html`<meta property="og:title" content="${title}" />`,
    html`<meta property="og:description" content="${desc}" />`,
    html`<meta property="og:url" content="${url}" />`,
    html`<meta property="og:type" content="website" />`,
    html`<meta property="og:site_name" content="Dowiz" />`,
    html`<meta property="og:locale" content="sq_AL" />`,
    html`<meta name="twitter:card" content="summary_large_image" />`,
    html`<meta name="twitter:title" content="${title}" />`,
    html`<meta name="twitter:description" content="${desc}" />`,
  ];
}

export async function renderMenuPage(
  slug: string,
  pool: any,
  baseUrl?: string,
): Promise<string> {
  const appBase = baseUrl || process.env.APP_BASE_URL || 'https://dowiz.fly.dev';
  const cacheKey = `ssr:menu:${slug}`;
  const cached = cache.get(cacheKey);
  if (cached && cached.slug === slug) return cached.html;

  const client = await pool.connect();
  try {
    const locRes = await client.query(
      `SELECT l.id, l.name, l.slug, l.currency_code, l.currency_minor_unit, l.default_locale,
              l.supported_locales, l.address, l.public_phone, l.hours_json, l.geo, o.owner_id
       FROM locations l JOIN organizations o ON o.id = l.org_id
       WHERE l.slug = $1`,
      [slug],
    );
    if (locRes.rowCount === 0) {
      return `<html><body><h1>404</h1><p>Location not found</p></body></html>`;
    }

    const loc = locRes.rows[0];
    // P6-2 (breaker B2 / counsel C1): a shadow tenant (org.owner_id IS NULL) is unconsented — never
    // emit its real name/logo OG to crawlers/unfurlers. Explicit gate (not the menu RPC's accidental
    // filtering). The honest, labeled preview render is P6-3.
    if (loc.owner_id === null || loc.owner_id === undefined) {
      return `<html><head><meta name="robots" content="noindex, nofollow" /><title>Dowiz</title></head><body><h1>Not available</h1></body></html>`;
    }
    const menuRes = await client.query(
      `SELECT read_public_menu_all_locales($1) as menu`,
      [slug],
    );
    const menu: MenuData | null = menuRes.rows[0]?.menu;
    if (!menu) {
      return `<html><body><h1>404</h1><p>Menu not found</p></body></html>`;
    }

    menu.location = {
      name: loc.name,
      address: loc.address,
      public_phone: loc.public_phone,
      hours: loc.hours_json,
      geo: loc.geo,
    };

    const defaultLocale = menu.default_locale || 'sq';
    const supportedLocales = menu.supported_locales || ['sq', 'en'];
    const currencyCode = menu.currency?.code || loc.currency_code || 'ALL';
    const minorUnit = menu.currency?.minor_unit ?? loc.currency_minor_unit ?? 0;

    const jsonld = buildJsonLd(menu, slug, appBase);

    const initialData = {
      menu_version: menu.menu_version,
      default_locale: defaultLocale,
      supported_locales: supportedLocales,
      currency: { code: currencyCode, minor_unit: minorUnit },
      location: menu.location,
      categories: menu.categories,
    };

    const menuContent = menu.categories?.length
      ? menu.categories.map((cat: CategoryData) =>
          html`<${MenuSection} category=${cat} locale=${defaultLocale} currencyCode=${currencyCode} minorUnit=${minorUnit} />`
        )
      : html`<p class="empty-menu">Menu not available yet.</p>`;

    const title = `${loc.name} — Order Online | Dowiz`;
    const metaDesc = loc.address
      ? `Order delivery from ${loc.name} at ${loc.address}. View menu, prices, and place your order online.`
      : `Order delivery from ${loc.name}. View menu, prices, and place your order online.`;

    const vdom = html`
      <html lang="${defaultLocale}">
        <head>
          <meta charset="utf-8" />
          <meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover" />
          <meta name="theme-color" content="#ea4f16" />
          <meta name="apple-mobile-web-app-capable" content="yes" />
          <meta name="mobile-web-app-capable" content="yes" />
          <meta name="description" content="${metaDesc}" />
          <title>${title}</title>
          <${OgMetaTags} loc=${menu.location} slug=${slug} baseUrl=${appBase} />
          <meta property="og:image" content="${appBase}/og-image.png" />
          <${HreflangLinks} slug=${slug} supportedLocales=${supportedLocales} defaultLocale=${defaultLocale} baseUrl=${appBase} />
          <link rel="canonical" href="${appBase}/s/${slug}" />
          <meta name="dos-location-id" content="${loc.id}" />
          <meta name="dos-menu-version" content="${menu.menu_version}" />
          <link rel="manifest" href="/s/${slug}/manifest.webmanifest" />
          <link rel="apple-touch-icon" href="/apple-touch-icon.png" />
          <link rel="icon" type="image/png" href="/favicon.png" />
          <link rel="preconnect" href="https://fonts.googleapis.com" />
          <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
          <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=DM+Sans:wght@400;500;600;700&display=swap" rel="stylesheet" />
          <link rel="stylesheet" href="/dist/tabler/tabler-icons.min.css" />
          <script dangerouslySetInnerHTML=${{ __html: "try{var s=localStorage.getItem('dowiz-sunlight');if(s==='on'||(s!=='off'&&window.matchMedia&&matchMedia('(prefers-contrast: more)').matches))document.documentElement.setAttribute('data-sunlight','on');}catch(e){}" }}></script>
          <link rel="stylesheet" href="/public/locations/${loc.id}/theme.css" />
          <script type="application/ld+json" dangerouslySetInnerHTML=${{ __html: jsonld }}></script>
          <style>
            /* Colours flow through the per-tenant --brand-* tokens (loaded via
               the location theme.css below). Fallbacks equal the previous
               hardcoded values, so an un-themed render is byte-identical. */
            :root { color-scheme: dark; }
            *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
            body { font-family: Inter, DM Sans, system-ui, sans-serif; background: var(--brand-bg, #121212); color: var(--brand-text, #e0e0e0); line-height: 1.6; -webkit-font-smoothing: antialiased; }
            .container { max-width: 800px; margin: 0 auto; padding: 1rem; }
            header { display: flex; align-items: center; justify-content: space-between; padding: 1rem 0; border-bottom: 1px solid var(--brand-border, #2a2a2a); margin-bottom: 1.5rem; }
            header h1 { font-size: 1.5rem; font-weight: 700; color: var(--brand-text, #fff); }
            header .brand { display: flex; align-items: center; gap: 0.5rem; }
            header .brand svg { width: 28px; height: 28px; stroke: var(--brand-primary, #ea4f16); }
            .menu-section { margin-bottom: 2rem; }
            .category-title { font-size: 1.25rem; font-weight: 600; color: var(--brand-text, #fff); margin-bottom: 1rem; padding-bottom: 0.5rem; border-bottom: 2px solid var(--brand-primary, #ea4f16); display: inline-block; }
            .product-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 1rem; }
            .product-card { background: var(--brand-surface, #1e1e1e); border-radius: 12px; overflow: hidden; transition: transform 0.15s, box-shadow 0.15s; cursor: pointer; border: 1px solid var(--brand-border, #2a2a2a); display: flex; flex-direction: column; }
            .product-card:hover { transform: translateY(-2px); box-shadow: 0 4px 12px rgba(0,0,0,0.3); border-color: var(--brand-primary, #ea4f16); }
            .product-image { width: 100%; height: 160px; object-fit: cover; }
            .product-image-placeholder { width: 100%; height: 160px; background: linear-gradient(135deg, var(--brand-border, #2a2a2a) 0%, var(--brand-surface, #1e1e1e) 100%); }
            .product-info { padding: 0.85rem; display: flex; flex-direction: column; gap: 0.35rem; flex: 1; }
            .product-name { font-size: 1rem; font-weight: 600; color: var(--brand-text, #fff); }
            .product-desc { font-size: 0.8rem; color: var(--brand-text-muted, #999); line-height: 1.4; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
            .product-foot { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; margin-top: auto; padding-top: 0.35rem; }
            .product-price { font-size: 0.95rem; font-weight: 700; color: var(--brand-primary, #ea4f16); }
            .product-add { flex-shrink: 0; width: 32px; height: 32px; border: none; border-radius: 8px; background: var(--brand-primary, #ea4f16); color: #fff; font-size: 1.25rem; line-height: 1; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: transform 0.1s, background 0.15s; }
            .product-add:hover { background: var(--brand-primary-hover, #ff5a1f); }
            .product-add:active { transform: scale(0.9); }
            .cart-fab-wrapper { position: fixed; bottom: 1.25rem; right: 1.25rem; z-index: 50; }
            .cart-fab-wrapper.hidden { display: none; }
            .cart-fab { display: flex; align-items: center; gap: 0.5rem; background: var(--brand-primary, #ea4f16); color: #fff; text-decoration: none; padding: 0.85rem 1.25rem; border-radius: 999px; font-weight: 700; box-shadow: 0 6px 18px rgba(0,0,0,0.4); }
            .cart-bounce { animation: cartBounce 0.3s ease; }
            @keyframes cartBounce { 0%,100% { transform: scale(1); } 50% { transform: scale(1.15); } }
            .empty-menu { text-align: center; padding: 3rem 1rem; color: var(--brand-text-muted, #666); font-size: 1.1rem; }
            @media (max-width: 540px) { .product-grid { grid-template-columns: 1fr; } }
            footer { text-align: center; padding: 2rem 0; color: var(--brand-text-muted, #555); font-size: 0.8rem; border-top: 1px solid var(--brand-border, #2a2a2a); margin-top: 2rem; }
            .locale-switcher { display: flex; gap: 0.5rem; }
            .locale-switcher a { color: var(--brand-primary, #ea4f16); text-decoration: none; font-size: 0.85rem; font-weight: 500; padding: 0.25rem 0.5rem; border-radius: 4px; }
            .locale-switcher a:hover { background: var(--brand-primary-light, rgba(234,79,22,0.1)); }
          </style>
        </head>
        <body>
          <div class="container">
            <header>
              <div class="brand">
                <svg viewBox="0 0 24 24" fill="none" stroke="#ea4f16" stroke-width="2"><path d="M3 3h18v18H3z"/><path d="M9 8h6M9 12h6M9 16h4"/></svg>
                <h1>${menu.location.name}</h1>
              </div>
              <div class="locale-switcher">
                ${supportedLocales.map((l: string) => html`<a href="/s/${slug}?locale=${l}">${l.toUpperCase()}</a>`)}
              </div>
            </header>
            <main>
              <div id="root">${menuContent}</div>
            </main>
            <div id="cartFabWrapper" class="cart-fab-wrapper hidden">
              <a id="cartFabBtn" class="cart-fab" href="/s/${slug}/checkout" aria-label="View cart">
                🛒 <span id="fabCount">0</span>
              </a>
            </div>
            <footer>
              <p>Order delivery from ${menu.location.name} via Dowiz</p>
            </footer>
          </div>
          <script>window.__INITIAL_STATE__ = ${JSON.stringify(initialData)};</script>
          <script type="module" src="/dist/menu/app.js"></script>
        </body>
      </html>
    `;

    const fullHtml = '<!DOCTYPE html>\n' + render(vdom as any);
    cache.set(cacheKey, { html: fullHtml, slug });
    return fullHtml;
  } finally {
    client.release();
  }
}
