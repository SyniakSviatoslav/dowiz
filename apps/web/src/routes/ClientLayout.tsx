import React, { useEffect, useState } from 'react';
import { Outlet, useParams, useNavigate, useLocation } from 'react-router-dom';
import { ThemeProvider, CartFAB, CartDrawer, LanguageSwitcher, ToastProvider, useI18n } from '@deliveryos/ui';
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
    
    // Handle embed mode
    const isEmbed = new URLSearchParams(window.location.search).get('embed') === 'true';
    if (isEmbed) {
      document.body.classList.add('embed-mode');
    } else {
      document.body.classList.remove('embed-mode');
    }
    
    return () => {
      window.removeEventListener('dos:bounceCart', handleBounce);
      document.body.classList.remove('embed-mode');
    };
  }, [location.search]);

  const [locationName, setLocationName] = useState('');
  const [logoUrl, setLogoUrl] = useState('');

  useEffect(() => {
    if (!slug) return;
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
  }, [slug]);

  const itemsCount = items.reduce((sum, i) => sum + i.quantity, 0);
  const total = items.reduce((sum, i) => sum + i.price * i.quantity, 0);

  return (
    <ThemeProvider theme={theme || undefined}>
      <ToastProvider>
        <div className="min-h-screen bg-[var(--brand-bg)] text-[var(--brand-text)] font-sans pb-24">
          <header className="sticky top-0 z-50 h-[56px] bg-[var(--brand-bg)]/95 backdrop-blur-sm border-b border-[var(--brand-border)] flex items-center px-4 gap-3">
            {logoUrl ? (
              <img src={logoUrl} alt="" className="h-8 w-8 rounded object-contain shrink-0" />
            ) : null}
            <h1 className="text-[16px] font-bold flex-1 truncate" style={{ fontFamily: 'var(--brand-font-heading)' }}>{locationName || t('client.menu', 'Menu')}</h1>
            <LanguageSwitcher variant="full" />
          </header>
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
