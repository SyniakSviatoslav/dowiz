import React, { useState } from 'react';
import { Routes, Route, Outlet, Navigate, useLocation, useNavigate } from 'react-router-dom';
import { LanguageSwitcher, BottomTabBar, CurrencySwitcher, SunlightToggle, useI18n, paperSkinAttr, ToastProvider } from '@deliveryos/ui';
import type { TabItem } from '@deliveryos/ui';
import { TasksPage } from '../pages/courier/TasksPage.js';
import { DeliveryPage } from '../pages/courier/DeliveryPage.js';
import { LoginPage } from '../pages/courier/LoginPage.js';
import { EarningsPage } from '../pages/courier/EarningsPage.js';
import { HistoryPage } from '../pages/courier/HistoryPage.js';
import { ShiftPage } from '../pages/courier/ShiftPage.js';

const TAB_DEFS = [
  { key: 'tasks', icon: 'ti ti-clipboard-list', href: '/courier' },
  { key: 'earnings', icon: 'ti ti-coin', href: '/courier/earnings' },
  { key: 'history', icon: 'ti ti-history', href: '/courier/history' },
  { key: 'shift', icon: 'ti ti-clock', href: '/courier/shift' },
];

function CourierLayout() {
  const location = useLocation();
  const navigate = useNavigate();
  const { t } = useI18n();
  const TABS: TabItem[] = TAB_DEFS.map(d => ({ ...d, label: t(`courier.${d.key}`) }));
  const isDev = typeof window !== 'undefined' && (sessionStorage.getItem('dos_dev') === '1' || new URLSearchParams(window.location.search).get('dev') === 'true');
  const devSuffix = isDev ? '?dev=true' : '';

  const navTo = (href: string) => navigate(href + devSuffix);

  const isDeliveryView = location.pathname.includes('/delivery/');
  const isLoginView = location.pathname.includes('/login');

  const getActiveKey = () => {
    for (const tab of TABS) {
      if (location.pathname === tab.href || (tab.href !== '/courier' && location.pathname.startsWith(tab.href))) {
        return tab.key;
      }
    }
    return 'tasks';
  };

  if (isDeliveryView || isLoginView) {
    return (
      // Active-delivery / login: paper palette, but grain OFF — contrast wins under sun.
      <div
        {...paperSkinAttr()}
        data-surface="dark"
        style={{ ['--paper-grain-opacity' as string]: '0' }}
        className="app-shell bg-[var(--brand-bg)] text-[var(--brand-text)]"
      >
        <main className="app-shell-main">
          <Outlet />
        </main>
      </div>
    );
  }

  return (
    <div {...paperSkinAttr()} data-surface="dark" className="app-shell bg-[var(--brand-bg)] text-[var(--brand-text)]">
      <div className="flex items-center px-4 h-14 bg-[var(--brand-surface)]/95 backdrop-blur-sm border-b border-[var(--brand-border)] shrink-0">
        <div className="w-full max-w-md mx-auto flex items-center justify-between">
          <span className="text-sm font-semibold" style={{ fontFamily: 'var(--brand-font-heading)' }}>Courier</span>
          <div className="flex items-center gap-1">
            <SunlightToggle />
            <CurrencySwitcher />
            <LanguageSwitcher variant="full" />
          </div>
        </div>
      </div>
      <main className="app-shell-main">
        <div className="w-full max-w-md mx-auto">
          <Outlet />
        </div>
      </main>
      <BottomTabBar
        tabs={TABS}
        activeKey={getActiveKey()}
        onTabClick={navTo}
      />
    </div>
  );
}

export function CourierRoutes() {
  return (
    // S4 fix: courier pages had no toast/notification mechanism mounted at all —
    // silent mutation failures (console.* only) had no user-facing alternative.
    // Mirrors AdminRoutes/ClientLayout, which already wrap their routes the same way.
    <ToastProvider>
      <Routes>
        <Route path="/" element={<CourierLayout />}>
          <Route index element={<TasksPage />} />
          <Route path="delivery/:id" element={<DeliveryPage />} />
          <Route path="login" element={<LoginPage />} />
          <Route path="earnings" element={<EarningsPage />} />
          <Route path="history" element={<HistoryPage />} />
          <Route path="shift" element={<ShiftPage />} />
          {/* BUGFIX: unknown /courier/* paths rendered a blank Outlet — redirect to tasks instead. */}
          <Route path="*" element={<Navigate to="/courier" replace />} />
        </Route>
      </Routes>
    </ToastProvider>
  );
}
