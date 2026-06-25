import { type ReactNode, useEffect, useRef, useState } from 'react';
import { useEmbed } from '../../hooks/use-embed.js';

interface StickyActionBarProps {
  children: ReactNode;
  className?: string;
  embedSticky?: boolean;
}

export function StickyActionBar({ children, className = '', embedSticky = true }: StickyActionBarProps) {
  const embed = useEmbed();
  const barRef = useRef<HTMLDivElement>(null);
  const [keyboardOffset, setKeyboardOffset] = useState(0);

  useEffect(() => {
    if (embed) return;
    if (typeof window === 'undefined' || !('visualViewport' in window)) return;

    const vv = window.visualViewport!;
    const handleResize = () => {
      const offset = window.innerHeight - vv.height;
      setKeyboardOffset(Math.max(0, offset));
    };
    vv.addEventListener('resize', handleResize);
    return () => vv.removeEventListener('resize', handleResize);
  }, [embed]);

  if (embed && !embedSticky) return null;

  const style = keyboardOffset > 0
    ? { transform: `translateY(-${keyboardOffset}px)`, transition: 'transform 0.2s ease' }
    : {};

  const baseClass = embed
    ? 'sticky bottom-0 left-0 right-0 z-sticky'
    : 'fixed bottom-0 left-0 right-0 z-sticky';

  return (
    <div
      ref={barRef}
      data-fixed={embed ? 'true' : undefined}
      className={`${baseClass} bg-[var(--brand-surface)] ${className}`}
      style={{ ...style, boxShadow: '0 -8px 24px rgba(0,0,0,.07), 0 -2px 6px rgba(0,0,0,.05)' }}
    >
      <div className="flex min-w-0 flex-col px-4 py-3" style={{ paddingBottom: `calc(0.75rem + var(--safe-bottom))` }}>
        {children}
      </div>
    </div>
  );
}
