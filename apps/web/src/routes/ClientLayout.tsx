import React, { useEffect, useState } from 'react';
import { Outlet, useParams, useNavigate, useLocation } from 'react-router-dom';
import { AnimatePresence, motion } from 'framer-motion';
import { ThemeProvider, LanguageSwitcher, ToastProvider, useI18n, StickyActionBar, ResponsiveDialog, AnimatedNumber, Pressable, CurrencySwitcher, PriceDisplay, useCurrency, derivePalette, isPaperSkinEnabled, paperSkinAttr, staggerChildren, listItem } from '@deliveryos/ui';
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
import { CheckoutPage } from '../pages/client/CheckoutPage.js';

function ClientLayoutInner() {
  const { slug } = useParams<{ slug: string }>();
  const navigate = useNavigate();
  const location = useLocation();
  const [theme, setTheme] = useState<ThemeConfig | null>(null);
  const [isCartOpen, setCartOpen] = useState(false);
  const [isCheckoutOpen, setCheckoutOpen] = useState(false);
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

  // §1 flow-simplification: /checkout is a REDIRECT SEAM → /s/:slug?checkout=1 opens the checkout sheet OVER
  // the menu (no page navigation; deep-link friendly; Back closes the sheet with the cart intact). Strip the
  // param after opening, and close the sheet once the order is placed (route changes to the order page).
  useEffect(() => {
    const params = new URLSearchParams(location.search);
    if (params.get('checkout') === '1') {
      setCheckoutOpen(true);
      params.delete('checkout');
      const qs = params.toString();
      navigate({ pathname: location.pathname, search: qs ? `?${qs}` : '' }, { replace: true });
    }
    if (location.pathname.includes('/order/')) setCheckoutOpen(false);
  }, [location.pathname, location.search, navigate]);

  const [locationName, setLocationName] = useState('');
  const [logoUrl, setLogoUrl] = useState('');
  const [supportedLocales, setSupportedLocales] = useState<string[] | undefined>(undefined);
  // Free-delivery threshold (minor units) for the cart nudge — null/0 = feature off (never promise it).
  const [freeDeliveryThreshold, setFreeDeliveryThreshold] = useState<number | null>(null);

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

    // Free-delivery threshold for the cart nudge (server-cached /info; best-effort, never blocks the cart).
    fetch(`/public/locations/${slug}/info`)
      .then((r) => (r.ok ? r.json() : null))
      .then((info) => { if (info && info.freeDeliveryThreshold != null) setFreeDeliveryThreshold(Number(info.freeDeliveryThreshold)); })
      .catch(() => {});

    return () => window.removeEventListener('message', handleMessage);
  }, [slug, location.search]);

  const { currency: activeCurrency, eurRate } = useCurrency();
  const itemsCount = items.reduce((sum, i) => sum + i.quantity, 0);
  const total = items.reduce((sum, i) => sum + i.price * i.quantity, 0);
  // Free-delivery nudge (subtotal vs the location threshold; the price engine zeroes the fee server-side).
  const fdt = freeDeliveryThreshold != null && freeDeliveryThreshold > 0 ? freeDeliveryThreshold : null;
  const fdReached = fdt != null && total >= fdt;
  const fdRemaining = fdt != null ? Math.max(0, fdt - total) : 0;
  const fdPct = fdt != null ? Math.min(100, Math.round((total / fdt) * 100)) : 0;

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
            <CurrencySwitcher />
            {/* Mobile: a single-button language dropdown (the full SQ|EN|UA segmented control
                crowded the header next to the logo, name + currency). Desktop keeps the segment. */}
            <span className="sm:hidden"><LanguageSwitcher variant="compact" allowed={supportedLocales} /></span>
            <span className="hidden sm:inline-flex"><LanguageSwitcher variant="full" allowed={supportedLocales} /></span>
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
                      className="absolute -top-2 -right-2 min-w-[18px] h-[18px] rounded-full bg-[var(--color-danger-strong)] text-white text-step-2xs font-bold flex items-center justify-center leading-none px-1 shadow-md"
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
                      variants={listItem}
                      className="flex items-center justify-between">
                      <div className="flex-1 min-w-0">
                        <div className="text-[var(--brand-text)] font-medium truncate">{item.name}</div>
                        <div className="text-[var(--brand-text-muted)] text-sm"><PriceDisplay amount={item.price} size="sm" /></div>
                      </div>
                      <div className="flex items-center gap-3 shrink-0">
                        <button
                          aria-label={t('cart.decrease', 'Decrease quantity')}
                          onClick={() => updateQuantity(item.id, item.quantity - 1)}
                          className="min-w-[44px] min-h-[44px] rounded-full bg-[var(--brand-surface-raised)] text-[var(--brand-text)] hover:bg-[var(--brand-border)] transition-colors active:scale-95 flex items-center justify-center"
                        >
                          <i className="ti ti-minus text-sm" aria-hidden="true" />
                        </button>
                        <span className="text-[var(--brand-text)] font-medium w-6 text-center">{item.quantity}</span>
                        <button
                          aria-label={t('cart.increase', 'Increase quantity')}
                          onClick={() => updateQuantity(item.id, item.quantity + 1)}
                          className="min-w-[44px] min-h-[44px] rounded-full bg-[var(--brand-surface-raised)] text-[var(--brand-text)] hover:bg-[var(--brand-border)] transition-colors active:scale-95 flex items-center justify-center"
                        >
                          <i className="ti ti-plus text-sm" aria-hidden="true" />
                        </button>
                      </div>
                    </motion.div>
                  ))}
                </motion.div>
                <div className="pt-4 border-t border-[var(--brand-border)] space-y-3">
                  {fdt != null && (
                    fdReached ? (
                      <div data-testid="free-delivery-nudge" className="flex items-center gap-1.5 text-sm font-semibold" style={{ color: 'var(--brand-primary-readable)' }}>
                        <i className="ti ti-truck-delivery" aria-hidden="true" />
                        <span>{t('cart.free_delivery_unlocked', 'Free delivery unlocked!')}</span>
                      </div>
                    ) : (
                      <div data-testid="free-delivery-nudge">
                        <p className="text-sm mb-1.5 text-[var(--brand-text-muted)]">
                          {t('cart.free_delivery_progress', 'Add {{amount}} more for free delivery', { amount: formatMoney(fdRemaining, activeCurrency, eurRate ?? undefined) })}
                        </p>
                        <div className="h-1.5 rounded-full overflow-hidden bg-[var(--brand-border)]" role="progressbar" aria-label={t('cart.free_delivery_label', 'Free delivery progress')} aria-valuenow={fdPct} aria-valuemin={0} aria-valuemax={100}>
                          <div className="h-full rounded-full bg-[var(--brand-primary)] transition-[width] duration-300" style={{ width: `${fdPct}%` }} />
                        </div>
                      </div>
                    )
                  )}
                  <div className="flex justify-between font-bold text-lg text-[var(--brand-text)]">
                    <span>{t('cart.total', 'Total')}</span>
                    <PriceDisplay amount={total} />
                  </div>
                  <button
                    data-testid="cart-checkout"
                    onClick={() => { setCartOpen(false); setCheckoutOpen(true); }}
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
          {/* §1: checkout as a bottom-sheet OVER the menu (the same panel-over-menu primitive as the cart) —
              the customer never leaves /s/:slug; closing keeps the cart. CheckoutPage renders headerless here
              (the dialog provides the chrome) and closes the sheet on Back/empty-state. */}
          <ResponsiveDialog open={isCheckoutOpen} onClose={() => setCheckoutOpen(false)} title={t('checkout.title', 'Checkout')}>
            <CheckoutPage onClose={() => setCheckoutOpen(false)} />
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
