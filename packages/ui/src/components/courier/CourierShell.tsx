import React from 'react';

interface CourierShellProps {
  children: React.ReactNode;
  currentPath: string;
  onNavigate: (path: string) => void;
  tasksCount?: number;
}

export function CourierShell({ children, currentPath, onNavigate, tasksCount = 0 }: CourierShellProps) {
  const tabs = [
    { path: '/courier', label: 'Tasks', icon: '\u2630' },
    { path: '/courier/earnings', label: 'Earnings', icon: '\u0024' },
    { path: '/courier/profile', label: 'Profile', icon: '\u263B' },
  ];

  return (
    <div className="h-screen bg-[var(--brand-bg)] text-[var(--brand-text)] flex flex-col overflow-hidden">
      
      {/* Main Content Area */}
      <div className="flex-1 overflow-auto pb-16">
        {children}
      </div>

      {/* Bottom Navigation */}
      <div className="fixed bottom-0 left-0 right-0 h-16 bg-[var(--brand-surface)] border-t border-[var(--brand-border)] flex items-center justify-around z-50">
        {tabs.map(tab => {
          const isActive = currentPath === tab.path || (tab.path !== '/courier' && currentPath.startsWith(tab.path));
          return (
            <button
              key={tab.path}
              onClick={() => onNavigate(tab.path)}
              className={`flex flex-col items-center justify-center w-full h-full transition-colors ${isActive ? 'text-[var(--brand-primary)]' : 'text-[var(--brand-text-muted)] hover:text-[var(--brand-text)]'}`}
            >
              <span className="relative inline-flex text-xl mb-1">
                {tab.icon}
                {tab.label === 'Tasks' && tasksCount > 0 && (
                  <span className="absolute -top-1.5 -right-2.5 min-w-[16px] h-[16px] rounded-full bg-[var(--color-danger-strong)] text-white text-step-2xs font-bold flex items-center justify-center leading-none px-1 shadow-md">
                    {tasksCount > 99 ? '99+' : tasksCount}
                  </span>
                )}
              </span>
              <span className="text-step-2xs font-semibold">{tab.label}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
