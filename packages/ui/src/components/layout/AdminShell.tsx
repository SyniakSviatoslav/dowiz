// @deprecated — Unused shell component. apps/web uses inline layout in AdminRoutes.tsx, not this component.
// Kept for reference; remove after confirming no external consumers.
import { type ReactNode, useState } from 'react';
import { Link } from 'react-router-dom';
import { useEmbed } from '../../hooks/use-embed.js';
import { useBrandTheme } from './ThemeProvider.js';
import { LanguageSwitcher, useI18n } from '../../lib/I18nProvider.js';

interface AdminShellProps {
  children: ReactNode;
  activeKey?: string;
}

export function AdminShell({ children, activeKey }: AdminShellProps) {
  const embed = useEmbed();
  const [mobileNavOpen, setMobileNavOpen] = useState(false);
  const { t } = useI18n();

  const NAV_ITEMS = [
    { key: 'dashboard', label: t('admin.dashboard'), icon: 'ti ti-layout-dashboard', href: '/admin/' },
    { key: 'orders', label: t('admin.orders'), icon: 'ti ti-clipboard-list', href: '/admin/orders' },
    { key: 'menu', label: t('admin.menu'), icon: 'ti ti-notebook', href: '/admin/menu' },
    { key: 'couriers', label: t('admin.couriers'), icon: 'ti ti-motorbike', href: '/admin/couriers' },
    { key: 'analytics', label: t('admin.analytics'), icon: 'ti ti-chart-bar', href: '/admin/analytics' },
    { key: 'crm', label: t('admin.crm'), icon: 'ti ti-users', href: '/admin/crm' },
    { key: 'branding', label: t('admin.branding'), icon: 'ti ti-palette', href: '/admin/branding' },
    { key: 'supplies', label: t('admin.supplies'), icon: 'ti ti-packages', href: '/admin/supplies' },
    { key: 'settings', label: t('admin.settings'), icon: 'ti ti-settings', href: '/admin/settings' },
    { key: 'signals', label: t('admin.signals'), icon: 'ti ti-shield-check', href: '/admin/signals' },
    { key: 'alerts', label: t('admin.alerts'), icon: 'ti ti-bell-ringing', href: '/admin/alerts' },
    { key: 'onboarding', label: t('admin.onboarding'), icon: 'ti ti-rocket', href: '/admin/onboarding' },
  ];

  const asideContent = (
    <>
      <div className="p-4 border-b border-brand-border flex items-center justify-between">
        <h2 className="text-lg font-heading font-bold">Dowiz</h2>
        <button
          className="lg:hidden text-brand-text-muted hover:text-brand-text"
          onClick={() => setMobileNavOpen(false)}
          aria-label="Close menu"
        >
          <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
      <nav className="flex-1 p-2 space-y-1 overflow-y-auto">
        {NAV_ITEMS.map(item => (
          <Link
            key={item.key}
            to={item.href}
            onClick={() => setMobileNavOpen(false)}
            className={`flex items-center gap-3 px-3 py-2 rounded-md text-sm transition-colors duration-200 ${
              activeKey === item.key
                ? 'bg-brand-primary text-white'
                : 'text-brand-text-muted hover:bg-brand-surface-raised hover:text-brand-text'
            }`}
          >
            <i className={item.icon} style={{ fontSize: '1.15rem' }} />
            {item.label}
          </Link>
        ))}
      </nav>
      <div className="p-2 border-t border-brand-border space-y-1">
        <ThemeSwitcherButton />
        <div className="px-3 py-1">
          <LanguageSwitcher variant="compact" />
        </div>
      </div>
    </>
  );

  return (
    <div className="h-screen overflow-hidden bg-brand-bg text-brand-text font-body flex">
      {!embed && (
        <>
          <aside className="w-64 bg-brand-surface border-r border-brand-border hidden lg:flex flex-col shrink-0 h-full">
            {asideContent}
          </aside>

          <div className="lg:hidden fixed top-0 left-0 right-0 z-sticky bg-brand-surface border-b border-brand-border px-4 h-14 flex items-center justify-between">
            <h2 className="text-lg font-heading font-bold">Dowiz</h2>
            <button
              onClick={() => setMobileNavOpen(true)}
              className="text-brand-text-muted hover:text-brand-text"
              aria-label="Open menu"
            >
              <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
              </svg>
            </button>
          </div>

          {mobileNavOpen && (
            <div className="lg:hidden fixed inset-0 z-modal flex">
              <div className="absolute inset-0 bg-black/50" onClick={() => setMobileNavOpen(false)} />
              <aside className="relative w-64 bg-brand-surface border-r border-brand-border flex flex-col z-10">
                {asideContent}
              </aside>
            </div>
          )}
        </>
      )}
      <main className={'flex-1 overflow-y-auto' + (!embed ? ' lg:pt-0 pt-14' : '')}>
        {children}
      </main>
    </div>
  );
}

function ThemeSwitcherButton() {
  try {
    const { cyclePreset, preset } = useBrandTheme();
    return (
      <button
        onClick={cyclePreset}
        className="flex items-center gap-2 w-full px-3 py-2 rounded-md text-sm text-brand-text-muted hover:text-brand-text hover:bg-brand-surface-raised transition-colors"
        title="Cycle theme preset"
        aria-label={`Current theme: ${preset.replace(/-/g, ' ')}. Click to cycle.`}
      >
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01" />
        </svg>
        <span className="capitalize">{preset.replace(/-/g, ' ')}</span>
      </button>
    );
  } catch {
    return null;
  }
}
