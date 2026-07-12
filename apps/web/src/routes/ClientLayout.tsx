import React, { useEffect, useState } from 'react';
import { Outlet, useParams, useNavigate, useLocation } from 'react-router-dom';
import { AnimatePresence, motion } from 'framer-motion';
import { ThemeProvider, LanguageSwitcher, ToastProvider, useI18n, StickyActionBar, ResponsiveDialog, AnimatedNumber, Pressable, CurrencySwitcher, SunlightToggle, PriceDisplay, useCurrency, derivePalette, isPaperSkinEnabled, paperSkinAttr, staggerChildren, listItem } from '@deliveryos/ui';
import { formatMoney } from '@deliveryos/shared-types';
import type { ThemeConfig } from '@deliveryos/ui';
import { apiClient } from '../lib/index.js';
import { z } from 'zod';

const PublicThemeResponse = z.object({
  // The API sends null (not undefined) for unset fields — accept null or the
  // parse throws and the whole theme/branding/supported-locales fetch is lost.
  locationName: z.string().nullable().optional(),
  logoUrl: z.string().nullable().optional(),
  primaryColor: z.string().nullable().optional(),
  bgColor: z.string().nullable().optional(),
  textColor: z.string().nullable().optional(),
  supportedLocales: z.array(z.string()).nullable().optional(),
}).passthrough();
import { CartProvider, useSharedCart } from '../lib/CartProvider.js';

function ClientLayoutInner() {
  const { slug } = useParams<{ slug: string }>();
  const navigate = useNavigate();
  const location = useLocation();
  const [theme, setTheme] = useState<ThemeConfig | null>(null);
  const [isCartOpen, setCartOpen] = useState(false);
  const [isBouncing, setIsBouncing] = useState(false);
  const { items, updateQuantity, clearCart } = useSharedCart();
  const { t } = useI18n();

  const isEmbed = new URLSearchParams(location.search).get('embed') === 'true';

  useEffect(() => {
    const handleBounce = () => {
      setIsBouncing(false);
      setTimeout(() => setIsBouncing(true), 10);
      setTimeout(() => setIsBouncing(false), 400);
    };
    window.addEventListener('dos:bounceCart', handleBounce);
    if (isEmbed) {
      document.body.classList.add('embed-mode');
    } else {
      document.body.classList.remove('embed-mode');
    }
    return () => {
      window.removeEventListener('dos:bounceCart', handleBounce);
      document.body.classList.remove('embed-mode');
    };
  }, [isEmbed]);

  const [locationName, setLocationName] = useState('');
  const [logoUrl, setLogoUrl] = useState('');
  const [supportedLocales, setSupportedLocales] = useState<string[] | undefined>(undefined);

  useEffect(() => {
    if (!slug) return;

    // Listen for postMessage from the branding-preview parent. Logo + live theme
    // updates arrive this way so the preview reflects edits WITHOUT reloading the
    // whole storefront (no flicker / scroll reset). Theme is derived into a full
    // coherent palette, identical to what real customers get.
    const handleMessage = (e: MessageEvent) => {
      if (e.data?.type === 'branding_preview_logo' && e.data.logoUrl) {
        setLogoUrl(e.data.logoUrl);
      }
      if (e.data?.type === 'branding_preview_theme') {
        setTheme(derivePalette({ primary: e.data.primary, bg: e.data.bg, text: e.data.text }));
      }
    };
    window.addEventListener('message', handleMessage);

    // Notify parent that iframe React app is mounted (branding preview)
    if (window.parent !== window) {
      window.parent.postMessage({ type: 'branding_preview_ready' }, '*');
    }

    const params = new URLSearchParams(location.search);
    const draftPrimary = params.get('draft_primary');
    const draftBg = params.get('draft_bg');
    const draftText = params.get('draft_text');

    // If draft params are present, use them directly (branding preview). Derive a
    // FULL coherent palette so the preview matches what customers will actually see
    // (surfaces/borders/muted text follow the chosen bg, not the default dark preset).
    if (draftPrimary || draftBg || draftText) {
      setLocationName(slug);
      setTheme(derivePalette({ primary: draftPrimary, bg: draftBg, text: draftText }));
      return;
    }

    apiClient<typeof PublicThemeResponse>(`/public/theme/${slug}`, { schema: PublicThemeResponse })
      .then((res) => {
        setLocationName(res.locationName || '');
        setLogoUrl(res.logoUrl || '');
        setSupportedLocales(res.supportedLocales || undefined);
        // Derive every remaining token from the tenant's primary/bg/text so a light
        // theme never inherits dark default surfaces (the dark-text-on-dark bug).
        const hasTheme = res.primaryColor || res.bgColor || res.textColor;
        setTheme(hasTheme ? derivePalette({ primary: res.primaryColor, bg: res.bgColor, text: res.textColor }) : null);
      })
      .catch(() => setTheme(null));

    return () => window.removeEventListener('message', handleMessage);
  }, [slug, location.search]);

  const { currency: activeCurrency, eurRate } = useCurrency();
  const itemsCount = items.reduce((sum, i) => sum + i.quantity, 0);
  const total = items.reduce((sum, i) => sum + i.price * i.quantity, 0);

  return (
    <ThemeProvider theme={theme || undefined}>
      <ToastProvider>
        <div {...paperSkinAttr()} className="app-shell bg-[var(--brand-bg)] text-[var(--brand-text)] font-sans" style={isPaperSkinEnabled() ? undefined : { ['--brand-font-heading' as any]: "'Playfair Display', 'Cormorant Garamond', Georgia, serif" }}>
          <header className="sticky top-0 z-sticky h-14 bg-[var(--brand-bg)]/95 backdrop-blur-sm border-b border-[var(--brand-border)] flex items-center px-4 gap-3 shrink-0">
            {logoUrl ? (
              // Hide on load failure instead of showing a broken-image icon (e.g. a stale
              // logoUrl pointing at a deleted object).
              <img src={logoUrl} alt="" onError={() => setLogoUrl('')} className="h-8 w-8 rounded object-contain shrink-0" />
            ) : null}
            {/* Persistent brand chrome — not the page <h1>; each route owns its own h1
               (menu hero, Checkout, Order) so the document has a single top heading. */}
            <div className="text-base font-bold flex-1 truncate" style={{ fontFamily: 'var(--brand-font-heading)' }}>{locationName || t('client.menu', 'Menu')}</div>
            <SunlightToggle />
            <CurrencySwitcher />
            <LanguageSwitcher variant="full" allowed={supportedLocales} />
          </header>
          <div className="app-shell-main">
            <Outlet />
          </div>
          {itemsCount > 0 && !location.pathname.includes('/checkout') && (
            <StickyActionBar embedSticky={true}>
              <Pressable>
                <button
                  data-testid="cart-open"
                  onClick={() => setCartOpen(true)}
                  className={`w-full h-12 flex items-center justify-center gap-2 text-[var(--brand-bg)] font-bold text-sm rounded-full shadow-lg ${isBouncing ? 'cart-bounce' : ''}`}
                  style={{ background: 'var(--brand-primary)', boxShadow: '0 4px 12px color-mix(in srgb, var(--brand-primary) 40%, transparent)' }}
                >
                  <span className="relative inline-flex">
                    <i className="ti ti-shopping-cart text-lg leading-none" />
                    <motion.span
                      key={itemsCount}
                      initial={{ scale: 1.4 }}
                      animate={{ scale: 1 }}
                      transition={{ type: 'spring', stiffness: 400, damping: 25 }}
                      className="absolute -top-2 -right-2 min-w-[18px] h-[18px] rounded-full bg-[var(--color-danger)] text-white text-step-2xs font-bold flex items-center justify-center leading-none px-1 shadow-md"
                    >
                      {itemsCount > 99 ? '99+' : itemsCount}
                    </motion.span>
                  </span>
                  <span className="mx-1">{t('cart.title', 'Cart')}</span>
                  <span className="opacity-40">·</span>
                  <AnimatedNumber value={total} className="" formatter={(v) => formatMoney(Math.round(v), activeCurrency, eurRate ?? undefined)} />
                </button>
              </Pressable>
            </StickyActionBar>
          )}
          <ResponsiveDialog open={isCartOpen} onClose={() => setCartOpen(false)} title={t('cart.title', 'Cart')}>
            {items.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-12 gap-3" style={{ color: 'var(--brand-text-muted)' }}>
                <i className="ti ti-shopping-cart text-3xl opacity-30" />
                <span className="text-sm">{t('cart.empty', 'Cart is empty')}</span>
              </div>
            ) : (
              <div className="flex flex-col">
                <motion.div
                  className="flex-1 overflow-y-auto space-y-4 pb-4 max-h-[50vh]"
                  variants={staggerChildren}
                  initial="hidden"
                  animate="visible"
                >
                  {items.map(item => (
                    <motion.div
                      key={item.id}
                      data-testid="cart-item"
                      variants={listItem}
                      className="flex items-center justify-between">
                      <div className="flex-1 min-w-0">
                        <div className="text-[var(--brand-text)] font-medium truncate">{item.name}</div>
                        <div className="text-[var(--brand-text-muted)] text-sm"><PriceDisplay amount={item.price} size="sm" /></div>
                      </div>
                      <div className="flex items-center gap-3 shrink-0">
                        <button
                          onClick={() => updateQuantity(item.id, item.quantity - 1)}
                          className="min-w-[44px] min-h-[44px] rounded-full bg-[var(--brand-surface-raised)] text-[var(--brand-text)] hover:bg-[var(--brand-border)] transition-colors active:scale-95 flex items-center justify-center"
                        >
                          <i className="ti ti-minus text-sm" />
                        </button>
                        <span data-testid="cart-item-qty" className="text-[var(--brand-text)] font-medium w-6 text-center">{item.quantity}</span>
                        <button
                          onClick={() => updateQuantity(item.id, item.quantity + 1)}
                          className="min-w-[44px] min-h-[44px] rounded-full bg-[var(--brand-surface-raised)] text-[var(--brand-text)] hover:bg-[var(--brand-border)] transition-colors active:scale-95 flex items-center justify-center"
                        >
                          <i className="ti ti-plus text-sm" />
                        </button>
                      </div>
                    </motion.div>
                  ))}
                </motion.div>
                <div className="pt-4 border-t border-[var(--brand-border)] space-y-3">
                  <div className="flex justify-between font-bold text-lg text-[var(--brand-text)]">
                    <span>{t('cart.total', 'Total')}</span>
                    <span data-testid="cart-total"><PriceDisplay amount={total} /></span>
                  </div>
                  <button
                    data-testid="cart-checkout"
                    onClick={() => { setCartOpen(false); navigate(`/s/${slug}/checkout`); }}
                    className="w-full h-12 rounded-full bg-[var(--brand-primary)] text-[var(--brand-bg)] font-bold text-base shadow-xl transition-all active:scale-[0.97]"
                  >
                    {t('cart.checkout', 'Checkout')}
                  </button>
                  {items.length > 1 && (
                    <button
                      onClick={() => clearCart()}
                      className="w-full text-sm text-[var(--brand-text-muted)] hover:text-[var(--color-danger)] transition-colors py-2"
                    >
                      {t('cart.clear', 'Clear cart')}
                    </button>
                  )}
                </div>
              </div>
            )}
          </ResponsiveDialog>
        </div>
      </ToastProvider>
    </ThemeProvider>
  );
}

export function ClientLayout() {
  const { slug } = useParams<{ slug: string }>();
  return (
    <CartProvider locationId={slug || 'default'}>
      <ClientLayoutInner />
    </CartProvider>
  );
}
