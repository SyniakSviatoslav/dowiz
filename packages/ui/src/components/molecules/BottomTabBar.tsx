import type { ReactNode } from 'react';
import { useEmbed } from '../../hooks/use-embed.js';

export interface TabItem {
  key: string;
  label: string;
  icon?: string;
  badge?: number;
  href: string;
}

interface BottomTabBarProps {
  tabs: TabItem[];
  activeKey: string;
  onTabClick: (href: string) => void;
  className?: string;
}

export function BottomTabBar({ tabs, activeKey, onTabClick, className = '' }: BottomTabBarProps) {
  const embed = useEmbed();
  if (embed) return null;

  return (
    <nav
      className={`embed-hidden sticky bottom-0 left-0 right-0 z-sticky bg-[var(--brand-surface)] border-t border-[var(--brand-border)] ${className}`}
      style={{ paddingBottom: 'var(--safe-bottom)' }}
      role="tablist"
    >
      <div className="flex items-center justify-around h-14">
        {tabs.map(tab => {
          const isActive = activeKey === tab.key;
          return (
            <button
              key={tab.key}
              onClick={() => onTabClick(tab.href)}
              role="tab"
              aria-selected={isActive}
              aria-current={isActive ? 'page' : undefined}
              className="relative flex flex-col items-center justify-center flex-1 h-full min-w-0 transition-colors"
              style={{ minHeight: '48px' }}
            >
              {tab.icon && (
                <i
                  className={`${tab.icon} text-xl ${isActive ? 'text-[var(--brand-primary)]' : 'text-[var(--brand-text-muted)]'}`}
                />
              )}
              <span
                className={`text-[11px] font-semibold leading-tight mt-0.5 truncate max-w-full px-1 ${
                  isActive ? 'text-[var(--brand-primary)]' : 'text-[var(--brand-text-muted)]'
                }`}
              >
                {tab.label}
              </span>
              {tab.badge !== undefined && tab.badge > 0 && (
                <span className="absolute -top-0.5 right-1/4 min-w-[18px] h-[18px] flex items-center justify-center rounded-full bg-[var(--color-danger)] text-white text-[10px] font-bold px-1 leading-none">
                  {tab.badge > 99 ? '99+' : tab.badge}
                </span>
              )}
            </button>
          );
        })}
      </div>
    </nav>
  );
}
