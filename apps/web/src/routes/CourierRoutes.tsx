import React, { useState } from 'react';
import { Routes, Route, Outlet, useLocation, useNavigate } from 'react-router-dom';
import { LanguageSwitcher, BottomTabBar, CurrencySwitcher, SunlightToggle, useI18n } from '@deliveryos/ui';
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
      <div className="app-shell bg-[var(--brand-bg)] text-[var(--brand-text)]">
        <main className="app-shell-main">
          <Outlet />
        </main>
      </div>
    );
  }

  return (
    <div className="app-shell bg-[var(--brand-bg)] text-[var(--brand-text)]">
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
    <Routes>
      <Route path="/" element={<CourierLayout />}>
        <Route index element={<TasksPage />} />
        <Route path="delivery/:id" element={<DeliveryPage />} />
        <Route path="login" element={<LoginPage />} />
        <Route path="earnings" element={<EarningsPage />} />
        <Route path="history" element={<HistoryPage />} />
        <Route path="shift" element={<ShiftPage />} />
      </Route>
    </Routes>
  );
}
