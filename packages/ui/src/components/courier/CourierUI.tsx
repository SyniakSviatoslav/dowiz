import React, { useState, useRef, useEffect } from 'react';
import { formatALL } from '@deliveryos/shared-types';

// --- CourierShell ---
interface CourierShellProps {
  children: React.ReactNode;
  currentPath: string;
  onNavigate: (path: string) => void;
}
export function CourierShell({ children, currentPath, onNavigate }: CourierShellProps) {
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
              <span className="text-xl mb-1">{tab.icon}</span>
              <span className="text-[10px] font-semibold">{tab.label}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

// --- TaskCard ---
export interface CourierTask {
  id: string;
  status: string;
  restaurant: { name: string; address: string; lat?: number; lng?: number; };
  customer: { address: string; phone?: string; instructions?: string; lat?: number; lng?: number; };
  total: number;
  eta: string;
}

interface TaskCardProps {
  task: CourierTask;
  onAccept: (id: string) => void;
  onReject?: (id: string) => void;
  isLoading?: boolean;
}
export function TaskCard({ task, onAccept, onReject, isLoading }: TaskCardProps) {
  return (
    <div className={`bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-4 space-y-4 ${isLoading ? 'opacity-50 pointer-events-none' : ''}`}>
      
      {/* Header */}
      <div className="flex justify-between items-start">
        <h3 className="font-bold text-lg text-[var(--brand-text)]">New Delivery</h3>
        <span className="bg-[var(--status-pending-bg)] text-[var(--status-pending)] font-bold px-2 py-1 rounded text-sm">{task.eta}</span>
      </div>

      {/* Locations */}
      <div className="relative pl-6 space-y-4 before:content-[''] before:absolute before:left-2.5 before:top-2 before:bottom-2 before:w-0.5 before:bg-[var(--brand-border)]">
        
        {/* Pickup */}
        <div className="relative">
          <div className="absolute -left-[26px] top-1 w-3 h-3 rounded-full bg-[var(--brand-primary)] border-2 border-[var(--brand-surface)]" />
          <div className="text-xs text-[var(--brand-text-muted)] uppercase font-bold tracking-wider">Pickup</div>
          <div className="font-medium text-[var(--brand-text)]">{task.restaurant.name}</div>
          <div className="text-sm text-[var(--brand-text-muted)]">{task.restaurant.address}</div>
        </div>

        {/* Dropoff */}
        <div className="relative">
          <div className="absolute -left-[26px] top-1 w-3 h-3 rounded-full bg-[var(--color-success)] border-2 border-[var(--brand-surface)]" />
          <div className="text-xs text-[var(--brand-text-muted)] uppercase font-bold tracking-wider">Drop-off</div>
          <div className="font-medium text-[var(--brand-text)]">{task.customer.address}</div>
        </div>

      </div>

      <div className="border-t border-[var(--brand-border)] pt-4 flex gap-3">
        {onReject && (
          <button 
            onClick={() => onReject(task.id)}
            className="flex-1 bg-[var(--brand-surface-raised)] hover:bg-[var(--brand-border)] text-[var(--brand-text)] py-3 rounded-[var(--brand-radius-btn)] font-semibold transition-colors"
          >
            Reject
          </button>
        )}
        <button 
          onClick={() => onAccept(task.id)}
          className="flex-1 bg-[var(--brand-primary)] hover:bg-[var(--brand-primary-hover)] text-white py-3 rounded-[var(--brand-radius-btn)] font-semibold transition-colors shadow-lg"
        >
          Accept Task
        </button>
      </div>

    </div>
  );
}

// --- SwipeToComplete ---
interface SwipeToCompleteProps {
  onComplete: () => Promise<void>;
  label?: string;
  isCompleted?: boolean;
}
export function SwipeToComplete({ onComplete, label = 'Slide to Deliver', isCompleted = false }: SwipeToCompleteProps) {
  const [loading, setLoading] = useState(false);
  const [slideRatio, setSlideRatio] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);
  const draggingRef = useRef(false);
  const moveHandlerRef = useRef<((e: TouchEvent | MouseEvent) => void) | null>(null);
  const endHandlerRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    return () => {
      // Cleanup drag listeners on unmount to prevent memory leak (P0-12)
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

    const maxMove = container.clientWidth - 56; // 56 is the width of the thumb

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
        Delivered \u2713
      </div>
    );
  }

  return (
    <div 
      ref={containerRef}
      tabIndex={0}
      role="button"
      aria-label={label}
      onKeyDown={handleKeyDown}
      className="relative h-14 bg-[var(--brand-surface-raised)] border border-[var(--brand-border)] rounded-full overflow-hidden flex items-center justify-center select-none focus:outline-2 focus:outline-[var(--brand-primary)]"
    >
      <div className="absolute inset-0 bg-[var(--status-delivered-bg)]" style={{ width: `${slideRatio * 100}%` }} />
      <span className="font-bold text-[var(--brand-text-muted)] z-10 pointer-events-none" style={{ opacity: 1 - slideRatio }}>{loading ? 'Processing...' : label}</span>
      
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
