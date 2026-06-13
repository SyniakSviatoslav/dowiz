import React, { useEffect, useState } from 'react';
import { Outlet, useParams, useNavigate, useLocation } from 'react-router-dom';
import { AnimatePresence, motion } from 'framer-motion';
import { ThemeProvider, LanguageSwitcher, ToastProvider, useI18n, StickyActionBar, ResponsiveDialog, AnimatedNumber, Pressable } from '@deliveryos/ui';
import type { ThemeConfig } from '@deliveryos/ui';
import { apiClient } from '../lib/index.js';
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

  useEffect(() => {
    if (!slug) return;

    // Listen for postMessage from branding preview parent (logo URL)
    const handleMessage = (e: MessageEvent) => {
      if (e.data?.type === 'branding_preview_logo' && e.data.logoUrl) {
        setLogoUrl(e.data.logoUrl);
      }
    };
    window.addEventListener('message', handleMessage);

    const params = new URLSearchParams(location.search);
    const draftPrimary = params.get('draft_primary');
    const draftBg = params.get('draft_bg');
    const draftText = params.get('draft_text');

    // If draft params are present, use them directly (branding preview)
    if (draftPrimary || draftBg || draftText) {
      setLocationName(slug);
      setTheme({
        primary: draftPrimary || 'var(--brand-primary)',
        primaryHover: 'var(--brand-primary-hover)',
        primaryLight: 'var(--brand-primary-light)',
        accent: 'var(--brand-accent)',
        bg: draftBg || 'var(--brand-bg)',
        surface: 'var(--brand-surface)',
        surfaceRaised: 'var(--brand-surface-raised)',
        text: draftText || 'var(--brand-text)',
        textMuted: 'var(--brand-text-muted)',
        border: 'var(--brand-border)',
      });
      return;
    }

    apiClient<any>(`/public/theme/${slug}`)
      .then((res: any) => {
        setLocationName(res.locationName || '');
        setLogoUrl(res.logoUrl || '');
        setTheme({
          primary: res.primaryColor || 'var(--brand-primary)',
          primaryHover: 'var(--brand-primary-hover)',
          primaryLight: 'var(--brand-primary-light)',
          accent: 'var(--brand-accent)',
          bg: res.bgColor || 'var(--brand-bg)',
          surface: 'var(--brand-surface)',
          surfaceRaised: 'var(--brand-surface-raised)',
          text: res.textColor || 'var(--brand-text)',
          textMuted: 'var(--brand-text-muted)',
          border: 'var(--brand-border)',
        });
      })
      .catch(() => setTheme(null));

    return () => window.removeEventListener('message', handleMessage);
  }, [slug, location.search]);

  const itemsCount = items.reduce((sum, i) => sum + i.quantity, 0);
  const total = items.reduce((sum, i) => sum + i.price * i.quantity, 0);

  return (
    <ThemeProvider theme={theme || undefined}>
      <ToastProvider>
        <div className="app-shell bg-[var(--brand-bg)] text-[var(--brand-text)] font-sans">
          <header className="sticky top-0 z-sticky h-14 bg-[var(--brand-bg)]/95 backdrop-blur-sm border-b border-[var(--brand-border)] flex items-center px-4 gap-3 shrink-0">
            {logoUrl ? (
              <img src={logoUrl} alt="" className="h-8 w-8 rounded object-contain shrink-0" />
            ) : null}
            <h1 className="text-base font-bold flex-1 truncate" style={{ fontFamily: 'var(--brand-font-heading)' }}>{locationName || t('client.menu', 'Menu')}</h1>
            <LanguageSwitcher variant="full" />
          </header>
          <div className="app-shell-main">
            <Outlet />
          </div>
          {itemsCount > 0 && (
            <StickyActionBar embedSticky={true}>
              <Pressable>
                <button
                  onClick={() => setCartOpen(true)}
                  className={`w-full h-12 flex items-center justify-center gap-2 text-white font-bold text-sm rounded-full shadow-lg ${isBouncing ? 'cart-bounce' : ''}`}
                  style={{ background: 'var(--brand-primary)', boxShadow: '0 4px 12px color-mix(in srgb, var(--brand-primary) 40%, transparent)' }}
                >
                  <span className="relative inline-flex">
                    <i className="ti ti-shopping-cart text-lg leading-none" />
                    <motion.span
                      key={itemsCount}
                      initial={{ scale: 1.4 }}
                      animate={{ scale: 1 }}
                      className="absolute -top-2 -right-2 min-w-[18px] h-[18px] rounded-full bg-[var(--color-danger)] text-white text-[10px] font-bold flex items-center justify-center leading-none px-1 shadow-md"
                    >
                      {itemsCount > 99 ? '99+' : itemsCount}
                    </motion.span>
                  </span>
                  <span className="mx-1">{t('cart.title', 'Cart')}</span>
                  <span className="opacity-40">·</span>
                  <AnimatedNumber value={total} className="" formatter={(v) => `${v} ALL`} />
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
                  variants={{ visible: { transition: { staggerChildren: 0.04 } } }}
                  initial="hidden"
                  animate="visible"
                >
                  {items.map(item => (
                    <motion.div
                      key={item.id}
                      variants={{ hidden: { opacity: 0, y: 8 }, visible: { opacity: 1, y: 0, transition: { duration: 0.2, ease: [0.16, 1, 0.3, 1] } } }}
                      className="flex items-center justify-between">
                      <div className="flex-1 min-w-0">
                        <div className="text-[var(--brand-text)] font-medium truncate">{item.name}</div>
                        <div className="text-[var(--brand-text-muted)] text-sm">{item.price} ALL</div>
                      </div>
                      <div className="flex items-center gap-3 shrink-0">
                        <button
                          onClick={() => updateQuantity(item.id, item.quantity - 1)}
                          className="min-w-[44px] min-h-[44px] rounded-full bg-[var(--brand-surface-raised)] text-[var(--brand-text)] hover:bg-[var(--brand-border)] transition-colors active:scale-95 flex items-center justify-center"
                        >
                          <i className="ti ti-minus text-sm" />
                        </button>
                        <span className="text-[var(--brand-text)] font-medium w-6 text-center">{item.quantity}</span>
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
                    <span>{total} ALL</span>
                  </div>
                  <button
                    onClick={() => { setCartOpen(false); navigate(`/s/${slug}/checkout`); }}
                    className="w-full h-12 rounded-full bg-[var(--brand-primary)] text-white font-bold text-sm transition-all active:scale-[0.97]"
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
