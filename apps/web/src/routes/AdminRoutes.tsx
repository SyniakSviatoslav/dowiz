import React, { useState } from 'react';
import { Routes, Route, Outlet, useLocation, useNavigate } from 'react-router-dom';
import { ToastProvider, LanguageSwitcher, useI18n, BottomTabBar, ResponsiveDialog } from '@deliveryos/ui';
import type { TabItem } from '@deliveryos/ui';
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

const ALL_NAV_ITEMS = [
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

const PRIMARY_TABS: TabItem[] = ALL_NAV_ITEMS.slice(0, 4).map(item => ({
  key: item.key,
  label: '',
  icon: item.icon,
  href: item.href,
}));

const MORE_ITEMS = ALL_NAV_ITEMS.slice(4);

function AdminLayout() {
  const location = useLocation();
  const navigate = useNavigate();
  const { t } = useI18n();
  const [collapsed, setCollapsed] = useState(false);
  const [logoClicks, setLogoClicks] = useState(0);
  const [showFlowTest, setShowFlowTest] = useState(false);
  const [moreOpen, setMoreOpen] = useState(false);
  const isDev = typeof window !== 'undefined' && (sessionStorage.getItem('dos_dev') === '1' || new URLSearchParams(window.location.search).get('dev') === 'true');
  const devSuffix = isDev ? '?dev=true' : '';

  const navTo = (href: string) => {
    navigate(href + devSuffix);
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

  const getActiveKey = () => {
    for (const item of ALL_NAV_ITEMS) {
      if (isActive(item.href)) return item.key;
    }
    return 'admin.dashboard';
  };

  const activeKey = getActiveKey();
  const isMoreActive = MORE_ITEMS.some(item => isActive(item.href));

  const SidebarNav = () => (
    <nav className="flex-1 p-2 space-y-0.5 overflow-auto">
      {[...ALL_NAV_ITEMS, ...(showFlowTest ? [{ key: 'admin.flow_test', href: '/admin/_flow-test', icon: 'ti ti-flask' }] : [])].map(item => (
        <button
          key={item.href}
          onClick={() => navTo(item.href)}
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
    <div className="app-shell bg-[var(--brand-bg)] text-[var(--brand-text)] overflow-hidden">
      {/* Desktop sidebar */}
      <aside className={`hidden lg:flex flex-col shrink-0 bg-[var(--brand-surface)] border-r border-[var(--brand-border)] sidebar-transition overflow-hidden ${collapsed ? 'w-[56px]' : 'w-56'}`}>
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
      <div className="lg:hidden flex items-center justify-between px-4 h-14 bg-[var(--brand-surface)]/95 backdrop-blur-sm border-b border-[var(--brand-border)] shrink-0">
        <div className="flex items-center gap-2">
          <i className="ti ti-tools-kitchen-2 text-lg" style={{ color: 'var(--brand-primary)' }} />
          <h2 className="text-lg font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>Dowiz</h2>
        </div>
        <LanguageSwitcher variant="full" />
      </div>

      {/* Main content */}
      <main className="app-shell-main lg:pt-0">
        <Outlet />
      </main>

      {/* Mobile bottom tab bar */}
      <div className="lg:hidden">
        <BottomTabBar
          tabs={[
            ...PRIMARY_TABS,
            {
              key: 'more',
              label: t('admin.more', 'More'),
              icon: isMoreActive ? 'ti ti-square-rounded-letter-m' : 'ti ti-dots-grid-horizontal',
              href: '#more',
            },
          ]}
          activeKey={isMoreActive ? 'more' : activeKey}
          onTabClick={(href) => {
            if (href === '#more') {
              setMoreOpen(true);
            } else {
              navTo(href);
            }
          }}
        />
      </div>

      {/* More sheet */}
      <ResponsiveDialog open={moreOpen} onClose={() => setMoreOpen(false)} title={t('admin.more', 'More')}>
        <div className="grid grid-cols-2 gap-2">
          {[...MORE_ITEMS, ...(showFlowTest ? [{ key: 'admin.flow_test', href: '/admin/_flow-test', icon: 'ti ti-flask' }] : [])].map(item => {
            const active = isActive(item.href);
            return (
              <button
                key={item.href}
                onClick={() => { setMoreOpen(false); navTo(item.href); }}
                className={`flex flex-col items-center justify-center gap-1.5 p-4 rounded-xl transition-all active:scale-[0.97] ${
                  active ? 'bg-[var(--brand-primary-light)]' : 'hover:bg-[var(--brand-surface-raised)]'
                }`}
                style={{ minHeight: 'var(--tap-min)' }}
              >
                <i className={`${item.icon} text-xl ${active ? 'text-[var(--brand-primary)]' : 'text-[var(--brand-text-muted)]'}`} />
                <span className={`text-xs font-medium ${active ? 'text-[var(--brand-primary)]' : 'text-[var(--brand-text)]'}`}>{t(item.key)}</span>
              </button>
            );
          })}
        </div>
      </ResponsiveDialog>
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
