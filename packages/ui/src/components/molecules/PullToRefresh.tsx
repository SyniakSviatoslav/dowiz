import { useState, useRef, useCallback, type ReactNode, type TouchEvent } from 'react';

interface PullToRefreshProps {
  onRefresh: () => Promise<void>;
  children: ReactNode;
  className?: string;
  pullThreshold?: number;
}

export function PullToRefresh({ onRefresh, children, className = '', pullThreshold = 60 }: PullToRefreshProps) {
  const [pullDistance, setPullDistance] = useState(0);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isPulling, setIsPulling] = useState(false);
  const startYRef = useRef(0);
  const containerRef = useRef<HTMLDivElement>(null);

  const handleTouchStart = useCallback((e: TouchEvent) => {
    if (containerRef.current && containerRef.current.scrollTop > 0) return;
    const touch = e.touches?.[0];
    if (!touch) return;
    startYRef.current = touch.clientY;
    setIsPulling(true);
  }, []);

  const handleTouchMove = useCallback((e: TouchEvent) => {
    if (!isPulling || isRefreshing) return;
    const touch = e.touches?.[0];
    if (!touch) return;
    const diff = touch.clientY - startYRef.current;
    if (diff > 0) {
      setPullDistance(Math.min(diff * 0.5, 120));
    }
  }, [isPulling, isRefreshing]);

  const handleTouchEnd = useCallback(async () => {
    setIsPulling(false);
    if (pullDistance >= pullThreshold && !isRefreshing) {
      setIsRefreshing(true);
      setPullDistance(pullThreshold);
      try {
        await onRefresh();
      } finally {
        setIsRefreshing(false);
        setPullDistance(0);
      }
    } else {
      setPullDistance(0);
    }
  }, [pullDistance, pullThreshold, isRefreshing, onRefresh]);

  return (
    <div
      ref={containerRef}
      className={`relative overflow-hidden ${className}`}
      onTouchStart={handleTouchStart}
      onTouchMove={handleTouchMove}
      onTouchEnd={handleTouchEnd}
      style={{ overscrollBehavior: 'contain' }}
    >
      <div
        className="flex items-center justify-center transition-transform duration-200"
        style={{
          height: `${pullDistance}px`,
          opacity: Math.min(1, pullDistance / pullThreshold),
        }}
      >
        {isRefreshing ? (
          <div className="flex items-center gap-2 text-sm text-[var(--brand-text-muted)]">
            <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24">
              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
            </svg>
          </div>
        ) : pullDistance > 0 ? (
          <i className="ti ti-arrow-down text-lg text-[var(--brand-text-muted)]" />
        ) : null}
      </div>
      <div style={{ transform: `translateY(${pullDistance}px)`, transition: isPulling ? 'none' : 'transform 0.3s ease' }}>
        {children}
      </div>
    </div>
  );
}
