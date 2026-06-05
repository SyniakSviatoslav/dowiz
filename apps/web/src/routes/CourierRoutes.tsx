import React, { useState } from 'react';
import { Routes, Route, Outlet, useLocation, useNavigate } from 'react-router-dom';
import { LanguageSwitcher } from '@deliveryos/ui';
import { TasksPage } from '../pages/courier/TasksPage.js';
import { DeliveryPage } from '../pages/courier/DeliveryPage.js';
import { LoginPage } from '../pages/courier/LoginPage.js';
import { EarningsPage } from '../pages/courier/EarningsPage.js';
import { HistoryPage } from '../pages/courier/HistoryPage.js';
import { ShiftPage } from '../pages/courier/ShiftPage.js';

const TABS = [
  { label: 'Tasks', href: '/courier' },
  { label: 'Earnings', href: '/courier/earnings' },
  { label: 'History', href: '/courier/history' },
  { label: 'Shift', href: '/courier/shift' },
];

function CourierLayout() {
  const location = useLocation();
  const navigate = useNavigate();
  const isDev = typeof window !== 'undefined' && (sessionStorage.getItem('dos_dev') === '1' || new URLSearchParams(window.location.search).get('dev') === 'true');
  const devSuffix = isDev ? '?dev=true' : '';

  const navTo = (href: string) => navigate(href + devSuffix);

  const isDeliveryView = location.pathname.includes('/delivery/');
  const isLoginView = location.pathname.includes('/login');

  if (isDeliveryView || isLoginView) {
    return <Outlet />;
  }

  return (
    <div className="h-screen bg-[var(--brand-bg)] text-[var(--brand-text)] flex flex-col overflow-hidden">
      <div className="flex-1 overflow-auto pb-16">
        <Outlet />
      </div>
      <div className="flex items-center justify-between px-4 h-12 bg-[var(--brand-surface)] border-b border-[var(--brand-border)] shrink-0">
        <span className="text-sm font-semibold">Courier</span>
        <LanguageSwitcher variant="compact" />
      </div>
      <div className="embed-hidden sticky bottom-0 left-0 right-0 h-16 bg-[var(--brand-surface)] border-t border-[var(--brand-border)] flex items-center justify-around z-50">
        {TABS.map(tab => {
          const active = location.pathname === tab.href || (tab.href !== '/courier' && location.pathname.startsWith(tab.href));
          return (
            <button
              key={tab.href}
              onClick={() => navTo(tab.href)}
              aria-current={active ? 'page' : undefined}
              className={`flex flex-col items-center justify-center w-full h-full transition-colors ${
                active ? 'text-[var(--brand-primary)]' : 'text-[var(--brand-text-muted)] hover:text-[var(--brand-text)]'
              }`}
            >
              <span className="text-[10px] font-semibold">{tab.label}</span>
            </button>
          );
        })}
      </div>
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
