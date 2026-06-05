import { useEffect, type ReactNode } from 'react';

interface EmbedShellProps {
  children: ReactNode;
}

export function EmbedShell({ children }: EmbedShellProps) {
  useEffect(() => {
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const height = entry.contentBoxSize?.[0]?.blockSize ?? entry.contentRect.height;
        window.parent.postMessage({ type: 'dos-embed-resize', height: Math.ceil(height + 16) }, '*');
      }
    });
    observer.observe(document.body);
    return () => observer.disconnect();
  }, []);

  return <>{children}</>;
}
