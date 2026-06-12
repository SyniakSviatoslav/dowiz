import type { ReactNode } from 'react';

interface SafeAreaViewProps {
  children: ReactNode;
  className?: string;
  top?: boolean;
  bottom?: boolean;
  left?: boolean;
  right?: boolean;
}

export function SafeAreaView({ children, className = '', top = false, bottom = false, left = false, right = false }: SafeAreaViewProps) {
  const paddingStyles = [
    top && 'var(--safe-top)',
    right && 'var(--safe-right)',
    bottom && 'var(--safe-bottom)',
    left && 'var(--safe-left)',
  ].filter(Boolean).join(' ');

  return (
    <div
      className={`flex-1 overflow-auto ${className}`}
      style={paddingStyles.length > 0 ? { padding: paddingStyles } : undefined}
    >
      {children}
    </div>
  );
}
