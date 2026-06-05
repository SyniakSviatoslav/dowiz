import type { ReactNode } from 'react';
import { useEmbed } from '../../hooks/use-embed.js';
import { LanguageSwitcher } from '../../lib/I18nProvider.js';

interface ClientShellProps {
  children: ReactNode;
  title?: string;
}

export function ClientShell({ children, title }: ClientShellProps) {
  const embed = useEmbed();
  return (
    <div className={`max-w-lg mx-auto min-h-screen bg-brand-bg text-brand-text font-body ${embed ? 'embed-mode' : ''}`}>
      {title && (
        <div className="sr-only">
          <h1>{title}</h1>
        </div>
      )}
      {!embed && (
        <div className="fixed top-3 right-3 z-30">
          <LanguageSwitcher variant="compact" />
        </div>
      )}
      {children}
    </div>
  );
}
