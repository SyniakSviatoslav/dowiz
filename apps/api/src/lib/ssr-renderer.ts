import { h } from 'preact';
import render from 'preact-render-to-string';
import htm from 'htm';
import { buildJsonLd } from './jsonld-builder.js';

const html = htm.bind(h);

export function renderMenuPage(data: any, slug: string): string {
  const defaultLocale = data.default_locale;
  const isEmbed = false; // Determined client-side usually, but we could pass it down

  const switchScript = `
    function setLocale(newLocale) {
      document.documentElement.lang = newLocale;
      document.documentElement.dataset.locale = newLocale;
      localStorage.setItem('dowiz_locale', newLocale);
      
      const elements = document.querySelectorAll('[data-text-' + newLocale + ']');
      for (let i = 0; i < elements.length; i++) {
        const el = elements[i];
        el.textContent = el.getAttribute('data-text-' + newLocale);
      }
    }
    
    document.addEventListener('DOMContentLoaded', () => {
      let current = localStorage.getItem('dowiz_locale') || navigator.language.split('-')[0] || '${defaultLocale}';
      if (${JSON.stringify(data.supported_locales)}.indexOf(current) === -1) {
        current = '${defaultLocale}';
      }
      setLocale(current);
    });
  `;

  // Helper to attach multiple data-text-* attributes
  const localizedAttrs = (translations: Record<string, string>) => {
    const attrs: Record<string, string> = {};
    if (!translations) return attrs;
    for (const [loc, text] of Object.entries(translations)) {
      attrs[`data-text-${loc}`] = text;
    }
    return attrs;
  };

  const jsonLd = buildJsonLd(slug, data);

  const title = `${data.location.name} — Menu`;
  let description = data.location.address || '';
  if (!description && data.categories[0]?.products[0]) {
    description = data.categories[0].products[0].available_names[defaultLocale];
  }

  // Precompute if location is closed based on status (simplified)
  // actual status is 'open', 'closed', etc. 
  // Let's assume open if not explicitly checking for now.

  const vdom = html`
    <html lang="${defaultLocale}" data-locale="${defaultLocale}" class="scroll-smooth">
      <head>
        <meta charset="utf-8" />
        <title>${title}</title>
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <meta name="description" content="${description}" />
        <meta name="dos-location-id" content="${data.location.id || slug}" />
        <meta name="dos-menu-version" content="${data.menu_version}" />
        
        <meta property="og:title" content="${title}" />
        <meta property="og:description" content="${description}" />
        <meta property="og:type" content="restaurant" />
        <meta property="og:url" content="https://dowiz.org/s/${slug}" />
        <meta name="twitter:card" content="summary_large_image" />
        <link rel="canonical" href="https://dowiz.org/s/${slug}" />
        
        ${data.supported_locales.map((loc: string) => html`
          <link rel="alternate" hreflang="${loc}" href="https://dowiz.org/s/${slug}?locale=${loc}" />
        `)}
        <link rel="alternate" hreflang="x-default" href="https://dowiz.org/s/${slug}" />
        
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="true" />
        <link href="https://fonts.googleapis.com/css2?family=DM+Sans:wght@400;500;600;700&family=DM+Serif+Display&family=Inter:wght@400;500;600;700&family=Cormorant+Garamond:wght@400;500;600;700&family=Playfair+Display:wght@400;500;600;700&display=swap" rel="stylesheet" />
        <link href="https://cdn.jsdelivr.net/npm/@tabler/icons-webfont@latest/tabler-icons.min.css" rel="stylesheet" />
        
        <script src="https://cdn.tailwindcss.com"></script>
        
        <style>
          :root {
            --brand-primary:        #ea4f16;
            --brand-primary-hover:  #ffa12e;
            --brand-primary-light:  rgba(234, 79, 22, 0.12);
            --brand-accent:         #2a2a2a;
            --brand-bg:             #121212;
            --brand-surface:        #1e1e1e;
            --brand-surface-raised: #2a2a2a;
            --brand-text:           #ffffff;
            --brand-text-muted:     #a8a8a8;
            --brand-border:         #2c2c2c;
            --brand-font-heading:   'Inter', sans-serif;
            --brand-font-body:      'Inter', sans-serif;
            --brand-radius:         12px;
            --brand-radius-sm:      8px;
            --brand-radius-btn:     78px;
            --color-success:  #059669;
            --color-warning:  #D97706;
            --color-danger:   #DC2626;
            --color-info:     #2563EB;
          }
          @media (prefers-color-scheme: dark) {
            :root {
              --brand-bg:             #0F172A;
              --brand-surface:        #1E293B;
              --brand-surface-raised: #263548;
              --brand-text:           #F1F5F9;
              --brand-text-muted:     #94A3B8;
              --brand-border:         #334155;
              --brand-primary-light:  rgba(234, 79, 22, 0.15);
              --brand-accent:         #1E1A14;
            }
          }

          body {
            font-family: var(--brand-font-body);
            background-color: var(--brand-bg);
            color: var(--brand-text);
            -webkit-font-smoothing: antialiased;
          }

          .hide-scrollbar::-webkit-scrollbar { display: none; }
          .hide-scrollbar { -ms-overflow-style: none; scrollbar-width: none; }

          .product-card {
            transition: all 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
            box-shadow: 0 2px 12px color-mix(in srgb, #000 30%, transparent);
          }
          .product-card:hover {
            transform: translateY(-3px);
            box-shadow: 0 12px 32px color-mix(in srgb, var(--brand-primary) 8%, transparent);
          }

          .hero-overlay {
            background: linear-gradient(to top, rgba(0,0,0,0.85) 0%, rgba(0,0,0,0.4) 50%, rgba(0,0,0,0.1) 100%);
          }

          body.embed-mode .embed-hidden { display: none !important; }
          body.embed-mode .no-fixed { position: static !important; }
          body.embed-mode header, body.embed-mode nav { display: none !important; }
          body.embed-mode .embed-show { display: flex !important; }
          .embed-show { display: none; }

          @media (prefers-reduced-motion: no-preference) {
            .cart-bounce { animation: cart-bounce 0.35s ease; }
            @keyframes cart-bounce {
              0%, 100% { transform: scale(1); }
              50% { transform: scale(1.12); }
            }
          }

          button, .interactive-card {
            transition: transform 0.15s cubic-bezier(0.34, 1.56, 0.64, 1), background-color 0.2s ease, box-shadow 0.2s ease, border-color 0.2s ease;
          }
          button:active, .interactive-card:active { transform: scale(0.97); }
        </style>
        <script type="application/ld+json" dangerouslySetInnerHTML=${{ __html: JSON.stringify(jsonLd) }}></script>
      </head>
      <body class="relative min-h-screen pb-20">
        <!-- Sticky Header -->
        <header class="sticky top-0 z-50 h-[56px] bg-[var(--brand-surface)] border-b border-[var(--brand-border)] px-4 flex items-center justify-between no-fixed w-full" style="backdrop-filter:blur(12px)">
          <div class="font-bold text-[20px]" style="color:var(--brand-primary);font-family:var(--brand-font-heading)" ...${localizedAttrs({ [defaultLocale]: data.location.name })}>
            ${data.location.name}
          </div>
          <button class="relative p-2 rounded-full transition-colors" style="color:var(--brand-text)" aria-label="Cart" onclick="window.DowizMenu.toggleClosedOverlay()">
            <i class="ti ti-shopping-cart text-[24px]"></i>
            <span id="headerCartCount" class="absolute top-1.5 right-1.5 flex items-center justify-center min-w-[18px] h-[18px] px-1 text-white text-[10px] font-bold rounded-full border-2" style="background:var(--brand-primary);border-color:var(--brand-bg)">0</span>
          </button>
        </header>

        <!-- Hero Section -->
        <section class="relative w-full h-[240px] flex items-end overflow-hidden" style="background:linear-gradient(160deg,var(--brand-surface-raised) 0%,var(--brand-accent) 60%,var(--brand-primary) 100%)">
          <div class="absolute inset-0 hero-overlay"></div>
          <div class="relative z-10 w-full px-5 pb-5">
            <h1 class="text-[32px] font-bold text-white" style="font-family:var(--brand-font-heading);text-shadow:0 2px 12px rgba(0,0,0,0.5)" ...${localizedAttrs({ [defaultLocale]: data.location.name })}>${data.location.name}</h1>
            <p class="text-[14px] font-medium mt-1" style="color:rgba(255,255,255,0.8)">${data.location.address}</p>
          </div>
        </section>

        <!-- Category Nav -->
        <nav id="categoryNav" class="sticky top-[56px] z-40 h-[48px] border-b no-fixed w-full" style="background:var(--brand-bg);border-color:var(--brand-border)">
          <div class="h-full overflow-x-auto hide-scrollbar flex items-center text-[14px]">
            ${data.categories.map((cat: any, index: number) => html`
              <button onclick="document.getElementById('cat-${cat.id}').scrollIntoView({behavior: 'smooth'})" class="cat-link h-full flex items-center px-4 whitespace-nowrap font-medium transition-colors border-b-2 ${index === 0 ? 'font-semibold border-[var(--brand-primary)]' : 'border-transparent'}" style="color:${index === 0 ? 'var(--brand-primary)' : 'var(--brand-text-muted)'}" ...${localizedAttrs(cat.available_names)}>${cat.available_names[defaultLocale] || Object.values(cat.available_names)[0]}</button>
            `)}
          </div>
        </nav>

        <main class="max-w-7xl mx-auto pt-4">
          <div id="menuContent" class="content-container">
            ${data.categories.map((cat: any) => html`
              <section id="cat-${cat.id}" class="cat-section mb-10 scroll-mt-[120px]">
                <h2 class="text-[22px] font-bold px-4 mb-4" style="font-family:var(--brand-font-heading);color:var(--brand-text)" ...${localizedAttrs(cat.available_names)}>${cat.available_names[defaultLocale] || Object.values(cat.available_names)[0]}</h2>
                <div class="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-3 px-4">
                  ${cat.products.map((prod: any) => {
                    const priceDisp = (prod.price / Math.pow(10, data.currency.minor_unit)).toFixed(data.currency.minor_unit) + ' ' + data.currency.code;
                    return html`
                      <article class="product-card rounded-[12px] flex flex-col cursor-pointer overflow-hidden border ${!prod.available ? 'opacity-60' : ''}" style="background:var(--brand-surface);border-color:var(--brand-border)">
                        <div class="relative w-full aspect-[4/3] flex items-center justify-center" style="background:var(--brand-surface-raised);color:var(--brand-border)">
                          <i class="ti ti-photo text-[32px]"></i>
                          ${!prod.available && html`
                            <div class="absolute inset-0 z-10" style="background:rgba(0,0,0,0.5)"></div>
                            <div class="absolute inset-0 flex items-center justify-center z-20">
                              <span class="text-[11px] px-2 py-1 rounded-[6px] font-medium" style="background:var(--brand-surface-raised);color:var(--brand-text)">Unavailable</span>
                            </div>
                          `}
                        </div>
                        <div class="p-3 flex flex-col flex-1">
                          <h3 class="font-medium text-[14px] mb-1" style="color:var(--brand-text)" ...${localizedAttrs(prod.available_names)}>${prod.available_names[defaultLocale] || Object.values(prod.available_names)[0]}</h3>
                          ${prod.available_descriptions && html`
                            <p class="text-[12px] mb-2" style="color:var(--brand-text-muted);display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden" ...${localizedAttrs(prod.available_descriptions)}>${prod.available_descriptions[defaultLocale] || Object.values(prod.available_descriptions)[0]}</p>
                          `}
                          <div class="flex items-center justify-between mt-auto pt-2">
                            <span class="font-bold text-[15px]" style="color:var(--brand-primary)">${priceDisp}</span>
                            <button class="min-w-[44px] min-h-[44px] flex items-center justify-center text-white active:scale-[0.97] ${!prod.available ? 'opacity-30 cursor-not-allowed' : ''}" style="background:var(--brand-primary);border-radius:var(--brand-radius-btn)" onclick=${prod.available ? `window.DowizMenu.addToCart(event, '${prod.id}', ${prod.price})` : undefined} aria-label="Add" disabled=${!prod.available}>
                              <i class="ti ti-plus text-[16px]"></i>
                            </button>
                          </div>
                        </div>
                      </article>
                    `;
                  })}
                </div>
              </section>
            `)}
          </div>
        </main>

        <!-- Cart FAB -->
        <div id="cartFabWrapper" class="fixed bottom-[80px] right-[20px] z-[100] embed-hidden hidden">
          <a href="/s/${slug}/checkout" id="cartFabBtn" class="h-[48px] px-5 text-white text-[14px] font-medium flex items-center justify-center gap-1" style="background:var(--brand-primary);border-radius:var(--brand-radius-btn);box-shadow:0 4px 12px color-mix(in srgb,var(--brand-primary) 40%,transparent)">
            <i class="ti ti-shopping-cart text-[18px]"></i>
            <span class="mx-1 opacity-40">·</span>Cart
            <span class="mx-1 opacity-40">·</span>
            <span id="fabCount">0</span>
            <!-- No total calculated naively yet, but count is available -->
          </a>
        </div>

        ${data.supported_locales.length > 1 && html`
          <div class="fixed bottom-[20px] left-[20px] z-[100] embed-hidden">
            <select onchange="setLocale(this.value)" class="p-2 rounded border bg-[var(--brand-surface)] text-[var(--brand-text)]">
              ${data.supported_locales.map((loc: string) => html`
                <option value="${loc}">${loc.toUpperCase()}</option>
              `)}
            </select>
          </div>
        `}

        <script dangerouslySetInnerHTML=${{ __html: switchScript }}></script>
        <script src="/dist/cart/app.js" type="module"></script>
        <script src="/dist/menu/app.js" type="module"></script>
        <script>
          if (window.location.search.includes('embed=true')) {
            document.body.classList.add('embed-mode');
          }
        </script>
      </body>
    </html>
  `;

  return '<!DOCTYPE html>\n' + render(vdom);
}
