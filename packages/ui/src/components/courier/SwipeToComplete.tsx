import React, { useState, useRef, useEffect } from 'react';
import { useI18n } from '../../lib/I18nProvider.js';

interface SwipeToCompleteProps {
  onComplete: () => Promise<void>;
  label?: string;
  isCompleted?: boolean;
}

export function SwipeToComplete({ onComplete, label, isCompleted = false }: SwipeToCompleteProps) {
  const { t } = useI18n();
  const resolvedLabel = label || t('courier.slide_to_deliver', 'Slide to Deliver');
  const [loading, setLoading] = useState(false);
  const [slideRatio, setSlideRatio] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);
  const draggingRef = useRef(false);
  const moveHandlerRef = useRef<((e: TouchEvent | MouseEvent) => void) | null>(null);
  const endHandlerRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    return () => {
      if (!draggingRef.current) return;
      if (moveHandlerRef.current) {
        document.removeEventListener('touchmove', moveHandlerRef.current);
        document.removeEventListener('mousemove', moveHandlerRef.current);
      }
      if (endHandlerRef.current) {
        document.removeEventListener('touchend', endHandlerRef.current);
        document.removeEventListener('mouseup', endHandlerRef.current);
      }
    };
  }, []);

  const handleDragStart = (e: React.TouchEvent | React.MouseEvent) => {
    if (loading || isCompleted) return;
    
    const startX = 'touches' in e ? (e.touches[0]?.clientX ?? 0) : e.clientX;
    const container = containerRef.current;
    if (!container) return;

    const maxMove = container.clientWidth - 56;

    const handleMove = (moveEvent: TouchEvent | MouseEvent) => {
      const currentX = 'touches' in moveEvent ? (moveEvent.touches[0]?.clientX ?? 0) : (moveEvent as MouseEvent).clientX;
      const move = Math.max(0, Math.min(currentX - startX, maxMove));
      setSlideRatio(move / maxMove);
    };

    const handleEnd = async () => {
      draggingRef.current = false;
      moveHandlerRef.current = null;
      endHandlerRef.current = null;
      document.removeEventListener('touchmove', handleMove);
      document.removeEventListener('mousemove', handleMove);
      document.removeEventListener('touchend', handleEnd);
      document.removeEventListener('mouseup', handleEnd);
      
      setSlideRatio(prev => {
        if (prev > 0.8) {
          triggerComplete();
          return 1;
        }
        return 0;
      });
    };

    draggingRef.current = true;
    moveHandlerRef.current = handleMove;
    endHandlerRef.current = handleEnd;
    document.addEventListener('touchmove', handleMove, { passive: false });
    document.addEventListener('mousemove', handleMove);
    document.addEventListener('touchend', handleEnd);
    document.addEventListener('mouseup', handleEnd);
  };

  const triggerComplete = async () => {
    setLoading(true);
    try {
      await onComplete();
    } catch (e) {
      setSlideRatio(0);
    } finally {
      setLoading(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (loading || isCompleted) return;
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      setSlideRatio(1);
      triggerComplete();
    }
  };

  if (isCompleted) {
    return (
      <div className="h-14 rounded-full bg-[var(--color-success)] flex items-center justify-center font-bold text-[var(--color-on-success)] shadow-lg">
        {t('order.delivered', 'Delivered')} \u2713
      </div>
    );
  }

  return (
    <div 
      ref={containerRef}
      tabIndex={0}
      role="button"
      aria-label={resolvedLabel}
      data-testid="task-deliver"
      onKeyDown={handleKeyDown}
      className="relative h-14 bg-[var(--brand-surface-raised)] border border-[var(--brand-border)] rounded-full overflow-hidden flex items-center justify-center select-none focus:outline-2 focus:outline-[var(--brand-primary)]"
    >
      <div className="absolute inset-0 bg-[var(--status-delivered-bg)]" style={{ width: `${slideRatio * 100}%` }} />
      <span className="font-bold text-[var(--brand-text-muted)] z-10 pointer-events-none" style={{ opacity: 1 - slideRatio }}>                  {loading ? t('common.processing', 'Processing...') : resolvedLabel}</span>
      
      <div 
        className="absolute left-1 top-1 bottom-1 w-12 bg-[var(--brand-primary)] rounded-full shadow-md flex items-center justify-center cursor-grab active:cursor-grabbing z-20 transition-transform"
        style={{ transform: `translateX(${slideRatio * (containerRef.current ? containerRef.current.clientWidth - 56 : 0)}px)` }}
        onMouseDown={handleDragStart}
        onTouchStart={handleDragStart}
      >
        <div className="w-1.5 h-6 border-x-2 border-white/50" />
      </div>
    </div>
  );
}
