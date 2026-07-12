// @deprecated — Unused shell component. apps/web uses inline layout in CourierRoutes.tsx, not this component.
// Kept for reference; remove after confirming no external consumers.
import type { ReactNode } from 'react';
import { useBrandTheme } from './ThemeProvider.js';
import { LanguageSwitcher } from '../../lib/I18nProvider.js';

interface CourierShellProps {
  children: ReactNode;
  courierName?: string;
  isOnline?: boolean;
}

function ThemeToggle() {
  try {
    const { cyclePreset, preset } = useBrandTheme();
    return (
      <button onClick={cyclePreset} className="text-brand-text-muted hover:text-brand-text transition-colors" title={`Theme: ${preset.replace(/-/g, ' ')}`} aria-label={`Current theme: ${preset.replace(/-/g, ' ')}. Click to cycle.`}>
        <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap={`strokeLinecap`} strokeLinejoin={`strokeLinejoin`} strokeWidth={2} d={`d`} />
        </svg>
      </button>
    );
  } catch { return null; }
}

export function CourierShell({ children, courierName, isOnline }: CourierShellProps) {
  return (
    <div className="max-w-md mx-auto min-h-screen bg-brand-bg text-brand-text font-body flex flex-col">
      <header className="h-14 px-4 flex items-center justify-between bg-brand-surface border-b border-brand-border shrink-0">
        <span className="font-semibold">{courierName || 'Courier'}</span>
        <span className="flex items-center gap-3">
          <LanguageSwitcher variant="compact" />
          <ThemeToggle />
          <span className={`flex items-center gap-1.5 text-sm ${isOnline ? 'text-semantic-success' : 'text-brand-text-muted'}`}>
            <span className={`w-2 h-2 rounded-full ${isOnline ? 'bg-semantic-success' : 'bg-brand-text-muted'}`} />
            {isOnline ? 'Online' : 'Offline'}
          </span>
        </span>
      </header>
      <div className="flex-1 overflow-auto">
        {children}
      </div>
    </div>
  );
}
