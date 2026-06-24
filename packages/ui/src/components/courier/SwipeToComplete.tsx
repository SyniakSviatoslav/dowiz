import React, { useState, useRef, useEffect } from 'react';
import { motion, useReducedMotion } from 'framer-motion';
import { useI18n } from '../../lib/I18nProvider.js';

// Matches the design-system --ease-out token cubic-bezier(0.16, 1, 0.3, 1): expo-out, no bounce.
const EASE_OUT: [number, number, number, number] = [0.16, 1, 0.3, 1];

interface SwipeToCompleteProps {
  onComplete: () => Promise<void>;
  label?: string;
  isCompleted?: boolean;
}

export function SwipeToComplete({ onComplete, label, isCompleted = false }: SwipeToCompleteProps) {
  const { t } = useI18n();
  const reduceMotion = useReducedMotion();
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
      <motion.div
        initial={reduceMotion ? false : { scale: 0.97, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        transition={{ duration: 0.24, ease: EASE_OUT }}
        className="h-14 rounded-full bg-[var(--color-success)] flex items-center justify-center gap-2 font-bold text-[var(--color-on-success)] shadow-[var(--elevation-2)]"
      >
        {t('order.delivered', 'Delivered')}
        <svg aria-hidden="true" className="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
          <path d="M5 13l4 4L19 7" />
        </svg>
      </motion.div>
    );
  }

  // Affordance: gentle, looping nudge inviting the swipe — only while idle (rest,
  // not loading) and not when the user prefers reduced motion.
  const idle = !loading && slideRatio === 0;
  const hint = idle && !reduceMotion;

  return (
    <div
      ref={containerRef}
      tabIndex={0}
      role="button"
      aria-label={resolvedLabel}
      data-testid="task-deliver"
      onKeyDown={handleKeyDown}
      className="relative h-14 bg-[var(--brand-surface-raised)] border border-[var(--brand-border)] rounded-full overflow-hidden flex items-center justify-center select-none transition-shadow duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)]"
    >
      <div
        className="absolute inset-y-0 left-0 bg-[var(--status-delivered-bg)] transition-[width] duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none"
        style={{ width: `${slideRatio * 100}%` }}
      />
      <span
        className="font-bold text-[var(--brand-text)] z-10 pointer-events-none transition-opacity duration-[var(--motion-fast)] ease-[var(--ease-out)] motion-reduce:transition-none"
        style={{ opacity: 1 - slideRatio }}
      >
        {loading ? t('common.processing', 'Processing...') : resolvedLabel}
      </span>

      <motion.div
        className="absolute left-1 top-1 bottom-1 w-12 bg-[var(--brand-primary)] rounded-full shadow-[var(--elevation-2)] flex items-center justify-center cursor-grab active:cursor-grabbing z-20"
        animate={hint
          ? { transform: ['translateX(0px)', 'translateX(6px)', 'translateX(0px)'] }
          : { transform: `translateX(${slideRatio * (containerRef.current ? containerRef.current.clientWidth - 56 : 0)}px)` }}
        transition={hint
          ? { duration: 1.4, repeat: Infinity, repeatDelay: 0.6, ease: EASE_OUT }
          : { duration: 0 }}
        role="presentation"
        aria-hidden="true"
        onMouseDown={handleDragStart}
        onTouchStart={handleDragStart}
      >
        <svg className="h-5 w-5 text-[var(--brand-bg)]" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
          <path d="M9 6l6 6-6 6" />
        </svg>
      </motion.div>

      {/* Keyboard-accessible fallback — explicit, visible action for non-pointer users. */}
      <button
        type="button"
        onClick={() => { if (!loading && !isCompleted) { setSlideRatio(1); triggerComplete(); } }}
        disabled={loading}
        className="sr-only focus:not-sr-only focus:absolute focus:inset-0 focus:z-30 focus:flex focus:items-center focus:justify-center focus:rounded-full focus:bg-[var(--brand-primary)] focus:text-[var(--brand-bg)] focus:font-bold focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)]"
      >
        {t('courier.confirm_delivery', 'Confirm delivery')}
      </button>
    </div>
  );
}
