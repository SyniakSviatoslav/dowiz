import React, { useEffect, useState } from 'react';
import { Outlet, useParams, useNavigate, useLocation } from 'react-router-dom';
import { ThemeProvider, CartFAB, CartDrawer, LanguageSwitcher, useI18n } from '@deliveryos/ui';
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
  const { items, updateQuantity } = useSharedCart();
  const { t } = useI18n();

  const isMenuPage = location.pathname.endsWith(`/s/${slug}`) || location.pathname === `/s/${slug}/`;

  useEffect(() => {
    const handleBounce = () => {
      setIsBouncing(false);
      setTimeout(() => setIsBouncing(true), 10);
      setTimeout(() => setIsBouncing(false), 400);
    };
    window.addEventListener('dos:bounceCart', handleBounce);
    return () => window.removeEventListener('dos:bounceCart', handleBounce);
  }, []);

  useEffect(() => {
    if (!slug) return;
    apiClient<any>(`/public/theme/${slug}`)
      .then((res: any) => {
        setTheme({
          primary: res.primaryColor || '#ea4f16',
          primaryHover: '#d44310',
          primaryLight: 'rgba(234, 79, 22, 0.12)',
          accent: '#2a2a2a',
          bg: res.bgColor || '#121212',
          surface: '#1e1e1e',
          surfaceRaised: '#2a2a2a',
          text: res.textColor || '#ffffff',
          textMuted: '#a8a8a8',
          border: '#2c2c2c',
        });
      })
      .catch(() => setTheme(null));
  }, [slug]);

  const itemsCount = items.reduce((sum, i) => sum + i.quantity, 0);
  const total = items.reduce((sum, i) => sum + i.price * i.quantity, 0);

  return (
    <ThemeProvider theme={theme || undefined}>
      <div className="min-h-screen bg-[var(--brand-bg)] text-[var(--brand-text)] font-sans pb-24">
        {isMenuPage && (
          <header className="sticky top-0 z-50 h-[56px] bg-[var(--brand-bg)]/95 backdrop-blur-sm border-b border-[var(--brand-border)] flex items-center px-4">
            <h1 className="text-[16px] font-bold flex-1 truncate" style={{ fontFamily: 'var(--brand-font-heading)' }}>Dubin &amp; Sushi</h1>
            <LanguageSwitcher variant="compact" />
          </header>
        )}
        <Outlet />
        <CartFAB itemsCount={itemsCount} total={total} onClick={() => setCartOpen(true)} isBouncing={isBouncing} />
        <CartDrawer
          isOpen={isCartOpen}
          onClose={() => setCartOpen(false)}
          items={items}
          onUpdateQuantity={updateQuantity}
          onCheckout={() => { setCartOpen(false); navigate(`/s/${slug}/checkout`); }}
          title={t('cart.title')}
          emptyText={t('cart.empty')}
          totalLabel={t('cart.total')}
          checkoutLabel={t('cart.checkout')}
          clearLabel={t('cart.clear')}
        />
      </div>
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
