import React from 'react';
import { useI18n } from '../../index.js';

interface AdminShellProps {
  children: React.ReactNode;
  currentPath: string;
  onNavigate: (path: string) => void;
  onLogout?: () => void;
}
export function AdminShell({ children, currentPath, onNavigate, onLogout }: AdminShellProps) {
  const { t } = useI18n();
  const navItems = [
    { path: '/admin', label: t('admin.live_orders', 'Live Orders') },
    { path: '/admin/menu', label: t('admin.menu_manager', 'Menu Manager') },
    { path: '/admin/branding', label: t('admin.theme_settings', 'Theme Settings') },
  ];

  return (
    <div className={`min-h-screen bg-brand-bg flex flex-col md:flex-row text-brand-text`}>
      {/* Desktop Sidebar / Mobile Topbar */}
      <div className={`w-full md:w-64 bg-brand-surface border-b md:border-b-0 md:border-r border-brand-border p-4 flex flex-col`}>
        <h1 className="text-xl font-bold mb-6 text-brand-primary" style={{ fontFamily: `fontFamily` }}>
          DeliveryOS Admin
        </h1>
        <nav className="flex md:flex-col gap-2 overflow-x-auto md:overflow-visible flex-1">
          {navItems.map(item => (
            <button
              key={item.path}
              onClick={() => onNavigate(item.path)}
              className={`w-full text-left px-4 py-2 rounded-lg transition-colors font-medium text-sm ${currentPath === item.path ? `bg-[var(--brand-primary-light)] text-brand-primary` : `hover:bg-brand-surface-raised`}`}
            >
              {item.label}
            </button>
          ))}
        </nav>
        {onLogout && (
          <button onClick={onLogout} className={`mt-auto px-4 py-2 text-sm text-semantic-danger hover:bg-color-danger-light rounded-lg transition-colors flex items-center gap-2`}>
            <i className="ti ti-logout" /> {t('admin.logout', 'Logout')}
          </button>
        )}
      </div>

      {/* Main Content */}
      <div className="flex-1 overflow-auto">
        {children}
      </div>
    </div>
  );
}
