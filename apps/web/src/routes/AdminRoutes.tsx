import React, { useState } from 'react';
import { Routes, Route, Outlet, useLocation, useNavigate } from 'react-router-dom';
import { ToastProvider, LanguageSwitcher, useI18n } from '@deliveryos/ui';
import { DashboardPage } from '../pages/admin/DashboardPage.js';
import { MenuManagerPage } from '../pages/admin/MenuManagerPage.js';
import { BrandingPage } from '../pages/admin/BrandingPage.js';
import { CouriersPage } from '../pages/admin/CouriersPage.js';
import { AnalyticsPage } from '../pages/admin/AnalyticsPage.js';
import { CRMPage } from '../pages/admin/CRMPage.js';
import { SettingsPage } from '../pages/admin/SettingsPage.js';
import { OnboardingPage } from '../pages/admin/OnboardingPage.js';
import { SupplyLibraryPage } from '../pages/admin/SupplyLibraryPage.js';
import { FlowTestPage } from '../pages/admin/FlowTestPage.js';

const NAV_ITEMS = [
  { key: 'admin.dashboard', href: '/admin', icon: 'ti ti-layout-dashboard' },
  { key: 'admin.orders', href: '/admin/orders', icon: 'ti ti-clipboard-list' },
  { key: 'admin.menu', href: '/admin/menu', icon: 'ti ti-tools-kitchen-2' },
  { key: 'admin.supplies', href: '/admin/supplies', icon: 'ti ti-packages' },
  { key: 'admin.couriers', href: '/admin/couriers', icon: 'ti ti-motorbike' },
  { key: 'admin.analytics', href: '/admin/analytics', icon: 'ti ti-chart-bar' },
  { key: 'admin.crm', href: '/admin/crm', icon: 'ti ti-users' },
  { key: 'admin.branding', href: '/admin/branding', icon: 'ti ti-palette' },
  { key: 'admin.settings', href: '/admin/settings', icon: 'ti ti-settings' },
];

function AdminLayout() {
  const location = useLocation();
  const navigate = useNavigate();
  const { t } = useI18n();
  const [collapsed, setCollapsed] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [logoClicks, setLogoClicks] = useState(0);
  const [showFlowTest, setShowFlowTest] = useState(false);
  const isDev = typeof window !== 'undefined' && (sessionStorage.getItem('dos_dev') === '1' || new URLSearchParams(window.location.search).get('dev') === 'true');
  const devSuffix = isDev ? '?dev=true' : '';

  const navTo = (href: string) => {
    navigate(href + devSuffix);
    setMobileOpen(false);
  };

  const isActive = (href: string) => location.pathname === href || (href !== '/admin' && location.pathname.startsWith(href));

  const handleLogoClick = () => {
    const next = logoClicks + 1;
    setLogoClicks(next);
    if (next >= 5) {
      setShowFlowTest(true);
      setLogoClicks(0);
    }
    setTimeout(() => { if (logoClicks < 5) setLogoClicks(0); }, 3000);
  };

  const sidebarWidth = collapsed ? 'w-[56px]' : 'w-56';

  const SidebarNav = () => (
    <nav className="flex-1 p-2 space-y-0.5 overflow-auto">
      {[...NAV_ITEMS, ...(showFlowTest ? [{ key: 'admin.flow_test', href: '/admin/_flow-test', icon: 'ti ti-flask' }] : [])].map(item => (
        <button
          key={item.href}
              onClick={() => { navTo(item.href); setMobileOpen(false); }}
          title={collapsed ? t(item.key) : undefined}
          className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-[var(--brand-radius-sm)] text-sm transition-all duration-200 ${
            isActive(item.href)
              ? 'bg-[var(--brand-primary)] text-white font-medium shadow-sm'
              : 'text-[var(--brand-text-muted)] hover:bg-[var(--brand-surface-raised)] hover:text-[var(--brand-text)]'
          }`}
        >
          <i className={`${item.icon} text-[18px] shrink-0 ${collapsed ? 'mx-auto' : ''}`} />
          {!collapsed && <span className="truncate">{t(item.key)}</span>}
        </button>
      ))}
    </nav>
  );

  return (
    <div className="h-screen bg-[var(--brand-bg)] text-[var(--brand-text)] flex overflow-hidden">
      {/* Desktop sidebar */}
      <aside className={`hidden lg:flex flex-col shrink-0 bg-[var(--brand-surface)] border-r border-[var(--brand-border)] sidebar-transition overflow-hidden ${sidebarWidth}`}>
        <div className={`flex items-center border-b border-[var(--brand-border)] ${collapsed ? 'justify-center p-3' : 'justify-between p-4'}`}>
          {!collapsed && (
            <button onClick={handleLogoClick} className="flex items-center gap-2 cursor-pointer select-none">
              <span className="text-xl">🍱</span>
              <h2 className="text-lg font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>Dowiz</h2>
            </button>
          )}
          <button
            onClick={() => setCollapsed(c => !c)}
            className="w-7 h-7 flex items-center justify-center rounded-md text-[var(--brand-text-muted)] hover:bg-[var(--brand-surface-raised)] hover:text-[var(--brand-text)] transition-colors"
            title={collapsed ? t('admin.expand_sidebar', 'Expand sidebar') : t('admin.collapse_sidebar', 'Collapse sidebar')}
          >
            <i className={`ti ${collapsed ? 'ti-chevron-right' : 'ti-chevron-left'} text-sm`} />
          </button>
        </div>
        <SidebarNav />
        <div className={`p-2 border-t border-[var(--brand-border)] space-y-1 ${collapsed ? 'text-center' : ''}`}>
          {!collapsed && <div className="px-1"><LanguageSwitcher variant="full" /></div>}
          <button
            onClick={() => {
              localStorage.removeItem('dos_access_token');
              navigate('/login');
            }}
            className={`flex items-center gap-2 w-full px-3 py-2 rounded-md text-sm text-[var(--brand-text-muted)] hover:bg-[var(--brand-surface-raised)] hover:text-[var(--brand-text)] transition-colors ${collapsed ? 'justify-center' : ''}`}
          >
            <i className="ti ti-logout text-[18px]" />
            {!collapsed && t('auth.logout', 'Exit')}
          </button>
        </div>
      </aside>

      {/* Mobile top bar */}
      <div className="lg:hidden fixed top-0 left-0 right-0 z-50 bg-[var(--brand-surface)]/95 backdrop-blur-sm border-b border-[var(--brand-border)] px-4 h-14 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <i className="ti ti-tools-kitchen-2 text-lg" style={{ color: 'var(--brand-primary)' }} />
          <h2 className="text-lg font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>Dowiz</h2>
        </div>
        <button onClick={() => setMobileOpen(true)} className="w-8 h-8 flex items-center justify-center rounded-md text-[var(--brand-text-muted)] hover:bg-[var(--brand-surface-raised)]">
          <i className="ti ti-menu-2 text-xl" />
        </button>
      </div>

      {/* Mobile drawer */}
      {mobileOpen && (
        <div className="lg:hidden fixed inset-0 z-50 flex fade-in">
          <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={() => setMobileOpen(false)} />
          <aside className="relative w-64 bg-[var(--brand-surface)] border-r border-[var(--brand-border)] flex flex-col z-10 slide-in-right">
            <div className="p-4 border-b border-[var(--brand-border)] flex justify-between items-center">
              <div className="flex items-center gap-2">
              <i className="ti ti-tools-kitchen-2 text-xl" style={{ color: 'var(--brand-primary)' }} />
                <h2 className="text-lg font-bold">Dowiz</h2>
              </div>
              <button onClick={() => setMobileOpen(false)} className="w-7 h-7 flex items-center justify-center rounded-md text-[var(--brand-text-muted)] hover:bg-[var(--brand-surface-raised)]">
                <i className="ti ti-x text-lg" />
              </button>
            </div>
            <SidebarNav />
          </aside>
        </div>
      )}

      {/* Main content */}
      <main className="flex-1 overflow-y-auto lg:pt-0 pt-14 h-full">
        <Outlet />
      </main>
    </div>
  );
}

export function AdminRoutes() {
  return (
    <ToastProvider>
      <Routes>
        <Route path="/" element={<AdminLayout />}>
          <Route index element={<DashboardPage />} />
          <Route path="orders" element={<DashboardPage />} />
          <Route path="menu" element={<MenuManagerPage />} />
          <Route path="supplies" element={<SupplyLibraryPage />} />
          <Route path="branding" element={<BrandingPage />} />
          <Route path="couriers" element={<CouriersPage />} />
          <Route path="analytics" element={<AnalyticsPage />} />
          <Route path="crm" element={<CRMPage />} />
          <Route path="settings" element={<SettingsPage />} />
          <Route path="onboarding" element={<OnboardingPage />} />
          <Route path="_flow-test" element={<FlowTestPage />} />
        </Route>
      </Routes>
    </ToastProvider>
  );
}
